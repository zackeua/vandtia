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
                toggle_selection(&mut selected, cursor);
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
                if let Some(index) =
                    card_at_position(mouse_x, mouse_y, count, screen_width(), screen_height())
                {
                    cursor = index;
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

    for index in 0..count {
        let x = start_x + index as f32 * (card_width + card_gap);
        let is_selected = selected.contains(&index);
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
        } else if let Some(card) = cards.get(index) {
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
    let fill = if selected {
        Color::new(1.0, 0.96, 0.78, 1.0)
    } else {
        WHITE
    };
    let outline = if selected {
        GREEN
    } else if focused {
        GOLD
    } else {
        LIGHTGRAY
    };
    draw_rectangle(x, y, width, CARD_HEIGHT, fill);
    draw_rectangle_lines(
        x,
        y,
        width,
        CARD_HEIGHT,
        if selected || focused { 4.0 } else { 2.0 },
        outline,
    );
    let color = suit_color(card.suit);
    draw_text(rank_label(card.rank), x + 8.0, y + 26.0, 23.0, BLACK);
    draw_text(suit_letter(card.suit), x + 8.0, y + 47.0, 20.0, color);
    draw_text(
        suit_letter(card.suit),
        x + width / 2.0 - 13.0,
        y + 94.0,
        38.0,
        color,
    );
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
}
