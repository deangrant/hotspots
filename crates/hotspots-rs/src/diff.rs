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
    let ranges = hunk_range_text(line)?;
    hunk_from_ranges(ranges)
}

fn hunk_range_text(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("@@ ")?;
    let (ranges, _) = rest.split_once(" @@")?;
    Some(ranges)
}

fn hunk_from_ranges(ranges: &str) -> Option<Hunk> {
    let mut parts = ranges.split_whitespace();
    let old = parts.next()?;
    let new = parts.next()?;
    let (old_start, old_count) = parse_signed_range(old, '-')?;
    let (new_start, new_count) = parse_signed_range(new, '+')?;
    Some(Hunk {
        old_start,
        old_count,
        new_start,
        new_count,
    })
}

fn parse_signed_range(raw: &str, sign: char) -> Option<(u32, u32)> {
    parse_range(raw.strip_prefix(sign)?)
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

    fn assert_hunk(hunks: &[Hunk], old: (u32, u32), new: (u32, u32)) {
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].old_start, old.0);
        assert_eq!(hunks[0].old_count, old.1);
        assert_eq!(hunks[0].new_start, new.0);
        assert_eq!(hunks[0].new_count, new.1);
    }

    #[test]
    fn parses_hunk_headers() {
        assert_hunk(
            &parse_hunks("@@ -10,0 +12,3 @@ fn foo\n+a\n+b\n+c\n"),
            (10, 0),
            (12, 3),
        );
        assert_hunk(
            &parse_hunks("diff --git a/x b/x\n@@ -5 +7 @@\n"),
            (5, 1),
            (7, 1),
        );
    }
}
