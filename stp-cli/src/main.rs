#[macro_use]
extern crate clap;

use clap::ArgMatches;

fn main() {
    let app_m = clap_app!(stp =>
        (version: crate_version!())
        (author: crate_authors!("\n"))
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

    let _rc = match app_m.subcommand() {
        ("streams", Some(sub_m)) => streams(&app_m, sub_m),
        ("packets", Some(sub_m)) => packets(&app_m, sub_m),
        _ => {
            println!("{}", app_m.usage());
            0
        }
    };
}

fn streams(_app_m: &ArgMatches, _sub_m: &ArgMatches) -> i32 {
    print!("Inside streams\n");
    0
}

fn packets(_app_m: &ArgMatches, _sub_m: &ArgMatches) -> i32 {
    print!("Inside packets\n");
    1
}
