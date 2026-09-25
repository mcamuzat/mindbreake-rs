use mindbreake::ai::{Agent, IsmctsAgent, RandomAgent};
use mindbreake::cards::{by_name, default_pool};
use mindbreake::{apply, new_game, GameAction, GameState, WaitingFor, Zone};

/// Player `active` starts. Each player gets a card in hand so nobody loses
/// for being unable to act.
fn scenario(active: usize) -> GameState {
    let mut s = GameState::empty(active);
    s.add_card(0, Zone::Hand, by_name("Moss Golem"));
    s.add_card(1, Zone::Hand, by_name("Moss Golem"));
    s
}

#[test]
fn ismcts_takes_the_winning_attack() {
    let mut s = scenario(0);
    let scout = s.add_card(0, Zone::Board, by_name("Ferret Scout"));
    s.players[1].life = 1;
    s.start();
    let action = IsmctsAgent::new(1, 300).choose(&s);
    assert_eq!(action, GameAction::Attack(scout));
}

#[test]
fn ismcts_blocks_when_the_attack_would_be_lethal() {
    let mut s = scenario(1);
    let golem = s.add_card(1, Zone::Board, by_name("Moss Golem"));
    let newt = s.add_card(0, Zone::Board, by_name("Venom Newt"));
    s.players[0].life = 1;
    s.start();
    apply(&mut s, GameAction::Attack(golem)).unwrap();
    assert!(matches!(s.waiting, WaitingFor::Block { chooser: 0, .. }));
    let action = IsmctsAgent::new(1, 300).choose(&s);
    assert_eq!(action, GameAction::Block(newt));
}

#[test]
fn ismcts_only_chooses_legal_actions_through_a_whole_game() {
    for seed in 0..3 {
        let mut s = new_game(&default_pool(), seed);
        let mut ismcts = IsmctsAgent::new(seed, 50);
        let mut random = RandomAgent::new(seed);
        while let Some(player) = s.waiting.player() {
            let action = if player == 0 {
                ismcts.choose(&s)
            } else {
                random.choose(&s)
            };
            apply(&mut s, action).unwrap_or_else(|e| panic!("{e}"));
        }
    }
}
