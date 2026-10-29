//! Shared playing-card types and standard deck construction.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rank {
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Card {
    pub rank: Rank,
    pub suit: Suit,
}

/// Creates the 52 cards of a standard deck, ordered by suit and then rank.
pub fn standard_deck() -> Vec<Card> {
    let ranks = [
        Rank::Two,
        Rank::Three,
        Rank::Four,
        Rank::Five,
        Rank::Six,
        Rank::Seven,
        Rank::Eight,
        Rank::Nine,
        Rank::Ten,
        Rank::Jack,
        Rank::Queen,
        Rank::King,
        Rank::Ace,
    ];
    let suits = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades];

    suits
        .into_iter()
        .flat_map(|suit| ranks.map(|rank| Card { rank, suit }))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn standard_deck_has_one_card_for_each_rank_and_suit() {
        let deck = standard_deck();
        let unique_cards = deck.iter().copied().collect::<HashSet<_>>();

        assert_eq!(deck.len(), 52);
        assert_eq!(unique_cards.len(), 52);
    }
}
