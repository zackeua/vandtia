//! Automated play and batch simulation, kept separate from the game engine.

use std::fmt;

use crate::{Card, Game, MoveError, PlayerPhase, TurnOutcome};

const MAX_TURNS_PER_GAME: usize = 100_000;

#[derive(Debug, Clone, PartialEq)]
pub struct SimulationStats {
    pub games: usize,
    pub wins: Vec<usize>,
    pub total_turns: usize,
}

impl SimulationStats {
    pub fn average_turns(&self) -> f64 {
        if self.games == 0 {
            0.0
        } else {
            self.total_turns as f64 / self.games as f64
        }
    }
}

#[derive(Debug)]
pub enum SimulationError {
    Engine(MoveError),
    TurnLimitReached,
}

impl fmt::Display for SimulationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Engine(error) => write!(f, "{error}"),
            Self::TurnLimitReached => f.write_str("simulation exceeded the per-game turn limit"),
        }
    }
}

impl std::error::Error for SimulationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Engine(error) => Some(error),
            Self::TurnLimitReached => None,
        }
    }
}

impl From<MoveError> for SimulationError {
    fn from(error: MoveError) -> Self {
        Self::Engine(error)
    }
}

/// Plays one turn with the simulator's basic strategy.
///
/// It plays the lowest legal hand/open card, picks up the pile when no card is
/// legal, and lets the engine choose a blind card at random.
pub fn play_computer_turn(game: &mut Game) -> Result<TurnOutcome, MoveError> {
    let player = game.current_player();
    match game.player_phase(player).expect("current player exists") {
        PlayerPhase::Hand => {
            let hand = game.hand(player).expect("current player exists");
            if let Some(index) = lowest_legal_index(hand, &game.legal_hand_indices()) {
                game.play_hand(index)
            } else {
                game.pick_up_pile()
            }
        }
        PlayerPhase::FaceUp => {
            let face_up = game.face_up(player).expect("current player exists");
            if let Some(index) = lowest_legal_index(face_up, &game.legal_face_up_indices()) {
                game.play_face_up(index)
            } else {
                game.pick_up_pile()
            }
        }
        PlayerPhase::FaceDown => game.play_face_down(),
    }
}

/// Simulates games using the basic computer strategy.
pub fn simulate(
    player_count: usize,
    games: usize,
    seed: u64,
) -> Result<SimulationStats, SimulationError> {
    if !(2..=5).contains(&player_count) {
        return Err(MoveError::InvalidPlayerCount.into());
    }

    let mut stats = SimulationStats {
        games,
        wins: vec![0; player_count],
        total_turns: 0,
    };

    for game_number in 0..games {
        let game_seed = seed.wrapping_add(game_number as u64);
        let mut game = Game::new(player_count, game_seed)?;
        let mut turns = 0;
        while game.winner().is_none() {
            if turns >= MAX_TURNS_PER_GAME {
                return Err(SimulationError::TurnLimitReached);
            }
            play_computer_turn(&mut game)?;
            turns += 1;
        }
        if let Some(winner) = game.winner() {
            stats.wins[winner] += 1;
        }
        stats.total_turns += turns;
    }

    Ok(stats)
}

fn lowest_legal_index(cards: &[Card], legal_indices: &[usize]) -> Option<usize> {
    legal_indices
        .iter()
        .copied()
        .min_by_key(|index| cards[*index].rank)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simulation_records_a_winner_for_each_game() {
        let stats = simulate(3, 10, 42).unwrap();
        assert_eq!(stats.wins.iter().sum::<usize>(), 10);
        assert!(stats.average_turns() > 0.0);
    }
}
