//! Mindbug-style rules engine.
//!
//! - [`types`]: static card definitions (keywords, triggers, effects).
//! - [`state`]: game state, [`state::GameAction`] and [`state::WaitingFor`].
//! - [`engine`]: the reducer ([`engine::apply`]) and [`engine::legal_actions`].
//! - [`query`]: derived values (current power and keywords, blocking).
//! - [`ai`]: agents (random, Monte Carlo, ISMCTS).
//! - [`balance`]: card statistics from AI self-play.
//! - [`view`]: what one player is allowed to see (UI / WASM).
//! - [`cards`]: example catalog.

pub mod ai;
pub mod balance;
pub mod cards;
pub mod engine;
pub mod query;
pub mod state;
pub mod types;
pub mod view;

pub use engine::{apply, apply_as, legal_actions, new_game, EngineError};
pub use state::{GameAction, GameState, WaitingFor, Zone};
pub use view::{view_for, GameView};
