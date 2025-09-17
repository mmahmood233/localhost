mod server;
mod net;
mod http;
mod config;
mod errors;
mod mime;
mod fs;

use std::process;
use server::Server;

fn main() {
    println!("Starting Localhost HTTP Server...");
    
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
