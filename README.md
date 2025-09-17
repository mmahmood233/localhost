# Localhost - Minimalist HTTP/1.1 Server in Rust

A high-performance, single-threaded HTTP/1.1 server built in Rust using non-blocking I/O and platform-native event loops (kqueue on macOS/BSD, epoll on Linux).

## Features (Current Implementation)

✅ **M0 Complete**: Minimal event-driven HTTP server
- Single-threaded, non-blocking I/O using kqueue (macOS) or epoll (Linux)
- Basic HTTP/1.1 response generation
- Connection management with proper cleanup
- Graceful handling of client connections and disconnections

## Planned Features

- [ ] HTTP/1.1 request parsing with keep-alive support
- [ ] Multiple port listeners and virtual host support
- [ ] Configuration file loader (custom syntax)
- [ ] Static file serving with MIME type detection
- [ ] Request routing and method handling (GET/POST/DELETE)
- [ ] File uploads (multipart/form-data and chunked)
- [ ] Cookie and session management
- [ ] CGI support (.py scripts)
- [ ] Request timeouts and connection limits
- [ ] Error pages (400/403/404/405/413/500)

## Build and Run

### Prerequisites
- Rust (stable toolchain)
- macOS, Linux, or other Unix-like system

### Build
```bash
cargo build
```

### Run
```bash
cargo run
```

The server will start listening on `127.0.0.1:8080`.

### Test
```bash
# Basic connectivity test
curl -v http://127.0.0.1:8080/

# Expected response:
# HTTP/1.1 200 OK
# Content-Length: 5
# Connection: close
# Server: Localhost
# 
# Hello
```

## Architecture

### Project Structure
```
src/
├── main.rs              # Entry point
├── server.rs            # Server bootstrap and coordination
├── net/
│   ├── mod.rs          # Network module exports
│   ├── epoll.rs        # Event loop (kqueue/epoll abstraction)
│   └── conn.rs         # Connection state management
├── http/               # HTTP protocol implementation (planned)
├── config/             # Configuration parsing (planned)
└── errors.rs           # Error handling (planned)
```

### Current Implementation Details

**Event Loop (Member A Responsibility)**
- Uses `kqueue` on macOS/BSD systems for efficient event notification
- Non-blocking socket operations with proper `EAGAIN`/`EWOULDBLOCK` handling
- Edge-triggered event processing for maximum performance
- Connection state machine: ACCEPT → READ → PROCESS → WRITE → CLOSE

**Connection Management**
- Each connection maintains separate read/write buffers
- Handles partial reads and writes gracefully
- Automatic cleanup on connection close or error
- Simple HTTP header detection (looks for `\r\n\r\n` terminator)

## Development Roadmap

### Next Steps (Member A)
1. **M1**: HTTP parser + keep-alive + static GET + errors + timeouts
2. **M2**: Config loader + multi-listener + vhost selection
3. **M4**: Body handling (Content-Length + chunked transfer encoding)

### Integration Points for Member B
- `HttpRequest`/`HttpResponse` types (to be implemented)
- Router interface `route::resolve(req) -> RouteAction`
- Body handoff for multipart/form-data processing
- CGI bridge `cgi::invoke(req, script_path, env) -> StreamedResponse`

## Performance Goals
- Handle high connection loads with `siege -b` stress testing
- Maintain low memory footprint
- Zero-copy operations where possible
- Efficient static file serving (with optional `sendfile` optimization)

## Compliance
- HTTP/1.1 specification compliance for basic features
- Proper status codes and headers
- Connection keep-alive support
- Chunked transfer encoding support (planned)

---

**Status**: M0 Complete ✅ - Minimal kqueue-based HTTP server operational
