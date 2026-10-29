#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, process};

use macroquad::prelude::*;
use vandtia::card_renderer::{self, CARD_HEIGHT, CARD_WIDTH, CardStyle};
use vandtia::{Card, Game, PlayerPhase, Rank, Suit, TurnOutcome, simulator::play_computer_turn};

const CARD_GAP: f32 = 14.0;
const ACTION_BUTTON_HEIGHT: f32 = 38.0;
const COMPACT_ACTION_BUTTON_HEIGHT: f32 = 58.0;
const NEXT_TURN_DELAY: f64 = 0.7;
const INITIAL_STATUS: &str = "Markera kort och klicka på SPELA MARKERADE eller spelhögen.";
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
    let (mut players, seed) = parse_args()?;
    let mut game_session = None;
    let mut cursor = 0usize;
    let mut selected = Vec::new();
    let mut status = INITIAL_STATUS.to_owned();
    let mut ai_next_at = 0.0;

    loop {
        if game_session.is_none() {
            draw_setup_screen(players, seed.is_some(), screen_width(), screen_height());
            if is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::Down) {
                players = players.saturating_sub(1).max(2);
            }
            if is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::Up) {
                players = (players + 1).min(5);
            }
            if is_mouse_button_pressed(MouseButton::Left) {
                let (mouse_x, mouse_y) = mouse_position();
                let (minus, plus) = setup_player_buttons(screen_width(), screen_height());
                if draw_button_contains(minus, mouse_x, mouse_y) {
                    players = players.saturating_sub(1).max(2);
                } else if draw_button_contains(plus, mouse_x, mouse_y) {
                    players = (players + 1).min(5);
                }
            }
            if is_key_pressed(KeyCode::Enter)
                || (is_mouse_button_pressed(MouseButton::Left)
                    && start_button_contains(mouse_position().0, mouse_position().1))
            {
                let game_seed = match seed {
                    Some(seed) => seed,
                    None => random_seed()?,
                };
                game_session = Some(Game::new(players, game_seed)?);
            }
            if is_key_pressed(KeyCode::Q) || is_key_pressed(KeyCode::Escape) {
                break;
            }
            next_frame().await;
            continue;
        }

        let game = game_session.as_mut().expect("game initialized above");
        let current_player = game.current_player();
        let phase = game.player_phase(0).unwrap_or(PlayerPhase::FaceDown);
        let display_order = player_display_order(game, phase);
        if game.winner().is_none() && current_player != 0 && get_time() >= ai_next_at {
            match play_computer_turn(game) {
                Ok(outcome) => {
                    record_successful_turn(
                        outcome,
                        &format!("Bot {} ", current_player + 1),
                        &mut status,
                        &mut cursor,
                        &mut selected,
                        &mut ai_next_at,
                    );
                }
                Err(error) => return Err(error.into()),
            }
        }

        if let Some(winner) = game.winner() {
            draw_game_over_screen(winner, screen_width(), screen_height());
            let (mouse_x, mouse_y) = mouse_position();
            let return_to_menu = is_key_pressed(KeyCode::Enter)
                || is_key_pressed(KeyCode::M)
                || (is_mouse_button_pressed(MouseButton::Left)
                    && menu_button_contains(mouse_x, mouse_y));
            if return_to_menu {
                game_session = None;
                cursor = 0;
                selected.clear();
                status = INITIAL_STATUS.to_owned();
                ai_next_at = 0.0;
            }
            if is_key_pressed(KeyCode::Q) || is_key_pressed(KeyCode::Escape) {
                break;
            }
            next_frame().await;
            continue;
        }

        draw_screen(game, cursor, &selected, &status);

        if current_player == 0 {
            let count = player_card_count(game, phase);
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
                        record_successful_turn(
                            outcome,
                            "Du ",
                            &mut status,
                            &mut cursor,
                            &mut selected,
                            &mut ai_next_at,
                        );
                    }
                    Err(error) => status = format!("Kunde inte dra kort: {error}"),
                }
            } else if is_key_pressed(KeyCode::P)
                && phase != PlayerPhase::FaceDown
                && !game.pile().is_empty()
            {
                match game.pick_up_pile() {
                    Ok(outcome) => {
                        record_successful_turn(
                            outcome,
                            "Du ",
                            &mut status,
                            &mut cursor,
                            &mut selected,
                            &mut ai_next_at,
                        );
                    }
                    Err(error) => status = format!("Kunde inte ta upp spelhögen: {error}"),
                }
            } else if is_key_pressed(KeyCode::Enter) && count > 0 {
                let indices = selected_indices(&selected, cursor, phase);
                if phase == PlayerPhase::FaceDown {
                    match game.play_face_down_at(cursor) {
                        Ok(outcome) => {
                            record_successful_turn(
                                outcome,
                                "Du ",
                                &mut status,
                                &mut cursor,
                                &mut selected,
                                &mut ai_next_at,
                            );
                        }
                        Err(error) => status = format!("Kunde inte vända kortet: {error}"),
                    }
                } else if indices.is_empty() {
                    status = "Markera minst ett kort först.".to_owned();
                } else {
                    play_selected(
                        game,
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
                                record_successful_turn(
                                    outcome,
                                    "Du ",
                                    &mut status,
                                    &mut cursor,
                                    &mut selected,
                                    &mut ai_next_at,
                                );
                            }
                            Err(error) => status = format!("Kunde inte vända kortet: {error}"),
                        }
                    } else {
                        toggle_selection(&mut selected, index);
                        status = selection_status(&selected);
                    }
                } else if phase != PlayerPhase::FaceDown
                    && play_button_contains(mouse_x, mouse_y, screen_width(), screen_height())
                {
                    if selected.is_empty() {
                        status = "Markera minst ett kort först.".to_owned();
                    } else {
                        let indices = selected.clone();
                        play_selected(
                            game,
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
                            record_successful_turn(
                                outcome,
                                "Du ",
                                &mut status,
                                &mut cursor,
                                &mut selected,
                                &mut ai_next_at,
                            );
                        }
                        Err(error) => status = format!("Kunde inte dra kort: {error}"),
                    }
                } else if phase != PlayerPhase::FaceDown
                    && !game.pile().is_empty()
                    && pickup_button_contains(mouse_x, mouse_y, screen_width(), screen_height())
                {
                    match game.pick_up_pile() {
                        Ok(outcome) => {
                            record_successful_turn(
                                outcome,
                                "Du ",
                                &mut status,
                                &mut cursor,
                                &mut selected,
                                &mut ai_next_at,
                            );
                        }
                        Err(error) => status = format!("Kunde inte ta upp spelhögen: {error}"),
                    }
                } else if phase != PlayerPhase::FaceDown
                    && pile_target_contains(mouse_x, mouse_y, screen_width(), screen_height())
                {
                    if selected.is_empty() {
                        status = "Markera minst ett kort först.".to_owned();
                    } else {
                        let indices = selected.clone();
                        play_selected(
                            game,
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
            record_successful_turn(outcome, "Du ", status, cursor, selection, ai_next_at);
        }
        Err(error) => *status = format!("Kunde inte spela de markerade korten: {error}"),
    }
}

fn record_successful_turn(
    outcome: TurnOutcome,
    actor: &str,
    status: &mut String,
    cursor: &mut usize,
    selection: &mut Vec<usize>,
    ai_next_at: &mut f64,
) {
    *status = format_outcome(actor, &outcome);
    *cursor = 0;
    selection.clear();
    *ai_next_at = get_time() + NEXT_TURN_DELAY;
}

fn selection_status(selected: &[usize]) -> String {
    if selected.is_empty() {
        "Markeringen har tagits bort.".to_owned()
    } else {
        format!(
            "{} kort markerade. Klicka på SPELA MARKERADE eller spelhögen.",
            selected.len()
        )
    }
}

fn parse_args() -> Result<(usize, Option<u64>), Box<dyn std::error::Error>> {
    let mut players = 4usize;
    let mut seed = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--players" | "-p" => {
                players = args.next().ok_or("--players kräver ett värde")?.parse()?;
            }
            "--seed" | "-s" => {
                seed = Some(args.next().ok_or("--seed kräver ett värde")?.parse()?);
            }
            "--help" | "-h" => {
                println!("Användning: vandtia-graphics [--players 2-5] [--seed N]");
                println!("Utan --seed skapas ett nytt startvärde när spelet startar.");
                process::exit(0);
            }
            unknown => return Err(format!("okänt argument: {unknown}").into()),
        }
    }
    if !(2..=5).contains(&players) {
        return Err("--players måste vara mellan 2 och 5".into());
    }
    Ok((players, seed))
}

#[cfg(not(target_arch = "wasm32"))]
fn random_seed() -> Result<u64, std::time::SystemTimeError> {
    let time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64;
    let process_salt = u64::from(process::id()).rotate_left(17);
    let frame_salt = (get_time() * 1_000_000_000.0) as u64;
    Ok(time ^ process_salt ^ frame_salt.rotate_left(31))
}

#[cfg(target_arch = "wasm32")]
fn random_seed() -> Result<u64, std::convert::Infallible> {
    let clock = (get_time() * 1_000_000_000.0) as u64;
    let random = u64::from(macroquad::rand::gen_range(0_u32, u32::MAX));
    Ok(clock ^ random.rotate_left(29) ^ (random << 32))
}

fn draw_setup_screen(players: usize, fixed_seed: bool, width: f32, height: f32) {
    let compact = compact_layout(width, height);
    let short = height < 520.0;
    clear_background(TABLE_COLOR);
    draw_centered_text(
        "VÄNDTIA",
        width / 2.0,
        height * if short { 0.16 } else { 0.22 },
        if compact { 36 } else { 48 },
        GOLD,
    );
    draw_centered_text(
        "Välj antal spelare",
        width / 2.0,
        height * if short { 0.28 } else { 0.36 },
        if compact { 20 } else { 24 },
        WHITE,
    );

    let center_y = setup_player_selector_y(height);
    let (minus, plus) = setup_player_buttons(width, height);
    draw_setup_button(minus, "-");
    draw_setup_button(plus, "+");
    draw_text(
        players.to_string(),
        width / 2.0 - 14.0,
        center_y + 17.0,
        42.0,
        WHITE,
    );
    if !compact {
        draw_centered_text(
            "Använd piltangenterna eller knapparna (2-5 spelare)",
            width / 2.0,
            center_y + 61.0,
            18,
            LIGHTGRAY,
        );
    }
    draw_centered_text(
        if fixed_seed {
            if compact {
                "Fast startvärde"
            } else {
                "Använder startvärdet från kommandoraden"
            }
        } else if compact {
            "Slumpat startvärde"
        } else {
            "Ett slumpat startvärde skapas för spelet"
        },
        width / 2.0,
        center_y + if short { 50.0 } else { 96.0 },
        if compact { 15 } else { 17 },
        LIGHTGRAY,
    );

    let start = setup_start_button(width, height);
    draw_setup_button(start, "STARTA SPELET");
    if !compact {
        draw_centered_text(
            "RETUR: STARTA    Q: AVSLUTA",
            width / 2.0,
            start.y + start.h + 33.0,
            17,
            LIGHTGRAY,
        );
    }
}

fn setup_player_buttons(width: f32, height: f32) -> (Rect, Rect) {
    let center_y = setup_player_selector_y(height);
    let size = if compact_layout(width, height) {
        66.0
    } else {
        58.0
    };
    let distance = 117.0_f32.min((width - size - 32.0).max(0.0) / 2.0);
    (
        Rect::new(
            width / 2.0 - distance - size / 2.0,
            center_y - size / 2.0,
            size,
            size,
        ),
        Rect::new(
            width / 2.0 + distance - size / 2.0,
            center_y - size / 2.0,
            size,
            size,
        ),
    )
}

fn setup_player_selector_y(height: f32) -> f32 {
    height * if height < 520.0 { 0.44 } else { 0.51 }
}

fn setup_start_button(width: f32, height: f32) -> Rect {
    let button_width = (width - 40.0).clamp(210.0, 300.0);
    let button_height = if compact_layout(width, height) {
        64.0
    } else {
        52.0
    };
    Rect::new(
        (width - button_width) / 2.0,
        height * 0.72,
        button_width,
        button_height,
    )
}

fn start_button_contains(x: f32, y: f32) -> bool {
    let start = setup_start_button(screen_width(), screen_height());
    draw_button_contains(start, x, y)
}

fn menu_button_rect(width: f32, height: f32) -> Rect {
    let button_width = (width - 32.0).clamp(170.0, 320.0);
    let button_height = if compact_layout(width, height) {
        64.0
    } else {
        54.0
    };
    Rect::new(
        (width - button_width) / 2.0,
        height * 0.62,
        button_width,
        button_height,
    )
}

fn menu_button_contains(x: f32, y: f32) -> bool {
    draw_button_contains(menu_button_rect(screen_width(), screen_height()), x, y)
}

fn draw_game_over_screen(winner: usize, width: f32, height: f32) {
    let compact = compact_layout(width, height);
    clear_background(TABLE_COLOR);
    let title = if winner == 0 {
        "DU VANN!"
    } else {
        "SPELET ÄR SLUT"
    };
    draw_centered_text(
        title,
        width / 2.0,
        height * 0.28,
        if compact { 34 } else { 42 },
        GOLD,
    );
    let message = if winner == 0 {
        "Grattis!"
    } else {
        "En bot vann!"
    };
    draw_centered_text(
        message,
        width / 2.0,
        height * 0.43,
        if compact { 24 } else { 30 },
        WHITE,
    );
    if winner > 0 {
        let bot = format!("Bot {} vann", winner + 1);
        draw_centered_text(&bot, width / 2.0, height * 0.50, 20, LIGHTGRAY);
    }

    let menu_button = menu_button_rect(width, height);
    draw_setup_button(menu_button, "TILLBAKA TILL MENYN");
    if !compact {
        draw_centered_text(
            "RETUR / M: MENY    Q: AVSLUTA",
            width / 2.0,
            menu_button.y + menu_button.h + 34.0,
            17,
            LIGHTGRAY,
        );
    }
}

fn draw_centered_text(text: &str, center_x: f32, y: f32, font_size: u16, color: Color) {
    let text_width = measure_text(text, None, font_size, 1.0).width;
    draw_text(
        text,
        center_x - text_width / 2.0,
        y,
        f32::from(font_size),
        color,
    );
}

fn draw_setup_button(rect: Rect, label: &str) {
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        Color::new(0.16, 0.48, 0.32, 1.0),
    );
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 3.0, GOLD);
    let font_size = if rect.h >= 64.0 { 22 } else { 24 };
    let text_width = measure_text(label, None, font_size, 1.0).width;
    draw_text(
        label,
        rect.x + (rect.w - text_width) / 2.0,
        rect.y + rect.h / 2.0 + f32::from(font_size) * 0.35,
        f32::from(font_size),
        WHITE,
    );
}

fn draw_screen(game: &Game, cursor: usize, selected: &[usize], status: &str) {
    clear_background(TABLE_COLOR);
    let width = screen_width();
    let height = screen_height();
    let compact = compact_layout(width, height);

    if compact {
        draw_text("VÄNDTIA", 12.0, 28.0, 20.0, GOLD);
    } else {
        draw_text("VÄNDTIA", 28.0, 42.0, 34.0, GOLD);
        draw_text(
            format!(
                "DRAGHÖG: {} kort     SPELHÖG: {} kort",
                game.draw_pile_size(),
                game.pile().len()
            ),
            240.0,
            38.0,
            22.0,
            WHITE,
        );
        draw_text(
            "FÄRGER: C=KLÖVER  D=RUTER  H=HJÄRTER  S=SPADER",
            28.0,
            61.0,
            15.0,
            LIGHTGRAY,
        );
    }
    let current = if game.current_player() == 0 {
        "Din tur".to_owned()
    } else {
        format!("Bot {}:s tur", game.current_player() + 1)
    };
    draw_text(
        &current,
        if compact {
            width - 110.0
        } else {
            width - 210.0
        },
        if compact { 28.0 } else { 38.0 },
        if compact { 16.0 } else { 22.0 },
        SKYBLUE,
    );

    draw_player_tables(game, width, height);
    draw_pile(game, width, height, compact);
    draw_player_cards(game, cursor, selected, width, height);
    draw_controls(game, status, width, height, compact);
}

fn draw_player_tables(game: &Game, width: f32, height: f32) {
    if compact_layout(width, height) {
        draw_compact_player_tables(game, width);
        return;
    }

    let columns = opponent_columns(game.player_count(), width);
    for player in 0..game.player_count() {
        let x = 28.0 + (player % columns) as f32 * 234.0;
        let y = 76.0 + (player / columns) as f32 * 154.0;
        draw_rectangle(x, y, 220.0, 140.0, PANEL_COLOR);
        draw_rectangle_lines(x, y, 220.0, 140.0, 2.0, DARKGREEN);
        let label = if player == 0 {
            "YOU".to_owned()
        } else {
            format!("BOT {}", player + 1)
        };
        let hand_count = game.hand_count(player).unwrap_or(0);
        draw_text(
            format!("{label}: {hand_count} PÅ HANDEN"),
            x + 12.0,
            y + 27.0,
            16.0,
            WHITE,
        );
        for (slot, (has_face_down, face_up_card)) in table_stack_layers(
            game.face_up(player).unwrap_or(&[]),
            game.face_down_count(player).unwrap_or(0),
        )
        .into_iter()
        .enumerate()
        {
            let card_x = x + 12.0 + slot as f32 * 58.0;
            let card_y = y + 48.0;
            if has_face_down {
                card_renderer::draw_back(Rect::new(card_x, card_y + 5.0, 54.0, 78.0), false);
            }
            if let Some(card) = face_up_card {
                card_renderer::draw_face(
                    Rect::new(card_x, card_y, 54.0, 78.0),
                    card,
                    CardStyle::default(),
                );
            }
        }
    }
}

fn draw_compact_player_tables(game: &Game, width: f32) {
    let columns = ((width / 170.0).floor() as usize)
        .max(1)
        .min(game.player_count());
    let gap = 6.0;
    let cell_width = (width - 16.0 - gap * (columns - 1) as f32) / columns as f32;
    let cell_height = 42.0;

    for player in 0..game.player_count() {
        let column = player % columns;
        let row = player / columns;
        let x = 8.0 + column as f32 * (cell_width + gap);
        let y = 42.0 + row as f32 * (cell_height + gap);
        let border = if player == game.current_player() {
            GOLD
        } else {
            DARKGREEN
        };
        draw_rectangle(x, y, cell_width, cell_height, PANEL_COLOR);
        draw_rectangle_lines(x, y, cell_width, cell_height, 2.0, border);

        let label = if player == 0 {
            "DU".to_owned()
        } else {
            format!("BOT {}", player + 1)
        };
        draw_text(
            format!("{label}  {}", game.hand_count(player).unwrap_or(0)),
            x + 7.0,
            y + 16.0,
            12.0,
            WHITE,
        );

        for (index, card) in game
            .face_up(player)
            .unwrap_or(&[])
            .iter()
            .copied()
            .take(3)
            .enumerate()
        {
            card_renderer::draw_face(
                Rect::new(
                    x + cell_width - 32.0 * (index + 1) as f32,
                    y + 4.0,
                    27.0,
                    34.0,
                ),
                card,
                CardStyle::default(),
            );
        }
    }
}

fn opponent_columns(player_count: usize, width: f32) -> usize {
    let available_width = (width - 56.0).max(220.0);
    let columns = ((available_width + 14.0) / 234.0).floor() as usize;
    columns.max(1).min(player_count.max(1))
}

fn table_stack_layers(face_up: &[Card], face_down_count: usize) -> [(bool, Option<Card>); 3] {
    std::array::from_fn(|index| (index < face_down_count, face_up.get(index).copied()))
}

fn draw_pile(game: &Game, width: f32, height: f32, compact: bool) {
    let pile = pile_card_rect(width, height);
    let center_x = pile.x + pile.w / 2.0;
    if !compact {
        draw_centered_text(
            "SPELHÖG - KLICKA FÖR ATT SPELA",
            center_x,
            pile.y - 14.0,
            18,
            LIGHTGRAY,
        );
    }
    if let Some(card) = game.pile().last() {
        card_renderer::draw_face(pile, *card, CardStyle::default());
        if !compact {
            draw_centered_text(
                &format!("{} kort i spelhögen", game.pile().len()),
                center_x,
                pile.y + pile.h + 22.0,
                18,
                WHITE,
            );
        }
    } else {
        draw_rectangle(
            pile.x,
            pile.y,
            pile.w,
            pile.h,
            Color::new(0.03, 0.2, 0.14, 1.0),
        );
        draw_rectangle_lines(pile.x, pile.y, pile.w, pile.h, 2.0, LIGHTGRAY);
        draw_centered_text("TOM", center_x, pile.y + 78.0, 18, LIGHTGRAY);
    }
}

fn pile_card_rect(width: f32, height: f32) -> Rect {
    let compact = compact_layout(width, height);
    let card_height = card_height_for_screen(width, height);
    let card_width = card_height * CARD_WIDTH / CARD_HEIGHT;
    let center_x = (width * 0.82).clamp(card_width / 2.0 + 12.0, width - card_width / 2.0 - 12.0);
    let center_y = if compact {
        height * 0.40
    } else {
        height * 0.36
    };
    Rect::new(
        center_x - card_width / 2.0,
        center_y - card_height * 0.34,
        card_width,
        card_height,
    )
}

fn draw_player_cards(game: &Game, cursor: usize, selected: &[usize], width: f32, height: f32) {
    let compact = compact_layout(width, height);
    let card_height = card_height_for_screen(width, height);
    let phase = game.player_phase(0).unwrap_or(PlayerPhase::FaceDown);
    let cards = player_cards(game, phase);
    let count = player_card_count(game, phase);
    let title = match phase {
        PlayerPhase::Hand => "DIN HAND",
        PlayerPhase::FaceUp => "DINA ÖPPNA KORT",
        PlayerPhase::FaceDown => "DINA BLINDA KORT",
    };
    let start_y = player_cards_y(width, height, card_height);
    draw_text(
        title,
        if compact { 12.0 } else { 28.0 },
        start_y - if compact { 10.0 } else { 16.0 },
        if compact { 14.0 } else { 20.0 },
        GOLD,
    );
    let (card_width, card_gap, start_x) = card_layout(count, width, card_height, compact);
    let display_order = player_display_order(game, phase);

    for (index, card_index) in display_order.iter().copied().enumerate() {
        let x = start_x + index as f32 * (card_width + card_gap);
        let is_selected = selected.contains(&card_index);
        let y = start_y;
        if phase == PlayerPhase::FaceDown {
            card_renderer::draw_back(Rect::new(x, y, card_width, card_height), index == cursor);
            draw_text(
                format!("{}", index + 1),
                x + card_width / 2.0 - 5.0,
                y + card_height + if compact { 16.0 } else { 19.0 },
                if compact { 12.0 } else { 16.0 },
                LIGHTGRAY,
            );
        } else if let Some(card) = cards.get(card_index) {
            card_renderer::draw_face(
                Rect::new(x, y, card_width, card_height),
                *card,
                CardStyle {
                    selected: is_selected,
                    focused: index == cursor,
                },
            );
            draw_text(
                format!("{}", index + 1),
                x + card_width / 2.0 - 5.0,
                y + card_height + if compact { 16.0 } else { 19.0 },
                if compact { 12.0 } else { 16.0 },
                WHITE,
            );
        }
    }
    if phase != PlayerPhase::FaceDown {
        draw_action_buttons(game, phase, width, height, selected.len());
    }
}

#[derive(Clone, Copy)]
struct ActionButtonRects {
    play: Rect,
    draw: Rect,
    pickup: Rect,
}

fn action_button_rects(width: f32, height: f32) -> ActionButtonRects {
    if compact_layout(width, height) {
        let margin = 10.0;
        let gap = 6.0;
        let button_width = (width - margin * 2.0 - gap * 2.0) / 3.0;
        let y = height - COMPACT_ACTION_BUTTON_HEIGHT - 24.0;
        return ActionButtonRects {
            play: Rect::new(margin, y, button_width, COMPACT_ACTION_BUTTON_HEIGHT),
            draw: Rect::new(
                margin + button_width + gap,
                y,
                button_width,
                COMPACT_ACTION_BUTTON_HEIGHT,
            ),
            pickup: Rect::new(
                margin + (button_width + gap) * 2.0,
                y,
                button_width,
                COMPACT_ACTION_BUTTON_HEIGHT,
            ),
        };
    }

    let scale = ((width - 24.0) / 464.0).clamp(0.0, 1.0);
    let play_width = 180.0 * scale;
    let other_width = 130.0 * scale;
    let gap = 12.0 * scale;
    let total_width = play_width + other_width * 2.0 + gap * 2.0;
    let left = ((width - total_width) / 2.0).max(12.0);
    let y = height - 101.0;
    ActionButtonRects {
        play: Rect::new(left, y, play_width, ACTION_BUTTON_HEIGHT),
        draw: Rect::new(
            left + play_width + gap,
            y,
            other_width,
            ACTION_BUTTON_HEIGHT,
        ),
        pickup: Rect::new(
            left + play_width + gap + other_width + gap,
            y,
            other_width,
            ACTION_BUTTON_HEIGHT,
        ),
    }
}

fn draw_action_buttons(
    game: &Game,
    phase: PlayerPhase,
    width: f32,
    height: f32,
    selected_count: usize,
) {
    let buttons = action_button_rects(width, height);
    let compact = compact_layout(width, height);
    let play_text = if compact {
        "SPELA"
    } else if selected_count == 0 {
        "MARKERA KORT"
    } else {
        "SPELA MARKERADE"
    };
    draw_button(buttons.play, play_text, selected_count > 0);

    if phase == PlayerPhase::Hand && game.draw_pile_size() > 0 {
        draw_button(
            buttons.draw,
            if compact { "DRA" } else { "DRA + SPELA" },
            true,
        );
    } else {
        draw_button(
            buttons.draw,
            if compact { "DRA" } else { "SLUT PÅ KORT" },
            false,
        );
    }

    if !game.pile().is_empty() && phase != PlayerPhase::FaceDown {
        draw_button(
            buttons.pickup,
            if compact { "PLOCKA" } else { "PLOCKA UPP" },
            true,
        );
    } else {
        draw_button(
            buttons.pickup,
            if compact { "PLOCKA" } else { "PLOCKA UPP" },
            false,
        );
    }
}

fn draw_button(rect: Rect, label: &str, enabled: bool) {
    let fill = if enabled {
        Color::new(0.18, 0.55, 0.35, 1.0)
    } else {
        Color::new(0.12, 0.19, 0.16, 1.0)
    };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        if enabled { 3.0 } else { 2.0 },
        if enabled { GOLD } else { DARKGRAY },
    );
    let base_font_size: u16 = if rect.h >= COMPACT_ACTION_BUTTON_HEIGHT {
        20
    } else {
        16
    };
    let text_width = measure_text(label, None, base_font_size, 1.0).width;
    let font_size = (f32::from(base_font_size) * (rect.w * 0.9 / text_width))
        .clamp(14.0, f32::from(base_font_size));
    draw_text(
        label,
        rect.x + (rect.w - text_width * font_size / f32::from(base_font_size)) / 2.0,
        rect.y + (rect.h + font_size * 0.62) / 2.0,
        font_size,
        if enabled { WHITE } else { LIGHTGRAY },
    );
}

fn draw_button_contains(rect: Rect, x: f32, y: f32) -> bool {
    x >= rect.x && x <= rect.x + rect.w && y >= rect.y && y <= rect.y + rect.h
}

fn play_button_contains(x: f32, y: f32, width: f32, height: f32) -> bool {
    draw_button_contains(action_button_rects(width, height).play, x, y)
}

fn draw_button_contains_at(x: f32, y: f32, width: f32, height: f32) -> bool {
    draw_button_contains(action_button_rects(width, height).draw, x, y)
}

fn pickup_button_contains(x: f32, y: f32, width: f32, height: f32) -> bool {
    draw_button_contains(action_button_rects(width, height).pickup, x, y)
}

fn pile_target_contains(x: f32, y: f32, width: f32, height: f32) -> bool {
    let pile = pile_card_rect(width, height);
    x >= pile.x && x <= pile.x + pile.w && y >= pile.y && y <= pile.y + pile.h
}

fn card_layout(
    count: usize,
    screen_width: f32,
    card_height: f32,
    compact: bool,
) -> (f32, f32, f32) {
    let available_width = (screen_width - if compact { 24.0 } else { 44.0 }).max(1.0);
    if count == 0 {
        return (card_height * CARD_WIDTH / CARD_HEIGHT, CARD_GAP, 12.0);
    }
    let count_f = count as f32;
    let natural_width = card_height * CARD_WIDTH / CARD_HEIGHT;
    let card_width = natural_width.min(available_width);
    let preferred_gap = CARD_GAP * card_height / CARD_HEIGHT;
    let gap =
        if count > 1 && count_f * (card_width + preferred_gap) - preferred_gap > available_width {
            (available_width - card_width * count_f) / (count_f - 1.0)
        } else if count > 1 {
            preferred_gap
        } else {
            0.0
        };
    let total_width = count_f * card_width + (count_f - 1.0).max(0.0) * gap;
    let start_x = ((screen_width - total_width) / 2.0).max(12.0);
    (card_width, gap, start_x)
}

fn draw_controls(game: &Game, status: &str, width: f32, height: f32, compact: bool) {
    if compact {
        if status != INITIAL_STATUS && status.starts_with("Kunde inte") {
            let font_size = 12;
            let text_width = measure_text(status, None, font_size, 1.0).width;
            let scale = ((width - 16.0) / text_width).min(1.0);
            let fitted_width = text_width * scale;
            draw_text(
                status,
                (width - fitted_width) / 2.0,
                action_button_rects(width, height).play.y - 8.0,
                f32::from(font_size) * scale,
                YELLOW,
            );
        }
        return;
    }
    let panel_y = height - 43.0;
    draw_rectangle(0.0, panel_y - 6.0, width, 52.0, PANEL_COLOR);
    draw_text(status, 20.0, panel_y + 12.0, 18.0, YELLOW);
    let phase = game.player_phase(0).unwrap_or(PlayerPhase::FaceDown);
    let controls = match phase {
        PlayerPhase::Hand => {
            "PILTANGENTER: FLYTTA  MELLANSLAG: MARKERA  RETUR: SPELA  D: DRA  P: PLOCKA UPP  Q: AVSLUTA"
        }
        PlayerPhase::FaceUp => {
            "PILTANGENTER: FLYTTA  MELLANSLAG: MARKERA  RETUR: SPELA  P: PLOCKA UPP  Q: AVSLUTA"
        }
        PlayerPhase::FaceDown => "PILTANGENTER: FLYTTA  RETUR: VÄND  Q: AVSLUTA",
    };
    draw_text(controls, 20.0, panel_y + 34.0, 14.0, LIGHTGRAY);
}

fn compact_layout(width: f32, height: f32) -> bool {
    width < 700.0 || height < 520.0
}

fn card_height_for_screen(width: f32, height: f32) -> f32 {
    if compact_layout(width, height) {
        compact_card_height(height)
    } else {
        (CARD_HEIGHT * height / 760.0).clamp(120.0, CARD_HEIGHT)
    }
}

fn compact_card_height(height: f32) -> f32 {
    (height * 0.24).clamp(86.0, 128.0)
}

fn player_cards_y(width: f32, height: f32, card_height: f32) -> f32 {
    let bottom_space = if compact_layout(width, height) {
        if height < 520.0 { 130.0 } else { 190.0 }
    } else {
        154.0
    };
    height - card_height - bottom_space
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

fn card_at_position(
    mouse_x: f32,
    mouse_y: f32,
    count: usize,
    width: f32,
    height: f32,
) -> Option<usize> {
    let compact = compact_layout(width, height);
    let card_height = card_height_for_screen(width, height);
    let (card_width, gap, start_x) = card_layout(count, width, card_height, compact);
    let start_y = player_cards_y(width, height, card_height);
    (0..count).rev().find(|index| {
        let x = start_x + *index as f32 * (card_width + gap);
        let y = start_y;
        mouse_x >= x && mouse_x <= x + card_width && mouse_y >= y && mouse_y <= y + card_height
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
            format!(
                "{actor}drog och spelade {}.",
                card_renderer::format_card(card)
            )
        } else {
            format!(
                "{actor}drog {} och tog upp spelhögen.",
                card_renderer::format_card(card)
            )
        }
    } else if !outcome.played_cards.is_empty() {
        let cards = outcome
            .played_cards
            .iter()
            .map(|card| card_renderer::format_card(*card))
            .collect::<Vec<_>>()
            .join(" och ");
        format!("{actor}spelade {cards}.")
    } else if let Some(card) = outcome.played {
        format!("{actor}vände {}.", card_renderer::format_card(card))
    } else {
        format!("{actor}plockade upp spelhögen.")
    };
    if outcome.burned {
        message.push_str(" Spelhögen tömdes - en tur till!");
    } else if outcome.extra_turn {
        message.push_str(" En tur till!");
    }
    if outcome.picked_up > 0 && outcome.drawn.is_none() {
        message.push_str(&format!(" {} kort plockades upp.", outcome.picked_up));
    }
    message
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
        let (card_width, gap, start_x) = card_layout(count, width, CARD_HEIGHT, false);
        let step = card_width + gap;
        let y = player_cards_y(width, height, CARD_HEIGHT) + CARD_HEIGHT / 2.0;

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
            let buttons = action_button_rects(width, height);
            let center_y = buttons.play.y + buttons.play.h / 2.0;

            assert!(play_button_contains(
                buttons.play.x + buttons.play.w / 2.0,
                center_y,
                width,
                height
            ));
            assert!(draw_button_contains_at(
                buttons.draw.x + buttons.draw.w / 2.0,
                center_y,
                width,
                height
            ));
            assert!(pickup_button_contains(
                buttons.pickup.x + buttons.pickup.w / 2.0,
                center_y,
                width,
                height
            ));
            assert!(!play_button_contains(
                buttons.draw.x + buttons.draw.w / 2.0,
                center_y,
                width,
                height
            ));
        }
    }

    #[test]
    fn compact_layout_gives_mobile_controls_and_cards_readable_dimensions() {
        for (width, height) in [(390.0, 844.0), (844.0, 390.0)] {
            assert!(compact_layout(width, height));

            let buttons = action_button_rects(width, height);
            assert_eq!(buttons.play.h, COMPACT_ACTION_BUTTON_HEIGHT);
            assert_eq!(buttons.draw.h, COMPACT_ACTION_BUTTON_HEIGHT);
            assert_eq!(buttons.pickup.h, COMPACT_ACTION_BUTTON_HEIGHT);
            assert!(buttons.play.w >= 80.0);
            assert!(buttons.draw.w >= 80.0);
            assert!(buttons.pickup.w >= 80.0);

            let card_height = card_height_for_screen(width, height);
            assert!(card_height >= 86.0);
            let (card_width, _, _) = card_layout(3, width, card_height, true);
            assert!(card_width >= 60.0);
            assert!(
                player_cards_y(width, height, card_height) + card_height + 16.0 < buttons.play.y
            );
        }
    }

    #[test]
    fn setup_controls_fit_narrow_mobile_widths() {
        for width in [280.0, 320.0, 390.0] {
            let (minus, plus) = setup_player_buttons(width, 700.0);
            let start = setup_start_button(width, 700.0);

            assert!(minus.x >= 0.0);
            assert!(plus.x + plus.w <= width);
            assert!(minus.w >= 64.0);
            assert!(start.x >= 0.0);
            assert!(start.x + start.w <= width);
            assert!(start.h >= 64.0);
        }
    }

    #[test]
    fn setup_screen_fits_short_landscape_viewports() {
        let height = 390.0;
        let center_y = setup_player_selector_y(height);
        let (_, plus) = setup_player_buttons(844.0, height);
        let start = setup_start_button(844.0, height);

        assert!(plus.y + plus.h < center_y + 50.0);
        assert!(center_y + 50.0 < start.y);
    }

    #[test]
    fn card_hit_regions_follow_responsive_card_dimensions() {
        let (width, height) = (390.0, 844.0);
        let card_height = card_height_for_screen(width, height);
        let (card_width, gap, start_x) = card_layout(3, width, card_height, true);
        let card_y = player_cards_y(width, height, card_height);

        assert_eq!(
            card_at_position(
                start_x + card_width / 2.0,
                card_y + card_height / 2.0,
                3,
                width,
                height,
            ),
            Some(0)
        );
        assert_eq!(
            card_at_position(
                start_x + card_width + gap / 2.0,
                card_y + card_height / 2.0,
                3,
                width,
                height,
            ),
            None
        );
    }

    #[test]
    fn opponent_grid_keeps_all_players_in_the_reserved_area() {
        assert_eq!(opponent_columns(4, 990.0), 4);
        assert_eq!(opponent_columns(4, 1100.0), 4);
        assert_eq!(opponent_columns(4, 500.0), 1);
        assert_eq!(4_usize.div_ceil(opponent_columns(4, 1100.0)), 1);
        assert_eq!(4_usize.div_ceil(opponent_columns(4, 500.0)), 4);
    }

    #[test]
    fn setup_controls_stay_within_the_player_selector() {
        let (minus, plus) = setup_player_buttons(1100.0, 760.0);
        let start = setup_start_button(1100.0, 760.0);
        assert_eq!(minus.y, plus.y);
        assert!(minus.y + minus.h < start.y);
        assert!(76.0 + 140.0 < player_cards_y(1100.0, 760.0, CARD_HEIGHT));
    }

    #[test]
    fn game_over_menu_button_is_centered_and_clickable() {
        let button = menu_button_rect(1100.0, 760.0);
        assert_eq!(button.x + button.w / 2.0, 550.0);
        assert_eq!(button.y + button.h / 2.0, 498.2);
        assert!(draw_button_contains(
            button,
            button.x + button.w / 2.0,
            button.y + button.h / 2.0
        ));
        assert!(!draw_button_contains(button, button.x - 1.0, button.y));
    }

    #[test]
    fn player_table_grid_includes_every_player() {
        assert_eq!(2_usize.div_ceil(opponent_columns(2, 1100.0)), 1);
        assert_eq!(5_usize.div_ceil(opponent_columns(5, 1100.0)), 2);
        assert_eq!(opponent_columns(5, 1100.0), 4);
    }

    #[test]
    fn table_stacks_show_face_up_cards_over_remaining_face_down_cards() {
        let face_up = [
            Card {
                rank: Rank::Jack,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Two,
                suit: Suit::Clubs,
            },
        ];
        assert_eq!(
            table_stack_layers(&face_up, 3),
            [
                (true, Some(face_up[0])),
                (true, Some(face_up[1])),
                (true, None),
            ]
        );
        assert_eq!(
            table_stack_layers(&[], 2),
            [(true, None), (true, None), (false, None)]
        );
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
}
