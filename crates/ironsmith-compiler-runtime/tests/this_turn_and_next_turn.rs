//! "This turn and next turn, ..." (Peace Talks): restrictions lasting through
//! the end of the next turn (CR 611.2a). Source-authored, deliberately UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effect::{Effect, Restriction, Until};
use ironsmith::effects::{CantEffect, EffectContext, execute_effect};
use ironsmith::target::ObjectFilter;
use ironsmith::{GameState, PlayerId, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;

const PEACE_TALKS: &str = "Mana cost: {1}{W}\nType: Sorcery\nThis turn and next turn, creatures can't attack, and players and permanents can't be the targets of spells or activated abilities.";

#[test]
fn peace_talks_lowers_three_restrictions_spanning_two_turns() {
    for definition in support::definitions("Peace Talks", PEACE_TALKS) {
        let cants = support::find_all::<CantEffect>(&definition);
        assert_eq!(cants.len(), 3, "{cants:#?}");
        assert!(cants.iter().all(|cant| cant.duration == Until::EndOfTurn
            && cant.duration_surface == ironsmith_core::RestrictionDurationSurface::ThisTurnAndNextTurn));
        assert!(cants.iter().any(|cant| matches!(cant.restriction, Restriction::Attack(_))));
        assert!(cants.iter().any(|cant| matches!(cant.restriction, Restriction::BeTargetedPlayerFrom(..))));
        assert!(cants.iter().any(|cant| matches!(cant.restriction, Restriction::BeTargetedFrom(..))));
    }
}

#[test]
fn the_restriction_survives_into_the_next_turn_and_ends_after_it() {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let body = compile_to_runtime_definition("Probe", "Type: Creature\nPower/Toughness: 2/2", false)
        .unwrap();
    let creature = game.create_object_from_definition(&body, PlayerId(1), Zone::Battlefield);
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = EffectContext::new(creature, PlayerId(0), &mut dm);
    execute_effect(
        &mut game,
        &Effect::new(
            CantEffect::new(Restriction::attack(ObjectFilter::creature()), Until::EndOfTurn)
                .with_duration_surface(ironsmith_core::RestrictionDurationSurface::ThisTurnAndNextTurn),
        ),
        &mut ctx,
    )
    .unwrap();
    let turn = game.turn.turn_number;
    let instance = game.effect_store.restriction_effects.last().unwrap();
    assert_eq!(instance.expires_end_of_turn, turn + 1);
    assert!(instance.is_active(&game, turn + 1));
    assert!(!instance.is_active(&game, turn + 2));
}

#[test]
fn peace_talks_renders_its_printed_two_turn_duration() {
    for definition in support::definitions("Peace Talks", PEACE_TALKS) {
        let rendered = ironsmith_text::canonical_compiled_lines(&definition).join("\n");
        assert!(rendered.contains("This turn and next turn, "), "{rendered}");
        assert!(!rendered.contains("until end of turn"), "{rendered}");
    }
}
