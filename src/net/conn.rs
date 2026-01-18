use std::io::{self, Read, Write, ErrorKind};
use std::net::{TcpStream, SocketAddr};
use crate::http::parse::HttpParser;
use crate::http::request::{HttpRequest, Method};
use crate::http::response::HttpResponse;
use crate::fs::static_files::StaticFileServer;
use crate::routing::router::{Router, VirtualHost};
use crate::routing::route::RouteConfig;
use crate::config::server::VirtualHostConfig;
use crate::session::{SessionStore, CookieJar};
use std::collections::HashMap;
use std::path::Path;

pub struct Connection {
    stream: TcpStream,
    addr: SocketAddr,
    parser: HttpParser,
    write_buffer: Vec<u8>,
    write_pos: usize,
    current_request: Option<HttpRequest>,
    keep_alive: bool,
    static_server: StaticFileServer,
    router: Router,
    session_store: SessionStore,
    vhost_config: Option<VirtualHostConfig>,
}

impl Connection {
    pub fn new(stream: TcpStream, addr: SocketAddr) -> io::Result<Self> {
        Self::new_with_config(stream, addr, None, None)
    }
    
    pub fn new_with_config(
        stream: TcpStream,
        addr: SocketAddr,
        vhost_config: Option<VirtualHostConfig>,
        session_store: Option<SessionStore>,
    ) -> io::Result<Self> {
        // Determine document root from config or use default
        let document_root = vhost_config.as_ref()
            .map(|c| c.document_root.to_string_lossy().to_string())
            .unwrap_or_else(|| "./www".to_string());
        
        // Create static file server
        let static_server = StaticFileServer::new(&document_root, None)
            .unwrap_or_else(|_| {
                StaticFileServer::new("./www", None)
                    .expect("Failed to create static file server")
            });
        
        // Create router with configuration
        let mut router = Router::new();
        
        // Configure virtual host from config or use default
        if let Some(ref config) = vhost_config {
            let vhost = Self::config_to_vhost(config);
            router.add_virtual_host(vhost);
        } else {
            // Add default virtual host with default route allowing all methods
            use std::collections::HashSet;
            
            // Route 1: Root - GET/HEAD only
            let mut root_methods = HashSet::new();
            root_methods.insert(Method::GET);
            root_methods.insert(Method::HEAD);
            
            let root_route = RouteConfig {
                path: "/".to_string(),
                allowed_methods: root_methods,
                document_root: None,
                index_file: Some("index.html".to_string()),
                directory_listing: false,
                redirect: None,
                cgi_extension: None,
                max_body_size: Some(10 * 1024 * 1024),
                error_pages: HashMap::new(),
            };
            
            // Route 2: Uploads - GET/DELETE
            let mut upload_methods = HashSet::new();
            upload_methods.insert(Method::GET);
            upload_methods.insert(Method::DELETE);
            
            let uploads_route = RouteConfig {
                path: "/uploads/*".to_string(),
                allowed_methods: upload_methods,
                document_root: None,
                index_file: None,
                directory_listing: false,
                redirect: None,
                cgi_extension: None,
                max_body_size: Some(10 * 1024 * 1024),
                error_pages: HashMap::new(),
            };
            
            // Route 3: Upload endpoint - POST
            let mut upload_post_methods = HashSet::new();
            upload_post_methods.insert(Method::POST);
            upload_post_methods.insert(Method::GET);
            
            let upload_endpoint_route = RouteConfig {
                path: "/upload".to_string(),
                allowed_methods: upload_post_methods,
                document_root: None,
                index_file: None,
                directory_listing: false,
                redirect: None,
                cgi_extension: None,
                max_body_size: Some(10 * 1024 * 1024),
                error_pages: HashMap::new(),
            };
            
            // Route 4: Session endpoints - GET/POST/DELETE
            let mut session_methods = HashSet::new();
            session_methods.insert(Method::GET);
            session_methods.insert(Method::POST);
            session_methods.insert(Method::DELETE);
            
            let session_route = RouteConfig {
                path: "/session/*".to_string(),
                allowed_methods: session_methods,
                document_root: None,
                index_file: None,
                directory_listing: false,
                redirect: None,
                cgi_extension: None,
                max_body_size: Some(10 * 1024 * 1024),
                error_pages: HashMap::new(),
            };
            
            let default_vhost = VirtualHost {
                server_name: "localhost".to_string(),
                routes: vec![uploads_route, upload_endpoint_route, session_route, root_route],
                document_root: "./www".to_string(),
                error_pages: HashMap::new(),
                max_body_size: 10 * 1024 * 1024,
            };
            router.add_virtual_host(default_vhost);
        }
        
        // Use provided session store or create default
        let session_store = session_store.unwrap_or_else(|| {
            use crate::session::SessionConfig;
            SessionStore::new(SessionConfig::default())
        });
        
        Ok(Connection {
            stream,
            addr,
            parser: HttpParser::new(),
            write_buffer: Vec::new(),
            write_pos: 0,
            current_request: None,
            keep_alive: true,
            static_server,
            router,
            session_store,
            vhost_config,
        })
    }
    
    /// Convert VirtualHostConfig to VirtualHost for router
    fn config_to_vhost(config: &VirtualHostConfig) -> VirtualHost {
        let mut error_pages = HashMap::new();
        for (code, path) in &config.error_pages {
            error_pages.insert(*code, path.to_string_lossy().to_string());
        }
        
        // Convert routes from config
        let routes = config.routes.iter().map(|r| {
            Self::convert_route_config(r)
        }).collect();
        
        VirtualHost {
            server_name: config.server_name.clone(),
            routes,
            document_root: config.document_root.to_string_lossy().to_string(),
            error_pages,
            max_body_size: config.max_body_size,
        }
    }
    
    /// Convert config RouteConfig to routing RouteConfig
    fn convert_route_config(config: &crate::config::server::RouteConfig) -> RouteConfig {
        use crate::http::request::Method;
        use std::collections::HashSet;
        
        let mut allowed_methods = HashSet::new();
        
        // If no methods specified, allow all common methods
        if config.methods.is_empty() {
            allowed_methods.insert(Method::GET);
            allowed_methods.insert(Method::POST);
            allowed_methods.insert(Method::DELETE);
            allowed_methods.insert(Method::HEAD);
        } else {
            for method_str in &config.methods {
                match method_str.to_uppercase().as_str() {
                    "GET" => { allowed_methods.insert(Method::GET); }
                    "POST" => { allowed_methods.insert(Method::POST); }
                    "DELETE" => { allowed_methods.insert(Method::DELETE); }
                    "PUT" => { allowed_methods.insert(Method::PUT); }
                    "HEAD" => { allowed_methods.insert(Method::HEAD); }
                    "OPTIONS" => { allowed_methods.insert(Method::OPTIONS); }
                    _ => {}
                }
            }
        }
        
        RouteConfig {
            path: config.path.clone(),
            allowed_methods,
            document_root: None,
            index_file: None,
            directory_listing: false,
            redirect: None,
            cgi_extension: None,
            max_body_size: config.settings.max_body_size,
            error_pages: config.settings.error_pages.clone(),
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
                            
                            // Debug: Print all headers to see what we're receiving
                            println!("Request headers:");
                            for (name, value) in &request.headers {
                                println!("  {}: {}", name, value);
                            }
                            
                            // Validate required headers for HTTP/1.1 (be more lenient for local development)
                            if request.version == "HTTP/1.1" {
                                match request.host() {
                                    Some(host) => {
                                        println!("Host header found: {}", host);
                                    }
                                    None => {
                                        // For local development, allow requests without Host header
                                        // or provide a default host
                                        println!("Warning: No Host header found, using default localhost");
                                        // Don't fail the request - HTTP/1.1 spec allows this for local servers
                                    }
                                }
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
            Some(req) => req.clone(),
            None => return Err(io::Error::new(ErrorKind::InvalidInput, "No request to respond to")),
        };
        
        // Generate response based on request
        let mut response = self.generate_response(&request)?;
        
        // Set connection header based on keep-alive preference
        response.set_keep_alive(self.keep_alive);
        
        // Convert to bytes and queue for sending
        self.write_buffer = response.to_bytes();
        self.write_pos = 0;
        
        Ok(())
    }
    
    fn generate_response(&mut self, request: &HttpRequest) -> io::Result<HttpResponse> {
        // Parse cookies and handle session middleware
        let cookies = if let Some(cookie_header) = request.get_header("Cookie") {
            CookieJar::parse_cookie_header(cookie_header)
        } else {
            CookieJar::new()
        };
        
        // Check for session and update activity
        if let Some(session) = self.session_store.get_session_from_cookies(&cookies) {
            // Update session activity
            let _ = self.session_store.update_session(session);
        }
        
        // Route request based on method and path
        let response = match request.method {
            Method::GET | Method::HEAD => {
                self.handle_get_request(request)
            }
            Method::POST => {
                self.handle_post_request(request)
            }
            Method::DELETE => {
                self.handle_delete_request(request)
            }
            _ => {
                // Other methods not supported - return 405
                let mut response = HttpResponse::new(405);
                response.set_body_string("405 Method Not Allowed");
                response.set_header("Content-Type", "text/plain");
                response.set_header("Allow", "GET, HEAD, POST, DELETE");
                Ok(response)
            }
        }?;
        
        Ok(response)
    }
    
    fn handle_get_request(&mut self, request: &HttpRequest) -> io::Result<HttpResponse> {
        let path = request.path();
        
        // Check if this is a session endpoint
        if path.starts_with("/session/") {
            return self.router.route_request(request);
        }
        
        // Check if path is a CGI script
        let document_root = self.vhost_config.as_ref()
            .map(|c| c.document_root.as_path())
            .unwrap_or_else(|| Path::new("./www"));
        
        let file_path = document_root.join(path.trim_start_matches('/'));
        
        // Try router first for CGI and special routes
        if file_path.exists() && self.is_cgi_path(&file_path) {
            return self.router.route_request(request);
        }
        
        // Serve static files
        match self.static_server.serve_file(path) {
            Ok(mut response) => {
                // For HEAD requests, remove the body but keep headers
                if matches!(request.method, Method::HEAD) {
                    response.body.clear();
                    response.set_header("Content-Length", "0");
                }
                Ok(response)
            }
            Err(_) => {
                // Try router as fallback (might have custom error pages)
                self.router.route_request(request)
            }
        }
    }
    
    fn handle_post_request(&mut self, request: &HttpRequest) -> io::Result<HttpResponse> {
        // Route all POST requests through the router
        // This handles uploads, forms, CGI POST, and session operations
        self.router.route_request(request)
    }
    
    fn handle_delete_request(&mut self, request: &HttpRequest) -> io::Result<HttpResponse> {
        // Route all DELETE requests through the router
        // This handles file deletion with proper security checks
        self.router.route_request(request)
    }
    
    fn is_cgi_path(&self, path: &Path) -> bool {
        // Check if path is in cgi-bin directory or has CGI extension
        if let Some(parent) = path.parent() {
            if parent.ends_with("cgi-bin") {
                return true;
            }
        }
        
        // Check for CGI extensions
        if let Some(ext) = path.extension() {
            if let Some(ext_str) = ext.to_str() {
                return matches!(ext_str, "py" | "pl" | "sh" | "rb" | "php" | "cgi");
            }
        }
        
        false
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
        self.keep_alive
    }
    
    /// Check if the connection is currently reading request body
    pub fn is_reading_body(&self) -> bool {
        self.parser.is_reading_body()
    }
}
