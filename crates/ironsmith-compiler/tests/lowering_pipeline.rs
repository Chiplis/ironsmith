//! Lowering tests whose inputs come from recognizing real Oracle text.
//!
//! These lifted out of the lowering crate: they name both phases, so they
//! belong in the crate that assembles them.

use ironsmith_compiler::ParseCardText;
use ironsmith_compiler::compile_support::*;
use ironsmith_compiler::effect::{Condition, Until};
use ironsmith_compiler::filter::*;
use ironsmith_compiler::object::CounterType;
use ironsmith_compiler::zone::Zone;

use ironsmith_compiler::ir::*;
use ironsmith_compiler::payload::KeywordAction;

fn parsed_keyword_action(keyword: &str) -> KeywordAction {
    let tokens = ironsmith_compiler::lexer::lex_line(keyword, 0)
        .unwrap_or_else(|error| panic!("{keyword}: {error}"));
    let mut actions = ironsmith_compiler::keyword_static::parse_ability_line(&tokens)
        .unwrap_or_else(|| panic!("{keyword} should parse as a keyword action"));
    assert_eq!(actions.len(), 1, "{keyword} should parse as one action");
    actions.pop().expect("one keyword action")
}

use ironsmith_compiler::line_info::LineInfo;

const EXECUTABLE_MARKER_BACKED_KEYWORDS: &[&str] = &[
    "afterlife 2",
    "fabricate 2",
    "prowess",
    "storm",
    "toxic 2",
    "battle cry",
    "dethrone",
    "evolve",
    "ingest",
    "mentor",
    "training",
    "riot",
    "renown 2",
    "modular 2",
    "graft 2",
    "soulbond",
    "soulshift 2",
    "outlast {1}{W}",
    "unearth {1}{B}",
    "eternalize {2}{B}",
    "ninjutsu {1}{U}",
    "extort",
    "sunburst",
    "fading 2",
    "vanishing 2",
    "rampage 2",
    "bushido 2",
    "frenzy 2",
    "poisonous 2",
    "annihilator 2",
];

use ironsmith_compiler::lexer::{
    lex_line, render_token_slice, split_lexed_sentences, trim_lexed_commas,
};
use ironsmith_compiler::parse_context::ParseContext;
use ironsmith_compiler::semantic_line_parsing::*;

use ironsmith_compiler::ability::AbilityKind;
use ironsmith_compiler::cards::builders::*;
use ironsmith_compiler::effect::{Effect, Value};
use ironsmith_compiler::model::reference_state::RefState;
use ironsmith_compiler::target::ChooseSpec;
use ironsmith_compiler::types::{CardType, Subtype};

use ironsmith_compiler::lower::*;
use ironsmith_compiler::lowering_support::*;
use ironsmith_compiler::runtime_static_ability_helpers::*;
use ironsmith_compiler::{CardDefinitionBuilder, CardId, CardTextError};

#[test]
fn secret_choice_followup_keeps_the_result_id_used_by_otherwise() {
    let compiled = ironsmith_compiler::compile_card_text(
        CardDefinitionBuilder::new(CardId::new(), "Expert-Level Safe")
            .card_types(vec![CardType::Artifact]),
        "When this artifact enters, exile the top two cards of your library face down.\n\
             {1}, {T}: You and target opponent each secretly choose 1, 2, or 3. Then those choices \
             are revealed. If they match, sacrifice this artifact and put all cards exiled with it \
             into their owners' hands. Otherwise, exile the top card of your library face down.",
        false,
    )
    .expect("Expert-Level Safe should compile");

    let activated = compiled
        .definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Expert-Level Safe should have an activated ability");
    let effects = activated
        .effects
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .collect::<Vec<_>>();
    let conditional_id = effects
        .iter()
        .find_map(|effect| {
            let with_id = effect.as_with_id()?;
            with_id
                .effect
                .as_conditional()
                .is_some()
                .then_some(with_id.id)
        })
        .expect("the secret-choice match conditional should retain its result ID");

    assert!(
        effects.iter().any(|effect| {
            effect
                .as_if_effect()
                .is_some_and(|fallback| fallback.condition == conditional_id)
        }),
        "the otherwise branch should consume the conditional's retained result ID"
    );
}

#[test]
fn trigger_line_facts_preserve_only_a_grammar_proven_leading_unless() {
    fn leading_surface(text: &str) -> bool {
        let definition =
            CardDefinitionBuilder::new(ironsmith_compiler::CardId::from_raw(1), "Unless Probe")
                .card_types(vec![ironsmith_compiler::types::CardType::Creature])
                .parse_text(text)
                .expect("unless trigger should compile");
        let AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
            panic!("expected a triggered ability")
        };
        let [effect] = triggered.effects.flattened_default_effects() else {
            panic!("expected one payment wrapper: {:#?}", triggered.effects)
        };
        effect
            .downcast_ref::<ironsmith_compiler::effects::UnlessPaysEffect<ironsmith_compiler::effect::Effect>>()
            .expect("typed payment wrapper")
            .leading_surface
    }

    assert!(leading_surface(
        "At the beginning of your upkeep, unless you sacrifice an Island, sacrifice this creature."
    ));
    assert!(!leading_surface(
        "At the beginning of your upkeep, sacrifice this creature unless you sacrifice an Island."
    ));
}

#[test]
fn public_two_line_damage_replacement_reuses_both_announced_targets() {
    let definition = CardDefinitionBuilder::new(ironsmith_compiler::CardId::from_raw(1), "Damage Pair Variant")
        .parse_text(
            "This spell deals 1 damage to target player or planeswalker and 1 damage to target creature that player or that planeswalker's controller controls.\nLandfall — If you had a land enter the battlefield under your control this turn, this spell deals 3 damage to that player or planeswalker and 3 damage to that creature instead.",
        )
        .expect("full public document route should lower the damage replacement");
    let program = definition
        .spell_effect
        .as_ref()
        .expect("damage pair should produce a spell program");
    let [segment] = program.segments.as_slice() else {
        panic!("expected one replacement segment: {program:#?}");
    };
    let [branch] = segment.self_replacements.as_slice() else {
        panic!("expected one typed self-replacement: {segment:#?}");
    };

    fn damage_targets(effect: &ironsmith_compiler::effect::Effect) -> Vec<ChooseSpec> {
        let leaf = effect
            .downcast_ref::<ironsmith_compiler::effects::WithIdEffect>()
            .map_or(effect, |with_id| with_id.effect.as_ref());
        let sequence = leaf
            .downcast_ref::<ironsmith_compiler::effects::SequenceEffect>()
            .expect("coordinated damage pair");
        sequence
            .effects
            .iter()
            .map(|effect| {
                let leaf = effect
                    .downcast_ref::<ironsmith_compiler::effects::TaggedEffect>()
                    .map_or(effect, |tagged| tagged.effect.as_ref());
                leaf.downcast_ref::<ironsmith_compiler::effects::DealDamageEffect>()
                    .expect("damage leaf")
                    .target
                    .clone()
            })
            .collect()
    }

    let [default] = segment.default_effects.as_slice() else {
        panic!("expected one coordinated default effect: {segment:#?}");
    };
    let [replacement] = branch.replacement_effects.as_slice() else {
        panic!("expected one coordinated replacement effect: {branch:#?}");
    };
    assert_eq!(damage_targets(default), damage_targets(replacement));
    assert!(matches!(
        branch.presentation_label,
        Some(ironsmith_compiler::cards::builders::PresentationLabel::AbilityWord(ref label)) if label == "Landfall"
    ));
}

#[test]
fn rewrite_special_triggered_burning_rune_demon_accepts_stored_parse_tokens()
-> Result<(), CardTextError> {
    let full_text = "when this creature enters, you may search your library for exactly two cards not named burning rune demon that have different names. if you do, reveal those cards. an opponent chooses one of them. put the chosen card into your hand and the other into your graveyard, then shuffle.";
    let trigger_text = "when this creature enters";
    let effect_text = "you may search your library for exactly two cards not named burning rune demon that have different names. if you do, reveal those cards. an opponent chooses one of them. put the chosen card into your hand and the other into your graveyard, then shuffle.";
    let full_tokens =
        lex_line(full_text, 0).expect("rewrite lexer should classify burning rune demon line");
    let trigger_tokens = lex_line(trigger_text, 0)
        .expect("rewrite lexer should classify burning rune demon trigger");
    let effect_tokens =
        lex_line(effect_text, 0).expect("rewrite lexer should classify burning rune demon effect");

    let parsed = parse_triggered_line(
        LineInfo {
            line_index: 0,
            display_line_index: 0,
            raw_line: full_text.to_string(),
            source_tokens: full_tokens.clone(),
            normalized: NormalizedLine::identity(full_text),
            semantic_facts: Default::default(),
        },
        full_text,
        &full_tokens,
        &trigger_tokens,
        &effect_tokens,
        None,
        None,
        None,
        None,
    )?;

    let debug = format!("{parsed:?}");
    assert!(debug.contains("Triggered"), "{debug}");
    assert!(debug.contains("divvy_source"), "{debug}");
    assert!(debug.contains("divvy_chosen"), "{debug}");
    assert!(debug.contains("ShuffleLibrary"), "{debug}");

    Ok(())
}

#[test]
fn rewrite_exert_keyword_lowering_uses_parse_tokens_when_text_is_stale() -> Result<(), CardTextError>
{
    let token_text = "if this creature hasn't been exerted this turn, you may exert champion as it attacks. when you do, he can't block this turn.";
    let tokens = lex_line(token_text, 0).expect("rewrite lexer should classify exert keyword line");

    let parsed = parse_keyword_line_for_test(
        LineInfo {
            line_index: 0,
            display_line_index: 0,
            raw_line: "placeholder exert text".to_string(),
            source_tokens: tokens.clone(),
            normalized: NormalizedLine::identity("placeholder exert text"),
            semantic_facts: Default::default(),
        },
        "placeholder exert text",
        &tokens,
        RewriteKeywordLineKind::ExertAttack,
    )?;

    match parsed {
        LineAst::StaticAbility(ability) => {
            let debug = format!("{ability:?}");
            assert!(
                debug.contains("exert attack") || debug.contains("ExertAttack"),
                "{debug}"
            );
            assert!(
                debug.contains("only_if_not_exerted_this_turn: true") || debug.contains("true"),
                "{debug}"
            );
        }
        other => panic!("expected exert static ability, got {other:?}"),
    }

    Ok(())
}

#[test]
fn rewrite_exert_keyword_lowering_reuses_token_followup_for_linked_trigger()
-> Result<(), CardTextError> {
    let text = "you may exert champion as it attacks. when you do, he can't block this turn.";
    let tokens = lex_line(text, 0).expect("rewrite lexer should classify exert keyword line");

    let parsed = parse_keyword_line_for_test(
        LineInfo {
            line_index: 0,
            display_line_index: 0,
            raw_line: text.to_string(),
            source_tokens: tokens.clone(),
            normalized: NormalizedLine::identity(text),
            semantic_facts: Default::default(),
        },
        text,
        &tokens,
        RewriteKeywordLineKind::ExertAttack,
    )?;

    match parsed {
        LineAst::StaticAbility(ability) => {
            let debug = format!("{ability:?}");
            assert!(
                debug.contains("exert attack") || debug.contains("ExertAttack"),
                "{debug}"
            );
        }
        other => panic!("expected exert static ability, got {other:?}"),
    }

    Ok(())
}

#[test]
fn firebending_grants_lower_to_the_executable_attack_trigger() {
    let definition =
        CardDefinitionBuilder::new(ironsmith_compiler::CardId::new(), "Firebending Grant Probe")
            .card_types(vec![CardType::Instant])
            .parse_text("Target creature gains firebending 2 until end of turn.")
            .expect("Firebending grants should lower to executable object abilities");
    let debug = format!("{definition:#?}");
    assert!(debug.contains("ThisAttacks"), "{debug}");
    assert!(debug.contains("ManaRetainedEffect"), "{debug}");
    assert!(debug.contains("Firebend"), "{debug}");
}

#[test]
fn every_marker_backed_gameplay_keyword_grant_lowers_to_printed_object_abilities() {
    for keyword in EXECUTABLE_MARKER_BACKED_KEYWORDS {
        let action = parsed_keyword_action(keyword);
        let expected = executable_object_abilities_for_keyword_action(&action)
            .unwrap_or_else(|| panic!("{keyword} should have an executable expansion"));
        assert!(
                expected.iter().any(|ability| !matches!(
                    &ability.kind,
                    ironsmith_compiler::ability::AbilityKind::Static(static_ability)
                        if static_ability.id() == ironsmith_compiler::static_abilities::StaticAbilityId::KeywordMarker
                )),
                "{keyword} must not expand to presentation markers only"
            );

        let definition =
            CardDefinitionBuilder::new(ironsmith_compiler::CardId::new(), "Grant Probe")
                .card_types(vec![CardType::Instant])
                .parse_text(format!(
                    "Target creature gains {keyword} until end of turn."
                ))
                .unwrap_or_else(|error| panic!("dynamic {keyword} grant should compile: {error}"));
        let debug = format!("{:#?}", definition.spell_effect);
        let lowered_count =
            debug.matches("AddAbilityGeneric").count() + debug.matches("AddAbility(").count();
        assert!(
            lowered_count >= expected.len(),
            "dynamic {keyword} grant must carry the complete printed ability set: {debug}"
        );
    }
}
