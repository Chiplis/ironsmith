//! UNVALIDATED implementation-first coverage: Craft materials described by a
//! general object filter ("four or more nonlands with activated abilities")
//! or a whole-selection relation ("two that share a card type"), CR 702.167a.
use ironsmith::ability::AbilityKind;
use ironsmith::effects::{ChooseObjectsEffect, EmitKeywordActionEffect};
use ironsmith::events::KeywordActionKind;
use ironsmith::{CardType, Zone};

#[path = "p09_common/mod.rs"]
mod common;

fn craft_choice(definition: &ironsmith::cards::CardDefinition) -> ChooseObjectsEffect {
    definition.abilities.iter().find_map(|ability| match &ability.kind {
        AbilityKind::Activated(activated) if activated.mana_cost.costs().iter().any(|cost|
            cost.effect_ref().and_then(|e| e.downcast_ref::<EmitKeywordActionEffect>())
                .is_some_and(|emit| emit.action == KeywordActionKind::Craft)) =>
            activated.mana_cost.costs().iter().find_map(|cost|
                cost.effect_ref().and_then(|e| e.downcast_ref::<ChooseObjectsEffect>()).cloned()),
        _ => None,
    }).expect("craft material choice")
}

#[test]
fn filtered_and_relation_materials_compile() {
    let rows = common::rows(include_str!("../../../fixtures/craft_filtered_materials.json.fixture"));
    assert_eq!(rows.len(), 2);
    for row in &rows {
        let name = row["name"].as_str().unwrap();
        for definition in common::definitions(row) {
            let choice = craft_choice(&definition);
            let battlefield = choice.filter.any_of.iter().find(|b| b.zone == Some(Zone::Battlefield)).unwrap();
            match name {
                "The Enigma Jewel" => {
                    assert_eq!(choice.count.min, 4);
                    assert_eq!(choice.count.max, None);
                    assert!(battlefield.has_activated_ability, "{name}");
                    assert!(battlefield.excluded_card_types.contains(&CardType::Land), "{name}");
                }
                "Eye of Ojer Taq" => {
                    assert_eq!(choice.count.min, 2);
                    assert_eq!(choice.count.max, Some(2));
                    assert!(choice.filter.shares_card_type, "the relation constrains the whole selection");
                }
                other => panic!("unexpected cohort member {other}"),
            }
        }
    }
}
