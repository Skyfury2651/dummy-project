Summary

Create a function port_service_check that determines which service is using a given TCP or UDP port.

Steps

Import Dependencies

Add netstat2 = "0.11.1" to Cargo.toml.

Add sysinfo = { version = "0.35.0", features = ["process"] } to Cargo.toml.

Implement port_service_check

Use netstat2::get_sockets_info with flags AddressFamilyFlags::IPV4 | IPV6 and ProtocolFlags::TCP | UDP.

Refresh process list with sysinfo::System::new_all() and refresh_processes().

Match on ProtocolSocketInfo::Tcp and ProtocolSocketInfo::Udp for the target port.

Retrieve PID from associated_pids and lookup the process name via sys.process(pid).

