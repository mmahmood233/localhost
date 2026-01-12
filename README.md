# Localhost - Minimalist HTTP/1.1 Server in Rust

A production-ready, single-threaded HTTP/1.1 server built in Rust using non-blocking I/O and platform-native event loops (kqueue on macOS/BSD, epoll on Linux).

## ✨ Features

### Core HTTP Server
- ✅ **Single-threaded, non-blocking I/O** - kqueue (macOS) / epoll (Linux)
- ✅ **HTTP/1.1 compliant** - Full request parsing and response generation
- ✅ **Keep-alive connections** - Persistent connections with timeout management
- ✅ **Static file serving** - MIME type detection for 20+ file types
- ✅ **HEAD method support** - Headers-only responses

### Advanced Features
- ✅ **POST/DELETE methods** - Full HTTP method support
- ✅ **File uploads** - multipart/form-data parsing with size limits
- ✅ **Cookie & Session management** - Secure session handling with cleanup
- ✅ **CGI support** - Python, Perl, Shell, Ruby, PHP script execution
- ✅ **HTTP redirects** - 301, 302, 303, 307, 308 redirect types
- ✅ **Chunked transfer encoding** - Streaming request/response support
- ✅ **Multiple listeners** - Virtual host support with default selection

### Configuration & Management
- ✅ **TOML configuration** - Comprehensive server.toml with validation
- ✅ **Custom error pages** - Beautiful error pages (400, 403, 404, 405, 413, 500)
- ✅ **Directory listing** - Automatic directory browsing
- ✅ **Timeout management** - 5 timeout types (header, body, write, keepalive, request)
- ✅ **Security headers** - X-Frame-Options, X-Content-Type-Options, HSTS, CSP

### Testing & Quality
- ✅ **Comprehensive test suite** - Automated testing script included
- ✅ **Stress tested** - Designed for 99.5%+ availability under load
- ✅ **Memory safe** - No leaks, proper resource cleanup
- ✅ **Error handling** - Graceful handling of all edge cases

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

### Quick Test
```bash
# Open browser
open http://127.0.0.1:8080

# Or test with curl
curl http://127.0.0.1:8080/

# Run automated test suite
./test_server.sh
```

## 🧪 Testing

### Automated Tests
```bash
# Run comprehensive test suite
./test_server.sh

# Expected output: All tests pass ✅
```

### Manual Testing

**Static Files:**
```bash
curl http://127.0.0.1:8080/hello.txt
curl http://127.0.0.1:8080/test.json
```

**File Upload:**
```bash
echo "test content" > test.txt
curl -X POST -F "file=@test.txt" http://127.0.0.1:8080/upload
```

**File Delete:**
```bash
curl -X DELETE http://127.0.0.1:8080/uploads/test.txt
```

**CGI Scripts:**
```bash
curl http://127.0.0.1:8080/cgi-bin/test.py
curl http://127.0.0.1:8080/cgi-bin/test.sh
curl http://127.0.0.1:8080/cgi-bin/test.pl
```

**Sessions:**
```bash
curl -c cookies.txt http://127.0.0.1:8080/session/create
curl -b cookies.txt http://127.0.0.1:8080/session/info
```

**Redirects:**
```bash
curl -i http://127.0.0.1:8080/redirect/301/home
curl -i http://127.0.0.1:8080/redirect/302/home
```

See [TESTING.md](TESTING.md) for comprehensive testing guide.

### Stress Testing
```bash
# Install siege
brew install siege  # macOS
apt-get install siege  # Linux

# Run stress test
siege -c 100 -t 1M http://127.0.0.1:8080/
```

**Target:** 99.5%+ availability

## Architecture

### Project Structure
```
localhost/
├── src/
│   ├── main.rs                 # Entry point and server startup
│   ├── net/
│   │   ├── mod.rs             # Network module exports
│   │   ├── event_loop.rs      # kqueue/epoll event loop
│   │   └── conn.rs            # Connection state management
│   ├── http/
│   │   ├── mod.rs             # HTTP module exports
│   │   ├── request.rs         # HTTP request parsing
│   │   ├── response.rs        # HTTP response generation
│   │   ├── parse.rs           # HTTP parser state machine
│   │   └── headers.rs         # Header parsing utilities
│   ├── routing/
│   │   ├── mod.rs             # Routing module exports
│   │   ├── router.rs          # Request routing logic
│   │   ├── route.rs           # Route configuration
│   │   ├── handler.rs         # Request handlers
│   │   └── redirections.rs    # Redirect rules
│   ├── fs/
│   │   ├── mod.rs             # Filesystem module
│   │   └── static_files.rs    # Static file serving
│   ├── upload/
│   │   ├── mod.rs             # Upload module exports
│   │   ├── multipart.rs       # Multipart form parsing
│   │   ├── form_data.rs       # URL-encoded forms
│   │   └── file_storage.rs    # File storage management
│   ├── session/
│   │   ├── mod.rs             # Session module exports
│   │   ├── store.rs           # Session storage
│   │   └── cookie.rs          # Cookie parsing
│   ├── cgi/
│   │   ├── mod.rs             # CGI module exports
│   │   ├── executor.rs        # CGI script execution
│   │   ├── environment.rs     # CGI environment variables
│   │   └── response.rs        # CGI response parsing
│   ├── config/
│   │   ├── mod.rs             # Config module exports
│   │   ├── parser.rs          # TOML parser
│   │   ├── server.rs          # Server configuration
│   │   └── validation.rs      # Config validation
│   └── timeout/
│       ├── mod.rs             # Timeout module exports
│       └── manager.rs         # Timeout management
├── www/
│   ├── index.html             # Homepage
│   ├── upload.html            # File upload interface
│   ├── redirect-test.html     # Redirect testing page
│   ├── style.css              # Stylesheet
│   ├── script.js              # Client-side JavaScript
│   ├── error_pages/           # Custom error pages
│   │   ├── 400.html
│   │   ├── 403.html
│   │   ├── 404.html
│   │   ├── 405.html
│   │   ├── 413.html
│   │   └── 500.html
│   └── cgi-bin/               # CGI scripts
│       ├── test.py            # Python CGI test
│       ├── test.sh            # Shell CGI test
│       └── test.pl            # Perl CGI test
├── server.toml                # Server configuration
├── test_server.sh             # Automated test suite
├── TESTING.md                 # Testing documentation
└── README.md                  # This file
```

### Implementation Details

**Event Loop**
- Platform-native event notification: `kqueue` (macOS/BSD) or `epoll` (Linux)
- Non-blocking socket operations with proper `EAGAIN`/`EWOULDBLOCK` handling
- Edge-triggered event processing for maximum performance
- Connection state machine with timeout tracking

**HTTP Parser**
- Incremental parsing with state machine
- Support for chunked transfer encoding
- Header validation and normalization
- Body size limits and security checks

**Routing System**
- Virtual host support with pattern matching
- Route-specific configuration (methods, body size, etc.)
- Handler chain for request processing
- Redirect rules with conditions

**File Upload**
- Multipart/form-data parsing
- Streaming upload support
- File size limits and extension validation
- Secure filename sanitization

**Session Management**
- In-memory session storage
- Automatic cleanup of expired sessions
- Secure session ID generation
- Cookie-based session tracking

**CGI Execution**
- Process forking with timeout protection
- Environment variable setup per CGI/1.1 spec
- Support for multiple interpreters
- Output size limits

## Configuration

The server uses a TOML configuration file (`server.toml`) with comprehensive settings:

```toml
[server]
name = "Localhost"
version = "1.0.0"

[global]
max_body_size = 10485760  # 10MB
request_timeout = 30
keep_alive_timeout = 10

[[listener]]
address = "127.0.0.1"
port = 8080
default = true

[[vhost]]
server_name = "localhost"
document_root = "./www"
default = true
```

See `server.toml` for full configuration options.

## Browser Testing

Open http://127.0.0.1:8080 in your browser to access:

- **Homepage** - Feature overview with test cards
- **File Upload** - Interactive upload/delete interface
- **Redirect Tests** - Test all redirect types (301, 302, 303, 307, 308)
- **CGI Scripts** - Python, Shell, and Perl CGI examples
- **Error Pages** - Beautiful custom error pages

## Performance

**Design Goals:**
- 99.5%+ availability under load
- Low memory footprint
- Efficient static file serving
- Proper resource cleanup

**Benchmarks:**
```bash
# Run with siege
siege -c 100 -t 1M http://127.0.0.1:8080/

# Run with Apache Bench
ab -n 10000 -c 100 http://127.0.0.1:8080/

# Run with wrk
wrk -t 4 -c 100 -d 30s http://127.0.0.1:8080/
```

## Security

**Implemented Security Features:**
- Directory traversal protection
- Filename sanitization
- File size limits
- Extension validation
- Security headers (X-Frame-Options, X-Content-Type-Options, etc.)
- Timeout protection against slowloris attacks
- CGI execution timeouts
- Session security (HttpOnly, Secure flags)

## HTTP/1.1 Compliance

- ✅ Request parsing and validation
- ✅ Keep-alive connections
- ✅ Chunked transfer encoding
- ✅ Content-Length handling
- ✅ Multiple HTTP methods (GET, POST, DELETE, HEAD)
- ✅ Proper status codes and headers
- ✅ Host header validation
- ✅ Connection management

## Troubleshooting

**Port already in use:**
```bash
lsof -i :8080
kill -9 <PID>
```

**CGI scripts not executing:**
```bash
chmod +x www/cgi-bin/*.py
chmod +x www/cgi-bin/*.sh
chmod +x www/cgi-bin/*.pl
```

**File upload fails:**
```bash
mkdir -p uploads
chmod 755 uploads
```

## License

This project is built for educational purposes as part of an HTTP server implementation exercise.

---

**Status**: ✅ Production Ready - Full HTTP/1.1 server with all features implemented
