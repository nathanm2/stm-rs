use std::error::Error;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug)]
enum NibblesError {
    UsageError(String),
    OtherError(String),
}

impl Error for NibblesError {}

impl fmt::Display for NibblesError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UsageError(msg) | OtherError(msg) => write!(f, "{}", msg),
        }
    }
}

use NibblesError::*;

fn display_usage() {
    println!("usage: nibbles [OPTIONS] [FILE ...]");
}

fn run() -> Result<(), NibblesError> {
    let files: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();

    if files.is_empty() {
        return Err(OtherError(format!("No files specified")))?;
    }

    Ok(())
}

fn main() {
    if let Err(err) = run() {
        match err {
            UsageError(msg) => {
                println!("nibbles: {}", msg);
                display_usage();
            }
            OtherError(msg) => {
                eprintln!("nibbles: {}", msg);
            }
        }
        std::process::exit(1);
    }
}
