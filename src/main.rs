use clap::Parser;
use colored::*;
use chrono::{DateTime, Local};
use pnet::packet::icmp::echo_request::MutableEchoRequestPacket;
use pnet::packet::icmp::{IcmpCode, IcmpTypes};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::Packet;
use pnet::transport::{
    icmp_packet_iter, transport_channel, TransportChannelType::Layer4,
    TransportProtocol::Ipv4,
};
use serde::Serialize;
use std::fs::File;
use std::io::Write;
use std::net::IpAddr;
use std::time::{Duration, Instant};

/// Rust Ping Tool with CLI graphs and export options
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// IP address or hostname to ping
    host: String,

    /// Number of pings to send
    #[arg(short, long, default_value_t = 10)]
    count: u32,

    /// Timeout in seconds
    #[arg(short, long, default_value_t = 2)]
    timeout: u64,

    /// Show bar graph
    #[arg(short, long)]
    graph: bool,

    /// Show line graph at the end
    #[arg(short, long)]
    line_graph: bool,

    /// Export results to JSON file
    #[arg(long, value_name = "FILE")]
    json: Option<String>,

    /// Export results to CSV file
    #[arg(long, value_name = "FILE")]
    csv: Option<String>,
}

// Result of each ping
#[derive(Clone, Serialize)]
struct PingResult {
    seq: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    rtt_ms: Option<f64>,
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<String>,
}

// Statistics structure for export
#[derive(Serialize)]
struct PingStatistics {
    min_ms: Option<f64>,
    max_ms: Option<f64>,
    avg_ms: Option<f64>,
    std_dev_ms: Option<f64>,
    packets_sent: u32,
    packets_received: u32,
    packets_lost: u32,
    packet_loss_percent: f64,
}

// Complete report structure for JSON export
#[derive(Serialize)]
struct PingReport {
    host: String,
    ip_address: String,
    timestamp_start: String,
    timestamp_end: String,
    timeout_seconds: u64,
    results: Vec<PingResult>,
    statistics: PingStatistics,
}

fn checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut i = 0;

    while i < data.len() - 1 {
        sum += u16::from_be_bytes([data[i], data[i + 1]]) as u32;
        i += 2;
    }

    if data.len() % 2 == 1 {
        sum += (data[data.len() - 1] as u32) << 8;
    }

    while (sum >> 16) > 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    !sum as u16
}

fn create_icmp_packet(sequence: u16, identifier: u16) -> Vec<u8> {
    let mut buffer = vec![0u8; 64];
    
    let mut packet = MutableEchoRequestPacket::new(&mut buffer).unwrap();
    packet.set_icmp_type(IcmpTypes::EchoRequest);
    packet.set_icmp_code(IcmpCode::new(0));
    packet.set_sequence_number(sequence);
    packet.set_identifier(identifier);
    packet.set_payload(b"RustPing!");
    
    let cs = checksum(packet.packet());
    packet.set_checksum(cs);
    
    buffer
}

/// Get color based on latency
fn get_latency_color(rtt: f64) -> ColoredString {
    let rtt_str = format!("{:>7.2}ms", rtt);
    if rtt < 20.0 {
        rtt_str.green()
    } else if rtt < 50.0 {
        rtt_str.yellow()
    } else if rtt < 100.0 {
        rtt_str.truecolor(255, 165, 0) // orange
    } else {
        rtt_str.red()
    }
}

/// Draw proportional horizontal bar
fn draw_bar(rtt: f64, max_rtt: f64, width: usize) -> String {
    let bar_width = ((rtt / max_rtt) * width as f64).min(width as f64) as usize;
    let empty_width = width.saturating_sub(bar_width);
    
    let bar_char = "█";
    let empty_char = "░";
    
    let bar: String = bar_char.repeat(bar_width);
    let empty: String = empty_char.repeat(empty_width);
    
    // Color based on latency
    let colored_bar = if rtt < 20.0 {
        bar.green()
    } else if rtt < 50.0 {
        bar.yellow()
    } else if rtt < 100.0 {
        bar.truecolor(255, 165, 0)
    } else {
        bar.red()
    };
    
    format!("│{}{}│", colored_bar, empty.dimmed())
}

/// Print result with bar graph
fn print_with_bar(seq: u32, rtt: Option<f64>, max_rtt: f64, addr: IpAddr) {
    const BAR_WIDTH: usize = 40;
    
    match rtt {
        Some(time) => {
            let bar = draw_bar(time, max_rtt.max(1.0), BAR_WIDTH);
            println!(
                "  seq={:<3} {} {}  <- {}",
                seq,
                bar,
                get_latency_color(time),
                addr.to_string().dimmed()
            );
        }
        None => {
            let timeout_bar = "×".repeat(BAR_WIDTH);
            println!(
                "  seq={:<3} │{}│ {}",
                seq,
                timeout_bar.red(),
                "TIMEOUT".red().bold()
            );
        }
    }
}

/// Draw ASCII line graph at the end
fn draw_line_graph(results: &[PingResult]) {
    let times: Vec<f64> = results.iter()
        .filter_map(|r| r.rtt_ms)
        .collect();
    
    if times.is_empty() {
        println!("{}", "No data to graph".red());
        return;
    }

    let max_rtt = times.iter().cloned().fold(0.0_f64, f64::max);
    let min_rtt = times.iter().cloned().fold(f64::MAX, f64::min);
    let height = 10;
    let width = results.len().min(60);
    
    println!("\n{}", "╔════════════════════════════════════════════════════════════╗".cyan());
    println!("{}", "║              📈 LATENCY GRAPH OVER TIME                     ║".cyan());
    println!("{}", "╚════════════════════════════════════════════════════════════╝".cyan());
    
    // Create matrix for the graph
    let mut graph: Vec<Vec<char>> = vec![vec![' '; width]; height];
    
    // Fill the graph
    for (i, result) in results.iter().enumerate().take(width) {
        if let Some(rtt) = result.rtt_ms {
            let normalized = if max_rtt > min_rtt {
                ((rtt - min_rtt) / (max_rtt - min_rtt) * (height - 1) as f64) as usize
            } else {
                height / 2
            };
            let row = height - 1 - normalized.min(height - 1);
            graph[row][i] = '●';
            
            // Fill downward with line
            for r in (row + 1)..height {
                if graph[r][i] == ' ' {
                    graph[r][i] = '│';
                }
            }
        } else {
            // Timeout - mark with X at the bottom
            graph[height - 1][i] = '✗';
        }
    }
    
    // Print graph with axes
    for (i, row) in graph.iter().enumerate() {
        let y_value = max_rtt - (i as f64 / (height - 1) as f64) * (max_rtt - min_rtt);
        let y_label = format!("{:>6.1}ms", y_value);
        
        let line: String = row.iter().collect();
        let colored_line = if i < height / 3 {
            line.red()
        } else if i < 2 * height / 3 {
            line.yellow()
        } else {
            line.green()
        };
        
        if i == 0 {
            println!("  {} ┤{}", y_label.dimmed(), colored_line);
        } else if i == height - 1 {
            println!("  {} ┤{}", y_label.dimmed(), colored_line);
        } else {
            println!("  {} │{}", y_label.dimmed(), colored_line);
        }
    }
    
    // X axis
    println!("         └{}", "─".repeat(width));
    
    // X axis labels
    let x_labels: String = (0..width)
        .map(|i| if i % 5 == 0 { format!("{}", i % 10) } else { " ".to_string() })
        .collect();
    println!("          {}", x_labels.dimmed());
    println!("          {}", "seq ->".dimmed());
}

/// Show latency distribution histogram
fn draw_histogram(times: &[f64]) {
    if times.is_empty() {
        return;
    }
    
    println!("\n{}", "╔════════════════════════════════════════════════════════════╗".magenta());
    println!("{}", "║               📊 LATENCY DISTRIBUTION                       ║".magenta());
    println!("{}", "╚════════════════════════════════════════════════════════════╝".magenta());
    
    // Create buckets
    let buckets = [
        (0.0, 10.0, "  0-10ms"),
        (10.0, 20.0, " 10-20ms"),
        (20.0, 50.0, " 20-50ms"),
        (50.0, 100.0, "50-100ms"),
        (100.0, f64::MAX, "  >100ms"),
    ];
    
    let total = times.len();
    
    for (min, max, label) in buckets.iter() {
        let count = times.iter().filter(|&&t| t >= *min && t < *max).count();
        let percentage = (count as f64 / total as f64) * 100.0;
        let bar_len = (percentage / 2.0) as usize;
        
        let bar = "█".repeat(bar_len);
        let colored_bar = if *max <= 20.0 {
            bar.green()
        } else if *max <= 50.0 {
            bar.yellow()
        } else {
            bar.red()
        };
        
        println!(
            "  {} │{:<50} {:>3} ({:>5.1}%)",
            label.cyan(),
            colored_bar,
            count,
            percentage
        );
    }
}

/// Print color legend
fn print_legend() {
    println!("\n  {} {} {} {} {} {} {}",
        "Legend:".dimmed(),
        "●".green(), "<20ms".green(),
        "●".yellow(), "20-50ms".yellow(),
        "●".red(), ">50ms".red()
    );
}

fn calculate_statistics(times: &[f64], total: u32) -> PingStatistics {
    let successful = times.len() as u32;
    let failed = total - successful;
    
    if times.is_empty() {
        return PingStatistics {
            min_ms: None,
            max_ms: None,
            avg_ms: None,
            std_dev_ms: None,
            packets_sent: total,
            packets_received: successful,
            packets_lost: failed,
            packet_loss_percent: 100.0,
        };
    }
    
    let min = times.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = times.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let avg: f64 = times.iter().sum::<f64>() / times.len() as f64;
    
    let variance: f64 = times.iter()
        .map(|t| (t - avg).powi(2))
        .sum::<f64>() / times.len() as f64;
    let std_dev = variance.sqrt();
    
    PingStatistics {
        min_ms: Some((min * 100.0).round() / 100.0),
        max_ms: Some((max * 100.0).round() / 100.0),
        avg_ms: Some((avg * 100.0).round() / 100.0),
        std_dev_ms: Some((std_dev * 100.0).round() / 100.0),
        packets_sent: total,
        packets_received: successful,
        packets_lost: failed,
        packet_loss_percent: ((failed as f64 / total as f64) * 100.0 * 100.0).round() / 100.0,
    }
}

fn print_stats(times: &[f64], total: u32, successful: u32, addr: IpAddr) {
    let failed = total - successful;
    
    println!("\n{}", "╔════════════════════════════════════════════════════════════╗".blue());
    println!("{}", "║                      📋 STATISTICS                          ║".blue());
    println!("{}", "╚════════════════════════════════════════════════════════════╝".blue());
    
    println!("  Host: {}", addr.to_string().cyan());
    println!("  Packets: {} sent, {} received, {} lost ({:.1}%)",
        total.to_string().white(),
        successful.to_string().green(),
        failed.to_string().red(),
        (failed as f64 / total as f64) * 100.0
    );

    if !times.is_empty() {
        let min = times.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = times.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let avg: f64 = times.iter().sum::<f64>() / times.len() as f64;
        
        // Calculate standard deviation
        let variance: f64 = times.iter()
            .map(|t| (t - avg).powi(2))
            .sum::<f64>() / times.len() as f64;
        let std_dev = variance.sqrt();
        
        println!("\n  RTT:");
        println!("    Min: {}", format!("{:.2}ms", min).green());
        println!("    Avg: {}", format!("{:.2}ms", avg).yellow());
        println!("    Max: {}", format!("{:.2}ms", max).red());
        println!("    StdDev: {}", format!("{:.2}ms", std_dev).cyan());
    }
}

/// Export results to JSON file
fn export_json(
    report: &PingReport,
    filename: &str,
) -> Result<(), String> {
    let json = serde_json::to_string_pretty(report)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
    
    let mut file = File::create(filename)
        .map_err(|e| format!("Failed to create file '{}': {}", filename, e))?;
    
    file.write_all(json.as_bytes())
        .map_err(|e| format!("Failed to write to file '{}': {}", filename, e))?;
    
    println!("\n  {} Exported to JSON: {}", "✓".green(), filename.cyan());
    Ok(())
}

/// Export results to CSV file
fn export_csv(
    results: &[PingResult],
    stats: &PingStatistics,
    host: &str,
    addr: IpAddr,
    filename: &str,
) -> Result<(), String> {
    let mut file = File::create(filename)
        .map_err(|e| format!("Failed to create file '{}': {}", filename, e))?;
    
    // Write header
    writeln!(file, "# Ping Report")
        .map_err(|e| format!("Failed to write to file: {}", e))?;
    writeln!(file, "# Host: {}", host)
        .map_err(|e| format!("Failed to write to file: {}", e))?;
    writeln!(file, "# IP: {}", addr)
        .map_err(|e| format!("Failed to write to file: {}", e))?;
    writeln!(file, "# Generated: {}", Local::now().format("%Y-%m-%d %H:%M:%S"))
        .map_err(|e| format!("Failed to write to file: {}", e))?;
    writeln!(file, "#")
        .map_err(|e| format!("Failed to write to file: {}", e))?;
    
    // Write column headers
    writeln!(file, "seq,rtt_ms,success,timestamp")
        .map_err(|e| format!("Failed to write to file: {}", e))?;
    
    // Write data rows
    for result in results {
        let rtt_str = result.rtt_ms.map_or("".to_string(), |r| format!("{:.2}", r));
        let timestamp = result.timestamp.clone().unwrap_or_default();
        writeln!(
            file,
            "{},{},{},{}",
            result.seq,
            rtt_str,
            result.success,
            timestamp
        ).map_err(|e| format!("Failed to write to file: {}", e))?;
    }
    
    // Write statistics section
    writeln!(file, "\n# Statistics")
        .map_err(|e| format!("Failed to write to file: {}", e))?;
    writeln!(file, "# packets_sent,packets_received,packets_lost,loss_percent,min_ms,avg_ms,max_ms,std_dev_ms")
        .map_err(|e| format!("Failed to write to file: {}", e))?;
    writeln!(
        file,
        "{},{},{},{:.2},{},{},{},{}",
        stats.packets_sent,
        stats.packets_received,
        stats.packets_lost,
        stats.packet_loss_percent,
        stats.min_ms.map_or("".to_string(), |v| format!("{:.2}", v)),
        stats.avg_ms.map_or("".to_string(), |v| format!("{:.2}", v)),
        stats.max_ms.map_or("".to_string(), |v| format!("{:.2}", v)),
        stats.std_dev_ms.map_or("".to_string(), |v| format!("{:.2}", v)),
    ).map_err(|e| format!("Failed to write to file: {}", e))?;
    
    println!("  {} Exported to CSV: {}", "✓".green(), filename.cyan());
    Ok(())
}

fn ping(
    host: &str,
    addr: IpAddr,
    count: u32,
    timeout: Duration,
    show_graph: bool,
    show_line: bool,
    json_file: Option<String>,
    csv_file: Option<String>,
) -> Result<(), String> {
    let protocol = Layer4(Ipv4(IpNextHeaderProtocols::Icmp));
    
    let (mut tx, mut rx) = transport_channel(1024, protocol)
        .map_err(|e| format!("Error creating channel (root permissions?): {}", e))?;

    let mut rx_iter = icmp_packet_iter(&mut rx);
    let identifier = std::process::id() as u16;
    
    let mut results: Vec<PingResult> = Vec::new();
    let mut times: Vec<f64> = Vec::new();
    
    // Initial estimate for bar max
    let mut max_rtt_estimate = 50.0_f64;
    
    let timestamp_start: DateTime<Local> = Local::now();

    // Header
    println!("\n{}", "╔════════════════════════════════════════════════════════════╗".cyan());
    println!("{}       PING {} - {} packets                {}",
        "║".cyan(),
        addr.to_string().yellow().bold(),
        count.to_string().green(),
        "║".cyan()
    );
    println!("{}", "╚════════════════════════════════════════════════════════════╝".cyan());
    
    if show_graph {
        print_legend();
        println!();
    }

    for seq in 0..count {
        let packet = create_icmp_packet(seq as u16, identifier);
        let start = Instant::now();
        let ping_timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();

        if let Err(e) = tx.send_to(
            pnet::packet::icmp::IcmpPacket::new(&packet).unwrap(),
            addr,
        ) {
            println!("  {} Send error: {}", "✗".red(), e);
            results.push(PingResult {
                seq,
                rtt_ms: None,
                success: false,
                timestamp: Some(ping_timestamp),
            });
            continue;
        }

        match rx_iter.next_with_timeout(timeout) {
            Ok(Some((_, reply_addr))) => {
                let rtt = start.elapsed().as_secs_f64() * 1000.0;
                let rtt_rounded = (rtt * 100.0).round() / 100.0;
                times.push(rtt);
                results.push(PingResult {
                    seq,
                    rtt_ms: Some(rtt_rounded),
                    success: true,
                    timestamp: Some(ping_timestamp),
                });
                
                // Update max estimate
                max_rtt_estimate = max_rtt_estimate.max(rtt * 1.2);
                
                if show_graph {
                    print_with_bar(seq, Some(rtt), max_rtt_estimate, reply_addr);
                } else {
                    println!(
                        "  {} Reply from {}: seq={} time={}",
                        "✓".green(),
                        reply_addr,
                        seq,
                        get_latency_color(rtt)
                    );
                }
            }
            Ok(None) => {
                results.push(PingResult {
                    seq,
                    rtt_ms: None,
                    success: false,
                    timestamp: Some(ping_timestamp),
                });
                if show_graph {
                    print_with_bar(seq, None, max_rtt_estimate, addr);
                } else {
                    println!("  {} Timeout for seq={}", "✗".red(), seq);
                }
            }
            Err(e) => {
                results.push(PingResult {
                    seq,
                    rtt_ms: None,
                    success: false,
                    timestamp: Some(ping_timestamp),
                });
                println!("  {} Error: {}", "✗".red(), e);
            }
        }

        if seq < count - 1 {
            std::thread::sleep(Duration::from_secs(1));
        }
    }

    let timestamp_end: DateTime<Local> = Local::now();
    
    // Statistics
    let successful = times.len() as u32;
    print_stats(&times, count, successful, addr);
    
    // Line graph
    if show_line && !results.is_empty() {
        draw_line_graph(&results);
    }
    
    // Histogram
    if (show_graph || show_line) && !times.is_empty() {
        draw_histogram(&times);
    }

    // Calculate statistics for export
    let stats = calculate_statistics(&times, count);
    
    // Export section header
    if json_file.is_some() || csv_file.is_some() {
        println!("\n{}", "╔════════════════════════════════════════════════════════════╗".yellow());
        println!("{}", "║                    📁 EXPORT RESULTS                        ║".yellow());
        println!("{}", "╚════════════════════════════════════════════════════════════╝".yellow());
    }
    
    // JSON export
    if let Some(filename) = json_file {
        let report = PingReport {
            host: host.to_string(),
            ip_address: addr.to_string(),
            timestamp_start: timestamp_start.format("%Y-%m-%d %H:%M:%S").to_string(),
            timestamp_end: timestamp_end.format("%Y-%m-%d %H:%M:%S").to_string(),
            timeout_seconds: timeout.as_secs(),
            results: results.clone(),
            statistics: calculate_statistics(&times, count),
        };
        export_json(&report, &filename)?;
    }
    
    // CSV export
    if let Some(filename) = csv_file {
        export_csv(&results, &stats, host, addr, &filename)?;
    }

    Ok(())
}

fn main() {
    let args = Args::parse();

    let addr: IpAddr = match args.host.parse() {
        Ok(ip) => ip,
        Err(_) => {
            use std::net::ToSocketAddrs;
            match (args.host.as_str(), 0).to_socket_addrs() {
                Ok(mut addrs) => match addrs.next() {
                    Some(socket_addr) => socket_addr.ip(),
                    None => {
                        eprintln!("{} Could not resolve: {}", "Error:".red(), args.host);
                        return;
                    }
                },
                Err(e) => {
                    eprintln!("{} DNS error: {}", "Error:".red(), e);
                    return;
                }
            }
        }
    };

    let timeout = Duration::from_secs(args.timeout);
    
    if let Err(e) = ping(
        &args.host,
        addr,
        args.count,
        timeout,
        args.graph,
        args.line_graph,
        args.json,
        args.csv,
    ) {
        eprintln!("{} {}", "Error:".red(), e);
    }
}
