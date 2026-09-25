//! Small gameplay-only presentation fallback.
//!
//! Audit-grade canonical rendering lives in `ironsmith-text`. The engine keeps
//! only these non-recursive labels so browser gameplay does not link the full
//! renderer. Compiled artifacts and browser snapshots should carry authored
//! presentation strings whenever exact wording matters.

pub mod effect_sentences;

use crate::ability::{Ability, AbilityKind};
use crate::cards::CardDefinition;
use crate::continuous::{AbilityOrigin, CalculatedAbilities};
use crate::effect::{Condition, Effect, Value};
use crate::filter::ObjectFilter;
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::mana::ManaCost;
use crate::object::Object;
use crate::target::PlayerFilter;
use ironsmith_core::ability_model::PresentationLabel;

/// A plain-words summary of an effect list, for surfaces with no printed
/// sentence to quote: each effect is named by the words its executor type
/// implies ("create token", "attach objects"), never by its structure.
pub fn compile_effect_list(effects: &[Effect]) -> String {
    effect_sentences::effect_phrase(effects)
}

/// One printed line per ability, when a definition's presentation allows it.
///
/// Labels rendered by the compiler win, then the canonical lines when there is
/// exactly one per ability, and finally the runtime's own per-ability wording
/// for a definition with no canonical text at all (a token). A definition
/// whose lines and abilities disagree in count with no labels to reconcile
/// them yields nothing, so callers never index a neighbour's line.
pub fn definition_ability_labels(definition: &CardDefinition) -> Vec<String> {
    let count = definition.abilities.len();
    if count == 0 {
        return Vec::new();
    }
    if definition.ability_labels.len() == count
        && definition
            .ability_labels
            .iter()
            .any(|label| !label.trim().is_empty())
    {
        return definition.ability_labels.clone();
    }
    let canonical = canonical_lines(&definition.canonical_text);
    if canonical.len() == count {
        return canonical;
    }
    if canonical.is_empty() {
        return definition
            .abilities
            .iter()
            .map(ability_surface_text)
            .collect();
    }
    Vec::new()
}

/// The label of ability `index` given labels and compiled text that may or
/// may not line up with `ability_count` abilities.
pub fn aligned_ability_label(
    labels: &[String],
    compiled_card_text: &str,
    ability_count: usize,
    index: usize,
) -> Option<String> {
    if index >= ability_count {
        return None;
    }
    if labels.len() == ability_count {
        return labels
            .get(index)
            .map(|label| label.trim())
            .filter(|label| !label.is_empty())
            .map(str::to_string);
    }
    let lines = canonical_lines(compiled_card_text);
    if lines.len() == ability_count {
        return lines.get(index).cloned();
    }
    None
}

fn canonical_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

/// Collapse runs of identical lines: several abilities compiled out of one
/// printed sentence all read as that sentence, which a text box prints once.
pub fn dedupe_consecutive_lines(lines: Vec<String>) -> Vec<String> {
    let mut deduped: Vec<String> = Vec::with_capacity(lines.len());
    for line in lines {
        if deduped.last().is_some_and(|previous| *previous == line) {
            continue;
        }
        deduped.push(line);
    }
    deduped
}

/// The printed line behind one ability of an object's current characteristics.
///
/// An ability a permanent gained records the card that lent it, so its wording
/// is that card's printed line rather than anything this object prints: a
/// creature copying Walking Ballista's abilities through Agatha's Soul Cauldron
/// has no line of its own for them. An ability a continuous effect granted
/// (Vile Consumption's `All creatures have "At the beginning of your upkeep,
/// ..."`) reads as the quotation the granting card prints for it. Returns
/// `None` when no card prints the ability's wording.
pub fn printed_ability_line(
    game: &GameState,
    object: &Object,
    abilities: &CalculatedAbilities,
    ability_index: usize,
) -> Option<String> {
    fn resolve(
        game: &GameState,
        object: &Object,
        origin: &AbilityOrigin,
        ability: &Ability,
    ) -> Option<String> {
        match origin {
            AbilityOrigin::Printed(index) => object.ability_label(*index),
            AbilityOrigin::Borrowed { source, origin, .. } => game
                .object(*source)
                .and_then(|lender| resolve(game, lender, origin, ability)),
            AbilityOrigin::Effect { effect, .. } => granted_ability_quote(game, effect, ability),
        }
    }

    let ability = abilities.get(ability_index)?;
    abilities
        .origin(ability_index)
        .and_then(|origin| resolve(game, object, origin, ability))
}

/// The quotation a granting card prints for an ability its effect grants.
///
/// The granted ability's wording exists only inside the grant's own printed
/// line, between quotation marks. The line is the granting static ability's
/// label when the effect records which static ability generated it, else any
/// of the source's lines (a resolved "gains \"...\"" instruction). A line can
/// quote several abilities (`has "{T}: ..." and "{4}: ..."`), so the
/// quotation is chosen by the ability's shape and then by shared wording.
fn granted_ability_quote(
    game: &GameState,
    effect: &crate::continuous::AbilityEffectOrigin,
    ability: &Ability,
) -> Option<String> {
    let lender = game.object(effect.source())?;
    let granting_line = effect.static_ability().and_then(|instance| {
        let index = lender.abilities.iter().position(|printed| {
            matches!(&printed.kind, AbilityKind::Static(static_ability)
                if static_ability.instance_id() == instance)
        })?;
        lender.ability_label(index)
    });
    let lines = match granting_line {
        Some(line) => vec![line],
        None => canonical_lines(&lender.compiled_card_text),
    };

    let mut quotes: Vec<String> = lines
        .iter()
        .flat_map(|line| quoted_segments(line))
        .filter(|quote| quote_fits_ability_kind(quote, ability))
        .collect();
    // An activated ability's cost is printed verbatim before its colon, which
    // tells sibling quotations apart even when the effect has no plain words.
    if let AbilityKind::Activated(activated) = &ability.kind {
        let cost = activated.mana_cost.display().to_ascii_lowercase();
        let same_cost: Vec<String> = quotes
            .iter()
            .filter(|quote| {
                quote
                    .split_once(": ")
                    .is_some_and(|(printed, _)| printed.trim().to_ascii_lowercase() == cost.trim())
            })
            .cloned()
            .collect();
        if !same_cost.is_empty() {
            quotes = same_cost;
        }
    }
    // A granted keyword (the "flying" of `have flying and "..."`) is printed
    // bare, so a static ability only takes a quotation that shares its words.
    let chosen = match quotes.as_slice() {
        [] => return None,
        [only] if !matches!(ability.kind, AbilityKind::Static(_)) => only,
        _ => {
            let index =
                effect_sentences::best_matching_text(&quotes, &ability_surface_text(ability))?;
            &quotes[index]
        }
    };
    let mut text = capitalize_first(chosen.trim());
    if !matches!(ability.kind, AbilityKind::Static(_)) && !text.ends_with(['.', '!', '?', ')']) {
        text.push('.');
    }
    Some(text)
}

/// The quotations in a printed line, outside its reminder text.
///
/// Reminder text quotes the abilities of the tokens it describes (`(It's an
/// artifact with "{T}, Sacrifice this token: ...")`), which no grant gives.
fn quoted_segments(line: &str) -> Vec<String> {
    let mut quotes = Vec::new();
    let mut depth = 0usize;
    let mut open: Option<String> = None;
    for ch in line.chars() {
        match (&mut open, ch) {
            (Some(quote), '"') => {
                let quote = std::mem::take(quote);
                open = None;
                if depth == 0 && !quote.trim().is_empty() {
                    quotes.push(quote.trim().trim_end_matches(',').to_string());
                }
            }
            (Some(quote), _) => quote.push(ch),
            (None, '"') => open = Some(String::new()),
            (None, '(') => depth += 1,
            (None, ')') => depth = depth.saturating_sub(1),
            (None, _) => {}
        }
    }
    quotes
}

/// Whether a quotation reads as an ability of `ability`'s kind: a trigger
/// opens with its trigger word, an activated ability puts its cost before a
/// colon, and a static ability does neither.
fn quote_fits_ability_kind(quote: &str, ability: &Ability) -> bool {
    let lower = quote.trim().to_ascii_lowercase();
    let triggered = ["at ", "when ", "whenever "]
        .iter()
        .any(|word| lower.starts_with(word));
    let activated = !triggered
        && lower
            .split_once(": ")
            .is_some_and(|(cost, _)| !cost.contains(". ") && !cost.contains(", then"));
    match ability.kind {
        AbilityKind::Triggered(_) => triggered,
        AbilityKind::Activated(_) => activated,
        AbilityKind::Static(_) => !triggered && !activated,
    }
}

/// The printed line behind one currently active ability of `source`.
pub fn printed_ability_line_for_object(
    game: &GameState,
    source: ObjectId,
    ability_index: usize,
) -> Option<String> {
    let object = game.object(source)?;
    let characteristics = game.current_characteristics(source)?;
    printed_ability_line(game, object, &characteristics.abilities, ability_index)
}

/// Gameplay wording for one ability that has no printed line to quote.
///
/// Keyword abilities read as their keyword, ability words keep their label,
/// and anything else is spelled out from its trigger or cost and a plain-words
/// summary of its effects. Nothing here ever prints the ability's structure: a
/// rendering that still looks like one is replaced by a generic label.
pub fn ability_surface_text(ability: &Ability) -> String {
    if let Some(text) = fixed_mana_ability_surface_text(ability) {
        return text;
    }
    let (text, generic) = match &ability.kind {
        AbilityKind::Static(static_ability) => (static_ability.display(), "Static ability"),
        AbilityKind::Triggered(triggered) => {
            if let Some(PresentationLabel::Keyword(keyword)) = &triggered.presentation_label {
                return keyword.display();
            }
            let body = sentence(
                &triggered.trigger.display(),
                &effect_sentences::effect_phrase(triggered.effects.flattened_default_effects()),
            );
            (
                with_presentation_prefix(triggered.presentation_label.as_ref(), body),
                "Triggered ability",
            )
        }
        AbilityKind::Activated(activated) => {
            let cost = activated.mana_cost.display();
            let resolution = match activated
                .mana_output
                .as_ref()
                .filter(|mana| !mana.is_empty())
            {
                Some(mana) => format!("Add {}", ManaCost::from_symbols(mana.clone()).to_oracle()),
                None => capitalize_first(&effect_sentences::effect_phrase(
                    activated.effects.flattened_default_effects(),
                )),
            };
            let body = match (cost.trim().is_empty(), resolution.trim().is_empty()) {
                (true, true) => String::new(),
                (false, true) => cost,
                (true, false) => format!("{resolution}."),
                (false, false) => format!("{cost}: {resolution}."),
            };
            (body, "Activated ability")
        }
    };
    if text.trim().is_empty() || effect_sentences::looks_like_compiled_structure(&text) {
        generic.to_string()
    } else {
        text
    }
}

fn with_presentation_prefix(label: Option<&PresentationLabel>, body: String) -> String {
    match label.and_then(PresentationLabel::display_prefix) {
        Some(prefix) if !body.trim().is_empty() => format!("{prefix} — {body}"),
        Some(prefix) => prefix,
        None => body,
    }
}

fn sentence(head: &str, tail: &str) -> String {
    let head = head.trim().trim_end_matches(['.', ',']);
    let tail = tail.trim();
    match (head.is_empty(), tail.is_empty()) {
        (true, true) => String::new(),
        (false, true) => format!("{head}."),
        (true, false) => format!("{}.", capitalize_first(tail)),
        (false, false) => format!("{head}, {tail}."),
    }
}

fn capitalize_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn fixed_mana_ability_surface_text(ability: &Ability) -> Option<String> {
    let AbilityKind::Activated(activated) = &ability.kind else {
        return None;
    };
    let mana = activated.mana_output.as_ref()?;
    if mana.is_empty()
        || !activated.effects.is_empty()
        || !activated.choices.is_empty()
        || activated.activation_condition.is_some()
        || !activated.activation_restrictions.is_empty()
        || !activated.additional_restrictions.is_empty()
        || !activated.mana_usage_restrictions.is_empty()
    {
        return None;
    }

    let cost = activated.mana_cost.display();
    let output = ManaCost::from_symbols(mana.clone()).to_oracle();
    Some(format!("{cost}: Add {output}."))
}

/// Merge exact compiled labels with abilities added by current characteristics.
///
/// Printed abilities retain their artifact wording when the same executable
/// ability is still present. Abilities granted by land types or continuous
/// effects use the lightweight runtime renderer instead of invalidating every
/// printed label merely because the list length changed. Abilities compiled
/// out of one printed line share its wording, which is listed once.
pub fn current_ability_surface_texts(
    current_abilities: &[Ability],
    definition: Option<&CardDefinition>,
) -> Vec<String> {
    let Some(definition) = definition else {
        return dedupe_consecutive_lines(
            current_abilities.iter().map(ability_surface_text).collect(),
        );
    };

    let definition_labels = definition_ability_labels(definition);
    let mut used_definition_abilities = vec![false; definition.abilities.len()];

    let texts = current_abilities
        .iter()
        .map(|current| {
            definition
                .abilities
                .iter()
                .enumerate()
                .find(|(index, printed)| {
                    !used_definition_abilities[*index]
                        && definition_labels.get(*index).is_some()
                        && *printed == current
                })
                .and_then(|(index, _)| {
                    used_definition_abilities[index] = true;
                    definition_labels.get(index).cloned()
                })
                .unwrap_or_else(|| ability_surface_text(current))
        })
        .collect();
    dedupe_consecutive_lines(texts)
}

/// Wording for an object's current abilities outside the battlefield.
///
/// A current ability still printed on the object reads as the object's own
/// label for it (which follows copies and tokens, unlike a registry lookup).
/// Printed abilities are matched by value first; a list of the printed length
/// then matches its leftovers by position and kind, since an ability rebound
/// for its zone is still the printed ability. One the object does not print
/// falls back to the definition's label for an equal ability, then to the
/// runtime wording. Shared lines are listed once.
pub fn object_ability_surface_texts(
    object: &Object,
    current_abilities: &[Ability],
    definition: Option<&CardDefinition>,
) -> Vec<String> {
    let mut texts: Vec<Option<String>> = vec![None; current_abilities.len()];
    let mut used_object_abilities = vec![false; object.abilities.len()];
    for (slot, current) in current_abilities.iter().enumerate() {
        if let Some((index, _)) = object
            .abilities
            .iter()
            .enumerate()
            .find(|(index, printed)| !used_object_abilities[*index] && *printed == current)
        {
            used_object_abilities[index] = true;
            texts[slot] = object.ability_label(index);
        }
    }
    if current_abilities.len() == object.abilities.len() {
        for (slot, current) in current_abilities.iter().enumerate() {
            if texts[slot].is_none()
                && !used_object_abilities[slot]
                && std::mem::discriminant(&object.abilities[slot].kind)
                    == std::mem::discriminant(&current.kind)
            {
                used_object_abilities[slot] = true;
                texts[slot] = object.ability_label(slot);
            }
        }
    }

    let definition_labels = definition
        .map(definition_ability_labels)
        .unwrap_or_default();
    let mut used_definition_abilities = definition
        .map(|definition| vec![false; definition.abilities.len()])
        .unwrap_or_default();
    let texts = texts
        .into_iter()
        .zip(current_abilities)
        .map(|(text, current)| {
            text.or_else(|| {
                let definition = definition?;
                let (index, _) =
                    definition
                        .abilities
                        .iter()
                        .enumerate()
                        .find(|(index, printed)| {
                            !used_definition_abilities[*index]
                                && definition_labels.get(*index).is_some()
                                && *printed == current
                        })?;
                used_definition_abilities[index] = true;
                definition_labels.get(index).cloned()
            })
            .unwrap_or_else(|| ability_surface_text(current))
        })
        .collect();
    dedupe_consecutive_lines(texts)
}

/// Return the presentation text for one ability in a runtime text box.
///
/// Compiled card text is the authoritative lightweight presentation carried by
/// game objects. When its non-empty lines map one-to-one to the current
/// abilities, preserve that canonical wording instead of falling through to
/// the gameplay-only debug renderer. Dynamic ability changes deliberately use
/// the fallback unless their text box changed in lockstep.
pub fn indexed_ability_surface_text(
    abilities: &[Ability],
    compiled_card_text: &str,
    ability_index: usize,
) -> Option<String> {
    let ability = abilities.get(ability_index)?;
    let canonical = compiled_card_text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();

    if canonical.len() == abilities.len() {
        return canonical.get(ability_index).cloned();
    }

    Some(ability_surface_text(ability))
}

#[cfg(test)]
pub fn ability_surface_text_for_tests(ability: &Ability) -> String {
    ability_surface_text(ability)
}

/// The lines a runtime text box prints for a definition.
///
/// The compiler's canonical text is authoritative. A definition without any
/// (a token created by an effect) prints each ability's gameplay wording and,
/// for a spell, a plain-words summary of what it does.
pub fn compiled_text_lines(definition: &CardDefinition) -> Vec<String> {
    let canonical = canonical_lines(&definition.canonical_text);
    if !canonical.is_empty() {
        return canonical;
    }

    let labels = definition_ability_labels(definition);
    let ability_lines = if labels.len() == definition.abilities.len() {
        labels
    } else {
        definition
            .abilities
            .iter()
            .map(ability_surface_text)
            .collect()
    };
    let spell_lines = definition
        .spell_effect
        .iter()
        .map(|program| {
            capitalize_first(&effect_sentences::effect_phrase(
                program.flattened_default_effects(),
            ))
        })
        .filter(|line| !line.is_empty())
        .map(|line| format!("{line}."));
    dedupe_consecutive_lines(ability_lines.into_iter().chain(spell_lines).collect())
}

pub fn debug_compiled_lines(definition: &CardDefinition) -> Vec<String> {
    compiled_text_lines(definition)
}

pub fn unprocessed_compiled_lines(definition: &CardDefinition) -> Vec<String> {
    compiled_text_lines(definition)
}

pub fn canonical_compiled_lines(definition: &CardDefinition) -> Vec<String> {
    compiled_text_lines(definition)
}

pub fn describe_effect(effect: &Effect) -> String {
    format!("{effect:?}")
}

pub fn describe_value(value: &Value) -> String {
    format!("{value:?}")
}

pub fn describe_condition(condition: &Condition) -> String {
    format!("{condition:?}")
}

pub fn pluralize_noun_phrase_for_trigger(phrase: &str) -> String {
    if phrase.ends_with('s') {
        phrase.to_string()
    } else {
        format!("{phrase}s")
    }
}

pub fn describe_party_size_for_each_basis(_value: &Value) -> Option<(i32, String)> {
    None
}

pub fn describe_counter_for_each_basis(_value: &Value) -> Option<(i32, String)> {
    None
}

pub fn describe_for_each_multiplier_and_basis(_value: &Value) -> Option<(i32, String)> {
    None
}

pub fn describe_turn_history_for_each_basis(_value: &Value) -> Option<String> {
    None
}

pub fn describe_aggregate_filter_value_subject(filter: &ObjectFilter) -> String {
    filter.description()
}

pub fn describe_death_history_subject(
    subject: &str,
    controller: Option<&PlayerFilter>,
    _controller_surface: ironsmith_core::DeathHistoryControllerSurface,
) -> String {
    controller.map_or_else(
        || format!("{subject} that died this turn"),
        |controller| format!("{subject} ({controller:?}) that died this turn"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexed_ability_text_prefers_canonical_one_to_one_lines() {
        let abilities = vec![crate::ability::flying(), crate::ability::trample()];

        assert_eq!(
            indexed_ability_surface_text(&abilities, "Flying\nTrample", 0).as_deref(),
            Some("Flying")
        );
        assert_eq!(
            indexed_ability_surface_text(&abilities, "Flying\nTrample", 1).as_deref(),
            Some("Trample")
        );
    }

    #[test]
    fn keyword_trigger_reads_as_its_keyword() {
        let mut prowess = Ability::triggered(
            crate::triggers::Trigger::spell_cast(None, crate::target::PlayerFilter::You),
            vec![],
        );
        if let AbilityKind::Triggered(triggered) = &mut prowess.kind {
            triggered.presentation_label = Some(PresentationLabel::Keyword(
                ironsmith_core::ability_model::PresentationKeyword::Prowess,
            ));
        }
        assert_eq!(ability_surface_text(&prowess), "Prowess");

        // A token created by an effect carries no canonical text: its text
        // box is its abilities' wording, one line each, and its labels align.
        let otter =
            crate::cards::builders::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Otter")
                .token()
                .with_ability(prowess)
                .with_ability(crate::ability::flying())
                .build();
        assert_eq!(compiled_text_lines(&otter), vec!["Prowess", "Flying"]);
        assert_eq!(definition_ability_labels(&otter), vec!["Prowess", "Flying"]);
    }

    #[test]
    fn generated_damage_token_keeps_amount_and_target_in_display() {
        let definition = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Munitions",
        )
        .token()
        .with_ability(Ability::triggered(
            crate::triggers::Trigger::this_leaves_battlefield(),
            vec![Effect::deal_damage(2, crate::target::ChooseSpec::AnyTarget)],
        ))
        .build();
        let object = Object::from_card_definition(
            ObjectId::from_raw(1),
            &definition,
            crate::ids::PlayerId::from_index(0),
            crate::zone::Zone::Battlefield,
        );
        assert!(
            object.compiled_card_text.contains("2 damage to any target"),
            "token text lost its damage amount or target: {}",
            object.compiled_card_text
        );
        assert!(object.ability_labels[0].contains("2 damage to any target"));
    }

    #[test]
    fn runtime_wording_never_prints_structure() {
        let activated = Ability::activated(
            crate::TotalCost::from_cost(crate::costs::Cost::remove_counters(
                crate::object::CounterType::PlusOnePlusOne,
                1,
            )),
            vec![crate::effect::Effect::deal_damage(
                1,
                crate::target::ChooseSpec::AnyTarget,
            )],
        );
        let text = ability_surface_text(&activated);
        assert!(
            !effect_sentences::looks_like_compiled_structure(&text),
            "activated ability reads as structure: {text}"
        );
        assert!(
            text.contains(':'),
            "activated ability keeps its cost: {text}"
        );

        let triggered = Ability::triggered(
            crate::triggers::Trigger::this_enters_battlefield(),
            vec![crate::effect::Effect::deal_damage(
                1,
                crate::target::ChooseSpec::AnyTarget,
            )],
        );
        let text = ability_surface_text(&triggered);
        assert!(
            !effect_sentences::looks_like_compiled_structure(&text),
            "triggered ability reads as structure: {text}"
        );

        let list = compile_effect_list(&[crate::effect::Effect::deal_damage(
            1,
            crate::target::ChooseSpec::AnyTarget,
        )]);
        assert!(
            !list.is_empty() && !effect_sentences::looks_like_compiled_structure(&list),
            "effect list reads as structure: {list}"
        );
    }

    #[test]
    fn aligned_labels_win_and_misaligned_text_is_never_indexed() {
        let labels = vec!["A".to_string(), "A".to_string(), "B".to_string()];
        assert_eq!(
            aligned_ability_label(&labels, "A\nB", 3, 1).as_deref(),
            Some("A")
        );
        assert_eq!(
            aligned_ability_label(&labels, "A\nB", 3, 2).as_deref(),
            Some("B")
        );
        assert_eq!(aligned_ability_label(&[], "A\nB", 3, 1), None);
        assert_eq!(
            aligned_ability_label(&[], "A\nB\nC", 3, 2).as_deref(),
            Some("C")
        );
        assert_eq!(aligned_ability_label(&labels, "A\nB", 3, 3), None);
        assert_eq!(dedupe_consecutive_lines(labels), vec!["A", "B"]);
    }

    #[test]
    fn indexed_ability_text_does_not_misalign_changed_ability_lists() {
        let abilities = vec![crate::ability::flying(), crate::ability::trample()];
        let fallback = indexed_ability_surface_text(&abilities, "Flying", 1)
            .expect("the second ability should still have a fallback label");

        assert_ne!(fallback, "Flying");
        assert!(indexed_ability_surface_text(&abilities, "Flying", 2).is_none());
    }
}
