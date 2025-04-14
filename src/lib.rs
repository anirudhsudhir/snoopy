use tokio::net::UdpSocket;
use tracing::{error, trace};
use tracing_subscriber::{FmtSubscriber, fmt::time};

use tun::{AsyncDevice, Configuration};

use std::{env, io};

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
    udp_local_addr: String,
    udp_remote_addr: String,
}

impl VpnConfig {
    pub fn new(
        tun_addr: String,
        udp_local_addr: String,
        udp_remote_addr: String,
    ) -> Result<Self, Error> {
        Ok(VpnConfig {
            tun_addr,
            udp_local_addr,
            udp_remote_addr,
        })
    }
}

pub struct Vpn {
    tun_device: AsyncDevice,
    tun_addr: String,
    udp_local_sock: UdpSocket,
    udp_remote_addr: String,
}

impl Vpn {
    pub async fn new(vpn_config: VpnConfig) -> Result<Self, Error> {
        bootstrap_tracing();

        let mut config = Configuration::default();

        config
            .address(&vpn_config.tun_addr)
            .netmask((255, 255, 255, 0))
            .up();

        // network connection might have an mtu of 1500
        // the udp header has 28 bytes
        // setting packet mtu to 1472 at the tun interface prevents fragmentation in the network
        config.mtu(TUN_PACKET_MTU);

        let tun_device = tun::create_as_async(&config).inspect_err(|e| {
            error!(
                "[Vpn::new] failed to create tun device with addr = {:?} -> {:?}",
                vpn_config.tun_addr, e
            )
        })?;

        let udp_local_sock = UdpSocket::bind(&vpn_config.udp_local_addr)
            .await
            .inspect_err(|e| {
                error!(
                    "[Vpn::new] failed to bind to udp send socket to addr = {} -> {:?}",
                    vpn_config.udp_local_addr, e
                )
            })?;

        trace!("[Vpn::new] finished setting up tun interface and udp sockets");

        Ok(Vpn {
            tun_device,
            tun_addr: vpn_config.tun_addr,
            udp_local_sock,
            udp_remote_addr: vpn_config.udp_remote_addr,
        })
    }

    pub async fn network_listen(&self) -> Result<(), Error> {
        trace!("[Vpn::network_listen] listening for packets on udp socket");
        let mut buf = [0u8; 1600];
        loop {
            let (len, _) = self
                .udp_local_sock
                // NOTE: recv() requires UDP socket to be connected, else fails
                // recv_from() can receive UDP datagrams from arbitrary connections
                .recv_from(&mut buf)
                .await
                .inspect_err(|e| {
                    error!(
                        "[Vpn::network_listen] failed to recv from udp recv socket = {:?} -> {:?}",
                        self.udp_remote_addr, e
                    )
                })?;

            trace!(
                "[Vpn::network_listen] received packet at udp socket, attempting to forward packet to tun interface = {:?}",
                self.tun_addr
            );

            self.tun_device.send(&buf[..len]).await.inspect_err(|e| {
                error!(
                    "[Vpn::network_listen] failed to send packet to tun interface = {:?} -> {:?}",
                    self.tun_addr, e
                )
            })?;

            trace!(
                "[Vpn::network_listen] forwarded packet to tun interface = {:?}",
                self.tun_addr
            );
        }

        #[allow(unreachable_code)]
        Ok(())
    }

    pub async fn tun_listen(&self) -> Result<(), Error> {
        trace!(
            "[Vpn::tun_listen] listening for packets on the tun interface = {:?}",
            self.tun_addr
        );

        let mut buf = [0u8; TUN_PACKET_MTU as usize];
        loop {
            self.tun_device.recv(&mut buf).await.inspect_err(|e| {
                error!(
                    "[Vpn::tun_listen] failed to recv from tun interface = {:?} -> {:?}",
                    self.tun_addr, e
                )
            })?;

            trace!(
                "[Vpn::tun_listen] received packet at tun interface, attempting to forward packet to remote udp socket = {:?}",
                self.udp_remote_addr
            );

            self.udp_local_sock
                .send_to(&buf, &self.udp_remote_addr)
                .await
                .inspect_err(|e| {
                    error!(
                        "[Vpn::tun_listen] failed to send packet to remote udp socket = {:?} -> {:?}", self.udp_remote_addr,
                        e
                    )
                })?;

            trace!(
                "[Vpn::tun_listen] forwarded packet to remote udp socket = {:?}",
                self.udp_remote_addr
            );
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
