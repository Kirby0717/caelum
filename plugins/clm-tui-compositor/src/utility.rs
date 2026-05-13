use clm_plugin_api::data::tui_layout::SizeRange;

pub fn distribute(constraints: &[(f64, SizeRange)], total: u16) -> Vec<u16> {
    assert!(
        constraints
            .iter()
            .all(|(w, _)| f64::EPSILON <= *w && w.is_finite())
    );
    let n = constraints.len();
    if n == 0 {
        return vec![];
    }

    let sum_lo = constraints.iter().map(|(_, r)| r.0 as u32).sum::<u32>();
    let sum_hi = constraints.iter().map(|(_, r)| r.1 as u32).sum::<u32>();
    let t = total as u32;

    if t < sum_lo {
        let lo: Vec<f64> = constraints.iter().map(|(_, r)| r.0 as f64).collect();
        weighted_round(total, &lo)
    } else if t > sum_hi {
        let hi: Vec<f64> = constraints.iter().map(|(_, r)| r.1 as f64).collect();
        weighted_round(total, &hi)
    } else {
        distribute_by_waterlevel(constraints, total)
    }
}

fn distribute_by_waterlevel(constraints: &[(f64, SizeRange)], total: u16) -> Vec<u16> {
    let h = find_waterlevel(constraints, total as f64);
    let ideals: Vec<f64> = constraints
        .iter()
        .map(|(w, r)| (w * h).clamp(r.0 as f64, r.1 as f64))
        .collect();
    round_largest_remainder(&ideals, total)
}

fn find_waterlevel(constraints: &[(f64, SizeRange)], target: f64) -> f64 {
    let f_at = |h: f64| -> f64 {
        constraints
            .iter()
            .map(|(w, r)| (w * h).clamp(r.0 as f64, r.1 as f64))
            .sum()
    };

    let mut lo = 0.0;
    let mut hi = constraints
        .iter()
        .map(|(w, r)| r.1 as f64 / w)
        .fold(0.0, f64::max);

    for _ in 0..100 {
        let mid = (hi + lo) / 2.0;
        if f_at(mid) < target {
            lo = mid;
        } else {
            hi = mid;
        }
    }

    (hi + lo) / 2.0
}

fn weighted_round(total: u16, weights: &[f64]) -> Vec<u16> {
    let sum_w = weights.iter().sum::<f64>();
    let ideals = weights
        .iter()
        .map(|&w| total as f64 * (w / sum_w))
        .collect::<Vec<_>>();
    round_largest_remainder(&ideals, total)
}

fn round_largest_remainder(ideals: &[f64], target: u16) -> Vec<u16> {
    let floors = ideals.iter().map(|&x| x.floor() as u16).collect::<Vec<_>>();
    let floor_sum = floors.iter().sum::<u16>();
    assert!(floor_sum <= target);
    let shortfall = target - floor_sum;

    let mut fracts = ideals
        .iter()
        .enumerate()
        .map(|(i, &x)| (x.fract(), i))
        .collect::<Vec<_>>();
    fracts.sort_unstable_by(|(a, _), (b, _)| f64::total_cmp(a, b));

    let mut result = floors;
    for (_, i) in fracts.into_iter().take(shortfall as usize) {
        result[i] += 1;
    }
    result
}
