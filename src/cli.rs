use anyhow::{Context, Result};
use std::path::PathBuf;

pub struct CliOptions {
    pub crate_root: PathBuf,
    pub output: Option<PathBuf>,
    pub stdout: bool,
}

pub fn parse_args() -> Result<CliOptions> {
    let mut crate_root = None;
    let mut output = None;
    let mut stdout = false;

    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_usage();
                std::process::exit(0);
            }
            "-r" | "--crate-root" => {
                let value = args.next().context("Missing value after --crate-root")?;
                crate_root = Some(PathBuf::from(value));
            }
            "-o" | "--output" => {
                let value = args.next().context("Missing value after --output")?;
                output = Some(PathBuf::from(value));
            }
            "--stdout" => {
                stdout = true;
            }
            value if value.starts_with('-') => {
                anyhow::bail!("Unknown argument: {}", value);
            }
            // Positional arguments are allowed as the crate_root shorthand
            value => {
                if crate_root.is_some() {
                    anyhow::bail!("Unexpected positional argument: {}", value);
                }
                crate_root = Some(PathBuf::from(value));
            }
        }
    }

    // --stdout and --output conflict: output is either file or stream, not both
    if stdout && output.is_some() {
        anyhow::bail!("--stdout cannot be combined with --output");
    }

    // In debug builds, default to the example crate for quick iteration;
    // in release, default to cwd so `cargo run` works as expected
    let crate_root = match crate_root {
        Some(value) => value,
        None if cfg!(debug_assertions) => PathBuf::from("example"),
        None => std::env::current_dir().context("Failed to resolve current directory")?,
    };

    let output = match (stdout, output) {
        (true, output) => output,
        (false, Some(output)) => Some(output),
        // In debug mode, write to web/ for the frontend dev server by default
        (false, None) if cfg!(debug_assertions) => Some(PathBuf::from("web/mdoc.json")),
        (false, None) => None,
    };

    Ok(CliOptions {
        crate_root,
        output,
        stdout,
    })
}

pub fn print_usage() {
    eprintln!("Usage: cargo-mdoc [crate-root] [--output PATH] [--stdout]");
    eprintln!("       cargo-mdoc --crate-root PATH [--output PATH] [--stdout]");
}
