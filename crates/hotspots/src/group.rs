//! Optional architectural layer mapping for entities.
//!
//! Rules are tried in file order; the first matching prefix wins. Put longer
//! or more specific prefixes before broader ones. Paths that match no rule keep
//! their original entity name, so layers and files can appear in one analysis.

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
    /// Matching uses file order: the first matching prefix wins. Unmatched
    /// paths are left unchanged when [`Self::apply`] runs.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read or a line is malformed.
    pub fn load(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let rules = read_rules(BufReader::new(file))?;
        Ok(Self { rules })
    }

    /// Builds a map from already-parsed prefix/layer pairs.
    #[must_use]
    pub const fn from_rules(rules: Vec<(String, String)>) -> Self {
        Self { rules }
    }

    /// Rewrites each change entity to its layer when a prefix matches.
    ///
    /// First matching rule in load order wins. Entities with no match are
    /// unchanged.
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

fn read_rules(reader: impl BufRead) -> Result<Vec<(String, String)>> {
    let mut rules = Vec::new();
    for line in reader.lines() {
        push_parsed_rule(&mut rules, &line?)?;
    }
    Ok(rules)
}

fn push_parsed_rule(rules: &mut Vec<(String, String)>, line: &str) -> Result<()> {
    if let Some(rule) = parse_rule(line)? {
        rules.push(rule);
    }
    Ok(())
}

fn parse_rule(line: &str) -> Result<Option<(String, String)>> {
    let trimmed = line.trim();
    if is_skippable(trimmed) {
        return Ok(None);
    }
    split_rule(trimmed, line)
}

fn is_skippable(trimmed: &str) -> bool {
    trimmed.is_empty() || trimmed.starts_with('#')
}

fn split_rule(trimmed: &str, line: &str) -> Result<Option<(String, String)>> {
    let Some((prefix, layer)) = trimmed.split_once("=>") else {
        return Err(Error::msg(format!(
            "group rule must look like `prefix => layer`: {line}"
        )));
    };
    let prefix = prefix.trim().trim_end_matches('/').to_owned();
    let layer = layer.trim().to_owned();
    ensure_nonempty_rule(&prefix, &layer, line)?;
    Ok(Some((prefix, layer)))
}

fn ensure_nonempty_rule(prefix: &str, layer: &str, line: &str) -> Result<()> {
    if prefix.is_empty() || layer.is_empty() {
        return Err(Error::msg(format!("empty group rule: {line}")));
    }
    Ok(())
}

fn matches_prefix(path: &str, prefix: &str) -> bool {
    path == prefix || path.strip_prefix(prefix).is_some_and(|rest| rest.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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

    #[test]
    fn load_accepts_comments_and_blank_lines() {
        let path = temp_rules("# comment\n\nsrc/ => app\n");
        let loaded = LayerMap::load(&path);
        assert!(loaded.is_ok_and(|map| {
            let changes = map.apply(vec![Change::new(
                "1",
                "Ada",
                "2024-01-01",
                "src/x.rs",
                None,
                None,
            )]);
            changes[0].entity == "app"
        }));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn load_rejects_bad_and_empty_rules() {
        let bad = temp_rules("not-a-rule\n");
        assert!(LayerMap::load(&bad).is_err());
        let _ = std::fs::remove_file(bad);
        let empty = temp_rules(" => \n");
        assert!(LayerMap::load(&empty).is_err());
        let _ = std::fs::remove_file(empty);
        let empty_layer = temp_rules("src =>\n");
        assert!(LayerMap::load(&empty_layer).is_err());
        let _ = std::fs::remove_file(empty_layer);
    }

    #[test]
    fn unmatched_paths_stay_unchanged() {
        let map = LayerMap::from_rules(vec![(String::from("src"), String::from("app"))]);
        let changes = map.apply(vec![Change::new(
            "1",
            "Ada",
            "2024-01-01",
            "docs/a.md",
            None,
            None,
        )]);
        assert_eq!(changes[0].entity, "docs/a.md");
    }

    #[test]
    fn first_matching_rule_wins_over_later_specific_prefix() {
        let map = LayerMap::from_rules(vec![
            (String::from("src"), String::from("A")),
            (String::from("src/api"), String::from("B")),
        ]);
        let changes = map.apply(vec![Change::new(
            "1",
            "Ada",
            "2024-01-01",
            "src/api/x.rs",
            None,
            None,
        )]);
        assert_eq!(changes[0].entity, "A");
    }

    fn temp_rules(contents: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "hotspots-group-{}-{}.txt",
            std::process::id(),
            contents.len()
        ));
        let _ = std::fs::write(&path, contents);
        path
    }
}
