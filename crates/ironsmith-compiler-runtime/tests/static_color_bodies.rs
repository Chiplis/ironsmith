//! Complete frozen bodies with independent direct/artifact and native witnesses.
//! Authored only: no build, compilation, test, or runtime execution was performed.
use ironsmith::ability::{Ability, AbilityKind};
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::card::PowerToughness;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::color::{Color, ColorSet};
use ironsmith::continuous::{ContinuousEffect, EffectSourceType, EffectTarget, Modification, TextBoxOverlay};
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::TargetsContext;
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::Phase;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::snapshot::CopiableValues;
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
use ironsmith::target::{ChooseSpec, ObjectFilter};
use ironsmith::targeting::compute_legal_targets;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, GameProgress, GameState, ObjectId, PlayerId, Subtype, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
fn all_colors() -> ColorSet { Color::ALL.into_iter().collect() }
fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/static_color_bodies.json.fixture")).unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {p}/{t}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    from_text(name, &text)
}
fn from_text(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) = parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    let direct = direct.unwrap_or_else(|error| panic!("direct {name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (compiled, loss) = parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("artifact {name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.phase = Phase::FirstMain;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    for color in [ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black,
        ManaSymbol::Red, ManaSymbol::Green, ManaSymbol::Colorless] {
        game.player_mut(A).unwrap().mana_pool.add(color, 20);
    }
    game
}
fn native(name: &str, colors: ColorSet, subtype: Subtype) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![CardType::Creature])
        .subtypes(vec![subtype]).color_indicator(colors)
        .power_toughness(PowerToughness::fixed(2, 2)).build()
}
fn refresh(game: &mut GameState) { game.refresh_continuous_state().unwrap(); }
fn color(game: &GameState, id: ObjectId) -> ColorSet { game.current_colors(id).unwrap() }
fn permanent(game: &mut GameState, owner: PlayerId, colors: ColorSet, subtype: Subtype) -> ObjectId {
    let id = game.create_object_from_definition(&native("Color witness", colors, subtype), owner, Zone::Battlefield);
    game.remove_summoning_sickness(id);
    id
}
fn static_effect(game: &mut GameState, source: ObjectId, target: EffectTarget, modification: Modification) {
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(source, A, target, modification));
    refresh(game);
}
struct TargetChoice(Target);
impl DecisionMaker for TargetChoice {
    fn decide_targets(&mut self, _game: &GameState, context: &TargetsContext) -> Vec<Target> {
        assert_eq!(context.requirements.len(), 1);
        assert!(context.requirements[0].legal_targets.contains(&self.0));
        vec![self.0]
    }
}
fn announce(game: &mut GameState, definition: &CardDefinition, target: Target) -> ObjectId {
    let id = game.create_object_from_definition(definition, A, Zone::Hand);
    let action = LegalAction::CastSpell { spell_id: id, from_zone: Zone::Hand, casting_method: CastingMethod::Normal };
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
    let before = game.player(A).unwrap().mana_pool.total();
    let mut dm = TargetChoice(target);
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
    for _ in 0..32 {
        if state.pending_cast.is_none() && state.pending_mana_ability.is_none() { break; }
        let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_mana_ability.is_none());
    let spell = game.object_ids_in_deterministic_order().into_iter()
        .find(|id| game.object(*id).unwrap().zone == Zone::Stack).expect("announced spell on stack");
    assert_eq!(color(game, spell), ColorSet::COLORLESS);
    assert_eq!(game.object(spell).unwrap().mana_cost.as_ref().unwrap().mana_value(), 3);
    assert_eq!(game.player(A).unwrap().mana_pool.total(), before - 3);
    spell
}
fn cast(game: &mut GameState, definition: &CardDefinition, target: Target) {
    announce(game, definition, target);
    resolve_stack_entry_with(game, &mut TargetChoice(target)).unwrap();
    assert!(game.stack_is_empty());
}

#[test]
fn full_bodies_preserve_identity_and_base_colors_without_fabricated_indicators() {
    for (name, base, identity, abilities) in [
        ("Ghostfire", ColorSet::RED, ColorSet::RED, 1),
        ("Ghostflame Sliver", ColorSet::BLACK.union(ColorSet::RED), ColorSet::BLACK.union(ColorSet::RED), 1),
        ("Transguild Courier", ColorSet::COLORLESS, all_colors(), 1),
        ("Sphinx of the Guildpact", ColorSet::COLORLESS, all_colors(), 3),
    ] {
        for definition in definitions(name) {
            assert_eq!(definition.card.colors(), base);
            assert_eq!(definition.card.color_identity(), identity);
            assert_eq!(definition.card.color_indicator, None);
            assert_eq!(definition.abilities.len(), abilities);
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            for ability in &definition.abilities {
                if let AbilityKind::Static(ability) = &ability.kind { assert!(!ability.is_devoid()); }
            }
            assert_eq!(definition.spell_effect.is_some(), name == "Ghostfire");
        }
    }
}

#[test]
fn source_color_statements_function_in_all_zones_and_keep_mana_identity() {
    for name in ["Ghostfire", "Transguild Courier", "Sphinx of the Guildpact"] {
        for definition in definitions(name) {
            let expected = if name == "Ghostfire" { ColorSet::COLORLESS } else { all_colors() };
            for zone in [Zone::Library, Zone::Hand, Zone::Stack, Zone::Battlefield,
                Zone::Graveyard, Zone::Exile, Zone::Command, Zone::Ante, Zone::OutsideGame] {
                let mut game = game();
                let id = game.create_object_from_definition(&definition, A, zone);
                refresh(&mut game);
                assert_eq!(color(&game, id), expected, "{name} in {zone:?}");
                assert_eq!(game.object(id).unwrap().color_identity(), definition.card.color_identity());
            }
        }
    }
}

#[test]
fn ghostfire_casts_for_red_mana_but_hits_monocolored_hexproof_as_a_colorless_spell() {
    for route in 0..2 {
        let mut game = game();
        let sphinx = game.create_object_from_definition(&definitions("Sphinx of the Guildpact")[route], B, Zone::Battlefield);
        cast(&mut game, &definitions("Ghostfire")[route], Target::Object(sphinx));
        assert_eq!(game.damage_on(sphinx), 3);
        let mut game = self::game();
        cast(&mut game, &definitions("Ghostfire")[route], Target::Player(B));
        assert_eq!(game.player(B).unwrap().life, 17);
    }
}

#[test]
fn sphinx_flying_and_hexproof_test_current_colors_controllers_and_real_blocking() {
    for definition in definitions("Sphinx of the Guildpact") {
        let mut game = game();
        let sphinx = game.create_object_from_definition(&definition, B, Zone::Battlefield);
        game.remove_summoning_sickness(sphinx);
        let ground = permanent(&mut game, A, ColorSet::GREEN, Subtype::Bear);
        assert!(!ironsmith::rules::combat::can_block(game.object(sphinx).unwrap(), game.object(ground).unwrap(), &game));
        static_effect(&mut game, ground, EffectTarget::Source, Modification::AddAbility(StaticAbility::reach()));
        assert!(ironsmith::rules::combat::can_block(game.object(sphinx).unwrap(), game.object(ground).unwrap(), &game));
        for (colors, controller, allowed) in [
            (ColorSet::RED, A, false), (ColorSet::RED, B, true),
            (ColorSet::COLORLESS, A, true), (ColorSet::RED.union(ColorSet::BLUE), A, true),
        ] {
            let source = game.create_object_from_definition(&native("Target source", colors, Subtype::Wizard), controller, Zone::Stack);
            assert_eq!(compute_legal_targets(&game, &ChooseSpec::target_creature(), controller, Some(source))
                .contains(&Target::Object(sphinx)), allowed);
        }
        let source = game.create_object_from_definition(&native("Current-color source", ColorSet::RED, Subtype::Wizard), A, Zone::Stack);
        assert!(!compute_legal_targets(&game, &ChooseSpec::target_creature(), A, Some(source)).contains(&Target::Object(sphinx)));
        static_effect(&mut game, source, EffectTarget::Source, Modification::SetColors(ColorSet::COLORLESS));
        assert!(compute_legal_targets(&game, &ChooseSpec::target_creature(), A, Some(source)).contains(&Target::Object(sphinx)));
        static_effect(&mut game, ground, EffectTarget::Specific(sphinx), Modification::RemoveAllAbilities);
        assert!(!game.object_has_ability(sphinx, &StaticAbility::flying()));
        assert_eq!(color(&game, sphinx), all_colors(), "color applies in layer five before layer-six loss");
    }
}

#[test]
fn ghostflame_affects_current_sliver_permanents_of_every_controller_only_while_present() {
    for definition in definitions("Ghostflame Sliver") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let own = permanent(&mut game, A, ColorSet::WHITE, Subtype::Sliver);
        let enemy = permanent(&mut game, B, ColorSet::BLUE, Subtype::Sliver);
        let elf = permanent(&mut game, B, ColorSet::GREEN, Subtype::Elf);
        let kindred = CardDefinitionBuilder::new(CardId::new(), "Noncreature Sliver")
            .card_types(vec![CardType::Kindred, CardType::Enchantment]).subtypes(vec![Subtype::Sliver])
            .color_indicator(ColorSet::BLACK).build();
        let noncreature = game.create_object_from_definition(&kindred, B, Zone::Battlefield);
        for zone in [Zone::Hand, Zone::Library, Zone::Stack, Zone::Graveyard, Zone::Exile, Zone::Command, Zone::OutsideGame] {
            let outside = game.create_object_from_definition(&definition, B, zone);
            assert_eq!(color(&game, outside), ColorSet::BLACK.union(ColorSet::RED));
        }
        refresh(&mut game);
        for id in [source, own, enemy, noncreature] { assert_eq!(color(&game, id), ColorSet::COLORLESS); }
        assert_eq!(color(&game, elf), ColorSet::GREEN);
        static_effect(&mut game, elf, EffectTarget::Source, Modification::AddSubtypes(vec![Subtype::Sliver]));
        assert_eq!(color(&game, elf), ColorSet::COLORLESS, "the layer-four subtype is current");
        static_effect(&mut game, enemy, EffectTarget::Source, Modification::SetSubtypes(vec![Subtype::Elf]));
        assert_eq!(color(&game, enemy), ColorSet::BLUE);
        game.set_current_controller(source, B).unwrap();
        assert_eq!(color(&game, own), ColorSet::COLORLESS);
        game.phase_out(source);
        refresh(&mut game);
        assert_eq!(color(&game, own), ColorSet::WHITE);
        assert_eq!(color(&game, noncreature), ColorSet::BLACK);
        game.phase_in(source);
        refresh(&mut game);
        assert_eq!(color(&game, own), ColorSet::COLORLESS);
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        refresh(&mut game);
        assert_eq!(color(&game, own), ColorSet::WHITE);
        assert_eq!(color(&game, noncreature), ColorSet::BLACK);
        assert_eq!(color(&game, elf), ColorSet::GREEN);
    }
}

#[test]
fn source_cdas_precede_older_color_effects_and_copies_keep_that_order() {
    for name in ["Ghostfire", "Transguild Courier", "Sphinx of the Guildpact"] {
        for definition in definitions(name) {
            let mut game = game();
            let painter = permanent(&mut game, A, ColorSet::GREEN, Subtype::Elf);
            static_effect(&mut game, painter, EffectTarget::AllPermanents, Modification::SetColors(ColorSet::BLUE));
            let original = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            assert_eq!(color(&game, original), ColorSet::BLUE, "CDA precedes the older ordinary setter");
            let copy = permanent(&mut game, A, ColorSet::RED, Subtype::Shapeshifter);
            let values = CopiableValues::from_object(game.object(original).unwrap());
            static_effect(&mut game, copy, EffectTarget::Source, Modification::CopyOf {
                target_id: original, copiable_values: Box::new(values), preserve_source_abilities: false,
                name_override: None, name_override_surface: None, add_supertypes: vec![],
            });
            assert_eq!(color(&game, copy), ColorSet::BLUE, "a copied CDA also precedes the older setter");
            game.effect_store.continuous_effects.remove_effects_from_source(painter);
            refresh(&mut game);
            let expected = if name == "Ghostfire" { ColorSet::COLORLESS } else { all_colors() };
            assert_eq!(color(&game, original), expected);
            assert_eq!(color(&game, copy), expected);
            static_effect(&mut game, painter, EffectTarget::Specific(copy), Modification::SetColors(ColorSet::RED));
            assert_eq!(color(&game, copy), ColorSet::RED, "later ordinary color effects remain effective");
        }
    }
}

#[test]
fn native_printed_and_ordinary_granted_colors_use_typed_origins() {
    let mut game = game();
    let painter = permanent(&mut game, A, ColorSet::GREEN, Subtype::Elf);
    static_effect(&mut game, painter, EffectTarget::AllPermanents, Modification::SetColors(ColorSet::RED));
    let cda = StaticAbility::set_colors(ObjectFilter::source(), ColorSet::BLUE);
    let printed = CardDefinitionBuilder::new(CardId::new(), "Unrelated native name")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 2))
        .with_ability(Ability::static_ability(cda.clone())).build();
    assert_eq!(printed.card.color_identity(), ColorSet::BLUE);
    let printed_id = game.create_object_from_definition(&printed, A, Zone::Battlefield);
    assert_eq!(color(&game, printed_id), ColorSet::RED);
    let granted_id = permanent(&mut game, A, ColorSet::GREEN, Subtype::Elf);
    game.grant_temporary_static_ability_payload_to_object_until_end_of_turn(
        granted_id, StaticAbilityId::SetColors, Some(cda));
    refresh(&mut game);
    let effects = ironsmith::static_ability_processor::generate_continuous_effects_from_static_abilities(&game);
    assert!(effects.iter().any(|effect| effect.source == printed_id
        && matches!(effect.source_type, EffectSourceType::CharacteristicDefining)));
    assert!(effects.iter().any(|effect| effect.source == granted_id
        && matches!(effect.modification, Modification::SetColors(ColorSet::BLUE))
        && matches!(effect.source_type, EffectSourceType::StaticAbility)));
    // Ordinary acquisition stays outside the CDA partition.
    assert_eq!(color(&game, granted_id), ColorSet::BLUE, "ordinary grant is not classified as a CDA");
    assert_eq!(game.object(granted_id).unwrap().color_identity(), ColorSet::GREEN);
    let hand = game.create_object_from_definition(&native("Hand grant witness", ColorSet::GREEN, Subtype::Elf), A, Zone::Hand);
    game.grant_temporary_static_ability_payload_to_object_until_end_of_turn(hand,
        StaticAbilityId::SetColors,
        Some(StaticAbility::set_colors(ObjectFilter::source(), ColorSet::BLUE)));
    refresh(&mut game);
    assert_eq!(color(&game, hand), ColorSet::GREEN, "ordinary granted color abilities do not inherit CDA functional zones");
    game.effect_store.continuous_effects.remove_effects_from_source(painter);
    refresh(&mut game);
    assert_eq!(color(&game, printed_id), ColorSet::BLUE);
    game.object_mut(printed_id).unwrap().abilities_mut().clear();
    refresh(&mut game);
    assert_eq!(color(&game, printed_id), ColorSet::COLORLESS, "no fabricated color indicator survives source-text removal");
}

#[test]
fn native_devoid_and_literal_colorless_have_equal_colors_but_distinct_keyword_identity() {
    let literal = StaticAbility::set_colors(ObjectFilter::source(), ColorSet::COLORLESS);
    let devoid = StaticAbility::make_colorless(ObjectFilter::source());
    assert!(!literal.is_devoid());
    assert!(devoid.is_devoid());
    assert!(literal.display().contains("colorless"));
    assert!(!literal.display().contains("Devoid"));
    for ability in [literal, devoid] {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Independent native color witness")
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Red]))
            .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(1, 1))
            .with_ability(Ability::static_ability(ability)).build();
        assert_eq!(definition.card.colors(), ColorSet::RED);
        assert_eq!(definition.card.color_identity(), ColorSet::RED);
        for zone in [Zone::Library, Zone::Hand, Zone::Stack, Zone::Graveyard, Zone::Exile, Zone::Command, Zone::Battlefield] {
            let mut game = game();
            let id = game.create_object_from_definition(&definition, A, zone);
            assert_eq!(color(&game, id), ColorSet::COLORLESS);
        }
    }
}

#[test]
fn renamed_full_source_statements_preserve_semantics_and_reject_unsupported_tails() {
    for definition in from_text("Unrelated source", "Mana cost: {2}{R}\nType: Instant\nUnrelated source is colorless.\nUnrelated source deals 3 damage to any target.") {
        let mut game = game();
        cast(&mut game, &definition, Target::Player(B));
        assert_eq!(game.player(B).unwrap().life, 17);
    }
    for text in ["Type: Artifact\nThis artifact is all colors except blue.",
        "Type: Artifact\nThis artifact is colorless and has unsupported tail."] {
        assert!(compile_to_runtime_definition("Reject the tail", text, false).is_err());
    }
}

#[test]
fn copied_ghostfire_keeps_literal_colorless_and_the_full_damage_program() {
    use ironsmith::effects::{CopySpellEffect, EffectContext, EffectExecutor};
    for definition in definitions("Ghostfire") {
        let mut game = game();
        let original = announce(&mut game, &definition, Target::Player(B));
        let mut choices = TargetChoice(Target::Player(B));
        CopySpellEffect::single(ChooseSpec::SpecificObject(original)).execute(
            &mut game, &mut EffectContext::new(original, A, &mut choices)).unwrap();
        assert_eq!(game.stack.len(), 2);
        let copy = game.stack.last().unwrap().object_id;
        assert_ne!(copy, original);
        assert_eq!(color(&game, copy), ColorSet::COLORLESS);
        assert_eq!(game.object(copy).unwrap().color_identity(), ColorSet::RED);
        assert!(!game.current_abilities(copy).unwrap().iter().any(|ability|
            matches!(&ability.kind, AbilityKind::Static(ability) if ability.is_devoid())));
        resolve_stack_entry_with(&mut game, &mut choices).unwrap();
        assert_eq!(game.player(B).unwrap().life, 17);
        resolve_stack_entry_with(&mut game, &mut choices).unwrap();
        assert_eq!(game.player(B).unwrap().life, 14);
    }
}

#[test]
fn text_box_replacement_gets_cda_order_but_cannot_rewrite_commander_identity() {
    let mut game = game();
    let painter = permanent(&mut game, A, ColorSet::BLACK, Subtype::Elf);
    static_effect(&mut game, painter, EffectTarget::AllPermanents, Modification::SetColors(ColorSet::RED));
    let recipient = permanent(&mut game, A, ColorSet::GREEN, Subtype::Elf);
    game.set_as_commander(recipient, A);
    static_effect(&mut game, recipient, EffectTarget::Source, Modification::SetTextBox(TextBoxOverlay {
        compiled_card_text: "irrelevant display".into(),
        abilities: vec![Ability::static_ability(StaticAbility::set_colors(ObjectFilter::source(), ColorSet::BLUE))],
        ability_labels: vec!["unrelated presentation".into()].into(),
    }));
    assert_eq!(color(&game, recipient), ColorSet::RED);
    assert_eq!(game.get_commander_color_identity(A), ColorSet::GREEN);
    game.effect_store.continuous_effects.remove_effects_from_source(painter);
    refresh(&mut game);
    assert_eq!(color(&game, recipient), ColorSet::BLUE);
    static_effect(&mut game, recipient, EffectTarget::Source, Modification::SetTextBox(TextBoxOverlay {
        compiled_card_text: "".into(), abilities: vec![], ability_labels: vec![].into(),
    }));
    assert_eq!(color(&game, recipient), ColorSet::GREEN);
    assert_eq!(game.get_commander_color_identity(A), ColorSet::GREEN);
}

#[test]
fn canonical_complete_bodies_reparse_independently_and_keep_runtime_scope() {
    for row in rows() {
        let name = row["name"].as_str().unwrap();
        for original in definitions(name) {
            let rendered = ironsmith_text::compiled_text_lines(&original).join("\n");
            let mut text = format!("Mana cost: {}\nType: {}\n",
                row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
            if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
                text.push_str(&format!("Power/Toughness: {p}/{t}\n"));
            }
            text.push_str(&rendered);
            for reparsed in from_text(name, &text) {
                assert_eq!(reparsed.card.color_identity(), original.card.color_identity());
                assert_eq!(reparsed.card.color_indicator, None);
                assert_eq!(reparsed.abilities.len(), original.abilities.len());
                assert_eq!(reparsed.spell_effect.is_some(), original.spell_effect.is_some());
                let mut game = game();
                match name {
                    "Ghostfire" => {
                        cast(&mut game, &reparsed, Target::Player(B));
                        assert_eq!(game.player(B).unwrap().life, 17);
                    }
                    "Ghostflame Sliver" => {
                        game.create_object_from_definition(&reparsed, A, Zone::Battlefield);
                        let creature = permanent(&mut game, B, ColorSet::BLUE, Subtype::Sliver);
                        let noncreature = CardDefinitionBuilder::new(CardId::new(), "Kindred reparse witness")
                            .card_types(vec![CardType::Kindred, CardType::Enchantment])
                            .subtypes(vec![Subtype::Sliver]).color_indicator(ColorSet::GREEN).build();
                        let permanent = game.create_object_from_definition(&noncreature, B, Zone::Battlefield);
                        let hand = game.create_object_from_definition(&noncreature, B, Zone::Hand);
                        assert_eq!(color(&game, creature), ColorSet::COLORLESS);
                        assert_eq!(color(&game, permanent), ColorSet::COLORLESS);
                        assert_eq!(color(&game, hand), ColorSet::GREEN);
                    }
                    "Transguild Courier" | "Sphinx of the Guildpact" => {
                        let source = game.create_object_from_definition(&reparsed, B, Zone::Battlefield);
                        let hand = game.create_object_from_definition(&reparsed, B, Zone::Hand);
                        assert_eq!(color(&game, source), all_colors());
                        assert_eq!(color(&game, hand), all_colors());
                        if name == "Sphinx of the Guildpact" {
                            assert!(game.object_has_ability(source, &StaticAbility::flying()));
                            let red = game.create_object_from_definition(&native("Monocolored reparse witness", ColorSet::RED, Subtype::Wizard), A, Zone::Stack);
                            assert!(!compute_legal_targets(&game, &ChooseSpec::target_creature(), A, Some(red)).contains(&Target::Object(source)));
                        }
                    }
                    _ => panic!("unexpected frozen fixture"),
                }
            }
        }
    }
}

#[test]
fn live_color_grants_use_acquisition_time_then_preserve_native_clone_identity() {
    let mut game = game();
    let host = permanent(&mut game, A, ColorSet::WHITE, Subtype::Elf);
    let source = permanent(&mut game, B, ColorSet::BLACK, Subtype::Wizard);
    static_effect(&mut game, source, EffectTarget::Specific(host), Modification::SetColors(ColorSet::RED));
    assert_eq!(color(&game, host), ColorSet::RED);
    let before_grant = game.effect_store.continuous_effects.current_timestamp();
    game.grant_temporary_static_ability_payload_to_object_until_end_of_turn(host,
        StaticAbilityId::SetColors,
        Some(StaticAbility::set_colors(ObjectFilter::source(), ColorSet::BLUE)));
    let grant_time = game.effect_store.continuous_effects.current_timestamp();
    assert!(grant_time > before_grant);
    refresh(&mut game);
    assert_eq!(color(&game, host), ColorSet::BLUE, "a later grant beats red on an older host");
    let original_origin = game.object(host).unwrap().temporary_static_ability_grants.origin(0).unwrap().clone();
    for _ in 0..3 {
        refresh(&mut game);
        let effects = ironsmith::static_ability_processor::generate_continuous_effects_from_static_abilities(&game);
        let granted = effects.iter().find(|effect| effect.source == host
            && matches!(effect.modification, Modification::SetColors(ColorSet::BLUE))).unwrap();
        assert_eq!(granted.timestamp, grant_time, "refresh must not reacquire the ability");
        assert!(matches!(granted.source_type, EffectSourceType::StaticAbility));
    }
    let mut cloned = game.clone();
    assert_eq!(cloned.object(host).unwrap().temporary_static_ability_grants.origin(0), Some(&original_origin));
    let original_payload = game.object(host).unwrap().temporary_static_ability_grants[0].materialize().unwrap();
    let cloned_payload = cloned.object(host).unwrap().temporary_static_ability_grants[0].materialize().unwrap();
    assert_eq!(original_payload.instance_id(), cloned_payload.instance_id());
    static_effect(&mut cloned, source, EffectTarget::Specific(host), Modification::SetColors(ColorSet::GREEN));
    assert_eq!(color(&cloned, host), ColorSet::GREEN, "an ordinary effect acquired after the grant wins");
    assert_eq!(color(&game, host), ColorSet::BLUE, "a clone does not mutate authoritative chronology");
    game.turn.turn_number += 1;
    refresh(&mut game);
    assert_eq!(color(&game, host), ColorSet::RED, "expiry removes the blue grant, not the older red effect");
    assert_eq!(game.object(host).unwrap().temporary_static_ability_grants.origin(0), Some(&original_origin));
}

#[test]
fn granted_frozen_self_color_bodies_are_not_cdas_on_an_older_recipient() {
    for name in ["Ghostfire", "Transguild Courier", "Sphinx of the Guildpact"] {
        for definition in definitions(name) {
            let color_ability = definition.abilities.iter().find_map(|ability| match &ability.kind {
                AbilityKind::Static(ability) if ability.id() == StaticAbilityId::SetColors => Some(ability.clone()),
                _ => None,
            }).unwrap();
            let mut game = game();
            let host = permanent(&mut game, A, ColorSet::WHITE, Subtype::Elf);
            let source = permanent(&mut game, B, ColorSet::BLACK, Subtype::Wizard);
            static_effect(&mut game, source, EffectTarget::Specific(host), Modification::SetColors(ColorSet::RED));
            game.grant_temporary_static_ability_payload_to_object_until_end_of_turn(host,
                StaticAbilityId::SetColors, Some(color_ability));
            refresh(&mut game);
            let expected = if name == "Ghostfire" { ColorSet::COLORLESS } else { all_colors() };
            assert_eq!(color(&game, host), expected, "an acquired {name} color ability has its grant timestamp");
            assert_eq!(game.object(host).unwrap().color_identity(), ColorSet::WHITE);
            static_effect(&mut game, source, EffectTarget::Specific(host), Modification::SetColors(ColorSet::GREEN));
            assert_eq!(color(&game, host), ColorSet::GREEN);
        }
    }
}

#[test]
fn ordinary_color_grants_use_the_hosts_normal_zone_instead_of_cda_zones() {
    for card_type in [CardType::Instant, CardType::Sorcery, CardType::Creature] {
        for zone in [Zone::Hand, Zone::Stack, Zone::Battlefield, Zone::Graveyard] {
            let mut game = game();
            let definition = CardDefinitionBuilder::new(CardId::new(), "Ordinary-zone grant witness")
                .card_types(vec![card_type]).color_indicator(ColorSet::RED)
                .power_toughness(PowerToughness::fixed(2, 2)).build();
            let host = game.create_object_from_definition(&definition, A, zone);
            game.grant_temporary_static_ability_payload_to_object_until_end_of_turn(host,
                StaticAbilityId::SetColors,
                Some(StaticAbility::set_colors(ObjectFilter::source(), ColorSet::BLUE)));
            refresh(&mut game);
            let functional = if matches!(card_type, CardType::Instant | CardType::Sorcery) {
                Zone::Stack
            } else { Zone::Battlefield };
            assert_eq!(color(&game, host), if zone == functional { ColorSet::BLUE } else { ColorSet::RED });
        }
    }
}

#[test]
fn a_resolving_permanent_keeps_grant_identity_but_uses_its_later_entry_timestamp() {
    let mut game = game();
    let spell = game.create_object_from_definition(&native("Entry chronology witness", ColorSet::WHITE, Subtype::Elf), A, Zone::Stack);
    game.grant_temporary_static_ability_payload_to_object_until_end_of_turn(spell,
        StaticAbilityId::SetColors,
        Some(StaticAbility::set_colors(ObjectFilter::source(), ColorSet::BLUE)));
    let origin = game.object(spell).unwrap().temporary_static_ability_grants.origin(0).unwrap().clone();
    let source = permanent(&mut game, B, ColorSet::BLACK, Subtype::Wizard);
    static_effect(&mut game, source, EffectTarget::AllPermanents, Modification::SetColors(ColorSet::RED));
    let permanent = game.move_object_by_effect(spell, Zone::Battlefield).unwrap();
    refresh(&mut game);
    assert_eq!(game.object(permanent).unwrap().temporary_static_ability_grants.origin(0), Some(&origin));
    assert_eq!(color(&game, permanent), ColorSet::BLUE, "entry is later than the intervening red setter");
    let graveyard = game.move_object_by_effect(permanent, Zone::Graveyard).unwrap();
    assert!(game.object(graveyard).unwrap().temporary_static_ability_grants.is_empty());
    assert_eq!(color(&game, graveyard), ColorSet::WHITE);
}

#[test]
fn ghostfire_copy_keeps_printed_colorless_but_does_not_copy_an_ordinary_blue_grant() {
    use ironsmith::effects::{CopySpellEffect, EffectContext, EffectExecutor};
    for definition in definitions("Ghostfire") {
        let mut game = game();
        let original = announce(&mut game, &definition, Target::Player(B));
        game.grant_temporary_static_ability_payload_to_object_until_end_of_turn(original,
            StaticAbilityId::SetColors,
            Some(StaticAbility::set_colors(ObjectFilter::source(), ColorSet::BLUE)));
        refresh(&mut game);
        assert_eq!(color(&game, original), ColorSet::BLUE);
        let mut choices = TargetChoice(Target::Player(B));
        CopySpellEffect::single(ChooseSpec::SpecificObject(original)).execute(
            &mut game, &mut EffectContext::new(original, A, &mut choices)).unwrap();
        let copy = game.stack.last().unwrap().object_id;
        assert_ne!(copy, original);
        assert!(game.object(copy).unwrap().temporary_static_ability_grants.is_empty());
        assert_eq!(color(&game, copy), ColorSet::COLORLESS);
        assert_eq!(color(&game, original), ColorSet::BLUE);
        resolve_stack_entry_with(&mut game, &mut choices).unwrap();
        assert_eq!(game.player(B).unwrap().life, 17);
        resolve_stack_entry_with(&mut game, &mut choices).unwrap();
        assert_eq!(game.player(B).unwrap().life, 14);
    }
}

#[test]
fn spell_copies_reconstruct_dash_and_blitz_riders_from_their_copied_cast_choices() {
    use ironsmith::effects::{CopySpellEffect, EffectContext, EffectExecutor};
    for blitz in [false, true] {
        for reconstruct in [false, true] {
            let mut game = game();
            let builder = CardDefinitionBuilder::new(CardId::new(), "Alternative-copy guard")
                .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(3)]))
                .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 2));
            let price = ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]);
            let mut definition = (if blitz { builder.blitz(price) } else { builder.dash(price) }).build();
            if reconstruct { definition.abilities.clear(); }
            let hand = game.create_object_from_definition(&definition, A, Zone::Hand);
            let action = LegalAction::CastSpell { spell_id: hand, from_zone: Zone::Hand,
                casting_method: CastingMethod::Alternative(0) };
            assert!(compute_legal_actions(&game, A).unwrap().contains(&action));
            let mut state = PriorityLoopState::new(2);
            let mut queue = TriggerQueue::new();
            let mut choices = SelectFirstDecisionMaker;
            let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
                &PriorityResponse::PriorityAction(action), &mut choices).unwrap();
            for _ in 0..32 {
                if !state.has_pending_action() { break; }
                let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}"); };
                progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &context, &mut choices).unwrap();
            }
            assert!(!state.has_pending_action());
            let original = game.stack.last().unwrap().object_id;
            if reconstruct { assert!(!game.object(original).unwrap().temporary_static_ability_grants.is_empty()); }
            CopySpellEffect::single(ChooseSpec::SpecificObject(original)).execute(
                &mut game, &mut EffectContext::new(original, A, &mut choices)).unwrap();
            let copy = game.stack.last().unwrap().object_id;
            assert!(game.object(copy).unwrap().temporary_static_ability_grants.is_empty());
            resolve_stack_entry_with(&mut game, &mut choices).unwrap();
            let copied_permanent = *game.battlefield.last().unwrap();
            assert!(game.object_has_ability(copied_permanent, &StaticAbility::haste()));
            assert!(ironsmith::rules::combat::can_attack(game.object(copied_permanent).unwrap(), &game));
            if blitz {
                assert!(game.current_abilities(copied_permanent).unwrap().iter()
                    .any(ironsmith::alternative_cast::is_blitz_death_draw_ability));
            }
            resolve_stack_entry_with(&mut game, &mut choices).unwrap();
            assert_eq!(game.battlefield.len(), 2);
            for id in game.battlefield.iter() {
                assert!(game.object_has_ability(*id, &StaticAbility::haste()));
            }
        }
    }
}
