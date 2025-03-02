use near_sdk::{env, AccountId, Balance};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

const MAX_HISTORY_SIZE: usize = 24; // Keep 24 hours of history
const ALERT_THRESHOLD_BPS: u32 = 1000; // 10% change triggers alert
const EMERGENCY_THRESHOLD_BPS: u32 = 2000; // 20% change triggers emergency
const VOLATILITY_WINDOW: usize = 12; // 12 hour window for volatility calculation

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum AlertLevel {
    Normal,
    Warning,
    Critical,
    Emergency,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct YieldAlert {
    pub protocol: String,
    pub level: AlertLevel,
    pub old_apy: u32,
    pub new_apy: u32,
    pub timestamp: u64,
    pub volatility_score: u32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ProtocolMetrics {
    pub apy_history: VecDeque<(u64, u32)>,
    pub alert_history: VecDeque<YieldAlert>,
    pub last_alert: Option<YieldAlert>,
    pub volatility_score: u32,
}

pub struct AlertSystem {
    protocol_metrics: std::collections::HashMap<String, ProtocolMetrics>,
    alert_callbacks: Vec<Box<dyn Fn(&YieldAlert)>>,
}

impl AlertSystem {
    pub fn new() -> Self {
        Self {
            protocol_metrics: std::collections::HashMap::new(),
            alert_callbacks: Vec::new(),
        }
    }

    pub fn register_alert_callback<F>(&mut self, callback: F)
    where
        F: Fn(&YieldAlert) + 'static,
    {
        self.alert_callbacks.push(Box::new(callback));
    }

    pub fn monitor_yield_change(
        &mut self,
        protocol: &str,
        new_apy: u32,
    ) -> Option<YieldAlert> {
        let metrics = self.get_or_create_metrics(protocol);
        
        // Add new APY to history
        metrics.apy_history.push_back((env::block_timestamp(), new_apy));
        if metrics.apy_history.len() > MAX_HISTORY_SIZE {
            metrics.apy_history.pop_front();
        }

        // Calculate volatility
        metrics.volatility_score = self.calculate_volatility(&metrics.apy_history);

        // Check for significant changes
        let alert = if let Some((_, old_apy)) = metrics.apy_history.iter().rev().nth(1) {
            let change_bps = self.calculate_change_bps(*old_apy, new_apy);
            
            let alert_level = if change_bps >= EMERGENCY_THRESHOLD_BPS {
                AlertLevel::Emergency
            } else if change_bps >= ALERT_THRESHOLD_BPS {
                AlertLevel::Critical
            } else if metrics.volatility_score > 500 {
                AlertLevel::Warning
            } else {
                AlertLevel::Normal
            };

            if alert_level != AlertLevel::Normal {
                Some(YieldAlert {
                    protocol: protocol.to_string(),
                    level: alert_level,
                    old_apy: *old_apy,
                    new_apy,
                    timestamp: env::block_timestamp(),
                    volatility_score: metrics.volatility_score,
                })
            } else {
                None
            }
        } else {
            None
        };

        // Update alert history if there's an alert
        if let Some(alert) = &alert {
            metrics.alert_history.push_back(alert.clone());
            if metrics.alert_history.len() > MAX_HISTORY_SIZE {
                metrics.alert_history.pop_front();
            }
            metrics.last_alert = Some(alert.clone());

            // Trigger callbacks
            for callback in &self.alert_callbacks {
                callback(alert);
            }
        }

        alert
    }

    pub fn get_protocol_health(&self, protocol: &str) -> AlertLevel {
        self.protocol_metrics
            .get(protocol)
            .and_then(|m| m.last_alert.as_ref())
            .map(|a| a.level.clone())
            .unwrap_or(AlertLevel::Normal)
    }

    pub fn get_alert_history(&self, protocol: &str) -> Vec<YieldAlert> {
        self.protocol_metrics
            .get(protocol)
            .map(|m| m.alert_history.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn get_volatility_score(&self, protocol: &str) -> u32 {
        self.protocol_metrics
            .get(protocol)
            .map(|m| m.volatility_score)
            .unwrap_or(0)
    }

    fn get_or_create_metrics(&mut self, protocol: &str) -> &mut ProtocolMetrics {
        if !self.protocol_metrics.contains_key(protocol) {
            self.protocol_metrics.insert(
                protocol.to_string(),
                ProtocolMetrics {
                    apy_history: VecDeque::new(),
                    alert_history: VecDeque::new(),
                    last_alert: None,
                    volatility_score: 0,
                },
            );
        }
        self.protocol_metrics.get_mut(protocol).unwrap()
    }

    fn calculate_change_bps(&self, old_value: u32, new_value: u32) -> u32 {
        let change = if new_value > old_value {
            new_value - old_value
        } else {
            old_value - new_value
        };

        (change * 10_000) / old_value
    }

    fn calculate_volatility(&self, history: &VecDeque<(u64, u32)>) -> u32 {
        if history.len() < 2 {
            return 0;
        }

        let window = history
            .iter()
            .rev()
            .take(VOLATILITY_WINDOW)
            .collect::<Vec<_>>();

        let mut total_change = 0;
        for i in 1..window.len() {
            let change = self.calculate_change_bps(window[i].1, window[i-1].1);
            total_change += change;
        }

        total_change / (window.len() - 1) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::testing_env;

    fn setup_context(timestamp: u64) {
        let context = VMContextBuilder::new()
            .block_timestamp(timestamp)
            .build();
        testing_env!(context);
    }

    #[test]
    fn test_yield_monitoring() {
        setup_context(1_000_000);
        let mut alert_system = AlertSystem::new();
        
        // Initial APY
        assert!(alert_system.monitor_yield_change("aave", 1000).is_none()); // 10%
        
        // Small change - no alert
        setup_context(1_100_000);
        assert!(alert_system.monitor_yield_change("aave", 1050).is_none()); // 10.5%
        
        // Large change - should alert
        setup_context(1_200_000);
        let alert = alert_system.monitor_yield_change("aave", 1500).unwrap(); // 15%
        assert_eq!(alert.level, AlertLevel::Critical);
        
        // Emergency change
        setup_context(1_300_000);
        let alert = alert_system.monitor_yield_change("aave", 2000).unwrap(); // 20%
        assert_eq!(alert.level, AlertLevel::Emergency);
    }

    #[test]
    fn test_volatility_calculation() {
        setup_context(1_000_000);
        let mut alert_system = AlertSystem::new();
        
        // Add several APY points
        for (i, apy) in vec![1000, 1100, 900, 1200, 800].iter().enumerate() {
            setup_context(1_000_000 + (i as u64 * 100_000));
            alert_system.monitor_yield_change("compound", *apy);
        }
        
        let volatility = alert_system.get_volatility_score("compound");
        assert!(volatility > 0, "Should detect volatility");
    }
} 