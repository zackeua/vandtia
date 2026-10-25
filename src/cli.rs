use std::{
    error::Error,
    io::{self, Write},
};

use vandtia::{Card, Game, PlayerPhase, Rank, Suit, TurnOutcome, simulator::play_computer_turn};

pub fn run(player_count: usize, seed: u64) -> Result<(), Box<dyn Error>> {
    let mut game = Game::new(player_count, seed)?;
    println!("Du är spelare 1. De övriga spelarna styrs av datorn.");
    println!("Kommandon: kortnummer, flera kort som 1,3, d=chansa, p=plocka upp, q=avsluta.\n");

    while game.winner().is_none() {
        let player = game.current_player();
        display_table(&game);
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
    } else if let Some(winner) = game.winner() {
        println!("Spelare {} vann.", winner + 1);
    }
    Ok(())
}

fn display_table(game: &Game) {
    println!("Draghög: {} kort kvar.", game.draw_pile_size());
    match game.pile().last() {
        Some(card) => println!(
            "Hög: {} kort, överst {}. Drag: spelare {}.",
            game.pile().len(),
            format_card(*card),
            game.current_player() + 1
        ),
        None => println!("Högen är tom. Drag: spelare {}.", game.current_player() + 1),
    }

    println!("Öppna kort och handkort:");
    for player in 0..game.player_count() {
        let open_cards = game.face_up(player).expect("player exists");
        let open_cards = if open_cards.is_empty() {
            "inga".to_owned()
        } else {
            format_cards(open_cards)
        };
        println!(
            "  Spelare {}: {} kort på handen; öppna: {}",
            player + 1,
            game.hand_count(player).expect("player exists"),
            open_cards
        );
    }

    if game.current_player() != 0 {
        return;
    }
    match game.player_phase(0).expect("player zero exists") {
        PlayerPhase::Hand => display_cards(
            "Din hand:",
            game.hand(0).expect("player zero exists"),
            &game.legal_hand_indices(),
        ),
        PlayerPhase::FaceUp => display_cards(
            "Dina öppna kort:",
            game.face_up(0).expect("player zero exists"),
            &game.legal_face_up_indices(),
        ),
        PlayerPhase::FaceDown => println!(
            "Du har {} blinda kort kvar.",
            game.face_down_count(0).expect("player zero exists")
        ),
    }
}

fn display_cards(title: &str, cards: &[Card], legal: &[usize]) {
    println!("{title}");
    for (index, card) in cards.iter().enumerate() {
        let marker = if legal.contains(&index) {
            ""
        } else {
            " (ogiltigt)"
        };
        println!("  {}: {}{}", index + 1, format_card(*card), marker);
    }
}

fn human_turn(game: &mut Game) -> Result<Option<TurnOutcome>, Box<dyn Error>> {
    let phase = game.player_phase(0).expect("player zero exists");
    loop {
        let (card_count, legal_indices) = match phase {
            PlayerPhase::Hand => (
                game.hand(0).expect("player zero exists").len(),
                game.legal_hand_indices(),
            ),
            PlayerPhase::FaceUp => (
                game.face_up(0).expect("player zero exists").len(),
                game.legal_face_up_indices(),
            ),
            PlayerPhase::FaceDown => (
                game.face_down_count(0).expect("player zero exists"),
                Vec::new(),
            ),
        };
        let can_draw = phase == PlayerPhase::Hand && game.draw_pile_size() > 0;
        if phase != PlayerPhase::FaceDown && legal_indices.is_empty() && !can_draw {
            println!("Du har inget giltigt kort att spela och måste plocka upp högen.");
            return Ok(Some(game.pick_up_pile()?));
        }

        let can_pick_up = phase != PlayerPhase::FaceDown && !game.pile().is_empty();
        let options = match phase {
            PlayerPhase::Hand => {
                let cards_option = if legal_indices.is_empty() {
                    String::new()
                } else {
                    format!("kortnummer 1-{card_count}, flera som 1,3")
                };
                [
                    cards_option,
                    option(can_draw, "d=chansa"),
                    option(can_pick_up, "p=plocka upp"),
                    "q=avsluta".into(),
                ]
                .into_iter()
                .filter(|option| !option.is_empty())
                .collect::<Vec<_>>()
                .join(", ")
            }
            PlayerPhase::FaceUp => [
                format!("kortnummer 1-{card_count}, flera som 1,3"),
                option(can_pick_up, "p=plocka upp"),
                "q=avsluta".into(),
            ]
            .into_iter()
            .filter(|option| !option.is_empty())
            .collect::<Vec<_>>()
            .join(", "),
            PlayerPhase::FaceDown => format!("blint kort 1-{card_count}, q=avsluta"),
        };

        let prompt = match phase {
            PlayerPhase::Hand | PlayerPhase::FaceUp => format!("Ditt drag ({options}): "),
            PlayerPhase::FaceDown => format!("Vänd ett kort ({options}): "),
        };
        let Some(input) = prompt_line(&prompt)? else {
            return Ok(None);
        };
        if input.eq_ignore_ascii_case("q") {
            return Ok(None);
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
        if input.eq_ignore_ascii_case("p") && can_pick_up {
            return Ok(Some(game.pick_up_pile()?));
        }

        let Some(indices) = parse_card_indices(&input, card_count) else {
            println!("Ogiltigt val. Ange kortnummer eller ett av kommandona som visas.");
            continue;
        };
        if indices
            .iter()
            .enumerate()
            .any(|(index, item)| indices[..index].contains(item))
        {
            println!("Samma kortnummer kan inte väljas flera gånger.");
            continue;
        }
        if indices.len() > 1 && phase == PlayerPhase::FaceDown {
            println!("Blinda kort vänds ett i taget.");
            continue;
        }
        if phase != PlayerPhase::FaceDown
            && indices.iter().any(|index| !legal_indices.contains(index))
        {
            println!("Ett eller flera valda kort kan inte spelas.");
            continue;
        }
        let cards = match phase {
            PlayerPhase::Hand => game.hand(0).expect("player zero exists"),
            PlayerPhase::FaceUp => game.face_up(0).expect("player zero exists"),
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

fn option(enabled: bool, label: &str) -> String {
    if enabled { label.into() } else { String::new() }
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

fn format_cards(cards: &[Card]) -> String {
    cards
        .iter()
        .map(|card| format_card(*card))
        .collect::<Vec<_>>()
        .join(" och ")
}

fn format_card(card: Card) -> String {
    let rank = match card.rank {
        Rank::Two => "2",
        Rank::Three => "3",
        Rank::Four => "4",
        Rank::Five => "5",
        Rank::Six => "6",
        Rank::Seven => "7",
        Rank::Eight => "8",
        Rank::Nine => "9",
        Rank::Ten => "10",
        Rank::Jack => "knekt",
        Rank::Queen => "dam",
        Rank::King => "kung",
        Rank::Ace => "ess",
    };
    let suit = match card.suit {
        Suit::Clubs => "klöver",
        Suit::Diamonds => "ruter",
        Suit::Hearts => "hjärter",
        Suit::Spades => "spader",
    };
    format!("{rank} i {suit}")
}

#[cfg(test)]
mod tests {
    use super::parse_card_indices;

    #[test]
    fn parses_single_and_multiple_cli_card_numbers() {
        assert_eq!(parse_card_indices("1", 3), Some(vec![0]));
        assert_eq!(parse_card_indices("1, 3", 3), Some(vec![0, 2]));
        assert_eq!(parse_card_indices("4", 3), None);
    }
}
