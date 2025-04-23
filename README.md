# Rust TUI Cross-Chain Swap Demo

This repository demonstrates how to perform cross-chain swaps using the Garden API through a Terminal User Interface (TUI) built with Rust. The application provides an interactive way to understand and execute the swap process across different blockchains.

## Structure

### Core Components

- `main.rs` - Entry point for the TUI application
- `order_script.rs` - Example script demonstrating the flow of API calls required for executing a swap

### UI Components

- `ui/`
  - Display components for the terminal interface
  - Input handling and validation
  - Request body builders that transform user inputs into API requests

### Service Layer

- `service/`
  - `blockchain/`
    - Chain-specific implementations for different blockchains
    - Transaction construction and signing utilities
    - HTLC contract interaction logic
  - `garden/`
    - Rust types that mirror Garden API request/response formats
    - API client functions for all Garden endpoints
    - Response parsing and error handling

## Features

- Interactive terminal interface for executing and monitoring cross-chain swaps
- Support for Bitcoin redemption through manual transaction building or gasless relayer
- Real-time swap status monitoring

## Environment Variables

The application requires the following environment variables to be set:

- `PRIV_KEY` - Private key for EVM-compatible chains
- `BTC_PRIV_KEY` - Bitcoin private key for Bitcoin transactions

You can set these in your shell before running the application:

```bash
export PRIV_KEY="your_private_key_here"
export BTC_PRIV_KEY="your_bitcoin_private_key_here"
```

## Installation

1. Clone this repository
```bash
git clone <repository-url>
cd <repository-name>
```

2. Build the application
```bash
cargo build --release
```

## Configuration

The application requires a configuration file (e.g., `config.json`) that specifies API endpoints, network settings, and other parameters which is included.

## Usage

### TUI Application

Run the application with:
```bash
cargo run --bin garden_tui -- -c config.json -n localnet
```

Parameters:
- `-c, --config`: Path to the configuration file
- `-n, --network`: Network to connect to (as defined in your config)

Navigate the interface using the keyboard shortcuts displayed at the bottom of each screen to:
1. Configure swap parameters
2. Initialize cross-chain swaps
3. Monitor ongoing swap status
4. Complete redemption or refund processes

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.