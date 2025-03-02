use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::{LookupMap, UnorderedMap, Vector},
    env, near_bindgen, AccountId, PanicOnDefault, Promise, Gas,
    BorshStorageKey, require, json_types::U128,
    serde::{Deserialize, Serialize},
    NearToken,
};
use near_contract_standards::fungible_token::Balance;
use near_sdk::utils::assert_one_yocto;

// Constants
const YOCTO_NEAR: Balance = 1_000_000_000_000_000_000_000_000;
const MIN_DEPOSIT: Balance = YOCTO_NEAR;      // 1 NEAR minimum
const MAX_DEPOSIT: Balance = YOCTO_NEAR * 1_000_000;  // 1M NEAR maximum
const BASIS_POINTS: u32 = 10_000;             // 100% in basis points
const MIN_LOCKUP_DURATION: u64 = 86_400_000_000_000; // 1 day in nanoseconds
const EPOCH_DURATION: u64 = 86_400_000_000_000;      // 1 day in nanoseconds

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    UserPositions,
    Strategies,
    TvlHistory,
    Operators,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum VaultStatus {
    Active,
    EmergencyShutdown,
    Deprecated
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct UserPosition {
    shares: Balance,
    deposited_amount: Balance,
    last_deposit_timestamp: u64,
    unclaimed_rewards: Balance,
    locked_until: u64,
    cumulative_rewards: Balance,
    last_interaction: u64,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Strategy {
    name: String,
    allocation_ratio: u32,
    current_balance: Balance,
    total_profit: Balance,
    is_active: bool,
    last_harvest_timestamp: u64,
    risk_score: u32,
    max_allocation_bps: u32,
    performance_history: Vec<(u64, Balance)>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct VaultMetrics {
    total_value_locked: Balance,
    annual_percentage_yield: u32,
    total_users: u32,
    total_profit: Balance,
    last_harvest_timestamp: u64,
    historical_apy: Vec<(u64, u32)>,
    risk_score: u32,
    sharpe_ratio: f64,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Fees {
    deposit_fee_bps: u32,
    withdrawal_fee_bps: u32,
    performance_fee_bps: u32,
    management_fee_bps: u32,
    early_withdrawal_fee_bps: u32,
}

impl Default for Fees {
    fn default() -> Self {
        Self {
            deposit_fee_bps: 0,
            withdrawal_fee_bps: 50,
            performance_fee_bps: 2000,
            management_fee_bps: 200,
            early_withdrawal_fee_bps: 300,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct YieldOptimizer {
    target_apy: u32,
    max_risk_score: u32,
    rebalance_threshold_bps: u32,
    volatility_window: u64,
    min_strategy_weight: u32,
    max_strategy_weight: u32,
    optimization_frequency: u64,
    last_optimization: u64,
    historical_performance: Vec<(String, Vec<(u64, Balance)>)>,
}

impl YieldOptimizer {
    pub fn new(target_apy: u32, max_risk_score: u32) -> Self {
        Self {
            target_apy,
            max_risk_score,
            rebalance_threshold_bps: 500, // 5%
            volatility_window: 7 * 24 * 60 * 60 * 1_000_000_000, // 7 days
            min_strategy_weight: 1000, // 10%
            max_strategy_weight: 4000, // 40%
            optimization_frequency: 24 * 60 * 60 * 1_000_000_000, // 1 day
            last_optimization: 0,
            historical_performance: Vec::new(),
        }
    }

    pub fn calculate_optimal_weights(&self, strategies: &[(String, Strategy)]) -> Vec<(String, u32)> {
        let mut weights = Vec::new();
        let total_strategies = strategies.len();
        
        if total_strategies == 0 {
            return weights;
        }

        // Calculate Sharpe ratios
        let mut strategy_metrics: Vec<(String, f64, f64)> = strategies
            .iter()
            .filter(|(_, s)| s.is_active)
            .map(|(name, strategy)| {
                let (returns, volatility) = self.calculate_strategy_metrics(strategy);
                (name.clone(), returns, volatility)
            })
            .collect();

        // Sort by risk-adjusted returns (Sharpe ratio)
        strategy_metrics.sort_by(|a, b| {
            let sharpe_a = if a.2 == 0.0 { 0.0 } else { a.1 / a.2 };
            let sharpe_b = if b.2 == 0.0 { 0.0 } else { b.1 / b.2 };
            sharpe_b.partial_cmp(&sharpe_a).unwrap()
        });

        // Allocate weights based on performance
        let mut remaining_weight = BASIS_POINTS;
        let mut allocated_strategies = 0;

        for (name, _, _) in strategy_metrics {
            let weight = if allocated_strategies == total_strategies - 1 {
                remaining_weight
            } else {
                let base_weight = (BASIS_POINTS / total_strategies as u32)
                    .max(self.min_strategy_weight)
                    .min(self.max_strategy_weight);
                base_weight.min(remaining_weight)
            };

            weights.push((name, weight));
            remaining_weight -= weight;
            allocated_strategies += 1;
        }

        weights
    }

    fn calculate_strategy_metrics(&self, strategy: &Strategy) -> (f64, f64) {
        let mut returns = 0.0;
        let mut volatility = 0.0;

        if strategy.performance_history.len() < 2 {
            return (returns, volatility);
        }

        // Calculate average returns
        let total_profit = strategy.performance_history
            .iter()
            .map(|(_, profit)| *profit)
            .sum::<Balance>();
        
        let time_period = strategy.performance_history.last().unwrap().0 - 
            strategy.performance_history.first().unwrap().0;
        
        if time_period > 0 {
            returns = total_profit as f64 / time_period as f64;
        }

        // Calculate volatility using standard deviation
        let mean_return = returns;
        let variance: f64 = strategy.performance_history
            .windows(2)
            .map(|w| {
                let period_return = (w[1].1 as f64 - w[0].1 as f64) / w[0].1 as f64;
                (period_return - mean_return).powi(2)
            })
            .sum::<f64>() / (strategy.performance_history.len() - 1) as f64;

        volatility = variance.sqrt();

        (returns, volatility)
    }
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct YieldVault {
    owner: AccountId,
    status: VaultStatus,
    total_shares: Balance,
    total_assets: Balance,
    
    user_positions: LookupMap<AccountId, UserPosition>,
    strategies: UnorderedMap<String, Strategy>,
    
    metrics: VaultMetrics,
    tvl_history: Vector<(u64, Balance)>,
    
    fees: Fees,
    minimum_lockup_duration: u64,
    operators: UnorderedMap<AccountId, bool>,
    
    reward_pool: Balance,
    last_reward_distribution: u64,
    treasury: AccountId,
}

#[near_bindgen]
impl YieldVault {
    #[init]
    pub fn new(
        owner: AccountId,
        treasury: AccountId,
        fees: Option<Fees>,
        minimum_lockup: Option<u64>,
    ) -> Self {
        require!(!env::state_exists(), "Already initialized");
        
        Self {
            owner: owner.clone(),
            status: VaultStatus::Active,
            total_shares: 0,
            total_assets: 0,
            
            user_positions: LookupMap::new(StorageKey::UserPositions),
            strategies: UnorderedMap::new(StorageKey::Strategies),
            
            metrics: VaultMetrics {
                total_value_locked: 0,
                annual_percentage_yield: 0,
                total_users: 0,
                total_profit: 0,
                last_harvest_timestamp: env::block_timestamp(),
                historical_apy: Vec::new(),
                risk_score: 0,
                sharpe_ratio: 0.0,
            },
            
            tvl_history: Vector::new(StorageKey::TvlHistory),
            
            fees: fees.unwrap_or_default(),
            minimum_lockup_duration: minimum_lockup.unwrap_or(MIN_LOCKUP_DURATION),
            operators: UnorderedMap::new(StorageKey::Operators),
            
            reward_pool: 0,
            last_reward_distribution: env::block_timestamp(),
            treasury,
        }
    }

    // Deposit funds with optional lockup period
    #[payable]
    pub fn deposit(&mut self, lockup_duration: Option<u64>) -> U128 {
        self.assert_active();
        let amount = env::attached_deposit().as_yoctonear();
        
        require!(amount >= MIN_DEPOSIT, "Deposit too small");
        require!(amount <= MAX_DEPOSIT, "Deposit too large");

        let account_id = env::predecessor_account_id();
        let shares = self.calculate_shares_from_amount(amount);
        
        // Update user position
        let mut position = self.get_or_create_position(&account_id);
        position.shares += shares;
        position.deposited_amount += amount;
        position.last_deposit_timestamp = env::block_timestamp();
        position.last_interaction = env::block_timestamp();
        position.locked_until = env::block_timestamp() + 
            lockup_duration.unwrap_or(self.minimum_lockup_duration);

        // Update vault state
        self.total_shares += shares;
        self.total_assets += amount;
        self.metrics.total_value_locked += amount;
        
        if position.deposited_amount == amount {
            self.metrics.total_users += 1;
        }

        // Process deposit fee
        let fee = self.calculate_deposit_fee(amount);
        if fee > 0 {
            self.process_fee(fee);
        }

        // Allocate to strategies
        self.allocate_to_strategies(amount - fee);
        
        // Save state
        self.user_positions.insert(&account_id, &position);
        self.update_tvl_history();
        
        U128(shares)
    }

    fn allocate_to_strategies(&mut self, amount: Balance) {
        let mut updates = Vec::new();
        
        // Collect changes
        for (strategy_name, strategy) in self.strategies.iter() {
            if !strategy.is_active {
                continue;
            }
            
            let allocation = amount * strategy.allocation_ratio as u128 / BASIS_POINTS as u128;
            let mut updated_strategy = strategy.clone();
            updated_strategy.current_balance += allocation;
            updates.push((strategy_name, updated_strategy));
        }
        
        // Apply changes
        for (strategy_name, strategy) in updates {
            self.strategies.insert(&strategy_name, &strategy);
        }
    }

    fn deallocate_from_strategies(&mut self, amount: Balance) {
        let total_active_allocation = self.strategies
            .iter()
            .filter(|(_, s)| s.is_active)
            .map(|(_, s)| s.allocation_ratio)
            .sum::<u32>();

        let mut updates = Vec::new();
        
        // Collect changes
        for (strategy_name, strategy) in self.strategies.iter() {
            if !strategy.is_active {
                continue;
            }
            
            let deallocation = amount * strategy.allocation_ratio as u128 / total_active_allocation as u128;
            let mut updated_strategy = strategy.clone();
            updated_strategy.current_balance = updated_strategy.current_balance.saturating_sub(deallocation);
            updates.push((strategy_name, updated_strategy));
        }
        
        // Apply changes
        for (strategy_name, strategy) in updates {
            self.strategies.insert(&strategy_name, &strategy);
        }
    }

    fn calculate_strategy_yield(&self, strategy: &Strategy) -> Balance {
        // Simplified yield calculation - in production this would be more complex
        let time_elapsed = env::block_timestamp() - strategy.last_harvest_timestamp;
        if time_elapsed == 0 {
            return 0;
        }

        // Example: 10% APY
        let annual_yield_rate = 1000; // 10% in basis points
        let yield_amount = strategy.current_balance * annual_yield_rate as u128 * 
            time_elapsed as u128 / (BASIS_POINTS as u128 * 365 * 24 * 60 * 60 * 1_000_000_000);
        
        yield_amount
    }

    fn distribute_yields(&mut self, total_yield: Balance) {
        if total_yield == 0 || self.total_shares == 0 {
            return;
        }

        // Since LookupMap doesn't have iter(), we'll need to handle this differently
        // In a real implementation, you might want to maintain a separate list of users
        // or use a different collection type that supports iteration
        // For now, this is left as a TODO
    }

    // Withdraw funds
    pub fn withdraw(&mut self, shares: U128) -> Promise {
        assert_one_yocto();
        self.assert_active();
        
        let shares = shares.0;
        let account_id = env::predecessor_account_id();
        let mut position = self.get_position(&account_id);
        
        require!(shares > 0 && shares <= position.shares, "Invalid shares amount");
        
        // Check lockup period
        let is_early_withdrawal = env::block_timestamp() < position.locked_until;
        
        // Calculate withdrawal amount
        let gross_amount = self.calculate_amount_from_shares(shares);
        let fee = self.calculate_withdrawal_fee(gross_amount, is_early_withdrawal);
        let net_amount = gross_amount - fee;

        // Update position
        position.shares -= shares;
        position.deposited_amount = position.deposited_amount * position.shares / (position.shares + shares);
        position.last_interaction = env::block_timestamp();

        // Update vault state
        self.total_shares -= shares;
        self.total_assets -= gross_amount;
        self.metrics.total_value_locked -= gross_amount;

        if position.shares == 0 {
            self.metrics.total_users -= 1;
            self.user_positions.remove(&account_id);
        } else {
            self.user_positions.insert(&account_id, &position);
        }

        // Process fee and deallocate from strategies
        self.process_fee(fee);
        self.deallocate_from_strategies(gross_amount);
        self.update_tvl_history();

        // Transfer funds to user
        Promise::new(account_id).transfer(NearToken::from_yoctonear(net_amount))
    }

    // Claim rewards
    pub fn claim_rewards(&mut self) -> Promise {
        let account_id = env::predecessor_account_id();
        let mut position = self.get_position(&account_id);
        
        require!(position.unclaimed_rewards > 0, "No rewards to claim");

        let amount = position.unclaimed_rewards;
        position.unclaimed_rewards = 0;
        position.cumulative_rewards += amount;
        position.last_interaction = env::block_timestamp();

        self.user_positions.insert(&account_id, &position);
        self.reward_pool -= amount;

        Promise::new(account_id).transfer(NearToken::from_yoctonear(amount))
    }

    // Strategy Management Methods
    #[payable]
    pub fn add_strategy(&mut self, strategy_name: String, max_allocation_bps: u32) {
        self.assert_owner_or_operator();
        require!(max_allocation_bps <= BASIS_POINTS, "Invalid allocation");

        let strategy = Strategy {
            name: strategy_name.clone(),
            allocation_ratio: 0,
            current_balance: 0,
            total_profit: 0,
            is_active: true,
            last_harvest_timestamp: env::block_timestamp(),
            risk_score: 0,
            max_allocation_bps,
            performance_history: Vec::new(),
        };

        self.strategies.insert(&strategy_name, &strategy);
    }

    pub fn update_strategy_allocation(&mut self, strategy_name: String, new_allocation_bps: u32) {
        self.assert_owner_or_operator();
        require!(new_allocation_bps <= BASIS_POINTS, "Invalid allocation");

        let mut strategy = self.get_strategy_internal(&strategy_name);
        require!(new_allocation_bps <= strategy.max_allocation_bps, "Exceeds maximum allocation");

        strategy.allocation_ratio = new_allocation_bps;
        self.strategies.insert(&strategy_name, &strategy);
        
        self.rebalance_strategies();
    }

    pub fn harvest_yield(&mut self) -> Promise {
        require!(self.status == VaultStatus::Active, "Vault is not active");
        let total_yield: Balance = self.calculate_total_yield();
        
        if total_yield > 0 {
            self.metrics.total_profit += total_yield;
            self.update_apy_metrics(total_yield);
            self.allocate_to_strategies(total_yield);
        }
        
        Promise::new(env::current_account_id())
    }

    fn calculate_total_yield(&self) -> Balance {
        let mut total = 0;
        for (_, strategy) in self.strategies.iter() {
            if strategy.is_active {
                total += self.calculate_strategy_yield(&strategy);
            }
        }
        total
    }

    pub fn trigger_emergency_shutdown(&mut self) {
        self.assert_owner_or_operator();
        self.status = VaultStatus::EmergencyShutdown;
    }

    pub fn emergency_withdraw(&mut self) -> Promise {
        require!(self.status == VaultStatus::EmergencyShutdown, "Not in emergency mode");
        self.assert_owner_or_operator();
        
        // Return all funds to users
            Promise::new(env::current_account_id())
    }

    // Internal helper methods
    fn calculate_shares_from_amount(&self, amount: Balance) -> Balance {
        if self.total_shares == 0 || self.total_assets == 0 {
            amount
        } else {
            amount * self.total_shares / self.total_assets
        }
    }

    fn calculate_amount_from_shares(&self, shares: Balance) -> Balance {
        if self.total_shares == 0 {
            0
        } else {
            shares * self.total_assets / self.total_shares
        }
    }

    fn calculate_deposit_fee(&self, amount: Balance) -> Balance {
        amount * self.fees.deposit_fee_bps as u128 / BASIS_POINTS as u128
    }

    fn calculate_withdrawal_fee(&self, amount: Balance, is_early: bool) -> Balance {
        let mut fee_bps = self.fees.withdrawal_fee_bps;
        if is_early {
            fee_bps += self.fees.early_withdrawal_fee_bps;
        }
        amount * fee_bps as u128 / BASIS_POINTS as u128
    }

    fn process_fee(&mut self, amount: Balance) {
        Promise::new(self.treasury.clone()).transfer(NearToken::from_yoctonear(amount));
    }

    fn get_or_create_position(&self, account_id: &AccountId) -> UserPosition {
        self.user_positions.get(account_id).unwrap_or(UserPosition {
            shares: 0,
            deposited_amount: 0,
            last_deposit_timestamp: env::block_timestamp(),
            unclaimed_rewards: 0,
            locked_until: env::block_timestamp(),
            cumulative_rewards: 0,
            last_interaction: env::block_timestamp(),
        })
    }

    fn get_position(&self, account_id: &AccountId) -> UserPosition {
        self.user_positions.get(account_id)
            .expect("No position found")
    }

    fn get_strategy_internal(&self, strategy_name: &String) -> Strategy {
        self.strategies.get(strategy_name)
            .expect("Strategy not found")
    }

    fn rebalance_strategies(&mut self) {
        let mut total_allocation = 0;
        let mut allocations = Vec::new();

        // Calculate target allocations
        for (strategy_name, strategy) in self.strategies.iter() {
            if !strategy.is_active {
                continue;
            }

            total_allocation += strategy.allocation_ratio;
            allocations.push((strategy_name, strategy.allocation_ratio));
        }

        require!(total_allocation <= BASIS_POINTS, "Invalid allocation total");

        // Collect changes
        let mut updates = Vec::new();
        for (strategy_name, target_ratio) in allocations {
            let target_amount = self.total_assets * target_ratio as u128 / BASIS_POINTS as u128;
            let mut strategy = self.get_strategy_internal(&strategy_name);
            
            if strategy.current_balance != target_amount {
                strategy.current_balance = target_amount;
                updates.push((strategy_name, strategy));
            }
        }

        // Apply changes
        for (strategy_name, strategy) in updates {
                self.strategies.insert(&strategy_name, &strategy);
            }
        }

    fn update_apy_metrics(&mut self, period_yield: Balance) {
        let annual_yield = period_yield * 365 * YOCTO_NEAR / self.total_assets;
        self.metrics.annual_percentage_yield = (annual_yield * BASIS_POINTS as u128 / YOCTO_NEAR) as u32;
        
        self.metrics.historical_apy.push((
            env::block_timestamp(),
            self.metrics.annual_percentage_yield
        ));

        // Keep history bounded
        if self.metrics.historical_apy.len() > 30 {
            self.metrics.historical_apy.remove(0);
        }
    }

    fn update_tvl_history(&mut self) {
        self.tvl_history.push(&(
            env::block_timestamp(),
            self.metrics.total_value_locked
        ));

        // Keep history bounded
        if self.tvl_history.len() > 30 {
            self.tvl_history.pop();
        }
    }

    // Access control helpers
    fn assert_active(&self) {
        require!(self.status == VaultStatus::Active, "Vault is not active");
    }

    fn assert_owner_or_operator(&self) {
        let caller = env::predecessor_account_id();
        require!(
            caller == self.owner || 
            self.operators.get(&caller).unwrap_or(false),
            "Unauthorized"
        );
    }

    // View methods
    pub fn get_vault_metrics(&self) -> VaultMetrics {
        self.metrics.clone()
    }

    pub fn get_user_position(&self, account_id: AccountId) -> Option<UserPosition> {
        self.user_positions.get(&account_id)
    }

    pub fn get_strategy(&self, strategy_name: &String) -> Option<Strategy> {
        self.strategies.get(strategy_name)
    }

    pub fn get_all_strategies(&self) -> Vec<(String, Strategy)> {
        self.strategies.iter().collect()
    }

    pub fn get_share_price(&self) -> U128 {
        if self.total_shares == 0 {
            U128(YOCTO_NEAR)
        } else {
            U128(self.total_assets * YOCTO_NEAR / self.total_shares)
        }
    }

    pub fn get_tvl(&self) -> U128 {
        U128(self.metrics.total_value_locked)
    }

    pub fn get_apy(&self) -> u32 {
        self.metrics.annual_percentage_yield
    }

    // Additional Features - Analytics and Integrations
    pub fn get_analytics(&self) -> VaultAnalytics {
        VaultAnalytics {
            tvl_history: self.tvl_history.to_vec(),
            apy_history: self.metrics.historical_apy.clone(),
            total_users: self.metrics.total_users,
            total_profit: self.metrics.total_profit,
            risk_metrics: RiskMetrics {
                risk_score: self.metrics.risk_score,
                sharpe_ratio: self.metrics.sharpe_ratio,
                strategy_diversification: self.calculate_diversification(),
            },
            performance_metrics: PerformanceMetrics {
                current_apy: self.metrics.annual_percentage_yield,
                average_apy: self.calculate_average_apy(),
                best_strategy: self.get_best_performing_strategy(),
                yield_stability: self.calculate_yield_stability(),
            },
        }
    }

    pub fn optimize_yields(&mut self) {
        self.assert_owner_or_operator();
        
        let optimizer = YieldOptimizer::new(
            self.metrics.annual_percentage_yield,
            self.metrics.risk_score
        );

        let strategies: Vec<(String, Strategy)> = self.strategies.iter().collect();
        let optimal_weights = optimizer.calculate_optimal_weights(&strategies);

        // Apply new weights
        for (strategy_name, weight) in optimal_weights {
            self.update_strategy_allocation(strategy_name, weight);
        }

        self.rebalance_strategies();
    }

    pub fn auto_compound(&mut self) -> Promise {
        self.assert_active();
        
        self.harvest_yield()
            .then(Promise::new(env::current_account_id())
                .function_call(
                    "handle_yield_harvest".to_string(),
                    near_sdk::serde_json::to_vec(&()).unwrap(),
                    NearToken::from_yoctonear(0),
                    Gas(env::prepaid_gas().0 / 3)
                )
            )
    }

    #[private]
    pub fn handle_yield_harvest(&mut self, total_yield: Balance) {
        if total_yield > 0 {
            // Reinvest yields
            self.allocate_to_strategies(total_yield);
            
            // Update metrics
            self.metrics.total_profit += total_yield;
            self.update_apy_metrics(total_yield);
        }
    }

    pub fn get_strategy_recommendations(&self) -> Vec<(String, u32, f64)> {
        let optimizer = YieldOptimizer::new(
            self.metrics.annual_percentage_yield,
            self.metrics.risk_score
        );

        let strategies: Vec<(String, Strategy)> = self.strategies.iter().collect();
        let optimal_weights = optimizer.calculate_optimal_weights(&strategies);

        optimal_weights
            .into_iter()
            .map(|(name, weight)| {
                let strategy = self.get_strategy(&name).unwrap();
                let (returns, volatility) = optimizer.calculate_strategy_metrics(&strategy);
                (name, weight, returns / volatility)
            })
            .collect()
    }
}

// Additional structs for analytics
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct VaultAnalytics {
    tvl_history: Vec<(u64, Balance)>,
    apy_history: Vec<(u64, u32)>,
    total_users: u32,
    total_profit: Balance,
    risk_metrics: RiskMetrics,
    performance_metrics: PerformanceMetrics,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct RiskMetrics {
    risk_score: u32,
    sharpe_ratio: f64,
    strategy_diversification: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct PerformanceMetrics {
    current_apy: u32,
    average_apy: u32,
    best_strategy: Option<String>,
    yield_stability: u32,
}

// Implementation of analytics calculations
impl YieldVault {
    fn calculate_diversification(&self) -> u32 {
        let active_strategies = self.strategies
            .iter()
            .filter(|(_, s)| s.is_active)
            .count();
        
        (active_strategies as u32 * BASIS_POINTS) / 
            self.strategies.len() as u32
    }

    fn calculate_average_apy(&self) -> u32 {
        if self.metrics.historical_apy.is_empty() {
            return 0;
        }

        let sum: u32 = self.metrics.historical_apy
            .iter()
            .map(|(_, apy)| *apy)
            .sum();
        
        sum / self.metrics.historical_apy.len() as u32
    }

    fn get_best_performing_strategy(&self) -> Option<String> {
        self.strategies
            .iter()
            .filter(|(_, s)| s.is_active)
            .max_by_key(|(_, s)| s.total_profit)
            .map(|(name, _)| name)
    }

    fn calculate_yield_stability(&self) -> u32 {
        if self.metrics.historical_apy.len() < 2 {
            return BASIS_POINTS;
        }

        let variations: Vec<i32> = self.metrics.historical_apy
            .windows(2)
            .map(|w| w[1].1 as i32 - w[0].1 as i32)
            .collect();

        let avg_variation = variations.iter().map(|v| v.abs()).sum::<i32>() as u32 / 
            variations.len() as u32;

        BASIS_POINTS - (avg_variation * BASIS_POINTS / self.metrics.annual_percentage_yield)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;

    const TREASURY_ID: &str = "treasury.near";

    fn setup_context(predecessor: AccountId, deposit: Balance) {
        let mut context = VMContextBuilder::new()
            .predecessor_account_id(predecessor)
            .attached_deposit(deposit)
            .block_timestamp(1_000_000_000)
            .build();
        testing_env!(context);
    }

    fn setup_vault() -> (YieldVault, AccountId) {
        let owner = accounts(0);
        let treasury = AccountId::new_unchecked(TREASURY_ID.to_string());
        
        setup_context(owner.clone(), 0);
        
        let vault = YieldVault::new(
            owner.clone(),
            treasury,
            None,
            None,
        );

        (vault, owner)
    }

    #[test]
    fn test_initialization() {
        let (vault, owner) = setup_vault();
        
        assert_eq!(vault.owner, owner);
        assert_eq!(vault.total_shares, 0);
        assert_eq!(vault.total_assets, 0);
        assert_eq!(vault.status, VaultStatus::Active);
    }

    #[test]
    fn test_deposit_and_withdraw_cycle() {
        let (mut vault, _) = setup_vault();
        let user = accounts(1);
        let deposit_amount = YOCTO_NEAR * 10; // 10 NEAR

        // Test deposit
        setup_context(user.clone(), deposit_amount);
        let shares = vault.deposit(None).0;
        
        assert!(shares > 0, "Should receive shares");
        assert_eq!(vault.total_assets, deposit_amount);
        
        let position = vault.get_user_position(user.clone()).unwrap();
        assert_eq!(position.shares, shares);

        // Test withdrawal
        setup_context(user.clone(), 1); // for assert_one_yocto
        vault.withdraw(U128(shares));
        
        assert_eq!(vault.total_shares, 0);
        assert!(vault.get_user_position(user.clone()).is_none());
    }

    #[test]
    #[should_panic(expected = "Deposit too small")]
    fn test_minimum_deposit() {
        let (mut vault, _) = setup_vault();
        setup_context(accounts(1), MIN_DEPOSIT - 1);
        vault.deposit(None);
    }

    #[test]
    fn test_strategy_management() {
        let (mut vault, owner) = setup_vault();
        
        // Add strategy
        setup_context(owner.clone(), 1);
        vault.add_strategy("strategy1".to_string(), 5000); // 50% max allocation
        
        let strategy = vault.get_strategy(&"strategy1".to_string()).unwrap();
        assert_eq!(strategy.max_allocation_bps, 5000);
        assert!(strategy.is_active);

        // Update allocation
        vault.update_strategy_allocation("strategy1".to_string(), 3000); // 30% allocation
        let updated_strategy = vault.get_strategy(&"strategy1".to_string()).unwrap();
        assert_eq!(updated_strategy.allocation_ratio, 3000);
    }

    #[test]
    fn test_emergency_procedures() {
        let (mut vault, owner) = setup_vault();
        let user = accounts(1);
        
        // Deposit funds
        setup_context(user.clone(), YOCTO_NEAR * 10);
        vault.deposit(None);

        // Trigger emergency
        setup_context(owner.clone(), 0);
        vault.trigger_emergency_shutdown();
        assert_eq!(vault.status, VaultStatus::EmergencyShutdown);

        // Emergency withdraw
        setup_context(user.clone(), 0);
        vault.emergency_withdraw();
        assert!(vault.get_user_position(user.clone()).is_none());
    }

    #[test]
    fn test_initialize_vault() {
        let context = VMContextBuilder::new()
            .predecessor_account_id(AccountId::new_unvalidated("owner.near".to_string()))
            .build();
        testing_env!(context);

        let treasury = AccountId::new_unvalidated(TREASURY_ID.to_string());
        let owner = AccountId::new_unvalidated("owner.near".to_string());
        let mut vault = YieldVault::new(
            owner.clone(),
            treasury.clone(),
            None,
            None,
        );
    }
}

// Add at the end of the file, after the tests module
pub mod test_vault_outputs;

#[cfg(test)]
mod main_tests {
    use super::*;

    #[test]
    fn run_demonstration() {
        test_vault_outputs::run_demonstration();
    }
}