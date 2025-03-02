use near_sdk::{env, AccountId, Balance, Promise};
use serde::{Deserialize, Serialize};
use reqwest;

const AI_ENDPOINT: &str = "http://localhost:5000/get_signal";
const REBALANCE_THRESHOLD: u32 = 500; // 5% in basis points
const MAX_SLIPPAGE: u32 = 100; // 1% in basis points

#[derive(Serialize, Deserialize)]
pub struct AISignal {
    pub target_allocations: Vec<(String, u32)>,
    pub risk_score: u32,
    pub confidence_score: u32,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize)]
pub struct RebalanceResult {
    pub success: bool,
    pub gas_used: u64,
    pub slippage: u32,
    pub new_allocations: Vec<(String, u32)>,
}

pub struct Rebalancer {
    last_rebalance: u64,
    min_interval: u64,
    current_allocations: Vec<(String, u32)>,
}

impl Rebalancer {
    pub fn new(min_interval: u64) -> Self {
        Self {
            last_rebalance: 0,
            min_interval,
            current_allocations: Vec::new(),
        }
    }

    pub async fn fetch_ai_signal(&self) -> Result<AISignal, String> {
        let client = reqwest::Client::new();
        let response = client
            .get(AI_ENDPOINT)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        
        response.json::<AISignal>()
            .await
            .map_err(|e| e.to_string())
    }

    pub fn should_rebalance(&self, current_apys: &[(String, u32)]) -> bool {
        if env::block_timestamp() - self.last_rebalance < self.min_interval {
            return false;
        }

        // Check if any APY difference exceeds threshold
        for (protocol, current_apy) in current_apys {
            if let Some((_, target_apy)) = self.current_allocations
                .iter()
                .find(|(p, _)| p == protocol)
            {
                let difference = if current_apy > target_apy {
                    current_apy - target_apy
                } else {
                    target_apy - current_apy
                };

                if difference > REBALANCE_THRESHOLD {
                    return true;
                }
            }
        }
        false
    }

    pub async fn execute_rebalance(
        &mut self,
        total_assets: Balance,
        current_apys: Vec<(String, u32)>
    ) -> Result<RebalanceResult, String> {
        // 1. Fetch AI signal
        let signal = self.fetch_ai_signal().await?;
        
        // 2. Validate signal
        if !self.validate_signal(&signal) {
            return Err("Invalid AI signal".to_string());
        }

        // 3. Calculate optimal moves
        let moves = self.calculate_rebalance_moves(
            total_assets,
            &signal.target_allocations
        );

        // 4. Execute moves with slippage protection
        let result = self.execute_moves(moves)?;

        // 5. Update state
        self.current_allocations = signal.target_allocations;
        self.last_rebalance = env::block_timestamp();

        Ok(result)
    }

    fn validate_signal(&self, signal: &AISignal) -> bool {
        // Validate timestamp
        if signal.timestamp < self.last_rebalance {
            return false;
        }

        // Validate allocation total
        let total_allocation: u32 = signal.target_allocations
            .iter()
            .map(|(_, allocation)| *allocation)
            .sum();

        if total_allocation != 10_000 {
            return false;
        }

        // Validate confidence score
        signal.confidence_score >= 7000 // 70% minimum confidence
    }

    fn calculate_rebalance_moves(
        &self,
        total_assets: Balance,
        target_allocations: &[(String, u32)]
    ) -> Vec<(String, String, Balance)> {
        let mut moves = Vec::new();
        let mut excess_pools = Vec::new();
        let mut deficit_pools = Vec::new();

        // Calculate required moves
        for (protocol, target_bps) in target_allocations {
            let target_amount = total_assets * *target_bps as u128 / 10_000;
            let current_amount = self.get_current_amount(protocol);

            if current_amount > target_amount {
                excess_pools.push((protocol.clone(), current_amount - target_amount));
            } else if current_amount < target_amount {
                deficit_pools.push((protocol.clone(), target_amount - current_amount));
            }
        }

        // Match excess with deficit pools
        for (excess_protocol, excess_amount) in excess_pools {
            for (deficit_protocol, deficit_amount) in deficit_pools.iter_mut() {
                if *deficit_amount == 0 {
                    continue;
                }

                let move_amount = excess_amount.min(*deficit_amount);
                if move_amount > 0 {
                    moves.push((
                        excess_protocol.clone(),
                        deficit_protocol.clone(),
                        move_amount
                    ));
                    *deficit_amount -= move_amount;
                }
            }
        }

        moves
    }

    fn execute_moves(
        &self,
        moves: Vec<(String, String, Balance)>
    ) -> Result<RebalanceResult, String> {
        let mut gas_used = 0;
        let mut slippage = 0;
        let mut new_allocations = self.current_allocations.clone();

        for (from, to, amount) in moves {
            // Execute move with slippage check
            let actual_amount = self.execute_single_move(&from, &to, amount)?;
            let move_slippage = ((amount - actual_amount) * 10_000 / amount) as u32;

            if move_slippage > MAX_SLIPPAGE {
                return Err("Slippage too high".to_string());
            }

            slippage = slippage.max(move_slippage);
            gas_used += env::used_gas().0;

            // Update allocations
            if let Some(from_allocation) = new_allocations
                .iter_mut()
                .find(|(p, _)| *p == from)
            {
                from_allocation.1 -= (amount * 10_000 / env::account_balance()) as u32;
            }

            if let Some(to_allocation) = new_allocations
                .iter_mut()
                .find(|(p, _)| *p == to)
            {
                to_allocation.1 += (actual_amount * 10_000 / env::account_balance()) as u32;
            }
        }

        Ok(RebalanceResult {
            success: true,
            gas_used,
            slippage,
            new_allocations,
        })
    }

    fn execute_single_move(
        &self,
        from: &str,
        to: &str,
        amount: Balance
    ) -> Result<Balance, String> {
        // Implementation would integrate with specific protocols
        // For now, return the amount as if perfectly executed
        Ok(amount)
    }

    fn get_current_amount(&self, protocol: &str) -> Balance {
        // This would fetch actual balance from protocol
        // For now, return 0
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::testing_env;

    fn setup_context() {
        let context = VMContextBuilder::new()
            .block_timestamp(1_000_000_000)
            .build();
        testing_env!(context);
    }

    #[test]
    fn test_should_rebalance() {
        setup_context();
        let rebalancer = Rebalancer::new(3600 * 1_000_000_000); // 1 hour

        let current_apys = vec![
            ("protocol1".to_string(), 1000),
            ("protocol2".to_string(), 1500),
        ];

        assert!(rebalancer.should_rebalance(&current_apys));
    }

    #[test]
    fn test_validate_signal() {
        setup_context();
        let rebalancer = Rebalancer::new(3600 * 1_000_000_000);

        let signal = AISignal {
            target_allocations: vec![
                ("protocol1".to_string(), 5000),
                ("protocol2".to_string(), 5000),
            ],
            risk_score: 7,
            confidence_score: 8000,
            timestamp: 2_000_000_000,
        };

        assert!(rebalancer.validate_signal(&signal));
    }
} 