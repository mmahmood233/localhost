use crate::http::request::HttpRequest;
use std::collections::HashMap;
use std::path::Path;

/// CGI environment variable manager
#[derive(Debug, Clone)]
pub struct CgiEnvironment {
    variables: HashMap<String, String>,
}

impl CgiEnvironment {
    pub fn new() -> Self {
        CgiEnvironment {
            variables: HashMap::new(),
        }
    }
    
    /// Create CGI environment from HTTP request and script path
    pub fn from_request(
        request: &HttpRequest,
        script_path: &Path,
        document_root: &Path,
        server_name: &str,
        server_port: u16,
    ) -> Self {
        let mut env = CgiEnvironment::new();
        
        // Required CGI environment variables
        env.set("REQUEST_METHOD", request.method().as_str());
        env.set("SERVER_NAME", server_name);
        env.set("SERVER_PORT", &server_port.to_string());
        env.set("SERVER_SOFTWARE", "localhost/1.0");
        env.set("GATEWAY_INTERFACE", "CGI/1.1");
        env.set("SERVER_PROTOCOL", "HTTP/1.1");
        
        // Request URI and path info
        let request_uri = request.path();
        env.set("REQUEST_URI", request_uri);
        
        // Calculate SCRIPT_NAME and PATH_INFO
        if let Ok(script_relative) = script_path.strip_prefix(document_root) {
            let script_name = format!("/{}", script_relative.to_string_lossy());
            env.set("SCRIPT_NAME", &script_name);
            
            // PATH_INFO is the part of the URI after the script name
            if let Some(path_info) = request_uri.strip_prefix(&script_name) {
                if !path_info.is_empty() {
                    env.set("PATH_INFO", path_info);
                    
                    // PATH_TRANSLATED is the filesystem path for PATH_INFO
                    let translated_path = document_root.join(path_info.trim_start_matches('/'));
                    env.set("PATH_TRANSLATED", &translated_path.to_string_lossy());
                }
            }
        }
        
        env.set("SCRIPT_FILENAME", &script_path.to_string_lossy());
        
        // Query string
        if let Some(query_pos) = request_uri.find('?') {
            let query_string = &request_uri[query_pos + 1..];
            env.set("QUERY_STRING", query_string);
        } else {
            env.set("QUERY_STRING", "");
        }
        
        // Content type and length
        if let Some(content_type) = request.get_header("Content-Type") {
            env.set("CONTENT_TYPE", content_type);
        }
        
        if let Some(content_length) = request.get_header("Content-Length") {
            env.set("CONTENT_LENGTH", content_length);
        } else if let Some(body) = request.body() {
            env.set("CONTENT_LENGTH", &body.len().to_string());
        }
        
        // HTTP headers (convert to HTTP_* format)
        // Note: This would need to be implemented when HttpRequest has a headers() method
        // For now, we'll handle the most common headers individually
        if let Some(user_agent) = request.get_header("User-Agent") {
            env.set("HTTP_USER_AGENT", user_agent);
        }
        if let Some(accept) = request.get_header("Accept") {
            env.set("HTTP_ACCEPT", accept);
        }
        if let Some(accept_encoding) = request.get_header("Accept-Encoding") {
            env.set("HTTP_ACCEPT_ENCODING", accept_encoding);
        }
        if let Some(accept_language) = request.get_header("Accept-Language") {
            env.set("HTTP_ACCEPT_LANGUAGE", accept_language);
        }
        
        // Remote address (simplified - in production would get from connection)
        env.set("REMOTE_ADDR", "127.0.0.1");
        env.set("REMOTE_HOST", "localhost");
        
        // Document root
        env.set("DOCUMENT_ROOT", &document_root.to_string_lossy());
        
        env
    }
    
    /// Set an environment variable
    pub fn set(&mut self, name: &str, value: &str) {
        self.variables.insert(name.to_string(), value.to_string());
    }
    
    /// Get an environment variable
    pub fn get(&self, name: &str) -> Option<&str> {
        self.variables.get(name).map(|s| s.as_str())
    }
    
    /// Get all environment variables
    pub fn variables(&self) -> &HashMap<String, String> {
        &self.variables
    }
    
    /// Convert to vector of "KEY=VALUE" strings for process execution
    pub fn to_env_strings(&self) -> Vec<String> {
        self.variables
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect()
    }
    
    /// Add system environment variables that should be passed through
    pub fn add_system_env(&mut self) {
        // Add PATH from system environment
        if let Ok(path) = std::env::var("PATH") {
            self.set("PATH", &path);
        }
        
        // Add other useful system variables
        if let Ok(home) = std::env::var("HOME") {
            self.set("HOME", &home);
        }
        
        if let Ok(user) = std::env::var("USER") {
            self.set("USER", &user);
        }
        
        if let Ok(shell) = std::env::var("SHELL") {
            self.set("SHELL", &shell);
        }
    }
    
    /// Debug print all environment variables
    pub fn debug_print(&self) {
        println!("CGI Environment Variables:");
        let mut vars: Vec<_> = self.variables.iter().collect();
        vars.sort_by_key(|(k, _)| *k);
        
        for (key, value) in vars {
            println!("  {}={}", key, value);
        }
    }
}

impl Default for CgiEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::request::Method;
    use std::path::PathBuf;
    
    #[test]
    fn test_cgi_environment_creation() {
        let env = CgiEnvironment::new();
        assert!(env.variables.is_empty());
    }
    
    #[test]
    fn test_set_and_get() {
        let mut env = CgiEnvironment::new();
        env.set("TEST_VAR", "test_value");
        
        assert_eq!(env.get("TEST_VAR"), Some("test_value"));
        assert_eq!(env.get("NONEXISTENT"), None);
    }
    
    #[test]
    fn test_from_request() {
        let mut request = HttpRequest::new();
        request.set_method(Method::GET);
        request.set_path("/cgi-bin/test.py/path/info?query=value".to_string());
        request.set_header("Host".to_string(), "example.com".to_string());
        request.set_header("User-Agent".to_string(), "TestAgent/1.0".to_string());
        
        let script_path = PathBuf::from("/var/www/cgi-bin/test.py");
        let document_root = PathBuf::from("/var/www");
        
        let env = CgiEnvironment::from_request(
            &request,
            &script_path,
            &document_root,
            "example.com",
            80,
        );
        
        assert_eq!(env.get("REQUEST_METHOD"), Some("GET"));
        assert_eq!(env.get("SERVER_NAME"), Some("example.com"));
        assert_eq!(env.get("SERVER_PORT"), Some("80"));
        assert_eq!(env.get("SCRIPT_NAME"), Some("/cgi-bin/test.py"));
        assert_eq!(env.get("PATH_INFO"), Some("/path/info"));
        assert_eq!(env.get("QUERY_STRING"), Some("query=value"));
        assert_eq!(env.get("HTTP_HOST"), Some("example.com"));
        assert_eq!(env.get("HTTP_USER_AGENT"), Some("TestAgent/1.0"));
    }
    
    #[test]
    fn test_to_env_strings() {
        let mut env = CgiEnvironment::new();
        env.set("VAR1", "value1");
        env.set("VAR2", "value2");
        
        let env_strings = env.to_env_strings();
        assert_eq!(env_strings.len(), 2);
        assert!(env_strings.contains(&"VAR1=value1".to_string()));
        assert!(env_strings.contains(&"VAR2=value2".to_string()));
    }
    
    #[test]
    fn test_add_system_env() {
        let mut env = CgiEnvironment::new();
        env.add_system_env();
        
        // Should have at least PATH
        assert!(env.get("PATH").is_some());
    }
}
