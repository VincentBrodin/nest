# nest

**Your windows always find their way home.**

nest is an automatic window switcher for [Hyprland](https://github.com/hyprwm/Hyprland).  
It learns where you like your apps to live and ensures they always end up in the right workspace, no hunting, no moving, no friction.
Kind of like [zoxide](https://github.com/ajeetdsouza/zoxide), but for window switching.
## Features

- **Learns your habits** - remembers where you usually place apps.
- **Workspace aware** - keep your browser on workspace 2 and terminal on workspace 1, automatically.
- **Floating window restore** - remembers size, position, and state of floating windows and restores them seamlessly.
- **Lightweight** - runs quietly in the background without slowing you down.

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
save_frequency = 10 # Seconds between saves
log_level = "INFO" # OFF, ERROR, WARN, INFO, DEBUG, TRACE

[workspace]
buffer = 30 # Number of records nest will keep per program class
tau = 604800.0 # Decay constant for learning: e^(-age/tau), where age is in seconds (default = 1h)

[workspace.filter]
mode = "Exclude" # Include, Exclude
programs = [] # List of program classes you wish to either include or exclude

[floating]
frequency = 5 # How often nest will look for new floating windows

[floating.filter]
mode = "Include" # Include, Exclude
programs = [] # List of program classes you wish to either include or exclude
```
