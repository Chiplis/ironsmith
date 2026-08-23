use super::shard_16::{assert_oracle_card_parses_strict, parse_oracle_card_definition};
use super::*;

fn assert_revealed_collection_count(name: &str, effects: &[crate::effect::Effect], rendered: &str) {
    let consult = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>())
        .unwrap_or_else(|| panic!("{name} should contain a typed library consult: {effects:#?}"));
    let damage = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::DealDamageEffect>())
        .unwrap_or_else(|| panic!("{name} should contain typed damage: {effects:#?}"));
    let crate::effect::Value::Count(counted) = damage.amount.unhinted() else {
        panic!("{name} damage should count the revealed collection: {damage:#?}");
    };
    let counted_tag = counted
        .tagged_constraints
        .iter()
        .find(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        })
        .map(|constraint| &constraint.tag)
        .unwrap_or_else(|| panic!("{name} damage count should retain a typed tag: {counted:#?}"));

    assert_eq!(
        counted_tag, &consult.all_tag,
        "{name} should count every card exposed by the consult"
    );
    assert_ne!(
        counted_tag, &consult.match_tag,
        "{name} must not count only the singular matching card"
    );
    assert!(
        rendered.contains("the number of cards revealed this way")
            && !rendered.contains("the number of those cards"),
        "{name} should retain the revealed-collection antecedent in compiled text: {rendered}"
    );
}

#[test]
fn audacious_reshapers_and_madcap_experiment_count_all_cards_revealed_this_way() {
    assert_oracle_card_parses_strict("Audacious Reshapers");
    let audacious = parse_oracle_card_definition("Audacious Reshapers");
    let activated = audacious
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Audacious Reshapers should have an activated ability");
    let audacious_rendered = canonical_compiled_lines(&audacious).join("\n");
    assert_revealed_collection_count(
        "Audacious Reshapers",
        activated.effects.flattened_default_effects(),
        &audacious_rendered,
    );

    assert_oracle_card_parses_strict("Madcap Experiment");
    let madcap = parse_oracle_card_definition("Madcap Experiment");
    let spell_effect = madcap
        .spell_effect
        .as_ref()
        .expect("Madcap Experiment should have a spell effect");
    let madcap_rendered = canonical_compiled_lines(&madcap).join("\n");
    assert_revealed_collection_count(
        "Madcap Experiment",
        spell_effect.flattened_default_effects(),
        &madcap_rendered,
    );
}
