//! Building blocks added for the expansions: Action abilities, evolution,
//! "you may … if you do", turn-long effects, restrictions, end-of-turn
//! triggers, hidden-zone effects… Ad-hoc cards only, independent of any catalog.

mod common;

use mindbreake::state::{ChoicePurpose, ConfirmKind};
use mindbreake::types::*;
use mindbreake::{apply, legal_actions, GameAction, GameState, WaitingFor, Zone};

const ENEMY: CreatureFilter = CreatureFilter::side(Side::Enemy);
const ANY: CreatureFilter = CreatureFilter::side(Side::Any).including_source();
const ANY_CARD: CreatureFilter = CreatureFilter::side(Side::Any);
const ONE: Count = Count::exactly(1);

const fn ability(trigger: Trigger, effect: Effect) -> Ability {
    Ability { trigger, effect }
}

const fn card_with(name: &'static str, power: i32, abilities: &'static [Ability]) -> CardDef {
    CardDef {
        abilities,
        ..CardDef::new(name, power)
    }
}

const fn card_with_static(
    name: &'static str,
    power: i32,
    statics: &'static [StaticAbility],
) -> CardDef {
    CardDef {
        statics,
        ..CardDef::new(name, power)
    }
}

const LOSE_ONE: Effect = Effect::LoseLife {
    player: PlayerRef::Opponent,
    amount: Quantity::Fixed(1),
};
const DEFEAT_ENEMY: Effect = Effect::ChooseCreatures {
    action: CreatureAction::Defeat,
    filter: ENEMY,
    count: ONE,
};

fn card(name: &str) -> &'static CardDef {
    common::card(name)
}

fn act(s: &mut GameState, action: GameAction) {
    apply(s, action).unwrap_or_else(|e| panic!("{e}"));
}

/// Player 0 active; each player holds a filler card so nobody is stuck. No Mindbugs.
fn scenario() -> GameState {
    let mut s = GameState::empty(0);
    s.add_card(0, Zone::Hand, card("Moss Golem"));
    s.add_card(1, Zone::Hand, card("Moss Golem"));
    s.players = s
        .players
        .map(|p| mindbreake::state::Player { mindbugs: 0, ..p });
    s
}

// ---- Action, evolution ----------------------------------------------------

static ELDER: CardDef = card_with("Elder", 9, &[]);
static LARVA: CardDef = card_with(
    "Larva",
    1,
    &[ability(
        Trigger::Action,
        Effect::Sequence(&[
            Effect::GainLife {
                player: PlayerRef::You,
                amount: Quantity::Fixed(1),
            },
            Effect::Evolve { into: &ELDER },
        ]),
    )],
);

#[test]
fn an_action_replaces_the_turns_action_and_can_evolve_the_source() {
    let mut s = scenario();
    let larva = s.add_card(0, Zone::Board, &LARVA);
    s.start();
    assert!(legal_actions(&s).contains(&GameAction::Activate(larva)));
    act(&mut s, GameAction::Activate(larva));
    assert_eq!(s.players[0].life, 4);
    assert_eq!(s.name(larva), "Elder");
    assert_eq!(s.power(larva), 9);
    assert_eq!(s.waiting, WaitingFor::Action { player: 1 });
    // Evolved: the Action ability is gone.
    s.active = 0;
    s.start();
    assert!(!legal_actions(&s).contains(&GameAction::Activate(larva)));
}

#[test]
fn a_creature_that_leaves_play_loses_its_evolution() {
    let mut s = scenario();
    let larva = s.add_card(0, Zone::Board, &LARVA);
    let bulldozer = s.add_card(1, Zone::Board, &BULLDOZER);
    s.start();
    act(&mut s, GameAction::Activate(larva));
    // Player 1 (bulldozer, power 10) attacks; the elder blocks and loses.
    act(&mut s, GameAction::Attack(bulldozer));
    act(&mut s, GameAction::Block(larva));
    assert!(s.players[0].discard.contains(&larva));
    assert_eq!(s.name(larva), "Larva");
}

static BULLDOZER: CardDef = CardDef::new("Bulldozer", 10);

static CHRYSALIS: CardDef = CardDef {
    statics: &[StaticAbility::EvolveInsteadOfDefeat { into: &ELDER }],
    ..CardDef::new("Chrysalis", 1)
};

#[test]
fn evolving_instead_of_being_defeated() {
    let mut s = scenario();
    let chrysalis = s.add_card(0, Zone::Board, &CHRYSALIS);
    let bulldozer = s.add_card(1, Zone::Board, &BULLDOZER);
    s.start();
    act(&mut s, GameAction::Attack(chrysalis));
    act(&mut s, GameAction::Block(bulldozer));
    assert_eq!(s.controller(chrysalis), Some(0), "still in play");
    assert_eq!(s.name(chrysalis), "Elder");
    assert!(s.players[0].discard.is_empty());
}

// ---- "You may … If you do" ---------------------------------------------------

static MAY_LOSE_THEN_DEFEAT: CardDef = card_with(
    "Gambler",
    2,
    &[ability(
        Trigger::Play,
        Effect::Then {
            first: &Effect::Optional(&Effect::LoseLife {
                player: PlayerRef::You,
                amount: Quantity::Fixed(1),
            }),
            then: &DEFEAT_ENEMY,
            per: false,
        },
    )],
);

#[test]
fn optional_effect_asks_and_the_follow_up_needs_it_to_be_done() {
    for accept in [true, false] {
        let mut s = scenario();
        let gambler = s.add_card(0, Zone::Hand, &MAY_LOSE_THEN_DEFEAT);
        let victim = s.add_card(1, Zone::Board, card("Moss Golem"));
        s.start();
        act(&mut s, GameAction::Play(gambler));
        assert!(matches!(
            s.waiting,
            WaitingFor::Confirm {
                player: 0,
                kind: ConfirmKind::MayDo,
                ..
            }
        ));
        if accept {
            act(&mut s, GameAction::Accept);
            assert_eq!(s.players[0].life, 2);
            assert!(matches!(
                s.waiting,
                WaitingFor::ChooseCards {
                    purpose: ChoicePurpose::Defeat,
                    ..
                }
            ));
            act(&mut s, GameAction::Choose(victim));
            assert!(s.players[1].discard.contains(&victim));
        } else {
            act(&mut s, GameAction::Decline);
            assert_eq!(s.players[0].life, 3);
            assert_eq!(s.controller(victim), Some(1));
            assert_eq!(s.waiting, WaitingFor::Action { player: 1 });
        }
    }
}

static DISCARD_FOR_DEFEATS: CardDef = card_with(
    "Tornado",
    8,
    &[ability(
        Trigger::Play,
        Effect::Then {
            first: &Effect::Discard {
                player: PlayerRef::You,
                amount: Quantity::Fixed(2),
                up_to: true,
            },
            then: &DEFEAT_ENEMY,
            per: true,
        },
    )],
);

#[test]
fn per_done_repeats_the_follow_up_once_per_time() {
    let mut s = scenario();
    let tornado = s.add_card(0, Zone::Hand, &DISCARD_FOR_DEFEATS);
    let (a, b, c) = (
        s.add_card(0, Zone::Hand, card("Ferret Scout")),
        s.add_card(0, Zone::Hand, card("Rabid Otter")),
        s.add_card(0, Zone::Hand, card("Venom Newt")),
    );
    let targets: Vec<_> = (0..3)
        .map(|_| s.add_card(1, Zone::Board, card("Moss Golem")))
        .collect();
    s.start();
    act(&mut s, GameAction::Play(tornado));
    act(&mut s, GameAction::Choose(a));
    act(&mut s, GameAction::Choose(b));
    // Two cards discarded: two creatures to defeat.
    act(&mut s, GameAction::Choose(targets[0]));
    act(&mut s, GameAction::Choose(targets[1]));
    assert_eq!(s.players[1].board, vec![targets[2]]);
    assert!(s.players[0].hand.contains(&c));
    assert_eq!(s.waiting, WaitingFor::Action { player: 1 });
}

#[test]
fn per_done_does_nothing_when_nothing_was_done() {
    let mut s = scenario();
    let tornado = s.add_card(0, Zone::Hand, &DISCARD_FOR_DEFEATS);
    s.add_card(0, Zone::Hand, card("Ferret Scout"));
    let target = s.add_card(1, Zone::Board, card("Moss Golem"));
    s.start();
    act(&mut s, GameAction::Play(tornado));
    act(&mut s, GameAction::Done);
    assert_eq!(s.controller(target), Some(1));
}

// ---- Restrictions ---------------------------------------------------------------

static PACIFIER: CardDef = card_with_static(
    "Pacifier",
    4,
    &[StaticAbility::Restrict {
        restriction: Restriction::CantBlock,
        filter: ENEMY.ranked(Rank::Highest),
    }],
);
static LION: CardDef = card_with_static(
    "Lion",
    8,
    &[StaticAbility::Restrict {
        restriction: Restriction::CantAttack,
        filter: ENEMY.ranked(Rank::Lowest),
    }],
);

#[test]
fn highest_power_enemies_cannot_block() {
    let mut s = scenario();
    let attacker = s.add_card(0, Zone::Board, card("Moss Golem"));
    s.add_card(0, Zone::Board, &PACIFIER);
    let big = s.add_card(1, Zone::Board, &BULLDOZER);
    let small = s.add_card(1, Zone::Board, card("Ferret Scout"));
    s.start();
    assert!(!s.may_block(big));
    assert!(s.may_block(small));
    act(&mut s, GameAction::Attack(attacker));
    assert!(
        matches!(&s.waiting, WaitingFor::Block { candidates, .. } if candidates == &vec![small])
    );
}

#[test]
fn lowest_power_enemies_cannot_attack_and_being_unable_to_act_loses() {
    let mut s = GameState::empty(1);
    s.add_card(0, Zone::Board, &LION);
    let small = s.add_card(1, Zone::Board, card("Ferret Scout"));
    s.add_card(1, Zone::Board, card("Moss Golem"));
    s.start();
    assert!(!s.can_attack(small));
    assert!(!legal_actions(&s).contains(&GameAction::Attack(small)));
    // Only the small creature and an empty hand: nothing to do.
    let mut s = GameState::empty(1);
    s.add_card(0, Zone::Board, &LION);
    s.add_card(1, Zone::Board, card("Ferret Scout"));
    s.add_card(1, Zone::Board, card("Ferret Scout"));
    s.start();
    assert_eq!(s.winner(), Some(0));
}

static BOARSTER: CardDef = card_with_static(
    "Boarster",
    6,
    &[StaticAbility::Restrict {
        restriction: Restriction::CantBlock,
        filter: CreatureFilter::this(),
    }],
);

#[test]
fn a_creature_that_cannot_block_cannot_be_picked_by_a_hunter_either() {
    let mut s = scenario();
    let hunter = s.add_card(0, Zone::Board, card("Sky Harrier"));
    let boar = s.add_card(1, Zone::Board, &BOARSTER);
    let golem = s.add_card(1, Zone::Board, card("Moss Golem"));
    s.start();
    act(&mut s, GameAction::Attack(hunter));
    assert!(!s.may_block(boar));
    assert!(
        matches!(&s.waiting, WaitingFor::Block { chooser: 0, candidates, .. } if candidates == &vec![golem])
    );
}

static WATTS: CardDef = card_with_static(
    "Watts",
    5,
    &[StaticAbility::CantBeBlockedBy {
        attackers: Affected::This,
        blockers: ANY_CARD.with_any_keyword(),
    }],
);

#[test]
fn cannot_be_blocked_by_creatures_with_keywords() {
    let mut s = scenario();
    let watts = s.add_card(0, Zone::Board, &WATTS);
    s.add_card(1, Zone::Board, card("Ferret Scout")); // sneaky
    let plain = s.add_card(1, Zone::Board, card("Moss Golem"));
    s.start();
    act(&mut s, GameAction::Attack(watts));
    assert!(
        matches!(&s.waiting, WaitingFor::Block { candidates, .. } if candidates == &vec![plain])
    );
}

static HIPPO: CardDef = card_with_static("Hippo", 7, &[StaticAbility::MustBeHunted]);

#[test]
fn a_creature_demanding_to_be_hunted_forces_a_hunter_attack() {
    let mut s = scenario();
    s.add_card(0, Zone::Board, &HIPPO);
    s.add_card(0, Zone::Board, card("Moss Golem"));
    let hunter = s.add_card(1, Zone::Board, card("Sky Harrier"));
    let other = s.add_card(1, Zone::Board, card("Moss Golem"));
    s.active = 1;
    s.start();
    let actions = legal_actions(&s);
    assert_eq!(actions, vec![GameAction::Attack(hunter)]);
    assert!(!actions.contains(&GameAction::Attack(other)));
    act(&mut s, GameAction::Attack(hunter));
    // The hunter has to pick the hippo: no other candidate, no way to decline.
    let WaitingFor::Block { candidates, .. } = &s.waiting else {
        panic!()
    };
    assert_eq!(candidates.len(), 1);
    assert!(!legal_actions(&s).contains(&GameAction::NoBlock));
}

// ---- Effects lasting until the end of the turn ----------------------------------

static SURFER: CardDef = card_with(
    "Surfer",
    8,
    &[ability(
        Trigger::Attack,
        Effect::ChooseCreatures {
            action: CreatureAction::ThisTurn(TurnMod::CantBlock),
            filter: ANY,
            count: ONE,
        },
    )],
);
static BLASTFISH: CardDef = CardDef {
    keywords: &[Keyword::Poisonous],
    abilities: &[ability(
        Trigger::Attack,
        Effect::AllCreatures {
            action: CreatureAction::ThisTurn(TurnMod::CantBeDefeated),
            filter: CreatureFilter::this(),
        },
    )],
    ..CardDef::new("Blastfish", 1)
};

#[test]
fn a_creature_marked_this_turn_cannot_block_until_the_turn_ends() {
    let mut s = scenario();
    let surfer = s.add_card(0, Zone::Board, &SURFER);
    let golem = s.add_card(1, Zone::Board, card("Moss Golem"));
    s.start();
    act(&mut s, GameAction::Attack(surfer));
    let candidates_ok = matches!(&s.waiting, WaitingFor::ChooseCards { .. });
    assert!(candidates_ok);
    act(&mut s, GameAction::Choose(golem));
    // Nobody can block: the attack goes through.
    assert_eq!(s.players[1].life, 2);
    assert_eq!(s.waiting, WaitingFor::Action { player: 1 });
    assert!(s.may_block(golem), "the mark is gone");
}

#[test]
fn cannot_be_defeated_this_turn() {
    let mut s = scenario();
    let fish = s.add_card(0, Zone::Board, &BLASTFISH);
    let golem = s.add_card(1, Zone::Board, card("Moss Golem"));
    s.start();
    act(&mut s, GameAction::Attack(fish));
    act(&mut s, GameAction::Block(golem));
    assert_eq!(s.controller(fish), Some(0));
    assert!(
        s.players[1].discard.contains(&golem),
        "poisonous still kills"
    );
}

// ---- Forced attack -----------------------------------------------------------------

static TRICKSTER: CardDef = card_with(
    "Trickster",
    6,
    &[ability(
        Trigger::Action,
        Effect::ChooseCreatures {
            action: CreatureAction::MustAttack,
            filter: ENEMY,
            count: ONE,
        },
    )],
);

#[test]
fn an_enemy_creature_can_be_forced_to_attack() {
    let mut s = scenario();
    let trickster = s.add_card(0, Zone::Board, &TRICKSTER);
    let victim = s.add_card(1, Zone::Board, card("Rabid Otter"));
    s.start();
    act(&mut s, GameAction::Activate(trickster));
    act(&mut s, GameAction::Choose(victim));
    // The forced attacker's controller is the attacker: I choose the blocker.
    assert!(
        matches!(&s.waiting, WaitingFor::Block { chooser: 0, attacker, .. } if *attacker == victim)
    );
    act(&mut s, GameAction::Block(trickster));
    assert!(s.players[1].discard.contains(&victim));
    assert_eq!(s.waiting, WaitingFor::Action { player: 1 });
}

// ---- Roll, control, hands, winning ---------------------------------------------------

static ALWAYS_ROLLS: CardDef = card_with(
    "Lucky",
    3,
    &[ability(
        Trigger::Play,
        Effect::Roll {
            threshold: 1,
            then: &LOSE_ONE,
        },
    )],
);
static NEVER_ROLLS: CardDef = card_with(
    "Unlucky",
    3,
    &[ability(
        Trigger::Play,
        Effect::Roll {
            threshold: 7,
            then: &LOSE_ONE,
        },
    )],
);

#[test]
fn a_successful_roll_repeats_until_it_fails() {
    let mut s = scenario();
    let lucky = s.add_card(0, Zone::Hand, &ALWAYS_ROLLS);
    s.start();
    act(&mut s, GameAction::Play(lucky));
    assert_eq!(s.winner(), Some(0), "the opponent lost all their life");

    let mut s = scenario();
    let unlucky = s.add_card(0, Zone::Hand, &NEVER_ROLLS);
    s.start();
    act(&mut s, GameAction::Play(unlucky));
    assert_eq!(s.players[1].life, 3);
}

static GIFT: CardDef = CardDef {
    abilities: &[
        ability(Trigger::Play, Effect::GiveControl),
        ability(
            Trigger::Defeated,
            Effect::LoseLife {
                player: PlayerRef::You,
                amount: Quantity::Fixed(2),
            },
        ),
    ],
    ..CardDef::new("Gift", 1)
};

#[test]
fn the_opponent_takes_control_of_a_creature_that_gives_itself() {
    let mut s = scenario();
    let gift = s.add_card(0, Zone::Hand, &GIFT);
    s.start();
    act(&mut s, GameAction::Play(gift));
    assert_eq!(s.controller(gift), Some(1));
}

static MACAW: CardDef = card_with("Macaw", 8, &[ability(Trigger::Attack, Effect::SwapHands)]);
static UNIGON: CardDef = card_with(
    "Unigon",
    9,
    &[ability(
        Trigger::Attack,
        Effect::If {
            condition: Condition::Compare {
                left: Quantity::Hand(PlayerRef::You),
                comparator: Comparator::Equal,
                right: Quantity::Fixed(0),
            },
            then: &Effect::EndGame {
                winner: PlayerRef::You,
            },
        },
    )],
);

#[test]
fn swapping_hands_and_winning_with_an_empty_hand() {
    let mut s = GameState::empty(0);
    let macaw = s.add_card(0, Zone::Board, &MACAW);
    let mine = s.add_card(0, Zone::Hand, card("Ferret Scout"));
    let theirs = s.add_card(1, Zone::Hand, card("Rabid Otter"));
    s.start();
    act(&mut s, GameAction::Attack(macaw));
    assert_eq!(s.players[0].hand, vec![theirs]);
    assert_eq!(s.players[1].hand, vec![mine]);

    let mut s = GameState::empty(0);
    let unigon = s.add_card(0, Zone::Board, &UNIGON);
    s.add_card(1, Zone::Hand, card("Moss Golem"));
    s.start();
    act(&mut s, GameAction::Attack(unigon));
    assert_eq!(s.winner(), Some(0));
}

// ---- End of turn -------------------------------------------------------------------------

static JAZZ: CardDef = card_with(
    "Jazz",
    9,
    &[ability(
        Trigger::EndOfTurn,
        Effect::If {
            condition: Condition::YourTurn,
            then: &Effect::AllCreatures {
                action: CreatureAction::TakeControl,
                filter: ENEMY.blocked_this_turn(),
            },
        },
    )],
);

static TOUGH_GOLEM: CardDef = CardDef {
    keywords: &[Keyword::Tough],
    ..CardDef::new("Tough Golem", 3)
};

#[test]
fn end_of_turn_trigger_takes_control_of_creatures_that_blocked() {
    let mut s = scenario();
    let jazz = s.add_card(0, Zone::Board, &JAZZ);
    let blocker = s.add_card(1, Zone::Board, &TOUGH_GOLEM);
    let bystander = s.add_card(1, Zone::Board, card("Moss Golem"));
    s.start();
    act(&mut s, GameAction::Attack(jazz));
    act(&mut s, GameAction::Block(blocker));
    // The tough blocker survived (exhausted) and now changes sides.
    assert_eq!(s.controller(blocker), Some(0));
    assert_eq!(s.controller(bystander), Some(1));
    assert_eq!(s.waiting, WaitingFor::Action { player: 1 });
}

static FELIX: CardDef = CardDef {
    keywords: &[Keyword::Hunter],
    abilities: &[ability(
        Trigger::EndOfTurn,
        Effect::If {
            condition: Condition::SourceDefeatedEnemyThisTurn,
            then: &Effect::Evolve { into: &ELDER },
        },
    )],
    ..CardDef::new("Felix", 7)
};

#[test]
fn evolving_at_the_end_of_a_turn_in_which_it_defeated_an_enemy() {
    let mut s = scenario();
    let felix = s.add_card(0, Zone::Board, &FELIX);
    let victim = s.add_card(1, Zone::Board, card("Rabid Otter"));
    s.start();
    act(&mut s, GameAction::Attack(felix));
    act(&mut s, GameAction::Block(victim));
    assert_eq!(s.name(felix), "Elder");

    // No kill, no evolution.
    let mut s = scenario();
    let felix = s.add_card(0, Zone::Board, &FELIX);
    s.start();
    act(&mut s, GameAction::Attack(felix));
    assert_eq!(s.name(felix), "Felix");
}

// ---- Hidden zones and hands ----------------------------------------------------------------

static HAMSTER: CardDef = card_with(
    "Hamster",
    2,
    &[ability(
        Trigger::Play,
        Effect::Sequence(&[
            Effect::TakeFromOpponentHand,
            Effect::PlayReceived { optional: true },
        ]),
    )],
);

#[test]
fn the_opponent_gives_a_card_and_you_choose_to_play_it_or_keep_it() {
    for play in [true, false] {
        let mut s = scenario();
        let hamster = s.add_card(0, Zone::Hand, &HAMSTER);
        let given = s.add_card(1, Zone::Hand, card("Healing Moth"));
        s.start();
        act(&mut s, GameAction::Play(hamster));
        // The opponent picks which card to give.
        assert_eq!(s.waiting.player(), Some(1));
        assert!(matches!(
            s.waiting,
            WaitingFor::ChooseCards {
                purpose: ChoicePurpose::GiveCard,
                ..
            }
        ));
        act(&mut s, GameAction::Choose(given));
        assert!(matches!(
            s.waiting,
            WaitingFor::Confirm {
                player: 0,
                kind: ConfirmKind::PlayIt,
                ..
            }
        ));
        act(
            &mut s,
            if play {
                GameAction::Accept
            } else {
                GameAction::Decline
            },
        );
        assert_eq!(s.controller(given) == Some(0), play);
        assert_eq!(s.players[0].hand.contains(&given), !play);
        if play {
            assert_eq!(s.players[0].life, 5, "its Play effect triggered");
        }
    }
}

static RATOMANCER: CardDef = card_with(
    "Ratomancer",
    2,
    &[ability(
        Trigger::Play,
        Effect::PlayFromDiscard {
            from: PlayerRef::You,
            filter: ANY_CARD.max_power(4),
            count: Count::any(),
            play_effects: false,
        },
    )],
);

#[test]
fn play_any_number_of_small_cards_from_the_discard_without_their_play_effects() {
    let mut s = scenario();
    let rat = s.add_card(0, Zone::Hand, &RATOMANCER);
    let moth = s.add_card(0, Zone::Discard, card("Healing Moth"));
    let scout = s.add_card(0, Zone::Discard, card("Ferret Scout"));
    let big = s.add_card(0, Zone::Discard, &BULLDOZER);
    s.start();
    act(&mut s, GameAction::Play(rat));
    let WaitingFor::ChooseCards {
        candidates,
        optional: true,
        ..
    } = &s.waiting
    else {
        panic!()
    };
    assert!(!candidates.contains(&big));
    act(&mut s, GameAction::Choose(moth));
    act(&mut s, GameAction::Choose(scout));
    assert_eq!(s.players[0].board, vec![rat, moth, scout]);
    assert_eq!(
        s.players[0].life, 3,
        "Healing Moth's Play effect did not trigger"
    );
    assert_eq!(s.players[0].discard, vec![big]);
}

static HYENIX: CardDef = CardDef {
    discard_abilities: &[ability(
        Trigger::LifeLost,
        Effect::Optional(&Effect::PlaySelf),
    )],
    ..CardDef::new("Hyenix", 7)
};

#[test]
fn a_card_in_the_discard_pile_can_react_to_losing_life() {
    let mut s = scenario();
    let hyenix = s.add_card(0, Zone::Discard, &HYENIX);
    let bomber = s.add_card(1, Zone::Board, card("Moss Golem"));
    s.active = 1;
    s.start();
    act(&mut s, GameAction::Attack(bomber));
    // Player 0 has no creature to block: they lose a life, and Hyenix reacts.
    assert!(matches!(
        s.waiting,
        WaitingFor::Confirm { player: 0, source, .. } if source == hyenix
    ));
    act(&mut s, GameAction::Accept);
    assert_eq!(s.controller(hyenix), Some(0));
    assert_eq!(s.players[0].life, 2);
}

static CHIMP: CardDef = card_with(
    "Chimp",
    5,
    &[ability(
        Trigger::Play,
        Effect::Discard {
            player: PlayerRef::Opponent,
            amount: Quantity::CreaturesControlled(PlayerRef::Opponent),
            up_to: false,
        },
    )],
);
static FORTRESS: CardDef = card_with(
    "Fortress",
    10,
    &[ability(
        Trigger::Play,
        Effect::DiscardZones {
            player: PlayerRef::Opponent,
            hand: true,
            deck: true,
        },
    )],
);

#[test]
fn discard_amounts_can_depend_on_the_board_and_whole_zones_can_be_discarded() {
    let mut s = scenario();
    let chimp = s.add_card(0, Zone::Hand, &CHIMP);
    s.add_card(1, Zone::Board, card("Moss Golem"));
    s.add_card(1, Zone::Board, card("Moss Golem"));
    for _ in 0..4 {
        s.add_card(1, Zone::Hand, card("Ferret Scout"));
    }
    s.start();
    act(&mut s, GameAction::Play(chimp));
    let WaitingFor::ChooseCards { remaining: 2, .. } = &s.waiting else {
        panic!("{:?}", s.waiting)
    };

    let mut s = scenario();
    let fortress = s.add_card(0, Zone::Hand, &FORTRESS);
    s.add_card(1, Zone::Hand, card("Ferret Scout"));
    s.add_card(1, Zone::Deck, card("Rabid Otter"));
    s.start();
    act(&mut s, GameAction::Play(fortress));
    assert!(s.players[1].hand.is_empty() && s.players[1].deck.is_empty());
    assert_eq!(s.players[1].discard.len(), 3);
}

static RECALLER: CardDef = card_with(
    "Recaller",
    6,
    &[ability(
        Trigger::Play,
        Effect::AllCreatures {
            action: CreatureAction::ReturnToHand,
            filter: ENEMY,
        },
    )],
);

#[test]
fn enemy_creatures_return_to_their_controllers_hand() {
    let mut s = scenario();
    let recaller = s.add_card(0, Zone::Hand, &RECALLER);
    let victim = s.add_card(1, Zone::Board, &TOUGH_GOLEM);
    s.cards[victim.0 as usize].exhausted = true;
    s.start();
    act(&mut s, GameAction::Play(recaller));
    assert!(s.players[1].hand.contains(&victim));
    assert!(!s.card(victim).exhausted);
}

static UTILITY: CardDef = card_with(
    "Utility",
    4,
    &[ability(
        Trigger::Play,
        Effect::ChooseCreatures {
            action: CreatureAction::CopyPlayEffect,
            filter: ANY_CARD.with_trigger(Trigger::Play),
            count: Count::up_to(1),
        },
    )],
);

#[test]
fn copying_the_play_effect_of_another_creature() {
    let mut s = scenario();
    let utility = s.add_card(0, Zone::Hand, &UTILITY);
    let moth = s.add_card(1, Zone::Board, card("Healing Moth"));
    s.add_card(1, Zone::Board, card("Moss Golem"));
    s.start();
    act(&mut s, GameAction::Play(utility));
    let WaitingFor::ChooseCards {
        candidates,
        optional: true,
        ..
    } = &s.waiting
    else {
        panic!()
    };
    assert_eq!(candidates, &vec![moth], "only creatures with a Play effect");
    act(&mut s, GameAction::Choose(moth));
    assert_eq!(
        s.players[0].life, 5,
        "the copier's controller gains the life"
    );
}

// ---- Player-level rules ------------------------------------------------------------------------

static TAXER: CardDef = card_with_static("Taxer", 7, &[StaticAbility::MindbugTax { life: 1 }]);
static CATCHER: CardDef = card_with_static("Catcher", 9, &[StaticAbility::PreventMindbugs]);
static KNIGHT: CardDef = CardDef {
    statics: &[StaticAbility::CantLoseLife {
        player: PlayerRef::You,
    }],
    abilities: &[ability(
        Trigger::Defeated,
        Effect::EndGame {
            winner: PlayerRef::Opponent,
        },
    )],
    ..CardDef::new("Knight", 5)
};

#[test]
fn mindbugs_can_cost_life_or_be_forbidden() {
    let mut s = scenario();
    s.players[1].mindbugs = 2;
    s.add_card(0, Zone::Board, &TAXER);
    let golem = s.add_card(0, Zone::Hand, card("Moss Golem"));
    s.start();
    act(&mut s, GameAction::Play(golem));
    act(&mut s, GameAction::UseMindbug);
    assert_eq!(s.players[1].life, 2, "lost 1 life first");

    let mut s = scenario();
    s.players[1].mindbugs = 2;
    s.add_card(0, Zone::Board, &CATCHER);
    let golem = s.add_card(0, Zone::Hand, card("Moss Golem"));
    s.start();
    act(&mut s, GameAction::Play(golem));
    assert_eq!(
        s.waiting,
        WaitingFor::Action { player: 1 },
        "no Mindbug offered"
    );
}

#[test]
fn a_player_who_cannot_lose_life_ignores_attacks_and_a_defeated_knight_loses() {
    let mut s = scenario();
    let attacker = s.add_card(0, Zone::Board, card("Moss Golem"));
    let knight = s.add_card(1, Zone::Board, &KNIGHT);
    s.start();
    act(&mut s, GameAction::Attack(attacker));
    assert_eq!(s.waiting.player(), Some(1));
    act(&mut s, GameAction::NoBlock);
    assert_eq!(s.players[1].life, 3);
    // Now the knight dies in a fight.
    s.active = 0;
    s.start();
    s.add_card(0, Zone::Board, &BULLDOZER);
    let big = s.players[0].board[1];
    act(&mut s, GameAction::Attack(big));
    act(&mut s, GameAction::Block(knight));
    assert_eq!(s.winner(), Some(0));
}

static REVERSER: CardDef = card_with_static("Reverser", 5, &[StaticAbility::ReverseCombat]);

#[test]
fn reversed_combat_defeats_the_stronger_creature() {
    let mut s = scenario();
    let reverser = s.add_card(0, Zone::Board, &REVERSER);
    let weak = s.add_card(1, Zone::Board, card("Ferret Scout"));
    s.start();
    act(&mut s, GameAction::Attack(reverser));
    act(&mut s, GameAction::Block(weak));
    assert!(s.players[0].discard.contains(&reverser));
    assert_eq!(s.controller(weak), Some(1));
}

static ALPACA: CardDef = CardDef {
    abilities: &[
        ability(Trigger::Play, Effect::SetAsideOthers),
        ability(Trigger::Defeated, Effect::ReturnSetAside),
    ],
    ..CardDef::new("Alpaca", 2)
};

#[test]
fn creatures_set_aside_come_back_when_the_source_is_defeated() {
    let mut s = scenario();
    let alpaca = s.add_card(0, Zone::Hand, &ALPACA);
    let mine = s.add_card(0, Zone::Board, card("Moss Golem"));
    let theirs = s.add_card(1, Zone::Board, card("Rabid Otter"));
    s.start();
    act(&mut s, GameAction::Play(alpaca));
    assert!(!s.on_board(mine) && !s.on_board(theirs));
    // Player 1 has nothing on board and attacks with nothing... they play a big creature.
    let bulldozer = s.add_card(1, Zone::Board, &BULLDOZER);
    s.start();
    s.active = 1;
    s.start();
    act(&mut s, GameAction::Attack(bulldozer));
    act(&mut s, GameAction::Block(alpaca));
    assert_eq!(s.controller(mine), Some(0));
    assert_eq!(s.controller(theirs), Some(1));
}

static ERIC: CardDef = card_with(
    "Eric",
    3,
    &[ability(Trigger::Play, Effect::UnusedToBottom { count: 2 })],
);

#[test]
fn unused_cards_go_under_the_draw_pile() {
    let mut s = scenario();
    let eric = s.add_card(0, Zone::Hand, &ERIC);
    let old = s.add_card(0, Zone::Deck, card("Ferret Scout"));
    let (u1, u2, u3) = (
        s.add_card(0, Zone::Unused, card("Moss Golem")),
        s.add_card(0, Zone::Unused, card("Rabid Otter")),
        s.add_card(0, Zone::Unused, card("Venom Newt")),
    );
    s.start();
    act(&mut s, GameAction::Play(eric));
    // Played card refilled the hand from the top of the deck (the old card).
    assert!(s.players[0].hand.contains(&old));
    assert_eq!(
        s.players[0].deck,
        vec![u2, u3],
        "top of the unused pile first, at the bottom"
    );
    assert_eq!(s.unused, vec![u1]);
}

static STEVE: CardDef = card_with_static(
    "Steve",
    8,
    &[StaticAbility::CantPlayFromHand {
        player: PlayerRef::Opponent,
        filter: ANY_CARD.max_power(4),
    }],
);
static BRAIN: CardDef = card_with_static(
    "Brain",
    3,
    &[StaticAbility::PreventHandGain {
        player: PlayerRef::Opponent,
    }],
);

#[test]
fn hands_can_be_restricted() {
    let mut s = GameState::empty(1);
    s.add_card(0, Zone::Board, &STEVE);
    let weak = s.add_card(1, Zone::Hand, card("Ferret Scout"));
    let strong = s.add_card(1, Zone::Hand, &BULLDOZER);
    s.start();
    let actions = legal_actions(&s);
    assert!(!actions.contains(&GameAction::Play(weak)));
    assert!(actions.contains(&GameAction::Play(strong)));

    // Alien Brain: the opponent's Recall-style effects gain them nothing.
    let mut s = GameState::empty(1);
    s.add_card(0, Zone::Board, &BRAIN);
    let victim = s.add_card(1, Zone::Board, &TOUGH_GOLEM);
    s.add_card(1, Zone::Hand, card("Moss Golem"));
    let recaller = s.add_card(1, Zone::Hand, &RECALLER);
    s.start();
    act(&mut s, GameAction::Play(recaller));
    // The recaller is player 1's: its enemies are player 0's creatures.
    assert_eq!(s.controller(victim), Some(1));
    assert!(s.hand_gain_prevented(1));
}

static PANDAMME: CardDef = card_with_static(
    "Pandamme",
    5,
    &[StaticAbility::Modify {
        affected: Affected::Enemies,
        condition: None,
        modification: Modification::Ability(&Ability {
            trigger: Trigger::Attack,
            effect: Effect::Discard {
                player: PlayerRef::You,
                amount: Quantity::Fixed(1),
                up_to: false,
            },
        }),
    }],
);

#[test]
fn abilities_can_be_granted_to_other_creatures() {
    let mut s = GameState::empty(1);
    s.add_card(0, Zone::Board, &PANDAMME);
    let attacker = s.add_card(1, Zone::Board, card("Moss Golem"));
    let in_hand = s.add_card(1, Zone::Hand, card("Ferret Scout"));
    s.start();
    act(&mut s, GameAction::Attack(attacker));
    // The attacker's controller discards a card.
    assert!(matches!(
        &s.waiting,
        WaitingFor::ChooseCards {
            player: 1,
            purpose: ChoicePurpose::Discard,
            ..
        }
    ));
    act(&mut s, GameAction::Choose(in_hand));
    assert!(s.players[1].discard.contains(&in_hand));
}

static FROBLIN: CardDef = card_with_static(
    "Froblin",
    1,
    &[StaticAbility::Modify {
        affected: Affected::This,
        condition: None,
        modification: Modification::PowerPerOtherAlly(2),
    }],
);

#[test]
fn power_can_scale_with_the_other_allies() {
    let mut s = scenario();
    let froblin = s.add_card(0, Zone::Board, &FROBLIN);
    assert_eq!(s.power(froblin), 1);
    s.add_card(0, Zone::Board, card("Moss Golem"));
    s.add_card(0, Zone::Board, card("Moss Golem"));
    assert_eq!(s.power(froblin), 5);
    s.add_card(1, Zone::Board, card("Moss Golem"));
    assert_eq!(s.power(froblin), 5, "enemy creatures do not count");
}
