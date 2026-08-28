//! Sequential Temperature Scaling (STS) calibration for the Confidence Head.
//!
//! Adapted from the DSpark confidence-scheduled verification framework
//! (arXiv:2607.05147, Section 3.2.1 "Post-hoc Calibration"). Neural confidence
//! estimates are typically overconfident; raw scores distort the scheduler's
//! expected-acceptance accounting. STS calibrates the *cumulative prefix survival
//! product* `a_k = prod_{i<=k} c_i` position by position, left to right: at each
//! position k a 1-D grid search picks the temperature t_k minimizing the Expected
//! Calibration Error (ECE) of the temperature-scaled cumulative product against
//! observed prefix-survival labels, keeping the already-calibrated preceding
//! positions fixed.
//!
//! The transform is standard logit temperature scaling `q = sigmoid(logit(p)/T)`
//! (T > 1 flattens overconfident probabilities; T = 1 is the identity). It is
//! order-preserving (strictly monotonic in p), so calibration fixes the absolute
//! magnitudes the scheduler needs without disturbing the rankings learned by the
//! confidence head.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StsCalibrator {
    /// Temperature per position (1-indexed, t_k for k = 1..=gamma).
    pub temperatures: Vec<f64>,
}

/// Logit temperature scaling: sigmoid(logit(p) / T), clipped at the boundaries.
fn logit_scale(p: f64, t: f64) -> f64 {
    if p <= 0.0 {
        return 0.0;
    }
    if p >= 1.0 {
        return 1.0;
    }
    let logit = (p / (1.0 - p)).ln();
    1.0 / (1.0 + (-logit / t.max(f64::EPSILON)).exp())
}

impl StsCalibrator {
    pub fn new(temperatures: Vec<f64>) -> Self {
        Self { temperatures }
    }

    /// Fits per-position temperatures on (confidence, outcome) pairs.
    ///
    /// `samples[i]` are the raw per-position confidences of sample i and
    /// `outcomes[i][k-1]` whether the draft prefix through position k survived
    /// verification (accepted). The default grid mirrors the paper's 1-D sweep.
    pub fn fit(
        samples: &[Vec<f64>],
        outcomes: &[Vec<bool>],
        temp_grid: Option<&[f64]>,
    ) -> StsCalibrator {
        let default_grid = [0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 5.0, 8.0];
        let grid = temp_grid.unwrap_or(&default_grid);
        let gamma = samples.iter().map(|s| s.len()).max().unwrap_or(0);
        let mut temperatures = Vec::with_capacity(gamma);

        // Calibrated cumulative products A_k, recomputed with already-fixed temps.
        let mut calibrated_cumulative: Vec<f64> = vec![1.0; samples.len()];

        for k in 1..=gamma {
            // Raw cumulative survival through position k:
            // A_k = A_{k-1}' * c_k, keeping the calibrated prefix fixed.
            let raw_a: Vec<f64> = samples
                .iter()
                .enumerate()
                .map(|(s, sample)| {
                    let c_k = *sample.get(k - 1).unwrap_or(&1.0);
                    calibrated_cumulative[s] * c_k.clamp(0.0, 1.0)
                })
                .collect();
            let labels: Vec<f64> = outcomes
                .iter()
                .map(|o| if o.get(k - 1).copied().unwrap_or(false) { 1.0 } else { 0.0 })
                .collect();

            let best_t = grid
                .iter()
                .min_by(|a, b| {
                    let ece_a = expected_calibration_error(&scale(&raw_a, **a), &labels, 15);
                    let ece_b = expected_calibration_error(&scale(&raw_a, **b), &labels, 15);
                    ece_a.partial_cmp(&ece_b).unwrap_or(std::cmp::Ordering::Equal)
                })
                .copied()
                .unwrap_or(1.0);

            for (s, a) in raw_a.iter().enumerate() {
                calibrated_cumulative[s] = logit_scale(*a, best_t);
            }
            temperatures.push(best_t);
        }

        StsCalibrator { temperatures }
    }

    /// Applies the calibrated temperatures left-to-right, returning the calibrated
    /// conditional confidences c'_k = A_k' / A_{k-1}' (order-preserving transform).
    pub fn calibrate(&self, confidences: &[f64]) -> Vec<f64> {
        let mut calibrated: Vec<f64> = Vec::with_capacity(confidences.len());
        let mut cumulative = 1.0f64;
        for (k, &c) in confidences.iter().enumerate() {
            let t = self.temperatures.get(k).copied().unwrap_or(1.0);
            let raw_a = cumulative * c.clamp(0.0, 1.0);
            let new_cumulative = logit_scale(raw_a, t);
            calibrated.push((new_cumulative / cumulative.max(f64::EPSILON)).clamp(0.0, 1.0));
            cumulative = new_cumulative;
        }
        calibrated
    }
}

/// Applies temperature t to a batch of probabilities (logit temperature scaling).
fn scale(probs: &[f64], t: f64) -> Vec<f64> {
    probs.iter().map(|p| logit_scale(*p, t)).collect()
}

/// Expected Calibration Error with equal-width confidence bins.
fn expected_calibration_error(probs: &[f64], labels: &[f64], bins: usize) -> f64 {
    if probs.is_empty() {
        return 0.0;
    }
    let mut ece = 0.0;
    for b in 0..bins {
        let lo = b as f64 / bins as f64;
        let hi = (b + 1) as f64 / bins as f64;
        let mut sum_conf = 0.0;
        let mut sum_acc = 0.0;
        let mut count = 0.0;
        for (p, y) in probs.iter().zip(labels) {
            if *p >= lo && (*p < hi || (b + 1 == bins && *p <= hi)) {
                sum_conf += p;
                sum_acc += y;
                count += 1.0;
            }
        }
        if count > 0.0 {
            ece += (count / probs.len() as f64) * ((sum_acc - sum_conf) / count).abs();
        }
    }
    ece
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calibration_reduces_ece_on_overconfident_estimates() {
        // True prefix survival is 0.5 at every position; the raw head is overconfident.
        let mut rng: u64 = 0x9E3779B97F4A7C15;
        let mut rand = || {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            (rng as f64) / (u64::MAX as f64)
        };
        let n = 2000usize;
        let gamma = 4usize;
        let samples: Vec<Vec<f64>> = (0..n)
            .map(|_| (0..gamma).map(|_| 0.85 + 0.05 * rand()).collect())
            .collect();
        let outcomes: Vec<Vec<bool>> = samples
            .iter()
            .map(|_| (0..gamma).map(|_| rand() < 0.5).collect())
            .collect();

        let raw_labels: Vec<f64> = outcomes.iter().map(|o| if o[0] { 1.0 } else { 0.0 }).collect();
        let raw_ece = expected_calibration_error(
            &samples.iter().map(|s| s[0]).collect::<Vec<_>>(),
            &raw_labels,
            15,
        );

        let calibrator = StsCalibrator::fit(&samples, &outcomes, None);
        let calibrated: Vec<Vec<f64>> = samples.iter().map(|s| calibrator.calibrate(s)).collect();
        let cal_labels: Vec<f64> = outcomes.iter().map(|o| if o[0] { 1.0 } else { 0.0 }).collect();
        let cal_ece = expected_calibration_error(
            &calibrated.iter().map(|s| s[0]).collect::<Vec<_>>(),
            &cal_labels,
            15,
        );

        assert!(
            cal_ece < raw_ece,
            "STS must reduce ECE on overconfident estimates: raw {raw_ece} vs calibrated {cal_ece}"
        );
        assert!(raw_ece > 0.2, "sanity: raw head must actually be overconfident");
    }

    #[test]
    fn calibration_is_order_preserving() {
        let calibrator = StsCalibrator::new(vec![2.0, 1.5, 0.75]);
        let calibrated = calibrator.calibrate(&[0.9, 0.6, 0.8]);
        for v in calibrated {
            assert!((0.0..=1.0).contains(&v));
        }
        let single = StsCalibrator::new(vec![2.0]);
        let c = single.calibrate(&[0.9, 0.7]);
        assert!(c[0] > c[1], "order-preserving: 0.9 stays above 0.7");
    }

    #[test]
    fn identity_temperature_preserves_confidences() {
        let calibrator = StsCalibrator::new(vec![1.0, 1.0, 1.0]);
        let calibrated = calibrator.calibrate(&[0.9, 0.6, 0.8]);
        for (a, b) in calibrated.iter().zip([0.9, 0.6, 0.8]) {
            assert!((a - b).abs() < 1e-9);
        }
    }

    #[test]
    fn t_above_one_flattens_overconfidence() {
        let calibrated = StsCalibrator::new(vec![2.0]).calibrate(&[0.9]);
        assert!(calibrated[0] < 0.9 && calibrated[0] > 0.5);
    }
}
