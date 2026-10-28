use std::{env, process};

use macroquad::prelude::*;
use vandtia::{Card, Game, PlayerPhase, Rank, Suit, TurnOutcome, simulator::play_computer_turn};

const CARD_WIDTH: f32 = 100.0;
const CARD_HEIGHT: f32 = 142.0;
const CARD_GAP: f32 = 14.0;
const TABLE_COLOR: Color = Color::new(0.05, 0.28, 0.19, 1.0);
const PANEL_COLOR: Color = Color::new(0.02, 0.12, 0.09, 1.0);

fn window_conf() -> Conf {
    Conf {
        window_title: "Vändtia".to_owned(),
        window_width: 1100,
        window_height: 760,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Fel: {error}");
        process::exit(2);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let (players, seed) = parse_args()?;
    let mut game = Game::new(players, seed)?;
    let mut cursor = 0usize;
    let mut selected = Vec::new();
    let mut status = String::from("Piltangenter väljer kort · mellanslag markerar · Enter spelar");
    let mut ai_next_at = 0.0;

    loop {
        let current_player = game.current_player();
        let phase = game.player_phase(0).unwrap_or(PlayerPhase::FaceDown);
        if game.winner().is_none() && current_player != 0 && get_time() >= ai_next_at {
            match play_computer_turn(&mut game) {
                Ok(outcome) => {
                    status = format_outcome(&format!("Dator {} ", current_player + 1), &outcome);
                    ai_next_at = get_time() + 0.7;
                    cursor = 0;
                    selected.clear();
                }
                Err(error) => return Err(error.into()),
            }
        }

        draw_screen(&game, cursor, &selected, &status);

        if let Some(winner) = game.winner() {
            status = if winner == 0 {
                "Du vann! Tryck Q för att avsluta.".to_owned()
            } else {
                format!("Spelare {} vann. Tryck Q för att avsluta.", winner + 1)
            };
            if is_key_pressed(KeyCode::Q) || is_key_pressed(KeyCode::Escape) {
                break;
            }
            next_frame().await;
            continue;
        }

        if current_player == 0 {
            let count = player_card_count(&game, phase);
            if count > 0 {
                cursor = cursor.min(count - 1);
            } else {
                cursor = 0;
            }
            if is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::Up) {
                cursor = cursor.saturating_sub(1);
            }
            if is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::Down) {
                cursor = cursor.saturating_add(1).min(count.saturating_sub(1));
            }
            if (is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::M))
                && phase != PlayerPhase::FaceDown
                && count > 0
            {
                toggle_selection(&mut selected, cursor);
            }
            if is_key_pressed(KeyCode::D) && phase == PlayerPhase::Hand {
                match game.draw_and_play() {
                    Ok(outcome) => {
                        status = format_outcome("Du ", &outcome);
                        cursor = 0;
                        selected.clear();
                        ai_next_at = get_time() + 0.7;
                    }
                    Err(error) => status = format!("Kan inte dra kort: {error}"),
                }
            } else if is_key_pressed(KeyCode::P)
                && phase != PlayerPhase::FaceDown
                && !game.pile().is_empty()
            {
                match game.pick_up_pile() {
                    Ok(outcome) => {
                        status = format_outcome("Du ", &outcome);
                        cursor = 0;
                        selected.clear();
                        ai_next_at = get_time() + 0.7;
                    }
                    Err(error) => status = format!("Kan inte plocka upp högen: {error}"),
                }
            } else if is_key_pressed(KeyCode::Enter) && count > 0 {
                let cards = player_cards(&game, phase);
                let indices = selected_indices(&selected, cursor, phase, cards);
                let result = match phase {
                    PlayerPhase::Hand => game.play_hand_indices(&indices),
                    PlayerPhase::FaceUp => game.play_face_up_indices(&indices),
                    PlayerPhase::FaceDown => game.play_face_down_at(cursor),
                };
                match result {
                    Ok(outcome) => {
                        status = format_outcome("Du ", &outcome);
                        cursor = 0;
                        selected.clear();
                        ai_next_at = get_time() + 0.7;
                    }
                    Err(error) => status = format!("Draget kunde inte spelas: {error}"),
                }
            }
            if is_key_pressed(KeyCode::Q) || is_key_pressed(KeyCode::Escape) {
                break;
            }

            if is_mouse_button_pressed(MouseButton::Left) {
                let (mouse_x, mouse_y) = mouse_position();
                if let Some(index) =
                    card_at_position(mouse_x, mouse_y, count, screen_width(), screen_height())
                {
                    cursor = index;
                    if phase != PlayerPhase::FaceDown {
                        toggle_selection(&mut selected, index);
                    } else if is_key_down(KeyCode::LeftAlt) {
                        let _ = game.play_face_down_at(index);
                    }
                }
            }
        } else if is_key_pressed(KeyCode::Q) || is_key_pressed(KeyCode::Escape) {
            break;
        }

        next_frame().await;
    }

    Ok(())
}

fn parse_args() -> Result<(usize, u64), Box<dyn std::error::Error>> {
    let mut players = 4usize;
    let mut seed = 1u64;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--players" | "-p" => {
                players = args.next().ok_or("--players requires a value")?.parse()?;
            }
            "--seed" | "-s" => {
                seed = args.next().ok_or("--seed requires a value")?.parse()?;
            }
            "--help" | "-h" => {
                println!("Usage: vandtia-graphics [--players 2-5] [--seed N]");
                process::exit(0);
            }
            unknown => return Err(format!("unknown argument: {unknown}").into()),
        }
    }
    Ok((players, seed))
}

fn draw_screen(game: &Game, cursor: usize, selected: &[usize], status: &str) {
    clear_background(TABLE_COLOR);
    let width = screen_width();
    let height = screen_height();

    draw_text("VÄNDTIA", 28.0, 42.0, 34.0, GOLD);
    draw_text(
        format!(
            "Draghög: {} kort     Spelhög: {} kort",
            game.draw_pile_size(),
            game.pile().len()
        ),
        240.0,
        38.0,
        22.0,
        WHITE,
    );
    let current = if game.current_player() == 0 {
        "Din tur".to_owned()
    } else {
        format!("Dator {} spelar", game.current_player() + 1)
    };
    draw_text(&current, width - 210.0, 38.0, 22.0, SKYBLUE);

    draw_opponents(game, width);
    draw_pile(game, width, height);
    draw_player_cards(game, cursor, selected, width, height);
    draw_controls(game, status, width, height);
}

fn draw_opponents(game: &Game, width: f32) {
    let mut x = 28.0;
    let y = 72.0;
    for player in 0..game.player_count() {
        if player == 0 {
            continue;
        }
        draw_rectangle(x, y, 220.0, 140.0, PANEL_COLOR);
        draw_rectangle_lines(x, y, 220.0, 140.0, 2.0, DARKGREEN);
        draw_text(
            format!(
                "Dator {}: {} på hand",
                player + 1,
                game.hand_count(player).unwrap_or(0)
            ),
            x + 12.0,
            y + 27.0,
            18.0,
            WHITE,
        );
        let open = game.face_up(player).unwrap_or(&[]);
        draw_text("Öppna:", x + 12.0, y + 53.0, 17.0, LIGHTGRAY);
        for (index, card) in open.iter().enumerate() {
            draw_card(x + 12.0 + (index as f32 * 63.0), y + 65.0, *card, false);
        }
        x += 238.0;
        if x + 220.0 > width {
            break;
        }
    }
}

fn draw_pile(game: &Game, width: f32, height: f32) {
    let center_x = width * 0.68;
    let center_y = height * 0.38;
    draw_text("SPELHÖG", center_x - 65.0, center_y - 62.0, 22.0, LIGHTGRAY);
    if let Some(card) = game.pile().last() {
        draw_card(center_x - CARD_WIDTH / 2.0, center_y - 48.0, *card, false);
        draw_text(
            format!("{} kort i högen", game.pile().len()),
            center_x - 75.0,
            center_y + 116.0,
            18.0,
            WHITE,
        );
    } else {
        draw_rectangle(
            center_x - CARD_WIDTH / 2.0,
            center_y - 48.0,
            CARD_WIDTH,
            CARD_HEIGHT,
            Color::new(0.03, 0.2, 0.14, 1.0),
        );
        draw_rectangle_lines(
            center_x - CARD_WIDTH / 2.0,
            center_y - 48.0,
            CARD_WIDTH,
            CARD_HEIGHT,
            2.0,
            LIGHTGRAY,
        );
        draw_text("Tom", center_x - 18.0, center_y + 30.0, 20.0, LIGHTGRAY);
    }
}

fn draw_player_cards(game: &Game, cursor: usize, selected: &[usize], width: f32, height: f32) {
    let phase = game.player_phase(0).unwrap_or(PlayerPhase::FaceDown);
    let cards = player_cards(game, phase);
    let count = player_card_count(game, phase);
    let title = match phase {
        PlayerPhase::Hand => "DIN HAND",
        PlayerPhase::FaceUp => "DINA ÖPPNA KORT",
        PlayerPhase::FaceDown => "DINA BLINDA KORT",
    };
    let start_y = height - CARD_HEIGHT - 92.0;
    draw_text(title, 28.0, start_y - 16.0, 20.0, GOLD);
    let total_width = count as f32 * (CARD_WIDTH + CARD_GAP) - CARD_GAP;
    let start_x = ((width - total_width) / 2.0).max(22.0);

    for index in 0..count {
        let x = start_x + index as f32 * (CARD_WIDTH + CARD_GAP);
        let is_selected = selected.contains(&index);
        let y = start_y - if index == cursor { 16.0 } else { 0.0 };
        if phase == PlayerPhase::FaceDown {
            draw_card_back(x, y, index == cursor);
            draw_text(format!("{}", index + 1), x + 45.0, y + 78.0, 24.0, WHITE);
        } else if let Some(card) = cards.get(index) {
            draw_card(x, y, *card, is_selected || index == cursor);
            draw_text(
                format!("{}", index + 1),
                x + 44.0,
                y + CARD_HEIGHT + 19.0,
                16.0,
                WHITE,
            );
        }
    }
}

fn draw_controls(game: &Game, status: &str, width: f32, height: f32) {
    let panel_y = height - 63.0;
    draw_rectangle(0.0, panel_y - 8.0, width, 72.0, PANEL_COLOR);
    draw_text(status, 20.0, panel_y + 12.0, 18.0, YELLOW);
    let phase = game.player_phase(0).unwrap_or(PlayerPhase::FaceDown);
    let controls = match phase {
        PlayerPhase::Hand => {
            "←/→ välj  ·  Enter spela  ·  Space markera  ·  D dra  ·  P plocka upp  ·  Q avsluta"
        }
        PlayerPhase::FaceUp => {
            "←/→ välj  ·  Enter spela  ·  Space markera  ·  P plocka upp  ·  Q avsluta"
        }
        PlayerPhase::FaceDown => "←/→ välj kort  ·  Enter vänd  ·  Q avsluta",
    };
    draw_text(controls, 20.0, panel_y + 42.0, 16.0, LIGHTGRAY);
}

fn draw_card(x: f32, y: f32, card: Card, highlighted: bool) {
    let fill = if highlighted {
        Color::new(1.0, 0.96, 0.78, 1.0)
    } else {
        WHITE
    };
    let outline = if highlighted { GOLD } else { LIGHTGRAY };
    draw_rectangle(x, y, CARD_WIDTH, CARD_HEIGHT, fill);
    draw_rectangle_lines(x, y, CARD_WIDTH, CARD_HEIGHT, 3.0, outline);
    let color = suit_color(card.suit);
    draw_text(rank_label(card.rank), x + 9.0, y + 28.0, 25.0, BLACK);
    draw_text(suit_symbol(card.suit), x + 8.0, y + 53.0, 23.0, color);
    draw_text(suit_symbol(card.suit), x + 35.0, y + 94.0, 42.0, color);
}

fn draw_card_back(x: f32, y: f32, highlighted: bool) {
    let color = Color::new(0.12, 0.28, 0.62, 1.0);
    draw_rectangle(x, y, CARD_WIDTH, CARD_HEIGHT, color);
    draw_rectangle_lines(
        x,
        y,
        CARD_WIDTH,
        CARD_HEIGHT,
        if highlighted { 4.0 } else { 2.0 },
        if highlighted { GOLD } else { WHITE },
    );
    draw_text("✦", x + 34.0, y + 79.0, 34.0, WHITE);
}

fn card_at_position(
    mouse_x: f32,
    mouse_y: f32,
    count: usize,
    width: f32,
    height: f32,
) -> Option<usize> {
    let start_y = height - CARD_HEIGHT - 92.0;
    let total_width = count as f32 * (CARD_WIDTH + CARD_GAP) - CARD_GAP;
    let start_x = ((width - total_width) / 2.0).max(22.0);
    if mouse_y < start_y - 18.0 || mouse_y > start_y + CARD_HEIGHT {
        return None;
    }
    (0..count).find(|index| {
        let x = start_x + *index as f32 * (CARD_WIDTH + CARD_GAP);
        mouse_x >= x && mouse_x <= x + CARD_WIDTH
    })
}

fn player_cards(game: &Game, phase: PlayerPhase) -> &[Card] {
    match phase {
        PlayerPhase::Hand => game.hand(0).unwrap_or(&[]),
        PlayerPhase::FaceUp => game.face_up(0).unwrap_or(&[]),
        PlayerPhase::FaceDown => &[],
    }
}

fn player_card_count(game: &Game, phase: PlayerPhase) -> usize {
    match phase {
        PlayerPhase::Hand => game.hand(0).map_or(0, <[Card]>::len),
        PlayerPhase::FaceUp => game.face_up(0).map_or(0, <[Card]>::len),
        PlayerPhase::FaceDown => game.face_down_count(0).unwrap_or(0),
    }
}

fn selected_indices(
    selected: &[usize],
    cursor: usize,
    phase: PlayerPhase,
    cards: &[Card],
) -> Vec<usize> {
    if !selected.is_empty() {
        return selected.to_vec();
    }
    if phase == PlayerPhase::FaceDown {
        return vec![cursor];
    }
    let Some(card) = cards.get(cursor) else {
        return Vec::new();
    };
    cards
        .iter()
        .enumerate()
        .filter_map(|(index, other)| (other.rank == card.rank).then_some(index))
        .collect()
}

fn toggle_selection(selected: &mut Vec<usize>, index: usize) {
    if let Some(position) = selected
        .iter()
        .position(|selected_index| *selected_index == index)
    {
        selected.remove(position);
    } else {
        selected.push(index);
        selected.sort_unstable();
    }
}

fn format_outcome(actor: &str, outcome: &TurnOutcome) -> String {
    let mut message = if let Some(card) = outcome.drawn {
        if outcome.played.is_some() {
            format!("{actor}drog och spelade {}.", format_card(card))
        } else {
            format!("{actor}drog {}, tog upp högen.", format_card(card))
        }
    } else if !outcome.played_cards.is_empty() {
        let cards = outcome
            .played_cards
            .iter()
            .map(|card| format_card(*card))
            .collect::<Vec<_>>()
            .join(" och ");
        format!("{actor}spelade {cards}.")
    } else if let Some(card) = outcome.played {
        format!("{actor}vände {}.", format_card(card))
    } else {
        format!("{actor}plockade upp högen.")
    };
    if outcome.burned {
        message.push_str(" Högen vänds bort — extra tur!");
    } else if outcome.extra_turn {
        message.push_str(" Extra tur!");
    }
    if outcome.picked_up > 0 && outcome.drawn.is_none() {
        message.push_str(&format!(" Tog upp {} kort.", outcome.picked_up));
    }
    message
}

fn format_card(card: Card) -> String {
    format!("{}{}", rank_label(card.rank), suit_symbol(card.suit))
}

fn rank_label(rank: Rank) -> &'static str {
    match rank {
        Rank::Two => "2",
        Rank::Three => "3",
        Rank::Four => "4",
        Rank::Five => "5",
        Rank::Six => "6",
        Rank::Seven => "7",
        Rank::Eight => "8",
        Rank::Nine => "9",
        Rank::Ten => "10",
        Rank::Jack => "J",
        Rank::Queen => "Q",
        Rank::King => "K",
        Rank::Ace => "A",
    }
}

fn suit_symbol(suit: Suit) -> &'static str {
    match suit {
        Suit::Clubs => "♣",
        Suit::Diamonds => "♦",
        Suit::Hearts => "♥",
        Suit::Spades => "♠",
    }
}

fn suit_color(suit: Suit) -> Color {
    match suit {
        Suit::Diamonds | Suit::Hearts => RED,
        Suit::Clubs | Suit::Spades => BLACK,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(rank: Rank, suit: Suit) -> Card {
        Card { rank, suit }
    }

    #[test]
    fn enter_selection_includes_matching_ranks() {
        let cards = [
            card(Rank::Seven, Suit::Clubs),
            card(Rank::Ace, Suit::Hearts),
            card(Rank::Seven, Suit::Spades),
        ];
        assert_eq!(
            selected_indices(&[], 0, PlayerPhase::Hand, &cards),
            vec![0, 2]
        );
        assert_eq!(
            selected_indices(&[1, 2], 0, PlayerPhase::Hand, &cards),
            vec![1, 2]
        );
        assert_eq!(
            selected_indices(&[], 2, PlayerPhase::FaceDown, &[]),
            vec![2]
        );
    }

    #[test]
    fn mouse_position_selects_card_bounds() {
        assert_eq!(card_at_position(250.0, 530.0, 3, 800.0, 760.0), Some(0));
        assert_eq!(card_at_position(340.0, 530.0, 3, 800.0, 760.0), None);
    }
}
