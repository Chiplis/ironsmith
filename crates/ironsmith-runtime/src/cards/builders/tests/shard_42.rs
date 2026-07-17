use super::shard_16::{assert_oracle_card_parses_strict, parse_oracle_card_definition};
use super::*;

#[derive(Default)]
struct ChooseLastReplacementDecisionMaker;

impl crate::decision::DecisionMaker for ChooseLastReplacementDecisionMaker {
    fn decide_options(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        ctx.options
            .iter()
            .rev()
            .find(|option| option.legal)
            .map(|option| vec![option.index])
            .unwrap_or_default()
    }
}

#[test]
pub(super) fn cursed_mirror_temporary_copy_reverts_to_its_underlying_artifact() {
    assert_oracle_card_parses_strict("Cursed Mirror");
    let mirror = parse_oracle_card_definition("Cursed Mirror");
    let spec = mirror
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(ability) => ability.enter_as_copy_as_enters(),
            _ => None,
        })
        .expect("Cursed Mirror must compile to an as-enters copy replacement");
    assert_eq!(spec.copy_duration, Some(crate::effect::Until::EndOfTurn));
    assert!(spec.added_abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(ability)
                if ability.id() == crate::static_abilities::StaticAbilityId::Haste
        )
    }));

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = CardDefinitionBuilder::new(CardId::new(), "Mirror Source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    game.create_object_from_definition(&source, alice, Zone::Battlefield);
    let mirror_id = game.create_object_from_definition(&mirror, alice, Zone::Hand);
    let mut decisions = ChooseLastReplacementDecisionMaker;
    let entered = game
        .move_object_with_etb_processing_with_dm(mirror_id, Zone::Battlefield, &mut decisions)
        .expect("Cursed Mirror should enter");

    assert_eq!(
        game.current_name(entered.new_id).as_deref(),
        Some("Mirror Source")
    );
    assert!(game.current_has_card_type(entered.new_id, CardType::Creature));
    assert!(game.object_has_static_ability_id(
        entered.new_id,
        crate::static_abilities::StaticAbilityId::Haste,
    ));

    game.effect_store.continuous_effects.cleanup_end_of_turn();
    game.refresh_continuous_state();
    assert_eq!(
        game.current_name(entered.new_id).as_deref(),
        Some("Cursed Mirror")
    );
    assert!(game.current_has_card_type(entered.new_id, CardType::Artifact));
    assert!(!game.current_has_card_type(entered.new_id, CardType::Creature));
    assert!(!game.object_has_static_ability_id(
        entered.new_id,
        crate::static_abilities::StaticAbilityId::Haste,
    ));

    assert!(compiled_text_lines(&mirror).iter().any(|line| {
        line == "As this artifact enters, you may have it become a copy of any creature on the battlefield until end of turn, except it has haste."
    }));
}

#[test]
pub(super) fn love_on_the_battlefield_links_both_attackers_for_this_combat() {
    assert_oracle_card_parses_strict("Love on the Battlefield");
    let definition = parse_oracle_card_definition("Love on the Battlefield");
    let triggered = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        triggered.len(),
        1,
        "the linked clause is one triggered ability"
    );
    let flat = triggered[0].effects.flattened_default_effects();
    let capture = flat
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::TagMatchingObjectsEffect>())
        .expect("the attack trigger must capture its exact attacking group");
    assert_eq!(capture.tag.as_str(), ironsmith_core::ATTACKING_GROUP_TAG);
    let schedule = flat
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>())
        .expect("the combat-damage reward must be a delayed trigger");
    assert_eq!(
        schedule.target_tag.as_ref().map(TagKey::as_str),
        Some(ironsmith_core::ATTACKING_GROUP_TAG),
    );
    assert!(schedule.until_end_of_combat);
    assert!(
        !schedule.one_shot,
        "each linked creature may deal damage more than once"
    );

    let expected = "Whenever you attack with exactly two creatures, those creatures gain first strike until end of turn, then draw a card. Whenever either of those creatures deals combat damage to a player this combat, put a +1/+1 counter on it.";
    assert!(
        compiled_text_lines(&definition)
            .iter()
            .any(|line| line == expected),
        "linked combat surface must remain intact: {:#?}",
        compiled_text_lines(&definition),
    );
}

#[test]
pub(super) fn love_on_the_battlefield_delayed_watchers_expire_at_end_of_combat() {
    let definition = parse_oracle_card_definition("Love on the Battlefield");
    let schedule = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .effects
                .flattened_default_effects()
                .iter()
                .find_map(|effect| {
                    effect
                        .downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
                        .cloned()
                }),
            _ => None,
        })
        .expect("Love must contain its combat-scoped delayed trigger");

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let creature = |name: &str| {
        CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    };
    let first =
        game.create_object_from_definition(&creature("First Attacker"), alice, Zone::Battlefield);
    let second =
        game.create_object_from_definition(&creature("Second Attacker"), alice, Zone::Battlefield);
    let snapshots = [first, second]
        .into_iter()
        .map(|id| crate::snapshot::ObjectSnapshot::from_object(game.object(id).unwrap(), &game))
        .collect();
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    ctx.set_tagged_objects(ironsmith_core::ATTACKING_GROUP_TAG, snapshots);
    schedule
        .execute(&mut game, &mut ctx)
        .expect("schedule Love's linked damage watchers");
    assert_eq!(game.effect_store.delayed_triggers.len(), 2);
    assert!(
        game.effect_store
            .delayed_triggers
            .iter()
            .all(|delayed| delayed.expires_at_end_of_combat)
    );

    let event = crate::triggers::TriggerEvent::new(
        crate::events::EndOfCombatEvent::new(),
        crate::provenance::ProvNodeId::default(),
    );
    assert!(crate::triggers::check_delayed_triggers(&mut game, &event).is_empty());
    assert!(game.effect_store.delayed_triggers.is_empty());
}

#[test]
pub(super) fn adjacent_counter_grants_do_not_invent_an_authored_conjunction() {
    let cases = [
        (
            "Ajani, Valiant Protector",
            "where X is your life total, then it gains trample until end of turn",
        ),
        (
            "Estinien Varlineau",
            "put a +1/+1 counter on Estinien Varlineau, then it gains flying until end of turn",
        ),
        (
            "Tales of Master Seshiro",
            "Put a +1/+1 counter on target creature or Vehicle you control, then it gains vigilance until end of turn",
        ),
        (
            "Baylen, the Haymaker",
            "Put three +1/+1 counters on Baylen, then it gains trample until end of turn",
        ),
        (
            "Silvar, Devourer of the Free",
            "Put a +1/+1 counter on Silvar, then it gains indestructible until end of turn",
        ),
    ];

    for (name, expected) in cases {
        assert_oracle_card_parses_strict(name);
        let definition = parse_oracle_card_definition(name);
        let compiled = compiled_text_lines(&definition).join("\n");
        assert!(
            compiled.contains(expected),
            "{name} must keep sequential counter/grant rendering: {compiled}"
        );
    }
}

#[test]
pub(super) fn authored_counter_grant_conjunctions_survive_parser_and_renderer() {
    let cases = [
        (
            "First Day of Class",
            "put a +1/+1 counter on it and it gains haste until end of turn",
        ),
        (
            "Skyrider Patrol",
            "put a +1/+1 counter on another target creature you control and it gains flying until end of turn",
        ),
    ];

    for (name, expected) in cases {
        assert_oracle_card_parses_strict(name);
        let definition = parse_oracle_card_definition(name);
        let compiled = compiled_text_lines(&definition).join("\n");
        assert!(
            compiled.contains(expected),
            "{name} must keep its authored counter/grant conjunction: {compiled}"
        );
    }
}

fn exact_singleton_look_effects(definition: &CardDefinition) -> Vec<&crate::effect::Effect> {
    definition
        .abilities
        .iter()
        .find_map(|ability| {
            let effects = match &ability.kind {
                AbilityKind::Triggered(triggered) => triggered.effects.flattened_default_effects(),
                AbilityKind::Activated(activated) => activated.effects.flattened_default_effects(),
                _ => return None,
            };
            effects
                .iter()
                .any(|effect| {
                    effect
                        .downcast_ref::<crate::effects::LookAtTopCardsEffect>()
                        .is_some()
                })
                .then(|| effects.iter().collect())
        })
        .expect("card should contain its look-two ability")
}

fn looked_owner_matches_filter_owner(looked: &PlayerFilter, owner: Option<&PlayerFilter>) -> bool {
    matches!(
        (looked, owner),
        (PlayerFilter::You, Some(PlayerFilter::You))
            | (
                PlayerFilter::Target(_),
                Some(PlayerFilter::AliasedTarget(_))
            )
    )
}

#[test]
pub(super) fn look_two_put_one_graveyard_cards_share_an_exact_selected_tag() {
    for (name, expected_surface) in [
        (
            "Celestus Sanctifier",
            "Look at the top two cards of your library. Put one of them into your graveyard",
        ),
        (
            "Soulcipher Board",
            "Look at the top two cards of your library. Put one of them into your graveyard",
        ),
        (
            "Wu Spy",
            "Look at the top two cards of target player's library. Put one of them into their graveyard",
        ),
    ] {
        assert_oracle_card_parses_strict(name);
        let definition = parse_oracle_card_definition(name);
        let effects = exact_singleton_look_effects(&definition);
        let look = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<crate::effects::LookAtTopCardsEffect>())
            .expect("look-two effect should be structured");
        let choose = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<ChooseObjectsEffect>())
            .expect("looked pool should have a structured exact selection");
        let move_to_zone = effects
            .iter()
            .find_map(|effect| {
                effect.downcast_ref::<MoveToZoneEffect>().or_else(|| {
                    effect
                        .downcast_ref::<TaggedEffect>()
                        .and_then(|tagged| tagged.effect.downcast_ref::<MoveToZoneEffect>())
                })
            })
            .expect("selected card should have a structured graveyard move");

        assert_eq!(look.count, crate::effect::Value::Fixed(2), "{name}");
        assert!(!look.reveal, "{name}");
        assert_eq!(
            choose.count,
            crate::effect::ChoiceCount::exactly(1),
            "{name}"
        );
        assert_eq!(choose.chooser, PlayerFilter::You, "{name}");
        assert_eq!(choose.zone, Some(Zone::Library), "{name}");
        assert!(
            looked_owner_matches_filter_owner(&look.player, choose.filter.owner.as_ref()),
            "{name} must select from the same player's looked library: {choose:#?}"
        );
        assert!(choose.filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == look.tag
                && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        }));
        assert_eq!(move_to_zone.zone, Zone::Graveyard, "{name}");
        assert!(
            matches!(
                move_to_zone.target.base(),
                ChooseSpec::Tagged(tag) if tag == &choose.tag
            ),
            "{name} must move only the exact selected tag: {move_to_zone:#?}"
        );
        let rendered_lines = compiled_text_lines(&definition);
        let expected_surface = expected_surface.to_ascii_lowercase();
        assert!(
            rendered_lines
                .iter()
                .any(|line| line.to_ascii_lowercase().contains(&expected_surface)),
            "{name} should retain the exact singleton surface: {:#?}",
            rendered_lines
        );
    }
}
