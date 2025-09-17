use crate::net::event_loop::EventLoop;
use std::io;

pub struct Server {
    event_loop: EventLoop,
}

impl Server {
    pub fn new() -> io::Result<Self> {
        let event_loop = EventLoop::new("127.0.0.1:8080")?;
        
        Ok(Server {
            event_loop,
        })
    }
    
    pub fn run(&mut self) -> io::Result<()> {
        println!("Server listening on 127.0.0.1:8080");
        self.event_loop.event_loop()
    }
}
