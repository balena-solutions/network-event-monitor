# Security Warning

This is a proof-of-concept that you can use as a starting point to build your own monitoring system for Network Manager on balenaOS. 

Make sure you understand container and balenaos security architecture before going further.

The balenaos host dbus interface is powerful and breaks the isolation of your container.

**Adding the `io.balena.features.dbus: '1'` significantly lower the security of your container, treat your container as if it was running `privileged` and had full root access to the host.**

It's recommended to only give that level of access to containers that have a single, well-defined purpose, very limited interfaces and tighly-controlled lifecycle.

# Balena network-event-monitor

This project demonstrates how to track changes in network connectivity using the `/dispatcher.d/` NetworkManager hooks, as documented here: https://docs.balena.io/reference/OS/network/#networkmanager-user-scripts

This is a perfect example of how to track network state changes on edge devices with unreliable connectivity (e.g. highly populated areas, or remote mountainous regions) and offload data via an appropriate method.

You can either use this example as a rust library, or as an example of a container you might build.

## Boot Partition Scripts (NetworkManager Dispatcher)

This project includes a NetworkManager dispatcher script in the `boot_partition/` directory that handles network event signaling. According to [Balena's NetworkManager documentation](https://docs.balena.io/reference/OS/network/#networkmanager-user-scripts), dispatcher scripts must be placed in the boot partition's `/dispatcher.d/` directory.

**To deploy the script:**

1. Download your balenaOS image from the Balena Dashboard
2. Mount the `resin-boot` partition of the `.img` file
3. Copy the script into the `/dispatcher.d/` directory (create the `/dispatcher.d/` directory if it doesn't yet exist):
4. Flash the modified image to your device

**How it works:**

During device boot, Balena copies all scripts from `/dispatcher.d/` on the boot partition into `/etc/NetworkManager/dispatcher.d` on the running device.

NetworkManager then executes these scripts when network events occur (interface up/down, connection changes, etc.).

The `50-event-emit.sh` script fires events that the monitor library listens to via dbus, allowing the event tracking and offloading mechanism to function properly.

## Monitor Usage

### 1. As a Rust Library

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
    // Call token.process() periodically or token.run() to block
    token.run()?;

    Ok(())
}
```

### 2. As a Balena Project


**Setup a Balena project:**

1. Create a Balena application at [balena.io](https://balena.io)
2. Clone this repository and add your Balena remote:

```bash
git clone https://github.com/balena-solutions/network-event-monitor
cd network-event-monitor
```

3. Deploy using the Balena CLI:

```bash
balena push <app-name>
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

## Architecture

```
┌─────────────────────────────────────────┐
│    External Application                 │
│  (uses monitor as library)              │
└────────────┬────────────────────────────┘
             │
    ┌────────▼────────┐
    │ MonitorToken    │
    │ (listener mgmt) │
    └────────┬────────┘
             │
    ┌────────▼────────────┐
    │ DBus Connection      │
    │ (signal listening)   │
    └────────┬────────────┘
             │
    ┌────────▼─────────────────┐
    │ Event Callback Triggered │
    │ (on offload_actions)     │
    └────────┬─────────────────┘
             │
    ┌────────▼────────────────┐
    │ EventStore              │
    │ (SQLite persistence)    │
    └─────────────────────────┘
```

## Testing

Run the test suite:

```bash
cd container
cargo test --lib
```

Tests include:
- Event store creation and initialization
- Event recording and retrieval
- Atomic event clearing
- Multi-event handling

## System Requirements

### For Library Usage
- Rust 1.70+
- Access to system dbus (typically `/var/run/dbus/system_bus_socket`)
- SQLite 3.x

### For Container/Balena Deployment
- Balena-compatible device (Raspberry Pi, BeagleBone, etc.)
- Balena application created at [balena.io](https://balena.io)
- Balena CLI installed locally
- Named volumes support for persistent database storage

### For Development
- Rust toolchain with Cargo
- libdbus development headers (dbus-dev on Alpine, libdbus-1-dev on Debian)
- SQLite development headers (sqlite-dev on Alpine)

## Troubleshooting

### DBus Connection Fails
- Verify the dbus socket is accessible at the configured path
- Check dbus service is running: `systemctl status dbus`
- In containers, ensure socket is properly mounted

### SQLite Lock Errors
- Ensure only one process writes to the database at a time
- Check file permissions on the database directory

### Missing Signals
- Verify the dbus interface and member names match actual signals
- Use `dbus-monitor` to inspect available signals: `sudo dbus-monitor --system`

## License

MIT

## Contributing

Contributions welcome! Please ensure:
- Code compiles with `cargo build --release`
- Tests pass with `cargo test --lib`
- Code follows Rust conventions
