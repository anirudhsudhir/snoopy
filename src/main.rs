mod config;

use std::{env, sync::Arc};

use snoopy::{Vpn, VpnConfig};

const USAGE: &str = "Usage: ./snoopy [path_to_config]";

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
    let conf = config::parse_config(&args.next().expect(USAGE));

    let vpn_config = VpnConfig::new(
        conf.interface.virtual_address,
        conf.interface.endpoint,
        conf.peer.endpoint,
    )
    .unwrap();
    let vpn = Vpn::new(vpn_config).await.unwrap();
    let vpn_tun_listen = Arc::new(vpn);
    let vpn_network_listen = vpn_tun_listen.clone();

    tokio::spawn(async move {
        vpn_tun_listen.tun_listen().await.unwrap();
    });
    vpn_network_listen.network_listen().await.unwrap();
}
