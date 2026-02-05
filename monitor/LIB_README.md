# Network Event Monitor - Example Binary

This directory contains the library implementation and a working example binary that demonstrates how to use the network event monitor library.

## Building the Library

```bash
cargo build --release --lib
```

## Example Binary

The `examples/monitor.rs` file shows how to use the library in a real application:

```bash
cargo build --release --example monitor
./target/release/examples/monitor
```

This example:
1. Creates a `MonitorConfig` with default settings
2. Sets up a callback that prints events to stdout
3. Starts the monitor listening to dbus events
4. Runs indefinitely processing network events

## Configuration

The example uses these defaults:

```rust
MonitorConfig {
    db_path: "/app/data/network_events.db",
    dbus_interface: "com.example.NetworkEvent",
    dbus_member: "InterfaceAction",
    offload_actions: vec!["up", "vpn-up"],
}
```

Modify `examples/monitor.rs` to change these values.

## Using as a Library

See [README.md](README.md) for comprehensive library documentation.

Quick start:

```rust
use monitor::{MonitorConfig, start_monitor};
use std::sync::{Arc, Mutex};

let config = MonitorConfig::default();
let callback = Arc::new(Mutex::new(|events| {
    println!("Got {} events", events.len());
}));
let monitor = start_monitor(config, callback)?;
monitor.run()?;
```

## Environment Variables

The monitor respects `DBUS_SYSTEM_BUS_ADDRESS` for connecting to a custom dbus socket:

```bash
DBUS_SYSTEM_BUS_ADDRESS=unix:path=/custom/dbus/socket cargo run --example monitor
```

## Testing

Run all tests:

```bash
cargo test --lib
```

Tests validate:
- Event store creation
- Event recording and retrieval
- Atomic event clearing
- Multi-event handling

## Deployment

### On Balena Devices

Deploy this library to Balena via `docker-compose.yml`:

```bash
balena push <app-name>
```

The container automatically:
- Has access to dbus via the Balena runtime
- Mounts a persistent volume at `/app/data` for the SQLite database
- Executes the NetworkManager dispatcher script

### Standalone Use

For non-Balena Rust projects, add as a git dependency:

```toml
[dependencies]
monitor = { git = "https://github.com/balena-solutions/network-event-monitor" }
```

## Architecture

The library is designed around three main components:

1. **EventStore**: Thread-safe SQLite persistence layer
   - Methods: `record_event()`, `get_events()`, `get_and_clear_events()`, `clear_events()`
   - Wrapped in `Arc<Mutex<>>` for concurrent access

2. **MonitorToken**: Active dbus listener holder
   - Methods: `process()`, `run()`
   - Keeps the dbus connection alive while in scope

3. **start_monitor()**: Entry point
   - Takes `MonitorConfig` and event `EventCallback`
   - Returns `MonitorToken` or an error

## Thread Safety

All components are thread-safe:
- `EventStore` is wrapped in `Arc<Mutex<_>>`
- Callbacks execute within the dbus message loop
- Multiple threads can safely access the store via clones

## License

MIT

## Contributing

Contributions welcome! Please ensure:
- Code compiles: `cargo build --release`
- Tests pass: `cargo test --lib`
- Code follows Rust conventions
