use super::render_effects::{
    damage_with_source_view, describe_annihilator_keyword,
    describe_choose_color_reveal_hand_discard_that_color,
    describe_choose_name_target_mills_conditional_draw, describe_declined_may_mill_then_damage,
    describe_exile_then_free_cast_while_exiled_structural,
    describe_look_hand_choose_then_discard_or_exile, describe_repeated_die_parity_result_program,
    describe_reveal_hand_choose_discard_then_adventure_move,
    describe_reveal_hand_choose_graveyard_exile_bundle,
    describe_reveal_hand_choose_graveyard_or_hand_exile,
    describe_reveal_top_to_hand_then_lose_mana_value_effects,
    describe_roll_result_damage_then_random_source_attachment_program,
    describe_same_name_reference_search_bundle,
    describe_separated_countered_spell_exile_with_counters_gain_suspend,
    describe_sequenced_d20_numeric_result_table_program,
    describe_single_hand_reveal_same_name_search, describe_single_hand_reveal_setup,
    describe_target_player_draw_exile_then_copy_result,
    render_search_reveal_opponent_choose_rest_bundle,
};
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
        .map(|line| rewrite_source_owner_reference(def, &line))
        .map(RawRenderedLine)
        .collect()
}

fn rewrite_source_owner_reference(def: &CardDefinition, line: &str) -> String {
    let Some(primary_type) = def.card.card_types.first() else {
        return line.to_string();
    };
    line.replace(
        "this source's owner",
        &format!("this {}'s owner", primary_type.name().to_ascii_lowercase()),
    )
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
    if let Some(label) = label.strip_prefix(
        ironsmith_core::static_ability_model::EXPLICIT_STATIC_PRESENTATION_LABEL_PREFIX,
    ) {
        // Unlike ability words such as "Max speed", this label precedes an
        // authored condition that must remain visible in Oracle text.
        let conditioned = first_inner.with_condition(condition).unwrap_or(first_inner);
        return Some((
            format!(
                "{label} — {}",
                render_labeled_static_body(&conditioned, subject)
            ),
            1,
        ));
    }
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

const SOURCE_LINE_KEYWORD_GROUP_SENTINEL: &str = "\0ironsmith:source-line-keyword-group:";

fn source_line_keyword_group_count(ability: &Ability) -> Option<usize> {
    let AbilityKind::Static(static_ability) = &ability.kind else {
        return None;
    };
    let model = static_ability.compiled_model()?;
    let ironsmith_core::StaticAbilityPayload::SourceLineKeywordGroup { keyword_count } =
        &model.payload
    else {
        return None;
    };
    Some(*keyword_count)
}

fn source_line_static_group_count(ability: &Ability) -> Option<usize> {
    let AbilityKind::Static(static_ability) = &ability.kind else {
        return None;
    };
    let model = static_ability.compiled_model()?;
    let ironsmith_core::StaticAbilityPayload::SourceLineStaticGroup { member_count } =
        &model.payload
    else {
        return None;
    };
    Some(*member_count)
}

fn source_line_static_group_presentation_label(ability: &Ability) -> Option<&str> {
    let AbilityKind::Static(static_ability) = &ability.kind else {
        return None;
    };
    static_ability.compiled_model()?.label.strip_prefix(
        ironsmith_core::static_ability_model::EXPLICIT_STATIC_PRESENTATION_LABEL_PREFIX,
    )
}

fn source_line_keyword_group_sentinel(keyword_count: usize) -> String {
    format!("{SOURCE_LINE_KEYWORD_GROUP_SENTINEL}{keyword_count}")
}

fn source_line_keyword_group_sentinel_count(line: &str) -> Option<usize> {
    line.strip_prefix(SOURCE_LINE_KEYWORD_GROUP_SENTINEL)?
        .parse()
        .ok()
}

fn merge_adjacent_keyword_surface_lines(lines: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut idx = 0usize;
    while idx < lines.len() {
        if let Some(keyword_count) = source_line_keyword_group_sentinel_count(&lines[idx]) {
            let mut prefix = None;
            let mut keywords = Vec::with_capacity(keyword_count);
            for offset in 0..keyword_count {
                let Some(line) = lines.get(idx + 1 + offset) else {
                    keywords.clear();
                    break;
                };
                if let Some((line_prefix, keyword)) = split_intrinsic_keyword_line(line) {
                    prefix.get_or_insert(line_prefix);
                    keywords.push(keyword.to_string());
                } else if offset + 1 == keyword_count
                    && let Some(keyword) = split_trailing_numbered_intrinsic_keyword_line(line)
                {
                    keywords.push(keyword.to_string());
                } else {
                    keywords.clear();
                    break;
                }
            }
            if keywords.len() == keyword_count
                && let Some(prefix) = prefix
            {
                out.push(format!(
                    "{prefix}{}",
                    render_intrinsic_keyword_list(&keywords, true)
                ));
                idx += keyword_count + 1;
                continue;
            }

            // The marker is presentation-only. If a structural keyword did
            // not render as one recognizable surface, discard only the marker
            // and preserve every following ability line unchanged.
            idx += 1;
            continue;
        }

        if let Some((prefix, keyword)) = split_intrinsic_keyword_line(&lines[idx]) {
            let mut keywords = vec![keyword.to_string()];
            let mut consumed = 1usize;
            while let Some(next) = lines.get(idx + consumed) {
                let Some((_, next_keyword)) = split_intrinsic_keyword_line(next) else {
                    break;
                };
                keywords.push(next_keyword.to_string());
                consumed += 1;
            }
            if let Some(next) = lines.get(idx + consumed)
                && let Some(next_keyword) = split_trailing_numbered_intrinsic_keyword_line(next)
            {
                keywords.push(next_keyword.to_string());
                consumed += 1;
            }
            if should_compact_numbered_intrinsic_keywords(&keywords) {
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
                // This compact conditional surface represents the single
                // five-arm WUBRG clause used by cards such as Scion of Draco.
                // A shorter adjacent run can instead be independent authored
                // source lines (for example, Righteous War), so preserve those
                // line boundaries rather than inventing one conditional.
                if clauses
                    .iter()
                    .map(|(_, color)| *color)
                    .eq(["white", "blue", "black", "red", "green"])
                {
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

fn split_intrinsic_keyword_line(line: &str) -> Option<(&str, &str)> {
    split_keyword_ability_line(line).or_else(|| split_static_intrinsic_keyword_line(line))
}

/// A numbered intrinsic can finish an ordinary keyword list even when it is
/// kept out of the general mergeable-keyword set. Keeping this trailing-only
/// avoids treating a standalone parameterized ability as a list anchor.
fn split_trailing_numbered_intrinsic_keyword_line(line: &str) -> Option<&str> {
    let (prefix, keyword) = line.split_once(": ")?;
    if !prefix.starts_with("Keyword ability ") && !prefix.starts_with("Static ability ") {
        return None;
    }
    let keyword = keyword.trim_end_matches('.');
    keyword
        .to_ascii_lowercase()
        .strip_prefix("annihilator ")?
        .parse::<u32>()
        .ok()?;
    Some(keyword)
}

fn should_compact_numbered_intrinsic_keywords(keywords: &[String]) -> bool {
    if keywords.len() < 2 {
        return false;
    }
    // A two-ability run is also the exact AST shape produced by adjacent
    // authored keyword lines (for example, Reach followed by Daybound), so
    // keep those top-level boundaries. Longer runs are conventional inline
    // keyword lists. A homogeneous protection run is one decomposed
    // protection surface ("from black and from red"), not separate lines.
    keywords.len() > 2
        || keywords.iter().all(|keyword| {
            keyword
                .trim_end_matches('.')
                .to_ascii_lowercase()
                .starts_with("protection from ")
        })
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
    // Partner-with carries a structurally modeled card-name payload. Keep it
    // on its own line so the ordinary intrinsic-keyword list renderer does
    // not lowercase that payload or absorb a following keyword.
    let carries_named_partner = lower.starts_with("partner with ");
    let is_numbered_firebending = lower
        .strip_prefix("firebending ")
        .is_some_and(|amount| amount.parse::<u32>().is_ok());
    (is_keyword_phrase(keyword) && lower != "changeling" && !carries_named_partner)
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
                | "soulbond"
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

fn modeled_source_keyword_grant(
    ability: &Ability,
) -> Option<(crate::static_abilities::StaticAbilityId, String, &Condition)> {
    if ability.functional_zones.as_slice() != [Zone::Battlefield] {
        return None;
    }
    let AbilityKind::Static(static_ability) = &ability.kind else {
        return None;
    };
    let model = static_ability.compiled_model()?;
    if let ironsmith_core::StaticAbilityPayload::Conditional {
        ability: granted,
        condition,
    } = &model.payload
    {
        let keyword = crate::static_abilities::StaticAbility::from_model(granted.as_ref().clone());
        if !keyword.is_keyword() {
            return None;
        }
        let keyword_text = keyword
            .display()
            .trim()
            .trim_end_matches('.')
            .to_ascii_lowercase();
        if !is_mergeable_keyword_surface(&keyword_text) {
            return None;
        }
        return Some((keyword.id(), keyword_text, condition));
    }
    let ironsmith_core::StaticAbilityPayload::GrantObjectAbilityForFilter(grant) = &model.payload
    else {
        return None;
    };
    if grant.filter != ObjectFilter::source()
        || !grant.additional_abilities.is_empty()
        || grant.set_quantifier_surface.is_some()
        || grant.ability.functional_zones.as_slice() != [Zone::Battlefield]
    {
        return None;
    }
    let ironsmith_core::AbilityKind::Static(granted) = &grant.ability.kind else {
        return None;
    };
    let keyword = crate::static_abilities::StaticAbility::from_model(granted.clone());
    if !keyword.is_keyword() {
        return None;
    }
    let keyword_text = keyword
        .display()
        .trim()
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if !is_mergeable_keyword_surface(&keyword_text) {
        return None;
    }
    Some((keyword.id(), keyword_text, grant.condition.as_ref()?))
}

fn modeled_object_static_grant(
    ability: &Ability,
) -> Option<(
    &ObjectFilter,
    Option<&Condition>,
    crate::static_abilities::StaticAbility,
)> {
    if ability.functional_zones.as_slice() != [Zone::Battlefield] {
        return None;
    }
    let AbilityKind::Static(static_ability) = &ability.kind else {
        return None;
    };
    let model = static_ability.compiled_model()?;
    let ironsmith_core::StaticAbilityPayload::GrantObjectAbilityForFilter(grant) = &model.payload
    else {
        return None;
    };
    if !grant.additional_abilities.is_empty()
        || grant.set_quantifier_surface.is_some()
        || grant.ability.functional_zones.as_slice() != [Zone::Battlefield]
    {
        return None;
    }
    let ironsmith_core::AbilityKind::Static(granted) = &grant.ability.kind else {
        return None;
    };
    Some((
        &grant.filter,
        grant.condition.as_ref(),
        crate::static_abilities::StaticAbility::from_model(granted.clone()),
    ))
}

fn modeled_filter_static_grant(
    ability: &Ability,
) -> Option<(
    &ObjectFilter,
    Option<&Condition>,
    Option<&ironsmith_core::SetQuantifierSurface>,
    crate::static_abilities::StaticAbility,
)> {
    if ability.functional_zones.as_slice() != [Zone::Battlefield] {
        return None;
    }
    let AbilityKind::Static(static_ability) = &ability.kind else {
        return None;
    };
    let model = static_ability.compiled_model()?;
    let ironsmith_core::StaticAbilityPayload::GrantAbility(grant) = &model.payload else {
        return None;
    };
    if grant.ability.functional_zones.as_slice() != [Zone::Battlefield] {
        return None;
    }
    let ironsmith_core::AbilityKind::Static(granted) = &grant.ability.kind else {
        return None;
    };
    Some((
        &grant.filter,
        grant.condition.as_ref(),
        grant.set_quantifier_surface.as_ref(),
        crate::static_abilities::StaticAbility::from_model(granted.clone()),
    ))
}

fn is_can_block_additional_each_combat_rule(
    ability: &crate::static_abilities::StaticAbility,
) -> bool {
    ability.compiled_model().is_some_and(|model| {
        matches!(
            model.payload,
            ironsmith_core::StaticAbilityPayload::CanBlockAdditionalCreatureEachCombat(_)
        )
    })
}

/// Render complete nonkeyword static abilities granted to the same filtered
/// object set as quoted ability text. The executable grants stay independent;
/// this only restores the single authored `have "..." and "..."` surface.
fn describe_structural_quoted_static_grant_bundle(
    abilities: &[Ability],
) -> Option<(String, usize)> {
    let first = abilities.first()?;
    let (filter, condition, set_quantifier, first_granted) = modeled_filter_static_grant(first)?;
    if condition.is_some()
        || first_granted.is_keyword()
        || is_can_block_additional_each_combat_rule(&first_granted)
    {
        return None;
    }
    let AbilityKind::Static(first_static) = &first.kind else {
        return None;
    };
    let first_display = first_static.display();
    let (subject, _, verb) =
        split_static_predicate_with_verb(&first_display, &[" has ", " have "])?;
    let self_subject = granted_ability_self_subject_for_filter(filter);

    let mut quoted = Vec::new();
    let mut consumed = 0usize;
    for ability in abilities {
        let Some((next_filter, next_condition, next_quantifier, granted)) =
            modeled_filter_static_grant(ability)
        else {
            break;
        };
        if next_filter != filter
            || next_condition != condition
            || next_quantifier != set_quantifier
            || granted.is_keyword()
            || is_can_block_additional_each_combat_rule(&granted)
        {
            break;
        }
        let AbilityKind::Static(next_static) = &ability.kind else {
            break;
        };
        let next_display = next_static.display();
        let Some((next_subject, _, next_verb)) =
            split_static_predicate_with_verb(&next_display, &[" has ", " have "])
        else {
            break;
        };
        if next_subject != subject || next_verb != verb {
            break;
        }

        let rendered = describe_static_ability_with_subject(&granted, self_subject);
        let rendered = rendered.trim().trim_end_matches('.');
        if rendered.is_empty() {
            break;
        }
        quoted.push(rendered.to_string());
        consumed += 1;
    }
    if quoted.is_empty() {
        return None;
    }

    let last = quoted.len().saturating_sub(1);
    let quoted = quoted
        .into_iter()
        .enumerate()
        .map(|(idx, mut ability)| {
            if idx == last && !ability.ends_with(['!', '?']) {
                ability.push('.');
            }
            format!("\"{ability}\"")
        })
        .collect::<Vec<_>>();
    Some((
        format!("{subject} {verb} {}", join_english_list(&quoted)),
        consumed,
    ))
}

#[cfg(test)]
mod quoted_static_grant_bundle_tests {
    use super::*;

    #[test]
    fn complete_static_rules_granted_to_one_filter_render_as_quoted_abilities() {
        let definition = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Static Grant Probe",
        )
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Commander creatures you own have \"This creature enters with an additional +1/+1 counter on it\" and \"Other creatures you control enter with an additional +1/+1 counter on them.\"",
        )
        .expect("quoted static grant bundle should compile");

        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition),
            vec![
                "Commander creatures you own have \"This creature enters with an additional +1/+1 counter on it\" and \"Other creatures you control enter with an additional +1/+1 counter on them.\""
                    .to_string()
            ]
        );
    }

    #[test]
    fn keyword_grants_remain_unquoted() {
        let definition = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Keyword Grant Probe",
        )
        .card_types(vec![CardType::Enchantment])
        .parse_text("Creatures you control have flying.")
        .expect("keyword grant should compile");

        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition),
            vec!["Creatures you control have flying.".to_string()]
        );
    }
}

/// Render a granted blocking-capacity rule as the direct rule text it
/// represents instead of treating the rule as a keyword-like noun.
///
/// The generic grant display is intentionally phrased as
/// `"<objects> have <ability>"`, which is appropriate for keywords and quoted
/// abilities. `CanBlockAdditionalCreatureEachCombat` is a rules sentence,
/// though: its authored surface is `"<objects> can block ..."`. Keeping this
/// distinction typed also prevents a separately authored vigilance line from
/// being merged into the blocking rule as one synthetic trait list.
fn describe_structural_can_block_additional_grant(ability: &Ability) -> Option<String> {
    let AbilityKind::Static(static_ability) = &ability.kind else {
        return None;
    };
    let (condition, granted) =
        if let Some((_, condition, _, granted)) = modeled_filter_static_grant(ability) {
            (condition, granted)
        } else {
            let (_, condition, granted) = modeled_object_static_grant(ability)?;
            (condition, granted)
        };
    if condition.is_some() || !is_can_block_additional_each_combat_rule(&granted) {
        return None;
    }

    let display = static_ability.display();
    let display = display.trim().trim_end_matches('.');
    let (subject, _, _) = split_static_predicate_with_verb(display, &[" has ", " have "])?;
    let rule = granted.display();
    let rule = rule.trim().trim_end_matches('.');
    Some(format!("{subject} {}", lowercase_first(rule)))
}

#[cfg(test)]
mod can_block_additional_grant_tests {
    use super::*;

    #[test]
    fn both_typed_grant_families_render_block_capacity_as_a_direct_rule() {
        let filter = ObjectFilter::creature().you_control();
        let blocking_model: crate::static_abilities::CompiledStaticAbility =
            ironsmith_core::StaticAbility {
                id: Some(
                    crate::static_abilities::StaticAbilityId::CanBlockAdditionalCreatureEachCombat,
                ),
                label: "can block additional creature".to_string(),
                payload: ironsmith_core::StaticAbilityPayload::CanBlockAdditionalCreatureEachCombat(
                    1,
                ),
            };
        let granted = ironsmith_core::Ability {
            kind: ironsmith_core::AbilityKind::Static(blocking_model),
            functional_zones: vec![Zone::Battlefield],
        };
        let grant_ability_model: crate::static_abilities::CompiledStaticAbility =
            ironsmith_core::StaticAbility::new(ironsmith_core::GrantAbility::new(
                filter.clone(),
                granted.clone(),
            ));
        let grant_object_model: crate::static_abilities::CompiledStaticAbility =
            ironsmith_core::StaticAbility::new(ironsmith_core::GrantObjectAbilityForFilter::new(
                filter,
                granted,
                "Can block an additional creature each combat",
            ));
        let grants = [
            crate::static_abilities::StaticAbility::from_model(grant_ability_model),
            crate::static_abilities::StaticAbility::from_model(grant_object_model),
        ];

        for grant in grants {
            let definition = crate::cards::builders::CardDefinitionBuilder::new(
                crate::ids::CardId::new(),
                "Blocking Capacity Grant Probe",
            )
            .card_types(vec![CardType::Enchantment])
            .with_ability(Ability::static_ability(grant))
            .build();

            assert_eq!(
                crate::compiled_text::compiled_text_lines(&definition),
                vec![
                    "Creatures you control can block an additional creature each combat."
                        .to_string()
                ]
            );
        }
    }
}

/// Recombine adjacent, independently executable grants from an authored
/// "has [keywords] and can't be blocked by more than N creatures" clause.
/// Matching the typed affected filter and condition prevents a restriction on
/// the source from being folded into an attachment grant.
fn describe_structural_keyword_maximum_blocker_bundle(
    abilities: &[Ability],
) -> Option<(String, usize)> {
    let first = abilities.first()?;
    let (filter, condition, first_granted) = modeled_object_static_grant(first)?;
    if !first_granted.is_keyword() {
        return None;
    }
    let AbilityKind::Static(first_static) = &first.kind else {
        return None;
    };
    let first_display = first_static.display();
    let (subject, _, verb) =
        split_static_predicate_with_verb(&first_display, &[" has ", " have "])?;
    let has_verb = if verb == "have" { "have" } else { "has" };

    let mut keywords = Vec::new();
    let mut consumed = 0usize;
    for ability in abilities {
        let (next_filter, next_condition, granted) = modeled_object_static_grant(ability)?;
        if next_filter != filter || next_condition != condition {
            return None;
        }
        if granted.is_keyword() {
            let keyword = granted
                .display()
                .trim()
                .trim_end_matches('.')
                .to_ascii_lowercase();
            if !is_mergeable_keyword_surface(&keyword) {
                return None;
            }
            keywords.push(keyword);
            consumed += 1;
            continue;
        }
        let model = granted.compiled_model()?;
        let ironsmith_core::StaticAbilityPayload::CantBeBlockedByMoreThan(maximum) = &model.payload
        else {
            return None;
        };
        if keywords.is_empty() {
            return None;
        }
        consumed += 1;
        let maximum = small_number_word(*maximum as u32).unwrap_or_else(|| maximum.to_string());
        let creature = if maximum == "one" {
            "creature"
        } else {
            "creatures"
        };
        return Some((
            format!(
                "{subject} {has_verb} {} and can't be blocked by more than {maximum} {creature}",
                render_keyword_list(&keywords, false)
            ),
            consumed,
        ));
    }
    None
}

/// Render one conditional keyword granted to the source from its typed model.
/// Keeping the condition out of the grammatical subject avoids plural nouns in
/// the condition (for example, "three artifacts") selecting `have` for the
/// singular source.
fn describe_structural_conditional_source_keyword_grant(
    ability: &Ability,
    subject: &str,
) -> Option<String> {
    let (_, keyword, condition) = modeled_source_keyword_grant(ability)?;
    if matches!(
        condition,
        Condition::ActivationTiming(crate::ability::ActivationTiming::DuringYourTurn)
    ) {
        return Some(format!("During your turn, {subject} has {keyword}"));
    }
    let prefix_surface = matches!(
        condition,
        Condition::Not(inner)
            if matches!(inner.as_ref(), Condition::PlayerCastSpellsThisTurnOrMore { .. })
    ) || matches!(
        condition,
        Condition::ValueComparison { left, .. }
            if matches!(left.unhinted(), Value::MaxDiceRolledThisTurn(_))
    ) || matches!(
        condition,
        Condition::TurnHistory(
            ironsmith_core::TurnHistoryCondition::PlayerVisitedAttractionThisTurn(_)
        )
    );
    let condition = describe_condition(condition);
    let condition = condition
        .strip_prefix("this permanent")
        .map(|rest| format!("{}{rest}", lowercase_first(subject)))
        .unwrap_or(condition);
    if prefix_surface {
        Some(format!("As long as {condition}, {subject} has {keyword}"))
    } else {
        Some(format!("{subject} has {keyword} as long as {condition}"))
    }
}

fn modeled_conditioned_source_anthem(ability: &Ability) -> Option<(i32, i32, &Condition)> {
    if ability.functional_zones.as_slice() != [Zone::Battlefield] {
        return None;
    }
    let AbilityKind::Static(static_ability) = &ability.kind else {
        return None;
    };
    if static_ability.id() != crate::static_abilities::StaticAbilityId::Anthem {
        return None;
    }
    let model = static_ability.compiled_model()?;
    let ironsmith_core::StaticAbilityPayload::Anthem(anthem) = &model.payload else {
        return None;
    };
    if anthem.filter.is_some()
        || anthem.set_quantifier_surface.is_some()
        || anthem.count_uses_where_x
        || anthem.replacement_surface.is_some()
    {
        return None;
    }
    let (ironsmith_core::AnthemValue::Fixed(power), ironsmith_core::AnthemValue::Fixed(toughness)) =
        (&anthem.power, &anthem.toughness)
    else {
        return None;
    };
    if (*power, *toughness) == (0, 0) {
        return None;
    }
    Some((*power, *toughness, anthem.condition.as_ref()?))
}

fn modeled_direct_conditional_source_rule(
    ability: &Ability,
    expected_id: crate::static_abilities::StaticAbilityId,
) -> Option<&Condition> {
    if ability.functional_zones.as_slice() != [Zone::Battlefield] {
        return None;
    }
    let AbilityKind::Static(static_ability) = &ability.kind else {
        return None;
    };
    let model = static_ability.compiled_model()?;
    let ironsmith_core::StaticAbilityPayload::Conditional {
        ability: inner,
        condition,
    } = &model.payload
    else {
        return None;
    };
    (inner.id == Some(expected_id)
        && matches!(&inner.payload, ironsmith_core::StaticAbilityPayload::None))
    .then_some(condition)
}

fn delirium_condition_surface(condition: &Condition) -> Option<&'static str> {
    matches!(
        condition,
        Condition::PlayerHasCardTypesInGraveyardOrMore {
            player: PlayerFilter::You,
            count: 4,
        }
    )
    .then_some("there are four or more card types among cards in your graveyard")
}

/// Recombine the layer-correct static siblings emitted for a conditioned
/// source P/T modifier plus a keyword grant. The two executable payloads stay
/// independent; this only restores their single authored condition surface
/// after proving that the source, zone, and typed condition are identical.
fn describe_structural_conditioned_source_anthem_keyword_bundle(
    abilities: &[Ability],
    subject: &str,
) -> Option<(String, usize)> {
    let [anthem_ability, keyword_ability, ..] = abilities else {
        return None;
    };
    let (power, toughness, condition) = modeled_conditioned_source_anthem(anthem_ability)?;
    let (_, keyword, keyword_condition) = modeled_source_keyword_grant(keyword_ability)?;
    if keyword_condition != condition {
        return None;
    }

    let modifier = describe_fixed_anthem_modifier(power, toughness);
    let has_matching_must_attack = abilities.get(2).is_some_and(|ability| {
        modeled_direct_conditional_source_rule(
            ability,
            crate::static_abilities::StaticAbilityId::MustAttack,
        ) == Some(condition)
    });
    if has_matching_must_attack && let Some(condition) = delirium_condition_surface(condition) {
        return Some((
            format!(
                "Delirium — As long as {condition}, {subject} gets {modifier}, has {keyword}, and attacks each combat if able"
            ),
            3,
        ));
    }
    if matches!(
        condition,
        Condition::ActivationTiming(crate::ability::ActivationTiming::DuringYourTurn)
    ) {
        return Some((
            format!("During your turn, {subject} gets {modifier} and has {keyword}"),
            2,
        ));
    }

    let prefix_surface = matches!(
        condition,
        Condition::Not(inner)
            if matches!(inner.as_ref(), Condition::PlayerCastSpellsThisTurnOrMore { .. })
    ) || matches!(
        condition,
        Condition::ValueComparison { left, .. }
            if matches!(left.unhinted(), Value::MaxDiceRolledThisTurn(_))
    );
    let condition = describe_condition(condition);
    let condition = condition
        .strip_prefix("this permanent")
        .map(|rest| format!("{}{rest}", lowercase_first(subject)))
        .unwrap_or(condition);
    let text = if prefix_surface {
        format!("As long as {condition}, {subject} gets {modifier} and has {keyword}")
    } else {
        format!("{subject} gets {modifier} and has {keyword} as long as {condition}")
    };
    Some((text, 2))
}

/// Recombine a layer-correct source animation and its granted annihilator
/// ability when all three executable static models carry the same condition.
/// The compiler intentionally keeps type addition, base P/T, and the granted
/// attack trigger independent; this restores their single authored surface
/// only after proving the source filter, condition, zone, and granted keyword.
fn describe_structural_conditioned_source_animation_annihilator_bundle(
    abilities: &[Ability],
) -> Option<(String, usize)> {
    let [type_ability, pt_ability, grant_ability, ..] = abilities else {
        return None;
    };
    if type_ability.functional_zones.as_slice() != [Zone::Battlefield]
        || pt_ability.functional_zones != type_ability.functional_zones
        || grant_ability.functional_zones != type_ability.functional_zones
    {
        return None;
    }

    let AbilityKind::Static(type_static) = &type_ability.kind else {
        return None;
    };
    let AbilityKind::Static(pt_static) = &pt_ability.kind else {
        return None;
    };
    let AbilityKind::Static(grant_static) = &grant_ability.kind else {
        return None;
    };
    if type_static.id() != crate::static_abilities::StaticAbilityId::AddCardTypes
        || pt_static.id()
            != crate::static_abilities::StaticAbilityId::SetBasePowerToughnessForFilter
        || grant_static.id()
            != crate::static_abilities::StaticAbilityId::GrantObjectAbilityForFilter
    {
        return None;
    }

    let (type_model, type_condition) = unwrapped_static_model(type_static)?;
    let (pt_model, pt_condition) = unwrapped_static_model(pt_static)?;
    let condition = type_condition?;
    if pt_condition != Some(condition) {
        return None;
    }
    let ironsmith_core::StaticAbilityPayload::AddCardTypes {
        filter: type_filter,
        card_types,
    } = &type_model.payload
    else {
        return None;
    };
    let ironsmith_core::StaticAbilityPayload::SetBasePowerToughness {
        filter: pt_filter,
        power,
        toughness,
    } = &pt_model.payload
    else {
        return None;
    };
    if type_filter != &ObjectFilter::source() || pt_filter != type_filter || card_types.is_empty() {
        return None;
    }

    let ironsmith_core::StaticAbilityPayload::GrantObjectAbilityForFilter(grant) =
        &grant_static.compiled_model()?.payload
    else {
        return None;
    };
    if &grant.filter != type_filter
        || grant.condition.as_ref() != Some(condition)
        || !grant.additional_abilities.is_empty()
        || grant.set_quantifier_surface.is_some()
    {
        return None;
    }
    let granted = grant_static.source_granted_inline_abilities();
    let [granted] = granted.as_slice() else {
        return None;
    };
    let AbilityKind::Triggered(triggered) = &granted.kind else {
        return None;
    };
    let keyword = lowercase_first(&describe_annihilator_keyword(triggered)?);

    let type_phrase = card_types
        .iter()
        .map(|card_type| card_type.name().to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    // A numeric power/toughness prefix is adjectival here, so the generic
    // noun-phrase helper intentionally leaves it articleless. Animation
    // surfaces still require the authored "a 3/3 creature" form.
    let animated = format!("a {power}/{toughness} {type_phrase}");
    Some((
        format!(
            "As long as {}, it's {animated} in addition to its other types and it has {keyword}",
            describe_condition(condition),
        ),
        3,
    ))
}

fn threshold_graveyard_presentation_label(condition: &Condition) -> Option<&'static str> {
    // Threshold is an ability word with no rules meaning. This exact typed
    // gate is therefore safe presentation provenance without consulting a
    // card name, Oracle text, or a rendered condition string.
    matches!(
        condition,
        Condition::ValueComparison {
            left: Value::CardsInGraveyard(PlayerFilter::You),
            operator: ironsmith_core::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(7),
        }
    )
    .then_some("Threshold")
}

/// Recombine the layer-correct static siblings emitted for a Threshold source
/// modifier. Every semantic fact is proven from compiled payloads: the source
/// gets exactly +1/+1, becomes black, and gains exactly one activated ability
/// under the same seven-card graveyard gate.
fn describe_structural_threshold_source_modifier_bundle(
    abilities: &[Ability],
    subject: &str,
) -> Option<(String, usize)> {
    let [anthem_ability, color_ability, grant_ability, ..] = abilities else {
        return None;
    };
    if anthem_ability.functional_zones.as_slice() != [Zone::Battlefield]
        || color_ability.functional_zones != anthem_ability.functional_zones
        || grant_ability.functional_zones != anthem_ability.functional_zones
    {
        return None;
    }

    let AbilityKind::Static(anthem_static) = &anthem_ability.kind else {
        return None;
    };
    let AbilityKind::Static(color_static) = &color_ability.kind else {
        return None;
    };
    let AbilityKind::Static(grant_static) = &grant_ability.kind else {
        return None;
    };
    if anthem_static.id() != crate::static_abilities::StaticAbilityId::Anthem
        || color_static.id() != crate::static_abilities::StaticAbilityId::SetColors
        || grant_static.id()
            != crate::static_abilities::StaticAbilityId::GrantObjectAbilityForFilter
    {
        return None;
    }

    let ironsmith_core::StaticAbilityPayload::Anthem(anthem) =
        &anthem_static.compiled_model()?.payload
    else {
        return None;
    };
    let (ironsmith_core::AnthemValue::Fixed(power), ironsmith_core::AnthemValue::Fixed(toughness)) =
        (&anthem.power, &anthem.toughness)
    else {
        return None;
    };
    if anthem.filter.is_some()
        || (*power, *toughness) != (1, 1)
        || anthem.set_quantifier_surface.is_some()
        || anthem.count_uses_where_x
        || anthem.replacement_surface.is_some()
    {
        return None;
    }
    let condition = anthem.condition.as_ref()?;
    let presentation_label = threshold_graveyard_presentation_label(condition)?;

    let color_model = color_static.compiled_model()?;
    let ironsmith_core::StaticAbilityPayload::Conditional {
        ability: inner_color,
        condition: color_condition,
    } = &color_model.payload
    else {
        return None;
    };
    let ironsmith_core::StaticAbilityPayload::SetColors {
        filter: color_filter,
        colors,
    } = &inner_color.payload
    else {
        return None;
    };
    if inner_color.id != Some(crate::static_abilities::StaticAbilityId::SetColors)
        || *colors != crate::color::ColorSet::BLACK
        || color_condition != condition
    {
        return None;
    }

    let grant_model = grant_static.compiled_model()?;
    let ironsmith_core::StaticAbilityPayload::GrantObjectAbilityForFilter(grant) =
        &grant_model.payload
    else {
        return None;
    };
    if color_filter != &grant.filter
        || grant.filter != ObjectFilter::source()
        || grant.condition.as_ref() != Some(condition)
        || !grant.additional_abilities.is_empty()
        || grant.set_quantifier_surface.is_some()
        || grant.ability.functional_zones.as_slice() != [Zone::Battlefield]
        || !matches!(
            &grant.ability.kind,
            ironsmith_core::AbilityKind::Activated(_)
        )
    {
        return None;
    }

    let granted = grant_static.source_granted_inline_abilities();
    let [granted] = granted.as_slice() else {
        return None;
    };
    if !matches!(&granted.kind, AbilityKind::Activated(_)) {
        return None;
    }
    let mut granted_text = describe_inline_ability_with_self_subject(granted, subject)
        .trim()
        .trim_end_matches('.')
        .to_string();
    if granted_text.is_empty() {
        return None;
    }
    if !granted_text.ends_with('!') && !granted_text.ends_with('?') {
        granted_text.push('.');
    }

    Some((
        format!(
            "{presentation_label} — As long as there are seven or more cards in your graveyard, {subject} gets +1/+1, is black, and has \"{granted_text}\""
        ),
        3,
    ))
}

fn modeled_keyword_count_condition(
    condition: &Condition,
    keyword: crate::static_abilities::StaticAbilityId,
) -> Option<(ObjectFilter, &str)> {
    let Condition::CountComparison {
        count: ironsmith_core::AnthemCountExpression::MatchingFilter(filter),
        comparison: Comparison::GreaterThanOrEqual(1),
        display: Some(display),
    } = condition
    else {
        return None;
    };
    if filter.static_abilities.as_slice() != [keyword]
        || !filter.excluded_static_abilities.is_empty()
        || !filter.ability_markers.is_empty()
        || !filter.excluded_ability_markers.is_empty()
    {
        return None;
    }
    let mut basis = filter.clone();
    basis.static_abilities.clear();
    Some((basis, display.trim()))
}

fn normalize_same_true_condition_statement(condition: &str, keyword: &str) -> String {
    let condition = condition.trim().trim_end_matches('.');
    if let Some(existence) = condition.strip_prefix("there is ") {
        let keyword_suffix = format!(" with {keyword}");
        if let Some(subject) = existence.strip_suffix(&keyword_suffix) {
            return format!("{subject} has {keyword}");
        }
        if let Some(subject) = existence.strip_suffix(" in a graveyard")
            && subject.ends_with(&keyword_suffix)
        {
            return format!("{subject} is in a graveyard");
        }
    }
    if let Some(subject) = condition.strip_prefix("your opponents control ") {
        return format!("an opponent controls {subject}");
    }
    condition.to_string()
}

/// Render a structurally complete family of source keyword grants whose
/// conditions differ only by requiring the same keyword on matching cards.
///
/// This is the executable shape behind "The same is true for ..." ladders.
/// Requiring one independent typed grant and one independent matching-filter
/// condition per keyword prevents a malformed, concatenated condition from
/// being made to look complete by the renderer.
fn describe_structural_keyword_same_is_true_ladder(
    abilities: &[Ability],
    subject: &str,
) -> Option<(String, usize)> {
    let mut keywords = Vec::new();
    let mut shared_condition_basis: Option<ObjectFilter> = None;
    let mut first_condition = None;
    let mut consumed = 0usize;

    for ability in abilities {
        let Some((keyword_id, keyword, condition)) = modeled_source_keyword_grant(ability) else {
            break;
        };
        let Some((condition_basis, condition_surface)) =
            modeled_keyword_count_condition(condition, keyword_id)
        else {
            break;
        };
        if let Some(shared) = &shared_condition_basis {
            if shared != &condition_basis {
                break;
            }
        } else {
            shared_condition_basis = Some(condition_basis);
            first_condition = Some(condition_surface.to_string());
        }
        keywords.push(keyword);
        consumed += 1;
    }

    if consumed < 3 {
        return None;
    }
    let first_keyword = &keywords[0];
    let condition =
        normalize_same_true_condition_statement(first_condition.as_deref()?, first_keyword);
    Some((
        format!(
            "As long as {condition}, {subject} has {first_keyword}. The same is true for {}",
            render_keyword_list(&keywords[1..], false),
        ),
        consumed,
    ))
}

fn modeled_all_subtypes_of_family_surface(
    ability: &Ability,
) -> Option<(&ObjectFilter, ironsmith_core::SubtypeFamily, String)> {
    if ability.functional_zones.as_slice() != [Zone::Battlefield] {
        return None;
    }
    let AbilityKind::Static(static_ability) = &ability.kind else {
        return None;
    };
    let model = static_ability.compiled_model()?;
    let ironsmith_core::StaticAbilityPayload::AddAllSubtypesOfFamily { filter, family } =
        &model.payload
    else {
        return None;
    };
    let display = static_ability.display();
    let (subject, predicate, _) = split_static_predicate_with_verb(&display, &[" is ", " are "])?;
    if predicate != format!("every {}", family.type_phrase()) {
        return None;
    }
    Some((filter, *family, subject.to_string()))
}

fn owned_nonbattlefield_card_union_subject(filter: &ObjectFilter) -> Option<String> {
    ironsmith_core::filter_model::describe_owned_nonbattlefield_card_union(filter)
}

/// Rejoin the typed battlefield/spell/nonbattlefield-card scopes of a shared
/// all-subtypes grant. The three filters retain distinct runtime zones while
/// the identical subtype family proves the "same is true" relationship.
fn describe_structural_all_subtypes_scope_ladder(abilities: &[Ability]) -> Option<(String, usize)> {
    let [battlefield, stack, nonbattlefield, ..] = abilities else {
        return None;
    };
    let (battlefield_filter, family, battlefield_subject) =
        modeled_all_subtypes_of_family_surface(battlefield)?;
    let (stack_filter, stack_family, stack_subject) =
        modeled_all_subtypes_of_family_surface(stack)?;
    let (nonbattlefield_filter, nonbattlefield_family, _) =
        modeled_all_subtypes_of_family_surface(nonbattlefield)?;
    if family != stack_family
        || family != nonbattlefield_family
        || battlefield_filter.zone != Some(Zone::Battlefield)
        || battlefield_filter.controller != Some(PlayerFilter::You)
        || stack_filter.zone != Some(Zone::Stack)
        || stack_filter.controller != Some(PlayerFilter::You)
        || stack_filter.stack_kind != Some(crate::filter::StackObjectKind::Spell)
        || battlefield_filter.card_types != stack_filter.card_types
    {
        return None;
    }
    let nonbattlefield_subject = owned_nonbattlefield_card_union_subject(nonbattlefield_filter)?;
    let first = format!("{battlefield_subject} are every {}", family.type_phrase());
    Some((
        format!(
            "{first}. The same is true for {} and {}",
            lowercase_first(&stack_subject),
            lowercase_first(&nonbattlefield_subject),
        ),
        3,
    ))
}

#[cfg(test)]
mod all_subtypes_scope_ladder_tests {
    use super::*;

    #[test]
    fn battlefield_spell_and_owned_card_scopes_rejoin_with_same_is_true() {
        let oracle = "Creatures you control are every creature type. The same is true for creature spells you control and creature cards you own that aren't on the battlefield.";
        let definition = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "All Subtypes Scope Probe",
        )
        .card_types(vec![CardType::Artifact])
        .parse_text(oracle)
        .expect("all-subtypes scope ladder should compile");

        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition),
            vec![oracle.to_string()]
        );
    }
}

fn modeled_static_variant_copy(
    ability: &Ability,
) -> Option<&ironsmith_core::CopyStaticAbilityVariants> {
    if ability.functional_zones.as_slice() != [Zone::Battlefield] {
        return None;
    }
    let AbilityKind::Static(static_ability) = &ability.kind else {
        return None;
    };
    let model = static_ability.compiled_model()?;
    let ironsmith_core::StaticAbilityPayload::CopyStaticAbilityVariants(copy) = &model.payload
    else {
        return None;
    };
    Some(copy)
}

fn static_variant_selector_keyword(
    selector: ironsmith_core::StaticAbilityVariantSelector,
) -> Option<&'static str> {
    use crate::static_abilities::StaticAbilityId::*;
    use ironsmith_core::StaticAbilityVariantSelector::{Any, ProtectionFromColor};

    match selector {
        Any(Flying) => Some("flying"),
        Any(Fear) => Some("fear"),
        Any(FirstStrike) => Some("first strike"),
        Any(DoubleStrike) => Some("double strike"),
        Any(Deathtouch) => Some("deathtouch"),
        Any(Haste) => Some("haste"),
        Any(Landwalk) => Some("landwalk"),
        Any(Lifelink) => Some("lifelink"),
        Any(Protection) => Some("protection"),
        ProtectionFromColor => Some("protection from any color"),
        Any(Reach) => Some("reach"),
        Any(Trample) => Some("trample"),
        Any(Shroud) => Some("shroud"),
        Any(Vigilance) => Some("vigilance"),
        Any(Hexproof) | Any(HexproofFrom) => Some("hexproof"),
        Any(Indestructible) => Some("indestructible"),
        Any(Menace) => Some("menace"),
        Any(Shadow) => Some("shadow"),
        Any(Skulk) => Some("skulk"),
        _ => None,
    }
}

/// Rejoin a leading conditional keyword grant with the payload-preserving
/// variant-copy carrier for the remaining "The same is true" branches.
///
/// Delve-linked exile is executable provenance: the leading grant and every
/// copied variant must inspect the same source-exiled object filter, and the
/// source must actually have delve. This avoids collapsing unrelated keyword
/// grants that merely happen to be adjacent.
fn describe_structural_delve_keyword_variant_ladder(
    abilities: &[Ability],
    subject: &str,
    source_has_delve: bool,
) -> Option<(String, usize)> {
    if !source_has_delve {
        return None;
    }
    let [leading, variants, ..] = abilities else {
        return None;
    };
    let (leading_id, leading_keyword, condition) = modeled_source_keyword_grant(leading)?;
    let (condition_basis, _) = modeled_keyword_count_condition(condition, leading_id)?;
    let copy = modeled_static_variant_copy(variants)?;
    if !copy.exclude_source_id
        || copy.filter != condition_basis
        || !copy.filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
                && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        })
        || !copy
            .display
            .to_ascii_lowercase()
            .contains("the same is true for")
    {
        return None;
    }

    let mut trailing_keywords = Vec::new();
    for selector in &copy.selectors {
        let keyword = static_variant_selector_keyword(*selector)?.to_string();
        if keyword != leading_keyword && !trailing_keywords.contains(&keyword) {
            trailing_keywords.push(keyword);
        }
    }
    if trailing_keywords.len() < 2 {
        return None;
    }

    let mut noun_filter = copy.filter.clone();
    noun_filter.zone = None;
    noun_filter.tagged_constraints.retain(|constraint| {
        constraint.tag.as_str() != crate::tag::SOURCE_EXILED_TAG
            || constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
    });
    let noun_description = noun_filter.description();
    let noun = strip_leading_article(&noun_description);
    Some((
        format!(
            "If {} with {leading_keyword} was exiled with {subject}'s delve ability, {subject} has {leading_keyword}. The same is true for {}",
            with_indefinite_article(noun),
            render_keyword_list(&trailing_keywords, false),
        ),
        2,
    ))
}

fn modeled_basic_land_count_condition(condition: &Condition) -> Option<(Subtype, &str)> {
    let Condition::CountComparison {
        count: ironsmith_core::AnthemCountExpression::MatchingFilter(filter),
        comparison: Comparison::GreaterThanOrEqual(1),
        display: Some(display),
    } = condition
    else {
        return None;
    };
    let [subtype] = filter.subtypes.as_slice() else {
        return None;
    };
    if filter.controller != Some(PlayerFilter::You)
        || !matches!(
            subtype,
            Subtype::Plains
                | Subtype::Island
                | Subtype::Swamp
                | Subtype::Mountain
                | Subtype::Forest
        )
    {
        return None;
    }
    let mut basis = filter.clone();
    basis.controller = None;
    basis.subtypes.clear();
    if basis != ObjectFilter::default() {
        return None;
    }
    Some((*subtype, display.trim()))
}

fn modeled_basic_land_source_modifier(ability: &Ability) -> Option<(Subtype, String)> {
    if ability.functional_zones.as_slice() != [Zone::Battlefield] {
        return None;
    }
    let AbilityKind::Static(static_ability) = &ability.kind else {
        return None;
    };
    let model = static_ability.compiled_model()?;
    if let ironsmith_core::StaticAbilityPayload::Anthem(anthem) = &model.payload {
        if anthem.filter.is_some()
            || anthem.set_quantifier_surface.is_some()
            || anthem.count_uses_where_x
            || anthem.replacement_surface.is_some()
        {
            return None;
        }
        let (
            ironsmith_core::AnthemValue::Fixed(power),
            ironsmith_core::AnthemValue::Fixed(toughness),
        ) = (&anthem.power, &anthem.toughness)
        else {
            return None;
        };
        if *power == 0 && *toughness == 0 {
            return None;
        }
        let (subtype, condition) = modeled_basic_land_count_condition(anthem.condition.as_ref()?)?;
        return Some((
            subtype,
            format!("gets {power:+}/{toughness:+} as long as {condition}"),
        ));
    }

    let (_, keyword, condition) = modeled_source_keyword_grant(ability)?;
    let (subtype, condition) = modeled_basic_land_count_condition(condition)?;
    Some((subtype, format!("has {keyword} as long as {condition}")))
}

/// Combine the full five-basic-land source-modifier ladder while retaining
/// each independent typed condition and executable modifier.
fn describe_structural_five_basic_land_modifier_ladder(
    abilities: &[Ability],
    subject: &str,
) -> Option<(String, usize)> {
    let mut items = Vec::new();
    for ability in abilities.iter().take(6) {
        let Some(item) = modeled_basic_land_source_modifier(ability) else {
            break;
        };
        items.push(item);
    }
    if items.len() != 5 {
        return None;
    }
    for required in [
        Subtype::Plains,
        Subtype::Island,
        Subtype::Swamp,
        Subtype::Mountain,
        Subtype::Forest,
    ] {
        if items
            .iter()
            .filter(|(subtype, _)| *subtype == required)
            .count()
            != 1
        {
            return None;
        }
    }
    let predicates = items
        .iter()
        .map(|(_, predicate)| predicate.clone())
        .collect::<Vec<_>>();
    Some((
        format!(
            "{} {}",
            capitalize_first(subject),
            join_english_list(&predicates)
        ),
        5,
    ))
}

#[derive(Clone, Copy)]
enum CrossSegmentResultEffect<'a> {
    If(&'a crate::effects::IfEffect),
    Reflexive(&'a crate::effects::ReflexiveTriggerEffect),
    ForPlayers(&'a crate::effects::ForPlayersEffect),
}

fn cross_segment_result_effect(effect: &Effect) -> Option<CrossSegmentResultEffect<'_>> {
    if let Some(if_effect) = effect.downcast_ref::<crate::effects::IfEffect>() {
        return Some(CrossSegmentResultEffect::If(if_effect));
    }
    if let Some(reflexive) = effect.downcast_ref::<crate::effects::ReflexiveTriggerEffect>() {
        return Some(CrossSegmentResultEffect::Reflexive(reflexive));
    }
    if let Some(for_players) = effect.downcast_ref::<crate::effects::ForPlayersEffect>() {
        return Some(CrossSegmentResultEffect::ForPlayers(for_players));
    }
    effect
        .downcast_ref::<crate::effects::WithIdEffect>()?
        .effect
        .downcast_ref::<crate::effects::IfEffect>()
        .map(CrossSegmentResultEffect::If)
}

fn describe_cross_segment_look_optional_payment_disposition_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [
        look_segment,
        payment_segment,
        accepted_segment,
        declined_segment,
    ] = segments.get(start..start + 4)?
    else {
        return None;
    };
    if [
        look_segment,
        payment_segment,
        accepted_segment,
        declined_segment,
    ]
    .iter()
    .any(|segment| !segment.self_replacements.is_empty())
    {
        return None;
    }

    let [look_effect] = look_segment.default_effects.as_slice() else {
        return None;
    };
    let look = unwrap_basic_render_wrapper(look_effect)
        .downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    if look.player != PlayerFilter::You
        || look.reveal
        || !matches!(look.count.unhinted(), Value::Fixed(count) if *count >= 2)
    {
        return None;
    }

    let [payment_effect] = payment_segment.default_effects.as_slice() else {
        return None;
    };
    let payment_with_id = payment_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = payment_with_id
        .effect
        .downcast_ref::<crate::effects::MayEffect>()?;
    if may.fallback != crate::decision::FallbackStrategy::Decline
        || !matches!(may.decider.as_ref(), None | Some(PlayerFilter::You))
    {
        return None;
    }

    fn result_if(effect: &Effect) -> Option<&crate::effects::IfEffect> {
        unwrap_basic_render_wrapper(effect).downcast_ref::<crate::effects::IfEffect>()
    }

    let [accepted_effect] = accepted_segment.default_effects.as_slice() else {
        return None;
    };
    let accepted = result_if(accepted_effect)?;
    let [declined_effect] = declined_segment.default_effects.as_slice() else {
        return None;
    };
    let declined = result_if(declined_effect)?;
    if accepted.condition != payment_with_id.id
        || declined.condition != payment_with_id.id
        || accepted.predicate != EffectPredicate::Happened
        || !matches!(
            declined.predicate,
            EffectPredicate::DidNotHappen | EffectPredicate::WasDeclined
        )
        || !accepted.else_.is_empty()
        || !declined.else_.is_empty()
    {
        return None;
    }

    fn exact_one_looked_move<'a>(
        effects: &'a [Effect],
        looked_tag: &TagKey,
        zone: Zone,
    ) -> Option<&'a crate::effects::MoveToZoneEffect> {
        let [effect] = effects else {
            return None;
        };
        let move_to_zone = unwrap_basic_render_wrapper(effect)
            .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
        let ChooseSpec::WithCount(target, count) = move_to_zone.target.unhinted() else {
            return None;
        };
        (move_to_zone.zone == zone
            && !move_to_zone.to_top
            && *count == ChoiceCount::exactly(1)
            && matches!(target.base(), ChooseSpec::Tagged(tag) if tag == looked_tag)
            && matches!(
                move_to_zone.actor_surface.as_ref(),
                None | Some(PlayerFilter::You)
            )
            && matches!(
                move_to_zone.destination_player_surface.as_ref(),
                None | Some(PlayerFilter::You)
            ))
        .then_some(move_to_zone)
    }

    exact_one_looked_move(&accepted.then, &look.tag, Zone::Hand)?;
    exact_one_looked_move(&declined.then, &look.tag, Zone::Library)?;

    let look_text = capitalize_first(describe_effect(look_effect).trim().trim_end_matches('.'));
    let payment_text =
        capitalize_first(describe_effect(payment_effect).trim().trim_end_matches('.'));
    Some((
        format!(
            "{look_text}. {payment_text}. If you do, put one of those cards into your hand. If you don't, put one of those cards on the bottom of your library"
        ),
        4,
    ))
}

/// Rejoin a will-of-the-council vote with its option conditionals when
/// sentence lowering placed each result sentence in its own resolution
/// segment. The named-vote helper still proves every option/condition edge.
fn describe_cross_segment_named_vote_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let first = segments.get(start)?;
    if !first.self_replacements.is_empty() {
        return None;
    }
    let [vote_effect] = first.default_effects.as_slice() else {
        return None;
    };
    vote_effect.downcast_ref::<crate::effects::VoteEffect>()?;

    let mut effects: Vec<&Effect> = vec![vote_effect];
    let mut end = start + 1;
    while let Some(segment) = segments.get(end) {
        if !segment.self_replacements.is_empty() || segment.default_effects.is_empty() {
            break;
        }
        let all_vote_conditionals = segment.default_effects.iter().all(|effect| {
            effect
                .downcast_ref::<crate::effects::ConditionalEffect>()
                .is_some_and(|conditional| {
                    matches!(
                        &conditional.condition,
                        Condition::VoteOptionGetsMoreVotes(_)
                            | Condition::VoteOptionGetsMoreVotesOrTied(_)
                    )
                })
        });
        if !all_vote_conditionals {
            break;
        }
        effects.extend(segment.default_effects.iter());
        end += 1;
    }
    if end == start + 1 {
        return None;
    }
    let rendered =
        crate::compiled_text::render_effects::describe_planeswalk_chaos_vote_sequence(&effects)
            .or_else(|| {
                crate::compiled_text::render_effects::describe_named_vote_conditional_sequence(
                    &effects,
                )
            })?;
    Some((rendered, end - start))
}

/// Rejoin an authored discard-hand/add-mana/draw sequence when sentence
/// lowering preserved its result references but split the actions into
/// adjacent resolution segments.
fn describe_cross_segment_discard_hand_add_mana_draw_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let mut effects = Vec::with_capacity(3);
    let mut end = start;
    while effects.len() < 3 {
        let segment = segments.get(end)?;
        if !segment.self_replacements.is_empty()
            || segment.default_effects.is_empty()
            || effects.len() + segment.default_effects.len() > 3
        {
            return None;
        }
        effects.extend(segment.default_effects.iter());
        end += 1;
    }
    if end == start + 1 {
        return None;
    }
    let rendered =
        crate::compiled_text::render_effects::describe_discard_hand_add_mana_draw_sequence(
            &effects,
        )?;
    Some((rendered, end - start))
}

fn describe_cross_segment_player_damage_then_discard_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [damage_segment, discard_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !damage_segment.self_replacements.is_empty() || !discard_segment.self_replacements.is_empty()
    {
        return None;
    }
    let [damage_effect] = damage_segment.default_effects.as_slice() else {
        return None;
    };
    let [discard_effect] = discard_segment.default_effects.as_slice() else {
        return None;
    };
    let rendered =
        crate::compiled_text::render_effects::describe_player_damage_then_same_player_discards(&[
            damage_effect,
            discard_effect,
        ])?;
    Some((rendered, 2))
}

fn describe_draw_then_discard_unless_condition(condition: &Condition) -> String {
    if let Condition::ValueComparison {
        left,
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right,
    } = condition
        && let (Value::Count(filter), Value::Fixed(count)) = (left.unhinted(), right.unhinted())
        && *filter
            == ObjectFilter::default()
                .in_zone(Zone::Graveyard)
                .owned_by(PlayerFilter::You)
        && *count >= 0
    {
        let count = u32::try_from(*count)
            .ok()
            .and_then(small_number_word)
            .unwrap_or_else(|| count.to_string());
        return format!("there are {count} or more cards in your graveyard");
    }
    if let Condition::TriggeringSpellManaSpentToCastAtLeast {
        amount,
        symbol: None,
    } = condition
    {
        let amount = small_number_word(*amount).unwrap_or_else(|| amount.to_string());
        return format!("{amount} or more mana was spent to cast that spell");
    }
    describe_condition(condition)
}

fn describe_cross_segment_draw_then_discard_unless_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [draw_segment, discard_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !draw_segment.self_replacements.is_empty() || !discard_segment.self_replacements.is_empty() {
        return None;
    }
    let [draw_effect] = draw_segment.default_effects.as_slice() else {
        return None;
    };
    let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
    let [sequence_effect] = discard_segment.default_effects.as_slice() else {
        return None;
    };
    let sequence = sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    if sequence.surface != ironsmith_core::SequenceSurface::SentenceLeadingThen {
        return None;
    }
    let [conditional_effect] = sequence.effects.as_slice() else {
        return None;
    };
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if conditional.surface != ironsmith_core::ConditionalSurface::LeadingIf
        || !conditional.if_false.is_empty()
    {
        return None;
    }
    let Condition::Not(positive_condition) = &conditional.condition else {
        return None;
    };
    let [discard_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let discard = discard_effect.downcast_ref::<crate::effects::DiscardEffect>()?;
    let draw_then_discard =
        crate::compiled_text::render_effects::describe_draw_then_discard(draw, discard)?;
    let (draw_clause, discard_clause) = draw_then_discard.split_once(", then ")?;
    let condition = describe_draw_then_discard_unless_condition(positive_condition);

    Some((
        format!("{draw_clause}. Then {discard_clause} unless {condition}"),
        2,
    ))
}

fn describe_cross_segment_return_animation_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [return_segment, animation_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !return_segment.self_replacements.is_empty()
        || !animation_segment.self_replacements.is_empty()
    {
        return None;
    }
    let [return_effect] = return_segment.default_effects.as_slice() else {
        return None;
    };
    let [animation_effect] = animation_segment.default_effects.as_slice() else {
        return None;
    };
    crate::compiled_text::render_effects::describe_returned_battlefield_object_then_animated_pair(
        return_effect,
        animation_effect,
    )
    .map(|rendered| (rendered, 2))
}

/// Rejoin a battlefield return/move with the coordinated color-and-subtype
/// modifications authored in the following sentence. A third source-move
/// segment is retained when it is part of the same typed spell procedure.
fn describe_cross_segment_color_subtype_addition_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [producer_segment, modification_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !producer_segment.self_replacements.is_empty()
        || !modification_segment.self_replacements.is_empty()
    {
        return None;
    }
    let [producer] = producer_segment.default_effects.as_slice() else {
        return None;
    };
    let [modifications] = modification_segment.default_effects.as_slice() else {
        return None;
    };

    if let Some(followup_segment) = segments.get(start + 2)
        && followup_segment.self_replacements.is_empty()
        && let [followup] = followup_segment.default_effects.as_slice()
        && let Some(rendered) =
            crate::compiled_text::render_effects::describe_return_then_color_subtype_addition(&[
                producer,
                modifications,
                followup,
            ])
    {
        return Some((rendered, 3));
    }

    crate::compiled_text::render_effects::describe_return_then_color_subtype_addition(&[
        producer,
        modifications,
    ])
    .or_else(|| {
        crate::compiled_text::render_effects::describe_move_then_color_subtype_addition(&[
            producer,
            modifications,
        ])
    })
    .map(|rendered| (rendered, 2))
}

/// Expose a segmented same-name extraction program to its exact typed
/// renderer. Sequential wrapper effects are expanded only inside this window;
/// the matcher still verifies every chooser, zone, tag, shuffle, and token
/// count relationship before accepting it.
fn describe_cross_segment_necromentia_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let mut effects = Vec::new();
    for end in start..(start + 3).min(segments.len()) {
        let segment = &segments[end];
        if !segment.self_replacements.is_empty() || segment.default_effects.is_empty() {
            return None;
        }
        for effect in &segment.default_effects {
            if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
                effects.extend(sequence.effects.iter());
            } else {
                effects.push(effect);
            }
        }
        if end > start
            && let Some(rendered) =
                crate::compiled_text::render_effects::render_necromentia_shape(&effects)
        {
            return Some((rendered, end - start + 1));
        }
    }
    None
}

/// Rejoin a typed prior-result gate and its separately lowered `otherwise`
/// branch when both IDs prove one choose-name/mill conditional. Runtime keeps
/// the gate result addressable for the fallback; rendering can recover the
/// authored conditional once the producer tag, both IDs, and both branches
/// agree.
fn describe_cross_segment_choose_name_mill_conditional_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [producer_segment, gate_segment, fallback_segment] = segments.get(start..start + 3)? else {
        return None;
    };
    if [producer_segment, gate_segment, fallback_segment]
        .iter()
        .any(|segment| !segment.self_replacements.is_empty())
    {
        return None;
    }

    let [choose_effect, target_effect, producer_effect] =
        producer_segment.default_effects.as_slice()
    else {
        return None;
    };
    let producer_with_id = producer_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let tagged_mill = producer_with_id
        .effect
        .downcast_ref::<crate::effects::TaggedEffect>()?;
    tagged_mill
        .effect
        .downcast_ref::<crate::effects::MillEffect>()?;

    let [gate_effect] = gate_segment.default_effects.as_slice() else {
        return None;
    };
    let gate_with_id = gate_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let gate = gate_with_id
        .effect
        .downcast_ref::<crate::effects::IfEffect>()?;
    let EffectPredicate::PriorEffectResult(surface) = &gate.predicate else {
        return None;
    };
    if gate.condition != producer_with_id.id
        || !gate.else_.is_empty()
        || surface.action != crate::effect::PriorEffectAction::Milled
        || surface.actor != crate::effect::PriorEffectResultActor::Passive
        || surface.quantifier != crate::effect::PriorEffectResultQuantifier::One
    {
        return None;
    }

    let [fallback_effect] = fallback_segment.default_effects.as_slice() else {
        return None;
    };
    let fallback = fallback_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if fallback.condition != gate_with_id.id
        || fallback.predicate != EffectPredicate::DidNotHappen
        || !fallback.else_.is_empty()
    {
        return None;
    }

    let conditional = Effect::conditional(
        Condition::TaggedObjectMatches(tagged_mill.tag.clone(), surface.filter.clone()),
        gate.then.clone(),
        fallback.then.clone(),
    );
    let normalized = [
        choose_effect.clone(),
        target_effect.clone(),
        (*producer_with_id.effect).clone(),
        conditional,
    ];
    describe_choose_name_target_mills_conditional_draw(&normalized).map(|rendered| (rendered, 3))
}

fn describe_same_target_continuous_outcome_branches(
    accepted_effect: &Effect,
    fallback_effect: &Effect,
) -> Option<(String, String)> {
    let accepted_tagged = accepted_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let fallback_tagged = fallback_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let accepted = accepted_tagged
        .effect
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let fallback = fallback_tagged
        .effect
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let accepted_target = accepted.target_spec.as_ref()?;
    let fallback_target = fallback.target_spec.as_ref()?;
    if accepted_target.unhinted() != fallback_target.unhinted() {
        return None;
    }
    let ChooseSpec::Target(inner) = accepted_target.unhinted() else {
        return None;
    };
    let ChooseSpec::Object(filter) = inner.unhinted() else {
        return None;
    };
    if !filter.card_types.contains(&CardType::Creature) {
        return None;
    }

    let target_text = describe_choose_spec(accepted_target);
    let capitalized_target = capitalize_first(&target_text);
    let accepted_text = describe_effect(accepted_effect);
    let fallback_text = describe_effect(fallback_effect);
    let accepted_tail = accepted_text
        .strip_prefix(&target_text)
        .or_else(|| accepted_text.strip_prefix(&capitalized_target))?;
    let fallback_tail = fallback_text
        .strip_prefix(&target_text)
        .or_else(|| fallback_text.strip_prefix(&capitalized_target))?;
    Some((
        format!("{target_text}{accepted_tail}"),
        format!("that creature{fallback_tail}"),
    ))
}

/// Rejoin an optional action with its two exclusive same-target continuous
/// outcomes. Both branches declare the exact same target spec; the distinct
/// result tags track each branch's modification, not distinct target choices.
fn describe_cross_segment_may_same_target_continuous_outcomes_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [may_segment, accepted_segment, fallback_segment] = segments.get(start..start + 3)? else {
        return None;
    };
    if [may_segment, accepted_segment, fallback_segment]
        .iter()
        .any(|segment| !segment.self_replacements.is_empty())
    {
        return None;
    }
    let [may_effect] = may_segment.default_effects.as_slice() else {
        return None;
    };
    let may_with_id = may_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = may_with_id
        .effect
        .downcast_ref::<crate::effects::MayEffect>()?;
    if may.fallback != crate::decision::FallbackStrategy::Decline
        || !matches!(may.decider.as_ref(), None | Some(PlayerFilter::You))
    {
        return None;
    }

    let [accepted_effect] = accepted_segment.default_effects.as_slice() else {
        return None;
    };
    let accepted_with_id = accepted_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let accepted = accepted_with_id
        .effect
        .downcast_ref::<crate::effects::IfEffect>()?;
    let [accepted_branch] = accepted.then.as_slice() else {
        return None;
    };
    let [fallback_effect] = fallback_segment.default_effects.as_slice() else {
        return None;
    };
    let fallback = fallback_effect.downcast_ref::<crate::effects::IfEffect>()?;
    let [fallback_branch] = fallback.then.as_slice() else {
        return None;
    };
    if accepted.condition != may_with_id.id
        || fallback.condition != may_with_id.id
        || accepted.predicate != EffectPredicate::Happened
        || fallback.predicate != EffectPredicate::DidNotHappen
        || !accepted.else_.is_empty()
        || !fallback.else_.is_empty()
    {
        return None;
    }
    let (accepted_text, fallback_text) =
        describe_same_target_continuous_outcome_branches(accepted_branch, fallback_branch)?;
    let may_text = describe_effect(may_effect);
    Some((
        format!(
            "{}. If you do, {accepted_text}. Otherwise, {fallback_text}",
            may_text.trim().trim_end_matches('.')
        ),
        3,
    ))
}

#[cfg(test)]
mod same_target_continuous_outcome_tests {
    use super::*;

    fn tagged_pump(target: ChooseSpec, amount: i32, tag: &str) -> Effect {
        Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
            target,
            crate::continuous::Modification::ModifyPowerToughness {
                power: amount,
                toughness: amount,
            },
            Until::EndOfTurn,
        ))
        .tag(TagKey::from(tag))
    }

    #[test]
    fn outcome_branch_backref_rejects_different_declared_targets() {
        let accepted = tagged_pump(
            ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature())),
            4,
            "pumped_1",
        );
        let fallback = tagged_pump(
            ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().you_control())),
            2,
            "pumped_2",
        );

        assert_eq!(
            describe_same_target_continuous_outcome_branches(&accepted, &fallback),
            None
        );
    }
}

fn describe_cross_segment_result_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    if let Some(rendered) =
        describe_cross_segment_may_same_target_continuous_outcomes_window(segments, start)
    {
        return Some(rendered);
    }
    let antecedent_segment = segments.get(start)?;
    if !antecedent_segment.self_replacements.is_empty() {
        return None;
    }
    let antecedent_effects = &antecedent_segment.default_effects;
    let antecedent = antecedent_effects
        .last()?
        .downcast_ref::<crate::effects::WithIdEffect>()?;

    let mut end = start + 1;
    let mut saw_result = false;
    let mut saw_reflexive = false;
    while let Some(segment) = segments.get(end) {
        if !segment.self_replacements.is_empty() {
            break;
        }
        let [effect] = segment.default_effects.as_slice() else {
            break;
        };
        let Some(result) = cross_segment_result_effect(effect) else {
            break;
        };
        let matches_antecedent = match result {
            CrossSegmentResultEffect::If(if_effect) => {
                !saw_reflexive
                    && if_effect.condition == antecedent.id
                    && describe_with_id_if_clause(antecedent, if_effect).is_some()
            }
            CrossSegmentResultEffect::Reflexive(reflexive) => {
                !saw_result
                    && reflexive.condition == antecedent.id
                    && describe_with_id_then_reflexive_trigger(antecedent, reflexive).is_some()
            }
            CrossSegmentResultEffect::ForPlayers(for_players) => {
                !saw_result
                    && (describe_with_id_then_for_players_if_happened(antecedent, for_players)
                        .is_some()
                        || describe_with_id_then_for_players_if_didnt(antecedent, for_players)
                            .is_some())
            }
        };
        if !matches_antecedent {
            break;
        }
        saw_result = true;
        saw_reflexive = matches!(result, CrossSegmentResultEffect::Reflexive(_));
        end += 1;
        if saw_reflexive || matches!(result, CrossSegmentResultEffect::ForPlayers(_)) {
            break;
        }
    }
    if !saw_result {
        return None;
    }

    let combined = segments[start..end]
        .iter()
        .flat_map(|segment| segment.default_effects.iter().cloned())
        .collect::<Vec<_>>();
    Some((describe_effect_list(&combined), end - start))
}

/// Rejoin linked graveyard choices and their optional return when sentence
/// boundaries placed the three instructions in adjacent resolution segments.
/// The effect-list matcher verifies the shared tag, reciprocal chooser, zones,
/// cardinalities, destination, and controller before compacting the surface.
fn describe_cross_segment_linked_graveyard_choices_then_may_return_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let mut effects = Vec::with_capacity(3);
    let mut consumed = 0usize;
    for segment in segments.iter().skip(start).take(3) {
        if !segment.self_replacements.is_empty()
            || segment.default_effects.is_empty()
            || effects.len() + segment.default_effects.len() > 3
        {
            return None;
        }
        effects.extend(segment.default_effects.iter());
        consumed += 1;
        if effects.len() == 3 {
            break;
        }
    }
    if effects.len() != 3 {
        return None;
    }
    describe_linked_graveyard_choices_then_may_return_bundle(&effects)
        .map(|rendered| (rendered, consumed))
}

/// Rejoin a face-up exile choice with the following library exchange when
/// source sentence boundaries place the choice producer and its two linked
/// consumers in adjacent resolution segments. The flat matcher still proves
/// the exact zones, owner, count, tag, and top-of-library destination.
fn describe_cross_segment_choose_exiled_cards_exile_library_put_chosen_on_top_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [choice_segment, disposition_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !choice_segment.self_replacements.is_empty()
        || !disposition_segment.self_replacements.is_empty()
    {
        return None;
    }
    let [choose_effect] = choice_segment.default_effects.as_slice() else {
        return None;
    };
    let (exile_effect, move_effect) = match disposition_segment.default_effects.as_slice() {
        [exile_effect, move_effect] => (exile_effect, move_effect),
        [sequence_effect] => {
            let sequence = structural_unwrap_render_wrappers(sequence_effect)
                .downcast_ref::<crate::effects::SequenceEffect>()?;
            if sequence.surface != ironsmith_core::SequenceSurface::CommaThen {
                return None;
            }
            let [exile_effect, move_effect] = sequence.effects.as_slice() else {
                return None;
            };
            (exile_effect, move_effect)
        }
        _ => return None,
    };
    let effects = [
        choose_effect.clone(),
        exile_effect.clone(),
        move_effect.clone(),
    ];
    super::render_effects::describe_choose_exiled_cards_exile_library_put_chosen_on_top(&effects)
        .map(|rendered| (rendered, 2))
}

/// Rejoin a source-linked exile disposition with its following source
/// sacrifice when sentence lowering preserves the authored "then" pair as a
/// sequence in one segment and the final sentence in the next. The structural
/// matcher still proves the exact source-exiled filter, owner-relative
/// graveyard destination, affected-object count, token blueprint, and source
/// sacrifice.
fn describe_cross_segment_source_exiled_graveyard_token_sacrifice_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [disposition_segment, sacrifice_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !disposition_segment.self_replacements.is_empty()
        || !sacrifice_segment.self_replacements.is_empty()
    {
        return None;
    }
    let [sequence_effect] = disposition_segment.default_effects.as_slice() else {
        return None;
    };
    let sequence = structural_unwrap_render_wrappers(sequence_effect)
        .downcast_ref::<crate::effects::SequenceEffect>()?;
    if sequence.surface != ironsmith_core::SequenceSurface::CommaThen {
        return None;
    }
    let [move_effect, create_effect] = sequence.effects.as_slice() else {
        return None;
    };
    let [sacrifice_effect] = sacrifice_segment.default_effects.as_slice() else {
        return None;
    };

    let effects = [
        move_effect.clone(),
        create_effect.clone(),
        sacrifice_effect.clone(),
    ];
    super::render_effects::describe_source_exiled_graveyard_token_sacrifice_structural(&effects)
        .map(|rendered| (rendered, 2))
}

/// Rejoin a target declaration and its investigate action when sentence
/// lowering placed them in adjacent resolution segments. The existing effect
/// list matcher proves the target cardinality and the target-relative count;
/// this bridge only restores the context that matcher needs.
fn describe_cross_segment_target_players_investigate_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [target_segment, investigate_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !target_segment.self_replacements.is_empty()
        || !investigate_segment.self_replacements.is_empty()
    {
        return None;
    }
    let [target_effect] = target_segment.default_effects.as_slice() else {
        return None;
    };
    let [investigate_effect] = investigate_segment.default_effects.as_slice() else {
        return None;
    };
    let rendered = super::render_effects::compile_effect_list(&[
        target_effect.clone(),
        investigate_effect.clone(),
    ]);
    (rendered
        == "Choose any number of target players. Investigate X times, where X is the total number of creatures those players control")
        .then_some((rendered, 2))
}

fn describe_cross_segment_damage_and_die_replacement_program(
    program: &crate::resolution::ResolutionProgram,
) -> Option<String> {
    let [damage_segment, replacement_segment] = program.segments.as_slice() else {
        return None;
    };
    if !damage_segment.self_replacements.is_empty()
        || !replacement_segment.self_replacements.is_empty()
        || replacement_segment.default_effects.len() != 1
    {
        return None;
    }
    let mut combined = match damage_segment.default_effects.as_slice() {
        [first, second] => vec![first.clone(), second.clone()],
        [sequence_effect] => {
            let sequence = sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()?;
            if sequence.surface != ironsmith_core::SequenceSurface::Coordinated
                || sequence.effects.len() != 2
            {
                return None;
            }
            sequence.effects.clone()
        }
        _ => return None,
    };
    combined.extend(replacement_segment.default_effects.iter().cloned());
    let rendered = super::render_effects::compile_effect_list(&combined);
    rendered
        .contains("If that creature would die this turn, exile it instead")
        .then_some(rendered)
}

fn describe_cross_segment_countered_spell_exile_with_counters_gain_suspend_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let window = segments.get(start..start + 3)?;
    if window
        .iter()
        .any(|segment| !segment.self_replacements.is_empty())
    {
        return None;
    }
    let effects = window
        .iter()
        .map(|segment| match segment.default_effects.as_slice() {
            [effect] => Some(effect.clone()),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    let rendered = describe_separated_countered_spell_exile_with_counters_gain_suspend(&effects)?;
    Some((rendered, 3))
}

/// Rejoin a time-counter instruction with its exact tagged-object suspend
/// condition when authored sentence boundaries placed them in adjacent
/// resolution segments. The underlying helpers prove that the condition and
/// permanent grant reference the same single object that received the
/// counters; an optional preceding exile is accepted only when its tag is
/// that counter target.
fn describe_cross_segment_put_counters_then_gain_suspend_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let window = segments.get(start..start + 2)?;
    let effects = flattened_cross_segment_effects(window)?;
    let rendered = match effects.as_slice() {
        [put_effect, conditional_effect] => {
            super::render_effects::describe_put_counters_then_gain_suspend(&[
                put_effect.clone(),
                conditional_effect.clone(),
            ])
        }
        [exile_effect, put_effect, conditional_effect] => {
            super::render_effects::describe_exile_with_counters_then_gain_suspend(&[
                exile_effect.clone(),
                put_effect.clone(),
                conditional_effect.clone(),
            ])
        }
        _ => None,
    }?;
    Some((rendered, 2))
}

/// Rejoin a countered-spell result gate with the optional hand-to-battlefield
/// move it enables. Sentence lowering keeps the counter's result ID and object
/// tag executable across segments; this renderer accepts the window only when
/// the condition, decider, hand choice, and tagged move all refer to the
/// controller of that exact countered spell.
fn describe_cross_segment_countered_spell_may_put_from_hand_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [counter_segment, result_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !counter_segment.self_replacements.is_empty() || !result_segment.self_replacements.is_empty()
    {
        return None;
    }

    let [counter_effect] = counter_segment.default_effects.as_slice() else {
        return None;
    };
    let counter_with_id = counter_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let tagged_counter = counter_with_id
        .effect
        .downcast_ref::<crate::effects::TaggedEffect>()?;
    let counter = tagged_counter
        .effect
        .downcast_ref::<crate::effects::CounterEffect>()?;
    if counter.target != ChooseSpec::target(ChooseSpec::spell()) {
        return None;
    }

    let [result_effect] = result_segment.default_effects.as_slice() else {
        return None;
    };
    let result = result_effect.downcast_ref::<crate::effects::IfEffect>()?;
    let EffectPredicate::PriorEffectResult(surface) = &result.predicate else {
        return None;
    };
    if result.condition != counter_with_id.id
        || surface.action != crate::effect::PriorEffectAction::Countered
        || surface.actor != crate::effect::PriorEffectResultActor::Passive
        || surface.quantifier != crate::effect::PriorEffectResultQuantifier::One
        || !result.else_.is_empty()
    {
        return None;
    }

    let [may_effect] = result.then.as_slice() else {
        return None;
    };
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [choose_effect, move_effect] = may.effects.as_slice() else {
        return None;
    };
    let countered_ref = crate::filter::ObjectRef::Tagged(tagged_counter.tag.clone());
    let controller = PlayerFilter::ControllerOf(countered_ref.clone());
    let aliased_controller = PlayerFilter::AliasedControllerOf(countered_ref);
    if may.decider.as_ref() != Some(&controller) {
        return None;
    }

    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !choose.count.is_single()
        || choose.count_value.is_some()
        || choose.aggregate_constraint.is_some()
        || choose.chooser != aliased_controller
        || choose.zone != Some(Zone::Hand)
        || !choose.additional_zones.is_empty()
        || choose.is_search
        || choose.reveal
        || choose.top_only
        || choose.bottom_only
        || choose.filter.zone != Some(Zone::Hand)
        || choose.filter.owner.as_ref() != Some(&aliased_controller)
    {
        return None;
    }

    let move_to_zone = unwrap_basic_render_wrapper(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if !super::render_effects::move_to_battlefield_uses_chosen_tag(
        move_to_zone,
        choose.tag.as_str(),
    ) || move_to_zone.to_top
        || move_to_zone.library_order.is_some()
        || move_to_zone.verb_surface != ironsmith_core::MoveToZoneVerbSurface::Put
        || move_to_zone.actor_surface.as_ref() != Some(&controller)
        || move_to_zone.destination_player_surface.is_some()
        || move_to_zone.destination_player_reference_surface.is_some()
        || move_to_zone.battlefield_controller != crate::effects::BattlefieldController::Preserve
        || move_to_zone.controller_surface_explicit
        || !move_to_zone.enters_with_counters.is_empty()
        || move_to_zone.enters_tapped
        || move_to_zone.enters_attacking
        || move_to_zone.attack_target_mode.is_some()
        || move_to_zone.enters_face_down
        || move_to_zone.enters_transformed
        || move_to_zone.transfer_exiled_with_source_links
    {
        return None;
    }

    let mut choice_filter = choose.filter.clone();
    choice_filter.zone = None;
    choice_filter.owner = None;
    let choice_description = choice_filter.description();
    let choice_noun = strip_leading_article(&choice_description).trim();
    if choice_noun.is_empty() {
        return None;
    }
    let choice = with_indefinite_article(choice_noun);

    Some((
        format!(
            "Counter target spell. If that spell is countered this way, its controller may put {choice} from their hand onto the battlefield"
        ),
        2,
    ))
}

/// Rejoin a counter-placement producer with an adjacent exact-set consumer
/// when sentence lowering kept the authored sentence boundary as resolution
/// segments. The typed tag proves both effects refer to the same object set;
/// the effect-list matchers choose the appropriate demonstrative surface.
fn describe_cross_segment_countered_set_followup_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [producer_segment, consumer_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !producer_segment.self_replacements.is_empty()
        || !consumer_segment.self_replacements.is_empty()
    {
        return None;
    }
    let [producer] = producer_segment.default_effects.as_slice() else {
        return None;
    };
    let rendered = match consumer_segment.default_effects.as_slice() {
        [consumer] => {
            super::render_effects::describe_put_counters_then_untap_them(producer, consumer)
                .or_else(|| {
                    super::render_effects::describe_put_counters_then_gain_life_for_each_of_them(
                        producer, consumer,
                    )
                })
        }
        [tag_matching, consumer] => {
            super::render_effects::describe_put_counters_then_tag_matching_untap_them(
                producer,
                tag_matching,
                consumer,
            )
        }
        _ => None,
    }?;
    Some((rendered, 2))
}

fn direct_or_for_players_choice_tag(effect: &Effect) -> Option<&TagKey> {
    if let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() {
        return Some(&choose.tag);
    }
    let for_players = effect.downcast_ref::<crate::effects::ForPlayersEffect>()?;
    let [choose_effect] = for_players.effects.as_slice() else {
        return None;
    };
    choose_effect
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()
        .map(|choose| &choose.tag)
}

fn describe_random_target_opponent_graveyard_choice_to_your_battlefield(
    choice_segment: &crate::resolution::ResolutionSegment,
    move_to_zone: &crate::effects::MoveToZoneEffect,
    moved_tag: &TagKey,
) -> Option<String> {
    let [choice_effect] = choice_segment.default_effects.as_slice() else {
        return None;
    };
    let choose = choice_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;

    let mut creature_card = ObjectFilter::default().with_type(CardType::Creature);
    creature_card.zone = Some(Zone::Graveyard);
    creature_card.owner = Some(PlayerFilter::Target(Box::new(PlayerFilter::Opponent)));
    creature_card.set_explicit_card_noun(true);
    creature_card.set_explicit_card_type_noun(Some(CardType::Creature));
    let expected_choice = crate::effects::ChooseObjectsEffect::new(
        creature_card,
        ChoiceCount::exactly(1).at_random(),
        PlayerFilter::You,
        moved_tag.clone(),
    )
    .in_zone(Zone::Graveyard);
    if choose != &expected_choice {
        return None;
    }

    let expected_move = crate::effects::MoveToZoneEffect::new(
        ChooseSpec::Tagged(moved_tag.clone()),
        Zone::Battlefield,
        false,
    )
    .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put)
    .under_you_control();
    if move_to_zone != &expected_move {
        return None;
    }

    Some(
        "Choose a creature card at random from target opponent's graveyard. Put that card onto the battlefield under your control"
            .to_string(),
    )
}

fn describe_target_opponent_graveyard_choice_to_your_battlefield(
    choice_segment: &crate::resolution::ResolutionSegment,
    move_to_zone: &crate::effects::MoveToZoneEffect,
    moved_tag: &TagKey,
) -> Option<String> {
    let [target_effect, choose_effect] = choice_segment.default_effects.as_slice() else {
        return None;
    };
    let target = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if !target.target.is_target()
        || !matches!(
            target.target.base(),
            ChooseSpec::Player(PlayerFilter::Opponent)
        )
    {
        return None;
    }
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !choose.count.is_single()
        || choose.tag != *moved_tag
        || choose.chooser != PlayerFilter::Target(Box::new(PlayerFilter::Opponent))
        || choose.filter.zone != Some(Zone::Graveyard)
        || choose.filter.owner != Some(PlayerFilter::IteratedPlayer)
        || choose.is_search
        || choose.reveal
        || move_to_zone.zone != Zone::Battlefield
        || move_to_zone.to_top
        || move_to_zone.battlefield_controller != crate::effects::BattlefieldController::You
        || !move_to_zone.enters_with_counters.is_empty()
        || move_to_zone.enters_tapped
        || move_to_zone.enters_attacking
        || move_to_zone.enters_face_down
    {
        return None;
    }

    let choice_text = capitalize_first(
        &describe_effect_list(&choice_segment.default_effects)
            .replace(" in a graveyard", " in their graveyard"),
    );
    choice_text
        .starts_with("Target opponent chooses ")
        .then(|| format!("{choice_text}. Put that card onto the battlefield under your control"))
}

/// Preserve a choice's player/zone surface when its tagged disposition was
/// lowered into the following sentence. This is deliberately limited to one
/// choice producer and one tag-linked move, so unrelated sentence boundaries
/// remain intact.
fn describe_cross_segment_choice_move_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [choice_segment, move_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !choice_segment.self_replacements.is_empty() || !move_segment.self_replacements.is_empty() {
        return None;
    }
    let [move_effect] = move_segment.default_effects.as_slice() else {
        return None;
    };
    let move_to_zone = unwrap_basic_render_wrapper(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let ChooseSpec::Tagged(moved_tag) = move_to_zone.target.base() else {
        return None;
    };

    if let Some(rendered) = describe_random_target_opponent_graveyard_choice_to_your_battlefield(
        choice_segment,
        move_to_zone,
        moved_tag,
    ) {
        return Some((rendered, 2));
    }

    if let Some(rendered) = describe_target_opponent_graveyard_choice_to_your_battlefield(
        choice_segment,
        move_to_zone,
        moved_tag,
    ) {
        return Some((rendered, 2));
    }

    let choice_tags = choice_segment
        .default_effects
        .iter()
        .filter_map(direct_or_for_players_choice_tag)
        .collect::<Vec<_>>();
    if choice_tags.len() != 1 || choice_tags[0] != moved_tag {
        return None;
    }
    if choice_segment.default_effects.iter().any(|effect| {
        effect
            .downcast_ref::<crate::effects::TargetOnlyEffect>()
            .is_none()
            && direct_or_for_players_choice_tag(effect).is_none()
    }) {
        return None;
    }

    let combined = choice_segment
        .default_effects
        .iter()
        .chain(move_segment.default_effects.iter())
        .cloned()
        .collect::<Vec<_>>();
    Some((super::render_effects::compile_effect_list(&combined), 2))
}

#[cfg(test)]
mod random_graveyard_choice_move_tests {
    use super::*;

    fn choice_segment(tag: &TagKey) -> crate::resolution::ResolutionSegment {
        let mut creature_card = ObjectFilter::default().with_type(CardType::Creature);
        creature_card.zone = Some(Zone::Graveyard);
        creature_card.owner = Some(PlayerFilter::Target(Box::new(PlayerFilter::Opponent)));
        creature_card.set_explicit_card_noun(true);
        creature_card.set_explicit_card_type_noun(Some(CardType::Creature));
        crate::resolution::ResolutionSegment::from_effects(vec![Effect::new(
            crate::effects::ChooseObjectsEffect::new(
                creature_card,
                ChoiceCount::exactly(1).at_random(),
                PlayerFilter::You,
                tag.clone(),
            )
            .in_zone(Zone::Graveyard),
        )])
    }

    fn battlefield_move(tag: &TagKey) -> crate::effects::MoveToZoneEffect {
        crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Tagged(tag.clone()),
            Zone::Battlefield,
            false,
        )
        .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put)
        .under_you_control()
    }

    #[test]
    fn random_graveyard_choice_keeps_that_card_only_for_the_exact_tag() {
        let chosen = TagKey::from("chosen");
        assert_eq!(
            describe_random_target_opponent_graveyard_choice_to_your_battlefield(
                &choice_segment(&chosen),
                &battlefield_move(&chosen),
                &chosen,
            )
            .as_deref(),
            Some(
                "Choose a creature card at random from target opponent's graveyard. Put that card onto the battlefield under your control"
            )
        );

        let unrelated = TagKey::from("unrelated");
        assert!(
            describe_random_target_opponent_graveyard_choice_to_your_battlefield(
                &choice_segment(&chosen),
                &battlefield_move(&unrelated),
                &unrelated,
            )
            .is_none()
        );
    }
}

fn effect_contains_zone_replacement(effect: &Effect) -> bool {
    if effect
        .downcast_ref::<crate::effects::RegisterZoneReplacementEffect>()
        .is_some()
    {
        return true;
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return effect_contains_zone_replacement(&tagged.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return effect_contains_zone_replacement(&with_id.effect);
    }
    if let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() {
        return conditional
            .if_true
            .iter()
            .chain(&conditional.if_false)
            .any(effect_contains_zone_replacement);
    }
    false
}

fn describe_cross_segment_death_replacement_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    if segments.len().saturating_sub(start) < 2 {
        return None;
    }

    let mut effects = Vec::new();
    for (offset, segment) in segments[start..].iter().enumerate() {
        if !segment.self_replacements.is_empty() || segment.default_effects.is_empty() {
            break;
        }
        let contains_replacement = segment
            .default_effects
            .iter()
            .any(effect_contains_zone_replacement);
        if offset == 0 && contains_replacement {
            break;
        }
        effects.extend(segment.default_effects.iter().cloned());
        if offset == 0 {
            continue;
        }
        if let Some(rendered) = describe_cross_segment_death_replacement_bundle(&effects) {
            return Some((rendered, offset + 1));
        }
        if contains_replacement {
            break;
        }
    }
    None
}

fn effect_consult_count_for_window(effect: &Effect) -> usize {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return effect_consult_count_for_window(&with_id.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return effect_consult_count_for_window(&tagged.effect);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return effect_consult_count_for_window(&tag_all.effect);
    }
    if effect
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()
        .is_some()
    {
        return 1;
    }
    effect
        .downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
        .map(|schedule| {
            schedule
                .effects
                .flattened_default_effects()
                .iter()
                .map(effect_consult_count_for_window)
                .sum()
        })
        .unwrap_or(0)
}

fn describe_cross_segment_consult_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let mut effects = Vec::new();
    let mut consult_count = 0usize;
    for (offset, segment) in segments[start..].iter().take(5).enumerate() {
        if !segment.self_replacements.is_empty() || segment.default_effects.is_empty() {
            break;
        }
        consult_count += segment
            .default_effects
            .iter()
            .map(effect_consult_count_for_window)
            .sum::<usize>();
        if consult_count > 1 {
            break;
        }
        effects.extend(segment.default_effects.iter().cloned());
        if offset == 0 {
            continue;
        }
        if let Some(rendered) = describe_cross_segment_consult_bundle(&effects) {
            return Some((rendered, offset + 1));
        }
    }
    None
}

fn describe_cross_segment_turn_start_hand_conditions_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [first, second] = segments.get(start..start + 2)? else {
        return None;
    };
    if !first.self_replacements.is_empty() || !second.self_replacements.is_empty() {
        return None;
    }
    let [first_effect] = first.default_effects.as_slice() else {
        return None;
    };
    let [second_effect] = second.default_effects.as_slice() else {
        return None;
    };
    let effects = [first_effect.clone(), second_effect.clone()];
    describe_turn_start_hand_condition_effects(&effects).map(|rendered| (rendered, 2))
}

fn describe_cross_segment_choose_pay_untap_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [choice, untap] = segments.get(start..start + 2)? else {
        return None;
    };
    if !choice.self_replacements.is_empty() || !untap.self_replacements.is_empty() {
        return None;
    }
    let [choice_effect] = choice.default_effects.as_slice() else {
        return None;
    };
    let [untap_effect] = untap.default_effects.as_slice() else {
        return None;
    };
    describe_may_choose_pay_for_each_then_untap_tagged(&[choice_effect, untap_effect])
        .map(|rendered| (rendered, 2))
}

/// Rejoin a searched-card conditional whose optional battlefield move,
/// declined move, and shuffle were kept in separate source-sentence segments.
/// The existing flat structural renderer still proves the searched tag,
/// condition, effect ID, both hand fallbacks, destination, and shuffle scope.
fn describe_cross_segment_search_reveal_may_move_else_hand_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [
        search_segment,
        conditional_segment,
        fallback_segment,
        shuffle_segment,
    ] = segments.get(start..start + 4)?
    else {
        return None;
    };
    if [
        search_segment,
        conditional_segment,
        fallback_segment,
        shuffle_segment,
    ]
    .iter()
    .any(|segment| !segment.self_replacements.is_empty())
    {
        return None;
    }

    let [search_effect] = search_segment.default_effects.as_slice() else {
        return None;
    };
    let search_sequence = search_effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    let [choose_effect, reveal_effect] = search_sequence.effects.as_slice() else {
        return None;
    };

    let [conditional_effect] = conditional_segment.default_effects.as_slice() else {
        return None;
    };
    let with_id = conditional_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let conditional = with_id
        .effect
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    let [may_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    if !conditional.if_false.is_empty()
        || may_effect
            .downcast_ref::<crate::effects::MayEffect>()
            .is_none()
    {
        return None;
    }

    let [fallback_effect] = fallback_segment.default_effects.as_slice() else {
        return None;
    };
    let fallback = fallback_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if fallback.condition != with_id.id
        || fallback.predicate != EffectPredicate::DidNotHappen
        || !fallback.else_.is_empty()
        || fallback.then.len() != 1
    {
        return None;
    }
    let [shuffle_effect] = shuffle_segment.default_effects.as_slice() else {
        return None;
    };

    let mut merged = conditional.clone();
    merged.if_true = vec![
        Effect::with_id(with_id.id.0, may_effect.clone()),
        fallback_effect.clone(),
    ];
    merged.if_false = fallback.then.clone();
    let flat = [
        choose_effect.clone(),
        reveal_effect.clone(),
        Effect::new(merged),
        shuffle_effect.clone(),
    ];
    describe_search_reveal_conditional_battlefield_or_hand(&flat).map(|rendered| (rendered, 4))
}

/// Rejoin the dynamic kicked target declaration with its per-target counter
/// action when source-sentence preservation places them in adjacent segments.
/// The existing structural helper proves the 1 + kick-count target cardinality,
/// shared tag, iterated target, and counter amount before we cross the boundary.
fn describe_cross_segment_kicked_targets_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [target_segment, counter_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !target_segment.self_replacements.is_empty() || !counter_segment.self_replacements.is_empty()
    {
        return None;
    }
    let effects = target_segment
        .default_effects
        .iter()
        .chain(&counter_segment.default_effects)
        .collect::<Vec<_>>();
    describe_kicked_additional_targets_put_counters(&effects).map(|rendered| (rendered, 2))
}

/// Preserve the authored choose/conditional-prevention sentence when source
/// sentence boundaries place its producer and consumer in adjacent segments.
fn describe_cross_segment_color_matched_prevention_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [choose_segment, prevention_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !choose_segment.self_replacements.is_empty()
        || !prevention_segment.self_replacements.is_empty()
    {
        return None;
    }
    let [choose_effect] = choose_segment.default_effects.as_slice() else {
        return None;
    };
    let [prevention_effect] = prevention_segment.default_effects.as_slice() else {
        return None;
    };
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let conditional = structural_unwrap_render_wrappers(prevention_effect)
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    describe_choose_then_color_matched_combat_prevention(choose, conditional)
        .map(|rendered| (rendered, 2))
}

/// Rejoin an authored standalone player-target declaration with the damage
/// sentence that consumes that exact target. The established helper proves
/// the player filters are identical before introducing "that player."
fn describe_cross_segment_target_only_damage_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [target_segment, damage_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !target_segment.self_replacements.is_empty() || !damage_segment.self_replacements.is_empty()
    {
        return None;
    }
    let [target_effect] = target_segment.default_effects.as_slice() else {
        return None;
    };
    let [damage_effect] = damage_segment.default_effects.as_slice() else {
        return None;
    };
    let target = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let damage = structural_unwrap_render_wrappers(damage_effect)
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    describe_target_only_then_damage_that_player(target, damage).map(|rendered| (rendered, 2))
}

/// Rejoin damage to a tagged creature with the adjacent destruction of
/// attachments anchored to that exact target.
fn describe_cross_segment_target_creature_damage_then_destroy_attached_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [damage_segment, destroy_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !damage_segment.self_replacements.is_empty() || !destroy_segment.self_replacements.is_empty()
    {
        return None;
    }
    let [damage_effect] = damage_segment.default_effects.as_slice() else {
        return None;
    };
    let [destroy_effect] = destroy_segment.default_effects.as_slice() else {
        return None;
    };
    let effects = [damage_effect, destroy_effect];
    super::render_effects::describe_target_creature_damage_then_destroy_attached(&effects)
        .map(|rendered| (rendered, 2))
}

fn exact_creatures_blocked_by_tag_filter(filter: &ObjectFilter, tag: &TagKey) -> bool {
    let mut expected = ObjectFilter::creature();
    expected.blocked = true;
    expected.blocked_by = Some(crate::filter::ObjectRef::Tagged(tag.clone()));
    filter == &expected
}

/// Rejoin destruction of a blocking creature with the exact historical set
/// that creature blocked. The destroyed-object tag is the executable identity
/// edge; rendering it as an arbitrary plural tagged set loses the authored
/// singular antecedent.
fn describe_cross_segment_destroy_then_grant_blocked_by_that_creature_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [destroy_segment, grant_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !destroy_segment.self_replacements.is_empty()
        || !grant_segment.self_replacements.is_empty()
        || destroy_segment.starts_new_source_line
        || grant_segment.starts_new_source_line
    {
        return None;
    }

    let [destroy_effect] = destroy_segment.default_effects.as_slice() else {
        return None;
    };
    let tagged_destroy = destroy_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let destroy = tagged_destroy
        .effect
        .downcast_ref::<crate::effects::DestroyEffect>()?;
    let mut blocking_creature = ObjectFilter::creature();
    blocking_creature.blocking = true;
    if destroy.spec != ChooseSpec::target(ChooseSpec::Object(blocking_creature)) {
        return None;
    }

    let [grant_effect] = grant_segment.default_effects.as_slice() else {
        return None;
    };
    let tagged_grant = grant_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let grant = tagged_grant
        .effect
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let crate::continuous::EffectTarget::Filter(granted_filter) = &grant.target else {
        return None;
    };
    let crate::continuous::Modification::AddAbility(ability) = grant.modification.as_ref()? else {
        return None;
    };
    if !exact_creatures_blocked_by_tag_filter(granted_filter, &tagged_destroy.tag)
        || ability.id() != crate::static_abilities::StaticAbilityId::Trample
        || grant.target_spec.is_some()
        || !grant.additional_modifications.is_empty()
        || !grant.runtime_modifications.is_empty()
        || grant.until != Until::EndOfTurn
        || grant.condition.is_some()
        || grant.source_type.is_some()
        || grant.source_reference_surface.is_some()
        || grant.set_quantifier_surface.is_some()
        || grant.type_retention_surface.is_some()
        || grant.animation_pt_surface.is_some()
        || grant.animation_duration_surface.is_some()
        || !grant.lock_filter_at_resolution
        || grant.resolve_set_pt_values_at_resolution
        || grant.require_creature_target
    {
        return None;
    }

    Some((
        "Destroy target blocking creature. Creatures that were blocked by that creature this combat gain trample until end of turn".to_string(),
        2,
    ))
}

/// Preserve the singular target identity through an end-of-combat delayed
/// trigger. The nested destroy filter must point to the exact standalone
/// target declaration, not merely any tagged object set.
fn describe_cross_segment_target_then_delayed_destroy_blocked_by_that_creature_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [target_segment, delayed_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !target_segment.self_replacements.is_empty()
        || !delayed_segment.self_replacements.is_empty()
        || target_segment.starts_new_source_line
        || delayed_segment.starts_new_source_line
    {
        return None;
    }

    let [target_effect] = target_segment.default_effects.as_slice() else {
        return None;
    };
    let tagged_target = target_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let target = tagged_target
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let wall = ObjectFilter::creature().with_subtype(Subtype::Wall);
    if target.target != ChooseSpec::target(ChooseSpec::Object(wall))
        || target.chooser.is_some()
        || !target.explicit_declaration
    {
        return None;
    }

    let [delayed_effect] = delayed_segment.default_effects.as_slice() else {
        return None;
    };
    let delayed = delayed_effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()?;
    if delayed
        .trigger
        .downcast_ref::<crate::triggers::EndOfCombatTrigger>()
        .is_none()
        || !delayed.one_shot
        || delayed.start_next_turn
        || delayed.duration != ironsmith_core::DelayedTriggerDuration::Forever
        || delayed.until_end_of_turn
        || delayed.until_end_of_combat
        || delayed.leading_duration_surface
        || delayed.watch_ability_source
        || delayed.watch_all_object_targets
        || delayed.either_of_watched_objects
        || delayed.while_any_tagged_object_in_zone.is_some()
        || !delayed.target_objects.is_empty()
        || delayed.target_tag.is_some()
        || delayed.target_filter.is_some()
        || delayed.controller != PlayerFilter::You
    {
        return None;
    }
    let [destroy_segment] = delayed.effects.segments.as_slice() else {
        return None;
    };
    if !destroy_segment.self_replacements.is_empty() || destroy_segment.starts_new_source_line {
        return None;
    }
    let [destroy_effect] = destroy_segment.default_effects.as_slice() else {
        return None;
    };
    let tagged_destroy = destroy_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let destroy = tagged_destroy
        .effect
        .downcast_ref::<crate::effects::DestroyEffect>()?;
    let ChooseSpec::All(destroyed_filter) = destroy.spec.base() else {
        return None;
    };
    if !exact_creatures_blocked_by_tag_filter(destroyed_filter, &tagged_target.tag) {
        return None;
    }

    Some((
        "Choose target Wall creature. At this turn's next end of combat, destroy all creatures that were blocked by that creature this turn".to_string(),
        2,
    ))
}

/// Rejoin the two authored sentences of reciprocal power-based damage. The
/// structural matcher verifies the shared target tag, both damage sources,
/// both destinations, and both power references.
fn describe_cross_segment_power_damage_exchange_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    fn collect(effect: &Effect, flattened: &mut Vec<Effect>) {
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            collect(&with_id.effect, flattened);
        } else if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
            for member in &sequence.effects {
                collect(member, flattened);
            }
        } else {
            flattened.push(effect.clone());
        }
    }

    let [first_segment, reciprocal_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !first_segment.self_replacements.is_empty()
        || !reciprocal_segment.self_replacements.is_empty()
    {
        return None;
    }
    if let [conditional_effect] = first_segment.default_effects.as_slice()
        && let Some(conditional) =
            conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()
        && conditional.surface == ironsmith_core::ConditionalSurface::LeadingIf
        && conditional.if_false.is_empty()
        && let [first_target_effect, first_damage_effect] = conditional.if_true.as_slice()
        && let Some(first_target) =
            first_target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()
        && let [tagged_target_effect, reciprocal_effect] =
            reciprocal_segment.default_effects.as_slice()
        && let Some(tagged_target) =
            tagged_target_effect.downcast_ref::<crate::effects::TaggedEffect>()
        && let Some(reciprocal_target) = tagged_target
            .effect
            .downcast_ref::<crate::effects::TargetOnlyEffect>()
        && first_target.target.unhinted() == reciprocal_target.target.unhinted()
    {
        let normalized = [
            first_damage_effect.clone(),
            tagged_target_effect.clone(),
            reciprocal_effect.clone(),
        ];
        if let Some(rendered) = describe_power_damage_exchange_clause(&normalized)
            && let Some((_, reciprocal_clause)) = rendered.split_once(", then ")
        {
            let first_sentence = describe_effect(conditional_effect);
            return Some((
                format!(
                    "{}. {}",
                    first_sentence.trim().trim_end_matches('.'),
                    capitalize_first(reciprocal_clause)
                ),
                2,
            ));
        }
    }
    let mut flattened = Vec::new();
    for effect in first_segment
        .default_effects
        .iter()
        .chain(&reciprocal_segment.default_effects)
    {
        collect(effect, &mut flattened);
    }
    let rendered = describe_power_damage_exchange_clause(&flattened)?;
    let (first_sentence, second_sentence) = rendered.split_once(", then ")?;
    Some((
        format!("{first_sentence}. {}", capitalize_first(second_sentence)),
        2,
    ))
}

/// Rejoin an optional search/reveal/shuffle procedure with a trailing
/// condition and its `otherwise` disposition. Lowering gives the condition an
/// effect ID so the following `DidNotHappen` branch is executable; combine
/// those two runtime nodes only after proving they share that exact ID.
fn describe_cross_segment_may_search_conditional_disposition_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [search_segment, conditional_segment, fallback_segment] = segments.get(start..start + 3)?
    else {
        return None;
    };
    if [search_segment, conditional_segment, fallback_segment]
        .iter()
        .any(|segment| !segment.self_replacements.is_empty())
    {
        return None;
    }

    let [search_effect] = search_segment.default_effects.as_slice() else {
        return None;
    };
    let may = search_effect.downcast_ref::<crate::effects::MayEffect>()?;

    let [conditional_effect] = conditional_segment.default_effects.as_slice() else {
        return None;
    };
    let conditional_with_id = conditional_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let conditional = conditional_with_id
        .effect
        .downcast_ref::<crate::effects::ConditionalEffect>()?;

    let [fallback_effect] = fallback_segment.default_effects.as_slice() else {
        return None;
    };
    let fallback = fallback_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if fallback.condition != conditional_with_id.id
        || fallback.predicate != EffectPredicate::DidNotHappen
        || !fallback.else_.is_empty()
        || fallback.then.is_empty()
        || !conditional.if_false.is_empty()
    {
        return None;
    }

    let mut combined = conditional.clone();
    combined.if_false = fallback.then.clone();
    describe_may_search_reveal_shuffle_then_conditional_move(may, &combined)
        .map(|rendered| (rendered, 3))
}

/// Rejoin the named-card search branch used by effects such as Nazahn's. The
/// producer sequence, typed reveal predicate, conditional result ID,
/// `otherwise` branch, searched-object tag, and final shuffle must all agree
/// before the authored four-sentence surface is restored.
fn describe_cross_segment_named_search_conditional_disposition_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [
        search_segment,
        conditional_segment,
        fallback_segment,
        shuffle_segment,
    ] = segments.get(start..start + 4)?
    else {
        return None;
    };
    if [
        search_segment,
        conditional_segment,
        fallback_segment,
        shuffle_segment,
    ]
    .iter()
    .any(|segment| !segment.self_replacements.is_empty())
    {
        return None;
    }

    let [search_effect] = search_segment.default_effects.as_slice() else {
        return None;
    };
    let search_with_id = search_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let search_sequence = search_with_id
        .effect
        .downcast_ref::<crate::effects::SequenceEffect>()?;
    let [choose_effect, reveal_effect] = search_sequence.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let reveal = reveal_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()?;

    let [conditional_effect] = conditional_segment.default_effects.as_slice() else {
        return None;
    };
    let conditional_with_id = conditional_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let conditional = conditional_with_id
        .effect
        .downcast_ref::<crate::effects::IfEffect>()?;
    let EffectPredicate::PriorEffectResult(surface) = &conditional.predicate else {
        return None;
    };
    if conditional.condition != search_with_id.id
        || !conditional.else_.is_empty()
        || surface.action != ironsmith_core::PriorEffectAction::Revealed
        || surface.actor != ironsmith_core::PriorEffectResultActor::You
        || surface.quantifier != ironsmith_core::PriorEffectResultQuantifier::One
    {
        return None;
    }

    let [fallback_effect] = fallback_segment.default_effects.as_slice() else {
        return None;
    };
    let fallback = fallback_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if fallback.condition != conditional_with_id.id
        || fallback.predicate != EffectPredicate::DidNotHappen
        || !fallback.else_.is_empty()
    {
        return None;
    }

    let [shuffle_effect] = shuffle_segment.default_effects.as_slice() else {
        return None;
    };
    let shuffle_effect = shuffle_effect
        .downcast_ref::<crate::effects::SequenceEffect>()
        .and_then(|sequence| match sequence.effects.as_slice() {
            [shuffle] => Some(shuffle),
            _ => None,
        })
        .unwrap_or(shuffle_effect);
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;

    let combined = crate::effects::ConditionalEffect::new(
        Condition::TaggedObjectMatches(choose.tag.clone(), surface.filter.clone()),
        conditional.then.clone(),
        fallback.then.clone(),
    );
    describe_search_reveal_named_conditional_move_then_shuffle(choose, reveal, &combined, shuffle)
        .map(|rendered| (rendered, 4))
}

fn describe_cross_segment_consult_hand_remainder_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [consult_segment, disposition_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !consult_segment.self_replacements.is_empty()
        || !disposition_segment.self_replacements.is_empty()
    {
        return None;
    }
    let [consult_effect] = consult_segment.default_effects.as_slice() else {
        return None;
    };
    let consult = structural_unwrap_render_wrappers(consult_effect)
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if consult.player != PlayerFilter::You
        || consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
        || !matches!(
            consult.stop_rule,
            crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
                | crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1))
        )
    {
        return None;
    }

    let [sequence_effect] = disposition_segment.default_effects.as_slice() else {
        return None;
    };
    let sequence = structural_unwrap_render_wrappers(sequence_effect)
        .downcast_ref::<crate::effects::SequenceEffect>()?;
    let [move_match_effect, remainder_effect] = sequence.effects.as_slice() else {
        return None;
    };
    let move_match = structural_unwrap_render_wrappers(move_match_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_match.zone != Zone::Hand
        || move_match.to_top
        || !matches!(
            move_match.target.base(),
            ChooseSpec::Tagged(tag) if tag == &consult.match_tag
        )
    {
        return None;
    }

    let remainder = structural_unwrap_render_wrappers(remainder_effect)
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if remainder.tag != consult.all_tag {
        return None;
    }
    let nested = if let [nested_sequence] = remainder.effects.as_slice()
        && let Some(nested_sequence) = structural_unwrap_render_wrappers(nested_sequence)
            .downcast_ref::<crate::effects::SequenceEffect>()
    {
        nested_sequence.effects.as_slice()
    } else {
        remainder.effects.as_slice()
    };
    let [conditional_effect] = nested else {
        return None;
    };
    let conditional = structural_unwrap_render_wrappers(conditional_effect)
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !condition_matches_tagged_object_membership(
        &conditional.condition,
        consult.match_tag.as_str(),
    ) || !conditional.if_true.is_empty()
    {
        return None;
    }
    let [move_remainder_effect] = conditional.if_false.as_slice() else {
        return None;
    };
    let move_remainder = structural_unwrap_render_wrappers(move_remainder_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if !matches!(move_remainder.target.base(), ChooseSpec::Iterated)
        && !matches!(
            move_remainder.target.base(),
            ChooseSpec::Tagged(tag) if tag.as_str() == "__it__"
        )
    {
        return None;
    }
    let disposition = match move_remainder.zone {
        Zone::Graveyard if !move_remainder.to_top => {
            "put that card into your hand and all other cards revealed this way into your graveyard"
        }
        Zone::Exile => "put that card into your hand and exile all other cards revealed this way",
        _ => return None,
    };
    let rendered_consult = describe_effect(consult_effect);
    let rendered_consult = rendered_consult.trim().trim_end_matches('.');
    let consult_text = rendered_consult
        .strip_prefix("you ")
        .or_else(|| rendered_consult.strip_prefix("You "))
        .map(capitalize_first)
        .unwrap_or_else(|| capitalize_first(rendered_consult));
    Some((
        format!("{consult_text}. {}", capitalize_first(disposition)),
        2,
    ))
}

/// Rejoin a reveal-hand sentence, its two same-set choices, and the trailing
/// collection move when source sentence preservation splits them into three
/// resolution segments. The established structural helper verifies both
/// zones, the shared opponent, the shared result tag, and the exile consumer.
fn describe_cross_segment_reveal_hand_choose_graveyard_exile_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [look_segment, choose_segment, exile_segment] = segments.get(start..start + 3)? else {
        return None;
    };
    if [look_segment, choose_segment, exile_segment]
        .iter()
        .any(|segment| !segment.self_replacements.is_empty())
    {
        return None;
    }
    let [look] = look_segment.default_effects.as_slice() else {
        return None;
    };
    let [hand_choose, graveyard_choose] = choose_segment.default_effects.as_slice() else {
        return None;
    };
    let [exile] = exile_segment.default_effects.as_slice() else {
        return None;
    };
    describe_reveal_hand_choose_graveyard_exile_bundle(&[
        look,
        hand_choose,
        graveyard_choose,
        exile,
    ])
    .map(|rendered| (rendered, 3))
}

fn append_flattened_cross_segment_effect(effect: &Effect, flattened: &mut Vec<Effect>) {
    let unwrapped = structural_unwrap_render_wrappers(effect);
    if let Some(sequence) = unwrapped.downcast_ref::<crate::effects::SequenceEffect>() {
        for member in &sequence.effects {
            append_flattened_cross_segment_effect(member, flattened);
        }
    } else {
        flattened.push(effect.clone());
    }
}

fn flattened_cross_segment_effects(
    segments: &[crate::resolution::ResolutionSegment],
) -> Option<Vec<Effect>> {
    let mut flattened = Vec::new();
    for segment in segments {
        if !segment.self_replacements.is_empty() {
            return None;
        }
        for effect in &segment.default_effects {
            append_flattened_cross_segment_effect(effect, &mut flattened);
        }
    }
    Some(flattened)
}

/// Rejoin a target-plus-fanout collection and a later action over the exact
/// captured union when an authored sentence boundary split the effects into
/// adjacent resolution segments (for example, Radiance followed by "Those
/// creatures ..."). The underlying matcher validates every producer,
/// relation, collection tag, and consumer before any prose is combined.
fn describe_cross_segment_linked_target_set_followup_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let max_consumed = segments.len().saturating_sub(start).min(4);
    for consumed in 2..=max_consumed {
        let window = segments.get(start..start + consumed)?;
        let flattened = flattened_cross_segment_effects(window)?;
        if let Some((rendered, effect_count)) =
            describe_linked_target_set_followup_prefix(&flattened)
                .or_else(|| describe_same_name_exile_then_investigate_prefix(&flattened))
            && effect_count == flattened.len()
        {
            return Some((rendered, consumed));
        }
    }
    None
}

/// Preserve the producer's typed object noun for a later per-object consumer
/// even when source sentence boundaries converted `ForEachTagged` into an
/// equivalent exact-tag `ForEachObject` in the next segment.
fn describe_cross_segment_result_producer_for_each_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let window = segments.get(start..start + 2)?;
    let flattened = flattened_cross_segment_effects(window)?;
    let [producer, for_each] = flattened.as_slice() else {
        return None;
    };
    describe_result_producer_then_for_each_tagged(producer, for_each).map(|rendered| (rendered, 2))
}

/// Rejoin a searched-and-revealed collection, an opponent's division of that
/// collection, and the two destinations when authored sentence boundaries put
/// the linked effects in separate resolution segments. The underlying bundle
/// matcher validates every tag, count, zone, chooser, and optional source-exile
/// effect before it produces text.
fn describe_cross_segment_search_reveal_opponent_choose_rest_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let max_consumed = segments.len().saturating_sub(start).min(8);
    for consumed in (1..=max_consumed).rev() {
        let window = segments.get(start..start + consumed)?;
        let flattened = flattened_cross_segment_effects(window)?;
        let refs = flattened.iter().collect::<Vec<_>>();
        if let Some(rendered) = render_search_reveal_opponent_choose_rest_bundle(&refs) {
            return Some((rendered, consumed));
        }
    }
    None
}

/// Rejoin an optional action with its exact declined branch when authored
/// sentence boundaries place the `WithId` producer and `If` consumer in
/// adjacent resolution segments.
fn describe_cross_segment_declined_may_mill_damage_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [producer_segment, branch_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !producer_segment.self_replacements.is_empty()
        || !branch_segment.self_replacements.is_empty()
    {
        return None;
    }
    let with_ids = producer_segment
        .default_effects
        .iter()
        .filter_map(|effect| effect.downcast_ref::<crate::effects::WithIdEffect>())
        .collect::<Vec<_>>();
    let [with_id] = with_ids.as_slice() else {
        return None;
    };
    let [branch_effect] = branch_segment.default_effects.as_slice() else {
        return None;
    };
    let if_effect = branch_effect.downcast_ref::<crate::effects::IfEffect>()?;
    let declined = describe_declined_may_mill_then_damage(with_id, if_effect)?;
    let setup = describe_effect_list(&producer_segment.default_effects);
    (!setup.trim().is_empty()).then(|| {
        (
            format!(
                "{}. {}",
                setup.trim().trim_end_matches('.'),
                declined.trim().trim_end_matches('.')
            ),
            2,
        )
    })
}

/// Rejoin a hand reveal with a graveyard-or-hand choice when source sentence
/// boundaries place the reveal and the tag-linked choice/move in adjacent
/// resolution segments.
fn describe_cross_segment_reveal_hand_choose_graveyard_or_hand_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let window = segments.get(start..start + 2)?;
    let flattened = flattened_cross_segment_effects(window)?;
    let refs = flattened.iter().collect::<Vec<_>>();
    describe_reveal_hand_choose_graveyard_or_hand_exile(&refs).map(|rendered| (rendered, 2))
}

/// Rejoin a standalone color choice with the following reveal-and-discard
/// sentence when both use the same chosen-color state.
fn describe_cross_segment_choose_color_reveal_discard_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let window = segments.get(start..start + 2)?;
    let flattened = flattened_cross_segment_effects(window)?;
    let refs = flattened.iter().collect::<Vec<_>>();
    describe_choose_color_reveal_hand_discard_that_color(&refs).map(|rendered| (rendered, 2))
}

/// Rejoin a revealed hand with a tag-linked choose/discard-or-exile action
/// when the authored sentence boundary leaves the reveal in its own segment.
fn describe_cross_segment_look_hand_choose_action_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let window = segments.get(start..start + 2)?;
    let flattened = flattened_cross_segment_effects(window)?;
    let refs = flattened.iter().collect::<Vec<_>>();
    describe_look_hand_choose_then_discard_or_exile(&refs).map(|rendered| (rendered, 2))
}

/// Rejoin an optional, causative hand reveal with the exact successful
/// reveal-result choice in the following `if you do` sentence.
///
/// Lowering keeps the optional reveal under `WithId(May(..))` so the branch
/// can test whether it happened. The ordinary hand-reveal sequence renderer
/// still proves that the choice is constrained to the revealed collection and
/// that the final action consumes that singular choice.
fn describe_cross_segment_may_reveal_hand_choose_action_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [producer_segment, branch_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !producer_segment.self_replacements.is_empty()
        || !branch_segment.self_replacements.is_empty()
    {
        return None;
    }

    let [producer_effect] = producer_segment.default_effects.as_slice() else {
        return None;
    };
    let with_id = producer_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may.fallback != crate::decision::FallbackStrategy::Decline
        || may
            .decider
            .as_ref()
            .is_some_and(|decider| decider != &PlayerFilter::You)
    {
        return None;
    }
    let [target_effect, look_effect] = may.effects.as_slice() else {
        return None;
    };
    let target = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let look = look_effect.downcast_ref::<crate::effects::LookAtHandEffect>()?;
    if !look.reveal
        || target.target != look.target
        || target
            .chooser
            .as_ref()
            .is_some_and(|chooser| chooser != &PlayerFilter::You)
    {
        return None;
    }

    let [branch_effect] = branch_segment.default_effects.as_slice() else {
        return None;
    };
    let branch = branch_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if branch.condition != with_id.id
        || branch.predicate != EffectPredicate::Happened
        || !branch.else_.is_empty()
    {
        return None;
    }
    let [choose_effect, action_effect] = branch.then.as_slice() else {
        return None;
    };
    let compact = describe_look_hand_choose_then_discard_or_exile(&[
        look_effect,
        choose_effect,
        action_effect,
    ])?;
    let (_, successful_branch) = compact.split_once(". You choose ")?;
    let setup = describe_effect_list(&producer_segment.default_effects);
    let setup = setup.trim().trim_end_matches('.');
    (!setup.is_empty()).then(|| (format!("{setup}. If you do, choose {successful_branch}"), 2))
}

/// Preserve the coordinated reveal/move surface when the following life-loss
/// sentence consumes the revealed card's mana value.
fn describe_cross_segment_reveal_to_hand_lose_mana_value_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    for consumed in 2..=3 {
        let Some(window) = segments.get(start..start + consumed) else {
            break;
        };
        let flattened = flattened_cross_segment_effects(window)?;
        if let Some(rendered) = describe_reveal_top_to_hand_then_lose_mana_value_effects(&flattened)
        {
            return Some((rendered, consumed));
        }
    }
    None
}

/// Rejoin a hand reveal, its tag-linked choice/discard, and a trailing optional
/// move from exile when source sentence preservation split the authored
/// sequence into adjacent resolution segments.
fn describe_cross_segment_reveal_hand_choose_discard_adventure_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    for consumed in 2..=5 {
        let Some(window) = segments.get(start..start + consumed) else {
            break;
        };
        let flattened = flattened_cross_segment_effects(window)?;
        let refs = flattened.iter().collect::<Vec<_>>();
        if let Some(rendered) = describe_reveal_hand_choose_discard_then_adventure_move(&refs) {
            return Some((rendered, consumed));
        }
    }
    None
}

/// The while-exiled cast permission and its free-cast modifier are separate
/// runtime effects, but jointly represent one authored permission sentence.
fn describe_cross_segment_exile_then_free_cast_while_exiled_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let window = segments.get(start..start + 2)?;
    let flattened = flattened_cross_segment_effects(window)?;
    describe_exile_then_free_cast_while_exiled_structural(&flattened).map(|rendered| (rendered, 2))
}

fn describe_cross_segment_observation_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let mut effects = Vec::new();
    for segment in segments[start..].iter().take(5) {
        if !segment.self_replacements.is_empty() {
            break;
        }
        let [effect] = segment.default_effects.as_slice() else {
            break;
        };
        effects.push(effect);
    }
    if effects.len() < 2 {
        return None;
    }
    let (rendered, consumed) = describe_immediate_observation_conditionals(&effects)?;
    (consumed >= 2 && consumed <= effects.len()).then_some((rendered, consumed))
}

fn describe_cross_segment_wrapped_search_two_split_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [partition_segment, suffix_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !partition_segment.self_replacements.is_empty()
        || !suffix_segment.self_replacements.is_empty()
    {
        return None;
    }

    let [sequence_effect] = partition_segment.default_effects.as_slice() else {
        return None;
    };
    let sequence = sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    let suffix_effects = match suffix_segment.default_effects.as_slice() {
        [suffix_effect]
            if suffix_effect
                .downcast_ref::<crate::effects::SequenceEffect>()
                .is_some_and(|suffix| {
                    suffix.surface == ironsmith_core::SequenceSurface::CommaThen
                }) =>
        {
            &suffix_effect
                .downcast_ref::<crate::effects::SequenceEffect>()?
                .effects
        }
        effects => effects,
    };
    if sequence.surface != ironsmith_core::SequenceSurface::Coordinated
        || sequence.effects.len() != 5
        || !matches!(suffix_effects.len(), 1 | 2)
    {
        return None;
    }

    // Lowering keeps the coordinated search/reveal/split sentence inside one
    // sequence, then starts a new resolution segment for shuffle and scry;
    // that suffix may itself retain its authored comma-then sequence.
    // Rejoin only that exact boundary; the established matcher still proves
    // every tag, count, destination, player, and optional-search invariant.
    let mut flattened = Vec::with_capacity(sequence.effects.len() + suffix_effects.len());
    flattened.extend(sequence.effects.iter());
    flattened.extend(suffix_effects.iter());
    let rendered = describe_search_two_split_hand_graveyard_sequence(&flattened)?;
    Some((rendered, 2))
}

fn describe_cross_segment_search_move_shuffle_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    if let Some([search_segment, followup_segment]) = segments.get(start..start + 2)
        && search_segment.self_replacements.is_empty()
        && followup_segment.self_replacements.is_empty()
        && let Some(choose) = cross_segment_search_choose(search_segment)
        && let [reveal_effect, move_effect, shuffle_effect] =
            followup_segment.default_effects.as_slice()
        && let Some(reveal) = reveal_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()
        && let Some(move_to_zone) = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()
        && let Some(shuffle) = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()
        && let Some(rendered) =
            describe_search_choose_then_move(choose, Some(reveal), move_to_zone, Some(shuffle))
    {
        return Some((rendered, 2));
    }

    let [search_segment, move_segment, shuffle_segment] = segments.get(start..start + 3)? else {
        return None;
    };
    if [search_segment, move_segment, shuffle_segment]
        .iter()
        .any(|segment| !segment.self_replacements.is_empty())
    {
        return None;
    }

    let choose = cross_segment_search_choose(search_segment)?;

    let [move_effect] = move_segment.default_effects.as_slice() else {
        return None;
    };
    let tagged_move = move_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let move_to_zone = tagged_move
        .effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Battlefield
        || !matches!(
            move_to_zone.target.base(),
            ChooseSpec::Tagged(tag) if tag.as_str() == choose.tag.as_str()
        )
    {
        return None;
    }

    let [shuffle_effect] = shuffle_segment.default_effects.as_slice() else {
        return None;
    };
    // Sentence lowering may preserve the authored `Then` boundary by wrapping
    // its sole executable effect in a one-item sequence. That wrapper carries
    // no extra semantics, so accept either representation here.
    let shuffle_effect = shuffle_effect
        .downcast_ref::<crate::effects::SequenceEffect>()
        .and_then(|sequence| match sequence.effects.as_slice() {
            [shuffle] => Some(shuffle),
            _ => None,
        })
        .unwrap_or(shuffle_effect);
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    let rendered = describe_search_choose_then_move(choose, None, move_to_zone, Some(shuffle))?;
    let rendered = if choose.count.max == Some(1) {
        rendered
            .replacen(", put it ", ". Put that card ", 1)
            .replacen(", then ", ". Then ", 1)
    } else {
        rendered
            .replacen(", put ", ". Put ", 1)
            .replacen(", then ", ". Then ", 1)
    };
    Some((rendered, 3))
}

fn cross_segment_search_choose(
    search_segment: &crate::resolution::ResolutionSegment,
) -> Option<&crate::effects::ChooseObjectsEffect> {
    match search_segment.default_effects.as_slice() {
        [search_effect] => {
            Some(search_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?)
        }
        [target_effect, search_effect] => {
            let target_only = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
            let choose = search_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
            let ChooseSpec::Player(target_player) = target_only.target.base() else {
                return None;
            };
            let search_owner = choose.filter.owner.as_ref().unwrap_or(&choose.chooser);
            if !same_search_player_filter(target_player, search_owner) {
                return None;
            }
            Some(choose)
        }
        _ => return None,
    }
}

fn count_damage_controller_references_to_tag(effect: &Effect, tag: &TagKey) -> usize {
    let mut count = effect
        .downcast_ref::<crate::effects::DealDamageEffect>()
        .is_some_and(|damage| {
            matches!(
                damage.target.base(),
                ChooseSpec::Player(PlayerFilter::ControllerOf(
                    crate::target::ObjectRef::Tagged(reference),
                )) if reference == tag
            )
        }) as usize;
    effect.visit_child_effects(&mut |child| {
        count += count_damage_controller_references_to_tag(child, tag);
    });
    count
}

fn tagged_object_filter(effect: &Effect) -> Option<(&TagKey, &ObjectFilter)> {
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let filter = if let Some(destroy) = tagged
        .effect
        .downcast_ref::<crate::effects::DestroyEffect>()
    {
        exact_single_target_object_filter(&destroy.spec)?
    } else if let Some(target_only) = tagged
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()
    {
        exact_single_target_object_filter(&target_only.target)?
    } else if let Some(counter) = tagged
        .effect
        .downcast_ref::<crate::effects::CounterEffect>()
    {
        exact_single_target_object_filter(&counter.target)?
    } else if let Some(unless_pays) = tagged
        .effect
        .downcast_ref::<crate::effects::UnlessPaysEffect>()
    {
        let [counter_effect] = unless_pays.effects.as_slice() else {
            return None;
        };
        let counter = counter_effect.downcast_ref::<crate::effects::CounterEffect>()?;
        exact_single_target_object_filter(&counter.target)?
    } else {
        return None;
    };
    Some((&tagged.tag, filter))
}

fn collect_tagged_object_filters(effect: &Effect, filters: &mut Vec<(TagKey, ObjectFilter)>) {
    if let Some((tag, filter)) = tagged_object_filter(effect) {
        filters.push((tag.clone(), filter.clone()));
    }
    effect.visit_child_effects(&mut |child| {
        collect_tagged_object_filters(child, filters);
    });
}

fn effect_tree_has_tagged_destroy(effect: &Effect, tag: &TagKey) -> bool {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>()
        && tagged.tag == *tag
        && tagged
            .effect
            .downcast_ref::<crate::effects::DestroyEffect>()
            .is_some()
    {
        return true;
    }
    let mut found = false;
    effect.visit_child_effects(&mut |child| {
        if !found && effect_tree_has_tagged_destroy(child, tag) {
            found = true;
        }
    });
    found
}

fn describe_cross_segment_tagged_tap_untap_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [tap_segment, restriction_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !tap_segment.self_replacements.is_empty()
        || !restriction_segment.self_replacements.is_empty()
    {
        return None;
    }
    let [tagged_effect] = tap_segment.default_effects.as_slice() else {
        return None;
    };
    let tagged = tagged_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let tap = tagged.effect.downcast_ref::<crate::effects::TapEffect>()?;
    let ChooseSpec::Object(tapped_filter) = tap.target.base() else {
        return None;
    };
    if tap.target.count().is_single() {
        return None;
    }

    let [restriction_effect] = restriction_segment.default_effects.as_slice() else {
        return None;
    };
    let cant = restriction_effect.downcast_ref::<crate::effects::CantEffect>()?;
    let crate::effect::Restriction::Untap(restricted_filter) = &cant.restriction else {
        return None;
    };
    let [constraint] = restricted_filter.tagged_constraints.as_slice() else {
        return None;
    };
    if constraint.tag != tagged.tag
        || constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
    {
        return None;
    }
    let mut untagged_filter = restricted_filter.clone();
    untagged_filter.tagged_constraints.clear();
    let mut selected_filter = tapped_filter.clone();
    if untagged_filter.controller.is_none() {
        selected_filter.controller = None;
    }
    if untagged_filter.owner.is_none() {
        selected_filter.owner = None;
    }
    if untagged_filter != selected_filter {
        return None;
    }

    let tapped_text = describe_effect(tagged_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    let mut referent_filter = tapped_filter.clone();
    referent_filter.controller = None;
    referent_filter.owner = None;
    let subject = pluralize_noun_phrase(
        referent_filter
            .description()
            .trim_start_matches("a ")
            .trim_start_matches("an "),
    );
    let restriction_text = match cant.duration {
        Until::ControllersNextUntapStep => {
            format!("Those {subject} don't untap during their controller's next untap step")
        }
        Until::YouStopControllingThis => format!(
            "Those {subject} don't untap during their controllers' untap steps {}",
            describe_until(&cant.duration)
        ),
        _ => return None,
    };
    Some((format!("{tapped_text}. {restriction_text}"), 2))
}

fn describe_cross_segment_conditional_tagged_tap_untap_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [conditional_segment, restriction_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !conditional_segment.self_replacements.is_empty()
        || !restriction_segment.self_replacements.is_empty()
    {
        return None;
    }
    let [conditional_effect] = conditional_segment.default_effects.as_slice() else {
        return None;
    };
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let [tag_effect, tap_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    if !conditional.if_false.is_empty()
        || conditional.surface != ironsmith_core::ConditionalSurface::LeadingIf
    {
        return None;
    }
    let tag = tag_effect.downcast_ref::<crate::effects::TagMatchingObjectsEffect>()?;
    let tap = tap_effect.downcast_ref::<crate::effects::TapEffect>()?;
    let ChooseSpec::All(tapped_filter) = tap.target.base() else {
        return None;
    };
    if tag.filter != *tapped_filter || !tag.additional_zones.is_empty() || tag.zone.is_some() {
        return None;
    }

    let [restriction_effect] = restriction_segment.default_effects.as_slice() else {
        return None;
    };
    let cant = restriction_effect.downcast_ref::<crate::effects::CantEffect>()?;
    if cant.duration != Until::ControllersNextUntapStep {
        return None;
    }
    let crate::effect::Restriction::Untap(restricted_filter) = &cant.restriction else {
        return None;
    };
    let [constraint] = restricted_filter.tagged_constraints.as_slice() else {
        return None;
    };
    if constraint.tag != tag.tag
        || constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
    {
        return None;
    }
    let mut untagged_filter = restricted_filter.clone();
    untagged_filter.tagged_constraints.clear();
    let mut selected_filter = tapped_filter.clone();
    if !untagged_filter.attacking {
        selected_filter.attacking = false;
    }
    if untagged_filter != selected_filter {
        return None;
    }

    let condition_text = describe_condition(&conditional.condition);
    let label = matches!(
        &conditional.condition,
        Condition::ValueComparison {
            left: Value::LifeTotal(PlayerFilter::You),
            operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
            right: Value::Fixed(5),
        }
    )
    .then_some("Fateful hour — ")
    .unwrap_or_default();
    let tap_text = lowercase_first(describe_effect(tap_effect).trim().trim_end_matches('.'));
    let subject = pluralize_noun_phrase(
        untagged_filter
            .description()
            .trim_start_matches("a ")
            .trim_start_matches("an "),
    );
    Some((
        format!(
            "{label}If {condition_text}, {tap_text}. Those {subject} don't untap during their controller's next untap step"
        ),
        2,
    ))
}

fn describe_cross_segment_typed_controller_damage_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [producer_segment, damage_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !producer_segment.self_replacements.is_empty()
        || !damage_segment.self_replacements.is_empty()
    {
        return None;
    }
    let [damage_effect] = damage_segment.default_effects.as_slice() else {
        return None;
    };
    let mut tagged_filters = Vec::new();
    for effect in &producer_segment.default_effects {
        collect_tagged_object_filters(effect, &mut tagged_filters);
    }
    let candidates = tagged_filters
        .iter()
        .filter(|(tag, _)| count_damage_controller_references_to_tag(damage_effect, tag) == 1)
        .collect::<Vec<_>>();
    let [(tag, filter)] = candidates.as_slice() else {
        return None;
    };
    let noun = if filter.stack_kind == Some(crate::filter::StackObjectKind::Spell) {
        "spell"
    } else if !filter.subtypes.is_empty()
        && filter.subtypes.iter().all(crate::Subtype::is_land_subtype)
    {
        "land"
    } else {
        match filter.card_types.as_slice() {
            [CardType::Land] => "land",
            [CardType::Artifact] => "artifact",
            [CardType::Creature] => "creature",
            [CardType::Enchantment] => "enchantment",
            [CardType::Planeswalker] => "planeswalker",
            [CardType::Battle] => "battle",
            _ => return None,
        }
    };
    if count_damage_controller_references_to_tag(damage_effect, tag) != 1 {
        return None;
    }
    let producer_text = describe_effect_list(&producer_segment.default_effects)
        .trim()
        .trim_end_matches('.')
        .to_string();
    let mut damage_text = describe_effect(damage_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    if damage_text.matches("that object's controller").count() != 1 {
        return None;
    }
    damage_text = damage_text.replace(
        "that object's controller",
        &format!("that {noun}'s controller"),
    );
    if producer_segment
        .default_effects
        .iter()
        .any(|effect| effect_tree_has_tagged_destroy(effect, tag))
    {
        damage_text = damage_text.replacen(
            "If a permanent was destroyed this way, ",
            &format!("If that {noun} is put into a graveyard this way, "),
            1,
        );
    }
    if noun == "land" {
        damage_text = damage_text.replace(
            "If it was a nonbasic permanent, ",
            "If that land was nonbasic, ",
        );
    } else if noun == "creature" {
        damage_text =
            damage_text.replace("If it was attacking, ", "If that creature was attacking, ");
        if damage_effect
            .downcast_ref::<crate::effects::ConditionalEffect>()
            .is_some_and(|conditional| {
                matches!(
                    &conditional.condition,
                    Condition::TaggedObjectMatches(condition_tag, condition_filter)
                        if condition_tag == tag && condition_filter.attacking
                )
            })
        {
            damage_text = damage_text.replace(
                "If that creature was attacking, ",
                "If that creature is attacking, ",
            );
        }
    }
    Some((
        format!("{producer_text}. {}", capitalize_first(&damage_text)),
        2,
    ))
}

fn describe_cross_segment_chain_copy_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [antecedent_segment, enabling_segment, conditional_segment] =
        segments.get(start..start + 3)?
    else {
        return None;
    };
    if [antecedent_segment, enabling_segment, conditional_segment]
        .iter()
        .any(|segment| !segment.self_replacements.is_empty())
        || antecedent_segment.default_effects.is_empty()
    {
        return None;
    }

    let [enabling_effect] = enabling_segment.default_effects.as_slice() else {
        return None;
    };
    let enabling_with_id = enabling_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    enabling_with_id
        .effect
        .downcast_ref::<crate::effects::MayEffect>()?;

    let [conditional_effect] = conditional_segment.default_effects.as_slice() else {
        return None;
    };
    conditional_effect.downcast_ref::<crate::effects::IfEffect>()?;

    let mut combined = antecedent_segment.default_effects.clone();
    combined.push(enabling_effect.clone());
    combined.push(conditional_effect.clone());
    let rendered = describe_chain_copy_effect_list(&combined)?;
    Some((rendered, 3))
}

fn describe_cross_segment_chooser_target_exchange_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [target_segment, exchange_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !target_segment.self_replacements.is_empty()
        || !exchange_segment.self_replacements.is_empty()
    {
        return None;
    }
    let effects = target_segment
        .default_effects
        .iter()
        .chain(&exchange_segment.default_effects)
        .collect::<Vec<_>>();
    describe_target_only_then_exchange_control(&effects).map(|rendered| (rendered, 2))
}

fn describe_cross_segment_created_token_followup_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [create_segment, followup_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !create_segment.self_replacements.is_empty()
        || !followup_segment.self_replacements.is_empty()
    {
        return None;
    }
    let [create_effect] = create_segment.default_effects.as_slice() else {
        return None;
    };
    let (created_tag, create) = tagged_create_token_effect_for_keyword(create_effect)?;
    if matches!(create.count.unhinted(), Value::Fixed(1)) {
        return None;
    }
    let [followup_effect] = followup_segment.default_effects.as_slice() else {
        return None;
    };

    let followup = if let Some(exile) =
        followup_effect.downcast_ref::<crate::effects::ExileTaggedWhenSourceLeavesEffect>()
        && &exile.tag == created_tag
    {
        "Exile those tokens when this permanent leaves the battlefield".to_string()
    } else if let Some(schedule) =
        followup_effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
        && schedule.one_shot
        && schedule.start_next_turn
        && !schedule.until_end_of_turn
        && schedule.controller == PlayerFilter::You
        && schedule
            .trigger
            .downcast_ref::<crate::triggers::BeginningOfUpkeepTrigger>()
            .is_some()
    {
        let [delayed_effect] = schedule.effects.flattened_default_effects() else {
            return None;
        };
        let sacrifice = delayed_effect.downcast_ref::<crate::effects::SacrificeTargetEffect>()?;
        if !matches!(&sacrifice.target, ChooseSpec::Tagged(tag) if tag == created_tag) {
            return None;
        }
        "Sacrifice those tokens at the beginning of your next upkeep".to_string()
    } else {
        return None;
    };

    let creation = describe_effect(create_effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    Some((format!("{creation}. {followup}"), 2))
}

fn describe_cross_segment_created_token_copy_retarget_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [create_copy_segment, retarget_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !create_copy_segment.self_replacements.is_empty()
        || !retarget_segment.self_replacements.is_empty()
    {
        return None;
    }

    let sequence_effect = match create_copy_segment.default_effects.as_slice() {
        [sequence_effect] => sequence_effect,
        [trigger_tag, sequence_effect]
            if trigger_tag
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_some() =>
        {
            sequence_effect
        }
        _ => return None,
    };
    let sequence = structural_unwrap_render_wrappers(sequence_effect)
        .downcast_ref::<crate::effects::SequenceEffect>()?;
    if sequence.surface != ironsmith_core::SequenceSurface::CommaThen {
        return None;
    }
    let [create_effect, copy_effect] = sequence.effects.as_slice() else {
        return None;
    };
    let [retarget_effect] = retarget_segment.default_effects.as_slice() else {
        return None;
    };

    super::render_effects::describe_create_token_then_copy_retarget_to_created_token(&[
        create_effect,
        copy_effect,
        retarget_effect,
    ])
    .map(|rendered| (rendered, 2))
}

fn describe_cross_segment_shuffle_exile_top_cast_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [producer_segment, permission_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !producer_segment.self_replacements.is_empty()
        || !permission_segment.self_replacements.is_empty()
    {
        return None;
    }
    let [shuffle_effect, exile_effect] = producer_segment.default_effects.as_slice() else {
        return None;
    };
    let [permission_effect] = permission_segment.default_effects.as_slice() else {
        return None;
    };
    render_shuffle_exile_top_then_cast_any_number_with_mana_value_cap(&[
        shuffle_effect,
        exile_effect,
        permission_effect,
    ])
    .map(|rendered| (rendered, 2))
}

/// Rejoin a coordinated producer sentence with the following permission
/// sentence when lowering placed them in adjacent resolution segments. The
/// shared exile tag proves the permission consumes the cards produced by the
/// first segment, so the typed effect-list renderer can recover the authored
/// duration surface and sentence boundary.
fn describe_cross_segment_linked_exile_top_play_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    fn flattened_segment_effects(
        segment: &crate::resolution::ResolutionSegment,
    ) -> Option<Vec<Effect>> {
        if !segment.self_replacements.is_empty() || segment.default_effects.is_empty() {
            return None;
        }
        if let [effect] = segment.default_effects.as_slice()
            && let Some(sequence) = structural_unwrap_render_wrappers(effect)
                .downcast_ref::<crate::effects::SequenceEffect>()
        {
            return Some(sequence.effects.clone());
        }
        Some(segment.default_effects.clone())
    }

    let mut effects = flattened_segment_effects(segments.get(start)?)?;
    let mut linked_tags = effects
        .iter()
        .filter_map(|effect| {
            structural_unwrap_render_wrappers(effect)
                .downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()
        })
        .flat_map(|exile| exile.moved_tags.iter().cloned())
        .collect::<Vec<_>>();
    let mut consumed_following_producer_segment = false;
    if linked_tags.is_empty() {
        // A preceding cost-reduction sentence can define the same authored X
        // used by an exile-top producer in the next source sentence. Start
        // the linked window at that typed reduction so the effect-list
        // renderer can emit the shared `where X is ...` basis only once.
        let [reduction_effect] = effects.as_slice() else {
            return None;
        };
        let reduction = structural_unwrap_render_wrappers(reduction_effect)
            .downcast_ref::<crate::effects::GrantNextSpellCostReductionEffect>()?;
        let reduction_value = reduction.generic_reduction.as_ref()?;
        let producer_effects = flattened_segment_effects(segments.get(start + 1)?)?;
        linked_tags = producer_effects
            .iter()
            .filter_map(|effect| {
                structural_unwrap_render_wrappers(effect)
                    .downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()
            })
            .filter(|exile| &exile.count == reduction_value)
            .flat_map(|exile| exile.moved_tags.iter().cloned())
            .collect();
        if linked_tags.is_empty() {
            return None;
        }
        effects.extend(producer_effects);
        consumed_following_producer_segment = true;
    }

    let next_segment = if consumed_following_producer_segment {
        start + 2
    } else {
        start + 1
    };
    for end in next_segment..(start + 5).min(segments.len()) {
        let segment_effects = flattened_segment_effects(&segments[end])?;
        let has_linked_grant = segment_effects.iter().any(|effect| {
            structural_unwrap_render_wrappers(effect)
                .downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
                .is_some_and(|grant| linked_tags.contains(&grant.tag))
        });
        effects.extend(segment_effects);
        if has_linked_grant {
            return describe_linked_exile_top_play_clause(&effects)
                .map(|rendered| (rendered, end - start + 1));
        }
    }
    None
}

fn describe_cross_segment_exile_top_put_from_among_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [producer_segment, disposition_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !producer_segment.self_replacements.is_empty()
        || !disposition_segment.self_replacements.is_empty()
    {
        return None;
    }
    let [exile_effect] = producer_segment.default_effects.as_slice() else {
        return None;
    };
    let [selection_effect, put_effect] = disposition_segment.default_effects.as_slice() else {
        return None;
    };
    render_exile_top_then_put_from_among_onto_battlefield(&[
        exile_effect,
        selection_effect,
        put_effect,
    ])
    .map(|rendered| (rendered, 2))
}

fn describe_cross_segment_exile_top_choose_play_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [selection_segment, permission_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !selection_segment.self_replacements.is_empty()
        || !permission_segment.self_replacements.is_empty()
    {
        return None;
    }
    let [exile_effect, choose_effect] = selection_segment.default_effects.as_slice() else {
        return None;
    };
    let [grant_effect] = permission_segment.default_effects.as_slice() else {
        return None;
    };
    let exile_top = exile_effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let grant_play = grant_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
    describe_exile_top_choose_one_then_play(exile_top, choose, grant_play)
        .map(|rendered| (rendered, 2))
}

fn describe_cross_segment_shuffle_exile_top_free_play_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [producer_segment, permission_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !producer_segment.self_replacements.is_empty()
        || !permission_segment.self_replacements.is_empty()
    {
        return None;
    }
    let [shuffle_effect, exile_effect] = producer_segment.default_effects.as_slice() else {
        return None;
    };
    let [grant_play_effect, grant_free_cast_effect] = permission_segment.default_effects.as_slice()
    else {
        return None;
    };
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if shuffle.player != PlayerFilter::You || shuffle.target_spec.is_some() {
        return None;
    }
    let exile_top = exile_effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    let grant_play = grant_play_effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
    let grant_free_cast = grant_free_cast_effect
        .downcast_ref::<crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect>(
    )?;
    let tail =
        describe_exile_top_then_play_without_paying_mana(exile_top, grant_play, grant_free_cast)?;
    let tail = tail.replacen(" of your library.", ".", 1);
    Some((
        format!("Shuffle your library, then {}", lowercase_first(&tail)),
        2,
    ))
}

fn describe_cross_segment_shuffle_reveal_top_free_play_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let max_consumed = segments.len().saturating_sub(start).min(5);
    for consumed in 1..=max_consumed {
        let window = segments.get(start..start + consumed)?;
        let effects = flattened_cross_segment_effects(window)?;
        if effects.len() > 5 {
            return None;
        }
        if effects.len() < 5 {
            continue;
        }

        let [
            shuffle_effect,
            reveal_effect,
            reveal_permission_effect,
            grant_play_effect,
            grant_free_effect,
        ] = effects.as_slice()
        else {
            return None;
        };
        let shuffle = structural_unwrap_render_wrappers(shuffle_effect)
            .downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
        let reveal_top = structural_unwrap_render_wrappers(reveal_effect)
            .downcast_ref::<crate::effects::RevealTopEffect>()?;
        let reveal_permission = structural_unwrap_render_wrappers(reveal_permission_effect)
            .downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
        let grant_play = structural_unwrap_render_wrappers(grant_play_effect)
            .downcast_ref::<crate::effects::GrantPlayTaggedEffect>()?;
        let grant_free_cast = structural_unwrap_render_wrappers(grant_free_effect)
            .downcast_ref::<crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect>(
        )?;
        let rendered = describe_shuffle_then_reveal_top_then_temporarily_play_revealed_top_card(
            shuffle,
            reveal_top,
            reveal_permission,
            grant_play,
            grant_free_cast,
        )?;
        return Some((rendered, consumed));
    }
    None
}

/// Rejoin an authored choice sentence with its adjacent tagged play/free-cast
/// permission sentence. The effect-list matcher proves the exact shared tag,
/// exile/owner/counter filter, duration, and payment semantics; this window
/// only exposes those typed effects across the resolution-segment boundary.
fn describe_cross_segment_choose_exiled_card_free_play_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [choice_segment, permission_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !choice_segment.self_replacements.is_empty()
        || !permission_segment.self_replacements.is_empty()
    {
        return None;
    }
    let [choose_effect] = choice_segment.default_effects.as_slice() else {
        return None;
    };
    let [grant_play_effect, grant_free_effect] = permission_segment.default_effects.as_slice()
    else {
        return None;
    };
    let effects = vec![
        choose_effect.clone(),
        grant_play_effect.clone(),
        grant_free_effect.clone(),
    ];
    describe_choose_exiled_card_then_play_without_paying(&effects).map(|rendered| (rendered, 2))
}

/// Preserve the typed producer target across an authored sentence boundary.
/// The effect-list matcher still proves the implicit tag edge, matching
/// counter type, duration, and single granted ability; this window only makes
/// the two adjacent default-only segments visible to that matcher together.
fn describe_cross_segment_counter_linked_grant_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [put_segment, grant_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !put_segment.self_replacements.is_empty() || !grant_segment.self_replacements.is_empty() {
        return None;
    }
    let [put_effect] = put_segment.default_effects.as_slice() else {
        return None;
    };
    let [grant_effect] = grant_segment.default_effects.as_slice() else {
        return None;
    };
    describe_counter_linked_grant_after_put(put_effect, grant_effect).map(|rendered| (rendered, 2))
}

/// Rejoin a tagged target modification with a conditional that inspects the
/// same affected object when authored sentence boundaries split the pair.
fn describe_cross_segment_tagged_continuous_then_counter_conditional_draw_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [leading_segment, conditional_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !leading_segment.self_replacements.is_empty()
        || !conditional_segment.self_replacements.is_empty()
    {
        return None;
    }
    let [leading_effect] = leading_segment.default_effects.as_slice() else {
        return None;
    };
    let [conditional_effect] = conditional_segment.default_effects.as_slice() else {
        return None;
    };
    let effects = [leading_effect, conditional_effect];
    super::render_effects::describe_tagged_continuous_then_counter_conditional_draw(&effects)
        .map(|rendered| (rendered, 2))
}

/// Rejoin a tagged temporary pump with the adjacent conditional keyword grant
/// that inspects the same affected object.
fn describe_cross_segment_tagged_pump_then_conditional_keyword_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [pump_segment, conditional_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !pump_segment.self_replacements.is_empty()
        || !conditional_segment.self_replacements.is_empty()
    {
        return None;
    }
    let [pump_effect] = pump_segment.default_effects.as_slice() else {
        return None;
    };
    let [conditional_effect] = conditional_segment.default_effects.as_slice() else {
        return None;
    };
    let effects = [pump_effect, conditional_effect];
    super::render_effects::describe_tagged_pump_then_conditional_keyword(&effects)
        .map(|rendered| (rendered, 2))
}

/// Rejoin a counted draw with the adjacent grant that consumes the exact same
/// filter. Sentence lowering keeps these effects in separate segments, while
/// the typed effect-list matcher proves both set identity and cardinality.
fn describe_cross_segment_draw_count_grant_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [draw_segment, grant_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !draw_segment.self_replacements.is_empty() || !grant_segment.self_replacements.is_empty() {
        return None;
    }
    let [draw_effect] = draw_segment.default_effects.as_slice() else {
        return None;
    };
    let [grant_effect] = grant_segment.default_effects.as_slice() else {
        return None;
    };
    describe_draw_count_then_grant_same_filter(&[draw_effect.clone(), grant_effect.clone()])
        .map(|rendered| (rendered, 2))
}

/// Rejoin a hidden-pile producer with its adjacent manifest/cloak consumer.
/// The effect-list matcher proves the shared accumulating tag, face-down
/// exiles, controller, and final operation; this window only exposes those
/// typed effects across the authored sentence boundary.
fn describe_cross_segment_face_down_pile_manifest_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [pile_segment, manifest_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !pile_segment.self_replacements.is_empty()
        || !manifest_segment.self_replacements.is_empty()
        || pile_segment.default_effects.is_empty()
        || manifest_segment.default_effects.is_empty()
    {
        return None;
    }
    let effects = pile_segment
        .default_effects
        .iter()
        .chain(&manifest_segment.default_effects)
        .cloned()
        .collect::<Vec<_>>();
    describe_face_down_pile_then_manifest(&effects).map(|rendered| (rendered, 2))
}

/// Rejoin quantified choices with later consumers only when an existing
/// structural renderer proves the shared tag edge. Parser-authored sentence
/// boundaries are useful for execution, but they must not hide an aggregate
/// chosen set from renderers that already understand its typed producer and
/// consumers.
fn describe_cross_segment_quantified_choice_collection_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let collect_default_effects = |count: usize| {
        let window = segments.get(start..start + count)?;
        if window.iter().any(|segment| {
            !segment.self_replacements.is_empty() || segment.default_effects.is_empty()
        }) {
            return None;
        }
        Some(
            window
                .iter()
                .flat_map(|segment| segment.default_effects.iter().cloned())
                .collect::<Vec<_>>(),
        )
    };

    if let Some(effects) = collect_default_effects(3)
        && effects
            .first()
            .and_then(|effect| {
                structural_unwrap_render_wrappers(effect)
                    .downcast_ref::<crate::effects::ForPlayersEffect>()
            })
            .is_some()
        && let Some(rendered) = describe_structural_multisentence_effect_list(&effects)
    {
        return Some((rendered, 3));
    }

    let effects = collect_default_effects(2)?;
    if let Some(for_players) = structural_unwrap_render_wrappers(&effects[0])
        .downcast_ref::<crate::effects::ForPlayersEffect>()
        && let Some(destroy) =
            crate::compiled_text::render_effects::destroy_effect_for_choose_compaction(&effects[1])
        && let Some(rendered) =
            crate::compiled_text::render_effects::describe_for_players_may_choose_then_destroy_chosen(
                for_players,
                destroy,
            )
    {
        return Some((rendered, 2));
    }
    if let Some(rendered) = describe_direct_then_players_choose_destroy_complement(&effects) {
        return Some((rendered, 2));
    }
    if let Some(rendered) = describe_distinct_power_choice_destroy_complement(&effects) {
        return Some((rendered, 2));
    }
    // A source sentence beginning with "Then" is retained as a one-effect
    // SequenceEffect. The correlated choice/complement renderer already owns
    // that sentence boundary, so expose only that exact singleton wrapper
    // before asking it to prove the shared chosen-set tag.
    let split_choice_effects = effects
        .iter()
        .map(|effect| {
            effect
                .downcast_ref::<crate::effects::SequenceEffect>()
                .filter(|sequence| {
                    sequence.surface == ironsmith_core::SequenceSurface::SentenceLeadingThen
                        && sequence.effects.len() == 1
                })
                .map(|sequence| sequence.effects[0].clone())
                .unwrap_or_else(|| effect.clone())
        })
        .collect::<Vec<_>>();
    describe_split_for_players_choose_then_sacrifice(&split_choice_effects)
        .map(|rendered| (rendered, 2))
}

#[cfg(test)]
mod split_for_players_cross_segment_surface_tests {
    use super::*;

    #[test]
    fn sentence_leading_then_keeps_correlated_type_slot_choices_together() {
        let oracle = "For each player, you choose from among the permanents that player controls an artifact, a creature, an enchantment, and a planeswalker. Then each player sacrifices all other nonland permanents they control.";
        let definition = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Correlated Type Slot Probe",
        )
        .card_types(vec![CardType::Sorcery])
        .parse_text(oracle)
        .expect("typed type-slot choice/complement program should parse");

        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition),
            vec![oracle.to_string()]
        );
    }
}

/// Rejoin a looked-card procedure split across source-sentence resolution
/// segments. The effect-list helper still has to prove the shared looked tag,
/// selected tag, destination, grant target, and exact remainder, and it must
/// consume every flattened effect before this window crosses a boundary.
fn describe_cross_segment_looked_cards_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let remaining = segments.len().saturating_sub(start).min(6);
    for consumed in (2..=remaining).rev() {
        let window = segments.get(start..start + consumed)?;
        if window.iter().any(|segment| {
            !segment.self_replacements.is_empty() || segment.default_effects.is_empty()
        }) {
            continue;
        }
        let effects = window
            .iter()
            .flat_map(|segment| segment.default_effects.iter().cloned())
            .collect::<Vec<_>>();
        if let Some(rendered) = describe_complete_looked_cards_clause(&effects) {
            return Some((rendered, consumed));
        }
    }
    None
}

/// Rejoin an optional looked-card producer with the exact `if you do`
/// singleton top/rest-bottom partition in the following source sentence.
fn describe_cross_segment_may_look_top_rest_bottom_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [producer_segment, branch_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !producer_segment.self_replacements.is_empty()
        || !branch_segment.self_replacements.is_empty()
    {
        return None;
    }
    let with_ids = producer_segment
        .default_effects
        .iter()
        .filter_map(|effect| effect.downcast_ref::<crate::effects::WithIdEffect>())
        .collect::<Vec<_>>();
    let [with_id] = with_ids.as_slice() else {
        return None;
    };
    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may.fallback != crate::decision::FallbackStrategy::Decline {
        return None;
    }
    let [look_effect] = may.effects.as_slice() else {
        return None;
    };
    let look = look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()?;
    if may
        .decider
        .as_ref()
        .is_some_and(|decider| decider != &look.player)
    {
        return None;
    }

    let [branch_effect] = branch_segment.default_effects.as_slice() else {
        return None;
    };
    let branch = branch_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if branch.condition != with_id.id
        || branch.predicate != EffectPredicate::Happened
        || !branch.else_.is_empty()
    {
        return None;
    }
    let [choose_effect, move_effect, remainder_effect] = branch.then.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let remainder = remainder_effect
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    describe_may_look_then_put_one_top_rest_bottom(look, choose, move_effect, remainder)
        .map(|rendered| (rendered, 2))
}

/// Rejoin a declared pair of targets with the following tap-and-unattach
/// sentence when authored sentence boundaries split the shared tagged set.
fn describe_cross_segment_choose_two_tap_unattach_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let window = segments.get(start..start + 2)?;
    let effects = flattened_cross_segment_effects(window)?;
    describe_choose_two_tap_then_unattach_equipment_sequence(&effects).map(|rendered| (rendered, 2))
}

/// Rejoin a bulk battlefield return with the permanent decayed grant applied
/// to that exact returned collection in the next authored sentence.
fn describe_cross_segment_bulk_return_decayed_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let window = segments.get(start..start + 2)?;
    let effects = flattened_cross_segment_effects(window)?;
    describe_bulk_battlefield_move_then_grant_decayed(&effects).map(|rendered| (rendered, 2))
}

/// Rejoin coordinated player choices with the following count of the exact
/// affected collection. The structural matcher proves both player actions,
/// the shared tag, and the tagged graveyard count before crossing the authored
/// sentence boundary.
fn describe_cross_segment_joint_discard_or_sacrifice_then_draw_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [choice_segment, draw_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !choice_segment.self_replacements.is_empty() || !draw_segment.self_replacements.is_empty() {
        return None;
    }
    let [choice_effect] = choice_segment.default_effects.as_slice() else {
        return None;
    };
    let [draw_effect] = draw_segment.default_effects.as_slice() else {
        return None;
    };
    let effects = [choice_effect.clone(), draw_effect.clone()];
    describe_joint_discard_or_sacrifice_then_draw(&effects).map(|rendered| (rendered, 2))
}

/// Rejoin a standalone opponent choice with the following two correlated
/// graveyard choices and returns. The structural bundle matcher validates the
/// chosen-player tag and both return actors before producing Offering-style
/// coordinated text.
fn describe_cross_segment_opponent_choice_returns_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [choice_segment, returns_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !choice_segment.self_replacements.is_empty()
        || !returns_segment.self_replacements.is_empty()
        || !matches!(choice_segment.default_effects.as_slice(), [effect] if effect.downcast_ref::<crate::effects::ChoosePlayerEffect>().is_some())
        || returns_segment.default_effects.len() != 4
    {
        return None;
    }
    let effects =
        flattened_cross_segment_effects(&[choice_segment.clone(), returns_segment.clone()])?;
    describe_structural_multisentence_effect_list(&effects).map(|rendered| (rendered, 2))
}

/// Restore a relationship-aware fight surface when sentence lowering keeps
/// the two targets, the conditional action, and the fight in separate
/// resolution segments. The exact helpers prove every tag edge and require
/// the complete four-effect shape before any boundaries are crossed.
fn describe_cross_segment_conditional_fight_program(
    program: &crate::resolution::ResolutionProgram,
) -> Option<String> {
    if program.segments.len() < 2
        || program
            .segments
            .iter()
            .any(|segment| !segment.self_replacements.is_empty())
    {
        return None;
    }
    let effects = program
        .flattened_default_effects()
        .iter()
        .collect::<Vec<_>>();
    describe_two_distinct_targets_conditional_then_fight(&effects)
        .or_else(|| describe_two_distinct_targets_counter_then_fight(&effects))
        .or_else(|| describe_targeted_conditional_action_then_fight(&effects))
}

/// Rejoin the chosen-creature untap, counters, keyword grant, additional
/// combat, and attack restriction when each authored sentence occupies its
/// own resolution segment.
fn describe_cross_segment_chosen_creatures_blessing_program(
    program: &crate::resolution::ResolutionProgram,
) -> Option<String> {
    if program.segments.len() < 2
        || program
            .segments
            .iter()
            .any(|segment| !segment.self_replacements.is_empty())
    {
        return None;
    }
    let effects = flattened_cross_segment_effects(&program.segments)?;
    describe_chosen_creatures_blessing_additional_combat_clause(&effects)
}

/// Target declarations and optional actions can occupy one or more resolution
/// segments even when Oracle presents them as one multi-sentence procedure.
/// Flatten only the default branches, then require the complete typed
/// target/chooser/consult shape before choosing that authored surface.
fn describe_cross_segment_relative_player_target_consult_program(
    program: &crate::resolution::ResolutionProgram,
) -> Option<String> {
    if program.segments.is_empty()
        || program
            .segments
            .iter()
            .any(|segment| !segment.self_replacements.is_empty())
    {
        return None;
    }
    describe_relative_player_target_then_optional_consult(program.flattened_default_effects())
        .or_else(|| {
            describe_relative_player_target_then_optional_search(
                program.flattened_default_effects(),
            )
        })
}

/// A mixed player/planeswalker target collection is represented by separate
/// typed player and object iterators in the second source-sentence segment.
/// Flatten only replacement-free default branches, then let the structural
/// matcher prove the shared declaration and complete equivalent procedures.
fn describe_cross_segment_mixed_target_consult_program(
    program: &crate::resolution::ResolutionProgram,
) -> Option<String> {
    if program.segments.is_empty()
        || program
            .segments
            .iter()
            .any(|segment| !segment.self_replacements.is_empty())
    {
        return None;
    }
    describe_mixed_target_collection_consult_damage(program.flattened_default_effects())
}

#[cfg(test)]
mod relative_player_target_consult_program_tests {
    use super::*;

    #[test]
    fn rejoins_active_player_target_and_optional_consult_across_segments() {
        const EXPECTED: &str = "That player chooses target player who controls more creatures than they do and is their opponent. The first player may reveal cards from the top of their library until they reveal a creature card. If the first player does, that player puts that card onto the battlefield and all other cards revealed this way into their graveyard";

        let all_tag = TagKey::from("revealed");
        let match_tag = TagKey::from("matched");
        let target = Effect::new(
            crate::effects::TargetOnlyEffect::explicit(ChooseSpec::target(ChooseSpec::Player(
                PlayerFilter::OpponentWithMoreControlledObjectsThan {
                    player: Box::new(PlayerFilter::Active),
                    filter: Box::new(ObjectFilter::creature()),
                },
            )))
            .with_chooser(PlayerFilter::Active),
        );

        let mut creature_card = ObjectFilter::creature();
        creature_card.zone = None;
        let consult = Effect::new(crate::effects::ConsultTopOfLibraryEffect::new(
            PlayerFilter::Active,
            crate::effects::consult_helpers::LibraryConsultMode::Reveal,
            creature_card,
            crate::effects::ConsultTopOfLibraryStopRule::FirstMatch,
            all_tag.clone(),
            match_tag.clone(),
        ));
        let move_match = Effect::new(crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Tagged(match_tag.clone()),
            Zone::Battlefield,
            false,
        ));
        let mut iterated_is_match = ObjectFilter::default();
        iterated_is_match
            .tagged_constraints
            .push(crate::filter::TaggedObjectConstraint {
                tag: match_tag,
                relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
            });
        let move_remainder = Effect::new(crate::effects::ForEachTaggedEffect::new(
            all_tag,
            vec![Effect::conditional(
                Condition::TaggedObjectMatches(TagKey::from("__it__"), iterated_is_match),
                vec![],
                vec![Effect::new(crate::effects::MoveToZoneEffect::new(
                    ChooseSpec::Iterated,
                    Zone::Graveyard,
                    false,
                ))],
            )],
        ));
        let may = Effect::new(crate::effects::MayEffect::new_for_player(
            vec![consult, move_match, move_remainder],
            PlayerFilter::Active,
        ));

        let single_segment =
            crate::resolution::ResolutionProgram::from_effects(vec![target.clone(), may.clone()]);
        assert_eq!(describe_resolution_program(&single_segment), EXPECTED);

        let program = crate::resolution::ResolutionProgram::new(vec![
            crate::resolution::ResolutionSegment::from_effects(vec![target]),
            crate::resolution::ResolutionSegment::from_effects(vec![may]),
        ]);

        assert_eq!(describe_resolution_program(&program), EXPECTED);
    }
}

/// Rejoin the same-name extraction procedure when source sentence boundaries
/// placed its typed effects in adjacent resolution segments. Sequence wrappers
/// only preserve punctuation/coordination here; the shared renderer still has
/// to prove the chosen-name or targeted-object tag, the three-zone search, the
/// matching exile consumer, and the searched player's shuffle (and draw, when
/// present) before this window can succeed.
fn describe_cross_segment_same_name_extraction_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    fn collect_sequence_members(effect: &Effect, flattened: &mut Vec<Effect>) {
        let unwrapped = structural_unwrap_render_wrappers(effect);
        if let Some(sequence) = unwrapped.downcast_ref::<crate::effects::SequenceEffect>() {
            for member in &sequence.effects {
                collect_sequence_members(member, flattened);
            }
        } else if unwrapped
            .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
            .is_some()
            || unwrapped
                .downcast_ref::<crate::effects::TagTriggeringSourceEffect>()
                .is_some()
            || unwrapped
                .downcast_ref::<crate::effects::TagTriggeringBlockersEffect>()
                .is_some()
        {
            // Trigger referent setup has no authored surface. The typed
            // player/object filters on the remaining effects retain the
            // relationship that the extraction matcher must prove.
        } else {
            flattened.push(effect.clone());
        }
    }

    let remaining = segments.len().saturating_sub(start).min(6);
    for consumed in (1..=remaining).rev() {
        let window = segments.get(start..start + consumed)?;
        if window.iter().any(|segment| {
            !segment.self_replacements.is_empty() || segment.default_effects.is_empty()
        }) {
            continue;
        }
        let mut flattened = Vec::new();
        for effect in window
            .iter()
            .flat_map(|segment| segment.default_effects.iter())
        {
            collect_sequence_members(effect, &mut flattened);
        }
        if let Some(rendered) = describe_same_name_three_zone_extraction(&flattened) {
            return Some((rendered, consumed));
        }
    }
    None
}

/// Expose same-name search pipelines across both sentence segments and
/// punctuation-only `SequenceEffect` wrappers. The downstream matchers still
/// require the complete typed producer/tag/search/reveal/move/shuffle graph,
/// so flattening here cannot turn an unrelated sequence into authored prose.
fn describe_cross_segment_same_name_search_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    fn collect_sequence_members<'a>(effect: &'a Effect, flattened: &mut Vec<&'a Effect>) {
        let unwrapped = structural_unwrap_render_wrappers(effect);
        if let Some(sequence) = unwrapped.downcast_ref::<crate::effects::SequenceEffect>() {
            for member in &sequence.effects {
                collect_sequence_members(member, flattened);
            }
        } else {
            flattened.push(effect);
        }
    }

    let remaining = segments.len().saturating_sub(start).min(4);
    for consumed in (1..=remaining).rev() {
        let window = segments.get(start..start + consumed)?;
        if window.iter().any(|segment| {
            !segment.self_replacements.is_empty() || segment.default_effects.is_empty()
        }) {
            continue;
        }
        let mut flattened = Vec::new();
        for effect in window
            .iter()
            .flat_map(|segment| segment.default_effects.iter())
        {
            collect_sequence_members(effect, &mut flattened);
        }
        if let Some(rendered) = describe_single_hand_reveal_same_name_search(&flattened)
            .or_else(|| describe_same_name_reference_search_bundle(&flattened))
            .or_else(|| describe_target_player_search_exile_shuffle_bundle(&flattened))
            .or_else(|| describe_single_hand_reveal_setup(&flattened))
        {
            return Some((rendered, consumed));
        }
    }
    None
}

fn describe_cross_segment_target_player_draw_exile_copy_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [setup_segment, result_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !setup_segment.self_replacements.is_empty() || !result_segment.self_replacements.is_empty() {
        return None;
    }
    let [setup_effect] = setup_segment.default_effects.as_slice() else {
        return None;
    };
    let [result_effect] = result_segment.default_effects.as_slice() else {
        return None;
    };
    let rendered = describe_target_player_draw_exile_then_copy_result(&[
        setup_effect.clone(),
        result_effect.clone(),
    ])?;
    Some((rendered, 2))
}

/// Compact the general "other than basic land cards" graveyard-exile
/// procedure. The parser represents that exception as the De Morgan union
/// `nonland OR nonbasic`, then iterates the exact exiled tag to search and
/// exile same-name cards before the targeted player shuffles.
fn describe_graveyard_exception_same_name_exile_program(
    program: &crate::resolution::ResolutionProgram,
) -> Option<String> {
    let [exile_segment, search_segment, shuffle_segment] = program.segments.as_slice() else {
        return None;
    };
    if [exile_segment, search_segment, shuffle_segment]
        .iter()
        .any(|segment| !segment.self_replacements.is_empty())
    {
        return None;
    }
    let [exile_effect] = exile_segment.default_effects.as_slice() else {
        return None;
    };
    let tagged_exile = exile_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let exile = tagged_exile
        .effect
        .downcast_ref::<crate::effects::ExileEffect>()?;
    let ChooseSpec::All(exile_filter) = &exile.spec else {
        return None;
    };
    let [first_exception, second_exception] = exile_filter.any_of.as_slice() else {
        return None;
    };

    let target_graveyard = |filter: &ObjectFilter| {
        filter.zone == Some(Zone::Graveyard)
            && matches!(
                filter.owner.as_ref(),
                Some(PlayerFilter::Target(player)) if **player == PlayerFilter::Any
            )
    };
    if !target_graveyard(first_exception) || !target_graveyard(second_exception) {
        return None;
    }
    let only_exception =
        |filter: &ObjectFilter, card_type: Option<CardType>, supertype: Option<Supertype>| {
            if card_type.is_some_and(|kind| filter.excluded_card_types.as_slice() != [kind])
                || card_type.is_none() && !filter.excluded_card_types.is_empty()
                || supertype.is_some_and(|kind| filter.excluded_supertypes.as_slice() != [kind])
                || supertype.is_none() && !filter.excluded_supertypes.is_empty()
            {
                return false;
            }
            let mut bare = filter.clone();
            bare.zone = None;
            bare.owner = None;
            bare.excluded_card_types.clear();
            bare.excluded_supertypes.clear();
            bare.set_explicit_card_noun(false);
            bare == ObjectFilter::default()
        };
    let is_nonland = |filter: &ObjectFilter| only_exception(filter, Some(CardType::Land), None);
    let is_nonbasic = |filter: &ObjectFilter| only_exception(filter, None, Some(Supertype::Basic));
    if !((is_nonland(first_exception) && is_nonbasic(second_exception))
        || (is_nonbasic(first_exception) && is_nonland(second_exception)))
    {
        return None;
    }
    let mut exile_base = exile_filter.clone();
    exile_base.any_of.clear();
    if exile_base != ObjectFilter::default() {
        return None;
    }

    let [search_effect] = search_segment.default_effects.as_slice() else {
        return None;
    };
    let for_each = search_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if for_each.tag != tagged_exile.tag {
        return None;
    }
    let [search_sequence_effect] = for_each.effects.as_slice() else {
        return None;
    };
    let search_sequence =
        search_sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    let [choose_effect, move_each_effect] = search_sequence.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !choose.is_search
        || choose.count.min != 0
        || choose.count.max.is_some()
        || choose.filter.zone != Some(Zone::Library)
        || !matches!(
            choose.filter.owner.as_ref(),
            Some(PlayerFilter::AliasedTarget(player)) if **player == PlayerFilter::Any
        )
        || !choose.filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::SameNameAsTagged
        })
    {
        return None;
    }
    let move_each = move_each_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let [move_effect] = move_each.effects.as_slice() else {
        return None;
    };
    let move_to_zone = move_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_each.tag != choose.tag
        || move_to_zone.zone != Zone::Exile
        || !matches!(move_to_zone.target.base(), ChooseSpec::Iterated)
    {
        return None;
    }

    let [shuffle_effect] = shuffle_segment.default_effects.as_slice() else {
        return None;
    };
    let shuffle_effect = shuffle_effect
        .downcast_ref::<crate::effects::SequenceEffect>()
        .and_then(|sequence| match sequence.effects.as_slice() {
            [shuffle] => Some(shuffle),
            _ => None,
        })
        .unwrap_or(shuffle_effect);
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if !matches!(shuffle.player, PlayerFilter::Target(ref player) if **player == PlayerFilter::Any)
    {
        return None;
    }

    Some(
        "Exile all cards from target player's graveyard other than basic land cards. For each card exiled this way, search that player's library for all cards with the same name as that card and exile them. Then that player shuffles"
            .to_string(),
    )
}

fn describe_cross_segment_may_search_conditional_else_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let first = segments.get(start)?;
    if !first.self_replacements.is_empty() {
        return None;
    }
    let (may_effect, conditional_effect, otherwise_segment, consumed) =
        if let [may_effect, conditional_effect] = first.default_effects.as_slice() {
            (may_effect, conditional_effect, segments.get(start + 1)?, 2)
        } else {
            let [may_effect] = first.default_effects.as_slice() else {
                return None;
            };
            let conditional_segment = segments.get(start + 1)?;
            let [conditional_effect] = conditional_segment.default_effects.as_slice() else {
                return None;
            };
            if !conditional_segment.self_replacements.is_empty() {
                return None;
            }
            (may_effect, conditional_effect, segments.get(start + 2)?, 3)
        };
    if !otherwise_segment.self_replacements.is_empty() {
        return None;
    }
    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let with_id = conditional_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let conditional = with_id
        .effect
        .downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() {
        return None;
    }

    let [otherwise_effect] = otherwise_segment.default_effects.as_slice() else {
        return None;
    };
    let otherwise = otherwise_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if otherwise.condition != with_id.id
        || otherwise.predicate != crate::effect::EffectPredicate::DidNotHappen
        || !otherwise.else_.is_empty()
        || otherwise.then.len() != 1
    {
        return None;
    }

    let mut merged = conditional.clone();
    merged.if_false = otherwise.then.clone();
    describe_may_search_reveal_shuffle_then_conditional_move(may, &merged)
        .map(|rendered| (rendered, consumed))
}

/// Rejoin an all-creatures goad result with a restriction on the exact
/// generated `goaded_*` set. Source-sentence lowering keeps these as adjacent
/// executable segments; the result tag proves that "those creatures" is a
/// back-reference rather than an unrelated global blocking restriction.
fn describe_cross_segment_goaded_set_block_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [goad_effect] = segments.get(start)?.default_effects.as_slice() else {
        return None;
    };
    let first = segments.get(start)?;
    let second = segments.get(start + 1)?;
    if !first.self_replacements.is_empty() || !second.self_replacements.is_empty() {
        return None;
    }
    let [cant_effect] = second.default_effects.as_slice() else {
        return None;
    };
    let goad = goad_effect.downcast_ref::<crate::effects::GoadEffect>()?;
    let ChooseSpec::All(goaded_filter) = &goad.target else {
        return None;
    };
    if goad.duration != Until::YourNextTurn
        || !goaded_filter.card_types.contains(&CardType::Creature)
    {
        return None;
    }

    let cant = cant_effect.downcast_ref::<crate::effects::CantEffect>()?;
    let crate::effect::Restriction::Block(block_filter) = &cant.restriction else {
        return None;
    };
    if cant.duration != Until::YourNextTurn
        || !matches!(cant.start, crate::effect::RestrictionStart::Immediate)
        || prior_effect_action_for_filter(block_filter)
            != Some(crate::effect::PriorEffectAction::Goaded)
    {
        return None;
    }
    let mut reference_only = block_filter.clone();
    reference_only.set_prior_effect_action_surface(None);
    reference_only.tagged_constraints.retain(|constraint| {
        constraint.relation != crate::filter::TaggedOpbjectRelation::IsTaggedObject
    });
    if reference_only != ObjectFilter::default() {
        return None;
    }

    let goad_text = describe_effect(goad_effect);
    Some((
        format!(
            "{}. Until your next turn, those creatures can't block",
            goad_text.trim().trim_end_matches('.')
        ),
        2,
    ))
}

/// Rejoin an Aura's attached-object setup, tap action, and Equipment-only
/// unattach rider. The attachment tag proves that the conditional's "it" and
/// both actions refer to the same object across the two source sentences.
fn describe_cross_segment_attached_tap_equipment_unattach_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let first = segments.get(start)?;
    let second = segments.get(start + 1)?;
    if !first.self_replacements.is_empty() || !second.self_replacements.is_empty() {
        return None;
    }

    let [tag_effect, tap_effect] = first.default_effects.as_slice() else {
        return None;
    };
    let tag_attached = tag_effect.downcast_ref::<crate::effects::TagAttachedToSourceEffect>()?;
    if tag_attached.tag.as_str() != "enchanted" {
        return None;
    }
    let tap =
        unwrap_basic_render_wrapper(tap_effect).downcast_ref::<crate::effects::TapEffect>()?;
    if tap.target != ChooseSpec::Tagged(tag_attached.tag.clone()) {
        return None;
    }

    let [conditional_effect] = second.default_effects.as_slice() else {
        return None;
    };
    let conditional = conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let crate::effect::Condition::TaggedObjectMatches(condition_tag, equipment_filter) =
        &conditional.condition
    else {
        return None;
    };
    let mut expected_equipment = ObjectFilter::default();
    expected_equipment.subtypes.push(Subtype::Equipment);
    if condition_tag != &tag_attached.tag
        || equipment_filter != &expected_equipment
        || conditional.surface != ironsmith_core::ConditionalSurface::LeadingIf
        || !conditional.if_false.is_empty()
    {
        return None;
    }

    let [unattach_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let unattach = unwrap_basic_render_wrapper(unattach_effect)
        .downcast_ref::<crate::effects::UnattachObjectsEffect>()?;
    if unattach.objects != ChooseSpec::Tagged(tag_attached.tag.clone()) {
        return None;
    }

    Some((
        "Tap enchanted creature. If it's an Equipment, unattach it".to_string(),
        2,
    ))
}

/// Rejoin an authored target-player declaration with the following
/// choose-half/return bundle. Reference resolution aliases the target inside
/// the second segment, while the explicit target-only effect remains the
/// source of the opening `Choose target player` sentence.
fn describe_cross_segment_target_player_choose_half_return_window(
    segments: &[crate::resolution::ResolutionSegment],
    start: usize,
) -> Option<(String, usize)> {
    let [target_segment, return_segment] = segments.get(start..start + 2)? else {
        return None;
    };
    if !target_segment.self_replacements.is_empty() || !return_segment.self_replacements.is_empty()
    {
        return None;
    }
    let [target_effect] = target_segment.default_effects.as_slice() else {
        return None;
    };
    let target = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if !target.explicit_declaration
        || target.chooser.is_some()
        || target.target != ChooseSpec::target_player()
    {
        return None;
    }

    let [choose_effect, return_effect] = return_segment.default_effects.as_slice() else {
        return None;
    };
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let return_to_hand = structural_unwrap_render_wrappers(return_effect)
        .downcast_ref::<crate::effects::ReturnToHandEffect>()?;
    describe_target_player_choose_half_then_return_to_hand(choose, return_to_hand)
        .map(|rendered| (rendered, 2))
}

fn describe_participant_choice_complement_phase_out_program(
    program: &crate::resolution::ResolutionProgram,
) -> Option<String> {
    let [choice_segment, phase_segment] = program.segments.as_slice() else {
        return None;
    };
    if !choice_segment.self_replacements.is_empty() || !phase_segment.self_replacements.is_empty() {
        return None;
    }
    let [player_effect] = choice_segment.default_effects.as_slice() else {
        return None;
    };
    let for_players = structural_unwrap_render_wrappers(player_effect)
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if for_players.filter != PlayerFilter::Any
        || !for_players.starting_with_controller
        || for_players.stop_after_first_happened
    {
        return None;
    }
    let [choose_effect] = for_players.effects.as_slice() else {
        return None;
    };
    let choose = structural_unwrap_render_wrappers(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let expected_choice = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::permanent_card()
            .in_zone(crate::zone::Zone::Battlefield)
            .controlled_by(PlayerFilter::IteratedPlayer),
        ChoiceCount::up_to(5),
        PlayerFilter::IteratedPlayer,
        choose.tag.clone(),
    );
    let mut semantic_choice = choose.clone();
    if semantic_choice.zone == Some(crate::zone::Zone::Battlefield) {
        // Lowering also records the filter's zone on the choice envelope.
        // That redundant execution hint does not change the selected set.
        semantic_choice.zone = None;
    }
    if semantic_choice != expected_choice {
        return None;
    }

    let [phase_effect] = phase_segment.default_effects.as_slice() else {
        return None;
    };
    let phase_out = structural_unwrap_render_wrappers(phase_effect)
        .downcast_ref::<crate::effects::PhaseOutEffect>()?;
    let ChooseSpec::All(phase_filter) = &phase_out.spec else {
        return None;
    };
    let [constraint] = phase_filter.tagged_constraints.as_slice() else {
        return None;
    };
    if constraint.tag != choose.tag
        || constraint.relation != crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
        || !phase_filter.other
        || phase_filter.prior_effect_action_surface()
            != Some(ironsmith_core::PriorEffectAction::Chosen)
        || !matches!(
            phase_filter.source_surface.as_ref(),
            Some(crate::target::SourceReferenceSurface::ThisPermanentType(surface))
                if surface.eq_ignore_ascii_case("this creature")
        )
        || phase_out.duration != crate::effects::PhaseOutDuration::UntilNextUntap
        || phase_out.source_surface.is_some()
    {
        return None;
    }
    let mut semantic_phase_filter = phase_filter.clone();
    semantic_phase_filter.other = false;
    semantic_phase_filter.source_surface = None;
    semantic_phase_filter.tagged_constraints.clear();
    semantic_phase_filter.set_prior_effect_action_surface(None);
    if semantic_phase_filter
        != ObjectFilter::permanent_card().in_zone(crate::zone::Zone::Battlefield)
    {
        return None;
    }

    Some(
        "Starting with you, each player chooses up to five permanents they control. All permanents other than this creature that weren't chosen this way phase out"
            .to_string(),
    )
}

/// Restore the authored sentence boundary in a target-player life-loss spell
/// whose second sentence coordinates life gain with token creation. The
/// parser keeps that second sentence as a `SequenceEffect`; matching its typed
/// actors and the token's independently rendered ability sentence prevents
/// the generic list renderer from dropping "you" and folding all three
/// sentences into one comma chain.
fn describe_target_player_life_loss_then_gain_and_create_program(
    program: &crate::resolution::ResolutionProgram,
) -> Option<String> {
    let [segment] = program.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let [target_effect, lose_effect, sequence_effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let target = target_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let lose = lose_effect.downcast_ref::<crate::effects::LoseLifeEffect>()?;
    let sequence = sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    if target.explicit_declaration
        || target.chooser.is_some()
        || target.target != ChooseSpec::target_player()
        || lose.player != ChooseSpec::Player(PlayerFilter::target_player())
        || !matches!(
            sequence.surface,
            ironsmith_core::SequenceSurface::Coordinated
        )
    {
        return None;
    }
    let [gain_effect, create_effect] = sequence.effects.as_slice() else {
        return None;
    };
    let gain = gain_effect.downcast_ref::<crate::effects::GainLifeEffect>()?;
    let create = structural_unwrap_render_wrappers(create_effect)
        .downcast_ref::<crate::effects::CreateTokenEffect>()?;
    if gain.player != ChooseSpec::Player(PlayerFilter::You)
        || gain.amount != lose.amount
        || create.controller != PlayerFilter::You
    {
        return None;
    }

    let create_text = describe_effect(create_effect);
    let (create_main, token_ability) = create_text.split_once(". They have ")?;
    let create_main = create_main.strip_prefix("Create ")?;
    let create_main = if let Value::Fixed(count) = create.count {
        let numeric_prefix = format!("{count} ");
        create_main
            .strip_prefix(&numeric_prefix)
            .and_then(|rest| number_word(count).map(|word| format!("{word} {rest}")))
            .unwrap_or_else(|| create_main.to_string())
    } else {
        create_main.to_string()
    };
    Some(format!(
        "Target player loses {}. You gain {} and create {create_main}. They have {token_ability}",
        describe_life_amount_phrase(&lose.amount),
        describe_life_amount_phrase(&gain.amount),
    ))
}

pub(super) fn describe_resolution_program(
    program: &crate::resolution::ResolutionProgram,
) -> String {
    if let [segment] = program.segments.as_slice()
        && segment.self_replacements.is_empty()
    {
        let effects = segment.default_effects.iter().collect::<Vec<_>>();
        if let Some(rendered) = describe_energy_then_pay_any_then_destroy(&effects) {
            return rendered;
        }
    }
    if let Some(rendered) = describe_target_player_life_loss_then_gain_and_create_program(program) {
        return rendered;
    }
    if let Some(rendered) = describe_participant_choice_complement_phase_out_program(program) {
        return rendered;
    }
    if let Some(rendered) = describe_graveyard_exception_same_name_exile_program(program) {
        return rendered;
    }
    if let Some(rendered) = describe_spell_mastery_reanimation_program(program) {
        return rendered;
    }
    if let Some(rendered) = describe_group_pump_then_conditional_untap_program(program) {
        return rendered;
    }
    if let Some(rendered) = describe_repeated_die_parity_result_program(program) {
        return rendered;
    }
    if let Some(rendered) = describe_roll_die_then_draw_equal_result_program(program) {
        return rendered;
    }
    if let Some(rendered) = describe_sequenced_d20_numeric_result_table_program(program) {
        return rendered;
    }
    if let Some(rendered) =
        describe_roll_result_damage_then_random_source_attachment_program(program)
    {
        return rendered;
    }
    if let Some(rendered) = describe_cross_segment_conditional_fight_program(program) {
        return rendered;
    }
    if let Some(rendered) = describe_cross_segment_chosen_creatures_blessing_program(program) {
        return rendered;
    }
    if let Some(rendered) = describe_cross_segment_relative_player_target_consult_program(program) {
        return rendered;
    }
    if let Some(rendered) = describe_cross_segment_mixed_target_consult_program(program) {
        return rendered;
    }
    if let Some(rendered) = describe_cross_segment_damage_and_die_replacement_program(program) {
        return rendered;
    }
    if let [segment] = program.segments.as_slice()
        && segment.self_replacements.is_empty()
        && let Some(rendered) =
            describe_nested_search_for_each_conditional_shuffle(&segment.default_effects)
    {
        return rendered;
    }

    let mut rendered_segments = Vec::new();
    let mut skipped_segments = 0usize;
    for (segment_index, segment) in program.segments.iter().enumerate() {
        if skipped_segments > 0 {
            skipped_segments -= 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_target_player_choose_half_return_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_attached_tap_equipment_unattach_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_goaded_set_block_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        // Preserve the free-cast payment rider before the broader linked-play
        // window consumes the same exile/grant pair independently.
        if let Some((rendered, consumed)) =
            describe_cross_segment_shuffle_exile_top_free_play_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_linked_exile_top_play_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_may_search_conditional_else_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_countered_spell_may_put_from_hand_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_countered_set_followup_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) = describe_cross_segment_linked_target_set_followup_window(
            &program.segments,
            segment_index,
        ) {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_result_producer_for_each_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_countered_spell_exile_with_counters_gain_suspend_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_put_counters_then_gain_suspend_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        // Exact linked-choice bundles must run before broader two-segment
        // windows consume their opening choices independently.
        if let Some((rendered, consumed)) =
            describe_cross_segment_linked_graveyard_choices_then_may_return_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_choose_exiled_cards_exile_library_put_chosen_on_top_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_source_exiled_graveyard_token_sacrifice_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_search_reveal_opponent_choose_rest_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) = describe_cross_segment_turn_start_hand_conditions_window(
            &program.segments,
            segment_index,
        ) {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_choose_pay_untap_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_search_reveal_may_move_else_hand_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_declined_may_mill_damage_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) = describe_cross_segment_target_players_investigate_window(
            &program.segments,
            segment_index,
        ) {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_reveal_hand_choose_graveyard_or_hand_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_choose_color_reveal_discard_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_may_reveal_hand_choose_action_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_look_hand_choose_action_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_reveal_to_hand_lose_mana_value_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_reveal_hand_choose_discard_adventure_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_exile_then_free_cast_while_exiled_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_choice_move_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) = describe_cross_segment_discard_hand_add_mana_draw_window(
            &program.segments,
            segment_index,
        ) {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) = describe_cross_segment_player_damage_then_discard_window(
            &program.segments,
            segment_index,
        ) {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_draw_then_discard_unless_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_return_animation_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_color_subtype_addition_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_necromentia_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_named_vote_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_named_search_conditional_disposition_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_may_search_conditional_disposition_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_same_name_search_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_target_player_draw_exile_copy_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_same_name_extraction_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_may_look_top_rest_bottom_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_looked_cards_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_choose_two_tap_unattach_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_bulk_return_decayed_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_joint_discard_or_sacrifice_then_draw_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_opponent_choice_returns_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if segment.self_replacements.is_empty()
            && let Some(rendered) = describe_face_down_pile_then_manifest(&segment.default_effects)
        {
            rendered_segments.push(rendered);
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_choose_exiled_card_free_play_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_tagged_continuous_then_counter_conditional_draw_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_tagged_pump_then_conditional_keyword_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_counter_linked_grant_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_draw_count_grant_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_face_down_pile_manifest_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_kicked_targets_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_color_matched_prevention_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_target_creature_damage_then_destroy_attached_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_target_only_damage_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_destroy_then_grant_blocked_by_that_creature_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_target_then_delayed_destroy_blocked_by_that_creature_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_power_damage_exchange_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_quantified_choice_collection_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_chooser_target_exchange_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_created_token_copy_retarget_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_created_token_followup_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_exile_top_put_from_among_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_exile_top_choose_play_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_shuffle_reveal_top_free_play_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_shuffle_exile_top_cast_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_observation_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_reveal_hand_choose_graveyard_exile_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_consult_hand_remainder_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_consult_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_wrapped_search_two_split_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_search_move_shuffle_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_tagged_tap_untap_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_conditional_tagged_tap_untap_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_typed_controller_damage_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_chain_copy_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_look_optional_payment_disposition_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_choose_name_mill_conditional_window(
                &program.segments,
                segment_index,
            )
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_result_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
            continue;
        }
        if let Some((rendered, consumed)) =
            describe_cross_segment_death_replacement_window(&program.segments, segment_index)
        {
            rendered_segments.push(rendered);
            skipped_segments = consumed - 1;
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
            rendered_segments.push(apply_self_replacement_presentation_label(
                &segment.self_replacements[0],
                rendered,
            ));
            continue;
        }

        if !segment.default_effects.is_empty() {
            if let [look_effect, conditional_effect] = segment.default_effects.as_slice()
                && let Some(look) =
                    look_effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>()
                && let Some(conditional) =
                    conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()
                && let Some(rendered) =
                    describe_nested_look_top_card_matching_hand_else_bottom(look, conditional)
            {
                rendered_segments.push(rendered);
                continue;
            }
            if let [choose_effect, cant_effect] = segment.default_effects.as_slice()
                && let Some(rendered) = describe_split_piles_then_choose_attack_or_block_restriction(
                    choose_effect,
                    cant_effect,
                )
            {
                rendered_segments.push(rendered);
                continue;
            }
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
    fn collect_implicit_source_damage<'a>(
        effect: &'a Effect,
        collected: &mut Vec<&'a crate::effects::DealDamageEffect>,
    ) -> Option<()> {
        let effect = unwrap_basic_render_wrapper(effect);
        if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
            if sequence.surface != ironsmith_core::SequenceSurface::Coordinated {
                return None;
            }
            for child in &sequence.effects {
                collect_implicit_source_damage(child, collected)?;
            }
            return Some(());
        }
        if let Some(for_each) = effect.downcast_ref::<crate::effects::ForEachObject>() {
            let [inner] = for_each.effects.as_slice() else {
                return None;
            };
            let (source, damage) = damage_with_source_view(inner)?;
            if source.is_some() || !matches!(damage.target.base(), ChooseSpec::Iterated) {
                return None;
            }
            collected.push(damage);
            return Some(());
        }
        if let Some(for_players) = effect.downcast_ref::<crate::effects::ForPlayersEffect>() {
            let [inner] = for_players.effects.as_slice() else {
                return None;
            };
            let (source, damage) = damage_with_source_view(inner)?;
            if source.is_some()
                || !matches!(
                    damage.target.base(),
                    ChooseSpec::Player(PlayerFilter::IteratedPlayer)
                )
            {
                return None;
            }
            collected.push(damage);
            return Some(());
        }
        let (source, damage) = damage_with_source_view(effect)?;
        if source.is_some() {
            return None;
        }
        collected.push(damage);
        Some(())
    }

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
    if let Some(compact) =
        describe_player_or_planeswalker_damage_then_controlled_creature_damage(first, second)
    {
        return Some(compact);
    }
    if let Some(compact) = describe_joint_subject_pair(first, second) {
        if compact.starts_with("Deal ") {
            return Some(compact);
        }
        if let Some(rest) = compact.strip_prefix("this deals ") {
            return Some(format!("Deal {rest}"));
        }
    }

    let mut damage = Vec::new();
    for effect in effects {
        collect_implicit_source_damage(effect, &mut damage)?;
    }
    let [first_damage, rest @ ..] = damage.as_slice() else {
        return None;
    };
    if rest.is_empty()
        || rest.iter().any(|candidate| {
            candidate.source_is_combat != first_damage.source_is_combat
                || candidate.unpreventable != first_damage.unpreventable
        })
    {
        return None;
    }
    let parts = effects
        .iter()
        .map(describe_effect)
        .map(|rendered| rendered.trim().trim_end_matches('.').to_string())
        .map(|rendered| {
            rendered
                .strip_prefix("this deals ")
                .or_else(|| rendered.strip_prefix("Deal "))
                .map(|damage| {
                    let Some((amount, _)) = damage.split_once(" damage to ") else {
                        return damage.to_string();
                    };
                    damage.replace(&format!(" and deal {amount} damage to "), " and ")
                })
        })
        .collect::<Option<Vec<_>>>()?;
    let joined = match parts.as_slice() {
        [] => return None,
        [only] => only.clone(),
        [first, second] => format!("{first} and {second}"),
        _ => {
            let (last, leading) = parts.split_last()?;
            format!("{}, and {last}", leading.join(", "))
        }
    };
    Some(format!("Deal {joined}"))
}

fn format_self_replacement_fallback(
    default_text: &str,
    condition_text: &str,
    replacement: &str,
    leading_instead_surface: bool,
) -> String {
    if leading_instead_surface || replacement.contains(". ") {
        format!("{default_text}. If {condition_text}, instead {replacement}")
    } else {
        format!("{default_text}. If {condition_text}, {replacement} instead")
    }
}

/// Render the typed replacement that prepends a scry to the same draw action.
/// Requiring the replacement draw to match the default draw exactly prevents
/// this presentation rule from hiding a changed player, count, or draw
/// surface. The leading `instead` is significant for the authored
/// replacement clause, as is the imperative surface of a spell's default
/// draw instruction.
fn describe_scry_then_same_draw_self_replacement(
    segment: &crate::resolution::ResolutionSegment,
) -> Option<String> {
    let [branch] = segment.self_replacements.as_slice() else {
        return None;
    };
    if branch.condition_after_replacement {
        return None;
    }
    let [default_effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let [scry_effect, replacement_draw_effect] = branch.replacement_effects.as_slice() else {
        return None;
    };
    let default_draw = unwrap_basic_render_wrapper(default_effect)
        .downcast_ref::<crate::effects::DrawCardsEffect>()?;
    let scry =
        unwrap_basic_render_wrapper(scry_effect).downcast_ref::<crate::effects::ScryEffect>()?;
    let replacement_draw = unwrap_basic_render_wrapper(replacement_draw_effect)
        .downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if default_draw.player != replacement_draw.player
        || default_draw.count != replacement_draw.count
    {
        return None;
    }

    let rendered_default = describe_effect(default_effect);
    let rendered_default = rendered_default.trim().trim_end_matches('.');
    let default_text = if default_draw.player == PlayerFilter::You {
        rendered_default
            .strip_prefix("You ")
            .or_else(|| rendered_default.strip_prefix("you "))
            .unwrap_or(rendered_default)
    } else {
        rendered_default
    };
    let replacement_text = describe_scry_then_draw(scry, replacement_draw)?;
    let raw_condition_text = super::normalize_common::describe_condition(&branch.condition);
    let condition_text = normalize_target_quality_condition(&default_text, &raw_condition_text);
    Some(format!(
        "{}. If {condition_text}, instead {}",
        capitalize_first(&default_text),
        super::normalize_common::lowercase_first(replacement_text.trim().trim_end_matches('.'))
    ))
}

fn collect_self_replacement_action_targets<'a>(
    effect: &'a Effect,
    targets: &mut Vec<&'a ChooseSpec>,
) {
    let effect = unwrap_basic_render_wrapper(effect);
    if let Some(unless_pays) = effect.downcast_ref::<crate::effects::UnlessPaysEffect>() {
        for nested in &unless_pays.effects {
            collect_self_replacement_action_targets(nested, targets);
        }
        return;
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        for nested in &sequence.effects {
            collect_self_replacement_action_targets(nested, targets);
        }
        return;
    }

    let target = rendered_action_target(effect)
        .or_else(|| {
            effect
                .downcast_ref::<crate::effects::DealDamageEffect>()
                .map(|damage| &damage.target)
        })
        .or_else(|| {
            effect
                .downcast_ref::<crate::effects::CounterEffect>()
                .map(|counter| &counter.target)
        })
        .or_else(|| {
            effect
                .downcast_ref::<crate::effects::ExileEffect>()
                .map(|exile| &exile.spec)
        })
        .or_else(|| {
            effect
                .downcast_ref::<crate::effects::ReturnToHandEffect>()
                .map(|return_to_hand| &return_to_hand.spec)
        })
        .or_else(|| {
            effect
                .downcast_ref::<crate::effects::MoveToZoneEffect>()
                .map(|move_to_zone| &move_to_zone.target)
        })
        .or_else(|| {
            effect
                .downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()
                .map(|return_to_hand| &return_to_hand.target)
        })
        .or_else(|| {
            effect
                .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
                .map(|return_to_battlefield| &return_to_battlefield.target)
        })
        .or_else(|| {
            effect
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .map(|target_only| &target_only.target)
        });
    if let Some(target) = target {
        targets.push(target);
    }
}

/// Return the one target selection shared by every target-bearing action in
/// this branch. Follow-up actions without targets (for example, scrying) are
/// ignored, while a second, different target makes the branch ineligible for
/// implicit-reference rendering.
fn shared_self_replacement_action_target(effects: &[Effect]) -> Option<&ChooseSpec> {
    let mut targets = Vec::new();
    for effect in effects {
        collect_self_replacement_action_targets(effect, &mut targets);
    }
    let first = *targets.first()?;
    targets
        .iter()
        .all(|target| target_specs_select_same_objects(first, target))
        .then_some(first)
}

fn self_replacement_target_referent(target: &ChooseSpec) -> Option<&'static str> {
    match target.base() {
        ChooseSpec::AnyTarget | ChooseSpec::AnyOtherTarget => Some("that permanent or player"),
        ChooseSpec::Player(_) | ChooseSpec::SpecificPlayer(_) => Some("that player"),
        ChooseSpec::PlayerOrPlaneswalker(_) | ChooseSpec::ObjectOrPlayer(_, _) => {
            Some("that permanent or player")
        }
        ChooseSpec::Object(filter) => {
            if let Some(stack_kind) = filter.stack_kind {
                return Some(match stack_kind {
                    crate::filter::StackObjectKind::Spell => "that spell",
                    crate::filter::StackObjectKind::Ability
                    | crate::filter::StackObjectKind::ActivatedAbility
                    | crate::filter::StackObjectKind::TriggeredAbility => "that ability",
                    crate::filter::StackObjectKind::SpellOrAbility => "that spell or ability",
                });
            }
            if filter.zone.is_some_and(|zone| zone != Zone::Battlefield) {
                return Some("that card");
            }
            if filter.card_types.as_slice() == [CardType::Creature] {
                return Some("that creature");
            }
            Some("that permanent")
        }
        _ => None,
    }
}

fn condition_refers_to_self_replacement_target(condition_text: &str, referent: &str) -> bool {
    let lower = condition_text.to_ascii_lowercase();
    lower.contains(referent)
        || lower.starts_with("it's ")
        || lower.starts_with("it is ")
        || lower.starts_with("it was ")
        || lower.starts_with("it has ")
}

fn first_self_replacement_action_moves_target_to_card_zone(effects: &[Effect]) -> bool {
    fn first_action(effect: &Effect) -> Option<&Effect> {
        let effect = unwrap_basic_render_wrapper(effect);
        if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
            return sequence.effects.iter().find_map(first_action);
        }
        rendered_action_target(effect)
            .or_else(|| {
                effect
                    .downcast_ref::<crate::effects::ExileEffect>()
                    .map(|exile| &exile.spec)
            })
            .or_else(|| {
                effect
                    .downcast_ref::<crate::effects::ReturnToHandEffect>()
                    .map(|return_to_hand| &return_to_hand.spec)
            })
            .or_else(|| {
                effect
                    .downcast_ref::<crate::effects::MoveToZoneEffect>()
                    .map(|move_to_zone| &move_to_zone.target)
            })
            .map(|_| effect)
    }

    let effect = effects.iter().find_map(first_action);
    effect.is_some_and(|effect| {
        effect
            .downcast_ref::<crate::effects::ExileEffect>()
            .is_some()
            || effect
                .downcast_ref::<crate::effects::ReturnToHandEffect>()
                .is_some()
            || effect
                .downcast_ref::<crate::effects::MoveToZoneEffect>()
                .is_some_and(|move_to_zone| move_to_zone.zone != Zone::Battlefield)
            || effect
                .downcast_ref::<crate::effects::DestroyEffect>()
                .is_some()
    })
}

/// Replace a repeated rendered target only after typed target identity proves
/// that the replacement action reuses the default action's target. The text
/// operation is deliberately limited to target surfaces produced from that
/// same `ChooseSpec`. A sequential zone move can mention the same target in
/// more than one action; after the first move, later references use the card
/// referent rather than repeating a fresh target selection.
fn describe_shared_target_self_replacement(
    default_effects: &[Effect],
    replacement_effects: &[Effect],
    default_text: &str,
    replacement_text: &str,
    condition_text: &str,
    leading_instead_surface: bool,
) -> Option<String> {
    let default_target = shared_self_replacement_action_target(default_effects)?;
    let replacement_target = shared_self_replacement_action_target(replacement_effects)?;
    if !default_target.is_target()
        || !default_target.is_single()
        || !replacement_target.is_target()
        || !replacement_target.is_single()
        || !target_specs_select_same_objects(default_target, replacement_target)
    {
        return None;
    }

    let target_surface = super::normalize_common::describe_choose_spec(replacement_target);
    let mut replacement = super::normalize_common::lowercase_first(replacement_text)
        .trim_end_matches('.')
        .to_string();
    let target_starts = replacement
        .match_indices(target_surface.as_str())
        .map(|(start, _)| start)
        .collect::<Vec<_>>();
    let target_start = *target_starts.first()?;
    let referent = self_replacement_target_referent(default_target)?;
    let before_target = &replacement[..target_start];
    let first_referent = if before_target.ends_with(" on ")
        || condition_refers_to_self_replacement_target(condition_text, referent)
    {
        "it"
    } else {
        referent
    };
    if target_starts.len() == 1 {
        replacement.replace_range(
            target_start..target_start + target_surface.len(),
            first_referent,
        );
    } else {
        let later_referent =
            if first_self_replacement_action_moves_target_to_card_zone(replacement_effects) {
                "that card"
            } else {
                "it"
            };
        let mut compact = String::with_capacity(replacement.len());
        let mut cursor = 0usize;
        for (index, start) in target_starts.into_iter().enumerate() {
            compact.push_str(&replacement[cursor..start]);
            compact.push_str(if index == 0 {
                first_referent
            } else {
                later_referent
            });
            cursor = start + target_surface.len();
        }
        compact.push_str(&replacement[cursor..]);
        replacement = compact;
    }

    Some(format_self_replacement_fallback(
        default_text,
        condition_text,
        &replacement,
        leading_instead_surface,
    ))
}

/// Render the common "same target, different damage result" self-replacement
/// directly from its damage nodes. This preserves an attached scry action or
/// the unpreventable-damage rider, neither of which survives the generic
/// one-damage-effect conditional compactor.
fn describe_shared_target_damage_self_replacement(
    segment: &crate::resolution::ResolutionSegment,
) -> Option<String> {
    let [branch] = segment.self_replacements.as_slice() else {
        return None;
    };
    if branch.condition_after_replacement {
        return None;
    }
    let [default_effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let default_damage = self_replacement_damage_view(default_effect)?;
    let (replacement_effect, scry) = match branch.replacement_effects.as_slice() {
        [replacement_effect] => (replacement_effect, None),
        [replacement_effect, scry_effect] => (
            replacement_effect,
            Some(
                unwrap_basic_render_wrapper(scry_effect)
                    .downcast_ref::<crate::effects::ScryEffect>()?,
            ),
        ),
        _ => return None,
    };
    let replacement_damage = self_replacement_damage_view(replacement_effect)?;
    if default_damage.source_is_combat
        || default_damage.unpreventable
        || replacement_damage.source_is_combat
        || !default_damage.target.is_target()
        || !default_damage.target.is_single()
        || !replacement_damage.target.is_target()
        || !replacement_damage.target.is_single()
        || !target_specs_select_same_objects(&default_damage.target, &replacement_damage.target)
    {
        return None;
    }
    if let Some(scry) = scry
        && scry.player != PlayerFilter::You
    {
        return None;
    }

    let referent = self_replacement_target_referent(&default_damage.target)?;
    let plain_amount_override = scry.is_none() && !replacement_damage.unpreventable;
    let target_suffix = if plain_amount_override {
        String::new()
    } else {
        format!(" to {referent}")
    };
    let mut replacement = format!(
        "it deals {} damage{}",
        super::normalize_common::describe_value(&replacement_damage.amount),
        target_suffix
    );
    if let Some(scry) = scry {
        replacement.push_str(&format!(
            " and you scry {}",
            super::normalize_common::describe_value(&scry.count)
        ));
    }
    if replacement_damage.unpreventable {
        replacement.push_str(" and the damage can't be prevented");
    }
    let default_text = describe_effect_list(&segment.default_effects)
        .trim()
        .trim_end_matches('.')
        .to_string();
    let raw_condition_text = super::normalize_common::describe_condition(&branch.condition);
    let condition_text = normalize_target_quality_condition(&default_text, &raw_condition_text);
    if plain_amount_override {
        return Some(format!(
            "{default_text}. If {condition_text}, {replacement} instead"
        ));
    }
    Some(format!(
        "{}. If {condition_text}, instead {replacement}",
        default_text
    ))
}

/// View damage through the bookkeeping wrappers used by self-replacement
/// lowering. `ExecuteWithSourceEffect` preserves the authored damage source;
/// it does not change the target, amount, or prevention semantics that this
/// renderer compares.
fn self_replacement_damage_view(effect: &Effect) -> Option<&crate::effects::DealDamageEffect> {
    let effect = unwrap_basic_render_wrapper(effect);
    let effect = effect
        .downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
        .map(|execute| unwrap_basic_render_wrapper(&execute.effect))
        .unwrap_or(effect);
    effect.downcast_ref::<crate::effects::DealDamageEffect>()
}

/// Render a trailing-condition damage replacement only when the typed damage
/// nodes prove that both instructions use the same source and target. A bare
/// damage effect implicitly uses the resolving spell or ability as its source,
/// so an explicit `ChooseSpec::Source` wrapper is equivalent to that implicit
/// source; no other implicit/explicit source pairing is safe to collapse to
/// the pronoun "It."
fn describe_trailing_same_source_damage_self_replacement(
    segment: &crate::resolution::ResolutionSegment,
) -> Option<String> {
    let [branch] = segment.self_replacements.as_slice() else {
        return None;
    };
    if !branch.condition_after_replacement {
        return None;
    }
    let [default_effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let [replacement_effect] = branch.replacement_effects.as_slice() else {
        return None;
    };
    let default_damage_tag = &default_effect
        .downcast_ref::<crate::effects::TaggedEffect>()?
        .tag;
    let Condition::TaggedObjectMatches(condition_tag, _) = &branch.condition else {
        return None;
    };
    if condition_tag != default_damage_tag
        || !effect_has_result_tag(replacement_effect, default_damage_tag)
    {
        return None;
    }
    let (default_source, default_damage) = damage_with_source_view(default_effect)?;
    let (replacement_source, replacement_damage) = damage_with_source_view(replacement_effect)?;
    let sources_match = match (default_source, replacement_source) {
        (None, None) => true,
        (None, Some(source)) | (Some(source), None) => {
            matches!(source.unhinted(), ChooseSpec::Source)
        }
        (Some(default_source), Some(replacement_source)) => {
            default_source.unhinted() == replacement_source.unhinted()
        }
    };
    if !sources_match
        || default_damage.source_is_combat
        || replacement_damage.source_is_combat
        || default_damage.unpreventable
        || replacement_damage.unpreventable
        || !default_damage.target.is_target()
        || !default_damage.target.is_single()
        || !replacement_damage.target.is_target()
        || !replacement_damage.target.is_single()
        || !target_specs_select_same_objects(&default_damage.target, &replacement_damage.target)
    {
        return None;
    }

    let referent = self_replacement_target_referent(&default_damage.target)?;
    let default_text = describe_effect_list(&segment.default_effects);
    let default_text = default_text.trim().trim_end_matches('.');
    let raw_condition_text = super::normalize_common::describe_condition(&branch.condition);
    let condition_text = normalize_target_quality_condition(default_text, &raw_condition_text);
    Some(format!(
        "{default_text}. It deals {} damage to {referent} instead if {condition_text}",
        super::normalize_common::describe_value(&replacement_damage.amount)
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TargetThresholdCharacteristic {
    Toughness,
    ManaValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TargetThresholdAction {
    Destroy,
    GainControl,
}

struct TargetThresholdConditional<'a> {
    characteristic: TargetThresholdCharacteristic,
    limit: Value,
    action: TargetThresholdAction,
    action_effect: &'a Effect,
    action_target: &'a ChooseSpec,
    characteristic_target: ChooseSpec,
}

fn target_threshold_action(effect: &Effect) -> Option<(TargetThresholdAction, &ChooseSpec)> {
    let action = unwrap_basic_render_wrapper(effect);
    if let Some(destroy) = action.downcast_ref::<crate::effects::DestroyEffect>() {
        return Some((TargetThresholdAction::Destroy, &destroy.spec));
    }
    let apply = action.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if apply.until != Until::Forever
        || apply.modification.is_some()
        || !apply.additional_modifications.is_empty()
        || apply.condition.is_some()
        || !matches!(
            apply.runtime_modifications.as_slice(),
            [crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController]
        )
    {
        return None;
    }
    Some((
        TargetThresholdAction::GainControl,
        apply.target_spec.as_ref()?,
    ))
}

fn effect_has_result_tag(effect: &Effect, expected: &TagKey) -> bool {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return &tagged.tag == expected || effect_has_result_tag(&tagged.effect, expected);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return effect_has_result_tag(&with_id.effect, expected);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return &tag_all.tag == expected || effect_has_result_tag(&tag_all.effect, expected);
    }
    false
}

fn characteristic_reference_matches_action(
    characteristic_target: &ChooseSpec,
    action_target: &ChooseSpec,
    action_effect: &Effect,
) -> bool {
    if target_specs_select_same_objects(characteristic_target, action_target) {
        return true;
    }
    matches!(
        characteristic_target.base(),
        ChooseSpec::Tagged(tag)
            if matches!(action_target.base(), ChooseSpec::Tagged(action_tag) if action_tag == tag)
                || effect_has_result_tag(action_effect, tag)
    )
}

fn target_threshold_conditional(effect: &Effect) -> Option<TargetThresholdConditional<'_>> {
    let conditional =
        unwrap_basic_render_wrapper(effect).downcast_ref::<crate::effects::ConditionalEffect>()?;
    if conditional.surface != ironsmith_core::ConditionalSurface::TrailingIf
        || !conditional.if_false.is_empty()
    {
        return None;
    }
    let [action_effect] = conditional.if_true.as_slice() else {
        return None;
    };
    let (action, action_target) = target_threshold_action(action_effect)?;
    let (characteristic, characteristic_target, limit) = match &conditional.condition {
        Condition::ValueComparison {
            left,
            operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
            right,
        } if matches!(right.unhinted(), Value::Fixed(_) | Value::X) => {
            let (characteristic, target) = match left.unhinted() {
                Value::ToughnessOf(target) => {
                    (TargetThresholdCharacteristic::Toughness, target.as_ref())
                }
                Value::ManaValueOf(target) => {
                    (TargetThresholdCharacteristic::ManaValue, target.as_ref())
                }
                _ => return None,
            };
            (characteristic, target.clone(), right.clone())
        }
        Condition::TaggedObjectMatches(tag, filter) => {
            let mut residual = filter.clone();
            let (characteristic, limit) = match (&filter.toughness, &filter.mana_value) {
                (Some(crate::filter::Comparison::LessThanOrEqual(limit)), None) => {
                    residual.toughness = None;
                    (TargetThresholdCharacteristic::Toughness, *limit)
                }
                (None, Some(crate::filter::Comparison::LessThanOrEqual(limit))) => {
                    residual.mana_value = None;
                    (TargetThresholdCharacteristic::ManaValue, *limit)
                }
                _ => return None,
            };
            if residual != ObjectFilter::default() {
                return None;
            }
            (
                characteristic,
                ChooseSpec::Tagged(tag.clone()),
                Value::Fixed(limit),
            )
        }
        _ => return None,
    };
    if !characteristic_reference_matches_action(
        &characteristic_target,
        action_target,
        action_effect,
    ) {
        return None;
    }
    Some(TargetThresholdConditional {
        characteristic,
        limit,
        action,
        action_effect,
        action_target,
        characteristic_target,
    })
}

/// Fold a synthesized target declaration back into two threshold-guarded uses
/// of that same target. Both conditional actions and both characteristic
/// references must be linked to the one typed target before this emits the
/// authored pronouns.
fn describe_target_threshold_paid_cost_self_replacement(
    segment: &crate::resolution::ResolutionSegment,
) -> Option<String> {
    let [target_effect, default_effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let target_only = unwrap_basic_render_wrapper(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if target_only.explicit_declaration {
        return None;
    }
    exact_single_target_object_filter(&target_only.target)?;
    let [branch] = segment.self_replacements.as_slice() else {
        return None;
    };
    if branch.condition_after_replacement || branch.presentation_label.is_some() {
        return None;
    }
    let replacement_effect = match branch.replacement_effects.as_slice() {
        [effect] => effect,
        [replacement_target_effect, effect] => {
            let replacement_target = unwrap_basic_render_wrapper(replacement_target_effect)
                .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
            if replacement_target.explicit_declaration
                || !target_specs_select_same_objects(
                    &replacement_target.target,
                    &target_only.target,
                )
            {
                return None;
            }
            effect
        }
        _ => return None,
    };
    let default = target_threshold_conditional(default_effect)?;
    let replacement = target_threshold_conditional(replacement_effect)?;
    if default.characteristic != replacement.characteristic
        || default.action != replacement.action
        || !target_specs_select_same_objects(default.action_target, &target_only.target)
        || !target_specs_select_same_objects(replacement.action_target, &target_only.target)
        || !characteristic_reference_matches_action(
            &default.characteristic_target,
            default.action_target,
            default.action_effect,
        )
        || !characteristic_reference_matches_action(
            &replacement.characteristic_target,
            replacement.action_target,
            replacement.action_effect,
        )
    {
        return None;
    }
    if !matches!(
        &branch.condition,
        Condition::ThisSpellWasKicked
            | Condition::ThisSpellPaidLabel(_)
            | Condition::TurnHistory(ironsmith_core::TurnHistoryCondition::SourceWasKicked { .. })
    ) {
        return None;
    }

    let target_phrase = describe_choose_spec(&target_only.target);
    let target_noun = target_phrase.strip_prefix("target ")?;
    if target_noun.is_empty() || target_noun.split_whitespace().count() != 1 {
        return None;
    }
    let characteristic = match default.characteristic {
        TargetThresholdCharacteristic::Toughness => "toughness",
        TargetThresholdCharacteristic::ManaValue => "mana value",
    };
    let (default_action, replacement_action) = match default.action {
        TargetThresholdAction::Destroy => (
            format!("Destroy target {target_noun}"),
            format!("destroy that {target_noun}"),
        ),
        TargetThresholdAction::GainControl => (
            format!("Gain control of target {target_noun}"),
            format!("gain control of that {target_noun}"),
        ),
    };
    let default_limit = super::normalize_common::describe_value(&default.limit);
    let replacement_limit = super::normalize_common::describe_value(&replacement.limit);
    let paid_condition = super::normalize_common::describe_condition(&branch.condition);
    let default_clause =
        format!("{default_action} if its {characteristic} is {default_limit} or less");
    let replacement_clause =
        format!("{replacement_action} if its {characteristic} is {replacement_limit} or less");
    if branch.leading_instead_surface {
        Some(format!(
            "{default_clause}. If {paid_condition}, instead {replacement_clause}"
        ))
    } else {
        Some(format!(
            "{default_clause}. If {paid_condition}, {replacement_clause} instead"
        ))
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
            branch.leading_instead_surface,
        ));
    }
    if let Some(rendered) = describe_scry_then_same_draw_self_replacement(segment) {
        return Some(rendered);
    }
    if let Some(rendered) = describe_looked_and_or_destination_self_replacement(segment) {
        return Some(rendered);
    }
    if let Some(rendered) = describe_looked_count_self_replacement(segment) {
        return Some(rendered);
    }
    if let Some(rendered) = describe_trailing_same_source_damage_self_replacement(segment) {
        return Some(rendered);
    }
    if let Some(rendered) = describe_shared_target_damage_self_replacement(segment) {
        return Some(rendered);
    }
    if let Some(rendered) = describe_target_threshold_paid_cost_self_replacement(segment) {
        return Some(rendered);
    }
    if !branch.condition_after_replacement
        && let [default_effect] = segment.default_effects.as_slice()
        && default_effect
            .downcast_ref::<crate::effects::DiscardEffect>()
            .is_some()
    {
        let replacement_refs = branch.replacement_effects.iter().collect::<Vec<_>>();
        if let Some(inline) = describe_reveal_hand_choose_discard_inline(&replacement_refs) {
            let default_text = describe_effect_list(&segment.default_effects);
            let condition_text = super::normalize_common::describe_condition(&branch.condition);
            let rendered = format!(
                "{default_text}. If {condition_text}, instead {}",
                super::normalize_common::lowercase_first(&inline)
            );
            return Some(super::normalize_common::capitalize_first(&rendered));
        }
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
    if !branch.condition_after_replacement
        && let Some(shared_target_text) = describe_shared_target_self_replacement(
            &segment.default_effects,
            &branch.replacement_effects,
            &default_text,
            &replacement_text,
            &condition_text,
            branch.leading_instead_surface,
        )
    {
        return Some(shared_target_text);
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
        branch.leading_instead_surface,
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
        let [
            choose_effect,
            reveal_effect,
            for_each_effect,
            shuffle_effect,
        ] = effects
        else {
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
    let return_to_hand = unwrap_basic_render_wrapper(default_effect)
        .downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()?;
    let replacement_effect = unwrap_basic_render_wrapper(replacement_effect);
    let (replacement_target, tapped, verb) = if let Some(return_to_battlefield) =
        replacement_effect.downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
    {
        if return_to_battlefield.as_aura.is_some()
            || !return_to_battlefield.enters_with_counters.is_empty()
        {
            return None;
        }
        (
            &return_to_battlefield.target,
            return_to_battlefield.tapped,
            "return",
        )
    } else if let Some(move_to_zone) =
        replacement_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()
    {
        if move_to_zone.zone != Zone::Battlefield
            || move_to_zone.to_top
            || move_to_zone.library_order.is_some()
            || move_to_zone.target_plural_surface
            || move_to_zone.actor_surface.is_some()
            || move_to_zone.destination_player_surface.is_some()
            || move_to_zone.destination_player_reference_surface.is_some()
            || move_to_zone.exiled_with_source_surface.is_some()
            || move_to_zone.battlefield_controller
                != crate::effects::BattlefieldController::Preserve
            || move_to_zone.controller_surface_explicit
            || !move_to_zone.enters_with_counters.is_empty()
            || move_to_zone.enters_attacking
            || move_to_zone.attack_target_mode.is_some()
            || move_to_zone.enters_face_down
            || move_to_zone.enters_transformed
            || move_to_zone.transfer_exiled_with_source_links
        {
            return None;
        }
        let verb = match move_to_zone.verb_surface {
            ironsmith_core::MoveToZoneVerbSurface::Put => "put",
            ironsmith_core::MoveToZoneVerbSurface::Return => "return",
            _ => return None,
        };
        (&move_to_zone.target, move_to_zone.enters_tapped, verb)
    } else {
        return None;
    };
    if !return_to_hand.target.is_target()
        || !return_to_hand.target.is_single()
        || !replacement_target.is_target()
        || !replacement_target.is_single()
        || !target_specs_select_same_objects(&return_to_hand.target, replacement_target)
    {
        return None;
    }
    let destination = if verb == "put" {
        "onto the battlefield"
    } else {
        "to the battlefield"
    };
    let replacement = format!(
        "{verb} that card {destination}{} instead",
        if tapped { " tapped" } else { "" }
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
    let default_lower = default_text.to_ascii_lowercase();
    if default_lower.contains("target creature")
        && let Some(quality) = condition_text.strip_prefix("that creature is a ")
    {
        if let Some(property) = quality
            .strip_prefix("creature with ")
            .or_else(|| quality.strip_prefix("permanent with "))
        {
            return format!("that creature has {property}");
        }
        if let Some(colors) = quality.strip_suffix(" permanent")
            && colors
                .split_whitespace()
                .all(|word| word == "or" || is_color_quality_word(word))
        {
            return format!("that creature is {colors}");
        }
        if quality.ends_with(" creature") {
            return format!("it's {}", with_indefinite_article(quality));
        }
    }
    if let Some(quality) = condition_text
        .strip_suffix(" was dealt damage this way")
        .and_then(|subject| {
            subject
                .strip_prefix("a ")
                .or_else(|| subject.strip_prefix("an "))
        })
    {
        let target_subject = if default_lower.contains("target creature or planeswalker")
            || default_lower.contains("target red creature or planeswalker")
            || default_lower.contains("target permanent")
        {
            Some("that permanent")
        } else if default_lower.contains("target creature") {
            Some("that creature")
        } else if default_lower.contains("any target") {
            Some("it")
        } else {
            None
        };
        if let Some(subject) = target_subject {
            if let Some(property) = quality
                .strip_prefix("creature with ")
                .or_else(|| quality.strip_prefix("permanent with "))
            {
                return format!("{subject} has {property}");
            }
            if let Some(colors) = quality.strip_suffix(" permanent")
                && colors
                    .split_whitespace()
                    .all(|word| word == "or" || is_color_quality_word(word))
            {
                return format!("{subject} is {colors}");
            }
            if subject == "that creature" && quality.ends_with(" creature") {
                return format!("it's {}", with_indefinite_article(quality));
            }
            return format!("{subject} is {}", with_indefinite_article(quality));
        }
    }

    let mut creature_quality = None;
    if let Some(quality) = condition_text.strip_prefix("it's a ") {
        creature_quality = Some(quality);
    }
    let Some(quality) = condition_text
        .strip_prefix("it's a ")
        .and_then(|rest| rest.strip_suffix(" permanent"))
    else {
        if default_lower.contains("target") && default_lower.contains("creature") {
            if let Some(quality) = creature_quality {
                if let Some(rest) = quality.strip_prefix("creature with ") {
                    return format!("that creature has {rest}");
                }
                if let Some(rest) = quality.strip_prefix("permanent with ") {
                    return format!("that creature has {rest}");
                }
                if quality.ends_with(" creature") {
                    return condition_text.to_string();
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
    describe_create_token_blueprint(default_create)
        == describe_create_token_blueprint(replacement_create)
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
    let mut token = describe_create_token_blueprint(create);
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

fn describe_resolution_program_preserving_source_lines(
    program: &crate::resolution::ResolutionProgram,
) -> String {
    if !program
        .segments
        .iter()
        .any(|segment| segment.starts_new_source_line)
    {
        return describe_resolution_program(program);
    }

    let mut source_lines = Vec::new();
    let mut line_segments = Vec::new();
    for segment in &program.segments {
        if segment.starts_new_source_line && !line_segments.is_empty() {
            source_lines.push(describe_resolution_program(
                &crate::resolution::ResolutionProgram::new(std::mem::take(&mut line_segments)),
            ));
        }
        let mut segment = segment.clone();
        segment.starts_new_source_line = false;
        line_segments.push(segment);
    }
    if !line_segments.is_empty() {
        source_lines.push(describe_resolution_program(
            &crate::resolution::ResolutionProgram::new(line_segments),
        ));
    }
    source_lines
        .into_iter()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn bracket_removed_cleave_words(default_text: &str, cleaved_text: &str) -> Option<String> {
    // This word-level reconstruction is deliberately scoped to one authored
    // instruction. Collapsing whitespace across distinct source lines would
    // discard the line provenance preserved by the card-level renderer.
    if default_text.contains('\n') || cleaved_text.contains('\n') {
        return None;
    }
    let default_words = default_text.split_whitespace().collect::<Vec<_>>();
    let cleaved_words = cleaved_text.split_whitespace().collect::<Vec<_>>();
    if default_words.is_empty()
        || cleaved_words.is_empty()
        || default_words.len() <= cleaved_words.len()
    {
        return None;
    }

    let mut lcs = vec![vec![0usize; cleaved_words.len() + 1]; default_words.len() + 1];
    for default_index in (0..default_words.len()).rev() {
        for cleaved_index in (0..cleaved_words.len()).rev() {
            lcs[default_index][cleaved_index] = if default_words[default_index]
                .eq_ignore_ascii_case(cleaved_words[cleaved_index])
            {
                1 + lcs[default_index + 1][cleaved_index + 1]
            } else {
                lcs[default_index + 1][cleaved_index].max(lcs[default_index][cleaved_index + 1])
            };
        }
    }
    if lcs[0][0] != cleaved_words.len() {
        return None;
    }

    let mut retained = vec![false; default_words.len()];
    let (mut default_index, mut cleaved_index) = (0usize, 0usize);
    while default_index < default_words.len() && cleaved_index < cleaved_words.len() {
        if default_words[default_index].eq_ignore_ascii_case(cleaved_words[cleaved_index])
            && lcs[default_index][cleaved_index] == 1 + lcs[default_index + 1][cleaved_index + 1]
        {
            retained[default_index] = true;
            default_index += 1;
            cleaved_index += 1;
        } else if lcs[default_index + 1][cleaved_index] >= lcs[default_index][cleaved_index + 1] {
            default_index += 1;
        } else {
            cleaved_index += 1;
        }
    }

    let mut rendered = Vec::new();
    let mut index = 0usize;
    while index < default_words.len() {
        if retained[index] {
            rendered.push(default_words[index].to_string());
            index += 1;
            continue;
        }
        let start = index;
        while index < default_words.len() && !retained[index] {
            index += 1;
        }
        rendered.push(format!("[{}]", default_words[start..index].join(" ")));
    }
    Some(rendered.join(" "))
}

fn preserve_cleave_bracket_surface(def: &CardDefinition, default_text: &str) -> Option<String> {
    let cleaved_effects = def
        .alternative_casts
        .iter()
        .find_map(AlternativeCastingMethod::cleave_effects)?;
    let cleaved_text = describe_effect_list(cleaved_effects);
    bracket_removed_cleave_words(default_text, &cleaved_text)
}

fn describe_named_spell_shared_target_damage_self_replacement(
    def: &CardDefinition,
    program: &crate::resolution::ResolutionProgram,
) -> Option<String> {
    if !(def.card.is_instant() || def.card.is_sorcery())
        || def.card.name.contains(" // ")
        || !def.optional_costs.is_empty()
    {
        return None;
    }
    let [segment] = program.segments.as_slice() else {
        return None;
    };
    let [branch] = segment.self_replacements.as_slice() else {
        return None;
    };
    if branch.condition_after_replacement
        || branch.leading_instead_surface
        || branch.presentation_label.is_some()
    {
        return None;
    }
    let [default_effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let [replacement_effect] = branch.replacement_effects.as_slice() else {
        return None;
    };
    let (default_source, default_damage) = damage_with_source_view(default_effect)?;
    let (replacement_source, replacement_damage) = damage_with_source_view(replacement_effect)?;
    let source_is_resolving_spell = |source: Option<&ChooseSpec>| {
        source.is_none()
            || source.is_some_and(|source| matches!(source.unhinted(), ChooseSpec::Source))
    };
    if !source_is_resolving_spell(default_source)
        || !source_is_resolving_spell(replacement_source)
        || default_damage.source_is_combat
        || default_damage.unpreventable
        || replacement_damage.source_is_combat
        || replacement_damage.unpreventable
        || !default_damage.target.is_target()
        || !default_damage.target.is_single()
        || !replacement_damage.target.is_target()
        || !replacement_damage.target.is_single()
        || !target_specs_select_same_objects(&default_damage.target, &replacement_damage.target)
        || self_replacement_target_referent(&default_damage.target) != Some("that creature")
    {
        return None;
    }

    let default_text = rewrite_spell_resolution_damage_source(
        def,
        describe_effect_list(&segment.default_effects)
            .trim()
            .trim_end_matches('.'),
    );
    let raw_condition = super::normalize_common::describe_condition(&branch.condition);
    let condition = normalize_target_quality_condition(&default_text, &raw_condition);
    Some(format!(
        "{default_text}. If {condition}, {} deals {} damage to it instead",
        def.card.name,
        super::normalize_common::describe_value(&replacement_damage.amount)
    ))
}

#[cfg(test)]
mod source_line_program_tests {
    use super::*;

    #[test]
    fn card_level_program_renderer_preserves_authored_line_boundaries() {
        let first = crate::resolution::ResolutionSegment::from_effects(vec![Effect::new(
            crate::effects::DrawCardsEffect::new(1, PlayerFilter::You),
        )]);
        let mut second =
            crate::resolution::ResolutionSegment::from_effects(vec![Effect::gain_life(1)]);
        second.starts_new_source_line = true;
        let program = crate::resolution::ResolutionProgram::new(vec![first, second]);

        assert_eq!(
            describe_resolution_program_preserving_source_lines(&program),
            "You draw a card\nYou gain 1 life"
        );
    }

    #[test]
    fn cleave_word_diff_brackets_each_removed_source_span() {
        assert_eq!(
            bracket_removed_cleave_words(
                "Search your library for a basic land card, reveal it, put it into your hand, then shuffle",
                "Search your library for a card, put it into your hand, then shuffle",
            )
            .as_deref(),
            Some(
                "Search your library for a [basic land] card, [reveal it,] put it into your hand, then shuffle"
            )
        );
        assert_eq!(
            bracket_removed_cleave_words(
                "Return target nonland permanent you control to its owner's hand",
                "Return target nonland permanent to its owner's hand",
            )
            .as_deref(),
            Some("Return target nonland permanent [you control] to its owner's hand")
        );
        assert_eq!(
            bracket_removed_cleave_words("Draw a card\nGain 2 life", "Draw a card"),
            None,
            "cleave reconstruction must not erase authored source lines"
        );
    }

    #[test]
    fn cleave_card_renderer_preserves_cast_origin_restriction_brackets() {
        let oracle = "Cleave {1}{U}{U}\nCounter target spell [that wasn't cast from its owner's hand].";
        let definition = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Wash Away",
        )
        .card_types(vec![CardType::Instant])
        .parse_text(oracle)
        .expect("Wash Away should compile");

        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition).join("\n"),
            oracle
        );
    }

    #[test]
    fn cleave_card_renderer_preserves_mana_value_restriction_brackets() {
        let oracle =
            "Cleave {4}{W}{B}\nDestroy all creatures [with mana value 2 or less].";
        let definition = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Path of Peril",
        )
        .card_types(vec![CardType::Sorcery])
        .parse_text(oracle)
        .expect("Path of Peril should compile");

        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition).join("\n"),
            oracle
        );
    }

    #[test]
    fn cleave_card_renderer_preserves_other_frozen_restriction_brackets() {
        for (name, card_type, oracle) in [
            (
                "Dig Up",
                CardType::Sorcery,
                "Cleave {1}{B}{B}{G}\nSearch your library for a [basic land] card, [reveal it,] put it into your hand, then shuffle.",
            ),
            (
                "Dread Fugue",
                CardType::Sorcery,
                "Cleave {2}{B}\nTarget player reveals their hand. You choose a nonland card from it [with mana value 2 or less]. That player discards that card.",
            ),
            (
                "Fierce Retribution",
                CardType::Instant,
                "Cleave {5}{W}\nDestroy target [attacking] creature.",
            ),
        ] {
            let definition = crate::cards::builders::CardDefinitionBuilder::new(
                crate::ids::CardId::new(),
                name,
            )
            .card_types(vec![card_type])
            .parse_text(oracle)
            .expect("frozen Cleave card should compile");

            assert_eq!(
                crate::compiled_text::compiled_text_lines(&definition).join("\n"),
                oracle,
                "{name}"
            );
        }
    }
}

fn describe_resolution_program_for_card(
    def: &CardDefinition,
    program: &crate::resolution::ResolutionProgram,
) -> String {
    if let Some(rendered) = describe_named_spell_shared_target_damage_self_replacement(def, program)
    {
        return rendered;
    }
    let has_visible_gift_line = def
        .optional_costs
        .iter()
        .any(|cost| matches!(cost.kind, crate::cost::OptionalCostKind::Gift));
    if !has_visible_gift_line {
        let rendered = describe_resolution_program_preserving_source_lines(program);
        return rewrite_spell_resolution_damage_source(def, &rendered);
    }

    // Gift setup is represented as a runtime-only resolution segment. Remove
    // only that recognized payload, then render the remaining program as a
    // whole so cross-sentence relationships (target pairs, replacements,
    // consult pipelines, and similar structures) remain visible to the same
    // structural matchers used by non-Gift cards.
    let visible_segments = program
        .segments
        .iter()
        .filter(|segment| !is_hidden_gift_resolution_segment(segment))
        .cloned()
        .collect();
    let visible_program = crate::resolution::ResolutionProgram::new(visible_segments);
    let rendered = describe_resolution_program_preserving_source_lines(&visible_program);
    rewrite_spell_resolution_damage_source(def, &rendered)
}

/// A direct Spree program is one authored modal source block: its header and
/// every cost-labeled mode are distinct source lines even though they execute
/// through one typed `ChooseModeEffect`.
fn direct_spree_mode_count(program: &crate::resolution::ResolutionProgram) -> Option<usize> {
    let [segment] = program.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let [effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let modal = effect.downcast_ref::<crate::effects::ChooseModeEffect>()?;
    (modal.spree
        && !modal.modes.is_empty()
        && modal.mode_additional_mana_costs.len() == modal.modes.len())
    .then_some(modal.modes.len())
}

fn typed_spree_source_lines(
    program: &crate::resolution::ResolutionProgram,
    rendered: &str,
) -> Option<Vec<String>> {
    let mode_count = direct_spree_mode_count(program)?;
    let lines = rendered
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    (lines.len() == mode_count + 1).then_some(lines)
}

fn rewrite_spell_resolution_damage_source(def: &CardDefinition, rendered: &str) -> String {
    if !(def.card.is_instant() || def.card.is_sorcery()) || def.card.name.contains(" // ") {
        return rendered.to_string();
    }
    let mut rewritten = String::with_capacity(rendered.len());
    let mut segment_start = 0usize;
    let mut quoted = false;
    for (index, character) in rendered.char_indices() {
        if character != '"' {
            continue;
        }
        let segment = &rendered[segment_start..index];
        if quoted {
            rewritten.push_str(segment);
        } else {
            rewritten.push_str(&rewrite_unquoted_spell_resolution_damage_source(
                def, segment,
            ));
        }
        rewritten.push('"');
        quoted = !quoted;
        segment_start = index + character.len_utf8();
    }
    let segment = &rendered[segment_start..];
    if quoted {
        rewritten.push_str(segment);
    } else {
        rewritten.push_str(&rewrite_unquoted_spell_resolution_damage_source(
            def, segment,
        ));
    }
    rewritten
}

fn rewrite_unquoted_spell_resolution_damage_source(def: &CardDefinition, rendered: &str) -> String {
    let rendered = if let Some(rest) = rendered.strip_prefix("This creature deals ") {
        format!("{} deals {rest}", def.card.name)
    } else {
        rendered.to_string()
    };
    let rendered = rewrite_damage_phrases_for_permanent_abilities(&rendered, &def.card.name, false)
        .replace("This creature deals ", "It deals ")
        .replace("this creature deals ", "it deals ")
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
    let canonical_name_lower = card.name.to_ascii_lowercase();
    let line = if canonical_name_lower != card.name {
        replace_outside_quotes(&line, &canonical_name_lower, &card.name)
    } else {
        line
    };
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
        || lower.contains("you may have this creature gain ")
        || lower.contains("if this creature is untapped")
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
    let quoted_cant_attack = format!("{source_name} gain \"this creature can't attack\"");
    let substituted = if substituted.contains(&quoted_cant_attack) {
        substituted.replace(
            &quoted_cant_attack,
            &format!("{source_name} gain \"{source_name} can't attack\""),
        )
    } else {
        substituted
    };
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

        if let Some(pay_life) = pay.downcast_ref::<crate::effects::PayLifeEffect>() {
            if !matches!(pay_life.player, ChooseSpec::Player(PlayerFilter::You)) {
                return None;
            }
            return Some(format!("—Pay {} life", describe_value(&pay_life.amount)));
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

fn describe_structural_play_from_with_mana_permission_pair(
    first: &Ability,
    second: &Ability,
) -> Option<String> {
    fn render(grant_ability: &Ability, mana_ability: &Ability) -> Option<String> {
        let AbilityKind::Static(grant_static) = &grant_ability.kind else {
            return None;
        };
        let grant = grant_static.grant_spec()?;
        if !matches!(grant.grantable, crate::grant::Grantable::PlayFrom)
            || grant.beneficiary != PlayerFilter::You
        {
            return None;
        }

        let AbilityKind::Static(mana_static) = &mana_ability.kind else {
            return None;
        };
        let ironsmith_core::StaticAbilityPayload::ManaSpendPermission { permission, .. } =
            &mana_static.compiled_model()?.payload
        else {
            return None;
        };
        let ironsmith_core::ManaSpendScope::CastingSpellsMatching(mana_filter) = &permission.scope
        else {
            return None;
        };
        if permission.player != PlayerFilter::You
            || *mana_filter != grant.filter
            || permission.any_color_mana_symbol.is_some()
            || permission.other_mana_only_as_colorless
        {
            return None;
        }

        let source_phrase = match &permission.mana_source_filter {
            None => "mana".to_string(),
            Some(source_filter) => {
                let mut normalized = source_filter.clone();
                if normalized.supertypes.as_slice() != [Supertype::Snow] {
                    return None;
                }
                normalized.supertypes.clear();
                if normalized != ObjectFilter::default() {
                    return None;
                }
                "mana from snow sources".to_string()
            }
        };
        let mana_clause = match permission.mode {
            ironsmith_core::value_model::ManaSpendMode::AnyColor => format!(
                "you may spend {source_phrase} as though it were mana of any color to cast those spells"
            ),
            ironsmith_core::value_model::ManaSpendMode::AnyType
                if permission.mana_source_filter.is_none() =>
            {
                "mana of any type can be spent to cast those spells".to_string()
            }
            _ => return None,
        };
        Some(format!("{}, and {mana_clause}", grant.display()))
    }

    render(first, second).or_else(|| render(second, first))
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

    // Compiled counter-entry abilities retain the typed value that determines
    // their counter count. Only fixed, X, and sunburst counts belong to the
    // structural keyword bundles below. In particular, an authored
    // "a counter ... for each ..." line must not be mistaken for a fixed one
    // merely because its rendered surface starts with the article "a".
    if let Some(model) = static_ability.compiled_model()
        && let ironsmith_core::StaticAbilityPayload::EntersWithCountersValue { counter, count } =
            &model.payload
    {
        let amount = match count.unhinted() {
            Value::Fixed(amount) => CounterKeywordAmount::Fixed(u32::try_from(*amount).ok()?),
            Value::X => CounterKeywordAmount::X,
            Value::ColorsOfManaSpentToCastThisSpell => CounterKeywordAmount::Sunburst,
            _ => return None,
        };
        return Some((*counter, amount));
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
    if display.contains(" for each ")
        || display.contains(" equal to ")
        || display.contains("where x is ")
    {
        return None;
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

fn describe_repeatable_sacrifice_copy_followup(
    cost: &crate::cost::OptionalCost,
    ability: &Ability,
) -> Option<String> {
    if cost.kind != crate::cost::OptionalCostKind::Additional || !cost.repeatable {
        return None;
    }
    let [cost_component] = cost.cost.as_all()? else {
        return None;
    };
    let sacrifice = unwrap_basic_render_wrapper(cost_component.effect_ref()?)
        .downcast_ref::<crate::effects::SacrificeEffect>()?;
    if sacrifice.player != PlayerFilter::You || sacrifice.count != Value::Fixed(1) {
        return None;
    }

    let AbilityKind::Triggered(triggered) = &ability.kind else {
        return None;
    };
    if ability.functional_zones != [Zone::Stack]
        || !triggered.choices.is_empty()
        || triggered
            .trigger
            .downcast_ref::<crate::triggers::YouCastThisSpellTrigger>()
            .is_none()
        || !matches!(
            triggered.intervening_if.as_ref(),
            Some(Condition::ThisSpellPaidLabel(label)) if label == &cost.reference
        )
    {
        return None;
    }
    let [copy_effect] = triggered.effects.flattened_default_effects() else {
        return None;
    };
    let copy = unwrap_basic_render_wrapper(copy_effect)
        .downcast_ref::<crate::effects::CopySpellEffect>()?;
    if !matches!(copy.target.unhinted(), ChooseSpec::Source)
        || copy.copier != PlayerFilter::You
        || !matches!(
            &copy.count,
            Value::TimesPaidLabel(label) if label == &cost.reference
        )
    {
        return None;
    }

    let mut noun_filter = sacrifice.filter.clone();
    noun_filter.zone = None;
    noun_filter.controller = None;
    noun_filter.owner = None;
    let noun_description = noun_filter.description();
    let noun = strip_indefinite_article(&noun_description);
    Some(format!(
        "When you do, copy this spell for each {noun} sacrificed this way"
    ))
}

fn is_repeatable_sacrifice_copy_helper(def: &CardDefinition, ability: &Ability) -> bool {
    def.optional_costs
        .iter()
        .any(|cost| describe_repeatable_sacrifice_copy_followup(cost, ability).is_some())
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
        AlternativeCastingMethod::Cleave { cost, .. } => {
            format!("Cleave {}", cost.to_oracle())
        }
        AlternativeCastingMethod::Awaken { amount, cost, .. } => {
            format!("Awaken {amount}—{}", cost.to_oracle())
        }
        AlternativeCastingMethod::Flashback { total_cost } => {
            let costs = method.non_mana_costs();
            let mana_cost = total_cost.mana_cost().map(|cost| cost.to_oracle());
            if costs.is_empty() {
                format!(
                    "Flashback—{}",
                    mana_cost.unwrap_or_else(|| "{0}".to_string())
                )
            } else {
                let extra = capitalize_first(&describe_alternative_costs(&costs));
                if let Some(mana_cost) = mana_cost {
                    format!("Flashback—{mana_cost}, {extra}")
                } else {
                    format!("Flashback—{extra}")
                }
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
        AlternativeCastingMethod::Blitz { total_cost } => {
            let costs = method.non_mana_costs();
            let mana_cost = total_cost
                .mana_cost()
                .map(|cost| cost.to_oracle())
                .unwrap_or_else(|| "{0}".to_string());
            if costs.is_empty() {
                format!("Blitz {mana_cost}")
            } else {
                let extra = capitalize_first(&describe_alternative_costs(&costs));
                format!("Blitz—{mana_cost}, {extra}")
            }
        }
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
        "{}. {}.",
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

fn aura_attaches_to_creature_or_planeswalker(
    attachment: Option<&crate::object::AuraAttachmentFilter>,
) -> bool {
    let Some(crate::object::AuraAttachmentFilter::Object(filter)) = attachment else {
        return false;
    };
    let mut normalized = filter.clone();
    normalized.zone = Some(Zone::Battlefield);
    let mut expected = ObjectFilter::default().in_zone(Zone::Battlefield);
    expected.card_types = vec![CardType::Creature, CardType::Planeswalker];
    normalized == expected
}

fn aura_attaches_to_plain_permanent_type_union(
    attachment: Option<&crate::object::AuraAttachmentFilter>,
) -> bool {
    let Some(crate::object::AuraAttachmentFilter::Object(filter)) = attachment else {
        return false;
    };
    if filter.card_types.len() < 2
        || filter.card_types.iter().any(|card_type| {
            !matches!(
                card_type,
                CardType::Artifact
                    | CardType::Battle
                    | CardType::Creature
                    | CardType::Enchantment
                    | CardType::Land
                    | CardType::Planeswalker
            )
        })
    {
        return false;
    }
    let mut normalized = filter.clone();
    normalized.zone = Some(Zone::Battlefield);
    let mut expected = ObjectFilter::default().in_zone(Zone::Battlefield);
    expected.card_types = filter.card_types.clone();
    normalized == expected
}

fn rewrite_plain_creature_aura_subject(
    attachment: Option<&crate::object::AuraAttachmentFilter>,
    line: &str,
) -> String {
    if aura_attaches_to_plain_creature(attachment) {
        return line
            .replace("Enchanted permanent", "Enchanted creature")
            .replace("enchanted permanent", "enchanted creature");
    }
    if aura_attaches_to_creature_or_planeswalker(attachment)
        || aura_attaches_to_plain_permanent_type_union(attachment)
    {
        return line
            .replace("Enchanted creature", "Enchanted permanent")
            .replace("enchanted creature", "enchanted permanent");
    }
    line.to_string()
}

/// The `Enchant ...` line and the spell-resolution `AttachToEffect` are two
/// typed views of the same Aura attachment operation. Keep the executable
/// effect, but remove that one encoded operation from the program used for
/// compiled-text rendering so it cannot surface as a second
/// "Attach this source to target ..." sentence when the Aura also has other
/// cast-time setup such as gift.
fn spell_program_without_encoded_aura_attachment(
    program: &crate::resolution::ResolutionProgram,
    attachment: &crate::object::AuraAttachmentFilter,
) -> crate::resolution::ResolutionProgram {
    let expected_target = attachment.target_spec();
    let mut removed = false;
    let mut segments = Vec::with_capacity(program.segments.len());
    for segment in &program.segments {
        let mut default_effects = Vec::with_capacity(segment.default_effects.len());
        for effect in &segment.default_effects {
            let is_encoded_attachment = !removed
                && effect
                    .downcast_ref::<crate::effects::AttachToEffect>()
                    .is_some_and(|attach| attach.target == expected_target);
            if is_encoded_attachment {
                removed = true;
            } else {
                default_effects.push(effect.clone());
            }
        }
        if !default_effects.is_empty() || !segment.self_replacements.is_empty() {
            segments.push(ironsmith_core::ResolutionSegment {
                default_effects,
                self_replacements: segment.self_replacements.clone(),
                starts_new_source_line: segment.starts_new_source_line,
            });
        }
    }
    crate::resolution::ResolutionProgram::new(segments)
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

/// Render the typed counter-removal prevention family as its authored inline
/// replacement sentence. The generic conditional-static renderer otherwise
/// presents this model as an `As long as` ability grant, even though the
/// condition is part of the damage replacement event itself.
fn structural_source_counter_threshold(condition: &Condition) -> Option<CounterType> {
    match condition {
        Condition::SourceHasCounterAtLeast {
            counter_type,
            count: 1,
            ..
        } => Some(*counter_type),
        Condition::CountComparison {
            count: ironsmith_core::AnthemCountExpression::MatchingFilter(filter),
            comparison: Comparison::GreaterThanOrEqual(1),
            ..
        } => {
            let Some(ironsmith_core::CounterConstraint::Typed(counter_type)) = filter.with_counter
            else {
                return None;
            };
            (filter == &ObjectFilter::source().with_counter_type(counter_type))
                .then_some(counter_type)
        }
        _ => None,
    }
}

fn describe_structural_counter_removal_damage_prevention(
    ability: &Ability,
    subject: &str,
) -> Option<String> {
    let AbilityKind::Static(static_ability) = &ability.kind else {
        return None;
    };
    let model = static_ability.compiled_model()?;
    let ironsmith_core::StaticAbilityPayload::Conditional {
        ability: prevention,
        condition,
    } = &model.payload
    else {
        return None;
    };
    let condition_counter = structural_source_counter_threshold(condition)?;
    let ironsmith_core::StaticAbilityPayload::PreventDamageToSelfRemoveCounter {
        counter_type,
        amount,
        follow_up,
    } = &prevention.payload
    else {
        return None;
    };
    if condition_counter != *counter_type {
        return None;
    }

    let (amount_text, plural) = match amount.unhinted() {
        Value::Fixed(1) => ("a".to_string(), false),
        Value::Fixed(amount) if *amount > 1 => (
            u32::try_from(*amount)
                .ok()
                .and_then(small_number_word)
                .unwrap_or_else(|| amount.to_string()),
            true,
        ),
        Value::EventValue(EventValueSpec::Amount) => ("that many".to_string(), true),
        _ => return None,
    };
    let counter = counter_type.description();
    let counter_noun = if plural { "counters" } else { "counter" };
    let head = format!(
        "If damage would be dealt to {subject} while it has a {counter} counter on it, prevent that damage"
    );
    let removal = format!("remove {amount_text} {counter} {counter_noun} from it");

    match follow_up {
        None => Some(format!("{head} and {removal}")),
        Some(ironsmith_core::CounterRemovalFollowUp::EachPlayerGetsCounters {
            counter_type: gained_counter,
            counters_per_removed,
        }) => {
            let (gained_amount, gained_plural) = if *counters_per_removed == 1 {
                ("a".to_string(), "")
            } else {
                (
                    small_number_word(*counters_per_removed)
                        .unwrap_or_else(|| counters_per_removed.to_string()),
                    "s",
                )
            };
            Some(format!(
                "{head}, {removal}, then give each player {gained_amount} {} counter{gained_plural} for each {counter} counter removed this way",
                gained_counter.description()
            ))
        }
    }
}

/// Preserve the authored pronoun and event-derived amount for the reusable
/// "counters removed this way" damage trigger. Generic permanent damage
/// normalization changes an effect-leading `it deals` into the source type,
/// which loses the surface relationship carried by this exact typed shape.
fn describe_structural_counter_removed_this_way_damage_trigger(
    ability: &Ability,
) -> Option<String> {
    let AbilityKind::Triggered(triggered) = &ability.kind else {
        return None;
    };
    if triggered.intervening_if.is_some()
        || triggered.choices.as_slice() != [ChooseSpec::AnyTarget].as_slice()
        || triggered.presentation_label.is_some()
    {
        return None;
    }
    let counter_removed = triggered
        .trigger
        .downcast_ref::<crate::triggers::CounterRemovedFromTrigger>()?;
    let mut trigger_filter = counter_removed.filter.clone();
    trigger_filter.source_surface = None;
    if trigger_filter != ObjectFilter::source()
        || !counter_removed.one_or_more
        || !counter_removed.caused_by_source
    {
        return None;
    }
    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() {
        return None;
    }
    let [tag_effect, execute_effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let triggering = tag_effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
    let execute = execute_effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()?;
    if !matches!(
        execute.source.unhinted(),
        ChooseSpec::Tagged(tag) if *tag == triggering.tag
    ) {
        return None;
    }
    let damage = execute
        .effect
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    if damage.source_is_combat
        || damage.unpreventable
        || !matches!(damage.target.base(), ChooseSpec::AnyTarget)
        || !matches!(
            damage.amount.unhinted(),
            Value::EventValue(EventValueSpec::Amount)
        )
        || !damage
            .amount
            .has_surface_hint(ironsmith_core::ValueSurfaceHint::CountersRemovedThisWay)
    {
        return None;
    }

    Some(format!(
        "{}, it deals that much damage to any target",
        triggered.trigger.display()
    ))
}

fn conditioned_combat_keyword_application(
    effect: &Effect,
) -> Option<(
    &crate::effects::ApplyContinuousEffect,
    &crate::ConditionExpr,
)> {
    if let Some(apply) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>() {
        return Some((apply, apply.condition.as_ref()?));
    }
    let conditional = effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !matches!(
        conditional.surface,
        ironsmith_core::ConditionalSurface::LeadingIf
            | ironsmith_core::ConditionalSurface::TrailingIf
    ) || !conditional.if_false.is_empty()
    {
        return None;
    }
    let [inner] = conditional.if_true.as_slice() else {
        return None;
    };
    let apply = inner.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    apply
        .condition
        .is_none()
        .then_some((apply, &conditional.condition))
}

fn modeled_each_combat_matching_keyword_grant(
    ability: &Ability,
) -> Option<(&ObjectFilter, String)> {
    if ability.functional_zones.as_slice() != [Zone::Battlefield] {
        return None;
    }
    let AbilityKind::Triggered(triggered) = &ability.kind else {
        return None;
    };
    if !triggered.choices.is_empty()
        || triggered.intervening_if.is_some()
        || triggered.presentation_label.is_some()
    {
        return None;
    }
    let combat = triggered
        .trigger
        .downcast_ref::<crate::triggers::BeginningOfCombatTrigger>()?;
    if combat.player != PlayerFilter::Any {
        return None;
    }
    let [segment] = triggered.effects.segments.as_slice() else {
        return None;
    };
    if !segment.self_replacements.is_empty() || segment.starts_new_source_line {
        return None;
    }
    let [effect] = segment.default_effects.as_slice() else {
        return None;
    };
    let (apply, condition) = conditioned_combat_keyword_application(effect)?;
    if apply.target_spec.is_some()
        || !apply.additional_modifications.is_empty()
        || !apply.runtime_modifications.is_empty()
        || apply.until != Until::EndOfTurn
        || apply.source_type.is_some()
        || apply.source_reference_surface.is_some()
        || apply.set_quantifier_surface.is_some()
        || apply.type_retention_surface.is_some()
        || apply.animation_pt_surface.is_some()
        || apply.animation_duration_surface.is_some()
        || !apply.lock_filter_at_resolution
        || apply.resolve_set_pt_values_at_resolution
        || apply.require_creature_target
    {
        return None;
    }
    let crate::continuous::EffectTarget::Filter(target_filter) = &apply.target else {
        return None;
    };
    let crate::continuous::Modification::AddAbility(granted) = apply.modification.as_ref()? else {
        return None;
    };
    if !granted.is_keyword() {
        return None;
    }
    let keyword_id = granted.id();
    let keyword = granted
        .display()
        .trim()
        .trim_end_matches('.')
        .to_ascii_lowercase();
    let Condition::PlayerControls {
        player: PlayerFilter::You,
        filter: condition_filter,
    } = condition
    else {
        return None;
    };
    if condition_filter.static_abilities.as_slice() != [keyword_id]
        || !condition_filter.excluded_static_abilities.is_empty()
        || !condition_filter.ability_markers.is_empty()
        || !condition_filter.excluded_ability_markers.is_empty()
    {
        return None;
    }
    let mut condition_basis = condition_filter.clone();
    condition_basis.static_abilities.clear();
    if &condition_basis != target_filter {
        return None;
    }
    Some((target_filter, keyword))
}

/// Rejoin a run of independent "at the beginning of each combat" keyword
/// grants when every typed condition asks whether the exact same affected set
/// already has that branch's keyword. This preserves all executable grants,
/// including branches lowered through an explicit conditional wrapper.
fn describe_structural_each_combat_keyword_grant_ladder(
    abilities: &[Ability],
) -> Option<(String, usize)> {
    let mut target_filter = None;
    let mut keywords = Vec::new();
    let mut consumed = 0usize;
    for ability in abilities {
        let Some((target, keyword)) = modeled_each_combat_matching_keyword_grant(ability) else {
            break;
        };
        if let Some(shared) = target_filter {
            if shared != target {
                break;
            }
        } else {
            target_filter = Some(target);
        }
        if keywords.contains(&keyword) {
            break;
        }
        keywords.push(keyword);
        consumed += 1;
    }
    if consumed < 3 {
        return None;
    }
    let target_filter = target_filter?;
    let (subject, singular) =
        crate::static_abilities::grant_subject_with_set_quantifier(target_filter, None);
    if singular {
        return None;
    }
    let condition_noun = target_filter.description();
    let condition_noun = with_indefinite_article(strip_leading_article(&condition_noun));
    Some((
        format!(
            "At the beginning of each combat, {subject} gain {} until end of turn if {condition_noun} has {}. The same is true for {}",
            keywords[0],
            keywords[0],
            render_keyword_list(&keywords[1..], false),
        ),
        consumed,
    ))
}

#[cfg(test)]
mod each_combat_keyword_grant_ladder_tests {
    use super::*;

    #[test]
    fn all_matching_keyword_branches_rejoin_across_direct_and_wrapped_conditions() {
        let oracle = "At the beginning of each combat, creatures you control gain first strike until end of turn if a creature you control has first strike. The same is true for flying, deathtouch, double strike, haste, hexproof, indestructible, lifelink, menace, reach, skulk, trample, and vigilance.";
        let definition = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Combat Keyword Ladder Probe",
        )
        .card_types(vec![CardType::Creature])
        .parse_text(oracle)
        .expect("matching combat keyword ladder should compile");

        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition),
            vec![oracle.to_string()]
        );
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
    let spell_effects_for_render = def.spell_effect.as_ref().map(|effects| {
        def.aura_attach_filter
            .as_ref()
            .map(|attachment| spell_program_without_encoded_aura_attachment(effects, attachment))
            .unwrap_or_else(|| effects.clone())
    });
    for (idx, method) in def.alternative_casts.iter().enumerate() {
        let line = describe_alternative_cast_with_qualified_reduction(def, method, idx)
            .unwrap_or_else(|| describe_alternative_cast_line(method, idx));
        let is_prototype = method.name().eq_ignore_ascii_case("Prototype")
            && method.prototype_power_toughness().is_some();
        if is_prototype
            || method.is_mutate()
            || matches!(method, AlternativeCastingMethod::Cleave { .. })
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
        let mut line = describe_optional_cost_line(cost);
        if let Some(followup) = def
            .abilities
            .iter()
            .find_map(|ability| describe_repeatable_sacrifice_copy_followup(cost, ability))
        {
            line = format!("{}. {followup}", line.trim_end_matches('.'));
        }
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
        let attachment_target = describe_enchant_filter(filter);
        if def.card.subtypes.contains(&Subtype::Equipment) {
            // The shared attachment-legality field models both Aura enchant
            // restrictions and Equipment's "can be attached only to"
            // restriction. Card type/subtype distinguishes their authored
            // rules surface without changing the executable legality filter.
            out.push(format!(
                "This Equipment can be attached only to {attachment_target}"
            ));
        } else {
            out.push(format!("Enchant {attachment_target}"));
        }
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
        let source_has_delve = def.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id() == crate::static_abilities::StaticAbilityId::Delve
            )
        });
        let mut ability_idx = 0usize;
        while ability_idx < def.abilities.len() {
            let ability = &def.abilities[ability_idx];
            if let Some(keyword_count) = source_line_keyword_group_count(ability) {
                output.push(source_line_keyword_group_sentinel(keyword_count));
                ability_idx += 1;
                continue;
            }
            if let Some(member_count) = source_line_static_group_count(ability) {
                if let Some(text) = describe_source_line_static_group(
                    &def.abilities[ability_idx + 1..],
                    member_count,
                ) {
                    let text =
                        if let Some(label) = source_line_static_group_presentation_label(ability) {
                            format!("{label} — {text}")
                        } else {
                            text
                        };
                    output.push(format!("Static ability {}: {text}", ability_idx + 2));
                    ability_idx += member_count + 1;
                } else {
                    // Unsupported structural families retain the preexisting
                    // per-ability surfaces; only the inert marker is skipped.
                    ability_idx += 1;
                }
                continue;
            }
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
            if is_repeatable_sacrifice_copy_helper(def, ability) {
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
                describe_structural_each_combat_keyword_grant_ladder(&def.abilities[ability_idx..])
            {
                output.push(format!("Triggered ability {}: {text}", ability_idx + 1));
                ability_idx += consumed;
                continue;
            }
            if let Some(text) = describe_structural_counter_removed_this_way_damage_trigger(ability)
            {
                output.push(format!("Triggered ability {}: {text}", ability_idx + 1));
                ability_idx += 1;
                continue;
            }
            if let Some(text) =
                describe_structural_counter_removal_damage_prevention(ability, subject)
            {
                output.push(format!("Static ability {}: {text}", ability_idx + 1));
                ability_idx += 1;
                continue;
            }
            if let Some((text, consumed)) = describe_structural_threshold_source_modifier_bundle(
                &def.abilities[ability_idx..],
                subject,
            ) {
                output.push(format!("Static ability {}: {text}", ability_idx + 1));
                ability_idx += consumed;
                continue;
            }
            if let Some((text, consumed)) =
                describe_structural_conditioned_source_anthem_keyword_bundle(
                    &def.abilities[ability_idx..],
                    subject,
                )
            {
                output.push(format!("Static ability {}: {text}", ability_idx + 1));
                ability_idx += consumed;
                continue;
            }
            if let Some((text, consumed)) =
                describe_structural_conditioned_source_animation_annihilator_bundle(
                    &def.abilities[ability_idx..],
                )
            {
                output.push(format!("Static ability {}: {text}", ability_idx + 1));
                ability_idx += consumed;
                continue;
            }
            if let Some((text, consumed)) =
                describe_structural_quoted_static_grant_bundle(&def.abilities[ability_idx..])
            {
                output.push(format!("Static ability {}: {text}", ability_idx + 1));
                ability_idx += consumed;
                continue;
            }
            if let Some((text, consumed)) =
                describe_structural_keyword_maximum_blocker_bundle(&def.abilities[ability_idx..])
            {
                output.push(format!("Static ability {}: {text}", ability_idx + 1));
                ability_idx += consumed;
                continue;
            }
            if let Some((text, consumed)) = describe_structural_shared_subject_block_attack_bundle(
                &def.abilities[ability_idx..],
            ) {
                output.push(format!("Static ability {}: {text}", ability_idx + 1));
                ability_idx += consumed;
                continue;
            }
            if let Some((text, consumed)) =
                describe_structural_remove_all_abilities_untap_bundle(&def.abilities[ability_idx..])
            {
                output.push(format!("Static ability {}: {text}", ability_idx + 1));
                ability_idx += consumed;
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
                describe_structural_dynamic_animation_bundle(&def.abilities[ability_idx..])
            {
                output.push(format!("Static ability {}: {text}", ability_idx + 1));
                ability_idx += consumed;
                continue;
            }
            if let Some((text, consumed)) =
                describe_structural_source_fixed_animation_bundle(&def.abilities[ability_idx..])
            {
                output.push(format!("Static ability {}: {text}", ability_idx + 1));
                ability_idx += consumed;
                continue;
            }
            if let Some((lines, consumed)) =
                describe_structural_anthem_then_color_subtype_bundle(&def.abilities[ability_idx..])
            {
                for (offset, text) in lines.into_iter().enumerate() {
                    output.push(format!(
                        "Static ability {}: {text}",
                        ability_idx + offset + 1
                    ));
                }
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
                describe_structural_all_subtypes_scope_ladder(&def.abilities[ability_idx..])
            {
                output.push(format!("Static ability {}: {text}", ability_idx + 1));
                ability_idx += consumed;
                continue;
            }
            if let Some((text, consumed)) = describe_structural_delve_keyword_variant_ladder(
                &def.abilities[ability_idx..],
                subject,
                source_has_delve,
            ) {
                output.push(format!("Static ability {}: {text}", ability_idx + 1));
                ability_idx += consumed;
                continue;
            }
            if let Some((text, consumed)) = describe_structural_keyword_same_is_true_ladder(
                &def.abilities[ability_idx..],
                subject,
            ) {
                output.push(format!("Static ability {}: {text}", ability_idx + 1));
                ability_idx += consumed;
                continue;
            }
            if let Some((text, consumed)) = describe_structural_five_basic_land_modifier_ladder(
                &def.abilities[ability_idx..],
                subject,
            ) {
                output.push(format!("Static ability {}: {text}", ability_idx + 1));
                ability_idx += consumed;
                continue;
            }
            if let Some(text) =
                describe_structural_conditional_source_keyword_grant(ability, subject)
            {
                output.push(format!("Static ability {}: {text}", ability_idx + 1));
                ability_idx += 1;
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
            if let Some((text, consumed)) =
                describe_structural_anthem_set_colors_bundle(&def.abilities[ability_idx..])
            {
                output.push(format!("Static ability {}: {text}", ability_idx + 1));
                ability_idx += consumed;
                continue;
            }
            if let Some(text) = describe_structural_can_block_additional_grant(ability) {
                output.push(format!("Static ability {}: {text}", ability_idx + 1));
                ability_idx += 1;
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
                && let Some(permission) = describe_structural_play_from_with_mana_permission_pair(
                    ability,
                    &def.abilities[ability_idx + 1],
                )
            {
                output.push(format!("Static ability {}: {permission}", ability_idx + 1));
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
    if let Some(spell_effects) = &spell_effects_for_render
        && !spell_effects.is_empty()
    {
        if is_choose_background_spell_effect(spell_effects) {
            out.push("Keyword ability 0: Choose a Background".to_string());
        } else {
            let rendered = describe_resolution_program_for_card(def, spell_effects);
            let rendered = preserve_cleave_bracket_surface(def, &rendered).unwrap_or(rendered);
            let spell_text = rewrite_additional_sacrifice_reference_surface(def, &rendered);
            if !spell_text.trim().is_empty() {
                if let Some(mut source_lines) = typed_spree_source_lines(spell_effects, &spell_text)
                {
                    let header = source_lines.remove(0);
                    out.push(format!("Spell effects: {header}"));
                    out.extend(source_lines);
                } else {
                    out.push(format!("Spell effects: {}", spell_text));
                }
            }
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

fn structural_static_effect_filter(
    ability: &Ability,
    expected_id: crate::static_abilities::StaticAbilityId,
) -> Option<&ObjectFilter> {
    let AbilityKind::Static(static_ability) = &ability.kind else {
        return None;
    };
    if static_ability.id() != expected_id {
        return None;
    }
    if let Some(filter) = static_ability.structural_effect_filter() {
        return Some(filter);
    }
    match &static_ability.compiled_model()?.payload {
        ironsmith_core::StaticAbilityPayload::Anthem(anthem)
            if expected_id == crate::static_abilities::StaticAbilityId::Anthem =>
        {
            anthem.filter.as_ref()
        }
        ironsmith_core::StaticAbilityPayload::RemoveAllAbilities(filter)
            if expected_id
                == crate::static_abilities::StaticAbilityId::RemoveAllAbilitiesForFilter =>
        {
            Some(filter)
        }
        ironsmith_core::StaticAbilityPayload::SetColors { filter, .. }
            if expected_id == crate::static_abilities::StaticAbilityId::SetColors =>
        {
            Some(filter)
        }
        ironsmith_core::StaticAbilityPayload::AddSubtypes { filter, .. }
            if expected_id == crate::static_abilities::StaticAbilityId::AddSubtypes =>
        {
            Some(filter)
        }
        _ => None,
    }
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

fn split_static_ability_loss_predicate(display: &str) -> Option<(&str, &str, &'static str)> {
    if let Some((subject, tail)) = split_static_predicate(display, " loses ") {
        return Some((subject, tail, "loses"));
    }
    split_static_predicate(display, " lose ").map(|(subject, tail)| (subject, tail, "lose"))
}

/// Recombine a typed list of keyword-loss effects emitted from one authored
/// static source-line chunk. The source marker proves the line boundary; the
/// payload checks prove every member has the same affected set and loss mode.
fn describe_source_line_static_ability_loss_group(abilities: &[Ability]) -> Option<String> {
    let first = abilities.first()?;
    let AbilityKind::Static(first_static) = &first.kind else {
        return None;
    };
    let first_model = first_static.compiled_model()?;
    let ironsmith_core::StaticAbilityPayload::RemoveAbilityForFilter {
        filter: first_filter,
        mode: first_mode,
        ..
    } = &first_model.payload
    else {
        return None;
    };
    if *first_mode != ironsmith_core::AbilityLossMode::Lose {
        return None;
    }
    let first_display = first_static.display();
    let first_display = first_display.trim().trim_end_matches('.');
    let (subject, first_tail, verb) = split_static_ability_loss_predicate(first_display)?;
    let mut tails = vec![first_tail.to_string()];

    for ability in &abilities[1..] {
        if ability.functional_zones != first.functional_zones {
            return None;
        }
        let AbilityKind::Static(static_ability) = &ability.kind else {
            return None;
        };
        let model = static_ability.compiled_model()?;
        let ironsmith_core::StaticAbilityPayload::RemoveAbilityForFilter { filter, mode, .. } =
            &model.payload
        else {
            return None;
        };
        if filter != first_filter || mode != first_mode {
            return None;
        }
        let display = static_ability.display();
        let display = display.trim().trim_end_matches('.');
        let (next_subject, tail, _) = split_static_ability_loss_predicate(display)?;
        if !static_bundle_subjects_match(subject, next_subject) {
            return None;
        }
        tails.push(tail.to_string());
    }

    Some(format!(
        "{} {verb} {}",
        capitalize_first(subject),
        render_static_bundle_list(&tails)
    ))
}

/// Recombine the paired player restrictions represented by Hand to Hand's
/// static model family. Matching typed player filters and conditions establish
/// the shared subject; the two prohibited actions use Oracle's `or` surface.
fn describe_source_line_cast_activation_restriction_group(abilities: &[Ability]) -> Option<String> {
    let [first, second] = abilities else {
        return None;
    };
    if first.functional_zones != second.functional_zones {
        return None;
    }
    let AbilityKind::Static(first_static) = &first.kind else {
        return None;
    };
    let AbilityKind::Static(second_static) = &second.kind else {
        return None;
    };
    let (first_restriction, first_display, first_condition) =
        first_static.rule_restriction_parts()?;
    let (second_restriction, second_display, second_condition) =
        second_static.rule_restriction_parts()?;
    if first_condition != second_condition || first_condition.is_none() {
        return None;
    }

    let matching_players = match (first_restriction, second_restriction) {
        (
            crate::effect::Restriction::CastSpellsMatching(cast_player, _),
            crate::effect::Restriction::ActivateNonManaAbilities(activation_player),
        )
        | (
            crate::effect::Restriction::ActivateNonManaAbilities(activation_player),
            crate::effect::Restriction::CastSpellsMatching(cast_player, _),
        ) => cast_player == activation_player,
        _ => false,
    };
    if !matching_players {
        return None;
    }

    let first_display = first_display.trim().trim_end_matches('.');
    let second_display = second_display.trim().trim_end_matches('.');
    let (subject, first_action) = split_static_predicate(first_display, " can't ")?;
    let (second_subject, second_action) = split_static_predicate(second_display, " can't ")?;
    if !static_bundle_subjects_match(subject, second_subject) {
        return None;
    }

    let first_full = first_static.display();
    let second_full = second_static.display();
    let first_full = first_full.trim().trim_end_matches('.');
    let second_full = second_full.trim().trim_end_matches('.');
    let prefix = first_full.strip_suffix(first_display)?;
    if second_full.strip_suffix(second_display)? != prefix {
        return None;
    }
    let rendered_subject = if prefix.is_empty() {
        capitalize_first(subject)
    } else {
        subject.to_string()
    };

    Some(format!(
        "{prefix}{rendered_subject} can't {first_action} or {second_action}"
    ))
}

/// Recombine a base-P/T setter, one granted static keyword, and ability loss
/// when all three typed models affect the exact same object and came from one
/// authored source-line chunk.
fn describe_source_line_base_pt_grant_loss_group(abilities: &[Ability]) -> Option<String> {
    let [first, second, third] = abilities else {
        return None;
    };
    if first.functional_zones != second.functional_zones
        || first.functional_zones != third.functional_zones
    {
        return None;
    }

    let mut base = None;
    let mut removal = None;
    let mut grant = None;
    for ability in abilities {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            return None;
        };
        let model = static_ability.compiled_model()?;
        match &model.payload {
            ironsmith_core::StaticAbilityPayload::SetBasePowerToughness { filter, .. }
                if base.is_none() =>
            {
                base = Some((static_ability, filter));
            }
            ironsmith_core::StaticAbilityPayload::RemoveAllAbilities(filter)
                if removal.is_none() =>
            {
                removal = Some((static_ability, filter));
            }
            ironsmith_core::StaticAbilityPayload::GrantObjectAbilityForFilter(spec)
                if grant.is_none()
                    && spec.condition.is_none()
                    && spec.additional_abilities.is_empty()
                    && spec.set_quantifier_surface.is_none()
                    && matches!(&spec.ability.kind, ironsmith_core::AbilityKind::Static(_)) =>
            {
                grant = Some((static_ability, &spec.filter));
            }
            _ => return None,
        }
    }
    let (base, base_filter) = base?;
    let (removal, removal_filter) = removal?;
    let (grant, grant_filter) = grant?;
    if base_filter != removal_filter || base_filter != grant_filter {
        return None;
    }

    let base_display = base.display();
    let base_display = base_display.trim().trim_end_matches('.');
    let (subject, base_tail, base_verb) =
        split_static_predicate_with_verb(base_display, &[" has ", " have "])?;
    let grant_display = grant.display();
    let grant_display = grant_display.trim().trim_end_matches('.');
    let (grant_subject, grant_tail, grant_verb) =
        split_static_predicate_with_verb(grant_display, &[" has ", " have "])?;
    let removal_display = removal.display();
    let removal_display = removal_display.trim().trim_end_matches('.');
    let (removal_subject, removal_tail, removal_verb) =
        split_static_ability_loss_predicate(removal_display)?;
    if removal_tail != "all abilities"
        || !static_bundle_subjects_match(subject, grant_subject)
        || !static_bundle_subjects_match(subject, removal_subject)
    {
        return None;
    }

    Some(format!(
        "{} {base_verb} {base_tail}, {grant_verb} {grant_tail}, and {removal_verb} all other abilities",
        capitalize_first(subject)
    ))
}

/// Recombine a granted ability and an all-abilities removal emitted from one
/// authored static source line. Matching typed filters prove the two layer-6
/// pieces share a subject; the source marker proves that `other` refers to the
/// granted ability from this same clause.
fn describe_source_line_grant_all_other_loss_group(abilities: &[Ability]) -> Option<String> {
    let [first, second] = abilities else {
        return None;
    };
    if first.functional_zones != second.functional_zones {
        return None;
    }

    let mut removal = None;
    let mut grant = None;
    for ability in [first, second] {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            return None;
        };
        let model = static_ability.compiled_model()?;
        match &model.payload {
            ironsmith_core::StaticAbilityPayload::RemoveAllAbilities(filter)
                if removal.is_none() =>
            {
                removal = Some((static_ability, filter));
            }
            ironsmith_core::StaticAbilityPayload::GrantObjectAbilityForFilter(spec)
                if grant.is_none()
                    && spec.condition.is_none()
                    && spec.additional_abilities.is_empty() =>
            {
                grant = Some((static_ability, &spec.filter));
            }
            _ => return None,
        }
    }
    let (removal, removal_filter) = removal?;
    let (grant, grant_filter) = grant?;
    if removal_filter != grant_filter {
        return None;
    }

    let grant_display = grant.display();
    let grant_display = grant_display.trim().trim_end_matches('.');
    let (subject, grant_tail, grant_verb) =
        split_static_predicate_with_verb(grant_display, &[" has ", " have "])?;
    let removal_display = removal.display();
    let removal_display = removal_display.trim().trim_end_matches('.');
    let (removal_subject, removal_tail, removal_verb) =
        split_static_ability_loss_predicate(removal_display)?;
    if removal_tail != "all abilities" || !static_bundle_subjects_match(subject, removal_subject) {
        return None;
    }
    let grant_tail = grant_tail
        .strip_prefix('"')
        .and_then(|body| body.strip_suffix('"'))
        .map(|body| format!("\"{}\"", body.trim_end_matches('.')))
        .unwrap_or_else(|| grant_tail.to_string());

    Some(format!(
        "{} {grant_verb} {grant_tail} and {removal_verb} all other abilities",
        capitalize_first(subject)
    ))
}

enum TypeAdditionGrantSurface {
    Quoted(String),
    Equip(String),
}

fn render_type_addition_grant_surfaces(items: &[TypeAdditionGrantSurface]) -> Option<String> {
    if items.len() < 2
        || !matches!(items.first(), Some(TypeAdditionGrantSurface::Quoted(_)))
        || !items
            .iter()
            .any(|item| matches!(item, TypeAdditionGrantSurface::Equip(_)))
    {
        return None;
    }

    let last = items.len() - 1;
    let rendered = items
        .iter()
        .enumerate()
        .map(|(idx, item)| match item {
            TypeAdditionGrantSurface::Quoted(text) => {
                let punctuation = if idx < last { "," } else { "." };
                format!("\"{}{punctuation}\"", text.trim_end_matches(['.', ',']))
            }
            TypeAdditionGrantSurface::Equip(text) if idx < last && items.len() > 2 => {
                format!("{},", lowercase_first(text.trim_end_matches('.')))
            }
            TypeAdditionGrantSurface::Equip(text) => lowercase_first(text.trim_end_matches('.')),
        })
        .collect::<Vec<_>>();
    let (final_item, leading) = rendered.split_last()?;
    Some(format!("{} and {final_item}", leading.join(" ")))
}

/// Recombine a type-addition clause and its mixed quoted/equip grants when
/// lowering emitted one continuous ability per executable layer component.
fn describe_source_line_type_addition_grant_group(abilities: &[Ability]) -> Option<String> {
    let [first, grants @ ..] = abilities else {
        return None;
    };
    if first.functional_zones.as_slice() != [Zone::Battlefield] || grants.is_empty() {
        return None;
    }
    let AbilityKind::Static(first_static) = &first.kind else {
        return None;
    };
    let first_model = first_static.compiled_model()?;
    let affected_filter = match &first_model.payload {
        ironsmith_core::StaticAbilityPayload::AddCardTypes { filter, card_types }
            if !card_types.is_empty() =>
        {
            filter
        }
        ironsmith_core::StaticAbilityPayload::AddSubtypes { filter, subtypes }
            if !subtypes.is_empty() =>
        {
            filter
        }
        _ => return None,
    };
    let type_display = first_static.display();
    let type_display = type_display.trim().trim_end_matches('.');
    let (_, addition, type_verb) =
        split_static_predicate_with_verb(type_display, &[" is ", " are "])?;
    if !addition.contains("in addition to")
        || !addition.ends_with("other types")
        || type_display.starts_with("During ")
    {
        return None;
    }

    let mut surfaces = Vec::with_capacity(grants.len());
    for ability in grants {
        if ability.functional_zones != first.functional_zones {
            return None;
        }
        let AbilityKind::Static(static_ability) = &ability.kind else {
            return None;
        };
        let model = static_ability.compiled_model()?;
        match &model.payload {
            ironsmith_core::StaticAbilityPayload::GrantAbility(grant)
                if &grant.filter == affected_filter
                    && grant.condition.is_none()
                    && grant.set_quantifier_surface.is_none()
                    && grant.ability.functional_zones.as_slice() == [Zone::Battlefield] =>
            {
                let ironsmith_core::AbilityKind::Static(granted_model) = &grant.ability.kind else {
                    return None;
                };
                let granted =
                    crate::static_abilities::StaticAbility::from_model(granted_model.clone());
                if granted.is_keyword() {
                    return None;
                }
                let text = describe_static_ability_with_subject(
                    &granted,
                    granted_ability_self_subject_for_filter(affected_filter),
                );
                let text = capitalize_first(text.trim().trim_end_matches('.'));
                if text.is_empty() {
                    return None;
                }
                surfaces.push(TypeAdditionGrantSurface::Quoted(text));
            }
            ironsmith_core::StaticAbilityPayload::GrantObjectAbilityForFilter(grant)
                if &grant.filter == affected_filter
                    && grant.condition.is_none()
                    && grant.additional_abilities.is_empty()
                    && grant.set_quantifier_surface.is_none() =>
            {
                let granted =
                    crate::static_abilities::StaticAbilityModelInterpreter::ability_from_model(
                        &grant.ability,
                    );
                let AbilityKind::Activated(activated) = &granted.kind else {
                    return None;
                };
                let equip = super::render_effects::describe_structural_equip_keyword(activated)?;
                surfaces.push(TypeAdditionGrantSurface::Equip(equip));
            }
            _ => return None,
        }
    }

    let verb = match type_verb {
        "are" => "have",
        "is" => "has",
        _ => return None,
    };
    Some(format!(
        "{type_display} and {verb} {}",
        render_type_addition_grant_surfaces(&surfaces)?
    ))
}

fn describe_source_line_static_group(abilities: &[Ability], member_count: usize) -> Option<String> {
    if member_count < 2 {
        return None;
    }
    let members = abilities.get(..member_count)?;
    describe_source_line_static_ability_loss_group(members)
        .or_else(|| describe_source_line_cast_activation_restriction_group(members))
        .or_else(|| describe_source_line_base_pt_grant_loss_group(members))
        .or_else(|| describe_source_line_grant_all_other_loss_group(members))
        .or_else(|| describe_source_line_type_addition_grant_group(members))
}

#[cfg(test)]
mod source_line_grant_loss_tests {
    use super::*;

    #[test]
    fn labeled_grant_and_all_other_loss_rejoin_one_authored_static_line() {
        let definition = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Static Grant And Loss Probe",
        )
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Hypertoxic Miasma — All lands have \"{T}: Add one mana of any color\" and lose all other abilities.",
        )
        .expect("grant and loss static line should compile");

        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition),
            vec![
                "Hypertoxic Miasma — All lands have \"{T}: Add one mana of any color\" and lose all other abilities."
                    .to_string()
            ]
        );
    }
}

fn unwrapped_static_model(
    ability: &crate::static_abilities::StaticAbility,
) -> Option<(
    &crate::static_abilities::CompiledStaticAbility,
    Option<&Condition>,
)> {
    let model = ability.compiled_model()?;
    match &model.payload {
        ironsmith_core::StaticAbilityPayload::Conditional {
            ability: inner,
            condition,
        } => Some((inner, Some(condition))),
        _ => Some((model, None)),
    }
}

fn describe_iterated_animation_value(value: &Value) -> Option<String> {
    match value.unhinted() {
        Value::ManaValueOf(value_spec) if matches!(value_spec.base(), ChooseSpec::Iterated) => {
            Some("its mana value".to_string())
        }
        Value::CountersOn(value_spec, Some(counter_type))
            if matches!(value_spec.base(), ChooseSpec::Iterated) =>
        {
            Some(format!(
                "the number of {} counters on it",
                counter_type.description()
            ))
        }
        _ => None,
    }
}

/// Recombine the layer-correct pieces of a permanent type animation. The
/// compiler intentionally lowers type-setting, dynamic base P/T, and optional
/// ability removal separately; this renderer rejoins them only when their
/// typed filters, conditions, and per-object values prove they are one clause.
fn describe_structural_dynamic_animation_bundle(abilities: &[Ability]) -> Option<(String, usize)> {
    let first = abilities.first()?;
    let AbilityKind::Static(first_static) = &first.kind else {
        return None;
    };
    let has_remove =
        first_static.id() == crate::static_abilities::StaticAbilityId::RemoveAllAbilitiesForFilter;
    let (remove_ability, set_types_ability, base_pt_ability, consumed) = if has_remove {
        (Some(first), abilities.get(1)?, abilities.get(2)?, 3usize)
    } else {
        (None, first, abilities.get(1)?, 2usize)
    };
    if first.functional_zones.as_slice() != [Zone::Battlefield]
        || set_types_ability.functional_zones != first.functional_zones
        || base_pt_ability.functional_zones != first.functional_zones
    {
        return None;
    }

    let AbilityKind::Static(set_types_static) = &set_types_ability.kind else {
        return None;
    };
    let AbilityKind::Static(base_pt_static) = &base_pt_ability.kind else {
        return None;
    };
    if set_types_static.id() != crate::static_abilities::StaticAbilityId::SetCardTypes
        || base_pt_static.id()
            != crate::static_abilities::StaticAbilityId::SetBasePowerToughnessForFilter
    {
        return None;
    }

    let (set_types_model, set_condition) = unwrapped_static_model(set_types_static)?;
    let (base_pt_model, base_pt_condition) = unwrapped_static_model(base_pt_static)?;
    let ironsmith_core::StaticAbilityPayload::SetCardTypes {
        filter: set_types_filter,
        card_types,
    } = &set_types_model.payload
    else {
        return None;
    };
    let ironsmith_core::StaticAbilityPayload::SetBasePowerToughnessValue {
        filter: base_pt_filter,
        power,
        toughness,
    } = &base_pt_model.payload
    else {
        return None;
    };
    if set_types_filter != base_pt_filter
        || set_condition != base_pt_condition
        || power != toughness
        || card_types.is_empty()
        || !card_types.contains(&CardType::Creature)
    {
        return None;
    }
    let value = describe_iterated_animation_value(power)?;

    let set_types_display =
        crate::static_abilities::StaticAbility::from_model(set_types_model.clone()).display();
    let (subject, type_phrase, type_verb) =
        split_static_predicate_with_verb(&set_types_display, &[" is ", " are "])?;
    if type_verb != "is" {
        return None;
    }

    let remove_subject = if let Some(remove_ability) = remove_ability {
        let AbilityKind::Static(remove_static) = &remove_ability.kind else {
            return None;
        };
        let (remove_model, remove_condition) = unwrapped_static_model(remove_static)?;
        let ironsmith_core::StaticAbilityPayload::RemoveAllAbilities(remove_filter) =
            &remove_model.payload
        else {
            return None;
        };
        if remove_filter != set_types_filter || remove_condition != set_condition {
            return None;
        }
        let remove_display =
            crate::static_abilities::StaticAbility::from_model(remove_model.clone()).display();
        let remove_subject = remove_display
            .strip_suffix(" lose all abilities")
            .or_else(|| remove_display.strip_suffix(" loses all abilities"))?;
        static_bundle_subjects_match(subject, remove_subject).then(|| remove_subject.to_string())?
    } else {
        subject.to_string()
    };

    let predicate =
        format!("{type_verb} {type_phrase} with power and toughness each equal to {value}");
    let text = match set_condition {
        Some(Condition::Not(inner)) => {
            let Condition::AttachedToSourceMatches(match_filter) = inner.as_ref() else {
                return None;
            };
            if has_remove {
                return None;
            }
            let excluded = describe_simple_attached_match(match_filter)?;
            format!(
                "As long as {} isn't {excluded}, it's {type_phrase} with power and toughness each equal to {value}",
                lowercase_first(subject),
            )
        }
        Some(_) => return None,
        None if has_remove => {
            format!("{remove_subject} loses all abilities and {predicate}")
        }
        None => format!("{subject} {predicate}"),
    };
    Some((text, consumed))
}

#[cfg(test)]
mod dynamic_animation_bundle_tests {
    use super::*;

    fn set_each_surface(filter: &mut ObjectFilter) {
        filter.set_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::Each));
    }

    fn modeled(model: crate::static_abilities::CompiledStaticAbility) -> Ability {
        Ability::static_ability(crate::static_abilities::StaticAbility::from_model(model))
    }

    #[test]
    fn animate_artifact_rejoins_conditional_type_and_dynamic_pt_layers() {
        let filter = ObjectFilter::artifact().match_tagged(
            "enchanted",
            crate::target::TaggedOpbjectRelation::IsTaggedObject,
        );
        let condition = Condition::Not(Box::new(Condition::AttachedToSourceMatches(
            ObjectFilter::creature(),
        )));
        let mana_value = Value::ManaValueOf(Box::new(ChooseSpec::Iterated));
        let set_types: crate::static_abilities::CompiledStaticAbility =
            ironsmith_core::StaticAbility::set_card_types(
                filter.clone(),
                vec![CardType::Artifact, CardType::Creature],
            )
            .with_condition(condition.clone());
        let set_pt: crate::static_abilities::CompiledStaticAbility =
            ironsmith_core::StaticAbility::set_base_power_toughness_value(
                filter,
                mana_value.clone(),
                mana_value,
            )
            .with_condition(condition);
        let abilities = vec![modeled(set_types), modeled(set_pt)];

        assert_eq!(
            describe_structural_dynamic_animation_bundle(&abilities),
            Some((
                "As long as enchanted artifact isn't a creature, it's an artifact creature with power and toughness each equal to its mana value"
                    .to_string(),
                2,
            )),
        );
    }

    #[test]
    fn march_rejoins_type_and_per_artifact_mana_value_layers() {
        let mut filter = ObjectFilter::artifact().without_type(CardType::Creature);
        set_each_surface(&mut filter);
        let mana_value = Value::ManaValueOf(Box::new(ChooseSpec::Iterated));
        let set_types: crate::static_abilities::CompiledStaticAbility =
            ironsmith_core::StaticAbility::set_card_types(
                filter.clone(),
                vec![CardType::Artifact, CardType::Creature],
            );
        let set_pt: crate::static_abilities::CompiledStaticAbility =
            ironsmith_core::StaticAbility::set_base_power_toughness_value(
                filter,
                mana_value.clone(),
                mana_value,
            );
        let abilities = vec![modeled(set_types), modeled(set_pt)];

        assert_eq!(
            describe_structural_dynamic_animation_bundle(&abilities),
            Some((
                "Each noncreature artifact is an artifact creature with power and toughness each equal to its mana value"
                    .to_string(),
                2,
            )),
        );
    }

    #[test]
    fn spark_rupture_rejoins_ability_loss_type_and_counter_value_layers() {
        let mut filter = ObjectFilter::planeswalker().with_counter_type(CounterType::Loyalty);
        set_each_surface(&mut filter);
        filter.set_counter_requirement_surface(true, true, false);
        let loyalty = Value::CountersOn(Box::new(ChooseSpec::Iterated), Some(CounterType::Loyalty));
        let remove: crate::static_abilities::CompiledStaticAbility =
            ironsmith_core::StaticAbility::remove_all_abilities(filter.clone());
        let set_types: crate::static_abilities::CompiledStaticAbility =
            ironsmith_core::StaticAbility::set_card_types(filter.clone(), vec![CardType::Creature]);
        let set_pt: crate::static_abilities::CompiledStaticAbility =
            ironsmith_core::StaticAbility::set_base_power_toughness_value(
                filter,
                loyalty.clone(),
                loyalty,
            );
        let abilities = vec![modeled(remove), modeled(set_types), modeled(set_pt)];

        assert_eq!(
            describe_structural_dynamic_animation_bundle(&abilities),
            Some((
                "Each planeswalker with one or more loyalty counters on it loses all abilities and is a creature with power and toughness each equal to the number of loyalty counters on it"
                    .to_string(),
                3,
            )),
        );
    }
}

/// Recombine the layer-correct pieces of a fixed-stat source animation
/// ("During your turn, this creature is a Bear with base power and toughness
/// 4/2."). The compiler lowers the creature type set, the subtype
/// replacement, and the fixed base P/T separately; rejoin them only when
/// their shared source filter and identical conditions prove they are one
/// authored clause.
fn describe_structural_source_fixed_animation_bundle(
    abilities: &[Ability],
) -> Option<(String, usize)> {
    let [set_types_ability, set_subtypes_ability, base_pt_ability, ..] = abilities else {
        return None;
    };
    for ability in [set_types_ability, set_subtypes_ability, base_pt_ability] {
        if ability.functional_zones.as_slice() != [Zone::Battlefield] {
            return None;
        }
    }
    let AbilityKind::Static(set_types_static) = &set_types_ability.kind else {
        return None;
    };
    let AbilityKind::Static(set_subtypes_static) = &set_subtypes_ability.kind else {
        return None;
    };
    let AbilityKind::Static(base_pt_static) = &base_pt_ability.kind else {
        return None;
    };
    let (set_types_model, set_condition) = unwrapped_static_model(set_types_static)?;
    let (set_subtypes_model, subtypes_condition) = unwrapped_static_model(set_subtypes_static)?;
    let (base_pt_model, base_pt_condition) = unwrapped_static_model(base_pt_static)?;
    let ironsmith_core::StaticAbilityPayload::SetCardTypes { filter, card_types } =
        &set_types_model.payload
    else {
        return None;
    };
    let ironsmith_core::StaticAbilityPayload::SetCreatureSubtypes {
        filter: subtypes_filter,
        subtypes,
    } = &set_subtypes_model.payload
    else {
        return None;
    };
    let ironsmith_core::StaticAbilityPayload::SetBasePowerToughnessValue {
        filter: base_pt_filter,
        power,
        toughness,
    } = &base_pt_model.payload
    else {
        return None;
    };
    if card_types.as_slice() != [CardType::Creature]
        || subtypes.is_empty()
        || !filter.source
        || filter != subtypes_filter
        || filter != base_pt_filter
        || set_condition != subtypes_condition
        || set_condition != base_pt_condition
    {
        return None;
    }
    let (Value::Fixed(power), Value::Fixed(toughness)) = (power.unhinted(), toughness.unhinted())
    else {
        return None;
    };
    let condition_prefix = match set_condition {
        None => "",
        Some(Condition::ActivationTiming(crate::ability::ActivationTiming::DuringYourTurn))
        | Some(Condition::YourTurn) => "During your turn, ",
        Some(_) => return None,
    };
    let subtype_phrase = subtypes
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" ");
    let text = format!(
        "{condition_prefix}this creature is {} with base power and toughness {power}/{toughness}",
        with_indefinite_article(&subtype_phrase),
    );
    Some((capitalize_first(&text), 3))
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
            crate::static_abilities::StaticAbilityId::GrantAbility
            | crate::static_abilities::StaticAbilityId::GrantObjectAbilityForFilter => {
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
                            .or_else(|| tail.strip_suffix(" in addition to their other colors"))?,
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
        format!("{subject} {first_predicate}, loses all other abilities, and {type_predicate}")
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
            format!("{subject} {first_predicate}, {grant_predicate}, and {type_predicate}")
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
        | crate::static_abilities::StaticAbilityId::SetLandSubtypes
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
fn describe_authored_attached_transform_bundle(abilities: &[Ability]) -> Option<(String, usize)> {
    let first_static = match &abilities.first()?.kind {
        AbilityKind::Static(static_ability) => static_ability,
        _ => return None,
    };
    let (set_idx, grant_tail) =
        if first_static.id() == crate::static_abilities::StaticAbilityId::AttachedAbilityGrant {
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
    let (subject, _, _) = split_static_predicate_with_verb(&set_display, &[" is ", " are "])?;
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

/// Recombine the independent layer-six ability removal and untap restriction
/// produced from one shared-subject attached-permanent clause. Both executable
/// models must identify the exact same enchanted object before their predicates
/// can share one rendered subject.
fn describe_structural_remove_all_abilities_untap_bundle(
    abilities: &[Ability],
) -> Option<(String, usize)> {
    let [remove_ability, untap_ability, ..] = abilities else {
        return None;
    };
    if remove_ability.functional_zones.as_slice() != [Zone::Battlefield]
        || untap_ability.functional_zones != remove_ability.functional_zones
    {
        return None;
    }

    let remove_filter = structural_static_effect_filter(
        remove_ability,
        crate::static_abilities::StaticAbilityId::RemoveAllAbilitiesForFilter,
    )?;
    let remove_display = static_display_with_id(
        remove_ability,
        crate::static_abilities::StaticAbilityId::RemoveAllAbilitiesForFilter,
    )?;

    let AbilityKind::Static(untap_static) = &untap_ability.kind else {
        return None;
    };
    let (untap_restriction, untap_display, untap_condition) =
        untap_static.rule_restriction_parts()?;
    let crate::effect::Restriction::Untap(untap_filter) = untap_restriction else {
        return None;
    };
    if untap_condition.is_some() || remove_filter != untap_filter {
        return None;
    }

    let subject = exact_enchanted_restriction_subject(remove_filter)?;
    let subject_lower = subject.to_ascii_lowercase();
    if !restriction_display_matches(
        &remove_display,
        &format!("{subject_lower} loses all abilities"),
    ) || !restriction_display_matches(
        untap_display,
        &format!("{subject_lower} doesn't untap during its controller's untap step"),
    ) {
        return None;
    }

    Some((
        format!(
            "{subject} loses all abilities and doesn't untap during its controller's untap step"
        ),
        2,
    ))
}

fn restriction_display_matches(display: &str, expected: &str) -> bool {
    display
        .trim()
        .trim_end_matches('.')
        .eq_ignore_ascii_case(expected)
}

/// Recombine adjacent block and attack-target restrictions that were authored
/// with one shared object subject. The two runtime restrictions remain
/// independent; this changes only their rendered sentence surface.
fn describe_structural_shared_subject_block_attack_bundle(
    abilities: &[Ability],
) -> Option<(String, usize)> {
    let [block_ability, attack_ability, ..] = abilities else {
        return None;
    };
    if block_ability.functional_zones.as_slice() != [Zone::Battlefield]
        || attack_ability.functional_zones != block_ability.functional_zones
    {
        return None;
    }

    let AbilityKind::Static(block_static) = &block_ability.kind else {
        return None;
    };
    let (block_restriction, block_display, block_condition) =
        block_static.rule_restriction_parts()?;
    let crate::effect::Restriction::Block(block_filter) = block_restriction else {
        return None;
    };
    if block_condition.is_some() {
        return None;
    }

    let AbilityKind::Static(attack_static) = &attack_ability.kind else {
        return None;
    };
    let (attack_restriction, attack_display, attack_condition) =
        attack_static.rule_restriction_parts()?;
    let crate::effect::Restriction::AttackPlayerOrPlaneswalkersControlledBy {
        attackers,
        player: PlayerFilter::You,
    } = attack_restriction
    else {
        return None;
    };
    if attack_condition.is_some() || attackers != block_filter {
        return None;
    }

    let block_display = block_display.trim().trim_end_matches('.');
    let marker = " can't block";
    let block_display_lower = block_display.to_ascii_lowercase();
    if !block_display_lower.ends_with(marker) {
        return None;
    }
    let subject = block_display[..block_display.len() - marker.len()].trim();
    if subject.is_empty()
        || !restriction_display_matches(
            attack_display,
            &format!("{subject} can't attack you or planeswalkers you control"),
        )
    {
        return None;
    }

    Some((
        format!(
            "{} can't block, and they can't attack you or planeswalkers you control",
            capitalize_first(subject)
        ),
        2,
    ))
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
    let (combat_filter, combat_predicate, connector) = match combat_restriction {
        crate::effect::Restriction::AttackOrBlock(filter) => {
            (filter, "can't attack or block", ", and")
        }
        crate::effect::Restriction::Block(filter) => (filter, "can't block", ", and"),
        crate::effect::Restriction::Untap(filter) => (
            filter,
            "doesn't untap during its controller's untap step",
            " and",
        ),
        _ => return None,
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
        &format!("{subject_lower} {combat_predicate}"),
    ) || !restriction_display_matches(
        activation_display,
        &format!("{subject_lower} activated abilities can't be activated"),
    ) {
        return None;
    }

    let mut text = format!(
        "{subject} {combat_predicate}{connector} its activated abilities can't be activated"
    );
    let mut consumed = 2;
    if let Some(ignore_ability) = abilities.get(2)
        && ignore_ability.functional_zones.as_slice() == [Zone::Battlefield]
        && let AbilityKind::Static(ignore_static) = &ignore_ability.kind
        && ignore_static.id()
            == crate::static_abilities::StaticAbilityId::AttachedControllerMaySacrificePermanentToIgnoreSourceEffectUntilEndOfTurn
    {
        let attached_noun = subject
            .strip_prefix("Enchanted ")
            .map(str::to_ascii_lowercase)?;
        text.push_str(&format!(
            ". That {attached_noun}'s controller may sacrifice a permanent of their choice for that player to ignore this effect until end of turn"
        ));
        consumed = 3;
    }

    Some((text, consumed))
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
    if anthem_ability.functional_zones.as_slice() != [Zone::Battlefield]
        || remove_abilities_ability.functional_zones != anthem_ability.functional_zones
    {
        return None;
    }
    let anthem_filter = structural_static_effect_filter(
        anthem_ability,
        crate::static_abilities::StaticAbilityId::Anthem,
    )?;
    let remove_filter = structural_static_effect_filter(
        remove_abilities_ability,
        crate::static_abilities::StaticAbilityId::RemoveAllAbilitiesForFilter,
    )?;
    if anthem_filter != remove_filter {
        return None;
    }
    let anthem = static_display_with_id(
        anthem_ability,
        crate::static_abilities::StaticAbilityId::Anthem,
    )?;
    let remove_abilities = static_display_with_id(
        remove_abilities_ability,
        crate::static_abilities::StaticAbilityId::RemoveAllAbilitiesForFilter,
    )?;
    let (subject, modifier) = split_static_predicate(&anthem, " gets ")?;
    let remove_subject = remove_abilities
        .strip_suffix(" loses all abilities")
        .or_else(|| remove_abilities.strip_suffix(" lose all abilities"))?
        .trim();
    if !static_bundle_subjects_match(subject, remove_subject) {
        return None;
    }

    Some((
        capitalize_first(&format!(
            "{subject} gets {modifier} and loses all abilities"
        )),
        2,
    ))
}

fn describe_structural_anthem_set_colors_bundle(abilities: &[Ability]) -> Option<(String, usize)> {
    let [anthem_ability, set_colors_ability, ..] = abilities else {
        return None;
    };
    if anthem_ability.functional_zones.as_slice() != [Zone::Battlefield]
        || set_colors_ability.functional_zones != anthem_ability.functional_zones
    {
        return None;
    }
    let anthem_filter = structural_static_effect_filter(
        anthem_ability,
        crate::static_abilities::StaticAbilityId::Anthem,
    )?;
    let color_filter = structural_static_effect_filter(
        set_colors_ability,
        crate::static_abilities::StaticAbilityId::SetColors,
    )?;
    if anthem_filter != color_filter {
        return None;
    }

    let anthem = static_display_with_id(
        anthem_ability,
        crate::static_abilities::StaticAbilityId::Anthem,
    )?;
    let set_colors = static_display_with_id(
        set_colors_ability,
        crate::static_abilities::StaticAbilityId::SetColors,
    )?;
    let (subject, modifier, get_verb) =
        split_static_predicate_with_verb(&anthem, &[" gets ", " get "])?;
    let (color_subject, color, is_verb) =
        split_static_predicate_with_verb(&set_colors, &[" is ", " are "])?;
    if !static_bundle_subjects_match(subject, color_subject) {
        return None;
    }

    Some((
        capitalize_first(&format!(
            "{subject} {get_verb} {modifier} and {is_verb} {color}"
        )),
        2,
    ))
}

/// Preserve an authored sentence boundary when a P/T modifier is followed by
/// a separate characteristic-setting clause. The color and additive subtype
/// pieces belong together by layer semantics, while the preceding anthem does
/// not. All three typed pieces must affect the exact same object set.
fn describe_structural_anthem_then_color_subtype_bundle(
    abilities: &[Ability],
) -> Option<(Vec<String>, usize)> {
    let [anthem_ability, set_colors_ability, add_subtypes_ability, ..] = abilities else {
        return None;
    };
    if anthem_ability.functional_zones.as_slice() != [Zone::Battlefield]
        || set_colors_ability.functional_zones != anthem_ability.functional_zones
        || add_subtypes_ability.functional_zones != anthem_ability.functional_zones
    {
        return None;
    }
    let anthem_filter = structural_static_effect_filter(
        anthem_ability,
        crate::static_abilities::StaticAbilityId::Anthem,
    )?;
    let color_filter = structural_static_effect_filter(
        set_colors_ability,
        crate::static_abilities::StaticAbilityId::SetColors,
    )?;
    let subtype_filter = structural_static_effect_filter(
        add_subtypes_ability,
        crate::static_abilities::StaticAbilityId::AddSubtypes,
    )?;
    if anthem_filter != color_filter || anthem_filter != subtype_filter {
        return None;
    }

    let anthem = static_display_with_id(
        anthem_ability,
        crate::static_abilities::StaticAbilityId::Anthem,
    )?;
    let set_colors = static_display_with_id(
        set_colors_ability,
        crate::static_abilities::StaticAbilityId::SetColors,
    )?;
    let add_subtypes = static_display_with_id(
        add_subtypes_ability,
        crate::static_abilities::StaticAbilityId::AddSubtypes,
    )?;
    let (subject, modifier, get_verb) =
        split_static_predicate_with_verb(&anthem, &[" gets ", " get "])?;
    let (color_subject, color, is_verb) =
        split_static_predicate_with_verb(&set_colors, &[" is ", " are "])?;
    let (subtype_subject, subtype, subtype_verb) =
        split_static_predicate_with_verb(&add_subtypes, &[" is ", " are "])?;
    if !static_bundle_subjects_match(subject, color_subject)
        || !static_bundle_subjects_match(subject, subtype_subject)
    {
        return None;
    }

    Some((
        vec![
            capitalize_first(&format!("{subject} {get_verb} {modifier}")),
            capitalize_first(&format!(
                "{color_subject} {is_verb} {color} and {subtype_verb} {subtype}"
            )),
        ],
        3,
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

    let costs = activated.mana_cost.as_all()?;
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
            if matches!(
                static_ability.id(),
                crate::static_abilities::StaticAbilityId::MakeColorless
                    | crate::static_abilities::StaticAbilityId::CantBeCountered
            )
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
mod counter_linked_cross_segment_rendering_tests {
    use super::*;

    fn granted_sacrifice_trigger() -> crate::ability::Ability {
        crate::ability::Ability::triggered(
            crate::triggers::Trigger::becomes_targeted(),
            vec![Effect::new(crate::effects::SacrificeTargetEffect::new(
                ChooseSpec::Source,
            ))],
        )
    }

    #[test]
    fn adjacent_counter_and_grant_segments_retain_the_typed_object_subject() {
        for (filter, object_kind) in [
            (ObjectFilter::land(), "land"),
            (ObjectFilter::creature(), "creature"),
        ] {
            let target_tag = TagKey::from("targeted_0");
            let put = Effect::new(crate::effects::PutCountersEffect::new(
                CounterType::Named("test"),
                Value::Fixed(1),
                ChooseSpec::target(ChooseSpec::Object(filter)),
            ))
            .tag(target_tag.clone());
            let granted_trigger = crate::ability::Ability::triggered(
                crate::triggers::Trigger::beginning_of_upkeep(PlayerFilter::You),
                vec![Effect::deal_damage(
                    Value::Fixed(1),
                    ChooseSpec::Player(PlayerFilter::You),
                )],
            );
            let grant = Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
                ChooseSpec::Tagged(target_tag),
                crate::continuous::Modification::AddAbilityGeneric(granted_trigger),
                Until::ForAsLongAs(
                    ironsmith_core::ContinuousDurationPredicate::affected_object_has_counter(
                        CounterType::Named("test"),
                    ),
                ),
            ))
            .tag("granted_0");
            let program = crate::resolution::ResolutionProgram::new(vec![
                crate::resolution::ResolutionSegment::from_effects(vec![put]),
                crate::resolution::ResolutionSegment::from_effects(vec![grant]),
            ]);

            let rendered = describe_resolution_program(&program);
            assert!(
                rendered.contains(&format!(
                    "For as long as that {object_kind} has a test counter on it"
                )),
                "{rendered}"
            );
            assert!(
                rendered.contains(&format!("this {object_kind} deals 1 damage to you")),
                "{rendered}"
            );
            assert!(!rendered.contains("For as long as it has"), "{rendered}");
        }
    }

    #[test]
    fn returned_creature_entry_counter_and_linked_grant_retain_duration_scope() {
        let returned_tag = TagKey::from("returned_0");
        let returned = Effect::new(
            crate::effects::ReturnFromGraveyardToBattlefieldEffect::new(
                ChooseSpec::target(ChooseSpec::Object(
                    ObjectFilter::creature()
                        .owned_by(PlayerFilter::You)
                        .in_zone(Zone::Graveyard),
                )),
                false,
            )
            .with_entry_counter(
                ironsmith_core::BattlefieldEntryCounterSpec::new(
                    CounterType::Named("mannequin"),
                    Value::Fixed(1),
                    ironsmith_core::BattlefieldEntryCounterSurface::Inline,
                )
                .for_matching_object(ObjectFilter::creature()),
            ),
        )
        .tag(returned_tag.clone());
        let grant = Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
            ChooseSpec::Tagged(returned_tag),
            crate::continuous::Modification::AddAbilityGeneric(granted_sacrifice_trigger()),
            Until::ForAsLongAs(
                ironsmith_core::ContinuousDurationPredicate::affected_object_has_counter(
                    CounterType::Named("mannequin"),
                ),
            ),
        ))
        .tag("granted_0");
        let program = crate::resolution::ResolutionProgram::new(vec![
            crate::resolution::ResolutionSegment::from_effects(vec![returned]),
            crate::resolution::ResolutionSegment::from_effects(vec![grant]),
        ]);

        let rendered = describe_resolution_program(&program);
        assert!(
            rendered.contains("For as long as that creature has a mannequin counter on it, it has"),
            "{rendered}"
        );
        assert!(!rendered.contains("For as long as it has"), "{rendered}");
    }
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

    fn threshold_action(action: TargetThresholdAction, target: ChooseSpec, tag: &TagKey) -> Effect {
        match action {
            TargetThresholdAction::Destroy => {
                Effect::new(crate::effects::DestroyEffect::with_spec(target)).tag(tag.clone())
            }
            TargetThresholdAction::GainControl => Effect::new(
                crate::effects::ApplyContinuousEffect::with_spec_runtime(
                    target,
                    crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController,
                    Until::Forever,
                ),
            )
            .tag(tag.clone()),
        }
    }

    fn threshold_condition(
        characteristic: TargetThresholdCharacteristic,
        tag: &TagKey,
        limit: Value,
    ) -> Condition {
        let target = Box::new(ChooseSpec::Tagged(tag.clone()));
        let left = match characteristic {
            TargetThresholdCharacteristic::Toughness => Value::ToughnessOf(target),
            TargetThresholdCharacteristic::ManaValue => Value::ManaValueOf(target),
        };
        Condition::ValueComparison {
            left,
            operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
            right: limit,
        }
    }

    fn trailing_threshold_effect(
        action: TargetThresholdAction,
        characteristic: TargetThresholdCharacteristic,
        target: ChooseSpec,
        tag: &TagKey,
        limit: Value,
    ) -> Effect {
        Effect::new(
            crate::effects::ConditionalEffect::new(
                threshold_condition(characteristic, tag, limit),
                vec![threshold_action(action, target, tag)],
                Vec::new(),
            )
            .with_surface(ironsmith_core::ConditionalSurface::TrailingIf),
        )
    }

    fn threshold_replacement_segment(
        action: TargetThresholdAction,
        characteristic: TargetThresholdCharacteristic,
        target: ChooseSpec,
        default_limit: Value,
        replacement_limit: Value,
        paid_condition: Condition,
        leading_instead: bool,
    ) -> crate::resolution::ResolutionSegment {
        let default_tag = TagKey::from("threshold_default");
        let replacement_tag = TagKey::from("threshold_replacement");
        let mut branch = crate::resolution::SelfReplacementBranch::new(
            paid_condition,
            vec![trailing_threshold_effect(
                action,
                characteristic,
                target.clone(),
                &replacement_tag,
                replacement_limit,
            )],
        );
        branch.leading_instead_surface = leading_instead;
        crate::resolution::ResolutionSegment {
            default_effects: vec![
                Effect::new(crate::effects::TargetOnlyEffect::new(target.clone())),
                trailing_threshold_effect(
                    action,
                    characteristic,
                    target,
                    &default_tag,
                    default_limit,
                ),
            ],
            self_replacements: vec![branch],
            starts_new_source_line: false,
        }
    }

    #[test]
    fn folds_one_creature_target_through_madness_toughness_thresholds() {
        let segment = threshold_replacement_segment(
            TargetThresholdAction::GainControl,
            TargetThresholdCharacteristic::Toughness,
            ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature())),
            Value::Fixed(2),
            Value::X,
            Condition::ThisSpellPaidLabel("Madness".into()),
            true,
        );

        assert_eq!(
            describe_target_threshold_paid_cost_self_replacement(&segment).as_deref(),
            Some(
                "Gain control of target creature if its toughness is 2 or less. If this spell's madness cost was paid, instead gain control of that creature if its toughness is X or less"
            )
        );
    }

    #[test]
    fn folds_one_artifact_target_through_kicked_mana_value_thresholds() {
        let segment = threshold_replacement_segment(
            TargetThresholdAction::Destroy,
            TargetThresholdCharacteristic::ManaValue,
            ChooseSpec::target(ChooseSpec::Object(ObjectFilter::artifact())),
            Value::Fixed(2),
            Value::Fixed(5),
            Condition::ThisSpellWasKicked,
            false,
        );

        assert_eq!(
            describe_target_threshold_paid_cost_self_replacement(&segment).as_deref(),
            Some(
                "Destroy target artifact if its mana value is 2 or less. If this spell was kicked, destroy that artifact if its mana value is 5 or less instead"
            )
        );
    }

    #[test]
    fn folds_repeated_target_and_filter_backed_replacement_threshold() {
        let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::artifact()));
        let mut segment = threshold_replacement_segment(
            TargetThresholdAction::Destroy,
            TargetThresholdCharacteristic::ManaValue,
            target.clone(),
            Value::Fixed(2),
            Value::Fixed(5),
            Condition::TurnHistory(ironsmith_core::TurnHistoryCondition::SourceWasKicked {
                surface: crate::target::SourceReferenceSurface::ThisPermanentType(
                    "this spell".to_string(),
                ),
            }),
            false,
        );
        let replacement_tag = TagKey::from("threshold_replacement");
        let replacement = segment.self_replacements[0].replacement_effects[0]
            .downcast_ref::<crate::effects::ConditionalEffect>()
            .expect("replacement threshold")
            .clone();
        let mut replacement = replacement;
        let mut threshold_filter = ObjectFilter::default();
        threshold_filter.mana_value = Some(crate::filter::Comparison::LessThanOrEqual(5));
        replacement.condition = Condition::TaggedObjectMatches(replacement_tag, threshold_filter);
        segment.self_replacements[0].replacement_effects = vec![
            Effect::new(crate::effects::TargetOnlyEffect::new(target)),
            Effect::new(replacement),
        ];

        assert_eq!(
            describe_target_threshold_paid_cost_self_replacement(&segment).as_deref(),
            Some(
                "Destroy target artifact if its mana value is 2 or less. If this spell was kicked, destroy that artifact if its mana value is 5 or less instead"
            )
        );
    }

    #[test]
    fn threshold_replacement_does_not_fold_a_different_replacement_target() {
        let mut segment = threshold_replacement_segment(
            TargetThresholdAction::Destroy,
            TargetThresholdCharacteristic::ManaValue,
            ChooseSpec::target(ChooseSpec::Object(ObjectFilter::artifact())),
            Value::Fixed(2),
            Value::Fixed(5),
            Condition::ThisSpellWasKicked,
            false,
        );
        let replacement_tag = TagKey::from("threshold_replacement");
        segment.self_replacements[0].replacement_effects = vec![trailing_threshold_effect(
            TargetThresholdAction::Destroy,
            TargetThresholdCharacteristic::ManaValue,
            ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature())),
            &replacement_tag,
            Value::Fixed(5),
        )];

        assert_eq!(
            describe_target_threshold_paid_cost_self_replacement(&segment),
            None
        );
    }

    #[test]
    fn threshold_replacement_does_not_fold_an_unlinked_characteristic() {
        let mut segment = threshold_replacement_segment(
            TargetThresholdAction::GainControl,
            TargetThresholdCharacteristic::Toughness,
            ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature())),
            Value::Fixed(2),
            Value::X,
            Condition::ThisSpellPaidLabel("Madness".into()),
            true,
        );
        let replacement = segment.self_replacements[0].replacement_effects[0]
            .downcast_ref::<crate::effects::ConditionalEffect>()
            .expect("replacement threshold")
            .clone();
        let mut replacement = replacement;
        replacement.condition = Condition::ValueComparison {
            left: Value::ToughnessOf(Box::new(ChooseSpec::Tagged(TagKey::from(
                "unrelated_target",
            )))),
            operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
            right: Value::X,
        };
        segment.self_replacements[0].replacement_effects = vec![Effect::new(replacement)];

        assert_eq!(
            describe_target_threshold_paid_cost_self_replacement(&segment),
            None
        );
    }

    #[test]
    fn morbid_discard_replacement_keeps_reveal_choice_discard_inline() {
        let player = PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any));
        let chosen = TagKey::from("chosen_hand_cards");
        let look = Effect::new(crate::effects::LookAtHandEffect::reveal(
            ChooseSpec::Player(player.clone()),
        ));
        let choose = Effect::new(
            crate::effects::ChooseObjectsEffect::new(
                ObjectFilter::default()
                    .in_zone(Zone::Hand)
                    .owned_by(player.clone()),
                ChoiceCount::exactly(2),
                PlayerFilter::You,
                chosen.clone(),
            )
            .in_zone(Zone::Hand),
        );
        let chosen_filter = ObjectFilter::tagged(chosen.clone());
        let discard = Effect::new(crate::effects::DiscardEffect::new_with_filter(
            Value::Count(chosen_filter.clone()),
            player,
            false,
            Some(chosen_filter),
        ));
        let branch = crate::resolution::SelfReplacementBranch::new(
            Condition::CreatureDiedThisTurn,
            vec![look, choose, discard],
        )
        .with_presentation_label(Some(
            crate::ability::PresentationLabel::from_ability_word("Morbid"),
        ));
        let segment = crate::resolution::ResolutionSegment {
            default_effects: vec![Effect::new(crate::effects::DiscardEffect::new(
                Value::Fixed(2),
                PlayerFilter::target_player(),
                false,
            ))],
            self_replacements: vec![branch],
            starts_new_source_line: false,
        };

        let rendered = describe_single_self_replacement_segment(&segment)
            .expect("typed hand replacement should render");
        let rendered =
            apply_self_replacement_presentation_label(&segment.self_replacements[0], rendered);
        assert_eq!(
            rendered,
            "Target player discards two cards. Morbid — If a creature died this turn, instead that player reveals their hand, you choose two cards from it, then that player discards those cards"
        );
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
            starts_new_source_line: false,
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
    fn conjoins_distinct_amount_same_source_damage_groups_without_sequencing_them() {
        let mut attacking_filter = ObjectFilter::creature().in_zone(Zone::Battlefield);
        attacking_filter.attacking = true;
        let attacking_creatures = Effect::for_each(
            attacking_filter,
            vec![Effect::deal_damage(Value::Fixed(2), ChooseSpec::Iterated)],
        );
        let controller_and_creatures =
            Effect::new(crate::effects::SequenceEffect::coordinated(vec![
                Effect::deal_damage(Value::Fixed(1), ChooseSpec::SourceController),
                Effect::for_each(
                    ObjectFilter::creature()
                        .controlled_by(PlayerFilter::You)
                        .in_zone(Zone::Battlefield),
                    vec![Effect::deal_damage(Value::Fixed(1), ChooseSpec::Iterated)],
                ),
            ]));

        assert_eq!(
            describe_conjoined_same_source_damage(&[attacking_creatures, controller_and_creatures])
                .as_deref(),
            Some(
                "Deal 2 damage to each attacking creature and 1 damage to you and each creature you control"
            )
        );
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
            starts_new_source_line: false,
        };

        assert_eq!(
            describe_single_self_replacement_segment(&segment).as_deref(),
            Some(
                "Deal 1 damage to each creature and each player. If you control a snow Swamp, deal 2 damage to each creature and each player instead"
            )
        );
    }

    #[test]
    fn trailing_damage_replacement_reuses_typed_source_and_target() {
        let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()));
        let replacement = Effect::new(crate::effects::ExecuteWithSourceEffect::new(
            ChooseSpec::Source,
            Effect::deal_damage(Value::Fixed(4), target.clone()),
        ));
        let mut branch = crate::resolution::SelfReplacementBranch::new(
            Condition::ThisSpellPaidLabel(crate::cost::OptionalCostKind::Additional.into()),
            vec![replacement],
        );
        branch.condition_after_replacement = true;
        let segment = crate::resolution::ResolutionSegment {
            default_effects: vec![Effect::deal_damage(Value::Fixed(2), target.clone())],
            self_replacements: vec![branch],
            starts_new_source_line: false,
        };

        assert_eq!(
            describe_single_self_replacement_segment(&segment).as_deref(),
            Some(
                "Deal 2 damage to target creature. It deals 4 damage to that creature instead if this spell's additional cost was paid"
            )
        );

        let mut mismatched_source = segment;
        mismatched_source.self_replacements[0].replacement_effects =
            vec![Effect::new(crate::effects::ExecuteWithSourceEffect::new(
                ChooseSpec::Tagged(TagKey::from("other_source")),
                Effect::deal_damage(Value::Fixed(4), target),
            ))];
        assert_eq!(
            describe_trailing_same_source_damage_self_replacement(&mismatched_source),
            None,
            "an unrelated explicit damage source must not collapse to the pronoun `It`"
        );
    }

    #[test]
    fn same_target_zone_sequence_uses_pronoun_then_card_referent() {
        let oracle = "{2}{W}: Return target permanent you control to its owner's hand. If it has unearth, instead exile it, then return that card to its owner's hand. Activate only during your turn.";
        let definition =
            crate::cards::builders::CardDefinitionBuilder::new(
                crate::ids::CardId::new(),
                "Meticulous Excavation",
            )
            .card_types(vec![CardType::Enchantment])
            .parse_text(oracle)
            .expect("same-target sequential zone replacement should compile");

        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition),
            vec![oracle.to_string()]
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
            starts_new_source_line: false,
        };

        assert_eq!(
            describe_single_self_replacement_segment(&segment).as_deref(),
            Some(
                "Deal 1 damage to each creature. It deals 2 damage to each creature instead if this spell was kicked"
            )
        );
    }

    #[test]
    fn damaged_target_quality_conditions_reuse_the_original_target_referent() {
        for (default_text, condition, expected) in [
            (
                "Deal 2 damage to target creature",
                "a creature with toxic was dealt damage this way",
                "that creature has toxic",
            ),
            (
                "Deal 3 damage to target creature",
                "a permanent with a +1/+1 counter on it was dealt damage this way",
                "that creature has a +1/+1 counter on it",
            ),
            (
                "Deal 1 damage to target creature",
                "a white or blue permanent was dealt damage this way",
                "that creature is white or blue",
            ),
            (
                "Deal 3 damage to target creature",
                "that creature is a permanent with a +1/+1 counter on it",
                "that creature has a +1/+1 counter on it",
            ),
            (
                "Deal 1 damage to target creature",
                "that creature is a white or blue permanent",
                "that creature is white or blue",
            ),
            (
                "Deal 2 damage to target creature",
                "that creature is an artifact creature",
                "it's an artifact creature",
            ),
            (
                "Deal 2 damage to target creature",
                "an artifact creature was dealt damage this way",
                "it's an artifact creature",
            ),
            (
                "Deal 2 damage to target creature",
                "it's an artifact creature",
                "it's an artifact creature",
            ),
            (
                "Deal 5 damage to target creature or planeswalker",
                "a Spirit was dealt damage this way",
                "that permanent is a Spirit",
            ),
            (
                "Deal 2 damage to any target",
                "a creature was dealt damage this way",
                "it is a creature",
            ),
        ] {
            assert_eq!(
                normalize_target_quality_condition(default_text, condition),
                expected,
                "{default_text}; {condition}"
            );
        }
    }

    #[test]
    fn named_spell_damage_replacements_keep_source_target_and_quality_surfaces() {
        for (name, oracle) in [
            (
                "Lightning Dart",
                "Lightning Dart deals 1 damage to target creature. If that creature is white or blue, Lightning Dart deals 4 damage to it instead.",
            ),
            (
                "Bring Low",
                "Bring Low deals 3 damage to target creature. If that creature has a +1/+1 counter on it, Bring Low deals 5 damage to it instead.",
            ),
            (
                "Electrostatic Bolt",
                "Electrostatic Bolt deals 2 damage to target creature. If it's an artifact creature, Electrostatic Bolt deals 4 damage to it instead.",
            ),
        ] {
            let definition =
                crate::cards::builders::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
                    .card_types(vec![CardType::Instant])
                    .parse_text(oracle)
                    .expect("the typed damage self-replacement should compile");
            let raw_condition = definition
                .spell_effect
                .as_ref()
                .and_then(|program| program.segments.first())
                .and_then(|segment| segment.self_replacements.first())
                .map(|branch| super::normalize_common::describe_condition(&branch.condition));

            assert_eq!(
                crate::compiled_text::compiled_text_lines(&definition),
                vec![oracle.to_string()],
                "{name}; raw condition: {raw_condition:?}"
            );
        }
    }

    #[test]
    fn named_spell_damage_replacement_rejects_mismatched_target_or_source() {
        let oracle = "Shape Probe deals 1 damage to target creature. If that creature is white, Shape Probe deals 4 damage to it instead.";
        let definition = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Shape Probe",
        )
        .card_types(vec![CardType::Instant])
        .parse_text(oracle)
        .expect("the typed damage self-replacement should compile");
        let mut program = definition
            .spell_effect
            .clone()
            .expect("the instant should have a spell-resolution program");
        let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()));

        program.segments[0].self_replacements[0].replacement_effects = vec![Effect::deal_damage(
            Value::Fixed(4),
            ChooseSpec::target(ChooseSpec::Object(ObjectFilter::artifact())),
        )];
        assert_eq!(
            describe_named_spell_shared_target_damage_self_replacement(&definition, &program),
            None
        );

        program.segments[0].self_replacements[0].replacement_effects =
            vec![Effect::new(crate::effects::ExecuteWithSourceEffect::new(
                ChooseSpec::Tagged(TagKey::from("other_source")),
                Effect::deal_damage(Value::Fixed(4), target),
            ))];
        assert_eq!(
            describe_named_spell_shared_target_damage_self_replacement(&definition, &program),
            None
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
            starts_new_source_line: false,
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
            false,
        );
        assert_eq!(
            rendered,
            "Destroy all enchantments. If there are seven or more cards in your graveyard, instead destroy all enchantments. Return all cards destroyed this way to the battlefield"
        );
        assert!(!rendered.contains("and return"));
    }

    #[test]
    fn preserves_authored_leading_instead_on_one_coordinated_clause() {
        let rendered = format_self_replacement_fallback(
            "Draw X cards",
            "X is 10 or more",
            "shuffle your graveyard into your library, draw X cards, and untap five lands",
            true,
        );
        assert_eq!(
            rendered,
            "Draw X cards. If X is 10 or more, instead shuffle your graveyard into your library, draw X cards, and untap five lands"
        );
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
    fn addendum_scry_then_same_draw_keeps_leading_instead_and_imperative_draw() {
        let mut branch = crate::resolution::SelfReplacementBranch::new(
            Condition::ThisSpellPaidLabel("CastDuringYourMainPhase".into()),
            vec![Effect::scry(Value::Fixed(3)), Effect::draw(Value::Fixed(3))],
        )
        .with_presentation_label(Some(
            crate::ability::PresentationLabel::from_ability_word("Addendum"),
        ));
        branch.condition_after_replacement = false;
        let segment = crate::resolution::ResolutionSegment {
            default_effects: vec![Effect::draw(Value::Fixed(3))],
            self_replacements: vec![branch],
            starts_new_source_line: false,
        };

        let rendered = describe_single_self_replacement_segment(&segment)
            .expect("typed scry/draw replacement should render");
        let rendered =
            apply_self_replacement_presentation_label(&segment.self_replacements[0], rendered);
        assert_eq!(
            rendered,
            "Draw three cards. Addendum — If you cast this spell during your main phase, instead scry 3, then draw three cards"
        );

        let mut mismatched = segment.clone();
        mismatched.self_replacements[0].replacement_effects[1] = Effect::draw(Value::Fixed(2));
        assert_eq!(
            describe_scry_then_same_draw_self_replacement(&mismatched),
            None,
            "a replacement that changes the draw count must not use the same-draw surface"
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
    fn numbered_intrinsic_keyword_abilities_preserve_ast_boundaries() {
        let merged = merge_adjacent_keyword_surface_lines(vec![
            "Keyword ability 0: Flying".to_string(),
            "Keyword ability 1: Menace".to_string(),
        ]);

        assert_eq!(
            merged,
            vec![
                "Keyword ability 0: Flying".to_string(),
                "Keyword ability 1: Menace".to_string(),
            ]
        );
    }

    #[test]
    fn daybound_card_keeps_each_modeled_keyword_on_its_own_oracle_line() {
        let definition = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Daybound Surface Probe",
        )
        .card_types(vec![CardType::Creature])
        .reach()
        .daybound()
        .build();

        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition),
            vec!["Reach".to_string(), "Daybound".to_string()]
        );
    }

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
    fn partial_color_grant_run_keeps_authored_source_lines_separate() {
        let lines = vec![
            "Static ability 0: White creatures you control have protection from black".to_string(),
            "Static ability 1: Black creatures you control have protection from white".to_string(),
        ];

        assert_eq!(merge_adjacent_keyword_surface_lines(lines.clone()), lines);
    }

    #[test]
    fn full_wubrg_grant_run_keeps_the_conditional_compaction() {
        let merged = merge_adjacent_keyword_surface_lines(vec![
            "Static ability 0: White creatures you control have vigilance".to_string(),
            "Static ability 1: Blue creatures you control have hexproof".to_string(),
            "Static ability 2: Black creatures you control have lifelink".to_string(),
            "Static ability 3: Red creatures you control have first strike".to_string(),
            "Static ability 4: Green creatures you control have trample".to_string(),
        ]);

        assert_eq!(
            merged,
            vec![
                "Static ability 0: Each creature you control has vigilance if it's white, hexproof if it's blue, lifelink if it's black, first strike if it's red, and trample if it's green"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn annihilator_stays_on_its_own_keyword_line() {
        let merged = merge_adjacent_keyword_surface_lines(vec![
            "Keyword ability 0: Annihilator 2".to_string(),
            "Keyword ability 1: trample".to_string(),
        ]);

        assert_eq!(
            merged,
            vec![
                "Keyword ability 0: Annihilator 2".to_string(),
                "Keyword ability 1: trample".to_string(),
            ]
        );
    }

    #[test]
    fn trailing_annihilator_preserves_its_ast_boundary() {
        let merged = merge_adjacent_keyword_surface_lines(vec![
            "Keyword ability 0: Trample".to_string(),
            "Static ability 1: Annihilator 1".to_string(),
        ]);

        assert_eq!(
            merged,
            vec![
                "Keyword ability 0: Trample".to_string(),
                "Static ability 1: Annihilator 1".to_string(),
            ]
        );
    }

    #[test]
    fn conventional_long_numbered_keyword_list_still_compacts() {
        let merged = merge_adjacent_keyword_surface_lines(vec![
            "Keyword ability 0: Flying".to_string(),
            "Keyword ability 1: First strike".to_string(),
            "Keyword ability 2: Vigilance".to_string(),
            "Keyword ability 3: Trample".to_string(),
        ]);

        assert_eq!(
            merged,
            vec!["Keyword ability 0: Flying, first strike, vigilance, trample".to_string()]
        );
    }

    #[test]
    fn decomposed_protection_pair_still_compacts_to_one_surface() {
        let merged = merge_adjacent_keyword_surface_lines(vec![
            "Keyword ability 0: Protection from black".to_string(),
            "Keyword ability 1: Protection from red".to_string(),
        ]);

        assert_eq!(
            merged,
            vec!["Keyword ability 0: Protection from black and from red".to_string()]
        );
    }

    #[test]
    fn unnumbered_keyword_fragments_still_compact_within_one_surface() {
        let merged =
            merge_adjacent_keyword_surface_lines(vec!["Flying".to_string(), "Haste".to_string()]);

        assert_eq!(merged, vec!["Flying, haste".to_string()]);
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
mod keyword_condition_ladder_tests {
    use super::*;

    fn keyword_model(
        id: crate::static_abilities::StaticAbilityId,
        label: &str,
    ) -> crate::static_abilities::CompiledStaticAbility {
        ironsmith_core::StaticAbility {
            id: Some(id),
            label: label.to_string(),
            payload: ironsmith_core::StaticAbilityPayload::None,
        }
    }

    fn count_condition(filter: ObjectFilter, display: impl Into<String>) -> Condition {
        Condition::CountComparison {
            count: ironsmith_core::AnthemCountExpression::MatchingFilter(filter),
            comparison: Comparison::GreaterThanOrEqual(1),
            display: Some(display.into()),
        }
    }

    fn modeled_keyword_grant(
        id: crate::static_abilities::StaticAbilityId,
        label: &str,
        condition: Condition,
    ) -> Ability {
        let granted = ironsmith_core::Ability {
            kind: ironsmith_core::AbilityKind::Static(keyword_model(id, label)),
            functional_zones: vec![Zone::Battlefield],
        };
        let grant = ironsmith_core::GrantObjectAbilityForFilter::new(
            ObjectFilter::source(),
            granted,
            label,
        )
        .with_condition(condition);
        let model: crate::static_abilities::CompiledStaticAbility =
            ironsmith_core::StaticAbility::new(grant);
        Ability::static_ability(crate::static_abilities::StaticAbility::from_model(model))
    }

    fn keyword_condition_filter(id: crate::static_abilities::StaticAbilityId) -> ObjectFilter {
        let mut filter = ObjectFilter::default().in_zone(Zone::Exile);
        filter.static_abilities.push(id);
        filter
    }

    fn modeled_source_anthem(power: i32, toughness: i32, condition: Condition) -> Ability {
        let anthem = ironsmith_core::Anthem::for_source(power, toughness).with_condition(condition);
        let model: crate::static_abilities::CompiledStaticAbility =
            ironsmith_core::StaticAbility::new(anthem);
        Ability::static_ability(crate::static_abilities::StaticAbility::from_model(model))
    }

    fn basic_land_condition(subtype: Subtype, article: &str) -> Condition {
        let mut filter = ObjectFilter::default();
        filter.controller = Some(PlayerFilter::You);
        filter.subtypes.push(subtype);
        count_condition(filter, format!("you control {article}"))
    }

    #[test]
    fn complete_typed_keyword_ladder_uses_same_is_true_surface() {
        use crate::static_abilities::StaticAbilityId::{Flying, Haste, Vigilance};

        let abilities = vec![
            modeled_keyword_grant(
                Flying,
                "flying",
                count_condition(
                    keyword_condition_filter(Flying),
                    "there is a card exiled with it with flying",
                ),
            ),
            modeled_keyword_grant(
                Haste,
                "haste",
                count_condition(
                    keyword_condition_filter(Haste),
                    "there is a card exiled with it with haste",
                ),
            ),
            modeled_keyword_grant(
                Vigilance,
                "vigilance",
                count_condition(
                    keyword_condition_filter(Vigilance),
                    "there is a card exiled with it with vigilance",
                ),
            ),
        ];

        assert_eq!(
            describe_structural_keyword_same_is_true_ladder(&abilities, "this creature"),
            Some((
                "As long as a card exiled with it has flying, this creature has flying. The same is true for haste and vigilance"
                    .to_string(),
                3,
            )),
        );
    }

    #[test]
    fn delve_linked_leading_grant_rejoins_variant_copy_ladder() {
        use crate::static_abilities::StaticAbilityId::{FirstStrike, Flying, Vigilance};
        use ironsmith_core::StaticAbilityVariantSelector::Any;

        let mut source_exiled = ObjectFilter::default()
            .in_zone(Zone::Exile)
            .with_type(CardType::Creature);
        source_exiled.set_explicit_card_noun(true);
        source_exiled
            .tagged_constraints
            .push(crate::filter::TaggedObjectConstraint {
                tag: crate::tag::SOURCE_EXILED_TAG.into(),
                relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
            });
        let mut flying_condition = source_exiled.clone();
        flying_condition.static_abilities.push(Flying);
        let leading = modeled_keyword_grant(
            Flying,
            "flying",
            count_condition(
                flying_condition,
                "there is a creature card exiled with this creature with flying",
            ),
        );
        let copy = ironsmith_core::CopyStaticAbilityVariants::new(
            source_exiled,
            vec![Any(FirstStrike), Any(Vigilance)],
            "As long as there is a creature card exiled with this creature with first strike, this creature has first strike. The same is true for vigilance.",
        );
        let copy_model: crate::static_abilities::CompiledStaticAbility =
            ironsmith_core::StaticAbility::new(copy);
        let variants = Ability::static_ability(crate::static_abilities::StaticAbility::from_model(
            copy_model,
        ));

        assert_eq!(
            describe_structural_delve_keyword_variant_ladder(
                &[leading.clone(), variants.clone()],
                "this creature",
                true,
            ),
            Some((
                "If a creature card with flying was exiled with this creature's delve ability, this creature has flying. The same is true for first strike and vigilance"
                    .to_string(),
                2,
            )),
        );
        assert_eq!(
            describe_structural_delve_keyword_variant_ladder(
                &[leading, variants],
                "this creature",
                false,
            ),
            None,
            "the same structural pair must not invent delve provenance on a source without delve",
        );
    }

    #[test]
    fn concatenated_or_mismatched_keyword_conditions_are_not_hidden() {
        use crate::static_abilities::StaticAbilityId::{Flying, Haste, Vigilance};

        let flying = count_condition(
            keyword_condition_filter(Flying),
            "there is a card exiled with it with flying",
        );
        let malformed = Condition::And(
            Box::new(flying),
            Box::new(count_condition(
                keyword_condition_filter(Haste),
                "there is a card exiled with it with haste",
            )),
        );
        let abilities = vec![
            modeled_keyword_grant(Flying, "flying", malformed),
            modeled_keyword_grant(
                Haste,
                "haste",
                count_condition(
                    keyword_condition_filter(Haste),
                    "there is a card exiled with it with haste",
                ),
            ),
            modeled_keyword_grant(
                Vigilance,
                "vigilance",
                count_condition(
                    keyword_condition_filter(Vigilance),
                    "there is a card exiled with it with vigilance",
                ),
            ),
        ];
        assert_eq!(
            describe_structural_keyword_same_is_true_ladder(&abilities, "this creature"),
            None,
        );

        let wrong_filter = vec![
            modeled_keyword_grant(
                Flying,
                "flying",
                count_condition(
                    keyword_condition_filter(Haste),
                    "there is a card exiled with it with flying",
                ),
            ),
            abilities[1].clone(),
            abilities[2].clone(),
        ];
        assert_eq!(
            describe_structural_keyword_same_is_true_ladder(&wrong_filter, "this creature"),
            None,
        );
    }

    #[test]
    fn full_five_basic_land_modifier_ladder_compacts_to_one_sentence() {
        use crate::static_abilities::StaticAbilityId::{FirstStrike, Flying, Trample};

        let abilities = vec![
            modeled_source_anthem(0, 2, basic_land_condition(Subtype::Plains, "a plains")),
            modeled_keyword_grant(
                Flying,
                "flying",
                basic_land_condition(Subtype::Island, "an island"),
            ),
            modeled_source_anthem(2, 0, basic_land_condition(Subtype::Swamp, "a swamp")),
            modeled_keyword_grant(
                FirstStrike,
                "first strike",
                basic_land_condition(Subtype::Mountain, "a mountain"),
            ),
            modeled_keyword_grant(
                Trample,
                "trample",
                basic_land_condition(Subtype::Forest, "a forest"),
            ),
        ];

        assert_eq!(
            describe_structural_five_basic_land_modifier_ladder(&abilities, "this creature"),
            Some((
                "This creature gets +0/+2 as long as you control a plains, has flying as long as you control an island, gets +2/+0 as long as you control a swamp, has first strike as long as you control a mountain, and has trample as long as you control a forest"
                    .to_string(),
                5,
            )),
        );
    }

    #[test]
    fn incomplete_basic_land_modifier_ladder_stays_explicit() {
        let abilities = vec![modeled_source_anthem(
            0,
            2,
            basic_land_condition(Subtype::Plains, "a plains"),
        )];
        assert_eq!(
            describe_structural_five_basic_land_modifier_ladder(&abilities, "this creature"),
            None,
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
    fn attached_pt_and_ability_loss_static_pieces_recombine() {
        for (power, expected) in [
            (-5, "Enchanted creature gets -5/-0 and loses all abilities"),
            (-3, "Enchanted creature gets -3/-0 and loses all abilities"),
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

    #[test]
    fn attached_pt_and_set_color_static_pieces_recombine() {
        let filter = enchanted_creature_filter();
        let abilities = vec![
            Ability::static_ability(crate::static_abilities::StaticAbility::anthem(
                filter.clone(),
                3,
                1,
            )),
            Ability::static_ability(crate::static_abilities::StaticAbility::set_colors(
                filter,
                crate::color::ColorSet::BLACK,
            )),
        ];

        assert_eq!(
            describe_structural_anthem_set_colors_bundle(&abilities),
            Some(("Enchanted creature gets +3/+1 and is black".to_string(), 2,)),
        );
    }

    #[test]
    fn attached_characteristic_bundles_require_identical_filters() {
        let filter = enchanted_creature_filter();
        let other_filter = ObjectFilter::creature().in_zone(Zone::Battlefield);
        let anthem =
            Ability::static_ability(crate::static_abilities::StaticAbility::anthem(filter, 3, 1));
        let remove = Ability::static_ability(
            crate::static_abilities::StaticAbility::remove_all_abilities(other_filter.clone()),
        );
        let color = Ability::static_ability(crate::static_abilities::StaticAbility::set_colors(
            other_filter,
            crate::color::ColorSet::BLACK,
        ));

        assert_eq!(
            describe_structural_anthem_remove_all_abilities_bundle(&[anthem.clone(), remove,]),
            None,
        );
        assert_eq!(
            describe_structural_anthem_set_colors_bundle(&[anthem, color]),
            None,
        );
    }
}

#[cfg(test)]
mod anthem_color_subtype_boundary_tests {
    use super::*;

    fn modeled(model: crate::static_abilities::CompiledStaticAbility) -> Ability {
        Ability::static_ability(crate::static_abilities::StaticAbility::from_model(model))
    }

    fn goblin_filter() -> ObjectFilter {
        ObjectFilter::default()
            .in_zone(Zone::Battlefield)
            .with_subtype(Subtype::Goblin)
    }

    #[test]
    fn typed_anthem_color_and_subtype_pieces_keep_authored_sentence_boundary() {
        let filter = goblin_filter();
        let anthem: crate::static_abilities::CompiledStaticAbility =
            ironsmith_core::StaticAbility::new(
                ironsmith_core::Anthem::new(filter.clone(), 1, 1)
                    .with_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::All)),
            );
        let colors: crate::static_abilities::CompiledStaticAbility =
            ironsmith_core::StaticAbility::set_colors(
                filter.clone(),
                crate::color::ColorSet::BLACK,
            );
        let subtypes: crate::static_abilities::CompiledStaticAbility =
            ironsmith_core::StaticAbility::add_subtypes(filter, vec![Subtype::Zombie]);
        let abilities = vec![modeled(anthem), modeled(colors), modeled(subtypes)];

        assert_eq!(
            describe_structural_anthem_then_color_subtype_bundle(&abilities),
            Some((
                vec![
                    "All Goblins get +1/+1".to_string(),
                    "All Goblins are black and are Zombies in addition to their other creature types"
                        .to_string(),
                ],
                3,
            )),
        );
    }

    #[test]
    fn differing_typed_filters_do_not_merge_across_authored_clauses() {
        let goblins = goblin_filter();
        let zombies = ObjectFilter::default()
            .in_zone(Zone::Battlefield)
            .with_subtype(Subtype::Zombie);
        let anthem: crate::static_abilities::CompiledStaticAbility =
            ironsmith_core::StaticAbility::new(
                ironsmith_core::Anthem::new(goblins.clone(), 1, 1)
                    .with_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::All)),
            );
        let colors: crate::static_abilities::CompiledStaticAbility =
            ironsmith_core::StaticAbility::set_colors(goblins, crate::color::ColorSet::BLACK);
        let subtypes: crate::static_abilities::CompiledStaticAbility =
            ironsmith_core::StaticAbility::add_subtypes(zombies, vec![Subtype::Zombie]);

        assert_eq!(
            describe_structural_anthem_then_color_subtype_bundle(&[
                modeled(anthem),
                modeled(colors),
                modeled(subtypes),
            ]),
            None,
        );
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

    fn ignore_restriction_special_action() -> Ability {
        let model: crate::static_abilities::CompiledStaticAbility =
            ironsmith_core::StaticAbility::attached_controller_may_sacrifice_permanent_to_ignore_source_effect_until_end_of_turn(
                "That creature's controller may sacrifice a permanent of their choice for that player to ignore this effect until end of turn",
            );
        Ability::static_ability(crate::static_abilities::StaticAbility::from_model(model))
    }

    #[test]
    fn exact_attached_restrictions_and_ignore_special_action_recombine() {
        let mut abilities = restriction_pair("Enchanted creature");
        abilities.push(ignore_restriction_special_action());
        let expected = "Enchanted creature can't attack or block, and its activated abilities can't be activated. That creature's controller may sacrifice a permanent of their choice for that player to ignore this effect until end of turn";
        assert_eq!(
            describe_structural_enchanted_combat_activation_restriction_bundle(&abilities),
            Some((expected.to_string(), 3)),
        );

        let mut wrong_zone = abilities;
        wrong_zone[2].functional_zones = vec![Zone::Hand];
        assert_eq!(
            describe_structural_enchanted_combat_activation_restriction_bundle(&wrong_zone),
            Some((
                "Enchanted creature can't attack or block, and its activated abilities can't be activated"
                    .to_string(),
                2,
            )),
            "an inactive-zone marker must remain separate rather than being folded"
        );
    }

    #[test]
    fn exact_ability_loss_and_untap_restriction_recombine() {
        let filter = enchanted_filter("Enchanted permanent");
        let abilities = vec![
            Ability::static_ability(
                crate::static_abilities::StaticAbility::remove_all_abilities(filter.clone()),
            ),
            modeled_rule_ability(
                crate::effect::Restriction::untap(filter),
                "enchanted permanent doesn't untap during its controller's untap step",
            ),
        ];
        let expected = "Enchanted permanent loses all abilities and doesn't untap during its controller's untap step";
        assert_eq!(
            describe_structural_remove_all_abilities_untap_bundle(&abilities),
            Some((expected.to_string(), 2)),
        );

        let mut builder = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Restriction Surface Probe",
        )
        .card_types(vec![CardType::Enchantment]);
        for ability in abilities {
            builder = builder.with_ability(ability);
        }
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&builder.build()),
            vec![format!("{expected}.")],
        );
    }

    #[test]
    fn exact_shared_subject_block_and_attack_restrictions_recombine() {
        let filter = ObjectFilter::default()
            .with_subtype(Subtype::Cleric)
            .controlled_by(PlayerFilter::Opponent)
            .in_zone(Zone::Battlefield);
        let abilities = vec![
            modeled_rule_ability(
                crate::effect::Restriction::block(filter.clone()),
                "clerics your opponents control can't block",
            ),
            modeled_rule_ability(
                crate::effect::Restriction::attack_player_or_planeswalkers_controlled_by(
                    filter.clone(),
                    PlayerFilter::You,
                ),
                "clerics your opponents control can't attack you or planeswalkers you control",
            ),
        ];
        let expected = "Clerics your opponents control can't block, and they can't attack you or planeswalkers you control";
        assert_eq!(
            describe_structural_shared_subject_block_attack_bundle(&abilities),
            Some((expected.to_string(), 2)),
        );

        let mut builder = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Restriction Surface Probe",
        )
        .card_types(vec![CardType::Creature]);
        for ability in abilities.clone() {
            builder = builder.with_ability(ability);
        }
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&builder.build()),
            vec![format!("{expected}.")],
        );

        let mut different_subject = abilities;
        different_subject[1] = modeled_rule_ability(
            crate::effect::Restriction::attack_player_or_planeswalkers_controlled_by(
                ObjectFilter::default()
                    .with_subtype(Subtype::Rogue)
                    .controlled_by(PlayerFilter::Opponent)
                    .in_zone(Zone::Battlefield),
                PlayerFilter::You,
            ),
            "rogues your opponents control can't attack you or planeswalkers you control",
        );
        assert_eq!(
            describe_structural_shared_subject_block_attack_bundle(&different_subject),
            None,
        );
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

        let creature = enchanted_filter("Enchanted creature");
        let block_and_activation = vec![
            modeled_rule_ability(
                crate::effect::Restriction::block(creature.clone()),
                "enchanted creature can't block",
            ),
            modeled_rule_ability(
                crate::effect::Restriction::activate_abilities_of(creature),
                "enchanted creature activated abilities can't be activated",
            ),
        ];
        assert_eq!(
            describe_structural_enchanted_combat_activation_restriction_bundle(
                &block_and_activation,
            ),
            Some((
                "Enchanted creature can't block, and its activated abilities can't be activated"
                    .to_string(),
                2,
            )),
        );

        let permanent = enchanted_filter("Enchanted permanent");
        let untap_and_activation = vec![
            modeled_rule_ability(
                crate::effect::Restriction::untap(permanent.clone()),
                "enchanted permanent doesn't untap during its controller's untap step",
            ),
            modeled_rule_ability(
                crate::effect::Restriction::activate_abilities_of(permanent),
                "enchanted permanent activated abilities can't be activated",
            ),
        ];
        assert_eq!(
            describe_structural_enchanted_combat_activation_restriction_bundle(
                &untap_and_activation,
            ),
            Some((
                "Enchanted permanent doesn't untap during its controller's untap step and its activated abilities can't be activated"
                    .to_string(),
                2,
            )),
        );
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
    fn typed_aura_attachment_is_removed_only_from_the_rendering_program() {
        let attachment = crate::object::AuraAttachmentFilter::Object(
            ObjectFilter::creature().in_zone(Zone::Battlefield),
        );
        let executable = crate::resolution::ResolutionProgram::from_effects(vec![
            Effect::attach_to(attachment.target_spec()),
            Effect::draw(1),
        ]);

        let rendered = spell_program_without_encoded_aura_attachment(&executable, &attachment);
        assert_eq!(executable.flattened_default_effects().len(), 2);
        assert_eq!(rendered.flattened_default_effects().len(), 1);
        assert!(
            rendered.flattened_default_effects()[0]
                .downcast_ref::<crate::effects::DrawCardsEffect>()
                .is_some()
        );

        let land_attachment = crate::object::AuraAttachmentFilter::Object(
            ObjectFilter::land().in_zone(Zone::Battlefield),
        );
        let untouched =
            spell_program_without_encoded_aura_attachment(&executable, &land_attachment);
        assert_eq!(untouched.flattened_default_effects().len(), 2);
    }

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

        let mut creature_or_planeswalker = ObjectFilter::default().in_zone(Zone::Battlefield);
        creature_or_planeswalker.card_types = vec![CardType::Creature, CardType::Planeswalker];
        let creature_or_planeswalker =
            crate::object::AuraAttachmentFilter::Object(creature_or_planeswalker);
        assert_eq!(
            rewrite_plain_creature_aura_subject(
                Some(&creature_or_planeswalker),
                "When this Aura enters, tap enchanted creature and investigate"
            ),
            "When this Aura enters, tap enchanted permanent and investigate"
        );

        let mut artifact_creature_or_planeswalker =
            ObjectFilter::default().in_zone(Zone::Battlefield);
        artifact_creature_or_planeswalker.card_types = vec![
            CardType::Artifact,
            CardType::Creature,
            CardType::Planeswalker,
        ];
        let artifact_creature_or_planeswalker =
            crate::object::AuraAttachmentFilter::Object(artifact_creature_or_planeswalker);
        assert_eq!(
            rewrite_plain_creature_aura_subject(
                Some(&artifact_creature_or_planeswalker),
                "When this Aura enters, tap enchanted creature"
            ),
            "When this Aura enters, tap enchanted permanent"
        );
    }

    #[test]
    fn attached_tap_then_equipment_unattach_uses_one_typed_referent() {
        let enchanted = crate::TagKey::from("enchanted");
        let first = crate::resolution::ResolutionSegment {
            default_effects: vec![
                Effect::new(crate::effects::TagAttachedToSourceEffect::new(
                    enchanted.clone(),
                )),
                Effect::new(crate::effects::TapEffect::with_spec(ChooseSpec::Tagged(
                    enchanted.clone(),
                )))
                .tag("tapped_0"),
            ],
            self_replacements: vec![],
            starts_new_source_line: false,
        };
        let conditional = Effect::conditional_only(
            crate::effect::Condition::TaggedObjectMatches(
                enchanted.clone(),
                ObjectFilter::default().with_subtype(Subtype::Equipment),
            ),
            vec![Effect::new(crate::effects::UnattachObjectsEffect::new(
                ChooseSpec::Tagged(enchanted),
            ))],
        );
        let second = crate::resolution::ResolutionSegment {
            default_effects: vec![conditional],
            self_replacements: vec![],
            starts_new_source_line: false,
        };
        let program = crate::resolution::ResolutionProgram::new(vec![first, second]);

        assert_eq!(
            describe_resolution_program(&program),
            "Tap enchanted creature. If it's an Equipment, unattach it"
        );
    }
}

#[cfg(test)]
mod keyword_maximum_blocker_bundle_tests {
    use super::*;

    fn parsed_equipment_abilities(text: &str) -> Vec<Ability> {
        crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Keyword Maximum Blocker Probe",
        )
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .parse_text(text)
        .expect("compound Equipment grant should parse")
        .abilities
    }

    #[test]
    fn matching_typed_grants_recombine_without_a_source_restriction() {
        let abilities = parsed_equipment_abilities(
            "Equipped creature has trample and can't be blocked by more than one creature.",
        );

        assert_eq!(
            describe_structural_keyword_maximum_blocker_bundle(&abilities),
            Some((
                "Equipped creature has trample and can't be blocked by more than one creature"
                    .to_string(),
                2,
            ))
        );
        assert_eq!(abilities.len(), 2);
    }

    #[test]
    fn an_unrelated_followup_is_not_folded_into_the_grant() {
        let mut abilities = parsed_equipment_abilities(
            "Equipped creature has trample and can't be blocked by more than one creature.",
        );
        abilities.swap(0, 1);

        assert_eq!(
            describe_structural_keyword_maximum_blocker_bundle(&abilities),
            None
        );
    }
}

#[cfg(test)]
mod threshold_source_modifier_bundle_tests {
    use super::*;

    fn threshold_abilities() -> Vec<Ability> {
        crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Threshold Source Modifier Probe",
        )
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Threshold — As long as there are seven or more cards in your graveyard, this creature gets +1/+1, is black, and has \"{2}{B}, {T}: Destroy target blue creature.\"",
        )
        .expect("threshold source modifier should parse")
        .abilities
    }

    fn mutate_model(
        ability: &mut Ability,
        mutate: impl FnOnce(&mut crate::static_abilities::CompiledStaticAbility),
    ) {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            panic!("expected static ability");
        };
        let mut model = static_ability
            .compiled_model()
            .expect("compiled static model")
            .clone();
        mutate(&mut model);
        ability.kind =
            AbilityKind::Static(crate::static_abilities::StaticAbility::from_model(model));
    }

    #[test]
    fn exact_typed_threshold_siblings_recombine() {
        let abilities = threshold_abilities();
        assert_eq!(
            describe_structural_threshold_source_modifier_bundle(&abilities, "this creature"),
            Some((
                "Threshold — As long as there are seven or more cards in your graveyard, this creature gets +1/+1, is black, and has \"{2}{B}, {T}: Destroy target blue creature.\""
                    .to_string(),
                3,
            )),
        );
    }

    #[test]
    fn near_miss_threshold_siblings_stay_separate() {
        let valid = threshold_abilities();

        let mut wrong_anthem = valid.clone();
        mutate_model(&mut wrong_anthem[0], |model| {
            let ironsmith_core::StaticAbilityPayload::Anthem(anthem) = &mut model.payload else {
                panic!("expected anthem payload");
            };
            anthem.power = ironsmith_core::AnthemValue::Fixed(2);
        });
        assert_eq!(
            describe_structural_threshold_source_modifier_bundle(&wrong_anthem, "this creature"),
            None,
        );

        let mut wrong_condition = valid.clone();
        mutate_model(&mut wrong_condition[1], |model| {
            let ironsmith_core::StaticAbilityPayload::Conditional { condition, .. } =
                &mut model.payload
            else {
                panic!("expected conditional color payload");
            };
            let Condition::ValueComparison { right, .. } = condition else {
                panic!("expected threshold comparison");
            };
            *right = Value::Fixed(8);
        });
        assert_eq!(
            describe_structural_threshold_source_modifier_bundle(&wrong_condition, "this creature"),
            None,
        );

        let mut wrong_color = valid.clone();
        mutate_model(&mut wrong_color[1], |model| {
            let ironsmith_core::StaticAbilityPayload::Conditional { ability, .. } =
                &mut model.payload
            else {
                panic!("expected conditional color payload");
            };
            let ironsmith_core::StaticAbilityPayload::SetColors { colors, .. } =
                &mut ability.payload
            else {
                panic!("expected set-colors payload");
            };
            *colors = crate::color::ColorSet::RED;
        });
        assert_eq!(
            describe_structural_threshold_source_modifier_bundle(&wrong_color, "this creature"),
            None,
        );

        let mut wrong_filter = valid.clone();
        mutate_model(&mut wrong_filter[2], |model| {
            let ironsmith_core::StaticAbilityPayload::GrantObjectAbilityForFilter(grant) =
                &mut model.payload
            else {
                panic!("expected object-ability grant payload");
            };
            grant.filter = ObjectFilter::creature();
        });
        assert_eq!(
            describe_structural_threshold_source_modifier_bundle(&wrong_filter, "this creature"),
            None,
        );

        let mut multiple_grants = valid.clone();
        mutate_model(&mut multiple_grants[2], |model| {
            let ironsmith_core::StaticAbilityPayload::GrantObjectAbilityForFilter(grant) =
                &mut model.payload
            else {
                panic!("expected object-ability grant payload");
            };
            let additional = grant.ability.clone();
            grant.additional_abilities.push(additional);
        });
        assert_eq!(
            describe_structural_threshold_source_modifier_bundle(&multiple_grants, "this creature"),
            None,
        );

        let mut wrong_zone = valid;
        wrong_zone[1].functional_zones = vec![Zone::Hand];
        assert_eq!(
            describe_structural_threshold_source_modifier_bundle(&wrong_zone, "this creature"),
            None,
        );
    }
}

#[cfg(test)]
mod conditioned_source_anthem_keyword_bundle_tests {
    use super::*;

    fn conditioned_source_modifier_abilities() -> Vec<Ability> {
        crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Conditioned Source Modifier Probe",
        )
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature gets +1/+0 and has first strike as long as there are four or more card types among cards in your graveyard.",
        )
        .expect("conditioned source modifier should parse")
        .abilities
    }

    fn mutate_model(
        ability: &mut Ability,
        mutate: impl FnOnce(&mut crate::static_abilities::CompiledStaticAbility),
    ) {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            panic!("expected static ability");
        };
        let mut model = static_ability
            .compiled_model()
            .expect("compiled static model")
            .clone();
        mutate(&mut model);
        ability.kind =
            AbilityKind::Static(crate::static_abilities::StaticAbility::from_model(model));
    }

    #[test]
    fn matching_typed_source_modifier_siblings_recombine() {
        let abilities = conditioned_source_modifier_abilities();
        assert_eq!(abilities.len(), 2);
        assert_eq!(
            describe_structural_conditioned_source_anthem_keyword_bundle(
                &abilities,
                "this creature",
            ),
            Some((
                "this creature gets +1/+0 and has first strike as long as there are four or more card types among cards in your graveyard"
                    .to_string(),
                2,
            )),
        );
    }

    #[test]
    fn delirium_three_way_source_modifier_recombines() {
        let definition = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Delirium Source Modifier Probe",
        )
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Delirium — As long as there are four or more card types among cards in your graveyard, this creature gets +2/+2, has flying, and attacks each combat if able.",
        )
        .expect("three-way delirium source modifier should parse");

        assert_eq!(
            describe_structural_conditioned_source_anthem_keyword_bundle(
                &definition.abilities,
                "this creature",
            ),
            Some((
                "Delirium — As long as there are four or more card types among cards in your graveyard, this creature gets +2/+2, has flying, and attacks each combat if able"
                    .to_string(),
                3,
            )),
        );
    }

    #[test]
    fn near_miss_source_modifier_siblings_stay_separate() {
        let valid = conditioned_source_modifier_abilities();

        let mut wrong_condition = valid.clone();
        mutate_model(&mut wrong_condition[1], |model| {
            let ironsmith_core::StaticAbilityPayload::GrantObjectAbilityForFilter(grant) =
                &mut model.payload
            else {
                panic!("expected grant payload");
            };
            let Some(Condition::PlayerHasCardTypesInGraveyardOrMore { count, .. }) =
                &mut grant.condition
            else {
                panic!("expected graveyard card-type condition");
            };
            *count = 5;
        });
        assert_eq!(
            describe_structural_conditioned_source_anthem_keyword_bundle(
                &wrong_condition,
                "this creature",
            ),
            None,
        );

        let mut wrong_filter = valid.clone();
        mutate_model(&mut wrong_filter[0], |model| {
            let ironsmith_core::StaticAbilityPayload::Anthem(anthem) = &mut model.payload else {
                panic!("expected anthem payload");
            };
            anthem.filter = Some(ObjectFilter::creature());
        });
        assert_eq!(
            describe_structural_conditioned_source_anthem_keyword_bundle(
                &wrong_filter,
                "this creature",
            ),
            None,
        );

        let mut multiple_grants = valid.clone();
        mutate_model(&mut multiple_grants[1], |model| {
            let ironsmith_core::StaticAbilityPayload::GrantObjectAbilityForFilter(grant) =
                &mut model.payload
            else {
                panic!("expected grant payload");
            };
            grant.additional_abilities.push(grant.ability.clone());
        });
        assert_eq!(
            describe_structural_conditioned_source_anthem_keyword_bundle(
                &multiple_grants,
                "this creature",
            ),
            None,
        );

        let mut wrong_zone = valid;
        wrong_zone[1].functional_zones = vec![Zone::Hand];
        assert_eq!(
            describe_structural_conditioned_source_anthem_keyword_bundle(
                &wrong_zone,
                "this creature",
            ),
            None,
        );
    }
}

#[cfg(test)]
mod conditioned_source_animation_annihilator_bundle_tests {
    use super::*;

    fn animation_definition() -> CardDefinition {
        crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Conditioned Source Animation Probe",
        )
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "As long as this artifact has eight or more +1/+1 counters on it, it's a 0/0 creature in addition to its other types and it has annihilator 2.",
        )
        .expect("conditioned source animation should parse")
    }

    #[test]
    fn matching_animation_and_annihilator_siblings_recombine() {
        let definition = animation_definition();
        assert_eq!(
            describe_structural_conditioned_source_animation_annihilator_bundle(
                &definition.abilities,
            ),
            Some((
                "As long as this artifact has eight or more +1/+1 counters on it, it's a 0/0 creature in addition to its other types and it has annihilator 2"
                    .to_string(),
                3,
            )),
        );
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition),
            vec![
                "As long as this artifact has eight or more +1/+1 counters on it, it's a 0/0 creature in addition to its other types and it has annihilator 2."
                    .to_string(),
            ],
        );
    }

    #[test]
    fn mismatched_grant_condition_does_not_recombine() {
        let mut definition = animation_definition();
        let AbilityKind::Static(grant_static) = &definition.abilities[2].kind else {
            panic!("expected annihilator grant");
        };
        let mut model = grant_static
            .compiled_model()
            .expect("compiled grant model")
            .clone();
        let ironsmith_core::StaticAbilityPayload::GrantObjectAbilityForFilter(grant) =
            &mut model.payload
        else {
            panic!("expected typed grant payload");
        };
        grant.condition = Some(Condition::YourTurn);
        definition.abilities[2].kind =
            AbilityKind::Static(crate::static_abilities::StaticAbility::from_model(model));

        assert_eq!(
            describe_structural_conditioned_source_animation_annihilator_bundle(
                &definition.abilities,
            ),
            None,
        );
    }
}

#[cfg(test)]
mod target_player_life_loss_then_gain_and_create_tests {
    use super::*;

    #[test]
    fn coordinated_gain_and_token_sentence_retains_actor_and_boundaries() {
        let definition = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Life And Token Probe",
        )
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Target player loses 3 life. You gain 3 life and create three 0/1 colorless Eldrazi Spawn creature tokens. They have \"Sacrifice this token: Add {C}.\"",
        )
        .expect("life-and-token spell should parse");
        let program = definition
            .spell_effect
            .as_ref()
            .expect("spell resolution program");

        assert_eq!(
            describe_target_player_life_loss_then_gain_and_create_program(program),
            Some(
                "Target player loses 3 life. You gain 3 life and create three 0/1 colorless Eldrazi Spawn creature tokens. They have \"Sacrifice this token: Add {C}.\""
                    .to_string(),
            ),
        );
        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition),
            vec![
                "Target player loses 3 life. You gain 3 life and create three 0/1 colorless Eldrazi Spawn creature tokens. They have \"Sacrifice this token: Add {C}.\""
                    .to_string(),
            ],
        );
    }
}

#[cfg(test)]
mod shared_x_cross_segment_exile_permission_tests {
    use super::*;

    #[test]
    fn reduction_and_exile_top_share_one_where_x_basis_across_source_sentences() {
        let definition = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Shared X Exile Permission Probe",
        )
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever you attack, spells you cast from exile this turn cost {X} less to cast, where X is the number of players being attacked. Exile the top X cards of your library. Until end of turn, you may cast spells from among those exiled cards.",
        )
        .expect("shared-X exile permission trigger should parse");

        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition),
            vec![
                "Whenever you attack, spells you cast from exile this turn cost {X} less to cast, where X is the number of players being attacked. Exile the top X cards of your library. Until end of turn, you may cast spells from among those exiled cards."
                    .to_string(),
            ],
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

#[cfg(test)]
mod counter_removal_prevention_surface_tests {
    use super::*;

    fn prevention_ability(
        amount: Value,
        follow_up: Option<ironsmith_core::CounterRemovalFollowUp>,
    ) -> Ability {
        let model: crate::static_abilities::CompiledStaticAbility =
            ironsmith_core::StaticAbility::prevent_damage_to_self_remove_counter_with_follow_up(
                CounterType::PlusOnePlusOne,
                amount,
                follow_up,
            )
            .with_condition(Condition::CountComparison {
                count: ironsmith_core::AnthemCountExpression::MatchingFilter(
                    ObjectFilter::source().with_counter_type(CounterType::PlusOnePlusOne),
                ),
                comparison: Comparison::GreaterThanOrEqual(1),
                display: Some("it has a +1/+1 counter on it".to_string()),
            });
        Ability::static_ability(crate::static_abilities::StaticAbility::from_model(model))
    }

    #[test]
    fn five_card_family_renders_inline_replacement_surface() {
        let fixed = "If damage would be dealt to this creature while it has a +1/+1 counter on it, prevent that damage and remove a +1/+1 counter from it.";
        let dynamic = "If damage would be dealt to this creature while it has a +1/+1 counter on it, prevent that damage and remove that many +1/+1 counters from it.";
        let bloatfly = "If damage would be dealt to this creature while it has a +1/+1 counter on it, prevent that damage, remove that many +1/+1 counters from it, then give each player a rad counter for each +1/+1 counter removed this way.";
        let cases = [
            ("Oathsworn Knight", Value::Fixed(1), None, fixed),
            ("Undergrowth Champion", Value::Fixed(1), None, fixed),
            (
                "Ugin's Conjurant",
                Value::EventValue(EventValueSpec::Amount),
                None,
                dynamic,
            ),
            (
                "Magma Pummeler",
                Value::EventValue(EventValueSpec::Amount),
                None,
                dynamic,
            ),
            (
                "Bloatfly Swarm",
                Value::EventValue(EventValueSpec::Amount),
                Some(
                    ironsmith_core::CounterRemovalFollowUp::EachPlayerGetsCounters {
                        counter_type: CounterType::Rad,
                        counters_per_removed: 1,
                    },
                ),
                bloatfly,
            ),
        ];

        for (name, amount, follow_up, expected) in cases {
            let definition =
                crate::cards::builders::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
                    .card_types(vec![CardType::Creature])
                    .with_ability(prevention_ability(amount, follow_up))
                    .build();
            assert_eq!(
                crate::compiled_text::compiled_text_lines(&definition),
                vec![expected.to_string()],
                "{name}"
            );
        }
    }

    #[test]
    fn near_miss_condition_and_bare_payload_do_not_get_inline_surface() {
        let mut mismatched = prevention_ability(Value::Fixed(1), None);
        let AbilityKind::Static(static_ability) = &mut mismatched.kind else {
            unreachable!();
        };
        let mut model = static_ability
            .compiled_model()
            .expect("compiled model")
            .clone();
        let ironsmith_core::StaticAbilityPayload::Conditional { condition, .. } =
            &mut model.payload
        else {
            unreachable!();
        };
        *condition = Condition::CountComparison {
            count: ironsmith_core::AnthemCountExpression::MatchingFilter(
                ObjectFilter::source().with_counter_type(CounterType::Shield),
            ),
            comparison: Comparison::GreaterThanOrEqual(1),
            display: Some("it has a shield counter on it".to_string()),
        };
        mismatched.kind =
            AbilityKind::Static(crate::static_abilities::StaticAbility::from_model(model));
        assert_eq!(
            describe_structural_counter_removal_damage_prevention(&mismatched, "this creature"),
            None
        );

        let mut non_source = prevention_ability(Value::Fixed(1), None);
        let AbilityKind::Static(static_ability) = &mut non_source.kind else {
            unreachable!();
        };
        let mut model = static_ability
            .compiled_model()
            .expect("compiled model")
            .clone();
        let ironsmith_core::StaticAbilityPayload::Conditional { condition, .. } =
            &mut model.payload
        else {
            unreachable!();
        };
        *condition = Condition::CountComparison {
            count: ironsmith_core::AnthemCountExpression::MatchingFilter(
                ObjectFilter::creature().with_counter_type(CounterType::PlusOnePlusOne),
            ),
            comparison: Comparison::GreaterThanOrEqual(1),
            display: Some("a creature has a +1/+1 counter on it".to_string()),
        };
        non_source.kind =
            AbilityKind::Static(crate::static_abilities::StaticAbility::from_model(model));
        assert_eq!(
            describe_structural_counter_removal_damage_prevention(&non_source, "this creature"),
            None
        );

        let bare_model: crate::static_abilities::CompiledStaticAbility =
            ironsmith_core::StaticAbility::prevent_damage_to_self_remove_counter(
                CounterType::PlusOnePlusOne,
                1,
            );
        let bare = Ability::static_ability(crate::static_abilities::StaticAbility::from_model(
            bare_model,
        ));
        assert_eq!(
            describe_structural_counter_removal_damage_prevention(&bare, "this creature"),
            None
        );
    }

    fn removed_this_way_damage_ability(triggering_tag: &str, damage_source_tag: &str) -> Ability {
        let trigger = crate::triggers::Trigger::new(
            crate::triggers::CounterRemovedFromTrigger::new(ObjectFilter::source_with_surface(
                crate::target::SourceReferenceSurface::ThisPermanentType(
                    "this creature".to_string(),
                ),
            ))
            .one_or_more()
            .caused_by_source(),
        )
        .with_intro_surface(crate::triggers::TriggerIntroSurface::When);
        let amount = Value::EventValue(EventValueSpec::Amount)
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::CountersRemovedThisWay);
        let effects = vec![
            Effect::tag_triggering_object(TagKey::from(triggering_tag)),
            Effect::new(crate::effects::ExecuteWithSourceEffect::new(
                ChooseSpec::Tagged(TagKey::from(damage_source_tag)),
                Effect::deal_damage(amount, ChooseSpec::AnyTarget),
            )),
        ];
        let mut ability = Ability::triggered(trigger, effects);
        let AbilityKind::Triggered(triggered) = &mut ability.kind else {
            unreachable!();
        };
        triggered.choices = vec![ChooseSpec::AnyTarget];
        ability
    }

    #[test]
    fn removed_this_way_trigger_renders_event_amount_damage_to_any_target() {
        let ability = removed_this_way_damage_ability("triggering", "triggering");
        let definition = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Magma Pummeler Surface Probe",
        )
        .card_types(vec![CardType::Creature])
        .with_ability(ability)
        .build();

        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition),
            vec!["When one or more counters are removed from this creature this way, it deals that much damage to any target.".to_string()]
        );
    }

    #[test]
    fn removed_this_way_trigger_requires_the_tagged_triggering_object_as_damage_source() {
        let mismatched = removed_this_way_damage_ability("triggering", "other");
        assert_eq!(
            describe_structural_counter_removed_this_way_damage_trigger(&mismatched),
            None
        );
    }
}

#[cfg(test)]
mod flashback_non_mana_cost_surface_tests {
    use super::*;

    fn sacrifice_creature_cost() -> crate::costs::Cost {
        crate::costs::Cost::sacrifice(crate::filter::ObjectFilter::creature().you_control())
    }

    #[test]
    fn effect_only_flashback_omits_invented_zero_mana_cost() {
        let effect_only = AlternativeCastingMethod::Flashback {
            total_cost: crate::cost::TotalCost::from_cost(sacrifice_creature_cost()),
        };
        assert_eq!(
            describe_alternative_cast_line(&effect_only, 0),
            "Flashback—Sacrifice a creature"
        );
    }

    #[test]
    fn explicit_zero_and_mixed_flashback_costs_keep_their_mana_surface() {
        let explicit_zero = AlternativeCastingMethod::Flashback {
            total_cost: crate::cost::TotalCost::free(),
        };
        assert_eq!(
            describe_alternative_cast_line(&explicit_zero, 0),
            "Flashback—{0}"
        );

        let mixed = AlternativeCastingMethod::Flashback {
            total_cost: crate::cost::TotalCost::from_costs(vec![
                crate::costs::Cost::mana(crate::mana::ManaCost::from_symbols(vec![
                    crate::mana::ManaSymbol::Generic(1),
                ])),
                sacrifice_creature_cost(),
            ]),
        };
        assert_eq!(
            describe_alternative_cast_line(&mixed, 0),
            "Flashback—{1}, Sacrifice a creature"
        );
    }
}

#[cfg(test)]
mod countered_set_cross_segment_surface_tests {
    use super::*;

    fn tagged_counter_each(tag: &TagKey, filter: ObjectFilter) -> Effect {
        Effect::new(crate::effects::ForEachObject::new(
            filter,
            vec![Effect::new(crate::effects::PutCountersEffect::new(
                CounterType::PlusOnePlusOne,
                Value::Fixed(1),
                ChooseSpec::Iterated,
            ))],
        ))
        .tag(tag.clone())
    }

    #[test]
    fn countered_set_untap_back_reference_survives_segment_boundary() {
        let tag = TagKey::from("counters_0");
        let counters = tagged_counter_each(
            &tag,
            ObjectFilter::creature().controlled_by(PlayerFilter::You),
        );
        let untap = Effect::new(crate::effects::UntapEffect::with_spec(ChooseSpec::All(
            ObjectFilter::creature()
                .match_tagged(tag, crate::filter::TaggedOpbjectRelation::IsTaggedObject),
        )));
        let program = crate::resolution::ResolutionProgram::new(vec![
            crate::resolution::ResolutionSegment::from_effects(vec![counters]),
            crate::resolution::ResolutionSegment::from_effects(vec![untap]),
        ]);

        assert_eq!(
            describe_resolution_program(&program),
            "Put a +1/+1 counter on each creature you control. Untap those creatures"
        );
    }

    #[test]
    fn countered_set_count_back_reference_survives_segment_boundary() {
        let tag = TagKey::from("counters_0");
        let counters = tagged_counter_each(
            &tag,
            ObjectFilter::creature().controlled_by(PlayerFilter::You),
        );
        let counted = ObjectFilter::creature()
            .match_tagged(tag, crate::filter::TaggedOpbjectRelation::IsTaggedObject);
        let gain = Effect::new(crate::effects::GainLifeEffect::you(
            Value::Count(counted).with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach),
        ));
        let program = crate::resolution::ResolutionProgram::new(vec![
            crate::resolution::ResolutionSegment::from_effects(vec![counters]),
            crate::resolution::ResolutionSegment::from_effects(vec![gain]),
        ]);

        assert_eq!(
            describe_resolution_program(&program),
            "Put a +1/+1 counter on each creature you control. You gain 1 life for each of those creatures"
        );
    }
}
