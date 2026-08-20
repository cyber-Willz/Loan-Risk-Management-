use serde::{Deserialize, Serialize};

/// Input dimension the emission network is configured for. Keep this in
/// sync with the number of fields read out in [`PaymentFeatures::to_vector`].
pub const FEATURE_DIM: usize = 8;

/// A single observation window's worth of loan-behavior signals, computed
/// from `payments` (and, for the last two fields, `loans`/`relationship
/// network state) as of the assessment time. This is the HMM's
/// observation `y_t`; the filter never sees raw payment rows directly.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaymentFeatures {
    /// Mean days late across the last 3 due payments (0.0 if none late).
    pub avg_days_late_recent: f32,
    /// Max days late across the last 6 due payments.
    pub max_days_late_6: f32,
    /// Count of missed (unpaid past due date) payments in the last 6.
    pub missed_count_6: f32,
    /// Count of on-time payments in the last 6 (signal for recovery, not
    /// just deterioration).
    pub on_time_count_6: f32,
    /// amount_paid / amount_due, averaged over the last 3 due payments;
    /// 1.0 = paying in full, < 1.0 = partial payments.
    pub payment_ratio_recent: f32,
    /// outstanding_principal / collateral_value, clamped to [0, 3]; higher
    /// means thinner collateral cushion. 0.0 if uncollateralized data is
    /// unavailable (treated as neutral, not as zero risk).
    pub collateral_utilization: f32,
    /// Months since origination / term_months, in [0, 1]; where the loan
    /// is in its lifecycle (early-life delinquency reads differently than
    /// late-life).
    pub loan_age_fraction: f32,
    /// This borrower's current network contagion score in [0, 1] from
    /// `credit_network`, or 0.0 if the borrower has no modeled
    /// relationships.
    pub network_contagion_score: f32,
}

impl PaymentFeatures {
    /// Converts to the network's input vector, normalizing every field to
    /// roughly `[0, 1]`.
    ///
    /// This matters more than it looks: `avg_days_late_recent` and
    /// `max_days_late_6` are naturally unbounded (a payment can be
    /// thousands of days overdue), and feeding raw unbounded magnitudes
    /// into a linear layer -- especially one with untrained,
    /// small-random-scale weights -- saturates the softmax. A live run
    /// against 200 real loan records (see repo history / this crate's
    /// test suite) confirmed this in practice: with un-normalized
    /// features, *every* loan regardless of actual payment behavior was
    /// classified `Default`, because a handful of large-magnitude
    /// days-late values dominated the dot product and pinned the output
    /// to whichever class the random weights happened to favor for large
    /// inputs. Normalizing bounds every feature's contribution
    /// comparably, which fixes that collapse for the untrained network
    /// and remains the right input scaling once the network is trained.
    pub fn to_vector(&self) -> [f32; FEATURE_DIM] {
        [
            (self.avg_days_late_recent / 90.0).clamp(0.0, 1.0),
            (self.max_days_late_6 / 180.0).clamp(0.0, 1.0),
            (self.missed_count_6 / 6.0).clamp(0.0, 1.0),
            (self.on_time_count_6 / 6.0).clamp(0.0, 1.0),
            (self.payment_ratio_recent / 2.0).clamp(0.0, 1.0),
            (self.collateral_utilization / 3.0).clamp(0.0, 1.0),
            self.loan_age_fraction.clamp(0.0, 1.0),
            self.network_contagion_score.clamp(0.0, 1.0),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_vector_stays_bounded_even_for_extreme_unbounded_inputs() {
        let extreme = PaymentFeatures {
            avg_days_late_recent: 5000.0,
            max_days_late_6: 5000.0,
            missed_count_6: 6.0,
            on_time_count_6: 0.0,
            payment_ratio_recent: 0.0,
            collateral_utilization: 3.0,
            loan_age_fraction: 1.0,
            network_contagion_score: 1.0,
        };
        for v in extreme.to_vector() {
            assert!((0.0..=1.0).contains(&v), "feature value {v} escaped [0, 1]");
        }
    }

    #[test]
    fn to_vector_is_zero_for_a_healthy_loan_with_no_late_payments() {
        let healthy = PaymentFeatures {
            avg_days_late_recent: 0.0,
            max_days_late_6: 0.0,
            missed_count_6: 0.0,
            on_time_count_6: 0.0,
            payment_ratio_recent: 0.0,
            collateral_utilization: 0.0,
            loan_age_fraction: 0.0,
            network_contagion_score: 0.0,
        };
        assert_eq!(healthy.to_vector(), [0.0; FEATURE_DIM]);
    }

    #[test]
    fn on_time_count_normalizes_to_full_scale() {
        let f = PaymentFeatures {
            avg_days_late_recent: 0.0,
            max_days_late_6: 0.0,
            missed_count_6: 0.0,
            on_time_count_6: 6.0,
            payment_ratio_recent: 1.0,
            collateral_utilization: 0.0,
            loan_age_fraction: 0.0,
            network_contagion_score: 0.0,
        };
        assert_eq!(f.to_vector()[3], 1.0);
    }
}
