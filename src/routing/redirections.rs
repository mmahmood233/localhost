use crate::http::response::HttpResponse;
use crate::http::request::HttpRequest;
use std::collections::HashMap;
use std::fmt;

/// HTTP redirect types
#[derive(Debug, Clone, PartialEq)]
pub enum RedirectType {
    /// 301 Moved Permanently - Resource permanently moved
    Permanent,
    /// 302 Found - Resource temporarily moved
    Temporary,
    /// 303 See Other - Redirect to different resource (POST -> GET)
    SeeOther,
    /// 307 Temporary Redirect - Preserve method and body
    TemporaryPreserve,
    /// 308 Permanent Redirect - Preserve method and body
    PermanentPreserve,
}

impl RedirectType {
    /// Get HTTP status code for redirect type
    pub fn status_code(&self) -> u16 {
        match self {
            RedirectType::Permanent => 301,
            RedirectType::Temporary => 302,
            RedirectType::SeeOther => 303,
            RedirectType::TemporaryPreserve => 307,
            RedirectType::PermanentPreserve => 308,
        }
    }
    
    /// Get status text for redirect type
    pub fn status_text(&self) -> &'static str {
        match self {
            RedirectType::Permanent => "Moved Permanently",
            RedirectType::Temporary => "Found",
            RedirectType::SeeOther => "See Other",
            RedirectType::TemporaryPreserve => "Temporary Redirect",
            RedirectType::PermanentPreserve => "Permanent Redirect",
        }
    }
    
    /// Check if redirect preserves request method and body
    pub fn preserves_method(&self) -> bool {
        matches!(self, RedirectType::TemporaryPreserve | RedirectType::PermanentPreserve)
    }
}

impl fmt::Display for RedirectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.status_text())
    }
}

/// Redirect rule configuration
#[derive(Debug, Clone)]
pub struct RedirectRule {
    /// Source pattern to match (supports wildcards and regex)
    pub from: String,
    /// Target URL or pattern
    pub to: String,
    /// Type of redirect
    pub redirect_type: RedirectType,
    /// Whether to preserve query parameters
    pub preserve_query: bool,
    /// Whether to preserve URL fragments
    pub preserve_fragment: bool,
    /// Conditions for applying redirect
    pub conditions: RedirectConditions,
    /// Whether rule is enabled
    pub enabled: bool,
}

impl Default for RedirectRule {
    fn default() -> Self {
        RedirectRule {
            from: String::new(),
            to: String::new(),
            redirect_type: RedirectType::Temporary,
            preserve_query: true,
            preserve_fragment: false,
            conditions: RedirectConditions::default(),
            enabled: true,
        }
    }
}

/// Conditions for applying redirects
#[derive(Debug, Clone, Default)]
pub struct RedirectConditions {
    /// HTTP methods to apply redirect to (empty = all methods)
    pub methods: Vec<String>,
    /// Required request headers
    pub headers: HashMap<String, String>,
    /// Required query parameters
    pub query_params: HashMap<String, String>,
    /// Host conditions
    pub hosts: Vec<String>,
    /// User agent patterns
    pub user_agents: Vec<String>,
    /// IP address patterns
    pub ip_addresses: Vec<String>,
}

/// Redirect engine for processing redirect rules
#[derive(Debug)]
pub struct RedirectEngine {
    /// Redirect rules in priority order
    rules: Vec<RedirectRule>,
    /// Cache for compiled patterns (simplified without regex)
    pattern_cache: HashMap<String, String>,
}

impl RedirectEngine {
    /// Create new redirect engine
    pub fn new() -> Self {
        RedirectEngine {
            rules: Vec::new(),
            pattern_cache: HashMap::new(),
        }
    }
    
    /// Add redirect rule
    pub fn add_rule(&mut self, rule: RedirectRule) {
        self.rules.push(rule);
    }
    
    /// Add multiple redirect rules
    pub fn add_rules(&mut self, rules: Vec<RedirectRule>) {
        self.rules.extend(rules);
    }
    
    /// Clear all redirect rules
    pub fn clear_rules(&mut self) {
        self.rules.clear();
        self.pattern_cache.clear();
    }
    
    /// Process request and return redirect response if applicable
    pub fn process_request(&mut self, request: &HttpRequest) -> Option<HttpResponse> {
        let rules = self.rules.clone(); // Clone to avoid borrow checker issues
        for rule in &rules {
            if !rule.enabled {
                continue;
            }
            
            if let Some(target) = self.match_rule(rule, request) {
                return Some(self.create_redirect_response(rule, &target));
            }
        }
        
        None
    }
    
    /// Check if rule matches request
    fn match_rule(&mut self, rule: &RedirectRule, request: &HttpRequest) -> Option<String> {
        // Check conditions first
        if !self.check_conditions(&rule.conditions, request) {
            return None;
        }
        
        // Check path pattern
        let path = request.path();
        if let Some(target) = self.match_pattern(&rule.from, path, &rule.to, request) {
            return Some(target);
        }
        
        None
    }
    
    /// Check if request meets redirect conditions
    fn check_conditions(&self, conditions: &RedirectConditions, request: &HttpRequest) -> bool {
        // Check HTTP method
        if !conditions.methods.is_empty() {
            let method = format!("{:?}", request.method()).to_uppercase();
            if !conditions.methods.iter().any(|m| m.to_uppercase() == method) {
                return false;
            }
        }
        
        // Check required headers
        for (header_name, expected_value) in &conditions.headers {
            if let Some(header_value) = request.headers.get(header_name) {
                if header_value != expected_value {
                    return false;
                }
            } else {
                return false;
            }
        }
        
        // Check query parameters
        if !conditions.query_params.is_empty() {
            let query_string = request.query_string.as_deref().unwrap_or("");
            let query_params = parse_query_string(query_string);
            
            for (param_name, expected_value) in &conditions.query_params {
                if let Some(param_value) = query_params.get(param_name) {
                    if param_value != expected_value {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }
        
        // Check host conditions
        if !conditions.hosts.is_empty() {
            if let Some(host) = request.headers.get("Host") {
                if !conditions.hosts.iter().any(|h| host.contains(h)) {
                    return false;
                }
            } else {
                return false;
            }
        }
        
        // Check user agent patterns
        if !conditions.user_agents.is_empty() {
            if let Some(user_agent) = request.headers.get("User-Agent") {
                if !conditions.user_agents.iter().any(|ua| user_agent.contains(ua)) {
                    return false;
                }
            } else {
                return false;
            }
        }
        
        // All conditions met
        true
    }
    
    /// Match pattern against path and generate target URL
    fn match_pattern(&mut self, pattern: &str, path: &str, target: &str, request: &HttpRequest) -> Option<String> {
        // Simple wildcard matching
        if pattern.contains('*') {
            if let Some(target_url) = self.match_wildcard(pattern, path, target) {
                return Some(self.build_target_url(&target_url, request));
            }
        }
        // Exact match
        else if pattern == path {
            return Some(self.build_target_url(target, request));
        }
        // Prefix match
        else if pattern.ends_with('/') && path.starts_with(pattern) {
            let suffix = &path[pattern.len()..];
            let target_url = if target.ends_with('/') {
                format!("{}{}", target, suffix)
            } else {
                format!("{}/{}", target, suffix)
            };
            return Some(self.build_target_url(&target_url, request));
        }
        // Regex match (if pattern starts with ^)
        else if pattern.starts_with('^') {
            if let Some(target_url) = self.match_regex(pattern, path, target) {
                return Some(self.build_target_url(&target_url, request));
            }
        }
        
        None
    }
    
    /// Match wildcard pattern
    fn match_wildcard(&self, pattern: &str, path: &str, target: &str) -> Option<String> {
        if pattern == "*" {
            return Some(target.to_string());
        }
        
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() != 2 {
            return None; // Only support one wildcard for now
        }
        
        let prefix = parts[0];
        let suffix = parts[1];
        
        if path.starts_with(prefix) && path.ends_with(suffix) {
            let captured = &path[prefix.len()..path.len() - suffix.len()];
            let result = target.replace("*", captured);
            return Some(result);
        }
        
        None
    }
    
    /// Match regex pattern (simplified without regex crate)
    fn match_regex(&mut self, pattern: &str, path: &str, target: &str) -> Option<String> {
        // Simplified pattern matching without regex crate
        // For now, just do basic pattern matching
        if pattern.starts_with('^') && pattern.ends_with('$') {
            let inner_pattern = &pattern[1..pattern.len()-1];
            if path == inner_pattern {
                return Some(target.to_string());
            }
        }
        
        None
    }
    
    /// Build complete target URL with query parameters and fragments
    fn build_target_url(&self, target: &str, request: &HttpRequest) -> String {
        let mut url = target.to_string();
        
        // Add query parameters if preserving
        if let Some(query) = &request.query_string {
            if !query.is_empty() {
                if url.contains('?') {
                    url.push('&');
                } else {
                    url.push('?');
                }
                url.push_str(query);
            }
        }
        
        url
    }
    
    /// Create redirect HTTP response
    fn create_redirect_response(&self, rule: &RedirectRule, target: &str) -> HttpResponse {
        let mut response = HttpResponse::new(rule.redirect_type.status_code());
        response.set_header("Location", target);
        response.set_header("Cache-Control", "no-cache");
        
        // Add redirect body for better user experience
        let body = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>Redirecting...</title>
    <meta http-equiv="refresh" content="0; url={}">
</head>
<body>
    <h1>Redirecting...</h1>
    <p>If you are not redirected automatically, <a href="{}">click here</a>.</p>
</body>
</html>"#,
            html_escape(target),
            html_escape(target)
        );
        
        response.set_body(body.as_bytes());
        response.set_header("Content-Type", "text/html; charset=utf-8");
        response.set_header("Content-Length", &body.len().to_string());
        
        response
    }
    
    /// Get all redirect rules
    pub fn rules(&self) -> &[RedirectRule] {
        &self.rules
    }
    
    /// Get redirect rules count
    pub fn rules_count(&self) -> usize {
        self.rules.len()
    }
    
    /// Enable/disable rule by index
    pub fn set_rule_enabled(&mut self, index: usize, enabled: bool) -> bool {
        if let Some(rule) = self.rules.get_mut(index) {
            rule.enabled = enabled;
            true
        } else {
            false
        }
    }
    
    /// Remove rule by index
    pub fn remove_rule(&mut self, index: usize) -> Option<RedirectRule> {
        if index < self.rules.len() {
            Some(self.rules.remove(index))
        } else {
            None
        }
    }
}

impl Default for RedirectEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Route-specific settings
#[derive(Debug, Clone)]
pub struct RouteSettings {
    /// Custom headers to add to responses
    pub custom_headers: HashMap<String, String>,
    /// CORS settings
    pub cors: CorsSettings,
    /// Security headers
    pub security_headers: SecurityHeaders,
    /// Caching settings
    pub cache_settings: CacheSettings,
    /// Rate limiting settings
    pub rate_limit: Option<RateLimitSettings>,
    /// Request/response modification
    pub modifications: RequestModifications,
}

impl Default for RouteSettings {
    fn default() -> Self {
        RouteSettings {
            custom_headers: HashMap::new(),
            cors: CorsSettings::default(),
            security_headers: SecurityHeaders::default(),
            cache_settings: CacheSettings::default(),
            rate_limit: None,
            modifications: RequestModifications::default(),
        }
    }
}

/// CORS (Cross-Origin Resource Sharing) settings
#[derive(Debug, Clone)]
pub struct CorsSettings {
    /// Allowed origins (* for all)
    pub allowed_origins: Vec<String>,
    /// Allowed methods
    pub allowed_methods: Vec<String>,
    /// Allowed headers
    pub allowed_headers: Vec<String>,
    /// Exposed headers
    pub exposed_headers: Vec<String>,
    /// Allow credentials
    pub allow_credentials: bool,
    /// Max age for preflight cache
    pub max_age: Option<u32>,
    /// Whether CORS is enabled
    pub enabled: bool,
}

impl Default for CorsSettings {
    fn default() -> Self {
        CorsSettings {
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec!["GET".to_string(), "POST".to_string(), "PUT".to_string(), "DELETE".to_string()],
            allowed_headers: vec!["Content-Type".to_string(), "Authorization".to_string()],
            exposed_headers: Vec::new(),
            allow_credentials: false,
            max_age: Some(86400), // 24 hours
            enabled: false,
        }
    }
}

/// Security headers configuration
#[derive(Debug, Clone)]
pub struct SecurityHeaders {
    /// X-Frame-Options header
    pub frame_options: Option<String>,
    /// X-Content-Type-Options header
    pub content_type_options: bool,
    /// X-XSS-Protection header
    pub xss_protection: Option<String>,
    /// Strict-Transport-Security header
    pub hsts: Option<String>,
    /// Content-Security-Policy header
    pub csp: Option<String>,
    /// Referrer-Policy header
    pub referrer_policy: Option<String>,
    /// Whether security headers are enabled
    pub enabled: bool,
}

impl Default for SecurityHeaders {
    fn default() -> Self {
        SecurityHeaders {
            frame_options: Some("DENY".to_string()),
            content_type_options: true,
            xss_protection: Some("1; mode=block".to_string()),
            hsts: None,
            csp: None,
            referrer_policy: Some("strict-origin-when-cross-origin".to_string()),
            enabled: true,
        }
    }
}

/// Cache settings for responses
#[derive(Debug, Clone)]
pub struct CacheSettings {
    /// Cache-Control header value
    pub cache_control: Option<String>,
    /// Expires header (duration in seconds)
    pub expires: Option<u32>,
    /// ETag generation
    pub etag_enabled: bool,
    /// Last-Modified header
    pub last_modified_enabled: bool,
    /// Whether caching is enabled
    pub enabled: bool,
}

impl Default for CacheSettings {
    fn default() -> Self {
        CacheSettings {
            cache_control: None,
            expires: None,
            etag_enabled: false,
            last_modified_enabled: true,
            enabled: false,
        }
    }
}

/// Rate limiting settings
#[derive(Debug, Clone)]
pub struct RateLimitSettings {
    /// Requests per minute
    pub requests_per_minute: u32,
    /// Burst size
    pub burst_size: u32,
    /// Time window in seconds
    pub window_size: u32,
    /// Whether to use IP-based limiting
    pub per_ip: bool,
}

impl Default for RateLimitSettings {
    fn default() -> Self {
        RateLimitSettings {
            requests_per_minute: 60,
            burst_size: 10,
            window_size: 60,
            per_ip: true,
        }
    }
}

/// Request/response modifications
#[derive(Debug, Clone, Default)]
pub struct RequestModifications {
    /// Headers to remove from requests
    pub remove_request_headers: Vec<String>,
    /// Headers to add to requests
    pub add_request_headers: HashMap<String, String>,
    /// Headers to remove from responses
    pub remove_response_headers: Vec<String>,
    /// Headers to add to responses
    pub add_response_headers: HashMap<String, String>,
}

/// Route settings processor
#[derive(Debug)]
pub struct RouteSettingsProcessor {
    /// Default settings
    default_settings: RouteSettings,
}

impl RouteSettingsProcessor {
    /// Create new route settings processor
    pub fn new(default_settings: RouteSettings) -> Self {
        RouteSettingsProcessor {
            default_settings,
        }
    }
    
    /// Apply route settings to response
    pub fn apply_settings(&self, response: &mut HttpResponse, settings: &RouteSettings, request: &HttpRequest) {
        // Apply custom headers
        for (name, value) in &settings.custom_headers {
            response.set_header(name, value);
        }
        
        // Apply CORS headers
        if settings.cors.enabled {
            self.apply_cors_headers(response, &settings.cors, request);
        }
        
        // Apply security headers
        if settings.security_headers.enabled {
            self.apply_security_headers(response, &settings.security_headers);
        }
        
        // Apply cache headers
        if settings.cache_settings.enabled {
            self.apply_cache_headers(response, &settings.cache_settings);
        }
        
        // Apply response modifications
        self.apply_response_modifications(response, &settings.modifications);
    }
    
    /// Apply CORS headers
    fn apply_cors_headers(&self, response: &mut HttpResponse, cors: &CorsSettings, request: &HttpRequest) {
        // Access-Control-Allow-Origin
        let origin = if cors.allowed_origins.contains(&"*".to_string()) {
            "*"
        } else if let Some(origin) = request.headers.get("Origin") {
            if cors.allowed_origins.iter().any(|allowed| allowed == origin) {
                origin
            } else {
                return; // Origin not allowed
            }
        } else {
            return;
        };
        
        response.set_header("Access-Control-Allow-Origin", origin);
        
        // Access-Control-Allow-Methods
        if !cors.allowed_methods.is_empty() {
            response.set_header("Access-Control-Allow-Methods", &cors.allowed_methods.join(", "));
        }
        
        // Access-Control-Allow-Headers
        if !cors.allowed_headers.is_empty() {
            response.set_header("Access-Control-Allow-Headers", &cors.allowed_headers.join(", "));
        }
        
        // Access-Control-Expose-Headers
        if !cors.exposed_headers.is_empty() {
            response.set_header("Access-Control-Expose-Headers", &cors.exposed_headers.join(", "));
        }
        
        // Access-Control-Allow-Credentials
        if cors.allow_credentials {
            response.set_header("Access-Control-Allow-Credentials", "true");
        }
        
        // Access-Control-Max-Age
        if let Some(max_age) = cors.max_age {
            response.set_header("Access-Control-Max-Age", &max_age.to_string());
        }
    }
    
    /// Apply security headers
    fn apply_security_headers(&self, response: &mut HttpResponse, security: &SecurityHeaders) {
        if let Some(ref frame_options) = security.frame_options {
            response.set_header("X-Frame-Options", frame_options);
        }
        
        if security.content_type_options {
            response.set_header("X-Content-Type-Options", "nosniff");
        }
        
        if let Some(ref xss_protection) = security.xss_protection {
            response.set_header("X-XSS-Protection", xss_protection);
        }
        
        if let Some(ref hsts) = security.hsts {
            response.set_header("Strict-Transport-Security", hsts);
        }
        
        if let Some(ref csp) = security.csp {
            response.set_header("Content-Security-Policy", csp);
        }
        
        if let Some(ref referrer_policy) = security.referrer_policy {
            response.set_header("Referrer-Policy", referrer_policy);
        }
    }
    
    /// Apply cache headers
    fn apply_cache_headers(&self, response: &mut HttpResponse, cache: &CacheSettings) {
        if let Some(ref cache_control) = cache.cache_control {
            response.set_header("Cache-Control", cache_control);
        }
        
        if let Some(expires) = cache.expires {
            let expires_time = std::time::SystemTime::now() + std::time::Duration::from_secs(expires as u64);
            if let Ok(duration) = expires_time.duration_since(std::time::UNIX_EPOCH) {
                response.set_header("Expires", &format_http_date(duration.as_secs()));
            }
        }
    }
    
    /// Apply response modifications
    fn apply_response_modifications(&self, response: &mut HttpResponse, modifications: &RequestModifications) {
        // Remove specified headers
        for header_name in &modifications.remove_response_headers {
            response.headers.remove(header_name);
        }
        
        // Add specified headers
        for (name, value) in &modifications.add_response_headers {
            response.set_header(name, value);
        }
    }
    
    /// Get default settings
    pub fn default_settings(&self) -> &RouteSettings {
        &self.default_settings
    }
}

/// Parse query string into key-value pairs
fn parse_query_string(query: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    
    for pair in query.split('&') {
        if let Some(eq_pos) = pair.find('=') {
            let key = &pair[..eq_pos];
            let value = &pair[eq_pos + 1..];
            params.insert(
                url_decode(key),
                url_decode(value),
            );
        } else if !pair.is_empty() {
            params.insert(
                url_decode(pair),
                String::new(),
            );
        }
    }
    
    params
}

/// Simple URL decoding without external dependencies
fn url_decode(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if ch == '%' {
            if let (Some(h1), Some(h2)) = (chars.next(), chars.next()) {
                if let (Some(d1), Some(d2)) = (h1.to_digit(16), h2.to_digit(16)) {
                    let byte = (d1 * 16 + d2) as u8;
                    if let Ok(decoded_char) = std::str::from_utf8(&[byte]) {
                        result.push_str(decoded_char);
                        continue;
                    }
                }
            }
            result.push(ch); // If decoding fails, keep the original character
        } else if ch == '+' {
            result.push(' ');
        } else {
            result.push(ch);
        }
    }
    
    result
}

/// HTML escape utility
fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Format HTTP date
fn format_http_date(timestamp: u64) -> String {
    // Simple HTTP date formatting
    format!("{}", timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::request::Method;
    
    #[test]
    fn test_redirect_types() {
        assert_eq!(RedirectType::Permanent.status_code(), 301);
        assert_eq!(RedirectType::Temporary.status_code(), 302);
        assert_eq!(RedirectType::SeeOther.status_code(), 303);
        assert_eq!(RedirectType::TemporaryPreserve.status_code(), 307);
        assert_eq!(RedirectType::PermanentPreserve.status_code(), 308);
        
        assert!(RedirectType::TemporaryPreserve.preserves_method());
        assert!(RedirectType::PermanentPreserve.preserves_method());
        assert!(!RedirectType::Temporary.preserves_method());
    }
    
    #[test]
    fn test_redirect_engine() {
        let mut engine = RedirectEngine::new();
        
        let rule = RedirectRule {
            from: "/old".to_string(),
            to: "/new".to_string(),
            redirect_type: RedirectType::Permanent,
            preserve_query: true,
            preserve_fragment: false,
            conditions: RedirectConditions::default(),
            enabled: true,
        };
        
        engine.add_rule(rule);
        assert_eq!(engine.rules_count(), 1);
    }
    
    #[test]
    fn test_wildcard_matching() {
        let engine = RedirectEngine::new();
        
        let result = engine.match_wildcard("/api/*", "/api/users", "/v2/*");
        assert_eq!(result, Some("/v2/users".to_string()));
        
        let result = engine.match_wildcard("*.html", "index.html", "*.php");
        assert_eq!(result, Some("index.php".to_string()));
    }
    
    #[test]
    fn test_query_string_parsing() {
        let params = parse_query_string("name=john&age=30&city=new%20york");
        
        assert_eq!(params.get("name"), Some(&"john".to_string()));
        assert_eq!(params.get("age"), Some(&"30".to_string()));
        assert_eq!(params.get("city"), Some(&"new york".to_string()));
    }
    
    #[test]
    fn test_cors_settings() {
        let cors = CorsSettings::default();
        assert!(cors.allowed_origins.contains(&"*".to_string()));
        assert!(cors.allowed_methods.contains(&"GET".to_string()));
        assert_eq!(cors.max_age, Some(86400));
    }
    
    #[test]
    fn test_security_headers() {
        let security = SecurityHeaders::default();
        assert_eq!(security.frame_options, Some("DENY".to_string()));
        assert!(security.content_type_options);
        assert_eq!(security.xss_protection, Some("1; mode=block".to_string()));
    }
}
