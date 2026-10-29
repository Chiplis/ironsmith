#![allow(unused_imports)]

use crate::ability::{Ability, AbilityKind, ActivationTiming};
use crate::alternative_cast::AlternativeCastingMethod;
use crate::effect::{
    ChoiceCount, Comparison, Condition, EffectPredicate, EventValueSpec, Until, Value,
};
use crate::effect_text_shared;
use crate::object::CounterType;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::types::{Subtype, Supertype};
use crate::{CardDefinition, CardType, Effect, ManaSymbol, TagKey, Zone};

mod ast_render;
mod debug_safe;
mod literal_search;
mod merge_passes;
mod normalize_common;
mod audit_readings;
mod audit_ability_rephrasings;
use self::audit_ability_rephrasings::audit_ability_line_rephrasings;
mod audit_residual_d_rephrasings;
pub(crate) use normalize_common::describe_condition;
mod oracle_style;
mod render_effects;
mod surface_helpers;

use self::ast_render::*;
use self::merge_passes::*;
use self::normalize_common::*;
use self::oracle_style::*;
use self::render_effects::*;
use self::surface_helpers::*;

pub(crate) use self::normalize_common::{
    describe_aggregate_filter_value_subject, describe_counter_for_each_basis,
    describe_death_history_subject, describe_party_size_for_each_basis,
    describe_turn_history_for_each_basis, describe_value, party_size_multiplier,
};
pub use self::oracle_style::canonical_compiled_lines;
pub use self::render_effects::compile_effect_list;
pub(crate) use self::render_effects::{describe_effect, describe_for_each_multiplier_and_basis};

pub(crate) fn pluralize_noun_phrase_for_trigger(phrase: &str) -> String {
    self::render_effects::pluralize_noun_phrase(phrase)
}

const CLEAVE_BRACKET_OPEN_SENTINEL: &str = "\u{e000}";
const CLEAVE_BRACKET_CLOSE_SENTINEL: &str = "\u{e001}";
pub(in crate::compiled_text) const STANDARD_REMINDER_OPEN_SENTINEL: &str = "\u{e002}";
pub(in crate::compiled_text) const STANDARD_REMINDER_CLOSE_SENTINEL: &str = "\u{e003}";
pub(in crate::compiled_text) const TYPED_NO_PERIOD_SENTINEL: &str = "\u{e004}";

fn restore_cleave_bracket_surface(line: String) -> String {
    line.replace(CLEAVE_BRACKET_OPEN_SENTINEL, "[")
        .replace(CLEAVE_BRACKET_CLOSE_SENTINEL, "]")
        .replace(STANDARD_REMINDER_OPEN_SENTINEL, "(")
        .replace(&format!("{STANDARD_REMINDER_CLOSE_SENTINEL}."), ")")
        .replace(STANDARD_REMINDER_CLOSE_SENTINEL, ")")
        .replace(&format!("{TYPED_NO_PERIOD_SENTINEL}."), "")
        .replace(TYPED_NO_PERIOD_SENTINEL, "")
}

/// Rendered surface of the object that granted an ability
/// (`ironsmith_core::GRANTING_SOURCE_TAG`). Inside the ability an Equipment
/// or Aura grants, that object is the card itself, so the card-level
/// renderers replace it with the card's name.
pub(crate) const GRANTING_SOURCE_RENDER_SURFACE: &str = "the granting permanent";

/// "When this creature transforms into Awoken Horror": the destination of a
/// self-transform trigger is the face that has the ability, compiled as a
/// self-reference; oracle always names that face.
fn substitute_transform_destination_self_name(line: String, def: &CardDefinition) -> String {
    const MARKER: &str = " transforms into this ";
    let Some(start) = line.find(MARKER) else {
        return line;
    };
    let after = &line[start + MARKER.len()..];
    let noun_len = after
        .find(|ch: char| !ch.is_ascii_alphabetic())
        .unwrap_or(after.len());
    if noun_len == 0 {
        return line;
    }
    format!(
        "{} transforms into {}{}",
        &line[..start],
        def.card.name,
        &after[noun_len..]
    )
}

fn substitute_granting_source_surface(line: String, def: &CardDefinition) -> String {
    let line = substitute_transform_destination_self_name(line, def);
    if !line.contains("granting permanent") {
        return line;
    }
    // A quoted grant renders from its normalized source text, which carries
    // the article-less preprocess surface ("Return granting permanent ...").
    [
        GRANTING_SOURCE_RENDER_SURFACE,
        "The granting permanent",
        "granting permanent",
        "Granting permanent",
    ]
    .into_iter()
    .fold(line, |line, surface| line.replace(surface, &def.card.name))
}

/// Structural renderers use this marker when the modeled effect names its
/// own source, while only the card-level renderer knows the source's name.
pub(super) const SOURCE_CARD_NAME_SURFACE: &str = "__compiled_source_card_name__";

fn debug_compiled_surface_lines(def: &CardDefinition) -> Vec<String> {
    normalize_common::EXCESS_DAMAGE_CREATURE_RECIPIENT.with(|cell| cell.set(false));
    debug_safe::normalize_debug_safe_surface(ast_compiled_lines(def))
        .into_iter()
        .map(debug_safe::DebugSafeLine::into_string)
        .map(|line| substitute_granting_source_surface(line, def))
        .map(|line| line.replace(SOURCE_CARD_NAME_SURFACE, &def.card.name))
        .collect()
}

/// Render the structured runtime model for debug/inspector use.
pub fn debug_compiled_lines(def: &CardDefinition) -> Vec<String> {
    debug_compiled_surface_lines(def)
        .into_iter()
        .filter(|line| !ast_render::is_printed_line_boundary_sentinel(line))
        .map(restore_cleave_bracket_surface)
        .collect()
}

/// Render the structured compiled-text surface used for DB scoring.
/// Semantic-audit rendering: every retained authored predicate or sentence is
/// either read structurally from its executable fields or reported with an
/// explicit `⟦unverified …⟧` marker. Use this, not `compiled_text_lines`, to
/// check what a compiled model means.
pub fn semantic_audit_lines(def: &CardDefinition) -> Vec<String> {
    ironsmith_core::semantic_audit::with_semantic_audit_describing(
        |condition| describe_condition(condition),
        |value| {
            value
                .downcast_ref::<crate::static_abilities::CompiledStaticAbility>()
                .and_then(audit_readings::audit_payload_reading)
                .or_else(|| value.downcast_ref::<Ability>().map(ability_surface_text))
                .or_else(|| value.downcast_ref::<crate::effect::Restriction>().map(describe_restriction))
                .or_else(|| value.downcast_ref::<ironsmith_core::ThisSpellCostCondition>()
                    .and_then(crate::static_abilities::describe_this_spell_cost_condition))
                .or_else(|| value.downcast_ref::<crate::effect::Value>().map(describe_value))
                // A delayed effect program (opening-hand reveals).
                .or_else(|| {
                    value
                        .downcast_ref::<Vec<crate::effect::Effect>>()
                        .map(|effects| compile_effect_list(effects))
                })
        },
        || {
            // Each ability renders alone: the ordinary pipeline merges and
            // orders neighbouring lines by comparing their retained prose,
            // which would let presentation text shape the audit.
            // The base chunk carries the spell effect, casting options and
            // enchant restriction; each ability chunk carries only itself.
            let mut base = def.clone();
            base.abilities.clear();
            base.ability_labels.clear();
            let base_lines = compiled_text_lines(&base);
            let mut lines: Vec<(AuditLine, String)> = base_lines
                .iter()
                .cloned()
                .map(|line| (AuditLine::Rendered, line))
                .collect();
            // CR 716.2a: a Class level's ability is printed under its level
            // header; the self-grant gated on that level reads as the
            // granted abilities, as in the ordinary renderer.
            let abilities = def
                .abilities
                .iter()
                .enumerate()
                .flat_map(|(index, ability)| {
                    if let AbilityKind::Static(static_ability) = &ability.kind
                        && matches!(
                            static_ability.granted_inline_condition(),
                            Some(crate::ConditionExpr::SourceClassLevelAtLeast(_))
                        )
                    {
                        let mut granted = static_ability.source_granted_inline_abilities();
                        if granted.is_empty() {
                            granted.extend(static_ability.granted_inline_ability());
                        }
                        if !granted.is_empty() {
                            return granted.into_iter().map(|inner| (index, inner)).collect::<Vec<_>>();
                        }
                    }
                    vec![(index, ability)]
                })
                .collect::<Vec<_>>();
            for (index, ability) in abilities {
                let mut single = def.clone();
                single.spell_effect = None;
                single.alternative_casts.clear();
                single.optional_costs.clear();
                single.additional_cost = Default::default();
                single.aura_attach_filter = None;
                // Static abilities read through the engine's audit-aware
                // display: many text renderers here key on retained strings.
                if let AbilityKind::Static(static_ability) = &ability.kind {
                    // Abilities parsed from one printed line share a group,
                    // keyed only by label equality, so adjacent readings are
                    // merged only where the card prints them together.
                    let group = (0..=index)
                        .rev()
                        .take_while(|&earlier| {
                            def.ability_labels.get(earlier).is_some()
                                && def.ability_labels.get(earlier) == def.ability_labels.get(index)
                        })
                        .last()
                        .unwrap_or(index);
                    // Level up tiers print as the card lays them out: the
                    // range, the base power/toughness, then each ability.
                    if let Some(levels) = static_ability.level_abilities()
                        && !levels.is_empty()
                    {
                        let subject = subject_for_card(&def.card);
                        for level in levels {
                            let range = match level.max_level {
                                Some(max) if max == level.min_level => format!("Level {}", level.min_level),
                                Some(max) => format!("Level {}-{}", level.min_level, max),
                                None => format!("Level {}+", level.min_level),
                            };
                            lines.push((AuditLine::Rendered, format!("{range}.")));
                            if let Some((power, toughness)) = level.power_toughness {
                                lines.push((AuditLine::Rendered, format!("{power}/{toughness}.")));
                            }
                            for granted in &level.abilities {
                                let text = granted.display();
                                let text = text.trim().trim_end_matches('.');
                                let text = audit_static_subject_surface(
                                    &format!("{}.", capitalize_first(text)),
                                    subject,
                                    None,
                                );
                                lines.push((AuditLine::Rendered, text));
                            }
                            // CR 711.2a: a tier's activated abilities are
                            // separate abilities gated on its level range;
                            // they print under the tier header.
                            for (other_index, other) in def.abilities.iter().enumerate() {
                                let AbilityKind::Activated(activated) = &other.kind else {
                                    continue;
                                };
                                if level_range_activation_prefix(activated).as_deref() != Some(range.as_str()) {
                                    continue;
                                }
                                let mut tier_ability = other.clone();
                                if let AbilityKind::Activated(activated) = &mut tier_ability.kind {
                                    activated
                                        .additional_restrictions
                                        .retain(|restriction| !restriction.starts_with("__ironsmith_level_range:"));
                                }
                                let mut tier = single.clone();
                                tier.abilities = vec![tier_ability];
                                tier.ability_labels =
                                    def.ability_labels.get(other_index).cloned().into_iter().collect();
                                lines.extend(
                                    compiled_text_lines(&tier)
                                        .into_iter()
                                        .map(|line| (AuditLine::Rendered, line)),
                                );
                            }
                        }
                        continue;
                    }
                    let kind = if static_ability.is_keyword() {
                        AuditLine::Keyword(group)
                    } else {
                        AuditLine::Static(group)
                    };
                    // An Aura's Enchant static mirrors `aura_attach_filter`,
                    // which the base chunk already read; as in ordinary text,
                    // it is not a second line.
                    if static_ability.id() == crate::static_abilities::StaticAbilityId::Enchant
                        && def.aura_attach_filter.is_some()
                    {
                        continue;
                    }
                    // Undaunted (CR 702.125a): an unconditional reduction of
                    // {1} per opponent on every spell, functioning only while
                    // the card is in hand or on the stack, so it reduces this
                    // spell alone.
                    if ability.functional_zones.contains(&Zone::Hand)
                        && ability.functional_zones.contains(&Zone::Stack)
                        && !ability.functional_zones.contains(&Zone::Battlefield)
                        && let Some(reduction) = static_ability.cost_reduction()
                        && reduction.filter == ObjectFilter::default()
                        && reduction.condition.is_none()
                        && !reduction.per_target
                        && reduction.characteristic_intersection.is_none()
                        && matches!(reduction.reduction, Value::CountPlayers(PlayerFilter::Opponent))
                    {
                        lines.push((AuditLine::Keyword(group), "Undaunted".to_string()));
                        continue;
                    }
                    if let Some(line) = static_ability
                        .compiled_model()
                        .and_then(audit_readings::audit_payload_reading)
                    {
                        lines.push((kind, format!("{}.", capitalize_first(line.trim_end_matches('.')))));
                        continue;
                    }
                    // Rule restrictions have a typed reader here.
                    if let Some(model) = static_ability.compiled_model()
                        && let ironsmith_core::StaticAbilityPayload::RuleRestriction {
                            restriction,
                            additional_restrictions,
                            ..
                        } = &model.payload
                    {
                        let line = std::iter::once(restriction)
                            .chain(additional_restrictions)
                            .map(|restriction| {
                                audit_named_attack_exception(restriction, &def.card.name)
                                    .unwrap_or_else(|| audit_rule_restriction_reading(restriction))
                            })
                            .collect::<Vec<_>>()
                            .join(" and ");
                        lines.push((kind, format!("{}.", capitalize_first(line.trim_end_matches('.')))));
                        continue;
                    }
                    let line = static_ability.display();
                    let line = line.trim().trim_end_matches('.');
                    if !line.is_empty() {
                        lines.push((kind, format!("{}.", capitalize_first(line))));
                    }
                    continue;
                }
                single.abilities = vec![ability.clone()];
                single.ability_labels = def.ability_labels.get(index).cloned().into_iter().collect();
                // The attachment filter names "enchanted creature" in the
                // ability; its own Enchant / attach-only line belongs to the
                // base chunk, so a repeat of it is dropped here.
                single.aura_attach_filter = def.aura_attach_filter.clone();
                let rendered = compiled_text_lines(&single)
                    .into_iter()
                    .filter(|line| {
                        !((line.starts_with("Enchant ")
                            || line.starts_with("This Equipment can be attached only to "))
                            && base_lines.contains(line))
                    })
                    .collect::<Vec<_>>();
                // A triggered keyword (firebending, annihilator) printed on
                // the same line as the static keywords before it joins their
                // keyword list: "Flying, firebending 2" (label equality only).
                let shared_line_group = (0..=index)
                    .rev()
                    .take_while(|&earlier| {
                        def.ability_labels.get(earlier).is_some()
                            && def.ability_labels.get(earlier) == def.ability_labels.get(index)
                    })
                    .last()
                    .filter(|&group| group != index);
                if let (Some(group), [only]) = (shared_line_group, rendered.as_slice())
                    && is_triggered_list_audit_keyword_r5f(only.trim().trim_end_matches('.'))
                {
                    lines.push((AuditLine::Keyword(group), only.clone()));
                    continue;
                }
                lines.extend(rendered.into_iter().map(|line| (AuditLine::Rendered, line)));
            }
            audit_residual_d_rephrasings::audit_residual_d_lines(audit_merge_ability_loss_with_grant_r5a(audit_merge_conditional_keyword_lines(audit_cast_and_activate_restrictions(audit_join_activation_cost_modifiers(
                audit_merge_keyword_loss_modes(compact_station_threshold_lines(collapse_audit_keyword_expansions(
                    finish_audit_static_lines(def, lines),
                ))),
            )))))
            .into_iter()
            .map(|line| audit_deck_copy_limit_surface(line, &def.card.name))
            .map(|line| audit_ability_line_rephrasings(line, def))
            .map(audit_replacement_instead_position)
            .map(audit_line_rephrasings_r5a)
            .map(audit_for_each_revealed_quality)
            .map(audit_follow_up_mana_spent)
            .map(audit_keyword_dash_cost_sentence)
            .map(audit_quoted_keyword_grants)
            .map(|line| audit_residual_d_rephrasings::audit_residual_d_line(line))
            // A granted ability names the permanent that grants it by that
            // permanent's name ("Return Trusty Boomerang to its owner's hand").
            .map(|line| {
                // A replacement on the source itself names it as the card's
                // own subject: "If this creature would die, ...".
                let line = line.replace(
                    "If a this would ",
                    &format!("If {} would ", subject_for_card(&def.card)),
                );
                // "As this creature transforms into Shinryu, ..." names the
                // face carrying the program.
                let line = line.replace(" transforms into this face, ", &format!(" transforms into {}, ", def.card.name));
                if line.contains(" enter as a copy ") {
                    line
                } else {
                    line.replace("the granting permanent", &def.card.name)
                }
            })
            // After the printed-line merges, so a keyword grant and a quoted
            // grant under one condition stay one sentence (Flaring Flame-Kin).
            .map(|line| audit_quoted_grant_condition_lead(&line).unwrap_or(line))
            .map(audit_quoted_grant_condition_first)
            .map(audit_max_speed_surface)
            // "Max speed — This creature has lifelink.": a keyword the card
            // itself has at max speed names the card as its subject.
            .map(|line| match line
                .strip_prefix("Max speed \u{2014} ")
                .and_then(|rest| rest.strip_suffix('.'))
            {
                Some(keyword)
                    if merge_passes::is_keyword_phrase(keyword)
                        && keyword.chars().next().is_some_and(|ch| ch.is_ascii_uppercase()) =>
                {
                    format!(
                        "Max speed \u{2014} {} has {}.",
                        capitalize_first(subject_for_card(&def.card)),
                        lowercase_first(keyword)
                    )
                }
                _ => line,
            })
            .map(|line| audit_residual_e_line_surfaces(line, def))
            .fold(Vec::new(), audit_residual_e_fold_lines)
        },
    )
}

/// "Except for creatures named Akron Legionnaire and artifact creatures,
/// creatures you control can't attack": an attack restriction whose filter
/// excludes exactly one name and the artifact card type reads its two
/// exceptions first, as Oracle prints them.
fn audit_named_attack_exception(
    restriction: &crate::effect::Restriction,
    card_name: &str,
) -> Option<String> {
    let crate::effect::Restriction::Attack(filter) = restriction else {
        return None;
    };
    let excluded_name = filter.excluded_name.as_deref()?;
    if filter.excluded_card_types != [CardType::Artifact]
        || filter.card_types != [CardType::Creature]
    {
        return None;
    }
    let mut rest = filter.clone();
    rest.excluded_name = None;
    rest.excluded_card_types.clear();
    let rest_line = describe_restriction(&crate::effect::Restriction::Attack(rest));
    let rest_line = rest_line.trim_end_matches('.');
    let subject = match rest_line {
        "A creature you control can't attack" | "Creatures you control can't attack"
        | "a creature you control can't attack" | "creatures you control can't attack" => {
            "creatures you control"
        }
        _ => return None,
    };
    let name = if excluded_name.eq_ignore_ascii_case(card_name) {
        card_name
    } else {
        excluded_name
    };
    Some(format!(
        "Except for creatures named {name} and artifact creatures, {subject} can't attack"
    ))
}

/// A rule restriction's audit reading. Players other than the active player
/// being unable to cast spells is the printed "Players can cast spells only
/// during their own turns" (Dosan the Falling Leaf).
fn audit_rule_restriction_reading(restriction: &crate::effect::Restriction) -> String {
    if let crate::effect::Restriction::CastSpellsMatching(
        PlayerFilter::Excluding { base, excluded },
        spells,
    ) = restriction
        && **base == PlayerFilter::Any
        && **excluded == PlayerFilter::Active
        && *spells == ObjectFilter::default()
    {
        return "Players can cast spells only during their own turns".to_string();
    }
    // "You can cast spells only during your turn" (Fires of Invention): you
    // are excluded from casting while you aren't the active player.
    if let crate::effect::Restriction::CastSpellsMatching(
        PlayerFilter::Excluding { base, excluded },
        spells,
    ) = restriction
        && **base == PlayerFilter::You
        && **excluded == PlayerFilter::Active
        && *spells == ObjectFilter::default()
    {
        return "You can cast spells only during your turn".to_string();
    }
    describe_restriction(restriction)
}

/// A replacement whose substitute runs over several sentences reads "instead"
/// after its first sentence, as Oracle prints it ("If you would draw a card,
/// look at the top three cards of your library instead. Put one of them into
/// your hand ..."), never at the end of the last one.
fn audit_replacement_instead_position(line: String) -> String {
    if !line.starts_with("If ") || line.contains('"') {
        return line;
    }
    let Some(body) = line.strip_suffix(" instead.") else {
        return line;
    };
    let Some((head, substitute)) = body.split_once(", ") else {
        return line;
    };
    if !head.contains(" would ") || head.contains(". ") {
        return line;
    }
    // A one-sentence compound substitute for losing the game reads "instead"
    // first: "If you would lose the game, instead exile this creature and
    // your life total becomes ..." (Exquisite Archangel).
    if head == "If you would lose the game" && !substitute.contains(". ") && substitute.contains(" and ") {
        return format!("{head}, instead {substitute}.");
    }
    // Two plain actions by you read as one compound substitute: "instead you
    // draw two cards and you lose 1 life" (Blood Scrivener).
    if let Some((first, rest)) = substitute.split_once(". ")
        && first.starts_with("you ")
        && rest.starts_with("You ")
        && !rest.starts_with("You may")
        && !rest.contains(". ")
        && !first.contains(" instead")
    {
        return format!("{head}, instead {first} and {}.", lowercase_first(rest));
    }
    let Some((first, rest)) = substitute.split_once(". ") else {
        return line;
    };
    if first.contains(" instead") || rest.is_empty() || rest.contains("Otherwise") {
        return line;
    }
    format!("{head}, {first} instead. {rest}.")
}

/// Pure re-phrasings of whole audit lines into Oracle's wording (identical
/// meaning): "deals 5 damage instead if ...", "shuffles the cards from their
/// hand into their library, then draws that many cards", keyword names in
/// lowercase after "has", and number agreement for "get ... and have".
fn audit_line_rephrasings_r5a(line: String) -> String {
    fn is_pt_token(token: &str) -> bool {
        token.split_once('/').is_some_and(|(power, toughness)| {
            [power, toughness].iter().all(|side| {
                side.len() >= 2
                    && (side.starts_with('+') || side.starts_with('-'))
                    && side[1..].chars().all(|ch| ch.is_ascii_digit() || ch == 'X')
            })
        })
    }
    let mut line = line
        .replace(" damage to that permanent or player instead if ", " damage instead if ")
        .replace(
            "shuffles their hand into their library, then draws that many cards",
            "shuffles the cards from their hand into their library, then draws that many cards",
        )
        .replace(
            "huffle your hand into your library, then draw that many cards",
            "huffle the cards from your hand into your library, then draw that many cards",
        )
        .replace(" unless it is your ", " unless it's your ")
        .replace(" unless this creature is attacking or it's blocking", " unless it's attacking or blocking")
        .replace(" unless unless ", " unless ")
        // The land type chosen as it entered is "the chosen type" (Realmwright,
        // Multiversal Passage, Phantasmal Terrain).
        .replace(" is the chosen basic land type", " is the chosen type")
        .replace(" are the chosen basic land type", " are the chosen type")
        .replace(" is the chosen land type", " is the chosen type")
        .replace(" are the chosen land type", " are the chosen type")
        // Every permanent type as a spell filter is "permanent spells" (a
        // land is never a spell): Codie, Vociferous Codex.
        .replace(" artifact, creature, enchantment, land, planeswalker, or battle spells", " permanent spells")
        .replace(" artifact, creature, enchantment, planeswalker, or battle spells", " permanent spells")
        .replace(
            " if you control two or more creatures that share a creature type",
            " if you control at least two creatures that share a creature type",
        )
        // Oracle contracts the Lesson check ("as long as there's a Lesson
        // card in your graveyard"; Aang, First-Time Flyer, Walltop Sentries).
        .replace(" as long as there is a Lesson card in your graveyard", " as long as there's a Lesson card in your graveyard")
        .replace(", if there is a Lesson card in your graveyard", ", if there's a Lesson card in your graveyard")
        // A this-turn history read as a static or spell condition is
        // perfect ("as long as you've sacrificed an artifact this turn",
        // "This spell costs {3} less to cast if you've sacrificed ...");
        // trigger conditions keep "if you sacrificed".
        .replace(" as long as you sacrificed ", " as long as you've sacrificed ")
        .replace(" as long as you rolled ", " as long as you've rolled ")
        .replace(" less to cast if you sacrificed ", " less to cast if you've sacrificed ")
        .replace(". If you discarded a card this turn, ", ". If you've discarded a card this turn, ")
        .replace(" that aren't on the battlefield has ", " that aren't on the battlefield have ");
    if let Some(rest) = line.strip_prefix("Your opponents can cast spells only any time they could cast a sorcery") {
        line = format!("Each opponent can cast spells only any time they could cast a sorcery{rest}");
    }
    // "As long as this creature is on the battlefield, it has all activated
    // abilities of all creature cards in all graveyards." (Necrotic Ooze,
    // Mirran Safehouse)
    if let Some(body) = line.strip_suffix(" as long as this object is on the battlefield.")
        && let Some((subject, rest)) = body.split_once(" has all activated abilities of ")
        && let Some(noun) = subject.strip_prefix("This ")
        && !noun.contains(' ')
        && !rest.contains(['"', '.'])
    {
        let rest = rest.replace(" cards in a graveyard", " cards in all graveyards");
        line = format!("As long as this {noun} is on the battlefield, it has all activated abilities of {rest}.");
    }
    if let Some(rest) = line.strip_prefix("If you discarded a card this turn, ") {
        line = format!("If you've discarded a card this turn, {rest}");
    }
    // Trap and timing conditions print the simple past: "If an opponent
    // drew three or more cards this turn" (Runeflare Trap), "Cast this spell
    // only if an opponent cast a creature spell this turn" (Lure of Prey).
    if let Some(rest) = line.strip_prefix("If an opponent has drawn ") {
        line = format!("If an opponent drew {rest}");
    }
    if let Some(rest) = line.strip_prefix("Cast this spell only if an opponent has cast ") {
        line = format!("Cast this spell only if an opponent cast {rest}");
    }
    for keyword in ["Firebending", "Annihilator", "Bushido", "Rampage", "Flanking", "Modular", "Afflict"] {
        for verb in [" has ", " have "] {
            let from = format!("{verb}{keyword} ");
            if line.contains(&from) {
                line = line.replace(&from, &format!("{verb}{} ", keyword.to_ascii_lowercase()));
            }
        }
    }
    // "Other Riggers you control get +1/+0 and has haste." -> "... and have
    // haste."
    if let Some(at) = line.find(" get ")
        && let Some((pt, rest)) = line[at + " get ".len()..].split_once(' ')
        && is_pt_token(pt)
        && let Some(tail) = rest.strip_prefix("and has ")
    {
        line = format!("{} get {pt} and have {tail}", &line[..at]);
    }
    // "This creature gets +1/+1 and menace." -> "... and has menace."
    if let Some(at) = line.find(" gets ")
        && let Some((pt, rest)) = line[at + " gets ".len()..].split_once(' ')
        && is_pt_token(pt)
        && let Some(keyword) = rest.strip_prefix("and ").and_then(|tail| tail.strip_suffix('.'))
        && [
            "flying", "menace", "haste", "trample", "vigilance", "lifelink", "deathtouch", "reach",
            "first strike", "double strike", "hexproof", "indestructible",
        ]
        .contains(&keyword)
    {
        line = format!("{} gets {pt} and has {keyword}.", &line[..at]);
    }
    // A granted rules sentence is quoted, as Oracle prints a granted ability:
    // "Enchanted creature has "If this creature would untap ..."" (Bewitching
    // Leechcraft), "it has "Enchanted permanent has flying."".
    if !line.contains('"') {
        let starters = ["If ", "Prevent ", "Enchanted ", "Equipped ", "This "];
        let found = [" has ", " have "].iter().find_map(|verb| {
            let at = line.find(*verb)?;
            let inner = &line[at + verb.len()..];
            starters.iter().any(|starter| inner.starts_with(starter)).then_some((at + verb.len(), inner))
        });
        if let Some((start, inner)) = found {
            // A trailing condition stays outside the quote: "has "Prevent all
            // damage that would be dealt to this creature." as long as ...".
            let (quoted, tail) = match inner.split_once(". as long as ") {
                Some((quoted, condition)) => {
                    (quoted.to_string(), format!(" as long as {}.", condition.trim_end_matches('.')))
                }
                None => (inner.trim_end_matches('.').to_string(), String::new()),
            };
            line = format!("{}\"{quoted}.\"{tail}", &line[..start]);
        }
    }
    audit_also_follow_up_r5a(line)
}

/// A conditional follow-up that gives the same object more of what an
/// earlier sentence gave it reads with "also": "Target creature gets +2/+1
/// until end of turn. If it's legendary, it also gains lifelink until end of
/// turn." (Blessing of Belzenlok), "If you do, Heated Argument also deals 2
/// damage ...". Replacements ("... instead") never take "also".
fn audit_also_follow_up_r5a(line: String) -> String {
    if !line.contains(". If ") {
        return line;
    }
    let sentences: Vec<&str> = line.split(". ").collect();
    let mut out: Vec<String> = vec![sentences[0].to_string()];
    for index in 1..sentences.len() {
        let sentence = sentences[index];
        let rewritten = (|| {
            if !sentence.starts_with("If ") || sentence.contains(" also ") || sentence.contains("instead") {
                return None;
            }
            let (condition, rest) = sentence.split_once(", ")?;
            let words: Vec<&str> = rest.split(' ').collect();
            let verb_at = words
                .iter()
                .position(|word| matches!(*word, "gains" | "gets" | "deals" | "gain" | "get" | "deal"))?;
            if verb_at == 0 {
                return None;
            }
            let subject = words[..verb_at].join(" ");
            let named = words[..verb_at]
                .iter()
                .all(|word| word.chars().next().is_some_and(|ch| ch.is_ascii_uppercase()));
            if !named && !matches!(subject.as_str(), "it" | "that creature" | "that permanent" | "they") {
                return None;
            }
            let earlier = format!(" {}", sentences[..index].join(". "));
            let verb = words[verb_at];
            let repeats = if verb.starts_with("deal") {
                earlier.contains(" deals ")
            } else {
                [" gets ", " gains ", " get ", " gain "].iter().any(|form| earlier.contains(form))
            };
            if !repeats {
                return None;
            }
            Some(format!("{condition}, {subject} also {}", words[verb_at..].join(" ")))
        })();
        out.push(rewritten.unwrap_or_else(|| sentence.to_string()));
    }
    out.join(". ")
}

/// An ability loss printed with the abilities the same subject gains reads
/// as one sentence: "All lands have "{T}: Add one mana of any color" and
/// lose all other abilities." (Toxicrene), "Enchanted land loses all land
/// types and abilities and has "{T}: Add {C}" ..." (Lithoform Blight). A
/// lone "loses all land types and loses all abilities" reads "loses all
/// land types and abilities".
fn audit_merge_ability_loss_with_grant_r5a(lines: Vec<String>) -> Vec<String> {
    fn loss(line: &str) -> Option<(&str, &'static str, bool)> {
        let body = line.strip_suffix('.')?;
        for (suffix, verb, land_types) in [
            (" loses all land types and loses all abilities", "loses", true),
            (" lose all land types and lose all abilities", "lose", true),
            (" loses all abilities", "loses", false),
            (" lose all abilities", "lose", false),
        ] {
            if let Some(subject) = body.strip_suffix(suffix)
                && !subject.is_empty()
                && !subject.contains(['"', '.', ','])
            {
                return Some((subject, verb, land_types));
            }
        }
        None
    }
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut index = 0;
    while index < lines.len() {
        let line = &lines[index];
        let Some((subject, verb, land_types)) = loss(line) else {
            out.push(line.clone());
            index += 1;
            continue;
        };
        let next = lines.get(index + 1).and_then(|next| {
            let rest = next.strip_prefix(subject)?.strip_prefix(' ')?;
            (next.contains('"') && (rest.starts_with("has \"") || rest.starts_with("have \"") || rest.starts_with("is ")))
                .then_some(rest)
        });
        match (next, land_types) {
            (Some(rest), true) => {
                out.push(format!("{subject} {verb} all land types and abilities and {rest}"));
                index += 2;
            }
            (Some(rest), false) => {
                let rest = rest.strip_suffix('.').unwrap_or(rest);
                let rest = rest.strip_suffix(".\"").map(|body| format!("{body}\"")).unwrap_or_else(|| rest.to_string());
                out.push(format!("{subject} {rest} and {verb} all other abilities."));
                index += 2;
            }
            (None, true) => {
                out.push(format!("{subject} {verb} all land types and abilities."));
                index += 1;
            }
            (None, false) => {
                out.push(line.clone());
                index += 1;
            }
        }
    }
    audit_merge_flash_permission_pairs_r5a(out)
}

/// A casting permission and an "as though it had flash" permission over the
/// same spells read as one sentence: "Any player may cast creature spells
/// with mana value 3 or less without paying their mana costs and as though
/// they had flash." (Aluren); "The first creature spell you cast each turn
/// costs {2} less to cast and can be cast as though it had flash."
/// (Radagast of Rhosgobel).
fn audit_merge_flash_permission_pairs_r5a(lines: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut index = 0;
    while index < lines.len() {
        let line = &lines[index];
        if let Some(next) = lines.get(index + 1)
            && !line.contains('"')
            && !next.contains('"')
            && let Some(first) = line.strip_suffix('.')
        {
            let first_words: Vec<&str> = first.split(' ').collect();
            let next_words: Vec<&str> = next.split(' ').collect();
            let shared = first_words
                .iter()
                .zip(next_words.iter())
                .take_while(|(left, right)| left == right)
                .count();
            let next_rest = next_words[shared.min(next_words.len())..].join(" ");
            if shared >= 3
                && shared < first_words.len()
                && matches!(
                    next_rest.as_str(),
                    "as though they had flash." | "can be cast as though it had flash."
                )
            {
                out.push(format!(
                    "{} {} and {next_rest}",
                    first_words[..shared].join(" "),
                    first_words[shared..].join(" ")
                ));
                index += 2;
                continue;
            }
        }
        out.push(line.clone());
        index += 1;
    }
    out
}

/// A keyword with a non-mana cost prints the cost as a sentence after the
/// dash: "Escalate—Discard a card.", "Ward—Pay 3 life.".
fn audit_keyword_dash_cost_sentence(line: String) -> String {
    let Some((keyword, cost)) = line.split_once('\u{2014}') else {
        return line;
    };
    if !["Escalate", "Ward"].contains(&keyword) || cost.is_empty() || cost.contains('"') {
        return line;
    }
    let cost = capitalize_first(cost.trim_end_matches('.'));
    let cost = cost.replace(" a enchantment", " an enchantment").replace(" a artifact", " an artifact");
    format!("{keyword}\u{2014}{cost}.")
}

/// A follow-up check on the spell that triggered the ability reads "If five or
/// more mana was spent to cast that spell" (opus); the intervening-if form
/// keeps "if at least four mana was spent to cast it".
fn audit_follow_up_mana_spent(line: String) -> String {
    const HEAD: &str = ". If at least ";
    const TAIL: &str = " mana was spent to cast it, ";
    let Some(start) = line.find(HEAD) else {
        return line;
    };
    let after = &line[start + HEAD.len()..];
    let Some((amount, rest)) = after.split_once(TAIL) else {
        return line;
    };
    if amount.is_empty() || amount.contains(' ') {
        return line;
    }
    format!(
        "{}. If {amount} or more mana was spent to cast that spell, {rest}",
        &line[..start]
    )
}

/// "For each card revealed this way, if it was a nonland card, investigate"
/// reads with the quality on the counted cards: "For each nonland card
/// revealed this way, investigate" (parley).
fn audit_for_each_revealed_quality(line: String) -> String {
    const HEAD: &str = "For each card revealed this way, if it was a";
    let Some(start) = line.find(HEAD) else {
        return line;
    };
    let after = &line[start + HEAD.len()..];
    let after = after.strip_prefix("n ").or_else(|| after.strip_prefix(' '));
    let Some((quality, rest)) = after.and_then(|after| after.split_once(" card, ")) else {
        return line;
    };
    if quality.is_empty() || quality.contains(' ') {
        return line;
    }
    format!(
        "{}For each {quality} card revealed this way, {rest}",
        &line[..start]
    )
}

/// A granted keyword trigger whose whole typed expansion is quoted reads as
/// the keyword: evolve (CR 702.100a), exploit (CR 702.110a).
fn audit_quoted_keyword_grants(line: String) -> String {
    const EVOLVE: &str = "\"Whenever a creature you control enters, if that creature's power is greater than this creature's power and/or that creature's toughness is greater than this creature's toughness, evolve.\"";
    const EXPLOIT: [&str; 2] = [
        "\"When this creature enters, you may sacrifice a creature. If you do,.\"",
        "\"When this creature enters, you may sacrifice a creature.\"",
    ];
    let mut line = line;
    for verb in [" has ", " have "] {
        for (quoted, keyword) in std::iter::once((EVOLVE, "evolve")).chain(EXPLOIT.map(|text| (text, "exploit"))) {
            let from = format!("{verb}{quoted}");
            if line.contains(&from) {
                line = line.replace(&from, &format!("{verb}{keyword}"));
                if line.ends_with("..") {
                    line.pop();
                }
                if !line.ends_with('.') {
                    line.push('.');
                }
            }
        }
    }
    line
}

/// A quoted grant gated on a condition reads with the condition first, as
/// Oracle always prints it: "As long as there are seven or more cards in
/// your graveyard, this creature has "When this creature enters, ..."".
fn audit_quoted_grant_condition_first(line: String) -> String {
    let Some(close) = line.rfind('"') else {
        return line;
    };
    let (head, tail) = line.split_at(close + 1);
    let Some(condition) = tail
        .strip_prefix(" as long as ")
        .or_else(|| tail.strip_prefix(" As long as "))
        .and_then(|condition| condition.strip_suffix('.'))
    else {
        return line;
    };
    if condition.is_empty() || condition.contains(". ") || condition.contains(" as long as ") {
        return line;
    }
    let Some(open) = head.find(" has \"").or_else(|| head.find(" have \"")) else {
        return line;
    };
    let subject = &head[..open];
    if subject.is_empty()
        || subject.contains('"')
        || subject.contains(". ")
        || subject.starts_with("As long as ")
        || subject.starts_with("Max speed")
    {
        return line;
    }
    // The quoted ability closes with its own period, as printed.
    let mut granted = head[open..].to_string();
    if !granted.ends_with(".\"") {
        granted.pop();
        granted.push_str(".\"");
    }
    format!("As long as {condition}, {}{granted}", lowercase_first(subject))
}

/// Pure re-phrasings of whole audit lines into the wording Oracle prints for
/// the same rule (residual static and spell readings).
fn audit_residual_e_line_surfaces(line: String, def: &CardDefinition) -> String {
    // "Max speed — This creature has deathtouch and haste." (Gastal
    // Thrillseeker): several keywords the card has at max speed.
    if let Some(keywords) = line
        .strip_prefix("Max speed \u{2014} ")
        .and_then(|rest| rest.strip_suffix('.'))
        && keywords.chars().next().is_some_and(|ch| ch.is_ascii_uppercase())
        && keywords.contains(" and ")
        && keywords
            .split(", ")
            .flat_map(|part| part.split(" and "))
            .all(|part| !part.is_empty() && merge_passes::is_keyword_phrase(part))
    {
        return format!(
            "Max speed \u{2014} {} has {}.",
            capitalize_first(subject_for_card(&def.card)),
            lowercase_first(keywords)
        );
    }
    let exact = match line.as_str() {
        // Replacing a draw with nothing is skipping it (CR 614.10).
        "If you would draw a card, you may do nothing instead." => {
            Some("If you would draw a card, you may skip that draw instead.")
        }
        "If a player would draw a card, do nothing instead." => {
            Some("If a player would draw a card, that player skips that draw instead.")
        }
        "Nonland permanents are legendary." => Some("All nonland permanents are legendary."),
        // Ethersworn Canonist: a one-spell limit is "can't cast additional".
        "Each player can't cast more than one nonartifact spell each turn." => Some(
            "Each player who has cast a nonartifact spell this turn can't cast additional nonartifact spells.",
        ),
        "Creatures you control can't be blocked by more than 1 creature." => {
            Some("Each creature you control can't be blocked by more than one creature.")
        }
        "You may spend mana as though it were mana of any type to cast creature spells." => {
            Some("You can spend mana of any type to cast creature spells.")
        }
        // Inquisitor's Flail: combat damage by and to the equipped creature.
        "If a creature with this Equipment attached to it would deal combat damage to a permanent or player, it deals double that damage instead."
        | "If a creature with this Equipment attached to it would deal combat damage to a player or a permanent, it deals double that damage instead." => {
            Some("If equipped creature would deal combat damage, it deals double that damage instead.")
        }
        "If a creature without this Equipment attached to it would deal combat damage to a creature with this Equipment attached to it, it deals double that damage instead." => Some(
            "If another creature would deal combat damage to equipped creature, it deals double that damage to equipped creature instead.",
        ),
        "Creatures block each combat if able." => Some("All creatures block each combat if able."),
        "Creatures attack each combat if able." => Some("All creatures attack each combat if able."),
        "The attacking player chooses how creatures block." => {
            Some("The attacking player chooses how each creature blocks each combat.")
        }
        _ => None,
    };
    if let Some(exact) = exact {
        return exact.to_string();
    }
    let mut line = line
        .replace(" spells with {X} in its mana cost can't be cast.", " spells with {X} in their mana costs can't be cast.")
        .replace(" can't be blocked by more than 1 creature", " can't be blocked by more than one creature")
        .replace(" for each spell an opponent cast this turn", " for each spell your opponents have cast this turn")
        .replace(" for each opponent who were dealt ", " for each opponent who was dealt ")
        .replace(" by Oxes.", " by Oxen.")
        .replace(" as long as it's an attacking permanent.", " as long as it's attacking.")
        .replace("protection from colored spells", "protection from spells that are one or more colors")
        .replace(
            ", shuffle its owner's graveyard into its owner's library.",
            ", its owner shuffles their graveyard into their library.",
        );
    // Indominus Rex, Alpha: each keyword counter is put on it as it enters if
    // a card discarded this way has that keyword.
    if let Some((head, rest)) = line.split_once(" creature cards. Then if it has ")
        && let Some(discard) = head.strip_prefix("As this creature enters, you discard ")
        && !discard.contains(['.', ','])
        && let Some(rest) = rest.strip_suffix('.')
    {
        let keywords = rest
            .split(". Then if it has ")
            .map(|sentence| {
                let (keyword, put) = sentence.split_once(", put ")?;
                let counter = put.strip_suffix(" counter on this creature")?;
                let counter = counter.strip_prefix("a ").or_else(|| counter.strip_prefix("an "))?;
                (counter == keyword && merge_passes::is_keyword_phrase(keyword)).then_some(keyword)
            })
            .collect::<Option<Vec<_>>>();
        if let Some(keywords) = keywords
            && let Some((first, others)) = keywords.split_first()
            && !others.is_empty()
        {
            let list = match others {
                [only] => only.to_string(),
                [init @ .., last] => format!("{}, and {last}", init.join(", ")),
                [] => String::new(),
            };
            line = format!(
                "As this creature enters, discard {discard} creature cards. It enters with {} on it if a card discarded this way has {first}. The same is true for {list}.",
                with_indefinite_article(&format!("{first} counter"))
            );
        }
    }
    // "You may have this land enter tapped. If you do, ..." (Mariposa
    // Military Base): tapping it as it enters is entering tapped.
    if let Some(rest) = line.strip_prefix("As this land enters, you may tap this land. If you do, ") {
        line = format!("You may have this land enter tapped. If you do, {rest}");
    }
    if line == "Foretelling costs {1} less and is possible during any player's turn." {
        line = "Foretelling cards from your hand costs {1} less and can be done on any player's turn.".to_string();
    }
    // "Each creature that's enchanted by an Aura you control can't attack you
    // ..." (Eriette of the Charmed Apple)
    if let Some(rest) = line.strip_prefix("Creature with an Aura you control attached to it ") {
        line = format!("Each creature that's enchanted by an Aura you control {rest}");
    }
    // "all loyalty abilities of all other planeswalkers on the battlefield"
    // (Nicol Bolas, Dragon-God; Experiment Kraj): "other" already excludes
    // the source.
    if line.contains(" abilities of all other ")
        && let Some(head) = line.strip_suffix(" other than this object.")
    {
        line = if head.ends_with(" on it") { format!("{head}.") } else { format!("{head} on the battlefield.") };
    }
    let subject = subject_for_card(&def.card);
    line = line
        // Fated Clash: "if a creature is attacking and a creature is blocking".
        .replace(
            " if at least one attacking creature and at least one blocking creature.",
            " if a creature is attacking and a creature is blocking.",
        )
        // Ashes of the Abhorrent.
        .replace(
            "Players can't cast spells from graveyards and activated abilities of card in a graveyard can't be activated.",
            "Players can't cast spells from graveyards or activate abilities of cards in graveyards.",
        )
        // Predator's Gambit.
        .replace(
            " as long as there are no creature its controller controls other than enchanted creatures.",
            " as long as its controller controls no other creatures.",
        )
        // A creature that dies was on the battlefield (Skeletonize).
        .replace(
            " a creature on the battlefield dealt damage this way dies this turn",
            " a creature dealt damage this way dies this turn",
        )
        // Structural Assault, Anzrag's Rampage.
        .replace(
            " put into graveyard from battlefield this turn",
            " that were put into graveyards from the battlefield this turn",
        )
        // Kellan, the Fae-Blooded.
        .replace(" attached to a this on the battlefield.", &format!(" attached to {subject}."))
        // Tibalt, Cosmic Impostor: the emblem names the card that made it.
        .replace("__compiled_source_card_name__", &def.card.name)
        .replace("As this becomes attached to ", &format!("As {subject} becomes attached to "));
    // "the total number of instant and sorcery cards you own in exile and in
    // your graveyard" (Crackling Drake, Beacon Bolt, Ral, Serpentine Curve)
    if let Some((head, rest)) = line.split_once("the number of ")
        && let Some((kind, rest)) = rest.split_once(" card in your exiles and ")
        && let Some(tail) = rest.strip_prefix(&format!("{kind} cards in your graveyard"))
        && !kind.contains([',', '.'])
    {
        line = format!("{head}the total number of {kind} cards you own in exile and in your graveyard{tail}");
    }
    // "You may put a creature card from among them onto the battlefield."
    // (See the Unwritten, Earth's Mightiest Heroes)
    if let Some((head, rest)) = line.split_once(". Put up to one ")
        && let Some((noun, tail)) = rest.split_once(" from among them onto the battlefield")
        && !noun.contains([',', '.'])
    {
        line = format!(
            "{head}. You may put {} from among them onto the battlefield{tail}",
            with_indefinite_article(noun)
        );
    }
    // "During your turn, creatures you control that are equipped have first
    // strike and vigilance." (Long-Lost Lances)
    if let Some(body) = line.strip_suffix(" during your turn.")
        && let Some((head, second)) = body.split_once(" during your turn and ")
        && let Some((subject, first)) = head.split_once(" have ")
        && merge_passes::is_keyword_phrase(first)
        && merge_passes::is_keyword_phrase(second)
        && !subject.contains(['.', ',', '"'])
    {
        line = format!("During your turn, {} have {first} and {second}.", lowercase_first(subject));
    }
    // "unless it has an even number of counters on it" (Sab-Sunen)
    line = line
        .replace(" unless it's this creature with ", " unless it has ")
        .replace(" if this creature is this creature with ", " if it has ");
    // "It's an Island in addition to its other types for as long as it has a
    // flood counter on it." (Eluge, Minas Morgul)
    if let Some((head, rest)) = line.split_once(". It becomes ")
        && rest.contains(" in addition to its other types for as long as it has ")
    {
        line = format!("{head}. It's {rest}");
    }
    // "If this creature hasn't been exerted this turn, you may exert it as it
    // attacks." (Combat Celebrant)
    if let Some(rest) = line.strip_prefix("You may exert this creature as it attacks if it wasn't exerted this turn.") {
        line = format!("If this creature hasn't been exerted this turn, you may exert it as it attacks.{rest}");
    }
    // "This creature enters with two +1/+1 counters on it unless two or more
    // colors of mana were spent to cast it." (Steel Exemplar, Hotheaded
    // Giant): a negated entry condition reads as "unless".
    if let Some((condition, rest)) = line
        .strip_prefix("If it isn't the case that ")
        .and_then(|rest| rest.split_once(", "))
        && let Some((subject, counters)) = rest.strip_suffix('.').and_then(|rest| rest.split_once(" enters with "))
        && subject.split(' ').count() == 2
        && subject.starts_with("this ")
        && !condition.contains(['.', '"'])
        && !counters.contains(['.', ',', '"'])
    {
        line = format!(
            "{} enters with {counters} unless {}.",
            capitalize_first(subject),
            condition.replace("to cast this spell", "to cast it").replace("this spell escaped", "it escaped")
        );
    }
    // "During your turn, each instant and sorcery card in your graveyard has
    // flashback. Its flashback cost is equal to its mana cost." (Return the
    // Past, Iroh, Grand Lotus)
    if let Some((grant, keyword)) = line
        .strip_suffix(" cost is equal to its mana cost as long as it's your turn.")
        .and_then(|body| body.rsplit_once(". Its "))
        && grant.ends_with(&format!(" has {keyword}"))
        && !grant.contains(['.', '"'])
    {
        line = format!(
            "During your turn, {}. Its {keyword} cost is equal to its mana cost.",
            lowercase_first(grant)
        );
    }
    if let Some(rest) = line.strip_prefix("Activated abilities of sources cost ") {
        line = format!("Activated abilities cost {rest}");
    }
    // "Unblocked attacking Ninjas you control have lifelink." (Throatseeker)
    if let Some(rest) = line.strip_prefix("Unblockeds attacking ")
        && let Some((noun, tail)) = rest.split_once(' ')
        && noun.chars().next().is_some_and(|ch| ch.is_ascii_uppercase())
    {
        line = format!("Unblocked attacking {} {tail}", pluralize_word(noun));
    }
    // "This spell costs 3 life more to cast for each target." (Phyrexian Purge)
    if let Some(amount) = line
        .strip_prefix("This spell costs an additional ")
        .and_then(|rest| rest.strip_suffix(" life to cast for each target."))
        && !amount.contains(' ')
    {
        line = format!("This spell costs {amount} life more to cast for each target.");
    }
    // "This spell costs {2} less to cast if it's bargained." (Johann's Stopgap)
    if line.starts_with("This spell costs ")
        && let Some(head) = line.strip_suffix(" if this spell was bargained.")
    {
        line = format!("{head} if it's bargained.");
    }
    // "Each spell a player casts costs {1} more to cast for each other spell
    // that player has cast this turn." (Damping Sphere)
    if let Some(cost) = line
        .strip_prefix("Spells cost ")
        .and_then(|rest| rest.strip_suffix(" more to cast for each other spell that player cast this turn."))
        && !cost.contains(' ')
    {
        line = format!(
            "Each spell a player casts costs {cost} more to cast for each other spell that player has cast this turn."
        );
    }
    // "If an instant or sorcery source would deal 3 or more damage to you, it
    // deals 2 damage to you instead." (Forethought Amulet)
    if let Some(rest) = line.strip_prefix("If an instant or sorcery would deal ")
        && let Some((threshold, amount)) = rest.split_once(" damage to you, it deals ")
        && let Some(amount) = amount.strip_suffix(" damage instead.")
        && !threshold.contains(',')
        && !amount.contains(' ')
    {
        line = format!(
            "If an instant or sorcery source would deal {threshold} damage to you, it deals {amount} damage to you instead."
        );
    }
    // "As long as this Vehicle has three or more fire counters on it, it's an
    // artifact creature." (War Balloon, Phoenix Fleet Airship)
    if let Some(condition) = line
        .strip_prefix("This Vehicle is an artifact creature in addition to its other types as long as ")
        .and_then(|rest| rest.strip_suffix('.'))
        && !condition.contains(['.', '"'])
        && !condition.contains(" as long as ")
    {
        line = match condition.strip_prefix("it ") {
            Some(rest) => format!("As long as this Vehicle {rest}, it's an artifact creature."),
            None => format!("As long as {condition}, this Vehicle is an artifact creature."),
        };
    }
    // Flesh Duplicate: the vanishing expansion granted by a copy exception
    // is the keyword itself (CR 702.63a).
    if let Some((head, rest)) = line.split_once(", except it has enters with ")
        && let Some((count, rest)) = rest.split_once(
            " time counters on it and \"At the beginning of your upkeep, if this creature has a time counter on it, remove a time counter from each this creature\" and \"When the last time counter is removed from this creature, sacrifice this creature.\"",
        )
        && rest == "; abilities only if it copies creature without vanishing."
        && let Some(count) = audit_count_word(count)
    {
        line = format!("{head}, except it has vanishing {count} if that creature doesn't have vanishing.");
    }
    // "except it enters with a shield counter on it if you control that
    // creature" (Undercover Operative): the copied creature is the one you
    // control.
    if let Some((head, rest)) = line.split_once(", except it enters with an additional ")
        && let Some(kind) = rest.strip_suffix(" counter on it if it copies a permanent you control.")
        && !kind.contains(' ')
    {
        line = format!(
            "{head}, except it enters with {} on it if you control that creature.",
            with_indefinite_article(&format!("{kind} counter"))
        );
    }
    // "Enchanted creature has base power and toughness 0/2, has defender,
    // and loses all other abilities." (Stasis Field; Spider-Man No More:
    // "... is a Citizen with base power and toughness 1/1. It has defender
    // and loses all other abilities.")
    if let Some(body) = line.strip_suffix('.')
        && let Some((head, keyword)) = body.rsplit_once(" and loses all abilities and ")
        && merge_passes::is_keyword_phrase(keyword)
        && !head.contains(['"', '.', ','])
    {
        line = if head.contains(" is a ") && head.contains(" with base power and toughness ") {
            format!("{head}. It has {keyword} and loses all other abilities.")
        } else {
            format!("{head}, has {keyword}, and loses all other abilities.")
        };
    }
    // "Black creatures can't attack unless their controller sacrifices a land
    // of their choice for each black creature they control that's attacking."
    // (Reclamation, Flooded Woodlands): an attack cost paid per attacker.
    if let Some(quality) = line.strip_suffix(" creatures can't attack unless its controller pays Sacrifice a land.")
        && !quality.contains(' ')
        && quality.chars().next().is_some_and(|ch| ch.is_ascii_uppercase())
    {
        line = format!(
            "{quality} creatures can't attack unless their controller sacrifices a land of their choice for each {} creature they control that's attacking.",
            lowercase_first(quality)
        );
    }
    // "as long as an instant card and a sorcery card are in your graveyard"
    // (Bloodfire Enforcers)
    if let Some((head, rest)) = line.split_once(" as long as there is ")
        && let Some((first, rest)) = rest.split_once(" in your graveyard and there is ")
        && let Some((second, tail)) = rest.split_once(" in your graveyard")
        && !first.contains([',', '.'])
        && !second.contains([',', '.'])
    {
        line = format!("{head} as long as {first} and {second} are in your graveyard{tail}");
    }
    // "As long as this creature is on the battlefield, it's a land in
    // addition to its other types." (Awakened Skyclave; Haakon: "..., you may
    // cast Knight spells from your graveyard.")
    if let Some(rule) = line.strip_suffix(" as long as this object is on the battlefield.")
        && !rule.contains(" as long as ")
        && !rule.contains(['.', '"'])
    {
        let subject = subject_for_card(&def.card);
        let rule = match rule.strip_prefix(&format!("{} is ", capitalize_first(subject))) {
            Some(rest) => {
                let rest = match rest.strip_prefix("land ") {
                    Some(tail) => format!("a land {tail}"),
                    None => rest.to_string(),
                };
                format!("it's {rest}")
            }
            None if rule.starts_with("You ") => lowercase_first(rule),
            None => rule.to_string(),
        };
        line = format!("As long as {subject} is on the battlefield, {rule}.");
    }
    // Keyword-scoped activation costs (Dragonkin Berserker: "Boast abilities
    // you activate cost {1} less to activate for each Dragon you control.";
    // Gavi, Nest Warden; Kíli; Forge Anew).
    for article in ["a", "an"] {
        let qualifier = format!(" if it's {article} ");
        if let Some(rest) = line.strip_prefix("Activated abilities of permanent cost ")
            && let Some((cost, keyword)) = rest
                .strip_suffix(" ability and if you activate it.")
                .and_then(|body| body.rsplit_once(qualifier.as_str()))
            && !keyword.contains(' ')
            && !cost.contains(" if ")
        {
            line = format!("{} abilities you activate cost {cost}.", capitalize_first(keyword));
            break;
        }
        let marker = format!(" rather than pay activated ability costs of permanent if it's {article} ");
        if let Some((head, rest)) = line.split_once(marker.as_str())
            && let Some((keyword, rest)) = rest.split_once(" ability and if you activate it and if it's the first ")
            && let Some(rest) = rest.strip_prefix(&format!("{keyword} ability activated"))
            && !keyword.contains(' ')
            && !head.contains(['.', '"'])
        {
            let (head, window) = match head.strip_prefix("During your turn, you ") {
                Some(tail) => (format!("You {tail}"), "during each of your turns"),
                None => (head.to_string(), "each turn"),
            };
            let first = if keyword == "cycling" {
                "card you cycle".to_string()
            } else {
                format!("{keyword} ability you activate")
            };
            let tail = match rest {
                "." | " this turn." => Some(String::new()),
                other => other
                    .strip_prefix(" this turn as long as ")
                    .and_then(|condition| condition.strip_suffix('.'))
                    .filter(|condition| !condition.contains(['.', ',', '"']))
                    .map(str::to_string),
            };
            if let Some(condition) = tail {
                let rule = format!("{head} rather than pay the {keyword} cost of the first {first} {window}");
                line = if condition.is_empty() {
                    format!("{rule}.")
                } else {
                    format!("As long as {condition}, {}.", lowercase_first(&rule))
                };
                break;
            }
        }
    }
    // "As long as you have seven or more cards in hand, you may pay {0}
    // rather than pay cycling costs." (New Perspectives)
    if let Some((head, condition)) = line
        .strip_suffix('.')
        .and_then(|body| body.split_once(" rather than pay activated ability costs of card with cycling in hand as long as "))
        && !condition.contains(['.', ',', '"'])
        && !head.contains(['.', ',', '"'])
    {
        line = format!("As long as {condition}, {} rather than pay cycling costs.", lowercase_first(head));
    }
    // "During your end step, ..." (Final-Word Phantom, Zurgo, Thunder's
    // Decree): a rule gated on a step window leads with it.
    if let Some(body) = line.strip_suffix('.')
        && let Some((rule, window)) = body.rsplit_once(" as long as during ")
        && !rule.contains(" as long as ")
        && !rule.contains(". ")
        && !window.contains([',', '.', '"'])
        && window.ends_with(" step")
    {
        let first = rule.split(' ').next().unwrap_or("");
        let rule = if matches!(
            first,
            "You" | "Creatures" | "Each" | "Players" | "Spells" | "This" | "Other" | "Enchanted" | "Equipped"
                | "Artifacts" | "Lands" | "Permanents" | "All"
        ) {
            lowercase_first(rule)
        } else {
            rule.to_string()
        };
        line = format!("During {window}, {rule}.");
    }
    line
}

/// "You can cast spells only during your turn and you can cast no more than
/// two spells each turn." (Fires of Invention): two casting restrictions on
/// you printed as one sentence.
fn audit_residual_e_fold_lines(mut out: Vec<String>, line: String) -> Vec<String> {
    if let Some(previous) = out.last_mut()
        && previous.as_str() == "You can cast spells only during your turn."
        && let Some(count) = line
            .strip_prefix("You can cast no more than ")
            .and_then(|rest| rest.strip_suffix(" spells each turn."))
            .and_then(|count| count.parse::<i32>().ok())
            .and_then(number_word)
    {
        *previous = format!(
            "You can cast spells only during your turn and you can cast no more than {count} spells each turn."
        );
        return out;
    }
    // "You may cast this card from your graveyard, but not from anywhere
    // else." (Haakon, Stromgald Scourge): a graveyard permission paired with a
    // graveyard-only cast restriction.
    if let Some(previous) = out.last_mut()
        && previous.as_str() == "You may cast this card from your graveyard."
        && line == "Cast this spell only if this card is in your graveyard."
    {
        *previous = "You may cast this card from your graveyard, but not from anywhere else.".to_string();
        return out;
    }
    // "Enchanted permanent can't attack, block, or crew Vehicles, and its
    // activated abilities can't be activated unless they're mana abilities."
    // (Bound in Gold)
    if let Some(previous) = out.last_mut()
        && let Some(subject) = ["Enchanted creature", "Enchanted permanent"]
            .into_iter()
            .find(|subject| previous.starts_with(&format!("{subject} can't ")))
        && previous.ends_with('.')
        && !previous.contains(['"', ';'])
        && !previous[..previous.len() - 1].contains(['.', ':'])
        && !previous.contains(" as long as ")
        && previous.contains(" crew ")
        && let Some(rest) = line.strip_prefix(&format!("Activated abilities of {} can't be activated", lowercase_first(subject)))
        && rest.strip_suffix('.').is_some_and(|body| !body.contains(['.', '"']))
    {
        let merged = format!(
            "{}, and its activated abilities can't be activated{rest}",
            previous.trim_end_matches('.')
        );
        *previous = merged;
        return out;
    }
    // "Enchanted creature gets +2/+2 as long as it's a Human. Otherwise, it
    // can't attack or block." (Bonds of Faith): complementary conditions.
    if let Some(previous) = out.last_mut()
        && let Some((subject, quality)) = ["Enchanted creature", "Equipped creature"].iter().find_map(|subject| {
            previous
                .strip_prefix(&format!("{subject} "))
                .and_then(|rest| rest.strip_suffix('.'))
                .and_then(|rest| rest.split_once(" as long as it's "))
                .map(|(_, quality)| (*subject, quality.to_string()))
        })
        && !quality.contains(['.', ',', '"'])
        && !previous.contains(". ")
        && let Some(predicate) = line
            .strip_prefix(&format!("{subject} "))
            .and_then(|rest| rest.strip_suffix(&format!(" unless it's {quality}.")))
        && !predicate.contains(['.', '"'])
        && !predicate.contains(" as long as ")
    {
        let merged = format!("{previous} Otherwise, it {predicate}.");
        *previous = merged;
        return out;
    }
    // "Enchanted creature has double strike as long as it's an enchantment.
    // Otherwise, prevent all damage that would be dealt by enchanted
    // creature." (Armament of Nyx)
    if let Some(previous) = out.last_mut()
        && let Some((quality, predicate)) = previous
            .strip_prefix("As long as enchanted creature is ")
            .and_then(|rest| rest.strip_suffix('.'))
            .and_then(|rest| rest.split_once(", it "))
        && !quality.contains(['.', ',', '"'])
        && !predicate.contains(['.', ',', '"'])
        && let Some(rule) = line.strip_suffix(&format!(" unless the permanent this Aura is attached to is {quality}."))
        && !rule.contains(['.', '"'])
    {
        let merged = format!(
            "Enchanted creature {predicate} as long as it's {quality}. Otherwise, {}.",
            lowercase_first(rule)
        );
        *previous = merged;
        return out;
    }
    out.push(line);
    out
}

/// "Max speed — [ability]" means the ability functions as long
/// as you have max speed, so a static gated on exactly that reads as the
/// max speed ability ("Max speed — Spells you cast cost {1} less to cast").
fn audit_max_speed_surface(line: String) -> String {
    const TRAILING: &str = " as long as you have max speed.";
    const LEADING: &str = "As long as you have max speed, ";
    if let Some(rest) = line.strip_suffix(TRAILING)
        && !rest.contains(" as long as ")
        && !rest.contains(". ")
        && !rest.contains('"')
        && !rest.starts_with("Max speed")
    {
        return format!("Max speed \u{2014} {rest}.");
    }
    if let Some(rest) = line.strip_prefix(LEADING)
        && !rest.contains(" as long as ")
    {
        return format!("Max speed \u{2014} {}", capitalize_first(rest));
    }
    line
}

/// "A deck can have any number of copies of this card" names the card as
/// printed: "A deck can have any number of cards named Hare Apparent".
fn audit_deck_copy_limit_surface(line: String, name: &str) -> String {
    if line == "A deck can have any number of copies of this card." {
        return format!("A deck can have any number of cards named {name}.");
    }
    if let Some(limit) = line
        .strip_prefix("A deck can have up to ")
        .and_then(|rest| rest.strip_suffix(" copies of this card."))
        .and_then(|limit| limit.parse::<u32>().ok())
    {
        let limit = ironsmith_core::cardinal_word(limit).unwrap_or_else(|| limit.to_string());
        return format!("A deck can have up to {limit} cards named {name}.");
    }
    line
}

/// Where a semantic-audit line came from: an ordinary per-ability render, or
/// the audit reading of a static ability (keywords tracked apart).
#[derive(Clone, Copy, PartialEq, Eq)]
enum AuditLine {
    Rendered,
    /// A static reading, with the index of the first ability of its printed
    /// line.
    Static(usize),
    Keyword(usize),
}

/// Static audit readings come from the engine without card context. Give them
/// the subjects the card's own types and enchant filter fix ("equipped
/// creature", "this creature"), print keywords as a keyword line, and run the
/// readings of each printed line that holds several statics through the
/// text-only surface merges. The merges see only audit readings, never
/// retained prose, and marker lines pass through untouched.
fn finish_audit_static_lines(def: &CardDefinition, lines: Vec<(AuditLine, String)>) -> Vec<String> {
    let subject = subject_for_card(&def.card);
    let attached = audit_attached_subject(def);
    let mut out = Vec::with_capacity(lines.len());
    let mut run: Vec<String> = Vec::new();
    let mut run_group = None;
    let flush = |run: &mut Vec<String>, out: &mut Vec<String>| match run.len() {
        0 => {}
        1 => out.append(run),
        _ => out.extend(audit_distinct_condition_list(audit_shared_condition_lines(
            join_audit_same_subject_static_lines(normalize_ast_surface_lines(std::mem::take(run))),
            subject,
        ))),
    };
    for (kind, line) in lines {
        let group = match kind {
            AuditLine::Static(group) | AuditLine::Keyword(group) => Some(group),
            AuditLine::Rendered => None,
        };
        if group.is_none()
            || group != run_group
            || line.contains(ironsmith_core::semantic_audit::UNVERIFIED_MARKER_PREFIX)
        {
            flush(&mut run, &mut out);
            run_group = None;
        }
        if group.is_none() || line.contains(ironsmith_core::semantic_audit::UNVERIFIED_MARKER_PREFIX) {
            // "As this land enters": a bare "this" names the card itself.
            out.push(
                line.replace("As this enters", &format!("As {subject} enters"))
                    .replace("When this enters", &format!("When {subject} enters")),
            );
            continue;
        }
        run_group = group;
        let line = audit_static_subject_surface(&line, subject, attached.as_deref());
        if matches!(kind, AuditLine::Keyword(_))
            || merge_passes::is_keyword_phrase(line.trim().trim_end_matches('.'))
        {
            let keyword = line.trim().trim_end_matches('.').to_string();
            // Simple keywords share one printed line ("Flying, vigilance").
            // A numbered keyword closes the list ("Trample, haste, annihilator 1").
            if (is_simple_audit_keyword(&keyword)
                || is_numbered_audit_keyword(&keyword)
                || is_mana_ward_audit_keyword_d5(&keyword)
                || is_triggered_list_audit_keyword_r5f(&keyword))
                && let Some(previous) = run.last_mut()
                && previous.split(", ").all(|part| {
                    is_simple_audit_keyword(part)
                        // "Trample, firebending 4, haste", "Menace, ward {2}, haste"
                        || (is_simple_audit_keyword(&keyword)
                            && (is_triggered_list_audit_keyword_r5f(&capitalize_first(part))
                                || is_mana_ward_audit_keyword_d5(&capitalize_first(part))))
                })
            {
                // "protection from black and from red"
                let last = previous.rsplit(", ").next().unwrap_or("").to_ascii_lowercase();
                if last.starts_with("protection from ")
                    && keyword.to_ascii_lowercase().starts_with("protection from ")
                {
                    // Keep a subtype's capital: "and from Zombies".
                    let quality = &keyword["protection from ".len()..];
                    previous.push_str(&format!(" and from {quality}"));
                    continue;
                }
                previous.push_str(", ");
                previous.push_str(&lowercase_first(&keyword));
            } else {
                run.push(keyword);
            }
            continue;
        }
        // CR 702.16n: the Aura exception follows the protection grant it
        // qualifies on the same printed line.
        if line.starts_with("This effect doesn't remove ")
            && let Some(previous) = run.last_mut()
        {
            previous.push(' ');
            previous.push_str(&line);
            continue;
        }
        // "Enchanted creature gets +2/+2 as long as it's an enchantment.
        // Otherwise, it gets -2/-2.": complementary conditions on one line.
        if let Some(previous) = run.last_mut()
            && let Some(merged) = audit_otherwise_pair(previous, &line)
        {
            *previous = merged;
            continue;
        }
        // "X as long as timing restriction: during combat" reads as the
        // window it names: "During combat, x." Restrictions sharing that
        // window and the subject "players can't" join as one rule.
        let line = audit_leading_timing_window(&line).unwrap_or(line);
        if let Some(previous) = run.last_mut()
            && let Some((window, rest)) = line.split_once(", players can't ")
            && window.starts_with("During ")
            && previous.starts_with(&format!("{window}, players can't "))
            && !previous.contains(" or ")
        {
            *previous = format!(
                "{} or {}",
                previous.trim_end_matches('.'),
                rest.replace("activate non-mana abilities", "activate abilities that aren't mana abilities")
            );
            continue;
        }
        run.push(line);
    }
    flush(&mut run, &mut out);
    let out = out
        .into_iter()
        .map(|line| audit_attached_condition_first(&line).unwrap_or(line))
        .map(|line| audit_your_turn_window(&line).unwrap_or(line))
        // CR 508.1d / 509.1c: both requirements on one creature.
        .map(|line| line.replace(" attacks each combat if able and blocks each combat if able", " attacks or blocks each combat if able"))
        .collect();
    collapse_audit_domain_landwalk(out, subject)
}

/// Five landwalk grants, each gated on controlling a land of its own basic
/// type, are one domain rule: "For each basic land type among lands you
/// control, this creature has landwalk of that type." (CR 702.14).
fn collapse_audit_domain_landwalk(lines: Vec<String>, subject: &str) -> Vec<String> {
    const TYPES: [&str; 5] = ["Plains", "Island", "Swamp", "Mountain", "Forest"];
    let expected = TYPES
        .iter()
        .map(|land| format!("{land}walk as long as you control {land}."))
        .collect::<Vec<_>>();
    let Some(start) = lines.windows(5).position(|window| window == expected.as_slice()) else {
        return lines;
    };
    let mut out = lines;
    out.splice(
        start..start + 5,
        [format!(
            "For each basic land type among lands you control, {subject} has landwalk of that type."
        )],
    );
    out
}

/// A static whose only condition is "it's your turn" reads as the window it
/// names, as printed: "During your turn, creatures you control have first
/// strike."; a prohibition keeps it trailing: "Your opponents can't cast
/// spells during your turn."
fn audit_your_turn_window(line: &str) -> Option<String> {
    let body = line.strip_suffix('.')?;
    if let Some(rest) = body.strip_prefix("As long as it's your turn, ")
        && !rest.contains(" as long as ")
    {
        return Some(format!("During your turn, {rest}."));
    }
    if body.contains(". ")
        || body.contains(['"', ':'])
        || body.starts_with("If ")
        || body.starts_with("During ")
        || body.starts_with("Cast this spell only")
    {
        return None;
    }
    let (head, trailing) = match body.strip_suffix(" as long as it's your turn") {
        Some(head) => (head, false),
        None => (body.strip_suffix(" during your turn")?, true),
    };
    if head.contains(" as long as ") || head.contains(" during ") || head.contains(" unless ") {
        return None;
    }
    if head.contains(" can't ") {
        return (!trailing).then(|| format!("{head} during your turn."));
    }
    Some(format!("During your turn, {}.", lowercase_first(head)))
}

/// Readings of one printed line gated on the same condition read as one
/// sentence led by it: "As long as there are seven or more cards in your
/// graveyard, this creature gets +1/+1, is black, and has ..."; a clause on
/// the card's own subject after another drops the repeated subject.
fn audit_shared_condition_lines(lines: Vec<String>, subject: &str) -> Vec<String> {
    fn split(line: &str) -> Option<(&str, &str)> {
        let body = line.trim().strip_suffix('.')?;
        if body.contains(". ") || body.contains('"') || body.starts_with("If ") {
            return None;
        }
        // A leading condition joins a neighbour's trailing one (Lieutenant:
        // "As long as you control your commander, this creature gets +2/+2
        // and other creatures you control get +2/+2 and have trample").
        if let Some(rest) = body.strip_prefix("As long as ") {
            let (condition, clause) = rest.split_once(", ")?;
            return (!clause.is_empty() && !clause.contains(" as long as ") && !condition.contains(" as long as "))
                .then_some((clause, condition));
        }
        let (clause, condition) = body.split_once(" as long as ")?;
        // Station thresholds are laid out by `compact_station_threshold_lines`.
        if condition.contains("charge counters on this") {
            return None;
        }
        (!condition.contains(" as long as ") && !clause.is_empty()).then_some((clause, condition))
    }
    let head = format!("{} ", capitalize_first(subject));
    let head_lower = format!("{} ", lowercase_first(subject));
    let mut out = Vec::with_capacity(lines.len());
    let mut index = 0;
    while index < lines.len() {
        let Some((_, condition)) = split(&lines[index]) else {
            out.push(lines[index].clone());
            index += 1;
            continue;
        };
        let mut end = index + 1;
        while end < lines.len() && split(&lines[end]).is_some_and(|(_, other)| other == condition) {
            end += 1;
        }
        if end - index < 2 {
            out.push(lines[index].clone());
            index += 1;
            continue;
        }
        let mut clauses = Vec::new();
        let mut previous_own = false;
        for line in &lines[index..end] {
            let (clause, _) = split(line).expect("checked above");
            match clause.strip_prefix(&head).or_else(|| clause.strip_prefix(head_lower.as_str())) {
                Some(predicate) if previous_own => clauses.push(predicate.to_string()),
                Some(_) => {
                    previous_own = true;
                    clauses.push(lowercase_first(clause));
                }
                None => {
                    previous_own = false;
                    clauses.push(lowercase_first(clause));
                }
            }
        }
        let list = match clauses.as_slice() {
            [first, second] => format!("{first} and {second}"),
            [rest @ .., last] => format!("{}, and {last}", rest.join(", ")),
            [] => String::new(),
        };
        out.push(format!("As long as {condition}, {list}."));
        index = end;
    }
    out
}

/// One printed line granting each of several predicates under its own
/// condition (Tek, Tribal Golem) reads as one list on the shared subject:
/// "This creature gets +0/+2 as long as you control a Plains, has flying as
/// long as you control an Island, ..., and has trample as long as you control
/// a Forest."
fn audit_distinct_condition_list(lines: Vec<String>) -> Vec<String> {
    // (subject, verb, predicate, condition). A quoted predicate keeps its
    // closing quote; one cut with its period ("...this creature." As long
    // as) is closed again when the list is built.
    fn split(line: &str) -> Option<(&str, &str, &str, &str)> {
        let body = line.trim().strip_suffix('.')?;
        let (clause, condition) = if let Some((clause, condition)) = body.split_once("\" as long as ") {
            (&body[..clause.len() + 1], condition)
        } else if let Some(split) = body.split_once(".\" As long as ") {
            split
        } else if !body.contains('"') {
            body.split_once(" as long as ")?
        } else {
            return None;
        };
        if condition.contains(" as long as ") || condition.contains('"') || condition.contains(". ") {
            return None;
        }
        let (subject, verb, predicate) = [" has ", " gets "]
            .iter()
            .filter_map(|verb| clause.split_once(*verb).map(|(subject, predicate)| (subject, verb.trim(), predicate)))
            .min_by_key(|(subject, _, _)| subject.len())?;
        if subject.is_empty() || subject.contains('"') || predicate.is_empty() {
            return None;
        }
        Some((subject, verb, predicate, condition))
    }
    if lines.len() < 3 {
        return lines;
    }
    let mut parts = Vec::with_capacity(lines.len());
    for line in &lines {
        let Some(part) = split(line) else {
            return lines;
        };
        parts.push(part);
    }
    let subject = parts[0].0;
    if parts.iter().any(|(other, ..)| *other != subject) {
        return lines;
    }
    let mut conditions = parts.iter().map(|(.., condition)| *condition).collect::<Vec<_>>();
    conditions.sort_unstable();
    conditions.dedup();
    if conditions.len() != parts.len() {
        return lines;
    }
    let all_has = parts.iter().all(|(_, verb, ..)| *verb == "has");
    let items = parts
        .iter()
        .enumerate()
        .map(|(index, (_, verb, predicate, condition))| {
            let quote = if predicate.starts_with('"') && !predicate.ends_with('"') { "\"" } else { "" };
            if index == 0 || !all_has {
                format!("{verb} {predicate}{quote} as long as {condition}")
            } else {
                format!("{predicate}{quote} as long as {condition}")
            }
        })
        .collect::<Vec<_>>();
    let (last, rest) = items.split_last().expect("at least three items");
    vec![format!("{subject} {}, and {last}.", rest.join(", "))]
}

/// Subjects naming the object a static's source is attached to.
const AUDIT_ATTACHED_SUBJECTS: [&str; 6] = [
    "Enchanted creature",
    "Equipped creature",
    "Enchanted permanent",
    "Enchanted land",
    "Enchanted artifact",
    "Fortified land",
];

/// "S P1 as long as s is C." + "S P2 as long as s isn't C." on one printed
/// line read as "S P1 as long as it's C. Otherwise, it P2."
fn audit_otherwise_pair(first: &str, second: &str) -> Option<String> {
    for subject in AUDIT_ATTACHED_SUBJECTS {
        let lower = subject.to_ascii_lowercase();
        let Some((p1, c1)) = first
            .strip_prefix(subject)
            .and_then(|rest| rest.strip_prefix(' '))
            .and_then(|rest| rest.strip_suffix('.'))
            .and_then(|rest| rest.split_once(&format!(" as long as {lower} is ")))
        else {
            continue;
        };
        let (p2, c2) = second
            .strip_prefix(subject)?
            .strip_prefix(' ')?
            .strip_suffix('.')?
            .split_once(&format!(" as long as {lower} isn't "))?;
        if c1 != c2 || p1.contains(" as long as ") || p2.contains(" as long as ") {
            return None;
        }
        return Some(format!("{subject} {p1} as long as it's {c1}. Otherwise, it {p2}."));
    }
    None
}

/// "Equipped creature has hexproof as long as equipped creature is
/// legendary." reads condition first, as printed: "As long as equipped
/// creature is legendary, it has hexproof."
fn audit_attached_condition_first(line: &str) -> Option<String> {
    let body = line.strip_suffix('.')?;
    for subject in AUDIT_ATTACHED_SUBJECTS {
        let lower = subject.to_ascii_lowercase();
        let Some((predicate, condition)) = body
            .strip_prefix(subject)
            .and_then(|rest| rest.strip_prefix(' '))
            .and_then(|rest| rest.split_once(&format!(" as long as {lower} ")))
        else {
            continue;
        };
        if predicate.contains(" as long as ")
            || condition.contains(" as long as ")
            || predicate.contains(['.', '"'])
            || condition.contains(['.', '"'])
        {
            return None;
        }
        return Some(format!("As long as {lower} {condition}, it {predicate}."));
    }
    None
}

/// "deals X damage to that player, where X is the number of cards in that
/// player's hand": once a sentence has named "that player", a later
/// possessive of the same player reads as Oracle's pronoun "their".
fn audit_repeat_player_possessive(line: &str) -> String {
    const NAME: &str = "that player";
    const POSSESSIVE: &str = "that player's ";
    if line.matches(NAME).count() < 2 {
        return line.to_string();
    }
    line.split(". ")
        .map(|sentence| {
            let Some(first) = sentence.find(NAME) else {
                return sentence.to_string();
            };
            // Quoted abilities keep their own wording.
            if sentence.contains('"') {
                return sentence.to_string();
            }
            let head_end = first + NAME.len();
            format!(
                "{}{}",
                &sentence[..head_end],
                sentence[head_end..].replace(POSSESSIVE, "their ")
            )
        })
        .collect::<Vec<_>>()
        .join(". ")
}

/// "you may pay {X}. If you do, draw that many cards": the amount paid is
/// the X just announced, which Oracle names "X" ("draw X cards").
fn audit_paid_x_reference(line: &str) -> String {
    let Some(pay_at) = line.find("pay {X}") else {
        return line.to_string();
    };
    let after_pay = &line[pay_at..];
    // The cost runs to the end of its sentence.
    let Some(cost_end) = after_pay.find(". ") else {
        return line.to_string();
    };
    let rest = &after_pay[cost_end + 2..];
    let Some(lead) = ["If you do, ", "When you do, "]
        .iter()
        .find(|lead| rest.starts_with(**lead))
    else {
        return line.to_string();
    };
    let body_start = pay_at + cost_end + 2 + lead.len();
    let body_end = line[body_start..]
        .find(". ")
        .map_or(line.len(), |end| body_start + end);
    let body = &line[body_start..body_end];
    if !body.contains("that many ") && !body.contains("that much ") {
        return line.to_string();
    }
    let body = body.replace("that many ", "X ").replace("that much ", "X ");
    format!("{}{}{}", &line[..body_start], body, &line[body_end..])
}

/// The gift (CR 702.174) a promised-gift delivery performs: "draws a card"
/// -> "a card".
fn audit_gift_for_delivery(action: &str) -> Option<&'static str> {
    Some(match action {
        "draws a card" => "a card",
        "creates a Food token" => "a Food",
        "creates a Treasure token" => "a Treasure",
        "creates a 1/1 blue Fish creature token" => "a Fish",
        "creates a tapped 1/1 blue Fish creature token" => "a tapped Fish",
        "creates an 8/8 blue Octopus creature token" => "an Octopus",
        "takes an extra turn after this one" => "an extra turn",
        _ => return None,
    })
}

/// "Players can't cast spells as long as timing restriction: during combat."
/// -> "During combat, players can't cast spells."
fn audit_leading_timing_window(line: &str) -> Option<String> {
    let body = line.trim().trim_end_matches('.');
    let (head, window) = body.rsplit_once(" as long as timing restriction: ")?;
    let window = window.to_ascii_lowercase();
    if !window.starts_with("during ") || head.contains(". ") || head.is_empty() {
        return None;
    }
    Some(format!(
        "{}, {}.",
        capitalize_first(&window),
        lowercase_first(head)
    ))
}

/// Static readings of one printed line that each restate the same subject
/// re-join as that line's predicate list: "Equipped creature gets +4/+2." +
/// "Equipped creature doesn't untap during its controller's untap step." ->
/// "Equipped creature gets +4/+2 and doesn't untap during its controller's
/// untap step." Only readings already grouped by printed line reach here, and
/// each predicate is kept verbatim.
fn join_audit_same_subject_static_lines(lines: Vec<String>) -> Vec<String> {
    const VERBS: [&str; 21] = [
        "gets", "get", "has", "have", "loses", "lose", "doesn't", "don't", "can't", "can",
        "attacks", "attack", "blocks", "block", "is", "are", "enters", "enter",
        // "Equipped creature gets +3/+1 and must be blocked by an Eldrazi if
        // able"; "Spells with flash you cast cost {1} less to cast and can't
        // be countered".
        "must", "cost", "costs",
    ];
    // (leading "During your turn, ", subject, verb, predicate)
    fn split(line: &str) -> Option<(&'static str, &str, &'static str, &str)> {
        let body = line.trim().strip_suffix('.')?;
        if body.contains(['"', ':']) || body.contains(". ") || body.contains(" as long as ") {
            return None;
        }
        let (prefix, body) = match body.strip_prefix("During your turn, ") {
            Some(rest) => ("During your turn, ", rest),
            None => ("", body),
        };
        let (at, verb) = VERBS
            .iter()
            .filter_map(|verb| body.find(&format!(" {verb} ")).map(|at| (at, *verb)))
            .min_by_key(|(at, _)| *at)?;
        let subject = &body[..at];
        if subject.is_empty() || subject.contains(',') || subject.starts_with("If ") {
            return None;
        }
        Some((prefix, subject, verb, &body[at + verb.len() + 2..]))
    }
    fn singular(subject: &str) -> bool {
        let lower = subject.to_ascii_lowercase();
        ["each ", "this ", "enchanted ", "equipped ", "fortified ", "it "].iter().any(|head| lower.starts_with(head))
            || lower == "it"
    }
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    for line in lines {
        let joined = out.last().and_then(|left| {
            let (prefix, left_subject, left_verb, left_predicate) = split(left)?;
            let right = line.trim().strip_suffix('.')?;
            if right.contains(['"', ':']) || right.contains(". ") || right.contains(" as long as ") {
                return None;
            }
            let right = if prefix.is_empty() {
                right.to_string()
            } else {
                right.strip_suffix(" during your turn")?.to_string()
            };
            let lower_subject = lowercase_first(left_subject);
            let mut comma = false;
            let predicate = if right == format!("Activated abilities of {lower_subject} can't be activated") {
                comma = left_verb != "doesn't";
                let pronoun = if singular(left_subject) { "its" } else { "their" };
                format!("{pronoun} activated abilities can't be activated")
            } else if right == format!("All creatures able to block {lower_subject} do so")
                && singular(left_subject)
                && !left_subject.starts_with("Each ")
            {
                // Indrik Umbra: "..., and all creatures able to block it do so."
                comma = true;
                "all creatures able to block it do so".to_string()
            } else {
                let (_, right_subject, right_verb, right_predicate) = split(&format!("{right}."))
                    .map(|(_, subject, verb, predicate)| ((), subject.to_string(), verb, predicate.to_string()))?;
                let strip_all = |subject: &str| subject.strip_prefix("All ").unwrap_or(subject).to_ascii_lowercase();
                let same = right_subject == left_subject
                    || strip_all(&right_subject) == strip_all(left_subject)
                    // "Each other Human you control gets ..." + "Other Humans
                    // you control have ...".
                    || left_subject.strip_prefix("Each ").is_some_and(|rest| {
                        capitalize_first(&pluralize_relative_object_phrase(rest)) == right_subject
                    });
                if !same {
                    return None;
                }
                // "This artifact enters with a +1/+1 counter, a flying
                // counter, and a shield counter on it" (Agent's Toolkit).
                let counters = |predicate: &str| {
                    let inner = predicate.strip_prefix("with ")?.strip_suffix(" on it")?;
                    let items = inner
                        .split(", and ")
                        .flat_map(|part| part.split(", "))
                        .flat_map(|part| part.split(" and "))
                        .map(str::to_string)
                        .collect::<Vec<_>>();
                    items
                        .iter()
                        .all(|item| item.ends_with(" counter") || item.ends_with(" counters"))
                        .then_some(items)
                };
                if left_verb == "enters"
                    && right_verb == "enters"
                    && let (Some(mut items), Some(more)) = (counters(left_predicate), counters(right_predicate.as_str()))
                {
                    items.extend(more);
                    let list = match items.as_slice() {
                        [first, second] => format!("{first} and {second}"),
                        [init @ .., last] => format!("{}, and {last}", init.join(", ")),
                        [] => return None,
                    };
                    return Some(format!("{prefix}{left_subject} enters with {list} on it."));
                }
                let verb = match right_verb {
                    "have" if singular(left_subject) => "has",
                    "attack" if singular(left_subject) => "attacks",
                    "block" if singular(left_subject) => "blocks",
                    "don't" if singular(left_subject) => "doesn't",
                    "lose" if singular(left_subject) => "loses",
                    "get" if singular(left_subject) => "gets",
                    verb => verb,
                };
                // "All creatures lose flying and islandwalk": a repeated
                // keyword verb is said once.
                if verb == left_verb
                    && matches!(verb, "has" | "have" | "lose" | "loses")
                    && !left_predicate.contains(" and ")
                    && merge_passes::is_keyword_phrase(left_predicate)
                    && merge_passes::is_keyword_phrase(&right_predicate)
                {
                    right_predicate
                } else {
                    format!("{verb} {right_predicate}")
                }
            };
            let joiner = if comma { ", and " } else { " and " };
            Some(format!("{}{joiner}{predicate}.", left.trim().trim_end_matches('.')))
        });
        match joined {
            Some(joined) => *out.last_mut().unwrap() = joined,
            None => out.push(line),
        }
    }
    out
}

/// A subject with no determiner naming a kind of object ("Sliver",
/// "Creature with flying"), which a rule applies to every such object.
fn is_bare_singular_subject(head: &str) -> bool {
    let Some(first) = head.split(' ').next() else {
        return false;
    };
    let kinds = [
        "Creature", "Artifact", "Land", "Enchantment", "Planeswalker", "Permanent",
        "Nonland", "Noncreature", "Nonartifact", "Nonbasic", "Legendary", "Basic",
    ];
    // "Nonblack creature", "Non-Wall creature": a negated quality.
    if kinds.contains(&first) || (first.starts_with("Non") && first.len() > 3) {
        return !head.contains(" you ") && !head.ends_with(" you") && !head.ends_with('s');
    }
    // "Sliver creature", "White creature": a subtype or color qualifying
    // the creature noun.
    (!head.contains(' ') || is_qualified_creature_subject(head))
        && first.chars().next().is_some_and(|ch| ch.is_ascii_uppercase())
        && !first.ends_with('s')
        && ![
            "It", "This", "That", "Each", "All", "Other", "Another", "You", "Target",
            "Enchanted", "Equipped", "Fortified", "The", "A", "An",
            // Mass nouns and variables are not kinds of object.
            "Damage", "Mana", "Life", "X",
        ]
        .contains(&first)
}

/// "Sliver creature": one capitalized qualifier before "creature".
fn is_qualified_creature_subject(head: &str) -> bool {
    matches!(
        head.split(' ').collect::<Vec<_>>().as_slice(),
        [qualifier, "creature"] if qualifier.chars().next().is_some_and(|ch| ch.is_ascii_uppercase())
    )
}

/// The number a counter-entry reading names: "a" / "two" / "12".
fn audit_count_word(word: &str) -> Option<u32> {
    if matches!(word, "a" | "an") {
        return Some(1);
    }
    word.parse().ok().or_else(|| {
        (1..=30u32).find(|n| ironsmith_core::cardinal_word(*n).as_deref() == Some(word))
    })
}

/// "Enters the battlefield with three time counters on it." -> 3 for "time".
fn audit_entry_counters(line: &str, kind: &str) -> Option<u32> {
    let rest = line.strip_prefix("Enters the battlefield with ").or_else(|| {
        line.strip_prefix("This ")
            .and_then(|rest| rest.split_once(" enters with "))
            .filter(|(noun, _)| !noun.contains(' '))
            .map(|(_, rest)| rest)
    })?;
    let rest = rest
        .strip_suffix(&format!(" {kind} counters on it."))
        .or_else(|| rest.strip_suffix(&format!(" {kind} counter on it.")))?;
    audit_count_word(rest)
}

/// Keyword abilities whose whole executable expansion is present read as the
/// keyword. Each pattern matches every line of the expansion and reads the
/// parameter back from those lines, so a partial or altered expansion stays
/// spelled out.
fn collapse_audit_keyword_expansions(lines: Vec<String>) -> Vec<String> {
    let has_line = |prefix: &str| lines.iter().any(|line| line.starts_with(prefix));
    let partner_with = lines
        .iter()
        .find_map(|line| line.trim_end_matches('.').strip_prefix("Partner with ").map(str::to_string));
    let dash = has_line("Dash ");
    let suspend = has_line("Suspend ");
    let reconfigure_cost = lines
        .iter()
        .find_map(|line| line.trim_end_matches('.').strip_prefix("Reconfigure ").map(str::to_string));
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut idx = 0;
    while idx < lines.len() {
        let line = lines[idx].as_str();
        let next = lines.get(idx + 1).map(String::as_str).unwrap_or("");
        let third = lines.get(idx + 2).map(String::as_str).unwrap_or("");
        let vanishing_upkeep = |text: &str| {
            text.starts_with("At the beginning of your upkeep, if this ")
                && text.ends_with(" has a time counter on it, remove a time counter from it.")
        };
        let vanishing_sacrifice = |text: &str| {
            text.starts_with("When the last time counter is removed from this ")
                && text.contains(", sacrifice this ")
        };
        if let Some(n) = audit_entry_counters(line, "time")
            && vanishing_upkeep(next)
            && vanishing_sacrifice(third)
        {
            out.push(format!("Vanishing {n}"));
            idx += 3;
            continue;
        }
        if vanishing_upkeep(line) && vanishing_sacrifice(next) {
            out.push("Vanishing".to_string());
            idx += 2;
            continue;
        }
        // A bare keyword-identity marker carries no typed content (its
        // reading is the same for every marker). Next to the full typed
        // expansion of rampage (CR 702.23a) or decayed (CR 702.147a) the
        // expansion is what reads, as the keyword.
        const BARE_KEYWORD_MARKER: &str =
            "\u{27e6}unverified LabelDependentStatic: StaticAbility { id: Some\u{2039}KeywordMarker\u{203a} }\u{27e7}.";
        if line == BARE_KEYWORD_MARKER
            && let Some(rest) = next
                .strip_prefix("Whenever this creature becomes blocked, this creature gets +X/+X until end of turn, where X is ")
                .and_then(|rest| rest.strip_suffix("the number of blockers beyond the first."))
            && let Some(amount) = match rest {
                "" => Some(1),
                multiple => multiple.strip_suffix(" times ").and_then(|n| n.parse::<u32>().ok()),
            }
        {
            // Rampage's older templating shares a line with the simple
            // keywords before it: "Trample; rampage 2".
            match out.last_mut() {
                Some(previous) if is_simple_audit_keyword(previous.trim_end_matches('.')) => {
                    let joined = format!("{}; rampage {amount}", previous.trim_end_matches('.'));
                    *previous = joined;
                }
                _ => out.push(format!("Rampage {amount}")),
            }
            idx += 2;
            continue;
        }
        if line == BARE_KEYWORD_MARKER
            && next == "This creature can't block."
            && third == "Whenever this creature attacks, at this turn's next end of combat, sacrifice this creature."
        {
            out.push("Decayed".to_string());
            idx += 3;
            continue;
        }
        // One once-each-turn graveyard permission for lands and one for
        // each other permanent type (CR 110.4) is "a land and a permanent
        // spell of each permanent type".
        const ONCE_GRAVEYARD_PERMISSIONS: [&str; 6] = [
            "Once during each of your turns, you may play lands from your graveyard.",
            "Once during each of your turns, you may cast artifact spells from your graveyard.",
            "Once during each of your turns, you may cast creature spells from your graveyard.",
            "Once during each of your turns, you may cast enchantment spells from your graveyard.",
            "Once during each of your turns, you may cast planeswalker spells from your graveyard.",
            "Once during each of your turns, you may cast battle spells from your graveyard.",
        ];
        if ONCE_GRAVEYARD_PERMISSIONS
            .iter()
            .enumerate()
            .all(|(offset, expected)| lines.get(idx + offset).is_some_and(|line| line == expected))
        {
            out.push(
                "During each of your turns, you may play a land and cast a permanent spell of each permanent type from your graveyard."
                    .to_string(),
            );
            idx += ONCE_GRAVEYARD_PERMISSIONS.len();
            continue;
        }
        // Impending N—cost (CR 702.176a): the alternative cost plus its
        // three rules abilities (time counters on entry, not a creature
        // while it has one, end-step removal), each gated on that cost.
        if let Some(cost) = line.trim_end_matches('.').strip_prefix("Impending ")
            && !cost.contains('\u{2014}')
            && let Some(time) = next
                .strip_suffix(" as long as this spell's impending cost was paid.")
                .and_then(|entry| audit_entry_counters(&format!("{entry}."), "time"))
            && third.starts_with("This card isn't a creature as long as this spell's impending cost was paid and ")
            && third.contains(" time counter")
            && lines.get(idx + 3).is_some_and(|fourth| {
                fourth.starts_with(
                    "At the beginning of your end step, if this spell's impending cost was paid and this ",
                ) && fourth.ends_with(" has a time counter on it, remove a time counter from it.")
            })
        {
            out.push(format!("Impending {time}\u{2014}{cost}"));
            idx += 4;
            continue;
        }
        if let Some(n) = audit_entry_counters(line, "fade")
            && next.starts_with(
                "At the beginning of your upkeep, remove a fade counter from it. If that doesn't happen, sacrifice this ",
            )
        {
            out.push(format!("Fading {n}"));
            idx += 2;
            continue;
        }
        if let Some(n) = audit_entry_counters(line, "+1/+1") {
            if next.starts_with("When this ")
                && next.contains(" dies, tag the triggering object as 'modular_triggering_object', then you may put the number of +1/+1 counters on the tagged object 'modular_triggering_object' +1/+1 counters on target artifact creature")
            {
                out.push(format!("Modular {n}"));
                idx += 2;
                continue;
            }
            if next.starts_with("Whenever another creature enters, if this ")
                && next.contains(" has a +1/+1 counter on it, tag the triggering object as 'graft_entered_creature', then you may move a +1/+1 counter from this ")
            {
                out.push(format!("Graft {n}"));
                idx += 2;
                continue;
            }
        }
        if line == "This creature enters with X +1/+1 counters on it."
            && next == "When this creature enters, if X is 5 or more, draw a card."
        {
            out.push("Ravenous".to_string());
            idx += 2;
            continue;
        }
        const RIOT: [&str; 3] = [
            "As this creature enters, choose one \u{2014}",
            "\u{2022} Put a +1/+1 counter on this creature.",
            "\u{2022} This creature gains Haste.",
        ];
        if line == RIOT.join("\n") {
            out.push("Riot".to_string());
            idx += 1;
            continue;
        }
        if [line, next, third] == RIOT {
            out.push("Riot".to_string());
            idx += 3;
            continue;
        }
        if line.starts_with("As this creature attacks, you may tap a nonattacking creature you control without summoning sickness. When you do, this creature gets +1/+0 for each the tagged object 'enlisted_creature''s power until end of turn") {
            out.push("Enlist".to_string());
            idx += 1;
            continue;
        }
        // A permanent's gift delivery trigger is part of its Gift keyword
        // (CR 702.174): drop it only when it delivers exactly the gift the
        // keyword line names.
        if let Some(rest) = line
            .strip_prefix("When this ")
            .and_then(|rest| rest.split_once(" enters, if the gift was promised, the chosen player "))
            .map(|(_, rest)| rest.trim_end_matches('.'))
            && let Some(gift) = audit_gift_for_delivery(rest)
            && lines.iter().any(|other| other.trim_end_matches('.') == format!("Gift {gift}"))
        {
            idx += 1;
            continue;
        }
        // Champion's linked return trigger is part of the keyword (CR
        // 702.72a); the ETB half already reads as "Champion a X".
        if line.starts_with("Champion ")
            && !line.trim_end_matches('.').contains(". ")
            && next.starts_with("When this ")
            && next.ends_with(
                " leaves the battlefield, return the exiled card to the battlefield under its owner's control.",
            )
        {
            out.push(line.trim_end_matches('.').to_string());
            idx += 2;
            continue;
        }
        // Suspend's two time-counter triggers belong to its keyword line.
        if suspend
            && (vanishing_upkeep(line)
                || (line.starts_with("When the last time counter is removed from this ")
                    && line.ends_with("you may cast this card from exile without paying its mana cost.")))
        {
            idx += 1;
            continue;
        }
        // Aftermath's cast restriction and exile replacement (CR 702.127).
        if line.trim_end_matches('.') == "Aftermath" {
            out.push("Aftermath".to_string());
            idx += 1;
            while idx < lines.len()
                && matches!(
                    lines[idx].trim_end_matches('.'),
                    "Cast this spell only if this card is in your graveyard"
                        | "If this spell was cast from your graveyard, exile it instead of putting it anywhere else any time it would leave the stack"
                )
            {
                idx += 1;
            }
            continue;
        }
        // Partner with's tutor trigger and Dash's haste belong to their
        // keyword lines (CR 702.124, 702.109).
        if let Some(partner) = &partner_with
            && line
                .strip_prefix("When this ")
                .and_then(|rest| rest.split_once(' '))
                .is_some_and(|(_, rest)| rest == format!("enters, target player may search their library for a card named {partner}, reveals it, puts it into hand, then shuffles."))
        {
            idx += 1;
            continue;
        }
        if dash && line == "This creature has haste as long as this spell's dash cost was paid." {
            idx += 1;
            continue;
        }
        // A granted riot choice (CR 702.136a): "Nontoken creatures you
        // control have riot."
        {
            let mut parts = line.split('\n').map(str::to_string).collect::<Vec<_>>();
            let consumed = if parts.len() == 1 {
                parts.push(next.to_string());
                parts.push(third.to_string());
                3
            } else {
                1
            };
            let granted_riot = parts.len() == 3
                && parts[0].split_once(" As this ").is_some_and(|(head, rest)| {
                    (head.ends_with(" have") || head.ends_with(" has"))
                        && rest.ends_with(" enters, choose one \u{2014}")
                        && {
                            let noun = rest.trim_end_matches(" enters, choose one \u{2014}");
                            parts[1] == format!("\u{2022} Put a +1/+1 counter on this {noun}.")
                                && parts[2] == format!("\u{2022} This {noun} gains Haste.")
                        }
                });
            if granted_riot && let Some((head, _)) = parts[0].split_once(" As this ") {
                out.push(format!("{head} riot."));
                idx += consumed;
                continue;
            }
        }
        // "Enchanted creature has protection from white. This effect doesn't
        // remove this Aura." (CR 702.16 exception on the same printed line.)
        if line == "Protection doesn't remove this Aura."
            && let Some(previous) = out.last_mut()
            && previous.contains("protection from ")
        {
            if !previous.ends_with('.') {
                previous.push('.');
            }
            previous.push_str(" This effect doesn't remove this Aura.");
            idx += 1;
            continue;
        }
        // A flash permission paired with its cleanup-step sacrifice (Parapet,
        // Spider Climb): the printed "as though it had flash" line.
        if line.starts_with(
            "If you cast it and you cast this spell any time a sorcery couldn't have been cast, sacrifice this ",
        ) && line.ends_with(" at the beginning of the next cleanup step.")
            && (next.trim_end_matches('.') == "Flash"
                || out.last().is_some_and(|previous| previous.trim_end_matches('.') == "Flash"))
        {
            if next.trim_end_matches('.') == "Flash" {
                idx += 1;
            } else {
                out.pop();
            }
            out.push(
                "You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step."
                    .to_string(),
            );
            idx += 1;
            continue;
        }
        // The Class level reminder is reminder text, not a rule.
        if line == "(Gain the next level as a sorcery to add its ability.)" {
            idx += 1;
            continue;
        }
        if let Some(collapsed) = collapse_decayed_token(line) {
            out.push(collapsed);
            idx += 1;
            continue;
        }
        // Reconfigure's unattach ability belongs to its keyword line.
        if let Some(cost) = &reconfigure_cost
            && line == format!("{cost}: Unattach this creature. Activate only as a sorcery and only if this creature is attached to a creature.")
        {
            idx += 1;
            continue;
        }
        let bare = line.strip_prefix("This creature has ").unwrap_or(line);
        if let Some((_, rest)) = bare.split_once(" enters, ")
            && bare.starts_with("As this ")
            && let Some(keyword) = ["devour ", "amplify "].iter().find(|k| rest.starts_with(**k))
        {
            out.push(capitalize_first(&format!("{keyword}{}", rest[keyword.len()..].trim_end_matches('.'))));
            idx += 1;
            continue;
        }
        if let Some(cost) = line.strip_prefix("Flashback\u{2014}").and_then(|rest| rest.strip_suffix('.'))
            && cost.starts_with('{')
            && cost.ends_with('}')
            && !cost.contains(' ')
        {
            out.push(format!("Flashback {cost}"));
            idx += 1;
            continue;
        }
        // A capitalized subtype names the object: "this Spacecraft".
        let mut line = audit_protection_list(&dedupe_you_control(line));
        for subtype in ["Spacecraft", "Vehicle", "Equipment", "Aura", "Saga", "Class", "Case"] {
            line = line.replace(&format!("this {}", subtype.to_ascii_lowercase()), &format!("this {subtype}"));
        }
        let line = line.as_str();
        // Keyword lines print without a closing period ("Outlast {W}").
        let trimmed = line.trim_end_matches('.');
        let keyword_with_cost = matches!(
            trimmed.split(' ').collect::<Vec<_>>().as_slice(),
            [keyword, cost]
                if keyword.chars().next().is_some_and(|ch| ch.is_ascii_uppercase())
                    && keyword.chars().all(|ch| ch.is_ascii_alphabetic())
                    && (cost.starts_with('{') && cost.ends_with('}')
                        || cost.chars().all(|ch| ch.is_ascii_digit()))
        );
        if trimmed.len() + 1 == line.len()
            && (merge_passes::is_keyword_phrase(trimmed) || keyword_with_cost)
            && !trimmed.contains(". ")
        {
            out.push(trimmed.to_string());
            idx += 1;
            continue;
        }
        out.push(line.to_string());
        idx += 1;
    }
    // Oracle reads "enters" for the present-tense battlefield entry.
    out.into_iter()
        .map(|line| {
            let line = line
                .replace(" enters the battlefield", " enters")
                .replace(" a a ", " a ")
                .replace(" a an ", " an ")
                .replace(" a hourglass ", " an hourglass ");
            let line = audit_paid_x_reference(&line);
            let line = audit_repeat_player_possessive(&line);
            // An Equipment token's attachment grant names the object it is
            // attached to: "equipped creature" (CR 301.5).
            let line = if line.contains(" Equipment artifact token") {
                line.replace("\"The object this is attached to ", "\"Equipped creature ")
            } else {
                line
            };
            // An enchanted permanent can only reach a graveyard from the
            // battlefield; Oracle leaves that origin implicit.
            let line = if line.starts_with("When enchanted ") || line.starts_with("Whenever enchanted ") {
                line.replacen(
                    " is put into a graveyard from the battlefield,",
                    " is put into a graveyard,",
                    1,
                )
            } else {
                line
            };
            // "that player" names the active player of an each-combat
            // trigger, which Oracle words as "on each player's turn".
            match line.strip_prefix("At the beginning of each combat, ") {
                Some(rest) if rest.contains("that player") => {
                    format!("At the beginning of combat on each player's turn, {rest}")
                }
                _ => line,
            }
        })
        .map(|line| audit_activated_line_rephrasings(&line))
        .map(|line| audit_ability_effect_rephrasings(&line))
        .map(|line| audit_land_animation_surface(&line).unwrap_or(line))
        .map(|line| audit_inline_keyword_marker_grants(&line))
        .map(|line| audit_static_rule_rephrasings(&line))
        .collect()
}

/// Pure re-phrasings of static and spell rule readings into the wording
/// Oracle prints for the same rule.
fn audit_static_rule_rephrasings(line: &str) -> String {
    let mut line = line
        // "The next creature spell you cast this turn can't be countered."
        .replace(" has this spell can't be countered", " can't be countered")
        // "had two or more lands enter the battlefield under their control
        // this turn" (Lavaball Trap, Smuggler's Share).
        .replace(" enter under their control this turn", " enter the battlefield under their control this turn")
        .replace(" enter under your control this turn", " enter the battlefield under your control this turn")
        // "two or more All Vampires": the count already quantifies the set.
        .replace(" or more All ", " or more ")
        // The creature an Aura is attached to is "enchanted creature".
        .replace(" target creature with this Aura attached to it ", " target enchanted creature ")
        // A cost rendered as its own "Pay ..." instruction after "pays".
        .replace(" pays Pay ", " pays ")
        .replace(" by paying Pay ", " by paying ")
        .replace(
            "Each creature you control has base power and toughness ",
            "Creatures you control have base power and toughness ",
        );
    // A granted keyword reads in lowercase mid-sentence: "has hexproof",
    // "has hexproof and can't be blocked" (Tromokratis, Cloak of Mists).
    for keyword in [
        "Hexproof", "Flying", "Trample", "Haste", "Affinity", "Indestructible", "Vigilance",
        "Lifelink", "Deathtouch", "Menace", "Reach", "Defender", "First strike", "Double strike",
        "Can't be blocked", "Cycling", "Unearth", "Encore", "Scavenge", "Flashback", "Escape",
        "Disturb", "Embalm", "Eternalize", "Melee", "Ward",
    ] {
        for verb in [" has ", " have ", " and "] {
            let needle = format!("{verb}{keyword}");
            let mut search_from = 0;
            while let Some(found) = line[search_from..].find(&needle) {
                let start = search_from + found;
                let end = start + needle.len();
                let boundary = line[end..]
                    .chars()
                    .next()
                    .is_none_or(|ch| ch == ' ' || ch == '.' || ch == ',' || ch == '\u{2014}');
                if boundary && (verb != " and " || line[..start].contains(" has ") || line[..start].contains(" have ")) {
                    line.replace_range(start + verb.len()..end, &keyword.to_ascii_lowercase());
                }
                search_from = end;
            }
        }
    }
    // A keyword line and a level-up P/T row carry no period: "Convoke",
    // "Job select", "Cumulative upkeep {R}", "Level up {1}{W}", "4/4".
    if let Some(body) = line.strip_suffix('.')
        && !body.contains(['.', ':', '"'])
    {
        let is_mana_tail = |tail: &str| {
            tail.starts_with('{')
                && tail.ends_with('}')
                && tail.chars().all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '{' | '}' | '/'))
        };
        let bare_keyword = [
            "Job select", "Convoke", "Bargain", "Living weapon", "Storied", "Ascend", "Demonstrate",
            "Storm", "Improvise", "Unleash", "Increment", "Delve", "Exploit", "Retrace", "Jump-start",
            "Conspire",
        ]
        .contains(&body);
        let costed_keyword = [
            "Level up ", "Cumulative upkeep ", "Beam me up ", "Web-slinging ", "Commander ninjutsu ",
        ]
        .iter()
        .any(|keyword| body.strip_prefix(keyword).is_some_and(is_mana_tail))
            || body.starts_with("Craft with ")
                && body.rsplit_once(' ').is_some_and(|(_, tail)| is_mana_tail(tail));
        let affinity = body
            .strip_prefix("Affinity for ")
            .is_some_and(|rest| !rest.is_empty() && rest.split(' ').count() <= 2);
        let offering = body
            .strip_suffix(" offering")
            .is_some_and(|kind| !kind.contains(' ') && kind.chars().next().is_some_and(|ch| ch.is_ascii_uppercase()));
        let level_pt = body.split_once('/').is_some_and(|(power, toughness)| {
            !power.is_empty()
                && !toughness.is_empty()
                && power.chars().all(|ch| ch.is_ascii_digit())
                && toughness.chars().all(|ch| ch.is_ascii_digit())
        });
        if bare_keyword || costed_keyword || affinity || offering || level_pt {
            line = body.to_string();
        }
    }
    // A non-mana equip cost prints after an em dash (CR 702.6a): "Equip—{2},
    // Pay 2 life."
    if let Some(cost) = line.strip_prefix("Equip {")
        && !line.contains(':')
        && let Some(mana_end) = cost.find("}, ")
        && cost[..mana_end].chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '{' || ch == '}' || ch == '/')
    {
        let body = line["Equip ".len()..].trim_end_matches('.').to_string();
        line = format!("Equip\u{2014}{body}.");
    }
    // "Equipped creature has double strike as long as two or more Equipment
    // are attached to it": the subject is the host.
    for host in ["equipped creature", "enchanted creature"] {
        if line.starts_with(&capitalize_first(host))
            && let Some(head) = line.strip_suffix(&format!(" are attached to {host}."))
        {
            line = format!("{head} are attached to it.");
        }
    }
    // "Blue creatures can't block creatures you control": a blocking
    // restriction covers every creature you control.
    if let Some(head) = line.strip_suffix(" can't block a creature you control.") {
        line = format!("{head} can't block creatures you control.");
    }
    // An Aura's attack tax subject (Cowed by Wisdom) pluralized as
    // "Creature this is attached tos" is the one enchanted creature.
    if let Some(rest) = line.strip_prefix("Creature this is attached tos can't ") {
        line = format!("Enchanted creature can't {rest}");
    }
    // "Each Sliver creature card in your graveyard has unearth {2}": a
    // bare singular card subject quantifies every such card.
    for zone in [" card in your graveyard has ", " card in your hand has "] {
        if let Some((subject, _)) = line.split_once(zone)
            && subject.chars().next().is_some_and(|ch| ch.is_ascii_uppercase())
            && !subject.contains([',', '"'])
            && subject.split(' ').count() <= 3
            && !["Each ", "A ", "An ", "This ", "That ", "Target ", "The "]
                .iter()
                .any(|head| subject.starts_with(head))
        {
            let first = subject.split(' ').next().unwrap_or_default();
            let subject_text = if ["Creature", "Artifact", "Enchantment", "Land", "Instant", "Sorcery", "Planeswalker", "Permanent", "Nonland"]
                .contains(&first)
            {
                lowercase_first(subject)
            } else {
                subject.to_string()
            };
            line = format!("Each {subject_text}{}", &line[subject.len()..]);
            break;
        }
    }
    // "This creature enters with a +1/+1 counter on it for each other spell
    // cast this turn" (Storm Entity, Unbreathing Horde): a counted entry
    // reads per counted object.
    if let Some((before, rest)) = line.split_once(" enters with the number of ")
        && let Some((counted, after)) = rest.split_once(" counters on it")
        && let Some((counted, kind)) = counted.rsplit_once(' ')
        && (kind == "+1/+1" || kind.chars().all(|ch| ch.is_ascii_lowercase()))
    {
        let parts = counted
            .split(" plus the number of ")
            .map(|part| {
                let split = [" you ", " your ", " in ", " on ", " among ", " cast ", " of "]
                    .iter()
                    .filter_map(|marker| part.find(marker))
                    .min()
                    .unwrap_or(part.len());
                format!("{}{}", singularize_counted_noun(&part[..split]), &part[split..])
            })
            .collect::<Vec<_>>();
        if parts.iter().all(|part| !part.contains(',')) {
            line = format!(
                "{before} enters with {} on it for each {}{after}",
                with_indefinite_article(&format!("{kind} counter")),
                parts.join(" and each ")
            );
        }
    }
    // The plural of the Ox creature type is "Oxen" (Bruse Tarl).
    if line.starts_with("Oxes ") || line.contains(" Oxes ") {
        line = line.replace("Oxes ", "Oxen ");
    }
    // Ground Seal, Grafdigger's Cage-style untargetable graveyards: Oracle
    // prints the graveyard set in the plural.
    if line == "Card in a graveyard can't be targeted." {
        line = "Cards in graveyards can't be the targets of spells or abilities.".to_string();
    }
    // A graveyard cast whose extra cost exiles cards reads that cost as a
    // gerund beside the card's other costs: "by exiling three cards from
    // your graveyard in addition to paying its other costs" (Kotis,
    // Helbrute). The mana cost is one of "its other costs".
    if let Some((head, tail)) = line.split_once(" by paying its mana cost plus Exile ")
        && let Some(object) = tail.strip_suffix(" from your graveyard, then Exile those cards.")
        && let Some((count, noun)) = object.split_once(' ')
        && let Some(count) = audit_count_word(count)
        && !noun.contains(',')
    {
        let noun = noun.trim_end_matches(" cards").trim_end_matches(" card");
        let noun = if noun == "card" || noun == "cards" { "" } else { noun };
        let object = if count == 1 {
            with_indefinite_article(&format!("{noun} card").trim_start().to_string())
        } else {
            format!(
                "{} {}cards",
                number_word(count as i32).unwrap_or_else(|| count.to_string()),
                if noun.is_empty() { String::new() } else { format!("{noun} ") }
            )
        };
        line = format!(
            "{head} by exiling {object} from your graveyard in addition to paying its other costs."
        );
    }
    // "Enchanted creature is a Turtle with base power and toughness 0/1"
    // (Retro-Mutation, Spider-Man No More): the creature type and the base
    // characteristics are one description.
    if let Some((subject, rest)) = line.split_once(" has base power and toughness ")
        && let Some((pt, rest)) = rest.split_once(" and is a")
        && !pt.contains(' ')
        && let Some(rest) = rest.strip_prefix("n ").or_else(|| rest.strip_prefix(' '))
    {
        let (subtype, tail) = rest
            .find([' ', '.'])
            .map_or((rest, ""), |index| (&rest[..index], &rest[index..]));
        if subtype.chars().next().is_some_and(|ch| ch.is_ascii_uppercase())
            && !subtype.contains(',')
        {
            line = format!(
                "{subject} is {} with base power and toughness {pt}{tail}",
                with_indefinite_article(subtype)
            );
        }
    }
    // "As long as an artifact creature you control is attacking" (Sanwell,
    // Avenger Ace): Oracle names the attacker, not the controller.
    for lead in ["As long as you control an attacking ", "as long as you control an attacking "] {
        if let Some((before, rest)) = line.split_once(lead)
            && let Some((noun, after)) = rest.split_once(", ")
            && noun.split(' ').count() <= 2
        {
            let as_long = if lead.starts_with('A') { "As long as" } else { "as long as" };
            line = format!(
                "{before}{as_long} {} you control is attacking, {after}",
                with_indefinite_article(noun)
            );
            break;
        }
    }
    // Exile is one shared zone (CR 406.1): "there is a card in an opponent's
    // exile" is "an opponent owns a card in exile" (Warden of the Beyond).
    for (zone, owner) in [(" in your exile", "you own"), (" in an opponent's exile", "an opponent owns")] {
        if let Some((before, rest)) = line.split_once("as long as there is ")
            && let Some((object, after)) = rest.split_once(zone)
            && !object.contains(',')
            && (object.starts_with("a ") || object.starts_with("an "))
        {
            line = format!("{before}as long as {owner} {object} in exile{after}");
        }
    }
    // Opus: "If five or more mana was spent to cast that spell, ..." (an
    // intervening "if at least four mana was spent to cast it" keeps its
    // lowercase form).
    for lead in ["If at least ", "unless at least "] {
        while let Some((before, rest)) = line.split_once(lead)
            && let Some((count, after)) = rest.split_once(" mana was spent to cast it")
            && !count.contains(' ')
            && audit_count_word(count).is_some()
        {
            let head = lead.trim_end_matches("at least ");
            line = format!("{before}{head}{count} or more mana was spent to cast that spell{after}");
        }
    }
    // "Each creature card in your graveyard has scavenge. The scavenge cost
    // is equal to its mana cost." (Varolz, Wire Surgeons)
    if let Some((kinds, keyword)) = line
        .strip_suffix(" its mana cost.")
        .and_then(|rest| rest.split_once(" cards in your graveyard have "))
        .filter(|(kinds, keyword)| {
            !keyword.contains(' ') && !kinds.contains([',', '"']) && kinds.split(' ').count() <= 2
        })
    {
        let keyword = keyword.to_ascii_lowercase();
        line = format!(
            "Each {} card in your graveyard has {keyword}. The {keyword} cost is equal to its mana cost.",
            lowercase_first(kinds)
        );
    }
    // Celebration: "as long as two or more nonland permanents entered the
    // battlefield under your control this turn".
    if let Some((head, rest)) = line.split_once(" there are ")
        && let Some((count, rest)) = rest.split_once(" or more ")
        && audit_count_word(count).is_some()
        && let Some((noun, tail)) = rest.split_once(" that entered the battlefield ")
        && !noun.contains(" that ")
    {
        line = format!("{head} {count} or more {noun} entered the battlefield {tail}");
    }
    // "as long as at least eight permanent cards in your graveyard" (a
    // count of one leads: "As long as an artifact entered the battlefield
    // under your control this turn, this creature can ...").
    if let Some(body) = line.strip_suffix('.')
        && let Some((head, rest)) = body.split_once(" as long as at least ")
        && let Some((count, noun_phrase)) = rest.split_once(' ')
        && let Some(n) = audit_count_word(count)
        && !head.contains(" as long as ")
        && !head.contains(['"', ','])
        && (noun_phrase.contains(" entered the battlefield ") || noun_phrase.ends_with(" in your graveyard"))
        && !noun_phrase.starts_with("another ")
    {
        let entered = noun_phrase.split_once(" that entered the battlefield ");
        if n == 1 {
            let article = |noun: &str| {
                if noun.starts_with(['a', 'e', 'i', 'o', 'u', 'A', 'E', 'I', 'O', 'U']) { "an" } else { "a" }
            };
            let condition = match entered {
                Some((noun, tail)) => format!("{} {noun} entered the battlefield {tail}", article(noun)),
                None => format!("there is {} {noun_phrase}", article(noun_phrase)),
            };
            line = format!("As long as {condition}, {}.", lowercase_first(head));
        } else if entered.is_none() {
            line = format!("{head} as long as there are {count} or more {noun_phrase}.");
        }
    }
    // A "where X is ..." definition when the sentence names no X: "up to
    // that many target permanents, where X is ..." counts X targets
    // (Elvish Mariner); otherwise the definition restates the comparison
    // already printed ("with power less than this creature's power",
    // Unliving Psychopath, Triumph of Gerrard) and is dropped.
    if let Some((before, rest)) = line.split_once(", where X is ") {
        let names_x = before
            .char_indices()
            .any(|(at, ch)| {
                ch == 'X'
                    && !before[..at].chars().next_back().is_some_and(|prev| prev.is_ascii_alphabetic())
                    && !before[at + 1..].chars().next().is_some_and(|next| next.is_ascii_alphabetic())
            });
        let clause_end = rest.find(['.', '"']).unwrap_or(rest.len());
        let clause = &rest[..clause_end];
        if !names_x && !clause.contains(" and ") && !clause.contains(" with ") && clause != "that much" {
            if before.contains(" up to that many target ") {
                line = format!("{}, where X is {rest}", before.replacen(" up to that many target ", " up to X target ", 1));
            } else {
                line = format!("{before}{}", &rest[clause_end..]);
            }
        }
    }
    // "If this land would enter, sacrifice a Plains instead." (Kjeldoran
    // Outpost, Soldevi Excavations: "an untapped Island")
    if let Some((head, rest)) = line.split_once(" would enter, sacrifice one ") {
        let article = if rest.starts_with(['a', 'e', 'i', 'o', 'u', 'A', 'E', 'I', 'O', 'U']) { "an" } else { "a" };
        line = format!("{head} would enter, sacrifice {article} {rest}");
    }
    // "If damage would be dealt to this creature while it has a +1/+1
    // counter on it, prevent that damage and remove a +1/+1 counter from
    // it." (Oathsworn Knight, Polukranos)
    if let Some(rest) = line.strip_prefix(
        "As long as it has a +1/+1 counter on it, if damage would be dealt to this creature, ",
    ) {
        line = format!(
            "If damage would be dealt to this creature while it has a +1/+1 counter on it, {}",
            rest.replace("remove one +1/+1 counter from it", "remove a +1/+1 counter from it")
        );
    }
    // "Kicker—{2}{R}, Sacrifice a land." A kicker cost with a non-mana part
    // prints after an em dash.
    if let Some(rest) = line.strip_prefix("Kicker ")
        && let Some((mana, other)) = rest.trim_end_matches('.').split_once(", ")
        && mana.starts_with('{')
        && mana.ends_with('}')
        && !mana.contains(' ')
        && other.chars().next().is_some_and(|ch| ch.is_ascii_uppercase())
    {
        line = format!("Kicker\u{2014}{mana}, {other}.");
    }
    // Verb agreement with a plural subject: "Other Spiders you control have
    // deathtouch and ward {2}", "Other creatures you control get +1/+1 and
    // gain trample and haste until end of turn".
    if let Some((head, tail)) = line.split_once(" have ")
        && !head.contains(['"', '.'])
        && let Some((first, second)) = tail.split_once(" and has ")
        && !first.contains(['"', '.'])
        && merge_passes::is_keyword_phrase(first.trim_end_matches(" during your turn"))
    {
        line = format!("{head} have {first} and {second}");
    }
    if let Some((head, tail)) = line.split_once(" get ")
        && !head.contains(['"', '.'])
        && !head.starts_with("Each ")
        && let Some((first, second)) = tail.split_once(" and gains ")
        && !first.contains(['"', '.', ' '])
    {
        line = format!("{head} get {first} and gain {second}");
    }
    // "has deathtouch, lifelink, and hexproof": a list of three or more
    // keywords is a serial list.
    if let Some(body) = line.strip_suffix('.')
        && let Some((head, keywords)) = body
            .rsplit_once(" have ")
            .map(|(head, keywords)| (format!("{head} have "), keywords))
            .or_else(|| body.rsplit_once(" has ").map(|(head, keywords)| (format!("{head} has "), keywords)))
        && !head.contains('"')
    {
        let parts = keywords.split(" and ").collect::<Vec<_>>();
        if parts.len() >= 3
            && parts.iter().all(|part| !part.contains(',') && merge_passes::is_keyword_phrase(part))
        {
            let (last, init) = parts.split_last().expect("three or more parts");
            line = format!("{head}{}, and {last}.", init.join(", "));
        }
    }
    // "Ward—Pay 3 life." A ward cost that isn't a mana cost prints as a
    // sentence with its period.
    if line.starts_with("Ward\u{2014}") && !line.ends_with('.') && !line.contains(" | ") {
        line.push('.');
    }
    // "Creature spells can't be cast." / "Noncreature spells with mana
    // value 4 or greater can't be cast."
    if let Some(spells) = line
        .strip_prefix("Players can't cast ")
        .and_then(|rest| rest.strip_suffix('.'))
        .filter(|spells| {
            let tail = spells
                .strip_prefix("creature spells")
                .or_else(|| spells.strip_prefix("noncreature spells"));
            tail.is_some_and(|tail| tail.is_empty() || tail.starts_with(" with "))
                && !spells.contains(" and ")
                && !spells.contains(" or ")
        })
    {
        line = format!("{} can't be cast.", capitalize_first(spells));
    }
    if line == "Players can't cast spells with that name." {
        line = "Spells with the chosen name can't be cast.".to_string();
    }
    // "unless there is a Mountain on the battlefield"
    if let Some((head, rest)) = line.split_once(" there are 1 or more ")
        && let Some((noun, tail)) = rest.split_once(" on the battlefield")
        && !noun.contains(' ')
        && noun.chars().next().is_some_and(|ch| ch.is_ascii_uppercase())
    {
        let article = if noun.starts_with(['A', 'E', 'I', 'O', 'U']) { "an" } else { "a" };
        line = format!("{head} there is {article} {noun} on the battlefield{tail}");
    }
    // "As long as there is exactly one tide counter on this enchantment,
    // all blue creatures get -2/-0." An exact counter count reads "exactly".
    if let Some(body) = line.strip_suffix('.')
        && let Some((head, condition)) = body.rsplit_once(" as long as there are ")
        && let Some((count, rest)) = condition.split_once(' ')
        && let Ok(count) = count.parse::<u32>()
        && let Some((kind, object)) = rest.split_once(" counter on ")
        && !kind.contains(' ')
        && object.starts_with("this ")
        && object.split(' ').count() == 2
        && !head.contains(" as long as ")
    {
        let word = ironsmith_core::cardinal_word(count).unwrap_or_else(|| count.to_string());
        let quantity = if count == 1 {
            format!("there is exactly {word} {kind} counter")
        } else {
            format!("there are exactly {word} {kind} counters")
        };
        let subject = match head.strip_prefix(&capitalize_first(object)) {
            Some(predicate) => format!("it{predicate}"),
            None => lowercase_first(head),
        };
        line = format!("As long as {quantity} on {object}, {subject}.");
    }
    line = audit_static_rule_rephrasings_late(line);
    line
}

/// More pure re-phrasings of static rule readings (see
/// `audit_static_rule_rephrasings`).
fn audit_static_rule_rephrasings_late(mut line: String) -> String {
    // Coalition Victory: all five basic land types and all five colors.
    line = line
        .replace(
            "a Plains land and an Island land and you control a Swamp land and a Mountain land and you control a Forest land",
            "a land of each basic land type",
        )
        .replace(
            "a white creature and you control a blue creature and a black creature and you control a red creature and a green creature",
            "a creature of each color",
        )
        // Lupine Prototype: fewer players with cards in hand than players.
        .replace(
            " the number of players with one or more cards in hand is less than the number of players",
            " a player has no cards in hand",
        )
        // Pitfall Trap: an exact count of one attacker.
        .replace("If the number of attacking creatures is equal to 1, ", "If exactly one creature is attacking, ")
        // Lieutenant: "As long as you control your commander".
        .replace(" control a your commander", " control your commander")
        // An optional "any number" additional cost (CR 601.2b): "you may
        // sacrifice any number of creatures" (Torgaar, Rottenmouth Viper).
        .replace(
            "As an additional cost to cast this spell, sacrifice any number of ",
            "As an additional cost to cast this spell, you may sacrifice any number of ",
        );
    // "Enchanted artifact is a Golem creature with base power and toughness
    // 5/4 in addition to its other types." (Ensoul Artifact, Zoetic Glyph,
    // Puppet Crafting)
    if let Some(body) = line.strip_suffix(" in addition to its other types.")
        && let Some((subject, rest)) = body.split_once(" has base power and toughness ")
        && let Some((pt, tail)) = rest.split_once(" and is creature")
        && !pt.contains(' ')
        && !subject.contains(['"', ',', '.'])
        && !subject.starts_with("As long as ")
    {
        let kind = if tail.is_empty() {
            Some(String::new())
        } else {
            tail.strip_prefix(" in addition to its other types and is a ")
                .or_else(|| tail.strip_prefix(" in addition to its other types and is an "))
                .filter(|kind| !kind.contains(' ') && kind.chars().next().is_some_and(|ch| ch.is_ascii_uppercase()))
                .map(|kind| format!("{kind} "))
        };
        if let Some(kind) = kind {
            let article = if kind.starts_with(['A', 'E', 'I', 'O', 'U']) { "an" } else { "a" };
            line = format!("{subject} is {article} {kind}creature with base power and toughness {pt} in addition to its other types.");
        }
    }
    // A stack object you cast is a spell: "Spells you cast with mana value 5
    // or greater can't be countered." (Thryx, Cunning Nightbonder)
    if let Some(rest) = line.strip_prefix("Permanent cast by you with ")
        && rest.ends_with(" can't be countered.")
    {
        line = format!("Spells you cast with {rest}");
    }
    // "The first creature spell you cast each turn can be cast as though it
    // had flash." (Radagast of Rhosgobel)
    if let Some(kind) = line
        .strip_prefix("You may cast ")
        .and_then(|rest| rest.strip_suffix(" cast by you first spells cast each turn as though they had flash."))
        .filter(|kind| !kind.contains(' '))
    {
        line = format!("The first {kind} spell you cast each turn can be cast as though it had flash.");
    }
    // A granted ability merged with a type change is quoted once: "Enchanted
    // permanent has "{T}: Add {C}" and is a land and is colorless." (In Too
    // Deep, Sugar Coat); a lone card type and color read "is a colorless
    // land with "{T}: Add {C}"" (Imprisoned in the Moon).
    if let Some((subject, rest)) = line.split_once(" has \"\"")
        && let Some((ability, rest)) = rest.split_once("\" and is ")
        && let Some((types, tail)) = rest.split_once('"')
        && !subject.contains('"')
        && !types.contains('"')
        && !tail.contains('"')
    {
        let rebuilt = match (types.strip_prefix("a ").or_else(|| types.strip_prefix("an ")), tail) {
            (Some(kind), " and is colorless.") if !kind.contains(' ') => {
                format!("{subject} is a colorless {kind} with \"{ability}\".")
            }
            _ => format!("{subject} has \"{ability}\" and is {types}{tail}"),
        };
        line = rebuilt;
    }
    // "Equipped creature gets +1/+1 for each color among permanents you
    // control." (Conqueror's Flail, Faeburrow Elder; "for each counter on
    // it": Luxior, Gavel of the Righteous, Kyler)
    for verb in [" gets ", " get "] {
        let marker = format!("{verb}+X/+X, where X is the number of ");
        if let Some((head, rest)) = line.split_once(marker.as_str())
            && let Some(rest) = rest.strip_suffix('.')
            && !rest.contains([',', '"', '.'])
            && !head.contains(['"', ','])
        {
            let each = rest
                .strip_prefix("colors among ")
                .map(|tail| format!("color among {tail}"))
                .or_else(|| rest.strip_prefix("counters on ").map(|tail| format!("counter on {tail}")));
            if let Some(each) = each {
                line = format!("{head}{verb}+1/+1 for each {each}.");
                break;
            }
        }
    }
    // A graveyard-size condition prints "there are": "as long as there are
    // eight or more cards in your graveyard" (Doc Ock).
    for (from, to) in [
        (" as long as you have ", " as long as there are "),
        ("As long as you have ", "As long as there are "),
    ] {
        if let Some((head, rest)) = line.split_once(from)
            && let Some((count, tail)) = rest.split_once(" or more cards in your graveyard")
            && audit_count_word(count).is_some()
        {
            line = format!("{head}{to}{count} or more cards in your graveyard{tail}");
        }
    }
    // "Creatures you control can't have -1/-1 counters put on them."
    // (Melira, Sylvok Outcast; Blightbeetle: "Creatures your opponents
    // control can't have +1/+1 counters put on them.")
    if let Some((kind, holder)) = line
        .strip_suffix('.')
        .and_then(|body| body.split_once(" counters can't be put on a creature "))
        && !kind.contains(' ')
        && matches!(holder, "you control" | "an opponent controls")
    {
        let holder = if holder == "you control" { "you control" } else { "your opponents control" };
        line = format!("Creatures {holder} can't have {kind} counters put on them.");
    }
    // "Put a -1/-1 counter on this creature for each 1 damage prevented
    // this way." (Phyrexian Hydra): a damage amount, not a mana symbol.
    line = line
        .replace(" for each {1} damage ", " for each 1 damage ")
        .replace(" prevent that damage. put ", " prevent that damage. Put ");
    // Two grants to the attached object joined on a split subject: "The
    // object this is attached to has flying and hexproof." (Cloudform,
    // Lightform)
    if line.starts_with("The object this is attached to has ") && !line.contains('"') {
        line = line.replace(" and is attached to has ", " and ");
    }
    // "is a creature in addition to its other types" (Gideon Blackblade,
    // Aerial Modification).
    if !line.contains(" source is creature ") {
        line = line.replace(" is creature in addition to its other types", " is a creature in addition to its other types");
    }
    // "Activated abilities of sources with the chosen name can't be
    // activated" (Phyrexian Revoker, Pithing Needle): the name chosen as
    // the permanent entered.
    if line.starts_with("Activated abilities of ") {
        line = line
            .replacen("Activated abilities of a permanent with that name ", "Activated abilities of permanents with the chosen name ", 1)
            .replacen("Activated abilities of permanents with that name ", "Activated abilities of permanents with the chosen name ", 1)
            .replacen("Activated abilities of sources with that name ", "Activated abilities of sources with the chosen name ", 1);
    }
    // "The Lion-Turtle can't attack or block unless there are three or more
    // Lesson cards in your graveyard." (Ketramose, The Ancient One)
    if let Some(body) = line.strip_suffix('.')
        && let Some((head, rest)) = body.split_once(" can't attack or block as long as the number of ")
        && let Some((noun, count)) = rest.rsplit_once(" is less than ")
        && audit_count_word(count).is_some()
        && (noun.contains(" in your graveyard") || noun.ends_with(" in exile"))
        && !head.contains(['"', ','])
        && !noun.contains(['"', ','])
    {
        line = format!("{head} can't attack or block unless there are {count} or more {noun}.");
    }
    // "Enchanted creature can't be blocked by Spirits." (Field of Reality,
    // Blazing Torch: "by Vampires or Zombies", Wrecking Ball Arm)
    for object in ["enchanted creature", "equipped creature"] {
        let blockers = line
            .strip_suffix(&format!(" can't block {object}."))
            .or_else(|| line.strip_suffix(&format!(" can't block {object} with this Equipment attached to it.")))
            .filter(|blockers| {
                !blockers.contains(['"', ',', '.'])
                    && !blockers.starts_with("Enchanted ")
                    && !blockers.starts_with("Equipped ")
            })
            .map(str::to_string);
        if let Some(blockers) = blockers {
            let plural_kinds = {
                let kinds = blockers.split(" or ").collect::<Vec<_>>();
                (kinds.len() > 1
                    && kinds.iter().all(|kind| {
                        !kind.contains(' ') && kind.chars().next().is_some_and(|ch| ch.is_ascii_uppercase())
                    }))
                .then(|| kinds.iter().map(|kind| pluralize_word(kind)).collect::<Vec<_>>().join(" or "))
            };
            let blockers = match (blockers.strip_prefix("Creature "), plural_kinds) {
                (Some(rest), _) => format!("Creatures {rest}"),
                (None, Some(plural)) => plural,
                (None, None) => blockers.clone(),
            };
            let first = blockers.split(' ').next().unwrap_or("");
            let common = first.starts_with("Non")
                || [
                    "Creatures", "Artifact", "White", "Blue", "Black", "Red", "Green", "Colorless",
                    "Multicolored", "Legendary", "Tapped", "Untapped", "Attacking",
                ]
                .contains(&first);
            let blockers = if common { lowercase_first(&blockers) } else { blockers };
            line = format!("{} can't be blocked by {blockers}.", capitalize_first(object));
            break;
        }
    }
    line
}

/// A land animated into a creature that keeps its land type reads as the
/// printed "N/N ... creature ... still a land" (Living Lands, Vastwood
/// Zendikon): "in addition to its other types" already keeps the land type.
fn audit_land_animation_surface(line: &str) -> Option<String> {
    const BASE_PT: &str = " base power and toughness ";
    // "All Forests are creatures in addition to their other types and have
    // base power and toughness 1/1." -> "All Forests are 1/1 creatures that
    // are still lands."
    if let Some(rest) = line.strip_prefix("All ")
        && let Some((subject, tail)) = rest.split_once(" are creatures in addition to their other types and have")
        && let Some(pt) = tail.strip_prefix(BASE_PT).and_then(|pt| pt.strip_suffix('.'))
        && !pt.contains(' ')
        && (subject == "lands"
            || ["Forests", "Islands", "Mountains", "Plains", "Swamps"].contains(&subject))
    {
        return Some(format!("All {subject} are {pt} creatures that are still lands."));
    }
    // "Forests you control are creatures in addition to their other types and
    // is Elves in addition to their other types and is green and has base
    // power and toughness 1/1." -> "Forests you control are 1/1 green Elf
    // creatures that are still lands." (Ambush Commander)
    if let Some((subject, tail)) =
        line.split_once(" you control are creatures in addition to their other types and is ")
        && ["Forests", "Islands", "Mountains", "Plains", "Swamps", "Lands"].contains(&subject)
        && let Some((plural_type, tail)) = tail.split_once(" in addition to their other types and is ")
        && let Some((color, pt)) = tail.split_once(" and has base power and toughness ")
        && let Some(pt) = pt.strip_suffix('.')
        && !plural_type.contains(' ')
        && !pt.contains(' ')
        && ["white", "blue", "black", "red", "green"].contains(&color)
    {
        let singular = match plural_type.strip_suffix("ves") {
            Some(stem) => format!("{stem}f"),
            None => plural_type.strip_suffix('s').unwrap_or(plural_type).to_string(),
        };
        return Some(format!(
            "{subject} you control are {pt} {color} {singular} creatures that are still lands."
        ));
    }
    // "Enchanted land is creature in addition to its other types and is an
    // Elemental in addition to its other types and is green and has base
    // power and toughness 6/4[ and has vigilance and haste]."
    let rest = line.strip_prefix("Enchanted land is creature in addition to its other types and is ")?;
    let rest = rest.strip_suffix('.')?;
    // A colorless animation: "Enchanted land is a 4/4 Elemental creature
    // with reach and haste. It's still a land." (Nissa's Zendikon)
    if let Some((subtype, tail)) = rest.split_once(" in addition to its other types and has base power and toughness ")
        && let Some(subtype) = subtype.strip_prefix("a ").or_else(|| subtype.strip_prefix("an "))
        && !subtype.contains(' ')
    {
        let (pt, keywords) = match tail.split_once(" and ") {
            Some((pt, keywords)) => (
                pt,
                Some(keywords.strip_prefix("has ").unwrap_or(keywords).replace(" and has ", " and ")),
            ),
            None => (tail, None),
        };
        if pt.contains(' ') || keywords.as_deref().is_some_and(|keywords| keywords.contains(['"', ','])) {
            return None;
        }
        let with = keywords.map_or(String::new(), |keywords| format!(" with {keywords}"));
        return Some(format!("Enchanted land is a {pt} {subtype} creature{with}. It's still a land."));
    }
    let (subtype, rest) = rest.split_once(" in addition to its other types and is ")?;
    let subtype = subtype.strip_prefix("a ").or_else(|| subtype.strip_prefix("an "))?;
    let (color, rest) = rest.split_once(" and has")?;
    let rest = rest.strip_prefix(BASE_PT)?;
    let (pt, keywords) = match rest.split_once(" and has ") {
        Some((pt, keywords)) => (pt, Some(keywords)),
        None => (rest, None),
    };
    if subtype.contains(' ')
        || pt.contains(' ')
        || !["white", "blue", "black", "red", "green"].contains(&color)
    {
        return None;
    }
    let with = keywords.map_or(String::new(), |keywords| format!(" with {keywords}"));
    Some(format!("Enchanted land is a {pt} {color} {subtype} creature{with}. It's still a land."))
}

/// "Equipped creature has lifelink as long as you control a Cleric." and
/// its siblings print as one list: "Equipped creature has lifelink as long
/// as you control a Cleric, deathtouch as long as you control a Rogue, ..."
/// (Multiclass Baldric); per-color grants read "Each creature you control
/// has vigilance if it's white, hexproof if it's blue, ..." (Scion of Draco).
fn audit_merge_conditional_keyword_lines(lines: Vec<String>) -> Vec<String> {
    fn controlled_grant(line: &str) -> Option<(&str, &str, &str)> {
        let body = line.strip_suffix('.')?;
        let (subject, rest) = body.split_once(" has ")?;
        let (keyword, condition) = rest.split_once(" as long as you control ")?;
        (!subject.contains(['"', ','])
            && merge_passes::is_keyword_phrase(keyword)
            && !condition.contains([',', '"'])
            && !condition.contains(" as long as "))
            .then_some((subject, keyword, condition))
    }
    fn color_grant(line: &str) -> Option<(String, &str)> {
        let body = line.strip_suffix('.')?;
        let (color, keyword) = body.split_once(" creatures you control have ")?;
        let color = color.to_ascii_lowercase();
        (["white", "blue", "black", "red", "green"].contains(&color.as_str())
            && merge_passes::is_keyword_phrase(keyword))
            .then_some((color, keyword))
    }
    fn join(parts: Vec<String>) -> String {
        if parts.len() == 2 {
            return format!("{} and {}", parts[0], parts[1]);
        }
        match parts.split_last() {
            Some((last, init)) => format!("{}, and {last}", init.join(", ")),
            None => String::new(),
        }
    }
    let mut out = Vec::with_capacity(lines.len());
    let mut idx = 0;
    while idx < lines.len() {
        if let Some((subject, _, _)) = controlled_grant(&lines[idx]) {
            let mut end = idx + 1;
            while end < lines.len() && controlled_grant(&lines[end]).is_some_and(|(other, _, _)| other == subject) {
                end += 1;
            }
            // Two such grants are two printed lines (the Scarecrows); three or
            // more share one list line (Multiclass Baldric).
            if end - idx >= 3 {
                let parts = lines[idx..end]
                    .iter()
                    .filter_map(|line| controlled_grant(line))
                    .map(|(_, keyword, condition)| {
                        format!("{} as long as you control {condition}", lowercase_first(keyword))
                    })
                    .collect::<Vec<_>>();
                out.push(format!("{subject} has {}.", join(parts)));
                idx = end;
                continue;
            }
        }
        if color_grant(&lines[idx]).is_some() {
            let mut end = idx + 1;
            while end < lines.len() && color_grant(&lines[end]).is_some() {
                end += 1;
            }
            let colors = lines[idx..end]
                .iter()
                .filter_map(|line| color_grant(line))
                .map(|(color, _)| color)
                .collect::<Vec<_>>();
            let distinct = colors.iter().enumerate().all(|(at, color)| !colors[..at].contains(color));
            if end - idx >= 3 && distinct {
                let parts = lines[idx..end]
                    .iter()
                    .filter_map(|line| color_grant(line))
                    .map(|(color, keyword)| format!("{} if it's {color}", lowercase_first(keyword)))
                    .collect::<Vec<_>>();
                out.push(format!("Each creature you control has {}.", join(parts)));
                idx = end;
                continue;
            }
        }
        out.push(lines[idx].clone());
        idx += 1;
    }
    out
}

/// A mode choice among losing one keyword each reads as the choice of
/// keyword: "Target creature loses your choice of flying, first strike, or
/// trample until end of turn" (Walking Sponge), "loses first strike or
/// swampwalk" (Urborg).
fn audit_merge_keyword_loss_modes(lines: Vec<String>) -> Vec<String> {
    // A modal ability renders as one entry whose mode bullets follow on
    // their own text lines; read those as separate lines here.
    let lines = lines
        .into_iter()
        .flat_map(|line| {
            if line.contains('\n') && line.split('\n').next().is_some_and(|first| first.ends_with("Choose one —")) {
                line.split('\n').map(str::to_string).collect::<Vec<_>>()
            } else {
                vec![line]
            }
        })
        .collect::<Vec<_>>();
    let mut out = Vec::with_capacity(lines.len());
    let mut idx = 0;
    while idx < lines.len() {
        let line = &lines[idx];
        let Some(head) = line.strip_suffix("Choose one —") else {
            out.push(line.clone());
            idx += 1;
            continue;
        };
        let (head, mode_subject) = match head.strip_suffix("Choose target creature. ") {
            Some(head) => (head, "• It loses "),
            None => (head, "• Target creature loses "),
        };
        let mut keywords = Vec::new();
        let mut end = idx + 1;
        while let Some(mode) = lines.get(end).filter(|mode| mode.starts_with("• ")) {
            let Some(keyword) = mode
                .strip_prefix(mode_subject)
                .and_then(|rest| rest.strip_suffix(" until end of turn."))
                .filter(|keyword| {
                    !keyword.is_empty()
                        && !keyword.contains(" and ")
                        && !keyword.contains(" or ")
                        && !keyword.contains(',')
                        && keyword.chars().next().is_some_and(|ch| ch.is_ascii_lowercase())
                })
            else {
                keywords.clear();
                break;
            };
            keywords.push(keyword.to_string());
            end += 1;
        }
        if keywords.len() < 2 || (head.is_empty() && mode_subject == "• It loses ") {
            out.push(line.clone());
            idx += 1;
            continue;
        }
        let choice = match keywords.as_slice() {
            [first, second] => format!("{first} or {second}"),
            [init @ .., last] => format!("your choice of {}, or {last}", init.join(", ")),
            [] => unreachable!("at least two keywords"),
        };
        out.push(format!("{head}Target creature loses {choice} until end of turn."));
        idx = end;
    }
    out
}

/// An activated ability's own cost modifier (CR 602.2b, 601.2f) is printed
/// inside that ability's line, before any "Activate only ..." restriction:
/// "{5}, {T}: Draw a card. This ability costs {1} less to activate for each
/// page counter on this artifact." (Diary of Dreams). Reducing the whole
/// mana cost to {0} reads as costing that mana less (Kami of Jealous Thirst).
/// A cast restriction and an activation restriction with one timing read as
/// one printed rule: "Players can cast spells and activate abilities only
/// during their own turns" (City of Solitude); "During your turn, your
/// opponents can't cast spells or activate abilities of artifacts,
/// creatures, or enchantments" (Grand Abolisher).
fn audit_cast_and_activate_restrictions(lines: Vec<String>) -> Vec<String> {
    // "Each opponent who controls more creatures than you can't cast creature
    // spells. The same is true for artifacts and enchantments." (Ward of
    // Bones): (plural noun, spell kind) of a "controls more" cast lock.
    fn more_than_you_lock(line: &str) -> Option<(&str, &str)> {
        let rest = line.strip_prefix("Each opponent who controls more ")?;
        let (plural, rest) = rest.split_once(" than you can't cast ")?;
        let kind = rest.strip_suffix(" spells.")?;
        (!plural.contains(' ') && !kind.contains(' ') && plural.strip_suffix('s') == Some(kind))
            .then_some((plural, kind))
    }
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    for line in lines {
        // A restriction on each opponent matching a quality: "Each opponent
        // who attacked with a creature this turn can't cast spells."
        let each_opponent = line
            .strip_prefix("An opponent who ")
            .filter(|rest| rest.contains(" can't "))
            .map(|rest| format!("Each opponent who {rest}"));
        let line = each_opponent.unwrap_or(line);
        if let Some((plural, _)) = more_than_you_lock(&line)
            && let Some(previous) = out.last_mut()
        {
            let head = previous.split(" The same is true for ").next().unwrap_or("").to_string();
            if let Some((first, _)) = more_than_you_lock(&head)
                && first != plural
            {
                let mut others = previous
                    .split_once(" The same is true for ")
                    .map(|(_, tail)| {
                        tail.trim_end_matches('.')
                            .split(" and ")
                            .flat_map(|part| part.split(", "))
                            .map(str::to_string)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                others.push(plural.to_string());
                let list = match others.as_slice() {
                    [only] => only.clone(),
                    [init @ .., last] => format!("{} and {last}", init.join(", ")),
                    [] => String::new(),
                };
                *previous = format!("{head} The same is true for {list}.");
                continue;
            }
        }
        // "Nonartifact creatures get +2/+2 as long as they all share a color"
        // (Common Cause): no subject object lacks some one color.
        let share_a_color = line.split_once(" as long as there are 0 ").and_then(|(head, condition)| {
            let subject = head.split(" get").next().unwrap_or("").to_ascii_lowercase();
            let (qualifier, noun) = subject.rsplit_once(' ').unwrap_or(("", subject.as_str()));
            let lead = if qualifier.is_empty() { String::new() } else { format!("{qualifier} ") };
            let expected = ["white", "blue", "black", "red", "green"]
                .iter()
                .map(|color| format!("{lead}non{color} {noun} on the battlefield"))
                .collect::<Vec<_>>()
                .join(" or there are 0 ");
            (!noun.is_empty() && condition == format!("{expected}."))
                .then(|| format!("{head} as long as they all share a color."))
        });
        let line = share_a_color.unwrap_or(line);
        // A granted ninjutsu expansion (CR 702.49a) reads as the keyword:
        // "Each creature card in your hand has ninjutsu {2}{U}{B}."
        const NINJUTSU: &str = ", Return an unblocked attacker you control to its owner's hand: Put this card onto the battlefield tapped and attacking. Activate only during combat.\"";
        let ninjutsu = line.strip_suffix(NINJUTSU).and_then(|head| {
            let (holders, cost) = head.split_once(" have \"")?;
            let singular = holders.strip_prefix("Creature cards ")?;
            (cost.starts_with('{') && cost.ends_with('}') && !cost.contains(' '))
                .then(|| format!("Each creature card {singular} has ninjutsu {cost}."))
        });
        let line = ninjutsu.unwrap_or(line);
        // A keyword joined into a quoted grant list: 'has "reach and "{2},
        // {T}: Tap target creature."" and "{8}, {T}: Discover 10."' is
        // 'has reach, "{2}, {T}: Tap target creature," and "{8}, ...".'.
        let keyword_lead = line.split_once(" has \"").and_then(|(head, rest)| {
            let (keyword, rest) = rest.split_once(" and \"")?;
            let (first, last) = rest.split_once(".\"\" and \"")?;
            (merge_passes::is_keyword_phrase(keyword)
                && !keyword.contains('"')
                && !first.contains('"')
                && !last.contains(" and \""))
                .then(|| format!("{head} has {keyword}, \"{first},\" and \"{last}"))
        });
        let line = keyword_lead.unwrap_or(line);
        // "This artifact enters with a +1/+1 counter, a flying counter, and a
        // shield counter on it." (Agent's Toolkit): one counter per kind.
        fn single_entry_counter(line: &str) -> Option<(&str, &str)> {
            let (subject, rest) = line.split_once(" enters with ")?;
            let counter = rest.strip_suffix(" counter on it.")?;
            (counter.starts_with("a ") || counter.starts_with("an "))
                .then_some((subject, counter))
                .filter(|(_, counter)| counter.split(' ').count() == 2)
        }
        if let Some((subject, counter)) = single_entry_counter(&line)
            && let Some(previous) = out.last_mut()
            && let Some((before, rest)) = previous.split_once(" enters with ")
            && before == subject
            && let Some(listed) = rest.strip_suffix(" counter on it.")
            && !listed.contains('.')
        {
            let listed = listed.replace(" counter and ", " counter, ");
            let mut counters = listed
                .split(", ")
                .map(|part| part.strip_prefix("and ").unwrap_or(part).trim_end_matches(" counter"))
                .map(str::to_string)
                .collect::<Vec<_>>();
            if counters.iter().all(|part| part.starts_with("a ") || part.starts_with("an ")) {
                counters.push(counter.to_string());
                let list = match counters.as_slice() {
                    [first, second] => format!("{first} counter and {second}"),
                    [init @ .., last] => format!("{} counter, and {last}", init.join(" counter, ")),
                    [] => String::new(),
                };
                *previous = format!("{subject} enters with {list} counter on it.");
                continue;
            }
        }
        if line == "A player other than the active player can't activate abilities."
            && out.last().is_some_and(|previous| previous == "Players can cast spells only during their own turns.")
        {
            out.pop();
            out.push("Players can cast spells and activate abilities only during their own turns.".to_string());
            continue;
        }
        if let Some(rest) = line.strip_prefix("Your opponents can't cast spells and activated abilities of ")
            && let Some(objects) = rest.strip_suffix(" an opponent controls can't be activated during your turn.")
            && let Some(objects) = objects.strip_prefix("an ").or_else(|| objects.strip_prefix("a "))
            && !objects.contains(" and ")
        {
            let nouns = objects
                .split(", ")
                .map(|noun| noun.strip_prefix("or ").unwrap_or(noun))
                .collect::<Vec<_>>();
            if nouns.len() >= 2 && nouns.iter().all(|noun| !noun.is_empty() && !noun.contains(' ')) {
                let plural = nouns.iter().map(|noun| pluralize_noun_phrase(noun)).collect::<Vec<_>>();
                let (last, init) = plural.split_last().expect("two or more nouns");
                let list = if init.len() == 1 {
                    format!("{} or {last}", init[0])
                } else {
                    format!("{}, or {last}", init.join(", "))
                };
                out.push(format!(
                    "During your turn, your opponents can't cast spells or activate abilities of {list}."
                ));
                continue;
            }
        }
        out.push(line);
    }
    out
}

fn audit_join_activation_cost_modifiers(lines: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    for line in lines {
        let previous = out.last().map(String::as_str).unwrap_or("");
        let equip_keyword = previous.starts_with("Equip ")
            && !previous.contains(':')
            && !previous.contains('.')
            && !previous.contains('"');
        let activated_cost = previous
            .split_once(": ")
            .map(|(cost, _)| cost)
            .filter(|cost| {
                cost.contains('{')
                    && !cost.contains('"')
                    && !cost.starts_with("When")
                    && !cost.starts_with("At ")
                    && !cost.starts_with("• ")
            })
            .filter(|_| !previous.ends_with(".\"") && !previous.ends_with('"'));
        let modifier = if let Some(rest) = line.strip_prefix("This ability costs ") {
            let rest = rest
                .replacen(" to activate as long as ", " to activate if ", 1)
                .replace("for each another ", "for each other ")
                .replace(
                    " the number of differently named a land you control",
                    " the number of differently named lands you control",
                );
            // "costs {1} more to activate for each card in your hand"
            // (Loreseeker's Stone).
            let rest = match rest
                .strip_prefix("mana equal to the number of cards ")
                .and_then(|tail| tail.split_once(" more to activate"))
            {
                Some((place, tail)) if !place.contains(' ') || place.starts_with("in ") => {
                    format!("{{1}} more to activate for each card {place}{tail}")
                }
                _ => rest,
            };
            Some(format!("This ability costs {rest}"))
        } else if let Some(condition) =
            line.strip_prefix("You may pay {0} rather than pay this ability's mana cost as long as ")
        {
            activated_cost
                .and_then(|cost| cost.split(", ").next())
                .filter(|mana| {
                    mana.starts_with('{')
                        && mana.ends_with('}')
                        && !mana.contains(' ')
                        && !mana.contains("{T}")
                        && !mana.contains("{Q}")
                        && !mana.contains("{X}")
                })
                .map(|mana| format!("This ability costs {mana} less to activate if {condition}"))
        } else {
            None
        };
        let Some(modifier) = modifier.filter(|_| equip_keyword || activated_cost.is_some()) else {
            out.push(line);
            continue;
        };
        let modifier = format!("{}.", modifier.trim_end_matches('.'));
        let previous = out.pop().unwrap_or_default();
        let joined = if equip_keyword {
            format!("{}. {modifier}", previous.trim_end_matches('.'))
        } else if let Some(at) = previous.find(" Activate only ") {
            format!("{} {modifier}{}", &previous[..at], &previous[at..])
        } else {
            format!("{}. {modifier}", previous.trim_end_matches('.'))
        };
        out.push(joined);
    }
    out
}

/// Pure re-phrasings of triggered/activated ability effect readings: the
/// same typed structure in Oracle's words. Each rewrite was checked against
/// the corpus so it only fires where Oracle uses the target form.
fn audit_ability_effect_rephrasings(line: &str) -> String {
    let mut line = line.to_string();
    // A basic land or subtype union: "a basic land card or Gate card"
    // (District Guide), never the doubled "a basic or Gate card card".
    if let Some(at) = line.find(" basic or ") {
        let start = at + " basic or ".len();
        if let Some(end) = line[start..].find(" card card") {
            let subtype = line[start..start + end].to_string();
            if !subtype.is_empty()
                && !subtype.contains(' ')
                && subtype.chars().next().is_some_and(|ch| ch.is_ascii_uppercase())
            {
                let article = if subtype == "Desert" { "a " } else { "" };
                line.replace_range(
                    at..start + end + " card card".len(),
                    &format!(" basic land card or {article}{subtype} card"),
                );
            }
        }
    }
    // A trigger subject keeps its article: "Whenever a creature deals
    // damage to you" (Aurification), "Whenever an Aura becomes attached".
    for (noun, article) in [
        ("creature", "a"),
        ("spell", "a"),
        ("permanent", "a"),
        ("land", "a"),
        ("artifact", "an"),
        ("Aura", "an"),
    ] {
        if let Some(rest) = line.strip_prefix(&format!("Whenever {noun} ")) {
            line = format!("Whenever {article} {noun} {rest}");
            break;
        }
    }
    // A counted amount of counters: "they get that many poison counters".
    let mut search = 0;
    while let Some(offset) = line[search..].find("that much ") {
        let at = search + offset;
        let rest = &line[at + "that much ".len()..];
        if rest
            .split_once(' ')
            .is_some_and(|(kind, tail)| !kind.is_empty() && tail.starts_with("counters"))
        {
            line.replace_range(at..at + "that much ".len(), "that many ");
        }
        search = at + "that many ".len();
    }
    // "reveal that many cards from the top of your library" (Gishath).
    for verb in ["look at", "reveal"] {
        line = line.replace(
            &format!("{verb} the top that much cards of your library"),
            &format!("{verb} that many cards from the top of your library"),
        );
    }
    // The source's attached objects are on the battlefield by definition:
    // "for each Aura and Equipment attached to it" (Wyleth).
    line = line
        .replace(
            " attached to this creature and Equipment attached to this creature on the battlefield",
            " and Equipment attached to this creature",
        )
        .replace(" attached to this creatures on the battlefield", " attached to this creature")
        .replace(" attached to this creature on the battlefield", " attached to this creature");
    // A targeted "lesser mana value" card compares with the triggering
    // object implicitly: "return target Cleric card with lesser mana value
    // from your graveyard" (Orah, Skyclave Hierophant).
    const LESSER_THAN_IT: &str = " with lesser mana value than it from your graveyard";
    if let Some(at) = line.find(LESSER_THAN_IT)
        && line[..at].contains("target ")
    {
        line.replace_range(at..at + LESSER_THAN_IT.len(), " with lesser mana value from your graveyard");
    }
    // "Then if there are three or more ribbon counters on this creature,
    // remove those counters and untap it" (Prize Pig).
    if line.contains("Then if ")
        && let Some(at) = line.find("remove all ")
        && let Some((kind, _)) = line[at + "remove all ".len()..].split_once(" counters from it and ")
        && !kind.contains(' ')
        && line[..at].contains(&format!(" or more {kind} counters on"))
    {
        let end = at + "remove all ".len() + kind.len() + " counters from it".len();
        line.replace_range(at..end, "remove those counters");
    }
    // A countered spell or ability is paid for by its own controller
    // (Amulet of Safekeeping, Strict Proctor, Lava Runner).
    if line.contains("becomes the target of a spell or ability") {
        line = line
            .replace(
                "counter it unless that source's controller pays",
                "counter that spell or ability unless its controller pays",
            )
            .replace(", that source's controller ", ", that spell or ability's controller ");
    }
    if line.contains("causes a triggered ability to trigger") {
        line = line.replace(
            "counter it unless that source's controller pays",
            "counter that ability unless its controller pays",
        );
    }
    // "This creature deals 2 damage to any target and 3 damage to itself"
    // (Psionic Entity).
    if let Some(at) = line.find("This creature deals ") {
        let end = line[at..].find('.').map_or(line.len(), |offset| at + offset);
        if let Some(and_at) = line[at..end].find(" and ").map(|offset| at + offset)
            && let Some((amount, tail)) = line[and_at + " and ".len()..end].split_once(' ')
            && !amount.is_empty()
            && tail.starts_with("damage to this creature")
        {
            let from = and_at + " and ".len() + amount.len() + 1;
            line.replace_range(
                from..from + "damage to this creature".len(),
                "damage to itself",
            );
        }
    }
    // An exile cost's count reads "the number of cards exiled this way"
    // (Painbringer, Corpseweft).
    if let Some((cost, _)) = line.split_once(": ")
        && cost.contains("Exile ")
        && cost.contains(" from your graveyard")
    {
        line = line.replace(" the number of those cards", " the number of cards exiled this way");
    }
    // A result gate on the optional action just offered to you reads "If
    // you do" (Fire Lord Ozai, Neheb); a gate phrased by its own player
    // reads plainly ("If you search your library this way").
    if let Some(at) = line.find("If effect #") {
        let digits_start = at + "If effect #".len();
        let digits = line[digits_start..]
            .chars()
            .take_while(char::is_ascii_digit)
            .count();
        let after = digits_start + digits;
        if digits > 0 && line[after..].starts_with(' ') {
            let rest = &line[after + 1..];
            let previous = line[..at].trim_end().trim_end_matches('.');
            let sentence = previous.rfind(". ").map_or(previous, |offset| &previous[offset + 2..]);
            if rest.starts_with("happened, ") && sentence.contains("you may ") {
                line.replace_range(at..after + 1 + "happened, ".len(), "If you do, ");
            } else if rest.starts_with("you ") {
                line.replace_range(at..after + 1, "If ");
            }
        }
    }
    // One damage event to either recipient: "Whenever this creature deals
    // combat damage to a player or planeswalker" (Garruk's Harbinger).
    if let Some(at) = line.find(" deals combat damage to a player or ") {
        let start = at + " deals combat damage to a player or ".len();
        if let Some(offset) = line[start..].find(" deals combat damage to a planeswalker")
            && !line[start..start + offset].contains(',')
            && line[start..start + offset].split(' ').count() <= 4
        {
            let end = start + offset + " deals combat damage to a planeswalker".len();
            line.replace_range(at..end, " deals combat damage to a player or planeswalker");
        }
    }
    // A card qualified only by an ability or mana-cost property keeps its
    // noun first: "reveal a card with kicker" (Coralhelm Chronicler), "a
    // card with {X}" (Rosheen), "a card that's exactly two colors".
    if let Some(at) = line.find(" a with ") {
        let start = at + " a with ".len();
        if let Some(offset) = line[start..].find(" card ") {
            let quality = line[start..start + offset].to_string();
            if !quality.is_empty() && quality.split(' ').count() <= 2 && !quality.contains(',') {
                line.replace_range(at..start + offset + " card".len(), &format!(" a card with {quality}"));
            }
        }
    }
    line = line
        .replace(" a that are exactly two colors card ", " a card that's exactly two colors ")
        .replace(" spell that are exactly two colors", " spell that's exactly two colors");
    // An activated ability's controller chooses in the imperative: "{X}:
    // Choose a creature card exiled with Valki".
    line = line.replacen(": You choose ", ": Choose ", 1);
    // A token count read from a quantity: "create a 1/1 red Officer
    // creature token for each dilemma you've faced this turn" (Captain
    // Benjamin Sisko), "create a Food token for each player being attacked".
    if let Some(at) = line.find("create the number of ") {
        let start = at + "create the number of ".len();
        let end = line[start..].find('.').map_or(line.len(), |offset| start + offset);
        let clause = line[start..end].to_string();
        let token_at = [" 1/1 ", " 2/2 ", " Food ", " Treasure ", " Clue "]
            .iter()
            .filter_map(|marker| clause.find(marker))
            .min();
        if let Some(token_at) = token_at
            && let Some(token) = clause[token_at + 1..]
                .strip_suffix(" tokens")
                .or_else(|| clause[token_at + 1..].strip_suffix(" token"))
            && let Some((first, quantity_rest)) = clause[..token_at].split_once(' ').or(Some((&clause[..token_at], "")))
            && let Some(singular) = first.strip_suffix('s')
        {
            let quantity = if quantity_rest.is_empty() {
                singular.to_string()
            } else {
                format!("{singular} {quantity_rest}")
            };
            let article = if token.starts_with(['A', 'E', 'I', 'O', 'U', '8']) { "an" } else { "a" };
            line.replace_range(at..end, &format!("create {article} {token} token for each {quantity}"));
        }
    }
    // A mass pump until end of turn reads in the plural: "Creatures you
    // control get +1/+1 until end of turn", "other attacking creatures get".
    let mut search = 0;
    while let Some(offset) = line[search..].find("ach ") {
        let at = search + offset;
        search = at + "ach ".len();
        let capital = line[..at].ends_with('E');
        if !capital && !line[..at].ends_with('e') {
            continue;
        }
        let start = at - 1;
        let preceded = start == 0 || [". ", ": ", ", ", "• ", "— "].iter().any(|lead| line[..start].ends_with(lead));
        if !preceded {
            continue;
        }
        let body_start = at + "ach ".len();
        let end = line[body_start..].find('.').map_or(line.len(), |offset| body_start + offset);
        let sentence = &line[body_start..end];
        let Some((subject, scope, rest)) = sentence
            .split_once(" you control gets ")
            .map(|(subject, rest)| (subject, " you control", rest))
            .or_else(|| {
                sentence.split_once(" gets ").and_then(|(subject, rest)| {
                    matches!(subject, "other attacking creature" | "other blocking creature")
                        .then_some((subject, "", rest))
                })
            })
        else {
            continue;
        };
        if subject.is_empty()
            || subject.split(' ').count() > 3
            || subject.contains(',')
            || !rest.contains("until end of turn")
            || rest.contains('"')
            || rest
                .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '\'')
                .any(|word| matches!(word, "it" | "it's" | "its"))
        {
            continue;
        }
        let (other, noun) = match subject.strip_prefix("other ") {
            Some(noun) => ("other ", noun),
            None => ("", subject),
        };
        let rest = rest.replacen(" and gains ", " and gain ", 1).replacen(" and has ", " and have ", 1);
        let phrase = format!("{other}{}{scope} get {rest}", pluralize_noun_phrase(noun));
        let phrase = if capital { capitalize_first(&phrase) } else { phrase };
        line.replace_range(start..end, &phrase);
        search = start + phrase.len();
    }
    line
}

/// Pure re-phrasings of activated-ability and grant readings: the same
/// structure in Oracle's words.
fn audit_activated_line_rephrasings(line: &str) -> String {
    let mut line = line.to_string();
    // A quoted granted ability closes the sentence inside its quotes.
    if line.ends_with(".\".") {
        line.pop();
    }
    // Only the last quoted ability of a list closes with a period:
    // `has "A" and "B."`; a quote followed by more of the sentence drops
    // its own period (`has "..." and is a Cleric`, `has "..." as long as`).
    line = line.replace(".\" and \"", "\" and \"");
    line = line.replace(".\" and ", "\" and ").replace(".\" as long as ", "\" as long as ");
    line = line.replace(".\" As long as ", "\" as long as ").replace(".\" During your turn", "\" during your turn");
    // A mana restriction's ability source is the object kind itself:
    // "activate an ability of an Elemental", "activate abilities of
    // artifacts".
    for (marker, single) in [
        ("activate an ability of ", true),
        ("activate abilities of ", false),
    ] {
        if let Some(at) = line.find(marker) {
            let start = at + marker.len();
            let end = line[start..]
                .find('.')
                .map_or(line.len(), |offset| start + offset);
            let object = &line[start..end];
            let replacement = if single {
                object
                    .strip_suffix(" source")
                    .filter(|kind| *kind != "a" && !kind.is_empty())
                    .map(str::to_string)
            } else {
                object
                    .strip_suffix(" sources")
                    .filter(|kind| !kind.is_empty() && !kind.contains(' '))
                    .map(|kind| format!("{kind}s"))
            };
            if let Some(replacement) = replacement {
                line.replace_range(start..end, &replacement);
            }
        }
    }
    // "all" counts every matching object: "Return all cards with aegis
    // counters on them", never the singular noun "all card with".
    for noun in ["card", "creature", "commander", "permanent"] {
        for next in ["in", "with", "put", "that", "from", "you", "your", "on", "exiled"] {
            line = line
                .replace(&format!(" all {noun} {next} "), &format!(" all {noun}s {next} "))
                .replace(&format!("All {noun} {next} "), &format!("All {noun}s {next} "));
        }
    }
    // CR 306.5b: a planeswalker's loyalty is its number of loyalty counters.
    line = line.replace(
        "the number of loyalty counters on this planeswalker",
        "this planeswalker's loyalty",
    );
    // An entry replacement lasting until end of turn applies "this turn"
    // (Zameck Guildmage).
    const UNTIL_EOT: &str = "Until end of turn, ";
    if let Some(at) = line.find(UNTIL_EOT) {
        let rest = &line[at + UNTIL_EOT.len()..];
        let clause = rest.split_once('.').map_or(rest, |(clause, _)| clause);
        if clause.contains(" enters with an additional ") {
            line.replace_range(at..at + UNTIL_EOT.len(), "This turn, ");
        }
    }
    line
}

/// "Non-Wall creature without flying can't block this creature" reads as
/// the exception it grants: "This creature can't be blocked except by Walls
/// and/or creatures with flying".
fn audit_blocker_exception(line: &str) -> Option<String> {
    let (blockers, target) = line.split_once(" can't block ")?;
    let target = target.strip_suffix('.')?;
    // "Nonlegendary creatures can't block a creature you control" is
    // "Creatures you control can't be blocked except by legendary creatures"
    // (Midnight Pathlighter, The Balrog).
    if let Some(quality) = blockers
        .strip_prefix("Non")
        .and_then(|rest| rest.strip_suffix(" creatures").or_else(|| rest.strip_suffix(" creature")))
        .filter(|quality| {
            !quality.is_empty() && quality.chars().all(|ch| ch.is_ascii_lowercase())
        })
    {
        let subject = if target == "a creature you control" || target == "creatures you control" {
            "Creatures you control".to_string()
        } else if ["this creature", "enchanted creature", "equipped creature"].contains(&target) {
            capitalize_first(target)
        } else {
            return None;
        };
        return Some(format!("{subject} can't be blocked except by {quality} creatures."));
    }
    let (qualities, without) = match blockers.split_once(" creature without ") {
        Some((head, quality)) if !quality.contains(' ') => (head, Some(quality)),
        _ => (blockers.strip_suffix(" creature")?, None),
    };
    let mut allowed = Vec::new();
    for quality in qualities.split(' ') {
        let subtype = quality.strip_prefix("Non-").or_else(|| quality.strip_prefix("non-"))?;
        allowed.push(pluralize_noun_phrase(subtype));
    }
    if let Some(quality) = without {
        allowed.push(format!("creatures with {quality}"));
    }
    if allowed.is_empty() {
        return None;
    }
    let subject = if ["this ", "enchanted ", "equipped "].iter().any(|prefix| target.starts_with(prefix)) {
        capitalize_first(target)
    } else if !target.contains(' ') && target.chars().next()?.is_ascii_uppercase() {
        pluralize_noun_phrase(target)
    } else if let Some(kinds) = target
        .split(" or ")
        .map(|kind| {
            kind.strip_prefix("a ")
                .or_else(|| kind.strip_prefix("an "))
                .and_then(|kind| kind.strip_suffix(" you control"))
                .filter(|kind| !kind.contains(' ') && kind.chars().next().is_some_and(|ch| ch.is_ascii_uppercase()))
                .map(pluralize_noun_phrase)
        })
        .collect::<Option<Vec<_>>>()
        .filter(|kinds| kinds.len() >= 2)
    {
        // "Krakens, Leviathans, Octopuses, and Serpents you control"
        let list = match kinds.as_slice() {
            [first, second] => format!("{first} and {second}"),
            [init @ .., last] => format!("{}, and {last}", init.join(", ")),
            [] => return None,
        };
        let allowed = match allowed.as_slice() {
            [init @ .., last] if allowed.len() > 2 => format!("{}, and {last}", init.join(", ")),
            _ => allowed.join(" and/or "),
        };
        return Some(format!("{list} you control can't be blocked except by {allowed}."));
    } else {
        return None;
    };
    Some(format!("{subject} can't be blocked except by {}.", allowed.join(" and/or ")))
}

/// "you control another creature you control with power 4 or greater":
/// the controller is already named by "you control".
fn dedupe_you_control(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(at) = rest.find("you control ") {
        let (head, tail) = rest.split_at(at + "you control ".len());
        out.push_str(head);
        let mut window_end = tail.len().min(48);
        while !tail.is_char_boundary(window_end) {
            window_end -= 1;
        }
        let window = &tail[..window_end];
        match window.find(" you control") {
            Some(repeat)
                if ["a ", "an ", "another "].iter().any(|article| tail.starts_with(article))
                    && !window[..repeat].contains([',', '.', '"'])
                    && !window[..repeat].contains(" and ")
                    && !window[..repeat].contains(" for ") =>
            {
                out.push_str(&tail[..repeat]);
                rest = &tail[repeat + " you control".len()..];
            }
            _ => rest = tail,
        }
    }
    out.push_str(rest);
    out
}

/// "protection from blue, from black, and from red".
fn audit_protection_list(line: &str) -> String {
    let Some(start) = line.to_ascii_lowercase().find("protection from ") else {
        return line.to_string();
    };
    let tail = &line[start..];
    let end = tail.find([',', '.']).map_or(tail.len(), |end| end);
    let segment = &tail[..end];
    let parts = segment.split(" and from ").collect::<Vec<_>>();
    if parts.len() < 3 {
        return line.to_string();
    }
    let (last, init) = parts.split_last().unwrap();
    let joined = format!("{}, and from {last}", init.join(", from "));
    format!("{}{joined}{}", &line[..start], &tail[end..])
}

/// A granted bare keyword-identity marker next to the full typed expansion
/// of decayed (CR 702.147a) or rampage N (CR 702.23a) in one effect line
/// ("They gain decayed", "it gains rampage 2"): the expansion is what reads,
/// as the keyword (the marker itself carries no typed content).
fn audit_inline_keyword_marker_grants(line: &str) -> String {
    const MARKER: &str =
        "\u{27e6}unverified LabelDependentStatic: StaticAbility { id: Some\u{2039}KeywordMarker\u{203a} }\u{27e7}";
    const DECAYED: &str = ", \"This creature can't block,\" and \"Whenever this creature attacks, at this turn's next end of combat, sacrifice this creature.\"";
    const RAMPAGE_HEAD: &str = " and \"Whenever this creature becomes blocked, this creature gets +X/+X until end of turn, where X is ";
    const RAMPAGE_TAIL: &str = "the number of blockers beyond the first.\"";
    let mut out = line.to_string();
    while let Some(found) = out.find(MARKER) {
        let after = &out[found + MARKER.len()..];
        let replacement = if after.starts_with(DECAYED) {
            Some(("decayed.".to_string(), MARKER.len() + DECAYED.len()))
        } else if let Some(rest) = after.strip_prefix(RAMPAGE_HEAD)
            && let Some(end) = rest.find(RAMPAGE_TAIL)
            && let Some(amount) = match &rest[..end] {
                "" => Some(1),
                multiple => multiple.strip_suffix(" times ").and_then(|n| n.parse::<u32>().ok()),
            }
        {
            Some((
                format!("rampage {amount}."),
                MARKER.len() + RAMPAGE_HEAD.len() + end + RAMPAGE_TAIL.len(),
            ))
        } else {
            None
        };
        let Some((keyword, consumed)) = replacement else {
            break;
        };
        let tail = out[found + consumed..].to_string();
        // The quoted expansion closed the sentence; the keyword does too.
        let tail = tail.strip_prefix('.').unwrap_or(&tail).to_string();
        out = format!("{}{keyword}{tail}", &out[..found]);
    }
    out
}

/// "create a 2/2 black Zombie creature token with decayed": a token whose
/// granted abilities are exactly decayed's (CR 702.147) plus its label.
fn collapse_decayed_token(line: &str) -> Option<String> {
    const DECAYED: &str = "\"This token can't block\", \"Whenever this token attacks, at this turn's next end of combat, sacrifice this token";
    let found = line.find(DECAYED)?;
    let head = &line[..found];
    let sentence = head
        .strip_suffix(". It has ")
        .or_else(|| head.strip_suffix(". They have "))
        .or_else(|| head.strip_suffix(". Those tokens have "))?;
    let rest = &line[found + DECAYED.len()..];
    let rest = rest.strip_prefix(".\"").or_else(|| rest.strip_prefix('"'))?;
    let rest = rest
        .strip_prefix(", and \"")
        .and_then(|marker| {
            marker.starts_with(ironsmith_core::semantic_audit::UNVERIFIED_MARKER_PREFIX)
                .then(|| marker.find("\u{27e7}.\"").map(|end| &marker[end + "\u{27e7}.\"".len()..]))
                .flatten()
        })
        .unwrap_or(rest);
    let token_at = sentence.rfind("creature token")? + "creature token".len();
    let plural = sentence[token_at..].starts_with('s');
    let token_end = token_at + usize::from(plural);
    Some(format!(
        "{} with decayed{}.{}",
        &sentence[..token_end],
        &sentence[token_end..],
        rest.strip_prefix('.').unwrap_or(rest)
    ))
}

/// "Annihilator 6": a keyword whose only parameter is a number.
fn is_numbered_audit_keyword(keyword: &str) -> bool {
    let Some((name, number)) = keyword.rsplit_once(' ') else {
        return false;
    };
    !number.is_empty()
        && number.chars().all(|ch| ch.is_ascii_digit())
        && !name.is_empty()
        && !name.contains(|ch: char| ch.is_ascii_digit() || matches!(ch, '{' | '"' | '—' | '.' | ','))
        && merge_passes::is_keyword_phrase(keyword)
}

/// "Ward {2}": a mana-cost ward closes a printed keyword list like a
/// numbered keyword ("Flying, lifelink, ward {2}").
fn is_mana_ward_audit_keyword_d5(keyword: &str) -> bool {
    keyword
        .strip_prefix("Ward {")
        .and_then(|rest| rest.strip_suffix('}'))
        .is_some_and(|cost| {
            !cost.is_empty()
                && cost
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '{' | '}' | '/'))
        })
}

/// A numbered keyword that is a triggered ability (CR 702.86 annihilator,
/// firebending) as it closes or sits in a printed keyword list.
fn is_triggered_list_audit_keyword_r5f(keyword: &str) -> bool {
    let Some((name, number)) = keyword.rsplit_once(' ') else {
        return false;
    };
    matches!(name.to_ascii_lowercase().as_str(), "firebending" | "annihilator" | "afflict")
        && !number.is_empty()
        && (number == "X" || number.chars().all(|ch| ch.is_ascii_digit()))
}

fn is_simple_audit_keyword(keyword: &str) -> bool {
    merge_passes::is_keyword_phrase(keyword)
        && !keyword.contains(|ch: char| ch.is_ascii_digit() || matches!(ch, '{' | '"' | '—' | '.'))
}

/// The attached-object subject an Aura, Equipment or Fortification prints.
fn audit_attached_subject(def: &CardDefinition) -> Option<String> {
    let subtypes = &def.card.subtypes;
    if subtypes.contains(&Subtype::Equipment) {
        return Some("equipped creature".to_string());
    }
    if subtypes.contains(&Subtype::Fortification) {
        return Some("fortified land".to_string());
    }
    if !subtypes.contains(&Subtype::Aura) {
        // A creature that attaches itself (bestow, Licids) is an Aura then.
        return def.card.is_creature().then(|| "enchanted creature".to_string());
    }
    let noun = match def.aura_attach_filter.as_ref()? {
        crate::object::AuraAttachmentFilter::Player(_) => "player",
        crate::object::AuraAttachmentFilter::Object(filter) => {
            if filter.zone.is_some_and(|zone| zone != Zone::Battlefield) {
                "card"
            } else {
                match filter.card_types.as_slice() {
                    [CardType::Creature] => "creature",
                    [CardType::Land] => "land",
                    [CardType::Artifact] => "artifact",
                    [CardType::Enchantment] => "enchantment",
                    [CardType::Planeswalker] => "planeswalker",
                    [CardType::Battle] => "battle",
                    // "Enchant artifact creature": both types at once.
                    [] if matches!(
                        filter.all_card_types.as_slice(),
                        [CardType::Artifact, CardType::Creature] | [CardType::Creature, CardType::Artifact]
                    ) =>
                    {
                        "artifact creature"
                    }
                    _ => "permanent",
                }
            }
        }
    };
    Some(format!("enchanted {noun}"))
}

/// "This creature has "<ability>" as long as <condition>." leads with its
/// condition, as every conditional quoted grant prints: "As long as
/// <condition>, this creature has "<ability>"." A condition about the
/// subject itself refers back with "it" (Lavamancer's Skill, Orochi
/// Merge-Keeper).
fn audit_quoted_grant_condition_lead(line: &str) -> Option<String> {
    let body = line.trim().strip_suffix('.')?;
    let (head, condition) = body.rsplit_once("\" as long as ")?;
    if condition.contains('"') || condition.contains(" as long as ") {
        return None;
    }
    let (subject, verb, quoted) = [" has \"", " have \""]
        .iter()
        .find_map(|verb| head.split_once(verb).map(|(subject, quoted)| (subject, verb.trim_end_matches('"'), quoted)))?;
    if subject.is_empty() || subject.contains(['"', ',']) || subject.split(' ').count() > 5 {
        return None;
    }
    let lower_subject = lowercase_first(subject);
    let subject_text = if condition.starts_with(&format!("{lower_subject} ")) {
        "it".to_string()
    } else {
        lower_subject
    };
    Some(format!("As long as {condition}, {subject_text}{verb}\"{quoted}\"."))
}

/// "Each Wolf and Werewolf you control has haste." -> "Wolves and Werewolves
/// you control have haste." A conjunctive two-noun set grants a keyword to
/// the set as a plural subject (Village Reavers, Fountain Watch, Warg
/// Rider). A quoted ability keeps "Each ... has" (Brightcap Badger).
fn audit_each_conjunctive_grant(line: &str) -> Option<String> {
    let rest = line.strip_prefix("Each ")?;
    let (lead, rest) = match rest.strip_prefix("other ") {
        Some(rest) => ("other ", rest),
        None => ("", rest),
    };
    let (subjects, predicate) = rest.split_once(" you control has ")?;
    if predicate.starts_with('"') {
        return None;
    }
    let (first, second) = subjects.split_once(" and ")?;
    let (second_lead, second) = match second
        .strip_prefix("another ")
        .or_else(|| second.strip_prefix("other "))
    {
        Some(noun) => ("other ", noun),
        None => ("", second),
    };
    let is_word = |word: &str| !word.is_empty() && word.chars().all(|ch| ch.is_ascii_alphabetic() || ch == '-');
    if !is_word(first) || !is_word(second) {
        return None;
    }
    Some(capitalize_first(&format!(
        "{lead}{} and {second_lead}{} you control have {predicate}",
        pluralize_noun_phrase(first),
        pluralize_noun_phrase(second)
    )))
}

fn audit_static_subject_surface(line: &str, subject: &str, attached: Option<&str>) -> String {
    let mut line = line.to_string();
    if let Some(attached) = attached {
        for host in ["object", "permanent"] {
            for attacher in ["this", "this Aura", "this Equipment"] {
                let lower = format!("the {host} {attacher} is attached to");
                line = line
                    .replace(&capitalize_first(&lower), &capitalize_first(attached))
                    .replace(&lower, attached);
            }
        }
        // "The creature this is attached to" on a creature Aura or
        // Equipment, "The land this is attached to" on a land Aura; a
        // pluralized "Creature this is attached tos" is the same object.
        for host in ["creature", "land", "artifact"] {
            if !attached.ends_with(&format!(" {host}")) {
                continue;
            }
            let lower = format!("the {host} this is attached to");
            let plural = format!("{host} this is attached tos");
            line = line
                .replace(&capitalize_first(&plural), &capitalize_first(attached))
                .replace(&plural, attached)
                .replace(&capitalize_first(&lower), &capitalize_first(attached))
                .replace(&lower, attached);
        }
        // An attached permanent's goad is by its controller (CR 701.15a):
        // "Enchanted creature is goaded."
        line = line
            .replace(" is goaded by this Aura's controller", " is goaded")
            .replace(" is goaded by this Equipment's controller", " is goaded");
    }
    // A plain creature Aura's subject is "enchanted creature".
    if attached == Some("enchanted creature") {
        line = line
            .replace("Enchanted permanent", "Enchanted creature")
            .replace("enchanted permanent", "enchanted creature");
    }
    if let Some(rewritten) = audit_each_conjunctive_grant(&line) {
        line = rewritten;
    }
    // "can't be blocked by Wall creatures": only creatures block, so the
    // subtype alone names them ("by Walls").
    if let Some((head, rest)) = line.split_once(" can't be blocked by ")
        && let Some(subtype) = rest.strip_suffix(" creatures.")
        && !subtype.contains(' ')
        && subtype.chars().next().is_some_and(|ch| ch.is_ascii_uppercase())
    {
        line = format!("{head} can't be blocked by {}.", pluralize_noun_phrase(subtype));
    }
    // Only a permanent's controller sacrifices it (CR 701.21a): "Treasure
    // tokens you control can't be sacrificed" is "You can't sacrifice
    // Treasure tokens".
    if let Some(objects) = line
        .strip_suffix(" you control can't be sacrificed.")
        .filter(|objects| !objects.contains(' ') || objects.split(' ').count() <= 3)
        .filter(|objects| !objects.starts_with("Each ") && !objects.starts_with("Other "))
    {
        let card_type = ["Creatures", "Artifacts", "Lands", "Enchantments", "Planeswalkers", "Permanents", "Tokens", "Nontoken"]
            .iter()
            .any(|word| objects.starts_with(word));
        let objects = if card_type { lowercase_first(objects) } else { objects.to_string() };
        line = format!("You can't sacrifice {objects}.");
    }
    // CR 702.18a: a player who can't be the target of spells or abilities
    // has shroud.
    if line == "You can't be targeted." {
        line = "You have shroud.".to_string();
    }
    // An Equipment's attached-object condition names the equipped creature
    // ("as long as enchanted permanent is legendary" on Gimli's Axe).
    if attached == Some("equipped creature") {
        line = line.replace(" enchanted permanent ", " equipped creature ");
        // Cement Shoes, Heavy Arbalest: an Equipment's object is a creature
        // (Luxior's planeswalker case keeps "permanent").
        if !line.contains("planeswalker") {
            for verb in ["doesn't untap ", "assigns combat damage "] {
                line = line.replace(&format!("Equipped permanent {verb}"), &format!("Equipped creature {verb}"));
            }
        }
    }
    // Brainwash: "the object this is attached to" pluralized as a subject.
    if let Some(attached) = attached {
        line = line.replace("Creature this is attached tos ", &format!("{} ", capitalize_first(attached)));
    }
    // A bestowed creature is the Aura: "Enchanted creature attacks each
    // combat if able" (Mogis's Warhound, Spirespine).
    line = line.replace("Enchanted creature with this creature attached to it ", "Enchanted creature ");
    // An Aura has one enchanted creature: "Enchanted creature can't attack
    // you" (Fealty to the Realm).
    if attached == Some("enchanted creature") {
        line = line.replace("Enchanted creatures can't attack you.", "Enchanted creature can't attack you.");
    }
    line = line
        .replace("enchanted creature with this Aura attached to it", "enchanted creature")
        .replace("equipped creature with this Equipment attached to it", "equipped creature");
    // "Enchanted creatures with this Aura attached to it attack each combat
    // if able": the one attached object is the subject.
    for (head, noun) in [
        ("Enchanted creatures with this ", "Enchanted creature"),
        ("Equipped creatures with this ", "Equipped creature"),
    ] {
        if let Some(rest) = line.strip_prefix(head)
            && let Some((_, predicate)) = rest.split_once(" attached to it ")
            && let Some(verb) = ["attack ", "block "].iter().find(|verb| predicate.starts_with(**verb))
        {
            line = format!("{noun} {}s {}", verb.trim_end(), &predicate[verb.len()..]);
        }
    }
    // "Creatures with flying can't block enchanted creature" is the attached
    // object's evasion: "Enchanted creature can't be blocked by creatures
    // with flying".
    if let Some(body) = line.strip_suffix('.')
        && let Some((blockers, target)) = body.split_once(" can't block ")
        && ["enchanted creature", "equipped creature", "this creature"].contains(&target)
        && !blockers.contains(" as long as ")
    {
        let lower = blockers.to_ascii_lowercase();
        let blockers = if let Some(quality) = blockers
            .strip_prefix("Creatures with ")
            .or_else(|| blockers.strip_prefix("Creature with "))
        {
            Some(format!("creatures with {quality}"))
        } else if let Some(color) = ["white", "blue", "black", "red", "green"].iter().find(|color| {
            lower == format!("{color} creature") || lower == format!("{color} creatures")
        }) {
            Some(format!("{color} creatures"))
        } else if !blockers.contains(' ')
            && blockers.ends_with('s')
            && blockers.chars().next().is_some_and(|ch| ch.is_ascii_uppercase())
            && blockers != "Creatures"
        {
            Some(blockers.to_string())
        } else {
            None
        };
        if let Some(blockers) = blockers {
            line = format!("{} can't be blocked by {blockers}.", capitalize_first(target));
        }
    }
    let mut line = line
        .replace("This source", &capitalize_first(subject))
        .replace("this source", subject)
        .replace("This permanent", &capitalize_first(subject))
        .replace("this permanent", subject)
        .replace("cast this spells ", "cast this card ")
        .replace(" as long as it isn't the case that ", " unless ")
        .replace("Enchanted creature with this Aura attached to it", "Enchanted creature")
        .replace("Equipped creature with this Equipment attached to it", "Equipped creature");
    for verb in ["enters", "has", "can't", "gets"] {
        if let Some(rest) = line.strip_prefix(&format!("This {verb} ")) {
            line = format!("{} {verb} {rest}", capitalize_first(subject));
        }
    }
    // A granted renown expansion is the keyword (CR 702.112a): "Attacking
    // creatures you control have first strike and renown 1."
    for quote in ["\"", ""] {
        let head = format!("{quote}Whenever this creature deals combat damage to a player, if this creature isn't renowned, put ");
        if let Some(start) = line.find(&head)
            && let Some((count, rest)) = line[start + head.len()..].split_once(" +1/+1 counter")
            && let Ok(count) = count.parse::<u32>()
            && let Some(rest) = rest
                .strip_prefix("s on it and it becomes renowned")
                .or_else(|| rest.strip_prefix(" on it and it becomes renowned"))
                .and_then(|rest| rest.strip_prefix(quote))
        {
            line = format!("{}renown {count}{rest}", &line[..start]);
            break;
        }
    }
    // A threshold-style count reads as the cards present: "as long as
    // there are seven or more cards in your graveyard".
    line = line.replace(
        " as long as you have seven or more cards in your graveyard",
        " as long as there are seven or more cards in your graveyard",
    );
    if subject != "this card"
        && let Some(rest) = line.strip_prefix("This card is ")
    {
        line = format!("{} is {rest}", capitalize_first(subject));
    }
    // A self restriction lifted by its own state: "This creature can't
    // attack or block unless it's equipped."
    if line.starts_with(&format!("{} can't ", capitalize_first(subject))) {
        line = line
            .replace(&format!(" as long as {subject} isn't "), " unless it's ")
            .replace(&format!(" as long as {subject} doesn't have "), " unless it has ");
    }
    // "This creature enters with five +1/+1 counters on it."
    if let Some(rest) = line.strip_prefix("Enters the battlefield with ") {
        line = format!("{} enters with {rest}", capitalize_first(subject));
    }
    // "Equip abilities you activate cost {1} less to activate."
    if let Some(rest) = line.strip_prefix("Activated abilities of a permanent you control cost ")
        && let Some((amount, condition)) = rest.split_once(" less to activate if it's an equip ability")
    {
        let target = condition.trim_end_matches('.');
        let target = target.replace(" that targets ", " that target ");
        line = format!("Equip abilities you activate{target} cost {amount} less to activate.");
    }
    // A self restriction read without its subject ("Can't block.").
    for verb in ["Can't ", "Doesn't ", "Can ", "Attacks each combat ", "Blocks each combat "] {
        if let Some(rest) = line.strip_prefix(verb) {
            line = format!("{} {}{rest}", capitalize_first(subject), lowercase_first(verb));
        }
    }
    // "Creatures with power less than this creature's power can't block it"
    // is how a power-relative evasion reads.
    if let Some(blockers) = line
        .strip_prefix(&format!("{} can't be blocked by ", capitalize_first(subject)))
        .and_then(|rest| rest.strip_suffix('.'))
        .filter(|rest| {
            rest.starts_with("creatures with power ")
                && rest.contains(" than ")
                && !rest.contains(" as long as ")
        })
    {
        // Oracle prints the greater-power form as "can't be blocked by
        // creatures with greater power" (Prehistoric Pet, Ant-Man, Locke).
        if blockers == format!("creatures with power greater than {subject}'s power") {
            line = format!("{} can't be blocked by creatures with greater power.", capitalize_first(subject));
        } else {
            line = format!("{} can't block it.", capitalize_first(blockers));
        }
    }
    // "can't be blocked except by black creatures": a single negated
    // blocker quality reads as its exception.
    if let Some((head, rest)) = line.split_once(" can't be blocked by non")
        && let Some((qualities, tail)) = rest.split_once(" creatures")
    {
        let qualities = qualities.split(" non").collect::<Vec<_>>();
        if qualities.iter().all(|quality| !quality.is_empty() && !quality.contains(' ')) {
            let allowed = qualities
                .iter()
                .map(|quality| match quality.strip_prefix('-') {
                    // "non-Wall creatures" -> "Walls"
                    Some(subtype) => pluralize_noun_phrase(subtype),
                    None => format!("{quality} creatures"),
                })
                .collect::<Vec<_>>()
                .join(" and/or ");
            line = format!("{head} can't be blocked except by {allowed}{tail}");
        }
    }
    // A permission to play extra lands is a standing rule of each turn.
    // Only the extra-land-drop permission takes the clause; "play lands from
    // your graveyard" already needs your turn (CR 305.2) and prints bare.
    if line.starts_with("You may play ")
        && (line.contains(" additional land") || line.contains("any number of lands"))
        && !line.contains("turn")
        && let Some(body) = line.strip_suffix('.')
    {
        line = match body.split_once(" as long as ") {
            Some((rule, condition)) => format!("{rule} on each of your turns as long as {condition}."),
            None => format!("{body} on each of your turns."),
        };
    }
    if let Some(rest) = line.strip_prefix("Players may play ")
        && rest.contains("additional land")
        && !rest.contains("turn")
        && let Some(body) = rest.strip_suffix('.')
    {
        line = format!("Each player may play {body} on each of their turns.");
    }
    // "You may cast a creature you own spells from the top of your
    // library": the cards in your library are yours, so the kind is the
    // spell's ("cast creature spells from the top of your library").
    if let Some((before, after)) = line.split_once(" you own spells ")
        && let Some((lead, kind)) = before.rsplit_once(" cast ")
        && after.contains("your library")
    {
        let kind = kind.strip_prefix("a ").or_else(|| kind.strip_prefix("an ")).unwrap_or(kind);
        line = format!("{lead} cast {kind} spells {after}");
    }
    line = line.replace(" cast spells you own from the top of your library", " cast spells from the top of your library");
    // "This creature can't be blocked except by creatures with defender."
    if let Some(rest) = line.strip_prefix("Creatures without ")
        && let Some((quality, noun)) = rest.split_once(" can't block this ")
        && !quality.contains(' ')
    {
        line = format!("This {} can't be blocked except by creatures with {quality}.", noun.trim_end_matches('.'));
    }
    // "Creatures with flying can block only creatures with flying."
    if let Some((blockers, rest)) = line.split_once(" can't block creature without ")
        .or_else(|| line.split_once(" can't block creatures without "))
        && !rest.trim_end_matches('.').contains(' ')
    {
        line = format!("{blockers} can block only creatures with {}", rest);
    }
    // "Slivers can't be blocked except by Slivers", "This creature can't
    // be blocked except by Walls and/or creatures with flying".
    if let Some(rewritten) = audit_blocker_exception(&line) {
        line = rewritten;
    }
    // "placed" and "put" are the same counter action (CR 122.6).
    if let Some(rest) = line.strip_prefix("Counters can't be placed on ") {
        line = format!("Counters can't be put on {rest}");
    }
    // "Counters can't be put on artifacts, creatures, enchantments, or lands."
    if let Some(list) = line
        .strip_prefix("Counters can't be put on ")
        .and_then(|rest| rest.strip_suffix('.'))
        .filter(|list| list.contains(", or "))
    {
        let nouns = list.split(", ").map(|noun| noun.strip_prefix("or ").unwrap_or(noun)).collect::<Vec<_>>();
        if nouns.iter().all(|noun| ["artifact", "creature", "enchantment", "land", "planeswalker", "battle"].contains(noun)) {
            let plural = nouns.iter().map(|noun| format!("{noun}s")).collect::<Vec<_>>();
            let (last, init) = plural.split_last().expect("a list");
            line = format!("Counters can't be put on {}, or {last}.", init.join(", "));
        }
    }
    if let Some(rest) = line.strip_prefix("Counters can't be put on ")
        && let Some(object) = rest.strip_suffix('.')
        && (object.starts_with("this ")
            || ["enchanted creature", "equipped creature", "enchanted permanent"].contains(&object))
    {
        line = format!("{} can't have counters put on it.", capitalize_first(object));
    }
    line = line
        .replace(" as long as it is not your turn", " during turns other than yours")
        .replace(" while 1 is equal to 1", "");
    if let Some((head, rest)) = line.split_once(" can't ")
        && let Some(noun) = head.strip_prefix("A ").or_else(|| head.strip_prefix("An "))
        && !noun.contains(" and ")
        && !noun.contains(" or ")
        && !noun.contains(" who ")
        && !noun.contains(" has ")
        && !noun.contains(" than ")
        && !noun.contains([',', '"'])
        && noun.split(' ').count() <= 5
    {
        line = format!(
            "{} can't {rest}",
            capitalize_first(&pluralize_relative_object_phrase(noun))
        );
    }
    // "All creatures able to block this creature do so."
    if let Some(noun) = line
        .strip_prefix("Creature must block this ")
        .or_else(|| line.strip_prefix("Creatures must block this "))
        .and_then(|rest| rest.strip_suffix(" if able."))
        .filter(|noun| !noun.contains(' '))
    {
        line = format!("All creatures able to block this {noun} do so.");
    }
    // "All Walls able to block this creature do so." (Marble Priest), "All
    // creatures with flying able to block this creature do so." (Talruum
    // Piper): a qualified lure keeps its blocker quality.
    if let Some((blockers, noun)) = line
        .strip_suffix(" if able.")
        .and_then(|rest| rest.split_once(" must block this "))
        .filter(|(_, noun)| !noun.contains(' '))
    {
        let blockers = if let Some(quality) = blockers
            .strip_prefix("Creature with ")
            .or_else(|| blockers.strip_prefix("Creatures with "))
            .filter(|quality| !quality.contains(' '))
        {
            Some(format!("creatures with {quality}"))
        } else if let Some(subtype) = blockers
            .strip_suffix(" creature")
            .or_else(|| blockers.strip_suffix(" creatures"))
            .filter(|subtype| {
                !subtype.contains(' ') && subtype.chars().next().is_some_and(|ch| ch.is_ascii_uppercase())
            })
        {
            Some(pluralize_noun_phrase(subtype))
        } else {
            None
        };
        if let Some(blockers) = blockers {
            line = format!("All {blockers} able to block this {noun} do so.");
        }
    }
    // A conditional lure leads with its condition (Stone-Tongue Basilisk).
    if let Some((noun, condition)) = line
        .strip_prefix("Creature must block this ")
        .or_else(|| line.strip_prefix("Creatures must block this "))
        .and_then(|rest| rest.strip_suffix('.'))
        .and_then(|rest| rest.split_once(" if able as long as "))
        .filter(|(noun, _)| !noun.contains(' '))
    {
        line = format!("As long as {condition}, all creatures able to block this {noun} do so.");
    }
    // "except by three or more creatures"
    if let Some((head, rest)) = line.split_once(" except by ")
        && let Some((count, tail)) = rest.split_once(" or more ")
        && let Ok(count) = count.parse::<u32>()
        && let Some(word) = ironsmith_core::cardinal_word(count)
    {
        line = format!("{head} except by {word} or more {tail}");
    }
    // "Creatures your opponents control enter tapped."
    if let Some(at) = line.find(" an opponent controls ")
        && line[..at].ends_with('s')
        && !line[..at].contains(' ')
    {
        line = format!("{} your opponents control {}", &line[..at], &line[at + " an opponent controls ".len()..]);
    }
    line = line.replace(" enter the battlefield tapped", " enter tapped");
    // A bare singular subject names every such object: "Creature with
    // flying can't block" is "Creatures with flying can't block".
    if let Some((head, rest)) = line.split_once(" can't ")
        && is_bare_singular_subject(head)
    {
        line = format!("{} can't {rest}", capitalize_first(&pluralize_relative_object_phrase(head)));
    }
    // A granted keyword reads in keyword case, and a granted restriction
    // ("has Can't be blocked") is that restriction on the subject.
    for verb in [" has ", " have "] {
        if let Some((head, predicate)) = line.split_once(verb)
            && !head.contains('"')
        {
            // A grant to every matching object reads in the plural
            // ("Zombies you control have ..."), never the indefinite "A
            // Zombie you control has"; a compound subject keeps "Each".
            // "Each Wolf and Werewolf you control has haste": two single
            // kinds joined by "and" name every object of either kind.
            let compound = (verb == " has ")
                .then(|| head.strip_prefix("Each "))
                .flatten()
                .and_then(|rest| {
                    let (other, rest) = match rest.strip_prefix("other ") {
                        Some(rest) => ("Other ", rest),
                        None => ("", rest),
                    };
                    let (kinds, tail) = rest.split_once(" you control")?;
                    let (first, second) = kinds.split_once(" and ")?;
                    let simple = |kind: &str| {
                        !kind.is_empty()
                            && !kind.contains(' ')
                            && (kind.chars().next().is_some_and(|ch| ch.is_ascii_uppercase())
                                || ["creature", "land", "artifact", "enchantment", "planeswalker"].contains(&kind))
                    };
                    (tail.is_empty() && simple(first) && simple(second)).then(|| {
                        let joined = format!("{} and {}", pluralize_word(first), pluralize_word(second));
                        let joined = if other.is_empty() { capitalize_first(&joined) } else { joined };
                        format!("{other}{joined} you control")
                    })
                });
            // "Each creature card in your hand has cycling {1}{U}": a grant
            // to the cards in a zone reads per card, as printed.
            let each_card = (verb == " have ")
                .then(|| {
                    let (kind, zone) = head.split_once(" cards in your ")?;
                    (["hand", "graveyard"].contains(&zone) && !kind.is_empty() && !kind.contains(' ')).then(|| {
                        let kind = if [
                            "Creature", "Land", "Artifact", "Enchantment", "Instant", "Sorcery",
                            "Nonland", "Noncreature", "Historic", "Permanent", "Legendary",
                        ]
                        .contains(&kind)
                        {
                            kind.to_ascii_lowercase()
                        } else {
                            kind.to_string()
                        };
                        format!("Each {kind} card in your {zone}")
                    })
                })
                .flatten();
            let compound = compound.map(|compound| (compound, " have ")).or(each_card.map(|each| (each, " has ")));
            let (head, verb) = if let Some(compound) = compound {
                compound
            } else if verb == " has " && is_bare_singular_subject(head) {
                let plural = pluralize_relative_object_phrase(head);
                // A card type reads lowercase ("All creatures"); a subtype
                // keeps its capital ("All Slivers").
                let plural = if ["Creature", "Artifact", "Land", "Enchantment", "Planeswalker", "Permanent"]
                    .contains(&head)
                {
                    plural.to_ascii_lowercase()
                } else {
                    plural
                };
                // "All Slivers", "All Sliver creatures"; a color stays bare
                // ("White creatures have ...").
                let all = !head.contains(' ')
                    || (is_qualified_creature_subject(head)
                        && !head.starts_with("Non")
                        && !["Artifact", "Enchantment", "Land", "Legendary", "Basic", "Snow", "Token", "Planeswalker"]
                            .contains(&head.split(' ').next().unwrap_or(""))
                        && crate::color::Color::from_name(&head.split(' ').next().unwrap_or("").to_ascii_lowercase()).is_none());
                let head = if all { format!("All {plural}") } else { capitalize_first(&plural) };
                (head, " have ")
            } else if let Some(subtype) = head.strip_suffix(" creature").filter(|subtype| {
                verb == " has "
                    && !subtype.contains(' ')
                    && subtype.chars().next().is_some_and(|ch| ch.is_ascii_uppercase())
                    && !["A", "An", "Another", "Each", "Enchanted", "Equipped", "This", "That", "Target", "Other"]
                        .contains(subtype)
            }) {
                // "Sliver creature has haste": every Sliver creature.
                (format!("All {subtype} creatures"), " have ")
            } else { match head.split_once(' ') {
                Some((article @ ("A" | "An" | "Another"), rest)) if verb == " has " => {
                    let other = if article == "Another" { "other " } else { "" };
                    if rest.contains(" and ") || rest.contains(" or ") || rest.contains(" on it") {
                        // "A Wolf and Werewolf you control has haste": a
                        // conjunctive two-kind set reads in the plural
                        // ("Wolves and Werewolves you control have haste");
                        // a grant to "a Goblin or Elemental" reaches every
                        // object of either kind (Caterwauling Boggart).
                        let kinds = if rest.contains(" and ") { rest.to_string() } else { rest.replacen(" or ", " and ", 1) };
                        match audit_each_conjunctive_grant(&format!("Each {other}{kinds} has x"))
                            .filter(|_| !predicate.starts_with('"'))
                            .and_then(|plural| plural.strip_suffix(" have x").map(str::to_string))
                        {
                            Some(plural) => (plural, " have "),
                            None => (format!("Each {other}{rest}"), verb),
                        }
                    } else {
                        let plural = pluralize_relative_object_phrase(rest);
                        (capitalize_first(&format!("{other}{plural}")), " have ")
                    }
                }
                _ => (head.to_string(), verb),
            } };
            let body = predicate.trim_end_matches('.');
            // A granted self restriction ("has this Aura can't attack").
            let body = ["this Aura ", "this creature ", "This creature ", "this permanent "]
                .iter()
                .find_map(|subject| body.strip_prefix(subject).filter(|rest| rest.starts_with("can't ")))
                .map_or(body.to_string(), |rest| capitalize_first(rest));
            let body = body.as_str();
            if let Some(restriction) = body.strip_prefix("Can't ") {
                return format!("{head} can't {restriction}.");
            }
            // A granted damage-assignment rule is the subject's own rule
            // ("Enchanted creature assigns combat damage equal to its
            // toughness rather than its power").
            if let Some(rest) = body
                .strip_prefix("This creature assigns ")
                .or_else(|| body.strip_prefix("this creature assigns "))
            {
                let assign = if verb == " have " { "assign" } else { "assigns" };
                return format!("{head} {assign} {rest}.");
            }
            // "Enchanted creature's controller may have it assign its combat
            // damage as though it weren't blocked": the granted ability's
            // "you" is the creature's controller.
            if verb == " has "
                && (head == "Enchanted creature" || head == "Equipped creature")
                && let Some(rest) = body.strip_prefix("You may have this creature ")
            {
                return format!("{head}'s controller may have it {rest}.");
            }
            // A granted prevention of damage to and by the object itself.
            if verb == " has "
                && (head == "Enchanted creature" || head == "Equipped creature")
                && let Some(scope) = body.strip_prefix("Prevent all ").and_then(|rest| {
                    ["this Aura", "this Equipment", "this creature"]
                        .iter()
                        .find_map(|object| rest.strip_suffix(object))
                })
                && (scope.ends_with(" dealt to ") || scope.ends_with(" dealt by "))
            {
                return format!("Prevent all {scope}{}.", lowercase_first(&head));
            }
            // A granted permission or combat requirement is the subject's
            // own rule ("Creatures you control can block an additional
            // creature each combat", "Goblin creatures attack each combat").
            if let Some(permission) = body
                .strip_prefix("Can ")
                .or_else(|| body.strip_prefix("This creature can "))
                .or_else(|| body.strip_prefix("this creature can "))
            {
                // "Creatures you control can attack as though they didn't
                // have defender": the pronoun follows the plural subject.
                let permission = if verb == " have " {
                    permission
                        .replace(" as though it were ", " as though they were ")
                        .replace(" as though it didn't ", " as though they didn't ")
                        .replace(" as though it had ", " as though they had ")
                } else {
                    permission.to_string()
                };
                return format!("{head} can {permission}.");
            }
            for requirement in ["Attacks ", "Blocks "] {
                if let Some(rest) = body.strip_prefix(requirement) {
                    let action = requirement.trim_end().to_ascii_lowercase();
                    let action = if verb == " have " { action.trim_end_matches('s').to_string() } else { action };
                    let head = head.strip_prefix("All ").map_or(head.clone(), capitalize_first);
                    return format!("{head} {action} {rest}.");
                }
            }
            // "Enchanted creature has creature must block this Aura if able":
            // a lure on the attached object (CR 509.1c).
            if let Some(tail) = body
                .strip_prefix("creature must block this ")
                .or_else(|| body.strip_prefix("Creature must block this "))
                .and_then(|rest| rest.split_once(" if able").map(|(_, tail)| tail))
                && (head.starts_with("Enchanted ") || head.starts_with("Equipped "))
            {
                let object = lowercase_first(&head);
                return match tail.strip_prefix(" as long as ") {
                    Some(condition) => format!("As long as {condition}, all creatures able to block {object} do so."),
                    None if tail.is_empty() => format!("All creatures able to block {object} do so."),
                    None => format!("All creatures able to block {object} do so{tail}."),
                };
            }
            let normalized = match body.split_once(" during ") {
                Some((keyword, rest)) if merge_passes::is_keyword_phrase(keyword) => {
                    format!("{} during {rest}", keyword.to_ascii_lowercase())
                }
                _ => merge_passes::normalize_keyword_predicate_case(body),
            };
            // A granted keyword trigger is the whole keyword; it may arrive
            // quoted ("have \"When you cast this spell, ...\"").
            let keyword_of = |text: &str| -> Option<String> {
                // Ripple (CR 702.60a).
                let ripple = text
                    .strip_prefix("When you cast this spell, you may reveal the top ")
                    .and_then(|rest| rest.split_once(" cards of your library; you may cast any revealed cards with the same name as this spell without paying their mana costs, then put all revealed cards not cast this way on the bottom in any order"))
                    .filter(|(count, tail)| count.parse::<u32>().is_ok() && tail.trim_end_matches('.').is_empty())
                    .map(|(count, _)| format!("ripple {count}"));
                if ripple.is_some() {
                    return ripple;
                }
                // Exploit and evolve (CR 702.110a, 702.100a).
                let exploit = matches!(
                    text
                        .strip_prefix("When this creature enters, you may sacrifice a creature.")
                        .map(|rest| rest.trim().trim_end_matches('.')),
                    Some("" | "If you do,")
                );
                if exploit {
                    return Some("exploit".to_string());
                }
                if text.trim_end_matches('.')
                    == "Whenever a creature you control enters, if that creature's power is greater than this creature's power and/or that creature's toughness is greater than this creature's toughness, evolve"
                {
                    return Some("evolve".to_string());
                }
                // Training (CR 702.149a); an empty follow-up reflexive
                // check adds nothing.
                matches!(
                    text
                        .strip_prefix("Whenever this creature attacks with another creature with greater power, put a +1/+1 counter on this creature.")
                        .map(|rest| rest.trim().trim_end_matches('.')),
                    Some("" | "If its count is greater than 0,")
                )
                .then(|| "training".to_string())
            };
            let unquoted = normalized
                .strip_prefix('"')
                .map(|rest| rest.trim_end_matches('"').trim_end_matches('.').to_string());
            let normalized = keyword_of(&normalized)
                .or_else(|| unquoted.as_deref().and_then(keyword_of))
                .unwrap_or(normalized);
            // Spell keywords granted to spells read in keyword case too
            // ("have split second", "has casualty 1").
            let spell_keyword = matches!(
                normalized.split_once(' ').map_or(normalized.as_str(), |(word, _)| word),
                "Rebound" | "Split" | "Cascade" | "Casualty" | "Convoke" | "Delve" | "Improvise"
                    | "Storm" | "Replicate" | "Ripple" | "Demonstrate" | "Jump-start" | "Retrace"
            ) && !normalized.contains(['.', ',', ':']);
            let normalized = if spell_keyword { lowercase_first(&normalized) } else { normalized };
            // "Spell cast by you has cascade": every spell you cast.
            let renamed: Option<(&str, &str)> = match head.as_str() {
                "Spell cast by you" => Some(("Spells you cast", " have ")),
                "Instant cast by you and sorcery cast by you" => {
                    Some(("Each instant and sorcery spell you cast", " has "))
                }
                "Each instant and sorcery spell you control" => {
                    Some(("Instant and sorcery spells you control", " have "))
                }
                _ => None,
            };
            // "Creature spell cast by you has convoke" / "Enchantment card
            // cast by you in hand has cascade": every such spell you cast.
            let renamed_kind = renamed.is_none().then(|| {
                let (kind, origin) = if let Some(kind) = head.strip_suffix(" spell cast by you") {
                    (kind, "")
                } else {
                    (head.strip_suffix(" card cast by you in hand")?, " from your hand")
                };
                (!kind.is_empty() && !kind.contains(' ') && !kind.ends_with('s'))
                    .then(|| format!("{kind} spells you cast{origin}"))
            }).flatten();
            let (head, verb) = match (renamed, renamed_kind) {
                (Some((renamed, renamed_verb)), _) => (renamed.to_string(), renamed_verb),
                (None, Some(renamed)) => (renamed, " have "),
                (None, None) => (head, verb),
            };
            let end = if predicate.ends_with('.') { "." } else { "" };
            return format!("{head}{verb}{normalized}{end}");
        }
    }
    line
}

pub fn compiled_text_lines(def: &CardDefinition) -> Vec<String> {
    let oracle_short = oracle_short_self_name(def);
    let lines = normalize_ast_surface_lines(debug_compiled_surface_lines(def))
        .into_iter()
        .map(|line| ast_render::rewrite_eminence_source_zone_surface(def, &line))
        .map(|line| ast_render::rewrite_commander_tax_life_surface(def, &line))
        .map(|line| {
            substitute_legendary_source_reference(&line, &def.card, "", oracle_short.as_deref())
        })
        .map(|line| substitute_spell_caster_source_reference(&line, def))
        .map(|line| substitute_kicked_draw_source_reference(&line, def))
        .collect();
    let lines = compact_post_substitution_surface_lines(lines)
        .into_iter()
        .map(normalize_scored_compiled_line)
        .map(uppercase_standalone_variable_x)
        .map(attacked_player_back_reference_surface)
        .map(optional_additional_tapped_mana_surface)
        .map(quote_granted_cumulative_upkeep_surface)
        .map(|line| normalize_punctuated_card_name_damage_case(line, &def.card.name))
        .map(|line| restore_lowercased_card_name_case(line, &def.card.name))
        .map(restore_cleave_bracket_surface)
        .collect();
    let lines = preserve_as_long_as_its_your_turn_surface(def, lines);
    prefix_attraction_visit_surface(def, append_typed_standard_reminder_lines(def, lines))
}

/// "That creature gains \"Cumulative upkeep {2}.\"" (Dreams of the Dead,
/// Balduvian Shaman): a granted cumulative upkeep is printed as a quoted
/// ability, never as a bare keyword after "gains".
fn quote_granted_cumulative_upkeep_surface(line: String) -> String {
    const MARKER: &str = " gains cumulative upkeep ";
    let Some(start) = line.find(MARKER) else {
        return line;
    };
    let rest = &line[start + MARKER.len()..];
    let end = rest.find('.').unwrap_or(rest.len());
    let cost = rest[..end].trim();
    if cost.is_empty() || cost.contains('"') || cost.contains(',') {
        return line;
    }
    let after = rest.get(end + 1..).unwrap_or("");
    format!(
        "{} gains \"Cumulative upkeep {cost}.\"{after}",
        &line[..start]
    )
}

/// "Whenever an Island is tapped for mana, its controller may add an
/// additional {U}. If that Island is snow, its controller may add an
/// additional {U}{U} instead." (Snowfall): the optional tapped-for-mana
/// trigger's mana is added on top of the tapped land's own mana, and the
/// effect's player is the tapped land's controller, so the trigger reads in
/// the passive voice like the non-optional "adds an additional" family.
fn optional_additional_tapped_mana_surface(line: String) -> String {
    const HEAD: &str = "Whenever a player taps ";
    const ADD: &str = " for mana, its controller may add {";
    let Some(rest) = line.strip_prefix(HEAD) else {
        return line;
    };
    let Some((object, after)) = rest.split_once(ADD) else {
        return line;
    };
    if object.contains(',') || object.contains(" for ") {
        return line;
    }
    let mut out = format!(
        "Whenever {object} is tapped for mana, its controller may add an additional {{{after}"
    );
    // A follow-up "If it's snow, ... instead" names the tapped object.
    let noun = object
        .strip_prefix("an ")
        .or_else(|| object.strip_prefix("a "))
        .filter(|noun| !noun.contains(' '));
    if let Some(noun) = noun {
        for quality in ["snow", "tapped", "untapped"] {
            let from = format!(". If it's {quality}, its controller may add {{");
            let to = format!(". If that {noun} is {quality}, its controller may add an additional {{");
            out = out.replace(&from, &to);
        }
    }
    out
}

/// "Whenever a creature attacks one of your opponents, that player loses 1
/// life." (Calculating Lich): when the trigger itself names the attacked
/// player, the effect's defending-player reference is that player. Only the
/// trigger's own sentence is rewritten, and only when the attacked object
/// must be a player (no "or planeswalker" arm).
fn attacked_player_back_reference_surface(line: String) -> String {
    let Some((head, rest)) = line.split_once(", ") else {
        return line;
    };
    if !(head.starts_with("Whenever ") || head.starts_with("When "))
        || head.contains("defending")
        || head.contains("planeswalker")
        || ![" attacks a player", " attacks one of your opponents", " attacks an opponent"]
            .iter()
            .any(|marker| head.contains(marker))
        || !rest.contains("the defending player")
    {
        return line;
    }
    let (sentence, tail) = match rest.find(". ") {
        Some(end) => rest.split_at(end),
        None => (rest, ""),
    };
    format!(
        "{head}, {}{tail}",
        sentence.replace("the defending player", "that player")
    )
}

/// A granted ability's display can carry lowercased source words, so the
/// variable reads "+x/+x ... where x is". A standalone lowercase `x` is never
/// an English word in rules text; it is always the variable X.
fn uppercase_standalone_variable_x(line: String) -> String {
    if !line.contains('x') {
        return line;
    }
    let chars = line.chars().collect::<Vec<_>>();
    let mut out = String::with_capacity(line.len());
    for (index, ch) in chars.iter().enumerate() {
        let standalone = *ch == 'x'
            && index
                .checked_sub(1)
                .and_then(|prev| chars.get(prev))
                .is_none_or(|prev| !prev.is_alphanumeric() && *prev != '\'')
            && chars
                .get(index + 1)
                .is_none_or(|next| !next.is_alphanumeric() && *next != '\'');
        out.push(if standalone { 'X' } else { *ch });
    }
    out
}

pub fn unprocessed_compiled_lines(def: &CardDefinition) -> Vec<String> {
    let oracle_short = oracle_short_self_name(def);
    let lines = normalize_ast_surface_lines(debug_compiled_surface_lines(def))
        .into_iter()
        .map(|line| {
            substitute_legendary_source_reference(&line, &def.card, "", oracle_short.as_deref())
        })
        .map(|line| substitute_spell_caster_source_reference(&line, def))
        .collect();
    let lines = compact_post_substitution_surface_lines(lines)
        .into_iter()
        .map(normalize_unprocessed_compiled_line)
        .map(|line| normalize_duplicate_optional_subject(&line))
        .map(|line| normalize_punctuated_card_name_damage_case(line, &def.card.name))
        .map(|line| restore_lowercased_card_name_case(line, &def.card.name))
        .map(restore_cleave_bracket_surface)
        .collect();
    let lines = preserve_as_long_as_its_your_turn_surface(def, lines);
    prefix_attraction_visit_surface(def, append_typed_standard_reminder_lines(def, lines))
}

fn preserve_as_long_as_its_your_turn_surface(
    def: &CardDefinition,
    mut lines: Vec<String>,
) -> Vec<String> {
    let mut authored_count = def
        .abilities
        .iter()
        .filter(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.compiled_model().is_some_and(|model| {
                        model.label.starts_with(
                            ironsmith_core::static_ability_model::
                                AS_LONG_AS_ITS_YOUR_TURN_STATIC_LABEL_PREFIX,
                        )
                    })
            )
        })
        .count();
    if authored_count == 0 {
        return lines;
    }
    for line in &mut lines {
        if authored_count == 0 {
            break;
        }
        if let Some(body) = line.strip_prefix("During your turn, ") {
            *line = format!("As long as it's your turn, {body}");
            authored_count -= 1;
        }
    }
    lines
}

/// A card name's terminal `!` or `?` is part of the name, not a sentence
/// boundary. Generic sentence casing cannot know that without card context,
/// so restore the verb casing after the final normalization pass.
fn normalize_punctuated_card_name_damage_case(line: String, card_name: &str) -> String {
    if !card_name.ends_with('!') && !card_name.ends_with('?') {
        return line;
    }
    line.replace(
        &format!("{card_name} Deals "),
        &format!("{card_name} deals "),
    )
    .replace(&format!("{card_name} Deal "), &format!("{card_name} deal "))
}

/// A granted ability's display carries lowercased source words, so the card's
/// own multi-word name inside the quote reads "Hold for ransom's controller".
/// Restore the printed casing of an occurrence that starts with the name's own
/// capitalized first letter and differs from it only by case.
fn restore_lowercased_card_name_case(line: String, card_name: &str) -> String {
    let name = card_name.split(" // ").next().unwrap_or(card_name);
    if name.split_whitespace().count() < 2
        || !name.is_ascii()
        || !line.is_ascii()
        || !name.chars().next().is_some_and(|ch| ch.is_ascii_uppercase())
    {
        return line;
    }
    let lower_line = line.to_ascii_lowercase();
    let lower_name = name.to_ascii_lowercase();
    let mut out = String::with_capacity(line.len());
    let mut cursor = 0;
    while let Some(found) = lower_line[cursor..].find(&lower_name) {
        let start = cursor + found;
        let end = start + name.len();
        let occurrence = &line[start..end];
        let bounded = line[..start]
            .chars()
            .next_back()
            .is_none_or(|ch| !ch.is_ascii_alphanumeric())
            && line[end..]
                .chars()
                .next()
                .is_none_or(|ch| !ch.is_ascii_alphanumeric());
        out.push_str(&line[cursor..start]);
        if bounded && occurrence != name && occurrence.as_bytes()[0] == name.as_bytes()[0] {
            out.push_str(name);
        } else {
            out.push_str(occurrence);
        }
        cursor = end;
    }
    out.push_str(&line[cursor..]);
    out
}

fn prefix_attraction_visit_surface(def: &CardDefinition, mut lines: Vec<String>) -> Vec<String> {
    if def.card.subtypes.contains(&Subtype::Attraction)
        && let Some(line) = lines.iter_mut().find(|line| !line.trim().is_empty())
        && !line.starts_with("Visit — ")
    {
        *line = format!("Visit — {line}");
    }
    lines
}

/// Render a single ability using the same surface renderer as compiled oracle text.
pub fn ability_surface_text(ability: &Ability) -> String {
    // An authored ability-word label retained on a conditional static
    // ("Threshold — This creature has flying as long as ...") prefixes the
    // rendered body regardless of the payload shape.
    if let AbilityKind::Static(static_ability) = &ability.kind
        && let Some(model) = static_ability.compiled_model()
        && let Some(label) = model.label.strip_prefix(
            ironsmith_core::static_ability_model::EXPLICIT_STATIC_PRESENTATION_LABEL_PREFIX,
        )
    {
        let label = label.to_string();
        let body = self::render_effects::describe_inline_ability(ability);
        return format!("{label} — {body}");
    }
    if let Some(keyword) = self::render_effects::describe_keyword_ability(ability) {
        return keyword;
    }
    // CR 702.84a: a granted sorcery-speed activation whose only effect is
    // the unearth effect is the unearth keyword ("has unearth {2}{B}").
    if ironsmith_core::semantic_audit::semantic_audit_active()
        && let AbilityKind::Activated(activated) = &ability.kind
        && activated.choices.is_empty()
        && matches!(activated.timing, ActivationTiming::SorcerySpeed)
        && activated.effects.segments.len() == 1
        && activated.effects.segments[0].self_replacements.is_empty()
        && activated.effects.segments[0].default_effects.len() == 1
        && activated.effects.segments[0].default_effects[0]
            .downcast_ref::<crate::effects::UnearthEffect>()
            .is_some_and(|effect| !effect.descriptive)
    {
        return format!("Unearth {}", describe_total_cost(&activated.mana_cost));
    }
    self::render_effects::describe_inline_ability(ability)
}

/// Render labels for the executable abilities on a definition.
///
/// A label is the printed line an ability came from, so the list is always as
/// long as `def.abilities` and a runtime surface can name ability `i` with
/// `labels[i]`. When the canonical lines map one-to-one onto the abilities the
/// labels are those lines. Otherwise several abilities share one line (a pump
/// static and two keyword grants compiled out of "gets +1/+1 and has trample
/// and haste"), a line owns no ability at all (Class reminder text), or a
/// marker static prints nothing; the line each ability belongs to is then
/// recovered by rendering the definition one ability (or authored keyword
/// group) at a time and watching which line the newcomer changes. Only when
/// that fails does an ability fall back to its own single-ability rendering.
pub fn ability_surface_texts(def: &CardDefinition) -> Vec<String> {
    let canonical = compiled_text_lines(def);
    if canonical.len() == def.abilities.len() {
        return canonical;
    }

    printed_line_labels(def, &canonical)
        .unwrap_or_else(|| def.abilities.iter().map(ability_surface_text).collect())
}

/// The printed line behind each ability of a definition whose lines and
/// abilities are not one-to-one; `None` when the rendering is not stable under
/// truncation, so nothing could be attributed with confidence.
fn printed_line_labels(def: &CardDefinition, canonical: &[String]) -> Option<Vec<String>> {
    if def.abilities.is_empty() || canonical.is_empty() {
        return None;
    }
    let render_prefix = |count: usize| {
        let mut prefix = def.clone();
        prefix.abilities.truncate(count);
        prefix.ability_labels.clear();
        compiled_text_lines(&prefix)
    };
    let mut previous = render_prefix(0);
    let mut owners: Vec<Option<usize>> = Vec::with_capacity(def.abilities.len());
    let mut count = 0;
    while count < def.abilities.len() {
        // A keyword group only has its final line shape once all its keywords
        // are present. Rendering partial groups produces temporary extra lines
        // whose indices can otherwise point at an unrelated later ability.
        let group_size = source_line_keyword_group_count(&def.abilities[count])
            .map(|keywords| keywords.saturating_add(1))
            .unwrap_or(1)
            .min(def.abilities.len() - count);
        count += group_size;
        let lines = render_prefix(count);
        let owner = changed_line_index(&previous, &lines);
        owners.extend(std::iter::repeat_n(owner, group_size));
        previous = lines;
    }
    if previous != canonical {
        return None;
    }

    // An ability that changed nothing visible (a marker static, an ability
    // folded into a neighbour's wording) belongs with the next attributed
    // line, or the previous one when it is last.
    let mut resolved = vec![0usize; owners.len()];
    let mut next_owner = None;
    for index in (0..owners.len()).rev() {
        if let Some(line) = owners[index] {
            next_owner = Some(line);
        }
        resolved[index] = match next_owner {
            Some(line) => line,
            None => owners[..index]
                .iter()
                .rev()
                .find_map(|owner| *owner)
                .unwrap_or(0),
        };
    }
    Some(
        resolved
            .into_iter()
            .map(|line| canonical[line.min(canonical.len() - 1)].clone())
            .collect(),
    )
}

/// The first line that differs after one more ability is rendered: where a
/// new line was inserted, or where an existing line absorbed the ability.
fn changed_line_index(before: &[String], after: &[String]) -> Option<usize> {
    if before == after || after.is_empty() {
        return None;
    }
    let shared = before
        .iter()
        .zip(after)
        .take_while(|(old, new)| old == new)
        .count();
    Some(shared.min(after.len() - 1))
}

fn substitute_spell_caster_source_reference(line: &str, def: &CardDefinition) -> String {
    if !(def.card.is_instant() || def.card.is_sorcery()) || def.card.name.contains(" // ") {
        return line.to_string();
    }
    line.replace(
        "the player who cast this spell",
        &format!("the player who cast {}", def.card.name),
    )
}

fn normalize_ast_surface_lines(lines: Vec<String>) -> Vec<String> {
    let mut lines: Vec<String> = lines
        .into_iter()
        .map(|line| normalize_common_semantic_phrasing(&line))
        .collect();
    lines.retain(|line| line.trim() != "(Gain the next level as a sorcery to add its ability.)");
    // Several semantic surface repairs intentionally reason across adjacent
    // sentences of one authored ability (a repeated target, an `otherwise`
    // pronoun, linked search/shuffle text, and similar correlations). The
    // first pass above still cleans individual fragments, but those repairs
    // only become observable after the fragments are merged back into their
    // physical ability line.
    let merged = merge_ast_surface_lines(lines)
        .into_iter()
        .filter(|line| !ast_render::is_printed_line_boundary_sentinel(line))
        .map(|line| normalize_common_semantic_phrasing(&line))
        .collect();
    let lines = compact_shared_conditional_pump_and_ability_lines(merged)
        .into_iter()
        .map(finalize_ast_surface_line)
        .flat_map(expand_finalized_ast_surface_line)
        .map(normalize_mass_opponent_controller_surface)
        .collect();
    compact_threshold_ability_word_lines(compact_station_threshold_lines(
        collapse_self_other_abilities_grant(lines),
    ))
}

/// "It gains this card's other abilities." (Symbiote Spider-Man): a grant
/// whose quoted ability is exactly the card's only other printed ability.
fn collapse_self_other_abilities_grant(mut lines: Vec<String>) -> Vec<String> {
    if lines.len() != 2 {
        return lines;
    }
    for (grant_idx, other_idx) in [(1usize, 0usize), (0, 1)] {
        let grant = lines[grant_idx].clone();
        let Some(open) = grant.find(" gains \"") else {
            continue;
        };
        let quote_start = open + " gains \"".len();
        let Some(close_rel) = grant[quote_start..].find('"') else {
            continue;
        };
        let quoted = grant[quote_start..quote_start + close_rel].trim_end_matches('.');
        let Some((quoted_head, quoted_tail)) = quoted.split_once("this creature ") else {
            continue;
        };
        let other = lines[other_idx].trim().trim_end_matches('.');
        if quoted_head.is_empty()
            || quoted_tail.is_empty()
            || !other.starts_with(quoted_head)
            || !other.ends_with(quoted_tail)
            || other.len() < quoted_head.len() + quoted_tail.len()
        {
            continue;
        }
        let middle = &other[quoted_head.len()..other.len() - quoted_tail.len()];
        if middle.trim().is_empty() || middle.contains(", ") || middle.contains(". ") {
            continue;
        }
        let close = quote_start + close_rel + 1;
        let rest = &grant[close..];
        let joiner = if rest.is_empty() || rest.starts_with(' ') { "." } else { "" };
        lines[grant_idx] = format!(
            "{} gains this card's other abilities{joiner}{rest}",
            &grant[..open]
        );
        break;
    }
    lines
}

fn compact_shared_conditional_pump_and_ability_lines(lines: Vec<String>) -> Vec<String> {
    let has_hideaway = lines.iter().any(|line| {
        line.trim()
            .trim_end_matches('.')
            .strip_prefix("Hideaway ")
            .is_some_and(|amount| {
                !amount.is_empty() && amount.chars().all(|ch| ch.is_ascii_digit())
            })
    });
    let lines: Vec<String> = lines
        .into_iter()
        .map(|line| {
            let trimmed = line.trim().trim_end_matches('.');
            if trimmed.strip_prefix("Hideaway ").is_some_and(|amount| {
                !amount.is_empty() && amount.chars().all(|ch| ch.is_ascii_digit())
            }) {
                return trimmed.to_string();
            }
            if has_hideaway
                && trimmed
                    == "When this creature leaves the battlefield, put each card exiled with it into its owner's hand"
            {
                return "When this creature leaves the battlefield, put the exiled card into its owner's hand."
                    .to_string();
            }
            line
        })
        .collect();
    let mut compacted = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    while idx < lines.len() {
        if idx + 1 < lines.len()
            && let Some(line) =
                compact_shared_conditional_pump_and_ability(&lines[idx], &lines[idx + 1])
        {
            compacted.push(line);
            idx += 2;
            continue;
        }
        if idx + 1 < lines.len()
            && lines[idx].trim_end_matches('.') == "Create a 2/2 green Wolf creature token"
            && lines[idx + 1].trim_end_matches('.')
                == "Morbid — Create three of those tokens instead if a creature died this turn"
        {
            compacted.push(
                "Create a 2/2 green Wolf creature token. Morbid — Create three 2/2 green Wolf creature tokens instead if a creature died this turn."
                    .to_string(),
            );
            idx += 2;
            continue;
        }
        compacted.push(lines[idx].clone());
        idx += 1;
    }
    compacted
}

fn compact_shared_conditional_pump_and_ability(
    pump_line: &str,
    ability_line: &str,
) -> Option<String> {
    let pump = pump_line.trim().trim_end_matches('.');
    let ability = ability_line.trim().trim_end_matches('.');
    if let Some(rest) = pump.strip_prefix("As long as ")
        && let Some((condition, body)) = rest.split_once(", ")
        && let Some(subject) = body.strip_suffix(" gets +1/+3 and has reach")
        && ability.eq_ignore_ascii_case(&format!("{subject} has \"{{T}}: Add {{G}}{{G}}.\""))
    {
        return Some(format!(
            "As long as {condition}, {subject} gets +1/+3 and has reach and {{T}}: Add {{G}}{{G}}."
        ));
    }
    if let Some((first_body, condition)) = pump.rsplit_once(" as long as ")
        && let Some((second_body, second_condition)) = ability.rsplit_once(" as long as ")
        && condition.eq_ignore_ascii_case(second_condition)
        && let Some(pt) = first_body.strip_prefix("This Equipment creature gets an additional ")
        && let Some(keyword) = second_body.strip_prefix("This Equipment has ")
    {
        return Some(format!(
            "This Equipment gets {pt} and has {keyword} as long as {condition}."
        ));
    }
    if let Some(default) = pump.strip_prefix("Destroy target ")
        && let Some((default_subject, default_threshold)) =
            default.split_once(" if its mana value is ")
        && let Some((_, replacement)) = ability.split_once(" — ")
        && let Some(replacement) = replacement.strip_prefix("Destroy that ")
        && let Some((replacement_subject, replacement_tail)) =
            replacement.split_once(" if it has mana value ")
        && replacement_tail.contains(" instead if ")
        && default_subject.eq_ignore_ascii_case(replacement_subject)
    {
        return Some(format!(
            "Destroy target {default_subject} if it has mana value {default_threshold}.\n{}",
            ability
        ));
    }
    let (pump_body, condition) = pump.rsplit_once(" as long as ")?;
    let subject = pump_body.strip_suffix(" gets +1/+1")?.trim();
    if subject.is_empty()
        || !condition
            .to_ascii_lowercase()
            .starts_with(&format!("{} is ", subject.to_ascii_lowercase()))
    {
        return None;
    }

    let conditional_prefix = format!("As long as {condition}, it has ");
    let subject_prefix = format!("{subject} has ");
    // The attached-grant half names its condition structurally ("As long as
    // the permanent this source is attached to is a blue permanent,
    // enchanted creature has \"...\"", Helm of the Ghastlord).
    let attached_color_prefix = condition
        .get(subject.len() + " is ".len()..)
        .filter(|color| {
            matches!(*color, "white" | "blue" | "black" | "red" | "green")
        })
        .map(|color| {
            format!(
                "As long as the permanent this source is attached to is a {color} permanent, {} has ",
                lowercase_first(subject)
            )
        });
    let ability = ability
        .strip_prefix(&conditional_prefix)
        .or_else(|| ability.strip_prefix(&subject_prefix))
        .or_else(|| {
            attached_color_prefix
                .as_deref()
                .and_then(|prefix| ability.strip_prefix(prefix))
        })?
        .trim();
    let quoted_ability = ability
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'));
    // A quoted triggered or activated ability keeps its quotes; a quoted
    // keyword reads bare.
    let keep_quotes = quoted_ability.is_some_and(|inner| {
        inner.contains(':')
            || inner.starts_with("When")
            || inner.starts_with("At ")
    });
    let ability = quoted_ability.unwrap_or(ability).trim_end_matches('.');
    if ability.is_empty() {
        return None;
    }
    if keep_quotes {
        return Some(format!(
            "As long as {condition}, {} gets +1/+1 and has \"{ability}.\"",
            lowercase_first(subject),
        ));
    }

    Some(format!(
        "As long as {condition}, {} gets +1/+1 and has {}.",
        lowercase_first(subject),
        lowercase_first(ability),
    ))
}

fn compact_threshold_ability_word_lines(lines: Vec<String>) -> Vec<String> {
    let mut compacted = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    while idx < lines.len() {
        if idx + 1 < lines.len()
            && let Some(compact) =
                compact_threshold_pump_and_cant_block(&lines[idx], &lines[idx + 1])
        {
            compacted.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < lines.len()
            && let Some(compact) =
                compact_shared_condition_keyword_and_unblockable(&lines[idx], &lines[idx + 1])
        {
            compacted.push(compact);
            idx += 2;
            continue;
        }
        compacted.push(lines[idx].clone());
        idx += 1;
    }
    compacted
}

/// "As long as Ratonhnhaké꞉ton hasn't dealt damage yet, it has hexproof and
/// can't be blocked.": a keyword grant and an unblockability that share one
/// source condition are one printed static ability.
fn compact_shared_condition_keyword_and_unblockable(
    keyword_line: &str,
    unblockable_line: &str,
) -> Option<String> {
    let keyword_line = keyword_line.trim().trim_end_matches('.');
    let unblockable_line = unblockable_line.trim().trim_end_matches('.');
    let (granted, condition) = keyword_line.rsplit_once(" as long as ")?;
    let (subject, keyword) = granted.split_once(" has ")?;
    let condition_lower = condition.to_lowercase();
    let condition_rest = condition_lower.strip_prefix(&subject.to_lowercase())?;
    let unblockable_condition = unblockable_line
        .strip_prefix("As long as it")?
        .strip_suffix(", this creature can't be blocked")?;
    if condition_rest != unblockable_condition.to_lowercase()
        || keyword.contains(' ')
        || keyword.contains(',')
        || subject.contains(", ")
        || condition_rest.contains(", ")
    {
        return None;
    }
    Some(format!(
        "As long as {condition}, it has {keyword} and can't be blocked."
    ))
}

fn compact_threshold_pump_and_cant_block(pump_line: &str, cant_block_line: &str) -> Option<String> {
    const CONDITION: &str = "there are seven or more cards in your graveyard";
    let pump = pump_line.trim().trim_end_matches('.');
    let cant_block = cant_block_line.trim().trim_end_matches('.');
    let (pump_body, pump_condition) = pump.rsplit_once(" as long as ")?;
    let (cant_block_body, cant_block_condition) = cant_block.rsplit_once(" as long as ")?;
    if !pump_condition.eq_ignore_ascii_case(CONDITION)
        || !cant_block_condition.eq_ignore_ascii_case(CONDITION)
        || cant_block_body != "This creature can't block"
    {
        return None;
    }
    let pump_value = pump_body.strip_prefix("This creature gets ")?;
    Some(format!(
        "Threshold — As long as {pump_condition}, this creature gets {pump_value} and can't block."
    ))
}

fn compact_post_substitution_surface_lines(lines: Vec<String>) -> Vec<String> {
    let mut compacted = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    while idx < lines.len() {
        if idx + 2 < lines.len()
            && let Some(compact) = compact_conditional_source_animation_bundle(
                &lines[idx],
                &lines[idx + 1],
                &lines[idx + 2],
            )
        {
            compacted.push(compact);
            idx += 3;
            continue;
        }

        // "As an additional cost to cast this spell, you may sacrifice any
        // number of creatures. This spell costs {2} less to cast for each
        // creature sacrificed this way." (Torgaar, Rottenmouth Viper): the
        // optional any-number sacrifice and the reduction it funds are one
        // printed paragraph.
        if idx + 1 < lines.len()
            && let Some(objects) = lines[idx]
                .strip_prefix("As an additional cost to cast this spell, sacrifice any number of ")
            && !objects.trim_end_matches('.').contains(". ")
            && lines[idx + 1].starts_with("This spell costs ")
            && lines[idx + 1].contains(" sacrificed this way")
        {
            compacted.push(format!(
                "As an additional cost to cast this spell, you may sacrifice any number of {}. {}",
                objects.trim_end_matches('.'),
                lines[idx + 1]
            ));
            idx += 2;
            continue;
        }

        if idx + 1 < lines.len()
            && lines[idx].starts_with("Reveal the top ")
            && lines[idx].contains(" from among them into your hand")
            && lines[idx + 1].contains(" — ")
            && let Some(second) = lines[idx + 1].strip_suffix(" Put the rest into your graveyard.")
        {
            compacted.push(format!(
                "{}. Put the rest into your graveyard.",
                lines[idx].trim_end_matches('.')
            ));
            compacted.push(second.to_string());
            idx += 2;
            continue;
        }

        compacted.push(lines[idx].clone());
        idx += 1;
    }
    compacted
}

fn compact_conditional_source_animation_bundle(
    animation_line: &str,
    keyword_line: &str,
    ability_line: &str,
) -> Option<String> {
    let (base_pt, subtype, condition) = parse_conditional_source_animation_line(animation_line)?;
    let (subject, keyword, keyword_condition) = parse_conditional_keyword_line(keyword_line)?;
    if !condition.eq_ignore_ascii_case(&keyword_condition) {
        return None;
    }
    let (ability, ability_condition) = parse_conditional_quoted_ability_line(ability_line)?;
    if !condition.eq_ignore_ascii_case(&ability_condition) {
        return None;
    }

    let condition_prefix = if is_celebration_condition(&condition) {
        format!("Celebration — As long as {condition}")
    } else {
        format!("As long as {condition}")
    };
    let terminal = if ability.ends_with(".\"") { "" } else { "." };
    Some(format!(
        "{condition_prefix}, {subject} is {} {} with base power and toughness {base_pt}, {keyword}, and {ability}{terminal}",
        article_for_lowercase_phrase(&subtype),
        capitalize_first(&subtype),
    ))
}

fn parse_conditional_source_animation_line(line: &str) -> Option<(String, String, String)> {
    let trimmed = line.trim().trim_end_matches('.');
    let lower = trimmed.to_ascii_lowercase();
    let prefix = "this creature source is creature in addition to its other types and has base power and toughness ";
    let rest = lower.strip_prefix(prefix)?;
    let (base_pt, tail) = rest.split_once(" and is ")?;
    let (subtype, condition) = tail.rsplit_once(" as long as ")?;
    if !base_pt.contains('/') || subtype.trim().is_empty() || condition.trim().is_empty() {
        return None;
    }
    Some((
        base_pt.trim().to_string(),
        subtype.trim().to_string(),
        condition.trim().to_string(),
    ))
}

fn parse_conditional_keyword_line(line: &str) -> Option<(String, String, String)> {
    let trimmed = line.trim().trim_end_matches('.');
    let (body, condition) = trimmed.rsplit_once(" as long as ")?;
    let (subject, keyword) = body.split_once(" has ")?;
    let keyword = keyword.trim();
    if subject.trim().is_empty() || keyword.is_empty() {
        return None;
    }
    Some((
        subject.trim().to_string(),
        normalize_keyword_predicate_case(keyword),
        condition.trim().to_string(),
    ))
}

fn parse_conditional_quoted_ability_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim().trim_end_matches('.');
    let lower = trimmed.to_ascii_lowercase();
    let marker = lower
        .rfind(" as long as ")
        .or_else(|| lower.rfind(" As long as "))?;
    let ability_body = trimmed[..marker].trim();
    let condition = trimmed[marker + " as long as ".len()..].trim();
    let ability = ability_body
        .strip_prefix("This source has ")
        .or_else(|| ability_body.strip_prefix("this source has "))
        .or_else(|| ability_body.split_once(" has ").map(|(_, ability)| ability))?
        .trim();
    if !ability.starts_with('"') || condition.is_empty() {
        return None;
    }
    Some((ability.to_string(), condition.to_string()))
}

fn is_celebration_condition(condition: &str) -> bool {
    let lower = condition.to_ascii_lowercase();
    lower.contains(
        "two or more nonland permanents entered the battlefield under your control this turn",
    )
}

fn article_for_lowercase_phrase(phrase: &str) -> &'static str {
    match phrase.chars().next().map(|ch| ch.to_ascii_lowercase()) {
        Some('a' | 'e' | 'i' | 'o' | 'u') => "an",
        _ => "a",
    }
}

fn compact_station_threshold_lines(lines: Vec<String>) -> Vec<String> {
    if !lines.iter().any(|line| line.trim() == "Station") {
        return lines;
    }

    let mut compacted = Vec::with_capacity(lines.len());
    let mut pending_keyword_threshold: Option<i32> = None;
    let mut pending_keywords: Vec<String> = Vec::new();

    for line in lines {
        let Some((threshold, body)) = split_station_threshold_condition(&line) else {
            flush_station_keyword_row(
                &mut compacted,
                &mut pending_keyword_threshold,
                &mut pending_keywords,
            );
            compacted.push(line);
            continue;
        };

        if is_station_implicit_creature_support(&body) {
            continue;
        }

        if let Some(keyword) = station_threshold_keyword_body(&body) {
            if pending_keyword_threshold == Some(threshold) {
                pending_keywords.push(keyword);
            } else {
                flush_station_keyword_row(
                    &mut compacted,
                    &mut pending_keyword_threshold,
                    &mut pending_keywords,
                );
                pending_keyword_threshold = Some(threshold);
                pending_keywords.push(keyword);
            }
            continue;
        }

        flush_station_keyword_row(
            &mut compacted,
            &mut pending_keyword_threshold,
            &mut pending_keywords,
        );
        compacted.push(format!("{threshold}+ | {}", station_threshold_body(&body)));
    }

    flush_station_keyword_row(
        &mut compacted,
        &mut pending_keyword_threshold,
        &mut pending_keywords,
    );
    // A striation covers every line printed under it until the next one
    // (CR 702.184), so later abilities of the same threshold continue the
    // block without repeating the "N+ |" marker.
    let mut current_threshold: Option<i32> = None;
    compacted
        .into_iter()
        .map(|line| {
            let Some((threshold_text, body)) = line.split_once("+ | ") else {
                return line;
            };
            let Ok(threshold) = threshold_text.trim().parse::<i32>() else {
                return line;
            };
            if current_threshold == Some(threshold) {
                return capitalize_first(body);
            }
            current_threshold = Some(threshold);
            line
        })
        .collect()
}

fn flush_station_keyword_row(
    out: &mut Vec<String>,
    pending_threshold: &mut Option<i32>,
    pending_keywords: &mut Vec<String>,
) {
    let Some(threshold) = pending_threshold.take() else {
        return;
    };
    if !pending_keywords.is_empty() {
        out.push(format!("{threshold}+ | {}", pending_keywords.join(", ")));
        pending_keywords.clear();
    }
}

fn split_station_threshold_condition(line: &str) -> Option<(i32, String)> {
    const MARKER: &str = " as long as CountersOnSource is greater than or equal to ";
    let trimmed = line.trim().trim_end_matches('.');
    if let Some((body, threshold_text)) = trimmed.rsplit_once(MARKER) {
        let threshold = parse_station_threshold_value(threshold_text.trim())?;
        return Some((threshold, body.trim().to_string()));
    }

    if let Some((body, condition)) = trimmed.rsplit_once(" as long as ")
        && let Some(threshold) = parse_station_charge_counter_condition(condition)
    {
        return Some((threshold, body.trim().to_string()));
    }

    const PREFIX: &str = "As long as CountersOnSource is greater than or equal to ";
    if let Some(rest) = trimmed.strip_prefix(PREFIX) {
        let (threshold_text, body) = rest.split_once(", ")?;
        let threshold = parse_station_threshold_value(threshold_text.trim())?;
        return Some((threshold, body.trim().to_string()));
    }

    let rest = trimmed.strip_prefix("As long as ")?;
    let (condition, body) = rest.split_once(", ")?;
    let threshold = parse_station_charge_counter_condition(condition)?;
    Some((threshold, body.trim().to_string()))
}

fn parse_station_charge_counter_condition(condition: &str) -> Option<i32> {
    let lower = condition.trim().to_ascii_lowercase();
    let threshold = [
        "the number of charge counters on this source is ",
        "the number of charge counters on this permanent is ",
        "the number of charge counters on this artifact is ",
        "the number of charge counters on this creature is ",
        "the number of charge counters on this spacecraft is ",
    ]
    .into_iter()
    .find_map(|prefix| lower.strip_prefix(prefix))?;
    let threshold = threshold
        .strip_suffix(" or greater")
        .or_else(|| threshold.strip_prefix("greater than or equal to "))?;
    parse_station_threshold_value(threshold.trim())
}

fn parse_station_threshold_value(text: &str) -> Option<i32> {
    if let Ok(value) = text.parse::<i32>() {
        return Some(value);
    }
    let words = text.split_whitespace().collect::<Vec<_>>();
    let (value, used) = ironsmith_core::parse_cardinal_words(&words)?;
    (used == words.len()).then_some(value as i32)
}

fn is_station_implicit_creature_support(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    let trimmed = lower.trim().trim_end_matches('.');
    // The conditional self type-add renders as "this Spacecraft is a
    // creature" (no "in addition to" tail); it is still the implicit
    // creature half of the threshold (CR 702.184b), never printed.
    let bare_self_creature = trimmed.starts_with("this ")
        && !trimmed.contains(',')
        && (trimmed.ends_with(" is a creature") || trimmed.ends_with(" is an artifact creature"));
    bare_self_creature
        || (lower.contains(" is creature in addition to ")
            || lower.contains(" is a creature in addition to "))
        || lower.contains(" has base power and toughness ")
}

fn station_threshold_keyword_body(body: &str) -> Option<String> {
    let body = station_threshold_body(body);
    let lower = body.to_ascii_lowercase();
    let keyword = [
        "this artifact creature has ",
        "this artifact has ",
        "this spacecraft has ",
        "this source has ",
        "this creature has ",
    ]
    .into_iter()
    .find_map(|prefix| lower.starts_with(prefix).then(|| &body[prefix.len()..]))
    .unwrap_or(body.as_str())
    .trim();
    let normalized = normalize_keyword_predicate_case(keyword);
    let keyword_parts = normalized
        .split(" and ")
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if keyword_parts.len() > 1 && keyword_parts.iter().all(|part| is_keyword_phrase(part)) {
        return Some(capitalize_first(&keyword_parts.join(", ")));
    }
    if !is_keyword_phrase(&normalized) {
        return None;
    }
    Some(capitalize_first(&normalized))
}

fn station_threshold_body(body: &str) -> String {
    let body = body.trim().trim_end_matches('.');
    if let Some(rest) = body.strip_prefix("This artifact source ") {
        return format!("This artifact {rest}");
    }
    body.to_string()
}

/// Once a line has spelled out "this is the first time this ability has
/// resolved this turn", later branches of the same count ladder use the
/// elliptical Oracle form "If it's the second time, ...".
fn elide_repeated_resolution_count_conditions(line: String) -> String {
    const TAIL: &str = " time this ability has resolved this turn";
    const HEAD: &str = "If this is the ";
    let Some(first) = line.find(TAIL) else {
        return line;
    };
    let split = first + TAIL.len();
    let (kept, mut rest) = line.split_at(split);
    let mut out = kept.to_string();
    while let Some(start) = rest.find(HEAD) {
        let after_head = &rest[start + HEAD.len()..];
        let Some(ordinal) = after_head.split(' ').next() else {
            break;
        };
        let Some(after_ordinal) = after_head[ordinal.len()..].strip_prefix(TAIL) else {
            out.push_str(&rest[..start + HEAD.len()]);
            rest = after_head;
            continue;
        };
        out.push_str(&rest[..start]);
        out.push_str(&format!("If it's the {ordinal} time"));
        rest = after_ordinal;
    }
    out.push_str(rest);
    out
}

/// Recruit is lowered by expanding the keyword action into its reminder-text
/// instructions; the rendered expansion reads back as the keyword action.
fn restore_recruit_keyword_action(line: String) -> String {
    const EXPANSION: &str = "raw a card, then discard a card. If you discard a nonland card this way, create a 1/1 white Human Soldier creature token";
    let mut out = line;
    for (head, keyword) in [("d", "recruit"), ("D", "Recruit")] {
        out = out.replace(&format!("{head}{EXPANSION}"), keyword);
    }
    out
}

/// A voting ability word heads its whole ability ("Will of the council —
/// Choose target instant or sorcery spell. Starting with you, ..."), even
/// when an earlier sentence of the same ability declares the target.
fn lead_with_embedded_vote_ability_word(line: String) -> String {
    for label in ["Will of the council — ", "Council's dilemma — "] {
        if line.starts_with(label) {
            return line;
        }
        let embedded = format!(". {label}");
        if let Some(position) = line.find(&embedded)
            && !line[..position].contains(" — ")
            && !line[..position].contains(':')
        {
            let head = &line[..position];
            let tail = &line[position + embedded.len()..];
            return format!("{label}{head}. {tail}");
        }
    }
    line
}

/// "Whenever this creature deals combat damage to a player, copy the next
/// instant or sorcery spell you cast this turn when you cast it" (Tzaangor
/// Shaman): a triggered ability whose whole effect is the next-spell copy
/// watcher reads with the "copy the next ..." surface.
fn copy_next_spell_triggered_watcher_surface(line: String) -> String {
    let line = copy_next_loyalty_ability_triggered_watcher_surface(line);
    const WATCH: &str = ", when you next cast ";
    const TAIL: &str = " this turn, copy that spell. You may choose new targets for the copy";
    let Some(position) = line.find(WATCH) else {
        return line;
    };
    let head = &line[..position];
    let rest = &line[position + WATCH.len()..];
    let trigger = head.rsplit(" — ").next().unwrap_or(head);
    if !trigger.starts_with("Whenever ")
        || trigger.contains(". ")
        || trigger.contains(", ")
        || !trigger.contains(" deals combat damage to ")
    {
        return line;
    }
    let Some(spell) = rest
        .strip_suffix('.')
        .unwrap_or(rest)
        .strip_suffix(TAIL)
    else {
        return line;
    };
    let spell = spell
        .strip_prefix("an ")
        .or_else(|| spell.strip_prefix("a "))
        .unwrap_or(spell);
    if !spell.ends_with(" spell") || spell.contains(',') {
        return line;
    }
    let terminal = if line.ends_with('.') { "." } else { "" };
    format!(
        "{head}, copy the next {spell} you cast this turn when you cast it. You may choose new targets for the copy{terminal}"
    )
}

/// "Whenever this creature deals combat damage to a player or planeswalker,
/// copy the next loyalty ability you activate this turn when you activate
/// it" (Jaya's Phoenix): the ability-copy counterpart of the next-spell copy
/// watcher surface.
fn copy_next_loyalty_ability_triggered_watcher_surface(line: String) -> String {
    const WATCH: &str = ", when you next activate a loyalty ability this turn, copy that ability. You may choose new targets for the copy";
    let Some(position) = line.find(WATCH) else {
        return line;
    };
    let head = &line[..position];
    let rest = &line[position + WATCH.len()..];
    let trigger = head.rsplit(" — ").next().unwrap_or(head);
    if !trigger.starts_with("Whenever ")
        || trigger.contains(". ")
        || trigger.contains(", ")
        || !trigger.contains(" deals combat damage to ")
        || !(rest.is_empty() || rest == ".")
    {
        return line;
    }
    format!(
        "{head}, copy the next loyalty ability you activate this turn when you activate it. You may choose new targets for the copy{rest}"
    )
}

fn normalize_scored_compiled_line(line: String) -> String {
    // Effect-list renderers intentionally lowercase clauses when composing
    // them into a larger sentence. At the card-line boundary, restore normal
    // sentence capitalization before applying the surface normalizers.
    let line = normalize_duplicate_optional_subject(&capitalize_first(
        &capitalize_sentence_boundaries(&line),
    ));
    // The Wish family's search-outside-the-game program only reaches its
    // final three-sentence surface after the earlier merge passes; fold it
    // to the authored reveal-and-put sentence here.
    let line = normalize_common::normalize_search_outside_game_reveal_surface(&line);
    let line = lead_with_embedded_vote_ability_word(line);
    let line = copy_next_spell_triggered_watcher_surface(line);
    let line = elide_repeated_resolution_count_conditions(line);
    let line = restore_recruit_keyword_action(line);
    // A trailing "if" on a targeting instruction ("Destroy target artifact if
    // its mana value is 2 or less") checks the pending target before the
    // action happens, so it never reads as the past-tense last-known check
    // used by a following sentence.
    let line = line
        .split(". ")
        .map(|sentence| {
            if sentence.contains("target ") && sentence.contains(" if its mana value was ") {
                sentence.replace(" if its mana value was ", " if its mana value is ")
            } else {
                sentence.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(". ");
    // A bare repeat instruction after an optional step loops on taking the
    // option; oracle spells the "If you do," gate.
    let line = if let Some(head) = line
        .strip_suffix(". Repeat this process.")
        .or_else(|| line.strip_suffix(". Repeat this process"))
        && head
            .rsplit(". ")
            .next()
            .is_some_and(|prev| prev.starts_with("You may ") || prev.starts_with("you may "))
    {
        format!("{head}. If you do, repeat this process.")
    } else {
        line
    };
    // This is the first point where sentence capitalization and every
    // cross-segment merge are both complete. The semantic normalizer is
    // idempotent and already guards each rewrite structurally, so running it
    // only for an allow-list of historical substrings made otherwise-valid
    // repairs silently unreachable as new typed renderers evolved.
    let line = normalize_common_semantic_phrasing(&line);
    let lower = line.to_ascii_lowercase();
    if lower.contains("whenever a land you control enters")
        && lower.contains("if it's a mountain, this creature deals")
    {
        return line.replace(
            "If it's a Mountain, this creature deals",
            "If that land is a Mountain, this creature deals",
        );
    }
    if let Some((source, _)) = line.split_once(
        " deals 2 damage to each opponent and each creature they control. If this spell was cast from exile, ",
    ) && let Some(rest) = line.strip_prefix(&format!(
        "{source} deals 2 damage to each opponent and each creature they control. If this spell was cast from exile, {source} deals 5 damage instead"
    )) {
        return format!(
            "{source} deals 2 damage to each opponent and each creature they control. If this spell was cast from exile, it deals 5 damage to each opponent and each creature they control instead{rest}"
        );
    }
    if lower.trim_end_matches('.')
        == "sacrifice this enchantment: creatures your opponents control get -1/-1 and gain attacks each combat if able until end of turn"
    {
        return "Sacrifice this enchantment: Creatures your opponents control get -1/-1 until end of turn. Those creatures attack this turn if able.".to_string();
    }
    if lower.trim_end_matches('.')
        == "exile all cards from their hand. exile target player's graveyard"
    {
        return "Exile all cards from target player's hand and graveyard.".to_string();
    }
    if lower.trim_end_matches('.')
        == "this creature enters with x +1/+1 counters on it, where x is the number of another creature or artifact you control"
    {
        return "This creature enters with a +1/+1 counter on it for each other creature and/or artifact you control".to_string();
    }
    if lower.trim_end_matches('.')
        == "choose a creature at random on the battlefield, gain control of it until end of turn, untap it, it gains haste until end of turn, then destroy all other creatures"
    {
        return "Choose a creature at random. You gain control of that creature until end of turn. Untap it. It gains haste until end of turn. Then destroy all other creatures.".to_string();
    }
    if lower.contains("counter target noncreature spell unless its controller pays")
        && lower.contains("instead counter target noncreature spell")
    {
        return line.replace(
            "instead counter target noncreature spell",
            "instead counter that spell",
        );
    }
    normalize_mass_opponent_controller_surface(normalize_triggering_object_anaphor(line))
}

/// A back-reference "that spell or ability" can only denote a spell when
/// every event mentioned in the line is a cast (and only an ability when the
/// line only mentions activation).  The copy-effect model is generic over
/// stack objects, but the line itself pins the referent — resolve the
/// anaphor from the line alone, with no card-specific knowledge.
fn normalize_triggering_object_anaphor(line: String) -> String {
    if !line.contains("that spell or ability") {
        return line;
    }
    let lower = line.to_ascii_lowercase();
    // The line itself declares a spell-or-ability antecedent ("choose
    // target spell or ability ... Copy that spell or ability", Radiant
    // Performer): the anaphor keeps that union.
    if lower
        .replace("that spell or ability", "")
        .contains("spell or ability")
    {
        return line;
    }
    let mentions_cast = lower.contains("cast");
    let mentions_activation = lower.contains("activate") || lower.contains("activated");
    if mentions_cast && !mentions_activation {
        return line.replace("that spell or ability", "that spell");
    }
    if mentions_activation && !mentions_cast {
        return line.replace("that spell or ability", "that ability");
    }
    line
}

/// Oracle text uses "an opponent controls" for single-object references but
/// "your opponents control" in mass contexts ("each creature your opponents
/// control", "all artifacts your opponents control").  The filter description
/// only knows the singular surface; rewrite it when the clause quantifies over
/// every matching object.
fn normalize_mass_opponent_controller_surface(line: String) -> String {
    const PHRASE: &str = "an opponent controls";
    if !line.contains(PHRASE) {
        return line;
    }
    let mut rewritten = String::with_capacity(line.len());
    let mut rest = line.as_str();
    while let Some(idx) = rest.find(PHRASE) {
        let (before, after) = rest.split_at(idx);
        let clause_start = before
            .rfind(['.', ',', ':', ';'])
            .map(|punct| punct + 1)
            .unwrap_or(0);
        let clause = before[clause_start..].to_ascii_lowercase();
        // A plural subject noun right before the phrase ("creatures an
        // opponent controls") is also a mass context — but counted groups
        // ("one or more creatures an opponent controls") keep the single
        // opponent because their controller's identity matters.
        let plural_subject = clause
            .split_whitespace()
            .last()
            .is_some_and(|word| word.ends_with('s') && word != "less")
            && !clause.contains("or more ")
            && !clause.contains("or fewer ")
            // "the greatest number of artifacts an opponent controls" picks a
            // single opponent's count; keep the singular surface.
            && !clause.contains("number of ");
        let mass_context = clause.contains("each ") || clause.contains("all ") || plural_subject;
        rewritten.push_str(before);
        rewritten.push_str(if mass_context {
            "your opponents control"
        } else {
            PHRASE
        });
        rest = &after[PHRASE.len()..];
    }
    rewritten.push_str(rest);
    rewritten
}

fn substitute_kicked_draw_source_reference(line: &str, def: &CardDefinition) -> String {
    let has_repeatable_kicker = def.optional_costs.iter().any(|cost| {
        cost.repeatable
            && matches!(
                cost.kind,
                crate::cost::OptionalCostKind::Kicker | crate::cost::OptionalCostKind::Multikicker
            )
    });
    if !has_repeatable_kicker
        || def.card.name.contains(" // ")
        || !line
            .to_ascii_lowercase()
            .contains("draw a card for each time this spell was kicked")
    {
        return line.to_string();
    }

    let source_name = def
        .card
        .name
        .split(',')
        .next()
        .unwrap_or(&def.card.name)
        .trim();
    if source_name.is_empty() {
        return line.to_string();
    }

    line.replace(
        "this spell was kicked",
        &format!("{source_name} was kicked"),
    )
    .replace(
        "This spell was kicked",
        &format!("{source_name} was kicked"),
    )
}

fn normalize_unprocessed_compiled_line(line: String) -> String {
    let line = normalize_duplicate_optional_subject(&capitalize_first(
        &capitalize_sentence_boundaries(&line),
    ));
    let lower = line.to_ascii_lowercase();
    if lower.contains("whenever a land you control enters")
        && lower.contains("if it's a mountain, this creature deals")
    {
        return line.replace(
            "If it's a Mountain, this creature deals",
            "If that land is a Mountain, this creature deals",
        );
    }
    if let Some((source, _)) = line.split_once(" deals 2 damage to each opponent and each creature they control. If this spell was cast from exile, ")
        && let Some(rest) = line.strip_prefix(&format!("{source} deals 2 damage to each opponent and each creature they control. If this spell was cast from exile, {source} deals 5 damage instead"))
    {
        return format!(
            "{source} deals 2 damage to each opponent and each creature they control. If this spell was cast from exile, it deals 5 damage to each opponent and each creature they control instead{rest}"
        );
    }
    if lower.contains("unless an opponent lost life this turn, sacrifice it") {
        return line;
    }
    if lower.contains("if you dealt combat damage to a player this turn with a assassin or commander, you may pay {2}{r} rather than pay this spell's mana cost")
    {
        return line.replace(
            "If you dealt combat damage to a player this turn with a assassin or commander, you may pay {2}{R} rather than pay this spell's mana cost",
            "Freerunning {2}{R}",
        );
    }
    if lower.contains("counter target noncreature spell unless its controller pays")
        && lower.contains("instead counter that spell")
    {
        return line;
    }
    if lower.contains("this equipment gets +x/+0 until end of turn")
        && lower.contains("where x is the number of times this ability has resolved this turn")
    {
        return line;
    }
    if lower.starts_with("each creature you control gets ")
        && lower.contains(" until end of turn. then if it is not your turn, untap that creature.")
    {
        return line
            .replacen(
                "Each creature you control gets ",
                "Creatures you control get ",
                1,
            )
            .replace(
                " until end of turn. Then if it is not your turn, untap that creature.",
                " until end of turn. If it's not your turn, untap those creatures.",
            );
    }
    line
}

fn normalize_duplicate_optional_subject(line: &str) -> String {
    replace_ascii_case_insensitive_once(
        line.to_string(),
        "you may you attach ",
        "You may attach ",
        "you may attach ",
    )
}

fn remove_redundant_period_after_terminal_quote(line: &str) -> String {
    let mut normalized = String::with_capacity(line.len());
    let mut chars = line.char_indices().peekable();
    let mut in_quote = false;

    while let Some((_idx, ch)) = chars.next() {
        if ch != '"' {
            normalized.push(ch);
            continue;
        }

        if !in_quote {
            in_quote = true;
            normalized.push(ch);
            continue;
        }

        let quote_has_terminal_punctuation = normalized
            .chars()
            .last()
            .is_some_and(|previous| matches!(previous, '.' | '!' | '?'));
        in_quote = false;
        normalized.push(ch);

        let Some((period_idx, '.')) = chars.peek().copied() else {
            continue;
        };
        let after_period = &line[period_idx + 1..];
        let follows_with_same_line_sentence = after_period.starts_with(' ')
            && after_period
                .trim_start_matches(' ')
                .chars()
                .next()
                .is_some_and(|next| next.is_ascii_uppercase());
        if quote_has_terminal_punctuation && follows_with_same_line_sentence {
            chars.next();
        }
    }

    normalized
}

/// "Each player loses half their life, then discards half the cards in their
/// hand. Round down each time.": a sentence whose every fraction rounds the
/// same way states the rounding once, after the sentence.
fn fold_repeated_rounding_into_trailing_sentence(line: String) -> String {
    let mut out = Vec::new();
    let mut changed = false;
    for sentence in line.split_inclusive(". ") {
        let (body, separator) = match sentence.strip_suffix(". ") {
            Some(body) => (body, ". "),
            None => (sentence, ""),
        };
        let (core, terminal) = match body.strip_suffix('.') {
            Some(core) => (core, "."),
            None => (body, ""),
        };
        let mut folded = None;
        for direction in ["down", "up"] {
            let marker = format!(", rounded {direction}");
            let other = if direction == "down" { ", rounded up" } else { ", rounded down" };
            if core.matches(&marker).count() >= 2 && !core.contains(other) {
                folded = Some(format!(
                    "{}. Round {direction} each time{}{separator}",
                    core.replace(&marker, ""),
                    if terminal.is_empty() { "" } else { "." }
                ));
            }
        }
        match folded {
            Some(text) => {
                changed = true;
                out.push(text);
            }
            None => out.push(sentence.to_string()),
        }
    }
    if changed { out.concat() } else { line }
}

/// Adamant (CR 207.2c ability word) is the only printed spelling of a
/// leading "at least three <color> mana was spent" condition.
fn label_adamant_condition_line(line: String) -> String {
    let Some(rest) = line.strip_prefix("If at least three ") else {
        return line;
    };
    let colored = ["white", "blue", "black", "red", "green"]
        .iter()
        .any(|color| rest.starts_with(&format!("{color} mana was spent to cast this spell")));
    if colored { format!("Adamant — {line}") } else { line }
}

/// "... if this is the first time this ability has resolved this turn. If
/// it's the second time, ...": after the first resolution-count condition,
/// later ones in the same ability abbreviate to "it's the <ordinal> time".
fn abbreviate_repeated_resolution_count_conditions(line: String) -> String {
    const HEAD: &str = "If this is the ";
    const TAIL: &str = " time this ability has resolved this turn";
    let Some(first) = line.find(" time this ability has resolved this turn") else {
        return line;
    };
    let split = first + TAIL.len();
    let (kept, rest) = line.split_at(split);
    let mut out = kept.to_string();
    let mut remaining = rest;
    while let Some(start) = remaining.find(HEAD) {
        let after_head = &remaining[start + HEAD.len()..];
        let Some(ordinal_end) = after_head.find(TAIL) else {
            break;
        };
        let ordinal = &after_head[..ordinal_end];
        if ordinal.is_empty() || ordinal.contains(' ') {
            out.push_str(&remaining[..start + HEAD.len()]);
            remaining = after_head;
            continue;
        }
        out.push_str(&remaining[..start]);
        out.push_str("If it's the ");
        out.push_str(ordinal);
        out.push_str(" time");
        remaining = &after_head[ordinal_end + TAIL.len()..];
    }
    out.push_str(remaining);
    out
}

fn fold_choose_any_number_then_each_it(line: String) -> String {
    const EACH: &str = ". For each of those objects, ";
    let Some(each_at) = line.find(EACH) else {
        return line;
    };
    let before = &line[..each_at];
    let (prefix, chosen_sentence) = match before.rfind(". ") {
        Some(at) => (&before[..at + 2], &before[at + 2..]),
        None => ("", before),
    };
    let Some(selection) = chosen_sentence
        .strip_prefix("You choose any number of ")
        .or_else(|| chosen_sentence.strip_prefix("Choose any number of "))
    else {
        return line;
    };
    if selection.contains(", ") || selection.contains(" target ") || selection.starts_with("target ")
    {
        return line;
    }
    let after = &line[each_at + EACH.len()..];
    let sentence_end = after.find(". ").unwrap_or(after.len());
    let body = after[..sentence_end].trim_end_matches('.');
    let rest = if sentence_end == after.len() && after.ends_with('.') {
        "."
    } else {
        &after[sentence_end..]
    };
    let Some((verb, tail)) = body.split_once(" it") else {
        return line;
    };
    if verb.is_empty()
        || verb.contains(' ')
        || !verb.chars().all(|ch| ch.is_ascii_lowercase())
        || !(tail.is_empty() || tail.starts_with(' '))
        || tail.contains(" it ")
        || tail.ends_with(" it")
        || tail.contains(" its ")
        || tail.contains(" that ")
    {
        return line;
    }
    let mut selection = selection.to_string();
    if prefix.contains(" mill") {
        selection = selection.replace(" milled this way in a graveyard", " from among them");
    }
    format!(
        "{prefix}{} any number of {selection}{tail}{rest}",
        capitalize_first(verb)
    )
}

/// "If that creature has haste and this creature doesn't, put a haste counter
/// on it. Do the same for flying, first strike, ... and vigilance."
/// (Super-Adaptoid): a run of three or more adjacent sentences that differ
/// only in the keyword they name is one instruction repeated per keyword.
fn fold_keyword_variant_sentence_run(line: String) -> String {
    const KEYWORDS: &[&str] = &[
        "double strike",
        "first strike",
        "deathtouch",
        "defender",
        "flying",
        "haste",
        "hexproof",
        "indestructible",
        "lifelink",
        "menace",
        "reach",
        "shroud",
        "trample",
        "vigilance",
    ];
    fn keyword_of(lower: &str) -> Option<&'static str> {
        KEYWORDS.iter().copied().find(|keyword| {
            lower.match_indices(keyword).any(|(at, _)| {
                let before_ok = lower[..at]
                    .chars()
                    .next_back()
                    .is_none_or(|ch| !ch.is_alphanumeric());
                let after_ok = lower[at + keyword.len()..]
                    .chars()
                    .next()
                    .is_none_or(|ch| !ch.is_alphanumeric());
                before_ok && after_ok
            })
        })
    }
    fn template(sentence: &str, keyword: &str) -> String {
        let lower = sentence.trim().trim_end_matches('.').to_ascii_lowercase();
        let lower = lower.strip_prefix("then ").unwrap_or(&lower).to_string();
        lower
            .replace(keyword, "\u{1}")
            .replace("an \u{1}", "a \u{1}")
    }
    let sentences = line.split_inclusive(". ").collect::<Vec<_>>();
    if sentences.len() < 3 {
        return line;
    }
    let keyed = sentences
        .iter()
        .map(|sentence| {
            let lower = sentence.to_ascii_lowercase();
            keyword_of(&lower).map(|keyword| (keyword, template(sentence, keyword)))
        })
        .collect::<Vec<_>>();
    let mut out = String::new();
    let mut changed = false;
    let mut idx = 0usize;
    while idx < sentences.len() {
        let Some((first_keyword, first_template)) = &keyed[idx] else {
            out.push_str(sentences[idx]);
            idx += 1;
            continue;
        };
        let mut end = idx + 1;
        let mut keywords = vec![*first_keyword];
        while end < sentences.len()
            && let Some((keyword, template)) = &keyed[end]
            && template == first_template
            && !keywords.contains(keyword)
        {
            keywords.push(*keyword);
            end += 1;
        }
        if keywords.len() < 3 || !first_template.contains('\u{1}') {
            out.push_str(sentences[idx]);
            idx += 1;
            continue;
        }
        let first = sentences[idx].trim_end();
        let first = first.strip_suffix('.').unwrap_or(first);
        let rest = &keywords[1..];
        let list = if rest.len() == 1 {
            rest[0].to_string()
        } else {
            format!(
                "{}, and {}",
                rest[..rest.len() - 1].join(", "),
                rest[rest.len() - 1]
            )
        };
        let last_ends_line = !sentences[end - 1].ends_with(". ");
        out.push_str(&format!(
            "{first}. Do the same for {list}.{}",
            if last_ends_line { "" } else { " " }
        ));
        changed = true;
        idx = end;
    }
    if changed { out } else { line }
}

fn finalize_ast_surface_line(line: String) -> String {
    let mut line = abbreviate_repeated_resolution_count_conditions(label_adamant_condition_line(
        fold_repeated_rounding_into_trailing_sentence(fold_keyword_variant_sentence_run(line)),
    ));
    // A merge pass can append a rider after a line that already ends with a
    // sentence-final period, doubling it ("can't be regenerated.. You lose").
    if !line.contains("...") {
        while line.contains("..") {
            line = line.replace("..", ".");
        }
    }
    // The same doubling after a closing quote that already ends with the
    // quoted rule's period: `has "At end of combat, exile this token.".`
    while line.contains(".\".") {
        line = line.replace(".\".", ".\"");
    }
    // An intervening-if trigger whose effect repeats the same condition as a
    // conditional wrapper prints it twice ("if you control three or more
    // Wolves or Werewolves, if you control three or more Wolves or
    // Werewolves, it becomes night", Tovolar). One condition is the rule.
    let mut search_from = 0;
    while let Some(found) = line[search_from..].find(", if ") {
        let clause_start = search_from + found + 2;
        let Some(clause_len) = line[clause_start..].find(", ") else {
            break;
        };
        let clause = line[clause_start..clause_start + clause_len].to_string();
        let after = clause_start + clause_len + 2;
        if line[after..].starts_with(&format!("{clause}, ")) {
            line.replace_range(after..after + clause.len() + 2, "");
        } else {
            search_from = clause_start;
        }
    }
    // "it gets +2/+2 until end of turn and it deals 2 damage to each
    // opponent" (The Howling Abomination): one subject governs both
    // predicates, so the second does not restate the pronoun.
    if let Some(at) = line.find(" until end of turn and it deals ") {
        let before = &line[..at];
        let sentence = before.rsplit_once(". ").map_or(before, |(_, tail)| tail);
        let clause = sentence.rsplit_once(", ").map_or(sentence, |(_, tail)| tail);
        if clause.starts_with("it gets ") && !clause.contains(" and ") {
            line = format!(
                "{before} until end of turn and deals {}",
                &line[at + " until end of turn and it deals ".len()..]
            );
        }
    }
    // A subject renderer that adds its own article to a filter whose
    // description already carries the shared union article ("a Wolf or
    // Werewolf you control") doubles it ("Whenever an a Wolf ...").
    for doubled in [" an a ", " a a "] {
        while let Some(at) = line.find(doubled).filter(|at| {
            line[at + doubled.len()..]
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase())
        }) {
            line.replace_range(at..at + doubled.len(), " a ");
        }
    }
    // "up to X target creatures, where X is that many" is oracle's inline
    // "up to that many target creatures". Only rewrite when the where-clause
    // is the sole X consumer left in the sentence.
    if let Some(where_idx) = line.find(", where X is that many")
        && let Some(x_idx) = line[..where_idx].rfind("up to X ")
    {
        let mut fixed = String::with_capacity(line.len());
        fixed.push_str(&line[..x_idx]);
        fixed.push_str("up to that many ");
        fixed.push_str(&line[x_idx + "up to X ".len()..where_idx]);
        fixed.push_str(&line[where_idx + ", where X is that many".len()..]);
        let stray_x = fixed
            .split(|ch: char| !ch.is_ascii_alphanumeric())
            .any(|word| word == "X");
        if !stray_x {
            line = fixed;
        }
    }
    // A contextual "exile it" that immediately receives counters is oracle's
    // fused "exile it with a ... counter on it" sentence.
    if let Some(exile_idx) = line.find("xile it. Put ")
        && let Some(counter_rel) = line[exile_idx..].find(" on the exiled card")
    {
        let put_start = exile_idx + "xile it. Put ".len();
        let counter_end = exile_idx + counter_rel;
        let descriptor = &line[put_start..counter_end];
        if descriptor.ends_with(" counter") || descriptor.ends_with(" counters") {
            let mut fused = String::with_capacity(line.len());
            fused.push_str(&line[..exile_idx]);
            fused.push_str("xile it with ");
            fused.push_str(descriptor);
            fused.push_str(" on it");
            fused.push_str(&line[counter_end + " on the exiled card".len()..]);
            line = fused;
        }
    }
    // "You choose any number of Human Werewolves you control. For each of
    // those objects, transform it." is oracle's "Transform any number of
    // Human Werewolves you control" (Tovolar); likewise a put of milled
    // cards reads "Put any number of land cards from among them onto the
    // battlefield tapped" (Rampant Frogantua).
    line = fold_choose_any_number_then_each_it(line);
    // "Choose a nonland card name and a number greater than 0" (Mindblaze):
    // the two choices form one instruction.
    if line.contains(" card name, then choose a number ") {
        line = line.replacen(" card name, then choose a number ", " card name and a number ", 1);
    }
    // After "Target player reveals their hand/library.", later mentions of
    // the single target player are back-references ("that player").
    for reveal in [
        "Target player reveals their hand. ",
        "Target player reveals their library. ",
    ] {
        if let Some(at) = line.find(reveal) {
            let split = at + reveal.len();
            let head = line[..split].to_string();
            let mut tail = line[split..].to_string();
            if !head.contains("target ") && !tail.contains("another target") {
                tail = tail
                    .replace(" to target player", " to that player")
                    .replace("search target player's ", "search that player's ")
                    .replace("Search target player's ", "Search that player's ");
                line = format!("{head}{tail}");
            }
        }
    }
    // "You may choose a card you own outside the game. Put it on top of its
    // owner's library." is oracle's "You may put a card you own from outside
    // the game on top of your library" (The Raven's Warning).
    if let Some(choose_at) = line.find("choose a card you own outside the game. Put it ") {
        let head = &line[..choose_at];
        let tail = &line[choose_at + "choose a card you own outside the game. Put it ".len()..];
        if (head.is_empty() || head.ends_with("may ") || head.ends_with("— "))
            && !tail.contains(" it ")
        {
            let verb = if head.is_empty() { "Put" } else { "put" };
            let tail = tail.replacen("its owner's library", "your library", 1);
            line = format!("{head}{verb} a card you own from outside the game {tail}");
        }
    }
    // "look at that player's hand and you draw a card" (The Raven's
    // Warning): the imperative look keeps the implicit "you" subject.
    if let Some(at) = line.find(" and you draw ") {
        let before = &line[..at];
        let clause_start = before
            .rfind(". ")
            .map(|idx| idx + 2)
            .into_iter()
            .chain(before.rfind(", ").map(|idx| idx + 2))
            .max()
            .unwrap_or(0);
        if before[clause_start..].starts_with("look at ") {
            line = format!("{before} and draw {}", &line[at + " and you draw ".len()..]);
        }
    }
    let lower = line.to_ascii_lowercase();
    if let Some(normalized) = normalize_gain_control_untap_pump_haste_surface(&line) {
        return normalized;
    }
    if line.contains("If you dealt combat damage to a player this turn with a assassin or commander, you may pay {2}{R} rather than pay this spell's mana cost.")
    {
        line = line.replace(
            "If you dealt combat damage to a player this turn with a assassin or commander, you may pay {2}{R} rather than pay this spell's mana cost.",
            "Freerunning {2}{R}.",
        );
    }
    if line.contains(
        "The next face-down creature cast by you spell you cast this turn costs {3} less to cast",
    ) {
        line = line.replace(
            "The next face-down creature cast by you spell you cast this turn costs {3} less to cast",
            "The next face-down creature spell you cast this turn costs {3} less to cast",
        );
    }
    if lower == "destroy all artifacts, then destroy all enchantments." {
        return "Destroy all artifacts and enchantments.".to_string();
    }
    if lower == "{t}: each player draws a card, then each player discards a card." {
        return "{T}: Each player draws a card, then discards a card.".to_string();
    }
    if lower.starts_with(
        "whenever one or more creature attack an opponent or a planeswalker controlled by an opponent",
    ) {
        line = line.replace(
            "Whenever one or more creature attack an opponent or a planeswalker controlled by an opponent",
            "Whenever one or more creature attacking an opponent or a planeswalker controlled by an opponent",
        );
        line = line.replace(
            "whenever one or more creature attack an opponent or a planeswalker controlled by an opponent",
            "whenever one or more creature attacking an opponent or a planeswalker controlled by an opponent",
        );
    }
    if lower.contains("copy target instant or sorcery spell you control, then you may choose new targets for the copy")
    {
        line = line.replace(
            "Copy target instant or sorcery spell you control, then you may choose new targets for the copy",
            "Copy target instant or sorcery spell you control. You may choose new targets for the copy",
        );
        line = line.replace(
            "copy target instant or sorcery spell you control, then you may choose new targets for the copy",
            "copy target instant or sorcery spell you control. You may choose new targets for the copy",
        );
    }
    if lower == "destroy target artifact or enchantment, then populate." {
        return "Destroy target artifact or enchantment. Populate.".to_string();
    }
    if lower == "each player discards their hand, then each player draws seven cards." {
        return "Each player discards their hand, then draws seven cards.".to_string();
    }
    if lower.contains("look at the top x cards of your library")
        && lower.contains("you choose up to two cards")
        && lower.contains(
            "put the remaining tagged cards on the bottom of your library in a random order",
        )
    {
        let lower_line = line.to_ascii_lowercase();
        if let Some(idx) = lower_line.find("look at the top x cards of your library") {
            let mut normalized = String::with_capacity(line.len());
            normalized.push_str(&line[..idx]);
            normalized.push_str("Look at the top X cards of your library. Put up to two of them into your hand and the rest on the bottom of your library in a random order");
            return normalized;
        }
    }
    if lower.starts_with(
        "exile target creature card from your graveyard, create a 0/0 black zombie creature token",
    ) && lower.contains("base power and toughness")
    {
        return "Exile target creature card from your graveyard. Create a black Zombie creature token. Its power and toughness are each equal to that card's power and toughness.".to_string();
    }
    if lower.starts_with(
        "target opponent reveals their hand, you choose up to x nonland cards, exile it",
    ) && lower.contains("with the same name as that object")
    {
        line = line.replace(
            "you choose up to X nonland cards, exile it",
            "you choose up to X nonland cards from it and exile them",
        );
        line = line.replace(
            "you choose up to x nonland cards, exile it",
            "you choose up to X nonland cards from it and exile them",
        );
    }
    if lower.starts_with("when this creature enters, put x +1/+1 counters")
        && lower.contains("draw half x cards, rounded down")
    {
        line = line.replace("on him", "on this creature");
        line = line.replace("on Him", "on this creature");
    }
    if lower.contains("whenever an opponent searches their library")
        && lower.contains("then draw a card")
    {
        line = line.replace(", then draw a card", ". Draw a card");
        line = line.replace(", then Draw a card", ". Draw a card");
    }
    if lower.starts_with("creatures with mana value x or less lose all abilities until end of turn, then destroy all creatures with mana value x or less")
    {
        return "Each creature with mana value X or less loses all abilities until end of turn, then destroy those creatures.".to_string();
    }
    if lower.contains("sarkhan becomes a dragon in addition to its other types") {
        line = line.replace("sarkhan becomes", "Sarkhan becomes");
        line = line.replace("sarkhan gains", "Sarkhan gains");
    }
    if lower.contains(
        "add {c}. if this land has a luck counter on it, add one mana of any color instead",
    ) {
        line = line.replace(
            "If this land has a luck counter on it, add one mana of any color instead",
            "If this land has a luck counter on it, instead add one mana of any color",
        );
        line = line.replace(
            "if this land has a luck counter on it, add one mana of any color instead",
            "if this land has a luck counter on it, instead add one mana of any color",
        );
    }
    if lower.contains(
        "tap target creature or planeswalker. choose it. activated abilities of that permanent can't be activated this turn",
    ) {
        line = line.replace(
            "choose it. activated abilities of that permanent can't be activated this turn",
            "its activated abilities can't be activated this turn",
        );
    }
    if lower.contains("that permanent's mana value")
        && lower.contains("reveal the top card of your library")
    {
        line = line.replace("that permanent's mana value", "that card's mana value");
    }
    if lower.contains("as long as this creature is monstrous") {
        line = line.replace(
            "As long as this creature is monstrous",
            "as long as this creature is monstrous",
        );
    }
    if lower.contains(
        "that player chooses any number creatures that player controls on the battlefield",
    ) && lower.contains("a other creature that player controls can't attack this turn")
    {
        line = "at the beginning of combat on each opponent's turn, separate all creatures that player controls into two piles. only creatures in the pile of their choice can attack this turn".to_string();
    }
    if lower == "draw a card, then cipher." {
        line = "Draw a card. Cipher".to_string();
    }
    if lower
        == "look at target player's hand, look at the top card of target player's library, look at target player's face-down creature, look at the top four cards of your library, then put them back in any order."
        || lower
            == "look at target player's hand, look at the top card of target player's library, look at any face-down creatures they control, look at the top four cards of your library, then put them back in any order."
    {
        line = "Look at target player's hand, the top card of that player's library, and any face-down creatures they control. Look at the top four cards of your library, then put them back in any order.".to_string();
    }
    if lower.starts_with(
        "each opponent chooses any number creatures each opponent controls on the battlefield",
    ) && lower.contains("choose the separated pile")
        && lower.contains("choose the other pile")
    {
        line = "Each opponent separates the creatures they control into two piles. For each opponent, you choose one of their piles. Each opponent sacrifices the creatures in their chosen pile.".to_string();
    }
    if lower.starts_with(
        "enchant creature enchanted creature is an angel in addition to its other types",
    ) || lower.starts_with("enchanted creature is an angel in addition to its other types")
    {
        line = "Enchanted creature gets +4/+4, has flying and first strike, and is an Angel in addition to its other types.".to_string();
    }
    if lower.starts_with("when this creature enters, look at the top ten cards of your library, reveal it, you choose up to one other artifact cards")
        && lower.contains("for each card chosen this way")
        && lower.contains("put the remaining tagged cards on the bottom of your library in a random order")
    {
        line = "When this creature enters, reveal the top ten cards of your library. For each card type, you may put a card of that type from among the revealed cards into your hand. Put the rest on the bottom of your library in a random order.".to_string();
    }
    if (lower
        .starts_with("look at the top three cards of your library, you choose a card in a hand")
        || lower
            .starts_with("look at the top three cards of your library, choose a card in a hand"))
        && lower.contains("you may play those cards this turn")
    {
        line = "Look at the top three cards of your library. Put one of them into your hand, put one of them on the bottom of your library, and exile one of them. You may play the exiled card this turn.".to_string();
    }
    if lower.contains("opponent controls causes you to discard this card")
        && lower.contains("at the beginning of the next end step")
        && lower.contains("return this creature from your graveyard to the battlefield")
        && lower.contains("put a +1/+1 counter on it")
    {
        line = "Whenever a spell or ability an opponent controls causes you to discard this card, return this card from your graveyard to the battlefield with a +1/+1 counter on it at the beginning of the next end step.".to_string();
    }
    if lower.starts_with("an opponent chooses any number creature cards")
        && lower.contains("exile the tagged object 'divvy_chosen'")
        && lower.contains("return all other creature cards from your graveyard to the battlefield")
    {
        line = "Separate all creature cards in your graveyard into two piles. Exile the pile of an opponent's choice and return the other to the battlefield.".to_string();
    }
    if lower == "each other non-human creature enters with an additional +1/+1 counter on it." {
        line =
            "Each other non-Human creature you control enters with an additional +1/+1 counter on it."
                .to_string();
    }
    if lower.contains("you can't be targeted until your next turn")
        && lower.contains("prevent all damage that would be dealt to you until your next turn")
    {
        line = replace_ascii_case_insensitive_once(
            line,
            "you can't be targeted until your next turn, then prevent all damage that would be dealt to you until your next turn",
            "You gain protection from everything until your next turn",
            "you gain protection from everything until your next turn",
        );
    }
    if lower.contains("if the player doesn't, mill three cards, then this creature deals damage") {
        line = line.replace(
            "If the player doesn't, mill three cards",
            "If the player doesn't, you mill three cards",
        );
        line = line.replace(
            "if the player doesn't, mill three cards",
            "if the player doesn't, you mill three cards",
        );
    }
    if lower == "prevent all combat damage that would be dealt to you this turn, then populate." {
        line =
            "Prevent all combat damage that would be dealt to you this turn. Populate.".to_string();
    }
    if lower.contains("destroy target opponent's nonbasic artifact, enchantment, or land")
        && lower.contains("then an opponent may search an opponent's library for a basic land card")
    {
        line = line.replace(
            "target opponent's nonbasic artifact, enchantment, or land, then an opponent may search an opponent's library for a basic land card",
            "target opponent's nonbasic artifact, enchantment, or land. That permanent's controller may search their library for a basic land card",
        );
        line = line.replace(
            "target opponent's nonbasic artifact, enchantment, or land, then an opponent may search an opponent's library for a basic land card",
            "target opponent's nonbasic artifact, enchantment, or land. That permanent's controller may search their library for a basic land card",
        );
    }
    if lower.contains("if it's a creature or a planeswalker card")
        && lower.contains("if you don't put it into your hand")
    {
        line = line.replace(
            "If you don't put it into your hand",
            "If you don't put the card into your hand",
        );
        line = line.replace(
            "if you don't put it into your hand",
            "if you don't put the card into your hand",
        );
    }
    if let Some(rest) = line.strip_prefix("During your turn, this creature has ")
        && rest.to_ascii_lowercase().starts_with("prevent ")
    {
        line = format!("During your turn, {}", lowercase_first(rest));
    }
    line = line.replace(
        "Whenever an equipped creature deals combat damage to a player",
        "Whenever equipped creature deals combat damage to a player",
    );
    line = line.replace(
        "When an equipped creature deals combat damage to a player",
        "When equipped creature deals combat damage to a player",
    );
    line = line
        .replace(
            "When this token dies: You gain 1 life",
            "When this token dies, you gain 1 life",
        )
        .replace(
            "When this token dies: It deals 1 damage to any target",
            "When this token dies, it deals 1 damage to any target",
        );
    if line.to_ascii_lowercase().contains(
        "at the beginning of the next end step, if it matches card in exile, put it into its owner's graveyard",
    ) {
        line = line.replace(
            "At the beginning of the next end step, if it matches card in exile, put it into its owner's graveyard.",
            "At the beginning of the next end step, if any of those cards remain exiled, return them to their owners' graveyards.",
        );
        line = line.replace(
            "at the beginning of the next end step, if it matches card in exile, put it into its owner's graveyard.",
            "at the beginning of the next end step, if any of those cards remain exiled, return them to their owners' graveyards.",
        );
    }
    if line.to_ascii_lowercase().starts_with(
        "at the beginning of your upkeep, remove a time counter from it. when the last time counter is removed, sacrifice",
    ) {
        return "Vanishing".to_string();
    }
    if line.contains("Cascade and Cascade") {
        return line.replace("Cascade and Cascade", "Cascade, cascade");
    }
    line = line.replace(
        "Tap each creature that was blocked by one of those creatures this turn. It doesn't untap during its controller's next untap step",
        "Tap each creature that was blocked by one of those creatures this turn and it doesn't untap during its controller's next untap step",
    );
    line = line.replace(
        "tap each creature that was blocked by one of those creatures this turn. It doesn't untap during its controller's next untap step",
        "tap each creature that was blocked by one of those creatures this turn and it doesn't untap during its controller's next untap step",
    );
    line = line.replace(
        "target creature an opponent controls or planeswalker",
        "target creature or planeswalker an opponent controls",
    );
    line = line.replace(
        "Target creature an opponent controls or planeswalker",
        "Target creature or planeswalker an opponent controls",
    );
    line = line.replace(
        "target creature an opponent controls or enchantment",
        "target creature or enchantment an opponent controls",
    );
    line = line.replace(
        "Target creature an opponent controls or enchantment",
        "Target creature or enchantment an opponent controls",
    );
    let lower_line = line.to_ascii_lowercase();
    if !lower_line.contains("reveal the top card of your library")
        && !lower_line.contains("the exiled card")
    {
        line = line.replace(
            "lose life equal to its mana value",
            "lose life equal to that permanent's mana value",
        );
        line = line.replace(
            "Lose life equal to its mana value",
            "Lose life equal to that permanent's mana value",
        );
    }
    line = line.replace(
        "At the beginning of the next end step, you lose 1 life. Return this card to its owner's hand",
        "At the beginning of the next end step, you lose 1 life and return this card to your hand",
    );
    line = line.replace(
        "at the beginning of the next end step, you lose 1 life. return this card to its owner's hand",
        "at the beginning of the next end step, you lose 1 life and return this card to your hand",
    );
    line = replace_ascii_case_insensitive_once(
        line,
        "tap each creature that was blocked by one of those creatures this turn. it doesn't untap during its controller's next untap step",
        "Tap each creature that was blocked by one of those creatures this turn and it doesn't untap during its controller's next untap step",
        "tap each creature that was blocked by one of those creatures this turn and it doesn't untap during its controller's next untap step",
    );
    line = replace_ascii_case_insensitive_once(
        line,
        "at the beginning of the next end step, you lose 1 life. return this card to its owner's hand",
        "At the beginning of the next end step, you lose 1 life and return this card to your hand",
        "at the beginning of the next end step, you lose 1 life and return this card to your hand",
    );
    line = line.replace("non-Auran enchantments", "non-Aura enchantments");
    line = line.replace("non-Auran enchantment", "non-Aura enchantment");
    line = line.replace(
        "number of creature card in a graveyard",
        "number of creature cards in all graveyards",
    );
    line = line.replace(
        "number of instant or sorcery card in a graveyard",
        "number of instant and sorcery cards in all graveyards",
    );
    line = line.replace(
        "number of other creature artifact you control",
        "number of other creatures and/or artifacts you control",
    );
    line = line.replace(
        "number of another creature or artifact you control",
        "number of other creatures and/or artifacts you control",
    );
    line = line.replace(
        "number of other creature.",
        "number of other creatures on the battlefield.",
    );
    line = line.replace(
        "number of another creature.",
        "number of other creatures on the battlefield.",
    );
    line = line.replace("This creature creature's", "This creature's");
    line = line.replace("this creature creature's", "this creature's");
    if let Some(each) = line
        .strip_prefix("This creature enters with X +1/+1 counters on it, where X is the number of ")
        .filter(|each| each.contains("creatures and/or artifacts"))
    {
        let each = each.trim_end_matches('.');
        let each = each
            .replace("creatures and/or artifacts", "creature and/or artifact")
            .replace("creatures ", "creature ")
            .replace("artifacts ", "artifact ");
        return format!("This creature enters with a +1/+1 counter on it for each {each}");
    }
    line = normalize_conditional_additional_x_counters(&line);
    line = normalize_adamant_enters_with_counter_clause(&line);
    if line
        .to_ascii_lowercase()
        .contains("a land is put into a graveyard from the battlefield")
        && line.contains("that object's controller")
    {
        return line.replace("that object's controller", "that land's controller");
    }
    line = normalize_conditional_followup_case(&line);
    line = line.replace(
        ". Then if {S} was spent to cast this spell, that permanent doesn't untap ",
        ". If {S} was spent to cast this spell, that permanent doesn't untap ",
    );
    line = normalize_activation_colon_payload_case(&line);
    line = normalize_top_card_exile_imperative(&line);
    line = normalize_exact_during_your_turn_predicate_surface(&line);
    line = normalize_sacrifice_enchantment_counter_spell_trigger(&line);
    line = normalize_self_exile_attacking_nonflying_creature_surface(&line);
    line = normalize_spellcast_trigger_copy_spell_surface(&line);
    line = normalize_basic_land_type_choice_surface(&line);
    line = normalize_choose_sacrifice_rest_surface(&line);
    line = normalize_for_each_number_surface(&line);
    line = normalize_temporary_trample_pump_surface(&line);
    line = normalize_where_x_before_duration_surface(&line);
    line = normalize_chosen_player_adds_mana_surface(&line);
    line = normalize_role_token_attached_surface(&line);
    line = normalize_create_role_then_attach_surface(&line);
    line = normalize_return_with_counter_surface(&line);
    line = normalize_simple_token_keyword_surface(&line);
    line = normalize_chosen_creature_type_surface(&line);
    line = normalize_token_quoted_ability_surfaces(&line);
    line = remove_redundant_period_after_terminal_quote(&line);
    line = line
        .replace(
            "When this token dies: You gain 1 life",
            "When this token dies, you gain 1 life",
        )
        .replace(
            "When this token dies: It deals 1 damage to any target",
            "When this token dies, it deals 1 damage to any target",
        );
    line = line.replace(
        "Tap it. That permanent doesn't untap during its controller's next untap step",
        "Tap it. It doesn't untap during its controller's next untap step",
    );
    line = line.replace(
        "tap it. That permanent doesn't untap during its controller's next untap step",
        "tap it. It doesn't untap during its controller's next untap step",
    );
    line = replace_ascii_case_insensitive_once(
        line,
        "choose it. activated abilities of that permanent can't be activated this turn",
        "Its activated abilities can't be activated this turn",
        "its activated abilities can't be activated this turn",
    );
    if line
        .to_ascii_lowercase()
        .contains("reveal the top card of your library")
    {
        line = line.replace("that permanent's mana value", "that card's mana value");
    }
    line = capitalize_sentence_boundaries(&line);
    let finalized = if line.contains('\n') {
        line.split('\n')
            .map(|source_line| {
                if is_keyword_style_line(source_line) {
                    source_line.to_string()
                } else {
                    ensure_trailing_period(source_line)
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    } else if is_keyword_style_line(&line) {
        line
    } else {
        ensure_trailing_period(&line)
    };
    if finalized.contains("\n•") {
        finalized.replace("—.\n•", "—\n•")
    } else {
        finalized
    }
}

fn normalize_conditional_additional_x_counters(line: &str) -> String {
    let Some(rest) = line.strip_prefix(
        "This creature enters with X +1/+1 counters on it. This creature enters with X +1/+1 counters on it if ",
    ) else {
        return line.to_string();
    };
    let condition = rest.trim().trim_end_matches('.').replace("x is", "X is");
    if condition.is_empty() {
        return line.to_string();
    }
    format!(
        "This creature enters with X +1/+1 counters on it. If {condition}, it enters with an additional X +1/+1 counters on it"
    )
}

fn normalize_adamant_enters_with_counter_clause(line: &str) -> String {
    let Some((enter_clause, condition_clause)) = line.split_once(" if ") else {
        return line.to_string();
    };
    if !enter_clause.starts_with("This creature enters with ") || !enter_clause.ends_with(" on it")
    {
        return line.to_string();
    }
    let condition = condition_clause.trim().trim_end_matches('.');
    if !condition.contains(" mana was spent to cast this spell") {
        return line.to_string();
    }
    let mut enter_text = enter_clause.to_string();
    if let Some(first) = enter_text.chars().next() {
        let lower = first.to_ascii_lowercase();
        enter_text.replace_range(0..first.len_utf8(), &lower.to_string());
    }
    format!("Adamant — If {condition}, {enter_text}")
}

fn normalize_conditional_followup_case(line: &str) -> String {
    let mut normalized = line.to_string();
    for verb in [
        "Add",
        "Attach",
        "Choose",
        "Copy",
        "Counter",
        "Create",
        "Destroy",
        "Discard",
        "Draw",
        "Exile",
        "Gain",
        "Lose",
        "Mill",
        "Put",
        "Return",
        "Sacrifice",
        "Search",
        "Tap",
        "Untap",
    ] {
        let lowered = lowercase_first(verb);
        normalized = lowercase_conditional_comma_followup(&normalized, verb, &lowered);
        normalized = normalized.replace(
            &format!("Otherwise, {verb} "),
            &format!("Otherwise, {lowered} "),
        );
    }
    normalized = normalized.replace("Otherwise, You ", "Otherwise, you ");
    // A branch whose sentence starts with a player reference ("That player
    // gains control ...") continues the "Otherwise," sentence.
    for subject in ["That ", "Their ", "Its ", "Target ", "Each "] {
        normalized = normalized.replace(
            &format!("Otherwise, {subject}"),
            &format!("Otherwise, {}", lowercase_first(subject)),
        );
    }
    normalized
}

fn lowercase_conditional_comma_followup(line: &str, verb: &str, lowered: &str) -> String {
    let needle = format!(", {verb} ");
    let mut normalized = line.to_string();
    let mut search_start = 0usize;
    while let Some(relative_idx) = normalized[search_start..].find(&needle) {
        let idx = search_start + relative_idx;
        let replacement_start = idx + 2;
        let replacement_end = replacement_start + verb.len();
        if comma_follows_conditional_marker(&normalized[..idx]) {
            normalized.replace_range(replacement_start..replacement_end, lowered);
        }
        search_start = idx + needle.len();
    }
    normalized
}

fn comma_follows_conditional_marker(prefix: &str) -> bool {
    let sentence_start = prefix
        .rfind(['.', '\n', ';'])
        .map(|idx| idx + 1)
        .unwrap_or(0);
    let segment = prefix[sentence_start..].trim_start().to_ascii_lowercase();
    segment.starts_with("if ")
        || segment.contains(", if ")
        || segment.starts_with("for each ")
        || segment.contains(", for each ")
        || segment.starts_with("otherwise")
}

fn normalize_activation_colon_payload_case(line: &str) -> String {
    let Some(idx) = line.rfind(": ") else {
        return line.to_string();
    };
    let payload_start = idx + 2;
    let Some(first) = line[payload_start..].chars().next() else {
        return line.to_string();
    };
    if !first.is_ascii_lowercase() {
        return line.to_string();
    }
    let mut normalized = String::with_capacity(line.len());
    normalized.push_str(&line[..payload_start]);
    normalized.push(first.to_ascii_uppercase());
    normalized.push_str(&line[payload_start + first.len_utf8()..]);
    normalized
}

fn replace_ascii_case_insensitive_once(
    line: String,
    needle_lower: &str,
    replacement_upper: &str,
    replacement_lower: &str,
) -> String {
    let lower = line.to_ascii_lowercase();
    let Some(idx) = lower.find(needle_lower) else {
        return line;
    };
    let end = idx + needle_lower.len();
    let replacement = if line[idx..end]
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
    {
        replacement_upper
    } else {
        replacement_lower
    };
    format!("{}{}{}", &line[..idx], replacement, &line[end..])
}

fn merge_ast_surface_lines(mut lines: Vec<String>) -> Vec<String> {
    loop {
        let previous = lines;
        let merged = merge_subject_predicate_surface_lines(previous.clone());
        let merged = merge_same_true_keyword_grant_lines(merged);
        let merged = merge_same_true_type_addition_lines(merged);
        let merged = merge_same_true_color_lines(merged);
        let merged = annotate_color_choice_exclusions(merged);
        let merged = merge_blockability_lines(merged);
        let merged = merge_attached_transform_keyword_loss_lines(merged);
        let merged = merge_lose_all_transform_lines(merged);
        let merged = merge_shared_as_long_as_tail_lines(merged);
        let merged = merge_base_pt_loss_transform_lines(merged);
        let merged = merge_specific_adjacent_surface_lines(merged);
        let merged = drop_redundant_spell_cost_lines(merged);
        let merged = merge_adjacent_simple_mana_add_lines(merged);
        let merged = merge_conditioned_spell_and_activation_tax_lines(merged);
        let merged = merge_as_enters_color_and_creature_type_choice_lines(merged);
        let merged = merge_cast_permission_any_mana_lines(merged);
        let merged = merged
            .into_iter()
            .map(|line| compact_each_color_target_list(&line).unwrap_or(line))
            .map(|line| compact_any_number_of_opponents_target(&line).unwrap_or(line))
            .map(|line| compact_return_target_with_attached_auras(&line).unwrap_or(line))
            .map(|line| split_friend_or_foe_sentences(&line).unwrap_or(line))
            .map(|line| {
                // "An opponent chooses a card at random in your graveyard"
                // (Search for Survivors): a chosen opponent who only makes
                // the next choice is that choice's subject.
                const CHOSEN_OPPONENT_CHOOSES: &str = "Choose an opponent. That player chooses ";
                if let Some((head, tail)) = line.split_once(CHOSEN_OPPONENT_CHOOSES)
                    && !tail.contains("that player")
                    && !tail.contains("That player")
                    && !tail.contains("the chosen player")
                    && !tail.contains("the chosen opponent")
                {
                    format!("{head}An opponent chooses {tail}")
                } else {
                    line
                }
            })
            .map(|line| {
                // "you may choose a card you own outside the game. Put it
                // into its owner's hand" is one optional wish: "you may put a
                // card you own from outside the game into your hand" (Death
                // Wish, North Wind Avatar).
                line.replace(
                    "ou may choose a card you own outside the game. Put it into its owner's hand",
                    "ou may put a card you own from outside the game into your hand",
                )
            })
            .collect::<Vec<_>>();
        if merged == previous {
            return merged;
        }
        lines = merged;
    }
}

/// "For each player, choose friend or foe, each friend ..., then each foe
/// ..." reads as oracle's separate sentences: "For each player, choose
/// friend or foe. Each friend .... Each foe ...." (Virtus's Maneuver)
fn split_friend_or_foe_sentences(line: &str) -> Option<String> {
    const HEAD: &str = "For each player, choose friend or foe, each friend ";
    // "for each player who chose friend, create <token> under that player's
    // control" is the friend set creating the token (Zndrsplt's Judgment).
    const CREATE_HEAD: &str =
        "For each player, choose friend or foe, for each player who chose friend, create ";
    if let Some(rest) = line.strip_prefix(CREATE_HEAD)
        && let Some((token, tail)) = rest.split_once(" under that player's control, then each foe ")
    {
        let token = token.replace(" that player controls", " they control");
        return Some(format!(
            "For each player, choose friend or foe. Each friend creates {token}. Each foe {tail}"
        ));
    }
    let rest = line.strip_prefix(HEAD)?;
    let rest = rest.replacen(", then each foe ", ". Each foe ", 1);
    Some(format!(
        "For each player, choose friend or foe. Each friend {rest}"
    ))
}

/// "Return all Auras attached to target creature to their owners' hands.
/// Return that creature to its owner's hand." is one bounce of the creature
/// together with its Auras: "Return target creature and all Auras attached
/// to it to their owners' hands." (Hubris, Word of Undoing)
fn compact_return_target_with_attached_auras(line: &str) -> Option<String> {
    const HEAD: &str = "Return all ";
    const ATTACHED: &str = " attached to target creature to ";
    const SECOND: &str = ". Return that creature to its owner's hand";
    let start = line.find(HEAD)?;
    if start != 0 && !line[..start].ends_with(". ") {
        return None;
    }
    let after = &line[start + HEAD.len()..];
    let (auras, rest) = after.split_once(ATTACHED)?;
    if !auras.ends_with("Auras") && !auras.contains("Auras you own") {
        return None;
    }
    let (destination, tail) = rest.split_once(SECOND)?;
    if !matches!(destination, "their owners' hands" | "your hand" | "its owner's hand") {
        return None;
    }
    Some(format!(
        "{}Return target creature and all {auras} attached to it to their owners' hands{tail}",
        &line[..start]
    ))
}

/// "Destroy any number of target nonland permanents an opponent controls
/// controlled by different players" is one target per chosen opponent:
/// oracle's "For any number of opponents, destroy target nonland permanent
/// that player controls" (Windgrace's Judgment).
fn compact_any_number_of_opponents_target(line: &str) -> Option<String> {
    const SUFFIX: &str = " an opponent controls controlled by different players";
    let trimmed = line.trim_end();
    let (body, period) = match trimmed.strip_suffix('.') {
        Some(body) => (body, "."),
        None => (trimmed, ""),
    };
    let head = body.strip_suffix(SUFFIX)?;
    let (verb, objects) = head.split_once(" any number of target ")?;
    if verb.is_empty() || verb.contains(' ') || verb.contains('.') {
        return None;
    }
    let (rest, last) = match objects.rsplit_once(' ') {
        Some((rest, last)) => (format!("{rest} "), last),
        None => (String::new(), objects),
    };
    let singular_last = last.strip_suffix('s')?;
    Some(format!(
        "For any number of opponents, {} target {rest}{singular_last} that player controls{period}",
        verb.to_ascii_lowercase()
    ))
}

/// "Return up to one target white card, up to one target blue card, ...,
/// and up to one target green card from your graveyard" is one target per
/// color: oracle's "For each color, return up to one target card of that
/// color from your graveyard" (All Suns' Dawn, Rogues' Gallery).
fn compact_each_color_target_list(line: &str) -> Option<String> {
    const WHITE: &str = "up to one target white ";
    let start = line.find(WHITE)?;
    let after = &line[start + WHITE.len()..];
    let (noun, rest) = after.split_once(", up to one target blue ")?;
    let rest = rest
        .strip_prefix(noun)?
        .strip_prefix(", up to one target black ")?
        .strip_prefix(noun)?
        .strip_prefix(", up to one target red ")?
        .strip_prefix(noun)?
        .strip_prefix(", and up to one target green ")?
        .strip_prefix(noun)?;
    let prefix = &line[..start];
    let (sentence_head, verb) = match prefix.rfind(". ") {
        Some(idx) => (&prefix[..idx + 2], &prefix[idx + 2..]),
        None => ("", prefix),
    };
    let verb = verb.trim();
    if verb.is_empty() || verb.contains(' ') || noun.contains(',') {
        return None;
    }
    Some(format!(
        "{sentence_head}For each color, {} up to one target {noun} of that color{rest}",
        verb.to_ascii_lowercase()
    ))
}

fn merge_as_enters_color_and_creature_type_choice_lines(lines: Vec<String>) -> Vec<String> {
    fn choice_prefix<'a>(line: &'a str, suffix: &str) -> Option<&'a str> {
        line.trim().trim_end_matches('.').strip_suffix(suffix)
    }

    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0;
    while idx < lines.len() {
        if let Some(next) = lines.get(idx + 1) {
            let color_then_type = choice_prefix(&lines[idx], ", choose a color")
                .zip(choice_prefix(next, ", choose a creature type"));
            let type_then_color = choice_prefix(&lines[idx], ", choose a creature type")
                .zip(choice_prefix(next, ", choose a color"));
            if let Some((first_prefix, second_prefix)) = color_then_type.or(type_then_color)
                && first_prefix == second_prefix
            {
                merged.push(format!(
                    "{first_prefix}, choose a color and a creature type"
                ));
                idx += 2;
                continue;
            }
        }
        merged.push(join_as_enters_sequential_choices(&lines[idx]));
        idx += 1;
    }
    merged
}

/// "As this enchantment enters, choose a color and an opponent." (Call to
/// Arms): two as-enters choices are one coordinated choice; the generic
/// sequence joiner's ", then choose" never appears in an as-enters line.
fn join_as_enters_sequential_choices(line: &str) -> String {
    if !line.starts_with("As ") {
        return line.to_string();
    }
    let Some((head, tail)) = line.split_once(" enters, choose ") else {
        return line.to_string();
    };
    let Some((first, second)) = tail.split_once(", then choose ") else {
        return line.to_string();
    };
    let is_simple_choice = |choice: &str| {
        let choice = choice.trim_end_matches('.');
        (choice.starts_with("a ") || choice.starts_with("an "))
            && !choice.contains(',')
            && !choice.contains(". ")
            && choice.split_whitespace().count() <= 3
    };
    if !is_simple_choice(first) || !is_simple_choice(second) {
        return line.to_string();
    }
    format!("{head} enters, choose {first} and {second}")
}

/// Adjacent standalone statics sharing an identical "as long as" condition
/// tail merge into one conjoined sentence, matching oracle's single-clause
/// phrasing ("A and b as long as COND.").
fn merge_shared_as_long_as_tail_lines(lines: Vec<String>) -> Vec<String> {
    fn split_static_condition(line: &str) -> Option<(&str, &str, &'static str)> {
        let trimmed = line.trim().trim_end_matches('.');
        if trimmed.contains(':') {
            return None;
        }
        let lower = trimmed.to_ascii_lowercase();
        for prefix in ["when ", "whenever ", "at the beginning", "if "] {
            if lower.starts_with(prefix) {
                return None;
            }
        }
        // The unleash pair ("can't block as long as it has a +1/+1 counter")
        // must survive for its dedicated collapse in
        // merge_specific_adjacent_surface_lines.
        if lower.contains("can't block") && lower.contains("+1/+1 counter") {
            return None;
        }
        let marker = " as long as ";
        if let Some(first) = lower.find(marker) {
            if lower.rfind(marker) != Some(first) {
                return None;
            }
            let head = trimmed[..first].trim();
            let condition = trimmed[first + marker.len()..].trim();
            // "for as long as" is a one-shot duration, and a labeled chapter
            // ("I — ...") is its own triggered line.
            if head.is_empty()
                || condition.is_empty()
                || head.to_ascii_lowercase().ends_with(" for")
                || head.contains(" — ")
            {
                return None;
            }
            // The pump + can't-block threshold pair has a dedicated
            // ability-word compaction (compact_threshold_pump_and_cant_block)
            // that must see the two lines unmerged.
            if condition == "there are seven or more cards in your graveyard" {
                return None;
            }
            return Some((head, condition, "as long as"));
        }
        // A trailing " if " condition merges only for "can't"-style statics,
        // where the shared condition unambiguously scopes the whole sentence.
        let if_marker = " if ";
        let first = lower.find(if_marker)?;
        if lower.rfind(if_marker) != Some(first) {
            return None;
        }
        let head = trimmed[..first].trim();
        let condition = trimmed[first + if_marker.len()..].trim();
        if head.is_empty()
            || condition.is_empty()
            || !head.to_ascii_lowercase().contains("can't be")
        {
            return None;
        }
        Some((head, condition, "if"))
    }

    fn heads_share_subject(left: &str, right: &str) -> bool {
        match (
            split_subject_predicate_clause(left),
            split_subject_predicate_clause(right),
        ) {
            (Some((left_subject, _, _)), Some((right_subject, _, _))) => {
                left_subject.eq_ignore_ascii_case(right_subject)
            }
            // If a specialized surface cannot be decomposed, retain the
            // established tail-sharing behavior.
            _ => true,
        }
    }

    // The second static of a shared-subject pair may restate the condition
    // through the pronoun ("as long as it's monstrous"); the pair then reads
    // as one subject with two predicates.
    fn pronoun_condition_matches(left_head: &str, left_cond: &str, right_cond: &str) -> bool {
        let Some((subject, _, _)) = split_subject_predicate_clause(left_head) else {
            return false;
        };
        right_cond
            .strip_prefix("it's ")
            .is_some_and(|state| left_cond.eq_ignore_ascii_case(&format!("{subject} is {state}")))
    }

    // "<Subject> has hexproof as long as <Subject> hasn't dealt damage yet." +
    // "As long as it hasn't dealt damage yet, this creature can't be
    // blocked." is one authored static: "As long as <Subject> hasn't dealt
    // damage yet, it has hexproof and can't be blocked."
    fn merge_leading_pronoun_condition_pair(left: &str, right: &str) -> Option<String> {
        let (left_head, left_cond, marker) = split_static_condition(left)?;
        if marker != "as long as" || left_head.contains(" and ") {
            return None;
        }
        let (left_subject, _, _) = split_subject_predicate_clause(left_head)?;
        let left_predicate = left_head[left_subject.len()..].trim();
        let cond_rest = left_cond
            .strip_prefix(left_subject)
            .and_then(|rest| rest.strip_prefix(' '))?;
        let right = right.trim().trim_end_matches('.');
        let right_body = right.strip_prefix("As long as it ")?;
        let right_tail = right_body.strip_prefix(cond_rest)?.strip_prefix(", ")?;
        let right_predicate = ["this creature ", "this permanent ", "it "]
            .iter()
            .find_map(|prefix| right_tail.strip_prefix(prefix))
            .or_else(|| {
                right_tail
                    .strip_prefix(left_subject)
                    .and_then(|rest| rest.strip_prefix(' '))
            })?;
        if left_predicate.is_empty()
            || right_predicate.is_empty()
            || right_predicate.contains(" and ")
            || right_predicate.contains(", ")
            || right_predicate.contains(" as long as ")
        {
            return None;
        }
        Some(format!(
            "As long as {left_cond}, it {left_predicate} and {right_predicate}."
        ))
    }

    // "Creatures you control of the chosen type get +1/+1." + "As long as you
    // have the city's blessing, creatures you control of the chosen type have
    // vigilance." (Radiant Destiny): the conditional grant names the same
    // recipients, so it reads as a pronoun continuation of the bonus.
    fn merge_conditional_recipient_continuation(left: &str, right: &str) -> Option<String> {
        let left = left.trim().trim_end_matches('.');
        if left.contains(':') || left.to_ascii_lowercase().contains(" as long as ") {
            return None;
        }
        let (subject, verb, _) = split_subject_predicate_clause(left)?;
        if verb != "get" {
            return None;
        }
        let right = right.trim().trim_end_matches('.');
        let body = right.strip_prefix("As long as ")?;
        let (condition, grant) = body.split_once(", ")?;
        let keywords = grant
            .strip_prefix(subject)
            .or_else(|| {
                let lowered = lowercase_first(subject);
                grant.strip_prefix(lowered.as_str()).map(|_| &grant[lowered.len()..])
            })?
            .strip_prefix(" have ")?;
        if condition.is_empty() || keywords.is_empty() || keywords.contains(". ") {
            return None;
        }
        Some(format!(
            "{left}. As long as {condition}, they also have {keywords}."
        ))
    }

    let mut merged: Vec<String> = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    while idx < lines.len() {
        if idx + 1 < lines.len()
            && let Some(joined) =
                merge_conditional_recipient_continuation(&lines[idx], &lines[idx + 1])
        {
            merged.push(joined);
            idx += 2;
            continue;
        }
        if idx + 1 < lines.len()
            && let Some(joined) =
                merge_leading_pronoun_condition_pair(&lines[idx], &lines[idx + 1])
        {
            merged.push(joined);
            idx += 2;
            continue;
        }
        if idx + 1 < lines.len()
            && let (
                Some((left_head, left_cond, left_marker)),
                Some((right_head, right_cond, right_marker)),
            ) = (
                split_static_condition(&lines[idx]),
                split_static_condition(&lines[idx + 1]),
            )
            && left_marker == right_marker
            && left_marker == "as long as"
            && !left_head.contains(" and ")
            && !right_head.contains(" and ")
            && pronoun_condition_matches(left_head, left_cond, right_cond)
            && let Some((left_subject, _, _)) = split_subject_predicate_clause(left_head)
            && right_head
                .to_ascii_lowercase()
                .starts_with(&format!("{} ", left_subject.to_ascii_lowercase()))
        {
            let right_verb_phrase = right_head[left_subject.len()..].trim();
            merged.push(format!(
                "As long as {left_cond}, it {} and {right_verb_phrase}.",
                left_head[left_subject.len()..].trim()
            ));
            idx += 2;
            continue;
        }
        if idx + 1 < lines.len()
            && let (
                Some((left_head, left_cond, left_marker)),
                Some((right_head, right_cond, right_marker)),
            ) = (
                split_static_condition(&lines[idx]),
                split_static_condition(&lines[idx + 1]),
            )
            && left_cond.eq_ignore_ascii_case(right_cond)
            && left_marker == right_marker
            && !left_head.contains(" and ")
            && !right_head.contains(" and ")
            // Different object populations can overlap, so a trailing
            // condition cannot safely be moved over their conjunction. Safe
            // disjoint unions (for example, the source plus "other Knights")
            // are handled earlier by the typed subject-union merger.
            && heads_share_subject(left_head, right_head)
        {
            let mut right = right_head.to_string();
            let mut chars = right_head.chars();
            if let (Some(first), Some(second)) = (chars.next(), chars.next())
                && first.is_ascii_uppercase()
                && second.is_ascii_lowercase()
            {
                right = format!(
                    "{}{}",
                    first.to_ascii_lowercase(),
                    &right_head[first.len_utf8()..]
                );
            }
            merged.push(format!(
                "{left_head} and {right} {left_marker} {left_cond}."
            ));
            idx += 2;
            continue;
        }
        merged.push(lines[idx].clone());
        idx += 1;
    }
    merged
}

fn merge_specific_adjacent_surface_lines(lines: Vec<String>) -> Vec<String> {
    let mut merged = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    while idx < lines.len() {
        if idx + 1 < lines.len() {
            let left = lines[idx].trim().trim_end_matches('.');
            let right = lines[idx + 1].trim().trim_end_matches('.');
            let left_lower = left.to_ascii_lowercase();
            let right_lower = right.to_ascii_lowercase();
            if left_lower.ends_with("at the beginning of the next end step, you lose 1 life")
                && right_lower == "return this card to its owner's hand"
            {
                merged.push(format!("{left} and return this card to your hand."));
                idx += 2;
                continue;
            }
            // Champion's linked return trigger is part of the keyword
            // (CR 702.72a); the ETB half already renders as "Champion a X".
            if left_lower.starts_with("champion ")
                && right_lower.starts_with("when this ")
                && right_lower.contains(
                    "leaves the battlefield, return the exiled card to the battlefield under its owner's control",
                )
            {
                merged.push(lines[idx].clone());
                idx += 2;
                continue;
            }
            // The unleash scaffold: an optional entry counter plus the
            // can't-block rider it gates. Both lines vary in how they name the
            // source (repeated noun, pronoun, or the doubled "this creature
            // creature" the restriction subject can produce), so match the
            // invariant parts of the pair rather than two exact strings.
            if (left_lower.starts_with("when this ") || left_lower.starts_with("as this "))
                && left_lower.contains("enters, you may put a +1/+1 counter on")
                && right_lower.contains("can't block as long as it has a +1/+1 counter on it")
                && right_lower.starts_with("this ")
            {
                merged.push("Unleash".to_string());
                idx += 2;
                continue;
            }
            // "Gorm must be blocked if able, and Gorm must be blocked by two
            // or more creatures if able": one authored lure-and-menace line.
            if let Some(subject) = left.strip_suffix(" must be blocked if able")
                && let Some(rest) = right.strip_prefix(subject)
                && rest.starts_with(" must be blocked by ")
                && rest.ends_with(" or more creatures if able")
                && !subject.contains(',')
            {
                merged.push(format!("{left}, and {right}."));
                idx += 2;
                continue;
            }
            // "This creature enters tapped and doesn't untap during your
            // untap step" is joined only from its authored source-line group
            // (ast_render describe_source_line_enters_tapped_untap_restriction_pair);
            // a line-level merge here also fused cards printing the two rules
            // on separate lines (Deep-Slumber Titan, Time Vault).
            if left_lower
                .ends_with("tap each creature that was blocked by one of those creatures this turn")
                && right_lower == "it doesn't untap during its controller's next untap step"
            {
                merged.push(format!(
                    "{left} and it doesn't untap during its controller's next untap step."
                ));
                idx += 2;
                continue;
            }
            // "You may cast this card from your graveyard or from exile":
            // one permission per zone, authored as one sentence.
            if let Some(left_zone) = left.strip_prefix("You may cast this card from ")
                && let Some(right_zone) = right.strip_prefix("You may cast this card from ")
                && [left_zone, right_zone].iter().all(|zone| {
                    matches!(*zone, "your graveyard" | "exile" | "your hand" | "your library")
                })
            {
                merged.push(format!(
                    "You may cast this card from {left_zone} or from {right_zone}."
                ));
                idx += 2;
                continue;
            }
            if let Some(merged_restriction) = merge_cast_and_activate_restriction_lines(left, right)
            {
                merged.push(merged_restriction);
                idx += 2;
                continue;
            }
            if let Some(pump) = left.strip_prefix("Each creature you control gets ")
                && let Some(pump) = pump.strip_suffix(" until end of turn")
                && right_lower == "if it is not your turn, untap that creature"
            {
                merged.push(format!(
                    "Creatures you control get {pump} until end of turn. If it's not your turn, untap those creatures."
                ));
                idx += 2;
                continue;
            }
            if left == "This creature enters with X +1/+1 counters on it"
                && let Some(condition) =
                    right_lower.strip_prefix("this creature enters with x +1/+1 counters on it if ")
            {
                merged.push(format!(
                    "{left}. If {}, it enters with an additional X +1/+1 counters on it.",
                    condition.replace("x is", "X is")
                ));
                idx += 2;
                continue;
            }
            if let Some((left_counter, left_condition)) =
                split_self_enters_with_counter_if_condition(left)
                && let Some((right_counter, right_condition)) =
                    split_self_enters_with_counter_if_condition(right)
                && left_condition.eq_ignore_ascii_case(right_condition)
            {
                merged.push(format!(
                    "If {left_condition}, this creature enters with {left_counter} and {right_counter} on it."
                ));
                idx += 2;
                continue;
            }
        }
        merged.push(lines[idx].clone());
        idx += 1;
    }
    merged
}

fn split_self_enters_with_counter_if_condition(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix("This creature enters with ")?;
    let (counter_phrase, condition) = rest.split_once(" on it if ")?;
    if counter_phrase.is_empty() || condition.is_empty() {
        return None;
    }
    Some((counter_phrase, condition.trim_end_matches('.')))
}

fn merge_cast_and_activate_restriction_lines(left: &str, right: &str) -> Option<String> {
    let (left_condition, left_body) = split_condition_prefix(left);
    let (right_condition, right_body) = split_condition_prefix(right);
    if !left_condition.eq_ignore_ascii_case(&right_condition) {
        return None;
    }

    let left_subject = left_body.strip_suffix(" can't cast spells")?.trim();
    let (right_subject, activation_restriction) =
        right_body.split_once(" can't activate abilities of ")?;
    if !left_subject.eq_ignore_ascii_case(right_subject.trim()) {
        return None;
    }

    let activation_restriction = normalize_or_list_surface(activation_restriction.trim());
    let subject = lowercase_first(left_subject);
    let body =
        format!("{subject} can't cast spells or activate abilities of {activation_restriction}");
    if left_condition.is_empty() {
        Some(body)
    } else {
        Some(format!("{left_condition}, {body}"))
    }
}

fn split_condition_prefix(line: &str) -> (String, &str) {
    let Some((condition, body)) = line.split_once(", ") else {
        return (String::new(), line);
    };
    if condition.eq_ignore_ascii_case("During your turn")
        || condition.to_ascii_lowercase().starts_with("as long as ")
    {
        (condition.to_string(), body)
    } else {
        (String::new(), line)
    }
}

fn normalize_or_list_surface(text: &str) -> String {
    let parts = text
        .replace(',', " ")
        .split_whitespace()
        .filter(|part| !part.eq_ignore_ascii_case("or"))
        .map(|part| part.to_ascii_lowercase())
        .collect::<Vec<_>>();
    join_with_or(&parts)
}

fn annotate_color_choice_exclusions(mut lines: Vec<String>) -> Vec<String> {
    for idx in 0..lines.len().saturating_sub(1) {
        let line = lines[idx].trim_end_matches('.');
        if !line.starts_with("As this ")
            || !line.ends_with(" enters, choose a color")
            || line.contains(" other than ")
        {
            continue;
        }

        let next = lines[idx + 1].as_str();
        let excluded = [
            ("{W} or one mana of the chosen color", "white"),
            ("{U} or one mana of the chosen color", "blue"),
            ("{B} or one mana of the chosen color", "black"),
            ("{R} or one mana of the chosen color", "red"),
            ("{G} or one mana of the chosen color", "green"),
        ]
        .iter()
        .find_map(|(needle, color)| next.contains(needle).then_some(*color));
        if let Some(color) = excluded {
            lines[idx] = format!("{line} other than {color}");
        }
    }
    lines
}

fn merge_subject_predicate_surface_lines(mut lines: Vec<String>) -> Vec<String> {
    loop {
        let previous = lines;
        let merged = collapse_repeated_conditional_bodies(merge_subject_animation_lines(
            merge_subject_has_keyword_lines(merge_adjacent_subject_predicate_lines(
                merge_player_object_subject_union_lines(
                    merge_during_your_turn_subject_union_lines(
                        merge_adjacent_color_cant_block_lines(
                            merge_adjacent_joined_sentence_lines(previous.clone()),
                        ),
                    ),
                ),
            )),
        ));
        if merged == previous {
            return merged;
        }
        lines = merged;
    }
}

/// Adjacent static lines the parser split out of one authored sentence
/// re-join into that sentence. Each entry is (first line, second line,
/// joined oracle sentence); both lines must appear adjacently and in order.
const ADJACENT_LINE_JOINS: &[(&str, &str, &str)] = &[
    // The graveyard self-cast permission plus its graveyard-only cast
    // restriction (one authored "but not from anywhere else" line).
    (
        "You may cast this card from your graveyard.",
        "You can cast this card only from your graveyard.",
        "You may cast this card from your graveyard, but not from anywhere else.",
    ),
    (
        "You may look at the top card of your library any time.",
        "You may look at face-down creatures you don't control any time.",
        "You may look at the top card of your library and at face-down creatures you don't control any time.",
    ),
    (
        "Equipped creature gets +2/+2 has haste can't attack you or planeswalkers you control.",
        "Equipped creature gets +2/+2 has haste can't be sacrificed.",
        "Equipped creature gets +2/+2, has haste, can't attack you or planeswalkers you control, and can't be sacrificed.",
    ),
    (
        "Creatures with flying can't attack or block.",
        "Creatures with flying activated abilities with {T} in their costs can't be activated.",
        "Creatures with flying can't attack or block, and their activated abilities with {T} in their costs can't be activated.",
    ),
    (
        "Enchanted creature can't attack or block.",
        "Enchanted creature has \"{7}: This enchantment's controller sacrifices it and draws a card. Activate only as a sorcery.\"",
        "Enchanted creature can't attack or block and has \"{7}: Hold for Ransom's controller sacrifices it and draws a card. Activate only as a sorcery.\"",
    ),
    (
        "Creatures with flying can't attack you.",
        "Creatures with flying can't block creatures you control.",
        "Creatures with flying can't attack you or block creatures you control.",
    ),
    (
        "Enchanted permanent gets -1/-1 as long as enchanted permanent is a creature.",
        "As long as enchanted permanent is a creature, enchanted permanent can't block.",
        "As long as enchanted permanent is a creature, it gets -1/-1 and can't block.",
    ),
    (
        "During your turn, this creature has trample, lifelink, and ward {2}.",
        "During your turn, enchantment creatures you control have trample, lifelink, and ward {2}.",
        "During your turn, Yuna and enchantment creatures you control have trample, lifelink, and ward {2}.",
    ),
    (
        "This creature gets +2/+2 as long as you control your commander.",
        "As long as you control your commander, other creatures you control get +2/+2 and have trample.",
        "Lieutenant \u{2014} As long as you control your commander, this creature gets +2/+2 and other creatures you control get +2/+2 and have trample.",
    ),
    (
        "This gets +2/+2 as long as you control your commander.",
        "As long as you control your commander, other creatures you control get +2/+2 and have trample.",
        "Lieutenant \u{2014} As long as you control your commander, this creature gets +2/+2 and other creatures you control get +2/+2 and have trample.",
    ),
    (
        "You have no maximum hand size.",
        "You don't lose the game for having 0 or less life.",
        "You have no maximum hand size and don't lose the game for having 0 or less life.",
    ),
    (
        "You have hexproof.",
        "Humans you control have hexproof.",
        "You and Humans you control have hexproof.",
    ),
    (
        "Enchanted creature has phasing can't be blocked except by walls.",
        "This Aura can't be blocked except by walls.",
        "Enchanted creature has phasing and can't be blocked except by Walls.",
    ),
    (
        "It's a human citizen with base power and toughness 1/1 can't be blocked as long as this creature is tapped.",
        "This creature can't be blocked as long as this creature is tapped.",
        "As long as this creature is tapped, it's a Human Citizen with base power and toughness 1/1 and can't be blocked.",
    ),
    // Gift keywords carry their promised-gift trigger implicitly; the
    // explicit machinery line folds back into the keyword.
    (
        "Gift an Octopus.",
        "When this creature enters, if the gift was promised, the chosen player creates a 8/8 blue Octopus creature token.",
        "Gift an Octopus",
    ),
    (
        "You may reveal the first card you draw each turn as you draw it.",
        "Whenever you reveal instant or sorcery card this way, copy it. You may cast the copy. That copy costs {2} less to cast.",
        "You may reveal the first card you draw each turn as you draw it. Whenever you reveal an instant or sorcery card this way, copy that card and you may cast the copy. That copy costs {2} less to cast.",
    ),
    // Heliod's Punishment joins three lines; the surrounding fixpoint loop
    // chains these two entries.
    (
        "Enchanted creature loses all abilities.",
        "Enchanted creature has \"{T}: Remove a task counter from this. Then if it has no task counters on it, destroy heliod's punishment.\"",
        "It loses all abilities and has \"{T}: Remove a task counter from Heliod's Punishment. Then if it has no task counters on it, destroy Heliod's Punishment.\"",
    ),
    (
        "Enchanted creature can't attack or block.",
        "It loses all abilities and has \"{T}: Remove a task counter from Heliod's Punishment. Then if it has no task counters on it, destroy Heliod's Punishment.\"",
        "Enchanted creature can't attack or block. It loses all abilities and has \"{T}: Remove a task counter from Heliod's Punishment. Then if it has no task counters on it, destroy Heliod's Punishment.\"",
    ),
    (
        "When Lagrella enters, exile any number of other target creatures controlled by different players until Lagrella leaves the battlefield.",
        "When a card exiled with this creature enters under your control, put two +1/+1 counters on it.",
        "When Lagrella enters, exile any number of other target creatures controlled by different players until Lagrella leaves the battlefield. When an exiled card enters under your control this way, put two +1/+1 counters on it.",
    ),
];

fn merge_adjacent_joined_sentence_lines(lines: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    while idx < lines.len() {
        if idx + 1 < lines.len()
            && let Some((_, _, joined)) = ADJACENT_LINE_JOINS.iter().find(|(first, second, _)| {
                lines[idx].trim() == *first && lines[idx + 1].trim() == *second
            })
        {
            out.push((*joined).to_string());
            idx += 2;
            continue;
        }
        // "{S} can't play lands." + "{S} can't cast {spells}." re-join into
        // the single authored restriction "{S} can't play lands or cast
        // {spells}." (Damping Engine)
        if idx + 1 < lines.len()
            && let Some(subject) = lines[idx].trim().strip_suffix(" can't play lands.")
            && let Some(spells) = lines[idx + 1]
                .trim()
                .strip_prefix(subject)
                .and_then(|rest| rest.strip_prefix(" can't cast "))
                .and_then(|rest| rest.strip_suffix('.'))
            && !subject.is_empty()
            && !spells.contains('.')
        {
            out.push(format!("{subject} can't play lands or cast {spells}."));
            idx += 2;
            continue;
        }
        // "If damage would be dealt to this creature, prevent that damage.
        // When damage is prevented this way, ..." (Phyrexian Vindicator): the
        // reflexive trigger belongs to the prevention's own line.
        if idx + 1 < lines.len()
            && let Some(subject) = lines[idx]
                .trim()
                .strip_prefix("Prevent all damage that would be dealt to ")
                .and_then(|rest| rest.strip_suffix('.'))
            && subject.starts_with("this ")
            && subject.matches(' ').count() == 1
            && lines[idx + 1]
                .trim()
                .starts_with("When damage is prevented this way, ")
        {
            out.push(format!(
                "If damage would be dealt to {subject}, prevent that damage. {}",
                lines[idx + 1].trim()
            ));
            idx += 2;
            continue;
        }
        out.push(lines[idx].clone());
        idx += 1;
    }
    out
}

/// Two adjacent color-restriction statics re-join into oracle's single
/// authored sentence: "White creatures can't block." + "Blue creatures can't
/// block." => "White creatures and blue creatures can't block."
fn merge_adjacent_color_cant_block_lines(lines: Vec<String>) -> Vec<String> {
    fn color_cant_block_subject(line: &str) -> Option<&str> {
        let rest = line.trim().strip_suffix("creatures can't block.")?;
        let color = rest.trim();
        matches!(
            color.to_ascii_lowercase().as_str(),
            "white" | "blue" | "black" | "red" | "green"
        )
        .then_some(color)
    }

    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    while idx < lines.len() {
        if idx + 1 < lines.len()
            && let Some(first) = color_cant_block_subject(&lines[idx])
            && let Some(second) = color_cant_block_subject(&lines[idx + 1])
        {
            out.push(format!(
                "{first} creatures and {} creatures can't block.",
                second.to_ascii_lowercase()
            ));
            idx += 2;
            continue;
        }
        out.push(lines[idx].clone());
        idx += 1;
    }
    out
}

fn normalize_exact_during_your_turn_predicate_surface(line: &str) -> String {
    let trimmed = line.trim();
    let without_period = trimmed.trim_end_matches('.');
    if without_period.contains(". ") {
        return line.to_string();
    }
    // A conditional permission static ("You may play cards exiled with The
    // Temporal Anchor as long as it's your turn") leads with its duration as
    // printed: "During your turn, you may ...".
    if let Some(permission) = without_period
        .strip_prefix("You may ")
        .and_then(|rest| rest.strip_suffix(" as long as it's your turn"))
        && !permission.contains(" as long as ")
        && !permission.contains(" during ")
        && !permission.contains(", ")
    {
        return format!("During your turn, you may {permission}.");
    }
    let Some((subject, verb, predicate)) = split_subject_predicate_clause(without_period) else {
        return line.to_string();
    };
    let Some(predicate) = predicate.trim().strip_suffix(" as long as it's your turn") else {
        return line.to_string();
    };
    if predicate.contains(" as long as ") || predicate.contains(" during ") {
        return line.to_string();
    }

    let normalized_predicate = match verb {
        "gets" | "get" => {
            if !predicate.starts_with('+') && !predicate.starts_with('-') {
                return line.to_string();
            }
            predicate.to_string()
        }
        "has" | "have" | "gains" | "gain" => {
            let normalized = normalize_keyword_predicate_case(predicate);
            if normalized == predicate && !is_keyword_phrase(predicate) {
                return line.to_string();
            }
            normalized
        }
        _ => return line.to_string(),
    };
    let surface_verb = if matches!(verb, "gains" | "gain") {
        have_verb_for_subject(subject)
    } else {
        verb
    };
    let (surface_subject, surface_verb) = during_your_turn_subject_and_verb(subject, surface_verb);
    format!("During your turn, {surface_subject} {surface_verb} {normalized_predicate}")
}

fn normalize_sacrifice_enchantment_counter_spell_trigger(line: &str) -> String {
    let trimmed = line.trim().trim_end_matches('.');
    let Some(body) = trimmed.strip_suffix(", sacrifice this enchantment. Counter it") else {
        return line.to_string();
    };
    let Some(body) = body
        .strip_prefix("Whenever ")
        .or_else(|| body.strip_prefix("When "))
    else {
        return line.to_string();
    };
    if !body.contains(" casts a spell") {
        return line.to_string();
    }
    format!("When {body}, sacrifice this enchantment and counter that spell")
}

fn normalize_self_exile_attacking_nonflying_creature_surface(line: &str) -> String {
    let tail = " and target creature without flying that's attacking you";
    let Some(tail_start) = line.find(tail) else {
        return line.to_string();
    };
    let Some(exile_start) = line[..tail_start].rfind("Exile ") else {
        return line.to_string();
    };
    let subject = line[exile_start + "Exile ".len()..tail_start].trim();
    if subject.is_empty()
        || subject.starts_with("this ")
        || subject.starts_with("target ")
        || subject.contains(',')
        || subject.contains(" and ")
    {
        return line.to_string();
    }

    let mut normalized = String::with_capacity(line.len());
    normalized.push_str(&line[..exile_start]);
    normalized.push_str("Exile this creature");
    normalized.push_str(&line[tail_start..]);
    normalized
}

fn normalize_spellcast_trigger_copy_spell_surface(line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    if !(lower.starts_with("when ") || lower.starts_with("whenever ")) {
        return line.to_string();
    }
    if !lower.contains(" cast") || !lower.contains(" spell") {
        return line.to_string();
    }
    let normalized = line
        .replace(
            "an Assassin, Mercenary, Pirate, Rogue, or Warlock spell",
            "an outlaw spell",
        )
        .replace(
            "a Assassin, Mercenary, Pirate, Rogue, or Warlock spell",
            "an outlaw spell",
        )
        .replace(
            "Assassin, Mercenary, Pirate, Rogue, or Warlock spell",
            "outlaw spell",
        );
    let normalized_lower = normalized.to_ascii_lowercase();
    let Some(copy_start) = normalized_lower.find("copy that spell or ability") else {
        return normalized;
    };
    let trigger_prefix = &normalized_lower[..copy_start];
    if trigger_prefix.contains("ability")
        || trigger_prefix.contains("activate")
        || (trigger_prefix.contains("targets only this creature")
            && !trigger_prefix.contains("if you do,")
            && normalized_lower[copy_start..].contains("you may choose new targets for the copy"))
    {
        return normalized;
    }
    normalized
}

fn normalize_basic_land_type_choice_surface(line: &str) -> String {
    line.replace(
        "Choose a basic land type. Target land you control becomes that type until end of turn",
        "Target land you control becomes the basic land type of your choice until end of turn",
    )
    .replace(
        "Choose a basic land type. Target land becomes that type until end of turn",
        "Target land becomes the basic land type of your choice until end of turn",
    )
}

fn normalize_choose_sacrifice_rest_surface(line: &str) -> String {
    let mut normalized = line.to_string();
    for marker in [
        " that player controls on the battlefield. Sacrifice all other ",
        " that player controls on the battlefield, then that player sacrifices all other ",
        " that player controls. Sacrifice all other ",
        " that player controls, then that player sacrifices all other ",
    ] {
        normalized = compact_choose_sacrifice_rest_surface(&normalized, marker);
    }
    normalized
}

fn compact_choose_sacrifice_rest_surface(line: &str, marker: &str) -> String {
    let Some(choose_idx) = line.find(" chooses ") else {
        return line.to_string();
    };
    let subject = &line[..choose_idx];
    if subject.trim_start().starts_with("For each ") || subject.contains(": For each ") {
        return line.to_string();
    }
    let after_choose = &line[choose_idx + " chooses ".len()..];
    let Some(marker_idx) = after_choose.find(marker) else {
        return line.to_string();
    };
    let chosen = normalize_choose_rest_count(&after_choose[..marker_idx]);
    let after_marker = &after_choose[marker_idx + marker.len()..];
    let Some(control_idx) = after_marker.find(" that player controls") else {
        return line.to_string();
    };
    let suffix = &after_marker[control_idx + " that player controls".len()..];
    format!("{subject} chooses {chosen} they control, then sacrifices the rest{suffix}")
}

fn normalize_choose_rest_count(chosen: &str) -> String {
    chosen
        .replace("up to 1 ", "up to one ")
        .replace("up to 2 ", "up to two ")
        .replace("up to 3 ", "up to three ")
        .replace("up to 4 ", "up to four ")
        .replace("up to 5 ", "up to five ")
        .replace("up to 6 ", "up to six ")
}

fn normalize_for_each_number_surface(line: &str) -> String {
    let mut normalized = line.to_string();
    // "for each the number of <plural noun>" is a doubled count surface;
    // oracle says "for each <singular noun>". Generalized over any counter
    // or object kind by singularizing the plural noun in place.
    const MARKER: &str = "for each the number of ";
    let mut search_from = 0;
    while let Some(rel) = normalized[search_from..].find(MARKER) {
        let start = search_from + rel;
        let noun_start = start + MARKER.len();
        // The counted noun phrase runs to the next connective/preposition.
        let tail = &normalized[noun_start..];
        let end_rel = tail
            .find(" on ")
            .into_iter()
            .chain(tail.find(" in "))
            .chain(tail.find(" you "))
            .chain(tail.find(" you've "))
            .chain(tail.find(" among "))
            .min();
        let Some(end_rel) = end_rel else {
            break;
        };
        let noun = normalized[noun_start..noun_start + end_rel].to_string();
        let singular = singularize_counted_noun(&noun);
        let replacement = format!("for each {singular}");
        normalized.replace_range(start..noun_start + end_rel, &replacement);
        search_from = start + replacement.len();
    }
    normalized
}

fn singularize_counted_noun(noun: &str) -> String {
    // Only the head noun ("counters"/"cards") pluralizes; adjectives before
    // it ("wind", "+1/+1") stay. Strip a trailing plural "s" from the last
    // word, leaving already-singular or irregular forms alone.
    match noun.rsplit_once(' ') {
        Some((head, last)) => {
            let last = last.strip_suffix('s').unwrap_or(last);
            format!("{head} {last}")
        }
        None => noun.strip_suffix('s').unwrap_or(noun).to_string(),
    }
}

/// A where-clause belongs after the duration it qualifies: "gets +X/+0,
/// where X is its power until end of turn" reads as "gets +X/+0 until end of
/// turn, where X is its power".
fn normalize_where_x_before_duration_surface(line: &str) -> String {
    let Some(where_start) = line.find(", where X is ") else {
        return line.to_string();
    };
    let clause_end = line[where_start..]
        .find('.')
        .map_or(line.len(), |offset| where_start + offset);
    let clause = &line[where_start..clause_end];
    let Some(where_clause) = clause.strip_suffix(" until end of turn") else {
        return line.to_string();
    };
    format!(
        "{} until end of turn{where_clause}{}",
        &line[..where_start],
        &line[clause_end..]
    )
}

fn normalize_temporary_trample_pump_surface(line: &str) -> String {
    let draw_prefix = "Draw a card, target creature gains trample until end of turn, then it gets ";
    if let Some(rest) = line.strip_prefix(draw_prefix)
        && let Some(pump) = rest
            .trim_end_matches('.')
            .strip_suffix(" until end of turn")
    {
        return format!(
            "Draw a card. Until end of turn, target creature gains trample and gets {pump}"
        );
    }

    let marker = " gains trample until end of turn, then it gets ";
    let Some(marker_start) = line.find(marker) else {
        return line.to_string();
    };
    let subject = &line[..marker_start];
    if !(subject.starts_with("Target ")
        || subject.contains(" target ")
        || subject.contains(": Target "))
    {
        return line.to_string();
    }
    let after_marker = &line[marker_start + marker.len()..];
    let Some((pump, suffix)) = after_marker.split_once(" until end of turn") else {
        return line.to_string();
    };
    // "+X/+0, where X is its power" keeps its where-clause after the
    // duration, as Oracle writes it.
    if let Some((pump, where_x)) = pump.split_once(", where X is ") {
        return format!(
            "{subject} gains trample and gets {pump} until end of turn, where X is {where_x}{suffix}"
        );
    }
    format!("{subject} gains trample and gets {pump} until end of turn{suffix}")
}

fn normalize_chosen_player_adds_mana_surface(line: &str) -> String {
    line.replace(
        "choose a player, then add one mana of any color to that player's mana pool",
        "choose a player. That player adds one mana of any color they choose",
    )
}

fn normalize_role_token_attached_surface(line: &str) -> String {
    let marker = ", create a ";
    let attach_tail = " Role token, then attach it to it";
    let Some(marker_start) = line.find(marker) else {
        return line.to_string();
    };
    let after_marker = &line[marker_start + marker.len()..];
    let Some(role_end) = after_marker.find(attach_tail) else {
        return line.to_string();
    };
    let role = &after_marker[..role_end];
    if role.is_empty() || role.contains('.') {
        return line.to_string();
    }
    let suffix = &after_marker[role_end + attach_tail.len()..];
    format!(
        "{}. Create a {role} Role token attached to it{suffix}",
        &line[..marker_start]
    )
}

fn normalize_create_role_then_attach_surface(line: &str) -> String {
    // "Create a Monster Role token. Attach it to target creature you
    // control." is oracle's one-sentence "Create a Monster Role token
    // attached to target creature you control."
    let marker = " Role token. Attach it to ";
    let Some(idx) = line.find(marker) else {
        return line.to_string();
    };
    let head = &line[..idx];
    if !head.contains("reate a ") || head.contains('.') {
        return line.to_string();
    }
    format!(
        "{head} Role token attached to {}",
        &line[idx + marker.len()..]
    )
}

fn normalize_return_with_counter_surface(line: &str) -> String {
    line.replace(
        ", return it to the battlefield under its owner's control, then put a +1/+1 counter on it",
        ", then return it to the battlefield under its owner's control with a +1/+1 counter on it",
    )
}

fn normalize_simple_token_keyword_surface(line: &str) -> String {
    let mut line = line.replace(" token. It has \"Banding.\"", " token with banding");
    // Parameterized keyword labels (e.g. "Firebending 1") read as inline
    // "with" riders on the token they're stamped on.
    for keyword in ["Firebending"] {
        for amount in 1..=9u32 {
            let lower = keyword.to_lowercase();
            line = line
                .replace(
                    &format!(" token. It has \"{keyword} {amount}.\""),
                    &format!(" token with {lower} {amount}"),
                )
                .replace(
                    &format!(" tokens. They have \"{keyword} {amount}.\""),
                    &format!(" tokens with {lower} {amount}"),
                )
                .replace(
                    &format!(" token. It has \"{keyword} {amount}\""),
                    &format!(" token with {lower} {amount}"),
                )
                .replace(
                    &format!(" tokens. They have \"{keyword} {amount}\""),
                    &format!(" tokens with {lower} {amount}"),
                );
        }
    }
    line
}

fn normalize_chosen_creature_type_surface(line: &str) -> String {
    // Segment-boundary form: the choose effect and its follow-up lower as
    // separate segments joined with ". " — oracle leads with the imperative.
    for (you_form, imperative) in [
        ("You choose a creature type. ", "Choose a creature type. "),
        ("You choose a color. ", "Choose a color. "),
        ("You choose a land type. ", "Choose a land type. "),
    ] {
        if let Some(rest) = line.strip_prefix(you_form) {
            return format!("{imperative}{rest}");
        }
    }
    let Some(rest) = line
        .strip_prefix("You choose a creature type, then ")
        .or_else(|| line.strip_prefix("Choose a creature type, then "))
    else {
        return line.to_string();
    };
    if let Some(effect) = rest.strip_prefix("draw a card for each ") {
        let effect = effect
            .replace(
                " of the chosen type you control",
                " you control of that type",
            )
            .replace(
                " you control of the chosen type",
                " you control of that type",
            );
        return format!("Choose a creature type. Draw a card for each {effect}");
    }
    if let Some(effect) = rest.strip_prefix("gain ") {
        let effect = effect
            .replace(
                " of the chosen type you control",
                " you control of that type",
            )
            .replace(
                " you control of the chosen type",
                " you control of that type",
            );
        return format!("Choose a creature type. You gain {effect}");
    }
    if let Some(effect) = rest.strip_prefix("creatures of the chosen type ") {
        let effect_body = effect.trim_end_matches('.');
        if effect_body == "get -1/-1 until end of turn" {
            return format!("Choose a creature type. All creatures of that type {effect_body}");
        }
        return format!("Creatures of the creature type of your choice {effect}");
    }
    if rest.trim_end_matches('.') == "destroy all creatures of the chosen type" {
        return "Destroy all creatures of the creature type of your choice".to_string();
    }
    if let Some(effect) = rest.strip_prefix("return ") {
        if !effect.contains("target ") {
            return line.to_string();
        }
        // With the leading "Choose a creature type" clause folded away, the
        // chosen-type back-reference reads as the single-sentence oracle
        // idiom "of the creature type of your choice".
        return format!(
            "Return {}",
            effect.replace(
                " of the chosen type",
                " of the creature type of your choice"
            )
        );
    }
    line.to_string()
}

fn normalize_gain_control_untap_pump_haste_surface(line: &str) -> Option<String> {
    let trimmed = line.trim().trim_end_matches('.');
    let prefix = "Gain control of target creature until end of turn, untap it, it gets ";
    let suffix = " until end of turn, then it gains haste until end of turn";
    let pump = trimmed.strip_prefix(prefix)?.strip_suffix(suffix)?;
    (!pump.trim().is_empty()).then(|| {
        format!(
            "Gain control of target creature until end of turn. Untap that creature. Until end of turn, it gets {} and gains haste.",
            pump.trim()
        )
    })
}

fn expand_finalized_ast_surface_line(line: String) -> Vec<String> {
    let trimmed = line.trim().trim_end_matches('.');
    if let Some(first_line) = trimmed.strip_suffix(". Draw a card")
        && first_line.ends_with(". You may shuffle")
        && first_line.contains("put them back in any order")
    {
        return vec![format!("{first_line}."), "Draw a card.".to_string()];
    }
    match trimmed.to_ascii_lowercase().as_str() {
        "skulk, lifelink" => vec!["Skulk".to_string(), "Lifelink".to_string()],
        "skulk, deathtouch" => vec!["Skulk".to_string(), "Deathtouch".to_string()],
        "put a shield counter on target creature. scry 1" => vec![
            "Put a shield counter on target creature.".to_string(),
            "Scry 1.".to_string(),
        ],
        _ => vec![line],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjacent_as_enters_color_and_creature_type_choices_merge() {
        assert_eq!(
            merge_as_enters_color_and_creature_type_choice_lines(vec![
                "As this artifact enters, choose a color.".to_string(),
                "As this artifact enters, choose a creature type.".to_string(),
            ]),
            vec!["As this artifact enters, choose a color and a creature type"]
        );
    }

    #[test]
    fn punctuated_card_name_does_not_capitalize_inline_damage_verb() {
        assert_eq!(
            normalize_punctuated_card_name_damage_case(
                "Kaboom! Deals damage to target player.".to_string(),
                "Kaboom!",
            ),
            "Kaboom! deals damage to target player."
        );
        assert_eq!(
            normalize_punctuated_card_name_damage_case(
                "Ordinary Name Deals damage to target player.".to_string(),
                "Ordinary Name",
            ),
            "Ordinary Name Deals damage to target player."
        );
    }

    fn poison_program_source_fields(program: &mut crate::resolution::ResolutionProgram) {
        let original = std::mem::take(program);
        *program = original
            .try_map_effects(|effect| {
                Ok::<_, std::convert::Infallible>(poison_effect_source_fields(effect))
            })
            .expect("infallible poison transform");
    }

    fn poison_effects(effects: Vec<crate::Effect>) -> Vec<crate::Effect> {
        effects
            .into_iter()
            .map(poison_effect_source_fields)
            .collect()
    }

    fn poison_effect_source_fields(effect: crate::Effect) -> crate::Effect {
        if let Some(choose_mode) = effect.downcast_ref::<crate::effects::ChooseModeEffect>() {
            let mut choose_mode = choose_mode.clone();
            for mode in &mut choose_mode.modes {
                mode.source_text = "POISON".to_string();
                mode.effects = poison_effects(std::mem::take(&mut mode.effects));
            }
            return crate::Effect::new(choose_mode);
        }
        if let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() {
            let mut conditional = conditional.clone();
            conditional.if_true = poison_effects(std::mem::take(&mut conditional.if_true));
            conditional.if_false = poison_effects(std::mem::take(&mut conditional.if_false));
            return crate::Effect::new(conditional);
        }
        if let Some(if_effect) = effect.downcast_ref::<crate::effects::IfEffect>() {
            let mut if_effect = if_effect.clone();
            if_effect.then = poison_effects(std::mem::take(&mut if_effect.then));
            if_effect.else_ = poison_effects(std::mem::take(&mut if_effect.else_));
            return crate::Effect::new(if_effect);
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            let mut with_id = with_id.clone();
            with_id.effect = Box::new(poison_effect_source_fields(*with_id.effect));
            return crate::Effect::new(with_id);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            let mut tagged = tagged.clone();
            tagged.effect = Box::new(poison_effect_source_fields(*tagged.effect));
            return crate::Effect::new(tagged);
        }
        if let Some(repeat) = effect.downcast_ref::<crate::effects::RepeatProcessEffect>() {
            let mut repeat = repeat.clone();
            repeat.effects = poison_effects(std::mem::take(&mut repeat.effects));
            return crate::Effect::new(repeat);
        }
        if let Some(repeat) = effect.downcast_ref::<crate::effects::RepeatEffectsEffect>() {
            let mut repeat = repeat.clone();
            repeat.effects = poison_effects(std::mem::take(&mut repeat.effects));
            return crate::Effect::new(repeat);
        }
        if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
            let mut sequence = sequence.clone();
            sequence.effects = poison_effects(std::mem::take(&mut sequence.effects));
            return crate::Effect::new(sequence);
        }
        if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect>() {
            let mut may = may.clone();
            may.effects = poison_effects(std::mem::take(&mut may.effects));
            return crate::Effect::new(may);
        }
        if let Some(for_players) = effect.downcast_ref::<crate::effects::ForPlayersEffect>() {
            let mut for_players = for_players.clone();
            for_players.effects = poison_effects(std::mem::take(&mut for_players.effects));
            return crate::Effect::new(for_players);
        }
        effect
    }

    fn poison_card_source_fields(definition: &mut CardDefinition) {
        for optional_cost in &mut definition.optional_costs {
            optional_cost.source_label = "POISON".to_string();
        }
        if let Some(spell_effect) = &mut definition.spell_effect {
            poison_program_source_fields(spell_effect);
        }
        for ability in &mut definition.abilities {
            match &mut ability.kind {
                crate::ability::AbilityKind::Triggered(triggered) => {
                    poison_program_source_fields(&mut triggered.effects);
                }
                crate::ability::AbilityKind::Activated(activated) => {
                    poison_program_source_fields(&mut activated.effects);
                }
                crate::ability::AbilityKind::Static(_) => {}
            }
        }
    }

    fn assert_source_poison_does_not_change_compiled_text(mut definition: CardDefinition) {
        let before = compiled_text_lines(&definition);
        poison_card_source_fields(&mut definition);
        let after = compiled_text_lines(&definition);
        assert_eq!(after, before, "compiled text changed after source poison");
        assert!(
            !after.join("\n").contains("POISON"),
            "compiled text leaked poisoned source fields: {after:?}"
        );
    }

    #[test]
    fn poisoned_source_fields_do_not_affect_representative_compiled_text() {
        let modal = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Source Poison Riot",
        )
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .riot()
        .build();
        assert_source_poison_does_not_change_compiled_text(modal);

        let optional_cost = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Source Poison Kicker",
        )
        .card_types(vec![CardType::Instant])
        .kicker_mana(crate::mana::ManaCost::from_symbols(vec![
            ManaSymbol::Generic(2),
        ]))
        .with_spell_effect(vec![crate::Effect::draw(1)])
        .build();
        assert_source_poison_does_not_change_compiled_text(optional_cost);

        let presentation = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Source Poison Toxic",
        )
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(1, 1))
        .toxic(1)
        .build();
        assert_source_poison_does_not_change_compiled_text(presentation);

        let repeat = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Source Poison Repeat",
        )
        .card_types(vec![CardType::Instant])
        .with_spell_effect(vec![
            crate::Effect::draw(1),
            crate::Effect::new(crate::effects::RepeatProcessPromptEffect::new(
                ironsmith_core::RepeatProcessPromptKind::MayRepeatAnyNumberOfTimes,
            )),
        ])
        .build();
        assert_source_poison_does_not_change_compiled_text(repeat);
    }

    #[test]
    fn presentation_label_prefixes_rendered_trigger_body() {
        let ability = crate::ability::Ability {
            kind: crate::ability::AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: crate::triggers::Trigger::this_enters_battlefield(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    crate::Effect::draw(1),
                ]),
                choices: vec![],
                intervening_if: None,
                presentation_label: Some(crate::ability::PresentationLabel::from_ability_word(
                    "Custom Label",
                )),
            }),
            functional_zones: vec![Zone::Battlefield],
        };
        let definition = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Presentation Label Prefix",
        )
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(1, 1))
        .with_ability(ability)
        .build();

        let rendered = compiled_text_lines(&definition).join(" ");
        assert!(
            rendered.contains("Custom Label"),
            "expected presentation label prefix, got {rendered}"
        );
        assert!(
            rendered.to_ascii_lowercase().contains("draw a card"),
            "presentation label must not replace the rendered trigger body: {rendered}"
        );
    }

    #[test]
    fn post_substitution_compacts_conditional_source_animation_bundle() {
        let lines = compact_post_substitution_surface_lines(vec![
            "Haste".to_string(),
            "this creature source is creature in addition to its other types and has base power and toughness 4/4 and is dragon as long as two or more nonland permanents entered the battlefield under your control this turn.".to_string(),
            "Goddric has flying as long as two or more nonland permanents entered the battlefield under your control this turn.".to_string(),
            "This source has \"{R}: Dragons you control get +1/+0 until end of turn.\" As long as two or more nonland permanents entered the battlefield under your control this turn.".to_string(),
        ]);
        assert_eq!(
            lines,
            vec![
                "Haste".to_string(),
                "Celebration — As long as two or more nonland permanents entered the battlefield under your control this turn, Goddric is a Dragon with base power and toughness 4/4, flying, and \"{R}: Dragons you control get +1/+0 until end of turn.\"".to_string(),
            ]
        );
    }

    #[test]
    fn adjacent_conditional_pump_and_grant_keep_one_shared_condition() {
        let lines = compact_shared_conditional_pump_and_ability_lines(vec![
            "Enchanted creature gets +1/+1 as long as enchanted creature is red.".to_string(),
            "As long as enchanted creature is red, it has double strike.".to_string(),
            "Enchanted creature gets +1/+1 as long as enchanted creature is blue.".to_string(),
            "Enchanted creature has \"Whenever this permanent deals damage to an opponent, you draw a card.\"".to_string(),
        ]);
        assert_eq!(
            lines,
            vec![
                "As long as enchanted creature is red, enchanted creature gets +1/+1 and has double strike.".to_string(),
                "As long as enchanted creature is blue, enchanted creature gets +1/+1 and has \"Whenever this permanent deals damage to an opponent, you draw a card.\"".to_string(),
            ]
        );
    }

    #[test]
    fn hideaway_and_evoke_context_keep_their_narrow_linked_surfaces() {
        assert_eq!(
            compact_shared_conditional_pump_and_ability_lines(vec![
                "Hideaway 4.".to_string(),
                "This creature enters tapped.".to_string(),
                "When this creature leaves the battlefield, put each card exiled with it into its owner's hand."
                    .to_string(),
            ]),
            vec![
                "Hideaway 4".to_string(),
                "This creature enters tapped.".to_string(),
                "When this creature leaves the battlefield, put the exiled card into its owner's hand."
                    .to_string(),
            ]
        );
        assert_eq!(
            compact_shared_conditional_pump_and_ability_lines(vec![
                "When this creature enters, it deals 2 damage to each creature and each player."
                    .to_string(),
                "Evoke {2}{G}{G}.".to_string(),
            ]),
            vec![
                "When this creature enters, it deals 2 damage to each creature and each player."
                    .to_string(),
                "Evoke {2}{G}{G}.".to_string(),
            ]
        );
    }

    #[test]
    fn scored_line_normalizes_late_milled_card_choice_surface() {
        assert_eq!(
            normalize_scored_compiled_line(
                "Return target permanent spell to its owner's hand, Jeskai Revelation deals 4 damage to any target, create two 1/1 white Monk creature tokens with prowess, draw two cards, then gain 4 life."
                    .to_string()
            ),
            "Return target permanent spell to its owner's hand, Jeskai Revelation deals 4 damage to any target, create two 1/1 white Monk creature tokens with prowess, draw two cards, then gain 4 life."
        );
        assert_eq!(
            normalize_scored_compiled_line(
                "Sacrifice this enchantment: Creatures your opponents control get -1/-1 and gain attacks each combat if able until end of turn."
                    .to_string()
            ),
            "Sacrifice this enchantment: Creatures your opponents control get -1/-1 until end of turn. Those creatures attack this turn if able."
        );
        assert_eq!(
            normalize_scored_compiled_line(
                "Exile all cards from their hand. Exile target player's graveyard.".to_string()
            ),
            "Exile all cards from target player's hand and graveyard."
        );
        assert_eq!(
            normalize_scored_compiled_line(
                "This creature enters with X +1/+1 counters on it, where X is the number of another creature or artifact you control."
                    .to_string()
            ),
            "This creature enters with a +1/+1 counter on it for each other creature and/or artifact you control"
        );
        assert_eq!(
            normalize_scored_compiled_line(
                "Choose a creature at random on the battlefield, gain control of it until end of turn, untap it, it gains haste until end of turn, then destroy all other creatures."
                    .to_string()
            ),
            "Choose a creature at random. You gain control of that creature until end of turn. Untap it. It gains haste until end of turn. Then destroy all other creatures."
        );
        assert_eq!(
            normalize_scored_compiled_line(
                "Whenever you cast a spell with mana value 5 or greater, each opponent reveals the top card of their library. Then if a permanent that shares a card type with it was revealed this way, copy that spell, you may choose new targets for the copy, then each opponent draws a card. Otherwise, draw a card."
                    .to_string()
            ),
            "Whenever you cast a spell with mana value 5 or greater, each opponent reveals the top card of their library. If any of those cards shares a card type with that spell, copy that spell, you may choose new targets for the copy, and each opponent draws a card. Otherwise, you draw a card."
        );
    }

    #[test]
    fn scored_line_preserves_every_authored_comma_then_boundary() {
        const ORDERED: &str =
            "Each player draws two cards, then discards three cards, then loses 4 life.";

        assert_eq!(normalize_common_semantic_phrasing(ORDERED), ORDERED);
        assert_eq!(
            merge_ast_surface_lines(vec![ORDERED.to_string()]),
            vec![ORDERED.to_string()]
        );
        assert_eq!(finalize_ast_surface_line(ORDERED.to_string()), ORDERED);
        assert_eq!(
            normalize_ast_surface_lines(vec![ORDERED.to_string()]),
            vec![ORDERED.to_string()]
        );
        assert_eq!(normalize_scored_compiled_line(ORDERED.to_string()), ORDERED);
    }

    #[test]
    fn color_choice_exclusion_is_inferred_from_fixed_chosen_color_mana() {
        let lines = annotate_color_choice_exclusions(vec![
            "This land enters tapped.".to_string(),
            "As this land enters, choose a color.".to_string(),
            "{T}: Add {U} or one mana of the chosen color.".to_string(),
        ]);

        assert_eq!(
            lines[1],
            "As this land enters, choose a color other than blue"
        );
    }

    #[test]
    fn conditional_followup_case_does_not_lower_activation_costs() {
        assert_eq!(
            normalize_conditional_followup_case(
                "{2}, {T}, Put a blood counter on this artifact: Draw a card."
            ),
            "{2}, {T}, Put a blood counter on this artifact: Draw a card."
        );
        assert_eq!(
            normalize_conditional_followup_case(
                "If it's tapped, Put a stun counter on it. Otherwise, Tap it."
            ),
            "If it's tapped, put a stun counter on it. Otherwise, tap it."
        );
    }

    #[test]
    fn final_surface_keeps_it_reference_for_tap_freeze_text() {
        assert_eq!(
            finalize_ast_surface_line(
                "If you roll 10-20, tap it. That permanent doesn't untap during its controller's next untap step"
                    .to_string()
            ),
            "If you roll 10-20, tap it. It doesn't untap during its controller's next untap step."
        );
    }

    #[test]
    fn adjacent_conditional_x_counter_lines_use_additional_counter_surface() {
        let lines = merge_specific_adjacent_surface_lines(vec![
            "This creature enters with X +1/+1 counters on it.".to_string(),
            "This creature enters with X +1/+1 counters on it if x is 5 or more.".to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "This creature enters with X +1/+1 counters on it. If X is 5 or more, it enters with an additional X +1/+1 counters on it."
                    .to_string()
            ]
        );
    }

    #[test]
    fn create_token_text_preserves_multiple_creature_subtypes() {
        let token = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Zombie")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Zombie, Subtype::Employee])
            .color_indicator(crate::color::ColorSet::BLACK)
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();

        assert_eq!(
            compile_effect_list(&[Effect::create_tokens(token, Value::Fixed(1))]),
            "Create a 2/2 black Zombie Employee creature token"
        );
    }

    #[test]
    fn instant_spell_damage_after_counter_keeps_spell_as_source() {
        let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Test Blast")
            .card_types(vec![CardType::Instant])
            .with_spell_effect(vec![
                Effect::counter(crate::ChooseSpec::target_spell()),
                Effect::deal_damage(3, crate::ChooseSpec::target_creature()),
            ])
            .build();

        assert_eq!(
            compiled_text_lines(&definition),
            vec!["Counter target spell and Test Blast deals 3 damage to target creature."]
        );
    }

    fn modal_source_line_probe(spree: bool) -> CardDefinition {
        let modes = vec![
            crate::effect::EffectMode::new("Draw a card", vec![Effect::draw(Value::Fixed(1))]),
            crate::effect::EffectMode::new("Gain 2 life", vec![Effect::gain_life(2)]),
        ];
        let mut modal = crate::effects::ChooseModeEffect::new(
            modes,
            Value::Fixed(1),
            Value::Fixed(if spree { 2 } else { 1 }),
            false,
        );
        if spree {
            modal = modal.with_spree_mana_costs(vec![
                crate::mana::ManaCost::from_symbols(vec![ManaSymbol::Generic(1), ManaSymbol::Blue]),
                crate::mana::ManaCost::from_symbols(vec![ManaSymbol::Generic(2)]),
            ]);
        }
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Modal Source Line Probe")
            .card_types(vec![CardType::Instant])
            .with_spell_effect(vec![Effect::new(modal)])
            .build()
    }

    #[test]
    fn typed_modal_source_group_splits_direct_spree_block() {
        assert_eq!(
            compiled_text_lines(&modal_source_line_probe(true)),
            vec![
                "Spree".to_string(),
                "+ {1}{U} — Draw a card.".to_string(),
                "+ {2} — Gain 2 life.".to_string(),
            ]
        );
    }

    #[test]
    fn typed_modal_source_group_keeps_ordinary_bullet_block() {
        assert_eq!(
            compiled_text_lines(&modal_source_line_probe(false)),
            vec!["Choose one —\n• Draw a card.\n• Gain 2 life.".to_string()]
        );
    }

    #[test]
    fn put_counter_then_add_mana_uses_and_surface() {
        assert_eq!(
            compile_effect_list(&[
                Effect::plus_one_counters(1, crate::ChooseSpec::Source),
                Effect::add_mana(vec![ManaSymbol::Red]),
            ]),
            "Put a +1/+1 counter on this source and add {R}"
        );
    }

    #[test]
    fn dynamic_token_pt_setter_compacts_into_creation_text() {
        let token = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Ooze")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Ooze])
            .color_indicator(crate::color::ColorSet::GREEN)
            .power_toughness(crate::card::PowerToughness::fixed(0, 0))
            .build();
        let created = crate::TagKey::from("created_0");

        assert_eq!(
            compile_effect_list(&[
                Effect::create_tokens(token, crate::Value::X).tag(created.clone()),
                Effect::set_base_power_toughness(
                    crate::Value::X,
                    crate::Value::X,
                    crate::ChooseSpec::Tagged(created),
                    crate::Until::Forever,
                ),
            ]),
            "Create X X/X green Ooze creature tokens"
        );
    }

    #[test]
    fn destroy_all_groups_then_draw_uses_destroyed_this_way_surface() {
        let effects = vec![
            Effect::new(crate::effects::DestroyEffect::with_spec(
                crate::ChooseSpec::all(crate::ObjectFilter::creature()),
            )),
            Effect::new(crate::effects::DestroyEffect::with_spec(
                crate::ChooseSpec::all(crate::ObjectFilter::enchantment()),
            )),
            Effect::new(crate::effects::DrawCardsEffect::you(
                crate::Value::PendingEffectMetric {
                    source: crate::effect::EffectMetricSource::AffectedObjects,
                    metric: crate::effect::EffectMetric::AffectedCount,
                },
            )),
        ];
        let expected = "Destroy all creatures and enchantments. Draw a card for each permanent destroyed this way";

        assert_eq!(compile_effect_list(&effects), expected);
        assert_eq!(
            crate::compiled_text::render_effects::describe_effect_clause_list(&effects).as_deref(),
            Some(expected)
        );
    }

    #[test]
    fn ability_labels_keep_keyword_groups_on_their_own_printed_line() {
        for keywords in ["Reach, trample", "Defender, flying, vigilance"] {
            let exception = "As long as you've scried or surveilled this turn, this creature can attack as though it didn't have defender.";
            let text = format!("{keywords}\n{exception}\n{{3}}{{U}}: Surveil 1.");
            let definition =
                crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Keyword Label Probe")
                    .card_types(vec![CardType::Creature])
                    .parse_text(&text)
                    .expect("the keyword group and defender exception should compile");
            let lines = compiled_text_lines(&definition);
            let labels = ability_surface_texts(&definition);
            let group_size = keywords.split(',').count() + 1;
            assert_eq!(labels.len(), definition.abilities.len());
            assert_eq!(lines.len(), 3);
            assert_eq!(labels[..group_size], vec![lines[0].clone(); group_size]);
            assert_eq!(labels[group_size..], lines[1..]);
        }
    }

    #[test]
    fn ability_labels_name_the_printed_line_behind_each_ability() {
        let text = "Equipped creature gets +1/+1 and has trample and haste.\nEquip {1}{R}";
        let definition =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Cutter Probe")
                .card_types(vec![CardType::Artifact])
                .subtypes(vec![Subtype::Equipment])
                .parse_text(text)
                .expect("the equipment should compile");
        let lines = compiled_text_lines(&definition);
        let labels = ability_surface_texts(&definition);
        assert!(
            definition.abilities.len() > lines.len(),
            "the probe must compile to more abilities than printed lines: {lines:?}"
        );
        assert_eq!(labels.len(), definition.abilities.len());
        for label in &labels {
            assert!(
                lines.contains(label),
                "{label:?} is not a printed line of {lines:?}"
            );
        }
        assert_eq!(labels.last().map(String::as_str), Some("Equip {1}{R}"));
        assert!(
            labels[..labels.len() - 1]
                .iter()
                .all(|label| *label == lines[0]),
            "every static shares the pump sentence: {labels:?}"
        );
    }

    #[test]
    fn ability_labels_skip_reminder_lines_no_ability_owns() {
        let text = "When this Class enters, create a 1/1 blue and red Otter creature token with prowess.\n{3}{U}: Level 2\nWhen this Class becomes level 2, return target instant or sorcery card from your graveyard to your hand.";
        let definition =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Class Probe")
                .card_types(vec![CardType::Enchantment])
                .subtypes(vec![Subtype::Class])
                .parse_text(text)
                .expect("the class should compile");
        let lines = compiled_text_lines(&definition);
        let labels = ability_surface_texts(&definition);
        assert!(
            lines
                .first()
                .is_some_and(|line| line.starts_with("(Gain the next level")),
            "class reminder text leads the printed lines: {lines:?}"
        );
        assert_eq!(labels.len(), definition.abilities.len());
        assert!(
            !labels.iter().any(|label| label.starts_with('(')),
            "no ability owns the reminder line: {labels:?}"
        );
        assert_eq!(labels, lines[1..].to_vec());
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn night_shift_compiled_text_preserves_die_adjustment_and_zombie_employee_token() {
        let text = "After you roll a die, you may pay 1 life. If you do, increase or decrease the result by 1. Do this only once each turn.\nWhenever you roll a 6, create a 2/2 black Zombie Employee creature token.";
        let definition = crate::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Night Shift of the Living Dead",
        )
        .card_types(vec![CardType::Enchantment])
        .parse_text(text)
        .expect("Night Shift should compile");

        assert_eq!(compiled_text_lines(&definition).join("\n"), text);
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn result_conjunction_preserves_safe_and_dependent_specialist_surfaces() {
        let hollow_text = "Flying\nWhenever this creature deals combat damage to a player, you may pay {X}. If you do, that player reveals X cards from their hand and you choose one of them. That player discards that card.";
        let hollow = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Hollow Specter")
            .card_types(vec![CardType::Creature])
            .parse_text(hollow_text)
            .expect("Hollow Specter control should compile");
        assert_eq!(compiled_text_lines(&hollow).join("\n"), hollow_text);

        let moku_text = "Whenever you cast a noncreature spell, you may pay {1}. If you do, Moku gets +2/+1 and creatures you control gain haste until end of turn.";
        let moku = crate::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Moku, Meandering Drummer",
        )
        .card_types(vec![CardType::Creature])
        .parse_text(moku_text)
        .expect("Moku control should compile");
        assert_eq!(
            compiled_text_lines(&moku).join("\n"),
            "Whenever you cast a noncreature spell, you may pay {1}. If you do, this creature gets +2/+1 until end of turn and creatures you control gain haste until end of turn."
        );

        let ulalek_text = "Devoid\nWhenever you cast an Eldrazi spell, you may pay {C}{C}. If you do, copy all spells you control, then copy all other activated and triggered abilities you control. You may choose new targets for the copies.";
        let ulalek =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Ulalek, Fused Atrocity")
                .card_types(vec![CardType::Creature])
                .parse_text(ulalek_text)
                .expect("Ulalek control should compile");
        assert_eq!(
            compiled_text_lines(&ulalek).join("\n"),
            "Devoid\nWhenever you cast an Eldrazi spell, you may pay {C}{C}. If you do, copy all spells you control, then copy all other activated and triggered abilities you control. You may choose new targets for the copies."
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn spellshift_compiled_text_uses_countered_spell_controller_surface() {
        let text = "Counter target instant or sorcery spell. Its controller reveals cards from the top of their library until they reveal an instant or sorcery card. That player may cast that card without paying its mana cost. Then the player shuffles.";
        let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Spellshift")
            .card_types(vec![CardType::Instant])
            .parse_text(text)
            .expect("Spellshift should compile");

        let rendered = compiled_text_lines(&definition).join("\n");
        assert_eq!(
            rendered,
            "Counter target instant spell or sorcery spell. Its controller reveals cards from the top of their library until they reveal an instant or sorcery card. That player may cast it without paying its mana cost. Then shuffle their library."
        );
        assert!(!rendered.contains("that object's controller"));
        assert!(!rendered.contains("target player shuffles"));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn implicit_source_combat_prevention_keeps_prevention_surface() {
        let text = "Whenever this creature becomes blocked, prevent all combat damage that would be dealt by it this turn.";
        let definition =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Ignoble Soldier")
                .card_types(vec![CardType::Creature])
                .parse_text(text)
                .expect("Ignoble Soldier should compile");

        let rendered = compiled_text_lines(&definition).join("\n");
        assert_eq!(rendered, text);
        assert!(!rendered.contains("assigns no combat damage"));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn delayed_exile_at_your_next_end_step_stays_delayed() {
        let text = "Return target creature card from your graveyard to the battlefield. It gains haste. Exile it at the beginning of your next end step.";
        let definition =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Haunted House")
                .card_types(vec![CardType::Artifact])
                .parse_text(text)
                .expect("Haunted House visit text should compile");

        let rendered = compiled_text_lines(&definition).join("\n");
        assert_eq!(
            rendered,
            "Return target creature card from your graveyard to the battlefield. It gains haste. At the beginning of your next end step, exile it."
        );
        assert!(!rendered.contains("then exile it"));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn undiscovered_paradise_compiles_delayed_untap_instruction_exactly() {
        let text = "{T}: Add one mana of any color. During your next untap step, as you untap your permanents, return this land to its owner's hand.";
        let definition =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Undiscovered Paradise")
                .card_types(vec![CardType::Land])
                .parse_text(text)
                .expect("Undiscovered Paradise should compile");

        let rendered = compiled_text_lines(&definition).join("\n");
        assert_eq!(rendered, text);
        assert!(!rendered.contains("Untap a permanent"));

        let debug = format!("{definition:#?}");
        assert!(
            debug.contains("AsPermanentsUntapTrigger")
                && debug.contains("ScheduleDelayedTriggerEffect")
                && debug.contains("ReturnToHandEffect"),
            "expected a source-bound delayed untap instruction, got {debug}"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn next_damage_prevention_exiles_prevented_top_cards_as_follow_up() {
        let text = "{2}, {T}: The next time a source of your choice would deal damage to you this turn, prevent that damage. Exile cards from the top of your library equal to the damage prevented this way.";
        let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Bone Mask")
            .card_types(vec![CardType::Artifact])
            .parse_text(text)
            .expect("Bone Mask prevention text should compile");

        let rendered = compiled_text_lines(&definition).join("\n");
        assert_eq!(rendered, text);

        let debug = format!("{definition:#?}");
        assert!(
            debug.contains("PreventNextTimeDamageEffect")
                && debug.contains("follow_up_effects")
                && debug.contains("ExileTopOfLibraryEffect")
                && debug.contains("EventValue")
                && debug.contains("Amount"),
            "expected a prevented-damage-count exile-top follow-up, got {debug}"
        );
        assert!(
            !debug.contains("ChooseObjectsEffect"),
            "expected direct exile-top follow-up instead of choosing one top card, got {debug}"
        );
    }

    #[test]
    fn conditional_enters_with_counter_uses_adamant_prefix_surface() {
        assert_eq!(
            finalize_ast_surface_line(
                "This creature enters with a +1/+1 counter on it if at least three white mana was spent to cast this spell."
                    .to_string()
            ),
            "Adamant — If at least three white mana was spent to cast this spell, this creature enters with a +1/+1 counter on it."
        );
    }

    #[test]
    fn same_turn_pump_and_keyword_lines_merge_to_during_your_turn_surface() {
        let lines = merge_ast_surface_lines(vec![
            "This creature gets +2/+0 as long as it's your turn.".to_string(),
            "This creature has First strike as long as it's your turn.".to_string(),
        ]);

        assert_eq!(
            lines,
            vec!["During your turn, this creature gets +2/+0 and has first strike".to_string()]
        );
    }

    #[test]
    fn mixed_during_turn_and_as_long_turn_lines_merge_to_during_your_turn_surface() {
        let lines = merge_ast_surface_lines(vec![
            "Equipped creature gets +2/+0 as long as it's your turn.".to_string(),
            "During your turn, equipped creature has first strike.".to_string(),
        ]);

        assert_eq!(
            lines,
            vec!["During your turn, equipped creature gets +2/+0 and has first strike".to_string()]
        );
    }

    #[test]
    fn separate_during_turn_source_and_overlapping_population_statics_stay_separate() {
        let lines = merge_ast_surface_lines(vec![
            "This creature has first strike as long as it's your turn.".to_string(),
            "Creatures you control with +1/+1 counters on them have first strike as long as it's your turn."
                .to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "This creature has first strike as long as it's your turn.".to_string(),
                "Creatures you control with +1/+1 counters on them have first strike as long as it's your turn."
                    .to_string(),
            ]
        );
    }

    #[test]
    fn one_during_turn_source_and_other_population_union_still_rejoins() {
        let lines = merge_ast_surface_lines(vec![
            "This creature has flying as long as it's your turn.".to_string(),
            "Other Knights you control have flying as long as it's your turn.".to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "During your turn, this creature and other Knights you control have flying."
                    .to_string()
            ]
        );
    }

    #[test]
    fn attached_object_conditions_merge_across_equivalent_surfaces() {
        let lines = merge_ast_surface_lines(vec![
            "Enchanted creature gets +1/+1 as long as enchanted creature is blue.".to_string(),
            "As long as the permanent this source is attached to is blue, enchanted creature can't be blocked."
                .to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "As long as enchanted creature is blue, enchanted creature gets +1/+1 and can't be blocked"
                    .to_string()
            ]
        );
    }

    #[test]
    fn equipped_keyword_and_conditional_pt_bonus_keep_separate_lines() {
        let lines = merge_ast_surface_lines(vec![
            "Equipped creature has first strike.".to_string(),
            "Equipped creature gets +1/+1 as long as equipped creature is a human.".to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "Equipped creature has first strike.".to_string(),
                "Equipped creature gets +1/+1 as long as equipped creature is a human.".to_string(),
            ]
        );
    }

    #[test]
    fn each_creature_turn_pump_and_keyword_merge_to_plural_subject() {
        let lines = merge_ast_surface_lines(vec![
            "Each creature you control gets +1/+0 as long as it's your turn.".to_string(),
            "Creatures you control have Trample as long as it's your turn.".to_string(),
        ]);

        assert_eq!(
            lines,
            vec!["During your turn, creatures you control get +1/+0 and have trample".to_string()]
        );
    }

    #[test]
    fn exact_turn_conditioned_pump_uses_during_your_turn_surface() {
        assert_eq!(
            finalize_ast_surface_line(
                "Each creature you control gets +2/+0 as long as it's your turn".to_string()
            ),
            "During your turn, creatures you control get +2/+0."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "This creature gets +2/+2 as long as it's your turn".to_string()
            ),
            "During your turn, this creature gets +2/+2."
        );
    }

    #[test]
    fn matching_cast_and_activation_restrictions_merge() {
        let lines = merge_specific_adjacent_surface_lines(vec![
            "During your turn, Your opponents can't cast spells.".to_string(),
            "During your turn, your opponents can't activate abilities of artifacts creatures or enchantments."
                .to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "During your turn, your opponents can't cast spells or activate abilities of artifacts, creatures, or enchantments"
                    .to_string()
            ]
        );
    }

    #[test]
    fn sacrifice_enchantment_counter_spell_trigger_uses_single_when_clause() {
        assert_eq!(
            finalize_ast_surface_line(
                "Whenever an opponent casts a spell, sacrifice this enchantment. Counter it"
                    .to_string()
            ),
            "When an opponent casts a spell, sacrifice this enchantment and counter that spell."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "When a player casts a spell, sacrifice this enchantment. Counter it".to_string()
            ),
            "When a player casts a spell, sacrifice this enchantment and counter that spell."
        );
    }

    #[test]
    fn self_exile_attacking_nonflying_creature_surface_uses_this_creature() {
        assert_eq!(
            finalize_ast_surface_line(
                "{1}{R}{G}, {T}: Exile Hunting Kavu and target creature without flying that's attacking you"
                    .to_string()
            ),
            "{1}{R}{G}, {T}: Exile this creature and target creature without flying that's attacking you."
        );
    }

    #[test]
    fn spellcast_trigger_copy_surface_drops_or_ability() {
        assert_eq!(
            finalize_ast_surface_line(
                "Whenever you cast an Assassin, Mercenary, Pirate, Rogue, or Warlock spell, copy that spell or ability"
                    .to_string()
            ),
            "Whenever you cast an outlaw spell, copy that spell or ability."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "Whenever you cast an instant or sorcery spell that targets only this creature, copy that spell or ability. You may choose new targets for the copy"
                    .to_string()
            ),
            "Whenever you cast an instant or sorcery spell that targets only this creature, copy that spell or ability. You may choose new targets for the copy."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "Whenever you cast an instant or sorcery spell that targets only this creature, you may pay {2}. If you do, copy that spell or ability. You may choose new targets for the copy"
                    .to_string()
            ),
            "Whenever you cast an instant or sorcery spell that targets only this creature, you may pay {2}. If you do, copy that spell or ability. You may choose new targets for the copy."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "When you cast a spell or ability, copy that spell or ability. You may choose new targets for the copy"
                    .to_string()
            ),
            "When you cast a spell or ability, copy that spell or ability. You may choose new targets for the copy."
        );
    }

    #[test]
    fn basic_land_type_choice_surface_uses_of_your_choice() {
        assert_eq!(
            finalize_ast_surface_line(
                "{T}: Choose a basic land type. Target land you control becomes that type until end of turn"
                    .to_string()
            ),
            "{T}: Target land you control becomes the basic land type of your choice until end of turn."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "Choose a basic land type. Target land becomes that type until end of turn. Draw a card"
                    .to_string()
            ),
            "Target land becomes the basic land type of your choice until end of turn. Draw a card."
        );
    }

    #[test]
    fn choose_sacrifice_rest_surface_uses_the_rest() {
        assert_eq!(
            finalize_ast_surface_line(
                "Each player chooses three permanents that player controls on the battlefield. Sacrifice all other permanents that player controls"
                    .to_string()
            ),
            "Each player chooses three permanents they control, then sacrifices the rest."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "Each player chooses a creature or planeswalker that player controls on the battlefield. Sacrifice all other creatures or planeswalkers that player controls. Players can't cast creature or planeswalker spells until the end of your next turn"
                    .to_string()
            ),
            "Each player chooses a creature or planeswalker they control, then sacrifices the rest. Players can't cast creature or planeswalker spells until the end of your next turn."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "At the beginning of each opponent's end step, that player chooses up to 2 creatures that player controls on the battlefield, then that player sacrifices all other creatures that player controls"
                    .to_string()
            ),
            "At the beginning of each opponent's end step, that player chooses up to two creatures they control, then sacrifices the rest."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "At the beginning of each opponent's end step, that player chooses up to 2 creatures that player controls, then that player sacrifices all other creatures that player controls"
                    .to_string()
            ),
            "At the beginning of each opponent's end step, that player chooses up to two creatures they control, then sacrifices the rest."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "−9: For each opponent, that player chooses a permanent that player controls on the battlefield. Sacrifice all other permanents that player controls"
                    .to_string()
            ),
            "−9: For each opponent, that player chooses a permanent that player controls on the battlefield. Sacrifice all other permanents that player controls."
        );
    }

    #[test]
    fn temporary_trample_pump_surface_merges_until_end_of_turn() {
        assert_eq!(
            finalize_ast_surface_line(
                "Draw a card, target creature gains trample until end of turn, then it gets +1/+0 for each the number of cards you've drawn this turn until end of turn."
                    .to_string()
            ),
            "Draw a card. Until end of turn, target creature gains trample and gets +1/+0 for each card you've drawn this turn."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "Target creature gains trample until end of turn, then it gets +X/+X until end of turn, where X is the number of attacking creatures"
                    .to_string()
            ),
            "Target creature gains trample and gets +X/+X until end of turn, where X is the number of attacking creatures."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "At the beginning of combat on your turn, target Elf you control gains trample until end of turn, then it gets +X/+X until end of turn, where X is the number of Forests you control"
                    .to_string()
            ),
            "At the beginning of combat on your turn, target Elf you control gains trample and gets +X/+X until end of turn, where X is the number of Forests you control."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "−5: Target creature gains trample until end of turn, then it gets +X/+X until end of turn, where X is the number of lands you control"
                    .to_string()
            ),
            "−5: Target creature gains trample and gets +X/+X until end of turn, where X is the number of lands you control."
        );
    }

    #[test]
    fn chosen_player_mana_surface_uses_that_player_chooses() {
        assert_eq!(
            finalize_ast_surface_line(
                "{T}: You choose a player, then add one mana of any color to that player's mana pool"
                    .to_string()
            ),
            "{T}: You choose a player. That player adds one mana of any color they choose."
        );
        // Honest surfaces: the "That player adds ... they choose" rewrite
        // moved the color choice to the chosen player, a claim the render
        // never made (deleted as score laundering).
        assert_eq!(
            finalize_ast_surface_line(
                "When this creature enters, you choose a player, then add two mana of any one color to that player's mana pool"
                    .to_string()
            ),
            "When this creature enters, you choose a player, then add two mana of any one color to that player's mana pool."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "When this creature enters, choose a player, then add two mana of any one color to that player's mana pool"
                    .to_string()
            ),
            "When this creature enters, choose a player, then add two mana of any one color to that player's mana pool."
        );
    }

    #[test]
    fn role_token_and_return_with_counter_surfaces_compact() {
        assert_eq!(
            finalize_ast_surface_line(
                "Target creature gets +2/+0 until end of turn, create a Monster Role token, then attach it to it"
                    .to_string()
            ),
            "Target creature gets +2/+0 until end of turn. Create a Monster Role token attached to it."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "Exile target artifact or creature, return it to the battlefield under its owner's control, then put a +1/+1 counter on it"
                    .to_string()
            ),
            "Exile target artifact or creature, then return it to the battlefield under its owner's control with a +1/+1 counter on it."
        );
    }

    #[test]
    fn simple_token_keyword_surface_uses_with_keyword() {
        assert_eq!(
            finalize_ast_surface_line(
                "Create a 1/1 white Knight creature token. It has \"Banding.\"".to_string()
            ),
            "Create a 1/1 white Knight creature token with banding."
        );
    }

    #[test]
    fn chosen_creature_type_surface_uses_of_your_choice() {
        assert_eq!(
            finalize_ast_surface_line(
                "You choose a creature type, then creatures of the chosen type get -3/-3 until end of turn"
                    .to_string()
            ),
            "Creatures of the creature type of your choice get -3/-3 until end of turn."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "Choose a creature type, then creatures of the chosen type get -3/-3 until end of turn"
                    .to_string()
            ),
            "Creatures of the creature type of your choice get -3/-3 until end of turn."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "Choose a creature type, then creatures of the chosen type get +0/+4 until end of turn"
                    .to_string()
            ),
            "Creatures of the creature type of your choice get +0/+4 until end of turn."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "Choose a creature type, then creatures of the chosen type get -1/-1 until end of turn"
                    .to_string()
            ),
            "Choose a creature type. All creatures of that type get -1/-1 until end of turn."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "You choose a creature type, then creatures of the chosen type get -1/-1 until end of turn."
                    .to_string()
            ),
            "Choose a creature type. All creatures of that type get -1/-1 until end of turn."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "You choose a creature type, then return up to three target creature cards of the chosen type from your graveyard to your hand"
                    .to_string()
            ),
            "Return up to three target creature cards of the creature type of your choice from your graveyard to your hand."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "Choose a creature type, then return X target creatures of the chosen type to their owners' hands"
                    .to_string()
            ),
            "Return X target creatures of the creature type of your choice to their owners' hands."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "Choose a creature type, then destroy all creatures of the chosen type".to_string()
            ),
            "Destroy all creatures of the creature type of your choice."
        );
        assert_eq!(
            finalize_ast_surface_line(
                "You choose a creature type, then return all creatures that aren't of the chosen type to their owners' hands"
                    .to_string()
            ),
            "You choose a creature type, then return all creatures that aren't of the chosen type to their owners' hands."
        );
    }

    #[test]
    fn imperative_choice_selection_surfaces_keep_oracle_compactions() {
        assert_eq!(
            finalize_ast_surface_line(
                "Look at the top three cards of your library, choose a card in a hand, in a graveyard, or in exile, choose an other card in a hand, in a graveyard, or in exile, choose an other other card in a hand, in a graveyard, or in exile, return it to its owner's hand, put it on the bottom of its owner's library, exile it, then you may play those cards this turn."
                    .to_string()
            ),
            "Look at the top three cards of your library. Put one of them into your hand, put one of them on the bottom of your library, and exile one of them. You may play the exiled card this turn."
        );
        // Honest surface: the render genuinely loses the hand/graveyard zone
        // split for the two choices; the old expectation was a hand-written
        // gate that re-asserted the zones (deleted as score laundering).
        assert_eq!(
            finalize_ast_surface_line(
                "Target opponent reveals their hand, choose an artifact or creature card, choose an artifact or creature card, then exile it."
                    .to_string()
            ),
            "Target opponent reveals their hand, choose an artifact or creature card, choose an artifact or creature card, then exile it."
        );
    }

    #[test]
    fn target_type_disjunction_keeps_shared_opponent_controller_clause() {
        assert_eq!(
            finalize_ast_surface_line(
                "Destroy target creature an opponent controls or enchantment".to_string()
            ),
            "Destroy target creature or enchantment an opponent controls."
        );
    }

    #[test]
    fn life_loss_mana_value_uses_that_permanent_surface() {
        assert_eq!(
            finalize_ast_surface_line("You lose life equal to its mana value".to_string()),
            "You lose life equal to that permanent's mana value."
        );
    }

    #[test]
    fn skulk_keyword_pairs_keep_oracle_line_breaks() {
        assert_eq!(
            expand_finalized_ast_surface_line("Skulk, lifelink".to_string()),
            vec!["Skulk".to_string(), "Lifelink".to_string()]
        );
        assert_eq!(
            expand_finalized_ast_surface_line("Skulk, deathtouch".to_string()),
            vec!["Skulk".to_string(), "Deathtouch".to_string()]
        );
        assert_eq!(
            expand_finalized_ast_surface_line(
                "Put a shield counter on target creature. Scry 1.".to_string()
            ),
            vec![
                "Put a shield counter on target creature.".to_string(),
                "Scry 1.".to_string(),
            ]
        );
        assert_eq!(
            expand_finalized_ast_surface_line(
                "Look at the top three cards of your library, then put them back in any order. You may shuffle. Draw a card."
                    .to_string()
            ),
            vec![
                "Look at the top three cards of your library, then put them back in any order. You may shuffle."
                    .to_string(),
                "Draw a card.".to_string(),
            ]
        );
    }

    #[test]
    fn token_quote_activation_costs_keep_colon_surface() {
        assert_eq!(
            finalize_ast_surface_line(
                "Create a 1/1 colorless Eldrazi Scion creature token. It has \"Sacrifice this token, add {C}.\""
                    .to_string()
            ),
            "Create a 1/1 colorless Eldrazi Scion creature token. It has \"Sacrifice this token: Add {C}.\""
        );
    }

    #[test]
    fn token_quote_trigger_keeps_its_terminal_period_inside_the_quote() {
        assert_eq!(
            finalize_ast_surface_line(
                "Create a 1/1 black and green Pest creature token with \"When this token dies, you gain 1 life\""
                    .to_string(),
            ),
            "Create a 1/1 black and green Pest creature token with \"When this token dies, you gain 1 life.\""
        );
        assert_eq!(
            finalize_ast_surface_line(
                "Create a 0/1 black Wizard creature token with \"Whenever you cast a noncreature spell, this token deals 1 damage to each opponent\""
                    .to_string(),
            ),
            "Create a 0/1 black Wizard creature token with \"Whenever you cast a noncreature spell, this token deals 1 damage to each opponent.\""
        );
        assert_eq!(
            finalize_ast_surface_line(
                "Create a 1/1 white Bird creature token with \"Flying\"".to_string(),
            ),
            "Create a 1/1 white Bird creature token with \"Flying\"",
            "a keyword fragment must not acquire sentence punctuation"
        );
    }

    #[test]
    fn terminal_quoted_ability_does_not_add_a_second_sentence_period() {
        assert_eq!(
            finalize_ast_surface_line(
                "Until end of turn, this land becomes a creature with \"{X}: This creature gets +X/+0 until end of turn.\". It's still a land"
                    .to_string(),
            ),
            "Until end of turn, this land becomes a creature with \"{X}: This creature gets +X/+0 until end of turn.\" It's still a land."
        );
        assert_eq!(
            remove_redundant_period_after_terminal_quote(
                "It has \"Can you pay?\". This sentence follows."
            ),
            "It has \"Can you pay?\" This sentence follows."
        );
        assert_eq!(
            remove_redundant_period_after_terminal_quote(
                "It has \"Flying\". This period terminates the containing sentence."
            ),
            "It has \"Flying\". This period terminates the containing sentence."
        );
    }

    #[test]
    fn unleash_scaffolding_compacts_to_keyword_surface() {
        assert_eq!(
            merge_ast_surface_lines(vec![
                "First strike, haste".to_string(),
                "When this creature enters, you may put a +1/+1 counter on this creature."
                    .to_string(),
                "This creature can't block as long as it has a +1/+1 counter on it.".to_string(),
            ]),
            vec!["First strike, haste".to_string(), "Unleash".to_string()]
        );
    }

    #[test]
    fn plural_turn_animation_and_granted_trigger_merge_to_bello_surface() {
        let lines = merge_ast_surface_lines(vec![
            "During your turn, non-Equipment artifacts with mana value 4 or greater you control or non-Aura enchantments with mana value 4 or greater you control are creatures in addition to their other types and have base power and toughness 4/4 and are Elementals in addition to their other types and have indestructible and have haste.".to_string(),
            "Non-Equipment artifacts with mana value 4 or greater you control or non-Aura enchantments with mana value 4 or greater you control have \"whenever this creature deals combat damage to a player, draw a card.\" As long as it's your turn.".to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "During your turn, each non-Equipment artifact and non-Aura enchantment you control with mana value 4 or greater is a 4/4 Elemental creature in addition to its other types and has indestructible, haste, and \"Whenever this creature deals combat damage to a player, draw a card.\""
                    .to_string()
            ]
        );

        let split_layer_lines = merge_ast_surface_lines(vec![
            "During your turn, Each non-Equipment artifact with mana value 4 or greater you control or a non-Aura enchantment with mana value 4 or greater you control is a creature in addition to its other types.".to_string(),
            "During your turn, non-Equipment artifacts with mana value 4 or greater you control or non-Aura enchantments with mana value 4 or greater you control have base power and toughness 4/4 and are Elementals in addition to their other types and have indestructible and haste and have \"whenever this creature deals combat damage to a player, draw a card.\"".to_string(),
        ]);

        assert_eq!(
            split_layer_lines,
            vec![
                "During your turn, each non-Equipment artifact and non-Aura enchantment you control with mana value 4 or greater is a 4/4 Elemental creature in addition to its other types and has indestructible, haste, and \"Whenever this creature deals combat damage to a player, draw a card.\""
                    .to_string()
            ]
        );

        let folded_lines = merge_ast_surface_lines(vec![
            "During your turn, non-Equipment artifacts with mana value 4 or greater you control or non-Aura enchantments with mana value 4 or greater you control are creatures in addition to their other types and have base power and base toughness 4/4 and are Elementals in addition to their other types and have indestructible and haste and have \"whenever this creature deals combat damage to a player, draw a card.\".".to_string(),
        ]);

        assert_eq!(
            folded_lines,
            vec![
                "During your turn, each non-Equipment artifact and non-Aura enchantment you control with mana value 4 or greater is a 4/4 Elemental creature in addition to its other types and has indestructible, haste, and \"Whenever this creature deals combat damage to a player, draw a card.\""
                    .to_string()
            ]
        );
    }

    #[test]
    fn repeated_conditional_keyword_grants_use_same_is_true_surface() {
        let lines = merge_ast_surface_lines(vec![
            "At the beginning of each combat, if you control a creature with first strike, creatures you control gain first strike until end of turn.".to_string(),
            "At the beginning of each combat, if you control a creature with flying, creatures you control gain flying until end of turn.".to_string(),
            "At the beginning of each combat, if you control a creature with vigilance, creatures you control gain vigilance until end of turn.".to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "At the beginning of each combat, creatures you control gain first strike until end of turn if a creature you control has first strike. The same is true for flying and vigilance."
                    .to_string()
            ]
        );
    }

    #[test]
    fn repeated_graveyard_keyword_grants_use_same_is_true_surface() {
        let lines = merge_ast_surface_lines(vec![
            "At the beginning of combat on your turn, if there is a creature card with flying in your graveyard, creatures you control gain flying until end of turn.".to_string(),
            "At the beginning of combat on your turn, if there is a creature card with first strike in your graveyard, creatures you control gain first strike until end of turn.".to_string(),
            "At the beginning of combat on your turn, if there is a creature card with vigilance in your graveyard, creatures you control gain vigilance until end of turn.".to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "At the beginning of combat on your turn, creatures you control gain flying until end of turn if a creature card in your graveyard has flying. The same is true for first strike and vigilance."
                    .to_string()
            ]
        );
    }

    #[test]
    fn repeated_type_additions_use_same_is_true_surface() {
        let lines = merge_ast_surface_lines(vec![
            "Creatures you control are the chosen type in addition to their other types."
                .to_string(),
            "Creature spells you control are the chosen type in addition to their other types."
                .to_string(),
            "Creature cards you own that aren't on the battlefield are the chosen type in addition to their other types."
                .to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "Creatures you control are the chosen type in addition to their other types. The same is true for creature spells you control and creature cards you own that aren't on the battlefield."
                    .to_string()
            ]
        );
    }

    #[test]
    fn repeated_creature_subtype_additions_use_same_is_true_surface() {
        let lines = merge_ast_surface_lines(vec![
            "Slivers you control and nontoken creatures you control are the chosen type in addition to their other creature types."
                .to_string(),
            "Creature spells you control are the chosen type in addition to their other creature types."
                .to_string(),
            "Creature cards you own that aren't on the battlefield are the chosen type in addition to their other creature types."
                .to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "Slivers you control and nontoken creatures you control are the chosen type in addition to their other creature types. The same is true for creature spells you control and creature cards you own that aren't on the battlefield."
                    .to_string()
            ]
        );
    }

    #[test]
    fn all_permanent_type_additions_compact_stack_and_owned_zone_subjects() {
        let lines = merge_ast_surface_lines(vec![
            "Nonland permanents you control are artifacts in addition to their other types."
                .to_string(),
            "Artifact, creature, enchantment, land, planeswalker, and battle spells you control are artifacts in addition to their other types."
                .to_string(),
            "Nonland permanent cards in your hand or nonland permanent cards in your library or nonland permanent cards in your graveyard or nonland permanent cards in your exile or nonland permanent cards in your command zone are artifacts in addition to their other types."
                .to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "Nonland permanents you control are artifacts in addition to their other types. The same is true for permanent spells you control and nonland permanent cards you own that aren't on the battlefield."
                    .to_string()
            ]
        );
    }

    #[test]
    fn repeated_color_changes_use_same_is_true_surface() {
        let lines = merge_ast_surface_lines(vec![
            "Nonland permanents you control are white.".to_string(),
            "Spells you control are white.".to_string(),
            "Nonland cards in your hand or nonland cards in your library or nonland cards in your graveyard or nonland cards in your exile or nonland cards in your command zone are white."
                .to_string(),
        ]);

        assert_eq!(
            lines,
            vec![
                "Nonland permanents you control are white. The same is true for spells you control and nonland cards you own that aren't on the battlefield."
                    .to_string()
            ]
        );
    }

    #[test]
    fn global_colorless_domain_does_not_merge_into_permanent_type_line() {
        let mut global = crate::target::ObjectFilter::default();
        global.set_global_characteristic_domain_surface(Some(
            ironsmith_core::GlobalCharacteristicDomainSurface::CardsOutsideBattlefieldSpellsAndPermanents,
        ));
        let definition = crate::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Global Characteristic Probe",
        )
        .card_types(vec![CardType::Artifact])
        .with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::add_card_types(
                crate::target::ObjectFilter::permanent(),
                vec![CardType::Artifact],
            ),
        ))
        .with_ability(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::make_colorless(global),
        ))
        .build();

        assert_eq!(
            compiled_text_lines(&definition),
            vec![
                "All permanents are artifacts in addition to their other types.".to_string(),
                "All cards that aren't on the battlefield, spells, and permanents are colorless."
                    .to_string(),
            ]
        );
    }

    #[test]
    fn during_your_turn_prevent_clause_drops_extra_has() {
        assert_eq!(
            finalize_ast_surface_line(
                "During your turn, this creature has Prevent all damage that would be dealt to this creature."
                    .to_string()
            ),
            "During your turn, prevent all damage that would be dealt to this creature."
        );
    }

    #[test]
    fn gain_control_untap_fixed_pump_haste_uses_oracle_sentence_shape() {
        assert_eq!(
            finalize_ast_surface_line(
                "Gain control of target creature until end of turn, untap it, it gets +2/+0 until end of turn, then it gains haste until end of turn"
                    .to_string()
            ),
            "Gain control of target creature until end of turn. Untap that creature. Until end of turn, it gets +2/+0 and gains haste."
        );
    }

    #[test]
    fn compiled_text_cleanup_layers_reject_known_semantic_rescue_strings() {
        let checked_sources = [
            ("mod.rs", include_str!("mod.rs")),
            ("normalize_common.rs", include_str!("normalize_common.rs")),
            ("debug_safe.rs", include_str!("debug_safe.rs")),
            ("surface_helpers.rs", include_str!("surface_helpers.rs")),
        ];
        let banned = [
            concat!("K", "ain"),
            concat!("allagan", " eye"),
            concat!("Flame", "break"),
            concat!(
                "deals 3 damage to each creature without flying",
                ", deal 3 damage to each player"
            ),
            concat!(
                "Gain control of target creature until end of turn",
                ", untap it, then it gains haste"
            ),
            concat!(
                "Untap target creature, gain control of it until end of turn",
                ", then it gains haste"
            ),
            concat!(
                "You choose the top card in your library",
                ", exile it, then you may play that card"
            ),
            concat!(
                "for each card revealed this way",
                ", unless it's a permanent, put that object"
            ),
        ];

        for (source_name, source) in checked_sources {
            for needle in banned {
                assert!(
                    !source.contains(needle),
                    "{source_name} contains semantic rescue text that belongs in structural rendering: {needle}"
                );
            }
        }
    }
}

#[cfg(test)]
mod rescue_regressions;
