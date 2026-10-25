//! Game engine for Vändtia.
//!
//! The engine uses these rule assumptions: 2 resets the minimum rank, 10 burns
//! the pile, and four consecutive cards of the same rank burn the pile. A burn
//! lets that player take another turn. Players refill their hands to three
//! cards while the draw pile lasts.

pub mod simulator;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Card {
    pub rank: Rank,
    pub suit: Suit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerPhase {
    Hand,
    FaceUp,
    FaceDown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnOutcome {
    pub played: Option<Card>,
    pub burned: bool,
    pub extra_turn: bool,
    pub picked_up: usize,
    pub winner: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveError {
    InvalidPlayerCount,
    WrongPhase,
    InvalidCardIndex,
    IllegalPlay,
    EmptyPile,
    GameOver,
}

impl std::fmt::Display for MoveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::InvalidPlayerCount => "player count must be between 2 and 5",
            Self::WrongPhase => "that card zone is not active",
            Self::InvalidCardIndex => "card index is out of range",
            Self::IllegalPlay => "card is lower than the current minimum",
            Self::EmptyPile => "there is no pile to pick up",
            Self::GameOver => "the game is already over",
        };
        f.write_str(message)
    }
}

impl std::error::Error for MoveError {}

#[derive(Debug, Clone)]
struct Player {
    hand: Vec<Card>,
    face_up: Vec<Card>,
    face_down: Vec<Card>,
}

impl Player {
    fn phase(&self) -> PlayerPhase {
        if !self.hand.is_empty() {
            PlayerPhase::Hand
        } else if !self.face_up.is_empty() {
            PlayerPhase::FaceUp
        } else {
            PlayerPhase::FaceDown
        }
    }

    fn has_cards(&self) -> bool {
        !self.hand.is_empty() || !self.face_up.is_empty() || !self.face_down.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct Game {
    players: Vec<Player>,
    draw_pile: Vec<Card>,
    pile: Vec<Card>,
    current_player: usize,
    winner: Option<usize>,
    rng: SimpleRng,
}

impl Game {
    /// Starts a shuffled game for 2-5 players. The seed makes games reproducible.
    pub fn new(player_count: usize, seed: u64) -> Result<Self, MoveError> {
        if !(2..=5).contains(&player_count) {
            return Err(MoveError::InvalidPlayerCount);
        }

        let mut rng = SimpleRng::new(seed);
        let mut deck = fresh_deck();
        for i in (1..deck.len()).rev() {
            let j = rng.index(i + 1);
            deck.swap(i, j);
        }

        let mut players = vec![
            Player {
                hand: Vec::with_capacity(3),
                face_up: Vec::with_capacity(3),
                face_down: Vec::with_capacity(3),
            };
            player_count
        ];
        for player in &mut players {
            for _ in 0..3 {
                player
                    .face_down
                    .push(deck.pop().expect("deck has enough cards"));
            }
            for _ in 0..3 {
                player
                    .face_up
                    .push(deck.pop().expect("deck has enough cards"));
            }
            for _ in 0..3 {
                player.hand.push(deck.pop().expect("deck has enough cards"));
            }
        }
        let current_player = rng.index(player_count);

        Ok(Self {
            players,
            draw_pile: deck,
            pile: Vec::new(),
            current_player,
            winner: None,
            rng,
        })
    }

    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    pub fn current_player(&self) -> usize {
        self.current_player
    }

    pub fn winner(&self) -> Option<usize> {
        self.winner
    }

    pub fn pile(&self) -> &[Card] {
        &self.pile
    }

    pub fn draw_pile_size(&self) -> usize {
        self.draw_pile.len()
    }

    pub fn player_phase(&self, player: usize) -> Option<PlayerPhase> {
        self.players.get(player).map(Player::phase)
    }

    pub fn hand(&self, player: usize) -> Option<&[Card]> {
        self.players.get(player).map(|p| p.hand.as_slice())
    }

    pub fn face_up(&self, player: usize) -> Option<&[Card]> {
        self.players.get(player).map(|p| p.face_up.as_slice())
    }

    pub fn face_down_count(&self, player: usize) -> Option<usize> {
        self.players.get(player).map(|p| p.face_down.len())
    }

    /// Returns legal hand-card indices for the player whose turn it is.
    pub fn legal_hand_indices(&self) -> Vec<usize> {
        self.legal_indices_for_phase(PlayerPhase::Hand)
    }

    /// Returns legal face-up card indices for the player whose turn it is.
    pub fn legal_face_up_indices(&self) -> Vec<usize> {
        self.legal_indices_for_phase(PlayerPhase::FaceUp)
    }

    fn legal_indices_for_phase(&self, phase: PlayerPhase) -> Vec<usize> {
        if self.winner.is_some() || self.players[self.current_player].phase() != phase {
            return Vec::new();
        }
        let cards = match phase {
            PlayerPhase::Hand => &self.players[self.current_player].hand,
            PlayerPhase::FaceUp => &self.players[self.current_player].face_up,
            PlayerPhase::FaceDown => return Vec::new(),
        };
        cards
            .iter()
            .enumerate()
            .filter_map(|(i, card)| self.is_legal(*card).then_some(i))
            .collect()
    }

    /// Plays a card from the current player's hand.
    pub fn play_hand(&mut self, index: usize) -> Result<TurnOutcome, MoveError> {
        self.ensure_in_progress()?;
        if self.players[self.current_player].phase() != PlayerPhase::Hand {
            return Err(MoveError::WrongPhase);
        }
        let card = *self.players[self.current_player]
            .hand
            .get(index)
            .ok_or(MoveError::InvalidCardIndex)?;
        if !self.is_legal(card) {
            return Err(MoveError::IllegalPlay);
        }
        self.players[self.current_player].hand.remove(index);
        Ok(self.finish_play(card))
    }

    /// Plays a selected open card after the player's hand is empty.
    pub fn play_face_up(&mut self, index: usize) -> Result<TurnOutcome, MoveError> {
        self.ensure_in_progress()?;
        if self.players[self.current_player].phase() != PlayerPhase::FaceUp {
            return Err(MoveError::WrongPhase);
        }
        let card = *self.players[self.current_player]
            .face_up
            .get(index)
            .ok_or(MoveError::InvalidCardIndex)?;
        if !self.is_legal(card) {
            return Err(MoveError::IllegalPlay);
        }
        self.players[self.current_player].face_up.remove(index);
        Ok(self.finish_play(card))
    }

    /// Reveals and plays one blind card after hand and open cards are gone.
    /// If it is illegal, it and the pile are picked up.
    pub fn play_face_down(&mut self) -> Result<TurnOutcome, MoveError> {
        self.ensure_in_progress()?;
        if self.players[self.current_player].phase() != PlayerPhase::FaceDown {
            return Err(MoveError::WrongPhase);
        }
        let card_index = self
            .rng
            .index(self.players[self.current_player].face_down.len());
        self.play_face_down_at(card_index)
    }

    /// Reveals and plays the selected blind card after hand and open cards are gone.
    pub fn play_face_down_at(&mut self, index: usize) -> Result<TurnOutcome, MoveError> {
        self.ensure_in_progress()?;
        if self.players[self.current_player].phase() != PlayerPhase::FaceDown {
            return Err(MoveError::WrongPhase);
        }
        let card = self.players[self.current_player]
            .face_down
            .get(index)
            .copied()
            .ok_or(MoveError::InvalidCardIndex)?;
        self.players[self.current_player].face_down.remove(index);
        if !self.is_legal(card) {
            self.pile.push(card);
            let picked_up = self.pick_up_current_pile();
            self.advance_turn();
            return Ok(TurnOutcome {
                played: Some(card),
                burned: false,
                extra_turn: false,
                picked_up,
                winner: None,
            });
        }
        Ok(self.finish_play(card))
    }

    /// Picks up the pile and ends the current player's turn.
    pub fn pick_up_pile(&mut self) -> Result<TurnOutcome, MoveError> {
        self.ensure_in_progress()?;
        if self.pile.is_empty() {
            return Err(MoveError::EmptyPile);
        }
        let picked_up = self.pick_up_current_pile();
        self.advance_turn();
        Ok(TurnOutcome {
            played: None,
            burned: false,
            extra_turn: false,
            picked_up,
            winner: None,
        })
    }

    fn ensure_in_progress(&self) -> Result<(), MoveError> {
        if self.winner.is_some() {
            Err(MoveError::GameOver)
        } else {
            Ok(())
        }
    }

    fn is_legal(&self, card: Card) -> bool {
        if matches!(card.rank, Rank::Two | Rank::Ten) || self.pile.is_empty() {
            return true;
        }
        match self.pile.last().map(|top| top.rank) {
            Some(Rank::Two) => true,
            Some(top_rank) => card.rank >= top_rank,
            None => true,
        }
    }

    fn finish_play(&mut self, card: Card) -> TurnOutcome {
        let player = self.current_player;
        self.pile.push(card);

        let four_of_a_kind = self.pile.len() >= 4
            && self.pile[self.pile.len() - 4..]
                .iter()
                .all(|pile_card| pile_card.rank == card.rank);
        let burned = card.rank == Rank::Ten || four_of_a_kind;
        let extra_turn = burned || card.rank == Rank::Two;
        if burned {
            self.pile.clear();
        }

        if !self.draw_pile.is_empty() {
            while self.players[player].hand.len() < 3 && !self.draw_pile.is_empty() {
                self.players[player]
                    .hand
                    .push(self.draw_pile.pop().expect("draw pile was checked"));
            }
        }

        if !self.players[player].has_cards() {
            self.winner = Some(player);
        } else if !extra_turn {
            self.advance_turn();
        }

        TurnOutcome {
            played: Some(card),
            burned,
            extra_turn,
            picked_up: 0,
            winner: self.winner,
        }
    }

    fn pick_up_current_pile(&mut self) -> usize {
        let picked_up = self.pile.len();
        self.players[self.current_player]
            .hand
            .append(&mut self.pile);
        picked_up
    }

    fn advance_turn(&mut self) {
        self.current_player = (self.current_player + 1) % self.players.len();
    }
}

fn fresh_deck() -> Vec<Card> {
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

#[derive(Debug, Clone)]
struct SimpleRng(u64);

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self(if seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed
        })
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn index(&mut self, upper: usize) -> usize {
        (self.next() as usize) % upper
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(rank: Rank, suit: Suit) -> Card {
        Card { rank, suit }
    }

    fn game_with_pile(pile: Vec<Card>) -> Game {
        let mut game = Game::new(2, 1).unwrap();
        game.pile = pile;
        game.current_player = 0;
        game
    }

    #[test]
    fn two_resets_minimum_and_ace_is_highest() {
        let mut game = game_with_pile(vec![card(Rank::Ace, Suit::Clubs)]);
        assert!(game.is_legal(card(Rank::Two, Suit::Hearts)));
        game.pile.push(card(Rank::Two, Suit::Hearts));
        assert!(game.is_legal(card(Rank::King, Suit::Diamonds)));
        game.pile.push(card(Rank::Eight, Suit::Spades));
        assert!(!game.is_legal(card(Rank::Seven, Suit::Diamonds)));
        assert!(game.is_legal(card(Rank::Ace, Suit::Spades)));
        assert!(Rank::Ace > Rank::King);
    }

    #[test]
    fn ten_burns_the_pile() {
        let mut game = game_with_pile(vec![card(Rank::King, Suit::Clubs)]);
        game.current_player = 0;
        game.players[0].hand[0] = card(Rank::Ten, Suit::Hearts);
        let outcome = game.play_hand(0).unwrap();
        assert!(outcome.burned);
        assert!(outcome.extra_turn);
        assert!(game.pile().is_empty());
        assert_eq!(game.current_player(), 0);
    }

    #[test]
    fn playing_two_keeps_the_turn_and_next_card_is_played_by_same_player() {
        let mut game = game_with_pile(vec![card(Rank::Five, Suit::Clubs)]);
        game.players[0].hand[0] = card(Rank::Two, Suit::Hearts);
        game.players[0].hand[1] = card(Rank::Three, Suit::Diamonds);

        let two_outcome = game.play_hand(0).unwrap();
        assert!(!two_outcome.burned);
        assert!(two_outcome.extra_turn);
        assert_eq!(game.current_player(), 0);
        assert!(game.legal_hand_indices().contains(&0));

        let next_outcome = game.play_hand(0).unwrap();
        assert_eq!(next_outcome.played, Some(card(Rank::Three, Suit::Diamonds)));
        assert!(!next_outcome.extra_turn);
        assert_eq!(game.current_player(), 1);
    }

    #[test]
    fn four_consecutive_cards_of_same_rank_burn() {
        let pile = vec![
            card(Rank::Eight, Suit::Clubs),
            card(Rank::Eight, Suit::Diamonds),
            card(Rank::Eight, Suit::Hearts),
        ];
        let mut game = game_with_pile(pile);
        game.players[0].hand[0] = card(Rank::Eight, Suit::Spades);
        let outcome = game.play_hand(0).unwrap();
        assert!(outcome.burned);
        assert!(game.pile().is_empty());
    }

    #[test]
    fn illegal_hand_play_does_not_change_state() {
        let mut game = game_with_pile(vec![card(Rank::Queen, Suit::Clubs)]);
        game.players[0].hand[0] = card(Rank::Jack, Suit::Diamonds);
        let previous_pile = game.pile().to_vec();
        assert_eq!(game.play_hand(0), Err(MoveError::IllegalPlay));
        assert_eq!(game.pile(), previous_pile);
        assert_eq!(game.current_player(), 0);
    }

    #[test]
    fn selected_blind_card_is_revealed_and_played() {
        let mut game = game_with_pile(vec![card(Rank::Five, Suit::Clubs)]);
        game.players[0].hand.clear();
        game.players[0].face_up.clear();
        game.players[0].face_down = vec![
            card(Rank::Six, Suit::Hearts),
            card(Rank::Seven, Suit::Spades),
        ];

        let outcome = game.play_face_down_at(1).unwrap();

        assert_eq!(outcome.played, Some(card(Rank::Seven, Suit::Spades)));
        assert!(!outcome.burned);
        assert!(!outcome.extra_turn);
        assert_eq!(game.face_down_count(0), Some(1));
    }
}
