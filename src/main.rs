use std::io::{self, Write};

use argparse::{ArgumentParser, Store, StoreTrue};
use vandtia::{
    Card, Game, PlayerPhase, TurnOutcome,
    simulator::{play_computer_turn, simulate},
};

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
            "Spela interaktivt mot datorstyrda motståndare",
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
        play_game(players, seed)
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

fn play_game(players: usize, seed: u64) -> Result<(), Box<dyn std::error::Error>> {
    let mut game = Game::new(players, seed)?;
    println!("Du är spelare 1. De övriga spelarna styrs av datorn.");
    println!("Skriv q när som helst vid en fråga för att avsluta.\n");

    while game.winner().is_none() {
        let player = game.current_player();
        display_table(&game, player);

        let outcome = if player == 0 {
            let Some(outcome) = human_turn(&mut game)? else {
                println!("Avslutar spelet.");
                return Ok(());
            };
            outcome
        } else {
            let outcome = play_computer_turn(&mut game)?;
            if !outcome.played_cards.is_empty() {
                println!(
                    "Dator {} spelar {}.",
                    player + 1,
                    format_cards(&outcome.played_cards)
                );
            } else if let Some(card) = outcome.played {
                println!("Dator {} vänder {}.", player + 1, format_card(card));
            } else {
                println!("Dator {} plockar upp högen.", player + 1);
            }
            outcome
        };

        display_outcome(&outcome);
        println!();
    }

    if game.winner() == Some(0) {
        println!("Du vann!");
    } else {
        println!("Spelare {} vann.", game.winner().unwrap() + 1);
    }
    Ok(())
}

fn display_table(game: &Game, player: usize) {
    println!("Draghög: {} kort kvar.", game.draw_pile_size());
    let top = game.pile().last().copied();
    match top {
        Some(card) => println!(
            "Hög: {} kort, överst {}. Drag: spelare {}.",
            game.pile().len(),
            format_card(card),
            player + 1
        ),
        None => println!("Högen är tom. Drag: spelare {}.", player + 1),
    }

    println!("Öppna kort och handkort:");
    for seat in 0..game.player_count() {
        let open_cards = game.face_up(seat).expect("player exists");
        let open_cards = if open_cards.is_empty() {
            "inga".to_owned()
        } else {
            open_cards
                .iter()
                .map(|card| format_card(*card))
                .collect::<Vec<_>>()
                .join(", ")
        };
        println!(
            "  Spelare {}: {} kort på handen; öppna: {}",
            seat + 1,
            game.hand_count(seat).expect("player exists"),
            open_cards
        );
    }

    if player != 0 {
        return;
    }
    match game.player_phase(player).expect("current player exists") {
        PlayerPhase::Hand => {
            let hand = game.hand(player).expect("player exists");
            let legal = game.legal_hand_indices();
            println!("Din hand:");
            for (index, card) in hand.iter().enumerate() {
                let marker = if legal.contains(&index) {
                    ""
                } else {
                    " (ogiltigt)"
                };
                println!("  {}: {}{}", index + 1, format_card(*card), marker);
            }
        }
        PlayerPhase::FaceUp => {
            let cards = game.face_up(player).expect("player exists");
            let legal = game.legal_face_up_indices();
            println!("Dina öppna kort:");
            for (index, card) in cards.iter().enumerate() {
                let marker = if legal.contains(&index) {
                    ""
                } else {
                    " (ogiltigt)"
                };
                println!("  {}: {}{}", index + 1, format_card(*card), marker);
            }
        }
        PlayerPhase::FaceDown => {
            println!(
                "Du har {} blinda kort kvar.",
                game.face_down_count(player).expect("player exists")
            );
        }
    }
}

fn human_turn(game: &mut Game) -> Result<Option<TurnOutcome>, Box<dyn std::error::Error>> {
    let player = game.current_player();
    let phase = game.player_phase(player).expect("current player exists");
    let (prompt, card_count, legal_indices) = match phase {
        PlayerPhase::Hand => (
            "Välj kortnummer (separera flera av samma valör med komma)",
            game.hand(player).expect("player exists").len(),
            game.legal_hand_indices(),
        ),
        PlayerPhase::FaceUp => (
            "Välj kortnummer (separera flera av samma valör med komma)",
            game.face_up(player).expect("player exists").len(),
            game.legal_face_up_indices(),
        ),
        PlayerPhase::FaceDown => (
            "Välj ett blint kortnummer att vända",
            game.face_down_count(player).expect("player exists"),
            Vec::new(),
        ),
    };

    loop {
        let can_draw = phase == PlayerPhase::Hand && game.draw_pile_size() > 0;
        if phase != PlayerPhase::FaceDown && legal_indices.is_empty() && !can_draw {
            println!("Du har inget giltigt kort att spela och måste plocka upp högen.");
            return Ok(Some(game.pick_up_pile()?));
        }

        let can_pick_up = !game.pile().is_empty() && phase != PlayerPhase::FaceDown;
        let mut choices = Vec::new();
        if !legal_indices.is_empty() || phase == PlayerPhase::FaceDown {
            choices.push(format!("1-{card_count}"));
        }
        if can_draw {
            choices.push("d för att dra ett kort".to_owned());
        }
        if can_pick_up {
            choices.push("p för att plocka upp".to_owned());
        }
        choices.push("q avslutar".to_owned());
        let Some(input) = prompt_line(&format!("{prompt} ({}): ", choices.join(", ")))? else {
            return Ok(None);
        };
        if input.eq_ignore_ascii_case("q") {
            return Ok(None);
        }
        if input.eq_ignore_ascii_case("p") && can_pick_up {
            return Ok(Some(game.pick_up_pile()?));
        }
        if input.eq_ignore_ascii_case("d") && can_draw {
            let outcome = game.draw_and_play()?;
            if let Some(card) = outcome.drawn {
                if outcome.played.is_some() {
                    println!("Du drar och spelar {}.", format_card(card));
                } else {
                    println!("Du drar {}, men kortet kan inte spelas.", format_card(card));
                }
            }
            return Ok(Some(outcome));
        }
        let Some(indices) = parse_card_indices(&input, card_count) else {
            println!("Ange kortnummer, separerade med kommatecken om du vill spela flera.");
            continue;
        };
        if indices
            .iter()
            .enumerate()
            .any(|(i, index)| indices[..i].contains(index))
        {
            println!("Samma kortnummer kan inte väljas flera gånger.");
            continue;
        }
        if indices.len() > 1 && phase == PlayerPhase::FaceDown {
            println!("Blinda kort vänds ett i taget.");
            continue;
        }
        if indices.iter().any(|index| !legal_indices.contains(index))
            && phase != PlayerPhase::FaceDown
        {
            println!("Ett eller flera av korten kan inte spelas nu.");
            continue;
        }
        let cards = match phase {
            PlayerPhase::Hand => game.hand(player).expect("player exists"),
            PlayerPhase::FaceUp => game.face_up(player).expect("player exists"),
            PlayerPhase::FaceDown => &[],
        };
        if indices.len() > 1
            && indices
                .iter()
                .any(|index| cards[*index].rank != cards[indices[0]].rank)
        {
            println!("Du kan bara spela flera kort av samma valör.");
            continue;
        }
        let outcome = match phase {
            PlayerPhase::Hand => game.play_hand_indices(&indices),
            PlayerPhase::FaceUp => game.play_face_up_indices(&indices),
            PlayerPhase::FaceDown => game.play_face_down_at(indices[0]),
        }?;
        if phase == PlayerPhase::FaceDown
            && let Some(card) = outcome.played
        {
            println!("Du vände {}.", format_card(card));
        } else if outcome.played_cards.len() > 1 {
            println!("Du spelar {}.", format_cards(&outcome.played_cards));
        }
        return Ok(Some(outcome));
    }
}

fn parse_card_indices(input: &str, card_count: usize) -> Option<Vec<usize>> {
    let numbers: Vec<usize> = input
        .split(',')
        .map(|part| part.trim().parse::<usize>().ok())
        .collect::<Option<Vec<_>>>()?;
    if numbers
        .iter()
        .any(|number| *number == 0 || *number > card_count)
    {
        return None;
    }
    Some(numbers.into_iter().map(|number| number - 1).collect())
}

fn prompt_line(prompt: &str) -> io::Result<Option<String>> {
    print!("{prompt}");
    io::stdout().flush()?;
    let mut input = String::new();
    if io::stdin().read_line(&mut input)? == 0 {
        return Ok(None);
    }
    Ok(Some(input.trim().to_owned()))
}

fn display_outcome(outcome: &TurnOutcome) {
    if outcome.burned {
        println!("Högen vänds bort! Spelaren får spela igen.");
    } else if outcome.extra_turn {
        println!("Tvåa spelad! Spelaren får spela igen.");
    }
    if outcome.picked_up > 0 {
        println!("{} kort plockades upp.", outcome.picked_up);
    }
}

fn format_card(card: Card) -> String {
    let rank = match card.rank {
        vandtia::Rank::Two => "2",
        vandtia::Rank::Three => "3",
        vandtia::Rank::Four => "4",
        vandtia::Rank::Five => "5",
        vandtia::Rank::Six => "6",
        vandtia::Rank::Seven => "7",
        vandtia::Rank::Eight => "8",
        vandtia::Rank::Nine => "9",
        vandtia::Rank::Ten => "10",
        vandtia::Rank::Jack => "knekt",
        vandtia::Rank::Queen => "dam",
        vandtia::Rank::King => "kung",
        vandtia::Rank::Ace => "ess",
    };
    let suit = match card.suit {
        vandtia::Suit::Clubs => "klöver",
        vandtia::Suit::Diamonds => "ruter",
        vandtia::Suit::Hearts => "hjärter",
        vandtia::Suit::Spades => "spader",
    };
    format!("{rank} i {suit}")
}

fn format_cards(cards: &[Card]) -> String {
    cards
        .iter()
        .map(|card| format_card(*card))
        .collect::<Vec<_>>()
        .join(" och ")
}

#[cfg(test)]
mod tests {
    use super::parse_card_indices;

    #[test]
    fn accepts_first_and_last_one_based_card_numbers() {
        assert_eq!(parse_card_indices("1", 3), Some(vec![0]));
        assert_eq!(parse_card_indices("3", 3), Some(vec![2]));
    }

    #[test]
    fn accepts_a_list_of_card_numbers() {
        assert_eq!(parse_card_indices("1, 3", 3), Some(vec![0, 2]));
    }

    #[test]
    fn rejects_card_numbers_outside_the_hand() {
        assert_eq!(parse_card_indices("0", 3), None);
        assert_eq!(parse_card_indices("4", 3), None);
    }
}
