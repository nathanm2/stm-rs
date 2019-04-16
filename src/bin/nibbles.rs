use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{ErrorKind, Read};
use std::path::PathBuf;

#[derive(Debug)]
enum CliError {
    UsageError(String),
    OtherError(String),
}

use CliError::*;

impl Error for CliError {}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UsageError(msg) | OtherError(msg) => write!(f, "{}", msg),
        }
    }
}

type Result<T> = std::result::Result<T, CliError>;

impl std::convert::From<std::io::Error> for CliError {
    fn from(err: std::io::Error) -> CliError {
        OtherError(format!("{}", err))
    }
}

const DEFAULT_BUF_SIZE: usize = 4 * 1024;

fn process_stream<R: ?Sized>(reader: &mut R) -> Result<()>
where
    R: Read,
{
    let mut buf = [0; DEFAULT_BUF_SIZE];
    loop {
        let _len = match reader.read(&mut buf) {
            Ok(0) => return Ok(()),
            Ok(len) => len,
            Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(CliError::from(e)),
        };
    }
}

fn run() -> Result<()> {
    let paths: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();

    if paths.is_empty() {
        return Err(UsageError(format!("No files specified")))?;
    }

    for path in paths {
        let mut f = File::open(path)?;
        process_stream(&mut f)?;
    }

    Ok(())
}

const PROG_NAME: &str = "nibbles";

fn display_usage() {
    println!("usage: {} [OPTIONS] [FILE ...]", PROG_NAME);
}

fn main() {
    if let Err(err) = run() {
        match err {
            UsageError(msg) => {
                println!("{}: {}", PROG_NAME, msg);
                display_usage();
            }
            OtherError(msg) => {
                eprintln!("{}: {}", PROG_NAME, msg);
            }
        }
        std::process::exit(1);
    }
}
