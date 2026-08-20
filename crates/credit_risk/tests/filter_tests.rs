use credit_risk::{state_of, CreditRiskFilter, PaymentFeatures, RiskState};

fn healthy_features() -> PaymentFeatures {
    PaymentFeatures {
        avg_days_late_recent: 0.0,
        max_days_late_6: 0.0,
        missed_count_6: 0.0,
        on_time_count_6: 6.0,
        payment_ratio_recent: 1.0,
        collateral_utilization: 0.5,
        loan_age_fraction: 0.3,
        network_contagion_score: 0.0,
    }
}

fn distressed_features() -> PaymentFeatures {
    PaymentFeatures {
        avg_days_late_recent: 45.0,
        max_days_late_6: 60.0,
        missed_count_6: 4.0,
        on_time_count_6: 0.0,
        payment_ratio_recent: 0.1,
        collateral_utilization: 2.5,
        loan_age_fraction: 0.8,
        network_contagion_score: 0.9,
    }
}

#[test]
fn belief_stays_a_valid_probability_distribution() {
    let filter = CreditRiskFilter::new().expect("filter should construct");
    let prior = credit_risk::initial_belief().expect("uniform prior should construct");

    let posterior = filter.assess(&prior, &healthy_features()).expect("assess should succeed");
    let sum: f32 = posterior.as_slice().iter().sum();
    assert!((sum - 1.0).abs() < 1e-4, "belief should sum to ~1.0, got {sum}");
    assert!(posterior.as_slice().iter().all(|&p| (0.0..=1.0).contains(&p)));
}

#[test]
fn state_of_matches_argmax_of_belief() {
    let filter = CreditRiskFilter::new().expect("filter should construct");
    let prior = credit_risk::initial_belief().expect("uniform prior should construct");
    let posterior = filter.assess(&prior, &distressed_features()).expect("assess should succeed");

    let (state, probability) = state_of(&posterior);
    let max_in_slice = posterior.as_slice().iter().cloned().fold(0.0f32, f32::max);
    assert!((probability - max_in_slice).abs() < 1e-6);
    assert!(matches!(
        state,
        RiskState::Current | RiskState::Watch | RiskState::Delinquent | RiskState::Default
    ));
}

#[test]
fn repeated_assessment_is_deterministic_for_fixed_weights() {
    // Same untrained filter instance, same inputs, twice -- should be
    // bit-for-bit reproducible since there's no randomness in a forward
    // pass once weights are fixed.
    let filter = CreditRiskFilter::new().expect("filter should construct");
    let prior = credit_risk::initial_belief().expect("uniform prior should construct");

    let first = filter.assess(&prior, &healthy_features()).unwrap();
    let second = filter.assess(&prior, &healthy_features()).unwrap();
    assert_eq!(first.as_slice(), second.as_slice());
}
