use crate::net::multi_server::{MultiServer, ServerStats};
use crate::config::server::ServerConfig;
use std::io;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

/// Main HTTP server with multi-listener and virtual host support
pub struct Server {
    /// Multi-listener server instance
    multi_server: MultiServer,
    /// Server configuration
    config: ServerConfig,
    /// Running state
    running: Arc<AtomicBool>,
}

impl Server {
    /// Create new server from configuration file
    pub fn from_config_file<P: AsRef<Path>>(_config_path: P) -> io::Result<Self> {
        // For now, use default config since ConfigLoader is not implemented
        let config = ServerConfig::default();
        Self::from_config(config)
    }
    
    /// Create new server from configuration
    pub fn from_config(config: ServerConfig) -> io::Result<Self> {
        let multi_server = MultiServer::new(config.clone())?;
        let running = Arc::new(AtomicBool::new(false));
        
        Ok(Server {
            multi_server,
            config,
            running,
        })
    }
    
    /// Create server with default configuration
    pub fn new() -> io::Result<Self> {
        let config = ServerConfig::default();
        Self::from_config(config)
    }
    
    /// Start the server
    pub fn start(&mut self) -> io::Result<()> {
        if self.running.load(Ordering::Relaxed) {
            return Err(io::Error::new(io::ErrorKind::AlreadyExists, "Server already running"));
        }
        
        // Start all listeners
        self.multi_server.start()?;
        self.running.store(true, Ordering::Relaxed);
        
        // Setup signal handlers for graceful shutdown
        self.setup_signal_handlers()?;
        
        println!("HTTP Server started successfully");
        println!("Listeners: {}", self.multi_server.listener_count());
        println!("Virtual Hosts: {}", self.multi_server.virtual_host_count());
        
        if let Some(default_host) = self.multi_server.default_host() {
            println!("Default Host: {}", default_host);
        }
        
        Ok(())
    }
    
    /// Run the server event loop
    pub fn run(&mut self) -> io::Result<()> {
        if !self.running.load(Ordering::Relaxed) {
            self.start()?;
        }
        
        println!("Server running... Press Ctrl+C to stop");
        
        // Start statistics reporting thread
        let stats_running = self.running.clone();
        let stats_thread = thread::spawn(move || {
            while stats_running.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_secs(60)); // Report every minute
                // Stats reporting would be implemented here
            }
        });
        
        // Run main event loop
        let result = self.multi_server.run();
        
        // Cleanup
        self.running.store(false, Ordering::Relaxed);
        let _ = stats_thread.join();
        
        result
    }
    
    /// Stop the server gracefully
    pub fn stop(&mut self) -> io::Result<()> {
        if !self.running.load(Ordering::Relaxed) {
            return Ok(());
        }
        
        println!("Stopping server...");
        self.running.store(false, Ordering::Relaxed);
        
        // Stop multi-server
        self.multi_server.stop()?;
        
        // Print final statistics
        self.print_final_stats();
        
        println!("Server stopped successfully");
        Ok(())
    }
    
    /// Reload server configuration
    pub fn reload_config<P: AsRef<Path>>(&mut self, _config_path: P) -> io::Result<()> {
        println!("Reloading configuration...");
        
        // For now, use default config since ConfigLoader is not implemented
        let new_config = ServerConfig::default();
        self.config = new_config.clone();
        
        // Reload multi-server configuration
        self.multi_server.reload_config(new_config)?;
        
        println!("Configuration reloaded successfully");
        Ok(())
    }
    
    /// Get server statistics
    pub fn stats(&self) -> &ServerStats {
        self.multi_server.stats()
    }
    
    /// Print server status
    pub fn print_status(&self) {
        let stats = self.stats();
        
        println!("=== Server Status ===");
        println!("Running: {}", self.running.load(Ordering::Relaxed));
        println!("Listeners: {}", self.multi_server.listener_count());
        println!("Virtual Hosts: {}", self.multi_server.virtual_host_count());
        println!("Active Connections: {}", stats.active_connections);
        println!("Total Connections: {}", stats.connections_accepted);
        println!("Requests Processed: {}", stats.requests_processed);
        println!("Bytes Sent: {}", format_bytes(stats.bytes_sent));
        println!("Bytes Received: {}", format_bytes(stats.bytes_received));
        println!("Errors: {}", stats.errors);
        println!("Uptime: {} seconds", stats.uptime_seconds);
        
        // Print listener details
        println!("\n=== Listeners ===");
        for (_, listener) in self.multi_server.listeners() {
            println!("  {}:{} (default: {})", 
                listener.config.address, 
                listener.config.port,
                listener.is_default);
        }
        
        // Print virtual host details
        println!("\n=== Virtual Hosts ===");
        for (name, vhost) in self.multi_server.virtual_hosts() {
            println!("  {} -> {}", name, vhost.document_root.display());
        }
    }
    
    /// Setup signal handlers for graceful shutdown
    fn setup_signal_handlers(&self) -> io::Result<()> {
        // This would integrate with signal handling
        // For now, we'll just return Ok
        Ok(())
    }
    
    /// Print final statistics on shutdown
    fn print_final_stats(&self) {
        let stats = self.stats();
        
        println!("\n=== Final Statistics ===");
        println!("Total Connections Accepted: {}", stats.connections_accepted);
        println!("Total Requests Processed: {}", stats.requests_processed);
        println!("Total Bytes Sent: {}", format_bytes(stats.bytes_sent));
        println!("Total Bytes Received: {}", format_bytes(stats.bytes_received));
        println!("Total Errors: {}", stats.errors);
        println!("Total Uptime: {} seconds", stats.uptime_seconds);
        
        if stats.requests_processed > 0 {
            let avg_response_size = stats.bytes_sent / stats.requests_processed;
            println!("Average Response Size: {}", format_bytes(avg_response_size));
        }
    }
    
    /// Check if server is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
    
    /// Get server configuration
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }
    
    /// Get multi-server instance
    pub fn multi_server(&self) -> &MultiServer {
        &self.multi_server
    }
    
    /// Get multi-server instance (mutable)
    pub fn multi_server_mut(&mut self) -> &mut MultiServer {
        &mut self.multi_server
    }
}

impl Default for Server {
    fn default() -> Self {
        Self::new().expect("Failed to create default server")
    }
}

/// Format bytes in human-readable format
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    
    if bytes == 0 {
        return "0 B".to_string();
    }
    
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

/// Server builder for easy configuration
pub struct ServerBuilder {
    config: ServerConfig,
}

impl ServerBuilder {
    /// Create new server builder
    pub fn new() -> Self {
        ServerBuilder {
            config: ServerConfig::default(),
        }
    }
    
    /// Set server configuration
    pub fn with_config(mut self, config: ServerConfig) -> Self {
        self.config = config;
        self
    }
    
    /// Load configuration from file
    pub fn with_config_file<P: AsRef<Path>>(mut self, _config_path: P) -> io::Result<Self> {
        // For now, use default config since ConfigLoader is not implemented
        self.config = ServerConfig::default();
        Ok(self)
    }
    
    /// Build the server
    pub fn build(self) -> io::Result<Server> {
        Server::from_config(self.config)
    }
}

impl Default for ServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]  
mod tests {
    use super::*;
    
    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }
    
    #[test]
    fn test_server_builder() {
        let builder = ServerBuilder::new();
        assert!(builder.build().is_ok());
    }
}
