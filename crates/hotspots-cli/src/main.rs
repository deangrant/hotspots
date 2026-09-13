//! Command-line entry point for Git history maintenance metrics.

mod args;

use std::fs::File;
use std::io::{self, BufReader, Write};
use std::process::ExitCode;

use hotspots::{analyze_log, write_table};

use crate::args::{Args, help_text, parse_args};

fn main() -> ExitCode {
    exit_from(run_with_args(&std::env::args().collect::<Vec<_>>()))
}

fn exit_from(result: Result<(), String>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            let _ = writeln!(io::stderr(), "error: {message}");
            ExitCode::FAILURE
        }
    }
}

/// Runs the CLI against an explicit argv slice (testable entry point).
///
/// # Errors
///
/// Returns a human-readable message when parsing or analysis fails.
pub fn run_with_args(argv: &[String]) -> Result<(), String> {
    let parsed = parse_args(argv)?;
    if parsed.help {
        print_help();
        return Ok(());
    }
    execute(&parsed)
}

fn print_help() {
    #[expect(
        clippy::print_stdout,
        reason = "CLI help is intentionally written to stdout"
    )]
    {
        println!("{}", help_text());
    }
}

fn execute(parsed: &Args) -> Result<(), String> {
    let file = File::open(&parsed.log).map_err(|e| format!("cannot open log: {e}"))?;
    let mut reader = BufReader::new(file);
    let table =
        analyze_log(&mut reader, &parsed.vcs, &parsed.options).map_err(|e| e.to_string())?;
    let mut stdout = io::stdout().lock();
    write_table(&mut stdout, &table, parsed.format).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_runs_cleanly() {
        let argv = vec![String::from("hotspots"), String::from("--help")];
        assert!(run_with_args(&argv).is_ok());
    }

    #[test]
    fn analyzes_temp_log() {
        let path = std::env::temp_dir().join("hotspots-cli-test.log");
        let created = File::create(&path);
        assert!(created.is_ok(), "temp log create failed");
        let mut files: Vec<File> = created.ok().into_iter().collect();
        assert_eq!(files.len(), 1);
        let mut file = files.remove(0);
        let payload =
            b"--aaa--2024-01-01--Ada\n1\t0\tsrc/a.rs\n--bbb--2024-01-02--Bea\n1\t0\tsrc/a.rs\n";
        assert!(std::io::Write::write_all(&mut file, payload).is_ok());
        let argv = vec![
            String::from("hotspots"),
            String::from("-l"),
            path.to_string_lossy().into_owned(),
            String::from("-c"),
            String::from("git2"),
            String::from("-a"),
            String::from("summary"),
            String::from("-n"),
            String::from("1"),
        ];
        assert!(run_with_args(&argv).is_ok());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn bad_log_content_errors() {
        let path = std::env::temp_dir().join("hotspots-cli-bad.log");
        assert!(
            std::fs::write(&path, "1\t0\torphan.rs\n").is_ok(),
            "temp bad log write failed"
        );
        let argv = vec![
            String::from("hotspots"),
            String::from("-l"),
            path.to_string_lossy().into_owned(),
            String::from("-c"),
            String::from("git2"),
        ];
        assert!(run_with_args(&argv).is_err());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn missing_file_errors() {
        let argv = vec![
            String::from("hotspots"),
            String::from("-l"),
            String::from("/no/such/hotspots.log"),
            String::from("-c"),
            String::from("git2"),
        ];
        assert!(run_with_args(&argv).is_err());
    }
}
