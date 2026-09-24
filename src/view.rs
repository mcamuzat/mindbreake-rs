//! What one player is allowed to see. This is the only state a UI receives:
//! hidden information is removed here, and derived values (current power and
//! keywords, rules text, legal actions) are computed here, never on the
//! client side.

use serde::Serialize;

use crate::engine::legal_actions;
use crate::state::{GameAction, GameState, WaitingFor};
use crate::types::{
    Affected, CardDef, CardId, Comparator, Condition, Count, CreatureAction, CreatureFilter,
    Effect, Keyword, KeywordFilter, Modification, PlayerId, PlayerRef, Quantity, Rank, Restriction,
    Side, StaticAbility, Trigger, TurnMod,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameView {
    pub viewer: PlayerId,
    pub active: PlayerId,
    /// The player who must decide, or `None` when the game is over.
    pub to_act: Option<PlayerId>,
    pub winner: Option<PlayerId>,
    /// Choice candidates are only visible to the player choosing.
    pub waiting: WaitingFor,
    /// Legal actions for the viewer (empty when it is not their decision).
    pub legal_actions: Vec<GameAction>,
    pub you: PlayerView,
    pub opponent: PlayerView,
    pub log: Vec<String>,
    /// What each keyword does, for the help panel and tooltips.
    pub keyword_help: Vec<KeywordHelp>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KeywordHelp {
    pub keyword: Keyword,
    pub text: &'static str,
}

/// Rules text of every keyword (see RULES.md).
pub fn keyword_help() -> Vec<KeywordHelp> {
    use Keyword::*;
    [
        (
            Frenzy,
            "If it survives its first attack of the turn, it can attack a second time.",
        ),
        (
            Hunter,
            "When it attacks, its controller chooses the enemy creature that has to block it, even one that could not block it normally.",
        ),
        (
            Poisonous,
            "In combat, it always defeats the enemy creature, whatever its power.",
        ),
        (Sneaky, "It can only be blocked by Sneaky creatures."),
        (
            Tough,
            "The first time it would be defeated, it is exhausted instead.",
        ),
    ]
    .into_iter()
    .map(|(keyword, text)| KeywordHelp { keyword, text })
    .collect()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerView {
    pub id: PlayerId,
    pub life: i32,
    pub mindbugs: u8,
    /// `None` for the opponent: only the size of their hand is visible.
    pub hand: Option<Vec<CardView>>,
    pub hand_count: usize,
    pub deck_count: usize,
    pub board: Vec<CardView>,
    pub discard: Vec<CardView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardView {
    pub id: CardId,
    pub name: &'static str,
    /// Current power (with bonuses) and printed power.
    pub power: i32,
    pub base_power: i32,
    /// Current keywords (printed, granted or copied).
    pub keywords: Vec<Keyword>,
    /// The subset of `keywords` not printed on the card (granted or copied).
    pub granted_keywords: Vec<Keyword>,
    pub rules_text: Vec<String>,
    pub exhausted: bool,
}

pub fn view_for(state: &GameState, viewer: PlayerId) -> GameView {
    let to_act = state.waiting.player();
    let mut waiting = state.waiting.clone();
    if to_act != Some(viewer) {
        if let WaitingFor::ChooseCards { candidates, .. } = &mut waiting {
            candidates.clear();
        }
    }
    GameView {
        viewer,
        active: state.active,
        to_act,
        winner: state.winner(),
        waiting,
        legal_actions: if to_act == Some(viewer) {
            legal_actions(state)
        } else {
            Vec::new()
        },
        you: player_view(state, viewer, true),
        opponent: player_view(state, 1 - viewer, false),
        log: state.log.clone(),
        keyword_help: keyword_help(),
    }
}

fn player_view(state: &GameState, id: PlayerId, reveal_hand: bool) -> PlayerView {
    let p = &state.players[id];
    let cards = |ids: &[CardId]| ids.iter().map(|&c| card_view(state, c)).collect::<Vec<_>>();
    PlayerView {
        id,
        life: p.life,
        mindbugs: p.mindbugs,
        hand: reveal_hand.then(|| cards(&p.hand)),
        hand_count: p.hand.len(),
        deck_count: p.deck.len(),
        board: cards(&p.board),
        discard: cards(&p.discard),
    }
}

fn card_view(state: &GameState, id: CardId) -> CardView {
    let card = state.card(id);
    let keywords = if state.on_board(id) {
        state.keywords(id)
    } else {
        card.def.keywords.to_vec()
    };
    let granted_keywords = keywords
        .iter()
        .copied()
        .filter(|k| !card.def.keywords.contains(k))
        .collect();
    CardView {
        id,
        name: card.def.name,
        power: state.power(id),
        base_power: card.def.power,
        keywords,
        granted_keywords,
        rules_text: rules_text(card.def),
        exhausted: card.exhausted,
    }
}

/// English rules text generated from the typed definition.
pub fn rules_text(def: &CardDef) -> Vec<String> {
    let abilities = def.abilities.iter().map(|a| {
        let trigger = match a.trigger {
            Trigger::Play => "Play",
            Trigger::Attack => "Attack",
            Trigger::Defeated => "Defeated",
            Trigger::Action => "Action",
            Trigger::EndOfTurn => "At the end of turn",
            Trigger::LifeLost => "When you lose life",
        };
        format!("{trigger}: {}.", effect_text(&a.effect))
    });
    let statics = def.statics.iter().map(|s| static_text(s) + ".");
    let discard = def.discard_abilities.iter().map(|a| {
        let when = match a.trigger {
            Trigger::LifeLost => "when you lose 1 or more life points",
            _ => "",
        };
        format!(
            "In your discard pile, {when}: {}.",
            lowercase_first(&effect_text(&a.effect))
        )
    });
    statics.chain(abilities).chain(discard).collect()
}

fn player_text(p: PlayerRef) -> &'static str {
    match p {
        PlayerRef::You => "you",
        PlayerRef::Opponent => "the opponent",
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

fn lowercase_first(s: &str) -> String {
    let mut chars = s.chars();
    chars
        .next()
        .map(|c| c.to_lowercase().chain(chars).collect())
        .unwrap_or_default()
}

/// Verb agreeing with the subject: "you gain" / "the opponent gains".
fn verb(p: PlayerRef, base: &str, third_person: &str) -> String {
    let form = match p {
        PlayerRef::You => base,
        PlayerRef::Opponent => third_person,
    };
    format!("{} {form}", capitalize(player_text(p)))
}

fn have(p: PlayerRef) -> &'static str {
    match p {
        PlayerRef::You => "you have",
        PlayerRef::Opponent => "the opponent has",
    }
}

fn control(p: PlayerRef) -> &'static str {
    match p {
        PlayerRef::You => "you control",
        PlayerRef::Opponent => "the opponent controls",
    }
}

fn quantity_text(q: Quantity) -> String {
    match q {
        Quantity::Fixed(n) => n.to_string(),
        Quantity::Life(p) => format!("{} life total", possessive(p)),
        Quantity::CreaturesControlled(p) => {
            format!("the number of creatures {}", control(p))
        }
        Quantity::Mindbugs(p) => format!("the number of Mindbugs {}", have(p)),
        Quantity::Hand(p) => format!("the number of cards in {} hand", possessive(p)),
        Quantity::CreatureLead => "your creature lead over the opponent".into(),
    }
}

/// "for each …": what one unit of a non-fixed quantity is.
fn per_text(q: Quantity) -> String {
    match q {
        Quantity::Fixed(_) => String::new(),
        Quantity::Life(p) => format!("life point {}", have(p)),
        Quantity::CreaturesControlled(p) => format!("creature {}", control(p)),
        Quantity::Mindbugs(p) => format!("Mindbug {}", have(p)),
        Quantity::Hand(p) => format!("card in {} hand", possessive(p)),
        Quantity::CreatureLead => "creature you lead by".into(),
    }
}

/// "2 life", "1 life for each creature they control".
fn amount_text(q: Quantity, noun: &str) -> String {
    match q {
        Quantity::Fixed(n) => format!("{n} {noun}"),
        other => format!("1 {noun} for each {}", per_text(other)),
    }
}

fn possessive(p: PlayerRef) -> &'static str {
    match p {
        PlayerRef::You => "your",
        PlayerRef::Opponent => "the opponent's",
    }
}

fn their(p: PlayerRef) -> &'static str {
    match p {
        PlayerRef::You => "your",
        PlayerRef::Opponent => "their",
    }
}

fn condition_text(c: Condition) -> String {
    match c {
        Condition::YourTurn => "it is your turn".into(),
        Condition::SourceDefeatedEnemyThisTurn => {
            "this defeated an enemy creature this turn".into()
        }
        Condition::Compare {
            left,
            comparator,
            right,
        } => {
            let cmp = match comparator {
                Comparator::Less => "is less than",
                Comparator::LessOrEqual => "is at most",
                Comparator::Equal if matches!(right, Quantity::Fixed(_)) => "is",
                Comparator::Equal => "is equal to",
                Comparator::GreaterOrEqual => "is at least",
                Comparator::Greater => "is greater than",
            };
            format!("{} {cmp} {}", quantity_text(left), quantity_text(right))
        }
    }
}

fn count_text(count: Count, noun: &str) -> String {
    let plural = if count.max == 1 { "" } else { "s" };
    match (count.up_to, count.max) {
        (true, u8::MAX) => format!("any number of {noun}s"),
        (false, 1) => format!("a {noun}"),
        (false, n) => format!("{n} {noun}{plural}"),
        (true, n) => format!("up to {n} {noun}{plural}"),
    }
}

fn sentence(effect: &Effect) -> String {
    effect_text(effect)
}

fn effect_text(effect: &Effect) -> String {
    match *effect {
        Effect::GainLife { player, amount } => format!(
            "{} {}",
            verb(player, "gain", "gains"),
            amount_text(amount, "life")
        ),
        Effect::LoseLife { player, amount } => format!(
            "{} {}",
            verb(player, "lose", "loses"),
            amount_text(amount, "life")
        ),
        Effect::SetLife { player, value } => format!(
            "{} life total becomes {}",
            capitalize(possessive(player)),
            quantity_text(value)
        ),
        Effect::ChooseCreatures {
            action,
            filter,
            count,
        } => {
            let one = Count::exactly(1);
            let optional = count.up_to && count.max == 1;
            let target = filter_text(filter, Some(if optional { one } else { count }));
            let text = match action {
                CreatureAction::Defeat => format!("Defeat {target}"),
                CreatureAction::TakeControl => format!("Take control of {target}"),
                CreatureAction::ReturnToHand => {
                    format!("Return {target} to its controller's hand")
                }
                CreatureAction::ThisTurn(m) => {
                    format!("Choose {target}. It {} this turn", turn_mod_text(m))
                }
                CreatureAction::MustAttack => {
                    let who = if filter.side == Side::Enemy {
                        "The opponent"
                    } else {
                        "Its controller"
                    };
                    format!("Choose {target}. {who} attacks with it if able")
                }
                CreatureAction::CopyPlayEffect => {
                    format!("Copy the Play effect of {target}")
                }
            };
            match (optional, action) {
                (true, CreatureAction::ThisTurn(_) | CreatureAction::MustAttack) => {
                    format!("You may choose {}", &text["Choose ".len()..])
                }
                (true, _) => format!("You may {}", lowercase_first(&text)),
                _ => text,
            }
        }
        Effect::AllCreatures { action, filter } => match action {
            CreatureAction::ThisTurn(m) => format!(
                "{} {} this turn",
                capitalize(&filter_text(filter, None)),
                turn_mod_text(m)
            ),
            CreatureAction::ReturnToHand => format!(
                "Return {} to {} hand",
                filter_text(filter, None),
                if filter.side == Side::Enemy {
                    "the opponent's"
                } else {
                    "their controllers'"
                }
            ),
            _ => format!("{} {}", action_text(action), filter_text(filter, None)),
        },
        Effect::Discard {
            player,
            amount,
            up_to,
        } => {
            let cards = match amount {
                Quantity::Fixed(n) => count_text(
                    Count {
                        max: n as u8,
                        up_to,
                    },
                    "card",
                ),
                other => format!("a card for each {}", per_text(other)),
            };
            let may = if up_to && matches!(amount, Quantity::Fixed(1)) {
                format!("{} may discard a card", capitalize(player_text(player)))
            } else {
                format!("{} {cards}", verb(player, "discard", "discards"))
            };
            may
        }
        Effect::DiscardZones { player, hand, deck } => {
            let what = match (hand, deck) {
                (true, true) => "hand and draw pile",
                (true, false) => "hand",
                _ => "draw pile",
            };
            format!(
                "{} {} {what}",
                verb(player, "discard", "discards"),
                their(player)
            )
        }
        Effect::StealRandomFromHand { amount } => format!(
            "Steal {} at random from the opponent's hand",
            count_text(Count::exactly(amount), "card")
        ),
        Effect::PlayFromDiscard {
            from,
            filter,
            count,
            play_effects,
        } => {
            let mut s = format!("Play {}", count_text(count, "card"));
            s += &attributes_text(filter);
            s += &format!(" from {} discard pile", possessive(from));
            if !play_effects {
                s += " without activating their Play effects";
            }
            s
        }
        Effect::ReturnFromDiscard { count: None } => {
            "Put your entire discard pile into your hand".into()
        }
        Effect::ReturnFromDiscard { count: Some(count) } => format!(
            "Put {} from your discard pile into your hand",
            count_text(count, "card")
        ),
        Effect::If { condition, then } => {
            format!(
                "If {}, {}",
                condition_text(condition),
                lowercase_first(&effect_text(then))
            )
        }
        Effect::Sequence(effects) => effects.iter().map(sentence).collect::<Vec<_>>().join(". "),
        Effect::Optional(inner) => {
            let text = lowercase_first(&effect_text(inner));
            let text = text.strip_prefix("you ").unwrap_or(&text);
            format!("You may {text}")
        }
        Effect::Then { first, then, per } => {
            let link = if per {
                "For each time you do so,"
            } else {
                "If you do,"
            };
            format!(
                "{}. {link} {}",
                effect_text(first),
                lowercase_first(&effect_text(then))
            )
        }
        Effect::Roll { threshold, then } => format!(
            "Roll a 6-sided die. On {threshold} to 6, {} and then repeat this effect",
            lowercase_first(&effect_text(then))
        ),
        Effect::GiveControl => "The opponent takes control of this".into(),
        Effect::SwapHands => "Swap hands with the opponent".into(),
        Effect::EndGame { winner } => format!("{} the game", verb(winner, "win", "wins")),
        Effect::TakeFromOpponentHand => "The opponent gives you a card from their hand".into(),
        Effect::PlayReceived { optional: true } => "Play it or put it into your hand".into(),
        Effect::PlayReceived { optional: false } => "Play it".into(),
        Effect::Evolve { into } => format!("Evolve to {}", into.name),
        Effect::SetAsideOthers => "Set aside all other creatures".into(),
        Effect::ReturnSetAside => {
            "Return them to play without activating their Play effects".into()
        }
        Effect::PlaySelf => "Play this".into(),
        Effect::UnusedToBottom { count } => {
            format!("Put the top {count} cards of the unused pile on the bottom of your draw pile")
        }
    }
}

fn action_text(action: CreatureAction) -> &'static str {
    match action {
        CreatureAction::Defeat => "Defeat",
        CreatureAction::TakeControl => "Take control of",
        CreatureAction::ReturnToHand => "Return",
        CreatureAction::ThisTurn(_) | CreatureAction::MustAttack => "Choose",
        CreatureAction::CopyPlayEffect => "Copy the Play effect of",
    }
}

fn turn_mod_text(m: TurnMod) -> String {
    match m {
        TurnMod::Power(n) => format!("has {n:+} power"),
        TurnMod::Keyword(k) => format!("has {}", keyword_list(&[k])),
        TurnMod::CantBlock => "cannot block".into(),
        TurnMod::CantBeDefeated => "cannot be defeated".into(),
    }
}

/// " with power 4 or less, with hunter, …": conditions on the card itself.
fn attributes_text(filter: CreatureFilter) -> String {
    let mut parts = Vec::new();
    match (filter.min_power, filter.max_power) {
        (Some(min), Some(max)) => parts.push(format!("power {min} to {max}")),
        (Some(min), None) => parts.push(format!("power {min} or more")),
        (None, Some(max)) => parts.push(format!("power {max} or less")),
        (None, None) => {}
    }
    match filter.keyword {
        Some(KeywordFilter::Any) => parts.push("1 or more keywords".into()),
        Some(KeywordFilter::Has(k)) => parts.push(keyword_list(&[k])),
        None => {}
    }
    let mut s = if parts.is_empty() {
        String::new()
    } else {
        format!(" with {}", parts.join(" and "))
    };
    if let Some(trigger) = filter.trigger {
        let name = match trigger {
            Trigger::Play => "a Play",
            Trigger::Attack => "an Attack",
            Trigger::Defeated => "a Defeated",
            Trigger::Action => "an Action",
            Trigger::EndOfTurn | Trigger::LifeLost => "an",
        };
        s += &format!(" with {name} effect");
    }
    s
}

/// `count: None` means "all".
fn filter_text(filter: CreatureFilter, count: Option<Count>) -> String {
    if filter.side == Side::This {
        return "this creature".into();
    }
    let side = match filter.side {
        Side::Ally => "allied ",
        Side::Enemy => "enemy ",
        _ => "",
    };
    let mut s = match count {
        None => {
            // The source can only be among the matches when allies are included.
            let other = if filter.exclude_source && filter.side != Side::Enemy {
                "other "
            } else {
                ""
            };
            format!("all {other}{side}creatures")
        }
        Some(count) => {
            let other = if filter.exclude_source && filter.side != Side::Enemy {
                "another "
            } else {
                ""
            };
            let noun = format!("{other}{side}creature");
            let text = count_text(count, &noun);
            // "a enemy" → "an enemy"
            match text.strip_prefix("a e") {
                Some(rest) => format!("an e{rest}"),
                None => text.replace("a another", "another"),
            }
        }
    };
    s += &attributes_text(filter);
    match filter.rank {
        Some(Rank::Highest) => s += " with the highest power",
        Some(Rank::Lowest) => s += " with the lowest power",
        None => {}
    }
    if filter.blocked_this_turn {
        s += " that blocked this turn";
    }
    s
}

fn affected_text(affected: Affected) -> String {
    match affected {
        Affected::This => "This creature".into(),
        Affected::AllAllies => "Your creatures".into(),
        Affected::Enemies => "Enemy creatures".into(),
        Affected::OtherAllies { max_power: None } => "Your other creatures".into(),
        Affected::OtherAllies {
            max_power: Some(max),
        } => format!("Your other creatures with power {max} or less"),
    }
}

fn keyword_list(keywords: &[Keyword]) -> String {
    keywords
        .iter()
        .map(|k| format!("{k:?}").to_lowercase())
        .collect::<Vec<_>>()
        .join(", ")
}

fn static_text(s: &StaticAbility) -> String {
    match *s {
        StaticAbility::Modify {
            affected,
            condition,
            modification,
        } => {
            let subject = affected_text(affected);
            let verb = if matches!(affected, Affected::This) {
                "has"
            } else {
                "have"
            };
            let body = match modification {
                Modification::Power(n) => format!("{subject} {verb} {n:+} power"),
                Modification::PowerPerOtherAlly(n) => {
                    format!("{subject} {verb} {n:+} power for each other allied creature")
                }
                Modification::Keywords(list) => format!("{subject} {verb} {}", keyword_list(list)),
                Modification::CopyEnemyKeywords(list) => format!(
                    "{subject} {verb} each of {} while an enemy creature has it",
                    keyword_list(list)
                ),
                Modification::Ability(a) => {
                    let trigger = match a.trigger {
                        Trigger::Play => "Play",
                        Trigger::Attack => "Attack",
                        Trigger::Defeated => "Defeated",
                        Trigger::Action => "Action",
                        Trigger::EndOfTurn => "At the end of turn",
                        Trigger::LifeLost => "When you lose life",
                    };
                    format!("{subject} {verb} \"{trigger}: {}\"", effect_text(&a.effect))
                }
            };
            match condition {
                Some(c) => format!("{body} while {}", condition_text(c)),
                None => body,
            }
        }
        StaticAbility::CantBeBlockedBy {
            attackers,
            blockers,
        } => {
            let subject = match attackers {
                Affected::This => "This creature".to_string(),
                other => affected_text(other),
            };
            format!(
                "{subject} cannot be blocked by creatures{}",
                attributes_text(blockers)
            )
        }
        StaticAbility::Restrict {
            restriction,
            filter,
        } => {
            let subject = filter_text(filter, None);
            let subject = capitalize(subject.strip_prefix("all ").unwrap_or(&subject));
            let what = match restriction {
                Restriction::CantAttack => "attack",
                Restriction::CantBlock => "block",
            };
            format!("{subject} cannot {what}")
        }
        StaticAbility::PreventPlayEffects { player } => {
            format!(
                "{} cannot activate Play effects",
                capitalize(player_text(player))
            )
        }
        StaticAbility::PreventMindbugs => "Players cannot use Mindbugs".into(),
        StaticAbility::MindbugTax { life } => format!(
            "When the opponent uses a Mindbug, they first lose {life} life"
        ),
        StaticAbility::CantLoseLife { player } => {
            format!("{} cannot lose life", capitalize(player_text(player)))
        }
        StaticAbility::ReverseCombat => {
            "When this fights, the creature with the highest power is defeated instead of the lowest".into()
        }
        StaticAbility::MustBeHunted => {
            "The opponent must always attack this with a creature with hunter if able".into()
        }
        StaticAbility::CantPlayFromHand { player, filter } => format!(
            "{} cannot play cards{} from {} hand",
            capitalize(player_text(player)),
            attributes_text(filter),
            their(player)
        ),
        StaticAbility::PreventHandGain { player } => format!(
            "{} cannot put cards into {} hand",
            capitalize(player_text(player)),
            their(player)
        ),
        StaticAbility::EvolveInsteadOfDefeat { into } => {
            format!("If this would be defeated, evolve to {} instead", into.name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::by_name;

    #[test]
    fn rules_text_is_generated_from_the_definition() {
        assert_eq!(
            rules_text(by_name("Tusk Mauler")),
            ["Attack: Defeat an enemy creature with power 2 or less."]
        );
        assert_eq!(
            rules_text(by_name("Quake Tortoise")),
            ["Play: Defeat all other creatures with power 3 or less."]
        );
        assert_eq!(
            rules_text(by_name("Pack Alpha")),
            ["Your other creatures have +1 power."]
        );
        assert_eq!(
            rules_text(by_name("Healing Moth")),
            ["Play: You gain 2 life."]
        );
    }
}
