use std::collections::HashMap;
use std::io::{self, ErrorKind};
use std::net::TcpListener;
use std::os::unix::io::{AsRawFd, RawFd};
use libc::{self, c_int};
use crate::net::conn::Connection;

const MAX_EVENTS: usize = 1024;
const KQUEUE_TIMEOUT_MS: c_int = 1000;

pub struct EpollServer {
    listener: TcpListener,
    kqueue_fd: RawFd,
    connections: HashMap<RawFd, Connection>,
}

impl EpollServer {
    pub fn new(addr: &str) -> io::Result<Self> {
        // Create and bind listener socket
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        
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
        
        Ok(EpollServer {
            listener,
            kqueue_fd,
            connections: HashMap::new(),
        })
    }
    
    pub fn event_loop(&mut self) -> io::Result<()> {
        let mut events: [libc::kevent; MAX_EVENTS] = unsafe { std::mem::zeroed() };
        
        loop {
            // Wait for events
            let timeout = libc::timespec {
                tv_sec: KQUEUE_TIMEOUT_MS as libc::time_t / 1000,
                tv_nsec: (KQUEUE_TIMEOUT_MS as libc::c_long % 1000) * 1_000_000,
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
                    // New connection
                    self.accept_connections()?;
                } else {
                    // Existing connection
                    self.handle_connection_event(fd, event.filter)?;
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
                    let conn = Connection::new(stream, addr);
                    
                    // Add to kqueue
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
                        continue;
                    }
                    
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
    
    fn handle_connection_event(&mut self, fd: RawFd, filter: i16) -> io::Result<()> {
        let should_close = if let Some(conn) = self.connections.get_mut(&fd) {
            if filter == libc::EVFILT_READ {
                // Data available to read
                match conn.handle_read() {
                    Ok(true) => {
                        // Request complete, send response
                        conn.send_response()?;
                        
                        // Add write filter to kqueue
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
                        false
                    }
                    Ok(false) => false, // Need more data
                    Err(_) => true,     // Error, close connection
                }
            } else if filter == libc::EVFILT_WRITE {
                // Ready to write
                match conn.handle_write() {
                    Ok(true) => true,   // Response sent, close connection
                    Ok(false) => false, // More data to send
                    Err(_) => true,     // Error, close connection
                }
            } else {
                // Other event (error, etc.)
                true
            }
        } else {
            true // Connection not found, should close
        };
        
        if should_close {
            self.close_connection(fd)?;
        }
        
        Ok(())
    }
    
    fn close_connection(&mut self, fd: RawFd) -> io::Result<()> {
        // Remove from kqueue (both read and write filters)
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
        
        // Remove from connections map (this will drop the TcpStream and close the socket)
        if let Some(conn) = self.connections.remove(&fd) {
            println!("Closed connection from: {}", conn.addr());
        }
        
        Ok(())
    }
}

impl Drop for EpollServer {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.kqueue_fd);
        }
    }
}
