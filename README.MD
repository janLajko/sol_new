# Solana Token Monitor

A Rust-based monitoring system for tracking new pump tokens on the Solana blockchain with specific market cap thresholds.

## Features

- Monitors newly created tokens on Solana
- Filters tokens by market cap
- Tracks tokens created within the last 10 minutes
- Redis-based caching system
- Twitter API integration for social monitoring

## Prerequisites

- Rust and Cargo
- Redis
- Twitter API credentials

## Installation

1. Install Redis:

```bash
# macOS (using Homebrew)
brew install redis

# Linux (Ubuntu/Debian)
sudo apt-get update
sudo apt-get install redis-server
```

2. Clone the repository:

```bash
git clone https://github.com/nick199910/sol_new.git
cd sol_new
```

3. Configure environment variables:
   - Copy `.env.template` to `.env`
   - Update the configuration values in `.env`

## Running the Application

1. Start Redis server:

```bash
redis-server
```

2. Run the application:

```bash
cargo run
```

## Configuration

The application can be configured through environment variables in the `.env` file:

- `GRPC_URL`: Solana GRPC endpoint
- `RPC_URL`: Solana RPC endpoint
- `REDIS_URL`: Redis connection URL
- `MARKET_CAP`: Minimum market cap threshold
- `CK`: Twitter Consumer Key
- `CS`: Twitter Consumer Secret

## License

MIT