//! CLI:
//!   mindbreake sim [games] [playouts]   Monte Carlo vs random
//!   mindbreake arena [games] [iterations] [playouts]   ISMCTS vs Monte Carlo
//!   mindbreake balance [games] [iterations]   per-card win rates, ISMCTS self-play
//!   mindbreake play [seed]              you (player 0) vs ISMCTS
//!   mindbreake cards                    lists the pool with its generated rules text
//!
//! `--sets "First Contact,New Servants"` (any position) chooses the sets the
//! decks are dealt from; `--sets list` shows their names.

use std::io::{self, BufRead, Write};

use mindbreake::ai::{Agent, IsmctsAgent, MonteCarloAgent, RandomAgent};
use mindbreake::cards::{default_pool, pool_of_sets, sets};
use mindbreake::{apply, legal_actions, new_game, GameAction, GameState, WaitingFor};

/// The names given with `--sets` (removing the flag from `args`), if any.
fn chosen_sets(args: &mut Vec<String>) -> Option<Vec<String>> {
    let i = args.iter().position(|a| a == "--sets")?;
    let names = args.get(i + 1).cloned().unwrap_or_default();
    args.drain(i..(i + 2).min(args.len()));
    if names == "list" {
        for (name, set) in sets() {
            let copies: usize = set.iter().map(|(n, _)| *n as usize).sum();
            println!("{name} ({} cards, {copies} copies)", set.len());
        }
        std::process::exit(0);
    }
    Some(names.split(',').map(|n| n.trim().to_string()).collect())
}

fn pool_from(names: &Option<Vec<String>>) -> Vec<&'static mindbreake::types::CardDef> {
    let Some(names) = names else {
        return default_pool();
    };
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    pool_of_sets(&refs).unwrap_or_else(|| {
        eprintln!("unknown set in {names:?} (try --sets list)");
        std::process::exit(1)
    })
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let names = chosen_sets(&mut args);
    let pool = pool_from(&names);
    let arg = |i: usize, default: u64| args.get(i).and_then(|s| s.parse().ok()).unwrap_or(default);
    match args.first().map(String::as_str) {
        Some("play") => play(&pool, arg(1, rand::random())),
        Some("cards") => list_cards(&names),
        Some("sim") | None => {
            let (games, playouts) = (arg(1, 200), arg(2, 30) as usize);
            let wins = duel(&pool, games, |seed| {
                (MonteCarloAgent::new(seed, playouts), RandomAgent::new(seed))
            });
            report(
                &format!("Monte Carlo ({playouts} playouts/action) vs random"),
                wins,
                games,
            );
        }
        Some("arena") => {
            let (games, iterations, playouts) =
                (arg(1, 100), arg(2, 2000) as usize, arg(3, 150) as usize);
            let wins = duel(&pool, games, |seed| {
                (
                    IsmctsAgent::new(seed, iterations),
                    MonteCarloAgent::new(seed, playouts),
                )
            });
            report(
                &format!(
                    "ISMCTS ({iterations} iterations) vs Monte Carlo ({playouts} playouts/action)"
                ),
                wins,
                games,
            );
        }
        Some("balance") => balance(&pool, &names, arg(1, 2000), arg(2, 300) as usize),
        Some(other) => eprintln!("unknown command: {other} (sim | arena | balance | play | cards)"),
    }
}

fn list_cards(names: &Option<Vec<String>>) {
    let catalog = sets();
    let names = names
        .clone()
        .unwrap_or_else(|| vec!["First Contact".to_string()]);
    for name in names {
        let Some((_, set)) = catalog.iter().find(|(n, _)| n.eq_ignore_ascii_case(&name)) else {
            eprintln!("unknown set {name} (try --sets list)");
            std::process::exit(1)
        };
        for (copies, def) in set.iter() {
            let keywords = if def.keywords.is_empty() {
                String::new()
            } else {
                format!(" {:?}", def.keywords)
            };
            println!("{} x{copies} [{}]{keywords}", def.name, def.power);
            for line in mindbreake::view::rules_text(def) {
                println!("    {line}");
            }
        }
    }
}

/// Plays `games` games between the two agents built by `agents(seed)` and
/// returns how many the first one won. Seats alternate so the starting player
/// doesn't skew the result.
fn duel<A: Agent, B: Agent>(
    pool: &[&'static mindbreake::types::CardDef],
    games: u64,
    agents: impl Fn(u64) -> (A, B),
) -> u64 {
    let mut wins = 0;
    for seed in 0..games {
        let mut state = new_game(pool, seed);
        let (mut first, mut second) = agents(seed);
        let first_seat = (seed % 2) as usize;
        while let Some(player) = state.waiting.player() {
            let action = if player == first_seat {
                first.choose(&state)
            } else {
                second.choose(&state)
            };
            apply(&mut state, action).expect("the agent chose a legal action");
        }
        if state.winner() == Some(first_seat) {
            wins += 1;
        }
    }
    wins
}

fn report(matchup: &str, wins: u64, games: u64) {
    println!(
        "{matchup}: {wins}/{games} wins ({:.1}%)",
        100.0 * wins as f64 / games as f64
    );
}

/// Per-card win rates from self-play. Cards outside the Examples set are the
/// reference: an example card outside their 10th–90th percentile is flagged.
fn balance(
    pool: &[&'static mindbreake::types::CardDef],
    names: &Option<Vec<String>>,
    games: u64,
    iterations: usize,
) {
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let sets_label = names
        .as_ref()
        .map_or("default pool".to_string(), |n| n.join(", "));
    eprintln!("{games} games, ISMCTS {iterations} iterations, {threads} threads, {sets_label}…");
    let mut stats = mindbreake::balance::self_play(pool, games, iterations, threads);
    stats.sort_by(|a, b| b.win_rate().total_cmp(&a.win_rate()));

    let catalog = sets();
    let examples: Vec<&str> = catalog
        .iter()
        .find(|(n, _)| *n == "Examples")
        .map(|(_, set)| set.iter().map(|(_, def)| def.name).collect())
        .unwrap_or_default();
    let is_example = |name: &str| examples.contains(&name);

    // Enough plays for a rate to mean something (margin around ±10%).
    const MIN_PLAYS: u32 = 100;
    let mut reference: Vec<f64> = stats
        .iter()
        .filter(|s| !is_example(s.name) && s.played >= MIN_PLAYS)
        .map(|s| s.win_rate())
        .collect();
    reference.sort_by(f64::total_cmp);
    let band = (reference.len() >= 5).then(|| {
        let at = |q: f64| reference[((reference.len() - 1) as f64 * q).round() as usize];
        (at(0.1), at(0.9))
    });

    println!(
        "{:<28} {:>5} {:>6} {:>12} {:>9}",
        "card", "power", "played", "win rate", "stolen"
    );
    for s in &stats {
        let flag = match band {
            Some((low, high)) if is_example(s.name) && s.played >= MIN_PLAYS => {
                if s.win_rate() > high {
                    "  ▲ above the official cards"
                } else if s.win_rate() < low {
                    "  ▼ below the official cards"
                } else {
                    ""
                }
            }
            _ if s.played < MIN_PLAYS => "  (too few plays)",
            _ => "",
        };
        println!(
            "{:<28} {:>5} {:>6} {:>5.1}% ±{:>4.1} {:>8.1}%{}{flag}",
            s.name,
            s.power,
            s.played,
            100.0 * s.win_rate(),
            100.0 * s.margin(),
            100.0 * s.steal_rate(),
            if is_example(s.name) { " *" } else { "" },
        );
    }
    match band {
        Some((low, high)) => println!(
            "\n* example card. Official cards (10th–90th percentile): {:.1}% – {:.1}%.",
            100.0 * low,
            100.0 * high
        ),
        None => println!(
            "\nNo reference band: add an official set (--sets \"Examples,First Contact\")."
        ),
    }
}

fn play(pool: &[&'static mindbreake::types::CardDef], seed: u64) {
    let mut state = new_game(pool, seed);
    let mut ai = IsmctsAgent::new(seed, 2000);
    let mut shown_log = 0;
    println!("Game #{seed}. You are player 0.");

    while let Some(player) = state.waiting.player() {
        for line in &state.log[shown_log..] {
            println!("  · {line}");
        }
        shown_log = state.log.len();

        let action = if player == 0 {
            render(&state);
            ask(&state)
        } else {
            ai.choose(&state)
        };
        apply(&mut state, action).expect("legal action");
    }
    for line in &state.log[shown_log..] {
        println!("  · {line}");
    }
    match state.winner() {
        Some(0) => println!("\nYou win!"),
        _ => println!("\nThe AI wins."),
    }
}

fn describe_card(state: &GameState, id: mindbreake::types::CardId) -> String {
    let card = state.card(id);
    let mut s = format!("{} [{}]", card.def.name, state.power(id));
    let keywords = state.keywords(id);
    if !keywords.is_empty() {
        s += &format!(" {keywords:?}");
    }
    if card.exhausted {
        s += " (exhausted)";
    }
    s
}

fn render(state: &GameState) {
    let me = &state.players[0];
    let opp = &state.players[1];
    let list = |ids: &[mindbreake::types::CardId]| {
        ids.iter()
            .map(|&c| describe_card(state, c))
            .collect::<Vec<_>>()
            .join(", ")
    };
    println!("\n──────────────────────────────────────────");
    println!(
        "AI  : {} life, {} Mindbug(s), {} card(s) in hand, draw pile {}",
        opp.life,
        opp.mindbugs,
        opp.hand.len(),
        opp.deck.len()
    );
    println!("      board: {}", list(&opp.board));
    println!(
        "You : {} life, {} Mindbug(s), draw pile {}",
        me.life,
        me.mindbugs,
        me.deck.len()
    );
    println!("      board: {}", list(&me.board));
    println!("      hand : {}", list(&me.hand));
    let prompt = match &state.waiting {
        WaitingFor::Action { .. } => "Your turn: play a creature or attack.".to_string(),
        WaitingFor::Mindbug { creature, .. } => {
            format!(
                "The AI plays {}. Use a Mindbug?",
                describe_card(state, *creature)
            )
        }
        WaitingFor::Block { attacker, .. } => {
            format!(
                "{} attacks. Choose the blocker.",
                describe_card(state, *attacker)
            )
        }
        WaitingFor::FrenzyAttack { attacker, .. } => {
            format!("{} can attack again (Frenzy).", state.name(*attacker))
        }
        WaitingFor::ChooseCards {
            source,
            purpose,
            remaining,
            ..
        } => format!(
            "{}: choose a card ({purpose:?}, {remaining} left).",
            state.name(*source)
        ),
        WaitingFor::Confirm { source, kind, .. } => {
            format!("{}: {kind:?}? (accept / decline)", state.name(*source))
        }
        WaitingFor::GameOver { .. } => String::new(),
    };
    println!("\n{prompt}");
}

fn label(state: &GameState, action: GameAction) -> String {
    match action {
        GameAction::Play(c) => format!("Play {}", describe_card(state, c)),
        GameAction::Attack(c) => format!("Attack with {}", describe_card(state, c)),
        GameAction::Activate(c) => format!("Activate {}", describe_card(state, c)),
        GameAction::Accept => "Yes".into(),
        GameAction::Decline => "No".into(),
        GameAction::UseMindbug => "Use a Mindbug".into(),
        GameAction::PassMindbug => "Let it be".into(),
        GameAction::Block(c) => format!("Block with {}", describe_card(state, c)),
        GameAction::NoBlock => "Don't block / let the defender choose".into(),
        GameAction::Choose(c) => format!("Choose {}", describe_card(state, c)),
        GameAction::Done => "Stop choosing".into(),
        GameAction::FrenzyAttack => "Attack again".into(),
        GameAction::EndTurn => "End the turn".into(),
    }
}

fn ask(state: &GameState) -> GameAction {
    let actions = legal_actions(state);
    for (i, &a) in actions.iter().enumerate() {
        println!("  {}. {}", i + 1, label(state, a));
    }
    let stdin = io::stdin();
    loop {
        print!("> ");
        io::stdout().flush().ok();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
            std::process::exit(0);
        }
        match line.trim().parse::<usize>() {
            Ok(n) if (1..=actions.len()).contains(&n) => return actions[n - 1],
            _ => println!("Number between 1 and {}.", actions.len()),
        }
    }
}
