//! "Put <objects> on top of their owners' libraries, then those players
//! shuffle": every object moves first, then each distinct owner shuffles
//! exactly once (CR 701.24a). Source-authored, deliberately UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effect::Effect;
use ironsmith::effects::{EffectContext, MoveToZoneEffect, ShuffleLibraryEffect, execute_effect};
use ironsmith::events::ShuffleLibraryEvent;
use ironsmith::snapshot::ObjectSnapshot;
use ironsmith::target::{ObjectRef, PlayerFilter};
use ironsmith::{GameState, PlayerId, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

const VOID_STALKER: &str = "Mana cost: {1}{U}\nType: Creature — Elemental\nPower/Toughness: 2/1\n{2}{U}, {T}: Put this creature and target creature on top of their owners' libraries, then those players shuffle their libraries.";
const VORTEX_ELEMENTAL: &str = "Mana cost: {U}\nType: Creature — Elemental\nPower/Toughness: 0/1\n{U}: Put this creature and each creature blocking or blocked by it on top of their owners' libraries, then those players shuffle.\n{3}{U}{U}: Target creature blocks this creature this turn if able.";

fn shuffles(definition: &ironsmith::cards::CardDefinition) -> Vec<ShuffleLibraryEffect> {
    support::find_all::<ShuffleLibraryEffect>(definition)
}

#[test]
fn void_stalker_moves_both_objects_then_shuffles_their_owners_once() {
    for definition in support::definitions("Void Stalker", VOID_STALKER) {
        let moves = support::find_all::<MoveToZoneEffect>(&definition);
        assert_eq!(moves.len(), 2, "{moves:#?}");
        assert!(moves.iter().all(|m| m.zone == Zone::Library && m.to_top));
        let shuffles = shuffles(&definition);
        assert_eq!(shuffles.len(), 1, "one owner-set shuffle: {shuffles:#?}");
        assert!(
            matches!(&shuffles[0].player, PlayerFilter::OwnerOf(ObjectRef::Tagged(_))),
            "{shuffles:#?}"
        );
    }
}

#[test]
fn vortex_elemental_moves_the_combat_group_then_shuffles_their_owners_once() {
    for definition in support::definitions("Vortex Elemental", VORTEX_ELEMENTAL) {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("in_combat_with_source: true"), "{debug}");
        assert_eq!(shuffles(&definition).len(), 1, "{debug}");
    }
}

#[test]
fn an_owner_of_two_moved_objects_shuffles_exactly_once() {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let body = compile_to_runtime_definition("Moved", "Type: Creature\nPower/Toughness: 1/1", false)
        .unwrap();
    let mut snapshots = Vec::new();
    for owner in [A, A, B] {
        let id = game.create_object_from_definition(&body, owner, Zone::Library);
        snapshots.push(ObjectSnapshot::from_object(game.object(id).unwrap(), &game));
    }
    let source = game.create_object_from_definition(&body, A, Zone::Battlefield);
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = EffectContext::new(source, A, &mut dm);
    ctx.tag_objects("moved", snapshots);
    let outcome = execute_effect(
        &mut game,
        &Effect::new(ShuffleLibraryEffect::new(PlayerFilter::OwnerOf(ObjectRef::tagged(
            "moved",
        )))),
        &mut ctx,
    )
    .unwrap();
    let shuffled: Vec<PlayerId> = outcome
        .events
        .iter()
        .filter_map(|event| event.downcast::<ShuffleLibraryEvent>().map(|e| e.player))
        .collect();
    assert_eq!(shuffled, vec![A, B], "each owner once, APNAP order");
}
