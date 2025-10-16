mod server;
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
use server::Server;

fn main() {
    println!("Starting Localhost HTTP Server...");
    println!("Server will listen on http://127.0.0.1:8080");
    println!("Press Ctrl+C to stop the server");
    
    // For now, let's use the original simple server that actually works
    let mut server = match Server::new() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to create server: {}", e);
            process::exit(1);
        }
    };
    
    if let Err(e) = server.run() {
        eprintln!("Server error: {}", e);
        process::exit(1);
    }
}
