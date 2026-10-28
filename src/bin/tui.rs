#[path = "../tui.rs"]
mod tui;

use argparse::{ArgumentParser, Store};

fn main() {
    let mut players = 4;
    let mut seed = 1;
    {
        let mut parser = ArgumentParser::new();
        parser.set_description("Spela Vändtia i ett terminalgränssnitt.");
        parser.refer(&mut players).add_option(
            &["-p", "--players"],
            Store,
            "Antal spelare, 2-5 (standard: 4)",
        );
        parser
            .refer(&mut seed)
            .add_option(&["-s", "--seed"], Store, "Slumptalsfrö (standard: 1)");
        parser.parse_args_or_exit();
    }

    if let Err(error) = tui::run(players, seed) {
        eprintln!("Fel: {error}");
        std::process::exit(2);
    }
}
