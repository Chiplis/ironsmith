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
