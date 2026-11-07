//! Safe rules engine for hardware-aware recommendations.
//!
//! M0 defines the serializable rule shape and validation. The expression
//! evaluator lands in M4.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    pub id: String,
    pub title: String,
    /// Safe expression, evaluated by our own parser in M4 (never `eval`).
    pub condition: String,
    pub optimization_id: String,
    pub priority: u8,
    pub reason: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RuleError {
    #[error("duplicate rule id: {0}")]
    DuplicateId(String),
    #[error("rule {id} has an empty condition")]
    EmptyCondition { id: String },
}

pub fn validate(rules: &[Rule]) -> Result<(), RuleError> {
    let mut seen = HashSet::with_capacity(rules.len());
    for rule in rules {
        if !seen.insert(rule.id.as_str()) {
            return Err(RuleError::DuplicateId(rule.id.clone()));
        }
        if rule.condition.trim().is_empty() {
            return Err(RuleError::EmptyCondition {
                id: rule.id.clone(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(id: &str, condition: &str) -> Rule {
        Rule {
            id: id.into(),
            title: "t".into(),
            condition: condition.into(),
            optimization_id: "o".into(),
            priority: 50,
            reason: "r".into(),
        }
    }

    #[test]
    fn valid_rules_pass() {
        validate(&[rule("a", "disk.kind == 'nvme'"), rule("b", "true")]).unwrap();
    }

    #[test]
    fn empty_conditions_are_rejected() {
        let error = validate(&[rule("a", "   ")]).unwrap_err();
        assert_eq!(
            error,
            RuleError::EmptyCondition {
                id: "a".to_string()
            }
        );
    }
}
