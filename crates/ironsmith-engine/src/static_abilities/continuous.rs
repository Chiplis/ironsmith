//! Effect-generating static abilities.
//!
//! These abilities generate continuous effects that modify other objects
//! through the layer system.

use super::{
    StaticAbility, StaticAbilityId, StaticAbilityKind,
    text_utils::{capitalize_first, join_with_and, number_word_u32},
};
use crate::ability::{Ability, AbilityKind};
use crate::continuous::{
    ContinuousEffect, EffectSourceType, EffectTarget, Modification, PtSublayer,
};
use crate::effect::{Comparison, Value};
use crate::filter::ObjectFilterExt as _;
use crate::filter::{
    PlayerFilterExt, TaggedConstraintSubject, TaggedOpbjectRelation, describe_player_filter,
};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;
use crate::target::{ChooseSpec, ObjectFilter, ObjectRef, PlayerFilter, SourceReferenceSurface};
use crate::types::{CardType, Subtype, SubtypeFamily, Supertype};
use crate::zone::Zone;

mod grants;

pub use grants::*;

#[cfg(test)]
mod tests;

fn attached_subject(filter: &ObjectFilter) -> Option<String> {
    if filter.controller.is_some() || filter.owner.is_some() || filter.other {
        return None;
    }
    let attachment = filter.tagged_constraints.iter().find_map(|constraint| {
        if constraint.relation != TaggedOpbjectRelation::IsTaggedObject {
            return None;
        }
        match constraint.tag.as_str() {
            "enchanted" => Some("enchanted"),
            "equipped" => Some("equipped"),
            _ => None,
        }
    })?;

    let noun = if filter.card_types.len() == 1 {
        filter.card_types[0].name().to_string()
    } else {
        "permanent".to_string()
    };
    Some(format!("{attachment} {noun}"))
}

fn effect_target_for_filter(source: ObjectId, filter: &ObjectFilter) -> EffectTarget {
    if attached_subject(filter).is_some() {
        EffectTarget::AttachedTo(source)
    } else {
        EffectTarget::Filter(filter.clone())
    }
}

fn filter_for_attached_subject_match(filter: &ObjectFilter) -> ObjectFilter {
    let mut stripped = filter.clone();
    if attached_subject(filter).is_some() {
        stripped.tagged_constraints.retain(|constraint| {
            constraint.relation != TaggedOpbjectRelation::IsTaggedObject
                || !matches!(constraint.tag.as_str(), "enchanted" | "equipped")
        });
    }
    stripped
}

fn color_list(colors: crate::color::ColorSet) -> Vec<String> {
    let mut list = Vec::new();
    if colors.contains(crate::color::Color::White) {
        list.push("white".to_string());
    }
    if colors.contains(crate::color::Color::Blue) {
        list.push("blue".to_string());
    }
    if colors.contains(crate::color::Color::Black) {
        list.push("black".to_string());
    }
    if colors.contains(crate::color::Color::Red) {
        list.push("red".to_string());
    }
    if colors.contains(crate::color::Color::Green) {
        list.push("green".to_string());
    }
    list
}

fn is_all_colors(colors: crate::color::ColorSet) -> bool {
    let all_colors: crate::color::ColorSet = crate::color::Color::ALL.into_iter().collect();
    colors == all_colors
}

fn is_exactly_basic_land_types(subtypes: &[Subtype]) -> bool {
    subtypes.len() == 5
        && [
            Subtype::Plains,
            Subtype::Island,
            Subtype::Swamp,
            Subtype::Mountain,
            Subtype::Forest,
        ]
        .iter()
        .all(|subtype| subtypes.contains(subtype))
}

fn lowercase_first_ascii(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_ascii_lowercase(), chars.as_str()),
        None => String::new(),
    }
}

fn object_ability_is_static_keyword(ability: &Ability) -> bool {
    matches!(&ability.kind, AbilityKind::Static(static_ability) if static_ability.is_keyword())
}

fn object_ability_keyword_label(ability: &Ability) -> Option<String> {
    match &ability.kind {
        AbilityKind::Triggered(triggered) => match triggered.presentation_label.as_ref()? {
            crate::ability::PresentationLabel::Keyword(keyword) => {
                Some(keyword.display().to_ascii_lowercase())
            }
            _ => None,
        },
        _ => None,
    }
}

fn explicit_granted_keyword_label(display: &str) -> Option<String> {
    let label = display.trim().trim_end_matches('.');
    let lower = label.to_ascii_lowercase();
    if lower == "scavenge. the scavenge cost is equal to its mana cost" {
        // This two-sentence surface is one structured granted keyword. Keep
        // it unquoted while preserving the authored sentence capitalization.
        return Some(label.to_string());
    }
    if matches!(lower.as_str(), "storm" | "gravestorm") {
        return Some(lower);
    }
    let amount = lower.strip_prefix("afflict ")?;
    amount
        .chars()
        .all(|ch| ch.is_ascii_digit())
        .then_some(lower)
}

fn capitalize_excluded_subtype_terms(filter: &ObjectFilter, mut text: String) -> String {
    for subtype in &filter.excluded_subtypes {
        let canonical = subtype.to_string();
        text = text.replace(
            &format!("non-{}", canonical.to_ascii_lowercase()),
            &format!("non-{canonical}"),
        );
    }
    text
}

fn subject_text(filter: &ObjectFilter) -> String {
    capitalize_excluded_subtype_terms(
        filter,
        attached_subject(filter).unwrap_or_else(|| filter.description()),
    )
}

fn shared_head_characteristic_modifier(filter: &ObjectFilter) -> Option<String> {
    if !filter.any_of.is_empty() {
        return None;
    }

    let modifier = match (filter.supertypes.as_slice(), filter.subtypes.as_slice()) {
        ([supertype], []) => supertype.to_string(),
        ([], [subtype]) => subtype.to_string(),
        _ => return None,
    };
    let mut remainder = filter.clone();
    remainder.supertypes.clear();
    remainder.subtypes.clear();
    (remainder == ObjectFilter::default()).then_some(modifier)
}

/// Render an elided coordinated characteristic head from its typed filter.
///
/// For example, `snow and Zombie creatures you control` is represented as a
/// shared outer creature/controller filter plus `Snow` and `Zombie` union
/// arms. Keeping the shared noun outside the arms is what makes both runtime
/// alternatives creatures; this helper only restores the authored elision.
fn shared_head_characteristic_anthem_subject(filter: &ObjectFilter) -> Option<String> {
    if !filter.has_conjunctive_set_surface()
        || filter.any_of.len() < 2
        || filter.card_types.len() != 1
        || !filter.all_card_types.is_empty()
        || !filter.subtypes.is_empty()
        || !filter.supertypes.is_empty()
    {
        return None;
    }

    let modifiers = filter
        .any_of
        .iter()
        .map(shared_head_characteristic_modifier)
        .collect::<Option<Vec<_>>>()?;
    let mut outer = filter.clone();
    outer.any_of.clear();
    outer.set_conjunctive_set_surface(false);
    let mut subject = pluralized_subject_text(&outer);
    let shared_noun = pluralize_subject_clause(filter.card_types[0].name());
    if !subject.contains(&shared_noun) {
        return None;
    }
    let replacement = format!("{} {shared_noun}", join_with_and(&modifiers));
    subject = subject.replacen(&shared_noun, &replacement, 1);
    Some(subject)
}

fn strip_plural_subject_article(subject: &str) -> &str {
    for article in ["a ", "an "] {
        if let Some(rest) = subject.strip_prefix(article) {
            let first_word = rest.split_whitespace().next().unwrap_or_default();
            if !first_word.ends_with("'s") {
                return rest;
            }
        }
    }
    subject
}

fn split_subject_suffix(subject: &str) -> (&str, &str) {
    const SUFFIXES: &[&str] = &[
        " you control",
        " you don't control",
        " your team controls",
        " that player controls",
        " that player or that object's controller controls",
        " you own",
        " you don't own",
        " an opponent owns",
        " a player owns",
        " the active player owns",
        " the player who cast this spell owns",
        " that player owns",
        " a teammate owns",
        " the defending player owns",
        " an attacking player owns",
        " the damaged player owns",
        " target player owns",
        " target opponent owns",
        " that object's controller owns",
        " that object's owner owns",
    ];
    for suffix in SUFFIXES {
        if let Some(base) = subject.strip_suffix(suffix) {
            return (base, suffix);
        }
    }
    const CLAUSE_MARKERS: &[&str] = &[
        " in ",
        " with ",
        " without ",
        " that ",
        " from ",
        " named ",
        " of ",
    ];
    let lower = subject.to_ascii_lowercase();
    if let Some(split_at) = CLAUSE_MARKERS
        .iter()
        .filter_map(|marker| lower.find(marker))
        .min()
    {
        return (&subject[..split_at], &subject[split_at..]);
    }
    (subject, "")
}

pub(crate) fn pluralized_subject_text(filter: &ObjectFilter) -> String {
    if let Some(subject) = shared_head_characteristic_anthem_subject(filter) {
        return subject;
    }
    // Coordinated relative characteristic lists put the grammatical head
    // before the final selector ("a creature you control that's a Zombie
    // and/or token").  The local fallback pluralizer normally pluralizes the
    // rightmost noun, which leaves that head singular.  Reuse the compiled
    // text pluralizer for this authored surface; it understands both the
    // relative-clause agreement and each coordinated selector.
    if filter.has_relative_characteristic_list_surface() {
        return crate::runtime_display::pluralize_noun_phrase_for_trigger(&subject_text(filter));
    }
    if filter.has_relative_attachment_state_surface() && filter.any_of.len() == 2 {
        let mut base_filter = None;
        let mut attachments = Vec::new();
        for branch in &filter.any_of {
            let mut branch_base = branch.clone();
            let Some(index) = branch_base
                .tagged_constraints
                .iter()
                .position(|constraint| {
                    constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                        && matches!(constraint.tag.as_str(), "enchanted" | "equipped")
                })
            else {
                attachments.clear();
                break;
            };
            attachments.push(
                branch_base
                    .tagged_constraints
                    .remove(index)
                    .tag
                    .as_str()
                    .to_string(),
            );
            if base_filter
                .as_ref()
                .is_some_and(|base| base != &branch_base)
            {
                attachments.clear();
                break;
            }
            base_filter = Some(branch_base);
        }
        if attachments.len() == 2
            && attachments[0] != attachments[1]
            && let Some(mut base) = base_filter
        {
            base.set_relative_attachment_state_surface(false);
            return format!(
                "{} that are {} or {}",
                pluralized_subject_text(&base),
                attachments[0],
                attachments[1]
            );
        }
    }
    if filter.has_relative_attachment_state_surface()
        && let Some(attachment) = &filter.with_attached_object
    {
        let mut base = filter.clone();
        base.with_attached_object = None;
        base.set_relative_attachment_state_surface(false);
        return format!(
            "{} that are enchanted by {}",
            pluralized_subject_text(&base),
            pluralize_subject_clause(&attachment.description())
        );
    }
    let mut subject = subject_text(filter);
    if filter.has_relative_attachment_state_surface()
        && let Some(attachment) = filter.tagged_constraints.iter().find_map(|constraint| {
            (constraint.relation == TaggedOpbjectRelation::IsTaggedObject)
                .then_some(constraint.tag.as_str())
                .filter(|tag| matches!(*tag, "enchanted" | "equipped"))
        })
    {
        let without_article = strip_plural_subject_article(&subject);
        if let Some(base) = without_article.strip_prefix(&format!("{attachment} ")) {
            return format!("{} that are {attachment}", pluralize_subject_clause(base));
        }
    }
    if subject.starts_with("another ") {
        subject = subject.replacen("another ", "other ", 1);
    }
    if filter.has_conjunctive_set_surface()
        && let Some(serial) = pluralize_serial_subject_list(&subject)
    {
        return serial;
    }
    if let Some((base, tail)) = subject.split_once(" blocking or blocked by ") {
        return format!(
            "{} blocking or blocked by {}",
            pluralize_noun_phrase(base),
            tail
        );
    }
    let should_preserve_singular = (subject.starts_with("enchanted ")
        || subject.starts_with("equipped "))
        && filter.controller.is_none()
        && filter.owner.is_none()
        && !filter.other;
    if should_preserve_singular || subject.starts_with("this ") || subject.starts_with("that ") {
        return subject;
    }

    // Strip indefinite articles from object nouns, including each side of a
    // compound filter such as "artifact or enchantment you control".
    let subject = strip_plural_subject_article(&subject).to_string();

    let (base, suffix) = split_subject_suffix(&subject);
    if !base.is_empty() {
        // Oracle quantifies unscoped grants: "All creatures have protection
        // from black", never bare "Creatures have ...".
        let prefix_all = suffix.is_empty()
            && !base.contains(" or ")
            && !base.contains(" and ")
            && !base.contains(' ')
            && base
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_alphabetic());
        let subject = pluralize_subject_clause(&subject);
        return if prefix_all {
            format!("All {subject}")
        } else {
            subject
        };
    }

    subject
}

fn pluralize_serial_subject_list(subject: &str) -> Option<String> {
    let subject = strip_plural_subject_article(subject.trim());
    let (base, suffix) = split_subject_suffix(subject);
    let (separator, connective) = if base.contains(", and ") {
        (", and ", "and")
    } else if base.contains(", or ") {
        (", or ", "or")
    } else {
        return None;
    };
    let normalized = base.replace(separator, ", ");
    let parts = normalized
        .split(", ")
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(pluralize_noun_phrase)
        .collect::<Vec<_>>();
    if parts.len() < 3 {
        return None;
    }
    let joined = format!(
        "{}, {connective} {}",
        parts[..parts.len() - 1].join(", "),
        parts.last()?
    );
    Some(format!("{joined}{suffix}"))
}

fn exact_one_condition_antecedent_subject(
    filter: &ObjectFilter,
    condition: Option<&crate::ConditionExpr>,
) -> Option<String> {
    let Some(crate::ConditionExpr::CountComparison {
        count: AnthemCountExpression::MatchingFilter(counted_filter),
        comparison: Comparison::Equal(1),
        ..
    }) = condition
    else {
        return None;
    };
    if filter != counted_filter {
        return None;
    }
    let [card_type] = filter.card_types.as_slice() else {
        return None;
    };
    Some(format!("that {}", card_type.name()))
}

fn subtype_creature_anthem_subject(filter: &ObjectFilter) -> Option<String> {
    if filter.zone != Some(Zone::Battlefield)
        || filter.controller != Some(PlayerFilter::You)
        || filter.owner.is_some()
        || filter.card_types != [CardType::Creature]
        || !filter.all_card_types.is_empty()
        || filter.subtypes.len() < 2
        || filter.type_or_subtype_union
        || !filter.excluded_card_types.is_empty()
        || !filter.excluded_subtypes.is_empty()
    {
        return None;
    }

    let subtype_text = filter
        .subtypes
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" or ");
    let article = indefinite_article_for(&subtype_text);
    let other = if filter.other { "other " } else { "" };
    Some(format!(
        "Each {other}creature you control that's {article} {subtype_text}"
    ))
}

fn pluralize_subject_clause(subject: &str) -> String {
    let subject = strip_plural_subject_article(subject.trim());
    let (qualified_base, qualified_suffix) = split_subject_suffix(subject);
    if !qualified_suffix.is_empty()
        && (qualified_suffix.contains(" or ") || qualified_suffix.contains(" and "))
    {
        return format!(
            "{}{}",
            pluralize_subject_clause(qualified_base),
            qualified_suffix
        );
    }
    if let Some((head, tail)) = subject.split_once(" or ") {
        return format!(
            "{} or {}",
            pluralize_subject_clause(head),
            pluralize_subject_clause(tail)
        );
    }
    if let Some((head, tail)) = subject.split_once(" and ") {
        return format!(
            "{} and {}",
            pluralize_subject_clause(head),
            pluralize_subject_clause(tail)
        );
    }

    let (base, suffix) = split_subject_suffix(subject);
    if base.is_empty() {
        subject.to_string()
    } else {
        format!("{}{}", pluralize_noun_phrase(base), suffix)
    }
}

fn simple_pluralize(word: &str) -> String {
    let lower = word.to_ascii_lowercase();
    if lower == "plains" || lower == "urzas" {
        return word.to_string();
    }
    if lower == "elf" {
        return if word
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            "Elves".to_string()
        } else {
            "elves".to_string()
        };
    }
    if lower == "dwarf" {
        return if word
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            "Dwarves".to_string()
        } else {
            "dwarves".to_string()
        };
    }
    if lower == "wolf" {
        return if word
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            "Wolves".to_string()
        } else {
            "wolves".to_string()
        };
    }
    if lower == "werewolf" {
        return if word
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            "Werewolves".to_string()
        } else {
            "werewolves".to_string()
        };
    }
    if lower == "myr" || lower == "merfolk" || lower == "equipment" {
        return word.to_string();
    }
    if lower == "mouse" {
        return if word
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            "Mice".to_string()
        } else {
            "mice".to_string()
        };
    }
    if lower.ends_with('s')
        || lower.ends_with('x')
        || lower.ends_with('z')
        || lower.ends_with("ch")
        || lower.ends_with("sh")
    {
        format!("{word}es")
    } else if lower.ends_with('y')
        && lower.len() > 1
        && !matches!(
            lower.chars().nth(lower.len() - 2),
            Some('a' | 'e' | 'i' | 'o' | 'u')
        )
    {
        format!("{}ies", &word[..word.len() - 1])
    } else if matches!(lower.as_str(), "hero" | "potato" | "tomato") {
        // English -o plurals are irregular (Heroes, but Rhinos); enumerate
        // the -es cases that appear as game nouns.
        format!("{word}es")
    } else {
        format!("{word}s")
    }
}

fn pluralize_noun_phrase(phrase: &str) -> String {
    if let Some(rest) = phrase.strip_prefix("another ") {
        return format!("other {}", pluralize_noun_phrase(rest));
    }

    const NOUNS: &[(&str, &str)] = &[
        ("permanent", "permanents"),
        ("creature", "creatures"),
        ("artifact", "artifacts"),
        ("enchantment", "enchantments"),
        ("land", "lands"),
        ("planeswalker", "planeswalkers"),
        ("battle", "battles"),
        ("spell", "spells"),
        ("card", "cards"),
        ("token", "tokens"),
        ("ability", "abilities"),
        ("source", "sources"),
    ];

    let lower = phrase.to_ascii_lowercase();
    // `ObjectFilter::description` may already expose a plural coordinated
    // set (for example, "lands you control and land cards you own ...").
    // Track both singular and plural noun matches so this helper stays
    // idempotent instead of producing "landses" or "lands cards".
    let mut best_match: Option<(usize, usize, Option<&'static str>)> = None;
    for &(singular, plural) in NOUNS {
        for (noun, replacement) in [(singular, Some(plural)), (plural, None)] {
            let mut search_start = 0;
            while let Some(relative_pos) = lower[search_start..].find(noun) {
                let pos = search_start + relative_pos;
                let before_ok = pos == 0 || phrase.as_bytes()[pos - 1] == b' ';
                let after_pos = pos + noun.len();
                let after_ok = after_pos >= phrase.len()
                    || phrase.as_bytes()[after_pos] == b' '
                    || phrase.as_bytes()[after_pos] == b'.';
                if before_ok
                    && after_ok
                    && best_match
                        .as_ref()
                        .is_none_or(|(best_pos, _, _)| pos >= *best_pos)
                {
                    best_match = Some((pos, noun.len(), replacement));
                }
                search_start = after_pos;
            }
        }
    }

    if let Some((pos, noun_len, replacement)) = best_match {
        let Some(plural) = replacement else {
            return phrase.to_string();
        };
        let suffix_start = pos + noun_len;
        return format!("{}{}{}", &phrase[..pos], plural, &phrase[suffix_start..]);
    }

    pluralize_terminal_word(phrase)
}

fn indefinite_article_for(word: &str) -> &'static str {
    match word.chars().next().map(|ch| ch.to_ascii_lowercase()) {
        Some('a' | 'e' | 'i' | 'o' | 'u') => "an",
        _ => "a",
    }
}

fn with_indefinite_article_unless_present(text: String) -> String {
    if text.starts_with("a ")
        || text.starts_with("an ")
        || text.starts_with("the ")
        || text.starts_with("your ")
        || text.starts_with("their ")
        || text.starts_with("this ")
        || text.starts_with("that ")
    {
        text
    } else {
        format!("{} {text}", indefinite_article_for(&text))
    }
}

fn pluralize_terminal_word(phrase: &str) -> String {
    if let Some((head, tail)) = phrase.rsplit_once(' ') {
        if head.trim().is_empty() {
            simple_pluralize(tail)
        } else {
            format!("{head} {}", simple_pluralize(tail))
        }
    } else {
        simple_pluralize(phrase)
    }
}

fn grant_subject_text(filter: &ObjectFilter) -> String {
    if let Some(subject) = spell_grant_subject_text(filter) {
        return subject;
    }
    pluralized_subject_text(filter)
}

pub fn grant_subject_with_set_quantifier(
    filter: &ObjectFilter,
    surface: Option<ironsmith_core::SetQuantifierSurface>,
) -> (String, bool) {
    let (subject, singular) = match surface {
        Some(ironsmith_core::SetQuantifierSurface::Each) => {
            let mut subject = if let Some(spell_subject) = spell_grant_subject_text(filter) {
                if let Some(index) = spell_subject.find("spells") {
                    format!(
                        "{}spell{}",
                        &spell_subject[..index],
                        &spell_subject[index + "spells".len()..]
                    )
                } else {
                    spell_subject
                }
            } else {
                strip_article(filter.description())
            };
            if subject.starts_with("another ") {
                subject = subject.replacen("another ", "other ", 1);
            }
            (format!("Each {}", lowercase_first_ascii(&subject)), true)
        }
        Some(ironsmith_core::SetQuantifierSurface::All) => {
            let subject = lowercase_first_ascii(&grant_subject_text(filter));
            // Unscoped generic filters already describe themselves with an
            // implicit "All" (for example, `creature` -> "All creatures").
            // The authored set quantifier owns that surface here, so avoid
            // stacking the two sources into "All all creatures".
            let subject = subject.strip_prefix("all ").unwrap_or(&subject);
            (format!("All {subject}"), false)
        }
        Some(ironsmith_core::SetQuantifierSurface::They) => ("They".to_string(), false),
        Some(ironsmith_core::SetQuantifierSurface::Those) => (
            format!(
                "Those {}",
                lowercase_first_ascii(&pluralized_subject_text(filter))
            ),
            false,
        ),
        None => (
            grant_subject_text(filter),
            filter.first_spell_cast_each_turn,
        ),
    };
    (capitalize_excluded_subtype_terms(filter, subject), singular)
}

fn describe_filter_comparison(cmp: &crate::filter::Comparison) -> String {
    let describe_values = |values: &[i32]| -> String {
        match values.len() {
            0 => String::new(),
            1 => values[0].to_string(),
            2 => format!("{} or {}", values[0], values[1]),
            _ => {
                let head = values[..values.len() - 1]
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{head}, or {}", values[values.len() - 1])
            }
        }
    };

    match cmp {
        crate::filter::Comparison::Equal(v) => v.to_string(),
        crate::filter::Comparison::OneOf(values) => describe_values(values),
        crate::filter::Comparison::NotEqual(v) => format!("not equal to {v}"),
        crate::filter::Comparison::LessThan(v) => format!("less than {v}"),
        crate::filter::Comparison::LessThanOrEqual(v) => format!("{v} or less"),
        crate::filter::Comparison::GreaterThan(v) => format!("greater than {v}"),
        crate::filter::Comparison::GreaterThanOrEqual(v) => format!("{v} or greater"),
        crate::filter::Comparison::EqualExpr(_)
        | crate::filter::Comparison::NotEqualExpr(_)
        | crate::filter::Comparison::LessThanExpr(_)
        | crate::filter::Comparison::LessThanOrEqualExpr(_)
        | crate::filter::Comparison::GreaterThanExpr(_)
        | crate::filter::Comparison::GreaterThanOrEqualExpr(_) => "a dynamic value".to_string(),
    }
}

fn spell_grant_subject_text(filter: &ObjectFilter) -> Option<String> {
    let is_spell_subject = matches!(
        filter.stack_kind,
        Some(crate::filter::StackObjectKind::Spell)
    ) || filter.has_mana_cost
        && (filter.cast_by.is_some()
            || matches!(
                filter.zone,
                Some(
                    Zone::Stack
                        | Zone::Hand
                        | Zone::Graveyard
                        | Zone::Exile
                        | Zone::Library
                        | Zone::Command
                )
            ));
    if !is_spell_subject {
        return None;
    }

    let coordinated_card_types = filter
        .has_conjunctive_set_surface()
        .then(|| {
            filter
                .any_of
                .iter()
                .map(|branch| {
                    let [card_type] = branch.card_types.as_slice() else {
                        return None;
                    };
                    (branch == &ObjectFilter::default().with_type(*card_type)).then_some(*card_type)
                })
                .collect::<Option<Vec<_>>>()
        })
        .flatten()
        .filter(|card_types| card_types.len() >= 2);

    // A spell's controller is independent of who cast it (most visibly for
    // spell copies). Preserve the generic description for arbitrary
    // controller-qualified stack filters, but let the exact coordinated
    // card-type shape below keep its shared noun: "instant and sorcery
    // spells you control", not "instants and sorcery spells you control".
    let coordinated_controller_scope =
        matches!(filter.zone, Some(Zone::Stack)) && filter.controller.is_some();
    if coordinated_controller_scope {
        let mut base = filter.clone();
        base.controller = None;
        base.any_of.clear();
        base.set_conjunctive_set_surface(false);
        // The spell noun already carries this semantic distinction; it does
        // not add another surface qualifier to the coordinated type head.
        base.has_mana_cost = false;
        // A color on the outer filter qualifies the shared spell head, not
        // either coordinated card-type arm independently. The specialized
        // path below renders that color before `instant and sorcery spells`;
        // do not force it through the generic arm-by-arm pluralizer.
        base.colors = None;
        if coordinated_card_types.is_none()
            || filter.cast_by.is_some()
            || base != ObjectFilter::spell()
        {
            return Some(pluralized_subject_text(filter));
        }
    }

    let mut qualifiers = Vec::new();
    if filter.nontoken {
        qualifiers.push("nontoken".to_string());
    }
    if filter.colorless {
        qualifiers.push("colorless".to_string());
    }
    if filter.multicolored {
        qualifiers.push("multicolored".to_string());
    }
    if filter.monocolored {
        qualifiers.push("monocolored".to_string());
    }
    if let Some(colors) = filter.colors {
        qualifiers.push(join_with_and(&color_list(colors)));
    }
    if filter.historic {
        qualifiers.push("historic".to_string());
    }
    if filter.nonhistoric {
        qualifiers.push("nonhistoric".to_string());
    }
    if filter.is_commander {
        qualifiers.push("commander".to_string());
    }
    for card_type in &filter.excluded_card_types {
        qualifiers.push(format!("non{}", card_type.name().to_ascii_lowercase()));
    }
    for supertype in &filter.excluded_supertypes {
        qualifiers.push(format!("non{}", supertype.name().to_ascii_lowercase()));
    }
    if !filter.subtypes.is_empty() {
        qualifiers.push(join_with_and(
            &filter
                .subtypes
                .iter()
                .map(std::string::ToString::to_string)
                .collect::<Vec<_>>(),
        ));
    }
    if let Some(card_types) = coordinated_card_types.as_ref() {
        qualifiers.push(join_with_and(
            &card_types
                .iter()
                .map(|card_type| card_type.name().to_ascii_lowercase())
                .collect::<Vec<_>>(),
        ));
    } else if !filter.card_types.is_empty() {
        qualifiers.push(join_with_and(
            &filter
                .card_types
                .iter()
                .map(|card_type| card_type.name().to_ascii_lowercase())
                .collect::<Vec<_>>(),
        ));
    }

    let mut subject = if filter.first_spell_cast_each_turn {
        if qualifiers.is_empty() {
            "the first spell".to_string()
        } else {
            format!("the first {} spell", qualifiers.join(" "))
        }
    } else if qualifiers.is_empty() {
        "spells".to_string()
    } else {
        format!("{} spells", qualifiers.join(" "))
    };

    if let Some(cast_by) = &filter.cast_by {
        match cast_by {
            crate::target::PlayerFilter::You => subject.push_str(" you cast"),
            crate::target::PlayerFilter::Opponent => subject.push_str(" your opponents cast"),
            other => subject.push_str(&format!(" cast by {}", other.description())),
        }
    }
    if coordinated_controller_scope {
        let controller = describe_player_filter(
            filter
                .controller
                .as_ref()
                .expect("controller scope checked above"),
        );
        subject.push(' ');
        subject.push_str(&controller);
        subject.push(' ');
        subject.push_str(if controller == "you" {
            "control"
        } else {
            "controls"
        });
    }

    let zone_suffix = match filter.zone {
        Some(Zone::Hand) => Some(match filter.owner.as_ref() {
            Some(crate::target::PlayerFilter::You) => "from your hand".to_string(),
            Some(owner) => format!("from {} hand", owner.description()),
            None => "from your hand".to_string(),
        }),
        Some(Zone::Graveyard) => Some(match filter.owner.as_ref() {
            Some(crate::target::PlayerFilter::You) => "from your graveyard".to_string(),
            Some(owner) => format!("from {} graveyard", owner.description()),
            None => "from a graveyard".to_string(),
        }),
        Some(Zone::Exile) => Some(match filter.owner.as_ref() {
            Some(crate::target::PlayerFilter::You) => "from exile you own".to_string(),
            Some(owner) => format!("from exile {}", owner.description()),
            None => "from exile".to_string(),
        }),
        Some(Zone::Library) => Some(match filter.owner.as_ref() {
            Some(crate::target::PlayerFilter::You) => "from your library".to_string(),
            Some(owner) => format!("from {} library", owner.description()),
            None => "from a library".to_string(),
        }),
        Some(Zone::Command) => Some("from the command zone".to_string()),
        _ => None,
    };
    if let Some(zone_suffix) = zone_suffix {
        subject.push(' ');
        subject.push_str(&zone_suffix);
    }

    if filter.first_spell_cast_each_turn {
        subject.push_str(" each turn");
    }

    if let Some(source_filter) = &filter.mana_from_source_spent_to_cast {
        subject.push_str(" that mana from ");
        subject.push_str(&with_indefinite_article_unless_present(
            source_filter.description(),
        ));
        subject.push_str(" was spent to cast");
    }

    if let Some(power) = &filter.power {
        subject.push_str(" with power ");
        subject.push_str(&describe_filter_comparison(power));
    }
    if let Some(toughness) = &filter.toughness {
        subject.push_str(" with toughness ");
        subject.push_str(&describe_filter_comparison(toughness));
    }
    if let Some(mana_value) = &filter.mana_value {
        subject.push_str(" with mana value ");
        subject.push_str(&describe_filter_comparison(mana_value));
    }

    Some(subject)
}

fn subject_verb_and_possessive(subject: &str) -> (&'static str, &'static str) {
    let singular = subject.starts_with("enchanted ")
        || subject.starts_with("equipped ")
        || subject.starts_with("this ")
        || subject.starts_with("that ")
        || subject.starts_with("Each ");
    if singular {
        ("is", "its")
    } else {
        ("are", "their")
    }
}

/// Anthem effect: "Creatures you control get +N/+M"
pub use ironsmith_core::{AnthemCountExpression, AnthemValue};

trait AnthemValueRuntimeExt {
    fn evaluate(&self, game: &GameState, source: ObjectId, controller: PlayerId) -> i32;
}

impl AnthemValueRuntimeExt for AnthemValue {
    fn evaluate(&self, game: &GameState, source: ObjectId, controller: PlayerId) -> i32 {
        match self {
            Self::Fixed(value) => *value,
            Self::Dynamic(value) => crate::continuous::resolve_value_direct(
                value,
                game.objects_map(),
                &[],
                &game.battlefield,
                game.commander_objects(),
                source,
                controller,
                game,
            ),
            Self::PerCount { multiplier, count } => {
                multiplier * resolve_anthem_count_expression(count, game, source, controller)
            }
            Self::CappedPerCount {
                multiplier,
                count,
                maximum,
            } => (multiplier * resolve_anthem_count_expression(count, game, source, controller))
                .min(*maximum),
        }
    }
}

fn anthem_value_as_layer_value(value: &AnthemValue) -> Option<Value> {
    match value {
        AnthemValue::Fixed(value) => Some(Value::Fixed(*value)),
        AnthemValue::Dynamic(_) => None,
        AnthemValue::PerCount {
            multiplier,
            count: AnthemCountExpression::MatchingFilter(filter),
        } => Some(if *multiplier == 1 {
            Value::Count(filter.clone())
        } else {
            Value::CountScaled(filter.clone(), *multiplier)
        }),
        AnthemValue::PerCount { .. } => None,
        AnthemValue::CappedPerCount { .. } => None,
    }
}

fn color_count_multiplier(value: &AnthemValue) -> Option<i32> {
    match value {
        AnthemValue::Fixed(0) => Some(0),
        AnthemValue::PerCount {
            multiplier,
            count: AnthemCountExpression::ColorsOfAffected,
        } => Some(*multiplier),
        _ => None,
    }
}

fn strip_article(text: String) -> String {
    if let Some(rest) = text.strip_prefix("a ") {
        return rest.to_string();
    }
    if let Some(rest) = text.strip_prefix("an ") {
        return rest.to_string();
    }
    text
}

/// Bare battlefield filters need an explicit zone when they are used as a
/// count subject. Controller, owner, combat, and attachment predicates already
/// make the battlefield provenance clear.
fn anthem_count_filter_needs_battlefield_surface(filter: &ObjectFilter, subject: &str) -> bool {
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
        && !filter.didnt_enter_battlefield_this_turn
        && !filter.entered_battlefield_this_turn
        && filter.entered_battlefield_controller.is_none()
        && !filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.relation == TaggedOpbjectRelation::IsTaggedObject)
}

fn describe_anthem_for_each_matching_filter(filter: &ObjectFilter) -> String {
    let mut subject = strip_article(filter.description());
    if let Some(rest) = subject.strip_prefix("another ") {
        subject = format!("other {rest}");
    }
    if anthem_count_filter_needs_battlefield_surface(filter, &subject) {
        subject.push_str(" on the battlefield");
    }
    subject
}

fn describe_source_reference_surface(surface: &SourceReferenceSurface) -> String {
    surface.display_text()
}

fn sticker_action_count_noun(action: crate::events::KeywordActionKind) -> &'static str {
    match action {
        crate::events::KeywordActionKind::NameSticker => "name sticker",
        crate::events::KeywordActionKind::ArtSticker => "art sticker",
        crate::events::KeywordActionKind::AbilitySticker => "ability sticker",
        crate::events::KeywordActionKind::PowerToughnessSticker => "power and toughness sticker",
        _ => "sticker",
    }
}

fn describe_sticker_count_subject(
    action: crate::events::KeywordActionKind,
    surface: Option<&SourceReferenceSurface>,
    min_name_letters: Option<u32>,
    max_name_letters: Option<u32>,
) -> String {
    let source_text = surface
        .map(describe_source_reference_surface)
        .unwrap_or_else(|| "this permanent".to_string());
    let mut text = format!("{} on {source_text}", sticker_action_count_noun(action));
    if let Some(min_letters) = min_name_letters {
        let min_letters_text =
            number_word_u32(min_letters).unwrap_or_else(|| min_letters.to_string());
        text.push_str(&format!(" with {min_letters_text} or more letters"));
    }
    if let Some(max_letters) = max_name_letters {
        let max_letters_text =
            number_word_u32(max_letters).unwrap_or_else(|| max_letters.to_string());
        text.push_str(&format!(" with {} or fewer letters", max_letters_text));
    }
    text
}

fn counter_source_location(expr: &AnthemCountExpression) -> Option<(CounterType, String)> {
    match expr {
        AnthemCountExpression::CountersOnSource(counter_type) => {
            Some((*counter_type, "this permanent".to_string()))
        }
        AnthemCountExpression::CountersOnSourceWithSurface {
            counter_type,
            surface,
        } => Some((*counter_type, describe_source_reference_surface(surface))),
        AnthemCountExpression::CountersOnSourceWithPronoun { counter_type, .. } => {
            Some((*counter_type, "this permanent".to_string()))
        }
        AnthemCountExpression::CountersOnAffected(counter_type) => {
            Some((*counter_type, "it".to_string()))
        }
        _ => None,
    }
}

fn matching_counter_source_location(
    left: &AnthemCountExpression,
    right: &AnthemCountExpression,
) -> Option<(CounterType, String)> {
    let (left_counter, left_location) = counter_source_location(left)?;
    let (right_counter, right_location) = counter_source_location(right)?;
    (left_counter == right_counter && left_location == right_location)
        .then_some((left_counter, left_location))
}

fn describe_anthem_count_expression(expr: &AnthemCountExpression) -> String {
    match expr {
        AnthemCountExpression::MatchingFilter(filter) => {
            let mut subject = pluralized_subject_text(filter);
            if filter.owner.is_none()
                && !filter.single_graveyard
                && filter.zone == Some(Zone::Graveyard)
            {
                subject = subject.replace(" in a graveyard", " in all graveyards");
                subject = subject.replace(" in graveyard", " in all graveyards");
                if !subject.contains("graveyard") {
                    subject.push_str(" in all graveyards");
                }
            }
            if anthem_count_filter_needs_battlefield_surface(filter, &subject) {
                subject.push_str(" on the battlefield");
            }
            subject
        }
        AnthemCountExpression::GraveyardsWithAtLeastCards { minimum_cards } => {
            let count =
                number_word_u32(*minimum_cards).unwrap_or_else(|| minimum_cards.to_string());
            format!("graveyards with {count} or more cards in them")
        }
        AnthemCountExpression::GreatestManaValueAmong(filter) => {
            format!(
                "the greatest mana value among {}",
                pluralized_subject_text(filter)
            )
        }
        AnthemCountExpression::AttachedToSource(filter) => {
            format!(
                "{} attached to this creature",
                strip_article(filter.description())
            )
        }
        AnthemCountExpression::AttachedToAffected(filter) => {
            format!("{} attached to it", strip_article(filter.description()))
        }
        AnthemCountExpression::ColorsOfAffected => "color it has".to_string(),
        AnthemCountExpression::AffectedAttackedThisTurn => {
            "time it has attacked this turn".to_string()
        }
        AnthemCountExpression::CountersOnSource(counter_type) => {
            format!("{} counter on this permanent", counter_type.description())
        }
        AnthemCountExpression::CountersOnSourceWithSurface {
            counter_type,
            surface,
        } => {
            format!(
                "{} counter on {}",
                counter_type.description(),
                describe_source_reference_surface(surface)
            )
        }
        AnthemCountExpression::CountersOnSourceWithPronoun { counter_type, .. } => {
            format!("{} counter on this permanent", counter_type.description())
        }
        AnthemCountExpression::StickersOnSource {
            action,
            surface,
            min_name_letters,
            max_name_letters,
        } => describe_sticker_count_subject(
            *action,
            surface.as_ref(),
            *min_name_letters,
            *max_name_letters,
        ),
        AnthemCountExpression::CountersOnAffected(counter_type) => {
            format!("{} counter on it", counter_type.description())
        }
        AnthemCountExpression::CountersAmong(filter, counter_type) => format!(
            "{} counter on {}",
            counter_type.description(),
            pluralized_subject_text(filter)
        ),
        AnthemCountExpression::DistinctCounterTypesAmong(filter) => format!(
            "different kind of counter among {}",
            pluralized_subject_text(filter)
        ),
        AnthemCountExpression::BasicLandTypesAmong(_) => {
            "basic land type among lands you control".to_string()
        }
        AnthemCountExpression::CreatureTypesAmong(filter) if filter == &ObjectFilter::source() => {
            "creature type it has".to_string()
        }
        AnthemCountExpression::CreatureTypesAmong(filter) => {
            format!("creature type among {}", pluralized_subject_text(filter))
        }
        AnthemCountExpression::BlockingSource => "creature blocking it".to_string(),
        AnthemCountExpression::CommanderCastCount(player) => match player {
            crate::target::PlayerFilter::You => {
                "times you've cast your commander from the command zone this game".to_string()
            }
            crate::target::PlayerFilter::Opponent => {
                "times an opponent has cast their commander from the command zone this game"
                    .to_string()
            }
            crate::target::PlayerFilter::Any => {
                "times a player has cast their commander from the command zone this game"
                    .to_string()
            }
            other => format!(
                "times {} has cast their commander from the command zone this game",
                other.description()
            ),
        },
        AnthemCountExpression::PlayerSpeed(player) => match player {
            crate::target::PlayerFilter::You => "your speed".to_string(),
            crate::target::PlayerFilter::Opponent => "an opponent's speed".to_string(),
            crate::target::PlayerFilter::Any => "a player's speed".to_string(),
            _ => "that player's speed".to_string(),
        },
        AnthemCountExpression::UnspentMana { player, symbol } => {
            format!(
                "{} unspent {} mana",
                player.description(),
                mana_symbol_word(*symbol)
            )
        }
    }
}

fn describe_anthem_for_each_count_expression(expr: &AnthemCountExpression) -> Option<String> {
    if let AnthemCountExpression::MatchingFilter(filter) = expr
        && filter.zone == Some(Zone::Hand)
        && matches!(filter.owner.as_ref(), Some(PlayerFilter::You))
    {
        return Some("card in your hand".to_string());
    }

    if let AnthemCountExpression::MatchingFilter(filter) = expr
        && filter.zone == Some(Zone::Exile)
        && matches!(filter.owner.as_ref(), Some(PlayerFilter::Opponent))
    {
        let mut card_filter = filter.clone();
        card_filter.zone = None;
        card_filter.owner = None;
        card_filter.set_explicit_card_noun(true);
        return Some(format!(
            "{} your opponents own in exile",
            strip_article(card_filter.description())
        ));
    }

    if let AnthemCountExpression::MatchingFilter(filter) = expr
        && filter.zone == Some(Zone::Hand)
        && matches!(
            filter.owner.as_ref(),
            Some(PlayerFilter::ControllerOf(ObjectRef::Target))
        )
    {
        return Some("card in its controller's hand".to_string());
    }

    if let AnthemCountExpression::MatchingFilter(filter) = expr
        && filter.has_conjunctive_set_surface()
        && filter.any_of.len() >= 2
    {
        // Conjunctive count domains are additive sets, not alternatives:
        // "for each card in your hand and each foretold card you own in
        // exile." Keep shared owner/controller scope on each typed branch
        // while restoring the repeated distributive quantifier.
        let mut shared_scope = filter.clone();
        shared_scope.any_of.clear();
        shared_scope.set_conjunctive_set_surface(false);
        let supported_scope = ObjectFilter {
            controller: filter.controller.clone(),
            owner: filter.owner.clone(),
            ..ObjectFilter::default()
        };
        if shared_scope == supported_scope {
            let parts = filter
                .any_of
                .iter()
                .map(|branch| {
                    let mut described = branch.clone();
                    if described.controller.is_none() {
                        described.controller = filter.controller.clone();
                    }
                    if described.owner.is_none() {
                        described.owner = filter.owner.clone();
                    }
                    strip_article(described.description())
                })
                .collect::<Vec<_>>();
            let mut iter = parts.into_iter();
            let first = iter.next()?;
            let rest = iter
                .map(|part| format!("each {part}"))
                .collect::<Vec<_>>()
                .join(" and ");
            if !rest.is_empty() {
                return Some(format!("{first} and {rest}"));
            }
        }
    }

    if let AnthemCountExpression::MatchingFilter(filter) = expr
        && !filter.any_of.is_empty()
        && filter
            .any_of
            .iter()
            .any(|branch| branch.zone == Some(Zone::Graveyard))
    {
        let parts: Vec<String> = filter
            .any_of
            .iter()
            .map(|branch| {
                let mut text = strip_article(branch.description());
                if let Some(rest) = text.strip_prefix("another ") {
                    text = format!("other {rest}");
                }
                text
            })
            .collect();
        let mut iter = parts.into_iter();
        let first = iter.next()?;
        let rest = iter
            .map(|part| format!("each {part}"))
            .collect::<Vec<_>>()
            .join(" and ");
        if rest.is_empty() {
            return Some(first);
        }
        return Some(format!("{first} and {rest}"));
    }

    match expr {
        AnthemCountExpression::MatchingFilter(filter) if filter.zone == Some(Zone::Battlefield) => {
            Some(describe_anthem_for_each_matching_filter(filter))
        }
        AnthemCountExpression::GraveyardsWithAtLeastCards { minimum_cards } => {
            let count =
                number_word_u32(*minimum_cards).unwrap_or_else(|| minimum_cards.to_string());
            Some(format!("graveyard with {count} or more cards in it"))
        }
        AnthemCountExpression::MatchingFilter(filter)
            if filter.zone == Some(Zone::Graveyard)
                && filter.name.is_some()
                && !filter.single_graveyard =>
        {
            let name = filter.name.as_deref().expect("checked above");
            Some(format!("card named {name} in each graveyard"))
        }
        AnthemCountExpression::AttachedToAffected(filter) => Some(format!(
            "{} attached to it",
            strip_article(filter.description())
        )),
        AnthemCountExpression::AttachedToSource(filter) => Some(format!(
            "{} attached to it",
            strip_article(filter.description())
        )),
        AnthemCountExpression::ColorsOfAffected => Some("of its colors".to_string()),
        AnthemCountExpression::AffectedAttackedThisTurn => {
            Some("time it has attacked this turn".to_string())
        }
        AnthemCountExpression::CountersOnSource(counter_type) => {
            Some(format!("{} counter on it", counter_type.description()))
        }
        AnthemCountExpression::CountersOnSourceWithSurface {
            counter_type,
            surface,
        } => Some(format!(
            "{} counter on {}",
            counter_type.description(),
            describe_source_reference_surface(surface)
        )),
        AnthemCountExpression::CountersOnSourceWithPronoun {
            counter_type,
            pronoun,
        } => Some(format!(
            "{} counter on {}",
            counter_type.description(),
            pronoun.object_pronoun()
        )),
        AnthemCountExpression::StickersOnSource {
            action,
            surface,
            min_name_letters,
            max_name_letters,
        } => Some(describe_sticker_count_subject(
            *action,
            surface.as_ref(),
            *min_name_letters,
            *max_name_letters,
        )),
        AnthemCountExpression::CountersOnAffected(counter_type) => {
            Some(format!("{} counter on it", counter_type.description()))
        }
        AnthemCountExpression::CountersAmong(filter, counter_type) => Some(format!(
            "{} counter on {}",
            counter_type.description(),
            pluralized_subject_text(filter)
        )),
        AnthemCountExpression::DistinctCounterTypesAmong(filter) => Some(format!(
            "different kind of counter among {}",
            pluralized_subject_text(filter)
        )),
        AnthemCountExpression::BasicLandTypesAmong(_) => {
            Some("basic land type among lands you control".to_string())
        }
        AnthemCountExpression::CreatureTypesAmong(filter) if filter == &ObjectFilter::source() => {
            Some("of its creature types".to_string())
        }
        AnthemCountExpression::CreatureTypesAmong(filter) => Some(format!(
            "creature type among {}",
            pluralized_subject_text(filter)
        )),
        AnthemCountExpression::BlockingSource => Some("creature blocking it".to_string()),
        AnthemCountExpression::CommanderCastCount(player) => Some(match player {
            crate::target::PlayerFilter::You => {
                "time you've cast your commander from the command zone this game".to_string()
            }
            crate::target::PlayerFilter::Opponent => {
                "time an opponent has cast their commander from the command zone this game"
                    .to_string()
            }
            crate::target::PlayerFilter::Any => {
                "time a player has cast their commander from the command zone this game".to_string()
            }
            other => format!(
                "time {} has cast their commander from the command zone this game",
                other.description()
            ),
        }),
        AnthemCountExpression::UnspentMana { player, symbol } => Some(format!(
            "unspent {} mana {} have",
            mana_symbol_word(*symbol),
            player.description()
        )),
        _ => None,
    }
}

/// "for each" subject for counts the primary helper leaves unhandled but which
/// still read naturally with a "for each" surface — currently cards matched in a
/// graveyard (e.g. "for each Lesson card in your graveyard"). Only consulted when
/// the original oracle text did not use a "where X is" clause.
fn describe_anthem_for_each_graveyard_count_expression(
    expr: &AnthemCountExpression,
) -> Option<String> {
    match expr {
        AnthemCountExpression::MatchingFilter(filter) if filter.zone == Some(Zone::Graveyard) => {
            Some(strip_article(filter.description()))
        }
        _ => None,
    }
}

fn describe_anthem_where_x_count_expression(expr: &AnthemCountExpression) -> String {
    match expr {
        AnthemCountExpression::GreatestManaValueAmong(filter) => {
            format!(
                "the greatest mana value among {}",
                pluralized_subject_text(filter)
            )
        }
        AnthemCountExpression::BlockingSource => "the number of creatures blocking it".to_string(),
        _ => format!("the number of {}", describe_anthem_count_expression(expr)),
    }
}

fn mana_symbol_word(symbol: crate::mana::ManaSymbol) -> &'static str {
    match symbol {
        crate::mana::ManaSymbol::White => "white",
        crate::mana::ManaSymbol::Blue => "blue",
        crate::mana::ManaSymbol::Black => "black",
        crate::mana::ManaSymbol::Red => "red",
        crate::mana::ManaSymbol::Green => "green",
        crate::mana::ManaSymbol::Colorless => "colorless",
        _ => "mana",
    }
}

fn comparison_display(cmp: &Comparison) -> String {
    match cmp {
        Comparison::GreaterThan(n) => format!("more than {n}"),
        Comparison::GreaterThanOrEqual(n) => format!("{n} or more"),
        Comparison::Equal(n) => n.to_string(),
        Comparison::LessThan(n) => format!("less than {n}"),
        Comparison::LessThanOrEqual(0) => "no".to_string(),
        Comparison::LessThanOrEqual(n) => format!("{n} or less"),
        Comparison::NotEqual(n) => format!("not {n}"),
        Comparison::OneOf(values) => values
            .iter()
            .map(i32::to_string)
            .collect::<Vec<_>>()
            .join(" or "),
        Comparison::BetweenInclusive(min, max) => format!("{min} to {max}"),
    }
}

fn describe_static_condition_value(value: &Value) -> String {
    match value {
        Value::Fixed(n) => number_word_u32(*n as u32).unwrap_or_else(|| n.to_string()),
        Value::Add(left, right)
            if matches!(left.as_ref(), Value::Devotion { .. })
                && matches!(right.as_ref(), Value::Devotion { .. }) =>
        {
            let (
                Value::Devotion {
                    player: left_player,
                    color: left_color,
                },
                Value::Devotion {
                    player: right_player,
                    color: right_color,
                },
            ) = (left.as_ref(), right.as_ref())
            else {
                unreachable!();
            };
            if left_player == right_player {
                return format!(
                    "{} devotion to {} and {}",
                    describe_static_possessive_player(left_player),
                    left_color.name(),
                    right_color.name()
                );
            }
            format!(
                "{} plus {}",
                describe_static_condition_value(left),
                describe_static_condition_value(right)
            )
        }
        Value::Devotion { player, color } => {
            format!(
                "{} devotion to {}",
                describe_static_possessive_player(player),
                color.name()
            )
        }
        Value::Add(left, right) => format!(
            "{} plus {}",
            describe_static_condition_value(left),
            describe_static_condition_value(right)
        ),
        other => format!("{other:?}"),
    }
}

fn describe_attached_subject_static_condition(
    condition: &crate::ConditionExpr,
    subject: &str,
) -> Option<String> {
    let attached_subject = matches!(
        subject,
        "enchanted artifact"
            | "enchanted creature"
            | "enchanted land"
            | "enchanted permanent"
            | "equipped creature"
            | "fortified land"
    );
    let (filter, negative, use_pronoun) = match condition {
        crate::ConditionExpr::AttachedToSourceMatches(filter) => (filter, false, false),
        crate::ConditionExpr::TargetMatches(filter) if attached_subject => (filter, false, true),
        crate::ConditionExpr::Not(inner) => match inner.as_ref() {
            crate::ConditionExpr::AttachedToSourceMatches(filter) => (filter, true, false),
            crate::ConditionExpr::TargetMatches(filter) if attached_subject => (filter, true, true),
            _ => return None,
        },
        _ => return None,
    };
    let subject = if use_pronoun { "it" } else { subject };
    let verb = if negative {
        "isn't"
    } else if use_pronoun {
        "'s"
    } else {
        "is"
    };
    // A bare color (or supertype) is an adjective predicate in oracle ("as
    // long as enchanted creature is white"), not a classified noun.
    if let Some(colors) = filter.colors {
        let mut bare = filter.clone();
        bare.colors = None;
        if bare == ObjectFilter::default() {
            let color_words = crate::color::Color::ALL
                .into_iter()
                .filter(|color| colors.contains(*color))
                .map(|color| color.name().to_ascii_lowercase())
                .collect::<Vec<_>>();
            if !color_words.is_empty() {
                let separator = if use_pronoun && !negative { "" } else { " " };
                return Some(format!(
                    "as long as {subject}{separator}{verb} {}",
                    color_words.join(" and ")
                ));
            }
        }
    }
    if let [supertype] = filter.supertypes.as_slice() {
        let mut bare = filter.clone();
        bare.supertypes.clear();
        if bare == ObjectFilter::default() {
            let separator = if use_pronoun && !negative { "" } else { " " };
            return Some(format!(
                "as long as {subject}{separator}{verb} {}",
                supertype.name().to_ascii_lowercase()
            ));
        }
    }
    let descriptor = strip_article(filter.description());
    let article = indefinite_article_for(&descriptor);
    let separator = if use_pronoun && !negative { "" } else { " " };
    Some(format!(
        "as long as {subject}{separator}{verb} {article} {descriptor}"
    ))
}

fn flatten_static_condition_and(
    condition: &crate::ConditionExpr,
    out: &mut Vec<crate::ConditionExpr>,
) {
    match condition {
        crate::ConditionExpr::And(left, right) => {
            flatten_static_condition_and(left, out);
            flatten_static_condition_and(right, out);
        }
        _ => out.push(condition.clone()),
    }
}

fn describe_source_keyword_condition(filter: &ObjectFilter) -> Option<String> {
    if filter.static_abilities.len() + filter.ability_markers.len() != 1 {
        return None;
    }
    let mut unqualified = filter.clone();
    unqualified.static_abilities.clear();
    unqualified.ability_markers.clear();
    if unqualified != ObjectFilter::default() {
        return None;
    }
    let description = filter.description();
    let keyword = description.strip_prefix("permanent with ")?;
    (!keyword.is_empty()).then(|| format!("as long as it has {keyword}"))
}

pub(super) fn describe_static_condition(condition: &crate::ConditionExpr) -> String {
    if source_is_attacking_alone_condition(condition) {
        return "as long as this creature is attacking alone".to_string();
    }
    if let Some(described) = describe_attached_subject_static_condition(
        condition,
        "the permanent this source is attached to",
    ) {
        return described;
    }
    match condition {
        crate::ConditionExpr::And(_, _) => {
            let mut clauses = Vec::new();
            flatten_static_condition_and(condition, &mut clauses);
            let described = clauses
                .iter()
                .map(describe_static_condition)
                .collect::<Vec<_>>();
            if described
                .iter()
                .all(|clause| clause.starts_with("as long as "))
            {
                let joined = described
                    .iter()
                    .map(|clause| clause.trim_start_matches("as long as "))
                    .collect::<Vec<_>>()
                    .join(" and ");
                return format!("as long as {joined}");
            }
            described.join(" and ")
        }
        crate::ConditionExpr::ThisSpellWasKicked => "as long as this spell was kicked".to_string(),
        crate::ConditionExpr::XValueAtLeast(amount) => {
            format!("as long as X is {amount} or more")
        }
        crate::ConditionExpr::YourTurn => "as long as it's your turn".to_string(),
        crate::ConditionExpr::CurrentTurnIsExtra => "during extra turns".to_string(),
        crate::ConditionExpr::Not(inner)
            if matches!(inner.as_ref(), crate::ConditionExpr::YourTurn) =>
        {
            "during turns other than yours".to_string()
        }
        crate::ConditionExpr::SourceMatches(filter) => describe_source_keyword_condition(filter)
            .unwrap_or_else(|| {
                format!(
                    "as long as {}",
                    crate::runtime_display::describe_condition(condition)
                )
            }),
        crate::ConditionExpr::AttachmentCount { display, .. } => {
            format!("as long as {display}")
        }
        crate::ConditionExpr::SourceIsEquipped => {
            "as long as this creature is equipped".to_string()
        }
        crate::ConditionExpr::SourceIsEnchanted => {
            "as long as this creature is enchanted".to_string()
        }
        crate::ConditionExpr::SourceIsMonstrous => {
            "as long as this creature is monstrous".to_string()
        }
        crate::ConditionExpr::SourceCrewedByExactly { count, filter } => {
            let count_text =
                ironsmith_core::cardinal_word(*count).unwrap_or_else(|| count.to_string());
            let filter_text = if *count == 1 {
                filter.description()
            } else {
                let desc = filter.description();
                if desc.ends_with('s') {
                    desc
                } else {
                    format!("{desc}s")
                }
            };
            format!("if it was crewed by exactly {count_text} {filter_text}")
        }
        crate::ConditionExpr::SourceDevouredCreaturesOrMore(count) => {
            if *count == 1 {
                "if it devoured a creature".to_string()
            } else {
                format!("if it devoured {count} or more creatures")
            }
        }
        crate::ConditionExpr::SourceFirstCrewedThisTurn => {
            "if it was crewed for the first time this turn".to_string()
        }
        crate::ConditionExpr::EnchantedPermanentIsCreature => {
            "as long as enchanted permanent is a creature".to_string()
        }
        crate::ConditionExpr::EnchantedPermanentIsLand => {
            "as long as enchanted permanent is a land".to_string()
        }
        crate::ConditionExpr::EnchantedPermanentIsEquipment => {
            "as long as enchanted permanent is an equipment".to_string()
        }
        crate::ConditionExpr::EnchantedPermanentIsVehicle => {
            "as long as enchanted permanent is a vehicle".to_string()
        }
        crate::ConditionExpr::EquippedCreatureTapped => {
            "as long as equipped creature is tapped".to_string()
        }
        crate::ConditionExpr::EquippedCreatureUntapped => {
            "as long as equipped creature is untapped".to_string()
        }
        crate::ConditionExpr::EquippedCreatureAttacking => {
            "as long as equipped creature is attacking".to_string()
        }
        crate::ConditionExpr::TaggedObjectMatches(tag, filter)
            if tag.as_str() == "equipped"
                && filter.subtypes.len() == 1
                && filter == &ObjectFilter::default().with_subtype(filter.subtypes[0]) =>
        {
            let subtype = format!("{:?}", filter.subtypes[0]).to_ascii_lowercase();
            format!("as long as equipped creature is a {subtype}")
        }
        crate::ConditionExpr::SourceChosenOption(option) => {
            format!("as long as the chosen option is {}", option)
        }
        crate::ConditionExpr::SourceIsAttacking => {
            "as long as this creature is attacking".to_string()
        }
        crate::ConditionExpr::SourceAttackedThisTurn => {
            "as long as this creature attacked this turn".to_string()
        }
        crate::ConditionExpr::SourceAttackedBattleThisTurn => {
            "as long as this creature attacked a battle this turn".to_string()
        }
        crate::ConditionExpr::OpponentLostLifeThisTurn => {
            "as long as an opponent lost life this turn".to_string()
        }
        crate::ConditionExpr::AnyPlayerLostLifeThisTurnOrMore { count } => {
            format!("as long as a player lost {count} or more life this turn")
        }
        crate::ConditionExpr::PlayerWasDealtCombatDamageByCreatureSubtypeThisTurn {
            player,
            subtype,
        } => {
            let player_text = match player {
                PlayerFilter::Any => "a player".to_string(),
                PlayerFilter::Opponent => "an opponent".to_string(),
                _ => describe_player_filter(player),
            };
            format!("as long as {player_text} was dealt combat damage by a {subtype} this turn")
        }
        crate::ConditionExpr::SourceCameUnderYourControlThisTurn => {
            "as long as this creature came under your control this turn".to_string()
        }
        crate::ConditionExpr::Not(inner)
            if matches!(inner.as_ref(), crate::ConditionExpr::SourceAttackedThisTurn) =>
        {
            "as long as this creature didn't attack this turn".to_string()
        }
        crate::ConditionExpr::Not(inner) => {
            if let crate::ConditionExpr::PlayerIsMonarch { player } = inner.as_ref() {
                return match player {
                    crate::target::PlayerFilter::You => "unless you're the monarch".to_string(),
                    crate::target::PlayerFilter::Opponent => {
                        "unless an opponent is the monarch".to_string()
                    }
                    crate::target::PlayerFilter::Any => {
                        "unless a player is the monarch".to_string()
                    }
                    crate::target::PlayerFilter::NotYou => {
                        "unless another player is the monarch".to_string()
                    }
                    crate::target::PlayerFilter::Defending => {
                        "unless the defending player is the monarch".to_string()
                    }
                    crate::target::PlayerFilter::Attacking => {
                        "unless the attacking player is the monarch".to_string()
                    }
                    crate::target::PlayerFilter::IteratedPlayer => {
                        "unless that player is the monarch".to_string()
                    }
                    _ => format!("unless {} is the monarch", describe_player_filter(player)),
                };
            }
            if let crate::ConditionExpr::PlayerCastSpellsThisTurnOrMore { player, count: 1 } =
                inner.as_ref()
            {
                return match player {
                    crate::target::PlayerFilter::You => {
                        "as long as you haven't cast a spell this turn".to_string()
                    }
                    crate::target::PlayerFilter::Opponent => {
                        "as long as an opponent hasn't cast a spell this turn".to_string()
                    }
                    crate::target::PlayerFilter::Any => {
                        "as long as no player has cast a spell this turn".to_string()
                    }
                    _ => "as long as that player hasn't cast a spell this turn".to_string(),
                };
            }
            if let crate::ConditionExpr::CountComparison { display, .. } = inner.as_ref() {
                let condition_text = display
                    .clone()
                    .unwrap_or_else(|| {
                        describe_static_condition(inner).replacen("as long as ", "", 1)
                    })
                    .replace(" dont ", " don't ");
                return format!("unless {condition_text}");
            }
            format!("as long as not ({})", describe_static_condition(inner))
        }
        crate::ConditionExpr::PlayerCastSpellsThisTurnOrMore { player, count } => {
            let subject = describe_static_player(player);
            let count_text = number_word_u32(*count)
                .unwrap_or_else(|| count.to_string());
            let verb = if matches!(player, crate::target::PlayerFilter::You) {
                "have"
            } else {
                "has"
            };
            format!("as long as {subject} {verb} cast {count_text} or more spells this turn")
        }
        crate::ConditionExpr::SourceIsTapped => "as long as this creature is tapped".to_string(),
        crate::ConditionExpr::SourceIsUntapped => {
            "as long as this creature is untapped".to_string()
        }
        crate::ConditionExpr::SourceIsSoulbondPaired => {
            "as long as this creature is paired with another creature".to_string()
        }
        crate::ConditionExpr::PlayerHasCardTypesInGraveyardOrMore { player, count } => {
            let graveyard_owner = match player {
                crate::target::PlayerFilter::You => "your".to_string(),
                crate::target::PlayerFilter::Opponent => "an opponent's".to_string(),
                crate::target::PlayerFilter::Any => "a player's".to_string(),
                _ => "that player's".to_string(),
            };
            format!(
                "as long as there are {count} or more card types among cards in {graveyard_owner} graveyard"
            )
        }
        crate::ConditionExpr::PlayerCardsInHandOrMore { player, count } => {
            let subject = match player {
                crate::target::PlayerFilter::You => "you",
                crate::target::PlayerFilter::Opponent => "an opponent",
                crate::target::PlayerFilter::Any => "a player",
                _ => "that player",
            };
            let verb = if *player == crate::target::PlayerFilter::You {
                "have"
            } else {
                "has"
            };
            let count_text = u32::try_from(*count)
                .ok()
                .and_then(number_word_u32)
                .unwrap_or_else(|| count.to_string());
            format!("as long as {subject} {verb} {count_text} or more cards in hand")
        }
        crate::ConditionExpr::PlayerCardsInHandOrFewer { player, count } => {
            let subject = match player {
                crate::target::PlayerFilter::You => "you",
                crate::target::PlayerFilter::Opponent => "an opponent",
                crate::target::PlayerFilter::Any => "a player",
                _ => "that player",
            };
            let verb = if *player == crate::target::PlayerFilter::You {
                "have"
            } else {
                "has"
            };
            let count_text = match count {
                0 => "no".to_string(),
                1 => "one".to_string(),
                _ => count.to_string(),
            };
            if *count == 0 {
                return format!("as long as {subject} {verb} no cards in hand");
            }
            format!("as long as {subject} {verb} {count_text} or fewer cards in hand")
        }
        crate::ConditionExpr::PlayerHasAtLeast {
            player,
            filter,
            count,
        } => {
            let subject = match player {
                crate::target::PlayerFilter::You => "you",
                crate::target::PlayerFilter::Opponent => "an opponent",
                crate::target::PlayerFilter::Any => "a player",
                crate::target::PlayerFilter::NotYou => "another player",
                crate::target::PlayerFilter::Teammate => "a teammate",
                crate::target::PlayerFilter::Active => "the active player",
                crate::target::PlayerFilter::Defending => "the defending player",
                crate::target::PlayerFilter::Attacking => "the attacking player",
                crate::target::PlayerFilter::DamagedPlayer => "the damaged player",
                _ => "that player",
            };
            let count_text = number_word_u32(*count).unwrap_or_else(|| count.to_string());
            let mut described_filter = filter.clone();
            if described_filter.controller.as_ref() == Some(player) {
                described_filter.controller = None;
            }
            let object_text = pluralized_subject_text(&described_filter);
            let verb = if matches!(player, crate::target::PlayerFilter::You) {
                "control"
            } else {
                "controls"
            };
            format!("as long as {subject} {verb} {count_text} or more {object_text}")
        }
        crate::ConditionExpr::PlayerLifeAtMostHalfStartingLifeTotal { player } => {
            let subject = match player {
                crate::target::PlayerFilter::You => "your".to_string(),
                _ => format!("{}'s", describe_static_player(player)),
            };
            format!(
                "as long as {subject} life total is less than or equal to half {} starting life total",
                describe_static_possessive_player(player)
            )
        }
        crate::ConditionExpr::CountComparison {
            count,
            comparison,
            display,
        } => {
            if let Some(display) = display {
                return format!("as long as {}", display.replace(" dont ", " don't "));
            }
            format!(
                "as long as there are {} {}",
                comparison_display(comparison),
                describe_anthem_count_expression(count)
            )
        }
        crate::ConditionExpr::Or(left, right) => {
            let left_text = describe_static_condition(left).replacen("as long as ", "", 1);
            let right_text = describe_static_condition(right).replacen("as long as ", "", 1);
            format!("as long as {left_text} or {right_text}")
        }
        crate::ConditionExpr::ValueComparison {
            left: Value::Speed(crate::target::PlayerFilter::You),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(4),
        } => "as long as you have max speed".to_string(),
        crate::ConditionExpr::ValueComparison {
            left:
                Value::SpellsCastThisTurnMatching {
                    exclude_source: false,
                    ..
                },
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(count),
        } if *count >= 1 => format!(
            "as long as {}",
            crate::runtime_display::describe_condition(condition)
        ),
        crate::ConditionExpr::ValueComparison {
            left: Value::PlayerCounters(player, counter_type),
            operator,
            right: Value::Fixed(count),
        } if *count >= 0 => {
            let count_text = number_word_u32(*count as u32).unwrap_or_else(|| count.to_string());
            let comparison = match operator {
                crate::effect::ValueComparisonOperator::GreaterThan => {
                    format!("more than {count_text}")
                }
                crate::effect::ValueComparisonOperator::GreaterThanOrEqual => {
                    format!("{count_text} or more")
                }
                crate::effect::ValueComparisonOperator::Equal => {
                    format!("exactly {count_text}")
                }
                crate::effect::ValueComparisonOperator::LessThan => {
                    format!("fewer than {count_text}")
                }
                crate::effect::ValueComparisonOperator::LessThanOrEqual => {
                    format!("{count_text} or fewer")
                }
                crate::effect::ValueComparisonOperator::NotEqual => {
                    format!("not exactly {count_text}")
                }
            };
            let subject = describe_static_player(player);
            let verb = if matches!(player, crate::target::PlayerFilter::You) {
                "have"
            } else {
                "has"
            };
            format!(
                "as long as {subject} {verb} {comparison} {} counters",
                counter_type.description()
            )
        }
        crate::ConditionExpr::ValueComparison {
            left: Value::CardsInGraveyard(player),
            operator,
            right: Value::Fixed(count),
        } if *count >= 0 => {
            let count_text = number_word_u32(*count as u32).unwrap_or_else(|| count.to_string());
            let comparison = match operator {
                crate::effect::ValueComparisonOperator::GreaterThan => {
                    format!("more than {count_text}")
                }
                crate::effect::ValueComparisonOperator::GreaterThanOrEqual => {
                    format!("{count_text} or more")
                }
                crate::effect::ValueComparisonOperator::Equal => {
                    format!("exactly {count_text}")
                }
                crate::effect::ValueComparisonOperator::LessThan => {
                    format!("fewer than {count_text}")
                }
                crate::effect::ValueComparisonOperator::LessThanOrEqual => {
                    format!("{count_text} or fewer")
                }
                crate::effect::ValueComparisonOperator::NotEqual => {
                    format!("not exactly {count_text}")
                }
            };
            format!(
                "as long as there are {comparison} cards in {} graveyard",
                describe_static_possessive_player(player)
            )
        }
        crate::ConditionExpr::ValueComparison {
            left: Value::MaxCardsDrawnThisTurn(player),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(count),
        } if *count >= 0 => {
            let count_text = number_word_u32(*count as u32).unwrap_or_else(|| count.to_string());
            let subject = match player {
                crate::target::PlayerFilter::You => "you've".to_string(),
                crate::target::PlayerFilter::Opponent | crate::target::PlayerFilter::NotYou => {
                    "an opponent has".to_string()
                }
                crate::target::PlayerFilter::Any => "a player has".to_string(),
                _ => format!("{} has", describe_static_player(player)),
            };
            format!("as long as {subject} drawn {count_text} or more cards this turn")
        }
        crate::ConditionExpr::ValueComparison {
            left: Value::MaxDiceRolledThisTurn(player),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(count),
        } if *count >= 0 => {
            let count_text = number_word_u32(*count as u32).unwrap_or_else(|| count.to_string());
            let subject = match player {
                crate::target::PlayerFilter::You => "you've".to_string(),
                crate::target::PlayerFilter::Opponent | crate::target::PlayerFilter::NotYou => {
                    "an opponent has".to_string()
                }
                crate::target::PlayerFilter::Any => "a player has".to_string(),
                _ => format!("{} has", describe_static_player(player)),
            };
            format!("as long as {subject} rolled {count_text} or more dice this turn")
        }
        crate::ConditionExpr::ValueComparison {
            left,
            operator,
            right,
        } => {
            if let (
                crate::effect::Value::LifeTotal(player),
                crate::effect::ValueComparisonOperator::LessThanOrEqual,
                crate::effect::Value::Fixed(threshold),
            ) = (left, operator, right)
            {
                let subject = describe_static_player(player);
                let verb = if subject == "you" { "have" } else { "has" };
                return format!("as long as {subject} {verb} {threshold} or less life");
            }
            if let (
                crate::effect::Value::LifeTotal(player),
                crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                crate::effect::Value::Fixed(threshold),
            ) = (left, operator, right)
            {
                let subject = describe_static_player(player);
                let verb = if subject == "you" { "have" } else { "has" };
                return format!("as long as {subject} {verb} {threshold} or more life");
            }
            let operator_text = match operator {
                crate::effect::ValueComparisonOperator::GreaterThan => "is greater than",
                crate::effect::ValueComparisonOperator::GreaterThanOrEqual => {
                    "is greater than or equal to"
                }
                crate::effect::ValueComparisonOperator::Equal => "is equal to",
                crate::effect::ValueComparisonOperator::LessThan => "is less than",
                crate::effect::ValueComparisonOperator::LessThanOrEqual => {
                    "is less than or equal to"
                }
                crate::effect::ValueComparisonOperator::NotEqual => "is not equal to",
            };
            format!(
                "as long as {} {} {}",
                describe_static_condition_value(left),
                operator_text,
                describe_static_condition_value(right)
            )
        }
        crate::ConditionExpr::PlayerIsMonarch { player } => match player {
            crate::target::PlayerFilter::You => "as long as you're the monarch".to_string(),
            crate::target::PlayerFilter::Opponent => {
                "as long as an opponent is the monarch".to_string()
            }
            crate::target::PlayerFilter::Any => "as long as a player is the monarch".to_string(),
            crate::target::PlayerFilter::NotYou => {
                "as long as another player is the monarch".to_string()
            }
            crate::target::PlayerFilter::Teammate => {
                "as long as a teammate is the monarch".to_string()
            }
            crate::target::PlayerFilter::PlayerToYourLeft => {
                "as long as the player to your left is the monarch".to_string()
            }
            crate::target::PlayerFilter::PlayerToYourRight => {
                "as long as the player to your right is the monarch".to_string()
            }
            crate::target::PlayerFilter::Active => {
                "as long as the active player is the monarch".to_string()
            }
            crate::target::PlayerFilter::Specific(_) => {
                "as long as that player is the monarch".to_string()
            }
            crate::target::PlayerFilter::Defending => {
                "as long as the defending player is the monarch".to_string()
            }
            crate::target::PlayerFilter::Attacking => {
                "as long as the attacking player is the monarch".to_string()
            }
            crate::target::PlayerFilter::DamagedPlayer => {
                "as long as the damaged player is the monarch".to_string()
            }
            crate::target::PlayerFilter::EffectController => {
                "as long as that effect's controller is the monarch".to_string()
            }
            crate::target::PlayerFilter::MostLifeTied => {
                "as long as the player with the most life is the monarch".to_string()
            }
            crate::target::PlayerFilter::LowestLifeTied => {
                "as long as the player with the lowest life is the monarch".to_string()
            }
            crate::target::PlayerFilter::MostCardsInHand => {
                "as long as the player who has the most cards in hand is the monarch".to_string()
            }
            crate::target::PlayerFilter::CardsInHandAtLeastMoreThanYou { .. } => {
                "as long as that player is the monarch".to_string()
            }
            crate::target::PlayerFilter::HasMoreLifeThanYou { .. } => {
                "as long as that player is the monarch".to_string()
            }
            crate::target::PlayerFilter::OpponentWithMoreControlledObjectsThan { .. } => {
                "as long as that player is the monarch".to_string()
            }
            crate::target::PlayerFilter::ControlsMost { .. } => {
                "as long as that player is the monarch".to_string()
            }
            crate::target::PlayerFilter::MaxSpeed { .. } => {
                "as long as that player is the monarch".to_string()
            }
            crate::target::PlayerFilter::CastCardTypeThisTurn(_) => {
                "as long as that player is the monarch".to_string()
            }
            crate::target::PlayerFilter::AttackedBySourceThisTurn => {
                "as long as a player this creature attacked this turn is the monarch".to_string()
            }
            crate::target::PlayerFilter::WasDealtDamageBySourceThisGame { .. } => {
                "as long as a player this source has dealt damage to this game is the monarch"
                    .to_string()
            }
            crate::target::PlayerFilter::WasDealtCombatDamageBySourcesThisGame { .. } => {
                "as long as that player was dealt combat damage this game by a matching source"
                    .to_string()
            }
            crate::target::PlayerFilter::LostLifeThisTurn { .. } => {
                "as long as a player who lost life this turn is the monarch".to_string()
            }
            filter @ crate::target::PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn {
                ..
            } => format!("as long as {} is the monarch", filter.description()),
            crate::target::PlayerFilter::ChosenPlayer => {
                "as long as the chosen player is the monarch".to_string()
            }
            crate::target::PlayerFilter::TaggedPlayer(_) => {
                "as long as that player is the monarch".to_string()
            }
            crate::target::PlayerFilter::IteratedPlayer => {
                "as long as that player is the monarch".to_string()
            }
            crate::target::PlayerFilter::TargetPlayerOrControllerOfTarget => {
                "as long as that player is the monarch".to_string()
            }
            crate::target::PlayerFilter::Target(_) => {
                "as long as that player is the monarch".to_string()
            }
            crate::target::PlayerFilter::AliasedTarget(_) => {
                "as long as that player is the monarch".to_string()
            }
            crate::target::PlayerFilter::Excluding { .. } => {
                "as long as that player is the monarch".to_string()
            }
            crate::target::PlayerFilter::ControllerOf(_) => {
                "as long as that target's controller is the monarch".to_string()
            }
            crate::target::PlayerFilter::OwnerOf(_) => {
                "as long as that target's owner is the monarch".to_string()
            }
            crate::target::PlayerFilter::AliasedOwnerOf(_) => {
                "as long as that player is the monarch".to_string()
            }
            crate::target::PlayerFilter::AliasedControllerOf(_) => {
                "as long as that player is the monarch".to_string()
            }
        },
        crate::ConditionExpr::PlayerHasInitiative { player } => match player {
            crate::target::PlayerFilter::You => "as long as you have the initiative".to_string(),
            _ => "as long as that player has the initiative".to_string(),
        },
        crate::ConditionExpr::PlayerHasCitysBlessing { player } => match player {
            crate::target::PlayerFilter::You => {
                "as long as you have the city's blessing".to_string()
            }
            _ => "as long as that player has the city's blessing".to_string(),
        },
        crate::ConditionExpr::ActivationTiming(
            crate::ability::ActivationTiming::DuringYourTurn,
        ) => "during your turn".to_string(),
        crate::ConditionExpr::ActivationTiming(
            crate::ability::ActivationTiming::DuringOpponentsTurn,
        ) => "during opponents' turns".to_string(),
        crate::ConditionExpr::OwnsCardExiledWithCounter(counter) => {
            let counter_name = format!("{counter:?}").to_ascii_lowercase();
            format!("as long as you own a card exiled with a {counter_name} counter")
        }
        crate::ConditionExpr::TurnHistory(
            ironsmith_core::TurnHistoryCondition::PlayerVisitedAttractionThisTurn(player),
        ) => match player {
            crate::target::PlayerFilter::You => {
                "as long as you've visited an Attraction this turn".to_string()
            }
            crate::target::PlayerFilter::Opponent => {
                "as long as an opponent has visited an Attraction this turn".to_string()
            }
            crate::target::PlayerFilter::Any => {
                "as long as a player has visited an Attraction this turn".to_string()
            }
            _ => "as long as that player has visited an Attraction this turn".to_string(),
        },
        crate::ConditionExpr::PlayerCommittedCrimeThisTurn { player } => match player {
            crate::target::PlayerFilter::You => {
                "as long as you've committed a crime this turn".to_string()
            }
            _ => "as long as that player committed a crime this turn".to_string(),
        },
        crate::ConditionExpr::PlayerCompletedDungeon {
            player,
            dungeon_name,
        } => match (player, dungeon_name.as_deref()) {
            (crate::target::PlayerFilter::You, None) => {
                "as long as you've completed a dungeon".to_string()
            }
            (crate::target::PlayerFilter::You, Some(name)) => {
                format!("as long as you've completed {name}")
            }
            (_, None) => "as long as that player completed a dungeon".to_string(),
            (_, Some(name)) => format!("as long as that player completed {name}"),
        },
        _ => format!(
            "as long as {}",
            crate::runtime_display::describe_condition(condition)
        ),
    }
}

fn describe_same_source_static_condition(condition: &crate::ConditionExpr) -> String {
    if source_is_attacking_alone_condition(condition) {
        return "as long as it's attacking alone".to_string();
    }
    match condition {
        crate::ConditionExpr::SourceIsEquipped => "as long as it's equipped".to_string(),
        crate::ConditionExpr::SourceIsEnchanted => "as long as it's enchanted".to_string(),
        crate::ConditionExpr::SourceIsMonstrous => "as long as it's monstrous".to_string(),
        crate::ConditionExpr::SourceIsAttacking => "as long as it's attacking".to_string(),
        crate::ConditionExpr::SourceIsUntapped => "as long as it's untapped".to_string(),
        other => describe_static_condition(other),
    }
}

fn source_is_attacking_alone_condition(condition: &crate::ConditionExpr) -> bool {
    fn is_source_attacking(condition: &crate::ConditionExpr) -> bool {
        matches!(condition, crate::ConditionExpr::SourceIsAttacking)
    }

    fn counts_exactly_one_attacking_creature(condition: &crate::ConditionExpr) -> bool {
        let crate::ConditionExpr::CountComparison {
            count: AnthemCountExpression::MatchingFilter(filter),
            comparison: Comparison::Equal(1),
            ..
        } = condition
        else {
            return false;
        };
        let mut attacking_creatures = ObjectFilter::creature();
        attacking_creatures.attacking = true;
        *filter == attacking_creatures
    }

    let crate::ConditionExpr::And(left, right) = condition else {
        return false;
    };
    (is_source_attacking(left) && counts_exactly_one_attacking_creature(right))
        || (is_source_attacking(right) && counts_exactly_one_attacking_creature(left))
}

fn describe_static_possessive_player(player: &crate::target::PlayerFilter) -> &'static str {
    match player {
        crate::target::PlayerFilter::You => "your",
        crate::target::PlayerFilter::Opponent => "an opponent's",
        crate::target::PlayerFilter::Any => "a player's",
        crate::target::PlayerFilter::NotYou => "another player's",
        crate::target::PlayerFilter::Teammate => "a teammate's",
        crate::target::PlayerFilter::Active => "the active player's",
        crate::target::PlayerFilter::Defending => "the defending player's",
        crate::target::PlayerFilter::Attacking => "the attacking player's",
        crate::target::PlayerFilter::DamagedPlayer => "the damaged player's",
        _ => "that player's",
    }
}

fn describe_static_player(player: &crate::target::PlayerFilter) -> &'static str {
    match player {
        crate::target::PlayerFilter::You => "you",
        crate::target::PlayerFilter::Opponent => "an opponent",
        crate::target::PlayerFilter::Any => "a player",
        crate::target::PlayerFilter::NotYou => "another player",
        crate::target::PlayerFilter::Teammate => "a teammate",
        crate::target::PlayerFilter::Active => "the active player",
        crate::target::PlayerFilter::Defending => "the defending player",
        crate::target::PlayerFilter::Attacking => "the attacking player",
        crate::target::PlayerFilter::DamagedPlayer => "the damaged player",
        crate::target::PlayerFilter::EffectController => "that effect's controller",
        crate::target::PlayerFilter::MostLifeTied => "the player with the most life",
        crate::target::PlayerFilter::LowestLifeTied => "the player with the lowest life",
        crate::target::PlayerFilter::MostCardsInHand => "the player who has the most cards in hand",
        crate::target::PlayerFilter::ChosenPlayer => "the chosen player",
        _ => "that player",
    }
}

fn all_game_object_ids(game: &GameState) -> Vec<ObjectId> {
    let mut ids = Vec::new();
    ids.extend(game.battlefield.iter().copied());
    ids.extend(game.exile.iter().copied());
    ids.extend(game.command_zone.iter().copied());
    ids.extend(game.stack.iter().map(|entry| entry.object_id));
    for player in &game.players {
        ids.extend(player.library.iter().copied());
        ids.extend(player.hand.iter().copied());
        ids.extend(player.graveyard.iter().copied());
    }
    ids
}

fn entered_battlefield_this_turn_count(
    filter: &ObjectFilter,
    game: &GameState,
    source: ObjectId,
    controller: PlayerId,
) -> i32 {
    let mut filter_ctx = game.filter_context_for(controller, Some(source));
    if let Some(affected) = game.object(source) {
        filter_ctx.target_objects =
            vec![crate::snapshot::ObjectSnapshot::from_object(affected, game)];
    }
    let type_effects = game.cached_continuous_effects_snapshot();
    game.turn_store
        .turn_history
        .event_records
        .iter()
        .chain(game.turn_store.turn_history.staged_event_records.iter())
        .filter(|record| {
            record
                .event
                .downcast::<crate::events::EnterBattlefieldEvent>()
                .is_some()
                || record
                    .event
                    .downcast::<crate::events::zones::ZoneChangeEvent>()
                    .is_some_and(|event| event.is_etb())
        })
        .filter(|record| {
            record.object_snapshot.as_ref().is_some_and(|snapshot| {
                if filter.other && snapshot.object_id == source {
                    return false;
                }
                let current_id = game
                    .object(snapshot.object_id)
                    .map(|object| object.id)
                    .or_else(|| game.find_object_by_stable_id(snapshot.stable_id))
                    .or_else(|| {
                        game.battlefield.iter().copied().find(|id| {
                            game.object(*id).is_some_and(|object| {
                                object.name == snapshot.name
                                    && object.owner == snapshot.owner
                                    && game.controller_of(object) == snapshot.controller
                            })
                        })
                    });
                if let Some(current) = current_id.and_then(|id| game.object(id))
                    && current.zone == Zone::Battlefield
                {
                    let mut adjusted = current.clone();
                    for effect in &type_effects {
                        if !matches!(
                            effect.modification,
                            Modification::AddCardTypes(_)
                                | Modification::RemoveCardTypes(_)
                                | Modification::SetCardTypes(_)
                        ) {
                            continue;
                        }
                        let applies = match &effect.applies_to {
                            EffectTarget::Specific(target) => *target == current.id,
                            EffectTarget::Source => effect.source == current.id,
                            EffectTarget::AllPermanents => current.zone == Zone::Battlefield,
                            EffectTarget::Filter(effect_filter) => effect_filter
                                .matches_non_recursive(
                                    current,
                                    &game
                                        .filter_context_for(effect.controller, Some(effect.source)),
                                    game,
                                ),
                            EffectTarget::AllCreatures => {
                                current.zone == Zone::Battlefield
                                    && current.card_types.contains(&CardType::Creature)
                            }
                            EffectTarget::AttachedTo(attached_source) => {
                                game.object(*attached_source)
                                    .and_then(|source_obj| source_obj.attached_to)
                                    .and_then(|target| target.object_id())
                                    == Some(current.id)
                            }
                        };
                        if !applies {
                            continue;
                        }
                        if !crate::continuous::continuous_effect_duration_and_condition_are_active(
                            effect, game,
                        ) {
                            continue;
                        }
                        match &effect.modification {
                            Modification::AddCardTypes(card_types) => {
                                for card_type in card_types {
                                    if !adjusted.card_types.contains(card_type) {
                                        adjusted.card_types.push(*card_type);
                                    }
                                }
                            }
                            Modification::RemoveCardTypes(card_types) => {
                                adjusted
                                    .card_types
                                    .retain(|card_type| !card_types.contains(card_type));
                            }
                            Modification::SetCardTypes(card_types) => {
                                crate::continuous::replace_card_types_and_prune_subtypes(
                                    &mut adjusted.card_types,
                                    &mut adjusted.subtypes,
                                    card_types,
                                );
                            }
                            _ => {}
                        }
                    }
                    return filter.matches_non_recursive(&adjusted, &filter_ctx, game);
                }
                filter.matches_snapshot(snapshot, &filter_ctx, game)
            })
        })
        .count() as i32
}

pub(crate) fn resolve_anthem_count_expression(
    count: &AnthemCountExpression,
    game: &GameState,
    source: ObjectId,
    controller: PlayerId,
) -> i32 {
    let mut filter_ctx = game.filter_context_for(controller, Some(source));
    if let Some(affected) = game.object(source) {
        filter_ctx.target_objects =
            vec![crate::snapshot::ObjectSnapshot::from_object(affected, game)];
    }
    match count {
        AnthemCountExpression::MatchingFilter(filter)
            if filter.entered_battlefield_this_turn
                || filter.entered_battlefield_controller.is_some() =>
        {
            entered_battlefield_this_turn_count(filter, game, source, controller)
        }
        AnthemCountExpression::MatchingFilter(filter) => all_game_object_ids(game)
            .into_iter()
            .filter_map(|id| game.object(id))
            .filter(|obj| filter.matches_non_recursive(obj, &filter_ctx, game))
            .count() as i32,
        AnthemCountExpression::GraveyardsWithAtLeastCards { minimum_cards } => {
            game.turn_store
                .turn_order
                .iter()
                .filter_map(|player| game.player(*player))
                .filter(|player| {
                    player.is_in_game() && player.graveyard.len() >= *minimum_cards as usize
                })
                .count() as i32
        }
        AnthemCountExpression::GreatestManaValueAmong(filter) => all_game_object_ids(game)
            .into_iter()
            .filter_map(|id| game.object(id))
            .filter(|obj| filter.matches_non_recursive(obj, &filter_ctx, game))
            .map(|obj| obj.subject_mana_value())
            .max()
            .unwrap_or(0) as i32,
        AnthemCountExpression::AttachedToSource(filter)
        | AnthemCountExpression::AttachedToAffected(filter) => game
            .object(source)
            .map(|source_obj| {
                source_obj
                    .attachments
                    .iter()
                    .filter_map(|id| game.object(*id))
                    .filter(|obj| filter.matches_non_recursive(obj, &filter_ctx, game))
                    .count() as i32
            })
            .unwrap_or(0),
        AnthemCountExpression::ColorsOfAffected => game
            .calculated_characteristics(source)
            .map(|chars| chars.colors.count() as i32)
            .unwrap_or(0),
        AnthemCountExpression::AffectedAttackedThisTurn => {
            game.creature_attack_count_this_turn(source) as i32
        }
        AnthemCountExpression::CountersOnSource(counter_type) => {
            game.counter_count(source, *counter_type) as i32
        }
        AnthemCountExpression::CountersOnSourceWithSurface { counter_type, .. } => {
            game.counter_count(source, *counter_type) as i32
        }
        AnthemCountExpression::CountersOnSourceWithPronoun { counter_type, .. } => {
            game.counter_count(source, *counter_type) as i32
        }
        AnthemCountExpression::StickersOnSource {
            action,
            min_name_letters,
            max_name_letters,
            ..
        } => game.sticker_count_on_object_with_name_letter_range(
            source,
            *action,
            *min_name_letters,
            *max_name_letters,
        ) as i32,
        AnthemCountExpression::CountersOnAffected(counter_type) => {
            game.counter_count(source, *counter_type) as i32
        }
        AnthemCountExpression::CountersAmong(filter, counter_type) => all_game_object_ids(game)
            .into_iter()
            .filter_map(|id| game.object(id))
            .filter(|obj| filter.matches_non_recursive(obj, &filter_ctx, game))
            .map(|obj| obj.counters.get(counter_type).copied().unwrap_or(0) as i32)
            .sum(),
        AnthemCountExpression::DistinctCounterTypesAmong(filter) => {
            use std::collections::HashSet;

            let mut seen = HashSet::new();
            for obj in all_game_object_ids(game)
                .into_iter()
                .filter_map(|id| game.object(id))
                .filter(|obj| filter.matches_non_recursive(obj, &filter_ctx, game))
            {
                seen.extend(obj.counters.keys().copied());
            }
            seen.len() as i32
        }
        AnthemCountExpression::BasicLandTypesAmong(filter) => {
            use std::collections::HashSet;

            let mut seen = HashSet::new();
            for obj in all_game_object_ids(game)
                .into_iter()
                .filter_map(|id| game.object(id))
                .filter(|obj| filter.matches_non_recursive(obj, &filter_ctx, game))
            {
                for subtype in &obj.subtypes {
                    if matches!(
                        subtype,
                        Subtype::Plains
                            | Subtype::Island
                            | Subtype::Swamp
                            | Subtype::Mountain
                            | Subtype::Forest
                    ) {
                        seen.insert(*subtype);
                    }
                }
            }
            seen.len() as i32
        }
        AnthemCountExpression::CreatureTypesAmong(filter) if filter == &ObjectFilter::source() => {
            game.current_subtypes(source)
                .unwrap_or_default()
                .into_iter()
                .filter(|subtype| subtype.is_creature_type())
                .collect::<std::collections::HashSet<_>>()
                .len() as i32
        }
        AnthemCountExpression::CreatureTypesAmong(filter) => {
            use std::collections::HashSet;

            let mut seen = HashSet::new();
            for obj in all_game_object_ids(game)
                .into_iter()
                .filter_map(|id| game.object(id))
                .filter(|obj| filter.matches_non_recursive(obj, &filter_ctx, game))
            {
                for subtype in SubtypeFamily::Creature.all_subtypes() {
                    if obj.has_subtype(*subtype) {
                        seen.insert(*subtype);
                    }
                }
            }
            seen.len() as i32
        }
        AnthemCountExpression::BlockingSource => game
            .combat
            .as_ref()
            .and_then(|combat| combat.blockers.get(&source))
            .map(|blockers| blockers.len() as i32)
            .unwrap_or(0),
        AnthemCountExpression::CommanderCastCount(player_filter) => game
            .players
            .iter()
            .filter(|player| {
                player.is_in_game() && player_filter.matches_player(player.id, &filter_ctx)
            })
            .map(|player| game.commander_cast_count_for_player(player.id) as i32)
            .sum(),
        AnthemCountExpression::PlayerSpeed(player_filter) => game
            .players
            .iter()
            .filter(|player| {
                player.is_in_game()
                    && crate::filter::player_filter_matches_game(
                        player_filter,
                        player.id,
                        game,
                        &filter_ctx,
                    )
            })
            .map(|player| game.player_speed(player.id).unwrap_or(0) as i32)
            .sum(),
        AnthemCountExpression::UnspentMana {
            player: player_filter,
            symbol,
        } => game
            .players
            .iter()
            .filter(|player| {
                player.is_in_game()
                    && crate::filter::player_filter_matches_game(
                        player_filter,
                        player.id,
                        game,
                        &filter_ctx,
                    )
            })
            .map(|player| match symbol {
                crate::mana::ManaSymbol::White => player.mana_pool.white,
                crate::mana::ManaSymbol::Blue => player.mana_pool.blue,
                crate::mana::ManaSymbol::Black => player.mana_pool.black,
                crate::mana::ManaSymbol::Red => player.mana_pool.red,
                crate::mana::ManaSymbol::Green => player.mana_pool.green,
                crate::mana::ManaSymbol::Colorless => player.mana_pool.colorless,
                _ => 0,
            } as i32)
            .sum(),
    }
}

pub(super) fn static_condition_is_active(
    condition: &crate::ConditionExpr,
    game: &GameState,
    source: ObjectId,
    controller: PlayerId,
) -> bool {
    static_condition_is_active_with_iterated_player(condition, game, source, controller, None)
}

fn static_condition_is_active_with_iterated_player(
    condition: &crate::ConditionExpr,
    game: &GameState,
    source: ObjectId,
    controller: PlayerId,
    iterated_player: Option<PlayerId>,
) -> bool {
    let eval_ctx = crate::condition_eval::ExternalEvaluationContext {
        controller,
        source,
        defending_player: None,
        attacking_player: None,
        filter_source: Some(source),
        iterated_player,
        triggering_event: None,
        trigger_identity: None,
        ability_index: None,
        options: Default::default(),
    };
    crate::condition_eval::evaluate_condition_external(game, condition, &eval_ctx)
}

fn effect_with_optional_static_condition(
    effect: ContinuousEffect,
    condition: &Option<crate::ConditionExpr>,
) -> ContinuousEffect {
    match condition {
        Some(condition) => effect.with_condition(condition.clone()),
        None => effect,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Anthem {
    /// Filter for which permanents are affected.
    pub filter: ObjectFilter,
    /// If true, the source permanent itself is the only affected object.
    pub source_only: bool,
    /// Power modification.
    pub power: AnthemValue,
    /// Toughness modification.
    pub toughness: AnthemValue,
    /// Optional activation condition.
    pub condition: Option<crate::ConditionExpr>,
    /// Original leading set quantifier, retained only for compiled-text surface.
    pub set_quantifier_surface: Option<ironsmith_core::SetQuantifierSurface>,
    /// True when the original oracle text scaled with a "where X is …" clause
    /// rather than "for each …". Surface hint for rendering only.
    pub count_uses_where_x: bool,
    /// Authored "an additional P/T" surface. Execution remains the same
    /// additive continuous modifier.
    pub additional_surface: bool,
    /// Absolute P/T from a typed "gets P/T instead" continuation. The effect
    /// values above remain the executable conditional delta.
    pub replacement_surface: Option<ironsmith_core::AnthemReplacementSurface>,
}

impl Anthem {
    pub fn new(filter: ObjectFilter, power: i32, toughness: i32) -> Self {
        Self {
            filter,
            source_only: false,
            power: AnthemValue::Fixed(power),
            toughness: AnthemValue::Fixed(toughness),
            condition: None,
            set_quantifier_surface: None,
            count_uses_where_x: false,
            additional_surface: false,
            replacement_surface: None,
        }
    }

    pub fn for_source(power: i32, toughness: i32) -> Self {
        Self {
            filter: ObjectFilter::creature(),
            source_only: true,
            power: AnthemValue::Fixed(power),
            toughness: AnthemValue::Fixed(toughness),
            condition: None,
            set_quantifier_surface: None,
            count_uses_where_x: false,
            additional_surface: false,
            replacement_surface: None,
        }
    }

    pub fn with_values(mut self, power: AnthemValue, toughness: AnthemValue) -> Self {
        self.power = power;
        self.toughness = toughness;
        self
    }

    pub fn with_count_uses_where_x(mut self, uses_where_x: bool) -> Self {
        self.count_uses_where_x = uses_where_x;
        self
    }

    pub fn with_additional_surface(mut self, additional_surface: bool) -> Self {
        self.additional_surface = additional_surface;
        self
    }

    pub fn with_set_quantifier_surface(
        mut self,
        surface: Option<ironsmith_core::SetQuantifierSurface>,
    ) -> Self {
        self.set_quantifier_surface = surface;
        self
    }

    pub fn with_replacement_surface(mut self, power: i32, toughness: i32) -> Self {
        self.replacement_surface =
            Some(ironsmith_core::AnthemReplacementSurface { power, toughness });
        self
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }

    /// Create a standard anthem for creatures you control.
    pub fn creatures_you_control(power: i32, toughness: i32) -> Self {
        Self::new(ObjectFilter::creature().you_control(), power, toughness)
    }
}

impl StaticAbilityKind for Anthem {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Anthem
    }

    fn structural_effect_filter(&self) -> Option<&ObjectFilter> {
        (!self.source_only).then_some(&self.filter)
    }

    fn display(&self) -> String {
        let source_and_land_creatures = !self.source_only
            && self.filter.source
            && self.filter.zone == Some(crate::zone::Zone::Battlefield)
            && self.filter.controller == Some(crate::target::PlayerFilter::You)
            && self.filter.card_types.len() == 2
            && self
                .filter
                .card_types
                .contains(&crate::types::CardType::Land)
            && self
                .filter
                .card_types
                .contains(&crate::types::CardType::Creature)
            && self.filter.subtypes.is_empty()
            && self.filter.any_of.is_empty();
        let subject = if self.source_only {
            "this creature".to_string()
        } else if source_and_land_creatures {
            "this creature and land creatures you control".to_string()
        } else if let Some(surface) = self.set_quantifier_surface {
            let mut subject = subject_text(&self.filter);
            if let Some(rest) = subject.strip_prefix("another ") {
                subject = format!("other {rest}");
            }
            let subject = strip_plural_subject_article(&subject);
            match surface {
                ironsmith_core::SetQuantifierSurface::All => {
                    format!("All {}", pluralize_subject_clause(subject))
                }
                ironsmith_core::SetQuantifierSurface::Each => format!("Each {subject}"),
                ironsmith_core::SetQuantifierSurface::They => "They".to_string(),
                ironsmith_core::SetQuantifierSurface::Those => {
                    format!("Those {}", pluralize_subject_clause(subject))
                }
            }
        } else if let Some(subject) =
            exact_one_condition_antecedent_subject(&self.filter, self.condition.as_ref())
        {
            subject
        } else {
            subtype_creature_anthem_subject(&self.filter)
                .unwrap_or_else(|| pluralized_subject_text(&self.filter))
        };
        let subject_mentions_plural = subject.contains("creatures")
            || subject.contains("tokens")
            || subject.contains("permanents")
            || subject.contains("artifacts")
            || subject.contains("enchantments")
            || subject.contains("lands")
            || subject.contains("planeswalkers")
            || subject.contains("battles")
            || subject.contains("spells")
            || subject.contains("cards")
            || subject.contains("allies");
        let singular = self.source_only
            || subject.starts_with("Each ")
            || subject.starts_with("a ")
            || subject.starts_with("an ")
            || (subject.starts_with("this ") && !subject.contains(" and "))
            || subject.starts_with("that ")
            || ((subject.starts_with("enchanted ") || subject.starts_with("equipped "))
                && !subject_mentions_plural);
        let verb = if singular { "gets" } else { "get" };

        let signed = |value: i32| {
            if value >= 0 {
                format!("+{value}")
            } else {
                value.to_string()
            }
        };
        let x_component = |value: i32| {
            if value == 1 {
                "+X".to_string()
            } else if value == -1 {
                "-X".to_string()
            } else if value > 0 {
                format!("+{value}X")
            } else {
                format!("{value}X")
            }
        };
        let signed_toughness = |power: i32, toughness: i32| {
            if power < 0 && toughness == 0 {
                "-0".to_string()
            } else {
                signed(toughness)
            }
        };
        let dynamic_for_each = |value: &Value| {
            value
                .has_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach)
                .then(|| {
                    crate::runtime_display::describe_party_size_for_each_basis(value)
                        .or_else(|| crate::runtime_display::describe_counter_for_each_basis(value))
                        .or_else(|| {
                            crate::runtime_display::describe_for_each_multiplier_and_basis(value)
                        })
                })
                .flatten()
        };

        if let (Some(surface), Some(condition)) = (self.replacement_surface, &self.condition) {
            let described = describe_attached_subject_static_condition(condition, &subject)
                .unwrap_or_else(|| describe_static_condition(condition));
            let condition_text = described
                .strip_prefix("as long as ")
                .unwrap_or(&described)
                .to_string();
            return format!(
                "If {condition_text}, {subject} {verb} {}/{} instead",
                signed(surface.power),
                signed_toughness(surface.power, surface.toughness),
            );
        }

        let mut text = match (&self.power, &self.toughness) {
            (AnthemValue::Fixed(power), AnthemValue::Fixed(toughness)) => {
                let additional = if self.additional_surface {
                    "an additional "
                } else {
                    ""
                };
                format!(
                    "{subject} {verb} {additional}{}/{}",
                    signed(*power),
                    signed_toughness(*power, *toughness),
                )
            }
            (AnthemValue::Dynamic(power), AnthemValue::Dynamic(toughness))
                if dynamic_for_each(power).is_some()
                    && dynamic_for_each(power).map(|(_, subject)| subject)
                        == dynamic_for_each(toughness).map(|(_, subject)| subject) =>
            {
                let (power, basis) =
                    dynamic_for_each(power).expect("for-each value checked in match guard");
                let (toughness, _) = dynamic_for_each(toughness)
                    .expect("matching for-each value checked in match guard");
                format!(
                    "{subject} {verb} {}/{} for each {basis}",
                    signed(power),
                    signed_toughness(power, toughness),
                )
            }
            (AnthemValue::Dynamic(power), AnthemValue::Fixed(toughness))
                if dynamic_for_each(power).is_some() =>
            {
                let (power, basis) =
                    dynamic_for_each(power).expect("for-each value checked in match guard");
                format!(
                    "{subject} {verb} {}/{} for each {basis}",
                    signed(power),
                    signed_toughness(power, *toughness),
                )
            }
            (AnthemValue::Fixed(power), AnthemValue::Dynamic(toughness))
                if dynamic_for_each(toughness).is_some() =>
            {
                let (toughness, basis) =
                    dynamic_for_each(toughness).expect("for-each value checked in match guard");
                format!(
                    "{subject} {verb} {}/{} for each {basis}",
                    signed(*power),
                    signed_toughness(*power, toughness),
                )
            }
            (AnthemValue::Dynamic(power), AnthemValue::Dynamic(toughness))
                if power == toughness =>
            {
                format!(
                    "{subject} {verb} +X/+X, where X is {}",
                    crate::runtime_display::describe_value(power),
                )
            }
            (AnthemValue::Dynamic(power), AnthemValue::Dynamic(toughness)) => {
                format!(
                    "{subject} {verb} +X/+Y, where X is {}, and Y is {}",
                    crate::runtime_display::describe_value(power),
                    crate::runtime_display::describe_value(toughness),
                )
            }
            (AnthemValue::Dynamic(power), AnthemValue::Fixed(toughness)) => {
                format!(
                    "{subject} {verb} +X/{}, where X is {}",
                    signed(*toughness),
                    crate::runtime_display::describe_value(power),
                )
            }
            (AnthemValue::Fixed(power), AnthemValue::Dynamic(toughness)) => {
                format!(
                    "{subject} {verb} {}/+X, where X is {}",
                    signed(*power),
                    crate::runtime_display::describe_value(toughness),
                )
            }
            (
                AnthemValue::CappedPerCount {
                    multiplier: power,
                    count: power_count,
                    maximum: power_maximum,
                },
                AnthemValue::CappedPerCount {
                    multiplier: toughness,
                    count: toughness_count,
                    maximum: toughness_maximum,
                },
            ) if power_count == toughness_count
                && power_maximum == toughness_maximum
                && describe_anthem_for_each_count_expression(power_count).is_some() =>
            {
                let count_subject = describe_anthem_for_each_count_expression(power_count)
                    .expect("checked capped anthem count surface");
                format!(
                    "{subject} {verb} {}/{} for each {count_subject}, to a maximum of {power_maximum}",
                    signed(*power),
                    signed_toughness(*power, *toughness),
                )
            }
            (
                AnthemValue::PerCount {
                    multiplier: power,
                    count: power_count,
                },
                AnthemValue::PerCount {
                    multiplier: toughness,
                    count: toughness_count,
                },
            ) if self.count_uses_where_x
                && *power == 1
                && *toughness == 1
                && matching_counter_source_location(power_count, toughness_count).is_some() =>
            {
                let (counter_type, location) =
                    matching_counter_source_location(power_count, toughness_count)
                        .expect("checked counter source location");
                format!(
                    "{subject} {verb} +X/+X, where X is the number of {} counters on {location}",
                    counter_type.description(),
                )
            }
            (
                AnthemValue::PerCount {
                    multiplier: power,
                    count: power_count,
                },
                AnthemValue::PerCount {
                    multiplier: toughness,
                    count: toughness_count,
                },
            ) if self.count_uses_where_x
                && power_count == toughness_count
                && power == toughness =>
            {
                format!(
                    "{subject} {verb} {}/{}, where X is {}",
                    x_component(*power),
                    x_component(*toughness),
                    describe_anthem_where_x_count_expression(power_count),
                )
            }
            (
                AnthemValue::PerCount {
                    multiplier: power,
                    count: power_count,
                },
                AnthemValue::PerCount {
                    multiplier: toughness,
                    count: toughness_count,
                },
            ) if power_count == toughness_count
                && matches!(power_count, AnthemCountExpression::PlayerSpeed(_)) =>
            {
                format!(
                    "{subject} {verb} {}/{}, where X is {}",
                    x_component(*power),
                    x_component(*toughness),
                    describe_anthem_count_expression(power_count),
                )
            }
            (
                AnthemValue::PerCount {
                    multiplier: power,
                    count: power_count,
                },
                AnthemValue::PerCount {
                    multiplier: toughness,
                    count: toughness_count,
                },
            ) if power_count == toughness_count && *power == 1 && *toughness == 1 => {
                if self.count_uses_where_x {
                    format!(
                        "{subject} {verb} +X/+X, where X is {}",
                        describe_anthem_where_x_count_expression(power_count),
                    )
                } else if let Some(count_subject) =
                    describe_anthem_for_each_count_expression(power_count)
                {
                    if matches!(power_count, AnthemCountExpression::CommanderCastCount(_))
                        && !self.source_only
                        && self.filter.zone == Some(Zone::Battlefield)
                        && self.filter.controller == Some(crate::target::PlayerFilter::You)
                        && self.filter.card_types == vec![CardType::Creature]
                    {
                        format!("Creatures you control get +1/+1 for each {count_subject}")
                    } else {
                        format!("{subject} {verb} +1/+1 for each {count_subject}")
                    }
                } else if let Some(count_subject) =
                    describe_anthem_for_each_graveyard_count_expression(power_count)
                {
                    // The original oracle wrote "for each …" (no "where X is" clause):
                    // prefer the "for each" surface for counts the primary helper can't
                    // express on its own, such as cards in a graveyard.
                    format!("{subject} {verb} +1/+1 for each {count_subject}")
                } else {
                    format!(
                        "{subject} {verb} +X/+X, where X is {}",
                        describe_anthem_where_x_count_expression(power_count),
                    )
                }
            }
            (
                AnthemValue::PerCount {
                    multiplier: power,
                    count: power_count,
                },
                AnthemValue::PerCount {
                    multiplier: toughness,
                    count: toughness_count,
                },
            ) if power_count == toughness_count => {
                format!(
                    "{subject} {verb} {}/{} for each {}",
                    signed(*power),
                    signed_toughness(*power, *toughness),
                    describe_anthem_count_expression(power_count),
                )
            }
            (
                AnthemValue::PerCount {
                    multiplier: power,
                    count: count @ AnthemCountExpression::PlayerSpeed(_),
                },
                AnthemValue::Fixed(toughness),
            ) if *toughness == 0 => {
                format!(
                    "{subject} {verb} {}/+0, where X is {}",
                    x_component(*power),
                    describe_anthem_count_expression(count),
                )
            }
            (
                AnthemValue::PerCount {
                    multiplier: power,
                    count,
                },
                AnthemValue::Fixed(toughness),
            ) if *toughness == 0 => {
                if self.count_uses_where_x {
                    format!(
                        "{subject} {verb} {}/+0, where X is {}",
                        x_component(*power),
                        describe_anthem_where_x_count_expression(count),
                    )
                } else if let Some(count_subject) = describe_anthem_for_each_count_expression(count)
                {
                    format!(
                        "{subject} {verb} {}/+0 for each {count_subject}",
                        signed(*power),
                    )
                } else if let Some(count_subject) =
                    describe_anthem_for_each_graveyard_count_expression(count)
                {
                    format!(
                        "{subject} {verb} {}/+0 for each {count_subject}",
                        signed(*power),
                    )
                } else {
                    format!(
                        "{subject} {verb} {}/{}, where X is {}",
                        x_component(*power),
                        signed(*toughness),
                        describe_anthem_where_x_count_expression(count),
                    )
                }
            }
            (
                AnthemValue::PerCount {
                    multiplier: power,
                    count,
                },
                AnthemValue::Fixed(toughness),
            ) => format!(
                "{subject} {verb} {}/{}, where X is {}",
                x_component(*power),
                signed(*toughness),
                describe_anthem_where_x_count_expression(count),
            ),
            (
                AnthemValue::Fixed(power),
                AnthemValue::PerCount {
                    multiplier: toughness,
                    count,
                },
            ) if *power == 0 => {
                if let Some(count_subject) = describe_anthem_for_each_count_expression(count) {
                    format!(
                        "{subject} {verb} +0/{} for each {count_subject}",
                        signed(*toughness),
                    )
                } else if let Some(count_subject) =
                    describe_anthem_for_each_graveyard_count_expression(count)
                {
                    format!(
                        "{subject} {verb} +0/{} for each {count_subject}",
                        signed(*toughness),
                    )
                } else {
                    format!(
                        "{subject} {verb} {}/{}, where X is {}",
                        signed(*power),
                        x_component(*toughness),
                        describe_anthem_where_x_count_expression(count),
                    )
                }
            }
            (
                AnthemValue::Fixed(power),
                AnthemValue::PerCount {
                    multiplier: toughness,
                    count: count @ AnthemCountExpression::PlayerSpeed(_),
                },
            ) if *power == 0 => {
                format!(
                    "{subject} {verb} +0/{}, where X is {}",
                    x_component(*toughness),
                    describe_anthem_count_expression(count),
                )
            }
            (
                AnthemValue::Fixed(power),
                AnthemValue::PerCount {
                    multiplier: toughness,
                    count,
                },
            ) => format!(
                "{subject} {verb} {}/{}, where X is the number of {}",
                signed(*power),
                x_component(*toughness),
                describe_anthem_count_expression(count),
            ),
            _ => format!("{subject} {verb} dynamic power/toughness"),
        };

        if let Some(condition) = &self.condition {
            let condition_text = if self.source_only {
                describe_same_source_static_condition(condition)
            } else if let Some(described) =
                describe_attached_subject_static_condition(condition, &subject)
            {
                described
            } else {
                describe_static_condition(condition)
            };
            if static_condition_is_during_your_turn(condition) {
                return format!("During your turn, {text}");
            }
            if matches!(condition, crate::ConditionExpr::SourceControllersEndStep) {
                return format!("During your end step, {text}");
            }
            if source_and_land_creatures
                && let Some(rest) = condition_text.strip_prefix("as long as ")
            {
                return format!("As long as {rest}, {text}");
            }
            // Only the "different kinds of counters among ..." count condition reads
            // naturally in the leading form (e.g. Hundred-Battle Veteran). All other
            // source-only pump conditions keep the trailing "... as long as ..." form.
            if self.source_only
                && matches!(
                    condition,
                    crate::ConditionExpr::CountComparison {
                        count: AnthemCountExpression::DistinctCounterTypesAmong(_),
                        ..
                    }
                )
                && let Some(rest) = condition_text.strip_prefix("as long as ")
            {
                return format!("As long as {rest}, {text}");
            }
            text.push(' ');
            text.push_str(&condition_text);
        }
        text
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        game: &GameState,
    ) -> Vec<ContinuousEffect> {
        // Color-count anthems need the affected object's layer-5 colors, so keep
        // the target dynamic and defer the count until layer 7 applies.
        if let (Some(power_multiplier), Some(toughness_multiplier)) = (
            color_count_multiplier(&self.power),
            color_count_multiplier(&self.toughness),
        ) {
            let target = if self.source_only {
                EffectTarget::Source
            } else {
                effect_target_for_filter(source, &self.filter)
            };
            return vec![effect_with_optional_static_condition(
                ContinuousEffect::new(
                    source,
                    controller,
                    target,
                    Modification::ModifyPowerToughnessByColorCount {
                        power_multiplier,
                        toughness_multiplier,
                    },
                )
                .with_source_type(EffectSourceType::StaticAbility),
                &self.condition,
            )];
        }

        let uses_affected =
            self.power.uses_affected_object() || self.toughness.uses_affected_object();

        // Other affected-object counts are game-state counts, so enumerate each
        // affected creature and evaluate the count per-creature.
        if uses_affected && !self.source_only {
            let filter_ctx = game.filter_context_for(controller, Some(source));
            let attached_target = if attached_subject(&self.filter).is_some() {
                game.object(source)
                    .and_then(|source_obj| source_obj.attached_to)
                    .and_then(|target| target.object_id())
            } else {
                None
            };
            let match_filter = attached_target
                .is_some()
                .then(|| filter_for_attached_subject_match(&self.filter));
            let mut effects = Vec::new();
            let candidate_ids: Vec<ObjectId> = attached_target
                .map(|id| vec![id])
                .unwrap_or_else(|| game.battlefield.clone());
            for obj_id in candidate_ids {
                let Some(obj) = game.object(obj_id) else {
                    continue;
                };
                if obj.zone != Zone::Battlefield {
                    continue;
                }
                let matches_filter = match_filter
                    .as_ref()
                    .unwrap_or(&self.filter)
                    .matches_non_recursive(obj, &filter_ctx, game);
                if !matches_filter {
                    continue;
                }
                // Evaluate the anthem values using the affected creature's id
                // so AttachedToAffected counts attachments on *that* creature.
                let power = self.power.evaluate(game, obj_id, controller);
                let toughness = self.toughness.evaluate(game, obj_id, controller);
                effects.push(effect_with_optional_static_condition(
                    ContinuousEffect::new(
                        source,
                        controller,
                        EffectTarget::Specific(obj_id),
                        Modification::ModifyPowerToughness { power, toughness },
                    )
                    .with_source_type(EffectSourceType::StaticAbility),
                    &self.condition,
                ));
            }
            return effects;
        }

        let target = if self.source_only {
            EffectTarget::Source
        } else {
            effect_target_for_filter(source, &self.filter)
        };
        if (!matches!(self.power, AnthemValue::Fixed(_))
            || !matches!(self.toughness, AnthemValue::Fixed(_)))
            && let (Some(power), Some(toughness)) = (
                anthem_value_as_layer_value(&self.power),
                anthem_value_as_layer_value(&self.toughness),
            )
        {
            return vec![effect_with_optional_static_condition(
                ContinuousEffect::new(
                    source,
                    controller,
                    target,
                    Modification::ModifyPowerToughnessValue { power, toughness },
                )
                .with_source_type(EffectSourceType::StaticAbility),
                &self.condition,
            )];
        }

        let power = self.power.evaluate(game, source, controller);
        let toughness = self.toughness.evaluate(game, source, controller);
        vec![effect_with_optional_static_condition(
            ContinuousEffect::new(
                source,
                controller,
                target,
                Modification::ModifyPowerToughness { power, toughness },
            )
            .with_source_type(EffectSourceType::StaticAbility),
            &self.condition,
        )]
    }

    fn is_active(&self, game: &GameState, source: ObjectId) -> bool {
        let Some(condition) = &self.condition else {
            return true;
        };
        let Some(source_obj) = game.object(source) else {
            return false;
        };
        static_condition_is_active(condition, game, source, game.controller_of(source_obj))
    }

    fn is_anthem(&self) -> bool {
        true
    }
}

/// Grant ability: "Creatures you control have [ability]"
#[derive(Debug, Clone)]
pub struct GrantAbility {
    /// Filter for which permanents gain the ability.
    pub filter: ObjectFilter,
    /// If true, this grants only to the source object.
    pub source_only: bool,
    /// The ability to grant.
    pub ability: StaticAbility,
    /// Optional activation condition.
    pub condition: Option<crate::ConditionExpr>,
    /// Original leading set quantifier, retained only for compiled-text surface.
    pub set_quantifier_surface: Option<ironsmith_core::SetQuantifierSurface>,
}

impl GrantAbility {
    pub fn new(filter: ObjectFilter, ability: StaticAbility) -> Self {
        Self {
            filter,
            source_only: false,
            ability,
            condition: None,
            set_quantifier_surface: None,
        }
    }

    pub fn source(ability: StaticAbility) -> Self {
        Self {
            filter: ObjectFilter::creature(),
            source_only: true,
            ability,
            condition: None,
            set_quantifier_surface: None,
        }
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }

    pub fn with_set_quantifier_surface(
        mut self,
        surface: Option<ironsmith_core::SetQuantifierSurface>,
    ) -> Self {
        self.set_quantifier_surface = surface;
        self
    }

    fn applies_to_source(&self) -> bool {
        self.source_only || self.filter == ObjectFilter::source()
    }
}

impl PartialEq for GrantAbility {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter
            && self.source_only == other.source_only
            && self.ability == other.ability
            && self.condition == other.condition
            && self.set_quantifier_surface == other.set_quantifier_surface
    }
}

impl StaticAbilityKind for GrantAbility {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::GrantAbility
    }

    fn display(&self) -> String {
        let applies_to_source = self.applies_to_source();
        let (subject, explicitly_singular_subject) = if applies_to_source {
            ("this creature".to_string(), true)
        } else if let Some(subject) =
            exact_one_condition_antecedent_subject(&self.filter, self.condition.as_ref())
        {
            (subject, false)
        } else {
            grant_subject_with_set_quantifier(&self.filter, self.set_quantifier_surface)
        };
        if applies_to_source
            && self.ability.id() == StaticAbilityId::CanAttackAsThoughHaste
            && let Some(crate::ConditionExpr::Not(inner)) = &self.condition
            && matches!(
                inner.as_ref(),
                crate::ConditionExpr::ObjectEnteredBattlefieldThisTurn(filter)
                    if *filter == ObjectFilter::source()
            )
        {
            return format!(
                "{subject} can attack as though it had haste unless it entered this turn"
            );
        }
        let raw_ability_text = self.ability.display();
        let mut ability_text = raw_ability_text.clone();
        if self.ability.is_keyword() {
            ability_text = lowercase_first_ascii(&ability_text);
        }
        let is_quoted_cost_modifier = matches!(
            self.ability.id(),
            StaticAbilityId::CostReduction
                | StaticAbilityId::CostReductionManaCost
                | StaticAbilityId::CostIncrease
                | StaticAbilityId::CostIncreaseManaCost
        );
        if is_quoted_cost_modifier {
            ability_text = capitalize_first(&ability_text);
        }
        if matches!(
            ability_text.split_whitespace().next(),
            Some("If" | "When" | "Whenever" | "At")
        ) || self.ability.id() == StaticAbilityId::DungeonRoomTriggerDuplication
            || self.ability.id() == StaticAbilityId::RuleRestriction
            || is_quoted_cost_modifier
        {
            ability_text = format!("\"{ability_text}\"");
        }
        if self.condition.is_none()
            && self.filter.has_mana_source_spent_trailing_if_surface()
            && let Some(source_filter) = &self.filter.mana_from_source_spent_to_cast
        {
            let mut affected_filter = self.filter.clone();
            affected_filter.mana_from_source_spent_to_cast = None;
            affected_filter.set_mana_source_spent_trailing_if_surface(false);
            let (affected, singular) =
                grant_subject_with_set_quantifier(&affected_filter, self.set_quantifier_surface);
            let verb = if singular { "has" } else { "have" };
            let mana_source = with_indefinite_article_unless_present(source_filter.description());
            let mut rendered = format!(
                "{affected} {verb} {ability_text} if mana from {mana_source} was spent to cast it"
            );
            if self.ability.id() == StaticAbilityId::SplitSecond
                && !rendered.to_ascii_lowercase().contains("as long as")
            {
                rendered.push_str(
                    ". (As long as it's on the stack, players can't cast spells or activate abilities that aren't mana abilities.)",
                );
            }
            return rendered;
        }
        let singular_subject = explicitly_singular_subject
            || subject.starts_with("enchanted ")
            || subject.starts_with("equipped ")
            || subject.starts_with("this ")
            || subject.starts_with("that ");
        let ability_text_lower = ability_text.to_ascii_lowercase();
        let mut text = match self.ability.id() {
            StaticAbilityId::CanAttackAsThoughNoDefender => format!(
                "{subject} can attack as though {} didn't have defender",
                if singular_subject { "it" } else { "they" }
            ),
            StaticAbilityId::Unblockable => format!("{subject} can't be blocked"),
            StaticAbilityId::CantAttack => format!("{subject} can't attack"),
            StaticAbilityId::CantBlock => format!("{subject} can't block"),
            // These restrictions are complete verb phrases, not abilities
            // introduced by "has"/"have". Keeping them structural here also
            // gives token-carried anthems the right surface (for example,
            // "Creatures you control attack each combat if able").
            StaticAbilityId::MustAttack => format!(
                "{subject} {} each combat if able",
                if singular_subject {
                    "attacks"
                } else {
                    "attack"
                }
            ),
            StaticAbilityId::MustBlock => format!(
                "{subject} {} each combat if able",
                if singular_subject { "blocks" } else { "block" }
            ),
            _ if ability_text_lower.starts_with("can't ") => {
                format!("{subject} {}", lowercase_first_ascii(&ability_text))
            }
            _ => {
                let verb = if singular_subject { "has" } else { "have" };
                // Oracle quantifies unscoped grants ("All creatures with an
                // odd mana value have haste"); scoped subjects ("Creatures
                // you control ...") stay bare.
                let lower_subject = subject.to_ascii_lowercase();
                let already_quantified = singular_subject
                    || lower_subject.starts_with("all ")
                    || lower_subject.starts_with("each ")
                    || lower_subject.starts_with("other ")
                    || lower_subject.starts_with("another ");
                let scoped = [
                    " you control",
                    " you don't control",
                    " your team controls",
                    " an opponent controls",
                    " your opponents control",
                    " that player controls",
                    " you own",
                    " they control",
                    " you cast",
                    " spells",
                    " spell",
                ]
                .iter()
                .any(|suffix| lower_subject.contains(suffix));
                // Subtype-qualified anthems stay bare in oracle ("Cleric
                // creatures have vigilance"); only generic nouns quantify.
                let generic_noun_subject = matches!(
                    lower_subject.split_whitespace().next(),
                    Some(
                        "creature"
                            | "creatures"
                            | "permanent"
                            | "permanents"
                            | "artifact"
                            | "artifacts"
                            | "enchantment"
                            | "enchantments"
                            | "land"
                            | "lands"
                            | "planeswalker"
                            | "planeswalkers"
                            | "card"
                            | "cards"
                            | "token"
                            | "tokens"
                            | "nonland"
                            | "nontoken"
                            | "nonbasic"
                    )
                );
                if !already_quantified && !scoped && generic_noun_subject {
                    format!(
                        "All {} {verb} {ability_text}",
                        lowercase_first_ascii(&subject)
                    )
                } else {
                    format!("{subject} {verb} {ability_text}")
                }
            }
        };
        if let Some(condition) = &self.condition {
            if matches!(condition, crate::ConditionExpr::SourceControllersEndStep) {
                return format!("During your end step, {text}");
            }
            if applies_to_source
                && self.ability.is_keyword()
                && leading_source_keyword_condition(condition)
            {
                let condition_text = normalize_source_counter_condition_text(
                    &describe_same_source_static_condition(condition),
                );
                if let Some(rest) = condition_text.strip_prefix("as long as ") {
                    return format!("as long as {rest}, {subject} has {ability_text}");
                }
            }
            let condition_text = if applies_to_source {
                describe_same_source_static_condition(condition)
            } else {
                describe_static_condition(condition)
            };
            if static_condition_is_during_your_turn(condition) {
                return format!("During your turn, {text}");
            }
            if let Some(rest) = condition_text.strip_prefix("as long as ") {
                if applies_to_source {
                    return format!("{text} as long as {rest}");
                }
                return format!("as long as {rest}, {text}");
            }
            text.push(' ');
            text.push_str(&condition_text);
        }
        text
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
    }

    fn grants_abilities(&self) -> bool {
        true
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        let target = if self.source_only {
            EffectTarget::Source
        } else if self.filter == ObjectFilter::source() {
            EffectTarget::Source
        } else {
            effect_target_for_filter(source, &self.filter)
        };
        vec![effect_with_optional_static_condition(
            ContinuousEffect::new(
                source,
                controller,
                target,
                Modification::AddAbility(self.ability.clone()),
            )
            .with_source_type(EffectSourceType::StaticAbility),
            &self.condition,
        )]
    }

    fn apply_restrictions(&self, game: &mut GameState, _source: ObjectId, controller: PlayerId) {
        if self.applies_to_source() {
            self.ability.apply_restrictions(game, _source, controller);
            return;
        }

        // Find permanents matching the filter
        let filter_ctx = game.filter_context_for(controller, None);
        let matching: Vec<ObjectId> = game
            .battlefield
            .iter()
            .filter(|&&id| {
                game.object(id)
                    .map(|obj| self.filter.matches(obj, &filter_ctx, game))
                    .unwrap_or(false)
            })
            .copied()
            .collect();

        // Apply the granted ability's restrictions to each matching permanent
        for perm_id in matching {
            self.ability.apply_restrictions(game, perm_id, controller);
        }
    }

    fn is_active(&self, game: &GameState, source: ObjectId) -> bool {
        let Some(condition) = &self.condition else {
            return true;
        };
        let Some(source_obj) = game.object(source) else {
            return false;
        };
        static_condition_is_active(condition, game, source, game.controller_of(source_obj))
    }
}

fn leading_source_keyword_condition(condition: &crate::ConditionExpr) -> bool {
    match condition {
        crate::ConditionExpr::OwnsCardExiledWithCounter(_) => true,
        crate::ConditionExpr::TurnHistory(
            ironsmith_core::TurnHistoryCondition::PlayerVisitedAttractionThisTurn(_),
        ) => true,
        crate::ConditionExpr::CountComparison {
            display: Some(display),
            ..
        } => {
            display.starts_with("you own a card exiled with ")
                || display.starts_with("this has ")
                || display.starts_with("this creature has ")
        }
        _ => false,
    }
}

fn static_condition_is_during_your_turn(condition: &crate::ConditionExpr) -> bool {
    matches!(
        condition,
        crate::ConditionExpr::YourTurn
            | crate::ConditionExpr::ActivationTiming(
                crate::ability::ActivationTiming::DuringYourTurn
            )
    )
}

fn normalize_source_counter_condition_text(condition_text: &str) -> String {
    condition_text
        .strip_prefix("as long as this has ")
        .map(|rest| format!("as long as this creature has {rest}"))
        .unwrap_or_else(|| condition_text.to_string())
}

#[derive(Debug, Clone, PartialEq)]
pub enum SoulbondSharedMode {
    PowerToughness { power: i32, toughness: i32 },
    Ability(StaticAbility),
    ObjectAbility(Ability),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SoulbondSharedBonus {
    pub mode: SoulbondSharedMode,
}

impl SoulbondSharedBonus {
    pub fn power_toughness(power: i32, toughness: i32) -> Self {
        Self {
            mode: SoulbondSharedMode::PowerToughness { power, toughness },
        }
    }

    pub fn ability(ability: StaticAbility) -> Self {
        Self {
            mode: SoulbondSharedMode::Ability(ability),
        }
    }

    pub fn object_ability(ability: Ability) -> Self {
        Self {
            mode: SoulbondSharedMode::ObjectAbility(ability),
        }
    }
}

impl StaticAbilityKind for SoulbondSharedBonus {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SoulbondSharedBonus
    }

    fn display(&self) -> String {
        match &self.mode {
            SoulbondSharedMode::PowerToughness { power, toughness } => {
                let signed = |value: i32| {
                    if value >= 0 {
                        format!("+{value}")
                    } else {
                        value.to_string()
                    }
                };
                format!(
                    "As long as this creature is paired with another creature, each of those creatures gets {}/{}",
                    signed(*power),
                    signed(*toughness)
                )
            }
            SoulbondSharedMode::Ability(ability) => format!(
                "As long as this creature is paired with another creature, both creatures have {}",
                ability.display()
            ),
            SoulbondSharedMode::ObjectAbility(ability) => {
                let text = match &ability.kind {
                    AbilityKind::Static(static_ability) => static_ability.display(),
                    AbilityKind::Triggered(triggered) => {
                        format!("a triggered ability ({})", triggered.trigger.display())
                    }
                    AbilityKind::Activated(_) => "an activated ability".to_string(),
                };
                format!(
                    "As long as this creature is paired with another creature, both creatures have \"{}\"",
                    text
                )
            }
        }
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        game: &GameState,
    ) -> Vec<ContinuousEffect> {
        let Some(partner) = game.soulbond_partner_for_shared_bonus(source) else {
            return Vec::new();
        };

        let modification = |mode: &SoulbondSharedMode| match mode {
            SoulbondSharedMode::PowerToughness { power, toughness } => {
                Modification::ModifyPowerToughness {
                    power: *power,
                    toughness: *toughness,
                }
            }
            SoulbondSharedMode::Ability(ability) => Modification::AddAbility(ability.clone()),
            SoulbondSharedMode::ObjectAbility(ability) => {
                Modification::AddAbilityGeneric(ability.clone())
            }
        };

        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Specific(source),
                modification(&self.mode),
            )
            .with_source_type(EffectSourceType::StaticAbility),
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Specific(partner),
                modification(&self.mode),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }

    fn apply_restrictions(&self, game: &mut GameState, source: ObjectId, controller: PlayerId) {
        let SoulbondSharedMode::Ability(ability) = &self.mode else {
            return;
        };
        let Some(partner) = game.soulbond_partner_for_shared_bonus(source) else {
            return;
        };
        ability.apply_restrictions(game, source, controller);
        ability.apply_restrictions(game, partner, controller);
    }

    fn granted_inline_ability(&self) -> Option<&crate::ability::Ability> {
        match &self.mode {
            SoulbondSharedMode::ObjectAbility(ability) => Some(ability),
            _ => None,
        }
    }
}

/// Remove ability: "Creatures lose [ability]"
#[derive(Debug, Clone)]
pub struct RemoveAbilityForFilter {
    /// Filter for which permanents lose the ability.
    pub filter: ObjectFilter,
    /// The ability to remove.
    pub abilities: Vec<Ability>,
    pub display: String,
    pub mode: ironsmith_core::AbilityLossMode,
    pub condition: Option<crate::ConditionExpr>,
}

impl RemoveAbilityForFilter {
    pub fn new(filter: ObjectFilter, ability: StaticAbility) -> Self {
        Self::new_with_mode(filter, ability, ironsmith_core::AbilityLossMode::Lose)
    }

    pub fn new_with_mode(
        filter: ObjectFilter,
        ability: StaticAbility,
        mode: ironsmith_core::AbilityLossMode,
    ) -> Self {
        let display = ability.display();
        Self {
            filter,
            abilities: vec![Ability::static_ability(ability)],
            display,
            mode,
            condition: None,
        }
    }

    pub fn object_abilities(
        filter: ObjectFilter,
        abilities: Vec<Ability>,
        display: String,
    ) -> Self {
        Self::object_abilities_with_mode(
            filter,
            abilities,
            display,
            ironsmith_core::AbilityLossMode::Lose,
        )
    }

    pub fn object_abilities_with_mode(
        filter: ObjectFilter,
        abilities: Vec<Ability>,
        display: String,
        mode: ironsmith_core::AbilityLossMode,
    ) -> Self {
        Self {
            filter,
            abilities,
            display,
            mode,
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

impl PartialEq for RemoveAbilityForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter
            && self.abilities == other.abilities
            && self.display == other.display
            && self.mode == other.mode
            && self.condition == other.condition
    }
}

impl StaticAbilityKind for RemoveAbilityForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::RemoveAbilityForFilter
    }

    fn display(&self) -> String {
        if self
            .abilities
            .first()
            .and_then(|ability| match &ability.kind {
                AbilityKind::Static(ability) => ability.landwalk_kind(),
                _ => None,
            })
            .is_some()
            && self.filter.card_types.len() == 1
            && self.filter.card_types[0] == crate::types::CardType::Creature
            && self
                .filter
                .ability_markers
                .iter()
                .any(|marker| marker.eq_ignore_ascii_case(&self.display))
        {
            return format!(
                "{} can be blocked as though they didn't have {}",
                pluralized_subject_text(&self.filter),
                self.display.to_ascii_lowercase()
            );
        }
        let subject = pluralized_subject_text(&self.filter);
        let singular_subject = subject.starts_with("enchanted ")
            || subject.starts_with("equipped ")
            || subject.starts_with("this ")
            || subject.starts_with("that ");
        let verb = if singular_subject { "loses" } else { "lose" };
        // Keyword names render lowercase mid-sentence ("lose trample", not
        // "lose Trample"); non-keyword displays (quoted ability text) keep
        // their original casing.
        let ability_text = if self.abilities.iter().all(object_ability_is_static_keyword) {
            self.display.to_ascii_lowercase()
        } else {
            self.display.clone()
        };
        let suffix = match self.mode {
            ironsmith_core::AbilityLossMode::Lose => String::new(),
            ironsmith_core::AbilityLossMode::LoseAndCantGain => {
                format!(" and can't gain {ability_text}")
            }
            ironsmith_core::AbilityLossMode::LoseAndCantHaveOrGain => {
                format!(" and can't have or gain {ability_text}")
            }
        };
        let text = format!("{subject} {verb} {ability_text}{suffix}");
        if let Some(condition) = &self.condition {
            let condition_text = describe_static_condition(condition);
            if let Some(rest) = condition_text.strip_prefix("as long as ") {
                if subject.eq_ignore_ascii_case("enchanted permanent") {
                    return format!("As long as {rest}, it {verb} {ability_text}{suffix}");
                }
                return format!("As long as {rest}, {text}");
            }
            return format!("{text} {condition_text}");
        }
        text
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        self.abilities
            .iter()
            .cloned()
            .map(|ability| {
                effect_with_optional_static_condition(
                    ContinuousEffect::new(
                        source,
                        controller,
                        EffectTarget::Filter(self.filter.clone()),
                        Modification::RemoveAbilityGeneric {
                            ability,
                            mode: self.mode,
                        },
                    )
                    .with_source_type(EffectSourceType::StaticAbility),
                    &self.condition,
                )
            })
            .collect()
    }
}

/// Remove all abilities: "Creatures lose all abilities"
#[derive(Debug, Clone, PartialEq)]
pub struct RemoveAllAbilitiesForFilter {
    /// Filter for which permanents lose all abilities.
    pub filter: ObjectFilter,
}

impl RemoveAllAbilitiesForFilter {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl StaticAbilityKind for RemoveAllAbilitiesForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::RemoveAllAbilitiesForFilter
    }

    fn structural_effect_filter(&self) -> Option<&ObjectFilter> {
        Some(&self.filter)
    }

    fn display(&self) -> String {
        let (subject, _) =
            grant_subject_with_set_quantifier(&self.filter, self.filter.set_quantifier_surface());
        let (copula, _) = subject_verb_and_possessive(&subject);
        let verb = if copula == "is" { "loses" } else { "lose" };
        format!("{subject} {verb} all abilities")
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Filter(self.filter.clone()),
                Modification::RemoveAllAbilities,
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// Remove all non-mana abilities: "Lands lose all abilities except mana abilities"
#[derive(Debug, Clone, PartialEq)]
pub struct RemoveAllAbilitiesExceptManaForFilter {
    /// Filter for which permanents lose non-mana abilities.
    pub filter: ObjectFilter,
}

impl RemoveAllAbilitiesExceptManaForFilter {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl StaticAbilityKind for RemoveAllAbilitiesExceptManaForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::RemoveAllAbilitiesExceptManaForFilter
    }

    fn display(&self) -> String {
        let (subject, _) =
            grant_subject_with_set_quantifier(&self.filter, self.filter.set_quantifier_surface());
        let (copula, _) = subject_verb_and_possessive(&subject);
        let verb = if copula == "is" { "loses" } else { "lose" };
        format!("{subject} {verb} all abilities except mana abilities")
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Filter(self.filter.clone()),
                Modification::RemoveAllAbilitiesExceptMana,
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// Set base P/T: "... have base power and toughness N/M"
#[derive(Debug, Clone, PartialEq)]
pub struct SetBasePowerToughnessForFilter {
    /// Filter for which permanents get base P/T set.
    pub filter: ObjectFilter,
    /// Base power value.
    pub power: i32,
    /// Base toughness value.
    pub toughness: i32,
    pub condition: Option<crate::ConditionExpr>,
}

impl SetBasePowerToughnessForFilter {
    pub fn new(filter: ObjectFilter, power: i32, toughness: i32) -> Self {
        Self {
            filter,
            power,
            toughness,
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

impl StaticAbilityKind for SetBasePowerToughnessForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SetBasePowerToughnessForFilter
    }

    fn display(&self) -> String {
        let subject = pluralized_subject_text(&self.filter);
        let singular = subject.starts_with("enchanted ")
            || subject.starts_with("equipped ")
            || subject.starts_with("this ")
            || subject.starts_with("that ");
        let verb = if singular { "has" } else { "have" };
        let mut text = format!(
            "{subject} {verb} base power and toughness {}/{}",
            self.power, self.toughness
        );
        if let Some(condition) = &self.condition {
            if matches!(
                condition,
                crate::ConditionExpr::EnchantedPermanentIsCreature
                    | crate::ConditionExpr::EnchantedPermanentIsLand
                    | crate::ConditionExpr::EnchantedPermanentIsEquipment
                    | crate::ConditionExpr::EnchantedPermanentIsVehicle
            ) {
                let condition_text = describe_static_condition(condition);
                if let Some(condition_body) = condition_text.strip_prefix("as long as ") {
                    if subject.eq_ignore_ascii_case("enchanted permanent") {
                        return format!(
                            "As long as {condition_body}, it has base power and toughness {}/{}",
                            self.power, self.toughness
                        );
                    }
                    return format!("As long as {condition_body}, {text}");
                }
            }
            if static_condition_is_during_your_turn(condition) {
                return format!("During your turn, {text}");
            }
            text.push(' ');
            text.push_str(&describe_static_condition(condition));
        }
        text
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![effect_with_optional_static_condition(
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Filter(self.filter.clone()),
                Modification::SetPowerToughness {
                    power: Value::Fixed(self.power),
                    toughness: Value::Fixed(self.toughness),
                    sublayer: PtSublayer::Setting,
                },
            )
            .with_source_type(EffectSourceType::StaticAbility),
            &self.condition,
        )]
    }
}

/// Set only base power, leaving base toughness unchanged.
#[derive(Debug, Clone, PartialEq)]
pub struct SetBasePowerForFilter {
    pub filter: ObjectFilter,
    pub power: i32,
    pub condition: Option<crate::ConditionExpr>,
}

impl SetBasePowerForFilter {
    pub fn new(filter: ObjectFilter, power: i32) -> Self {
        Self {
            filter,
            power,
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

impl StaticAbilityKind for SetBasePowerForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SetBasePowerToughnessForFilter
    }

    fn display(&self) -> String {
        let subject = pluralized_subject_text(&self.filter);
        let singular = subject.starts_with("enchanted ")
            || subject.starts_with("equipped ")
            || subject.starts_with("this ")
            || subject.starts_with("that ");
        let verb = if singular { "has" } else { "have" };
        let mut text = format!("{subject} {verb} base power {}", self.power);
        if let Some(condition) = &self.condition {
            text.push(' ');
            text.push_str(&describe_static_condition(condition));
        }
        text
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![effect_with_optional_static_condition(
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Filter(self.filter.clone()),
                Modification::SetPower {
                    value: Value::Fixed(self.power),
                    sublayer: PtSublayer::Setting,
                },
            )
            .with_source_type(EffectSourceType::StaticAbility),
            &self.condition,
        )]
    }
}

/// Set dynamic base P/T: "... have base power and toughness each equal to ..."
#[derive(Debug, Clone, PartialEq)]
pub struct SetBasePowerToughnessValueForFilter {
    /// Filter for which permanents get base P/T set.
    pub filter: ObjectFilter,
    /// Base power value.
    pub power: Value,
    /// Base toughness value.
    pub toughness: Value,
    pub condition: Option<crate::ConditionExpr>,
}

impl SetBasePowerToughnessValueForFilter {
    pub fn new(filter: ObjectFilter, power: Value, toughness: Value) -> Self {
        Self {
            filter,
            power,
            toughness,
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

fn describe_static_iterated_value(value: &Value, possessive: &str) -> String {
    if let Value::ManaValueOf(spec) = value.unhinted()
        && matches!(spec.base(), ChooseSpec::Iterated)
    {
        return format!("{possessive} mana value");
    }
    crate::runtime_display::describe_value(value)
}

impl StaticAbilityKind for SetBasePowerToughnessValueForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SetBasePowerToughnessForFilter
    }

    fn display(&self) -> String {
        let (subject, _) =
            grant_subject_with_set_quantifier(&self.filter, self.filter.set_quantifier_surface());
        let (copula, possessive) = subject_verb_and_possessive(&subject);
        let verb = if copula == "is" { "has" } else { "have" };
        let mut text = if self.power == self.toughness {
            format!(
                "{subject} {verb} base power and base toughness each equal to {}",
                describe_static_iterated_value(&self.power, possessive)
            )
        } else {
            format!(
                "{subject} {verb} base power {} and base toughness {}",
                describe_static_iterated_value(&self.power, possessive),
                describe_static_iterated_value(&self.toughness, possessive)
            )
        };
        if let Some(condition) = &self.condition {
            text.push(' ');
            text.push_str(&describe_static_condition(condition));
        }
        text
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![effect_with_optional_static_condition(
            ContinuousEffect::new(
                source,
                controller,
                effect_target_for_filter(source, &self.filter),
                Modification::SetPowerToughness {
                    power: self.power.clone(),
                    toughness: self.toughness.clone(),
                    sublayer: PtSublayer::Setting,
                },
            )
            .with_source_type(EffectSourceType::StaticAbility),
            &self.condition,
        )]
    }
}

/// Copy activated abilities from objects matching a filter.
#[derive(Debug, Clone, PartialEq)]
pub struct CopyActivatedAbilities {
    pub filter: ObjectFilter,
    pub counter: Option<CounterType>,
    pub include_mana: bool,
    pub only_loyalty: bool,
    pub exclude_source_name: bool,
    pub exclude_source_id: bool,
    pub force_once_each_turn: bool,
    pub condition: Option<crate::ConditionExpr>,
    pub display: String,
}

impl CopyActivatedAbilities {
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            counter: None,
            include_mana: true,
            only_loyalty: false,
            exclude_source_name: false,
            exclude_source_id: true,
            force_once_each_turn: false,
            condition: None,
            display: "Has all activated abilities of matching objects".to_string(),
        }
    }

    pub fn with_counter(mut self, counter: CounterType) -> Self {
        self.counter = Some(counter);
        self
    }

    pub fn with_only_loyalty(mut self) -> Self {
        self.only_loyalty = true;
        self
    }

    pub fn with_exclude_source_name(mut self, exclude: bool) -> Self {
        self.exclude_source_name = exclude;
        self
    }

    pub fn with_exclude_source_id(mut self, exclude: bool) -> Self {
        self.exclude_source_id = exclude;
        self
    }

    pub fn with_once_each_turn(mut self) -> Self {
        self.force_once_each_turn = true;
        self
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }

    pub fn with_display(mut self, display: String) -> Self {
        self.display = display;
        self
    }
}

impl StaticAbilityKind for CopyActivatedAbilities {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CopyActivatedAbilities
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![effect_with_optional_static_condition(
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Source,
                Modification::CopyActivatedAbilities {
                    filter: self.filter.clone(),
                    counter: self.counter,
                    include_mana: self.include_mana,
                    only_loyalty: self.only_loyalty,
                    exclude_source_name: self.exclude_source_name,
                    exclude_source_id: self.exclude_source_id,
                    force_once_each_turn: self.force_once_each_turn,
                },
            )
            .with_source_type(EffectSourceType::StaticAbility),
            &self.condition,
        )]
    }

    fn is_active(&self, game: &GameState, source: ObjectId) -> bool {
        let Some(condition) = &self.condition else {
            return true;
        };

        let Some(source_obj) = game.object(source) else {
            return false;
        };
        static_condition_is_active(condition, game, source, game.controller_of(source_obj))
    }
}

/// Inherit complete static-ability variants from objects matching a filter.
#[derive(Debug, Clone, PartialEq)]
pub struct CopyStaticAbilityVariants {
    pub filter: ObjectFilter,
    pub selectors: Vec<ironsmith_core::StaticAbilityVariantSelector>,
    pub exclude_source_id: bool,
    pub condition: Option<crate::ConditionExpr>,
    pub display: String,
}

impl CopyStaticAbilityVariants {
    pub fn new(
        filter: ObjectFilter,
        selectors: Vec<ironsmith_core::StaticAbilityVariantSelector>,
        display: String,
    ) -> Self {
        Self {
            filter,
            selectors,
            exclude_source_id: true,
            condition: None,
            display,
        }
    }

    pub fn with_exclude_source_id(mut self, exclude: bool) -> Self {
        self.exclude_source_id = exclude;
        self
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

impl StaticAbilityKind for CopyStaticAbilityVariants {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CopyStaticAbilityVariants
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![effect_with_optional_static_condition(
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Source,
                Modification::CopyStaticAbilityVariants {
                    filter: self.filter.clone(),
                    selectors: self.selectors.clone(),
                    exclude_source_id: self.exclude_source_id,
                },
            )
            .with_source_type(EffectSourceType::StaticAbility),
            &self.condition,
        )]
    }

    fn is_active(&self, game: &GameState, source: ObjectId) -> bool {
        let Some(condition) = &self.condition else {
            return true;
        };
        let Some(source_obj) = game.object(source) else {
            return false;
        };
        static_condition_is_active(condition, game, source, game.controller_of(source_obj))
    }
}

/// Copy triggered abilities from objects matching a filter.
#[derive(Debug, Clone, PartialEq)]
pub struct CopyTriggeredAbilities {
    pub filter: ObjectFilter,
    pub exclude_source_name: bool,
    pub exclude_source_id: bool,
    pub condition: Option<crate::ConditionExpr>,
    pub display: String,
}

impl CopyTriggeredAbilities {
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            exclude_source_name: false,
            exclude_source_id: true,
            condition: None,
            display: "Has all triggered abilities of matching objects".to_string(),
        }
    }

    pub fn with_exclude_source_name(mut self, exclude: bool) -> Self {
        self.exclude_source_name = exclude;
        self
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }

    pub fn with_display(mut self, display: String) -> Self {
        self.display = display;
        self
    }
}

impl StaticAbilityKind for CopyTriggeredAbilities {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CopyTriggeredAbilities
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![effect_with_optional_static_condition(
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Source,
                Modification::CopyTriggeredAbilities {
                    filter: self.filter.clone(),
                    exclude_source_name: self.exclude_source_name,
                    exclude_source_id: self.exclude_source_id,
                },
            )
            .with_source_type(EffectSourceType::StaticAbility),
            &self.condition,
        )]
    }

    fn is_active(&self, game: &GameState, source: ObjectId) -> bool {
        let Some(condition) = &self.condition else {
            return true;
        };

        let Some(source_obj) = game.object(source) else {
            return false;
        };
        static_condition_is_active(condition, game, source, game.controller_of(source_obj))
    }
}

/// Equipment grant: "Equipped creature has [abilities]"
#[derive(Debug, Clone)]
pub struct EquipmentGrant {
    /// The abilities to grant to the equipped creature.
    pub abilities: Vec<StaticAbility>,
}

/// Set colors: "All creatures are black."
#[derive(Debug, Clone)]
pub struct SetColorsForFilter {
    pub filter: ObjectFilter,
    pub colors: crate::color::ColorSet,
    pub condition: Option<crate::ConditionExpr>,
}

impl SetColorsForFilter {
    pub fn new(filter: ObjectFilter, colors: crate::color::ColorSet) -> Self {
        Self {
            filter,
            colors,
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

impl PartialEq for SetColorsForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter
            && self.colors == other.colors
            && self.condition == other.condition
    }
}

impl StaticAbilityKind for SetColorsForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SetColors
    }

    fn structural_effect_filter(&self) -> Option<&ObjectFilter> {
        Some(&self.filter)
    }

    fn display(&self) -> String {
        let subject = pluralized_subject_text(&self.filter);
        let (verb, _) = subject_verb_and_possessive(&subject);
        if is_all_colors(self.colors) {
            if verb == "are" {
                let each_subject = lowercase_first_ascii(strip_plural_subject_article(
                    &subject_text(&self.filter),
                ));
                let mut text = format!("Each {each_subject} is all colors");
                if let Some(condition) = &self.condition {
                    if static_condition_is_during_your_turn(condition) {
                        return format!("During your turn, {text}");
                    }
                    text.push(' ');
                    text.push_str(&describe_static_condition(condition));
                }
                return text;
            }
            let mut text = format!("{subject} {verb} all colors");
            if let Some(condition) = &self.condition {
                if static_condition_is_during_your_turn(condition) {
                    return format!("During your turn, {text}");
                }
                text.push(' ');
                text.push_str(&describe_static_condition(condition));
            }
            return text;
        }
        let colors = join_with_and(&color_list(self.colors));
        let mut text = format!("{subject} {verb} {colors}");
        if let Some(condition) = &self.condition {
            if static_condition_is_during_your_turn(condition) {
                return format!("During your turn, {text}");
            }
            text.push(' ');
            text.push_str(&describe_static_condition(condition));
        }
        text
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![effect_with_optional_static_condition(
            ContinuousEffect::new(
                source,
                controller,
                effect_target_for_filter(source, &self.filter),
                Modification::SetColors(self.colors),
            )
            .with_source_type(EffectSourceType::StaticAbility),
            &self.condition,
        )]
    }
}

/// Set name: "Enchanted creature is named Legitimate Businessperson."
#[derive(Debug, Clone, PartialEq)]
pub struct SetNameForFilter {
    pub filter: ObjectFilter,
    pub name: String,
}

impl SetNameForFilter {
    pub fn new(filter: ObjectFilter, name: String) -> Self {
        Self { filter, name }
    }
}

impl StaticAbilityKind for SetNameForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SetName
    }

    fn display(&self) -> String {
        let subject = pluralized_subject_text(&self.filter);
        let (verb, _) = subject_verb_and_possessive(&subject);
        format!("{subject} {verb} named {}", self.name)
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                effect_target_for_filter(source, &self.filter),
                Modification::SetName(self.name.to_string()),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// Add colors: "Enchanted creature is black in addition to its other colors."
#[derive(Debug, Clone)]
pub struct AddColorsForFilter {
    pub filter: ObjectFilter,
    pub colors: crate::color::ColorSet,
}

impl AddColorsForFilter {
    pub fn new(filter: ObjectFilter, colors: crate::color::ColorSet) -> Self {
        Self { filter, colors }
    }
}

impl PartialEq for AddColorsForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter && self.colors == other.colors
    }
}

impl StaticAbilityKind for AddColorsForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::AddColors
    }

    fn display(&self) -> String {
        let subject = pluralized_subject_text(&self.filter);
        let (verb, possessive) = subject_verb_and_possessive(&subject);
        let colors = join_with_and(&color_list(self.colors));
        format!("{subject} {verb} {colors} in addition to {possessive} other colors")
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                effect_target_for_filter(source, &self.filter),
                Modification::AddColors(self.colors),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// Add card types: "All permanents are artifacts in addition to their other types."
#[derive(Debug, Clone)]
pub struct AddCardTypesForFilter {
    pub filter: ObjectFilter,
    pub card_types: Vec<CardType>,
    pub condition: Option<crate::ConditionExpr>,
}

impl AddCardTypesForFilter {
    pub fn new(filter: ObjectFilter, card_types: Vec<CardType>) -> Self {
        Self {
            filter,
            card_types,
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

impl PartialEq for AddCardTypesForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter
            && self.card_types == other.card_types
            && self.condition == other.condition
    }
}

impl StaticAbilityKind for AddCardTypesForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::AddCardTypes
    }

    fn display(&self) -> String {
        let (subject, _) =
            grant_subject_with_set_quantifier(&self.filter, self.filter.set_quantifier_surface());
        let (verb, possessive) = subject_verb_and_possessive(&subject);
        let types = self
            .card_types
            .iter()
            .map(|card_type| {
                let name = card_type.name();
                if verb == "are" {
                    simple_pluralize(name)
                } else {
                    name.to_string()
                }
            })
            .collect::<Vec<_>>();
        let types = if self.filter.set_quantifier_surface()
            == Some(ironsmith_core::SetQuantifierSurface::Each)
            && verb == "is"
            && types.len() == 1
        {
            format!("{} {}", indefinite_article_for(&types[0]), types[0])
        } else {
            join_with_and(&types)
        };
        let mut text = format!(
            "{subject} {verb} {} in addition to {possessive} other types",
            types
        );
        if let Some(condition) = &self.condition {
            if static_condition_is_during_your_turn(condition) {
                return format!("During your turn, {text}");
            }
            text.push(' ');
            text.push_str(&describe_static_condition(condition));
        }
        text
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![effect_with_optional_static_condition(
            ContinuousEffect::new(
                source,
                controller,
                effect_target_for_filter(source, &self.filter),
                Modification::AddCardTypes(self.card_types.clone()),
            )
            .with_source_type(EffectSourceType::StaticAbility),
            &self.condition,
        )]
    }
}

/// Remove card types: "This creature isn't a creature."
#[derive(Debug, Clone)]
pub struct RemoveCardTypesForFilter {
    pub filter: ObjectFilter,
    pub card_types: Vec<CardType>,
    pub condition: Option<crate::ConditionExpr>,
}

impl RemoveCardTypesForFilter {
    pub fn new(filter: ObjectFilter, card_types: Vec<CardType>) -> Self {
        Self {
            filter,
            card_types,
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

impl PartialEq for RemoveCardTypesForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter
            && self.card_types == other.card_types
            && self.condition == other.condition
    }
}

impl StaticAbilityKind for RemoveCardTypesForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::RemoveCardTypes
    }

    fn display(&self) -> String {
        if self.filter.source && self.card_types == [CardType::Creature] {
            let mut text = "this card isn't a creature".to_string();
            if let Some(condition) = &self.condition {
                if static_condition_is_during_your_turn(condition) {
                    return format!("During your turn, {text}");
                }
                text.push(' ');
                text.push_str(&describe_static_condition(condition));
            }
            return text;
        }
        let subject = pluralized_subject_text(&self.filter);
        let (verb, _) = subject_verb_and_possessive(&subject);
        let types = self
            .card_types
            .iter()
            .map(|card_type| {
                let name = card_type.name();
                if verb == "are" {
                    simple_pluralize(name)
                } else {
                    name.to_string()
                }
            })
            .collect::<Vec<_>>();
        let mut text = format!("{subject} {verb} no longer {}", join_with_and(&types));
        if let Some(condition) = &self.condition {
            if static_condition_is_during_your_turn(condition) {
                return format!("During your turn, {text}");
            }
            text.push(' ');
            text.push_str(&describe_static_condition(condition));
        }
        text
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![effect_with_optional_static_condition(
            ContinuousEffect::new(
                source,
                controller,
                effect_target_for_filter(source, &self.filter),
                Modification::RemoveCardTypes(self.card_types.clone()),
            )
            .with_source_type(EffectSourceType::StaticAbility),
            &self.condition,
        )]
    }
}

/// Set card types: "Enchanted permanent is a creature."
#[derive(Debug, Clone)]
pub struct SetCardTypesForFilter {
    pub filter: ObjectFilter,
    pub card_types: Vec<CardType>,
    pub condition: Option<crate::ConditionExpr>,
}

impl SetCardTypesForFilter {
    pub fn new(filter: ObjectFilter, card_types: Vec<CardType>) -> Self {
        Self {
            filter,
            card_types,
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

impl PartialEq for SetCardTypesForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter
            && self.card_types == other.card_types
            && self.condition == other.condition
    }
}

impl StaticAbilityKind for SetCardTypesForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SetCardTypes
    }

    fn display(&self) -> String {
        let (subject, _) =
            grant_subject_with_set_quantifier(&self.filter, self.filter.set_quantifier_surface());
        let (verb, _) = subject_verb_and_possessive(&subject);
        let mut types = self
            .card_types
            .iter()
            .map(|card_type| card_type.name().to_string())
            .collect::<Vec<_>>();
        if verb == "are"
            && let Some(last) = types.last_mut()
        {
            *last = simple_pluralize(last);
        }
        let type_phrase = types.join(" ");
        let type_phrase = if verb == "are" || type_phrase.is_empty() {
            type_phrase
        } else {
            format!("{} {type_phrase}", indefinite_article_for(&type_phrase))
        };
        let mut text = format!("{subject} {verb} {type_phrase}");
        if let Some(condition) = &self.condition {
            let condition_text =
                normalize_source_counter_condition_text(&describe_static_condition(condition));
            if static_condition_is_during_your_turn(condition) {
                return format!("During your turn, {text}");
            }
            if condition_text.starts_with("as long as ") {
                return format!("{condition_text}, {text}");
            }
            text.push(' ');
            text.push_str(&condition_text);
        }
        text
    }

    fn prefers_card_name_subject(&self) -> bool {
        self.filter.source && self.filter.source_surface.is_none()
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![effect_with_optional_static_condition(
            ContinuousEffect::new(
                source,
                controller,
                effect_target_for_filter(source, &self.filter),
                Modification::SetCardTypes(self.card_types.clone()),
            )
            .with_source_type(EffectSourceType::StaticAbility),
            &self.condition,
        )]
    }
}

/// Add subtypes: "Enchanted creature is a Zombie in addition to its other types."
#[derive(Debug, Clone)]
pub struct AddSubtypesForFilter {
    pub filter: ObjectFilter,
    pub subtypes: Vec<Subtype>,
    pub condition: Option<crate::ConditionExpr>,
}

impl AddSubtypesForFilter {
    pub fn new(filter: ObjectFilter, subtypes: Vec<Subtype>) -> Self {
        Self {
            filter,
            subtypes,
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

impl PartialEq for AddSubtypesForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter
            && self.subtypes == other.subtypes
            && self.condition == other.condition
    }
}

impl StaticAbilityKind for AddSubtypesForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::AddSubtypes
    }

    fn display(&self) -> String {
        if self.filter == ObjectFilter::land()
            && let [subtype] = self.subtypes.as_slice()
            && subtype.is_land_subtype()
        {
            let subtype = subtype.to_string();
            let mut text = format!(
                "Each land is {} {subtype} in addition to its other land types",
                indefinite_article_for(&subtype)
            );
            if let Some(condition) = &self.condition {
                let condition_text =
                    normalize_source_counter_condition_text(&describe_static_condition(condition));
                if static_condition_is_during_your_turn(condition) {
                    return format!("During your turn, {text}");
                }
                if condition_text.starts_with("as long as ") {
                    return format!("{condition_text}, {text}");
                }
                text.push(' ');
                text.push_str(&condition_text);
            }
            return text;
        }

        let subject = if self.filter == ObjectFilter::source() {
            "this creature".to_string()
        } else {
            pluralized_subject_text(&self.filter)
        };
        let (verb, possessive) = subject_verb_and_possessive(&subject);
        if is_exactly_basic_land_types(&self.subtypes) {
            let mut text = format!(
                "{subject} {verb} every basic land type in addition to {possessive} other types"
            );
            if let Some(condition) = &self.condition {
                let condition_text =
                    normalize_source_counter_condition_text(&describe_static_condition(condition));
                if static_condition_is_during_your_turn(condition) {
                    return format!("During your turn, {text}");
                }
                if condition_text.starts_with("as long as ") {
                    return format!("{condition_text}, {text}");
                }
                text.push(' ');
                text.push_str(&condition_text);
            }
            return text;
        }
        let subtype_words = self
            .subtypes
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>();
        let base_phrase = subtype_words.join(" ");
        let subtype_phrase = if verb == "are" {
            pluralize_terminal_word(&base_phrase)
        } else if let Some(first) = subtype_words.first() {
            format!("{} {base_phrase}", indefinite_article_for(first))
        } else {
            base_phrase
        };
        let filter_subject = subject_text(&self.filter);
        let (base, suffix) = split_subject_suffix(&filter_subject);
        let filter_is_single_creature_type = suffix.is_empty()
            && !base.contains(' ')
            && base
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase());
        let adding_creature_types = self.subtypes.iter().all(Subtype::is_creature_type);
        let other_types = if filter_is_single_creature_type && adding_creature_types {
            "other creature types"
        } else {
            "other types"
        };
        let mut text =
            format!("{subject} {verb} {subtype_phrase} in addition to {possessive} {other_types}",);
        if let Some(condition) = &self.condition {
            let condition_text =
                normalize_source_counter_condition_text(&describe_static_condition(condition));
            if static_condition_is_during_your_turn(condition) {
                return format!("During your turn, {text}");
            }
            if condition_text.starts_with("as long as ") {
                return format!("{condition_text}, {text}");
            }
            text.push(' ');
            text.push_str(&condition_text);
        }
        text
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![effect_with_optional_static_condition(
            ContinuousEffect::new(
                source,
                controller,
                effect_target_for_filter(source, &self.filter),
                Modification::AddSubtypes(self.subtypes.clone()),
            )
            .with_source_type(EffectSourceType::StaticAbility),
            &self.condition,
        )]
    }
}

/// Add every subtype from a family: "Creatures you control are every creature type."
#[derive(Debug, Clone)]
pub struct AddAllSubtypesOfFamilyForFilter {
    pub filter: ObjectFilter,
    pub family: SubtypeFamily,
    pub condition: Option<crate::ConditionExpr>,
}

impl AddAllSubtypesOfFamilyForFilter {
    pub fn new(filter: ObjectFilter, family: SubtypeFamily) -> Self {
        Self {
            filter,
            family,
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

impl PartialEq for AddAllSubtypesOfFamilyForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter
            && self.family == other.family
            && self.condition == other.condition
    }
}

impl StaticAbilityKind for AddAllSubtypesOfFamilyForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::AddAllSubtypesOfFamily
    }

    fn display(&self) -> String {
        let subject = pluralized_subject_text(&self.filter);
        let (verb, _) = subject_verb_and_possessive(&subject);
        let mut text = format!("{subject} {verb} every {}", self.family.type_phrase());
        if let Some(condition) = &self.condition {
            text.push(' ');
            text.push_str(&describe_static_condition(condition));
        }
        text
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![effect_with_optional_static_condition(
            ContinuousEffect::new(
                source,
                controller,
                effect_target_for_filter(source, &self.filter),
                Modification::AddAllSubtypesOfFamily(self.family),
            )
            .with_source_type(EffectSourceType::StaticAbility),
            &self.condition,
        )]
    }
}

/// Set land subtypes by removing all land types first, then adding the new list.
#[derive(Debug, Clone)]
pub struct SetLandSubtypesForFilter {
    pub filter: ObjectFilter,
    pub subtypes: Vec<Subtype>,
    pub condition: Option<crate::ConditionExpr>,
}

impl SetLandSubtypesForFilter {
    pub fn new(filter: ObjectFilter, subtypes: Vec<Subtype>) -> Self {
        Self {
            filter,
            subtypes,
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

impl PartialEq for SetLandSubtypesForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter
            && self.subtypes == other.subtypes
            && self.condition == other.condition
    }
}

impl StaticAbilityKind for SetLandSubtypesForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SetLandSubtypes
    }

    fn display(&self) -> String {
        let subject = pluralized_subject_text(&self.filter);
        let (verb, _) = subject_verb_and_possessive(&subject);
        let subtypes = if verb == "is"
            && self.subtypes.iter().all(Subtype::is_land_subtype)
            && !self.subtypes.is_empty()
        {
            self.subtypes
                .iter()
                .enumerate()
                .map(|(index, subtype)| {
                    let name = subtype.to_string();
                    if index == 0 {
                        format!("{} {name}", indefinite_article_for(&name))
                    } else {
                        name
                    }
                })
                .collect::<Vec<_>>()
        } else {
            self.subtypes
                .iter()
                .map(|subtype| {
                    let name = subtype.to_string().to_ascii_lowercase();
                    if verb == "are" {
                        simple_pluralize(&name)
                    } else {
                        format!("{} {name}", indefinite_article_for(&name))
                    }
                })
                .collect::<Vec<_>>()
        };
        let mut text = format!("{subject} {verb} {}", join_with_and(&subtypes));
        if let Some(condition) = &self.condition {
            text.push(' ');
            text.push_str(&describe_static_condition(condition));
        }
        text
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            effect_with_optional_static_condition(
                ContinuousEffect::new(
                    source,
                    controller,
                    effect_target_for_filter(source, &self.filter),
                    Modification::SetSubtypes(self.subtypes.clone()),
                )
                .with_source_type(EffectSourceType::StaticAbility),
                &self.condition,
            ),
            effect_with_optional_static_condition(
                ContinuousEffect::new(
                    source,
                    controller,
                    effect_target_for_filter(source, &self.filter),
                    Modification::RemoveAllAbilities,
                )
                .with_source_type(EffectSourceType::StaticAbility),
                &self.condition,
            ),
        ]
    }
}

/// Set creature subtypes by removing all creature types first, then adding the new list.
#[derive(Debug, Clone)]
pub struct SetCreatureSubtypesForFilter {
    pub filter: ObjectFilter,
    pub subtypes: Vec<Subtype>,
    pub condition: Option<crate::ConditionExpr>,
}

impl SetCreatureSubtypesForFilter {
    pub fn new(filter: ObjectFilter, subtypes: Vec<Subtype>) -> Self {
        Self {
            filter,
            subtypes,
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

impl PartialEq for SetCreatureSubtypesForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter
            && self.subtypes == other.subtypes
            && self.condition == other.condition
    }
}

impl StaticAbilityKind for SetCreatureSubtypesForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SetCreatureSubtypes
    }

    fn display(&self) -> String {
        let subject = pluralized_subject_text(&self.filter);
        let (verb, _) = subject_verb_and_possessive(&subject);
        let subtypes = self
            .subtypes
            .iter()
            .map(|subtype| subtype.to_string().to_ascii_lowercase())
            .collect::<Vec<_>>();
        let mut text = format!("{subject} {verb} {}", join_with_and(&subtypes));
        if let Some(condition) = &self.condition {
            text.push(' ');
            text.push_str(&describe_static_condition(condition));
        }
        text
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            effect_with_optional_static_condition(
                ContinuousEffect::new(
                    source,
                    controller,
                    effect_target_for_filter(source, &self.filter),
                    Modification::RemoveAllSubtypesOfFamily(SubtypeFamily::Creature),
                )
                .with_source_type(EffectSourceType::StaticAbility),
                &self.condition,
            ),
            effect_with_optional_static_condition(
                ContinuousEffect::new(
                    source,
                    controller,
                    effect_target_for_filter(source, &self.filter),
                    Modification::AddSubtypes(self.subtypes.clone()),
                )
                .with_source_type(EffectSourceType::StaticAbility),
                &self.condition,
            ),
        ]
    }
}

/// This source has the base power, toughness, and creature types of the last
/// matching creature card exiled with it, while retaining listed creature types.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceCharacteristicsOfLastExiledCreatureCard {
    pub filter: ObjectFilter,
    pub retained_subtypes: Vec<Subtype>,
}

impl SourceCharacteristicsOfLastExiledCreatureCard {
    pub fn new(filter: ObjectFilter, retained_subtypes: Vec<Subtype>) -> Self {
        Self {
            filter,
            retained_subtypes,
        }
    }

    fn linked_card_characteristics(
        &self,
        source: ObjectId,
        controller: PlayerId,
        game: &GameState,
    ) -> Option<(i32, i32, Vec<Subtype>)> {
        let filter_ctx = game.filter_context_for_combat(controller, Some(source), None, None);
        game.get_exiled_with_source_links(source)
            .iter()
            .rev()
            .filter_map(|id| game.object(*id))
            .find(|object| self.filter.matches_non_recursive(object, &filter_ctx, game))
            .and_then(|object| {
                let power = object.power()?;
                let toughness = object.toughness()?;
                let mut subtypes = object
                    .subtypes
                    .iter()
                    .copied()
                    .filter(Subtype::is_creature_type)
                    .collect::<Vec<_>>();
                for retained in &self.retained_subtypes {
                    if !subtypes.contains(retained) {
                        subtypes.push(*retained);
                    }
                }
                Some((power, toughness, subtypes))
            })
    }
}

impl StaticAbilityKind for SourceCharacteristicsOfLastExiledCreatureCard {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SourceCharacteristicsOfLastExiledCreatureCard
    }

    fn display(&self) -> String {
        let retained = if self.retained_subtypes.is_empty() {
            String::new()
        } else {
            let names = self
                .retained_subtypes
                .iter()
                .map(|subtype| subtype.to_string())
                .collect::<Vec<_>>();
            format!(" It's still a {}.", join_with_and(&names))
        };
        format!(
            "As long as a card exiled with this creature is a creature card, this creature has the power, toughness, and creature types of the last creature card exiled with it.{retained}"
        )
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        game: &GameState,
    ) -> Vec<ContinuousEffect> {
        let Some((power, toughness, subtypes)) =
            self.linked_card_characteristics(source, controller, game)
        else {
            return Vec::new();
        };

        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Source,
                Modification::RemoveAllSubtypesOfFamily(SubtypeFamily::Creature),
            )
            .with_source_type(EffectSourceType::StaticAbility),
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Source,
                Modification::AddSubtypes(subtypes),
            )
            .with_source_type(EffectSourceType::StaticAbility),
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Source,
                Modification::SetPowerToughness {
                    power: Value::Fixed(power),
                    toughness: Value::Fixed(toughness),
                    sublayer: PtSublayer::Setting,
                },
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// Make colorless: "All permanents are colorless."
#[derive(Debug, Clone)]
pub struct MakeColorlessForFilter {
    pub filter: ObjectFilter,
}

impl MakeColorlessForFilter {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl PartialEq for MakeColorlessForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter
    }
}

impl StaticAbilityKind for MakeColorlessForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::MakeColorless
    }

    fn display(&self) -> String {
        if self.filter == ObjectFilter::source() {
            "Devoid".to_string()
        } else if self.filter.global_characteristic_domain_surface()
            == Some(
                ironsmith_core::GlobalCharacteristicDomainSurface::CardsOutsideBattlefieldSpellsAndPermanents,
            )
        {
            "All cards that aren't on the battlefield, spells, and permanents are colorless"
                .to_string()
        } else {
            let subject = pluralized_subject_text(&self.filter);
            let (verb, _) = subject_verb_and_possessive(&subject);
            format!("{subject} {verb} colorless")
        }
    }

    fn is_devoid(&self) -> bool {
        self.filter == ObjectFilter::source()
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Filter(self.filter.clone()),
                Modification::MakeColorless,
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// Remove supertypes: "All lands are no longer snow."
#[derive(Debug, Clone)]
pub struct RemoveSupertypesForFilter {
    pub filter: ObjectFilter,
    pub supertypes: Vec<Supertype>,
}

/// Add supertypes: "Enchanted creature is legendary."
#[derive(Debug, Clone)]
pub struct AddSupertypesForFilter {
    pub filter: ObjectFilter,
    pub supertypes: Vec<Supertype>,
}

impl AddSupertypesForFilter {
    pub fn new(filter: ObjectFilter, supertypes: Vec<Supertype>) -> Self {
        Self { filter, supertypes }
    }
}

impl PartialEq for AddSupertypesForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter && self.supertypes == other.supertypes
    }
}

impl StaticAbilityKind for AddSupertypesForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::AddSupertypes
    }

    fn display(&self) -> String {
        let subject = pluralized_subject_text(&self.filter);
        let singular = subject.starts_with("enchanted ")
            || subject.starts_with("equipped ")
            || subject.starts_with("this ")
            || subject.starts_with("that ")
            || subject == "land";
        let verb = if singular { "is" } else { "are" };
        let supertypes = self
            .supertypes
            .iter()
            .map(|supertype| supertype.name().to_string())
            .collect::<Vec<_>>()
            .join(" and ");
        format!("{subject} {verb} {supertypes}")
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Filter(self.filter.clone()),
                Modification::AddSupertypes(self.supertypes.clone()),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

impl RemoveSupertypesForFilter {
    pub fn new(filter: ObjectFilter, supertypes: Vec<Supertype>) -> Self {
        Self { filter, supertypes }
    }
}

impl PartialEq for RemoveSupertypesForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter && self.supertypes == other.supertypes
    }
}

impl StaticAbilityKind for RemoveSupertypesForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::RemoveSupertypes
    }

    fn display(&self) -> String {
        let subject = if self.filter == ObjectFilter::land() {
            "All lands".to_string()
        } else {
            pluralized_subject_text(&self.filter)
        };
        let singular = subject.starts_with("enchanted ")
            || subject.starts_with("equipped ")
            || subject.starts_with("this ")
            || subject.starts_with("that ")
            || subject == "land";
        let verb = if singular { "is" } else { "are" };
        let supertypes = self
            .supertypes
            .iter()
            .map(|supertype| supertype.name().to_string())
            .collect::<Vec<_>>()
            .join(" and ");
        format!("{subject} {verb} no longer {supertypes}")
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Filter(self.filter.clone()),
                Modification::RemoveSupertypes(self.supertypes.clone()),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

impl EquipmentGrant {
    pub fn new(abilities: Vec<StaticAbility>) -> Self {
        Self { abilities }
    }
}

impl PartialEq for EquipmentGrant {
    fn eq(&self, other: &Self) -> bool {
        self.abilities == other.abilities
    }
}

impl StaticAbilityKind for EquipmentGrant {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EquipmentGrant
    }

    fn display(&self) -> String {
        let ability_names: Vec<String> = self.abilities.iter().map(|a| a.display()).collect();
        format!("Equipped creature has {}", ability_names.join(", "))
    }

    fn grants_abilities(&self) -> bool {
        true
    }

    fn equipment_grant_abilities(&self) -> Option<&[StaticAbility]> {
        Some(&self.abilities)
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        self.abilities
            .iter()
            .map(|ability| {
                ContinuousEffect::new(
                    source,
                    controller,
                    EffectTarget::AttachedTo(source),
                    Modification::AddAbility(ability.clone()),
                )
                .with_source_type(EffectSourceType::StaticAbility)
            })
            .collect()
    }
}

/// Enchanted/attached permanent has an activated or triggered ability.
#[derive(Debug, Clone, PartialEq)]
pub struct AttachedAbilityGrant {
    pub ability: Ability,
    pub additional_abilities: Vec<Ability>,
    pub display: String,
    pub condition: Option<crate::ConditionExpr>,
    pub protection_does_not_remove_controlled_attachments: bool,
}

impl AttachedAbilityGrant {
    pub fn new(ability: Ability, display: String) -> Self {
        Self {
            ability,
            additional_abilities: Vec::new(),
            display,
            condition: None,
            protection_does_not_remove_controlled_attachments: false,
        }
    }

    pub fn with_additional_abilities(mut self, abilities: Vec<Ability>) -> Self {
        self.additional_abilities = abilities;
        self
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }

    pub fn with_protection_attachment_exception(mut self, enabled: bool) -> Self {
        self.protection_does_not_remove_controlled_attachments = enabled;
        self
    }
}

fn materialize_named_granting_source_in_effect(
    effect: &crate::effect::Effect,
    source: ObjectId,
) -> crate::effect::Effect {
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        let mut sequence = sequence.clone();
        sequence.effects = sequence
            .effects
            .iter()
            .map(|effect| materialize_named_granting_source_in_effect(effect, source))
            .collect();
        return crate::effect::Effect::new(sequence);
    }

    let Some(with_source) = effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>() else {
        return effect.clone();
    };
    if !matches!(with_source.source.base(), ChooseSpec::Source)
        || !matches!(
            with_source.source.source_reference_surface(),
            Some(SourceReferenceSurface::FullName(_) | SourceReferenceSurface::ShortName(_))
        )
    {
        return effect.clone();
    }

    let materialized_source = ChooseSpec::SpecificObject(source)
        .with_surface_hints(with_source.source.surface_hints().iter().cloned());
    crate::effect::Effect::new(crate::effects::ExecuteWithSourceEffect::new(
        materialized_source,
        (*with_source.effect).clone(),
    ))
}

fn materialize_named_granting_source(ability: &Ability, source: ObjectId) -> Ability {
    let mut ability = ability.clone();
    let program = match &mut ability.kind {
        AbilityKind::Triggered(triggered) => &mut triggered.effects,
        AbilityKind::Activated(activated) => &mut activated.effects,
        AbilityKind::Static(_) => return ability,
    };
    *program = program
        .clone()
        .try_map_effects(|effect| {
            Ok::<_, std::convert::Infallible>(materialize_named_granting_source_in_effect(
                &effect, source,
            ))
        })
        .expect("infallible granting-source materialization");
    ability
}

impl StaticAbilityKind for AttachedAbilityGrant {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::AttachedAbilityGrant
    }

    fn display(&self) -> String {
        // A conditioned grant that removes all abilities describes a rule on
        // the attached permanent, not a global loss rule.  Keep the authored
        // attachment subject in front of the condition and use the pronoun in
        // the consequent: "As long as enchanted creature is red, it loses all
        // abilities."  Restrict this presentation to the typed ability-loss
        // payload so ordinary conditional grants retain their existing form.
        if self.additional_abilities.is_empty()
            && let Some(condition) = &self.condition
            && let AbilityKind::Static(granted) = &self.ability.kind
            && granted.id() == StaticAbilityId::RemoveAllAbilitiesForFilter
            && let Some(subject) = self
                .display
                .trim()
                .trim_end_matches('.')
                .strip_suffix(" loses all abilities")
            && matches!(
                subject,
                "enchanted creature" | "enchanted permanent" | "equipped creature"
            )
            && let Some(condition_text) =
                describe_attached_subject_static_condition(condition, subject)
        {
            return format!("{condition_text}, it loses all abilities");
        }
        let mut text = self.display.clone();
        if let Some(condition) = &self.condition {
            text.push(' ');
            let attached_subject = self
                .display
                .trim()
                .trim_end_matches('.')
                .split_once(" has ")
                .map(|(subject, _)| subject)
                .or_else(|| {
                    self.display
                        .trim()
                        .trim_end_matches('.')
                        .split_once(" have ")
                        .map(|(subject, _)| subject)
                });
            let condition_text = attached_subject
                .and_then(|subject| describe_attached_subject_static_condition(condition, subject))
                .unwrap_or_else(|| describe_static_condition(condition));
            text.push_str(&condition_text);
        }
        text
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
    }

    fn granted_inline_ability(&self) -> Option<&crate::ability::Ability> {
        Some(&self.ability)
    }

    fn apply_restrictions(&self, game: &mut GameState, source: ObjectId, controller: PlayerId) {
        if let Some(condition) = &self.condition {
            let iterated_player = game
                .object(source)
                .and_then(|object| object.attached_to.and_then(|target| target.object_id()))
                .and_then(|attached_to| game.object(attached_to))
                .map(|attached_object| game.controller_of(attached_object));
            if !static_condition_is_active_with_iterated_player(
                condition,
                game,
                source,
                controller,
                iterated_player,
            ) {
                return;
            }
        }

        let Some(attached_to) = game
            .object(source)
            .and_then(|object| object.attached_to)
            .and_then(|target| target.object_id())
        else {
            return;
        };
        let attached_controller = game
            .object(attached_to)
            .map(|object| game.controller_of(object))
            .unwrap_or(controller);

        if let crate::ability::AbilityKind::Static(static_ability) = &self.ability.kind {
            static_ability.apply_restrictions(game, attached_to, attached_controller);
        }
        for ability in &self.additional_abilities {
            if let crate::ability::AbilityKind::Static(static_ability) = &ability.kind {
                static_ability.apply_restrictions(game, attached_to, attached_controller);
            }
        }
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        // Removing all abilities is itself a layer-6 modification. Adding a
        // granted static ability whose payload removes abilities would require
        // the layer engine to discover a new continuous effect while already
        // evaluating that layer, and the newly granted rule would never take
        // effect. Apply this exact typed payload directly to the attached
        // object while retaining the grant's attachment-relative condition.
        if self.additional_abilities.is_empty()
            && let AbilityKind::Static(static_ability) = &self.ability.kind
            && let Some(model) = static_ability.compiled_model()
            && matches!(
                &model.payload,
                ironsmith_core::StaticAbilityPayload::RemoveAllAbilities(filter)
                    if filter == &ObjectFilter::source()
            )
        {
            return vec![effect_with_optional_static_condition(
                ContinuousEffect::new(
                    source,
                    controller,
                    EffectTarget::AttachedTo(source),
                    Modification::RemoveAllAbilities,
                )
                .with_source_type(EffectSourceType::StaticAbility),
                &self.condition,
            )];
        }
        let mut effects = Vec::with_capacity(1 + self.additional_abilities.len());
        effects.push(effect_with_optional_static_condition(
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::AttachedTo(source),
                Modification::AddAbilityGeneric(materialize_named_granting_source(
                    &self.ability,
                    source,
                )),
            )
            .with_source_type(EffectSourceType::StaticAbility),
            &self.condition,
        ));
        effects.extend(self.additional_abilities.iter().cloned().map(|ability| {
            effect_with_optional_static_condition(
                ContinuousEffect::new(
                    source,
                    controller,
                    EffectTarget::AttachedTo(source),
                    Modification::AddAbilityGeneric(materialize_named_granting_source(
                        &ability, source,
                    )),
                )
                .with_source_type(EffectSourceType::StaticAbility),
                &self.condition,
            )
        }));
        effects
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<crate::replacement::ReplacementEffect> {
        let AbilityKind::Static(static_ability) = &self.ability.kind else {
            return None;
        };
        if static_ability.id() != StaticAbilityId::PreventAllDamageToSelf {
            return None;
        }

        Some(crate::replacement::ReplacementEffect::with_matcher(
            source,
            controller,
            crate::events::DamageToAttachedObjectMatcher::new(),
            crate::replacement::ReplacementAction::PreventDamage,
        ))
    }
}

/// Enchanted/attached permanent has landwalk of the chosen land type.
#[derive(Debug, Clone, PartialEq)]
pub struct AttachedChosenLandwalkGrant {
    pub display: String,
    pub snow: bool,
    pub condition: Option<crate::ConditionExpr>,
}

impl AttachedChosenLandwalkGrant {
    pub fn new(display: String, snow: bool) -> Self {
        Self {
            display,
            snow,
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

impl StaticAbilityKind for AttachedChosenLandwalkGrant {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::AttachedChosenLandwalkGrant
    }

    fn display(&self) -> String {
        let mut text = self.display.clone();
        if let Some(condition) = &self.condition {
            text.push(' ');
            text.push_str(&describe_static_condition(condition));
        }
        text
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        game: &GameState,
    ) -> Vec<ContinuousEffect> {
        let Some(chosen_type) = game.chosen_land_type(source) else {
            return Vec::new();
        };

        let ability = if self.snow {
            StaticAbility::snow_landwalk(chosen_type)
        } else {
            StaticAbility::landwalk(chosen_type)
        };

        vec![effect_with_optional_static_condition(
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::AttachedTo(source),
                Modification::AddAbility(ability),
            )
            .with_source_type(EffectSourceType::StaticAbility),
            &self.condition,
        )]
    }
}
