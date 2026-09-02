use super::super::super::lexer::lex_line;
use super::super::super::util::tokenize_line;
use super::*;
use crate::model::CompilerAbilityKindCore as AbilityKind;
use crate::{CardId, ChoiceCount};
#[cfg(test)]
use ironsmith_compiler::ParseCardText;
#[cfg(test)]
use ironsmith_compiler_lowering::CardDefinitionBuilder;

fn sole_typed_coordination(effects: &[EffectAst]) -> &crate::model::CoordinationAst {
    match effects {
        [EffectAst::Coordination(coordination)] => coordination,
        [EffectAst::ControlFlow(control)] => {
            let crate::model::control_flow::ControlFlowNodeAst::Duration { program, .. } =
                &control.node
            else {
                panic!("expected a duration-wrapped coordination: {effects:#?}");
            };
            sole_typed_coordination(&control.program(*program).expect("duration program").effects)
        }
        _ => panic!("expected one canonical coordination: {effects:#?}"),
    }
}

#[test]
fn quoted_filtered_static_rule_remains_an_ability_of_the_token() {
    let definition = crate::grammar::token_definitions::parse_token_definition_shape_text(
        "1/1 red Pirate creature token",
    )
    .expect("Pirate token definition");
    let tokens = lex_line("Creatures you control attack each combat if able.", 0)
        .expect("filtered quoted rule should lex");
    let parsed = parse_granted_abilities_for_token_definition(&definition, &tokens)
        .expect("filtered quoted rule should parse under the token identity");
    let [GrantedAbilityAst::StaticAbility(ability)] = parsed.as_slice() else {
        panic!("expected one filtered static carrier: {parsed:#?}");
    };
    let crate::cards::builders::StaticAbilityAst::GrantStaticAbility {
        filter, ability, ..
    } = ability.as_ref()
    else {
        panic!("expected one compiler filtered static grant: {ability:#?}");
    };
    let crate::cards::builders::StaticAbilityAst::Static(ability) = ability.as_ref() else {
        panic!("expected one compiler static ability: {ability:#?}");
    };
    assert_eq!(filter.card_types, [CardType::Creature]);
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert_eq!(
        ability.id(),
        crate::static_abilities::StaticAbilityId::MustAttack
    );
}

#[test]
fn triggered_grant_display_keeps_fixed_numbers_out_of_mana_braces() {
    let tokens = lex_line(
        "whenever this creature dies, each opponent loses 1 life and you gain 2 life",
        0,
    )
    .expect("granted trigger should lex");

    assert_eq!(
        display_text_for_tokens(&tokens),
        "whenever this creature dies, each opponent loses 1 life and you gain 2 life"
    );
}

#[test]
fn leading_trigger_wins_over_colon_inside_nested_token_ability() {
    let tokens = lex_line(
            "When this token dies, create a 2/2 red Dragon creature token with flying and '{R}: This token gets +1/+0 until end of turn.'",
            0,
        )
        .expect("nested token trigger should lex");
    let words = crate::lexer::token_word_refs(&tokens);
    let parsed = parse_granted_activated_or_triggered_ability_for_gain(&tokens, &words)
        .expect("nested token trigger should parse")
        .expect("nested token trigger should produce an ability");
    let GrantedAbilityAst::ParsedObjectAbility { ability, .. } = parsed else {
        panic!("expected a parsed object ability");
    };
    assert!(
        matches!(ability.kind(), AbilityKind::Triggered(_)),
        "the nested activation must not become the outer ability: {ability:#?}"
    );
}

#[test]
fn named_quoted_token_death_trigger_keeps_authored_when_surface() {
    for (intro, expected) in [
        ("When", crate::model::ast::TriggerIntroSurfaceAst::When),
        (
            "Whenever",
            crate::model::ast::TriggerIntroSurfaceAst::Whenever,
        ),
    ] {
        let tokens = lex_line(
            &format!("{intro} Ember dies, create fourteen Treasure tokens."),
            0,
        )
        .expect("named token death trigger should lex");
        let words = crate::lexer::token_word_refs(&tokens);
        let parsed = parse_granted_activated_or_triggered_ability_for_gain(&tokens, &words)
            .expect("named token death trigger should parse")
            .expect("named token death trigger should produce an ability");
        let GrantedAbilityAst::ParsedObjectAbility { ability, .. } = parsed else {
            panic!("expected a parsed object ability");
        };
        let AbilityKind::Triggered(_) = ability.kind() else {
            panic!("expected a triggered token ability: {ability:#?}");
        };
        assert!(matches!(
            ability.trigger_spec.as_deref(),
            Some(crate::model::ast::TriggerSpec::WithIntro { intro, .. }) if *intro == expected
        ));
    }
}

#[test]
fn edge_trimming_preserves_nested_rules_closing_quote() {
    for text in [
        "When this token dies, create a token with '{R}: This token gets +1/+0 until end of turn.'",
        "When this token dies, create a token with \"{R}: This token gets +1/+0 until end of turn.\"",
    ] {
        let tokens = lex_line(text, 0).expect("nested token rule should lex");
        let trimmed = trim_edge_punctuation_and_quotes(&tokens);
        let quote_count = trimmed
            .iter()
            .filter(|token| matches!(token.kind, TokenKind::Quote | TokenKind::Apostrophe))
            .count();

        assert_eq!(quote_count, 2, "{trimmed:#?}");
        assert!(
            trimmed.last().is_some_and(|token| matches!(
                token.kind,
                TokenKind::Quote | TokenKind::Apostrophe
            )),
            "{trimmed:#?}"
        );
    }
}

#[test]
fn quoted_mixed_ability_list_splits_only_at_top_level_separators() {
    let ability_tokens = lex_line(
        "indestructible, \"Equipped creature gets +5/+5 and has double strike,\" and equip {0}.",
        0,
    )
    .expect("mixed granted-ability list should lex");
    let clause_words = crate::lexer::token_word_refs(&ability_tokens);
    let (abilities, is_choice) =
        parse_granted_abilities_for_gain_clause(&ability_tokens, &clause_words, false)
            .expect("mixed granted-ability list should parse");
    assert!(!is_choice);
    let debug = format!("{abilities:#?}");
    assert!(debug.contains("Indestructible"), "{debug}");
    assert!(debug.contains("Anthem"), "{debug}");
    assert!(debug.contains("DoubleStrike"), "{debug}");
    assert!(debug.contains("Equip {0}"), "{debug}");
}

#[test]
fn oxford_list_with_final_quoted_ability_is_not_a_choice() {
    let ability_tokens = lex_line(
        "vigilance, indestructible, and \"This creature can't be blocked.\"",
        0,
    )
    .expect("mixed granted-ability list should lex");
    let clause_words = crate::lexer::token_word_refs(&ability_tokens);
    let (abilities, is_choice) =
        parse_granted_abilities_for_gain_clause(&ability_tokens, &clause_words, true)
            .expect("mixed granted-ability list should parse");

    assert!(!is_choice, "{abilities:#?}");
    assert_eq!(abilities.len(), 3, "{abilities:#?}");
    assert!(matches!(
        &abilities[2],
        GrantedAbilityAst::StaticAbility(ability)
            if matches!(
                ability.as_ref(),
                crate::cards::builders::StaticAbilityAst::Static(ability)
                    if ability.id() == StaticAbilityId::RuleRestriction
            )
    ));
}

#[test]
fn keyword_before_final_quoted_ability_is_preserved() {
    let ability_tokens = lex_line("trample and \"{G}: Regenerate this creature.\"", 0)
        .expect("mixed granted-ability list should lex");
    let clause_words = crate::lexer::token_word_refs(&ability_tokens);
    let (abilities, is_choice) =
        parse_granted_abilities_for_gain_clause(&ability_tokens, &clause_words, true)
            .expect("mixed granted-ability list should parse");
    let debug = format!("{abilities:#?}");

    assert!(!is_choice, "{debug}");
    assert_eq!(abilities.len(), 2, "{debug}");
    assert!(debug.contains("Trample"), "{debug}");
    assert!(debug.contains("Regenerate"), "{debug}");
}

#[test]
fn become_then_oxford_grant_list_keeps_all_grants_nonmodal() {
    let tokens = tokenize_line(
        "Target artifact you control becomes a 9/9 Construct artifact creature and gains vigilance, indestructible, and \"This creature can't be blocked.\"",
        0,
    );
    let effects = super::super::parse_effect_sentences_lexed(&tokens)
        .expect("become-and-grant sentence should parse through the full effect pipeline");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("BecomeBasePtCreature"), "{debug}");
    assert!(debug.contains("GrantAbilitiesToTarget"), "{debug}");
    assert!(!debug.contains("GrantAbilitiesChoiceToTarget"), "{debug}");
    assert!(
        debug.contains("RuleRestriction"),
        "quoted can't-be-blocked clause must remain a typed quoted rule: {debug}"
    );
}

#[test]
fn explicit_copy_subject_uses_the_copy_result_tag() {
    let tokens = tokenize_line("The copy gains haste.", 0);
    let effects = parse_gain_ability_sentence(&tokens)
        .expect("copy-result grant should parse")
        .expect("copy-result grant should produce effects");
    let debug = format!("{effects:#?}");

    assert!(
        debug.contains(crate::tag::CompilerReferenceTag::CopiedStackObject.as_str()),
        "{debug}"
    );
    assert!(
        !debug.contains(&format!(
            "TagKey(\n                    \"{}\"",
            crate::tag::CompilerReferenceTag::It.as_str()
        )),
        "{debug}"
    );
}

#[test]
fn mixed_keyword_list_keeps_static_keyword_after_executable_keyword() {
    let ability_tokens =
        lex_line("trample, annihilator 2, and haste", 0).expect("mixed keyword grant should lex");
    let clause_words = crate::lexer::token_word_refs(&ability_tokens);
    let (abilities, is_choice) =
        parse_granted_abilities_for_gain_clause(&ability_tokens, &clause_words, false)
            .expect("mixed keyword grant should parse");

    assert!(!is_choice);
    assert_eq!(abilities.len(), 3, "{abilities:#?}");
    assert!(
        matches!(
            &abilities[0],
            GrantedAbilityAst::KeywordAction(action)
                if matches!(action.as_ref(), KeywordAction::Trample)
        ),
        "{abilities:#?}"
    );
    assert!(
        matches!(
            &abilities[1],
            GrantedAbilityAst::KeywordAction(action)
                if matches!(action.as_ref(), KeywordAction::Annihilator(2))
        ),
        "{abilities:#?}"
    );
    assert!(
        matches!(
            &abilities[2],
            GrantedAbilityAst::KeywordAction(action)
                if matches!(action.as_ref(), KeywordAction::Haste)
        ),
        "{abilities:#?}"
    );
}

#[test]
fn effect_chain_keeps_keyword_after_oxford_comma_normalization() {
    let tokens = lex_line(
            "Until end of turn, it has base power and toughness 10/10 and gains trample, annihilator 2, and haste.",
            0,
        )
        .expect("leading-duration mixed grant should lex");
    let effects = parse_effect_chain(&tokens)
        .expect("leading-duration mixed grant should parse through the effect chain");

    let ast_debug = format!("{effects:#?}");
    assert!(ast_debug.contains("SetBasePowerToughness"), "{ast_debug}");
    assert!(ast_debug.contains("Trample"), "{ast_debug}");
    assert!(ast_debug.contains("Annihilator"), "{ast_debug}");
    assert!(ast_debug.contains("Haste"), "{ast_debug}");

    let compiled = compile_statement_effects(&effects)
        .expect("leading-duration mixed grant should lower to runtime effects");
    let compiled_debug = format!("{compiled:#?}");
    assert!(compiled_debug.contains("Trample"), "{compiled_debug}");
    // Annihilator is a keyword action in the AST, but lowers to its
    // trigger-and-sacrifice runtime representation rather than retaining
    // the keyword name in the compiled debug form.
    assert!(
        compiled_debug.contains("SacrificePlayerEffect"),
        "{compiled_debug}"
    );
    let compact_debug: String = compiled_debug.split_whitespace().collect();
    assert!(compact_debug.contains("count:Fixed(2"), "{compiled_debug}");
    assert!(compiled_debug.contains("Haste"), "{compiled_debug}");
}

#[test]
fn gain_ability_to_source_keeps_parsed_ability_until_lowering() {
    let tokens = tokenize_line("This creature gains {T}: Draw a card.", 0);
    let effect = parse_gain_ability_to_source_sentence(&tokens)
        .expect("gain-to-source sentence should parse")
        .expect("gain-to-source sentence should produce an effect");

    let debug = format!("{effect:?}");
    assert!(
        string_contains(&debug, "GrantAbilityToSource"),
        "expected source grant effect, got {debug}"
    );
    assert!(
        string_contains(&debug, "duration: Forever"),
        "source ability grants without an explicit duration should be indefinite, got {debug}"
    );
    assert!(
        string_contains(&debug, "effects_ast: Some"),
        "expected parsed ability to remain unlowered in the AST, got {debug}"
    );

    let compiled =
        compile_statement_effects(&[effect]).expect("grant-to-source effect should lower");
    let compiled_debug = format!("{compiled:?}");
    assert!(
        (string_contains(&compiled_debug, "ApplyContinuousEffect")
            && string_contains(&compiled_debug, "AddAbilityGeneric")
            && string_contains(&compiled_debug, "target_spec: Some(Source)")
            && string_contains(&compiled_debug, "until: Forever"))
            || (string_contains(&compiled_debug, "GrantObjectAbilityEffect")
                && string_contains(&compiled_debug, "target: Source")),
        "expected source grant effect after lowering, got {compiled_debug}"
    );
}

#[test]
fn gain_ability_to_source_respects_explicit_until_end_of_turn_duration() {
    let tokens = tokenize_line("This creature gains {T}: Draw a card until end of turn.", 0);
    let effect = parse_gain_ability_to_source_sentence(&tokens)
        .expect("gain-to-source sentence should parse")
        .expect("gain-to-source sentence should produce an effect");

    let debug = format!("{effect:?}");
    assert!(
        string_contains(&debug, "GrantAbilityToSource"),
        "expected source grant effect, got {debug}"
    );
    assert!(
        string_contains(&debug, "duration: EndOfTurn"),
        "explicit source ability grant duration should be preserved, got {debug}"
    );
}

#[test]
fn quoted_nested_trigger_grant_keeps_outer_until_end_of_turn_duration() {
    let tokens = tokenize_line(
        "It gains \"Whenever this creature deals combat damage to a player, draw two cards\" until end of turn.",
        0,
    );
    let effect = parse_gain_ability_sentence(&tokens)
        .expect("quoted nested trigger grant should parse")
        .expect("quoted nested trigger grant should produce effects")
        .into_iter()
        .next()
        .expect("quoted nested trigger grant should produce one effect");

    let debug = format!("{effect:?}");
    assert!(
        string_contains(&debug, "GrantAbilities")
            && string_contains(&debug, "ParsedObjectAbility")
            && string_contains(&debug, "duration: EndOfTurn")
            && string_contains(&debug, "Draw")
            && string_contains(&debug, "Fixed(2)"),
        "expected quoted combat-damage draw trigger to be granted until end of turn, got {debug}"
    );
}

#[test]
fn keyword_and_quoted_trigger_share_target_and_duration() {
    let tokens = tokenize_line(
        "Until end of turn, target creature you control with power 4 or greater gains trample and \"Whenever this creature deals combat damage to a player, draw a card.\"",
        0,
    );
    let effects = parse_gain_ability_sentence(&tokens)
        .expect("mixed keyword and quoted trigger grant should parse")
        .expect("mixed grant should produce effects");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("Trample"), "{debug}");
    assert!(debug.contains("ThisDealsCombatDamageToPlayer"), "{debug}");
    assert!(debug.contains("Draw"), "{debug}");
    assert!(debug.contains("duration: EndOfTurn"), "{debug}");
    assert!(debug.contains("power: Some"), "{debug}");
}

#[test]
fn activated_line_keeps_mixed_keyword_and_quoted_trigger_together() {
    let (parsed, trace) = crate::parse_trace::capture(|| {
        CardDefinitionBuilder::new(CardId::from_raw(1), "Test Heirloom").parse_text(
                "{T}: Until end of turn, target creature you control with power 4 or greater gains trample and \"Whenever this creature deals combat damage to a player, draw a card.\"",
            )
    });
    let def = parsed.unwrap_or_else(|error| {
        panic!(
            "mixed grant inside an activated line should parse: {error:?}\n{}",
            trace.render()
        )
    });
    let debug = format!("{def:#?}");

    assert!(debug.contains("Trample"), "{debug}");
    assert!(debug.contains("ThisDealsCombatDamageToPlayer"), "{debug}");
    assert!(debug.contains("Draw"), "{debug}");
    assert!(debug.contains("EndOfTurn"), "{debug}");
}

#[test]
fn target_gain_activated_ability_stays_unlowered_until_compile() {
    let tokens = tokenize_line(
        "Target creature gains {T}: Draw a card until end of turn.",
        0,
    );
    let effect = parse_simple_gain_ability_clause(&tokens)
        .expect("target gain clause should parse")
        .expect("target gain clause should produce an effect");

    let debug = format!("{effect:?}");
    assert!(
        string_contains(&debug, "ParsedObjectAbility"),
        "expected parsed granted ability in AST, got {debug}"
    );
    assert!(
        string_contains(&debug, "effects_ast: Some"),
        "expected granted ability to remain unlowered in AST, got {debug}"
    );

    let compiled = compile_statement_effects(&[effect]).expect("target gain clause should lower");
    let compiled_debug = format!("{compiled:?}");
    assert!(
        string_contains(&compiled_debug, "ApplyContinuousEffect")
            && (string_contains(&compiled_debug, "AddAbilityGeneric")
                || string_contains(&compiled_debug, "GrantObjectAbilityForFilter")),
        "expected lowered granted ability effect, got {compiled_debug}"
    );
}

#[test]
fn target_lose_activated_ability_stays_unlowered_until_compile() {
    let tokens = tokenize_line(
        "Target creature loses {T}: Draw a card until end of turn.",
        0,
    );
    let effect = parse_simple_lose_ability_clause(&tokens)
        .expect("target lose clause should parse")
        .expect("target lose clause should produce an effect");

    let debug = format!("{effect:?}");
    assert!(
        string_contains(&debug, "ParsedObjectAbility"),
        "expected parsed removed ability in AST, got {debug}"
    );
    assert!(
        string_contains(&debug, "effects_ast: Some"),
        "expected removed ability to remain unlowered in AST, got {debug}"
    );

    let compiled = compile_statement_effects(&[effect]).expect("target lose clause should lower");
    let compiled_debug = format!("{compiled:?}");
    assert!(
        string_contains(&compiled_debug, "RemoveAbility"),
        "expected lowered remove-ability effect, got {compiled_debug}"
    );
    assert!(
        string_contains(&compiled_debug, "ApplyContinuousEffect"),
        "expected removed ability to lower through a continuous effect, got {compiled_debug}"
    );
}

#[test]
fn pump_and_lose_ability_sentence_keeps_shared_until_your_next_turn() {
    let tokens = tokenize_line(
        "Target creature gets -2/-0 and loses flying until your next turn.",
        0,
    );
    let effects = parse_gain_ability_sentence(&tokens)
        .expect("pump-and-lose sentence should parse")
        .expect("pump-and-lose sentence should produce effects");

    let debug = format!("{effects:?}");
    assert!(
        string_contains(&debug, "Pump") && string_contains(&debug, "RemoveAbilitiesFromTarget"),
        "expected pump plus remove-ability effects, got {debug}"
    );
    assert!(
        debug.matches("YourNextTurn").count() >= 2,
        "expected shared duration to apply to both effects, got {debug}"
    );
}

#[test]
fn leading_duration_pump_and_keyword_chain_preserves_optional_target_count() {
    let tokens = tokenize_line(
        "Until end of turn, up to one target creature gets +2/+2 and gains vigilance and haste.",
        0,
    );
    let effects = parse_gain_ability_sentence(&tokens)
        .expect("optional-target pump-and-grant sentence should parse")
        .expect("optional-target pump-and-grant sentence should produce effects");

    let coordinated = sole_typed_coordination(&effects);
    let parsed_count = coordinated.effects().find_map(|effect| match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Pump {
                    target: TargetAst::WithCount(_, count),
                    ..
                },
            ..
        }) => Some(*count),
        _ => None,
    });
    assert_eq!(parsed_count, Some(ChoiceCount::up_to(1)), "{effects:#?}");

    let compiled = compile_statement_effects(&effects)
        .expect("optional-target pump-and-grant sentence should lower");

    fn contains_optional_target(effect: &crate::effect::Effect) -> bool {
        if effect
            .target_spec()
            .is_some_and(|target| target.count() == ChoiceCount::up_to(1))
        {
            return true;
        }
        let mut found = false;
        effect.visit_child_effects(&mut |child| {
            found |= contains_optional_target(child);
        });
        found
    }
    assert!(
        compiled.iter().any(contains_optional_target),
        "the authored optional target must survive lowering: {compiled:#?}"
    );
}

#[test]
fn pump_then_gain_is_preserved_as_one_coordinated_typed_clause() {
    let tokens = tokenize_line(
        "This creature gets +2/+2 and gains trample until end of turn.",
        0,
    );
    let effects = parse_gain_ability_sentence(&tokens)
        .expect("pump-and-grant sentence should parse")
        .expect("pump-and-grant sentence should produce effects");

    let coordinated = sole_typed_coordination(&effects);
    let debug = format!("{coordinated:#?}");
    assert!(debug.contains("Pump"), "{debug}");
    assert!(debug.contains("GrantAbilitiesToTarget"), "{debug}");
    assert!(debug.contains("Trample"), "{debug}");
}

#[test]
fn pump_then_serial_keyword_grant_keeps_every_keyword() {
    for (text, expected) in [
        (
            "Target creature gets +1/+1 and gains flying, first strike, and trample until end of turn.",
            &["Flying", "FirstStrike", "Trample"][..],
        ),
        (
            "Target creature you control gets +3/+3 and gains trample, hexproof, and indestructible until end of turn.",
            &["Trample", "Hexproof", "Indestructible"][..],
        ),
    ] {
        let tokens = tokenize_line(text, 0);
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("serial pump-and-grant sentence should parse")
            .expect("serial pump-and-grant sentence should produce effects");

        let debug = format!("{:#?}", sole_typed_coordination(&effects));
        for keyword in expected {
            assert!(
                debug.contains(keyword),
                "missing {keyword} for {text}: {debug}"
            );
        }
    }
}

#[test]
fn pump_and_grant_keep_the_object_noun_before_target_player_controller() {
    let tokens = tokenize_line(
        "Creatures target player controls get +2/+0 and gain haste until end of turn.",
        0,
    );
    let effects = parse_gain_ability_sentence(&tokens)
        .expect("controlled-creature pump and grant should parse")
        .expect("controlled-creature pump and grant should produce effects");
    let debug = format!("{effects:#?}");
    assert!(debug.contains("Creature"), "{debug}");
    assert!(debug.contains("Target"), "{debug}");
}

#[test]
fn shared_target_where_x_possessive_binds_only_the_bare_pronoun() {
    fn parsed_pump_power(text: &str) -> Value {
        let tokens = tokenize_line(text, 0);
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("shared target gain/pump sentence should parse")
            .expect("shared target gain/pump sentence should produce effects");
        sole_typed_coordination(&effects)
            .effects()
            .find_map(|effect| match effect {
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::Pump { power, .. },
                    ..
                }) => Some(power.clone()),
                _ => None,
            })
            .expect("shared target clause should retain its pump")
    }

    let target_relative = parsed_pump_power(
        "Target creature you control gains flying and gets +X/+X until end of turn, where X is its power.",
    );
    assert!(
        matches!(
            target_relative.unhinted(),
            Value::PowerOf(spec)
                if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
        ),
        "a bare possessive must use the shared target: {target_relative:#?}"
    );

    let source_relative = parsed_pump_power(
        "Another target creature you control gains trample and gets +X/+X until end of turn, where X is this creature's power.",
    );
    assert!(
        matches!(
            source_relative.unhinted(),
            Value::PowerOf(spec) if matches!(spec.base(), ChooseSpec::Source)
        ),
        "an explicit source reference must remain source-relative: {source_relative:#?}"
    );
}

#[test]
fn leading_become_lose_then_gain_keeps_the_trailing_keyword() {
    let tokens = tokenize_line(
        "Until end of turn, target creature you control becomes a blue Dragon Illusion with base power and toughness 4/4, loses all abilities, and gains flying.",
        0,
    );
    let effects = parse_gain_ability_sentence(&tokens)
        .expect("become-lose-gain sentence should parse")
        .expect("become-lose-gain sentence should produce effects");

    let coordinated = sole_typed_coordination(&effects);
    let debug = format!("{coordinated:#?}");
    assert!(debug.contains("BecomeBasePtCreature"), "{debug}");
    assert!(debug.contains("RemoveAbilitiesFromTarget"), "{debug}");
    assert!(debug.contains("GrantAbilitiesToTarget"), "{debug}");
    assert!(debug.contains("Flying"), "{debug}");
}

#[test]
fn base_pt_then_gains_keyword_in_single_clause_parses() {
    let tokens = tokenize_line(
        "This creature has base power and toughness 4/5 until end of turn and gains wither until end of turn.",
        0,
    );
    let effects = parse_gain_ability_sentence(&tokens)
        .expect("base-pt then gains clause should parse")
        .expect("base-pt then gains clause should produce effects");

    let debug = format!("{effects:?}").to_ascii_lowercase();
    assert!(
        string_contains(&debug, "setbasepowertoughness")
            && string_contains(&debug, "grantabilitiestotarget")
            && string_contains(&debug, "wither")
            && debug.matches("endofturn").count() >= 2,
        "expected shared self-targeted base P/T plus wither grant until EOT, got {debug}"
    );
}

#[test]
fn leading_duration_demonstrative_base_pt_then_gains_keyword_parses() {
    let tokens = tokenize_line(
        "Until end of turn, that creature has base power and toughness 4/4 and gains indestructible.",
        0,
    );
    let effects = parse_gain_ability_sentence(&tokens)
        .expect("leading-duration base-pt then gains clause should parse")
        .expect("leading-duration base-pt then gains clause should produce effects");

    let debug = format!("{effects:?}").to_ascii_lowercase();
    assert!(
        string_contains(&debug, "setbasepowertoughness")
            && string_contains(&debug, "grantabilitiestotarget")
            && string_contains(&debug, "indestructible")
            && string_contains(&debug, "controlflow")
            && debug.matches("endofturn").count() == 1,
        "expected demonstrative base P/T plus keyword grant until EOT, got {debug}"
    );
}

#[test]
fn gain_landwalk_until_next_upkeep_sentence_parses() {
    let tokens = tokenize_line(
        "Target non-Wall creature an opponent controls gains forestwalk until your next upkeep.",
        0,
    );
    let effects = parse_gain_ability_sentence(&tokens)
        .expect("gain-until-next-upkeep sentence should parse")
        .expect("gain-until-next-upkeep sentence should produce effects");

    let debug = format!("{effects:?}");
    assert!(
        string_contains(&debug, "GrantAbilitiesToTarget"),
        "expected target ability grant, got {debug}"
    );
    assert!(
        string_contains(&debug, "Landwalk(Subtype { subtype: Forest, snow: false })")
            && string_contains(&debug, "YourNextUpkeep"),
        "expected forestwalk grant to keep next-upkeep duration, got {debug}"
    );
}

#[test]
fn lexed_gain_landwalk_until_next_upkeep_sentence_parses() {
    let mut tokens = lex_line(
        "Target non-Wall creature an opponent controls gains forestwalk until your next upkeep.",
        0,
    )
    .expect("rewrite lexer should classify landwalk gain clause");
    for token in &mut tokens {
        token.lowercase_word();
    }
    let effects = parse_gain_ability_sentence(&tokens)
        .expect("lexed gain-until-next-upkeep sentence should parse")
        .expect("lexed gain-until-next-upkeep sentence should produce effects");

    let debug = format!("{effects:?}");
    assert!(
        string_contains(&debug, "GrantAbilitiesToTarget"),
        "expected target ability grant, got {debug}"
    );
    assert!(
        string_contains(&debug, "Landwalk(Subtype { subtype: Forest, snow: false })")
            && string_contains(&debug, "YourNextUpkeep"),
        "expected forestwalk grant to keep next-upkeep duration, got {debug}"
    );
}

#[test]
fn gain_haste_and_except_by_haste_with_trailing_where_clause_keeps_unblockable_grant() {
    let tokens = tokenize_line(
        "Up to X target creatures you control each gain haste until end of turn and can't be blocked this turn except by creatures with haste, where X is the number of Bobbleheads you control as you activate this ability.",
        0,
    );
    let effects = parse_gain_ability_sentence(&tokens)
        .expect("agility bobblehead-style grant clause should parse")
        .expect("agility bobblehead-style grant clause should produce effects");

    let debug = format!("{effects:?}").to_ascii_lowercase();
    assert!(
        string_contains(&debug, "haste")
            && string_contains(&debug, "can't be blocked except by creatures with haste"),
        "expected haste plus except-by-haste unblockable grant, got {debug}"
    );
}

#[test]
fn you_and_permanents_gain_hexproof_splits_player_and_permanent_grants() {
    let tokens = tokenize_line(
        "You and permanents you control gain hexproof until end of turn.",
        0,
    );
    let effects = parse_gain_ability_sentence(&tokens)
        .expect("mixed player/permanent grant should parse")
        .expect("mixed player/permanent grant should produce effects");

    let debug = format!("{effects:?}");
    assert!(
        string_contains(&debug, "Cant")
            && string_contains(&debug, "BeTargetedPlayerFrom")
            && string_contains(&debug, "GrantAbilitiesAll")
            && string_contains(&debug, "Hexproof"),
        "expected player hexproof restriction plus permanent hexproof grant, got {debug}"
    );
}

#[test]
fn you_gain_shroud_lowers_to_unscoped_player_target_restriction() {
    let tokens = tokenize_line("You gain shroud until end of turn.", 0);
    let effects = parse_gain_ability_sentence(&tokens)
        .expect("player shroud grant should parse")
        .expect("player shroud grant should produce effects");

    let debug = format!("{effects:?}");
    assert!(
        string_contains(&debug, "Cant")
            && string_contains(&debug, "BeTargetedPlayer")
            && !string_contains(&debug, "BeTargetedPlayerFrom"),
        "expected shroud to prevent all targeting of the player, got {debug}"
    );
}

#[test]
fn you_and_permanents_gain_hexproof_from_keeps_player_grant_opponent_scoped() {
    let tokens = tokenize_line(
        "You and permanents you control gain hexproof from blue and from black until end of turn.",
        0,
    );
    let effects = parse_gain_ability_sentence(&tokens)
        .expect("mixed player/permanent hexproof-from grant should parse")
        .expect("mixed player/permanent hexproof-from grant should produce effects");

    let debug = format!("{effects:?}");
    assert!(
        string_contains(&debug, "BeTargetedPlayerFrom")
            && string_contains(&debug, "Opponent")
            && string_contains(&debug, "GrantAbilitiesAll")
            && string_contains(&debug, "HexproofFrom"),
        "expected player hexproof-from restriction to apply only to opponents' sources plus permanent hexproof-from grant, got {debug}"
    );
}

#[test]
fn gain_ability_subject_ignores_also_before_gain() {
    let tokens = tokenize_line(
        "Permanents you control also gain indestructible until end of turn.",
        0,
    );
    let effects = parse_gain_ability_sentence(&tokens)
        .expect("also-gain sentence should parse")
        .expect("also-gain sentence should produce effects");

    let debug = format!("{effects:?}");
    assert!(
        string_contains(&debug, "GrantAbilitiesAll") && string_contains(&debug, "Indestructible"),
        "expected also to be ignored in the subject filter, got {debug}"
    );
}

#[test]
fn mass_ability_loss_keeps_spent_mana_condition_through_lowering() {
    let tokens = tokenize_line(
        "Creatures your opponents control lose flying until end of turn if {G} was spent to cast this spell.",
        0,
    );
    let effects = parse_gain_ability_sentence(&tokens)
        .expect("conditional mass ability loss should parse")
        .expect("conditional mass ability loss should produce effects");

    let [
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        },
    ] = effects.as_slice()
    else {
        panic!("expected conditional mass ability removal, got {effects:#?}");
    };
    assert!(matches!(
        predicate,
        PredicateAst::ManaSpentToCastThisSpellAtLeast {
            amount: 1,
            symbol: Some(crate::mana::ManaSymbol::Green),
        }
    ));
    assert!(matches!(
        if_true.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::RemoveAbilitiesAll { .. },
            ..
        })]
    ));
    assert!(if_false.is_empty());

    let compiled =
        compile_statement_effects(&effects).expect("conditional mass ability loss should lower");
    let debug = format!("{compiled:#?}");
    assert!(debug.contains("ManaSpentToCastThisSpellAtLeast"), "{debug}");
    assert!(debug.contains("Green"), "{debug}");
}

#[test]
fn bare_card_type_and_subtype_mass_loss_uses_union_filter() {
    let tokens = tokenize_line(
        "All creatures and Vehicles lose indestructible until end of turn.",
        0,
    );
    let effects = parse_gain_ability_sentence(&tokens)
        .expect("cross-kind mass ability loss should parse")
        .expect("cross-kind mass ability loss should produce effects");

    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::RemoveAbilitiesAll { filter, .. },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected one mass ability-removal AST, got {effects:#?}");
    };
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    assert!(
        filter
            .any_of
            .iter()
            .any(|branch| branch.card_types == [CardType::Creature]),
        "{filter:#?}"
    );
    assert!(
        filter
            .any_of
            .iter()
            .any(|branch| branch.subtypes == [crate::types::Subtype::Vehicle]),
        "{filter:#?}"
    );
}

#[test]
fn dawns_truce_gift_line_compiles_promised_and_not_promised_branches() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Dawn's Truce")
            .parse_text(
                "Gift a card (You may promise an opponent a gift as you cast this spell. If you do, they draw a card before its other effects.)\nYou and permanents you control gain hexproof until end of turn. If the gift was promised, permanents you control also gain indestructible until end of turn.",
            )
            .expect("Dawn's Truce gift text should parse");

    let debug = format!("{def:#?}");
    assert!(
        string_contains(&debug, "ThisSpellPaidLabel")
            && string_contains(&debug, "kind: Gift")
            && string_contains(&debug, "EmitGiftGiven")
            && string_contains(&debug, "Hexproof")
            && string_contains(&debug, "Indestructible"),
        "expected Gift condition, gift event, hexproof, and indestructible effects, got {debug}"
    );
}

#[test]
fn source_reference_simple_gain_clause_keeps_leading_duration_and_source_target() {
    let tokens = tokenize_line("Until end of turn, this creature gains flying.", 0);
    let effect = parse_simple_gain_ability_clause(&tokens)
        .expect("source-referenced simple gain clause should parse")
        .expect("source-referenced simple gain clause should produce an effect");

    let debug = format!("{effect:?}");
    assert!(
        string_contains(&debug, "GrantAbilitiesToTarget"),
        "expected a self-targeted temporary grant effect, got {debug}"
    );
    assert!(
        string_contains(&debug, "source: true"),
        "expected the simple gain clause to stay targeted on the source, got {debug}"
    );
    assert!(
        string_contains(&debug, "ThisPermanentType(\"this creature\")"),
        "expected the simple gain clause to preserve the source surface, got {debug}"
    );
    assert!(
        string_contains(&debug, "EndOfTurn"),
        "expected the leading duration to survive lowering, got {debug}"
    );
    assert!(
        !string_contains(&debug, "GrantAbilitiesAll"),
        "expected no broad battlefield-wide grant effect, got {debug}"
    );
}

#[test]
fn source_reference_simple_lose_clause_keeps_leading_duration_and_source_target() {
    let tokens = tokenize_line("Until end of turn, this creature loses defender.", 0);
    let effect = parse_simple_lose_ability_clause(&tokens)
        .expect("source-referenced simple lose clause should parse")
        .expect("source-referenced simple lose clause should produce an effect");

    let debug = format!("{effect:?}");
    assert!(
        string_contains(&debug, "RemoveAbilitiesFromTarget"),
        "expected a self-targeted temporary removal effect, got {debug}"
    );
    assert!(
        string_contains(&debug, "Source("),
        "expected the simple lose clause to stay targeted on the source, got {debug}"
    );
    assert!(
        string_contains(&debug, "EndOfTurn"),
        "expected the leading duration to survive lowering, got {debug}"
    );
    assert!(
        !string_contains(&debug, "RemoveAbilitiesAll"),
        "expected no broad battlefield-wide removal effect, got {debug}"
    );
}

#[test]
fn quoted_granted_trigger_keeps_all_sentences_inside_the_grant() {
    let tokens = tokenize_line(
        "Until end of turn, permanents your opponents control gain \"When this permanent deals damage to the player who cast Hellish Rebuke, sacrifice this permanent. You lose 2 life.\"",
        0,
    );
    let effects = parse_gain_ability_sentence(&tokens)
        .expect("quoted granted trigger should parse")
        .expect("quoted granted trigger should produce effects");

    assert_eq!(
        effects.len(),
        1,
        "quoted granted trigger should stay inside a single grant effect: {effects:?}"
    );

    let debug = format!("{effects:?}");
    assert!(
        string_contains(&debug, "GrantAbilitiesAll"),
        "expected a global grant effect, got {debug}"
    );
    assert!(
        string_contains(&debug, "ParsedObjectAbility"),
        "expected parsed granted ability payload, got {debug}"
    );
    assert!(
        string_contains(&debug, "LoseLife"),
        "expected lose-life text to remain inside the granted ability payload, got {debug}"
    );
}

#[test]
fn quoted_granted_trigger_keeps_trailing_if_otherwise_branch() {
    let tokens = tokenize_line(
        "Sliver creatures you control have \"When this creature enters, Slivers you control get +1/+1 until end of turn if you're the monarch. Otherwise, you become the monarch.\"",
        0,
    );
    let effects = parse_gain_ability_sentence(&tokens)
        .expect("quoted monarch trigger should parse")
        .expect("quoted monarch trigger should produce effects");

    let granted_abilities = effects
        .iter()
        .find_map(|effect| match effect {
            EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
                SubjectVerbActionAst::GrantAbilitiesAll { abilities, .. } => Some(abilities),
                _ => None,
            },
            _ => None,
        })
        .expect("expected global grant effect");
    let granted_trigger = granted_abilities
        .iter()
        .find_map(|ability| match ability {
            GrantedAbilityAst::ParsedObjectAbility { ability, .. } => Some(ability),
            _ => None,
        })
        .expect("expected parsed granted trigger");
    let trigger_effects = granted_trigger
        .effects_ast
        .as_ref()
        .expect("expected granted trigger effects");
    let false_branch = trigger_effects
        .iter()
        .find_map(|effect| match effect {
            EffectAst::Conditional {
                predicate: PredicateAst::PlayerIsMonarch { .. },
                if_false,
                ..
            } => Some(if_false),
            EffectAst::ControlFlow(control) => {
                let crate::model::control_flow::ControlFlowNodeAst::Condition {
                    condition,
                    alternative_program: Some(alternative),
                    ..
                } = &control.node
                else {
                    return None;
                };
                if !matches!(
                    &condition.predicate,
                    crate::model::control_flow::ControlPredicateAst::State(
                        PredicateAst::PlayerIsMonarch { .. }
                    )
                ) {
                    return None;
                }
                Some(
                    &control
                        .program(*alternative)
                        .expect("otherwise program")
                        .effects,
                )
            }
            _ => None,
        })
        .unwrap_or_else(|| {
            panic!("expected monarch conditional inside granted trigger: {granted_trigger:#?}")
        });
    assert!(
        false_branch.iter().any(|effect| matches!(
            effect,
            EffectAst::SubjectVerb(subject_verb)
                if matches!(subject_verb.action, SubjectVerbActionAst::BecomeMonarch)
        )),
        "expected otherwise branch to become the monarch"
    );
}

#[test]
fn hellish_rebuke_lowering_keeps_lose_life_inside_granted_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Hellish Rebuke")
            .parse_text(
                "Until end of turn, permanents your opponents control gain \"When this permanent deals damage to the player who cast Hellish Rebuke, sacrifice this permanent. You lose 2 life.\"",
            )
            .expect("hellish rebuke grant line should parse");

    let spell_effects = def
        .spell_effect
        .as_ref()
        .expect("hellish rebuke should compile to spell effects");
    assert_eq!(
        spell_effects.len(),
        1,
        "lose life should not be hoisted to a top-level spell effect: {spell_effects:?}"
    );

    let debug = format!("{spell_effects:?}");
    assert!(
        string_contains(&debug, "AddAbilityGeneric")
            && string_contains(&debug, "TriggeredAbility")
            && string_contains(&debug, "LoseLifeEffect")
            && (string_contains(&debug, "sacrifice_source")
                || (string_contains(&debug, "SacrificeTargetEffect")
                    && string_contains(&debug, "Source"))),
        "granted trigger should keep its inline trigger effects together, got {debug}"
    );
    assert!(
        string_contains(&debug, "this_deals_damage_to_player")
            || string_contains(&debug, "ThisDealsDamageTrigger"),
        "granted trigger should constrain damage-to-player semantics: {debug}"
    );
}

#[test]
fn counter_linked_leading_duration_keeps_quoted_trigger_as_a_grant() {
    for (text, counter_name) in [
        (
            "For as long as that land has a blaze counter on it, it has \"At the beginning of your upkeep, this land deals 1 damage to you.\"",
            "blaze",
        ),
        (
            "For as long as that creature has a bounty counter on it, it has \"When this creature dies, each opponent draws a card and gains 2 life.\"",
            "bounty",
        ),
    ] {
        let tokens = tokenize_line(text, 0);
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("counter-linked quoted grant should parse")
            .expect("counter-linked quoted grant should produce an effect");
        let debug = format!("{effects:#?}");
        let normalized_debug = debug.to_ascii_lowercase();

        assert!(debug.contains("GrantAbilitiesToTarget"), "{debug}");
        assert!(debug.contains("ParsedObjectAbility"), "{debug}");
        assert!(
            debug.contains("ForAsLongAs") && normalized_debug.contains(counter_name),
            "{debug}"
        );
    }
}

#[test]
fn mixed_keyword_and_quoted_trigger_grant_stays_targeted() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Strength of Will")
            .parse_text(
                "Until end of turn, target creature you control gains indestructible and \"Whenever this creature is dealt damage, put that many +1/+1 counters on it.\"",
            )
            .expect("strength of will grant line should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        string_contains(&debug, "TriggeredAbility"),
        "grant should keep the quoted triggered ability payload: {debug}"
    );

    let rendered = format!("{def:#?}").to_ascii_lowercase();
    let compact_rendered = rendered.split_whitespace().collect::<String>();
    assert!(
        (string_contains(&compact_rendered, "targetonlyeffect")
            || string_contains(&compact_rendered, "target_spec:some(target(object("))
            && string_contains(&compact_rendered, "controller:some(you")
            && string_contains(&compact_rendered, "addability")
            && string_contains(&compact_rendered, "indestructible")
            && string_contains(&compact_rendered, "addabilitygeneric")
            && string_contains(&compact_rendered, "isdealtdamage")
            && string_contains(&compact_rendered, "putcounterseffect"),
        "grant should stay targeted in the lowered structure: {rendered}"
    );
}

#[test]
fn players_gain_hexproof_clause_parses_as_player_wide_targeting_restriction() {
    let tokens = tokenize_line("Players gain hexproof until end of turn.", 0);
    let effect = parse_simple_gain_ability_clause(&tokens)
        .expect("players gain clause should parse")
        .expect("players gain clause should produce an effect");

    let debug = format!("{effect:?}");
    assert!(
        string_contains(&debug, "Cant")
            && string_contains(&debug, "BeTargetedPlayerFrom(Any")
            && string_contains(&debug, "EndOfTurn"),
        "expected a player-wide temporary targeting restriction, got {debug}"
    );
}

#[test]
fn lose_become_and_base_pt_chain_keeps_one_unmodified_subject() {
    let tokens = tokenize_line(
        "Each creature target opponent controls loses all abilities, becomes a Coward in addition to its other types, and has base power and toughness 1/1.",
        0,
    );
    let effects = parse_gain_ability_sentence(&tokens)
        .expect("shared-subject continuous chain should parse")
        .expect("shared-subject continuous chain should produce effects");
    let coordinated = sole_typed_coordination(&effects)
        .effects()
        .cloned()
        .collect::<Vec<_>>();
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::RemoveAbilitiesAll {
                    filter: remove,
                    set_quantifier_surface: remove_quantifier,
                    ..
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::AddSubtypes {
                    target: TargetAst::Object(add, ..),
                    subtypes,
                    ..
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::SetBasePowerToughness {
                    target: TargetAst::Object(set_pt, ..),
                    power: Value::Fixed(1),
                    toughness: Value::Fixed(1),
                    ..
                },
            ..
        }),
    ] = coordinated.as_slice()
    else {
        panic!("expected remove/add-subtype/set-P/T actions, got {coordinated:#?}");
    };
    assert_eq!(
        *remove_quantifier,
        Some(ironsmith_core::SetQuantifierSurface::Each)
    );

    for filter in [remove, add, set_pt] {
        assert_eq!(filter.card_types, [CardType::Creature], "{filter:#?}");
        assert_eq!(
            filter.controller,
            Some(PlayerFilter::Target(Box::new(PlayerFilter::Opponent))),
            "{filter:#?}"
        );
        assert!(filter.subtypes.is_empty(), "{filter:#?}");
        assert!(!filter.other, "{filter:#?}");
    }
    assert_eq!(subtypes, &[crate::types::Subtype::Coward]);
}

#[test]
fn sentence_dispatch_preserves_loss_become_and_base_pt_coordination() {
    let tokens = tokenize_line(
        "Each creature target opponent controls loses all abilities, becomes a Coward in addition to its other types, and has base power and toughness 1/1.",
        0,
    );
    let effects = parse_effect_sentence_lexed(&tokens)
        .expect("full sentence dispatch should preserve the coordinated chain");
    let coordinated = sole_typed_coordination(&effects)
        .effects()
        .collect::<Vec<_>>();
    assert!(
        matches!(
            coordinated.as_slice(),
            [
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::RemoveAbilitiesAll { .. },
                    ..
                }),
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::AddSubtypes { .. },
                    ..
                }),
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::SetBasePowerToughness { .. },
                    ..
                }),
            ]
        ),
        "full sentence route must not contaminate the subject filter: {effects:#?}"
    );
}

#[test]
fn target_controller_qualifier_does_not_hide_an_explicit_object_target() {
    let tokens = tokenize_line(
        "Target creature an opponent controls loses all abilities and has base power and toughness 1/1 until end of turn.",
        0,
    );
    let effects = parse_gain_ability_sentence(&tokens)
        .expect("targeted continuous chain should parse")
        .expect("targeted continuous chain should produce effects");
    let coordinated = sole_typed_coordination(&effects)
        .effects()
        .cloned()
        .collect::<Vec<_>>();
    assert!(
        matches!(
            coordinated.as_slice(),
            [
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::RemoveAbilitiesFromTarget { .. },
                    ..
                }),
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::SetBasePowerToughness { .. },
                    ..
                })
            ]
        ),
        "explicit object target must remain targeted: {coordinated:#?}"
    );
}

#[test]
fn plural_pronoun_grant_is_typed_without_pluralizing_singular_it() {
    let surface = |text: &str| {
        let tokens = tokenize_line(text, 0);
        let effects = parse_gain_ability_sentence(&tokens)
            .expect("pronoun grant should parse")
            .expect("pronoun grant should produce an effect");
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::GrantAbilitiesToTarget {
                        set_quantifier_surface,
                        ..
                    },
                ..
            }),
        ] = effects.as_slice()
        else {
            panic!("expected one typed target grant, got {effects:#?}");
        };
        *set_quantifier_surface
    };
    let simple_surface = |text: &str| {
        let tokens = tokenize_line(text, 0);
        let effect = parse_simple_gain_ability_clause(&tokens)
            .expect("simple pronoun grant should parse")
            .expect("simple pronoun grant should produce an effect");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantAbilitiesToTarget {
                    set_quantifier_surface,
                    ..
                },
            ..
        }) = effect
        else {
            panic!("expected one simple typed target grant, got {effect:#?}");
        };
        set_quantifier_surface
    };

    assert_eq!(
        surface("They gain haste until end of turn."),
        Some(ironsmith_core::SetQuantifierSurface::They)
    );
    assert_eq!(surface("It gains haste until end of turn."), None);
    assert_eq!(
        simple_surface("They gain haste until end of turn."),
        Some(ironsmith_core::SetQuantifierSurface::They)
    );
    assert_eq!(simple_surface("It gains haste until end of turn."), None);
}

#[test]
fn this_creature_keyword_grant_targets_only_the_ability_source() {
    let tokens = tokenize_line("This creature gains indestructible until end of turn.", 0);
    let effects = parse_gain_ability_sentence(&tokens)
        .expect("source keyword grant should parse")
        .expect("source keyword grant should produce an effect");
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantAbilitiesToTarget {
                    target: TargetAst::Object(source_filter, None, None),
                    abilities,
                    duration: Until::EndOfTurn,
                    ..
                },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("source grant must not widen to an unscoped object filter: {effects:#?}");
    };
    assert!(source_filter.source, "{source_filter:#?}");
    assert_eq!(
        source_filter.source_surface,
        Some(crate::target::SourceReferenceSurface::ThisPermanentType(
            "this creature".to_string()
        ))
    );
    assert_eq!(abilities, &[KeywordAction::Indestructible.into()]);
}
