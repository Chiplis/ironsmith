use super::*;

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

pub(crate) fn describe_card_count(value: &Value) -> String {
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
        value if value.has_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo) => {
            format!("cards equal to {}", describe_value(value.unhinted()))
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

pub(crate) fn value_text_describes_card_count(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower == "a card"
        || lower.ends_with(" card")
        || lower.ends_with(" cards")
        || lower.contains("number of cards")
        || lower.contains("cards a player")
        || lower.contains("cards that player")
}

pub(crate) fn describe_discard_count(value: &Value, filter: Option<&ObjectFilter>) -> String {
    let Some(filter) = filter else {
        return match value {
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

    if !filter.tagged_constraints.is_empty() {
        return match value {
            Value::Fixed(1) => "that card".to_string(),
            _ => "those cards".to_string(),
        };
    }

    if let Value::Count(count_filter) = value {
        if count_filter.zone == Some(Zone::Hand) && count_filter.owner.is_some() {
            return describe_value(value);
        }
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
    filter.zone.is_none()
        && filter.controller.is_none()
        && (filter.owner.is_none() || matches!(filter.owner, Some(PlayerFilter::You)))
        && filter.targets_player.is_none()
        && filter.targets_object.is_none()
        && filter.card_types.is_empty()
        && filter.all_card_types.is_empty()
        && filter.excluded_card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.excluded_subtypes.is_empty()
        && filter.supertypes.is_empty()
        && filter.excluded_supertypes.is_empty()
        && filter.colors.is_none()
        && filter.excluded_colors.is_empty()
        && !filter.colorless
        && !filter.multicolored
        && !filter.monocolored
        && !filter.token
        && !filter.nontoken
        && !filter.other
        && !filter.tapped
        && !filter.untapped
        && !filter.attacking
        && !filter.nonattacking
        && !filter.blocking
        && !filter.nonblocking
        && !filter.blocked
        && !filter.unblocked
        && filter.power.is_none()
        && filter.toughness.is_none()
        && filter.mana_value.is_none()
        && !filter.has_mana_cost
        && !filter.has_tap_activated_ability
        && !filter.no_x_in_cost
        && !filter.has_x_in_cost
        && filter.name.is_none()
        && filter.excluded_name.is_none()
        && filter.alternative_cast.is_none()
        && filter.static_abilities.is_empty()
        && filter.excluded_static_abilities.is_empty()
        && filter.ability_markers.is_empty()
        && filter.excluded_ability_markers.is_empty()
        && !filter.is_commander
        && !filter.noncommander
        && filter.tagged_constraints.is_empty()
        && filter.specific.is_none()
        && filter.any_of.is_empty()
        && !filter.source
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

pub(crate) fn describe_count_filter_value_subject(filter: &ObjectFilter) -> String {
    if let Some(subject) = describe_commander_zone_union_subject(filter) {
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
    if filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str().starts_with("milled_")
    }) {
        return "those cards".to_string();
    }
    if filter.zone == Some(Zone::Hand)
        && filter.card_types.is_empty()
        && filter.all_card_types.is_empty()
        && filter.subtypes.is_empty()
        && let Some(owner) = &filter.owner
    {
        return format!("cards in {} hand", describe_possessive_player_filter(owner));
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
    let mut subject = strip_indefinite_article(&filter.description())
        .trim()
        .to_string();
    subject = pluralize_noun_phrase(&subject);
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
        && !filter.entered_battlefield_this_turn
        && filter.entered_battlefield_controller.is_none()
    {
        subject.push_str(" on the battlefield");
    }
    if has_sacrificed_tag && !subject.to_ascii_lowercase().starts_with("the sacrificed ") {
        subject = format!(
            "the sacrificed {}",
            subject.trim_start_matches("the ").trim()
        );
    }

    subject
}

pub(crate) fn describe_domain_union_count_filter_subject(filter: &ObjectFilter) -> Option<String> {
    if filter.any_of.len() < 2 {
        return None;
    }

    let mut outer = filter.clone();
    outer.any_of.clear();
    if outer != ObjectFilter::default() {
        return None;
    }

    let first_signature = domain_union_signature(filter.any_of.first()?)?;
    if filter.any_of.iter().any(|branch| {
        domain_union_signature(branch)
            .as_ref()
            .is_none_or(|signature| signature != &first_signature)
    }) {
        return None;
    }

    let subjects = filter
        .any_of
        .iter()
        .map(describe_count_filter_value_subject)
        .collect::<Vec<_>>();
    if subjects.iter().any(|subject| subject.trim().is_empty()) {
        return None;
    }

    Some(join_with_and(&subjects))
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
    if let Some(subject) = describe_tagged_hand_origin_count_filter(filter) {
        return subject;
    }

    let mut bare = filter.clone();
    let controller = bare.controller.clone();
    let owner = bare.owner.clone();
    bare.controller = None;
    let keep_owner_in_subject = owner.is_some()
        && matches!(
            bare.zone,
            Some(Zone::Graveyard | Zone::Hand | Zone::Library | Zone::Exile | Zone::Command)
        );
    if !keep_owner_in_subject {
        bare.owner = None;
    }

    let mut subject = strip_indefinite_article(&bare.description()).to_string();
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
    if let Some(action) = describe_tagged_this_way_action(filter) {
        if action == "exiled" {
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
        subject = format!("{subject} {action} this way");
    }

    let controller_suffix = match controller {
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
        Some(PlayerFilter::Target(_)) | Some(PlayerFilter::IteratedPlayer) => {
            Some("that player controls")
        }
        Some(PlayerFilter::TaggedPlayer(_)) | Some(PlayerFilter::ChosenPlayer) => {
            Some("they control")
        }
        _ => None,
    };
    if let Some(suffix) = controller_suffix {
        if let Some((head, tail)) = subject.split_once(" that shares ") {
            return format!("{} {} that shares {}", head.trim(), suffix, tail.trim());
        }
        if let Some((head, tail)) = subject.split_once(" named ") {
            return format!("{} {} named {}", head.trim(), suffix, tail.trim());
        }
        if let Some((head, tail)) = subject.split_once(" not named ") {
            return format!("{} {} not named {}", head.trim(), suffix, tail.trim());
        }
        if let Some(head) = subject.strip_suffix(" of the chosen type") {
            return format!("{} {suffix} of the chosen type", head.trim());
        }
        return format!("{subject} {suffix}");
    }

    let owner_suffix = if keep_owner_in_subject {
        None
    } else {
        match owner {
            Some(PlayerFilter::You) => Some("you own"),
            Some(PlayerFilter::NotYou) => Some("you don't own"),
            Some(PlayerFilter::Opponent) => Some("an opponent owns"),
            Some(PlayerFilter::Any) => Some("a player owns"),
            Some(PlayerFilter::Active) => Some("they own"),
            Some(PlayerFilter::Defending) => Some("defending player owns"),
            Some(PlayerFilter::Attacking) => Some("attacking player owns"),
            Some(PlayerFilter::DamagedPlayer) => Some("that player owns"),
            Some(PlayerFilter::Teammate) => Some("a teammate owns"),
            Some(PlayerFilter::Specific(_)) => Some("that player owns"),
            Some(PlayerFilter::Target(_)) | Some(PlayerFilter::IteratedPlayer) => {
                Some("that player owns")
            }
            Some(PlayerFilter::TaggedPlayer(_)) | Some(PlayerFilter::ChosenPlayer) => {
                Some("they own")
            }
            _ => None,
        }
    };
    if let Some(suffix) = owner_suffix {
        if let Some((head, tail)) = subject.split_once(" named ") {
            return format!("{} {} named {}", head.trim(), suffix, tail.trim());
        }
        if let Some((head, tail)) = subject.split_once(" not named ") {
            return format!("{} {} not named {}", head.trim(), suffix, tail.trim());
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

    if base == crate::filter::ObjectFilter::default() {
        return Some("that object".to_string());
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

pub(crate) fn describe_choose_spec(spec: &ChooseSpec) -> String {
    match spec {
        ChooseSpec::SurfaceHinted { spec, hints } => {
            match hints.iter().find_map(|hint| match hint {
                crate::target::ChooseSpecSurfaceHint::SourceReference(surface) => Some(surface),
            }) {
                Some(surface) => describe_source_reference_surface_text(surface),
                None => describe_choose_spec(spec),
            }
        }
        ChooseSpec::Target(inner) => {
            if let ChooseSpec::Object(filter) = inner.as_ref()
                && let Some(exiled_card) = describe_simple_exiled_card_filter(filter)
            {
                return format!("target {exiled_card}");
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
                    strip_indefinite_article(&filter.description()).to_string()
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
        ChooseSpec::Object(filter) => {
            if let Some(exiled_card) = describe_simple_exiled_card_filter(filter) {
                ensure_indefinite_article(&exiled_card)
            } else if filter.source && filter.source_surface.is_some() {
                filter.description()
            } else if let Some(tagged_text) = describe_demonstrative_tagged_object_filter(filter) {
                tagged_text
            } else {
                ensure_indefinite_article(&filter.description())
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
            if is_implicit_reference_tag(tag.as_str()) {
                "it".to_string()
            } else {
                format!("the tagged object '{}'", tag.as_str())
            }
        }
        ChooseSpec::All(filter) => {
            if let Some(tagged_text) = describe_demonstrative_tagged_object_filter(filter) {
                if tagged_text == "that object" {
                    return "them".to_string();
                }
                if let Some(rest) = tagged_text.strip_prefix("that ") {
                    return format!("those {}", pluralize_noun_phrase(rest));
                }
            }
            let desc = filter.description();
            let stripped = strip_leading_article(&desc);
            format!("all {}", pluralize_relative_object_phrase(stripped))
        }
        ChooseSpec::EachPlayer(filter) => format!("each {}", describe_player_filter(filter)),
        ChooseSpec::SpecificObject(_) => "that object".to_string(),
        ChooseSpec::SpecificPlayer(_) => "that player".to_string(),
        ChooseSpec::Iterated => "that object".to_string(),
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
                    let plural = pluralize_relative_object_phrase(base);
                    let count_text =
                        |n: usize| number_word(n as i32).unwrap_or_else(|| n.to_string());
                    if count.is_up_to_dynamic_x() {
                        return format!(
                            "up to X target {plural}{controller_suffix}{random_suffix}"
                        );
                    }
                    if count.is_dynamic_x() {
                        return format!("X target {plural}{controller_suffix}{random_suffix}");
                    }
                    match (count.min, count.max) {
                        (0, None) => {
                            format!(
                                "any number of target {plural}{controller_suffix}{random_suffix}"
                            )
                        }
                        (min, None) => {
                            format!(
                                "at least {min} target {plural}{controller_suffix}{random_suffix}"
                            )
                        }
                        (0, Some(max)) => {
                            if max == 1 {
                                format!("up to one target {base}{controller_suffix}{random_suffix}")
                            } else {
                                format!(
                                    "up to {} target {plural}{controller_suffix}{random_suffix}",
                                    count_text(max)
                                )
                            }
                        }
                        (min, Some(max)) if min == max => {
                            if min == 1 {
                                format!("target {base}{controller_suffix}{random_suffix}")
                            } else {
                                format!(
                                    "{} target {plural}{controller_suffix}{random_suffix}",
                                    count_text(min)
                                )
                            }
                        }
                        (1, Some(2)) => {
                            format!("one or two target {plural}{controller_suffix}{random_suffix}")
                        }
                        (1, Some(3)) => {
                            format!(
                                "one, two, or three target {plural}{controller_suffix}{random_suffix}"
                            )
                        }
                        (min, Some(max)) => {
                            format!(
                                "{} to {} target {plural}{controller_suffix}{random_suffix}",
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
                        (min, None) => {
                            if min == 1 {
                                format!("at least one {base}{controller_suffix}{random_suffix}")
                            } else {
                                format!(
                                    "at least {} {plural}{controller_suffix}{random_suffix}",
                                    count_text(min)
                                )
                            }
                        }
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
        return Some(format!("{base}{random_suffix}"));
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
            let looks_like_plain_creature_filter = filter.zone == Some(Zone::Battlefield)
                && filter.card_types == vec![CardType::Creature]
                && filter.all_card_types.is_empty()
                && filter.excluded_card_types.is_empty()
                && filter.subtypes.is_empty()
                && filter.excluded_subtypes.is_empty()
                && !filter.source;
            if looks_like_plain_creature_filter {
                if let Some(controller) = filter.controller.as_ref() {
                    return match controller {
                        PlayerFilter::Opponent | PlayerFilter::NotYou => {
                            "all creatures you don't control".to_string()
                        }
                        PlayerFilter::Target(inner) => {
                            let who = describe_player_filter(inner);
                            if who == "player" {
                                "each creature target player controls".to_string()
                            } else {
                                format!("each creature target {who} controls")
                            }
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
        ChooseSpec::Target(inner)
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
                let text = filter.description();
                let suffix = match &filter.owner {
                    Some(owner) => {
                        format!(
                            " in {} graveyard",
                            describe_possessive_graveyard_owner_filter(owner)
                        )
                    }
                    None => {
                        if filter.single_graveyard {
                            " in single graveyard".to_string()
                        } else {
                            " in a graveyard".to_string()
                        }
                    }
                };
                if let Some(stripped) = text
                    .strip_suffix(&suffix)
                    .or_else(|| text.strip_suffix(" in graveyard"))
                {
                    return ensure_indefinite_article(&render_artifact_non_aura_enchantment_text(
                        filter, stripped,
                    ));
                }
                return ensure_indefinite_article(&render_artifact_non_aura_enchantment_text(
                    filter, &text,
                ));
            }
            ensure_indefinite_article(&render_artifact_non_aura_enchantment_text(
                filter,
                &filter.description(),
            ))
        }
        ChooseSpec::PlayerOrPlaneswalker(filter) => match filter {
            PlayerFilter::Opponent => "target opponent or planeswalker".to_string(),
            PlayerFilter::Any => "target player or planeswalker".to_string(),
            other => format!("target {} or planeswalker", describe_player_filter(other)),
        },
        ChooseSpec::AttackedPlayerOrPlaneswalker => {
            "the player or planeswalker it's attacking".to_string()
        }
        ChooseSpec::All(filter) => {
            if filter.zone == Some(Zone::Graveyard) {
                let text = filter.description();
                let suffix = match &filter.owner {
                    Some(owner) => {
                        format!(
                            " in {} graveyard",
                            describe_possessive_graveyard_owner_filter(owner)
                        )
                    }
                    None => {
                        if filter.single_graveyard {
                            " in single graveyard".to_string()
                        } else {
                            " in a graveyard".to_string()
                        }
                    }
                };
                if let Some(stripped) = text
                    .strip_suffix(&suffix)
                    .or_else(|| text.strip_suffix(" in graveyard"))
                {
                    let stripped = strip_leading_article(stripped);
                    return format!("all {}", pluralize_relative_object_phrase(stripped));
                }
                let text = strip_leading_article(&text);
                return format!("all {}", pluralize_relative_object_phrase(text));
            }
            let desc = filter.description();
            let stripped = strip_leading_article(&desc);
            format!("all {}", pluralize_relative_object_phrase(stripped))
        }
        ChooseSpec::WithCount(inner, count) | ChooseSpec::WithCountValue(inner, count, _) => {
            let inner_text = describe_choose_spec_without_graveyard_zone(inner);
            if count.is_single() {
                inner_text
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
                        return format!("up to X target {plural}");
                    }
                    if count.is_dynamic_x() {
                        return format!("X target {plural}");
                    }
                    match (count.min, count.max) {
                        (0, None) => format!("any number of target {plural}"),
                        (min, None) => format!("at least {min} target {plural}"),
                        (0, Some(max)) => {
                            if max == 1 {
                                format!("up to one target {base}")
                            } else {
                                format!("up to {} target {plural}", count_text(max))
                            }
                        }
                        (min, Some(max)) if min == max => {
                            if min == 1 {
                                format!("target {base}")
                            } else {
                                format!("{} target {plural}", count_text(min))
                            }
                        }
                        (1, Some(2)) => format!("one or two target {plural}"),
                        (1, Some(3)) => format!("one, two, or three target {plural}"),
                        (min, Some(max)) => {
                            format!("{} to {} target {plural}", count_text(min), count_text(max))
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
                        return format!("up to X {plural}");
                    }
                    if count.is_dynamic_x() {
                        return format!("X {plural}");
                    }
                    match (count.min, count.max) {
                        (0, None) => format!("any number of {plural}"),
                        (min, None) => {
                            if min == 1 {
                                format!("at least one {base}")
                            } else {
                                format!("at least {} {plural}", count_text(min))
                            }
                        }
                        (0, Some(max)) => {
                            if max == 1 {
                                format!("up to one {base}")
                            } else {
                                format!("up to {} {plural}", count_text(max))
                            }
                        }
                        (min, Some(max)) if min == max => {
                            if min == 1 {
                                format!("one {base}")
                            } else {
                                format!("{} {plural}", count_text(min))
                            }
                        }
                        (min, Some(max)) => {
                            format!("{} to {} {plural}", count_text(min), count_text(max))
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
        && filter.card_types.contains(&CardType::Enchantment);
    if !has_artifact_enchantment_types
        || filter.excluded_subtypes != [Subtype::Aura]
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
    if text.contains("artifacts or enchantment cards with mana value") {
        format!("artifact and/or non-Aura enchantment cards each with mana value {mana_value_text}")
    } else if text.contains("artifact or enchantment card with mana value") {
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
}

pub(crate) fn describe_choice_count(count: &ChoiceCount) -> String {
    let base = if count.is_up_to_dynamic_x() {
        "up to X".to_string()
    } else if count.is_dynamic_x() {
        "X".to_string()
    } else {
        match (count.min, count.max) {
            (0, None) => "any number".to_string(),
            (min, None) => format!("at least {min}"),
            (0, Some(max)) => format!("up to {max}"),
            (min, Some(max)) if min == max => format!("exactly {min}"),
            (min, Some(max)) => format!("{min} to {max}"),
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

pub(crate) fn describe_search_selection_with_cards(selection: &str) -> String {
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
    if let Some(subtype) = selection.strip_prefix("a basic land card ") {
        return format!("a basic {} card", subtype.trim());
    }
    if let Some(subtype) = selection.strip_prefix("basic land card ") {
        return format!("a basic {} card", subtype.trim());
    }
    if let Some(subtype) = selection.strip_prefix("a land card ") {
        return format!("{} card", with_indefinite_article(subtype.trim()));
    }
    if let Some(subtype) = selection.strip_prefix("land card ") {
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
    if selection.contains(" card") {
        return selection.to_string();
    }
    if let Some(rest) = selection.strip_prefix("up to ") {
        let mut parts = rest.splitn(2, ' ');
        let amount = parts.next().unwrap_or_default().trim();
        let tail = parts.next().unwrap_or_default().trim();
        if !tail.is_empty() {
            if amount == "1" || amount.eq_ignore_ascii_case("one") {
                return format!("a {tail} card");
            }
            return format!("up to {amount} {tail} cards");
        }
    }
    if let Some(rest) = selection.strip_prefix("any number ") {
        let rest = rest.trim_start_matches("of ").trim();
        if !rest.is_empty() {
            return format!("any number of {rest} cards");
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
                allow_colorless = true;
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

    if color_mode_count != 5 {
        return None;
    }
    let target_desc = describe_choose_spec(target?);
    Some(if allow_colorless {
        format!(
            "{target_desc} gains protection from colorless or from the color of your choice until end of turn"
        )
    } else {
        format!("{target_desc} gains protection from the color of your choice until end of turn")
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
    let mut abilities = Vec::new();

    for mode in &choose_mode.modes {
        if mode.effects.len() != 1 {
            return None;
        }
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
    let verb = if plural_subject { "gain" } else { "gains" };
    let choice_text = join_with_or(&abilities);
    Some(format!(
        "{subject} {verb} your choice of {choice_text} until end of turn"
    ))
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
            "the difference".to_string()
        }
        Value::SurfaceHinted { value, .. } => describe_value(value),
        Value::Fixed(n) => n.to_string(),
        Value::Add(left, right) => {
            if left == right {
                return format!("twice {}", describe_value(left));
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
            }
            format!("half {}, rounded down", describe_value(value))
        }
        Value::X => "X".to_string(),
        Value::XTimes(factor) => {
            if *factor == 1 {
                "X".to_string()
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
        Value::TotalPower(filter) => {
            format!(
                "the total power of {}",
                describe_count_filter_value_subject(filter)
            )
        }
        Value::TotalToughness(filter) => {
            format!(
                "the total toughness of {}",
                describe_count_filter_value_subject(filter)
            )
        }
        Value::TotalManaValue(filter) => {
            format!(
                "the total mana value of {}",
                describe_count_filter_value_subject(filter)
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
            format!(
                "the greatest mana value among {}",
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
        Value::DistinctNames(filter) => format!(
            "the number of differently named {}",
            describe_count_filter_value_subject(filter)
        ),
        Value::DistinctPowers(filter) => format!(
            "the number of different powers among {}",
            describe_count_filter_value_subject(filter)
        ),
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
        Value::PlayersWhoControlMoreThanYou(filter) => {
            let mut controlled_filter = filter.clone();
            if controlled_filter.zone == Some(Zone::Battlefield) {
                controlled_filter.zone = None;
            }
            format!(
                "the number of players who control more {} than you",
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
            if let ChooseSpec::Tagged(tag) = spec.base()
                && tag.as_str() == crate::tag::SOURCE_EXILED_TAG
            {
                "the exiled card's power".to_string()
            } else {
                format!("{} power", describe_possessive_choose_spec(spec))
            }
        }
        Value::ToughnessOf(spec) => {
            if let ChooseSpec::Tagged(tag) = spec.base()
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
            if let ChooseSpec::Tagged(tag) = spec.base()
                && tag.as_str() == "triggering"
            {
                "that spell's mana value".to_string()
            } else if let ChooseSpec::Tagged(tag) = spec.base()
                && (tag.as_str().starts_with("revealed_")
                    || tag.as_str() == crate::effects::PUBLIC_REVEALED_TAG
                    || tag.as_str().starts_with("searched_")
                    || tag.as_str().starts_with("milled_")
                    || tag.as_str().starts_with("discarded_")
                    || tag.as_str().starts_with("exiled_")
                    || tag.as_str().starts_with("__sentence_helper_exiled"))
            {
                "that card's mana value".to_string()
            } else {
                format!("{} mana value", describe_possessive_choose_spec(spec))
            }
        }
        Value::LifeTotal(filter) => {
            format!("{} life total", describe_possessive_player_filter(filter))
        }
        Value::LifeTotalAsTurnBegan(filter) => format!(
            "{} life total as the turn began",
            describe_possessive_player_filter(filter)
        ),
        Value::LifeTotalDifference(filter) => match filter {
            PlayerFilter::Target(_) => "the difference between those players' life totals".to_string(),
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
            PlayerFilter::You => "the total life you lost this turn".to_string(),
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
        Value::DamageDealtToPlayersThisTurn(filter) => match filter {
            PlayerFilter::You => "the damage already dealt to you this turn".to_string(),
            PlayerFilter::Opponent => {
                "the damage already dealt to your opponents this turn".to_string()
            }
            PlayerFilter::Target(_) => {
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
                    "the number of {} counters among {}",
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
                    "the number of counters among {}",
                    render_effects::pluralize_noun_phrase(&filter.description())
                )
            } else {
                format!("the number of counters on {}", describe_choose_spec(spec))
            }
        }
        Value::TaggedCount => "the tagged object count".to_string(),
    }
}
