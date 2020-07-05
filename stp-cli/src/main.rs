#[macro_use]
extern crate clap;

use clap::ArgMatches;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, prelude::*, ErrorKind};
use std::result;
use stp_core::frame_decoder::{self, FrameDecoder};

const PROG_NAME: &str = crate_name!();

struct CliError(Option<String>);
type Result = result::Result<(), CliError>;

fn main() {
    if let Err(CliError(o)) = run() {
        if let Some(msg) = o {
            io::stdout().flush().unwrap();
            eprintln!("\n{}: {}", PROG_NAME, msg);
        }
        std::process::exit(1);
    }
}

fn run() -> Result {
    let app_m = clap_app!(stp =>
        (version: crate_version!())
        (author: crate_authors!())
        (about: crate_description!())
        (@subcommand nibbles =>
            (about: "Displays trace data as nibbles")
            (@arg FILE: "STP file")
            (@arg bail: -b --bail "Stop on first error.")
            (@arg file_offsets: --("file-offsets") "Display offsets relative to the file.")
        )
        (@subcommand packets =>
            (about: "Displays STP packets")
            (@arg FILE: "STP file")
        )
    )
    .get_matches();

    match app_m.subcommand() {
        ("nibbles", Some(sub_m)) => nibbles(&app_m, sub_m),
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
            Err(e) => Err(CliError(Some(format!("{}: {}", e, path)))),
        },
        None => Ok(Box::new(io::stdin())),
    }
}

impl std::convert::From<frame_decoder::Error> for CliError {
    fn from(err: frame_decoder::Error) -> CliError {
        match err {
            // 'Stop' does not need to report anything.
            e if e.reason == frame_decoder::ErrorReason::Stop => CliError(None),
            e => CliError(Some(format!("{}", e))),
        }
    }
}

const BUF_SIZE: usize = 4 * 1024;

fn nibbles(_app_m: &ArgMatches, sub_m: &ArgMatches) -> Result {
    let mut input = get_input(sub_m)?;
    let bail = sub_m.is_present("bail");
    let file_offset = sub_m.is_present("file_offsets");
    let mut buf = [0; BUF_SIZE];
    let mut display = NibbleDisplay::new(bail, file_offset);
    let mut decoder = FrameDecoder::new(false, None);

    loop {
        match input.read(&mut buf) {
            Ok(0) => {
                decoder.finish(|r| display.display(r))?;
                print!("\n");
                break;
            }
            Ok(len) => decoder.decode(&buf[..len], |r| display.display(r))?,
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(CliError(Some(format!("{}", e)))),
        };
    }
    Ok(())
}

struct NibbleDisplay {
    offsets: HashMap<Option<u8>, usize>,
    cur_id: Option<u8>,
    cur_offset: usize,
    column: usize,
    bail: bool,
    file_offset: bool,
    first_stream: bool,
}

impl NibbleDisplay {
    fn new(bail: bool, file_offset: bool) -> NibbleDisplay {
        NibbleDisplay {
            offsets: HashMap::new(),
            cur_id: Some(0xFF), // Intentionally set to an invalid Stream ID.
            cur_offset: 0,
            column: 0,
            bail,
            file_offset,
            first_stream: true,
        }
    }

    fn display_data(&mut self, id: Option<u8>, data: u8, offset: usize) {
        if id != self.cur_id {
            self.offsets.insert(self.cur_id, self.cur_offset);
            self.cur_offset = *self.offsets.entry(id).or_insert(0);
            self.column = 0;
            self.cur_id = id;
            if self.first_stream {
                self.first_stream = false;
            } else {
                print!("\n\n");
            }
            match id {
                None => print!("Stream None:"),
                Some(id) => print!("Stream {:#X}:", id),
            }
        }

        if self.column % 16 == 0 {
            let o = if self.file_offset {
                offset
            } else {
                self.cur_offset * 2
            };
            print!("\n{:012X} |", o);
            self.column = 0;
        } else if self.column == 8 {
            print!(" ");
        }
        print!(" {:x} {:x}", data & 0xF, data >> 4);

        self.column += 1;
        self.cur_offset += 1;
    }

    fn display_error(&mut self, err: frame_decoder::Error) {
        print!("\n\n**{}\n", err);
        self.column = 0;
    }

    fn display(
        &mut self,
        r: frame_decoder::Result<frame_decoder::Data>,
    ) -> frame_decoder::Result<()> {
        match r {
            Ok(d) => {
                self.display_data(d.id, d.data, d.offset);
                Ok(())
            }
            Err(e) => {
                let offset = e.offset;
                self.display_error(e);
                if self.bail {
                    Err(frame_decoder::Error {
                        offset,
                        reason: frame_decoder::ErrorReason::Stop,
                    })
                } else {
                    Ok(())
                }
            }
        }
    }
}

fn packets(_app_m: &ArgMatches, sub_m: &ArgMatches) -> Result {
    let mut _input = get_input(sub_m)?;
    Ok(())
}
