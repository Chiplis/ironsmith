#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "{2}, {T}: Tap all other artifacts. They don't untap during their controllers' untap steps for as long as this artifact remains tapped.";

fn artifact_definition(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Artifact])
        .build()
}

fn creature_definition(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

fn activated_program(definition: &CardDefinition) -> &crate::resolution::ResolutionProgram {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(&activated.effects),
            _ => None,
        })
        .expect("Kill Switch must retain its activated ability")
}

#[test]
fn kill_switch_keeps_the_exact_plural_locked_set_surface() {
    let definition = parse_oracle_card_definition("Kill Switch");
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![ORACLE.to_string()]
    );

    let debug = format!("{:#?}", activated_program(&definition));
    assert!(debug.contains("TapEffect"), "{debug}");
    assert!(debug.contains("DoesntUntap"), "{debug}");
    assert!(debug.contains("SourceUntaps"), "{debug}");
    assert!(debug.contains("SourceIsTapped"), "{debug}");
    assert!(debug.contains("lock_filter_at_resolution: true"), "{debug}");
}

#[test]
fn kill_switch_locks_only_the_resolution_set_until_the_source_untaps() {
    let definition = parse_oracle_card_definition("Kill Switch");
    let relic = artifact_definition("Locked Relic");
    let late_relic = artifact_definition("Late Relic");
    let creature = creature_definition("Unaffected Creature");
    let mut game = crate::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let locked = game.create_object_from_definition(&relic, bob, Zone::Battlefield);
    let unaffected = game.create_object_from_definition(&creature, bob, Zone::Battlefield);

    // The activation's tap cost has already been paid when its program begins.
    game.tap(source);
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        activated_program(&definition),
        None,
        &[],
    )
    .expect("Kill Switch resolution program should execute");
    assert!(game.is_tapped(locked));
    assert!(!game.is_tapped(unaffected));

    let lock = game
        .effect_store
        .continuous_effects
        .effects()
        .iter()
        .find(|effect| {
            effect.modification == crate::continuous::Modification::DoesntUntap
                && effect.source == source
        })
        .expect("resolution must register the doesn't-untap continuous effect");
    let crate::continuous::EffectSourceType::Resolution { locked_targets } = &lock.source_type
    else {
        panic!("the artifact set must be locked at resolution: {lock:#?}");
    };
    assert!(locked_targets.contains(&locked), "{lock:#?}");
    assert!(!locked_targets.contains(&unaffected), "{lock:#?}");

    let late = game.create_object_from_definition(&late_relic, bob, Zone::Battlefield);
    game.tap(late);
    game.refresh_continuous_state();
    assert!(game.current_has_static_ability_id(
        locked,
        crate::static_abilities::StaticAbilityId::DoesntUntap,
    ));
    assert!(
        !game.current_has_static_ability_id(
            late,
            crate::static_abilities::StaticAbilityId::DoesntUntap,
        ),
        "an artifact entering after resolution must not join the locked set"
    );

    game.turn.active_player = bob;
    crate::turn::execute_untap_step(&mut game);
    assert!(game.is_tapped(locked));
    assert!(!game.is_tapped(late));

    game.untap(source);
    game.refresh_continuous_state();
    assert!(!game.current_has_static_ability_id(
        locked,
        crate::static_abilities::StaticAbilityId::DoesntUntap,
    ));
    crate::turn::execute_untap_step(&mut game);
    assert!(
        !game.is_tapped(locked),
        "the original set must untap normally after the source untaps"
    );
}
