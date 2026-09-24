//! Players: random, and flat Monte Carlo with determinization of the hidden information.

use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

use crate::engine::{apply, legal_actions};
use crate::state::{GameAction, GameState};
use crate::types::PlayerId;

pub trait Agent {
    fn choose(&mut self, state: &GameState) -> GameAction;
}

pub struct RandomAgent {
    rng: SmallRng,
}

impl RandomAgent {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: SmallRng::seed_from_u64(seed),
        }
    }
}

impl Agent for RandomAgent {
    fn choose(&mut self, state: &GameState) -> GameAction {
        *legal_actions(state)
            .choose(&mut self.rng)
            .expect("choose called with no legal action")
    }
}

/// For each legal action, plays `playouts` random games to the end from a
/// determinized version of the state (unknown cards reshuffled), then keeps
/// the action with the best win rate.
pub struct MonteCarloAgent {
    rng: SmallRng,
    pub playouts: usize,
}

impl MonteCarloAgent {
    pub fn new(seed: u64, playouts: usize) -> Self {
        Self {
            rng: SmallRng::seed_from_u64(seed),
            playouts,
        }
    }

    fn playout(&mut self, mut state: GameState, me: PlayerId) -> bool {
        // Safety bound: a game always ends, but this guards against engine bugs.
        for _ in 0..1_000 {
            if let Some(winner) = state.winner() {
                return winner == me;
            }
            let action = *legal_actions(&state).choose(&mut self.rng).unwrap();
            apply(&mut state, action).unwrap();
        }
        false
    }
}

impl Agent for MonteCarloAgent {
    fn choose(&mut self, state: &GameState) -> GameAction {
        let me = state.waiting.player().expect("game is over");
        let actions = legal_actions(state);
        if actions.len() == 1 {
            return actions[0];
        }
        let mut best = (actions[0], -1.0);
        for &action in &actions {
            let mut wins = 0;
            for _ in 0..self.playouts {
                let mut sim = determinize(state, me, &mut self.rng);
                apply(&mut sim, action).unwrap();
                if self.playout(sim, me) {
                    wins += 1;
                }
            }
            let rate = wins as f64 / self.playouts as f64;
            if rate > best.1 {
                best = (action, rate);
            }
        }
        best.0
    }
}

/// Copy of the state as seen by `viewer`: their draw pile is reshuffled, and
/// the opponent's hand and draw pile are redealt at random (same sizes).
pub fn determinize(state: &GameState, viewer: PlayerId, rng: &mut SmallRng) -> GameState {
    let mut sim = state.clone();
    // Future random effects must not be known in advance either.
    sim.rng = SmallRng::from_rng(&mut *rng).expect("SmallRng seeding cannot fail");
    sim.players[viewer].deck.shuffle(rng);

    let opponent = &mut sim.players[1 - viewer];
    let hand_len = opponent.hand.len();
    let mut hidden: Vec<_> = opponent
        .hand
        .drain(..)
        .chain(opponent.deck.drain(..))
        .collect();
    hidden.shuffle(rng);
    opponent.deck = hidden.split_off(hand_len);
    opponent.hand = hidden;
    sim
}
