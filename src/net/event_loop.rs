use std::collections::HashMap;
use std::io::{self, ErrorKind};
use std::net::TcpListener;
use std::os::unix::io::{AsRawFd, RawFd};
use libc::{self, c_int};
use crate::net::conn::Connection;
use crate::net::timeout::{TimeoutManager, TimeoutConfig, ConnectionState};

const MAX_EVENTS: usize = 1024;
const TIMEOUT_MS: c_int = 1000;

pub struct EventLoop {
    listener: TcpListener,
    #[cfg(target_os = "macos")]
    kqueue_fd: RawFd,
    #[cfg(target_os = "linux")]
    epoll_fd: RawFd,
    connections: HashMap<RawFd, Connection>,
    timeout_manager: TimeoutManager,
}

impl EventLoop {
    pub fn new(addr: &str) -> io::Result<Self> {
        // Create and bind listener socket
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        
        #[cfg(target_os = "macos")]
        let event_fd = Self::create_kqueue(&listener)?;
        
        #[cfg(target_os = "linux")]
        let event_fd = Self::create_epoll(&listener)?;
        
        Ok(EventLoop {
            listener,
            #[cfg(target_os = "macos")]
            kqueue_fd: event_fd,
            #[cfg(target_os = "linux")]
            epoll_fd: event_fd,
            connections: HashMap::new(),
            timeout_manager: TimeoutManager::new(TimeoutConfig::default()),
        })
    }
    
    #[cfg(target_os = "macos")]
    fn create_kqueue(listener: &TcpListener) -> io::Result<RawFd> {
        // Create kqueue instance
        let kqueue_fd = unsafe { libc::kqueue() };
        if kqueue_fd == -1 {
            return Err(io::Error::last_os_error());
        }
        
        // Add listener to kqueue
        let mut kevent = libc::kevent {
            ident: listener.as_raw_fd() as libc::uintptr_t,
            filter: libc::EVFILT_READ,
            flags: libc::EV_ADD | libc::EV_ENABLE,
            fflags: 0,
            data: 0,
            udata: std::ptr::null_mut(),
        };
        
        let result = unsafe {
            libc::kevent(
                kqueue_fd,
                &mut kevent as *mut libc::kevent,
                1,
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
            )
        };
        
        if result == -1 {
            unsafe { libc::close(kqueue_fd); }
            return Err(io::Error::last_os_error());
        }
        
        Ok(kqueue_fd)
    }
    
    #[cfg(target_os = "linux")]
    fn create_epoll(listener: &TcpListener) -> io::Result<RawFd> {
        // Create epoll instance
        let epoll_fd = unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) };
        if epoll_fd == -1 {
            return Err(io::Error::last_os_error());
        }
        
        // Add listener to epoll
        let mut event = libc::epoll_event {
            events: (libc::EPOLLIN | libc::EPOLLET) as u32,
            u64: listener.as_raw_fd() as u64,
        };
        
        let result = unsafe {
            libc::epoll_ctl(
                epoll_fd,
                libc::EPOLL_CTL_ADD,
                listener.as_raw_fd(),
                &mut event as *mut libc::epoll_event,
            )
        };
        
        if result == -1 {
            unsafe { libc::close(epoll_fd); }
            return Err(io::Error::last_os_error());
        }
        
        Ok(epoll_fd)
    }
    
    pub fn event_loop(&mut self) -> io::Result<()> {
        #[cfg(target_os = "macos")]
        return self.kqueue_event_loop();
        
        #[cfg(target_os = "linux")]
        return self.epoll_event_loop();
    }
    
    #[cfg(target_os = "macos")]
    fn kqueue_event_loop(&mut self) -> io::Result<()> {
        let mut events: [libc::kevent; MAX_EVENTS] = unsafe { std::mem::zeroed() };
        
        loop {
            // Check for timed-out connections first
            self.handle_timeouts();
            
            // Calculate timeout based on next timeout check
            let timeout_duration = self.timeout_manager.next_timeout_check();
            let timeout = libc::timespec {
                tv_sec: timeout_duration.as_secs() as libc::time_t,
                tv_nsec: (timeout_duration.subsec_nanos()) as libc::c_long,
            };
            
            let nfds = unsafe {
                libc::kevent(
                    self.kqueue_fd,
                    std::ptr::null(),
                    0,
                    events.as_mut_ptr(),
                    MAX_EVENTS as c_int,
                    &timeout,
                )
            };
            
            if nfds == -1 {
                let err = io::Error::last_os_error();
                if err.kind() == ErrorKind::Interrupted {
                    continue;
                }
                return Err(err);
            }
            
            // Process events
            for i in 0..nfds as usize {
                let event = events[i];
                let fd = event.ident as RawFd;
                
                if fd == self.listener.as_raw_fd() {
                    self.accept_connections()?;
                } else {
                    self.handle_kqueue_connection_event(fd, event.filter)?;
                }
            }
        }
    }
    
    #[cfg(target_os = "linux")]
    fn epoll_event_loop(&mut self) -> io::Result<()> {
        let mut events: [libc::epoll_event; MAX_EVENTS] = unsafe { std::mem::zeroed() };
        
        loop {
            // Check for timed-out connections first
            self.handle_timeouts();
            
            // Calculate timeout based on next timeout check
            let timeout_duration = self.timeout_manager.next_timeout_check();
            let timeout_ms = timeout_duration.as_millis().min(i32::MAX as u128) as c_int;
            
            // Wait for events
            let nfds = unsafe {
                libc::epoll_wait(
                    self.epoll_fd,
                    events.as_mut_ptr(),
                    MAX_EVENTS as c_int,
                    timeout_ms,
                )
            };
            
            if nfds == -1 {
                let err = io::Error::last_os_error();
                if err.kind() == ErrorKind::Interrupted {
                    continue;
                }
                return Err(err);
            }
            
            // Process events
            for i in 0..nfds as usize {
                let event = events[i];
                let fd = event.u64 as RawFd;
                
                if fd == self.listener.as_raw_fd() {
                    self.accept_connections()?;
                } else {
                    self.handle_epoll_connection_event(fd, event.events)?;
                }
            }
        }
    }
    
    fn accept_connections(&mut self) -> io::Result<()> {
        loop {
            match self.listener.accept() {
                Ok((stream, addr)) => {
                    println!("New connection from: {}", addr);
                    
                    // Set non-blocking
                    stream.set_nonblocking(true)?;
                    
                    let fd = stream.as_raw_fd();
                    let conn = match Connection::new(stream, addr) {
                        Ok(c) => c,
                        Err(e) => {
                            eprintln!("Failed to create connection: {}", e);
                            continue;
                        }
                    };
                    
                    // Add to event system
                    self.add_connection_to_events(fd)?;
                    
                    // Add to timeout manager
                    self.timeout_manager.add_connection(fd);
                    
                    self.connections.insert(fd, conn);
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    // No more connections to accept
                    break;
                }
                Err(e) => {
                    eprintln!("Accept error: {}", e);
                    break;
                }
            }
        }
        Ok(())
    }
    
    #[cfg(target_os = "macos")]
    fn add_connection_to_events(&mut self, fd: RawFd) -> io::Result<()> {
        let mut kevent = libc::kevent {
            ident: fd as libc::uintptr_t,
            filter: libc::EVFILT_READ,
            flags: libc::EV_ADD | libc::EV_ENABLE,
            fflags: 0,
            data: 0,
            udata: std::ptr::null_mut(),
        };
        
        let result = unsafe {
            libc::kevent(
                self.kqueue_fd,
                &mut kevent as *mut libc::kevent,
                1,
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
            )
        };
        
        if result == -1 {
            eprintln!("Failed to add connection to kqueue: {}", io::Error::last_os_error());
        }
        Ok(())
    }
    
    #[cfg(target_os = "linux")]
    fn add_connection_to_events(&mut self, fd: RawFd) -> io::Result<()> {
        let mut event = libc::epoll_event {
            events: (libc::EPOLLIN | libc::EPOLLET) as u32,
            u64: fd as u64,
        };
        
        let result = unsafe {
            libc::epoll_ctl(
                self.epoll_fd,
                libc::EPOLL_CTL_ADD,
                fd,
                &mut event as *mut libc::epoll_event,
            )
        };
        
        if result == -1 {
            eprintln!("Failed to add connection to epoll: {}", io::Error::last_os_error());
        }
        Ok(())
    }
    
    #[cfg(target_os = "macos")]
    fn handle_kqueue_connection_event(&mut self, fd: RawFd, filter: i16) -> io::Result<()> {
        // Update activity timestamp
        self.timeout_manager.update_activity(fd);
        
        let should_close = if let Some(conn) = self.connections.get_mut(&fd) {
            if filter == libc::EVFILT_READ {
                // Set reading state based on current parser state
                let state = if conn.is_reading_body() {
                    ConnectionState::ReadingBody
                } else {
                    ConnectionState::ReadingHeaders
                };
                self.timeout_manager.set_connection_state(fd, state);
                
                match conn.handle_read() {
                    Ok(true) => {
                        // Ready to write response
                        println!("Request parsed, generating response...");
                        self.timeout_manager.set_connection_state(fd, ConnectionState::Writing);
                        
                        match conn.send_response() {
                            Ok(()) => {
                                println!("Response generated successfully, enabling write events...");
                                match self.enable_write_events_kqueue(fd) {
                                    Ok(()) => {
                                        println!("Write events enabled successfully");
                                        false
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to enable write events: {}", e);
                                        true
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to generate response: {}", e);
                                true
                            }
                        }
                    }
                    Ok(false) => false,
                    Err(e) => {
                        eprintln!("Error in handle_read: {}", e);
                        true
                    }
                }
            } else if filter == libc::EVFILT_WRITE {
                self.timeout_manager.set_connection_state(fd, ConnectionState::Writing);
                
                match conn.handle_write() {
                    Ok(keep_alive) => {
                        if keep_alive {
                            // Reset for next request on keep-alive connection
                            self.timeout_manager.reset_connection_for_new_request(fd);
                            false
                        } else {
                            true
                        }
                    }
                    Ok(false) => false,
                    Err(_) => true,
                }
            } else {
                true
            }
        } else {
            true
        };
        
        if should_close {
            self.close_connection(fd)?;
        }
        Ok(())
    }
    
    #[cfg(target_os = "linux")]
    fn handle_epoll_connection_event(&mut self, fd: RawFd, events: u32) -> io::Result<()> {
        // Update activity timestamp
        self.timeout_manager.update_activity(fd);
        
        let should_close = if let Some(conn) = self.connections.get_mut(&fd) {
            if events & libc::EPOLLIN as u32 != 0 {
                // Set reading state based on current parser state
                let state = if conn.is_reading_body() {
                    ConnectionState::ReadingBody
                } else {
                    ConnectionState::ReadingHeaders
                };
                self.timeout_manager.set_connection_state(fd, state);
                
                match conn.handle_read() {
                    Ok(true) => {
                        // Ready to write response
                        self.timeout_manager.set_connection_state(fd, ConnectionState::Writing);
                        conn.send_response()?;
                        self.enable_write_events_epoll(fd)?;
                        false
                    }
                    Ok(false) => false,
                    Err(_) => true,
                }
            } else if events & libc::EPOLLOUT as u32 != 0 {
                self.timeout_manager.set_connection_state(fd, ConnectionState::Writing);
                
                match conn.handle_write() {
                    Ok(keep_alive) => {
                        if keep_alive {
                            // Reset for next request on keep-alive connection
                            self.timeout_manager.reset_connection_for_new_request(fd);
                            false
                        } else {
                            true
                        }
                    }
                    Ok(false) => false,
                    Err(_) => true,
                }
            } else if events & (libc::EPOLLHUP | libc::EPOLLERR) as u32 != 0 {
                true
            } else {
                false
            }
        } else {
            true
        };
        
        if should_close {
            self.close_connection(fd)?;
        }
        Ok(())
    }
    
    #[cfg(target_os = "macos")]
    fn enable_write_events_kqueue(&mut self, fd: RawFd) -> io::Result<()> {
        let mut kevent = libc::kevent {
            ident: fd as libc::uintptr_t,
            filter: libc::EVFILT_WRITE,
            flags: libc::EV_ADD | libc::EV_ENABLE | libc::EV_ONESHOT,
            fflags: 0,
            data: 0,
            udata: std::ptr::null_mut(),
        };
        
        unsafe {
            libc::kevent(
                self.kqueue_fd,
                &mut kevent as *mut libc::kevent,
                1,
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
            );
        }
        Ok(())
    }
    
    #[cfg(target_os = "linux")]
    fn enable_write_events_epoll(&mut self, fd: RawFd) -> io::Result<()> {
        let mut event = libc::epoll_event {
            events: (libc::EPOLLOUT | libc::EPOLLET) as u32,
            u64: fd as u64,
        };
        
        unsafe {
            libc::epoll_ctl(
                self.epoll_fd,
                libc::EPOLL_CTL_MOD,
                fd,
                &mut event as *mut libc::epoll_event,
            );
        }
        Ok(())
    }
    
    fn handle_timeouts(&mut self) {
        let timed_out_fds = self.timeout_manager.check_timeouts();
        
        for fd in timed_out_fds {
            println!("Connection {} timed out, closing", fd);
            if let Err(e) = self.close_connection(fd) {
                eprintln!("Error closing timed-out connection {}: {}", fd, e);
            }
        }
    }
    
    fn close_connection(&mut self, fd: RawFd) -> io::Result<()> {
        #[cfg(target_os = "macos")]
        self.remove_from_kqueue(fd);
        
        #[cfg(target_os = "linux")]
        self.remove_from_epoll(fd);
        
        // Remove from timeout manager
        self.timeout_manager.remove_connection(fd);
        
        if let Some(conn) = self.connections.remove(&fd) {
            println!("Closed connection from: {}", conn.addr());
        }
        
        Ok(())
    }
    
    #[cfg(target_os = "macos")]
    fn remove_from_kqueue(&mut self, fd: RawFd) {
        let mut kevents = [
            libc::kevent {
                ident: fd as libc::uintptr_t,
                filter: libc::EVFILT_READ,
                flags: libc::EV_DELETE,
                fflags: 0,
                data: 0,
                udata: std::ptr::null_mut(),
            },
            libc::kevent {
                ident: fd as libc::uintptr_t,
                filter: libc::EVFILT_WRITE,
                flags: libc::EV_DELETE,
                fflags: 0,
                data: 0,
                udata: std::ptr::null_mut(),
            },
        ];
        
        unsafe {
            libc::kevent(
                self.kqueue_fd,
                kevents.as_mut_ptr(),
                2,
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
            );
        }
    }
    
    #[cfg(target_os = "linux")]
    fn remove_from_epoll(&mut self, fd: RawFd) {
        unsafe {
            libc::epoll_ctl(self.epoll_fd, libc::EPOLL_CTL_DEL, fd, std::ptr::null_mut());
        }
    }
}

impl Drop for EventLoop {
    fn drop(&mut self) {
        #[cfg(target_os = "macos")]
        unsafe {
            libc::close(self.kqueue_fd);
        }
        
        #[cfg(target_os = "linux")]
        unsafe {
            libc::close(self.epoll_fd);
        }
    }
}
