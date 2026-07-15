use super::*;
use crate::cards::CardDefinitionRuntimeExt;

pub(super) struct RawRenderedLine(String);

impl RawRenderedLine {
    pub(super) fn into_string(self) -> String {
        self.0
    }
}

pub(super) fn ast_compiled_lines(def: &CardDefinition) -> Vec<RawRenderedLine> {
    stacker::maybe_grow(1024 * 1024, 8 * 1024 * 1024, || compiled_lines_inner(def))
        .into_iter()
        .map(|line| rewrite_self_exile_cost_source(def, &line))
        .map(|line| rewrite_named_source_counter_reference(def, &line))
        .map(RawRenderedLine)
        .collect()
}

fn rewrite_self_exile_cost_source(def: &CardDefinition, line: &str) -> String {
    if line.contains("Exile this card from your graveyard")
        || line.contains("exile this card from your graveyard")
    {
        return line.to_string();
    }
    let Some(primary_type) = def.card.card_types.first() else {
        return line.to_string();
    };
    if *primary_type == CardType::Land {
        return line.to_string();
    }
    let source_phrase = format!("Exile this {}", primary_type.name().to_ascii_lowercase());
    if !line.contains(&source_phrase) || !line.contains(':') {
        return line.to_string();
    }
    line.replace(&source_phrase, &format!("Exile {}", def.card.name))
}

fn rewrite_named_source_counter_reference(def: &CardDefinition, line: &str) -> String {
    if !def.card.name.starts_with("The ") {
        return line.to_string();
    }
    let Some(primary_type) = def.card.card_types.first() else {
        return line.to_string();
    };
    let source_type = primary_type.name().to_ascii_lowercase();
    let mut rewritten = line.to_string();
    for counter_noun in ["counter", "counters"] {
        let pattern = format!("{counter_noun} on this {source_type}");
        let replacement = format!("{counter_noun} on {}", def.card.name);
        rewritten = rewritten.replace(&pattern, &replacement);
    }
    rewritten
}

fn render_labeled_static_body(
    ability: &crate::static_abilities::StaticAbility,
    subject: &str,
) -> String {
    if ability.is_keyword() {
        let keyword = ability
            .display()
            .trim()
            .trim_end_matches('.')
            .to_ascii_lowercase();
        let verb = if subject_text_uses_have(subject) {
            "have"
        } else {
            "has"
        };
        return format!("{} {verb} {keyword}", capitalize_first(subject));
    }
    capitalize_first(
        describe_static_ability_with_subject(ability, subject)
            .trim()
            .trim_end_matches('.'),
    )
}

fn render_labeled_static_predicates(predicates: &[String]) -> String {
    match predicates {
        [] => String::new(),
        [only] => only.clone(),
        [left, right] => format!("{left} and {right}"),
        _ => {
            let (last, leading) = predicates.split_last().expect("nonempty predicates");
            format!("{}, and {last}", leading.join(", "))
        }
    }
}

/// Coalesce adjacent static abilities that came from the same ability-word
/// wrapper. Equality is structural: both the retained label and the typed
/// runtime condition must match.
fn describe_labeled_static_bundle(abilities: &[Ability], subject: &str) -> Option<(String, usize)> {
    let AbilityKind::Static(first) = &abilities.first()?.kind else {
        return None;
    };
    let (label, first_inner, condition) = first.labeled_static_condition()?;
    let mut bodies = vec![render_labeled_static_body(&first_inner, subject)];
    let mut consumed = 1usize;

    while let Some(Ability {
        kind: AbilityKind::Static(next),
        ..
    }) = abilities.get(consumed)
    {
        let Some((next_label, next_inner, next_condition)) = next.labeled_static_condition() else {
            break;
        };
        if next_label != label || next_condition != condition {
            break;
        }
        bodies.push(render_labeled_static_body(&next_inner, subject));
        consumed += 1;
    }

    let subject = capitalize_first(subject);
    let subject_prefix = format!("{subject} ");
    let predicates = bodies
        .iter()
        .map(|body| body.strip_prefix(&subject_prefix).map(str::to_string))
        .collect::<Option<Vec<_>>>();
    let body = if let Some(predicates) = predicates {
        if predicates
            .iter()
            .all(|predicate| predicate.starts_with("has "))
        {
            let keywords = predicates
                .iter()
                .map(|predicate| predicate.trim_start_matches("has ").to_string())
                .collect::<Vec<_>>();
            format!(
                "{subject} has {}",
                render_labeled_static_predicates(&keywords)
            )
        } else {
            format!(
                "{subject} {}",
                render_labeled_static_predicates(&predicates)
            )
        }
    } else {
        render_labeled_static_predicates(&bodies)
    };

    Some((format!("{label} — {body}"), consumed))
}

fn merge_adjacent_keyword_surface_lines(lines: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut idx = 0usize;
    while idx < lines.len() {
        if let Some((prefix, keyword)) = split_keyword_ability_line(&lines[idx]) {
            let mut keywords = vec![keyword.to_string()];
            let mut consumed = 1usize;
            while let Some(next) = lines.get(idx + consumed) {
                let Some((_, next_keyword)) = split_keyword_ability_line(next) else {
                    break;
                };
                keywords.push(next_keyword.to_string());
                consumed += 1;
            }
            if consumed > 1 {
                out.push(format!(
                    "{prefix}{}",
                    render_intrinsic_keyword_list(&keywords, true)
                ));
                idx += consumed;
                continue;
            }
        }

        if let Some((prefix, keyword)) = split_static_intrinsic_keyword_line(&lines[idx]) {
            let mut keywords = vec![keyword.to_string()];
            let mut consumed = 1usize;
            while let Some(next) = lines.get(idx + consumed) {
                let Some((_, next_keyword)) = split_static_intrinsic_keyword_line(next) else {
                    break;
                };
                keywords.push(next_keyword.to_string());
                consumed += 1;
            }
            if consumed > 1 {
                out.push(format!(
                    "{prefix}{}",
                    render_intrinsic_keyword_list(&keywords, true)
                ));
                idx += consumed;
                continue;
            }
        }

        if let Some(cost) = split_kicker_cost_line(&lines[idx]) {
            let mut costs = vec![cost.to_string()];
            let mut consumed = 1usize;
            while let Some(next) = lines.get(idx + consumed) {
                let Some(next_cost) = split_kicker_cost_line(next) else {
                    break;
                };
                costs.push(next_cost.to_string());
                consumed += 1;
            }
            if consumed > 1 {
                out.push(format!("Kicker {}", costs.join(" and/or ")));
                idx += consumed;
                continue;
            }
        }

        if let Some(keyword) = split_bare_keyword_line(&lines[idx]) {
            let mut keywords = vec![keyword.to_string()];
            let mut consumed = 1usize;
            while let Some(next) = lines.get(idx + consumed) {
                let Some(next_keyword) = split_bare_keyword_line(next) else {
                    break;
                };
                keywords.push(next_keyword.to_string());
                consumed += 1;
            }
            if consumed > 1 {
                out.push(render_bare_keyword_list(&keywords));
                idx += consumed;
                continue;
            }
        }

        if let Some((label, subject, verb, keyword)) =
            split_static_granted_keyword_line(&lines[idx])
        {
            if let Some((label, color, base_subject, keyword)) =
                split_color_static_granted_keyword_line(&lines[idx])
            {
                let mut clauses = vec![(keyword.to_string(), color)];
                let mut consumed = 1usize;
                while let Some(next) = lines.get(idx + consumed) {
                    let Some((_, next_color, next_base_subject, next_keyword)) =
                        split_color_static_granted_keyword_line(next)
                    else {
                        break;
                    };
                    if next_base_subject != base_subject {
                        break;
                    }
                    clauses.push((next_keyword.to_string(), next_color));
                    consumed += 1;
                }
                if consumed > 1 {
                    out.push(render_color_conditional_keyword_grants(
                        label,
                        base_subject,
                        &clauses,
                    ));
                    idx += consumed;
                    continue;
                }
            }

            let mut keywords = vec![keyword.to_string()];
            let mut consumed = 1usize;
            while let Some(next) = lines.get(idx + consumed) {
                let Some((_, next_subject, next_verb, next_keyword)) =
                    split_static_granted_keyword_line(next)
                else {
                    break;
                };
                if next_subject != subject || next_verb != verb {
                    break;
                }
                keywords.push(next_keyword.to_string());
                consumed += 1;
            }
            if consumed > 1 {
                out.push(format!(
                    "{label}{subject} {verb} {}",
                    render_keyword_list(&keywords, false)
                ));
                idx += consumed;
                continue;
            }
        }

        out.push(lines[idx].clone());
        idx += 1;
    }
    out
}

fn split_kicker_cost_line(line: &str) -> Option<&str> {
    let trimmed = line.trim().trim_end_matches('.');
    let cost = trimmed.strip_prefix("Kicker ")?.trim();
    (!cost.is_empty()).then_some(cost)
}

fn split_keyword_ability_line(line: &str) -> Option<(&str, &str)> {
    let (prefix, keyword) = line.split_once(": ")?;
    if !prefix.starts_with("Keyword ability ") || !is_mergeable_keyword_surface(keyword) {
        return None;
    }
    Some((&line[..prefix.len() + 2], keyword.trim_end_matches('.')))
}

fn split_bare_keyword_line(line: &str) -> Option<&str> {
    let keyword = line.trim().trim_end_matches('.');
    if keyword.contains(':') || !is_mergeable_keyword_surface(keyword) {
        return None;
    }
    Some(keyword)
}

fn split_static_intrinsic_keyword_line(line: &str) -> Option<(&str, &str)> {
    let (prefix, keyword) = line.split_once(": ")?;
    if !prefix.starts_with("Static ability ") || !is_mergeable_keyword_surface(keyword) {
        return None;
    }
    Some((&line[..prefix.len() + 2], keyword.trim_end_matches('.')))
}

fn split_static_granted_keyword_line(line: &str) -> Option<(&str, &str, &str, &str)> {
    let (label, text) = line.split_once(": ")?;
    if !label.starts_with("Static ability ") {
        return None;
    }
    let keyword = text.trim_end_matches('.');
    let (subject, verb, keyword) = if let Some((subject, keyword)) = keyword.split_once(" have ") {
        (subject, "have", keyword)
    } else if let Some((subject, keyword)) = keyword.split_once(" has ") {
        (subject, "has", keyword)
    } else {
        return None;
    };
    if !is_mergeable_keyword_surface(keyword) {
        return None;
    }
    Some((&line[..label.len() + 2], subject, verb, keyword))
}

fn split_color_static_granted_keyword_line(line: &str) -> Option<(&str, &'static str, &str, &str)> {
    let (label, text) = line.split_once(": ")?;
    if !label.starts_with("Static ability ") {
        return None;
    }
    let text = text.trim_end_matches('.');
    let (subject, keyword) = text.split_once(" have ")?;
    if !is_mergeable_keyword_surface(keyword) {
        return None;
    }
    let (color, base_subject) = subject.split_once(' ')?;
    let color = match color.to_ascii_lowercase().as_str() {
        "white" => "white",
        "blue" => "blue",
        "black" => "black",
        "red" => "red",
        "green" => "green",
        _ => return None,
    };
    Some((&line[..label.len() + 2], color, base_subject, keyword))
}

fn render_color_conditional_keyword_grants(
    label: &str,
    base_subject: &str,
    clauses: &[(String, &'static str)],
) -> String {
    let clauses = clauses
        .iter()
        .map(|(keyword, color)| {
            format!(
                "{} if it's {color}",
                keyword.trim_end_matches('.').to_ascii_lowercase()
            )
        })
        .collect::<Vec<_>>();
    let subject = if base_subject.eq_ignore_ascii_case("creatures you control") {
        "Each creature you control has".to_string()
    } else {
        format!("{base_subject} have")
    };
    format!("{label}{subject} {}", join_english_list(&clauses))
}

fn ability_level_range_prefix(ability: &Ability) -> Option<String> {
    let AbilityKind::Activated(activated) = &ability.kind else {
        return None;
    };
    level_range_activation_prefix(activated)
}

fn ability_has_level_tiers(ability: &Ability) -> bool {
    matches!(
        &ability.kind,
        AbilityKind::Static(static_ability)
            if static_ability.level_abilities().is_some_and(|levels| !levels.is_empty())
    )
}

fn level_tier_header_range(line: &str) -> Option<String> {
    let (_, text) = line.split_once(": ")?;
    let range = text.trim().trim_end_matches('.');
    range.starts_with("Level ").then(|| range.to_string())
}

fn interleave_level_range_activations(
    lines: Vec<String>,
    abilities: &[Ability],
    subject: &str,
    rewrite_it_deals: bool,
) -> Vec<String> {
    let mut out = Vec::new();
    let mut active_range: Option<String> = None;
    for line in lines {
        if let Some(next_range) = level_tier_header_range(&line) {
            flush_level_range_activations(
                &mut out,
                active_range.as_deref(),
                abilities,
                subject,
                rewrite_it_deals,
            );
            active_range = Some(next_range);
        }
        out.push(line);
    }
    flush_level_range_activations(
        &mut out,
        active_range.as_deref(),
        abilities,
        subject,
        rewrite_it_deals,
    );
    out
}

fn flush_level_range_activations(
    out: &mut Vec<String>,
    range: Option<&str>,
    abilities: &[Ability],
    subject: &str,
    rewrite_it_deals: bool,
) {
    let Some(range) = range else {
        return;
    };
    let range_prefix = format!("{range}. ");
    for (idx, ability) in abilities.iter().enumerate() {
        if ability_level_range_prefix(ability).as_deref() != Some(range) {
            continue;
        }
        for line in describe_ability(idx + 1, ability, subject, rewrite_it_deals) {
            out.push(
                line.strip_prefix(&range_prefix)
                    .unwrap_or(line.as_str())
                    .to_string(),
            );
        }
    }
}

fn is_mergeable_keyword_surface(keyword: &str) -> bool {
    let keyword = keyword.trim_end_matches('.');
    let lower = keyword.to_ascii_lowercase();
    let is_numbered_firebending = lower
        .strip_prefix("firebending ")
        .is_some_and(|amount| amount.parse::<u32>().is_ok());
    (is_keyword_phrase(keyword) && lower != "changeling")
        || is_numbered_firebending
        || matches!(
            lower.as_str(),
            "flying"
                | "first strike"
                | "double strike"
                | "vigilance"
                | "trample"
                | "haste"
                | "lifelink"
                | "deathtouch"
                | "menace"
                | "reach"
                | "hexproof"
                | "indestructible"
                | "shroud"
                | "fear"
                | "ward pay 3 life"
        )
}

fn render_keyword_list(keywords: &[String], capitalize: bool) -> String {
    render_keyword_list_with_separator(keywords, capitalize, KeywordListSeparator::EnglishAnd)
}

#[derive(Clone, Copy)]
enum KeywordListSeparator {
    Comma,
    EnglishAnd,
}

fn render_intrinsic_keyword_list(keywords: &[String], capitalize: bool) -> String {
    render_keyword_list_with_separator(keywords, capitalize, KeywordListSeparator::Comma)
}

fn render_keyword_list_with_separator(
    keywords: &[String],
    capitalize: bool,
    separator: KeywordListSeparator,
) -> String {
    let mut items = Vec::new();
    let mut protections = Vec::new();
    for keyword in keywords {
        let lower = keyword.trim_end_matches('.').to_ascii_lowercase();
        if let Some(from) = lower.strip_prefix("protection from ") {
            protections.push(from.to_string());
        } else {
            items.push(lower);
        }
    }
    if !protections.is_empty() {
        let protection = if protections.len() == 1 {
            format!("protection from {}", protections[0])
        } else {
            format!(
                "protection from {}",
                join_english_list(
                    &protections
                        .iter()
                        .enumerate()
                        .map(|(idx, value)| {
                            if idx == 0 {
                                value.clone()
                            } else {
                                format!("from {value}")
                            }
                        })
                        .collect::<Vec<_>>()
                )
            )
        };
        items.push(protection);
    }
    let rendered = match separator {
        KeywordListSeparator::Comma => items.join(", "),
        KeywordListSeparator::EnglishAnd => join_english_list(&items),
    };
    if capitalize {
        capitalize_first(&rendered)
    } else {
        rendered
    }
}

fn render_bare_keyword_list(keywords: &[String]) -> String {
    let mut items = Vec::new();
    for keyword in keywords {
        let lower = keyword.trim_end_matches('.').to_ascii_lowercase();
        if !items.iter().any(|existing| existing == &lower) {
            items.push(lower);
        }
    }
    capitalize_first(&items.join(", "))
}

fn join_english_list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [first, second] => format!("{first} and {second}"),
        _ => {
            let (last, rest) = items.split_last().expect("nonempty");
            format!("{}, and {}", rest.join(", "), last)
        }
    }
}

pub(super) fn describe_resolution_program(
    program: &crate::resolution::ResolutionProgram,
) -> String {
    if let Some(rendered) = describe_spell_mastery_reanimation_program(program) {
        return rendered;
    }
    if let Some(rendered) = describe_group_pump_then_conditional_untap_program(program) {
        return rendered;
    }

    let mut rendered_segments = Vec::new();
    for segment in &program.segments {
        if segment.self_replacements.len() == 1 {
            let rendered = describe_single_self_replacement_segment(segment).unwrap_or_else(|| {
                let branch = &segment.self_replacements[0];
                describe_effect_list(&[Effect::conditional(
                    branch.condition.clone(),
                    branch.replacement_effects.clone(),
                    segment.default_effects.clone(),
                )])
            });
            rendered_segments.push(apply_self_replacement_presentation_label(
                &segment.self_replacements[0],
                rendered,
            ));
            continue;
        }

        if !segment.default_effects.is_empty() {
            if let Some(rendered) = describe_conjoined_same_source_damage(&segment.default_effects)
            {
                rendered_segments.push(rendered);
                continue;
            }
            if let Some(rendered) =
                describe_target_player_exile_hand_delayed_return(&segment.default_effects)
            {
                rendered_segments.push(rendered);
                continue;
            }
            if let [effect] = segment.default_effects.as_slice()
                && effect
                    .downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
                    .is_some_and(|schedule| schedule.target_tag.is_some())
            {
                rendered_segments.push(describe_effect(effect));
                continue;
            }
            if let [exile_top_effect, grant_play_effect, grant_free_cast_effect] =
                segment.default_effects.as_slice()
                && let Some(exile_top) =
                    exile_top_effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()
                && let Some(grant_play) =
                    grant_play_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
                && let Some(grant_free_cast) = grant_free_cast_effect
                    .downcast_ref::<crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect>()
                && let Some(rendered) =
                    describe_exile_top_then_play_without_paying_mana(exile_top, grant_play, grant_free_cast)
            {
                rendered_segments.push(rendered);
                continue;
            }
            if let [create_effect, manifest_effect] = segment.default_effects.as_slice()
                && let Some(create) =
                    create_effect.downcast_ref::<crate::effects::CreateTokenEffect>()
                && let Some(manifest) =
                    manifest_effect.downcast_ref::<crate::effects::ManifestTopCardOfLibraryEffect>()
                && let Some(rendered) =
                    describe_create_token_and_manifest_top_card(create, manifest)
            {
                rendered_segments.push(rendered);
                continue;
            }
            if let Some(rendered) =
                describe_chosen_name_consult_after_top_exile_effects(&segment.default_effects)
            {
                rendered_segments.push(rendered);
                continue;
            }
            if let Some(rendered) =
                describe_reveal_hand_choose_discard_then_random_effects(&segment.default_effects)
            {
                rendered_segments.push(rendered);
                continue;
            }
            if let Some(rendered) =
                describe_choose_sacrifice_then_source_damage_effects(&segment.default_effects)
            {
                rendered_segments.push(rendered);
                continue;
            }
            rendered_segments.push(
                describe_pre_clause_structural_effect_list(&segment.default_effects)
                    .or_else(|| {
                        describe_structural_multisentence_effect_list(&segment.default_effects)
                    })
                    .or_else(|| {
                        render_consult_reveal_move_matches_then_bottom(&segment.default_effects)
                    })
                    .or_else(|| {
                        describe_effect_clause_list(&segment.default_effects)
                            .map(|text| capitalize_first(&text))
                    })
                    .unwrap_or_else(|| describe_effect_list(&segment.default_effects)),
            );
        }
        for branch in &segment.self_replacements {
            rendered_segments.push(describe_effect_list(&branch.replacement_effects));
        }
    }
    rendered_segments
        .into_iter()
        .map(|segment| segment.trim().trim_start_matches(". ").to_string())
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join(". ")
}

fn describe_group_pump_then_conditional_untap_program(
    program: &crate::resolution::ResolutionProgram,
) -> Option<String> {
    let [pump_segment, untap_segment] = program.segments.as_slice() else {
        return None;
    };
    if !pump_segment.self_replacements.is_empty()
        || !untap_segment.self_replacements.is_empty()
        || pump_segment.default_effects.len() != 1
        || untap_segment.default_effects.len() != 1
    {
        return None;
    }

    let tagged_pump =
        pump_segment.default_effects[0].downcast_ref::<crate::effects::TaggedEffect>()?;
    let pump = tagged_pump
        .effect
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let crate::continuous::EffectTarget::Filter(pump_filter) = &pump.target else {
        return None;
    };
    if pump_filter.card_types != [CardType::Creature] {
        return None;
    }

    let conditional =
        untap_segment.default_effects[0].downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !matches!(
        &conditional.condition,
        Condition::Not(inner) if matches!(inner.as_ref(), Condition::YourTurn)
    ) || !conditional.if_false.is_empty()
        || conditional.if_true.len() != 1
    {
        return None;
    }
    let tagged_untap = conditional.if_true[0].downcast_ref::<crate::effects::TaggedEffect>()?;
    let untap = tagged_untap
        .effect
        .downcast_ref::<crate::effects::UntapEffect>()?;
    let ChooseSpec::Object(untap_filter) = untap.target.base() else {
        return None;
    };
    if !untap_filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == tagged_pump.tag
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    }) {
        return None;
    }

    Some(format!(
        "{}. If it's not your turn, untap those creatures",
        describe_effect_list(&pump_segment.default_effects).trim_end_matches('.')
    ))
}

pub(super) fn describe_mana_ability_resolution_program(
    program: &crate::resolution::ResolutionProgram,
) -> Option<String> {
    if program.segments.len() != 1 {
        return None;
    }
    let segment = &program.segments[0];
    if segment.self_replacements.len() != 1 || segment.default_effects.is_empty() {
        return None;
    }
    let branch = &segment.self_replacements[0];
    let default_text = describe_effect_list(&segment.default_effects);
    let replacement_text = describe_effect_list(&branch.replacement_effects);
    let condition_text = super::normalize_common::describe_condition(&branch.condition);
    Some(format!(
        "{default_text}. If {condition_text}, {} instead",
        super::normalize_common::lowercase_first(&replacement_text)
    ))
}

fn is_standard_gift_render_payload(lower: &str) -> bool {
    lower.contains("chosen player draws a card")
        || lower.contains("chosen player creates a treasure token")
        || lower.contains("create a treasure token under the chosen player's control")
        || lower.contains("chosen player creates a food token")
        || lower.contains("create a food token under the chosen player's control")
        || lower.contains("chosen player creates a tapped 1/1 blue fish creature token")
        || lower.contains("chosen player creates a 1/1 blue fish creature token, tapped")
        || lower.contains("create a 1/1 blue fish creature token under the chosen player's control")
        || lower.contains("chosen player takes an extra turn after this one")
        || lower.contains("chosen player creates an 8/8 blue octopus creature token")
        || lower
            .contains("create an 8/8 blue octopus creature token under the chosen player's control")
        || lower
            .contains("create a 8/8 blue octopus creature token under the chosen player's control")
}

fn is_hidden_gift_resolution_segment(segment: &crate::resolution::ResolutionSegment) -> bool {
    if !segment.self_replacements.is_empty() || segment.default_effects.is_empty() {
        return false;
    }

    let lower = describe_effect_list(&segment.default_effects).to_ascii_lowercase();
    lower.starts_with("if the gift was promised") && is_standard_gift_render_payload(&lower)
}

fn is_hidden_gift_etb_ability(ability: &Ability) -> bool {
    let AbilityKind::Triggered(triggered) = &ability.kind else {
        return false;
    };
    if !matches!(
        triggered.intervening_if.as_ref(),
        Some(Condition::ThisSpellPaidLabel(label))
            if label.display_label().eq_ignore_ascii_case("Gift")
    ) || !triggered.choices.is_empty()
        || !trigger_is_this_enters_battlefield(&triggered.trigger)
    {
        return false;
    }
    let rendered = describe_resolution_program(&triggered.effects).to_ascii_lowercase();
    rendered.starts_with("if the gift was promised") || is_standard_gift_render_payload(&rendered)
}

fn apply_self_replacement_presentation_label(
    branch: &crate::resolution::SelfReplacementBranch,
    line: String,
) -> String {
    let Some(label) = branch
        .presentation_label
        .as_ref()
        .and_then(crate::ability::PresentationLabel::display_prefix)
    else {
        return line;
    };
    let label = label.trim();
    if label.is_empty() || label.starts_with("__ironsmith_") || line.contains(&format!("{label} —"))
    {
        return line;
    }
    if branch.condition_after_replacement {
        if let Some((default, conditional)) = line.split_once(". If ")
            && let Some((condition, replacement)) = conditional.split_once(", ")
            && let Some(replacement) = replacement.strip_suffix(" instead")
        {
            return format!(
                "{default}. {label} — {} instead if {condition}",
                capitalize_first(replacement)
            );
        }
        if let Some((default, replacement)) = line.split_once(". ")
            && replacement.contains(" instead if ")
        {
            return format!("{default}. {label} — {replacement}");
        }
    }
    if let Some((default, replacement)) = line.split_once(". If ") {
        return format!("{default}. {label} — If {replacement}");
    }
    format!("{label} — {line}")
}

fn describe_conjoined_same_source_damage(effects: &[Effect]) -> Option<String> {
    let effects = match effects {
        [only] => {
            let sequence = unwrap_basic_render_wrapper(only)
                .downcast_ref::<crate::effects::SequenceEffect>()?;
            if matches!(
                sequence.surface,
                ironsmith_core::SequenceSurface::Sequential
            ) {
                return None;
            }
            sequence.effects.as_slice()
        }
        _ => effects,
    };
    let [first, second] = effects else {
        return None;
    };
    let compact = describe_joint_subject_pair(first, second)?;
    compact
        .strip_prefix("this deals ")
        .map(|rest| format!("Deal {rest}"))
}

fn format_self_replacement_fallback(
    default_text: &str,
    condition_text: &str,
    replacement: &str,
) -> String {
    if replacement.contains(". ") {
        format!("{default_text}. If {condition_text}, instead {replacement}")
    } else {
        format!("{default_text}. If {condition_text}, {replacement} instead")
    }
}

fn describe_single_self_replacement_segment(
    segment: &crate::resolution::ResolutionSegment,
) -> Option<String> {
    if segment.self_replacements.len() != 1 || segment.default_effects.is_empty() {
        return None;
    }
    let branch = &segment.self_replacements[0];
    let compact_default = describe_conjoined_same_source_damage(&segment.default_effects);
    let compact_replacement = describe_conjoined_same_source_damage(&branch.replacement_effects);
    if let (Some(default_text), Some(replacement_text)) = (&compact_default, &compact_replacement) {
        let raw_condition_text = super::normalize_common::describe_condition(&branch.condition);
        let condition_text = normalize_target_quality_condition(default_text, &raw_condition_text);
        return Some(format_self_replacement_fallback(
            default_text,
            &condition_text,
            &super::normalize_common::lowercase_first(replacement_text),
        ));
    }
    let conditional = Effect::conditional(
        branch.condition.clone(),
        branch.replacement_effects.clone(),
        segment.default_effects.clone(),
    );
    let conditional_text = describe_effect_list(&[conditional]);
    let loses_repeated_set_target =
        conditional_text
            .split_once(". ")
            .is_some_and(|(default, replacement)| {
                default.contains(" damage to each ")
                    && replacement.contains(" damage instead if ")
                    && !replacement.contains(" damage to each ")
            });
    if conditional_text.contains(" damage instead if ") && !loses_repeated_set_target {
        return Some(conditional_text);
    }
    let default_text =
        compact_default.unwrap_or_else(|| describe_effect_list(&segment.default_effects));
    let replacement_text =
        compact_replacement.unwrap_or_else(|| describe_effect_list(&branch.replacement_effects));
    let raw_condition_text = super::normalize_common::describe_condition(&branch.condition);
    let condition_text = normalize_target_quality_condition(&default_text, &raw_condition_text);
    if let Some(return_text) = describe_same_target_hand_to_battlefield_replacement(
        &segment.default_effects,
        &branch.replacement_effects,
        &default_text,
        &condition_text,
        branch.condition_after_replacement,
    ) {
        return Some(return_text);
    }
    if let Some(search_destination_text) =
        describe_shared_search_hand_to_battlefield_self_replacement(
            &segment.default_effects,
            &branch.replacement_effects,
            &default_text,
            &condition_text,
        )
    {
        return Some(search_destination_text);
    }
    if let Some(looked_cards_text) = describe_looked_cards_non_hand_self_replacement(
        &segment.default_effects,
        &branch.replacement_effects,
        &condition_text,
    ) {
        return Some(looked_cards_text);
    }
    if let Some(local_rewrite_text) = describe_rendered_optional_zone_rewrite_self_replacement(
        &default_text,
        &replacement_text,
        &condition_text,
    ) {
        return Some(local_rewrite_text);
    }
    if let Some(count_override_text) = describe_rendered_count_override_self_replacement(
        &default_text,
        &replacement_text,
        &condition_text,
    ) {
        return Some(count_override_text);
    }
    if let Some(mill_override_text) = describe_mill_count_override_self_replacement(
        &segment.default_effects,
        &branch.replacement_effects,
        &default_text,
        &replacement_text,
        &condition_text,
    ) {
        return Some(mill_override_text);
    }
    if let Some(draw_discard_text) = describe_target_player_draw_discard_self_replacement(
        &segment.default_effects,
        &branch.replacement_effects,
        &condition_text,
    ) {
        return Some(draw_discard_text);
    }
    if let Some(phase_out_text) =
        describe_phase_out_exile_self_replacement(&default_text, &replacement_text, &condition_text)
    {
        return Some(phase_out_text);
    }
    if let Some(gets_text) =
        describe_rendered_gets_self_replacement(&default_text, &replacement_text, &condition_text)
    {
        return Some(gets_text);
    }
    if let Some(destroy_text) = describe_tagged_target_set_destroy_self_replacement(
        &segment.default_effects,
        &branch.replacement_effects,
        &default_text,
        &condition_text,
    ) {
        return Some(destroy_text);
    }
    if let Some(damage_text) =
        describe_rendered_damage_self_replacement(&default_text, &replacement_text, &condition_text)
    {
        return Some(damage_text);
    }
    if let Some(counter_unless_text) = describe_counter_unless_self_replacement(
        &segment.default_effects,
        &branch.replacement_effects,
        &default_text,
        &condition_text,
    ) {
        return Some(counter_unless_text);
    }
    if let Some(counter_bonus_text) = describe_counter_bonus_self_replacement(
        &segment.default_effects,
        &branch.replacement_effects,
        &default_text,
        &condition_text,
    ) {
        return Some(counter_bonus_text);
    }
    if let Some(token_life_text) = describe_token_life_self_replacement(
        &segment.default_effects,
        &branch.replacement_effects,
        &condition_text,
    ) {
        return Some(token_life_text);
    }
    if let Some(void_text) = describe_void_self_replacement(
        &segment.default_effects,
        &branch.replacement_effects,
        &branch.condition,
        &condition_text,
    ) {
        return Some(void_text);
    }
    if let Some(shared_shuffle_text) = describe_shared_terminal_shuffle_self_replacement(
        &default_text,
        &replacement_text,
        &condition_text,
    ) {
        return Some(shared_shuffle_text);
    }
    let mut replacement =
        rewrite_self_replacement_referent_phrase(&default_text, &replacement_text);
    if condition_text.starts_with("that creature is ")
        && replacement.starts_with("that creature gets ")
    {
        replacement = replacement.replacen("that creature", "it", 1);
    }
    if condition_text.starts_with("that creature has ") {
        if replacement.starts_with("that creature gets ") {
            replacement = replacement.replacen("that creature", "it", 1);
        } else if replacement.starts_with("that creature you control gets ") {
            replacement = replacement.replacen("that creature you control", "it", 1);
        }
    }
    Some(format_self_replacement_fallback(
        &default_text,
        &condition_text,
        &replacement,
    ))
}

/// Recognize a complete search/reveal/move/shuffle replacement pipeline whose
/// only semantic change is the searched collection's destination.  The full
/// structural comparison is intentional: it lets compiled text express the
/// local "instead of putting them into your hand" replacement without hiding
/// a changed selector, count, reveal, tag, or shuffle instruction.
fn describe_shared_search_hand_to_battlefield_self_replacement(
    default_effects: &[Effect],
    replacement_effects: &[Effect],
    default_text: &str,
    condition_text: &str,
) -> Option<String> {
    struct Pipeline<'a> {
        choose: &'a crate::effects::ChooseObjectsEffect,
        reveal: &'a crate::effects::RevealTaggedEffect,
        for_each_tag: &'a TagKey,
        move_wrapper_tag: Option<&'a TagKey>,
        move_to_zone: &'a crate::effects::MoveToZoneEffect,
        shuffle: &'a crate::effects::ShuffleLibraryEffect,
    }

    fn move_with_wrapper_tag(
        effect: &Effect,
    ) -> Option<(Option<&TagKey>, &crate::effects::MoveToZoneEffect)> {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return Some((
                Some(&tagged.tag),
                tagged
                    .effect
                    .downcast_ref::<crate::effects::MoveToZoneEffect>()?,
            ));
        }
        Some((
            None,
            effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?,
        ))
    }

    fn pipeline(effects: &[Effect]) -> Option<Pipeline<'_>> {
        let [choose_effect, reveal_effect, for_each_effect, shuffle_effect] = effects else {
            return None;
        };
        let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
        let reveal = reveal_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()?;
        let for_each = for_each_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
        let [move_effect] = for_each.effects.as_slice() else {
            return None;
        };
        let (move_wrapper_tag, move_to_zone) = move_with_wrapper_tag(move_effect)?;
        let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
        if !choose.is_search
            || reveal.tag != choose.tag
            || for_each.tag != choose.tag
            || !matches!(move_to_zone.target.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag)
        {
            return None;
        }
        Some(Pipeline {
            choose,
            reveal,
            for_each_tag: &for_each.tag,
            move_wrapper_tag,
            move_to_zone,
            shuffle,
        })
    }

    let default = pipeline(default_effects)?;
    let replacement = pipeline(replacement_effects)?;
    let mut replacement_move_at_default_destination = replacement.move_to_zone.clone();
    replacement_move_at_default_destination.zone = default.move_to_zone.zone;
    if default.choose != replacement.choose
        || default.reveal != replacement.reveal
        || default.for_each_tag != replacement.for_each_tag
        || default.move_wrapper_tag != replacement.move_wrapper_tag
        || default.shuffle != replacement.shuffle
        || default.move_to_zone != &replacement_move_at_default_destination
        || default.move_to_zone.zone != Zone::Hand
        || replacement.move_to_zone.zone != Zone::Battlefield
    {
        return None;
    }

    Some(format!(
        "{}. If {condition_text}, put those cards onto the battlefield instead of putting them into your hand",
        default_text.trim().trim_end_matches('.')
    ))
}

fn describe_same_target_hand_to_battlefield_replacement(
    default_effects: &[Effect],
    replacement_effects: &[Effect],
    default_text: &str,
    condition_text: &str,
    condition_after_replacement: bool,
) -> Option<String> {
    let [default_effect] = default_effects else {
        return None;
    };
    let [replacement_effect] = replacement_effects else {
        return None;
    };
    let default_tagged = default_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let replacement_tagged = replacement_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    if default_tagged.tag != replacement_tagged.tag {
        return None;
    }
    let return_to_hand = default_tagged
        .effect
        .downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()?;
    let return_to_battlefield = replacement_tagged
        .effect
        .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()?;
    if return_to_hand.target.unhinted() != return_to_battlefield.target.unhinted()
        || return_to_battlefield.as_aura.is_some()
        || !return_to_battlefield.enters_with_counters.is_empty()
    {
        return None;
    }
    let replacement = format!(
        "return that card to the battlefield{} instead",
        if return_to_battlefield.tapped {
            " tapped"
        } else {
            ""
        }
    );
    if condition_after_replacement {
        Some(format!(
            "{default_text}. {} if {condition_text}",
            capitalize_first(&replacement)
        ))
    } else {
        Some(format!(
            "{default_text}. If {condition_text}, {replacement}"
        ))
    }
}

fn describe_shared_terminal_shuffle_self_replacement(
    default_text: &str,
    replacement_text: &str,
    condition_text: &str,
) -> Option<String> {
    fn ends_with_shuffle(text: &str) -> bool {
        text.trim_end_matches('.')
            .to_ascii_lowercase()
            .ends_with("then shuffle")
    }

    if !ends_with_shuffle(default_text) || !ends_with_shuffle(replacement_text) {
        return None;
    }

    Some(format!(
        "{default_text}. If {condition_text}, instead {}",
        super::normalize_common::lowercase_first(replacement_text)
    ))
}

fn describe_phase_out_exile_self_replacement(
    default_text: &str,
    replacement_text: &str,
    condition_text: &str,
) -> Option<String> {
    let target = default_text.strip_prefix("Phase out ")?;
    let replacement_target = replacement_text.strip_prefix("Exile ")?;
    if !target.eq_ignore_ascii_case(replacement_target) {
        return None;
    }
    let condition = condition_text
        .strip_prefix("it's a ")
        .and_then(|quality| quality.strip_suffix(" permanent"))
        .map(|quality| format!("that permanent is {quality}"))
        .unwrap_or_else(|| condition_text.to_string());
    Some(format!(
        "{} phases out. If {condition}, exile it instead",
        capitalize_first(target)
    ))
}

fn normalize_target_quality_condition(default_text: &str, condition_text: &str) -> String {
    let mut creature_quality = None;
    if let Some(quality) = condition_text.strip_prefix("it's a ") {
        creature_quality = Some(quality);
    }
    let Some(quality) = condition_text
        .strip_prefix("it's a ")
        .and_then(|rest| rest.strip_suffix(" permanent"))
    else {
        let default_lower = default_text.to_ascii_lowercase();
        if default_lower.contains("target") && default_lower.contains("creature") {
            if let Some(quality) = creature_quality {
                if let Some(rest) = quality.strip_prefix("creature with ") {
                    return format!("that creature has {rest}");
                }
                if let Some(rest) = quality.strip_prefix("permanent with ") {
                    return format!("that creature has {rest}");
                }
                if default_lower.starts_with("target creature gets")
                    || default_lower.starts_with("target creature you control gets")
                {
                    return condition_text.to_string();
                }
                return format!("that creature is a {quality}");
            }
        }
        return condition_text.to_string();
    };
    if !quality.split(" or ").all(is_color_quality_word) {
        return condition_text.to_string();
    }
    let default_lower = default_text.to_ascii_lowercase();
    let subject = if default_lower.contains("target creature or planeswalker")
        || default_lower.contains("target permanent")
    {
        "that permanent"
    } else if default_lower.contains("target creature") {
        "that creature"
    } else {
        return condition_text.to_string();
    };
    format!("{subject} is {quality}")
}

fn is_color_quality_word(word: &str) -> bool {
    matches!(
        word.trim(),
        "white" | "blue" | "black" | "red" | "green" | "colorless" | "multicolored"
    )
}

fn describe_void_self_replacement(
    default_effects: &[Effect],
    replacement_effects: &[Effect],
    condition: &Condition,
    condition_text: &str,
) -> Option<String> {
    if !is_void_condition(condition) {
        return None;
    }
    let replacement_clause = describe_void_replacement_clause(replacement_effects)?;
    Some(format!(
        "{}. Void — If {condition_text}, instead {replacement_clause}",
        describe_effect_list(default_effects)
    ))
}

fn is_void_condition(condition: &Condition) -> bool {
    let Condition::Or(left, right) = condition else {
        return false;
    };
    (matches!(
        left.as_ref(),
        Condition::NonlandPermanentLeftBattlefieldThisTurn
    ) && matches!(right.as_ref(), Condition::SpellWasWarpedThisTurn))
        || (matches!(left.as_ref(), Condition::SpellWasWarpedThisTurn)
            && matches!(
                right.as_ref(),
                Condition::NonlandPermanentLeftBattlefieldThisTurn
            ))
}

fn describe_void_replacement_clause(effects: &[Effect]) -> Option<String> {
    if let [draw_effect, for_players_effect] = effects {
        let draw = unwrap_basic_render_wrapper(draw_effect)
            .downcast_ref::<crate::effects::DrawCardsEffect>()?;
        let for_players = unwrap_basic_render_wrapper(for_players_effect)
            .downcast_ref::<crate::effects::ForPlayersEffect>()?;
        let [inner_effect] = for_players.effects.as_slice() else {
            return None;
        };
        let lose = unwrap_basic_render_wrapper(inner_effect)
            .downcast_ref::<crate::effects::LoseLifeEffect>()?;
        if matches!(
            draw.player,
            PlayerFilter::You | PlayerFilter::EffectController
        ) && for_players.filter == PlayerFilter::Opponent
            && matches!(
                lose.player,
                ChooseSpec::Player(PlayerFilter::IteratedPlayer)
            )
        {
            return Some(format!(
                "you draw {} and each opponent loses {} life",
                describe_card_count(&draw.count),
                describe_value(&lose.amount)
            ));
        }
    }

    let mut rendered = super::normalize_common::lowercase_first(
        describe_effect_list(effects).trim_end_matches('.'),
    )
    .replace(". Each ", " and each ")
    .replace(". each ", " and each ");
    if rendered.starts_with("draw ") {
        rendered = format!("you {rendered}");
    }
    Some(rendered)
}

fn describe_looked_cards_non_hand_self_replacement(
    default_effects: &[Effect],
    replacement_effects: &[Effect],
    condition_text: &str,
) -> Option<String> {
    if condition_text != "this spell was cast from anywhere other than your hand" {
        return None;
    }

    let [default_look, choose, move_chosen, remainder] = default_effects else {
        return None;
    };
    let [replacement_look, move_all] = replacement_effects else {
        return None;
    };

    let default_look = default_look.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    let replacement_look =
        replacement_look.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    if default_look != replacement_look
        || default_look.reveal
        || default_look.player != PlayerFilter::You
    {
        return None;
    }

    let choose = choose.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if choose.count.min != 1
        || choose.count.max != Some(1)
        || choose.chooser != PlayerFilter::You
        || choose.filter.zone != Some(Zone::Library)
        || !choose
            .filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag == default_look.tag)
    {
        return None;
    }

    let move_chosen =
        unwrap_tagged_effect(move_chosen).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_chosen.zone != Zone::Hand
        || !matches!(&move_chosen.target, ChooseSpec::Tagged(tag) if tag == &choose.tag)
    {
        return None;
    }

    let remainder =
        remainder.downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    if remainder.tag != default_look.tag
        || remainder.keep_tagged.as_ref() != Some(&choose.tag)
        || remainder.player != PlayerFilter::You
    {
        return None;
    }

    let move_all =
        unwrap_tagged_effect(move_all).downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_all.zone != Zone::Hand
        || !matches!(&move_all.target, ChooseSpec::Tagged(tag) if tag == &default_look.tag)
    {
        return None;
    }

    let count_text = describe_value(&default_look.count);
    let order_suffix = match remainder.order {
        crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses => " in any order",
        crate::effects::consult_helpers::LibraryBottomOrder::Random => " in a random order",
    };
    Some(format!(
        "Look at the top {count_text} cards of your library. Put one of those cards into your hand and the rest on the bottom of your library{order_suffix}. If {condition_text}, put each of those cards into your hand instead"
    ))
}

fn describe_token_life_self_replacement(
    default_effects: &[Effect],
    replacement_effects: &[Effect],
    condition_text: &str,
) -> Option<String> {
    let default_create = token_life_create_effect(default_effects)?;
    let replacement_create = token_life_create_effect(replacement_effects)?;
    if !same_token_life_replacement_token(default_create, replacement_create) {
        return None;
    }

    let default_clause = describe_token_life_clause(default_effects, false)?;
    let replacement_clause = describe_token_life_clause(replacement_effects, true)?;
    Some(format!(
        "{default_clause}. If {condition_text}, instead {replacement_clause}"
    ))
}

fn token_life_create_effect(effects: &[Effect]) -> Option<&crate::effects::CreateTokenEffect> {
    let [create_effect, gain_effect] = effects else {
        return None;
    };
    let create =
        unwrap_tagged_effect(create_effect).downcast_ref::<crate::effects::CreateTokenEffect>()?;
    let gain =
        unwrap_tagged_effect(gain_effect).downcast_ref::<crate::effects::GainLifeEffect>()?;
    if create.controller != PlayerFilter::You
        || create.controller_target.is_some()
        || create.enters_attacking
        || create.exile_at_end_of_combat
        || create.sacrifice_at_end_of_combat
        || create.sacrifice_at_next_end_step
        || create.exile_at_next_end_step
        || gain.player != ChooseSpec::Player(PlayerFilter::You)
    {
        return None;
    }
    Some(create)
}

fn same_token_life_replacement_token(
    default_create: &crate::effects::CreateTokenEffect,
    replacement_create: &crate::effects::CreateTokenEffect,
) -> bool {
    describe_token_blueprint(&default_create.token)
        == describe_token_blueprint(&replacement_create.token)
        && default_create.enters_tapped == replacement_create.enters_tapped
}

fn describe_token_life_clause(effects: &[Effect], refer_to_prior_token: bool) -> Option<String> {
    let [create_effect, gain_effect] = effects else {
        return None;
    };
    let create =
        unwrap_tagged_effect(create_effect).downcast_ref::<crate::effects::CreateTokenEffect>()?;
    let gain =
        unwrap_tagged_effect(gain_effect).downcast_ref::<crate::effects::GainLifeEffect>()?;
    let created = if refer_to_prior_token {
        format!("{} of those tokens", describe_value(&create.count))
    } else {
        describe_create_token_amount(create)
    };
    Some(format!(
        "create {created} and you gain {} life",
        describe_value(&gain.amount)
    ))
}

fn describe_create_token_amount(create: &crate::effects::CreateTokenEffect) -> String {
    let mut token = describe_token_blueprint(&create.token);
    if create.enters_tapped {
        token = format!("tapped {token}");
    }
    match create.count.unhinted() {
        Value::Fixed(1) => format!("a {}", singular_token_phrase(&token)),
        Value::Fixed(n) => {
            let count = number_word(*n).unwrap_or_else(|| n.to_string());
            format!("{count} {}", plural_token_phrase(&token))
        }
        _ => format!(
            "{} {}",
            describe_value(&create.count),
            plural_token_phrase(&token)
        ),
    }
}

fn singular_token_phrase(token: &str) -> String {
    if token.ends_with(" token") {
        token.to_string()
    } else {
        format!("{token} token")
    }
}

fn plural_token_phrase(token: &str) -> String {
    if let Some(stem) = token.strip_suffix(" token") {
        format!("{stem} tokens")
    } else {
        format!("{token} tokens")
    }
}

fn unwrap_tagged_effect(mut effect: &Effect) -> &Effect {
    while let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        effect = &tagged.effect;
    }
    effect
}

fn counter_effect_target(effect: &Effect) -> Option<&ChooseSpec> {
    unwrap_tagged_effect(effect)
        .downcast_ref::<crate::effects::CounterEffect>()
        .map(|counter| &counter.target)
}

fn unless_pays_counter_target(effect: &Effect) -> Option<&ChooseSpec> {
    let unless_pays =
        unwrap_tagged_effect(effect).downcast_ref::<crate::effects::UnlessPaysEffect>()?;
    let [counter_effect] = unless_pays.effects.as_slice() else {
        return None;
    };
    counter_effect_target(counter_effect)
}

fn counter_replacement_referent(target: &ChooseSpec) -> Option<&'static str> {
    let target_text = super::normalize_common::describe_choose_spec(target).to_ascii_lowercase();
    if target_text.contains(" spell or ability") || target_text.contains(" ability or spell") {
        Some("that spell or ability")
    } else if target_text.contains(" ability") && !target_text.contains(" spell") {
        Some("that ability")
    } else if target_text.contains(" spell") {
        Some("that spell")
    } else {
        None
    }
}

fn describe_counter_unless_self_replacement(
    default_effects: &[Effect],
    replacement_effects: &[Effect],
    default_text: &str,
    condition_text: &str,
) -> Option<String> {
    let [default_effect] = default_effects else {
        return None;
    };
    let replacement_counter_target = counter_effect_target(replacement_effects.first()?)?;
    let default_counter_target = unless_pays_counter_target(default_effect)?;
    if replacement_counter_target != default_counter_target {
        return None;
    }

    let referent = counter_replacement_referent(default_counter_target)?;
    let mut replacement_text = format!("instead counter {referent}");
    if replacement_effects.len() > 1 {
        let followup_text = describe_effect_list(&replacement_effects[1..]);
        replacement_text.push_str(", then ");
        replacement_text.push_str(&super::normalize_common::lowercase_first(&followup_text));
    }

    Some(format!(
        "{default_text}. If {condition_text}, {replacement_text}"
    ))
}

fn normalize_counter_bonus_condition(condition_text: &str) -> String {
    for prefix in [
        "it's a permanent that targets ",
        "it's a spell that targets ",
        "it targets ",
    ] {
        if let Some(rest) = condition_text.strip_prefix(prefix) {
            return format!(
                "that spell targets {}",
                rest.replace("commander permanent", "commander")
            );
        }
    }
    condition_text.to_string()
}

fn describe_counter_bonus_self_replacement(
    default_effects: &[Effect],
    replacement_effects: &[Effect],
    default_text: &str,
    condition_text: &str,
) -> Option<String> {
    let [default_effect] = default_effects else {
        return None;
    };
    let default_counter_target = counter_effect_target(default_effect)?;
    let replacement_counter_target = counter_effect_target(replacement_effects.first()?)?;
    if replacement_counter_target != default_counter_target {
        return None;
    }

    let referent = counter_replacement_referent(default_counter_target)?;
    let condition = normalize_counter_bonus_condition(condition_text);
    let mut replacement_text = format!("instead counter {referent}");
    if replacement_effects.len() > 1 {
        let followup_text = describe_effect_list(&replacement_effects[1..]);
        replacement_text.push_str(", ");
        replacement_text.push_str(&super::normalize_common::lowercase_first(&followup_text));
    }

    Some(format!(
        "{default_text}. If {condition}, {replacement_text}"
    ))
}

fn rewrite_self_replacement_referent_phrase(default_text: &str, replacement_text: &str) -> String {
    let mut replacement = super::normalize_common::lowercase_first(replacement_text);
    if default_text
        .to_ascii_lowercase()
        .contains("target creature")
    {
        for prefix in ["put ", "you may put "] {
            let needle = format!("{prefix}target creature on");
            if replacement.starts_with(&needle) {
                replacement = replacement.replacen("target creature", "it", 1);
                break;
            }
        }
    }
    if default_text
        .to_ascii_lowercase()
        .contains("target permanent")
    {
        for prefix in ["put ", "you may put "] {
            let needle = format!("{prefix}target permanent on");
            if replacement.starts_with(&needle) {
                replacement = replacement.replacen("target permanent", "it", 1);
                break;
            }
        }
    }
    if default_text
        .to_ascii_lowercase()
        .contains("target creature")
        && replacement.starts_with("target creature ")
    {
        replacement = replacement.replacen("target creature", "that creature", 1);
    }
    if default_text.to_ascii_lowercase().contains("target player")
        && replacement.starts_with("target player ")
    {
        replacement = replacement.replacen("target player", "that player", 1);
    }
    if default_text
        .to_ascii_lowercase()
        .contains("target opponent")
        && replacement.starts_with("target opponent ")
    {
        replacement = replacement.replacen("target opponent", "that opponent", 1);
    }
    let default_lower = default_text.to_ascii_lowercase();
    if default_lower.contains("counter target")
        && default_lower.contains("spell unless")
        && replacement == "counter those spells"
    {
        replacement = "counter that spell".to_string();
    }
    replacement
}

fn describe_rendered_optional_zone_rewrite_self_replacement(
    default_text: &str,
    replacement_text: &str,
    condition_text: &str,
) -> Option<String> {
    let suffix = replacement_text.strip_prefix(default_text)?.trim();
    if !suffix.contains("you may put it into battlefield instead") {
        return None;
    }
    let lower_default = default_text.to_ascii_lowercase();
    if lower_default.starts_with("search your library for a basic land card") {
        return Some(format!(
            "{default_text}. If {condition_text}, you may put that card onto the battlefield instead of putting it into your hand"
        ));
    }
    if lower_default.starts_with("return up to two target creature cards from your graveyard") {
        return Some(format!(
            "{default_text}. If {condition_text}, you may put one of those cards with mana value 4 or less onto the battlefield instead of putting it into your hand"
        ));
    }
    None
}

fn describe_rendered_count_override_self_replacement(
    default_text: &str,
    replacement_text: &str,
    condition_text: &str,
) -> Option<String> {
    let default_sentences: Vec<_> = default_text.trim_end_matches('.').split(". ").collect();
    let replacement_sentences: Vec<_> =
        replacement_text.trim_end_matches('.').split(". ").collect();
    // Optional single-card selections are commonly rendered as "You may put
    // a ..." while the larger replacement count is rendered as "Put up to
    // two ...".  They are the same count override even though neither branch
    // uses the literal "Put one" / "Put two" surface handled below.
    if let [default_intro, default_choice, default_rest] = default_sentences.as_slice()
        && let [replacement_intro, replacement_choice_and_rest] = replacement_sentences.as_slice()
        && default_intro == replacement_intro
        && let Some(default_selection) = default_choice.strip_prefix("You may put ")
        && let Some((_, default_destination)) = default_selection.split_once(" from among them")
        && let Some(replacement_choice) = replacement_choice_and_rest
            .strip_prefix("Put up to ")
            .map(|_| *replacement_choice_and_rest)
        && let Some((replacement_choice, replacement_rest)) =
            replacement_choice.split_once(" and the rest ")
        && let Some((_, replacement_destination)) =
            replacement_choice.split_once(" from among them")
        && default_destination == replacement_destination
        && default_rest.strip_prefix("Put the rest ") == Some(replacement_rest)
    {
        return Some(format!(
            "{default_intro}. {default_choice}. If {condition_text}, {} instead of one. {default_rest}",
            super::normalize_common::lowercase_first(replacement_choice)
        ));
    }
    if let (
        [default_intro, default_choice, default_rest],
        [replacement_intro, replacement_choice, replacement_rest],
    ) = (
        default_sentences.as_slice(),
        replacement_sentences.as_slice(),
    ) && default_intro == replacement_intro
        && default_rest == replacement_rest
        && let Some(default_selection) = default_choice.strip_prefix("You may put ")
        && let Some((_, default_destination)) = default_selection.split_once(" from among them")
        && replacement_choice.starts_with("Put up to ")
        && let Some((_, replacement_destination)) =
            replacement_choice.split_once(" from among them")
        && default_destination == replacement_destination
    {
        return Some(format!(
            "{default_intro}. {default_choice}. If {condition_text}, {} instead of one. {default_rest}",
            super::normalize_common::lowercase_first(replacement_choice)
        ));
    }
    if let ([default_intro, default_choice], [replacement_intro, replacement_choice]) = (
        default_sentences.as_slice(),
        replacement_sentences.as_slice(),
    ) && default_intro == replacement_intro
        && default_choice.starts_with("Put one ")
        && replacement_choice.starts_with("Put two ")
        && let Some((default_choice_hand, default_rest)) =
            default_choice.split_once(" and the rest ")
        && let Some((replacement_choice_hand, replacement_rest)) =
            replacement_choice.split_once(" and the rest ")
        && default_rest == replacement_rest
    {
        let default_choice_hand = default_choice_hand.replace("of them", "of those cards");
        let replacement_choice_hand = replacement_choice_hand.replace("of them", "of those cards");
        return Some(format!(
            "{default_intro}. {default_choice_hand}. If {condition_text}, {} instead. Put the rest {default_rest}",
            super::normalize_common::lowercase_first(&replacement_choice_hand)
        ));
    }
    let [default_intro, default_choice, default_rest] = default_sentences.as_slice() else {
        return None;
    };
    let [replacement_intro, replacement_choice, replacement_rest] =
        replacement_sentences.as_slice()
    else {
        return None;
    };
    if default_intro != replacement_intro
        || default_rest != replacement_rest
        || !default_choice.starts_with("Put one ")
        || !replacement_choice.starts_with("Put two ")
    {
        return None;
    }
    Some(format!(
        "{default_intro}. {default_choice}. If {condition_text}, {} instead. {default_rest}",
        super::normalize_common::lowercase_first(replacement_choice)
    ))
}

fn describe_mill_count_override_self_replacement(
    default_effects: &[Effect],
    replacement_effects: &[Effect],
    default_text: &str,
    replacement_text: &str,
    condition_text: &str,
) -> Option<String> {
    let default_mill = single_mill_effect(default_effects)?;
    let replacement_mill = single_mill_effect(replacement_effects)?;
    if default_mill.player != replacement_mill.player {
        return None;
    }
    if !mill_count_is_twice_default(&default_mill.count, &replacement_mill.count) {
        return None;
    }

    let replacement_count = super::normalize_common::describe_value(&replacement_mill.count);
    let mut replacement = rewrite_self_replacement_referent_phrase(default_text, replacement_text);
    replacement = replacement.trim_end_matches('.').replace(
        &format!("{replacement_count} cards"),
        "twice that many cards",
    );
    Some(format!(
        "{default_text}. If {condition_text}, {replacement} instead"
    ))
}

fn mill_count_is_twice_default(default_count: &Value, replacement_count: &Value) -> bool {
    matches!(replacement_count, Value::Scaled(inner, 2) if inner.as_ref() == default_count)
        || matches!(
            (default_count, replacement_count),
            (Value::X, Value::XTimes(2))
        )
}

fn single_mill_effect(effects: &[Effect]) -> Option<&crate::effects::MillEffect> {
    let mut found = None;
    for effect in effects {
        let Some(mill) =
            unwrap_basic_render_wrapper(effect).downcast_ref::<crate::effects::MillEffect>()
        else {
            continue;
        };
        if found.is_some() {
            return None;
        }
        found = Some(mill);
    }
    found
}

fn unwrap_basic_render_wrapper(effect: &Effect) -> &Effect {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return unwrap_basic_render_wrapper(&with_id.effect);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return unwrap_basic_render_wrapper(&tag_all.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return unwrap_basic_render_wrapper(&tagged.effect);
    }
    effect
}

fn target_player_draw_discard_counts(effects: &[Effect]) -> Option<(&Value, &Value)> {
    let [target_effect, draw_effect, discard_effect] = effects else {
        return None;
    };
    let target = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if target.target != target_any_player_spec() {
        return None;
    }

    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    let discard = discard_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    if draw.player != target_any_player_filter()
        || discard.player != target_any_player_filter()
        || discard.random
        || discard.any_number
        || discard.card_filter.is_some()
    {
        return None;
    }
    Some((&draw.count, &discard.count))
}

fn card_count_noun(count: &Value) -> &'static str {
    if matches!(count.unhinted(), Value::Fixed(1)) {
        "card"
    } else {
        "cards"
    }
}

fn card_count_text(count: &Value) -> String {
    match count.unhinted() {
        Value::Fixed(n) => number_word(*n).unwrap_or_else(|| n.to_string()),
        _ => describe_value(count),
    }
}

fn describe_target_player_draw_discard_self_replacement(
    default_effects: &[Effect],
    replacement_effects: &[Effect],
    condition_text: &str,
) -> Option<String> {
    let (default_draw, default_discard) = target_player_draw_discard_counts(default_effects)?;
    let (replacement_draw, replacement_discard) =
        target_player_draw_discard_counts(replacement_effects)?;
    if default_draw != replacement_draw {
        return None;
    }

    let draw_count = card_count_text(default_draw);
    let default_discard_count = card_count_text(default_discard);
    let replacement_discard_count = card_count_text(replacement_discard);
    Some(format!(
        "Target player draws {draw_count} {}, then discards {default_discard_count} {}. If {condition_text}, instead that player draws {draw_count} {}, then discards {replacement_discard_count} {}",
        card_count_noun(default_draw),
        card_count_noun(default_discard),
        card_count_noun(replacement_draw),
        card_count_noun(replacement_discard),
    ))
}

fn describe_rendered_gets_self_replacement(
    default_text: &str,
    replacement_text: &str,
    condition_text: &str,
) -> Option<String> {
    let default_text = default_text.trim().trim_end_matches('.');
    if !default_text.contains(" gets ") {
        return None;
    }
    let replacement_text = replacement_text.trim().trim_end_matches('.');
    let replacement = replacement_text
        .strip_prefix("It gets ")
        .or_else(|| replacement_text.strip_prefix("it gets "))?;
    Some(format!(
        "{default_text}. It gets {replacement} instead if {condition_text}"
    ))
}

fn describe_tagged_target_set_destroy_self_replacement(
    default_effects: &[Effect],
    replacement_effects: &[Effect],
    default_text: &str,
    condition_text: &str,
) -> Option<String> {
    let [default] = default_effects else {
        return None;
    };
    let tagged_damage = default.downcast_ref::<crate::effects::TaggedEffect>()?;
    let damage = tagged_damage
        .effect
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    let ChooseSpec::WithCount(inner, count) = damage.target.unhinted() else {
        return None;
    };
    if count.max.is_some_and(|max| max <= 1) {
        return None;
    }
    let ChooseSpec::Target(target) = inner.unhinted() else {
        return None;
    };
    let ChooseSpec::Object(target_filter) = target.unhinted() else {
        return None;
    };
    let noun = match target_filter.card_types.as_slice() {
        [CardType::Artifact] => "artifact",
        [CardType::Battle] => "battle",
        [CardType::Creature] => "creature",
        [CardType::Enchantment] => "enchantment",
        [CardType::Land] => "land",
        [CardType::Planeswalker] => "planeswalker",
        _ => "permanent",
    };

    let [replacement] = replacement_effects else {
        return None;
    };
    let destroy =
        unwrap_basic_render_wrapper(replacement).downcast_ref::<crate::effects::DestroyEffect>()?;
    let ChooseSpec::Object(destroy_filter) = destroy.spec.base() else {
        return None;
    };
    if !destroy_filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == tagged_damage.tag
            && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    }) {
        return None;
    }

    Some(format!(
        "{default_text}. Destroy each {noun} instead if {condition_text}"
    ))
}

fn describe_rendered_damage_self_replacement(
    default_text: &str,
    replacement_text: &str,
    condition_text: &str,
) -> Option<String> {
    let (base_amount, base_target) = split_rendered_damage(default_text)?;
    let (replacement_amount, replacement_target) = split_rendered_damage(replacement_text)?;
    if base_target != replacement_target {
        return None;
    }
    let repeated_target = if base_target.starts_with("each ") || base_target.starts_with("all ") {
        format!(" to {base_target}")
    } else {
        String::new()
    };
    Some(format!(
        "Deal {base_amount} damage to {base_target}. It deals {replacement_amount} damage{repeated_target} instead if {condition_text}"
    ))
}

fn split_rendered_damage(text: &str) -> Option<(&str, &str)> {
    let text = text.trim().trim_end_matches('.');
    let rest = text.strip_prefix("Deal ")?;
    let (amount, target) = rest.split_once(" damage to ")?;
    if amount.trim().is_empty() || target.trim().is_empty() {
        return None;
    }
    Some((amount.trim(), target.trim()))
}

fn describe_resolution_program_for_card(
    def: &CardDefinition,
    program: &crate::resolution::ResolutionProgram,
) -> String {
    let has_visible_gift_line = def
        .optional_costs
        .iter()
        .any(|cost| matches!(cost.kind, crate::cost::OptionalCostKind::Gift));
    if !has_visible_gift_line {
        let rendered = describe_resolution_program(program);
        return rewrite_spell_resolution_damage_source(def, &rendered);
    }

    let mut rendered_segments = Vec::new();
    for segment in &program.segments {
        if is_hidden_gift_resolution_segment(segment) {
            continue;
        }

        if segment.self_replacements.len() == 1 {
            let rendered = describe_single_self_replacement_segment(segment).unwrap_or_else(|| {
                let branch = &segment.self_replacements[0];
                describe_effect_list(&[Effect::conditional(
                    branch.condition.clone(),
                    branch.replacement_effects.clone(),
                    segment.default_effects.clone(),
                )])
            });
            let rendered =
                apply_self_replacement_presentation_label(&segment.self_replacements[0], rendered);
            rendered_segments.push(rewrite_spell_resolution_damage_source(def, &rendered));
            continue;
        }

        if !segment.default_effects.is_empty() {
            let rendered = describe_effect_list(&segment.default_effects);
            rendered_segments.push(rewrite_spell_resolution_damage_source(def, &rendered));
        }
        for branch in &segment.self_replacements {
            let rendered = describe_effect_list(&branch.replacement_effects);
            rendered_segments.push(rewrite_spell_resolution_damage_source(def, &rendered));
        }
    }

    rendered_segments.join(". ")
}

fn rewrite_spell_resolution_damage_source(def: &CardDefinition, rendered: &str) -> String {
    if !(def.card.is_instant() || def.card.is_sorcery()) || def.card.name.contains(" // ") {
        return rendered.to_string();
    }
    let rendered = rewrite_damage_phrases_for_permanent_abilities(rendered, &def.card.name, false)
        .replace("This spell deals ", &format!("{} deals ", def.card.name))
        .replace("this spell deals ", &format!("{} deals ", def.card.name))
        .replace("This source deal ", &format!("{} deal ", def.card.name))
        .replace("this source deal ", &format!("{} deal ", def.card.name))
        .replace("Exile this source", &format!("Exile {}", def.card.name));
    let rendered = if let Some(rest) = rendered.strip_prefix("Deal ") {
        format!("{} deals {rest}", def.card.name)
    } else {
        rendered
    };
    let rendered = rendered
        .replace(" — Deal ", &format!(" — {} deals ", def.card.name))
        .replace(" — It deals ", &format!(" — {} deals ", def.card.name))
        .replace(", deal ", &format!(", {} deals ", def.card.name));
    let rendered = rewrite_standalone_spell_self_exile(&rendered, &def.card.name);
    rewrite_inline_spell_self_exile(&rendered, &def.card.name)
}

fn rewrite_standalone_spell_self_exile(rendered: &str, card_name: &str) -> String {
    let needle = "Exile this";
    let replacement = format!("Exile {card_name}");
    let mut out = String::with_capacity(rendered.len() + card_name.len());
    let mut rest = rendered;

    while let Some(idx) = rest.find(needle) {
        out.push_str(&rest[..idx]);
        let after = &rest[idx + needle.len()..];
        if matches!(
            after.chars().next(),
            None | Some('.') | Some(',') | Some(';')
        ) {
            out.push_str(&replacement);
        } else {
            out.push_str(needle);
        }
        rest = after;
    }

    out.push_str(rest);
    out
}

fn rewrite_inline_spell_self_exile(rendered: &str, card_name: &str) -> String {
    let needle = "exile this";
    let replacement = format!("you exile {card_name}");
    let mut out = String::with_capacity(rendered.len() + card_name.len() + 4);
    let mut rest = rendered;

    while let Some(idx) = rest.find(needle) {
        out.push_str(&rest[..idx]);
        let after = &rest[idx + needle.len()..];
        if out.ends_with("then ")
            && matches!(
                after.chars().next(),
                None | Some('.') | Some(',') | Some(';')
            )
        {
            out.push_str(&replacement);
        } else {
            out.push_str(needle);
        }
        rest = after;
    }

    out.push_str(rest);
    out
}

fn ability_has_begin_on_battlefield_pregame(ability: &Ability) -> bool {
    let AbilityKind::Static(static_ability) = &ability.kind else {
        return false;
    };
    matches!(
        static_ability.pregame_action_kind(),
        Some(crate::static_abilities::PregameActionKind::BeginOnBattlefield(_))
    )
}

fn substitute_pregame_self_reference(line: &str, card_name: &str) -> String {
    line.replace(
        "begin the game with this on the battlefield",
        &format!("begin the game with {card_name} on the battlefield"),
    )
}

fn substitute_pregame_card_self_reference(line: &str, subject: &str, card_name: &str) -> String {
    if subject.is_empty() || subject.eq_ignore_ascii_case("this source") {
        return line.to_string();
    }
    let lower = line.to_ascii_lowercase();
    if !lower.contains("begin the game") {
        return line.to_string();
    }
    let capitalized = capitalize_first_ascii(subject);
    line.replace(subject, card_name)
        .replace(&capitalized, card_name)
}

pub(super) fn substitute_legendary_source_reference(
    line: &str,
    card: &crate::card::Card,
    _subject: &str,
    oracle_short_name: Option<&str>,
) -> String {
    let line = collapse_duplicate_source_type_subject(line);
    let line = line.as_str();
    if card.name.contains(" // ") {
        return line.to_string();
    }

    // Prefer the name the oracle text actually used to refer to the source (a
    // captured ShortName surface) over a naive comma split, so a card whose
    // oracle shortens itself ("Loran of the Third Path" -> "Loran") renders that
    // way, while one that doesn't ("Kodama of the Center Tree") stays full.
    let comma_short = card.name.split(',').next().unwrap_or(&card.name).trim();
    let self_name = oracle_short_name.map(str::trim).unwrap_or(comma_short);

    let lower = line.to_ascii_lowercase();
    let conditional_static_self_surface = ((lower.starts_with("as long as ")
        || lower.contains(": as long as "))
        && (lower.contains(", this creature has ")
            || lower.contains(" this creature has ")
            || lower.contains(", this creature is ")
            || lower.contains(" this creature is ")))
        || (lower.starts_with("this creature has ") && lower.contains(" as long as "));
    let uses_named_source_surface = lower.starts_with("this creature gets ")
        || lower.starts_with("this creature's power and toughness ")
        || lower.starts_with("this creature gains ")
        || lower.starts_with("as this enters")
        || conditional_static_self_surface
        || lower.contains("if this land has ")
        || lower.contains("if this creature has one or more ")
        || lower.contains(" counters on this artifact")
        || lower.starts_with("whenever this creature enters or attacks")
        || lower.starts_with("whenever this creature attacks")
        || lower.starts_with("whenever this creature deals combat damage to a player")
        || lower.starts_with("whenever this creature or another ")
        || lower.starts_with("whenever this or another ")
        || lower.starts_with("this creature can't be the target ")
        || lower.contains(" this creature deals ")
        || lower.contains(", this creature deals ")
        || lower.contains(": target creature blocks this creature")
        || lower.contains(": exile target creature blocking or blocked by this creature")
        || (lower.contains(": put ") && lower.contains(" exiled with this creature"))
        || lower.contains(": this creature gets ")
        || lower.contains(": this creature gains ")
        || lower.contains(": this creature deals ")
        || lower.contains(": whenever this creature deals combat damage to a player")
        || lower.starts_with("this planeswalker has ")
        || lower.starts_with("this planeswalker deals ")
        || lower.contains(": this planeswalker deals ");
    if card.supertypes.contains(&Supertype::Legendary) && lower.starts_with("soulshift ") {
        if !self_name.is_empty() {
            return format!("{self_name} has {}", lowercase_first(line));
        }
    }
    if !card.supertypes.contains(&Supertype::Legendary) || !uses_named_source_surface {
        return line.to_string();
    }

    let source_name = self_name;
    if source_name.is_empty() {
        return line.to_string();
    }

    let line = line
        .strip_prefix("Whenever this or another ")
        .map(|rest| format!("Whenever {source_name} or another {rest}"))
        .unwrap_or_else(|| line.to_string());

    let line = if let Some(rest) = line.strip_prefix("As this enters") {
        format!("As {source_name} enters{rest}")
    } else if let Some(rest) = line.strip_prefix("as this enters") {
        format!("As {source_name} enters{rest}")
    } else {
        line
    };

    let substituted = [
        ("This creature", source_name),
        ("this creature", source_name),
        ("This land", source_name),
        ("this land", source_name),
        ("This artifact", source_name),
        ("this artifact", source_name),
        ("This planeswalker", source_name),
        ("this planeswalker", source_name),
    ]
    .into_iter()
    .fold(line, |line, (from, to)| {
        replace_outside_quotes(&line, from, to)
    });
    let lower_source_name = source_name.to_ascii_lowercase();
    if lower_source_name != source_name {
        replace_outside_quotes(&substituted, &lower_source_name, source_name)
    } else {
        substituted
    }
}

fn collapse_duplicate_source_type_subject(line: &str) -> String {
    let mut collapsed = line.to_string();
    for noun in [
        "artifact",
        "battle",
        "card",
        "creature",
        "enchantment",
        "land",
        "permanent",
        "planeswalker",
        "source",
        "spell",
    ] {
        let upper_from = format!("This {noun} {noun} ");
        let upper_to = format!("This {noun} ");
        collapsed = collapsed.replace(&upper_from, &upper_to);

        let lower_from = format!("this {noun} {noun} ");
        let lower_to = format!("this {noun} ");
        collapsed = collapsed.replace(&lower_from, &lower_to);
    }
    collapsed
}

/// The shortened self-name the card's oracle text uses for itself, if any —
/// captured as a `ShortName` source-reference surface on one of its triggers
/// (e.g. "When Loran enters" on "Loran of the Third Path"). Returns `None` when
/// the oracle only ever names the card in full ("Kodama of the Center Tree"),
/// so render-side substitution stays faithful to the printed wording.
pub(super) fn oracle_short_self_name(def: &CardDefinition) -> Option<String> {
    for ability in &def.abilities {
        let crate::ability::AbilityKind::Triggered(triggered) = &ability.kind else {
            continue;
        };
        if let Some(zone_change) = triggered
            .trigger
            .downcast_ref::<crate::triggers::zone_changes::ZoneChangeTrigger>()
            && let Some(crate::target::SourceReferenceSurface::ShortName(name)) =
                &zone_change.this_object_surface
        {
            return Some(name.clone());
        }
    }
    None
}

fn replace_outside_quotes(input: &str, from: &str, to: &str) -> String {
    if from.is_empty() {
        return input.to_string();
    }

    let mut output = String::with_capacity(input.len());
    let mut in_quote = false;
    let mut index = 0;
    while index < input.len() {
        let ch = input[index..]
            .chars()
            .next()
            .expect("index should be on a char boundary");
        if ch == '"' {
            in_quote = !in_quote;
            output.push(ch);
            index += ch.len_utf8();
        } else if !in_quote && input[index..].starts_with(from) {
            output.push_str(to);
            index += from.len();
        } else {
            output.push(ch);
            index += ch.len_utf8();
        }
    }
    output
}

fn capitalize_first_ascii(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => {
            let mut out = c.to_ascii_uppercase().to_string();
            out.push_str(chars.as_str());
            out
        }
        None => String::new(),
    }
}

fn title_case_card_name_surface(name: &str) -> String {
    let small_words = [
        "a", "an", "and", "as", "at", "but", "by", "for", "from", "in", "of", "or", "the", "to",
        "with",
    ];
    name.split_whitespace()
        .enumerate()
        .map(|(idx, word)| {
            if idx > 0 && small_words.contains(&word) {
                word.to_string()
            } else {
                capitalize_first_ascii(word)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn target_any_player_spec() -> ChooseSpec {
    ChooseSpec::Target(Box::new(ChooseSpec::Player(PlayerFilter::Any)))
}

fn target_any_player_filter() -> PlayerFilter {
    PlayerFilter::Target(Box::new(PlayerFilter::Any))
}

fn describe_structural_partner_with_pair(first: &Ability, second: &Ability) -> Option<String> {
    let AbilityKind::Static(static_ability) = &first.kind else {
        return None;
    };
    if static_ability.id() != crate::static_abilities::StaticAbilityId::PartnerWith {
        return None;
    }

    let AbilityKind::Triggered(triggered) = &second.kind else {
        return None;
    };
    if triggered.choices.as_slice() != [target_any_player_spec()].as_slice() {
        return None;
    }

    let zone_change = triggered
        .trigger
        .downcast_ref::<crate::triggers::ZoneChangeTrigger>()?;
    if zone_change.from != crate::triggers::ZonePattern::Any
        || zone_change.to != crate::triggers::ZonePattern::Specific(Zone::Battlefield)
        || zone_change.player != crate::triggers::PlayerRelation::Any
        || zone_change.count_mode != crate::triggers::CountMode::Each
        || !zone_change.this_object
        || zone_change.object_filter != ObjectFilter::default()
    {
        return None;
    }

    fn partner_name_from_search_library(
        search: &crate::effects::SearchLibraryEffect,
    ) -> Option<&str> {
        let partner_name = search.filter.name.as_deref()?;
        if search.filter.zone != Some(Zone::Library)
            || search.filter.owner != Some(target_any_player_filter())
            || search.destination != Zone::Hand
            || search.chooser != target_any_player_filter()
            || search.player != target_any_player_filter()
            || !search.reveal
            || search.search_mode != crate::effect::SearchSelectionMode::Exact
            || search.library_position_from_top.is_some()
        {
            return None;
        }
        Some(partner_name)
    }

    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let (outer_target_effect, may_effect) = match segment.default_effects.as_slice() {
        [may_effect] => (None, may_effect),
        [target_effect, may_effect] => (Some(target_effect), may_effect),
        _ => return None,
    };
    if let Some(target_effect) = outer_target_effect {
        let target_only = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
        if target_only.target != target_any_player_spec() {
            return None;
        }
    }
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider != Some(target_any_player_filter()) {
        return None;
    }
    let [target_effect, payload_effect] = may.effects.as_slice() else {
        return None;
    };
    let target_only = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if target_only.target != target_any_player_spec() {
        return None;
    }

    if let Some(search) = payload_effect.downcast_ref::<crate::effects::SearchLibraryEffect>() {
        let partner_name = partner_name_from_search_library(search)?;
        return Some(format!(
            "Partner with {}",
            title_case_card_name_surface(partner_name)
        ));
    }

    let sequence = payload_effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    let [choose_effect, for_each_effect, shuffle_effect] = sequence.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let partner_name = choose.filter.name.as_deref()?;
    if choose.filter.zone != Some(Zone::Library)
        || choose.filter.owner != Some(target_any_player_filter())
        || choose.count != ChoiceCount::exactly(1)
        || choose.count_value.is_some()
        || choose.chooser != target_any_player_filter()
        || choose.zone != Some(Zone::Library)
        || !choose.additional_zones.is_empty()
        || !choose.is_search
        || !choose.reveal
        || choose.search_mode != crate::effect::SearchSelectionMode::Exact
        || choose.top_only
        || choose.replace_tagged_objects
    {
        return None;
    }

    let for_each = for_each_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if for_each.tag != choose.tag {
        return None;
    }
    let [move_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let move_to_zone = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.target != ChooseSpec::Iterated
        || move_to_zone.zone != Zone::Hand
        || move_to_zone.to_top
        || move_to_zone.battlefield_controller != crate::effects::BattlefieldController::Preserve
        || move_to_zone.enters_tapped
    {
        return None;
    }

    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if shuffle.player != target_any_player_filter()
        || shuffle.target_spec != Some(target_any_player_spec())
    {
        return None;
    }

    Some(format!(
        "Partner with {}",
        title_case_card_name_surface(partner_name)
    ))
}

fn describe_structural_echo_pair(first: &Ability, second: &Ability) -> Option<String> {
    let AbilityKind::Static(static_ability) = &first.kind else {
        return None;
    };
    let static_display = static_ability
        .display()
        .trim()
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if static_ability.id() != crate::static_abilities::StaticAbilityId::EnterWithCounters
        || !static_display.contains("echo")
        || !static_display.contains("counter")
    {
        return None;
    }
    let AbilityKind::Triggered(triggered) = &second.kind else {
        return None;
    };
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || triggered
            .trigger
            .downcast_ref::<crate::triggers::BeginningOfUpkeepTrigger>()
            .is_none_or(|trigger| trigger.player != PlayerFilter::You)
    {
        return None;
    }
    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let echo_effects = match segment.default_effects.as_slice() {
        [conditional_effect] => {
            let conditional =
                conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
            if conditional.condition != crate::effect::Condition::SourceIsInZone(Zone::Battlefield)
                || !conditional.if_false.is_empty()
            {
                return None;
            }
            conditional.if_true.as_slice()
        }
        effects => effects,
    };
    let [remove_effect, if_effect] = echo_effects else {
        return None;
    };
    let Some(remove_with_id) = remove_effect.downcast_ref::<crate::effects::WithIdEffect>() else {
        return None;
    };
    let remove = remove_with_id
        .effect
        .downcast_ref::<crate::effects::RemoveCountersEffect>()?;
    if remove.counter_type != CounterType::Echo
        || remove.count != Value::Fixed(1)
        || !matches!(remove.target, ChooseSpec::Source)
    {
        return None;
    }

    let if_effect = if_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if if_effect.condition != remove_with_id.id
        || if_effect.predicate != crate::effect::EffectPredicate::Happened
        || !if_effect.else_.is_empty()
    {
        return None;
    }
    let [unless] = if_effect.then.as_slice() else {
        return None;
    };
    let unless = unless.downcast_ref::<crate::effects::UnlessActionEffect>()?;
    if unless.player != PlayerFilter::You {
        return None;
    }
    let [sacrifice] = unless.effects.as_slice() else {
        return None;
    };
    let sacrifice = sacrifice.downcast_ref::<crate::effects::SacrificeTargetEffect>()?;
    if !matches!(sacrifice.target, ChooseSpec::Source) {
        return None;
    }
    let cost = describe_echo_alternative_cost(&unless.alternative)?;
    Some(format!("Echo{cost}"))
}

fn describe_echo_alternative_cost(alternative: &[Effect]) -> Option<String> {
    if let [pay] = alternative {
        if let Some(pay) = pay.downcast_ref::<crate::effects::PayManaEffect>() {
            if !matches!(pay.player, ChooseSpec::SourceController) {
                return None;
            }
            return Some(format!(" {}", pay.cost.to_oracle()));
        }

        if let Some(discard) = pay.downcast_ref::<crate::effects::DiscardEffect>() {
            if discard.random
                || discard.player != PlayerFilter::You
                || discard.tag.is_some()
                || discard.card_filter.is_some()
            {
                return None;
            }
            let Value::Fixed(count) = discard.count else {
                return None;
            };
            if count == 1 {
                return Some("—Discard a card".to_string());
            }
            return Some(format!("—Discard {count} cards"));
        }

        if let Some(lose_life) = pay.downcast_ref::<crate::effects::LoseLifeEffect>() {
            if !matches!(lose_life.player, ChooseSpec::Player(PlayerFilter::You)) {
                return None;
            }
            return Some(format!("—Pay {} life", describe_value(&lose_life.amount)));
        }
    }

    if let [choose, sacrifice] = alternative {
        let choose = choose.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
        let sacrifice = sacrifice.downcast_ref::<crate::effects::SacrificeEffect>()?;
        let exact = choose.count.max.filter(|max| *max == choose.count.min)?;
        if exact == 0
            || choose.is_search
            || choose.chooser != PlayerFilter::You
            || choose.filter.controller != Some(PlayerFilter::You)
            || sacrifice.player != PlayerFilter::You
            || sacrifice.count != Value::Fixed(exact as i32)
            || !sacrifice
                .filter
                .tagged_constraints
                .iter()
                .any(|constraint| {
                    constraint.tag == choose.tag
                        && matches!(
                            constraint.relation,
                            crate::filter::TaggedOpbjectRelation::IsTaggedObject
                        )
                })
        {
            return None;
        }
        let description = choose.filter.description();
        let noun = strip_leading_article(&description);
        if exact == 1 {
            return Some(format!("—Sacrifice {}", with_indefinite_article(noun)));
        }
        let count = number_word(exact as i32).unwrap_or_else(|| exact.to_string());
        return Some(format!(
            "—Sacrifice {count} {}",
            pluralize_noun_phrase(noun)
        ));
    }

    None
}

fn describe_structural_equipment_token_keyword(ability: &Ability) -> Option<String> {
    let AbilityKind::Triggered(triggered) = &ability.kind else {
        return None;
    };
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || !trigger_is_this_enters_battlefield(&triggered.trigger)
    {
        return None;
    }
    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let [create_effect, attach_effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let (tag, create) = tagged_create_token_effect_for_keyword(create_effect)?;
    if create.count != Value::Fixed(1)
        || create.controller != PlayerFilter::You
        || create.controller_target.is_some()
        || create.suppress_aura_attachment_choice
        || create.enters_tapped
        || create.enters_attacking
        || create.exile_at_end_of_combat
        || create.sacrifice_at_end_of_combat
        || create.sacrifice_at_next_end_step
        || create.exile_at_next_end_step
    {
        return None;
    }
    let attach = unwrap_effect_for_keyword(attach_effect)
        .downcast_ref::<crate::effects::AttachToEffect>()?;
    if !matches!(&attach.target, ChooseSpec::Tagged(found) if found == tag) {
        return None;
    }
    if is_living_weapon_germ_token(&create.token) {
        return Some("Living weapon".to_string());
    }
    if is_for_mirrodin_rebel_token(&create.token) {
        return Some("For Mirrodin!".to_string());
    }
    None
}

fn unwrap_effect_for_keyword(effect: &Effect) -> &Effect {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return unwrap_effect_for_keyword(&with_id.effect);
    }
    effect
}

fn tagged_create_token_effect_for_keyword(
    effect: &Effect,
) -> Option<(&TagKey, &crate::effects::CreateTokenEffect)> {
    let tagged =
        unwrap_effect_for_keyword(effect).downcast_ref::<crate::effects::TaggedEffect>()?;
    let create = unwrap_effect_for_keyword(&tagged.effect)
        .downcast_ref::<crate::effects::CreateTokenEffect>()?;
    Some((&tagged.tag, create))
}

fn is_living_weapon_germ_token(token: &CardDefinition) -> bool {
    token.card.is_token
        && token.card.name == "Phyrexian Germ"
        && token.card.colors() == crate::color::ColorSet::BLACK
        && token.card.card_types == [CardType::Creature]
        && token.card.subtypes == [Subtype::Phyrexian, Subtype::Germ]
        && matches!(
            token.card.power_toughness,
            Some(crate::card::PowerToughness {
                power: crate::card::PtValue::Fixed(0),
                toughness: crate::card::PtValue::Fixed(0),
            })
        )
        && token.abilities.is_empty()
}

fn is_for_mirrodin_rebel_token(token: &CardDefinition) -> bool {
    token.card.is_token
        && token.card.name == "Rebel"
        && token.card.colors() == crate::color::ColorSet::RED
        && token.card.card_types == [CardType::Creature]
        && token.card.subtypes == [Subtype::Rebel]
        && matches!(
            token.card.power_toughness,
            Some(crate::card::PowerToughness {
                power: crate::card::PtValue::Fixed(2),
                toughness: crate::card::PtValue::Fixed(2),
            })
        )
        && token.abilities.is_empty()
}

fn trigger_is_this_enters_battlefield(trigger: &crate::triggers::Trigger) -> bool {
    trigger
        .downcast_ref::<crate::triggers::ZoneChangeTrigger>()
        .is_some_and(|zone_change| {
            zone_change.this_object
                && zone_change.to.matches(Zone::Battlefield)
                && zone_change.cause_filter.is_none()
        })
}

fn describe_structural_evolve_keyword(ability: &Ability) -> Option<String> {
    let AbilityKind::Triggered(triggered) = &ability.kind else {
        return None;
    };
    if triggered.intervening_if.is_some() || !triggered.choices.is_empty() {
        return None;
    }
    let zone_change = triggered
        .trigger
        .downcast_ref::<crate::triggers::ZoneChangeTrigger>()?;
    if zone_change.this_object
        || !zone_change.to.matches(Zone::Battlefield)
        || zone_change.object_filter.controller != Some(PlayerFilter::You)
        || !zone_change
            .object_filter
            .card_types
            .contains(&CardType::Creature)
    {
        return None;
    }
    let [effect] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    effect
        .downcast_ref::<crate::effects::EvolveEffect>()
        .is_some()
        .then(|| "Evolve".to_string())
}

fn describe_structural_training_keyword(ability: &Ability) -> Option<String> {
    let AbilityKind::Triggered(triggered) = &ability.kind else {
        return None;
    };
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || triggered
            .trigger
            .downcast_ref::<crate::triggers::ThisAttacksWithGreaterPowerTrigger>()
            .is_none()
    {
        return None;
    }
    let [put, emit] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let put = put.downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.counter_type != CounterType::PlusOnePlusOne
        || put.amount != Value::Fixed(1)
        || !matches!(put.target, ChooseSpec::Source)
        || put.target_count.is_some()
        || put.distributed
    {
        return None;
    }
    let emit = emit.downcast_ref::<crate::effects::EmitKeywordActionEffect>()?;
    (emit.action == ironsmith_core::KeywordActionKind::Train && emit.amount == 1)
        .then(|| "Training".to_string())
}

fn describe_structural_ingest_keyword(ability: &Ability) -> Option<String> {
    let AbilityKind::Triggered(triggered) = &ability.kind else {
        return None;
    };
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || triggered
            .trigger
            .downcast_ref::<crate::triggers::ThisDealsCombatDamageToPlayerTrigger>()
            .is_none()
    {
        return None;
    }
    let [effect] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let exile_top = effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    (exile_top.player == PlayerFilter::DamagedPlayer
        && exile_top.count == Value::Fixed(1)
        && exile_top.accumulated_tags.is_empty())
    .then(|| "Ingest".to_string())
}

fn describe_structural_rampage_pair(first: &Ability, second: &Ability) -> Option<String> {
    let AbilityKind::Static(static_ability) = &first.kind else {
        return None;
    };
    if static_ability.id() != crate::static_abilities::StaticAbilityId::KeywordMarker {
        return None;
    }
    let display = static_ability.display();
    let lower = display.trim().trim_end_matches('.').to_ascii_lowercase();
    let amount: i32 = lower.strip_prefix("rampage ")?.parse().ok()?;
    if amount <= 0 {
        return None;
    }

    let AbilityKind::Triggered(triggered) = &second.kind else {
        return None;
    };
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || triggered
            .trigger
            .downcast_ref::<crate::triggers::ThisBecomesBlockedTrigger>()
            .is_none()
    {
        return None;
    }
    let [effect] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let pump = effect.downcast_ref::<crate::effects::ModifyPowerToughnessEffect>()?;
    if !matches!(pump.target, ChooseSpec::Source)
        || !matches!(pump.duration, Until::EndOfTurn)
        || !matches!(pump.power, Value::EventValue(EventValueSpec::BlockersBeyondFirst { multiplier }) if multiplier == amount)
        || !matches!(pump.toughness, Value::EventValue(EventValueSpec::BlockersBeyondFirst { multiplier }) if multiplier == amount)
    {
        return None;
    }
    Some(format!("Rampage {amount}"))
}

fn describe_structural_graveyard_or_exile_cast_pair(
    first: &Ability,
    second: &Ability,
) -> Option<String> {
    fn play_from_source_zone(ability: &Ability) -> Option<Zone> {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            return None;
        };
        let spec = static_ability.grant_spec()?;
        (matches!(spec.grantable, crate::grant::Grantable::PlayFrom)
            && spec.filter == crate::target::ObjectFilter::source())
        .then_some(spec.zone)
    }

    match (play_from_source_zone(first), play_from_source_zone(second)) {
        (Some(Zone::Graveyard), Some(Zone::Exile)) | (Some(Zone::Exile), Some(Zone::Graveyard)) => {
            Some("You may cast this card from your graveyard or from exile".to_string())
        }
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CounterKeywordAmount {
    Fixed(u32),
    Sunburst,
    X,
}

impl CounterKeywordAmount {
    fn render(self) -> String {
        match self {
            CounterKeywordAmount::Fixed(amount) => amount.to_string(),
            CounterKeywordAmount::Sunburst => "Sunburst".to_string(),
            CounterKeywordAmount::X => "X".to_string(),
        }
    }
}

fn static_enter_counter_amount(ability: &Ability) -> Option<(CounterType, CounterKeywordAmount)> {
    let AbilityKind::Static(static_ability) = &ability.kind else {
        return None;
    };
    if static_ability.id() != crate::static_abilities::StaticAbilityId::EnterWithCounters {
        return None;
    }

    let display = static_ability
        .display()
        .trim()
        .trim_end_matches('.')
        .to_ascii_lowercase();
    let counter_type = if display.contains("+1/+1 counter") {
        CounterType::PlusOnePlusOne
    } else if display.contains("fade counter") {
        CounterType::Fade
    } else if display.contains("time counter") {
        CounterType::Time
    } else {
        return None;
    };

    if display.contains("for each color of mana spent to cast") {
        return Some((counter_type, CounterKeywordAmount::Sunburst));
    }
    if display.contains("with x ") {
        return Some((counter_type, CounterKeywordAmount::X));
    }

    let after_enters = display
        .strip_prefix("enters with ")
        .or_else(|| display.strip_prefix("enters the battlefield with "))?;
    let amount = parse_leading_counter_count(after_enters)?;
    Some((counter_type, CounterKeywordAmount::Fixed(amount)))
}

fn parse_leading_counter_count(text: &str) -> Option<u32> {
    let first = text.split_whitespace().next()?;
    ironsmith_core::parse_cardinal_word(first)
}

fn is_upkeep_remove_counter(
    triggered: &crate::ability::TriggeredAbility,
    counter: CounterType,
) -> bool {
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || triggered
            .trigger
            .downcast_ref::<crate::triggers::BeginningOfUpkeepTrigger>()
            .is_none_or(|trigger| trigger.player != PlayerFilter::You)
    {
        return false;
    }
    let [segment] = triggered.effects.segments.as_slice() else {
        return false;
    };
    if !segment.self_replacements.is_empty() {
        return false;
    }
    let [effect] = segment.default_effects.as_slice() else {
        return false;
    };
    let Some(remove) = effect.downcast_ref::<crate::effects::RemoveCountersEffect>() else {
        return false;
    };
    remove.counter_type == counter
        && remove.count == Value::Fixed(1)
        && matches!(remove.target, ChooseSpec::Source)
}

fn sacrifices_source(triggered: &crate::ability::TriggeredAbility) -> bool {
    let [segment] = triggered.effects.segments.as_slice() else {
        return false;
    };
    if !segment.self_replacements.is_empty() {
        return false;
    }
    let [effect] = segment.default_effects.as_slice() else {
        return false;
    };
    let Some(sacrifice) = effect.downcast_ref::<crate::effects::SacrificeTargetEffect>() else {
        return false;
    };
    matches!(sacrifice.target, ChooseSpec::Source)
}

fn is_fading_sacrifice_trigger(
    triggered: &crate::ability::TriggeredAbility,
    counter: CounterType,
) -> bool {
    if !matches!(triggered.intervening_if, Some(Condition::SourceHasNoCounter(found)) if found == counter)
        || !triggered.choices.is_empty()
        || !sacrifices_source(triggered)
    {
        return false;
    }
    triggered
        .trigger
        .downcast_ref::<crate::triggers::CounterRemovedFromTrigger>()
        .is_some_and(|trigger| trigger.filter.source)
}

fn is_vanishing_sacrifice_trigger(triggered: &crate::ability::TriggeredAbility) -> bool {
    if triggered.intervening_if.is_some()
        || !triggered.choices.is_empty()
        || !sacrifices_source(triggered)
    {
        return false;
    }
    triggered
        .trigger
        .downcast_ref::<crate::triggers::CustomTrigger>()
        .is_some_and(|trigger| {
            trigger.id == "vanishing-last-time-counter-removed"
                || trigger
                    .description
                    .eq_ignore_ascii_case("when the last time counter is removed")
        })
}

fn is_graft_trigger(triggered: &crate::ability::TriggeredAbility) -> bool {
    if triggered.intervening_if.is_some() || !triggered.choices.is_empty() {
        return false;
    }
    let Some(zone_change) = triggered
        .trigger
        .downcast_ref::<crate::triggers::zone_changes::ZoneChangeTrigger>()
    else {
        return false;
    };
    if zone_change.from != crate::triggers::zone_changes::ZonePattern::Any
        || zone_change.to != crate::triggers::zone_changes::ZonePattern::Specific(Zone::Battlefield)
        || zone_change.player != crate::triggers::zone_changes::PlayerRelation::Any
        || zone_change.count_mode != crate::triggers::zone_changes::CountMode::Each
        || !zone_change.object_filter.other
        || !zone_change
            .object_filter
            .card_types
            .contains(&CardType::Creature)
    {
        return false;
    }

    let [segment] = triggered.effects.segments.as_slice() else {
        return false;
    };
    if !segment.self_replacements.is_empty() {
        return false;
    }
    let [tag_effect, may_effect] = segment.default_effects.as_slice() else {
        return false;
    };
    let Some(tag) = tag_effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>() else {
        return false;
    };
    let Some(may) = may_effect.downcast_ref::<crate::effects::MayEffect>() else {
        return false;
    };
    let [move_effect] = may.effects.as_slice() else {
        return false;
    };
    let Some(move_counters) = move_effect.downcast_ref::<crate::effects::MoveCountersEffect>()
    else {
        return false;
    };
    move_counters.counter_type == CounterType::PlusOnePlusOne
        && move_counters.count == Value::Fixed(1)
        && matches!(move_counters.from, ChooseSpec::Source)
        && matches!(&move_counters.to, ChooseSpec::Tagged(found) if found == &tag.tag)
}

fn is_modular_trigger(triggered: &crate::ability::TriggeredAbility) -> bool {
    if triggered.intervening_if.is_some() || triggered.choices.is_empty() {
        return false;
    }
    let Some(zone_change) = triggered
        .trigger
        .downcast_ref::<crate::triggers::zone_changes::ZoneChangeTrigger>()
    else {
        return false;
    };
    if zone_change.from != crate::triggers::zone_changes::ZonePattern::Specific(Zone::Battlefield)
        || zone_change.to != crate::triggers::zone_changes::ZonePattern::Specific(Zone::Graveyard)
        || !zone_change.this_object
    {
        return false;
    }

    let [segment] = triggered.effects.segments.as_slice() else {
        return false;
    };
    if !segment.self_replacements.is_empty() {
        return false;
    }
    let [tag_effect, may_effect] = segment.default_effects.as_slice() else {
        return false;
    };
    if tag_effect
        .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
        .is_none()
    {
        return false;
    }
    let Some(may) = may_effect.downcast_ref::<crate::effects::MayEffect>() else {
        return false;
    };
    let [put_effect] = may.effects.as_slice() else {
        return false;
    };
    let Some(put) = put_effect.downcast_ref::<crate::effects::PutCountersEffect>() else {
        return false;
    };
    put.counter_type == CounterType::PlusOnePlusOne
        && matches!(
            &put.amount,
            Value::CountersOn(_, Some(CounterType::PlusOnePlusOne))
        )
}

fn is_ravenous_draw_trigger(triggered: &crate::ability::TriggeredAbility) -> bool {
    if !matches!(triggered.intervening_if, Some(Condition::XValueAtLeast(5)))
        || !triggered.choices.is_empty()
        || !trigger_is_this_enters_battlefield(&triggered.trigger)
    {
        return false;
    }

    let [effect] = triggered.effects.flattened_default_effects() else {
        return false;
    };
    let Some(draw) = effect.downcast_ref::<crate::effects::DrawCardsEffect>() else {
        return false;
    };
    draw.player == PlayerFilter::You && draw.count == Value::Fixed(1)
}

fn is_suspend_helper_ability(ability: &Ability) -> bool {
    let AbilityKind::Triggered(triggered) = &ability.kind else {
        return false;
    };
    ability.functional_zones == [Zone::Exile]
        && (is_suspend_remove_time_counter_trigger(triggered)
            || is_suspend_cast_when_last_counter_removed_trigger(triggered))
}

fn is_conspire_helper_ability(ability: &Ability) -> bool {
    let AbilityKind::Triggered(triggered) = &ability.kind else {
        return false;
    };
    if ability.functional_zones != [Zone::Stack]
        || !matches!(
            triggered.intervening_if.as_ref(),
            Some(Condition::ThisSpellPaidLabel(label))
                if label.display_label().eq_ignore_ascii_case("Conspire")
        )
        || !triggered.choices.is_empty()
        || triggered
            .trigger
            .downcast_ref::<crate::triggers::YouCastThisSpellTrigger>()
            .is_none()
    {
        return false;
    }

    let [copy_effect, choose_targets] = triggered.effects.flattened_default_effects() else {
        return false;
    };
    let Some(with_id) = copy_effect.downcast_ref::<crate::effects::WithIdEffect>() else {
        return false;
    };
    let Some(copy) = with_id
        .effect
        .downcast_ref::<crate::effects::CopySpellEffect>()
    else {
        return false;
    };
    let Some(retarget) = choose_targets.downcast_ref::<crate::effects::ChooseNewTargetsEffect>()
    else {
        return false;
    };
    matches!(copy.target.unhinted(), ChooseSpec::Source)
        && copy.count == Value::Fixed(1)
        && copy.copier == PlayerFilter::You
        && retarget.from_effect == with_id.id
        && retarget.may
}

fn is_squad_helper_ability(ability: &Ability) -> bool {
    let AbilityKind::Triggered(triggered) = &ability.kind else {
        return false;
    };
    if !triggered.choices.is_empty()
        || triggered.intervening_if.is_some()
        || !trigger_is_this_enters_battlefield(&triggered.trigger)
    {
        return false;
    }
    let [effect] = triggered.effects.flattened_default_effects() else {
        return false;
    };
    let Some(copy) = effect.downcast_ref::<crate::effects::CreateTokenCopyEffect>() else {
        return false;
    };
    matches!(copy.target.unhinted(), ChooseSpec::Source)
        && copy.controller == PlayerFilter::You
        && matches!(
            &copy.count,
            Value::TimesPaidLabel(label)
                if matches!(label.kind, crate::cost::OptionalCostKind::Squad)
        )
}

fn is_suspend_remove_time_counter_trigger(triggered: &crate::ability::TriggeredAbility) -> bool {
    if !matches!(
        triggered.intervening_if,
        Some(Condition::SourceHasCounterAtLeast {
            counter_type: CounterType::Time,
            count: 1,
            ..
        })
    ) || !triggered.choices.is_empty()
        || triggered
            .trigger
            .downcast_ref::<crate::triggers::BeginningOfUpkeepTrigger>()
            .is_none_or(|trigger| trigger.player != PlayerFilter::You)
    {
        return false;
    }

    let [effect] = triggered.effects.flattened_default_effects() else {
        return false;
    };
    let Some(remove) = effect.downcast_ref::<crate::effects::RemoveCountersEffect>() else {
        return false;
    };
    remove.counter_type == CounterType::Time
        && remove.count == Value::Fixed(1)
        && matches!(remove.target, ChooseSpec::Source)
}

fn is_suspend_cast_when_last_counter_removed_trigger(
    triggered: &crate::ability::TriggeredAbility,
) -> bool {
    if !matches!(
        triggered.intervening_if,
        Some(Condition::SourceHasNoCounter(CounterType::Time))
    ) || !triggered.choices.is_empty()
        || triggered
            .trigger
            .downcast_ref::<crate::triggers::CounterRemovedFromTrigger>()
            .is_none_or(|trigger| !trigger.filter.source)
    {
        return false;
    }

    let [effect] = triggered.effects.flattened_default_effects() else {
        return false;
    };
    let Some(may) = effect.downcast_ref::<crate::effects::MayEffect>() else {
        return false;
    };
    let [cast] = may.effects.as_slice() else {
        return false;
    };
    let Some(cast) = cast.downcast_ref::<crate::effects::CastSourceEffect>() else {
        return false;
    };
    cast.without_paying_mana_cost && cast.require_exile
}

fn describe_structural_counter_keyword_bundle(abilities: &[Ability]) -> Option<(String, usize)> {
    if let (
        Some(Ability {
            kind: AbilityKind::Triggered(remove_triggered),
            ..
        }),
        Some(Ability {
            kind: AbilityKind::Triggered(sacrifice_triggered),
            ..
        }),
    ) = (abilities.first(), abilities.get(1))
        && is_upkeep_remove_counter(remove_triggered, CounterType::Time)
        && is_vanishing_sacrifice_trigger(sacrifice_triggered)
    {
        return Some(("Vanishing".to_string(), 2));
    }

    let (counter, amount) = static_enter_counter_amount(abilities.first()?)?;

    if counter == CounterType::PlusOnePlusOne
        && amount == CounterKeywordAmount::X
        && let Some(Ability {
            kind: AbilityKind::Triggered(triggered),
            ..
        }) = abilities.get(1)
        && is_ravenous_draw_trigger(triggered)
    {
        return Some(("Ravenous".to_string(), 2));
    }

    if counter == CounterType::PlusOnePlusOne
        && let Some(Ability {
            kind: AbilityKind::Triggered(triggered),
            ..
        }) = abilities.get(1)
    {
        if is_modular_trigger(triggered) {
            return Some((format!("Modular {}", amount.render()), 2));
        }
        if matches!(amount, CounterKeywordAmount::Fixed(_)) && is_graft_trigger(triggered) {
            return Some((format!("Graft {}", amount.render()), 2));
        }
    }

    if (counter == CounterType::Fade || counter == CounterType::Time)
        && let (
            Some(Ability {
                kind: AbilityKind::Triggered(remove_triggered),
                ..
            }),
            Some(Ability {
                kind: AbilityKind::Triggered(sacrifice_triggered),
                ..
            }),
        ) = (abilities.get(1), abilities.get(2))
        && is_upkeep_remove_counter(remove_triggered, counter)
    {
        if counter == CounterType::Fade
            && is_fading_sacrifice_trigger(sacrifice_triggered, CounterType::Fade)
        {
            return Some((format!("Fading {}", amount.render()), 3));
        }
        if counter == CounterType::Time && is_vanishing_sacrifice_trigger(sacrifice_triggered) {
            return Some((format!("Vanishing {}", amount.render()), 3));
        }
    }

    None
}

fn is_ascend_condition(condition: &Condition) -> bool {
    let Condition::And(left, right) = condition else {
        return false;
    };

    fn controls_ten_permanents(condition: &Condition) -> bool {
        matches!(
            condition,
            Condition::PlayerHasAtLeast {
                player: PlayerFilter::You,
                filter,
                count: 10,
            } if filter.controller == Some(PlayerFilter::You)
                && filter.card_types.is_empty()
                && filter.subtypes.is_empty()
                && filter.zone == Some(Zone::Battlefield)
        )
    }

    fn lacks_citys_blessing(condition: &Condition) -> bool {
        matches!(
            condition,
            Condition::Not(inner)
                if matches!(
                    inner.as_ref(),
                    Condition::PlayerHasCitysBlessing {
                        player: PlayerFilter::You
                    }
                )
        )
    }

    (controls_ten_permanents(left) && lacks_citys_blessing(right))
        || (controls_ten_permanents(right) && lacks_citys_blessing(left))
}

fn describe_structural_ascend_ability(ability: &Ability) -> Option<String> {
    let AbilityKind::Triggered(triggered) = &ability.kind else {
        return None;
    };
    if !triggered.choices.is_empty()
        || !triggered
            .intervening_if
            .as_ref()
            .is_some_and(is_ascend_condition)
    {
        return None;
    }

    let zone_change = triggered
        .trigger
        .downcast_ref::<crate::triggers::zone_changes::ZoneChangeTrigger>()?;
    if zone_change.from != crate::triggers::zone_changes::ZonePattern::Any
        || zone_change.to != crate::triggers::zone_changes::ZonePattern::Specific(Zone::Battlefield)
        || zone_change.player != crate::triggers::zone_changes::PlayerRelation::Any
        || zone_change.count_mode != crate::triggers::zone_changes::CountMode::Each
        || zone_change.object_filter.controller != Some(PlayerFilter::You)
    {
        return None;
    }

    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let [effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let create_emblem = effect.downcast_ref::<crate::effects::CreateEmblemEffect>()?;
    if !create_emblem
        .emblem
        .name
        .eq_ignore_ascii_case("City's Blessing")
    {
        return None;
    }

    Some("Ascend".to_string())
}

pub(super) fn describe_alternative_cast_line(
    method: &AlternativeCastingMethod,
    idx: usize,
) -> String {
    match method {
        method if method.is_composed_cost() && method.name().eq_ignore_ascii_case("Spectacle") => {
            method
                .mana_cost()
                .map(|cost| format!("Spectacle {}", cost.to_oracle()))
                .unwrap_or_else(|| "Spectacle".to_string())
        }
        method if method.is_composed_cost() && method.name().eq_ignore_ascii_case("Emerge") => {
            method
                .mana_cost()
                .map(|cost| format!("Emerge {}", cost.to_oracle()))
                .unwrap_or_else(|| "Emerge".to_string())
        }
        method if method.is_composed_cost() && method.name().eq_ignore_ascii_case("Surge") => {
            method
                .mana_cost()
                .map(|cost| format!("Surge {}", cost.to_oracle()))
                .unwrap_or_else(|| "Surge".to_string())
        }
        method if method.is_composed_cost() && method.name().eq_ignore_ascii_case("Freerunning") => {
            method
                .mana_cost()
                .map(|cost| {
                    format!(
                        "Freerunning {} (You may cast this spell for its freerunning cost if you dealt combat damage to a player this turn with an Assassin or commander.)",
                        cost.to_oracle()
                    )
                })
                .unwrap_or_else(|| "Freerunning".to_string())
        }
        method if method.is_composed_cost() && method.name().eq_ignore_ascii_case("Sneak") => {
            method
                .mana_cost()
                .map(|cost| {
                    format!(
                        "Sneak {} (You may cast this spell for {} if you also return an unblocked attacker you control to hand during the declare blockers step.)",
                        cost.to_oracle(),
                        cost.to_oracle()
                    )
                })
                .unwrap_or_else(|| "Sneak".to_string())
        }
        method if method.is_composed_cost() && method.name().eq_ignore_ascii_case("Prowl") => {
            method
                .mana_cost()
                .map(|cost| format!("Prowl {}", cost.to_oracle()))
                .unwrap_or_else(|| "Prowl".to_string())
        }
        method if method.is_composed_cost() => {
            let name = method.name();
            let mana_cost = method.mana_cost();
            let costs = method.non_mana_costs();
            let cast_condition = method.cast_condition();
            if name.eq_ignore_ascii_case("Prototype")
                && let (Some(cost), Some(power_toughness)) =
                    (mana_cost, method.prototype_power_toughness())
            {
                return format!(
                    "Prototype {} — {}/{}",
                    cost.to_oracle(),
                    power_toughness.power,
                    power_toughness.toughness
                );
            }
            // Named keyword costs keep their oracle keyword surface
            // ("Evoke {2}{U}"), like the dedicated variants above.
            if !name.is_empty()
                && !name.eq_ignore_ascii_case("Parsed alternative cost")
                && cast_condition.is_none()
                && costs.is_empty()
                && let Some(cost) = mana_cost
            {
                return format!("{name} {}", cost.to_oracle());
            }
            let mut parts = Vec::new();
            if let Some(cost) = mana_cost {
                parts.push(format!("pay {}", cost.to_oracle()));
            }
            if !costs.is_empty() {
                parts.push(describe_alternative_costs(&costs));
            }
            let clause = if parts.is_empty() {
                "cast this spell without paying its mana cost".to_string()
            } else {
                parts.join(" and ")
            };
            let mut line = if parts.is_empty() {
                "You may cast this spell without paying its mana cost".to_string()
            } else {
                format!("You may {clause} rather than pay this spell's mana cost")
            };
            if !name.is_empty() {
                line.push_str(&format!(" ({name})"));
            }
            if let Some(condition) = cast_condition
                && let Some(condition_text) =
                    crate::static_abilities::describe_this_spell_cost_condition(condition)
            {
                line = format!("If {condition_text}, {}", lowercase_first(&line));
            }
            line
        }
        AlternativeCastingMethod::Madness { cost } => render_madness_cost(cost),
        AlternativeCastingMethod::Miracle { cost } => format!("Miracle {}", cost.to_oracle()),
        AlternativeCastingMethod::FlashWithAdditionalCost {
            additional_cost, ..
        } => format!(
            "You may cast this spell as though it had flash if you pay {} more to cast it",
            additional_cost.to_oracle()
        ),
        AlternativeCastingMethod::Plot { cost } => format!("Plot {}", cost.to_oracle()),
        AlternativeCastingMethod::Warp { cost } => format!("Warp {}", cost.to_oracle()),
        AlternativeCastingMethod::Suspend { cost, time } => {
            format!("Suspend {time}—{}", cost.to_oracle())
        }
        AlternativeCastingMethod::Disturb { cost } => format!("Disturb {}", cost.to_oracle()),
        AlternativeCastingMethod::Overload { cost, .. } => {
            format!("Overload {}", cost.to_oracle())
        }
        AlternativeCastingMethod::Awaken { amount, cost, .. } => {
            format!("Awaken {amount}—{}", cost.to_oracle())
        }
        AlternativeCastingMethod::Flashback { total_cost } => {
            let costs = method.non_mana_costs();
            let mana_cost = total_cost
                .mana_cost()
                .map(|cost| cost.to_oracle())
                .unwrap_or_else(|| "{0}".to_string());
            if costs.is_empty() {
                format!("Flashback—{mana_cost}")
            } else {
                let extra = capitalize_first(&describe_alternative_costs(&costs));
                format!("Flashback—{mana_cost}, {extra}")
            }
        }
        AlternativeCastingMethod::Harmonize { total_cost } => {
            let costs = method.non_mana_costs();
            let mana_cost = total_cost
                .mana_cost()
                .map(|cost| cost.to_oracle())
                .unwrap_or_else(|| "{0}".to_string());
            if costs.is_empty() {
                format!("Harmonize {mana_cost}")
            } else {
                let extra = capitalize_first(&describe_alternative_costs(&costs));
                format!("Harmonize {mana_cost}, {extra}")
            }
        }
        AlternativeCastingMethod::Retrace { .. } => "Retrace".to_string(),
        AlternativeCastingMethod::JumpStart { .. } => "Jump-start".to_string(),
        AlternativeCastingMethod::Escape {
            cost, exile_count, ..
        } => {
            let count_text =
                small_number_word(*exile_count).unwrap_or_else(|| exile_count.to_string());
            if let Some(cost) = cost {
                format!(
                    "Escape—{}, Exile {count_text} other cards from your graveyard",
                    cost.to_oracle()
                )
            } else {
                format!("Escape—Exile {count_text} other cards from your graveyard")
            }
        }
        AlternativeCastingMethod::Dash { cost } => format!("Dash {}", cost.to_oracle()),
        AlternativeCastingMethod::Blitz { total_cost } => total_cost
            .mana_cost()
            .map(|cost| format!("Blitz {}", cost.to_oracle()))
            .unwrap_or_else(|| "Blitz".to_string()),
        AlternativeCastingMethod::Bestow { total_cost } => {
            let costs = method.non_mana_costs();
            let mana_cost = total_cost
                .mana_cost()
                .map(|cost| cost.to_oracle())
                .unwrap_or_else(|| "{0}".to_string());
            if costs.is_empty() {
                format!("Bestow {mana_cost}")
            } else {
                let extra = capitalize_first(&describe_alternative_costs(&costs));
                format!("Bestow {mana_cost}, {extra}")
            }
        }
        other => {
            if other.name().eq_ignore_ascii_case("Parsed alternative cost") {
                if let Some(cost) = other.mana_cost() {
                    format!(
                        "You may pay {} rather than pay this spell's mana cost",
                        cost.to_oracle()
                    )
                } else {
                    "You may cast this spell rather than pay its mana cost".to_string()
                }
            } else if let Some(cost) = other.mana_cost() {
                format!(
                    "Alternative cast {}: {} {}",
                    idx + 1,
                    other.name(),
                    cost.to_oracle()
                )
            } else {
                format!("Alternative cast {}: {}", idx + 1, other.name())
            }
        }
    }
}

fn alternative_cast_method_matches_kind(
    method: &AlternativeCastingMethod,
    kind: crate::filter::AlternativeCastKind,
) -> bool {
    use crate::filter::AlternativeCastKind;

    matches!(
        (kind, method),
        (
            AlternativeCastKind::Blitz,
            AlternativeCastingMethod::Blitz { .. }
        ) | (
            AlternativeCastKind::Dash,
            AlternativeCastingMethod::Dash { .. }
        ) | (
            AlternativeCastKind::Flashback,
            AlternativeCastingMethod::Flashback { .. }
        ) | (
            AlternativeCastKind::JumpStart,
            AlternativeCastingMethod::JumpStart { .. }
        ) | (
            AlternativeCastKind::Escape,
            AlternativeCastingMethod::Escape { .. }
        ) | (
            AlternativeCastKind::Madness,
            AlternativeCastingMethod::Madness { .. }
        ) | (
            AlternativeCastKind::Miracle,
            AlternativeCastingMethod::Miracle { .. }
        ) | (
            AlternativeCastKind::Suspend,
            AlternativeCastingMethod::Suspend { .. }
        )
    )
}

fn qualified_cost_reduction_ability_for_method<'a>(
    def: &'a CardDefinition,
    method: &AlternativeCastingMethod,
) -> Option<&'a crate::static_abilities::StaticAbility> {
    let mut matches = def.abilities.iter().filter_map(|ability| {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            return None;
        };
        let reduction = static_ability.this_spell_cost_reduction()?;
        reduction
            .alternative_cast
            .filter(|kind| alternative_cast_method_matches_kind(method, *kind))?;
        Some(static_ability)
    });
    let ability = matches.next()?;
    matches.next().is_none().then_some(ability)
}

fn describe_alternative_cast_with_qualified_reduction(
    def: &CardDefinition,
    method: &AlternativeCastingMethod,
    idx: usize,
) -> Option<String> {
    let reduction = qualified_cost_reduction_ability_for_method(def, method)?;
    let mut method_line = describe_alternative_cast_line(method, idx);
    if matches!(method, AlternativeCastingMethod::Flashback { .. }) {
        method_line = method_line.replacen("Flashback—", "Flashback ", 1);
    }
    Some(format!(
        "{}. {}",
        method_line.trim_end_matches('.'),
        reduction.display().trim_end_matches('.')
    ))
}

fn ability_is_rendered_with_alternative_cast(def: &CardDefinition, ability: &Ability) -> bool {
    let AbilityKind::Static(static_ability) = &ability.kind else {
        return false;
    };
    if static_ability
        .this_spell_cost_reduction()
        .and_then(|reduction| reduction.alternative_cast)
        .is_none()
    {
        return false;
    }
    def.alternative_casts
        .iter()
        .filter_map(|method| qualified_cost_reduction_ability_for_method(def, method))
        .any(|rendered| std::ptr::eq(rendered, static_ability))
}

fn render_madness_cost(cost: &crate::mana::ManaCost) -> String {
    let pips = cost.pips();
    if !pips.is_empty()
        && pips
            .iter()
            .all(|pip| matches!(pip.as_slice(), [crate::mana::ManaSymbol::Colorless]))
    {
        let amount = small_number_word(pips.len() as u32).unwrap_or_else(|| pips.len().to_string());
        return format!("Madness—Pay {amount} {{C}}");
    }
    format!("Madness {}", cost.to_oracle())
}

fn aura_attaches_to_plain_creature(
    attachment: Option<&crate::object::AuraAttachmentFilter>,
) -> bool {
    let Some(crate::object::AuraAttachmentFilter::Object(filter)) = attachment else {
        return false;
    };
    if !matches!(filter.zone, None | Some(Zone::Battlefield)) {
        return false;
    }
    let mut normalized = filter.clone();
    // `ObjectFilter::creature()` carries the battlefield zone by default.
    // Treat an omitted zone as that same ordinary Aura attachment domain.
    normalized.zone = Some(Zone::Battlefield);
    normalized == ObjectFilter::creature()
}

fn rewrite_plain_creature_aura_subject(
    attachment: Option<&crate::object::AuraAttachmentFilter>,
    line: &str,
) -> String {
    if !aura_attaches_to_plain_creature(attachment) {
        return line.to_string();
    }
    line.replace("Enchanted permanent", "Enchanted creature")
        .replace("enchanted permanent", "enchanted creature")
}

fn is_exact_discard_source_cost(cost: &crate::costs::Cost) -> bool {
    let Some(effect) = cost.effect_ref() else {
        return false;
    };
    let Some(discard) = effect.downcast_ref::<crate::effects::DiscardEffect>() else {
        return false;
    };
    discard.count == Value::Fixed(1)
        && discard.player == PlayerFilter::You
        && !discard.random
        && !discard.any_number
        && discard.card_filter.as_ref() == Some(&ObjectFilter::source().in_zone(Zone::Hand))
        && discard.tag.is_none()
}

/// Recover Reinforce only from its complete executable shape. This deliberately
/// ignores activation display text: the hand zone, exact mana/discard cost,
/// and sole fixed +1/+1-counter effect prove the keyword and its amount.
fn describe_structural_reinforce_ability(ability: &Ability) -> Option<String> {
    if ability.functional_zones.as_slice() != [Zone::Hand] {
        return None;
    }
    let AbilityKind::Activated(activated) = &ability.kind else {
        return None;
    };
    if !activated.choices.is_empty()
        || activated.timing != ActivationTiming::AnyTime
        || activated.is_loyalty_ability
        || !activated.additional_restrictions.is_empty()
        || !activated.activation_restrictions.is_empty()
        || activated.mana_output.is_some()
        || activated.activation_condition.is_some()
        || !activated.mana_usage_restrictions.is_empty()
    {
        return None;
    }

    let costs = activated.mana_cost.as_all()?;
    if costs.len() != 2 {
        return None;
    }
    let mut mana_cost = None;
    let mut saw_discard_source = false;
    for cost in costs {
        if let Some(mana) = cost.mana_cost_ref() {
            if mana_cost.replace(mana).is_some() {
                return None;
            }
        } else if is_exact_discard_source_cost(cost) {
            if saw_discard_source {
                return None;
            }
            saw_discard_source = true;
        } else {
            return None;
        }
    }
    let mana_cost = mana_cost?;
    if !saw_discard_source {
        return None;
    }

    let [segment] = activated.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let [effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let put = effect.downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.counter_type != crate::object::CounterType::PlusOnePlusOne
        || put.target_count.is_some()
        || put.distributed
        || put.target
            != ChooseSpec::target(ChooseSpec::Object(
                ObjectFilter::creature().in_zone(Zone::Battlefield),
            ))
    {
        return None;
    }
    let Value::Fixed(amount) = &put.amount else {
        return None;
    };
    if *amount <= 0 {
        return None;
    }

    Some(format!("Reinforce {amount}—{}", mana_cost.to_oracle()))
}

fn compiled_lines_inner(def: &CardDefinition) -> Vec<String> {
    let mut out = Vec::new();
    let mut leading_alternative_cast_lines = Vec::new();
    let mut alternative_cast_lines = Vec::new();
    let mut deferred_spell_optional_lines = Vec::new();
    let subject = subject_for_card(&def.card);
    let rewrite_it_deals = def.card.card_types.contains(&CardType::Creature)
        || def.card.card_types.contains(&CardType::Artifact)
        || def.card.card_types.contains(&CardType::Land)
        || def.card.card_types.contains(&CardType::Planeswalker)
        || def.card.card_types.contains(&CardType::Battle);
    let spell_like_card = def.card.card_types.contains(&CardType::Instant)
        || def.card.card_types.contains(&CardType::Sorcery);
    let has_attach_only_spell_effect = def.spell_effect.as_ref().is_some_and(|effects| {
        effects.len() == 1
            && effects[0]
                .downcast_ref::<crate::effects::AttachToEffect>()
                .is_some()
    });
    for (idx, method) in def.alternative_casts.iter().enumerate() {
        let line = describe_alternative_cast_with_qualified_reduction(def, method, idx)
            .unwrap_or_else(|| describe_alternative_cast_line(method, idx));
        let is_prototype = method.name().eq_ignore_ascii_case("Prototype")
            && method.prototype_power_toughness().is_some();
        if is_prototype
            || matches!(
                method,
                AlternativeCastingMethod::FlashWithAdditionalCost { .. }
            )
            || line.contains("rather than pay this spell's mana cost")
        {
            leading_alternative_cast_lines.push(line);
        } else {
            alternative_cast_lines.push(line);
        }
    }
    out.extend(leading_alternative_cast_lines);
    for cost in &def.optional_costs {
        let line = describe_optional_cost_line(cost);
        if spell_like_card && matches!(cost.kind, crate::cost::OptionalCostKind::Conspire) {
            deferred_spell_optional_lines.push(line);
        } else {
            out.push(line);
        }
    }
    let has_visible_gift_line = def
        .optional_costs
        .iter()
        .any(|cost| matches!(cost.kind, crate::cost::OptionalCostKind::Gift));
    let leading_aura_flash_ability = def.aura_attach_filter.as_ref().and_then(|_| {
        def.abilities.iter().position(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::Flash
            )
        })
    });
    if leading_aura_flash_ability.is_some() {
        out.push("Flash".to_string());
    }
    if let Some(filter) = &def.aura_attach_filter {
        out.push(format!("Enchant {}", describe_enchant_filter(filter)));
    }
    let derived_final_chapter = def
        .abilities
        .iter()
        .filter_map(|ability| {
            if let AbilityKind::Triggered(triggered) = &ability.kind {
                triggered
                    .trigger
                    .saga_chapters()
                    .and_then(|chapters| chapters.iter().copied().max())
            } else {
                None
            }
        })
        .max();
    if let Some(max_chapter) = derived_final_chapter
        && let Some(roman) = chapter_number_to_roman(max_chapter)
    {
        out.push(format!(
            "(As this Saga enters and after your draw step, add a lore counter. Sacrifice after {roman}.)"
        ));
    }

    let has_begin_on_battlefield_pregame = def
        .abilities
        .iter()
        .any(ability_has_begin_on_battlefield_pregame);
    let render_leading_spell_abilities = spell_like_card
        && def.abilities.iter().any(|ability| {
            ability_precedes_spell_resolution(ability)
                && !ability_is_rendered_with_alternative_cast(def, ability)
        });
    let push_abilities = |output: &mut Vec<String>| {
        let has_suspend = def
            .alternative_casts
            .iter()
            .any(|method| matches!(method, AlternativeCastingMethod::Suspend { .. }));
        let has_squad = def
            .optional_costs
            .iter()
            .any(|cost| matches!(cost.kind, crate::cost::OptionalCostKind::Squad));
        let mut ability_idx = 0usize;
        while ability_idx < def.abilities.len() {
            let ability = &def.abilities[ability_idx];
            if leading_aura_flash_ability == Some(ability_idx) {
                ability_idx += 1;
                continue;
            }
            if ability_is_rendered_with_alternative_cast(def, ability) {
                ability_idx += 1;
                continue;
            }
            if render_leading_spell_abilities && ability_precedes_spell_resolution(ability) {
                ability_idx += 1;
                continue;
            }
            if has_suspend && is_suspend_helper_ability(ability) {
                ability_idx += 1;
                continue;
            }
            if is_conspire_helper_ability(ability) {
                ability_idx += 1;
                continue;
            }
            if has_squad && is_squad_helper_ability(ability) {
                ability_idx += 1;
                continue;
            }
            if has_visible_gift_line && is_hidden_gift_etb_ability(ability) {
                ability_idx += 1;
                continue;
            }
            if ability_level_range_prefix(ability).is_some() {
                ability_idx += 1;
                continue;
            }
            if let Some((text, consumed)) =
                describe_structural_enchanted_combat_activation_restriction_bundle(
                    &def.abilities[ability_idx..],
                )
            {
                output.push(format!("Static ability {}: {text}", ability_idx + 1));
                ability_idx += consumed;
                continue;
            }
            if let Some((text, consumed)) = describe_structural_conditional_anthem_otherwise_bundle(
                &def.abilities[ability_idx..],
            ) {
                output.push(format!("Static ability {}: {text}", ability_idx + 1));
                ability_idx += consumed;
                continue;
            }
            if let Some((text, consumed)) =
                describe_structural_modifier_type_addition_bundle(&def.abilities[ability_idx..])
            {
                output.push(format!("Static ability {}: {text}", ability_idx + 1));
                ability_idx += consumed;
                continue;
            }
            if let Some((text, consumed)) =
                describe_authored_attached_transform_bundle(&def.abilities[ability_idx..])
            {
                output.push(format!("Static ability {}: {text}", ability_idx + 1));
                ability_idx += consumed;
                continue;
            }
            if let Some((text, consumed)) =
                describe_labeled_static_bundle(&def.abilities[ability_idx..], subject)
            {
                output.push(format!("Static ability {}: {text}", ability_idx + 1));
                ability_idx += consumed;
                continue;
            }
            if let Some((text, consumed)) = describe_structural_anthem_remove_all_abilities_bundle(
                &def.abilities[ability_idx..],
            ) {
                output.push(format!("Static ability {}: {text}", ability_idx + 1));
                ability_idx += consumed;
                continue;
            }
            if let Some(keyword) = describe_structural_ascend_ability(ability) {
                output.push(format!("Keyword ability {}: {keyword}", ability_idx + 1));
                ability_idx += 1;
                continue;
            }
            if let Some(keyword) = describe_structural_reinforce_ability(ability) {
                output.push(format!("Keyword ability {}: {keyword}", ability_idx + 1));
                ability_idx += 1;
                continue;
            }
            if let Some(keyword) = describe_structural_equipment_token_keyword(ability) {
                output.push(format!("Keyword ability {}: {keyword}", ability_idx + 1));
                ability_idx += 1;
                continue;
            }
            if let Some(keyword) = describe_structural_evolve_keyword(ability) {
                output.push(format!("Keyword ability {}: {keyword}", ability_idx + 1));
                ability_idx += 1;
                continue;
            }
            if let Some(keyword) = describe_structural_training_keyword(ability) {
                output.push(format!("Keyword ability {}: {keyword}", ability_idx + 1));
                ability_idx += 1;
                continue;
            }
            if let Some(keyword) = describe_structural_ingest_keyword(ability) {
                output.push(format!("Keyword ability {}: {keyword}", ability_idx + 1));
                ability_idx += 1;
                continue;
            }
            if let Some(keyword) = describe_structural_station_keyword(ability) {
                output.push(format!("Keyword ability {}: {keyword}", ability_idx + 1));
                ability_idx += 1;
                continue;
            }
            if ability_idx + 1 < def.abilities.len()
                && let Some(partner_with) =
                    describe_structural_partner_with_pair(ability, &def.abilities[ability_idx + 1])
            {
                output.push(format!(
                    "Keyword ability {}: {partner_with}",
                    ability_idx + 1
                ));
                ability_idx += 2;
                continue;
            }
            if ability_idx + 1 < def.abilities.len()
                && let Some(keyword) =
                    describe_structural_rampage_pair(ability, &def.abilities[ability_idx + 1])
            {
                output.push(format!("Keyword ability {}: {keyword}", ability_idx + 1));
                ability_idx += 2;
                continue;
            }
            if ability_idx + 1 < def.abilities.len()
                && let Some(permission) = describe_structural_graveyard_or_exile_cast_pair(
                    ability,
                    &def.abilities[ability_idx + 1],
                )
            {
                output.push(format!("Static ability {}: {permission}", ability_idx + 1));
                ability_idx += 2;
                continue;
            }
            if let Some((keyword, consumed)) =
                describe_structural_counter_keyword_bundle(&def.abilities[ability_idx..])
            {
                output.push(format!("Keyword ability {}: {keyword}", ability_idx + 1));
                ability_idx += consumed;
                continue;
            }
            if let Some((text, consumed)) =
                describe_structural_attached_land_type_setting_bundle(&def.abilities[ability_idx..])
            {
                output.push(format!("Static ability {}: {text}", ability_idx + 1));
                ability_idx += consumed;
                continue;
            }
            if let Some((text, consumed)) =
                describe_structural_type_base_pt_addition_bundle(&def.abilities[ability_idx..])
            {
                output.push(format!("Static ability {}: {text}", ability_idx + 1));
                ability_idx += consumed;
                continue;
            }
            if let Some((text, consumed)) =
                describe_structural_pt_color_type_addition_bundle(&def.abilities[ability_idx..])
            {
                output.push(format!("Static ability {}: {text}", ability_idx + 1));
                ability_idx += consumed;
                continue;
            }
            if ability_idx + 1 < def.abilities.len()
                && let Some(echo) =
                    describe_structural_echo_pair(ability, &def.abilities[ability_idx + 1])
            {
                output.push(format!("Keyword ability {}: {echo}", ability_idx + 1));
                ability_idx += 2;
                continue;
            }
            if let AbilityKind::Activated(first) = &ability.kind
                && first.is_mana_ability()
                && first.effects.is_empty()
                && first.activation_condition.is_none()
                && first.additional_restrictions.is_empty()
                && first.mana_usage_restrictions.is_empty()
                && first.mana_symbols().len() == 1
            {
                let mut symbols = vec![first.mana_symbols()[0]];
                let mut consumed = 1usize;
                while ability_idx + consumed < def.abilities.len() {
                    let next = &def.abilities[ability_idx + consumed];
                    let AbilityKind::Activated(next_mana) = &next.kind else {
                        break;
                    };
                    if !next_mana.is_mana_ability()
                        || !next_mana.effects.is_empty()
                        || next_mana.activation_condition.is_some()
                        || !next_mana.additional_restrictions.is_empty()
                        || !next_mana.mana_usage_restrictions.is_empty()
                        || next_mana.mana_symbols().len() != 1
                        || next_mana.mana_cost != first.mana_cost
                    {
                        break;
                    }
                    symbols.push(next_mana.mana_symbols()[0]);
                    consumed += 1;
                }
                if consumed > 1 {
                    let mut line = format!("Mana ability {}", ability_idx + 1);
                    let add = format!("Add {}", describe_mana_alternatives(&symbols));
                    if !first.mana_cost.costs().is_empty() {
                        let cost = describe_cost_list(first.mana_cost.costs());
                        line.push_str(": ");
                        line.push_str(&cost);
                        line.push_str(": ");
                        line.push_str(&add);
                    } else {
                        line.push_str(": ");
                        line.push_str(&add);
                    }
                    line = normalize_ability_self_reference_surface(&line, subject);
                    output.push(line);
                    ability_idx += consumed;
                    continue;
                }
            }
            let ability_subject = if ability_prefers_card_name_subject(ability) {
                def.card.name.as_str()
            } else {
                subject
            };
            let mut ability_lines =
                describe_ability(ability_idx + 1, ability, ability_subject, rewrite_it_deals);
            if ability_has_level_tiers(ability) {
                ability_lines = interleave_level_range_activations(
                    ability_lines,
                    &def.abilities,
                    ability_subject,
                    rewrite_it_deals,
                );
            }
            if ability_has_begin_on_battlefield_pregame(ability) {
                for line in &mut ability_lines {
                    *line = substitute_pregame_self_reference(line, &def.card.name);
                }
            }
            if has_begin_on_battlefield_pregame {
                for line in &mut ability_lines {
                    *line = substitute_pregame_card_self_reference(line, subject, &def.card.name);
                }
            }
            let oracle_short = oracle_short_self_name(def);
            for line in &mut ability_lines {
                *line = substitute_legendary_source_reference(
                    line,
                    &def.card,
                    subject,
                    oracle_short.as_deref(),
                );
                *line = rewrite_plain_creature_aura_subject(def.aura_attach_filter.as_ref(), line);
            }
            output.extend(ability_lines);
            ability_idx += 1;
        }
    };

    let additional_costs = def.additional_non_mana_costs();
    if !additional_costs.is_empty() {
        let additional_cost_text = describe_additional_costs(&additional_costs);
        let additional_cost_lower = additional_cost_text.to_ascii_lowercase();
        // Only the explicit optional keyword is Blight. A mandatory printed
        // "put a -1/-1 counter" additional cost must stay mandatory.
        let additional_cost_text = if additional_cost_lower == "you may blight 1" {
            "you may blight 1 by putting a -1/-1 counter on a creature you control".to_string()
        } else {
            additional_cost_text
        };
        out.push(format!(
            "As an additional cost to cast this spell, {}",
            lowercase_first(&additional_cost_text)
        ));
    }
    if !spell_like_card {
        push_abilities(&mut out);
    }
    if render_leading_spell_abilities {
        for (idx, ability) in def.abilities.iter().enumerate() {
            if !ability_precedes_spell_resolution(ability) {
                continue;
            }
            if ability_is_rendered_with_alternative_cast(def, ability) {
                continue;
            }
            let ability_subject = if ability_prefers_card_name_subject(ability) {
                def.card.name.as_str()
            } else {
                subject
            };
            let mut ability_lines =
                describe_ability(idx + 1, ability, ability_subject, rewrite_it_deals);
            let oracle_short = oracle_short_self_name(def);
            for line in &mut ability_lines {
                *line = substitute_legendary_source_reference(
                    line,
                    &def.card,
                    subject,
                    oracle_short.as_deref(),
                );
                *line = rewrite_plain_creature_aura_subject(def.aura_attach_filter.as_ref(), line);
            }
            out.extend(ability_lines);
        }
    }
    if let Some(spell_effects) = &def.spell_effect
        && !spell_effects.is_empty()
        && !(def.aura_attach_filter.is_some() && has_attach_only_spell_effect)
    {
        if is_choose_background_spell_effect(spell_effects) {
            out.push("Keyword ability 0: Choose a Background".to_string());
        } else {
            let spell_text = rewrite_additional_sacrifice_reference_surface(
                def,
                &describe_resolution_program_for_card(def, spell_effects),
            );
            out.push(format!("Spell effects: {}", spell_text));
        }
    }
    out.extend(deferred_spell_optional_lines);
    if spell_like_card {
        push_abilities(&mut out);
    }
    if def.has_fuse {
        out.push("Fuse".to_string());
    }
    out.extend(alternative_cast_lines);
    let oracle_short = oracle_short_self_name(def);
    merge_adjacent_keyword_surface_lines(out)
        .into_iter()
        .map(|line| {
            substitute_legendary_source_reference(
                &line,
                &def.card,
                subject,
                oracle_short.as_deref(),
            )
        })
        .collect()
}

fn ability_prefers_card_name_subject(ability: &Ability) -> bool {
    let AbilityKind::Static(static_ability) = &ability.kind else {
        return false;
    };
    if static_ability.prefers_card_name_subject() {
        return true;
    }
    if static_ability.id() == crate::static_abilities::StaticAbilityId::CanBeCommander {
        return true;
    }
    static_ability
        .enter_as_copy_as_enters()
        .is_some_and(|spec| spec.linked_exile_pair.is_some())
}

fn static_display_with_id(
    ability: &Ability,
    expected_id: crate::static_abilities::StaticAbilityId,
) -> Option<String> {
    let AbilityKind::Static(static_ability) = &ability.kind else {
        return None;
    };
    (static_ability.id() == expected_id).then(|| {
        static_ability
            .display()
            .trim()
            .trim_end_matches('.')
            .to_string()
    })
}

fn static_bundle_subjects_match(first: &str, next: &str) -> bool {
    if first.eq_ignore_ascii_case(next) {
        return true;
    }
    let normalize_each = |subject: &str| {
        let lower = subject.trim().to_ascii_lowercase();
        let Some(rest) = lower.strip_prefix("each ") else {
            return lower;
        };
        let Some((noun, tail)) = rest.split_once(' ') else {
            return format!("{rest}s");
        };
        let plural = match noun {
            "creature" => "creatures".to_string(),
            "artifact" => "artifacts".to_string(),
            "enchantment" => "enchantments".to_string(),
            "permanent" => "permanents".to_string(),
            "land" => "lands".to_string(),
            "token" => "tokens".to_string(),
            other => format!("{other}s"),
        };
        format!("{plural} {tail}")
    };
    normalize_each(first) == normalize_each(next)
}

fn render_static_bundle_list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [only] => only.clone(),
        [left, right] => format!("{left} and {right}"),
        _ => {
            let (last, leading) = items.split_last().expect("nonempty static bundle list");
            format!("{}, and {last}", leading.join(", "))
        }
    }
}

/// Recombine layer-correct static pieces from one authored modifier clause.
/// The terminal type addition is the structural anchor: every consumed piece
/// must affect the same subject, and only P/T, granted-ability, and additive
/// type predicates are accepted.
fn describe_structural_modifier_type_addition_bundle(
    abilities: &[Ability],
) -> Option<(String, usize)> {
    let AbilityKind::Static(first) = &abilities.first()?.kind else {
        return None;
    };
    let first_display = first.display();
    let (subject, first_tail, first_verb) = match first.id() {
        crate::static_abilities::StaticAbilityId::Anthem => {
            split_static_predicate_with_verb(&first_display, &[" gets ", " get "])?
        }
        crate::static_abilities::StaticAbilityId::SetBasePowerToughnessForFilter => {
            split_static_predicate_with_verb(&first_display, &[" has ", " have "])?
        }
        _ => return None,
    };
    let singular = matches!(first_verb, "gets" | "has");
    let first_predicate = format!("{first_verb} {first_tail}");
    let mut grant_items = Vec::new();
    let mut has_quoted_grant = false;
    let mut removes_all_other_abilities = false;
    let mut color_descriptors = Vec::new();
    let mut colors_are_additive = false;
    let mut type_descriptors = Vec::new();
    let mut consumed = 1usize;
    let mut saw_type_addition = false;

    while let Some(Ability {
        kind: AbilityKind::Static(next),
        ..
    }) = abilities.get(consumed)
    {
        let display = next.display();
        match next.id() {
            crate::static_abilities::StaticAbilityId::RemoveAllAbilitiesForFilter => {
                if saw_type_addition || removes_all_other_abilities {
                    break;
                }
                let next_subject = display
                    .strip_suffix(" lose all abilities")
                    .or_else(|| display.strip_suffix(" loses all abilities"))?;
                if !static_bundle_subjects_match(subject, next_subject) {
                    break;
                }
                removes_all_other_abilities = true;
            }
            crate::static_abilities::StaticAbilityId::GrantAbility => {
                if saw_type_addition {
                    break;
                }
                let (next_subject, tail, _) =
                    split_static_predicate_with_verb(&display, &[" has ", " have "])?;
                if !static_bundle_subjects_match(subject, next_subject) {
                    break;
                }
                grant_items.push(tail.to_string());
            }
            crate::static_abilities::StaticAbilityId::AttachedAbilityGrant => {
                if saw_type_addition {
                    break;
                }
                let (next_subject, tail, _) =
                    split_static_predicate_with_verb(&display, &[" has ", " have "])?;
                if !static_bundle_subjects_match(subject, next_subject) {
                    break;
                }
                let body = tail.trim().trim_matches('"').trim_end_matches('.');
                let terminal = if body.ends_with('?') || body.ends_with('!') {
                    ""
                } else {
                    "."
                };
                grant_items.push(format!("\"{body}{terminal}\""));
                has_quoted_grant = true;
            }
            crate::static_abilities::StaticAbilityId::SetColors
            | crate::static_abilities::StaticAbilityId::AddColors => {
                if saw_type_addition {
                    break;
                }
                let (next_subject, tail, _) =
                    split_static_predicate_with_verb(&display, &[" is ", " are "])?;
                if !static_bundle_subjects_match(subject, next_subject) {
                    break;
                }
                let (descriptor, additive) = if next.id()
                    == crate::static_abilities::StaticAbilityId::AddColors
                {
                    (
                        tail.strip_suffix(" in addition to its other colors")
                            .or_else(|| {
                                tail.strip_suffix(" in addition to their other colors")
                            })?,
                        true,
                    )
                } else {
                    (tail, false)
                };
                color_descriptors.push(descriptor.trim().to_string());
                colors_are_additive |= additive;
            }
            crate::static_abilities::StaticAbilityId::AddCardTypes
            | crate::static_abilities::StaticAbilityId::AddSubtypes => {
                let (next_subject, tail, type_verb) =
                    split_static_predicate_with_verb(&display, &[" is ", " are "])?;
                if !static_bundle_subjects_match(subject, next_subject) {
                    break;
                }
                let descriptor = tail
                    .strip_suffix(" in addition to its other types")
                    .or_else(|| tail.strip_suffix(" in addition to their other types"))?;
                let descriptor = descriptor
                    .strip_prefix("a ")
                    .or_else(|| descriptor.strip_prefix("an "))
                    .unwrap_or(descriptor)
                    .trim();
                let descriptor = if singular && type_verb == "are" {
                    let mut words = descriptor
                        .split_whitespace()
                        .map(str::to_string)
                        .collect::<Vec<_>>();
                    if let Some(last) = words.last_mut() {
                        *last = singular_subtype_word(last);
                    }
                    words.join(" ")
                } else {
                    descriptor.to_string()
                };
                type_descriptors.push(descriptor);
                saw_type_addition = true;
            }
            _ => break,
        }
        consumed += 1;
    }

    if !saw_type_addition || type_descriptors.is_empty() {
        return None;
    }
    let mut descriptors = color_descriptors;
    descriptors.extend(type_descriptors);
    let descriptor = descriptors.join(" ");
    let descriptor = if singular {
        with_indefinite_article(&descriptor)
    } else {
        descriptor
    };
    let scopes = if colors_are_additive {
        if singular {
            "its other colors and types"
        } else {
            "their other colors and types"
        }
    } else if singular {
        "its other types"
    } else {
        "their other types"
    };
    let type_predicate = format!(
        "{} {descriptor} in addition to {scopes}",
        if singular { "is" } else { "are" },
    );

    let text = if grant_items.is_empty() && removes_all_other_abilities {
        format!(
            "{subject} {first_predicate}, loses all other abilities, and {type_predicate}"
        )
    } else if grant_items.is_empty() {
        format!("{subject} {first_predicate} and {type_predicate}")
    } else {
        let grant_predicate = format!("has {}", render_static_bundle_list(&grant_items));
        if removes_all_other_abilities {
            format!(
                "{subject} {first_predicate}, {grant_predicate}, loses all other abilities, and {type_predicate}"
            )
        } else if has_quoted_grant && singular {
            format!(
                "{subject} {first_predicate} and {grant_predicate}. It's {descriptor} in addition to its other types"
            )
        } else {
            format!(
                "{subject} {first_predicate}, {grant_predicate}, and {type_predicate}"
            )
        }
    };
    Some((text, consumed))
}

fn static_subject_for_attached_transform_piece(ability: &Ability) -> Option<String> {
    let AbilityKind::Static(static_ability) = &ability.kind else {
        return None;
    };
    let display = static_ability.display();
    match static_ability.id() {
        crate::static_abilities::StaticAbilityId::AddSubtypes
        | crate::static_abilities::StaticAbilityId::SetColors
        | crate::static_abilities::StaticAbilityId::MakeColorless => {
            split_static_predicate_with_verb(&display, &[" is ", " are "])
                .map(|(subject, _, _)| subject.to_string())
        }
        crate::static_abilities::StaticAbilityId::RemoveAllAbilitiesForFilter => display
            .strip_suffix(" lose all abilities")
            .or_else(|| display.strip_suffix(" loses all abilities"))
            .map(str::to_string),
        _ => None,
    }
}

/// Attached characteristic-setting lines lower into independent layer-correct
/// static abilities. Preserve the authored line on SetCardTypes and consume the
/// adjacent pieces only when their subjects prove they belong to that line.
fn describe_authored_attached_transform_bundle(
    abilities: &[Ability],
) -> Option<(String, usize)> {
    let first_static = match &abilities.first()?.kind {
        AbilityKind::Static(static_ability) => static_ability,
        _ => return None,
    };
    let (set_idx, grant_tail) = if first_static.id()
        == crate::static_abilities::StaticAbilityId::AttachedAbilityGrant
    {
        let display = first_static.display();
        let (_, tail) = split_static_predicate(&display, " has ")?;
        (1usize, Some(tail.to_string()))
    } else {
        (0usize, None)
    };

    let AbilityKind::Static(set_types) = &abilities.get(set_idx)?.kind else {
        return None;
    };
    if set_types.id() != crate::static_abilities::StaticAbilityId::SetCardTypes {
        return None;
    }
    let surface = set_types.authored_line_surface()?;
    if let Some(grant_tail) = grant_tail {
        let surface_lower = surface.to_ascii_lowercase();
        let grant_lower = grant_tail.to_ascii_lowercase();
        if !surface_lower.contains(&grant_lower) {
            return None;
        }
    } else if surface.to_ascii_lowercase().contains(" with ") {
        return None;
    }

    let set_display = set_types.display();
    let (subject, _, _) =
        split_static_predicate_with_verb(&set_display, &[" is ", " are "])?;
    let mut consumed = set_idx + 1;
    while let Some(next) = abilities.get(consumed) {
        let Some(next_subject) = static_subject_for_attached_transform_piece(next) else {
            break;
        };
        if next_subject != subject {
            break;
        }
        consumed += 1;
    }

    Some((surface.trim().trim_end_matches('.').to_string(), consumed))
}

fn exact_enchanted_restriction_subject(filter: &ObjectFilter) -> Option<&'static str> {
    let tagged = |filter: ObjectFilter| {
        filter.match_tagged(
            "enchanted",
            crate::target::TaggedOpbjectRelation::IsTaggedObject,
        )
    };
    [
        ("Enchanted creature", tagged(ObjectFilter::creature())),
        (
            "Enchanted permanent",
            tagged(ObjectFilter::permanent_card().in_zone(Zone::Battlefield)),
        ),
        ("Enchanted artifact", tagged(ObjectFilter::artifact())),
    ]
    .into_iter()
    .find_map(|(subject, expected)| (filter == &expected).then_some(subject))
}

fn restriction_display_matches(display: &str, expected: &str) -> bool {
    display
        .trim()
        .trim_end_matches('.')
        .eq_ignore_ascii_case(expected)
}

/// Recombine the two adjacent typed restrictions produced from Oracle's
/// compound enchanted-object sentence. This intentionally accepts only the
/// exact no-exception restrictions for the exact same enchanted object filter.
fn describe_structural_enchanted_combat_activation_restriction_bundle(
    abilities: &[Ability],
) -> Option<(String, usize)> {
    let [combat_ability, activation_ability, ..] = abilities else {
        return None;
    };
    if combat_ability.functional_zones.as_slice() != [Zone::Battlefield]
        || activation_ability.functional_zones != combat_ability.functional_zones
    {
        return None;
    }

    let AbilityKind::Static(combat_static) = &combat_ability.kind else {
        return None;
    };
    let (combat_restriction, combat_display, combat_condition) =
        combat_static.rule_restriction_parts()?;
    if combat_condition.is_some() {
        return None;
    }
    let crate::effect::Restriction::AttackOrBlock(combat_filter) = combat_restriction else {
        return None;
    };

    let AbilityKind::Static(activation_static) = &activation_ability.kind else {
        return None;
    };
    let (activation_restriction, activation_display, activation_condition) =
        activation_static.rule_restriction_parts()?;
    if activation_condition.is_some() {
        return None;
    }
    let crate::effect::Restriction::ActivateAbilitiesOf(activation_filter) = activation_restriction
    else {
        return None;
    };
    if combat_filter != activation_filter {
        return None;
    }

    let subject = exact_enchanted_restriction_subject(combat_filter)?;
    let subject_lower = subject.to_ascii_lowercase();
    if !restriction_display_matches(
        combat_display,
        &format!("{subject_lower} can't attack or block"),
    ) || !restriction_display_matches(
        activation_display,
        &format!("{subject_lower} activated abilities can't be activated"),
    ) {
        return None;
    }

    Some((
        format!("{subject} can't attack or block, and its activated abilities can't be activated"),
        2,
    ))
}

fn split_static_predicate<'a>(display: &'a str, marker: &str) -> Option<(&'a str, &'a str)> {
    let (subject, predicate) = display.split_once(marker)?;
    (!subject.trim().is_empty() && !predicate.trim().is_empty())
        .then_some((subject.trim(), predicate.trim()))
}

fn exact_enchanted_creature_anthem_filter(filter: &ObjectFilter) -> bool {
    let expected = ObjectFilter::creature().match_tagged(
        "enchanted",
        crate::target::TaggedOpbjectRelation::IsTaggedObject,
    );
    filter == &expected
}

/// Describe the simple characteristic predicates used by a mutually-exclusive
/// attached-creature anthem pair. Removing the one rendered characteristic
/// must leave the default filter, so this cannot silently omit extra facts.
fn describe_simple_attached_match(filter: &ObjectFilter) -> Option<String> {
    let mut normalized = filter.clone();
    if normalized.zone == Some(Zone::Battlefield) {
        normalized.zone = None;
    }
    let filter = &normalized;

    if let [subtype] = filter.subtypes.as_slice() {
        let mut remainder = filter.clone();
        remainder.subtypes.clear();
        if remainder == ObjectFilter::default() {
            return Some(with_indefinite_article(&subtype.to_string()));
        }
    }

    if let [card_type] = filter.card_types.as_slice() {
        let mut remainder = filter.clone();
        remainder.card_types.clear();
        if remainder == ObjectFilter::default() {
            return Some(with_indefinite_article(card_type.name()));
        }
    }

    if let Some(colors) = filter.colors
        && colors.count() == 1
    {
        let mut remainder = filter.clone();
        remainder.colors = None;
        if remainder == ObjectFilter::default() {
            let color = crate::color::Color::ALL
                .into_iter()
                .find(|color| colors.contains(*color))?;
            return Some(color.name().to_string());
        }
    }

    if filter.attacking {
        let mut remainder = filter.clone();
        remainder.attacking = false;
        if remainder == ObjectFilter::default() {
            return Some("attacking".to_string());
        }
    }

    None
}

fn describe_fixed_anthem_modifier(power: i32, toughness: i32) -> String {
    let signed = |value: i32| {
        if value >= 0 {
            format!("+{value}")
        } else {
            value.to_string()
        }
    };
    let toughness = if power < 0 && toughness == 0 {
        "-0".to_string()
    } else {
        signed(toughness)
    };
    format!("{}/{toughness}", signed(power))
}

/// Recombine adjacent typed anthems produced from
/// "... as long as it's X. Otherwise, ...". Both branches must affect the
/// exact same enchanted creature, use fixed P/T-only payloads, and carry exact
/// inverse attachment predicates.
fn describe_structural_conditional_anthem_otherwise_bundle(
    abilities: &[Ability],
) -> Option<(String, usize)> {
    let [positive_ability, negative_ability, ..] = abilities else {
        return None;
    };
    if positive_ability.functional_zones.as_slice() != [Zone::Battlefield]
        || negative_ability.functional_zones != positive_ability.functional_zones
    {
        return None;
    }

    let AbilityKind::Static(positive_static) = &positive_ability.kind else {
        return None;
    };
    let AbilityKind::Static(negative_static) = &negative_ability.kind else {
        return None;
    };
    let positive = positive_static.anthem_payload()?;
    let negative = negative_static.anthem_payload()?;

    let target = positive.filter.as_ref()?;
    if !exact_enchanted_creature_anthem_filter(target)
        || negative.filter.as_ref() != Some(target)
        || positive.replacement_surface.is_some()
        || negative.replacement_surface.is_some()
        || positive.set_quantifier_surface.is_some()
        || negative.set_quantifier_surface.is_some()
        || positive.count_uses_where_x
        || negative.count_uses_where_x
    {
        return None;
    }

    let crate::ConditionExpr::AttachedToSourceMatches(match_filter) =
        positive.condition.as_ref()?
    else {
        return None;
    };
    let crate::ConditionExpr::Not(negative_inner) = negative.condition.as_ref()? else {
        return None;
    };
    if negative_inner.as_ref() != positive.condition.as_ref()? {
        return None;
    }

    let (
        ironsmith_core::AnthemValue::Fixed(positive_power),
        ironsmith_core::AnthemValue::Fixed(positive_toughness),
    ) = (&positive.power, &positive.toughness)
    else {
        return None;
    };
    let (
        ironsmith_core::AnthemValue::Fixed(negative_power),
        ironsmith_core::AnthemValue::Fixed(negative_toughness),
    ) = (&negative.power, &negative.toughness)
    else {
        return None;
    };
    if (*positive_power < 0 || *positive_toughness < 0)
        || (*positive_power == 0 && *positive_toughness == 0)
        || *negative_power >= 0
        || *negative_toughness > 0
    {
        return None;
    }

    let predicate = describe_simple_attached_match(match_filter)?;
    Some((
        format!(
            "Enchanted creature gets {} as long as it's {predicate}. Otherwise, it gets {}",
            describe_fixed_anthem_modifier(*positive_power, *positive_toughness),
            describe_fixed_anthem_modifier(*negative_power, *negative_toughness),
        ),
        2,
    ))
}

fn describe_structural_anthem_remove_all_abilities_bundle(
    abilities: &[Ability],
) -> Option<(String, usize)> {
    let [anthem_ability, remove_abilities_ability, ..] = abilities else {
        return None;
    };
    let anthem = static_display_with_id(
        anthem_ability,
        crate::static_abilities::StaticAbilityId::Anthem,
    )?;
    let remove_abilities = static_display_with_id(
        remove_abilities_ability,
        crate::static_abilities::StaticAbilityId::RemoveAllAbilitiesForFilter,
    )?;
    let (subject, modifier) = split_static_predicate(&anthem, " gets ")?;
    let remove_subject = remove_abilities.strip_suffix(" lose all abilities")?.trim();
    if remove_subject != subject {
        return None;
    }

    Some((
        format!("{subject} gets {modifier} and loses all abilities"),
        2,
    ))
}

fn render_all_union_subject(subject: &str) -> String {
    let pieces = subject
        .split(" or ")
        .map(str::trim)
        .filter(|piece| !piece.is_empty())
        .collect::<Vec<_>>();
    if pieces.len() < 2 {
        return subject.to_string();
    }
    let rendered = pieces
        .iter()
        .map(|piece| {
            if piece.starts_with("all ") || piece.starts_with("All ") {
                piece.to_string()
            } else {
                format!("all {piece}")
            }
        })
        .collect::<Vec<_>>()
        .join(" and ");
    capitalize_first(&rendered)
}

fn singular_subtype_word(word: &str) -> String {
    let trimmed = word.trim_matches(|ch: char| !ch.is_ascii_alphabetic());
    if trimmed.eq_ignore_ascii_case("plains") {
        "Plains".to_string()
    } else if trimmed.ends_with('s') && trimmed.len() > 1 {
        trimmed.trim_end_matches('s').to_string()
    } else {
        trimmed.to_string()
    }
}

fn is_land_subtype_surface(word: &str) -> bool {
    matches!(
        word.to_ascii_lowercase().as_str(),
        "plains" | "island" | "swamp" | "mountain" | "forest"
    )
}

fn render_pt_color_type_addition_descriptor(
    color_text: &str,
    card_type_text: &str,
    subtype_text: &str,
    pt_text: &str,
) -> Option<String> {
    let card_type_words = card_type_text
        .split(" and ")
        .map(str::trim)
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    let has_creatures = card_type_words
        .iter()
        .any(|word| word.eq_ignore_ascii_case("creatures"));
    let has_lands = card_type_words
        .iter()
        .any(|word| word.eq_ignore_ascii_case("lands"));

    let mut creature_subtypes = Vec::new();
    let mut land_subtypes = Vec::new();
    for word in subtype_text.split_whitespace() {
        let subtype = singular_subtype_word(word);
        if subtype.is_empty() {
            continue;
        }
        if is_land_subtype_surface(&subtype) {
            land_subtypes.push(subtype);
        } else {
            creature_subtypes.push(subtype);
        }
    }

    let mut type_phrases = Vec::new();
    if has_creatures {
        if creature_subtypes.is_empty() {
            type_phrases.push("creatures".to_string());
        } else {
            type_phrases.push(format!("{} creatures", creature_subtypes.join(" ")));
        }
    }
    if has_lands {
        if land_subtypes.is_empty() {
            type_phrases.push("lands".to_string());
        } else {
            type_phrases.push(format!("{} lands", land_subtypes.join(" ")));
        }
    }
    for card_type in card_type_words {
        if (card_type.eq_ignore_ascii_case("creatures") && has_creatures)
            || (card_type.eq_ignore_ascii_case("lands") && has_lands)
        {
            continue;
        }
        type_phrases.push(card_type.to_string());
    }
    if type_phrases.is_empty() {
        return None;
    }

    Some(format!(
        "{pt_text} {color_text} {}",
        type_phrases.join(" and ")
    ))
}

fn describe_structural_pt_color_type_addition_bundle(
    abilities: &[Ability],
) -> Option<(String, usize)> {
    let [
        color_ability,
        card_type_ability,
        subtype_ability,
        pt_ability,
        ..,
    ] = abilities
    else {
        return None;
    };
    let color_display = static_display_with_id(
        color_ability,
        crate::static_abilities::StaticAbilityId::SetColors,
    )?;
    let card_type_display = static_display_with_id(
        card_type_ability,
        crate::static_abilities::StaticAbilityId::AddCardTypes,
    )?;
    let subtype_display = static_display_with_id(
        subtype_ability,
        crate::static_abilities::StaticAbilityId::AddSubtypes,
    )?;
    let pt_display = static_display_with_id(
        pt_ability,
        crate::static_abilities::StaticAbilityId::SetBasePowerToughnessForFilter,
    )?;

    let (subject, color_text) = split_static_predicate(&color_display, " are ")?;
    let (card_type_subject, card_type_tail) = split_static_predicate(&card_type_display, " are ")?;
    let (subtype_subject, subtype_tail) = split_static_predicate(&subtype_display, " are ")?;
    let (pt_subject, pt_text) =
        split_static_predicate(&pt_display, " have base power and toughness ")?;
    if card_type_subject != subject || subtype_subject != subject || pt_subject != subject {
        return None;
    }

    let card_type_text = card_type_tail.strip_suffix(" in addition to their other types")?;
    let subtype_text = subtype_tail.strip_suffix(" in addition to their other types")?;
    let descriptor = render_pt_color_type_addition_descriptor(
        color_text,
        card_type_text,
        subtype_text,
        pt_text,
    )?;
    Some((
        format!(
            "{} are {descriptor} in addition to their other types",
            render_all_union_subject(subject)
        ),
        4,
    ))
}

fn split_static_predicate_with_verb<'a>(
    display: &'a str,
    verbs: &[&str],
) -> Option<(&'a str, &'a str, &'static str)> {
    for verb in verbs {
        if let Some((subject, tail)) = split_static_predicate(display, verb) {
            let normalized = match *verb {
                " are " => "are",
                " is " => "is",
                " have " => "have",
                " has " => "has",
                " get " => "get",
                " gets " => "gets",
                _ => continue,
            };
            return Some((subject, tail, normalized));
        }
    }
    None
}

/// Recombine the structural pieces of effects such as "Enchanted permanent is
/// a colorless Forest land." The individual static abilities deliberately keep
/// card-type replacement, land-subtype replacement, and color replacement
/// separate so the layer system can apply them correctly; their oracle surface
/// is one predicate.
fn describe_structural_attached_land_type_setting_bundle(
    abilities: &[Ability],
) -> Option<(String, usize)> {
    let [card_type_ability, subtype_ability, rest @ ..] = abilities else {
        return None;
    };
    let card_type_display = static_display_with_id(
        card_type_ability,
        crate::static_abilities::StaticAbilityId::SetCardTypes,
    )?;
    let subtype_display = static_display_with_id(
        subtype_ability,
        crate::static_abilities::StaticAbilityId::SetLandSubtypes,
    )?;
    let (subject, card_type_tail, type_verb) =
        split_static_predicate_with_verb(&card_type_display, &[" is ", " are "])?;
    let (subtype_subject, subtype_tail, subtype_verb) =
        split_static_predicate_with_verb(&subtype_display, &[" is ", " are "])?;
    if subtype_subject != subject || subtype_verb != type_verb {
        return None;
    }
    let expected_land_type = if type_verb == "is" { "land" } else { "lands" };
    if !card_type_tail.eq_ignore_ascii_case(expected_land_type) {
        return None;
    }

    let subtype_tail = subtype_tail
        .strip_prefix("an ")
        .or_else(|| subtype_tail.strip_prefix("a "))
        .unwrap_or(subtype_tail);
    let mut consumed = 2usize;
    let mut descriptor = subtype_tail.to_string();
    if let Some(color_ability) = rest.first()
        && let Some(color_display) = static_display_with_id(
            color_ability,
            crate::static_abilities::StaticAbilityId::MakeColorless,
        )
        && let Some((color_subject, color_tail, color_verb)) =
            split_static_predicate_with_verb(&color_display, &[" is ", " are "])
        && color_subject == subject
        && color_verb == type_verb
        && color_tail.eq_ignore_ascii_case("colorless")
    {
        descriptor = format!("colorless {descriptor}");
        consumed += 1;
    }

    let type_phrase = format!("{descriptor} {expected_land_type}");
    let type_phrase = if type_verb == "is" {
        with_indefinite_article(&type_phrase)
    } else {
        type_phrase
    };
    Some((format!("{subject} {type_verb} {type_phrase}"), consumed))
}

fn static_subject_is_land_kind(subject: &str) -> bool {
    let lower = subject.trim().to_ascii_lowercase();
    let lower = lower
        .strip_prefix("all ")
        .or_else(|| lower.strip_prefix("other "))
        .unwrap_or(lower.as_str());
    matches!(
        lower,
        "lands"
            | "forests"
            | "islands"
            | "mountains"
            | "plains"
            | "swamps"
            | "deserts"
            | "gates"
            | "lairs"
            | "locuses"
            | "loci"
            | "mines"
            | "power-plants"
            | "towers"
            | "urza's lands"
    )
}

fn describe_structural_type_base_pt_addition_bundle(
    abilities: &[Ability],
) -> Option<(String, usize)> {
    let [card_type_ability, pt_ability, ..] = abilities else {
        return None;
    };
    let card_type_display = static_display_with_id(
        card_type_ability,
        crate::static_abilities::StaticAbilityId::AddCardTypes,
    )?;
    let pt_display = static_display_with_id(
        pt_ability,
        crate::static_abilities::StaticAbilityId::SetBasePowerToughnessForFilter,
    )?;

    let (subject, card_type_tail, type_verb) =
        split_static_predicate_with_verb(&card_type_display, &[" are ", " is "])?;
    let (pt_subject, pt_tail, pt_verb) =
        split_static_predicate_with_verb(&pt_display, &[" have ", " has "])?;
    if pt_subject != subject {
        return None;
    }

    let (type_text, other_types) = card_type_tail
        .strip_suffix(" in addition to their other types")
        .map(|text| (text, "their"))
        .or_else(|| {
            card_type_tail
                .strip_suffix(" in addition to its other types")
                .map(|text| (text, "its"))
        })?;
    let base_prefix = if pt_tail.starts_with("base power and base toughness ") {
        "base power and base toughness "
    } else if pt_tail.starts_with("base power and toughness ") {
        "base power and toughness "
    } else {
        return None;
    };
    let pt_text = pt_tail.strip_prefix(base_prefix)?;
    if static_subject_is_land_kind(subject) {
        let still_land_tail = if type_verb == "is" {
            "that's still a land"
        } else {
            "that are still lands"
        };
        return Some((
            format!("{subject} {type_verb} {pt_text} {type_text} {still_land_tail}"),
            2,
        ));
    }
    Some((
        format!(
            "{subject} {type_verb} {type_text} in addition to {other_types} other types and {pt_verb} base power and base toughness {pt_text}"
        ),
        2,
    ))
}

fn describe_structural_station_keyword(ability: &Ability) -> Option<&'static str> {
    let AbilityKind::Activated(activated) = &ability.kind else {
        return None;
    };
    if !matches!(activated.timing, ActivationTiming::SorcerySpeed)
        || activated.activation_condition.is_some()
        || !activated.choices.is_empty()
        || !activated.mana_usage_restrictions.is_empty()
    {
        return None;
    }
    if !activated
        .additional_restrictions
        .iter()
        .any(|restriction| restriction.eq_ignore_ascii_case("Activate only as a sorcery"))
    {
        return None;
    }

    let costs = activated.mana_cost.costs();
    if !costs.iter().any(station_cost_chooses_tap_cost_creature)
        || !costs.iter().any(station_cost_taps_chosen_creature)
    {
        return None;
    }

    let [effect] = activated.effects.flattened_default_effects() else {
        return None;
    };
    let put = effect.downcast_ref::<crate::effects::PutCountersEffect>()?;
    if put.counter_type != crate::CounterType::Charge
        || !matches!(put.target.unhinted(), ChooseSpec::Source)
    {
        return None;
    }
    let Value::PowerOf(source) = put.amount.unhinted() else {
        return None;
    };
    match source.unhinted() {
        ChooseSpec::Tagged(tag) if tag.as_str() == "tap_cost_0" => Some("Station"),
        _ => None,
    }
}

fn station_cost_chooses_tap_cost_creature(cost: &crate::costs::Cost) -> bool {
    let Some(effect) = cost.effect_ref() else {
        return false;
    };
    let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() else {
        return false;
    };
    choose.tag.as_str() == "tap_cost_0"
        && choose.filter.controller == Some(PlayerFilter::You)
        && choose.filter.card_types.contains(&CardType::Creature)
        && choose.filter.other
        && choose.filter.untapped
}

fn station_cost_taps_chosen_creature(cost: &crate::costs::Cost) -> bool {
    let Some(effect) = cost.effect_ref() else {
        return false;
    };
    let Some(tap) = effect.downcast_ref::<crate::effects::TapEffect>() else {
        return false;
    };
    matches!(tap.target.unhinted(), ChooseSpec::Tagged(tag) if tag.as_str() == "tap_cost_0")
}

fn rewrite_additional_sacrifice_reference_surface(def: &CardDefinition, text: &str) -> String {
    if !has_additional_creature_sacrifice_cost(def) {
        return text.to_string();
    }

    let mut normalized = text
        .replace(
            "You draw X cards, where X is that creature's power",
            "Draw cards equal to the sacrificed creature's power",
        )
        .replace(
            "you draw X cards, where X is that creature's power",
            "draw cards equal to the sacrificed creature's power",
        )
        .replace("that creature's power", "the sacrificed creature's power")
        .replace(
            "that creature's toughness",
            "the sacrificed creature's toughness",
        )
        .replace(
            "that creature's mana value",
            "the sacrificed creature's mana value",
        )
        .replace(
            "the total power of those creatures",
            "the total power of the sacrificed creatures",
        )
        .replace(
            "the total toughness of those creatures",
            "the total toughness of the sacrificed creatures",
        );

    if !normalized.contains("sacrificed creature's") {
        normalized = normalized
            .replace("its power", "the sacrificed creature's power")
            .replace("its toughness", "the sacrificed creature's toughness")
            .replace("its mana value", "the sacrificed creature's mana value");
    }

    normalized
}

fn has_additional_creature_sacrifice_cost(def: &CardDefinition) -> bool {
    def.additional_non_mana_costs().iter().any(|cost| {
        let Some(effect) = cost.effect_ref() else {
            return false;
        };
        let effect = effect
            .downcast_ref::<crate::effects::WithIdEffect>()
            .map(|with_id| with_id.effect.as_ref())
            .unwrap_or(effect);
        if effect
            .downcast_ref::<crate::effects::ChooseObjectsEffect>()
            .is_some_and(|choose| {
                choose.tag.as_str().starts_with("sacrificed_")
                    && choose.filter.card_types.contains(&CardType::Creature)
            })
        {
            return true;
        }
        effect
            .downcast_ref::<crate::effects::zones::SacrificePlayerEffect>()
            .is_some_and(|sacrifice| sacrifice.filter.card_types.contains(&CardType::Creature))
    })
}

fn ability_is_this_spell_cost_modifier(ability: &Ability) -> bool {
    let AbilityKind::Static(static_ability) = &ability.kind else {
        return false;
    };
    static_ability.this_spell_cost_reduction().is_some()
        || static_ability
            .this_spell_cost_reduction_mana_cost()
            .is_some()
}

fn ability_precedes_spell_resolution(ability: &Ability) -> bool {
    if ability_is_this_spell_cost_modifier(ability) {
        return true;
    }
    matches!(
        &ability.kind,
        AbilityKind::Static(static_ability)
            if static_ability.id()
                == crate::static_abilities::StaticAbilityId::MakeColorless
    )
}

fn is_choose_background_spell_effect(spell_effects: &crate::ResolutionProgram) -> bool {
    let [effect] = spell_effects.flattened_default_effects() else {
        return false;
    };
    let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() else {
        return false;
    };

    choose.filter.zone == Some(Zone::Battlefield)
        && matches!(choose.filter.controller, None | Some(PlayerFilter::You))
        && choose.filter.card_types.is_empty()
        && choose.filter.subtypes == [Subtype::Background]
        && choose.filter.supertypes.is_empty()
        && choose.count == ChoiceCount::exactly(1)
        && choose.chooser == PlayerFilter::You
        && choose.zone == Some(Zone::Battlefield)
        && choose.additional_zones.is_empty()
        && !choose.is_search
        && !choose.reveal
        && !choose.top_only
        && !choose.replace_tagged_objects
}

#[cfg(test)]
mod self_replacement_rendering_tests {
    use super::*;

    fn damage_each_creature_and_player(amount: i32) -> Vec<Effect> {
        vec![
            Effect::for_each(
                ObjectFilter::creature(),
                vec![Effect::deal_damage(
                    Value::Fixed(amount),
                    ChooseSpec::Iterated,
                )],
            ),
            Effect::new(crate::effects::ForPlayersEffect::new(
                PlayerFilter::Any,
                vec![Effect::deal_damage(
                    Value::Fixed(amount),
                    ChooseSpec::Player(PlayerFilter::IteratedPlayer),
                )],
            )),
        ]
    }

    fn labeled_branch(
        label: &str,
        condition_after_replacement: bool,
    ) -> crate::resolution::SelfReplacementBranch {
        let mut branch = crate::resolution::SelfReplacementBranch::new(
            Condition::PlayerHasCardTypesInGraveyardOrMore {
                player: PlayerFilter::You,
                count: 4,
            },
            Vec::new(),
        )
        .with_presentation_label(Some(
            crate::ability::PresentationLabel::from_ability_word(label),
        ));
        branch.condition_after_replacement = condition_after_replacement;
        branch
    }

    #[test]
    fn conjoins_same_source_damage_in_default_and_replacement_branches() {
        assert_eq!(
            describe_conjoined_same_source_damage(&damage_each_creature_and_player(1)).as_deref(),
            Some("Deal 1 damage to each creature and each player")
        );

        let segment = crate::resolution::ResolutionSegment {
            default_effects: damage_each_creature_and_player(1),
            self_replacements: vec![crate::resolution::SelfReplacementBranch::new(
                Condition::PlayerControls {
                    player: PlayerFilter::You,
                    filter: ObjectFilter::default()
                        .you_control()
                        .with_supertype(Supertype::Snow)
                        .with_subtype(Subtype::Swamp),
                },
                damage_each_creature_and_player(2),
            )],
        };
        let rendered = describe_single_self_replacement_segment(&segment)
            .expect("same-source damage replacement should render");
        assert_eq!(
            rendered,
            "Deal 1 damage to each creature and each player. If you control a snow Swamp, deal 2 damage to each creature and each player instead"
        );
        assert_eq!(
            rendered
                .matches("damage to each creature and each player")
                .count(),
            2,
            "{rendered}"
        );
        assert_eq!(rendered.matches(" instead").count(), 1, "{rendered}");
    }

    #[test]
    fn conjoins_coordinated_mass_damage_in_self_replacement_branches() {
        let coordinated = |amount| {
            vec![Effect::new(crate::effects::SequenceEffect::coordinated(
                damage_each_creature_and_player(amount),
            ))]
        };
        let segment = crate::resolution::ResolutionSegment {
            default_effects: coordinated(1),
            self_replacements: vec![crate::resolution::SelfReplacementBranch::new(
                Condition::PlayerControls {
                    player: PlayerFilter::You,
                    filter: ObjectFilter::default()
                        .you_control()
                        .with_supertype(Supertype::Snow)
                        .with_subtype(Subtype::Swamp),
                },
                coordinated(2),
            )],
        };

        assert_eq!(
            describe_single_self_replacement_segment(&segment).as_deref(),
            Some(
                "Deal 1 damage to each creature and each player. If you control a snow Swamp, deal 2 damage to each creature and each player instead"
            )
        );
    }

    #[test]
    fn repeated_mass_damage_target_survives_amount_replacement() {
        let damage_each_creature = |amount| {
            vec![Effect::for_each(
                ObjectFilter::creature(),
                vec![Effect::deal_damage(
                    Value::Fixed(amount),
                    ChooseSpec::Iterated,
                )],
            )]
        };
        let segment = crate::resolution::ResolutionSegment {
            default_effects: damage_each_creature(1),
            self_replacements: vec![crate::resolution::SelfReplacementBranch::new(
                Condition::ThisSpellWasKicked,
                damage_each_creature(2),
            )],
        };

        assert_eq!(
            describe_single_self_replacement_segment(&segment).as_deref(),
            Some(
                "Deal 1 damage to each creature. It deals 2 damage to each creature instead if this was kicked"
            )
        );
    }

    #[test]
    fn tagged_multi_target_destroy_replacement_keeps_each_creature_referent() {
        let damaged = TagKey::from("damaged_0");
        let default = Effect::deal_damage(
            Value::Fixed(2),
            ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()))
                .with_count(ChoiceCount::exactly(2)),
        )
        .tag(damaged.clone());
        let destroy_filter = ObjectFilter::creature().match_tagged(
            damaged,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject,
        );
        let replacement = Effect::destroy(ChooseSpec::Object(destroy_filter)).tag("destroyed_0");
        let segment = crate::resolution::ResolutionSegment {
            default_effects: vec![default],
            self_replacements: vec![crate::resolution::SelfReplacementBranch::new(
                Condition::Not(Box::new(Condition::CardsInHandOrMore(1))),
                vec![replacement],
            )],
        };

        assert_eq!(
            describe_single_self_replacement_segment(&segment).as_deref(),
            Some(
                "Deal 2 damage to two target creatures. Destroy each creature instead if you have no cards in hand"
            )
        );
    }

    #[test]
    fn preserves_multi_sentence_replacement_boundaries() {
        let rendered = format_self_replacement_fallback(
            "Destroy all enchantments",
            "there are seven or more cards in your graveyard",
            "destroy all enchantments. Return all cards destroyed this way to the battlefield",
        );
        assert_eq!(
            rendered,
            "Destroy all enchantments. If there are seven or more cards in your graveyard, instead destroy all enchantments. Return all cards destroyed this way to the battlefield"
        );
        assert!(!rendered.contains("and return"));
    }

    #[test]
    fn labeled_replacement_preserves_leading_condition_surface() {
        let rendered = apply_self_replacement_presentation_label(
            &labeled_branch("Landfall", false),
            "Default action. If a land entered this turn, replacement action instead".to_string(),
        );
        assert_eq!(
            rendered,
            "Default action. Landfall — If a land entered this turn, replacement action instead"
        );
    }

    #[test]
    fn labeled_replacement_preserves_trailing_condition_surface() {
        let rendered = apply_self_replacement_presentation_label(
            &labeled_branch("Metalcraft", true),
            "Default action. If you control three artifacts, that player sacrifices two creatures instead"
                .to_string(),
        );
        assert_eq!(
            rendered,
            "Default action. Metalcraft — That player sacrifices two creatures instead if you control three artifacts"
        );

        let damage = apply_self_replacement_presentation_label(
            &labeled_branch("Morbid", true),
            "Deal 3 damage to any target. It deals 5 damage instead if a creature died this turn"
                .to_string(),
        );
        assert_eq!(
            damage,
            "Deal 3 damage to any target. Morbid — It deals 5 damage instead if a creature died this turn"
        );
    }
}

#[cfg(test)]
mod keyword_surface_merge_tests {
    use super::*;

    #[test]
    fn parameterized_keyword_marker_stays_separate_from_intrinsic_keyword() {
        let merged = merge_adjacent_keyword_surface_lines(vec![
            "Static ability 0: Mutate {4}{B}".to_string(),
            "Static ability 1: flying".to_string(),
        ]);

        assert_eq!(
            merged,
            vec![
                "Static ability 0: Mutate {4}{B}".to_string(),
                "Static ability 1: flying".to_string(),
            ]
        );
    }

    #[test]
    fn adjacent_singular_protection_grants_keep_has_and_compact_colors() {
        let merged = merge_adjacent_keyword_surface_lines(vec![
            "Static ability 0: Equipped creature has protection from green".to_string(),
            "Static ability 1: Equipped creature has protection from blue".to_string(),
        ]);

        assert_eq!(
            merged,
            vec![
                "Static ability 0: Equipped creature has protection from green and from blue"
                    .to_string()
            ]
        );
    }
}

#[cfg(test)]
mod reinforce_structural_render_tests {
    use super::*;

    fn mana_cost(symbols: Vec<ManaSymbol>) -> crate::mana::ManaCost {
        crate::mana::ManaCost::from_symbols(symbols)
    }

    fn reinforce_ability(amount: i32, mana: crate::mana::ManaCost) -> Ability {
        let costs = crate::cost::TotalCost::from_costs(vec![
            crate::costs::Cost::mana(mana),
            crate::costs::Cost::discard_source(),
        ]);
        let target = ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::creature().in_zone(Zone::Battlefield),
        ));
        let effect =
            Effect::put_counters(crate::object::CounterType::PlusOnePlusOne, amount, target);
        let mut ability = Ability::activated(costs, vec![effect]);
        ability.functional_zones = vec![Zone::Hand];
        ability
    }

    #[test]
    fn exact_hand_activation_renders_generic_reinforce_amount() {
        let one = reinforce_ability(1, mana_cost(vec![ManaSymbol::Generic(1), ManaSymbol::Red]));
        let three = reinforce_ability(
            3,
            mana_cost(vec![ManaSymbol::Generic(3), ManaSymbol::Green]),
        );
        assert_eq!(
            describe_structural_reinforce_ability(&one).as_deref(),
            Some("Reinforce 1—{1}{R}"),
        );
        assert_eq!(
            describe_structural_reinforce_ability(&three).as_deref(),
            Some("Reinforce 3—{3}{G}"),
        );

        let definition = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Reinforce Surface Probe",
        )
        .card_types(vec![CardType::Creature])
        .with_ability(Ability::static_ability(
            crate::static_abilities::StaticAbility::flying(),
        ))
        .with_ability(three)
        .build();
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition),
            vec!["Flying".to_string(), "Reinforce 3—{3}{G}".to_string()],
        );
    }

    #[test]
    fn near_miss_zone_cost_metadata_and_effect_shapes_stay_explicit() {
        let base = reinforce_ability(
            2,
            mana_cost(vec![ManaSymbol::Generic(2), ManaSymbol::White]),
        );

        let mut wrong_zone = base.clone();
        wrong_zone.functional_zones = vec![Zone::Battlefield];
        assert_eq!(describe_structural_reinforce_ability(&wrong_zone), None);

        let mut extra_cost = base.clone();
        let AbilityKind::Activated(extra_cost_activated) = &mut extra_cost.kind else {
            unreachable!();
        };
        extra_cost_activated.mana_cost = crate::cost::TotalCost::from_costs(vec![
            crate::costs::Cost::mana(mana_cost(vec![ManaSymbol::Generic(2), ManaSymbol::White])),
            crate::costs::Cost::discard_source(),
            crate::costs::Cost::life(1),
        ]);
        assert_eq!(describe_structural_reinforce_ability(&extra_cost), None);

        let mut extra_choice = base.clone();
        let AbilityKind::Activated(extra_choice_activated) = &mut extra_choice.kind else {
            unreachable!();
        };
        extra_choice_activated
            .choices
            .push(ChooseSpec::target_creature());
        assert_eq!(describe_structural_reinforce_ability(&extra_choice), None);

        let mut extra_effect = base.clone();
        let AbilityKind::Activated(extra_effect_activated) = &mut extra_effect.kind else {
            unreachable!();
        };
        extra_effect_activated.effects.push(Effect::draw(1));
        assert_eq!(describe_structural_reinforce_ability(&extra_effect), None);

        let mut wrong_target = base.clone();
        let AbilityKind::Activated(wrong_target_activated) = &mut wrong_target.kind else {
            unreachable!();
        };
        wrong_target_activated.effects =
            crate::resolution::ResolutionProgram::from_effects(vec![Effect::put_counters(
                crate::object::CounterType::PlusOnePlusOne,
                2,
                ChooseSpec::target(ChooseSpec::Object(ObjectFilter::artifact())),
            )]);
        assert_eq!(describe_structural_reinforce_ability(&wrong_target), None);

        let mut dynamic_amount = base;
        let AbilityKind::Activated(dynamic_activated) = &mut dynamic_amount.kind else {
            unreachable!();
        };
        dynamic_activated.effects =
            crate::resolution::ResolutionProgram::from_effects(vec![Effect::put_counters(
                crate::object::CounterType::PlusOnePlusOne,
                Value::CardsInHand(PlayerFilter::You),
                ChooseSpec::target(ChooseSpec::Object(
                    ObjectFilter::creature().in_zone(Zone::Battlefield),
                )),
            )]);
        assert_eq!(describe_structural_reinforce_ability(&dynamic_amount), None,);
    }
}

#[cfg(test)]
mod attached_anthem_bundle_tests {
    use super::*;

    fn enchanted_creature_filter() -> ObjectFilter {
        ObjectFilter::creature()
            .in_zone(Zone::Battlefield)
            .match_tagged(
                "enchanted",
                crate::target::TaggedOpbjectRelation::IsTaggedObject,
            )
    }

    #[test]
    fn fresh_start_and_mystic_subdual_static_pieces_recombine() {
        for (power, expected) in [
            (-5, "Enchanted creature gets -5/-0 and loses all abilities"),
            (-2, "Enchanted creature gets -2/-0 and loses all abilities"),
        ] {
            let filter = enchanted_creature_filter();
            let abilities = vec![
                Ability::static_ability(crate::static_abilities::StaticAbility::anthem(
                    filter.clone(),
                    power,
                    0,
                )),
                Ability::static_ability(
                    crate::static_abilities::StaticAbility::remove_all_abilities(filter),
                ),
            ];

            assert_eq!(
                describe_structural_anthem_remove_all_abilities_bundle(&abilities),
                Some((expected.to_string(), 2)),
            );
        }
    }
}

#[cfg(test)]
mod conditional_anthem_otherwise_bundle_tests {
    use super::*;

    fn enchanted_creature_filter() -> ObjectFilter {
        ObjectFilter::creature().match_tagged(
            "enchanted",
            crate::target::TaggedOpbjectRelation::IsTaggedObject,
        )
    }

    fn modeled_anthem(payload: ironsmith_core::Anthem) -> Ability {
        let model: crate::static_abilities::CompiledStaticAbility =
            ironsmith_core::StaticAbility::new(payload);
        Ability::static_ability(crate::static_abilities::StaticAbility::from_model(model))
    }

    fn otherwise_pair(
        condition_filter: ObjectFilter,
        positive: (i32, i32),
        negative: (i32, i32),
    ) -> Vec<Ability> {
        let condition = crate::ConditionExpr::AttachedToSourceMatches(condition_filter);
        vec![
            modeled_anthem(
                ironsmith_core::Anthem::new(enchanted_creature_filter(), positive.0, positive.1)
                    .with_condition(condition.clone()),
            ),
            modeled_anthem(
                ironsmith_core::Anthem::new(enchanted_creature_filter(), negative.0, negative.1)
                    .with_condition(crate::ConditionExpr::Not(Box::new(condition))),
            ),
        ]
    }

    #[test]
    fn adjacent_inverse_fixed_anthems_render_as_otherwise() {
        let mut attacking = ObjectFilter::default();
        attacking.attacking = true;
        for (condition, positive, negative, expected) in [
            (
                ObjectFilter::default().with_subtype(Subtype::Zombie),
                (3, 3),
                (-3, -3),
                "Enchanted creature gets +3/+3 as long as it's a Zombie. Otherwise, it gets -3/-3",
            ),
            (
                ObjectFilter::enchantment(),
                (2, 2),
                (-2, -2),
                "Enchanted creature gets +2/+2 as long as it's an enchantment. Otherwise, it gets -2/-2",
            ),
            (
                ObjectFilter::default().with_colors(crate::color::ColorSet::BLACK),
                (2, 1),
                (-1, -2),
                "Enchanted creature gets +2/+1 as long as it's black. Otherwise, it gets -1/-2",
            ),
            (
                attacking,
                (3, 0),
                (-2, -1),
                "Enchanted creature gets +3/+0 as long as it's attacking. Otherwise, it gets -2/-1",
            ),
        ] {
            let abilities = otherwise_pair(condition, positive, negative);
            assert_eq!(
                describe_structural_conditional_anthem_otherwise_bundle(&abilities),
                Some((expected.to_string(), 2)),
            );
        }

        let pirate = otherwise_pair(
            ObjectFilter::default().with_subtype(Subtype::Pirate),
            (0, 2),
            (-2, 0),
        );
        let mut builder = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Otherwise Surface Probe",
        )
        .card_types(vec![CardType::Enchantment]);
        for ability in pirate {
            builder = builder.with_ability(ability);
        }
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&builder.build()),
            vec![
                "Enchanted creature gets +0/+2 as long as it's a Pirate. Otherwise, it gets -2/-0."
                    .to_string()
            ],
        );
    }

    #[test]
    fn target_predicate_and_fixed_payload_near_misses_do_not_merge() {
        let zombie = ObjectFilter::default().with_subtype(Subtype::Zombie);

        let mut different_target = otherwise_pair(zombie.clone(), (3, 3), (-3, -3));
        let equipped_target = ObjectFilter::creature().match_tagged(
            "equipped",
            crate::target::TaggedOpbjectRelation::IsTaggedObject,
        );
        different_target[1] = modeled_anthem(
            ironsmith_core::Anthem::new(equipped_target, -3, -3).with_condition(
                crate::ConditionExpr::Not(Box::new(crate::ConditionExpr::AttachedToSourceMatches(
                    zombie.clone(),
                ))),
            ),
        );
        assert_eq!(
            describe_structural_conditional_anthem_otherwise_bundle(&different_target),
            None,
        );

        let mut noninverse = otherwise_pair(zombie.clone(), (3, 3), (-3, -3));
        noninverse[1] = modeled_anthem(
            ironsmith_core::Anthem::new(enchanted_creature_filter(), -3, -3).with_condition(
                crate::ConditionExpr::Not(Box::new(crate::ConditionExpr::AttachedToSourceMatches(
                    ObjectFilter::default().with_subtype(Subtype::Vampire),
                ))),
            ),
        );
        assert_eq!(
            describe_structural_conditional_anthem_otherwise_bundle(&noninverse),
            None,
        );

        let mut dynamic = otherwise_pair(zombie.clone(), (3, 3), (-3, -3));
        dynamic[1] = modeled_anthem(
            ironsmith_core::Anthem::new(enchanted_creature_filter(), -3, -3)
                .with_values(
                    ironsmith_core::AnthemValue::Dynamic(Value::Fixed(-3)),
                    ironsmith_core::AnthemValue::Fixed(-3),
                )
                .with_condition(crate::ConditionExpr::Not(Box::new(
                    crate::ConditionExpr::AttachedToSourceMatches(zombie),
                ))),
        );
        assert_eq!(
            describe_structural_conditional_anthem_otherwise_bundle(&dynamic),
            None,
        );

        let mut non_pt = otherwise_pair(
            ObjectFilter::default().with_subtype(Subtype::Pirate),
            (0, 2),
            (-2, 0),
        );
        non_pt[1] = Ability::static_ability(crate::static_abilities::StaticAbility::flying());
        assert_eq!(
            describe_structural_conditional_anthem_otherwise_bundle(&non_pt),
            None,
        );
    }
}

#[cfg(test)]
mod enchanted_restriction_bundle_tests {
    use super::*;

    fn enchanted_filter(subject: &str) -> ObjectFilter {
        let filter = match subject {
            "Enchanted creature" => ObjectFilter::creature(),
            "Enchanted permanent" => ObjectFilter::permanent_card().in_zone(Zone::Battlefield),
            "Enchanted artifact" => ObjectFilter::artifact(),
            other => panic!("unsupported restriction test subject: {other}"),
        };
        filter.match_tagged(
            "enchanted",
            crate::target::TaggedOpbjectRelation::IsTaggedObject,
        )
    }

    fn modeled_rule_static(
        restriction: crate::effect::Restriction,
        display: impl Into<String>,
    ) -> crate::static_abilities::StaticAbility {
        let display = display.into();
        let model: crate::static_abilities::CompiledStaticAbility =
            ironsmith_core::StaticAbility::restriction(restriction, display);
        crate::static_abilities::StaticAbility::from_model(model)
    }

    fn modeled_rule_ability(
        restriction: crate::effect::Restriction,
        display: impl Into<String>,
    ) -> Ability {
        Ability::static_ability(modeled_rule_static(restriction, display))
    }

    fn restriction_pair(subject: &str) -> Vec<Ability> {
        let filter = enchanted_filter(subject);
        let subject = subject.to_ascii_lowercase();
        vec![
            modeled_rule_ability(
                crate::effect::Restriction::attack_or_block(filter.clone()),
                format!("{subject} can't attack or block"),
            ),
            modeled_rule_ability(
                crate::effect::Restriction::activate_abilities_of(filter),
                format!("{subject} activated abilities can't be activated"),
            ),
        ]
    }

    #[test]
    fn exact_adjacent_creature_permanent_and_artifact_restrictions_recombine() {
        for subject in [
            "Enchanted creature",
            "Enchanted permanent",
            "Enchanted artifact",
        ] {
            let expected = format!(
                "{subject} can't attack or block, and its activated abilities can't be activated"
            );
            let abilities = restriction_pair(subject);
            assert_eq!(
                describe_structural_enchanted_combat_activation_restriction_bundle(&abilities),
                Some((expected.clone(), 2)),
            );

            let mut builder = crate::cards::builders::CardDefinitionBuilder::new(
                crate::ids::CardId::new(),
                "Restriction Surface Probe",
            )
            .card_types(vec![CardType::Enchantment]);
            for ability in abilities {
                builder = builder.with_ability(ability);
            }
            let definition = builder.build();
            assert_eq!(
                crate::compiled_text::compiled_text_lines(&definition),
                vec![format!("{expected}.")],
            );
        }
    }

    #[test]
    fn nonadjacent_or_different_subject_restrictions_stay_separate() {
        let mut nonadjacent = restriction_pair("Enchanted creature");
        nonadjacent.insert(
            1,
            Ability::static_ability(crate::static_abilities::StaticAbility::flying()),
        );
        assert_eq!(
            describe_structural_enchanted_combat_activation_restriction_bundle(&nonadjacent),
            None,
        );

        let mut different_subject = restriction_pair("Enchanted creature");
        let permanent_filter = enchanted_filter("Enchanted permanent");
        different_subject[1] = modeled_rule_ability(
            crate::effect::Restriction::activate_abilities_of(permanent_filter),
            "enchanted permanent activated abilities can't be activated",
        );
        assert_eq!(
            describe_structural_enchanted_combat_activation_restriction_bundle(&different_subject,),
            None,
        );
    }

    #[test]
    fn conditioned_or_mana_exception_restrictions_stay_separate() {
        let mut conditioned = restriction_pair("Enchanted creature");
        conditioned[1] = Ability::static_ability(
            modeled_rule_static(
                crate::effect::Restriction::activate_abilities_of(enchanted_filter(
                    "Enchanted creature",
                )),
                "enchanted creature activated abilities can't be activated",
            )
            .with_condition(crate::ConditionExpr::YourTurn)
            .expect("rule restriction supports a temporal condition"),
        );
        assert_eq!(
            describe_structural_enchanted_combat_activation_restriction_bundle(&conditioned),
            None,
        );

        let mut mana_exception = restriction_pair("Enchanted permanent");
        mana_exception[1] = modeled_rule_ability(
            crate::effect::Restriction::activate_non_mana_abilities_of(enchanted_filter(
                "Enchanted permanent",
            )),
            "enchanted permanent activated abilities can't be activated unless they're mana abilities",
        );
        assert_eq!(
            describe_structural_enchanted_combat_activation_restriction_bundle(&mana_exception),
            None,
        );
    }
}

#[cfg(test)]
mod creature_aura_subject_tests {
    use super::*;

    #[test]
    fn plain_creature_attachment_refines_enchanted_permanent_references() {
        let creature = crate::object::AuraAttachmentFilter::Object(
            ObjectFilter::creature().in_zone(Zone::Battlefield),
        );
        assert_eq!(
            rewrite_plain_creature_aura_subject(
                Some(&creature),
                "At the beginning of the upkeep of enchanted permanent's controller"
            ),
            "At the beginning of the upkeep of enchanted creature's controller"
        );

        let land = crate::object::AuraAttachmentFilter::Object(
            ObjectFilter::land().in_zone(Zone::Battlefield),
        );
        assert_eq!(
            rewrite_plain_creature_aura_subject(
                Some(&land),
                "At the beginning of the upkeep of enchanted permanent's controller"
            ),
            "At the beginning of the upkeep of enchanted permanent's controller"
        );
    }
}

#[cfg(test)]
mod prototype_surface_tests {
    use super::*;

    #[test]
    fn prototype_frame_line_keeps_separator_and_precedes_other_abilities() {
        let definition = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Prototype Surface Probe",
        )
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .parse_text("Prototype {2}{R} — 3/2\nHaste")
        .expect("Prototype surface probe should parse");

        let lines = compiled_lines_inner(&definition);
        assert_eq!(
            lines.first().map(String::as_str),
            Some("Prototype {2}{R} — 3/2")
        );
        assert!(
            lines.iter().skip(1).any(|line| line.ends_with("Haste")),
            "non-Prototype abilities should follow the Prototype frame line: {lines:?}"
        );
    }
}
