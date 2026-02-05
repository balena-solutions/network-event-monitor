//! Example binary showing how to use the network event monitor library

use monitor::{MonitorConfig, EventCallback, start_monitor};
use std::sync::{Arc, Mutex};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get database path from command-line argument or use default
    let db_path = env::args()
        .nth(1)
        .unwrap_or_else(|| "/app/data/network_events.db".to_string());

    // Create configuration
    let config = MonitorConfig {
        db_path,
        ..Default::default()
    };

    // Set up the callback for connection restoration
    let callback: EventCallback = Arc::new(Mutex::new(|events: Vec<(String, String, String)>| {
        println!("Connection restored! Offloading {} events:", events.len());
        for (interface, action, timestamp) in events {
            println!("  [{}] {} on {}", timestamp, action, interface);
        }
    }));

    println!("Network Event Monitor started. Listening for dbus network events...");

    // Start the monitor
    let monitor = start_monitor(config, callback)?;

    // Block and continuously process incoming messages
    monitor.run()
}
