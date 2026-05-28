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
        .map(|line| rewrite_source_target_restriction_subject(def, &line))
        .map(RawRenderedLine)
        .collect()
}

pub(super) fn rewrite_source_target_restriction_subject(
    def: &CardDefinition,
    line: &str,
) -> String {
    if def.card.name.trim().is_empty() {
        return line.to_string();
    }

    for prefix in [
        "This creature can't be the target of ",
        "This permanent can't be the target of ",
        "This can't be the target of ",
    ] {
        if let Some(rest) = line.strip_prefix(prefix) {
            return format!("{} can't be the target of {rest}", def.card.name);
        }
    }

    line.to_string()
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
    let source_phrase = format!("Exile this {}", primary_type.name().to_ascii_lowercase());
    if !line.contains(&source_phrase) || !line.contains(':') {
        return line.to_string();
    }
    line.replace(&source_phrase, &format!("Exile {}", def.card.name))
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

        if let Some((label, subject, keyword)) = split_static_granted_keyword_line(&lines[idx]) {
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
                let Some((_, next_subject, next_keyword)) = split_static_granted_keyword_line(next)
                else {
                    break;
                };
                if next_subject != subject {
                    break;
                }
                keywords.push(next_keyword.to_string());
                consumed += 1;
            }
            if consumed > 1 {
                out.push(format!(
                    "{label}{subject} have {}",
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

fn split_static_granted_keyword_line(line: &str) -> Option<(&str, &str, &str)> {
    let (label, text) = line.split_once(": ")?;
    if !label.starts_with("Static ability ") {
        return None;
    }
    let keyword = text.trim_end_matches('.');
    let (subject, keyword) = keyword.split_once(" have ")?;
    if !is_mergeable_keyword_surface(keyword) {
        return None;
    }
    Some((&line[..label.len() + 2], subject, keyword))
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

fn is_mergeable_keyword_surface(keyword: &str) -> bool {
    let keyword = keyword.trim_end_matches('.');
    let lower = keyword.to_ascii_lowercase();
    (is_keyword_phrase(keyword) && lower != "changeling")
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
            rendered_segments.push(rendered);
            continue;
        }

        if !segment.default_effects.is_empty() {
            rendered_segments.push(
                describe_effect_clause_list(&segment.default_effects)
                    .map(|text| capitalize_first(&text))
                    .unwrap_or_else(|| describe_effect_list(&segment.default_effects)),
            );
        }
        for branch in &segment.self_replacements {
            rendered_segments.push(describe_effect_list(&branch.replacement_effects));
        }
    }
    rendered_segments.join(". ")
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
        Some(Condition::ThisSpellPaidLabel(label)) if label == "Gift"
    ) || !triggered.choices.is_empty()
        || !trigger_is_this_enters_battlefield(&triggered.trigger)
    {
        return false;
    }
    let rendered = describe_resolution_program(&triggered.effects).to_ascii_lowercase();
    rendered.starts_with("if the gift was promised") || is_standard_gift_render_payload(&rendered)
}

fn describe_single_self_replacement_segment(
    segment: &crate::resolution::ResolutionSegment,
) -> Option<String> {
    if segment.self_replacements.len() != 1 || segment.default_effects.is_empty() {
        return None;
    }
    let branch = &segment.self_replacements[0];
    let conditional = Effect::conditional(
        branch.condition.clone(),
        branch.replacement_effects.clone(),
        segment.default_effects.clone(),
    );
    let conditional_text = describe_effect_list(&[conditional]);
    if conditional_text.contains(" damage instead if ") {
        return Some(conditional_text);
    }
    let default_text = describe_effect_list(&segment.default_effects);
    let replacement_text = describe_effect_list(&branch.replacement_effects);
    let condition_text = super::normalize_common::describe_condition(&branch.condition);
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
    if let Some(damage_text) =
        describe_rendered_damage_self_replacement(&default_text, &replacement_text, &condition_text)
    {
        return Some(damage_text);
    }
    Some(format!(
        "{default_text}. If {condition_text}, {} instead",
        rewrite_self_replacement_referent_phrase(&default_text, &replacement_text)
    ))
}

fn rewrite_self_replacement_referent_phrase(default_text: &str, replacement_text: &str) -> String {
    let mut replacement = super::normalize_common::lowercase_first(replacement_text);
    if default_text
        .to_ascii_lowercase()
        .contains("target creature")
        && replacement.starts_with("target creature ")
    {
        replacement = replacement.replacen("target creature", "that creature", 1);
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
    Some(format!(
        "Deal {base_amount} damage to {base_target}. It deals {replacement_amount} damage instead if {condition_text}"
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
        .any(|cost| cost.label.trim().to_ascii_lowercase().starts_with("gift "));
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
    rewrite_damage_phrases_for_permanent_abilities(rendered, &def.card.name, false)
        .replace("Exile this source", &format!("Exile {}", def.card.name))
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
    let capitalized = capitalize_first_ascii(subject);
    line.replace(subject, card_name)
        .replace(&capitalized, card_name)
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
                if label == "Conspire" || label.starts_with("Conspire ")
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
    matches!(copy.target, ChooseSpec::Source)
        && copy.count == Value::Fixed(1)
        && copy.copier == PlayerFilter::You
        && retarget.from_effect == with_id.id
        && retarget.may
}

fn is_suspend_remove_time_counter_trigger(triggered: &crate::ability::TriggeredAbility) -> bool {
    if !matches!(
        triggered.intervening_if,
        Some(Condition::SourceHasCounterAtLeast {
            counter_type: CounterType::Time,
            count: 1,
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
            Condition::PlayerControlsAtLeast {
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
                .map(|cost| {
                    format!(
                        "Surge {} (You may cast this spell for its surge cost if you or a teammate has cast another spell this turn.)",
                        cost.to_oracle()
                    )
                })
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
            let mut line = format!("You may {clause} rather than pay this spell's mana cost");
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
        AlternativeCastingMethod::Madness { cost } => format!("Madness {}", cost.to_oracle()),
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
        AlternativeCastingMethod::JumpStart => "Jump-start".to_string(),
        AlternativeCastingMethod::Escape { cost, exile_count } => {
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
        let line = describe_alternative_cast_line(method, idx);
        if matches!(
            method,
            AlternativeCastingMethod::FlashWithAdditionalCost { .. }
        ) {
            leading_alternative_cast_lines.push(line);
        } else {
            alternative_cast_lines.push(line);
        }
    }
    out.extend(leading_alternative_cast_lines);
    for cost in &def.optional_costs {
        let line = describe_optional_cost_line(cost);
        if spell_like_card && cost.label == "Conspire" {
            deferred_spell_optional_lines.push(line);
        } else {
            out.push(line);
        }
    }
    let has_visible_gift_line = def
        .optional_costs
        .iter()
        .any(|cost| cost.label.trim().to_ascii_lowercase().starts_with("gift "));
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
    let push_abilities = |output: &mut Vec<String>| {
        let has_suspend = def
            .alternative_casts
            .iter()
            .any(|method| matches!(method, AlternativeCastingMethod::Suspend { .. }));
        let mut ability_idx = 0usize;
        while ability_idx < def.abilities.len() {
            let ability = &def.abilities[ability_idx];
            if has_suspend && is_suspend_helper_ability(ability) {
                ability_idx += 1;
                continue;
            }
            if is_conspire_helper_ability(ability) {
                ability_idx += 1;
                continue;
            }
            if has_visible_gift_line && is_hidden_gift_etb_ability(ability) {
                ability_idx += 1;
                continue;
            }
            if let Some(keyword) = describe_structural_ascend_ability(ability) {
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
            if let Some((keyword, consumed)) =
                describe_structural_counter_keyword_bundle(&def.abilities[ability_idx..])
            {
                output.push(format!("Keyword ability {}: {keyword}", ability_idx + 1));
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
            let mut ability_lines =
                describe_ability(ability_idx + 1, ability, subject, rewrite_it_deals);
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
            output.extend(ability_lines);
            ability_idx += 1;
        }
    };

    let additional_costs = def.additional_non_mana_costs();
    if !additional_costs.is_empty() {
        let additional_cost_text = describe_additional_costs(&additional_costs);
        let additional_cost_text = if additional_cost_text == "you may blight 1"
            || additional_cost_text.contains("put a -1/-1 counter on a creature you control")
        {
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
    merge_adjacent_keyword_surface_lines(out)
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
