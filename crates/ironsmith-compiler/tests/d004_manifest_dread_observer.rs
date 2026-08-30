use ironsmith_compiler::ability::AbilityKind;
use ironsmith_compiler::cards::CardDefinitionBuilder;
use ironsmith_compiler::effects::{MoveToZoneEffect, TagTriggeringObjectEffect};
use ironsmith_compiler::events::KeywordActionKind;
use ironsmith_compiler::ids::CardId;
use ironsmith_compiler::tag::CompilerReferenceTag;
use ironsmith_compiler::target::{ChooseSpec, PlayerFilter, TaggedOpbjectRelation};
use ironsmith_compiler::triggers::TriggerKind;
use ironsmith_compiler::types::CardType;
use ironsmith_compiler::zone::Zone;

#[test]
fn manifest_dread_observer_lowers_to_the_distinct_action_and_tagged_graveyard_object() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Paranormal Analyst Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever you manifest dread, put a card you put into your graveyard this way into your hand.",
        )
        .expect("manifest-dread observer should compile");

    let AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
        panic!("expected a triggered ability: {:#?}", definition.abilities);
    };
    assert_eq!(
        triggered.trigger.kind,
        TriggerKind::KeywordAction {
            action: KeywordActionKind::ManifestDread,
            player: PlayerFilter::You,
        }
    );

    let [tag_effect, move_effect] = triggered.effects.flattened_default_effects() else {
        panic!("expected a prelude and move effect: {triggered:#?}");
    };
    let tag = tag_effect
        .downcast_ref::<TagTriggeringObjectEffect>()
        .unwrap_or_else(|| panic!("expected triggering-object prelude: {triggered:#?}"));
    assert_eq!(
        tag.tag.as_str(),
        CompilerReferenceTag::ManifestDreadGraveyard.as_str()
    );

    let move_to_hand = move_effect
        .downcast_ref::<MoveToZoneEffect>()
        .unwrap_or_else(|| panic!("expected move-to-hand effect: {triggered:#?}"));
    assert_eq!(move_to_hand.zone, Zone::Hand);
    match move_to_hand.target.base() {
        ChooseSpec::Tagged(move_tag) => {
            assert_eq!(
                move_tag.as_str(),
                CompilerReferenceTag::ManifestDreadGraveyard.as_str()
            );
        }
        ChooseSpec::Object(filter) => {
            assert_eq!(filter.zone, Some(Zone::Graveyard));
            assert_eq!(filter.tagged_constraints.len(), 1);
            assert_eq!(
                filter.tagged_constraints[0].tag.as_str(),
                CompilerReferenceTag::ManifestDreadGraveyard.as_str()
            );
            assert_eq!(
                filter.tagged_constraints[0].relation,
                TaggedOpbjectRelation::IsTaggedObject
            );
        }
        other => panic!("expected a tagged graveyard-card move, got {other:#?}"),
    }
}
