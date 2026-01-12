# Testing Guide - Localhost HTTP Server

This guide provides comprehensive testing instructions for all implemented features.

## Quick Start

1. **Start the server:**
   ```bash
   cargo run
   ```

2. **Run automated tests:**
   ```bash
   ./test_server.sh
   ```

3. **Open browser:**
   ```
   http://127.0.0.1:8080
   ```

## Manual Testing

### 1. Static File Serving (GET)

Test serving static files with proper MIME types:

```bash
# Homepage
curl -i http://127.0.0.1:8080/

# Plain text file
curl -i http://127.0.0.1:8080/hello.txt

# JSON file
curl -i http://127.0.0.1:8080/test.json

# Test upload file
curl -i http://127.0.0.1:8080/test_upload.txt
```

**Expected:** HTTP 200 OK with correct Content-Type headers

### 2. Custom Error Pages

Test custom error page rendering:

```bash
# 404 Not Found
curl -i http://127.0.0.1:8080/nonexistent-page

# 405 Method Not Allowed
curl -i -X POST http://127.0.0.1:8080/hello.txt

# 403 Forbidden (if configured)
curl -i http://127.0.0.1:8080/forbidden-path
```

**Expected:** Beautiful custom error pages with proper status codes

### 3. File Upload (POST)

Test file upload functionality:

```bash
# Create test file
echo "Hello, this is a test upload!" > test_file.txt

# Upload single file
curl -i -X POST -F "file=@test_file.txt" http://127.0.0.1:8080/upload

# Upload with custom filename
curl -i -X POST -F "file=@test_file.txt;filename=custom.txt" http://127.0.0.1:8080/upload

# Upload with description
curl -i -X POST \
  -F "file=@test_file.txt" \
  -F "description=Test upload" \
  http://127.0.0.1:8080/upload

# Upload multiple files
curl -i -X POST \
  -F "file1=@test_file.txt" \
  -F "file2=@test_upload.txt" \
  http://127.0.0.1:8080/upload
```

**Expected:** HTTP 200 OK, files saved to uploads directory

### 4. File Deletion (DELETE)

Test file deletion:

```bash
# Delete uploaded file
curl -i -X DELETE http://127.0.0.1:8080/uploads/test_file.txt

# Try to delete non-existent file
curl -i -X DELETE http://127.0.0.1:8080/uploads/nonexistent.txt
```

**Expected:** HTTP 200 OK for existing files, 404 for non-existent

### 5. Cookie & Session Management

Test session creation and management:

```bash
# Create session
curl -i -c cookies.txt http://127.0.0.1:8080/session/create

# Get session info
curl -i -b cookies.txt http://127.0.0.1:8080/session/info

# Set session data
curl -i -b cookies.txt -X POST \
  -d "value=test_data" \
  http://127.0.0.1:8080/session/set/mykey

# Get session data
curl -i -b cookies.txt http://127.0.0.1:8080/session/get/mykey

# Get session stats
curl -i http://127.0.0.1:8080/session/stats

# Destroy session
curl -i -b cookies.txt -X DELETE http://127.0.0.1:8080/session/destroy
```

**Expected:** Set-Cookie headers, session data persistence

### 6. CGI Script Execution

Test CGI scripts with different interpreters:

```bash
# Python CGI
curl -i http://127.0.0.1:8080/cgi-bin/test.py

# Shell CGI
curl -i http://127.0.0.1:8080/cgi-bin/test.sh

# Perl CGI
curl -i http://127.0.0.1:8080/cgi-bin/test.pl

# CGI with query string
curl -i "http://127.0.0.1:8080/cgi-bin/test.py?name=John&age=30"

# CGI with POST data
curl -i -X POST \
  -d "username=test&password=secret" \
  http://127.0.0.1:8080/cgi-bin/test.py
```

**Expected:** Dynamic HTML output with environment variables displayed

### 7. HTTP Redirects

Test different redirect types:

```bash
# 301 Permanent Redirect
curl -i http://127.0.0.1:8080/redirect/301/home

# 302 Temporary Redirect
curl -i http://127.0.0.1:8080/redirect/302/home

# 303 See Other (POST -> GET)
curl -i -X POST http://127.0.0.1:8080/redirect/303

# 307 Temporary Redirect (preserve method)
curl -i http://127.0.0.1:8080/redirect/307/home

# 308 Permanent Redirect (preserve method)
curl -i http://127.0.0.1:8080/redirect/308/home

# Follow redirects automatically
curl -L http://127.0.0.1:8080/redirect/301/home
```

**Expected:** Proper Location headers and status codes

### 8. HEAD Method

Test HEAD requests (headers only, no body):

```bash
# HEAD on homepage
curl -I http://127.0.0.1:8080/

# HEAD on file
curl -I http://127.0.0.1:8080/hello.txt

# Compare with GET
curl -i http://127.0.0.1:8080/hello.txt
```

**Expected:** Same headers as GET but no response body

### 9. HTTP/1.1 Features

Test keep-alive connections:

```bash
# Check Connection header
curl -i http://127.0.0.1:8080/ | grep -i connection

# Multiple requests on same connection
curl -i --http1.1 --keepalive-time 60 \
  http://127.0.0.1:8080/ \
  http://127.0.0.1:8080/hello.txt
```

**Expected:** Connection: keep-alive header

### 10. Configuration File

Test server configuration:

```bash
# View current configuration
cat server.toml

# Test with custom config
cargo run -- --config server.toml

# Test configuration validation
cargo run -- --validate-config server.toml
```

**Expected:** Server starts with configured settings

## Browser Testing

### Interactive Tests

1. **Homepage:** http://127.0.0.1:8080/
   - View all features
   - Click test cards

2. **File Upload:** http://127.0.0.1:8080/upload.html
   - Upload files via form
   - View uploaded files list
   - Delete files

3. **Redirect Tests:** http://127.0.0.1:8080/redirect-test.html
   - Test all redirect types
   - Test cookie setting during redirects

4. **CGI Scripts:**
   - http://127.0.0.1:8080/cgi-bin/test.py
   - http://127.0.0.1:8080/cgi-bin/test.sh
   - http://127.0.0.1:8080/cgi-bin/test.pl

5. **Error Pages:**
   - http://127.0.0.1:8080/nonexistent (404)
   - POST to static file (405)

## Stress Testing

Test server under load:

```bash
# Install siege (if not installed)
# macOS: brew install siege
# Linux: apt-get install siege

# Basic load test (100 concurrent users, 1 minute)
siege -c 100 -t 1M http://127.0.0.1:8080/

# File upload stress test
siege -c 50 -t 30S -f upload_urls.txt

# Mixed workload test
siege -c 200 -t 2M -f test_urls.txt
```

**Expected:** 99.5%+ availability, no crashes

## Memory Leak Testing

Test for memory leaks:

```bash
# Using valgrind (Linux)
valgrind --leak-check=full --show-leak-kinds=all \
  ./target/debug/localhost

# Using instruments (macOS)
instruments -t Leaks ./target/debug/localhost

# Long-running test
siege -c 100 -t 1H http://127.0.0.1:8080/
# Monitor memory usage with Activity Monitor or htop
```

**Expected:** No memory leaks, stable memory usage

## Error Handling Tests

Test error conditions:

```bash
# Malformed request
echo -e "INVALID REQUEST\r\n\r\n" | nc 127.0.0.1 8080

# Missing Host header (HTTP/1.1)
curl -i --http1.1 -H "Host:" http://127.0.0.1:8080/

# Oversized request body
dd if=/dev/zero bs=1M count=100 | \
  curl -i -X POST --data-binary @- http://127.0.0.1:8080/upload

# Timeout test (slow client)
(echo -n "GET / HTTP/1.1\r\n"; sleep 10; echo -e "\r\n") | \
  nc 127.0.0.1 8080
```

**Expected:** Proper error responses, no crashes

## Performance Benchmarks

Benchmark server performance:

```bash
# Apache Bench
ab -n 10000 -c 100 http://127.0.0.1:8080/

# wrk (modern alternative)
wrk -t 4 -c 100 -d 30s http://127.0.0.1:8080/

# Test file upload performance
ab -n 1000 -c 10 -p test_file.txt http://127.0.0.1:8080/upload
```

**Expected:** High throughput, low latency

## Security Tests

Test security features:

```bash
# Directory traversal attempt
curl -i http://127.0.0.1:8080/../../../etc/passwd

# Path normalization
curl -i http://127.0.0.1:8080/./././hello.txt

# XSS in error pages
curl -i "http://127.0.0.1:8080/<script>alert('xss')</script>"

# Check security headers
curl -i http://127.0.0.1:8080/ | grep -E "X-Frame-Options|X-Content-Type-Options"
```

**Expected:** Attacks blocked, security headers present

## Troubleshooting

### Server won't start
- Check if port 8080 is already in use: `lsof -i :8080`
- Verify configuration file syntax: `cargo run -- --validate-config`

### File uploads fail
- Check upload directory permissions: `ls -la ./uploads`
- Verify file size limits in server.toml
- Check disk space: `df -h`

### CGI scripts don't execute
- Verify scripts are executable: `chmod +x www/cgi-bin/*.py`
- Check interpreter paths in scripts
- Test scripts directly: `./www/cgi-bin/test.py`

### Memory issues
- Monitor with: `ps aux | grep localhost`
- Check for leaks: `valgrind --leak-check=full`
- Review timeout settings in server.toml

## Test Checklist

- [ ] Static file serving (GET)
- [ ] Custom error pages (400, 403, 404, 405, 413, 500)
- [ ] File upload (POST with multipart/form-data)
- [ ] File deletion (DELETE)
- [ ] Cookie setting in response headers
- [ ] Session management (create, read, update, delete)
- [ ] CGI script execution (Python, Perl, Shell)
- [ ] HTTP redirects (301, 302, 303, 307, 308)
- [ ] HEAD method support
- [ ] Keep-alive connections
- [ ] Configuration file loading
- [ ] Timeout management
- [ ] Error handling
- [ ] Security features
- [ ] Performance under load
- [ ] Memory leak testing

## Success Criteria

✅ All automated tests pass
✅ 99.5%+ availability under siege testing
✅ No memory leaks during extended operation
✅ Proper error handling for all edge cases
✅ Security features working correctly
✅ All HTTP methods implemented correctly
✅ Configuration file properly loaded
✅ Beautiful error pages displayed
✅ CGI scripts execute successfully
✅ File uploads/deletes work correctly
✅ Sessions persist across requests
✅ Redirects work with proper status codes
