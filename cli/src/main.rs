use clap::{Parser, Subcommand, Args};
use near_sdk::json_types::U128;
use serde_json::Value;
use std::path::PathBuf;
use tokio;
//use colored::*;
use prettytable::{Table, Row, Cell};
use rust_decimal::Decimal;
use log::info;

mod commands;
use commands::{vault, bridge, oracle, monitoring};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    #[arg(short, long, value_name = "NETWORK")]
    network: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Vault management commands
    Vault {
        #[command(subcommand)]
        command: VaultCommands,
    },
    /// Bridge operations
    Bridge {
        #[command(subcommand)]
        command: BridgeCommands,
    },
    /// Oracle interactions
    Oracle {
        #[command(subcommand)]
        command: OracleCommands,
    },
    /// Monitoring and analytics
    Monitor {
        #[command(subcommand)]
        command: MonitorCommands,
    },
}

#[derive(Subcommand)]
enum VaultCommands {
    /// Deposit funds into the vault
    Deposit {
        #[arg(long)]
        amount: String,
    },
    /// Withdraw funds from the vault
    Withdraw {
        #[arg(long)]
        amount: String,
    },
}

#[derive(Subcommand)]
enum BridgeCommands {
    /// Initiate a cross-chain transfer
    Transfer {
        #[arg(long)]
        amount: String,
        #[arg(long)]
        to_chain: String,
    },
    /// Check transfer status
    Status {
        #[arg(long)]
        tx_hash: String,
    },
}

#[derive(Subcommand)]
enum OracleCommands {
    /// Get price data
    Price {
        #[arg(long)]
        token: String,
    },
    /// Get protocol TVL
    Tvl {
        #[arg(long)]
        protocol: String,
    },
}

#[derive(Subcommand)]
enum MonitorCommands {
    /// View analytics
    Analytics {
        #[arg(long)]
        type_: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let cli = Cli::parse();

    // Load configuration if provided
    let config = if let Some(config_path) = &cli.config {
        Some(load_config(config_path)?)
    } else {
        None
    };

    // Set up network configuration
    let network = cli.network.unwrap_or_else(|| String::from("mainnet"));

    match &cli.command {
        Commands::Vault { command } => {
            match command {
                VaultCommands::Deposit { amount } => {
                    info!("Depositing {} into vault", amount);
                    let amount = parse_amount(amount)?;
                    vault::deposit(U128(amount)).await?
                        .map_err(|e| format!("Deposit failed: {}", e))?;
                }
                VaultCommands::Withdraw { amount } => {
                    info!("Withdrawing {} from vault", amount);
                    let amount = parse_amount(amount)?;
                    vault::withdraw(U128(amount)).await?
                        .map_err(|e| format!("Withdrawal failed: {}", e))?;
                }
            }
        }
        Commands::Bridge { command } => {
            match command {
                BridgeCommands::Transfer { amount, to_chain } => {
                    info!("Transferring {} to {}", amount, to_chain);
                    bridge::transfer(amount.parse()?, to_chain).await?;
                }
                BridgeCommands::Status { tx_hash } => {
                    info!("Checking status of transfer {}", tx_hash);
                    bridge::check_status(tx_hash).await?;
                }
            }
        }
        Commands::Oracle { command } => {
            match command {
                OracleCommands::Price { token } => {
                    info!("Getting price for {}", token);
                    oracle::get_price(token).await?;
                }
                OracleCommands::Tvl { protocol } => {
                    info!("Getting TVL for {}", protocol);
                    oracle::get_tvl(protocol).await?
                        .map_err(|e| format!("Failed to get TVL: {}", e))?;
                }
            }
        }
        Commands::Monitor { command } => {
            match command {
                MonitorCommands::Analytics { type_ } => {
                    info!("Viewing {} analytics", type_);
                    monitoring::view_events(type_).await?;
                }
            }
        }
    }

    Ok(())
}

/// Parse amount string into a numeric value
fn parse_amount(amount: &str) -> Result<u128, Box<dyn std::error::Error>> {
    amount.trim()
        .replace(',', "")
        .parse::<u128>()
        .map_err(|e| format!("Invalid amount format: {}", e).into())
}

/// Load configuration from file
fn load_config(path: &PathBuf) -> Result<Value, Box<dyn std::error::Error>> {
    let config_str = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;
    serde_json::from_str(&config_str)
        .map_err(|e| format!("Invalid config format: {}", e).into())
}

// Example usage:
/*
$ vault-cli --network testnet vault deposit 100
$ vault-cli bridge transfer 50 aurora
$ vault-cli oracle price ETH
$ vault-cli monitor health
*/ 