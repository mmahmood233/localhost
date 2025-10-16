use crate::config::server::*;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Configuration file format
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigFormat {
    /// TOML format (recommended)
    Toml,
    /// JSON format
    Json,
    /// Auto-detect from file extension
    Auto,
}

/// Configuration file parser
#[derive(Debug)]
pub struct ConfigParser {
    format: ConfigFormat,
}

impl ConfigParser {
    pub fn new(format: ConfigFormat) -> Self {
        ConfigParser { format }
    }
    
    /// Parse configuration from file
    pub fn parse_file<P: AsRef<Path>>(&self, path: P) -> io::Result<ServerConfig> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)?;
        
        let format = match self.format {
            ConfigFormat::Auto => self.detect_format(path)?,
            _ => self.format.clone(),
        };
        
        self.parse_content(&content, format)
    }
    
    /// Parse configuration from string content
    pub fn parse_content(&self, content: &str, format: ConfigFormat) -> io::Result<ServerConfig> {
        match format {
            ConfigFormat::Toml => self.parse_toml(content),
            ConfigFormat::Json => self.parse_json(content),
            ConfigFormat::Auto => {
                // Try TOML first, then JSON
                self.parse_toml(content)
                    .or_else(|_| self.parse_json(content))
            }
        }
    }
    
    /// Detect format from file extension
    fn detect_format(&self, path: &Path) -> io::Result<ConfigFormat> {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("toml") => Ok(ConfigFormat::Toml),
            Some("json") => Ok(ConfigFormat::Json),
            _ => Ok(ConfigFormat::Toml), // Default to TOML
        }
    }
    
    /// Parse TOML configuration
    fn parse_toml(&self, content: &str) -> io::Result<ServerConfig> {
        // Simple TOML-like parser (in production, use a proper TOML library)
        let mut config = ServerConfig::default();
        let mut current_section = String::new();
        let mut current_vhost: Option<VirtualHostConfig> = None;
        let mut current_route: Option<RouteConfig> = None;
        
        for line in content.lines() {
            let line = line.trim();
            
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            // Handle sections
            if line.starts_with('[') && line.ends_with(']') {
                // Save previous vhost/route if any
                if let Some(route) = current_route.take() {
                    if let Some(ref mut vhost) = current_vhost {
                        vhost.routes.push(route);
                    }
                }
                if let Some(vhost) = current_vhost.take() {
                    config.virtual_hosts.push(vhost);
                }
                
                current_section = line[1..line.len()-1].to_string();
                
                // Start new vhost or route
                if current_section.starts_with("vhost.") {
                    current_vhost = Some(VirtualHostConfig::default());
                } else if current_section.starts_with("route.") {
                    current_route = Some(RouteConfig::default());
                }
                continue;
            }
            
            // Handle key-value pairs
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim();
                let value = line[eq_pos + 1..].trim();
                let value = self.parse_value(value);
                
                self.set_config_value(&mut config, &current_section, key, &value, 
                                    &mut current_vhost, &mut current_route)?;
            }
        }
        
        // Save final vhost/route
        if let Some(route) = current_route {
            if let Some(ref mut vhost) = current_vhost {
                vhost.routes.push(route);
            }
        }
        if let Some(vhost) = current_vhost {
            config.virtual_hosts.push(vhost);
        }
        
        Ok(config)
    }
    
    /// Parse JSON configuration
    fn parse_json(&self, content: &str) -> io::Result<ServerConfig> {
        // Simple JSON parser (in production, use serde_json)
        // For now, return default config with error
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "JSON parsing not fully implemented - use TOML format",
        ))
    }
    
    /// Parse configuration value
    fn parse_value(&self, value: &str) -> String {
        // Remove quotes if present
        if (value.starts_with('"') && value.ends_with('"')) ||
           (value.starts_with('\'') && value.ends_with('\'')) {
            value[1..value.len()-1].to_string()
        } else {
            value.to_string()
        }
    }
    
    /// Set configuration value based on section and key
    fn set_config_value(
        &self,
        config: &mut ServerConfig,
        section: &str,
        key: &str,
        value: &str,
        current_vhost: &mut Option<VirtualHostConfig>,
        current_route: &mut Option<RouteConfig>,
    ) -> io::Result<()> {
        match section {
            "server" => self.set_server_value(config, key, value)?,
            "global" => self.set_global_value(&mut config.global, key, value)?,
            "timeouts" => self.set_timeout_value(&mut config.global.timeouts, key, value)?,
            "uploads" => self.set_upload_value(&mut config.global.uploads, key, value)?,
            "sessions" => self.set_session_value(&mut config.global.sessions, key, value)?,
            "cgi" => self.set_cgi_value(&mut config.global.cgi, key, value)?,
            "logging" => self.set_logging_value(&mut config.global.logging, key, value)?,
            "security" => self.set_security_value(&mut config.global.security, key, value)?,
            s if s.starts_with("vhost.") => {
                if let Some(ref mut vhost) = current_vhost {
                    self.set_vhost_value(vhost, key, value)?;
                }
            }
            s if s.starts_with("route.") => {
                if let Some(ref mut route) = current_route {
                    self.set_route_value(route, key, value)?;
                }
            }
            _ => {
                // Unknown section, ignore
            }
        }
        Ok(())
    }
    
    /// Set server-level configuration value
    fn set_server_value(&self, config: &mut ServerConfig, key: &str, value: &str) -> io::Result<()> {
        match key {
            "default_host" => config.default_host = Some(value.to_string()),
            "listen" => {
                // Parse "address:port" format
                if let Some(colon_pos) = value.find(':') {
                    let address = value[..colon_pos].to_string();
                    let port = value[colon_pos + 1..].parse::<u16>()
                        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid port"))?;
                    
                    config.listeners.push(ListenerConfig {
                        address,
                        port,
                        default: config.listeners.is_empty(),
                    });
                }
            }
            _ => {}
        }
        Ok(())
    }
    
    /// Set global configuration value
    fn set_global_value(&self, global: &mut GlobalConfig, key: &str, value: &str) -> io::Result<()> {
        match key {
            "server_name" => global.server_name = value.to_string(),
            "workers" => global.workers = value.parse().unwrap_or(1),
            _ => {}
        }
        Ok(())
    }
    
    /// Set timeout configuration value
    fn set_timeout_value(&self, timeouts: &mut TimeoutConfig, key: &str, value: &str) -> io::Result<()> {
        let duration = self.parse_duration(value)?;
        match key {
            "read_header" => timeouts.read_header = duration,
            "read_body" => timeouts.read_body = duration,
            "write" => timeouts.write = duration,
            "keep_alive" => timeouts.keep_alive = duration,
            "request" => timeouts.request = duration,
            _ => {}
        }
        Ok(())
    }
    
    /// Set upload configuration value
    fn set_upload_value(&self, uploads: &mut UploadConfig, key: &str, value: &str) -> io::Result<()> {
        match key {
            "directory" => uploads.directory = PathBuf::from(value),
            "max_file_size" => uploads.max_file_size = self.parse_size(value)?,
            "max_total_size" => uploads.max_total_size = self.parse_size(value)?,
            "allowed_extensions" => {
                uploads.allowed_extensions = Some(
                    value.split(',').map(|s| s.trim().to_string()).collect()
                );
            }
            _ => {}
        }
        Ok(())
    }
    
    /// Set session configuration value
    fn set_session_value(&self, sessions: &mut SessionConfig, key: &str, value: &str) -> io::Result<()> {
        match key {
            "cookie_name" => sessions.cookie_name = value.to_string(),
            "expiration" => sessions.expiration = self.parse_duration(value)?,
            "secure_cookies" => sessions.secure_cookies = self.parse_bool(value),
            "http_only" => sessions.http_only = self.parse_bool(value),
            "same_site" => sessions.same_site = value.to_string(),
            "cleanup_interval" => sessions.cleanup_interval = self.parse_duration(value)?,
            "max_sessions" => sessions.max_sessions = value.parse().unwrap_or(10000),
            _ => {}
        }
        Ok(())
    }
    
    /// Set CGI configuration value
    fn set_cgi_value(&self, cgi: &mut CgiConfig, key: &str, value: &str) -> io::Result<()> {
        match key {
            "enabled" => cgi.enabled = self.parse_bool(value),
            "directory" => cgi.directory = PathBuf::from(value),
            "timeout" => cgi.timeout = self.parse_duration(value)?,
            "max_output_size" => cgi.max_output_size = self.parse_size(value)?,
            k if k.starts_with("interpreter.") => {
                let ext = &k[12..]; // Remove "interpreter." prefix
                cgi.interpreters.insert(ext.to_string(), value.to_string());
            }
            _ => {}
        }
        Ok(())
    }
    
    /// Set logging configuration value
    fn set_logging_value(&self, logging: &mut LoggingConfig, key: &str, value: &str) -> io::Result<()> {
        match key {
            "level" => logging.level = value.to_string(),
            "access_log" => logging.access_log = Some(PathBuf::from(value)),
            "error_log" => logging.error_log = Some(PathBuf::from(value)),
            _ => {}
        }
        Ok(())
    }
    
    /// Set security configuration value
    fn set_security_value(&self, security: &mut SecurityConfig, key: &str, value: &str) -> io::Result<()> {
        match key {
            "hide_version" => security.hide_version = self.parse_bool(value),
            "max_header_size" => security.max_header_size = self.parse_size(value)?,
            "max_headers" => security.max_headers = value.parse().unwrap_or(100),
            _ => {}
        }
        Ok(())
    }
    
    /// Set virtual host configuration value
    fn set_vhost_value(&self, vhost: &mut VirtualHostConfig, key: &str, value: &str) -> io::Result<()> {
        match key {
            "server_name" => vhost.server_name = value.to_string(),
            "document_root" => vhost.document_root = PathBuf::from(value),
            "max_body_size" => vhost.max_body_size = self.parse_size(value)?,
            "access_log" => vhost.access_log = Some(PathBuf::from(value)),
            "error_log" => vhost.error_log = Some(PathBuf::from(value)),
            _ => {}
        }
        Ok(())
    }
    
    /// Set route configuration value
    fn set_route_value(&self, route: &mut RouteConfig, key: &str, value: &str) -> io::Result<()> {
        match key {
            "path" => route.path = value.to_string(),
            "methods" => {
                route.methods = value.split(',').map(|s| s.trim().to_uppercase()).collect();
            }
            "type" => {
                route.route_type = match value {
                    "static" => RouteType::Static {
                        directory_listing: false,
                        index_files: vec!["index.html".to_string()],
                        cache_control: Some("public, max-age=3600".to_string()),
                    },
                    "cgi" => RouteType::Cgi {
                        script_dir: PathBuf::from("cgi-bin"),
                        interpreters: HashMap::new(),
                        timeout: Duration::from_secs(30),
                    },
                    "redirect" => RouteType::Redirect {
                        target: "/".to_string(),
                        status: 302,
                    },
                    _ => return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Unknown route type: {}", value),
                    )),
                };
            }
            _ => {}
        }
        Ok(())
    }
    
    /// Parse duration from string (e.g., "30s", "5m", "1h")
    fn parse_duration(&self, value: &str) -> io::Result<Duration> {
        if value.ends_with('s') {
            let secs = value[..value.len()-1].parse::<u64>()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid duration"))?;
            Ok(Duration::from_secs(secs))
        } else if value.ends_with('m') {
            let mins = value[..value.len()-1].parse::<u64>()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid duration"))?;
            Ok(Duration::from_secs(mins * 60))
        } else if value.ends_with('h') {
            let hours = value[..value.len()-1].parse::<u64>()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid duration"))?;
            Ok(Duration::from_secs(hours * 3600))
        } else {
            // Assume seconds
            let secs = value.parse::<u64>()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid duration"))?;
            Ok(Duration::from_secs(secs))
        }
    }
    
    /// Parse size from string (e.g., "1MB", "512KB", "2GB")
    fn parse_size(&self, value: &str) -> io::Result<usize> {
        let value = value.to_uppercase();
        if value.ends_with("GB") {
            let gb = value[..value.len()-2].parse::<usize>()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid size"))?;
            Ok(gb * 1024 * 1024 * 1024)
        } else if value.ends_with("MB") {
            let mb = value[..value.len()-2].parse::<usize>()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid size"))?;
            Ok(mb * 1024 * 1024)
        } else if value.ends_with("KB") {
            let kb = value[..value.len()-2].parse::<usize>()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid size"))?;
            Ok(kb * 1024)
        } else if value.ends_with("B") {
            let bytes = value[..value.len()-1].parse::<usize>()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid size"))?;
            Ok(bytes)
        } else {
            // Assume bytes
            value.parse::<usize>()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid size"))
        }
    }
    
    /// Parse boolean from string
    fn parse_bool(&self, value: &str) -> bool {
        match value.to_lowercase().as_str() {
            "true" | "yes" | "1" | "on" => true,
            _ => false,
        }
    }
    
    /// Generate example configuration file
    pub fn generate_example_config() -> String {
        r#"# Localhost HTTP Server Configuration
# This is an example configuration file in TOML format

[server]
# Default virtual host
default_host = "localhost"

# Server listeners (can have multiple)
listen = "127.0.0.1:8080"
listen = "0.0.0.0:8080"

[global]
# Server identification
server_name = "localhost/1.0"
# Number of worker processes (always 1 for this server)
workers = 1

[timeouts]
# Connection timeouts
read_header = "5s"
read_body = "15s"
write = "5s"
keep_alive = "10s"
request = "30s"

[uploads]
# File upload settings
directory = "./uploads"
max_file_size = "10MB"
max_total_size = "100MB"
allowed_extensions = "jpg,png,gif,pdf,txt,zip"

[sessions]
# Session management
cookie_name = "session_id"
expiration = "1h"
secure_cookies = false
http_only = true
same_site = "Lax"
cleanup_interval = "5m"
max_sessions = 10000

[cgi]
# CGI script execution
enabled = true
directory = "./cgi-bin"
timeout = "30s"
max_output_size = "1MB"

# CGI interpreters
interpreter.py = "python3"
interpreter.pl = "perl"
interpreter.sh = "sh"
interpreter.rb = "ruby"
interpreter.php = "php"

[logging]
# Logging configuration
level = "info"
access_log = "access.log"
error_log = "error.log"

[security]
# Security settings
hide_version = false
max_header_size = "8KB"
max_headers = 100

# Virtual host configuration
[vhost.localhost]
server_name = "localhost"
document_root = "./www"
max_body_size = "10MB"
access_log = "localhost_access.log"
error_log = "localhost_error.log"

# Route configuration for localhost
[route.root]
path = "/"
methods = "GET,POST,HEAD"
type = "static"

[route.cgi]
path = "/cgi-bin/*"
methods = "GET,POST"
type = "cgi"

[route.uploads]
path = "/uploads/*"
methods = "GET,POST,DELETE"
type = "static"

# Another virtual host example
[vhost.example.com]
server_name = "example.com"
document_root = "./sites/example"
max_body_size = "5MB"

[route.example_root]
path = "/"
methods = "GET,HEAD"
type = "static"

[route.api]
path = "/api/*"
methods = "GET,POST,PUT,DELETE"
type = "cgi"
"#.to_string()
    }
}

impl Default for ConfigParser {
    fn default() -> Self {
        Self::new(ConfigFormat::Auto)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_parser_creation() {
        let parser = ConfigParser::new(ConfigFormat::Toml);
        assert_eq!(parser.format, ConfigFormat::Toml);
    }
    
    #[test]
    fn test_parse_duration() {
        let parser = ConfigParser::default();
        
        assert_eq!(parser.parse_duration("30s").unwrap(), Duration::from_secs(30));
        assert_eq!(parser.parse_duration("5m").unwrap(), Duration::from_secs(300));
        assert_eq!(parser.parse_duration("1h").unwrap(), Duration::from_secs(3600));
        assert_eq!(parser.parse_duration("60").unwrap(), Duration::from_secs(60));
    }
    
    #[test]
    fn test_parse_size() {
        let parser = ConfigParser::default();
        
        assert_eq!(parser.parse_size("1024").unwrap(), 1024);
        assert_eq!(parser.parse_size("1KB").unwrap(), 1024);
        assert_eq!(parser.parse_size("1MB").unwrap(), 1024 * 1024);
        assert_eq!(parser.parse_size("1GB").unwrap(), 1024 * 1024 * 1024);
    }
    
    #[test]
    fn test_parse_bool() {
        let parser = ConfigParser::default();
        
        assert!(parser.parse_bool("true"));
        assert!(parser.parse_bool("yes"));
        assert!(parser.parse_bool("1"));
        assert!(parser.parse_bool("on"));
        assert!(!parser.parse_bool("false"));
        assert!(!parser.parse_bool("no"));
        assert!(!parser.parse_bool("0"));
    }
    
    #[test]
    fn test_parse_value() {
        let parser = ConfigParser::default();
        
        assert_eq!(parser.parse_value("\"hello\""), "hello");
        assert_eq!(parser.parse_value("'world'"), "world");
        assert_eq!(parser.parse_value("test"), "test");
    }
    
    #[test]
    fn test_detect_format() {
        let parser = ConfigParser::default();
        
        assert_eq!(parser.detect_format(Path::new("config.toml")).unwrap(), ConfigFormat::Toml);
        assert_eq!(parser.detect_format(Path::new("config.json")).unwrap(), ConfigFormat::Json);
        assert_eq!(parser.detect_format(Path::new("config")).unwrap(), ConfigFormat::Toml);
    }
    
    #[test]
    fn test_generate_example_config() {
        let example = ConfigParser::generate_example_config();
        assert!(example.contains("[server]"));
        assert!(example.contains("[global]"));
        assert!(example.contains("[vhost.localhost]"));
        assert!(example.contains("listen = \"127.0.0.1:8080\""));
    }
}
