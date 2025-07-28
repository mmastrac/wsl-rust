# WSL API

A higher-level Rust API for interacting with Windows Subsystem for Linux (WSL) through COM.

## Overview

This crate provides a safe, high-level interface to the WSL COM API. It manages COM initialization in a background thread and provides a clean API for common WSL operations.

## Architecture

The API uses a background thread pattern:

1. **Background COM Thread**: A dedicated thread that initializes COM with apartment threading and maintains the WSL COM session
2. **Message Passing**: Communication between the main thread and COM thread via mpsc channels
3. **Safe Wrapper**: The `Wsl` struct provides a safe, high-level interface

## Usage

```rust
use wsl_api::Wsl;

fn main() -> windows::core::Result<()> {
    // Create a new WSL API instance
    let wsl = Wsl::new()?;
    
    // Use the API
    wsl.shutdown()?;
    
    Ok(())
}
```

## Features

- **Automatic COM Management**: COM is initialized and cleaned up automatically
- **Thread Safety**: All COM operations are performed on a dedicated thread
- **Error Handling**: Proper error propagation using Windows Result types
- **Resource Management**: Automatic cleanup when the `Wsl` instance is dropped

## Examples

See the `examples/` directory for usage examples:

```bash
cargo run --example basic_usage
```

## Dependencies

- `wsl-com-api`: Low-level COM interface bindings
- `windows`: Windows API bindings for COM functionality

## Requirements

- Windows 10/11 with WSL enabled
- Administrator privileges (for most WSL operations)
- Rust toolchain

## API Methods

### `Wsl::new() -> Result<Wsl>`
Creates a new WSL API instance with a background COM thread.

### `wsl.shutdown() -> Result<()>`
Shuts down all WSL instances.

### `wsl.get_default_distribution() -> Result<()>`
Gets the default WSL distribution (placeholder implementation).

## Thread Safety

The `Wsl` struct is designed to be thread-safe. You can clone the `Wsl` instance and use it from multiple threads, as all COM operations are serialized through the background thread.

## Error Handling

All methods return `windows::core::Result<T>` which provides detailed error information for COM operations. Check the return values and handle errors appropriately in your application.

## Notes

- The background thread will be joined when the `Wsl` instance is dropped
- COM will be automatically cleaned up in the background thread
- All WSL operations require administrator privileges
- The `shutdown()` method will terminate all running WSL instances 