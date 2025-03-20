use tokio::net::UdpSocket;
use tracing::error;
use tracing_subscriber::{FmtSubscriber, fmt::time};

use tun::{AsyncDevice, Configuration};

use std::{
    env, io,
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
    tun_addr: String,
    udp_remote_addr: String,
    udp_local_port: u16,
}

impl VpnConfig {
    pub fn new(
        tun_addr: String,
        udp_remote_addr: String,
        udp_local_port: u16,
    ) -> Result<Self, Error> {
        Ok(VpnConfig {
            tun_addr,
            udp_remote_addr,
            udp_local_port,
        })
    }
}

pub struct Vpn {
    tun_device: AsyncDevice,
    udp_local_sock: UdpSocket,
    udp_remote_addr: String,
}

impl Vpn {
    pub async fn new(vpn_config: VpnConfig) -> Result<Self, Error> {
        bootstrap_tracing();

        let mut config = Configuration::default();

        config
            .address(vpn_config.tun_addr)
            .netmask((255, 255, 255, 0))
            .up();

        // network connection might have an mtu of 1500
        // the udp header has 28 bytes
        // setting packet mtu to 1472 at the tun interface prevents fragmentation in the network
        config.mtu(TUN_PACKET_MTU);

        let tun_device = tun::create_as_async(&config)
            .inspect_err(|e| error!("[Vpn::new] failed to create tun device -> {:?}", e))?;

        let udp_local_sock = UdpSocket::bind(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            vpn_config.udp_local_port,
        ))
        .await
        .inspect_err(|e| error!("[Vpn::new] failed to bind to udp send socket -> {:?}", e))?;

        Ok(Vpn {
            tun_device,
            udp_local_sock,
            udp_remote_addr: vpn_config.udp_remote_addr,
        })
    }

    pub async fn network_listen(&self) -> Result<(), Error> {
        let mut buf = [0u8; 1500];
        loop {
            self.udp_local_sock.recv(&mut buf).await.inspect_err(|e| {
                error!(
                    "[Vpn::network_listen] failed to recv from udp recv socket -> {:?}",
                    e
                )
            })?;
        }

        #[allow(unreachable_code)]
        Ok(())
    }

    pub async fn tun_listen(&self) -> Result<(), Error> {
        let mut buf = [0u8; TUN_PACKET_MTU as usize];
        loop {
            self.tun_device.recv(&mut buf).await.inspect_err(|e| {
                error!(
                    "[Vpn::tun_listen] failed to recv from tun interface -> {:?}",
                    e
                )
            })?;

            self.udp_local_sock
                .send_to(&buf, &self.udp_remote_addr)
                .await
                .inspect_err(|e| {
                    error!(
                        "[Vpn::tun_listen] failed to send tun packet to udp send socket -> {:?}",
                        e
                    )
                })?;
        }

        #[allow(unreachable_code)]
        Ok(())
    }
}

fn bootstrap_tracing() {
    let logging_level = match env::var("LOG_LEVEL") {
        Ok(level) => match level.as_str() {
            "TRACE" => tracing::Level::TRACE,
            "DEBUG" => tracing::Level::DEBUG,
            "INFO" => tracing::Level::INFO,
            "WARN" => tracing::Level::WARN,
            "ERROR" => tracing::Level::ERROR,
            _ => tracing::Level::INFO,
        },
        Err(_) => tracing::Level::INFO,
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(logging_level)
        .with_timer(time::ChronoLocal::rfc_3339())
        .with_target(true)
        .with_writer(std::io::stderr)
        .finish();

    let _ = tracing::subscriber::set_global_default(subscriber);
}
