pub mod config;

use etherparse::{NetSlice, SlicedPacket};
use tokio::net::UdpSocket;
use tracing::{error, trace};
use tracing_subscriber::{FmtSubscriber, fmt::time};
use tun::{AsyncDevice, Configuration};

use std::{collections::HashMap, env, io, net::IpAddr};

pub use config::{Config, Peer};

const TUN_PACKET_MTU: u16 = 1472;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("tun error")]
    TunError(#[from] tun::Error),
    #[error("I/O error")]
    IoError(#[from] io::Error),
}

pub struct Device {
    tun_iface: AsyncDevice,
    virtual_addr: IpAddr,
    endpoint_sock: UdpSocket,
    peers: HashMap<IpAddr, String>,
}

impl Device {
    pub async fn new(vpn_config: Config) -> Result<Self, Error> {
        bootstrap_tracing();

        let mut config = Configuration::default();

        config
            .address(vpn_config.interface.virtual_address)
            .netmask(vpn_config.interface.virtual_netmask)
            .up();

        // network connection might have an mtu of 1500
        // the udp header has 28 bytes
        // setting packet mtu to 1472 at the tun interface prevents fragmentation in the network
        config.mtu(TUN_PACKET_MTU);

        let tun_device = tun::create_as_async(&config).inspect_err(|e| {
            error!(
                "[Vpn::new] failed to create tun device with addr = {:?} -> {:?}",
                vpn_config.interface.virtual_address, e
            )
        })?;

        let udp_local_sock = UdpSocket::bind(&vpn_config.interface.endpoint)
            .await
            .inspect_err(|e| {
                error!(
                    "[Vpn::new] failed to bind to udp send socket to addr = {} -> {:?}",
                    vpn_config.interface.endpoint, e
                )
            })?;

        trace!("[Vpn::new] finished setting up tun interface and UDP endpoint socket");

        let mut map = HashMap::new();
        for peer in vpn_config.peers {
            map.insert(peer.virtual_address, peer.endpoint);
        }

        Ok(Device {
            tun_iface: tun_device,
            virtual_addr: vpn_config.interface.virtual_address,
            endpoint_sock: udp_local_sock,
            peers: map,
        })
    }

    #[warn(unused_parens)]
    pub async fn start(&self) -> Result<(), Error> {
        trace!("[Vpn::start] listening on the UDP socket and the tun device");
        loop {
            let mut udp_buf = [0u8; 1600];
            let mut tun_buf = [0u8; 1600];

            tokio::select! {
                res = self.endpoint_sock.recv_from(&mut udp_buf) => {
                    let (len, recv_addr) = res.inspect_err(|e| {
                    error!(
                        "[Vpn::start] failed to recv from udp recv socket -> {:?}",
                        e
                    )
                    })?;

                    trace!(
                        "[Vpn::start] received packet at udp socket from {:?}, attempting to forward packet to tun interface = {:?}",
                        recv_addr, self.virtual_addr
                    );

                    self.tun_iface.send(&udp_buf[..len]).await.inspect_err(|e| {
                        error!(
                            "[Vpn::start] failed to send packet to tun interface = {:?} -> {:?}",
                            self.virtual_addr, e
                        )
                    })?;

                    trace!(
                        "[Vpn::start] forwarded packet to tun interface = {:?}",
                        self.virtual_addr
                    );
                }

                res = self.tun_iface.recv(&mut tun_buf) => {
                    let len = res.inspect_err(|e| {
                    error!(
                        "[Vpn::start] failed to recv from tun interface = {:?} -> {:?}",
                        self.virtual_addr, e
                    )
                    })?;

                    let headers = SlicedPacket::from_ip(&tun_buf).unwrap();
                    let dest_ip_addr = match headers.net.unwrap() {
                        NetSlice::Ipv4(ipv4) => {
                            trace!("[vpn::start] received Ipv4 packet at tun interface: src addr: {:?}, dst addr: {:?}", ipv4.header().source_addr(), ipv4.header().destination_addr());
                            IpAddr::V4(ipv4.header().destination_addr())
                        }
                        NetSlice::Ipv6(ipv6) => {
                            trace!("[vpn::start] received Ipv6 packet at tun interface: src addr: {:?}, dst addr: {:?}", ipv6.header().source_addr(), ipv6.header().destination_addr());
                            continue;
                        }
                        NetSlice::Arp(arp) => {
                            trace!("[vpn::start] received arp packet at tun interface, skipping: {:?}", arp);
                            continue;
                        }
                    };

                    let des_addr = dest_ip_addr.to_string();
                    let send_addr = self.peers.get(&dest_ip_addr).unwrap_or(&des_addr);
                    trace!(
                        "[Vpn::start] received packet at tun interface, attempting to forward packet to remote udp socket = {:?}",
                        send_addr
                    );

                    self.endpoint_sock
                        .send_to(&tun_buf[..len], send_addr)
                        .await
                        .inspect_err(|e| {
                            error!(
                                "[Vpn::start] failed to send packet to remote udp socket = {:?} -> {:?}",send_addr,
                                e
                            )
                        })?;

                    trace!(
                        "[Vpn::start] forwarded packet to remote udp socket = {:?}",
                        send_addr
                    );
                }

            }
        }
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
