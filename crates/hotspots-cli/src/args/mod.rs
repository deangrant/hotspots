//! Command-line argument parsing for the hotspots binary.

mod flags;
mod validate;

#[cfg(test)]
mod tests;

use hotspots::{Options, OutputFormat};

use self::flags::parse_flag;
use self::validate::validate;

/// Parsed CLI invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    /// Path to the VCS log file.
    pub log: String,
    /// VCS parser name (`git` or `git2`).
    pub vcs: String,
    /// Analysis options.
    pub options: Options,
    /// stdout encoding.
    pub format: OutputFormat,
    /// When true, print help and exit successfully.
    pub help: bool,
    /// Bitset of scalar flags already seen during parse (not include/exclude).
    pub(crate) seen_flags: u32,
}

/// Parses process arguments.
///
/// # Errors
///
/// Returns a human-readable message when flags are missing or invalid.
pub fn parse_args(argv: &[String]) -> Result<Args, String> {
    if wants_help(argv) {
        return Ok(help_args());
    }
    parse_required_args(argv)
}

fn wants_help(argv: &[String]) -> bool {
    argv.iter().any(|a| a == "-h" || a == "--help")
}

fn help_args() -> Args {
    Args {
        log: String::new(),
        vcs: String::new(),
        options: Options::default(),
        format: OutputFormat::Text,
        help: true,
        seen_flags: 0,
    }
}

fn parse_required_args(argv: &[String]) -> Result<Args, String> {
    let mut parsed = Args {
        log: String::new(),
        vcs: String::new(),
        options: Options::default(),
        format: OutputFormat::Text,
        help: false,
        seen_flags: 0,
    };
    let mut index = 1;
    while index < argv.len() {
        index = parse_flag(argv[index].as_str(), argv, index, &mut parsed)?;
    }
    validate(&parsed)?;
    Ok(parsed)
}
