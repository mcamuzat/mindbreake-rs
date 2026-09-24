//! Game state, the actions players can take, and what the engine is waiting for.

use std::collections::VecDeque;

use rand::rngs::SmallRng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};

use crate::types::{CardDef, CardId, Effect, PlayerId, TurnMod};

pub const STARTING_LIFE: i32 = 3;
pub const STARTING_MINDBUGS: u8 = 2;
pub const HAND_SIZE: usize = 5;
pub const DECK_SIZE: usize = 10;

#[derive(Debug, Clone)]
pub struct CardInstance {
    pub id: CardId,
    /// The current definition: differs from `printed` once evolved.
    pub def: &'static CardDef,
    pub printed: &'static CardDef,
    /// A Tough creature has already absorbed a defeat.
    pub exhausted: bool,
    /// Modifications lasting until the end of the turn.
    pub turn_mods: Vec<TurnMod>,
}

#[derive(Debug, Clone, Default)]
pub struct Player {
    pub life: i32,
    pub mindbugs: u8,
    pub hand: Vec<CardId>,
    /// Draw pile; the top card is the last element.
    pub deck: Vec<CardId>,
    pub discard: Vec<CardId>,
    pub board: Vec<CardId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    Hand,
    Deck,
    Discard,
    Board,
    /// Cards left out of the game (not dealt): see `GameState::unused`.
    Unused,
}

/// Why cards are being chosen (lets a UI phrase the prompt).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ChoicePurpose {
    Defeat,
    TakeControl,
    Discard,
    PlayFromDiscard,
    ReturnFromDiscard,
    ReturnToHand,
    /// A creature is marked until the end of the turn.
    Affect,
    /// The chosen creature's controller attacks with it.
    MustAttack,
    CopyPlayEffect,
    /// The opponent chooses the card they give you.
    GiveCard,
}

/// A yes/no question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ConfirmKind {
    /// "You may …": accepting resolves the effect.
    MayDo,
    /// Accepting plays the card, declining puts it into the hand.
    PlayIt,
}

/// What the engine needs, and from whom, before the game can continue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all_fields = "camelCase")]
pub enum WaitingFor {
    /// Main decision of the turn: play a creature or attack.
    Action {
        player: PlayerId,
    },
    /// The opponent may use a Mindbug to take the creature that was just played.
    Mindbug {
        player: PlayerId,
        creature: CardId,
    },
    /// Choosing the blocker. `chooser` is the defender, or the attacker if the
    /// attacking creature has Hunter.
    Block {
        chooser: PlayerId,
        attacker: CardId,
        candidates: Vec<CardId>,
    },
    /// A Frenzy creature survived its first attack: attack again or end the turn.
    FrenzyAttack {
        player: PlayerId,
        attacker: CardId,
    },
    /// An effect is waiting for `player` to choose cards, one at a time.
    ChooseCards {
        player: PlayerId,
        source: CardId,
        purpose: ChoicePurpose,
        candidates: Vec<CardId>,
        remaining: u8,
        /// `Done` is allowed ("up to N").
        optional: bool,
        #[serde(skip)]
        controller: PlayerId,
        #[serde(skip)]
        effect: Effect,
    },
    /// A yes/no decision (`Accept` / `Decline`).
    Confirm {
        player: PlayerId,
        source: CardId,
        kind: ConfirmKind,
        #[serde(skip)]
        controller: PlayerId,
        #[serde(skip)]
        effect: Effect,
    },
    GameOver {
        winner: PlayerId,
    },
}

impl WaitingFor {
    /// The player who must act, or `None` if the game is over.
    pub fn player(&self) -> Option<PlayerId> {
        match *self {
            WaitingFor::Action { player }
            | WaitingFor::Mindbug { player, .. }
            | WaitingFor::FrenzyAttack { player, .. }
            | WaitingFor::Confirm { player, .. }
            | WaitingFor::ChooseCards { player, .. } => Some(player),
            WaitingFor::Block { chooser, .. } => Some(chooser),
            WaitingFor::GameOver { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "card")]
pub enum GameAction {
    Play(CardId),
    Attack(CardId),
    /// Use an Action ability instead of playing or attacking.
    Activate(CardId),
    UseMindbug,
    PassMindbug,
    Block(CardId),
    /// As the defender: don't block. As a Hunter attacker: let the defender choose.
    NoBlock,
    Choose(CardId),
    /// Stop choosing ("up to N" choices).
    Done,
    FrenzyAttack,
    EndTurn,
    /// Answer "yes" to a `Confirm`.
    Accept,
    Decline,
}

/// The next step of the turn once all pending effects have resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Step {
    DeclareBlock {
        attacker: CardId,
    },
    AfterAttack {
        attacker: CardId,
    },
    /// Turn over: end-of-turn triggers go on the stack.
    EndTurn,
    /// End-of-turn triggers resolved: the next turn begins.
    NextTurn,
}

/// When a pending effect runs, relative to the effect that came before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Gate {
    Always,
    /// Only if the previous effect did something.
    IfDone,
    /// Once for each time the previous effect did something.
    PerDone,
}

/// A creature set aside by an effect of `source`, to return to `controller`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetAside {
    pub source: CardId,
    pub card: CardId,
    pub controller: PlayerId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PendingEffect {
    pub source: CardId,
    pub controller: PlayerId,
    pub effect: Effect,
    pub gate: Gate,
}

#[derive(Debug, Clone)]
pub struct GameState {
    pub players: [Player; 2],
    pub cards: Vec<CardInstance>,
    pub active: PlayerId,
    pub waiting: WaitingFor,
    pub log: Vec<String>,
    pub(crate) pending: VecDeque<PendingEffect>,
    pub(crate) next_step: Option<Step>,
    /// The active player plays again after this turn (their creature was Mindbugged).
    pub(crate) extra_turn: bool,
    /// Attacks made by each creature during the current turn (for Frenzy).
    pub(crate) attacks_this_turn: Vec<(CardId, u8)>,
    /// Creatures that blocked during the current turn.
    pub(crate) blocked_this_turn: Vec<CardId>,
    /// Creatures that defeated an enemy creature in combat during the current turn.
    pub(crate) kills_this_turn: Vec<CardId>,
    /// How many times the last effect did something (for "if you do").
    pub(crate) done: u32,
    /// The card last given to the controller by an effect.
    pub(crate) last_card: Option<CardId>,
    pub(crate) set_aside: Vec<SetAside>,
    /// Cards not dealt, face down; the top card is the last element.
    pub unused: Vec<CardId>,
    /// For random effects ("steal 2 random cards").
    pub(crate) rng: SmallRng,
}

impl GameState {
    /// Empty state, used to build scenarios (tests, puzzles). Call
    /// [`GameState::start`] once the cards are in place.
    pub fn empty(active: PlayerId) -> Self {
        let player = Player {
            life: STARTING_LIFE,
            mindbugs: STARTING_MINDBUGS,
            ..Player::default()
        };
        Self {
            players: [player.clone(), player],
            cards: Vec::new(),
            active,
            waiting: WaitingFor::Action { player: active },
            log: Vec::new(),
            pending: VecDeque::new(),
            next_step: None,
            extra_turn: false,
            attacks_this_turn: Vec::new(),
            blocked_this_turn: Vec::new(),
            kills_this_turn: Vec::new(),
            done: 0,
            last_card: None,
            set_aside: Vec::new(),
            unused: Vec::new(),
            rng: SmallRng::seed_from_u64(0),
        }
    }

    pub fn add_card(&mut self, player: PlayerId, zone: Zone, def: &'static CardDef) -> CardId {
        let id = CardId(self.cards.len() as u32);
        self.cards.push(CardInstance {
            id,
            def,
            printed: def,
            exhausted: false,
            turn_mods: Vec::new(),
        });
        if zone == Zone::Unused {
            self.unused.push(id);
            return id;
        }
        let player = &mut self.players[player];
        match zone {
            Zone::Hand => player.hand.push(id),
            Zone::Deck => player.deck.push(id),
            Zone::Discard => player.discard.push(id),
            Zone::Board => player.board.push(id),
            Zone::Unused => unreachable!(),
        }
        id
    }

    /// Starts the active player's turn (they lose if they cannot act).
    pub fn start(&mut self) {
        self.waiting = WaitingFor::Action {
            player: self.active,
        };
        crate::engine::check_can_act(self);
    }

    pub fn card(&self, id: CardId) -> &CardInstance {
        &self.cards[id.0 as usize]
    }

    pub fn name(&self, id: CardId) -> &'static str {
        self.card(id).def.name
    }

    pub fn controller(&self, id: CardId) -> Option<PlayerId> {
        (0..2).find(|&p| self.players[p].board.contains(&id))
    }

    pub fn on_board(&self, id: CardId) -> bool {
        self.controller(id).is_some()
    }

    pub fn winner(&self) -> Option<PlayerId> {
        match self.waiting {
            WaitingFor::GameOver { winner } => Some(winner),
            _ => None,
        }
    }
}
