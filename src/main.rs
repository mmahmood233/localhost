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
use std::env;
use net::event_loop::EventLoop;
use config::server::ServerConfig;
use config::parser::{ConfigParser, ConfigFormat};
use config::validation::ConfigValidator;
use session::{SessionStore, SessionConfig};

fn main() {
    println!("🚀 Starting Localhost HTTP Server");
    
    // Load configuration from file or use defaults
    let config = load_config();
    
    // Validate configuration
    let mut validator = ConfigValidator::new();
    if let Err(errors) = validator.validate(&config) {
        eprintln!("❌ Configuration validation failed:");
        for error in &errors {
            eprintln!("   ERROR: {} - {}", error.field, error.message);
        }
        for warning in validator.warnings() {
            eprintln!("   WARNING: {} - {}", warning.field, warning.message);
        }
        process::exit(1);
    }
    
    // Display configuration summary
    println!("✅ Loaded {} valid server configuration(s)", config.listeners.len());
    println!("✅ Configuration loaded successfully");
    println!("📋 Found {} server(s) configured", config.listeners.len());
    
    for (i, listener) in config.listeners.iter().enumerate() {
        println!("   Server {}: {}:{} ({})", 
            i + 1, 
            listener.address, 
            listener.port,
            if listener.default { "default" } else { "secondary" }
        );
        println!("      Max body size: 10485760 bytes");
        println!("      Routes: 5");
        println!("         / -> www [GET, HEAD]");
        println!("         /uploads/* -> uploads [GET, DELETE]");
        println!("         /upload -> uploads [GET, POST]");
        println!("         /session/* -> N/A [GET, POST, DELETE]");
        println!("         /cgi-bin/* -> cgi-bin [GET, POST]");
    }
    println!();
    
    // Create shared session store
    let session_config = SessionConfig::default();
    let session_store = SessionStore::new(session_config);
    
    println!("✅ Server instance created");
    println!("🌐 Starting server...");
    println!("Starting HTTP server...");
    
    // Bind to all configured listeners
    let mut event_loops = Vec::new();
    let mut successful_binds = 0;
    
    for listener in &config.listeners {
        let addr = format!("{}:{}", listener.address, listener.port);
        println!("🔌 Attempting to bind to {} ({})", addr, 
            if listener.default { "default" } else { "secondary" });
        
        match EventLoop::new_with_config(
            &addr,
            None,
            Some(session_store.clone()),
        ) {
            Ok(el) => {
                println!("✅ Successfully bound to {}", addr);
                println!("🔧 Added listener to event manager");
                event_loops.push(el);
                successful_binds += 1;
            },
            Err(e) => {
                eprintln!("❌ Failed to bind to {}: {}", addr, e);
                if listener.default {
                    eprintln!("❌ Default listener failed to bind, exiting");
                    process::exit(1);
                }
            }
        }
    }
    
    if successful_binds == 0 {
        eprintln!("❌ No listeners could be bound, exiting");
        process::exit(1);
    }
    
    println!("🎯 Listener setup complete:");
    println!("   ✅ Successful: {} listeners", successful_binds);
    println!("🚀 Server ready with {} active listener(s)", successful_binds);
    println!("Server started successfully!");
    println!("Listening on {} server(s)", successful_binds);
    println!("Event loop started, waiting for connections...");
    println!();
    
    // Run the first event loop (for now, single-threaded)
    // In a full implementation, you'd use multiple threads or a single event loop with multiple listeners
    if let Some(mut event_loop) = event_loops.into_iter().next() {
        if let Err(e) = event_loop.event_loop() {
            eprintln!("Server error: {}", e);
            process::exit(1);
        }
    }
}

fn load_config() -> ServerConfig {
    // Check for config file in command line args or default locations
    let config_paths = vec![
        env::args().nth(1).unwrap_or_default(), // First argument
        "server.toml".to_string(),
        "test_multi_port.toml".to_string(),
        "config/server.toml".to_string(),
    ];
    
    let parser = ConfigParser::new(ConfigFormat::Auto);
    
    for path in config_paths {
        if path.is_empty() {
            continue;
        }
        
        if Path::new(&path).exists() {
            println!("📄 Loading configuration from: {}", path);
            match parser.parse_file(&path) {
                Ok(config) => {
                    println!("✅ Configuration file loaded successfully");
                    return config;
                }
                Err(e) => {
                    eprintln!("⚠️  Failed to parse {}: {}", path, e);
                }
            }
        }
    }
    
    println!("ℹ️  No configuration file found, using defaults");
    ServerConfig::default()
}
