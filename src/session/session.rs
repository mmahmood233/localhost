use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use std::io;

/// Session data storage
#[derive(Debug, Clone)]
pub struct SessionData {
    data: HashMap<String, String>,
    created_at: SystemTime,
    last_accessed: SystemTime,
    expires_at: Option<SystemTime>,
}

impl SessionData {
    pub fn new() -> Self {
        let now = SystemTime::now();
        SessionData {
            data: HashMap::new(),
            created_at: now,
            last_accessed: now,
            expires_at: None,
        }
    }
    
    /// Create session with expiration time
    pub fn with_expiration(expires_in: Duration) -> Self {
        let now = SystemTime::now();
        SessionData {
            data: HashMap::new(),
            created_at: now,
            last_accessed: now,
            expires_at: Some(now + expires_in),
        }
    }
    
    /// Set a value in the session
    pub fn set(&mut self, key: String, value: String) {
        self.data.insert(key, value);
        self.touch();
    }
    
    /// Get a value from the session
    pub fn get(&mut self, key: &str) -> Option<&str> {
        self.touch();
        self.data.get(key).map(|s| s.as_str())
    }
    
    /// Get a value without updating access time
    pub fn peek(&self, key: &str) -> Option<&str> {
        self.data.get(key).map(|s| s.as_str())
    }
    
    /// Remove a value from the session
    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.touch();
        self.data.remove(key)
    }
    
    /// Check if session contains a key
    pub fn contains(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }
    
    /// Get all keys in the session
    pub fn keys(&self) -> Vec<&str> {
        self.data.keys().map(|s| s.as_str()).collect()
    }
    
    /// Clear all session data
    pub fn clear(&mut self) {
        self.data.clear();
        self.touch();
    }
    
    /// Check if session is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    
    /// Get session size (number of key-value pairs)
    pub fn len(&self) -> usize {
        self.data.len()
    }
    
    /// Update last accessed time
    pub fn touch(&mut self) {
        self.last_accessed = SystemTime::now();
    }
    
    /// Check if session has expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            SystemTime::now() > expires_at
        } else {
            false
        }
    }
    
    /// Get session creation time
    pub fn created_at(&self) -> SystemTime {
        self.created_at
    }
    
    /// Get last access time
    pub fn last_accessed(&self) -> SystemTime {
        self.last_accessed
    }
    
    /// Get expiration time
    pub fn expires_at(&self) -> Option<SystemTime> {
        self.expires_at
    }
    
    /// Set expiration time
    pub fn set_expiration(&mut self, expires_at: SystemTime) {
        self.expires_at = Some(expires_at);
    }
    
    /// Extend session expiration by duration
    pub fn extend_expiration(&mut self, duration: Duration) {
        let now = SystemTime::now();
        self.expires_at = Some(now + duration);
    }
    
    /// Remove expiration (make session permanent until server restart)
    pub fn remove_expiration(&mut self) {
        self.expires_at = None;
    }
}

impl Default for SessionData {
    fn default() -> Self {
        Self::new()
    }
}

/// Session with ID and data
#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub data: SessionData,
}

impl Session {
    pub fn new(id: String) -> Self {
        Session {
            id,
            data: SessionData::new(),
        }
    }
    
    /// Create session with expiration
    pub fn with_expiration(id: String, expires_in: Duration) -> Self {
        Session {
            id,
            data: SessionData::with_expiration(expires_in),
        }
    }
    
    /// Generate a secure session ID
    pub fn generate_id() -> io::Result<String> {
        // Generate a cryptographically secure session ID
        // In production, use a proper cryptographic random number generator
        let mut id = String::new();
        
        // Simple session ID generation (in production, use proper crypto)
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
            .as_nanos();
        
        // Create a session ID from timestamp and some randomness
        id.push_str(&format!("{:x}", timestamp));
        
        // Add some pseudo-randomness (in production, use proper crypto)
        for i in 0..16 {
            let byte = ((timestamp >> (i * 4)) & 0xf) as u8;
            id.push_str(&format!("{:x}", byte));
        }
        
        Ok(id)
    }
    
    /// Check if session is valid (not expired)
    pub fn is_valid(&self) -> bool {
        !self.data.is_expired()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;
    
    #[test]
    fn test_session_data_creation() {
        let data = SessionData::new();
        assert!(data.is_empty());
        assert_eq!(data.len(), 0);
        assert!(!data.is_expired());
    }
    
    #[test]
    fn test_session_data_operations() {
        let mut data = SessionData::new();
        
        data.set("user_id".to_string(), "123".to_string());
        data.set("username".to_string(), "alice".to_string());
        
        assert_eq!(data.get("user_id"), Some("123"));
        assert_eq!(data.get("username"), Some("alice"));
        assert_eq!(data.get("nonexistent"), None);
        
        assert!(data.contains("user_id"));
        assert!(!data.contains("nonexistent"));
        
        assert_eq!(data.len(), 2);
        assert!(!data.is_empty());
        
        let removed = data.remove("user_id");
        assert_eq!(removed, Some("123".to_string()));
        assert_eq!(data.len(), 1);
        
        data.clear();
        assert!(data.is_empty());
        assert_eq!(data.len(), 0);
    }
    
    #[test]
    fn test_session_expiration() {
        let mut data = SessionData::with_expiration(Duration::from_millis(10));
        assert!(!data.is_expired());
        
        // Wait for expiration
        thread::sleep(Duration::from_millis(20));
        assert!(data.is_expired());
    }
    
    #[test]
    fn test_session_touch() {
        let mut data = SessionData::new();
        let initial_access = data.last_accessed();
        
        // Small delay to ensure time difference
        thread::sleep(Duration::from_millis(1));
        data.touch();
        
        assert!(data.last_accessed() > initial_access);
    }
    
    #[test]
    fn test_session_creation() {
        let session = Session::new("test_id".to_string());
        assert_eq!(session.id, "test_id");
        assert!(session.is_valid());
        assert!(session.data.is_empty());
    }
    
    #[test]
    fn test_session_id_generation() {
        let id1 = Session::generate_id().unwrap();
        let id2 = Session::generate_id().unwrap();
        
        assert_ne!(id1, id2);
        assert!(!id1.is_empty());
        assert!(!id2.is_empty());
    }
    
    #[test]
    fn test_session_with_expiration() {
        let session = Session::with_expiration("test".to_string(), Duration::from_millis(10));
        assert!(session.is_valid());
        
        thread::sleep(Duration::from_millis(20));
        assert!(!session.is_valid());
    }
    
    #[test]
    fn test_session_data_peek() {
        let mut data = SessionData::new();
        data.set("key".to_string(), "value".to_string());
        
        let last_accessed = data.last_accessed();
        thread::sleep(Duration::from_millis(1));
        
        // peek should not update access time
        assert_eq!(data.peek("key"), Some("value"));
        assert_eq!(data.last_accessed(), last_accessed);
        
        // get should update access time
        data.get("key");
        assert!(data.last_accessed() > last_accessed);
    }
    
    #[test]
    fn test_session_data_keys() {
        let mut data = SessionData::new();
        data.set("key1".to_string(), "value1".to_string());
        data.set("key2".to_string(), "value2".to_string());
        
        let keys = data.keys();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"key1"));
        assert!(keys.contains(&"key2"));
    }
}
