//! Recognition-to-runtime tests.
//!
//! Each of these recognizes real Oracle text and asserts what it lowers to, so
//! they exercise both phases and live in the crate that assembles them.

use ironsmith_compiler::ParseCardText;
use ironsmith_compiler::card::PowerToughness;
use ironsmith_compiler::color::ColorSet;
use ironsmith_compiler::compile_support::*;
use ironsmith_compiler::front_end::NormalizedLine;
use ironsmith_compiler::reference_helpers::*;
use ironsmith_compiler::tag_support::*;
use ironsmith_compiler_resolve::SpanMappingContext;

/// Lower a token definition named by its printed shape text.
///
/// Recognizing the shape is the grammar's job and lowering it is the lowering
/// crate's, so the two only meet here.
fn token_definition_for(name: &str) -> Option<ironsmith_compiler::CardDefinition> {
    let shape = token_definition_shape_text(name)?;
    lower_token_definition_shape(shape)
}

/// The shape a token definition's text parses to. The fixture is tokenized
/// here: it is test text, not a line the document phase has seen.
fn token_definition_shape_text(
    text: &str,
) -> Option<ironsmith_compiler::grammar::token_definitions::TokenDefinitionSpec> {
    ironsmith_compiler::grammar::token_definitions::parse_token_definition_shape_tokens(
        &lex_line(text, 0).ok()?,
    )
}

use ironsmith_compiler::effect::{Condition, Until};
use ironsmith_compiler::filter::*;
use ironsmith_compiler::object::CounterType;
use ironsmith_compiler::zone::Zone;

use ironsmith_compiler::lexer::{
    lex_line, render_token_slice, split_lexed_sentences, trim_lexed_commas,
};
use ironsmith_compiler::parse_context::ParseContext;
use ironsmith_compiler::semantic_line_parsing::*;

use ironsmith_compiler::ability::AbilityKind;
use ironsmith_compiler::cards::builders::*;
use ironsmith_compiler::effect::{Effect, Value};

use ironsmith_compiler::lowering_support::*;
use ironsmith_compiler::model::reference_state::RefState;
use ironsmith_compiler::target::ChooseSpec;
use ironsmith_compiler::types::{CardType, Subtype};
use ironsmith_compiler::{CardDefinitionBuilder, CardId, CardTextError, TagKey};
use std::path::Path;

#[test]
fn quantified_damage_binds_that_players_life_total_to_each_recipient() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Quantified Life Damage Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{T}: This creature deals damage to each player equal to half that player's life total, rounded down.",
        )
        .expect("quantified player-relative damage should compile");
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith_compiler::ability::AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("activated damage ability");
    let effects = activated.effects.flattened_default_effects();
    let for_players = effects[0]
        .downcast_ref::<ironsmith_compiler::effects::ForPlayersEffect<Effect>>()
        .expect("quantified player effect");
    let damage = for_players.effects[0]
        .downcast_ref::<ironsmith_compiler::effects::DealDamageEffect>()
        .expect("damage action");
    assert_eq!(
        damage.target.unhinted(),
        &ChooseSpec::Player(ironsmith_compiler::target::PlayerFilter::IteratedPlayer)
    );
    let Value::HalfRoundedDown(inner) = damage.amount.unhinted() else {
        panic!("expected half-rounded-down amount: {damage:#?}");
    };
    assert_eq!(
        inner.as_ref(),
        &Value::LifeTotal(ironsmith_compiler::target::PlayerFilter::IteratedPlayer),
        "the amount must use the same participant as the recipient"
    );
}

#[test]
fn triggered_unless_effect_cost_keeps_the_choice_payer_relative() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Landfall Unless-Cost Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever a player puts a Swamp onto the battlefield, this enchantment deals 3 damage to that player unless the player puts a -1/-1 counter on a creature they control.",
        )
        .expect("a trigger participant should be able to pay a typed effect cost");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith_compiler::ability::AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("land-entry trigger");
    fn find_unless(
        effect: &Effect,
    ) -> Option<ironsmith_compiler::effects::UnlessPaysEffect<Effect>> {
        if let Some(unless) =
            effect.downcast_ref::<ironsmith_compiler::effects::UnlessPaysEffect<Effect>>()
        {
            return Some(unless.clone());
        }
        let mut found = None;
        effect.visit_child_effects(&mut |child| {
            if found.is_none() {
                found = find_unless(child);
            }
        });
        found
    }
    let unless = triggered
        .effects
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .find_map(find_unless)
        .unwrap_or_else(|| panic!("damage should retain its typed unless-payment: {triggered:#?}"));
    let [ironsmith_compiler::costs::Cost::Effect(cost_effect)] = unless
        .cost
        .as_all()
        .expect("the counter placement should be one effect cost")
    else {
        panic!("expected one effect cost: {unless:#?}");
    };
    let put = cost_effect
        .downcast_ref::<ironsmith_compiler::effects::PutCountersEffect>()
        .expect("the player pays by putting a -1/-1 counter");
    let ChooseSpec::WithCount(target, _) = &put.target else {
        panic!("expected an exact-one creature choice: {put:#?}");
    };
    let ChooseSpec::Object(filter) = target.as_ref() else {
        panic!("expected a creature object filter: {put:#?}");
    };
    assert_eq!(
        filter.controller,
        Some(ironsmith_compiler::target::PlayerFilter::You),
        "cost execution rebases `You` to the player selected by UnlessPays"
    );
    assert!(
        !format!("{:#?}", unless.cost).contains("IteratedPlayer"),
        "the payer must not require a second unbound iteration context: {unless:#?}"
    );
}

fn find_create_token_effect(
    effects: &[Effect],
) -> Option<&ironsmith_compiler::effects::CreateTokenEffect> {
    effects.iter().find_map(|effect| {
        if let Some(create) = effect.as_create_token() {
            return Some(create);
        }
        if let Some(tagged) = effect.as_tagged() {
            return find_create_token_effect(std::slice::from_ref(&tagged.effect));
        }
        if let Some(with_id) = effect.as_with_id() {
            return find_create_token_effect(std::slice::from_ref(&with_id.effect));
        }
        effect
            .downcast_ref::<ironsmith_compiler::effects::SequenceEffect>()
            .and_then(|sequence| find_create_token_effect(&sequence.effects))
    })
}

trait FindCreateTokenEffect {
    fn find_create_token_effect(&self) -> Option<&ironsmith_compiler::effects::CreateTokenEffect>;
}

impl FindCreateTokenEffect for [Effect] {
    fn find_create_token_effect(&self) -> Option<&ironsmith_compiler::effects::CreateTokenEffect> {
        find_create_token_effect(self)
    }
}

fn find_cant_effect(effects: &[Effect]) -> Option<&ironsmith_compiler::effects::CantEffect> {
    effects.iter().find_map(|effect| {
        if let Some(cant) = effect.downcast_ref::<ironsmith_compiler::effects::CantEffect>() {
            return Some(cant);
        }
        if let Some(tagged) = effect.as_tagged() {
            return find_cant_effect(std::slice::from_ref(&tagged.effect));
        }
        if let Some(with_id) = effect.as_with_id() {
            return find_cant_effect(std::slice::from_ref(&with_id.effect));
        }
        effect
            .downcast_ref::<ironsmith_compiler::effects::SequenceEffect>()
            .and_then(|sequence| find_cant_effect(&sequence.effects))
    })
}

#[test]
fn clash_then_return_if_you_win_lowers_to_a_clash_observed_local_rewrite() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Clash Return Probe")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Clash with an opponent, then return target creature to its owner's hand. If you win, you may put that creature on top of its owner's library instead.",
        )
        .expect("clash replacement text should compile");
    let effects = definition
        .spell_effect
        .as_ref()
        .expect("the instant should have a spell effect")
        .flattened_default_effects();
    let [clash_with_id, conditional] = effects else {
        panic!("expected an observed clash and one conditional rewrite: {effects:#?}");
    };

    let clash_with_id = clash_with_id
        .as_with_id()
        .expect("the clash result should have an observation id");
    assert!(
        clash_with_id
            .effect
            .downcast_ref::<ironsmith_compiler::effects::ClashEffect>()
            .is_some(),
        "only the clash should be observed: {clash_with_id:#?}"
    );

    let conditional = conditional
        .downcast_ref::<ironsmith_compiler::effects::IfEffect>()
        .expect("the win branch should be an effect-result conditional");
    assert_eq!(conditional.condition, clash_with_id.id);
    assert_eq!(
        conditional.predicate,
        ironsmith_compiler::effect::EffectPredicate::Value(
            ironsmith_compiler::effect::Comparison::GreaterThan(0)
        )
    );
    let [rewrite] = conditional.then.as_slice() else {
        panic!("the clash-win branch should contain one rewrite: {conditional:#?}");
    };
    let rewrite = rewrite
        .downcast_ref::<ironsmith_compiler::effects::LocalRewriteEffect>()
        .expect("the library choice should rewrite the return event locally");
    let tagged_return = rewrite
        .effect
        .as_tagged()
        .expect("the returned creature should retain its result tag");
    assert!(
        tagged_return
            .effect
            .downcast_ref::<ironsmith_compiler::effects::ReturnToHandEffect>()
            .is_some(),
        "the local action should remain the original return: {rewrite:#?}"
    );
    let [replacement] = rewrite.zone_replacements.as_slice() else {
        panic!("the local return should have one zone replacement: {rewrite:#?}");
    };
    assert_eq!(
        replacement.target,
        ChooseSpec::Tagged(tagged_return.tag.clone())
    );
    assert_eq!(
        replacement.from_zone,
        Some(ironsmith_compiler::zone::Zone::Battlefield)
    );
    assert_eq!(
        replacement.to_zone,
        Some(ironsmith_compiler::zone::Zone::Hand)
    );
    assert_eq!(
        replacement.replacement_zone,
        ironsmith_compiler::zone::Zone::Library
    );
    assert!(replacement.optional);

    let [loss_return] = conditional.else_.as_slice() else {
        panic!("the clash-loss branch should keep the ordinary return: {conditional:#?}");
    };
    let loss_return = loss_return
        .as_tagged()
        .expect("the clash-loss return should retain the same result tag");
    assert_eq!(loss_return.tag, tagged_return.tag);
    assert!(
        loss_return
            .effect
            .downcast_ref::<ironsmith_compiler::effects::ReturnToHandEffect>()
            .is_some(),
        "losing the clash should perform the ordinary return: {conditional:#?}"
    );
}

#[test]
fn lowering_preserves_past_control_lki_mode_and_authored_noun() {
    let mut filter = ObjectFilter::default();
    filter.set_demonstrative_antecedent_surface(Some(
        ironsmith_core::DemonstrativeAntecedentSurface::Permanent,
    ));
    let predicate = PredicateAst::PlayerTaggedObjectMatches {
        player: PlayerAst::You,
        tag: TagKey::from("returned_0"),
        filter,
        mode: ironsmith_core::TaggedObjectMatchMode::LastKnown,
    };

    let condition =
        compile_condition_from_predicate_ast(&predicate, &mut EffectLoweringContext::new(), &None)
            .expect("past-control predicate should lower");
    let Condition::PlayerTaggedObjectMatches {
        player,
        tag,
        filter,
        mode,
    } = condition
    else {
        panic!("expected a tagged-object player condition");
    };
    assert_eq!(player, PlayerFilter::You);
    assert_eq!(tag.as_str(), "returned_0");
    assert_eq!(mode, ironsmith_core::TaggedObjectMatchMode::LastKnown);
    assert_eq!(
        filter.demonstrative_antecedent_surface(),
        Some(ironsmith_core::DemonstrativeAntecedentSurface::Permanent)
    );
}

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

/// The crate that owns lowering, whose sources these boundary checks read.
fn compiler_manifest_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates directory")
        .join("ironsmith-compiler-lowering")
}

#[test]
fn player_filter_resolution_stays_behind_subject_context() {
    let manifest_dir = compiler_manifest_dir();
    let src = manifest_dir.join("src");
    let compile_support = manifest_dir
        .join("src/lowering_impl/compile_support.rs")
        .canonicalize()
        .expect("canonical compile_support.rs");
    let helper = manifest_dir
        .join("src/lowering_impl/compile_support/player_effect_helpers.rs")
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
    let manifest_dir = compiler_manifest_dir();
    let compile_support_dir = manifest_dir.join("src/lowering_impl/compile_support");
    let compile_support_rs = manifest_dir.join("src/lowering_impl/compile_support.rs");
    let hidden_filter_field = concat!(".", "player_filter");
    let hidden_choices_field = concat!(".", "choices");

    let mut files = vec![compile_support_rs];
    for entry in std::fs::read_dir(&compile_support_dir).expect("read compile_support dir") {
        let path = entry.expect("read compile_support entry").path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if path.extension().is_none_or(|ext| ext != "rs") {
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
            ironsmith_compiler::cards::builders::PlayerAst::Implicit,
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
            ironsmith_compiler::zone::Zone::Hand,
            false,
            ironsmith_compiler::cards::builders::ReturnControllerAst::Preserve,
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
            .find_map(|effect| {
                effect.downcast_ref::<ironsmith_compiler::effects::MoveToZoneEffect>()
            })
            .expect("move-to-zone effect should remain typed");
        assert_eq!(lowered.verb_surface, surface);
        assert_eq!(lowered.actor_surface, Some(expected_actor));
    }
}

#[test]
fn single_exile_lowering_preserves_explicit_actor() {
    let EffectAst::SubjectVerb(mut subject_verb) =
        EffectAst::subject_verb_exile(TargetAst::Source(None), false)
    else {
        unreachable!("exile constructor must produce a subject-verb AST")
    };
    subject_verb.subject.player = PlayerAst::Opponent;

    let (effects, choices) = compile_effect(
        &EffectAst::SubjectVerb(subject_verb),
        &mut EffectLoweringContext::new(),
    )
    .expect("typed single-object exile should lower");
    assert!(choices.is_empty());
    let lowered = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith_compiler::effects::MoveToZoneEffect>())
        .expect("face-up single exile should lower to a move effect");
    assert_eq!(lowered.zone, ironsmith_compiler::zone::Zone::Exile);
    assert_eq!(lowered.actor_surface, Some(PlayerFilter::Opponent));
}

#[test]
fn source_top_only_zone_actions_choose_the_ordered_card_before_moving_it() {
    let source_filter = ObjectFilter::creature()
        .in_zone(ironsmith_compiler::zone::Zone::Graveyard)
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
        .downcast_ref::<ironsmith_compiler::effects::ChooseObjectsEffect>()
        .expect("ordered exile should begin with a typed choice");
    assert!(exile_choose.top_only);
    assert_eq!(exile_choose.filter, source_filter);
    let exile = exile_effects[1]
        .downcast_ref::<ironsmith_compiler::effects::ExileEffect>()
        .expect("ordered exile should consume the chosen tag");
    assert_eq!(exile.spec, ChooseSpec::tagged(exile_choose.tag.clone()));
    assert!(exile.face_down);

    let move_ast = EffectAst::subject_verb_move_to_zone(
        TargetAst::Object(source_filter.clone(), None, None),
        ironsmith_compiler::zone::Zone::Hand,
        false,
        ironsmith_compiler::cards::builders::ReturnControllerAst::Preserve,
        false,
        None,
    )
    .with_source_top_only(true);
    let (move_effects, move_choices) = compile_effect(&move_ast, &mut EffectLoweringContext::new())
        .expect("top graveyard move should lower");
    assert!(move_choices.is_empty());
    assert_eq!(move_effects.len(), 2);
    let move_choose = move_effects[0]
        .downcast_ref::<ironsmith_compiler::effects::ChooseObjectsEffect>()
        .expect("ordered move should begin with a typed choice");
    assert!(move_choose.top_only);
    assert_eq!(move_choose.filter, source_filter);
    let move_effect = move_effects[1]
        .as_tagged()
        .map(|tagged| tagged.effect.as_ref())
        .unwrap_or(&move_effects[1])
        .downcast_ref::<ironsmith_compiler::effects::MoveToZoneEffect>()
        .expect("ordered move should consume the chosen tag");
    assert_eq!(
        move_effect.target,
        ChooseSpec::tagged(move_choose.tag.clone())
    );
    assert_eq!(move_effect.zone, ironsmith_compiler::zone::Zone::Hand);
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
        .downcast_ref::<ironsmith_compiler::effects::ChooseObjectsEffect>()
        .expect("top-card collection should begin with an ordered choice");
    assert_eq!(
        choose.filter.zone,
        Some(ironsmith_compiler::zone::Zone::Library)
    );
    assert_eq!(choose.count, ChoiceCount::dynamic_x());
    assert_eq!(choose.count_value, Some(Value::Fixed(4)));
    assert!(choose.top_only);
    assert!(ctx.last_exiled_collection_is_plural);

    let battlefield_target = TargetAst::Object(
        ObjectFilter::default().in_zone(ironsmith_compiler::zone::Zone::Battlefield),
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
            TargetAst::Tagged(
                ironsmith_compiler::tag::CompilerReferenceTag::Triggering.key(),
                None,
            ),
            false,
            false,
            false,
            ironsmith_compiler::cards::builders::ReturnControllerAst::You,
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
                .downcast_ref::<ironsmith_compiler::effects::TaggedEffect>()
                .and_then(|tagged| {
                    tagged
                        .effect
                        .downcast_ref::<ironsmith_compiler::effects::MoveToZoneEffect>()
                })
                .or_else(|| effect.downcast_ref::<ironsmith_compiler::effects::MoveToZoneEffect>())
        })
        .expect("explicit-controller return should lower to a move-to-zone effect");
    assert_eq!(
        lowered.verb_surface,
        ironsmith_core::MoveToZoneVerbSurface::Return
    );
    assert_eq!(
        lowered.battlefield_controller,
        ironsmith_compiler::effects::BattlefieldController::You
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
    assert_eq!(
        ctx.last_object_tag.as_ref().map(|tag| tag.as_str()),
        Some("amassed_0")
    );
}

#[test]
fn coordinated_equal_target_specs_keep_distinct_lowered_target_slots() {
    let repeated_target = TargetAst::WithCount(
        Box::new(TargetAst::Object(
            ObjectFilter::creature().other(),
            Some(TextSpan::synthetic()),
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
        .and_then(|effect| effect.downcast_ref::<ironsmith_compiler::effects::SequenceEffect>())
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
        .downcast_ref::<ironsmith_compiler::effects::SequenceEffect>()
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
    let mut recipients = ObjectFilter::creature().without_subtype(Subtype::Army);
    recipients.set_plural_object_noun_surface(true);
    let (effects, choices) = compile_effect(
        &EffectAst::subject_verb_damage_equal_to_power(
            TargetAst::Tagged(TagKey::from("amassed_0"), None),
            TargetAst::Object(recipients, None, None),
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
        debug.contains("PowerOf(Source)"),
        "the source-scoped damage must read the tagged source's power: {debug}"
    );
    assert!(
        debug.contains("target: Iterated"),
        "fan-out iterated target: {debug}"
    );
}

#[test]
fn compile_each_object_power_damage_iterates_sources_and_keeps_prior_target() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Each Source Damage Probe")
        .parse_text(
            "Choose up to one target creature or planeswalker. Each creature with power 4 or greater you control deals damage equal to its power to that permanent.",
        )
        .expect("each-object power damage should parse");
    let debug = format!("{:#?}", def.spell_effect.expect("spell effect"));
    let compact = debug
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();

    assert!(
        compact.contains("TaggedEffect")
            && compact.contains("tag:TagKey(\"targeted_0\",)")
            && compact.contains("TargetOnlyEffect"),
        "the single chosen permanent should remain tagged outside the source loop: {debug}"
    );
    assert!(
        compact.contains("ForEachObject")
            && compact.contains("GreaterThanOrEqual(4")
            && compact.contains("controller:Some(You"),
        "the qualifying controlled creatures should be the iterated set: {debug}"
    );
    assert!(
        compact.contains("ExecuteWithSourceEffect{source:Iterated")
            && (compact.contains("PowerOf(Iterated")
                || compact.contains("PowerOf(SurfaceHinted{spec:Iterated")),
        "each iterand should be both the damage source and its own power value: {debug}"
    );
    assert!(
        compact.contains("target:SurfaceHinted{spec:Object(ObjectFilter")
            && compact.contains("\"targeted_0\"")
            && !compact.contains("target:Iterated"),
        "damage should stay bound to the one prior target, not the source iterand: {debug}"
    );
}

#[test]
fn referenced_pair_toughness_damage_reuses_exactly_the_two_prior_targets() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mutual Toughness Damage Probe")
        .parse_text(
            "Choose target creature you control and target creature an opponent controls. \
             Each of those creatures deals damage equal to its toughness to the other.",
        )
        .expect("mutual toughness damage should parse");
    let program = def.spell_effect.expect("spell effect");
    let debug = format!("{program:#?}");
    let compact = debug
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();

    assert_eq!(
        compact.matches("TargetOnlyEffect").count(),
        2,
        "the reciprocal damage must not announce a third target: {debug}"
    );
    assert!(
        compact.contains("ForEachObject")
            && compact.contains("ExecuteWithSourceEffect{source:Iterated")
            && (compact.contains("ToughnessOf(Iterated")
                || compact.contains("ToughnessOf(SurfaceHinted{spec:Iterated"))
            && compact.contains("target:Object(ObjectFilter")
            && compact.contains("other:true")
            && compact.matches("TagKey(\"__chosen_objects__\"").count() >= 2
            && !compact.contains("target:AnyOtherTarget"),
        "each selected creature must use its own toughness against the other selected creature: {debug}"
    );
}

#[test]
fn cross_sentence_conditional_fights_expose_two_stable_target_slots() {
    for (name, text) in [
        (
            "Blizzard Brawl Probe",
            "Choose target creature you control and target creature you don't control. \
             If you control three or more snow permanents, the creature you control gets +1/+0 \
             and gains indestructible until end of turn. Then those creatures fight each other.",
        ),
        (
            "Ancient Animus Probe",
            "Put a +1/+1 counter on target creature you control if it's legendary. \
             Then it fights target creature an opponent controls.",
        ),
    ] {
        let definition = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Instant])
            .parse_text(text)
            .expect("conditional fight text should compile");
        let effects = definition
            .spell_effect
            .as_ref()
            .expect("conditional fight should be a spell effect")
            .flattened_default_effects();
        let [first_target, second_target, conditional, fight] = effects else {
            panic!("expected two targets, conditional action, and fight for {name}: {effects:#?}");
        };

        let target_tag = |effect: &Effect| {
            let mut effect = effect;
            loop {
                let tagged = effect.as_tagged().unwrap_or_else(|| {
                    panic!("each target declaration should be tagged: {effects:#?}")
                });
                if tagged
                    .effect
                    .downcast_ref::<ironsmith_compiler::effects::TargetOnlyEffect>()
                    .is_some()
                {
                    break tagged.tag.clone();
                }
                effect = &tagged.effect;
            }
        };
        let first_tag = target_tag(first_target);
        let second_tag = target_tag(second_target);
        assert_ne!(first_tag, second_tag, "{name}: {effects:#?}");

        fn contains_state_condition(effect: &Effect) -> bool {
            if effect
                .downcast_ref::<ironsmith_compiler::effects::ConditionalEffect>()
                .is_some()
                || effect
                    .downcast_ref::<ironsmith_compiler::effects::ApplyContinuousEffect>()
                    .is_some_and(|continuous| continuous.condition.is_some())
            {
                return true;
            }
            let mut found = false;
            effect.visit_child_effects(&mut |child| {
                found |= contains_state_condition(child);
            });
            found
        }
        assert!(
            contains_state_condition(conditional),
            "third effect should retain its state condition for {name}: {effects:#?}"
        );
        let fight = fight
            .downcast_ref::<ironsmith_compiler::effects::FightEffect>()
            .expect("fourth effect should be a fight");
        let (ChooseSpec::Tagged(fight_first), ChooseSpec::Tagged(fight_second)) =
            (&fight.creature1, &fight.creature2)
        else {
            panic!("{name}: fight participants should be stable target tags");
        };
        assert!(
            (fight_first == &first_tag || fight_first == &second_tag)
                && (fight_second == &first_tag || fight_second == &second_tag),
            "{name}: fight participants should reuse the two declared target tags"
        );
        assert_ne!(
            fight_first, fight_second,
            "{name}: fight participants should be distinct"
        );
    }
}

#[test]
fn cross_sentence_two_target_power_damage_reuses_both_target_slots() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Targeted Power Damage Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Choose target creature you control and target creature an opponent controls.\n\
             Delirium — If there are four or more card types among cards in your graveyard, put \
             two +1/+1 counters on the creature you control.\n\
             The creature you control deals damage equal to its power to the creature an \
             opponent controls.",
        )
        .expect("the correlated target procedure should compile");
    let effects = definition
        .spell_effect
        .as_ref()
        .expect("expected a spell effect")
        .flattened_default_effects();
    let [first_target, second_target, conditional, damage] = effects else {
        panic!("expected two targets, a conditional modifier, and damage: {effects:#?}");
    };

    let target_tag = |effect: &Effect| {
        let mut effect = effect;
        loop {
            let tagged = effect
                .as_tagged()
                .unwrap_or_else(|| panic!("target declaration should be tagged: {effect:#?}"));
            if tagged
                .effect
                .downcast_ref::<ironsmith_compiler::effects::TargetOnlyEffect>()
                .is_some()
            {
                break tagged.tag.clone();
            }
            effect = &tagged.effect;
        }
    };
    let first_tag = target_tag(first_target);
    let second_tag = target_tag(second_target);
    assert_ne!(first_tag, second_tag);

    let conditional = conditional
        .downcast_ref::<ironsmith_compiler::effects::ConditionalEffect>()
        .expect("the third effect should remain conditional");
    let [counter_effect] = conditional.if_true.as_slice() else {
        panic!("expected one conditional counter effect: {conditional:#?}");
    };
    let counters = counter_effect
        .as_tagged()
        .and_then(|tagged| {
            tagged
                .effect
                .downcast_ref::<ironsmith_compiler::effects::PutCountersEffect>()
        })
        .expect("conditional branch should put counters");
    assert!(
        matches!(&counters.target, ChooseSpec::Tagged(tag) if tag == &first_tag),
        "the counters must reuse the controlled target: {counters:#?}"
    );

    let damage = damage
        .as_tagged()
        .expect("the damage recipient should retain its result tag");
    let with_source = damage
        .effect
        .downcast_ref::<ironsmith_compiler::effects::ExecuteWithSourceEffect>()
        .expect("the controlled target should execute the damage");
    assert!(
        matches!(&with_source.source, ChooseSpec::Tagged(tag) if tag == &first_tag),
        "damage source must reuse the controlled target: {with_source:#?}"
    );
    let damage = with_source
        .effect
        .downcast_ref::<ironsmith_compiler::effects::DealDamageEffect>()
        .expect("expected power-based damage");
    assert!(
        matches!(&damage.target, ChooseSpec::Tagged(tag) if tag == &second_tag)
            && matches!(
                damage.amount.unhinted(),
                Value::PowerOf(spec) if matches!(spec.unhinted(), ChooseSpec::Source)
            ),
        "damage must use the first target's power against the second target: {damage:#?}"
    );
}

#[test]
fn qualified_power_damage_clause_builds_canonical_ast_directly() {
    let tokens = ironsmith_compiler::lexer::lex_line(
        "The creature you control deals damage equal to its power to the creature an opponent controls.",
        0,
    )
    .expect("qualified power damage should lex");
    let effect =
        ironsmith_compiler::effect_sentences::parse_deal_damage_equal_to_power_clause(&tokens)
            .expect("qualified power damage clause should parse")
            .expect("qualified power damage clause should match");
    assert!(matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::DealDamageEqualToPower { .. },
            ..
        })
    ));
}

#[test]
fn qualified_power_damage_canonical_ast_lowers_directly() {
    let tokens = ironsmith_compiler::lexer::lex_line(
        "The creature you control deals damage equal to its power to the creature an opponent controls.",
        0,
    )
    .expect("qualified power damage should lex");
    let effect =
        ironsmith_compiler::effect_sentences::parse_deal_damage_equal_to_power_clause(&tokens)
            .expect("qualified power damage clause should parse")
            .expect("qualified power damage clause should match");
    compile_statement_effects_with_imports(&[effect], &ReferenceImports::default())
        .expect("the canonical damage effect should lower");
}

#[test]
fn inline_kicked_search_replacement_preserves_leading_instead_surface() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Additive Search Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Kicker {2}\n\
             Search your library for a basic land card. If this spell was kicked, instead search \
             your library for a basic land card and a Shrine card. Reveal those cards, put them \
             into your hand, then shuffle.",
        )
        .expect("the additive kicked search should compile");
    let program = definition.spell_effect.expect("expected a spell effect");
    let [segment] = program.segments.as_slice() else {
        panic!("expected one search segment: {program:#?}");
    };
    let [branch] = segment.self_replacements.as_slice() else {
        panic!("expected one kicked self replacement: {segment:#?}");
    };
    assert_eq!(branch.condition, Condition::ThisSpellWasKicked);
    assert!(
        branch.leading_instead_surface,
        "the explicit `instead search` connective must survive line lowering: {branch:#?}"
    );
}

#[test]
fn compile_seat_relative_control_preserves_the_explicit_right_player() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Control Pass Probe")
        .parse_text(
            "{2}, {T}: Draw a card. The player to your right gains control of this artifact.",
        )
        .expect("seat-relative control should parse");
    let debug = format!("{:#?}", def.abilities);

    assert!(
        debug.contains("ChangeControllerToPlayer")
            && debug.contains("PlayerToYourRight")
            && !debug.contains("IteratedPlayer"),
        "the control recipient should retain its authored seat relation: {debug}"
    );
}

#[test]
fn compile_each_other_becomes_copy_uses_prior_chosen_object_as_copy_source() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Copy Choice Probe")
        .parse_text(
            "Choose a creature you control. Each other creature you control becomes a copy of that creature until end of turn.",
        )
        .expect("choice followed by a mass copy effect should parse");

    let debug = format!("{:#?}", def.spell_effect.expect("spell effect"));
    assert!(debug.contains("ChooseObjectsEffect"), "{debug}");
    assert!(debug.contains("CopyOf"), "{debug}");
    assert!(debug.contains("IsNotTaggedObject"), "{debug}");
    assert!(
        debug.contains("source: Tagged(")
            && debug.contains("__chosen_objects__")
            && !debug.contains("source: Iterated"),
        "the copy source must remain the prior choice, not the current mass-effect iteration: {debug}"
    );
    assert!(
        !debug.contains("ForEachObject"),
        "a single continuous effect should lock the affected set at resolution: {debug}"
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
    let rules_text = ironsmith_compiler::grammar::token_definitions::parse_equipment_rules_tokens(
        &source_tokens,
    )
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
    let shape = token_definition_shape_text(source_text)
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
    let shape = token_definition_shape_text(source_text)
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
            [
                ironsmith_compiler::costs::Cost::Tap,
                ironsmith_compiler::costs::Cost::SacrificeSelf
            ]
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
fn comma_then_dynamic_token_keeps_pt_tag_and_delayed_cleanup() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dynamic Token Sequence Probe")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "At the beginning of combat on your turn, put an oil counter on this artifact, then create an X/1 red Phyrexian Horror creature token with trample and haste, where X is the number of oil counters on this artifact. Sacrifice that token at the beginning of the next end step.",
        )
        .expect("dynamic token sequence should parse");
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected a triggered ability");
    let [sequence_effect] = triggered.effects.segments[0].default_effects.as_slice() else {
        panic!("expected one authored sequence");
    };
    let sequence = sequence_effect
        .downcast_ref::<ironsmith_compiler::effects::SequenceEffect>()
        .expect("expected comma-then sequence");
    assert_eq!(sequence.surface, ironsmith_core::SequenceSurface::CommaThen);
    let [_, create_effect, set_pt_effect] = sequence.effects.as_slice() else {
        panic!("expected counter, token creation, and dynamic P/T setter");
    };
    let tagged_create = create_effect
        .as_tagged()
        .expect("created token should carry an identity tag");
    let create = tagged_create
        .effect
        .downcast_ref::<ironsmith_compiler::effects::CreateTokenEffect>()
        .expect("expected token creation");
    assert!(create.sacrifice_at_next_end_step);

    let set_pt = set_pt_effect
        .downcast_ref::<ironsmith_compiler::effects::SetBasePowerToughnessEffect>()
        .expect("expected dynamic base-P/T setter");
    assert!(
        matches!(&set_pt.target, ChooseSpec::Tagged(tag) if tag == &tagged_create.tag),
        "P/T setter must target the exact created token: {set_pt:#?}"
    );
    assert!(
        matches!(
            set_pt.power.unhinted(),
            Value::CountersOn(source, Some(ironsmith_compiler::object::CounterType::Oil))
                if matches!(source.base(), ChooseSpec::Source)
        ),
        "power must remain the source's oil-counter count: {set_pt:#?}"
    );
    assert_eq!(set_pt.toughness.unhinted(), &Value::Fixed(1));
}

#[test]
fn dynamic_token_stats_after_destroy_keep_the_destroyed_object_reference() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dynamic Token Destroy Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{B}, {T}: Destroy target creature. If that creature dies this way, create a black Vampire creature token. Its power is equal to that creature's power and its toughness is equal to that creature's toughness.",
        )
        .expect("destroy-linked dynamic token P/T should parse");
    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("expected an activated ability");
    let conditional = activated
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith_compiler::effects::IfEffect>())
        .unwrap_or_else(|| {
            panic!(
                "expected a death-result condition: {:#?}",
                activated.effects
            )
        });
    let set_pt = conditional.then[1]
        .downcast_ref::<ironsmith_compiler::effects::SetBasePowerToughnessEffect>()
        .expect("expected the created token's dynamic P/T effect");
    for value in [&set_pt.power, &set_pt.toughness] {
        let spec = match value {
            Value::PowerOf(spec) | Value::ToughnessOf(spec) => spec,
            other => panic!("expected a copied characteristic value, got {other:#?}"),
        };
        assert!(
            matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == "destroyed_0"),
            "the token characteristic must use the destroyed target, got {spec:#?}"
        );
    }
}

#[test]
fn payload_type_metadata_seeds_source_reference_context_before_rules_lines() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Metadata Context Probe")
        .parse_text(
            "Type: Enchantment\n\
             When a player doesn't pay this enchantment's cumulative upkeep, draw a card.",
        )
        .expect("payload metadata should seed typed source references");
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("CumulativeUpkeepNotPaid"),
        "the typed possessive source reference must survive payload metadata parsing: {debug}"
    );
}

#[test]
fn triggered_shared_dynamic_damage_keeps_target_controller_fanout() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dynamic Triggered Damage Probe")
        .parse_text(
            "Type: Enchantment\n\
             Cumulative upkeep {2}\n\
             When a player doesn't pay this enchantment's cumulative upkeep, this enchantment \
             deals X damage to target player or planeswalker and each creature that player or \
             that planeswalker's controller controls, where X is twice the number of age counters \
             on this enchantment minus 2.",
        )
        .expect("triggered shared dynamic damage should parse");
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("CumulativeUpkeepNotPaid"),
        "the unpaid-upkeep trigger must remain typed when paired with the keyword line: {debug}"
    );
    assert!(
        debug.contains("TargetPlayerOrControllerOfTarget"),
        "the creature fanout must remain linked to the damage target: {debug}"
    );
    assert!(
        debug.contains("Scaled(") && debug.contains("Age") && debug.contains("-2"),
        "the shared dynamic amount must retain its arithmetic expression: {debug}"
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
    let shape = token_definition_shape_text(source_text)
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
    let shape = token_definition_shape_text(source_text)
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
        .flattened_default_effects()
        .find_create_token_effect()
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
                .find_create_token_effect(),
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
        ironsmith_compiler::triggers::TriggerKind::ThisBlocksObject { filter, .. }
            if filter.card_types.as_slice() == [CardType::Creature]
    ));

    let cant = find_cant_effect(nested.effects.flattened_default_effects())
        .expect("nested trigger should apply an untap restriction");
    assert!(matches!(
        &cant.restriction,
        ironsmith_compiler::effect::Restriction::Untap(_)
    ));
    assert_eq!(
        cant.duration,
        ironsmith_compiler::effect::Until::ControllersNextUntapStep
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
        .flattened_default_effects()
        .find_create_token_effect()
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
        .flattened_default_effects()
        .find_create_token_effect()
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
        .flattened_default_effects()
        .find_create_token_effect()
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
    assert!(
        debug.matches("PutCountersEffect").count() >= 2
            && debug.contains("PlusOnePlusOne")
            && debug.contains("CountersOn")
            && debug.contains("\"invasion\"")
            && debug.contains("created_token"),
        "full-card production dispatch must retain both linked counter clauses: {debug}"
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
        .flattened_default_effects()
        .find_create_token_effect()
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
        .flattened_default_effects()
        .find_create_token_effect()
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
        .flattened_default_effects()
        .find_create_token_effect()
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
        .flattened_default_effects()
        .find_create_token_effect()
        .expect("created Etherium Cell token");
    let activated = create
        .token
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith_compiler::ability::AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Etherium Cell activated mana ability");
    assert!(matches!(
        activated.mana_cost.costs(),
        [
            ironsmith_compiler::costs::Cost::Tap,
            ironsmith_compiler::costs::Cost::SacrificeSelf
        ]
    ));
    assert!(
        format!("{:#?}", activated.effects).contains("AddManaOfAnyColorEffect"),
        "{activated:#?}"
    );
}

#[test]
fn builtin_food_blood_and_powerstone_tokens_keep_their_intrinsic_abilities() {
    for (name, expected_costs, effect_marker) in [
        ("Food", 3, "GainLifeEffect"),
        ("Blood", 4, "DrawCardsEffect"),
        ("Powerstone", 1, "AddManaEffect"),
    ] {
        let def = CardDefinitionBuilder::new(CardId::new(), format!("{name} Token Probe"))
            .card_types(vec![CardType::Sorcery])
            .parse_text(format!("Create a {name} token."))
            .unwrap_or_else(|error| panic!("{name} token text should parse: {error}"));
        let create = def
            .spell_effect
            .as_ref()
            .expect("token probe should have spell effects")
            .flattened_default_effects()
            .find_create_token_effect()
            .unwrap_or_else(|| panic!("{name} should lower to token creation"));
        let activated = create
            .token
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(activated),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{name} should retain its intrinsic activated ability"));
        assert_eq!(
            activated.mana_cost.costs().len(),
            expected_costs,
            "{name} intrinsic cost was incomplete: {activated:#?}"
        );
        assert!(
            format!("{:#?}", activated.effects).contains(effect_marker),
            "{name} should retain {effect_marker}: {activated:#?}"
        );
        if name == "Powerstone" {
            assert_eq!(activated.mana_usage_restrictions.len(), 1);
        }
    }
}

#[test]
fn builtin_sorcerer_role_keeps_its_buff_and_granted_attack_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sorcerer Role Token Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Create a Sorcerer Role token attached to target creature you control.")
        .expect("Sorcerer Role token text should parse");
    let create = def
        .spell_effect
        .as_ref()
        .expect("token probe should have spell effects")
        .flattened_default_effects()
        .find_create_token_effect()
        .expect("Sorcerer Role should lower to token creation");
    let static_ids = create
        .token
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => static_ability.id,
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        static_ids.contains(&ironsmith_compiler::static_abilities::StaticAbilityId::Anthem),
        "Sorcerer Role should give the enchanted creature +1/+1: {create:#?}"
    );
    assert!(
        static_ids
            .contains(&ironsmith_compiler::static_abilities::StaticAbilityId::AttachedAbilityGrant),
        "Sorcerer Role should grant its attack trigger: {create:#?}"
    );
    assert!(format!("{create:#?}").contains("ScryEffect"));
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
        .flattened_default_effects()
        .find_create_token_effect()
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
    let (parsed, trace) = ironsmith_compiler::parse_trace::capture(|| {
        CardDefinitionBuilder::new(CardId::new(), "Tezzeret the Schemer")
            .card_types(vec![CardType::Planeswalker])
            .loyalty(5)
            .parse_text(
                "+1: Create a colorless artifact token named Etherium Cell with \"{T}, Sacrifice this token: Add one mana of any color.\"\n−2: Target creature gets +X/-X until end of turn, where X is the number of artifacts you control.\n−7: You get an emblem with \"At the beginning of combat on your turn, target artifact you control becomes an artifact creature with base power and toughness 5/5.\"",
            )
    });
    let def = parsed.expect("full Tezzeret the Schemer text should parse");
    let create = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated
                .effects
                .flattened_default_effects()
                .find_create_token_effect(),
            _ => None,
        })
        .unwrap_or_else(|| {
            panic!(
                "Tezzeret's +1 should create an Etherium Cell:\n{}\nabilities: {:#?}",
                trace.render(),
                def.abilities,
            )
        });

    assert_eq!(
        create.token.abilities.len(),
        2,
        "the colorless marker and specialized mana rule should each occur once: {create:#?}"
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
        [
            ironsmith_compiler::costs::Cost::Tap,
            ironsmith_compiler::costs::Cost::SacrificeSelf
        ]
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
                .find_create_token_effect(),
            _ => None,
        })
        .expect("Toggo's landfall ability should create a Rock");

    assert_eq!(
        create.token.abilities.len(),
        3,
        "Rock should own one colorless marker, one attached grant, and one equip ability: {create:#?}"
    );
    let debug = format!("{:#?}", create.token.abilities);
    assert_eq!(
        create
            .token
            .abilities
            .iter()
            .filter(|ability| matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id
                        == Some(ironsmith_compiler::static_abilities::StaticAbilityId::AttachedAbilityGrant)
            ))
            .count(),
        1,
        "{debug}"
    );
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
        .flattened_default_effects()
        .find_create_token_effect()
        .expect("created Notebook token");
    let debug = format!("{:#?}", create.token.abilities);
    assert!(
        debug.contains("DrawCardsEffect"),
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
            && debug.contains("DrawCardsEffect"),
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
        .flattened_default_effects()
        .find_create_token_effect()
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
            ironsmith_compiler::target::SourceReferenceSurface::ThisPermanentType(
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
            ironsmith_compiler::target::ChooseSpecSurfaceHint::SourceReference(
                ironsmith_compiler::target::SourceReferenceSurface::ThisPermanentType(
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
fn predicate_tag_detection_descends_through_surface_hinted_characteristic_specs() {
    let referenced_target =
        ChooseSpec::Tagged(ironsmith_compiler::tag::CompilerReferenceTag::It.key())
            .with_surface_hint(
                ironsmith_compiler::target::ChooseSpecSurfaceHint::SourceReference(
                    ironsmith_compiler::target::SourceReferenceSurface::ThisPermanentType(
                        "it".to_string(),
                    ),
                ),
            );
    let predicate = PredicateAst::ValueComparison {
        left: Value::ManaValueOf(Box::new(referenced_target)),
        operator: ironsmith_compiler::effect::ValueComparisonOperator::LessThanOrEqual,
        right: Value::Fixed(2),
    };

    assert!(
        predicate_references_tag(
            &predicate,
            ironsmith_compiler::tag::CompilerReferenceTag::It.as_str()
        ),
        "surface metadata must not hide the target reference used to hoist a trailing condition"
    );
}

fn target_mana_value_threshold_branch(target_span: TextSpan, threshold: i32) -> EffectAst {
    EffectAst::TrailingIf {
        predicate: PredicateAst::ValueComparison {
            left: Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
                ironsmith_compiler::tag::CompilerReferenceTag::It.key(),
            ))),
            operator: ironsmith_compiler::effect::ValueComparisonOperator::LessThanOrEqual,
            right: Value::Fixed(threshold),
        },
        effects: vec![EffectAst::subject_verb_destroy(TargetAst::Object(
            ObjectFilter::artifact(),
            Some(target_span),
            None,
        ))],
    }
}

fn threshold_self_replacement(
    default_target_span: TextSpan,
    replacement_target_span: TextSpan,
) -> EffectAst {
    EffectAst::SelfReplacement {
        predicate: PredicateAst::ThisSpellWasKicked,
        if_true: vec![target_mana_value_threshold_branch(
            replacement_target_span,
            5,
        )],
        if_false: vec![target_mana_value_threshold_branch(default_target_span, 2)],
        attach_to_previous_ability: false,
    }
}

#[test]
fn self_replacement_reuses_a_copied_target_declaration_across_threshold_branches() {
    let shared_span = TextSpan {
        line: 0,
        start: 8,
        end: 23,
    };
    let lowered = compile_statement_effects_with_imports(
        &[threshold_self_replacement(shared_span, shared_span)],
        &ReferenceImports::default(),
    )
    .expect("shared-target self-replacement should lower");
    let debug = format!("{lowered:#?}");

    assert_eq!(debug.matches("TargetOnlyEffect").count(), 1, "{debug}");
    assert_eq!(debug.matches("ManaValueOf").count(), 2, "{debug}");
    assert!(!debug.contains("spec: Source"), "{debug}");
}

#[test]
fn self_replacement_keeps_separately_authored_equal_target_filters_distinct() {
    let lowered = compile_statement_effects_with_imports(
        &[threshold_self_replacement(
            TextSpan {
                line: 0,
                start: 8,
                end: 23,
            },
            TextSpan {
                line: 1,
                start: 35,
                end: 50,
            },
        )],
        &ReferenceImports::default(),
    )
    .expect("distinct-target self-replacement should lower");
    let debug = format!("{lowered:#?}");

    assert_eq!(
        debug.matches("TargetOnlyEffect").count(),
        2,
        "equal target filters from distinct declaration spans must not be merged: {debug}"
    );
}

#[test]
fn source_sacrifice_preserves_its_authored_permanent_noun() {
    let surface = ironsmith_compiler::target::SourceReferenceSurface::ThisPermanentType(
        "this permanent".to_string(),
    );
    let sacrifice = EffectAst::subject_verb_sacrifice(
        PlayerAst::You,
        ObjectFilter::source().with_source_surface(surface.clone()),
        1,
        None,
    );
    let mut ctx = EffectLoweringContext::new();
    let (effects, choices) =
        compile_effects(&[sacrifice], &mut ctx).expect("source sacrifice should lower");
    let lowered = effects[0]
        .downcast_ref::<ironsmith_compiler::effects::SacrificeTargetEffect>()
        .expect("source sacrifice should remain a typed sacrifice");

    assert_eq!(
        lowered.target,
        ChooseSpec::Source.with_surface_hint(
            ironsmith_compiler::target::ChooseSpecSurfaceHint::SourceReference(surface),
        )
    );
    assert!(choices.is_empty());
}

#[test]
fn resolve_target_spec_preserves_implicit_it_when_it_resolves_to_source() {
    let target = TargetAst::Tagged(
        ironsmith_compiler::tag::CompilerReferenceTag::It.key(),
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
            ironsmith_compiler::target::ChooseSpecSurfaceHint::SourceReference(
                ironsmith_compiler::target::SourceReferenceSurface::ThisPermanentType(
                    "it".to_string()
                ),
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
    NormalizedLine::identity(line)
}

fn span_mapping_context(line: &NormalizedLine) -> SpanMappingContext<'_> {
    SpanMappingContext::new(&line.normalized, &line.original, &line.char_map)
}

#[test]
fn collect_tag_spans_tracks_connive_and_destroy_no_regeneration_targets() {
    let mut annotations = ParseAnnotations::default();
    let line = test_ctx("alpha beta");
    let ctx = span_mapping_context(&line);
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
    let line = test_ctx("gamma");
    let ctx = span_mapping_context(&line);
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
        ironsmith_core::TotalCost::<ironsmith_compiler::model::CompilerCost>::free(),
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
            ironsmith_compiler::tag::CompilerReferenceTag::It.key(),
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
    assert_eq!(
        frame_out.last_object_tag.as_ref().map(|tag| tag.as_str()),
        Some("destroyed_0")
    );
}

#[test]
fn synthesis_pod_consult_match_keeps_its_tag_through_exile_and_cast() {
    let match_tag = TagKey::from("__sentence_helper_consult_match_l0_s0_e0");
    let effects = vec![
        EffectAst::subject_verb_consult_top_of_library(
            PlayerAst::You,
            ironsmith_compiler::cards::builders::LibraryConsultModeAst::Reveal,
            ObjectFilter::default(),
            ironsmith_compiler::cards::builders::LibraryConsultStopRuleAst::MatchCount(
                Value::Fixed(1),
            ),
            TagKey::from("__sentence_helper_revealed_l0_s0_e0"),
            match_tag.clone(),
        ),
        EffectAst::subject_verb_exile(
            TargetAst::Tagged(match_tag.clone(), Some(TextSpan::synthetic())),
            false,
        ),
        EffectAst::subject_verb_cast_tagged(
            ironsmith_compiler::tag::CompilerReferenceTag::It.key(),
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
        .find_map(|effect| effect.downcast_ref::<ironsmith_compiler::effects::CastTaggedEffect>())
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
            count: ironsmith_compiler::effect::ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: searched_tag.clone(),
        },
        EffectAst::subject_verb_exile(
            TargetAst::Tagged(searched_tag.clone(), Some(TextSpan::synthetic())),
            true,
        ),
        EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
            ironsmith_compiler::tag::CompilerReferenceTag::It.key(),
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
        .find_map(|effect| {
            effect.downcast_ref::<ironsmith_compiler::effects::GrantPlayTaggedEffect>()
        })
        .expect("searched exiled card should receive a play permission");
    assert_eq!(
        grant.tag.as_str(),
        ironsmith_compiler::tag::CompilerReferenceTag::SourceExiled.as_str()
    );
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
            .downcast_ref::<ironsmith_compiler::effects::cards::ImprintFromHandEffect>()
            .is_none()),
        "controller-hand-only imprint must not lower an opponent-hand choice: {compiled:#?}"
    );
    let may = compiled
        .iter()
        .find_map(|effect| {
            effect.downcast_ref::<ironsmith_compiler::effects::MayEffect<ironsmith_compiler::effect::Effect>>()
        })
        .expect("opponent-hand exile should retain its optional branch");
    assert!(
        may.effects.iter().any(|effect| effect
            .downcast_ref::<ironsmith_compiler::effects::ChooseObjectsEffect>()
            .is_some()),
        "optional branch must choose from the referenced hand: {may:#?}"
    );
    assert!(
        may.effects.iter().any(|effect| effect
            .downcast_ref::<ironsmith_compiler::effects::ExileEffect>()
            .is_some()),
        "optional branch must exile the chosen card: {may:#?}"
    );
}

#[test]
fn singular_untargeted_causative_damage_remains_one_object_choice() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Causative Damage Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, you may have it deal 2 damage to another creature you control. If you do, draw a card.",
        )
        .expect("singular causative damage should compile");
    let debug = format!("{:#?}", definition.abilities);

    assert!(debug.contains("MayEffect"), "{debug}");
    assert!(debug.contains("DealDamageEffect"), "{debug}");
    assert!(debug.contains("ExecuteWithSourceEffect"), "{debug}");
    assert!(debug.contains("other: true"), "{debug}");
    assert!(debug.contains("target: Object("), "{debug}");
    assert!(
        !debug.contains("ForEachObject"),
        "a singular untargeted object phrase must not widen into a mass effect: {debug}"
    );
    assert!(debug.contains("DrawCardsEffect"), "{debug}");
}

#[test]
fn plural_untargeted_causative_damage_still_fans_out() {
    let mut filter = ObjectFilter::creature().you_control();
    filter.set_plural_object_noun_surface(true);
    let ast = EffectAst::subject_verb_damage_with_source(
        TargetAst::Source(None),
        Value::Fixed(2),
        TargetAst::Object(filter, None, None),
    );

    let (compiled, _) = compile_effect(&ast, &mut EffectLoweringContext::new())
        .expect("plural causative damage should lower");
    let debug = format!("{compiled:#?}");
    assert!(debug.contains("ForEachObject"), "{debug}");
    assert!(debug.contains("DealDamageEffect"), "{debug}");
}

#[test]
fn optional_self_prompting_free_cast_does_not_gain_a_second_may_prompt() {
    let ast = EffectAst::MayByPlayer {
        player: PlayerAst::You,
        effects: vec![
            EffectAst::may_cast_matching_spell_without_paying_mana_cost_from_zone_owner(
                PlayerAst::You,
                PlayerAst::That,
                ObjectFilter::nonland().in_zone(Zone::Hand),
                Zone::Hand,
            ),
        ],
    };

    let (compiled, _) = compile_effect(&ast, &mut EffectLoweringContext::new())
        .expect("optional free cast should lower");
    assert!(
        matches!(
            compiled.as_slice(),
            [effect]
                if effect
                    .downcast_ref::<ironsmith_compiler::effects::MayCastMatchingSpellWithoutPayingManaCostEffect>()
                    .is_some()
        ),
        "the cast effect owns the one optional choice and must not be nested in MayEffect: {compiled:#?}"
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
            ironsmith_compiler::tag::CompilerReferenceTag::It.key(),
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
    assert_eq!(
        frame_out.last_object_tag.as_ref().map(|tag| tag.as_str()),
        Some("destroyed_0")
    );
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
    target_filter.stack_kind = Some(ironsmith_compiler::filter::StackObjectKind::Spell);
    let mut legendary_spell = ObjectFilter::spell();
    legendary_spell.supertypes = vec![ironsmith_compiler::types::Supertype::Legendary];
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
fn compile_live_permanent_spell_predicate_preserves_stack_identity() {
    let permanent_spell = ObjectFilter {
        card_types: vec![
            CardType::Artifact,
            CardType::Creature,
            CardType::Enchantment,
            CardType::Planeswalker,
            CardType::Battle,
        ],
        zone: Some(Zone::Stack),
        stack_kind: Some(ironsmith_compiler::filter::StackObjectKind::Spell),
        ..ObjectFilter::default()
    };

    let condition = compile_condition_from_predicate_ast(
        &PredicateAst::ItMatches(permanent_spell),
        &mut EffectLoweringContext::new(),
        &Some(ironsmith_compiler::tag::CompilerReferenceTag::CopiedStackObject.key()),
    )
    .expect("live permanent-spell predicate should lower");
    let Condition::TaggedObjectMatches(tag, filter) = condition else {
        panic!("expected tagged copied-spell condition");
    };
    assert_eq!(
        tag.as_str(),
        ironsmith_compiler::tag::CompilerReferenceTag::CopiedStackObject.as_str()
    );
    assert_eq!(filter.zone, Some(Zone::Stack));
    assert_eq!(
        filter.stack_kind,
        Some(ironsmith_compiler::filter::StackObjectKind::Spell)
    );
}

#[test]
fn compile_copy_does_not_replace_the_original_pronoun_antecedent() {
    let effects = vec![EffectAst::subject_verb_copy_spell(
        TargetAst::Tagged(
            ironsmith_compiler::tag::CompilerReferenceTag::Triggering.key(),
            None,
        ),
        Value::Fixed(1),
        PlayerAst::You,
        true,
        false,
        Vec::new(),
    )];
    let mut ctx = EffectLoweringContext::new();
    ctx.last_object_tag = Some(TagKey::from("original_spell"));
    compile_effects(&effects, &mut ctx).expect("copy should lower");
    assert_eq!(
        ctx.last_object_tag.as_ref().map(|tag| tag.as_str()),
        Some("original_spell")
    );
}

#[test]
fn compile_for_each_tagged_rewrites_it_targets_to_iterated_object() {
    let effects = vec![EffectAst::ForEachTagged {
        tag: TagKey::from("revealed_0"),
        effects: vec![EffectAst::Conditional {
            predicate: PredicateAst::ItMatches(ObjectFilter::permanent()),
            if_true: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(
                    ironsmith_compiler::tag::CompilerReferenceTag::It.key(),
                    None,
                ),
                Zone::Battlefield,
                false,
                ReturnControllerAst::Owner,
                false,
                None,
            )],
            if_false: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(
                    ironsmith_compiler::tag::CompilerReferenceTag::It.key(),
                    None,
                ),
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
fn consult_inside_for_each_tagged_does_not_steal_the_iteration_binding() {
    let effects = vec![EffectAst::ForEachTagged {
        tag: TagKey::from("chosen_targets"),
        effects: vec![
            EffectAst::subject_verb_consult_top_of_library(
                PlayerAst::You,
                ironsmith_compiler::cards::builders::LibraryConsultModeAst::Reveal,
                ObjectFilter::default().without_type(CardType::Land),
                ironsmith_compiler::cards::builders::LibraryConsultStopRuleAst::FirstMatch,
                TagKey::from("revealed"),
                TagKey::from("matched"),
            ),
            EffectAst::subject_verb(
                SubjectVerbRoleAst::Actor,
                PlayerAst::Implicit,
                SubjectVerbActionAst::DealDamage {
                    amount: Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
                        ironsmith_compiler::tag::CompilerReferenceTag::It.key(),
                    ))),
                    target: TargetAst::Tagged(
                        ironsmith_compiler::tag::CompilerReferenceTag::It.key(),
                        None,
                    ),
                    unpreventable: false,
                },
            ),
        ],
    }];

    let (compiled, _, _) = compile_effects_with_explicit_frame(
        &effects,
        &mut IdGenContext::default(),
        LoweringFrame::default(),
    )
    .expect("compile consult inside for-each-tagged");

    let for_each = compiled[0]
        .downcast_ref::<ironsmith_compiler::effects::ForEachTaggedEffect<ironsmith_compiler::effect::Effect>>()
        .expect("outer tagged-object iterator");
    let damage = for_each.effects[1]
        .downcast_ref::<ironsmith_compiler::effects::DealDamageEffect>()
        .expect("damage inside tagged-object iterator");
    assert!(matches!(damage.target.base(), ChooseSpec::Iterated));
    assert!(matches!(
        damage.amount.unhinted(),
        Value::ManaValueOf(spec)
            if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == "matched")
    ));

    let debug = format!("{compiled:#?}");
    assert!(
        debug.contains("ConsultTopOfLibraryEffect") && debug.contains("target: Iterated"),
        "the found card may update antecedent memory, but damage must still bind to the current loop member: {debug}"
    );
}

#[test]
fn compile_next_spell_grant_after_targeted_player_effect_binds_that_player() {
    let effects = vec![
        EffectAst::subject_verb_add_mana_any_one_color(PlayerAst::Target, Value::Fixed(2)),
        EffectAst::subject_verb_grant_next_spell_ability_this_turn(
            PlayerAst::That,
            ObjectFilter::spell().cast_by(PlayerFilter::IteratedPlayer),
            ironsmith_compiler::cards::builders::KeywordAction::Cascade.into(),
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
        debug.contains("SacrificePlayerEffect"),
        "sacrifice effect: {debug}"
    );
    assert!(
        debug.contains("GainLifeEffect"),
        "life-gain effect: {debug}"
    );
    assert!(
        debug.contains("player: Player(AliasedTarget(Any))"),
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
        debug.contains("ChooseObjectsEffect") && debug.contains("is_search: true"),
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
        ironsmith_compiler::cards::builders::KeywordAction::Cascade.into(),
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

#[test]
fn relative_cards_in_hand_value_binds_to_target_subject() {
    let mut value = Value::CardsInHand(PlayerFilter::IteratedPlayer)
        .with_surface_hint(ironsmith_core::ValueSurfaceHint::AllCardsInHand);

    bind_relative_iterated_player_in_value_to_player_filter(
        &mut value,
        &PlayerFilter::target_opponent(),
    );

    assert!(matches!(
        value.unhinted(),
        Value::CardsInHand(PlayerFilter::AliasedTarget(player))
            if matches!(player.as_ref(), PlayerFilter::Opponent)
    ));
}

#[test]
fn relative_hand_owner_does_not_turn_a_chosen_player_into_an_unannounced_target() {
    let mut value = Value::Count(ObjectFilter::default().in_zone(Zone::Hand).owned_by(
        PlayerFilter::AliasedTarget(Box::new(PlayerFilter::IteratedPlayer)),
    ));

    bind_relative_iterated_player_in_value_to_player_filter(
        &mut value,
        &PlayerFilter::ChosenPlayer,
    );

    assert!(matches!(
        value.unhinted(),
        Value::Count(filter) if filter.owner == Some(PlayerFilter::ChosenPlayer)
    ));

    let unresolved = Value::Count(ObjectFilter::default().in_zone(Zone::Hand).owned_by(
        PlayerFilter::AliasedTarget(Box::new(PlayerFilter::IteratedPlayer)),
    ));
    let refs = ReferenceEnv {
        last_player_filter: RefState::Known(PlayerFilter::ChosenPlayer),
        ..ReferenceEnv::default()
    };
    let resolved = resolve_value_it_tag(&unresolved, &refs)
        .expect("a relative possessive should resolve against the chosen player");
    assert!(matches!(
        resolved.unhinted(),
        Value::Count(filter) if filter.owner == Some(PlayerFilter::ChosenPlayer)
    ));
}

#[test]
fn explicit_damage_target_binds_same_clause_that_player_value_in_iterated_context() {
    let your_hand = ObjectFilter::default()
        .in_zone(Zone::Hand)
        .owned_by(PlayerFilter::You);
    let that_players_hand = Value::Count(
        ObjectFilter::default()
            .in_zone(Zone::Hand)
            .owned_by(PlayerFilter::IteratedPlayer),
    )
    .with_surface_hint(ironsmith_core::ValueSurfaceHint::ThatPlayerPossessive);
    let amount = Value::Add(
        Box::new(Value::Count(your_hand)),
        Box::new(Value::Scaled(Box::new(that_players_hand), -1)),
    )
    .with_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs);
    let effects = vec![EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::DealDamage {
            amount,
            target: TargetAst::Player(PlayerFilter::Opponent, Some(TextSpan::synthetic())),
            unpreventable: false,
        },
    )];
    for frame in [
        LoweringFrame {
            // A controller-scoped trigger can import "you" as the older
            // discourse antecedent.
            last_player_filter: Some(PlayerFilter::You),
            ..Default::default()
        },
        LoweringFrame {
            // Trigger and loop lowering can instead carry an active outer
            // iterated-player scope.
            iterated_player: true,
            last_player_filter: Some(PlayerFilter::IteratedPlayer),
            ..Default::default()
        },
    ] {
        let (compiled, _, _) =
            compile_effects_with_explicit_frame(&effects, &mut IdGenContext::default(), frame)
                .expect("same-clause targeted damage should compile");
        let damage = compiled[0]
            .downcast_ref::<ironsmith_compiler::effects::DealDamageEffect>()
            .expect("expected damage effect");
        let Value::Add(left, right) = damage.amount.unhinted() else {
            panic!("expected dynamic subtraction amount: {:#?}", damage.amount);
        };
        assert!(matches!(
            left.unhinted(),
            Value::Count(filter) if filter.owner == Some(PlayerFilter::You)
        ));
        assert!(matches!(
            right.unhinted(),
            Value::Scaled(inner, -1)
                if inner.has_surface_hint(
                    ironsmith_core::ValueSurfaceHint::ThatPlayerPossessive
                ) && matches!(
                    inner.unhinted(),
                    Value::Count(filter)
                        if matches!(
                            filter.owner.as_ref(),
                            Some(PlayerFilter::AliasedTarget(player))
                                if matches!(player.as_ref(), PlayerFilter::Opponent)
                        )
                )
        ));
    }
}

#[test]
fn nonexplicit_damage_recipient_preserves_outer_iterated_player_value() {
    let that_players_hand = ObjectFilter::default()
        .in_zone(Zone::Hand)
        .owned_by(PlayerFilter::IteratedPlayer);
    let effects = vec![EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::DealDamage {
            amount: Value::Count(that_players_hand),
            target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            unpreventable: false,
        },
    )];
    let frame = LoweringFrame {
        iterated_player: true,
        last_player_filter: Some(PlayerFilter::IteratedPlayer),
        ..Default::default()
    };

    let (compiled, _, _) =
        compile_effects_with_explicit_frame(&effects, &mut IdGenContext::default(), frame)
            .expect("outer iterated-player damage should compile");
    let damage = compiled[0]
        .downcast_ref::<ironsmith_compiler::effects::DealDamageEffect>()
        .expect("expected damage effect");
    assert_eq!(
        damage.amount.unhinted(),
        &Value::Count(
            ObjectFilter::default()
                .in_zone(Zone::Hand)
                .owned_by(PlayerFilter::IteratedPlayer)
        )
    );
}

#[test]
fn serial_keyword_filters_survive_trigger_and_effect_comma_boundaries() {
    use ironsmith_compiler::static_abilities::StaticAbilityId::{
        DoubleStrike, FirstStrike, Haste, Vigilance,
    };

    let filter_tokens = ironsmith_compiler::util::tokenize_line(
        "creatures that have first strike, double strike, vigilance, and/or haste",
        0,
    );
    let parsed_filter =
        ironsmith_compiler::object_filters::parse_object_filter_lexed(&filter_tokens, false)
            .expect("serial keyword object filter should parse");
    assert_eq!(parsed_filter.any_of.len(), 4, "{parsed_filter:#?}");

    let definition = CardDefinitionBuilder::new(CardId::new(), "Keyword Filter Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "When this enchantment enters, it deals 1 damage to each creature that doesn't have first strike, double strike, vigilance, or haste.\nWhenever you attack with at least two creatures that have first strike, double strike, vigilance, and/or haste, transform this enchantment.",
        )
        .expect("serial keyword filters should remain one parsed clause");

    let damage_fanout = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .flat_map(|triggered| triggered.effects.flattened_default_effects())
        .find_map(|effect| effect.downcast_ref::<ironsmith_compiler::effects::ForEachObject>())
        .expect("entry damage should iterate over its complete filtered object domain");
    assert_eq!(
        damage_fanout.filter.excluded_static_abilities,
        vec![FirstStrike, DoubleStrike, Vigilance, Haste]
    );
    assert!(damage_fanout.filter.any_of.is_empty());
    let [damage] = damage_fanout.effects.as_slice() else {
        panic!("mass damage must have one per-object action: {damage_fanout:#?}");
    };
    let damage = damage
        .downcast_ref::<ironsmith_compiler::effects::TaggedEffect>()
        .map_or(damage, |tagged| &tagged.effect);
    let damage = damage
        .downcast_ref::<ironsmith_compiler::effects::ExecuteWithSourceEffect>()
        .map_or(damage, |sourced| &sourced.effect)
        .downcast_ref::<ironsmith_compiler::effects::DealDamageEffect>()
        .expect("mass damage iteration must execute typed damage");
    assert!(matches!(damage.target.base(), ChooseSpec::Iterated));

    let attacks = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => match &triggered.trigger.kind {
                ironsmith_compiler::triggers::TriggerKind::AttacksOneOrMoreWithMinTotal {
                    filter,
                    min_total_attackers,
                } => Some((filter, *min_total_attackers)),
                _ => None,
            },
            _ => None,
        })
        .next()
        .expect("attack-with trigger should retain its typed matcher");
    assert_eq!(attacks.1, 2);
    assert_eq!(attacks.0.any_of.len(), 4, "{:#?}", attacks.0);
    let keyword_branches = attacks
        .0
        .any_of
        .iter()
        .flat_map(|branch| branch.static_abilities.iter().copied())
        .collect::<Vec<_>>();
    assert_eq!(
        keyword_branches,
        vec![FirstStrike, DoubleStrike, Vigilance, Haste]
    );
}

#[test]
fn delegated_subset_and_other_reuse_exact_prior_target_collection() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Delegated Pair Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Choose up to two target creature cards in your graveyard. An opponent chooses one of them. Return that card to your hand. Return the other to the battlefield under your control. It gains haste. Exile it at the beginning of the next end step.",
        )
        .expect("delegated subset procedure should compile");
    let debug = format!("{:#?}", definition.spell_effect.expect("spell effect"));
    let compact = debug
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();

    assert!(
        compact.contains("__delegated_subset"),
        "the opponent's subset needs a tag distinct from the original target pool: {debug}"
    );
    assert!(
        compact.contains("IsNotTaggedObject") && compact.contains("battlefield_controller:You"),
        "the other card must be the exact pool-minus-subset object returned under your control: {debug}"
    );
    assert!(
        !compact.contains("AnyOtherTarget"),
        "the other card is not a fresh target: {debug}"
    );
    assert!(
        compact.contains("DelayedTriggeredEffect") || compact.contains("Delayed"),
        "the complement's scheduled exile must retain its next-end-step timing: {debug}"
    );
}

#[test]
fn conditional_delegated_subset_keeps_remainder_move_in_false_branch() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Conditional Partition Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Choose up to four target cards in your graveyard. If you control a Bolas planeswalker, return those cards to your hand. Otherwise, an opponent chooses two of them. Leave the chosen cards in your graveyard and put the rest into your hand.",
        )
        .expect("conditional delegated partition should compile");
    let debug = format!("{:#?}", definition.spell_effect.expect("spell effect"));
    let compact = debug
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();

    assert!(
        compact.contains("__delegated_subset"),
        "the opponent-selected keep set needs its own stable tag: {debug}"
    );
    assert!(
        compact.contains("ForEachTagged") && compact.contains("IsTaggedObject"),
        "the false branch must move exactly the original pool minus the kept subset: {debug}"
    );
    assert!(
        compact.contains("Bolas"),
        "the planeswalker subtype qualifier must survive predicate lowering: {debug}"
    );
    assert!(
        !compact.contains("TagKey(\"rest\")"),
        "no literal unbound rest marker may reach runtime lowering: {debug}"
    );
}

#[test]
fn delegated_choice_from_activation_cost_exile_returns_exact_other_card() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Exiled Cost Partition Probe")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "{3}, Exile two creature cards from your graveyard, Sacrifice this artifact: An opponent chooses one of the exiled cards. You put that card on the bottom of your library and return the other to the battlefield tapped.",
        )
        .expect("activation-cost exile partition should compile");
    let ability = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith_compiler::ability::AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("activated ability");
    let debug = format!("{:#?}", ability.effects);
    let compact = debug
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();

    assert!(
        compact.contains("__source_exiled____delegated_subset"),
        "the opponent-selected exiled card needs a stable subset tag: {debug}"
    );
    assert!(
        compact.contains("IsNotTaggedObject") && compact.contains("enters_tapped:true"),
        "the other exact source-exiled card must return tapped: {debug}"
    );
    assert!(
        !compact.contains("AnyOtherTarget"),
        "the other exiled card is not a fresh graveyard target: {debug}"
    );
}

#[test]
fn delegated_choice_from_revealed_top_collection_exiles_exact_other_card() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Revealed Partition Probe")
        .card_types(vec![CardType::Planeswalker])
        .parse_text(
            "+1: Reveal the top two cards of your library. An opponent chooses one of them. Put that card into your hand and exile the other with a silver counter on it.",
        )
        .expect("revealed collection partition should compile");
    let debug = format!("{definition:#?}");
    let compact = debug
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();

    assert!(
        compact.contains("__delegated_subset"),
        "the opponent-selected revealed card needs its own subset tag: {debug}"
    );
    assert!(
        compact.contains("IsNotTaggedObject") && compact.contains("zone:Exile"),
        "the other exact revealed card must be exiled: {debug}"
    );
    assert!(
        !compact.contains("target:Source"),
        "the complement exile must not move the planeswalker source: {debug}"
    );
}

#[test]
fn those_creatures_followup_reuses_every_coordinated_target_result() {
    let (definition, trace) = ironsmith_compiler::parse_trace::capture(|| {
        CardDefinitionBuilder::new(CardId::new(), "Coordinated Pump Set Probe").parse_text(
            "Until end of turn, target creature gets +3/+3, up to one other target creature gets +2/+2, and up to one other target creature gets +1/+1. Those creatures gain vigilance until end of turn.",
        )
    });
    let definition = definition.unwrap_or_else(|error| {
        panic!(
            "coordinated target set should compile: {error:?}\n{}",
            trace.render()
        )
    });
    let program = definition.spell_effect.expect("spell effect");
    let debug = format!("{program:#?}");
    let compact = debug
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();

    for tag in ["pumped_0", "pumped_1", "pumped_2"] {
        assert_eq!(
            debug.matches(tag).count(),
            2,
            "each independently targeted result must occur in its producer and the plural follow-up union: {debug}"
        );
    }
    assert!(
        compact.contains("set_quantifier_surface:Some(Those,)")
            && compact.contains("any_of:[")
            && compact.contains("explicit_card_type_noun:Some(Creature,)"),
        "the follow-up must retain the authored demonstrative and executable union filter: {debug}"
    );
}

#[test]
fn where_x_possessive_uses_the_introduced_target_not_the_ability_source() {
    let body = "Target creature you control gains flying and gets +X/+X until end of turn, where X is its power.";
    let body_tokens = lex_line(body, 0).expect("target-relative body should lex");
    let parsed_body =
        ironsmith_compiler::semantic_line_parsing::parse_effect_sentences_preserving_source_boundaries(
            &body_tokens,
        )
        .expect("target-relative body should survive semantic sentence parsing");
    let parsed_body_debug = format!("{parsed_body:#?}");
    assert!(
        parsed_body_debug.contains("PowerOf") && parsed_body_debug.contains("\"__it__\""),
        "semantic sentence parsing must retain the target-relative characteristic: {parsed_body_debug}"
    );
    assert!(
        !parsed_body_debug.contains("SourcePower"),
        "semantic sentence parsing must not reinterpret the possessive as the source: {parsed_body_debug}"
    );

    let targeted = CardDefinitionBuilder::new(CardId::new(), "Target Stat Probe")
        .card_types(vec![CardType::Land])
        .parse_text(
            "{1}{G}{U}, {T}: Target creature you control gains flying and gets +X/+X until end of turn, where X is its power.",
        )
        .expect("target-relative where-X ability should compile");
    let targeted_debug = format!("{targeted:#?}");
    let targeted_compact = targeted_debug
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();
    assert!(
        targeted_compact.contains("value:PowerOf(")
            && targeted_compact.contains("\"targeted_0\"")
            && !targeted_compact.contains("value:PowerOf(Source"),
        "the possessive stat must use the creature target rather than the land source: {targeted_debug}"
    );
    assert!(
        !targeted_compact.contains("value:SourcePower"),
        "the target-relative stat must not retain the source fallback: {targeted_debug}"
    );

    let explicit_source = CardDefinitionBuilder::new(CardId::new(), "Source Stat Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{T}: Another target creature you control gains trample and gets +X/+X until end of turn, where X is this creature's power.",
        )
        .expect("explicit source-relative where-X ability should compile");
    let source_debug = format!("{explicit_source:#?}");
    assert!(
        source_debug.contains("PowerOf")
            && source_debug.contains("ThisPermanentType")
            && source_debug.contains("\"this creature\""),
        "an explicit `this creature's` stat must remain source-relative: {source_debug}"
    );
}
