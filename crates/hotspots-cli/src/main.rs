//! Command-line entry point for Git history maintenance metrics.

mod args;

use std::fs::File;
use std::io::{self, BufReader, Write};
use std::process::ExitCode;

use hotspots::{analyze_log, write_table};

use crate::args::{help_text, parse_args};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            let _ = writeln!(io::stderr(), "error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let argv: Vec<String> = std::env::args().collect();
    let parsed = parse_args(&argv)?;
    if parsed.help {
        #[expect(
            clippy::print_stdout,
            reason = "CLI help is intentionally written to stdout"
        )]
        {
            println!("{}", help_text());
        }
        return Ok(());
    }
    let file = File::open(&parsed.log).map_err(|e| format!("cannot open log: {e}"))?;
    let mut reader = BufReader::new(file);
    let table =
        analyze_log(&mut reader, &parsed.vcs, &parsed.options).map_err(|e| e.to_string())?;
    let mut stdout = io::stdout().lock();
    write_table(&mut stdout, &table).map_err(|e| e.to_string())?;
    Ok(())
}
