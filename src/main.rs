mod tui;

use argparse::{ArgumentParser, Store, StoreTrue};
use vandtia::simulator::simulate;

fn main() {
    let mut play = false;
    let mut games = 1_000;
    let mut players = 4;
    let mut seed = 1;

    {
        let mut parser = ArgumentParser::new();
        parser.set_description("Spela eller simulera partier Vändtia.");
        parser.refer(&mut play).add_option(
            &["-i", "--play"],
            StoreTrue,
            "Spela interaktivt i terminalen",
        );
        parser.refer(&mut games).add_option(
            &["-n", "--games"],
            Store,
            "Antal partier att simulera (standard: 1000)",
        );
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

    let result = if play {
        tui::run(players, seed)
    } else {
        run_simulation(games, players, seed)
    };
    if let Err(error) = result {
        eprintln!("Fel: {error}");
        std::process::exit(2);
    }
}

fn run_simulation(
    games: usize,
    players: usize,
    seed: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let stats = simulate(players, games, seed)?;
    println!(
        "Simulerade {} partier med {} spelare.",
        stats.games, players
    );
    for (player, wins) in stats.wins.iter().enumerate() {
        println!("Spelare {} vann {} partier.", player + 1, wins);
    }
    println!("Genomsnittligt antal turer: {:.1}", stats.average_turns());
    Ok(())
}
