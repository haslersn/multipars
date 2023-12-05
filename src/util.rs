use std::{
    fmt::Debug,
    io,
    net::{SocketAddr, ToSocketAddrs},
};

use log::{error, info};

pub fn log_error(name: &str, res: Result<(), impl Debug>) {
    if let Err(e) = res {
        error!("{} failed with error: {:?}", name, e)
    } else {
        info!("{} succeeded", name);
    }
}

pub fn resolve_host(hostname_port: &str) -> io::Result<SocketAddr> {
    let socketaddr = hostname_port.to_socket_addrs()?.next().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::AddrNotAvailable,
            format!("Could not find destination {hostname_port}"),
        )
    })?;
    Ok(socketaddr)
}
