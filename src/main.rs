mod constants;
mod utils;

use crate::utils::networks;

#[tokio::main]
async fn main() {
    networks::scan_tcp_ports("127.0.0.1").await;
    networks::check_udp_port("127.0.0.1", 1234).await;
}