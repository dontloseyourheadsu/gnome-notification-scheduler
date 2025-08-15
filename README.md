# Gnome Alert Scheduler
Scheduler that controls alerts on Gnome desktop to setup remainders through terminal.

## Features

- Schedule alerts with custom messages and intervals
- List all scheduled alerts
- Remove scheduled alerts

## Usage

```bash
# Schedule a new alert
gnome-alert-scheduler schedule --title "My Alert" --message "This is a test alert" --interval 60

# List all scheduled alerts
gnome-alert-scheduler list

# Remove a scheduled alert
gnome-alert-scheduler remove --id 1
```

## Tech Stack

- Rust
- Clap (Command Line Argument Parser)
- Shared Library for Common Functionality