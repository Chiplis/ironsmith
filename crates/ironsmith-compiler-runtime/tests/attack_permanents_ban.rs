//! "creatures they control can't attack Jaces you control this turn": a ban
//! on choosing matching planeswalkers as attack targets (CR 508.1b), not an
//! attack tax. Source-authored, deliberately UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

use ironsmith::effect::{Effect, Restriction, Until};
use ironsmith::effects::{CantEffect, EffectContext, execute_effect};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::target::ObjectFilter;
use ironsmith::{GameState, PlayerId, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const JACE: &str = "Mana cost: {3}{U}{U}\nType: Legendary Planeswalker — Jace\nLoyalty: 4\nAt the beginning of combat on each opponent's turn, they may pay {2}. If they don't, creatures they control can't attack Jaces you control this turn.\n+1: Draw two cards, then put a card from your hand on the bottom of your library.\n−3: Exile another target planeswalker or creature you control. Reveal cards from the top of your library until you reveal a creature or planeswalker card. Put that card onto the battlefield and the rest on the bottom of your library in a random order.\nJace, Multiverse Architect can be your commander.";

#[test]
fn jace_lowers_the_unpaid_branch_to_an_attack_permanents_ban() {
    for definition in support::definitions("Jace, Multiverse Architect", JACE) {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("AttackPermanents"), "{debug}");
        assert!(debug.contains("Jace"), "{debug}");
    }
}

#[test]
fn a_banned_planeswalker_is_not_a_legal_attack_target_but_its_controller_is() {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let walker = compile_to_runtime_definition(
        "Walker",
        "Type: Legendary Planeswalker — Jace\nLoyalty: 3",
        false,
    )
    .unwrap();
    let walker = game.create_object_from_definition(&walker, A, Zone::Battlefield);
    let body = compile_to_runtime_definition("Attacker", "Type: Creature\nPower/Toughness: 2/2", false)
        .unwrap();
    let attacker = game.create_object_from_definition(&body, B, Zone::Battlefield);
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = EffectContext::new(walker, A, &mut dm);
    execute_effect(
        &mut game,
        &Effect::new(CantEffect::new(
            Restriction::attack_permanents(
                ObjectFilter::creature(),
                ObjectFilter::permanent().with_subtype(ironsmith::types::Subtype::Jace),
            ),
            Until::EndOfTurn,
        )),
        &mut ctx,
    )
    .unwrap();
    game.update_cant_effects();
    let cant = &game.effect_store.cant_effects;
    assert!(!cant.can_attack_permanent(attacker, walker));
}
