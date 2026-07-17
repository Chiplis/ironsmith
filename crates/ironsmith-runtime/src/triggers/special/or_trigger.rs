//! Or trigger combinator - matches if any of the inner triggers match.

use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::triggers::{
    AbilityActivatedTrigger, AttacksTrigger, CountMode, DealsDamageToTrigger, DealsDamageTrigger,
    PermanentBecomesTappedTrigger, PlayerRelation, SpellCastTrigger, ThisAttacksTrigger,
    ThisBlocksTrigger, ThisDealsDamageToTrigger, ThisDealsDamageTrigger, TransformsTrigger,
    Trigger, TriggerEvent, ZoneChangeTrigger, ZonePattern,
};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

fn object_filter_is_your_commander(filter: &ObjectFilter, card_types: &[CardType]) -> bool {
    if filter.card_types != card_types {
        return false;
    }
    let mut normalized = filter.clone();
    normalized.zone = None;
    normalized.card_types.clear();
    normalized
        == ObjectFilter {
            owner: Some(PlayerFilter::You),
            is_commander: true,
            ..Default::default()
        }
}

fn object_filter_is_plain_card_type(filter: &ObjectFilter, card_type: CardType) -> bool {
    if filter.card_types.len() != 1 || filter.card_types[0] != card_type {
        return false;
    }
    let mut normalized = filter.clone();
    normalized.zone = None;
    normalized.card_types.clear();
    normalized == ObjectFilter::default()
}

fn object_filter_is_plain_subtype(filter: &ObjectFilter, subtype: Subtype) -> bool {
    if filter.subtypes.len() != 1 || filter.subtypes[0] != subtype {
        return false;
    }
    let mut normalized = filter.clone();
    normalized.zone = None;
    normalized.subtypes.clear();
    normalized == ObjectFilter::default()
}

fn is_this_enters_battlefield_trigger(trigger: &ZoneChangeTrigger) -> bool {
    trigger.this_object
        && trigger.from == ZonePattern::Any
        && trigger.to == ZonePattern::Specific(Zone::Battlefield)
        && trigger.player == PlayerRelation::Any
        && trigger.cause_filter.is_none()
        && trigger.during_turn.is_none()
        && trigger.count_mode == CountMode::Each
}

fn is_this_dies_trigger(trigger: &ZoneChangeTrigger) -> bool {
    trigger.this_object
        && trigger.from == ZonePattern::Specific(Zone::Battlefield)
        && trigger.to == ZonePattern::Specific(Zone::Graveyard)
        && trigger.player == PlayerRelation::Any
        && trigger.cause_filter.is_none()
        && trigger.during_turn.is_none()
        && trigger.count_mode == CountMode::Each
}

fn strip_leading_article(text: &str) -> &str {
    text.strip_prefix("a ")
        .or_else(|| text.strip_prefix("an "))
        .or_else(|| text.strip_prefix("the "))
        .unwrap_or(text)
}

fn pluralize_damage_recipient(text: &str) -> String {
    let text = strip_leading_article(text);
    if text.ends_with('s') {
        return text.to_string();
    }
    if let Some(stem) = text.strip_suffix("card") {
        return format!("{stem}cards");
    }
    if let Some(stem) = text.strip_suffix('y')
        && !stem
            .chars()
            .last()
            .is_some_and(|ch| matches!(ch.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u'))
    {
        return format!("{stem}ies");
    }
    format!("{text}s")
}

fn source_or_matching_subject(filter: &ObjectFilter) -> Option<String> {
    if !filter.source {
        return Some(filter.description());
    }
    let source_text = filter.source_surface.as_ref()?.display_text();
    let mut other_filter = filter.clone();
    other_filter.source = false;
    other_filter.source_surface = None;
    let other_subject = strip_leading_article(&other_filter.description()).to_string();
    if other_subject.is_empty() {
        return None;
    }
    Some(format!("{source_text} or another {other_subject}"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CastOrActivateComponent {
    InstantOrSorcery,
    LoyaltyAbility,
}

/// A trigger that matches if any of the inner triggers match.
///
/// This is useful for cards like Tivit, Seller of Secrets which trigger
/// "whenever ~ enters the battlefield or deals combat damage to a player".
///
/// # Example
///
/// ```ignore
/// let trigger = Trigger::or(vec![
///     Trigger::this_enters_battlefield(),
///     Trigger::this_deals_combat_damage_to_player(PlayerFilter::Any),
/// ]);
/// ```
#[derive(Debug, Clone)]
pub struct OrTrigger {
    /// The inner triggers - matches if any of these match.
    pub triggers: Vec<Trigger>,
}

impl OrTrigger {
    /// Create a new OrTrigger with the given triggers.
    pub fn new(triggers: Vec<Trigger>) -> Self {
        Self { triggers }
    }

    /// Create an OrTrigger from exactly two triggers.
    pub fn two(a: Trigger, b: Trigger) -> Self {
        Self::new(vec![a, b])
    }

    fn self_attacks_or_blocks_display(&self) -> Option<String> {
        let [first, second] = self.triggers.as_slice() else {
            return None;
        };
        // Explicit inner introductions may intentionally differ. The parser
        // puts the authored introduction on the outer Or trigger, so the
        // ordinary source-scoped pair has no inner surface to erase here.
        if first.intro_surface().is_some() || second.intro_surface().is_some() {
            return None;
        }
        let action = if first.downcast_ref::<ThisAttacksTrigger>().is_some()
            && second.downcast_ref::<ThisBlocksTrigger>().is_some()
        {
            "attacks or blocks"
        } else if first.downcast_ref::<ThisBlocksTrigger>().is_some()
            && second.downcast_ref::<ThisAttacksTrigger>().is_some()
        {
            "blocks or attacks"
        } else {
            return None;
        };
        Some(format!("Whenever this creature {action}"))
    }

    fn self_enters_or_attacks_display(&self) -> Option<String> {
        let [first, second] = self.triggers.as_slice() else {
            return None;
        };
        let (zone_change, attacks_first) = if let (Some(zone_change), Some(_)) = (
            first.downcast_ref::<ZoneChangeTrigger>(),
            second.downcast_ref::<ThisAttacksTrigger>(),
        ) {
            (zone_change, false)
        } else if let (Some(_), Some(zone_change)) = (
            first.downcast_ref::<ThisAttacksTrigger>(),
            second.downcast_ref::<ZoneChangeTrigger>(),
        ) {
            (zone_change, true)
        } else {
            return None;
        };

        if !zone_change.this_object
            || zone_change.from != ZonePattern::Any
            || zone_change.to != ZonePattern::Specific(Zone::Battlefield)
            || zone_change.player != crate::triggers::PlayerRelation::Any
            || zone_change.cause_filter.is_some()
        {
            return None;
        }

        let subject = zone_change.this_subject_text("creature");
        let action = if attacks_first {
            "attacks or enters"
        } else {
            "enters or attacks"
        };
        Some(format!("Whenever {subject} {action}"))
    }

    fn self_enters_or_dies_display(&self) -> Option<String> {
        let [first, second] = self.triggers.as_slice() else {
            return None;
        };
        let (enters, dies) = if let (Some(enters), Some(dies)) = (
            first.downcast_ref::<ZoneChangeTrigger>(),
            second.downcast_ref::<ZoneChangeTrigger>(),
        ) {
            if is_this_enters_battlefield_trigger(enters) && is_this_dies_trigger(dies) {
                (enters, dies)
            } else if is_this_enters_battlefield_trigger(dies) && is_this_dies_trigger(enters) {
                (dies, enters)
            } else {
                return None;
            }
        } else {
            return None;
        };

        let subject = enters.this_subject_text("creature");
        if subject != dies.this_subject_text("creature") {
            return None;
        }
        Some(format!("When {subject} enters or dies"))
    }

    fn your_commander_enters_or_attacks_display(&self) -> Option<String> {
        let [first, second] = self.triggers.as_slice() else {
            return None;
        };
        let (zone_change, attacks, attacks_first) = if let (Some(zone_change), Some(attacks)) = (
            first.downcast_ref::<ZoneChangeTrigger>(),
            second.downcast_ref::<AttacksTrigger>(),
        ) {
            (zone_change, attacks, false)
        } else if let (Some(attacks), Some(zone_change)) = (
            first.downcast_ref::<AttacksTrigger>(),
            second.downcast_ref::<ZoneChangeTrigger>(),
        ) {
            (zone_change, attacks, true)
        } else {
            return None;
        };

        if zone_change.this_object
            || zone_change.from != ZonePattern::Any
            || zone_change.to != ZonePattern::Specific(Zone::Battlefield)
            || zone_change.player != PlayerRelation::Any
            || zone_change.cause_filter.is_some()
            || zone_change.during_turn.is_some()
            || zone_change.count_mode != CountMode::Each
            || zone_change.this_object_surface.is_some()
            || !object_filter_is_your_commander(&zone_change.object_filter, &[])
            || attacks.one_or_more
            || attacks.min_total_attackers != 1
            || attacks.max_total_attackers.is_some()
            || !object_filter_is_your_commander(&attacks.filter, &[CardType::Creature])
        {
            return None;
        }

        let action = if attacks_first {
            "attacks or enters"
        } else {
            "enters or attacks"
        };
        Some(format!("Whenever your commander {action}"))
    }

    fn self_enters_or_transforms_display(&self) -> Option<String> {
        let [first, second] = self.triggers.as_slice() else {
            return None;
        };
        let (zone_change, transforms) = if let (Some(zone_change), Some(transforms)) = (
            first.downcast_ref::<ZoneChangeTrigger>(),
            second.downcast_ref::<TransformsTrigger>(),
        ) {
            (zone_change, transforms)
        } else if let (Some(transforms), Some(zone_change)) = (
            first.downcast_ref::<TransformsTrigger>(),
            second.downcast_ref::<ZoneChangeTrigger>(),
        ) {
            (zone_change, transforms)
        } else {
            return None;
        };

        if !zone_change.this_object
            || zone_change.from != ZonePattern::Any
            || zone_change.to != ZonePattern::Specific(Zone::Battlefield)
            || zone_change.player != crate::triggers::PlayerRelation::Any
            || zone_change.cause_filter.is_some()
        {
            return None;
        }

        let subject = zone_change.this_subject_text("creature");
        let destination = transforms.destination_text();
        Some(format!(
            "Whenever {subject} enters or transforms into {destination}"
        ))
    }

    fn this_or_another_enters_display(&self) -> Option<String> {
        let [first, second] = self.triggers.as_slice() else {
            return None;
        };
        let (this_enters, another_enters) = if let (Some(first_zone), Some(second_zone)) = (
            first.downcast_ref::<ZoneChangeTrigger>(),
            second.downcast_ref::<ZoneChangeTrigger>(),
        ) {
            if first_zone.this_object && !second_zone.this_object {
                (first_zone, second_zone)
            } else if second_zone.this_object && !first_zone.this_object {
                (second_zone, first_zone)
            } else {
                return None;
            }
        } else {
            return None;
        };

        let both_enter_battlefield = |zone_change: &ZoneChangeTrigger| {
            zone_change.from == ZonePattern::Any
                && zone_change.to == ZonePattern::Specific(Zone::Battlefield)
                && zone_change.player == crate::triggers::PlayerRelation::Any
                && zone_change.cause_filter.is_none()
        };
        if !both_enter_battlefield(this_enters)
            || !both_enter_battlefield(another_enters)
            || !another_enters.object_filter.other
        {
            return None;
        }

        let this_subject = this_enters.this_subject_text("creature");
        let mut other_filter = another_enters.object_filter.clone();
        other_filter.other = false;
        let other_description = other_filter.description();
        let other_subject = other_description
            .strip_prefix("a ")
            .or_else(|| other_description.strip_prefix("an "))
            .map(str::to_string)
            .unwrap_or(other_description);
        Some(format!(
            "Whenever {this_subject} or another {other_subject} enters"
        ))
    }

    fn this_or_another_zone_change_display(&self) -> Option<String> {
        let [first, second] = self.triggers.as_slice() else {
            return None;
        };
        let first = first.downcast_ref::<ZoneChangeTrigger>()?;
        let second = second.downcast_ref::<ZoneChangeTrigger>()?;

        let is_source_branch =
            |trigger: &ZoneChangeTrigger| trigger.this_object || trigger.object_filter.source;
        let (source_change, another_change) = if is_source_branch(first)
            && !is_source_branch(second)
            && second.object_filter.other
        {
            (first, second)
        } else if is_source_branch(second) && !is_source_branch(first) && first.object_filter.other
        {
            (second, first)
        } else {
            return None;
        };

        if source_change.from != another_change.from
            || source_change.to != another_change.to
            || source_change.player != another_change.player
            || source_change.cause_filter != another_change.cause_filter
            || source_change.during_turn != another_change.during_turn
            || source_change.count_mode != CountMode::Each
            || another_change.count_mode != CountMode::Each
        {
            return None;
        }

        let source_subject = if source_change.this_object {
            source_change.this_subject_text("permanent")
        } else {
            source_change
                .object_filter
                .source_surface
                .as_ref()?
                .display_text()
        };

        let mut explicit_other = another_change.clone();
        explicit_other.object_filter.other = false;
        if source_change.this_object
            && source_change.from == ZonePattern::Specific(Zone::Battlefield)
            && source_change.to == ZonePattern::Specific(Zone::Graveyard)
        {
            let other_subject = explicit_other.object_filter.description();
            return Some(format!(
                "Whenever {source_subject} or another {} dies",
                strip_leading_article(&other_subject)
            ));
        }
        let explicit_display = explicit_other.display();
        let other_clause = explicit_display
            .strip_prefix("Whenever ")
            .or_else(|| explicit_display.strip_prefix("When "))?;
        let other_clause = strip_leading_article(other_clause);

        Some(format!(
            "Whenever {source_subject} or another {other_clause}"
        ))
    }

    fn battlefield_graveyard_or_exile_display(&self) -> Option<String> {
        let [first, second] = self.triggers.as_slice() else {
            return None;
        };
        let first = first.downcast_ref::<ZoneChangeTrigger>()?;
        let second = second.downcast_ref::<ZoneChangeTrigger>()?;

        let (graveyard, exile) = if first.from == ZonePattern::Specific(Zone::Battlefield)
            && first.to == ZonePattern::Specific(Zone::Graveyard)
            && second.from == ZonePattern::Specific(Zone::Battlefield)
            && second.to == ZonePattern::Specific(Zone::Exile)
        {
            (first, second)
        } else if second.from == ZonePattern::Specific(Zone::Battlefield)
            && second.to == ZonePattern::Specific(Zone::Graveyard)
            && first.from == ZonePattern::Specific(Zone::Battlefield)
            && first.to == ZonePattern::Specific(Zone::Exile)
        {
            (second, first)
        } else {
            return None;
        };

        if graveyard.object_filter != exile.object_filter
            || graveyard.player != exile.player
            || graveyard.cause_filter != exile.cause_filter
            || graveyard.during_turn != exile.during_turn
            || graveyard.count_mode != CountMode::Each
            || exile.count_mode != CountMode::Each
            || graveyard.this_object != exile.this_object
            || graveyard.this_object_surface != exile.this_object_surface
        {
            return None;
        }

        let subject = if graveyard.this_object {
            graveyard.this_subject_text("permanent")
        } else {
            source_or_matching_subject(&graveyard.object_filter)?
        };
        Some(format!(
            "Whenever {subject} is put into a graveyard from the battlefield or is put into exile from the battlefield"
        ))
    }

    fn collect_you_cast_or_activate_components(
        trigger: &Trigger,
        components: &mut Vec<CastOrActivateComponent>,
    ) -> bool {
        if let Some(or_trigger) = trigger.downcast_ref::<OrTrigger>() {
            return or_trigger
                .triggers
                .iter()
                .all(|inner| Self::collect_you_cast_or_activate_components(inner, components));
        }
        if let Some(spell) = trigger.downcast_ref::<SpellCastTrigger>() {
            if spell.caster == PlayerFilter::You
                && spell.during_turn.is_none()
                && spell.min_spells_this_turn.is_none()
                && spell.exact_spells_this_turn.is_none()
                && !spell.from_not_hand
                && spell
                    .filter
                    .as_ref()
                    .is_some_and(|filter| *filter == ObjectFilter::instant_or_sorcery())
            {
                components.push(CastOrActivateComponent::InstantOrSorcery);
                return true;
            }
            return false;
        }
        if let Some(ability) = trigger.downcast_ref::<AbilityActivatedTrigger>() {
            if ability.activator == PlayerFilter::You
                && ability.filter == ObjectFilter::default()
                && !ability.non_mana_only
                && ability.loyalty_only
                && ability.activation_cost_has_tap.is_none()
            {
                components.push(CastOrActivateComponent::LoyaltyAbility);
                return true;
            }
        }
        false
    }

    fn you_cast_or_activate_display(&self) -> Option<String> {
        let mut components = Vec::new();
        for trigger in &self.triggers {
            if !Self::collect_you_cast_or_activate_components(trigger, &mut components) {
                return None;
            }
        }
        components.sort_by_key(|component| match component {
            CastOrActivateComponent::InstantOrSorcery => 0,
            CastOrActivateComponent::LoyaltyAbility => 1,
        });
        components.dedup();

        if components.len() < 2
            || !components.contains(&CastOrActivateComponent::InstantOrSorcery)
            || !components.contains(&CastOrActivateComponent::LoyaltyAbility)
        {
            return None;
        }

        Some(
            "Whenever you cast an instant spell, cast a sorcery spell, or activate a loyalty ability"
                .to_string(),
        )
    }

    fn spell_or_activated_ability_x_cost_display(&self) -> Option<String> {
        let [first, second] = self.triggers.as_slice() else {
            return None;
        };
        let (spell, ability) = if let (Some(spell), Some(ability)) = (
            first.downcast_ref::<SpellCastTrigger>(),
            second.downcast_ref::<AbilityActivatedTrigger>(),
        ) {
            (spell, ability)
        } else if let (Some(ability), Some(spell)) = (
            first.downcast_ref::<AbilityActivatedTrigger>(),
            second.downcast_ref::<SpellCastTrigger>(),
        ) {
            (spell, ability)
        } else {
            return None;
        };

        let Some(spell_filter) = spell.filter.as_ref() else {
            return None;
        };
        let mut spell_filter_without_x = spell_filter.clone();
        spell_filter_without_x.has_x_in_cost = false;
        let mut ability_filter_without_x = ability.filter.clone();
        ability_filter_without_x.has_x_in_cost = false;

        if spell.caster == PlayerFilter::You
            && spell.during_turn.is_none()
            && spell.min_spells_this_turn.is_none()
            && spell.exact_spells_this_turn.is_none()
            && !spell.from_not_hand
            && spell_filter.has_x_in_cost
            && spell_filter_without_x == ObjectFilter::instant_or_sorcery()
            && ability.activator == PlayerFilter::You
            && ability.filter.has_x_in_cost
            && ability_filter_without_x == ObjectFilter::default()
            && !ability.non_mana_only
            && !ability.loyalty_only
        {
            return Some(
                "Whenever you cast an instant or sorcery spell or activate an ability, if that spell's mana cost or that ability's activation cost contains {X}"
                    .to_string(),
            );
        }
        None
    }

    fn artifact_tapped_or_artifact_ability_without_tap_cost_display(&self) -> Option<String> {
        let [first, second] = self.triggers.as_slice() else {
            return None;
        };
        let (tapped, ability, tapped_first) = if let (Some(tapped), Some(ability)) = (
            first.downcast_ref::<PermanentBecomesTappedTrigger>(),
            second.downcast_ref::<AbilityActivatedTrigger>(),
        ) {
            (tapped, ability, true)
        } else if let (Some(ability), Some(tapped)) = (
            first.downcast_ref::<AbilityActivatedTrigger>(),
            second.downcast_ref::<PermanentBecomesTappedTrigger>(),
        ) {
            (tapped, ability, false)
        } else {
            return None;
        };

        if !object_filter_is_plain_card_type(&tapped.filter, CardType::Artifact)
            || ability.activator != PlayerFilter::Any
            || !object_filter_is_plain_card_type(&ability.filter, CardType::Artifact)
            || ability.non_mana_only
            || ability.loyalty_only
            || ability.activation_cost_has_tap != Some(false)
        {
            return None;
        }

        if tapped_first {
            Some(
                "Whenever an artifact becomes tapped or a player activates an artifact's ability without {T} in its activation cost"
                    .to_string(),
            )
        } else {
            Some(
                "Whenever a player activates an artifact's ability without {T} in its activation cost or an artifact becomes tapped"
                    .to_string(),
            )
        }
    }

    fn source_saddles_mount_or_crews_vehicle_display(&self) -> Option<String> {
        let [first, second] = self.triggers.as_slice() else {
            return None;
        };
        let first = first.downcast_ref::<crate::triggers::KeywordActionTrigger>()?;
        let second = second.downcast_ref::<crate::triggers::KeywordActionTrigger>()?;
        let (saddle, crew) = match (first.action, second.action) {
            (crate::events::KeywordActionKind::Saddle, crate::events::KeywordActionKind::Crew) => {
                (first, second)
            }
            (crate::events::KeywordActionKind::Crew, crate::events::KeywordActionKind::Saddle) => {
                (second, first)
            }
            _ => return None,
        };
        if saddle.player != crew.player
            || saddle.source_filter != crew.source_filter
            || saddle.during_your_main_phase != crew.during_your_main_phase
        {
            return None;
        }
        let (_, mount_filter) = saddle.tagged_object_filter.as_ref()?;
        let (_, vehicle_filter) = crew.tagged_object_filter.as_ref()?;
        if !object_filter_is_plain_subtype(mount_filter, Subtype::Mount)
            || !object_filter_is_plain_subtype(vehicle_filter, Subtype::Vehicle)
        {
            return None;
        }
        let source = saddle.source_filter.as_ref()?.description();
        let suffix = if saddle.during_your_main_phase {
            " during your main phase"
        } else {
            ""
        };
        Some(format!(
            "Whenever {source} saddles a Mount or crews a Vehicle{suffix}"
        ))
    }

    fn damage_to_player_or_object_display(&self) -> Option<String> {
        let [first, second] = self.triggers.as_slice() else {
            return None;
        };

        if let Some((object, player, player_first)) = (|| {
            if let (Some(object), Some(player)) = (
                first.downcast_ref::<DealsDamageToTrigger>(),
                second.downcast_ref::<DealsDamageTrigger>(),
            ) {
                return Some((object, player, false));
            }
            if let (Some(player), Some(object)) = (
                first.downcast_ref::<DealsDamageTrigger>(),
                second.downcast_ref::<DealsDamageToTrigger>(),
            ) {
                return Some((object, player, true));
            }
            None
        })() {
            if object.combat_only != player.combat_only
                || player.noncombat_only
                || object.source_filter != player.filter
                || object.target_filter.union_connective()
                    != crate::filter::ObjectFilterUnionConnective::AndOr
            {
                return None;
            }
            let damaged_player = player.damaged_player.as_ref()?;
            let object_display = object.display();
            let (prefix, object_recipient) = object_display.rsplit_once(" to ")?;
            let player_recipient = damaged_player.description();
            let one_or_more = object.target_filter.union_is_one_or_more();
            let object_recipient = if one_or_more {
                pluralize_damage_recipient(object_recipient)
            } else {
                object_recipient.to_string()
            };
            let player_recipient = if one_or_more {
                pluralize_damage_recipient(&player_recipient)
            } else {
                player_recipient
            };
            let recipients = if player_first {
                format!("{player_recipient} and/or {object_recipient}")
            } else {
                format!("{object_recipient} and/or {player_recipient}")
            };
            let quantifier = if one_or_more { "one or more " } else { "" };
            return Some(format!("{prefix} to {quantifier}{recipients}"));
        }

        let (object, player, player_first) = if let (Some(object), Some(player)) = (
            first.downcast_ref::<ThisDealsDamageToTrigger>(),
            second.downcast_ref::<ThisDealsDamageTrigger>(),
        ) {
            (object, player, false)
        } else if let (Some(player), Some(object)) = (
            first.downcast_ref::<ThisDealsDamageTrigger>(),
            second.downcast_ref::<ThisDealsDamageToTrigger>(),
        ) {
            (object, player, true)
        } else {
            return None;
        };
        if object.combat_only != player.combat_only
            || player.amount.is_some()
            || object.target_filter.union_connective()
                != crate::filter::ObjectFilterUnionConnective::AndOr
        {
            return None;
        }
        let damaged_player = player.damaged_player.as_ref()?;
        let object_display = object.display();
        let (prefix, object_recipient) = object_display.rsplit_once(" to ")?;
        let player_recipient = damaged_player.description();
        let one_or_more = object.target_filter.union_is_one_or_more();
        let object_recipient = if one_or_more {
            pluralize_damage_recipient(object_recipient)
        } else {
            object_recipient.to_string()
        };
        let player_recipient = if one_or_more {
            pluralize_damage_recipient(&player_recipient)
        } else {
            player_recipient
        };
        let recipients = if player_first {
            format!("{player_recipient} and/or {object_recipient}")
        } else {
            format!("{object_recipient} and/or {player_recipient}")
        };
        let quantifier = if one_or_more { "one or more " } else { "" };
        Some(format!("{prefix} to {quantifier}{recipients}"))
    }
}

impl TriggerMatcher for OrTrigger {
    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }

    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        self.triggers.iter().any(|t| t.matches(event, ctx))
    }

    fn trigger_count(&self, event: &TriggerEvent) -> u32 {
        self.triggers
            .iter()
            .map(|trigger| trigger.trigger_count(event))
            .max()
            .unwrap_or(0)
    }

    fn trigger_count_with_context(&self, event: &TriggerEvent, ctx: &TriggerContext) -> u32 {
        self.triggers
            .iter()
            .filter(|trigger| trigger.matches(event, ctx))
            .map(|trigger| trigger.trigger_count_with_context(event, ctx))
            .max()
            .unwrap_or(0)
    }

    fn subscribed_kinds(&self) -> Option<Vec<crate::events::EventKind>> {
        let mut kinds = Vec::new();
        for trigger in &self.triggers {
            for kind in trigger.subscribed_kinds()? {
                if !kinds.contains(&kind) {
                    kinds.push(kind);
                }
            }
        }
        Some(kinds)
    }

    fn source_must_match_event_object(&self, event_kind: crate::events::EventKind) -> bool {
        let mut has_relevant_trigger = false;
        for trigger in &self.triggers {
            let Some(kinds) = trigger.subscribed_kinds() else {
                return false;
            };
            if !kinds.contains(&event_kind) {
                continue;
            }
            has_relevant_trigger = true;
            if !trigger.source_must_match_event_object(event_kind) {
                return false;
            }
        }
        has_relevant_trigger
    }

    fn display(&self) -> String {
        if self.triggers.is_empty() {
            return "never".to_string();
        }
        if self.triggers.len() == 1 {
            return self.triggers[0].display();
        }
        if let Some(display) = self.self_attacks_or_blocks_display() {
            return display;
        }
        if let Some(display) = self.self_enters_or_attacks_display() {
            return display;
        }
        if let Some(display) = self.self_enters_or_dies_display() {
            return display;
        }
        if let Some(display) = self.your_commander_enters_or_attacks_display() {
            return display;
        }
        if let Some(display) = self.self_enters_or_transforms_display() {
            return display;
        }
        if let Some(display) = self.this_or_another_enters_display() {
            return display;
        }
        if let Some(display) = self.this_or_another_zone_change_display() {
            return display;
        }
        if let Some(display) = self.battlefield_graveyard_or_exile_display() {
            return display;
        }
        if let Some(display) = self.you_cast_or_activate_display() {
            return display;
        }
        if let Some(display) = self.spell_or_activated_ability_x_cost_display() {
            return display;
        }
        if let Some(display) = self.artifact_tapped_or_artifact_ability_without_tap_cost_display() {
            return display;
        }
        if let Some(display) = self.source_saddles_mount_or_crews_vehicle_display() {
            return display;
        }
        if let Some(display) = self.damage_to_player_or_object_display() {
            return display;
        }
        let displays: Vec<String> = self.triggers.iter().map(|t| t.display()).collect();
        let mut parts = vec![displays[0].clone()];
        for (idx, d) in displays[1..].iter().enumerate() {
            let first_intro = displays[0]
                .strip_prefix("When ")
                .map(|_| "When")
                .or_else(|| displays[0].strip_prefix("Whenever ").map(|_| "Whenever"));
            let next_intro = d
                .strip_prefix("When ")
                .map(|_| "When")
                .or_else(|| d.strip_prefix("Whenever ").map(|_| "Whenever"));
            if self.triggers[idx + 1].intro_surface().is_some()
                && first_intro.is_some()
                && next_intro.is_some()
                && first_intro != next_intro
            {
                let mut chars = d.chars();
                let lowered = chars
                    .next()
                    .map(|first| first.to_lowercase().collect::<String>() + chars.as_str())
                    .unwrap_or_default();
                parts.push(lowered);
            } else {
                let stripped = d
                    .strip_prefix("When ")
                    .or_else(|| d.strip_prefix("Whenever "))
                    .unwrap_or(d);
                parts.push(stripped.to_string());
            }
        }
        let joiner = if parts
            .iter()
            .skip(1)
            .any(|part| part.starts_with("when ") || part.starts_with("whenever "))
        {
            " and "
        } else {
            " or "
        };
        parts.join(joiner)
    }

    fn uses_snapshot(&self) -> bool {
        // Use snapshot if any inner trigger uses snapshot
        self.triggers.iter().any(|t| t.uses_snapshot())
    }

    fn looks_back_for_source(&self, event: &TriggerEvent) -> bool {
        self.triggers
            .iter()
            .any(|trigger| trigger.looks_back_for_source(event))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::DamageEvent;
    use crate::events::DamageTarget;
    use crate::events::zones::ZoneChangeEvent;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};
    use crate::triggers::ThisDealsCombatDamageToPlayerTrigger;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn etb_event(source_id: ObjectId) -> ZoneChangeEvent {
        ZoneChangeEvent::with_cause(
            source_id,
            Zone::Hand,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            None,
        )
    }

    #[test]
    fn display_compacts_your_commander_enters_or_attacks() {
        let enters = Trigger::enters_battlefield(
            ObjectFilter::default()
                .commander()
                .owned_by(PlayerFilter::You),
            None,
        );
        let attacks = Trigger::attacks(
            ObjectFilter::creature()
                .commander()
                .owned_by(PlayerFilter::You),
        );

        let trigger = Trigger::or(vec![enters, attacks])
            .with_intro_surface(crate::triggers::TriggerIntroSurface::Whenever);

        assert_eq!(
            trigger.display(),
            "Whenever your commander enters or attacks"
        );
    }

    #[test]
    fn display_compacts_this_creature_attacks_or_blocks_with_outer_intro() {
        let trigger = Trigger::or(vec![Trigger::this_attacks(), Trigger::this_blocks()])
            .with_intro_surface(crate::triggers::TriggerIntroSurface::When);
        assert_eq!(trigger.display(), "When this creature attacks or blocks");

        let reversed = OrTrigger::two(Trigger::this_blocks(), Trigger::this_attacks());
        assert_eq!(
            reversed.display(),
            "Whenever this creature blocks or attacks"
        );
    }

    #[test]
    fn display_does_not_compact_mixed_inner_introductions() {
        let trigger = OrTrigger::two(
            Trigger::this_attacks().with_intro_surface(crate::triggers::TriggerIntroSurface::When),
            Trigger::this_blocks()
                .with_intro_surface(crate::triggers::TriggerIntroSurface::Whenever),
        );
        let display = trigger.display();
        assert_ne!(display, "Whenever this creature attacks or blocks");
        assert!(display.contains("this creature attacks"), "{display}");
        assert!(display.contains("this creature blocks"), "{display}");
    }

    #[test]
    fn display_compacts_this_creature_enters_or_dies() {
        let enters =
            Trigger::new(ZoneChangeTrigger::enters_battlefield(ObjectFilter::creature()).this());
        let trigger = OrTrigger::two(enters, Trigger::this_dies());

        assert_eq!(trigger.display(), "When this creature enters or dies");
    }

    #[test]
    fn display_compacts_this_or_another_creature_dies() {
        let trigger = OrTrigger::two(
            Trigger::this_dies(),
            Trigger::new(ZoneChangeTrigger::dies(ObjectFilter::creature().other())),
        );

        assert_eq!(
            trigger.display(),
            "Whenever this creature or another creature dies"
        );
    }

    #[test]
    fn display_compacts_and_or_damage_recipients_without_losing_players() {
        let source = ObjectFilter::default()
            .with_colors(crate::color::ColorSet::RED)
            .controlled_by(PlayerFilter::You);
        let mut target = ObjectFilter::default().in_zone(Zone::Battlefield);
        target.set_union_connective(crate::filter::ObjectFilterUnionConnective::AndOr);
        target.set_union_one_or_more(true);

        let object_branch = Trigger::deals_damage_to_with_source_surface(
            source.clone(),
            target,
            ironsmith_core::trigger_model::DamageSourceSurface::Source,
        );
        let mut player_matcher = DealsDamageTrigger::new(source);
        player_matcher.damaged_player = Some(PlayerFilter::Any);
        let trigger = OrTrigger::two(object_branch, Trigger::new(player_matcher));

        assert_eq!(
            trigger.display(),
            "Whenever a red source you control deals damage to one or more permanents and/or players"
        );
    }

    #[test]
    fn and_or_damage_recipient_union_matches_the_player_branch() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);
        let mut object_filter = ObjectFilter::default().in_zone(Zone::Battlefield);
        object_filter.set_union_connective(crate::filter::ObjectFilterUnionConnective::AndOr);
        let trigger = OrTrigger::two(
            Trigger::this_deals_damage_to(object_filter),
            Trigger::this_deals_damage_to_player(PlayerFilter::Any, None),
        );
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        let event = TriggerEvent::new_with_provenance(
            DamageEvent::with_cause(
                source_id,
                DamageTarget::Player(bob),
                1,
                false,
                crate::events::cause::EventCause::effect(),
            ),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn display_compacts_this_or_another_graveyard_from_battlefield() {
        let source = ObjectFilter::source_with_surface(
            crate::target::SourceReferenceSurface::ThisPermanentType(
                "this enchantment".to_string(),
            ),
        );
        let trigger = OrTrigger::two(
            Trigger::new(
                ZoneChangeTrigger::new()
                    .from(Zone::Battlefield)
                    .to(Zone::Graveyard)
                    .filter(source),
            ),
            Trigger::new(
                ZoneChangeTrigger::new()
                    .from(Zone::Battlefield)
                    .to(Zone::Graveyard)
                    .filter(ObjectFilter::nonland_permanent().you_control().other()),
            ),
        );

        assert_eq!(
            trigger.display(),
            "Whenever this enchantment or another nonland permanent you control is put into a graveyard from the battlefield"
        );
    }

    #[test]
    fn display_compacts_source_or_another_artifact_graveyard_or_exile_from_battlefield() {
        let mut filter = ObjectFilter::source_with_surface(
            crate::target::SourceReferenceSurface::ThisPermanentType("this artifact".to_string()),
        )
        .nontoken()
        .controlled_by(PlayerFilter::You);
        filter.card_types = vec![CardType::Artifact];

        let graveyard = Trigger::new(
            ZoneChangeTrigger::new()
                .from(Zone::Battlefield)
                .to(Zone::Graveyard)
                .filter(filter.clone()),
        );
        let exile = Trigger::new(
            ZoneChangeTrigger::new()
                .from(Zone::Battlefield)
                .to(Zone::Exile)
                .filter(filter),
        );
        let trigger = OrTrigger::two(graveyard, exile);

        assert_eq!(
            trigger.display(),
            "Whenever this artifact or another nontoken artifact you control is put into a graveyard from the battlefield or is put into exile from the battlefield"
        );
    }

    #[test]
    fn display_compacts_you_cast_instant_sorcery_or_activate_loyalty() {
        let trigger = OrTrigger::two(
            Trigger::new(SpellCastTrigger::new(
                Some(ObjectFilter::instant_or_sorcery()),
                PlayerFilter::You,
            )),
            Trigger::new(
                AbilityActivatedTrigger::new(PlayerFilter::You, ObjectFilter::default(), false)
                    .loyalty_only(true),
            ),
        );

        assert_eq!(
            trigger.display(),
            "Whenever you cast an instant spell, cast a sorcery spell, or activate a loyalty ability"
        );
    }

    #[test]
    fn display_compacts_source_saddles_mount_or_crews_vehicle_during_main_phase() {
        let source_filter = ObjectFilter::source_with_surface(
            crate::target::SourceReferenceSurface::ThisPermanentType("this creature".to_string()),
        );
        let saddles_mount =
            Trigger::keyword_action_matching_source_and_tagged_object_during_your_main_phase(
                crate::events::KeywordActionKind::Saddle,
                PlayerFilter::Any,
                source_filter.clone(),
                crate::tag::TagKey::from("__it__"),
                ObjectFilter::default().with_subtype(Subtype::Mount),
            );
        let crews_vehicle =
            Trigger::keyword_action_matching_source_and_tagged_object_during_your_main_phase(
                crate::events::KeywordActionKind::Crew,
                PlayerFilter::Any,
                source_filter,
                crate::tag::TagKey::from("__it__"),
                ObjectFilter::default().with_subtype(Subtype::Vehicle),
            );

        let trigger = Trigger::or(vec![saddles_mount, crews_vehicle])
            .with_intro_surface(crate::triggers::TriggerIntroSurface::Whenever);

        assert_eq!(
            trigger.display(),
            "Whenever this creature saddles a Mount or crews a Vehicle during your main phase"
        );
    }

    #[test]
    fn test_or_trigger_matches_first() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let trigger = OrTrigger::two(
            Trigger::this_enters_battlefield(),
            Trigger::new(ThisDealsCombatDamageToPlayerTrigger::new(
                crate::target::PlayerFilter::Any,
            )),
        );
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        // ETB event should match
        let etb_event = TriggerEvent::new_with_provenance(
            etb_event(source_id),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&etb_event, &ctx));
    }

    #[test]
    fn test_or_trigger_matches_second() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);

        let trigger = OrTrigger::two(
            Trigger::this_enters_battlefield(),
            Trigger::new(ThisDealsCombatDamageToPlayerTrigger::new(
                crate::target::PlayerFilter::Any,
            )),
        );
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        // Combat damage event should match
        let damage_event = TriggerEvent::new_with_provenance(
            DamageEvent::with_cause(
                source_id,
                DamageTarget::Player(bob),
                3,
                true, // is_combat
                crate::events::cause::EventCause::combat_damage(source_id),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&damage_event, &ctx));
    }

    #[test]
    fn this_or_another_dies_other_branch_excludes_source() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);
        let trigger = OrTrigger::two(
            Trigger::this_dies(),
            Trigger::new(ZoneChangeTrigger::dies(ObjectFilter::creature().other())),
        );
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let mut snapshot = crate::snapshot::ObjectSnapshot::for_testing(source_id, alice, "Source");
        snapshot.card_types = vec![CardType::Creature];
        snapshot.zone = Zone::Battlefield;
        let event = TriggerEvent::new_with_provenance(
            ZoneChangeEvent::with_cause(
                source_id,
                Zone::Battlefield,
                Zone::Graveyard,
                crate::events::cause::EventCause::from_sba(),
                Some(snapshot),
            ),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(trigger.triggers[0].matches(&event, &ctx));
        assert!(
            !trigger.triggers[1].matches(&event, &ctx),
            "the another-creature branch must not also match the source death"
        );
        assert_eq!(trigger.trigger_count_with_context(&event, &ctx), 1);
    }

    #[test]
    fn test_or_trigger_no_match() {
        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);
        let other_id = ObjectId::from_raw(2);

        let trigger = OrTrigger::two(
            Trigger::this_enters_battlefield(),
            Trigger::new(ThisDealsCombatDamageToPlayerTrigger::new(
                crate::target::PlayerFilter::Any,
            )),
        );
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        // Non-combat damage from source shouldn't match
        let damage_event = TriggerEvent::new_with_provenance(
            DamageEvent::with_cause(
                source_id,
                DamageTarget::Player(bob),
                3,
                false, // not combat
                crate::events::cause::EventCause::effect(),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&damage_event, &ctx));

        // ETB of different object shouldn't match
        let etb_event = TriggerEvent::new_with_provenance(
            etb_event(other_id),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&etb_event, &ctx));
    }

    #[test]
    fn test_or_trigger_display() {
        let trigger = OrTrigger::two(
            Trigger::this_enters_battlefield(),
            Trigger::new(ThisDealsCombatDamageToPlayerTrigger::new(
                crate::target::PlayerFilter::Any,
            )),
        );

        let display = trigger.display();
        assert!(display.contains("enters the battlefield"));
        assert!(display.contains("or"));
        assert!(display.contains("deals combat damage"));
    }

    #[test]
    fn test_or_trigger_empty() {
        let trigger = OrTrigger::new(vec![]);
        assert_eq!(trigger.display(), "never");

        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        let event = TriggerEvent::new_with_provenance(
            etb_event(source_id),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_or_trigger_single() {
        let trigger = OrTrigger::new(vec![Trigger::this_enters_battlefield()]);
        assert_eq!(
            trigger.display(),
            "When this permanent enters the battlefield"
        );
    }
}
