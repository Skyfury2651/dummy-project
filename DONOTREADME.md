### How this works

1. It scans all ports from 1 to 65535.
2. It uses a semaphore to limit the number of concurrent connections.
3. It logs the results to a file.


### How to run
 make run

### How to build
 make build

### How to clean
 make clean






File structure


```
src/
├── constants
│ └── ports.rs
├── main.rs
└── utils/
    └── networks.rs
```

It's run mainly on `scan_ports` function in `networks.rs` file.
Receive ip address and scan all ports using some features of rust.

Idea is try connecting to all ports can be connected or not.


Running asynchronously using `tokio` spawn. 

`FutureUnordered` is used to run all tasks concurrently.

`Semaphore` is used to limit the number of concurrent connections in case of large number of connections.

`TcpStream` is used to connect to the port.

`mpsc::channel` is used to send the results to the main thread.











