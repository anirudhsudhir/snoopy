use std::{env, sync::Arc};

use snoopy::{Vpn, VpnConfig};

#[tokio::main]
async fn main() {
    println!(
        r#"
   _________  ____  ____  ____  __  __
  / ___/ __ \/ __ \/ __ \/ __ \/ / / /
 (__  ) / / / /_/ / /_/ / /_/ / /_/ / 
/____/_/ /_/\____/\____/ .___/\__, /  
                      /_/    /____/   

    A VPN written in Rust
    "#
    );

    let mut args = env::args();
    args.next();
    let tun_addr = args.next().unwrap();
    let udp_remote_addr = args.next().unwrap();
    let udp_local_port = args.next().unwrap().parse::<u16>().unwrap();

    let vpn_config = VpnConfig::new(tun_addr, udp_remote_addr, udp_local_port).unwrap();
    let vpn = Vpn::new(vpn_config).await.unwrap();
    let vpn_tun_listen = Arc::new(vpn);
    let vpn_network_listen = vpn_tun_listen.clone();

    tokio::spawn(async move {
        vpn_tun_listen.tun_listen().await.unwrap();
    });
    vpn_network_listen.network_listen().await.unwrap();
}
