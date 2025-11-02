<p align="center" style="margin-bottom: 0px !important;">
  <img width="200" src="./assets/nest_logo.png" alt="nest logo" align="center">
</p>

**Your windows always find their way home.**
nest is an intelligent window placement system for [Hyprland](https://github.com/hyprwm/Hyprland).
It learns where you like your apps to live and ensures they always open in the right workspace, no hunting, no dragging, no friction.
Think of it like [zoxide](https://github.com/ajeetdsouza/zoxide), but for your windows.


## Quick Demo

![demo](./assets/demo.gif)

## What makes nest special
Unlike traditional window rules, which are static (*you set them up once, and if your habits change, you have to update them yourself.*),
nest is dynamic, adaptive, and invisible.

It’s window rules on **steroids**: automatic, effortless, and always in sync with your habits.

**Here’s what sets it apart:**
- **Learns your habits** - nest observes where you place your apps and remembers it automatically. No manual rules, no configs.
- **Workspace aware** - your apps consistently open in the workspaces you expect (browser on 2, terminal on 1, etc.).
- **Floating window memory** - optionally restores the size, position, and state of floating windows.
- **Seamless experience** - runs quietly in the background; you should never notice it working.
- **Adaptive by design** - as your habits change, nest learns and adapts with you.
- **Lightweight** - minimal footprint, no unnecessary overhead, just smooth automation.

The goal of nest is to feel like an extension of your workflow, not another tool to manage.

## Growing with you
nest is constantly evolving.
Feedback isn’t just welcome, it’s part of the process.
Well-written ideas and suggestions often make it into the next release.

If you have feedback, open an issue or start a discussion - your input directly helps shape nest’s future.

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

On first run, nest will create a config directory at `~/.config/nest/` with the following files:

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
tau = 604800.0 # Decay constant for learning: e^(-age/tau), where age is in seconds (default = 1 week)

[workspace.filter]
mode = "Exclude" # Include, Exclude
programs = [] # List of program classes you wish to either include or exclude

[floating]
frequency = 5 # How often nest will look for new floating windows

[floating.filter]
mode = "Include" # Include, Exclude
programs = [] # List of program classes you wish to either include or exclude

[restore]
timeout = 120 # If a program closes before this timeout, you'll be returned to your previous workspace.

[restore.filter]
mode = "Include" # Include, Exclude
programs = [] # List of program classes you wish to either include or exclude
```
