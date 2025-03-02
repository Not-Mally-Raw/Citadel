#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::*;
    use near_sdk::json_types::U128;

    #[tokio::test]
    async fn test_complete_bridge_flow() {
        let context = get_context(vec![], false);
        testing_env!(context);

        let mut bridge = setup_bridge();
        let sender = AccountId::new_unchecked("alice.near".to_string());
        let receiver = AccountId::new_unchecked("bob.aurora".to_string());
        let amount = U128(1_000_000);

        // Test transfer initiation
        let tx_hash = bridge.transfer(sender.clone(), receiver.clone(), amount.0)
            .await
            .expect("Transfer should succeed");

        // Verify transaction status
        let tx = bridge.get_transaction(&tx_hash)
            .expect("Transaction should exist");
        assert_eq!(tx.status, TransactionStatus::Pending);

        // Test confirmation
        bridge.confirm_transfer(&tx_hash)
            .await
            .expect("Confirmation should succeed");

        let confirmed_tx = bridge.get_transaction(&tx_hash)
            .expect("Transaction should exist");
        assert_eq!(confirmed_tx.status, TransactionStatus::Completed);
    }

    #[test]
    fn test_fee_calculation() {
        let context = get_context(vec![], false);
        testing_env!(context);

        let bridge = setup_bridge();
        let amount = 1_000_000_000;
        let fee = bridge.calculate_fee(amount);
        
        // Fee should be BRIDGE_FEE_BPS% of amount
        assert_eq!(fee, amount * BRIDGE_FEE_BPS as u128 / 10_000);
    }
}