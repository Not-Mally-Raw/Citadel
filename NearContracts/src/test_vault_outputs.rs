use near_sdk::{
    AccountId,
    NearToken,
    test_utils::{accounts, VMContextBuilder},
    testing_env,
};

use crate::{YieldVault, VaultStatus, Strategy, Balance};

const YOCTO_NEAR: Balance = 1_000_000_000_000_000_000_000_000;

fn setup_test_context(predecessor: AccountId, deposit: Balance) {
    let context = VMContextBuilder::new()
        .predecessor_account_id(predecessor)
        .attached_deposit(NearToken::from_yoctonear(deposit))
        .build();
    testing_env!(context);
}

fn print_separator() {
    println!("\n{}", "-".repeat(50));
}

pub fn demonstrate_vault_operations() {
    // Initialize accounts
    let owner = accounts(0);
    let user1 = accounts(1);
    let user2 = accounts(2);
    let treasury = AccountId::try_from("treasury.near".to_string()).unwrap();

    println!("🏦 NEAR Protocol Yield Vault Demonstration");
    print_separator();

    // Initialize vault
    setup_test_context(owner.clone(), 0);
    let mut vault = YieldVault::new(
        owner.clone(),
        treasury,
        None,
        None,
    );

    println!("✅ Vault Initialized");
    println!("Owner: {}", owner);
    println!("Initial TVL: {}", vault.get_tvl().0);
    print_separator();

    // Add strategies
    println!("📈 Adding Investment Strategies");
    setup_test_context(owner.clone(), 1);
    
    vault.add_strategy("defi_lending".to_string(), 3000);  // 30% allocation
    vault.add_strategy("staking_pool".to_string(), 4000);  // 40% allocation
    vault.add_strategy("liquidity_pool".to_string(), 3000); // 30% allocation

    println!("Strategies Added:");
    for (name, strategy) in vault.get_all_strategies() {
        println!("- {} (Max Allocation: {}%)", name, strategy.max_allocation_bps as f32 / 100.0);
    }
    print_separator();

    // User deposits
    println!("💰 Processing User Deposits");
    
    // User 1 deposits 10 NEAR
    setup_test_context(user1.clone(), 10 * YOCTO_NEAR);
    let shares1 = vault.deposit(None);
    
    // User 2 deposits 20 NEAR
    setup_test_context(user2.clone(), 20 * YOCTO_NEAR);
    let shares2 = vault.deposit(None);

    println!("User1 Deposit:");
    println!("- Address: {}", user1);
    println!("- Amount: 10 NEAR");
    println!("- Shares Received: {}", shares1.0);

    println!("\nUser2 Deposit:");
    println!("- Address: {}", user2);
    println!("- Amount: 20 NEAR");
    println!("- Shares Received: {}", shares2.0);

    println!("\nVault Status After Deposits:");
    println!("- Total TVL: {} NEAR", vault.get_tvl().0 as f64 / YOCTO_NEAR as f64);
    println!("- Share Price: {} NEAR", vault.get_share_price().0 as f64 / YOCTO_NEAR as f64);
    print_separator();

    // Simulate yield generation
    println!("🌱 Simulating Yield Generation");
    setup_test_context(owner.clone(), 0);
    
    // Update strategy allocations
    vault.update_strategy_allocation("defi_lending".to_string(), 3000);
    vault.update_strategy_allocation("staking_pool".to_string(), 4000);
    vault.update_strategy_allocation("liquidity_pool".to_string(), 3000);

    // Harvest yields
    vault.harvest_yield();

    println!("Vault Metrics After Yield:");
    let metrics = vault.get_vault_metrics();
    println!("- APY: {}%", metrics.annual_percentage_yield as f32 / 100.0);
    println!("- Total Profit: {} NEAR", metrics.total_profit as f64 / YOCTO_NEAR as f64);
    println!("- Total Users: {}", metrics.total_users);
    print_separator();

    // Display analytics
    println!("📊 Vault Analytics");
    let analytics = vault.get_analytics();
    
    println!("Performance Metrics:");
    println!("- Current APY: {}%", analytics.performance_metrics.current_apy as f32 / 100.0);
    println!("- Average APY: {}%", analytics.performance_metrics.average_apy as f32 / 100.0);
    println!("- Best Strategy: {}", analytics.performance_metrics.best_strategy.unwrap_or_default());
    
    println!("\nRisk Metrics:");
    println!("- Risk Score: {}", analytics.risk_metrics.risk_score);
    println!("- Strategy Diversification: {}%", analytics.risk_metrics.strategy_diversification as f32 / 100.0);
    println!("- Sharpe Ratio: {:.2}", analytics.risk_metrics.sharpe_ratio);
    print_separator();

    // User positions
    println!("👤 User Positions");
    
    if let Some(position1) = vault.get_user_position(user1) {
        println!("\nUser1 Position:");
        println!("- Shares: {}", position1.shares);
        println!("- Deposited Amount: {} NEAR", position1.deposited_amount as f64 / YOCTO_NEAR as f64);
        println!("- Unclaimed Rewards: {} NEAR", position1.unclaimed_rewards as f64 / YOCTO_NEAR as f64);
    }

    if let Some(position2) = vault.get_user_position(user2) {
        println!("\nUser2 Position:");
        println!("- Shares: {}", position2.shares);
        println!("- Deposited Amount: {} NEAR", position2.deposited_amount as f64 / YOCTO_NEAR as f64);
        println!("- Unclaimed Rewards: {} NEAR", position2.unclaimed_rewards as f64 / YOCTO_NEAR as f64);
    }
    print_separator();

    println!("🏁 Demonstration Complete");
}

// Function to run the demonstration
pub fn run_demonstration() {
    println!("\nStarting Yield Vault Demonstration...\n");
    demonstrate_vault_operations();
}

#[test]
fn test_vault_outputs() {
    let treasury = AccountId::try_from("treasury.near".to_string()).unwrap();
    // ... existing code ...
} 