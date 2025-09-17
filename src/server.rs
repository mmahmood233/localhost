use crate::net::epoll::EpollServer;
use std::io;

pub struct Server {
    epoll_server: EpollServer,
}

impl Server {
    pub fn new() -> io::Result<Self> {
        let epoll_server = EpollServer::new("127.0.0.1:8080")?;
        
        Ok(Server {
            epoll_server,
        })
    }
    
    pub fn run(&mut self) -> io::Result<()> {
        println!("Server listening on 127.0.0.1:8080");
        self.epoll_server.event_loop()
    }
}
