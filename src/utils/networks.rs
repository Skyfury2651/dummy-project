use tokio::{net::TcpStream, sync::{Semaphore, mpsc}};
use futures::stream::{FuturesUnordered, StreamExt};
use std::net::IpAddr;


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

pub async fn port_check(ip: &str) -> Result<bool, String> {
    // Check TCP ports first
    let tcp_open = scan_tcp_ports(ip).await;
    if tcp_open {
        return Ok(true);
    }

    // If no TCP ports are open, check UDP ports
    let udp_open = scan_udp_ports(ip).await;
    Ok(udp_open)
}

pub async fn scan_tcp_ports(ip: &str) -> bool {
    let ports = ALL_PORTS;
    let ip_addr: IpAddr = ip.parse().unwrap();
    let semaphore = std::sync::Arc::new(Semaphore::new(100));
    let (log_tx, mut log_rx) = mpsc::channel::<PortLog>(1000);
    let mut found_open_port = false;

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
                    true
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
                    false
                }
            }
        }));
    }

    while let Some(result) = tasks.next().await {
        if let Ok(true) = result {
            found_open_port = true;
            break;
        }
    }

    found_open_port
}

pub async fn scan_udp_ports(ip: &str) -> bool {
    let ports = ALL_PORTS;
    let semaphore = std::sync::Arc::new(Semaphore::new(100));
    let mut found_open_port = false;

    let mut tasks = FuturesUnordered::new();

    for port in ports {
        let ip = ip.to_string();
        let sem = semaphore.clone();

        tasks.push(tokio::spawn(async move {
            let _permit = sem.acquire_owned().await.unwrap();
            match check_udp_port(&ip, port).await {
                Ok(true) => {
                    println!("UDP Port {} is open", port);
                    true
                },
                _ => false
            }
        }));
    }

    while let Some(result) = tasks.next().await {
        if let Ok(true) = result {
            found_open_port = true;
            break;
        }
    }

    found_open_port
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