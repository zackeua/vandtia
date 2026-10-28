use std::{env, process};

use macroquad::prelude::*;
use vandtia::{Card, Game, PlayerPhase, Rank, Suit, TurnOutcome, simulator::play_computer_turn};

const CARD_WIDTH: f32 = 100.0;
const CARD_HEIGHT: f32 = 142.0;
const CARD_GAP: f32 = 14.0;
const ACTION_BUTTON_HEIGHT: f32 = 38.0;
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
    let mut status =
        String::from("Click cards to select them, then click PLAY SELECTED or the pile.");
    let mut ai_next_at = 0.0;

    loop {
        let current_player = game.current_player();
        let phase = game.player_phase(0).unwrap_or(PlayerPhase::FaceDown);
        let display_order = player_display_order(&game, phase);
        if game.winner().is_none() && current_player != 0 && get_time() >= ai_next_at {
            match play_computer_turn(&mut game) {
                Ok(outcome) => {
                    status = format_outcome(&format!("Player {} ", current_player + 1), &outcome);
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
                "You won! Press Q to quit.".to_owned()
            } else {
                format!("Player {} won. Press Q to quit.", winner + 1)
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
                toggle_selection(&mut selected, display_order[cursor]);
                status = selection_status(&selected);
            }
            if is_key_pressed(KeyCode::D) && phase == PlayerPhase::Hand {
                match game.draw_and_play() {
                    Ok(outcome) => {
                        status = format_outcome("You ", &outcome);
                        cursor = 0;
                        selected.clear();
                        ai_next_at = get_time() + 0.7;
                    }
                    Err(error) => status = format!("Could not draw: {error}"),
                }
            } else if is_key_pressed(KeyCode::P)
                && phase != PlayerPhase::FaceDown
                && !game.pile().is_empty()
            {
                match game.pick_up_pile() {
                    Ok(outcome) => {
                        status = format_outcome("You ", &outcome);
                        cursor = 0;
                        selected.clear();
                        ai_next_at = get_time() + 0.7;
                    }
                    Err(error) => status = format!("Could not pick up pile: {error}"),
                }
            } else if is_key_pressed(KeyCode::Enter) && count > 0 {
                let indices = selected_indices(&selected, cursor, phase);
                if phase == PlayerPhase::FaceDown {
                    match game.play_face_down_at(cursor) {
                        Ok(outcome) => {
                            status = format_outcome("You ", &outcome);
                            cursor = 0;
                            selected.clear();
                            ai_next_at = get_time() + 0.7;
                        }
                        Err(error) => status = format!("Could not turn card: {error}"),
                    }
                } else if indices.is_empty() {
                    status = "Select one or more cards first.".to_owned();
                } else {
                    play_selected(
                        &mut game,
                        phase,
                        &indices,
                        &mut selected,
                        &mut status,
                        &mut cursor,
                        &mut ai_next_at,
                    );
                }
            }
            if is_key_pressed(KeyCode::Q) || is_key_pressed(KeyCode::Escape) {
                break;
            }

            if is_mouse_button_pressed(MouseButton::Left) {
                let (mouse_x, mouse_y) = mouse_position();
                if let Some(slot) =
                    card_at_position(mouse_x, mouse_y, count, screen_width(), screen_height())
                {
                    cursor = slot;
                    let index = display_order[slot];
                    if phase == PlayerPhase::FaceDown {
                        match game.play_face_down_at(index) {
                            Ok(outcome) => {
                                status = format_outcome("You ", &outcome);
                                cursor = 0;
                                ai_next_at = get_time() + 0.7;
                            }
                            Err(error) => status = format!("Could not turn card: {error}"),
                        }
                    } else {
                        toggle_selection(&mut selected, index);
                        status = selection_status(&selected);
                    }
                } else if phase != PlayerPhase::FaceDown
                    && play_button_contains(mouse_x, mouse_y, screen_width(), screen_height())
                {
                    if selected.is_empty() {
                        status = "Select one or more cards first.".to_owned();
                    } else {
                        let indices = selected.clone();
                        play_selected(
                            &mut game,
                            phase,
                            &indices,
                            &mut selected,
                            &mut status,
                            &mut cursor,
                            &mut ai_next_at,
                        );
                    }
                } else if phase == PlayerPhase::Hand
                    && draw_button_contains_at(mouse_x, mouse_y, screen_width(), screen_height())
                {
                    match game.draw_and_play() {
                        Ok(outcome) => {
                            status = format_outcome("You ", &outcome);
                            cursor = 0;
                            selected.clear();
                            ai_next_at = get_time() + 0.7;
                        }
                        Err(error) => status = format!("Could not draw: {error}"),
                    }
                } else if phase != PlayerPhase::FaceDown
                    && !game.pile().is_empty()
                    && pickup_button_contains(mouse_x, mouse_y, screen_width(), screen_height())
                {
                    match game.pick_up_pile() {
                        Ok(outcome) => {
                            status = format_outcome("You ", &outcome);
                            cursor = 0;
                            selected.clear();
                            ai_next_at = get_time() + 0.7;
                        }
                        Err(error) => status = format!("Could not pick up pile: {error}"),
                    }
                } else if phase != PlayerPhase::FaceDown
                    && pile_target_contains(mouse_x, mouse_y, screen_width(), screen_height())
                {
                    if selected.is_empty() {
                        status = "Select one or more cards first.".to_owned();
                    } else {
                        let indices = selected.clone();
                        play_selected(
                            &mut game,
                            phase,
                            &indices,
                            &mut selected,
                            &mut status,
                            &mut cursor,
                            &mut ai_next_at,
                        );
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

fn play_selected(
    game: &mut Game,
    phase: PlayerPhase,
    indices: &[usize],
    selection: &mut Vec<usize>,
    status: &mut String,
    cursor: &mut usize,
    ai_next_at: &mut f64,
) {
    let result = match phase {
        PlayerPhase::Hand => game.play_hand_indices(indices),
        PlayerPhase::FaceUp => game.play_face_up_indices(indices),
        PlayerPhase::FaceDown => return,
    };
    match result {
        Ok(outcome) => {
            *status = format_outcome("You ", &outcome);
            *cursor = 0;
            selection.clear();
            *ai_next_at = get_time() + 0.7;
        }
        Err(error) => *status = format!("Could not play selection: {error}"),
    }
}

fn selection_status(selected: &[usize]) -> String {
    if selected.is_empty() {
        "Selection cleared.".to_owned()
    } else {
        format!(
            "{} card(s) selected. Click PLAY SELECTED or the pile.",
            selected.len()
        )
    }
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
            "DRAW PILE: {} cards     DISCARD PILE: {} cards",
            game.draw_pile_size(),
            game.pile().len()
        ),
        240.0,
        38.0,
        22.0,
        WHITE,
    );
    draw_text(
        "SUITS: C=CLUBS  D=DIAMONDS  H=HEARTS  S=SPADES",
        28.0,
        61.0,
        15.0,
        LIGHTGRAY,
    );
    let current = if game.current_player() == 0 {
        "Your turn".to_owned()
    } else {
        format!("Player {} turn", game.current_player() + 1)
    };
    draw_text(&current, width - 210.0, 38.0, 22.0, SKYBLUE);

    draw_opponents(game, width);
    draw_pile(game, width, height);
    draw_player_cards(game, cursor, selected, width, height);
    draw_controls(game, status, width, height);
}

fn draw_opponents(game: &Game, width: f32) {
    let opponent_count = game.player_count().saturating_sub(1);
    let columns = opponent_columns(opponent_count, width);
    for (index, player) in (1..game.player_count()).enumerate() {
        let x = 28.0 + (index % columns) as f32 * 234.0;
        let y = 76.0 + (index / columns) as f32 * 154.0;
        draw_rectangle(x, y, 220.0, 140.0, PANEL_COLOR);
        draw_rectangle_lines(x, y, 220.0, 140.0, 2.0, DARKGREEN);
        draw_text(
            format!(
                "Player {}: {} in hand",
                player + 1,
                game.hand_count(player).unwrap_or(0)
            ),
            x + 12.0,
            y + 27.0,
            18.0,
            WHITE,
        );
        let open = game.face_up(player).unwrap_or(&[]);
        draw_text("FACE UP:", x + 12.0, y + 48.0, 15.0, LIGHTGRAY);
        for (index, card) in open.iter().take(3).enumerate() {
            draw_compact_card(x + 12.0 + index as f32 * 58.0, y + 56.0, *card);
        }
    }
}

fn opponent_columns(opponent_count: usize, width: f32) -> usize {
    let available_width = (width * 0.72 - 28.0).max(220.0);
    let columns = ((available_width + 14.0) / 234.0).floor() as usize;
    columns.max(1).min(opponent_count.max(1))
}

fn draw_compact_card(x: f32, y: f32, card: Card) {
    const WIDTH: f32 = 54.0;
    const HEIGHT: f32 = 78.0;
    let color = suit_color(card.suit);
    draw_rectangle(x, y, WIDTH, HEIGHT, WHITE);
    draw_rectangle_lines(x, y, WIDTH, HEIGHT, 1.0, LIGHTGRAY);
    draw_text(rank_label(card.rank), x + 5.0, y + 17.0, 15.0, BLACK);
    draw_text(suit_letter(card.suit), x + 5.0, y + 33.0, 14.0, color);
    draw_text(
        suit_letter(card.suit),
        x + WIDTH / 2.0 - 8.0,
        y + 62.0,
        24.0,
        color,
    );
}

fn draw_pile(game: &Game, width: f32, height: f32) {
    let pile = pile_card_rect(width, height);
    let center_x = pile.x + pile.w / 2.0;
    draw_text(
        "DISCARD PILE - CLICK TO PLAY",
        center_x - 145.0,
        pile.y - 14.0,
        18.0,
        LIGHTGRAY,
    );
    if let Some(card) = game.pile().last() {
        draw_card(pile.x, pile.y, *card, false);
        draw_text(
            format!("{} cards in pile", game.pile().len()),
            center_x - 75.0,
            pile.y + CARD_HEIGHT + 22.0,
            18.0,
            WHITE,
        );
    } else {
        draw_rectangle(
            pile.x,
            pile.y,
            pile.w,
            pile.h,
            Color::new(0.03, 0.2, 0.14, 1.0),
        );
        draw_rectangle_lines(pile.x, pile.y, pile.w, pile.h, 2.0, LIGHTGRAY);
        draw_text("EMPTY", center_x - 27.0, pile.y + 78.0, 18.0, LIGHTGRAY);
    }
}

fn pile_card_rect(width: f32, height: f32) -> Rect {
    let center_x = (width * 0.84).clamp(CARD_WIDTH / 2.0 + 12.0, width - CARD_WIDTH / 2.0 - 12.0);
    let center_y = height * 0.36;
    Rect::new(
        center_x - CARD_WIDTH / 2.0,
        center_y - 48.0,
        CARD_WIDTH,
        CARD_HEIGHT,
    )
}

fn draw_player_cards(game: &Game, cursor: usize, selected: &[usize], width: f32, height: f32) {
    let phase = game.player_phase(0).unwrap_or(PlayerPhase::FaceDown);
    let cards = player_cards(game, phase);
    let count = player_card_count(game, phase);
    let title = match phase {
        PlayerPhase::Hand => "YOUR HAND",
        PlayerPhase::FaceUp => "YOUR FACE-UP CARDS",
        PlayerPhase::FaceDown => "YOUR FACE-DOWN CARDS",
    };
    let start_y = height - CARD_HEIGHT - 154.0;
    draw_text(title, 28.0, start_y - 16.0, 20.0, GOLD);
    let (card_width, card_gap, start_x) = card_layout(count, width);
    let display_order = player_display_order(game, phase);

    for (index, card_index) in display_order.iter().copied().enumerate() {
        let x = start_x + index as f32 * (card_width + card_gap);
        let is_selected = selected.contains(&card_index);
        let y = start_y;
        if phase == PlayerPhase::FaceDown {
            draw_card_back(x, y, card_width, index == cursor);
            draw_text(
                format!("{}", index + 1),
                x + card_width / 2.0 - 5.0,
                y + 78.0,
                24.0,
                WHITE,
            );
        } else if let Some(card) = cards.get(card_index) {
            draw_card_sized(x, y, *card, card_width, is_selected, index == cursor);
            draw_text(
                format!("{}", index + 1),
                x + card_width / 2.0 - 5.0,
                y + CARD_HEIGHT + 19.0,
                16.0,
                WHITE,
            );
        }
    }
    if phase != PlayerPhase::FaceDown {
        draw_action_buttons(game, phase, width, height, selected.len());
    }
}

fn action_button_rects(width: f32, height: f32) -> (Rect, Rect, Rect) {
    let scale = ((width - 24.0) / 464.0).clamp(0.0, 1.0);
    let play_width = 180.0 * scale;
    let other_width = 130.0 * scale;
    let gap = 12.0 * scale;
    let total_width = play_width + other_width * 2.0 + gap * 2.0;
    let left = ((width - total_width) / 2.0).max(12.0);
    let y = height - 101.0;
    (
        Rect::new(left, y, play_width, ACTION_BUTTON_HEIGHT),
        Rect::new(
            left + play_width + gap,
            y,
            other_width,
            ACTION_BUTTON_HEIGHT,
        ),
        Rect::new(
            left + play_width + gap + other_width + gap,
            y,
            other_width,
            ACTION_BUTTON_HEIGHT,
        ),
    )
}

fn draw_action_buttons(
    game: &Game,
    phase: PlayerPhase,
    width: f32,
    height: f32,
    selected_count: usize,
) {
    let (play, draw, pickup) = action_button_rects(width, height);
    let play_text = if selected_count == 0 {
        "SELECT CARDS"
    } else {
        "PLAY SELECTED"
    };
    draw_button(play, play_text, selected_count > 0);

    if phase == PlayerPhase::Hand && game.draw_pile_size() > 0 {
        draw_button(draw, "DRAW + PLAY", true);
    } else {
        draw_button(draw, "DRAW EMPTY", false);
    }

    if !game.pile().is_empty() && phase != PlayerPhase::FaceDown {
        draw_button(pickup, "PICK UP", true);
    } else {
        draw_button(pickup, "PICK UP", false);
    }
}

fn draw_button(rect: Rect, label: &str, enabled: bool) {
    let fill = if enabled {
        Color::new(0.13, 0.4, 0.28, 1.0)
    } else {
        Color::new(0.16, 0.19, 0.18, 1.0)
    };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        2.0,
        if enabled { GOLD } else { DARKGRAY },
    );
    let text_width = measure_text(label, None, 16, 1.0).width;
    let font_size = (16.0 * (rect.w * 0.9 / text_width)).clamp(8.0, 16.0);
    draw_text(
        label,
        rect.x + (rect.w - text_width * font_size / 16.0) / 2.0,
        rect.y + (rect.h + font_size * 0.62) / 2.0,
        font_size,
        if enabled { WHITE } else { LIGHTGRAY },
    );
}

fn draw_button_contains(rect: Rect, x: f32, y: f32) -> bool {
    x >= rect.x && x <= rect.x + rect.w && y >= rect.y && y <= rect.y + rect.h
}

fn play_button_contains(x: f32, y: f32, width: f32, height: f32) -> bool {
    draw_button_contains(action_button_rects(width, height).0, x, y)
}

fn draw_button_contains_at(x: f32, y: f32, width: f32, height: f32) -> bool {
    draw_button_contains(action_button_rects(width, height).1, x, y)
}

fn pickup_button_contains(x: f32, y: f32, width: f32, height: f32) -> bool {
    draw_button_contains(action_button_rects(width, height).2, x, y)
}

fn pile_target_contains(x: f32, y: f32, width: f32, height: f32) -> bool {
    let pile = pile_card_rect(width, height);
    x >= pile.x && x <= pile.x + pile.w && y >= pile.y && y <= pile.y + pile.h
}

fn card_layout(count: usize, screen_width: f32) -> (f32, f32, f32) {
    let available_width = (screen_width - 44.0).max(1.0);
    if count == 0 {
        return (CARD_WIDTH, CARD_GAP, 22.0);
    }
    let count_f = count as f32;
    let card_width = CARD_WIDTH.min(available_width);
    let gap = if count > 1 && count_f * (card_width + CARD_GAP) - CARD_GAP > available_width {
        (available_width - card_width * count_f) / (count_f - 1.0)
    } else if count > 1 {
        CARD_GAP
    } else {
        0.0
    };
    let total_width = count_f * card_width + (count_f - 1.0).max(0.0) * gap;
    let start_x = ((screen_width - total_width) / 2.0).max(8.0);
    (card_width, gap, start_x)
}

fn draw_controls(game: &Game, status: &str, width: f32, height: f32) {
    let panel_y = height - 43.0;
    draw_rectangle(0.0, panel_y - 6.0, width, 52.0, PANEL_COLOR);
    draw_text(status, 20.0, panel_y + 12.0, 18.0, YELLOW);
    let phase = game.player_phase(0).unwrap_or(PlayerPhase::FaceDown);
    let controls = match phase {
        PlayerPhase::Hand => {
            "LEFT/RIGHT: MOVE  SPACE: TOGGLE  ENTER: PLAY  D: DRAW  P: PICK UP  Q: QUIT"
        }
        PlayerPhase::FaceUp => "LEFT/RIGHT: MOVE  SPACE: TOGGLE  ENTER: PLAY  P: PICK UP  Q: QUIT",
        PlayerPhase::FaceDown => "LEFT/RIGHT: MOVE  ENTER: REVEAL  Q: QUIT",
    };
    draw_text(controls, 20.0, panel_y + 34.0, 14.0, LIGHTGRAY);
}

fn draw_card(x: f32, y: f32, card: Card, highlighted: bool) {
    draw_card_sized(x, y, card, CARD_WIDTH, highlighted, false);
}

fn draw_card_sized(x: f32, y: f32, card: Card, width: f32, selected: bool, focused: bool) {
    let suit = suit_color(card.suit);
    let paper = if selected {
        Color::new(1.0, 0.95, 0.72, 1.0)
    } else {
        Color::new(0.98, 0.96, 0.87, 1.0)
    };
    let edge = if selected {
        GREEN
    } else if focused {
        GOLD
    } else {
        Color::new(0.12, 0.16, 0.2, 1.0)
    };
    draw_rectangle(x, y, width, CARD_HEIGHT, edge);
    draw_rectangle(x + 3.0, y + 3.0, width - 6.0, CARD_HEIGHT - 6.0, paper);
    draw_rectangle(x + 7.0, y + 7.0, width - 14.0, 4.0, suit);
    draw_rectangle(x + 7.0, y + CARD_HEIGHT - 11.0, width - 14.0, 4.0, suit);
    draw_pixel_text(
        rank_label(card.rank),
        x + 10.0,
        y + 17.0,
        3.0,
        Color::new(0.08, 0.1, 0.13, 1.0),
    );
    draw_pixel_pattern(
        x + width - 21.0,
        y + 10.0,
        2.0,
        suit_pattern(card.suit),
        suit,
    );

    if has_face_portrait(card.rank) {
        draw_card_mascot(x + width / 2.0 - 17.5, y + 48.0, card.rank);
    } else if card.rank == Rank::Ace {
        draw_card_mascot(x + width / 2.0 - 17.5, y + 55.0, card.rank);
    } else {
        let label = rank_label(card.rank);
        let columns = label.chars().count() * 3 + label.len().saturating_sub(1);
        let rank_pixel = ((width - 20.0) / columns as f32).clamp(3.0, 10.0);
        let rank_width = pixel_text_width(label, rank_pixel);
        draw_pixel_text(
            label,
            x + (width - rank_width) / 2.0,
            y + 53.0,
            rank_pixel,
            Color::new(0.08, 0.1, 0.13, 1.0),
        );
    }

    draw_pixel_pattern(
        x + width / 2.0 - 10.5,
        y + 119.0,
        1.5,
        suit_pattern(card.suit),
        suit,
    );
}

fn pixel_glyph(character: char) -> [u8; 5] {
    match character {
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
        '3' => [0b111, 0b001, 0b111, 0b001, 0b111],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b111, 0b001, 0b111],
        '6' => [0b111, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b010, 0b010, 0b010],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b111],
        'A' => [0b010, 0b101, 0b111, 0b101, 0b101],
        'J' => [0b111, 0b001, 0b001, 0b101, 0b111],
        'K' => [0b101, 0b101, 0b110, 0b101, 0b101],
        'Q' => [0b111, 0b101, 0b101, 0b111, 0b001],
        _ => [0; 5],
    }
}

fn pixel_text_width(text: &str, pixel: f32) -> f32 {
    text.chars().count() as f32 * 3.0 * pixel
        + text.chars().count().saturating_sub(1) as f32 * pixel
}

fn draw_pixel_text(text: &str, x: f32, y: f32, pixel: f32, color: Color) {
    let mut cursor_x = x;
    for character in text.chars() {
        for (row, bits) in pixel_glyph(character).iter().enumerate() {
            for column in 0..3 {
                if bits & (1 << (2 - column)) != 0 {
                    draw_rectangle(
                        cursor_x + column as f32 * pixel,
                        y + row as f32 * pixel,
                        pixel,
                        pixel,
                        color,
                    );
                }
            }
        }
        cursor_x += pixel * 4.0;
    }
}

fn suit_pattern(suit: Suit) -> &'static [u8] {
    match suit {
        Suit::Clubs => &[
            0b0011100, 0b0111110, 0b1111111, 0b0011100, 0b0011100, 0b0111110, 0b0111110,
        ],
        Suit::Diamonds => &[
            0b0001000, 0b0011100, 0b0111110, 0b1111111, 0b0111110, 0b0011100, 0b0001000,
        ],
        Suit::Hearts => &[
            0b0110110, 0b1111111, 0b1111111, 0b1111111, 0b0111110, 0b0011100, 0b0001000,
        ],
        Suit::Spades => &[
            0b0001000, 0b0011100, 0b0111110, 0b1111111, 0b1111111, 0b0011100, 0b0111110,
        ],
    }
}

fn draw_pixel_pattern(x: f32, y: f32, pixel: f32, rows: &[u8], color: Color) {
    for (row, bits) in rows.iter().enumerate() {
        for column in 0..7 {
            if bits & (1 << (6 - column)) != 0 {
                draw_rectangle(
                    x + column as f32 * pixel,
                    y + row as f32 * pixel,
                    pixel,
                    pixel,
                    color,
                );
            }
        }
    }
}

fn draw_card_mascot(x: f32, y: f32, rank: Rank) {
    let ink = Color::new(0.1, 0.13, 0.17, 1.0);
    let skin = Color::new(1.0, 0.69, 0.31, 1.0);
    match rank {
        Rank::Ace => {
            draw_pixel_pattern(
                x + 3.0,
                y + 2.0,
                4.0,
                &[
                    0b0001000, 0b0011100, 0b1111111, 0b0111110, 0b0111110, 0b0011100, 0b0011100,
                ],
                GOLD,
            );
            draw_rectangle(x + 14.0, y + 16.0, 4.0, 4.0, WHITE);
        }
        Rank::Jack => {
            let tunic = Color::new(0.15, 0.44, 0.72, 1.0);
            draw_pixel_pattern(x + 3.0, y, 4.0, &[0b0010000, 0b0111000, 0b1111111], GOLD);
            draw_pixel_pattern(x + 3.0, y + 11.0, 4.0, &[0b0011100, 0b0111110], ink);
            draw_pixel_pattern(
                x + 3.0,
                y + 19.0,
                4.0,
                &[0b0111110, 0b1111111, 0b1111111, 0b1111111, 0b0111110],
                skin,
            );
            draw_rectangle(x + 10.0, y + 27.0, 4.0, 4.0, ink);
            draw_rectangle(x + 22.0, y + 27.0, 4.0, 4.0, ink);
            draw_pixel_pattern(x + 11.0, y + 39.0, 4.0, &[0b0001000, 0b0011100], skin);
            draw_pixel_pattern(
                x,
                y + 47.0,
                4.0,
                &[0b0010000, 0b0111110, 0b1111111, 0b1111111, 0b1111111],
                tunic,
            );
            draw_rectangle(x + 15.0, y + 47.0, 4.0, 16.0, GOLD);
        }
        Rank::Queen => {
            let hair = Color::new(0.55, 0.16, 0.39, 1.0);
            draw_pixel_pattern(x + 3.0, y, 4.0, &[0b1010101, 0b1111111], GOLD);
            draw_pixel_pattern(
                x + 3.0,
                y + 9.0,
                4.0,
                &[
                    0b0111110, 0b1111111, 0b1111111, 0b1111111, 0b0111110, 0b0111110,
                ],
                hair,
            );
            draw_pixel_pattern(
                x + 7.0,
                y + 17.0,
                4.0,
                &[0b0011100, 0b0111110, 0b0111110, 0b0111110],
                skin,
            );
            draw_rectangle(x + 11.0, y + 29.0, 4.0, 4.0, ink);
            draw_rectangle(x + 21.0, y + 29.0, 4.0, 4.0, ink);
            draw_pixel_pattern(x + 11.0, y + 38.0, 4.0, &[0b0111110, 0b0011100], PINK);
            draw_pixel_pattern(
                x,
                y + 46.0,
                4.0,
                &[0b0010000, 0b0111110, 0b1111111, 0b1111111, 0b1111111],
                hair,
            );
            draw_pixel_pattern(x + 14.0, y + 47.0, 4.0, &[0b0011100, 0b0011100], GOLD);
        }
        Rank::King => {
            let beard = Color::new(0.42, 0.22, 0.11, 1.0);
            let robe = Color::new(0.43, 0.2, 0.58, 1.0);
            draw_pixel_pattern(x + 3.0, y, 4.0, &[0b1010101, 0b1111111, 0b1111111], GOLD);
            draw_pixel_pattern(
                x + 3.0,
                y + 13.0,
                4.0,
                &[0b0111110, 0b1111111, 0b1111111, 0b1111111, 0b0111110],
                skin,
            );
            draw_rectangle(x + 10.0, y + 25.0, 4.0, 4.0, ink);
            draw_rectangle(x + 22.0, y + 25.0, 4.0, 4.0, ink);
            draw_pixel_pattern(
                x + 7.0,
                y + 32.0,
                4.0,
                &[0b0111110, 0b0111110, 0b0011100, 0b0011100],
                beard,
            );
            draw_pixel_pattern(
                x,
                y + 48.0,
                4.0,
                &[0b0010000, 0b0111110, 0b1111111, 0b1111111, 0b1111111],
                robe,
            );
            draw_pixel_pattern(x + 15.0, y + 49.0, 4.0, &[0b0011100, 0b0011100], GOLD);
        }
        _ => (),
    }
}

fn has_face_portrait(rank: Rank) -> bool {
    matches!(rank, Rank::Jack | Rank::Queen | Rank::King)
}

fn player_display_order(game: &Game, phase: PlayerPhase) -> Vec<usize> {
    let count = player_card_count(game, phase);
    if phase != PlayerPhase::Hand {
        return (0..count).collect();
    }

    let mut indices = (0..count).collect::<Vec<_>>();
    if let Some(hand) = game.hand(0) {
        indices.sort_by_key(|index| {
            let card = hand[*index];
            (rank_sort_key(card.rank), suit_sort_key(card.suit))
        });
    }
    indices
}

fn rank_sort_key(rank: Rank) -> u8 {
    match rank {
        Rank::Three => 0,
        Rank::Four => 1,
        Rank::Five => 2,
        Rank::Six => 3,
        Rank::Seven => 4,
        Rank::Eight => 5,
        Rank::Nine => 6,
        Rank::Jack => 7,
        Rank::Queen => 8,
        Rank::King => 9,
        Rank::Ace => 10,
        Rank::Two => 11,
        Rank::Ten => 12,
    }
}

fn suit_sort_key(suit: Suit) -> u8 {
    match suit {
        Suit::Clubs => 0,
        Suit::Diamonds => 1,
        Suit::Hearts => 2,
        Suit::Spades => 3,
    }
}

fn draw_card_back(x: f32, y: f32, width: f32, highlighted: bool) {
    let color = Color::new(0.12, 0.28, 0.62, 1.0);
    draw_rectangle(x, y, width, CARD_HEIGHT, color);
    draw_rectangle_lines(
        x,
        y,
        width,
        CARD_HEIGHT,
        if highlighted { 4.0 } else { 2.0 },
        if highlighted { GOLD } else { WHITE },
    );
    draw_text("CARD", x + width / 2.0 - 19.0, y + 79.0, 17.0, WHITE);
}

fn card_at_position(
    mouse_x: f32,
    mouse_y: f32,
    count: usize,
    width: f32,
    height: f32,
) -> Option<usize> {
    let (card_width, gap, start_x) = card_layout(count, width);
    let start_y = height - CARD_HEIGHT - 154.0;
    (0..count).rev().find(|index| {
        let x = start_x + *index as f32 * (card_width + gap);
        let y = start_y;
        mouse_x >= x && mouse_x <= x + card_width && mouse_y >= y && mouse_y <= y + CARD_HEIGHT
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

fn selected_indices(selected: &[usize], cursor: usize, phase: PlayerPhase) -> Vec<usize> {
    if phase == PlayerPhase::FaceDown {
        return vec![cursor];
    }
    selected.to_vec()
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
            format!("{actor}drew and played {}.", format_card(card))
        } else {
            format!("{actor}drew {}, picked up the pile.", format_card(card))
        }
    } else if !outcome.played_cards.is_empty() {
        let cards = outcome
            .played_cards
            .iter()
            .map(|card| format_card(*card))
            .collect::<Vec<_>>()
            .join(" and ");
        format!("{actor}played {cards}.")
    } else if let Some(card) = outcome.played {
        format!("{actor}turned over {}.", format_card(card))
    } else {
        format!("{actor}picked up the pile.")
    };
    if outcome.burned {
        message.push_str(" Pile cleared - extra turn!");
    } else if outcome.extra_turn {
        message.push_str(" Extra turn!");
    }
    if outcome.picked_up > 0 && outcome.drawn.is_none() {
        message.push_str(&format!(" Picked up {} cards.", outcome.picked_up));
    }
    message
}

fn format_card(card: Card) -> String {
    format!("{} {}", rank_label(card.rank), suit_letter(card.suit))
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

fn suit_letter(suit: Suit) -> &'static str {
    match suit {
        Suit::Clubs => "C",
        Suit::Diamonds => "D",
        Suit::Hearts => "H",
        Suit::Spades => "S",
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

    #[test]
    fn enter_uses_only_toggled_cards() {
        assert_eq!(
            selected_indices(&[], 0, PlayerPhase::Hand),
            Vec::<usize>::new()
        );
        assert_eq!(selected_indices(&[1, 2], 0, PlayerPhase::Hand), vec![1, 2]);
        assert_eq!(selected_indices(&[], 2, PlayerPhase::FaceDown), vec![2]);
    }

    #[test]
    fn mouse_position_selects_card_bounds() {
        assert_eq!(card_at_position(250.0, 530.0, 3, 800.0, 760.0), Some(0));
        assert_eq!(card_at_position(340.0, 530.0, 3, 800.0, 760.0), None);
    }

    #[test]
    fn overlapping_cards_remain_individually_selectable() {
        let count = 30;
        let width = 800.0;
        let height = 760.0;
        let (card_width, gap, start_x) = card_layout(count, width);
        let step = card_width + gap;
        let y = height - CARD_HEIGHT - 154.0 + CARD_HEIGHT / 2.0;

        assert!(step > 0.0);
        for index in 0..count {
            let x = start_x + index as f32 * step;
            let click_x = if index + 1 == count {
                x + 5.0
            } else {
                x + step / 2.0
            };
            assert_eq!(
                card_at_position(click_x, y, count, width, height),
                Some(index)
            );
        }
    }

    #[test]
    fn action_button_hit_regions_match_their_layout() {
        for width in [360.0, 800.0, 1100.0] {
            let height = 760.0;
            let (play, draw, pickup) = action_button_rects(width, height);
            let center_y = play.y + play.h / 2.0;

            assert!(play_button_contains(
                play.x + play.w / 2.0,
                center_y,
                width,
                height
            ));
            assert!(draw_button_contains_at(
                draw.x + draw.w / 2.0,
                center_y,
                width,
                height
            ));
            assert!(pickup_button_contains(
                pickup.x + pickup.w / 2.0,
                center_y,
                width,
                height
            ));
            assert!(!play_button_contains(
                draw.x + draw.w / 2.0,
                center_y,
                width,
                height
            ));
        }
    }

    #[test]
    fn opponent_grid_keeps_all_players_in_the_reserved_area() {
        assert_eq!(opponent_columns(4, 990.0), 2);
        assert_eq!(opponent_columns(4, 1100.0), 3);
        assert_eq!(opponent_columns(4, 500.0), 1);
    }

    #[test]
    fn pile_hit_region_matches_its_draw_position() {
        for (width, height) in [(500.0, 600.0), (990.0, 700.0), (1100.0, 760.0)] {
            let pile = pile_card_rect(width, height);
            assert!(pile_target_contains(
                pile.x + pile.w / 2.0,
                pile.y + pile.h / 2.0,
                width,
                height
            ));
            assert!(!pile_target_contains(pile.x - 1.0, pile.y, width, height));
        }
    }

    #[test]
    fn hand_sort_places_twos_and_tens_after_aces() {
        let ranks = [
            Rank::Ace,
            Rank::Four,
            Rank::Ten,
            Rank::Two,
            Rank::King,
            Rank::Three,
        ];
        let mut sorted = ranks;
        sorted.sort_by_key(|rank| rank_sort_key(*rank));
        assert_eq!(
            sorted,
            [
                Rank::Three,
                Rank::Four,
                Rank::King,
                Rank::Ace,
                Rank::Two,
                Rank::Ten
            ]
        );
    }

    #[test]
    fn pixel_font_has_distinct_rank_glyphs() {
        for glyph in [
            '2', '3', '4', '5', '6', '7', '8', '9', '1', '0', 'A', 'J', 'Q', 'K',
        ] {
            assert_ne!(pixel_glyph(glyph), [0; 5]);
        }
        assert!(pixel_text_width("10", 5.0) > pixel_text_width("A", 5.0));
    }

    #[test]
    fn only_jack_queen_and_king_have_face_portraits() {
        for rank in [Rank::Jack, Rank::Queen, Rank::King] {
            assert!(has_face_portrait(rank));
        }
        for rank in [
            Rank::Two,
            Rank::Three,
            Rank::Four,
            Rank::Five,
            Rank::Six,
            Rank::Seven,
            Rank::Eight,
            Rank::Nine,
            Rank::Ten,
            Rank::Ace,
        ] {
            assert!(!has_face_portrait(rank));
        }
    }
}
