use std::env;

use snoopy::{Device, config};

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

    let dev = Device::new(conf).await.unwrap();
    dev.start().await.expect("failed to start the VPN");
}
