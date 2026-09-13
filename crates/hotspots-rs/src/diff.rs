//! Unified-diff hunk header parsing.

use hotspots::symbols::Hunk;

/// Parses `@@ -old,count +new,count @@` headers from a unified diff.
#[must_use]
pub fn parse_hunks(diff_text: &str) -> Vec<Hunk> {
    let mut hunks = Vec::new();
    for line in diff_text.lines() {
        if let Some(hunk) = parse_hunk_header(line) {
            hunks.push(hunk);
        }
    }
    hunks
}

fn parse_hunk_header(line: &str) -> Option<Hunk> {
    let rest = line.strip_prefix("@@ ")?;
    let (ranges, _) = rest.split_once(" @@")?;
    let mut parts = ranges.split_whitespace();
    let old = parts.next()?;
    let new = parts.next()?;
    let (old_start, old_count) = parse_range(old.strip_prefix('-')?)?;
    let (new_start, new_count) = parse_range(new.strip_prefix('+')?)?;
    Some(Hunk {
        old_start,
        old_count,
        new_start,
        new_count,
    })
}

fn parse_range(raw: &str) -> Option<(u32, u32)> {
    if let Some((start, count)) = raw.split_once(',') {
        return Some((start.parse().ok()?, count.parse().ok()?));
    }
    let start: u32 = raw.parse().ok()?;
    Some((start, 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_hunk_header() {
        let hunks = parse_hunks("@@ -10,0 +12,3 @@ fn foo\n+a\n+b\n+c\n");
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].old_start, 10);
        assert_eq!(hunks[0].old_count, 0);
        assert_eq!(hunks[0].new_start, 12);
        assert_eq!(hunks[0].new_count, 3);
    }

    #[test]
    fn parses_single_line_range_and_ignores_noise() {
        let hunks = parse_hunks("diff --git a/x b/x\n@@ -5 +7 @@\n");
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].old_start, 5);
        assert_eq!(hunks[0].old_count, 1);
        assert_eq!(hunks[0].new_start, 7);
        assert_eq!(hunks[0].new_count, 1);
    }
}
