use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Main server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Server listeners (IP:port combinations)
    pub listeners: Vec<ListenerConfig>,
    /// Virtual hosts configuration
    pub virtual_hosts: Vec<VirtualHostConfig>,
    /// Default virtual host (fallback)
    pub default_host: Option<String>,
    /// Global server settings
    pub global: GlobalConfig,
}

/// Listener configuration (IP:port binding)
#[derive(Debug, Clone)]
pub struct ListenerConfig {
    /// IP address to bind to
    pub address: String,
    /// Port to bind to
    pub port: u16,
    /// Whether this is the default listener
    pub default: bool,
}

/// Virtual host configuration
#[derive(Debug, Clone)]
pub struct VirtualHostConfig {
    /// Server name (domain)
    pub server_name: String,
    /// Document root directory
    pub document_root: PathBuf,
    /// Routes for this virtual host
    pub routes: Vec<RouteConfig>,
    /// Custom error pages
    pub error_pages: HashMap<u16, PathBuf>,
    /// Maximum request body size
    pub max_body_size: usize,
    /// Access log file
    pub access_log: Option<PathBuf>,
    /// Error log file
    pub error_log: Option<PathBuf>,
}

/// Route configuration
#[derive(Debug, Clone)]
pub struct RouteConfig {
    /// Path pattern (e.g., "/api/*", "/static/")
    pub path: String,
    /// Allowed HTTP methods
    pub methods: Vec<String>,
    /// Route type (static, cgi, redirect, proxy)
    pub route_type: RouteType,
    /// Route-specific settings
    pub settings: RouteSettings,
}

/// Route type enumeration
#[derive(Debug, Clone)]
pub enum RouteType {
    /// Static file serving
    Static {
        /// Directory listing enabled
        directory_listing: bool,
        /// Index files (e.g., index.html)
        index_files: Vec<String>,
        /// Cache control headers
        cache_control: Option<String>,
    },
    /// CGI script execution
    Cgi {
        /// CGI script directory
        script_dir: PathBuf,
        /// Supported extensions and interpreters
        interpreters: HashMap<String, String>,
        /// Execution timeout
        timeout: Duration,
    },
    /// HTTP redirect
    Redirect {
        /// Target URL
        target: String,
        /// HTTP status code (301, 302, etc.)
        status: u16,
    },
    /// Reverse proxy (future feature)
    Proxy {
        /// Backend server URL
        backend: String,
        /// Timeout for backend requests
        timeout: Duration,
    },
}

/// Route-specific settings
#[derive(Debug, Clone)]
pub struct RouteSettings {
    /// Maximum request body size for this route
    pub max_body_size: Option<usize>,
    /// Custom error pages for this route
    pub error_pages: HashMap<u16, PathBuf>,
    /// Authentication required
    pub auth_required: bool,
    /// Rate limiting (requests per minute)
    pub rate_limit: Option<u32>,
    /// Custom headers to add
    pub custom_headers: HashMap<String, String>,
}

/// Global server configuration
#[derive(Debug, Clone)]
pub struct GlobalConfig {
    /// Server identification string
    pub server_name: String,
    /// Worker process/thread count
    pub workers: usize,
    /// Connection timeouts
    pub timeouts: TimeoutConfig,
    /// File upload settings
    pub uploads: UploadConfig,
    /// Session management settings
    pub sessions: SessionConfig,
    /// CGI settings
    pub cgi: CgiConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Security settings
    pub security: SecurityConfig,
}

/// Timeout configuration
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    /// Read header timeout
    pub read_header: Duration,
    /// Read body timeout
    pub read_body: Duration,
    /// Write timeout
    pub write: Duration,
    /// Keep-alive timeout
    pub keep_alive: Duration,
    /// Overall request timeout
    pub request: Duration,
}

/// Upload configuration
#[derive(Debug, Clone)]
pub struct UploadConfig {
    /// Upload directory
    pub directory: PathBuf,
    /// Maximum file size
    pub max_file_size: usize,
    /// Maximum total upload size
    pub max_total_size: usize,
    /// Allowed file extensions
    pub allowed_extensions: Option<Vec<String>>,
    /// Temporary directory for uploads
    pub temp_directory: Option<PathBuf>,
}

/// Session configuration
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Session cookie name
    pub cookie_name: String,
    /// Session expiration time
    pub expiration: Duration,
    /// Cookie security settings
    pub secure_cookies: bool,
    /// HTTP-only cookies
    pub http_only: bool,
    /// SameSite attribute
    pub same_site: String,
    /// Session cleanup interval
    pub cleanup_interval: Duration,
    /// Maximum number of sessions
    pub max_sessions: usize,
}

/// CGI configuration
#[derive(Debug, Clone)]
pub struct CgiConfig {
    /// Whether CGI is enabled
    pub enabled: bool,
    /// CGI script directory
    pub directory: PathBuf,
    /// Script interpreters
    pub interpreters: HashMap<String, String>,
    /// Execution timeout
    pub timeout: Duration,
    /// Maximum output size
    pub max_output_size: usize,
}

/// Logging configuration
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Log level (error, warn, info, debug)
    pub level: String,
    /// Access log file
    pub access_log: Option<PathBuf>,
    /// Error log file
    pub error_log: Option<PathBuf>,
    /// Log rotation settings
    pub rotation: LogRotationConfig,
}

/// Log rotation configuration
#[derive(Debug, Clone)]
pub struct LogRotationConfig {
    /// Maximum log file size
    pub max_size: usize,
    /// Number of rotated files to keep
    pub max_files: usize,
    /// Rotation interval
    pub interval: Option<Duration>,
}

/// Security configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Hide server version in headers
    pub hide_version: bool,
    /// Maximum request header size
    pub max_header_size: usize,
    /// Maximum number of headers
    pub max_headers: usize,
    /// Rate limiting settings
    pub rate_limiting: RateLimitConfig,
    /// IP blacklist
    pub ip_blacklist: Vec<String>,
    /// IP whitelist (if set, only these IPs are allowed)
    pub ip_whitelist: Option<Vec<String>>,
}

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Whether rate limiting is enabled
    pub enabled: bool,
    /// Requests per minute per IP
    pub requests_per_minute: u32,
    /// Burst size
    pub burst_size: u32,
    /// Ban duration for rate limit violations
    pub ban_duration: Duration,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            listeners: vec![ListenerConfig {
                address: "127.0.0.1".to_string(),
                port: 8080,
                default: true,
            }],
            virtual_hosts: vec![VirtualHostConfig::default()],
            default_host: Some("localhost".to_string()),
            global: GlobalConfig::default(),
        }
    }
}

impl Default for VirtualHostConfig {
    fn default() -> Self {
        VirtualHostConfig {
            server_name: "localhost".to_string(),
            document_root: PathBuf::from("./www"),
            routes: vec![RouteConfig::default()],
            error_pages: HashMap::new(),
            max_body_size: 10 * 1024 * 1024, // 10MB
            access_log: None,
            error_log: None,
        }
    }
}

impl Default for RouteConfig {
    fn default() -> Self {
        RouteConfig {
            path: "/".to_string(),
            methods: vec!["GET".to_string(), "POST".to_string(), "HEAD".to_string()],
            route_type: RouteType::Static {
                directory_listing: false,
                index_files: vec!["index.html".to_string(), "index.htm".to_string()],
                cache_control: Some("public, max-age=3600".to_string()),
            },
            settings: RouteSettings::default(),
        }
    }
}

impl Default for RouteSettings {
    fn default() -> Self {
        RouteSettings {
            max_body_size: None,
            error_pages: HashMap::new(),
            auth_required: false,
            rate_limit: None,
            custom_headers: HashMap::new(),
        }
    }
}

impl Default for GlobalConfig {
    fn default() -> Self {
        GlobalConfig {
            server_name: "localhost/1.0".to_string(),
            workers: 1, // Single-threaded as per spec
            timeouts: TimeoutConfig::default(),
            uploads: UploadConfig::default(),
            sessions: SessionConfig::default(),
            cgi: CgiConfig::default(),
            logging: LoggingConfig::default(),
            security: SecurityConfig::default(),
        }
    }
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        TimeoutConfig {
            read_header: Duration::from_secs(5),
            read_body: Duration::from_secs(15),
            write: Duration::from_secs(5),
            keep_alive: Duration::from_secs(10),
            request: Duration::from_secs(30),
        }
    }
}

impl Default for UploadConfig {
    fn default() -> Self {
        UploadConfig {
            directory: PathBuf::from("./uploads"),
            max_file_size: 10 * 1024 * 1024, // 10MB
            max_total_size: 100 * 1024 * 1024, // 100MB
            allowed_extensions: None,
            temp_directory: None,
        }
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        SessionConfig {
            cookie_name: "session_id".to_string(),
            expiration: Duration::from_secs(3600), // 1 hour
            secure_cookies: false,
            http_only: true,
            same_site: "Lax".to_string(),
            cleanup_interval: Duration::from_secs(300), // 5 minutes
            max_sessions: 10000,
        }
    }
}

impl Default for CgiConfig {
    fn default() -> Self {
        let mut interpreters = HashMap::new();
        interpreters.insert("py".to_string(), "python3".to_string());
        interpreters.insert("pl".to_string(), "perl".to_string());
        interpreters.insert("sh".to_string(), "sh".to_string());
        interpreters.insert("rb".to_string(), "ruby".to_string());
        interpreters.insert("php".to_string(), "php".to_string());
        
        CgiConfig {
            enabled: true,
            directory: PathBuf::from("./cgi-bin"),
            interpreters,
            timeout: Duration::from_secs(30),
            max_output_size: 1024 * 1024, // 1MB
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        LoggingConfig {
            level: "info".to_string(),
            access_log: Some(PathBuf::from("access.log")),
            error_log: Some(PathBuf::from("error.log")),
            rotation: LogRotationConfig::default(),
        }
    }
}

impl Default for LogRotationConfig {
    fn default() -> Self {
        LogRotationConfig {
            max_size: 10 * 1024 * 1024, // 10MB
            max_files: 5,
            interval: Some(Duration::from_secs(24 * 3600)), // Daily
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        SecurityConfig {
            hide_version: false,
            max_header_size: 8192, // 8KB
            max_headers: 100,
            rate_limiting: RateLimitConfig::default(),
            ip_blacklist: Vec::new(),
            ip_whitelist: None,
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        RateLimitConfig {
            enabled: false,
            requests_per_minute: 60,
            burst_size: 10,
            ban_duration: Duration::from_secs(300), // 5 minutes
        }
    }
}
