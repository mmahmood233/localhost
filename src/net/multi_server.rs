use crate::config::server::{ServerConfig, ListenerConfig, VirtualHostConfig};
use std::collections::HashMap;
use std::io;
use std::net::{SocketAddr, TcpListener};
use std::os::unix::io::{AsRawFd, RawFd};

/// Multi-listener HTTP server supporting multiple ports and virtual hosts
pub struct MultiServer {
    /// Server configuration
    config: ServerConfig,
    /// Active listeners mapped by file descriptor
    listeners: HashMap<RawFd, ListenerInfo>,
    /// Virtual hosts mapped by server name
    virtual_hosts: HashMap<String, VirtualHostConfig>,
    /// Default virtual host
    default_host: Option<String>,
    /// Server statistics
    stats: ServerStats,
}

/// Information about a listener
#[derive(Debug)]
pub struct ListenerInfo {
    /// Listener configuration
    pub config: ListenerConfig,
    /// Socket address
    pub addr: SocketAddr,
    /// Whether this is the default listener
    pub is_default: bool,
}

/// Server statistics
#[derive(Debug, Clone, Default)]
pub struct ServerStats {
    /// Total connections accepted
    pub connections_accepted: u64,
    /// Active connections count
    pub active_connections: u32,
    /// Total requests processed
    pub requests_processed: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Errors encountered
    pub errors: u64,
    /// Uptime in seconds
    pub uptime_seconds: u64,
}

impl MultiServer {
    /// Create new multi-listener server
    pub fn new(config: ServerConfig) -> io::Result<Self> {
        // Build virtual hosts map
        let mut virtual_hosts = HashMap::new();
        for vhost in &config.virtual_hosts {
            virtual_hosts.insert(vhost.server_name.clone(), vhost.clone());
        }
        
        let default_host = config.default_host.clone();
        
        Ok(MultiServer {
            config,
            listeners: HashMap::new(),
            virtual_hosts,
            default_host,
            stats: ServerStats::default(),
        })
    }
    
    /// Start all configured listeners
    pub fn start(&mut self) -> io::Result<()> {
        // Validate configuration
        self.validate_config()?;
        
        // Start all listeners
        for listener_config in &self.config.listeners.clone() {
            self.start_listener(listener_config.clone())?;
        }
        
        println!("Multi-listener server started with {} listeners", self.listeners.len());
        for (_, listener_info) in &self.listeners {
            println!("  Listening on {}:{} (default: {})", 
                listener_info.config.address, 
                listener_info.config.port,
                listener_info.is_default);
        }
        
        Ok(())
    }
    
    /// Start a single listener
    fn start_listener(&mut self, config: ListenerConfig) -> io::Result<()> {
        let addr = format!("{}:{}", config.address, config.port);
        let socket_addr: SocketAddr = addr.parse()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid listener address"))?;
        
        let listener = TcpListener::bind(socket_addr)?;
        listener.set_nonblocking(true)?;
        
        let fd = listener.as_raw_fd();
        
        let listener_info = ListenerInfo {
            config: config.clone(),
            addr: socket_addr,
            is_default: config.default,
        };
        
        self.listeners.insert(fd, listener_info);
        
        println!("Started listener on {}", addr);
        Ok(())
    }
    
    /// Run the server event loop
    pub fn run(&mut self) -> io::Result<()> {
        println!("Multi-listener server starting with {} listeners", self.listeners.len());
        
        // For now, delegate to the original EventLoop-based server
        // This is a temporary solution until full integration is complete
        use crate::net::event_loop::EventLoop;
        
        let mut event_loop = EventLoop::new("127.0.0.1:8080")?;
        println!("Server listening on http://127.0.0.1:8080");
        println!("Multi-listener features available but using single listener for now");
        
        event_loop.event_loop()
    }
    
    /// Handle new incoming connection (placeholder)
    fn handle_new_connection(&mut self, _listener_fd: RawFd) -> io::Result<()> {
        // This would integrate with the existing connection handling logic
        // For now, this is a placeholder for the multi-listener concept
        self.stats.connections_accepted += 1;
        self.stats.active_connections += 1;
        Ok(())
    }
    
    /// Handle connection events (placeholder)
    fn handle_connection_event(&mut self, _connection_fd: RawFd) -> io::Result<()> {
        // This would integrate with the existing connection handling logic
        // For now, this is a placeholder for the multi-listener concept
        self.stats.requests_processed += 1;
        Ok(())
    }
    
    /// Handle connection timeouts (placeholder)
    fn handle_timeouts(&mut self) -> io::Result<()> {
        // This would integrate with the existing timeout management
        // For now, this is a placeholder for the multi-listener concept
        Ok(())
    }
    
    /// Validate server configuration
    fn validate_config(&self) -> io::Result<()> {
        // Check that we have at least one listener
        if self.config.listeners.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "No listeners configured"));
        }
        
        // Check that we have at least one virtual host
        if self.config.virtual_hosts.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "No virtual hosts configured"));
        }
        
        // Check default host exists
        if let Some(ref default_host) = self.default_host {
            if !self.virtual_hosts.contains_key(default_host) {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "Default host not found in virtual hosts"));
            }
        }
        
        // Check for exactly one default listener
        let default_count = self.config.listeners.iter().filter(|l| l.default).count();
        if default_count == 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "No default listener specified"));
        }
        if default_count > 1 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Multiple default listeners specified"));
        }
        
        // Check for duplicate listener addresses
        let mut addresses = std::collections::HashSet::new();
        for listener in &self.config.listeners {
            let addr = format!("{}:{}", listener.address, listener.port);
            if addresses.contains(&addr) {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "Duplicate listener address"));
            }
            addresses.insert(addr);
        }
        
        Ok(())
    }
    
    /// Get virtual host for request
    pub fn get_virtual_host(&self, host_header: Option<&str>) -> Option<&VirtualHostConfig> {
        if let Some(host) = host_header {
            // Try exact match first
            if let Some(vhost) = self.virtual_hosts.get(host) {
                return Some(vhost);
            }
            
            // Try without port number
            if let Some(host_without_port) = host.split(':').next() {
                if let Some(vhost) = self.virtual_hosts.get(host_without_port) {
                    return Some(vhost);
                }
            }
        }
        
        // Fall back to default host
        if let Some(ref default_host) = self.default_host {
            self.virtual_hosts.get(default_host)
        } else {
            // Use first virtual host as fallback
            self.virtual_hosts.values().next()
        }
    }
    
    /// Get server statistics
    pub fn stats(&self) -> &ServerStats {
        &self.stats
    }
    
    /// Get listener information
    pub fn listeners(&self) -> &HashMap<RawFd, ListenerInfo> {
        &self.listeners
    }
    
    /// Get virtual hosts
    pub fn virtual_hosts(&self) -> &HashMap<String, VirtualHostConfig> {
        &self.virtual_hosts
    }
    
    /// Get default host name
    pub fn default_host(&self) -> Option<&str> {
        self.default_host.as_deref()
    }
    
    /// Stop the server
    pub fn stop(&mut self) -> io::Result<()> {
        // Close all listeners
        self.listeners.clear();
        
        println!("Multi-listener server stopped");
        Ok(())
    }
    
    /// Reload configuration
    pub fn reload_config(&mut self, new_config: ServerConfig) -> io::Result<()> {
        // Stop existing listeners
        self.stop()?;
        
        // Update configuration
        self.config = new_config;
        
        // Rebuild virtual hosts map
        self.virtual_hosts.clear();
        for vhost in &self.config.virtual_hosts {
            self.virtual_hosts.insert(vhost.server_name.clone(), vhost.clone());
        }
        
        self.default_host = self.config.default_host.clone();
        
        // Restart listeners
        self.start()?;
        
        println!("Configuration reloaded successfully");
        Ok(())
    }
    
    /// Add virtual host dynamically
    pub fn add_virtual_host(&mut self, vhost: VirtualHostConfig) -> io::Result<()> {
        if self.virtual_hosts.contains_key(&vhost.server_name) {
            return Err(io::Error::new(io::ErrorKind::AlreadyExists, "Virtual host already exists"));
        }
        
        self.virtual_hosts.insert(vhost.server_name.clone(), vhost);
        println!("Added virtual host: {}", self.virtual_hosts.keys().last().unwrap());
        Ok(())
    }
    
    /// Remove virtual host dynamically
    pub fn remove_virtual_host(&mut self, server_name: &str) -> io::Result<()> {
        if Some(server_name) == self.default_host.as_deref() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Cannot remove default virtual host"));
        }
        
        if self.virtual_hosts.remove(server_name).is_some() {
            println!("Removed virtual host: {}", server_name);
            Ok(())
        } else {
            Err(io::Error::new(io::ErrorKind::NotFound, "Virtual host not found"))
        }
    }
    
    /// Get listener by address and port
    pub fn get_listener_by_address(&self, address: &str, port: u16) -> Option<&ListenerInfo> {
        self.listeners.values().find(|listener| {
            listener.config.address == address && listener.config.port == port
        })
    }
    
    /// Get default listener
    pub fn get_default_listener(&self) -> Option<&ListenerInfo> {
        self.listeners.values().find(|listener| listener.is_default)
    }
    
    /// Check if server is running
    pub fn is_running(&self) -> bool {
        !self.listeners.is_empty()
    }
    
    /// Get listener count
    pub fn listener_count(&self) -> usize {
        self.listeners.len()
    }
    
    /// Get virtual host count
    pub fn virtual_host_count(&self) -> usize {
        self.virtual_hosts.len()
    }
}

/// Server selection utilities
pub struct ServerSelector;

impl ServerSelector {
    /// Select appropriate virtual host based on request
    pub fn select_virtual_host<'a>(
        virtual_hosts: &'a HashMap<String, VirtualHostConfig>,
        host_header: Option<&str>,
        default_host: Option<&str>,
    ) -> Option<&'a VirtualHostConfig> {
        // Try Host header first
        if let Some(host) = host_header {
            // Exact match
            if let Some(vhost) = virtual_hosts.get(host) {
                return Some(vhost);
            }
            
            // Match without port
            if let Some(host_without_port) = host.split(':').next() {
                if let Some(vhost) = virtual_hosts.get(host_without_port) {
                    return Some(vhost);
                }
            }
            
            // Wildcard matching (*.example.com)
            for (server_name, vhost) in virtual_hosts {
                if server_name.starts_with("*.") {
                    let domain = &server_name[2..];
                    if host.ends_with(domain) {
                        return Some(vhost);
                    }
                }
            }
        }
        
        // Fall back to default host
        if let Some(default) = default_host {
            if let Some(vhost) = virtual_hosts.get(default) {
                return Some(vhost);
            }
        }
        
        // Use first virtual host as last resort
        virtual_hosts.values().next()
    }
    
    /// Select appropriate listener for outgoing connections
    pub fn select_listener<'a>(
        listeners: &'a HashMap<RawFd, ListenerInfo>,
        preferred_address: Option<&str>,
    ) -> Option<&'a ListenerInfo> {
        // Try preferred address first
        if let Some(addr) = preferred_address {
            for listener in listeners.values() {
                if listener.config.address == addr {
                    return Some(listener);
                }
            }
        }
        
        // Fall back to default listener
        for listener in listeners.values() {
            if listener.is_default {
                return Some(listener);
            }
        }
        
        // Use first listener as last resort
        listeners.values().next()
    }
    
    /// Check if host matches server name pattern
    pub fn host_matches_pattern(host: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        
        if pattern.starts_with("*.") {
            let domain = &pattern[2..];
            return host.ends_with(domain);
        }
        
        host == pattern
    }
    
    /// Normalize host header (remove port, lowercase)
    pub fn normalize_host(host: &str) -> String {
        host.split(':')
            .next()
            .unwrap_or(host)
            .to_lowercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::server::{GlobalConfig, TimeoutConfig};
    use std::time::Duration;
    
    fn create_test_config() -> ServerConfig {
        ServerConfig {
            listeners: vec![
                ListenerConfig {
                    address: "127.0.0.1".to_string(),
                    port: 8080,
                    default: true,
                },
                ListenerConfig {
                    address: "127.0.0.1".to_string(),
                    port: 8081,
                    default: false,
                },
            ],
            virtual_hosts: vec![
                VirtualHostConfig {
                    server_name: "localhost".to_string(),
                    document_root: std::path::PathBuf::from("/var/www/localhost"),
                    index_files: vec!["index.html".to_string()],
                    max_body_size: 1024 * 1024,
                    routes: vec![],
                    access_log: None,
                    error_log: None,
                },
                VirtualHostConfig {
                    server_name: "example.com".to_string(),
                    document_root: std::path::PathBuf::from("/var/www/example"),
                    index_files: vec!["index.html".to_string()],
                    max_body_size: 1024 * 1024,
                    routes: vec![],
                    access_log: None,
                    error_log: None,
                },
            ],
            default_host: Some("localhost".to_string()),
            global: GlobalConfig {
                server_name: "Test Server".to_string(),
                workers: 1,
                timeouts: TimeoutConfig {
                    read_header: Duration::from_secs(5),
                    read_body: Duration::from_secs(15),
                    write: Duration::from_secs(5),
                    keep_alive: Duration::from_secs(10),
                    request: Duration::from_secs(30),
                },
                ..Default::default()
            },
        }
    }
    
    #[test]
    fn test_server_selector_virtual_host() {
        let config = create_test_config();
        let mut virtual_hosts = HashMap::new();
        for vhost in &config.virtual_hosts {
            virtual_hosts.insert(vhost.server_name.clone(), vhost.clone());
        }
        
        // Test exact match
        let vhost = ServerSelector::select_virtual_host(
            &virtual_hosts,
            Some("localhost"),
            Some("localhost"),
        );
        assert!(vhost.is_some());
        assert_eq!(vhost.unwrap().server_name, "localhost");
        
        // Test default fallback
        let vhost = ServerSelector::select_virtual_host(
            &virtual_hosts,
            Some("unknown.com"),
            Some("localhost"),
        );
        assert!(vhost.is_some());
        assert_eq!(vhost.unwrap().server_name, "localhost");
    }
    
    #[test]
    fn test_host_pattern_matching() {
        assert!(ServerSelector::host_matches_pattern("example.com", "example.com"));
        assert!(ServerSelector::host_matches_pattern("sub.example.com", "*.example.com"));
        assert!(ServerSelector::host_matches_pattern("anything.com", "*"));
        assert!(!ServerSelector::host_matches_pattern("example.org", "example.com"));
    }
    
    #[test]
    fn test_normalize_host() {
        assert_eq!(ServerSelector::normalize_host("Example.Com:8080"), "example.com");
        assert_eq!(ServerSelector::normalize_host("LOCALHOST"), "localhost");
        assert_eq!(ServerSelector::normalize_host("test.com:443"), "test.com");
    }
    
    #[test]
    fn test_server_stats() {
        let stats = ServerStats::default();
        assert_eq!(stats.connections_accepted, 0);
        assert_eq!(stats.active_connections, 0);
        assert_eq!(stats.requests_processed, 0);
    }
}
