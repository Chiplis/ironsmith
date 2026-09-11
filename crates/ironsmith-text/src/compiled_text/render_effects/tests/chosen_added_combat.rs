use super::*;
use crate::CardDefinitionBuilder;
use crate::card::PowerToughness;
use crate::ids::{CardId, PlayerId};
use crate::static_abilities::StaticAbilityId;
const TEXT: &str = "Choose two target creatures. Untap them. Put two +1/+1 counters on each of them. They gain vigilance, indestructible, and haste until end of turn. After this main phase, there is an additional combat phase. Only the chosen creatures can attack during that combat phase.";

#[test]
fn chosen_attack_restriction_follows_its_added_combat() {
    for initially_tapped in [false, true] {
        for insert_extra in [false, true] {
            for skip_own_combat in [false, true] {
                check_chosen_combat(initially_tapped, insert_extra, skip_own_combat);
            }
        }
    }
}

fn check_chosen_combat(initially_tapped: bool, insert_extra: bool, skip_own_combat: bool) {
    let def = CardDefinitionBuilder::new(CardId::new(), "Last Night Together")
        .card_types(vec![CardType::Sorcery])
        .parse_text(TEXT)
        .unwrap();
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;
    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;

    let source = game.create_object_from_definition(&def, alice, Zone::Stack);
    let chosen_one = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(71_001), "Chosen One")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        alice,
        Zone::Battlefield,
    );
    let chosen_two = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(71_002), "Chosen Two")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        alice,
        Zone::Battlefield,
    );
    let unchosen = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(71_003), "Unchosen Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        alice,
        Zone::Battlefield,
    );
    for creature in [chosen_one, chosen_two, unchosen] {
        game.remove_summoning_sickness(creature);
        if initially_tapped {
            game.tap(creature);
        }
    }

    let effects = def
        .spell_effect
        .as_ref()
        .expect("Last Night Together should have a spell effect")
        .flattened_default_effects();
    let target_spec = effects[0]
        .0
        .get_target_spec()
        .expect("Last Night Together should start with target selection")
        .clone();
    let mut ctx = crate::effects::EffectContext::new_default(source, alice)
        .with_targets(vec![
            crate::effects::ResolvedTarget::Object(chosen_one),
            crate::effects::ResolvedTarget::Object(chosen_two),
        ])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: target_spec,
            range: 0..2,
        }]);
    ctx.snapshot_targets(&game);

    for effect in effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap_or_else(|err| {
            panic!("Last Night Together effect should resolve: {err:?}; effect={effect:?}")
        });
    }

    assert_eq!(
        game.turn_store.additional_phases,
        vec![crate::game_state::Phase::Combat],
        "Last Night Together should insert an additional combat phase"
    );
    for chosen in [chosen_one, chosen_two] {
        assert!(
            !game.is_tapped(chosen),
            "chosen creatures should be untapped"
        );
        assert_eq!(
            game.counter_count(chosen, crate::object::CounterType::PlusOnePlusOne),
            2,
            "chosen creatures should get two +1/+1 counters"
        );
        assert!(game.object_has_static_ability_id(chosen, StaticAbilityId::Vigilance));
        assert!(game.object_has_static_ability_id(chosen, StaticAbilityId::Indestructible));
        assert!(game.object_has_static_ability_id(chosen, StaticAbilityId::Haste));
        assert!(
            game.can_attack(chosen),
            "chosen creatures should be allowed to attack"
        );
    }
    assert!(
        game.can_attack(unchosen),
        "the restriction must wait for its added combat"
    );

    // A later-created extra combat happens first and is not the combat
    // whose attackers this spell restricted.
    if insert_extra {
        let extra = crate::effects::AdditionalPhasesEffect {
            phases: vec![crate::effects::AdditionalPhase::Combat],
            after_main_phase: false,
        };
        crate::effects::execute_effect(&mut game, &crate::effect::Effect::new(extra), &mut ctx)
            .unwrap();
        crate::turn::advance_phase(&mut game).unwrap();
        assert!(
            game.can_attack(unchosen),
            "an unrelated extra combat must allow unchosen attackers"
        );
        game.turn.step = None;
    }
    if skip_own_combat {
        game.turn_store.skip_next_combat_phases.insert(alice);
        crate::turn::advance_phase(&mut game).unwrap();
        assert!(
            game.can_attack(unchosen),
            "a skipped restricted combat must not restrict the next combat"
        );
        assert!(game.effect_store.restriction_effects.is_empty());
        return;
    }
    crate::turn::advance_phase(&mut game).expect("advance to the inserted combat phase");
    assert_eq!(game.turn.phase, crate::game_state::Phase::Combat);
    assert!(!game.can_attack(unchosen));

    let late_unchosen = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(71_004), "Late Unchosen Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        alice,
        Zone::Battlefield,
    );
    game.remove_summoning_sickness(late_unchosen);
    game.update_cant_effects();
    assert!(
        !game.can_attack(late_unchosen),
        "creatures that enter before that combat are still not chosen and cannot attack"
    );

    game.turn.step = None;
    crate::turn::advance_phase(&mut game).expect("advance out of the restricted combat phase");
    assert!(
        game.can_attack(unchosen),
        "the chosen-creatures-only restriction should expire after that combat phase"
    );
    assert!(
        game.can_attack(late_unchosen),
        "late unchosen creatures should be able to attack after the restricted combat ends"
    );
}

#[test]
fn chosen_added_combat_preserves_compiled_text() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Last Night Together")
        .card_types(vec![CardType::Sorcery])
        .parse_text(TEXT)
        .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join(" "),
        TEXT
    );
}

#[test]
fn untapped_antecedent_condition_preserves_current_creature_type() {
    let text = "Untap target creature you control. It gets +2/+2 until end of turn. If it's a Dwarf, you may attach an Equipment you control to it.";
    let definition = CardDefinitionBuilder::new(CardId::new(), "Vow to Erebor")
        .card_types(vec![CardType::Instant])
        .parse_text(text)
        .unwrap();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join(" "),
        text
    );
}
