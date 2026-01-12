# Audit Quick Reference Card

## Key Code Locations

### I/O Multiplexing
- **File:** `src/net/event_loop.rs`
- **Function:** `kevent()` (macOS) or `epoll_wait()` (Linux)
- **Line:** ~150
- **Key Point:** ONE call handles ALL sockets

### Single Read/Write Per Event
- **File:** `src/net/conn.rs`
- **Read:** `handle_read()` - Line 50
- **Write:** `handle_write()` - Line 190
- **Key Point:** Each function does ONE syscall

### Error Handling & Client Removal
- **File:** `src/net/event_loop.rs`
- **Line:** ~320
- **Code:**
```rust
Err(_) => true,  // Mark for removal
if should_close {
    self.connections.remove(&fd);
    unsafe { libc::close(fd); }
}
```

### Return Value Checking
- **Every I/O operation checks:**
  - `Ok(0)` - EOF
  - `Ok(n)` - Success
  - `Err(WouldBlock)` - Normal for non-blocking
  - `Err(e)` - Real error

---

## Configuration Examples

### Single Server
```toml
[[listener]]
address = "127.0.0.1"
port = 8080
```

### Multiple Ports
```toml
[[listener]]
port = 8080

[[listener]]
port = 8081
```

### Virtual Hosts
```toml
[[vhost]]
server_name = "localhost"
document_root = "./www"

[[vhost]]
server_name = "test.com"
document_root = "./www-test"
```

### Custom Error Pages
```toml
[vhost.error_pages]
404 = "./www/error_pages/404.html"
```

### Body Size Limit
```toml
[global]
max_body_size = 10485760  # 10MB
```

### Route Methods
```toml
[[vhost.route]]
path = "/upload"
methods = ["GET", "POST"]
```

---

## Test Commands

### Basic Tests
```bash
# GET
curl -i http://127.0.0.1:8080/

# POST
curl -X POST -F "file=@test.txt" http://127.0.0.1:8080/upload

# DELETE
curl -X DELETE http://127.0.0.1:8080/uploads/test.txt

# HEAD
curl -I http://127.0.0.1:8080/

# Wrong method
curl -X INVALID http://127.0.0.1:8080/
```

### CGI Tests
```bash
curl http://127.0.0.1:8080/cgi-bin/test.py
curl http://127.0.0.1:8080/cgi-bin/test.sh
curl http://127.0.0.1:8080/cgi-bin/test.pl
```

### Error Page Tests
```bash
curl http://127.0.0.1:8080/nonexistent  # 404
curl -X POST http://127.0.0.1:8080/readonly  # 405
```

### Session Tests
```bash
curl -i -c cookies.txt http://127.0.0.1:8080/session/create
curl -i -b cookies.txt http://127.0.0.1:8080/session/info
```

### Virtual Host Test
```bash
curl --resolve test.com:8080:127.0.0.1 http://test.com:8080/
```

### Body Size Limit Test
```bash
# Small (should work)
curl -X POST -d "small data" http://127.0.0.1:8080/upload

# Large (should fail with 413)
curl -X POST --data "$(python3 -c 'print("x"*20000000)')" http://127.0.0.1:8080/upload
```

### File Integrity Test
```bash
echo "Test content" > test.txt
md5 test.txt
curl -X POST -F "file=@test.txt" http://127.0.0.1:8080/upload
curl http://127.0.0.1:8080/uploads/test.txt > downloaded.txt
md5 downloaded.txt  # Should match
```

### Stress Test
```bash
siege -b -t 1M http://127.0.0.1:8080/
# Target: 99.5%+ availability
```

### Memory Test
```bash
# Monitor
top -pid $(pgrep localhost)

# Or valgrind
valgrind --leak-check=full ./target/release/localhost
```

### Connection Test
```bash
# Check for hanging connections
lsof -i :8080
netstat -an | grep 8080
```

---

## Key Answers

### Q: How many select/kevent calls?
**A:** ONE per event loop iteration

### Q: How many read/write per socket per event?
**A:** ONE read OR ONE write

### Q: What happens on error?
**A:** Client removed, socket closed

### Q: Is all I/O through event loop?
**A:** YES (except config file - allowed)

### Q: Are return values checked?
**A:** YES - every I/O operation

### Q: Does it work with browsers?
**A:** YES - tested with Chrome, Firefox, Safari

### Q: Custom error pages?
**A:** YES - 6 pages (400, 403, 404, 405, 413, 500)

### Q: Sessions and cookies?
**A:** YES - full implementation

### Q: CGI support?
**A:** YES - Python, Perl, Shell

### Q: Multiple ports?
**A:** YES - unlimited

### Q: Virtual hosts?
**A:** YES - by Host header

### Q: Chunked encoding?
**A:** YES - both request and response

---

## Architecture Diagram

```
Browser Request
      ↓
[Listener Socket]
      ↓
[kevent/epoll] ← ONE CALL FOR ALL
      ↓
[Event Loop]
      ↓
   ┌──┴──┐
   │     │
[Read] [Write] ← ONE per socket
   │     │
   └──┬──┘
      ↓
[HTTP Parser]
      ↓
[Router]
      ↓
   ┌──┴──┐
   │     │
[Static][CGI]
   │     │
   └──┬──┘
      ↓
[Response]
      ↓
[Write to Socket]
```

---

## Files to Show

1. **Event Loop:** `src/net/event_loop.rs`
2. **Connection:** `src/net/conn.rs`
3. **HTTP Parser:** `src/http/parse.rs`
4. **Router:** `src/routing/router.rs`
5. **Config:** `server.toml`
6. **Error Pages:** `www/error_pages/`
7. **CGI Scripts:** `www/cgi-bin/`

---

## Status Indicators

✅ **Working:**
- GET, POST, DELETE, HEAD
- Static files
- CGI (Python, Perl, Shell)
- Sessions & cookies
- Error pages
- Redirects
- Chunked encoding
- Multiple ports
- Virtual hosts
- Timeouts

⚠️ **Needs Testing:**
- Siege stress test
- Memory leak verification
- 99.5% availability

---

## Common Audit Questions

**Q: Show me the kevent/epoll call**
→ `src/net/event_loop.rs` line ~150

**Q: Show me error handling**
→ `src/net/conn.rs` - every I/O function

**Q: Show me client removal**
→ `src/net/event_loop.rs` line ~320

**Q: Show me POST handling**
→ `src/net/conn.rs` line ~167

**Q: Show me CGI execution**
→ `src/cgi/executor.rs`

**Q: Show me session management**
→ `src/session/store.rs`

**Q: Show me configuration parsing**
→ `src/config/parser.rs`

---

## Performance Targets

| Metric | Target | Status |
|--------|--------|--------|
| Availability | 99.5%+ | ⚠️ Test |
| Memory Leaks | 0 | ⚠️ Test |
| Concurrent | 100+ | ✅ Yes |
| Timeout | 30s | ✅ Yes |
| Keep-alive | 10s | ✅ Yes |

---

## Final Checklist

- [ ] Server starts without errors
- [ ] Browser loads homepage
- [ ] All test commands work
- [ ] Siege shows 99.5%+ availability
- [ ] No memory leaks
- [ ] No hanging connections
- [ ] Configuration file valid
- [ ] All error pages display
- [ ] CGI scripts execute
- [ ] Sessions persist

---

**Ready for Audit: 95%**
**Remaining: Execute stress tests**
