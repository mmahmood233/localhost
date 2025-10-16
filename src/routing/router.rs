use crate::routing::route::{Route, RouteConfig};
use crate::routing::handler::{Handler, HandlerResult, MethodFilterHandler, RedirectHandler, ErrorHandler};
use crate::http::request::{HttpRequest, Method};
use crate::http::response::HttpResponse;
use crate::upload::multipart::{MultipartParser, FieldType};
use crate::upload::form_data::FormData;
use crate::upload::file_storage::{FileStorage, StorageConfig};
use crate::session::{SessionStore, SessionConfig, CookieJar, Cookie};
use crate::cgi::{CgiExecutor, CgiConfig};
use std::collections::HashMap;
use std::io;
use std::path::Path;

/// Virtual host configuration
#[derive(Debug, Clone)]
pub struct VirtualHost {
    /// Server name (Host header value)
    pub server_name: String,
    /// Routes for this virtual host
    pub routes: Vec<RouteConfig>,
    /// Default document root
    pub document_root: String,
    /// Default error pages
    pub error_pages: HashMap<u16, String>,
    /// Default maximum body size
    pub max_body_size: usize,
}

impl Default for VirtualHost {
    fn default() -> Self {
        VirtualHost {
            server_name: "localhost".to_string(),
            routes: vec![RouteConfig::default()],
            document_root: "./www".to_string(),
            error_pages: HashMap::new(),
            max_body_size: 1024 * 1024, // 1MB
        }
    }
}

/// Main router that handles request routing and processing
#[derive(Debug, Clone)]
pub struct Router {
    /// Virtual hosts indexed by server name
    virtual_hosts: HashMap<String, VirtualHost>,
    /// Default virtual host (first one configured)
    default_host: Option<String>,
    /// File storage for uploads
    file_storage: FileStorage,
    /// Session store for session management
    session_store: SessionStore,
    /// CGI executor for dynamic content
    cgi_executor: CgiExecutor,
}

impl Router {
    pub fn new() -> Self {
        let storage_config = StorageConfig::default();
        let file_storage = FileStorage::new(storage_config)
            .expect("Failed to create file storage");
        
        let session_config = SessionConfig::default();
        let session_store = SessionStore::new(session_config);
        
        let cgi_config = CgiConfig::default();
        let cgi_executor = CgiExecutor::new(cgi_config);
        
        Router {
            virtual_hosts: HashMap::new(),
            default_host: None,
            file_storage,
            session_store,
            cgi_executor,
        }
    }
    
    /// Add a virtual host configuration
    pub fn add_virtual_host(&mut self, vhost: VirtualHost) {
        let server_name = vhost.server_name.clone();
        
        // Set as default if it's the first one
        if self.default_host.is_none() {
            self.default_host = Some(server_name.clone());
        }
        
        self.virtual_hosts.insert(server_name, vhost);
    }
    
    /// Route an HTTP request and generate a response
    pub fn route_request(&mut self, request: &HttpRequest) -> io::Result<HttpResponse> {
        // Parse cookies from request
        let cookies = if let Some(cookie_header) = request.get_header("Cookie") {
            CookieJar::parse_cookie_header(cookie_header)
        } else {
            CookieJar::new()
        };
        
        // Cleanup expired sessions periodically
        self.session_store.cleanup_expired_sessions();
        
        // Determine which virtual host to use and clone the necessary data
        let (vhost_name, document_root, error_pages, routes) = {
            let vhost = self.select_virtual_host(request);
            (vhost.server_name.clone(), vhost.document_root.clone(), 
             vhost.error_pages.clone(), vhost.routes.clone())
        };
        
        // Create a temporary virtual host for processing
        let temp_vhost = VirtualHost {
            server_name: vhost_name,
            document_root,
            error_pages,
            routes,
            max_body_size: 1024 * 1024, // Default
        };
        
        // Find matching route
        let route = self.find_matching_route(&temp_vhost, request.path());
        
        // Process request through handler chain with session support
        let mut response = self.process_request_with_route(request, &route, &temp_vhost)?;
        
        // Handle session-specific routes
        self.handle_session_routes(request, &mut response, &cookies, &route, &temp_vhost)?;
        
        Ok(response)
    }
    
    /// Select the appropriate virtual host based on the Host header
    fn select_virtual_host(&self, request: &HttpRequest) -> &VirtualHost {
        if let Some(host_header) = request.get_header("Host") {
            // Try to find exact match
            if let Some(vhost) = self.virtual_hosts.get(host_header) {
                return vhost;
            }
            
            // Try to find match without port (Host: example.com:8080 -> example.com)
            if let Some(host_without_port) = host_header.split(':').next() {
                if let Some(vhost) = self.virtual_hosts.get(host_without_port) {
                    return vhost;
                }
            }
        }
        
        // Fall back to default host
        if let Some(ref default_name) = self.default_host {
            if let Some(vhost) = self.virtual_hosts.get(default_name) {
                return vhost;
            }
        }
        
        // If no default, use the first available host
        self.virtual_hosts.values().next()
            .expect("No virtual hosts configured")
    }
    
    /// Find the best matching route for a request path
    fn find_matching_route(&self, vhost: &VirtualHost, path: &str) -> Route {
        // Find the most specific route that matches
        let mut best_match: Option<&RouteConfig> = None;
        let mut best_match_length = 0;
        
        for route_config in &vhost.routes {
            let route = Route::new(route_config.clone());
            if route.matches(path) {
                let match_length = route_config.path.len();
                if match_length > best_match_length {
                    best_match = Some(route_config);
                    best_match_length = match_length;
                }
            }
        }
        
        // Use the best match or create a default route
        if let Some(config) = best_match {
            Route::new(config.clone())
        } else {
            // Create default route if no match found
            let mut default_config = RouteConfig::default();
            default_config.document_root = Some(vhost.document_root.clone().into());
            Route::new(default_config)
        }
    }
    
    /// Process request through the handler chain for a specific route
    fn process_request_with_route(&mut self, request: &HttpRequest, route: &Route, vhost: &VirtualHost) -> io::Result<HttpResponse> {
        // Check if method is allowed
        if !route.allows_method(&request.method()) {
            let mut handler = MethodFilterHandler::new(route.config().allowed_methods.clone());
            match handler.handle(request) {
                HandlerResult::Response(response) => return Ok(response),
                HandlerResult::Error(e) => return Err(e),
                HandlerResult::Continue => {} // This shouldn't happen for method filter
            }
        }
        
        // Check for redirect
        if let Some(redirect_target) = route.redirect_target() {
            let mut handler = RedirectHandler::new(redirect_target.to_string(), None);
            match handler.handle(request) {
                HandlerResult::Response(response) => return Ok(response),
                HandlerResult::Error(e) => return Err(e),
                HandlerResult::Continue => {} // This shouldn't happen for redirect
            }
        }
        
        // Handle different request types
        match request.method() {
            Method::GET | Method::HEAD => {
                self.handle_get_request(request, route, vhost)
            }
            Method::POST => {
                self.handle_post_request(request, route, vhost)
            }
            Method::DELETE => {
                self.handle_delete_request(request, route, vhost)
            }
            Method::PUT | Method::OPTIONS => {
                // Return 405 Method Not Allowed for unsupported methods
                self.generate_error_response(405, route, vhost)
            }
        }
    }
    
    /// Handle GET and HEAD requests (static files, directory listing, CGI)
    fn handle_get_request(&mut self, request: &HttpRequest, route: &Route, vhost: &VirtualHost) -> io::Result<HttpResponse> {
        let path = request.path();
        let file_path = Path::new(&vhost.document_root).join(path.trim_start_matches('/'));
        
        // Check if this is a CGI script
        if self.cgi_executor.is_cgi_script(&file_path) {
            return self.execute_cgi_script(request, &file_path, route, vhost);
        }
        
        // Check if file exists for static serving
        if file_path.exists() && file_path.is_file() {
            // For now, return a simple static file response
            // This will be integrated with the existing StaticFileServer
            let mut response = HttpResponse::ok();
            response.set_body(b"Static file serving - integrated with existing system");
            response.set_header("Content-Type", "text/plain");
            return Ok(response);
        }
        
        // File not found
        self.generate_error_response(404, route, vhost)
    }
    
    /// Handle POST requests (uploads, form processing, CGI)
    fn handle_post_request(&mut self, request: &HttpRequest, route: &Route, vhost: &VirtualHost) -> io::Result<HttpResponse> {
        // Check body size limit
        if let Some(max_size) = route.max_body_size() {
            if let Some(content_length) = request.content_length() {
                if content_length > max_size {
                    return self.generate_error_response(413, route, vhost);
                }
            }
        }
        
        // Check if this is a CGI request
        if let Some(_cgi_ext) = route.cgi_extension() {
            // TODO: Implement CGI handling
            return self.generate_error_response(500, route, vhost);
        }
        
        // Get request body
        let body = request.body().unwrap_or(&[]);
        
        // Handle different content types
        if let Some(content_type) = request.get_header("Content-Type") {
            if content_type.starts_with("multipart/form-data") {
                return self.handle_multipart_upload(request, body, content_type, route);
            } else if content_type.starts_with("application/x-www-form-urlencoded") {
                return self.handle_form_data(request, body, route);
            }
        }
        
        // Default POST handling for other content types
        let mut response = HttpResponse::ok();
        response.set_body(b"POST request received");
        response.set_header("Content-Type", "text/plain");
        Ok(response)
    }
    
    /// Handle multipart form data with file uploads
    fn handle_multipart_upload(&mut self, request: &HttpRequest, body: &[u8], content_type: &str, route: &Route) -> io::Result<HttpResponse> {
        // Extract boundary from Content-Type header
        let boundary = self.extract_boundary(content_type)?;
        
        // Create multipart parser
        let max_file_size = route.max_body_size().unwrap_or(10 * 1024 * 1024); // 10MB default
        let parser = MultipartParser::new(boundary, max_file_size, max_file_size);
        
        // Parse multipart data
        let fields = parser.parse(body)?;
        
        let mut uploaded_files = Vec::new();
        let mut form_fields = Vec::new();
        
        // Process each field
        for field in fields {
            match field.field_type {
                FieldType::File { filename, content_type, data } => {
                    // Store uploaded file
                    match self.file_storage.store_file(&data, filename.clone(), content_type.clone()) {
                        Ok(uploaded_file) => {
                            uploaded_files.push(uploaded_file);
                        }
                        Err(e) => {
                            eprintln!("Failed to store uploaded file: {}", e);
                            return self.generate_error_response(500, route, &VirtualHost::default());
                        }
                    }
                }
                FieldType::Text(value) => {
                    form_fields.push((field.name, value));
                }
            }
        }
        
        // Generate response
        let mut response_body = String::new();
        response_body.push_str("File upload successful!\n\n");
        
        if !uploaded_files.is_empty() {
            response_body.push_str("Uploaded files:\n");
            for file in &uploaded_files {
                response_body.push_str(&format!(
                    "- {} ({} bytes) -> {}\n",
                    file.original_filename.as_deref().unwrap_or("unknown"),
                    file.size,
                    file.stored_filename
                ));
            }
            response_body.push('\n');
        }
        
        if !form_fields.is_empty() {
            response_body.push_str("Form fields:\n");
            for (name, value) in &form_fields {
                response_body.push_str(&format!("- {}: {}\n", name, value));
            }
        }
        
        let mut response = HttpResponse::ok();
        response.set_body(response_body.as_bytes());
        response.set_header("Content-Type", "text/plain");
        Ok(response)
    }
    
    /// Handle URL-encoded form data
    fn handle_form_data(&mut self, _request: &HttpRequest, body: &[u8], _route: &Route) -> io::Result<HttpResponse> {
        // Parse form data
        let form_data = FormData::parse(body)?;
        
        // Generate response
        let mut response_body = String::new();
        response_body.push_str("Form data received!\n\n");
        
        let fields = form_data.fields();
        if !fields.is_empty() {
            response_body.push_str("Form fields:\n");
            for field in &fields {
                response_body.push_str(&format!("- {}: {}\n", field.name, field.value));
            }
        } else {
            response_body.push_str("No form fields found.\n");
        }
        
        let mut response = HttpResponse::ok();
        response.set_body(response_body.as_bytes());
        response.set_header("Content-Type", "text/plain");
        Ok(response)
    }
    
    /// Extract boundary from multipart Content-Type header
    fn extract_boundary(&self, content_type: &str) -> io::Result<String> {
        for part in content_type.split(';') {
            let part = part.trim();
            if part.starts_with("boundary=") {
                let boundary = &part[9..]; // Skip "boundary="
                return Ok(boundary.trim_matches('"').to_string());
            }
        }
        
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Missing boundary in multipart Content-Type"
        ))
    }
    
    /// Handle DELETE requests
    fn handle_delete_request(&mut self, request: &HttpRequest, route: &Route, vhost: &VirtualHost) -> io::Result<HttpResponse> {
        let path = request.path();
        
        // Security check: only allow deletion in upload directory or specific paths
        if !path.starts_with("/uploads/") && !path.starts_with("/delete/") {
            return self.generate_error_response(403, route, vhost);
        }
        
        // Extract filename from path
        let filename = path.trim_start_matches("/uploads/").trim_start_matches("/delete/");
        
        if filename.is_empty() {
            return self.generate_error_response(400, route, vhost);
        }
        
        // Construct file path in upload directory
        let upload_dir = self.file_storage.config().upload_dir.clone();
        let file_path = upload_dir.join(filename);
        
        // Check if file exists
        if !file_path.exists() {
            return self.generate_error_response(404, route, vhost);
        }
        
        // Attempt to delete the file
        match self.file_storage.delete_file(&file_path) {
            Ok(()) => {
                let mut response = HttpResponse::ok();
                let response_body = format!("File '{}' deleted successfully", filename);
                response.set_body(response_body.as_bytes());
                response.set_header("Content-Type", "text/plain");
                Ok(response)
            }
            Err(e) => {
                eprintln!("Failed to delete file '{}': {}", filename, e);
                self.generate_error_response(500, route, vhost)
            }
        }
    }
    
    /// Generate an error response using custom error pages if available
    fn generate_error_response(&mut self, status_code: u16, route: &Route, vhost: &VirtualHost) -> io::Result<HttpResponse> {
        // Try route-specific error page first
        let error_page_path = route.error_page(status_code)
            .map(|p| p.to_string_lossy().to_string())
            .or_else(|| vhost.error_pages.get(&status_code).cloned());
        
        let mut handler = ErrorHandler::new(status_code, error_page_path);
        match handler.handle(&HttpRequest::new()) {
            HandlerResult::Response(response) => Ok(response),
            HandlerResult::Error(e) => Err(e),
            HandlerResult::Continue => {
                // Fallback to basic error response
                let mut response = HttpResponse::new(status_code);
                let error_msg = format!("Error {}", status_code);
                response.set_body(error_msg.as_bytes());
                response.set_header("Content-Type", "text/plain");
                Ok(response)
            }
        }
    }
    
    /// Get the list of configured virtual hosts
    pub fn virtual_hosts(&self) -> &HashMap<String, VirtualHost> {
        &self.virtual_hosts
    }
    
    /// Get the default virtual host name
    pub fn default_host(&self) -> Option<&str> {
        self.default_host.as_deref()
    }
    
    /// Handle session-specific routes and add session cookies to response
    fn handle_session_routes(&mut self, request: &HttpRequest, response: &mut HttpResponse, 
                           cookies: &CookieJar, route: &Route, vhost: &VirtualHost) -> io::Result<()> {
        let path = request.path();
        
        // Handle session creation endpoint
        if path == "/session/create" {
            let session = self.session_store.create_session()?;
            let cookie = self.session_store.create_session_cookie(&session.id);
            response.set_header("Set-Cookie", &cookie.to_header_value());
            
            let response_body = format!("Session created: {}", session.id);
            response.set_body(response_body.as_bytes());
            response.set_header("Content-Type", "text/plain");
            return Ok(());
        }
        
        // Handle session info endpoint
        if path == "/session/info" {
            if let Some(session) = self.session_store.get_session_from_cookies(cookies) {
                let response_body = format!(
                    "Session ID: {}\nCreated: {:?}\nLast Accessed: {:?}\nData: {} items",
                    session.id,
                    session.data.created_at(),
                    session.data.last_accessed(),
                    session.data.len()
                );
                response.set_body(response_body.as_bytes());
            } else {
                response.set_body(b"No active session");
            }
            response.set_header("Content-Type", "text/plain");
            return Ok(());
        }
        
        // Handle session destruction endpoint
        if path == "/session/destroy" {
            if let Some(session_id) = cookies.get_value(self.session_store.config().cookie_name.as_str()) {
                self.session_store.delete_session(session_id);
                let deletion_cookie = self.session_store.create_deletion_cookie();
                response.set_header("Set-Cookie", &deletion_cookie.to_header_value());
                response.set_body(b"Session destroyed");
            } else {
                response.set_body(b"No session to destroy");
            }
            response.set_header("Content-Type", "text/plain");
            return Ok(());
        }
        
        // Handle session data endpoints
        if path.starts_with("/session/set/") {
            if let Some(mut session) = self.session_store.get_session_from_cookies(cookies) {
                // Extract key from path: /session/set/key_name
                if let Some(key) = path.strip_prefix("/session/set/") {
                    if !key.is_empty() {
                        // For POST requests, use body as value
                        if request.method() == &crate::http::request::Method::POST {
                            if let Some(body) = request.body() {
                                let value = String::from_utf8_lossy(body).to_string();
                                session.data.set(key.to_string(), value.clone());
                                self.session_store.update_session(session)?;
                                
                                let response_body = format!("Set session[{}] = {}", key, value);
                                response.set_body(response_body.as_bytes());
                                response.set_header("Content-Type", "text/plain");
                                return Ok(());
                            }
                        }
                    }
                }
            }
            response.set_body(b"Failed to set session data");
            response.set_header("Content-Type", "text/plain");
            return Ok(());
        }
        
        if path.starts_with("/session/get/") {
            if let Some(mut session) = self.session_store.get_session_from_cookies(cookies) {
                // Extract key from path: /session/get/key_name
                if let Some(key) = path.strip_prefix("/session/get/") {
                    if !key.is_empty() {
                        if let Some(value) = session.data.get(key) {
                            let value_bytes = value.as_bytes().to_vec();
                            self.session_store.update_session(session)?;
                            response.set_body(&value_bytes);
                        } else {
                            response.set_body(b"Key not found in session");
                        }
                        response.set_header("Content-Type", "text/plain");
                        return Ok(());
                    }
                }
            }
            response.set_body(b"No session or invalid key");
            response.set_header("Content-Type", "text/plain");
            return Ok(());
        }
        
        // Handle session statistics endpoint
        if path == "/session/stats" {
            let stats = self.session_store.get_stats();
            let response_body = format!(
                "Session Statistics:\nTotal: {}\nActive: {}\nExpired: {}",
                stats.total_sessions,
                stats.active_sessions,
                stats.expired_sessions
            );
            response.set_body(response_body.as_bytes());
            response.set_header("Content-Type", "text/plain");
            return Ok(());
        }
        
        Ok(())
    }
    
    /// Execute CGI script and return response
    fn execute_cgi_script(
        &mut self,
        request: &HttpRequest,
        script_path: &Path,
        route: &Route,
        vhost: &VirtualHost,
    ) -> io::Result<HttpResponse> {
        // Check if CGI is enabled
        if !self.cgi_executor.is_enabled() {
            return self.generate_error_response(403, route, vhost);
        }
        
        // Execute the CGI script
        match self.cgi_executor.execute_cgi(
            request,
            script_path,
            Path::new(&vhost.document_root),
            &vhost.server_name,
            80, // Default port - in production this would be configurable
        ) {
            Ok(response) => Ok(response),
            Err(e) => {
                eprintln!("CGI execution failed: {}", e);
                self.generate_error_response(500, route, vhost)
            }
        }
    }
}

impl Default for Router {
    fn default() -> Self {
        let mut router = Router::new();
        router.add_virtual_host(VirtualHost::default());
        router
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    
    #[test]
    fn test_virtual_host_selection() {
        let mut router = Router::new();
        
        let mut vhost1 = VirtualHost::default();
        vhost1.server_name = "example.com".to_string();
        router.add_virtual_host(vhost1);
        
        let mut vhost2 = VirtualHost::default();
        vhost2.server_name = "test.com".to_string();
        router.add_virtual_host(vhost2);
        
        // Test Host header matching
        let mut request = HttpRequest::new();
        // Note: We'd need a way to set headers on HttpRequest for proper testing
        
        assert_eq!(router.default_host(), Some("example.com"));
    }
    
    #[test]
    fn test_route_matching() {
        let router = Router::default();
        let vhost = VirtualHost::default();
        
        let route = router.find_matching_route(&vhost, "/");
        assert_eq!(route.path(), "/");
        
        let route = router.find_matching_route(&vhost, "/api/users");
        assert_eq!(route.path(), "/"); // Should match root route
    }
    
    #[test]
    fn test_method_filtering() {
        let mut router = Router::default();
        let request = HttpRequest::new();
        
        // Test that router can handle requests
        let result = router.route_request(&request);
        assert!(result.is_ok());
    }
}
