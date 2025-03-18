use tokio::net::UdpSocket;
use tun::{AsyncDevice, Configuration};

use std::{
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};

const TUN_PACKET_MTU: u16 = 1472;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("tun error")]
    TunError(#[from] tun::Error),
    #[error("I/O error")]
    IoError(#[from] io::Error),
}

#[derive(Debug)]
pub struct VpnConfig {
    tun_addr: Ipv4Addr,
    udp_listen_port: u16,
    udp_send_port: u16,
}

impl VpnConfig {
    pub fn new(
        tun_addr: Ipv4Addr,
        udp_listen_port: u16,
        udp_send_port: u16,
    ) -> Result<Self, Error> {
        Ok(VpnConfig {
            tun_addr,
            udp_listen_port,
            udp_send_port,
        })
    }
}

pub struct Vpn {
    tun_device: AsyncDevice,
    udp_listen_sock: UdpSocket,
    udp_send_sock: UdpSocket,
}

impl Vpn {
    pub async fn new(vpn_config: VpnConfig) -> Result<Self, Error> {
        let mut config = Configuration::default();

        config
            .address(vpn_config.tun_addr)
            .netmask((255, 255, 255, 0))
            .up();

        // network connection might have an mtu of 1500
        // the udp header has 28 bytes
        // setting packet mtu to 1472 at the tun interface prevents fragmentation in the network
        config.mtu(TUN_PACKET_MTU);

        let tun_device = tun::create_as_async(&config)?;

        let udp_send_sock = UdpSocket::bind(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            vpn_config.udp_send_port,
        ))
        .await?;

        let udp_listen_sock = UdpSocket::bind(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            vpn_config.udp_listen_port,
        ))
        .await?;

        Ok(Vpn {
            tun_device,
            udp_send_sock,
            udp_listen_sock,
        })
    }

    pub async fn network_listen(&self) -> Result<(), Error> {
        let mut buf = [0u8; 1500];
        loop {
            self.udp_listen_sock.recv(&mut buf).await?;
        }

        #[allow(unreachable_code)]
        Ok(())
    }

    pub async fn tun_listen(&self) -> Result<(), Error> {
        let mut buf = [0u8; TUN_PACKET_MTU as usize];
        loop {
            self.tun_device.recv(&mut buf).await?;
            self.udp_send_sock.send(&buf).await?;
        }

        #[allow(unreachable_code)]
        Ok(())
    }
}
