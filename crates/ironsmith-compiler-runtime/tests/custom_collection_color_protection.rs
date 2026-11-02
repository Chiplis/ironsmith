//! Live color populations use their declared zone and exact source collection.
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler::word_claims::with_enforcement_options(true, true, ||
            ironsmith_compiler_runtime::compile_to_artifact(name, text, false)));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
fn card(name: &str, cost: &str) -> CardDefinition {
    ironsmith_compiler_runtime::compile_to_runtime_definition(name, &format!("Mana cost: {cost}\nType: Creature\nPower/Toughness: 2/2"), false).unwrap()
}
fn store(game: &mut GameState, source: ObjectId, card: ObjectId) {
    let mut ctx = EffectContext::new_default(source, A);
    execute_effect(game, &Effect::with_id(101, Effect::new(ironsmith::effects::ExileUntilEffect::source_leaves(ChooseSpec::SpecificObject(card)))), &mut ctx).unwrap();
    let schedule = ironsmith::effects::ScheduleDelayedTriggerEffect::new(
        ironsmith::triggers::Trigger::this_deals_combat_damage_to_player(PlayerFilter::Any), vec![], false, vec![], PlayerFilter::You)
        .watch_ability_source().with_source_collection_scope(Some(ironsmith_core::effect::SourceCollectionScope {
            name: "cargo".into(), tag: "source_collection:cargo".into(), zone: Zone::Exile,
            capture_from: ironsmith::effect::EffectId(101), capture_action: ironsmith_core::PriorEffectAction::Exiled,
        }));
    execute_effect(game, &Effect::new(schedule), &mut ctx).unwrap();
}
#[test]
fn collection_colors_follow_live_members_and_do_not_leak_between_sources_or_zone_incarnations() {
    for protected in definitions("Collection guardian", "Type: Creature\nPower/Toughness: 3/3\nThis creature has protection from each color among cards in exile from the cargo collection.") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let guardian = game.create_object_from_definition(&protected, A, Zone::Battlefield);
        let other = game.create_object_from_definition(&protected, A, Zone::Battlefield);
        let red = card("Red object", "{R}");
        let blue = card("Blue object", "{U}");
        let red_source = game.create_object_from_definition(&red, B, Zone::Stack);
        let blue_source = game.create_object_from_definition(&blue, B, Zone::Stack);
        let red_member = game.create_object_from_definition(&red, A, Zone::Graveyard);
        let blue_member = game.create_object_from_definition(&blue, A, Zone::Graveyard);
        store(&mut game, guardian, red_member);
        store(&mut game, other, blue_member);
        game.create_object_from_definition(&blue, A, Zone::Exile);
        game.refresh_continuous_state().unwrap();
        assert!(ironsmith::targeting::has_protection_from_source(&game, guardian, red_source));
        assert!(!ironsmith::targeting::has_protection_from_source(&game, guardian, blue_source), "other source collections and unrelated exile cards cannot supply colors");
        assert!(ironsmith::targeting::has_protection_from_source(&game, other, blue_source));
        let member = game.source_collection_objects(guardian, "cargo")[0];
        let hand = game.move_object_by_effect(member, Zone::Hand).unwrap();
        game.move_object_by_effect(hand, Zone::Exile).unwrap();
        game.refresh_continuous_state().unwrap();
        assert!(!ironsmith::targeting::has_protection_from_source(&game, guardian, red_source), "leaving the collection ends color membership permanently for that incarnation");
        game.set_current_controller(guardian, B).unwrap();
        game.refresh_continuous_state().unwrap();
        assert!(!ironsmith::targeting::has_protection_from_source(&game, guardian, blue_source), "control change cannot borrow another source's collection");
    }
}
#[test]
fn ordinary_nonbattlefield_color_population_uses_its_explicit_zone() {
    for protected in definitions("Graveyard guardian", "Type: Creature\nPower/Toughness: 3/3\nThis creature has protection from each color among creature cards in your graveyard.") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let guardian = game.create_object_from_definition(&protected, A, Zone::Battlefield);
        let red = card("Red object", "{R}");
        let red_source = game.create_object_from_definition(&red, B, Zone::Stack);
        let member = game.create_object_from_definition(&red, A, Zone::Graveyard);
        game.refresh_continuous_state().unwrap();
        assert!(ironsmith::targeting::has_protection_from_source(&game, guardian, red_source));
        game.move_object_by_effect(member, Zone::Exile).unwrap();
        game.refresh_continuous_state().unwrap();
        assert!(!ironsmith::targeting::has_protection_from_source(&game, guardian, red_source));
    }
}
#[test]
fn unquoted_resolving_protection_samples_nonbattlefield_population_once() {
    for definition in definitions("Population ward", "Type: Instant\nTarget creature gains protection from each color among creature cards in your graveyard until end of turn.") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let red = card("Red object", "{R}");
        let guardian = game.create_object_from_definition(&card("Guardian", "{0}"), A, Zone::Battlefield);
        let red_source = game.create_object_from_definition(&red, B, Zone::Stack);
        let member = game.create_object_from_definition(&red, A, Zone::Graveyard);
        let source = game.create_object_from_definition(&definition, A, Zone::Stack);
        let mut context = EffectContext::new_default(source, A).with_targets(vec![ironsmith::effects::ResolvedTarget::Object(guardian)]);
        for effect in definition.spell_effect.as_ref().unwrap().all_effects() { execute_effect(&mut game, effect, &mut context).unwrap(); }
        game.move_object_by_effect(member, Zone::Exile).unwrap();
        game.refresh_continuous_state().unwrap();
        assert!(ironsmith::targeting::has_protection_from_source(&game, guardian, red_source), "unquoted resolving grant captures the population's colors once");
    }
}
