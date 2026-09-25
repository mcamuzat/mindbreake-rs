use mindbreake::balance::self_play;
use mindbreake::cards::default_pool;

#[test]
fn self_play_counts_are_consistent_and_do_not_depend_on_threads() {
    let pool = default_pool();
    let one = self_play(&pool, 6, 20, 1);
    let three = self_play(&pool, 6, 20, 3);
    assert_eq!(one, three);
    assert!(one.iter().map(|s| s.played).sum::<u32>() > 0);
    for s in &one {
        assert!(s.wins <= s.played, "{s:?}");
        assert!(s.offered <= s.played, "{s:?}");
        assert!(s.stolen <= s.offered, "{s:?}");
    }
}
