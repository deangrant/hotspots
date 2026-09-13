//! Tunables that control filtering and metric engines.

/// Optional same-day commit merging for coupling-style metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalPeriod {
    /// Keep each commit hash as its own changeset.
    None,
    /// Merge commits that share author and calendar date.
    Day,
}

/// Analysis selection and thresholds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// Named analysis to run.
    pub analysis: String,
    /// Maximum rows written after sorting.
    pub rows: Option<usize>,
    /// Minimum revisions before an entity is kept.
    pub min_revs: u64,
    /// Minimum shared revisions for a coupling pair.
    pub min_shared_revs: u64,
    /// Inclusive lower bound on coupling degree percent.
    pub min_coupling: u64,
    /// Inclusive upper bound on coupling degree percent.
    pub max_coupling: u64,
    /// Ignore changesets larger than this entity count.
    pub max_changeset_size: usize,
    /// Reference date for age (`YYYY-MM-DD`), if set.
    pub age_time_now: Option<String>,
    /// Temporal merge strategy.
    pub temporal_period: TemporalPeriod,
    /// Path prefixes that must match when non-empty.
    pub include: Vec<String>,
    /// Path prefixes that drop matching entities.
    pub exclude: Vec<String>,
    /// Optional layer map file path.
    pub group_file: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            analysis: String::from("risk"),
            rows: None,
            min_revs: 5,
            min_shared_revs: 5,
            min_coupling: 30,
            max_coupling: 100,
            max_changeset_size: 30,
            age_time_now: None,
            temporal_period: TemporalPeriod::None,
            include: Vec::new(),
            exclude: Vec::new(),
            group_file: None,
        }
    }
}
