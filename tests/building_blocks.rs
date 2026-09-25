//! One test per engine building block, with ad-hoc cards: independent of
//! any catalog (official or example).

mod common;

use mindbreake::state::ChoicePurpose;
use mindbreake::types::*;
use mindbreake::{apply, view_for, GameAction, GameState, WaitingFor, Zone};

const fn on_play(name: &'static str, power: i32, effect: &'static [Ability]) -> CardDef {
    CardDef {
        abilities: effect,
        ..CardDef::new(name, power)
    }
}

static HARPY: CardDef = on_play(
    "Harpy",
    5,
    &[Ability {
        trigger: Trigger::Play,
        effect: Effect::ChooseCreatures {
            action: CreatureAction::TakeControl,
            filter: CreatureFilter::side(Side::Enemy).max_power(5),
            count: Count::up_to(2),
        },
    }],
);
static BOMBER: CardDef = on_play(
    "Bomber",
    2,
    &[Ability {
        trigger: Trigger::Play,
        effect: Effect::Discard {
            player: PlayerRef::Opponent,
            amount: Quantity::Fixed(2),
            up_to: false,
        },
    }],
);
static THIEF: CardDef = on_play(
    "Thief",
    6,
    &[Ability {
        trigger: Trigger::Play,
        effect: Effect::StealRandomFromHand { amount: 2 },
    }],
);
static COMPOST: CardDef = on_play(
    "Compost",
    3,
    &[Ability {
        trigger: Trigger::Play,
        effect: Effect::PlayFromDiscard {
            from: PlayerRef::You,
            filter: CreatureFilter::side(Side::Any),
            count: Count::exactly(1),
            play_effects: true,
        },
    }],
);
static RECALL: CardDef = on_play(
    "Recall",
    7,
    &[Ability {
        trigger: Trigger::Play,
        effect: Effect::ReturnFromDiscard { count: None },
    }],
);
static MIRROR: CardDef = on_play(
    "Mirror",
    7,
    &[Ability {
        trigger: Trigger::Play,
        effect: Effect::SetLife {
            player: PlayerRef::You,
            value: Quantity::Life(PlayerRef::Opponent),
        },
    }],
);
static UNDERDOG: CardDef = CardDef {
    abilities: &[Ability {
        trigger: Trigger::Attack,
        effect: Effect::If {
            condition: Condition::Compare {
                left: Quantity::CreaturesControlled(PlayerRef::You),
                comparator: Comparator::Less,
                right: Quantity::CreaturesControlled(PlayerRef::Opponent),
            },
            then: &Effect::ChooseCreatures {
                action: CreatureAction::Defeat,
                filter: CreatureFilter::side(Side::Any).including_source(),
                count: Count::exactly(1),
            },
        },
    }],
    ..CardDef::new("Underdog", 9)
};
static NIGHT_WOLF: CardDef = CardDef {
    statics: &[StaticAbility::Modify {
        affected: Affected::This,
        condition: Some(Condition::YourTurn),
        modification: Modification::Power(6),
    }],
    ..CardDef::new("Night Wolf", 2)
};
static MENTOR: CardDef = CardDef {
    statics: &[StaticAbility::Modify {
        affected: Affected::OtherAllies { max_power: Some(4) },
        condition: None,
        modification: Modification::Keywords(&[Keyword::Hunter, Keyword::Poisonous]),
    }],
    ..CardDef::new("Mentor", 1)
};
static COPYCAT: CardDef = CardDef {
    statics: &[StaticAbility::Modify {
        affected: Affected::This,
        condition: None,
        modification: Modification::CopyEnemyKeywords(&[Keyword::Hunter, Keyword::Sneaky]),
    }],
    ..CardDef::new("Copycat", 5)
};
static BULLY: CardDef = CardDef {
    statics: &[StaticAbility::CantBeBlockedBy {
        attackers: Affected::AllAllies,
        blockers: CreatureFilter::side(Side::Any).max_power(4),
    }],
    ..CardDef::new("Bully", 7)
};
static SILENCER: CardDef = CardDef {
    statics: &[StaticAbility::PreventPlayEffects {
        player: PlayerRef::Opponent,
    }],
    ..CardDef::new("Silencer", 2)
};

fn card(name: &str) -> &'static CardDef {
    common::card(name)
}

fn act(s: &mut GameState, action: GameAction) {
    apply(s, action).unwrap_or_else(|e| panic!("{e}"));
}

/// Player 0 active, both with a card in hand, no Mindbugs (unless re-added).
fn scenario() -> GameState {
    let mut s = GameState::empty(0);
    s.add_card(0, Zone::Hand, card("Moss Golem"));
    s.add_card(1, Zone::Hand, card("Moss Golem"));
    s.players[1].mindbugs = 0;
    s
}

#[test]
fn a_card_leaving_the_hand_is_replaced_immediately() {
    let mut s = scenario();
    s.players[1].mindbugs = 2;
    let golem = s.players[0].hand[0];
    let top = s.add_card(0, Zone::Deck, card("Ferret Scout"));
    s.start();
    act(&mut s, GameAction::Play(golem));
    assert!(matches!(s.waiting, WaitingFor::Mindbug { .. }));
    assert_eq!(s.players[0].hand, vec![top]);
}

#[test]
fn defeated_creature_goes_to_its_controllers_discard_and_triggers_for_them() {
    let mut s = scenario();
    s.players[1].mindbugs = 2;
    let golem = s.add_card(0, Zone::Board, card("Moss Golem"));
    let crow = s.add_card(0, Zone::Hand, card("Grave Crow"));
    s.start();
    act(&mut s, GameAction::Play(crow));
    act(&mut s, GameAction::UseMindbug);
    // Player 0 takes another turn and attacks; player 1 blocks with the crow.
    act(&mut s, GameAction::Attack(golem));
    act(&mut s, GameAction::Block(crow));
    assert!(s.players[1].discard.contains(&crow));
    // Grave Crow: its controller (player 1)'s opponent loses 1 life.
    assert_eq!(s.players[0].life, 2);
}

#[test]
fn up_to_n_choice_can_stop_early() {
    let mut s = scenario();
    let harpy = s.add_card(0, Zone::Hand, &HARPY);
    let scout = s.add_card(1, Zone::Board, card("Ferret Scout"));
    let otter = s.add_card(1, Zone::Board, card("Rabid Otter"));
    s.add_card(1, Zone::Board, card("Moss Golem"));
    s.start();
    act(&mut s, GameAction::Play(harpy));
    assert!(matches!(
        &s.waiting,
        WaitingFor::ChooseCards { purpose: ChoicePurpose::TakeControl, candidates, optional: true, .. }
            if candidates == &vec![scout, otter]
    ));
    act(&mut s, GameAction::Choose(scout));
    assert_eq!(s.controller(scout), Some(0));
    assert!(matches!(
        &s.waiting,
        WaitingFor::ChooseCards { remaining: 1, .. }
    ));
    act(&mut s, GameAction::Done);
    assert_eq!(s.controller(otter), Some(1));
    assert_eq!(s.waiting, WaitingFor::Action { player: 1 });
}

#[test]
fn discarding_player_chooses_and_refills_and_the_choice_is_private() {
    let mut s = GameState::empty(0);
    s.add_card(0, Zone::Hand, card("Moss Golem"));
    let bomber = s.add_card(0, Zone::Hand, &BOMBER);
    s.players[1].mindbugs = 0;
    let a = s.add_card(1, Zone::Hand, card("Ferret Scout"));
    let b = s.add_card(1, Zone::Hand, card("Rabid Otter"));
    let c = s.add_card(1, Zone::Hand, card("Venom Newt"));
    let d = s.add_card(1, Zone::Deck, card("Moss Golem"));
    s.start();
    act(&mut s, GameAction::Play(bomber));
    assert_eq!(s.waiting.player(), Some(1));
    let WaitingFor::ChooseCards { candidates, .. } = view_for(&s, 0).waiting else {
        panic!("expected a choice");
    };
    assert!(
        candidates.is_empty(),
        "player 0 must not see player 1's hand"
    );

    act(&mut s, GameAction::Choose(a));
    assert!(
        s.players[1].hand.contains(&d),
        "refilled right after discarding"
    );
    act(&mut s, GameAction::Choose(b));
    assert_eq!(s.players[1].discard, vec![a, b]);
    assert_eq!(s.players[1].hand, vec![c, d]);
    assert_eq!(s.waiting, WaitingFor::Action { player: 1 });
}

#[test]
fn stealing_random_cards_refills_the_victim() {
    let mut s = scenario();
    let thief = s.add_card(0, Zone::Hand, &THIEF);
    s.add_card(1, Zone::Hand, card("Ferret Scout"));
    s.add_card(1, Zone::Hand, card("Rabid Otter"));
    s.add_card(1, Zone::Deck, card("Venom Newt"));
    s.start();
    act(&mut s, GameAction::Play(thief));
    assert_eq!(s.players[0].hand.len(), 3);
    assert_eq!(s.players[1].hand.len(), 2);
    assert!(s.players[1].deck.is_empty());
}

#[test]
fn playing_from_discard_triggers_play_but_cannot_be_mindbugged() {
    let mut s = scenario();
    s.players[1].mindbugs = 2;
    let compost = s.add_card(0, Zone::Hand, &COMPOST);
    let moth = s.add_card(0, Zone::Discard, card("Healing Moth"));
    s.start();
    act(&mut s, GameAction::Play(compost));
    act(&mut s, GameAction::PassMindbug);
    act(&mut s, GameAction::Choose(moth));
    assert_eq!(s.controller(moth), Some(0));
    assert_eq!(s.players[0].life, 5);
    assert_eq!(s.waiting, WaitingFor::Action { player: 1 });
}

#[test]
fn return_entire_discard_pile_to_hand() {
    let mut s = scenario();
    let recall = s.add_card(0, Zone::Hand, &RECALL);
    let x = s.add_card(0, Zone::Discard, card("Ferret Scout"));
    let y = s.add_card(0, Zone::Discard, card("Rabid Otter"));
    s.start();
    act(&mut s, GameAction::Play(recall));
    assert!(s.players[0].discard.is_empty());
    assert!(s.players[0].hand.contains(&x) && s.players[0].hand.contains(&y));
}

#[test]
fn set_life_reads_a_quantity() {
    let mut s = scenario();
    let mirror = s.add_card(0, Zone::Hand, &MIRROR);
    s.players[1].life = 1;
    s.start();
    act(&mut s, GameAction::Play(mirror));
    assert_eq!(s.players[0].life, 1);
}

#[test]
fn conditional_effect_only_when_the_condition_holds() {
    let mut s = scenario();
    let underdog = s.add_card(0, Zone::Board, &UNDERDOG);
    s.add_card(1, Zone::Board, card("Ferret Scout"));
    s.add_card(1, Zone::Board, card("Rabid Otter"));
    s.start();
    act(&mut s, GameAction::Attack(underdog));
    assert!(matches!(
        s.waiting,
        WaitingFor::ChooseCards {
            purpose: ChoicePurpose::Defeat,
            ..
        }
    ));

    let mut s = scenario();
    let underdog = s.add_card(0, Zone::Board, &UNDERDOG);
    s.add_card(1, Zone::Board, card("Ferret Scout"));
    s.start();
    act(&mut s, GameAction::Attack(underdog));
    assert!(matches!(s.waiting, WaitingFor::Block { .. }));
}

#[test]
fn your_turn_power_bonus() {
    let mut s = scenario();
    let wolf = s.add_card(0, Zone::Board, &NIGHT_WOLF);
    assert_eq!(s.power(wolf), 8);
    s.active = 1;
    assert_eq!(s.power(wolf), 2);
}

#[test]
fn granted_keywords_respect_the_power_cap() {
    let mut s = scenario();
    let mentor = s.add_card(0, Zone::Board, &MENTOR);
    let scout = s.add_card(0, Zone::Board, card("Ferret Scout"));
    let golem = s.add_card(0, Zone::Board, card("Moss Golem"));
    assert_eq!(
        s.keywords(scout),
        [Keyword::Hunter, Keyword::Poisonous, Keyword::Sneaky]
    );
    assert!(s.keywords(golem).is_empty());
    assert!(s.keywords(mentor).is_empty());
}

#[test]
fn copied_keywords_follow_enemy_creatures() {
    let mut s = scenario();
    let copycat = s.add_card(0, Zone::Board, &COPYCAT);
    assert!(s.keywords(copycat).is_empty());
    s.add_card(1, Zone::Board, card("Ferret Scout"));
    assert_eq!(s.keywords(copycat), [Keyword::Sneaky]);
}

#[test]
fn blocking_restrictions_and_hunter_override() {
    let mut s = scenario();
    s.add_card(0, Zone::Board, &BULLY);
    let harrier = s.add_card(0, Zone::Board, card("Sky Harrier"));
    let scout = s.add_card(1, Zone::Board, card("Ferret Scout"));
    let golem = s.add_card(1, Zone::Board, card("Moss Golem"));
    s.start();
    act(&mut s, GameAction::Attack(harrier));
    // Hunter: any enemy creature, restrictions ignored.
    assert!(
        matches!(&s.waiting, WaitingFor::Block { chooser: 0, candidates, .. } if candidates == &vec![scout, golem])
    );
    act(&mut s, GameAction::NoBlock);
    // Defender: power 4 or less cannot block.
    assert!(
        matches!(&s.waiting, WaitingFor::Block { chooser: 1, candidates, .. } if candidates == &vec![golem])
    );
}

#[test]
fn prevented_play_effects_also_apply_to_the_mindbugger() {
    let mut s = scenario();
    s.players[1].mindbugs = 2;
    s.add_card(0, Zone::Board, &SILENCER);
    let moth = s.add_card(0, Zone::Hand, card("Healing Moth"));
    s.start();
    act(&mut s, GameAction::Play(moth));
    act(&mut s, GameAction::UseMindbug);
    assert_eq!(s.players[1].life, 3, "player 1 is the silencer's opponent");

    let mut s = scenario();
    s.players[1].mindbugs = 2;
    s.add_card(0, Zone::Board, &SILENCER);
    let moth = s.add_card(0, Zone::Hand, card("Healing Moth"));
    s.start();
    act(&mut s, GameAction::Play(moth));
    act(&mut s, GameAction::PassMindbug);
    assert_eq!(
        s.players[0].life, 5,
        "the silencer's controller is unaffected"
    );
}

#[test]
fn view_separates_granted_keywords_from_printed_ones() {
    let mut s = scenario();
    s.add_card(0, Zone::Board, &MENTOR);
    let scout = s.add_card(0, Zone::Board, card("Ferret Scout"));
    s.start();
    let view = view_for(&s, 0);
    let scout = view.you.board.iter().find(|c| c.id == scout).unwrap();
    assert_eq!(
        scout.granted_keywords,
        [Keyword::Hunter, Keyword::Poisonous]
    );
}
