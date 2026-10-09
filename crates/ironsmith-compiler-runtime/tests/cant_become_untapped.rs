//! "Can't become untapped" prohibits every untap, not only the untap step's
//! (Blossombind). Source-authored, deliberately UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effect::{Effect, Restriction, Until};
use ironsmith::effects::{CantEffect, EffectContext, execute_effect};
use ironsmith::target::{ChooseSpec, ObjectFilter};
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;

const A: PlayerId = PlayerId(0);
const BLOSSOMBIND: &str = "Mana cost: {1}{U}\nType: Enchantment — Aura\nEnchant creature\nWhen this Aura enters, tap enchanted creature.\nEnchanted creature can't become untapped and can't have counters put on it.";

#[test]
fn blossombind_lowers_both_prohibitions_onto_the_enchanted_creature() {
    for definition in support::definitions("Blossombind", BLOSSOMBIND) {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("BecomeUntapped"), "{debug}");
        assert!(debug.contains("HaveCountersPlaced"), "{debug}");
        assert!(!debug.contains("Untap(ObjectFilter"), "not the untap-step-only restriction");
    }
}

fn creature(game: &mut GameState) -> ObjectId {
    let def = compile_to_runtime_definition("Probe", "Type: Creature\nPower/Toughness: 2/2", false).unwrap();
    game.create_object_from_definition(&def, A, Zone::Battlefield)
}

#[test]
fn untap_effects_and_the_untap_step_both_fail_while_prohibited() {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.active_player = A;
    let target = creature(&mut game);
    let source = creature(&mut game);
    game.tap(target);
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = EffectContext::new(source, A, &mut dm);
    execute_effect(
        &mut game,
        &Effect::new(CantEffect::new(
            Restriction::become_untapped(ObjectFilter::specific(target)),
            Until::EndOfTurn,
        )),
        &mut ctx,
    )
    .unwrap();
    game.update_cant_effects();
    assert!(game.object(target).is_some(), "untap recipient still exists");
    ctx.targets = vec![ironsmith::effects::ResolvedTarget::Object(target)];
    execute_effect(&mut game, &Effect::untap(ChooseSpec::Object(ObjectFilter::specific(target))), &mut ctx).unwrap();
    assert!(game.is_tapped(target), "an untap effect can't untap it");
    assert!(!game.can_untap(target));
    game.untap(target);
    assert!(game.is_tapped(target), "no rule or primitive untaps it either");
}
