# CLAUDE.md

mindbreake.rs: a Mindbug-style rules engine in Rust (`mindbreake` crate), WASM bridge (`mindbreake-wasm/`), React page (`web/`).

## Commands
- `cargo test`: rules tests (`tests/rules.rs`).
- `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all` before every commit.
- `cargo run --release -- sim 200 30`: Monte Carlo vs random (quick check that the AI still works).
- `cargo run --release -- arena 200 1000 150`: ISMCTS (the page's AI) vs Monte Carlo; ~67% at this budget.
- Web: `./scripts/build-wasm.sh` then `cd web && pnpm dev` / `pnpm build` (includes `tsc`).

## Principles
- **The engine owns all logic.** `main.rs`, `mindbreake-wasm` and `web/` only display state and send `GameAction`s. Anything a UI needs (current power, rules text, legal actions, hidden information) is computed in `src/view.rs`.
- Multiplayer is host-authoritative P2P (`web/src/session/`): only the host runs the engine; the guest gets `view_for(state, 1)` and its actions go through `apply_as(state, 1, …)`. Never send the guest anything not derived from its own view. Bump `PROTOCOL_VERSION` (`web/src/session/protocol.ts`) on any wire change.
- Changing a serialized type (`GameAction`, `WaitingFor`, `GameView`…) means updating `web/src/engine/types.ts` in the same change.
- **Rules-correct.** `RULES.md` is the reference. It was written from memory: points marked ❓ must be checked against the official rulebook before being treated as settled. Any rule change updates `RULES.md`, the engine and a test together.
- **Real cards.** `src/cards.rs` holds original example cards (publishable). The official cards live in `src/cards_official.rs` and `data/official-cards.json`, both git-ignored (Nerdlab IP, personal use): never commit them, never publish a WASM/dist build that contains them. Transcribe from `data/official-cards.json`, never from memory, and check with `cargo run -- cards` that the generated text matches the official text.
- **Art.** `web/public/official/` (downloaded by `scripts/fetch-official-art.sh`) is git-ignored IP, like the official cards. The page must keep working without it (text-card fallback in `web/src/art.ts`).
- **Rules.** `RULES.md` cites its source ([R] rulebook, [FAQ]) for each rule; a rule without a source is marked ❓.
- **Parameterize, don't proliferate.** Extend `CreatureFilter`, `PlayerRef` and friends rather than adding one-off `Effect` variants.
- Test scenarios: `GameState::empty(active)` + `add_card(player, zone, def)` + `start()`, then drive them with `apply`. Building-block tests (`tests/building_blocks.rs`) use ad-hoc cards so they don't depend on any catalog.
- Derived values (power, keywords, blocking, conditions) live in `src/query.rs`: never read `def.power` / `def.keywords` directly for an on-board creature.
