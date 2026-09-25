//! Players: random, flat Monte Carlo, and ISMCTS, both with determinization of
//! the hidden information.

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
}

impl Agent for MonteCarloAgent {
    fn choose(&mut self, state: &GameState) -> GameAction {
        let me = state.waiting.player().expect("game is over");
        let actions = legal_actions(state);
        if actions.len() == 1 {
            return actions[0];
        }
        let state = &without_log(state);
        let mut best = (actions[0], -1.0);
        for &action in &actions {
            let mut wins = 0;
            for _ in 0..self.playouts {
                let mut sim = determinize(state, me, &mut self.rng);
                apply(&mut sim, action).unwrap();
                if playout(sim, &mut self.rng) == Some(me) {
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

/// Copy of the state without its log: nobody reads it during a search, and
/// it would be copied again with every determinization.
fn without_log(state: &GameState) -> GameState {
    let mut copy = state.clone();
    copy.log = Vec::new();
    copy
}

/// Plays random moves until the game ends and returns the winner.
fn playout(mut state: GameState, rng: &mut SmallRng) -> Option<PlayerId> {
    // Safety bound: a game always ends, but this guards against engine bugs.
    for _ in 0..1_000 {
        if state.waiting.player().is_none() {
            return state.winner();
        }
        let action = *legal_actions(&state).choose(rng).unwrap();
        apply(&mut state, action).unwrap();
    }
    None
}

/// Single-observer Information Set MCTS (Cowling, Powley & Whitehouse, 2012).
///
/// One tree over the decisions of both players, searched from the point of
/// view of the player to act. Each iteration draws a new determinization, so
/// the tree only follows actions that are legal in it; a child's UCB term uses
/// how often it was *available* rather than how often its parent was visited.
/// Unlike flat Monte Carlo, it plans the following decisions (blocks, the
/// opponent's replies, Mindbugs) instead of playing them at random.
pub struct IsmctsAgent {
    rng: SmallRng,
    pub iterations: usize,
    /// UCB exploration constant, for rewards in [0, 1].
    pub exploration: f64,
}

struct Node {
    /// The action that leads here from the parent (none for the root).
    action: Option<GameAction>,
    /// Who chose `action`: `wins` counts their wins.
    player: PlayerId,
    parent: Option<usize>,
    children: Vec<usize>,
    visits: u32,
    availability: u32,
    wins: f64,
}

impl IsmctsAgent {
    pub fn new(seed: u64, iterations: usize) -> Self {
        Self {
            rng: SmallRng::seed_from_u64(seed),
            iterations,
            exploration: 0.7,
        }
    }

    fn ucb(&self, node: &Node) -> f64 {
        let visits = node.visits as f64;
        node.wins / visits + self.exploration * ((node.availability as f64).ln() / visits).sqrt()
    }

    fn iterate(&mut self, tree: &mut Vec<Node>, root: &GameState, me: PlayerId) {
        let mut sim = determinize(root, me, &mut self.rng);
        let mut current = 0;
        // Selection, then expansion of one new node.
        while let Some(player) = sim.waiting.player() {
            let legal = legal_actions(&sim);
            let available: Vec<usize> = tree[current]
                .children
                .iter()
                .copied()
                .filter(|&c| legal.contains(&tree[c].action.unwrap()))
                .collect();
            for &c in &available {
                tree[c].availability += 1;
            }
            let untried: Vec<GameAction> = legal
                .into_iter()
                .filter(|&a| !available.iter().any(|&c| tree[c].action == Some(a)))
                .collect();
            if let Some(&action) = untried.choose(&mut self.rng) {
                tree.push(Node {
                    action: Some(action),
                    player,
                    parent: Some(current),
                    children: Vec::new(),
                    visits: 0,
                    availability: 1,
                    wins: 0.0,
                });
                let child = tree.len() - 1;
                tree[current].children.push(child);
                apply(&mut sim, action).unwrap();
                current = child;
                break;
            }
            let best = available
                .into_iter()
                .max_by(|&a, &b| self.ucb(&tree[a]).total_cmp(&self.ucb(&tree[b])))
                .expect("a state waiting for a player has legal actions");
            apply(&mut sim, tree[best].action.unwrap()).unwrap();
            current = best;
        }
        let winner = playout(sim, &mut self.rng);
        let mut node = Some(current);
        while let Some(i) = node {
            tree[i].visits += 1;
            if winner == Some(tree[i].player) {
                tree[i].wins += 1.0;
            }
            node = tree[i].parent;
        }
    }
}

impl Agent for IsmctsAgent {
    fn choose(&mut self, state: &GameState) -> GameAction {
        let me = state.waiting.player().expect("game is over");
        let actions = legal_actions(state);
        if actions.len() == 1 {
            return actions[0];
        }
        let state = &without_log(state);
        let mut tree = vec![Node {
            action: None,
            player: me,
            parent: None,
            children: Vec::new(),
            visits: 0,
            availability: 0,
            wins: 0.0,
        }];
        for _ in 0..self.iterations {
            self.iterate(&mut tree, state, me);
        }
        // The most visited action is the most robust choice.
        tree[0]
            .children
            .iter()
            .map(|&c| &tree[c])
            .filter(|n| actions.contains(&n.action.unwrap()))
            .max_by_key(|n| n.visits)
            .and_then(|n| n.action)
            .unwrap_or(actions[0])
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
