//! Acclimate's player action chooses from a union without restricting the fallback to targets.
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext};
use ironsmith_core::custom_subtypes::CustomSubtypeDefinition;
use ironsmith_core::SubtypeFamily;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn card(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    ironsmith_compiler::with_custom_subtypes(&[CustomSubtypeDefinition {singular:"Weather".into(),plural:"Weather".into(),family:SubtypeFamily::Enchantment}], || {
        ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false))
    }).unwrap().unwrap()
}
struct Choose { accept: bool, choice: Option<ObjectId> }
impl DecisionMaker for Choose {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool { self.accept }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        if let Some(id) = self.choice { if ctx.candidates.iter().any(|c| c.id == id && c.legal) { return vec![id]; } }
        SelectFirstDecisionMaker.decide_objects(game, ctx)
    }
}
#[test]
fn player_acclimate_uses_owned_union_choice_or_nontargeted_counter() {
    let spell = card("Weather choice", "Type: Sorcery\nYou may choose a Weather card you own from your graveyard or from outside the game. If you do, reveal it and put it into your hand. Otherwise, put a +1/+1 counter on a creature you control.");
    let weather = card("Rain", "Type: Enchantment — Weather");
    let nonweather = card("Plain enchantment", "Type: Enchantment");
    let creature = card("Sheltered creature", "Type: Creature\nPower/Toughness: 1/1\nShroud");
    for zone in [Zone::Graveyard, Zone::OutsideGame] { for accept in [false, true] { for available in [false, true] {
        let mut game = GameState::new(vec!["A".into(),"B".into()],20);
        let source = game.create_object_from_definition(&spell,A,Zone::Stack);
        let own = available.then(|| game.create_object_from_definition(&weather,A,zone));
        let wrong_owner = game.create_object_from_definition(&weather,B,zone);
        let wrong_kind = game.create_object_from_definition(&nonweather,A,zone);
        let host = game.create_object_from_definition(&creature,A,Zone::Battlefield);
        let mut dm = Choose {accept,choice:own};
        let mut ctx = ironsmith::effects::EffectContext::new(source,A,&mut dm);
        for effect in spell.spell_effect.as_ref().unwrap().all_effects() { ironsmith::effects::execute_effect(&mut game,effect,&mut ctx).unwrap(); }
        assert_eq!(game.object(wrong_owner).unwrap().zone,zone);
        assert_eq!(game.object(wrong_kind).unwrap().zone,zone);
        assert_eq!(game.player(A).unwrap().hand.len(),usize::from(accept && available));
        assert_eq!(game.counter_count(host,ironsmith::object::CounterType::PlusOnePlusOne),u32::from(!accept || !available));
    } } }
}
