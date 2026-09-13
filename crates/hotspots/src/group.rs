//! Optional architectural layer mapping for entities.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::error::{Error, Result};
use crate::model::Change;

/// Maps path prefixes to architectural layer names.
#[derive(Debug, Clone, Default)]
pub struct LayerMap {
    rules: Vec<(String, String)>,
}

impl LayerMap {
    /// Loads rules from a text file of `prefix => layer` lines.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read or a line is malformed.
    pub fn load(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut rules = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if let Some(rule) = parse_rule(&line)? {
                rules.push(rule);
            }
        }
        Ok(Self { rules })
    }

    /// Builds a map from already-parsed prefix/layer pairs.
    #[must_use]
    pub const fn from_rules(rules: Vec<(String, String)>) -> Self {
        Self { rules }
    }

    /// Rewrites each change entity to its layer when a prefix matches.
    #[must_use]
    pub fn apply(&self, mut changes: Vec<Change>) -> Vec<Change> {
        for change in &mut changes {
            if let Some(layer) = self.layer_for(&change.entity) {
                change.entity = layer;
            }
        }
        changes
    }

    fn layer_for(&self, path: &str) -> Option<String> {
        self.rules
            .iter()
            .find(|(prefix, _)| matches_prefix(path, prefix))
            .map(|(_, layer)| layer.clone())
    }
}

fn parse_rule(line: &str) -> Result<Option<(String, String)>> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return Ok(None);
    }
    let Some((prefix, layer)) = trimmed.split_once("=>") else {
        return Err(Error::msg(format!(
            "group rule must look like `prefix => layer`: {line}"
        )));
    };
    let prefix = prefix.trim().trim_end_matches('/').to_owned();
    let layer = layer.trim().to_owned();
    if prefix.is_empty() || layer.is_empty() {
        return Err(Error::msg(format!("empty group rule: {line}")));
    }
    Ok(Some((prefix, layer)))
}

fn matches_prefix(path: &str, prefix: &str) -> bool {
    path == prefix || path.strip_prefix(prefix).is_some_and(|rest| rest.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_paths_to_layers() {
        let map = LayerMap::from_rules(vec![(String::from("src"), String::from("app"))]);
        let changes = map.apply(vec![Change::new(
            "r1",
            "Ada",
            "2024-01-01",
            "src/main.rs",
            Some(1),
            Some(0),
        )]);
        assert_eq!(changes[0].entity, "app");
    }
}
