// mod server;
mod net;
mod http;
mod config;
mod errors;
mod mime;
mod fs;
mod routing;
mod upload;
mod session;
mod cgi;

use std::process;
use std::path::Path;
use net::event_loop::EventLoop;
use config::server::{ServerConfig, VirtualHostConfig};
use session::{SessionStore, SessionConfig};

fn main() {
    println!("Starting Localhost HTTP Server...");
    println!("Server will listen on http://127.0.0.1:8080");
    println!("Press Ctrl+C to stop the server");
    println!();
    
    // Load configuration if available, otherwise use defaults
    let config = load_config();
    
    // Create shared session store
    let session_config = SessionConfig::default();
    let session_store = SessionStore::new(session_config);
    
    // Get first virtual host config or use default
    let vhost_config = config.virtual_hosts.first().cloned();
    
    if let Some(ref vhost) = vhost_config {
        println!("Virtual Host: {}", vhost.server_name);
        println!("Document Root: {}", vhost.document_root.display());
        println!("Max Body Size: {} bytes", vhost.max_body_size);
        println!("Routes configured: {}", vhost.routes.len());
        println!();
    }
    
    // Create event loop with configuration
    let mut event_loop = match EventLoop::new_with_config(
        "127.0.0.1:8080",
        vhost_config,
        Some(session_store),
    ) {
        Ok(el) => el,
        Err(e) => {
            eprintln!("Failed to create event loop: {}", e);
            process::exit(1);
        }
    };
    
    println!("Server features enabled:");
    println!("  ✓ GET/HEAD - Static file serving");
    println!("  ✓ POST - File uploads, forms, CGI");
    println!("  ✓ DELETE - File deletion");
    println!("  ✓ Sessions - Cookie-based sessions");
    println!("  ✓ CGI - Python, Perl, Shell scripts");
    println!("  ✓ Keep-alive connections");
    println!("  ✓ Timeout management");
    println!();
    println!("Server ready! Press Ctrl+C to stop.");
    println!();
    
    // Run the event loop
    if let Err(e) = event_loop.event_loop() {
        eprintln!("Server error: {}", e);
        process::exit(1);
    }
}

fn load_config() -> ServerConfig {
    let config_path = Path::new("server.toml");
    
    if config_path.exists() {
        println!("Loading configuration from server.toml...");
        let parser = config::parser::ConfigParser::new(config::parser::ConfigFormat::Toml);
        match parser.parse_file(config_path) {
            Ok(config) => {
                println!("Configuration loaded successfully!");
                return config;
            }
            Err(e) => {
                eprintln!("Warning: Failed to load config: {}", e);
                eprintln!("Using default configuration...");
            }
        }
    } else {
        println!("No server.toml found, using default configuration...");
    }
    
    ServerConfig::default()
}
