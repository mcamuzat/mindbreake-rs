//! Reducer: `apply(state, action)` validates the action, then moves the game
//! forward to the next decision (`state.waiting`).

use std::fmt;

use rand::rngs::{SmallRng, StdRng};
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};

use crate::state::{
    ChoicePurpose, ConfirmKind, GameAction, GameState, Gate, PendingEffect, SetAside, Step,
    WaitingFor, Zone, DECK_SIZE, HAND_SIZE,
};
use crate::types::{CardDef, CardId, CreatureAction, Effect, Keyword, PlayerId, Trigger, TurnMod};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    IllegalAction {
        action: GameAction,
        waiting: WaitingFor,
    },
    NotYourDecision {
        player: PlayerId,
        waiting: WaitingFor,
    },
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineError::IllegalAction { action, waiting } => {
                write!(f, "illegal action {action:?} while waiting for {waiting:?}")
            }
            EngineError::NotYourDecision { player, waiting } => {
                write!(
                    f,
                    "player {player} cannot act while waiting for {waiting:?}"
                )
            }
        }
    }
}

impl std::error::Error for EngineError {}

/// Setup: shuffle the shared pool, deal 10 cards to each player (5 in hand,
/// 5 in the draw pile) and pick the starting player at random.
pub fn new_game(pool: &[&'static CardDef], seed: u64) -> GameState {
    assert!(
        pool.len() >= 2 * DECK_SIZE,
        "the pool must hold at least 20 cards"
    );
    let mut rng = StdRng::seed_from_u64(seed);
    let mut shuffled = pool.to_vec();
    shuffled.shuffle(&mut rng);

    let mut state = GameState::empty(0);
    for (i, def) in shuffled.into_iter().enumerate() {
        if i >= 2 * DECK_SIZE {
            state.add_card(0, Zone::Unused, def);
            continue;
        }
        let player = i / DECK_SIZE;
        let zone = if i % DECK_SIZE < HAND_SIZE {
            Zone::Hand
        } else {
            Zone::Deck
        };
        state.add_card(player, zone, def);
    }
    state.active = rng.gen_range(0..2);
    state.rng = SmallRng::seed_from_u64(rng.gen());
    state.log.push(format!("Player {} starts", state.active));
    state.start();
    state
}

pub fn legal_actions(state: &GameState) -> Vec<GameAction> {
    match &state.waiting {
        WaitingFor::Action { player } => {
            let p = &state.players[*player];
            let attackers: Vec<CardId> = p
                .board
                .iter()
                .copied()
                .filter(|&c| state.can_attack(c))
                .collect();
            // An enemy that demands to be hunted forces a Hunter to attack.
            let hunters: Vec<CardId> = attackers
                .iter()
                .copied()
                .filter(|&c| state.has_keyword(c, Keyword::Hunter))
                .collect();
            if !hunters.is_empty() && state.demands_hunting(*player) {
                return hunters.into_iter().map(GameAction::Attack).collect();
            }
            p.hand
                .iter()
                .filter(|&&c| !state.cannot_play_from_hand(*player, c))
                .map(|&c| GameAction::Play(c))
                .chain(attackers.into_iter().map(GameAction::Attack))
                .chain(
                    p.board
                        .iter()
                        .filter(|&&c| !state.abilities_for(c, Trigger::Action).is_empty())
                        .map(|&c| GameAction::Activate(c)),
                )
                .collect()
        }
        WaitingFor::Mindbug { .. } => vec![GameAction::UseMindbug, GameAction::PassMindbug],
        WaitingFor::Confirm { .. } => vec![GameAction::Accept, GameAction::Decline],
        WaitingFor::Block {
            candidates,
            chooser,
            attacker,
        } => {
            // A Hunter facing a creature that demands to be hunted has no choice.
            let forced = state.controller(*attacker) == Some(*chooser)
                && !forced_blockers(state, *attacker).is_empty();
            candidates
                .iter()
                .map(|&c| GameAction::Block(c))
                .chain((!forced).then_some(GameAction::NoBlock))
                .collect()
        }
        WaitingFor::FrenzyAttack { .. } => vec![GameAction::FrenzyAttack, GameAction::EndTurn],
        WaitingFor::ChooseCards {
            candidates,
            optional,
            ..
        } => candidates
            .iter()
            .map(|&c| GameAction::Choose(c))
            .chain(optional.then_some(GameAction::Done))
            .collect(),
        WaitingFor::GameOver { .. } => Vec::new(),
    }
}

/// Like [`apply`], but first checks that `player` is the one who must decide.
/// This is the entry point for anything outside the engine (UI, network).
pub fn apply_as(
    state: &mut GameState,
    player: PlayerId,
    action: GameAction,
) -> Result<(), EngineError> {
    if state.waiting.player() != Some(player) {
        return Err(EngineError::NotYourDecision {
            player,
            waiting: state.waiting.clone(),
        });
    }
    apply(state, action)
}

pub fn apply(state: &mut GameState, action: GameAction) -> Result<(), EngineError> {
    if !legal_actions(state).contains(&action) {
        return Err(EngineError::IllegalAction {
            action,
            waiting: state.waiting.clone(),
        });
    }

    match (state.waiting.clone(), action) {
        (WaitingFor::Action { player }, GameAction::Play(card)) => {
            play_from_hand(state, player, card)
        }
        (WaitingFor::Action { player }, GameAction::Attack(attacker))
        | (WaitingFor::FrenzyAttack { player, attacker }, GameAction::FrenzyAttack) => {
            begin_attack(state, player, attacker);
            advance(state);
        }
        (WaitingFor::Action { player }, GameAction::Activate(card)) => {
            state
                .log
                .push(format!("Player {player} activates {}", state.name(card)));
            queue_triggers(state, card, Trigger::Action, player);
            state.next_step = Some(Step::EndTurn);
            advance(state);
        }
        (WaitingFor::Mindbug { player, creature }, GameAction::UseMindbug) => {
            // The opponent plays the card instead: it enters their play area and
            // its Play effect is theirs. The active player then takes another turn.
            let tax = state.mindbug_tax(player);
            if tax > 0 {
                lose_life(state, player, tax);
                if check_life(state) {
                    return Ok(());
                }
            }
            state.players[player].mindbugs -= 1;
            let active = state.active;
            move_to_board(state, creature, active, player);
            state.extra_turn = true;
            state
                .log
                .push(format!("Player {player} Mindbugs {}", state.name(creature)));
            queue_play_triggers(state, creature, player);
            state.next_step = Some(Step::EndTurn);
            advance(state);
        }
        (WaitingFor::Mindbug { creature, .. }, GameAction::PassMindbug) => {
            let active = state.active;
            queue_play_triggers(state, creature, active);
            state.next_step = Some(Step::EndTurn);
            advance(state);
        }
        (WaitingFor::Block { attacker, .. }, GameAction::Block(blocker)) => {
            state.blocked_this_turn.push(blocker);
            fight(state, attacker, blocker);
            state.next_step = Some(Step::AfterAttack { attacker });
            advance(state);
        }
        (
            WaitingFor::Block {
                chooser, attacker, ..
            },
            GameAction::NoBlock,
        ) => {
            if Some(chooser) == state.controller(attacker) {
                // The Hunter does not choose: the defender blocks normally.
                offer_normal_block(state, attacker, 1 - chooser);
            } else {
                unblocked(state, attacker, chooser);
                state.next_step = Some(Step::AfterAttack { attacker });
                advance(state);
            }
        }
        (WaitingFor::FrenzyAttack { .. }, GameAction::EndTurn) => {
            state.next_step = Some(Step::EndTurn);
            advance(state);
        }
        (
            WaitingFor::Confirm {
                source,
                controller,
                effect,
                ..
            },
            GameAction::Accept,
        ) => {
            match effect {
                Effect::Optional(inner) => state.pending.push_front(PendingEffect {
                    source,
                    controller,
                    effect: *inner,
                    gate: Gate::Always,
                }),
                Effect::PlayReceived { .. } => {
                    if let Some(card) = state.last_card.take() {
                        play_from_hand_by_effect(state, controller, card);
                    }
                }
                other => unreachable!("{other:?} is not a yes/no question"),
            }
            advance(state);
        }
        (WaitingFor::Confirm { .. }, GameAction::Decline) => {
            state.last_card = None;
            advance(state);
        }
        (
            WaitingFor::ChooseCards {
                player,
                source,
                remaining,
                optional,
                controller,
                effect,
                ..
            },
            GameAction::Choose(card),
        ) => {
            perform_choice(state, source, controller, effect, card);
            let candidates = choice_candidates(state, effect, controller, source);
            if remaining > 1 && !candidates.is_empty() && state.winner().is_none() {
                state.waiting = WaitingFor::ChooseCards {
                    player,
                    source,
                    purpose: choice_purpose(effect),
                    candidates,
                    remaining: remaining - 1,
                    optional,
                    controller,
                    effect,
                };
            } else {
                advance(state);
            }
        }
        (WaitingFor::ChooseCards { .. }, GameAction::Done) => advance(state),
        (waiting, action) => unreachable!("legal action {action:?} not handled in {waiting:?}"),
    }
    Ok(())
}

fn play_from_hand(state: &mut GameState, player: PlayerId, card: CardId) {
    state.players[player].hand.retain(|&c| c != card);
    refill_hand(state, player);
    state.players[player].board.push(card);
    state
        .log
        .push(format!("Player {player} plays {}", state.name(card)));
    let opponent = 1 - player;
    if state.players[opponent].mindbugs > 0 && !state.mindbugs_prevented() {
        state.waiting = WaitingFor::Mindbug {
            player: opponent,
            creature: card,
        };
    } else {
        queue_play_triggers(state, card, player);
        state.next_step = Some(Step::EndTurn);
        advance(state);
    }
}

/// A card put into play from the hand by an effect: it cannot be Mindbugged.
fn play_from_hand_by_effect(state: &mut GameState, player: PlayerId, card: CardId) {
    if !state.players[player].hand.contains(&card) {
        return;
    }
    state.players[player].hand.retain(|&c| c != card);
    refill_hand(state, player);
    state.players[player].board.push(card);
    state
        .log
        .push(format!("Player {player} plays {}", state.name(card)));
    queue_play_triggers(state, card, player);
}

/// Counts the attack, queues Attack abilities and schedules the block step.
fn begin_attack(state: &mut GameState, player: PlayerId, attacker: CardId) {
    match state
        .attacks_this_turn
        .iter_mut()
        .find(|(c, _)| *c == attacker)
    {
        Some((_, n)) => *n += 1,
        None => state.attacks_this_turn.push((attacker, 1)),
    }
    state.log.push(format!(
        "Player {player} attacks with {}",
        state.name(attacker)
    ));
    queue_triggers(state, attacker, Trigger::Attack, player);
    state.next_step = Some(Step::DeclareBlock { attacker });
}

/// The enemy creatures a Hunter has to choose from, when one demands to be hunted.
fn forced_blockers(state: &GameState, attacker: CardId) -> Vec<CardId> {
    state
        .must_be_hunted(attacker)
        .into_iter()
        .filter(|&c| state.may_block(c))
        .collect()
}

/// Resolves pending effects, then the next step, until a decision is needed.
fn advance(state: &mut GameState) {
    loop {
        if check_life(state) {
            return;
        }
        if let Some(pending) = state.pending.pop_front() {
            if resolve_effect(state, pending) == Resolution::AwaitingChoice {
                return;
            }
            continue;
        }
        match state.next_step.take() {
            Some(Step::DeclareBlock { attacker }) => {
                let Some(attacking_player) = state.controller(attacker) else {
                    state.next_step = Some(Step::EndTurn);
                    continue;
                };
                let defender = 1 - attacking_player;
                let enemies: Vec<CardId> = state.players[defender]
                    .board
                    .iter()
                    .copied()
                    .filter(|&c| state.may_block(c))
                    .collect();
                if state.has_keyword(attacker, Keyword::Hunter) && !enemies.is_empty() {
                    // A Hunter may pick any enemy creature, even one that could
                    // not block it normally (official FAQ).
                    let forced = forced_blockers(state, attacker);
                    state.waiting = WaitingFor::Block {
                        chooser: attacking_player,
                        attacker,
                        candidates: if forced.is_empty() { enemies } else { forced },
                    };
                    return;
                }
                // Either waits for the defender, or resolves an unblocked attack
                // and carries on by itself.
                offer_normal_block(state, attacker, defender);
                return;
            }
            Some(Step::AfterAttack { attacker }) => {
                let attacks = state
                    .attacks_this_turn
                    .iter()
                    .find(|(c, _)| *c == attacker)
                    .map_or(0, |(_, n)| *n);
                match state.controller(attacker) {
                    Some(player)
                        if player == state.active
                            && attacks == 1
                            && state.has_keyword(attacker, Keyword::Frenzy) =>
                    {
                        state.waiting = WaitingFor::FrenzyAttack { player, attacker };
                        return;
                    }
                    _ => state.next_step = Some(Step::EndTurn),
                }
            }
            Some(Step::EndTurn) => {
                for player in [state.active, 1 - state.active] {
                    for card in state.players[player].board.clone() {
                        queue_triggers(state, card, Trigger::EndOfTurn, player);
                    }
                }
                state.next_step = Some(Step::NextTurn);
            }
            Some(Step::NextTurn) => {
                end_turn(state);
                return;
            }
            None => return,
        }
    }
}

/// The defender chooses among the creatures able to block; with none, the
/// attack is unblocked and the game moves on.
fn offer_normal_block(state: &mut GameState, attacker: CardId, defender: PlayerId) {
    let candidates: Vec<CardId> = state.players[defender]
        .board
        .iter()
        .copied()
        .filter(|&b| state.can_block(attacker, b))
        .collect();
    if candidates.is_empty() {
        unblocked(state, attacker, defender);
        state.next_step = Some(Step::AfterAttack { attacker });
        advance(state);
    } else {
        state.waiting = WaitingFor::Block {
            chooser: defender,
            attacker,
            candidates,
        };
    }
}

fn unblocked(state: &mut GameState, attacker: CardId, defender: PlayerId) {
    state.log.push(format!(
        "{} is not blocked: player {defender} loses 1 life",
        state.name(attacker)
    ));
    lose_life(state, defender, 1);
}

/// Loses life (unless something prevents it) and wakes the abilities that
/// react from the discard pile.
fn lose_life(state: &mut GameState, player: PlayerId, amount: i32) {
    if amount <= 0 {
        return;
    }
    if state.cannot_lose_life(player) {
        state.log.push(format!("Player {player} cannot lose life"));
        return;
    }
    state.players[player].life -= amount;
    state
        .log
        .push(format!("Player {player} loses {amount} life"));
    for card in state.players[player].discard.clone() {
        let effects: Vec<Effect> = state
            .card(card)
            .def
            .discard_abilities
            .iter()
            .filter(|a| a.trigger == Trigger::LifeLost)
            .map(|a| a.effect)
            .collect();
        for effect in effects {
            state.pending.push_back(PendingEffect {
                source: card,
                controller: player,
                effect,
                gate: Gate::Always,
            });
        }
    }
}

/// Combat: the lower power is defeated, both on a tie. Poisonous always
/// defeats the enemy creature. Both defeats are simultaneous.
fn fight(state: &mut GameState, attacker: CardId, blocker: CardId) {
    let (pa, pb) = (state.power(attacker), state.power(blocker));
    let reversed = state.reverses_combat(attacker) || state.reverses_combat(blocker);
    let (mut attacker_dies, mut blocker_dies) = if reversed {
        (pa >= pb, pb >= pa)
    } else {
        (pb >= pa, pa >= pb)
    };
    attacker_dies |= state.has_keyword(blocker, Keyword::Poisonous);
    blocker_dies |= state.has_keyword(attacker, Keyword::Poisonous);
    state.log.push(format!(
        "{} ({pa}) fights {} ({pb})",
        state.name(attacker),
        state.name(blocker)
    ));
    if attacker_dies && defeat(state, attacker) {
        state.kills_this_turn.push(blocker);
    }
    if blocker_dies && defeat(state, blocker) {
        state.kills_this_turn.push(attacker);
    }
}

/// A defeated creature goes to the discard pile of its current controller,
/// who also gets its Defeated effect (official FAQ). Tough: exhausted instead,
/// once. Returns whether the creature was really defeated.
fn defeat(state: &mut GameState, card: CardId) -> bool {
    let Some(controller) = state.controller(card) else {
        return false;
    };
    if let Some(into) = state.evolves_instead_of_defeat(card) {
        evolve(state, card, into);
        return false;
    }
    if state
        .card(card)
        .turn_mods
        .contains(&TurnMod::CantBeDefeated)
    {
        state
            .log
            .push(format!("{} cannot be defeated", state.name(card)));
        return false;
    }
    if state.has_keyword(card, Keyword::Tough) && !state.card(card).exhausted {
        state.cards[card.0 as usize].exhausted = true;
        state.log.push(format!(
            "{} is exhausted instead of defeated",
            state.name(card)
        ));
        return false;
    }
    state.players[controller].board.retain(|&c| c != card);
    state.players[controller].discard.push(card);
    state.log.push(format!("{} is defeated", state.name(card)));
    queue_triggers(state, card, Trigger::Defeated, controller);
    leave_play(state, card);
    true
}

/// A card that leaves play loses its exhaustion, its temporary effects and
/// its evolutions.
fn leave_play(state: &mut GameState, card: CardId) {
    let instance = &mut state.cards[card.0 as usize];
    instance.exhausted = false;
    instance.turn_mods.clear();
    instance.def = instance.printed;
}

fn evolve(state: &mut GameState, card: CardId, into: &'static CardDef) {
    state
        .log
        .push(format!("{} evolves into {}", state.name(card), into.name));
    state.cards[card.0 as usize].def = into;
}

fn queue_triggers(state: &mut GameState, source: CardId, trigger: Trigger, controller: PlayerId) {
    let effects = state.abilities_for(source, trigger);
    state
        .pending
        .extend(effects.into_iter().map(|effect| PendingEffect {
            source,
            controller,
            effect,
            gate: Gate::Always,
        }));
}

fn queue_play_triggers(state: &mut GameState, card: CardId, controller: PlayerId) {
    let has_play_effect = !state.abilities_for(card, Trigger::Play).is_empty();
    if has_play_effect && state.play_effects_prevented(controller) {
        state.log.push(format!(
            "Player {controller} cannot activate the Play effect of {}",
            state.name(card)
        ));
        return;
    }
    queue_triggers(state, card, Trigger::Play, controller);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Resolution {
    Done,
    AwaitingChoice,
}

fn push_front(
    state: &mut GameState,
    source: CardId,
    controller: PlayerId,
    effect: Effect,
    gate: Gate,
) {
    state.pending.push_front(PendingEffect {
        source,
        controller,
        effect,
        gate,
    });
}

fn resolve_effect(state: &mut GameState, pending: PendingEffect) -> Resolution {
    let PendingEffect {
        source,
        controller,
        effect,
        gate,
    } = pending;
    match gate {
        Gate::Always => {}
        Gate::IfDone if state.done == 0 => return Resolution::Done,
        Gate::IfDone => {}
        Gate::PerDone => {
            for _ in 0..state.done {
                push_front(state, source, controller, effect, Gate::Always);
            }
            return Resolution::Done;
        }
    }
    match effect {
        Effect::GainLife { player, amount } => {
            let p = player.resolve(controller);
            let amount = state.quantity(amount, controller);
            state.players[p].life += amount;
            state.log.push(format!("Player {p} gains {amount} life"));
            state.done += 1;
        }
        Effect::LoseLife { player, amount } => {
            let p = player.resolve(controller);
            let amount = state.quantity(amount, controller);
            lose_life(state, p, amount);
            state.done += 1;
        }
        Effect::SetLife { player, value } => {
            let p = player.resolve(controller);
            let value = state.quantity(value, controller);
            let current = state.players[p].life;
            if value < current {
                lose_life(state, p, current - value);
            } else {
                state.players[p].life = value;
                state.log.push(format!("Player {p}'s life becomes {value}"));
            }
            state.done += 1;
        }
        Effect::ChooseCreatures { count, .. } => {
            return start_choice(state, source, controller, effect, count.max, count.up_to);
        }
        Effect::AllCreatures { action, filter } => {
            for card in state.matching_creatures(filter, source, controller) {
                apply_creature_action(state, action, card, source, controller);
                state.done += 1;
            }
        }
        Effect::Discard { amount, up_to, .. } => {
            let n = state.quantity(amount, controller).clamp(0, u8::MAX as i32) as u8;
            return start_choice(state, source, controller, effect, n, up_to);
        }
        Effect::DiscardZones { player, hand, deck } => {
            let p = player.resolve(controller);
            let mut cards = Vec::new();
            if deck {
                cards.extend(state.players[p].deck.drain(..).rev());
            }
            if hand {
                cards.append(&mut state.players[p].hand);
            }
            state.log.push(format!(
                "Player {p} discards {} card(s) from their {}",
                cards.len(),
                match (hand, deck) {
                    (true, true) => "hand and draw pile",
                    (true, false) => "hand",
                    _ => "draw pile",
                }
            ));
            state.players[p].discard.extend(cards);
            refill_hand(state, p);
            state.done += 1;
        }
        Effect::StealRandomFromHand { amount } => {
            let victim = 1 - controller;
            if state.hand_gain_prevented(controller) {
                return Resolution::Done;
            }
            let hand = &mut state.players[victim].hand;
            hand.shuffle(&mut state.rng);
            let stolen: Vec<CardId> = hand.drain(..hand.len().min(amount as usize)).collect();
            state.log.push(format!(
                "Player {controller} steals {} card(s) from player {victim}'s hand",
                stolen.len()
            ));
            state.done += stolen.len() as u32;
            state.players[controller].hand.extend(stolen);
            refill_hand(state, victim);
        }
        Effect::PlayFromDiscard { count, .. } => {
            return start_choice(state, source, controller, effect, count.max, count.up_to);
        }
        Effect::ReturnFromDiscard { count: None } => {
            if state.hand_gain_prevented(controller) {
                return Resolution::Done;
            }
            let cards = std::mem::take(&mut state.players[controller].discard);
            state.log.push(format!(
                "Player {controller} puts {} card(s) from their discard pile into their hand",
                cards.len()
            ));
            state.done += cards.len() as u32;
            state.players[controller].hand.extend(cards);
        }
        Effect::ReturnFromDiscard { count: Some(count) } => {
            if state.hand_gain_prevented(controller) {
                return Resolution::Done;
            }
            return start_choice(state, source, controller, effect, count.max, count.up_to);
        }
        Effect::If { condition, then } => {
            if state.condition_holds(condition, controller, source) {
                push_front(state, source, controller, *then, Gate::Always);
            }
        }
        Effect::Sequence(effects) => {
            for &e in effects.iter().rev() {
                push_front(state, source, controller, e, Gate::Always);
            }
        }
        Effect::Optional(_) => {
            state.waiting = WaitingFor::Confirm {
                player: controller,
                source,
                kind: ConfirmKind::MayDo,
                controller,
                effect,
            };
            return Resolution::AwaitingChoice;
        }
        Effect::Then { first, then, per } => {
            state.done = 0;
            let gate = if per { Gate::PerDone } else { Gate::IfDone };
            push_front(state, source, controller, *then, gate);
            push_front(state, source, controller, *first, Gate::Always);
        }
        Effect::Roll { threshold, then } => {
            let roll = state.rng.gen_range(1..=6u8);
            state
                .log
                .push(format!("Player {controller} rolls a {roll}"));
            if roll >= threshold {
                push_front(state, source, controller, effect, Gate::Always);
                push_front(state, source, controller, *then, Gate::Always);
            }
        }
        Effect::GiveControl => {
            if state.controller(source) == Some(controller) {
                move_to_board(state, source, controller, 1 - controller);
                state.log.push(format!(
                    "Player {} takes control of {}",
                    1 - controller,
                    state.name(source)
                ));
                state.done += 1;
            }
        }
        Effect::SwapHands => {
            if state.hand_gain_prevented(0) || state.hand_gain_prevented(1) {
                return Resolution::Done;
            }
            let [a, b] = &mut state.players;
            std::mem::swap(&mut a.hand, &mut b.hand);
            state.log.push("The players swap hands".to_string());
            state.done += 1;
        }
        Effect::EndGame { winner } => {
            let winner = winner.resolve(controller);
            state.waiting = WaitingFor::GameOver { winner };
            state.log.push(format!("Player {winner} wins"));
        }
        Effect::TakeFromOpponentHand => {
            if state.hand_gain_prevented(controller) {
                return Resolution::Done;
            }
            return start_choice(state, source, controller, effect, 1, false);
        }
        Effect::PlayReceived { optional } => {
            let Some(card) = state
                .last_card
                .filter(|c| state.players[controller].hand.contains(c))
            else {
                return Resolution::Done;
            };
            if optional {
                state.waiting = WaitingFor::Confirm {
                    player: controller,
                    source: card,
                    kind: ConfirmKind::PlayIt,
                    controller,
                    effect,
                };
                return Resolution::AwaitingChoice;
            }
            state.last_card = None;
            play_from_hand_by_effect(state, controller, card);
        }
        Effect::Evolve { into } => {
            if state.on_board(source) {
                evolve(state, source, into);
                state.done += 1;
            }
        }
        Effect::SetAsideOthers => {
            for player in [0, 1] {
                let others: Vec<CardId> = state.players[player]
                    .board
                    .iter()
                    .copied()
                    .filter(|&c| c != source)
                    .collect();
                for card in others {
                    state.players[player].board.retain(|&c| c != card);
                    state.set_aside.push(SetAside {
                        source,
                        card,
                        controller: player,
                    });
                    state.log.push(format!("{} is set aside", state.name(card)));
                }
            }
        }
        Effect::ReturnSetAside => {
            let (mine, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut state.set_aside)
                .into_iter()
                .partition(|a| a.source == source);
            state.set_aside = rest;
            for a in mine {
                state.players[a.controller].board.push(a.card);
                state
                    .log
                    .push(format!("{} returns to play", state.name(a.card)));
            }
        }
        Effect::PlaySelf => {
            if state.players[controller].discard.contains(&source) {
                state.players[controller].discard.retain(|&c| c != source);
                state.players[controller].board.push(source);
                state.log.push(format!(
                    "Player {controller} plays {} from their discard pile",
                    state.name(source)
                ));
                queue_play_triggers(state, source, controller);
                state.done += 1;
            }
        }
        Effect::UnusedToBottom { count } => {
            for _ in 0..count {
                let Some(card) = state.unused.pop() else {
                    break;
                };
                state.players[controller].deck.insert(0, card);
            }
            state.log.push(format!(
                "Player {controller} puts {count} card(s) from the unused pile under their draw pile"
            ));
        }
    }
    Resolution::Done
}

/// Asks the right player to choose cards for `effect`, if there is anything
/// to choose from.
fn start_choice(
    state: &mut GameState,
    source: CardId,
    controller: PlayerId,
    effect: Effect,
    count: u8,
    optional: bool,
) -> Resolution {
    let candidates = choice_candidates(state, effect, controller, source);
    if candidates.is_empty() || count == 0 {
        return Resolution::Done;
    }
    // The player performing the action makes the choices (official FAQ):
    // the discarding player chooses their own discards.
    let player = match effect {
        Effect::Discard { player, .. } => player.resolve(controller),
        Effect::TakeFromOpponentHand => 1 - controller,
        _ => controller,
    };
    state.waiting = WaitingFor::ChooseCards {
        player,
        source,
        purpose: choice_purpose(effect),
        candidates,
        remaining: count,
        optional,
        controller,
        effect,
    };
    Resolution::AwaitingChoice
}

fn choice_purpose(effect: Effect) -> ChoicePurpose {
    match effect {
        Effect::ChooseCreatures { action, .. } => match action {
            CreatureAction::Defeat => ChoicePurpose::Defeat,
            CreatureAction::TakeControl => ChoicePurpose::TakeControl,
            CreatureAction::ReturnToHand => ChoicePurpose::ReturnToHand,
            CreatureAction::ThisTurn(_) => ChoicePurpose::Affect,
            CreatureAction::MustAttack => ChoicePurpose::MustAttack,
            CreatureAction::CopyPlayEffect => ChoicePurpose::CopyPlayEffect,
        },
        Effect::Discard { .. } => ChoicePurpose::Discard,
        Effect::PlayFromDiscard { .. } => ChoicePurpose::PlayFromDiscard,
        Effect::ReturnFromDiscard { .. } => ChoicePurpose::ReturnFromDiscard,
        Effect::TakeFromOpponentHand => ChoicePurpose::GiveCard,
        other => unreachable!("{other:?} does not involve a choice"),
    }
}

fn choice_candidates(
    state: &GameState,
    effect: Effect,
    controller: PlayerId,
    source: CardId,
) -> Vec<CardId> {
    match effect {
        Effect::ChooseCreatures { filter, action, .. } => {
            let mut cards = state.matching_creatures(filter, source, controller);
            if action == CreatureAction::MustAttack {
                cards.retain(|&c| state.can_attack(c));
            }
            cards
        }
        Effect::Discard { player, .. } => state.players[player.resolve(controller)].hand.clone(),
        Effect::TakeFromOpponentHand => state.players[1 - controller].hand.clone(),
        Effect::PlayFromDiscard { from, filter, .. } => state.players[from.resolve(controller)]
            .discard
            .iter()
            .copied()
            .filter(|&c| state.attributes_match(filter, c))
            .collect(),
        Effect::ReturnFromDiscard { .. } => state.players[controller].discard.clone(),
        _ => Vec::new(),
    }
}

fn perform_choice(
    state: &mut GameState,
    source: CardId,
    controller: PlayerId,
    effect: Effect,
    card: CardId,
) {
    state.done += 1;
    match effect {
        Effect::ChooseCreatures { action, .. } => {
            apply_creature_action(state, action, card, source, controller)
        }
        Effect::Discard { player, .. } => {
            let p = player.resolve(controller);
            state.players[p].hand.retain(|&c| c != card);
            state.players[p].discard.push(card);
            state
                .log
                .push(format!("Player {p} discards {}", state.name(card)));
            refill_hand(state, p);
        }
        Effect::PlayFromDiscard {
            from, play_effects, ..
        } => {
            let owner = from.resolve(controller);
            state.players[owner].discard.retain(|&c| c != card);
            state.players[controller].board.push(card);
            state.log.push(format!(
                "Player {controller} plays {} from player {owner}'s discard pile",
                state.name(card)
            ));
            if play_effects {
                queue_play_triggers(state, card, controller);
            }
        }
        Effect::ReturnFromDiscard { .. } => {
            state.players[controller].discard.retain(|&c| c != card);
            state.players[controller].hand.push(card);
            state.log.push(format!(
                "Player {controller} puts {} into their hand",
                state.name(card)
            ));
        }
        Effect::TakeFromOpponentHand => {
            let victim = 1 - controller;
            state.players[victim].hand.retain(|&c| c != card);
            state.players[controller].hand.push(card);
            state.last_card = Some(card);
            state.log.push(format!(
                "Player {victim} gives a card from their hand to player {controller}"
            ));
            refill_hand(state, victim);
        }
        other => unreachable!("{other:?} does not involve a choice"),
    }
}

fn apply_creature_action(
    state: &mut GameState,
    action: CreatureAction,
    card: CardId,
    source: CardId,
    controller: PlayerId,
) {
    match action {
        CreatureAction::Defeat => {
            defeat(state, card);
        }
        CreatureAction::TakeControl => {
            if let Some(from) = state.controller(card).filter(|&p| p != controller) {
                move_to_board(state, card, from, controller);
                state.log.push(format!(
                    "Player {controller} takes control of {}",
                    state.name(card)
                ));
            }
        }
        CreatureAction::ReturnToHand => {
            if let Some(owner) = state.controller(card) {
                if state.hand_gain_prevented(owner) {
                    return;
                }
                state.players[owner].board.retain(|&c| c != card);
                state.players[owner].hand.push(card);
                leave_play(state, card);
                state.log.push(format!(
                    "{} returns to player {owner}'s hand",
                    state.name(card)
                ));
            }
        }
        CreatureAction::ThisTurn(modification) => {
            state.cards[card.0 as usize].turn_mods.push(modification);
        }
        CreatureAction::MustAttack => {
            if let Some(owner) = state.controller(card).filter(|_| state.can_attack(card)) {
                begin_attack(state, owner, card);
            }
        }
        CreatureAction::CopyPlayEffect => {
            state.log.push(format!(
                "{} copies the Play effect of {}",
                state.name(source),
                state.name(card)
            ));
            if !state.play_effects_prevented(controller) {
                let effects: Vec<Effect> = state
                    .card(card)
                    .def
                    .abilities_for(Trigger::Play)
                    .copied()
                    .collect();
                for effect in effects.into_iter().rev() {
                    push_front(state, source, controller, effect, Gate::Always);
                }
            }
        }
    }
}

fn move_to_board(state: &mut GameState, card: CardId, from: PlayerId, to: PlayerId) {
    state.players[from].board.retain(|&c| c != card);
    state.players[to].board.push(card);
}

/// "Whenever you remove a card from your hand, you immediately draw cards
/// until you have 5 cards in hand" (rulebook). An empty draw pile stops it.
fn refill_hand(state: &mut GameState, player: PlayerId) {
    let p = &mut state.players[player];
    while p.hand.len() < HAND_SIZE {
        let Some(card) = p.deck.pop() else { break };
        p.hand.push(card);
    }
}

/// Ends the game if a player has no life left. Returns `true` if it is over.
fn check_life(state: &mut GameState) -> bool {
    if state.winner().is_some() {
        return true;
    }
    // Simultaneous 0: not covered by the rulebook. House rule: the active
    // player wins.
    let loser = [1 - state.active, state.active]
        .into_iter()
        .find(|&p| state.players[p].life <= 0);
    if let Some(loser) = loser {
        state.pending.clear();
        state.next_step = None;
        state.waiting = WaitingFor::GameOver { winner: 1 - loser };
        state.log.push(format!("Player {} wins", 1 - loser));
        return true;
    }
    false
}

fn end_turn(state: &mut GameState) {
    let active = state.active;
    if state.extra_turn {
        state.extra_turn = false;
        state
            .log
            .push(format!("Player {active} takes another turn"));
    } else {
        state.active = 1 - active;
    }
    state.attacks_this_turn.clear();
    state.blocked_this_turn.clear();
    state.kills_this_turn.clear();
    for card in &mut state.cards {
        card.turn_mods.clear();
    }
    state.start();
}

/// "A player always has to perform one of the two possible actions. If you
/// cannot do any of these, you lose the game" (official FAQ).
pub(crate) fn check_can_act(state: &mut GameState) {
    if legal_actions(state).is_empty() {
        let winner = 1 - state.active;
        state.waiting = WaitingFor::GameOver { winner };
        state.log.push(format!(
            "Player {} can neither play nor attack: player {winner} wins",
            state.active
        ));
    }
}
