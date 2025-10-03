# `nest`

Your windows always find their way home with `nest`.

`nest` is an automatic window switcher for [Hyprland]("https://github.com/hyprwm/Hyprland").
It learns where you like your apps to live and makes sure they always end up in the right workspace or monitor - no hunting, no dragging, no friction.

## Features

- **Learns your habits**:  `nest` remembers where you usually place apps.
- **Workspace aware**: Keep your browser on workspace 2, Discord on workspace 5, and terminal on workspace 1, automatically.
- **Real-time**: Reacts to new windows instantly via Hyprland’s IPC stream.
- **Lightweight**: Runs in the background without getting in your way.

## Install
### From Source
```bash
git clone https://github.com/VincentBrodin/nest.git
cd nest
cargo build --release
```

## Setup
Add this to your hyprland config
```conf
exec-once = /PATH/TO/nest/target/release/nest
```

The first time nest runs it will create files in your config directory (for most people it is `~/.config/nest/`)
In the nest directory you will find 3 files
- `config.toml`: In here you can set the settings `nest` will follow
- `output.txt`: This is the output/logs of the program
- `storage.txt`: In here your workspace data is stored

## Config
```toml
tau = 3600.0        # This is your decay constant nest uses e^(-age/tau) where age is the time ago in seconds so here the decay constant is an hour
buffer = 30         # This is how many records nest will keep per program class
save_frequency = 10 # How many seconds between each save (if no changes were made it will not save anything)
log_level = "INFO"  #  Possible values are: OFF, ERROR, WARN, INFO, DEBUG, TRACE
```
