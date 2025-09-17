use std::io::{self, Read, Write, ErrorKind};
use std::net::{TcpStream, SocketAddr};
use crate::http::parse::HttpParser;
use crate::http::request::HttpRequest;
use crate::http::response::HttpResponse;

pub struct Connection {
    stream: TcpStream,
    addr: SocketAddr,
    parser: HttpParser,
    write_buffer: Vec<u8>,
    write_pos: usize,
    current_request: Option<HttpRequest>,
    keep_alive: bool,
}

impl Connection {
    pub fn new(stream: TcpStream, addr: SocketAddr) -> Self {
        Connection {
            stream,
            addr,
            parser: HttpParser::new(),
            write_buffer: Vec::new(),
            write_pos: 0,
            current_request: None,
            keep_alive: false,
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
                    // Parse the incoming data
                    match self.parser.parse(&temp_buf[..n]) {
                        Ok(Some(request)) => {
                            // Request parsing complete
                            println!("Parsed request: {} {}", request.method.as_str(), request.path);
                            
                            // Validate required headers for HTTP/1.1
                            if request.version == "HTTP/1.1" && request.host().is_none() {
                                return Err(io::Error::new(ErrorKind::InvalidData, "Missing Host header for HTTP/1.1"));
                            }
                            
                            self.keep_alive = request.connection_keep_alive();
                            self.current_request = Some(request);
                            return Ok(true);
                        }
                        Ok(None) => {
                            // Need more data, continue reading
                        }
                        Err(e) => {
                            // Parse error
                            return Err(e);
                        }
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
        
        Ok(false)
    }
    
    /// Prepare and queue the HTTP response
    pub fn send_response(&mut self) -> io::Result<()> {
        let request = match &self.current_request {
            Some(req) => req,
            None => return Err(io::Error::new(ErrorKind::InvalidInput, "No request to respond to")),
        };
        
        // Generate response based on request
        let mut response = self.generate_response(request)?;
        
        // Set connection header based on keep-alive preference
        response.set_keep_alive(self.keep_alive);
        
        // Convert to bytes and queue for sending
        self.write_buffer = response.to_bytes();
        self.write_pos = 0;
        
        Ok(())
    }
    
    fn generate_response(&self, request: &HttpRequest) -> io::Result<HttpResponse> {
        // For now, just return a simple response based on the path
        match request.path.as_str() {
            "/" => {
                let mut response = HttpResponse::ok();
                response.set_body_string("Hello from Localhost HTTP Server!");
                response.set_header("Content-Type", "text/plain");
                Ok(response)
            }
            "/hello" => {
                let mut response = HttpResponse::ok();
                response.set_body_string("Hello, World!");
                response.set_header("Content-Type", "text/plain");
                Ok(response)
            }
            _ => {
                // Return 404 for unknown paths
                Ok(HttpResponse::not_found())
            }
        }
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
        
        // All data sent - check if we should keep the connection alive
        if self.keep_alive {
            // Reset for next request
            self.reset_for_next_request();
            Ok(false) // Don't close connection, wait for next request
        } else {
            Ok(true) // Close connection
        }
    }
    
    fn reset_for_next_request(&mut self) {
        self.parser.reset();
        self.write_buffer.clear();
        self.write_pos = 0;
        self.current_request = None;
        // keep_alive stays the same for the connection
    }
    
    pub fn should_keep_alive(&self) -> bool {
        self.keep_alive && self.current_request.is_none()
    }
}
