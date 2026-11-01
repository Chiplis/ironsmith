//! Optional filtered reveal/hand and optional exact looked-set remainder.
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn definitions(quality: &str) -> [CardDefinition; 2] {
    let text = format!("Type: Sorcery\nLook at the top card of your library. You may reveal a {quality} card from among them and put it into your hand. You may put the rest into your graveyard.");
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler_runtime::compile_to_artifact("Looked selection program", &text, false)));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(restored, artifact);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
struct Choices { take: bool, grave: bool, views: Vec<(bool, PlayerId, Vec<ObjectId>)> }
impl DecisionMaker for Choices {
    fn decide_objects(&mut self, game: &GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
        if ctx.min == 0 {
            if self.take {ctx.candidates.iter().filter(|o| o.legal).map(|o| o.id).take(1).collect()} else {Vec::new()}
        } else {SelectFirstDecisionMaker.decide_objects(game, ctx)}
    }
    fn decide_boolean(&mut self, _: &GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool {self.grave}
    fn view_cards(&mut self, _: &GameState, viewer: PlayerId, cards: &[ObjectId], ctx: &ironsmith::decisions::context::ViewCardsContext) {
        self.views.push((ctx.public, viewer, cards.to_vec()));
    }
}
fn apply(game: &mut GameState, definition: &CardDefinition, dm: &mut Choices) {
    let source = game.create_object_from_definition(definition, A, Zone::Stack);
    let effects = definition.spell_effect.as_ref().unwrap().all_effects_owned();
    execute_effect(game, &Effect::new(ironsmith::effects::SequenceEffect::new(effects)), &mut EffectContext::new_default(source, A).with_decision_maker(dm)).unwrap();
}
#[test]
fn quality_selection_and_declined_remainder_preserve_choices_and_information() {
    for definition in definitions("Pirate or Rogue") {
        for eligible in [false, true] { for take in [false, true] { for grave in [false, true] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let top_definition = ironsmith_compiler_runtime::compile_to_runtime_definition("Viewed card", if eligible {"Type: Creature — Rogue\nPower/Toughness: 2/2"} else {"Type: Creature — Beast\nPower/Toughness: 2/2"}, false).unwrap();
            let sentinel = game.create_object_from_definition(&top_definition, A, Zone::Library);
            let top = game.create_object_from_definition(&top_definition, A, Zone::Library);
            let stable = game.object(top).unwrap().stable_id;
            let mut dm = Choices {take, grave, views: Vec::new()};
            apply(&mut game, &definition, &mut dm);
            let arrival = game.find_object_by_stable_id(stable).unwrap();
            let selected = eligible && take;
            let destination = if selected {Zone::Hand} else if grave {Zone::Graveyard} else {Zone::Library};
            assert_eq!(game.object(arrival).unwrap().zone, destination, "eligible={eligible} take={take} grave={grave}");
            assert_eq!(game.object(sentinel).unwrap().zone, Zone::Library, "exact complement never moves a fresh unrelated top card");
            assert!(dm.views.iter().any(|(public, viewer, cards)| !public && *viewer == A && cards.contains(&top)));
            assert!(!dm.views.iter().any(|(public, viewer, _)| !public && *viewer == B), "look remains private");
            assert_eq!(dm.views.iter().any(|(public, _, cards)| *public && cards.contains(&top)), selected, "only the selected matching hand card is revealed");
            if destination == Zone::Library {assert_eq!(game.player(A).unwrap().library.last(), Some(&top));}
        } } }
    }
}
#[test]
fn empty_library_does_not_create_a_card_or_public_information() {
    for definition in definitions("artifact or enchantment") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let mut dm = Choices {take: true, grave: true, views: Vec::new()};
        apply(&mut game, &definition, &mut dm);
        assert!(game.player(A).unwrap().hand.is_empty());
        assert!(game.player(A).unwrap().graveyard.is_empty());
        assert!(game.player(A).unwrap().library.is_empty());
        assert!(dm.views.iter().all(|(_, _, cards)| cards.is_empty()));
    }
}

#[test]
fn optional_remainder_rejects_nonword_costs_and_unread_suffixes() {
    for clause in ["You may put the rest into your graveyard {2}.", "You may put the rest into your graveyard unless seven impossible things happen."] {
        let text = format!("Type: Sorcery\nLook at the top card of your library. You may reveal an artifact card from among them and put it into your hand. {clause}");
        let result = ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler_runtime::compile_to_runtime_definition("Unread remainder", &text, false));
        assert!(result.is_err(), "unread remainder compiled: {clause}");
    }
}
