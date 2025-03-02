use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use rust_decimal::Decimal;

#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityReport {
    pub contract_address: String,
    pub audit_date: u64,
    pub risk_level: RiskLevel,
    pub findings: Vec<Finding>,
    pub overall_score: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RiskLevel {
    Critical,
    High,
    Medium,
    Low,
    Informational,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub title: String,
    pub severity: RiskLevel,
    pub description: String,
    pub recommendation: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_report() {
        let report = SecurityReport {
            contract_address: "vault.near".to_string(),
            audit_date: 1677649200,
            risk_level: RiskLevel::Low,
            findings: vec![
                Finding {
                    id: "AUDIT-001".to_string(),
                    title: "Gas Optimization".to_string(),
                    severity: RiskLevel::Informational,
                    description: "Gas usage can be optimized".to_string(),
                    recommendation: "Implement batch processing".to_string(),
                }
            ],
            overall_score: 95,
        };
        assert_eq!(report.overall_score, 95);
    }
} 