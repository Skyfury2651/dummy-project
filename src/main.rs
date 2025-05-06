mod constants;
mod utils;

use crate::utils::networks;

#[tokio::main]
async fn main() {
    match networks::port_check("127.0.0.1").await {
        Ok(open_ports) => {
            for port in open_ports {
                println!("Port {} is open ({:?})", port.port, port.port_type);
                match networks::port_service_check(port.port).await {
                    Ok(service_info) => println!("Service info: {}", service_info),
                    Err(e) => println!("Service check error: {}", e),
                }
            }
        },
        Err(e) => println!("Error: {}", e),
    }
}