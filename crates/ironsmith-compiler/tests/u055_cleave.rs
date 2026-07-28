use ironsmith_compiler::alternative_cast::AlternativeCastingMethod;
use ironsmith_compiler::cards::CardDefinitionBuilder;
use ironsmith_compiler::effect::Effect;
use ironsmith_compiler::effects::{
    CounterEffect, DestroyEffect, ReturnToHandEffect, SearchLibraryEffect, TaggedEffect,
};
use ironsmith_compiler::filter::Comparison;
use ironsmith_compiler::ids::CardId;
use ironsmith_compiler::target::{ChooseSpec, PlayerFilter};
use ironsmith_compiler::types::{CardType, Supertype};
use ironsmith_compiler::zone::Zone;

fn unwrap_tagged(effect: &Effect) -> &Effect {
    if let Some(tagged) = effect.downcast_ref::<TaggedEffect>() {
        unwrap_tagged(&tagged.effect)
    } else {
        effect
    }
}

#[test]
fn cleave_compiles_normal_and_bracket_removed_search_programs() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Dig Up")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Cleave {1}{B}{B}{G} (You may cast this spell for its cleave cost. If you do, remove the words in square brackets.)\n\
             Search your library for a [basic land] card, [reveal it,] put it into your hand, then shuffle.",
        )
        .expect("Dig Up should compile");

    let normal = definition
        .spell_effect
        .as_ref()
        .expect("normal spell program")
        .flattened_default_effects();
    let normal_search = normal
        .iter()
        .find_map(|effect| unwrap_tagged(effect).downcast_ref::<SearchLibraryEffect>())
        .expect("normal Dig Up should search");
    assert_eq!(normal_search.filter.card_types, vec![CardType::Land]);
    assert_eq!(normal_search.filter.supertypes, vec![Supertype::Basic]);
    assert!(normal_search.reveal);

    let cleave = definition
        .alternative_casts
        .iter()
        .find_map(|method| match method {
            AlternativeCastingMethod::Cleave { cost, effects } => Some((cost, effects)),
            _ => None,
        })
        .expect("Dig Up should have a typed Cleave method");
    assert_eq!(cleave.0.to_oracle(), "{1}{B}{B}{G}");
    let cleave_search = cleave
        .1
        .iter()
        .find_map(|effect| unwrap_tagged(effect).downcast_ref::<SearchLibraryEffect>())
        .expect("cleaved Dig Up should search");
    assert!(cleave_search.filter.card_types.is_empty());
    assert!(cleave_search.filter.supertypes.is_empty());
    assert!(!cleave_search.reveal);
}

#[test]
fn cleave_changes_target_legality_before_announcement() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Alchemist's Retrieval")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Cleave {1}{U}\nReturn target nonland permanent [you control] to its owner's hand.",
        )
        .expect("Alchemist's Retrieval should compile");

    let normal = definition
        .spell_effect
        .as_ref()
        .expect("normal spell program")
        .flattened_default_effects();
    let normal_return = normal
        .iter()
        .find_map(|effect| unwrap_tagged(effect).downcast_ref::<ReturnToHandEffect>())
        .expect("normal Retrieval should return a permanent");
    let ChooseSpec::Object(normal_filter) = normal_return.spec.base() else {
        panic!("normal Retrieval should carry an object target: {normal_return:#?}");
    };
    assert_eq!(normal_filter.controller, Some(PlayerFilter::You));

    let cleave_effects = definition
        .alternative_casts
        .iter()
        .find_map(|method| match method {
            AlternativeCastingMethod::Cleave { effects, .. } => Some(effects),
            _ => None,
        })
        .expect("Retrieval should have a typed Cleave method");
    let cleave_return = cleave_effects
        .iter()
        .find_map(|effect| unwrap_tagged(effect).downcast_ref::<ReturnToHandEffect>())
        .expect("cleaved Retrieval should return a permanent");
    let ChooseSpec::Object(cleave_filter) = cleave_return.spec.base() else {
        panic!("cleaved Retrieval should carry an object target: {cleave_return:#?}");
    };
    assert_eq!(cleave_filter.controller, None);
}

#[test]
fn cleave_instances_remain_independent_alternative_costs() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Double Cleave Probe")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Cleave {1}{U}\nCleave {2}{U}\nReturn target permanent [you control] to its owner's hand.",
        )
        .expect("multiple Cleave instances should compile");

    let cleave_methods = definition
        .alternative_casts
        .iter()
        .filter_map(|method| match method {
            AlternativeCastingMethod::Cleave { cost, effects } => Some((cost, effects)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(cleave_methods.len(), 2);
    assert_eq!(cleave_methods[0].0.to_oracle(), "{1}{U}");
    assert_eq!(cleave_methods[1].0.to_oracle(), "{2}{U}");
    for (_, effects) in cleave_methods {
        let returned = effects
            .iter()
            .find_map(|effect| unwrap_tagged(effect).downcast_ref::<ReturnToHandEffect>())
            .expect("each Cleave method should carry the bracket-removed program");
        let ChooseSpec::Object(filter) = returned.spec.base() else {
            panic!("cleaved program should keep its target: {returned:#?}");
        };
        assert_eq!(filter.controller, None);
    }
}

#[test]
fn cleave_removes_negative_cast_origin_from_wash_away_target() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Wash Away")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Cleave {1}{U}{U}\n\
             Counter target spell [that wasn't cast from its owner's hand].",
        )
        .expect("Wash Away should compile");

    let normal = definition
        .spell_effect
        .as_ref()
        .expect("normal spell program")
        .flattened_default_effects();
    let normal_counter = normal
        .iter()
        .find_map(|effect| unwrap_tagged(effect).downcast_ref::<CounterEffect>())
        .expect("normal Wash Away should counter a spell");
    let ChooseSpec::Object(normal_filter) = normal_counter.target.base() else {
        panic!("normal Wash Away should carry an object target: {normal_counter:#?}");
    };
    assert_eq!(
        normal_filter.excluded_cast_origin_zone,
        Some(Zone::Hand)
    );

    let cleave_effects = definition
        .alternative_casts
        .iter()
        .find_map(|method| match method {
            AlternativeCastingMethod::Cleave { effects, .. } => Some(effects),
            _ => None,
        })
        .expect("Wash Away should have a typed Cleave method");
    let cleaved_counter = cleave_effects
        .iter()
        .find_map(|effect| unwrap_tagged(effect).downcast_ref::<CounterEffect>())
        .expect("cleaved Wash Away should counter a spell");
    let ChooseSpec::Object(cleaved_filter) = cleaved_counter.target.base() else {
        panic!("cleaved Wash Away should carry an object target: {cleaved_counter:#?}");
    };
    assert_eq!(cleaved_filter.excluded_cast_origin_zone, None);
}

#[test]
fn cleave_removes_mana_value_limit_from_path_of_peril() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Path of Peril")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Cleave {4}{W}{B}\n\
             Destroy all creatures [with mana value 2 or less].",
        )
        .expect("Path of Peril should compile");

    let normal = definition
        .spell_effect
        .as_ref()
        .expect("normal spell program")
        .flattened_default_effects();
    let normal_destroy = normal
        .iter()
        .find_map(|effect| unwrap_tagged(effect).downcast_ref::<DestroyEffect>())
        .expect("normal Path of Peril should destroy creatures");
    let ChooseSpec::All(normal_filter) = normal_destroy.spec.base() else {
        panic!("normal Path of Peril should carry an all-creatures filter: {normal_destroy:#?}");
    };
    assert_eq!(
        normal_filter.mana_value,
        Some(Comparison::LessThanOrEqual(2))
    );

    let cleave_effects = definition
        .alternative_casts
        .iter()
        .find_map(|method| match method {
            AlternativeCastingMethod::Cleave { effects, .. } => Some(effects),
            _ => None,
        })
        .expect("Path of Peril should have a typed Cleave method");
    let cleaved_destroy = cleave_effects
        .iter()
        .find_map(|effect| unwrap_tagged(effect).downcast_ref::<DestroyEffect>())
        .expect("cleaved Path of Peril should destroy creatures");
    let ChooseSpec::All(cleaved_filter) = cleaved_destroy.spec.base() else {
        panic!("cleaved Path of Peril should carry an all-creatures filter: {cleaved_destroy:#?}");
    };
    assert_eq!(cleaved_filter.mana_value, None);
}
