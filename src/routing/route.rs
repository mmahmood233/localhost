use crate::http::request::Method;
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct RouteConfig {
    /// Path pattern to match (e.g., "/api", "/uploads", "/")
    pub path: String,
    
    /// Allowed HTTP methods for this route
    pub allowed_methods: HashSet<Method>,
    
    /// Document root for this route (overrides server default)
    pub document_root: Option<PathBuf>,
    
    /// Default file to serve for directories
    pub index_file: Option<String>,
    
    /// Enable directory listing
    pub directory_listing: bool,
    
    /// HTTP redirection target
    pub redirect: Option<String>,
    
    /// CGI extension mapping (e.g., ".php" -> "/usr/bin/php")
    pub cgi_extension: Option<String>,
    
    /// Maximum request body size for uploads
    pub max_body_size: Option<usize>,
    
    /// Custom error pages for this route
    pub error_pages: std::collections::HashMap<u16, PathBuf>,
}

impl Default for RouteConfig {
    fn default() -> Self {
        let mut allowed_methods = HashSet::new();
        allowed_methods.insert(Method::GET);
        allowed_methods.insert(Method::HEAD);
        
        RouteConfig {
            path: "/".to_string(),
            allowed_methods,
            document_root: None,
            index_file: Some("index.html".to_string()),
            directory_listing: false,
            redirect: None,
            cgi_extension: None,
            max_body_size: Some(1024 * 1024), // 1MB default
            error_pages: std::collections::HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct Route {
    config: RouteConfig,
}

impl Route {
    pub fn new(config: RouteConfig) -> Self {
        Route { config }
    }
    
    /// Check if this route matches the given path
    pub fn matches(&self, path: &str) -> bool {
        // Exact match
        if self.config.path == path {
            return true;
        }
        
        // Directory-based matching (route "/api" matches "/api/users")
        if self.config.path != "/" && path.starts_with(&self.config.path) {
            // Ensure we match directory boundaries
            let route_path = &self.config.path;
            if path.len() > route_path.len() {
                let next_char = path.chars().nth(route_path.len()).unwrap();
                return next_char == '/';
            }
        }
        
        // Root route matches everything if no other route matches
        self.config.path == "/"
    }
    
    /// Check if the given HTTP method is allowed for this route
    pub fn allows_method(&self, method: &Method) -> bool {
        self.config.allowed_methods.contains(method)
    }
    
    /// Get the document root for this route
    pub fn document_root(&self) -> Option<&PathBuf> {
        self.config.document_root.as_ref()
    }
    
    /// Get the index file for this route
    pub fn index_file(&self) -> Option<&str> {
        self.config.index_file.as_deref()
    }
    
    /// Check if directory listing is enabled
    pub fn directory_listing_enabled(&self) -> bool {
        self.config.directory_listing
    }
    
    /// Get redirect target if configured
    pub fn redirect_target(&self) -> Option<&str> {
        self.config.redirect.as_deref()
    }
    
    /// Get CGI extension if configured
    pub fn cgi_extension(&self) -> Option<&str> {
        self.config.cgi_extension.as_deref()
    }
    
    /// Get maximum body size for this route
    pub fn max_body_size(&self) -> Option<usize> {
        self.config.max_body_size
    }
    
    /// Get custom error page for a status code
    pub fn error_page(&self, status_code: u16) -> Option<&PathBuf> {
        self.config.error_pages.get(&status_code)
    }
    
    /// Get the route path pattern
    pub fn path(&self) -> &str {
        &self.config.path
    }
    
    /// Get the full route configuration
    pub fn config(&self) -> &RouteConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_route_matching() {
        let mut config = RouteConfig::default();
        config.path = "/api".to_string();
        let route = Route::new(config);
        
        assert!(route.matches("/api"));
        assert!(route.matches("/api/users"));
        assert!(route.matches("/api/users/123"));
        assert!(!route.matches("/app"));
        assert!(!route.matches("/api2"));
    }
    
    #[test]
    fn test_root_route_matching() {
        let config = RouteConfig::default(); // path = "/"
        let route = Route::new(config);
        
        assert!(route.matches("/"));
        assert!(route.matches("/anything"));
        assert!(route.matches("/path/to/file"));
    }
    
    #[test]
    fn test_method_filtering() {
        let mut config = RouteConfig::default();
        config.allowed_methods.clear();
        config.allowed_methods.insert(Method::POST);
        let route = Route::new(config);
        
        assert!(route.allows_method(&Method::POST));
        assert!(!route.allows_method(&Method::GET));
        assert!(!route.allows_method(&Method::DELETE));
    }
    
    #[test]
    fn test_exact_matching() {
        let mut config = RouteConfig::default();
        config.path = "/exact".to_string();
        let route = Route::new(config);
        
        assert!(route.matches("/exact"));
        assert!(!route.matches("/exact/more"));
    }
}
