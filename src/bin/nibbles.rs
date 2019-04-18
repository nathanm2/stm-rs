use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{self, ErrorKind, Read};
use stm::frame_decoder::{self, FrameConsumer, FrameDecoder};

// CliError *************************************************************************************

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

impl std::convert::From<io::Error> for CliError {
    fn from(err: io::Error) -> CliError {
        OtherError(format!("{}", err))
    }
}

impl std::convert::From<frame_decoder::Error> for CliError {
    fn from(err: frame_decoder::Error) -> CliError {
        OtherError(format!("{}", err))
    }
}

// NibbleDump **********************************************************************************

struct NibbleFormat {
    offsets: HashMap<Option<u8>, usize>,
    cur_id: Option<u8>, // Current Id
    cur_offset: usize,
    col: usize,
}

impl NibbleFormat {
    fn new() -> NibbleFormat {
        NibbleFormat {
            offsets: HashMap::new(),
            cur_id: Some(0xFF), // Intentionally set to an invalid Stream ID.
            cur_offset: 0,
            col: 0,
        }
    }
}

impl FrameConsumer for NibbleFormat {
    fn stream_byte(&mut self, id: Option<u8>, data: u8) {
        if id != self.cur_id {
            self.offsets.insert(self.cur_id, self.cur_offset);
            self.cur_offset = *self.offsets.entry(id).or_insert(0);
            self.col = 0;
            self.cur_id = id;
            match id {
                None => print!("\n\nStream None:"),
                Some(id) => print!("\n\nStream {:#X}:", id),
            }
        }

        if self.col % 16 == 0 {
            print!("\n{:012X} |", self.cur_offset * 2);
            self.col = 0;
        } else if self.col == 8 {
            print!(" ");
        }
        print!(" {:x} {:x}", data & 0xF, data >> 4);

        self.col += 1;
        self.cur_offset += 1;
    }
}

// Driver ***************************************************************************************

const DEFAULT_BUF_SIZE: usize = 4 * 1024;

fn process_stream<R>(
    reader: &mut R,
    decoder: &mut FrameDecoder,
    fmt: &mut NibbleFormat,
) -> Result<()>
where
    R: ?Sized + Read,
{
    let mut buf = [0; DEFAULT_BUF_SIZE];
    let mut total = 0;
    loop {
        let len = match reader.read(&mut buf) {
            Ok(0) => return Ok(()),
            Ok(len) => len,
            Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(CliError::from(e)),
        };
        decoder.decode(&buf[..len], fmt, total)?;
        total += len;
    }
}

fn run() -> Result<()> {
    let mut decoder = FrameDecoder::new();
    let mut fmt = NibbleFormat::new();
    let paths: Vec<String> = std::env::args().skip(1).collect();

    if paths.is_empty() {
        return Err(UsageError(format!("No files specified")))?;
    }

    for path in paths {
        let mut f = match File::open(&path) {
            Ok(f) => f,
            Err(err) => return Err(OtherError(format!("{}: {}", err, path))),
        };
        process_stream(&mut f, &mut decoder, &mut fmt)?;
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
