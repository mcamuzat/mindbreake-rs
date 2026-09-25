//! Card balance from self-play: ISMCTS against itself, and for each card,
//! how often the player who ends up controlling it wins.

use std::collections::HashMap;

use crate::ai::{Agent, IsmctsAgent};
use crate::engine::{apply, new_game};
use crate::state::{GameAction, GameState, WaitingFor};
use crate::types::{CardDef, CardId, PlayerId};

/// What happened to one card (by name) over all the games.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CardStats {
    pub name: &'static str,
    pub power: i32,
    /// Times it was played from hand.
    pub played: u32,
    /// Of those, games won by its controller once the Mindbug decision was made.
    pub wins: u32,
    /// Times the opponent could Mindbug it.
    pub offered: u32,
    /// Times they did.
    pub stolen: u32,
}

impl CardStats {
    pub fn win_rate(&self) -> f64 {
        ratio(self.wins, self.played)
    }

    /// Half-width of the 95% confidence interval of [`Self::win_rate`].
    pub fn margin(&self) -> f64 {
        if self.played == 0 {
            return 1.0;
        }
        let p = self.win_rate();
        1.96 * (p * (1.0 - p) / self.played as f64).sqrt()
    }

    pub fn steal_rate(&self) -> f64 {
        ratio(self.stolen, self.offered)
    }

    fn merge(&mut self, other: &CardStats) {
        self.played += other.played;
        self.wins += other.wins;
        self.offered += other.offered;
        self.stolen += other.stolen;
    }
}

fn ratio(n: u32, d: u32) -> f64 {
    if d == 0 {
        0.0
    } else {
        n as f64 / d as f64
    }
}

/// One card played during a game.
struct Play {
    def: &'static CardDef,
    controller: PlayerId,
    offered: bool,
    stolen: bool,
}

/// Plays `games` games (seeds `0..games`) spread over `threads` threads, both
/// seats played by ISMCTS with `iterations` iterations. Deterministic for a
/// given set of arguments, whatever the number of threads.
pub fn self_play(
    pool: &[&'static CardDef],
    games: u64,
    iterations: usize,
    threads: usize,
) -> Vec<CardStats> {
    let threads = threads.max(1) as u64;
    let partials: Vec<HashMap<&'static str, CardStats>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                scope.spawn(move || {
                    let mut stats = HashMap::new();
                    for seed in (t..games).step_by(threads as usize) {
                        record(&mut stats, pool, seed, iterations);
                    }
                    stats
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("a self-play thread panicked"))
            .collect()
    });
    let mut total: HashMap<&'static str, CardStats> = HashMap::new();
    for partial in &partials {
        for (name, stats) in partial {
            total
                .entry(name)
                .or_insert_with(|| CardStats {
                    name: stats.name,
                    power: stats.power,
                    ..CardStats::default()
                })
                .merge(stats);
        }
    }
    let mut stats: Vec<CardStats> = total.into_values().collect();
    stats.sort_by(|a, b| a.name.cmp(b.name));
    stats
}

/// Plays one game and adds what happened to each played card to `stats`.
fn record(
    stats: &mut HashMap<&'static str, CardStats>,
    pool: &[&'static CardDef],
    seed: u64,
    iterations: usize,
) {
    let mut state = new_game(pool, seed);
    let mut agents = [
        IsmctsAgent::new(seed * 2, iterations),
        IsmctsAgent::new(seed * 2 + 1, iterations),
    ];
    let mut plays: Vec<Play> = Vec::new();
    // The last play of each card: a card can be played again after returning to hand.
    let mut last_play: HashMap<CardId, usize> = HashMap::new();

    while let Some(player) = state.waiting.player() {
        let action = agents[player].choose(&state);
        note(&state, player, action, &mut plays, &mut last_play);
        apply(&mut state, action).expect("the agent chose a legal action");
    }

    let winner = state.winner();
    for play in &plays {
        let entry = stats.entry(play.def.name).or_insert_with(|| CardStats {
            name: play.def.name,
            power: play.def.power,
            ..CardStats::default()
        });
        entry.played += 1;
        entry.wins += u32::from(winner == Some(play.controller));
        entry.offered += u32::from(play.offered);
        entry.stolen += u32::from(play.stolen);
    }
}

/// Records a card played from hand, and the Mindbug decision that follows.
fn note(
    state: &GameState,
    player: PlayerId,
    action: GameAction,
    plays: &mut Vec<Play>,
    last_play: &mut HashMap<CardId, usize>,
) {
    match (&state.waiting, action) {
        (WaitingFor::Action { .. }, GameAction::Play(card)) => {
            last_play.insert(card, plays.len());
            plays.push(Play {
                // As printed: an evolved creature is still counted as its card.
                def: state.card(card).printed,
                controller: player,
                offered: false,
                stolen: false,
            });
        }
        (WaitingFor::Mindbug { creature, .. }, _) => {
            if let Some(&i) = last_play.get(creature) {
                plays[i].offered = true;
                if action == GameAction::UseMindbug {
                    plays[i].stolen = true;
                    plays[i].controller = player;
                }
            }
        }
        _ => {}
    }
}
