use covgate::model::{
    CheckResult, ComputedMetric, GateEvaluation, GateRule, MetricKind, RuleOutcome,
};

pub fn single_scope_result(
    metrics: Vec<ComputedMetric>,
    rules: Vec<RuleOutcome>,
    passed: bool,
) -> CheckResult {
    CheckResult {
        gates: vec![GateEvaluation {
            label: None,
            rules,
            passed,
        }],
        changed_metrics: metrics.clone(),
        overall_metrics: metrics,
        passed,
    }
}

pub fn percent_outcome(
    metric: MetricKind,
    minimum_percent: f64,
    passed: bool,
    observed_percent: f64,
    covered: usize,
    total: usize,
) -> RuleOutcome {
    RuleOutcome {
        rule: GateRule::Percent {
            metric,
            minimum_percent,
        },
        passed,
        observed_percent,
        observed_covered_count: covered,
        observed_total_count: total,
        observed_uncovered_count: total.saturating_sub(covered),
    }
}

pub fn uncovered_outcome(
    metric: MetricKind,
    maximum_count: usize,
    passed: bool,
    observed_uncovered_count: usize,
) -> RuleOutcome {
    RuleOutcome {
        rule: GateRule::UncoveredCount {
            metric,
            maximum_count,
        },
        passed,
        observed_percent: 0.0,
        observed_covered_count: 0,
        observed_total_count: 0,
        observed_uncovered_count,
    }
}
