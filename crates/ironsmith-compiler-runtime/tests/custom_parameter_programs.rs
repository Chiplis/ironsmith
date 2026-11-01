//! Generic parameterized programs: full dynamic repetition and ordered fresh viewed pools.
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
fn body(quality: &str) -> String {
    format!("Look at the top card of your library. You may reveal a {quality} card from among them and put it into your hand. You may put the rest into your graveyard.")
}
fn definitions(text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler_runtime::compile_to_artifact("Typed program witness", text, false)));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(decoded, artifact);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
#[derive(Default)]
struct TakeCards { views: usize }
impl DecisionMaker for TakeCards {
    fn view_cards(&mut self, _: &GameState, _: PlayerId, _: &[ObjectId], ctx: &ironsmith::decisions::context::ViewCardsContext) {
        if !ctx.public { self.views += 1; }
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
        if ctx.min == 0 {ctx.candidates.iter().filter(|o| o.legal).map(|o| o.id).take(1).collect()}
        else {SelectFirstDecisionMaker.decide_objects(game, ctx)}
    }
    fn decide_boolean(&mut self, _: &GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool {false}
}
fn card(game: &mut GameState, name: &str, line: &str) -> ObjectId {
    let definition = ironsmith_compiler_runtime::compile_to_runtime_definition(name, line, false).unwrap();
    game.create_object_from_definition(&definition, A, Zone::Library)
}
#[test]
fn fixed_repetition_continues_after_a_declined_optional_action() {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let source = game.new_object_id();
    let effect = Effect::new(ironsmith::effects::RepeatEffectsEffect::new(3, vec![
        Effect::gain_life(1),
        Effect::new(ironsmith::effects::MayEffect::new(vec![Effect::gain_life(10)])),
    ]));
    let mut dm = TakeCards::default();
    execute_effect(&mut game, &effect, &mut EffectContext::new_default(source, A).with_decision_maker(&mut dm)).unwrap();
    assert_eq!(game.player(A).unwrap().life, 23);
}
#[test]
fn dynamic_repeat_uses_resolution_source_power_and_repeats_the_entire_pool_program() {
    let text = format!("Type: Creature — Shaman\nPower/Toughness: 1/4\nWhenever this creature attacks, Repeat the following process X times, where X is this creature's power. {}", body("creature or enchantment"));
    for definition in definitions(&text) { for power in [0, 1, 3] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        for n in 0..4 {card(&mut game, &format!("Matching {n}"), "Type: Creature\nPower/Toughness: 1/1");}
        game.queue_trigger_event(Default::default(), ironsmith::triggers::TriggerEvent::new(ironsmith::events::CreatureAttackedEvent::new(source, ironsmith::events::AttackEventTarget::Player(PlayerId(1))), Default::default()));
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut dm = TakeCards::default();
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        // Change after triggering: the declared source expression is evaluated
        // at resolution, not replaced with its printed power by the importer.
        let modification = ironsmith::effects::ApplyContinuousEffect::new(
            ironsmith::continuous::EffectTarget::Specific(source),
            ironsmith::continuous::Modification::SetPower { value: power.into(), sublayer: ironsmith::continuous::PtSublayer::Setting },
            ironsmith::effect::Until::EndOfTurn);
        execute_effect(&mut game, &Effect::new(modification), &mut EffectContext::new_default(source, A)).unwrap();
        assert_eq!(game.calculated_power(source), Some(power));
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(dm.views, power as usize, "every iteration must perform a fresh private look");
        assert_eq!(game.player(A).unwrap().hand.len(), power as usize);
        assert_eq!(game.player(A).unwrap().library.len(), 4 - power as usize);
    } }
}
#[test]
fn seven_sequential_qualities_each_view_the_new_top_card_in_order() {
    let qualities = ["artifact", "creature", "enchantment", "instant", "land", "planeswalker", "sorcery"];
    let text = format!("Type: Sorcery\n{}", qualities.map(body).join(" "));
    for definition in definitions(&text) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let mut cards = Vec::new();
        for quality in qualities.into_iter().rev() {
            let extra = match quality {"creature" => "\nPower/Toughness: 1/1", "planeswalker" => "\nLoyalty: 3", "instant" | "sorcery" => "\nYou gain 1 life.", _ => ""};
            cards.push(card(&mut game, &format!("Only {quality}"), &format!("Type: {quality}{extra}")));
        }
        let source = game.create_object_from_definition(&definition, A, Zone::Stack);
        let effects = definition.spell_effect.as_ref().unwrap().all_effects_owned();
        let mut dm = TakeCards::default();
        execute_effect(&mut game, &Effect::new(ironsmith::effects::SequenceEffect::new(effects)), &mut EffectContext::new_default(source, A).with_decision_maker(&mut dm)).unwrap();
        assert_eq!(dm.views, 7);
        assert_eq!(game.player(A).unwrap().hand.len(), 7);
        assert!(game.player(A).unwrap().library.is_empty());
        assert!(game.player(A).unwrap().graveyard.is_empty());
        assert_eq!(cards.len(), 7);
    }
}
