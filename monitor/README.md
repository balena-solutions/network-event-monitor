# Network Event Monitor - Rust Library

A robust Rust library for monitoring network interface state changes via dbus and storing them in SQLite. Designed specifically for Balena IoT edge devices with unreliable connectivity.

## Overview

This library provides a clean, thread-safe API for:
- **Listening to network events** via dbus signals
- **Storing events persistently** in SQLite with timestamps
- **Automatic offloading** via callbacks when network connection is restored
- **Direct database access** for custom event management

## Features

- **DBus Integration**: Listens to network interface signals on the system dbus
- **SQLite Storage**: Persists events with timestamps in a local SQLite database
- **Event Offloading**: Automatic callback triggering when specific actions occur (e.g., connection restored)
- **Thread-Safe**: Designed with concurrent access in mind using `Arc<Mutex<>>`
- **Flexible Configuration**: Customize dbus interface, member signals, and offload triggers
- **Zero Dependencies Bloat**: Only depends on chrono, dbus, and rusqlite

## Installation & Usage

### As a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
monitor = { git = "https://github.com/balena-solutions/network-event-monitor" }
```

Basic example:

```rust
use monitor::{MonitorConfig, EventStore, start_monitor};
use std::sync::{Arc, Mutex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create configuration
    let config = MonitorConfig {
        db_path: "/app/data/network_events.db".to_string(),
        dbus_interface: "com.example.NetworkEvent".to_string(),
        dbus_member: "InterfaceAction".to_string(),
        offload_actions: vec!["up".to_string(), "vpn-up".to_string()],
    };

    // Create callback for when events should be offloaded
    let callback = Arc::new(Mutex::new(|events: Vec<(String, String, String)>| {
        for (interface, action, timestamp) in events {
            println!("Offloading event: {} -> {} at {}", interface, action, timestamp);
            // Send to remote server, write to log, etc.
        }
    }));

    // Start the monitor
    let token = start_monitor(config, callback)?;

    // Keep the token alive - the monitor runs in the background
    token.run()?;

    Ok(())
}
```

### Direct Database Access

You can also use the `EventStore` directly without the dbus listener:

```rust
use monitor::EventStore;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = EventStore::new("/path/to/db.db")?;
    let store_lock = store.lock().unwrap();

    // Record an event
    store_lock.record_event("eth0", "up")?;

    // Query events without clearing
    let events = store_lock.get_events()?;

    // Get events and clear atomically
    let events = store_lock.get_and_clear_events()?;

    // Clear without reading
    store_lock.clear_events()?;

    Ok(())
}
```

## API Reference

### `MonitorConfig`

Configuration struct for the monitor:

```rust
pub struct MonitorConfig {
    pub db_path: String,                  // Path to SQLite database
    pub dbus_interface: String,           // DBus interface to listen to
    pub dbus_member: String,              // DBus member signal name
    pub offload_actions: Vec<String>,     // Actions that trigger callbacks
}
```

### `EventStore`

Manage network events in SQLite:

```rust
// Create a new event store (creates DB if needed)
let store = EventStore::new("/path/to/db.db")?;

// Record an event
store.record_event("eth0", "up")?;

// Get all events without clearing
let events = store.get_events()?;

// Get all events and clear them atomically
let events = store.get_and_clear_events()?;

// Clear all events
store.clear_events()?;
```

### `start_monitor()`

Start listening for network events:

```rust
pub fn start_monitor(
    config: MonitorConfig,
    callback: Arc<Mutex<dyn Fn(Vec<(String, String, String)>) + Send>>
) -> Result<MonitorToken, Box<dyn std::error::Error>>
```

Returns a `MonitorToken` that keeps the listener alive while held.

### `MonitorToken`

Token representing an active listener:

```rust
// Process pending messages with timeout
token.process(Duration::from_secs(1))?;

// Block indefinitely processing messages
token.run()?;
```

## Database Schema

The SQLite database contains a single table:

```sql
CREATE TABLE network_events (
    id INTEGER PRIMARY KEY,
    interface TEXT NOT NULL,
    action TEXT NOT NULL,
    timestamp TEXT NOT NULL
);
```

Events are stored with ISO 8601 timestamps via `chrono`.

## Event Format

Events are tuples of `(interface, action, timestamp)`:
- `interface`: Network interface name (e.g., "eth0", "wlan0")
- `action`: Network state (e.g., "up", "down", "vpn-up")
- `timestamp`: RFC3339 formatted timestamp

## Building

```bash
cargo build --release --lib
```

## Testing

Run the test suite:

```bash
cargo test --lib
```

Tests include:
- Event store creation and initialization
- Event recording and retrieval
- Atomic event clearing
- Multi-event handling

## System Requirements

- Rust 1.70+
- Access to system dbus (typically `/var/run/dbus/system_bus_socket`)
- SQLite 3.x
- libdbus development headers (for building)
- SQLite development headers (for building)

## License

MIT

## Contributing

Contributions welcome! Please ensure:
- Code compiles with `cargo build --release --lib`
- Tests pass with `cargo test --lib`
- Code follows Rust conventions
