#[macro_use]
extern crate clap;

fn main() {
    let matches = clap_app!(stp =>
        (version: "0.1")
        (author: "Nathan M. <nathanm2@gmail.com>")
        (about: "A tool for working with System Trace Protocol (STP) data.")
        (@subcommand nibbles =>
            (about: "Displays the nibble stream")
        )
    )
    .get_matches();
}
