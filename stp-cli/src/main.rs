#[macro_use]
extern crate clap;

use clap::ArgMatches;
use std::fs::File;
use std::io::{self, prelude::*, ErrorKind};
use std::result;

const PROG_NAME: &str = crate_name!();

type Result = result::Result<(), String>;

fn main() {
    if let Err(message) = run() {
        eprintln!("{}: {}", PROG_NAME, message);
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

fn get_input(sub_m: &ArgMatches) -> result::Result<Box<dyn Read>, String> {
    match sub_m.value_of("FILE") {
        Some(path) => match File::open(path) {
            Ok(f) => Ok(Box::new(f)),
            Err(e) => Err(format!("{}: {}", e, path)),
        },
        None => Ok(Box::new(io::stdin())),
    }
}

const BUF_SIZE: usize = 4 * 1024;

fn streams(_app_m: &ArgMatches, sub_m: &ArgMatches) -> Result {
    let mut input = get_input(sub_m)?;
    let mut buf = [0; BUF_SIZE];
    let mut total = 0;

    loop {
        let len = match input.read(&mut buf) {
            Ok(0) => return Ok(()),
            Ok(len) => len,
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(format!("{}", e)),
        };
        total += len;
    }
}

fn packets(_app_m: &ArgMatches, sub_m: &ArgMatches) -> Result {
    let mut input = get_input(sub_m)?;
    Ok(())
}
