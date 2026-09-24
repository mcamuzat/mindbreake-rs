use mindbreake::ai::{Agent, RandomAgent};
use mindbreake::cards::{by_name, default_pool};
use mindbreake::types::CardId;
use mindbreake::{apply, new_game, GameAction, GameState, WaitingFor, Zone};

/// Player 0 is active. Each player gets a card in hand so nobody loses for
/// being unable to act.
fn scenario() -> GameState {
    let mut s = GameState::empty(0);
    s.add_card(0, Zone::Hand, by_name("Moss Golem"));
    s.add_card(1, Zone::Hand, by_name("Moss Golem"));
    s
}

fn board(s: &mut GameState, player: usize, name: &str) -> CardId {
    s.add_card(player, Zone::Board, by_name(name))
}

fn act(s: &mut GameState, action: GameAction) {
    apply(s, action).unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn setup_deals_five_in_hand_and_five_in_deck() {
    let s = new_game(&default_pool(), 42);
    for p in &s.players {
        assert_eq!(
            (p.hand.len(), p.deck.len(), p.life, p.mindbugs),
            (5, 5, 3, 2)
        );
    }
    assert_eq!(s.waiting, WaitingFor::Action { player: s.active });
}

#[test]
fn unblocked_attack_costs_one_life_and_passes_the_turn() {
    let mut s = scenario();
    let golem = board(&mut s, 0, "Moss Golem");
    s.start();
    act(&mut s, GameAction::Attack(golem));
    assert_eq!(s.players[1].life, 2);
    assert_eq!(s.waiting, WaitingFor::Action { player: 1 });
}

#[test]
fn defender_may_decline_to_block() {
    let mut s = scenario();
    let golem = board(&mut s, 0, "Moss Golem");
    board(&mut s, 1, "Ferret Scout");
    s.start();
    act(&mut s, GameAction::Attack(golem));
    assert!(matches!(s.waiting, WaitingFor::Block { chooser: 1, .. }));
    act(&mut s, GameAction::NoBlock);
    assert_eq!(s.players[1].life, 2);
}

#[test]
fn equal_power_defeats_both_and_defeated_trigger_fires() {
    let mut s = scenario();
    let otter = board(&mut s, 0, "Rabid Otter");
    let crow = board(&mut s, 1, "Grave Crow");
    s.start();
    act(&mut s, GameAction::Attack(otter));
    act(&mut s, GameAction::Block(crow));
    assert!(s.players[0].discard.contains(&otter));
    assert!(s.players[1].discard.contains(&crow));
    // Grave Crow: its controller's opponent (player 0) loses 1 life.
    assert_eq!(s.players[0].life, 2);
}

#[test]
fn poisonous_defeats_a_stronger_creature() {
    let mut s = scenario();
    let golem = board(&mut s, 0, "Moss Golem");
    let newt = board(&mut s, 1, "Venom Newt");
    s.start();
    act(&mut s, GameAction::Attack(golem));
    act(&mut s, GameAction::Block(newt));
    assert!(!s.on_board(golem));
    assert!(!s.on_board(newt));
}

#[test]
fn tough_survives_its_first_defeat_only() {
    let mut s = scenario();
    let golem = board(&mut s, 0, "Moss Golem");
    let sentinel = board(&mut s, 1, "Stone Sentinel");
    s.start();
    act(&mut s, GameAction::Attack(golem));
    act(&mut s, GameAction::Block(sentinel));
    assert!(s.on_board(sentinel));
    assert!(s.card(sentinel).exhausted);

    // Player 1 plays; player 0 passes on the Mindbug, then attacks again.
    let p1_card = s.players[1].hand[0];
    act(&mut s, GameAction::Play(p1_card));
    act(&mut s, GameAction::PassMindbug);
    act(&mut s, GameAction::Attack(golem));
    act(&mut s, GameAction::Block(sentinel));
    assert!(!s.on_board(sentinel));
}

#[test]
fn sneaky_can_only_be_blocked_by_sneaky() {
    let mut s = scenario();
    let scout = board(&mut s, 0, "Ferret Scout");
    board(&mut s, 1, "Moss Golem");
    s.start();
    act(&mut s, GameAction::Attack(scout));
    assert_eq!(
        s.players[1].life, 2,
        "no eligible blocker: the attack goes through"
    );

    let mut s = scenario();
    let scout = board(&mut s, 0, "Ferret Scout");
    let imp = board(&mut s, 1, "Mirror Imp");
    s.start();
    act(&mut s, GameAction::Attack(scout));
    assert!(matches!(&s.waiting, WaitingFor::Block { candidates, .. } if candidates == &vec![imp]));
}

#[test]
fn hunter_chooses_the_blocker() {
    let mut s = scenario();
    let harrier = board(&mut s, 0, "Sky Harrier");
    let scout = board(&mut s, 1, "Ferret Scout");
    board(&mut s, 1, "Moss Golem");
    s.start();
    act(&mut s, GameAction::Attack(harrier));
    assert!(matches!(s.waiting, WaitingFor::Block { chooser: 0, .. }));
    act(&mut s, GameAction::Block(scout));
    assert!(!s.on_board(scout));
    assert!(s.on_board(harrier));
}

#[test]
fn hunter_can_let_the_defender_choose() {
    let mut s = scenario();
    let harrier = board(&mut s, 0, "Sky Harrier");
    board(&mut s, 1, "Ferret Scout");
    s.start();
    act(&mut s, GameAction::Attack(harrier));
    act(&mut s, GameAction::NoBlock);
    assert!(matches!(s.waiting, WaitingFor::Block { chooser: 1, .. }));
}

#[test]
fn mindbug_steals_the_creature_and_grants_another_turn() {
    let mut s = scenario();
    s.add_card(0, Zone::Hand, by_name("Ferret Scout"));
    s.start();
    let golem = s.players[0].hand[0];
    act(&mut s, GameAction::Play(golem));
    assert_eq!(
        s.waiting,
        WaitingFor::Mindbug {
            player: 1,
            creature: golem
        }
    );
    act(&mut s, GameAction::UseMindbug);
    assert_eq!(s.controller(golem), Some(1));
    assert_eq!(s.players[1].mindbugs, 1);
    assert_eq!(s.waiting, WaitingFor::Action { player: 0 });
}

#[test]
fn losing_your_last_card_to_a_mindbug_loses_the_game() {
    let mut s = scenario();
    s.start();
    let golem = s.players[0].hand[0];
    act(&mut s, GameAction::Play(golem));
    act(&mut s, GameAction::UseMindbug);
    assert_eq!(s.winner(), Some(1));
}

#[test]
fn stolen_play_ability_belongs_to_the_new_controller() {
    let mut s = GameState::empty(0);
    let moth = s.add_card(0, Zone::Hand, by_name("Healing Moth"));
    s.add_card(0, Zone::Hand, by_name("Moss Golem"));
    s.start();
    act(&mut s, GameAction::Play(moth));
    act(&mut s, GameAction::UseMindbug);
    assert_eq!((s.players[0].life, s.players[1].life), (3, 5));
}

#[test]
fn frenzy_attacks_a_second_time_if_it_survives() {
    let mut s = scenario();
    let otter = board(&mut s, 0, "Rabid Otter");
    s.start();
    act(&mut s, GameAction::Attack(otter));
    assert_eq!(
        s.waiting,
        WaitingFor::FrenzyAttack {
            player: 0,
            attacker: otter
        }
    );
    act(&mut s, GameAction::FrenzyAttack);
    assert_eq!(s.players[1].life, 1);
    assert_eq!(s.waiting, WaitingFor::Action { player: 1 });
}

#[test]
fn attack_trigger_asks_for_a_target_then_combat_continues() {
    let mut s = scenario();
    let mauler = board(&mut s, 0, "Tusk Mauler");
    let newt = board(&mut s, 1, "Venom Newt");
    let scout = board(&mut s, 1, "Ferret Scout");
    s.start();
    act(&mut s, GameAction::Attack(mauler));
    assert!(
        matches!(&s.waiting, WaitingFor::ChooseCards { candidates, .. } if candidates == &vec![newt, scout])
    );
    act(&mut s, GameAction::Choose(newt));
    assert!(!s.on_board(newt));
    assert!(
        matches!(&s.waiting, WaitingFor::Block { candidates, .. } if candidates == &vec![scout])
    );
}

#[test]
fn defeat_all_hits_both_sides_but_not_the_source() {
    let mut s = scenario();
    let tortoise = s.add_card(0, Zone::Hand, by_name("Quake Tortoise"));
    let mine = board(&mut s, 0, "Ferret Scout");
    let theirs = board(&mut s, 1, "Rabid Otter");
    let big = board(&mut s, 1, "Moss Golem");
    s.players[1].mindbugs = 0;
    s.start();
    act(&mut s, GameAction::Play(tortoise));
    assert!(!s.on_board(mine) && !s.on_board(theirs));
    assert!(s.on_board(big) && s.on_board(tortoise));
}

#[test]
fn pack_alpha_boosts_other_allies_only() {
    let mut s = scenario();
    let alpha = board(&mut s, 0, "Pack Alpha");
    let scout = board(&mut s, 0, "Ferret Scout");
    let enemy = board(&mut s, 1, "Ferret Scout");
    assert_eq!((s.power(alpha), s.power(scout), s.power(enemy)), (4, 3, 2));
}

#[test]
fn player_who_cannot_act_loses() {
    let mut s = GameState::empty(0);
    s.add_card(1, Zone::Hand, by_name("Moss Golem"));
    s.start();
    assert_eq!(s.winner(), Some(1));
}

#[test]
fn illegal_action_is_rejected_without_changing_state() {
    let mut s = scenario();
    s.start();
    let before = s.waiting.clone();
    assert!(apply(&mut s, GameAction::UseMindbug).is_err());
    assert_eq!(s.waiting, before);
}

#[test]
fn random_games_always_terminate() {
    let pool = default_pool();
    for seed in 0..500 {
        let mut s = new_game(&pool, seed);
        let mut agent = RandomAgent::new(seed);
        let mut steps = 0;
        while s.waiting.player().is_some() {
            let action = agent.choose(&s);
            act(&mut s, action);
            steps += 1;
            assert!(steps < 1_000, "seed {seed}: game does not end");
        }
        assert!(s.winner().is_some());
    }
}

#[test]
fn only_the_deciding_player_can_act() {
    let mut s = scenario();
    s.start();
    let card = s.players[0].hand[0];
    assert!(mindbreake::apply_as(&mut s, 1, GameAction::Play(card)).is_err());
    assert!(mindbreake::apply_as(&mut s, 0, GameAction::Play(card)).is_ok());
}

#[test]
fn view_hides_the_opponents_hand_and_their_actions() {
    let mut s = scenario();
    s.start();
    let mine = mindbreake::view_for(&s, 0);
    assert!(mine.you.hand.is_some() && mine.opponent.hand.is_none());
    assert_eq!(mine.opponent.hand_count, 1);
    assert!(!mine.legal_actions.is_empty());
    assert!(mindbreake::view_for(&s, 1).legal_actions.is_empty());
}

#[test]
fn actions_use_a_stable_json_shape() {
    let json = serde_json::to_string(&GameAction::Play(CardId(3))).unwrap();
    assert_eq!(json, r#"{"type":"Play","card":3}"#);
    let back: GameAction = serde_json::from_str(r#"{"type":"UseMindbug"}"#).unwrap();
    assert_eq!(back, GameAction::UseMindbug);
}
