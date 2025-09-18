use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::os::unix::io::RawFd;

#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    /// Timeout for reading request headers
    pub read_header_timeout: Duration,
    /// Timeout for reading request body
    pub read_body_timeout: Duration,
    /// Timeout for writing response data
    pub write_timeout: Duration,
    /// Timeout for keep-alive idle connections
    pub keep_alive_timeout: Duration,
    /// Maximum time for a complete request-response cycle
    pub request_timeout: Duration,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        TimeoutConfig {
            read_header_timeout: Duration::from_secs(5),
            read_body_timeout: Duration::from_secs(15),
            write_timeout: Duration::from_secs(5),
            keep_alive_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(30),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ConnectionState {
    ReadingHeaders,
    ReadingBody,
    Writing,
    KeepAlive,
}

#[derive(Debug)]
pub struct ConnectionTimeout {
    pub fd: RawFd,
    pub state: ConnectionState,
    pub last_activity: Instant,
    pub request_start: Instant,
}

impl ConnectionTimeout {
    pub fn new(fd: RawFd) -> Self {
        let now = Instant::now();
        ConnectionTimeout {
            fd,
            state: ConnectionState::ReadingHeaders,
            last_activity: now,
            request_start: now,
        }
    }
    
    pub fn update_activity(&mut self) {
        self.last_activity = Instant::now();
    }
    
    pub fn set_state(&mut self, state: ConnectionState) {
        self.state = state;
        self.update_activity();
    }
    
    pub fn reset_for_new_request(&mut self) {
        let now = Instant::now();
        self.state = ConnectionState::ReadingHeaders;
        self.last_activity = now;
        self.request_start = now;
    }
}

pub struct TimeoutManager {
    config: TimeoutConfig,
    connections: HashMap<RawFd, ConnectionTimeout>,
}

impl TimeoutManager {
    pub fn new(config: TimeoutConfig) -> Self {
        TimeoutManager {
            config,
            connections: HashMap::new(),
        }
    }
    
    pub fn add_connection(&mut self, fd: RawFd) {
        self.connections.insert(fd, ConnectionTimeout::new(fd));
    }
    
    pub fn remove_connection(&mut self, fd: RawFd) {
        self.connections.remove(&fd);
    }
    
    pub fn update_activity(&mut self, fd: RawFd) {
        if let Some(conn) = self.connections.get_mut(&fd) {
            conn.update_activity();
        }
    }
    
    pub fn set_connection_state(&mut self, fd: RawFd, state: ConnectionState) {
        if let Some(conn) = self.connections.get_mut(&fd) {
            conn.set_state(state);
        }
    }
    
    pub fn reset_connection_for_new_request(&mut self, fd: RawFd) {
        if let Some(conn) = self.connections.get_mut(&fd) {
            conn.reset_for_new_request();
        }
    }
    
    /// Check for timed-out connections and return their file descriptors
    pub fn check_timeouts(&self) -> Vec<RawFd> {
        let now = Instant::now();
        let mut timed_out = Vec::new();
        
        for (fd, conn) in &self.connections {
            let timeout_duration = match conn.state {
                ConnectionState::ReadingHeaders => self.config.read_header_timeout,
                ConnectionState::ReadingBody => self.config.read_body_timeout,
                ConnectionState::Writing => self.config.write_timeout,
                ConnectionState::KeepAlive => self.config.keep_alive_timeout,
            };
            
            // Check activity timeout
            if now.duration_since(conn.last_activity) > timeout_duration {
                timed_out.push(*fd);
                continue;
            }
            
            // Check overall request timeout
            if now.duration_since(conn.request_start) > self.config.request_timeout {
                timed_out.push(*fd);
                continue;
            }
        }
        
        timed_out
    }
    
    /// Get the next timeout check interval (when we should check timeouts again)
    pub fn next_timeout_check(&self) -> Duration {
        let now = Instant::now();
        let mut min_remaining = Duration::from_secs(60); // Default to 1 minute
        
        for conn in self.connections.values() {
            let timeout_duration = match conn.state {
                ConnectionState::ReadingHeaders => self.config.read_header_timeout,
                ConnectionState::ReadingBody => self.config.read_body_timeout,
                ConnectionState::Writing => self.config.write_timeout,
                ConnectionState::KeepAlive => self.config.keep_alive_timeout,
            };
            
            let elapsed = now.duration_since(conn.last_activity);
            if elapsed < timeout_duration {
                let remaining = timeout_duration - elapsed;
                if remaining < min_remaining {
                    min_remaining = remaining;
                }
            }
            
            // Also check request timeout
            let request_elapsed = now.duration_since(conn.request_start);
            if request_elapsed < self.config.request_timeout {
                let request_remaining = self.config.request_timeout - request_elapsed;
                if request_remaining < min_remaining {
                    min_remaining = request_remaining;
                }
            }
        }
        
        // Don't check too frequently, minimum 100ms
        if min_remaining < Duration::from_millis(100) {
            Duration::from_millis(100)
        } else {
            min_remaining
        }
    }
    
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }
    
    pub fn config(&self) -> &TimeoutConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    
    #[test]
    fn test_timeout_manager_basic() {
        let config = TimeoutConfig::default();
        let mut manager = TimeoutManager::new(config);
        
        // Add a connection
        manager.add_connection(1);
        assert_eq!(manager.connection_count(), 1);
        
        // Remove the connection
        manager.remove_connection(1);
        assert_eq!(manager.connection_count(), 0);
    }
    
    #[test]
    fn test_timeout_detection() {
        let mut config = TimeoutConfig::default();
        config.read_header_timeout = Duration::from_millis(10);
        
        let mut manager = TimeoutManager::new(config);
        manager.add_connection(1);
        
        // Should not timeout immediately
        let timed_out = manager.check_timeouts();
        assert!(timed_out.is_empty());
        
        // Wait for timeout
        thread::sleep(Duration::from_millis(15));
        
        let timed_out = manager.check_timeouts();
        assert_eq!(timed_out, vec![1]);
    }
    
    #[test]
    fn test_activity_update() {
        let mut config = TimeoutConfig::default();
        config.read_header_timeout = Duration::from_millis(20);
        
        let mut manager = TimeoutManager::new(config);
        manager.add_connection(1);
        
        // Wait a bit, then update activity
        thread::sleep(Duration::from_millis(10));
        manager.update_activity(1);
        
        // Wait a bit more, should not timeout due to activity update
        thread::sleep(Duration::from_millis(15));
        let timed_out = manager.check_timeouts();
        assert!(timed_out.is_empty());
        
        // Wait for actual timeout
        thread::sleep(Duration::from_millis(25));
        let timed_out = manager.check_timeouts();
        assert_eq!(timed_out, vec![1]);
    }
    
    #[test]
    fn test_state_transitions() {
        let config = TimeoutConfig::default();
        let mut manager = TimeoutManager::new(config);
        manager.add_connection(1);
        
        // Test state transitions
        manager.set_connection_state(1, ConnectionState::ReadingBody);
        manager.set_connection_state(1, ConnectionState::Writing);
        manager.set_connection_state(1, ConnectionState::KeepAlive);
        
        // Should still be tracked
        assert_eq!(manager.connection_count(), 1);
    }
}
