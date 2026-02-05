//! Network Event Monitor Library
//! 
//! A library for monitoring network interface state changes via dbus and storing them
//! in SQLite with event offloading callbacks when connection is restored.

use chrono::Local;
use dbus::blocking::Connection as DbusConnection;
use rusqlite::{Connection as SqliteConnection, Result as SqliteResult, params};
use std::sync::{Arc, Mutex};

/// Configuration for the network event monitor
#[derive(Clone, Debug)]
pub struct MonitorConfig {
    /// Path to the SQLite database
    pub db_path: String,
    /// Dbus interface to listen to
    pub dbus_interface: String,
    /// Dbus member signal name
    pub dbus_member: String,
    /// Actions that trigger offload (e.g., "up", "vpn-up")
    pub offload_actions: Vec<String>,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            db_path: "/app/data/network_events.db".to_string(),
            dbus_interface: "com.example.NetworkEvent".to_string(),
            dbus_member: "InterfaceAction".to_string(),
            offload_actions: vec!["up".to_string(), "vpn-up".to_string()],
        }
    }
}

/// Callback type for handling offloaded events
pub type EventCallback = Arc<Mutex<dyn Fn(Vec<(String, String, String)>) + Send>>;

/// Event store for managing network events in SQLite
pub struct EventStore {
    db_path: String,
}

impl EventStore {
    /// Create a new event store, initializing the database if needed
    pub fn new(db_path: &str) -> SqliteResult<Arc<Mutex<Self>>> {
        let conn = SqliteConnection::open(db_path)?;

        // Create table if it doesn't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS network_events (
                id INTEGER PRIMARY KEY,
                interface TEXT NOT NULL,
                action TEXT NOT NULL,
                timestamp TEXT NOT NULL
            )",
            [],
        )?;

        Ok(Arc::new(Mutex::new(EventStore {
            db_path: db_path.to_string(),
        })))
    }

    /// Record a network event in the database
    pub fn record_event(&self, interface: &str, action: &str) -> SqliteResult<()> {
        let conn = SqliteConnection::open(&self.db_path)?;
        let timestamp = Local::now().to_rfc3339();

        conn.execute(
            "INSERT INTO network_events (interface, action, timestamp) VALUES (?1, ?2, ?3)",
            params![interface, action, timestamp],
        )?;

        Ok(())
    }

    /// Retrieve all events and clear the database
    pub fn get_and_clear_events(&self) -> SqliteResult<Vec<(String, String, String)>> {
        let conn = SqliteConnection::open(&self.db_path)?;

        // Read all events
        let mut stmt = conn
            .prepare("SELECT interface, action, timestamp FROM network_events ORDER BY id ASC")?;

        let events = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        // Clear the table
        conn.execute("DELETE FROM network_events", [])?;

        Ok(events)
    }

    /// Query all events without clearing them
    pub fn get_events(&self) -> SqliteResult<Vec<(String, String, String)>> {
        let conn = SqliteConnection::open(&self.db_path)?;

        // Read all events
        let mut stmt = conn
            .prepare("SELECT interface, action, timestamp FROM network_events ORDER BY id ASC")?;

        let events = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(events)
    }

    /// Clear all events from the database
    pub fn clear_events(&self) -> SqliteResult<()> {
        let conn = SqliteConnection::open(&self.db_path)?;
        conn.execute("DELETE FROM network_events", [])?;
        Ok(())
    }
}

/// Start the network event monitor with dbus listening
/// 
/// # Arguments
/// * `config` - Monitor configuration
/// * `callback` - Function to call when events are offloaded
/// 
/// # Returns
/// A result containing a token that keeps the listener alive while held
pub fn start_monitor(
    config: MonitorConfig,
    callback: EventCallback,
) -> Result<MonitorToken, Box<dyn std::error::Error>> {
    // Initialize event store
    let event_store = EventStore::new(&config.db_path)
        .map_err(|e| format!("Failed to create event store: {}", e))?;

    // Connect to dbus system bus
    let conn = DbusConnection::new_system()
        .map_err(|e| format!("Failed to connect to dbus: {}", e))?;

    // Build match rule for network event signals
    let rule = dbus::message::MatchRule::new()
        .with_interface(config.dbus_interface.clone())
        .with_member(config.dbus_member.clone());

    // Clone for use in callback
    let store = event_store.clone();
    let offload_actions = config.offload_actions.clone();

    // Register the match rule with a callback that processes incoming signals
    let _token = conn
        .add_match(
            rule,
            move |args: (String, String), _conn: &DbusConnection, _msg| {
                let (interface, action) = args;

                println!("Received event: interface={}, action={}", interface, action);

                // Record the event
                let store_lock = store.lock().unwrap();
                if let Err(e) = store_lock.record_event(&interface, &action) {
                    eprintln!("Failed to record event: {}", e);
                    return true;
                }

                // Check if action triggers offload
                if offload_actions.contains(&action) {
                    match store_lock.get_and_clear_events() {
                        Ok(events) => {
                            if !events.is_empty() {
                                if let Ok(cb) = callback.lock() {
                                    cb(events);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to retrieve events: {}", e);
                        }
                    }
                }

                true // Continue listening for more signals
            },
        )
        .map_err(|e| format!("Failed to add match rule: {}", e))?;

    Ok(MonitorToken {
        conn: Arc::new(Mutex::new(conn)),
        _token,
    })
}

/// Token that represents an active monitor listener
/// 
/// The monitor continues to run while this token is held.
pub struct MonitorToken {
    conn: Arc<Mutex<DbusConnection>>,
    _token: dbus::channel::Token,
}

impl MonitorToken {
    /// Process pending dbus messages with a timeout
    pub fn process(&self, timeout: std::time::Duration) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        conn.process(timeout)
            .map_err(|e| format!("Failed to process dbus messages: {}", e))?;
        Ok(())
    }

    /// Block and continuously process incoming messages
    /// 
    /// This will run indefinitely until the token is dropped or an error occurs
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            self.process(std::time::Duration::from_secs(1))?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_store_creation() {
        // Use a temporary file instead of :memory: for tests
        let store = EventStore::new("/tmp/test_monitor_1.db").unwrap();
        let store_lock = store.lock().unwrap();
        
        // Should not error
        store_lock.record_event("eth0", "up").unwrap();
        
        // Clean up
        let _ = std::fs::remove_file("/tmp/test_monitor_1.db");
    }

    #[test]
    fn test_event_store_retrieval() {
        let store = EventStore::new("/tmp/test_monitor_2.db").unwrap();
        let store_lock = store.lock().unwrap();
        
        store_lock.record_event("eth0", "up").unwrap();
        let events = store_lock.get_events().unwrap();
        
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, "eth0");
        assert_eq!(events[0].1, "up");
        
        // Clean up
        let _ = std::fs::remove_file("/tmp/test_monitor_2.db");
    }

    #[test]
    fn test_event_store_clear() {
        let store = EventStore::new("/tmp/test_monitor_3.db").unwrap();
        let store_lock = store.lock().unwrap();
        
        store_lock.record_event("eth0", "up").unwrap();
        let events = store_lock.get_and_clear_events().unwrap();
        
        assert_eq!(events.len(), 1);
        
        let events_after = store_lock.get_events().unwrap();
        assert_eq!(events_after.len(), 0);
        
        // Clean up
        let _ = std::fs::remove_file("/tmp/test_monitor_3.db");
    }

    #[test]
    fn test_event_store_multiple_events() {
        let store = EventStore::new("/tmp/test_monitor_4.db").unwrap();
        let store_lock = store.lock().unwrap();

        store_lock.record_event("eth0", "down").unwrap();
        store_lock.record_event("wlan0", "up").unwrap();
        store_lock.record_event("vpn0", "vpn-up").unwrap();

        let events = store_lock.get_events().unwrap();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].0, "eth0");
        assert_eq!(events[0].1, "down");
        assert_eq!(events[1].0, "wlan0");
        assert_eq!(events[1].1, "up");
        assert_eq!(events[2].0, "vpn0");
        assert_eq!(events[2].1, "vpn-up");

        // Clean up
        let _ = std::fs::remove_file("/tmp/test_monitor_4.db");
    }
}

