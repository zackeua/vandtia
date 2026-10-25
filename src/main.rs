use argparse::{ArgumentParser, Store};
use vandtia::simulate;

fn main() {
    let mut games = 1_000;
    let mut players = 4;
    let mut seed = 1;

    {
        let mut parser = ArgumentParser::new();
        parser.set_description("Simulerar partier Vändtia.");
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

    if let Err(error) = run(games, players, seed) {
        eprintln!("Fel: {error}");
        std::process::exit(2);
    }
}

fn run(games: usize, players: usize, seed: u64) -> Result<(), Box<dyn std::error::Error>> {
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
