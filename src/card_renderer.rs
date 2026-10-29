//! Reusable Macroquad rendering for standard playing cards.

use crate::{Card, Rank, Suit};
use macroquad::prelude::*;

pub const CARD_WIDTH: f32 = 100.0;
pub const CARD_HEIGHT: f32 = 142.0;

const INK: Color = Color::new(0.08, 0.1, 0.13, 1.0);

/// Visual emphasis applied when drawing a face-up card.
#[derive(Clone, Copy, Default)]
pub struct CardStyle {
    pub selected: bool,
    pub focused: bool,
}

/// Draws a face-up card inside `rect`.
pub fn draw_face(rect: Rect, card: Card, style: CardStyle) {
    let scale = (rect.h / CARD_HEIGHT).min(rect.w / CARD_WIDTH);
    let suit = suit_color(card.suit);
    let paper = if style.selected {
        Color::new(1.0, 0.95, 0.72, 1.0)
    } else {
        Color::new(0.98, 0.96, 0.87, 1.0)
    };
    let edge = if style.selected {
        GREEN
    } else if style.focused {
        GOLD
    } else {
        Color::new(0.12, 0.16, 0.2, 1.0)
    };

    draw_rectangle(rect.x, rect.y, rect.w, rect.h, edge);
    draw_rectangle(
        rect.x + 3.0 * scale,
        rect.y + 3.0 * scale,
        rect.w - 6.0 * scale,
        rect.h - 6.0 * scale,
        paper,
    );
    draw_rectangle(
        rect.x + 7.0 * scale,
        rect.y + 7.0 * scale,
        rect.w - 14.0 * scale,
        4.0 * scale,
        suit,
    );
    draw_rectangle(
        rect.x + 7.0 * scale,
        rect.y + rect.h - 11.0 * scale,
        rect.w - 14.0 * scale,
        4.0 * scale,
        suit,
    );
    draw_pixel_text(
        rank_label(card.rank),
        rect.x + 10.0 * scale,
        rect.y + 17.0 * scale,
        3.0 * scale,
        INK,
    );
    draw_pixel_pattern(
        rect.x + rect.w - 21.0 * scale,
        rect.y + 10.0 * scale,
        2.0 * scale,
        suit_pattern(card.suit),
        suit,
    );

    if has_face_portrait(card.rank) {
        draw_card_mascot(
            rect.x + rect.w / 2.0 - 17.5 * scale,
            rect.y + 48.0 * scale,
            card.rank,
            scale,
        );
    } else if card.rank == Rank::Ace {
        draw_card_mascot(
            rect.x + rect.w / 2.0 - 17.5 * scale,
            rect.y + 55.0 * scale,
            card.rank,
            scale,
        );
    } else {
        let label = rank_label(card.rank);
        let columns = label.chars().count() * 3 + label.len().saturating_sub(1);
        let rank_pixel =
            ((rect.w - 20.0 * scale) / columns as f32).clamp(3.0 * scale, 10.0 * scale);
        let rank_width = pixel_text_width(label, rank_pixel);
        draw_pixel_text(
            label,
            rect.x + (rect.w - rank_width) / 2.0,
            rect.y + 53.0 * scale,
            rank_pixel,
            INK,
        );
    }

    draw_pixel_pattern(
        rect.x + rect.w / 2.0 - 10.5 * scale,
        rect.y + 119.0 * scale,
        1.5 * scale,
        suit_pattern(card.suit),
        suit,
    );
}

/// Draws a face-down card inside `rect`.
pub fn draw_back(rect: Rect, highlighted: bool) {
    let scale = (rect.h / CARD_HEIGHT).min(rect.w / CARD_WIDTH);
    let navy = Color::new(0.07, 0.17, 0.39, 1.0);
    let blue = Color::new(0.16, 0.38, 0.68, 1.0);
    let gold = Color::new(0.95, 0.73, 0.27, 1.0);

    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        if highlighted { GOLD } else { navy },
    );
    draw_rectangle(
        rect.x + 3.0 * scale,
        rect.y + 3.0 * scale,
        rect.w - 6.0 * scale,
        rect.h - 6.0 * scale,
        blue,
    );
    draw_rectangle_lines(
        rect.x + 7.0 * scale,
        rect.y + 7.0 * scale,
        rect.w - 14.0 * scale,
        rect.h - 14.0 * scale,
        2.0 * scale,
        gold,
    );

    for row in 0..5 {
        for column in 0..3 {
            let is_diamond = (row + column) % 2 == 0;
            draw_pixel_pattern(
                rect.x + (14.0 + column as f32 * 25.0) * scale,
                rect.y + (19.0 + row as f32 * 25.0) * scale,
                2.0 * scale,
                suit_pattern(if is_diamond {
                    Suit::Diamonds
                } else {
                    Suit::Hearts
                }),
                if is_diamond {
                    gold
                } else {
                    Color::new(0.86, 0.89, 0.96, 1.0)
                },
            );
        }
    }

    draw_pixel_pattern(
        rect.x + rect.w / 2.0 - 14.0 * scale,
        rect.y + rect.h / 2.0 - 14.0 * scale,
        4.0 * scale,
        suit_pattern(Suit::Spades),
        navy,
    );
}

/// Returns a compact rank-and-suit label such as `A S`.
pub fn format_card(card: Card) -> String {
    format!("{} {}", rank_label(card.rank), suit_letter(card.suit))
}

pub fn rank_label(rank: Rank) -> &'static str {
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

fn draw_card_mascot(x: f32, y: f32, rank: Rank, scale: f32) {
    macro_rules! pattern {
        ($dx:expr, $dy:expr, $pixel:expr, $rows:expr, $color:expr $(,)?) => {
            draw_pixel_pattern(
                x + $dx * scale,
                y + $dy * scale,
                $pixel * scale,
                $rows,
                $color,
            )
        };
    }
    macro_rules! rect {
        ($dx:expr, $dy:expr, $width:expr, $height:expr, $color:expr) => {
            draw_rectangle(
                x + $dx * scale,
                y + $dy * scale,
                $width * scale,
                $height * scale,
                $color,
            )
        };
    }
    let ink = Color::new(0.1, 0.13, 0.17, 1.0);
    let skin = Color::new(1.0, 0.69, 0.31, 1.0);

    match rank {
        Rank::Ace => {
            pattern!(
                3.0,
                2.0,
                4.0,
                &[
                    0b0001000, 0b0011100, 0b1111111, 0b0111110, 0b0111110, 0b0011100, 0b0011100,
                ],
                GOLD,
            );
            rect!(14.0, 16.0, 4.0, 4.0, WHITE);
        }
        Rank::Jack => {
            let tunic = Color::new(0.15, 0.44, 0.72, 1.0);
            pattern!(3.0, 0.0, 4.0, &[0b0010000, 0b0111000, 0b1111111], GOLD);
            pattern!(3.0, 11.0, 4.0, &[0b0011100, 0b0111110], ink);
            pattern!(
                3.0,
                19.0,
                4.0,
                &[0b0111110, 0b1111111, 0b1111111, 0b1111111, 0b0111110],
                skin,
            );
            rect!(10.0, 27.0, 4.0, 4.0, ink);
            rect!(22.0, 27.0, 4.0, 4.0, ink);
            pattern!(11.0, 39.0, 4.0, &[0b0001000, 0b0011100], skin);
            pattern!(
                0.0,
                47.0,
                4.0,
                &[0b0010000, 0b0111110, 0b1111111, 0b1111111, 0b1111111],
                tunic,
            );
            rect!(15.0, 47.0, 4.0, 16.0, GOLD);
        }
        Rank::Queen => {
            let hair = Color::new(0.55, 0.16, 0.39, 1.0);
            pattern!(3.0, 0.0, 4.0, &[0b1010101, 0b1111111], GOLD);
            pattern!(
                3.0,
                9.0,
                4.0,
                &[
                    0b0111110, 0b1111111, 0b1111111, 0b1111111, 0b0111110, 0b0111110,
                ],
                hair,
            );
            pattern!(
                7.0,
                17.0,
                4.0,
                &[0b0011100, 0b0111110, 0b0111110, 0b0111110],
                skin,
            );
            rect!(11.0, 29.0, 4.0, 4.0, ink);
            rect!(21.0, 29.0, 4.0, 4.0, ink);
            pattern!(11.0, 38.0, 4.0, &[0b0111110, 0b0011100], PINK);
            pattern!(
                0.0,
                46.0,
                4.0,
                &[0b0010000, 0b0111110, 0b1111111, 0b1111111, 0b1111111],
                hair,
            );
            pattern!(14.0, 47.0, 4.0, &[0b0011100, 0b0011100], GOLD);
        }
        Rank::King => {
            let beard = Color::new(0.42, 0.22, 0.11, 1.0);
            let robe = Color::new(0.43, 0.2, 0.58, 1.0);
            pattern!(3.0, 0.0, 4.0, &[0b1010101, 0b1111111, 0b1111111], GOLD);
            pattern!(
                3.0,
                13.0,
                4.0,
                &[0b0111110, 0b1111111, 0b1111111, 0b1111111, 0b0111110],
                skin,
            );
            rect!(10.0, 25.0, 4.0, 4.0, ink);
            rect!(22.0, 25.0, 4.0, 4.0, ink);
            pattern!(
                7.0,
                32.0,
                4.0,
                &[0b0111110, 0b0111110, 0b0011100, 0b0011100],
                beard,
            );
            pattern!(
                0.0,
                48.0,
                4.0,
                &[0b0010000, 0b0111110, 0b1111111, 0b1111111, 0b1111111],
                robe,
            );
            pattern!(15.0, 49.0, 4.0, &[0b0011100, 0b0011100], GOLD);
        }
        _ => (),
    }
}

fn has_face_portrait(rank: Rank) -> bool {
    matches!(rank, Rank::Jack | Rank::Queen | Rank::King)
}

#[cfg(test)]
mod tests {
    use super::*;

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
