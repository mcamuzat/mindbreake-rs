//! Card definitions: static, typed data. No game state lives here.

use serde::{Deserialize, Serialize};

pub type PlayerId = usize;

/// Identifies one physical card for the whole game (it keeps its id across zones).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CardId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Keyword {
    /// If it survives its first attack during a turn, it can attack a second time.
    Frenzy,
    /// When it attacks, its controller may choose the enemy creature that has to block it.
    Hunter,
    /// In combat, always defeats the enemy creature, whatever its power.
    Poisonous,
    /// Can only be blocked by Sneaky creatures.
    Sneaky,
    /// If it would be defeated and isn't exhausted, it is exhausted instead.
    Tough,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Trigger {
    Play,
    Attack,
    Defeated,
    /// Used instead of attacking or playing a card (one per turn action).
    Action,
    /// At the end of every turn (see `Condition` for "your turn").
    EndOfTurn,
    /// The controller lost 1 or more life points. Only meaningful for the
    /// `discard_abilities` of a card in its owner's discard pile.
    LifeLost,
}

/// A player, relative to the controller of the ability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PlayerRef {
    You,
    Opponent,
}

impl PlayerRef {
    pub fn resolve(self, controller: PlayerId) -> PlayerId {
        match self {
            PlayerRef::You => controller,
            PlayerRef::Opponent => 1 - controller,
        }
    }
}

/// Whose creatures a filter matches, relative to the controller of the ability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Side {
    Ally,
    Enemy,
    Any,
    /// The source of the ability, only.
    This,
}

/// "The creature(s) with the highest / lowest power" among the creatures of the filter's side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Rank {
    Highest,
    Lowest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum KeywordFilter {
    /// "With 1 or more keywords".
    Any,
    Has(Keyword),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatureFilter {
    pub side: Side,
    pub max_power: Option<i32>,
    pub min_power: Option<i32>,
    pub exclude_source: bool,
    pub rank: Option<Rank>,
    pub keyword: Option<KeywordFilter>,
    /// Only creatures with an ability of this kind (printed, not granted).
    pub trigger: Option<Trigger>,
    /// Only creatures that blocked during the current turn.
    pub blocked_this_turn: bool,
}

impl CreatureFilter {
    pub const fn side(side: Side) -> Self {
        Self {
            side,
            max_power: None,
            min_power: None,
            exclude_source: true,
            rank: None,
            keyword: None,
            trigger: None,
            blocked_this_turn: false,
        }
    }

    /// Only the source itself ("this creature").
    pub const fn this() -> Self {
        Self::side(Side::This).including_source()
    }

    pub const fn max_power(mut self, power: i32) -> Self {
        self.max_power = Some(power);
        self
    }

    pub const fn min_power(mut self, power: i32) -> Self {
        self.min_power = Some(power);
        self
    }

    pub const fn ranked(mut self, rank: Rank) -> Self {
        self.rank = Some(rank);
        self
    }

    pub const fn with_keyword(mut self, keyword: Keyword) -> Self {
        self.keyword = Some(KeywordFilter::Has(keyword));
        self
    }

    pub const fn with_any_keyword(mut self) -> Self {
        self.keyword = Some(KeywordFilter::Any);
        self
    }

    pub const fn with_trigger(mut self, trigger: Trigger) -> Self {
        self.trigger = Some(trigger);
        self
    }

    pub const fn blocked_this_turn(mut self) -> Self {
        self.blocked_this_turn = true;
        self
    }

    /// The source itself can be chosen ("defeat a creature").
    pub const fn including_source(mut self) -> Self {
        self.exclude_source = false;
        self
    }
}

/// A number read from the game state, relative to the controller of the ability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum Quantity {
    Fixed(i32),
    Life(PlayerRef),
    CreaturesControlled(PlayerRef),
    Mindbugs(PlayerRef),
    Hand(PlayerRef),
    /// Your creatures minus the opponent's.
    CreatureLead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Comparator {
    Less,
    LessOrEqual,
    Equal,
    GreaterOrEqual,
    Greater,
}

impl Comparator {
    pub fn eval(self, left: i32, right: i32) -> bool {
        match self {
            Comparator::Less => left < right,
            Comparator::LessOrEqual => left <= right,
            Comparator::Equal => left == right,
            Comparator::GreaterOrEqual => left >= right,
            Comparator::Greater => left > right,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "type")]
pub enum Condition {
    /// It is the turn of the ability's controller.
    YourTurn,
    /// The source defeated an enemy creature (in combat) during this turn.
    SourceDefeatedEnemyThisTurn,
    Compare {
        left: Quantity,
        comparator: Comparator,
        right: Quantity,
    },
}

/// What happens to each chosen / matching creature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CreatureAction {
    Defeat,
    /// The ability's controller puts it into their play area.
    TakeControl,
    /// It goes back to its controller's hand.
    ReturnToHand,
    /// A modification lasting until the end of the turn.
    ThisTurn(TurnMod),
    /// Its controller attacks with it if able.
    MustAttack,
    /// The ability's controller resolves its Play effects.
    CopyPlayEffect,
}

/// Lasts until the end of the turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum TurnMod {
    Power(i32),
    Keyword(Keyword),
    CantBlock,
    CantBeDefeated,
}

/// How many objects a choice picks: exactly `max` (as many as possible), or up to `max`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Count {
    pub max: u8,
    pub up_to: bool,
}

impl Count {
    pub const fn exactly(max: u8) -> Self {
        Self { max, up_to: false }
    }

    pub const fn up_to(max: u8) -> Self {
        Self { max, up_to: true }
    }

    /// "Any number of".
    pub const fn any() -> Self {
        Self::up_to(u8::MAX)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "type")]
pub enum Effect {
    GainLife {
        player: PlayerRef,
        amount: Quantity,
    },
    LoseLife {
        player: PlayerRef,
        amount: Quantity,
    },
    SetLife {
        player: PlayerRef,
        value: Quantity,
    },
    /// The ability's controller chooses creatures matching `filter`.
    ChooseCreatures {
        action: CreatureAction,
        filter: CreatureFilter,
        count: Count,
    },
    /// Every creature matching `filter`.
    AllCreatures {
        action: CreatureAction,
        filter: CreatureFilter,
    },
    /// `player` chooses and discards cards from their hand ("up to" if `up_to`).
    Discard {
        player: PlayerRef,
        amount: Quantity,
        up_to: bool,
    },
    /// `player` discards their whole hand and/or draw pile.
    DiscardZones {
        player: PlayerRef,
        hand: bool,
        deck: bool,
    },
    /// Random cards from the opponent's hand go to your hand.
    StealRandomFromHand {
        amount: u8,
    },
    /// You choose cards in `from`'s discard pile matching `filter` (power,
    /// keywords, abilities) and put them into play under your control. They
    /// are played (their Play effects trigger unless `play_effects` is false)
    /// but, not being played from hand, they cannot be Mindbugged.
    PlayFromDiscard {
        from: PlayerRef,
        filter: CreatureFilter,
        count: Count,
        play_effects: bool,
    },
    /// Cards from your discard pile go to your hand (`None`: all of them).
    ReturnFromDiscard {
        count: Option<Count>,
    },
    If {
        condition: Condition,
        then: &'static Effect,
    },
    /// The effects, one after the other.
    Sequence(&'static [Effect]),
    /// "You may …": the controller is asked first.
    Optional(&'static Effect),
    /// Resolves `first`, then `then` if `first` did something ("if you do"), or,
    /// with `per`, once for each time it did ("for each … this way").
    Then {
        first: &'static Effect,
        then: &'static Effect,
        per: bool,
    },
    /// Rolls a 6-sided die. On `threshold` or more, resolves `then` and rolls again.
    Roll {
        threshold: u8,
        then: &'static Effect,
    },
    /// The opponent takes control of the source.
    GiveControl,
    SwapHands,
    /// `winner` wins the game.
    EndGame {
        winner: PlayerRef,
    },
    /// The opponent chooses a card in their hand, and you receive it.
    TakeFromOpponentHand,
    /// Plays the card just received (if it is still in your hand), by choice if `optional`.
    PlayReceived {
        optional: bool,
    },
    /// The source replaces itself with the next card of its evolution line.
    Evolve {
        into: &'static CardDef,
    },
    /// Every other creature is set aside until the source's return effect.
    SetAsideOthers,
    /// The creatures set aside by the source return to play, without Play effects.
    ReturnSetAside,
    /// The source, in its controller's discard pile, is played.
    PlaySelf,
    /// The top cards of the unused pile go to the bottom of your draw pile.
    UnusedToBottom {
        count: u8,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Ability {
    pub trigger: Trigger,
    pub effect: Effect,
}

/// Which creatures a static ability applies to, relative to its source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "type")]
pub enum Affected {
    This,
    /// The controller's other creatures, optionally only up to a given power.
    OtherAllies {
        max_power: Option<i32>,
    },
    /// The controller's opponent's creatures.
    Enemies,
    /// Every creature of the controller, the source included.
    AllAllies,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum Modification {
    Power(i32),
    Keywords(&'static [Keyword]),
    /// Has each of these keywords while an enemy creature has it.
    CopyEnemyKeywords(&'static [Keyword]),
    /// `amount` power for each other creature its controller has.
    PowerPerOtherAlly(i32),
    /// The creature has this ability as if it were printed on it.
    Ability(&'static Ability),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Restriction {
    CantAttack,
    CantBlock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "type")]
pub enum StaticAbility {
    Modify {
        affected: Affected,
        condition: Option<Condition>,
        modification: Modification,
    },
    /// Attacking creatures in `attackers` cannot be blocked by creatures
    /// matching `blockers` (only its power and keyword conditions count).
    /// A Hunter ignores this.
    CantBeBlockedBy {
        attackers: Affected,
        blockers: CreatureFilter,
    },
    /// Creatures matching `filter` (sides are relative to the source's
    /// controller) cannot attack / block at all.
    Restrict {
        restriction: Restriction,
        filter: CreatureFilter,
    },
    /// `player` cannot activate Play effects.
    PreventPlayEffects {
        player: PlayerRef,
    },
    /// Nobody can use a Mindbug.
    PreventMindbugs,
    /// The controller's opponent first loses `life` when using a Mindbug.
    MindbugTax {
        life: i32,
    },
    CantLoseLife {
        player: PlayerRef,
    },
    /// When this fights, the highest power is defeated instead of the lowest.
    ReverseCombat,
    /// The opponent must attack this with a Hunter creature, if able.
    MustBeHunted,
    /// `player` cannot play from their hand the cards matching `filter`.
    CantPlayFromHand {
        player: PlayerRef,
        filter: CreatureFilter,
    },
    /// `player` cannot put cards into their hand (by effects).
    PreventHandGain {
        player: PlayerRef,
    },
    /// If this would be defeated, it evolves into `into` instead.
    EvolveInsteadOfDefeat {
        into: &'static CardDef,
    },
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct CardDef {
    pub name: &'static str,
    pub power: i32,
    pub keywords: &'static [Keyword],
    pub abilities: &'static [Ability],
    pub statics: &'static [StaticAbility],
    /// Abilities that work while the card is in its owner's discard pile.
    pub discard_abilities: &'static [Ability],
}

impl CardDef {
    pub const fn new(name: &'static str, power: i32) -> Self {
        Self {
            name,
            power,
            keywords: &[],
            abilities: &[],
            statics: &[],
            discard_abilities: &[],
        }
    }

    pub fn abilities_for(&self, trigger: Trigger) -> impl Iterator<Item = &Effect> {
        self.abilities
            .iter()
            .filter(move |a| a.trigger == trigger)
            .map(|a| &a.effect)
    }
}

/// A set's composition: each card with its number of copies.
pub type CardSet = &'static [(u8, CardDef)];

pub fn pool_of(set: CardSet) -> Vec<&'static CardDef> {
    set.iter()
        .flat_map(|(copies, def)| std::iter::repeat_n(def, *copies as usize))
        .collect()
}
