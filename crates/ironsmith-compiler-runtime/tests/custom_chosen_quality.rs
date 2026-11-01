//! Resolution timing and transport checks for manual and full-text quality programs.
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
fn branch(quality: &str) -> Vec<Effect> {
    let text = format!("Type: Sorcery\nLook at the top card of your library. You may reveal a {quality} card from among them and put it into your hand. You may put the rest into your graveyard.");
    let definition = ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler_runtime::compile_to_runtime_definition("Quality branch", text, false)).unwrap();
    definition.spell_effect.unwrap().all_effects_owned()
}
fn definitions() -> Vec<CardDefinition> {
    let mut direct = ironsmith_compiler_runtime::compile_to_runtime_definition("Quality chooser", "Type: Creature\nPower/Toughness: 2/2\nWhenever this creature attacks, you gain 1 life.", false).unwrap();
    let effects = vec![
        Effect::new(ironsmith::effects::ChooseNamedOptionEffect::new(ironsmith::target::PlayerFilter::You, vec!["land".into(), "nonland".into()])),
        Effect::new(ironsmith::effects::ConditionalEffect::new(ironsmith::effect::Condition::SourceChosenOption("land".into()), branch("land"), branch("nonland"))),
    ];
    let trigger = direct.abilities.iter_mut().find_map(|a| match &mut a.kind {ironsmith::ability::AbilityKind::Triggered(t) => Some(t), _ => None}).unwrap();
    trigger.effects = effects.into();
    // Manual runtime composition is a control; artifact transport is checked
    // below using the independently compiled full-text definition.
    let mut definitions = vec![direct];
    let viewed = |quality: &str| format!("Look at the top card of your library. You may reveal a {quality} card from among them and put it into your hand. You may put the rest into your graveyard.");
    let text = format!("Type: Creature\nPower/Toughness: 2/2\nWhenever this creature attacks, choose land or nonland. If land was chosen, {} Otherwise, {}", viewed("land"), viewed("nonland"));
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler_runtime::compile_to_artifact("Textual quality chooser", &text, false)));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(restored, artifact);
    definitions.push(direct);
    definitions.push(ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap());
    definitions
}
struct Choices {land: bool, events: Vec<&'static str>}
impl DecisionMaker for Choices {
    fn decide_options(&mut self, _: &GameState, ctx: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
        self.events.push("choice");
        vec![ctx.options[usize::from(!self.land)].index]
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
        if ctx.min == 0 {ctx.candidates.iter().filter(|o| o.legal).map(|o| o.id).take(1).collect()} else {SelectFirstDecisionMaker.decide_objects(game, ctx)}
    }
    fn decide_boolean(&mut self, _: &GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool {false}
    fn view_cards(&mut self, _: &GameState, _: PlayerId, _: &[ObjectId], ctx: &ironsmith::decisions::context::ViewCardsContext) {if !ctx.public {self.events.push("private-look");}}
}
fn add(game: &mut GameState, name: &str, land: bool) -> ObjectId {
    let text = if land {"Type: Land"} else {"Type: Artifact"};
    let definition = ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false).unwrap();
    game.create_object_from_definition(&definition, A, Zone::Library)
}
#[test]
fn selected_quality_is_chosen_during_resolution_before_fresh_private_look_even_after_source_leaves() {
    for definition in definitions() {for land in [false, true] {for top_land in [false, true] {for departed in [false, true] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let old_top = add(&mut game, "Original top", !top_land);
        let event = ironsmith::triggers::TriggerEvent::new(ironsmith::events::CreatureAttackedEvent::new(source, ironsmith::events::AttackEventTarget::Player(PlayerId(1))), Default::default());
        game.queue_trigger_event(Default::default(), event);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut dm = Choices {land, events: Vec::new()};
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(game.stack.len(), 1);
        assert!(dm.events.is_empty(), "neither choice nor look occurs at trigger announcement");
        let fresh_top = add(&mut game, "Changed top before resolution", top_land);
        let stable = game.object(fresh_top).unwrap().stable_id;
        if departed {execute_effect(&mut game, &Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(source), Zone::Graveyard, false)), &mut EffectContext::new_default(source, A)).unwrap();}
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(dm.events, vec!["choice", "private-look"]);
        let arrival = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(arrival).unwrap().zone, if land == top_land {Zone::Hand} else {Zone::Library});
        assert_eq!(game.object(old_top).unwrap().zone, Zone::Library);
    }}}}
}

#[test]
fn scavengermobile_materialized_source_chooses_only_on_combat_trigger_resolution() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-rediscover.json")).unwrap();
    let row = fixture["mechanics"].as_array().unwrap().iter().find(|row| row["id"] == "msem-348").unwrap();
    let card = &row["representative_cards"][0];
    let name = card["name"].as_str().unwrap();
    let text = card["executable_text"].as_str().unwrap();
    assert!(text.contains("Crew 1"));
    assert!(text.contains("deals combat damage to a player"));
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler_runtime::compile_to_artifact(name, text, false)));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    for definition in [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()] {
        for combat in [false, true] {for land in [false, true] {for top_land in [false, true] {for departed in [false, true] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let old_top = add(&mut game, "Old source-case top", !top_land);
            let mut dm = Choices {land, events: Vec::new()};
            // Use the real damage execution/event pipeline and the unchanged
            // source trigger, not a hand-queued attack or substituted ability.
            execute_effect(&mut game, &Effect::new(ironsmith::effects::DealDamageEffect::new(2, ironsmith::target::ChooseSpec::SpecificPlayer(PlayerId(1))).with_combat(combat)), &mut EffectContext::new_default(source, A)).unwrap();
            assert_eq!(game.player(PlayerId(1)).unwrap().life, 18);
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            assert!(dm.events.is_empty(), "choice/private look cannot occur at damage or trigger announcement");
            assert_eq!(game.stack.len(), usize::from(combat), "noncombat damage never triggers Rediscover source ability");
            let fresh = add(&mut game, "Source-case top changed after announcement", top_land);
            let stable = game.object(fresh).unwrap().stable_id;
            if departed {execute_effect(&mut game, &Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(source), Zone::Graveyard, false)), &mut EffectContext::new_default(source, A)).unwrap();}
            if combat {
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                assert_eq!(dm.events, vec!["choice", "private-look"]);
            }
            let arrival = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(arrival).unwrap().zone, if combat && land == top_land {Zone::Hand} else {Zone::Library});
            assert_eq!(game.object(old_top).unwrap().zone, Zone::Library);
        }}}}
    }
}
