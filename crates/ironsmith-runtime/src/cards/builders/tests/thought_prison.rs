#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::CardDefinition;
use crate::decision::DecisionMaker;
use crate::effect::Effect;
use crate::effects::{
    ChooseObjectsEffect, ExecutionContext, IfEffect, LookAtHandEffect, MayEffect, MoveToZoneEffect,
    TagAllEffect, execute_effect,
};
use crate::filter::{ObjectCharacteristicRelationKind, TaggedOpbjectRelation};

const THOUGHT_PRISON_ORACLE: &str = "Imprint — When this artifact enters, you may have target player reveal their hand. If you do, choose a nonland card from it and exile that card.\nWhenever a player casts a spell that shares a color or mana value with the exiled card, this artifact deals 2 damage to that player.";

fn nested_look_at_hand(effect: &Effect) -> Option<&LookAtHandEffect> {
    if let Some(look) = effect.downcast_ref::<LookAtHandEffect>() {
        return Some(look);
    }
    if let Some(with_id) = effect.downcast_ref::<WithIdEffect>() {
        return nested_look_at_hand(&with_id.effect);
    }
    if let Some(may) = effect.downcast_ref::<MayEffect>() {
        return may.effects.iter().find_map(nested_look_at_hand);
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        return sequence.effects.iter().find_map(nested_look_at_hand);
    }
    if let Some(if_effect) = effect.downcast_ref::<IfEffect>() {
        return if_effect
            .then
            .iter()
            .chain(&if_effect.else_)
            .find_map(nested_look_at_hand);
    }
    if let Some(tagged) = effect.downcast_ref::<TaggedEffect>() {
        return nested_look_at_hand(&tagged.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<TagAllEffect>() {
        return nested_look_at_hand(&tagged.effect);
    }
    None
}

fn nested_choose_objects(effect: &Effect) -> Option<&ChooseObjectsEffect> {
    if let Some(choose) = effect.downcast_ref::<ChooseObjectsEffect>() {
        return Some(choose);
    }
    if let Some(with_id) = effect.downcast_ref::<WithIdEffect>() {
        return nested_choose_objects(&with_id.effect);
    }
    if let Some(may) = effect.downcast_ref::<MayEffect>() {
        return may.effects.iter().find_map(nested_choose_objects);
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        return sequence.effects.iter().find_map(nested_choose_objects);
    }
    if let Some(if_effect) = effect.downcast_ref::<IfEffect>() {
        return if_effect
            .then
            .iter()
            .chain(&if_effect.else_)
            .find_map(nested_choose_objects);
    }
    if let Some(tagged) = effect.downcast_ref::<TaggedEffect>() {
        return nested_choose_objects(&tagged.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<TagAllEffect>() {
        return nested_choose_objects(&tagged.effect);
    }
    None
}

fn nested_move_to_zone(effect: &Effect) -> Option<&MoveToZoneEffect> {
    if let Some(move_to_zone) = effect.downcast_ref::<MoveToZoneEffect>() {
        return Some(move_to_zone);
    }
    if let Some(with_id) = effect.downcast_ref::<WithIdEffect>() {
        return nested_move_to_zone(&with_id.effect);
    }
    if let Some(may) = effect.downcast_ref::<MayEffect>() {
        return may.effects.iter().find_map(nested_move_to_zone);
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        return sequence.effects.iter().find_map(nested_move_to_zone);
    }
    if let Some(if_effect) = effect.downcast_ref::<IfEffect>() {
        return if_effect
            .then
            .iter()
            .chain(&if_effect.else_)
            .find_map(nested_move_to_zone);
    }
    if let Some(tagged) = effect.downcast_ref::<TaggedEffect>() {
        return nested_move_to_zone(&tagged.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<TagAllEffect>() {
        return nested_move_to_zone(&tagged.effect);
    }
    None
}

fn imprint_trigger(definition: &CardDefinition) -> &crate::ability::TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered
                    .effects
                    .segments
                    .iter()
                    .flat_map(|segment| &segment.default_effects)
                    .any(|effect| nested_look_at_hand(effect).is_some()) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Thought Prison should retain its imprint enters trigger")
}

#[test]
fn thought_prison_preserves_exact_text_and_typed_reveal_choice_provenance() {
    let definition = parse_oracle_card_definition("Thought Prison");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        THOUGHT_PRISON_ORACLE
    );

    let imprint = imprint_trigger(&definition);
    let choose = imprint
        .effects
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .find_map(nested_choose_objects)
        .expect("the successful reveal branch should choose one object");
    assert_eq!(choose.count, crate::effect::ChoiceCount::exactly(1));
    assert_eq!(choose.zone, Some(Zone::Hand));
    assert_eq!(choose.filter.zone, Some(Zone::Hand));
    assert!(choose.filter.excluded_card_types.contains(&CardType::Land));
    assert!(
        choose.filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag.as_str() == crate::tag::REVEALED_THIS_WAY_TAG
        }),
        "the choice must be constrained to the exact reveal result, not merely the target player's hand: {choose:#?}"
    );

    let exile = imprint
        .effects
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .find_map(nested_move_to_zone)
        .expect("the chosen card should move to exile");
    assert_eq!(exile.zone, Zone::Exile);
    assert!(
        matches!(
            exile.target.unhinted(),
            ChooseSpec::Tagged(tag) if tag == &choose.tag
        ),
        "exile must consume the singular chosen-card tag: {exile:#?}"
    );

    let cast_trigger = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .trigger
                .downcast_ref::<crate::triggers::SpellCastTrigger>(),
            _ => None,
        })
        .expect("Thought Prison should retain its spell-cast trigger");
    let filter = cast_trigger
        .filter
        .as_ref()
        .expect("the spell-cast trigger should have a characteristic filter");
    let relation = filter
        .characteristic_relations
        .first()
        .expect("the trigger should compare against the imprinted card");
    assert_eq!(relation.kind, ObjectCharacteristicRelationKind::SharesAny);
    assert_eq!(
        relation.characteristics,
        [
            crate::ObjectCharacteristic::Color,
            crate::ObjectCharacteristic::ManaValue
        ]
    );
    assert_eq!(relation.comparison.zone, Some(Zone::Exile));
    assert!(
        relation
            .comparison
            .tagged_constraints
            .iter()
            .any(|constraint| {
                constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                    && constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
            }),
        "the later trigger must compare with the card exiled by this source: {relation:#?}"
    );
}

#[derive(Debug, Default)]
struct SelectLastDecisionMaker {
    object_choices: Vec<Vec<(ObjectId, bool)>>,
}

impl DecisionMaker for SelectLastDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &crate::game_state::GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        true
    }

    fn decide_objects(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        self.object_choices.push(
            ctx.candidates
                .iter()
                .map(|candidate| (candidate.id, candidate.legal))
                .collect(),
        );
        let count = ctx.max.unwrap_or(1).min(
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .count(),
        );
        ctx.candidates
            .iter()
            .rev()
            .filter(|candidate| candidate.legal)
            .take(count)
            .map(|candidate| candidate.id)
            .collect()
    }
}

fn test_card(raw_id: u32, name: &str, card_type: CardType) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(raw_id), name)
        .card_types(vec![card_type])
        .build()
}

#[test]
fn thought_prison_exiles_only_a_card_present_when_the_target_hand_was_revealed() {
    let definition = parse_oracle_card_definition("Thought Prison");
    let imprint = imprint_trigger(&definition);
    let [reveal_segment, choice_segment] = imprint.effects.segments.as_slice() else {
        panic!(
            "the optional reveal and if-you-do choice should remain two segments: {:#?}",
            imprint.effects
        );
    };

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let revealed_card = game.create_object_from_definition(
        &test_card(97_001, "Revealed Nonland", CardType::Instant),
        bob,
        Zone::Hand,
    );
    let revealed_land = game.create_object_from_definition(
        &test_card(97_002, "Revealed Land", CardType::Land),
        bob,
        Zone::Hand,
    );
    let revealed_stable = game
        .object(revealed_card)
        .expect("revealed card exists")
        .stable_id;
    let land_stable = game
        .object(revealed_land)
        .expect("revealed land exists")
        .stable_id;

    let target_spec = imprint
        .choices
        .first()
        .cloned()
        .expect("the reveal should declare target player");
    let mut decisions = SelectLastDecisionMaker::default();
    let mut ctx = ExecutionContext::new(source, alice, &mut decisions)
        .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: target_spec,
            range: 0..1,
        }]);
    ctx.snapshot_targets(&game);

    for effect in &reveal_segment.default_effects {
        execute_effect(&mut game, effect, &mut ctx)
            .expect("Thought Prison's accepted reveal should resolve");
    }
    assert!(
        ctx.get_tagged_all(crate::tag::REVEALED_THIS_WAY_TAG)
            .is_some_and(|revealed| {
                revealed
                    .iter()
                    .any(|snapshot| snapshot.object_id == revealed_card)
                    && revealed
                        .iter()
                        .any(|snapshot| snapshot.object_id == revealed_land)
            }),
        "the reveal should publish the exact hand snapshot before the choice"
    );

    // This card has the right owner, zone, and nonland type, but it was not in
    // the revealed result set. A widened owner-only filter would let the
    // select-last decision maker exile it.
    let late_card = game.create_object_from_definition(
        &test_card(97_003, "Late Nonland", CardType::Sorcery),
        bob,
        Zone::Hand,
    );
    let late_stable = game.object(late_card).expect("late card exists").stable_id;

    for effect in &choice_segment.default_effects {
        execute_effect(&mut game, effect, &mut ctx)
            .expect("Thought Prison's successful choice branch should resolve");
    }
    drop(ctx);

    let revealed_after = game
        .find_object_by_stable_id(revealed_stable)
        .expect("revealed card should retain stable identity");
    let late_after = game
        .find_object_by_stable_id(late_stable)
        .expect("late card should retain stable identity");
    let land_after = game
        .find_object_by_stable_id(land_stable)
        .expect("land should retain stable identity");
    assert_eq!(
        game.object(revealed_after)
            .expect("revealed card exists")
            .zone,
        Zone::Exile
    );
    assert_eq!(
        game.object(late_after).expect("late card exists").zone,
        Zone::Hand,
        "a card added after the reveal must not become a legal `from it` choice"
    );
    assert_eq!(
        game.object(land_after).expect("land exists").zone,
        Zone::Hand,
        "the nonland restriction must remain active inside the revealed set"
    );
    assert_eq!(
        game.get_exiled_with_source_links(source),
        &[revealed_after],
        "the exact chosen card must also become Thought Prison's linked exiled card"
    );
    assert!(
        decisions
            .object_choices
            .iter()
            .flatten()
            .find(|(candidate, _)| *candidate == late_card)
            .is_none_or(|(_, legal)| !legal),
        "the late card must never be presented as a legal choice"
    );
}
