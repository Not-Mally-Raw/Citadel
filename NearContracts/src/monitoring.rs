use near_sdk::{env, AccountId, Balance};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use log::{info, warn, error};
use std::collections::VecDeque;

const MAX_EVENTS_HISTORY: usize = 1000;
const ALERT_WEBHOOK_URL: &str = "https://api.monitoring.com/webhook";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum EventType {
    Deposit,
    Withdrawal,
    Rebalance,
    BridgeTransfer,
    OracleUpdate,
    SecurityAlert,
    EmergencyAction,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Event {
    pub event_type: EventType,
    pub timestamp: u64,
    pub account_id: Option<AccountId>,
    pub amount: Option<Balance>,
    pub details: String,
    pub success: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HealthMetrics {
    pub total_tvl: Balance,
    pub active_users: u32,
    pub protocol_allocations: HashMap<String, u32>,
    pub recent_apy: f64,
    pub gas_usage: u64,
    pub error_count: u32,
}

pub struct Monitor {
    events: Vec<Event>,
    metrics: HealthMetrics,
    alert_callbacks: Vec<Box<dyn Fn(&Event)>>,
    anomaly_detectors: HashMap<String, AnomalyDetector>,
}

impl Monitor {
    pub fn new() -> Self {
        Self {
            events: Vec::with_capacity(MAX_EVENTS_HISTORY),
            metrics: HealthMetrics {
                total_tvl: 0,
                active_users: 0,
                protocol_allocations: HashMap::new(),
                recent_apy: 0.0,
                gas_usage: 0,
                error_count: 0,
            },
            alert_callbacks: Vec::new(),
            anomaly_detectors: HashMap::new(),
        }
    }

    pub fn log_event(&mut self, event: Event) {
        // Log to console/file
        match event.event_type {
            EventType::SecurityAlert | EventType::EmergencyAction => {
                error!("Critical event: {:?}", event);
                self.trigger_alerts(&event);
            },
            EventType::Withdrawal | EventType::BridgeTransfer => {
                warn!("Important event: {:?}", event);
            },
            _ => info!("Event: {:?}", event),
        }

        // Update metrics
        if !event.success {
            self.metrics.error_count += 1;
        }
        self.metrics.gas_usage += env::used_gas().0;

        // Store event
        self.events.push(event);
        if self.events.len() > MAX_EVENTS_HISTORY {
            self.events.remove(0);
        }
    }

    pub fn update_metrics(&mut self, metrics: HealthMetrics) {
        self.metrics = metrics;
        
        // Log significant changes
        info!(
            "TVL: {}, Users: {}, APY: {:.2}%",
            self.metrics.total_tvl,
            self.metrics.active_users,
            self.metrics.recent_apy
        );
    }

    pub fn register_alert_callback<F>(&mut self, callback: F)
    where
        F: Fn(&Event) + 'static,
    {
        self.alert_callbacks.push(Box::new(callback));
    }

    pub fn get_recent_events(&self, event_type: Option<EventType>) -> Vec<&Event> {
        match event_type {
            Some(et) => self.events
                .iter()
                .filter(|e| e.event_type == et)
                .collect(),
            None => self.events.iter().collect(),
        }
    }

    pub fn get_health_check(&self) -> HealthStatus {
        let status = if self.metrics.error_count > 10 {
            HealthStatus::Critical
        } else if self.metrics.error_count > 5 {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        };

        status
    }

    async fn trigger_alerts(&self, event: &Event) {
        // Call registered callbacks
        for callback in &self.alert_callbacks {
            callback(event);
        }

        // Send to webhook
        if let Err(e) = self.send_alert_webhook(event).await {
            error!("Failed to send alert: {}", e);
        }
    }

    async fn send_alert_webhook(&self, event: &Event) -> Result<(), String> {
        let client = reqwest::Client::new();
        let response = client
            .post(ALERT_WEBHOOK_URL)
            .json(event)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !response.status().is_success() {
            return Err("Webhook request failed".to_string());
        }

        Ok(())
    }

    pub fn add_anomaly_detectors(&mut self) {
        let tvl_detector = AnomalyDetector::new(24, 3.0); // 24 hours window, 3 sigma
        let apy_detector = AnomalyDetector::new(168, 2.5); // 1 week window, 2.5 sigma
        let gas_detector = AnomalyDetector::new(100, 4.0); // 100 tx window, 4 sigma

        self.anomaly_detectors.insert("tvl".to_string(), tvl_detector);
        self.anomaly_detectors.insert("apy".to_string(), apy_detector);
        self.anomaly_detectors.insert("gas".to_string(), gas_detector);
    }

    pub fn check_anomalies(&mut self) -> Vec<String> {
        let mut alerts = Vec::new();

        // Check TVL anomaly
        if let Some(detector) = self.anomaly_detectors.get_mut("tvl") {
            if detector.update(self.metrics.total_tvl as f64) {
                alerts.push(format!(
                    "TVL anomaly detected: {} (mean: {:.2}, std: {:.2})",
                    self.metrics.total_tvl, detector.mean, detector.std_dev
                ));
            }
        }

        // Check APY anomaly
        if let Some(detector) = self.anomaly_detectors.get_mut("apy") {
            if detector.update(self.metrics.recent_apy) {
                alerts.push(format!(
                    "APY anomaly detected: {:.2}% (mean: {:.2}%, std: {:.2}%)",
                    self.metrics.recent_apy, detector.mean, detector.std_dev
                ));
            }
        }

        // Check gas usage anomaly
        if let Some(detector) = self.anomaly_detectors.get_mut("gas") {
            if detector.update(self.metrics.gas_usage as f64) {
                alerts.push(format!(
                    "Gas usage anomaly detected: {} (mean: {:.2}, std: {:.2})",
                    self.metrics.gas_usage, detector.mean, detector.std_dev
                ));
            }
        }

        alerts
    }

    pub fn get_performance_metrics(&self) -> PerformanceMetrics {
        PerformanceMetrics {
            tvl_growth_rate: self.calculate_growth_rate("tvl"),
            user_growth_rate: self.calculate_growth_rate("users"),
            avg_transaction_cost: self.calculate_avg_gas(),
            protocol_distribution: self.metrics.protocol_allocations.clone(),
            risk_adjusted_apy: self.calculate_risk_adjusted_apy(),
        }
    }

    fn calculate_growth_rate(&self, metric_type: &str) -> f64 {
        let events = match metric_type {
            "tvl" => self.get_recent_events(Some(EventType::Deposit)),
            "users" => self.get_recent_events(None),
            _ => return 0.0,
        };

        if events.len() < 2 {
            return 0.0;
        }

        let oldest = events.first().unwrap();
        let newest = events.last().unwrap();
        let time_diff = (newest.timestamp - oldest.timestamp) as f64;

        if time_diff == 0.0 {
            return 0.0;
        }

        match metric_type {
            "tvl" => {
                let value_diff = self.metrics.total_tvl as f64;
                (value_diff / time_diff) * 100.0
            }
            "users" => {
                let user_diff = self.metrics.active_users as f64;
                (user_diff / time_diff) * 100.0
            }
            _ => 0.0,
        }
    }

    fn calculate_avg_gas(&self) -> u64 {
        if self.events.is_empty() {
            return 0;
        }
        self.metrics.gas_usage / self.events.len() as u64
    }

    fn calculate_risk_adjusted_apy(&self) -> f64 {
        if self.metrics.error_count == 0 {
            return self.metrics.recent_apy;
        }
        let risk_factor = 1.0 - (self.metrics.error_count as f64 * 0.1).min(0.5);
        self.metrics.recent_apy * risk_factor
    }
}

#[derive(Debug, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnomalyDetector {
    window_size: usize,
    threshold: f64,
    historical_data: VecDeque<f64>,
    mean: f64,
    std_dev: f64,
}

impl AnomalyDetector {
    pub fn new(window_size: usize, threshold: f64) -> Self {
        Self {
            window_size,
            threshold,
            historical_data: VecDeque::with_capacity(window_size),
            mean: 0.0,
            std_dev: 0.0,
        }
    }

    pub fn update(&mut self, value: f64) -> bool {
        // Add new value
        self.historical_data.push_back(value);
        if self.historical_data.len() > self.window_size {
            self.historical_data.pop_front();
        }

        // Update statistics
        self.update_statistics();

        // Check for anomaly
        self.is_anomaly(value)
    }

    fn update_statistics(&mut self) {
        let n = self.historical_data.len() as f64;
        if n < 2.0 {
            return;
        }

        // Calculate mean
        self.mean = self.historical_data.iter().sum::<f64>() / n;

        // Calculate standard deviation
        self.std_dev = (self.historical_data.iter()
            .map(|x| (x - self.mean).powi(2))
            .sum::<f64>() / (n - 1.0))
            .sqrt();
    }

    fn is_anomaly(&self, value: f64) -> bool {
        if self.std_dev == 0.0 {
            return false;
        }
        let z_score = (value - self.mean).abs() / self.std_dev;
        z_score > self.threshold
    }
}

#[derive(Debug, Serialize)]
pub struct PerformanceMetrics {
    pub tvl_growth_rate: f64,
    pub user_growth_rate: f64,
    pub avg_transaction_cost: u64,
    pub protocol_distribution: HashMap<String, u32>,
    pub risk_adjusted_apy: f64,
}

// Example usage:
/*
let mut monitor = Monitor::new();

// Register alert callback
monitor.register_alert_callback(|event| {
    println!("Alert: {:?}", event);
});

// Log events
monitor.log_event(Event {
    event_type: EventType::Deposit,
    timestamp: env::block_timestamp(),
    account_id: Some(env::predecessor_account_id()),
    amount: Some(1000),
    details: "User deposit".to_string(),
    success: true,
});

// Update metrics
monitor.update_metrics(HealthMetrics {
    total_tvl: 1_000_000,
    active_users: 100,
    protocol_allocations: HashMap::new(),
    recent_apy: 10.5,
    gas_usage: 0,
    error_count: 0,
});

// Check health
assert_eq!(monitor.get_health_check(), HealthStatus::Healthy);
*/ 