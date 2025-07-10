# Collector

A Rust program that embeds and executes the `process` and `sslsniff` binaries from the agent-tracer project.

## Features

- Embeds both `/home/yunwei37/agent-tracer/src/process` and `/home/yunwei37/agent-tracer/src/sslsniff` binaries at compile time
- Extracts binaries to a temporary directory at runtime
- Makes them executable and runs them concurrently
- Automatically cleans up temporary files when the program exits

## Usage

### Development Build
```bash
cargo run
```

### Release Build
```bash
cargo build --release
./target/release/collector
```

The release binary (`target/release/collector`) is completely self-contained and can be distributed without the original binary files.

## How It Works

1. **Compile-time embedding**: The binaries are embedded into the Rust executable using `include_bytes!`
2. **Runtime extraction**: Creates a temporary directory and extracts both binaries
3. **Execution**: Makes the binaries executable and runs them concurrently using async tasks
4. **Cleanup**: Automatically removes temporary files when the program exits

## Binary Size

The release binary is approximately 10MB and includes both embedded binaries plus the Rust runtime.

## Requirements

- Rust 1.88.0 or later
- Linux (tested on Ubuntu/Debian)
- For the embedded binaries to work properly, you may need appropriate permissions for BPF operations 