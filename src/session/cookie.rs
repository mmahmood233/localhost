use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// SameSite attribute for cookies
#[derive(Debug, Clone, PartialEq)]
pub enum SameSite {
    Strict,
    Lax,
    None,
}

impl fmt::Display for SameSite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SameSite::Strict => write!(f, "Strict"),
            SameSite::Lax => write!(f, "Lax"),
            SameSite::None => write!(f, "None"),
        }
    }
}

/// HTTP Cookie representation
#[derive(Debug, Clone)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: Option<String>,
    pub path: Option<String>,
    pub expires: Option<SystemTime>,
    pub max_age: Option<Duration>,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: Option<SameSite>,
}

impl Cookie {
    pub fn new(name: String, value: String) -> Self {
        Cookie {
            name,
            value,
            domain: None,
            path: None,
            expires: None,
            max_age: None,
            secure: false,
            http_only: false,
            same_site: None,
        }
    }
    
    /// Create a session cookie (expires when browser closes)
    pub fn session(name: String, value: String) -> Self {
        let mut cookie = Cookie::new(name, value);
        cookie.http_only = true;
        cookie.same_site = Some(SameSite::Lax);
        cookie
    }
    
    /// Set cookie domain
    pub fn domain(mut self, domain: String) -> Self {
        self.domain = Some(domain);
        self
    }
    
    /// Set cookie path
    pub fn path(mut self, path: String) -> Self {
        self.path = Some(path);
        self
    }
    
    /// Set cookie expiration time
    pub fn expires(mut self, expires: SystemTime) -> Self {
        self.expires = Some(expires);
        self
    }
    
    /// Set cookie max age
    pub fn max_age(mut self, max_age: Duration) -> Self {
        self.max_age = Some(max_age);
        self
    }
    
    /// Set cookie as secure (HTTPS only)
    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }
    
    /// Set cookie as HTTP only (no JavaScript access)
    pub fn http_only(mut self, http_only: bool) -> Self {
        self.http_only = http_only;
        self
    }
    
    /// Set SameSite attribute
    pub fn same_site(mut self, same_site: SameSite) -> Self {
        self.same_site = Some(same_site);
        self
    }
    
    /// Convert cookie to Set-Cookie header value
    pub fn to_header_value(&self) -> String {
        let mut parts = vec![format!("{}={}", self.name, self.value)];
        
        if let Some(ref domain) = self.domain {
            parts.push(format!("Domain={}", domain));
        }
        
        if let Some(ref path) = self.path {
            parts.push(format!("Path={}", path));
        }
        
        if let Some(expires) = self.expires {
            if let Ok(duration) = expires.duration_since(UNIX_EPOCH) {
                // Format as HTTP date (RFC 7231)
                parts.push(format!("Expires={}", format_http_date(duration.as_secs())));
            }
        }
        
        if let Some(max_age) = self.max_age {
            parts.push(format!("Max-Age={}", max_age.as_secs()));
        }
        
        if self.secure {
            parts.push("Secure".to_string());
        }
        
        if self.http_only {
            parts.push("HttpOnly".to_string());
        }
        
        if let Some(ref same_site) = self.same_site {
            parts.push(format!("SameSite={}", same_site));
        }
        
        parts.join("; ")
    }
}

/// Cookie jar for managing multiple cookies
#[derive(Debug, Clone)]
pub struct CookieJar {
    cookies: HashMap<String, Cookie>,
}

impl CookieJar {
    pub fn new() -> Self {
        CookieJar {
            cookies: HashMap::new(),
        }
    }
    
    /// Parse cookies from Cookie header value
    pub fn parse_cookie_header(header_value: &str) -> Self {
        let mut jar = CookieJar::new();
        
        // Parse "name1=value1; name2=value2" format
        for pair in header_value.split(';') {
            let pair = pair.trim();
            if let Some(eq_pos) = pair.find('=') {
                let name = pair[..eq_pos].trim().to_string();
                let value = pair[eq_pos + 1..].trim().to_string();
                
                if !name.is_empty() {
                    jar.add_cookie(Cookie::new(name, value));
                }
            }
        }
        
        jar
    }
    
    /// Add a cookie to the jar
    pub fn add_cookie(&mut self, cookie: Cookie) {
        self.cookies.insert(cookie.name.clone(), cookie);
    }
    
    /// Get a cookie by name
    pub fn get_cookie(&self, name: &str) -> Option<&Cookie> {
        self.cookies.get(name)
    }
    
    /// Get cookie value by name
    pub fn get_value(&self, name: &str) -> Option<&str> {
        self.cookies.get(name).map(|c| c.value.as_str())
    }
    
    /// Remove a cookie by name
    pub fn remove_cookie(&mut self, name: &str) -> Option<Cookie> {
        self.cookies.remove(name)
    }
    
    /// Get all cookies
    pub fn cookies(&self) -> Vec<&Cookie> {
        self.cookies.values().collect()
    }
    
    /// Check if jar contains a cookie
    pub fn contains(&self, name: &str) -> bool {
        self.cookies.contains_key(name)
    }
    
    /// Get all Set-Cookie header values
    pub fn to_set_cookie_headers(&self) -> Vec<String> {
        self.cookies.values().map(|c| c.to_header_value()).collect()
    }
}

impl Default for CookieJar {
    fn default() -> Self {
        Self::new()
    }
}

/// Format Unix timestamp as HTTP date
fn format_http_date(timestamp: u64) -> String {
    // Simple HTTP date formatting (RFC 7231)
    // In a real implementation, you'd use a proper date formatting library
    let days = timestamp / 86400;
    let hours = (timestamp % 86400) / 3600;
    let minutes = (timestamp % 3600) / 60;
    let seconds = timestamp % 60;
    
    // This is a simplified format - in production use proper RFC 7231 formatting
    format!("Thu, 01 Jan 1970 {:02}:{:02}:{:02} GMT", hours, minutes, seconds)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cookie_creation() {
        let cookie = Cookie::new("session_id".to_string(), "abc123".to_string());
        assert_eq!(cookie.name, "session_id");
        assert_eq!(cookie.value, "abc123");
        assert!(!cookie.secure);
        assert!(!cookie.http_only);
    }
    
    #[test]
    fn test_session_cookie() {
        let cookie = Cookie::session("session_id".to_string(), "abc123".to_string());
        assert!(cookie.http_only);
        assert_eq!(cookie.same_site, Some(SameSite::Lax));
    }
    
    #[test]
    fn test_cookie_header_value() {
        let cookie = Cookie::new("test".to_string(), "value".to_string())
            .path("/".to_string())
            .http_only(true)
            .secure(true);
        
        let header = cookie.to_header_value();
        assert!(header.contains("test=value"));
        assert!(header.contains("Path=/"));
        assert!(header.contains("HttpOnly"));
        assert!(header.contains("Secure"));
    }
    
    #[test]
    fn test_cookie_jar_parsing() {
        let jar = CookieJar::parse_cookie_header("session_id=abc123; user_pref=dark_mode");
        
        assert_eq!(jar.get_value("session_id"), Some("abc123"));
        assert_eq!(jar.get_value("user_pref"), Some("dark_mode"));
        assert_eq!(jar.get_value("nonexistent"), None);
    }
    
    #[test]
    fn test_cookie_jar_operations() {
        let mut jar = CookieJar::new();
        
        jar.add_cookie(Cookie::new("test1".to_string(), "value1".to_string()));
        jar.add_cookie(Cookie::new("test2".to_string(), "value2".to_string()));
        
        assert!(jar.contains("test1"));
        assert!(jar.contains("test2"));
        assert!(!jar.contains("test3"));
        
        assert_eq!(jar.cookies().len(), 2);
        
        jar.remove_cookie("test1");
        assert!(!jar.contains("test1"));
        assert_eq!(jar.cookies().len(), 1);
    }
    
    #[test]
    fn test_same_site_display() {
        assert_eq!(SameSite::Strict.to_string(), "Strict");
        assert_eq!(SameSite::Lax.to_string(), "Lax");
        assert_eq!(SameSite::None.to_string(), "None");
    }
}
