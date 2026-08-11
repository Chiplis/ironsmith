#![cfg(ironsmith_runtime_parser_tests)]

use super::*;
use crate::mana::{ManaCost, ManaSymbol};

const FLAMES_OF_REBIRTH: &str = "Return any number of target creature cards with total mana value 6 or less from your graveyard to the battlefield.";

fn creature_with_mana_value(id: u32, name: &str, mana_value: u8) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(id), name)
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
            mana_value,
        )]]))
        .build()
}

#[test]
fn phoenix_target_announcement_enforces_the_total_mana_value_budget() {
    let definition = CardDefinitionBuilder::new(CardId::from_raw(97_200), "Flames of Rebirth")
        .card_types(vec![CardType::Sorcery])
        .parse_text(FLAMES_OF_REBIRTH)
        .expect("Flames of Rebirth should compile");
    assert_eq!(
        canonical_compiled_lines(&definition).join(" "),
        FLAMES_OF_REBIRTH
    );

    let alice = PlayerId::from_index(0);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let four = game.create_object_from_definition(
        &creature_with_mana_value(97_201, "Four-Drop", 4),
        alice,
        Zone::Graveyard,
    );
    let three = game.create_object_from_definition(
        &creature_with_mana_value(97_202, "Three-Drop", 3),
        alice,
        Zone::Graveyard,
    );
    let two = game.create_object_from_definition(
        &creature_with_mana_value(97_203, "Two-Drop", 2),
        alice,
        Zone::Graveyard,
    );

    let program = definition
        .spell_effect
        .as_ref()
        .expect("the return instruction should have a spell program");
    let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
        &game,
        program,
        alice,
        Some(source),
        None,
    );
    let [requirement] = requirements.as_slice() else {
        panic!("expected one target requirement: {requirements:#?}");
    };
    assert_eq!(requirement.min_targets, 0);
    assert_eq!(requirement.max_targets, None);
    assert_eq!(requirement.legal_targets.len(), 3);
    assert!(
        [four, three, two].into_iter().all(|id| requirement
            .legal_targets
            .contains(&crate::game_state::Target::Object(id))),
        "all three cards are individually legal candidates: {requirement:#?}"
    );
    let aggregate = requirement
        .aggregate_constraint
        .as_ref()
        .expect("the announcement must carry the total mana-value budget");
    assert_eq!(aggregate.maximum, 6);
    assert_eq!(
        aggregate.value_for(crate::game_state::Target::Object(four)),
        4
    );
    assert_eq!(
        aggregate.value_for(crate::game_state::Target::Object(three)),
        3
    );
    assert_eq!(
        aggregate.value_for(crate::game_state::Target::Object(two)),
        2
    );

    let contexts = vec![crate::decisions::context::TargetRequirementContext {
        description: requirement.description.clone(),
        legal_targets: requirement.legal_targets.clone(),
        legal_target_sets: requirement.legal_target_sets.clone(),
        aggregate_constraint: requirement.aggregate_constraint.clone(),
        min_targets: requirement.min_targets,
        max_targets: requirement.max_targets,
        distinct_player_group: requirement.distinct_player_group,
    }];
    assert!(
        !crate::targeting::validate_flat_target_assignment(
            &contexts,
            &[
                crate::game_state::Target::Object(four),
                crate::game_state::Target::Object(three),
            ],
        ),
        "4 + 3 must be rejected even though each card individually costs at most 6"
    );
    assert!(crate::targeting::validate_flat_target_assignment(
        &contexts,
        &[
            crate::game_state::Target::Object(four),
            crate::game_state::Target::Object(two),
        ],
    ));
}
