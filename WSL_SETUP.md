# WSL Setup Guide for Member B (Windows Development)

This guide helps Member B set up the Localhost HTTP server project on Windows using WSL (Windows Subsystem for Linux).

## Prerequisites

1. **Windows 10/11** with WSL2 enabled
2. **WSL2** with Ubuntu 20.04+ or similar Linux distribution
3. **Git** for version control

## WSL Installation (if not already installed)

```powershell
# Run in PowerShell as Administrator
wsl --install
# Or install specific distribution:
wsl --install -d Ubuntu-22.04
```

## Setting Up the Development Environment

### 1. Install Rust in WSL

```bash
# Update system packages
sudo apt update && sudo apt upgrade -y

# Install build essentials
sudo apt install -y build-essential curl git

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Verify installation
rustc --version
cargo --version
```

### 2. Clone and Build the Project

```bash
# Clone the repository (replace with actual repo URL)
git clone <repository-url> localhost
cd localhost

# Build the project (will use epoll on Linux/WSL)
cargo build

# Run the server
cargo run
```

### 3. Test the Server

```bash
# In another WSL terminal or Windows terminal
curl -v http://127.0.0.1:8080/

# Expected output:
# HTTP/1.1 200 OK
# Content-Length: 5
# Connection: close
# Server: Localhost
# 
# Hello
```

## Cross-Platform Compatibility

The project automatically detects the platform and uses:
- **epoll** on Linux/WSL (Member B)
- **kqueue** on macOS/BSD (Member A)

Both implementations provide identical functionality and performance characteristics.

## Development Workflow

### File Sharing Between Windows and WSL

```bash
# Access Windows files from WSL
cd /mnt/c/Users/YourUsername/Projects/

# Access WSL files from Windows
# Navigate to: \\wsl$\Ubuntu-22.04\home\username\localhost
```

### Recommended Setup

1. **Keep project in WSL filesystem** for better performance
2. **Use VS Code with WSL extension** for seamless development
3. **Git operations work identically** between WSL and Windows

### VS Code Integration

```bash
# Install VS Code WSL extension, then:
code .  # Opens project in VS Code from WSL
```

## Performance Notes

- **WSL2 provides native Linux performance** for our server
- **All libc system calls work identically** to native Linux
- **Network performance is excellent** for local development
- **File I/O performance is optimal** when files are in WSL filesystem

## Troubleshooting

### Port Access Issues
```bash
# If port 8080 is blocked, check Windows Firewall
# Or use a different port by modifying server.rs
```

### Build Issues
```bash
# If libc compilation fails:
sudo apt install -y libc6-dev

# If linking fails:
sudo apt install -y gcc
```

### Network Issues
```bash
# Test if server is accessible from Windows:
# From Windows Command Prompt or PowerShell:
curl http://localhost:8080/
```

## Member B Development Focus

As Member B, you'll be working on:
- **Routing system** (`src/route/`)
- **File uploads** (`src/upload/`)
- **Session management** (`src/session.rs`)
- **CGI implementation** (`src/cgi/`)
- **Directory listing** (`src/dirlist.rs`)

All these components will work identically on WSL as they do on macOS, thanks to our cross-platform event loop abstraction.

## Integration Testing

```bash
# Run integration tests (when implemented)
cargo test

# Run with specific features
cargo build --features "debug-logging"

# Performance testing with siege (install first)
sudo apt install siege
siege -b http://127.0.0.1:8080/
```

---

**Ready to Code!** 🚀

The cross-platform abstraction ensures that both Member A (macOS) and Member B (WSL) can work on the same codebase without any compatibility issues.
