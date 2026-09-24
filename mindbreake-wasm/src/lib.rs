//! WASM bridge. Zero game logic: every function deserializes, calls the
//! engine, and serializes the result. The game lives in WASM memory; JS only
//! ever receives a player's view.

use std::cell::RefCell;

use mindbreake::ai::{Agent, MonteCarloAgent};
use mindbreake::cards::{pool_for, selectable_sets, SetInfo, MIN_POOL};
use mindbreake::types::PlayerId;
use mindbreake::{apply_as, new_game as engine_new_game, view_for, GameAction, GameState};
use wasm_bindgen::prelude::*;

thread_local! {
    static GAME: RefCell<Option<GameState>> = const { RefCell::new(None) };
}

#[wasm_bindgen(start)]
fn start() {
    console_error_panic_hook::set_once();
}

fn to_js<T: serde::Serialize>(value: &T) -> Result<JsValue, JsError> {
    // Plain JS objects rather than Maps, so TypeScript types can mirror the JSON.
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    Ok(value.serialize(&serializer)?)
}

fn with_game<R>(f: impl FnOnce(&mut GameState) -> Result<R, JsError>) -> Result<R, JsError> {
    GAME.with_borrow_mut(|game| match game.as_mut() {
        Some(state) => f(state),
        None => Err(JsError::new("no game in progress: call newGame first")),
    })
}

fn player(id: u8) -> Result<PlayerId, JsError> {
    match id {
        0 | 1 => Ok(id as PlayerId),
        _ => Err(JsError::new("player must be 0 or 1")),
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AvailableSets {
    min_cards: usize,
    sets: Vec<SetInfo>,
}

/// The sets a game can be built from, and how many cards it needs.
#[wasm_bindgen(js_name = availableSets)]
pub fn available_sets() -> Result<JsValue, JsError> {
    to_js(&AvailableSets {
        min_cards: MIN_POOL,
        sets: selectable_sets(),
    })
}

/// Starts a game from the named sets (the default pool if none) and returns
/// the view of `viewer`.
#[wasm_bindgen(js_name = newGame)]
pub fn new_game(seed: u32, viewer: u8, sets: Vec<String>) -> Result<JsValue, JsError> {
    let viewer = player(viewer)?;
    let pool = pool_for(&sets).map_err(|e| JsError::new(&e))?;
    let state = engine_new_game(&pool, seed.into());
    let view = to_js(&view_for(&state, viewer));
    GAME.with_borrow_mut(|game| *game = Some(state));
    view
}

#[wasm_bindgen]
pub fn view(viewer: u8) -> Result<JsValue, JsError> {
    let viewer = player(viewer)?;
    with_game(|state| to_js(&view_for(state, viewer)))
}

/// Plays `action` (JSON: `{ type: "Play", card: 3 }`) on behalf of `actor`,
/// and returns the actor's new view.
#[wasm_bindgen(js_name = applyAction)]
pub fn apply_action(actor: u8, action: JsValue) -> Result<JsValue, JsError> {
    let actor = player(actor)?;
    let action: GameAction = serde_wasm_bindgen::from_value(action)?;
    with_game(|state| {
        apply_as(state, actor, action).map_err(|e| JsError::new(&e.to_string()))?;
        to_js(&view_for(state, actor))
    })
}

/// Lets the Monte Carlo AI choose and play for `ai` if the decision is theirs.
/// Returns the chosen action, or `null` if it was not the AI's turn to decide.
#[wasm_bindgen(js_name = aiStep)]
pub fn ai_step(ai: u8, playouts: u32, seed: u32) -> Result<JsValue, JsError> {
    let ai = player(ai)?;
    with_game(|state| {
        if state.waiting.player() != Some(ai) {
            return Ok(JsValue::NULL);
        }
        let action = MonteCarloAgent::new(seed.into(), playouts as usize).choose(state);
        apply_as(state, ai, action).map_err(|e| JsError::new(&e.to_string()))?;
        to_js(&action)
    })
}
