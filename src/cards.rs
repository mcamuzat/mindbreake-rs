//! Card catalogs.
//!
//! - [`EXAMPLES`]: ORIGINAL creatures written in the Mindbug style to exercise
//!   the engine. They are not official cards and can be published.
//! - `cards_official` (optional, git-ignored): the official sets, for
//!   personal use only. When `src/cards_official.rs` exists, `build.rs`
//!   enables `cfg(official_cards)` and it becomes the default pool.

use serde::Serialize;

use crate::state::DECK_SIZE;
use crate::types::{
    pool_of, Ability, Affected, CardDef, CardSet, Count, CreatureAction, CreatureFilter, Effect,
    Keyword::*, Modification, PlayerRef, Quantity, Side, StaticAbility, Trigger,
};

#[cfg(official_cards)]
#[path = "cards_official.rs"]
pub mod official;

const fn with_keywords(def: CardDef, keywords: &'static [crate::types::Keyword]) -> CardDef {
    CardDef { keywords, ..def }
}

pub static EXAMPLES: CardSet = &[
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

/// The pool decks are dealt from: the official base set when available,
/// otherwise the example cards.
pub fn default_pool() -> Vec<&'static CardDef> {
    #[cfg(official_cards)]
    return pool_of(official::FIRST_CONTACT);
    #[cfg(not(official_cards))]
    return pool_of(EXAMPLES);
}

pub fn example_pool() -> Vec<&'static CardDef> {
    pool_of(EXAMPLES)
}

/// Every catalog that can be used to build a pool, by name. "Evolutions" and
/// "Variants" hold cards that are not in any default pool.
pub fn sets() -> Vec<(&'static str, CardSet)> {
    #[cfg_attr(not(official_cards), allow(unused_mut))]
    let mut sets: Vec<(&'static str, CardSet)> = vec![("Examples", EXAMPLES)];
    #[cfg(official_cards)]
    sets.extend([
        ("First Contact", official::FIRST_CONTACT),
        ("New Servants", official::NEW_SERVANTS),
        ("Beyond Evolution", official::BEYOND_EVOLUTION),
        ("Promo 2022", official::PROMO_2022),
        ("Promo 2023", official::PROMO_2023),
        ("Promo 2024", official::PROMO_2024),
        ("Variants", official::VARIANTS),
        ("Evolutions", official::EVOLUTIONS),
    ]);
    sets
}

/// The cards of the named sets (see [`sets`]), with their copies. `None` if a
/// name is unknown.
pub fn pool_of_sets(names: &[&str]) -> Option<Vec<&'static CardDef>> {
    let catalog = sets();
    let mut pool = Vec::new();
    for name in names {
        let (_, set) = catalog.iter().find(|(n, _)| n.eq_ignore_ascii_case(name))?;
        pool.extend(pool_of(set));
    }
    Some(pool)
}

/// Looks a card up by name in every available catalog.
pub fn by_name(name: &str) -> &'static CardDef {
    sets()
        .into_iter()
        .flat_map(|(_, set)| set.iter())
        .map(|(_, def)| def)
        .find(|def| def.name == name)
        .unwrap_or_else(|| panic!("unknown card: {name}"))
}

/// Fewest cards a pool can hold: each player is dealt 10.
pub const MIN_POOL: usize = 2 * DECK_SIZE;

#[derive(Debug, Clone, Serialize)]
pub struct SetInfo {
    pub name: &'static str,
    /// Cards it adds to the pool, counting copies.
    pub copies: usize,
}

/// The sets a player can add to a game (not the variants or evolution stages).
pub fn selectable_sets() -> Vec<SetInfo> {
    sets()
        .into_iter()
        .filter(|(name, _)| {
            let internal = ["Variants", "Evolutions"].contains(name);
            // The examples are only offered when there are no official cards.
            let examples_only_alone = *name == "Examples" && cfg!(official_cards);
            !internal && !examples_only_alone
        })
        .map(|(name, set)| SetInfo {
            name,
            copies: set.iter().map(|(n, _)| *n as usize).sum(),
        })
        .collect()
}

/// The pool for a game: the named sets (the default pool if none), checked
/// to be big enough to deal from.
pub fn pool_for(names: &[String]) -> Result<Vec<&'static CardDef>, String> {
    if names.is_empty() {
        return Ok(default_pool());
    }
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let pool = pool_of_sets(&refs).ok_or_else(|| format!("unknown set in {names:?}"))?;
    if pool.len() < MIN_POOL {
        return Err(format!(
            "not enough cards: {} selected, at least {MIN_POOL} needed",
            pool.len()
        ));
    }
    Ok(pool)
}
