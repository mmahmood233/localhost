use std::io::{self, Read, Write, ErrorKind};
use std::net::{TcpStream, SocketAddr};

pub struct Connection {
    stream: TcpStream,
    addr: SocketAddr,
    read_buffer: Vec<u8>,
    write_buffer: Vec<u8>,
    write_pos: usize,
    request_complete: bool,
}

impl Connection {
    pub fn new(stream: TcpStream, addr: SocketAddr) -> Self {
        Connection {
            stream,
            addr,
            read_buffer: Vec::with_capacity(8192),
            write_buffer: Vec::new(),
            write_pos: 0,
            request_complete: false,
        }
    }
    
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }
    
    /// Handle read event. Returns Ok(true) if request is complete, Ok(false) if more data needed
    pub fn handle_read(&mut self) -> io::Result<bool> {
        let mut temp_buf = [0u8; 4096];
        
        loop {
            match self.stream.read(&mut temp_buf) {
                Ok(0) => {
                    // EOF - connection closed by client
                    return Err(io::Error::new(ErrorKind::UnexpectedEof, "Client closed connection"));
                }
                Ok(n) => {
                    self.read_buffer.extend_from_slice(&temp_buf[..n]);
                    
                    // Simple check for end of HTTP headers (minimal for now)
                    if self.read_buffer.windows(4).any(|w| w == b"\r\n\r\n") {
                        self.request_complete = true;
                        return Ok(true);
                    }
                    
                    // Prevent buffer from growing too large
                    if self.read_buffer.len() > 64 * 1024 {
                        return Err(io::Error::new(ErrorKind::InvalidData, "Request too large"));
                    }
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    // No more data available right now
                    break;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
        
        Ok(self.request_complete)
    }
    
    /// Prepare and queue the HTTP response
    pub fn send_response(&mut self) -> io::Result<()> {
        if !self.request_complete {
            return Err(io::Error::new(ErrorKind::InvalidInput, "Request not complete"));
        }
        
        // Simple HTTP/1.1 response
        let response = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\nServer: Localhost\r\n\r\nHello";
        
        self.write_buffer = response.to_vec();
        self.write_pos = 0;
        
        Ok(())
    }
    
    /// Handle write event. Returns Ok(true) if all data sent, Ok(false) if more data to send
    pub fn handle_write(&mut self) -> io::Result<bool> {
        while self.write_pos < self.write_buffer.len() {
            match self.stream.write(&self.write_buffer[self.write_pos..]) {
                Ok(0) => {
                    // Connection closed by peer
                    return Err(io::Error::new(ErrorKind::WriteZero, "Write zero bytes"));
                }
                Ok(n) => {
                    self.write_pos += n;
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    // Can't write more right now
                    return Ok(false);
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
        
        // All data sent
        Ok(true)
    }
}
