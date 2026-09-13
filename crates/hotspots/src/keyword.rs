//! Keyword parsers for small closed enums.

/// Maps `raw` to a variant from `variants`, or returns a labeled error.
pub(crate) fn parse_keyword<T: Copy>(
    raw: &str,
    variants: &[(&str, T)],
    kind: &str,
    expected: &str,
) -> Result<T, String> {
    for &(name, value) in variants {
        if name == raw {
            return Ok(value);
        }
    }
    Err(format!("invalid {kind} `{raw}`; expected {expected}"))
}
