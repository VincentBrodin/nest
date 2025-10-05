# nest

**Your windows always find their way home.**

nest is an automatic window switcher for [Hyprland](https://github.com/hyprwm/Hyprland).  
It learns where you like your apps to live and ensures they always end up in the right workspace, no hunting, no moving, no friction.
Kind of like [zoxide](https://github.com/ajeetdsouza/zoxide), but for window switching.
## Features

- **Learns your habits** – remembers where you usually place apps.
- **Workspace aware** – keep your browser on workspace 2 and terminal on workspace 1, automatically.
- **Lightweight** – runs quietly in the background without slowing you down.

## Quick Demo

![demo](./assets/demo.gif)

##  Installation

### Cargo
```bash
cargo install hypr-nest
```

### From Source

```bash
git clone https://github.com/VincentBrodin/nest.git 
cd nest
cargo build --release
```

## Setup

Add this line to your Hyprland config:

### Cargo
```conf
exec-once = nest
```

### From Source
```conf
exec-once = /PATH/TO/nest/target/release/nest
```

On first run, nest will create a config directory at `~/.config/nest/` (by default using your env) with the following files:

- `config.toml` – configuration settings
- `output.txt` – program output/logs
- `storage.txt` – stored workspace data
    
## Configuration

Example `config.toml`:

```toml
tau = 3600.0  # Decay constant for learning: e^(-age/tau), where age is in seconds (default = 1h)  
buffer = 30  # Number of records to keep per program class  
save_frequency = 10  # Seconds between saves (no save if no changes)  
log_level = "INFO"  # OFF, ERROR, WARN, INFO, DEBUG, TRACE
ignore = []  # List of program classes to ignore (see storage.txt for the classes that are being tracked)
```
