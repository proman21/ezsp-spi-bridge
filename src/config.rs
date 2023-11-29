use std::net::{IpAddr, SocketAddr};


#[derive(Debug)]
pub struct Config {
    address: IpAddr,
    port: u16
}

impl Config {
    pub fn new(address: IpAddr, port: u16) -> Config {
        Config { address, port }
    }

    pub fn socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.address, self.port)
    }
}