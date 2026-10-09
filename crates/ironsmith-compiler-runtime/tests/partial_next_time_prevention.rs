//! cf8/p07: partial one-shot prevention shields (CR 615.1, 615.7): "the next
//! time <source> would deal [combat] damage to you this turn, prevent half
//! that damage, rounded down / all but 1 of that damage". The shield applies
//! to the next matching damage event only and prevents only its part.
//! Source-authored, deliberately unrun.
#[path = "p07_support/mod.rs"]
mod support;

use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::SelectObjectsContext;
use ironsmith::effect::Effect;
use ironsmith::effects::{EffectContext, PreventNextTimeDamageEffect, PreventNextTimeDamageTarget, execute_effect};
use ironsmith::game_state::Phase;
use ironsmith::target::ChooseSpec;
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith_core::NextTimeDamagePreventionPortion;
use ironsmith_compiler_runtime::compile_to_runtime_definition;

const FIXTURE: &str = include_str!("../../../fixtures/partial_next_time_prevention.json.fixture");
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

fn shields(definition: &ironsmith::cards::CardDefinition) -> Vec<PreventNextTimeDamageEffect> {
    let activated = support::activated(definition);
    assert_eq!(activated.len(), 1);
    support::find::<PreventNextTimeDamageEffect>(&support::activated_effects(activated[0]))
}

/// Chooses one specific object as the shield's source.
struct ChooseSource(ObjectId);

impl DecisionMaker for ChooseSource {
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        if ctx.candidates.iter().any(|candidate| candidate.id == self.0) {
            return vec![self.0];
        }
        SelectFirstDecisionMaker.decide_objects(game, ctx)
    }
}

#[test]
fn dark_sphere_prevents_half_the_next_damage_rounded_down() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Dark Sphere");
    assert_eq!(row["oracle_id"], "397f53f7-f801-4442-a778-2f26ac246b62");
    for definition in support::definitions(row) {
        let shields = shields(&definition);
        assert_eq!(shields.len(), 1);
        assert_eq!(shields[0].portion, NextTimeDamagePreventionPortion::HalfRoundedDown);
        assert!(!shields[0].combat_only);
        assert!(matches!(shields[0].target, PreventNextTimeDamageTarget::You));

        // Gameplay: 5 damage from the chosen source becomes 3; the shield is
        // then used up, so the next 5 damage is dealt in full.
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = A;
        game.turn.phase = Phase::FirstMain;
        let sphere = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let bear = compile_to_runtime_definition(
            "Witness Bear",
            "Mana cost: {1}{G}\nType: Creature — Bear\nPower/Toughness: 2/2",
            false,
        )
        .unwrap();
        let bear = game.create_object_from_definition(&bear, B, Zone::Battlefield);
        let activated = support::activated(&definition);
        let mut dm = ChooseSource(bear);
        let mut ctx = EffectContext::new(sphere, A, &mut dm);
        for effect in activated[0].effects.all_effects() {
            execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        let hit = |game: &mut GameState| {
            let mut dm = SelectFirstDecisionMaker;
            execute_effect(
                game,
                &Effect::deal_damage(5, ChooseSpec::SpecificPlayer(A)),
                &mut EffectContext::new(bear, B, &mut dm),
            )
            .unwrap();
        };
        hit(&mut game);
        assert_eq!(game.player(A).unwrap().life, 17);
        hit(&mut game);
        assert_eq!(game.player(A).unwrap().life, 12);
    }
}

#[test]
fn forcefield_prevents_all_but_one_combat_damage_from_a_chosen_unblocked_creature() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Forcefield");
    assert_eq!(row["oracle_id"], "bd6823fb-a696-4e6d-9c5e-3b55dfe03730");
    for definition in support::definitions(row) {
        let shields = shields(&definition);
        assert_eq!(shields.len(), 1);
        assert_eq!(shields[0].portion, NextTimeDamagePreventionPortion::AllBut(1));
        assert!(shields[0].combat_only);
        assert!(matches!(shields[0].target, PreventNextTimeDamageTarget::You));
        let debug = format!("{:?}", shields[0].source);
        assert!(debug.contains("ChoiceMatching"), "{debug}");
        assert!(debug.contains("Creature"), "{debug}");
        // The choice is limited to unblocked creatures, not any creature.
        assert!(debug.contains("unblocked: true"), "{debug}");
    }
}
