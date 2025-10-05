# Contributing

Thanks for your interest in improving nest.
Here’s how how you can set it up for development.

## Setup

Clone the repository:

```bash
git clone https://github.com/VincentBrodin/nest.git
cd nest
```

Before starting, stop any running instances:

```bash
killall nest
```

In your `~/.config/nest/config.toml`, set the log level to TRACE for detailed output:
```toml
log_level = "TRACE"
```

## Build and run
**Build in debug mode**:
```bash
cargo build
```

**Run directly**:
```bash
cargo run
```

**Build for release**:
```bash
cargo build --release
```
## Linting and formatting
Format and lint before committing:
```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
```

If you open a pull request, target the main branch and describe the change briefly.
