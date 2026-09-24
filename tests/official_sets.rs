//! Smoke test of the official sets (personal use: only built when the
//! git-ignored catalog is present): random games must never panic, and must
//! nearly always end.
#![cfg(official_cards)]

use mindbreake::ai::{Agent, RandomAgent};
use mindbreake::cards::pool_of_sets;
use mindbreake::new_game;

fn play_random_games(sets: &[&str], games: u64) {
    let pool = pool_of_sets(sets).expect("known sets");
    let mut unfinished = 0;
    for seed in 0..games {
        let mut state = new_game(&pool, seed);
        let mut agents = [RandomAgent::new(seed), RandomAgent::new(seed + 1_000)];
        let mut steps = 0;
        while let Some(player) = state.waiting.player() {
            let action = agents[player].choose(&state);
            mindbreake::apply(&mut state, action)
                .unwrap_or_else(|e| panic!("{sets:?} #{seed}: {e}"));
            steps += 1;
            if steps > 5_000 {
                unfinished += 1;
                break;
            }
        }
    }
    assert!(
        unfinished <= games / 20,
        "{sets:?}: {unfinished}/{games} games did not finish"
    );
}

#[test]
fn every_set_plays_random_games_without_panicking() {
    for set in [
        "New Servants",
        "Beyond Evolution",
        "Promo 2022",
        "Promo 2023",
        "Promo 2024",
    ] {
        play_random_games(&["First Contact", set], 150);
    }
    play_random_games(&["New Servants", "Beyond Evolution"], 150);
}

#[test]
fn pools_are_built_from_the_chosen_sets() {
    use mindbreake::cards::{pool_for, selectable_sets, MIN_POOL};
    let names = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<_>>();

    assert_eq!(pool_for(&[]).unwrap().len(), 48, "default: First Contact");
    assert!(
        pool_for(&names(&["Promo 2022"])).is_err(),
        "10 cards are too few"
    );
    assert!(pool_for(&names(&["Nope"])).is_err());
    let both = pool_for(&names(&["First Contact", "New Servants"])).unwrap();
    assert_eq!(both.len(), 48 + 24);
    assert!(both.len() >= MIN_POOL);

    let offered: Vec<_> = selectable_sets().into_iter().map(|s| s.name).collect();
    assert!(offered.contains(&"Beyond Evolution"));
    assert!(!offered.contains(&"Evolutions") && !offered.contains(&"Examples"));
}
