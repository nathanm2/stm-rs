#[macro_use]
extern crate clap;

use clap::ArgMatches;

fn main() {
    let app_m = clap_app!(stp =>
        (version: "0.1")
        (author: "Nathan M. <nathanm2@gmail.com>")
        (about: "A tool for working with System Trace Protocol (STP) data")
        (@subcommand nibbles =>
            (about: "Displays the nibble stream")
            (@arg FILE: "STP file")
        )
        (@subcommand packets =>
            (about: "Displays the STP packets")
            (@arg FILE: "STP file")
        )
    )
    .get_matches();

    match app_m.subcommand() {
        ("nibbles", Some(sub_m)) => nibbles(sub_m),
        ("packets", Some(sub_m)) => packets(sub_m),
        _ => {}
    }
}

fn nibbles(_sub_m: &ArgMatches) {
    print!("Inside nibbles\n");
}

fn packets(_sub_m: &ArgMatches) {
    print!("Inside packets\n");
}
