use criterion::{black_box, criterion_group, criterion_main, Criterion};
use security_audits::SecurityReport;

fn benchmark_security_report(c: &mut Criterion) {
    c.bench_function("create security report", |b| {
        b.iter(|| {
            let report = SecurityReport {
                contract_address: black_box("vault.near".to_string()),
                audit_date: black_box(1677649200),
                risk_level: black_box(security_audits::RiskLevel::Low),
                findings: black_box(vec![]),
                overall_score: black_box(95),
            };
            black_box(report)
        })
    });
}

criterion_group!(benches, benchmark_security_report);
criterion_main!(benches); 