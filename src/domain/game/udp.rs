//! Bounded UDP port probing for the legacy `/api/game/8level/udp/port` route.

use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};

const START_PORT: u16 = 10998;
const END_PORT: u16 = 11038;

/// Returns currently bindable UDP ports in Go's legacy DST range.
pub(crate) fn free_legacy_udp_ports() -> Vec<u16> {
    (START_PORT..=END_PORT)
        .filter(|port| {
            let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), *port);
            UdpSocket::bind(addr).is_ok()
        })
        .collect()
}

pub(crate) fn bounded_legacy_udp_ports() -> Result<Vec<u16>, &'static str> {
    let ports = free_legacy_udp_ports();
    if ports
        .iter()
        .any(|port| !(START_PORT..=END_PORT).contains(port))
    {
        tracing::error!("UDP scanner produced a port outside the bounded legacy range");
        return Err("scan udp ports");
    }
    tracing::debug!(count = ports.len(), "scanned legacy DST UDP port range");
    Ok(ports)
}
