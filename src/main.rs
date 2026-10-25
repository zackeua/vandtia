use std::env;
use vandtia::simulate;

fn main() {
    if let Err(error) = run() {
        eprintln!("Fel: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = env::args().skip(1).collect();
    if arguments.first().map(String::as_str) != Some("simulate") {
        println!("Användning: cargo run -- simulate [antal_partier] [antal_spelare] [frö]");
        return Ok(());
    }

    let games = parse_argument(&arguments, 1, 1_000)?;
    let players = parse_argument(&arguments, 2, 4)?;
    let seed = parse_argument(&arguments, 3, 1)?;
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

fn parse_argument<T>(
    arguments: &[String],
    index: usize,
    default: T,
) -> Result<T, Box<dyn std::error::Error>>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + 'static,
{
    arguments
        .get(index)
        .map(|value| value.parse().map_err(Into::into))
        .unwrap_or(Ok(default))
}
