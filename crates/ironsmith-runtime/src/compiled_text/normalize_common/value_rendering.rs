use super::*;
use ironsmith_core::TurnHistoryCount;

pub(crate) fn is_bracketed_loyalty_activation_cost(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed == "0" {
        return true;
    }
    let amount = trimmed
        .strip_prefix('+')
        .or_else(|| trimmed.strip_prefix('-'))
        .or_else(|| trimmed.strip_prefix('\u{2212}'))
        .unwrap_or(trimmed);
    !amount.is_empty() && (amount == "X" || amount.chars().all(|ch| ch.is_ascii_digit()))
}

pub(crate) fn strip_parenthetical_segments(text: &str) -> String {
    if !text.contains('(') {
        return text.trim().to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut depth = 0usize;
    for ch in text.chars() {
        if ch == '(' {
            depth += 1;
            continue;
        }
        if ch == ')' {
            depth = depth.saturating_sub(1);
            continue;
        }
        if depth == 0 {
            out.push(ch);
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn additionalize_card_count_phrase(phrase: &str) -> String {
    if let Some(rest) = phrase.strip_prefix("a card") {
        return format!("an additional card{rest}");
    }
    if let Some(rest) = phrase.strip_prefix("cards") {
        return format!("additional cards{rest}");
    }
    if let Some(card_index) = phrase.find(" cards") {
        return format!(
            "{} additional{}",
            &phrase[..card_index],
            &phrase[card_index..]
        );
    }
    format!("additional {phrase}")
}

pub(crate) fn describe_card_count(value: &Value) -> String {
    if value.has_surface_hint(ironsmith_core::ValueSurfaceHint::OpponentsDealtDamageThisWay) {
        return "cards equal to the number of opponents dealt damage this way".to_string();
    }
    if value.has_surface_hint(ironsmith_core::ValueSurfaceHint::AsManyCardsThisWay) {
        return "as many cards as they discarded this way".to_string();
    }
    if value.has_surface_hint(ironsmith_core::ValueSurfaceHint::ThatManyCards) {
        return "that many cards".to_string();
    }
    if value.has_surface_hint(ironsmith_core::ValueSurfaceHint::AdditionalCards) {
        let count = value
            .clone()
            .without_surface_hint(ironsmith_core::ValueSurfaceHint::AdditionalCards);
        return additionalize_card_count_phrase(&describe_card_count(&count));
    }
    match value {
        Value::Fixed(1) => "a card".to_string(),
        Value::Fixed(n) => {
            if *n >= 0 {
                let n_u32 = *n as u32;
                if let Some(word) = small_number_word(n_u32) {
                    return format!("{word} cards");
                }
            }
            format!("{n} cards")
        }
        Value::CardTypesAmong(_) | Value::CardTypesInGraveyard(_) => {
            format!("X cards, where X is {}", describe_value(value))
        }
        value if value.has_surface_hint(ironsmith_core::ValueSurfaceHint::Difference) => {
            "cards equal to the difference".to_string()
        }
        value
            if value.has_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo)
                && describe_effect_count_backref(value).is_none() =>
        {
            let amount = value
                .clone()
                .without_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo);
            format!("cards equal to {}", describe_value(&amount))
        }
        _ => {
            if let Some(backref) = describe_effect_count_backref(value) {
                format!("{backref} cards")
            } else {
                let value_text = describe_value(value);
                if value_text_describes_card_count(&value_text) {
                    value_text
                } else {
                    format!("{value_text} cards")
                }
            }
        }
    }
}

pub(crate) fn counters_removed_this_way_multiplier(value: &Value) -> Option<i32> {
    match value {
        Value::SurfaceHinted { value, hints }
            if hints.contains(&ironsmith_core::ValueSurfaceHint::CountersRemovedThisWay)
                || hints.contains(&ironsmith_core::ValueSurfaceHint::CountersRemoved) =>
        {
            match value.unhinted() {
                Value::Scaled(inner, multiplier) => {
                    counters_removed_this_way_multiplier(inner).map(|value| value * multiplier)
                }
                _ => Some(1),
            }
        }
        Value::SurfaceHinted { value, .. } => counters_removed_this_way_multiplier(value),
        Value::Add(left, right) => Some(
            counters_removed_this_way_multiplier(left)?
                + counters_removed_this_way_multiplier(right)?,
        ),
        Value::Scaled(inner, multiplier) => {
            counters_removed_this_way_multiplier(inner).map(|value| value * multiplier)
        }
        _ => None,
    }
}

pub(crate) fn value_text_describes_card_count(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower == "a card"
        || lower.ends_with(" card")
        || lower.ends_with(" cards")
        || lower.contains("number of cards")
        || lower.contains("cards a player")
        || lower.contains("cards that player")
}

/// Return the authored additional-cost object surface when a discard count
/// and its card filter name the same complete color-matched set.
///
/// This is a semantic shape check, not a text normalization: `Value::Count`
/// makes the number of cards to discard equal to the number of eligible cards
/// in the affected player's hand, so every eligible card must be discarded.
pub(crate) fn additional_cost_color_discard_surface(
    value: &Value,
    filter: Option<&ObjectFilter>,
) -> Option<ironsmith_core::AdditionalCostObjectSurface> {
    let filter = filter?;
    let Value::Count(count_filter) = value.unhinted() else {
        return None;
    };
    if count_filter != filter
        || filter.tagged_constraints.len() != 1
        || filter.tagged_constraints[0].relation
            != crate::filter::TaggedOpbjectRelation::SharesColorWithTagged
    {
        return None;
    }

    let surface = filter.additional_cost_object_surface()?;
    let mut semantic_rest = filter.clone();
    semantic_rest.zone = None;
    semantic_rest.owner = None;
    semantic_rest.controller = None;
    semantic_rest.tagged_constraints.clear();
    semantic_rest.set_additional_cost_object_surface(None);
    (semantic_rest == ObjectFilter::default()).then_some(surface)
}

pub(crate) fn describe_discard_count(value: &Value, filter: Option<&ObjectFilter>) -> String {
    if value.has_surface_hint(ValueSurfaceHint::ForEach)
        && let Value::PriorEffectMetric { query, .. } | Value::PendingPriorEffectMetric(query) =
            value.unhinted()
        && query.metric == crate::effect::EffectMetric::Count
    {
        let card_phrase = filter
            .map(describe_discard_card_phrase)
            .unwrap_or_else(|| "card".to_string());
        return format!(
            "a {card_phrase} for each {}",
            describe_prior_effect_metric_basis(query, false)
        );
    }

    let Some(filter) = filter else {
        return match value.unhinted() {
            Value::BasicLandTypesAmong(filter) => {
                format!(
                    "a card for each basic land type among {}",
                    describe_basic_land_type_scope(filter)
                )
            }
            Value::ColorsAmong(filter) => {
                format!("a card for each {}", describe_colors_among(filter))
            }
            Value::SourcePower
            | Value::SourceToughness
            | Value::PowerOf(_)
            | Value::ToughnessOf(_)
            | Value::ManaValueOf(_) => {
                format!("a number of cards equal to {}", describe_value(value))
            }
            _ => describe_card_count(value),
        };
    };

    if filter.source {
        return match value {
            Value::Fixed(1) => "this card".to_string(),
            _ => describe_card_count(value),
        };
    }

    if filter_is_only_same_mana_value_as_triggering_spell(filter) {
        return match value {
            Value::Fixed(1) => "a card with that spell's mana value".to_string(),
            Value::Fixed(n) if *n >= 0 => {
                let plural = "cards with that spell's mana value";
                small_number_word(*n as u32)
                    .map(|word| format!("{word} {plural}"))
                    .unwrap_or_else(|| format!("{n} {plural}"))
            }
            Value::Count(count_filter)
                if filter_is_only_same_mana_value_as_triggering_spell(count_filter) =>
            {
                "all cards with that spell's mana value".to_string()
            }
            _ => format!(
                "{} cards with that spell's mana value",
                describe_value(value)
            ),
        };
    }

    if let Some(surface) = additional_cost_color_discard_surface(value, Some(filter)) {
        return format!("all cards of each of {}'s colors", surface.description());
    }

    if !filter.tagged_constraints.is_empty() {
        return match value {
            Value::Fixed(1) => "that card".to_string(),
            _ => "those cards".to_string(),
        };
    }

    if let Value::Count(count_filter) = value {
        // Discarding as many matching cards as there are matching cards in
        // hand is a mandatory "discard all" — render the oracle idiom. The
        // discard player already scopes the hand, so ignore owner scoping
        // when comparing the counted set against the discarded set.
        let mut count_bare = count_filter.clone();
        count_bare.owner = None;
        let mut filter_bare = filter.clone();
        filter_bare.owner = None;
        if count_bare == filter_bare {
            return format!(
                "all {}",
                render_effects::pluralize_noun_phrase(&describe_discard_card_phrase(filter))
            );
        }
        if count_filter.zone == Some(Zone::Hand) && count_filter.owner.is_some() {
            return describe_value(value);
        }
    }

    let card_phrase = describe_discard_card_phrase(filter);
    let plural_card_phrase = pluralize_discard_card_phrase(&card_phrase);
    match value {
        Value::Fixed(1) => format!("a {card_phrase}"),
        Value::Fixed(n) => {
            if *n >= 0 {
                let n_u32 = *n as u32;
                if let Some(word) = small_number_word(n_u32) {
                    return format!("{word} {plural_card_phrase}");
                }
            }
            format!("{n} {plural_card_phrase}")
        }
        _ => {
            if let Some(backref) = describe_effect_count_backref(value) {
                format!("{backref} {plural_card_phrase}")
            } else {
                let value_text = describe_value(value);
                if value_text_describes_card_count(&value_text) {
                    value_text
                } else {
                    format!("{value_text} {plural_card_phrase}")
                }
            }
        }
    }
}

pub(crate) fn filter_is_only_same_mana_value_as_triggering_spell(filter: &ObjectFilter) -> bool {
    let mut bare = filter.clone();
    bare.zone = None;
    bare.owner = None;
    bare.controller = None;

    let tagged_constraints_before = bare.tagged_constraints.len();
    bare.tagged_constraints.retain(|constraint| {
        !(constraint.tag.as_str() == "triggering"
            && constraint.relation == crate::filter::TaggedOpbjectRelation::SameManaValueAsTagged)
    });

    tagged_constraints_before != bare.tagged_constraints.len() && bare == ObjectFilter::default()
}

pub(crate) fn describe_discard_card_phrase(filter: &ObjectFilter) -> String {
    let mut bare = filter.clone();
    bare.controller = None;
    bare.owner = None;
    bare.targets_player = None;
    bare.targets_object = None;
    bare.tagged_constraints.clear();

    let mut phrase = strip_indefinite_article(&bare.description()).to_string();
    if let Some(stripped) = phrase.strip_suffix(" in hand") {
        phrase = stripped.to_string();
    }
    if phrase.is_empty() || phrase == "object" || phrase == "objects" {
        return "card".to_string();
    }
    if !phrase.contains("card") {
        phrase.push_str(" card");
    }
    phrase
}

pub(crate) fn pluralize_discard_card_phrase(phrase: &str) -> String {
    if phrase.ends_with('s') {
        phrase.to_string()
    } else {
        format!("{phrase}s")
    }
}

pub(crate) fn describe_effect_count_backref(value: &Value) -> Option<String> {
    match value {
        Value::SurfaceHinted { hints, .. }
            if hints.contains(&ironsmith_core::ValueSurfaceHint::OpponentsDealtDamageThisWay) =>
        {
            None
        }
        Value::SurfaceHinted { value, .. } => describe_effect_count_backref(value),
        Value::EffectValue(_) => Some("that many".to_string()),
        Value::EffectValueOffset(_, offset) => {
            if *offset == 0 {
                Some("that many".to_string())
            } else if *offset > 0 {
                Some(format!("that many plus {}", offset))
            } else if *offset == -1 {
                Some("that many minus one".to_string())
            } else {
                Some(format!("that many minus {}", -offset))
            }
        }
        Value::EventValue(EventValueSpec::Amount) => Some("that many".to_string()),
        Value::EventValueOffset(EventValueSpec::Amount, offset) => {
            if *offset == 0 {
                Some("that many".to_string())
            } else if *offset > 0 {
                Some(format!("that many plus {}", offset))
            } else if *offset == -1 {
                Some("that many minus one".to_string())
            } else {
                Some(format!("that many minus {}", -offset))
            }
        }
        Value::EffectMetric {
            metric:
                crate::effect::EffectMetric::Count
                | crate::effect::EffectMetric::ChosenCount
                | crate::effect::EffectMetric::AffectedCount,
            ..
        }
        | Value::PendingEffectMetric {
            metric:
                crate::effect::EffectMetric::Count
                | crate::effect::EffectMetric::ChosenCount
                | crate::effect::EffectMetric::AffectedCount,
            ..
        } => Some("that many".to_string()),
        Value::EffectMetricOffset {
            metric:
                crate::effect::EffectMetric::Count
                | crate::effect::EffectMetric::ChosenCount
                | crate::effect::EffectMetric::AffectedCount,
            offset,
            ..
        }
        | Value::PendingEffectMetricOffset {
            metric:
                crate::effect::EffectMetric::Count
                | crate::effect::EffectMetric::ChosenCount
                | crate::effect::EffectMetric::AffectedCount,
            offset,
            ..
        } => {
            if *offset == 0 {
                Some("that many".to_string())
            } else if *offset > 0 {
                Some(format!("that many plus {}", offset))
            } else if *offset == -1 {
                Some("that many minus one".to_string())
            } else {
                Some(format!("that many minus {}", -offset))
            }
        }
        _ => None,
    }
}

pub(crate) fn is_generic_owned_card_search_filter(filter: &ObjectFilter) -> bool {
    if filter.zone.is_some()
        || filter
            .owner
            .as_ref()
            .is_some_and(|owner| *owner != PlayerFilter::You)
    {
        return false;
    }

    // ObjectFilter equality deliberately ignores purely authored surface
    // metadata while covering every semantic field. Normalize the one
    // ownership scope allowed by this helper, then compare against the
    // unqualified filter instead of maintaining a partial field checklist
    // that can silently classify newly added constraints as "a card".
    let mut semantic_filter = filter.clone();
    semantic_filter.owner = None;
    semantic_filter == ObjectFilter::default()
}

fn describe_all_with_relation_exception_and_additional_sets(
    filter: &ObjectFilter,
) -> Option<String> {
    if filter.any_of.len() < 2 {
        return None;
    }
    let mut outer = filter.clone();
    outer.any_of.clear();
    if outer != ObjectFilter::default() {
        return None;
    }

    let first = filter.any_of.first()?;
    let [relation] = first.characteristic_relations.as_slice() else {
        return None;
    };
    if relation.kind != crate::ObjectCharacteristicRelationKind::SharesNone {
        return None;
    }
    let mut base = first.clone();
    base.characteristic_relations.clear();
    if !base.any_of.is_empty() {
        return None;
    }

    let base = pluralize_relative_object_phrase(strip_indefinite_article(&base.description()));
    let characteristics = relation
        .characteristics
        .iter()
        .map(|characteristic| characteristic.sharing_phrase())
        .collect::<Vec<_>>()
        .join(" or ");
    let mut description = format!(
        "all {base} except those that share {characteristics} with {}",
        relation.comparison_description()
    );

    let additional = filter.any_of[1..]
        .iter()
        .map(|branch| {
            let branch =
                pluralize_relative_object_phrase(strip_indefinite_article(&branch.description()));
            format!("all {branch}")
        })
        .collect::<Vec<_>>();
    match additional.as_slice() {
        [] => return None,
        [only] => description.push_str(&format!(", and {only}")),
        [prefix @ .., last] => {
            description.push_str(", ");
            description.push_str(&prefix.join(", "));
            description.push_str(", and ");
            description.push_str(last);
        }
    }
    Some(description)
}

fn materialize_conjunctive_union_branch_scope(
    branch: &ObjectFilter,
    common_zone: Option<Zone>,
    common_controller: &Option<PlayerFilter>,
    common_owner: &Option<PlayerFilter>,
    common_single_graveyard: bool,
) -> Option<ObjectFilter> {
    let mut branch = branch.clone();
    if let Some(zone) = common_zone {
        if branch.zone.is_some_and(|branch_zone| branch_zone != zone) {
            return None;
        }
        branch.zone = Some(zone);
    }
    if let Some(controller) = common_controller {
        if branch
            .controller
            .as_ref()
            .is_some_and(|branch_controller| branch_controller != controller)
        {
            return None;
        }
        branch.controller = Some(controller.clone());
    }
    if let Some(owner) = common_owner {
        if branch
            .owner
            .as_ref()
            .is_some_and(|branch_owner| branch_owner != owner)
        {
            return None;
        }
        branch.owner = Some(owner.clone());
    }
    branch.single_graveyard |= common_single_graveyard;
    Some(branch)
}

fn describe_plural_conjunctive_union_branch(filter: &ObjectFilter) -> String {
    if let Some(attached_to) = filter
        .attached_to_object
        .as_deref()
        .filter(|attached_to| attached_to.has_plural_object_noun_surface())
    {
        let mut subject = filter.clone();
        subject.attached_to_object = None;
        let subject = describe_object_filter_with_fixed_pt_shorthand(&subject);
        let subject =
            pluralize_relative_object_phrase(strip_leading_article(subject.trim()).trim());
        let attached_to = if attached_to.zone == Some(Zone::Battlefield)
            && attached_to.controller == Some(PlayerFilter::Opponent)
            && attached_to.owner.is_none()
        {
            let mut host = attached_to.clone();
            host.controller = None;
            let host = describe_object_filter_with_fixed_pt_shorthand(&host);
            format!(
                "{} your opponents control",
                pluralize_relative_object_phrase(strip_leading_article(host.trim()).trim())
            )
        } else {
            let attached_to = describe_object_filter_with_fixed_pt_shorthand(attached_to);
            pluralize_relative_object_phrase(strip_leading_article(attached_to.trim()).trim())
        };
        return format!("{subject} attached to {attached_to}");
    }

    let description = describe_object_filter_with_fixed_pt_shorthand(filter);
    pluralize_relative_object_phrase(strip_leading_article(description.trim()).trim())
}

/// Render an authored conjunction of independently scoped object collections.
///
/// The outer filter can factor common zone/owner/controller facts for runtime
/// matching, but each authored collection still needs its own `all` and noun
/// agreement: "all enchantments ..., all Auras ..., and all Auras ...".
fn describe_all_conjunctive_branch_union(filter: &ObjectFilter) -> Option<String> {
    if filter.any_of.len() < 2 || !filter.has_conjunctive_set_surface() {
        return None;
    }

    let mut outer = filter.clone();
    outer.any_of.clear();
    let common_zone = outer.zone.take();
    let common_controller = outer.controller.take();
    let common_owner = outer.owner.take();
    let common_single_graveyard = outer.single_graveyard;
    outer.single_graveyard = false;
    if outer != ObjectFilter::default() {
        return None;
    }

    let branches = filter
        .any_of
        .iter()
        .map(|branch| {
            materialize_conjunctive_union_branch_scope(
                branch,
                common_zone,
                &common_controller,
                &common_owner,
                common_single_graveyard,
            )
        })
        .collect::<Option<Vec<_>>>()?;
    let branches = branches
        .iter()
        .map(|branch| {
            let mut semantic_branch = branch.clone();
            let source_surface = semantic_branch.source_surface.take();
            if semantic_branch == ObjectFilter::source()
                && let Some(source_surface) = source_surface
            {
                return source_surface.display_text();
            }
            if branch.set_quantifier_surface() == Some(ironsmith_core::SetQuantifierSurface::Each) {
                let mut singular = branch.clone();
                singular.set_set_quantifier_surface(None);
                if singular == ObjectFilter::creature().controlled_by(PlayerFilter::Opponent) {
                    return "each creature your opponents control".to_string();
                }
                let singular = describe_object_filter_with_fixed_pt_shorthand(&singular);
                return format!("each {}", strip_leading_article(singular.trim()).trim());
            }
            format!("all {}", describe_plural_conjunctive_union_branch(branch))
        })
        .collect::<Vec<_>>();
    Some(join_with_and(&branches))
}

pub(crate) fn describe_object_count(value: &Value) -> String {
    match value {
        Value::Fixed(1) => "a".to_string(),
        Value::Fixed(n) if *n > 1 && *n <= 20 => {
            small_number_word(*n as u32).unwrap_or_else(|| n.to_string())
        }
        _ => describe_value(value),
    }
}

fn is_cast_modified_creatures_snapshot_filter(filter: &ObjectFilter) -> bool {
    if !matches!(
        filter.tagged_constraints.as_slice(),
        [constraint]
            if constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag.as_str() == ironsmith_core::CAST_MODIFIED_CREATURES_TAG
    ) {
        return false;
    }

    let mut remainder = filter.clone();
    remainder.tagged_constraints.clear();
    remainder == ObjectFilter::default()
}

fn cast_controlled_objects_snapshot_subject_filter(filter: &ObjectFilter) -> Option<ObjectFilter> {
    if !matches!(
        filter.tagged_constraints.as_slice(),
        [constraint]
            if constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag.as_str() == ironsmith_core::CAST_CONTROLLED_OBJECTS_TAG
    ) {
        return None;
    }

    let mut subject = filter.clone();
    subject.tagged_constraints.clear();
    if subject.zone != Some(Zone::Battlefield)
        || subject.controller.is_some()
        || subject.owner.is_some()
        || subject.cast_by.is_some()
        || subject.cast_this_turn
    {
        return None;
    }
    subject.zone = None;
    Some(subject)
}

fn describe_spell_cast_history_filter_subject(filter: &ObjectFilter) -> Option<String> {
    if !filter.cast_this_turn || filter.zone != Some(Zone::Stack) {
        return None;
    }
    let caster = filter.cast_by.as_ref()?;

    let mut spell_domain = filter.clone();
    spell_domain.cast_by = None;
    spell_domain.cast_this_turn = false;
    // Objects that were cast are spells, even when the stored history filter
    // omitted the redundant stack-kind discriminator. Without it the generic
    // stack renderer broadens the noun to "spells or abilities".
    spell_domain.stack_kind = Some(StackObjectKind::Spell);
    let subject = describe_count_filter_value_subject(&spell_domain);
    let cast_surface = match caster {
        PlayerFilter::You => "you've cast this turn".to_string(),
        PlayerFilter::Opponent => "your opponents have cast this turn".to_string(),
        PlayerFilter::IteratedPlayer => "they've cast this turn".to_string(),
        PlayerFilter::Specific(_) | PlayerFilter::AliasedTarget(_) => {
            "that player has cast this turn".to_string()
        }
        PlayerFilter::Any => "cast this turn".to_string(),
        other => format!("cast this turn by {}", describe_player_filter(other)),
    };
    Some(format!("{subject} {cast_surface}"))
}

pub(crate) fn describe_count_filter_value_subject(filter: &ObjectFilter) -> String {
    if is_cast_modified_creatures_snapshot_filter(filter) {
        return "modified creatures you controlled as you cast this spell".to_string();
    }
    if let Some(subject) = cast_controlled_objects_snapshot_subject_filter(filter) {
        return format!(
            "{} you controlled as you cast this spell",
            describe_count_filter_value_subject(&subject)
        );
    }
    if let Some(subject) = describe_commander_zone_union_subject(filter) {
        return subject;
    }
    if let Some(subject) = describe_shared_tagged_attachment_union_count_subject(filter) {
        return subject;
    }
    if let Some(subject) = describe_domain_union_count_filter_subject(filter) {
        return subject;
    }
    if filter.stack_kind == Some(StackObjectKind::Spell)
        && filter.cast_by == Some(PlayerFilter::You)
        && filter.zone == Some(Zone::Stack)
    {
        return "spells you've cast this turn".to_string();
    }
    if filter.stack_kind == Some(StackObjectKind::Spell)
        && filter.cast_by == Some(PlayerFilter::Opponent)
        && filter.zone == Some(Zone::Stack)
    {
        return "spells your opponents have cast this turn".to_string();
    }
    if describe_tagged_this_way_action(filter) == Some("revealed") {
        return pluralize_noun_phrase(&describe_for_each_count_filter(filter));
    }
    if describe_tagged_this_way_action(filter) == Some("discarded") {
        let mut untagged = filter.clone();
        untagged.tagged_constraints.clear();
        if untagged == ObjectFilter::default() {
            return "cards discarded this way".to_string();
        }
    }
    if filter.has_explicit_card_noun()
        && filter.card_types.is_empty()
        && filter.all_card_types.is_empty()
        && filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.relation == TaggedOpbjectRelation::IsTaggedObject)
    {
        return "those cards".to_string();
    }
    if filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str().starts_with("milled_")
    }) {
        return "those cards".to_string();
    }
    if filter.zone == Some(Zone::Hand) && filter.owner.is_none() {
        let mut unscoped = filter.clone();
        unscoped.zone = None;
        unscoped.single_graveyard = false;
        unscoped.set_explicit_card_noun(true);
        let subject =
            pluralize_noun_phrase(strip_indefinite_article(&unscoped.description()).trim());
        return format!("{subject} in all players' hands");
    }
    if filter.zone == Some(Zone::Graveyard)
        && let Some(owner) = filter.owner.as_ref()
    {
        let mut unscoped = filter.clone();
        unscoped.zone = None;
        unscoped.owner = None;
        unscoped.single_graveyard = false;
        unscoped.set_explicit_card_noun(true);
        let subject =
            pluralize_noun_phrase(strip_indefinite_article(&unscoped.description()).trim());
        return format!("{subject} in {}", describe_card_type_graveyard_scope(owner));
    }
    if filter.zone == Some(Zone::Hand)
        && filter.card_types.is_empty()
        && filter.all_card_types.is_empty()
        && filter.subtypes.is_empty()
        && let Some(owner) = &filter.owner
    {
        return format!("cards in {} hand", describe_possessive_player_filter(owner));
    }
    if filter.attached_to_object.is_none()
        && let Some(attached_to_player) = filter.attached_to_player.as_ref()
    {
        // Pluralize the counted objects, not the trailing attachment player.
        // `ObjectFilter::description` places the attachment phrase last, so
        // pluralizing that complete surface would produce e.g. "Curse
        // attached to enchanted players". Attachment already implies the
        // battlefield, so describe the remaining typed filter without a zone
        // and then append the structured player relation.
        let mut unattached = filter.clone();
        unattached.attached_to_player = None;
        unattached.zone = None;
        return format!(
            "{} attached to {}",
            describe_count_filter_value_subject(&unattached),
            describe_player_filter(attached_to_player)
        );
    }
    if filter.zone == Some(Zone::Battlefield)
        && filter.controller == Some(PlayerFilter::Opponent)
        && filter.owner.is_none()
    {
        let mut bare = filter.clone();
        bare.controller = None;
        let subject = pluralize_noun_phrase(
            strip_indefinite_article(&bare.description())
                .trim()
                .trim_end_matches(" on the battlefield")
                .trim(),
        );
        return format!("{subject} your opponents control");
    }

    let has_sacrificed_tag = filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            && matches!(
                tag_action_from_name(constraint.tag.as_str()),
                Some("sacrificed")
            )
    });
    let has_sacrifice_cost_tag = filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str().starts_with("sacrifice_cost_")
    });
    // Count surfaces use the same authored controller/qualifier ordering as
    // explicit for-each loops. Calling the raw filter description here
    // canonicalized "creature with power 4 or greater you control" back to
    // "creature you control with power 4 or greater" even though the parser
    // retained the presentation-only ordering hint.
    let mut subject_filter = filter.clone();
    if filter.zone == Some(Zone::Battlefield)
        && filter.all_card_types.is_empty()
        && filter_explicitly_selects_permanent_cards(filter)
    {
        // A six-type permanent filter is a "permanent card" selector in
        // hidden zones, but those same types on the battlefield are simply
        // permanents. Preserve every other typed restriction while removing
        // only the redundant card-type expansion and its card-noun surface.
        subject_filter.card_types.clear();
        subject_filter.set_explicit_card_noun(false);
    }
    let mut subject = strip_indefinite_article(&describe_for_each_filter(&subject_filter))
        .trim()
        .to_string();
    subject = pluralize_noun_phrase(&subject);
    if let Some(rest) = subject.strip_prefix("another ") {
        subject = format!("other {rest}");
    }
    let attachment_already_implies_battlefield = filter.attached_to_object.is_some()
        || filter.attached_to_player.is_some()
        || filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.relation == TaggedOpbjectRelation::AttachedToTaggedObject);
    if attachment_already_implies_battlefield {
        subject = subject.trim_end_matches(" on the battlefield").to_string();
    }
    if filter.subtypes.len() == 2
        && filter.subtypes.contains(&crate::types::Subtype::Mount)
        && filter.subtypes.contains(&crate::types::Subtype::Vehicle)
    {
        subject = subject.replace("Mounts or Vehicles", "Mounts and/or Vehicles");
    }
    if let Some(rest) = subject.strip_prefix("the active player's ") {
        subject = format!("{rest} they control");
    }

    // Zone-restricted counts with no owner specified are typically phrased
    // as "in all <zone>s" in oracle text ("all players' hands", "all graveyards").
    if filter.owner.is_none() && filter.zone == Some(Zone::Hand) {
        subject = subject.replace(" in hand", " in all players' hands");
    }
    if filter.owner.is_none() && !filter.single_graveyard && filter.zone == Some(Zone::Graveyard) {
        subject = subject.replace(" in graveyard", " in all graveyards");
        subject = subject.replace(" in a graveyard", " in all graveyards");
        if !subject.contains("graveyard") {
            subject.push_str(" in all graveyards");
        }
    }
    // `ObjectFilter::description` returns immediately after rendering a named
    // object, before it reaches the zone suffix. Recover that structured zone
    // here so count values such as "cards named ... in your graveyard" do not
    // silently broaden to every zone.
    if filter.zone == Some(Zone::Graveyard) && !subject.contains("graveyard") {
        let scope = match &filter.owner {
            Some(owner) => describe_card_type_graveyard_scope(owner),
            None if filter.single_graveyard => "a graveyard".to_string(),
            None => "all graveyards".to_string(),
        };
        subject.push_str(" in ");
        subject.push_str(&scope);
    }

    let mentions_location = subject.contains(" in ") || subject.contains(" on ");
    // Prefer filter metadata over brittle string matching. Oracle typically omits
    // "on the battlefield" when "you control"/"an opponent controls"/ownership is stated.
    let mentions_controller_or_owner = filter.controller.is_some()
        || filter.owner.is_some()
        || subject.contains(" controls")
        || subject.contains(" owns");
    let is_combat_restricted = filter.attacking
        || filter.nonattacking
        || filter.blocking
        || filter.nonblocking
        || filter.blocked
        || filter.unblocked;
    if filter.zone == Some(Zone::Battlefield)
        && !mentions_location
        && !mentions_controller_or_owner
        && !is_combat_restricted
        && !has_sacrificed_tag
        && !filter.didnt_enter_battlefield_this_turn
        && !filter.entered_battlefield_this_turn
        && filter.entered_battlefield_controller.is_none()
    {
        subject.push_str(" on the battlefield");
    }
    if has_sacrificed_tag && !has_sacrifice_cost_tag {
        // The generic tagged-action surface appends "sacrificed this way".
        // Aggregate values use the compact noun phrase "the sacrificed
        // creatures", so remove that suffix even when the lower-level
        // renderer has already supplied the leading "the sacrificed".
        subject = subject
            .strip_suffix(" sacrificed this way")
            .unwrap_or(&subject)
            .to_string();
        if !subject.to_ascii_lowercase().starts_with("the sacrificed ") {
            subject = format!(
                "the sacrificed {}",
                subject.trim_start_matches("the ").trim()
            );
        }
    }

    subject
}

/// Describe a coordinated object union whose every arm is attached to the
/// same previously tagged object. The attachment relation is semantic, while
/// the union's ordinary description still owns the authored object nouns and
/// any shared restrictions.
pub(crate) fn describe_shared_tagged_attachment_union_count_subject(
    filter: &ObjectFilter,
) -> Option<String> {
    if filter.any_of.len() < 2
        || !filter.tagged_constraints.is_empty()
        || filter.attached_to_object.is_some()
        || filter.attached_to_player.is_some()
    {
        return None;
    }

    let [first_constraint] = filter.any_of.first()?.tagged_constraints.as_slice() else {
        return None;
    };
    if first_constraint.relation != TaggedOpbjectRelation::AttachedToTaggedObject
        || first_constraint.tag.as_str() != "__it__"
        || filter.any_of.iter().any(|branch| {
            !matches!(
                branch.tagged_constraints.as_slice(),
                [constraint]
                    if constraint.relation
                        == TaggedOpbjectRelation::AttachedToTaggedObject
                        && constraint.tag == first_constraint.tag
            )
        })
    {
        return None;
    }

    let mut unlinked = filter.clone();
    for branch in &mut unlinked.any_of {
        branch.tagged_constraints.clear();
    }
    if unlinked.any_of.len() == 2
        && unlinked
            .any_of
            .iter()
            .any(|branch| branch.subtypes == [crate::types::Subtype::Aura])
        && unlinked
            .any_of
            .iter()
            .any(|branch| branch.subtypes == [crate::types::Subtype::Equipment])
    {
        return Some("Aura and Equipment attached to it".to_string());
    }
    let subject = describe_count_filter_value_subject(&unlinked);
    let subject = subject
        .strip_suffix(" on the battlefield")
        .unwrap_or(&subject);
    Some(format!("{subject} attached to it"))
}

/// Describe the object set used by an aggregate characteristic value.
///
/// Open sets conventionally remain bare ("the total power of creatures you
/// control"), while a set captured by an earlier action is definite ("the
/// total power of the creatures sacrificed this way"). The result tag carries
/// that distinction even when lowering did not retain an explicit value
/// surface hint.
pub(crate) fn describe_aggregate_filter_value_subject(filter: &ObjectFilter) -> String {
    let subject = describe_count_filter_value_subject(filter);
    if prior_effect_action_for_filter(filter).is_some()
        && !subject.starts_with("the ")
        && !subject.starts_with("those ")
    {
        format!("the {subject}")
    } else {
        subject
    }
}

/// Whether an unscoped count subject needs its battlefield provenance spelled
/// out. Controller/owner and combat predicates already imply the battlefield,
/// while a bare type or subtype does not.
pub(crate) fn count_filter_needs_battlefield_surface(filter: &ObjectFilter, subject: &str) -> bool {
    filter.zone == Some(Zone::Battlefield)
        && filter.controller.is_none()
        && filter.owner.is_none()
        && !subject.contains(" in ")
        && !subject.contains(" on ")
        && !filter.attacking
        && !filter.nonattacking
        && !filter.blocking
        && !filter.nonblocking
        && !filter.blocked
        && !filter.unblocked
        && describe_tagged_this_way_action(filter).is_none()
        && !filter.didnt_enter_battlefield_this_turn
        && !filter.entered_battlefield_this_turn
        && filter.entered_battlefield_controller.is_none()
}

pub(crate) fn describe_domain_union_count_filter_subject(filter: &ObjectFilter) -> Option<String> {
    if filter.any_of.len() < 2 {
        return None;
    }

    let mut outer = filter.clone();
    outer.any_of.clear();

    if let Some(subject) = describe_owned_spell_domain_union_subject(&outer, &filter.any_of) {
        return Some(subject);
    }

    let first_signature = domain_union_signature(filter.any_of.first()?)?;
    if filter.any_of.iter().any(|branch| {
        domain_union_signature(branch)
            .as_ref()
            .is_none_or(|signature| signature != &first_signature)
    }) {
        return None;
    }

    let branches = if outer == ObjectFilter::default() {
        filter.any_of.clone()
    } else {
        // Some parsed count filters keep the shared card constraints on the
        // outer union and use the branches only to identify the domains. Fold
        // those domains into the shared filter before describing each arm so
        // `instant cards ... in exile and in your graveyard` does not lose the
        // instant/card-owner constraints at render time.
        if first_signature != ObjectFilter::default() {
            return None;
        }
        filter
            .any_of
            .iter()
            .map(|branch| {
                let mut merged = outer.clone();
                merged.zone = branch.zone.or(merged.zone);
                merged.controller = branch.controller.clone().or(merged.controller);
                merged.owner = branch.owner.clone().or(merged.owner);
                merged.single_graveyard |= branch.single_graveyard;
                merged.other |= branch.other;
                merged
            })
            .collect::<Vec<_>>()
    };

    let subjects = branches
        .iter()
        .map(describe_count_filter_value_subject)
        .collect::<Vec<_>>();
    if subjects.iter().any(|subject| subject.trim().is_empty()) {
        return None;
    }

    Some(join_with_and(&subjects))
}

fn describe_owned_spell_domain_union_subject(
    outer: &ObjectFilter,
    branches: &[ObjectFilter],
) -> Option<String> {
    if !outer.type_or_subtype_union
        || outer.card_types.is_empty()
        || outer.subtypes.as_slice() != [crate::types::Subtype::Adventure]
        || branches
            .iter()
            .any(|branch| domain_union_signature(branch) != Some(ObjectFilter::default()))
    {
        return None;
    }

    let mut remainder = outer.clone();
    remainder.owner = None;
    remainder.card_types.clear();
    remainder.subtypes.clear();
    remainder.type_or_subtype_union = false;
    if remainder != ObjectFilter::default() {
        return None;
    }

    let mut subject = match outer.owner.as_ref() {
        Some(PlayerFilter::You) => "cards you own".to_string(),
        None => "cards".to_string(),
        _ => return None,
    };
    let mut locations = Vec::new();
    for branch in branches {
        let location = match branch.zone {
            Some(Zone::Exile) => "in exile",
            Some(Zone::Graveyard) if outer.owner == Some(PlayerFilter::You) => "in your graveyard",
            Some(Zone::Graveyard) => "in a graveyard",
            _ => return None,
        };
        if !locations.iter().any(|found| found == location) {
            locations.push(location.to_string());
        }
    }
    if locations.is_empty() {
        return None;
    }
    subject.push(' ');
    subject.push_str(&join_with_and(&locations));

    let mut predicates = outer
        .card_types
        .iter()
        .map(|card_type| format!("are {} cards", card_type.to_string().to_ascii_lowercase()))
        .collect::<Vec<_>>();
    predicates.push("have an Adventure".to_string());
    let predicate_text = match predicates.as_slice() {
        [] => return None,
        [only] => only.clone(),
        [first, second] => format!("{first} and/or {second}"),
        many => format!(
            "{}, and/or {}",
            many[..many.len() - 1].join(", "),
            many.last()?
        ),
    };
    Some(format!("{subject} that {predicate_text}"))
}

pub(crate) fn domain_union_signature(filter: &ObjectFilter) -> Option<ObjectFilter> {
    if !filter.any_of.is_empty() {
        return None;
    }

    let mut signature = filter.clone();
    signature.zone = None;
    signature.controller = None;
    signature.owner = None;
    signature.single_graveyard = false;
    signature.other = false;
    Some(signature)
}

pub(crate) fn describe_commander_zone_union_subject(filter: &ObjectFilter) -> Option<String> {
    if filter.any_of.len() < 2 {
        return None;
    }

    let mut common = filter.any_of.first()?.clone();
    common.zone = None;

    if !common.is_commander
        || !common.card_types.is_empty()
        || !common.all_card_types.is_empty()
        || !common.subtypes.is_empty()
        || common.type_or_subtype_union
        || common.name.is_some()
        || common.excluded_name.is_some()
    {
        return None;
    }

    if filter.any_of.iter().any(|nested| {
        if !nested.any_of.is_empty() {
            return true;
        }
        let mut comparable = nested.clone();
        comparable.zone = None;
        comparable != common
    }) {
        return None;
    }

    let mut zone_phrases = Vec::new();
    for zone in filter.any_of.iter().filter_map(|nested| nested.zone) {
        let phrase = match zone {
            Zone::Battlefield => "on the battlefield",
            Zone::Command => "in the command zone",
            _ => return None,
        };
        if !zone_phrases.contains(&phrase.to_string()) {
            zone_phrases.push(phrase.to_string());
        }
    }

    if zone_phrases.is_empty() {
        return None;
    }

    let subject = if let Some(owner) = common.owner.as_ref() {
        let owner_text = describe_player_filter(owner);
        format!(
            "commanders {owner_text} {}",
            player_verb(&owner_text, "own", "owns")
        )
    } else {
        "commanders".to_string()
    };
    Some(format!("{subject} {}", join_with_and(&zone_phrases)))
}

pub(crate) fn describe_for_each_count_filter(filter: &ObjectFilter) -> String {
    if filter.attacking
        && filter.attacking_player_only
        && let Some(attacked_player) = filter
            .attacking_player_or_planeswalker_controlled_by
            .as_ref()
    {
        let mut plain_creature = filter.clone();
        plain_creature.attacking = false;
        plain_creature.attacking_player_only = false;
        plain_creature.attacking_player_or_planeswalker_controlled_by = None;
        if plain_creature == ObjectFilter::creature() {
            return format!(
                "creature attacking {}",
                describe_player_filter(attacked_player)
            );
        }
    }
    if is_cast_modified_creatures_snapshot_filter(filter) {
        return "modified creature you controlled as you cast this spell".to_string();
    }
    if let Some(subject) = cast_controlled_objects_snapshot_subject_filter(filter) {
        return format!(
            "{} you controlled as you cast this spell",
            describe_for_each_count_filter(&subject)
        );
    }
    if let Some(subject) = describe_tagged_hand_origin_count_filter(filter) {
        return subject;
    }

    if std::env::var("IRONSMITH_CHOICE_TRACE").is_ok() {
        eprintln!(
            "count-subject: excluded={:?} tags={:?}",
            filter.excluded_card_types,
            filter
                .tagged_constraints
                .iter()
                .map(|c| c.tag.as_str())
                .collect::<Vec<_>>()
        );
    }
    let tagged_this_way_action = describe_tagged_this_way_action(filter);
    let mut bare = filter.clone();
    if tagged_this_way_action
        == filter
            .prior_effect_action_surface()
            .map(describe_prior_effect_action)
    {
        // The concrete result tag and the typed surface can both preserve the
        // same authored provenance. The tag is rendered below; keep the local
        // noun description bare so `revealed this way` is emitted only once.
        bare.set_prior_effect_action_surface(None);
    }
    let controller = bare.controller.clone();
    let owner = bare.owner.clone();
    bare.controller = None;
    let keep_owner_in_subject = owner.is_some()
        && matches!(
            bare.zone,
            Some(Zone::Graveyard | Zone::Hand | Zone::Library | Zone::Command)
        );
    if !keep_owner_in_subject {
        bare.owner = None;
    }

    let mut subject =
        strip_indefinite_article(&describe_object_filter_with_fixed_pt_shorthand(&bare))
            .to_string();
    if !keep_owner_in_subject {
        subject = subject.replace("target player's ", "");
        subject = subject.replace("that player's ", "");
    }
    let lower_subject = subject.to_ascii_lowercase();
    if lower_subject.starts_with("a ") {
        subject = subject[2..].to_string();
    } else if lower_subject.starts_with("an ") {
        subject = subject[3..].to_string();
    } else if let Some(rest) = lower_subject.strip_prefix("another ") {
        subject = format!("other {}", rest.trim());
    }
    if let Some(action) = tagged_this_way_action {
        if action == "discarded" && filter.zone == Some(Zone::Graveyard) {
            // An explicit graveyard destination distinguishes oracle's movement wording
            // ("put into a graveyard this way") from the broader "discarded this way".
            // Keep the discard tag for execution-time identity, but render the narrower
            // destination fact rather than flattening it into "in a graveyard discarded".
            let mut moved = filter.clone();
            moved.zone = None;
            moved.owner = None;
            moved.tagged_constraints.clear();
            // Once the graveyard presentation zone is removed, retain the
            // fact that this is a card movement rather than a battlefield
            // permanent description (for example, "land card put into a
            // graveyard this way").
            moved.set_explicit_card_noun(true);
            let moved = strip_indefinite_article(&moved.description())
                .trim()
                .to_string();
            let destination = match &owner {
                Some(owner) => format!(
                    "{} graveyard",
                    describe_possessive_graveyard_owner_filter(owner)
                ),
                None if filter.single_graveyard => "a single graveyard".to_string(),
                None => "a graveyard".to_string(),
            };
            subject = format!("{moved} put into {destination} this way");
        } else if action == "exiled" {
            if let Some(head) = subject.strip_suffix(" in exile") {
                subject = head.trim().to_string();
            } else if let Some((head, tail)) = subject.split_once(" in exile ") {
                subject = format!("{} {}", head.trim(), tail.trim());
            }
            if should_drop_card_noun_for_tagged_exiled_objects(filter) {
                if let Some(head) = subject.strip_suffix(" cards") {
                    subject = head.trim().to_string();
                } else if let Some(head) = subject.strip_suffix(" card") {
                    subject = head.trim().to_string();
                }
            }
        } else if action == "revealed" {
            if subject == "permanent" {
                subject = "card".to_string();
            } else if subject == "permanents" {
                subject = "cards".to_string();
            } else if let Some(head) = subject.strip_suffix(" permanent") {
                subject = format!("{} card", head.trim());
            } else if let Some(head) = subject.strip_suffix(" permanents") {
                subject = format!("{} cards", head.trim());
            }
        }
        if !(action == "discarded" && filter.zone == Some(Zone::Graveyard)) {
            subject = format!("{subject} {action} this way");
        }
    }

    let target_controller_suffix = match controller {
        Some(PlayerFilter::Target(ref inner)) => {
            let described = describe_player_filter(inner.as_ref());
            Some(format!(
                "target {} controls",
                strip_leading_article(&described)
            ))
        }
        _ => None,
    };
    let historical_controller_suffix = match controller {
        Some(ref player @ PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn { .. }) => {
            Some(format!("controlled by {}", player.description()))
        }
        _ => None,
    };
    let controller_suffix = target_controller_suffix
        .as_deref()
        .or(historical_controller_suffix.as_deref())
        .or_else(|| match controller {
            Some(PlayerFilter::You) => Some("you control"),
            Some(PlayerFilter::NotYou) => Some("you don't control"),
            Some(PlayerFilter::Opponent) => Some("an opponent controls"),
            Some(PlayerFilter::Any) => Some("a player controls"),
            Some(PlayerFilter::Active) => Some("they control"),
            Some(PlayerFilter::Defending) => Some("defending player controls"),
            Some(PlayerFilter::Attacking) => Some("attacking player controls"),
            Some(PlayerFilter::DamagedPlayer) => Some("that player controls"),
            Some(PlayerFilter::Teammate) => Some("a teammate controls"),
            Some(PlayerFilter::Specific(_)) => Some("that player controls"),
            Some(PlayerFilter::Target(_)) => None,
            Some(PlayerFilter::AliasedTarget(_)) | Some(PlayerFilter::IteratedPlayer) => {
                Some("that player controls")
            }
            Some(PlayerFilter::TaggedPlayer(_)) | Some(PlayerFilter::ChosenPlayer) => {
                Some("they control")
            }
            Some(player) if player.is_your_team() => Some("your team controls"),
            Some(PlayerFilter::Excluding { base, excluded })
                if matches!(base.as_ref(), PlayerFilter::Any)
                    && matches!(
                        excluded.as_ref(),
                        PlayerFilter::ControllerOf(
                            crate::target::ObjectRef::Tagged(_) | crate::target::ObjectRef::Target
                        )
                    ) =>
            {
                Some("another player controls")
            }
            _ => None,
        });
    let target_owner_suffix = match owner {
        Some(PlayerFilter::Target(ref inner)) => {
            let described = describe_player_filter(inner.as_ref());
            Some(format!("target {} owns", strip_leading_article(&described)))
        }
        _ => None,
    };
    let owner_suffix = if keep_owner_in_subject {
        None
    } else {
        target_owner_suffix.as_deref().or_else(|| match owner {
            Some(PlayerFilter::You) => Some("you own"),
            Some(PlayerFilter::NotYou) => Some("you don't own"),
            Some(PlayerFilter::Opponent) => Some("your opponents own"),
            Some(PlayerFilter::Any) => Some("a player owns"),
            Some(PlayerFilter::Active) => Some("they own"),
            Some(PlayerFilter::Defending) => Some("defending player owns"),
            Some(PlayerFilter::Attacking) => Some("attacking player owns"),
            Some(PlayerFilter::DamagedPlayer) => Some("that player owns"),
            Some(PlayerFilter::Teammate) => Some("a teammate owns"),
            Some(PlayerFilter::Specific(_)) => Some("that player owns"),
            Some(PlayerFilter::Target(_)) => None,
            Some(PlayerFilter::AliasedTarget(_)) | Some(PlayerFilter::IteratedPlayer) => {
                Some("that player owns")
            }
            Some(PlayerFilter::TaggedPlayer(_)) | Some(PlayerFilter::ChosenPlayer) => {
                Some("they own")
            }
            _ => None,
        })
    };
    let scope_suffix = match (controller_suffix, owner_suffix) {
        (Some("you control"), Some("you own")) => Some("you both own and control".to_string()),
        (Some("you control"), Some("you don't own")) => {
            Some("you control but don't own".to_string())
        }
        (Some("that player controls"), Some("that player owns")) => {
            Some("that player both owns and controls".to_string())
        }
        (Some(controller), Some(owner)) => Some(format!("{owner} but {controller}")),
        (Some(controller), None) => Some(controller.to_string()),
        (None, Some(owner)) => Some(owner.to_string()),
        (None, None) => None,
    };
    if let Some(suffix) = scope_suffix {
        if filter.has_controller_after_qualifiers_surface() {
            return format!("{subject} {suffix}");
        }
        // Keep the controller scope next to the filtered noun. Qualifiers are
        // restrictions on that scoped object, so Oracle renders "creature you
        // control with flying", "spell you control with mana value 2 or
        // less", and "permanent you control that shares ..." rather than
        // moving the controller to the end of the whole filter.
        const QUALIFIER_BOUNDARIES: &[&str] = &[
            " with ",
            " without ",
            " that ",
            " that's ",
            " of the chosen ",
            " named ",
            " not named ",
            " attached to ",
            " cast by ",
            " put into ",
            " in ",
            " on ",
        ];
        let boundary = QUALIFIER_BOUNDARIES
            .iter()
            .filter_map(|marker| subject.find(marker))
            .min();
        if let Some(boundary) = boundary {
            let (head, tail) = subject.split_at(boundary);
            return format!("{} {suffix}{tail}", head.trim());
        }
        return format!("{subject} {suffix}");
    }

    if owner.is_none() && !filter.single_graveyard && filter.zone == Some(Zone::Graveyard) {
        subject = subject.replace(" in a graveyard", " in all graveyards");
        subject = subject.replace(" in graveyard", " in all graveyards");
        if !subject.contains("graveyard") {
            subject.push_str(" in all graveyards");
        }
    }
    if count_filter_needs_battlefield_surface(filter, &subject) {
        subject.push_str(" on the battlefield");
    }

    subject
}

pub(crate) fn describe_tagged_hand_origin_count_filter(filter: &ObjectFilter) -> Option<String> {
    if filter.zone != Some(Zone::Hand)
        || filter.controller.is_some()
        || !filter.card_types.is_empty()
        || !filter.all_card_types.is_empty()
        || !filter.subtypes.is_empty()
        || !filter.supertypes.is_empty()
        || filter.tagged_constraints.len() != 1
    {
        return None;
    }

    let constraint = &filter.tagged_constraints[0];
    if constraint.relation != TaggedOpbjectRelation::IsTaggedObject {
        return None;
    }
    let tag = constraint.tag.as_str();
    if !(tag.starts_with("searched")
        || tag.starts_with("exiled")
        || crate::cards::is_sentence_helper_tag(tag, "exiled"))
    {
        return None;
    }

    Some(match &filter.owner {
        Some(PlayerFilter::You) => "card exiled from your hand this way".to_string(),
        Some(PlayerFilter::NotYou) => "card exiled from your opponent's hand this way".to_string(),
        Some(PlayerFilter::Specific(_))
        | Some(PlayerFilter::Target(_))
        | Some(PlayerFilter::AliasedTarget(_))
        | Some(PlayerFilter::IteratedPlayer)
        | Some(PlayerFilter::ControllerOf(_)) => "card exiled from their hand this way".to_string(),
        Some(owner) => format!(
            "card exiled from {} hand this way",
            describe_possessive_player_filter(owner)
        ),
        None => "card exiled from hand this way".to_string(),
    })
}

pub(crate) fn should_drop_card_noun_for_tagged_exiled_objects(filter: &ObjectFilter) -> bool {
    filter.zone == Some(Zone::Exile)
        && filter.owner.is_none()
        && filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.relation == TaggedOpbjectRelation::IsTaggedObject)
        && !filter.card_types.is_empty()
        && filter.card_types.iter().all(|card_type| {
            matches!(
                card_type,
                CardType::Artifact
                    | CardType::Battle
                    | CardType::Creature
                    | CardType::Enchantment
                    | CardType::Land
                    | CardType::Planeswalker
            )
        })
}

pub(crate) fn describe_for_each_spells_cast_this_turn(
    player: &PlayerFilter,
    other_than_first: bool,
) -> String {
    let mut base = match player {
        PlayerFilter::You => "spell you've cast this turn".to_string(),
        PlayerFilter::Opponent => "spell an opponent has cast this turn".to_string(),
        PlayerFilter::Any => "spell cast this turn".to_string(),
        PlayerFilter::Active => "spell the active player has cast this turn".to_string(),
        PlayerFilter::Specific(_) => "spell that player has cast this turn".to_string(),
        _ => format!("spell cast this turn by {}", describe_player_filter(player)),
    };
    if other_than_first {
        base.push_str(" other than the first");
    }
    base
}

pub(crate) fn describe_demonstrative_tagged_object_filter(
    filter: &crate::filter::ObjectFilter,
) -> Option<String> {
    if let Some(attached) = describe_attached_tagged_object_filter(filter) {
        return Some(attached);
    }
    if let Some(prior_result) = describe_prior_effect_tagged_filter_surface(filter) {
        return Some(prior_result);
    }
    if let Some(attributed_choice) = describe_attributed_target_choice_filter(filter) {
        return Some(attributed_choice);
    }

    let implicit_constraints = filter
        .tagged_constraints
        .iter()
        .filter(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && is_implicit_reference_tag(constraint.tag.as_str())
        })
        .collect::<Vec<_>>();
    if implicit_constraints.len() != 1 {
        return None;
    }
    let implicit_tag = implicit_constraints[0].tag.as_str();

    let mut base = filter.clone();
    base.tagged_constraints.retain(|constraint| {
        !(constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == implicit_tag)
    });

    if implicit_tag == "blocking" && base.blocking {
        base.blocking = false;
        let noun = strip_leading_article(&base.description())
            .trim()
            .to_string();
        return Some(format!(
            "the blocking {}",
            if noun.is_empty() { "creature" } else { &noun }
        ));
    }
    if implicit_tag == "blocked" && base.attacking {
        base.attacking = false;
        let noun = strip_leading_article(&base.description())
            .trim()
            .to_string();
        return Some(format!(
            "the attacking {}",
            if noun.is_empty() { "creature" } else { &noun }
        ));
    }

    if base == crate::filter::ObjectFilter::default() {
        return Some("it".to_string());
    }

    let base_desc = strip_leading_article(&base.description())
        .trim()
        .to_string();
    if base_desc.is_empty() {
        Some("that object".to_string())
    } else {
        Some(format!("that {base_desc}"))
    }
}

const CHOSEN_OBJECTS_SURFACE_TAG: &str = "__chosen_objects__";

/// Render an `All` selection constrained to the durable union of authored
/// choices. Unlike generated antecedent tags, this tag denotes the complete
/// selected set, so its canonical surface is "the chosen ..." rather than
/// "those ..." or an unconstrained "all ...".
fn describe_chosen_object_set_filter(filter: &ObjectFilter) -> Option<String> {
    let chosen_constraints = filter
        .tagged_constraints
        .iter()
        .filter(|constraint| {
            constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag.as_str() == CHOSEN_OBJECTS_SURFACE_TAG
        })
        .count();
    if chosen_constraints != 1 {
        return None;
    }

    let mut base = filter.clone();
    base.tagged_constraints.retain(|constraint| {
        constraint.relation != TaggedOpbjectRelation::IsTaggedObject
            || constraint.tag.as_str() != CHOSEN_OBJECTS_SURFACE_TAG
    });
    let noun = strip_leading_article(&base.description())
        .trim()
        .to_string();
    let noun = if noun.is_empty() {
        "objects".to_string()
    } else {
        pluralize_relative_object_phrase(&noun)
    };
    Some(format!("the chosen {noun}"))
}

/// Render an identity reference to a target according to the player who made
/// that authored choice. Ordinary generated target tags only retain "that
/// creature"; this role-aware form preserves "the creature you chose" versus
/// "the creature your opponent chose" when both are live at once.
fn describe_attributed_target_choice_filter(
    filter: &crate::filter::ObjectFilter,
) -> Option<String> {
    let attributed = filter
        .tagged_constraints
        .iter()
        .filter_map(|constraint| {
            (constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject)
                .then(|| effect_text_shared::target_choice_attribution(constraint.tag.as_str()))
                .flatten()
        })
        .collect::<Vec<_>>();
    let [attribution] = attributed.as_slice() else {
        return None;
    };

    let mut base = filter.clone();
    base.tagged_constraints.retain(|constraint| {
        constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
            || effect_text_shared::target_choice_attribution(constraint.tag.as_str()).is_none()
    });
    let noun = strip_leading_article(&base.description())
        .trim()
        .to_string();
    let noun = if noun.is_empty() {
        "object"
    } else {
        noun.as_str()
    };
    let chooser = match attribution {
        effect_text_shared::TargetChoiceAttribution::AbilityController => "you",
        effect_text_shared::TargetChoiceAttribution::Opponent => "your opponent",
    };
    Some(format!("the {noun} {chooser} chose"))
}

/// Render an object reference that explicitly named the action producing it,
/// such as "a land card discarded this way". Runtime identity remains the
/// generated tag constraint; the typed surface prevents ordinary pronoun
/// rendering from erasing the provenance relationship.
pub(crate) fn describe_prior_effect_tagged_filter_surface(
    filter: &crate::filter::ObjectFilter,
) -> Option<String> {
    let action = prior_effect_action_for_filter(filter)?;
    let mut base = filter.clone();
    base.set_prior_effect_action_surface(None);
    base.tagged_constraints.retain(|constraint| {
        constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
    });
    // The producer action supplies the relevant zone transition. A destination
    // zone attached by the consuming move must not leak into the noun phrase.
    base.zone = None;
    let described = ensure_indefinite_article(&base.description());
    Some(format!(
        "{described} {} this way",
        describe_prior_effect_action_clause(action)
    ))
}

/// Recover authored action provenance from either its typed surface or the
/// generated result tag used by older parser paths. Runtime identity remains
/// the tag; this only restores the relationship in compiled text.
pub(crate) fn prior_effect_action_for_filter(
    filter: &crate::filter::ObjectFilter,
) -> Option<crate::effect::PriorEffectAction> {
    filter.prior_effect_action_surface().or_else(|| {
        filter.tagged_constraints.iter().find_map(|constraint| {
            if constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject {
                return None;
            }
            let base = constraint.tag.as_str().split('_').next()?;
            Some(match base {
                "cast" => crate::effect::PriorEffectAction::Cast,
                "chosen" => crate::effect::PriorEffectAction::Chosen,
                "connived" => crate::effect::PriorEffectAction::Connived,
                "countered" => crate::effect::PriorEffectAction::Countered,
                "counters" => crate::effect::PriorEffectAction::CountersPut,
                "damaged" => crate::effect::PriorEffectAction::DealtDamage,
                "destroyed" => crate::effect::PriorEffectAction::Destroyed,
                "discarded" | "discard" => crate::effect::PriorEffectAction::Discarded,
                "drawn" => crate::effect::PriorEffectAction::Drawn,
                "exiled" | "exile" => crate::effect::PriorEffectAction::Exiled,
                "goaded" => crate::effect::PriorEffectAction::Goaded,
                "milled" => crate::effect::PriorEffectAction::Milled,
                "phased" => crate::effect::PriorEffectAction::PhasedOut,
                "prevented" => crate::effect::PriorEffectAction::Prevented,
                "returned" => crate::effect::PriorEffectAction::Returned,
                "revealed" => crate::effect::PriorEffectAction::Revealed,
                "sacrifice" | "sacrificed" => crate::effect::PriorEffectAction::Sacrificed,
                "searched" => crate::effect::PriorEffectAction::Searched,
                "shuffled" => crate::effect::PriorEffectAction::Shuffled,
                "tapped" => crate::effect::PriorEffectAction::Tapped,
                _ => return None,
            })
        })
    })
}

pub(crate) fn describe_attached_tagged_object_filter(
    filter: &crate::filter::ObjectFilter,
) -> Option<String> {
    let attached_constraints = filter
        .tagged_constraints
        .iter()
        .filter(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && matches!(constraint.tag.as_str(), "enchanted" | "equipped")
        })
        .collect::<Vec<_>>();
    if attached_constraints.len() != 1 {
        return None;
    }
    let attached_tag = attached_constraints[0].tag.as_str();
    let mut base = filter.clone();
    base.tagged_constraints.retain(|constraint| {
        !(constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == attached_tag)
    });

    let mut surface_base = base;
    surface_base.zone = None;

    if surface_base == crate::filter::ObjectFilter::default() {
        return Some(describe_attached_object_for_tag(attached_tag, None));
    }
    if surface_base.card_types.len() == 1
        && surface_base.all_card_types.is_empty()
        && surface_base.subtypes.is_empty()
        && surface_base
            == crate::filter::ObjectFilter::default().with_type(surface_base.card_types[0])
    {
        return Some(format!(
            "{} {}",
            attached_tag,
            describe_card_type_word_local(surface_base.card_types[0])
        ));
    }
    if attached_tag == "enchanted"
        && surface_base.card_types.is_empty()
        && surface_base.all_card_types.is_empty()
        && !surface_base.subtypes.is_empty()
    {
        return Some(format!(
            "{attached_tag} {}",
            strip_leading_article(&surface_base.description())
        ));
    }
    None
}

pub(crate) fn describe_demonstrative_tagged_object_spec(spec: &ChooseSpec) -> Option<String> {
    let ChooseSpec::Object(filter) = spec else {
        return None;
    };
    describe_demonstrative_tagged_object_filter(filter)
}

fn describe_shared_creature_battlefield_or_graveyard_filter(
    filter: &ObjectFilter,
) -> Option<String> {
    if filter.any_of.len() != 2 {
        return None;
    }

    // The union parser may retain the shared Creature type on each branch so
    // the Battlefield arm can carry `other` while the Graveyard arm carries
    // the authored `card` noun.  This is semantically different from applying
    // `other` to the whole union, so prove both branch-local surfaces before
    // factoring the sentence.
    if filter.card_types.is_empty() {
        let mut outer = filter.clone();
        let branches = std::mem::take(&mut outer.any_of);
        outer.union_surface = Default::default();
        if outer != ObjectFilter::default() {
            return None;
        }

        let mut battlefield = false;
        let mut graveyard = false;
        for branch in branches {
            let zone = branch.zone?;
            let explicit_card_noun = branch.has_explicit_card_noun();
            let explicit_type_noun = branch.explicit_card_type_noun();
            let other = branch.other;
            let mut semantic = branch;
            semantic.zone = None;
            semantic.card_types.clear();
            semantic.other = false;
            semantic.union_surface = Default::default();
            if semantic != ObjectFilter::default() || explicit_type_noun != Some(CardType::Creature)
            {
                return None;
            }
            match zone {
                Zone::Battlefield if other && !explicit_card_noun && !battlefield => {
                    battlefield = true;
                }
                Zone::Graveyard if !other && explicit_card_noun && !graveyard => {
                    graveyard = true;
                }
                _ => return None,
            }
        }
        return (battlefield && graveyard).then_some(
            "other creature from the battlefield or creature card from a graveyard".to_string(),
        );
    }

    if filter.card_types.as_slice() != [CardType::Creature] {
        return None;
    }

    let mut outer = filter.clone();
    outer.card_types.clear();
    outer.other = false;
    outer.any_of.clear();
    outer.union_surface = Default::default();
    if outer != ObjectFilter::default() {
        return None;
    }

    let mut has_battlefield = false;
    let mut has_graveyard = false;
    for branch in &filter.any_of {
        let mut bare = branch.clone();
        let zone = bare.zone.take()?;
        bare.union_surface = Default::default();
        if bare != ObjectFilter::default() {
            return None;
        }
        match zone {
            Zone::Battlefield if !has_battlefield => has_battlefield = true,
            Zone::Graveyard if !has_graveyard => has_graveyard = true,
            _ => return None,
        }
    }
    if !has_battlefield || !has_graveyard {
        return None;
    }

    Some(format!(
        "{}creature from the battlefield or creature card from a graveyard",
        if filter.other { "another " } else { "a " }
    ))
}

pub(crate) fn describe_object_filter_with_fixed_pt_shorthand(filter: &ObjectFilter) -> String {
    if let Some(prior_result) = describe_prior_effect_tagged_filter_surface(filter) {
        return prior_result;
    }
    if filter.has_iterated_actor_pronoun_surface() && filter.controller.is_some() {
        let mut unscoped = filter.clone();
        unscoped.controller = None;
        unscoped.set_iterated_actor_pronoun_surface(false);
        return format!(
            "{} they control",
            describe_object_filter_with_fixed_pt_shorthand(&unscoped)
        );
    }
    let fixed_pt = match (&filter.power, &filter.toughness) {
        (
            Some(ironsmith_core::FilterComparison::Equal(power)),
            Some(ironsmith_core::FilterComparison::Equal(toughness)),
        ) if filter.card_types.as_slice() == [CardType::Creature]
            && filter.all_card_types.is_empty()
            && matches!(filter.zone, None | Some(Zone::Battlefield))
            && filter.power_reference == ironsmith_core::PtReference::Effective
            && filter.toughness_reference == ironsmith_core::PtReference::Effective
            && filter.power_parity.is_none()
            && filter.power_relative_to_source.is_none()
            && !filter.power_greater_than_base_power
            && filter.power_toughness_relation.is_none()
            && filter.total_power_toughness.is_none() =>
        {
            Some((*power, *toughness))
        }
        _ => None,
    };
    let Some((power, toughness)) = fixed_pt else {
        return filter.description();
    };

    let mut without_pt = filter.clone();
    without_pt.power = None;
    without_pt.toughness = None;
    let description = without_pt.description();
    let shorthand = format!("{power}/{toughness}");
    for determiner in [
        "a ", "an ", "another ", "other ", "target ", "this ", "that ", "those ",
    ] {
        if let Some(rest) = description.strip_prefix(determiner) {
            return format!("{determiner}{shorthand} {rest}");
        }
    }
    format!("{shorthand} {description}")
}

fn describe_object_or_player_union(
    object_filter: &ObjectFilter,
    object_text: String,
    player_filter: &PlayerFilter,
) -> String {
    let player_text = describe_player_filter(player_filter);
    let player_text = strip_leading_article(&player_text);
    if object_filter.card_types.len() > 1
        && let Some((leading_types, final_type)) = object_text.rsplit_once(", or ")
    {
        return format!("{leading_types}, {final_type}, or {player_text}");
    }
    if object_filter.card_types.len() == 2
        && let Some((first_type, second_type)) = object_text.rsplit_once(" or ")
    {
        return format!("{first_type}, {second_type}, or {player_text}");
    }
    format!("{object_text} or {player_text}")
}

fn describe_any_target_excluding_subtypes(
    object_filter: &ObjectFilter,
    player_filter: &PlayerFilter,
) -> Option<String> {
    if player_filter != &PlayerFilter::Any || object_filter.excluded_subtypes.is_empty() {
        return None;
    }
    let mut base = object_filter.clone();
    base.excluded_subtypes.clear();
    if base != ObjectFilter::default() {
        return None;
    }
    let excluded = object_filter
        .excluded_subtypes
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    Some(format!(
        "any target that isn't {}",
        with_indefinite_article(&join_with_or(&excluded))
    ))
}

pub(crate) fn describe_choose_spec(spec: &ChooseSpec) -> String {
    match spec {
        ChooseSpec::SurfaceHinted { spec, hints } => {
            if let Some(kind) = hints.iter().find_map(|hint| match hint {
                crate::target::ChooseSpecSurfaceHint::SacrificedObject(kind) => Some(*kind),
                crate::target::ChooseSpecSurfaceHint::SourceReference(_) => None,
            }) {
                return format!("the sacrificed {}", kind.noun());
            }
            match hints.iter().find_map(|hint| match hint {
                crate::target::ChooseSpecSurfaceHint::SourceReference(surface) => Some(surface),
                crate::target::ChooseSpecSurfaceHint::SacrificedObject(_) => None,
            }) {
                Some(surface) => describe_source_reference_surface_text(surface),
                None => describe_choose_spec(spec),
            }
        }
        ChooseSpec::Target(inner) => {
            if let ChooseSpec::Object(filter) = inner.as_ref()
                && filter.has_x_in_cost
            {
                return format!(
                    "target {}",
                    strip_indefinite_article(&describe_object_filter_with_fixed_pt_shorthand(
                        filter
                    ))
                );
            }
            if let ChooseSpec::Object(filter) = inner.as_ref()
                && filter.power_toughness_relation
                    == Some(ironsmith_core::PowerToughnessRelation::NotEqual)
            {
                let mut base = filter.clone();
                base.power_toughness_relation = None;
                let mut description = describe_object_filter_with_fixed_pt_shorthand(&base);
                for subtype in &base.excluded_subtypes {
                    let canonical = subtype.to_string();
                    description = description.replace(
                        &format!("non-{}", canonical.to_ascii_lowercase()),
                        &format!("non-{canonical}"),
                    );
                }
                return format!(
                    "target {} whose power and toughness aren't equal",
                    strip_indefinite_article(&description)
                );
            }
            if let ChooseSpec::ObjectOrPlayer(object_filter, player_filter) = inner.as_ref()
                && let Some(qualified) =
                    describe_any_target_excluding_subtypes(object_filter, player_filter)
            {
                return qualified;
            }
            if let ChooseSpec::Player(PlayerFilter::Excluding { base, excluded }) = inner.as_ref()
                && matches!(base.as_ref(), PlayerFilter::Any)
                && !matches!(excluded.as_ref(), PlayerFilter::OwnerOf(_))
            {
                return "another target player".to_string();
            }
            if let ChooseSpec::Object(filter) = inner.as_ref()
                && let Some(exiled_card) = describe_simple_exiled_card_filter(filter)
            {
                return format!("target {exiled_card}");
            }
            // An attachment reference inside a target spec is still a target
            // in oracle ("Destroy target enchanted creature"); a demonstrative
            // back-reference ("that creature") names an object the spell has
            // already chosen and takes no prefix.
            if let ChooseSpec::Object(filter) = inner.as_ref()
                && let Some(attached_text) = describe_attached_tagged_object_filter(filter)
            {
                return format!("target {attached_text}");
            }
            // The filter describer spells a bare commander subject possessively
            // ("Whenever your commander attacks"), but a targeted commander keeps
            // the long form — oracle says "Return target commander you own to its
            // owner's hand", never "target your commander". The filter itself is
            // identical in both cases; only this spec knows it is a target.
            if let ChooseSpec::Object(filter) = inner.as_ref()
                && filter.description() == "your commander"
            {
                return "target commander you own".to_string();
            }
            if let Some(tagged_text) = describe_demonstrative_tagged_object_spec(inner.as_ref()) {
                return tagged_text;
            }
            if let ChooseSpec::Object(filter) = inner.as_ref()
                && filter.zone == Some(Zone::Battlefield)
                && filter.controller.is_some()
                && !filter.source
            {
                let target_text = if filter.owner.is_some() {
                    strip_indefinite_article(&describe_object_filter_with_fixed_pt_shorthand(
                        filter,
                    ))
                    .to_string()
                } else {
                    describe_for_each_count_filter(filter)
                };
                if let Some(rest) = target_text.strip_prefix("other ") {
                    return format!("another target {rest}");
                }
                if let Some(rest) = target_text.strip_prefix("another ") {
                    return format!("another target {rest}");
                }
                return format!("target {target_text}");
            }
            let inner_text = describe_choose_spec(inner);
            // A bare object inside `Target` denotes exactly one object. The
            // object-filter renderer also serves plural set selections, so it
            // keeps plural agreement; the typed target wrapper is where we can
            // safely restore the singular oracle wording.
            let inner_text = if matches!(inner.as_ref(), ChooseSpec::Object(_)) {
                inner_text.replace(
                    " that aren't of the chosen type",
                    " that isn't of the chosen type",
                )
            } else {
                inner_text
            };
            if inner_text == "it" {
                inner_text
            } else if inner_text.starts_with("this ") {
                inner_text
            } else if inner_text.starts_with("that ") || inner_text.starts_with("those ") {
                inner_text
            } else if inner_text.starts_with("target ") {
                inner_text
            } else if let Some(rest) = inner_text.strip_prefix("another ") {
                format!("another target {rest}")
            } else if let Some(rest) = inner_text.strip_prefix("other ") {
                format!("other target {rest}")
            } else {
                let stripped = strip_leading_article(&inner_text);
                if stripped == inner_text {
                    format!("target {inner_text}")
                } else {
                    format!("target {stripped}")
                }
            }
        }
        ChooseSpec::AnyTarget => "any target".to_string(),
        ChooseSpec::AnyOtherTarget => "any other target".to_string(),
        ChooseSpec::AttackedPlayerOrPlaneswalker => {
            "the player or planeswalker it's attacking".to_string()
        }
        ChooseSpec::PlayerOrPlaneswalker(filter) => match filter {
            PlayerFilter::Opponent => "target opponent or planeswalker".to_string(),
            PlayerFilter::Any => "target player or planeswalker".to_string(),
            other => format!("target {} or planeswalker", describe_player_filter(other)),
        },
        ChooseSpec::ObjectOrPlayer(object_filter, player_filter) => {
            describe_object_or_player_union(
                object_filter,
                describe_choose_spec(&ChooseSpec::Object(object_filter.clone())),
                player_filter,
            )
        }
        ChooseSpec::Object(filter) => {
            if let Some(zone_union) =
                describe_shared_creature_battlefield_or_graveyard_filter(filter)
            {
                zone_union
            } else if let Some(exiled_card) = describe_simple_exiled_card_filter(filter) {
                ensure_indefinite_article(&exiled_card)
            } else if filter.source && filter.source_surface.is_some() {
                describe_object_filter_with_fixed_pt_shorthand(filter)
            } else if let Some(tagged_text) = describe_demonstrative_tagged_object_filter(filter) {
                tagged_text
            } else {
                ensure_indefinite_article(&describe_object_filter_with_fixed_pt_shorthand(filter))
            }
        }
        ChooseSpec::Player(filter) => describe_player_filter(filter),
        ChooseSpec::Source => "this source".to_string(),
        ChooseSpec::SourceController => "you".to_string(),
        ChooseSpec::SourceOwner => "this source's owner".to_string(),
        ChooseSpec::Tagged(tag) => {
            if tag.as_str().contains("copied") {
                return "the copy".to_string();
            }
            if tag.as_str() == "equipped" {
                return "equipped creature".to_string();
            }
            if tag.as_str() == "enchanted" {
                return "enchanted creature".to_string();
            }
            if tag.as_str() == "blocking" {
                return "that creature".to_string();
            }
            if tag.as_str() == crate::effects::PUBLIC_REVEALED_TAG {
                return "the revealed card".to_string();
            }
            if tag.as_str() == crate::tag::EXPLOITED_TAG {
                return "the exploited creature".to_string();
            }
            if tag.as_str() == crate::tag::SOURCE_EXILED_TAG {
                return "the exiled card".to_string();
            }
            if tag.as_str() == crate::tag::PRIOR_EXILED_CARD_TAG {
                return "the exiled card".to_string();
            }
            if tag.as_str() == "__chosen_objects__" {
                return "the chosen cards".to_string();
            }
            if tag.as_str() == "rest" {
                return "the rest".to_string();
            }
            if is_implicit_reference_tag(tag.as_str()) {
                "it".to_string()
            } else {
                format!("the tagged object '{}'", tag.as_str())
            }
        }
        ChooseSpec::All(filter) => {
            if let Some(description) = describe_all_conjunctive_branch_union(filter) {
                return description;
            }
            if let Some(description) =
                describe_all_with_relation_exception_and_additional_sets(filter)
            {
                return description;
            }
            if let Some(chosen_set) = describe_chosen_object_set_filter(filter) {
                return chosen_set;
            }
            if let Some(tagged_text) = describe_demonstrative_tagged_object_filter(filter) {
                if matches!(tagged_text.as_str(), "it" | "that object") {
                    return "them".to_string();
                }
                if let Some(rest) = tagged_text.strip_prefix("that ") {
                    return format!("those {}", pluralize_noun_phrase(rest));
                }
            }
            let desc = describe_object_filter_with_fixed_pt_shorthand(filter);
            let stripped = strip_leading_article(&desc);
            format!("all {}", pluralize_relative_object_phrase(stripped))
        }
        ChooseSpec::EachPlayer(filter) => format!("each {}", describe_player_filter(filter)),
        ChooseSpec::SpecificObject(_) => "that object".to_string(),
        ChooseSpec::SpecificPlayer(_) => "that player".to_string(),
        ChooseSpec::Iterated => "it".to_string(),
        ChooseSpec::WithCount(inner, count) | ChooseSpec::WithCountValue(inner, count, _) => {
            let inner_text = describe_choose_spec(inner);
            let controller_suffix = match inner.base() {
                ChooseSpec::Object(filter) if filter.target_set_same_controller => {
                    " controlled by the same player"
                }
                ChooseSpec::Object(filter) if filter.target_set_different_controllers => {
                    " controlled by different players"
                }
                _ => "",
            };
            let random_suffix = if count.is_random() {
                if count.is_single() {
                    " chosen at random"
                } else {
                    " at random"
                }
            } else {
                ""
            };
            if count.is_single() {
                format!("{inner_text}{random_suffix}")
            } else {
                if let ChooseSpec::Target(target_inner) = inner.as_ref() {
                    let target_desc = describe_choose_spec(target_inner);
                    let base = strip_leading_article(&target_desc);
                    let mut plural = pluralize_relative_object_phrase(base);
                    if let ChooseSpec::Object(filter) = target_inner.base()
                        && filter.zone == Some(Zone::Graveyard)
                        && filter.owner.is_none()
                        && !filter.single_graveyard
                    {
                        plural = plural
                            .replace(" in a graveyard", " in graveyards")
                            .replace(" from a graveyard", " from graveyards");
                    }
                    let plural_target = ["another target ", "other target ", "another ", "other "]
                        .iter()
                        .find_map(|prefix| base.strip_prefix(prefix))
                        .map(|rest| {
                            format!("other target {}", pluralize_relative_object_phrase(rest))
                        })
                        .unwrap_or_else(|| format!("target {plural}"));
                    let count_text =
                        |n: usize| number_word(n as i32).unwrap_or_else(|| n.to_string());
                    if count.is_up_to_dynamic_x() {
                        return format!(
                            "up to X {plural_target}{controller_suffix}{random_suffix}"
                        );
                    }
                    if count.is_dynamic_x() {
                        return format!("X {plural_target}{controller_suffix}{random_suffix}");
                    }
                    match (count.min, count.max) {
                        (0, None) => {
                            format!(
                                "any number of {plural_target}{controller_suffix}{random_suffix}"
                            )
                        }
                        (1, None) => {
                            format!("one or more {plural_target}{controller_suffix}{random_suffix}")
                        }
                        (min, None) => {
                            format!(
                                "at least {min} {plural_target}{controller_suffix}{random_suffix}"
                            )
                        }
                        (0, Some(max)) => {
                            if max == 1 {
                                if let Some(rest) = inner_text
                                    .strip_prefix("another target ")
                                    .or_else(|| inner_text.strip_prefix("other target "))
                                {
                                    format!(
                                        "up to one other target {rest}{controller_suffix}{random_suffix}"
                                    )
                                } else {
                                    format!(
                                        "up to one target {base}{controller_suffix}{random_suffix}"
                                    )
                                }
                            } else {
                                format!(
                                    "up to {} {plural_target}{controller_suffix}{random_suffix}",
                                    count_text(max)
                                )
                            }
                        }
                        (min, Some(max)) if min == max => {
                            if min == 1 {
                                format!("target {base}{controller_suffix}{random_suffix}")
                            } else {
                                format!(
                                    "{} {plural_target}{controller_suffix}{random_suffix}",
                                    count_text(min)
                                )
                            }
                        }
                        (1, Some(2)) => {
                            format!("one or two {plural_target}{controller_suffix}{random_suffix}")
                        }
                        (1, Some(3)) => {
                            format!(
                                "one, two, or three {plural_target}{controller_suffix}{random_suffix}"
                            )
                        }
                        (min, Some(max)) => {
                            format!(
                                "{} to {} {plural_target}{controller_suffix}{random_suffix}",
                                count_text(min),
                                count_text(max)
                            )
                        }
                    }
                } else {
                    let base = strip_leading_article(&inner_text);
                    let plural = pluralize_relative_object_phrase(base);
                    let count_text =
                        |n: usize| number_word(n as i32).unwrap_or_else(|| n.to_string());
                    if count.is_up_to_dynamic_x() {
                        return format!("up to X {plural}{controller_suffix}{random_suffix}");
                    }
                    if count.is_dynamic_x() {
                        return format!("X {plural}{controller_suffix}{random_suffix}");
                    }
                    match (count.min, count.max) {
                        (0, None) => {
                            format!("any number of {plural}{controller_suffix}{random_suffix}")
                        }
                        (1, None) => {
                            format!("one or more {plural}{controller_suffix}{random_suffix}")
                        }
                        (min, None) => format!(
                            "at least {} {plural}{controller_suffix}{random_suffix}",
                            count_text(min)
                        ),
                        (0, Some(max)) => {
                            if max == 1 {
                                format!("up to one {base}{controller_suffix}{random_suffix}")
                            } else {
                                format!(
                                    "up to {} {plural}{controller_suffix}{random_suffix}",
                                    count_text(max)
                                )
                            }
                        }
                        (min, Some(max)) if min == max => {
                            if min == 1 {
                                format!("one {base}{controller_suffix}{random_suffix}")
                            } else {
                                format!(
                                    "{} {plural}{controller_suffix}{random_suffix}",
                                    count_text(min)
                                )
                            }
                        }
                        (min, Some(max)) => {
                            format!(
                                "{} to {} {plural}{controller_suffix}{random_suffix}",
                                count_text(min),
                                count_text(max)
                            )
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod grouped_damage_event_value_surface_tests {
    use super::*;

    #[test]
    fn damaged_opponent_event_amount_preserves_its_authored_count_basis() {
        let value = Value::EventValue(EventValueSpec::Amount)
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::OpponentsDealtDamageThisWay);

        assert_eq!(
            describe_value(&value),
            "the number of opponents dealt damage this way"
        );
        assert_eq!(
            describe_card_count(&value),
            "cards equal to the number of opponents dealt damage this way"
        );
    }
}

#[cfg(test)]
mod chosen_object_set_surface_tests {
    use super::*;

    #[test]
    fn all_over_durable_chosen_tag_preserves_controller_partition() {
        for (controller, expected) in [
            (PlayerFilter::You, "the chosen permanents you control"),
            (
                PlayerFilter::NotYou,
                "the chosen permanents you don't control",
            ),
        ] {
            let filter = ObjectFilter::permanent()
                .in_zone(Zone::Battlefield)
                .controlled_by(controller)
                .match_tagged(
                    CHOSEN_OBJECTS_SURFACE_TAG,
                    TaggedOpbjectRelation::IsTaggedObject,
                );
            assert_eq!(describe_choose_spec(&ChooseSpec::All(filter)), expected);
        }
    }

    #[test]
    fn ordinary_implicit_tag_keeps_demonstrative_surface() {
        let filter = ObjectFilter::creature()
            .in_zone(Zone::Battlefield)
            .match_tagged("__it__", TaggedOpbjectRelation::IsTaggedObject);
        assert_eq!(
            describe_choose_spec(&ChooseSpec::All(filter)),
            "those creatures"
        );
    }
}

pub(crate) fn describe_simple_exiled_card_filter(filter: &ObjectFilter) -> Option<String> {
    if filter.zone != Some(Zone::Exile) {
        return None;
    }

    let mut base = filter.clone();
    let face_down = base.face_down;
    base.zone = None;
    base.face_down = None;
    if base != ObjectFilter::default() {
        return None;
    }

    let base = match face_down {
        Some(false) => "face-up exiled card",
        Some(true) => "face-down exiled card",
        None => "exiled card",
    };
    Some(base.to_string())
}

pub(crate) fn describe_attach_objects_spec(spec: &ChooseSpec) -> String {
    if let ChooseSpec::WithCount(inner, count) = spec
        && count.is_single()
        && !inner.is_target()
        && let Some(filter) = match inner.unhinted() {
            ChooseSpec::All(filter) | ChooseSpec::Object(filter) => Some(filter),
            _ => None,
        }
        && filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        })
    {
        return "one of them".to_string();
    }
    if let ChooseSpec::WithCount(inner, count) = spec
        && !inner.is_target()
        && let Some(text) = describe_counted_attach_objects_spec(inner, count)
    {
        return text;
    }
    if let ChooseSpec::All(filter) = spec
        && filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        })
    {
        if filter.subtypes.contains(&Subtype::Equipment) {
            return "that Equipment".to_string();
        }
        if filter.subtypes.contains(&Subtype::Aura) {
            return "that Aura".to_string();
        }
        if filter.card_types.is_empty() && filter.subtypes.is_empty() {
            return "it".to_string();
        }
    }
    if let ChooseSpec::All(filter) = spec
        && filter.zone == Some(Zone::Battlefield)
        && filter.controller.is_none()
        && filter.subtypes.as_slice() == [Subtype::Equipment]
        && filter.tagged_constraints.is_empty()
    {
        return "all Equipment on the battlefield".to_string();
    }
    describe_choose_spec(spec)
}

pub(crate) fn describe_counted_attach_objects_spec(
    spec: &ChooseSpec,
    count: &ChoiceCount,
) -> Option<String> {
    let filter = match spec.unhinted() {
        ChooseSpec::All(filter) | ChooseSpec::Object(filter) => filter,
        _ => return None,
    };
    let description = filter.description();
    let base = strip_leading_article(&description).trim();
    if base.is_empty() {
        return None;
    }
    let plural = pluralize_relative_object_phrase(base);
    let count_text = |n: usize| number_word(n as i32).unwrap_or_else(|| n.to_string());
    let random_suffix = if count.is_random() {
        if count.is_single() {
            " chosen at random"
        } else {
            " at random"
        }
    } else {
        ""
    };

    if count.is_single() {
        return Some(format!("{}{random_suffix}", with_indefinite_article(base)));
    }
    if count.is_up_to_dynamic_x() {
        return Some(format!("up to X {plural}{random_suffix}"));
    }
    if count.is_dynamic_x() {
        return Some(format!("X {plural}{random_suffix}"));
    }
    Some(match (count.min, count.max) {
        (0, None) => format!("any number of {plural}{random_suffix}"),
        (min, None) => {
            if min == 1 {
                format!("at least one {base}{random_suffix}")
            } else {
                format!("at least {} {plural}{random_suffix}", count_text(min))
            }
        }
        (0, Some(max)) => {
            if max == 1 {
                format!("up to one {base}{random_suffix}")
            } else {
                format!("up to {} {plural}{random_suffix}", count_text(max))
            }
        }
        (min, Some(max)) if min == max => {
            if min == 1 {
                format!("one {base}{random_suffix}")
            } else {
                format!("{} {plural}{random_suffix}", count_text(min))
            }
        }
        (min, Some(max)) => format!(
            "{} to {} {plural}{random_suffix}",
            count_text(min),
            count_text(max)
        ),
    })
}

pub(crate) fn describe_goad_target(spec: &ChooseSpec) -> String {
    match spec {
        ChooseSpec::Target(inner) => {
            if let ChooseSpec::Object(filter) = inner.as_ref()
                && filter.zone == Some(Zone::Battlefield)
                && filter.card_types == vec![CardType::Creature]
                && filter.controller == Some(PlayerFilter::Defending)
                && filter.subtypes.is_empty()
            {
                return "target creature that player controls".to_string();
            }
            describe_choose_spec(spec)
        }
        ChooseSpec::Tagged(tag) => {
            if tag.as_str().starts_with("counters_") {
                return "each creature that had counters put on it this way".to_string();
            }
            if is_implicit_reference_tag(tag.as_str()) {
                return "that creature".to_string();
            }
            describe_choose_spec(spec)
        }
        ChooseSpec::All(filter) => {
            if filter.set_quantifier_surface() == Some(ironsmith_core::SetQuantifierSurface::All)
                && let Some(source_surface) = filter.chosen_name_source_surface()
                && matches!(
                    filter.tagged_constraints.as_slice(),
                    [constraint]
                        if constraint.tag.as_str() == "__chosen_name__"
                            && constraint.relation
                                == TaggedOpbjectRelation::SameNameAsTagged
                )
            {
                let mut base = filter.clone();
                base.tagged_constraints.clear();
                if base == ObjectFilter::creature() {
                    return format!(
                        "all creatures with a name chosen for {}",
                        source_surface.phrase()
                    );
                }
            }
            if filter.has_relative_attachment_state_surface()
                && let Some(attachment) = filter.with_attached_object.as_deref()
            {
                let mut host = filter.clone();
                host.with_attached_object = None;
                host.set_relative_attachment_state_surface(false);
                return format!(
                    "each {} that's enchanted by {}",
                    describe_for_each_count_filter(&host),
                    with_indefinite_article(&attachment.description())
                );
            }
            if filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                    && constraint.tag.as_str().starts_with("counters_")
            }) {
                return "each creature that had counters put on it this way".to_string();
            }
            let looks_like_plain_creature_filter = filter.zone == Some(Zone::Battlefield)
                && filter.card_types == vec![CardType::Creature]
                && filter.all_card_types.is_empty()
                && filter.excluded_card_types.is_empty()
                && filter.subtypes.is_empty()
                && filter.excluded_subtypes.is_empty()
                && filter.with_attached_object.is_none()
                && filter.without_attached_object.is_none()
                && !filter.suspected
                && !filter.source;
            if looks_like_plain_creature_filter {
                if let Some(controller) = filter.controller.as_ref() {
                    return match controller {
                        PlayerFilter::Opponent => {
                            "all creatures your opponents control".to_string()
                        }
                        PlayerFilter::NotYou => "all creatures you don't control".to_string(),
                        PlayerFilter::Target(inner) => {
                            let described = describe_player_filter(inner);
                            let who = strip_leading_article(&described);
                            if who == "player" {
                                "each creature target player controls".to_string()
                            } else {
                                format!("each creature target {who} controls")
                            }
                        }
                        PlayerFilter::AliasedTarget(_) => {
                            "each creature that player controls".to_string()
                        }
                        PlayerFilter::IteratedPlayer => {
                            "each creature that player controls".to_string()
                        }
                        PlayerFilter::You => "each creature you control".to_string(),
                        _ => describe_choose_spec(spec),
                    };
                }
                return "each creature".to_string();
            }
            describe_choose_spec(spec)
        }
        _ => describe_choose_spec(spec),
    }
}

pub(crate) fn describe_transform_target(spec: &ChooseSpec) -> String {
    match spec {
        // Oracle text overwhelmingly uses "this creature" for source transforms
        // and this keeps compiled wording aligned with parser normalization.
        ChooseSpec::Source => "this creature".to_string(),
        _ => describe_choose_spec(spec),
    }
}

pub(crate) fn describe_flip_target(spec: &ChooseSpec) -> String {
    match spec {
        ChooseSpec::Source => "it".to_string(),
        _ => describe_choose_spec(spec),
    }
}

pub(crate) fn owner_for_zone_from_spec(
    spec: &ChooseSpec,
    zone: Zone,
) -> Option<Option<PlayerFilter>> {
    match spec {
        ChooseSpec::SurfaceHinted { spec: inner, .. }
        | ChooseSpec::Target(inner)
        | ChooseSpec::WithCount(inner, _)
        | ChooseSpec::WithCountValue(inner, _, _) => owner_for_zone_from_spec(inner, zone),
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            if filter.zone == Some(zone) {
                Some(filter.owner.clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

pub(crate) fn graveyard_owner_from_spec(spec: &ChooseSpec) -> Option<Option<PlayerFilter>> {
    owner_for_zone_from_spec(spec, Zone::Graveyard)
}

/// Render the current-turn graveyard-entry clause retained on an object
/// selection after a zone-specific effect renderer has emitted the graveyard
/// origin separately.
pub(crate) fn graveyard_entry_history_clause_for_spec(spec: &ChooseSpec) -> String {
    let filter = match spec.base() {
        ChooseSpec::Object(filter) | ChooseSpec::All(filter)
            if filter.zone == Some(Zone::Graveyard) =>
        {
            filter
        }
        _ => return String::new(),
    };
    let phrase = if filter.entered_graveyard_from_library_this_turn {
        "put there from their library this turn"
    } else if filter.entered_graveyard_from_battlefield_this_turn {
        "put there from the battlefield this turn"
    } else if filter.entered_graveyard_this_turn {
        match filter.graveyard_entry_history_surface() {
            Some(ironsmith_core::GraveyardEntryHistorySurface::PutThereFromAnywhereThisTurn) => {
                "put there from anywhere this turn"
            }
            _ => "put there this turn",
        }
    } else {
        return String::new();
    };
    let verb = if choose_spec_is_plural(spec) {
        "were"
    } else {
        "was"
    };
    format!(" that {verb} {phrase}")
}

pub(crate) fn graveyard_spec_is_single(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::Target(inner)
        | ChooseSpec::WithCount(inner, _)
        | ChooseSpec::WithCountValue(inner, _, _) => graveyard_spec_is_single(inner),
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            filter.zone == Some(Zone::Graveyard) && filter.single_graveyard
        }
        _ => false,
    }
}

pub(crate) fn hand_owner_from_spec(spec: &ChooseSpec) -> Option<Option<PlayerFilter>> {
    owner_for_zone_from_spec(spec, Zone::Hand)
}

pub(crate) fn is_you_owned_battlefield_object_spec(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::Target(inner)
        | ChooseSpec::WithCount(inner, _)
        | ChooseSpec::WithCountValue(inner, _, _) => is_you_owned_battlefield_object_spec(inner),
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            filter.zone == Some(Zone::Battlefield) && filter.owner == Some(PlayerFilter::You)
        }
        _ => false,
    }
}

pub(crate) fn describe_card_choice_count(count: ChoiceCount) -> String {
    if count.is_up_to_dynamic_x() {
        return "up to X cards".to_string();
    }
    if count.is_dynamic_x() {
        return "X cards".to_string();
    }
    match (count.min, count.max) {
        (1, Some(1)) => "a card".to_string(),
        (min, Some(max)) if min == max => format!("{min} cards"),
        (0, Some(max)) => format!("up to {max} cards"),
        (0, None) => "any number of cards".to_string(),
        (min, None) => format!("at least {min} cards"),
        (min, Some(max)) => format!("{min} to {max} cards"),
    }
}

pub(crate) fn describe_choose_spec_without_graveyard_zone(spec: &ChooseSpec) -> String {
    match spec {
        ChooseSpec::SurfaceHinted { spec: inner, .. } => {
            describe_choose_spec_without_graveyard_zone(inner)
        }
        ChooseSpec::Target(inner) => {
            if let Some(tagged_text) = describe_demonstrative_tagged_object_spec(inner.as_ref()) {
                return tagged_text;
            }
            let inner_text = describe_choose_spec_without_graveyard_zone(inner);
            if inner_text == "it" {
                inner_text
            } else if inner_text.starts_with("this ")
                || inner_text.starts_with("that ")
                || inner_text.starts_with("those ")
            {
                inner_text
            } else if inner_text.starts_with("target ") {
                inner_text
            } else if let Some(rest) = inner_text.strip_prefix("another ") {
                format!("another target {rest}")
            } else if let Some(rest) = inner_text.strip_prefix("other ") {
                format!("other target {rest}")
            } else {
                let stripped = strip_leading_article(&inner_text);
                if stripped == inner_text {
                    format!("target {inner_text}")
                } else {
                    format!("target {stripped}")
                }
            }
        }
        ChooseSpec::Object(filter) => {
            if let Some(tagged_text) = describe_demonstrative_tagged_object_filter(filter) {
                return tagged_text;
            }
            if filter.zone == Some(Zone::Graveyard) {
                // This renderer is used when the caller emits the origin
                // separately ("from their graveyard"). Re-render the typed
                // object filter in its nonbattlefield context, then remove the
                // redundant location. Clearing the zone first would make an
                // unconstrained card fall back to the battlefield noun
                // "permanent".
                let mut object = filter.clone();
                object.owner = None;
                object.single_graveyard = false;
                // The caller renders graveyard-entry history after the origin
                // ("from a graveyard that was put there this turn"). Keeping
                // the same predicate on this noun-only copy would render it
                // once here and once again beside the origin.
                object.entered_graveyard_this_turn = false;
                object.entered_graveyard_from_battlefield_this_turn = false;
                object.entered_graveyard_from_library_this_turn = false;
                object.set_graveyard_entry_history_surface(None);
                let text =
                    describe_nonbattlefield_card_filter_without_zone(&object, Zone::Graveyard);
                return ensure_indefinite_article(&render_artifact_non_aura_enchantment_text(
                    &object, &text,
                ));
            }
            ensure_indefinite_article(&render_artifact_non_aura_enchantment_text(
                filter,
                &describe_object_filter_with_fixed_pt_shorthand(filter),
            ))
        }
        ChooseSpec::PlayerOrPlaneswalker(filter) => match filter {
            PlayerFilter::Opponent => "target opponent or planeswalker".to_string(),
            PlayerFilter::Any => "target player or planeswalker".to_string(),
            other => format!("target {} or planeswalker", describe_player_filter(other)),
        },
        ChooseSpec::ObjectOrPlayer(object_filter, player_filter) => {
            describe_object_or_player_union(
                object_filter,
                describe_choose_spec_without_graveyard_zone(&ChooseSpec::Object(
                    object_filter.clone(),
                )),
                player_filter,
            )
        }
        ChooseSpec::AttackedPlayerOrPlaneswalker => {
            "the player or planeswalker it's attacking".to_string()
        }
        ChooseSpec::All(filter) => {
            if filter.zone == Some(Zone::Graveyard) {
                let mut objects = filter.clone();
                objects.owner = None;
                objects.single_graveyard = false;
                objects.entered_graveyard_this_turn = false;
                objects.entered_graveyard_from_battlefield_this_turn = false;
                objects.entered_graveyard_from_library_this_turn = false;
                objects.set_graveyard_entry_history_surface(None);
                let text =
                    describe_nonbattlefield_card_filter_without_zone(&objects, Zone::Graveyard);
                let text = strip_leading_article(&text);
                return format!("all {}", pluralize_relative_object_phrase(text));
            }
            let desc = describe_object_filter_with_fixed_pt_shorthand(filter);
            let stripped = strip_leading_article(&desc);
            format!("all {}", pluralize_relative_object_phrase(stripped))
        }
        ChooseSpec::WithCount(inner, count) | ChooseSpec::WithCountValue(inner, count, _) => {
            let inner_text = describe_choose_spec_without_graveyard_zone(inner);
            let random_suffix = if count.is_random() { " at random" } else { "" };
            if count.is_single() {
                format!("{inner_text}{random_suffix}")
            } else {
                if let ChooseSpec::Target(target_inner) = inner.as_ref() {
                    let target_desc = describe_choose_spec_without_graveyard_zone(target_inner);
                    let base = strip_leading_article(&target_desc);
                    let plural = render_counted_artifact_non_aura_enchantment_text(
                        target_inner,
                        &pluralize_noun_phrase(base),
                    );
                    let count_text =
                        |n: usize| number_word(n as i32).unwrap_or_else(|| n.to_string());
                    if count.is_up_to_dynamic_x() {
                        return format!("up to X target {plural}{random_suffix}");
                    }
                    if count.is_dynamic_x() {
                        return format!("X target {plural}{random_suffix}");
                    }
                    match (count.min, count.max) {
                        (0, None) => {
                            format!("any number of target {plural}{random_suffix}")
                        }
                        (1, None) => format!("one or more target {plural}{random_suffix}"),
                        (min, None) => {
                            format!("at least {min} target {plural}{random_suffix}")
                        }
                        (0, Some(max)) => {
                            if max == 1 {
                                format!("up to one target {base}{random_suffix}")
                            } else {
                                format!("up to {} target {plural}{random_suffix}", count_text(max))
                            }
                        }
                        (min, Some(max)) if min == max => {
                            if min == 1 {
                                format!("target {base}{random_suffix}")
                            } else {
                                format!("{} target {plural}{random_suffix}", count_text(min))
                            }
                        }
                        (1, Some(2)) => {
                            format!("one or two target {plural}{random_suffix}")
                        }
                        (1, Some(3)) => {
                            format!("one, two, or three target {plural}{random_suffix}")
                        }
                        (min, Some(max)) => {
                            format!(
                                "{} to {} target {plural}{random_suffix}",
                                count_text(min),
                                count_text(max)
                            )
                        }
                    }
                } else {
                    let base = strip_leading_article(&inner_text);
                    let plural = pluralize_noun_phrase(base);
                    let count_text = |n: usize| {
                        small_number_word(n as u32)
                            .or_else(|| number_word(n as i32))
                            .unwrap_or_else(|| n.to_string())
                    };
                    if count.is_up_to_dynamic_x() {
                        return format!("up to X {plural}{random_suffix}");
                    }
                    if count.is_dynamic_x() {
                        return format!("X {plural}{random_suffix}");
                    }
                    match (count.min, count.max) {
                        (0, None) => format!("any number of {plural}{random_suffix}"),
                        (1, None) => format!("one or more {plural}{random_suffix}"),
                        (min, None) => {
                            format!("at least {} {plural}{random_suffix}", count_text(min))
                        }
                        (0, Some(max)) => {
                            if max == 1 {
                                format!("up to one {base}{random_suffix}")
                            } else {
                                format!("up to {} {plural}{random_suffix}", count_text(max))
                            }
                        }
                        (min, Some(max)) if min == max => {
                            if min == 1 {
                                format!("one {base}{random_suffix}")
                            } else {
                                format!("{} {plural}{random_suffix}", count_text(min))
                            }
                        }
                        (min, Some(max)) => {
                            format!(
                                "{} to {} {plural}{random_suffix}",
                                count_text(min),
                                count_text(max)
                            )
                        }
                    }
                }
            }
        }
        _ => describe_choose_spec(spec),
    }
}

pub(crate) fn is_artifact_non_aura_enchantment_mana_value_filter(filter: &ObjectFilter) -> bool {
    let has_artifact_enchantment_types = filter.card_types.len() == 2
        && filter.card_types.contains(&CardType::Artifact)
        && filter.card_types.contains(&CardType::Enchantment)
        && filter.excluded_subtypes == [Subtype::Aura];
    let has_artifact_enchantment_union = filter.card_types.is_empty()
        && filter.excluded_subtypes.is_empty()
        && filter.any_of.len() == 2
        && filter.any_of.iter().any(|branch| {
            branch.card_types == [CardType::Artifact] && branch.excluded_subtypes.is_empty()
        })
        && filter.any_of.iter().any(|branch| {
            branch.card_types == [CardType::Enchantment]
                && branch.excluded_subtypes == [Subtype::Aura]
        })
        && filter.any_of.iter().all(|branch| {
            let mut remaining = branch.clone();
            remaining.card_types.clear();
            remaining.excluded_subtypes.clear();
            remaining.union_surface = Default::default();
            remaining == ObjectFilter::default()
        });
    if (!has_artifact_enchantment_types && !has_artifact_enchantment_union)
        || filter.mana_value.is_none()
    {
        return false;
    }

    let mut remaining = filter.clone();
    remaining.zone = None;
    remaining.owner = None;
    remaining.single_graveyard = false;
    remaining.card_types.clear();
    remaining.excluded_subtypes.clear();
    remaining.mana_value = None;
    remaining.any_of.clear();
    remaining.union_surface = Default::default();
    remaining == ObjectFilter::default()
}

pub(crate) fn render_artifact_non_aura_enchantment_text(
    filter: &ObjectFilter,
    text: &str,
) -> String {
    if !is_artifact_non_aura_enchantment_mana_value_filter(filter) {
        return text.to_string();
    }

    let Some((_, mana_value_text)) = text.split_once(" with mana value ") else {
        return text.to_string();
    };
    let mana_value_text = mana_value_text.trim();
    if text.contains("artifacts or enchantment cards with mana value")
        || text.contains("artifact and/or non-aura enchantment cards with mana value")
    {
        format!("artifact and/or non-Aura enchantment cards each with mana value {mana_value_text}")
    } else if text.contains("artifact or enchantment card with mana value")
        || text.contains("artifact and/or non-aura enchantment card with mana value")
    {
        format!("artifact and/or non-Aura enchantment card with mana value {mana_value_text}")
    } else {
        text.to_string()
    }
}

pub(crate) fn render_counted_artifact_non_aura_enchantment_text(
    spec: &ChooseSpec,
    text: &str,
) -> String {
    let ChooseSpec::Object(filter) = spec else {
        return text.to_string();
    };
    if !is_artifact_non_aura_enchantment_mana_value_filter(filter) {
        return text.to_string();
    }
    text.replace(
        "artifact and/or non-Aura enchantment cards with mana value",
        "artifact and/or non-Aura enchantment cards each with mana value",
    )
    .replace(
        "artifact and/or non-aura enchantment cards with mana value",
        "artifact and/or non-Aura enchantment cards each with mana value",
    )
}

pub(crate) fn describe_choice_count(count: &ChoiceCount) -> String {
    let count_word = |value: usize| number_word(value as i32).unwrap_or_else(|| value.to_string());
    let base = if count.is_up_to_dynamic_x() {
        "up to X".to_string()
    } else if count.is_dynamic_x() {
        if count.explicit_exactly {
            "exactly X".to_string()
        } else {
            "X".to_string()
        }
    } else {
        match (count.min, count.max) {
            (0, None) => "any number".to_string(),
            (1, None) => "one or more".to_string(),
            (min, None) => format!("at least {}", count_word(min)),
            (0, Some(max)) => format!("up to {}", count_word(max)),
            (min, Some(max)) if min == max => format!("exactly {}", count_word(min)),
            (min, Some(max)) => format!("{} to {}", count_word(min), count_word(max)),
        }
    };
    if count.is_random() {
        format!("{base} at random")
    } else {
        base
    }
}

pub(crate) fn ensure_trailing_period(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.ends_with('.')
        || trimmed.ends_with('!')
        || trimmed.ends_with('?')
        || trimmed.ends_with('—')
        || trimmed.ends_with('"')
        || trimmed.ends_with(')')
    {
        trimmed.to_string()
    } else {
        format!("{trimmed}.")
    }
}

pub(crate) fn is_nonbattlefield_card_zone(zone: Zone) -> bool {
    matches!(
        zone,
        Zone::Graveyard
            | Zone::Hand
            | Zone::Library
            | Zone::Exile
            | Zone::Command
            | Zone::OutsideGame
    )
}

fn rendered_filter_zone_clause(filter: &ObjectFilter) -> Option<String> {
    let zone = filter.zone?;
    let zone_name = match zone {
        Zone::Graveyard => "graveyard",
        Zone::Hand => "hand",
        Zone::Library => "library",
        Zone::Exile => "exile",
        Zone::Command => "command zone",
        Zone::Ante => "ante",
        Zone::OutsideGame => "outside the game",
        Zone::Battlefield | Zone::Stack => return None,
    };
    Some(if let Some(owner) = &filter.owner {
        format!(
            "in {} {zone_name}",
            describe_possessive_player_filter(owner)
        )
    } else if zone == Zone::Graveyard && filter.single_graveyard {
        "in single graveyard".to_string()
    } else if zone == Zone::Graveyard {
        "in a graveyard".to_string()
    } else {
        format!("in {zone_name}")
    })
}

/// Describe a card filter with its nonbattlefield zone still available to the
/// noun renderer, then remove only the redundant zone clause from the surface.
/// This keeps an unconstrained library filter as `card` while preserving an
/// explicit six-type restriction as `permanent card`.
pub(crate) fn describe_nonbattlefield_card_filter_without_zone(
    filter: &ObjectFilter,
    zone: Zone,
) -> String {
    debug_assert!(is_nonbattlefield_card_zone(zone));
    let mut contextual_filter = filter.clone();
    contextual_filter.zone = Some(zone);
    let mut description = contextual_filter.description();
    if let Some(zone_clause) = rendered_filter_zone_clause(&contextual_filter) {
        let needle = format!(" {zone_clause}");
        if let Some(start) = description.rfind(&needle) {
            description.replace_range(start..start + needle.len(), "");
        }
    }
    description.trim().to_string()
}

pub(crate) fn filter_explicitly_selects_permanent_cards(filter: &ObjectFilter) -> bool {
    let permanent_types = [
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];
    filter.card_types.len() == permanent_types.len()
        && permanent_types
            .iter()
            .all(|card_type| filter.card_types.contains(card_type))
}

pub(crate) fn describe_single_search_filter_in_zone(filter: &ObjectFilter, zone: Zone) -> String {
    let mut selection = describe_nonbattlefield_card_filter_without_zone(filter, zone);
    if filter_explicitly_selects_permanent_cards(filter) {
        if selection == "card" || selection.starts_with("card ") {
            selection = format!("permanent {selection}");
        }
        return with_indefinite_article(&selection);
    }
    if selection == "card" {
        return with_indefinite_article(&selection);
    }
    with_indefinite_article(&describe_search_selection_with_cards(&selection))
}

pub(crate) fn describe_search_selection_with_cards(selection: &str) -> String {
    fn pluralize_counted_card_phrase(phrase: &str) -> String {
        let phrase = phrase.trim();
        if phrase == "cards" || phrase.starts_with("cards ") || phrase.contains(" cards") {
            return phrase.to_string();
        }
        if phrase == "card" {
            return "cards".to_string();
        }
        if let Some(tail) = phrase.strip_prefix("card ") {
            return format!("cards {tail}");
        }
        if let Some((head, tail)) = phrase.split_once(" card") {
            return format!("{head} cards{tail}");
        }
        format!("{phrase} cards")
    }

    let selection = selection.trim();
    if selection.is_empty() {
        return "a card".to_string();
    }
    if let Some(rest) = selection.strip_prefix("all ") {
        let rest = rest.trim();
        if let Some(name) = rest.strip_prefix("permanent named ") {
            return format!("all cards named {name}");
        }
        if let Some(name) = rest.strip_prefix("card named ") {
            return format!("all cards named {name}");
        }
        if rest == "nonland permanent" || rest == "nonland permanent card" {
            return "all nonland cards".to_string();
        }
        if matches!(rest, "permanent" | "permanent card" | "card") {
            return "all cards".to_string();
        }
        if let Some(tail) = rest.strip_prefix("permanent ") {
            return format!("all cards {tail}");
        }
        if let Some(tail) = rest.strip_prefix("card ") {
            return format!("all cards {tail}");
        }
        if rest.contains(" cards") {
            return selection.to_string();
        }
    }
    if let Some(name) = selection.strip_prefix("a permanent named ") {
        return format!("a card named {name}");
    }
    if let Some(name) = selection.strip_prefix("permanent named ") {
        return format!("a card named {name}");
    }
    let unarticled = strip_leading_article(selection);
    if unarticled.starts_with("basic land card with ") || unarticled.starts_with("land card with ")
    {
        // `with ...` is a qualifier on the land card, not a subtype that
        // should be moved in front of the shared `card` noun.
        return with_indefinite_article(unarticled);
    }
    if let Some(subtype) = selection.strip_prefix("a basic land card ") {
        if subtype.starts_with("that ") {
            return selection.to_string();
        }
        return format!("a basic {} card", subtype.trim());
    }
    if let Some(subtype) = selection.strip_prefix("basic land card ") {
        if subtype.starts_with("that ") {
            return format!("a {selection}");
        }
        return format!("a basic {} card", subtype.trim());
    }
    if let Some(subtype) = selection.strip_prefix("a land card ") {
        if subtype.starts_with("that ") {
            return selection.to_string();
        }
        return format!("{} card", with_indefinite_article(subtype.trim()));
    }
    if let Some(subtype) = selection.strip_prefix("land card ") {
        if subtype.starts_with("that ") {
            return format!("a {selection}");
        }
        return format!("{} card", with_indefinite_article(subtype.trim()));
    }
    if selection == "permanent" || selection == "permanent card" {
        return "a card".to_string();
    }
    if let Some(tail) = selection.strip_prefix("permanent ") {
        return format!("a card {tail}");
    }
    if let Some(head) = selection.strip_suffix(" permanent card") {
        return format!("{} card", with_indefinite_article(head.trim()));
    }
    if let Some(head) = selection.strip_suffix(" permanent") {
        return format!("{} card", with_indefinite_article(head.trim()));
    }
    if let Some(tail) = selection.strip_prefix("card ") {
        return format!("a card {tail}");
    }
    if selection == "nonland permanent" || selection == "nonland permanent card" {
        return "a nonland card".to_string();
    }
    if let Some((head, tail)) = selection.split_once(" with mana value ") {
        let head = head.trim();
        let value = tail.trim_end_matches(" card").trim();
        if !head.is_empty() && !value.is_empty() {
            if matches!(head, "a permanent" | "permanent" | "permanent card") {
                return format!("a card with mana value {value}");
            }
            let head_with_card = if head.ends_with(" card") || head.ends_with(" cards") {
                head.to_string()
            } else {
                format!("{} card", with_indefinite_article(head))
            };
            return format!("{head_with_card} with mana value {value}");
        }
    }
    let card_union_parts = selection.split(" or ").map(str::trim).collect::<Vec<_>>();
    if card_union_parts.len() > 1
        && card_union_parts
            .iter()
            .all(|part| part.strip_suffix(" card").is_some())
    {
        let modifiers = card_union_parts
            .iter()
            .filter_map(|part| part.strip_suffix(" card"))
            .map(|part| {
                part.trim()
                    .strip_prefix("an ")
                    .or_else(|| part.trim().strip_prefix("a "))
                    .unwrap_or(part.trim())
            })
            .collect::<Vec<_>>();
        if modifiers.iter().all(|modifier| !modifier.is_empty()) {
            return with_indefinite_article(&format!("{} card", modifiers.join(" or ")));
        }
    }
    if let Some(rest) = selection.strip_prefix("up to ") {
        let mut parts = rest.splitn(2, ' ');
        let amount = parts.next().unwrap_or_default().trim();
        let tail = parts.next().unwrap_or_default().trim();
        if !tail.is_empty() {
            if let Some((noun, qualifier)) = tail.split_once(" with ") {
                let noun = noun.trim();
                let qualifier = qualifier.trim();
                if !noun.is_empty() && !qualifier.is_empty() {
                    if amount == "1" || amount.eq_ignore_ascii_case("one") {
                        let singular = if noun == "card" || noun.contains(" card") {
                            with_indefinite_article(noun)
                        } else {
                            format!("{} card", with_indefinite_article(noun))
                        };
                        return format!("{singular} with {qualifier}");
                    }
                    return format!(
                        "up to {amount} {} with {qualifier}",
                        pluralize_counted_card_phrase(noun)
                    );
                }
            }
            if amount == "1" || amount.eq_ignore_ascii_case("one") {
                return if tail == "card" || tail.contains(" card") {
                    with_indefinite_article(tail)
                } else {
                    format!("a {tail} card")
                };
            }
            return format!("up to {amount} {}", pluralize_counted_card_phrase(tail));
        }
    }
    if let Some(rest) = selection.strip_prefix("any number ") {
        let rest = rest.trim_start_matches("of ").trim();
        if !rest.is_empty() {
            if let Some((noun, qualifier)) = rest.split_once(" with ") {
                let noun = noun.trim();
                let qualifier = qualifier.trim();
                if !noun.is_empty() && !qualifier.is_empty() {
                    return format!(
                        "any number of {} with {qualifier}",
                        pluralize_counted_card_phrase(noun)
                    );
                }
            }
            return format!("any number of {}", pluralize_counted_card_phrase(rest));
        }
    }
    if let Some(rest) = selection.strip_prefix("a number of ") {
        return format!("a number of {}", pluralize_counted_card_phrase(rest));
    }
    if selection.contains(" card") {
        return selection.to_string();
    }
    if let Some((noun, qualifier)) = selection.split_once(" with ") {
        let noun = noun.trim();
        let qualifier = qualifier.trim();
        if !noun.is_empty() && !qualifier.is_empty() {
            return format!("{} card with {qualifier}", with_indefinite_article(noun));
        }
    }
    format!("{} card", with_indefinite_article(selection))
}

pub(crate) fn describe_revealed_selection_with_cards(selection: &str) -> String {
    let selection = selection.trim();
    if selection.is_empty() {
        return "card".to_string();
    }
    if let Some(rest) = selection.strip_prefix("all ") {
        let rest = rest.trim();
        if rest == "nonland permanent" || rest == "nonland permanent card" {
            return "all nonland permanent cards".to_string();
        }
        if rest == "permanent" || rest == "permanent card" {
            return "all permanent cards".to_string();
        }
        if let Some(tail) = rest.strip_prefix("nonland permanent ") {
            return format!("all nonland permanent cards {tail}");
        }
        if let Some(tail) = rest.strip_prefix("permanent ") {
            return format!("all permanent cards {tail}");
        }
    }
    if selection == "nonland permanent" || selection == "nonland permanent card" {
        return "nonland permanent card".to_string();
    }
    if selection == "permanent" || selection == "permanent card" {
        return "permanent card".to_string();
    }
    if let Some(name) = selection.strip_prefix("a permanent named ") {
        return format!("a permanent card named {name}");
    }
    if let Some(name) = selection.strip_prefix("permanent named ") {
        return format!("a permanent card named {name}");
    }
    if let Some(tail) = selection.strip_prefix("nonland permanent ") {
        return format!("nonland permanent card {tail}");
    }
    if let Some(tail) = selection.strip_prefix("permanent ") {
        return format!("permanent card {tail}");
    }
    describe_search_selection_with_cards(selection)
}

pub(crate) fn normalize_search_you_own_clause(text: &str) -> Option<String> {
    let rest = text.strip_prefix("Search your library for ")?;
    let (selection, tail) = rest.split_once(" you own")?;
    let selection = describe_search_selection_with_cards(selection);
    let tail = tail
        .replace(
            ", reveal it, put it into hand, then shuffle",
            ", reveal it, put it into your hand, then shuffle",
        )
        .replace(
            ", put it into hand, then shuffle",
            ", put it into your hand, then shuffle",
        );
    Some(format!("Search your library for {selection}{tail}"))
}

pub(crate) fn describe_mode_choice_header(
    max: &Value,
    min: Option<&Value>,
    mode_count: Option<usize>,
) -> String {
    if max.has_surface_hint(ValueSurfaceHint::WhereXIs) {
        let max_basis = describe_value(max);
        return match min {
            Some(Value::Fixed(0)) => format!("Choose up to X, where X is {max_basis} —"),
            Some(Value::Fixed(min_value)) => {
                let min_text = number_word(*min_value).unwrap_or_else(|| min_value.to_string());
                format!("Choose between {min_text} and X mode(s), where X is {max_basis} —")
            }
            Some(min) => format!(
                "Choose between {} and X mode(s), where X is {max_basis} —",
                describe_value(min)
            ),
            None => format!("Choose X mode(s), where X is {max_basis} —"),
        };
    }

    match (min, max) {
        (Some(Value::Fixed(min_value)), Value::Fixed(max_value)) => {
            match (*min_value, *max_value) {
                (0, 1) => "Choose up to one —".to_string(),
                (1, 1) => "Choose one —".to_string(),
                (1, 2) => "Choose one or both —".to_string(),
                (1, n) if n > 2 => "Choose one or more —".to_string(),
                (0, n) => {
                    if mode_count == Some(n as usize) && n > 1 {
                        return "Choose any number —".to_string();
                    }
                    if let Some(word) = number_word(n) {
                        format!("Choose up to {word} —")
                    } else {
                        format!("Choose up to {n} —")
                    }
                }
                (n, m) if n == m => {
                    if let Some(word) = number_word(n) {
                        format!("Choose {word} —")
                    } else {
                        format!("Choose {n} —")
                    }
                }
                _ => format!("Choose between {min_value} and {max_value} mode(s) —"),
            }
        }
        (None, Value::Fixed(value)) if *value > 0 => {
            if let Some(word) = number_word(*value) {
                format!("Choose {word} —")
            } else {
                format!("Choose {value} mode(s) —")
            }
        }
        (Some(Value::Fixed(0)), max) => {
            format!("Choose up to {} —", describe_value(max))
        }
        (Some(min), max) => format!(
            "Choose between {} and {} mode(s) —",
            describe_value(min),
            describe_value(max)
        ),
        (None, max) => format!("Choose {} mode(s) —", describe_value(max)),
    }
}

pub(crate) fn describe_compact_protection_choice(effect: &Effect) -> Option<String> {
    let choose_mode = effect.downcast_ref::<crate::effects::ChooseModeEffect>()?;
    if choose_mode.min_choose_count != choose_mode.choose_count
        || !matches!(choose_mode.choose_count, Value::Fixed(1))
    {
        return None;
    }

    let mut target: Option<&ChooseSpec> = None;
    let mut color_mode_count = 0usize;
    let mut allow_colorless = false;
    let mut card_type_modes = Vec::new();

    for mode in &choose_mode.modes {
        if mode.effects.len() != 1 {
            return None;
        }
        let grant = mode.effects[0].downcast_ref::<crate::effects::GrantAbilitiesTargetEffect>()?;
        if !matches!(grant.duration, Until::EndOfTurn) || grant.abilities.len() != 1 {
            return None;
        }
        match grant.abilities[0].protection_from()? {
            crate::ability::ProtectionFrom::Colorless => {
                if allow_colorless {
                    return None;
                }
                allow_colorless = true;
            }
            crate::ability::ProtectionFrom::CardType(card_type) => {
                if card_type_modes.contains(card_type) {
                    return None;
                }
                card_type_modes.push(*card_type);
            }
            crate::ability::ProtectionFrom::Color(colors) => {
                if colors.count() != 1 {
                    return None;
                }
                color_mode_count += 1;
            }
            _ => return None,
        }

        if let Some(existing) = target {
            if existing != &grant.target {
                return None;
            }
        } else {
            target = Some(&grant.target);
        }
    }

    let target_desc = describe_choose_spec(target?);
    let choice_owner = match choose_mode.chooser.as_ref() {
        None | Some(PlayerFilter::You) => "your".to_string(),
        Some(PlayerFilter::ControllerOf(crate::target::ObjectRef::Target)) => {
            "its controller's".to_string()
        }
        Some(chooser) => describe_possessive_player_filter(chooser),
    };
    const CHOOSABLE_CARD_TYPES: [crate::types::CardType; 9] = [
        crate::types::CardType::Artifact,
        crate::types::CardType::Battle,
        crate::types::CardType::Creature,
        crate::types::CardType::Enchantment,
        crate::types::CardType::Instant,
        crate::types::CardType::Kindred,
        crate::types::CardType::Land,
        crate::types::CardType::Planeswalker,
        crate::types::CardType::Sorcery,
    ];
    if color_mode_count == 0
        && !allow_colorless
        && card_type_modes.len() == CHOOSABLE_CARD_TYPES.len()
        && CHOOSABLE_CARD_TYPES
            .iter()
            .all(|card_type| card_type_modes.contains(card_type))
    {
        return Some(format!(
            "{target_desc} gains protection from the card type of {choice_owner} choice until end of turn"
        ));
    }

    let allow_artifacts = card_type_modes.as_slice() == [crate::types::CardType::Artifact];
    if color_mode_count != 5 || (allow_colorless && allow_artifacts) {
        return None;
    }
    Some(if allow_artifacts {
        format!(
            "{target_desc} gains protection from artifacts or from the color of {choice_owner} choice until end of turn"
        )
    } else if allow_colorless {
        format!(
            "{target_desc} gains protection from colorless or from the color of {choice_owner} choice until end of turn"
        )
    } else {
        format!(
            "{target_desc} gains protection from the color of {choice_owner} choice until end of turn"
        )
    })
}

pub(crate) fn describe_compact_destroy_color_choice(effect: &Effect) -> Option<String> {
    let choose_mode = effect.downcast_ref::<crate::effects::ChooseModeEffect>()?;
    if choose_mode.min_choose_count != choose_mode.choose_count
        || !matches!(choose_mode.choose_count, Value::Fixed(1))
        || choose_mode.modes.len() != 5
    {
        return None;
    }

    let mut base_filter: Option<crate::target::ObjectFilter> = None;
    let mut seen_colors = Vec::new();

    for mode in &choose_mode.modes {
        if mode.effects.len() != 1 {
            return None;
        }
        let destroy = mode.effects[0].downcast_ref::<crate::effects::DestroyEffect>()?;
        let ChooseSpec::All(filter) = &destroy.spec else {
            return None;
        };

        let colors = filter.colors?;
        if colors.count() != 1 {
            return None;
        }

        let color = crate::color::Color::ALL
            .iter()
            .copied()
            .find(|candidate| colors.contains(*candidate))?;
        if seen_colors.contains(&color) {
            return None;
        }
        seen_colors.push(color);

        let mut normalized_filter = filter.clone();
        normalized_filter.colors = None;
        if let Some(existing) = &base_filter {
            if existing != &normalized_filter {
                return None;
            }
        } else {
            base_filter = Some(normalized_filter);
        }
    }

    if seen_colors.len() != 5 {
        return None;
    }

    let base_desc = describe_choose_spec(&ChooseSpec::All(base_filter?));
    Some(format!(
        "Destroy {} of the color of your choice.",
        base_desc
    ))
}

pub(crate) fn describe_compact_return_to_hand_color_choice(effect: &Effect) -> Option<String> {
    let choose_mode = effect.downcast_ref::<crate::effects::ChooseModeEffect>()?;
    if choose_mode.min_choose_count != choose_mode.choose_count
        || !matches!(choose_mode.choose_count, Value::Fixed(1))
        || choose_mode.modes.len() != 5
    {
        return None;
    }

    let mut base_filter: Option<crate::target::ObjectFilter> = None;
    let mut seen_colors = Vec::new();

    for mode in &choose_mode.modes {
        if mode.effects.len() != 1 {
            return None;
        }
        let return_to_hand =
            mode.effects[0].downcast_ref::<crate::effects::ReturnToHandEffect>()?;
        let ChooseSpec::All(filter) = &return_to_hand.spec else {
            return None;
        };

        let colors = filter.colors?;
        if colors.count() != 1 {
            return None;
        }

        let color = crate::color::Color::ALL
            .iter()
            .copied()
            .find(|candidate| colors.contains(*candidate))?;
        if seen_colors.contains(&color) {
            return None;
        }
        seen_colors.push(color);

        let mut normalized_filter = filter.clone();
        normalized_filter.colors = None;
        if let Some(existing) = &base_filter {
            if existing != &normalized_filter {
                return None;
            }
        } else {
            base_filter = Some(normalized_filter);
        }
    }

    if seen_colors.len() != 5 {
        return None;
    }

    let base_spec = ChooseSpec::All(base_filter?);
    let base_desc = describe_choose_spec(&base_spec);
    Some(format!(
        "Return {} of the color of your choice to {}.",
        base_desc,
        owner_hand_phrase_for_spec(&base_spec)
    ))
}

fn describe_coordinated_keyword_choice_mode(effects: &[Effect]) -> Option<(String, String)> {
    let [effect] = effects else {
        return None;
    };
    let sequence = effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    if matches!(
        sequence.surface,
        ironsmith_core::SequenceSurface::Sequential
    ) || sequence.effects.len() < 2
    {
        return None;
    }

    let mut subjects = Vec::with_capacity(sequence.effects.len());
    let mut selected_ability = None;
    for effect in &sequence.effects {
        let apply = super::super::render_effects::unwrap_basic_tag_wrappers(effect)
            .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
        if !matches!(apply.until, Until::EndOfTurn)
            || !apply.additional_modifications.is_empty()
            || !apply.runtime_modifications.is_empty()
        {
            return None;
        }
        let Some(crate::continuous::Modification::AddAbility(ability)) = &apply.modification else {
            return None;
        };
        let ability_text = ability
            .granted_inline_ability()
            .map(describe_inline_ability)
            .unwrap_or_else(|| ability.display())
            .to_ascii_lowercase();
        if selected_ability
            .as_ref()
            .is_some_and(|expected| expected != &ability_text)
        {
            return None;
        }
        selected_ability = Some(ability_text);
        subjects.push(describe_apply_continuous_target(apply).0);
    }
    if subjects.iter().any(String::is_empty) || subjects.windows(2).any(|pair| pair[0] == pair[1]) {
        return None;
    }
    Some((join_with_and(&subjects), selected_ability?))
}

pub(crate) fn describe_permanent_keyword_choice_alternative(
    alternative_effects: &[Effect],
    primary_effects: &[Effect],
) -> Option<String> {
    let (effect, declared_target) = match alternative_effects {
        [effect] => (effect, None),
        [target_only, effect] => {
            let target_only = super::super::render_effects::unwrap_basic_tag_wrappers(target_only)
                .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
            (effect, Some(&target_only.target))
        }
        _ => return None,
    };
    let effect = super::super::render_effects::unwrap_basic_tag_wrappers(effect);
    let choose_mode = effect.downcast_ref::<crate::effects::ChooseModeEffect>()?;
    if choose_mode.min_choose_count != choose_mode.choose_count
        || !matches!(choose_mode.choose_count, Value::Fixed(1))
        || choose_mode.modes.len() < 2
    {
        return None;
    }

    let mut subject: Option<String> = None;
    let mut plural_subject = false;
    let mut abilities = Vec::with_capacity(choose_mode.modes.len());
    for mode in &choose_mode.modes {
        let [effect] = mode.effects.as_slice() else {
            return None;
        };
        let apply = super::super::render_effects::unwrap_basic_tag_wrappers(effect)
            .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
        if !matches!(apply.until, Until::Forever)
            || apply.condition.is_some()
            || !apply.additional_modifications.is_empty()
            || !apply.runtime_modifications.is_empty()
        {
            return None;
        }
        let Some(crate::continuous::Modification::AddAbility(ability)) = &apply.modification else {
            return None;
        };
        if !ability.is_keyword() {
            return None;
        }

        let (mode_subject, mode_plural_subject) = describe_apply_continuous_target(apply);
        if let Some(existing) = &subject {
            if existing != &mode_subject || plural_subject != mode_plural_subject {
                return None;
            }
        } else {
            subject = Some(mode_subject);
            plural_subject = mode_plural_subject;
        }
        let ability = ability.display().to_ascii_lowercase();
        if abilities.contains(&ability) {
            return None;
        }
        abilities.push(ability);
    }

    let mut subject = subject?;
    if subject.starts_with("target ") {
        let alternative_first = choose_mode.modes.first()?.effects.first()?;
        let alternative_target =
            super::super::render_effects::unwrap_basic_tag_wrappers(alternative_first)
                .downcast_ref::<crate::effects::ApplyContinuousEffect>()?
                .target_spec
                .as_ref();
        if declared_target.is_some_and(|declared_target| {
            alternative_target.is_none_or(|alternative_target| {
                declared_target.unhinted() != alternative_target.unhinted()
            })
        }) {
            return None;
        }
        let shares_primary_target = alternative_target.is_some_and(|alternative_target| {
            primary_effects.iter().any(|primary| {
                super::super::render_effects::unwrap_basic_tag_wrappers(primary)
                    .0
                    .get_target_spec()
                    .is_some_and(|primary_target| {
                        primary_target.unhinted() == alternative_target.unhinted()
                    })
            })
        });
        if shares_primary_target {
            subject.replace_range(.."target".len(), "that");
        }
    }

    let verb = if plural_subject { "gain" } else { "gains" };
    Some(format!("{subject} {verb} {}", join_with_or(&abilities)))
}

pub(crate) fn describe_compact_keyword_choice(effect: &Effect) -> Option<String> {
    let effect = if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>()
        && is_implicit_reference_tag(tagged.tag.as_str())
    {
        tagged.effect.as_ref()
    } else {
        effect
    };
    let choose_mode = effect.downcast_ref::<crate::effects::ChooseModeEffect>()?;
    if choose_mode.min_choose_count != choose_mode.choose_count
        || !matches!(choose_mode.choose_count, Value::Fixed(1))
        || choose_mode.modes.len() < 2
    {
        return None;
    }

    let mut subject: Option<String> = None;
    let mut plural_subject = false;
    let mut coordinated_subject: Option<bool> = None;
    let mut abilities = Vec::new();

    for mode in &choose_mode.modes {
        if let Some((mode_subject, ability)) =
            describe_coordinated_keyword_choice_mode(&mode.effects)
        {
            if coordinated_subject == Some(false) {
                return None;
            }
            coordinated_subject = Some(true);
            if let Some(existing) = &subject {
                if existing != &mode_subject {
                    return None;
                }
            } else {
                plural_subject = true;
                subject = Some(mode_subject);
            }
            abilities.push(ability);
            continue;
        }
        if mode.effects.len() != 1 {
            return None;
        }
        if coordinated_subject == Some(true) {
            return None;
        }
        coordinated_subject = Some(false);
        if let Some(grant_target) =
            mode.effects[0].downcast_ref::<crate::effects::GrantAbilitiesTargetEffect>()
        {
            if !matches!(grant_target.duration, Until::EndOfTurn)
                || grant_target.abilities.len() != 1
            {
                return None;
            }
            let mode_subject = describe_choose_spec(&grant_target.target);
            if let Some(existing) = &subject {
                if existing != &mode_subject {
                    return None;
                }
            } else {
                plural_subject = choose_spec_is_plural(&grant_target.target);
                subject = Some(mode_subject);
            }
            abilities.push(grant_target.abilities[0].display().to_ascii_lowercase());
            continue;
        }
        if let Some(grant_all) =
            mode.effects[0].downcast_ref::<crate::effects::GrantAbilitiesAllEffect>()
        {
            if !grant_all.filter.source
                || !matches!(grant_all.duration, Until::EndOfTurn)
                || grant_all.abilities.len() != 1
            {
                return None;
            }
            let mode_subject = if grant_all.filter.card_types.contains(&CardType::Creature) {
                "this creature".to_string()
            } else {
                "this permanent".to_string()
            };
            if let Some(existing) = &subject {
                if existing != &mode_subject {
                    return None;
                }
            } else {
                plural_subject = false;
                subject = Some(mode_subject);
            }
            abilities.push(grant_all.abilities[0].display().to_ascii_lowercase());
            continue;
        }
        if let Some(apply) = mode.effects[0].downcast_ref::<crate::effects::ApplyContinuousEffect>()
        {
            if !matches!(apply.until, Until::EndOfTurn)
                || !apply.additional_modifications.is_empty()
                || !apply.runtime_modifications.is_empty()
            {
                return None;
            }
            let Some(crate::continuous::Modification::AddAbility(ability)) = &apply.modification
            else {
                return None;
            };
            let (mode_subject, mode_plural_subject) = describe_apply_continuous_target(apply);
            if let Some(existing) = &subject {
                if existing != &mode_subject {
                    return None;
                }
            } else {
                plural_subject = mode_plural_subject;
                subject = Some(mode_subject);
            }
            abilities.push(
                ability
                    .granted_inline_ability()
                    .map(describe_inline_ability)
                    .unwrap_or_else(|| ability.display())
                    .to_ascii_lowercase(),
            );
            continue;
        }
        return None;
    }

    let mut unique_abilities = Vec::new();
    for ability in abilities {
        if !unique_abilities.contains(&ability) {
            unique_abilities.push(ability);
        }
    }
    let abilities = unique_abilities;
    if abilities.len() < 2 {
        return None;
    }

    let subject = subject?;
    let verb = if coordinated_subject == Some(true) {
        "both gain"
    } else if plural_subject {
        "gain"
    } else {
        "gains"
    };
    let choice_text = join_with_or(&abilities);
    Some(format!(
        "{subject} {verb} your choice of {choice_text} until end of turn"
    ))
}

#[cfg(test)]
mod optional_companion_keyword_choice_tests {
    use super::*;

    fn grant(spec: ChooseSpec, ability: crate::static_abilities::StaticAbility) -> Effect {
        Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
            spec,
            crate::continuous::Modification::AddAbility(ability),
            Until::EndOfTurn,
        ))
    }

    #[test]
    fn shared_keyword_choice_renders_required_and_optional_subjects_once() {
        let source = ChooseSpec::Source.with_surface_hint(
            crate::target::ChooseSpecSurfaceHint::SourceReference(
                crate::target::SourceReferenceSurface::ThisPermanentType(
                    "this creature".to_string(),
                ),
            ),
        );
        let mut companion_filter = ObjectFilter::creature().you_control();
        companion_filter.other = true;
        let companion = ChooseSpec::target(ChooseSpec::Object(companion_filter))
            .with_count(crate::effect::ChoiceCount::up_to(1));
        let mode = |ability: crate::static_abilities::StaticAbility| ironsmith_core::EffectMode {
            source_text: String::new(),
            effects: vec![Effect::new(crate::effects::SequenceEffect::coordinated(
                vec![
                    grant(source.clone(), ability.clone()),
                    grant(companion.clone(), ability),
                ],
            ))],
        };
        let choice = Effect::choose_one(vec![
            mode(crate::static_abilities::StaticAbility::first_strike()),
            mode(crate::static_abilities::StaticAbility::lifelink()),
        ]);

        assert_eq!(
            describe_compact_keyword_choice(&choice).as_deref(),
            Some(
                "this creature and up to one other target creature you control both gain your choice of first strike or lifelink until end of turn"
            )
        );
    }
}

#[cfg(test)]
mod structured_zone_count_surface_tests {
    use super::*;

    #[test]
    fn hand_and_owned_graveyard_counts_keep_plural_zone_scopes() {
        let all_hands = ObjectFilter::default().in_zone(Zone::Hand);
        assert_eq!(
            describe_value(&Value::Count(all_hands)),
            "the number of cards in all players' hands"
        );

        let mut opponent_graveyards = ObjectFilter::default().in_zone(Zone::Graveyard);
        opponent_graveyards.owner = Some(PlayerFilter::Opponent);
        assert_eq!(
            describe_value(&Value::Count(opponent_graveyards)),
            "the number of cards in your opponents' graveyards"
        );

        let mut chosen_graveyard = ObjectFilter::default().in_zone(Zone::Graveyard);
        chosen_graveyard.owner = Some(PlayerFilter::ChosenPlayer);
        assert_eq!(
            describe_value(&Value::Count(chosen_graveyard)),
            "the number of cards in the chosen player's graveyard"
        );
    }
}

#[cfg(test)]
mod whichever_is_greater_surface_tests {
    use super::*;

    #[test]
    fn arithmetic_maximum_keeps_the_authored_extremum_phrase() {
        let left = Value::Fixed(3);
        let right = Value::Fixed(5);
        let value = Value::Add(
            Box::new(Value::Add(Box::new(left.clone()), Box::new(right.clone()))),
            Box::new(Value::Scaled(
                Box::new(Value::Min(Box::new(left), Box::new(right))),
                -1,
            )),
        )
        .with_surface_hint(ValueSurfaceHint::WhicheverIsGreater);

        assert_eq!(describe_value(&value), "3 or 5, whichever is greater");
    }
}

#[cfg(test)]
mod death_history_controller_surface_tests {
    use super::*;

    fn death_count(controller_surface: ironsmith_core::DeathHistoryControllerSurface) -> Value {
        let mut filter = ObjectFilter::creature().controlled_by(PlayerFilter::You);
        filter.nontoken = true;
        Value::TurnHistoryCount(TurnHistoryCount::Died {
            filter,
            controller_surface,
        })
        .with_surface_hint(ValueSurfaceHint::ForEach)
    }

    #[test]
    fn death_history_preserves_both_controller_orders() {
        for (surface, expected_basis, expected_count) in [
            (
                ironsmith_core::DeathHistoryControllerSurface::DiedUnderControl,
                "nontoken creature that died under your control this turn",
                "the number of nontoken creatures that died under your control this turn",
            ),
            (
                ironsmith_core::DeathHistoryControllerSurface::ControlledThenDied,
                "nontoken creature you controlled that died this turn",
                "the number of nontoken creatures you controlled that died this turn",
            ),
        ] {
            let count = death_count(surface);
            assert_eq!(
                describe_turn_history_for_each_basis(&count).as_deref(),
                Some(expected_basis)
            );
            assert_eq!(describe_value(&count), expected_count);
        }
    }
}

#[cfg(test)]
mod shared_card_noun_union_tests {
    use super::*;

    #[test]
    fn search_selection_shares_card_noun_across_type_union() {
        assert_eq!(
            describe_search_selection_with_cards("Elf card or Elemental card"),
            "an Elf or Elemental card"
        );
        assert_eq!(
            describe_search_selection_with_cards("an artifact card or a creature card"),
            "an artifact or creature card"
        );
    }

    #[test]
    fn qualified_land_search_keeps_the_qualifier_after_card() {
        assert_eq!(
            describe_search_selection_with_cards("land card with a basic land type"),
            "a land card with a basic land type"
        );
    }

    #[test]
    fn counted_search_selection_pluralizes_the_card_noun_once() {
        assert_eq!(
            describe_search_selection_with_cards("up to X basic land card"),
            "up to X basic land cards"
        );
        assert_eq!(
            describe_search_selection_with_cards(
                "a number of basic land card less than or equal to the difference"
            ),
            "a number of basic land cards less than or equal to the difference"
        );
        assert_eq!(
            describe_search_selection_with_cards("any number of cards named Squee"),
            "any number of cards named Squee"
        );
    }
}

pub(crate) fn describe_mana_symbol(symbol: ManaSymbol) -> String {
    match symbol {
        ManaSymbol::White => "{W}".to_string(),
        ManaSymbol::Blue => "{U}".to_string(),
        ManaSymbol::Black => "{B}".to_string(),
        ManaSymbol::Red => "{R}".to_string(),
        ManaSymbol::Green => "{G}".to_string(),
        ManaSymbol::Colorless => "{C}".to_string(),
        ManaSymbol::Generic(v) => format!("{{{v}}}"),
        ManaSymbol::Snow => "{S}".to_string(),
        ManaSymbol::Life(_) => "{P}".to_string(),
        ManaSymbol::X => "{X}".to_string(),
    }
}

pub(crate) fn describe_mana_alternatives(symbols: &[ManaSymbol]) -> String {
    let rendered = symbols
        .iter()
        .copied()
        .map(describe_mana_symbol)
        .collect::<Vec<_>>();
    match rendered.len() {
        0 => "{0}".to_string(),
        1 => rendered[0].clone(),
        2 => format!("{} or {}", rendered[0], rendered[1]),
        _ => {
            let mut text = rendered[..rendered.len() - 1].join(", ");
            text.push_str(", or ");
            text.push_str(rendered.last().map(String::as_str).unwrap_or("{0}"));
            text
        }
    }
}

pub(crate) fn describe_counter_type(counter_type: CounterType) -> String {
    counter_type.description().into_owned()
}

pub(crate) fn describe_effect_metric_value(
    metric: crate::effect::EffectMetric,
    offset: Option<i32>,
) -> String {
    let base = match metric {
        crate::effect::EffectMetric::Count
        | crate::effect::EffectMetric::ChosenCount
        | crate::effect::EffectMetric::AffectedCount => "that many".to_string(),
        crate::effect::EffectMetric::LifeLost => "the life lost this way".to_string(),
        crate::effect::EffectMetric::LifeGained => "the life gained this way".to_string(),
        crate::effect::EffectMetric::DamageDealt => "the damage dealt this way".to_string(),
        crate::effect::EffectMetric::ExcessDamage => {
            "the excess damage dealt to that creature this way".to_string()
        }
        crate::effect::EffectMetric::DamagePrevented => "the damage prevented this way".to_string(),
        crate::effect::EffectMetric::FirstPower => "that creature's power".to_string(),
        crate::effect::EffectMetric::FirstToughness => "that creature's toughness".to_string(),
        crate::effect::EffectMetric::FirstManaValue => "that card's mana value".to_string(),
        crate::effect::EffectMetric::TotalPower => "the total power of those creatures".to_string(),
        crate::effect::EffectMetric::TotalToughness => {
            "the total toughness of those creatures".to_string()
        }
        crate::effect::EffectMetric::TotalManaValue => {
            "the total mana value of those cards".to_string()
        }
        crate::effect::EffectMetric::GreatestPower => {
            "the greatest power among those creatures".to_string()
        }
        crate::effect::EffectMetric::GreatestToughness => {
            "the greatest toughness among those creatures".to_string()
        }
        crate::effect::EffectMetric::GreatestManaValue => {
            "the greatest mana value among those cards".to_string()
        }
        crate::effect::EffectMetric::ColorsAmong => {
            "the number of colors among those cards".to_string()
        }
        crate::effect::EffectMetric::CardTypesAmong => {
            "the number of card types among those cards".to_string()
        }
        crate::effect::EffectMetric::GreatestPlayerCount => {
            "the greatest number of cards a player discarded this way".to_string()
        }
        crate::effect::EffectMetric::IteratedPlayerCount => "that many".to_string(),
        crate::effect::EffectMetric::PlayersWithPositiveCount => "that many players".to_string(),
        crate::effect::EffectMetric::OtherNumber => "the other result".to_string(),
    };
    match offset {
        Some(0) | None => base,
        Some(n) if n > 0 => format!("{base} plus {n}"),
        Some(n) => format!("{base} minus {}", -n),
    }
}

pub(crate) fn describe_prior_effect_action(
    action: crate::effect::PriorEffectAction,
) -> &'static str {
    match action {
        crate::effect::PriorEffectAction::Cast => "cast",
        crate::effect::PriorEffectAction::Chosen => "chosen",
        crate::effect::PriorEffectAction::Connived => "connived",
        crate::effect::PriorEffectAction::Countered => "countered",
        crate::effect::PriorEffectAction::CountersPut => "had counters put on them",
        crate::effect::PriorEffectAction::DealtDamage => "dealt damage",
        crate::effect::PriorEffectAction::Destroyed => "destroyed",
        crate::effect::PriorEffectAction::Discarded => "discarded",
        crate::effect::PriorEffectAction::Drawn => "drawn",
        crate::effect::PriorEffectAction::Exiled => "exiled",
        crate::effect::PriorEffectAction::Goaded => "goaded",
        crate::effect::PriorEffectAction::Milled => "milled",
        crate::effect::PriorEffectAction::PhasedOut => "phased out",
        crate::effect::PriorEffectAction::Prevented => "prevented",
        crate::effect::PriorEffectAction::PutOntoBattlefield => "put onto the battlefield",
        crate::effect::PriorEffectAction::Removed => "removed",
        crate::effect::PriorEffectAction::Returned => "returned",
        crate::effect::PriorEffectAction::Revealed => "revealed",
        crate::effect::PriorEffectAction::Sacrificed => "sacrificed",
        crate::effect::PriorEffectAction::Searched => "searched for",
        crate::effect::PriorEffectAction::Shuffled => "shuffled",
        crate::effect::PriorEffectAction::Tapped => "tapped",
    }
}

pub(crate) fn describe_prior_effect_action_clause(
    action: crate::effect::PriorEffectAction,
) -> String {
    let action_text = describe_prior_effect_action(action);
    if matches!(
        action,
        crate::effect::PriorEffectAction::CountersPut | crate::effect::PriorEffectAction::PhasedOut
    ) {
        format!("that {action_text}")
    } else {
        action_text.to_string()
    }
}

fn prior_effect_default_noun(
    query: &crate::effect::PriorEffectMetricQuery,
    plural: bool,
) -> &'static str {
    match query.action {
        Some(crate::effect::PriorEffectAction::Removed) => {
            if plural {
                "counters"
            } else {
                "counter"
            }
        }
        Some(
            crate::effect::PriorEffectAction::Cast
            | crate::effect::PriorEffectAction::Drawn
            | crate::effect::PriorEffectAction::Discarded
            | crate::effect::PriorEffectAction::Exiled
            | crate::effect::PriorEffectAction::Milled
            | crate::effect::PriorEffectAction::Returned
            | crate::effect::PriorEffectAction::Revealed
            | crate::effect::PriorEffectAction::Searched,
        ) => {
            if plural {
                "cards"
            } else {
                "card"
            }
        }
        _ => {
            if plural {
                "objects"
            } else {
                "object"
            }
        }
    }
}

fn prior_effect_filter_is_source_placeholder(filter: &crate::filter::ObjectFilter) -> bool {
    let mut base = filter.clone();
    base.set_prior_effect_action_surface(None);
    // Prior-effect metrics use captured LKI, so neither the producer's zone
    // nor the authored source pronoun changes what is being counted.
    base.zone = None;
    base.source_surface = None;
    base == crate::filter::ObjectFilter::source()
}

fn prior_effect_query_noun(query: &crate::effect::PriorEffectMetricQuery, plural: bool) -> String {
    if query.action == Some(crate::effect::PriorEffectAction::Removed)
        && let Some(counter_type) = query.counter_type
    {
        let counter_type = describe_counter_type(counter_type);
        return if plural {
            format!("{counter_type} counters")
        } else {
            format!("{counter_type} counter")
        };
    }
    let Some(filter) = query.filter.as_ref() else {
        return prior_effect_default_noun(query, plural).to_string();
    };
    let mut unqualified_filter = filter.clone();
    unqualified_filter.zone = None;
    unqualified_filter.set_prior_effect_action_surface(None);
    unqualified_filter.set_explicit_card_noun(false);
    unqualified_filter.set_explicit_card_type_noun(None);
    if unqualified_filter == crate::filter::ObjectFilter::default() {
        if filter.zone == Some(Zone::Battlefield) {
            return if plural {
                "permanents".to_string()
            } else {
                "permanent".to_string()
            };
        }
        return prior_effect_default_noun(query, plural).to_string();
    }
    // A source-only filter is a reference placeholder, not the noun being
    // counted. For actions such as removing counters, the typed action owns
    // that noun ("counter"), while the source identifies the producer target.
    if prior_effect_filter_is_source_placeholder(filter) {
        return prior_effect_default_noun(query, plural).to_string();
    }
    let mut filter = filter.clone();
    // The query is over captured LKI. The producer action already supplies
    // the relevant zone transition, so a parser-default battlefield zone
    // must not leak into "creatures destroyed this way" wording.
    filter.zone = None;
    if plural {
        describe_count_filter_value_subject(&filter)
    } else {
        describe_for_each_count_filter(&filter)
    }
}

pub(crate) fn describe_prior_effect_metric_basis(
    query: &crate::effect::PriorEffectMetricQuery,
    plural: bool,
) -> String {
    if query.action == Some(crate::effect::PriorEffectAction::Returned)
        && let Some(filter) = query.filter.as_ref()
        && filter.owner == Some(PlayerFilter::You)
    {
        // A permanent ceases to be a permanent when it leaves the
        // battlefield. Oracle text therefore refers to the result as a card
        // once it has been returned to hand.
        let noun = if plural { "cards" } else { "card" };
        return format!("{noun} returned to your hand this way");
    }
    let noun = prior_effect_query_noun(query, plural);
    match query.action {
        Some(action) => format!(
            "{noun} {} this way",
            describe_prior_effect_action_clause(action)
        ),
        None => noun,
    }
}

/// Render a typed prior-action count that still carries a source-only object
/// filter. This is the legacy `Value::Count` counterpart to
/// `PriorEffectMetricQuery`; both use the same typed action surface.
pub(crate) fn describe_prior_effect_source_count_basis(
    filter: &crate::filter::ObjectFilter,
    plural: bool,
) -> Option<String> {
    let action = filter.prior_effect_action_surface()?;
    if !prior_effect_filter_is_source_placeholder(filter) {
        return None;
    }
    let query = crate::effect::PriorEffectMetricQuery::new(
        crate::effect::EffectMetricSource::AffectedObjects,
        crate::effect::EffectMetric::Count,
    )
    .with_action(action);
    Some(describe_prior_effect_metric_basis(&query, plural))
}

/// Return the filtered object basis for a typed prior-action count.
///
/// Damage rendering uses this to preserve authored scalar surfaces such as
/// "damage equal to the number of creatures tapped this way" and
/// "1 damage ... for each creature tapped this way" without recovering the
/// action or object kind from rendered text.
pub(crate) fn describe_prior_effect_count_basis_for_action(
    value: &Value,
    action: crate::effect::PriorEffectAction,
    plural: bool,
) -> Option<String> {
    let query = match value.unhinted() {
        Value::PriorEffectMetric { query, .. } | Value::PendingPriorEffectMetric(query) => query,
        _ => return None,
    };
    if query.metric != crate::effect::EffectMetric::Count || query.action != Some(action) {
        return None;
    }
    Some(describe_prior_effect_metric_basis(query, plural))
}

pub(crate) fn describe_prior_effect_metric_value(
    query: &crate::effect::PriorEffectMetricQuery,
) -> String {
    let plural_basis = describe_prior_effect_metric_basis(query, true);
    let singular_basis = describe_prior_effect_metric_basis(query, false);
    match query.metric {
        crate::effect::EffectMetric::Count
        | crate::effect::EffectMetric::ChosenCount
        | crate::effect::EffectMetric::AffectedCount => {
            format!("the number of {plural_basis}")
        }
        crate::effect::EffectMetric::FirstPower => {
            format!("the power of the {singular_basis}")
        }
        crate::effect::EffectMetric::FirstToughness => {
            format!("the toughness of the {singular_basis}")
        }
        crate::effect::EffectMetric::FirstManaValue => {
            format!("the mana value of the {singular_basis}")
        }
        crate::effect::EffectMetric::TotalPower => {
            format!("the total power of {plural_basis}")
        }
        crate::effect::EffectMetric::TotalToughness => {
            format!("the total toughness of {plural_basis}")
        }
        crate::effect::EffectMetric::TotalManaValue => {
            format!("the total mana value of {plural_basis}")
        }
        crate::effect::EffectMetric::GreatestPower => {
            format!("the greatest power among {plural_basis}")
        }
        crate::effect::EffectMetric::GreatestToughness => {
            format!("the greatest toughness among {plural_basis}")
        }
        crate::effect::EffectMetric::GreatestManaValue => {
            format!("the greatest mana value among {plural_basis}")
        }
        crate::effect::EffectMetric::ColorsAmong => {
            format!("the number of colors among {plural_basis}")
        }
        crate::effect::EffectMetric::CardTypesAmong => {
            if query.action == Some(crate::effect::PriorEffectAction::Discarded)
                && query.filter.is_none()
                && query.player.is_none()
            {
                "the number of card types the discarded card has".to_string()
            } else {
                format!("the number of card types among {plural_basis}")
            }
        }
        metric => describe_effect_metric_value(metric, None),
    }
}

pub(crate) fn describe_explicit_where_x_surface(value: &Value) -> Option<&'static str> {
    if value.has_surface_hint(ValueSurfaceHint::CardsDrawnThisWay) {
        return Some("the number of cards drawn this way");
    }
    if value.has_surface_hint(ValueSurfaceHint::CardsRevealedThisWay) {
        return Some("the number of cards revealed this way");
    }
    if value.has_surface_hint(ValueSurfaceHint::CardsExiledThisWay) {
        return Some("the number of cards exiled this way");
    }
    if value.has_surface_hint(ValueSurfaceHint::CardsDiscardedThisWay) {
        return Some("the number of cards discarded this way");
    }
    if value.has_surface_hint(ValueSurfaceHint::PermanentsSacrificedThisWay) {
        return match value.unhinted() {
            Value::TotalPower(filter) if filter.card_types.contains(&CardType::Creature) => {
                Some("the total power of the creatures sacrificed this way")
            }
            Value::TotalPower(_) => Some("the total power of the permanents sacrificed this way"),
            Value::TotalToughness(filter) if filter.card_types.contains(&CardType::Creature) => {
                Some("the total toughness of the creatures sacrificed this way")
            }
            Value::TotalToughness(_) => {
                Some("the total toughness of the permanents sacrificed this way")
            }
            Value::TotalManaValue(filter) if filter.card_types.contains(&CardType::Creature) => {
                Some("the total mana value of the creatures sacrificed this way")
            }
            Value::TotalManaValue(_) => {
                Some("the total mana value of the permanents sacrificed this way")
            }
            _ => Some("the number of permanents sacrificed this way"),
        };
    }
    if value.has_surface_hint(ValueSurfaceHint::CountersRemovedThisWay) {
        return Some("the number of counters removed this way");
    }
    if value.has_surface_hint(ValueSurfaceHint::EnergyPaidThisWay) {
        return Some("the amount of {E} paid this way");
    }
    if value.has_surface_hint(ValueSurfaceHint::PriorEffectResult) {
        return Some("the result");
    }
    if value.has_surface_hint(ValueSurfaceHint::ManaValueOfPermanentExiledThisWay) {
        return Some("the mana value of the permanent exiled this way");
    }
    None
}

fn describe_history_event_object(filter: &ObjectFilter) -> String {
    let mut surface = filter.clone();
    surface.zone = None;
    strip_leading_article(&surface.description())
        .trim()
        .to_string()
}

pub(crate) fn describe_past_controller(controller: &PlayerFilter) -> String {
    format!("{} controlled", describe_player_filter(controller))
}

pub(crate) fn describe_death_history_subject(
    subject: &str,
    controller: Option<&PlayerFilter>,
    controller_surface: ironsmith_core::DeathHistoryControllerSurface,
) -> String {
    match (controller, controller_surface) {
        (Some(controller), ironsmith_core::DeathHistoryControllerSurface::ControlledThenDied) => {
            format!(
                "{subject} {} that died this turn",
                describe_past_controller(controller)
            )
        }
        (Some(controller), _) => format!(
            "{subject} that died under {} control this turn",
            describe_possessive_player_filter(controller)
        ),
        (None, _) => format!("{subject} that died this turn"),
    }
}

fn describe_history_spell(filter: &ObjectFilter) -> String {
    if filter.card_types.len() == 2
        && filter.card_types.contains(&CardType::Instant)
        && filter.card_types.contains(&CardType::Sorcery)
        && filter.all_card_types.is_empty()
    {
        return "instant and sorcery spell".to_string();
    }
    let mut surface = filter.clone();
    surface.zone = Some(Zone::Stack);
    surface.stack_kind = Some(StackObjectKind::Spell);
    surface.has_mana_cost = false;
    let described = strip_leading_article(&surface.description())
        .trim()
        .to_string();
    if described == "spell" || described.ends_with(" spell") {
        described
    } else {
        format!("{described} spell")
    }
}

/// Render the singular event basis selected by a typed `for each` history
/// count. This deliberately reads the structured query rather than trying to
/// singularize the ordinary "the number of ..." value surface.
pub(crate) fn describe_turn_history_for_each_basis(value: &Value) -> Option<String> {
    if !value.has_surface_hint(ValueSurfaceHint::ForEach)
        || value.has_surface_hint(ValueSurfaceHint::EqualTo)
        || value.has_surface_hint(ValueSurfaceHint::WhereXIs)
    {
        return None;
    }

    match value.unhinted() {
        Value::TurnHistoryCount(TurnHistoryCount::Died {
            filter,
            controller_surface,
        }) => {
            let mut subject_filter = filter.clone();
            let controller = subject_filter.controller.take();
            let subject = describe_history_event_object(&subject_filter);
            Some(describe_death_history_subject(
                &subject,
                controller.as_ref(),
                *controller_surface,
            ))
        }
        Value::TurnHistoryCount(TurnHistoryCount::CountersPutOn {
            counter_type,
            filter,
        }) => {
            let mut subject_filter = filter.clone();
            let controller = subject_filter.controller.take();
            subject_filter.zone = None;
            let subject = pluralize_noun_phrase(&describe_for_each_filter(&subject_filter));
            let controlled = controller.map_or_else(String::new, |controller| {
                format!(
                    " under {} control",
                    describe_possessive_player_filter(&controller)
                )
            });
            let counter = counter_type.map_or_else(
                || "counter".to_string(),
                |counter_type| format!("{} counter", counter_type.description()),
            );
            Some(format!(
                "{counter} you've put on {subject}{controlled} this turn"
            ))
        }
        Value::TurnHistoryCount(TurnHistoryCount::Sacrificed { player, filter }) => {
            let subject = describe_history_event_object(filter);
            Some(match player {
                PlayerFilter::You => format!("{subject} you've sacrificed this turn"),
                PlayerFilter::Any => format!("{subject} sacrificed this turn"),
                other => format!(
                    "{subject} {} sacrificed this turn",
                    describe_player_filter(other)
                ),
            })
        }
        Value::TurnHistoryCount(TurnHistoryCount::SpellsCast {
            player,
            filter,
            from_zone,
            from_outside_hand,
            exclude_source,
            before_triggering_spell,
        }) => {
            let mut subject = describe_history_spell(filter);
            if *exclude_source && !subject.starts_with("other ") {
                subject = format!("other {subject}");
            }
            if *before_triggering_spell {
                return Some(match player {
                    PlayerFilter::You => {
                        format!("{subject} you've cast before it this turn")
                    }
                    PlayerFilter::Any => {
                        format!("{subject} cast before that spell this turn")
                    }
                    other => format!(
                        "{subject} {} cast before that spell this turn",
                        describe_player_filter(other)
                    ),
                });
            }
            Some(match (player, from_zone, from_outside_hand) {
                (PlayerFilter::You, Some(zone), _) => {
                    format!("{subject} you've cast from {zone} this turn")
                }
                (_, Some(zone), _) => format!(
                    "{subject} {} cast from {zone} this turn",
                    describe_player_filter(player)
                ),
                (PlayerFilter::You, None, true) => {
                    format!("{subject} you've cast from anywhere other than your hand this turn")
                }
                (PlayerFilter::You, None, false) => {
                    format!("{subject} you've cast this turn")
                }
                (_, None, true) => format!(
                    "{subject} {} cast from anywhere other than their hand this turn",
                    describe_player_filter(player)
                ),
                // Oracle text leaves the actor implicit when any player counts
                // ("for each other spell cast this turn").
                (PlayerFilter::Any, None, false) => format!("{subject} cast this turn"),
                (_, None, false) => format!(
                    "{subject} {} cast this turn",
                    describe_player_filter(player)
                ),
            })
        }
        Value::PriorEffectMetric { query, .. } | Value::PendingPriorEffectMetric(query)
            if matches!(
                query.metric,
                crate::effect::EffectMetric::Count
                    | crate::effect::EffectMetric::ChosenCount
                    | crate::effect::EffectMetric::AffectedCount
            ) =>
        {
            Some(describe_prior_effect_metric_basis(query, false))
        }
        Value::AttractionsVisitedThisTurn(player) => Some(match player {
            PlayerFilter::You => "Attraction you've visited this turn".to_string(),
            PlayerFilter::Opponent => "Attraction an opponent has visited this turn".to_string(),
            PlayerFilter::Any => "Attraction a player has visited this turn".to_string(),
            other => {
                let subject = describe_player_filter(other);
                format!(
                    "Attraction {subject} {} visited this turn",
                    player_verb(&subject, "have", "has")
                )
            }
        }),
        _ => None,
    }
}

fn describe_turn_history_count(query: &TurnHistoryCount) -> String {
    match query {
        TurnHistoryCount::Died {
            filter,
            controller_surface,
        } => {
            let mut subject_filter = filter.clone();
            let controller = subject_filter.controller.take();
            subject_filter.zone = None;
            let subject = pluralize_noun_phrase(&describe_for_each_filter(&subject_filter));
            format!(
                "the number of {}",
                describe_death_history_subject(&subject, controller.as_ref(), *controller_surface,)
            )
        }
        TurnHistoryCount::EnteredBattlefield(filter) => {
            let mut subject_filter = filter.clone();
            let controller = subject_filter.controller.take();
            subject_filter.zone = None;
            let subject = pluralize_noun_phrase(&describe_for_each_filter(&subject_filter));
            match controller {
                Some(controller) => format!(
                    "the number of {subject} that entered the battlefield under {} control this turn",
                    describe_possessive_player_filter(&controller)
                ),
                None => format!("the number of {subject} that entered the battlefield this turn"),
            }
        }
        TurnHistoryCount::TokensCreated(player) => match player {
            PlayerFilter::You => "the number of tokens you created this turn".to_string(),
            _ => format!(
                "the number of tokens {} created this turn",
                describe_player_filter(player)
            ),
        },
        TurnHistoryCount::PutIntoGraveyard { owner, from } => {
            let origin = match from.as_slice() {
                [] => "from anywhere".to_string(),
                [Zone::Hand, Zone::Library] | [Zone::Library, Zone::Hand] => {
                    format!(
                        "from {} hand or library",
                        describe_possessive_player_filter(owner)
                    )
                }
                [zone] => format!("from {}", zone.name()),
                zones => format!(
                    "from {}",
                    join_with_or(
                        &zones
                            .iter()
                            .map(|zone| zone.name().to_string())
                            .collect::<Vec<_>>()
                    )
                ),
            };
            format!(
                "the number of cards put into {} graveyard {origin} this turn",
                describe_possessive_player_filter(owner)
            )
        }
        TurnHistoryCount::MovedZones { filter, from, to } => {
            let subject = pluralize_noun_phrase(&describe_for_each_filter(filter));
            match (from, to) {
                (Some(from), Some(to)) => {
                    format!("the number of {subject} put into {to} from {from} this turn")
                }
                (Some(from), None) => {
                    format!("the number of {subject} that left {from} this turn")
                }
                (None, Some(to)) => {
                    format!("the number of {subject} put into {to} this turn")
                }
                (None, None) => format!("the number of {subject} that changed zones this turn"),
            }
        }
        TurnHistoryCount::Sacrificed { player, filter } => format!(
            "the number of {} {} sacrificed this turn",
            pluralize_noun_phrase(&describe_for_each_filter(filter)),
            describe_player_filter(player)
        ),
        TurnHistoryCount::CountersPutOn {
            counter_type,
            filter,
        } => format!(
            "the number of {} counters put on {} this turn",
            counter_type.map_or("".to_string(), |counter_type| counter_type
                .description()
                .to_string()),
            pluralize_noun_phrase(&describe_for_each_filter(filter))
        ),
        TurnHistoryCount::CreaturesAttackedWith { player, filter } => format!(
            "the number of {} {} attacked with this turn",
            pluralize_noun_phrase(&describe_for_each_filter(filter)),
            describe_player_filter(player)
        ),
        TurnHistoryCount::OpponentsAttacked(player) => match player {
            PlayerFilter::You => "the number of opponents you attacked this turn".to_string(),
            _ => format!(
                "the number of opponents {} attacked this turn",
                describe_player_filter(player)
            ),
        },
        TurnHistoryCount::PlayersDiscarded(player) => match player {
            PlayerFilter::Any => "the number of players who discarded a card this turn".to_string(),
            _ => format!(
                "the number of {} who discarded a card this turn",
                pluralize_noun_phrase(&describe_player_filter(player))
            ),
        },
        TurnHistoryCount::PlayersDealtDamage(player) => format!(
            "the number of {} who were dealt damage this turn",
            pluralize_noun_phrase(&describe_player_filter(player))
        ),
        TurnHistoryCount::PlayersDealtCombatDamageBy { players, sources } => {
            let players = pluralize_noun_phrase(&describe_player_filter(players));
            if sources == &ObjectFilter::default() {
                format!("the number of {players} who were dealt combat damage this turn")
            } else {
                format!(
                    "the number of {players} who were dealt combat damage by {} this turn",
                    describe_for_each_filter(sources)
                )
            }
        }
        TurnHistoryCount::DiscardedOrCycled(player) => match player {
            PlayerFilter::You => {
                "the number of cards you've cycled or discarded this turn".to_string()
            }
            _ => format!(
                "the number of cards {} cycled or discarded this turn",
                describe_player_filter(player)
            ),
        },
        TurnHistoryCount::Cycled(player) => match player {
            PlayerFilter::You => "the number of cards you've cycled this turn".to_string(),
            _ => format!(
                "the number of cards {} cycled this turn",
                describe_player_filter(player)
            ),
        },
        TurnHistoryCount::PlayersLostLife(player) => format!(
            "the number of {} who lost life this turn",
            pluralize_noun_phrase(&describe_player_filter(player))
        ),
        TurnHistoryCount::UntappedLandsAtTurnStart(player) => match player {
            PlayerFilter::You => {
                "the number of untapped lands you controlled at the beginning of this turn"
                    .to_string()
            }
            PlayerFilter::IteratedPlayer => {
                "the number of untapped lands they controlled at the beginning of this turn"
                    .to_string()
            }
            other => format!(
                "the number of untapped lands {} controlled at the beginning of this turn",
                describe_player_filter(other)
            ),
        },
        TurnHistoryCount::Descended(player) => match player {
            PlayerFilter::You => "the number of times you descended this turn".to_string(),
            _ => format!(
                "the number of times {} descended this turn",
                describe_player_filter(player)
            ),
        },
        TurnHistoryCount::DamageDealtToSource => {
            "the amount of damage dealt to it this turn".to_string()
        }
        TurnHistoryCount::SpellsCast {
            player,
            filter,
            from_zone,
            from_outside_hand,
            exclude_source,
            before_triggering_spell,
        } => {
            let base = pluralize_noun_phrase(&describe_history_spell(filter));
            let mut out = if *before_triggering_spell {
                match player {
                    PlayerFilter::You => {
                        format!("the number of {base} you've cast before it this turn")
                    }
                    PlayerFilter::Any => {
                        format!("the number of {base} cast before that spell this turn")
                    }
                    other => format!(
                        "the number of {base} {} cast before that spell this turn",
                        describe_player_filter(other)
                    ),
                }
            } else {
                match (player, from_zone, from_outside_hand) {
                    (PlayerFilter::You, Some(zone), _) => {
                        format!("the number of {base} you've cast from {zone} this turn")
                    }
                    (_, Some(zone), _) => format!(
                        "the number of {base} {} cast from {zone} this turn",
                        describe_player_filter(player)
                    ),
                    (PlayerFilter::You, None, true) => format!(
                        "the number of {base} you've cast from anywhere other than your hand this turn"
                    ),
                    (PlayerFilter::You, None, false) => {
                        format!("the number of {base} you've cast this turn")
                    }
                    (_, None, true) => format!(
                        "the number of {base} {} cast from anywhere other than their hand this turn",
                        describe_player_filter(player)
                    ),
                    (_, None, false) => format!(
                        "the number of {base} {} cast this turn",
                        describe_player_filter(player)
                    ),
                }
            };
            if *exclude_source {
                out = out.replacen("the number of ", "the number of other ", 1);
            }
            out
        }
        TurnHistoryCount::ColorsAmongPermanentsAndSpellsCast(player) => match player {
            PlayerFilter::You => {
                "the number of colors among permanents you control and spells you've cast this turn"
                    .to_string()
            }
            _ => format!(
                "the number of colors among permanents {} controls and spells they cast this turn",
                describe_player_filter(player)
            ),
        },
    }
}

fn describe_absolute_difference(value: &Value) -> Option<String> {
    if !value.has_surface_hint(ironsmith_core::ValueSurfaceHint::Difference) {
        return None;
    }
    let Value::Scaled(minimum, -1) = value.unhinted() else {
        return None;
    };
    let Value::Min(forward, reverse) = minimum.as_ref() else {
        return None;
    };
    let Value::Add(left, negative_right) = forward.as_ref() else {
        return None;
    };
    let Value::Scaled(right, -1) = negative_right.as_ref() else {
        return None;
    };
    let Value::Add(reverse_right, negative_left) = reverse.as_ref() else {
        return None;
    };
    let Value::Scaled(reverse_left, -1) = negative_left.as_ref() else {
        return None;
    };
    if left.as_ref() != reverse_left.as_ref() || right.as_ref() != reverse_right.as_ref() {
        return None;
    }
    if let (Value::GreatestPower(left_filter), Value::LeastPower(right_filter)) =
        (left.as_ref(), right.as_ref())
        && left_filter == right_filter
        && let Some(chosen_set) = describe_chosen_object_set_filter(left_filter)
    {
        return Some(format!("the difference between {chosen_set}' powers"));
    }
    Some(format!(
        "the difference between {} and {}",
        describe_value(left),
        describe_value(right)
    ))
}

pub(crate) fn describe_value(value: &Value) -> String {
    fn describe_static_ability_id(
        ability_id: crate::static_abilities::StaticAbilityId,
    ) -> &'static str {
        match ability_id {
            crate::static_abilities::StaticAbilityId::Flying => "flying",
            crate::static_abilities::StaticAbilityId::FirstStrike => "first strike",
            crate::static_abilities::StaticAbilityId::DoubleStrike => "double strike",
            crate::static_abilities::StaticAbilityId::Deathtouch => "deathtouch",
            crate::static_abilities::StaticAbilityId::Haste => "haste",
            crate::static_abilities::StaticAbilityId::Hexproof => "hexproof",
            crate::static_abilities::StaticAbilityId::Indestructible => "indestructible",
            crate::static_abilities::StaticAbilityId::Lifelink => "lifelink",
            crate::static_abilities::StaticAbilityId::Menace => "menace",
            crate::static_abilities::StaticAbilityId::Reach => "reach",
            crate::static_abilities::StaticAbilityId::Trample => "trample",
            crate::static_abilities::StaticAbilityId::Vigilance => "vigilance",
            _ => "ability",
        }
    }

    match value {
        Value::SurfaceHinted { hints, .. }
            if hints.contains(&ironsmith_core::ValueSurfaceHint::Difference) =>
        {
            describe_absolute_difference(value).unwrap_or_else(|| "the difference".to_string())
        }
        Value::SurfaceHinted { value, hints } => {
            if hints.contains(&ironsmith_core::ValueSurfaceHint::WhicheverIsGreater)
                && let Value::Add(total, negative_minimum) = value.unhinted()
                && let Value::Add(left, right) = total.as_ref()
                && let Value::Scaled(minimum, -1) = negative_minimum.as_ref()
                && let Value::Min(minimum_left, minimum_right) = minimum.as_ref()
                && left.as_ref() == minimum_left.as_ref()
                && right.as_ref() == minimum_right.as_ref()
            {
                return format!(
                    "{} or {}, whichever is greater",
                    describe_value(left),
                    describe_value(right)
                );
            }
            if hints.contains(&ironsmith_core::ValueSurfaceHint::InExcessOf)
                && let Value::Add(left, right) = value.unhinted()
            {
                if let Value::Scaled(right, -1) = right.as_ref() {
                    return format!(
                        "{} in excess of {}",
                        describe_value(left),
                        describe_value(right)
                    );
                }
                if let Value::Fixed(right) = right.as_ref()
                    && *right < 0
                {
                    return format!(
                        "{} in excess of {}",
                        describe_value(left),
                        right.abs()
                    );
                }
            }
            if hints.contains(&ironsmith_core::ValueSurfaceHint::MasculineSourcePossessive)
                && matches!(value.unhinted(), Value::SourcePower)
            {
                return "his power".to_string();
            }
            if hints.contains(&ironsmith_core::ValueSurfaceHint::FeminineSourcePossessive)
                && matches!(value.unhinted(), Value::SourcePower)
            {
                return "her power".to_string();
            }
            if hints.contains(&ironsmith_core::ValueSurfaceHint::ThatPlayerPossessive) {
                return describe_value(value).replace("their hand", "that player's hand");
            }
            if hints.contains(&ironsmith_core::ValueSurfaceHint::PriorEffectResult) {
                return "the result".to_string();
            }
            if hints.contains(&ironsmith_core::ValueSurfaceHint::DiedThisWay)
                && let Value::PriorEffectMetric { query, .. }
                    | Value::PendingPriorEffectMetric(query) = value.unhinted()
                && query.metric == crate::effect::EffectMetric::Count
            {
                return format!(
                    "the number of {} that died this way",
                    prior_effect_query_noun(query, true)
                );
            }
            if hints.contains(
                &ironsmith_core::ValueSurfaceHint::CardsLookedAtWhileScryingThisWay,
            ) && matches!(value.unhinted(), Value::EventValue(EventValueSpec::Amount))
            {
                return "the number of cards looked at while scrying this way".to_string();
            }
            if hints.contains(&ironsmith_core::ValueSurfaceHint::DamageDealt)
                && matches!(value.unhinted(), Value::EventValue(EventValueSpec::Amount))
            {
                return "the damage dealt".to_string();
            }
            if hints.contains(
                &ironsmith_core::ValueSurfaceHint::OpponentsDealtDamageThisWay,
            ) && matches!(value.unhinted(), Value::EventValue(EventValueSpec::Amount))
            {
                return "the number of opponents dealt damage this way".to_string();
            }
            if hints.contains(&ironsmith_core::ValueSurfaceHint::RevealedCardReference)
                && matches!(value.unhinted(), Value::ManaValueOf(_))
            {
                return "the revealed card's mana value".to_string();
            }
            if hints.contains(
                &ironsmith_core::ValueSurfaceHint::TriggeringObjectCountersItHad,
            ) && matches!(
                value.unhinted(),
                Value::CountersOn(spec, None)
                    if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == "triggering")
            ) {
                return "the number of counters it had on it".to_string();
            }
            if let Some((card_type, action)) = hints.iter().find_map(|hint| match hint {
                ironsmith_core::ValueSurfaceHint::CharacteristicOfObjectThisWay {
                    card_type,
                    action,
                } => Some((*card_type, *action)),
                _ => None,
            }) {
                let characteristic = match value.unhinted() {
                    Value::PowerOf(_) => Some("power"),
                    Value::ToughnessOf(_) => Some("toughness"),
                    Value::ManaValueOf(_) => Some("mana value"),
                    _ => None,
                };
                if let Some(characteristic) = characteristic {
                    return format!(
                        "the {characteristic} of the {} {} this way",
                        card_type.name(),
                        describe_prior_effect_action_clause(action)
                    );
                }
            }
            if hints.contains(&ironsmith_core::ValueSurfaceHint::CountersAmong)
                && let Value::CountersOn(spec, counter_type) = value.unhinted()
                && let ChooseSpec::All(filter) = spec.unhinted()
            {
                let subject = render_effects::pluralize_noun_phrase(&filter.description());
                return match counter_type {
                    Some(counter_type) => format!(
                        "the number of {} counters among {subject}",
                        counter_type.description()
                    ),
                    None => format!("the number of counters among {subject}"),
                };
            }
            if let Some(kind) = hints.iter().find_map(|hint| match hint {
                ironsmith_core::ValueSurfaceHint::SacrificedObject(kind) => Some(*kind),
                _ => None,
            }) {
                let characteristic = match value.unhinted() {
                    Value::PowerOf(_) => Some("power"),
                    Value::ToughnessOf(_) => Some("toughness"),
                    Value::ManaValueOf(_) => Some("mana value"),
                    _ => None,
                };
                if let Some(characteristic) = characteristic {
                    return format!(
                        "the sacrificed {}'s {characteristic}",
                        kind.noun()
                    );
                }
                if let Value::ManaSymbolsInManaCostOf { color, .. } = value.unhinted() {
                    return format!(
                        "the number of {} mana symbols in the sacrificed {}'s mana cost",
                        color.name(),
                        kind.noun()
                    );
                }
            }
            describe_value(value)
        }
        Value::Fixed(n) => n.to_string(),
        Value::Add(left, right) => {
            if left == right {
                return format!("twice {}", describe_value(left));
            }
            if matches!(right.as_ref(), Value::XTimes(-1)) {
                return format!("{} minus X", describe_value(left));
            }
            if let Value::Scaled(value, -1) = right.as_ref() {
                return format!(
                    "{} minus {}",
                    describe_value(left),
                    describe_value(value)
                );
            }
            if let Value::Fixed(n) = right.as_ref()
                && *n < 0
            {
                format!("{} minus {}", describe_value(left), n.abs())
            } else {
                format!("{} plus {}", describe_value(left), describe_value(right))
            }
        }
        Value::Scaled(value, factor) => {
            if *factor == 1 {
                describe_value(value)
            } else if *factor == -1 {
                format!("-{}", describe_value(value))
            } else if *factor == 2 && value.has_surface_hint(ValueSurfaceHint::WhereXIs) {
                "twice X".to_string()
            } else if *factor == 2 {
                format!("twice {}", describe_value(value))
            } else {
                format!("{factor} times {}", describe_value(value))
            }
        }
        Value::DividedRoundedDown(value, divisor) => {
            format!("{} divided by {divisor}, rounded down", describe_value(value))
        }
        Value::Min(left, right) => {
            format!("the lesser of {} and {}", describe_value(left), describe_value(right))
        }
        Value::HalfRoundedDown(value) => {
            if let Value::Add(left, right) = value.as_ref() {
                let count_filter = match (left.as_ref(), right.as_ref()) {
                    (Value::Count(filter), Value::Fixed(1)) | (Value::Fixed(1), Value::Count(filter)) => {
                        Some(filter)
                    }
                    _ => None,
                };
                if let Some(filter) = count_filter {
                    return format!(
                        "half the number of {}, rounded up",
                        describe_count_filter_value_subject(filter)
                    );
                }
                let library_filter = match (left.as_ref(), right.as_ref()) {
                    (Value::CardsInLibrary(filter), Value::Fixed(1))
                    | (Value::Fixed(1), Value::CardsInLibrary(filter)) => Some(filter),
                    _ => None,
                };
                if let Some(filter) = library_filter {
                    return format!(
                        "half the number of cards in {} library, rounded up",
                        describe_possessive_player_filter(filter)
                    );
                }
                let rounded_up_basis = match (left.as_ref(), right.as_ref()) {
                    (basis, Value::Fixed(1)) | (Value::Fixed(1), basis) => Some(basis),
                    _ => None,
                };
                if let Some(basis) = rounded_up_basis {
                    return format!("half {}, rounded up", describe_value(basis));
                }
            }
            format!("half {}, rounded down", describe_value(value))
        }
        Value::X => "X".to_string(),
        Value::XTimes(factor) => {
            if *factor == 1 {
                "X".to_string()
            } else if *factor == 2 {
                "twice X".to_string()
            } else if *factor == -1 {
                "-X".to_string()
            } else {
                format!("{factor}*X")
            }
        }
        Value::VoteCount(option) => format!("the number of {} votes", option.to_ascii_lowercase()),
        Value::PlayerVoteCount(filter) => {
            format!("the number of votes {} received", filter.description())
        }
        Value::Count(filter) => {
            if filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation
                    == crate::filter::TaggedOpbjectRelation::IsTaggedObjectSacrificedAsSourceEntered
            }) {
                let mut sacrificed = filter.clone();
                sacrificed.zone = None;
                sacrificed.tagged_constraints.retain(|constraint| {
                    constraint.relation
                        != crate::filter::TaggedOpbjectRelation::IsTaggedObjectSacrificedAsSourceEntered
                });
                return format!(
                    "the number of {} sacrificed as it entered",
                    describe_count_filter_value_subject(&sacrificed)
                );
            }
            format!(
                "the number of {}",
                describe_count_filter_value_subject(filter)
            )
        }
        Value::CountScaled(filter, multiplier) => {
            let subject = describe_count_filter_value_subject(filter);
            if *multiplier == 2 {
                format!("twice the number of {subject}")
            } else {
                format!("{multiplier} times the number of {subject}")
            }
        }
        Value::GreatestCount(filter) => {
            format!(
                "the greatest number of {}",
                describe_count_filter_value_subject(filter)
            )
        }
        Value::GreatestSharedCreatureTypeCount(filter) => {
            format!(
                "the greatest number of {} that have a creature type in common",
                describe_count_filter_value_subject(filter)
            )
        }
        Value::TotalPower(filter) => {
            if filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                    && constraint.tag.as_str() == ironsmith_core::ATTACKING_GROUP_TAG
            }) {
                return "their total power".to_string();
            }
            let zone_change_group = ObjectFilter {
                card_types: vec![CardType::Creature],
                ..Default::default()
            }
            .match_tagged(
                ironsmith_core::ZONE_CHANGE_GROUP_TAG,
                TaggedOpbjectRelation::IsTaggedObject,
            );
            if filter == &zone_change_group {
                return "the total power of those creatures".to_string();
            }
            format!(
                "the total power of {}",
                describe_aggregate_filter_value_subject(filter)
            )
        }
        Value::TotalToughness(filter) => {
            format!(
                "the total toughness of {}",
                describe_aggregate_filter_value_subject(filter)
            )
        }
        Value::TotalManaValue(filter) => {
            format!(
                "the total mana value of {}",
                describe_aggregate_filter_value_subject(filter)
            )
        }
        Value::GreatestPower(filter) => {
            format!(
                "the greatest power among {}",
                describe_count_filter_value_subject(filter)
            )
        }
        Value::GreatestToughness(filter) => {
            format!(
                "the greatest toughness among {}",
                describe_count_filter_value_subject(filter)
            )
        }
        Value::GreatestManaValue(filter) => {
            let subject = describe_spell_cast_history_filter_subject(filter)
                .unwrap_or_else(|| describe_count_filter_value_subject(filter));
            format!(
                "the greatest mana value among {}",
                subject
            )
        }
        Value::LeastPower(filter) => {
            format!(
                "the least power among {}",
                describe_count_filter_value_subject(filter)
            )
        }
        Value::LeastToughness(filter) => {
            format!(
                "the lowest toughness among {}",
                describe_count_filter_value_subject(filter)
            )
        }
        Value::LeastManaValue(filter) => {
            format!(
                "the lowest mana value among {}",
                describe_count_filter_value_subject(filter)
            )
        }
        Value::BasicLandTypesAmong(filter) => {
            format!("the number of {}", describe_basic_land_types_among(filter))
        }
        Value::CreatureTypesAmong(filter) => format!(
            "the number of creature types among {}",
            describe_count_filter_value_subject(filter)
        ),
        Value::CardTypesAmong(filter) => format!(
            "the number of card types among {}",
            describe_count_filter_value_subject(filter)
        ),
        Value::StaticAbilitiesAmong { filter, abilities } => {
            let ability_list = abilities
                .iter()
                .map(|ability_id| describe_static_ability_id(*ability_id))
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "the number of abilities from among {ability_list} found among {}",
                describe_count_filter_value_subject(filter)
            )
        }
        Value::ColorsAmong(filter) => {
            format!("the number of {}", describe_colors_among(filter))
        }
        Value::ColorPairsAmong(filter) => format!(
            "the number of different color pairs among {}",
            describe_count_filter_value_subject(filter)
        ),
        Value::DistinctCounterTypesAmong(filter) => format!(
            "the number of kinds of counters among {}",
            describe_count_filter_value_subject(filter)
        ),
        Value::DistinctNames(filter) => format!(
            "the number of differently named {}",
            describe_count_filter_value_subject(filter)
        ),
        Value::DistinctPowers(filter) => format!(
            "the number of different powers among {}",
            describe_count_filter_value_subject(filter)
        ),
        Value::TurnHistoryCount(query) => describe_turn_history_count(query),
        Value::CreaturesDiedThisTurn => "the number of creatures that died this turn".to_string(),
        Value::CreaturesDiedThisTurnControlledBy(filter) => format!(
            "the number of creatures that died under {} control this turn",
            describe_possessive_player_filter(filter)
        ),
        Value::PlayersBeingAttacked => "the number of players being attacked".to_string(),
        Value::CountPlayers(filter) => match filter {
            PlayerFilter::Any => "the number of players".to_string(),
            PlayerFilter::Opponent => "the number of opponents".to_string(),
            PlayerFilter::NotYou => "the number of players other than you".to_string(),
            PlayerFilter::You => "the number of you".to_string(),
            _ => format!("the number of {}", describe_player_filter(filter)),
        },
        Value::CountPlayersWithCardsInHandAtLeast(filter, minimum) => {
            let players = match filter {
                PlayerFilter::Opponent => "your opponents".to_string(),
                PlayerFilter::Any => "players".to_string(),
                PlayerFilter::NotYou => "players other than you".to_string(),
                PlayerFilter::You => "you".to_string(),
                other => describe_player_set_filter(other),
            };
            let minimum = ironsmith_core::cardinal_word(*minimum)
                .unwrap_or_else(|| minimum.to_string());
            format!(
                "the number of {players} with {minimum} or more cards in hand"
            )
        }
        Value::PlayersWhoControlMoreThanYou { players, filter } => {
            let mut controlled_filter = filter.clone();
            if controlled_filter.zone == Some(Zone::Battlefield) {
                controlled_filter.zone = None;
            }
            let players = match players {
                PlayerFilter::Any => "players".to_string(),
                PlayerFilter::Opponent => "opponents".to_string(),
                other => describe_player_set_filter(other),
            };
            format!(
                "the number of {players} who control more {} than you",
                describe_count_filter_value_subject(&controlled_filter)
            )
        }
        Value::PlayersWhoControlAtLeastMoreThanYou {
            players,
            filter,
            minimum_difference,
        } => {
            let mut controlled_filter = filter.clone();
            if controlled_filter.zone == Some(Zone::Battlefield) {
                controlled_filter.zone = None;
            }
            let players = match players {
                PlayerFilter::Any => "players".to_string(),
                PlayerFilter::Opponent => "opponents".to_string(),
                other => describe_player_set_filter(other),
            };
            format!(
                "the number of {players} who control at least {} more {} than you",
                number_word(*minimum_difference as i32)
                    .unwrap_or_else(|| minimum_difference.to_string()),
                describe_count_filter_value_subject(&controlled_filter)
            )
        }
        Value::PartySize(filter) => {
            format!(
                "the number of creatures in {} party",
                describe_possessive_player_filter(filter)
            )
        }
        Value::SourcePower => "this source's power".to_string(),
        Value::SourceToughness => "this source's toughness".to_string(),
        Value::PowerOf(spec) => {
            if let Some(kind) = spec.sacrificed_object_kind() {
                format!("the sacrificed {}'s power", kind.noun())
            } else if let ChooseSpec::Tagged(tag) = spec.base()
                && tag.as_str() == crate::tag::SOURCE_EXILED_TAG
            {
                "the exiled card's power".to_string()
            } else {
                format!("{} power", describe_possessive_choose_spec(spec))
            }
        }
        Value::ToughnessOf(spec) => {
            if let Some(kind) = spec.sacrificed_object_kind() {
                format!("the sacrificed {}'s toughness", kind.noun())
            } else if let ChooseSpec::Tagged(tag) = spec.base()
                && tag.as_str() == crate::tag::SOURCE_EXILED_TAG
            {
                "the exiled card's toughness".to_string()
            } else {
                format!("{} toughness", describe_possessive_choose_spec(spec))
            }
        }
        Value::ManaValueOf(spec) => {
            // For implicit off-battlefield references, oracle text usually prefers
            // "that card's mana value" over "its mana value".
            if let Some(kind) = spec.sacrificed_object_kind() {
                format!("the sacrificed {}'s mana value", kind.noun())
            } else if spec.source_reference_surface().is_some() {
                format!("{} mana value", describe_possessive_choose_spec(spec))
            } else if let ChooseSpec::Tagged(tag) = spec.base()
                && tag.as_str() == "triggering"
            {
                "that spell's mana value".to_string()
            } else if let ChooseSpec::Tagged(tag) = spec.base()
                && (tag.as_str() == "discarded_cost"
                    || tag.as_str().starts_with("discard_cost_"))
            {
                "the discarded card's mana value".to_string()
            } else if let ChooseSpec::Tagged(tag) = spec.base()
                && (tag.as_str().starts_with("revealed_")
                    || tag.as_str() == crate::effects::PUBLIC_REVEALED_TAG
                    || tag.as_str().starts_with("searched_")
                    || tag.as_str().starts_with("milled_")
                    || tag.as_str().starts_with("discarded_")
                    || tag.as_str().starts_with("exiled_")
                    || tag.as_str().starts_with("__sentence_helper_exiled")
                    || tag.as_str().starts_with("__sentence_helper_revealed")
                    || tag.as_str().starts_with("__sentence_helper_consult_match"))
            {
                "that card's mana value".to_string()
            } else if let ChooseSpec::Tagged(tag) = spec.base()
                && tag.as_str().starts_with("exile_cost_")
            {
                "the exiled card's mana value".to_string()
            } else if let ChooseSpec::Tagged(tag) = spec.base()
                && tag.as_str() == crate::tag::PRIOR_EXILED_CARD_TAG
            {
                "the exiled card's mana value".to_string()
            } else {
                format!("{} mana value", describe_possessive_choose_spec(spec))
            }
        }
        Value::ManaSymbolsInManaCostOf { spec, color } => match spec.unhinted() {
            ChooseSpec::All(filter) => format!(
                "the number of {} mana symbols in the mana costs of {}",
                color.name(),
                describe_count_filter_value_subject(filter)
            ),
            _ => format!(
                "the number of {} mana symbols in {} mana cost",
                color.name(),
                describe_possessive_choose_spec(spec)
            ),
        },
        Value::NameStickerCharacterCountOnSource { character, surface } => {
            let source = surface
                .as_ref()
                .map(ironsmith_core::SourceReferenceSurface::display_text)
                .unwrap_or_else(|| "this permanent".to_string());
            format!("the number of {character}'s in name stickers on {source}")
        }
        Value::LifeTotal(filter) => {
            format!("{} life total", describe_possessive_player_filter(filter))
        }
        Value::LifeTotalAsTurnBegan(filter) => format!(
            "{} life total as the turn began",
            describe_possessive_player_filter(filter)
        ),
        Value::LifeTotalDifference(filter) => match filter {
            PlayerFilter::Target(_) | PlayerFilter::AliasedTarget(_) => {
                "the difference between those players' life totals".to_string()
            }
            _ => format!(
                "the difference between {} life totals",
                describe_player_filter(filter)
            ),
        },
        Value::Speed(filter) => {
            format!("{} speed", describe_possessive_player_filter(filter))
        }
        Value::StartingLifeTotal(filter) => {
            format!(
                "{} starting life total",
                describe_possessive_player_filter(filter)
            )
        }
        Value::HalfLifeTotalRoundedUp(filter) => format!(
            "half {} life total, rounded up",
            describe_possessive_player_filter(filter)
        ),
        Value::HalfLifeTotalRoundedDown(filter) => format!(
            "half {} life total, rounded down",
            describe_possessive_player_filter(filter)
        ),
        Value::HalfStartingLifeTotalRoundedUp(filter) => format!(
            "half {} starting life total, rounded up",
            describe_possessive_player_filter(filter)
        ),
        Value::HalfStartingLifeTotalRoundedDown(filter) => format!(
            "half {} starting life total, rounded down",
            describe_possessive_player_filter(filter)
        ),
        Value::CardsInHand(filter) => format!(
            "the number of cards in {} hand",
            describe_possessive_player_filter(filter)
        ),
        Value::DevotionToChosenColor(filter) => format!(
            "{} devotion to the chosen color",
            describe_possessive_player_filter(filter)
        ),
        Value::LifeGainedThisTurn(filter) => match filter {
            PlayerFilter::You => "the amount of life you gained this turn".to_string(),
            PlayerFilter::Opponent => {
                "the amount of life your opponents gained this turn".to_string()
            }
            _ => format!(
                "the amount of life {} gained this turn",
                describe_player_filter(filter)
            ),
        },
        Value::LifeLostThisTurn(filter) => match filter {
            PlayerFilter::You => "the amount of life you lost this turn".to_string(),
            PlayerFilter::Opponent => {
                "the total life your opponents lost this turn".to_string()
            }
            PlayerFilter::Any => "the total life lost by all players this turn".to_string(),
            _ => format!(
                "the total life {} lost this turn",
                describe_player_filter(filter)
            ),
        },
        Value::CardsDiscardedThisTurn(filter) => match filter {
            PlayerFilter::You => "the number of cards you've discarded this turn".to_string(),
            PlayerFilter::Opponent => {
                "the number of cards your opponents have discarded this turn".to_string()
            }
            PlayerFilter::Any => "the number of cards discarded this turn".to_string(),
            _ => format!(
                "the number of cards {} discarded this turn",
                describe_player_filter(filter)
            ),
        },
        Value::AttractionsVisitedThisTurn(filter) => match filter {
            PlayerFilter::You => {
                "the number of Attractions you've visited this turn".to_string()
            }
            PlayerFilter::Opponent => {
                "the number of Attractions your opponents have visited this turn".to_string()
            }
            PlayerFilter::Any => {
                "the number of Attractions players have visited this turn".to_string()
            }
            _ => format!(
                "the number of Attractions {} visited this turn",
                describe_player_filter(filter)
            ),
        },
        Value::DamageDealtToPlayersThisTurn(filter) => match filter {
            PlayerFilter::You => "the damage already dealt to you this turn".to_string(),
            PlayerFilter::Opponent => {
                "the damage already dealt to your opponents this turn".to_string()
            }
            PlayerFilter::Target(_) | PlayerFilter::AliasedTarget(_) => {
                "the damage already dealt to that player this turn".to_string()
            }
            _ => format!(
                "the damage already dealt to {} this turn",
                describe_player_filter(filter)
            ),
        },
        Value::NoncombatDamageDealtToPlayersThisTurn(filter) => match filter {
            PlayerFilter::You => {
                "the total amount of noncombat damage dealt to you this turn".to_string()
            }
            PlayerFilter::Opponent => {
                "the total amount of noncombat damage dealt to your opponents this turn".to_string()
            }
            _ => format!(
                "the total amount of noncombat damage dealt to {} this turn",
                describe_player_filter(filter)
            ),
        },
        Value::NoncombatDamageDealtBySourcesControlledThisTurn { player, colors } => {
            let source = match (player, colors) {
                (PlayerFilter::You, Some(colors))
                    if colors.contains(crate::color::Color::Red) && colors.count() == 1 =>
                {
                    "red sources you controlled"
                }
                (PlayerFilter::You, _) => "sources you controlled",
                (PlayerFilter::Opponent, _) => "sources your opponents controlled",
                _ => "matching sources",
            };
            format!("the total amount of noncombat damage {source} dealt this turn")
        }
        Value::MaxCardsDrawnThisTurn(filter) => match filter {
            PlayerFilter::You => "the number of cards you've drawn this turn".to_string(),
            PlayerFilter::IteratedPlayer => {
                "the number of cards that player has drawn this turn".to_string()
            }
            PlayerFilter::Opponent => {
                "the greatest number of cards an opponent has drawn this turn".to_string()
            }
            PlayerFilter::Any => "the greatest number of cards a player has drawn this turn".to_string(),
            _ => format!(
                "the greatest number of cards {} has drawn this turn",
                describe_player_filter(filter)
            ),
        },
        Value::MaxDiceRolledThisTurn(filter) => match filter {
            PlayerFilter::You => "the number of dice you've rolled this turn".to_string(),
            PlayerFilter::Opponent => {
                "the greatest number of dice an opponent has rolled this turn".to_string()
            }
            PlayerFilter::Any => "the greatest number of dice a player has rolled this turn".to_string(),
            _ => format!(
                "the greatest number of dice {} has rolled this turn",
                describe_player_filter(filter)
            ),
        },
        Value::LandsEnteredBattlefieldThisTurn(filter) => match filter {
            PlayerFilter::You => {
                "the number of lands that entered the battlefield under your control this turn"
                    .to_string()
            }
            PlayerFilter::Opponent => {
                "the number of lands that entered the battlefield under opponents' control this turn"
                    .to_string()
            }
            PlayerFilter::Any => {
                "the number of lands that entered the battlefield under players' control this turn"
                    .to_string()
            }
            _ => format!(
                "the number of lands that entered the battlefield under {}'s control this turn",
                describe_player_filter(filter)
            ),
        },
        Value::MaxCardsInHand(filter) => {
            // Prefer the oracle-style phrasing used on Adamaro, First to Desire.
            // (We keep this structured so that other filters still render coherently.)
            match filter {
                PlayerFilter::You => "the number of cards in your hand".to_string(),
                PlayerFilter::Opponent => "the number of cards in the hand of the opponent with the most cards in hand".to_string(),
                PlayerFilter::Any => "the number of cards in the hand of the player with the most cards in hand".to_string(),
                PlayerFilter::NotYou => "the number of cards in the hand of the player other than you with the most cards in hand".to_string(),
                _ => format!(
                    "the number of cards in the hand of the {} with the most cards in hand",
                    strip_leading_article(&describe_player_filter(filter))
                ),
            }
        }
        Value::CardsInGraveyard(filter) => format!(
            "the number of cards in {} graveyard",
            describe_possessive_player_filter(filter)
        ),
        Value::CardsInLibrary(filter) => format!(
            "the number of cards in {} library",
            describe_possessive_player_filter(filter)
        ),
        Value::SpellsCastThisTurn(filter) => {
            format!(
                "the number of spells cast this turn by {}",
                describe_player_filter(filter)
            )
        }
        Value::SpellsCastBeforeThisTurn(filter) => format!(
            "the number of spells cast before this spell this turn by {}",
            describe_player_filter(filter)
        ),
        Value::CommanderCastCount(filter) => match filter {
            PlayerFilter::You => {
                "the number of times you've cast your commander from the command zone this game"
                    .to_string()
            }
            PlayerFilter::Opponent => {
                "the number of times an opponent has cast their commander from the command zone this game"
                    .to_string()
            }
            PlayerFilter::Any => {
                "the number of times a player has cast their commander from the command zone this game"
                    .to_string()
            }
            _ => format!(
                "the number of times {} has cast their commander from the command zone this game",
                describe_player_filter(filter)
            ),
        },
        Value::ThisAbilityResolvedThisTurnCount => {
            "the number of times this ability has resolved this turn".to_string()
        }
        Value::SourceRegeneratedThisTurnCount => {
            "the number of times this permanent regenerated this turn".to_string()
        }
        Value::SourceMutationCount => {
            "the number of times this creature has mutated".to_string()
        }
        Value::SpellsCastThisTurnMatching {
            player,
            filter,
            exclude_source,
        } => {
            let base = pluralize_noun_phrase(&describe_for_each_filter(filter));
            let mut out = format!(
                "the number of {base} cast this turn by {}",
                describe_player_filter(player)
            );
            if *exclude_source {
                out.push_str(" other than this spell");
            }
            out
        }
        Value::TotalManaValueOfSpellsCastThisTurnMatching {
            player,
            filter,
            exclude_source,
        } => {
            let base = pluralize_noun_phrase(&describe_for_each_filter(filter));
            let subject = if *exclude_source {
                format!("other {base}")
            } else {
                base
            };
            let cast_surface = match player {
                PlayerFilter::You => "you've cast this turn".to_string(),
                PlayerFilter::Opponent => "your opponents have cast this turn".to_string(),
                PlayerFilter::IteratedPlayer => "they've cast this turn".to_string(),
                PlayerFilter::Specific(_) | PlayerFilter::AliasedTarget(_) => {
                    "that player has cast this turn".to_string()
                }
                PlayerFilter::Any => "cast this turn".to_string(),
                other => format!("cast this turn by {}", describe_player_filter(other)),
            };
            format!("the total mana value of {subject} {cast_surface}")
        }
        Value::DamageDealtThisTurnByTaggedSpellCast(_) => {
            "the damage dealt this turn by the chosen spell".to_string()
        }
        Value::CardTypesInGraveyard(filter) => format!(
            "the number of card types among cards in {}",
            describe_card_type_graveyard_scope(filter)
        ),
        Value::Devotion { player, color } => format!(
            "{} devotion to {}",
            describe_possessive_player_filter(player),
            color.name().to_string()
        ),
        Value::ColorsOfManaSpentToCastThisSpell => {
            "the number of colors of mana spent to cast this spell".to_string()
        }
        Value::ManaSpentToCastThisSpell => "the amount of mana spent to cast this spell".to_string(),
        Value::ManaSymbolSpentToCastThisSpell { symbol, reference } => format!(
            "the amount of {} spent to cast {}",
            describe_mana_symbol(*symbol),
            reference.text()
        ),
        Value::ManaFromSourceSpentToCastThisSpell {
            source_filter,
            include_source_noun,
            reference,
        } => {
            let mut source = source_filter.description();
            if *include_source_noun {
                source.push_str(" source");
            }
            format!(
                "the amount of mana from {} spent to cast {}",
                with_indefinite_article(&source),
                reference.text()
            )
        }
        Value::ManaSpentToCastTriggeringObject => {
            "the amount of mana spent to cast that spell".to_string()
        }
        Value::UnspentMana(player) => {
            let subject = describe_player_filter(player);
            let verb = player_verb(&subject, "have", "has");
            format!("the amount of unspent mana {subject} {verb}")
        }
        Value::MagicGamesLostToOpponentsSinceLastWin => {
            "the number of Magic games you've lost to one of your opponents since you last won a game against them".to_string()
        }
        Value::DraftNotedHighestNumber { card_name } => format!(
            "the highest number you noted for cards named {}",
            title_case_card_name_fragment(card_name)
        ),
        Value::LastNotedLifeTotal => "the last noted life total for this permanent".to_string(),
        Value::PlayerCounters(player, counter_type) => format!(
            "the number of {} counters {}",
            counter_type.description(),
            describe_player_counter_holder(player)
        ),
        Value::EffectValue(_) => "X".to_string(),
        Value::EffectValueOffset(_, offset) => {
            if *offset == 0 {
                "X".to_string()
            } else if *offset > 0 {
                format!("X plus {}", offset)
            } else {
                format!("X minus {}", -offset)
            }
        }
        Value::EffectMetric { metric, .. } => describe_effect_metric_value(*metric, None),
        Value::EffectMetricOffset { metric, offset, .. } => {
            describe_effect_metric_value(*metric, Some(*offset))
        }
        Value::PendingEffectMetric { metric, .. } => describe_effect_metric_value(*metric, None),
        Value::PendingEffectMetricOffset { metric, offset, .. } => {
            describe_effect_metric_value(*metric, Some(*offset))
        }
        Value::PriorEffectMetric { query, .. }
        | Value::PendingPriorEffectMetric(query) => describe_prior_effect_metric_value(query),
        Value::EventValue(EventValueSpec::Amount)
        | Value::EventValue(EventValueSpec::LifeAmount) => "that much".to_string(),
        Value::EventValueOffset(EventValueSpec::Amount, offset)
        | Value::EventValueOffset(EventValueSpec::LifeAmount, offset) => {
            if *offset == 0 {
                "that much".to_string()
            } else if *offset > 0 {
                format!("that much plus {}", offset)
            } else {
                format!("that much minus {}", -offset)
            }
        }
        Value::EventValue(EventValueSpec::BlockersBeyondFirst { multiplier }) => {
            if *multiplier == 1 {
                "the number of blockers beyond the first".to_string()
            } else {
                format!("{multiplier} times the number of blockers beyond the first")
            }
        }
        Value::EventValueOffset(EventValueSpec::BlockersBeyondFirst { multiplier }, offset) => {
            let base = if *multiplier == 1 {
                "the number of blockers beyond the first".to_string()
            } else {
                format!("{multiplier} times the number of blockers beyond the first")
            };
            if *offset == 0 {
                base
            } else if *offset > 0 {
                format!("{base} plus {}", offset)
            } else {
                format!("{base} minus {}", -offset)
            }
        }
        Value::WasKicked => "whether this spell was kicked (1 or 0)".to_string(),
        Value::WasBoughtBack => "whether buyback was paid (1 or 0)".to_string(),
        Value::WasEntwined => "whether entwine was paid (1 or 0)".to_string(),
        Value::WasPaid(index) => format!("whether optional cost #{index} was paid (1 or 0)"),
        Value::WasPaidLabel(label) => {
            format!("whether optional cost '{label}' was paid (1 or 0)")
        }
        Value::TimesPaid(index) => format!("how many times optional cost #{index} was paid"),
        Value::TimesPaidLabel(label) => {
            format!("how many times optional cost '{label}' was paid")
        }
        Value::KickCount => "how many times this spell was kicked".to_string(),
        Value::CountersOnSource(counter_type) => format!(
            "the number of {} counters on this source",
            counter_type.description()
        ),
        Value::CountersOn(spec, Some(counter_type)) => {
            if let ChooseSpec::All(filter) = spec.unhinted() {
                format!(
                    "the number of {} counters on {}",
                    counter_type.description(),
                    render_effects::pluralize_noun_phrase(&filter.description())
                )
            } else {
                format!(
                    "the number of {} counters on {}",
                    counter_type.description(),
                    describe_choose_spec(spec)
                )
            }
        }
        Value::CountersOn(spec, None) => {
            if let ChooseSpec::All(filter) = spec.unhinted() {
                format!(
                    "the number of counters on {}",
                    render_effects::pluralize_noun_phrase(&filter.description())
                )
            } else {
                format!("the number of counters on {}", describe_choose_spec(spec))
            }
        }
        Value::TaggedCount => "the tagged object count".to_string(),
    }
}
