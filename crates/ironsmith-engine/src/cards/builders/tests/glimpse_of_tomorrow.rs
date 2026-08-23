#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const EXPECTED: &str = "Suspend 3—{R}{R}\nShuffle all permanents you own into your library, then reveal that many cards from the top of your library. Put all non-Aura permanent cards revealed this way onto the battlefield, then do the same for Aura cards, then put the rest on the bottom of your library in a random order.";

fn collect_nested_effects<'a>(effect: &'a Effect, collected: &mut Vec<&'a Effect>) {
    collected.push(effect);
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        collect_nested_effects(&with_id.effect, collected);
    } else if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        for child in &sequence.effects {
            collect_nested_effects(child, collected);
        }
    } else if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        collect_nested_effects(&tagged.effect, collected);
    }
}

#[test]
fn glimpse_public_payload_keeps_two_ordered_revealed_permanent_partitions() {
    let definition = parse_oracle_card_definition("Glimpse of Tomorrow");
    let program = definition
        .spell_effect
        .as_ref()
        .expect("Glimpse should have a spell-resolution program");
    let mut effects = Vec::new();
    for segment in &program.segments {
        for effect in &segment.default_effects {
            collect_nested_effects(effect, &mut effects);
        }
    }
    let captures = effects
        .iter()
        .filter_map(|effect| effect.downcast_ref::<crate::effects::TagMatchingObjectsEffect>())
        .collect::<Vec<_>>();
    let [all_moved, non_aura, aura] = captures.as_slice() else {
        panic!("the union and both ordered groups must be captured separately: {program:#?}");
    };
    let look = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>())
        .expect("the shuffle count must feed one revealed top-card collection");
    assert_eq!(all_moved.filter.any_of.len(), 2, "{all_moved:#?}");
    assert_eq!(non_aura.filter, all_moved.filter.any_of[0]);
    assert_eq!(aura.filter, all_moved.filter.any_of[1]);
    for partition in [&non_aura.filter, &aura.filter] {
        let [constraint] = partition.tagged_constraints.as_slice() else {
            panic!("each partition must consume exactly one revealed collection: {partition:#?}");
        };
        assert_eq!(
            constraint.relation,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject
        );
        assert_eq!(
            constraint.tag, look.tag,
            "partition filters must consume the collection populated by LookAtTopCards"
        );
    }

    let moves = effects
        .iter()
        .filter_map(|effect| effect.downcast_ref::<crate::effects::ForEachTaggedEffect>())
        .collect::<Vec<_>>();
    assert_eq!(moves.len(), 2, "{program:#?}");
    assert_eq!(moves[0].tag, non_aura.tag);
    assert_eq!(moves[1].tag, aura.tag);
    let remainder = effects
        .iter()
        .find_map(|effect| {
            effect.downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()
        })
        .expect("revealed remainder should be retained");
    assert_eq!(
        remainder.tag, look.tag,
        "the remainder must be calculated from the same revealed collection"
    );
    assert_eq!(remainder.keep_tagged.as_ref(), Some(&all_moved.tag));

    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        EXPECTED,
        "ordered typed payload should render through the exact Glimpse surface: {program:#?}"
    );
}

#[test]
fn glimpse_reuses_the_revealed_collection_when_resolving_both_permanent_groups() {
    let definition = parse_oracle_card_definition("Glimpse of Tomorrow");
    let program = definition
        .spell_effect
        .as_ref()
        .expect("Glimpse should have a spell-resolution program");
    let non_aura = CardDefinitionBuilder::new(CardId::new(), "Glimpse Creature")
        .card_types(vec![CardType::Creature])
        .build();
    let aura = CardDefinitionBuilder::new(CardId::new(), "Glimpse Aura")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .build();
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let creature = game.create_object_from_definition(&non_aura, alice, Zone::Battlefield);
    let aura = game.create_object_from_definition(&aura, alice, Zone::Battlefield);
    let creature_stable = game.object(creature).expect("creature").stable_id;
    let aura_stable = game.object(aura).expect("Aura").stable_id;

    let mut context = crate::effects::ExecutionContext::new_default(source, alice);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        source,
        program,
        None,
        &[],
    )
    .expect("the revealed collection should remain executable across both segments");

    for stable in [creature_stable, aura_stable] {
        let object = game
            .find_object_by_stable_id(stable)
            .and_then(|id| game.object(id))
            .expect("shuffled permanent should retain stable identity");
        assert_eq!(
            object.zone,
            Zone::Battlefield,
            "both ordered partitions must use the populated reveal tag"
        );
    }
}
