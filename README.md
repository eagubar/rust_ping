# rust_ping

A command-line ping utility written in Rust with visual graphs, statistics, and export capabilities.

## Features

- **ICMP Echo Request/Reply** - Standard ping functionality using raw sockets
- **Visual Bar Graphs** - Real-time latency visualization with color-coded bars
- **Line Graphs** - ASCII line graph showing latency trends over time
- **Latency Distribution** - Histogram showing the distribution of response times
- **Color-Coded Output** - Green (<20ms), Yellow (20-50ms), Orange (50-100ms), Red (>100ms)
- **Statistics** - Min, Max, Average, Standard Deviation, and packet loss percentage
- **Export Options** - Save results to JSON or CSV format
- **DNS Resolution** - Supports both IP addresses and hostnames

## Installation

### Prerequisites

- Rust 1.70 or later
- Root/sudo privileges (required for raw sockets)

### Building from Source

```bash
git clone https://github.com/yourusername/rust_ping.git
cd rust_ping
cargo build --release
The binary will be available at ./target/release/rust_ping

Usage

Note: This tool requires root privileges to send ICMP packets.

Basic Ping
Bash

sudo ./target/release/rust_ping 8.8.8.8
Output:

text

╔════════════════════════════════════════════════════════════╗
║       PING 8.8.8.8 - 10 packets                            ║
╚════════════════════════════════════════════════════════════╝
  ✓ Reply from 8.8.8.8: seq=0 time=   9.65ms
  ✓ Reply from 8.8.8.8: seq=1 time=   9.56ms
  ✓ Reply from 8.8.8.8: seq=2 time=  12.49ms
  ...

╔════════════════════════════════════════════════════════════╗
║                      STATISTICS                             ║
╚════════════════════════════════════════════════════════════╝
  Host: 8.8.8.8
  Packets: 10 sent, 10 received, 0 lost (0.0%)

  RTT:
    Min: 9.56ms
    Avg: 12.82ms
    Max: 21.25ms
    StdDev: 3.14ms
Bar Graph Mode
Bash

sudo ./target/release/rust_ping 8.8.8.8 -g
Output:

text

╔════════════════════════════════════════════════════════════╗
║       PING 8.8.8.8 - 10 packets                            ║
╚════════════════════════════════════════════════════════════╝

  Legend: ● <20ms ● 20-50ms ● >50ms

  seq=0   │██████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░│    7.62ms  <- 8.8.8.8
  seq=1   │██████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░│    8.49ms  <- 8.8.8.8
  seq=2   │██████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░│    7.83ms  <- 8.8.8.8
  seq=3   │█████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░│   12.29ms  <- 8.8.8.8
  seq=4   │████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░│   10.84ms  <- 8.8.8.8
  ...
Line Graph Mode
Bash

sudo ./target/release/rust_ping 8.8.8.8 -l
Output:

text

╔════════════════════════════════════════════════════════════╗
║              LATENCY GRAPH OVER TIME                        ║
╚════════════════════════════════════════════════════════════╝
    10.3ms ┤       ●
    10.0ms │       │ ●
     9.6ms │       │ │
     9.2ms │       │ │
     8.8ms │ ● ●   │ │
     8.4ms │ │ │   │●│
     8.0ms │●│ │● ●│││
     7.6ms │││ ││ ││││
     7.2ms │││ ││ ││││
     6.8ms ┤││●││●││││
         └──────────
          0    5
          seq ->
Combined Mode with Export
Bash

sudo ./target/release/rust_ping 8.8.8.8 -g -l -c 20 --json results.json --csv results.csv
Command Line Options

Option  Short   Description     Default
<HOST>          IP address or hostname to ping  Required
--count -c      Number of ping requests to send 10
--timeout       -t      Timeout in seconds for each request     2
--graph -g      Display real-time bar graph     false
--line-graph    -l      Display line graph after completion     false
--json          Export results to JSON file     -
--csv           Export results to CSV file      -
--help  -h      Show help message       -
--version       -V      Show version    -
Export Formats

JSON Export
Bash

sudo ./target/release/rust_ping 1.1.1.1 -c 10 --json results.json
Output file structure:

JSON

{
  "host": "1.1.1.1",
  "ip_address": "1.1.1.1",
  "timestamp_start": "2024-01-15 10:30:00",
  "timestamp_end": "2024-01-15 10:30:10",
  "timeout_seconds": 2,
  "results": [
    {
      "seq": 0,
      "rtt_ms": 7.52,
      "success": true,
      "timestamp": "2024-01-15 10:30:00.123"
    },
    {
      "seq": 1,
      "rtt_ms": 12.95,
      "success": true,
      "timestamp": "2024-01-15 10:30:01.125"
    }
  ],
  "statistics": {
    "min_ms": 6.20,
    "max_ms": 12.98,
    "avg_ms": 10.26,
    "std_dev_ms": 2.36,
    "packets_sent": 10,
    "packets_received": 10,
    "packets_lost": 0,
    "packet_loss_percent": 0.0
  }
}
CSV Export
Bash

sudo ./target/release/rust_ping 1.1.1.1 -c 20 --csv results.csv
Output file structure:

csv

# Ping Report
# Host: 1.1.1.1
# IP: 1.1.1.1
# Generated: 2024-01-15 10:30:20
#
seq,rtt_ms,success,timestamp
0,8.74,true,2024-01-15 10:30:00.123
1,6.53,true,2024-01-15 10:30:01.125
2,6.30,true,2024-01-15 10:30:02.127

# Statistics
# packets_sent,packets_received,packets_lost,loss_percent,min_ms,avg_ms,max_ms,std_dev_ms
20,20,0,0.00,6.30,9.80,13.63,2.40
Latency Distribution

When using -g or -l flags, a histogram of latency distribution is displayed:

text

╔════════════════════════════════════════════════════════════╗
║               LATENCY DISTRIBUTION                          ║
╚════════════════════════════════════════════════════════════╝
    0-10ms │████████████████████████████████████████             8 ( 80.0%)
   10-20ms │██████████                                           2 ( 20.0%)
   20-50ms │                                                     0 (  0.0%)
  50-100ms │                                                     0 (  0.0%)
    >100ms │                                                     0 (  0.0%)
Color Coding

Color   Latency Range   Indication
Green   < 20ms  Excellent
Yellow  20-50ms Good
Orange  50-100ms        Fair
Red     > 100ms Poor
Dependencies

clap - Command line argument parsing
colored - Terminal colors
pnet - Low-level networking
serde - Serialization framework
serde_json - JSON support
chrono - Date and time handling
Platform Support

Platform        Status
Linux   Supported
macOS   Supported
Windows Not tested
Troubleshooting

Permission Denied
text

Error: Error creating channel (root permissions?): Operation not permitted
Solution: Run with sudo:

Bash

sudo ./target/release/rust_ping 8.8.8.8
Host Not Found
text

Error: Could not resolve: invalid.hostname
Solution: Verify the hostname is correct and DNS is working.

Examples

Bash

# Basic ping with 5 requests
sudo ./target/release/rust_ping google.com -c 5

# Visual bar graph with 20 requests
sudo ./target/release/rust_ping 8.8.8.8 -c 20 -g

# Full visualization with exports
sudo ./target/release/rust_ping 1.1.1.1 -c 30 -g -l --json report.json --csv report.csv

# Custom timeout (5 seconds)
sudo ./target/release/rust_ping 10.0.0.1 -t 5 -c 10

# Ping with line graph only
sudo ./target/release/rust_ping cloudflare.com -l -c 15
License

MIT License - see LICENSE for details.

Contributing

Contributions are welcome. Please open an issue or submit a pull request.

Fork the repository
Create your feature branch (git checkout -b feature/new-feature)
Commit your changes (git commit -am 'Add new feature')
Push to the branch (git push origin feature/new-feature)
Open a Pull Request
