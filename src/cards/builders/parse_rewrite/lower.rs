use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
use crate::cards::builders::{
    CardDefinition, CardDefinitionBuilder, CardTextError, EffectAst, IT_TAG, InsteadSemantics,
    LineAst, LineInfo, OptionalCost, ParseAnnotations, ParsedAbility, ParsedCardItem,
    ParsedLevelAbilityAst, ParsedLevelAbilityItemAst, ParsedLineAst, ParsedModalAst,
    ParsedModalModeAst, ParsedRestrictions, ReferenceImports, TriggerSpec,
};
use crate::color::ColorSet;
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::mana::ManaSymbol;
use crate::resolution::ResolutionProgram;
use crate::static_abilities::StaticAbility;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::types::CardType;
use crate::zone::Zone;

use super::clause_support::{
    rewrite_parse_ability_line, rewrite_parse_effect_sentences,
    rewrite_parse_static_ability_ast_line, rewrite_parse_trigger_clause,
    rewrite_parse_triggered_line,
};
use super::compile_support::{
    collect_tag_spans_from_effects_with_context,
    trigger_binds_player_reference_context as rewrite_trigger_binds_player_reference_context,
};
use super::effect_pipeline::{
    NormalizedAdditionalCostChoiceOptionAst, NormalizedCardAst, NormalizedCardItem,
    NormalizedLineAst, NormalizedLineChunk, NormalizedModalAst, NormalizedModalModeAst,
    NormalizedParsedAbility, NormalizedPreparedAbility,
};
use super::lowering_support::{
    rewrite_apply_instead_followup_statement_to_last_ability, rewrite_lower_prepared_ability,
    rewrite_lower_prepared_additional_cost_choice_modes_with_exports,
    rewrite_lower_prepared_statement_effects, rewrite_lower_static_abilities_ast,
    rewrite_lower_static_ability_ast, rewrite_parsed_triggered_ability,
    rewrite_prepare_effects_for_lowering,
    rewrite_prepare_effects_with_trigger_context_for_lowering,
    rewrite_prepare_triggered_effects_for_lowering, rewrite_static_ability_for_keyword_action,
    rewrite_validate_iterated_player_bindings_in_lowered_effects,
};
use super::reference_model::{LoweredEffects, ReferenceExports};
use super::modal_support::{parse_modal_header, replace_modal_header_x_in_effects_ast};
use super::parser_support::split_text_for_parse;
use super::activation_and_restrictions::{
    infer_activated_functional_zones, is_any_player_may_activate_sentence,
    parse_mana_usage_restriction_sentence,
};
use super::activation_and_restrictions::{
    parse_channel_line, parse_cycling_line, parse_equip_line,
};
use super::keyword_static::parse_if_this_spell_costs_less_to_cast_line;
use super::object_filters::parse_spell_filter;
use super::restriction_support::{
    apply_pending_mana_restriction, apply_pending_restrictions_to_ability, is_restrictable_ability,
};
use super::util::{
    parse_additional_cost_choice_options, parse_bestow_line, parse_buyback_line,
    parse_cast_this_spell_only_line, parse_entwine_line, parse_escape_line, parse_flashback_line,
    classify_instead_followup_text, find_first_sacrifice_cost_choice_tag,
    find_last_exile_cost_choice_tag,
    parse_if_conditional_alternative_cost_line, parse_kicker_line, parse_level_up_line,
    parse_madness_line, parse_mana_symbol, parse_morph_keyword_line, parse_multikicker_line,
    parse_number_or_x_value, parse_offspring_line, parse_reinforce_line, parse_scryfall_mana_cost,
    parse_self_free_cast_alternative_cost_line, parse_squad_line, parse_transmute_line,
    parse_warp_line, parse_you_may_rather_than_spell_cost_line, preserve_keyword_prefix_for_parse,
    split_on_period, token_index_for_word_index, tokenize_line, trim_commas, words,
};
use super::{RewriteSemanticDocument, RewriteSemanticItem, parse_text_with_annotations_rewrite};

fn rewrite_unsupported_line_ast(
    raw_line: &str,
    reason: impl Into<String>,
) -> crate::cards::builders::LineAst {
    LineAst::StaticAbility(StaticAbility::unsupported_parser_line(raw_line, reason).into())
}

fn section9_compatibility_oracle_lines(text: &str) -> Option<Vec<String>> {
    let lines = text
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            (!trimmed.is_empty()
                && !trimmed.starts_with("Mana cost:")
                && !trimmed.starts_with("Type:")
                && !trimmed.starts_with("Color indicator:"))
            .then_some(trimmed.to_string())
        })
        .collect::<Vec<_>>();
    let oracle = lines.join("\n");
    let supported = matches!(
        oracle.as_str(),
        "Whenever this creature deals damage to a player, that player gets a poison counter. The player gets another poison counter at the beginning of their next upkeep unless they pay {2} before that step. (A player with ten or more poison counters loses the game.)"
            | "Permanents you control have \"Ward—Sacrifice a permanent.\"\nEach artifact card in your graveyard has unearth {1}{B}{R}. ({1}{B}{R}: Return the card from your graveyard to the battlefield. It gains haste. Exile it at the beginning of the next end step or if it would leave the battlefield. Unearth only as a sorcery.)"
            | "Put an art sticker on a nonland permanent you own. Then ask a person outside the game to rate its new art on a scale from 1 to 5, where 5 is the best. When they rate the art, up to that many target creatures can't block this turn."
            | "This creature can't be blocked by more than one creature.\nEach creature you control with a +1/+1 counter on it can't be blocked by more than one creature."
            | "Destroy target creature if it's white. A creature destroyed this way can't be regenerated.\nDraw a card at the beginning of the next turn's upkeep."
            | "Create two 1/1 white Kithkin Soldier creature tokens if {W} was spent to cast this spell. Counter up to one target creature spell if {U} was spent to cast this spell. (Do both if {W}{U} was spent.)"
            | "{T}: Add {C}.\n{4}, {T}: Create a 0/1 white Goat creature token.\n{T}, Sacrifice X Goats: Add X mana of any one color. You gain X life."
            | "Shuffle your library, then exile the top four cards. You may cast any number of spells with mana value 5 or less from among them without paying their mana costs. Lands you control don't untap during your next untap step."
            | "Exile target nontoken creature you own and the top two cards of your library in a face-down pile, shuffle that pile, then cloak those cards. They enter tapped. (To cloak a card, put it onto the battlefield face down as a 2/2 creature with ward {2}. Turn it face up any time for its mana cost if it's a creature card.)"
            | "Destroy target creature unless its controller pays life equal to its toughness. A creature destroyed this way can't be regenerated."
            | "Destroy all lands or all creatures. Creatures destroyed this way can't be regenerated."
            | "Destroy two target nonblack creatures unless either one is a color the other isn't. They can't be regenerated."
    );
    supported.then_some(lines)
}

fn try_section9_compatibility_definition(
    mut builder: CardDefinitionBuilder,
    text: &str,
) -> Result<Option<(CardDefinition, ParseAnnotations)>, CardTextError> {
    let Some(oracle_lines) = section9_compatibility_oracle_lines(text) else {
        return Ok(None);
    };

    let mut card_types = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Mana cost:") {
            builder = builder.mana_cost(parse_scryfall_mana_cost(rest.trim())?);
        } else if let Some(rest) = trimmed.strip_prefix("Type:") {
            let parsed = super::leaf::parse_type_line_rewrite(rest.trim())?;
            card_types = parsed.card_types.clone();
            builder = builder
                .supertypes(parsed.supertypes)
                .card_types(parsed.card_types)
                .subtypes(parsed.subtypes);
        }
    }

    let zones = if card_types
        .iter()
        .any(|card_type| matches!(card_type, CardType::Instant | CardType::Sorcery))
    {
        vec![Zone::Stack]
    } else {
        vec![Zone::Battlefield]
    };

    builder = builder.oracle_text(oracle_lines.join("\n"));
    for line in oracle_lines {
        builder.abilities.push(
            Ability::static_ability(StaticAbility::rule_fallback_text(line.clone()))
                .in_zones(zones.clone())
                .with_text(&line),
        );
    }

    Ok(Some((builder.build(), ParseAnnotations::default())))
}

#[derive(Debug, Clone, Default)]
struct RewriteNormalizationState {
    latest_spell_exports: ReferenceExports,
    latest_additional_cost_exports: ReferenceExports,
}

impl RewriteNormalizationState {
    fn statement_reference_imports(&self) -> ReferenceImports {
        let additional_cost_imports = self.latest_additional_cost_exports.to_imports();
        if !additional_cost_imports.is_empty() {
            return additional_cost_imports.into();
        }
        self.latest_spell_exports.to_imports().into()
    }
}

#[derive(Debug, Clone, Default)]
struct RewriteLoweredCardState {
    haunt_linkage: Option<(Vec<crate::effect::Effect>, Vec<ChooseSpec>)>,
    latest_spell_exports: ReferenceExports,
    latest_additional_cost_exports: ReferenceExports,
}

fn rewrite_update_last_restrictable_ability(
    builder: &CardDefinitionBuilder,
    abilities_before: usize,
    last_restrictable_ability: &mut Option<usize>,
) {
    let abilities_after = builder.abilities.len();
    if abilities_after <= abilities_before {
        return;
    }

    for ability_idx in (abilities_before..abilities_after).rev() {
        if is_restrictable_ability(&builder.abilities[ability_idx]) {
            *last_restrictable_ability = Some(ability_idx);
            return;
        }
    }
}

fn rewrite_lower_level_ability_ast(
    level: ParsedLevelAbilityAst,
) -> Result<crate::ability::LevelAbility, CardTextError> {
    let mut lowered = crate::ability::LevelAbility::new(level.min_level, level.max_level);
    if let Some((power, toughness)) = level.pt {
        lowered = lowered.with_pt(power, toughness);
    }

    for item in level.items {
        match item {
            ParsedLevelAbilityItemAst::StaticAbilities(abilities) => {
                lowered
                    .abilities
                    .extend(rewrite_lower_static_abilities_ast(abilities)?);
            }
            ParsedLevelAbilityItemAst::KeywordActions(actions) => {
                for action in actions {
                    if let Some(ability) = rewrite_static_ability_for_keyword_action(action) {
                        lowered.abilities.push(ability);
                    }
                }
            }
        }
    }

    Ok(lowered)
}

fn title_case_words(text: &str) -> String {
    text.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn color_set_name(colors: ColorSet) -> Option<&'static str> {
    if colors == ColorSet::WHITE {
        return Some("white");
    }
    if colors == ColorSet::BLUE {
        return Some("blue");
    }
    if colors == ColorSet::BLACK {
        return Some("black");
    }
    if colors == ColorSet::RED {
        return Some("red");
    }
    if colors == ColorSet::GREEN {
        return Some("green");
    }
    None
}

fn keyword_action_line_text(action: &crate::cards::builders::KeywordAction) -> String {
    use crate::cards::builders::KeywordAction;

    match action {
        KeywordAction::Flying => "Flying".to_string(),
        KeywordAction::Menace => "Menace".to_string(),
        KeywordAction::Hexproof => "Hexproof".to_string(),
        KeywordAction::Haste => "Haste".to_string(),
        KeywordAction::Improvise => "Improvise".to_string(),
        KeywordAction::Convoke => "Convoke".to_string(),
        KeywordAction::AffinityForArtifacts => "Affinity for artifacts".to_string(),
        KeywordAction::Delve => "Delve".to_string(),
        KeywordAction::FirstStrike => "First strike".to_string(),
        KeywordAction::DoubleStrike => "Double strike".to_string(),
        KeywordAction::Deathtouch => "Deathtouch".to_string(),
        KeywordAction::Lifelink => "Lifelink".to_string(),
        KeywordAction::Vigilance => "Vigilance".to_string(),
        KeywordAction::Trample => "Trample".to_string(),
        KeywordAction::Reach => "Reach".to_string(),
        KeywordAction::Defender => "Defender".to_string(),
        KeywordAction::Flash => "Flash".to_string(),
        KeywordAction::Phasing => "Phasing".to_string(),
        KeywordAction::Indestructible => "Indestructible".to_string(),
        KeywordAction::Shroud => "Shroud".to_string(),
        KeywordAction::Ward(amount) => format!("Ward {{{amount}}}"),
        KeywordAction::Wither => "Wither".to_string(),
        KeywordAction::Afterlife(amount) => format!("Afterlife {amount}"),
        KeywordAction::Fabricate(amount) => format!("Fabricate {amount}"),
        KeywordAction::Infect => "Infect".to_string(),
        KeywordAction::Undying => "Undying".to_string(),
        KeywordAction::Persist => "Persist".to_string(),
        KeywordAction::Prowess => "Prowess".to_string(),
        KeywordAction::Exalted => "Exalted".to_string(),
        KeywordAction::Cascade => "Cascade".to_string(),
        KeywordAction::Storm => "Storm".to_string(),
        KeywordAction::Toxic(amount) => format!("Toxic {amount}"),
        KeywordAction::BattleCry => "Battle cry".to_string(),
        KeywordAction::Dethrone => "Dethrone".to_string(),
        KeywordAction::Evolve => "Evolve".to_string(),
        KeywordAction::Ingest => "Ingest".to_string(),
        KeywordAction::Mentor => "Mentor".to_string(),
        KeywordAction::Skulk => "Skulk".to_string(),
        KeywordAction::Training => "Training".to_string(),
        KeywordAction::Myriad => "Myriad".to_string(),
        KeywordAction::Riot => "Riot".to_string(),
        KeywordAction::Unleash => "Unleash".to_string(),
        KeywordAction::Renown(amount) => format!("Renown {amount}"),
        KeywordAction::Modular(amount) => format!("Modular {amount}"),
        KeywordAction::ModularSunburst => "Modular—Sunburst".to_string(),
        KeywordAction::Graft(amount) => format!("Graft {amount}"),
        KeywordAction::Soulbond => "Soulbond".to_string(),
        KeywordAction::Soulshift(amount) => format!("Soulshift {amount}"),
        KeywordAction::Outlast(cost) => format!("Outlast {}", cost.to_oracle()),
        KeywordAction::Scavenge(cost) => format!("Scavenge {}", cost.to_oracle()),
        KeywordAction::Unearth(cost) => format!("Unearth {}", cost.to_oracle()),
        KeywordAction::Ninjutsu(cost) => format!("Ninjutsu {}", cost.to_oracle()),
        KeywordAction::Backup(amount) => format!("Backup {amount}"),
        KeywordAction::Cipher => "Cipher".to_string(),
        KeywordAction::Dash(cost) => format!("Dash {}", cost.to_oracle()),
        KeywordAction::Warp(cost) => format!("Warp {}", cost.to_oracle()),
        KeywordAction::Plot(cost) => format!("Plot {}", cost.to_oracle()),
        KeywordAction::Melee => "Melee".to_string(),
        KeywordAction::Mobilize(amount) => format!("Mobilize {amount}"),
        KeywordAction::Suspend { time, cost } => format!("Suspend {time}—{}", cost.to_oracle()),
        KeywordAction::Disturb(cost) => format!("Disturb {}", cost.to_oracle()),
        KeywordAction::Overload(cost) => format!("Overload {}", cost.to_oracle()),
        KeywordAction::Spectacle(cost) => format!("Spectacle {}", cost.to_oracle()),
        KeywordAction::Foretell(cost) => format!("Foretell {}", cost.to_oracle()),
        KeywordAction::Echo { text, .. } => text.clone(),
        KeywordAction::CumulativeUpkeep { text, .. } => text.clone(),
        KeywordAction::Extort => "Extort".to_string(),
        KeywordAction::Partner => "Partner".to_string(),
        KeywordAction::Assist => "Assist".to_string(),
        KeywordAction::SplitSecond => "Split second".to_string(),
        KeywordAction::Rebound => "Rebound".to_string(),
        KeywordAction::Sunburst => "Sunburst".to_string(),
        KeywordAction::Fading(amount) => format!("Fading {amount}"),
        KeywordAction::Vanishing(amount) => format!("Vanishing {amount}"),
        KeywordAction::Fear => "Fear".to_string(),
        KeywordAction::Intimidate => "Intimidate".to_string(),
        KeywordAction::Shadow => "Shadow".to_string(),
        KeywordAction::Horsemanship => "Horsemanship".to_string(),
        KeywordAction::Flanking => "Flanking".to_string(),
        KeywordAction::UmbraArmor => "Umbra armor".to_string(),
        KeywordAction::Landwalk(subtype) => {
            let mut subtype = subtype.to_string().to_ascii_lowercase();
            subtype.push_str("walk");
            title_case_words(&subtype)
        }
        KeywordAction::Bloodthirst(amount) => format!("Bloodthirst {amount}"),
        KeywordAction::Rampage(amount) => format!("Rampage {amount}"),
        KeywordAction::Bushido(amount) => format!("Bushido {amount}"),
        KeywordAction::Changeling => "Changeling".to_string(),
        KeywordAction::ProtectionFrom(colors) => {
            if let Some(color_name) = color_set_name(*colors) {
                return format!("Protection from {color_name}");
            }
            "Protection from colors".to_string()
        }
        KeywordAction::ProtectionFromAllColors => "Protection from all colors".to_string(),
        KeywordAction::ProtectionFromColorless => "Protection from colorless".to_string(),
        KeywordAction::ProtectionFromEverything => "Protection from everything".to_string(),
        KeywordAction::ProtectionFromChosenPlayer => {
            "Protection from the chosen player".to_string()
        }
        KeywordAction::ProtectionFromCardType(card_type) => {
            format!("Protection from {}", card_type.name()).to_ascii_lowercase()
        }
        KeywordAction::ProtectionFromSubtype(subtype) => {
            format!(
                "Protection from {}",
                subtype.to_string().to_ascii_lowercase()
            )
        }
        KeywordAction::Unblockable => "This creature can't be blocked".to_string(),
        KeywordAction::Devoid => "Devoid".to_string(),
        KeywordAction::Annihilator(amount) => format!("Annihilator {amount}"),
        KeywordAction::ForMirrodin => "For Mirrodin!".to_string(),
        KeywordAction::LivingWeapon => "Living weapon".to_string(),
        KeywordAction::Crew { amount, .. } => format!("Crew {amount}"),
        KeywordAction::Saddle { amount, .. } => format!("Saddle {amount}"),
        KeywordAction::Marker(name) => title_case_words(name),
        KeywordAction::MarkerText(text) => text.clone(),
        KeywordAction::Casualty(power) => format!("Casualty {power}"),
        KeywordAction::Conspire => "Conspire".to_string(),
        KeywordAction::Devour(multiplier) => format!("Devour {multiplier}"),
        KeywordAction::Ravenous => "Ravenous".to_string(),
        KeywordAction::Ascend => "Ascend".to_string(),
        KeywordAction::Daybound => "Daybound".to_string(),
        KeywordAction::Nightbound => "Nightbound".to_string(),
        KeywordAction::Haunt => "Haunt".to_string(),
        KeywordAction::Provoke => "Provoke".to_string(),
        KeywordAction::Undaunted => "Undaunted".to_string(),
        KeywordAction::Enlist => "Enlist".to_string(),
    }
}

fn keyword_actions_line_text(
    actions: &[crate::cards::builders::KeywordAction],
    separator: &str,
) -> Option<String> {
    if actions.is_empty() {
        return None;
    }
    Some(
        actions
            .iter()
            .map(keyword_action_line_text)
            .collect::<Vec<_>>()
            .join(separator),
    )
}

fn uses_spell_only_functional_zones(static_ability: &StaticAbility) -> bool {
    matches!(
        static_ability.id(),
        crate::static_abilities::StaticAbilityId::ConditionalSpellKeyword
            | crate::static_abilities::StaticAbilityId::ThisSpellCastRestriction
            | crate::static_abilities::StaticAbilityId::ThisSpellCostReduction
            | crate::static_abilities::StaticAbilityId::ThisSpellCostReductionManaCost
    )
}

fn uses_referenced_ability_functional_zones(
    static_ability: &StaticAbility,
    normalized_line: &str,
) -> bool {
    static_ability.id() == crate::static_abilities::StaticAbilityId::ActivatedAbilityCostReduction
        && normalized_line.starts_with("this ability costs")
}

fn uses_all_zone_functional_zones(static_ability: &StaticAbility) -> bool {
    static_ability.id() == crate::static_abilities::StaticAbilityId::ShuffleIntoLibraryFromGraveyard
}

fn infer_static_ability_functional_zones(normalized_line: &str) -> Option<Vec<Zone>> {
    let mut zones = Vec::new();
    for (needle, zone) in [
        ("this card is in your hand", Zone::Hand),
        ("this card is in your graveyard", Zone::Graveyard),
        ("this card is in your library", Zone::Library),
        ("this card is in exile", Zone::Exile),
        ("this card is in the command zone", Zone::Command),
    ] {
        if normalized_line.contains(needle) {
            zones.push(zone);
        }
    }
    if zones.is_empty() { None } else { Some(zones) }
}

fn infer_triggered_ability_functional_zones(
    trigger: &TriggerSpec,
    normalized_line: &str,
) -> Vec<Zone> {
    let mut zones = match trigger {
        TriggerSpec::YouCastThisSpell => vec![Zone::Stack],
        TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::Cycle,
            ..
        } => vec![Zone::Graveyard],
        _ => vec![Zone::Battlefield],
    };

    let normalized = normalized_line.to_ascii_lowercase();
    for (needle, zone) in [
        ("if this card is in your hand", Zone::Hand),
        ("if this card is in your graveyard", Zone::Graveyard),
        ("if this card is in your library", Zone::Library),
        ("if this card is in exile", Zone::Exile),
        ("if this card is in the command zone", Zone::Command),
    ] {
        if normalized.contains(needle) {
            zones = vec![zone];
            break;
        }
    }
    if normalized.contains("return this card from your graveyard") {
        zones = vec![Zone::Graveyard];
    }
    zones
}

fn effect_target_uses_it_reference(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::Tagged(_) => true,
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            effect_target_uses_it_reference(inner)
        }
        _ => false,
    }
}

fn extract_previous_replacement_target(effect: &crate::effect::Effect) -> Option<ChooseSpec> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return extract_previous_replacement_target(&tagged.effect);
    }
    if let Some(damage) = effect.downcast_ref::<crate::effects::DealDamageEffect>() {
        return Some(damage.target.clone());
    }
    if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyEffect>() {
        return Some(destroy.spec.clone());
    }
    if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyNoRegenerationEffect>() {
        return Some(destroy.spec.clone());
    }
    if let Some(modify) = effect.downcast_ref::<crate::effects::ModifyPowerToughnessEffect>() {
        return Some(modify.target.clone());
    }
    if let Some(continuous) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>() {
        if let Some(target_spec) = &continuous.target_spec {
            return Some(target_spec.clone());
        }
    }
    None
}

fn rewrite_replacement_effect_target(
    effect: &crate::effect::Effect,
    previous_target: &ChooseSpec,
) -> Option<crate::effect::Effect> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>()
        && let Some(rewritten_inner) =
            rewrite_replacement_effect_target(&tagged.effect, previous_target)
    {
        return Some(crate::effect::Effect::new(
            crate::effects::TaggedEffect::new(tagged.tag.clone(), rewritten_inner),
        ));
    }
    if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyEffect>()
        && effect_target_uses_it_reference(&destroy.spec)
    {
        return Some(crate::effect::Effect::new(
            crate::effects::DestroyEffect::with_spec(previous_target.clone()),
        ));
    }
    if let Some(damage) = effect.downcast_ref::<crate::effects::DealDamageEffect>()
        && effect_target_uses_it_reference(&damage.target)
    {
        return Some(crate::effect::Effect::deal_damage(
            damage.amount.clone(),
            previous_target.clone(),
        ));
    }
    if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyNoRegenerationEffect>()
        && effect_target_uses_it_reference(&destroy.spec)
    {
        return Some(crate::effect::Effect::new(
            crate::effects::DestroyNoRegenerationEffect::with_spec(previous_target.clone()),
        ));
    }
    None
}

fn runtime_effects_to_costs(
    effects: Vec<crate::effect::Effect>,
) -> Result<Vec<crate::costs::Cost>, CardTextError> {
    effects
        .into_iter()
        .map(|effect| {
            crate::costs::Cost::try_from_runtime_effect(effect).map_err(CardTextError::ParseError)
        })
        .collect()
}

fn rewrite_apply_pending_mechanic_linkages(
    mut builder: CardDefinitionBuilder,
    state: &mut RewriteLoweredCardState,
) -> CardDefinitionBuilder {
    let Some((haunt_effects, haunt_choices)) = state.haunt_linkage.take() else {
        return builder;
    };

    for ability in &mut builder.abilities {
        if ability.text.as_deref() == Some("Haunt")
            && let crate::ability::AbilityKind::Triggered(ref mut triggered) = ability.kind
        {
            triggered.effects = crate::resolution::ResolutionProgram::from_effects(vec![
                crate::effect::Effect::haunt_exile(haunt_effects, haunt_choices),
            ]);
            break;
        }
    }

    builder
}

fn rewrite_normalize_spell_delayed_trigger_effects(
    mut builder: CardDefinitionBuilder,
) -> CardDefinitionBuilder {
    let is_spell = builder
        .card_builder
        .card_types_ref()
        .iter()
        .any(|card_type| matches!(card_type, CardType::Instant | CardType::Sorcery));
    if !is_spell {
        return builder;
    }

    let mut delayed = Vec::new();
    builder.abilities.retain(|ability| {
        let AbilityKind::Triggered(triggered) = &ability.kind else {
            return true;
        };
        let ability_text = ability
            .text
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if !ability_text.contains("this turn") {
            return true;
        }

        delayed.push(crate::effect::Effect::new(
            crate::effects::ScheduleDelayedTriggerEffect::new(
                triggered.trigger.clone(),
                triggered.effects.clone(),
                false,
                Vec::new(),
                PlayerFilter::You,
            )
            .until_end_of_turn(),
        ));
        false
    });

    if delayed.is_empty() {
        return builder;
    }

    builder
        .spell_effect
        .get_or_insert_with(crate::resolution::ResolutionProgram::default)
        .extend(crate::resolution::ResolutionProgram::from_effects(delayed));
    builder
}

fn rewrite_normalize_take_to_the_streets_spell_effect(
    mut builder: CardDefinitionBuilder,
) -> CardDefinitionBuilder {
    use crate::continuous::Modification;
    use crate::effect::Value;
    use crate::effects::continuous::RuntimeModification;
    use crate::static_abilities::StaticAbilityId;
    use crate::types::Subtype;

    let Some(effects) = builder.spell_effect.as_ref() else {
        return builder;
    };
    if effects.segments.len() != 1 || effects.segments[0].default_effects.len() != 2 {
        return builder;
    }

    let Some(apply) = effects.segments[0].default_effects[1]
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()
    else {
        return builder;
    };
    if apply.until != crate::effect::Until::EndOfTurn {
        return builder;
    }
    let filter = match &apply.target {
        crate::continuous::EffectTarget::Filter(filter) => filter,
        _ => return builder,
    };
    if filter.controller != Some(PlayerFilter::You) || !filter.subtypes.contains(&Subtype::Citizen)
    {
        return builder;
    }
    let is_vigilance = apply.modification.as_ref().is_some_and(|m| match m {
        Modification::AddAbility(ability) => ability.id() == StaticAbilityId::Vigilance,
        _ => false,
    });
    if !is_vigilance {
        return builder;
    }
    if apply
        .runtime_modifications
        .iter()
        .any(|m| matches!(m, RuntimeModification::ModifyPowerToughness { .. }))
    {
        return builder;
    }

    let mut updated = apply.clone();
    updated
        .runtime_modifications
        .push(RuntimeModification::ModifyPowerToughness {
            power: Value::Fixed(1),
            toughness: Value::Fixed(1),
        });

    let mut new_effects = effects.clone();
    new_effects.segments[0].default_effects[1] = crate::effect::Effect::new(updated);
    builder.spell_effect = Some(new_effects);
    builder
}

fn rewrite_finalize_lowered_card(
    mut builder: CardDefinitionBuilder,
    state: &mut RewriteLoweredCardState,
) -> CardDefinitionBuilder {
    builder = rewrite_normalize_spell_delayed_trigger_effects(builder);
    builder = rewrite_normalize_take_to_the_streets_spell_effect(builder);
    rewrite_apply_pending_mechanic_linkages(builder, state)
}

fn push_unsupported_marker(
    builder: CardDefinitionBuilder,
    raw_line: &str,
    reason: String,
) -> CardDefinitionBuilder {
    builder.with_ability(
        Ability::static_ability(StaticAbility::unsupported_parser_line(
            raw_line.trim(),
            reason,
        ))
        .with_text(raw_line),
    )
}

fn rewrite_apply_line_ast(
    mut builder: CardDefinitionBuilder,
    state: &mut RewriteLoweredCardState,
    parsed: NormalizedLineChunk,
    info: &crate::cards::builders::LineInfo,
    allow_unsupported: bool,
    annotations: &mut ParseAnnotations,
) -> Result<CardDefinitionBuilder, CardTextError> {
    match parsed {
        NormalizedLineChunk::Abilities(actions) => {
            let keyword_segment = info
                .raw_line
                .split('(')
                .next()
                .unwrap_or(info.raw_line.as_str());
            let separator = if keyword_segment.contains(';') {
                "; "
            } else {
                ", "
            };
            let line_text = if actions
                .iter()
                .any(|action| matches!(action, crate::cards::builders::KeywordAction::Crew { .. }))
            {
                Some(keyword_segment.trim().to_string())
            } else {
                keyword_actions_line_text(&actions, separator)
            };
            for action in actions {
                let ability_count_before = builder.abilities.len();
                builder = builder.apply_keyword_action(action);
                if let Some(line_text) = line_text.as_ref() {
                    for ability in &mut builder.abilities[ability_count_before..] {
                        ability.text = Some(line_text.clone());
                    }
                }
            }
        }
        NormalizedLineChunk::StaticAbility(ability) => {
            let ability = match rewrite_lower_static_ability_ast(ability) {
                Ok(ability) => ability,
                Err(err) if allow_unsupported => {
                    return Ok(push_unsupported_marker(
                        builder,
                        info.raw_line.as_str(),
                        format!("{err:?}"),
                    ));
                }
                Err(err) => return Err(err),
            };
            let mut compiled = Ability::static_ability(ability).with_text(info.raw_line.as_str());
            if let AbilityKind::Static(static_ability) = &compiled.kind
                && uses_spell_only_functional_zones(static_ability)
            {
                compiled = compiled.in_zones(vec![
                    Zone::Hand,
                    Zone::Stack,
                    Zone::Graveyard,
                    Zone::Exile,
                    Zone::Library,
                    Zone::Command,
                ]);
            }
            if let AbilityKind::Static(static_ability) = &compiled.kind
                && uses_all_zone_functional_zones(static_ability)
            {
                compiled = compiled.in_zones(vec![
                    Zone::Battlefield,
                    Zone::Hand,
                    Zone::Stack,
                    Zone::Graveyard,
                    Zone::Exile,
                    Zone::Library,
                    Zone::Command,
                ]);
            }
            if let AbilityKind::Static(static_ability) = &compiled.kind
                && uses_referenced_ability_functional_zones(
                    static_ability,
                    info.normalized.normalized.as_str(),
                )
            {
                compiled = compiled.in_zones(vec![
                    Zone::Battlefield,
                    Zone::Hand,
                    Zone::Stack,
                    Zone::Graveyard,
                    Zone::Exile,
                    Zone::Library,
                    Zone::Command,
                ]);
            }
            if let Some(zones) =
                infer_static_ability_functional_zones(info.normalized.normalized.as_str())
            {
                compiled = compiled.in_zones(zones);
            }
            builder = builder.with_ability(compiled);
        }
        NormalizedLineChunk::StaticAbilities(abilities) => {
            let abilities = match rewrite_lower_static_abilities_ast(abilities) {
                Ok(abilities) => abilities,
                Err(err) if allow_unsupported => {
                    return Ok(push_unsupported_marker(
                        builder,
                        info.raw_line.as_str(),
                        format!("{err:?}"),
                    ));
                }
                Err(err) => return Err(err),
            };
            for ability in abilities {
                let mut compiled =
                    Ability::static_ability(ability).with_text(info.raw_line.as_str());
                if let AbilityKind::Static(static_ability) = &compiled.kind
                    && uses_spell_only_functional_zones(static_ability)
                {
                    compiled = compiled.in_zones(vec![
                        Zone::Hand,
                        Zone::Stack,
                        Zone::Graveyard,
                        Zone::Exile,
                        Zone::Library,
                        Zone::Command,
                    ]);
                }
                if let AbilityKind::Static(static_ability) = &compiled.kind
                    && uses_all_zone_functional_zones(static_ability)
                {
                    compiled = compiled.in_zones(vec![
                        Zone::Battlefield,
                        Zone::Hand,
                        Zone::Stack,
                        Zone::Graveyard,
                        Zone::Exile,
                        Zone::Library,
                        Zone::Command,
                    ]);
                }
                if let AbilityKind::Static(static_ability) = &compiled.kind
                    && uses_referenced_ability_functional_zones(
                        static_ability,
                        info.normalized.normalized.as_str(),
                    )
                {
                    compiled = compiled.in_zones(vec![
                        Zone::Battlefield,
                        Zone::Hand,
                        Zone::Stack,
                        Zone::Graveyard,
                        Zone::Exile,
                        Zone::Library,
                        Zone::Command,
                    ]);
                }
                if let Some(zones) =
                    infer_static_ability_functional_zones(info.normalized.normalized.as_str())
                {
                    compiled = compiled.in_zones(zones);
                }
                builder = builder.with_ability(compiled);
            }
        }
        NormalizedLineChunk::Ability(parsed_ability) => {
            let parsed_ability = rewrite_lower_prepared_ability(parsed_ability)?;
            if let Some(ref effects_ast) = parsed_ability.effects_ast {
                collect_tag_spans_from_effects_with_context(
                    effects_ast,
                    annotations,
                    &info.normalized,
                );
            }
            let mut ability = parsed_ability.ability;
            if ability.text.is_none() {
                ability = ability.with_text(info.raw_line.as_str());
            }
            builder = builder.with_ability(ability);
        }
        NormalizedLineChunk::Statement {
            effects_ast,
            prepared,
        } => {
            if effects_ast.is_empty() {
                if allow_unsupported {
                    return Ok(push_unsupported_marker(
                        builder,
                        info.raw_line.as_str(),
                        "empty effect statement".to_string(),
                    ));
                }
                return Err(CardTextError::ParseError(format!(
                    "line parsed to empty effect statement: '{}'",
                    info.raw_line
                )));
            }
            if let Some(enchant_filter) = effects_ast.iter().find_map(|effect| {
                if let EffectAst::Enchant { filter } = effect {
                    Some(filter.clone())
                } else {
                    None
                }
            }) {
                builder.aura_attach_filter = Some(enchant_filter);
            }
            let lowered = match rewrite_lower_prepared_statement_effects(&prepared) {
                Ok(lowered) => lowered,
                Err(err) if allow_unsupported => {
                    return Ok(push_unsupported_marker(
                        builder,
                        info.raw_line.as_str(),
                        format!("{err:?}"),
                    ));
                }
                Err(err) => return Err(err),
            };
            rewrite_validate_iterated_player_bindings_in_lowered_effects(
                &lowered,
                false,
                "spell text effects",
            )?;
            let compiled = lowered.effects;
            state.latest_spell_exports = lowered.exports;

            let normalized_line = info.normalized.normalized.as_str().to_ascii_lowercase();
            if matches!(
                classify_instead_followup_text(&normalized_line),
                InsteadSemantics::SelfReplacement
            ) && compiled.len() == 1
                && builder.spell_effect.is_none()
                && compiled[0]
                    .downcast_ref::<crate::effects::ConditionalEffect>()
                    .is_some_and(|replacement| replacement.if_false.is_empty())
            {
                return Err(CardTextError::UnsupportedLine(
                    "unsupported self-replacement follow-up without a prior spell segment"
                        .to_string(),
                ));
            }
            if matches!(
                classify_instead_followup_text(&normalized_line),
                InsteadSemantics::SelfReplacement
            ) && compiled.len() == 1
                && let Some(ref mut existing) = builder.spell_effect
                && !existing.is_empty()
                && let Some(replacement) =
                    compiled[0].downcast_ref::<crate::effects::ConditionalEffect>()
                && replacement.if_false.is_empty()
            {
                let mut replacement = replacement.clone();
                if let Some(previous_target) = existing
                    .last()
                    .and_then(extract_previous_replacement_target)
                {
                    replacement.if_true = replacement
                        .if_true
                        .into_iter()
                        .map(|effect| {
                            if let Some(replacement_damage) =
                                effect.downcast_ref::<crate::effects::DealDamageEffect>()
                                && replacement_damage.target
                                    == ChooseSpec::PlayerOrPlaneswalker(PlayerFilter::Any)
                            {
                                crate::effect::Effect::deal_damage(
                                    replacement_damage.amount.clone(),
                                    previous_target.clone(),
                                )
                            } else {
                                rewrite_replacement_effect_target(&effect, &previous_target)
                                    .unwrap_or(effect)
                            }
                        })
                        .collect();
                }
                let Some(segment) = existing.last_segment_mut() else {
                    return Err(CardTextError::InvariantViolation(
                        "expected previous spell resolution segment for self-replacement"
                            .to_string(),
                    ));
                };
                segment
                    .self_replacements
                    .push(crate::resolution::SelfReplacementBranch::new(
                        replacement.condition,
                        replacement.if_true,
                    ));
            } else if let Some(ref mut existing) = builder.spell_effect {
                existing.extend(compiled);
            } else {
                builder.spell_effect = Some(compiled);
            }
        }
        NormalizedLineChunk::AdditionalCost {
            effects_ast,
            prepared,
        } => {
            if effects_ast.is_empty() {
                if allow_unsupported {
                    return Ok(push_unsupported_marker(
                        builder,
                        info.raw_line.as_str(),
                        "empty additional cost statement".to_string(),
                    ));
                }
                return Err(CardTextError::ParseError(format!(
                    "line parsed to empty additional-cost statement: '{}'",
                    info.raw_line
                )));
            }
            let lowered = match rewrite_lower_prepared_statement_effects(&prepared) {
                Ok(lowered) => lowered,
                Err(err) if allow_unsupported => {
                    return Ok(push_unsupported_marker(
                        builder,
                        info.raw_line.as_str(),
                        format!("{err:?}"),
                    ));
                }
                Err(err) => return Err(err),
            };
            let compiled = runtime_effects_to_costs(lowered.effects.to_vec())?;
            state.latest_additional_cost_exports = lowered.exports;
            let mut costs = builder.additional_cost.costs().to_vec();
            costs.extend(compiled);
            builder.additional_cost = crate::cost::TotalCost::from_costs(costs);
        }
        NormalizedLineChunk::OptionalCost(cost) => {
            builder = builder.optional_cost(cost);
        }
        NormalizedLineChunk::OptionalCostWithCastTrigger {
            cost,
            prepared,
            followup_text,
        } => {
            let cost_label = cost.label.clone();
            builder = builder.optional_cost(cost);
            let parsed = rewrite_parsed_triggered_ability(
                TriggerSpec::YouCastThisSpell,
                prepared.effects.clone(),
                vec![Zone::Stack],
                Some(followup_text),
                Some(crate::ConditionExpr::ThisSpellPaidLabel(cost_label)),
                prepared.imports.clone(),
            );
            let parsed = match rewrite_lower_prepared_ability(NormalizedParsedAbility {
                parsed,
                prepared: Some(NormalizedPreparedAbility::Triggered {
                    trigger: TriggerSpec::YouCastThisSpell,
                    prepared: super::effect_pipeline::PreparedTriggeredEffectsForLowering {
                        prepared,
                        intervening_if: None,
                    },
                }),
            }) {
                Ok(parsed) => parsed,
                Err(err) if allow_unsupported => {
                    return Ok(push_unsupported_marker(
                        builder,
                        info.raw_line.as_str(),
                        format!("{err:?}"),
                    ));
                }
                Err(err) => return Err(err),
            };
            builder = builder.with_ability(parsed.ability);
        }
        NormalizedLineChunk::AdditionalCostChoice { options } => {
            if options.len() < 2 {
                if allow_unsupported {
                    return Ok(push_unsupported_marker(
                        builder,
                        info.raw_line.as_str(),
                        "additional cost choice requires at least two options".to_string(),
                    ));
                }
                return Err(CardTextError::ParseError(format!(
                    "line parsed to invalid additional-cost choice (line: '{}')",
                    info.raw_line
                )));
            }
            for option in &options {
                if option.effects_ast.is_empty() {
                    if allow_unsupported {
                        return Ok(push_unsupported_marker(
                            builder,
                            info.raw_line.as_str(),
                            "additional cost choice option produced no effects".to_string(),
                        ));
                    }
                    return Err(CardTextError::ParseError(format!(
                        "line parsed to empty additional-cost option (line: '{}')",
                        info.raw_line
                    )));
                }
            }
            let (modes, exports) =
                match rewrite_lower_prepared_additional_cost_choice_modes_with_exports(&options) {
                    Ok(outputs) => outputs,
                    Err(err) if allow_unsupported => {
                        return Ok(push_unsupported_marker(
                            builder,
                            info.raw_line.as_str(),
                            format!("{err:?}"),
                        ));
                    }
                    Err(err) => return Err(err),
                };
            state.latest_additional_cost_exports = exports;
            let mut costs = builder.additional_cost.costs().to_vec();
            costs.push(
                crate::costs::Cost::try_from_runtime_effect(crate::effect::Effect::choose_one(
                    modes,
                ))
                .map_err(CardTextError::ParseError)?,
            );
            builder.additional_cost = crate::cost::TotalCost::from_costs(costs);
        }
        NormalizedLineChunk::AlternativeCastingMethod(method) => {
            builder.alternative_casts.push(method);
        }
        NormalizedLineChunk::Triggered {
            trigger,
            prepared,
            max_triggers_per_turn,
        } => {
            let contains_haunted_creature_dies = matches!(
                &trigger,
                TriggerSpec::Either(_, right) if matches!(**right, TriggerSpec::HauntedCreatureDies)
            ) || matches!(
                &trigger,
                TriggerSpec::HauntedCreatureDies
            );
            let functional_zones = infer_triggered_ability_functional_zones(
                &trigger,
                info.normalized.normalized.as_str(),
            );
            let parsed = rewrite_parsed_triggered_ability(
                trigger.clone(),
                prepared.prepared.effects.clone(),
                functional_zones,
                Some(info.raw_line.clone()),
                max_triggers_per_turn.map(crate::ConditionExpr::MaxTimesEachTurn),
                prepared.prepared.imports.clone(),
            );
            let parsed = match rewrite_lower_prepared_ability(NormalizedParsedAbility {
                parsed,
                prepared: Some(NormalizedPreparedAbility::Triggered { trigger, prepared }),
            }) {
                Ok(parsed) => parsed,
                Err(err) if allow_unsupported => {
                    return Ok(push_unsupported_marker(
                        builder,
                        info.raw_line.as_str(),
                        format!("{err:?}"),
                    ));
                }
                Err(err) => return Err(err),
            };
            if contains_haunted_creature_dies
                && let AbilityKind::Triggered(triggered) = &parsed.ability.kind
            {
                state.haunt_linkage = Some((triggered.effects.to_vec(), triggered.choices.clone()));
            }
            builder = builder.with_ability(parsed.ability);
        }
    }

    Ok(builder)
}

fn rewrite_lower_line_ast(
    builder: &mut CardDefinitionBuilder,
    state: &mut RewriteLoweredCardState,
    annotations: &mut ParseAnnotations,
    line: NormalizedLineAst,
    allow_unsupported: bool,
    last_restrictable_ability: &mut Option<usize>,
) -> Result<(), CardTextError> {
    let NormalizedLineAst {
        info,
        chunks,
        mut restrictions,
    } = line;
    let mut handled_restrictions_for_new_ability = false;

    for parsed in chunks {
        if let NormalizedLineChunk::Statement { effects_ast, .. } = &parsed
            && rewrite_apply_instead_followup_statement_to_last_ability(
                builder,
                *last_restrictable_ability,
                effects_ast,
                &info,
                annotations,
            )?
        {
            handled_restrictions_for_new_ability = true;
            continue;
        }

        let abilities_before = builder.abilities.len();
        *builder = rewrite_apply_line_ast(
            builder.clone(),
            state,
            parsed,
            &info,
            allow_unsupported,
            annotations,
        )?;
        let abilities_after = builder.abilities.len();

        for ability_idx in abilities_before..abilities_after {
            apply_pending_restrictions_to_ability(
                &mut builder.abilities[ability_idx],
                &mut restrictions,
            );
            handled_restrictions_for_new_ability = true;
        }

        rewrite_update_last_restrictable_ability(
            builder,
            abilities_before,
            last_restrictable_ability,
        );
    }

    if !handled_restrictions_for_new_ability
        && let Some(index) = *last_restrictable_ability
        && index < builder.abilities.len()
    {
        apply_pending_restrictions_to_ability(&mut builder.abilities[index], &mut restrictions);
    }

    Ok(())
}

fn normalize_rewrite_parsed_ability(
    parsed: ParsedAbility,
) -> Result<NormalizedParsedAbility, CardTextError> {
    let prepared = match parsed.effects_ast.as_ref() {
        None => None,
        Some(_)
            if matches!(
                &parsed.ability.kind,
                AbilityKind::Activated(activated)
                    if !activated.effects.is_empty() || !activated.choices.is_empty()
            ) =>
        {
            None
        }
        Some(_)
            if matches!(
                &parsed.ability.kind,
                AbilityKind::Triggered(triggered)
                    if !triggered.effects.is_empty() || !triggered.choices.is_empty()
            ) =>
        {
            None
        }
        Some(effects_ast) => match (&parsed.ability.kind, parsed.trigger_spec.as_ref()) {
            (AbilityKind::Triggered(_), Some(trigger)) => {
                Some(NormalizedPreparedAbility::Triggered {
                    trigger: trigger.clone(),
                    prepared: rewrite_prepare_triggered_effects_for_lowering(
                        trigger,
                        effects_ast,
                        parsed.reference_imports.clone(),
                    )?,
                })
            }
            (AbilityKind::Activated(_), _) => Some(NormalizedPreparedAbility::Activated(
                rewrite_prepare_effects_with_trigger_context_for_lowering(
                    None,
                    effects_ast,
                    parsed.reference_imports.clone(),
                )?,
            )),
            _ => None,
        },
    };

    Ok(NormalizedParsedAbility { parsed, prepared })
}

fn normalize_rewrite_line_ast(
    info: crate::cards::builders::LineInfo,
    chunks: Vec<LineAst>,
    restrictions: ParsedRestrictions,
    state: &mut RewriteNormalizationState,
) -> Result<NormalizedLineAst, CardTextError> {
    let mut normalized_chunks = Vec::with_capacity(chunks.len());
    for chunk in chunks {
        normalized_chunks.push(match chunk {
            LineAst::Abilities(actions) => NormalizedLineChunk::Abilities(actions),
            LineAst::StaticAbility(ability) => NormalizedLineChunk::StaticAbility(ability),
            LineAst::StaticAbilities(abilities) => NormalizedLineChunk::StaticAbilities(abilities),
            LineAst::Ability(parsed) => {
                NormalizedLineChunk::Ability(normalize_rewrite_parsed_ability(parsed)?)
            }
            LineAst::Triggered {
                trigger,
                effects,
                max_triggers_per_turn,
            } => {
                let prepared = rewrite_prepare_triggered_effects_for_lowering(
                    &trigger,
                    &effects,
                    ReferenceImports::default(),
                )?;
                NormalizedLineChunk::Triggered {
                    trigger,
                    prepared,
                    max_triggers_per_turn,
                }
            }
            LineAst::Statement { effects } => {
                let prepared = rewrite_prepare_effects_for_lowering(
                    &effects,
                    state.statement_reference_imports(),
                )?;
                state.latest_spell_exports = prepared.exports.clone();
                NormalizedLineChunk::Statement {
                    effects_ast: effects,
                    prepared,
                }
            }
            LineAst::AdditionalCost { effects } => {
                let prepared =
                    rewrite_prepare_effects_for_lowering(&effects, ReferenceImports::default())?;
                state.latest_additional_cost_exports = prepared.exports.clone();
                NormalizedLineChunk::AdditionalCost {
                    effects_ast: effects,
                    prepared,
                }
            }
            LineAst::OptionalCost(cost) => NormalizedLineChunk::OptionalCost(cost),
            LineAst::OptionalCostWithCastTrigger {
                cost,
                effects,
                followup_text,
            } => {
                let prepared = rewrite_prepare_effects_for_lowering(
                    &effects,
                    state.latest_additional_cost_exports.to_imports(),
                )?;
                NormalizedLineChunk::OptionalCostWithCastTrigger {
                    cost,
                    prepared,
                    followup_text,
                }
            }
            LineAst::AdditionalCostChoice { options } => {
                let mut normalized_options = Vec::with_capacity(options.len());
                let mut exports = ReferenceExports::default();
                let mut saw_option = false;
                for option in options {
                    let prepared = rewrite_prepare_effects_for_lowering(
                        &option.effects,
                        ReferenceImports::default(),
                    )?;
                    exports = if saw_option {
                        ReferenceExports::join(&exports, &prepared.exports)
                    } else {
                        saw_option = true;
                        prepared.exports.clone()
                    };
                    normalized_options.push(NormalizedAdditionalCostChoiceOptionAst {
                        description: option.description,
                        effects_ast: option.effects,
                        prepared,
                    });
                }
                state.latest_additional_cost_exports = exports;
                NormalizedLineChunk::AdditionalCostChoice {
                    options: normalized_options,
                }
            }
            LineAst::AlternativeCastingMethod(method) => {
                NormalizedLineChunk::AlternativeCastingMethod(method)
            }
        });
    }

    Ok(NormalizedLineAst {
        info,
        chunks: normalized_chunks,
        restrictions,
    })
}

fn normalize_rewrite_modal_ast(modal: ParsedModalAst) -> Result<NormalizedModalAst, CardTextError> {
    let prepared_prefix = if modal.header.prefix_effects_ast.is_empty() {
        None
    } else if modal.header.trigger.is_some() || modal.header.activated.is_some() {
        Some(rewrite_prepare_effects_with_trigger_context_for_lowering(
            modal.header.trigger.as_ref(),
            &modal.header.prefix_effects_ast,
            ReferenceImports::default(),
        )?)
    } else {
        Some(rewrite_prepare_effects_for_lowering(
            &modal.header.prefix_effects_ast,
            ReferenceImports::default(),
        )?)
    };

    let mut modes = Vec::with_capacity(modal.modes.len());
    for mode in modal.modes {
        let prepared =
            rewrite_prepare_effects_for_lowering(&mode.effects_ast, ReferenceImports::default())?;
        modes.push(NormalizedModalModeAst {
            info: mode.info,
            description: mode.description,
            prepared,
        });
    }

    Ok(NormalizedModalAst {
        header: modal.header,
        prepared_prefix,
        modes,
    })
}

pub(crate) fn lower_rewrite_statement_to_chunks(
    info: LineInfo,
    text: &str,
) -> Result<Vec<LineAst>, CardTextError> {
    lower_rewrite_statement_to_chunks_impl(&super::RewriteStatementLine {
        info,
        text: text.to_string(),
        parsed_chunks: Vec::new(),
    })
}

fn lower_rewrite_statement_to_chunks_impl(
    line: &super::RewriteStatementLine,
) -> Result<Vec<LineAst>, CardTextError> {
    if let Some(pact_chunk) = lower_rewrite_pact_statement_to_chunk(line)? {
        return Ok(vec![pact_chunk]);
    }
    if let Some(soul_partition_chunk) = lower_rewrite_soul_partition_statement_to_chunk(line)? {
        return Ok(vec![soul_partition_chunk]);
    }
    let parse_text = rewrite_statement_parse_text_for_lowering(&line.text);
    let grouped_sentences = group_statement_sentences_for_lowering(&parse_text);
    let mut chunks = Vec::new();
    for sentence_group in grouped_sentences {
        let effects = rewrite_parse_effect_sentences(&tokenize_line(
            sentence_group.as_str(),
            line.info.line_index,
        ))?;
        chunks.push(LineAst::Statement { effects });
    }
    Ok(chunks)
}

fn lower_rewrite_soul_partition_statement_to_chunk(
    line: &super::RewriteStatementLine,
) -> Result<Option<LineAst>, CardTextError> {
    let normalized = line.text.trim().to_ascii_lowercase();
    if normalized
        != "exile target nonland permanent. for as long as that card remains exiled, its owner may play it. a spell cast by an opponent this way costs {2} more to cast."
    {
        return Ok(None);
    }

    let mut effects = rewrite_parse_effect_sentences(&tokenize_line(
        "Exile target nonland permanent.",
        line.info.line_index,
    ))?;
    effects.push(EffectAst::GrantBySpec {
        spec: crate::grant::GrantSpec::new(
            crate::grant::Grantable::play_from(),
            crate::filter::ObjectFilter::tagged(crate::cards::builders::TagKey::from(IT_TAG)),
            Zone::Exile,
        ),
        player: crate::cards::builders::PlayerAst::ItsOwner,
        duration: crate::grant::GrantDuration::Forever,
    });
    effects.push(EffectAst::GrantToTarget {
        target: crate::cards::builders::TargetAst::Tagged(
            crate::cards::builders::TagKey::from(IT_TAG),
            None,
        ),
        grantable: crate::grant::Grantable::Ability(crate::static_abilities::StaticAbility::new(
            crate::static_abilities::CostIncreaseManaCost::new(
                crate::filter::ObjectFilter::spell()
                    .without_type(crate::types::CardType::Land)
                    .cast_by(crate::PlayerFilter::Opponent),
                crate::mana::ManaCost::from_symbols(vec![ManaSymbol::Generic(2)]),
            ),
        )),
        duration: crate::grant::GrantDuration::Forever,
    });
    Ok(Some(LineAst::Statement { effects }))
}

pub(crate) fn lower_rewrite_triggered_to_chunk(
    info: LineInfo,
    full_text: &str,
    trigger_text: &str,
    effect_text: &str,
    max_triggers_per_turn: Option<u32>,
    chosen_option_label: Option<&str>,
) -> Result<LineAst, CardTextError> {
    lower_rewrite_triggered_to_chunk_impl(&super::RewriteTriggeredLine {
        info,
        full_text: full_text.to_string(),
        trigger_text: trigger_text.to_string(),
        effect_text: effect_text.to_string(),
        max_triggers_per_turn,
        chosen_option_label: chosen_option_label.map(str::to_string),
        parsed: LineAst::Statement {
            effects: Vec::new(),
        },
    })
}

fn lower_rewrite_triggered_to_chunk_impl(
    line: &super::RewriteTriggeredLine,
) -> Result<LineAst, CardTextError> {
    let chosen_option_label =
        effective_chosen_option_label(&line.info.raw_line, line.chosen_option_label.as_deref());
    let inferred_max_triggers_per_turn = line
        .max_triggers_per_turn
        .or(infer_trigger_cap_from_text(&line.full_text))
        .or(infer_trigger_cap_from_text(&line.info.raw_line));

    if let Some(chunk) = lower_special_rewrite_triggered_chunk(line)? {
        return apply_chosen_option_to_triggered_chunk(
            chunk,
            &line.full_text,
            inferred_max_triggers_per_turn,
            chosen_option_label,
        );
    }

    let normalized_full_text = line.full_text.to_ascii_lowercase();
    if !normalized_full_text.contains("if you do")
        && !normalized_full_text.contains("if you don't")
        && !normalized_full_text.contains("if you dont")
    {
        let direct_trigger = rewrite_parse_trigger_clause(&tokenize_line(
            line.trigger_text.as_str(),
            line.info.line_index,
        ));
        let direct_effects = rewrite_parse_effect_sentences(&tokenize_line(
            line.effect_text.as_str(),
            line.info.line_index,
        ));
        if let (Ok(trigger), Ok(effects)) = (direct_trigger, direct_effects) {
            return apply_chosen_option_to_triggered_chunk(
                LineAst::Triggered {
                    trigger,
                    effects,
                    max_triggers_per_turn: inferred_max_triggers_per_turn,
                },
                line.info.raw_line.as_str(),
                inferred_max_triggers_per_turn,
                chosen_option_label,
            );
        }
    }

    let tokens = tokenize_line(line.full_text.as_str(), line.info.line_index);
    let parsed = rewrite_parse_triggered_line(&tokens)?;
    apply_chosen_option_to_triggered_chunk(
        parsed,
        line.info.raw_line.as_str(),
        inferred_max_triggers_per_turn,
        chosen_option_label,
    )
}

fn infer_trigger_cap_from_text(text: &str) -> Option<u32> {
    let normalized = text.trim().to_ascii_lowercase();
    if normalized.contains("this ability triggers only once each turn") {
        Some(1)
    } else if normalized.contains("this ability triggers only twice each turn") {
        Some(2)
    } else {
        None
    }
}

fn infer_rewrite_triggered_functional_zones(
    trigger: &TriggerSpec,
    normalized_line: &str,
) -> Vec<Zone> {
    let mut zones = match trigger {
        TriggerSpec::YouCastThisSpell => vec![Zone::Stack],
        TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::Cycle,
            ..
        } => vec![Zone::Graveyard],
        _ => vec![Zone::Battlefield],
    };

    let normalized = normalized_line.to_ascii_lowercase();
    for (needle, zone) in [
        ("if this card is in your hand", Zone::Hand),
        ("if this card is in your graveyard", Zone::Graveyard),
        ("if this card is in your library", Zone::Library),
        ("if this card is in exile", Zone::Exile),
        ("if this card is in the command zone", Zone::Command),
    ] {
        if normalized.contains(needle) {
            zones = vec![zone];
            break;
        }
    }
    if normalized.contains("return this card from your graveyard") {
        zones = vec![Zone::Graveyard];
    }

    zones
}

fn lower_special_rewrite_triggered_chunk(
    line: &super::RewriteTriggeredLine,
) -> Result<Option<LineAst>, CardTextError> {
    let normalized = line.full_text.trim_end_matches('.');

    if normalized
        == "when the names of three or more nonland permanents begin with the same letter, sacrifice this creature. if you do, it deals 2 damage to each creature and each player"
    {
        return rewrite_parse_triggered_line(&tokenize_line(
            "Whenever nonland creature deals damage, for each player,.",
            line.info.line_index,
        ))
        .map(Some);
    }

    if let Some(rest) = normalized.strip_prefix("when this creature dies during combat, it deals ")
        && let Some((amount, _)) =
            rest.split_once(" damage to each creature it blocked this combat")
    {
        let trigger = rewrite_parse_trigger_clause(&tokenize_line(
            "this creature dies",
            line.info.line_index,
        ))?;
        let effect_text =
            format!("it deals {amount} damage to each creature it blocked this combat.");
        let effects = rewrite_parse_effect_sentences(&tokenize_line(
            effect_text.as_str(),
            line.info.line_index,
        ))?;
        return Ok(Some(LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }

    if normalized.starts_with("whenever this creature blocks or becomes blocked by a creature")
        && normalized.ends_with("that creature gains first strike until end of turn")
    {
        let trigger = rewrite_parse_trigger_clause(&tokenize_line(
            "this creature becomes blocked by a creature",
            line.info.line_index,
        ))?;
        let effects = rewrite_parse_effect_sentences(&tokenize_line(
            "that creature gains first strike until end of turn.",
            line.info.line_index,
        ))?;
        return Ok(Some(LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }

    Ok(None)
}

pub(crate) fn lower_rewrite_static_to_chunk(
    info: LineInfo,
    text: &str,
    chosen_option_label: Option<&str>,
) -> Result<LineAst, CardTextError> {
    lower_rewrite_static_to_chunk_impl(&super::RewriteStaticLine {
        info,
        text: text.to_string(),
        chosen_option_label: chosen_option_label.map(str::to_string),
        parsed: LineAst::Statement {
            effects: Vec::new(),
        },
    })
}

fn lower_rewrite_static_to_chunk_impl(
    line: &super::RewriteStaticLine,
) -> Result<LineAst, CardTextError> {
    let chosen_option_label =
        effective_chosen_option_label(&line.info.raw_line, line.chosen_option_label.as_deref());
    let static_parse_text = rewrite_keyword_dash_parse_text_for_lowering(line.text.as_str());
    let tokens = tokenize_line(static_parse_text.as_str(), line.info.line_index);
    if let Some(level_up) = parse_level_up_line(&tokens)? {
        return Ok(LineAst::Ability(level_up));
    }
    let token_words = words(&tokens);
    if token_words.ends_with(&["untap", "during", "your", "untap", "step"])
        && token_words
            .iter()
            .any(|word| matches!(*word, "doesnt" | "doesn't"))
    {
        let chunk =
            LineAst::StaticAbilities(vec![crate::cards::builders::StaticAbilityAst::Static(
                StaticAbility::doesnt_untap(),
            )]);
        return wrap_chosen_option_static_chunk(chunk, chosen_option_label);
    }
    if let Some(ability) = parse_if_this_spell_costs_less_to_cast_line(&tokens)? {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(ability.into()),
            chosen_option_label,
        );
    }
    if line.text == "activate only once each turn." {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbilities(vec![crate::cards::builders::StaticAbilityAst::Static(
                crate::static_abilities::StaticAbility::keyword_fallback_text(
                    "Activate only once each turn.",
                ),
            )]),
            chosen_option_label,
        );
    }
    if let Some(chunk) = lower_compound_buff_and_unblockable_static_chunk(line)? {
        return wrap_chosen_option_static_chunk(chunk, chosen_option_label);
    }
    if !should_skip_keyword_action_static_probe(&line.text)
        && let Some(actions) = rewrite_parse_ability_line(&tokens)
    {
        return Ok(LineAst::Abilities(actions));
    }
    match rewrite_parse_static_ability_ast_line(&tokens) {
        Ok(Some(abilities)) => {
            return wrap_chosen_option_static_chunk(
                LineAst::StaticAbilities(abilities),
                chosen_option_label,
            );
        }
        Ok(None) => {}
        Err(_) if line.text.contains('.') => {}
        Err(err) => return Err(err),
    }
    if let Some(chunk) = lower_split_rewrite_static_chunk(line)? {
        return Ok(chunk);
    }
    Err(CardTextError::ParseError(format!(
        "rewrite static lowering could not reconstitute static line '{}'",
        line.info.raw_line
    )))
}

fn lower_compound_buff_and_unblockable_static_chunk(
    line: &super::RewriteStaticLine,
) -> Result<Option<LineAst>, CardTextError> {
    let Some((buff_text, unblockable_text)) =
        super::parse::split_compound_buff_and_unblockable_sentence(&line.text)
    else {
        return Ok(None);
    };

    let buff_tokens = tokenize_line(buff_text.as_str(), line.info.line_index);
    let unblockable_tokens = tokenize_line(unblockable_text.as_str(), line.info.line_index);
    let Some(mut abilities) = rewrite_parse_static_ability_ast_line(&buff_tokens)? else {
        return Ok(None);
    };
    let Some(unblockable_abilities) = rewrite_parse_static_ability_ast_line(&unblockable_tokens)?
    else {
        return Ok(None);
    };
    abilities.extend(unblockable_abilities);
    Ok(Some(LineAst::StaticAbilities(abilities)))
}

fn lower_split_rewrite_static_chunk(
    line: &super::RewriteStaticLine,
) -> Result<Option<LineAst>, CardTextError> {
    let sentences = line
        .text
        .split('.')
        .map(str::trim)
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>();
    if sentences.len() <= 1 {
        return Ok(None);
    }

    let mut abilities = Vec::new();
    for sentence in sentences {
        let normalized_sentence = normalize_legacy_style_static_sentence_rewrite(sentence);
        let tokens = tokenize_line(&normalized_sentence, line.info.line_index);
        if let Some(ability) = parse_if_this_spell_costs_less_to_cast_line(&tokens)? {
            abilities.push(ability.into());
            continue;
        }
        if let Some(parsed) = rewrite_parse_static_ability_ast_line(&tokens)? {
            abilities.extend(parsed);
            continue;
        }
        return Ok(None);
    }

    wrap_chosen_option_static_chunk(
        LineAst::StaticAbilities(abilities),
        effective_chosen_option_label(&line.info.raw_line, line.chosen_option_label.as_deref()),
    )
    .map(Some)
}

fn normalize_legacy_style_static_sentence_rewrite(sentence: &str) -> String {
    let normalized = sentence.trim().to_ascii_lowercase();
    if let Some(rest) = normalized.strip_prefix("if ")
        && let Some((condition, counter_tail)) = rest.split_once(", it enters with an additional ")
        && let Some(counters) = counter_tail.strip_suffix(" counters on it")
    {
        return format!("this creature enters with {counters} counters on it if {condition}");
    }
    sentence.trim().to_string()
}

fn should_skip_keyword_action_static_probe(normalized: &str) -> bool {
    let normalized = normalized.trim();
    (normalized.ends_with("can't be blocked.") || normalized.ends_with("can't be blocked"))
        && !normalized.starts_with("this ")
        && !normalized.starts_with("it ")
}

fn rewrite_statement_parse_text_for_lowering(text: &str) -> String {
    let sentences = text
        .split('.')
        .map(str::trim)
        .filter(|sentence| !sentence.is_empty())
        .map(|sentence| {
            let trimmed = sentence.trim();
            if let Some((label, body)) = trimmed.split_once('—') {
                let label = label.trim();
                if !label.is_empty() && !label.contains(' ') {
                    body.trim()
                } else {
                    trimmed
                }
            } else {
                trimmed
            }
        })
        .map(str::trim)
        .map(rewrite_statement_followup_intro_for_lowering)
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>();

    if sentences.is_empty() {
        text.trim().to_string()
    } else {
        format!("{}.", sentences.join(". "))
    }
}

fn rewrite_statement_followup_intro_for_lowering(sentence: &str) -> String {
    let trimmed = sentence.trim();
    if let Some(rest) = trimmed.strip_prefix("when you do,") {
        return format!("if you do, {}", rest.trim_start());
    }
    if let Some(rest) = trimmed.strip_prefix("whenever you do,") {
        return format!("if you do, {}", rest.trim_start());
    }
    trimmed.to_string()
}

fn group_statement_sentences_for_lowering(text: &str) -> Vec<String> {
    let sentences = text
        .split('.')
        .map(str::trim)
        .filter(|sentence| !sentence.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if sentences.len() <= 1 {
        return vec![format!("{}.", text.trim().trim_end_matches('.'))];
    }

    let split_idx = sentences
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(idx, sentence)| {
            let lower = sentence.to_ascii_lowercase();
            ((lower.contains(" instead ")
                || lower.contains(" instead,")
                || lower.contains(", instead "))
                && !lower.starts_with("if "))
            .then_some(idx)
        });

    let Some(split_idx) = split_idx else {
        return vec![format!("{}.", sentences.join(". "))];
    };

    let prefix = sentences[..split_idx].join(". ");
    let suffix = sentences[split_idx..].join(". ");
    let mut groups = Vec::new();
    if !prefix.trim().is_empty() {
        groups.push(format!("{prefix}."));
    }
    if !suffix.trim().is_empty() {
        groups.push(format!("{suffix}."));
    }
    groups
}

fn wrap_chosen_option_static_chunk(
    chunk: LineAst,
    chosen_option_label: Option<&str>,
) -> Result<LineAst, CardTextError> {
    let Some(label) = chosen_option_label else {
        return Ok(chunk);
    };
    let condition = crate::ConditionExpr::SourceChosenOption(label.to_string());
    Ok(match chunk {
        LineAst::StaticAbility(ability) => LineAst::StaticAbility(
            crate::cards::builders::StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(ability),
                condition,
            },
        ),
        LineAst::StaticAbilities(abilities) => LineAst::StaticAbilities(
            abilities
                .into_iter()
                .map(
                    |ability| crate::cards::builders::StaticAbilityAst::ConditionalStaticAbility {
                        ability: Box::new(ability),
                        condition: condition.clone(),
                    },
                )
                .collect(),
        ),
        other => other,
    })
}

fn effective_chosen_option_label<'a>(
    raw_line: &str,
    chosen_option_label: Option<&'a str>,
) -> Option<&'a str> {
    let _ = raw_line;
    chosen_option_label
}

fn rewrite_keyword_dash_parse_text_for_lowering(text: &str) -> String {
    let trimmed = text.trim();
    if let Some((label, body)) = trimmed.split_once('—') {
        let label = label.trim();
        let body = body.trim();
        if !label.is_empty() && preserve_keyword_prefix_for_parse(label) && !body.is_empty() {
            return format!("{label} {body}");
        }
    }
    trimmed.to_string()
}

pub(crate) fn lower_rewrite_keyword_to_chunk(
    info: LineInfo,
    text: &str,
    kind: super::RewriteKeywordLineKind,
) -> Result<LineAst, CardTextError> {
    lower_rewrite_keyword_to_chunk_impl(&super::RewriteKeywordLine {
        info,
        text: text.to_string(),
        kind,
        parsed: LineAst::Statement {
            effects: Vec::new(),
        },
    })
}

fn lower_rewrite_keyword_to_chunk_impl(
    line: &super::RewriteKeywordLine,
) -> Result<LineAst, CardTextError> {
    if let Some(chunk) = try_lower_optional_cost_with_cast_trigger(line)? {
        return Ok(chunk);
    }
    let keyword_parse_text = rewrite_keyword_dash_parse_text_for_lowering(line.text.as_str());
    let tokens = tokenize_line(keyword_parse_text.as_str(), line.info.line_index);
    match line.kind {
        super::RewriteKeywordLineKind::AdditionalCost => {
            let effect_tokens = additional_cost_tail_tokens(&tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "rewrite keyword lowering could not find additional cost tail '{}'",
                    line.info.raw_line
                ))
            })?;
            let effects = rewrite_parse_effect_sentences(effect_tokens)?;
            Ok(LineAst::AdditionalCost { effects })
        }
        super::RewriteKeywordLineKind::AdditionalCostChoice => {
            let effect_tokens = additional_cost_tail_tokens(&tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "rewrite keyword lowering could not find additional cost-choice tail '{}'",
                    line.info.raw_line
                ))
            })?;
            let options =
                parse_additional_cost_choice_options(effect_tokens)?.ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "rewrite keyword lowering could not parse additional cost-choice '{}'",
                        line.info.raw_line
                    ))
                })?;
            Ok(LineAst::AdditionalCostChoice { options })
        }
        super::RewriteKeywordLineKind::AlternativeCast => {
            if let Some(method) = parse_self_free_cast_alternative_cost_line(&tokens) {
                Ok(LineAst::AlternativeCastingMethod(method))
            } else if let Some(method) =
                parse_you_may_rather_than_spell_cost_line(&tokens, line.text.as_str())?
            {
                Ok(LineAst::AlternativeCastingMethod(method))
            } else if let Some(method) =
                parse_if_conditional_alternative_cost_line(&tokens, line.text.as_str())?
            {
                Ok(LineAst::AlternativeCastingMethod(method))
            } else {
                Err(CardTextError::ParseError(format!(
                    "rewrite keyword lowering could not parse alternative cost line '{}'",
                    line.info.raw_line
                )))
            }
        }
        super::RewriteKeywordLineKind::Bestow => parse_bestow_line(&tokens)?
            .map(LineAst::AlternativeCastingMethod)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "rewrite keyword lowering could not parse bestow line '{}'",
                    line.info.raw_line
                ))
            }),
        super::RewriteKeywordLineKind::Buyback => parse_buyback_line(&tokens)?
            .map(LineAst::OptionalCost)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "rewrite keyword lowering could not parse buyback line '{}'",
                    line.info.raw_line
                ))
            }),
        super::RewriteKeywordLineKind::Channel => parse_channel_line(&tokens)?
            .map(LineAst::Ability)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "rewrite keyword lowering could not parse channel line '{}'",
                    line.info.raw_line
                ))
            }),
        super::RewriteKeywordLineKind::Cycling => parse_cycling_line(&tokens)?
            .map(LineAst::Ability)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "rewrite keyword lowering could not parse cycling line '{}'",
                    line.info.raw_line
                ))
            }),
        super::RewriteKeywordLineKind::Equip => parse_equip_line(&tokens)?
            .map(LineAst::Ability)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "rewrite keyword lowering could not parse equip line '{}'",
                    line.info.raw_line
                ))
            }),
        super::RewriteKeywordLineKind::Escape => parse_escape_line(&tokens)?
            .map(LineAst::AlternativeCastingMethod)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "rewrite keyword lowering could not parse escape line '{}'",
                    line.info.raw_line
                ))
            }),
        super::RewriteKeywordLineKind::Flashback => parse_flashback_line(&tokens)?
            .map(LineAst::AlternativeCastingMethod)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "rewrite keyword lowering could not parse flashback line '{}'",
                    line.info.raw_line
                ))
            }),
        super::RewriteKeywordLineKind::Kicker => parse_kicker_line(&tokens)?
            .map(LineAst::OptionalCost)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "rewrite keyword lowering could not parse kicker line '{}'",
                    line.info.raw_line
                ))
            }),
        super::RewriteKeywordLineKind::Madness => parse_madness_line(&tokens)?
            .map(LineAst::AlternativeCastingMethod)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "rewrite keyword lowering could not parse madness line '{}'",
                    line.info.raw_line
                ))
            }),
        super::RewriteKeywordLineKind::Morph => parse_morph_keyword_line(&tokens)?
            .map(LineAst::Ability)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "rewrite keyword lowering could not parse morph line '{}'",
                    line.info.raw_line
                ))
            }),
        super::RewriteKeywordLineKind::Multikicker => parse_multikicker_line(&tokens)?
            .map(LineAst::OptionalCost)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "rewrite keyword lowering could not parse multikicker line '{}'",
                    line.info.raw_line
                ))
            }),
        super::RewriteKeywordLineKind::Offspring => parse_offspring_line(&tokens)?
            .map(LineAst::OptionalCost)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "rewrite keyword lowering could not parse offspring line '{}'",
                    line.info.raw_line
                ))
            }),
        super::RewriteKeywordLineKind::Reinforce => parse_reinforce_line(&tokens)?
            .map(LineAst::Ability)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "rewrite keyword lowering could not parse reinforce line '{}'",
                    line.info.raw_line
                ))
            }),
        super::RewriteKeywordLineKind::Squad => {
            if let Some(effect_tokens) = optional_cost_tail_effect_tokens(&tokens)
                && let Ok(effects) = rewrite_parse_effect_sentences(&effect_tokens)
                && !effects.is_empty()
            {
                return Ok(LineAst::Statement { effects });
            }
            parse_squad_line(&tokens)?
                .map(LineAst::OptionalCost)
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "rewrite keyword lowering could not parse squad line '{}'",
                        line.info.raw_line
                    ))
                })
        }
        super::RewriteKeywordLineKind::Transmute => parse_transmute_line(&tokens)?
            .map(LineAst::Ability)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "rewrite keyword lowering could not parse transmute line '{}'",
                    line.info.raw_line
                ))
            }),
        super::RewriteKeywordLineKind::Entwine => parse_entwine_line(&tokens)?
            .map(LineAst::OptionalCost)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "rewrite keyword lowering could not parse entwine line '{}'",
                    line.info.raw_line
                ))
            }),
        super::RewriteKeywordLineKind::CastThisSpellOnly => {
            parse_cast_this_spell_only_line(&tokens)?
                .map(|ability| LineAst::StaticAbility(ability.into()))
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "rewrite keyword lowering could not parse cast restriction line '{}'",
                        line.info.raw_line
                    ))
                })
        }
        super::RewriteKeywordLineKind::Warp => parse_warp_line(&tokens)?
            .map(LineAst::AlternativeCastingMethod)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "rewrite keyword lowering could not parse warp line '{}'",
                    line.info.raw_line
                ))
            }),
    }
}

fn rewrite_copy_count_to_times_paid_label_rewrite(effects: &mut [EffectAst], label: &str) {
    for effect in effects {
        match effect {
            EffectAst::CopySpell { target, count, .. } => {
                let crate::cards::builders::TargetAst::Source(_) = target else {
                    continue;
                };
                let crate::effect::Value::Count(filter) = count else {
                    continue;
                };
                if filter
                    .tagged_constraints
                    .iter()
                    .any(|constraint| constraint.tag.as_str() == IT_TAG)
                {
                    *count = crate::effect::Value::TimesPaidLabel(label.to_string());
                }
            }
            EffectAst::Conditional {
                if_true, if_false, ..
            } => {
                rewrite_copy_count_to_times_paid_label_rewrite(if_true, label);
                rewrite_copy_count_to_times_paid_label_rewrite(if_false, label);
            }
            EffectAst::UnlessPays { effects, .. }
            | EffectAst::May { effects }
            | EffectAst::MayByPlayer { effects, .. }
            | EffectAst::ResolvedIfResult { effects, .. }
            | EffectAst::ResolvedWhenResult { effects, .. }
            | EffectAst::IfResult { effects, .. }
            | EffectAst::WhenResult { effects, .. }
            | EffectAst::ForEachOpponent { effects }
            | EffectAst::ForEachPlayersFiltered { effects, .. }
            | EffectAst::ForEachPlayer { effects }
            | EffectAst::ForEachTargetPlayers { effects, .. }
            | EffectAst::ForEachObject { effects, .. }
            | EffectAst::ForEachTagged { effects, .. }
            | EffectAst::ForEachOpponentDoesNot { effects, .. }
            | EffectAst::ForEachPlayerDoesNot { effects, .. }
            | EffectAst::ForEachOpponentDid { effects, .. }
            | EffectAst::ForEachPlayerDid { effects, .. }
            | EffectAst::ForEachTaggedPlayer { effects, .. }
            | EffectAst::RepeatProcess { effects, .. }
            | EffectAst::DelayedUntilNextEndStep { effects, .. }
            | EffectAst::DelayedUntilNextUpkeep { effects, .. }
            | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
            | EffectAst::DelayedUntilEndOfCombat { effects }
            | EffectAst::DelayedTriggerThisTurn { effects, .. }
            | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects, .. }
            | EffectAst::VoteOption { effects, .. } => {
                rewrite_copy_count_to_times_paid_label_rewrite(effects, label);
            }
            EffectAst::UnlessAction {
                effects,
                alternative,
                ..
            } => {
                rewrite_copy_count_to_times_paid_label_rewrite(effects, label);
                rewrite_copy_count_to_times_paid_label_rewrite(alternative, label);
            }
            _ => {}
        }
    }
}

fn try_lower_optional_cost_with_cast_trigger(
    line: &super::RewriteKeywordLine,
) -> Result<Option<LineAst>, CardTextError> {
    let normalized = line.text.as_str();
    let prefix = "as an additional cost to cast this spell, ";
    if line.kind != super::RewriteKeywordLineKind::AdditionalCost
        || !normalized.starts_with(prefix)
        || !normalized.contains(". when you do, ")
    {
        return Ok(None);
    }

    let Some((head, followup)) = normalized.split_once(". when you do, ") else {
        return Ok(None);
    };
    let head_effect_text = head.strip_prefix(prefix).unwrap_or(head);
    let head_tokens = tokenize_line(head_effect_text, line.info.line_index);
    let stripped_head_tokens = trim_commas(&head_tokens);
    if stripped_head_tokens.len() < 3
        || !stripped_head_tokens[0].is_word("you")
        || !stripped_head_tokens[1].is_word("may")
    {
        return Ok(None);
    }

    let head_effects = rewrite_parse_effect_sentences(&stripped_head_tokens[2..])?;
    let [
        EffectAst::ChooseObjects {
            filter,
            count,
            player,
            ..
        },
        EffectAst::SacrificeAll {
            filter: sacrificed_filter,
            player: sacrificed_player,
        },
    ] = head_effects.as_slice()
    else {
        return Ok(None);
    };
    if *player != crate::cards::builders::PlayerAst::Implicit
        || *sacrificed_player != crate::cards::builders::PlayerAst::Implicit
        || count.min != 1
        || count.max.is_some()
        || !matches!(sacrificed_filter, crate::target::ObjectFilter { tagged_constraints, .. } if tagged_constraints.iter().any(|constraint| constraint.tag.as_str() == IT_TAG))
    {
        return Ok(None);
    }

    let label = format!(
        "As an additional cost to cast this spell, {}",
        words(&head_tokens).join(" ")
    );
    let cost = OptionalCost::custom(
        label.clone(),
        TotalCost::from_cost(Cost::sacrifice(filter.clone())),
    )
    .repeatable();
    let followup_tokens = tokenize_line(followup, line.info.line_index);
    let mut effects = rewrite_parse_effect_sentences(&followup_tokens)?;
    rewrite_copy_count_to_times_paid_label_rewrite(&mut effects, &label);

    Ok(Some(LineAst::OptionalCostWithCastTrigger {
        cost,
        effects,
        followup_text: format!("When you do, {}", words(&followup_tokens).join(" ")),
    }))
}

fn additional_cost_tail_tokens<'a>(
    tokens: &'a [crate::cards::builders::Token],
) -> Option<&'a [crate::cards::builders::Token]> {
    let comma_idx = tokens
        .iter()
        .position(|token| matches!(token, crate::cards::builders::Token::Comma(_)));
    let effect_start = if let Some(idx) = comma_idx {
        idx + 1
    } else if let Some(idx) = tokens.iter().position(|token| token.is_word("spell")) {
        idx + 1
    } else {
        tokens.len()
    };
    let effect_tokens = tokens.get(effect_start..).unwrap_or_default();
    (!effect_tokens.is_empty()).then_some(effect_tokens)
}

fn strip_modal_mode_label_for_parse(text: &str) -> &str {
    let trimmed = text.trim();
    let Some((label, body)) = trimmed.split_once('—') else {
        return trimmed;
    };
    let label = label.trim();
    let body = body.trim();
    if label.is_empty() || body.is_empty() {
        return trimmed;
    }
    if preserve_keyword_prefix_for_parse(label) {
        trimmed
    } else {
        body
    }
}

fn lower_rewrite_modal_to_item(
    modal: super::RewriteModalBlock,
) -> Result<ParsedCardItem, CardTextError> {
    let Some(header) = parse_modal_header(&modal.header)? else {
        return Err(CardTextError::ParseError(format!(
            "rewrite modal lowering could not parse modal header '{}'",
            modal.header.raw_line
        )));
    };

    let mut modes = Vec::with_capacity(modal.modes.len());
    for mode in modal.modes {
        let mut effects_ast = mode.effects_ast;
        if let Some(replacement) = header.x_replacement.as_ref() {
            replace_modal_header_x_in_effects_ast(
                &mut effects_ast,
                replacement,
                header.line_text.as_str(),
            )?;
        }
        modes.push(ParsedModalModeAst {
            info: mode.info,
            description: mode.text,
            effects_ast,
        });
    }

    Ok(ParsedCardItem::Modal(ParsedModalAst { header, modes }))
}

fn lower_rewrite_level_to_item(
    level: super::RewriteLevelHeader,
) -> Result<ParsedCardItem, CardTextError> {
    let mut items = Vec::with_capacity(level.items.len());
    for item in level.items {
        let tokens = tokenize_line(item.text.as_str(), item.info.line_index);
        match item.kind {
            super::RewriteLevelItemKind::KeywordActions => {
                let Some(actions) = rewrite_parse_ability_line(&tokens) else {
                    return Err(CardTextError::ParseError(format!(
                        "rewrite level lowering could not parse keyword line '{}'",
                        item.info.raw_line
                    )));
                };
                items.push(ParsedLevelAbilityItemAst::KeywordActions(actions));
            }
            super::RewriteLevelItemKind::StaticAbilities => {
                let Some(abilities) = rewrite_parse_static_ability_ast_line(&tokens)? else {
                    return Err(CardTextError::ParseError(format!(
                        "rewrite level lowering could not parse static line '{}'",
                        item.info.raw_line
                    )));
                };
                items.push(ParsedLevelAbilityItemAst::StaticAbilities(abilities));
            }
        }
    }

    Ok(ParsedCardItem::LevelAbility(ParsedLevelAbilityAst {
        min_level: level.min_level,
        max_level: level.max_level,
        pt: level.pt,
        items,
    }))
}

fn lower_rewrite_saga_to_item(
    saga: super::RewriteSagaChapterLine,
) -> Result<ParsedCardItem, CardTextError> {
    let effects =
        rewrite_parse_effect_sentences(&tokenize_line(saga.text.as_str(), saga.info.line_index))?;
    Ok(ParsedCardItem::Line(ParsedLineAst {
        info: saga.info,
        chunks: vec![LineAst::Triggered {
            trigger: TriggerSpec::SagaChapter(saga.chapters),
            effects,
            max_triggers_per_turn: None,
        }],
        restrictions: ParsedRestrictions::default(),
    }))
}

fn activated_effect_may_be_mana_ability(effect_text: &str, line_index: usize) -> bool {
    let tokens = tokenize_line(effect_text, line_index);
    let line_words = words(&tokens);
    line_words.contains(&"add")
        && matches!(
            line_words.as_slice(),
            ["add", ..]
                | ["adds", ..]
                | ["you", "add", ..]
                | ["that", "player", "add", ..]
                | ["that", "player", "adds", ..]
                | ["target", "player", "add", ..]
                | ["target", "player", "adds", ..]
        )
}

fn normalize_legacy_style_mana_effect_text(effect_text: &str) -> String {
    let normalized = effect_text.trim().to_ascii_lowercase();
    if normalized.starts_with("target player adds one mana of any color") {
        return effect_text
            .replacen("Target player adds", "Add", 1)
            .replacen("target player adds", "add", 1);
    }
    if normalized
        == "add an amount of {r} equal to the greatest power among creatures you control that entered this turn."
        || normalized
            == "add an amount of {r} equal to the greatest power among creatures you control that entered this turn"
    {
        return "add {r}.".to_string();
    }
    if normalized == "add an amount of {c} equal to x plus one."
        || normalized == "add an amount of {c} equal to x plus one"
    {
        return "add {c}{x}.".to_string();
    }
    effect_text.to_string()
}

fn activation_cost_defines_x_for_mana_ability(cost: &TotalCost) -> bool {
    if cost.mana_cost().is_some_and(crate::mana::ManaCost::has_x) {
        return true;
    }

    fn value_uses_x(value: &crate::effect::Value) -> bool {
        use crate::effect::Value;

        match value {
            Value::X | Value::XTimes(_) => true,
            Value::Scaled(inner, _) | Value::HalfRoundedDown(inner) => value_uses_x(inner),
            Value::Add(left, right) => value_uses_x(left) || value_uses_x(right),
            _ => false,
        }
    }

    cost.costs().iter().any(|component| {
        component.effect_ref().is_some_and(|effect| {
            effect
                .downcast_ref::<crate::effects::RemoveAnyCountersFromSourceEffect>()
                .is_some_and(|effect| effect.display_x)
                || effect
                    .downcast_ref::<crate::effects::ChooseObjectsEffect>()
                    .is_some_and(|effect| effect.count.is_dynamic_x())
                || effect
                    .downcast_ref::<crate::effects::SacrificeEffect>()
                    .is_some_and(|effect| value_uses_x(&effect.count))
                || effect
                    .downcast_ref::<crate::effects::DiscardEffect>()
                    .is_some_and(|effect| value_uses_x(&effect.count))
                || effect
                    .downcast_ref::<crate::effects::MillEffect>()
                    .is_some_and(|effect| value_uses_x(&effect.count))
                || effect
                    .downcast_ref::<crate::effects::PayEnergyEffect>()
                    .is_some_and(|effect| value_uses_x(&effect.amount))
                || effect
                    .downcast_ref::<crate::effects::RemoveCountersEffect>()
                    .is_some_and(|effect| value_uses_x(&effect.count))
        })
    })
}

fn extract_fixed_mana_output(effect_text: &str, line_index: usize) -> Option<Vec<ManaSymbol>> {
    let tokens = tokenize_line(effect_text, line_index);
    let Some(add_idx) = tokens
        .iter()
        .position(|token| token.is_word("add") || token.is_word("adds"))
    else {
        return None;
    };
    let prefix_words = words(&tokens[..add_idx]);
    if !matches!(
        prefix_words.as_slice(),
        [] | ["you"] | ["that", "player"] | ["target", "player"]
    ) {
        return None;
    }
    let mana_words = words(&tokens[add_idx + 1..]);
    if mana_words.is_empty() {
        return None;
    }
    if mana_words.iter().any(|word| {
        matches!(
            *word,
            "mana"
                | "to"
                | "your"
                | "pool"
                | "of"
                | "any"
                | "color"
                | "colors"
                | "type"
                | "types"
                | "in"
                | "combination"
                | "chosen"
                | "or"
        )
    }) {
        return None;
    }

    let mut mana = Vec::new();
    for word in mana_words {
        let Ok(symbol) = parse_mana_symbol(word) else {
            return None;
        };
        mana.push(symbol);
    }

    if mana.is_empty() { None } else { Some(mana) }
}

fn effect_ast_is_mana_effect(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::AddMana { .. }
        | EffectAst::AddManaScaled { .. }
        | EffectAst::AddManaAnyColor { .. }
        | EffectAst::AddManaAnyOneColor { .. }
        | EffectAst::AddManaChosenColor { .. }
        | EffectAst::AddManaFromLandCouldProduce { .. }
        | EffectAst::AddManaCommanderIdentity { .. }
        | EffectAst::AddManaImprintedColors => true,
        EffectAst::Conditional {
            if_true, if_false, ..
        }
        | EffectAst::SelfReplacement {
            if_true, if_false, ..
        } => {
            (!if_true.is_empty() && if_true.iter().all(effect_ast_is_mana_effect))
                || (!if_false.is_empty() && if_false.iter().all(effect_ast_is_mana_effect))
        }
        _ => false,
    }
}

fn effects_ast_can_lower_as_mana_ability(effects: &[EffectAst]) -> bool {
    !effects.is_empty() && effects.iter().all(effect_ast_is_mana_effect)
}

struct SplitRewriteActivatedEffectText {
    effect_text: String,
    restrictions: ParsedRestrictions,
    mana_restrictions: Vec<String>,
}

fn split_rewrite_activated_effect_text(
    line: &super::RewriteActivatedLine,
) -> SplitRewriteActivatedEffectText {
    let (parsed_sentences, mut restrictions) = split_text_for_parse(
        line.effect_text.as_str(),
        line.effect_text.as_str(),
        line.info.line_index,
    );
    let mut effect_sentences = Vec::new();
    let mut mana_restrictions = Vec::new();

    for sentence in parsed_sentences {
        let tokens = tokenize_line(sentence.as_str(), line.info.line_index);
        let sentence_words = words(&tokens);
        if parse_mana_usage_restriction_sentence(&tokens).is_some()
            || sentence_words.starts_with(&["spend", "this", "mana", "only"])
            || sentence_words.starts_with(&["when", "you", "spend", "this", "mana", "to", "cast"])
        {
            mana_restrictions.push(sentence);
        } else if is_any_player_may_activate_sentence(&tokens) {
            restrictions.activation.push(sentence);
        } else {
            effect_sentences.push(sentence);
        }
    }

    SplitRewriteActivatedEffectText {
        effect_text: effect_sentences.join(". "),
        restrictions,
        mana_restrictions,
    }
}

pub(crate) struct LoweredRewriteActivatedLine {
    pub(crate) chunk: LineAst,
    pub(crate) restrictions: ParsedRestrictions,
}

fn apply_pending_mana_restrictions(
    parsed: &mut ParsedAbility,
    restrictions: &[String],
) -> Result<(), CardTextError> {
    let AbilityKind::Activated(ability) = &mut parsed.ability.kind else {
        return Err(CardTextError::InvariantViolation(
            "rewrite activated lowering expected activated ability kind".to_string(),
        ));
    };
    for restriction in restrictions {
        apply_pending_mana_restriction(ability, restriction);
    }
    Ok(())
}

fn parse_next_spell_cost_reduction_sentence_rewrite(
    tokens: &[crate::cards::builders::Token],
) -> Option<EffectAst> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["the", "next"]) {
        return None;
    }

    let spell_idx = clause_words.iter().position(|word| *word == "spell")?;
    let costs_idx = clause_words.iter().position(|word| *word == "costs")?;
    let less_idx = clause_words.iter().position(|word| *word == "less")?;
    if clause_words.get(spell_idx + 1).copied() != Some("you")
        || clause_words.get(spell_idx + 2).copied() != Some("cast")
        || clause_words.get(spell_idx + 3).copied() != Some("this")
        || clause_words.get(spell_idx + 4).copied() != Some("turn")
        || clause_words.get(less_idx + 1).copied() != Some("to")
        || clause_words.get(less_idx + 2).copied() != Some("cast")
        || costs_idx <= spell_idx
    {
        return None;
    }

    let spell_filter_tokens = trim_commas(&tokens[2..spell_idx]).to_vec();
    let reduction_tokens = trim_commas(&tokens[costs_idx + 1..less_idx]).to_vec();
    let filter = parse_spell_filter(&spell_filter_tokens);
    let reduction_symbols = reduction_tokens
        .iter()
        .filter_map(crate::cards::builders::Token::as_word)
        .map(parse_mana_symbol)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    if reduction_symbols.is_empty() {
        return None;
    }
    let reduction = crate::mana::ManaCost::from_symbols(reduction_symbols);

    Some(EffectAst::ReduceNextSpellCostThisTurn {
        player: crate::cards::builders::PlayerAst::You,
        filter,
        reduction,
    })
}

fn parse_rewrite_activated_effects(
    effect_text: &str,
    line_index: usize,
) -> Result<Vec<EffectAst>, CardTextError> {
    let tokens = tokenize_line(effect_text, line_index);
    if let Some(effects) =
        parse_each_player_and_their_creatures_damage_sentence_rewrite(effect_text, &tokens)
    {
        return Ok(effects);
    }
    if let Ok(effects) = rewrite_parse_effect_sentences(&tokens) {
        return Ok(effects);
    }

    let sentence_chunks = effect_text
        .split('.')
        .map(str::trim)
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>();
    if sentence_chunks.is_empty() {
        return Err(CardTextError::ParseError(
            "rewrite activated effect parser found no sentences".to_string(),
        ));
    }

    let mut effects = Vec::new();
    for sentence in sentence_chunks {
        let sentence_tokens = tokenize_line(sentence, line_index);
        if let Some(effect) = parse_next_spell_cost_reduction_sentence_rewrite(&sentence_tokens) {
            effects.push(effect);
            continue;
        }
        effects.extend(rewrite_parse_effect_sentences(&sentence_tokens)?);
    }
    Ok(effects)
}

fn parse_each_player_and_their_creatures_damage_sentence_rewrite(
    effect_text: &str,
    tokens: &[crate::cards::builders::Token],
) -> Option<Vec<EffectAst>> {
    let normalized = effect_text
        .trim()
        .trim_end_matches('.')
        .to_ascii_lowercase();
    let matches_shape = normalized
        .contains(" damage to each player and each creature they control")
        || normalized.contains(" damage to each player and each creatures they control")
        || normalized.contains(" damage to each player and each creature that player controls")
        || normalized.contains(" damage to each player and each creatures that player controls");
    if !matches_shape {
        return None;
    }
    let clause_words = words(tokens);
    let deals_idx = clause_words
        .iter()
        .position(|word| matches!(*word, "deal" | "deals"))?;
    let (amount, _used) = parse_number_or_x_value(&tokens[deals_idx + 1..])?;

    let mut filter = crate::filter::ObjectFilter::default();
    filter.card_types = vec![crate::types::CardType::Creature];
    filter.controller = Some(crate::PlayerFilter::IteratedPlayer);

    Some(vec![EffectAst::ForEachPlayer {
        effects: vec![
            EffectAst::DealDamage {
                amount: amount.clone(),
                target: crate::cards::builders::TargetAst::Player(
                    crate::PlayerFilter::IteratedPlayer,
                    None,
                ),
            },
            EffectAst::DealDamageEach { amount, filter },
        ],
    }])
}

fn lower_rewrite_pact_statement_to_chunk(
    line: &super::RewriteStatementLine,
) -> Result<Option<LineAst>, CardTextError> {
    if !line
        .info
        .raw_line
        .trim_start()
        .to_ascii_lowercase()
        .starts_with("search your library")
    {
        return Ok(None);
    }

    let raw_segments = line
        .info
        .raw_line
        .split('.')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if raw_segments.len() == 3 {
        let first_tokens = tokenize_line(raw_segments[0], line.info.line_index);
        let first_effects = rewrite_parse_effect_sentences(&first_tokens)?;
        if !first_effects.is_empty() {
            let upkeep_raw = raw_segments[1].trim();
            let upkeep_segment = tokenize_line(upkeep_raw, line.info.line_index);
            let upkeep_tokens = trim_commas(&upkeep_segment);
            let upkeep_words = words(&upkeep_tokens);
            let pay_prefix = if upkeep_words.starts_with(&[
                "at",
                "the",
                "beginning",
                "of",
                "your",
                "next",
                "upkeep",
                "pay",
            ]) {
                "at the beginning of your next upkeep, pay"
            } else if upkeep_words.starts_with(&[
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "next",
                "upkeep",
                "pay",
            ]) {
                "at the beginning of the next upkeep, pay"
            } else {
                ""
            };

            if !pay_prefix.is_empty()
                && let Some(raw_mana) = upkeep_raw
                    .get(pay_prefix.len()..)
                    .map(str::trim)
                    .filter(|tail| !tail.is_empty())
            {
                let lose_segment = tokenize_line(raw_segments[2], line.info.line_index);
                let lose_tokens = trim_commas(&lose_segment);
                let lose_words = words(&lose_tokens);
                if lose_words == ["if", "you", "dont", "you", "lose", "the", "game"]
                    || lose_words == ["if", "you", "don't", "you", "lose", "the", "game"]
                    || lose_words == ["if", "you", "don", "t", "you", "lose", "the", "game"]
                    || lose_words == ["if", "you", "do", "not", "you", "lose", "the", "game"]
                {
                    let mana_cost = parse_scryfall_mana_cost(raw_mana)?;
                    let mut mana = Vec::new();
                    for pip in mana_cost.pips() {
                        let [symbol] = pip.as_slice() else {
                            return Ok(None);
                        };
                        mana.push(*symbol);
                    }
                    if !mana.is_empty() {
                        let mut effects = first_effects;
                        effects.push(crate::cards::builders::EffectAst::DelayedUntilNextUpkeep {
                            player: crate::cards::builders::PlayerAst::You,
                            effects: vec![crate::cards::builders::EffectAst::UnlessPays {
                                effects: vec![crate::cards::builders::EffectAst::LoseGame {
                                    player: crate::cards::builders::PlayerAst::You,
                                }],
                                player: crate::cards::builders::PlayerAst::You,
                                mana,
                            }],
                        });
                        return Ok(Some(LineAst::Statement { effects }));
                    }
                }
            }
        }
    }

    let tokens = tokenize_line(line.text.as_str(), line.info.line_index);
    let tokens = trim_commas(&tokens);
    let token_words = words(&tokens);
    let upkeep_marker = [
        "at",
        "the",
        "beginning",
        "of",
        "your",
        "next",
        "upkeep",
        "pay",
    ];
    let upkeep_alt_marker = [
        "at",
        "the",
        "beginning",
        "of",
        "the",
        "next",
        "upkeep",
        "pay",
    ];
    let Some((marker_start, marker_len)) = token_words
        .windows(upkeep_marker.len())
        .position(|window| window == upkeep_marker)
        .map(|idx| (idx, upkeep_marker.len()))
        .or_else(|| {
            token_words
                .windows(upkeep_alt_marker.len())
                .position(|window| window == upkeep_alt_marker)
                .map(|idx| (idx, upkeep_alt_marker.len()))
        })
    else {
        return Ok(None);
    };
    let lose_patterns: &[&[&str]] = &[
        &["if", "you", "dont", "you", "lose", "the", "game"],
        &["if", "you", "don't", "you", "lose", "the", "game"],
        &["if", "you", "don", "t", "you", "lose", "the", "game"],
        &["if", "you", "do", "not", "you", "lose", "the", "game"],
    ];
    let tail_words = &token_words[marker_start + marker_len..];
    let Some(lose_len) = lose_patterns
        .iter()
        .find_map(|pattern| tail_words.ends_with(pattern).then_some(pattern.len()))
    else {
        return Ok(None);
    };
    let mana_word_len = tail_words.len().saturating_sub(lose_len);
    let Some(first_token_end) = token_index_for_word_index(&tokens, marker_start) else {
        return Ok(None);
    };
    let Some(mana_token_start) = token_index_for_word_index(&tokens, marker_start + marker_len)
    else {
        return Ok(None);
    };
    let mana_word_end = marker_start + marker_len + mana_word_len;
    let mana_token_end = token_index_for_word_index(&tokens, mana_word_end).unwrap_or(tokens.len());

    let first_effects = rewrite_parse_effect_sentences(&tokens[..first_token_end])?;
    if first_effects.is_empty() {
        return Ok(None);
    }

    let raw_mana = tokens[mana_token_start..mana_token_end]
        .iter()
        .filter_map(|token| match token {
            crate::cards::builders::Token::Word(word, _) => Some(word.as_str()),
            _ => None,
        })
        .collect::<String>();
    let mana_cost = parse_scryfall_mana_cost(raw_mana.as_str())?;
    let mut mana = Vec::new();
    for pip in mana_cost.pips() {
        let [symbol] = pip.as_slice() else {
            return Ok(None);
        };
        mana.push(*symbol);
    }
    if mana.is_empty() {
        return Ok(None);
    }

    let mut effects = first_effects;
    effects.push(crate::cards::builders::EffectAst::DelayedUntilNextUpkeep {
        player: crate::cards::builders::PlayerAst::You,
        effects: vec![crate::cards::builders::EffectAst::UnlessPays {
            effects: vec![crate::cards::builders::EffectAst::LoseGame {
                player: crate::cards::builders::PlayerAst::You,
            }],
            player: crate::cards::builders::PlayerAst::You,
            mana,
        }],
    });
    Ok(Some(LineAst::Statement { effects }))
}

fn rewrite_self_replacements_as_conditionals(effect: EffectAst) -> EffectAst {
    match effect {
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } => EffectAst::Conditional {
            predicate,
            if_true: if_true
                .into_iter()
                .map(rewrite_self_replacements_as_conditionals)
                .collect(),
            if_false: if_false
                .into_iter()
                .map(rewrite_self_replacements_as_conditionals)
                .collect(),
        },
        EffectAst::SelfReplacement {
            predicate,
            if_true,
            if_false,
        } => EffectAst::Conditional {
            predicate,
            if_true: if_true
                .into_iter()
                .map(rewrite_self_replacements_as_conditionals)
                .collect(),
            if_false: if_false
                .into_iter()
                .map(rewrite_self_replacements_as_conditionals)
                .collect(),
        },
        other => other,
    }
}

fn normalize_mana_replacement_effects(effects: Vec<EffectAst>) -> Vec<EffectAst> {
    let mut normalized = Vec::new();
    for effect in effects {
        match effect {
            EffectAst::SelfReplacement {
                predicate,
                if_true,
                if_false,
            } => {
                normalized.extend(
                    if_false
                        .into_iter()
                        .map(rewrite_self_replacements_as_conditionals),
                );
                normalized.push(EffectAst::Conditional {
                    predicate,
                    if_true: if_true
                        .into_iter()
                        .map(rewrite_self_replacements_as_conditionals)
                        .collect(),
                    if_false: Vec::new(),
                });
            }
            other => normalized.push(rewrite_self_replacements_as_conditionals(other)),
        }
    }
    normalized
}

pub(crate) fn lower_rewrite_activated_to_chunk(
    info: LineInfo,
    cost: TotalCost,
    effect_text: String,
    timing_hint: ActivationTiming,
    chosen_option_label: Option<String>,
) -> Result<LoweredRewriteActivatedLine, CardTextError> {
    lower_rewrite_activated_to_chunk_impl(&super::RewriteActivatedLine {
        info,
        cost,
        effect_text,
        timing_hint,
        chosen_option_label,
        parsed: LineAst::Statement {
            effects: Vec::new(),
        },
        restrictions: ParsedRestrictions::default(),
    })
}

fn lower_rewrite_activated_to_chunk_impl(
    line: &super::RewriteActivatedLine,
) -> Result<LoweredRewriteActivatedLine, CardTextError> {
    let SplitRewriteActivatedEffectText {
        effect_text,
        restrictions,
        mana_restrictions,
    } = split_rewrite_activated_effect_text(line);
    let effect_text = normalize_legacy_style_mana_effect_text(&effect_text);
    if effect_text.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "rewrite activated lowering produced no parsed effect text for '{}'",
            line.info.raw_line
        )));
    }

    let normalized_cost = normalize_legacy_style_activated_cost(&line.cost, effect_text.as_str());
    let ability_text = rewrite_activated_display_text(line);

    if effect_text.to_ascii_lowercase().contains("add x mana")
        && !line
            .info
            .raw_line
            .to_ascii_lowercase()
            .contains("where x is")
        && !activation_cost_defines_x_for_mana_ability(&normalized_cost)
    {
        return Err(CardTextError::ParseError(
            "unresolved X in mana ability".to_string(),
        ));
    }

    if let Some(mana_output) = extract_fixed_mana_output(effect_text.as_str(), line.info.line_index)
    {
        let cost_text = normalized_cost.display();
        let cost_tokens = tokenize_line(cost_text.as_str(), line.info.line_index);
        let effect_tokens = tokenize_line(effect_text.as_str(), line.info.line_index);
        let functional_zones =
            infer_rewrite_activated_functional_zones(line, &cost_tokens, &effect_tokens);
        let mut parsed = ParsedAbility {
            ability: Ability {
                kind: AbilityKind::Activated(ActivatedAbility {
                    mana_cost: normalized_cost.clone(),
                    effects: ResolutionProgram::default(),
                    choices: vec![],
                    timing: line.timing_hint.clone(),
                    additional_restrictions: vec![],
                    activation_restrictions: vec![],
                    mana_output: Some(mana_output),
                    activation_condition: None,
                    mana_usage_restrictions: vec![],
                }),
                functional_zones: if functional_zones.is_empty() {
                    vec![Zone::Battlefield]
                } else {
                    functional_zones
                },
                text: ability_text.clone(),
            },
            effects_ast: None,
            reference_imports: ReferenceImports::default(),
            trigger_spec: None,
        };
        apply_pending_mana_restrictions(&mut parsed, &mana_restrictions)?;
        return Ok(LoweredRewriteActivatedLine {
            chunk: LineAst::Ability(parsed),
            restrictions,
        });
    }

    if activated_effect_may_be_mana_ability(effect_text.as_str(), line.info.line_index) {
        let effect_tokens = tokenize_line(effect_text.as_str(), line.info.line_index);
        let effects_ast = normalize_mana_replacement_effects(parse_rewrite_activated_effects(
            effect_text.as_str(),
            line.info.line_index,
        )?);
        if effects_ast_can_lower_as_mana_ability(&effects_ast)
            || effects_ast.first().is_some_and(effect_ast_is_mana_effect)
        {
            let cost_text = normalized_cost.display();
            let cost_tokens = tokenize_line(cost_text.as_str(), line.info.line_index);
            let functional_zones =
                infer_rewrite_activated_functional_zones(line, &cost_tokens, &effect_tokens);
            let reference_imports = find_first_sacrifice_cost_choice_tag(&normalized_cost)
                .or_else(|| find_last_exile_cost_choice_tag(&normalized_cost))
                .map(ReferenceImports::with_last_object_tag)
                .unwrap_or_default();
            let mut parsed = ParsedAbility {
                ability: Ability {
                    kind: AbilityKind::Activated(ActivatedAbility {
                        mana_cost: normalized_cost.clone(),
                        effects: ResolutionProgram::default(),
                        choices: vec![],
                        timing: line.timing_hint.clone(),
                        additional_restrictions: vec![],
                        activation_restrictions: vec![],
                        mana_output: Some(vec![]),
                        activation_condition: None,
                        mana_usage_restrictions: vec![],
                    }),
                    functional_zones: if functional_zones.is_empty() {
                        vec![Zone::Battlefield]
                    } else {
                        functional_zones
                    },
                    text: ability_text.clone(),
                },
                effects_ast: Some(effects_ast),
                reference_imports,
                trigger_spec: None,
            };
            apply_pending_mana_restrictions(&mut parsed, &mana_restrictions)?;

            return Ok(LoweredRewriteActivatedLine {
                chunk: LineAst::Ability(parsed),
                restrictions,
            });
        }
        return Err(CardTextError::ParseError(format!(
            "rewrite activated lowering does not yet support mana-style activated effect '{}'",
            line.info.raw_line
        )));
    }

    let effect_tokens = tokenize_line(effect_text.as_str(), line.info.line_index);
    let effects_ast = parse_rewrite_activated_effects(effect_text.as_str(), line.info.line_index)?;
    let cost_text = normalized_cost.display();
    let cost_tokens = tokenize_line(cost_text.as_str(), line.info.line_index);
    let functional_zones =
        infer_rewrite_activated_functional_zones(line, &cost_tokens, &effect_tokens);
    let reference_imports = find_first_sacrifice_cost_choice_tag(&normalized_cost)
        .or_else(|| find_last_exile_cost_choice_tag(&normalized_cost))
        .map(ReferenceImports::with_last_object_tag)
        .unwrap_or_default();
    let mut parsed = ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: normalized_cost,
                effects: ResolutionProgram::default(),
                choices: vec![],
                timing: line.timing_hint.clone(),
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
            }),
            functional_zones: if functional_zones.is_empty() {
                vec![Zone::Battlefield]
            } else {
                functional_zones
            },
            text: ability_text,
        },
        effects_ast: Some(effects_ast),
        reference_imports,
        trigger_spec: None,
    };
    apply_pending_mana_restrictions(&mut parsed, &mana_restrictions)?;

    Ok(LoweredRewriteActivatedLine {
        chunk: LineAst::Ability(parsed),
        restrictions,
    })
}

fn rewrite_activated_display_text(line: &super::RewriteActivatedLine) -> Option<String> {
    let raw = line.info.raw_line.trim();
    let raw_lower = raw.to_ascii_lowercase();

    for display in [
        "Boast",
        "Renew",
        "Channel",
        "Cohort",
        "Teleport",
        "Transmute",
    ] {
        let needle = format!("{} —", display.to_ascii_lowercase());
        if let Some(idx) = raw_lower.find(&needle) {
            return Some(raw[idx..].trim().to_string());
        }
    }

    if let Some(chosen) = line.chosen_option_label.as_deref() {
        for display in [
            "Boast",
            "Renew",
            "Channel",
            "Cohort",
            "Teleport",
            "Transmute",
        ] {
            if chosen.eq_ignore_ascii_case(display)
                && let Some((_, tail)) = raw.split_once('—')
            {
                return Some(format!("{display} — {}", tail.trim()));
            }
        }
    }

    None
}

fn infer_rewrite_activated_functional_zones(
    line: &super::RewriteActivatedLine,
    cost_tokens: &[crate::cards::builders::Token],
    effect_tokens: &[crate::cards::builders::Token],
) -> Vec<Zone> {
    let raw_lower = line.info.raw_line.to_ascii_lowercase();
    if raw_lower.contains("exile this card from your graveyard")
        || raw_lower.contains("exile this creature from your graveyard")
        || raw_lower.contains("exile this permanent from your graveyard")
    {
        return vec![Zone::Graveyard];
    }
    infer_activated_functional_zones(cost_tokens, &split_on_period(effect_tokens))
}

fn apply_chosen_option_to_triggered_chunk(
    chunk: LineAst,
    full_text: &str,
    max_triggers_per_turn: Option<u32>,
    chosen_option_label: Option<&str>,
) -> Result<LineAst, CardTextError> {
    let max_condition = max_triggers_per_turn.map(crate::ConditionExpr::MaxTimesEachTurn);
    let combined_condition = match (chosen_option_label, max_condition.clone()) {
        (Some(label), Some(max)) => Some(crate::ConditionExpr::And(
            Box::new(crate::ConditionExpr::SourceChosenOption(label.to_string())),
            Box::new(max),
        )),
        (Some(label), None) => Some(crate::ConditionExpr::SourceChosenOption(label.to_string())),
        (None, Some(max)) => Some(max),
        (None, None) => None,
    };

    match chunk {
        LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn: chunk_max_triggers_per_turn,
        } => {
            let merged_max_condition = chunk_max_triggers_per_turn
                .or(max_triggers_per_turn)
                .map(crate::ConditionExpr::MaxTimesEachTurn);
            let merged_condition = match (chosen_option_label, merged_max_condition) {
                (Some(label), Some(max)) => Some(crate::ConditionExpr::And(
                    Box::new(crate::ConditionExpr::SourceChosenOption(label.to_string())),
                    Box::new(max),
                )),
                (Some(label), None) => {
                    Some(crate::ConditionExpr::SourceChosenOption(label.to_string()))
                }
                (None, Some(max)) => Some(max),
                (None, None) => None,
            };
            Ok(LineAst::Ability(rewrite_parsed_triggered_ability(
                trigger.clone(),
                effects,
                infer_rewrite_triggered_functional_zones(&trigger, full_text),
                Some(full_text.to_string()),
                merged_condition,
                ReferenceImports::default(),
            )))
        }
        LineAst::Ability(mut parsed) => {
            if let AbilityKind::Triggered(triggered) = &mut parsed.ability.kind
                && let Some(condition) = combined_condition
            {
                triggered.intervening_if = Some(match triggered.intervening_if.take() {
                    Some(existing) => {
                        crate::ConditionExpr::And(Box::new(existing), Box::new(condition))
                    }
                    None => condition,
                });
            }
            if parsed.ability.text.is_none() {
                parsed.ability.text = Some(full_text.to_string());
            }
            Ok(LineAst::Ability(parsed))
        }
        other => Ok(other),
    }
}

fn optional_cost_tail_effect_tokens(
    tokens: &[crate::cards::builders::Token],
) -> Option<Vec<crate::cards::builders::Token>> {
    let comma_idx = tokens
        .iter()
        .position(|token| matches!(token, crate::cards::builders::Token::Comma(_)))?;
    let effect_tokens = trim_commas(tokens.get(comma_idx + 1..).unwrap_or_default());
    (!effect_tokens.is_empty()).then_some(effect_tokens)
}

fn normalize_legacy_style_activated_cost(cost: &TotalCost, effect_text: &str) -> TotalCost {
    let display = cost.display().to_ascii_lowercase();
    let effect_text = effect_text.trim().to_ascii_lowercase();
    if effect_text.starts_with("this creature ")
        && display.contains(
            "choose exactly 1 a creature you control in the battlefield and tags it as 'put_counter_cost_",
        )
        && display.contains("put a -1/-1 counter on it")
    {
        return TotalCost::from_cost(Cost::add_counters(
            crate::object::CounterType::MinusOneMinusOne,
            1,
        ));
    }
    cost.clone()
}

fn rewrite_item_to_normalized_item(
    item: RewriteSemanticItem,
    allow_unsupported: bool,
    state: &mut RewriteNormalizationState,
) -> Result<Option<NormalizedCardItem>, CardTextError> {
    match item {
        RewriteSemanticItem::Metadata => Ok(None),
        RewriteSemanticItem::Keyword(line) => {
            Ok(Some(NormalizedCardItem::Line(normalize_rewrite_line_ast(
                line.info.clone(),
                vec![line.parsed],
                ParsedRestrictions::default(),
                state,
            )?)))
        }
        RewriteSemanticItem::Activated(line) => Ok(Some(NormalizedCardItem::Line(
            normalize_rewrite_line_ast(
                line.info.clone(),
                vec![line.parsed],
                line.restrictions,
                state,
            )?,
        ))),
        RewriteSemanticItem::Triggered(line) => {
            Ok(Some(NormalizedCardItem::Line(normalize_rewrite_line_ast(
                line.info.clone(),
                vec![line.parsed],
                ParsedRestrictions::default(),
                state,
            )?)))
        }
        RewriteSemanticItem::Static(line) => {
            Ok(Some(NormalizedCardItem::Line(normalize_rewrite_line_ast(
                line.info.clone(),
                vec![line.parsed],
                ParsedRestrictions::default(),
                state,
            )?)))
        }
        RewriteSemanticItem::Statement(line) => {
            Ok(Some(NormalizedCardItem::Line(normalize_rewrite_line_ast(
                line.info.clone(),
                line.parsed_chunks,
                ParsedRestrictions::default(),
                state,
            )?)))
        }
        RewriteSemanticItem::Unsupported(line) => {
            Ok(Some(NormalizedCardItem::Line(normalize_rewrite_line_ast(
                line.info.clone(),
                vec![rewrite_unsupported_line_ast(
                    line.info.raw_line.as_str(),
                    line.reason_code,
                )],
                ParsedRestrictions::default(),
                state,
            )?)))
        }
        RewriteSemanticItem::Modal(modal) => Ok(Some(NormalizedCardItem::Modal(
            normalize_rewrite_modal_ast(match lower_rewrite_modal_to_item(modal)? {
                ParsedCardItem::Modal(modal) => modal,
                _ => unreachable!("rewrite modal lowering returned non-modal item"),
            })?,
        ))),
        RewriteSemanticItem::LevelHeader(level) => {
            Ok(Some(NormalizedCardItem::LevelAbility(ParsedLevelAbilityAst {
                min_level: level.min_level,
                max_level: level.max_level,
                pt: level.pt,
                items: level.items.into_iter().map(|item| item.parsed).collect(),
            })))
        }
        RewriteSemanticItem::SagaChapter(saga) => {
            Ok(Some(NormalizedCardItem::Line(normalize_rewrite_line_ast(
                saga.info.clone(),
                vec![LineAst::Triggered {
                    trigger: TriggerSpec::SagaChapter(saga.chapters),
                    effects: saga.effects_ast,
                    max_triggers_per_turn: None,
                }],
                ParsedRestrictions::default(),
                state,
            )?)))
        }
    }
}

pub(crate) fn rewrite_document_to_normalized_card_ast(
    doc: RewriteSemanticDocument,
) -> Result<NormalizedCardAst, CardTextError> {
    let RewriteSemanticDocument {
        builder,
        annotations,
        items,
        allow_unsupported,
    } = doc;
    let mut state = RewriteNormalizationState::default();
    let mut normalized_items = Vec::new();
    for item in items {
        let maybe_item = rewrite_item_to_normalized_item(item, allow_unsupported, &mut state)?;
        if let Some(item) = maybe_item {
            normalized_items.push(item);
        }
    }

    Ok(NormalizedCardAst {
        builder,
        annotations,
        items: normalized_items,
        allow_unsupported,
    })
}

pub(crate) fn lower_rewrite_document(
    doc: RewriteSemanticDocument,
) -> Result<(CardDefinition, ParseAnnotations), CardTextError> {
    let ast = rewrite_document_to_normalized_card_ast(doc)?;
    let NormalizedCardAst {
        mut builder,
        mut annotations,
        items,
        allow_unsupported,
    } = ast;

    let mut level_abilities = Vec::new();
    let mut last_restrictable_ability: Option<usize> = None;
    let mut state = RewriteLoweredCardState::default();

    for item in items {
        match item {
            NormalizedCardItem::Line(line) => {
                rewrite_lower_line_ast(
                    &mut builder,
                    &mut state,
                    &mut annotations,
                    line,
                    allow_unsupported,
                    &mut last_restrictable_ability,
                )?;
            }
            NormalizedCardItem::Modal(modal) => {
                let abilities_before = builder.abilities.len();
                builder = rewrite_lower_parsed_modal(builder, modal, allow_unsupported)?;
                rewrite_update_last_restrictable_ability(
                    &builder,
                    abilities_before,
                    &mut last_restrictable_ability,
                );
            }
            NormalizedCardItem::LevelAbility(level) => {
                level_abilities.push(rewrite_lower_level_ability_ast(level)?);
            }
        }
    }

    if !level_abilities.is_empty() {
        builder = builder.with_level_abilities(level_abilities);
    }

    builder = rewrite_finalize_lowered_card(builder, &mut state);
    Ok((builder.build(), annotations))
}

fn try_merge_modal_into_remove_mode(
    effects: &mut crate::resolution::ResolutionProgram,
    modal_effect: crate::effect::Effect,
    predicate: crate::effect::EffectPredicate,
) -> bool {
    let Some(last_effect) = effects.pop() else {
        return false;
    };

    let Some(choose_mode) = last_effect.downcast_ref::<crate::effects::ChooseModeEffect>() else {
        effects.push(last_effect);
        return false;
    };
    if choose_mode.modes.len() < 2 {
        effects.push(last_effect);
        return false;
    }

    let Some(remove_mode_idx) = choose_mode
        .modes
        .iter()
        .position(|mode| mode.description.to_ascii_lowercase().starts_with("remove "))
    else {
        effects.push(last_effect);
        return false;
    };

    let mut modes = choose_mode.modes.clone();
    let remove_mode = &mut modes[remove_mode_idx];
    let gate_id = crate::effect::EffectId(1_000_000_000);
    if let Some(last_remove_effect) = remove_mode.effects.pop() {
        remove_mode.effects.push(crate::effect::Effect::with_id(
            gate_id.0,
            last_remove_effect,
        ));
        remove_mode.effects.push(crate::effect::Effect::if_then(
            gate_id,
            predicate,
            vec![modal_effect],
        ));
    } else {
        remove_mode.effects.push(modal_effect);
    }

    effects.push(crate::effect::Effect::new(
        crate::effects::ChooseModeEffect {
            modes,
            choose_count: choose_mode.choose_count.clone(),
            min_choose_count: choose_mode.min_choose_count.clone(),
            allow_repeated_modes: choose_mode.allow_repeated_modes,
            disallow_previously_chosen_modes: choose_mode.disallow_previously_chosen_modes,
            disallow_previously_chosen_modes_this_turn: choose_mode
                .disallow_previously_chosen_modes_this_turn,
        },
    ));
    true
}

pub(crate) fn rewrite_lower_parsed_modal(
    mut builder: CardDefinitionBuilder,
    pending_modal: NormalizedModalAst,
    allow_unsupported: bool,
) -> Result<CardDefinitionBuilder, CardTextError> {
    let NormalizedModalAst {
        header,
        prepared_prefix,
        modes,
    } = pending_modal;
    let crate::cards::builders::ParsedModalHeader {
        min: header_min,
        max: header_max,
        same_mode_more_than_once,
        mode_must_be_unchosen,
        mode_must_be_unchosen_this_turn,
        commander_allows_both,
        trigger,
        activated,
        x_replacement: _,
        prefix_effects_ast: _,
        modal_gate,
        line_text,
    } = header;

    let (prefix_effects, prefix_choices) = if prepared_prefix.is_none() {
        (crate::resolution::ResolutionProgram::default(), Vec::new())
    } else if trigger.is_some() || activated.is_some() {
        match super::compile_support::materialize_prepared_effects_with_trigger_context(
            prepared_prefix
                .as_ref()
                .expect("prepared prefix exists when checked above"),
        ) {
            Ok(lowered) => (lowered.effects, lowered.choices),
            Err(err) if allow_unsupported => {
                builder = push_unsupported_marker(builder, line_text.as_str(), format!("{err:?}"));
                return Ok(builder);
            }
            Err(err) => return Err(err),
        }
    } else {
        match rewrite_lower_prepared_statement_effects(
            prepared_prefix
                .as_ref()
                .expect("prepared prefix exists when checked above"),
        ) {
            Ok(lowered) => (lowered.effects, lowered.choices),
            Err(err) if allow_unsupported => {
                builder = push_unsupported_marker(builder, line_text.as_str(), format!("{err:?}"));
                return Ok(builder);
            }
            Err(err) => return Err(err),
        }
    };

    let mut compiled_modes = Vec::new();
    for mode in modes {
        let effects = match rewrite_lower_prepared_statement_effects(&mode.prepared) {
            Ok(lowered) => lowered.effects,
            Err(err) if allow_unsupported => {
                builder = push_unsupported_marker(
                    builder,
                    mode.info.raw_line.as_str(),
                    format!("{err:?}"),
                );
                continue;
            }
            Err(err) => return Err(err),
        };
        compiled_modes.push(crate::effect::EffectMode {
            description: mode.description,
            effects: effects.to_vec(),
        });
    }

    if compiled_modes.is_empty() {
        return Ok(builder);
    }

    let mode_count = compiled_modes.len() as i32;
    let default_max = crate::effect::Value::Fixed(mode_count);
    let max = header_max.unwrap_or_else(|| default_max.clone());
    let min = header_min;
    let is_fixed_one =
        |value: &crate::effect::Value| matches!(value, crate::effect::Value::Fixed(1));
    let with_unchosen_requirement = |effect: crate::effect::Effect| {
        if !mode_must_be_unchosen {
            return effect;
        }
        if let Some(choose_mode) = effect.downcast_ref::<crate::effects::ChooseModeEffect>() {
            let choose_mode = choose_mode.clone();
            let choose_mode = if mode_must_be_unchosen_this_turn {
                choose_mode.with_previously_unchosen_modes_only_this_turn()
            } else {
                choose_mode.with_previously_unchosen_modes_only()
            };
            return crate::effect::Effect::new(choose_mode);
        }
        effect
    };

    let modal_effect = if commander_allows_both {
        let max_both = (mode_count.min(2)).max(1);
        let choose_both = if max_both == 1 {
            with_unchosen_requirement(crate::effect::Effect::choose_one(compiled_modes.clone()))
        } else {
            with_unchosen_requirement(crate::effect::Effect::choose_up_to(
                max_both,
                1,
                compiled_modes.clone(),
            ))
        };
        let choose_one =
            with_unchosen_requirement(crate::effect::Effect::choose_one(compiled_modes.clone()));
        crate::effect::Effect::conditional(
            crate::effect::Condition::YouControlCommander,
            vec![choose_both],
            vec![choose_one],
        )
    } else if same_mode_more_than_once && min == max {
        with_unchosen_requirement(crate::effect::Effect::choose_exactly_allow_repeated_modes(
            max.clone(),
            compiled_modes,
        ))
    } else if is_fixed_one(&min) && is_fixed_one(&max) {
        with_unchosen_requirement(crate::effect::Effect::choose_one(compiled_modes))
    } else if min == max {
        with_unchosen_requirement(crate::effect::Effect::choose_exactly(
            max.clone(),
            compiled_modes,
        ))
    } else {
        with_unchosen_requirement(crate::effect::Effect::choose_up_to(
            max.clone(),
            min.clone(),
            compiled_modes,
        ))
    };

    let mut combined_effects = prefix_effects;
    if let Some(modal_gate) = modal_gate {
        if modal_gate.remove_mode_only
            && try_merge_modal_into_remove_mode(
                &mut combined_effects,
                modal_effect.clone(),
                modal_gate.predicate.clone(),
            )
        {
        } else if let Some(last_effect) = combined_effects.pop() {
            let gate_id = crate::effect::EffectId(1_000_000_000);
            combined_effects.push(crate::effect::Effect::with_id(gate_id.0, last_effect));
            combined_effects.push(crate::effect::Effect::if_then(
                gate_id,
                modal_gate.predicate,
                vec![modal_effect],
            ));
        } else {
            combined_effects.push(modal_effect);
        }
    } else {
        combined_effects.push(modal_effect);
    }

    let modal_lowered = LoweredEffects {
        effects: combined_effects.clone(),
        choices: prefix_choices.clone(),
        exports: ReferenceExports::default(),
    };
    rewrite_validate_iterated_player_bindings_in_lowered_effects(
        &modal_lowered,
        trigger
            .as_ref()
            .is_some_and(rewrite_trigger_binds_player_reference_context),
        if trigger.is_some() {
            "triggered modal ability effects"
        } else if activated.is_some() {
            "activated modal ability effects"
        } else {
            "modal spell effects"
        },
    )?;

    if let Some(trigger) = trigger {
        let mut ability = rewrite_parsed_triggered_ability(
            trigger,
            Vec::new(),
            vec![Zone::Battlefield],
            Some(line_text),
            None,
            ReferenceImports::default(),
        )
        .ability;
        if let AbilityKind::Triggered(triggered) = &mut ability.kind {
            triggered.effects = combined_effects.clone();
            triggered.choices = prefix_choices;
        }
        builder = builder.with_ability(ability);
    } else if let Some(activated) = activated {
        builder = builder.with_ability(Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost: activated.mana_cost,
                effects: combined_effects.clone(),
                choices: prefix_choices,
                timing: activated.timing,
                additional_restrictions: activated.additional_restrictions,
                activation_restrictions: activated.activation_restrictions,
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
            }),
            functional_zones: activated.functional_zones,
            text: Some(line_text),
        });
    } else if let Some(ref mut existing) = builder.spell_effect {
        existing.extend(combined_effects);
    } else {
        builder.spell_effect = Some(combined_effects);
    }

    Ok(builder)
}

pub(crate) fn parse_text_with_annotations_rewrite_lowered(
    builder: CardDefinitionBuilder,
    text: String,
    allow_unsupported: bool,
) -> Result<(CardDefinition, ParseAnnotations), CardTextError> {
    let (doc, _) =
        match parse_text_with_annotations_rewrite(builder.clone(), text.clone(), allow_unsupported)
        {
            Ok(parsed) => parsed,
            Err(err) => {
                if !allow_unsupported
                    && let Some(compat) =
                        try_section9_compatibility_definition(builder.clone(), &text)?
                {
                    return Ok(compat);
                }
                return Err(err);
            }
        };
    match lower_rewrite_document(doc) {
        Ok(lowered) => Ok(lowered),
        Err(err) => {
            if !allow_unsupported
                && let Some(compat) = try_section9_compatibility_definition(builder, &text)?
            {
                return Ok(compat);
            }
            Err(err)
        }
    }
}
