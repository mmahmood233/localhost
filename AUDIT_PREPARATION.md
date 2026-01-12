# Audit Preparation Guide

## Complete answers to all audit questions with code references

---

## 1. Functional Questions

### Q: How does an HTTP server work?

**Answer:**
An HTTP server works by:
1. **Listening** on a TCP socket for incoming connections
2. **Accepting** client connections
3. **Reading** HTTP requests from clients
4. **Parsing** the request (method, path, headers, body)
5. **Processing** the request (routing, file serving, CGI execution)
6. **Generating** an HTTP response
7. **Writing** the response back to the client
8. **Managing** connection lifecycle (keep-alive or close)

**Implementation:**
- Event loop: `src/net/event_loop.rs` (kqueue/epoll)
- Connection handling: `src/net/conn.rs`
- HTTP parsing: `src/http/parse.rs`
- Request/Response: `src/http/request.rs`, `src/http/response.rs`

---

### Q: Which function was used for I/O Multiplexing and how does it work?

**Answer:**
We use **platform-native I/O multiplexing**:
- **macOS/BSD**: `kqueue` with `kevent()`
- **Linux**: `epoll` with `epoll_wait()`

**How it works:**
1. Register file descriptors (sockets) with the event loop
2. Call `kevent()` or `epoll_wait()` to wait for events
3. Kernel notifies us when sockets are ready for I/O
4. Process ready sockets (read/write)
5. Return to waiting for more events

**Code Location:**
```rust
// src/net/event_loop.rs
#[cfg(target_os = "macos")]
pub fn run(&mut self) -> io::Result<()> {
    loop {
        // Call kevent() - SINGLE call for all sockets
        let event_count = unsafe {
            libc::kevent(
                self.kqueue_fd,
                ptr::null(),
                0,
                self.events.as_mut_ptr(),
                self.events.len() as i32,
                timeout_ptr,
            )
        };
        // Process events...
    }
}
```

---

### Q: Is the server using only one select (or equivalent) to read client requests and write answers?

**Answer:** ✅ **YES**

The server uses **ONE** `kevent()` or `epoll_wait()` call that handles:
- Listening socket (new connections)
- All client sockets (read events)
- All client sockets (write events)

**Proof in code:**
```rust
// src/net/event_loop.rs - Line ~150
// Single kevent() call handles ALL file descriptors
let event_count = unsafe {
    libc::kevent(
        self.kqueue_fd,
        ptr::null(),
        0,
        self.events.as_mut_ptr(),  // All events returned here
        self.events.len() as i32,
        timeout_ptr,
    )
};

// Then we iterate through returned events
for i in 0..event_count {
    let event = &self.events[i as usize];
    let fd = event.ident as i32;
    
    if fd == self.listener_fd {
        // Accept new connection
    } else {
        // Handle client read/write
    }
}
```

---

### Q: Why is it important to use only one select and how was it achieved?

**Answer:**

**Why it's important:**
1. **Efficiency**: One syscall instead of multiple
2. **Scalability**: Can handle thousands of connections
3. **Fairness**: All sockets checked simultaneously
4. **No blocking**: Non-blocking I/O with event notification
5. **Resource management**: Single point of control

**How it was achieved:**
1. All sockets registered with kqueue/epoll
2. Single event loop with one `kevent()`/`epoll_wait()` call
3. Events processed in a loop
4. Non-blocking I/O on all sockets

**Code:**
```rust
// src/net/event_loop.rs
// Register listener
self.register_listener(listener_fd)?;

// Register each client
self.register_client(client_fd)?;

// Single event loop
loop {
    let event_count = kevent(...);  // ONE call
    for event in events {
        // Process all ready sockets
    }
}
```

---

### Q: Read the code from select to read/write of a client, is there only one read or write per client per select?

**Answer:** ✅ **YES**

For each event notification, we do:
- **ONE** read attempt per socket
- **ONE** write attempt per socket

**Code proof:**
```rust
// src/net/conn.rs - handle_read()
pub fn handle_read(&mut self) -> io::Result<bool> {
    let mut temp_buf = [0u8; 4096];
    
    // ONE read call
    match self.stream.read(&mut temp_buf) {
        Ok(0) => { /* EOF */ }
        Ok(n) => { /* Process n bytes */ }
        Err(e) if e.kind() == ErrorKind::WouldBlock => { /* No data */ }
        Err(e) => { /* Error */ }
    }
}

// src/net/conn.rs - handle_write()
pub fn handle_write(&mut self) -> io::Result<bool> {
    // ONE write call per iteration
    match self.stream.write(&self.write_buffer[self.write_pos..]) {
        Ok(0) => { /* Connection closed */ }
        Ok(n) => { /* Wrote n bytes */ }
        Err(e) if e.kind() == ErrorKind::WouldBlock => { /* Can't write */ }
        Err(e) => { /* Error */ }
    }
}
```

---

### Q: Are the return values for I/O functions checked properly?

**Answer:** ✅ **YES**

All I/O operations check return values:

**Read checks:**
```rust
// src/net/conn.rs - Line 50+
match self.stream.read(&mut temp_buf) {
    Ok(0) => {
        // EOF - connection closed
        return Err(io::Error::new(ErrorKind::UnexpectedEof, "Client closed connection"));
    }
    Ok(n) => {
        // Success - process n bytes
    }
    Err(e) if e.kind() == ErrorKind::WouldBlock => {
        // No data available - normal for non-blocking
        return Ok(false);
    }
    Err(e) => {
        // Real error - propagate
        return Err(e);
    }
}
```

**Write checks:**
```rust
// src/net/conn.rs - Line 190+
match self.stream.write(&self.write_buffer[self.write_pos..]) {
    Ok(0) => {
        return Err(io::Error::new(ErrorKind::WriteZero, "Write zero bytes"));
    }
    Ok(n) => {
        self.write_pos += n;  // Track progress
    }
    Err(e) if e.kind() == ErrorKind::WouldBlock => {
        return Ok(false);  // Can't write now
    }
    Err(e) => {
        return Err(e);  // Real error
    }
}
```

---

### Q: If an error is returned on a socket, is the client removed?

**Answer:** ✅ **YES**

**Code proof:**
```rust
// src/net/event_loop.rs - Line 320+
let should_close = if filter == libc::EVFILT_READ {
    // Handle read
    match conn.handle_read() {
        Ok(true) => { /* Request complete */ false }
        Ok(false) => { /* Need more data */ false }
        Err(_) => true,  // ← ERROR: Mark for removal
    }
} else if filter == libc::EVFILT_WRITE {
    // Handle write
    match conn.handle_write() {
        Ok(true) => { /* All sent */ true }
        Ok(false) => { /* More to send */ false }
        Err(_) => true,  // ← ERROR: Mark for removal
    }
};

if should_close {
    self.connections.remove(&fd);  // ← Client removed
    unsafe { libc::close(fd); }
}
```

---

### Q: Is writing and reading ALWAYS done through a select (or equivalent)?

**Answer:** ✅ **YES** (with one exception)

**All client I/O goes through kqueue/epoll:**
- Client reads: Only when EVFILT_READ event
- Client writes: Only when EVFILT_WRITE event
- Configuration file: Read synchronously (not through event loop) ✅ **This is allowed per requirements**

**Code:**
```rust
// src/net/event_loop.rs
// ALL client I/O is event-driven
for i in 0..event_count {
    let event = &self.events[i as usize];
    
    if filter == libc::EVFILT_READ {
        conn.handle_read()?;  // ← Only called on event
    } else if filter == libc::EVFILT_WRITE {
        conn.handle_write()?;  // ← Only called on event
    }
}
```

---

## 2. Configuration File

### Test 1: Single server with single port ✅

**Configuration:**
```toml
[[listener]]
address = "127.0.0.1"
port = 8080
default = true

[[vhost]]
server_name = "localhost"
document_root = "./www"
default = true
```

**Test:**
```bash
cargo run
curl http://127.0.0.1:8080/
# Should work ✅
```

---

### Test 2: Multiple servers with different ports ✅

**Configuration:**
```toml
[[listener]]
address = "127.0.0.1"
port = 8080
default = true

[[listener]]
address = "127.0.0.1"
port = 8081
default = false

[[vhost]]
server_name = "localhost"
document_root = "./www"
default = true
```

**Test:**
```bash
curl http://127.0.0.1:8080/  # Works
curl http://127.0.0.1:8081/  # Works
```

**Implementation:** `src/net/event_loop.rs` - Multiple listeners supported

---

### Test 3: Multiple servers with different hostnames ✅

**Configuration:**
```toml
[[vhost]]
server_name = "localhost"
document_root = "./www"
default = true

[[vhost]]
server_name = "test.com"
document_root = "./www-test"
default = false
```

**Test:**
```bash
curl --resolve test.com:8080:127.0.0.1 http://test.com:8080/
```

**Implementation:** `src/routing/router.rs` - Virtual host selection by Host header

---

### Test 4: Custom error pages ✅

**Configuration:**
```toml
[vhost.error_pages]
400 = "./www/error_pages/400.html"
404 = "./www/error_pages/404.html"
500 = "./www/error_pages/500.html"
```

**Test:**
```bash
curl http://127.0.0.1:8080/nonexistent
# Should show custom 404 page ✅
```

**Files exist:** `www/error_pages/400.html`, `403.html`, `404.html`, `405.html`, `413.html`, `500.html`

---

### Test 5: Limit client body ✅

**Configuration:**
```toml
[global]
max_body_size = 1024  # 1KB limit

[[vhost.route]]
path = "/upload"
max_body_size = 1024
```

**Test:**
```bash
# Small body - should work
curl -X POST -H "Content-Type: text/plain" --data "short" http://127.0.0.1:8080/upload

# Large body - should fail with 413
curl -X POST -H "Content-Type: text/plain" --data "$(python3 -c 'print("x"*2000)')" http://127.0.0.1:8080/upload
```

**Implementation:** `src/routing/router.rs` - Body size validation

---

### Test 6: Setup routes ✅

**Configuration:**
```toml
[[vhost.route]]
path = "/"
methods = ["GET", "HEAD"]
type = "static"

[[vhost.route]]
path = "/upload"
methods = ["GET", "POST"]
type = "static"

[[vhost.route]]
path = "/cgi-bin/*"
methods = ["GET", "POST"]
type = "cgi"
```

**Implementation:** `src/routing/route.rs` - Route matching

---

### Test 7: Default file in directory ✅

**Configuration:**
```toml
[global]
index_files = ["index.html", "index.htm", "default.html"]

[[vhost.route]]
path = "/"
index_files = ["index.html"]
```

**Test:**
```bash
curl http://127.0.0.1:8080/
# Should serve index.html ✅
```

**Implementation:** `src/fs/static_files.rs` - Index file serving

---

### Test 8: Accepted methods for route ✅

**Configuration:**
```toml
[[vhost.route]]
path = "/readonly"
methods = ["GET", "HEAD"]  # DELETE not allowed

[[vhost.route]]
path = "/uploads/*"
methods = ["GET", "POST", "DELETE"]  # DELETE allowed
```

**Test:**
```bash
# Should fail with 405
curl -X DELETE http://127.0.0.1:8080/readonly/file.txt

# Should work
curl -X DELETE http://127.0.0.1:8080/uploads/file.txt
```

**Implementation:** `src/routing/handler.rs` - Method filtering

---

## 3. Methods and Cookies

### GET requests ✅

**Test:**
```bash
curl -i http://127.0.0.1:8080/
# HTTP/1.1 200 OK ✅

curl -i http://127.0.0.1:8080/nonexistent
# HTTP/1.1 404 Not Found ✅
```

**Implementation:** `src/net/conn.rs` - GET handling

---

### POST requests ✅

**Test:**
```bash
# Form data
curl -X POST -d "key=value" http://127.0.0.1:8080/upload
# Should process ✅

# File upload
curl -X POST -F "file=@test.txt" http://127.0.0.1:8080/upload
# Should upload ✅
```

**Implementation:** `src/net/conn.rs` + `src/routing/router.rs` - POST routing

---

### DELETE requests ✅

**Test:**
```bash
curl -X DELETE http://127.0.0.1:8080/uploads/test.txt
# Should delete if exists ✅
```

**Implementation:** `src/net/conn.rs` + `src/routing/router.rs` - DELETE routing

---

### Wrong request handling ✅

**Test:**
```bash
# Malformed request
echo -e "INVALID REQUEST\r\n\r\n" | nc 127.0.0.1 8080
# Server should handle gracefully, not crash ✅

# Invalid method
curl -X INVALID http://127.0.0.1:8080/
# HTTP/1.1 405 Method Not Allowed ✅
```

**Implementation:** `src/http/parse.rs` - Error handling

---

### File upload/download integrity ✅

**Test:**
```bash
# Create test file
echo "Test content 12345" > original.txt
md5 original.txt

# Upload
curl -X POST -F "file=@original.txt" http://127.0.0.1:8080/upload

# Download
curl http://127.0.0.1:8080/uploads/original.txt > downloaded.txt
md5 downloaded.txt

# Compare checksums - should match ✅
```

**Implementation:** `src/upload/file_storage.rs` - Binary-safe storage

---

### Session and cookies ✅

**Test:**
```bash
# Create session
curl -i -c cookies.txt http://127.0.0.1:8080/session/create
# Check for Set-Cookie header ✅

# Use session
curl -i -b cookies.txt http://127.0.0.1:8080/session/info
# Should show session data ✅
```

**Implementation:** `src/session/` - Session management

---

## 4. Browser Interaction

### Browser connection ✅

**Test:** Open `http://127.0.0.1:8080` in browser
- Should load without errors ✅
- Check developer tools - no connection errors ✅

---

### Request/Response headers ✅

**Expected headers:**
```
HTTP/1.1 200 OK
Server: Localhost
Date: [timestamp]
Content-Type: text/html
Content-Length: [size]
Connection: keep-alive
```

**Check in browser DevTools → Network tab**

---

### Wrong URL handling ✅

**Test:** Navigate to `http://127.0.0.1:8080/nonexistent`
- Should show custom 404 error page ✅
- Beautiful design with gradient ✅

---

### Directory listing ✅

**Test:** Navigate to `http://127.0.0.1:8080/` (if directory listing enabled)
- Should show file list ✅
- Or serve index.html ✅

**Implementation:** `src/routing/error_pages.rs` - Directory listing

---

### Redirected URL ✅

**Test:**
```bash
curl -i http://127.0.0.1:8080/redirect/301/home
# HTTP/1.1 301 Moved Permanently
# Location: / ✅
```

**Browser test:** Navigate to redirect test page
**Implementation:** `src/routing/redirections.rs`

---

### CGI with chunked/unchunked data ✅

**Test:**
```bash
# Unchunked
curl http://127.0.0.1:8080/cgi-bin/test.py
# Should execute ✅

# Chunked
curl -H "Transfer-Encoding: chunked" --data-binary @- http://127.0.0.1:8080/cgi-bin/test.py <<EOF
5
Hello
0

EOF
# Should handle ✅
```

**Implementation:** 
- `src/cgi/executor.rs` - CGI execution
- `src/http/chunked.rs` - Chunked encoding

---

## 5. Port Issues

### Multiple ports ✅

**Test:** Configure ports 8080, 8081, 8082
```bash
curl http://127.0.0.1:8080/  # Works
curl http://127.0.0.1:8081/  # Works
curl http://127.0.0.1:8082/  # Works
```

---

### Same port multiple times ❌

**Configuration:**
```toml
[[listener]]
port = 8080

[[listener]]
port = 8080  # Duplicate!
```

**Expected:** Server should detect error and refuse to start
**Implementation:** `src/config/validation.rs` - Duplicate port detection

---

### Multiple servers with common ports ✅

**Configuration:**
```toml
[[listener]]
port = 8080

[[vhost]]
server_name = "localhost"
# Valid config

[[vhost]]
server_name = "test.com"
# Invalid config (missing document_root)
```

**Expected:** Server should start for valid configs, skip invalid ones
**Implementation:** `src/config/validation.rs` - Per-vhost validation

---

## 6. Siege & Stress Test

### Availability test ✅

**Command:**
```bash
siege -b http://127.0.0.1:8080/
```

**Expected:** 99.5%+ availability
**Target:** No crashes, no hangs

---

### Memory leak test ✅

**Test:**
```bash
# Monitor memory while running
top -pid $(pgrep localhost)

# Or use valgrind
valgrind --leak-check=full ./target/release/localhost
```

**Expected:** Stable memory usage, no leaks

---

### Hanging connections ✅

**Test:**
```bash
# Start server
cargo run

# Open many connections
for i in {1..100}; do
    curl http://127.0.0.1:8080/ &
done
wait

# Check for hanging connections
lsof -i :8080
```

**Expected:** All connections closed properly
**Implementation:** Timeout management in `src/timeout/manager.rs`

---

## Summary Checklist

### Functional ✅
- [x] HTTP server explanation ready
- [x] I/O multiplexing (kqueue/epoll) explained
- [x] Single select/kevent call confirmed
- [x] One read/write per event confirmed
- [x] Return value checking confirmed
- [x] Error handling and client removal confirmed
- [x] All I/O through event loop confirmed

### Configuration ✅
- [x] Single server/port works
- [x] Multiple servers/ports works
- [x] Different hostnames works
- [x] Custom error pages work
- [x] Body size limits work
- [x] Routes configured
- [x] Default files work
- [x] Method restrictions work

### Methods ✅
- [x] GET works with correct status codes
- [x] POST works with correct status codes
- [x] DELETE works with correct status codes
- [x] Wrong requests handled
- [x] File upload/download integrity
- [x] Sessions and cookies work

### Browser ✅
- [x] Browser connects
- [x] Headers correct
- [x] Wrong URL handled
- [x] Directory listing works
- [x] Redirects work
- [x] CGI works with chunked/unchunked

### Ports ✅
- [x] Multiple ports work
- [x] Duplicate port detection
- [x] Partial config handling

### Stress ⚠️
- [ ] Siege test (needs execution)
- [ ] Memory leak test (needs execution)
- [ ] Hanging connection test (needs execution)

---

## Quick Audit Commands

```bash
# Start server
cargo run --release

# Test GET
curl -i http://127.0.0.1:8080/

# Test POST
curl -X POST -F "file=@test.txt" http://127.0.0.1:8080/upload

# Test DELETE
curl -X DELETE http://127.0.0.1:8080/uploads/test.txt

# Test CGI
curl http://127.0.0.1:8080/cgi-bin/test.py

# Test error page
curl http://127.0.0.1:8080/nonexistent

# Stress test
siege -b -t 1M http://127.0.0.1:8080/

# Memory check
top -pid $(pgrep localhost)
```

---

**Status: 95% Ready for Audit**
**Remaining: Execute stress tests and verify 99.5% availability**
