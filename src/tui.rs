use std::{
    error::Error,
    io::{self, Stdout},
    time::Duration,
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};
use vandtia::{
    Card, Game, MoveError, PlayerPhase, Rank, Suit, TurnOutcome, simulator::play_computer_turn,
};

type AppResult<T> = Result<T, Box<dyn Error>>;

pub fn run(player_count: usize, seed: u64) -> AppResult<()> {
    let mut game = Game::new(player_count, seed)?;
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    if let Err(error) = execute!(stdout, EnterAlternateScreen) {
        let _ = disable_raw_mode();
        return Err(error.into());
    }

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = match Terminal::new(backend) {
        Ok(terminal) => terminal,
        Err(error) => {
            let mut stdout = io::stdout();
            let _ = execute!(stdout, LeaveAlternateScreen);
            let _ = disable_raw_mode();
            return Err(error.into());
        }
    };

    let result = run_game(&mut terminal, &mut game);
    let restore_result = restore_terminal(&mut terminal);
    match (result, restore_result) {
        (Err(game_error), Err(restore_error)) => Err(format!(
            "{game_error}; dessutom misslyckades återställning av terminalen: {restore_error}"
        )
        .into()),
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error.into()),
        (Ok(()), Ok(())) => Ok(()),
    }
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
    let mut errors = Vec::new();
    if let Err(error) = disable_raw_mode() {
        errors.push(format!("raw mode: {error}"));
    }
    if let Err(error) = execute!(terminal.backend_mut(), LeaveAlternateScreen) {
        errors.push(format!("alternate screen: {error}"));
    }
    if let Err(error) = terminal.show_cursor() {
        errors.push(format!("cursor: {error}"));
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(io::Error::other(errors.join("; ")))
    }
}

fn run_game(terminal: &mut Terminal<CrosstermBackend<Stdout>>, game: &mut Game) -> AppResult<()> {
    let mut cursor = 0usize;
    let mut selected = Vec::new();
    let mut message = String::from("Vändtia — välkommen! Din tur visas med gul markering.");

    loop {
        terminal.draw(|frame| draw(frame, game, cursor, &selected, &message))?;
        if let Some(winner) = game.winner() {
            if winner == 0 {
                message = "Du vann! Tryck q eller Esc för att avsluta.".to_owned();
            } else {
                message = format!(
                    "Spelare {} vann. Tryck q eller Esc för att avsluta.",
                    winner + 1
                );
            }
            terminal.draw(|frame| draw(frame, game, cursor, &selected, &message))?;
            if wait_for_quit()? {
                return Ok(());
            }
            continue;
        }

        if game.current_player() != 0 {
            if wait_for_computer_turn()? {
                return Ok(());
            }
            let player = game.current_player();
            match play_computer_turn(game) {
                Ok(outcome) => {
                    message = format_outcome(&format!("Dator {} ", player + 1), &outcome);
                }
                Err(error) => return Err(error.into()),
            }
            cursor = 0;
            selected.clear();
            continue;
        }

        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind == KeyEventKind::Release {
            continue;
        }

        let phase = game.player_phase(0).expect("player zero exists");
        let card_count = match phase {
            PlayerPhase::Hand => game.hand(0).expect("player zero exists").len(),
            PlayerPhase::FaceUp => game.face_up(0).expect("player zero exists").len(),
            PlayerPhase::FaceDown => game.face_down_count(0).expect("player zero exists"),
        };
        if card_count > 0 {
            cursor = cursor.min(card_count - 1);
        } else {
            cursor = 0;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
            KeyCode::Up | KeyCode::Left => {
                if card_count > 0 {
                    cursor = cursor.saturating_sub(1);
                }
            }
            KeyCode::Down | KeyCode::Right => {
                if card_count > 0 {
                    cursor = (cursor + 1).min(card_count - 1);
                }
            }
            KeyCode::Char(' ') if phase != PlayerPhase::FaceDown && card_count > 0 => {
                toggle_selection(&mut selected, cursor);
                message = if selected.is_empty() {
                    "Kortmarkering borttagen.".to_owned()
                } else {
                    format!("Markerade kort: {}. Enter spelar dem.", selected.len())
                };
            }
            KeyCode::Char('m') if phase != PlayerPhase::FaceDown && card_count > 0 => {
                toggle_selection(&mut selected, cursor);
                message = if selected.is_empty() {
                    "Kortmarkering borttagen.".to_owned()
                } else {
                    format!("Markerade kort: {}. Enter spelar dem.", selected.len())
                };
            }
            KeyCode::Char('d') if phase == PlayerPhase::Hand && game.draw_pile_size() > 0 => {
                match game.draw_and_play() {
                    Ok(outcome) => message = format_outcome("Du ", &outcome),
                    Err(MoveError::EmptyDrawPile) => message = "Draghögen är tom.".to_owned(),
                    Err(error) => return Err(error.into()),
                }
                selected.clear();
                cursor = 0;
            }
            KeyCode::Char('p') if phase != PlayerPhase::FaceDown && !game.pile().is_empty() => {
                match game.pick_up_pile() {
                    Ok(outcome) => message = format_outcome("Du ", &outcome),
                    Err(error) => return Err(error.into()),
                }
                selected.clear();
                cursor = 0;
            }
            KeyCode::Enter if card_count > 0 => {
                let cards = match phase {
                    PlayerPhase::Hand => game.hand(0).expect("player zero exists"),
                    PlayerPhase::FaceUp => game.face_up(0).expect("player zero exists"),
                    PlayerPhase::FaceDown => &[],
                };
                let indices = selected_indices(&selected, cursor, phase, cards);
                let outcome = match phase {
                    PlayerPhase::Hand => game.play_hand_indices(&indices),
                    PlayerPhase::FaceUp => game.play_face_up_indices(&indices),
                    PlayerPhase::FaceDown => game.play_face_down_at(cursor),
                };
                match outcome {
                    Ok(outcome) => {
                        message = format_outcome("Du ", &outcome);
                        selected.clear();
                        cursor = 0;
                    }
                    Err(MoveError::IllegalPlay) => {
                        message = "Det kortet eller den markerade kombinationen kan inte spelas."
                            .to_owned();
                    }
                    Err(error) => message = format!("Draget kunde inte utföras: {error}"),
                }
            }
            _ => {}
        }
    }
}

fn wait_for_computer_turn() -> AppResult<bool> {
    if !event::poll(Duration::from_millis(550))? {
        return Ok(false);
    }
    if let Event::Key(key) = event::read()?
        && matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
        && key.kind != KeyEventKind::Release
    {
        return Ok(true);
    }
    Ok(false)
}

fn wait_for_quit() -> AppResult<bool> {
    if let Event::Key(key) = event::read()?
        && matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
        && key.kind != KeyEventKind::Release
    {
        return Ok(true);
    }
    Ok(false)
}

fn draw(frame: &mut Frame, game: &Game, cursor: usize, selected: &[usize], message: &str) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(7),
            Constraint::Length(8),
            Constraint::Length(5),
        ])
        .split(frame.area());

    let header = Line::from(vec![
        Span::styled(" VÄNDTIA ", Style::default().fg(Color::Yellow).bold()),
        Span::raw(format!(
            "  Draghög: {} kort   |   Spelhög: {} kort   |   ",
            game.draw_pile_size(),
            game.pile().len()
        )),
        Span::styled(
            current_player_label(game),
            Style::default().fg(Color::Cyan).bold(),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(header).block(Block::default().borders(Borders::ALL)),
        areas[0],
    );

    let board = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(48), Constraint::Percentage(52)])
        .split(areas[1]);
    draw_players(frame, game, board[0]);
    draw_pile(frame, game, board[1]);
    draw_cards(frame, game, cursor, selected, areas[2]);

    let help = match game.player_phase(0) {
        Some(PlayerPhase::Hand) => {
            "←/→ välj  Enter spela valören  Space/m markera valfritt  d dra  p plocka upp  q avsluta"
        }
        Some(PlayerPhase::FaceUp) => {
            "←/→ välj  Enter spela valören  Space/m markera valfritt  p plocka upp  q avsluta"
        }
        Some(PlayerPhase::FaceDown) => "←/→ välj  Enter vänd kort  q avsluta",
        None => "q avsluta",
    };
    let footer = Text::from(vec![
        Line::from(Span::styled(message, Style::default().fg(Color::Yellow))),
        Line::from(Span::styled(help, Style::default().fg(Color::DarkGray))),
    ]);
    frame.render_widget(
        Paragraph::new(footer)
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: true }),
        areas[3],
    );
}

fn draw_players(frame: &mut Frame, game: &Game, area: Rect) {
    let mut lines = Vec::new();
    for player in 0..game.player_count() {
        let current = player == game.current_player();
        let label = if player == 0 {
            "Sinä".to_owned()
        } else {
            format!("Dator {}", player + 1)
        };
        let header_style = if current {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        lines.push(Line::from(vec![
            Span::styled(
                format!("{}{}", if current { "▶ " } else { "  " }, label),
                header_style,
            ),
            Span::raw(format!(
                "   {} på hand   {} blinda",
                game.hand_count(player).unwrap_or(0),
                game.face_down_count(player).unwrap_or(0)
            )),
        ]));
        let open = game.face_up(player).expect("player exists");
        let open_text = if open.is_empty() {
            "Inga öppna kort".to_owned()
        } else {
            open.iter()
                .map(|card| format_card(*card))
                .collect::<Vec<_>>()
                .join("   ")
        };
        lines.push(Line::from(Span::styled(
            format!("     Öppna: {open_text}"),
            Style::default().fg(Color::Cyan),
        )));
        lines.push(Line::raw(""));
    }
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(Block::default().title(" Spelare ").borders(Borders::ALL))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn draw_pile(frame: &mut Frame, game: &Game, area: Rect) {
    let lines = match game.pile().last() {
        Some(card) => vec![
            Line::from("Översta kortet"),
            Line::raw(""),
            card_line(*card),
            Line::raw(""),
            Line::from(format!("{} kort i högen", game.pile().len())),
            Line::from(format!("{} kort kvar att dra", game.draw_pile_size())),
        ],
        None => vec![
            Line::from("Högen är tom"),
            Line::raw(""),
            Line::from(format!("{} kort kvar att dra", game.draw_pile_size())),
        ],
    };
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(Block::default().title(" Spelhög ").borders(Borders::ALL))
            .alignment(ratatui::layout::Alignment::Center),
        area,
    );
}

fn draw_cards(frame: &mut Frame, game: &Game, cursor: usize, selected: &[usize], area: Rect) {
    let phase = game.player_phase(0);
    let (title, cards, legal): (&str, Vec<Card>, Vec<usize>) = match phase {
        Some(PlayerPhase::Hand) => (
            " Din hand ",
            game.hand(0).unwrap_or(&[]).to_vec(),
            game.legal_hand_indices(),
        ),
        Some(PlayerPhase::FaceUp) => (
            " Dina öppna kort ",
            game.face_up(0).unwrap_or(&[]).to_vec(),
            game.legal_face_up_indices(),
        ),
        Some(PlayerPhase::FaceDown) => (" Dina blinda kort ", Vec::new(), Vec::new()),
        None => (" Kort ", Vec::new(), Vec::new()),
    };

    let items: Vec<ListItem<'static>> = if phase == Some(PlayerPhase::FaceDown) {
        let count = game.face_down_count(0).unwrap_or(0);
        (0..count)
            .map(|index| ListItem::new(format!("  [{}] Vänd ett dolt kort", index + 1)))
            .collect()
    } else {
        cards
            .iter()
            .enumerate()
            .map(|(index, card)| {
                let marked = selected.contains(&index);
                let is_legal = legal.contains(&index);
                let prefix = if marked { "✓ " } else { "  " };
                let suffix = if is_legal { "" } else { "  · kan inte spelas" };
                let style = if !is_legal {
                    Style::default().fg(Color::DarkGray)
                } else if marked {
                    Style::default().fg(Color::Green).bold()
                } else {
                    card_style(*card)
                };
                ListItem::new(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(format!("[{}] {}", index + 1, format_card(*card)), style),
                    Span::styled(suffix, Style::default().fg(Color::DarkGray)),
                ]))
            })
            .collect()
    };
    let mut state = ListState::default();
    if !items.is_empty() {
        state.select(Some(cursor.min(items.len() - 1)));
    }
    frame.render_stateful_widget(
        List::new(items)
            .block(Block::default().title(title).borders(Borders::ALL))
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(48, 52, 65))
                    .add_modifier(Modifier::BOLD),
            ),
        area,
        &mut state,
    );
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

fn selected_indices(
    selected: &[usize],
    cursor: usize,
    phase: PlayerPhase,
    cards: &[Card],
) -> Vec<usize> {
    if selected.is_empty() {
        if phase == PlayerPhase::FaceDown {
            return vec![cursor];
        }
        let rank = cards[cursor].rank;
        cards
            .iter()
            .enumerate()
            .filter_map(|(index, card)| (card.rank == rank).then_some(index))
            .collect()
    } else {
        selected.to_vec()
    }
}

fn current_player_label(game: &Game) -> String {
    if game.current_player() == 0 {
        "Din tur".to_owned()
    } else {
        format!("Spelare {} tur", game.current_player() + 1)
    }
}

fn format_outcome(actor: &str, outcome: &TurnOutcome) -> String {
    if let Some(card) = outcome.drawn {
        if outcome.played.is_some() {
            return format!("{actor}drog och spelade {}.", format_card(card));
        }
        return format!(
            "{actor}drog {}, men kunde inte spela det och tog upp högen ({} kort).",
            format_card(card),
            outcome.picked_up
        );
    }
    if !outcome.played_cards.is_empty() {
        let cards = outcome
            .played_cards
            .iter()
            .map(|card| format_card(*card))
            .collect::<Vec<_>>()
            .join(" och ");
        let mut message = format!("{actor}spelade {cards}.");
        if outcome.burned {
            message.push_str(" Högen vänds bort — extra tur!");
        } else if outcome.extra_turn {
            message.push_str(" Extra tur!");
        }
        return message;
    }
    if let Some(card) = outcome.played {
        let mut message = format!("{actor}vände {}.", format_card(card));
        if outcome.picked_up > 0 {
            message.push_str(&format!(" Tog upp högen ({} kort).", outcome.picked_up));
        } else if outcome.burned {
            message.push_str(" Högen vänds bort — extra tur!");
        } else if outcome.extra_turn {
            message.push_str(" Extra tur!");
        }
        return message;
    }
    if outcome.picked_up > 0 {
        format!("{actor}tog upp högen ({} kort).", outcome.picked_up)
    } else {
        format!("{actor}spelade.")
    }
}

fn card_line(card: Card) -> Line<'static> {
    Line::from(Span::styled(
        format!("  {}  ", format_card(card)),
        card_style(card).add_modifier(Modifier::BOLD),
    ))
}

fn card_style(card: Card) -> Style {
    let color = match card.suit {
        Suit::Diamonds | Suit::Hearts => Color::LightRed,
        Suit::Clubs | Suit::Spades => Color::White,
    };
    Style::default().fg(color)
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
        Rank::Jack => "J",
        Rank::Queen => "Q",
        Rank::King => "K",
        Rank::Ace => "A",
    };
    let suit = match card.suit {
        Suit::Clubs => "♣",
        Suit::Diamonds => "♦",
        Suit::Hearts => "♥",
        Suit::Spades => "♠",
    };
    format!("{rank}{suit}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_toggles_and_sorts_indices() {
        let mut selected = Vec::new();
        toggle_selection(&mut selected, 2);
        toggle_selection(&mut selected, 0);
        assert_eq!(selected, vec![0, 2]);
        toggle_selection(&mut selected, 2);
        assert_eq!(selected, vec![0]);
    }

    #[test]
    fn empty_selection_uses_the_cursor() {
        let cards = [
            Card {
                rank: Rank::Eight,
                suit: Suit::Clubs,
            },
            Card {
                rank: Rank::Three,
                suit: Suit::Hearts,
            },
            Card {
                rank: Rank::Eight,
                suit: Suit::Spades,
            },
        ];
        assert_eq!(
            selected_indices(&[], 0, PlayerPhase::Hand, &cards),
            vec![0, 2]
        );
        assert_eq!(
            selected_indices(&[0, 1], 2, PlayerPhase::Hand, &cards),
            vec![0, 1]
        );
        assert_eq!(
            selected_indices(&[], 2, PlayerPhase::FaceDown, &[]),
            vec![2]
        );
    }
}
