//! Optimization registry and (from M2) the atomic apply/verify/rollback engine.

pub mod definitions;
pub mod engine;

use std::collections::HashSet;

use wexo_core::{Category, Optimization};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RegistryError {
    #[error("duplicate optimization id: {0}")]
    DuplicateId(String),
}

/// In-memory registry of known optimizations, validated once at startup.
#[derive(Debug, Default)]
pub struct Registry {
    items: Vec<Optimization>,
}

impl Registry {
    pub fn new(items: Vec<Optimization>) -> Result<Self, RegistryError> {
        let mut seen = HashSet::with_capacity(items.len());
        for item in &items {
            if !seen.insert(item.id.as_str()) {
                return Err(RegistryError::DuplicateId(item.id.clone()));
            }
        }
        Ok(Self { items })
    }

    pub fn empty() -> Self {
        Self { items: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Optimization> {
        self.items.iter()
    }

    pub fn by_category(&self, category: Category) -> impl Iterator<Item = &Optimization> {
        let category = Some(category);
        self.items
            .iter()
            .filter(move |item| Some(item.category) == category)
    }
}

/// Re-export the executable engine and registry.
pub use engine::{OptimizationEngine, OptimizationRegistry as ExecutableRegistry, ApplyResult, OptimizationResult, EngineError};

#[cfg(test)]
mod tests {
    use super::*;
    use wexo_core::{Applicability, RiskLevel, SnapshotSpec};

    fn sample(id: &str) -> Optimization {
        Optimization {
            id: id.into(),
            category: Category::Services,
            title: "Sample".into(),
            description: "Sample".into(),
            rationale: "Sample".into(),
            risk: RiskLevel::Low,
            requires_reboot: false,
            applies_to: Applicability::default(),
            snapshot: SnapshotSpec::default(),
            available: false,
        }
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let error = Registry::new(vec![sample("a"), sample("a")]).unwrap_err();
        assert_eq!(error, RegistryError::DuplicateId("a".into()));
    }

    #[test]
    fn registry_reports_size() {
        let registry = Registry::new(vec![sample("a"), sample("b")]).unwrap();
        assert_eq!(registry.len(), 2);
        assert!(!registry.is_empty());
    }
}