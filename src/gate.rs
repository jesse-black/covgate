use anyhow::Result;

use crate::model::{ComputedMetric, GateResult, GateRule, RuleOutcome};

pub fn evaluate(metrics: Vec<ComputedMetric>, rules: &[GateRule]) -> Result<GateResult> {
    let mut outcomes = Vec::new();
    let mut all_passed = true;

    for rule in rules {
        let metric = metrics
            .iter()
            .find(|m| m.metric == rule.metric())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "configured rule for {} is not supported by the loaded report",
                    rule.metric().as_str()
                )
            })?;

        let rule_passed = match rule {
            GateRule::Percent {
                minimum_percent, ..
            } => metric.percent + f64::EPSILON >= *minimum_percent,
            GateRule::UncoveredCount { maximum_count, .. } => {
                metric.uncovered_changed_opportunities.len() <= *maximum_count
            }
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

    Ok(GateResult {
        metrics,
        rules: outcomes,
        passed: all_passed,
    })
}
