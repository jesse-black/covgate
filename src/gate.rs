use anyhow::Result;

use crate::model::{ComputedMetric, GateRule, GateScopeResult, RuleOutcome};

pub fn evaluate(
    label: Option<String>,
    metrics: Vec<ComputedMetric>,
    rules: &[GateRule],
) -> Result<GateScopeResult> {
    let mut outcomes = Vec::new();
    let mut all_passed = true;

    for rule in rules {
        let metric = if let Some(metric) = metrics.iter().find(|m| m.metric == rule.metric()) {
            metric
        } else {
            let base = format!(
                "configured rule for {} is not supported by the loaded report",
                rule.metric().as_str()
            );
            if let Some(label) = &label {
                anyhow::bail!("{base} in gate `{label}`");
            }
            anyhow::bail!("{base}");
        };

        let rule_passed = match rule {
            GateRule::Percent {
                minimum_percent,
                metric: _,
            } => metric.percent + f64::EPSILON >= *minimum_percent,
            GateRule::UncoveredCount {
                maximum_count,
                metric: _,
            } => metric.uncovered_changed_opportunities.len() <= *maximum_count,
        };

        if !rule_passed {
            all_passed = false;
        }

        outcomes.push(RuleOutcome {
            rule: rule.clone(),
            passed: rule_passed,
            observed_percent: metric.percent,
            observed_uncovered_count: metric.uncovered_changed_opportunities.len(),
        });
    }

    Ok(GateScopeResult {
        label,
        metrics,
        rules: outcomes,
        passed: all_passed,
    })
}
