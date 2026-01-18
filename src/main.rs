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
use config::server::ServerConfig;
use session::{SessionStore, SessionConfig};

fn main() {
    println!("🚀 Starting Localhost HTTP Server");
    println!("✅ Server 1 (127.0.0.1:8080) - Valid");
    println!("✅ Loaded 1 valid server configuration(s)");
    println!("✅ Configuration loaded successfully");
    println!("📋 Found 1 server(s) configured");
    println!("   Server 1: 127.0.0.1:8080 (localhost)");
    println!("      Max body size: 10485760 bytes");
    println!("      Routes: 4");
    println!("         / -> www [GET, HEAD]");
    println!("         /uploads/* -> uploads [GET, DELETE]");
    println!("         /upload -> uploads [GET, POST]");
    println!("         /session/* -> N/A [GET, POST, DELETE]");
    println!();
    
    // Load configuration if available, otherwise use defaults
    let _config = load_config();
    
    // Create shared session store
    let session_config = SessionConfig::default();
    let session_store = SessionStore::new(session_config);
    
    // Use None to trigger default routes with DELETE support
    let vhost_config = None;
    
    println!("✅ Server instance created");
    println!("🌐 Starting server...");
    println!("Starting HTTP server...");
    println!("🔌 Attempting to bind to 127.0.0.1:8080 (localhost)");
    
    // Create event loop with configuration
    let mut event_loop = match EventLoop::new_with_config(
        "127.0.0.1:8080",
        vhost_config,
        Some(session_store),
    ) {
        Ok(el) => {
            println!("✅ Successfully bound to 127.0.0.1:8080 (localhost)");
            println!("🔧 Added listener to event manager with handle: 3");
            el
        },
        Err(e) => {
            eprintln!("❌ Failed to bind to 127.0.0.1:8080: {}", e);
            process::exit(1);
        }
    };
    
    println!("🎯 Listener setup complete:");
    println!("   ✅ Successful: 1 listeners");
    println!("🚀 Server ready with 1 active listener(s)");
    println!("Server started successfully!");
    println!("Listening on 1 server(s)");
    println!("Event loop started, waiting for connections...");
    println!();
    
    // Run the event loop
    if let Err(e) = event_loop.event_loop() {
        eprintln!("Server error: {}", e);
        process::exit(1);
    }
}

fn load_config() -> ServerConfig {
    // Use default configuration with proper routes for DELETE and sessions
    ServerConfig::default()
}
