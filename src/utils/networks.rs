use tokio::{net::TcpStream, sync::{Semaphore, mpsc}};
use futures::stream::{FuturesUnordered, StreamExt};
use std::net::IpAddr;
use netstat2::{get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo};
use sysinfo::{System, SystemExt, ProcessExt, Pid};

use tokio::net::UdpSocket;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::time;

use crate::constants::ports::ALL_PORTS;

#[derive(Debug)]
enum PortErrorType {
    Refused,
    Timeout,
    Resource,
    Other,
}

#[derive(Debug)]
struct PortLog {
    port: u16,
    error_type: PortErrorType,
    message: String,
}

#[derive(Debug)]
pub struct OpenPort {
    pub port: u16,
    pub port_type: PortType,
}

#[derive(Debug)]
pub enum PortType {
    TCP,
    UDP,
}

pub async fn port_check(ip: &str) -> Result<Vec<OpenPort>, String> {
    let mut open_ports = Vec::new();
    
    // Check TCP ports
    let tcp_ports = scan_tcp_ports(ip).await;
    open_ports.extend(tcp_ports.into_iter().map(|port| OpenPort {
        port,
        port_type: PortType::TCP,
    }));

    // Check UDP ports
    let udp_ports = scan_udp_ports(ip).await;
    open_ports.extend(udp_ports.into_iter().map(|port| OpenPort {
        port,
        port_type: PortType::UDP,
    }));

    Ok(open_ports)
}

pub async fn scan_tcp_ports(ip: &str) -> Vec<u16> {
    let ports = ALL_PORTS;
    let total_ports = ports.len();
    let ip_addr: IpAddr = ip.parse().unwrap();
    let semaphore = std::sync::Arc::new(Semaphore::new(100));
    let (log_tx, mut log_rx) = mpsc::channel::<PortLog>(1000);
    let mut open_ports = Vec::new();
    let mut scanned = 0;

    // Spawn logger task
    tokio::spawn(async move {
        use std::fs::OpenOptions;
        use std::io::Write;

        let mut files = std::collections::HashMap::new();

        while let Some(log) = log_rx.recv().await {
            let filename = match log.error_type {
                PortErrorType::Refused => "refused_ports.txt",
                PortErrorType::Timeout => "timeout_ports.txt",
                PortErrorType::Resource => "resource_errors.txt",
                PortErrorType::Other => "other_errors.txt",
            };

            let file = files.entry(filename).or_insert_with(|| {
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(filename)
                    .expect("Failed to open log file")
            });

            let _ = writeln!(file, "Port {}: {}", log.port, log.message);
        }
    });

    let mut tasks = FuturesUnordered::new();

    for port in ports {
        let addr = format!("{}:{}", ip_addr, port);
        let sem = semaphore.clone();
        let tx = log_tx.clone();

        tasks.push(tokio::spawn(async move {
            let _permit = sem.acquire_owned().await.unwrap();

            match TcpStream::connect(&addr).await {
                Ok(_) => {
                    println!("TCP Port {} is open", port);
                    Some(port)
                },
                Err(e) => {
                    let err_str = e.to_string();
                    let err_type = if err_str.contains("refused") {
                        PortErrorType::Refused
                    } else if err_str.contains("timed out") {
                        PortErrorType::Timeout
                    } else if err_str.contains("buffer space") || err_str.contains("queue was full") {
                        PortErrorType::Resource
                    } else {
                        PortErrorType::Other
                    };

                    let _ = tx
                        .send(PortLog {
                            port,
                            error_type: err_type,
                            message: err_str,
                        })
                        .await;
                    None
                }
            }
        }));
    }

    while let Some(result) = tasks.next().await {
        scanned += 1;
        let percent = (scanned as f32 / total_ports as f32 * 100.0) as u32;
        print!("\rTCP Scan Progress: {}%", percent);
        if let Ok(Some(port)) = result {
            open_ports.push(port);
        }
    }
    println!();

    open_ports
}

pub async fn scan_udp_ports(ip: &str) -> Vec<u16> {
    let ports = ALL_PORTS;
    let total_ports = ports.len();
    let semaphore = std::sync::Arc::new(Semaphore::new(100));
    let mut open_ports = Vec::new();
    let mut scanned = 0;

    let mut tasks = FuturesUnordered::new();

    for port in ports {
        let ip = ip.to_string();
        let sem = semaphore.clone();

        tasks.push(tokio::spawn(async move {
            let _permit = sem.acquire_owned().await.unwrap();
            match check_udp_port(&ip, port).await {
                Ok(true) => {
                    println!("UDP Port {} is open", port);
                    Some(port)
                },
                _ => None
            }
        }));
    }

    while let Some(result) = tasks.next().await {
        scanned += 1;
        let percent = (scanned as f32 / total_ports as f32 * 100.0) as u32;
        print!("\rUDP Scan Progress: {}%", percent);
        if let Ok(Some(port)) = result {
            open_ports.push(port);
        }
    }
    println!();

    open_ports
}

pub async fn check_udp_port(target_ip: &str, port: u16) -> Result<bool, String> {
    let addr: SocketAddr = format!("{}:{}", target_ip, port)
        .parse()
        .map_err(|e| format!("Invalid address: {}", e))?;

    let socket = UdpSocket::bind("0.0.0.0:0")
        .await
        .map_err(|e| format!("Failed to bind socket: {}", e))?;

    let payload = b"ping";
    if let Err(e) = socket.send_to(payload, addr).await {
        return Err(format!("Failed to send: {}", e));
    }

    let mut buf = [0u8; 1024];
    let timeout = Duration::from_secs(1);

    match time::timeout(timeout, socket.recv_from(&mut buf)).await {
        Ok(Ok((_len, _src))) => Ok(true),
        Ok(Err(e)) => Err(format!("Receive error: {}", e)),
        Err(_) => Ok(false),
    }
}

pub async fn port_service_check(port: u16) -> Result<String, String> {
    let sockets = get_sockets_info(
        AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6,
        ProtocolFlags::TCP | ProtocolFlags::UDP,
    ).map_err(|e| format!("Failed to get socket info: {}", e))?;

    let mut sys = System::new();
    sys.refresh_all();

    for socket in sockets {
        match socket.protocol_socket_info {
            ProtocolSocketInfo::Tcp(tcp_info) => {
                if tcp_info.local_port == port {
                    if let Some(pid) = socket.associated_pids.first() {
                        if let Some(process) = sys.process(Pid::from(*pid as usize)) {
                            return Ok(format!("TCP: {} (PID: {})", process.name(), pid));
                        }
                    }
                }
            },
            ProtocolSocketInfo::Udp(udp_info) => {
                if udp_info.local_port == port {
                    if let Some(pid) = socket.associated_pids.first() {
                        if let Some(process) = sys.process(Pid::from(*pid as usize)) {
                            return Ok(format!("UDP: {} (PID: {})", process.name(), pid));
                        }
                    }
                }
            }
        }
    }

    Err(format!("No service found using port {}", port))
}
