#[macro_use]
extern crate clap;

use clap::ArgMatches;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, prelude::*, ErrorKind};
use std::result;
use stp_core::frame_decoder::{self, decode_frames};

const PROG_NAME: &str = crate_name!();

struct CliError(String);

type Result = result::Result<(), CliError>;

fn main() {
    if let Err(cli_error) = run() {
        eprintln!("{}: {}", PROG_NAME, cli_error.0);
        std::process::exit(1);
    }
}

fn run() -> Result {
    let app_m = clap_app!(stp =>
        (version: crate_version!())
        (author: crate_authors!())
        (about: crate_description!())
        (@subcommand streams =>
            (about: "Displays the Trace Data streams")
            (@arg FILE: "STP file")
        )
        (@subcommand packets =>
            (about: "Displays the STP packets")
            (@arg FILE: "STP file")
        )
    )
    .get_matches();

    match app_m.subcommand() {
        ("streams", Some(sub_m)) => streams(&app_m, sub_m),
        ("packets", Some(sub_m)) => packets(&app_m, sub_m),
        _ => {
            println!("{}", app_m.usage());
            Ok(())
        }
    }
}

fn get_input(sub_m: &ArgMatches) -> result::Result<Box<dyn Read>, CliError> {
    match sub_m.value_of("FILE") {
        Some(path) => match File::open(path) {
            Ok(f) => Ok(Box::new(f)),
            Err(e) => Err(CliError(format!("{}: {}", e, path))),
        },
        None => Ok(Box::new(io::stdin())),
    }
}

const BUF_SIZE: usize = 4 * 1024;

impl std::convert::From<frame_decoder::Error> for CliError {
    fn from(err: frame_decoder::Error) -> CliError {
        CliError(format!("{}", err))
    }
}

fn streams(_app_m: &ArgMatches, sub_m: &ArgMatches) -> Result {
    let mut input = get_input(sub_m)?;
    let mut buf = [0; BUF_SIZE];
    let mut total = 0;
    let mut stream_id = None;
    let mut display = StreamDisplay::new();

    loop {
        let len = match input.read(&mut buf) {
            Ok(0) => return Ok(()),
            Ok(len) => len,
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(CliError(format!("{}", e))),
        };
        stream_id = decode_frames(
            &buf[..len],
            stream_id,
            |id, data| display.display(id, data),
            |mut e| {
                e.offset += total;
                Err(e)
            },
        )?;
        total += len;
    }
}

struct StreamDisplay {
    offsets: HashMap<Option<u8>, usize>,
    cur_id: Option<u8>,
    cur_offset: usize,
    col: usize,
}

impl StreamDisplay {
    fn new() -> StreamDisplay {
        StreamDisplay {
            offsets: HashMap::new(),
            cur_id: Some(0xFF), // Intentionally set to an invalid Stream ID.
            cur_offset: 0,
            col: 0,
        }
    }

    fn display(&mut self, id: Option<u8>, data: u8) {
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

fn packets(_app_m: &ArgMatches, sub_m: &ArgMatches) -> Result {
    let mut _input = get_input(sub_m)?;
    Ok(())
}
