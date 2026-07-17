use super::*;
use crate::cards::TextSpan;
use crate::effect::Value;
use crate::ids::CardId;
use crate::runtime_backend::RefState;
use crate::runtime_backend::lexer::lex_line;
use crate::target::ChooseSpec;
use crate::types::{CardType, Subtype};
use std::path::Path;

fn walk_rs_files(root: &Path, files: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(root).expect("read source directory") {
        let entry = entry.expect("read source entry");
        let path = entry.path();
        if path.is_dir() {
            walk_rs_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
}

#[test]
fn player_filter_resolution_stays_behind_subject_context() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = manifest_dir.join("src");
    let compile_support = manifest_dir
        .join("src/runtime_backend/lowering/compile_support.rs")
        .canonicalize()
        .expect("canonical compile_support.rs");
    let helper = manifest_dir
        .join("src/runtime_backend/lowering/compile_support/player_effect_helpers.rs")
        .canonicalize()
        .expect("canonical player_effect_helpers.rs");
    let needle = concat!("resolve_effect_player", "_filter(");

    let mut rs_files = Vec::new();
    walk_rs_files(&src, &mut rs_files);

    let mut unexpected = Vec::new();
    for path in rs_files {
        let canonical = path.canonicalize().expect("canonical source path");
        let source = std::fs::read_to_string(&path).expect("read source file");
        for (line_index, line) in source.lines().enumerate() {
            if !line.contains(needle) {
                continue;
            }
            let allowed = canonical == helper
                || canonical == compile_support
                    && (line.contains(concat!("fn resolve_effect_player", "_filter("))
                        || line.contains(needle) && line.contains("let needle = concat!("));
            if !allowed {
                unexpected.push(format!("{}:{}", path.display(), line_index + 1));
            }
        }
    }

    assert!(
        unexpected.is_empty(),
        "player filter resolution must go through LoweredSubject, found {unexpected:?}"
    );
}

#[test]
fn lowering_handlers_do_not_reach_into_lowered_subject_fields() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let compile_support_dir = manifest_dir.join("src/runtime_backend/lowering/compile_support");
    let compile_support_rs = manifest_dir.join("src/runtime_backend/lowering/compile_support.rs");
    let hidden_filter_field = concat!(".", "player_filter");
    let hidden_choices_field = concat!(".", "choices");

    let mut files = vec![compile_support_rs];
    for entry in std::fs::read_dir(&compile_support_dir).expect("read compile_support dir") {
        let path = entry.expect("read compile_support entry").path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !path.extension().is_some_and(|ext| ext == "rs") {
            continue;
        }
        if name == "player_effect_helpers.rs" || name == "choose_effect_helpers.rs" {
            continue;
        }
        files.push(path);
    }

    let mut unexpected = Vec::new();
    for path in files {
        let source = std::fs::read_to_string(&path).expect("read lowering source file");
        for (line_index, line) in source.lines().enumerate() {
            let reaches_filter_field =
                line.contains(hidden_filter_field) && !line.contains(".player_filter(");
            let reaches_choices_field =
                line.contains(hidden_choices_field) && !line.contains(".choices()");
            if line.contains("subject.") && (reaches_filter_field || reaches_choices_field) {
                unexpected.push(format!("{}:{}", path.display(), line_index + 1));
            }
        }
    }

    assert!(
        unexpected.is_empty(),
        "lowering handlers must use LoweredSubject methods, found {unexpected:?}"
    );
}

#[test]
fn compile_investigate_uses_ast_count() {
    let mut ctx = EffectLoweringContext::new();
    let (effects, choices) = compile_effect(
        &EffectAst::subject_verb_investigate(
            crate::cards::builders::PlayerAst::Implicit,
            Value::Fixed(2),
        ),
        &mut ctx,
    )
    .expect("compile investigate");

    assert!(choices.is_empty());
    assert_eq!(effects.len(), 1);
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("InvestigateEffect"),
        "investigate effect: {debug}"
    );
    assert!(
        debug.contains("count: Fixed(2)"),
        "investigate count: {debug}"
    );
    assert!(debug.contains("player: You"), "investigate player: {debug}");
}

#[test]
fn move_to_zone_lowering_preserves_oracle_verb_and_explicit_actor() {
    let move_ast = |verb_surface, player| {
        let EffectAst::SubjectVerb(mut subject_verb) = EffectAst::subject_verb_move_to_zone(
            TargetAst::Source(None),
            crate::zone::Zone::Hand,
            false,
            crate::cards::builders::ReturnControllerAst::Preserve,
            false,
            None,
        )
        .with_move_to_zone_verb_surface(verb_surface) else {
            unreachable!("move constructor must produce a subject-verb AST")
        };
        subject_verb.subject.player = player;
        EffectAst::SubjectVerb(subject_verb)
    };

    for (surface, player, expected_actor) in [
        (
            ironsmith_core::MoveToZoneVerbSurface::Put,
            PlayerAst::Opponent,
            PlayerFilter::Opponent,
        ),
        (
            ironsmith_core::MoveToZoneVerbSurface::Return,
            PlayerAst::You,
            PlayerFilter::You,
        ),
    ] {
        let (effects, choices) = compile_effect(
            &move_ast(surface, player),
            &mut EffectLoweringContext::new(),
        )
        .expect("typed move-to-zone AST should lower");
        assert!(choices.is_empty());
        let lowered = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<crate::effects::MoveToZoneEffect>())
            .expect("move-to-zone effect should remain typed");
        assert_eq!(lowered.verb_surface, surface);
        assert_eq!(lowered.actor_surface, Some(expected_actor));
    }
}

#[test]
fn source_top_only_zone_actions_choose_the_ordered_card_before_moving_it() {
    let source_filter = ObjectFilter::creature()
        .in_zone(crate::zone::Zone::Graveyard)
        .owned_by(PlayerFilter::You);

    let exile_ast =
        EffectAst::subject_verb_exile(TargetAst::Object(source_filter.clone(), None, None), true)
            .with_source_top_only(true);
    let (exile_effects, exile_choices) =
        compile_effect(&exile_ast, &mut EffectLoweringContext::new())
            .expect("top graveyard exile should lower");
    assert!(exile_choices.is_empty());
    assert_eq!(exile_effects.len(), 2);
    let exile_choose = exile_effects[0]
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()
        .expect("ordered exile should begin with a typed choice");
    assert!(exile_choose.top_only);
    assert_eq!(exile_choose.filter, source_filter);
    let exile = exile_effects[1]
        .downcast_ref::<crate::effects::ExileEffect>()
        .expect("ordered exile should consume the chosen tag");
    assert_eq!(exile.spec, ChooseSpec::tagged(exile_choose.tag.clone()));
    assert!(exile.face_down);

    let move_ast = EffectAst::subject_verb_move_to_zone(
        TargetAst::Object(source_filter.clone(), None, None),
        crate::zone::Zone::Hand,
        false,
        crate::cards::builders::ReturnControllerAst::Preserve,
        false,
        None,
    )
    .with_source_top_only(true);
    let (move_effects, move_choices) = compile_effect(&move_ast, &mut EffectLoweringContext::new())
        .expect("top graveyard move should lower");
    assert!(move_choices.is_empty());
    assert_eq!(move_effects.len(), 2);
    let move_choose = move_effects[0]
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()
        .expect("ordered move should begin with a typed choice");
    assert!(move_choose.top_only);
    assert_eq!(move_choose.filter, source_filter);
    let move_effect = move_effects[1]
        .as_tagged()
        .map(|tagged| tagged.effect.as_ref())
        .unwrap_or(&move_effects[1])
        .downcast_ref::<crate::effects::MoveToZoneEffect>()
        .expect("ordered move should consume the chosen tag");
    assert_eq!(
        move_effect.target,
        ChooseSpec::tagged(move_choose.tag.clone())
    );
    assert_eq!(move_effect.zone, crate::zone::Zone::Hand);
}

#[test]
fn source_top_only_lowering_preserves_count_value_and_scoped_library_default() {
    let target = TargetAst::WithCountValue(
        Box::new(TargetAst::Object(ObjectFilter::default(), None, None)),
        ChoiceCount::dynamic_x(),
        Value::Fixed(4),
    );
    let ast = EffectAst::subject_verb_exile(target, false).with_source_top_only(true);
    let mut ctx = EffectLoweringContext::new();
    let (effects, choices) = compile_effect(&ast, &mut ctx)
        .expect("a lexically proven bare top-card source should default to the library");

    assert!(choices.is_empty());
    assert_eq!(effects.len(), 2);
    let choose = effects[0]
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()
        .expect("top-card collection should begin with an ordered choice");
    assert_eq!(choose.filter.zone, Some(crate::zone::Zone::Library));
    assert_eq!(choose.count, ChoiceCount::dynamic_x());
    assert_eq!(choose.count_value, Some(Value::Fixed(4)));
    assert!(choose.top_only);
    assert!(ctx.last_exiled_collection_is_plural);

    let battlefield_target = TargetAst::Object(
        ObjectFilter::default().in_zone(crate::zone::Zone::Battlefield),
        None,
        None,
    );
    let battlefield_ast =
        EffectAst::subject_verb_exile(battlefield_target, false).with_source_top_only(true);
    let err = compile_effect(&battlefield_ast, &mut EffectLoweringContext::new())
        .expect_err("top-only must not reinterpret an explicitly unordered source");
    assert!(
        err.to_string()
            .contains("ordered graveyard or library source"),
        "unexpected top-only source error: {err:?}"
    );
}

#[test]
fn explicit_controller_return_lowering_retains_return_surface() {
    let (effects, choices) = compile_effect(
        &EffectAst::subject_verb_return_to_battlefield(
            TargetAst::Tagged(TagKey::from("triggering"), None),
            false,
            false,
            false,
            crate::cards::builders::ReturnControllerAst::You,
            None,
        ),
        &mut EffectLoweringContext::new(),
    )
    .expect("explicit-controller return should lower");

    assert!(choices.is_empty());
    let lowered = effects
        .iter()
        .find_map(|effect| {
            effect
                .downcast_ref::<crate::effects::TaggedEffect>()
                .and_then(|tagged| {
                    tagged
                        .effect
                        .downcast_ref::<crate::effects::MoveToZoneEffect>()
                })
                .or_else(|| effect.downcast_ref::<crate::effects::MoveToZoneEffect>())
        })
        .expect("explicit-controller return should lower to a move-to-zone effect");
    assert_eq!(
        lowered.verb_surface,
        ironsmith_core::MoveToZoneVerbSurface::Return
    );
    assert_eq!(
        lowered.battlefield_controller,
        crate::effects::BattlefieldController::You
    );
}

#[test]
fn parse_text_investigate_twice_compiles_to_count_two() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Investigate Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Investigate twice.")
        .expect("parse investigate twice");

    let effects = def.spell_effect.as_ref().expect("spell effects");
    assert_eq!(effects.len(), 1);
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("InvestigateEffect"),
        "investigate effect: {debug}"
    );
    assert!(
        debug.contains("count: Fixed(2)"),
        "investigate count: {debug}"
    );
    assert!(debug.contains("player: You"), "investigate player: {debug}");
}

#[test]
fn reveal_consult_exports_full_collection_for_later_cleanup() {
    for (card_type, text, order, reflexive) in [
        (
            CardType::Creature,
            "{2}{R}: Reveal cards from the top of your library until you reveal a nonland card. This creature gets +X/+0 until end of turn, where X is that card's mana value. Put the revealed cards on the bottom of your library in any order.",
            "ChooserChooses",
            false,
        ),
        (
            CardType::Instant,
            "Reveal cards from the top of your library until you reveal a nonland card. Put the revealed cards on the bottom of your library in a random order. When you reveal a nonland card this way, this deals damage equal to that card's mana value to any target.",
            "Random",
            true,
        ),
    ] {
        let def = CardDefinitionBuilder::new(CardId::new(), "Consult Cleanup Probe")
            .card_types(vec![card_type])
            .parse_text(text)
            .expect("consult followed by full revealed-set cleanup should lower");

        let debug = format!("{def:#?}");
        assert!(debug.contains("ConsultTopOfLibraryEffect"), "{debug}");
        assert!(
            debug.contains("PutTaggedRemainderOnLibraryBottomEffect"),
            "{debug}"
        );
        assert!(debug.contains(order), "{debug}");
        if reflexive {
            assert!(debug.contains("ReflexiveTriggerEffect"), "{debug}");
            assert!(debug.contains("DealDamageEffect"), "{debug}");
            assert!(debug.contains("ManaValueOf"), "{debug}");
            assert!(debug.contains("AnyTarget"), "{debug}");
        }
    }
}

#[test]
fn compile_amass_tags_output_when_followup_references_it() {
    let mut ctx = EffectLoweringContext::new();
    ctx.auto_tag_object_targets = true;

    let (effects, choices) = compile_effect(
        &EffectAst::subject_verb_amass(Some(Subtype::Orc), Value::Fixed(2)),
        &mut ctx,
    )
    .expect("compile amass");

    assert!(choices.is_empty());
    assert_eq!(effects.len(), 1);

    let debug = format!("{effects:?}");
    assert!(debug.contains("TaggedEffect"), "amass tagging: {debug}");
    assert!(debug.contains("amassed_0"), "amass tag: {debug}");
    assert!(debug.contains("AmassEffect"), "amass effect: {debug}");
    assert!(
        debug.contains("subtype: Some(Orc)"),
        "amass subtype: {debug}"
    );
    assert!(debug.contains("amount: Fixed(2)"), "amass amount: {debug}");
    assert_eq!(ctx.last_object_tag.as_deref(), Some("amassed_0"));
}

#[test]
fn coordinated_equal_target_specs_keep_distinct_lowered_target_slots() {
    let repeated_target = TargetAst::WithCount(
        Box::new(TargetAst::Object(
            ObjectFilter::creature().other(),
            None,
            None,
        )),
        ChoiceCount::up_to(1),
    );
    let coordinated = EffectAst::Coordinated {
        effects: vec![
            EffectAst::subject_verb_pump(
                Value::Fixed(-3),
                Value::Fixed(0),
                repeated_target.clone(),
                Until::YourNextTurn,
                None,
            ),
            EffectAst::subject_verb_pump(
                Value::Fixed(-2),
                Value::Fixed(0),
                repeated_target.clone(),
                Until::YourNextTurn,
                None,
            ),
            EffectAst::subject_verb_pump(
                Value::Fixed(-1),
                Value::Fixed(0),
                repeated_target,
                Until::YourNextTurn,
                None,
            ),
        ],
        leading_duration: false,
        result_conjunction: false,
    };
    let mut ctx = EffectLoweringContext::new();
    ctx.auto_tag_object_targets = true;

    // Exercise the enclosing annotated-sequence boundary used by production
    // card compilation. Calling `compile_effect` directly misses the choice
    // merge that previously collapsed the two equal optional target slots.
    let (effects, choices) = compile_effects(std::slice::from_ref(&coordinated), &mut ctx)
        .expect("compile coordination");

    assert_eq!(
        choices.len(),
        3,
        "equal-looking target words are three choices"
    );
    assert_eq!(choices[0], choices[1]);
    assert_eq!(choices[1], choices[2]);
    let sequence = effects
        .first()
        .and_then(|effect| effect.downcast_ref::<crate::effects::SequenceEffect>())
        .expect("coordinated lowering should produce a sequence");
    assert_eq!(sequence.effects.len(), 3, "no synthetic TargetOnly prelude");
    let tags = sequence
        .effects
        .iter()
        .map(|effect| {
            let tagged = effect
                .as_tagged()
                .expect("each target introduction is tagged");
            assert!(tagged.effect.as_apply_continuous().is_some());
            tagged.tag.clone()
        })
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(
        tags.len(),
        3,
        "each target choice has its own runtime identity"
    );

    let mut result_conjunction = coordinated;
    let EffectAst::Coordinated {
        result_conjunction: marker,
        ..
    } = &mut result_conjunction
    else {
        unreachable!("fixture is coordinated")
    };
    *marker = true;
    let (result_effects, _) = compile_effects(
        std::slice::from_ref(&result_conjunction),
        &mut EffectLoweringContext::new(),
    )
    .expect("compile grammar-confirmed result conjunction");
    let result_sequence = result_effects[0]
        .downcast_ref::<crate::effects::SequenceEffect>()
        .expect("result conjunction should lower to a sequence");
    assert_eq!(
        result_sequence.surface,
        ironsmith_core::SequenceSurface::ResultConjunction {
            leading_duration: false
        }
    );
}

#[test]
fn compile_damage_equal_to_power_over_each_object_fans_out_per_object() {
    let (effects, choices) = compile_effect(
        &EffectAst::subject_verb_damage_equal_to_power(
            TargetAst::Tagged(TagKey::from("amassed_0"), None),
            TargetAst::Object(
                ObjectFilter::creature().without_subtype(Subtype::Army),
                None,
                None,
            ),
        ),
        &mut EffectLoweringContext::new(),
    )
    .expect("compile power-based fanout damage");

    assert!(choices.is_empty());
    assert_eq!(effects.len(), 1);

    let debug = format!("{effects:?}");
    assert!(debug.contains("ForEachObject"), "fan-out wrapper: {debug}");
    assert!(
        debug.contains("Creature"),
        "fan-out creature filter: {debug}"
    );
    assert!(debug.contains("Army"), "fan-out excluded subtype: {debug}");
    assert!(
        debug.contains("ExecuteWithSourceEffect"),
        "fan-out source wrapper: {debug}"
    );
    assert!(debug.contains("amassed_0"), "fan-out source tag: {debug}");
    assert!(
        debug.contains("PowerOf(Tagged(TagKey(\"amassed_0\")))"),
        "fan-out damage amount: {debug}"
    );
    assert!(
        debug.contains("target: Iterated"),
        "fan-out iterated target: {debug}"
    );
}

#[test]
fn parse_text_gargoyle_sentinel_keeps_the_activation_on_self() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Gargoyle Sentinel")
        .parse_text(
            "Mana cost: {3}\n\
             Type: Artifact Creature — Gargoyle\n\
             Power/Toughness: 3/3\n\
             Defender (This creature can't attack.)\n\
             {3}: Until end of turn, this creature loses defender and gains flying.",
        )
        .expect("Gargoyle Sentinel text should parse");

    let _activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("expected Gargoyle Sentinel to have an activated ability");
    let debug = format!("{def:?}");
    assert!(
        debug.matches("ApplyContinuousEffect").count() >= 2,
        "expected two source-scoped continuous effects, got {debug}"
    );
    assert!(
        debug.matches("target_spec: Some(Source)").count() >= 2
            && debug.matches("until: EndOfTurn").count() >= 2,
        "expected the lowered activation to stay source-targeted until end of turn, got {debug}"
    );
    assert!(
        !debug.contains("GrantAbilitiesAll") && !debug.contains("RemoveAbilitiesAll"),
        "expected no broad battlefield-wide ability changes in the lowered definition, got {debug}"
    );
}

#[test]
fn parse_equipment_rules_text_keeps_single_quoted_activated_grant() {
    let source_text = "Colorless Equipment artifact token named Rock with \"Equipped creature has '{1}, {T}, Sacrifice Rock: This creature deals 2 damage to any target'\" and equip {1}.";

    let source_tokens = lex_line(source_text, 0).expect("equipment rules fixture should lex");
    let rules_text = token_grammar::parse_equipment_rules_tokens(&source_tokens)
        .expect("equipment rules shape")
        .text;

    assert!(
        rules_text.contains("Equipped creature has \"{1}, {T}, Sacrifice Rock: This creature deals 2 damage to any target.\"")
            && rules_text.contains("Equip {1}"),
        "expected quoted activated ability plus equip line, got {rules_text}"
    );
}

#[test]
fn typed_equipment_token_rules_lower_into_grant_and_equip() {
    let source_text = "colorless Equipment artifact token named Rock with \"Equipped creature has '{1}, {T}, Sacrifice Rock: This creature deals 2 damage to any target'\" and equip {1}.";
    let shape = token_grammar::parse_token_definition_shape_text(source_text)
        .expect("equipment token should have a typed definition");
    let def = lower_token_definition_shape(shape)
        .expect("typed equipment definition should lower without parsing display text");

    let activated_costs = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated.mana_cost.display()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        activated_costs.iter().any(|text| text.contains("{1}")),
        "expected typed equipment token to keep equip cost, got {activated_costs:?}"
    );
    assert!(
        format!("{def:?}").contains("AttachedAbilityGrant"),
        "expected typed equipment token to keep a structured granted ability, got {def:#?}"
    );
}

#[test]
fn typed_equipment_token_rules_lower_counter_scaled_grant() {
    let source_text = "colorless Book Equipment artifact token named Guide with \"Equipped creature gets +1/+1 for each quest counter among permanents you control\" and equip {1}.";
    let shape = token_grammar::parse_token_definition_shape_text(source_text)
        .expect("scaled equipment token should have a typed definition");
    let def = lower_token_definition_shape(shape)
        .expect("scaled equipment token definition should lower");
    let debug = format!("{def:#?}");

    assert!(
        debug.contains("CountersAmong") && debug.contains("Quest") && debug.contains("PerCount"),
        "expected counter-scaled attached anthem, got {debug}"
    );
}

#[test]
fn typed_token_damage_rules_lower_recipient_specific_triggers() {
    let poison = token_definition_for(
        "1/1 colorless Snake artifact creature token with \"Whenever this creature deals damage to a player, that player gets a poison counter.\"",
    )
    .expect("poison token definition");
    let poison_debug = format!("{poison:#?}");
    assert!(
        poison_debug.contains("ThisDealsDamageToPlayer")
            && !poison_debug.contains("ThisDealsCombatDamageToPlayer")
            && poison_debug.contains("PoisonCountersEffect")
            && poison_debug.contains("DamagedPlayer"),
        "expected noncombat poison trigger, got {poison_debug}"
    );

    let destroy = token_definition_for(
        "1/1 black Assassin creature token with \"Whenever this token deals damage to a planeswalker, destroy that planeswalker.\"",
    )
    .expect("planeswalker-destroy token definition");
    let destroy_debug = format!("{destroy:#?}");
    assert!(
        destroy_debug.contains("ThisDealsDamageTo")
            && destroy_debug.contains("Planeswalker")
            && destroy_debug.contains("TagTriggeringDamageTargetEffect")
            && destroy_debug.contains("DestroyEffect"),
        "expected damaged-planeswalker destroy trigger, got {destroy_debug}"
    );
}

#[test]
fn typed_multisentence_token_rules_lower_fallback_and_mana_life_programs() {
    let demon = token_definition_for(
        "6/6 black Demon creature token with \"At the beginning of your upkeep, sacrifice another creature. If you can't, this token deals 6 damage to you.\"",
    )
    .expect("upkeep Demon token definition");
    let demon_debug = format!("{demon:#?}");
    assert!(
        demon_debug.contains("BeginningOfUpkeep")
            && demon_debug.contains("SacrificePlayerEffect")
            && demon_debug.contains("DidNotHappen")
            && demon_debug.contains("DealDamageEffect"),
        "expected upkeep sacrifice fallback program, got {demon_debug}"
    );

    let banana = token_definition_for(
        "colorless artifact token named Banana with \"{T}, Sacrifice this token: Add {R} or {G}. You gain 2 life.\"",
    )
    .expect("Banana-style artifact token definition");
    let banana_debug = format!("{banana:#?}");
    let banana_activated = banana
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Banana token should have an activated ability");
    assert!(
        matches!(
            banana_activated.mana_cost.costs(),
            [crate::costs::Cost::Tap, crate::costs::Cost::SacrificeSelf]
        ) && banana_debug.contains("AddManaOfAnyColorEffect")
            && banana_debug.contains("GainLifeEffect"),
        "expected tap-sacrifice mana/life ability, got {banana_debug}"
    );
}

#[test]
fn inline_dynamic_token_power_toughness_lowers_from_typed_creation_fact() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dynamic Token Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Exile target creature card from your graveyard. Create a black Zombie creature token with power equal to that card's power and toughness equal to that card's toughness.",
        )
        .expect("inline dynamic token P/T should parse");
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("SetBasePowerToughnessEffect")
            && debug.contains("PowerOf")
            && debug.contains("ToughnessOf"),
        "expected the created token to inherit the exiled card's P/T, got {debug}"
    );
}

#[test]
fn token_definition_lowers_quoted_triggered_rules_text() {
    let source_text = "2/2 black Alien Angel artifact creature token with first strike, vigilance, and \"Whenever an opponent casts a creature spell, this token isn't a creature until end of turn.\"";

    let def = token_definition_for(source_text).expect("quoted token trigger should build a token");
    let debug = format!("{def:#?}");

    assert!(
        debug.contains("SpellCast") && debug.contains("caster: Opponent"),
        "expected quoted trigger to lower into a spell-cast trigger, got {debug}"
    );
    assert!(
        debug.contains("RemoveCardTypes"),
        "expected quoted trigger to compile into a real remove-card-types effect, got {debug}"
    );
}

#[test]
fn token_definition_lowers_quoted_cumulative_upkeep_keyword() {
    let source_text = "1/1 green Splinter creature token with flying and \"Cumulative upkeep {G}\"";

    let def =
        token_definition_for(source_text).expect("quoted cumulative upkeep should build token");
    let debug = format!("{def:#?}");

    assert!(
        debug.contains("CumulativeUpkeepEffect"),
        "expected quoted cumulative upkeep to lower into a real effect, got {debug}"
    );
    assert!(
        !debug.contains("label: \"Cumulative upkeep {G}\""),
        "quoted cumulative upkeep should not remain a keyword marker, got {debug}"
    );
}

#[test]
fn token_definition_lowers_quoted_unblockable_keyword() {
    let source_text = "1/1 blue Fish creature token with \"This token can't be blocked.\"";

    let def = token_definition_for(source_text)
        .expect("quoted unblockable token text should build token");
    let debug = format!("{def:#?}");

    assert!(
        debug.contains("Unblockable"),
        "expected quoted unblockable token text to lower into a static ability, got {debug}"
    );
}

#[test]
fn token_definition_lowers_unquoted_triggered_rules_tail() {
    let source_text = "2/2 black Alien Angel artifact creature token with first strike, vigilance, and Whenever an opponent casts a creature spell, this token isn't a creature until end of turn.";

    let def = token_definition_for(source_text)
        .expect("unquoted preserved trigger tail should still build a token");
    let debug = format!("{def:#?}");

    assert!(
        debug.contains("SpellCast") && debug.contains("caster: Opponent"),
        "expected inline trigger tail to lower into a spell-cast trigger, got {debug}"
    );
    assert!(
        debug.contains("RemoveCardTypes"),
        "expected inline trigger tail to compile into a real remove-card-types effect, got {debug}"
    );
}

#[test]
fn token_definition_named_construct_skips_urza_construct_shell() {
    let source_text =
        "0/0 colorless Construct artifact creature token named Twin that's attacking.";

    let def = token_definition_for(source_text).expect("named construct token should still build");
    let debug = format!("{def:#?}");

    assert_eq!(def.card.name, "Twin");
    assert!(
        !debug.contains("CharacteristicDefiningPT"),
        "named Construct tokens should not pick up the generic artifact-count CDA shell, got {debug}"
    );
}

#[test]
fn typed_token_rules_shape_lowers_blink_trigger() {
    let source_text = "2/2 black Alien Angel artifact creature token with first strike, vigilance, and \"Whenever an opponent casts a creature spell, this token isn't a creature until end of turn.\"";
    let shape = token_grammar::parse_token_definition_shape_text(source_text)
        .expect("quoted trigger should have a typed token shape");
    let parsed = lower_token_definition_shape(shape)
        .expect("typed quoted trigger should lower without reparsing text");
    let debug = format!("{parsed:#?}");

    assert!(
        debug.contains("RemoveCardTypes"),
        "expected generic quoted-token parse to compile remove-card-types, got {debug}"
    );
    assert!(
        debug.contains("until: EndOfTurn"),
        "expected generic quoted-token parse to keep until-end-of-turn duration, got {debug}"
    );
}

#[test]
fn typed_token_rules_shape_lowers_static_pt_ability() {
    let source_text = "green and white Elemental creature token with \"This token's power and toughness are each equal to the number of creatures you control.\"";
    let shape = token_grammar::parse_token_definition_shape_text(source_text)
        .expect("quoted static P/T should have a typed token shape");
    let parsed = lower_token_definition_shape(shape)
        .expect("typed quoted static P/T should lower without reparsing text");
    let debug = format!("{parsed:#?}");

    assert!(
        debug.contains("CharacteristicDefiningPT")
            && debug.contains("card_types: [\n                                    Creature"),
        "expected generic quoted-token parse to compile dynamic creature-count P/T, got {debug}"
    );
}

#[test]
fn inline_quoted_token_trigger_stays_on_the_created_token() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Token Trigger Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Create a 2/2 red Goblin Shaman creature token with \"Whenever this token attacks, create a Treasure token.\"",
        )
        .expect("inline quoted token trigger should parse");
    let create = def
        .spell_effect
        .as_ref()
        .expect("spell effects")
        .iter()
        .find_map(|effect| effect.as_create_token())
        .expect("created Goblin token");
    let debug = format!("{:#?}", create.token.abilities);
    assert!(
        debug.contains("Triggered")
            && debug.contains("Attacks")
            && debug.contains("CreateTokenEffect"),
        "quoted attack trigger should be nested in the Goblin definition: {debug}"
    );
}

#[test]
fn full_triggered_token_creation_keeps_quoted_blocks_untap_ability() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Quoted Blocking Token Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, create two 0/2 blue Illusion creature tokens with \"Whenever this token blocks a creature, that creature doesn't untap during its controller's next untap step.\"\nThis creature has hexproof as long as you control an Illusion.",
        )
        .expect("full quoted blocking-token document should parse");
    let create = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .effects
                .flattened_default_effects()
                .iter()
                .find_map(|effect| effect.as_create_token()),
            _ => None,
        })
        .expect("entry trigger should create Illusion tokens");

    assert_eq!(create.count, Value::Fixed(2));
    let nested = create
        .token
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("created Illusions should carry the quoted triggered ability");
    assert!(matches!(
        &nested.trigger.kind,
        crate::triggers::TriggerKind::ThisBlocksObject { filter }
            if filter.card_types.as_slice() == [CardType::Creature]
    ));

    let [effect] = nested.effects.flattened_default_effects() else {
        panic!("expected one nested untap restriction: {nested:#?}");
    };
    let cant = effect
        .downcast_ref::<crate::effects::CantEffect>()
        .expect("nested trigger should apply an untap restriction");
    assert!(matches!(
        &cant.restriction,
        crate::effect::Restriction::Untap(_)
    ));
    assert_eq!(
        cant.duration,
        crate::effect::Until::ControllersNextUntapStep
    );
}

#[test]
fn pest_token_attack_trigger_keeps_its_life_gain() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Pest Trigger Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Create a 1/1 black and green Pest creature token with \"Whenever this token attacks, you gain 1 life.\"",
        )
        .expect("Pest attack trigger should parse");
    let create = def
        .spell_effect
        .as_ref()
        .expect("spell effects")
        .iter()
        .find_map(|effect| effect.as_create_token())
        .expect("created Pest token");
    let debug = format!("{:#?}", create.token.abilities);
    assert!(
        debug.contains("Triggered")
            && debug.contains("Attacks")
            && debug.contains("GainLifeEffect"),
        "the quoted attack/life ability should remain on the Pest: {debug}"
    );
}

#[test]
fn full_send_in_the_pest_keeps_nested_token_attack_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Send in the Pest")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Each opponent discards a card. You create a 1/1 black and green Pest creature token with \"Whenever this token attacks, you gain 1 life.\"",
        )
        .expect("full Send in the Pest oracle text should parse");
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("CreateTokenEffect")
            && debug.contains("Triggered")
            && debug.contains("Attacks")
            && debug.contains("GainLifeEffect"),
        "full-card production dispatch must retain the Pest ability: {debug}"
    );
}

#[test]
fn create_token_modifiers_do_not_leak_from_for_each_filter() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Counted Tapped Permanents Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Create a Treasure token for each tapped Assassin, Pirate, and/or Vehicle you control.",
        )
        .expect("dynamic Treasure creation should parse");
    let create = def
        .spell_effect
        .as_ref()
        .expect("spell effects")
        .iter()
        .find_map(|effect| effect.as_create_token())
        .expect("created Treasure token");

    assert!(
        !create.enters_tapped,
        "the counted permanents are tapped, not the created Treasure: {create:#?}"
    );
}

#[test]
fn token_self_attack_requirement_lowers_as_must_attack() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Alien Rules Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Create a 1/1 red Alien creature token with haste and \"This token attacks each combat if able.\"",
        )
        .expect("Alien attack requirement should parse");
    let create = def
        .spell_effect
        .as_ref()
        .expect("spell effects")
        .iter()
        .find_map(|effect| effect.as_create_token())
        .expect("created Alien token");
    let debug = format!("{:#?}", create.token.abilities);
    assert!(
        debug.contains("MustAttack"),
        "the quoted attack requirement should remain on the Alien: {debug}"
    );
}

#[test]
fn full_alien_invasion_keeps_nested_token_attack_requirement() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Alien Invasion")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "At the beginning of combat on your turn, create a 1/1 red Alien creature token with haste and \"This token attacks each combat if able.\" Put a +1/+1 counter on it for each invasion counter on this enchantment, then put an invasion counter on this enchantment.",
        )
        .expect("full Alien Invasion oracle text should parse");
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("CreateTokenEffect") && debug.contains("MustAttack"),
        "full-card production dispatch must retain the Alien attack requirement: {debug}"
    );
}

#[test]
fn token_pronoun_followup_parses_under_the_token_source_identity() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Token CDA Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Create a white Avatar creature token. It has \"This token's power and toughness are each equal to your life total.\"",
        )
        .expect("token CDA follow-up should parse");
    let create = def
        .spell_effect
        .as_ref()
        .expect("spell effects")
        .iter()
        .find_map(|effect| effect.as_create_token())
        .expect("created Avatar token");
    let debug = format!("{:#?}", create.token.abilities);
    assert!(
        debug.contains("CharacteristicDefiningPT") && debug.contains("LifeTotal"),
        "the Avatar CDA should belong to the created token: {debug}"
    );
}

#[test]
fn token_pronoun_activation_resolves_this_token_to_source() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Regeneration Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Create a 1/1 black Skeleton creature token. It has \"{B}: Regenerate this token.\"",
        )
        .expect("token regeneration ability should parse");
    let create = def
        .spell_effect
        .as_ref()
        .expect("spell effects")
        .iter()
        .find_map(|effect| effect.as_create_token())
        .expect("created Skeleton token");
    let debug = format!("{:#?}", create.token.abilities);
    assert!(
        debug.contains("RegenerateEffect") && debug.contains("Source"),
        "`this token` should lower to the regeneration ability's source: {debug}"
    );
}

#[test]
fn multiple_quoted_artifact_token_rules_are_all_nested() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Artifact Rules Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Create a tapped colorless artifact token named Meteorite with \"When this token enters, it deals 2 damage to any target\" and \"{T}: Add one mana of any color.\"",
        )
        .expect("multiple artifact-token rules should parse");
    let create = def
        .spell_effect
        .as_ref()
        .expect("spell effects")
        .iter()
        .find_map(|effect| effect.as_create_token())
        .expect("created Meteorite token");
    let debug = format!("{:#?}", create.token.abilities);
    assert!(
        debug.contains("Triggered")
            && debug.contains("DealDamageEffect")
            && debug.contains("Activated")
            && debug.contains("AddManaOfAnyColorEffect"),
        "both quoted Meteorite abilities should remain on its definition: {debug}"
    );
}

#[test]
fn quoted_tap_sacrifice_any_color_rule_keeps_both_costs() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Etherium Cell Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Create a colorless artifact token named Etherium Cell with \"{T}, Sacrifice this token: Add one mana of any color.\"",
        )
        .expect("Etherium Cell rule should parse");
    let create = def
        .spell_effect
        .as_ref()
        .expect("spell effects")
        .iter()
        .find_map(|effect| effect.as_create_token())
        .expect("created Etherium Cell token");
    let activated = create
        .token
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            crate::ability::AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Etherium Cell activated mana ability");
    assert!(matches!(
        activated.mana_cost.costs(),
        [crate::costs::Cost::Tap, crate::costs::Cost::SacrificeSelf]
    ));
    assert!(
        format!("{:#?}", activated.effects).contains("AddManaOfAnyColorEffect"),
        "{activated:#?}"
    );
}

#[test]
fn full_lost_in_the_spirit_world_keeps_reciprocal_spirit_restriction_once() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Lost in the Spirit World")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Return up to one target creature to its owner's hand. Create a 1/1 colorless Spirit creature token with \"This token can't block or be blocked by non-Spirit creatures.\"",
        )
        .expect("full Lost in the Spirit World text should parse");
    let create = def
        .spell_effect
        .as_ref()
        .expect("instant spell effects")
        .iter()
        .find_map(|effect| effect.as_create_token())
        .expect("Lost in the Spirit World should create a Spirit");

    assert_eq!(
        create.token.abilities.len(),
        1,
        "the specialized restriction must not also be granted generically: {create:#?}"
    );
    let debug = format!("{:#?}", create.token.abilities);
    assert_eq!(debug.matches("BlockSpecificAttacker").count(), 2, "{debug}");
    assert!(debug.contains("Spirit"), "{debug}");
}

#[test]
fn full_tezzeret_the_schemer_keeps_etherium_cell_mana_ability_once() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Tezzeret the Schemer")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(5)
        .parse_text(
            "+1: Create a colorless artifact token named Etherium Cell with \"{T}, Sacrifice this token: Add one mana of any color.\"\n−2: Target creature gets +X/-X until end of turn, where X is the number of artifacts you control.\n−7: You get an emblem with \"At the beginning of combat on your turn, target artifact you control becomes an artifact creature with base power and toughness 5/5.\"",
        )
        .expect("full Tezzeret the Schemer text should parse");
    let create = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated
                .effects
                .flattened_default_effects()
                .iter()
                .find_map(|effect| effect.as_create_token()),
            _ => None,
        })
        .expect("Tezzeret's +1 should create an Etherium Cell");

    assert_eq!(
        create.token.abilities.len(),
        1,
        "the specialized mana rule must not also be granted generically: {create:#?}"
    );
    let activated = create
        .token
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Etherium Cell activated mana ability");
    assert!(matches!(
        activated.mana_cost.costs(),
        [crate::costs::Cost::Tap, crate::costs::Cost::SacrificeSelf]
    ));
    assert!(
        format!("{:#?}", activated.effects).contains("AddManaOfAnyColorEffect"),
        "{activated:#?}"
    );
}

#[test]
fn full_toggo_keeps_rock_grant_and_equip_once_each() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Toggo, Goblin Weaponsmith")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Landfall — Whenever a land you control enters, create a colorless Equipment artifact token named Rock with \"Equipped creature has '{1}, {T}, Sacrifice Rock: This creature deals 2 damage to any target'\" and equip {1}.\nPartner (You can have two commanders if both have partner.)",
        )
        .expect("full Toggo text should parse");
    let create = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .effects
                .flattened_default_effects()
                .iter()
                .find_map(|effect| effect.as_create_token()),
            _ => None,
        })
        .expect("Toggo's landfall ability should create a Rock");

    assert_eq!(
        create.token.abilities.len(),
        2,
        "Rock should own one attached grant and one equip ability: {create:#?}"
    );
    let debug = format!("{:#?}", create.token.abilities);
    assert_eq!(debug.matches("AttachedAbilityGrant").count(), 1, "{debug}");
    let equip_costs = create
        .token
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated.mana_cost.display()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(equip_costs.len(), 1, "{debug}");
    assert!(equip_costs[0].contains("{1}"), "{equip_costs:?}");
}

#[test]
fn later_quoted_artifact_token_activation_is_not_dropped() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Notebook Rules Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Create Tamiyo's Notebook, a legendary colorless Book artifact token with \"Spells you cast cost {2} less to cast\" and \"{T}: Draw a card.\"",
        )
        .expect("both Notebook abilities should parse");
    let create = def
        .spell_effect
        .as_ref()
        .expect("spell effects")
        .iter()
        .find_map(|effect| effect.as_create_token())
        .expect("created Notebook token");
    let debug = format!("{:#?}", create.token.abilities);
    assert!(
        debug.contains("DrawCardEffect"),
        "the second quoted Notebook ability should remain attached: {debug}"
    );
}

#[test]
fn full_tamiyo_compleated_sage_keeps_both_notebook_abilities() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Tamiyo, Compleated Sage")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(5)
        .parse_text(
            "Compleated\n+1: Tap up to one target artifact or creature. It doesn't untap during its controller's next untap step.\n−X: Exile target nonland permanent card with mana value X from your graveyard. Create a token that's a copy of that card.\n−7: Create Tamiyo's Notebook, a legendary colorless Book artifact token with \"Spells you cast cost {2} less to cast\" and \"{T}: Draw a card.\"",
        )
        .expect("full Tamiyo oracle text should parse");
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("CreateTokenEffect")
            && debug.contains("CostReduction")
            && debug.contains("DrawCardEffect"),
        "full-card production dispatch must retain both Notebook abilities: {debug}"
    );
}

#[test]
fn quoted_sacrifice_damage_activation_stays_on_created_token() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Token Activation Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Create a 1/1 colorless Triskelavite artifact creature token with flying. It has \"Sacrifice this token: This token deals 1 damage to any target.\"",
        )
        .expect("token sacrifice activation should parse");
    let create = def
        .spell_effect
        .as_ref()
        .expect("spell effects")
        .iter()
        .find_map(|effect| effect.as_create_token())
        .expect("created Triskelavite token");
    let debug = format!("{:#?}", create.token.abilities);
    assert!(
        debug.contains("Activated")
            && debug.contains("SacrificeSelf")
            && debug.contains("DealDamageEffect"),
        "quoted activation should be part of the Triskelavite definition: {debug}"
    );
}

#[test]
fn endure_builds_typed_spirit_without_token_text_parsing() {
    let action = SubjectVerbActionAst::Endure {
        target: TargetAst::Object(ObjectFilter::source(), None, None),
        amount: Value::Fixed(2),
    };
    let ast = EffectAst::subject_verb(SubjectVerbRoleAst::Actor, PlayerAst::You, action);
    let (effects, choices) =
        compile_effect(&ast, &mut EffectLoweringContext::new()).expect("fixed endure should lower");
    assert!(choices.is_empty());
    let choose = effects[0]
        .as_choose_mode()
        .expect("endure should lower to two modes");
    let create = choose.modes[1].effects[0]
        .as_create_token()
        .expect("endure's second mode should create a token");
    assert_eq!(create.token.card.name, "Spirit");
    assert_eq!(create.token.card.subtypes, vec![Subtype::Spirit]);
    assert_eq!(create.token.card.color_indicator, Some(ColorSet::WHITE));
    assert_eq!(
        create.token.card.power_toughness,
        Some(PowerToughness::fixed(2, 2))
    );
}

#[test]
fn resolve_target_spec_treats_source_object_filters_as_source() {
    let target = TargetAst::Object(ObjectFilter::source(), None, None);
    let (spec, choices) = resolve_target_spec_with_choices(&target, &ReferenceEnv::default())
        .expect("source object target should resolve cleanly");

    assert_eq!(
        spec,
        ChooseSpec::Source,
        "source object filters should resolve to the source choose spec"
    );
    assert!(
        choices.is_empty(),
        "self-targeted object filters should not create extra target choices"
    );
}

#[test]
fn resolve_target_spec_preserves_source_surface_when_collapsing_to_source() {
    let target = TargetAst::Object(
        ObjectFilter::source().with_source_surface(
            crate::target::SourceReferenceSurface::ThisPermanentType(
                "this enchantment".to_string(),
            ),
        ),
        None,
        None,
    );
    let (spec, choices) = resolve_target_spec_with_choices(&target, &ReferenceEnv::default())
        .expect("source object target should resolve cleanly");

    assert_eq!(
        spec,
        ChooseSpec::Source.with_surface_hint(
            crate::target::ChooseSpecSurfaceHint::SourceReference(
                crate::target::SourceReferenceSurface::ThisPermanentType(
                    "this enchantment".to_string(),
                ),
            ),
        ),
        "source object filters should keep their captured source surface"
    );
    assert!(
        choices.is_empty(),
        "self-targeted object filters should not create extra target choices"
    );
}

#[test]
fn resolve_target_spec_preserves_implicit_it_when_it_resolves_to_source() {
    let target = TargetAst::Tagged(
        TagKey::from(IT_TAG),
        Some(TextSpan {
            line: 0,
            start: 42,
            end: 44,
        }),
    );
    let refs = ReferenceEnv {
        source_object_antecedent: true,
        ..ReferenceEnv::default()
    };
    let (spec, choices) = resolve_target_spec_with_choices(&target, &refs)
        .expect("implicit it target should resolve cleanly");

    assert_eq!(
        spec,
        ChooseSpec::Source.with_surface_hint(
            crate::target::ChooseSpecSurfaceHint::SourceReference(
                crate::target::SourceReferenceSurface::ThisPermanentType("it".to_string()),
            ),
        ),
        "implicit it should keep its surface when reference resolution maps it to source"
    );
    assert!(
        choices.is_empty(),
        "self-targeted implicit references should not create extra target choices"
    );
}

#[test]
fn resolve_target_spec_preserves_source_object_filters_from_exile() {
    let target = TargetAst::Object(ObjectFilter::source().in_zone(Zone::Exile), None, None);
    let (spec, choices) = resolve_target_spec_with_choices(&target, &ReferenceEnv::default())
        .expect("source object target from exile should resolve cleanly");

    assert_eq!(
        spec,
        ChooseSpec::Object(ObjectFilter::source().in_zone(Zone::Exile)),
        "source object filters from exile should keep their zone"
    );
    assert!(
        choices.is_empty(),
        "self-targeted object filters should not create extra target choices"
    );
}

fn test_ctx(line: &str) -> NormalizedLine {
    NormalizedLine {
        original: line.to_string(),
        normalized: line.to_string(),
        char_map: (0..line.len()).collect(),
    }
}

#[test]
fn collect_tag_spans_tracks_connive_and_destroy_no_regeneration_targets() {
    let mut annotations = ParseAnnotations::default();
    let ctx = test_ctx("alpha beta");
    let alpha = TagKey::from("alpha");
    let beta = TagKey::from("beta");

    collect_tag_spans_from_effect(
        &EffectAst::subject_verb_connive(
            TargetAst::Tagged(
                alpha.clone(),
                Some(TextSpan {
                    line: 0,
                    start: 0,
                    end: 5,
                }),
            ),
            Value::Fixed(1),
        ),
        &mut annotations,
        &ctx,
    );
    collect_tag_spans_from_effect(
        &EffectAst::subject_verb_destroy_no_regeneration(TargetAst::Tagged(
            beta.clone(),
            Some(TextSpan {
                line: 0,
                start: 6,
                end: 10,
            }),
        )),
        &mut annotations,
        &ctx,
    );

    assert!(
        annotations
            .tag_spans
            .get(alpha.as_str())
            .is_some_and(|spans| spans.len() == 1),
        "expected span recorded for connive target tag"
    );
    assert!(
        annotations
            .tag_spans
            .get(beta.as_str())
            .is_some_and(|spans| spans.len() == 1),
        "expected span recorded for destroy-no-regeneration target tag"
    );
}

#[test]
fn collect_tag_spans_tracks_counter_unless_pays_target() {
    let mut annotations = ParseAnnotations::default();
    let ctx = test_ctx("gamma");
    let gamma = TagKey::from("gamma");
    let effect = EffectAst::subject_verb_counter_unless_pays(
        TargetAst::Tagged(
            gamma.clone(),
            Some(TextSpan {
                line: 0,
                start: 0,
                end: 5,
            }),
        ),
        TotalCost::free(),
    );

    collect_tag_spans_from_effect(&effect, &mut annotations, &ctx);
    assert!(
        annotations
            .tag_spans
            .get(gamma.as_str())
            .is_some_and(|spans| spans.len() == 1),
        "expected span recorded for counter-unless-pays target tag"
    );
    assert!(
        effect_references_tag(&effect, "gamma"),
        "counter-unless-pays tagged target should be detected by tag reference checks"
    );
}

#[test]
fn this_attacks_triggers_bind_the_defending_player() {
    assert_eq!(
        inferred_trigger_player_filter(&TriggerSpec::ThisAttacks),
        Some(PlayerFilter::Defending)
    );
}

#[test]
fn compile_statement_effects_drops_empty_global_ability_grants() {
    let effects = vec![EffectAst::subject_verb_grant_abilities_all(
        ObjectFilter::default(),
        Vec::new(),
        Until::EndOfTurn,
    )];

    let compiled =
        compile_statement_effects(&effects).expect("normalization should remove empty grants");
    assert!(compiled.is_empty());
}

#[test]
fn compile_statement_effects_with_imports_returns_reference_exports() {
    let effects = vec![EffectAst::subject_verb_destroy(TargetAst::Object(
        ObjectFilter::creature(),
        Some(TextSpan::synthetic()),
        None,
    ))];

    let lowered = compile_statement_effects_with_imports(&effects, &ReferenceImports::default())
        .expect("compile statement with imports");

    assert!(
        !lowered.effects.is_empty(),
        "expected at least one lowered effect for destroy statement"
    );
    assert_eq!(
        lowered.exports.last_object_tag,
        RefState::Known(TagKey::from("destroyed_0"))
    );
}

#[test]
fn compile_effects_with_explicit_frame_uses_annotated_reference_frames() {
    let effects = vec![
        EffectAst::subject_verb_destroy(TargetAst::Object(
            ObjectFilter::creature(),
            Some(TextSpan::synthetic()),
            None,
        )),
        EffectAst::subject_verb_grant_play_tagged_until_end_of_turn(
            TagKey::from(IT_TAG),
            PlayerAst::You,
            false,
            false,
            false,
        ),
    ];

    let (compiled, _, frame_out) = compile_effects_with_explicit_frame(
        &effects,
        &mut IdGenContext::default(),
        LoweringFrame::default(),
    )
    .expect("compile with explicit frame");

    let debug = format!("{compiled:?}");
    assert!(
        debug.contains("GrantPlayTaggedEffect"),
        "grant-play-tagged effect: {debug}"
    );
    assert!(
        debug.contains("destroyed_0"),
        "grant-play-tagged tag: {debug}"
    );
    assert_eq!(frame_out.last_object_tag.as_deref(), Some("destroyed_0"));
}

#[test]
fn synthesis_pod_consult_match_keeps_its_tag_through_exile_and_cast() {
    let match_tag = TagKey::from("__sentence_helper_consult_match_l0_s0_e0");
    let effects = vec![
        EffectAst::subject_verb_consult_top_of_library(
            PlayerAst::You,
            crate::cards::builders::LibraryConsultModeAst::Reveal,
            ObjectFilter::default(),
            crate::cards::builders::LibraryConsultStopRuleAst::MatchCount(Value::Fixed(1)),
            TagKey::from("__sentence_helper_revealed_l0_s0_e0"),
            match_tag.clone(),
        ),
        EffectAst::subject_verb_exile(
            TargetAst::Tagged(match_tag.clone(), Some(TextSpan::synthetic())),
            false,
        ),
        EffectAst::subject_verb_cast_tagged(
            TagKey::from(IT_TAG),
            PlayerAst::You,
            false,
            false,
            true,
            None,
        ),
    ];

    let (compiled, _, _) = compile_effects_with_explicit_frame(
        &effects,
        &mut IdGenContext::default(),
        LoweringFrame::default(),
    )
    .expect("compile Synthesis Pod consult/exile/cast chain");

    let cast = compiled
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::CastTaggedEffect>())
        .expect("consult follow-up should cast its tagged match");
    assert_eq!(cast.tag, match_tag);
}

#[test]
fn praetors_grasp_search_exile_uses_source_exiled_permission_provenance() {
    let searched_tag = TagKey::from("searched_face_down");
    let mut searched_filter = ObjectFilter::default().in_zone(Zone::Library);
    searched_filter.owner = Some(PlayerFilter::Opponent);
    let effects = vec![
        EffectAst::ChooseObjects {
            filter: searched_filter,
            count: crate::effect::ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: searched_tag.clone(),
        },
        EffectAst::subject_verb_exile(
            TargetAst::Tagged(searched_tag.clone(), Some(TextSpan::synthetic())),
            true,
        ),
        EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
            TagKey::from(IT_TAG),
            PlayerAst::You,
            true,
            false,
            false,
            None,
        ),
    ];

    let (compiled, _, _) = compile_effects_with_explicit_frame(
        &effects,
        &mut IdGenContext::default(),
        LoweringFrame::default(),
    )
    .expect("compile Praetor's Grasp search/exile/play chain");

    let grant = compiled
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>())
        .expect("searched exiled card should receive a play permission");
    assert_eq!(grant.tag.as_str(), crate::tag::SOURCE_EXILED_TAG);
    assert_ne!(grant.tag, searched_tag);
}

#[test]
fn optional_exile_from_another_players_hand_does_not_use_controller_hand_imprint() {
    let mut filter = ObjectFilter::nonland().in_zone(Zone::Hand);
    filter.owner = Some(PlayerFilter::target_opponent());
    let ast = EffectAst::MayByPlayer {
        player: PlayerAst::You,
        effects: vec![EffectAst::subject_verb_exile(
            TargetAst::Object(filter, None, None),
            false,
        )],
    };

    let (compiled, _) = compile_effect(&ast, &mut EffectLoweringContext::new())
        .expect("optional opponent-hand exile should lower");
    assert!(
        compiled.iter().all(|effect| effect
            .downcast_ref::<crate::effects::cards::ImprintFromHandEffect>()
            .is_none()),
        "controller-hand-only imprint must not lower an opponent-hand choice: {compiled:#?}"
    );
    let may = compiled
        .iter()
        .find_map(|effect| {
            effect.downcast_ref::<crate::effects::MayEffect<crate::effect::Effect>>()
        })
        .expect("opponent-hand exile should retain its optional branch");
    assert!(
        may.effects.iter().any(|effect| effect
            .downcast_ref::<crate::effects::ChooseObjectsEffect>()
            .is_some()),
        "optional branch must choose from the referenced hand: {may:#?}"
    );
    assert!(
        may.effects.iter().any(|effect| effect
            .downcast_ref::<crate::effects::ExileEffect>()
            .is_some()),
        "optional branch must exile the chosen card: {may:#?}"
    );
}

#[test]
fn compile_may_branch_preserves_auto_tagged_destroy_followup() {
    let effects = vec![
        EffectAst::May {
            effects: vec![EffectAst::subject_verb_destroy(TargetAst::WithCount(
                Box::new(TargetAst::Object(
                    ObjectFilter::creature(),
                    Some(TextSpan::synthetic()),
                    None,
                )),
                ChoiceCount::up_to(3),
            ))],
        },
        EffectAst::subject_verb_grant_play_tagged_until_end_of_turn(
            TagKey::from(IT_TAG),
            PlayerAst::You,
            false,
            false,
            false,
        ),
    ];

    let (compiled, _, frame_out) = compile_effects_with_explicit_frame(
        &effects,
        &mut IdGenContext::default(),
        LoweringFrame::default(),
    )
    .expect("compile may branch with tagged follow-up");

    let debug = format!("{compiled:?}");
    assert!(debug.contains("MayEffect"), "expected may effect: {debug}");
    assert!(
        debug.contains("TaggedEffect"),
        "destroy should stay tagged: {debug}"
    );
    assert!(
        debug.contains("DestroyEffect"),
        "expected destroy effect: {debug}"
    );
    assert!(
        debug.contains("destroyed_0"),
        "expected destroy tag: {debug}"
    );
    assert!(
        debug.contains("GrantPlayTaggedEffect"),
        "expected grant-play-tagged follow-up: {debug}"
    );
    assert_eq!(frame_out.last_object_tag.as_deref(), Some("destroyed_0"));
}

#[test]
fn compile_optional_turn_skip_keeps_if_result_inside_may_branch() {
    let effects = vec![EffectAst::Conditional {
        predicate: PredicateAst::SourceIsTapped,
        if_true: vec![EffectAst::May {
            effects: vec![
                EffectAst::subject_verb_skip_turn(PlayerAst::You),
                EffectAst::IfResult {
                    predicate: IfResultPredicate::Did,
                    effects: vec![EffectAst::subject_verb_untap(TargetAst::Source(None))],
                },
            ],
        }],
        if_false: Vec::new(),
    }];

    let mut ctx = EffectLoweringContext::new();
    let (compiled, _) = compile_effects(&effects, &mut ctx)
        .expect("optional turn skip should lower with its if-result followup");
    let debug = format!("{compiled:?}");
    assert!(debug.contains("SkipTurnEffect"), "{debug}");
    assert!(debug.contains("UntapEffect"), "{debug}");
}

#[test]
fn compile_last_known_countered_spell_preserves_stack_identity() {
    let mut target_filter = ObjectFilter::creature().in_zone(Zone::Stack);
    target_filter.stack_kind = Some(crate::filter::StackObjectKind::Spell);
    let mut legendary_spell = ObjectFilter::spell();
    legendary_spell.supertypes = vec![crate::types::Supertype::Legendary];
    let effects = vec![
        EffectAst::subject_verb_counter(TargetAst::Object(
            target_filter,
            Some(TextSpan::synthetic()),
            None,
        )),
        EffectAst::Conditional {
            predicate: PredicateAst::ItMatchedLastKnown(legendary_spell),
            if_true: vec![EffectAst::subject_verb_ring_tempts_you(PlayerAst::You)],
            if_false: Vec::new(),
        },
    ];

    let mut ctx = EffectLoweringContext::new();
    let (compiled, _) = compile_effects(&effects, &mut ctx)
        .expect("countered-spell last-known predicate should lower");
    let debug = format!("{compiled:#?}");
    assert!(debug.contains("TaggedObjectMatchedLastKnown"), "{debug}");
    assert!(
        debug.contains("zone: Some(\n                        Stack"),
        "{debug}"
    );
    assert!(
        debug.contains("stack_kind: Some(\n                        Spell"),
        "{debug}"
    );
}

#[test]
fn compile_for_each_tagged_rewrites_it_targets_to_iterated_object() {
    let effects = vec![EffectAst::ForEachTagged {
        tag: TagKey::from("revealed_0"),
        effects: vec![EffectAst::Conditional {
            predicate: PredicateAst::ItMatches(ObjectFilter::permanent()),
            if_true: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(TagKey::from(IT_TAG), None),
                Zone::Battlefield,
                false,
                ReturnControllerAst::Owner,
                false,
                None,
            )],
            if_false: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(TagKey::from(IT_TAG), None),
                Zone::Graveyard,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            )],
        }],
    }];

    let (compiled, _, _) = compile_effects_with_explicit_frame(
        &effects,
        &mut IdGenContext::default(),
        LoweringFrame::default(),
    )
    .expect("compile for-each-tagged");

    let debug = format!("{compiled:?}");
    assert!(
        debug.contains("ForEachTaggedEffect"),
        "for-each-tagged effect: {debug}"
    );
    assert!(
        debug.contains("ConditionalEffect"),
        "conditional effect: {debug}"
    );
    assert!(
        debug.contains("TaggedObjectMatches(TagKey(\"__it__\")"),
        "it-binding condition: {debug}"
    );
    assert!(
        debug.matches("target: Iterated").count() >= 2,
        "iterated move targets: {debug}"
    );
}

#[test]
fn compile_next_spell_grant_after_targeted_player_effect_binds_that_player() {
    let effects = vec![
        EffectAst::subject_verb_add_mana_any_one_color(PlayerAst::Target, Value::Fixed(2)),
        EffectAst::subject_verb_grant_next_spell_ability_this_turn(
            PlayerAst::That,
            ObjectFilter::spell().cast_by(PlayerFilter::IteratedPlayer),
            GrantedAbilityAst::KeywordAction(crate::cards::builders::KeywordAction::Cascade),
        ),
    ];

    let (compiled, _, _) = compile_effects_with_explicit_frame(
        &effects,
        &mut IdGenContext::default(),
        LoweringFrame::default(),
    )
    .expect("targeted player followup should compile");

    let debug = format!("{compiled:?}");
    assert!(
        debug.contains("GrantNextSpellAbilityEffect"),
        "expected next-spell grant effect: {debug}"
    );
    assert!(
        !debug.contains("player: IteratedPlayer"),
        "grant player should be rebound: {debug}"
    );
    assert!(
        !debug.contains("cast_by: Some(IteratedPlayer)"),
        "grant filter caster should be rebound: {debug}"
    );
    assert!(
        debug.contains("AliasedTarget(Any)"),
        "follow-up player should retain selected-target provenance: {debug}"
    );
}

#[test]
fn devour_flesh_carries_its_target_into_the_life_gain_without_retargeting() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Devour Flesh")
        .parse_text(
            "Target player sacrifices a creature of their choice, then gains life equal to that creature's toughness.",
        )
        .expect("Devour Flesh should parse");

    let debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    assert!(
        debug.contains("SacrificeEffect"),
        "sacrifice effect: {debug}"
    );
    assert!(
        debug.contains("GainLifeEffect"),
        "life-gain effect: {debug}"
    );
    assert!(
        debug.contains("player: AliasedTarget(Any)"),
        "implicit follow-up should use the previously selected target: {debug}"
    );
}

#[test]
fn restorative_technique_keeps_the_target_as_searcher_and_library_owner() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Restorative Technique")
        .parse_text(
            "Target player gains 2 life, then searches their library for a basic land card, puts it onto the battlefield tapped, then shuffles. Put a +1/+1 counter on up to one target creature.",
        )
        .expect("Restorative Technique should parse");

    let debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    assert!(
        debug.contains("SearchLibraryEffect"),
        "search effect: {debug}"
    );
    assert!(
        debug.contains("ShuffleLibraryEffect"),
        "shuffle effect: {debug}"
    );
    assert!(
        debug.matches("AliasedTarget(Any)").count() >= 2,
        "searcher/library owner and shuffle should share the selected player: {debug}"
    );
}

#[test]
fn kicked_targeted_search_keeps_one_library_owner_across_document_sentences() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Kicked Targeted Search Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Kicker {7}\nSearch target player's library for up to three cards, exile them, then that player shuffles. If this spell was kicked, instead search that player's library for up to fifteen cards, exile them, then that player shuffles.",
        )
        .expect("kicked targeted search should parse through the full document pipeline");

    let debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    assert!(
        !debug.contains("IteratedPlayer"),
        "both search branches must use the spell's selected target: {debug}"
    );
    assert_eq!(
        debug.matches("chooser: You").count(),
        2,
        "the spell controller should search both branches: {debug}"
    );
    assert_eq!(
        debug.matches("ShuffleLibraryEffect").count(),
        2,
        "each branch should shuffle the targeted library exactly once: {debug}"
    );
}

#[test]
fn compile_next_spell_grant_with_imported_target_player_binds_that_player() {
    let effects = vec![EffectAst::subject_verb_grant_next_spell_ability_this_turn(
        PlayerAst::That,
        ObjectFilter::spell().cast_by(PlayerFilter::IteratedPlayer),
        GrantedAbilityAst::KeywordAction(crate::cards::builders::KeywordAction::Cascade),
    )];

    let frame = LoweringFrame {
        last_player_filter: Some(PlayerFilter::target_player()),
        ..Default::default()
    };
    let (compiled, _, _) =
        compile_effects_with_explicit_frame(&effects, &mut IdGenContext::default(), frame)
            .expect("imported target-player followup should compile");

    let debug = format!("{compiled:?}");
    assert!(
        debug.contains("GrantNextSpellAbilityEffect"),
        "expected next-spell grant effect: {debug}"
    );
    assert!(
        !debug.contains("player: IteratedPlayer"),
        "grant player should be rebound: {debug}"
    );
    assert!(
        !debug.contains("cast_by: Some(IteratedPlayer)"),
        "grant filter caster should be rebound: {debug}"
    );
}

#[test]
fn compile_shared_you_then_that_player_draw_preserves_prior_non_you_binding() {
    let effects = vec![
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::Draw {
                count: Value::Fixed(1),
            },
        ),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::That,
            SubjectVerbActionAst::Draw {
                count: Value::Fixed(1),
            },
        ),
    ];

    let frame = LoweringFrame {
        last_player_filter: Some(PlayerFilter::DamagedPlayer),
        ..Default::default()
    };
    let (compiled, _, _) =
        compile_effects_with_explicit_frame(&effects, &mut IdGenContext::default(), frame)
            .expect("shared draw follow-up should compile");

    let debug = format!("{compiled:?}");
    assert_eq!(
        debug.matches("DrawCardsEffect").count(),
        2,
        "expected two draw effects: {debug}"
    );
    assert!(
        debug.contains("player: You"),
        "first draw should target you: {debug}"
    );
    assert!(
        debug.contains("player: DamagedPlayer"),
        "second draw should preserve damaged-player binding: {debug}"
    );
}
