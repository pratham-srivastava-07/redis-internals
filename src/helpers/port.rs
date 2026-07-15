use std::net::{IpAddr, SocketAddr};

pub fn get_socket_address() -> SocketAddr {
    let addrr = SocketAddr::from(([127, 0, 0, 1], 7379));
    addrr
}

pub fn port_and_host() -> (u16, IpAddr) {
    let addrr = get_socket_address();

    (addrr.port(), addrr.ip())
}