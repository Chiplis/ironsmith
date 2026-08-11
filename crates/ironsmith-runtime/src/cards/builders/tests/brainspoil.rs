#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn find_destroy_no_regeneration(
    effect: &crate::effect::Effect,
) -> Option<&crate::effects::DestroyNoRegenerationEffect> {
    if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyNoRegenerationEffect>() {
        return Some(destroy);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return find_destroy_no_regeneration(&with_id.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return find_destroy_no_regeneration(&tagged.effect);
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        return sequence
            .effects
            .iter()
            .find_map(find_destroy_no_regeneration);
    }
    None
}

#[test]
fn brainspoil_keeps_the_unenchanted_target_on_its_no_regeneration_destroy() {
    let definition = parse_oracle_card_definition("Brainspoil");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Destroy target creature that isn't enchanted. It can't be regenerated.\nTransmute {1}{B}{B}"
    );

    let destroy = definition
        .spell_effect
        .as_ref()
        .expect("Brainspoil should have a spell program")
        .flattened_default_effects()
        .into_iter()
        .find_map(find_destroy_no_regeneration)
        .expect("Brainspoil should lower to one no-regeneration destroy");
    let ChooseSpec::Target(inner) = destroy.spec.unhinted() else {
        panic!("Brainspoil should retain a targeted destroy: {destroy:#?}");
    };
    let ChooseSpec::Object(filter) = inner.unhinted() else {
        panic!("Brainspoil should target an object: {destroy:#?}");
    };
    assert_eq!(filter.card_types, vec![CardType::Creature]);
    let excluded_attachment = filter
        .without_attached_object
        .as_deref()
        .expect("the target must not have an Aura attached");
    assert_eq!(excluded_attachment.card_types, vec![CardType::Enchantment]);
    assert_eq!(excluded_attachment.subtypes, vec![Subtype::Aura]);
}
