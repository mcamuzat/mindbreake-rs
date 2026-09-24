//! Derived values: current power, current keywords, conditions, blocking.
//! Everything that depends on static abilities goes through here.

use crate::state::GameState;
use crate::types::{
    Ability, Affected, CardDef, CardId, Condition, CreatureFilter, Effect, Keyword, KeywordFilter,
    Modification, PlayerId, Quantity, Rank, Restriction, Side, StaticAbility, Trigger, TurnMod,
};

impl GameState {
    pub fn quantity(&self, quantity: Quantity, controller: PlayerId) -> i32 {
        match quantity {
            Quantity::Fixed(n) => n,
            Quantity::Life(p) => self.players[p.resolve(controller)].life,
            Quantity::CreaturesControlled(p) => {
                self.players[p.resolve(controller)].board.len() as i32
            }
            Quantity::Mindbugs(p) => self.players[p.resolve(controller)].mindbugs as i32,
            Quantity::Hand(p) => self.players[p.resolve(controller)].hand.len() as i32,
            Quantity::CreatureLead => {
                self.players[controller].board.len() as i32
                    - self.players[1 - controller].board.len() as i32
            }
        }
    }

    pub fn condition_holds(
        &self,
        condition: Condition,
        controller: PlayerId,
        source: CardId,
    ) -> bool {
        match condition {
            Condition::YourTurn => self.active == controller,
            Condition::SourceDefeatedEnemyThisTurn => self.kills_this_turn.contains(&source),
            Condition::Compare {
                left,
                comparator,
                right,
            } => comparator.eval(
                self.quantity(left, controller),
                self.quantity(right, controller),
            ),
        }
    }

    /// Every static ability in play, with its source and the source's controller.
    fn statics_in_play(
        &self,
    ) -> impl Iterator<Item = (CardId, PlayerId, &'static StaticAbility)> + '_ {
        (0..2).flat_map(move |p| {
            self.players[p].board.iter().flat_map(move |&source| {
                self.card(source)
                    .def
                    .statics
                    .iter()
                    .map(move |s| (source, p, s))
            })
        })
    }

    /// Does `affected` (relative to `source`) cover `target`? `target_power`
    /// is only read for `OtherAllies { max_power }`.
    fn affects(
        &self,
        affected: Affected,
        source: CardId,
        source_controller: PlayerId,
        target: CardId,
        target_power: impl FnOnce() -> i32,
    ) -> bool {
        match affected {
            Affected::This => source == target,
            Affected::AllAllies => self.controller(target) == Some(source_controller),
            Affected::Enemies => self.controller(target) == Some(1 - source_controller),
            Affected::OtherAllies { max_power } => {
                source != target
                    && self.controller(target) == Some(source_controller)
                    && max_power.is_none_or(|max| target_power() <= max)
            }
        }
    }

    /// Current power: printed power plus active power modifications.
    ///
    /// A power bonus limited to "creatures with power N or less" compares
    /// against printed power, so that power never depends on itself.
    pub fn power(&self, id: CardId) -> i32 {
        let base = self.card(id).def.power;
        let Some(owner) = self.controller(id) else {
            return base;
        };
        let bonus: i32 = self
            .statics_in_play()
            .filter_map(|(source, controller, s)| {
                let StaticAbility::Modify {
                    affected,
                    condition,
                    modification,
                } = *s
                else {
                    return None;
                };
                let applies = self.affects(affected, source, controller, id, || base)
                    && condition.is_none_or(|c| self.condition_holds(c, controller, source));
                match modification {
                    Modification::Power(n) if applies => Some(n),
                    Modification::PowerPerOtherAlly(n) if applies => {
                        Some(n * (self.players[owner].board.len() as i32 - 1))
                    }
                    _ => None,
                }
            })
            .sum();
        let turn: i32 = self
            .card(id)
            .turn_mods
            .iter()
            .map(|m| match m {
                TurnMod::Power(n) => *n,
                _ => 0,
            })
            .sum();
        base + bonus + turn
    }

    pub fn has_keyword(&self, id: CardId, keyword: Keyword) -> bool {
        self.has_keyword_inner(id, keyword, true)
    }

    /// `allow_copy: false` ignores "has X while an enemy creature does", so
    /// that two copiers facing each other don't grant each other anything.
    fn has_keyword_inner(&self, id: CardId, keyword: Keyword, allow_copy: bool) -> bool {
        if self.card(id).def.keywords.contains(&keyword) {
            return true;
        }
        let Some(target_controller) = self.controller(id) else {
            return false;
        };
        if self.card(id).turn_mods.contains(&TurnMod::Keyword(keyword)) {
            return true;
        }
        self.statics_in_play().any(|(source, controller, s)| {
            let StaticAbility::Modify {
                affected,
                condition,
                modification,
            } = *s
            else {
                return false;
            };
            let applies = self.affects(affected, source, controller, id, || self.power(id))
                && condition.is_none_or(|c| self.condition_holds(c, controller, source));
            applies
                && match modification {
                    Modification::Keywords(list) => list.contains(&keyword),
                    Modification::CopyEnemyKeywords(list) => {
                        allow_copy
                            && list.contains(&keyword)
                            && self.players[1 - target_controller]
                                .board
                                .iter()
                                .any(|&enemy| self.has_keyword_inner(enemy, keyword, false))
                    }
                    _ => false,
                }
        })
    }

    pub fn keywords(&self, id: CardId) -> Vec<Keyword> {
        use Keyword::*;
        [Frenzy, Hunter, Poisonous, Sneaky, Tough]
            .into_iter()
            .filter(|&k| self.has_keyword(id, k))
            .collect()
    }

    pub fn has_any_keyword(&self, id: CardId) -> bool {
        !self.keywords(id).is_empty()
    }

    /// The abilities of `id` for `trigger`: printed ones, and granted ones.
    pub fn abilities_for(&self, id: CardId, trigger: Trigger) -> Vec<Effect> {
        let printed = self.card(id).def.abilities_for(trigger).copied();
        let granted = self
            .statics_in_play()
            .filter_map(|(source, controller, s)| {
                let StaticAbility::Modify {
                    affected,
                    condition,
                    modification: Modification::Ability(&Ability { trigger: t, effect }),
                } = *s
                else {
                    return None;
                };
                (t == trigger
                    && self.affects(affected, source, controller, id, || self.power(id))
                    && condition.is_none_or(|c| self.condition_holds(c, controller, source)))
                .then_some(effect)
            });
        printed.chain(granted).collect()
    }

    // ---- Filters -------------------------------------------------------

    fn in_side(&self, side: Side, source: CardId, controller: PlayerId, id: CardId) -> bool {
        let Some(owner) = self.controller(id) else {
            return false;
        };
        match side {
            Side::Ally => owner == controller,
            Side::Enemy => owner != controller,
            Side::Any => true,
            Side::This => id == source,
        }
    }

    /// Conditions on the card itself (power, keywords, abilities), whatever
    /// its zone: used both for creatures and for cards in a discard pile.
    pub fn attributes_match(&self, filter: CreatureFilter, id: CardId) -> bool {
        let power = self.power(id);
        filter.max_power.is_none_or(|max| power <= max)
            && filter.min_power.is_none_or(|min| power >= min)
            && match filter.keyword {
                None => true,
                Some(KeywordFilter::Any) => self.has_any_keyword(id),
                Some(KeywordFilter::Has(k)) => self.has_keyword(id, k),
            }
            && filter
                .trigger
                .is_none_or(|t| self.card(id).def.abilities_for(t).next().is_some())
            && (!filter.blocked_this_turn || self.blocked_this_turn.contains(&id))
    }

    /// Is `id` a creature in play matching `filter`, for an ability of
    /// `source` controlled by `controller`?
    pub fn matches_filter(
        &self,
        filter: CreatureFilter,
        source: CardId,
        controller: PlayerId,
        id: CardId,
    ) -> bool {
        if !self.in_side(filter.side, source, controller, id)
            || (filter.exclude_source && id == source)
        {
            return false;
        }
        if let Some(rank) = filter.rank {
            let powers = (0..2)
                .flat_map(|p| self.players[p].board.iter().copied())
                .filter(|&c| self.in_side(filter.side, source, controller, c))
                .map(|c| self.power(c));
            let power = self.power(id);
            let extreme = match rank {
                Rank::Highest => powers.max(),
                Rank::Lowest => powers.min(),
            };
            if extreme != Some(power) {
                return false;
            }
        }
        self.attributes_match(filter, id)
    }

    pub fn matching_creatures(
        &self,
        filter: CreatureFilter,
        source: CardId,
        controller: PlayerId,
    ) -> Vec<CardId> {
        (0..2)
            .flat_map(|p| self.players[p].board.iter().copied())
            .filter(|&c| self.matches_filter(filter, source, controller, c))
            .collect()
    }

    // ---- Combat restrictions -------------------------------------------

    fn restricted(&self, restriction: Restriction, id: CardId) -> bool {
        self.statics_in_play().any(|(source, controller, s)| {
            matches!(*s, StaticAbility::Restrict { restriction: r, filter }
                if r == restriction && self.matches_filter(filter, source, controller, id))
        })
    }

    pub fn can_attack(&self, id: CardId) -> bool {
        !self.restricted(Restriction::CantAttack, id)
    }

    /// Can `id` block at all (a Hunter can only choose creatures that can)?
    pub fn may_block(&self, id: CardId) -> bool {
        !self.restricted(Restriction::CantBlock, id)
            && !self.card(id).turn_mods.contains(&TurnMod::CantBlock)
    }

    /// Normal blocking: Sneaky, and "cannot be blocked by…" restrictions.
    /// (A Hunter ignores these: it chooses its blocker directly.)
    pub fn can_block(&self, attacker: CardId, blocker: CardId) -> bool {
        if !self.may_block(blocker) {
            return false;
        }
        if self.has_keyword(attacker, Keyword::Sneaky)
            && !self.has_keyword(blocker, Keyword::Sneaky)
        {
            return false;
        }
        let Some(attacking_player) = self.controller(attacker) else {
            return false;
        };
        !self
            .statics_in_play()
            .any(|(source, controller, s)| match *s {
                StaticAbility::CantBeBlockedBy {
                    attackers,
                    blockers,
                } => {
                    controller == attacking_player
                        && self.affects(attackers, source, controller, attacker, || {
                            self.power(attacker)
                        })
                        && self.attributes_match(blockers, blocker)
                }
                _ => false,
            })
    }

    /// The creatures a Hunter has to pick as blocker, if some enemy creature
    /// demands to be hunted.
    pub fn must_be_hunted(&self, attacker: CardId) -> Vec<CardId> {
        let Some(attacking_player) = self.controller(attacker) else {
            return Vec::new();
        };
        self.statics_in_play()
            .filter(|&(_, controller, s)| {
                controller != attacking_player && matches!(s, StaticAbility::MustBeHunted)
            })
            .map(|(source, _, _)| source)
            .collect()
    }

    /// Does an enemy of `player` demand to be hunted?
    pub fn demands_hunting(&self, player: PlayerId) -> bool {
        self.statics_in_play().any(|(_, controller, s)| {
            controller != player && matches!(s, StaticAbility::MustBeHunted)
        })
    }

    /// Reverse combat: the higher power is defeated.
    pub fn reverses_combat(&self, id: CardId) -> bool {
        self.card(id)
            .def
            .statics
            .contains(&StaticAbility::ReverseCombat)
    }

    // ---- Player-level rules --------------------------------------------

    pub fn play_effects_prevented(&self, player: PlayerId) -> bool {
        self.statics_in_play().any(|(_, controller, s)| {
            matches!(*s, StaticAbility::PreventPlayEffects { player: p } if p.resolve(controller) == player)
        })
    }

    pub fn mindbugs_prevented(&self) -> bool {
        self.statics_in_play()
            .any(|(_, _, s)| matches!(s, StaticAbility::PreventMindbugs))
    }

    /// Life `player` first has to lose to use a Mindbug.
    pub fn mindbug_tax(&self, player: PlayerId) -> i32 {
        self.statics_in_play()
            .filter_map(|(_, controller, s)| match *s {
                StaticAbility::MindbugTax { life } if controller != player => Some(life),
                _ => None,
            })
            .sum()
    }

    pub fn cannot_lose_life(&self, player: PlayerId) -> bool {
        self.statics_in_play().any(|(_, controller, s)| {
            matches!(*s, StaticAbility::CantLoseLife { player: p } if p.resolve(controller) == player)
        })
    }

    pub fn hand_gain_prevented(&self, player: PlayerId) -> bool {
        self.statics_in_play().any(|(_, controller, s)| {
            matches!(*s, StaticAbility::PreventHandGain { player: p } if p.resolve(controller) == player)
        })
    }

    pub fn cannot_play_from_hand(&self, player: PlayerId, card: CardId) -> bool {
        self.statics_in_play().any(|(_, controller, s)| {
            matches!(*s, StaticAbility::CantPlayFromHand { player: p, filter }
                if p.resolve(controller) == player && self.attributes_match(filter, card))
        })
    }

    /// The card this one evolves into instead of being defeated.
    pub fn evolves_instead_of_defeat(&self, id: CardId) -> Option<&'static CardDef> {
        self.card(id).def.statics.iter().find_map(|s| match *s {
            StaticAbility::EvolveInsteadOfDefeat { into } => Some(into),
            _ => None,
        })
    }
}
