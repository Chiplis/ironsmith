//! "Whenever [filter] attacks" trigger.

use crate::effect::ChoiceAggregateMetric;
use crate::events::EventKind;
use crate::events::combat::CreatureAttackedEvent;
use crate::filter::Comparison;
use crate::filter::{ObjectFilterExt as _, PlayerFilterExt as _};
use crate::ids::ObjectId;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Trigger that fires when a matching creature attacks.
///
/// Used by cards that care about other creatures attacking.
#[derive(Debug, Clone, PartialEq)]
pub struct AttacksTrigger {
    /// Filter for creatures that trigger this ability.
    pub filter: ObjectFilter,
    /// If true, this trigger fires only once when one or more matching creatures attack.
    pub one_or_more: bool,
    /// Minimum number of total attackers required for this trigger to fire.
    pub min_total_attackers: usize,
    /// Maximum number of total attackers allowed for this trigger to fire.
    pub max_total_attackers: Option<usize>,
    /// Optional comparison over one characteristic summed across every
    /// matching attacker in the declaration.
    pub aggregate_constraint: Option<(ChoiceAggregateMetric, Comparison)>,
}

/// Trigger that fires once when one or more matching players are attacked.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayersAttackedTrigger {
    pub player_filter: PlayerFilter,
}

/// Trigger that fires once when a matching player attacks a matching kind of
/// defender. The target restriction is deliberately typed so a
/// planeswalker-only trigger cannot also match its controller or a Battle.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerAttacksOneOrMoreTrigger {
    pub attacker: PlayerFilter,
    pub target: ironsmith_core::AttackTargetRestriction,
    /// When true, each matching attacked defender owns an independent group
    /// of attackers. When false, all matching defenders share one group for
    /// the attack declaration.
    pub group_by_target: bool,
}

impl PlayerAttacksOneOrMoreTrigger {
    pub fn new(attacker: PlayerFilter, target: ironsmith_core::AttackTargetRestriction) -> Self {
        Self {
            attacker,
            target,
            group_by_target: false,
        }
    }

    pub fn grouped_by_target(
        attacker: PlayerFilter,
        target: ironsmith_core::AttackTargetRestriction,
    ) -> Self {
        Self {
            attacker,
            target,
            group_by_target: true,
        }
    }

    fn player_matches(
        &self,
        filter: &PlayerFilter,
        player: crate::ids::PlayerId,
        ctx: &TriggerContext,
    ) -> bool {
        crate::filter::player_filter_matches_game(filter, player, ctx.game, &ctx.filter_ctx)
    }

    fn target_matches(
        &self,
        target: &crate::combat_state::AttackTarget,
        ctx: &TriggerContext,
    ) -> bool {
        match (&self.target, target) {
            (
                ironsmith_core::AttackTargetRestriction::Player(filter),
                crate::combat_state::AttackTarget::Player(player),
            ) => self.player_matches(filter, *player, ctx),
            (
                ironsmith_core::AttackTargetRestriction::PlaneswalkerControlledBy(filter),
                crate::combat_state::AttackTarget::Planeswalker(planeswalker),
            ) => ctx.game.object(*planeswalker).is_some_and(|planeswalker| {
                self.player_matches(filter, ctx.game.controller_of(planeswalker), ctx)
            }),
            (
                ironsmith_core::AttackTargetRestriction::PlayerOrPlaneswalkerControlledBy(filter),
                crate::combat_state::AttackTarget::Player(player),
            ) => self.player_matches(filter, *player, ctx),
            (
                ironsmith_core::AttackTargetRestriction::PlayerOrPlaneswalkerControlledBy(filter),
                crate::combat_state::AttackTarget::Planeswalker(planeswalker),
            ) => ctx.game.object(*planeswalker).is_some_and(|planeswalker| {
                self.player_matches(filter, ctx.game.controller_of(planeswalker), ctx)
            }),
            _ => false,
        }
    }

    fn attacker_matches(&self, attacker: ObjectId, ctx: &TriggerContext) -> bool {
        ctx.game.object(attacker).is_some_and(|attacker| {
            self.player_matches(&self.attacker, ctx.game.controller_of(attacker), ctx)
        })
    }

    fn is_first_matching_attacker_this_combat(
        &self,
        attacker: ObjectId,
        attack_target: &crate::combat_state::AttackTarget,
        ctx: &TriggerContext,
    ) -> bool {
        let Some(current_attacker) = ctx.game.object(attacker) else {
            return false;
        };
        let current_player = ctx.game.controller_of(current_attacker);
        let Some(combat) = ctx.game.combat.as_ref() else {
            return true;
        };
        for info in &combat.attackers {
            let Some(candidate) = ctx.game.object(info.creature) else {
                continue;
            };
            if ctx.game.controller_of(candidate) == current_player
                && self.attacker_matches(info.creature, ctx)
                && self.target_matches(&info.target, ctx)
                && (!self.group_by_target || &info.target == attack_target)
            {
                return info.creature == attacker;
            }
        }
        true
    }
}

impl PlayersAttackedTrigger {
    pub fn one_or_more(player_filter: PlayerFilter) -> Self {
        Self { player_filter }
    }

    fn target_matches(
        &self,
        target: &crate::combat_state::AttackTarget,
        ctx: &TriggerContext,
    ) -> bool {
        let crate::combat_state::AttackTarget::Player(player) = target else {
            return false;
        };
        self.player_filter.matches_player(*player, &ctx.filter_ctx)
    }

    fn is_first_matching_attacker_this_combat(
        &self,
        attacker: ObjectId,
        attack_target: &crate::combat_state::AttackTarget,
        ctx: &TriggerContext,
    ) -> bool {
        let Some(combat) = ctx.game.combat.as_ref() else {
            return true;
        };
        if !self.target_matches(attack_target, ctx) {
            return false;
        }
        for info in &combat.attackers {
            if self.target_matches(&info.target, ctx) {
                return info.creature == attacker;
            }
        }
        true
    }
}

fn player_attack_subject(filter: &PlayerFilter) -> String {
    match filter {
        PlayerFilter::Opponent => "an opponent".to_string(),
        PlayerFilter::You => "you".to_string(),
        PlayerFilter::Any => "a player".to_string(),
        _ => filter.description(),
    }
}

fn controlled_planeswalker_target(filter: &PlayerFilter) -> String {
    match filter {
        PlayerFilter::You => "one or more planeswalkers you control".to_string(),
        PlayerFilter::Opponent => "one or more planeswalkers an opponent controls".to_string(),
        _ => format!(
            "one or more planeswalkers {} controls",
            filter.description()
        ),
    }
}

fn player_attack_target(restriction: &ironsmith_core::AttackTargetRestriction) -> String {
    match restriction {
        ironsmith_core::AttackTargetRestriction::Player(filter) => player_attack_subject(filter),
        ironsmith_core::AttackTargetRestriction::PlaneswalkerControlledBy(filter) => {
            controlled_planeswalker_target(filter)
        }
        ironsmith_core::AttackTargetRestriction::PlayerOrPlaneswalkerControlledBy(filter) => {
            match filter {
                PlayerFilter::You => "you and/or one or more planeswalkers you control".to_string(),
                PlayerFilter::Opponent => {
                    "an opponent and/or one or more planeswalkers they control".to_string()
                }
                _ => format!(
                    "{} and/or {}",
                    player_attack_subject(filter),
                    controlled_planeswalker_target(filter)
                ),
            }
        }
    }
}

fn player_attack_target_with_one_or_more_creatures(
    restriction: &ironsmith_core::AttackTargetRestriction,
) -> String {
    match restriction {
        ironsmith_core::AttackTargetRestriction::Player(filter) => {
            format!(
                "{} with one or more creatures",
                player_attack_subject(filter)
            )
        }
        ironsmith_core::AttackTargetRestriction::PlaneswalkerControlledBy(filter) => match filter {
            PlayerFilter::You => {
                "a planeswalker you control with one or more creatures".to_string()
            }
            PlayerFilter::Opponent => {
                "a planeswalker an opponent controls with one or more creatures".to_string()
            }
            _ => format!(
                "a planeswalker {} controls with one or more creatures",
                filter.description()
            ),
        },
        ironsmith_core::AttackTargetRestriction::PlayerOrPlaneswalkerControlledBy(filter) => {
            match filter {
                PlayerFilter::You => {
                    "you or a planeswalker you control with one or more creatures".to_string()
                }
                PlayerFilter::Opponent => {
                    "an opponent or a planeswalker they control with one or more creatures"
                        .to_string()
                }
                _ => format!(
                    "{} or a planeswalker they control with one or more creatures",
                    player_attack_subject(filter)
                ),
            }
        }
    }
}

impl AttacksTrigger {
    /// Create a new attacks trigger with the given filter.
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            one_or_more: false,
            min_total_attackers: 1,
            max_total_attackers: None,
            aggregate_constraint: None,
        }
    }

    /// Create an attacks trigger that fires once for one-or-more attackers.
    pub fn one_or_more(filter: ObjectFilter) -> Self {
        Self {
            filter,
            one_or_more: true,
            min_total_attackers: 1,
            max_total_attackers: None,
            aggregate_constraint: None,
        }
    }

    /// Create an attacks trigger that fires once for one-or-more attackers and
    /// only if at least `min_total_attackers` attackers were declared.
    pub fn one_or_more_with_min_total_attackers(
        filter: ObjectFilter,
        min_total_attackers: usize,
    ) -> Self {
        Self {
            filter,
            one_or_more: true,
            min_total_attackers: min_total_attackers.max(1),
            max_total_attackers: None,
            aggregate_constraint: None,
        }
    }

    /// Create an attacks trigger that fires once for one-or-more attackers and
    /// only if exactly `total_attackers` attackers were declared.
    pub fn one_or_more_with_exact_total_attackers(
        filter: ObjectFilter,
        total_attackers: usize,
    ) -> Self {
        let total_attackers = total_attackers.max(1);
        Self {
            filter,
            one_or_more: true,
            min_total_attackers: total_attackers,
            max_total_attackers: Some(total_attackers),
            aggregate_constraint: None,
        }
    }

    pub fn one_or_more_with_aggregate(
        filter: ObjectFilter,
        metric: ChoiceAggregateMetric,
        comparison: Comparison,
    ) -> Self {
        Self {
            filter,
            one_or_more: true,
            min_total_attackers: 1,
            max_total_attackers: None,
            aggregate_constraint: Some((metric, comparison)),
        }
    }

    /// Create an attacks trigger for any creature.
    pub fn any_creature() -> Self {
        Self::new(ObjectFilter::creature())
    }

    fn is_first_matching_attacker_this_combat(
        &self,
        attacker: ObjectId,
        attack_target: &crate::combat_state::AttackTarget,
        ctx: &TriggerContext,
    ) -> bool {
        let Some(combat) = ctx.game.combat.as_ref() else {
            return true;
        };
        let match_per_defending_player = self
            .filter
            .attacking_player_or_planeswalker_controlled_by
            .is_some();
        let current_defending_player = match_per_defending_player
            .then(|| defending_player_for_attack_target(attack_target, ctx.game))
            .flatten();
        for info in &combat.attackers {
            if match_per_defending_player
                && defending_player_for_attack_target(&info.target, ctx.game)
                    != current_defending_player
            {
                continue;
            }
            if self.matches_attacker_info(info, ctx) {
                return info.creature == attacker;
            }
        }
        true
    }

    fn matching_attacker_count_this_combat(&self, ctx: &TriggerContext) -> Option<i32> {
        let combat = ctx.game.combat.as_ref()?;
        let count = combat
            .attackers
            .iter()
            .filter(|info| self.matches_attacker_info(info, ctx))
            .count();
        (count > 0).then_some(count as i32)
    }

    fn matching_attacker_aggregate_this_combat(
        &self,
        metric: ChoiceAggregateMetric,
        ctx: &TriggerContext,
    ) -> Option<i32> {
        let combat = ctx.game.combat.as_ref()?;
        let values = combat
            .attackers
            .iter()
            .filter(|info| self.matches_attacker_info(info, ctx))
            .map(|info| crate::targeting::aggregate_object_value(ctx.game, info.creature, metric));
        let mut matched = false;
        let total = values.fold(0, |sum, value| {
            matched = true;
            sum + value
        });
        matched.then_some(total)
    }

    pub(crate) fn matches_attacker_info(
        &self,
        info: &crate::combat_state::AttackerInfo,
        ctx: &TriggerContext,
    ) -> bool {
        let Some(obj) = ctx.game.object(info.creature) else {
            return false;
        };
        self.matches_attacker_object_and_target(obj, &info.target, ctx)
    }

    fn matches_attacker_object_and_target(
        &self,
        obj: &crate::object::Object,
        target: &crate::combat_state::AttackTarget,
        ctx: &TriggerContext,
    ) -> bool {
        let mut object_filter = self.filter.clone();
        let attacked_player_filter = object_filter
            .attacking_player_or_planeswalker_controlled_by
            .take();
        let attacked_target_must_be_player = object_filter.targets_only_player.take().is_some();
        if !object_filter.matches(obj, &ctx.filter_ctx, ctx.game) {
            return false;
        }
        let Some(attacked_player_filter) = attacked_player_filter else {
            return true;
        };
        let attacked_player = match target {
            crate::combat_state::AttackTarget::Player(player) => Some(*player),
            crate::combat_state::AttackTarget::Planeswalker(planeswalker) => {
                if attacked_target_must_be_player {
                    None
                } else {
                    ctx.game
                        .object(*planeswalker)
                        .map(|planeswalker| ctx.game.controller_of(planeswalker))
                }
            }
            crate::combat_state::AttackTarget::Battle(battle) => {
                if attacked_target_must_be_player {
                    None
                } else {
                    ctx.game.battle_protector(*battle)
                }
            }
        };
        attacked_player.is_some_and(|player| {
            crate::filter::player_filter_matches_game(
                &attacked_player_filter,
                player,
                ctx.game,
                &ctx.filter_ctx,
            )
        })
    }
}

fn pluralize_attack_subject(subject: &str) -> String {
    if subject == "creature" {
        return "creatures".to_string();
    }
    if let Some(rest) = subject.strip_prefix("creature ") {
        return format!("creatures {rest}");
    }
    subject.to_string()
}

fn source_and_your_commander_attack_subject(filter: &ObjectFilter) -> Option<String> {
    if filter.union_connective() != crate::filter::ObjectFilterUnionConnective::AndOr
        || !filter.union_is_one_or_more()
        || filter.any_of.len() != 2
    {
        return None;
    }
    let source = filter.any_of.iter().find(|branch| branch.source)?;
    let commander = filter.any_of.iter().find(|branch| {
        branch.is_commander
            && (branch.owner.as_ref() == Some(&PlayerFilter::You)
                || branch.controller.as_ref() == Some(&PlayerFilter::You))
    })?;
    if std::ptr::eq(source, commander) {
        return None;
    }
    Some(
        source
            .source_surface
            .as_ref()
            .map(crate::target::SourceReferenceSurface::display_text)
            .unwrap_or_else(|| "this creature".to_string()),
    )
}

impl TriggerMatcher for AttacksTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureAttacked {
            return false;
        }
        let Some(e) = event.downcast::<CreatureAttackedEvent>() else {
            return false;
        };
        let Some(obj) = ctx.game.object(e.attacker) else {
            return false;
        };
        let attack_target = match e.target {
            crate::events::combat::AttackEventTarget::Player(player) => {
                crate::combat_state::AttackTarget::Player(player)
            }
            crate::events::combat::AttackEventTarget::Planeswalker(planeswalker) => {
                crate::combat_state::AttackTarget::Planeswalker(planeswalker)
            }
            crate::events::combat::AttackEventTarget::Battle(battle) => {
                crate::combat_state::AttackTarget::Battle(battle)
            }
        };
        if !self.matches_attacker_object_and_target(obj, &attack_target, ctx) {
            return false;
        }
        if e.total_attackers < self.min_total_attackers {
            return false;
        }
        let matching_attackers = self
            .matching_attacker_count_this_combat(ctx)
            .map(|count| count.max(0) as usize)
            .unwrap_or(e.total_attackers);
        if matching_attackers < self.min_total_attackers {
            return false;
        }
        if let Some(max_total_attackers) = self.max_total_attackers
            && matching_attackers > max_total_attackers
        {
            return false;
        }
        if let Some((metric, comparison)) = &self.aggregate_constraint {
            let Some(total) = self.matching_attacker_aggregate_this_combat(*metric, ctx) else {
                return false;
            };
            if !comparison.satisfies(total) {
                return false;
            }
        }
        if self.one_or_more {
            return self.is_first_matching_attacker_this_combat(e.attacker, &attack_target, ctx);
        }
        true
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::CreatureAttacked])
    }

    fn display(&self) -> String {
        if self.one_or_more
            && self.min_total_attackers == 1
            && self.max_total_attackers.is_none()
            && let Some(source_subject) = source_and_your_commander_attack_subject(&self.filter)
        {
            return format!("Whenever you attack with {source_subject} and/or your commander");
        }
        let mut display_filter = self.filter.clone();
        let explicit_attack_with_group = display_filter.union_is_one_or_more();
        // Attacking already implies a creature; oracle says "another Cat you
        // control attacks", not "another Cat creature you control attacks".
        if display_filter.card_types == [crate::types::CardType::Creature]
            && !display_filter.subtypes.is_empty()
            && display_filter.all_card_types.is_empty()
        {
            display_filter.card_types.clear();
        }
        let attacked_player = display_filter
            .attacking_player_or_planeswalker_controlled_by
            .take();
        let attacked_target_must_be_player = display_filter.targets_only_player.take().is_some();
        let described_subject = display_filter.description();
        let articleless_attachment_subject =
            display_filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    && matches!(constraint.tag.as_str(), "enchanted" | "equipped")
            }) || described_subject.starts_with("an enchanted ")
                || described_subject.starts_with("an equipped ");
        let mut subject = described_subject.clone();
        if let Some(stripped) = subject.strip_prefix("a ") {
            subject = stripped.to_string();
        } else if let Some(stripped) = subject.strip_prefix("an ") {
            subject = stripped.to_string();
        }
        let base_subject = subject.clone();
        let subject = if self.one_or_more {
            pluralize_one_or_more_attack_subject(&subject)
        } else {
            subject
        };
        let subject = if self.one_or_more {
            subject.replace(" an opponent controls", " your opponents control")
        } else {
            subject
        };
        let target_tail = match (attacked_player.as_ref(), attacked_target_must_be_player) {
            (Some(PlayerFilter::Opponent), true) => " an opponent".to_string(),
            (Some(PlayerFilter::Any), true) => " a player".to_string(),
            (Some(PlayerFilter::Opponent), false) => {
                if self.one_or_more {
                    " one of your opponents or a planeswalker they control".to_string()
                } else {
                    " one of your opponents or a planeswalker an opponent controls".to_string()
                }
            }
            (Some(PlayerFilter::Any), false) => " a player or planeswalker".to_string(),
            (Some(PlayerFilter::You), true) => " you".to_string(),
            (Some(PlayerFilter::TaggedPlayer(tag)), true) if tag.as_str() == "enchanted" => {
                " enchanted player".to_string()
            }
            (Some(PlayerFilter::TaggedPlayer(tag)), true)
                if tag.as_str() == crate::tag::INITIATIVE_HOLDER_TAG =>
            {
                " the player who has the initiative".to_string()
            }
            (Some(PlayerFilter::TaggedPlayer(tag)), false) if tag.as_str() == "enchanted" => {
                " enchanted player or a planeswalker they control".to_string()
            }
            (Some(PlayerFilter::HasMoreLifeThanYou { base }), true) => {
                format!(" {} who has more life than you", base.description())
            }
            (Some(PlayerFilter::MostLifeTied), true) => {
                " the player with the most life or tied for most life".to_string()
            }
            _ => String::new(),
        };

        if let Some((metric, comparison)) = &self.aggregate_constraint {
            let metric = match metric {
                ChoiceAggregateMetric::Power => "power",
                ChoiceAggregateMetric::Toughness => "toughness",
                ChoiceAggregateMetric::ManaValue => "mana value",
            };
            let comparison = match comparison {
                Comparison::Equal(value) => value.to_string(),
                Comparison::OneOf(values) => values
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" or "),
                Comparison::NotEqual(value) => format!("not equal to {value}"),
                Comparison::LessThan(value) => format!("less than {value}"),
                Comparison::LessThanOrEqual(value) => format!("{value} or less"),
                Comparison::GreaterThan(value) => format!("greater than {value}"),
                Comparison::GreaterThanOrEqual(value) => format!("{value} or greater"),
                _ => "the required amount".to_string(),
            };
            let group_subject = pluralize_attack_subject(
                &base_subject
                    .replace(" you control", "")
                    .replace(" an opponent controls", ""),
            );
            if base_subject.ends_with(" you control") {
                return format!(
                    "Whenever you attack{target_tail} with {group_subject} with total {metric} {comparison}"
                );
            }
            return format!(
                "Whenever {group_subject} with total {metric} {comparison} attack{target_tail}"
            );
        }

        if self.one_or_more {
            if let Some(exact_total) = self.max_total_attackers
                && exact_total == self.min_total_attackers
            {
                let exact_total_text = ironsmith_core::cardinal_word(exact_total as u32)
                    .unwrap_or_else(|| exact_total.to_string());
                if display_filter.source {
                    let other_count = exact_total.saturating_sub(1) as u32;
                    let other_text = ironsmith_core::cardinal_word(other_count)
                        .unwrap_or_else(|| other_count.to_string());
                    return format!(
                        "Whenever this creature and exactly {other_text} other creatures attack{target_tail}"
                    );
                }
                if target_tail.is_empty()
                    && explicit_attack_with_group
                    && subject.contains(" you control")
                {
                    let controlled_subject = subject.replacen(" you control", "", 1);
                    return format!(
                        "Whenever you attack with exactly {exact_total_text} {}",
                        pluralize_attack_subject(&controlled_subject)
                    );
                }
                return format!(
                    "Whenever exactly {exact_total_text} {subject} attack{target_tail}"
                );
            }
            if self.min_total_attackers > 1 {
                if display_filter.source {
                    let other_count = self.min_total_attackers.saturating_sub(1) as u32;
                    let other_text = ironsmith_core::cardinal_word(other_count)
                        .unwrap_or_else(|| other_count.to_string());
                    return format!(
                        "Whenever this creature and at least {other_text} other creatures attack{target_tail}"
                    );
                }
                let min_total = ironsmith_core::cardinal_word(self.min_total_attackers as u32)
                    .unwrap_or_else(|| self.min_total_attackers.to_string());
                if target_tail.is_empty()
                    && explicit_attack_with_group
                    && subject.contains(" you control")
                {
                    let controlled_subject = subject.replacen(" you control", "", 1);
                    return format!(
                        "Whenever you attack with {min_total} or more {}",
                        pluralize_attack_subject(&controlled_subject)
                    );
                }
                if explicit_attack_with_group
                    && base_subject == "creature an opponent controls"
                    && matches!(attacked_player.as_ref(), Some(PlayerFilter::You))
                    && attacked_target_must_be_player
                {
                    let attacker_subject =
                        pluralize_attack_subject(&subject.replacen(" an opponent controls", "", 1));
                    return format!(
                        "Whenever an opponent attacks you with {min_total} or more {attacker_subject}"
                    );
                }
                return format!("Whenever {min_total} or more {subject} attack{target_tail}");
            }
            if explicit_attack_with_group && subject.contains(" you control") {
                let controlled_subject = subject.replacen(" you control", "", 1);
                return format!(
                    "Whenever you attack{target_tail} with one or more {}",
                    pluralize_attack_subject(&controlled_subject)
                );
            }
            if base_subject == "creature you control" && target_tail.is_empty() {
                return "Whenever you attack".to_string();
            }
            if base_subject == "creature you control"
                && matches!(attacked_player.as_ref(), Some(PlayerFilter::Any))
                && attacked_target_must_be_player
            {
                return "Whenever you attack a player".to_string();
            }
            if base_subject == "creature you control"
                && matches!(
                    attacked_player.as_ref(),
                    Some(PlayerFilter::TaggedPlayer(tag))
                        if matches!(
                            tag.as_str(),
                            "enchanted" | crate::tag::INITIATIVE_HOLDER_TAG
                        )
                )
            {
                return format!("Whenever you attack{target_tail}");
            }
            if base_subject == "creature"
                && matches!(attacked_player.as_ref(), Some(PlayerFilter::You))
                && attacked_target_must_be_player
            {
                return "Whenever a player attacks you".to_string();
            }
            if base_subject == "creature an opponent controls"
                && matches!(attacked_player.as_ref(), Some(PlayerFilter::Opponent))
                && attacked_target_must_be_player
            {
                return "Whenever an opponent attacks another one of your opponents".to_string();
            }
            return format!("Whenever one or more {subject} attack{target_tail}");
        }
        if self.min_total_attackers > 1 {
            return format!(
                "Whenever {} or more {subject} attack{target_tail}",
                self.min_total_attackers,
            );
        }
        if display_filter.source {
            let source_subject = display_filter
                .source_surface
                .as_ref()
                .map(crate::target::SourceReferenceSurface::display_text)
                .unwrap_or_else(|| "this creature".to_string());
            return format!("Whenever {source_subject} attacks{target_tail}");
        }
        let subject = if articleless_attachment_subject {
            subject
        } else {
            described_subject
        };
        format!("Whenever {subject} attacks{target_tail}")
    }

    fn event_value_amount(&self, event: &TriggerEvent, ctx: &TriggerContext) -> Option<i32> {
        if !self.one_or_more || !self.matches(event, ctx) {
            return None;
        }
        self.matching_attacker_count_this_combat(ctx)
    }
}

impl TriggerMatcher for PlayersAttackedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureAttacked {
            return false;
        }
        let Some(e) = event.downcast::<CreatureAttackedEvent>() else {
            return false;
        };
        let attack_target = match e.target {
            crate::events::combat::AttackEventTarget::Player(player) => {
                crate::combat_state::AttackTarget::Player(player)
            }
            crate::events::combat::AttackEventTarget::Planeswalker(planeswalker) => {
                crate::combat_state::AttackTarget::Planeswalker(planeswalker)
            }
            crate::events::combat::AttackEventTarget::Battle(battle) => {
                crate::combat_state::AttackTarget::Battle(battle)
            }
        };
        self.is_first_matching_attacker_this_combat(e.attacker, &attack_target, ctx)
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::CreatureAttacked])
    }

    fn display(&self) -> String {
        match &self.player_filter {
            PlayerFilter::Opponent => {
                "Whenever one or more of your opponents are attacked".to_string()
            }
            PlayerFilter::You => "Whenever you are attacked".to_string(),
            PlayerFilter::Any => "Whenever one or more players are attacked".to_string(),
            _ => "Whenever one or more matching players are attacked".to_string(),
        }
    }
}

impl TriggerMatcher for PlayerAttacksOneOrMoreTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CreatureAttacked {
            return false;
        }
        let Some(event) = event.downcast::<CreatureAttackedEvent>() else {
            return false;
        };
        if !self.attacker_matches(event.attacker, ctx) {
            return false;
        }
        let target = match event.target {
            crate::events::combat::AttackEventTarget::Player(player) => {
                crate::combat_state::AttackTarget::Player(player)
            }
            crate::events::combat::AttackEventTarget::Planeswalker(planeswalker) => {
                crate::combat_state::AttackTarget::Planeswalker(planeswalker)
            }
            crate::events::combat::AttackEventTarget::Battle(battle) => {
                crate::combat_state::AttackTarget::Battle(battle)
            }
        };
        self.target_matches(&target, ctx)
            && self.is_first_matching_attacker_this_combat(event.attacker, &target, ctx)
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::CreatureAttacked])
    }

    fn display(&self) -> String {
        let target = if self.group_by_target {
            player_attack_target_with_one_or_more_creatures(&self.target)
        } else {
            player_attack_target(&self.target)
        };
        format!(
            "Whenever {} attacks {}",
            player_attack_subject(&self.attacker),
            target
        )
    }
}

fn defending_player_for_attack_target(
    target: &crate::combat_state::AttackTarget,
    game: &crate::game_state::GameState,
) -> Option<crate::ids::PlayerId> {
    match target {
        crate::combat_state::AttackTarget::Player(player) => Some(*player),
        crate::combat_state::AttackTarget::Planeswalker(planeswalker) => game
            .object(*planeswalker)
            .map(|planeswalker| game.controller_of(planeswalker)),
        crate::combat_state::AttackTarget::Battle(battle) => game.battle_protector(*battle),
    }
}

fn pluralize_one_or_more_attack_subject(subject: &str) -> String {
    if subject == "creature" {
        return "creatures".to_string();
    }
    if let Some(rest) = subject.strip_prefix("creature ") {
        return format!("creatures {rest}");
    }
    if subject == "permanent" {
        return "permanents".to_string();
    }
    if let Some(rest) = subject.strip_prefix("permanent ") {
        return format!("permanents {rest}");
    }
    // Bare subtype subjects ("Dragon you control") pluralize the subtype.
    if !subject.contains(" creature")
        && let Some((head, tail)) = subject.split_once(' ')
        && !head.is_empty()
        && head
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        && !head.ends_with('s')
        && (tail.starts_with("you ")
            || tail.starts_with("an opponent ")
            || tail.starts_with("your "))
    {
        return crate::runtime_display::pluralize_noun_phrase_for_trigger(subject);
    }
    if let Some((head, tail)) = subject.split_once(" creature ") {
        if !head.contains(' ')
            && head
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            return format!(
                "{} {tail}",
                crate::runtime_display::pluralize_noun_phrase_for_trigger(head)
            );
        }
        return format!("{head} creatures {tail}");
    }
    if let Some(stripped) = subject.strip_suffix(" creature") {
        if !stripped.contains(' ')
            && stripped
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            return crate::runtime_display::pluralize_noun_phrase_for_trigger(stripped);
        }
        return format!("{stripped} creatures");
    }
    if let Some((head, tail)) = subject.split_once(" permanent ") {
        return format!("{head} permanents {tail}");
    }
    if let Some(stripped) = subject.strip_suffix(" permanent") {
        return format!("{stripped} permanents");
    }
    subject.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use crate::events::combat::AttackEventTarget;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::AttachmentTarget;
    use crate::target::PlayerFilter;
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();

        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    #[test]
    fn test_matches_creature_attack() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(100);
        let creature_id = create_creature(&mut game, "Grizzly Bears", alice);

        let trigger = AttacksTrigger::any_creature();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::new(creature_id, AttackEventTarget::Player(bob)),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_display() {
        let trigger = AttacksTrigger::any_creature();
        assert!(trigger.display().contains("attacks"));
    }

    #[test]
    fn typed_player_attack_target_distinguishes_planeswalker_only_from_player_or_planeswalker() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(100);
        let attacker = create_creature(&mut game, "Attacker", bob);
        let walker_card = CardBuilder::new(CardId::from_raw(901), "Walker")
            .card_types(vec![CardType::Planeswalker])
            .loyalty(3)
            .build();
        let walker = game.create_object_from_card(&walker_card, alice, Zone::Battlefield);
        let battle_card = CardBuilder::new(CardId::from_raw(902), "Battle")
            .card_types(vec![CardType::Battle])
            .defense(3)
            .build();
        let battle = game.create_object_from_card(&battle_card, alice, Zone::Battlefield);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let planeswalker_only = PlayerAttacksOneOrMoreTrigger::new(
            PlayerFilter::Opponent,
            ironsmith_core::AttackTargetRestriction::PlaneswalkerControlledBy(PlayerFilter::You),
        );
        let player_or_planeswalker = PlayerAttacksOneOrMoreTrigger::new(
            PlayerFilter::Opponent,
            ironsmith_core::AttackTargetRestriction::PlayerOrPlaneswalkerControlledBy(
                PlayerFilter::You,
            ),
        );
        let player_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::new(attacker, AttackEventTarget::Player(alice)),
            crate::provenance::ProvNodeId::default(),
        );
        let walker_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::new(attacker, AttackEventTarget::Planeswalker(walker)),
            crate::provenance::ProvNodeId::default(),
        );
        let battle_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::new(attacker, AttackEventTarget::Battle(battle)),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(!planeswalker_only.matches(&player_event, &ctx));
        assert!(planeswalker_only.matches(&walker_event, &ctx));
        assert!(player_or_planeswalker.matches(&player_event, &ctx));
        assert!(player_or_planeswalker.matches(&walker_event, &ctx));
        assert!(!planeswalker_only.matches(&battle_event, &ctx));
        assert!(!player_or_planeswalker.matches(&battle_event, &ctx));
        assert_eq!(
            planeswalker_only.display(),
            "Whenever an opponent attacks one or more planeswalkers you control"
        );
    }

    #[test]
    fn per_defender_player_attack_group_triggers_once_for_each_attacked_planeswalker() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(100);
        let first = create_creature(&mut game, "First", bob);
        let second = create_creature(&mut game, "Second", bob);
        let third = create_creature(&mut game, "Third", bob);
        let first_walker_card = CardBuilder::new(CardId::from_raw(903), "First Walker")
            .card_types(vec![CardType::Planeswalker])
            .loyalty(3)
            .build();
        let first_walker =
            game.create_object_from_card(&first_walker_card, alice, Zone::Battlefield);
        let second_walker_card = CardBuilder::new(CardId::from_raw(904), "Second Walker")
            .card_types(vec![CardType::Planeswalker])
            .loyalty(3)
            .build();
        let second_walker =
            game.create_object_from_card(&second_walker_card, alice, Zone::Battlefield);

        game.combat = Some(CombatState {
            attackers: vec![
                AttackerInfo {
                    creature: first,
                    target: AttackTarget::Planeswalker(first_walker),
                },
                AttackerInfo {
                    creature: second,
                    target: AttackTarget::Planeswalker(first_walker),
                },
                AttackerInfo {
                    creature: third,
                    target: AttackTarget::Planeswalker(second_walker),
                },
            ],
            ..CombatState::default()
        });

        let target =
            ironsmith_core::AttackTargetRestriction::PlaneswalkerControlledBy(PlayerFilter::You);
        let across_all_targets =
            PlayerAttacksOneOrMoreTrigger::new(PlayerFilter::Opponent, target.clone());
        let per_target =
            PlayerAttacksOneOrMoreTrigger::grouped_by_target(PlayerFilter::Opponent, target);
        let event = |attacker, walker| {
            TriggerEvent::new_with_provenance(
                CreatureAttackedEvent::with_total_attackers(
                    attacker,
                    AttackEventTarget::Planeswalker(walker),
                    3,
                ),
                crate::provenance::ProvNodeId::default(),
            )
        };
        let first_event = event(first, first_walker);
        let second_event = event(second, first_walker);
        let third_event = event(third, second_walker);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        assert!(per_target.matches(&first_event, &ctx));
        assert!(!per_target.matches(&second_event, &ctx));
        assert!(per_target.matches(&third_event, &ctx));
        assert!(across_all_targets.matches(&first_event, &ctx));
        assert!(!across_all_targets.matches(&second_event, &ctx));
        assert!(!across_all_targets.matches(&third_event, &ctx));
        assert_eq!(
            per_target.display(),
            "Whenever an opponent attacks a planeswalker you control with one or more creatures"
        );
    }

    #[test]
    fn source_and_or_your_commander_attack_keeps_authored_aggregate_surface() {
        let source = ObjectFilter::source_with_surface(
            crate::target::SourceReferenceSurface::ThisPermanentType("this creature".to_string()),
        );
        let mut commander = ObjectFilter::default();
        commander.is_commander = true;
        commander.owner = Some(PlayerFilter::You);
        let mut filter = ObjectFilter::default();
        filter.any_of = vec![source, commander];
        filter.set_union_connective(crate::filter::ObjectFilterUnionConnective::AndOr);
        filter.set_union_one_or_more(true);

        assert_eq!(
            AttacksTrigger::one_or_more(filter).display(),
            "Whenever you attack with this creature and/or your commander"
        );
    }

    fn sea_creature_subtype_filter() -> ObjectFilter {
        ObjectFilter::default()
            .in_zone(Zone::Battlefield)
            .with_subtype(Subtype::Kraken)
            .with_subtype(Subtype::Leviathan)
            .with_subtype(Subtype::Merfolk)
            .with_subtype(Subtype::Octopus)
            .with_subtype(Subtype::Serpent)
    }

    #[test]
    fn serial_subtype_attack_subject_keeps_one_shared_article() {
        let mut filter = sea_creature_subtype_filter();
        filter.set_serial_or_list_surface(true);
        filter.set_shared_indefinite_article_surface(true);

        assert_eq!(
            AttacksTrigger::new(filter).display(),
            "Whenever a Kraken, Leviathan, Merfolk, Octopus, or Serpent attacks"
        );
    }

    #[test]
    fn repeated_or_attack_subject_does_not_infer_serial_surface_or_article() {
        assert_eq!(
            AttacksTrigger::new(sea_creature_subtype_filter()).display(),
            "Whenever Kraken or Leviathan or Merfolk or Octopus or Serpent attacks"
        );
    }

    #[test]
    fn one_or_more_you_control_attack_player_displays_you_attack_player() {
        let mut filter = ObjectFilter::creature().you_control();
        filter.attacking_player_or_planeswalker_controlled_by = Some(PlayerFilter::Any);
        filter.targets_only_player = Some(PlayerFilter::Any);
        let trigger = AttacksTrigger::one_or_more(filter);

        assert_eq!(trigger.display(), "Whenever you attack a player");
    }

    #[test]
    fn one_or_more_you_control_attack_enchanted_player_keeps_player_attack_surface() {
        let enchanted = PlayerFilter::TaggedPlayer(crate::tag::TagKey::from("enchanted"));
        let mut filter = ObjectFilter::creature().you_control();
        filter.attacking_player_or_planeswalker_controlled_by = Some(enchanted.clone());
        filter.targets_only_player = Some(enchanted);
        let trigger = AttacksTrigger::one_or_more(filter);

        assert_eq!(trigger.display(), "Whenever you attack enchanted player");
    }

    #[test]
    fn one_or_more_you_control_attack_enchanted_players_planeswalker_keeps_target_surface() {
        let enchanted = PlayerFilter::TaggedPlayer(crate::tag::TagKey::from("enchanted"));
        let mut filter = ObjectFilter::creature().you_control();
        filter.attacking_player_or_planeswalker_controlled_by = Some(enchanted);
        let trigger = AttacksTrigger::one_or_more(filter);

        assert_eq!(
            trigger.display(),
            "Whenever you attack enchanted player or a planeswalker they control"
        );
    }

    #[test]
    fn initiative_holder_attack_target_tracks_current_designation() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = ObjectId::from_raw(100);
        let attacker = create_creature(&mut game, "Initiative Seeker", alice);
        let initiative =
            PlayerFilter::TaggedPlayer(crate::tag::TagKey::from(crate::tag::INITIATIVE_HOLDER_TAG));
        let mut filter = ObjectFilter::creature().you_control();
        filter.attacking_player_or_planeswalker_controlled_by = Some(initiative.clone());
        filter.targets_only_player = Some(initiative);
        let trigger = AttacksTrigger::one_or_more(filter);
        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::new(attacker, AttackEventTarget::Player(bob)),
            crate::provenance::ProvNodeId::default(),
        );

        game.set_initiative(Some(bob));
        assert!(trigger.matches(&event, &TriggerContext::for_source(source, alice, &game)));
        game.set_initiative(Some(alice));
        assert!(!trigger.matches(&event, &TriggerContext::for_source(source, alice, &game)));
        assert_eq!(
            trigger.display(),
            "Whenever you attack the player who has the initiative"
        );
    }

    #[test]
    fn one_or_more_unfiltered_creatures_attack_you_keeps_player_subject_surface() {
        let mut filter = ObjectFilter::creature();
        filter.attacking_player_or_planeswalker_controlled_by = Some(PlayerFilter::You);
        filter.targets_only_player = Some(PlayerFilter::You);
        let trigger = AttacksTrigger::one_or_more(filter);

        assert_eq!(trigger.display(), "Whenever a player attacks you");
    }

    #[test]
    fn unqualified_and_subtype_attack_groups_stay_structurally_distinct() {
        let unqualified = AttacksTrigger::one_or_more(ObjectFilter::creature().you_control());
        let warriors = AttacksTrigger::one_or_more(
            ObjectFilter::creature()
                .you_control()
                .with_subtype(Subtype::Warrior),
        );

        assert!(unqualified.one_or_more);
        assert_eq!(unqualified.filter.controller, Some(PlayerFilter::You));
        assert!(unqualified.filter.subtypes.is_empty());
        assert!(warriors.one_or_more);
        assert_eq!(warriors.filter.controller, Some(PlayerFilter::You));
        assert_eq!(warriors.filter.subtypes, vec![Subtype::Warrior]);
    }

    #[test]
    fn named_source_attack_matches_only_a_player_with_more_life_than_controller() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, "Namor", alice);
        let other = create_creature(&mut game, "Other Attacker", alice);
        game.player_mut(alice).expect("Alice exists").life = 20;
        game.player_mut(bob).expect("Bob exists").life = 21;

        let relative_life = PlayerFilter::HasMoreLifeThanYou {
            base: Box::new(PlayerFilter::Any),
        };
        let mut filter = ObjectFilter::source_with_surface(
            crate::target::SourceReferenceSurface::FullName("Namor".to_string()),
        );
        filter.attacking_player_or_planeswalker_controlled_by = Some(relative_life.clone());
        filter.targets_only_player = Some(relative_life);
        let trigger = AttacksTrigger::new(filter);
        assert_eq!(
            trigger.display(),
            "Whenever Namor attacks a player who has more life than you"
        );

        let player_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::new(source, AttackEventTarget::Player(bob)),
            crate::provenance::ProvNodeId::default(),
        );
        let other_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::new(other, AttackEventTarget::Player(bob)),
            crate::provenance::ProvNodeId::default(),
        );
        {
            let ctx = TriggerContext::for_source(source, alice, &game);
            assert!(trigger.matches(&player_event, &ctx));
            assert!(!trigger.matches(&other_event, &ctx));
        }

        let walker_card = CardBuilder::new(CardId::from_raw(991), "Walker")
            .card_types(vec![CardType::Planeswalker])
            .build();
        let walker = game.create_object_from_card(&walker_card, bob, Zone::Battlefield);
        let walker_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::new(source, AttackEventTarget::Planeswalker(walker)),
            crate::provenance::ProvNodeId::default(),
        );
        {
            let ctx = TriggerContext::for_source(source, alice, &game);
            assert!(
                !trigger.matches(&walker_event, &ctx),
                "the relative-life player surface must not match an attack on their planeswalker"
            );
        }

        game.player_mut(bob).expect("Bob exists").life = 20;
        let ctx = TriggerContext::for_source(source, alice, &game);
        assert!(
            !trigger.matches(&player_event, &ctx),
            "equal life is not strictly more life"
        );
    }

    #[test]
    fn explicit_attack_with_group_preserves_count_antecedent_surface() {
        let mut filter = ObjectFilter::creature().you_control();
        filter.set_union_one_or_more(true);
        let trigger = AttacksTrigger::one_or_more(filter);

        assert_eq!(
            trigger.display(),
            "Whenever you attack with one or more creatures"
        );
    }

    #[test]
    fn suspected_attack_filter_matches_and_displays_the_designation() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(100);
        let suspected = create_creature(&mut game, "Suspected", alice);
        let ordinary = create_creature(&mut game, "Ordinary", alice);
        game.set_suspected(suspected);

        let trigger =
            AttacksTrigger::one_or_more(ObjectFilter::creature().you_control().suspected());
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        let suspected_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::new(suspected, AttackEventTarget::Player(bob)),
            crate::provenance::ProvNodeId::default(),
        );
        let ordinary_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::new(ordinary, AttackEventTarget::Player(bob)),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(trigger.matches(&suspected_event, &ctx));
        assert!(!trigger.matches(&ordinary_event, &ctx));
        assert_eq!(
            trigger.display(),
            "Whenever one or more suspected creatures you control attack"
        );
    }

    #[test]
    fn explicit_attack_with_group_preserves_player_or_planeswalker_target_surface() {
        let mut filter = ObjectFilter::creature().you_control();
        filter.attacking_player_or_planeswalker_controlled_by = Some(PlayerFilter::Any);
        filter.set_union_one_or_more(true);
        let trigger = AttacksTrigger::one_or_more(filter);

        assert_eq!(
            trigger.display(),
            "Whenever you attack a player or planeswalker with one or more creatures"
        );
    }

    #[test]
    fn one_or_more_opponent_controls_attack_opponent_displays_opponent_attacks_another() {
        let mut filter = ObjectFilter::creature();
        filter.controller = Some(PlayerFilter::Opponent);
        filter.attacking_player_or_planeswalker_controlled_by = Some(PlayerFilter::Opponent);
        filter.targets_only_player = Some(PlayerFilter::Opponent);
        let trigger = AttacksTrigger::one_or_more(filter);

        assert_eq!(
            trigger.display(),
            "Whenever an opponent attacks another one of your opponents"
        );
    }

    #[test]
    fn test_one_or_more_only_matches_first_attacker_in_declaration() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(100);
        let attacker_one = create_creature(&mut game, "A", alice);
        let attacker_two = create_creature(&mut game, "B", alice);

        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: attacker_one,
            target: AttackTarget::Player(bob),
        });
        combat.attackers.push(AttackerInfo {
            creature: attacker_two,
            target: AttackTarget::Player(bob),
        });
        game.combat = Some(combat);

        let trigger = AttacksTrigger::one_or_more(ObjectFilter::creature());
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let first_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                attacker_one,
                AttackEventTarget::Player(bob),
                2,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&first_event, &ctx));

        let second_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                attacker_two,
                AttackEventTarget::Player(bob),
                2,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&second_event, &ctx));
    }

    #[test]
    fn one_or_more_attack_an_opponent_matches_first_attacker_for_each_opponent() {
        let mut game = GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let source_id = ObjectId::from_raw(100);
        let bob_attacker_one = create_creature(&mut game, "A", alice);
        let bob_attacker_two = create_creature(&mut game, "B", alice);
        let charlie_attacker = create_creature(&mut game, "C", alice);

        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: bob_attacker_one,
            target: AttackTarget::Player(bob),
        });
        combat.attackers.push(AttackerInfo {
            creature: bob_attacker_two,
            target: AttackTarget::Player(bob),
        });
        combat.attackers.push(AttackerInfo {
            creature: charlie_attacker,
            target: AttackTarget::Player(charlie),
        });
        game.combat = Some(combat);

        let mut filter = ObjectFilter::creature().you_control();
        filter.attacking_player_or_planeswalker_controlled_by = Some(PlayerFilter::Opponent);
        filter.targets_only_player = Some(PlayerFilter::Any);
        let trigger = AttacksTrigger::one_or_more(filter);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let first_bob_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                bob_attacker_one,
                AttackEventTarget::Player(bob),
                3,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&first_bob_event, &ctx));

        let second_bob_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                bob_attacker_two,
                AttackEventTarget::Player(bob),
                3,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&second_bob_event, &ctx));

        let charlie_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                charlie_attacker,
                AttackEventTarget::Player(charlie),
                3,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&charlie_event, &ctx));
    }

    #[test]
    fn players_attacked_one_or_more_matches_first_attacker_across_all_opponents() {
        let mut game = GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let source_id = ObjectId::from_raw(100);
        let bob_attacker = create_creature(&mut game, "A", bob);
        let charlie_attacker = create_creature(&mut game, "B", charlie);

        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: bob_attacker,
            target: AttackTarget::Player(charlie),
        });
        combat.attackers.push(AttackerInfo {
            creature: charlie_attacker,
            target: AttackTarget::Player(bob),
        });
        game.combat = Some(combat);

        let trigger = PlayersAttackedTrigger::one_or_more(PlayerFilter::Opponent);
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        assert_eq!(
            trigger.display(),
            "Whenever one or more of your opponents are attacked"
        );

        let first_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                bob_attacker,
                AttackEventTarget::Player(charlie),
                2,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&first_event, &ctx));

        let second_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                charlie_attacker,
                AttackEventTarget::Player(bob),
                2,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&second_event, &ctx));

        let controller_attacked_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                bob_attacker,
                AttackEventTarget::Player(alice),
                1,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&controller_attacked_event, &ctx));
    }

    #[test]
    fn one_or_more_can_match_attackers_attacking_enchanted_player() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_card = CardBuilder::new(CardId::from_raw(900), "Curse Probe")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .build();
        let source_id = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        game.object_mut(source_id)
            .expect("source should exist")
            .attached_to = Some(AttachmentTarget::Player(bob));
        let attacker = create_creature(&mut game, "A", alice);

        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: attacker,
            target: AttackTarget::Player(bob),
        });
        game.combat = Some(combat);

        let mut filter = ObjectFilter::creature();
        filter.controller = Some(PlayerFilter::Any);
        filter.attacking_player_or_planeswalker_controlled_by = Some(PlayerFilter::TaggedPlayer(
            crate::tag::TagKey::from("enchanted"),
        ));
        let trigger = AttacksTrigger::one_or_more(filter);
        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                attacker,
                AttackEventTarget::Player(bob),
                1,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        assert!(trigger.matches(&event, &ctx));
        drop(ctx);

        let walker_card = CardBuilder::new(CardId::from_raw(901), "Walker")
            .card_types(vec![CardType::Planeswalker])
            .build();
        let walker = game.create_object_from_card(&walker_card, bob, Zone::Battlefield);
        let walker_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                attacker,
                AttackEventTarget::Planeswalker(walker),
                1,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        assert!(trigger.matches(&walker_event, &ctx));

        let mut player_only_filter = trigger.filter.clone();
        player_only_filter.targets_only_player = Some(PlayerFilter::Any);
        let player_only_trigger = AttacksTrigger::one_or_more(player_only_filter);
        assert!(!player_only_trigger.matches(&walker_event, &ctx));
    }

    #[test]
    fn test_one_or_more_with_min_total_attackers_requires_threshold() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(100);
        let attacker_one = create_creature(&mut game, "A", alice);
        let attacker_two = create_creature(&mut game, "B", alice);
        let attacker_three = create_creature(&mut game, "C", alice);

        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: attacker_one,
            target: AttackTarget::Player(bob),
        });
        combat.attackers.push(AttackerInfo {
            creature: attacker_two,
            target: AttackTarget::Player(bob),
        });
        combat.attackers.push(AttackerInfo {
            creature: attacker_three,
            target: AttackTarget::Player(bob),
        });
        game.combat = Some(combat);

        let trigger =
            AttacksTrigger::one_or_more_with_min_total_attackers(ObjectFilter::creature(), 3);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let below_threshold = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                attacker_one,
                AttackEventTarget::Player(bob),
                2,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&below_threshold, &ctx));

        let first_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                attacker_one,
                AttackEventTarget::Player(bob),
                3,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&first_event, &ctx));

        let second_event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                attacker_two,
                AttackEventTarget::Player(bob),
                3,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&second_event, &ctx));
    }

    #[test]
    fn aggregate_power_attack_trigger_sums_the_declared_group_once() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(100);
        let first = create_creature(&mut game, "A", alice);
        let second = create_creature(&mut game, "B", alice);
        let third = create_creature(&mut game, "C", alice);

        let trigger = AttacksTrigger::one_or_more_with_aggregate(
            ObjectFilter::creature().you_control(),
            ChoiceAggregateMetric::Power,
            Comparison::GreaterThanOrEqual(6),
        );
        assert_eq!(
            trigger.display(),
            "Whenever you attack with creatures with total power 6 or greater"
        );

        let event = |attacker, total_attackers| {
            TriggerEvent::new_with_provenance(
                CreatureAttackedEvent::with_total_attackers(
                    attacker,
                    AttackEventTarget::Player(bob),
                    total_attackers,
                ),
                crate::provenance::ProvNodeId::default(),
            )
        };

        game.combat = Some(CombatState {
            attackers: vec![
                AttackerInfo {
                    creature: first,
                    target: AttackTarget::Player(bob),
                },
                AttackerInfo {
                    creature: second,
                    target: AttackTarget::Player(bob),
                },
            ],
            ..CombatState::default()
        });
        let below = event(first, 2);
        assert!(!trigger.matches(&below, &TriggerContext::for_source(source_id, alice, &game)));

        game.combat
            .as_mut()
            .expect("combat exists")
            .attackers
            .push(AttackerInfo {
                creature: third,
                target: AttackTarget::Player(bob),
            });
        let first_event = event(first, 3);
        let second_event = event(second, 3);
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        assert!(trigger.matches(&first_event, &ctx));
        assert!(!trigger.matches(&second_event, &ctx));
    }

    #[test]
    fn minimum_attack_groups_render_filtered_cardinality_without_collapsing() {
        let opponent_creatures = ObjectFilter::creature().controlled_by(PlayerFilter::Opponent);
        assert_eq!(
            AttacksTrigger::one_or_more_with_min_total_attackers(opponent_creatures, 2).display(),
            "Whenever two or more creatures your opponents control attack"
        );

        let flying_you_control = ObjectFilter::creature()
            .you_control()
            .with_static_ability(crate::static_abilities::StaticAbilityId::Flying);
        assert_eq!(
            AttacksTrigger::one_or_more_with_min_total_attackers(flying_you_control.clone(), 3)
                .display(),
            "Whenever three or more creatures you control with flying attack"
        );

        let mut explicit_attack_with_flying = flying_you_control;
        explicit_attack_with_flying.set_union_one_or_more(true);
        assert_eq!(
            AttacksTrigger::one_or_more_with_min_total_attackers(explicit_attack_with_flying, 2)
                .display(),
            "Whenever you attack with two or more creatures with flying"
        );

        let mut attacking_a_player = ObjectFilter::creature().you_control();
        attacking_a_player.attacking_player_or_planeswalker_controlled_by = Some(PlayerFilter::Any);
        attacking_a_player.targets_only_player = Some(PlayerFilter::Any);
        assert_eq!(
            AttacksTrigger::one_or_more_with_min_total_attackers(attacking_a_player, 2).display(),
            "Whenever two or more creatures you control attack a player"
        );
    }

    #[test]
    fn minimum_attack_group_counts_only_creatures_matching_the_trigger_filter() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(100);
        let matching_one = create_creature(&mut game, "A", alice);
        let matching_two = create_creature(&mut game, "B", alice);
        let nonmatching = create_creature(&mut game, "C", bob);

        let mut combat = CombatState::default();
        for creature in [matching_one, matching_two, nonmatching] {
            combat.attackers.push(AttackerInfo {
                creature,
                target: AttackTarget::Player(bob),
            });
        }
        game.combat = Some(combat);

        let trigger = AttacksTrigger::one_or_more_with_min_total_attackers(
            ObjectFilter::creature().you_control(),
            3,
        );
        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                matching_one,
                AttackEventTarget::Player(bob),
                3,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        assert!(
            !trigger.matches(&event, &ctx),
            "an unrelated third attacker must not satisfy a three-creature filtered group"
        );
    }

    #[test]
    fn test_one_or_more_with_exact_total_attackers_requires_exact_count() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(100);
        let attacker_one = create_creature(&mut game, "A", alice);
        let attacker_two = create_creature(&mut game, "B", alice);
        let attacker_three = create_creature(&mut game, "C", alice);

        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: attacker_one,
            target: AttackTarget::Player(bob),
        });
        combat.attackers.push(AttackerInfo {
            creature: attacker_two,
            target: AttackTarget::Player(bob),
        });
        game.combat = Some(combat);

        let trigger =
            AttacksTrigger::one_or_more_with_exact_total_attackers(ObjectFilter::creature(), 2);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let below_exact = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                attacker_one,
                AttackEventTarget::Player(bob),
                1,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&below_exact, &ctx));

        let exact_first = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                attacker_one,
                AttackEventTarget::Player(bob),
                2,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&exact_first, &ctx));

        let exact_second = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                attacker_two,
                AttackEventTarget::Player(bob),
                2,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&exact_second, &ctx));

        drop(ctx);
        game.combat
            .as_mut()
            .expect("combat should exist")
            .attackers
            .push(AttackerInfo {
                creature: attacker_three,
                target: AttackTarget::Player(bob),
            });
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        let above_exact = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                attacker_one,
                AttackEventTarget::Player(bob),
                3,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&above_exact, &ctx));
    }

    #[test]
    fn test_one_or_more_event_value_counts_matching_attackers() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(100);
        let attacker_one = create_creature(&mut game, "A", alice);
        let attacker_two = create_creature(&mut game, "B", alice);
        let other_attacker = create_creature(&mut game, "C", bob);

        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo {
            creature: attacker_one,
            target: AttackTarget::Player(bob),
        });
        combat.attackers.push(AttackerInfo {
            creature: attacker_two,
            target: AttackTarget::Player(bob),
        });
        combat.attackers.push(AttackerInfo {
            creature: other_attacker,
            target: AttackTarget::Player(alice),
        });
        game.combat = Some(combat);

        let trigger = AttacksTrigger::one_or_more(ObjectFilter::creature().you_control());
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        let event = TriggerEvent::new_with_provenance(
            CreatureAttackedEvent::with_total_attackers(
                attacker_one,
                AttackEventTarget::Player(bob),
                3,
            ),
            crate::provenance::ProvNodeId::default(),
        );

        assert_eq!(trigger.event_value_amount(&event, &ctx), Some(2));
    }
}
