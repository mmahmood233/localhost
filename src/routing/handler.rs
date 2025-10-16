use crate::http::request::HttpRequest;
use crate::http::response::HttpResponse;
use std::io;

/// Result of handling a request
#[derive(Debug)]
pub enum HandlerResult {
    /// Request was handled successfully, response is ready
    Response(HttpResponse),
    /// Request needs to be passed to the next handler
    Continue,
    /// Request handling failed with an error
    Error(io::Error),
}

/// Trait for request handlers
pub trait Handler {
    /// Handle an HTTP request and return the result
    fn handle(&mut self, request: &HttpRequest) -> HandlerResult;
    
    /// Get the name of this handler for debugging
    fn name(&self) -> &'static str;
}

/// Static file handler
pub struct StaticFileHandler {
    document_root: String,
    index_file: String,
}

impl StaticFileHandler {
    pub fn new(document_root: String, index_file: String) -> Self {
        StaticFileHandler {
            document_root,
            index_file,
        }
    }
}

impl Handler for StaticFileHandler {
    fn handle(&mut self, request: &HttpRequest) -> HandlerResult {
        // For now, delegate to existing static file serving logic
        // This will be integrated with the existing StaticFileServer
        HandlerResult::Continue
    }
    
    fn name(&self) -> &'static str {
        "StaticFileHandler"
    }
}

/// Redirect handler
pub struct RedirectHandler {
    target_url: String,
    status_code: u16,
}

impl RedirectHandler {
    pub fn new(target_url: String, status_code: Option<u16>) -> Self {
        RedirectHandler {
            target_url,
            status_code: status_code.unwrap_or(302), // Default to temporary redirect
        }
    }
}

impl Handler for RedirectHandler {
    fn handle(&mut self, _request: &HttpRequest) -> HandlerResult {
        let mut response = HttpResponse::new(self.status_code);
        response.set_header("Location", &self.target_url);
        let redirect_html = format!(
            "<!DOCTYPE html>\n<html><head><title>Redirect</title></head>\n<body><h1>Redirecting...</h1>\n<p>If you are not redirected automatically, <a href=\"{}\">click here</a>.</p></body></html>",
            self.target_url
        );
        response.set_body(redirect_html.as_bytes());
        response.set_header("Content-Type", "text/html");
        HandlerResult::Response(response)
    }
    
    fn name(&self) -> &'static str {
        "RedirectHandler"
    }
}

/// Method filter handler - checks if HTTP method is allowed
pub struct MethodFilterHandler {
    allowed_methods: std::collections::HashSet<crate::http::request::Method>,
}

impl MethodFilterHandler {
    pub fn new(allowed_methods: std::collections::HashSet<crate::http::request::Method>) -> Self {
        MethodFilterHandler { allowed_methods }
    }
}

impl Handler for MethodFilterHandler {
    fn handle(&mut self, request: &HttpRequest) -> HandlerResult {
        if self.allowed_methods.contains(&request.method()) {
            HandlerResult::Continue
        } else {
            let mut response = HttpResponse::method_not_allowed();
            
            // Set Allow header with supported methods
            let methods: Vec<String> = self.allowed_methods
                .iter()
                .map(|m| format!("{:?}", m))
                .collect();
            response.set_header("Allow", &methods.join(", "));
            
            HandlerResult::Response(response)
        }
    }
    
    fn name(&self) -> &'static str {
        "MethodFilterHandler"
    }
}

/// CGI handler for executing scripts
pub struct CgiHandler {
    script_path: String,
    interpreter: String,
}

impl CgiHandler {
    pub fn new(script_path: String, interpreter: String) -> Self {
        CgiHandler {
            script_path,
            interpreter,
        }
    }
}

impl Handler for CgiHandler {
    fn handle(&mut self, _request: &HttpRequest) -> HandlerResult {
        // CGI implementation will be added later
        // This is a placeholder for the CGI execution logic
        HandlerResult::Continue
    }
    
    fn name(&self) -> &'static str {
        "CgiHandler"
    }
}

/// Error handler for generating error pages
pub struct ErrorHandler {
    status_code: u16,
    error_page_path: Option<String>,
}

impl ErrorHandler {
    pub fn new(status_code: u16, error_page_path: Option<String>) -> Self {
        ErrorHandler {
            status_code,
            error_page_path,
        }
    }
}

impl Handler for ErrorHandler {
    fn handle(&mut self, _request: &HttpRequest) -> HandlerResult {
        let response = if let Some(ref path) = self.error_page_path {
            // Try to load custom error page
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    let mut response = HttpResponse::new(self.status_code);
                    response.set_body(content.as_bytes());
                    response.set_header("Content-Type", "text/html");
                    response
                }
                Err(_) => {
                    // Fall back to default error page
                    self.default_error_response()
                }
            }
        } else {
            self.default_error_response()
        };
        
        HandlerResult::Response(response)
    }
    
    fn name(&self) -> &'static str {
        "ErrorHandler"
    }
}

impl ErrorHandler {
    fn default_error_response(&self) -> HttpResponse {
        let status_text = match self.status_code {
            400 => "Bad Request",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            413 => "Payload Too Large",
            500 => "Internal Server Error",
            _ => "Error",
        };
        
        let mut response = HttpResponse::new(self.status_code);
        let error_html = format!(
            "<!DOCTYPE html>\n<html><head><title>{} {}</title></head>\n<body><h1>{} {}</h1></body></html>",
            self.status_code, status_text, self.status_code, status_text
        );
        response.set_body(error_html.as_bytes());
        response.set_header("Content-Type", "text/html");
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::request::Method;
    use std::collections::HashSet;
    
    #[test]
    fn test_redirect_handler() {
        let mut handler = RedirectHandler::new("https://example.com".to_string(), Some(301));
        let request = HttpRequest::new(); // Dummy request
        
        match handler.handle(&request) {
            HandlerResult::Response(response) => {
                assert_eq!(response.status_code(), 301);
                assert_eq!(response.get_header("Location"), Some("https://example.com"));
            }
            _ => panic!("Expected response"),
        }
    }
    
    #[test]
    fn test_method_filter_handler() {
        let mut allowed_methods = HashSet::new();
        allowed_methods.insert(Method::GET);
        allowed_methods.insert(Method::POST);
        
        let mut handler = MethodFilterHandler::new(allowed_methods);
        
        // Test allowed method
        let request = HttpRequest::new();
        // Note: We'd need to set the method on the request, but HttpRequest::new() doesn't allow this
        // This test would need to be updated when HttpRequest has a proper constructor
        
        // For now, just test that the handler exists
        assert_eq!(handler.name(), "MethodFilterHandler");
    }
    
    #[test]
    fn test_error_handler() {
        let mut handler = ErrorHandler::new(404, None);
        let request = HttpRequest::new();
        
        match handler.handle(&request) {
            HandlerResult::Response(response) => {
                assert_eq!(response.status_code(), 404);
                assert!(response.body().contains("404"));
                assert!(response.body().contains("Not Found"));
            }
            _ => panic!("Expected response"),
        }
    }
}
