# Ethereum Contracts & Aurora Integration

This module provides the Ethereum-compatible smart contracts and Aurora EVM integration for the NEAR Smart Vault project.

## Features

### 1. Aurora EVM Integration
- Seamless integration with Aurora's EVM environment
- Contract deployment and interaction
- Gas estimation and optimization
- Cross-chain address conversion

### 2. DeFi Protocol Integration
- Aave lending/borrowing
- Uniswap swaps and liquidity provision
- Curve stable swaps
- Balancer pools
- Extensible interface for adding new protocols

### 3. Rainbow Bridge Integration
- NEAR ↔ Aurora token transfers
- Transfer status tracking
- Proof verification
- Bridge statistics and monitoring

## Architecture

```
src/
├── lib.rs           # Main Aurora integration
├── protocols.rs     # DeFi protocol implementations
├── bridge.rs        # Rainbow Bridge integration
└── utils.rs         # Helper functions
```

## Setup & Usage

1. Install Dependencies:
```bash
cargo build
```

2. Configure Aurora Integration:
```rust
let config = CrossChainConfig {
    aurora_endpoint: "https://testnet.aurora.dev",
    ethereum_rpc: "https://eth-goerli.alchemyapi.io/v2/your-key",
    bridge_contract: "bridge.testnet.near",
    gas_limit: U256::from(1_000_000),
    protocols: vec![/* protocol configs */],
};

let integration = AuroraIntegration::new(config);
```

3. Interact with DeFi Protocols:
```rust
// Aave example
let aave = AaveProtocol::new(lending_pool_address, rpc_url);
aave.deposit(token_address, amount).await?;

// Uniswap example
let uniswap = UniswapProtocol::new(router_address, factory_address, rpc_url);
uniswap.swap(swap_params).await?;
```

4. Use Rainbow Bridge:
```rust
let bridge_config = BridgeConfig {
    near_token_bridge: "bridge.testnet.near",
    aurora_token_bridge: aurora_bridge_address,
    eth_locker: eth_locker_address,
    confirmation_blocks: 12,
    max_transfer_amount: max_amount,
};

let bridge = RainbowBridge::new(bridge_config);
bridge.transfer_to_aurora(token_id, amount, recipient).await?;
```

## Security Considerations

1. Gas Optimization
- Batch transactions when possible
- Use Aurora's low-cost transactions
- Implement gas estimation for all operations

2. Bridge Safety
- Maximum transfer limits
- Proof verification
- Status monitoring
- Automatic retry mechanisms

3. Protocol Safety
- Slippage protection
- Transaction timeout handling
- Emergency withdrawal mechanisms

## Testing

Run the test suite:
```bash
cargo test
```

## Contributing

1. Fork the repository
2. Create your feature branch
3. Add tests for new functionality
4. Ensure all tests pass
5. Submit a pull request

## License

MIT License 