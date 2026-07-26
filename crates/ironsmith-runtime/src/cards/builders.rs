//! Extended card builder with ability support.
//!
//! This module extends the CardBuilder with methods for adding abilities,
//! making it easy to define cards with their complete gameplay mechanics.

#![allow(dead_code)]

use crate::ability::{
    self, Ability, AbilityKind, ActivationTiming, LevelAbility, TriggeredAbility,
};
use crate::alternative_cast::AlternativeCastingMethod;
#[cfg(any(test, ironsmith_runtime_parser_tests))]
use crate::card::PtValue;
use crate::card::{CardBuilder, LinkedFaceLayout, PowerToughness};
use crate::color::ColorSet;
use crate::cost::{OptionalCost, OptionalCostKind, TotalCost};
use crate::effect::{
    ChoiceCount, Condition, Effect, EffectId, EffectMode, EffectPredicate, EventValueSpec, Until,
    Value,
};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::object::{AuraAttachmentFilter, CounterType};
use crate::resolution::ResolutionProgram;
use crate::static_abilities::StaticAbility;
use crate::tag::TagKey;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::triggers::Trigger;
#[cfg(any(test, ironsmith_runtime_parser_tests))]
use crate::types::SubtypeFamily;
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;
#[cfg(any(test, ironsmith_runtime_parser_tests))]
use std::collections::HashMap;

use super::CardDefinition;

/// Runtime ability carried by the riot keyword.
///
/// Keeping this constructor reusable lets both printed riot and effects that
/// grant riot to a spell install the same gameplay representation.
pub(crate) fn riot_triggered_ability() -> Ability {
    let modes = vec![
        EffectMode {
            source_text: "This creature enters with a +1/+1 counter on it".to_string(),
            effects: vec![Effect::plus_one_counters(1, ChooseSpec::Source)],
        },
        EffectMode {
            source_text: "This creature gains haste until end of turn".to_string(),
            effects: vec![Effect::grant_abilities_all(
                ObjectFilter::source(),
                vec![StaticAbility::haste()],
                Until::EndOfTurn,
            )],
        },
    ];

    Ability::triggered(
        Trigger::this_enters_battlefield(),
        vec![Effect::choose_one(modes)],
    )
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CardTextError {
    UnsupportedLine(String),
    ParseError(String),
    InvariantViolation(String),
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
impl std::fmt::Display for CardTextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CardTextError::UnsupportedLine(message)
            | CardTextError::ParseError(message)
            | CardTextError::InvariantViolation(message) => f.write_str(message),
        }
    }
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
impl std::error::Error for CardTextError {}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
type ParsedAbility = ();

#[cfg(ironsmith_runtime_parser_tests)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ParseCacheKey {
    builder_context: String,
    text: String,
    allow_unsupported: bool,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl ParseCacheKey {
    fn new(builder: &CardDefinitionBuilder, text: &str, allow_unsupported: bool) -> Self {
        Self {
            builder_context: format!("{builder:?}"),
            text: text.to_string(),
            allow_unsupported,
        }
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
type CachedParseResult = Result<CardDefinition, CardTextError>;

#[cfg(ironsmith_runtime_parser_tests)]
#[allow(dead_code)]
#[path = "../../../ironsmith-registry/src/compiler_runtime.rs"]
mod compiler_runtime_for_tests;

#[cfg(ironsmith_runtime_parser_tests)]
fn parse_result_cache() -> &'static std::sync::Mutex<HashMap<ParseCacheKey, CachedParseResult>> {
    static PARSE_RESULT_CACHE: std::sync::OnceLock<
        std::sync::Mutex<HashMap<ParseCacheKey, CachedParseResult>>,
    > = std::sync::OnceLock::new();
    PARSE_RESULT_CACHE.get_or_init(|| std::sync::Mutex::new(HashMap::new()))
}

#[cfg(ironsmith_runtime_parser_tests)]
fn store_cached_parse(key: ParseCacheKey, result: CachedParseResult) -> CachedParseResult {
    parse_result_cache()
        .lock()
        .expect("parse result cache mutex poisoned")
        .insert(key, result.clone());
    result
}

#[cfg(ironsmith_runtime_parser_tests)]
fn finalize_definition(
    definition: CardDefinition,
    original_builder: &CardDefinitionBuilder,
    original_text: &str,
) -> Result<CardDefinition, CardTextError> {
    let _ = (original_builder, original_text);
    Ok(finalize_nonpermanent_delayed_triggered_abilities(
        definition,
    ))
}

#[cfg(ironsmith_runtime_parser_tests)]
fn normalize_delayed_trigger_text(text: &str) -> String {
    text.to_ascii_lowercase()
        .replace('’', "'")
        .replace("'s", "s")
}

#[cfg(ironsmith_runtime_parser_tests)]
fn spell_battlefield_trigger_text_implies_delayed_schedule(
    ability_text: &str,
    trigger: &Trigger,
) -> Option<bool> {
    let normalized = normalize_delayed_trigger_text(ability_text);
    let trigger_text = normalize_delayed_trigger_text(trigger.display().as_str());

    let trigger_is_upkeep_or_end_step = trigger_text.contains("beginning of")
        && (trigger_text.contains("upkeep") || trigger_text.contains("end step"));
    if !trigger_is_upkeep_or_end_step {
        return None;
    }

    if normalized.contains("next upkeep") || normalized.contains("next turns upkeep") {
        return Some(true);
    }
    if normalized.contains("that turns end step")
        || normalized.contains("that players next upkeep")
        || normalized.contains("that players next end step")
        || normalized.contains("end step of that players next turn")
    {
        return Some(true);
    }
    if normalized.contains("next end step") || normalized.contains("next turns end step") {
        return Some(false);
    }

    None
}

#[cfg(ironsmith_runtime_parser_tests)]
fn delayed_trigger_spec_from_label(
    trigger_label: &str,
    ability_text: Option<&str>,
) -> Option<Trigger> {
    let label = trigger_label.to_ascii_lowercase();
    let text = ability_text.unwrap_or_default().to_ascii_lowercase();
    if label == "beginning_of_upkeep" || label.contains("upkeep") {
        let player = if text.contains("your next upkeep") {
            PlayerFilter::You
        } else {
            PlayerFilter::Any
        };
        return Some(Trigger::beginning_of_upkeep(player));
    }
    if label == "beginning_of_draw_step" || label.contains("draw step") {
        let player = if text.contains("your next draw step") {
            PlayerFilter::You
        } else {
            PlayerFilter::Any
        };
        return Some(Trigger::beginning_of_draw_step(player));
    }
    if label == "beginning_of_end_step" || label.contains("end step") {
        return Some(Trigger::beginning_of_end_step(PlayerFilter::Any));
    }
    match label.as_str() {
        "beginning_of_upkeep" => {
            let player = if text.contains("your next upkeep") {
                PlayerFilter::You
            } else {
                PlayerFilter::Any
            };
            Some(Trigger::beginning_of_upkeep(player))
        }
        "beginning_of_draw_step" => {
            let player = if text.contains("your next draw step") {
                PlayerFilter::You
            } else {
                PlayerFilter::Any
            };
            Some(Trigger::beginning_of_draw_step(player))
        }
        "beginning_of_end_step" => Some(Trigger::beginning_of_end_step(PlayerFilter::Any)),
        "end_of_combat" => Some(Trigger::end_of_combat()),
        "this_dies" => Some(Trigger::this_dies()),
        _ => None,
    }
}

fn ability_with_inherent_functional_zones(ability: Ability) -> Ability {
    let AbilityKind::Static(static_ability) = &ability.kind else {
        return ability;
    };
    match static_ability.id() {
        crate::static_abilities::StaticAbilityId::ExileToExileInsteadOfGraveyard
        | crate::static_abilities::StaticAbilityId::ExileToCounteredExileInsteadOfGraveyard
        | crate::static_abilities::StaticAbilityId::ExileWouldDieInstead => ability.in_zones(vec![
            Zone::Battlefield,
            Zone::Stack,
            Zone::Graveyard,
            Zone::Hand,
            Zone::Library,
            Zone::Exile,
            Zone::Command,
        ]),
        crate::static_abilities::StaticAbilityId::Grants => {
            if let Some(spec) = static_ability.grant_spec()
                && spec.filter.source
                && spec.zone != Zone::Battlefield
            {
                ability.in_zones(vec![spec.zone])
            } else {
                ability
            }
        }
        _ => ability,
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
fn convert_nonpermanent_delayed_triggered_ability_to_spell_effect(
    ability: &Ability,
) -> Option<Effect> {
    if ability.functional_zones.as_slice() != [Zone::Battlefield] {
        return None;
    }

    let AbilityKind::Triggered(triggered) = &ability.kind else {
        return None;
    };
    if !triggered.choices.is_empty() || triggered.intervening_if.is_some() {
        return None;
    }

    let trigger_display = triggered.trigger.display().to_ascii_lowercase();
    let is_delayed_step_trigger = trigger_display.contains("beginning of")
        && (trigger_display.contains("upkeep")
            || trigger_display.contains("draw step")
            || trigger_display.contains("end step"));
    if !is_delayed_step_trigger {
        return None;
    }

    let mut schedule = crate::effects::ScheduleDelayedTriggerEffect::new(
        triggered.trigger.clone(),
        triggered.effects.clone(),
        true,
        Vec::new(),
        PlayerFilter::You,
    );
    schedule.start_next_turn =
        trigger_display.contains("upkeep") || trigger_display.contains("draw step");
    Some(Effect::new(schedule))
}

#[cfg(ironsmith_runtime_parser_tests)]
fn finalize_nonpermanent_delayed_triggered_abilities(
    mut definition: CardDefinition,
) -> CardDefinition {
    if !definition.card.is_instant() && !definition.card.is_sorcery() {
        return definition;
    }

    let mut rewritten_effects = Vec::new();
    let mut remaining_abilities = Vec::with_capacity(definition.abilities.len());
    for ability in std::mem::take(&mut definition.abilities) {
        if let Some(effect) =
            convert_nonpermanent_delayed_triggered_ability_to_spell_effect(&ability)
        {
            rewritten_effects.push(effect);
        } else {
            remaining_abilities.push(ability);
        }
    }

    definition.abilities = remaining_abilities;
    if !rewritten_effects.is_empty() {
        definition
            .spell_effect
            .get_or_insert_with(ResolutionProgram::default)
            .extend(ResolutionProgram::from_effects(rewritten_effects));
    }
    definition
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
fn replace_whole_word_case_insensitive(text: &str, from: &str, to: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut i = 0usize;
    let from_chars = from.chars().count();

    while i < text.len() {
        let rest = &text[i..];
        let prefix: String = rest.chars().take(from_chars).collect();
        if !prefix.is_empty()
            && prefix.eq_ignore_ascii_case(from)
            && (i == 0
                || !text[..i]
                    .chars()
                    .next_back()
                    .is_some_and(|ch| ch.is_ascii_alphanumeric()))
            && (i + prefix.len() == text.len()
                || !text[i + prefix.len()..]
                    .chars()
                    .next()
                    .is_some_and(|ch| ch.is_ascii_alphanumeric()))
        {
            let replacement = if prefix
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase())
            {
                let mut chars = to.chars();
                if let Some(first) = chars.next() {
                    format!("{}{}", first.to_ascii_uppercase(), chars.as_str())
                } else {
                    to.to_string()
                }
            } else {
                to.to_string()
            };
            out.push_str(&replacement);
            i += prefix.len();
            continue;
        }

        let mut chars = rest.chars();
        let ch = chars
            .next()
            .expect("rest is non-empty while walking replacement text");
        out.push(ch);
        i += ch.len_utf8();
    }

    out
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
fn describe_hexproof_from_filter(filter: &ObjectFilter) -> String {
    if !filter.any_of.is_empty() {
        return filter
            .any_of
            .iter()
            .map(describe_hexproof_from_filter)
            .collect::<Vec<_>>()
            .join(" or ");
    }

    let description = filter.description();
    let fragment = description
        .strip_suffix(" permanent")
        .or_else(|| description.strip_suffix(" spell"))
        .or_else(|| description.strip_suffix(" source"))
        .unwrap_or(description.as_str());
    // A bare type noun reads as a class: "hexproof from planeswalkers".
    if filter.card_types.len() == 1
        && filter.card_types[0].to_string().eq_ignore_ascii_case(fragment)
    {
        return format!("{fragment}s");
    }
    fragment.to_string()
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InsteadSemantics {
    SelfReplacement,
    FutureReplacement,
    NonReplacement,
}

fn finalize_backup_abilities(definition: CardDefinition) -> CardDefinition {
    definition
}

fn finalize_cipher_effects(definition: CardDefinition) -> CardDefinition {
    definition
}

#[cfg(ironsmith_runtime_parser_tests)]
fn lookup_cached_parse(key: &ParseCacheKey) -> Option<CachedParseResult> {
    parse_result_cache()
        .lock()
        .expect("parse result cache mutex poisoned")
        .get(key)
        .cloned()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub fn parse_card_text(
    builder: CardDefinitionBuilder,
    text: impl Into<String>,
) -> Result<CardDefinition, CardTextError> {
    parse_card_text_with_policy(builder, text, false)
}

#[cfg(ironsmith_runtime_parser_tests)]
pub fn parse_card_text_allow_unsupported(
    builder: CardDefinitionBuilder,
    text: impl Into<String>,
) -> Result<CardDefinition, CardTextError> {
    parse_card_text_with_policy(builder, text, true)
}

#[cfg(ironsmith_runtime_parser_tests)]
fn parse_card_text_with_policy(
    builder: CardDefinitionBuilder,
    text: impl Into<String>,
    allow_unsupported: bool,
) -> Result<CardDefinition, CardTextError> {
    let text = text.into();
    let cache_key = ParseCacheKey::new(&builder, &text, allow_unsupported);
    if let Some(cached) = lookup_cached_parse(&cache_key) {
        return cached;
    }
    let result = stacker::maybe_grow(32 * 1024 * 1024, 64 * 1024 * 1024, || {
        compile_to_runtime_definition(&builder, text, allow_unsupported)
    });
    store_cached_parse(cache_key, result)
}

#[cfg(ironsmith_runtime_parser_tests)]
pub fn parse_card_text_with_annotations(
    builder: CardDefinitionBuilder,
    text: impl Into<String>,
) -> Result<(CardDefinition, ParseAnnotations), CardTextError> {
    parse_card_text_with_annotations_policy(builder, text, false)
}

#[cfg(ironsmith_runtime_parser_tests)]
pub fn parse_card_text_with_annotations_allow_unsupported(
    builder: CardDefinitionBuilder,
    text: impl Into<String>,
) -> Result<(CardDefinition, ParseAnnotations), CardTextError> {
    parse_card_text_with_annotations_policy(builder, text, true)
}

#[cfg(ironsmith_runtime_parser_tests)]
fn parse_card_text_with_annotations_policy(
    builder: CardDefinitionBuilder,
    text: impl Into<String>,
    allow_unsupported: bool,
) -> Result<(CardDefinition, ParseAnnotations), CardTextError> {
    let text = text.into();
    let cache_key = ParseCacheKey::new(&builder, &text, allow_unsupported);
    let compiled = stacker::maybe_grow(32 * 1024 * 1024, 64 * 1024 * 1024, || {
        compiler_runtime_for_tests::compile_runtime_builder_snapshot_to_runtime_compiled_card_text(
            runtime_builder_snapshot(&builder),
            text,
            allow_unsupported,
        )
    })
    .map_err(compiler_runtime_error_to_card_text_error)?;
    let _ = store_cached_parse(cache_key, Ok(compiled.definition.clone()));
    Ok((
        compiled.definition,
        parse_annotations_from_compiler(compiled.annotations),
    ))
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum KeywordAction {
    Flying,
    Menace,
    Banding,
    Hexproof,
    Haste,
    Improvise,
    Convoke,
    AffinityForArtifacts,
    Delve,
    FirstStrike,
    DoubleStrike,
    Deathtouch,
    Lifelink,
    Vigilance,
    Trample,
    Reach,
    Defender,
    Flash,
    Phasing,
    Indestructible,
    Shroud,
    Ward(u32),
    Wither,
    Afflict(u32),
    Afterlife(u32),
    Fabricate(u32),
    Infect,
    Undying,
    Persist,
    Prowess,
    Exalted,
    Cascade,
    Storm,
    Gravestorm,
    Toxic(u32),
    Poisonous(u32),
    BattleCry,
    Dethrone,
    Evolve,
    Ingest,
    Mentor,
    Skulk,
    Training,
    Myriad,
    Riot,
    Unleash,
    Renown(u32),
    Modular(u32),
    ModularSunburst,
    Graft(u32),
    Soulbond,
    Soulshift(u32),
    SoulshiftValue(Value),
    Recover(ManaCost),
    Outlast(ManaCost),
    Scavenge(ManaCost),
    Unearth(ManaCost),
    Embalm(ManaCost),
    Eternalize(ManaCost),
    Emerge(ManaCost),
    Ninjutsu(ManaCost),
    Backup(u32),
    Cipher,
    Dash(ManaCost),
    Blitz(ManaCost),
    BlitzFromGraveyard,
    Warp(ManaCost),
    Plot(ManaCost),
    Melee,
    Mobilize(u32),
    Suspend {
        time: u32,
        cost: ManaCost,
    },
    Disturb(ManaCost),
    Overload(ManaCost),
    Cleave(ManaCost),
    Awaken {
        amount: u32,
        cost: ManaCost,
    },
    Spectacle(ManaCost),
    Foretell(ManaCost),
    Echo {
        total_cost: TotalCost,
        text: String,
    },
    CumulativeUpkeep {
        mana_symbols_per_counter: Vec<ManaSymbol>,
        life_per_counter: u32,
        text: String,
    },
    Casualty(u32),
    VariableCasualtyPlaneswalkerCopy,
    Demonstrate,
    Conspire,
    Amplify(u32),
    AuraSwap(ManaCost),
    Devour(u32),
    Ravenous,
    Ascend,
    Daybound,
    Nightbound,
    Haunt,
    Provoke,
    Undaunted,
    Enlist,
    Extort,
    Partner,
    StartYourEngines,
    Assist,
    SplitSecond,
    Rebound,
    Sunburst,
    ReadAhead,
    Firebending(u32),
    Fading(u32),
    Vanishing(u32),
    Fear,
    Intimidate,
    Shadow,
    Horsemanship,
    Flanking,
    UmbraArmor,
    Landwalk(crate::static_abilities::LandwalkKind),
    Bloodthirst(u32),
    Tribute(u32),
    Rampage(u32),
    Bushido(u32),
    Frenzy(u32),
    Changeling,
    HexproofFrom(ObjectFilter),
    ProtectionFrom(ColorSet),
    ProtectionFromAllColors,
    ProtectionFromColorless,
    ProtectionFromEverything,
    ProtectionFromChosenPlayer,
    ProtectionFromChosenColor,
    ProtectionFromFilter(ObjectFilter),
    ProtectionFromEachManaValueAmong(ObjectFilter),
    ProtectionFromCardType(CardType),
    ProtectionFromSubtype(Subtype),
    Unblockable,
    Devoid,
    Annihilator(u32),
    ForMirrodin,
    LivingWeapon,
    Crew {
        amount: u32,
        timing: ActivationTiming,
        additional_restrictions: Vec<String>,
    },
    Saddle {
        amount: u32,
        timing: ActivationTiming,
        additional_restrictions: Vec<String>,
    },
    Marker(&'static str),
    MarkerText(String),
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
fn describe_soulshift_value(value: &Value) -> String {
    if let Value::Count(filter) = value
        && filter.zone == Some(Zone::Battlefield)
        && filter.controller == Some(PlayerFilter::You)
        && filter.subtypes.contains(&Subtype::Spirit)
    {
        return "the number of Spirits you control".to_string();
    }
    "that value".to_string()
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
impl KeywordAction {
    pub(crate) fn lowers_to_static_ability(&self) -> bool {
        matches!(
            self,
            Self::Flying
                | Self::Menace
                | Self::Hexproof
                | Self::Haste
                | Self::Improvise
                | Self::Convoke
                | Self::AffinityForArtifacts
                | Self::Delve
                | Self::FirstStrike
                | Self::DoubleStrike
                | Self::Deathtouch
                | Self::Lifelink
                | Self::Vigilance
                | Self::Trample
                | Self::Reach
                | Self::Defender
                | Self::Flash
                | Self::Phasing
                | Self::Indestructible
                | Self::Shroud
                | Self::Ward(_)
                | Self::Wither
                | Self::Afterlife(_)
                | Self::Fabricate(_)
                | Self::Infect
                | Self::Undying
                | Self::Persist
                | Self::Prowess
                | Self::Exalted
                | Self::Cascade
                | Self::Storm
                | Self::Gravestorm
                | Self::Toxic(_)
                | Self::Poisonous(_)
                | Self::BattleCry
                | Self::Dethrone
                | Self::Evolve
                | Self::Ingest
                | Self::Mentor
                | Self::Skulk
                | Self::Training
                | Self::Riot
                | Self::Unleash
                | Self::Renown(_)
                | Self::Modular(_)
                | Self::Graft(_)
                | Self::Soulbond
                | Self::Soulshift(_)
                | Self::SoulshiftValue(_)
                | Self::Outlast(_)
                | Self::Unearth(_)
                | Self::Eternalize(_)
                | Self::Ninjutsu(_)
                | Self::Extort
                | Self::Partner
                | Self::StartYourEngines
                | Self::Assist
                | Self::SplitSecond
                | Self::Rebound
                | Self::Sunburst
                | Self::ReadAhead
                | Self::Firebending(_)
                | Self::Fading(_)
                | Self::Vanishing(_)
                | Self::Fear
                | Self::Intimidate
                | Self::Shadow
                | Self::Horsemanship
                | Self::Flanking
                | Self::UmbraArmor
                | Self::Landwalk(_)
                | Self::Bloodthirst(_)
                | Self::Tribute(_)
                | Self::Rampage(_)
                | Self::Bushido(_)
                | Self::Frenzy(_)
                | Self::Changeling
                | Self::HexproofFrom(_)
                | Self::ProtectionFrom(_)
                | Self::ProtectionFromAllColors
                | Self::ProtectionFromColorless
                | Self::ProtectionFromEverything
                | Self::ProtectionFromChosenPlayer
                | Self::ProtectionFromChosenColor
                | Self::ProtectionFromFilter(_)
                | Self::ProtectionFromEachManaValueAmong(_)
                | Self::ProtectionFromCardType(_)
                | Self::ProtectionFromSubtype(_)
                | Self::Unblockable
                | Self::Devoid
                | Self::Annihilator(_)
                | Self::Marker(_)
                | Self::MarkerText(_)
        )
    }

    pub(crate) fn display_text(&self) -> String {
        fn single_color_name(colors: ColorSet) -> Option<&'static str> {
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

        match self {
            Self::Flying => "Flying".to_string(),
            Self::Menace => "Menace".to_string(),
            Self::Banding => "Banding".to_string(),
            Self::Hexproof => "Hexproof".to_string(),
            Self::Haste => "Haste".to_string(),
            Self::Improvise => "Improvise".to_string(),
            Self::Convoke => "Convoke".to_string(),
            Self::AffinityForArtifacts => "Affinity for artifacts".to_string(),
            Self::Delve => "Delve".to_string(),
            Self::FirstStrike => "First strike".to_string(),
            Self::DoubleStrike => "Double strike".to_string(),
            Self::Deathtouch => "Deathtouch".to_string(),
            Self::Lifelink => "Lifelink".to_string(),
            Self::Vigilance => "Vigilance".to_string(),
            Self::Trample => "Trample".to_string(),
            Self::Reach => "Reach".to_string(),
            Self::Defender => "Defender".to_string(),
            Self::Flash => "Flash".to_string(),
            Self::Phasing => "Phasing".to_string(),
            Self::Indestructible => "Indestructible".to_string(),
            Self::Shroud => "Shroud".to_string(),
            Self::Ward(amount) => format!("Ward {{{amount}}}"),
            Self::Wither => "Wither".to_string(),
            Self::Afflict(amount) => format!("Afflict {amount}"),
            Self::Afterlife(amount) => format!("Afterlife {amount}"),
            Self::Fabricate(amount) => format!("Fabricate {amount}"),
            Self::Infect => "Infect".to_string(),
            Self::Undying => "Undying".to_string(),
            Self::Persist => "Persist".to_string(),
            Self::Prowess => "Prowess".to_string(),
            Self::Exalted => "Exalted".to_string(),
            Self::Cascade => "Cascade".to_string(),
            Self::Storm => "Storm".to_string(),
            Self::Gravestorm => "Gravestorm".to_string(),
            Self::Toxic(amount) => format!("Toxic {amount}"),
            Self::Poisonous(amount) => format!("Poisonous {amount}"),
            Self::BattleCry => "Battle cry".to_string(),
            Self::Dethrone => "Dethrone".to_string(),
            Self::Evolve => "Evolve".to_string(),
            Self::Ingest => "Ingest".to_string(),
            Self::Mentor => "Mentor".to_string(),
            Self::Skulk => "Skulk".to_string(),
            Self::Training => "Training".to_string(),
            Self::Myriad => "Myriad".to_string(),
            Self::Riot => "Riot".to_string(),
            Self::Unleash => "Unleash".to_string(),
            Self::Renown(amount) => format!("Renown {amount}"),
            Self::Modular(amount) => format!("Modular {amount}"),
            Self::ModularSunburst => "Modular-Sunburst".to_string(),
            Self::Graft(amount) => format!("Graft {amount}"),
            Self::Soulbond => "Soulbond".to_string(),
            Self::Soulshift(amount) => format!("Soulshift {amount}"),
            Self::SoulshiftValue(value) => format!(
                "Soulshift X, where X is {}",
                describe_soulshift_value(value)
            ),
            Self::Recover(cost) => format!("Recover {}", cost.to_oracle()),
            Self::Outlast(cost) => format!("Outlast {}", cost.to_oracle()),
            Self::Scavenge(cost) => format!("Scavenge {}", cost.to_oracle()),
            Self::Unearth(cost) => format!("Unearth {}", cost.to_oracle()),
            Self::Embalm(cost) => format!("Embalm {}", cost.to_oracle()),
            Self::Eternalize(cost) => format!("Eternalize {}", cost.to_oracle()),
            Self::Emerge(cost) => format!("Emerge {}", cost.to_oracle()),
            Self::Ninjutsu(cost) => format!("Ninjutsu {}", cost.to_oracle()),
            Self::Backup(amount) => format!("Backup {amount}"),
            Self::Cipher => "Cipher".to_string(),
            Self::Dash(cost) => format!("Dash {}", cost.to_oracle()),
            Self::Blitz(cost) => format!("Blitz {}", cost.to_oracle()),
            Self::BlitzFromGraveyard => {
                "You may cast this card from your graveyard using its blitz ability.".to_string()
            }
            Self::Warp(cost) => format!("Warp {}", cost.to_oracle()),
            Self::Plot(cost) => format!("Plot {}", cost.to_oracle()),
            Self::Melee => "Melee".to_string(),
            Self::Mobilize(amount) => format!("Mobilize {amount}"),
            Self::Suspend { time, cost } => format!("Suspend {time}—{}", cost.to_oracle()),
            Self::Disturb(cost) => format!("Disturb {}", cost.to_oracle()),
            Self::Overload(cost) => format!("Overload {}", cost.to_oracle()),
            Self::Cleave(cost) => format!("Cleave {}", cost.to_oracle()),
            Self::Awaken { amount, cost } => format!("Awaken {amount}—{}", cost.to_oracle()),
            Self::Spectacle(cost) => format!("Spectacle {}", cost.to_oracle()),
            Self::Foretell(cost) => format!("Foretell {}", cost.to_oracle()),
            Self::Echo { text, .. } => text.clone(),
            Self::CumulativeUpkeep { text, .. } => text.clone(),
            Self::Casualty(amount) => format!("Casualty {amount}"),
            Self::VariableCasualtyPlaneswalkerCopy => {
                "Casualty X. The copy isn't legendary and has starting loyalty X.".to_string()
            }
            Self::Demonstrate => "Demonstrate".to_string(),
            Self::Conspire => "Conspire".to_string(),
            Self::Amplify(amount) => format!("Amplify {amount}"),
            Self::AuraSwap(cost) => format!("Aura swap {}", cost.to_oracle()),
            Self::Devour(amount) => format!("Devour {amount}"),
            Self::Ravenous => "Ravenous".to_string(),
            Self::Ascend => "Ascend".to_string(),
            Self::Daybound => "Daybound".to_string(),
            Self::Nightbound => "Nightbound".to_string(),
            Self::Haunt => "Haunt".to_string(),
            Self::Provoke => "Provoke".to_string(),
            Self::Undaunted => "Undaunted".to_string(),
            Self::Enlist => "Enlist".to_string(),
            Self::Extort => "Extort".to_string(),
            Self::Partner => "Partner".to_string(),
            Self::StartYourEngines => "Start your engines!".to_string(),
            Self::Assist => "Assist".to_string(),
            Self::SplitSecond => "Split second".to_string(),
            Self::Rebound => "Rebound".to_string(),
            Self::Sunburst => "Sunburst".to_string(),
            Self::ReadAhead => "Read ahead".to_string(),
            Self::Firebending(amount) => format!("Firebending {amount}"),
            Self::Fading(amount) => format!("Fading {amount}"),
            Self::Vanishing(amount) => format!("Vanishing {amount}"),
            Self::Fear => "Fear".to_string(),
            Self::Intimidate => "Intimidate".to_string(),
            Self::Shadow => "Shadow".to_string(),
            Self::Horsemanship => "Horsemanship".to_string(),
            Self::Flanking => "Flanking".to_string(),
            Self::UmbraArmor => "Umbra armor".to_string(),
            Self::Landwalk(kind) => kind.display(),
            Self::Bloodthirst(amount) => format!("Bloodthirst {amount}"),
            Self::Tribute(amount) => format!("Tribute {amount}"),
            Self::Rampage(amount) => format!("Rampage {amount}"),
            Self::Bushido(amount) => format!("Bushido {amount}"),
            Self::Frenzy(amount) => format!("Frenzy {amount}"),
            Self::Changeling => "Changeling".to_string(),
            Self::HexproofFrom(filter) => {
                format!("Hexproof from {}", describe_hexproof_from_filter(filter))
            }
            Self::ProtectionFrom(colors) => single_color_name(*colors)
                .map(|name| format!("Protection from {name}"))
                .unwrap_or_else(|| "Protection from colors".to_string()),
            Self::ProtectionFromAllColors => "Protection from all colors".to_string(),
            Self::ProtectionFromColorless => "Protection from colorless".to_string(),
            Self::ProtectionFromEverything => "Protection from everything".to_string(),
            Self::ProtectionFromChosenPlayer => "Protection from the chosen player".to_string(),
            Self::ProtectionFromChosenColor => "Protection from the chosen color".to_string(),
            Self::ProtectionFromFilter(filter) => {
                format!("Protection from {}", filter.description())
            }
            Self::ProtectionFromEachManaValueAmong(filter) => {
                format!(
                    "Protection from each mana value among {}",
                    crate::static_abilities::describe_protection_mana_value_scope(filter)
                )
            }
            Self::ProtectionFromCardType(card_type) => format!(
                "Protection from {}",
                card_type.to_string().to_ascii_lowercase()
            ),
            Self::ProtectionFromSubtype(subtype) => format!(
                "Protection from {}",
                subtype.to_string().to_ascii_lowercase()
            ),
            Self::Unblockable => "This can't be blocked".to_string(),
            Self::Devoid => "Devoid".to_string(),
            Self::Annihilator(amount) => format!("Annihilator {amount}"),
            Self::ForMirrodin => "For Mirrodin!".to_string(),
            Self::LivingWeapon => "Living weapon".to_string(),
            Self::Crew { amount, .. } => format!("Crew {amount}"),
            Self::Saddle { amount, .. } => format!("Saddle {amount}"),
            Self::Marker(name) => (*name).to_string(),
            Self::MarkerText(text) => text.clone(),
        }
    }
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextSpan {
    pub line: usize,
    pub start: usize,
    pub end: usize,
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
impl TextSpan {
    fn synthetic() -> Self {
        Self {
            line: 0,
            start: 0,
            end: 0,
        }
    }
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
#[derive(Debug, Clone)]
pub(crate) enum GrantedAbilityAst {
    KeywordAction(KeywordAction),
    StaticAbility(StaticAbility),
    ThisAbility,
    MustAttack,
    MustBlock,
    CanAttackAsThoughNoDefender,
    CanBlockAdditionalCreatureEachCombat {
        additional: usize,
    },
    #[cfg(any(test, ironsmith_runtime_parser_tests))]
    ParsedObjectAbility {
        ability: ParsedAbility,
        display: String,
    },
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
impl From<KeywordAction> for GrantedAbilityAst {
    fn from(action: KeywordAction) -> Self {
        Self::KeywordAction(action)
    }
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
#[derive(Debug, Clone, Copy)]
pub(crate) enum DamageBySpec {
    ThisCreature,
    EquippedCreature,
    EnchantedCreature,
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerAst {
    You,
    Active,
    Any,
    Chosen,
    Defending,
    Attacking,
    MostCardsInHand,
    MostLifeTied,
    LowestLifeTied,
    Target,
    TargetOpponent,
    Opponent,
    That,
    ThatPlayerOrTargetController,
    ItsController,
    ItsOwner,
    Implicit,
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReturnControllerAst {
    Preserve,
    Owner,
    You,
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LibraryConsultModeAst {
    Reveal,
    Exile,
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum LibraryConsultStopRuleAst {
    FirstMatch,
    MatchCount(Value),
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LibraryBottomOrderAst {
    Random,
    ChooserChooses,
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TargetAst {
    Source(Option<TextSpan>),
    AnyTarget(Option<TextSpan>),
    AnyOtherTarget(Option<TextSpan>),
    PlayerOrPlaneswalker(PlayerFilter, Option<TextSpan>),
    AttackedPlayerOrPlaneswalker(Option<TextSpan>),
    Spell(Option<TextSpan>),
    Player(PlayerFilter, Option<TextSpan>),
    Object(ObjectFilter, Option<TextSpan>, Option<TextSpan>),
    Tagged(TagKey, Option<TextSpan>),
    WithCount(Box<TargetAst>, ChoiceCount),
    WithCountValue(Box<TargetAst>, ChoiceCount, Value),
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ObjectRefAst {
    Tagged(TagKey),
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SearchLibrarySlotAst {
    pub(crate) filter: ObjectFilter,
    pub(crate) optional: bool,
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ZoneReplacementDurationAst {
    OneShot,
    UntilEndOfTurn,
    Persistent,
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ControlDurationAst {
    UntilEndOfTurn,
    DuringNextTurn,
    AsLongAsYouControlSource,
    Forever,
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExtraTurnAnchorAst {
    CurrentTurn,
    ReferencedTurn,
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SharedTypeConstraintAst {
    CardType,
    PermanentType,
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExchangeValueKindAst {
    Power,
    Toughness,
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ExchangeValueAst {
    LifeTotal(PlayerAst),
    Stat {
        target: TargetAst,
        kind: ExchangeValueKindAst,
    },
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RetargetModeAst {
    All,
    OneToFixed { target: TargetAst },
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PreventNextTimeDamageSourceAst {
    Choice,
    Target(TargetAst),
    Filter(ObjectFilter),
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RedirectNextTimeDamageDestinationAst {
    SourceObject,
    Controller,
    SourceController,
    TargetObject,
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PreventNextTimeDamageTargetAst {
    AnyTarget,
    You,
    Target(TargetAst),
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClashOpponentAst {
    Opponent,
    TargetOpponent,
    DefendingPlayer,
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
#[derive(Debug, Clone, Default)]
pub struct ParseAnnotations {
    pub tag_spans: HashMap<TagKey, Vec<TextSpan>>,
    pub normalized_lines: HashMap<usize, String>,
    pub original_lines: HashMap<usize, String>,
    pub normalized_char_maps: HashMap<usize, Vec<usize>>,
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetadataLine {
    ManaCost(String),
    TypeLine(String),
    PowerToughness(String),
    Loyalty(String),
    Defense(String),
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
impl ParseAnnotations {
    fn record_tag_span(&mut self, tag: &TagKey, span: TextSpan) {
        self.tag_spans.entry(tag.clone()).or_default().push(span);
    }

    fn record_normalized_line(&mut self, line_index: usize, line: &str) {
        self.normalized_lines
            .entry(line_index)
            .or_insert_with(|| line.to_string());
    }

    fn record_original_line(&mut self, line_index: usize, line: &str) {
        self.original_lines
            .entry(line_index)
            .or_insert_with(|| line.to_string());
    }

    fn record_char_map(&mut self, line_index: usize, map: Vec<usize>) {
        self.normalized_char_maps.entry(line_index).or_insert(map);
    }
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
fn parse_scryfall_mana_cost(raw: &str) -> Result<ManaCost, CardTextError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(ManaCost::new());
    }

    let mut pips = Vec::new();
    let mut rest = trimmed;
    while !rest.is_empty() {
        let Some(after_open) = rest.strip_prefix('{') else {
            return Err(CardTextError::ParseError(format!(
                "expected mana symbol in braces in cost `{raw}`"
            )));
        };
        let Some(end) = after_open.find('}') else {
            return Err(CardTextError::ParseError(format!(
                "unterminated mana symbol in cost `{raw}`"
            )));
        };
        let group = &after_open[..end];
        pips.push(parse_mana_symbol_group(group)?);
        rest = after_open[end + 1..].trim_start();
    }

    Ok(ManaCost::from_pips(pips))
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
fn parse_mana_symbol_group(raw: &str) -> Result<Vec<ManaSymbol>, CardTextError> {
    let alternatives = raw
        .split('/')
        .map(parse_mana_symbol)
        .collect::<Result<Vec<_>, _>>()?;
    if alternatives.is_empty() {
        Err(CardTextError::ParseError(
            "empty mana symbol group".to_string(),
        ))
    } else {
        Ok(alternatives)
    }
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
fn parse_mana_symbol(raw: &str) -> Result<ManaSymbol, CardTextError> {
    match raw.trim().to_ascii_uppercase().as_str() {
        "W" => Ok(ManaSymbol::White),
        "U" => Ok(ManaSymbol::Blue),
        "B" => Ok(ManaSymbol::Black),
        "R" => Ok(ManaSymbol::Red),
        "G" => Ok(ManaSymbol::Green),
        "C" => Ok(ManaSymbol::Colorless),
        "S" => Ok(ManaSymbol::Snow),
        "P" => Ok(ManaSymbol::Life(2)),
        "X" => Ok(ManaSymbol::X),
        value => value
            .parse::<u8>()
            .map(ManaSymbol::Generic)
            .map_err(|_| CardTextError::ParseError(format!("unknown mana symbol `{raw}`"))),
    }
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
fn parse_type_line(
    raw: &str,
) -> Result<(Vec<Supertype>, Vec<CardType>, Vec<Subtype>), CardTextError> {
    let normalized = raw.replace('—', "-");
    let mut supertypes = Vec::new();
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();

    for face in normalized
        .split("//")
        .map(str::trim)
        .filter(|face| !face.is_empty())
    {
        let (type_part, subtype_part) = face
            .split_once('-')
            .map_or((face, ""), |(types, subtypes)| {
                (types.trim(), subtypes.trim())
            });

        for word in type_part.split_whitespace() {
            let lower = word.to_ascii_lowercase();
            match lower.as_str() {
                "basic" => push_unique(&mut supertypes, Supertype::Basic),
                "legendary" => push_unique(&mut supertypes, Supertype::Legendary),
                "ongoing" => push_unique(&mut supertypes, Supertype::Ongoing),
                "snow" => push_unique(&mut supertypes, Supertype::Snow),
                "world" => push_unique(&mut supertypes, Supertype::World),
                "land" => push_unique(&mut card_types, CardType::Land),
                "creature" => push_unique(&mut card_types, CardType::Creature),
                "artifact" => push_unique(&mut card_types, CardType::Artifact),
                "enchantment" => push_unique(&mut card_types, CardType::Enchantment),
                "planeswalker" => push_unique(&mut card_types, CardType::Planeswalker),
                "instant" => push_unique(&mut card_types, CardType::Instant),
                "sorcery" => push_unique(&mut card_types, CardType::Sorcery),
                "battle" => push_unique(&mut card_types, CardType::Battle),
                "plane" => push_unique(&mut card_types, CardType::Plane),
                "phenomenon" => push_unique(&mut card_types, CardType::Phenomenon),
                "vanguard" => push_unique(&mut card_types, CardType::Vanguard),
                "scheme" => push_unique(&mut card_types, CardType::Scheme),
                "conspiracy" => push_unique(&mut card_types, CardType::Conspiracy),
                "kindred" | "tribal" => push_unique(&mut card_types, CardType::Kindred),
                _ => {
                    return Err(CardTextError::ParseError(format!(
                        "unknown type word `{word}` in `{raw}`"
                    )));
                }
            }
        }

        for word in subtype_part.split_whitespace() {
            if let Some(subtype) = parse_subtype_word(word) {
                push_unique(&mut subtypes, subtype);
            } else {
                return Err(CardTextError::ParseError(format!(
                    "unknown subtype word `{word}` in `{raw}`"
                )));
            }
        }
    }

    Ok((supertypes, card_types, subtypes))
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
fn push_unique<T: PartialEq>(values: &mut Vec<T>, value: T) {
    if !values.contains(&value) {
        values.push(value);
    }
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
fn parse_subtype_word(raw: &str) -> Option<Subtype> {
    let normalized = normalize_type_word(raw);
    SubtypeFamily::Land
        .all_subtypes()
        .iter()
        .chain(SubtypeFamily::Creature.all_subtypes())
        .chain(SubtypeFamily::Artifact.all_subtypes())
        .chain(SubtypeFamily::Enchantment.all_subtypes())
        .chain(SubtypeFamily::Spell.all_subtypes())
        .chain(SubtypeFamily::Planeswalker.all_subtypes())
        .chain(SubtypeFamily::Battle.all_subtypes())
        .copied()
        .find(|subtype| {
            let display = normalize_type_word(&subtype.display_name());
            display == normalized
                || display.strip_suffix('s') == Some(normalized.as_str())
                || normalized.strip_suffix('s') == Some(display.as_str())
        })
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
fn normalize_type_word(raw: &str) -> String {
    raw.chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() {
                Some(ch.to_ascii_lowercase())
            } else {
                None
            }
        })
        .collect()
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
fn parse_power_toughness(raw: &str) -> Option<PowerToughness> {
    let (power, toughness) = raw.trim().split_once('/')?;
    Some(PowerToughness::new(
        parse_pt_value(power.trim())?,
        parse_pt_value(toughness.trim())?,
    ))
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
fn parse_pt_value(raw: &str) -> Option<PtValue> {
    if raw == ".5" || raw == "0.5" {
        return Some(PtValue::Fixed(0));
    }
    if raw == "*" {
        return Some(PtValue::Star);
    }
    if let Some(stripped) = raw.strip_prefix("*+") {
        return stripped.trim().parse::<i32>().ok().map(PtValue::StarPlus);
    }
    if let Some(stripped) = raw.strip_suffix("+*") {
        return stripped.trim().parse::<i32>().ok().map(PtValue::StarPlus);
    }
    raw.parse::<i32>().ok().map(PtValue::Fixed)
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IfResultPredicate {
    Did,
    DidNot,
    DiesThisWay,
    ExcessDamageDealt,
    WasDeclined,
    Value(crate::effect::Comparison),
}

#[cfg(any(test, ironsmith_runtime_parser_tests))]
const IT_TAG: &str = "__it__";

/// Builder for creating CardDefinitions with abilities.
#[derive(Debug, Clone)]
pub struct CardDefinitionBuilder {
    /// The underlying card builder
    card_builder: CardBuilder,

    /// Abilities to add to the card
    abilities: Vec<Ability>,

    /// Spell effects for instants/sorceries
    spell_effect: Option<ResolutionProgram>,

    /// Alternative casting methods (flashback, escape, etc.)
    alternative_casts: Vec<AlternativeCastingMethod>,

    /// Optional costs (kicker, buyback, etc.)
    optional_costs: Vec<OptionalCost>,

    /// Additional non-printed costs paid while casting this spell.
    additional_cost: TotalCost,

    /// For Auras: what this card can enchant (used for non-target attachments)
    aura_attach_filter: Option<AuraAttachmentFilter>,

    /// True if this split card may be cast fused from hand.
    has_fuse: bool,
}

#[cfg(ironsmith_runtime_parser_tests)]
fn compile_to_runtime_definition(
    builder: &CardDefinitionBuilder,
    text: impl Into<String>,
    allow_unsupported: bool,
) -> Result<CardDefinition, CardTextError> {
    compiler_runtime_for_tests::compile_runtime_builder_snapshot_to_runtime_definition(
        runtime_builder_snapshot(builder),
        text,
        allow_unsupported,
    )
    .map_err(compiler_runtime_error_to_card_text_error)
}

#[cfg(ironsmith_runtime_parser_tests)]
fn runtime_builder_snapshot(
    builder: &CardDefinitionBuilder,
) -> compiler_runtime_for_tests::RuntimeBuilderSnapshot {
    compiler_runtime_for_tests::RuntimeBuilderSnapshot {
        card: builder.card_builder.clone().build(),
        has_fuse: builder.has_fuse,
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
fn compiler_runtime_error_to_card_text_error(
    err: compiler_runtime_for_tests::CompilerIntegrationError,
) -> CardTextError {
    match err {
        compiler_runtime_for_tests::CompilerIntegrationError::Parse(
            ironsmith_compiler::CardTextError::UnsupportedLine(message),
        ) => CardTextError::UnsupportedLine(message),
        compiler_runtime_for_tests::CompilerIntegrationError::Parse(
            ironsmith_compiler::CardTextError::ParseError(message),
        ) => CardTextError::ParseError(message),
        compiler_runtime_for_tests::CompilerIntegrationError::Parse(
            ironsmith_compiler::CardTextError::InvariantViolation(message),
        ) => CardTextError::InvariantViolation(message),
        other => CardTextError::ParseError(other.to_string()),
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
fn parse_annotations_from_compiler(
    annotations: ironsmith_compiler::ParseAnnotations,
) -> ParseAnnotations {
    ParseAnnotations {
        tag_spans: annotations
            .tag_spans
            .into_iter()
            .map(|(tag, spans)| {
                (
                    TagKey::from(tag),
                    spans
                        .into_iter()
                        .map(|span| TextSpan {
                            line: span.line,
                            start: span.start,
                            end: span.end,
                        })
                        .collect(),
                )
            })
            .collect(),
        normalized_lines: annotations.normalized_lines,
        original_lines: annotations.original_lines,
        normalized_char_maps: annotations.normalized_char_maps,
    }
}

impl CardDefinitionBuilder {
    #[cfg(ironsmith_runtime_parser_tests)]
    fn parse_cache_key(&self, text: &str, allow_unsupported: bool) -> ParseCacheKey {
        ParseCacheKey::new(self, text, allow_unsupported)
    }

    #[cfg(any(test, ironsmith_runtime_parser_tests))]
    fn pt_value_text(value: PtValue) -> String {
        match value {
            PtValue::Fixed(n) => n.to_string(),
            PtValue::Star => "*".to_string(),
            PtValue::StarPlus(n) => {
                if n >= 0 {
                    format!("*+{n}")
                } else {
                    format!("*{n}")
                }
            }
        }
    }

    #[cfg(any(test, ironsmith_runtime_parser_tests))]
    fn type_line_text(
        supertypes: &[Supertype],
        card_types: &[CardType],
        subtypes: &[Subtype],
    ) -> Option<String> {
        if supertypes.is_empty() && card_types.is_empty() && subtypes.is_empty() {
            return None;
        }

        let mut left = Vec::new();
        for supertype in supertypes {
            left.push(supertype.to_string());
        }
        for card_type in card_types {
            left.push(card_type.to_string());
        }

        let mut line = left.join(" ");
        if !subtypes.is_empty() {
            let right = subtypes
                .iter()
                .map(std::string::ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ");
            if !line.is_empty() {
                line.push_str(" — ");
            }
            line.push_str(&right);
        }
        Some(line)
    }

    #[cfg(test)]
    fn build_text_with_metadata(&self, rules: &str) -> String {
        let mut lines = Vec::new();
        if let Some(cost) = self.card_builder.mana_cost_ref() {
            lines.push(format!("Mana cost: {}", cost.to_oracle()));
        }
        if let Some(type_line) = Self::type_line_text(
            self.card_builder.supertypes_ref(),
            self.card_builder.card_types_ref(),
            self.card_builder.subtypes_ref(),
        ) {
            lines.push(format!("Type: {type_line}"));
        }
        if let Some(pt) = self.card_builder.power_toughness_ref() {
            lines.push(format!(
                "Power/Toughness: {}/{}",
                Self::pt_value_text(pt.power),
                Self::pt_value_text(pt.toughness)
            ));
        }
        if let Some(loyalty) = self.card_builder.loyalty_ref() {
            lines.push(format!("Loyalty: {loyalty}"));
        }
        if let Some(defense) = self.card_builder.defense_ref() {
            lines.push(format!("Defense: {defense}"));
        }

        if !rules.trim().is_empty() {
            lines.push(rules.trim().to_string());
        }

        lines.join("\n")
    }

    /// Create a new card definition builder.
    pub fn new(id: CardId, name: impl Into<String>) -> Self {
        Self {
            card_builder: CardBuilder::new(id, name),
            abilities: Vec::new(),
            spell_effect: None,
            alternative_casts: Vec::new(),
            optional_costs: Vec::new(),
            additional_cost: TotalCost::free(),
            aura_attach_filter: None,
            has_fuse: false,
        }
    }

    // === Card properties (delegated to CardBuilder) ===

    /// Set the mana cost.
    pub fn mana_cost(mut self, cost: ManaCost) -> Self {
        self.card_builder = self.card_builder.mana_cost(cost);
        self
    }

    /// Set the color indicator.
    pub fn color_indicator(mut self, colors: ColorSet) -> Self {
        self.card_builder = self.card_builder.color_indicator(colors);
        self
    }

    /// Set the supertypes.
    pub fn supertypes(mut self, supertypes: Vec<Supertype>) -> Self {
        self.card_builder = self.card_builder.supertypes(supertypes);
        self
    }

    /// Set the card types.
    pub fn card_types(mut self, types: Vec<CardType>) -> Self {
        self.card_builder = self.card_builder.card_types(types);
        self
    }

    /// Set the subtypes.
    pub fn subtypes(mut self, subtypes: Vec<Subtype>) -> Self {
        self.card_builder = self.card_builder.subtypes(subtypes);
        self
    }

    /// Set the oracle text.
    pub fn oracle_text(mut self, text: impl Into<String>) -> Self {
        self.card_builder = self.card_builder.oracle_text(text);
        self
    }

    /// Link this card to another face by card id.
    pub fn other_face(mut self, face: CardId) -> Self {
        self.card_builder = self.card_builder.other_face(face);
        self
    }

    /// Link this card to another face by name.
    pub fn other_face_name(mut self, name: impl Into<String>) -> Self {
        self.card_builder = self.card_builder.other_face_name(name);
        self
    }

    /// Set the linked-face layout semantics for this card.
    pub fn linked_face_layout(mut self, layout: LinkedFaceLayout) -> Self {
        self.card_builder = self.card_builder.linked_face_layout(layout);
        self
    }

    /// Mark this card as a split card that may be cast fused from hand.
    pub fn has_fuse(mut self) -> Self {
        self.has_fuse = true;
        self
    }

    /// Mark this card as an Aura that enchants objects matching the given filter.
    pub fn enchants(mut self, filter: impl Into<AuraAttachmentFilter>) -> Self {
        let filter = filter.into();
        self.aura_attach_filter = Some(filter.clone());
        self.spell_effect = Some(ResolutionProgram::from_effects(vec![Effect::attach_to(
            filter.target_spec(),
        )]));
        self
    }

    #[cfg(any(test, ironsmith_runtime_parser_tests))]
    fn apply_keyword_action(self, action: KeywordAction) -> Self {
        match action {
            KeywordAction::Flying => self.flying(),
            KeywordAction::Menace => self.menace(),
            KeywordAction::Banding => {
                self.with_ability(Ability::static_ability(StaticAbility::banding()))
            }
            KeywordAction::Hexproof => self.hexproof(),
            KeywordAction::Haste => self.haste(),
            KeywordAction::Improvise => self.improvise(),
            KeywordAction::Convoke => self.convoke(),
            KeywordAction::AffinityForArtifacts => self.affinity_for_artifacts(),
            KeywordAction::Delve => self.delve(),
            KeywordAction::FirstStrike => self.first_strike(),
            KeywordAction::DoubleStrike => self.double_strike(),
            KeywordAction::Deathtouch => self.deathtouch(),
            KeywordAction::Lifelink => self.lifelink(),
            KeywordAction::Vigilance => self.vigilance(),
            KeywordAction::Trample => self.trample(),
            KeywordAction::Reach => self.reach(),
            KeywordAction::Defender => self.defender(),
            KeywordAction::Flash => self.flash(),
            KeywordAction::Phasing => {
                self.with_ability(Ability::static_ability(StaticAbility::phasing()))
            }
            KeywordAction::Indestructible => self.indestructible(),
            KeywordAction::Shroud => self.shroud(),
            KeywordAction::Ward(amount) => self.ward_generic(amount),
            KeywordAction::Wither => self.wither(),
            KeywordAction::Afflict(amount) => self.afflict(amount),
            KeywordAction::Afterlife(amount) => self.afterlife(amount),
            KeywordAction::Fabricate(amount) => self.fabricate(amount),
            KeywordAction::Infect => self.infect(),
            KeywordAction::Undying => self.undying(),
            KeywordAction::Persist => self.persist(),
            KeywordAction::Prowess => self.prowess(),
            KeywordAction::Exalted => self.exalted(),
            KeywordAction::Cascade => self.cascade(),
            KeywordAction::Storm => self.storm(),
            KeywordAction::Gravestorm => self.gravestorm(),
            KeywordAction::Toxic(amount) => self.toxic(amount),
            KeywordAction::Poisonous(amount) => self.poisonous(amount),
            KeywordAction::BattleCry => self.battle_cry(),
            KeywordAction::Dethrone => self.dethrone(),
            KeywordAction::Evolve => self.evolve(),
            KeywordAction::Ingest => self.ingest(),
            KeywordAction::Mentor => self.mentor(),
            KeywordAction::Skulk => self.skulk(),
            KeywordAction::Training => self.training(),
            KeywordAction::Myriad => self.myriad(),
            KeywordAction::Riot => self.riot(),
            KeywordAction::Unleash => self.unleash(),
            KeywordAction::Renown(amount) => self.renown(amount),
            KeywordAction::Modular(amount) => self.modular(amount),
            KeywordAction::ModularSunburst => self.modular_sunburst(),
            KeywordAction::Graft(amount) => self.graft(amount),
            KeywordAction::Soulbond => self.soulbond(),
            KeywordAction::Soulshift(amount) => self.soulshift(amount),
            KeywordAction::SoulshiftValue(value) => self.soulshift_value(value),
            KeywordAction::Recover(cost) => self.recover(cost),
            KeywordAction::Outlast(cost) => self.outlast(cost),
            KeywordAction::Scavenge(cost) => self.scavenge(cost),
            KeywordAction::Unearth(cost) => self.unearth(cost),
            KeywordAction::Embalm(cost) => self.embalm(cost),
            KeywordAction::Eternalize(cost) => self.eternalize(cost),
            KeywordAction::Emerge(cost) => self.emerge(cost),
            KeywordAction::Ninjutsu(cost) => self.ninjutsu(cost),
            KeywordAction::Backup(amount) => self.backup(amount),
            KeywordAction::Cipher => self.cipher(),
            KeywordAction::Dash(cost) => self.dash(cost),
            KeywordAction::Blitz(cost) => self.blitz(cost),
            KeywordAction::BlitzFromGraveyard => self.with_ability(Ability::static_ability(
                StaticAbility::keyword_marker(KeywordAction::BlitzFromGraveyard.display_text()),
            )),
            KeywordAction::Warp(cost) => self.warp(cost),
            KeywordAction::Plot(cost) => self.plot(cost),
            KeywordAction::Melee => self.melee(),
            KeywordAction::Mobilize(amount) => self.mobilize(amount),
            KeywordAction::Suspend { time, cost } => self.suspend(time, cost),
            KeywordAction::Disturb(cost) => self.disturb(cost),
            KeywordAction::Overload(cost) => self.overload(cost),
            KeywordAction::Cleave(cost) => self.cleave(cost),
            KeywordAction::Awaken { amount, cost } => self.awaken(amount, cost),
            KeywordAction::Spectacle(cost) => self.spectacle(cost),
            KeywordAction::Foretell(cost) => self.foretell(cost),
            KeywordAction::Echo { total_cost, .. } => self.echo(total_cost),
            KeywordAction::CumulativeUpkeep {
                mana_symbols_per_counter,
                life_per_counter,
                ..
            } => self.cumulative_upkeep(mana_symbols_per_counter, life_per_counter),
            KeywordAction::Casualty(power) => self.casualty(power),
            KeywordAction::VariableCasualtyPlaneswalkerCopy => {
                self.variable_casualty_planeswalker_copy()
            }
            KeywordAction::Demonstrate => self.demonstrate(),
            KeywordAction::Conspire => self.conspire(),
            KeywordAction::Amplify(amount) => self.amplify(amount),
            KeywordAction::AuraSwap(cost) => self.aura_swap(cost),
            KeywordAction::Devour(multiplier) => self.devour(multiplier),
            KeywordAction::Ravenous => self.ravenous(),
            KeywordAction::Ascend => self.ascend(),
            KeywordAction::Daybound => self.daybound(),
            KeywordAction::Nightbound => self.nightbound(),
            KeywordAction::Haunt => self.haunt(),
            KeywordAction::Provoke => self.provoke(),
            KeywordAction::Undaunted => self.undaunted(),
            KeywordAction::Enlist => self.enlist(),
            KeywordAction::Extort => self.extort(),
            KeywordAction::Partner => self.partner(),
            KeywordAction::StartYourEngines => {
                self.with_ability(Ability::static_ability(StaticAbility::start_your_engines()))
            }
            KeywordAction::Assist => self.assist(),
            KeywordAction::SplitSecond => self.split_second(),
            KeywordAction::Rebound => self.rebound(),
            KeywordAction::Sunburst => self.sunburst(),
            KeywordAction::ReadAhead => self.read_ahead(),
            KeywordAction::Firebending(amount) => self.firebending(amount),
            KeywordAction::Fading(amount) => self.fading(amount),
            KeywordAction::Vanishing(amount) => self.vanishing(amount),
            KeywordAction::Fear => self.fear(),
            KeywordAction::Intimidate => self.intimidate(),
            KeywordAction::Shadow => self.shadow(),
            KeywordAction::Horsemanship => self.horsemanship(),
            KeywordAction::Flanking => {
                self.with_ability(Ability::static_ability(StaticAbility::flanking()))
            }
            KeywordAction::UmbraArmor => {
                self.with_ability(Ability::static_ability(StaticAbility::umbra_armor()))
            }
            KeywordAction::Landwalk(kind) => {
                let ability = match kind {
                    crate::static_abilities::LandwalkKind::Subtype {
                        subtype,
                        snow: false,
                    } => StaticAbility::landwalk(subtype),
                    crate::static_abilities::LandwalkKind::Subtype {
                        subtype,
                        snow: true,
                    } => StaticAbility::snow_landwalk(subtype),
                    crate::static_abilities::LandwalkKind::AnyLand => StaticAbility::any_landwalk(),
                    crate::static_abilities::LandwalkKind::NonbasicLand => {
                        StaticAbility::nonbasic_landwalk()
                    }
                    crate::static_abilities::LandwalkKind::ArtifactLand => {
                        StaticAbility::artifact_landwalk()
                    }
                };
                self.with_ability(Ability::static_ability(ability))
            }
            KeywordAction::Bloodthirst(amount) => self.bloodthirst(amount),
            KeywordAction::Tribute(amount) => self.tribute(amount),
            KeywordAction::Rampage(amount) => self.rampage(amount),
            KeywordAction::Bushido(amount) => self.bushido(amount),
            KeywordAction::Frenzy(amount) => self.frenzy(amount),
            KeywordAction::Changeling => {
                self.with_ability(Ability::static_ability(StaticAbility::changeling()))
            }
            KeywordAction::HexproofFrom(filter) => self.with_ability(Ability::static_ability(
                StaticAbility::hexproof_from(filter),
            )),
            KeywordAction::ProtectionFrom(colors) => self.protection_from(colors),
            KeywordAction::ProtectionFromAllColors => self.with_ability(Ability::static_ability(
                StaticAbility::protection(crate::ability::ProtectionFrom::AllColors),
            )),
            KeywordAction::ProtectionFromColorless => self.with_ability(Ability::static_ability(
                StaticAbility::protection(crate::ability::ProtectionFrom::Colorless),
            )),
            KeywordAction::ProtectionFromEverything => self.with_ability(Ability::static_ability(
                StaticAbility::protection(crate::ability::ProtectionFrom::Everything),
            )),
            KeywordAction::ProtectionFromChosenPlayer => {
                self.with_ability(Ability::static_ability(StaticAbility::protection(
                    crate::ability::ProtectionFrom::ChosenPlayer,
                )))
            }
            KeywordAction::ProtectionFromChosenColor => self.with_ability(Ability::static_ability(
                StaticAbility::protection(crate::ability::ProtectionFrom::ChosenColor),
            )),
            KeywordAction::ProtectionFromFilter(filter) => self.protection_from_filter(filter),
            KeywordAction::ProtectionFromEachManaValueAmong(filter) => {
                self.with_ability(Ability::static_ability(StaticAbility::protection(
                    crate::ability::ProtectionFrom::EachManaValueAmong(filter),
                )))
            }
            KeywordAction::ProtectionFromCardType(card_type) => {
                self.protection_from_card_type(card_type)
            }
            KeywordAction::ProtectionFromSubtype(subtype) => self.protection_from_subtype(subtype),
            KeywordAction::Unblockable => self.unblockable(),
            KeywordAction::Devoid => self.with_ability(
                Ability::static_ability(StaticAbility::make_colorless(ObjectFilter::source()))
                    .in_zones(vec![
                        Zone::Battlefield,
                        Zone::Stack,
                        Zone::Hand,
                        Zone::Library,
                        Zone::Graveyard,
                        Zone::Exile,
                        Zone::Command,
                    ]),
            ),
            KeywordAction::Annihilator(amount) => self.with_ability(Ability {
                kind: AbilityKind::Triggered(TriggeredAbility {
                    trigger: Trigger::this_attacks(),
                    effects: crate::resolution::ResolutionProgram::from_effects(vec![
                        Effect::sacrifice_player(
                            ObjectFilter::permanent(),
                            Value::Fixed(amount as i32),
                            PlayerFilter::Defending,
                        ),
                    ]),
                    choices: vec![],
                    intervening_if: None,
                    presentation_label: None,
                }),
                functional_zones: vec![Zone::Battlefield],
            }),
            KeywordAction::ForMirrodin => self.for_mirrodin(),
            KeywordAction::LivingWeapon => self.living_weapon(),
            KeywordAction::Crew {
                amount,
                timing,
                additional_restrictions,
            } => {
                let cost = TotalCost::from_cost(crate::costs::Cost::effect(
                    crate::effects::CrewCostEffect::new(amount),
                ));
                let animate = Effect::new(crate::effects::ApplyContinuousEffect::new(
                    crate::continuous::EffectTarget::Source,
                    crate::continuous::Modification::AddCardTypes(vec![CardType::Creature]),
                    Until::EndOfTurn,
                ));
                self.with_ability(Ability {
                    kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                        mana_cost: cost,
                        effects: crate::resolution::ResolutionProgram::from_effects(vec![animate]),
                        choices: Vec::new(),
                        timing,
                        additional_restrictions,
                        activation_restrictions: vec![],
                        mana_output: None,
                        activation_condition: None,
                        mana_usage_restrictions: vec![],
                        is_loyalty_ability: false,
                    }),
                    functional_zones: vec![Zone::Battlefield],
                })
            }
            KeywordAction::Saddle {
                amount,
                timing,
                additional_restrictions,
            } => {
                let cost = TotalCost::from_cost(crate::costs::Cost::effect(
                    crate::effects::SaddleCostEffect::new(amount),
                ));
                let saddle = Effect::new(crate::effects::BecomeSaddledUntilEotEffect::new());
                self.with_ability(Ability {
                    kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                        mana_cost: cost,
                        effects: crate::resolution::ResolutionProgram::from_effects(vec![saddle]),
                        choices: Vec::new(),
                        timing,
                        additional_restrictions,
                        activation_restrictions: vec![],
                        mana_output: None,
                        activation_condition: None,
                        mana_usage_restrictions: vec![],
                        is_loyalty_ability: false,
                    }),
                    functional_zones: vec![Zone::Battlefield],
                })
            }
            KeywordAction::Marker(name) => {
                if name.eq_ignore_ascii_case("fuse") {
                    self.has_fuse()
                } else if let Some(amount) = parse_standalone_bolster_marker(name) {
                    self.with_standalone_bolster_effect(amount)
                } else if supported_keyword_marker_text(name) {
                    self.with_ability(Ability::static_ability(StaticAbility::keyword_marker(name)))
                } else {
                    self.with_ability(Ability::static_ability(
                        StaticAbility::keyword_fallback_text(name),
                    ))
                }
            }
            KeywordAction::MarkerText(text) => {
                if text.eq_ignore_ascii_case("fuse") {
                    self.has_fuse()
                } else if let Some(amount) = parse_standalone_bolster_marker(&text) {
                    self.with_standalone_bolster_effect(amount)
                } else if supported_keyword_marker_text(&text) {
                    self.with_ability(Ability::static_ability(StaticAbility::keyword_marker(text)))
                } else {
                    self.with_ability(Ability::static_ability(
                        StaticAbility::keyword_fallback_text(text),
                    ))
                }
            }
        }
    }

    fn with_standalone_bolster_effect(mut self, amount: u32) -> Self {
        if !self
            .card_builder
            .card_types_ref()
            .iter()
            .any(|card_type| matches!(card_type, CardType::Instant | CardType::Sorcery))
        {
            return self.with_ability(Ability::static_ability(
                StaticAbility::keyword_fallback_text(format!("Bolster {amount}")),
            ));
        }

        let effect = Effect::bolster(amount);
        if let Some(existing) = &mut self.spell_effect {
            existing.push(effect);
        } else {
            self.spell_effect = Some(crate::resolution::ResolutionProgram::from_effects(vec![
                effect,
            ]));
        }
        self
    }

    /// Build a CardDefinition from oracle text.
    #[cfg(ironsmith_runtime_parser_tests)]
    pub fn parse_text(self, text: impl Into<String>) -> Result<CardDefinition, CardTextError> {
        parse_card_text(self, text)
    }

    /// Build a CardDefinition from oracle text.
    #[cfg(all(test, not(ironsmith_runtime_parser_tests)))]
    pub fn parse_text(self, text: impl Into<String>) -> Result<CardDefinition, CardTextError> {
        let _ = (self, text.into());
        Err(CardTextError::InvariantViolation(
            "runtime parser tests moved to ironsmith-registry".to_string(),
        ))
    }

    /// Build a CardDefinition from oracle text, preserving unsupported lines as markers.
    #[cfg(ironsmith_runtime_parser_tests)]
    pub fn parse_text_allow_unsupported(
        self,
        text: impl Into<String>,
    ) -> Result<CardDefinition, CardTextError> {
        parse_card_text_allow_unsupported(self, text)
    }

    /// Build a CardDefinition from oracle text, preserving unsupported lines as markers.
    #[cfg(all(test, not(ironsmith_runtime_parser_tests)))]
    pub fn parse_text_allow_unsupported(
        self,
        text: impl Into<String>,
    ) -> Result<CardDefinition, CardTextError> {
        let _ = (self, text.into());
        Err(CardTextError::InvariantViolation(
            "runtime parser tests moved to ironsmith-registry".to_string(),
        ))
    }

    /// Build a CardDefinition from oracle text, returning parse annotations.
    #[cfg(ironsmith_runtime_parser_tests)]
    pub fn parse_text_with_annotations(
        self,
        text: impl Into<String>,
    ) -> Result<(CardDefinition, ParseAnnotations), CardTextError> {
        parse_card_text_with_annotations(self, text)
    }

    /// Build a CardDefinition from oracle text, returning parse annotations while
    /// preserving unsupported lines as markers.
    #[cfg(ironsmith_runtime_parser_tests)]
    pub fn parse_text_with_annotations_allow_unsupported(
        self,
        text: impl Into<String>,
    ) -> Result<(CardDefinition, ParseAnnotations), CardTextError> {
        parse_card_text_with_annotations_allow_unsupported(self, text)
    }

    /// Build a CardDefinition from oracle text, prepending metadata lines
    /// derived from the builder's current fields (mana cost, type line, etc.).
    #[cfg(test)]
    pub fn from_text_with_metadata(
        self,
        text: impl Into<String>,
    ) -> Result<CardDefinition, CardTextError> {
        let text = self.build_text_with_metadata(&text.into());
        self.parse_text(text)
    }

    /// Backwards-compatible wrapper for prepending metadata to rules text.
    #[cfg(test)]
    pub fn text_box(self, text: impl Into<String>) -> Result<CardDefinition, CardTextError> {
        self.parse_text(text)
    }

    /// Build a CardDefinition from oracle text with metadata, without parsing rules text.
    /// Useful for cards with custom/manual abilities where parsing may be incomplete.
    #[cfg(ironsmith_runtime_parser_tests)]
    pub fn from_text_with_metadata_oracle_only(self, text: impl Into<String>) -> CardDefinition {
        fn pt_value_text(value: PtValue) -> String {
            match value {
                PtValue::Fixed(n) => n.to_string(),
                PtValue::Star => "*".to_string(),
                PtValue::StarPlus(n) => {
                    if n >= 0 {
                        format!("*+{n}")
                    } else {
                        format!("*{n}")
                    }
                }
            }
        }

        fn type_line_text(
            supertypes: &[Supertype],
            card_types: &[CardType],
            subtypes: &[Subtype],
        ) -> Option<String> {
            if supertypes.is_empty() && card_types.is_empty() && subtypes.is_empty() {
                return None;
            }

            let mut left = Vec::new();
            for supertype in supertypes {
                left.push(supertype.to_string());
            }
            for card_type in card_types {
                left.push(card_type.to_string());
            }

            let mut line = left.join(" ");
            if !subtypes.is_empty() {
                let right = subtypes
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" ");
                if !line.is_empty() {
                    line.push_str(" — ");
                }
                line.push_str(&right);
            }
            Some(line)
        }

        let mut lines = Vec::new();
        if let Some(cost) = self.card_builder.mana_cost_ref() {
            lines.push(format!("Mana cost: {}", cost.to_oracle()));
        }
        if let Some(type_line) = type_line_text(
            self.card_builder.supertypes_ref(),
            self.card_builder.card_types_ref(),
            self.card_builder.subtypes_ref(),
        ) {
            lines.push(format!("Type: {type_line}"));
        }
        if let Some(pt) = self.card_builder.power_toughness_ref() {
            lines.push(format!(
                "Power/Toughness: {}/{}",
                pt_value_text(pt.power),
                pt_value_text(pt.toughness)
            ));
        }
        if let Some(loyalty) = self.card_builder.loyalty_ref() {
            lines.push(format!("Loyalty: {loyalty}"));
        }
        if let Some(defense) = self.card_builder.defense_ref() {
            lines.push(format!("Defense: {defense}"));
        }

        let rules = text.into();
        if !rules.trim().is_empty() {
            lines.push(rules.trim().to_string());
        }

        let combined = lines.join("\n");
        self.oracle_text(combined).build()
    }

    #[cfg(any(test, ironsmith_runtime_parser_tests))]
    fn apply_metadata(mut self, meta: impl Into<MetadataLine>) -> Result<Self, CardTextError> {
        let meta = meta.into();
        match meta {
            MetadataLine::ManaCost(raw) => {
                let cost = parse_scryfall_mana_cost(&raw)?;
                if !cost.is_empty() {
                    self.card_builder = self.card_builder.mana_cost(cost);
                }
            }
            MetadataLine::TypeLine(raw) => {
                let (supertypes, card_types, subtypes) = parse_type_line(&raw)?;
                if !supertypes.is_empty() {
                    self.card_builder = self.card_builder.supertypes(supertypes);
                }
                if !card_types.is_empty() {
                    self.card_builder = self.card_builder.card_types(card_types);
                }
                if !subtypes.is_empty() {
                    self.card_builder = self.card_builder.subtypes(subtypes);
                }
            }
            MetadataLine::PowerToughness(raw) => {
                if let Some(pt) = parse_power_toughness(&raw) {
                    self.card_builder = self.card_builder.power_toughness(pt);
                }
            }
            MetadataLine::Loyalty(raw) => {
                if let Ok(value) = raw.trim().parse::<u32>() {
                    self.card_builder = self.card_builder.loyalty(value);
                }
            }
            MetadataLine::Defense(raw) => {
                if let Ok(value) = raw.trim().parse::<u32>() {
                    self.card_builder = self.card_builder.defense(value);
                }
            }
        }

        Ok(self)
    }

    /// Set the power/toughness.
    pub fn power_toughness(mut self, pt: PowerToughness) -> Self {
        self.card_builder = self.card_builder.power_toughness(pt);
        self
    }

    /// Set the starting loyalty.
    pub fn loyalty(mut self, loyalty: u32) -> Self {
        self.card_builder = self.card_builder.loyalty(loyalty);
        self
    }

    /// Set the defense value.
    pub fn defense(mut self, defense: u32) -> Self {
        self.card_builder = self.card_builder.defense(defense);
        self
    }

    /// Mark this card as a token.
    ///
    /// Tokens are not real cards - they are created by effects and cease to exist
    /// when they leave the battlefield.
    pub fn token(mut self) -> Self {
        self.card_builder = self.card_builder.token();
        self
    }

    // === Ability methods ===

    /// Add abilities to the card.
    pub fn with_abilities(mut self, abilities: Vec<Ability>) -> Self {
        self.abilities.extend(
            abilities
                .into_iter()
                .map(ability_with_inherent_functional_zones),
        );
        self
    }

    /// Add a single ability to the card.
    pub fn with_ability(mut self, ability: Ability) -> Self {
        self.abilities
            .push(ability_with_inherent_functional_zones(ability));
        self
    }

    // === Keyword shortcuts ===

    /// Add flying.
    pub fn flying(self) -> Self {
        self.with_ability(ability::flying())
    }

    /// Add first strike.
    pub fn first_strike(self) -> Self {
        self.with_ability(ability::first_strike())
    }

    /// Add double strike.
    pub fn double_strike(self) -> Self {
        self.with_ability(ability::double_strike())
    }

    /// Add deathtouch.
    pub fn deathtouch(self) -> Self {
        self.with_ability(ability::deathtouch())
    }

    /// Add lifelink.
    pub fn lifelink(self) -> Self {
        self.with_ability(ability::lifelink())
    }

    /// Add vigilance.
    pub fn vigilance(self) -> Self {
        self.with_ability(ability::vigilance())
    }

    /// Add trample.
    pub fn trample(self) -> Self {
        self.with_ability(ability::trample())
    }

    /// Add haste.
    pub fn haste(self) -> Self {
        self.with_ability(ability::haste())
    }

    /// Add reach.
    pub fn reach(self) -> Self {
        self.with_ability(ability::reach())
    }

    /// Add defender.
    pub fn defender(self) -> Self {
        self.with_ability(ability::defender())
    }

    /// Add hexproof.
    pub fn hexproof(self) -> Self {
        self.with_ability(ability::hexproof())
    }

    /// Add ward with a mana cost.
    ///
    /// Ward is a triggered ability that counters spells or abilities that target
    /// this permanent unless the opponent pays the ward cost.
    ///
    /// Example: `ward(TotalCost::mana("{3}"))` for "Ward {3}"
    pub fn ward(self, cost: TotalCost) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::ward(cost)))
    }

    /// Add ward with a generic mana cost.
    ///
    /// Convenience method for the common case of ward with just generic mana.
    /// Example: `ward_generic(3)` for "Ward {3}"
    pub fn ward_generic(self, amount: u32) -> Self {
        use crate::mana::{ManaCost, ManaSymbol};
        let mana = ManaCost::from_pips(vec![vec![ManaSymbol::Generic(amount as u8)]]);
        self.ward(TotalCost::mana(mana))
    }

    /// Add indestructible.
    pub fn indestructible(self) -> Self {
        self.with_ability(ability::indestructible())
    }

    /// Add menace.
    pub fn menace(self) -> Self {
        self.with_ability(ability::menace())
    }

    /// Add flash.
    pub fn flash(self) -> Self {
        self.with_ability(ability::flash())
    }

    /// Add shroud.
    pub fn shroud(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::shroud()))
    }

    /// Add wither.
    pub fn wither(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::wither()))
    }

    /// Add infect.
    pub fn infect(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::infect()))
    }

    /// Add undying.
    ///
    /// Undying is a triggered ability: "When this creature dies, if it had no +1/+1
    /// counters on it, return it to the battlefield under its owner's control with
    /// a +1/+1 counter on it."
    pub fn undying(self) -> Self {
        let trigger_tag = "undying_trigger";
        let return_tag = "undying_return";
        let returned_tag = "undying_returned";

        let filter = ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .same_stable_id_as_tagged(trigger_tag);

        let tag_return = Effect::new(crate::effects::TagMatchingObjectsEffect::new(
            filter, return_tag,
        ));
        let move_to_battlefield = Effect::new(
            crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Tagged(return_tag.into()),
                Zone::Battlefield,
                true,
            )
            .under_owner_control(),
        )
        .tag(returned_tag);
        let counters = Effect::for_each_tagged(
            returned_tag,
            vec![Effect::put_counters(
                CounterType::PlusOnePlusOne,
                1,
                ChooseSpec::Iterated,
            )],
        );
        let effects = vec![
            Effect::tag_triggering_object(trigger_tag),
            tag_return,
            move_to_battlefield,
            counters,
        ];
        self.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::this_dies(),
                effects: effects.into(),
                choices: vec![],
                intervening_if: Some(Condition::Not(Box::new(
                    Condition::TriggeringObjectHadCounters {
                        counter_type: CounterType::PlusOnePlusOne,
                        min_count: 1,
                    },
                ))),
                presentation_label: None,
            }),
            // Functions from both zones because triggers can be checked at different points:
            // - From Battlefield: SBAs check triggers BEFORE moving object to graveyard
            // - From Graveyard: Sacrifices check triggers AFTER moving object
            functional_zones: vec![crate::zone::Zone::Battlefield, crate::zone::Zone::Graveyard],
        })
    }

    /// Add persist.
    ///
    /// Persist is a triggered ability: "When this creature dies, if it had no -1/-1
    /// counters on it, return it to the battlefield under its owner's control with
    /// a -1/-1 counter on it."
    pub fn persist(self) -> Self {
        let trigger_tag = "persist_trigger";
        let return_tag = "persist_return";
        let returned_tag = "persist_returned";

        let filter = ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .same_stable_id_as_tagged(trigger_tag);

        let tag_return = Effect::new(crate::effects::TagMatchingObjectsEffect::new(
            filter, return_tag,
        ));
        let move_to_battlefield = Effect::new(
            crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Tagged(return_tag.into()),
                Zone::Battlefield,
                true,
            )
            .under_owner_control(),
        )
        .tag(returned_tag);
        let counters = Effect::for_each_tagged(
            returned_tag,
            vec![Effect::put_counters(
                CounterType::MinusOneMinusOne,
                1,
                ChooseSpec::Iterated,
            )],
        );
        let effects = vec![
            Effect::tag_triggering_object(trigger_tag),
            tag_return,
            move_to_battlefield,
            counters,
        ];
        self.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::this_dies(),
                effects: effects.into(),
                choices: vec![],
                intervening_if: Some(Condition::Not(Box::new(
                    Condition::TriggeringObjectHadCounters {
                        counter_type: CounterType::MinusOneMinusOne,
                        min_count: 1,
                    },
                ))),
                presentation_label: None,
            }),
            // Functions from both zones because triggers can be checked at different points:
            // - From Battlefield: SBAs check triggers BEFORE moving object to graveyard
            // - From Graveyard: Sacrifices check triggers AFTER moving object
            functional_zones: vec![crate::zone::Zone::Battlefield, crate::zone::Zone::Graveyard],
        })
    }

    /// Add prowess.
    ///
    /// Prowess means "Whenever you cast a noncreature spell, this creature gets +1/+1 until
    /// end of turn."
    pub fn prowess(self) -> Self {
        self.with_ability(Ability::triggered(
            Trigger::spell_cast(Some(ObjectFilter::noncreature_spell()), PlayerFilter::You),
            vec![Effect::pump(1, 1, ChooseSpec::Source, Until::EndOfTurn)],
        ))
    }

    /// Add exalted.
    ///
    /// Exalted means "Whenever a creature you control attacks alone, that creature gets +1/+1
    /// until end of turn."
    pub fn exalted(self) -> Self {
        let attacker_tag = "exalted_attacker";
        self.with_ability(Ability::triggered(
            Trigger::attacks_alone(ObjectFilter::creature().you_control()),
            vec![
                Effect::tag_triggering_object(attacker_tag),
                Effect::pump(
                    1,
                    1,
                    ChooseSpec::Tagged(attacker_tag.into()),
                    Until::EndOfTurn,
                ),
            ],
        ))
    }

    /// Add toxic N.
    ///
    /// Toxic N means "Players dealt combat damage by this creature also get N poison counters."
    pub fn toxic(self, amount: u32) -> Self {
        self.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::this_deals_combat_damage_to_player(PlayerFilter::Any),
                effects: vec![Effect::poison_counters_player(
                    amount as i32,
                    PlayerFilter::DamagedPlayer,
                )]
                .into(),
                choices: vec![],
                intervening_if: None,
                presentation_label: Some(ability::PresentationLabel::Keyword(
                    ability::PresentationKeyword::Toxic(amount),
                )),
            }),
            functional_zones: vec![Zone::Battlefield],
        })
    }

    /// Add poisonous N.
    pub fn poisonous(self, amount: u32) -> Self {
        self.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::this_deals_combat_damage_to_player(PlayerFilter::Any),
                effects: vec![Effect::poison_counters_player(
                    amount as i32,
                    PlayerFilter::DamagedPlayer,
                )]
                .into(),
                choices: vec![],
                intervening_if: None,
                presentation_label: Some(ability::PresentationLabel::Keyword(
                    ability::PresentationKeyword::Poisonous(amount),
                )),
            }),
            functional_zones: vec![Zone::Battlefield],
        })
    }

    /// Add battle cry.
    ///
    /// Battle cry means "Whenever this creature attacks, each other attacking creature
    /// gets +1/+0 until end of turn."
    pub fn battle_cry(self) -> Self {
        let mut filter = ObjectFilter::creature().you_control().other();
        filter.attacking = true;
        self.with_ability(Ability::triggered(
            Trigger::this_attacks(),
            vec![Effect::pump_all(filter, 1, 0, Until::EndOfTurn)],
        ))
    }

    /// Add Firebending N (CR 702.189).
    pub fn firebending(self, amount: u32) -> Self {
        let add_mana = Effect::new(crate::effects::AddScaledManaEffect::new(
            vec![ManaSymbol::Red],
            Value::Fixed(amount as i32),
            PlayerFilter::You,
        ));
        self.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::this_attacks(),
                effects: ResolutionProgram::from_effects(vec![
                    Effect::new(crate::effects::ManaRetainedEffect::until_end_of_combat(
                        vec![add_mana],
                    )),
                    Effect::emit_keyword_action(crate::events::KeywordActionKind::Firebend, 1),
                ]),
                choices: Vec::new(),
                intervening_if: None,
                presentation_label: Some(ability::PresentationLabel::Keyword(
                    ability::PresentationKeyword::Firebending(amount.to_string()),
                )),
            }),
            functional_zones: vec![Zone::Battlefield],
        })
    }

    /// Add melee.
    ///
    /// Melee means "Whenever this creature attacks, it gets +1/+1 until end of
    /// turn for each opponent you attacked this combat."
    pub fn melee(self) -> Self {
        self.with_ability(Ability::triggered(
            Trigger::this_attacks(),
            vec![Effect::new(crate::effects::MeleeEffect::new())],
        ))
    }

    /// Add dethrone.
    ///
    /// Dethrone means "Whenever this creature attacks the player with the most life
    /// or tied for most life, put a +1/+1 counter on it."
    pub fn dethrone(self) -> Self {
        self.with_ability(Ability::triggered(
            Trigger::this_attacks_player_with_most_life(),
            vec![Effect::plus_one_counters(1, ChooseSpec::Source)],
        ))
    }

    /// Add evolve.
    ///
    /// Evolve means "Whenever a creature enters under your control, if that creature has
    /// greater power or toughness than this creature, put a +1/+1 counter on this creature."
    pub fn evolve(self) -> Self {
        self.with_ability(Ability::triggered(
            Trigger::enters_battlefield(ObjectFilter::creature().you_control(), None),
            vec![Effect::evolve_source()],
        ))
    }

    /// Add mentor.
    ///
    /// Mentor means "Whenever this creature attacks, put a +1/+1 counter on target attacking
    /// creature with lesser power."
    pub fn mentor(self) -> Self {
        let mut target_filter = ObjectFilter::creature().with_power_less_than_source();
        target_filter.attacking = true;
        let target = ChooseSpec::target(ChooseSpec::Object(target_filter));

        self.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::this_attacks(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::plus_one_counters(1, target.clone()),
                ]),
                choices: vec![target],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![Zone::Battlefield],
        })
    }

    /// Add training.
    ///
    /// Training means "Whenever this creature attacks with another creature with greater power,
    /// put a +1/+1 counter on this creature."
    pub fn training(self) -> Self {
        self.with_ability(Ability::triggered(
            Trigger::this_attacks_with_greater_power(),
            vec![
                Effect::plus_one_counters(1, ChooseSpec::Source),
                Effect::emit_keyword_action(crate::events::KeywordActionKind::Train, 1),
            ],
        ))
    }

    /// Add renown N.
    ///
    /// Renown N means "When this creature deals combat damage to a player, if it isn't renowned,
    /// put N +1/+1 counters on it and it becomes renowned."
    pub fn renown(self, amount: u32) -> Self {
        self.with_ability(Ability::triggered(
            Trigger::this_deals_combat_damage_to_player(PlayerFilter::Any),
            vec![Effect::renown_source(amount)],
        ))
    }

    /// Add soulbond.
    ///
    /// Soulbond means "You may pair this creature with another unpaired creature
    /// when either enters. They remain paired while you control both."
    pub fn soulbond(self) -> Self {
        self.with_ability(Ability::triggered(
            Trigger::enters_battlefield(ObjectFilter::creature().you_control(), None),
            vec![Effect::new(crate::effects::SoulbondPairEffect::new())],
        ))
    }

    /// Add soulshift N.
    ///
    /// Soulshift means "When this creature dies, you may return target Spirit card
    /// with mana value N or less from your graveyard to your hand."
    pub fn soulshift(self, amount: u32) -> Self {
        self.with_ability(Self::soulshift_triggered_ability(
            crate::filter::Comparison::LessThanOrEqual(amount as i32),
            None,
        ))
    }

    /// Add soulshift X, where X is a dynamic value.
    pub fn soulshift_value(self, amount: Value) -> Self {
        self.with_ability(Self::soulshift_triggered_ability(
            crate::filter::Comparison::LessThanOrEqualExpr(Box::new(amount)),
            Some(ability::PresentationLabel::Keyword(
                ability::PresentationKeyword::Soulshift("X".to_string()),
            )),
        ))
    }

    fn soulshift_triggered_ability(
        mana_value: crate::filter::Comparison,
        presentation_label: Option<ability::PresentationLabel>,
    ) -> Ability {
        let filter = ObjectFilter::default()
            .with_subtype(Subtype::Spirit)
            .owned_by(PlayerFilter::You)
            .in_zone(Zone::Graveyard)
            .with_mana_value(mana_value);
        let target =
            ChooseSpec::target(ChooseSpec::Object(filter)).with_count(ChoiceCount::up_to(1));

        Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::this_dies(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::return_from_graveyard_to_hand(target.clone()),
                ]),
                choices: vec![target],
                intervening_if: None,
                presentation_label,
            }),
            functional_zones: vec![Zone::Battlefield],
        }
    }

    /// Add recover with a mana cost.
    ///
    /// Recover means "When a creature is put into your graveyard from the
    /// battlefield, you may pay {cost}. If you do, return this card from your
    /// graveyard to your hand. Otherwise, exile this card."
    pub fn recover(self, cost: ManaCost) -> Self {
        let payment_id = EffectId(0);
        let cost_text = cost.to_oracle();
        let trigger = Trigger::new(
            crate::triggers::ZoneChangeTrigger::new()
                .from(Zone::Battlefield)
                .to(Zone::Graveyard)
                .filter(ObjectFilter::creature().owned_by(PlayerFilter::You)),
        );

        self.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger,
                effects: ResolutionProgram::from_effects(vec![Effect::conditional_only(
                    Condition::SourceIsInZone(Zone::Graveyard),
                    vec![
                        Effect::with_id(
                            payment_id.0,
                            Effect::may_single(Effect::new(crate::effects::PayManaEffect::new(
                                cost,
                                ChooseSpec::SourceController,
                            ))),
                        ),
                        Effect::if_then(
                            payment_id,
                            EffectPredicate::Happened,
                            vec![Effect::return_from_graveyard_to_hand(ChooseSpec::Source)],
                        ),
                        Effect::if_then(
                            payment_id,
                            EffectPredicate::DidNotHappen,
                            vec![Effect::exile(ChooseSpec::Source)],
                        ),
                    ],
                )]),
                choices: vec![],
                intervening_if: None,
                presentation_label: Some(ability::PresentationLabel::Keyword(
                    ability::PresentationKeyword::Recover(cost_text),
                )),
            }),
            functional_zones: vec![Zone::Graveyard],
        })
    }

    /// Add outlast with a mana cost.
    ///
    /// Outlast means "{cost}, {T}: Put a +1/+1 counter on this creature.
    /// Activate only as a sorcery."
    pub fn outlast(self, cost: ManaCost) -> Self {
        let total_cost = TotalCost::from_costs(vec![
            crate::costs::Cost::mana(cost),
            crate::costs::Cost::tap(),
        ]);

        self.with_ability(Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost: total_cost,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::plus_one_counters(1, ChooseSpec::Source),
                ]),
                choices: vec![],
                timing: ActivationTiming::SorcerySpeed,
                additional_restrictions: Vec::new(),
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Battlefield],
        })
    }

    /// Add unearth with a mana cost.
    ///
    /// Unearth means "{cost}: Return this card from your graveyard to the battlefield.
    /// It gains haste. Exile it at the beginning of the next end step or if it would
    /// leave the battlefield. Activate only as a sorcery."
    pub fn unearth(self, cost: ManaCost) -> Self {
        let total_cost = TotalCost::from_cost(crate::costs::Cost::mana(cost));

        self.with_ability(Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost: total_cost,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::new(
                    crate::effects::UnearthEffect::new(),
                )]),
                choices: vec![],
                timing: ActivationTiming::SorcerySpeed,
                additional_restrictions: Vec::new(),
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Graveyard],
        })
    }

    /// Add embalm with a mana cost.
    ///
    /// Embalm creates a white Zombie token copy of this card from the graveyard,
    /// without a mana cost, and can be activated only as a sorcery.
    pub fn embalm(self, cost: ManaCost) -> Self {
        let total_cost = TotalCost::from_costs(vec![
            crate::costs::Cost::mana(cost),
            crate::costs::Cost::exile_self(),
        ]);
        let create_embalmed_copy = Effect::new(
            crate::effects::CreateTokenCopyEffect::new(ChooseSpec::Source, 1, PlayerFilter::You)
                .set_colors(ColorSet::WHITE)
                .added_subtype(Subtype::Zombie)
                .without_mana_cost(),
        );

        self.with_ability(Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost: total_cost,
                effects: ResolutionProgram::from_effects(vec![create_embalmed_copy]),
                choices: vec![],
                timing: ActivationTiming::SorcerySpeed,
                additional_restrictions: Vec::new(),
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Graveyard],
        })
    }

    /// Add eternalize with a mana cost.
    ///
    /// Eternalize creates a 4/4 black Zombie token copy of this card from the
    /// graveyard without a mana cost, and can be activated only as a sorcery.
    pub fn eternalize(self, cost: ManaCost) -> Self {
        let total_cost = TotalCost::from_costs(vec![
            crate::costs::Cost::mana(cost),
            crate::costs::Cost::exile_self(),
        ]);
        let create_eternalized_copy = Effect::new(
            crate::effects::CreateTokenCopyEffect::new(ChooseSpec::Source, 1, PlayerFilter::You)
                .set_colors(ColorSet::BLACK)
                .added_subtype(Subtype::Zombie)
                .set_base_power_toughness(4, 4)
                .without_mana_cost(),
        );

        self.with_ability(Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost: total_cost,
                effects: ResolutionProgram::from_effects(vec![create_eternalized_copy]),
                choices: vec![],
                timing: ActivationTiming::SorcerySpeed,
                additional_restrictions: Vec::new(),
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Graveyard],
        })
    }

    /// Add aura swap with a mana cost.
    pub fn aura_swap(self, cost: ManaCost) -> Self {
        let total_cost = TotalCost::from_cost(crate::costs::Cost::mana(cost));

        self.with_ability(Ability::activated(total_cost, vec![Effect::aura_swap()]))
    }

    /// Add emerge with a mana cost.
    ///
    /// Emerge lets this spell be cast by paying its emerge cost and sacrificing
    /// a creature, with the generic portion reduced by that creature's mana value.
    pub fn emerge(self, cost: ManaCost) -> Self {
        self.alternative_cast(AlternativeCastingMethod::alternative_cost(
            "Emerge",
            Some(cost),
            vec![crate::costs::Cost::sacrifice(
                ObjectFilter::creature().you_control(),
            )],
        ))
    }

    /// Add scavenge with a mana cost.
    ///
    /// Scavenge means "{cost}, Exile this card from your graveyard: Put a number
    /// of +1/+1 counters equal to this card's power on target creature. Activate
    /// only as a sorcery."
    pub fn scavenge(self, cost: ManaCost) -> Self {
        let total_cost = TotalCost::from_costs(vec![
            crate::costs::Cost::mana(cost),
            crate::costs::Cost::exile_self(),
        ]);
        let target = ChooseSpec::target(ChooseSpec::creature());

        self.with_ability(Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost: total_cost,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::put_counters(
                        CounterType::PlusOnePlusOne,
                        Value::SourcePower,
                        target.clone(),
                    ),
                ]),
                choices: vec![target],
                timing: ActivationTiming::SorcerySpeed,
                additional_restrictions: Vec::new(),
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Graveyard],
        })
    }

    /// Add ninjutsu with a mana cost.
    ///
    /// Ninjutsu means "{cost}, Return an unblocked attacker you control to hand:
    /// Put this card onto the battlefield from your hand tapped and attacking."
    pub fn ninjutsu(self, cost: ManaCost) -> Self {
        let total_cost = TotalCost::from_costs(vec![
            crate::costs::Cost::mana(cost),
            crate::costs::Cost::effect(crate::effects::NinjutsuCostEffect::new()),
        ]);

        self.with_ability(Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost: total_cost,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::new(
                    crate::effects::NinjutsuEffect::new(),
                )]),
                choices: vec![],
                timing: ActivationTiming::DuringCombat,
                additional_restrictions: Vec::new(),
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Hand],
        })
    }

    /// Add echo with a parsed payment cost.
    ///
    /// Echo means "At the beginning of your upkeep, if this came under your control
    /// since the beginning of your last upkeep, sacrifice it unless you pay its echo cost."
    ///
    /// Runtime model:
    /// - This permanent enters with an internal Echo counter.
    /// - At the beginning of each upkeep, remove one Echo counter from this permanent.
    /// - If a counter was removed this way, pay the echo cost or sacrifice this permanent.
    pub fn echo(self, total_cost: TotalCost) -> Self {
        let payment_effects = crate::costs::total_cost_to_payment_effects(&total_cost);

        self.with_ability(Ability::static_ability(
            StaticAbility::enters_with_counters(CounterType::Echo, 1),
        ))
        .with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::beginning_of_upkeep(PlayerFilter::You),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::conditional_only(
                        Condition::SourceIsInZone(Zone::Battlefield),
                        vec![
                            Effect::with_id(
                                0,
                                Effect::remove_counters(CounterType::Echo, 1, ChooseSpec::Source),
                            ),
                            Effect::if_then(
                                EffectId(0),
                                EffectPredicate::Happened,
                                vec![Effect::unless_action(
                                    vec![Effect::sacrifice_source()],
                                    payment_effects,
                                    PlayerFilter::You,
                                )],
                            ),
                        ],
                    ),
                ]),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![Zone::Battlefield],
        })
    }

    /// Add cumulative upkeep with generic and/or life payment per age counter.
    ///
    /// Runtime model:
    /// - At the beginning of your upkeep, put an age counter on this permanent.
    /// - Then sacrifice it unless you pay the cumulative payment for each age counter.
    pub fn cumulative_upkeep(
        self,
        mana_symbols_per_counter: Vec<ManaSymbol>,
        life_per_counter: u32,
    ) -> Self {
        let age_count = Value::CountersOnSource(CounterType::Age);
        let life = scale_value(age_count, life_per_counter);
        let mana_multiplier = if mana_symbols_per_counter.is_empty() {
            None
        } else {
            Some(Value::CountersOnSource(CounterType::Age))
        };

        self.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::beginning_of_upkeep(PlayerFilter::You),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::put_counters_on_source(CounterType::Age, 1),
                    Effect::unless_pays_with_life_additional_and_multiplier(
                        vec![Effect::sacrifice_source()],
                        PlayerFilter::You,
                        mana_symbols_per_counter,
                        life,
                        None,
                        mana_multiplier,
                    ),
                ]),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![Zone::Battlefield],
        })
    }

    /// Add haunt.
    ///
    /// Creature haunt reminder: "When this creature dies, exile it haunting target creature."
    /// Spell haunt reminder: "When this spell card is put into a graveyard after resolving,
    /// exile it haunting target creature."
    pub fn haunt(self) -> Self {
        let trigger = if self
            .card_builder
            .card_types_ref()
            .contains(&CardType::Creature)
        {
            Trigger::this_dies()
        } else {
            Trigger::new(
                crate::triggers::ZoneChangeTrigger::new()
                    .from(Zone::Stack)
                    .to(Zone::Graveyard)
                    .this(),
            )
        };

        let functional_zones = if self
            .card_builder
            .card_types_ref()
            .contains(&CardType::Creature)
        {
            vec![Zone::Battlefield]
        } else {
            vec![Zone::Graveyard]
        };

        self.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::exile(
                    ChooseSpec::Source,
                )]),
                choices: vec![ChooseSpec::target(ChooseSpec::creature())],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones,
        })
    }

    /// Add provoke.
    ///
    /// Provoke means "Whenever this creature attacks, you may have target creature defending
    /// player controls untap and block it if able."
    pub fn provoke(self) -> Self {
        let target_spec = ChooseSpec::Target(Box::new(ChooseSpec::Object(
            ObjectFilter::creature().controlled_by(PlayerFilter::Defending),
        )));
        let untap = Effect::new(crate::effects::UntapEffect::with_spec(target_spec.clone()));
        let must_block = Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
            target_spec.clone(),
            crate::continuous::Modification::AddAbility(StaticAbility::must_block()),
            Until::EndOfCombat,
        ));
        self.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::this_attacks(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    untap, must_block,
                ]),
                choices: vec![target_spec],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![Zone::Battlefield],
        })
    }

    /// Add casualty N.
    ///
    /// Casualty means "As you cast this spell, you may sacrifice a creature with power N
    /// or greater. When you do, copy this spell and you may choose new targets for the copy."
    pub fn casualty(self, power: u32) -> Self {
        use crate::effect::EffectId;
        use crate::filter::Comparison;
        let mut creature_filter = ObjectFilter::creature().you_control();
        creature_filter.power = Some(Comparison::GreaterThanOrEqual(power as i32));

        self.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::you_cast_this_spell(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::may(
                    vec![
                        Effect::sacrifice(creature_filter, 1),
                        Effect::with_id(0, Effect::copy_spell(ChooseSpec::Source)),
                        Effect::may_choose_new_targets(EffectId(0)),
                    ],
                )]),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![Zone::Stack],
        })
    }

    pub fn variable_casualty_planeswalker_copy(self) -> Self {
        self.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::you_cast_this_spell(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::new(
                    crate::effects::VariableCasualtyPlaneswalkerCopyEffect::new(),
                )]),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![Zone::Stack],
        })
    }

    /// Add demonstrate.
    ///
    /// Demonstrate means "When you cast this spell, you may copy it. If you do,
    /// choose an opponent to also copy it. Players may choose new targets for
    /// their copies."
    pub fn demonstrate(self) -> Self {
        use crate::effect::EffectId;

        let opponent_tag = TagKey::from("demonstrate_opponent");
        let opponent = PlayerFilter::TaggedPlayer(opponent_tag.clone());
        self.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::you_cast_this_spell(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::may(
                    vec![
                        Effect::with_id(0, Effect::copy_spell(ChooseSpec::Source)),
                        Effect::new(crate::effects::ChoosePlayerEffect::new(
                            PlayerFilter::You,
                            PlayerFilter::Opponent,
                            opponent_tag.clone(),
                        )),
                        Effect::with_id(
                            1,
                            Effect::new(crate::effects::CopySpellEffect::new_for_player(
                                ChooseSpec::Source,
                                1,
                                opponent.clone(),
                            )),
                        ),
                        Effect::may_choose_new_targets_player(EffectId(0), PlayerFilter::You),
                        Effect::may_choose_new_targets_player(EffectId(1), opponent),
                    ],
                )]),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![Zone::Stack],
        })
    }

    /// Add conspire.
    ///
    /// Conspire means "As an additional cost to cast this spell, you may tap two untapped
    /// creatures you control that each share a color with it" and "When you cast this spell,
    /// if its conspire cost was paid, copy it. If the spell has any targets, you may choose
    /// new targets for the copy."
    ///
    /// Multiple granted or printed instances use distinct internal labels so each paid
    /// instance can trigger independently per CR 702.78b.
    pub fn conspire(mut self) -> Self {
        use crate::effect::EffectId;

        let existing_instances = self
            .optional_costs
            .iter()
            .filter(|cost| matches!(cost.kind, OptionalCostKind::Conspire))
            .count();
        let label = if existing_instances == 0 {
            "Conspire".to_string()
        } else {
            format!("Conspire {}", existing_instances + 1)
        };
        let cost = TotalCost::from_cost(crate::costs::Cost::effect(
            crate::effects::ConspireCostEffect::new(),
        ));
        self.optional_costs
            .push(OptionalCost::custom(label.clone(), cost));
        self.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::you_cast_this_spell(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::with_id(0, Effect::copy_spell(ChooseSpec::Source)),
                    Effect::may_choose_new_targets(EffectId(0)),
                ]),
                choices: vec![],
                intervening_if: Some(Condition::ThisSpellPaidLabel(label.into())),
                presentation_label: None,
            }),
            functional_zones: vec![Zone::Stack],
        })
    }

    /// Add devour N.
    ///
    /// Devour means "As this creature enters, you may sacrifice any number of creatures.
    /// This creature enters with N times that many +1/+1 counters on it."
    pub fn devour(self, multiplier: u32) -> Self {
        self.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::this_enters_battlefield(),
                effects: vec![Effect::devour(multiplier)].into(),
                choices: vec![],
                intervening_if: None,
                presentation_label: Some(ability::PresentationLabel::Keyword(
                    ability::PresentationKeyword::Devour(multiplier),
                )),
            }),
            functional_zones: vec![Zone::Battlefield],
        })
    }

    /// Add amplify N.
    ///
    /// Amplify means "As this creature enters, reveal any number of cards from your hand
    /// that share a creature type with it. This creature enters with N times that many
    /// +1/+1 counters on it."
    pub fn amplify(self, amount: u32) -> Self {
        self.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::this_enters_battlefield(),
                effects: vec![Effect::amplify(amount)].into(),
                choices: vec![],
                intervening_if: None,
                presentation_label: Some(ability::PresentationLabel::Keyword(
                    ability::PresentationKeyword::Amplify(amount),
                )),
            }),
            functional_zones: vec![Zone::Battlefield],
        })
    }

    /// Add ravenous.
    ///
    /// Ravenous means "This creature enters with X +1/+1 counters on it. When it enters,
    /// if X is 5 or more, draw a card."
    pub fn ravenous(self) -> Self {
        use crate::effect::Value;
        use crate::object::CounterType;

        self.with_ability(Ability::static_ability(
            StaticAbility::enters_with_counters_value(CounterType::PlusOnePlusOne, Value::X),
        ))
        .with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::this_enters_battlefield(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::draw(1)]),
                choices: vec![],
                intervening_if: Some(Condition::XValueAtLeast(5)),
                presentation_label: None,
            }),
            functional_zones: vec![Zone::Battlefield],
        })
    }

    /// Add ascend.
    ///
    /// Ascend means "If you control ten or more permanents, you get the city's blessing
    /// for the rest of the game."
    pub fn ascend(self) -> Self {
        let is_nonpermanent_spell = self
            .card_builder
            .card_types_ref()
            .iter()
            .any(|card_type| matches!(card_type, CardType::Instant | CardType::Sorcery));
        if is_nonpermanent_spell {
            let mut out = self;
            let mut effects = out.spell_effect.take().unwrap_or_default();
            effects.insert(0, Effect::new(crate::effects::AscendEffect::new()));
            out.spell_effect = Some(effects);
            return out;
        }

        self.with_ability(Ability::static_ability(StaticAbility::ascend()))
    }

    /// Add daybound.
    pub fn daybound(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::daybound()))
    }

    /// Add nightbound.
    pub fn nightbound(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::nightbound()))
    }

    /// Add enlist.
    ///
    /// Enlist means "As this creature attacks, you may tap a nonattacking creature you
    /// control without summoning sickness. When you do, add its power to this creature's
    /// until end of turn."
    pub fn enlist(self) -> Self {
        let linked_trigger = crate::ability::TriggeredAbility {
            trigger: Trigger::this_attacks(),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![
                Effect::pump_for_each(
                    ChooseSpec::Source,
                    1,
                    0,
                    Value::PowerOf(Box::new(ChooseSpec::Tagged("enlisted_creature".into()))),
                    Until::EndOfTurn,
                ),
            ]),
            choices: Vec::new(),
            intervening_if: None,
            presentation_label: None,
        };
        self.with_ability(Ability::static_ability(StaticAbility::enlist_attack(
            linked_trigger,
            "Enlist",
        )))
    }

    /// Add undaunted.
    ///
    /// Undaunted means "This spell costs {1} less to cast for each opponent."
    pub fn undaunted(self) -> Self {
        let reduction = crate::static_abilities::CostReduction::new(
            ObjectFilter::default(),
            Value::CountPlayers(PlayerFilter::Opponent),
        );
        self.with_ability(
            Ability::static_ability(StaticAbility::new(reduction))
                .in_zones(vec![Zone::Stack, Zone::Hand]),
        )
    }

    /// Add extort.
    ///
    /// Extort means "Whenever you cast a spell, you may pay {W/B}.
    /// If you do, each opponent loses 1 life and you gain that much life."
    pub fn extort(self) -> Self {
        let pay_cost = ManaCost::from_pips(vec![vec![ManaSymbol::White, ManaSymbol::Black]]);
        self.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::spell_cast(None, PlayerFilter::You),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::with_id(
                        0,
                        Effect::may_single(Effect::new(crate::effects::PayManaEffect::new(
                            pay_cost,
                            ChooseSpec::SourceController,
                        ))),
                    ),
                    Effect::if_then(
                        EffectId(0),
                        EffectPredicate::Happened,
                        vec![
                            Effect::with_id(
                                1,
                                Effect::for_each_opponent(vec![Effect::lose_life_player(
                                    1,
                                    PlayerFilter::IteratedPlayer,
                                )]),
                            ),
                            Effect::gain_life(Value::EffectValue(EffectId(1))),
                        ],
                    ),
                ]),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![Zone::Battlefield],
        })
    }

    /// Add riot.
    ///
    /// Riot means "This creature enters with your choice of a +1/+1 counter or haste."
    pub fn riot(self) -> Self {
        self.with_ability(riot_triggered_ability())
    }

    /// Add unleash.
    ///
    /// Unleash means "You may have this creature enter with a +1/+1 counter on it.
    /// It can't block as long as it has a +1/+1 counter on it."
    pub fn unleash(self) -> Self {
        self.with_ability(Ability::triggered(
            Trigger::this_enters_battlefield(),
            vec![Effect::may_single(Effect::plus_one_counters(
                1,
                ChooseSpec::Source,
            ))],
        ))
        .with_ability(Ability::static_ability(StaticAbility::unleash()))
    }

    /// Add partner.
    ///
    /// Partner is a deck-construction ability used in Commander variants.
    /// It has no battlefield rules impact in this runtime.
    pub fn partner(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::partner()))
    }

    /// Add assist.
    ///
    /// Assist is relevant in multiplayer casting. In 1v1 it has no gameplay impact.
    pub fn assist(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::assist()))
    }

    /// Add split second.
    ///
    /// Split second means "As long as this spell is on the stack, players can't cast spells
    /// or activate abilities that aren't mana abilities."
    pub fn split_second(self) -> Self {
        self.with_ability(
            Ability::static_ability(StaticAbility::split_second()).in_zones(vec![Zone::Stack]),
        )
    }

    /// Add cascade.
    ///
    /// Cascade means "When you cast this spell, exile cards from the top of your library
    /// until you exile a nonland card with lesser mana value. You may cast it without
    /// paying its mana cost. Put the exiled cards not cast this way on the bottom in a
    /// random order."
    pub fn cascade(self) -> Self {
        self.with_ability(
            Ability::static_ability(StaticAbility::cascade()).in_zones(vec![Zone::Stack]),
        )
    }

    /// Add rebound.
    ///
    /// Rebound means "If this spell was cast from your hand, exile it as it resolves.
    /// At the beginning of your next upkeep, you may cast it from exile without paying
    /// its mana cost."
    pub fn rebound(self) -> Self {
        self.with_ability(
            Ability::static_ability(StaticAbility::rebound()).in_zones(vec![Zone::Stack]),
        )
    }

    /// Add read ahead.
    pub fn read_ahead(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::read_ahead()))
    }

    /// Add sunburst.
    ///
    /// Sunburst means "This permanent enters with a +1/+1 counter on it for each color
    /// of mana spent to cast it if it's a creature. Otherwise, it enters with that many
    /// charge counters on it."
    pub fn sunburst(self) -> Self {
        let counter_type = if self
            .card_builder
            .card_types_ref()
            .contains(&CardType::Creature)
        {
            CounterType::PlusOnePlusOne
        } else {
            CounterType::Charge
        };

        self.with_ability(Ability::static_ability(StaticAbility::keyword_marker(
            "Sunburst",
        )))
        .with_ability(Ability::static_ability(
            StaticAbility::enters_with_counters_value(
                counter_type,
                Value::ColorsOfManaSpentToCastThisSpell,
            ),
        ))
    }

    /// Add fading N.
    ///
    /// Fading means "This permanent enters with N fade counters on it.
    /// At the beginning of your upkeep, remove a fade counter from it.
    /// If you can't, sacrifice it."
    pub fn fading(self, amount: u32) -> Self {
        self.with_ability(Ability::static_ability(
            StaticAbility::enters_with_counters(CounterType::Fade, amount),
        ))
        .with_ability(Ability::triggered(
            Trigger::beginning_of_upkeep(PlayerFilter::You),
            vec![Effect::remove_counters(
                CounterType::Fade,
                1,
                ChooseSpec::Source,
            )],
        ))
        .with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::counter_removed_from(ObjectFilter::source()),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::sacrifice_source(),
                ]),
                choices: vec![],
                intervening_if: Some(Condition::SourceHasNoCounter(CounterType::Fade)),
                presentation_label: None,
            }),
            functional_zones: vec![Zone::Battlefield],
        })
    }

    /// Add vanishing N.
    ///
    /// Vanishing means "This permanent enters with N time counters on it.
    /// At the beginning of your upkeep, remove a time counter from it.
    /// When the last is removed, sacrifice it."
    pub fn vanishing(self, amount: u32) -> Self {
        let mut builder = self;
        if amount > 0 {
            builder = builder.with_ability(Ability::static_ability(
                StaticAbility::enters_with_counters(CounterType::Time, amount),
            ));
        }
        builder
            .with_ability(Ability::triggered(
                Trigger::beginning_of_upkeep(PlayerFilter::You),
                vec![Effect::remove_counters(
                    CounterType::Time,
                    1,
                    ChooseSpec::Source,
                )],
            ))
            .with_ability(Ability {
                kind: AbilityKind::Triggered(TriggeredAbility {
                    trigger: Trigger::counter_removed_from(ObjectFilter::source()),
                    effects: crate::resolution::ResolutionProgram::from_effects(vec![
                        Effect::sacrifice_source(),
                    ]),
                    choices: vec![],
                    intervening_if: Some(Condition::SourceHasNoCounter(CounterType::Time)),
                    presentation_label: None,
                }),
                functional_zones: vec![Zone::Battlefield],
            })
    }

    /// Add backup N as a placeholder printed ability. This is finalized into
    /// the real ETB trigger after the full card definition has been built, so
    /// it can grant the abilities printed below it.
    pub fn backup(self, amount: u32) -> Self {
        let text = format!("Backup {amount}");
        self.with_ability(Ability::static_ability(StaticAbility::keyword_marker(
            text.clone(),
        )))
    }

    /// Add cipher as a placeholder printed ability.
    ///
    /// This is finalized into a resolution add-on after the full definition has
    /// been built, so generated definitions do not rely on a marker static ability.
    pub fn cipher(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::keyword_marker(
            "Cipher",
        )))
    }

    /// Add modular N.
    ///
    /// Modular means "This creature enters with N +1/+1 counters on it. When it dies,
    /// you may put its +1/+1 counters on target artifact creature."
    pub fn modular(self, amount: u32) -> Self {
        let target = ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::default()
                .with_all_type(CardType::Artifact)
                .with_all_type(CardType::Creature),
        ));
        let trigger_tag = "modular_triggering_object";
        let dead_source_filter = ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .same_stable_id_as_tagged(trigger_tag);
        let transfer_count = Value::CountersOn(
            Box::new(ChooseSpec::All(dead_source_filter)),
            Some(CounterType::PlusOnePlusOne),
        );

        self.with_ability(Ability::static_ability(
            StaticAbility::enters_with_counters(CounterType::PlusOnePlusOne, amount),
        ))
        .with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::this_dies(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::tag_triggering_object(trigger_tag),
                    Effect::may_single(Effect::put_counters(
                        CounterType::PlusOnePlusOne,
                        transfer_count,
                        target.clone(),
                    )),
                ]),
                choices: vec![target],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![Zone::Battlefield],
        })
    }

    /// Add modular whose initial counters are determined by sunburst.
    ///
    /// This appears on cards such as Arcbound Wanderer and means:
    /// "This creature enters with a +1/+1 counter on it for each color of mana
    /// spent to cast it. When it dies, you may put its +1/+1 counters on target
    /// artifact creature."
    pub fn modular_sunburst(self) -> Self {
        let target = ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::default()
                .with_all_type(CardType::Artifact)
                .with_all_type(CardType::Creature),
        ));
        let trigger_tag = "modular_triggering_object";
        let dead_source_filter = ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .same_stable_id_as_tagged(trigger_tag);
        let transfer_count = Value::CountersOn(
            Box::new(ChooseSpec::All(dead_source_filter)),
            Some(CounterType::PlusOnePlusOne),
        );

        self.with_ability(Ability::static_ability(
            StaticAbility::enters_with_counters_value(
                CounterType::PlusOnePlusOne,
                Value::ColorsOfManaSpentToCastThisSpell,
            ),
        ))
        .with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::this_dies(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::tag_triggering_object(trigger_tag),
                    Effect::may_single(Effect::put_counters(
                        CounterType::PlusOnePlusOne,
                        transfer_count,
                        target.clone(),
                    )),
                ]),
                choices: vec![target],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![Zone::Battlefield],
        })
    }

    /// Add graft N.
    ///
    /// Graft means "This creature enters with N +1/+1 counters on it. Whenever another
    /// creature enters, you may move a +1/+1 counter from this creature onto it."
    pub fn graft(self, amount: u32) -> Self {
        let entered_tag = "graft_entered_creature";

        self.with_ability(Ability::static_ability(
            StaticAbility::enters_with_counters(CounterType::PlusOnePlusOne, amount),
        ))
        .with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::enters_battlefield(ObjectFilter::creature().other(), None),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::tag_triggering_object(entered_tag),
                    Effect::may_single(Effect::move_counters(
                        CounterType::PlusOnePlusOne,
                        1,
                        ChooseSpec::Source,
                        ChooseSpec::Tagged(entered_tag.into()),
                    )),
                ]),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![Zone::Battlefield],
        })
    }

    /// Add ingest.
    ///
    /// Ingest means "Whenever this creature deals combat damage to a player,
    /// that player exiles the top card of their library."
    pub fn ingest(self) -> Self {
        self.with_ability(Ability::triggered(
            Trigger::this_deals_combat_damage_to_player(PlayerFilter::Any),
            vec![Effect::exile_top_of_library_player(
                1,
                PlayerFilter::DamagedPlayer,
            )],
        ))
    }

    /// Add storm.
    ///
    /// Storm means "When you cast this spell, copy it for each spell cast before it this turn."
    pub fn storm(self) -> Self {
        self.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::you_cast_this_spell(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::with_id(
                        0,
                        Effect::copy_spell_n(
                            ChooseSpec::Source,
                            Value::SpellsCastBeforeThisTurn(PlayerFilter::You),
                        ),
                    ),
                    Effect::may_choose_new_targets(EffectId(0)),
                ]),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![Zone::Stack],
        })
    }

    /// Add gravestorm.
    ///
    /// Gravestorm copies this spell for each permanent put into a graveyard
    /// from the battlefield this turn.
    pub fn gravestorm(self) -> Self {
        self.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::you_cast_this_spell(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::with_id(
                        0,
                        Effect::copy_spell_n(
                            ChooseSpec::Source,
                            Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::Died(
                                ObjectFilter::default(),
                            )),
                        ),
                    ),
                    Effect::may_choose_new_targets(EffectId(0)),
                ]),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![Zone::Stack],
        })
    }

    /// Add fear.
    pub fn fear(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::fear()))
    }

    /// Add intimidate.
    pub fn intimidate(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::intimidate()))
    }

    /// Add skulk.
    ///
    /// Skulk means "This creature can't be blocked by creatures with greater power."
    pub fn skulk(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::skulk()))
    }

    /// Add afflict N.
    ///
    /// Afflict means "Whenever this creature becomes blocked, defending player loses N life."
    pub fn afflict(self, amount: u32) -> Self {
        self.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::this_becomes_blocked(),
                effects: vec![Effect::lose_life_player(
                    amount as i32,
                    PlayerFilter::Defending,
                )]
                .into(),
                choices: vec![],
                intervening_if: None,
                presentation_label: Some(ability::PresentationLabel::Keyword(
                    ability::PresentationKeyword::Afflict(amount),
                )),
            }),
            functional_zones: vec![Zone::Battlefield],
        })
    }

    /// Add afterlife N.
    ///
    /// Afterlife means "When this creature dies, create N 1/1 white and black Spirit creature
    /// tokens with flying."
    pub fn afterlife(self, amount: u32) -> Self {
        self.with_ability(Ability::triggered(
            Trigger::this_dies(),
            vec![Effect::create_tokens(
                Self::afterlife_spirit_token(),
                amount,
            )],
        ))
    }

    /// Add fabricate N.
    ///
    /// Fabricate means "When this creature enters, choose one —
    /// • Put N +1/+1 counters on it.
    /// • Create N 1/1 colorless Servo artifact creature tokens."
    pub fn fabricate(self, amount: u32) -> Self {
        let put_description = if amount == 1 {
            "Put a +1/+1 counter on this creature".to_string()
        } else {
            format!("Put {amount} +1/+1 counters on this creature")
        };
        let create_description = if amount == 1 {
            "Create a 1/1 colorless Servo artifact creature token".to_string()
        } else {
            format!("Create {amount} 1/1 colorless Servo artifact creature tokens")
        };
        let modes = vec![
            EffectMode {
                source_text: put_description,
                effects: vec![Effect::plus_one_counters(amount as i32, ChooseSpec::Source)],
            },
            EffectMode {
                source_text: create_description,
                effects: vec![Effect::create_tokens(Self::fabricate_servo_token(), amount)],
            },
        ];

        self.with_ability(Ability::triggered(
            Trigger::this_enters_battlefield(),
            vec![Effect::choose_one(modes)],
        ))
    }

    /// Add "For Mirrodin!"
    ///
    /// "When this Equipment enters, create a 2/2 red Rebel creature token, then attach this to it."
    pub fn for_mirrodin(self) -> Self {
        let created_tag = TagKey::from("for_mirrodin_created");
        self.with_ability(Ability::triggered(
            Trigger::this_enters_battlefield(),
            vec![
                Effect::create_tokens(Self::for_mirrodin_rebel_token(), 1).tag(created_tag.clone()),
                Effect::attach_to(ChooseSpec::Tagged(created_tag)),
            ],
        ))
    }

    /// Add living weapon.
    ///
    /// "When this Equipment enters, create a 0/0 black Phyrexian Germ creature token, then attach this to it."
    pub fn living_weapon(self) -> Self {
        let created_tag = TagKey::from("living_weapon_created");
        self.with_ability(Ability::triggered(
            Trigger::this_enters_battlefield(),
            vec![
                Effect::create_tokens(Self::living_weapon_germ_token(), 1).tag(created_tag.clone()),
                Effect::attach_to(ChooseSpec::Tagged(created_tag)),
            ],
        ))
    }

    /// Add myriad.
    ///
    /// "Whenever this creature attacks, for each opponent other than defending player,
    /// you may create a token that's a copy of this creature that's tapped and attacking
    /// that player or a planeswalker they control. Exile the tokens at end of combat."
    pub fn myriad(self) -> Self {
        let opponent_other_than_defending =
            PlayerFilter::excluding(PlayerFilter::Opponent, PlayerFilter::Defending);
        self.with_ability(Ability::triggered(
            Trigger::this_attacks(),
            vec![Effect::for_players(
                opponent_other_than_defending,
                vec![Effect::may(vec![Effect::new(
                    crate::effects::CreateTokenCopyEffect::new(
                        ChooseSpec::Source,
                        1,
                        PlayerFilter::You,
                    )
                    .enters_tapped(true)
                    .attacking_player_or_planeswalker_controlled_by(PlayerFilter::IteratedPlayer)
                    .exile_at_eoc(true),
                )])],
            )],
        ))
    }

    /// Add mobilize N.
    ///
    /// Mobilize means "Whenever this creature attacks, create N tapped and
    /// attacking 1/1 red Warrior creature tokens. Sacrifice them at the
    /// beginning of the next end step."
    pub fn mobilize(self, amount: u32) -> Self {
        let effect = crate::effects::CreateTokenEffect::new(
            Self::mobilize_warrior_token(),
            amount,
            PlayerFilter::You,
        )
        .tapped()
        .attacking()
        .sacrifice_at_next_end_step();

        self.with_ability(Ability::triggered(
            Trigger::this_attacks(),
            vec![Effect::new(effect)],
        ))
    }

    /// Add shadow.
    pub fn shadow(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::shadow()))
    }

    /// Add horsemanship.
    pub fn horsemanship(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::horsemanship()))
    }

    /// Add bushido N.
    ///
    /// Bushido means "Whenever this creature blocks or becomes blocked, it gets +N/+N until
    /// end of turn."
    pub fn bushido(self, amount: u32) -> Self {
        use crate::effect::Until;
        self.with_ability(Ability::triggered(
            Trigger::this_blocks_or_becomes_blocked(),
            vec![Effect::pump(
                amount,
                amount,
                ChooseSpec::Source,
                Until::EndOfTurn,
            )],
        ))
    }

    /// Add frenzy N.
    ///
    /// Frenzy means "Whenever this creature attacks and isn't blocked, it gets +N/+0 until
    /// end of turn."
    pub fn frenzy(self, amount: u32) -> Self {
        self.with_ability(Ability::triggered(
            Trigger::this_attacks_and_isnt_blocked(),
            vec![Effect::pump(
                amount,
                0,
                ChooseSpec::Source,
                Until::EndOfTurn,
            )],
        ))
    }

    /// Add bloodthirst N.
    ///
    /// Bloodthirst means "If an opponent was dealt damage this turn, this creature enters
    /// the battlefield with N +1/+1 counters on it."
    pub fn bloodthirst(self, amount: u32) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::bloodthirst(amount)))
    }

    /// Add tribute N.
    ///
    /// Tribute means "As this creature enters, an opponent may put N +1/+1 counters on it."
    pub fn tribute(self, amount: u32) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::tribute(amount)))
    }

    /// Add rampage N.
    ///
    /// Rampage means "Whenever this creature becomes blocked, it gets +N/+N until end of turn
    /// for each creature blocking it beyond the first."
    pub fn rampage(self, amount: u32) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::keyword_marker(
            format!("rampage {amount}"),
        )))
        .with_ability(Ability::triggered(
            Trigger::this_becomes_blocked(),
            vec![Effect::pump(
                Value::EventValue(EventValueSpec::BlockersBeyondFirst {
                    multiplier: amount as i32,
                }),
                Value::EventValue(EventValueSpec::BlockersBeyondFirst {
                    multiplier: amount as i32,
                }),
                ChooseSpec::Source,
                Until::EndOfTurn,
            )],
        ))
    }

    /// Add unblockable (can't be blocked).
    pub fn unblockable(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::unblockable()))
    }

    /// Add "may assign combat damage as though unblocked" (Thorn Elemental ability).
    pub fn may_assign_damage_as_unblocked(self) -> Self {
        self.with_ability(Ability::static_ability(
            StaticAbility::may_assign_damage_as_unblocked(),
        ))
    }

    /// Add "shuffle into library from graveyard" (Darksteel Colossus ability).
    pub fn shuffle_into_library_from_graveyard(self) -> Self {
        use crate::zone::Zone;
        self.with_ability(
            Ability::static_ability(StaticAbility::shuffle_into_library_from_graveyard()).in_zones(
                vec![
                    Zone::Battlefield,
                    Zone::Stack,
                    Zone::Hand,
                    Zone::Library,
                    Zone::Graveyard,
                    Zone::Exile,
                    Zone::Command,
                ],
            ),
        )
    }

    // === Cost Modifier Abilities ===

    /// Add affinity for artifacts (cost reduction based on artifacts you control).
    pub fn affinity_for_artifacts(self) -> Self {
        self.with_ability(Ability::static_ability(
            StaticAbility::affinity_for_artifacts(),
        ))
    }

    /// Add delve (exile cards from graveyard to pay generic mana).
    pub fn delve(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::delve()))
    }

    /// Add convoke (tap creatures to help pay for this spell).
    pub fn convoke(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::convoke()))
    }

    /// Add improvise (tap artifacts to pay generic mana).
    pub fn improvise(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::improvise()))
    }

    /// Add protection from a color.
    pub fn protection_from(self, colors: ColorSet) -> Self {
        use crate::ability::ProtectionFrom;
        let protection = StaticAbility::protection(ProtectionFrom::Color(colors));
        self.with_ability(Ability::static_ability(protection))
    }

    /// Add protection from a card type.
    pub fn protection_from_card_type(self, card_type: CardType) -> Self {
        use crate::ability::ProtectionFrom;
        let protection = StaticAbility::protection(ProtectionFrom::CardType(card_type));
        self.with_ability(Ability::static_ability(protection))
    }

    /// Add protection from objects matching a filter.
    pub fn protection_from_filter(self, filter: ObjectFilter) -> Self {
        use crate::ability::ProtectionFrom;
        let protection = StaticAbility::protection(ProtectionFrom::Permanents(filter));
        self.with_ability(Ability::static_ability(protection))
    }

    /// Add protection from a creature subtype (e.g., "Protection from Humans").
    pub fn protection_from_subtype(self, subtype: Subtype) -> Self {
        use crate::ability::ProtectionFrom;
        let protection = StaticAbility::protection(ProtectionFrom::Permanents(
            ObjectFilter::default().with_subtype(subtype),
        ));
        self.with_ability(Ability::static_ability(protection))
    }

    // === Triggered ability shortcuts ===

    /// Add an enters-the-battlefield trigger.
    pub fn with_etb(self, effects: Vec<Effect>) -> Self {
        self.with_ability(ability::etb_trigger(effects))
    }

    /// Add a dies trigger.
    pub fn with_dies_trigger(self, effects: Vec<Effect>) -> Self {
        self.with_ability(ability::dies_trigger(effects))
    }

    /// Add an upkeep trigger.
    pub fn with_upkeep_trigger(self, effects: Vec<Effect>) -> Self {
        self.with_ability(ability::upkeep_trigger(effects))
    }

    /// Add a custom triggered ability.
    pub fn with_trigger(self, trigger: crate::triggers::Trigger, effects: Vec<Effect>) -> Self {
        self.with_ability(Ability::triggered(trigger, effects))
    }

    /// Add a targeted ETB trigger (e.g., Snapcaster Mage).
    pub fn with_targeted_etb(
        self,
        target_spec: crate::target::ChooseSpec,
        effects: Vec<Effect>,
    ) -> Self {
        use crate::ability::{AbilityKind, TriggeredAbility};
        use crate::triggers::Trigger;
        use crate::zone::Zone;

        let ability = Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::this_enters_battlefield(),
                effects: effects.into(),
                choices: vec![target_spec],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![Zone::Battlefield],
        };
        self.with_ability(ability)
    }

    /// Add an optional triggered ability ("you may").
    pub fn with_optional_trigger(
        self,
        trigger: crate::triggers::Trigger,
        effects: Vec<Effect>,
    ) -> Self {
        use crate::ability::{AbilityKind, TriggeredAbility};
        use crate::zone::Zone;

        let ability = Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger,
                effects: effects.into(),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![Zone::Battlefield],
        };
        self.with_ability(ability)
    }

    // === Activated ability shortcuts ===

    /// Add an activated ability.
    pub fn with_activated(self, cost: TotalCost, effects: Vec<Effect>) -> Self {
        self.with_ability(Ability::activated(cost, effects))
    }

    /// Add a tap ability that does something.
    pub fn with_tap_ability(self, effects: Vec<Effect>) -> Self {
        self.with_ability(Ability::activated(
            TotalCost::from_cost(crate::costs::Cost::tap()),
            effects,
        ))
    }

    // === Mana ability shortcuts ===

    /// Add a mana ability that taps for a single color.
    pub fn taps_for(self, mana: ManaSymbol) -> Self {
        self.with_ability(Ability::mana(TotalCost::free(), vec![mana]))
    }

    /// Add a mana ability that taps for multiple mana.
    pub fn taps_for_mana(self, mana: Vec<ManaSymbol>) -> Self {
        self.with_ability(Ability::mana(TotalCost::free(), mana))
    }

    // === Spell effect shortcuts ===

    /// Set the spell effects (for instants/sorceries).
    pub fn with_spell_effect(mut self, effects: Vec<Effect>) -> Self {
        self.spell_effect = Some(ResolutionProgram::from_effects(effects));
        self
    }

    // === Alternative Casting Methods ===

    /// Add flashback with the given cost.
    pub fn flashback(mut self, cost: ManaCost) -> Self {
        self.alternative_casts
            .push(AlternativeCastingMethod::Flashback {
                total_cost: TotalCost::mana(cost),
            });
        self
    }

    /// Add jump-start (cast from graveyard, discard a card).
    pub fn jump_start(mut self) -> Self {
        self.alternative_casts
            .push(AlternativeCastingMethod::JumpStart {
                additional_cost: TotalCost::from_cost(crate::costs::Cost::discard(1, None)),
            });
        self
    }

    /// Add escape with the given cost and exile count.
    pub fn escape(mut self, cost: ManaCost, exile_count: u32) -> Self {
        self.alternative_casts
            .push(AlternativeCastingMethod::Escape {
                cost: Some(cost),
                exile_count,
                additional_cost: TotalCost::from_cost(crate::costs::Cost::exile_from_graveyard(
                    exile_count,
                    None,
                )),
            });
        self
    }

    /// Add madness with the given cost.
    pub fn madness(mut self, cost: ManaCost) -> Self {
        self.alternative_casts
            .push(AlternativeCastingMethod::Madness { cost });
        self
    }

    /// Add dash with the given cost.
    pub fn dash(mut self, cost: ManaCost) -> Self {
        self.alternative_casts
            .push(AlternativeCastingMethod::Dash { cost });
        self
    }

    /// Add blitz with the given cost.
    pub fn blitz(mut self, cost: ManaCost) -> Self {
        self.alternative_casts
            .push(AlternativeCastingMethod::Blitz {
                total_cost: TotalCost::mana(cost),
            });
        self
    }

    /// Add warp with the given cost.
    pub fn warp(mut self, cost: ManaCost) -> Self {
        self.alternative_casts
            .push(AlternativeCastingMethod::Warp { cost });
        self
    }

    /// Add plot with the given cost.
    pub fn plot(mut self, cost: ManaCost) -> Self {
        self.alternative_casts
            .push(AlternativeCastingMethod::Plot { cost });
        self
    }

    /// Add suspend with the given time count and cost.
    pub fn suspend(self, time: u32, cost: ManaCost) -> Self {
        self.alternative_cast(AlternativeCastingMethod::Suspend { cost, time })
            .with_ability(Ability {
                kind: AbilityKind::Triggered(TriggeredAbility {
                    trigger: Trigger::beginning_of_upkeep(PlayerFilter::You),
                    effects: crate::resolution::ResolutionProgram::from_effects(vec![
                        Effect::remove_counters(CounterType::Time, 1, ChooseSpec::Source),
                    ]),
                    choices: vec![],
                    intervening_if: Some(Condition::SourceHasCounterAtLeast {
                        counter_type: CounterType::Time,
                        count: 1,
                        surface: crate::effect::SourceCounterThresholdSurface::SourceHas,
                    }),
                    presentation_label: None,
                }),
                functional_zones: vec![Zone::Exile],
            })
            .with_ability(Ability {
                kind: AbilityKind::Triggered(TriggeredAbility {
                    trigger: Trigger::counter_removed_from(ObjectFilter::source()),
                    effects: crate::resolution::ResolutionProgram::from_effects(vec![
                        Effect::may_single(Effect::new(
                            crate::effects::CastSourceEffect::new()
                                .without_paying_mana_cost()
                                .require_exile()
                                .cast_as_suspend(),
                        )),
                    ]),
                    choices: vec![],
                    intervening_if: Some(Condition::SourceHasNoCounter(CounterType::Time)),
                    presentation_label: None,
                }),
                functional_zones: vec![Zone::Exile],
            })
    }

    /// Add disturb with the given cost.
    pub fn disturb(mut self, cost: ManaCost) -> Self {
        self.alternative_casts
            .push(AlternativeCastingMethod::Disturb { cost });
        self
    }

    /// Add overload with the given cost.
    pub fn overload(mut self, cost: ManaCost) -> Self {
        self.alternative_casts
            .push(AlternativeCastingMethod::Overload {
                cost,
                effects: Vec::new(),
            });
        self
    }

    /// Add Cleave with the given alternative cost. The compiler fills the
    /// bracket-removed effect program after document lowering.
    pub fn cleave(mut self, cost: ManaCost) -> Self {
        self.alternative_casts
            .push(AlternativeCastingMethod::Cleave {
                cost,
                effects: Vec::new(),
            });
        self
    }

    /// Add awaken with the given counter count and alternative cost.
    pub fn awaken(mut self, amount: u32, cost: ManaCost) -> Self {
        let mut effects = self
            .spell_effect
            .as_ref()
            .map(|program| program.all_effects_owned())
            .unwrap_or_default();
        let spec = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::land().you_control()));
        effects.push(Effect::new(crate::effects::EarthbendEffect::new(
            spec, amount,
        )));
        self.alternative_casts
            .push(AlternativeCastingMethod::Awaken {
                amount,
                cost,
                effects,
            });
        self
    }

    /// Add miracle with the given cost.
    ///
    /// Miracle is both an alternative casting method and a triggered ability:
    /// "When you draw this card, if it's the first card you've drawn this turn,
    /// you may reveal it. If you do, you may cast it for its miracle cost."
    pub fn miracle(mut self, cost: ManaCost) -> Self {
        use crate::effect::Effect;
        use crate::triggers::Trigger;

        // Add the alternative casting method
        self.alternative_casts
            .push(AlternativeCastingMethod::Miracle { cost });

        // Add the miracle trigger
        self.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::miracle(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::may_cast_for_miracle_cost(),
                ]),
                choices: vec![],
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![crate::zone::Zone::Hand], // Only triggers from hand
        })
    }

    /// Add foretell with the given cost.
    pub fn foretell(mut self, cost: ManaCost) -> Self {
        self.alternative_casts
            .push(AlternativeCastingMethod::Foretell { cost });
        self
    }

    /// Add spectacle with the given cost.
    pub fn spectacle(mut self, cost: ManaCost) -> Self {
        self.alternative_casts
            .push(AlternativeCastingMethod::alternative_cost_with_condition(
                "Spectacle",
                Some(cost),
                Vec::new(),
                crate::static_abilities::ThisSpellCostCondition::ConditionExpr {
                    condition: crate::ConditionExpr::OpponentLostLifeThisTurn,
                    display: "an opponent lost life this turn".to_string(),
                },
            ));
        self
    }

    /// Add a custom alternative casting method.
    pub fn alternative_cast(mut self, method: AlternativeCastingMethod) -> Self {
        self.alternative_casts.push(method);
        self
    }

    // === Optional Costs (Kicker, Buyback, etc.) ===

    /// Add a kicker cost (can pay once for additional effect).
    pub fn kicker(mut self, cost: TotalCost) -> Self {
        self.optional_costs.push(OptionalCost::kicker(cost));
        self
    }

    /// Add a kicker cost using just mana.
    pub fn kicker_mana(self, cost: ManaCost) -> Self {
        self.kicker(TotalCost::mana(cost))
    }

    /// Add a multikicker cost (can pay any number of times).
    pub fn multikicker(mut self, cost: TotalCost) -> Self {
        self.optional_costs.push(OptionalCost::multikicker(cost));
        self
    }

    /// Add a multikicker cost using just mana.
    pub fn multikicker_mana(self, cost: ManaCost) -> Self {
        self.multikicker(TotalCost::mana(cost))
    }

    /// Add a buyback cost (return spell to hand after resolution).
    pub fn buyback(mut self, cost: TotalCost) -> Self {
        self.optional_costs.push(OptionalCost::buyback(cost));
        self
    }

    /// Add a buyback cost using just mana.
    pub fn buyback_mana(self, cost: ManaCost) -> Self {
        self.buyback(TotalCost::mana(cost))
    }

    /// Add an entwine cost (for modal spells, choose all modes).
    pub fn entwine(mut self, cost: TotalCost) -> Self {
        self.optional_costs.push(OptionalCost::entwine(cost));
        self
    }

    /// Add an entwine cost using just mana.
    pub fn entwine_mana(self, cost: ManaCost) -> Self {
        self.entwine(TotalCost::mana(cost))
    }

    /// Add an offspring cost (can pay once for a 1/1 copy ETB trigger).
    pub fn offspring(mut self, cost: TotalCost) -> Self {
        self.optional_costs.push(OptionalCost::offspring(cost));
        self
    }

    /// Add an offspring cost using just mana.
    pub fn offspring_mana(self, cost: ManaCost) -> Self {
        self.offspring(TotalCost::mana(cost))
    }

    /// Add a custom optional cost.
    pub fn optional_cost(mut self, cost: OptionalCost) -> Self {
        self.optional_costs.push(cost);
        self
    }

    /// Set additional spell cost components.
    pub fn costs(mut self, costs: Vec<crate::costs::Cost>) -> Self {
        self.additional_cost = TotalCost::from_costs(costs);
        self
    }

    /// Set additional spell cost as a `TotalCost`.
    pub fn additional_cost(mut self, additional_cost: TotalCost) -> Self {
        self.additional_cost = additional_cost;
        self
    }

    /// Add a saga chapter ability that triggers on a single chapter.
    ///
    /// # Example
    /// ```ignore
    /// .with_chapter(1, vec![Effect::sacrifice(ObjectFilter::creature(), 1)])  // Chapter I
    /// ```
    pub fn with_chapter(self, chapter: u32, effects: Vec<Effect>) -> Self {
        use crate::triggers::Trigger;
        self.with_trigger(Trigger::saga_chapter(vec![chapter]), effects)
    }

    /// Add a saga chapter ability that triggers on multiple chapters.
    ///
    /// Use this for "I, II" style abilities that trigger on multiple chapters.
    ///
    /// # Example
    /// ```ignore
    /// .with_chapters(vec![1, 2], vec![Effect::draw(1)])  // Chapters I, II
    /// ```
    pub fn with_chapters(self, chapters: Vec<u32>, effects: Vec<Effect>) -> Self {
        use crate::triggers::Trigger;
        self.with_trigger(Trigger::saga_chapter(chapters), effects)
    }

    // === Level-Up Support ===

    /// Add a level-up activated ability.
    ///
    /// Level-up is an activated ability that can only be activated at sorcery speed.
    /// It adds a level counter to the creature.
    ///
    /// # Arguments
    /// * `cost` - The mana cost to level up
    ///
    /// # Example
    /// ```ignore
    /// .level_up(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
    /// ```
    pub fn level_up(self, cost: ManaCost) -> Self {
        use crate::ability::{AbilityKind, ActivatedAbility};
        use crate::zone::Zone;

        let ability = Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: TotalCost::mana(cost),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::put_counters_on_source(CounterType::Level, 1),
                ]),
                choices: vec![],
                timing: ActivationTiming::SorcerySpeed,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Battlefield],
        };
        self.with_ability(ability)
    }

    /// Add level-based abilities.
    ///
    /// Level abilities grant different P/T and abilities based on the number of
    /// level counters on the creature. Only one tier applies at a time.
    ///
    /// # Example
    /// ```ignore
    /// .with_level_abilities(vec![
    ///     LevelAbility::new(2, Some(6)).with_pt(3, 3).with_ability(StaticAbility::first_strike()),
    ///     LevelAbility::new(7, None).with_pt(4, 4).with_ability(StaticAbility::double_strike()),
    /// ])
    /// ```
    pub fn with_level_abilities(self, levels: Vec<LevelAbility>) -> Self {
        self.with_ability(Ability::static_ability(
            StaticAbility::with_level_abilities(levels),
        ))
    }

    fn fabricate_servo_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Servo")
            .token()
            .card_types(vec![CardType::Artifact, CardType::Creature])
            .subtypes(vec![Subtype::Servo])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build()
    }

    fn afterlife_spirit_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Spirit")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Spirit])
            .color_indicator(ColorSet::WHITE.union(ColorSet::BLACK))
            .power_toughness(PowerToughness::fixed(1, 1))
            .flying()
            .build()
    }

    fn for_mirrodin_rebel_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Rebel")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Rebel])
            .color_indicator(ColorSet::RED)
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn living_weapon_germ_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Phyrexian Germ")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Phyrexian, Subtype::Germ])
            .color_indicator(ColorSet::BLACK)
            .power_toughness(PowerToughness::fixed(0, 0))
            .build()
    }

    fn mobilize_warrior_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Warrior")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Warrior])
            .color_indicator(ColorSet::RED)
            .power_toughness(PowerToughness::fixed(1, 1))
            .build()
    }

    // === Build ===

    /// Build the card definition.
    pub fn build(self) -> CardDefinition {
        let refers_to_ante = self
            .card_builder
            .oracle_text_ref()
            .split(|character: char| !character.is_ascii_alphanumeric())
            .any(|word| word.eq_ignore_ascii_case("ante"));
        let definition = finalize_backup_abilities(CardDefinition {
            card: self.card_builder.build(),
            abilities: self.abilities,
            spell_effect: self.spell_effect,
            aura_attach_filter: self.aura_attach_filter,
            alternative_casts: self.alternative_casts,
            has_fuse: self.has_fuse,
            optional_costs: self.optional_costs,
            additional_cost: self.additional_cost,
            refers_to_ante,
        });
        finalize_cipher_effects(definition)
    }
}

fn supported_keyword_marker_text(text: &str) -> bool {
    let text = text.trim_start().to_ascii_lowercase();
    text == "compleated"
        || text.starts_with("prototype ")
        || text.starts_with("splice onto ")
        || is_ticket_sticker_marker_line(&text)
}

fn is_ticket_sticker_marker_line(text: &str) -> bool {
    let Some((cost, body_text)) = text.split_once('—') else {
        return false;
    };

    let mut saw_ticket_symbol = false;
    let mut remainder = cost.trim();
    while let Some(next) = remainder.strip_prefix("{tk}") {
        saw_ticket_symbol = true;
        remainder = next.trim_start();
    }
    if !saw_ticket_symbol || !remainder.is_empty() {
        return false;
    }

    !body_text.trim().is_empty()
}

fn parse_standalone_bolster_marker(text: &str) -> Option<u32> {
    let mut parts = text.split_whitespace();
    matches!(parts.next(), Some(keyword) if keyword.eq_ignore_ascii_case("bolster"))
        .then(|| parts.next().and_then(|amount| amount.parse::<u32>().ok()))
        .flatten()
        .filter(|_| parts.next().is_none())
}

#[cfg(all(test, ironsmith_runtime_legacy_parser_unit_tests))]
mod delayed_trigger_finalization_tests;

#[cfg(test)]
mod keyword_behavior_tests;

#[cfg(test)]
mod scheme_type_line_tests {
    use super::*;

    #[test]
    fn ongoing_scheme_is_a_supertype_and_nontraditional_card_type() {
        let (supertypes, card_types, subtypes) =
            parse_type_line("Ongoing Scheme").expect("canonical Scheme type line");
        assert_eq!(supertypes, vec![Supertype::Ongoing]);
        assert_eq!(card_types, vec![CardType::Scheme]);
        assert!(subtypes.is_empty());
    }
}

#[cfg(all(test, ironsmith_runtime_removed_parser_helper_unit_tests))]
mod target_parse_tests;

#[cfg(all(test, ironsmith_runtime_legacy_parser_unit_tests))]
mod effect_parse_tests;

fn scale_value(base: Value, factor: u32) -> Option<Value> {
    if factor == 0 {
        return None;
    }
    let mut value = base.clone();
    for _ in 1..factor {
        value = Value::Add(Box::new(value), Box::new(base.clone()));
    }
    Some(value)
}

#[cfg(all(test, ironsmith_runtime_legacy_parser_unit_tests))]
mod tests;
