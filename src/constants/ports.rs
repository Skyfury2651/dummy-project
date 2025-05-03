// src/constants/ports.rs
use std::ops::Range;

// pub const COMMON_PORTS: &[u16] = &[
//     80, 443, 22, 21, 3306, 5432, 6379, 1433,
// ];
// pub const WEB_PORTS: &[u16] = &[80, 443];
// pub const DB_PORTS: &[u16] = &[3306, 5432, 1433, 6379];
pub const ALL_PORTS: Range<u16> = 1..65535;
