# Gnome Alert Scheduler

Scheduler that controls alerts on Gnome desktop to setup reminders through terminal.

## Features

- Schedule alerts with custom messages and intervals
- List all scheduled alerts with status (RUNNING/STOPPED)
- Update existing scheduled alerts
- Remove scheduled alerts completely
- Stop/Start individual alerts (pause without deletion)
- Desktop notifications using GNOME notification system
- Automatic timer management with resource cleanup

## Architecture

This project consists of three main components:

1. **job-runner**: A daemon service that runs in the background and executes scheduled alerts
2. **scheduler**: A CLI client that communicates with the daemon via Unix socket
3. **shared_lib**: Common data structures and functionality

## Usage

### Starting the Daemon

First, start the background daemon service:

```bash
# Build and run the daemon
cd job-runner
cargo run
```

The daemon will:

- Load existing schedules on startup
- Start timers for all non-stopped schedules
- Listen for client connections on `/tmp/gnome-alert-scheduler.sock`
- Execute alerts by showing desktop notifications
- Handle graceful shutdown with Ctrl+C

### Using the CLI Client

In another terminal, use the CLI client to manage alerts:

```bash
# Schedule a new alert (runs every 60 seconds)
cargo run --bin rust-schedule-alerts -- schedule --title "My Alert" --message "This is a test alert" --interval 60

# List all scheduled alerts (shows status: RUNNING or STOPPED)
cargo run --bin rust-schedule-alerts -- list

# Update an existing alert
cargo run --bin rust-schedule-alerts -- update 1 --title "Updated Title" --message "Updated message" --interval 120

# Stop an alert (pauses execution but keeps the alert)
cargo run --bin rust-schedule-alerts -- stop 1

# Start a previously stopped alert
cargo run --bin rust-schedule-alerts -- start 1

# Remove a scheduled alert completely
cargo run --bin rust-schedule-alerts -- remove 1
```

### Stop vs Remove

- **Stop**: Temporarily pauses an alert. The alert remains in the list with status "STOPPED" and its timer is destroyed to free resources. Can be resumed with `start`.
- **Remove**: Permanently deletes an alert. The alert is completely removed from the system and cannot be recovered.

## Alert Execution

- Alerts are executed by showing desktop notifications using the GNOME notification system
- Each notification displays for 6 seconds
- Alerts repeat at their specified interval (in seconds)
- Only alerts with status "RUNNING" will execute
- Timers start immediately when the daemon starts or when an alert is created/started
- Resource-efficient: stopped alerts have their timers destroyed to save memory

## Data Storage

Alert schedules are stored in:

- Linux: `~/.local/share/gnome-alert-scheduler/alert-schedules.json`

The data includes:

- ID: Unique identifier for each alert
- Title: Short summary shown in notification
- Message: Detailed text shown in notification
- Interval: Repeat interval in seconds
- Stopped: Boolean flag indicating if the alert is paused

## Tech Stack

- **Rust** - Systems programming language
- **Tokio** - Async runtime for handling concurrent operations
- **Clap** - Command Line Argument Parser
- **notify-rust** - Desktop notification library
- **serde** - Serialization/deserialization
- **Unix Sockets** - IPC between daemon and client
