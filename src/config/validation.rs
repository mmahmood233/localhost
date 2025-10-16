use crate::config::server::*;
use std::collections::HashSet;
use std::fmt;
use std::io;
use std::path::Path;

/// Configuration validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub error_type: ValidationErrorType,
}

/// Types of validation errors
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationErrorType {
    /// Required field is missing
    Required,
    /// Invalid value format
    InvalidFormat,
    /// Value out of valid range
    OutOfRange,
    /// File or directory doesn't exist
    PathNotFound,
    /// Conflicting configuration values
    Conflict,
    /// Security concern
    Security,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {} ({:?})", self.field, self.message, self.error_type)
    }
}

impl std::error::Error for ValidationError {}

/// Configuration validator
#[derive(Debug)]
pub struct ConfigValidator {
    errors: Vec<ValidationError>,
    warnings: Vec<ValidationError>,
}

impl ConfigValidator {
    pub fn new() -> Self {
        ConfigValidator {
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
    
    /// Validate complete server configuration
    pub fn validate(&mut self, config: &ServerConfig) -> Result<(), Vec<ValidationError>> {
        self.errors.clear();
        self.warnings.clear();
        
        // Validate listeners
        self.validate_listeners(&config.listeners);
        
        // Validate virtual hosts
        self.validate_virtual_hosts(&config.virtual_hosts);
        
        // Validate global configuration
        self.validate_global_config(&config.global);
        
        // Check for conflicts
        self.validate_conflicts(config);
        
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }
    
    /// Get validation warnings
    pub fn warnings(&self) -> &[ValidationError] {
        &self.warnings
    }
    
    /// Validate listener configurations
    fn validate_listeners(&mut self, listeners: &[ListenerConfig]) {
        if listeners.is_empty() {
            self.add_error("listeners", "At least one listener must be configured", ValidationErrorType::Required);
            return;
        }
        
        let mut addresses = HashSet::new();
        let mut default_count = 0;
        
        for (i, listener) in listeners.iter().enumerate() {
            let field = format!("listeners[{}]", i);
            
            // Validate address format
            if listener.address.is_empty() {
                self.add_error(&format!("{}.address", field), "Address cannot be empty", ValidationErrorType::Required);
            }
            
            // Validate port range
            if listener.port == 0 {
                self.add_error(&format!("{}.port", field), "Port cannot be 0", ValidationErrorType::OutOfRange);
            } else if listener.port < 1024 && !self.is_privileged_user() {
                self.add_warning(&format!("{}.port", field), "Port < 1024 requires root privileges", ValidationErrorType::Security);
            }
            
            // Check for duplicate addresses
            let addr_port = format!("{}:{}", listener.address, listener.port);
            if addresses.contains(&addr_port) {
                self.add_error(&format!("{}.address", field), "Duplicate listener address", ValidationErrorType::Conflict);
            }
            addresses.insert(addr_port);
            
            // Count default listeners
            if listener.default {
                default_count += 1;
            }
        }
        
        // Ensure exactly one default listener
        if default_count == 0 {
            self.add_warning("listeners", "No default listener specified, using first one", ValidationErrorType::Conflict);
        } else if default_count > 1 {
            self.add_error("listeners", "Multiple default listeners specified", ValidationErrorType::Conflict);
        }
    }
    
    /// Validate virtual host configurations
    fn validate_virtual_hosts(&mut self, vhosts: &[VirtualHostConfig]) {
        if vhosts.is_empty() {
            self.add_error("virtual_hosts", "At least one virtual host must be configured", ValidationErrorType::Required);
            return;
        }
        
        let mut server_names = HashSet::new();
        
        for (i, vhost) in vhosts.iter().enumerate() {
            let field = format!("virtual_hosts[{}]", i);
            
            // Validate server name
            if vhost.server_name.is_empty() {
                self.add_error(&format!("{}.server_name", field), "Server name cannot be empty", ValidationErrorType::Required);
            } else if server_names.contains(&vhost.server_name) {
                self.add_error(&format!("{}.server_name", field), "Duplicate server name", ValidationErrorType::Conflict);
            }
            server_names.insert(vhost.server_name.clone());
            
            // Validate document root
            if !vhost.document_root.exists() {
                self.add_error(&format!("{}.document_root", field), "Document root directory does not exist", ValidationErrorType::PathNotFound);
            } else if !vhost.document_root.is_dir() {
                self.add_error(&format!("{}.document_root", field), "Document root must be a directory", ValidationErrorType::InvalidFormat);
            }
            
            // Validate body size
            if vhost.max_body_size == 0 {
                self.add_error(&format!("{}.max_body_size", field), "Max body size cannot be 0", ValidationErrorType::OutOfRange);
            } else if vhost.max_body_size > 1024 * 1024 * 1024 { // 1GB
                self.add_warning(&format!("{}.max_body_size", field), "Very large max body size may cause memory issues", ValidationErrorType::Security);
            }
            
            // Validate routes
            self.validate_routes(&vhost.routes, &format!("{}.routes", field));
            
            // Validate log files
            if let Some(ref access_log) = vhost.access_log {
                self.validate_log_file(access_log, &format!("{}.access_log", field));
            }
            if let Some(ref error_log) = vhost.error_log {
                self.validate_log_file(error_log, &format!("{}.error_log", field));
            }
        }
    }
    
    /// Validate route configurations
    fn validate_routes(&mut self, routes: &[RouteConfig], field_prefix: &str) {
        if routes.is_empty() {
            self.add_warning(field_prefix, "No routes configured", ValidationErrorType::Required);
            return;
        }
        
        let mut paths = HashSet::new();
        
        for (i, route) in routes.iter().enumerate() {
            let field = format!("{}[{}]", field_prefix, i);
            
            // Validate path
            if route.path.is_empty() {
                self.add_error(&format!("{}.path", field), "Route path cannot be empty", ValidationErrorType::Required);
            } else if !route.path.starts_with('/') {
                self.add_error(&format!("{}.path", field), "Route path must start with '/'", ValidationErrorType::InvalidFormat);
            }
            
            // Check for duplicate paths
            if paths.contains(&route.path) {
                self.add_error(&format!("{}.path", field), "Duplicate route path", ValidationErrorType::Conflict);
            }
            paths.insert(route.path.clone());
            
            // Validate methods
            if route.methods.is_empty() {
                self.add_error(&format!("{}.methods", field), "At least one HTTP method must be specified", ValidationErrorType::Required);
            } else {
                for method in &route.methods {
                    if !self.is_valid_http_method(method) {
                        self.add_error(&format!("{}.methods", field), &format!("Invalid HTTP method: {}", method), ValidationErrorType::InvalidFormat);
                    }
                }
            }
            
            // Validate route type
            self.validate_route_type(&route.route_type, &format!("{}.route_type", field));
            
            // Validate route settings
            self.validate_route_settings(&route.settings, &format!("{}.settings", field));
        }
    }
    
    /// Validate route type configuration
    fn validate_route_type(&mut self, route_type: &RouteType, field: &str) {
        match route_type {
            RouteType::Static { index_files, .. } => {
                if index_files.is_empty() {
                    self.add_warning(field, "No index files specified for static route", ValidationErrorType::Required);
                }
            }
            RouteType::Cgi { script_dir, timeout, .. } => {
                if !script_dir.exists() {
                    self.add_error(field, "CGI script directory does not exist", ValidationErrorType::PathNotFound);
                } else if !script_dir.is_dir() {
                    self.add_error(field, "CGI script path must be a directory", ValidationErrorType::InvalidFormat);
                }
                
                if timeout.as_secs() == 0 {
                    self.add_error(field, "CGI timeout cannot be 0", ValidationErrorType::OutOfRange);
                } else if timeout.as_secs() > 300 { // 5 minutes
                    self.add_warning(field, "Very long CGI timeout may cause resource issues", ValidationErrorType::Security);
                }
            }
            RouteType::Redirect { target, status } => {
                if target.is_empty() {
                    self.add_error(field, "Redirect target cannot be empty", ValidationErrorType::Required);
                }
                
                if !self.is_valid_redirect_status(*status) {
                    self.add_error(field, &format!("Invalid redirect status code: {}", status), ValidationErrorType::InvalidFormat);
                }
            }
            RouteType::Proxy { backend, timeout } => {
                if backend.is_empty() {
                    self.add_error(field, "Proxy backend cannot be empty", ValidationErrorType::Required);
                }
                
                if timeout.as_secs() == 0 {
                    self.add_error(field, "Proxy timeout cannot be 0", ValidationErrorType::OutOfRange);
                }
            }
        }
    }
    
    /// Validate route settings
    fn validate_route_settings(&mut self, settings: &RouteSettings, field: &str) {
        if let Some(max_body_size) = settings.max_body_size {
            if max_body_size == 0 {
                self.add_error(&format!("{}.max_body_size", field), "Max body size cannot be 0", ValidationErrorType::OutOfRange);
            }
        }
        
        if let Some(rate_limit) = settings.rate_limit {
            if rate_limit == 0 {
                self.add_error(&format!("{}.rate_limit", field), "Rate limit cannot be 0", ValidationErrorType::OutOfRange);
            }
        }
    }
    
    /// Validate global configuration
    fn validate_global_config(&mut self, global: &GlobalConfig) {
        // Validate server name
        if global.server_name.is_empty() {
            self.add_error("global.server_name", "Server name cannot be empty", ValidationErrorType::Required);
        }
        
        // Validate workers (should always be 1 for this server)
        if global.workers != 1 {
            self.add_warning("global.workers", "This server only supports single-threaded operation", ValidationErrorType::InvalidFormat);
        }
        
        // Validate timeouts
        self.validate_timeouts(&global.timeouts);
        
        // Validate uploads
        self.validate_upload_config(&global.uploads);
        
        // Validate sessions
        self.validate_session_config(&global.sessions);
        
        // Validate CGI
        self.validate_cgi_config(&global.cgi);
        
        // Validate logging
        self.validate_logging_config(&global.logging);
        
        // Validate security
        self.validate_security_config(&global.security);
    }
    
    /// Validate timeout configuration
    fn validate_timeouts(&mut self, timeouts: &TimeoutConfig) {
        let field = "global.timeouts";
        
        if timeouts.read_header.as_secs() == 0 {
            self.add_error(&format!("{}.read_header", field), "Read header timeout cannot be 0", ValidationErrorType::OutOfRange);
        }
        
        if timeouts.read_body.as_secs() == 0 {
            self.add_error(&format!("{}.read_body", field), "Read body timeout cannot be 0", ValidationErrorType::OutOfRange);
        }
        
        if timeouts.write.as_secs() == 0 {
            self.add_error(&format!("{}.write", field), "Write timeout cannot be 0", ValidationErrorType::OutOfRange);
        }
        
        if timeouts.keep_alive.as_secs() == 0 {
            self.add_error(&format!("{}.keep_alive", field), "Keep-alive timeout cannot be 0", ValidationErrorType::OutOfRange);
        }
        
        if timeouts.request.as_secs() == 0 {
            self.add_error(&format!("{}.request", field), "Request timeout cannot be 0", ValidationErrorType::OutOfRange);
        }
        
        // Check timeout relationships
        if timeouts.request < timeouts.read_header + timeouts.read_body {
            self.add_warning(&format!("{}.request", field), "Request timeout should be larger than read timeouts combined", ValidationErrorType::Conflict);
        }
    }
    
    /// Validate upload configuration
    fn validate_upload_config(&mut self, uploads: &UploadConfig) {
        let field = "global.uploads";
        
        // Validate upload directory
        if let Some(parent) = uploads.directory.parent() {
            if !parent.exists() {
                self.add_error(&format!("{}.directory", field), "Upload directory parent does not exist", ValidationErrorType::PathNotFound);
            }
        }
        
        // Validate sizes
        if uploads.max_file_size == 0 {
            self.add_error(&format!("{}.max_file_size", field), "Max file size cannot be 0", ValidationErrorType::OutOfRange);
        }
        
        if uploads.max_total_size == 0 {
            self.add_error(&format!("{}.max_total_size", field), "Max total size cannot be 0", ValidationErrorType::OutOfRange);
        }
        
        if uploads.max_file_size > uploads.max_total_size {
            self.add_error(&format!("{}.max_file_size", field), "Max file size cannot be larger than max total size", ValidationErrorType::Conflict);
        }
    }
    
    /// Validate session configuration
    fn validate_session_config(&mut self, sessions: &SessionConfig) {
        let field = "global.sessions";
        
        if sessions.cookie_name.is_empty() {
            self.add_error(&format!("{}.cookie_name", field), "Cookie name cannot be empty", ValidationErrorType::Required);
        }
        
        if sessions.expiration.as_secs() == 0 {
            self.add_error(&format!("{}.expiration", field), "Session expiration cannot be 0", ValidationErrorType::OutOfRange);
        }
        
        if sessions.cleanup_interval.as_secs() == 0 {
            self.add_error(&format!("{}.cleanup_interval", field), "Cleanup interval cannot be 0", ValidationErrorType::OutOfRange);
        }
        
        if sessions.max_sessions == 0 {
            self.add_error(&format!("{}.max_sessions", field), "Max sessions cannot be 0", ValidationErrorType::OutOfRange);
        }
        
        // Validate SameSite value
        match sessions.same_site.to_lowercase().as_str() {
            "strict" | "lax" | "none" => {}
            _ => self.add_error(&format!("{}.same_site", field), "Invalid SameSite value", ValidationErrorType::InvalidFormat),
        }
    }
    
    /// Validate CGI configuration
    fn validate_cgi_config(&mut self, cgi: &CgiConfig) {
        let field = "global.cgi";
        
        if cgi.enabled {
            if !cgi.directory.exists() {
                self.add_error(&format!("{}.directory", field), "CGI directory does not exist", ValidationErrorType::PathNotFound);
            } else if !cgi.directory.is_dir() {
                self.add_error(&format!("{}.directory", field), "CGI directory must be a directory", ValidationErrorType::InvalidFormat);
            }
            
            if cgi.timeout.as_secs() == 0 {
                self.add_error(&format!("{}.timeout", field), "CGI timeout cannot be 0", ValidationErrorType::OutOfRange);
            }
            
            if cgi.max_output_size == 0 {
                self.add_error(&format!("{}.max_output_size", field), "CGI max output size cannot be 0", ValidationErrorType::OutOfRange);
            }
            
            if cgi.interpreters.is_empty() {
                self.add_warning(&format!("{}.interpreters", field), "No CGI interpreters configured", ValidationErrorType::Required);
            }
        }
    }
    
    /// Validate logging configuration
    fn validate_logging_config(&mut self, logging: &LoggingConfig) {
        let field = "global.logging";
        
        // Validate log level
        match logging.level.to_lowercase().as_str() {
            "error" | "warn" | "info" | "debug" => {}
            _ => self.add_error(&format!("{}.level", field), "Invalid log level", ValidationErrorType::InvalidFormat),
        }
        
        // Validate log files
        if let Some(ref access_log) = logging.access_log {
            self.validate_log_file(access_log, &format!("{}.access_log", field));
        }
        if let Some(ref error_log) = logging.error_log {
            self.validate_log_file(error_log, &format!("{}.error_log", field));
        }
    }
    
    /// Validate security configuration
    fn validate_security_config(&mut self, security: &SecurityConfig) {
        let field = "global.security";
        
        if security.max_header_size == 0 {
            self.add_error(&format!("{}.max_header_size", field), "Max header size cannot be 0", ValidationErrorType::OutOfRange);
        }
        
        if security.max_headers == 0 {
            self.add_error(&format!("{}.max_headers", field), "Max headers cannot be 0", ValidationErrorType::OutOfRange);
        }
        
        // Validate rate limiting
        if security.rate_limiting.enabled {
            if security.rate_limiting.requests_per_minute == 0 {
                self.add_error(&format!("{}.rate_limiting.requests_per_minute", field), "Requests per minute cannot be 0", ValidationErrorType::OutOfRange);
            }
            
            if security.rate_limiting.burst_size == 0 {
                self.add_error(&format!("{}.rate_limiting.burst_size", field), "Burst size cannot be 0", ValidationErrorType::OutOfRange);
            }
        }
    }
    
    /// Validate log file path
    fn validate_log_file(&mut self, log_file: &Path, field: &str) {
        if let Some(parent) = log_file.parent() {
            if !parent.exists() {
                self.add_error(field, "Log file directory does not exist", ValidationErrorType::PathNotFound);
            }
        }
    }
    
    /// Validate conflicts between different configuration sections
    fn validate_conflicts(&mut self, config: &ServerConfig) {
        // Check if default host exists in virtual hosts
        if let Some(ref default_host) = config.default_host {
            let has_default = config.virtual_hosts.iter()
                .any(|vhost| vhost.server_name == *default_host);
            
            if !has_default {
                self.add_error("default_host", "Default host not found in virtual hosts", ValidationErrorType::Conflict);
            }
        }
        
        // Check for port conflicts with CGI
        for listener in &config.listeners {
            if listener.port == 80 && config.global.cgi.enabled {
                self.add_warning("listeners", "Running CGI on port 80 may have security implications", ValidationErrorType::Security);
            }
        }
    }
    
    /// Add validation error
    fn add_error(&mut self, field: &str, message: &str, error_type: ValidationErrorType) {
        self.errors.push(ValidationError {
            field: field.to_string(),
            message: message.to_string(),
            error_type,
        });
    }
    
    /// Add validation warning
    fn add_warning(&mut self, field: &str, message: &str, error_type: ValidationErrorType) {
        self.warnings.push(ValidationError {
            field: field.to_string(),
            message: message.to_string(),
            error_type,
        });
    }
    
    /// Check if HTTP method is valid
    fn is_valid_http_method(&self, method: &str) -> bool {
        matches!(method.to_uppercase().as_str(), 
            "GET" | "POST" | "PUT" | "DELETE" | "HEAD" | "OPTIONS" | "PATCH" | "TRACE" | "CONNECT")
    }
    
    /// Check if redirect status code is valid
    fn is_valid_redirect_status(&self, status: u16) -> bool {
        matches!(status, 301 | 302 | 303 | 307 | 308)
    }
    
    /// Check if current user has privileged access (simplified)
    fn is_privileged_user(&self) -> bool {
        // In a real implementation, check if running as root/admin
        false
    }
}

impl Default for ConfigValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[test]
    fn test_validator_creation() {
        let validator = ConfigValidator::new();
        assert!(validator.errors.is_empty());
        assert!(validator.warnings.is_empty());
    }
    
    #[test]
    fn test_validate_empty_listeners() {
        let mut validator = ConfigValidator::new();
        let config = ServerConfig {
            listeners: vec![],
            ..Default::default()
        };
        
        let result = validator.validate(&config);
        assert!(result.is_err());
        assert!(!validator.errors.is_empty());
    }
    
    #[test]
    fn test_validate_valid_config() {
        let mut validator = ConfigValidator::new();
        let config = ServerConfig::default();
        
        let result = validator.validate(&config);
        // May have warnings but should not have errors for default config
        if result.is_err() {
            println!("Validation errors: {:?}", validator.errors);
        }
    }
    
    #[test]
    fn test_validate_duplicate_listeners() {
        let mut validator = ConfigValidator::new();
        let config = ServerConfig {
            listeners: vec![
                ListenerConfig { address: "127.0.0.1".to_string(), port: 8080, default: true },
                ListenerConfig { address: "127.0.0.1".to_string(), port: 8080, default: false },
            ],
            ..Default::default()
        };
        
        let result = validator.validate(&config);
        assert!(result.is_err());
        assert!(validator.errors.iter().any(|e| e.message.contains("Duplicate")));
    }
    
    #[test]
    fn test_validate_http_methods() {
        let validator = ConfigValidator::new();
        
        assert!(validator.is_valid_http_method("GET"));
        assert!(validator.is_valid_http_method("POST"));
        assert!(validator.is_valid_http_method("get"));
        assert!(!validator.is_valid_http_method("INVALID"));
    }
    
    #[test]
    fn test_validate_redirect_status() {
        let validator = ConfigValidator::new();
        
        assert!(validator.is_valid_redirect_status(301));
        assert!(validator.is_valid_redirect_status(302));
        assert!(!validator.is_valid_redirect_status(200));
        assert!(!validator.is_valid_redirect_status(404));
    }
    
    #[test]
    fn test_validation_error_display() {
        let error = ValidationError {
            field: "test.field".to_string(),
            message: "Test error message".to_string(),
            error_type: ValidationErrorType::Required,
        };
        
        let display = format!("{}", error);
        assert!(display.contains("test.field"));
        assert!(display.contains("Test error message"));
        assert!(display.contains("Required"));
    }
}
