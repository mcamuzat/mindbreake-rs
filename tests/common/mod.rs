//! Test fixtures: the example cards as the rules tests were written against
//! them, frozen here so that rebalancing `cards::EXAMPLES` (checked with
//! `cargo run --release -- balance`) doesn't change what the tests exercise.
#![allow(dead_code)]

use mindbreake::cards::by_name;
use mindbreake::types::{
    Ability, Affected, CardDef, CardSet, Count, CreatureAction, CreatureFilter, Effect, Keyword::*,
    Modification, PlayerRef, Quantity, Side, StaticAbility, Trigger,
};

const fn with_keywords(def: CardDef, keywords: &'static [mindbreake::types::Keyword]) -> CardDef {
    CardDef { keywords, ..def }
}

static FIXTURES: CardSet = &[
    (2, with_keywords(CardDef::new("Ferret Scout", 2), &[Sneaky])),
    (
        2,
        with_keywords(CardDef::new("Thornback Boar", 5), &[Tough]),
    ),
    (
        2,
        with_keywords(CardDef::new("Venom Newt", 1), &[Poisonous]),
    ),
    (2, with_keywords(CardDef::new("Sky Harrier", 4), &[Hunter])),
    (2, with_keywords(CardDef::new("Rabid Otter", 3), &[Frenzy])),
    (2, CardDef::new("Moss Golem", 8)),
    (
        2,
        CardDef {
            abilities: &[Ability {
                trigger: Trigger::Play,
                effect: Effect::LoseLife {
                    player: PlayerRef::Opponent,
                    amount: Quantity::Fixed(1),
                },
            }],
            ..CardDef::new("Ember Salamander", 6)
        },
    ),
    (
        2,
        CardDef {
            abilities: &[Ability {
                trigger: Trigger::Play,
                effect: Effect::GainLife {
                    player: PlayerRef::You,
                    amount: Quantity::Fixed(2),
                },
            }],
            ..CardDef::new("Healing Moth", 2)
        },
    ),
    (
        2,
        CardDef {
            abilities: &[Ability {
                trigger: Trigger::Defeated,
                effect: Effect::LoseLife {
                    player: PlayerRef::Opponent,
                    amount: Quantity::Fixed(1),
                },
            }],
            ..CardDef::new("Grave Crow", 3)
        },
    ),
    (
        2,
        CardDef {
            keywords: &[Hunter],
            abilities: &[Ability {
                trigger: Trigger::Attack,
                effect: Effect::ChooseCreatures {
                    action: CreatureAction::Defeat,
                    filter: CreatureFilter::side(Side::Enemy).max_power(2),
                    count: Count::exactly(1),
                },
            }],
            ..CardDef::new("Tusk Mauler", 7)
        },
    ),
    (
        2,
        CardDef {
            keywords: &[Tough],
            abilities: &[Ability {
                trigger: Trigger::Play,
                effect: Effect::AllCreatures {
                    action: CreatureAction::Defeat,
                    filter: CreatureFilter::side(Side::Any).max_power(3),
                },
            }],
            ..CardDef::new("Quake Tortoise", 9)
        },
    ),
    (
        2,
        CardDef {
            statics: &[StaticAbility::Modify {
                affected: Affected::OtherAllies { max_power: None },
                condition: None,
                modification: Modification::Power(1),
            }],
            ..CardDef::new("Pack Alpha", 4)
        },
    ),
    (
        2,
        with_keywords(CardDef::new("Mirror Imp", 2), &[Sneaky, Frenzy]),
    ),
    (
        2,
        CardDef {
            keywords: &[Poisonous],
            abilities: &[Ability {
                trigger: Trigger::Attack,
                effect: Effect::GainLife {
                    player: PlayerRef::You,
                    amount: Quantity::Fixed(1),
                },
            }],
            ..CardDef::new("Bog Leech", 3)
        },
    ),
    (
        2,
        with_keywords(CardDef::new("Stone Sentinel", 6), &[Tough]),
    ),
    (
        2,
        CardDef {
            abilities: &[Ability {
                trigger: Trigger::Play,
                effect: Effect::LoseLife {
                    player: PlayerRef::You,
                    amount: Quantity::Fixed(1),
                },
            }],
            ..CardDef::new("Twin-Head Hydra", 10)
        },
    ),
    (
        2,
        CardDef {
            abilities: &[Ability {
                trigger: Trigger::Play,
                effect: Effect::ChooseCreatures {
                    action: CreatureAction::Defeat,
                    filter: CreatureFilter::side(Side::Enemy).min_power(6),
                    count: Count::exactly(1),
                },
            }],
            ..CardDef::new("Giant Slayer Wasp", 1)
        },
    ),
];

/// A fixture card by name, or any other catalog card (official sets) if it
/// is not one of the fixtures.
pub fn card(name: &str) -> &'static CardDef {
    FIXTURES
        .iter()
        .find(|(_, def)| def.name == name)
        .map_or_else(|| by_name(name), |(_, def)| def)
}
