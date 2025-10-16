use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use std::io;

use super::session::{Session, SessionData};
use super::cookie::{Cookie, CookieJar};

/// Configuration for session store
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Default session expiration time
    pub default_expiration: Duration,
    /// Session cookie name
    pub cookie_name: String,
    /// Cookie path
    pub cookie_path: String,
    /// Cookie domain
    pub cookie_domain: Option<String>,
    /// Whether to use secure cookies (HTTPS only)
    pub secure_cookies: bool,
    /// Whether to use HTTP-only cookies
    pub http_only_cookies: bool,
    /// Cleanup interval for expired sessions
    pub cleanup_interval: Duration,
    /// Maximum number of sessions to store
    pub max_sessions: usize,
}

impl Default for SessionConfig {
    fn default() -> Self {
        SessionConfig {
            default_expiration: Duration::from_secs(3600), // 1 hour
            cookie_name: "session_id".to_string(),
            cookie_path: "/".to_string(),
            cookie_domain: None,
            secure_cookies: false, // Set to true in production with HTTPS
            http_only_cookies: true,
            cleanup_interval: Duration::from_secs(300), // 5 minutes
            max_sessions: 10000,
        }
    }
}

/// In-memory session store
#[derive(Debug)]
pub struct SessionStore {
    sessions: Arc<Mutex<HashMap<String, Session>>>,
    config: SessionConfig,
    last_cleanup: Arc<Mutex<SystemTime>>,
}

impl SessionStore {
    pub fn new(config: SessionConfig) -> Self {
        SessionStore {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            config,
            last_cleanup: Arc::new(Mutex::new(SystemTime::now())),
        }
    }
    
    /// Create a new session
    pub fn create_session(&self) -> io::Result<Session> {
        let session_id = Session::generate_id()?;
        let session = Session::with_expiration(session_id, self.config.default_expiration);
        
        // Store the session
        {
            let mut sessions = self.sessions.lock().unwrap();
            
            // Check if we're at capacity
            if sessions.len() >= self.config.max_sessions {
                self.cleanup_expired_sessions_internal(&mut sessions);
                
                // If still at capacity, remove oldest session
                if sessions.len() >= self.config.max_sessions {
                    if let Some(oldest_id) = self.find_oldest_session(&sessions) {
                        sessions.remove(&oldest_id);
                    }
                }
            }
            
            sessions.insert(session.id.clone(), session.clone());
        }
        
        Ok(session)
    }
    
    /// Get a session by ID
    pub fn get_session(&self, session_id: &str) -> Option<Session> {
        let mut sessions = self.sessions.lock().unwrap();
        
        if let Some(session) = sessions.get_mut(session_id) {
            if session.is_valid() {
                session.data.touch();
                Some(session.clone())
            } else {
                // Remove expired session
                sessions.remove(session_id);
                None
            }
        } else {
            None
        }
    }
    
    /// Update a session
    pub fn update_session(&self, session: Session) -> io::Result<()> {
        let mut sessions = self.sessions.lock().unwrap();
        
        if session.is_valid() {
            sessions.insert(session.id.clone(), session);
            Ok(())
        } else {
            // Remove expired session
            sessions.remove(&session.id);
            Err(io::Error::new(io::ErrorKind::InvalidData, "Session has expired"))
        }
    }
    
    /// Delete a session
    pub fn delete_session(&self, session_id: &str) -> bool {
        let mut sessions = self.sessions.lock().unwrap();
        sessions.remove(session_id).is_some()
    }
    
    /// Get session from cookie jar
    pub fn get_session_from_cookies(&self, cookies: &CookieJar) -> Option<Session> {
        if let Some(session_id) = cookies.get_value(&self.config.cookie_name) {
            self.get_session(session_id)
        } else {
            None
        }
    }
    
    /// Create session cookie
    pub fn create_session_cookie(&self, session_id: &str) -> Cookie {
        let mut cookie = Cookie::session(
            self.config.cookie_name.clone(),
            session_id.to_string(),
        );
        
        cookie = cookie.path(self.config.cookie_path.clone());
        
        if let Some(ref domain) = self.config.cookie_domain {
            cookie = cookie.domain(domain.clone());
        }
        
        if self.config.secure_cookies {
            cookie = cookie.secure(true);
        }
        
        if self.config.http_only_cookies {
            cookie = cookie.http_only(true);
        }
        
        // Set expiration based on default expiration
        let expires_at = SystemTime::now() + self.config.default_expiration;
        cookie = cookie.expires(expires_at);
        
        cookie
    }
    
    /// Create session deletion cookie (expires immediately)
    pub fn create_deletion_cookie(&self) -> Cookie {
        let mut cookie = Cookie::new(
            self.config.cookie_name.clone(),
            "".to_string(),
        );
        
        cookie = cookie.path(self.config.cookie_path.clone());
        
        if let Some(ref domain) = self.config.cookie_domain {
            cookie = cookie.domain(domain.clone());
        }
        
        // Set expiration to past date to delete cookie
        let past_time = SystemTime::now() - Duration::from_secs(3600);
        cookie = cookie.expires(past_time);
        
        cookie
    }
    
    /// Cleanup expired sessions
    pub fn cleanup_expired_sessions(&self) {
        let mut last_cleanup = self.last_cleanup.lock().unwrap();
        let now = SystemTime::now();
        
        // Only cleanup if enough time has passed
        if now.duration_since(*last_cleanup).unwrap_or(Duration::ZERO) >= self.config.cleanup_interval {
            let mut sessions = self.sessions.lock().unwrap();
            self.cleanup_expired_sessions_internal(&mut sessions);
            *last_cleanup = now;
        }
    }
    
    /// Internal cleanup method
    fn cleanup_expired_sessions_internal(&self, sessions: &mut HashMap<String, Session>) {
        let expired_ids: Vec<String> = sessions
            .iter()
            .filter(|(_, session)| !session.is_valid())
            .map(|(id, _)| id.clone())
            .collect();
        
        for id in expired_ids {
            sessions.remove(&id);
        }
    }
    
    /// Find the oldest session ID
    fn find_oldest_session(&self, sessions: &HashMap<String, Session>) -> Option<String> {
        sessions
            .iter()
            .min_by_key(|(_, session)| session.data.created_at())
            .map(|(id, _)| id.clone())
    }
    
    /// Get session count
    pub fn session_count(&self) -> usize {
        self.sessions.lock().unwrap().len()
    }
    
    /// Get configuration
    pub fn config(&self) -> &SessionConfig {
        &self.config
    }
    
    /// Clear all sessions
    pub fn clear_all_sessions(&self) {
        let mut sessions = self.sessions.lock().unwrap();
        sessions.clear();
    }
    
    /// Get session statistics
    pub fn get_stats(&self) -> SessionStats {
        let sessions = self.sessions.lock().unwrap();
        let total_sessions = sessions.len();
        let expired_sessions = sessions
            .values()
            .filter(|session| !session.is_valid())
            .count();
        
        SessionStats {
            total_sessions,
            active_sessions: total_sessions - expired_sessions,
            expired_sessions,
        }
    }
}

impl Clone for SessionStore {
    fn clone(&self) -> Self {
        SessionStore {
            sessions: Arc::clone(&self.sessions),
            config: self.config.clone(),
            last_cleanup: Arc::clone(&self.last_cleanup),
        }
    }
}

/// Session store statistics
#[derive(Debug, Clone)]
pub struct SessionStats {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub expired_sessions: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    
    #[test]
    fn test_session_store_creation() {
        let config = SessionConfig::default();
        let store = SessionStore::new(config);
        
        assert_eq!(store.session_count(), 0);
    }
    
    #[test]
    fn test_create_and_get_session() {
        let config = SessionConfig::default();
        let store = SessionStore::new(config);
        
        let session = store.create_session().unwrap();
        assert!(!session.id.is_empty());
        assert!(session.is_valid());
        
        let retrieved = store.get_session(&session.id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, session.id);
    }
    
    #[test]
    fn test_session_expiration() {
        let mut config = SessionConfig::default();
        config.default_expiration = Duration::from_millis(10);
        let store = SessionStore::new(config);
        
        let session = store.create_session().unwrap();
        let session_id = session.id.clone();
        
        // Session should be valid initially
        assert!(store.get_session(&session_id).is_some());
        
        // Wait for expiration
        thread::sleep(Duration::from_millis(20));
        
        // Session should be expired and removed
        assert!(store.get_session(&session_id).is_none());
    }
    
    #[test]
    fn test_session_update() {
        let config = SessionConfig::default();
        let store = SessionStore::new(config);
        
        let mut session = store.create_session().unwrap();
        session.data.set("key".to_string(), "value".to_string());
        
        store.update_session(session.clone()).unwrap();
        
        let retrieved = store.get_session(&session.id).unwrap();
        assert_eq!(retrieved.data.peek("key"), Some("value"));
    }
    
    #[test]
    fn test_session_deletion() {
        let config = SessionConfig::default();
        let store = SessionStore::new(config);
        
        let session = store.create_session().unwrap();
        let session_id = session.id.clone();
        
        assert!(store.get_session(&session_id).is_some());
        
        let deleted = store.delete_session(&session_id);
        assert!(deleted);
        
        assert!(store.get_session(&session_id).is_none());
    }
    
    #[test]
    fn test_session_cookie_creation() {
        let config = SessionConfig::default();
        let store = SessionStore::new(config);
        
        let cookie = store.create_session_cookie("test_session_id");
        assert_eq!(cookie.name, "session_id");
        assert_eq!(cookie.value, "test_session_id");
        assert!(cookie.http_only);
    }
    
    #[test]
    fn test_session_from_cookies() {
        let config = SessionConfig::default();
        let store = SessionStore::new(config);
        
        let session = store.create_session().unwrap();
        let session_id = session.id.clone();
        
        let mut cookies = CookieJar::new();
        cookies.add_cookie(Cookie::new("session_id".to_string(), session_id.clone()));
        
        let retrieved = store.get_session_from_cookies(&cookies);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, session_id);
    }
    
    #[test]
    fn test_session_cleanup() {
        let mut config = SessionConfig::default();
        config.default_expiration = Duration::from_millis(10);
        config.cleanup_interval = Duration::from_millis(1);
        let store = SessionStore::new(config);
        
        // Create sessions
        let _session1 = store.create_session().unwrap();
        let _session2 = store.create_session().unwrap();
        
        assert_eq!(store.session_count(), 2);
        
        // Wait for expiration
        thread::sleep(Duration::from_millis(20));
        
        // Trigger cleanup
        store.cleanup_expired_sessions();
        
        // Sessions should be cleaned up
        assert_eq!(store.session_count(), 0);
    }
    
    #[test]
    fn test_session_stats() {
        let config = SessionConfig::default();
        let store = SessionStore::new(config);
        
        let _session1 = store.create_session().unwrap();
        let _session2 = store.create_session().unwrap();
        
        let stats = store.get_stats();
        assert_eq!(stats.total_sessions, 2);
        assert_eq!(stats.active_sessions, 2);
        assert_eq!(stats.expired_sessions, 0);
    }
    
    #[test]
    fn test_max_sessions_limit() {
        let mut config = SessionConfig::default();
        config.max_sessions = 2;
        let store = SessionStore::new(config);
        
        let _session1 = store.create_session().unwrap();
        let _session2 = store.create_session().unwrap();
        let _session3 = store.create_session().unwrap(); // Should evict oldest
        
        assert_eq!(store.session_count(), 2);
    }
}
