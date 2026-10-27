//! Behavior and keyword identity are intentionally separate assertions.
use super::*;
use ironsmith::ability::{Ability, AbilityKind};
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
use ironsmith::{GameState, PlayerId, Zone};

fn definition(name: &str, rules: &str, keyword: Option<&str>) -> ironsmith::cards::CardDefinition {
    let mut builder = compiler::CardDefinitionBuilder::new(compiler::ids::CardId::new(), name);
    if let Some(keyword) = keyword {
        builder = builder.with_custom_keyword(keyword);
    }
    compile_builder_to_runtime_definition(
        builder,
        format!("Type: Creature\nPower/Toughness: 2/2\n{rules}"),
        false,
    )
    .unwrap()
}

/// Execute the ordinary entry pipeline, including replacement programs.
fn enter_reminder_permanent(game: &mut GameState, id: ironsmith::ObjectId) -> ironsmith::ObjectId {
    let object = game.object(id).unwrap();
    let stable = object.stable_id;
    let controller = game.controller_of(object);
    let effect = ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(
        ironsmith::target::ChooseSpec::SpecificObject(id), Zone::Battlefield, false));
    let mut context = ironsmith::effects::EffectContext::new_default(id, controller);
    let outcome = ironsmith::effects::execute_effect(game, &effect, &mut context).unwrap();
    for event in outcome.events { game.queue_trigger_event(Default::default(), event); }
    game.find_object_by_stable_id(stable).unwrap()
}

fn drain_reminder_triggers(game: &mut GameState) -> Vec<ironsmith::triggers::TriggeredAbilityEntry> {
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events_with_dm(game, &mut queue, &mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();
    queue.take_all()
}

#[test]
fn reminder_identity_defender_is_not_an_unconditional_attack_restriction() {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let mut named = definition("Named", "Defender", None);
    let mut generic = definition("Generic", "This creature can't attack.", Some("Sentinel"));
    let permission = Ability::static_ability(StaticAbility::can_attack_as_though_no_defender());
    named.abilities.push(permission.clone());
    generic.abilities.push(permission);
    let a = game.create_object_from_definition(&named, PlayerId::from_index(0), Zone::Battlefield);
    let b =
        game.create_object_from_definition(&generic, PlayerId::from_index(0), Zone::Battlefield);
    game.remove_summoning_sickness(a);
    game.remove_summoning_sickness(b);
    assert!(ironsmith::rules::combat::can_attack(
        game.object(a).unwrap(),
        &game
    ));
    assert!(!ironsmith::rules::combat::can_attack(
        game.object(b).unwrap(),
        &game
    ));
    assert!(
        game.object(a)
            .unwrap()
            .has_static_ability_id(StaticAbilityId::Defender)
    );
    assert!(
        !game
            .object(b)
            .unwrap()
            .has_static_ability_id(StaticAbilityId::Defender)
    );
}

#[test]
fn reminder_identity_custom_evasion_keeps_only_its_own_name() {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let custom = definition(
        "Custom",
        "This creature can't be blocked except by creatures with flying or reach.",
        Some("Soar"),
    );
    let ground = definition("Ground", "", None);
    let flyer = definition("Flyer", "Flying", None);
    let a = game.create_object_from_definition(&custom, PlayerId::from_index(0), Zone::Battlefield);
    let b = game.create_object_from_definition(&ground, PlayerId::from_index(1), Zone::Battlefield);
    let c = game.create_object_from_definition(&flyer, PlayerId::from_index(1), Zone::Battlefield);
    assert!(!ironsmith::rules::combat::can_block(
        game.object(a).unwrap(),
        game.object(b).unwrap(),
        &game
    ));
    assert!(ironsmith::rules::combat::can_block(
        game.object(a).unwrap(),
        game.object(c).unwrap(),
        &game
    ));
    assert!(
        !game
            .object(a)
            .unwrap()
            .has_static_ability_id(StaticAbilityId::Flying)
    );
    let marker = custom
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(ability) if ability.custom_keyword_name().is_some() => {
                Some(ability)
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(marker.custom_keyword_name(), Some("Soar"));
    assert_ne!(marker, &StaticAbility::flying());
    let other = StaticAbility::from_model(
        ironsmith::static_abilities::CompiledStaticAbility::custom_keyword_identity("Glide"),
    );
    assert_ne!(marker, &other);
}

#[test]
fn reminder_identity_attacking_without_tapping_does_not_grant_vigilance() {
    for reminder in [
        "Attacking doesn't cause this creature to tap.",
        "Attacking doesn't cause it to tap.",
    ] {
        let card = definition("Custom", reminder, Some("Watchful"));
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let id =
            game.create_object_from_definition(&card, PlayerId::from_index(0), Zone::Battlefield);
        let object = game.object(id).unwrap();
        assert!(ironsmith::rules::combat::has_vigilance_with_game(
            object, &game
        ));
        assert!(!object.has_static_ability_id(StaticAbilityId::Vigilance));
        assert!(!game.object_has_static_ability_id(id, StaticAbilityId::Vigilance));
        use ironsmith::effects::{ApplyContinuousEffect, EffectContext, EffectExecutor};
        ApplyContinuousEffect::with_spec(
            ironsmith::target::ChooseSpec::SpecificObject(id),
            ironsmith::continuous::Modification::RemoveAllAbilities,
            ironsmith::effect::Until::EndOfTurn,
        )
        .execute(
            &mut game,
            &mut EffectContext::new_default(id, PlayerId::from_index(0)),
        )
        .unwrap();
        game.refresh_continuous_state().unwrap();
        assert!(!ironsmith::rules::combat::has_vigilance_with_game(
            game.object(id).unwrap(),
            &game
        ));
    }
}

#[test]
fn reminder_identity_attachment_reuses_equip_behavior_without_equip_identity() {
    let compile = |rules: &str| {
        compiler::parse_card_text(
            compiler::CardDefinitionBuilder::new(compiler::ids::CardId::new(), "Attachment"),
            format!("Type: Artifact — Equipment\n{rules}"),
        )
        .unwrap()
    };
    let named = compile("Equip {3}");
    for rules in [
        "{3}: Attach to target creature you control. Equip only as a sorcery.",
        "{3}: Attach this Equipment to target creature you control. Activate only as a sorcery.",
    ] {
        let generic = compile(rules);
        let activated = |card: &compiler::cards::CardDefinition| {
            card.abilities
                .iter()
                .find_map(|a| {
                    if let compiler::ability::AbilityKind::Activated(a) = &a.kind {
                        Some(a.clone())
                    } else {
                        None
                    }
                })
                .unwrap()
        };
        let expected = activated(&named);
        let actual = activated(&generic);
        assert!(expected.keyword.is_some());
        assert!(actual.keyword.is_none());
        assert_eq!(actual.timing, expected.timing);
        assert_eq!(
            format!("{:?}", actual.mana_cost),
            format!("{:?}", expected.mana_cost)
        );
        let attachment = |ability: &compiler::ability::ActivatedAbility| {
            let effects = ability.effects.all_effects();
            assert_eq!(effects.len(), 1);
            effects[0]
                .downcast_ref::<compiler::effects::AttachObjectsEffect>()
                .unwrap()
                .clone()
        };
        let mut actual_effect = attachment(&actual);
        let expected_effect = attachment(&expected);
        // Surface hints record the printed nouns; retain all targeting rules.
        if let ironsmith_core::ChooseSpec::Target(target) = &mut actual_effect.target
            && let ironsmith_core::ChooseSpec::Object(filter) = target.as_mut()
        {
            filter.set_explicit_card_type_noun(None);
        }
        assert_eq!(actual_effect, expected_effect);
    }
}

#[test]
fn reminder_identity_optional_payment_does_not_mark_spell_kicked() {
    for cost in ["{2}", "{R}", "{2}{B}", "{X}"] {
        let compile = |rules: String| {
            compiler::parse_card_text(
                compiler::CardDefinitionBuilder::new(compiler::ids::CardId::new(), "Payment"),
                format!("Type: Sorcery\n{rules}\nDraw a card."),
            )
            .unwrap()
        };
        let named = compile(format!("Kicker {cost}"));
        let generic = compile(format!(
            "You may pay an additional {cost} as you cast this spell."
        ));
        assert_eq!(generic.optional_costs.len(), 1);
        let actual = &generic.optional_costs[0];
        let expected = &named.optional_costs[0];
        assert_eq!(format!("{:?}", actual.cost), format!("{:?}", expected.cost));
        assert_eq!(actual.repeatable, expected.repeatable);
        assert_eq!(actual.kind, ironsmith_core::OptionalCostKind::Additional);
        let mut paid = ironsmith_core::OptionalCostsPaid::default();
        paid.mark_label_paid(actual.reference.clone());
        assert!(!paid.was_kicked());
        paid.mark_label_paid(expected.reference.clone());
        assert!(paid.was_kicked());
    }
}

#[test]
fn reminder_identity_repeated_and_independent_payments_keep_separate_counts() {
    let card = compiler::parse_card_text(
        compiler::CardDefinitionBuilder::new(compiler::ids::CardId::new(), "Payments"),
        "Type: Sorcery\nYou may pay an additional {R} and/or {2}{U} any number of times as you cast this spell.\nDraw a card.",
    ).unwrap();
    assert_eq!(card.optional_costs.len(), 2);
    let [red, blue] = card.optional_costs.as_slice() else {
        unreachable!()
    };
    assert!(red.repeatable && blue.repeatable);
    assert_ne!(red.reference, blue.reference);
    let mut paid = ironsmith_core::OptionalCostsPaid::default();
    paid.mark_label_paid(red.reference.clone());
    paid.mark_label_paid(red.reference.clone());
    paid.mark_label_paid(blue.reference.clone());
    assert_eq!(paid.times_paid_label(&red.reference), 2);
    assert_eq!(paid.times_paid_label(&blue.reference), 1);
    assert!(!paid.was_kicked());
}

#[test]
fn reminder_identity_attack_and_activation_permissions_do_not_grant_haste() {
    for (rules, tap) in [
        (
            "This creature can attack and {T} as soon as it comes under your control.",
            true,
        ),
        (
            "It can attack and {T} no matter when it came under your control.",
            true,
        ),
        (
            "This creature can attack as soon as it comes under your control.",
            false,
        ),
    ] {
        let card = definition("Custom", rules, Some("Eager"));
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let id =
            game.create_object_from_definition(&card, PlayerId::from_index(0), Zone::Battlefield);
        game.set_summoning_sick(id);
        assert!(game.is_summoning_sick(id));
        assert!(ironsmith::rules::combat::can_attack(
            game.object(id).unwrap(),
            &game
        ));
        assert_eq!(game.activates_abilities_as_though_haste(id), tap);
        assert!(!game.object_has_static_ability_id(id, StaticAbilityId::Haste));
        let other = game.create_object_from_definition(
            &definition("Other", "", None),
            PlayerId::from_index(0),
            Zone::Battlefield,
        );
        game.set_summoning_sick(other);
        assert!(!game.activates_abilities_as_though_haste(other));
        assert!(!ironsmith::rules::combat::can_attack(
            game.object(other).unwrap(),
            &game
        ));
    }
}

#[test]
fn reminder_identity_optional_payment_is_charged_during_real_casting() {
    use ironsmith::decision::{
        DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
    };
    use ironsmith::decisions::context::SelectOptionsContext;
    use ironsmith::game_loop::{
        PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
        apply_priority_response_with_dm,
    };
    struct PayExtra(bool);
    impl DecisionMaker for PayExtra {
        fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            if ctx.description.starts_with("Choose optional costs") {
                return if self.0 {
                    vec![
                        ctx.options
                            .iter()
                            .find(|option| option.legal)
                            .unwrap()
                            .index,
                    ]
                } else {
                    vec![]
                };
            }
            SelectFirstDecisionMaker.decide_options(game, ctx)
        }
    }
    for named in [false, true] {
        for pay in [false, true] {
            let rules = if named {
                "Kicker {2}"
            } else {
                "You may pay an additional {2} as you cast this spell."
            };
            let card = compile_to_runtime_definition(
                "Payment",
                format!("Mana cost: {{1}}\nType: Sorcery\n{rules}\nDraw a card."),
                false,
            )
            .unwrap();
            let a = PlayerId::from_index(0);
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            game.turn.active_player = a;
            game.turn.priority_player = Some(a);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.turn.step = None;
            game.player_mut(a)
                .unwrap()
                .mana_pool
                .add(ironsmith::mana::ManaSymbol::Colorless, 3);
            let spell = game.create_object_from_definition(&card, a, Zone::Hand);
            let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action|
            matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)).unwrap();
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            let mut state = PriorityLoopState::new(2);
            let mut dm = PayExtra(pay);
            let mut progress = apply_priority_response_with_dm(
                &mut game,
                &mut queue,
                &mut state,
                &PriorityResponse::PriorityAction(action),
                &mut dm,
            )
            .unwrap();
            for _ in 0..32 {
                if !state.has_pending_action() {
                    break;
                }
                let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else {
                    panic!("{progress:?}")
                };
                progress = apply_decision_context_with_dm(
                    &mut game, &mut queue, &mut state, &ctx, &mut dm,
                )
                .unwrap();
            }
            assert!(!state.has_pending_action());
            assert_eq!(
                game.player(a).unwrap().mana_pool.total(),
                if pay { 0 } else { 2 }
            );
            assert_eq!(
                game.stack.last().unwrap().optional_costs_paid.was_kicked(),
                named && pay
            );
        }
    }
}

#[test]
fn reminder_identity_instant_timing_does_not_grant_flash_or_a_cast_origin() {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    let a = PlayerId::from_index(0);
    for rules in [
        "Flash",
        "You may cast this spell any time you could cast an instant.",
    ] {
        let card = definition(
            "Timing",
            &format!("Mana cost: {{1}}\n{rules}"),
            Some("Swift"),
        );
        for zone in [Zone::Hand, Zone::Graveyard] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            game.turn.active_player = PlayerId::from_index(1);
            game.turn.priority_player = Some(a);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.turn.step = None;
            game.player_mut(a)
                .unwrap()
                .mana_pool
                .add(ironsmith::mana::ManaSymbol::Colorless, 2);
            let id = game.create_object_from_definition(&card, a, zone);
            let ordinary = game.create_object_from_definition(
                &definition("Ordinary", "Mana cost: {1}", None),
                a,
                Zone::Hand,
            );
            let actions = compute_legal_actions(&game, a).unwrap();
            let can_cast = |id| {
                actions.iter().any(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == id))
            };
            assert_eq!(can_cast(id), zone == Zone::Hand, "{rules} in {zone:?}");
            assert!(!can_cast(ordinary));
            if rules != "Flash" {
                assert!(!game.object_has_static_ability_id(id, StaticAbilityId::Flash));
            }
        }
    }
}

#[test]
fn reminder_identity_colorless_definition_works_in_every_zone_without_devoid() {
    use ironsmith::color::ColorSet;
    for (rules, custom) in [("Devoid", false), ("This card has no color.", true)] {
        let card = definition(
            "Blank Adept",
            &format!("Mana cost: {{B}}\n{rules}"),
            custom.then_some("Blank"),
        );
        assert_eq!(card.card.color_identity(), ColorSet::BLACK);
        for zone in [
            Zone::Library,
            Zone::Hand,
            Zone::Stack,
            Zone::Battlefield,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Command,
        ] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let id = game.create_object_from_definition(&card, PlayerId::from_index(0), zone);
            game.refresh_continuous_state().unwrap();
            assert_eq!(
                game.current_colors(id),
                Some(ColorSet::COLORLESS),
                "{rules} in {zone:?}"
            );
            assert_eq!(game.object(id).unwrap().abilities.iter().any(|ability| {
                matches!(&ability.kind, AbilityKind::Static(static_ability) if static_ability.is_devoid())
            }), !custom);
        }
    }
}

#[test]
fn reminder_identity_early_combat_damage_matches_named_keywords() {
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
    for (named, prose, regular_damage, identity) in [
        (
            "First strike",
            "This creature deals combat damage before creatures without first strike.",
            0,
            StaticAbilityId::FirstStrike,
        ),
        (
            "Double strike",
            "This creature deals both first-strike and regular combat damage.",
            2,
            StaticAbilityId::DoubleStrike,
        ),
    ] {
        for (rules, custom) in [(named, None), (prose, Some("Quickstrike"))] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let def = definition("Striker", rules, custom);
            let attacker = game.create_object_from_definition(
                &def,
                PlayerId::from_index(0),
                Zone::Battlefield,
            );
            assert_eq!(
                game.object(attacker)
                    .unwrap()
                    .has_static_ability_id(identity),
                custom.is_none()
            );
            let mut combat = CombatState::default();
            combat.attackers.push(AttackerInfo {
                creature: attacker,
                target: AttackTarget::Player(PlayerId::from_index(1)),
            });
            combat.blockers.insert(attacker, Vec::new());
            for (early, expected) in [(true, 2), (false, regular_damage)] {
                let events =
                    ironsmith::game_loop::execute_combat_damage_step(&mut game, &combat, early);
                assert_eq!(
                    events.iter().map(|event| event.amount).sum::<u32>(),
                    expected,
                    "{rules}, early={early}"
                );
            }
        }
    }
}

#[test]
fn reminder_identity_blocking_permission_does_not_grant_reach() {
    for (rules, custom) in [
        ("Reach", None),
        (
            "This creature can block creatures with flying.",
            Some("Skyguard"),
        ),
    ] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let blocker = game.create_object_from_definition(
            &definition("Blocker", rules, custom),
            PlayerId::from_index(1),
            Zone::Battlefield,
        );
        assert_eq!(
            game.object(blocker)
                .unwrap()
                .has_static_ability_id(StaticAbilityId::Reach),
            custom.is_none()
        );
        for (rules, expected) in [("Flying", true), ("", true), ("Flying\nShadow", false)] {
            let attacker = game.create_object_from_definition(
                &definition("Attacker", rules, None),
                PlayerId::from_index(0),
                Zone::Battlefield,
            );
            assert_eq!(
                ironsmith::rules::combat::can_block(
                    game.object(attacker).unwrap(),
                    game.object(blocker).unwrap(),
                    &game
                ),
                expected,
                "{rules}"
            );
        }
    }
}

#[test]
fn reminder_identity_damage_consequences_match_keywords_without_granting_identity() {
    use ironsmith::effect::Effect;
    use ironsmith::effects::{EffectContext, execute_effect};
    use ironsmith::target::ChooseSpec;
    for (named, prose, identity, lethal, gain_life) in [
        (
            "Deathtouch",
            "Any amount of damage this deals to a creature is enough to destroy it.",
            StaticAbilityId::Deathtouch,
            true,
            false,
        ),
        (
            "Lifelink",
            "Damage dealt by this creature also causes you to gain that much life.",
            StaticAbilityId::Lifelink,
            false,
            true,
        ),
    ] {
        for (rules, custom) in [(named, None), (prose, Some("Custom damage rule"))] {
            for (amount, zone) in [0, 1].into_iter().flat_map(|amount| {
                [Zone::Battlefield, Zone::Stack, Zone::Graveyard, Zone::Hand]
                    .into_iter()
                    .map(move |zone| (amount, zone))
            }) {
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let alice = PlayerId::from_index(0);
                let bob = PlayerId::from_index(1);
                let source = game.create_object_from_definition(
                    &definition("Source", rules, custom),
                    alice,
                    zone,
                );
                let target = game.create_object_from_definition(
                    &definition("Recipient", "", None),
                    bob,
                    Zone::Battlefield,
                );
                assert_eq!(
                    game.object(source).unwrap().has_static_ability_id(identity),
                    custom.is_none()
                );
                let mut ctx = EffectContext::new_default(source, alice);
                execute_effect(
                    &mut game,
                    &Effect::deal_damage(amount, ChooseSpec::SpecificObject(target)),
                    &mut ctx,
                )
                .unwrap();
                assert_eq!(game.damage_on(target), amount as u32);
                assert_eq!(
                    game.has_deathtouch_damage_since_sba(target),
                    lethal && amount > 0
                );
                assert_eq!(
                    game.player(alice).unwrap().life,
                    20 + if gain_life { amount } else { 0 }
                );
                execute_effect(
                    &mut game,
                    &Effect::deal_damage(amount, ChooseSpec::SpecificPlayer(bob)),
                    &mut ctx,
                )
                .unwrap();
                assert_eq!(game.player(bob).unwrap().life, 20 - amount);
                assert_eq!(
                    game.player(alice).unwrap().life,
                    20 + if gain_life { 2 * amount } else { 0 }
                );
            }
        }
    }
}

#[test]
fn reminder_identity_damage_uses_current_source_or_departed_source_snapshot() {
    use ironsmith::effect::Effect;
    use ironsmith::effects::{EffectContext, execute_effect};
    use ironsmith::target::ChooseSpec;
    let rules = "Any amount of damage this deals to a creature is enough to destroy it.\nDamage dealt by this creature also causes you to gain that much life.";
    for departed in [false, true] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.create_object_from_definition(
            &definition("Source", rules, Some("Siphon venom")),
            alice,
            Zone::Battlefield,
        );
        let target = game.create_object_from_definition(
            &definition("Recipient", "", None),
            bob,
            Zone::Battlefield,
        );
        let snapshot =
            ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                game.object(source).unwrap(),
                &game,
            );
        if departed {
            game.remove_object(source);
        } else {
            game.object_mut(source).unwrap().abilities_mut().clear();
            game.refresh_continuous_state().unwrap();
        }
        let mut ctx = EffectContext::new_default(source, alice).with_source_snapshot(snapshot);
        execute_effect(
            &mut game,
            &Effect::deal_damage(1, ChooseSpec::SpecificObject(target)),
            &mut ctx,
        )
        .unwrap();
        assert_eq!(game.damage_on(target), 1);
        assert_eq!(game.has_deathtouch_damage_since_sba(target), departed);
        assert_eq!(
            game.player(alice).unwrap().life,
            if departed { 21 } else { 20 }
        );
    }
}

#[test]
fn reminder_identity_bound_graveyard_cost_pays_mana_and_exiles_other_cards() {
    use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
    use ironsmith::game_loop::{
        PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
        apply_priority_response_with_dm,
    };
    for named in [false, true] {
        let rules = if named {
            "Escape—{2}, Exile two other cards from your graveyard."
        } else {
            "You may cast this card from your graveyard for {2}, Exile two other cards from your graveyard."
        };
        let card = definition(
            "Returning Adept",
            &format!("Mana cost: {{6}}\n{rules}"),
            (!named).then_some("Recur"),
        );
        assert_eq!(card.alternative_casts.len(), 1);
        assert_eq!(card.alternative_casts[0].name() == "Escape", named);
        let a = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.turn.active_player = a;
        game.turn.priority_player = Some(a);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        game.turn.step = None;
        game.player_mut(a)
            .unwrap()
            .mana_pool
            .add(ironsmith::mana::ManaSymbol::Colorless, 2);
        let spell = game.create_object_from_definition(&card, a, Zone::Graveyard);
        assert!(!compute_legal_actions(&game, a).unwrap().iter().any(|action|
            matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)), "cannot pay with the source itself");
        let fodder = definition("Fuel", "", None);
        game.create_object_from_definition(&fodder, a, Zone::Graveyard);
        assert!(!compute_legal_actions(&game, a).unwrap().iter().any(|action|
            matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)),
            "one other card plus the source does not pay for two other cards");
        game.create_object_from_definition(&fodder, a, Zone::Graveyard);
        let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action|
            matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)).expect("graveyard permission and complete cost must be usable");
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut state = PriorityLoopState::new(2);
        let mut dm = SelectFirstDecisionMaker;
        let mut progress = apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        )
        .unwrap();
        for _ in 0..32 {
            if !state.has_pending_action() {
                break;
            }
            let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else {
                panic!("{progress:?}")
            };
            progress =
                apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm)
                    .unwrap();
        }
        assert!(!state.has_pending_action());
        assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
        assert_eq!(game.exile.len(), 2);
        assert!(
            game.exile
                .iter()
                .all(|id| game.object(*id).unwrap().name == "Fuel")
        );
        assert_eq!(game.stack.len(), 1);
        assert_eq!(
            game.stack
                .last()
                .unwrap()
                .optional_costs_paid
                .was_paid_label("Escape"),
            named
        );
    }
}

#[test]
fn reminder_identity_damage_consequences_work_in_both_combat_paths() {
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
    for named in [false, true] {
        for blocked in [false, true] {
            let rules = if named {
                "Deathtouch\nLifelink"
            } else {
                "Any amount of damage this deals to a creature is enough to destroy it.\nDamage dealt by this creature also causes you to gain that much life."
            };
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let attacker = game.create_object_from_definition(
                &definition("Source", rules, (!named).then_some("Siphon venom")),
                alice,
                Zone::Battlefield,
            );
            let blocker = game.create_object_from_definition(
                &definition("Recipient", "", None),
                bob,
                Zone::Battlefield,
            );
            let mut combat = CombatState::default();
            combat.attackers.push(AttackerInfo {
                creature: attacker,
                target: AttackTarget::Player(bob),
            });
            combat
                .blockers
                .insert(attacker, if blocked { vec![blocker] } else { vec![] });
            let events =
                ironsmith::game_loop::execute_combat_damage_step(&mut game, &combat, false);
            assert_eq!(
                events
                    .iter()
                    .filter(|event| event.source == attacker)
                    .map(|event| event.amount)
                    .sum::<u32>(),
                2
            );
            assert_eq!(game.player(alice).unwrap().life, 22);
            assert_eq!(
                game.player(bob).unwrap().life,
                if blocked { 20 } else { 18 }
            );
            assert_eq!(game.has_deathtouch_damage_since_sba(blocker), blocked);
        }
    }
}

#[test]
fn reminder_identity_bound_alternative_price_preserves_its_condition() {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    for (rules, custom) in [
        ("Spectacle {1}", None),
        (
            "You may cast this spell for {1} rather than its mana cost if an opponent lost life this turn.",
            Some("Blood price"),
        ),
    ] {
        let card = definition("Adept", &format!("Mana cost: {{6}}\n{rules}"), custom);
        assert_eq!(card.alternative_casts.len(), 1);
        assert_eq!(
            card.alternative_casts[0].name() == "Spectacle",
            custom.is_none()
        );
        let a = PlayerId::from_index(0);
        let b = PlayerId::from_index(1);
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.turn.active_player = a;
        game.turn.priority_player = Some(a);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        game.turn.step = None;
        game.player_mut(a)
            .unwrap()
            .mana_pool
            .add(ironsmith::mana::ManaSymbol::Colorless, 1);
        let spell = game.create_object_from_definition(&card, a, Zone::Hand);
        let castable = |game: &GameState| {
            compute_legal_actions(game, a).unwrap().iter().any(|action|
            matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
        };
        assert!(!castable(&game));
        let mut ctx = ironsmith::effects::EffectContext::new_default(spell, a);
        ironsmith::effects::execute_effect(
            &mut game,
            &ironsmith::effect::Effect::lose_life_player(1, ironsmith::PlayerFilter::Specific(b)),
            &mut ctx,
        )
        .unwrap();
        assert!(castable(&game));
    }
}

#[test]
fn reminder_identity_price_reader_preserves_other_complete_payment_readings() {
    let card = definition("Price Carrier", "You may cast this spell for {4}{G}{G} if you also return a tapped creature you control to its owner's hand.", None);
    assert!(!card.alternative_casts.is_empty());
    assert!(compile_to_runtime_definition("Price Carrier", "Type: Creature\nPower/Toughness: 2/2\nYou may cast this spell for {1} if an unsupported imaginary condition happens.", false).is_err());
}

#[test]
fn reminder_identity_bound_upkeep_payment_tracks_control_and_is_optional() {
    for named in [false, true] {
        for can_pay in [false, true] {
            let rules = if named { "Echo {1}" } else {
                "At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay {1}."
            };
            let card = definition("Upkeep Adept", rules, (!named).then_some("Recall toll"));
            let trigger = card.abilities.iter().find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(trigger) => Some(trigger.clone()), _ => None,
            }).unwrap();
            let condition = trigger.intervening_if.as_ref().unwrap();
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            let hand = game.create_object_from_definition(&card, a, Zone::Hand);
            let source = game.move_object(hand, Zone::Battlefield, ironsmith::events::cause::EventCause::effect()).unwrap();
            game.mark_upkeep_began(a);
            let ctx = ironsmith::effects::EffectContext::new_default(source, a);
            assert!(ironsmith::condition_eval::evaluate_condition_resolution(&game, condition, &ctx).unwrap());
            if can_pay { game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 1); }
            game.push_to_stack(ironsmith::StackEntry::ability(source, a, trigger.effects.clone()));
            let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(game.object(source).is_some(), can_pay);
            assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
            if can_pay {
                game.mark_upkeep_began(a);
                assert!(!ironsmith::condition_eval::evaluate_condition_resolution(&game, condition, &ctx).unwrap(), "a second upkeep does not charge again without a new control change");
            }
        }
    }
}


#[test]
fn reminder_identity_bound_price_requires_and_returns_a_tapped_creature() {
    use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm, apply_priority_response_with_dm};
    for named in [false, true] {
        let rules = if named { "Web-slinging {1}" } else {
            "You may cast this spell for {1} if you also return a tapped creature you control to its owner's hand."
        };
        let card = definition("Returning Adept", &format!("Mana cost: {{6}}\n{rules}"), (!named).then_some("Recall"));
        let a = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.turn.active_player = a;
        game.turn.priority_player = Some(a);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        game.turn.step = None;
        game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 1);
        let spell = game.create_object_from_definition(&card, a, Zone::Hand);
        let helper = game.create_object_from_definition(&definition("Helper", "", None), a, Zone::Battlefield);
        let cast = |game: &GameState| compute_legal_actions(game, a).unwrap().into_iter().find(|action|
            matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell));
        assert!(cast(&game).is_none(), "untapped creatures cannot pay this cost");
        game.tap(helper);
        let action = cast(&game).expect("a tapped creature enables the alternative payment");
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut state = PriorityLoopState::new(2);
        let mut dm = SelectFirstDecisionMaker;
        let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
        for _ in 0..32 {
            if !state.has_pending_action() { break; }
            let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}") };
            progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
        }
        assert!(!state.has_pending_action());
        assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
        assert!(game.object(helper).is_none(), "payment removes the battlefield object");
        assert!(game.player(a).unwrap().hand.iter().any(|id| game.object(*id).unwrap().name == "Helper"));
        assert_eq!(game.stack.len(), 1);
    }
}


#[test]
fn reminder_identity_bound_team_price_uses_the_casting_players_team() {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    for named in [false, true] {
        for actor in 0..3 {
            let rules = if named { "Surge {1}" } else {
                "You may cast this spell for {1} if you or a teammate has cast another spell this turn."
            };
            let card = definition("Team Adept", &format!("Mana cost: {{6}}\n{rules}"), (!named).then_some("Team price"));
            assert_eq!(card.alternative_casts.len(), 1);
            assert_eq!(card.alternative_casts[0].name() == "Surge", named);
            let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into(), "D".into()], 20);
            let ids: Vec<_> = (0..4).map(PlayerId::from_index).collect();
            game.set_teams(vec![vec![ids[0], ids[1]], vec![ids[2], ids[3]]]).unwrap();
            let a = ids[0];
            game.turn.active_player = a;
            game.turn.priority_player = Some(a);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.turn.step = None;
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 1);
            let spell = game.create_object_from_definition(&card, a, Zone::Hand);
            let castable = |game: &GameState| compute_legal_actions(game, a).unwrap().iter().any(|action|
                matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell));
            assert!(!castable(&game));
            let other_spell = game.new_object_id();
            let event = ironsmith::events::RawEvent::new(
                ironsmith::events::SpellCastEvent::new(other_spell, ids[actor], Zone::Hand),
                ironsmith::ProvNodeId::default(),
            );
            game.turn_store.turn_history.record_event(&event, None, None);
            assert_eq!(castable(&game), actor < 2, "only you or your teammate enables this price");
        }
    }
}


// Deferred with the 130-reminder remediation batch: do not execute until
// every baseline failure has an implementation and behavior coverage.
#[test]
fn reminder_identity_counter_damage_matches_native_without_identity() {
    use ironsmith::effect::Effect;
    use ironsmith::effects::{EffectContext, execute_effect};
    use ironsmith::target::ChooseSpec;
    use ironsmith::object::CounterType;
    for (keyword, prose, identity, poison) in [
        ("Wither", "This deals damage to creatures in the form of -1/-1 counters.", StaticAbilityId::Wither, false),
        ("Infect", "This creature deals damage to creatures in the form of -1/-1 counters and to players in the form of poison counters.", StaticAbilityId::Infect, true),
    ] {
        for custom in [false, true] {
            for zone in [Zone::Battlefield, Zone::Stack, Zone::Graveyard, Zone::Hand] {
                for amount in [0, 1] {
                    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                    let a = PlayerId::from_index(0);
                    let b = PlayerId::from_index(1);
                    let source = game.create_object_from_definition(&definition("Source", if custom { prose } else { keyword }, custom.then_some("Corrode")), a, zone);
                    let target = game.create_object_from_definition(&definition("Recipient", "", None), b, Zone::Battlefield);
                    assert_eq!(game.object(source).unwrap().has_static_ability_id(identity), !custom);
                    let mut ctx = EffectContext::new_default(source, a);
                    execute_effect(&mut game, &Effect::deal_damage(amount, ChooseSpec::SpecificObject(target)), &mut ctx).unwrap();
                    assert_eq!(game.damage_on(target), 0);
                    assert_eq!(game.counter_count(target, CounterType::MinusOneMinusOne), amount as u32);
                    execute_effect(&mut game, &Effect::deal_damage(amount, ChooseSpec::SpecificPlayer(b)), &mut ctx).unwrap();
                    assert_eq!(game.player(b).unwrap().life, 20 - if poison { 0 } else { amount });
                    assert_eq!(game.player(b).unwrap().counter_count(CounterType::Poison), if poison { amount as u32 } else { 0 });
                }
            }
        }
    }
}

#[test]
fn reminder_identity_counter_damage_snapshot_and_combat_paths() {
    use ironsmith::effect::Effect;
    use ironsmith::effects::{EffectContext, execute_effect};
    use ironsmith::target::ChooseSpec;
    use ironsmith::object::CounterType;
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
    for poison in [false, true] {
        let prose = if poison { "This creature deals damage to creatures in the form of -1/-1 counters and to players in the form of poison counters." } else { "This deals damage to creatures in the form of -1/-1 counters." };
        for departed in [false, true] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            let b = PlayerId::from_index(1);
            let source = game.create_object_from_definition(&definition("Source", prose, Some("Corrode")), a, Zone::Battlefield);
            let target = game.create_object_from_definition(&definition("Recipient", "", None), b, Zone::Battlefield);
            let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(source).unwrap(), &game);
            if departed { game.remove_object(source); } else {
                game.object_mut(source).unwrap().abilities_mut().clear();
                game.refresh_continuous_state().unwrap();
            }
            let mut ctx = EffectContext::new_default(source, a).with_source_snapshot(snapshot);
            execute_effect(&mut game, &Effect::deal_damage(1, ChooseSpec::SpecificObject(target)), &mut ctx).unwrap();
            assert_eq!(game.counter_count(target, CounterType::MinusOneMinusOne), u32::from(departed));
            assert_eq!(game.damage_on(target), u32::from(!departed));
        }
        for blocked in [false, true] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            let b = PlayerId::from_index(1);
            let source = game.create_object_from_definition(&definition("Source", prose, Some("Corrode")), a, Zone::Battlefield);
            let target = game.create_object_from_definition(&definition("Recipient", "", None), b, Zone::Battlefield);
            let mut combat = CombatState::default();
            combat.attackers.push(AttackerInfo { creature: source, target: AttackTarget::Player(b) });
            combat.blockers.insert(source, if blocked { vec![target] } else { vec![] });
            ironsmith::game_loop::execute_combat_damage_step(&mut game, &combat, false);
            assert_eq!(game.counter_count(target, CounterType::MinusOneMinusOne), if blocked { 2 } else { 0 });
            assert_eq!(game.player(b).unwrap().counter_count(CounterType::Poison), if !blocked && poison { 2 } else { 0 });
            assert_eq!(game.player(b).unwrap().life, if !blocked && !poison { 18 } else { 20 });
        }
    }
}


#[test]
fn reminder_identity_destruction_immunity_preserves_non_destruction_deaths() {
    use ironsmith::effect::Effect;
    use ironsmith::effects::{EffectContext, execute_effect};
    use ironsmith::target::ChooseSpec;
    use ironsmith::object::CounterType;
    use ironsmith::rules::state_based::{check_state_based_actions, StateBasedAction};
    for native in [false, true] {
        for operation in ["destroy", "damage", "sacrifice", "zero toughness", "remove ability"] {
            let rules = if native { "Indestructible" } else { "Damage and effects that say \"destroy\" don't destroy this creature. If its toughness is 0 or less, it's still put into its owner's graveyard." };
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            let source = game.create_object_from_definition(&definition("Resilient", rules, (!native).then_some("Unbroken")), a, Zone::Battlefield);
            assert_eq!(game.object(source).unwrap().has_static_ability_id(StaticAbilityId::Indestructible), native);
            let mut ctx = EffectContext::new_default(source, a);
            match operation {
                "destroy" | "remove ability" => {
                    if operation == "remove ability" {
                        game.object_mut(source).unwrap().abilities_mut().clear();
                        game.refresh_continuous_state().unwrap();
                    }
                    execute_effect(&mut game, &Effect::destroy(ChooseSpec::SpecificObject(source)), &mut ctx).unwrap();
                    assert_eq!(game.object(source).is_some(), operation == "destroy");
                }
                "damage" => {
                    execute_effect(&mut game, &Effect::deal_damage(4, ChooseSpec::SpecificObject(source)), &mut ctx).unwrap();
                    assert_eq!(game.damage_on(source), 4);
                    assert!(!check_state_based_actions(&game).contains(&StateBasedAction::ObjectDies(source)));
                }
                "sacrifice" => {
                    execute_effect(&mut game, &Effect::sacrifice_source(), &mut ctx).unwrap();
                    assert!(game.object(source).is_none());
                }
                _ => {
                    game.add_counters(source, CounterType::MinusOneMinusOne, 2);
                    assert!(check_state_based_actions(&game).contains(&StateBasedAction::ObjectDies(source)));
                }
            }
        }
    }
}

#[test]
fn reminder_identity_named_activation_timing_keeps_sorcery_window() {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    for label in ["Equip", "Fortify", "Transmute", "Transfigure", "Level up", "Scavenge", "Unearth", "Outlast", "Reconfigure", "Craft", "Station", "Activate"] {
        let rules = format!("{{1}}: Put a charge counter on this creature. {label} only as a sorcery.");
        let card = definition("Charge Bearer", &rules, Some("Moon ritual"));
        let activated = card.abilities.iter().find_map(|ability| match &ability.kind { AbilityKind::Activated(a) => Some(a), _ => None }).unwrap();
        assert!(activated.keyword.is_none());
        assert_eq!(activated.timing, ironsmith::ability::ActivationTiming::SorcerySpeed);
        for own_main in [false, true] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            game.turn.active_player = PlayerId::from_index(if own_main { 0 } else { 1 });
            game.turn.priority_player = Some(a);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.turn.step = None;
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 1);
            let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
            let actions = compute_legal_actions(&game, a).unwrap();
            assert_eq!(actions.iter().any(|action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)), own_main, "{label}");
        }
    }
}


fn finish_reminder_cast(game: &mut GameState, action: ironsmith::decision::LegalAction) {
    finish_reminder_cast_with(game, action, &mut ironsmith::decision::SelectFirstDecisionMaker);
}

fn finish_reminder_cast_with(game: &mut GameState, action: ironsmith::decision::LegalAction, dm: &mut impl ironsmith::decision::DecisionMaker) -> ironsmith::triggers::TriggerQueue {
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm};
    let mut state = PriorityLoopState::new(game.players.len());
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..48 {
        if !state.has_pending_action() { break; }
        let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}") };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(!state.has_pending_action());
    queue
}

#[test]
fn reminder_identity_graveyard_additional_payment_keeps_printed_mana() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    for (keyword, reminder) in [
        ("Retrace", "You may cast this card from your graveyard by discarding a land card in addition to paying its other costs."),
        ("Jump-start", "You may cast this card from your graveyard by discarding a card in addition to paying its other costs. Then exile this card."),
    ] {
        for native in [false, true] {
            let rules = if native { keyword } else { reminder };
            let card = compile_to_runtime_definition("Returning Spell", &format!("Type: Sorcery\nMana cost: {{6}}\n{rules}\nYou gain 1 life."), false).unwrap();
            assert_eq!(card.alternative_casts[0].name() == keyword, native);
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            game.turn.active_player = a;
            game.turn.priority_player = Some(a);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.turn.step = None;
            let source = game.create_object_from_definition(&card, a, Zone::Graveyard);
            let land = compile_to_runtime_definition("Fuel", "Type: Land", false).unwrap();
            game.create_object_from_definition(&land, a, Zone::Hand);
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 5);
            let cast = |game: &GameState| compute_legal_actions(game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == source));
            assert!(cast(&game).is_none(), "the reminder cannot discard a card instead of paying printed mana");
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 1);
            let action = cast(&game).expect("complete printed mana plus discard must be usable");
            finish_reminder_cast(&mut game, action);
            assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
            assert!(game.player(a).unwrap().hand.is_empty());
            assert_eq!(game.stack.len(), 1);
            let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(game.player(a).unwrap().life, 21);
            assert_eq!(game.exile.iter().filter(|id| game.object(**id).unwrap().name == "Returning Spell").count(), usize::from(keyword == "Jump-start"));
        }
    }
}

#[test]
fn reminder_identity_exile_instruction_covers_every_stack_departure() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    for native in [false, true] {
        for departure in ["resolve", "counter", "bounce"] {
            let rules = if native { "Flashback {1}" } else { "You may cast this card from your graveyard for {1}. Then exile it." };
            let card = compile_to_runtime_definition("Returning Spell", &format!("Type: Sorcery\nMana cost: {{6}}\n{rules}\nYou gain 1 life."), false).unwrap();
            assert_eq!(card.alternative_casts[0].name() == "Flashback", native);
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            game.turn.active_player = a;
            game.turn.priority_player = Some(a);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.turn.step = None;
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 1);
            let source = game.create_object_from_definition(&card, a, Zone::Graveyard);
            let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == source)).unwrap();
            finish_reminder_cast(&mut game, action);
            let spell = game.stack[0].object_id;
            if departure == "resolve" {
                let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            } else {
                let effect = if departure == "counter" {
                    ironsmith::effect::Effect::counter(ironsmith::target::ChooseSpec::SpecificObject(spell))
                } else {
                    ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(spell), Zone::Hand, false))
                };
                let mut ctx = ironsmith::effects::EffectContext::new_default(spell, a);
                ironsmith::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
            }
            assert!(game.exile.iter().any(|id| game.object(*id).unwrap().name == "Returning Spell"), "{departure} must exile the spell");
            assert!(game.player(a).unwrap().hand.is_empty());
        }
    }
}


#[test]
fn reminder_identity_cast_linked_face_uses_back_face_characteristics() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    for (keyword, reminder, origin) in [
        ("Disturb {1}", "You may cast this card from your graveyard transformed for {1}.", Zone::Graveyard),
        ("More than meets the eye {1}", "You may cast this card converted for {1}.", Zone::Hand),
    ] {
        for native in [false, true] {
            let rules = if native { keyword } else { reminder };
            let mut front = definition("Dormant", &format!("Mana cost: {{6}}\n{rules}"), (!native).then_some("Awakened form"));
            let mut back = compile_to_runtime_definition("Awake", "Type: Creature\nPower/Toughness: 5/5\nFlying", false).unwrap();
            front.card.other_face = Some(back.card.id);
            front.card.other_face_name = Some(back.card.name.to_string());
            front.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
            front.card.transforming_dfc = true;
            back.card.other_face = Some(front.card.id);
            back.card.other_face_name = Some(front.card.name.to_string());
            back.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
            back.card.transforming_dfc = true;
            assert!(front.alternative_casts[0].casts_transformed());
            if !native { assert_eq!(front.alternative_casts[0].name(), if origin == Zone::Hand { "Parsed zone alternative cost" } else { "Parsed graveyard alternative cost" }); }
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            game.register_linked_face_definition(&back);
            game.turn.active_player = a;
            game.turn.priority_player = Some(a);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.turn.step = None;
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 1);
            let source = game.create_object_from_definition(&front, a, origin);
            let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == source)).expect("linked back face should be castable");
            finish_reminder_cast(&mut game, action);
            assert_eq!(game.object(game.stack[0].object_id).unwrap().name, "Awake");
            let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert!(game.battlefield.iter().any(|id| game.object(*id).unwrap().name == "Awake"));
            assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
        }
    }
}


#[test]
fn reminder_identity_discard_condition_uses_actual_discarded_card() {
    use ironsmith::effects::{EffectContext, execute_effect};
    for native in [false, true] {
        for land in [false, true] {
            let rules = if native { "Recruit." } else { "Draw a card, then discard a card. If you discarded a nonland card, create a 1/1 white Human Soldier creature token." };
            let card = compile_to_runtime_definition("Recruiting Spell", &format!("Type: Sorcery\n{rules}"), false).unwrap();
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            let source = game.create_object_from_definition(&card, a, Zone::Stack);
            let drawn = compile_to_runtime_definition("Drawn", if land { "Type: Land" } else { "Type: Creature\nPower/Toughness: 1/1" }, false).unwrap();
            game.create_object_from_definition(&drawn, a, Zone::Library);
            // An unrelated nonland card in the graveyard must not satisfy it.
            game.create_object_from_definition(&definition("Old card", "", None), a, Zone::Graveyard);
            let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
            let mut ctx = EffectContext::new(source, a, &mut dm);
            for effect in card.spell_effect.as_ref().unwrap().all_effects() {
                execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
            drop(ctx);
            assert!(game.player(a).unwrap().hand.is_empty());
            assert_eq!(game.battlefield.len(), usize::from(!land));
        }
    }
}

#[test]
fn reminder_identity_living_metal_tracks_current_controller() {
    use ironsmith::types::CardType;
    for native in [false, true] {
        let rules = if native { "Living metal" } else { "During your turn, this Vehicle is also a creature." };
        let mut builder = compiler::CardDefinitionBuilder::new(compiler::ids::CardId::new(), "Mobile Hull");
        if !native { builder = builder.with_custom_keyword("Awakened hull"); }
        let card = compile_builder_to_runtime_definition(builder, format!("Type: Artifact — Vehicle\nPower/Toughness: 3/3\n{rules}"), false).unwrap();
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0);
        let b = PlayerId::from_index(1);
        let id = game.create_object_from_definition(&card, a, Zone::Battlefield);
        assert_eq!(game.object_has_static_ability_id(id, StaticAbilityId::LivingMetal), native);
        for active in [a, b] {
            game.turn.active_player = active;
            assert_eq!(game.current_has_card_type(id, CardType::Creature), active == a);
            assert!(game.current_has_card_type(id, CardType::Artifact));
        }
        game.set_current_controller(id, b).unwrap();
        assert!(game.current_has_card_type(id, CardType::Creature));
        game.turn.active_player = a;
        assert!(!game.current_has_card_type(id, CardType::Creature));
    }
}

#[test]
fn reminder_identity_combat_mana_retains_only_its_own_production() {
    use ironsmith::game_state::{Phase, Step};
    use ironsmith::mana::ManaSymbol;
    for native in [false, true] {
        let rules = if native { "Firebending 2" } else { "Whenever this creature attacks, add {R}{R}. This mana lasts until end of combat." };
        let card = definition("Combat Mage", rules, (!native).then_some("Battle fuel"));
        let trigger = card.abilities.iter().find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(trigger) => Some(trigger.clone()), _ => None,
        }).unwrap();
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0);
        game.turn.active_player = a;
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
        game.player_mut(a).unwrap().mana_pool.add(ManaSymbol::Red, 1);
        game.push_to_stack(ironsmith::StackEntry::ability(source, a, trigger.effects.clone()));
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.player(a).unwrap().mana_pool.total(), 3);
        game.empty_mana_pools().unwrap();
        assert_eq!(game.player(a).unwrap().mana_pool.total(), 2);
        game.turn.step = Some(Step::DeclareBlockers);
        game.player_mut(a).unwrap().mana_pool.add(ManaSymbol::Red, 1);
        game.empty_mana_pools().unwrap();
        assert_eq!(game.player(a).unwrap().mana_pool.total(), 2);
        game.turn.step = Some(Step::EndCombat);
        game.empty_mana_pools().unwrap();
        assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
    }
}

#[test]
fn reminder_identity_library_partition_matches_native_for_every_subset() {
    use ironsmith::decision::DecisionMaker;
    use ironsmith::decisions::context::{OrderContext, PartitionContext, SelectObjectsContext};
    struct PartitionChoice { selected: Vec<ironsmith::ObjectId>, chooser: PlayerId }
    impl DecisionMaker for PartitionChoice {
        fn decide_partition(&mut self, _: &GameState, ctx: &PartitionContext) -> Vec<ironsmith::ObjectId> {
            assert_eq!(ctx.player, self.chooser);
            self.selected.clone()
        }
        fn decide_objects(&mut self, _: &GameState, ctx: &SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            assert_eq!(ctx.player, self.chooser);
            self.selected.iter().copied().filter(|id| ctx.candidates.iter().any(|c| c.id == *id && c.legal)).collect()
        }
        fn decide_order(&mut self, _: &GameState, ctx: &OrderContext) -> Vec<ironsmith::ObjectId> {
            assert_eq!(ctx.player, self.chooser);
            // One desired top-to-bottom arrangement, independent of how the
            // prompt lists the cards and of which edge its answer starts from.
            let mut items = ctx.items.clone();
            items.sort_by(|left, right| right.1.cmp(&left.1));
            if ctx.description.contains("first option becomes the bottom-most") { items.reverse(); }
            items.into_iter().map(|(id, _)| id).collect()
        }
    }
    for (keyword, reminder, opponent_library) in [
        ("Surveil 2.", "To surveil 2, look at the top two cards of your library, then put any number of them into your graveyard and the rest on top of your library in any order.", false),
        ("Scry 2.", "Look at the top two cards of your library, then put any number of them on the bottom of your library and the rest on top in any order.", false),
        ("Fateseal 2.", "To fateseal 2, look at the top two cards of an opponent's library, then put any number of them on the bottom of that player's library and the rest on top in any order.", true),
    ] {
        for size in 0..=4 {
            for mask in 0..(1 << size.min(2)) {
                let mut results = Vec::new();
                for native in [true, false] {
                    let card = compile_to_runtime_definition("Library Spell", &format!("Type: Sorcery\n{}", if native { keyword } else { reminder }), false).unwrap();
                    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                    let a = PlayerId::from_index(0);
                    let owner = PlayerId::from_index(u8::from(opponent_library));
                    let source = game.create_object_from_definition(&card, a, Zone::Stack);
                    for index in 0..size {
                        game.create_object_from_definition(&definition(&format!("Card {index}"), "", None), owner, Zone::Library);
                    }
                    let selected = game.player(owner).unwrap().library.iter().rev().take(2).enumerate().filter_map(|(i, id)| (mask & (1 << i) != 0).then_some(*id)).collect();
                    let mut dm = PartitionChoice { selected, chooser: a };
                    let mut ctx = ironsmith::effects::EffectContext::new(source, a, &mut dm);
                    for effect in card.spell_effect.as_ref().unwrap().all_effects() {
                        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                    }
                    drop(ctx);
                    let names = |ids: &[ironsmith::ObjectId]| ids.iter().map(|id| game.object(*id).unwrap().name.clone()).collect::<Vec<_>>();
                    let player = game.player(owner).unwrap();
                    // CR 404.3: the owner arranges cards put into a graveyard
                    // simultaneously, so only the library order is behavioral.
                    let mut graveyard = names(&player.graveyard);
                    graveyard.sort();
                    results.push((names(&player.library), graveyard));
                }
                assert_eq!(results[0], results[1], "{keyword}, library size {size}, selected subset {mask}");
            }
        }
    }
}

#[test]
fn reminder_identity_reciprocal_block_filter_does_not_grant_referenced_keyword() {
    let custom = definition("Border Guard", "This creature can block or be blocked by only creatures with vigilance.", Some("Guarded combat"));
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let a = PlayerId::from_index(0);
    let b = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&custom, a, Zone::Battlefield);
    let plain = game.create_object_from_definition(&definition("Plain", "", None), b, Zone::Battlefield);
    let qualified = game.create_object_from_definition(&definition("Vigilant", "Vigilance", None), b, Zone::Battlefield);
    assert!(!game.object_has_static_ability_id(source, StaticAbilityId::Vigilance));
    for (other, permitted) in [(plain, false), (qualified, true)] {
        assert_eq!(ironsmith::rules::combat::can_block(game.object(source).unwrap(), game.object(other).unwrap(), &game), permitted);
        assert_eq!(ironsmith::rules::combat::can_block(game.object(other).unwrap(), game.object(source).unwrap(), &game), permitted);
    }
    let shadow = definition("Veiled", "This creature can block or be blocked by only creatures with shadow.", Some("Veiled combat"));
    let veiled = game.create_object_from_definition(&shadow, a, Zone::Battlefield);
    assert!(!game.object_has_static_ability_id(veiled, StaticAbilityId::Shadow));
    assert!(!ironsmith::rules::combat::can_block(game.object(veiled).unwrap(), game.object(plain).unwrap(), &game));
    assert!(!ironsmith::rules::combat::can_block(game.object(plain).unwrap(), game.object(veiled).unwrap(), &game));
}

#[test]
fn reminder_identity_attachment_explanation_preserves_detach_behavior() {
    for (subtype, target_type, label) in [("Equipment", "creature", "Equip"), ("Fortification", "land", "Fortify")] {
        for native in [true, false] {
            let rules = if native { format!("{label} {{3}}") } else { format!("{{3}}: Attach to target {target_type} you control. {label} only as a sorcery. This card enters unattached and stays on the battlefield if the {target_type} leaves.") };
            let card = compile_to_runtime_definition("Attachment", &format!("Type: Artifact — {subtype}\n{rules}"), false).unwrap();
            let ability = card.abilities.iter().find_map(|a| match &a.kind { AbilityKind::Activated(a) => Some(a), _ => None }).unwrap();
            assert_eq!(ability.keyword.is_some(), native);
            assert_eq!(ability.timing, ironsmith::ability::ActivationTiming::SorcerySpeed);
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
            assert!(game.object(source).unwrap().attached_to.is_none());
            let host = compile_to_runtime_definition("Host", &format!("Type: {target_type}\n{}", if target_type == "creature" { "Power/Toughness: 2/2" } else { "" }), false).unwrap();
            let target = game.create_object_from_definition(&host, a, Zone::Battlefield);
            let mut ctx = ironsmith::effects::EffectContext::new_default(source, a).with_targets(vec![ironsmith::effects::ResolvedTarget::Object(target)]);
            for effect in ability.effects.all_effects() { ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
            assert_eq!(game.object(source).unwrap().attached_to, Some(ironsmith::object::AttachmentTarget::Object(target)));
            game.move_object(target, Zone::Graveyard, ironsmith::events::cause::EventCause::effect()).unwrap();
            assert!(game.battlefield.contains(&source));
            assert!(game.object(source).unwrap().attached_to.is_none());
        }
    }
}

#[test]
fn reminder_identity_filtered_targeting_checks_colors_and_controller() {
    for native in [false, true] {
        let rules = if native { "Hexproof from monocolored" } else { "This creature can't be the target of monocolored spells or abilities your opponents control." };
        let card = definition("Protected", rules, (!native).then_some("Chromatic veil"));
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0);
        let b = PlayerId::from_index(1);
        let target = game.create_object_from_definition(&card, a, Zone::Battlefield);
        if !native { assert!(!game.object_has_static_ability_id(target, StaticAbilityId::HexproofFrom)); }
        for controller in [a, b] {
            for (mana, monocolored) in [("{1}", false), ("{R}", true), ("{U}{R}", false)] {
                for (kind, zone) in [("Instant", Zone::Stack), ("Creature", Zone::Battlefield)] {
                    let source_card = compile_to_runtime_definition("Source", &format!("Type: {kind}\nMana cost: {mana}\n{}", if kind == "Creature" { "Power/Toughness: 2/2" } else { "" }), false).unwrap();
                    let source = game.create_object_from_definition(&source_card, controller, zone);
                    let result = ironsmith::targeting::can_target_object(&game, target, source, controller);
                    assert_eq!(result.is_legal(), controller == a || !monocolored, "native={native}, {kind}, {mana}, controller={controller:?}");
                }
            }
        }
    }
}

#[test]
fn reminder_identity_source_mana_value_survives_activation_cost() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    use ironsmith::mana::ManaSymbol;
    for (keyword, reminder, origin, destination, color) in [
        ("Transmute {1}{U}{U}", "{1}{U}{U}, Discard this card: Search your library for a card with the same mana value as this card, reveal it, put it into your hand, then shuffle. Transmute only as a sorcery.", Zone::Hand, Zone::Hand, ManaSymbol::Blue),
        ("Transfigure {1}{B}{B}", "{1}{B}{B}, Sacrifice this creature: Search your library for a creature card with the same mana value as this creature, put that card onto the battlefield, then shuffle. Transfigure only as a sorcery.", Zone::Battlefield, Zone::Battlefield, ManaSymbol::Black),
    ] {
        for native in [false, true] {
            let card = definition("Changing Source", &format!("Mana cost: {{4}}\n{}", if native { keyword } else { reminder }), (!native).then_some("Exchange form"));
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            game.turn.active_player = a;
            game.turn.priority_player = Some(a);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.turn.step = None;
            let source = game.create_object_from_definition(&card, a, origin);
            for (name, mana) in [("Wrong value", "{5}"), ("Matching value", "{4}")] {
                game.create_object_from_definition(&definition(name, &format!("Mana cost: {mana}"), None), a, Zone::Library);
            }
            game.player_mut(a).unwrap().mana_pool.add(color, 2);
            game.player_mut(a).unwrap().mana_pool.add(ManaSymbol::Colorless, 1);
            let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)).unwrap();
            finish_reminder_cast(&mut game, action);
            assert!(game.player(a).unwrap().graveyard.iter().any(|id| game.object(*id).unwrap().name == "Changing Source"));
            assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
            let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            let result = if destination == Zone::Hand { &game.player(a).unwrap().hand } else { &game.battlefield };
            assert!(result.iter().any(|id| game.object(*id).unwrap().name == "Matching value"));
            assert!(game.player(a).unwrap().library.iter().any(|id| game.object(*id).unwrap().name == "Wrong value"));
        }
    }
}

#[test]
fn reminder_identity_level_counter_activation_pays_and_increments() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    for native in [false, true] {
        let card = definition("Growing", if native { "Level up {3}{R}" } else { "{3}{R}: Put a level counter on this. Level up only as a sorcery." }, (!native).then_some("Development"));
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0);
        game.turn.active_player = a;
        game.turn.priority_player = Some(a);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        game.turn.step = None;
        let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
        for expected in 1..=2 {
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Red, 1);
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 3);
            let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)).unwrap();
            finish_reminder_cast(&mut game, action);
            let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(game.counter_count(source, ironsmith::object::CounterType::Level), expected);
            assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
        }
    }
}

#[test]
fn reminder_identity_exiled_source_power_determines_counter_count() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    for native in [false, true] {
        for power in [0, 5] {
            let rules = if native { "Scavenge {5}{G}{G}" } else { "{5}{G}{G}, Exile this card from your graveyard: Put a number of +1/+1 counters equal to this card's power on target creature. Scavenge only as a sorcery." };
            let card = compile_to_runtime_definition("Source", &format!("Type: Creature\nPower/Toughness: {power}/3\n{rules}"), false).unwrap();
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            game.turn.active_player = a;
            game.turn.priority_player = Some(a);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.turn.step = None;
            let source = game.create_object_from_definition(&card, a, Zone::Graveyard);
            let target = game.create_object_from_definition(&definition("Recipient", "", None), a, Zone::Battlefield);
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Green, 2);
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 5);
            let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)).unwrap();
            finish_reminder_cast(&mut game, action);
            assert!(game.exile.iter().any(|id| game.object(*id).unwrap().name == "Source"));
            let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(game.counter_count(target, ironsmith::object::CounterType::PlusOnePlusOne), power as u32);
            assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
        }
    }
}

#[test]
fn reminder_identity_spelled_out_energy_price_is_a_real_payment() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    for price in ["Pay eight energy counters", "Pay 8 {E}"] {
        let card = definition("Energy Sink", &format!("{price}: You gain 1 life."), None);
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0);
        game.turn.active_player = a;
        game.turn.priority_player = Some(a);
        let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
        let actions = |game: &GameState| compute_legal_actions(game, a).unwrap().into_iter().filter(|action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)).collect::<Vec<_>>();
        game.add_player_counters_with_source(a, ironsmith::object::CounterType::Energy, 7, None, None).unwrap();
        assert!(actions(&game).is_empty());
        game.add_player_counters_with_source(a, ironsmith::object::CounterType::Energy, 1, None, None).unwrap();
        let action = actions(&game).pop().unwrap();
        finish_reminder_cast(&mut game, action);
        assert_eq!(game.player(a).unwrap().counter_count(ironsmith::object::CounterType::Energy), 0);
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.player(a).unwrap().life, 21);
    }
}

#[test]
fn reminder_identity_greater_power_attack_checks_declaration_not_resolution() {
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use ironsmith::events::combat::{AttackEventTarget, CreatureAttackedEvent};
    use ironsmith::triggers::{TriggerContext, TriggerEvent};
    for native in [true, false] {
        let card = definition("Trainee", if native { "Training" } else { "Whenever this creature attacks with another creature with greater power, put a +1/+1 counter on this creature." }, (!native).then_some("Apprenticeship"));
        let trigger = card.abilities.iter().find_map(|ability| match &ability.kind { AbilityKind::Triggered(t) => Some(t), _ => None }).unwrap();
        for power in [1, 2, 3] {
            for attacks in [false, true] {
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let a = PlayerId::from_index(0);
                let b = PlayerId::from_index(1);
                let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
                let other_card = compile_to_runtime_definition("Partner", &format!("Type: Creature\nPower/Toughness: {power}/3"), false).unwrap();
                let other = game.create_object_from_definition(&other_card, a, Zone::Battlefield);
                let mut combat = CombatState::default();
                combat.attackers.push(AttackerInfo { creature: source, target: AttackTarget::Player(b) });
                if attacks { combat.attackers.push(AttackerInfo { creature: other, target: AttackTarget::Player(b) }); }
                game.combat = Some(combat);
                let event = TriggerEvent::new_with_provenance(CreatureAttackedEvent::new(source, AttackEventTarget::Player(b)), Default::default());
                let ctx = TriggerContext::for_source(source, a, &game);
                let matches = trigger.trigger.matches(&event, &ctx);
                assert_eq!(matches, attacks && power > 2);
                if matches {
                    assert!(trigger.intervening_if.is_none());
                    game.push_to_stack(ironsmith::StackEntry::ability(source, a, trigger.effects.clone()));
                    game.move_object(other, Zone::Graveyard, ironsmith::events::cause::EventCause::effect()).unwrap();
                    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
                    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                    assert_eq!(game.counter_count(source, ironsmith::object::CounterType::PlusOnePlusOne), 1);
                }
            }
        }
    }
}

#[test]
fn reminder_identity_aura_definition_targets_and_enters_attached() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    for native in [true, false] {
        let mut builder = compiler::CardDefinitionBuilder::new(compiler::ids::CardId::new(), "Bond");
        if !native { builder = builder.with_custom_keyword("Bind creature"); }
        let rules = if native { "Enchant creature" } else { "Target a creature as you cast this. This card enters attached to that creature." };
        let card = compile_builder_to_runtime_definition(builder, format!("Type: Enchantment — Aura\nMana cost: {{1}}\n{rules}"), false).unwrap();
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0);
        game.turn.active_player = a;
        game.turn.priority_player = Some(a);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        game.turn.step = None;
        game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 1);
        let source = game.create_object_from_definition(&card, a, Zone::Hand);
        let actions = |game: &GameState| compute_legal_actions(game, a).unwrap().into_iter().filter(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == source)).collect::<Vec<_>>();
        let land = compile_to_runtime_definition("Land", "Type: Land", false).unwrap();
        game.create_object_from_definition(&land, a, Zone::Battlefield);
        assert!(actions(&game).is_empty(), "an Aura must have an eligible casting target");
        let host = game.create_object_from_definition(&definition("Host", "", None), a, Zone::Battlefield);
        let action = actions(&game).pop().unwrap();
        finish_reminder_cast(&mut game, action);
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let aura = *game.battlefield.iter().find(|id| game.object(**id).unwrap().name == "Bond").unwrap();
        assert_eq!(game.object(aura).unwrap().attached_to, Some(ironsmith::object::AttachmentTarget::Object(host)));
        game.move_object(host, Zone::Graveyard, ironsmith::events::cause::EventCause::effect()).unwrap();
        assert!(!ironsmith::rules::state_based::check_state_based_actions(&game).is_empty(), "an unattached Aura must be put into its owner's graveyard");
    }
}

#[test]
fn reminder_identity_countdowns_keep_fading_and_vanishing_distinct() {
    use ironsmith::object::CounterType;
    use ironsmith::triggers::TriggerContext;
    for (keyword, reminder, counter, initial) in [
        ("Fading 7", "This enchantment enters with seven fade counters on it. At the beginning of your upkeep, remove a fade counter from it. If you can't, sacrifice it.", CounterType::Fade, 7),
        ("Vanishing 12", "This enchantment enters with twelve time counters on it. At the beginning of your upkeep, remove a time counter from it. When the last is removed, sacrifice it.", CounterType::Time, 12),
    ] {
        for native in [true, false] {
            let card = compile_to_runtime_definition("Countdown", &format!("Type: Enchantment\n{}", if native { keyword } else { reminder }), false).unwrap();
            let triggers = card.abilities.iter().filter_map(|ability| match &ability.kind { AbilityKind::Triggered(t) => Some(t.clone()), _ => None }).collect::<Vec<_>>();
            let upkeep = &triggers[0];
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            let in_hand = game.create_object_from_definition(&card, a, Zone::Hand);
            let source = enter_reminder_permanent(&mut game, in_hand);
            assert_eq!(game.counter_count(source, counter), initial);
            game.take_pending_trigger_events();
            let mut last_removal = None;
            for remaining in (0..initial).rev() {
                let mut events = Vec::new();
                let mut context = ironsmith::effects::EffectContext::new_default(source, a);
                for effect in upkeep.effects.all_effects() {
                    events.extend(ironsmith::effects::execute_effect(&mut game, effect, &mut context).unwrap().events);
                }
                assert_eq!(game.counter_count(source, counter), remaining);
                assert!(game.battlefield.contains(&source));
                if counter == CounterType::Time {
                    let ctx = TriggerContext::for_source(source, a, &game);
                    let matched = events.iter().any(|event| triggers[1].trigger.matches(event, &ctx));
                    assert_eq!(matched, remaining == 0);
                    if matched { last_removal = events.iter().find(|event| triggers[1].trigger.matches(event, &ctx)).cloned(); }
                }
            }
            if counter == CounterType::Time {
                let ctx = ironsmith::effects::EffectContext::new_default(source, a);
                assert!(!ironsmith::condition_eval::evaluate_condition_resolution(&game, upkeep.intervening_if.as_ref().expect("full-rule upkeep gate"), &ctx).unwrap());
            }
            let final_effects = if counter == CounterType::Time { triggers[1].effects.clone() } else { upkeep.effects.clone() };
            let mut entry = ironsmith::StackEntry::ability(source, a, final_effects);
            if let Some(event) = last_removal { entry = entry.with_triggering_event(event); }
            game.push_to_stack(entry);
            let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert!(!game.battlefield.contains(&source));
        }
    }
}

#[test]
fn reminder_identity_phasing_uses_turn_action_and_preserves_attachments() {
    for native in [true, false] {
        let card = definition("Shimmering", if native { "Phasing" } else { "This phases in or out before you untap during each of your untap steps. While it's phased out, it's treated as though it doesn't exist." }, (!native).then_some("Flickering"));
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0);
        let b = PlayerId::from_index(1);
        let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
        assert_eq!(game.object_has_static_ability_id(source, StaticAbilityId::Phasing), native);
        let equipment = compile_to_runtime_definition("Equipment", "Type: Artifact — Equipment", false).unwrap();
        let attachment = game.create_object_from_definition(&equipment, a, Zone::Battlefield);
        assert!(game.attach_object_to_target(attachment, ironsmith::object::AttachmentTarget::Object(source)));
        game.tap(source);
        for (player, phased) in [(b, false), (a, true), (b, true), (a, false)] {
            game.turn.active_player = player;
            game.turn.turn_number += 1;
            let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
            ironsmith::turn::execute_untap_step_with(&mut game, &mut dm).unwrap();
            assert_eq!(game.is_phased_out(source), phased);
            assert_eq!(game.is_phased_out(attachment), phased);
            assert!(game.stack.is_empty(), "phasing is a turn-based action");
            assert_eq!(game.object(attachment).unwrap().attached_to, Some(ironsmith::object::AttachmentTarget::Object(source)));
            if phased { assert!(game.is_tapped(source), "phase-out precedes untapping"); }
        }
        assert!(!game.is_tapped(source), "a phased-in permanent participates in untapping");
    }
}

#[test]
fn reminder_identity_combat_poison_is_additional_and_independent_of_damage_amount() {
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use ironsmith::effects::{EffectContext, execute_effect};
    use ironsmith::object::CounterType;
    for native in [true, false] {
        for power in [0, 1, 4] {
            for blocked in [false, true] {
                let rules = if native { "Toxic 2" } else { "Players dealt combat damage by this creature also get two poison counters. A player with ten or more poison counters loses the game." };
                let card = compile_to_runtime_definition("Poison Source", &format!("Type: Creature\nPower/Toughness: {power}/6\n{rules}"), false).unwrap();
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let a = PlayerId::from_index(0);
                let b = PlayerId::from_index(1);
                let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
                assert_eq!(game.object_has_static_ability_id(source, StaticAbilityId::Toxic), native);
                let blocker = game.create_object_from_definition(&definition("Blocker", "", None), b, Zone::Battlefield);
                let mut ctx = EffectContext::new_default(source, a);
                execute_effect(&mut game, &ironsmith::effect::Effect::deal_damage(1, ironsmith::target::ChooseSpec::SpecificPlayer(b)), &mut ctx).unwrap();
                assert_eq!(game.player(b).unwrap().counter_count(CounterType::Poison), 0, "noncombat damage doesn't add poison");
                let mut combat = CombatState::default();
                combat.attackers.push(AttackerInfo { creature: source, target: AttackTarget::Player(b) });
                combat.blockers.insert(source, if blocked { vec![blocker] } else { vec![] });
                ironsmith::game_loop::execute_combat_damage_step(&mut game, &combat, false);
                assert_eq!(game.player(b).unwrap().life, 19 - if blocked { 0 } else { power });
                assert_eq!(game.player(b).unwrap().counter_count(CounterType::Poison), if !blocked && power > 0 { 2 } else { 0 });
                assert!(game.stack.is_empty(), "additional poison is a damage result, not a trigger");
            }
        }
    }
}

#[test]
fn reminder_identity_bound_price_includes_explained_behold_payment() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    let reminder = "You may cast this card from your graveyard for {1}{R}, Behold three Elementals. Then exile it. To behold an Elemental, choose an Elemental you control or reveal an Elemental card from your hand.";
    for native in [false, true] {
        let rules = if native { "Flashback {1}{R}, Behold three Elementals" } else { reminder };
        let card = compile_to_runtime_definition("Returning Spell", &format!("Type: Sorcery\nMana cost: {{6}}\n{rules}\nYou gain 1 life."), false).unwrap();
        assert_eq!(card.alternative_casts[0].name() == "Flashback", native);
        for hand_count in 0..=3 {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            game.turn.active_player = a;
            game.turn.priority_player = Some(a);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.turn.step = None;
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Red, 2);
            let source = game.create_object_from_definition(&card, a, Zone::Graveyard);
            let elemental = compile_to_runtime_definition("Witness", "Type: Creature — Elemental\nPower/Toughness: 1/1", false).unwrap();
            let cast = |game: &GameState| compute_legal_actions(game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == source));
            for n in 0..3 {
                assert!(cast(&game).is_none(), "{n} witnesses cannot pay for three");
                game.create_object_from_definition(&elemental, a, if n < hand_count { Zone::Hand } else { Zone::Battlefield });
            }
            let action = cast(&game).expect("three distinct witnesses pay the non-mana price");
            finish_reminder_cast(&mut game, action);
            assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
            assert_eq!(game.player(a).unwrap().hand.len(), hand_count);
            let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(game.player(a).unwrap().life, 21);
            assert!(game.exile.iter().any(|id| game.object(*id).unwrap().name == "Returning Spell"));
        }
    }
    for invalid in [
        reminder.replace("choose an Elemental", "choose a Dragon"),
        reminder.replace("Behold three Elementals", "Behold three Dragons"),
        reminder.replace("reveal an Elemental card", "discard an Elemental card"),
    ] {
        assert!(compile_to_runtime_definition("Invalid price", &format!("Type: Sorcery\n{invalid}"), false).is_err());
    }
}

#[test]
fn reminder_identity_discarded_card_cast_permission_is_local_and_timed() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    for native in [false, true] {
        let rules = if native { "Mayhem {3}{U}{R}" } else { "You may cast this card from your graveyard for {3}{U}{R} if you discarded it this turn. Timing rules still apply." };
        let card = compile_to_runtime_definition("Discarded Spell", &format!("Type: Sorcery\nMana cost: {{9}}\n{rules}\nYou gain 1 life."), false).unwrap();
        assert_eq!(card.alternative_casts[0].name() == "Mayhem", native);
        for reset_history in [false, true] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            game.turn.active_player = a;
            game.turn.priority_player = Some(a);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.turn.step = None;
            for (symbol, count) in [(ironsmith::mana::ManaSymbol::Colorless, 3), (ironsmith::mana::ManaSymbol::Blue, 1), (ironsmith::mana::ManaSymbol::Red, 1)] {
                game.player_mut(a).unwrap().mana_pool.add(symbol, count);
            }
            let old = game.create_object_from_definition(&card, a, Zone::Graveyard);
            let hand = game.create_object_from_definition(&card, a, Zone::Hand);
            let mut ctx = ironsmith::effects::EffectContext::new_default(hand, a);
            ironsmith::effects::execute_effect(&mut game, &ironsmith::effect::Effect::discard(1), &mut ctx).unwrap();
            let source = *game.player(a).unwrap().graveyard.iter().find(|id| **id != old).unwrap();
            if reset_history { game.turn_store.turn_history.clear_for_new_turn(); }
            let can_cast = |game: &GameState, source| compute_legal_actions(game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == source));
            assert!(can_cast(&game, old).is_none(), "discarding another copy grants no permission");
            game.turn.active_player = PlayerId::from_index(1);
            assert!(can_cast(&game, source).is_none(), "sorcery timing still applies");
            game.turn.active_player = a;
            let action = can_cast(&game, source);
            assert_eq!(action.is_some(), !reset_history);
            if let Some(action) = action {
                finish_reminder_cast(&mut game, action);
                assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
                let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                assert_eq!(game.player(a).unwrap().life, 21);
                assert!(game.exile.is_empty(), "the permission has no exile rider");
            }
        }
    }
}

#[test]
fn reminder_identity_combat_history_price_requires_qualifying_source_and_damage() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
    for native in [true, false] {
        let rules = if native { "Prowl {3}{U}" } else { "You may cast this spell for {3}{U} if you dealt combat damage to a player this turn with a Sphinx or Rogue." };
        let card = compile_to_runtime_definition("History Spell", &format!("Type: Creature — Sphinx Rogue\nMana cost: {{9}}\nPower/Toughness: 2/2\n{rules}"), false).unwrap();
        for subtype in ["Sphinx", "Rogue", "Bird"] {
            for combat_damage in [false, true] {
                for own_source in [false, true] {
                    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                    let a = PlayerId::from_index(0);
                    let b = PlayerId::from_index(1);
                    game.turn.active_player = a;
                    game.turn.priority_player = Some(a);
                    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
                    game.turn.step = None;
                    game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue, 4);
                    let spell = game.create_object_from_definition(&card, a, Zone::Hand);
                    let attacker = compile_to_runtime_definition("Attacker", &format!("Type: Creature — {subtype}\nPower/Toughness: 2/2"), false).unwrap();
                    let attacker = game.create_object_from_definition(&attacker, if own_source { a } else { b }, Zone::Battlefield);
                    if combat_damage {
                        let mut combat = CombatState::default();
                        combat.attackers.push(AttackerInfo { creature: attacker, target: AttackTarget::Player(b) });
                        combat.blockers.insert(attacker, vec![]);
                        ironsmith::game_loop::execute_combat_damage_step(&mut game, &combat, false);
                    } else {
                        let mut ctx = ironsmith::effects::EffectContext::new_default(attacker, if own_source { a } else { b });
                        ironsmith::effects::execute_effect(&mut game, &ironsmith::effect::Effect::deal_damage(2, ironsmith::target::ChooseSpec::SpecificPlayer(b)), &mut ctx).unwrap();
                    }
                    // Qualification uses the damage snapshot even if its source has left.
                    game.move_object(attacker, Zone::Graveyard, ironsmith::events::cause::EventCause::effect()).unwrap();
                    let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell));
                    assert_eq!(action.is_some(), own_source && combat_damage && subtype != "Bird", "{native} {subtype} {combat_damage} {own_source}");
                    if let Some(action) = action {
                        finish_reminder_cast(&mut game, action);
                        assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
                    }
                }
            }
        }
    }
}

#[test]
fn reminder_identity_optional_sacrifice_pays_before_resolution_without_bargain_identity() {
    use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
    use ironsmith::decisions::context::SelectOptionsContext;
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm};
    struct PayExtra(bool);
    impl DecisionMaker for PayExtra {
        fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            if ctx.description.starts_with("Choose optional costs") {
                return if self.0 { ctx.options.iter().find(|option| option.legal).map(|option| vec![option.index]).unwrap_or_default() } else { vec![] };
            }
            SelectFirstDecisionMaker.decide_options(game, ctx)
        }
    }
    for native in [false, true] {
        let rules = if native { "Bargain" } else { "You may sacrifice an artifact, enchantment, or token as you cast this spell." };
        let card = compile_to_runtime_definition("Optional Sacrifice", &format!("Type: Sorcery\nMana cost: {{1}}\n{rules}\nYou gain 1 life."), false).unwrap();
        for (kind, token, eligible) in [("Artifact", false, true), ("Enchantment", false, true), ("Creature", true, true), ("Creature", false, false)] {
            for pay in [false, true] {
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let a = PlayerId::from_index(0);
                game.turn.active_player = a;
                game.turn.priority_player = Some(a);
                game.turn.phase = ironsmith::game_state::Phase::FirstMain;
                game.turn.step = None;
                game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 1);
                let source = game.create_object_from_definition(&card, a, Zone::Hand);
                let victim_card = compile_to_runtime_definition("Payment", &format!("Type: {kind}\nPower/Toughness: 2/2"), false).unwrap();
                let victim = game.create_object_from_definition(&victim_card, a, Zone::Battlefield);
                if token { game.object_mut(victim).unwrap().kind = ironsmith::object::ObjectKind::Token; }
                let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == source)).unwrap();
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                let mut state = PriorityLoopState::new(2);
                let mut dm = PayExtra(pay);
                let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
                for _ in 0..48 {
                    if !state.has_pending_action() { break; }
                    let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}") };
                    progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
                }
                assert!(!state.has_pending_action());
                assert_eq!(game.battlefield.contains(&victim), !(pay && eligible));
                assert_eq!(game.stack.last().unwrap().optional_costs_paid.was_paid_label("Bargain"), native && pay && eligible);
                assert_eq!(game.player(a).unwrap().life, 20, "payment precedes spell resolution");
                assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                assert_eq!(game.player(a).unwrap().life, 21);
            }
        }
    }
}

#[test]
fn reminder_identity_overflow_assigns_lethal_before_planeswalker_controller_damage() {
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use ironsmith::object::CounterType;
    for native in [false, true] {
        for power in [1, 3, 8] {
            for blocked in [false, true] {
                for planeswalker_present in [false, true] {
                    let rules = if native { "Trample over planeswalkers" } else { "This creature can deal excess combat damage to the controller of the planeswalker it's attacking." };
                    let card = compile_to_runtime_definition("Overflow", &format!("Type: Creature\nPower/Toughness: {power}/10\n{rules}"), false).unwrap();
                    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                    let a = PlayerId::from_index(0);
                    let b = PlayerId::from_index(1);
                    let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
                    assert_eq!(game.object_has_static_ability_id(source, StaticAbilityId::TrampleOverPlaneswalkers), native);
                    assert!(!game.object_has_static_ability_id(source, StaticAbilityId::Trample));
                    let walker = compile_to_runtime_definition("Defender", "Type: Planeswalker\nLoyalty: 3", false).unwrap();
                    let in_hand = game.create_object_from_definition(&walker, b, Zone::Hand);
                    let walker = enter_reminder_permanent(&mut game, in_hand);
                    assert_eq!(game.counter_count(walker, CounterType::Loyalty), 3);
                    let target = if planeswalker_present { AttackTarget::Planeswalker(walker) } else {
                        game.move_object(walker, Zone::Graveyard, ironsmith::events::cause::EventCause::effect()).unwrap();
                        AttackTarget::Nothing { defending_player: Some(b), was_planeswalker: true }
                    };
                    let blocker = game.create_object_from_definition(&definition("Blocker", "", None), b, Zone::Battlefield);
                    let mut combat = CombatState::default();
                    combat.attackers.push(AttackerInfo { creature: source, target });
                    combat.blockers.insert(source, if blocked { vec![blocker] } else { vec![] });
                    ironsmith::game_loop::execute_combat_damage_step(&mut game, &combat, false);
                    let after_blockers = (power - if blocked { 2 } else { 0 }).max(0);
                    let player_damage = (after_blockers - if planeswalker_present { 3 } else { 0 }).max(0);
                    assert_eq!(game.player(b).unwrap().life, 20 - player_damage, "{native} {power} {blocked} {planeswalker_present}");
                    if planeswalker_present {
                        assert_eq!(game.counter_count(walker, CounterType::Loyalty), (3 - after_blockers).max(0) as u32);
                    }
                }
            }
        }
    }
}

#[test]
fn reminder_identity_vehicle_payment_excludes_self_but_allows_summoning_sickness() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    use ironsmith::types::CardType;
    for native in [false, true] {
        for already_creature in [false, true] {
            let rules = if native { "Crew 2" } else { "Tap any number of creatures you control with total power 2 or more: This Vehicle becomes an artifact creature until end of turn. Creatures can't be attached to other permanents." };
            let card = compile_to_runtime_definition("Vehicle", &format!("Type: Artifact {}— Vehicle\nPower/Toughness: 5/5\n{rules}", if already_creature { "Creature " } else { "" }), false).unwrap();
            let activation = card.abilities.iter().find_map(|ability| match &ability.kind { AbilityKind::Activated(ability) => Some(ability), _ => None }).unwrap();
            assert_eq!(activation.mana_cost.non_mana_costs().any(|cost|
                cost.effect_ref().is_some_and(|effect| effect.downcast_ref::<ironsmith::effects::CrewCostEffect>().is_some_and(|cost| !cost.teamwork))), native);
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            game.turn.active_player = a;
            game.turn.priority_player = Some(a);
            let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
            let activate = |game: &GameState| compute_legal_actions(game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source));
            assert!(activate(&game).is_none(), "a creature Vehicle cannot pay with itself");
            let pilot = compile_to_runtime_definition("Pilot", "Type: Creature\nPower/Toughness: 1/1", false).unwrap();
            let first = game.create_object_from_definition(&pilot, a, Zone::Battlefield);
            assert!(activate(&game).is_none(), "one power cannot pay for two");
            let second = game.create_object_from_definition(&pilot, a, Zone::Battlefield);
            let action = activate(&game).expect("newly entered creatures may pay a tap instruction without a tap symbol");
            finish_reminder_cast(&mut game, action);
            assert!(game.is_tapped(first));
            assert!(game.is_tapped(second));
            assert!(!game.is_tapped(source));
            assert_eq!(game.current_has_card_type(source, CardType::Creature), already_creature);
            let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert!(game.current_has_card_type(source, CardType::Artifact));
            assert!(game.current_has_card_type(source, CardType::Creature));
            ironsmith::turn::execute_cleanup_step(&mut game);
            assert!(game.current_has_card_type(source, CardType::Artifact));
            assert_eq!(game.current_has_card_type(source, CardType::Creature), already_creature);
        }
    }
}

#[test]
fn reminder_identity_resource_payments_preserve_pips_and_separate_keyword_receipts() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    for (keyword, reminder, identity) in [
        ("Convoke", "Your creatures can help cast this spell. Each creature you tap while casting this spell pays for {1} or one mana of that creature's color.", StaticAbilityId::Convoke),
        ("Improvise", "Your artifacts can help cast this spell. Each artifact you tap after you're done activating mana abilities pays for {1}.", StaticAbilityId::Improvise),
        ("Delve", "Each card you exile from your graveyard while casting this spell pays for {1}.", StaticAbilityId::Delve),
    ] {
        for native in [false, true] {
            for colored_mana in [false, true] {
                let rules = if native { keyword } else { reminder };
                let card = compile_to_runtime_definition("Resource Spell", &format!("Type: Sorcery\nMana cost: {{2}}{{R}}\n{rules}\nYou gain 1 life."), false).unwrap();
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let a = PlayerId::from_index(0);
                game.turn.active_player = a;
                game.turn.priority_player = Some(a);
                game.turn.phase = ironsmith::game_state::Phase::FirstMain;
                game.turn.step = None;
                let source = game.create_object_from_definition(&card, a, Zone::Hand);
                assert_eq!(game.object_has_static_ability_id(source, identity), native);
                if colored_mana { game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Red, 1); }
                let resource = compile_to_runtime_definition("Resource", "Type: Artifact Creature\nMana cost: {R}\nPower/Toughness: 1/1", false).unwrap();
                let zone = if keyword == "Delve" { Zone::Graveyard } else { Zone::Battlefield };
                let resources = (0..3).map(|_| game.create_object_from_definition(&resource, a, zone)).collect::<Vec<_>>();
                let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == source));
                assert_eq!(action.is_some(), colored_mana || keyword == "Convoke", "{keyword} {native} must respect colored pips");
                let Some(action) = action else { continue; };
                finish_reminder_cast(&mut game, action);
                let used = if keyword == "Delve" { game.exile.len() } else { resources.iter().filter(|id| game.is_tapped(**id)).count() };
                assert_eq!(used, if colored_mana { 2 } else { 3 });
                assert_eq!(game.stack.last().unwrap().keyword_payment_contributions.len(), if native && keyword != "Delve" { used } else { 0 });
                assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
                let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                assert_eq!(game.player(a).unwrap().life, 21);
            }
        }
    }
}

#[test]
fn reminder_identity_counter_growth_preserves_each_holders_kinds_without_named_action() {
    use ironsmith::decision::DecisionMaker;
    use ironsmith::decisions::specs::ProliferateResponse;
    use ironsmith::object::CounterType;
    struct ChooseHolders { mask: u8, ordinary: bool }
    impl DecisionMaker for ChooseHolders {
        fn decide_proliferate(&mut self, _game: &GameState, ctx: &ironsmith::decisions::context::ProliferateContext) -> ProliferateResponse {
            if self.ordinary { assert!(ctx.context_text.as_deref().is_some_and(|text| !text.contains("proliferate"))); }
            ProliferateResponse {
                permanents: ctx.eligible_permanents.iter().enumerate().filter(|(index, _)| self.mask & (1 << index) != 0).map(|(_, (id, _))| *id).collect(),
                players: ctx.eligible_players.iter().filter(|_| self.mask & 4 != 0).map(|(id, _)| *id).collect(),
            }
        }
    }
    for native in [false, true] {
        let rules = if native { "Proliferate." } else { "Choose any number of permanents and/or players, then give each another counter of each kind already there." };
        let card = compile_to_runtime_definition("Counter Growth", &format!("Type: Sorcery\n{rules}"), false).unwrap();
        for mask in 0..8 {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            let b = PlayerId::from_index(1);
            let source = game.create_object_from_definition(&card, a, Zone::Stack);
            let permanent = definition("Holder", "Shroud", None);
            let first = game.create_object_from_definition(&permanent, a, Zone::Battlefield);
            let second = game.create_object_from_definition(&permanent, b, Zone::Battlefield);
            let empty = game.create_object_from_definition(&permanent, b, Zone::Battlefield);
            game.add_counters(first, CounterType::PlusOnePlusOne, 2);
            game.add_counters(first, CounterType::Charge, 3);
            game.add_counters(second, CounterType::Charge, 1);
            game.add_player_counters_with_source(b, CounterType::Poison, 2, None, None).unwrap();
            game.add_player_counters_with_source(b, CounterType::Energy, 3, None, None).unwrap();
            let mut dm = ChooseHolders { mask, ordinary: !native };
            let mut ctx = ironsmith::effects::EffectContext::new(source, a, &mut dm);
            let mut named_events = 0;
            for effect in card.spell_effect.as_ref().unwrap().all_effects() {
                let outcome = ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                named_events += outcome.events.iter().filter(|event| event.downcast::<ironsmith::events::KeywordActionEvent>().is_some_and(|event| event.action == ironsmith::events::KeywordActionKind::Proliferate)).count();
            }
            assert_eq!(named_events, usize::from(native), "zero chosen holders still completes the native action only");
            assert_eq!(game.counter_count(first, CounterType::PlusOnePlusOne), 2 + u32::from(mask & 1 != 0));
            assert_eq!(game.counter_count(first, CounterType::Charge), 3 + u32::from(mask & 1 != 0));
            assert_eq!(game.counter_count(second, CounterType::Charge), 1 + u32::from(mask & 2 != 0));
            assert_eq!(game.counter_count(second, CounterType::PlusOnePlusOne), 0, "counter kinds do not spread between holders");
            assert_eq!(game.counter_count(empty, CounterType::Charge), 0);
            assert_eq!(game.player(b).unwrap().counter_count(CounterType::Poison), 2 + u32::from(mask & 4 != 0));
            assert_eq!(game.player(b).unwrap().counter_count(CounterType::Energy), 3 + u32::from(mask & 4 != 0));
        }
    }
}

#[test]
fn reminder_identity_ordinary_counter_growth_ignores_proliferate_only_replacements() {
    use ironsmith::object::CounterType;
    for native in [false, true] {
        let rules = if native { "Proliferate." } else { "Choose any number of permanents and/or players, then give each another counter of each kind already there." };
        let card = compile_to_runtime_definition("Counter Growth", &format!("Type: Sorcery\n{rules}"), false).unwrap();
        let replacement = definition("Action Doubler", "If you would proliferate, proliferate twice instead.", None);
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&card, a, Zone::Stack);
        let holder = game.create_object_from_definition(&replacement, a, Zone::Battlefield);
        game.add_counters(holder, CounterType::Charge, 1);
        let mut ctx = ironsmith::effects::EffectContext::new_default(source, a);
        for effect in card.spell_effect.as_ref().unwrap().all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        assert_eq!(game.counter_count(holder, CounterType::Charge), if native { 3 } else { 2 });
    }
}

#[test]
fn reminder_identity_optional_counter_adjustment_checks_owner_controller_and_choice() {
    use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
    use ironsmith::decisions::context::SelectOptionsContext;
    use ironsmith::object::CounterType;
    struct CounterChoice(usize);
    impl DecisionMaker for CounterChoice {
        fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            if ctx.description.starts_with("Choose for time counter") { return vec![self.0]; }
            SelectFirstDecisionMaker.decide_options(game, ctx)
        }
    }
    for native in [false, true] {
        let rules = if native { "Time travel." } else { "For each suspended card you own and each permanent you control with a time counter on it, you may add or remove a time counter." };
        let card = compile_to_runtime_definition("Counter Adjustment", &format!("Type: Sorcery\n{rules}"), false).unwrap();
        for choice in 0..3 {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            let b = PlayerId::from_index(1);
            let source = game.create_object_from_definition(&card, a, Zone::Stack);
            let permanent = definition("Holder", "Shroud", None);
            let own = game.create_object_from_definition(&permanent, b, Zone::Battlefield);
            game.set_current_controller(own, a).unwrap();
            let theirs = game.create_object_from_definition(&permanent, a, Zone::Battlefield);
            game.set_current_controller(theirs, b).unwrap();
            let empty = game.create_object_from_definition(&permanent, a, Zone::Battlefield);
            let suspended = compile_to_runtime_definition("Suspended Card", "Type: Sorcery\nSuspend 2—{1}\nYou gain 1 life.", false).unwrap();
            let own_exile = game.create_object_from_definition(&suspended, a, Zone::Exile);
            let their_exile = game.create_object_from_definition(&suspended, b, Zone::Exile);
            let not_suspended = game.create_object_from_definition(&permanent, a, Zone::Exile);
            for id in [own, theirs, own_exile, their_exile, not_suspended] { game.add_counters(id, CounterType::Time, 2); }
            let mut dm = CounterChoice(choice);
            let mut ctx = ironsmith::effects::EffectContext::new(source, a, &mut dm);
            for effect in card.spell_effect.as_ref().unwrap().all_effects() {
                ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
            let expected = match choice { 0 => 3, 1 => 1, _ => 2 };
            assert_eq!(game.counter_count(own, CounterType::Time), expected);
            assert_eq!(game.counter_count(own_exile, CounterType::Time), expected);
            for untouched in [theirs, their_exile, not_suspended] { assert_eq!(game.counter_count(untouched, CounterType::Time), 2); }
            assert_eq!(game.counter_count(empty, CounterType::Time), 0);
        }
    }
    // The same grammar works with another counter kind and can add the first
    // counter when the authored holder filter doesn't require existing ones.
    let card = compile_to_runtime_definition("Charge Adjustment", "Type: Sorcery\nFor each artifact you control, you may add or remove a charge counter.", false).unwrap();
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let a = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&card, a, Zone::Stack);
    let artifact = compile_to_runtime_definition("Empty Holder", "Type: Artifact", false).unwrap();
    let holder = game.create_object_from_definition(&artifact, a, Zone::Battlefield);
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, a);
    for effect in card.spell_effect.as_ref().unwrap().all_effects() {
        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    assert_eq!(game.counter_count(holder, CounterType::Charge), 1);
}

#[test]
fn reminder_identity_dynamic_combat_mana_reads_current_source_power() {
    use ironsmith::object::CounterType;
    use ironsmith::mana::ManaSymbol;
    use ironsmith::game_state::{Phase, Step};
    for native in [false, true] {
        for initial_power in [0, 3] {
            for extra in [0, 2] {
                let rules = if native { "Firebending X, where X is this creature's power." } else { "Whenever this creature attacks, add X {R}, where X is this creature's power. This mana lasts until end of combat." };
                let card = compile_to_runtime_definition("Dynamic Combat Mana", &format!("Type: Creature\nPower/Toughness: {initial_power}/5\n{rules}"), false).unwrap();
                let trigger = card.abilities.iter().find_map(|ability| match &ability.kind { AbilityKind::Triggered(trigger) => Some(trigger.clone()), _ => None }).unwrap();
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let a = PlayerId::from_index(0);
                game.turn.active_player = a;
                game.turn.phase = Phase::Combat;
                game.turn.step = Some(Step::DeclareAttackers);
                let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
                game.push_to_stack(ironsmith::StackEntry::ability(source, a, trigger.effects.clone()));
                game.add_counters(source, CounterType::PlusOnePlusOne, extra);
                game.player_mut(a).unwrap().mana_pool.add(ManaSymbol::Red, 1);
                let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                let expected = initial_power + extra;
                assert_eq!(game.player(a).unwrap().mana_pool.total(), expected + 1);
                game.empty_mana_pools().unwrap();
                assert_eq!(game.player(a).unwrap().mana_pool.total(), expected);
                game.turn.step = Some(Step::EndCombat);
                game.empty_mana_pools().unwrap();
                assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
            }
        }
    }
}

#[test]
fn reminder_identity_blight_definition_chooses_own_creature_without_native_action() {
    use ironsmith::object::CounterType;
    for native in [false, true] {
        for available in [false, true] {
            let rules = if native { "Blight 2." } else { "To blight 2, put two -1/-1 counters on a creature you control." };
            let card = compile_to_runtime_definition("Counter Payment", &format!("Type: Sorcery\n{rules}"), false).unwrap();
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            let b = PlayerId::from_index(1);
            let source = game.create_object_from_definition(&card, a, Zone::Stack);
            let creature = compile_to_runtime_definition("Holder", "Type: Creature\nPower/Toughness: 4/4\nShroud", false).unwrap();
            let opponent = game.create_object_from_definition(&creature, b, Zone::Battlefield);
            let own = available.then(|| game.create_object_from_definition(&creature, a, Zone::Battlefield));
            let mut ctx = ironsmith::effects::EffectContext::new_default(source, a);
            let mut native_events = 0;
            for effect in card.spell_effect.as_ref().unwrap().all_effects() {
                let outcome = ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                native_events += outcome.events.iter().filter(|event| event.downcast::<ironsmith::events::KeywordActionEvent>().is_some_and(|event| event.action == ironsmith::events::KeywordActionKind::Blight)).count();
            }
            assert_eq!(game.counter_count(opponent, CounterType::MinusOneMinusOne), 0);
            if let Some(own) = own {
                assert_eq!(game.counter_count(own, CounterType::MinusOneMinusOne), 2);
                assert_eq!(native_events, usize::from(native));
            } else if !native { assert_eq!(native_events, 0); }
        }
    }
}

#[test]
fn reminder_identity_optional_forced_block_names_the_attacker_and_untaps_only_on_acceptance() {
    use ironsmith::decision::DecisionMaker;
    use ironsmith::decisions::context::BooleanContext;
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
    struct Accept(bool);
    impl DecisionMaker for Accept {
        fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool { self.0 }
    }
    for form in 0..3 {
        let native = form != 0;
        for accept in [false, true] {
            for tapped in [false, true] {
                let rules = if native { "Provoke" } else { "Whenever this creature attacks, you may have target creature defending player controls untap and block it if able." };
                let mut card = definition("Required Attacker", rules, (!native).then_some("Challenge"));
                if form == 2 { card.abilities = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::ids::CardId::new(), "Required Attacker").provoke().build().abilities; }
                let trigger = card.abilities.iter().find_map(|ability| match &ability.kind { AbilityKind::Triggered(trigger) => Some(trigger.clone()), _ => None }).unwrap();
                assert_eq!(trigger.presentation_label.is_some(), native);
                assert_eq!(trigger.choices.len(), 1);
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let a = PlayerId::from_index(0);
                let b = PlayerId::from_index(1);
                let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
                let plain = definition("Other", "", None);
                let other = game.create_object_from_definition(&plain, a, Zone::Battlefield);
                let blocker = game.create_object_from_definition(&plain, b, Zone::Battlefield);
                if tapped { game.tap(blocker); }
                let mut combat = CombatState::default();
                for creature in [source, other] { combat.attackers.push(AttackerInfo { creature, target: AttackTarget::Player(b) }); }
                game.combat = Some(combat.clone());
                let mut dm = Accept(accept);
                let mut ctx = ironsmith::effects::EffectContext::new(source, a, &mut dm)
                    .with_triggering_event(ironsmith::triggers::TriggerEvent::new_with_provenance(
                        ironsmith::events::CreatureAttackedEvent::new(source, ironsmith::events::AttackEventTarget::Player(b)), Default::default()))
                    .with_defending_player(b)
                    .with_targets(vec![ironsmith::effects::ResolvedTarget::Object(blocker)]);
                for effect in trigger.effects.all_effects() { ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
                game.refresh_continuous_state().unwrap();
                assert_eq!(game.is_tapped(blocker), tapped && !accept);
                assert_eq!(game.must_block_attacker(blocker, source), accept);
                assert!(!game.must_block_attacker(blocker, other));
                if accept {
                    assert!(ironsmith::combat_state::declare_blockers(&game, &mut combat.clone(), vec![(blocker, other)]).is_err());
                    assert!(ironsmith::combat_state::declare_blockers(&game, &mut combat.clone(), vec![(blocker, source)]).is_ok());
                }
                game.cleanup_effects_end_of_combat();
                game.refresh_continuous_state().unwrap();
                assert!(!game.must_block_attacker(blocker, source));
            }
        }
    }
}

#[test]
fn reminder_identity_counter_designations_are_atomic_once_and_preserve_state_events() {
    use ironsmith::object::CounterType;
    use ironsmith::events::other::{KeywordActionEvent, KeywordActionKind};
    for renowned in [false, true] {
        for native in [false, true] {
            // Replacement can double counters, prevent them, or remove the source.
            for replacement_kind in 0..4 {
                let rules = match (renowned, native) {
                    (true, true) => "Renown 2",
                    (true, false) => "Whenever this creature deals combat damage to a player, if it isn't renowned, put two +1/+1 counters on it and it becomes renowned.",
                    (false, true) => "{0}: Monstrosity 2.",
                    (false, false) => "{0}: If this creature isn't monstrous, put two +1/+1 counters on it and it becomes monstrous. {S} can be paid with one mana from a snow source.",
                };
                let card = definition("Stateful", rules, (!native).then_some("Ascension"));
                let effects: Vec<_> = card.abilities.iter().flat_map(|a| match &a.kind {
                    AbilityKind::Triggered(a) => a.effects.all_effects().into_iter().cloned().collect::<Vec<_>>(),
                    AbilityKind::Activated(a) => a.effects.all_effects().into_iter().cloned().collect::<Vec<_>>(),
                    _ => vec![],
                }).collect();
                assert!(!effects.is_empty());
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let a = PlayerId::from_index(0);
                let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
                if replacement_kind != 0 {
                    let mut replacement = StaticAbility::double_counters_replacement(
                        ironsmith::target::ObjectFilter::creature(), Some(CounterType::PlusOnePlusOne), "Counter replacement".into(),
                    ).generate_replacement_effect(source, a).unwrap();
                    if replacement_kind == 2 {
                        replacement.replacement = ironsmith::replacement::ReplacementAction::Instead(vec![]);
                    } else if replacement_kind == 3 {
                        replacement.replacement = ironsmith::replacement::ReplacementAction::Instead(vec![ironsmith::effect::Effect::destroy(ironsmith::target::ChooseSpec::Source)]);
                    }
                    game.effect_store.replacement_effects.add_resolution_effect(replacement);
                }
                let mut events = Vec::new();
                for _ in 0..2 {
                    let mut ctx = ironsmith::effects::EffectContext::new_default(source, a);
                    for effect in &effects { events.extend(ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap().events); }
                }
                let present = replacement_kind != 3;
                assert_eq!(game.is_renowned(source), renowned && present);
                assert_eq!(game.is_monstrous(source), !renowned && present);
                if present {
                    let expected = match replacement_kind { 1 => 4, 2 => 0, _ => 2 };
                    assert_eq!(game.object(source).unwrap().counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0), expected);
                } else { assert!(game.object(source).is_none()); }
                let completions = events.iter().filter(|event| {
                    if renowned {
                        event.downcast::<KeywordActionEvent>().is_some_and(|event| event.action == KeywordActionKind::Renown)
                    } else { event.downcast::<ironsmith::events::BecameMonstrousEvent>().is_some() }
                }).count();
                // These are explicit state-change triggers, even for the custom name.
                assert_eq!(completions, usize::from(present));
            }
        }
    }
}

#[test]
fn reminder_identity_destruction_recovery_is_one_shot_and_has_no_regeneration_identity() {
    use ironsmith::effect::Effect;
    use ironsmith::target::ChooseSpec;
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
    for native in [false, true] {
        for expires in [false, true] {
            for forbids_regeneration in [false, true] {
                let rules = if native { "{0}: Regenerate this creature." } else { "{0}: The next time this creature would be destroyed this turn, instead tap it, remove it from combat, and heal all damage on it." };
                let card = definition("Recovery", rules, (!native).then_some("Reconstitute"));
                let ability = card.abilities.iter().find_map(|a| match &a.kind { AbilityKind::Activated(a) => Some(a), _ => None }).unwrap();
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let a = PlayerId::from_index(0);
                let b = PlayerId::from_index(1);
                let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
                game.mark_damage(source, 1);
                let mut combat = CombatState::default();
                combat.attackers.push(AttackerInfo { creature: source, target: AttackTarget::Player(b) });
                game.combat = Some(combat);
                {
                    let mut ctx = ironsmith::effects::EffectContext::new_default(source, a);
                    for effect in ability.effects.all_effects() { ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
                }
                // Creating the replacement doesn't immediately tap or heal anything.
                assert!(!game.is_tapped(source));
                assert_eq!(game.damage_on(source), 1);
                if expires { ironsmith::turn::execute_cleanup_step(&mut game); }
                let destroy = if forbids_regeneration {
                    Effect::new(ironsmith::effects::DestroyNoRegenerationEffect::with_spec(ChooseSpec::SpecificObject(source)))
                } else { Effect::destroy(ChooseSpec::SpecificObject(source)) };
                let mut ctx = ironsmith::effects::EffectContext::new_default(source, a);
                ironsmith::effects::execute_effect(&mut game, &destroy, &mut ctx).unwrap();
                let survives = !expires && !(native && forbids_regeneration);
                assert_eq!(game.object(source).is_some(), survives);
                if survives {
                    assert!(game.is_tapped(source));
                    assert_eq!(game.damage_on(source), 0);
                    assert!(game.combat.as_ref().is_none_or(|combat| !combat.attackers.iter().any(|a| a.creature == source)));
                    ironsmith::effects::execute_effect(&mut game, &Effect::destroy(ChooseSpec::SpecificObject(source)), &mut ctx).unwrap();
                    assert!(game.object(source).is_none(), "the replacement must only apply once");
                }
            }
        }
    }
}

#[test]
fn reminder_identity_entry_x_counters_and_conditional_draw_share_cast_value() {
    use ironsmith::object::CounterType;
    use ironsmith::triggers::TriggerContext;
    for native in [false, true] {
        for x in [0, 4, 5, 8] {
            let rules = if native { "Ravenous" } else { "This creature enters with X +1/+1 counters on it. If X is 5 or more, draw a card when it enters." };
            let card = definition("Hungry", rules, (!native).then_some("Appetite"));
            let triggered = card.abilities.iter().find_map(|a| match &a.kind { AbilityKind::Triggered(a) => Some(a), _ => None }).unwrap();
            assert_eq!(triggered.trigger.intro_surface().is_some(), !native);
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            let on_stack = game.create_object_from_definition(&card, a, Zone::Stack);
            game.object_mut(on_stack).unwrap().x_value = Some(x);
            let draw_card = game.create_object_from_definition(&definition("Draw", "", None), a, Zone::Library);
            let source = enter_reminder_permanent(&mut game, on_stack);
            assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), x);
            let entries = drain_reminder_triggers(&mut game);
            assert_eq!(entries.len(), usize::from(x >= 5), "native={native} x={x}");
            let mut ctx = ironsmith::effects::EffectContext::new_default(source, a).with_x(x);
            let condition = triggered.intervening_if.as_ref().expect("X threshold must be checked at triggering and resolution");
            assert_eq!(ironsmith::condition_eval::evaluate_condition_resolution(&game, condition, &ctx).unwrap(), x >= 5);
            if x >= 5 {
                for effect in triggered.effects.all_effects() { ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
                assert!(game.player(a).unwrap().hand.iter().any(|id| game.object(*id).unwrap().name == "Draw"));
            } else { assert!(game.player(a).unwrap().library.contains(&draw_card)); }
        }
    }
}

#[test]
fn reminder_identity_exclusive_graveyard_permission_restricts_zone_and_all_departures() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    for native in [false, true] {
        for departure in ["resolve", "counter", "bounce"] {
            let rules = if native { "Aftermath" } else { "Cast this spell only from your graveyard. Then exile it." };
            let mut card = compile_to_runtime_definition("Exclusive", &format!("Type: Sorcery\nMana cost: {{3}}\n{rules}\nYou gain 1 life."), false).unwrap();
            assert_eq!(card.alternative_casts[0].name() == "Aftermath", native);
            // Use a different permission's unmarked price: the printed stack
            // rider must apply even without the chosen price's exile flag.
            if departure == "bounce" {
                let ironsmith::alternative_cast::AlternativeCastingMethod::FromZone { instructions, .. } = &mut card.alternative_casts[0] else { panic!("zone price") };
                instructions.exile_on_stack_exit = false;
            }
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            game.turn.active_player = a;
            game.turn.priority_player = Some(a);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.turn.step = None;
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 2);
            let source = game.create_object_from_definition(&card, a, Zone::Graveyard);
            let hand_copy = game.create_object_from_definition(&card, a, Zone::Hand);
            let find = |game: &GameState, id| compute_legal_actions(game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == id));
            assert!(find(&game, source).is_none());
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 1);
            assert!(find(&game, hand_copy).is_none(), "exclusive zone must forbid the ordinary hand cast");
            let action = find(&game, source).unwrap();
            finish_reminder_cast(&mut game, action);
            assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
            let spell = game.stack[0].object_id;
            if departure == "resolve" {
                let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                assert_eq!(game.player(a).unwrap().life, 21);
            } else {
                let effect = if departure == "counter" {
                    ironsmith::effect::Effect::counter(ironsmith::target::ChooseSpec::SpecificObject(spell))
                } else {
                    ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(spell), Zone::Hand, false))
                };
                let mut ctx = ironsmith::effects::EffectContext::new_default(spell, a);
                ironsmith::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
            }
            assert!(game.exile.iter().any(|id| game.object(*id).unwrap().name == "Exclusive"));
            assert_eq!(game.player(a).unwrap().hand.len(), 1);
        }
    }
}

#[test]
fn reminder_identity_outside_card_or_rummage_preserves_choices_and_custom_filter() {
    use ironsmith::decision::DecisionMaker;
    use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext};
    struct Choice(u8);
    impl DecisionMaker for Choice {
        fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool { self.0 == 0 }
        fn decide_objects(&mut self, _game: &GameState, ctx: &SelectObjectsContext) -> Vec<ironsmith::ids::ObjectId> {
            if self.0 == 2 { return vec![]; }
            ctx.candidates.iter().find(|candidate| candidate.legal).map(|candidate| vec![candidate.id]).unwrap_or_default()
        }
    }
    for form in 0..3 {
        for choice in 0..3 {
            for available in [false, true] {
                let rules = match form {
                    0 => "Learn.",
                    1 => "You may reveal a Lesson card you own from outside the game and put it into your hand, or discard a card to draw a card.",
                    _ => "You may reveal an artifact card you own from outside the game and put it into your hand, or discard a card to draw a card.",
                };
                let card = compile_to_runtime_definition("Study", &format!("Type: Sorcery\n{rules}"), false).unwrap();
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let a = PlayerId::from_index(0);
                let b = PlayerId::from_index(1);
                let source = game.create_object_from_definition(&card, a, Zone::Stack);
                let plain = definition("Fuel", "", None);
                game.create_object_from_definition(&plain, a, Zone::Hand);
                game.create_object_from_definition(&definition("Draw", "", None), a, Zone::Library);
                let candidate = compile_to_runtime_definition("Outside", if form == 2 { "Type: Artifact" } else { "Type: Sorcery — Lesson" }, false).unwrap();
                let opponent = game.create_object_from_definition(&candidate, b, Zone::OutsideGame);
                game.create_object_from_definition(&plain, a, Zone::OutsideGame);
                if available { game.create_object_from_definition(&candidate, a, Zone::OutsideGame); }
                let mut dm = Choice(choice);
                let mut ctx = ironsmith::effects::EffectContext::new(source, a, &mut dm);
                let mut named_events = 0;
                for effect in card.spell_effect.as_ref().unwrap().all_effects() {
                    let result = ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                    named_events += result.events.iter().filter(|event| event.downcast::<ironsmith::events::KeywordActionEvent>().is_some_and(|event| event.action == ironsmith::events::KeywordActionKind::Learn)).count();
                }
                assert_eq!(named_events, usize::from(form == 0));
                assert_eq!(game.object(opponent).unwrap().zone, Zone::OutsideGame);
                let hand: Vec<_> = game.player(a).unwrap().hand.iter().map(|id| game.object(*id).unwrap().name.as_str()).collect();
                if choice == 0 {
                    assert_eq!(hand, vec!["Draw"]);
                    assert_eq!(game.player(a).unwrap().graveyard.len(), 1);
                } else {
                    assert!(hand.contains(&"Fuel"));
                    assert_eq!(hand.contains(&"Outside"), choice == 1 && available);
                    assert!(game.player(a).unwrap().graveyard.is_empty());
                }
            }
        }
    }
}

#[test]
fn reminder_identity_aggregate_graveyard_exile_is_atomic_and_keeps_custom_identity() {
    use ironsmith::decision::DecisionMaker;
    use ironsmith::decisions::context::SelectObjectsContext;
    struct Choose(Vec<ironsmith::ids::ObjectId>);
    impl DecisionMaker for Choose {
        fn decide_objects(&mut self, _game: &GameState, _ctx: &SelectObjectsContext) -> Vec<ironsmith::ids::ObjectId> { self.0.clone() }
    }
    for native in [false, true] {
        for (required, indices, valid) in [
            (10, vec![0, 1, 2], true), (4, vec![2], true),
            (4, vec![0, 1], true), (0, vec![], true),
            (11, vec![], true), (10, vec![0], true),
        ] {
            let rules = if native { format!("Collect evidence {required}.") } else { format!("To assemble {required}, exile cards with total mana value {required} or greater from your graveyard.") };
            let card = compile_to_runtime_definition("Payment", &format!("Type: Sorcery\n{rules}"), false).unwrap();
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            let b = PlayerId::from_index(1);
            let source = game.create_object_from_definition(&card, a, Zone::Stack);
            let mut cards = Vec::new();
            for value in [2, 3, 5] {
                let card = compile_to_runtime_definition(&format!("Value {value}"), &format!("Type: Sorcery\nMana cost: {{{value}}}"), false).unwrap();
                cards.push(game.create_object_from_definition(&card, a, Zone::Graveyard));
                game.create_object_from_definition(&card, b, Zone::Graveyard);
            }
            let selected = indices.iter().map(|index| cards[*index]).collect();
            let mut dm = Choose(selected);
            let mut ctx = ironsmith::effects::EffectContext::new(source, a, &mut dm);
            let mut events = 0;
            let mut error = false;
            for effect in card.spell_effect.as_ref().unwrap().all_effects() {
                match ironsmith::effects::execute_effect(&mut game, effect, &mut ctx) {
                    Ok(result) => events += result.events.iter().filter(|event| event.downcast::<ironsmith::events::KeywordActionEvent>().is_some_and(|event| event.action == ironsmith::events::KeywordActionKind::CollectEvidence)).count(),
                    Err(_) => { error = true; break; }
                }
            }
            assert_eq!(error, !valid, "native={native} required={required} indices={indices:?} exile={:?}", game.exile);
            let paid = valid && required <= 10;
            // Invalid chooser answers are normalized to a complete legal payment.
            let chosen_count = if required == 10 && indices == [0] { 3 } else { indices.len() };
            assert_eq!(game.exile.len(), if paid { chosen_count } else { 0 });
            assert_eq!(game.player(b).unwrap().graveyard.len(), 3);
            assert_eq!(events, usize::from(native && paid));
        }
    }
}

#[test]
fn reminder_identity_cast_copy_counts_only_earlier_spells_and_creates_permanent_tokens() {
    use ironsmith::decision::{compute_legal_actions, LegalAction, SelectFirstDecisionMaker};
    for native in [false, true] {
        for prior in [0, 3] {
            let rules = if native { "Storm" } else { "When you cast this spell, copy it for each spell cast before it this turn. You may choose new targets for the copies. Copies become tokens." };
            let card = compile_to_runtime_definition("Repeated Creature", &format!("Type: Creature\nMana cost: {{0}}\nPower/Toughness: 2/2\n{rules}"), false).unwrap();
            let triggered = card.abilities.iter().find_map(|a| match &a.kind { AbilityKind::Triggered(a) => Some(a), _ => None }).unwrap();
            assert_eq!(triggered.presentation_label.is_some(), native);
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            let b = PlayerId::from_index(1);
            game.turn.active_player = a;
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.turn.step = None;
            let filler = compile_to_runtime_definition("History", "Type: Instant\nMana cost: {0}\nYou gain 1 life.", false).unwrap();
            for i in 0..prior {
                let player = if i % 2 == 0 { b } else { a };
                game.turn.priority_player = Some(player);
                let id = game.create_object_from_definition(&filler, player, Zone::Hand);
                let action = compute_legal_actions(&game, player).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == id)).unwrap();
                finish_reminder_cast(&mut game, action);
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
            }
            game.turn.priority_player = Some(a);
            let id = game.create_object_from_definition(&card, a, Zone::Hand);
            let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == id)).unwrap();
            let mut cast_triggers = finish_reminder_cast_with(&mut game, action, &mut SelectFirstDecisionMaker);
            ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut cast_triggers, &mut SelectFirstDecisionMaker).unwrap();
            let source = game.stack.iter().find(|entry| game.object(entry.object_id).is_some_and(|object| object.name == "Repeated Creature")).unwrap().object_id;
            // A spell cast in response must not increase the captured-before count.
            game.turn.priority_player = Some(b);
            let late = game.create_object_from_definition(&filler, b, Zone::Hand);
            let action = compute_legal_actions(&game, b).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == late)).unwrap();
            finish_reminder_cast(&mut game, action);
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
            // Exercise the compiled trigger once even if the casting harness
            // already put it on the stack while finishing the spell proposal.
            let event = cast_triggers.entries.first().map(|entry| entry.triggering_event.clone())
                .or_else(|| game.stack.iter().find_map(|entry| entry.triggering_event.clone())).expect("cast trigger");
            game.stack.retain(|entry| entry.ability_effects.is_none());
            let before = game.stack.len();
            let mut ctx = ironsmith::effects::EffectContext::new_default(source, a).with_triggering_event(event);
            for effect in triggered.effects.all_effects() { ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
            assert_eq!(game.stack.len(), before + prior);
            while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap(); }
            let creatures: Vec<_> = game.battlefield.iter().filter_map(|id| game.object(*id)).filter(|object| object.name == "Repeated Creature").collect();
            assert_eq!(creatures.len(), prior + 1);
            assert_eq!(creatures.iter().filter(|object| object.kind == ironsmith::object::ObjectKind::Token).count(), prior);
        }
    }
}

#[test]
fn reminder_identity_optional_entry_copies_count_payments_not_exiled_cards() {
    use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker, LegalAction, compute_legal_actions};
    use ironsmith::decisions::context::SelectOptionsContext;
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm};
    struct Payments(usize);
    impl DecisionMaker for Payments {
        fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            if ctx.description.starts_with("Choose optional costs") {
                return ctx.options.iter().find(|option| option.legal).map(|option| vec![option.index; self.0]).unwrap_or_default();
            }
            SelectFirstDecisionMaker.decide_options(game, ctx)
        }
    }
    for offspring in [false, true] {
        for native in [false, true] {
            for payments in 0..=if offspring { 1 } else { 2 } {
                let rules = match (offspring, native) {
                    (false, true) => "Squad—Exile four cards from your graveyard.",
                    (false, false) => "As an additional cost to cast this spell, you may exile four cards from your graveyard any number of times. When this creature enters, create that many tokens that are copies of it.",
                    (true, true) => "Offspring {2}{B}",
                    (true, false) => "You may pay an additional {2}{B} as you cast this spell. If you do, when this creature enters, create a 1/1 token copy of it.",
                };
                let card = compile_to_runtime_definition("Parent", &format!("Type: Creature\nMana cost: {{0}}\nPower/Toughness: 3/3\n{rules}"), false).unwrap();
                let trigger = card.abilities.iter().find_map(|a| match &a.kind { AbilityKind::Triggered(t) => Some(t.clone()), _ => None }).unwrap();
                let label = if offspring { "Offspring" } else { "Squad" };
                let reference = card.optional_costs[0].reference.clone();
                // Printed instances carry a per-line discriminator; identity is the kind.
                assert_eq!(reference.kind == ironsmith::cost::OptionalCostRef::from(label).kind, native, "offspring={offspring} native={native} costs={:?}", card.optional_costs);
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let a = PlayerId::from_index(0);
                game.turn.active_player = a;
                game.turn.priority_player = Some(a);
                game.turn.phase = ironsmith::game_state::Phase::FirstMain;
                game.turn.step = None;
                game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Black, 3);
                let source = game.create_object_from_definition(&card, a, Zone::Hand);
                for _ in 0..8 { game.create_object_from_definition(&definition("Fuel", "", None), a, Zone::Graveyard); }
                let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == source)).unwrap();
                let mut state = PriorityLoopState::new(2);
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                let mut dm = Payments(payments);
                let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
                for _ in 0..64 {
                    if !state.has_pending_action() { break; }
                    let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}") };
                    progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
                }
                assert!(!state.has_pending_action());
                assert_eq!(game.exile.len(), if offspring { 0 } else { payments * 4 });
                assert_eq!(game.player(a).unwrap().mana_pool.total(), if offspring { 3 - payments as u32 * 3 } else { 3 });
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
                let parent = *game.battlefield.iter().find(|id| game.object(**id).is_some_and(|object| object.name == "Parent")).unwrap();
                let paid = game.object(parent).unwrap().optional_costs_paid.clone();
                assert_eq!(paid.times_paid_label(reference.clone()), payments as u32);
                assert_eq!(paid.times_paid_label(label), if native { payments as u32 } else { 0 });
                let captured = game.stack.iter().find_map(|entry| entry.triggering_event.clone())
                    .or_else(|| drain_reminder_triggers(&mut game).first().map(|entry| entry.triggering_event.clone()));
                let mut ctx = ironsmith::effects::EffectContext::new_default(parent, a).with_optional_costs_paid(paid);
                if let Some(event) = captured { ctx = ctx.with_triggering_event(event); }
                let allowed = trigger.intervening_if.as_ref().map(|condition| ironsmith::condition_eval::evaluate_condition_resolution(&game, condition, &ctx).unwrap()).unwrap_or(true);
                if allowed {
                    for effect in trigger.effects.all_effects() { ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
                }
                let copies: Vec<_> = game.battlefield.iter().filter_map(|id| game.object(*id)).filter(|object| object.kind == ironsmith::object::ObjectKind::Token).collect();
                assert_eq!(copies.len(), payments);
                for copy in copies {
                    assert_eq!(copy.power(), Some(if offspring { 1 } else { 3 }));
                    assert_eq!(copy.toughness(), Some(if offspring { 1 } else { 3 }));
                    assert_eq!(copy.optional_costs_paid.times_paid_label(reference.clone()), 0);
                }
            }
        }
    }
}

#[test]
fn reminder_identity_repeated_copy_payment_shares_behavior_without_replicate_receipt() {
    use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker, LegalAction, compute_legal_actions};
    use ironsmith::decisions::context::SelectOptionsContext;
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm};
    struct Payments(usize);
    impl DecisionMaker for Payments {
        fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            if ctx.description.starts_with("Choose optional costs") {
                return ctx.options.iter().find(|option| option.legal).map(|option| vec![option.index; self.0]).unwrap_or_default();
            }
            SelectFirstDecisionMaker.decide_options(game, ctx)
        }
    }
    for native in [false, true] {
        for nonmana in [false, true] {
            for payments in 0..=2 {
                let price = if nonmana { "Discard a card" } else { "{2}" };
                let rules = if native { format!("Replicate—{price}") } else {
                    let payment = if nonmana { price.to_owned() } else { format!("pay {price}") };
                    format!("As an additional cost to cast this spell, you may {payment} any number of times. When you cast this spell, copy it for each time this spell's additional cost was paid. You may choose new targets for the copies. Copies become tokens.")
                };
                let card = compile_to_runtime_definition("Echoing", &format!("Type: Creature\nMana cost: {{0}}\nPower/Toughness: 2/2\n{rules}"), false).unwrap();
                let reference = card.optional_costs[0].reference.clone();
                assert_eq!(reference == ironsmith::cost::OptionalCostRef::from("Replicate"), native);
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let a = PlayerId::from_index(0);
                game.turn.active_player = a;
                game.turn.priority_player = Some(a);
                game.turn.phase = ironsmith::game_state::Phase::FirstMain;
                game.turn.step = None;
                game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 4);
                let source = game.create_object_from_definition(&card, a, Zone::Hand);
                for _ in 0..2 { game.create_object_from_definition(&definition("Fuel", "", None), a, Zone::Hand); }
                let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == source)).unwrap();
                let mut state = PriorityLoopState::new(2);
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                let mut dm = Payments(payments);
                let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
                for _ in 0..64 {
                    if !state.has_pending_action() { break; }
                    let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}") };
                    progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
                }
                assert!(!state.has_pending_action());
                assert_eq!(game.player(a).unwrap().graveyard.len(), if nonmana { payments } else { 0 });
                assert_eq!(game.player(a).unwrap().mana_pool.total(), if nonmana { 4 } else { 4 - payments as u32 * 2 });
                let source = game.stack.iter().find(|entry| entry.ability_effects.is_none()).unwrap().object_id;
                let paid = game.object(source).unwrap().optional_costs_paid.clone();
                assert_eq!(paid.times_paid_label(reference), payments as u32);
                assert_eq!(paid.times_paid_label("Replicate"), if native { payments as u32 } else { 0 });
                ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
                ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                if game.stack.last().is_some_and(|entry| entry.ability_effects.is_some()) {
                    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
                }
                assert_eq!(game.stack.len(), payments + 1);
                while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap(); }
                assert_eq!(game.battlefield.len(), payments + 1);
            }
        }
    }
}

#[test]
fn reminder_identity_counter_upkeep_payment_is_atomic_and_not_native_identity() {
    use ironsmith::decision::DecisionMaker;
    use ironsmith::decisions::context::BooleanContext;
    use ironsmith::object::CounterType;
    struct Accept(bool);
    impl DecisionMaker for Accept {
        fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool { self.0 }
    }
    for form in 0..3 {
        let native = form == 0;
        let counter = if form == 2 { CounterType::Charge } else { CounterType::Age };
        let counter_name = if form == 2 { "charge" } else { "age" };
        for nonmana in [false, true] {
            let price = if nonmana { "Pay 3 life" } else { "{2}" };
            let payment = if nonmana { "pay 3 life" } else { "pay {2}" };
            let rules = if native { format!("Cumulative upkeep—{price}") } else {
                format!("At the beginning of your upkeep, put a {counter_name} counter on this permanent, then sacrifice it unless you {payment} for each {counter_name} counter on it.")
            };
            let card = definition("Growing Cost", &rules, (!native).then_some("Burden"));
            let trigger = card.abilities.iter().find_map(|a| match &a.kind { AbilityKind::Triggered(t) => Some(t), _ => None }).unwrap();
            for initial in [0, 2] {
                for accept in [false, true] {
                    for affordable in [false, true] {
                        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                        let a = PlayerId::from_index(0);
                        let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
                        game.add_counters(source, counter, initial);
                        let required = (initial + 1) * if nonmana { 3 } else { 2 };
                        let resources = if affordable { required + 1 } else { required - 1 };
                        if nonmana { game.player_mut(a).unwrap().life = resources as i32; }
                        else { game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, resources); }
                        let mut dm = Accept(accept);
                        let mut ctx = ironsmith::effects::EffectContext::new(source, a, &mut dm);
                        let mut receipts = Vec::new();
                        for effect in trigger.effects.all_effects() {
                            let outcome = ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                            receipts.extend(outcome.events.into_iter().filter_map(|e| e.downcast::<ironsmith::events::KeywordActionEvent>().map(|e| e.action)));
                        }
                        let paid = affordable && accept;
                        assert_eq!(game.battlefield.contains(&source), paid);
                        if paid { assert_eq!(game.counter_count(source, counter), initial + 1); }
                        let remaining = if nonmana { game.player(a).unwrap().life as u32 } else { game.player(a).unwrap().mana_pool.total() };
                        assert_eq!(remaining, resources - if paid { required } else { 0 }, "failed repeated payments must not spend a partial cost");
                        let expected = if paid { ironsmith::events::KeywordActionKind::CumulativeUpkeepPaid } else { ironsmith::events::KeywordActionKind::CumulativeUpkeepNotPaid };
                        assert_eq!(receipts.iter().filter(|r| **r == expected).count(), usize::from(native));
                    }
                }
            }
        }
    }
}

#[test]
fn reminder_identity_player_designation_is_continuous_and_persists_after_source_leaves() {
    for story in [false, true] {
        for native in [false, true] {
            for source_phased in [false, true] {
                let threshold = if story { 3 } else { 10 };
                let rules = match (story, native) {
                    (false, true) => "Ascend",
                    (true, true) => "Storied",
                    (false, false) => "If you control ten or more permanents, you get the city's blessing for the rest of the game.",
                    (true, false) => "If you control three or more artifacts, legendaries, and/or Sagas, you have an enduring story for the rest of the game.",
                };
                let card = definition("Chronicler", rules, (!native).then_some("Chronicle"));
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let a = PlayerId::from_index(0);
                let b = PlayerId::from_index(1);
                let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
                if source_phased { game.phase_out(source); }
                let filler = compile_to_runtime_definition("Historic", "Type: Legendary Artifact — Saga\nMana cost: {0}", false).unwrap();
                // A permanent satisfying all three historic categories counts once.
                let before_threshold = threshold - if story || source_phased { 1 } else { 2 };
                for _ in 0..before_threshold { game.create_object_from_definition(&filler, a, Zone::Battlefield); }
                for _ in 0..10 { game.create_object_from_definition(&filler, b, Zone::Battlefield); }
                game.refresh_continuous_state().unwrap();
                let has = |game: &GameState| if story { game.has_enduring_story(a) } else { game.has_citys_blessing(a) };
                assert!(!has(&game));
                game.create_object_from_definition(&filler, a, Zone::Battlefield);
                game.refresh_continuous_state().unwrap();
                assert_eq!(has(&game), !source_phased);
                assert!(game.stack.is_empty(), "continuous designations do not use the stack");
                let identity = if story { StaticAbilityId::Storied } else { StaticAbilityId::Ascend };
                assert_eq!(game.object(source).unwrap().has_static_ability_id(identity), native);
                if !source_phased {
                    for id in game.battlefield.clone() { game.move_object(id, Zone::Graveyard, ironsmith::events::EventCause::effect()); }
                    game.refresh_continuous_state().unwrap();
                    assert!(has(&game));
                }
            }
        }
    }
}

#[test]
fn reminder_identity_spell_designation_checks_only_on_resolution_and_accepts_custom_thresholds() {
    for native in [false, true] {
        for enough in [false, true] {
            let minimum = if native { 10 } else { 4 };
            let rules = if native { "Ascend".to_owned() } else { format!("If you control {minimum} or more permanents, you get the city's blessing for the rest of the game.") };
            let card = compile_to_runtime_definition("Blessing", &format!("Type: Sorcery\nMana cost: {{0}}\n{rules}"), false).unwrap();
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            let source = game.create_object_from_definition(&card, a, Zone::Stack);
            let filler = definition("Permanent", "", None);
            for _ in 0..minimum - usize::from(!enough) { game.create_object_from_definition(&filler, a, Zone::Battlefield); }
            game.refresh_continuous_state().unwrap();
            assert!(!game.has_citys_blessing(a));
            let mut ctx = ironsmith::effects::EffectContext::new_default(source, a);
            for effect in card.spell_effect.as_ref().unwrap().all_effects() { ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
            assert_eq!(game.has_citys_blessing(a), enough);
        }
    }
}

#[test]
fn reminder_identity_entry_choice_happens_before_entry_and_keeps_only_authored_identity() {
    use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker, LegalAction, compute_legal_actions};
    use ironsmith::decisions::context::SelectOptionsContext;
    struct Mode(usize);
    impl DecisionMaker for Mode {
        fn decide_options(&mut self, _: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            ctx.options.iter().filter(|option| option.legal).nth(self.0).map(|option| vec![option.index]).unwrap_or_default()
        }
    }
    for form in 0..3 {
        for choice in 0..2 {
            let native = form == 0;
            let rules = match form {
                0 => "Riot",
                1 => "This creature enters with your choice of an additional +1/+1 counter or haste.",
                _ => "This creature enters with your choice of an additional charge counter or flying.",
            };
            let mut builder = compiler::CardDefinitionBuilder::new(compiler::ids::CardId::new(), "Adaptive entrant");
            if !native { builder = builder.with_custom_keyword("Adaptation"); }
            let card = compile_builder_to_runtime_definition(builder, format!("Type: Creature\nMana cost: {{0}}\nPower/Toughness: 2/2\n{rules}"), false).unwrap();
            let entry = card.abilities.iter().find_map(|a| match &a.kind {
                AbilityKind::Static(a) => match &a.compiled_model()?.payload {
                    ironsmith_core::StaticAbilityPayload::AsEntersEffectProgram { presentation_label, .. } => Some(presentation_label),
                    _ => None,
                }, _ => None,
            }).unwrap();
            assert_eq!(matches!(entry, Some(ironsmith::ability::PresentationLabel::Keyword(ironsmith::ability::PresentationKeyword::Riot))), native);
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            game.turn.active_player = a;
            game.turn.priority_player = Some(a);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.turn.step = None;
            let source = game.create_object_from_definition(&card, a, Zone::Hand);
            let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == source)).unwrap();
            finish_reminder_cast(&mut game, action);
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut Mode(choice)).unwrap();
            let source = game.battlefield[0];
            game.refresh_continuous_state().unwrap();
            let counter = if form == 2 { ironsmith::object::CounterType::Charge } else { ironsmith::object::CounterType::PlusOnePlusOne };
            let ability = if form == 2 { StaticAbilityId::Flying } else { StaticAbilityId::Haste };
            assert_eq!(game.counter_count(source, counter), u32::from(choice == 0));
            assert_eq!(game.current_has_static_ability_id(source, ability), choice == 1);
            assert!(game.stack.is_empty(), "the choice is part of entry, not a triggered ability");
            game.move_object(source, Zone::Graveyard, ironsmith::events::EventCause::effect());
            let source = game.player(a).unwrap().graveyard[0];
            game.move_object(source, Zone::Battlefield, ironsmith::events::EventCause::effect());
            game.refresh_continuous_state().unwrap();
            let source = game.battlefield[0];
            assert!(!game.current_has_static_ability_id(source, ability), "the prior object's granted ability must not carry through a zone change");
            let _ = SelectFirstDecisionMaker;
        }
    }
}

#[test]
fn reminder_identity_graveyard_return_tracks_arrival_and_exiles_on_deadline_or_departure() {
    for native in [false, true] {
        for exit in 0..5 {
            let rules = if native { "Unearth—Pay eight {E}." } else {
                "Pay eight energy counters: Return this card from your graveyard to the battlefield. It gains haste. Exile it at the beginning of the next end step or if it would leave the battlefield. Reawaken only as a sorcery."
            };
            let card = definition("Visitor", rules, (!native).then_some("Reawaken"));
            let (ability, activated) = card.abilities.iter().find_map(|ability| match &ability.kind { AbilityKind::Activated(a) => Some((ability, a)), _ => None }).unwrap();
            assert_eq!(ability.functional_zones, vec![Zone::Graveyard]);
            assert_eq!(activated.timing, ironsmith::ability::ActivationTiming::SorcerySpeed);
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            let b = PlayerId::from_index(1);
            let source = game.create_object_from_definition(&card, a, Zone::Graveyard);
            let marker = ironsmith::target::ObjectFilter::default().in_zone(Zone::Graveyard).with_ability_marker("unearth");
            let mut ctx = ironsmith::effects::EffectContext::new_default(source, a);
            let matching = ironsmith::effects::helpers::resolve_objects_from_spec(&game, &ironsmith::target::ChooseSpec::All(marker), &ctx).unwrap();
            assert_eq!(matching.contains(&source), native);
            for effect in activated.effects.all_effects() { ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
            let arrival = game.battlefield[0];
            assert!(game.current_has_static_ability_id(arrival, StaticAbilityId::Haste));
            if exit >= 2 {
                let destination = if exit == 2 { Zone::Hand } else { Zone::Graveyard };
                // Leave through ordinary effect processing so replacements apply.
                let mut move_ctx = ironsmith::effects::EffectContext::new_default(game.new_object_id(), b);
                let departure = ironsmith::effect::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(arrival), destination, false));
                ironsmith::effects::execute_effect(&mut game, &departure, &mut move_ctx).unwrap();
                assert!(game.battlefield.is_empty());
                assert_eq!(game.exile.len(), 1, "every departure is replaced by exile native={native} exit={exit}");
                if exit == 4 {
                    let exiled = game.exile[0];
                    game.move_object(exiled, Zone::Battlefield, ironsmith::events::EventCause::effect());
                }
            }
            let event = ironsmith::triggers::TriggerEvent::new(ironsmith::events::BeginningOfEndStepEvent::new(b), Default::default());
            let entries = ironsmith::triggers::check_delayed_triggers(&mut game, &event);
            if exit == 1 {
                assert_eq!(entries.len(), 1);
                // Countering the delayed trigger does not end the haste grant.
                assert!(game.current_has_static_ability_id(arrival, StaticAbilityId::Haste));
                assert!(ironsmith::triggers::check_delayed_triggers(&mut game, &event).is_empty());
            } else {
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                for entry in entries { queue.add(entry); }
                ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
                assert_eq!(game.battlefield.len(), usize::from(exit == 4));
                if exit == 4 { assert!(!game.current_has_static_ability_id(game.battlefield[0], StaticAbilityId::Haste)); }
            }
        }
    }
}

#[test]
fn reminder_identity_extended_operations_accept_legacy_unit_artifacts() {
    let native: ironsmith_core::AscendEffect = serde_json::from_str("null").unwrap();
    assert_eq!(native, ironsmith_core::AscendEffect::new());
    let native: ironsmith_core::LearnEffect = serde_json::from_str("null").unwrap();
    assert_eq!(native, ironsmith_core::LearnEffect::new());
    let native: ironsmith_core::UnearthEffect = serde_json::from_str("null").unwrap();
    assert_eq!(native, ironsmith_core::UnearthEffect::new());
    let generic = ironsmith_core::UnearthEffect::return_with_haste_and_exile_deadline();
    let decoded: ironsmith_core::UnearthEffect = serde_json::from_value(serde_json::to_value(generic).unwrap()).unwrap();
    assert!(decoded.descriptive);
    let generic = ironsmith_core::AscendEffect::designation_if_permanent_count(4, true, true);
    let decoded: ironsmith_core::AscendEffect = serde_json::from_value(serde_json::to_value(generic).unwrap()).unwrap();
    assert_eq!(decoded, generic);
}

#[test]
fn reminder_identity_attachment_exchange_is_optional_atomic_and_keeps_the_attachment() {
    use ironsmith::decision::DecisionMaker;
    use ironsmith::decisions::context::SelectObjectsContext;
    struct Choose(bool);
    impl DecisionMaker for Choose {
        fn decide_objects(&mut self, _: &GameState, ctx: &SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            if !self.0 { return vec![]; }
            ctx.candidates.iter().find(|candidate| candidate.legal).map(|candidate| vec![candidate.id]).unwrap_or_default()
        }
    }
    for native in [false, true] {
        for scenario in 0..5 {
            let rules = if native { "Aura swap {2}{U}" } else { "{2}{U}: Exchange this Aura with an Aura card in your hand." };
            let card = compile_to_runtime_definition("Outgoing", &format!("Type: Enchantment — Aura\nMana cost: {{1}}\nEnchant creature\n{rules}"), false).unwrap();
            let incoming = compile_to_runtime_definition("Incoming", &format!("Type: Enchantment — Aura\nMana cost: {{1}}\nEnchant {}", if scenario == 2 { "land" } else { "creature" }), false).unwrap();
            let activated = card.abilities.iter().find_map(|a| match &a.kind { AbilityKind::Activated(a) => Some(a), _ => None }).unwrap();
            let effects = activated.effects.all_effects();
            let exchange = effects.iter().find_map(|e| e.downcast_ref::<ironsmith_core::AuraSwapEffect>()).unwrap();
            assert_eq!(exchange.descriptive, !native);
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            let b = PlayerId::from_index(1);
            let target = game.create_object_from_definition(&definition("Host", "", None), a, Zone::Battlefield);
            let source = game.create_object_from_definition(&card, if scenario == 3 { b } else { a }, Zone::Battlefield);
            let hand = game.create_object_from_definition(&incoming, a, Zone::Hand);
            assert!(game.attach_object_to_target(source, ironsmith::object::AttachmentTarget::Object(target)));
            if scenario == 4 {
                let replacement = ironsmith::replacement::ReplacementEffect::with_matcher(source, a,
                    ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ironsmith::target::ObjectFilter::specific(hand), Some(Zone::Hand), Some(Zone::Battlefield)),
                    ironsmith::replacement::ReplacementAction::ChangeDestination(Zone::Exile));
                let effect = ironsmith::effect::Effect::new(ironsmith::effects::ApplyReplacementEffect::resolution(replacement));
                ironsmith::effects::execute_effect(&mut game, &effect, &mut ironsmith::effects::EffectContext::new_default(source, a)).unwrap();
            }
            let mut dm = Choose(scenario != 1);
            let mut ctx = ironsmith::effects::EffectContext::new(source, a, &mut dm);
            for effect in effects { ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
            if scenario == 4 {
                assert!(game.player(a).unwrap().hand.iter().any(|id| game.object(*id).unwrap().name == "Outgoing"));
                assert!(game.exile.iter().any(|id| game.object(*id).unwrap().name == "Incoming"));
            } else if scenario == 0 {
                assert!(game.player(a).unwrap().hand.iter().any(|id| game.object(*id).unwrap().name == "Outgoing"));
                let arriving = game.battlefield.iter().find_map(|id| game.object(*id).filter(|object| object.name == "Incoming")).unwrap();
                assert_eq!(arriving.attached_to, Some(ironsmith::object::AttachmentTarget::Object(target)));
            } else {
                assert!(game.battlefield.contains(&source));
                assert!(game.player(a).unwrap().hand.contains(&hand));
                assert!(game.exile.is_empty(), "a failed exchange must not partially commit");
                assert_eq!(game.object(source).unwrap().attached_to, Some(ironsmith::object::AttachmentTarget::Object(target)));
            }
        }
    }
}

#[test]
fn reminder_identity_alternative_price_battlefield_riders_follow_copied_cast_choices() {
    use ironsmith::decision::{SelectFirstDecisionMaker, LegalAction, compute_legal_actions};
    use ironsmith::alternative_cast::CastingMethod;
    for sacrifice in [false, true] {
        for native in [false, true] {
            for alternative in [false, true] {
                for copied in [false, true] {
                    let rules = match (sacrifice, native) {
                        (false, true) => "Dash {1}",
                        (true, true) => "Blitz {1}",
                        (false, false) => "You may cast this spell for {1}. If you do, it gains haste, and it's returned from the battlefield to its owner's hand at the beginning of the next end step.",
                        (true, false) => "If you cast this spell for {1}, it gains haste and \"When this creature dies, draw a card.\" Sacrifice it at the beginning of the next end step.",
                    };
                    let card = compile_to_runtime_definition("Short Visit", &format!("Type: Creature\nMana cost: {{3}}\nPower/Toughness: 2/2\n{rules}"), false).unwrap();
                    assert_eq!(card.alternative_casts.len(), 1);
                    let native_name = if sacrifice { "Blitz" } else { "Dash" };
                    assert_eq!(card.alternative_casts[0].name() == native_name, native);
                    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                    let a = PlayerId::from_index(0);
                    game.turn.active_player = a;
                    game.turn.priority_player = Some(a);
                    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
                    game.turn.step = None;
                    game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 3);
                    let source = game.create_object_from_definition(&card, a, Zone::Hand);
                    let method = if alternative { CastingMethod::Alternative(0) } else { CastingMethod::Normal };
                    let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, casting_method, .. } if *spell_id == source && *casting_method == method)).unwrap();
                    finish_reminder_cast(&mut game, action);
                    assert_eq!(game.player(a).unwrap().mana_pool.total(), if alternative { 2 } else { 0 });
                    let spell = game.stack.last().unwrap().object_id;
                    assert_eq!(game.object(spell).unwrap().optional_costs_paid.was_paid_label(native_name), native && alternative);
                    if copied {
                        let copy = ironsmith::effect::Effect::new(ironsmith::effects::CopySpellEffect::new(ironsmith::target::ChooseSpec::SpecificObject(spell), 1));
                        let mut ctx = ironsmith::effects::EffectContext::new_default(spell, a).with_casting_method(method);
                        ironsmith::effects::execute_effect(&mut game, &copy, &mut ctx).unwrap();
                    }
                    while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
                    let count = 1 + usize::from(copied);
                    assert_eq!(game.battlefield.len(), count);
                    for id in game.battlefield.iter().copied() { assert_eq!(game.current_has_static_ability_id(id, StaticAbilityId::Haste), alternative); }
                    for _ in 0..count { game.create_object_from_definition(&definition("Draw", "", None), a, Zone::Library); }
                    let event = ironsmith::triggers::TriggerEvent::new(ironsmith::events::BeginningOfEndStepEvent::new(PlayerId::from_index(1)), Default::default());
                    let mut queue = ironsmith::triggers::TriggerQueue::new();
                    let entries = ironsmith::triggers::check_delayed_triggers(&mut game, &event);
                    assert_eq!(entries.len(), if alternative { count } else { 0 });
                    for entry in entries { queue.add(entry); }
                    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                    for _ in 0..12 {
                        if game.stack.is_empty() { break; }
                        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
                        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
                        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                    }
                    assert!(game.stack.is_empty());
                    assert_eq!(game.battlefield.len(), if alternative { 0 } else { count });
                    if sacrifice { assert_eq!(game.player(a).unwrap().hand.len(), if alternative { count } else { 0 }); }
                    else if alternative { assert!(game.player(a).unwrap().hand.iter().any(|id| game.object(*id).unwrap().name == "Short Visit")); }
                }
            }
        }
    }
}

#[test]
fn reminder_identity_entry_sacrifice_is_a_trigger_bound_to_the_selected_price() {
    use ironsmith::decision::{SelectFirstDecisionMaker, LegalAction, compute_legal_actions};
    use ironsmith::alternative_cast::CastingMethod;
    for native in [false, true] {
        for nonmana in [false, true] {
            for alternative in [false, true] {
                for blink in [false, true] {
                    let price = if nonmana { "Pay 2 life" } else { "{1}" };
                    let rules = if native { format!("Evoke—{price}") } else { format!("You may cast this spell for {price}. If you do, it's sacrificed when it enters.") };
                    let card = compile_to_runtime_definition("Brief Entry", &format!("Type: Creature\nMana cost: {{3}}\nPower/Toughness: 2/2\n{rules}"), false).unwrap();
                    assert_eq!(card.alternative_casts[0].name() == "Evoke", native);
                    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                    let a = PlayerId::from_index(0);
                    game.turn.active_player = a;
                    game.turn.priority_player = Some(a);
                    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
                    game.turn.step = None;
                    game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 3);
                    let source = game.create_object_from_definition(&card, a, Zone::Hand);
                    let method = if alternative { CastingMethod::Alternative(0) } else { CastingMethod::Normal };
                    let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, casting_method, .. } if *spell_id == source && *casting_method == method)).unwrap();
                    finish_reminder_cast(&mut game, action);
                    assert_eq!(game.player(a).unwrap().life, if alternative && nonmana { 18 } else { 20 });
                    assert_eq!(game.player(a).unwrap().mana_pool.total(), if !alternative { 0 } else if nonmana { 3 } else { 2 });
                    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                    assert_eq!(game.battlefield.len(), 1, "sacrifice must wait for the entry trigger");
                    let mut queue = ironsmith::triggers::TriggerQueue::new();
                    ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
                    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                    assert_eq!(game.stack.len(), usize::from(alternative));
                    if blink {
                        game.move_object(game.battlefield[0], Zone::Exile, ironsmith::events::EventCause::effect());
                        game.move_object(game.exile[0], Zone::Battlefield, ironsmith::events::EventCause::effect());
                    }
                    while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
                    assert_eq!(game.battlefield.len(), usize::from(!alternative || blink), "a new incarnation must not be sacrificed by the old trigger");
                }
            }
        }
    }
}

#[test]
fn reminder_identity_entry_countdown_binds_price_counter_kind_and_continuous_type_change() {
    use ironsmith::decision::{SelectFirstDecisionMaker, LegalAction, compute_legal_actions};
    use ironsmith::alternative_cast::CastingMethod;
    for form in 0..3 {
        for alternative in [false, true] {
            let counter = if form == 2 { ironsmith::object::CounterType::Charge } else { ironsmith::object::CounterType::Time };
            let count = if form == 2 { 2 } else { 4 };
            let rules = match form {
                0 => "Impending 4—{1}",
                1 => "If you cast this spell for {1}, it enters with four time counters and isn't a creature until the last is removed. At the beginning of your end step, remove a time counter from it.",
                _ => "If you cast this spell for {1}, it enters with two charge counters and isn't a creature until the last is removed. At the beginning of your end step, remove a charge counter from it.",
            };
            let card = compile_to_runtime_definition("Countdown", &format!("Type: Enchantment Creature\nMana cost: {{3}}\nPower/Toughness: 4/4\n{rules}"), false).unwrap();
            assert_eq!(card.alternative_casts[0].name() == "Impending", form == 0);
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            let b = PlayerId::from_index(1);
            game.turn.active_player = a;
            game.turn.priority_player = Some(a);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.turn.step = None;
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 3);
            let source = game.create_object_from_definition(&card, a, Zone::Hand);
            let method = if alternative { CastingMethod::Alternative(0) } else { CastingMethod::Normal };
            let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, casting_method, .. } if *spell_id == source && *casting_method == method)).unwrap();
            finish_reminder_cast(&mut game, action);
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            let source = game.battlefield[0];
            assert_eq!(game.counter_count(source, counter), if alternative { count } else { 0 });
            let is_creature = |game: &GameState| game.current_card_types(source).unwrap().contains(&ironsmith::types::CardType::Creature);
            assert_eq!(is_creature(&game), !alternative);
            for remaining in (0..count).rev() {
                let opponent_end = ironsmith::triggers::TriggerEvent::new(ironsmith::events::BeginningOfEndStepEvent::new(b), Default::default());
                assert!(ironsmith::triggers::check_triggers(&game, &opponent_end).is_empty());
                let own_end = ironsmith::triggers::TriggerEvent::new(ironsmith::events::BeginningOfEndStepEvent::new(a), Default::default());
                let entries = ironsmith::triggers::check_triggers(&game, &own_end);
                assert_eq!(entries.len(), usize::from(alternative));
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                for entry in entries { queue.add(entry); }
                ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap(); }
                game.refresh_continuous_state().unwrap();
                assert_eq!(game.counter_count(source, counter), if alternative { remaining } else { 0 });
                assert_eq!(is_creature(&game), !alternative || remaining == 0);
            }
            // The full rule is a continuous condition: putting a counter back
            // restores the type restriction only if that price was paid.
            game.add_counters(source, counter, 1);
            game.refresh_continuous_state().unwrap();
            assert_eq!(is_creature(&game), !alternative);
            assert!(game.current_card_types(source).unwrap().contains(&ironsmith::types::CardType::Enchantment));
        }
    }
}

#[test]
fn reminder_identity_tapped_cost_power_and_counter_threshold_keep_station_identity_separate() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    use ironsmith::object::CounterType;
    for native in [false, true] {
        for threshold in [5, 20] {
            for toughness_override in [false, true] {
                for change in 0..4 {
                    let reminder = format!("Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at {threshold}+.");
                    let rules = if native { format!("Station ({reminder})") } else { reminder };
                    let card = compile_to_runtime_definition("Vessel", &format!("Type: Artifact — Spacecraft\nMana cost: {{3}}\nPower/Toughness: 20/20\n{rules}"), false).unwrap();
                    let mut donor = compile_to_runtime_definition("Contributor", "Type: Creature\nPower/Toughness: 3/5", false).unwrap();
                    if toughness_override { donor.abilities.push(Ability::static_ability(StaticAbility::keyword_marker("This creature stations permanents using its toughness rather than its power."))); }
                    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                    let a = PlayerId::from_index(0);
                    game.turn.active_player = a;
                    game.turn.priority_player = Some(a);
                    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
                    game.turn.step = None;
                    let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
                    let donor = game.create_object_from_definition(&donor, a, Zone::Battlefield);
                    game.add_counters(source, CounterType::Charge, threshold - 3);
                    game.refresh_continuous_state().unwrap();
                    assert!(!game.current_card_types(source).unwrap().contains(&ironsmith::types::CardType::Creature));
                    let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)).unwrap();
                    finish_reminder_cast(&mut game, action);
                    assert!(game.is_tapped(donor));
                    if change % 2 == 1 { game.add_counters(donor, CounterType::PlusOnePlusOne, 1); }
                    if change >= 2 { game.move_object(donor, Zone::Graveyard, ironsmith::events::EventCause::effect()); }
                    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                    let contribution = if native && toughness_override { 5 } else { 3 } + u32::from(change % 2 == 1);
                    assert_eq!(game.counter_count(source, CounterType::Charge), threshold - 3 + contribution);
                    assert!(game.current_card_types(source).unwrap().contains(&ironsmith::types::CardType::Creature));
                    game.remove_counters(source, CounterType::Charge, threshold, None, None);
                    game.refresh_continuous_state().unwrap();
                    assert!(!game.current_card_types(source).unwrap().contains(&ironsmith::types::CardType::Creature));
                    assert!(game.current_card_types(source).unwrap().contains(&ironsmith::types::CardType::Artifact));
                }
            }
        }
    }
}

#[test]
fn reminder_identity_attachment_pair_has_two_activations_and_complete_alternative_prices() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    use ironsmith::object::CounterType;
    for native in [false, true] {
        for energy in [false, true] {
            let rules = if native { "Reconfigure—Pay {2} or {E}{E}{E}." } else {
                "Pay {2} or {E}{E}{E}: Attach to target creature you control; or unattach from a creature. Reconfigure only as a sorcery. While attached, this isn't a creature."
            };
            let card = compile_to_runtime_definition("Attachment", &format!("Type: Artifact Creature — Equipment\nMana cost: {{3}}\nPower/Toughness: 3/3\n{rules}"), false).unwrap();
            let indices: Vec<_> = card.abilities.iter().enumerate().filter_map(|(i, a)| matches!(&a.kind, AbilityKind::Activated(_)).then_some(i)).collect();
            assert_eq!(indices.len(), 2, "attach and detach are independently activated abilities");
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            game.turn.active_player = a;
            game.turn.priority_player = Some(a);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.turn.step = None;
            let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
            let host = game.create_object_from_definition(&definition("Host", "", None), a, Zone::Battlefield);
            if energy { game.add_player_counters_with_source(a, CounterType::Energy, 6, None, None).unwrap(); }
            else { game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 4); }
            let actions = |game: &GameState| compute_legal_actions(game, a).unwrap().into_iter().filter(|action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)).collect::<Vec<_>>();
            assert!(!actions(&game).iter().any(|action| matches!(action, LegalAction::ActivateAbility { ability_index, .. } if *ability_index == indices[1])));
            for branch in 0..2 {
                let action = actions(&game).into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { ability_index, .. } if *ability_index == indices[branch])).unwrap();
                finish_reminder_cast(&mut game, action);
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                game.refresh_continuous_state().unwrap();
                assert_eq!(game.object(source).unwrap().attached_to, if branch == 0 { Some(ironsmith::object::AttachmentTarget::Object(host)) } else { None });
                assert_eq!(game.current_card_types(source).unwrap().contains(&ironsmith::types::CardType::Creature), branch == 1);
                assert_eq!(game.player(a).unwrap().counter_count(CounterType::Energy), if energy { (1 - branch) as u32 * 3 } else { 0 });
                assert_eq!(game.player(a).unwrap().mana_pool.total(), if energy { 0 } else { (1 - branch) as u32 * 2 });
                if branch == 0 {
                    game.turn.phase = ironsmith::game_state::Phase::Combat;
                    assert!(actions(&game).is_empty());
                    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
                    game.turn.priority_player = Some(a);
                }
            }
        }
    }
}

#[test]
fn reminder_identity_attachment_destruction_replaces_the_event_without_keyword_identity() {
    use ironsmith::object::AttachmentTarget;
    use ironsmith::events::processing::{process_destroy_full, DestroyResult};
    for native in [false, true] {
        for indestructible_attachment in [false, true] {
            for sacrifice in [false, true] {
                let player = PlayerId::from_index(0);
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let creature = definition("Protected", "", None);
                let protected = game.create_object_from_definition(&creature, player, Zone::Battlefield);
                let unrelated = game.create_object_from_definition(&creature, player, Zone::Battlefield);
                let rules = if native { "Umbra armor" } else {
                    "If enchanted creature would be destroyed, instead remove all damage from it and destroy this Aura."
                };
                let mut builder = compiler::CardDefinitionBuilder::new(compiler::ids::CardId::new(), "Protective Shell");
                if !native { builder = builder.with_custom_keyword("Safeguard"); }
                let aura = compile_builder_to_runtime_definition(builder, format!(
                    "Type: Enchantment — Aura\nEnchant creature\n{rules}{}",
                    if indestructible_attachment { "\nIndestructible" } else { "" }
                ), false).unwrap();
                let attachment = game.create_object_from_definition(&aura, player, Zone::Battlefield);
                game.object_mut(attachment).unwrap().attached_to = Some(AttachmentTarget::Object(protected));
                game.object_mut(protected).unwrap().attachments.push(attachment);
                game.mark_damage(protected, 2);
                assert_eq!(game.current_has_static_ability_id(attachment, StaticAbilityId::UmbraArmor), native);
                assert_eq!(process_destroy_full(&mut game, unrelated, None).unwrap(), DestroyResult::Destroyed { final_zone: Zone::Graveyard });
                if sacrifice {
                    // A sacrifice is a zone move, not a destruction event.
                    game.move_object(protected, Zone::Graveyard, ironsmith::events::EventCause::effect());
                    assert!(game.object(attachment).is_some());
                } else {
                    assert_eq!(process_destroy_full(&mut game, protected, None).unwrap(), DestroyResult::Replaced);
                    assert_eq!(game.object(protected).unwrap().zone, Zone::Battlefield);
                    assert_eq!(game.damage_on(protected), 0);
                    assert_eq!(game.object(attachment).is_some(), indestructible_attachment);
                    // Replacement still saves the host when its instruction
                    // cannot destroy the indestructible attachment.
                    if indestructible_attachment {
                        game.mark_damage(protected, 3);
                        assert_eq!(process_destroy_full(&mut game, protected, None).unwrap(), DestroyResult::Replaced);
                        assert_eq!(game.damage_on(protected), 0);
                    }
                }
            }
        }
    }
}

#[test]
fn reminder_identity_paid_spell_condition_uses_actual_payment_and_rechecks_on_resolution() {
    use ironsmith::decision::{compute_legal_actions, LegalAction, SelectFirstDecisionMaker};
    use ironsmith::object::CounterType;
    for native in [false, true] {
        for (spent, before_resolution, expected) in [(0, 0, 0), (2, 0, 0), (3, 0, 1), (6, 0, 1), (3, 2, 0)] {
            for opponent in [false, true] {
                let rules = if native { "Increment" } else { "Whenever you cast a spell, if the amount of mana you spent is greater than this creature's power or toughness, put a +1/+1 counter on this creature." };
                let card = definition("Growing", rules, (!native).then_some("Accumulate"));
                let triggered = card.abilities.iter().find_map(|a| match &a.kind { AbilityKind::Triggered(a) => Some(a), _ => None }).unwrap();
                assert_eq!(triggered.presentation_label.is_some(), native);
                assert_eq!(triggered.intervening_if.as_ref(), Some(&ironsmith::ConditionExpr::increment()));
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let a = PlayerId::from_index(0);
                let caster = PlayerId::from_index(u8::from(opponent));
                let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
                game.turn.active_player = caster;
                game.turn.priority_player = Some(caster);
                game.turn.phase = ironsmith::game_state::Phase::FirstMain;
                game.turn.step = None;
                let spell = compile_to_runtime_definition("Paid spell", &format!("Type: Sorcery\nMana cost: {{9}}\nYou may cast this spell for {{{spent}}}.\nYou gain 1 life."), false).unwrap();
                let hand = game.create_object_from_definition(&spell, caster, Zone::Hand);
                game.player_mut(caster).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, spent);
                let action = compute_legal_actions(&game, caster).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == hand)).unwrap();
                finish_reminder_cast(&mut game, action);
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                let mut dm = SelectFirstDecisionMaker;
                ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut dm).unwrap();
                ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                assert_eq!(game.stack.iter().filter(|entry| entry.is_ability).count(), usize::from(!opponent && spent > 2));
                if before_resolution > 0 { game.add_counters(source, CounterType::PlusOnePlusOne, before_resolution); }
                while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap(); }
                assert_eq!(game.object(source).unwrap().counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0), before_resolution + if opponent { 0 } else { expected });
            }
        }
    }
}

#[test]
fn reminder_identity_activation_limits_keep_native_exceptions_separate() {
    use ironsmith::decision::{compute_legal_actions, LegalAction, SelectFirstDecisionMaker};
    use ironsmith::object::CounterType;
    for native in [false, true] {
        for lifetime in [false, true] {
            let rules = match (native, lifetime) {
                (true, true) => "Exhaust — {1}, Pay 2 life: Put two +1/+1 counters on this creature.",
                (false, true) => "{1}, Pay 2 life: Put two +1/+1 counters on this creature. Activate this ability only once.",
                (true, false) => "Boast — {1}, Pay 2 life: Put two +1/+1 counters on this creature.",
                (false, false) => "{1}, Pay 2 life: Put two +1/+1 counters on this creature. Activate only if this creature attacked this turn and only once each turn.",
            };
            let card = definition("Performer", rules, (!native).then_some("Performance"));
            let activated = card.abilities.iter().find_map(|a| match &a.kind { AbilityKind::Activated(a) => Some(a), _ => None }).unwrap();
            assert_eq!(activated.is_exhaust_ability(), native && lifetime);
            assert_eq!(activated.keyword.is_some(), native);
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
            game.turn.active_player = a;
            game.turn.priority_player = Some(a);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.turn.step = None;
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 5);
            let action = |game: &GameState, source| compute_legal_actions(game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source));
            assert_eq!(action(&game, source).is_some(), lifetime);
            game.mark_creature_attacked_this_turn(source);
            let activation = action(&game, source).unwrap();
            finish_reminder_cast(&mut game, activation);
            assert_eq!(game.player(a).unwrap().life, 18);
            assert_eq!(game.player(a).unwrap().mana_pool.total(), 4);
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
            assert_eq!(game.object(source).unwrap().counters.get(&CounterType::PlusOnePlusOne), Some(&2));
            assert!(action(&game, source).is_none());
            game.turn_store.turn_history.clear_for_new_turn();
            if lifetime {
                let mut permission = definition("Native exception", "", None);
                permission.abilities.push(Ability::static_ability(StaticAbility::exhaust_abilities_as_though_unactivated_this_turn()));
                game.create_object_from_definition(&permission, a, Zone::Battlefield);
                assert_eq!(action(&game, source).is_some(), native, "an Exhaust exception must not bypass a custom lifetime restriction");
            } else {
                // Activation counts are per turn; attack history must also be
                // newly satisfied, independently of the custom label.
                assert!(action(&game, source).is_none());
                game.mark_creature_attacked_this_turn(source);
                assert!(action(&game, source).is_some());
            }
            game.move_object(source, Zone::Exile, ironsmith::events::EventCause::effect());
            let exile = *game.exile.last().unwrap();
            let returned = game.move_object(exile, Zone::Battlefield, ironsmith::events::EventCause::effect()).unwrap();
            game.mark_creature_attacked_this_turn(returned);
            assert!(action(&game, returned).is_some(), "a new incarnation has a fresh activation limit");
        }
    }
}

#[test]
fn reminder_identity_upkeep_reveal_is_a_lasting_cost_with_hand_and_owner_timing() {
    use ironsmith::decision::{compute_legal_actions, LegalAction, SelectFirstDecisionMaker};
    use ironsmith::game_state::{Phase, Step};
    for native in [false, true] {
        let rules = if native { "Forecast — {1}, Reveal this card from your hand: You gain 2 life." } else {
            "{1}, Reveal this card from your hand until upkeep ends: You gain 2 life. Activate only during this card's owner's upkeep and only once each turn."
        };
        let card = definition("Seer", rules, (!native).then_some("Augury"));
        let ability = card.abilities.iter().find(|a| matches!(a.kind, AbilityKind::Activated(_))).unwrap();
        assert_eq!(ability.functional_zones, vec![Zone::Hand]);
        let AbilityKind::Activated(activated) = &ability.kind else { unreachable!() };
        assert_eq!(activated.timing, ironsmith::ability::ActivationTiming::DuringSourceOwnersUpkeep);
        assert_eq!(activated.additional_restrictions.iter().any(|r| r == "__ironsmith_activation_label:Forecast"), native);
        let reveal = activated.mana_cost.costs().iter().filter_map(|cost| cost.effect_ref())
            .find_map(|effect| effect.downcast_ref::<ironsmith::effects::RevealSourceFromHandEffect>()).unwrap();
        assert_eq!(reveal.duration, ironsmith_core::RevealSourceFromHandDuration::UntilUpkeepEndsOrLeavesHand);
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&card, a, Zone::Hand);
        game.turn.active_player = a;
        game.turn.priority_player = Some(a);
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Upkeep);
        game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 3);
        let action = |game: &GameState| compute_legal_actions(game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source));
        game.turn.active_player = PlayerId::from_index(1);
        assert!(action(&game).is_none());
        game.turn.active_player = a;
        game.turn.step = Some(Step::Draw);
        assert!(action(&game).is_none());
        game.turn.step = Some(Step::Upkeep);
        let activation = action(&game).unwrap();
        assert!(!game.is_hand_card_revealed_until_upkeep_ends(source));
        finish_reminder_cast(&mut game, activation);
        assert!(game.is_hand_card_revealed_until_upkeep_ends(source), "the reveal is paid before the effect resolves");
        assert!(action(&game).is_none());
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.player(a).unwrap().life, 22);
        assert!(game.is_hand_card_revealed_until_upkeep_ends(source), "resolution does not end the reveal");
        let mut next_step = game.clone();
        ironsmith::turn::advance_step(&mut next_step).unwrap();
        assert!(!next_step.is_hand_card_revealed_until_upkeep_ends(source));
        game.move_object(source, Zone::Graveyard, ironsmith::events::EventCause::effect());
        assert!(!game.is_hand_card_revealed_until_upkeep_ends(source));
    }
}

#[test]
fn reminder_identity_paid_all_modes_uses_full_payment_without_entwine_receipt() {
    use ironsmith::decision::{DecisionMaker, LegalAction, compute_legal_actions, SelectFirstDecisionMaker};
    use ironsmith::decisions::context::SelectOptionsContext;
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm};
    struct ChooseCost { pay: bool, expect_available: bool }
    impl DecisionMaker for ChooseCost {
        fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            if ctx.description.starts_with("Choose optional costs") {
                let available = ctx.options.iter().any(|option| option.legal && option.index == 0);
                assert_eq!(available, self.expect_available);
                return if self.pay && available { vec![0] } else { vec![] };
            }
            ctx.options.iter().filter(|option| option.legal).take(ctx.min).map(|option| option.index).collect()
        }
    }
    for native in [false, true] {
        for pay in [false, true] {
            for missing_target in [false, true] {
                let rules = if native { "Entwine—{2}, Pay 2 life" } else { "Choose both if you pay {2}, Pay 2 life." };
                let second = if missing_target { "Destroy target artifact." } else { "You gain 2 life." };
                let mut builder = compiler::CardDefinitionBuilder::new(compiler::ids::CardId::new(), "Choices");
                if !native { builder = builder.with_custom_keyword("Combine"); }
                let card = compile_builder_to_runtime_definition(builder, format!("Type: Sorcery\nMana cost: {{1}}\nChoose one —\n• You gain 1 life.\n• {second}\n{rules}"), false).unwrap();
                assert_eq!(card.optional_costs.len(), 1);
                assert_eq!(card.optional_costs[0].kind == ironsmith_core::OptionalCostKind::Entwine, native);
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let a = PlayerId::from_index(0);
                game.turn.active_player = a;
                game.turn.priority_player = Some(a);
                game.turn.phase = ironsmith::game_state::Phase::FirstMain;
                game.turn.step = None;
                game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 3);
                let source = game.create_object_from_definition(&card, a, Zone::Hand);
                let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == source)).unwrap();
                let mut dm = ChooseCost { pay, expect_available: !missing_target };
                let mut state = PriorityLoopState::new(2);
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
                for _ in 0..48 {
                    if !state.has_pending_action() { break; }
                    let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}") };
                    progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
                }
                assert!(!state.has_pending_action());
                let paid = pay && !missing_target;
                let receipt = &game.stack[0].optional_costs_paid;
                assert_eq!(receipt.paid_to_choose_all_modes(), paid);
                assert_eq!(receipt.was_entwined(), paid && native);
                assert_eq!(game.player(a).unwrap().mana_pool.total(), if paid { 0 } else { 2 });
                assert_eq!(game.player(a).unwrap().life, if paid { 18 } else { 20 });
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
                assert_eq!(game.player(a).unwrap().life, 21, "both modes grant three life after paying two; one mode grants one");
            }
        }
    }
}

#[test]
fn reminder_identity_each_extra_mode_repeats_mana_and_nonmana_payment() {
    use ironsmith::decision::{compute_legal_actions, LegalAction, SelectFirstDecisionMaker};
    for native in [false, true] {
        for modes in 1..=3 {
            let keyword = if native { "Escalate—{1}, Pay 2 life" } else { "Pay {1}, Pay 2 life for each mode chosen beyond the first." };
            let choice = ["one", "two", "three"][modes - 1];
            let mut builder = compiler::CardDefinitionBuilder::new(compiler::ids::CardId::new(), "Mode cost");
            if !native { builder = builder.with_custom_keyword("Broaden"); }
            let card = compile_builder_to_runtime_definition(builder, format!("Type: Sorcery\nMana cost: {{1}}\nChoose {choice} —\n• You gain 1 life.\n• You gain 2 life.\n• You gain 4 life.\n{keyword}"), false).unwrap();
            let payment = card.abilities.iter().find_map(|a| match &a.kind { AbilityKind::Static(a) if a.escalate_spec().is_some() => Some(a), _ => None }).unwrap_or_else(|| panic!("native={native} modes={modes} abilities={:#?}", card.abilities));
            assert_eq!(payment.id() == StaticAbilityId::Escalate, native);
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            game.turn.active_player = a;
            game.turn.priority_player = Some(a);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            game.turn.step = None;
            let hand = game.create_object_from_definition(&card, a, Zone::Hand);
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, modes as u32);
            let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == hand)).unwrap();
            finish_reminder_cast(&mut game, action);
            assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
            let life = 20 - 2 * (modes as i32 - 1);
            assert_eq!(game.player(a).unwrap().life, life);
            assert_eq!(game.stack[0].chosen_modes.as_ref().unwrap().len(), modes);
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
            assert_eq!(game.player(a).unwrap().life, life + (1 << modes) - 1);
        }
    }
}

#[test]
fn reminder_identity_reveal_to_add_text_keeps_source_in_hand_and_copies_added_program() {
    use ironsmith::decision::{DecisionMaker, LegalAction, compute_legal_actions, SelectFirstDecisionMaker};
    use ironsmith::decisions::context::SelectObjectsContext;
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm};
    struct Reveal { source: ironsmith::ids::ObjectId, accept: bool }
    impl DecisionMaker for Reveal {
        fn decide_objects(&mut self, _game: &GameState, ctx: &SelectObjectsContext) -> Vec<ironsmith::ids::ObjectId> {
            if ctx.description.contains("splice") { return if self.accept && ctx.candidates.iter().any(|c| c.id == self.source) { vec![self.source] } else { vec![] }; }
            ctx.candidates.iter().take(ctx.min).map(|c| c.id).collect()
        }
    }
    for native in [false, true] {
        for accept in [false, true] {
            for (quality, host_type, matches) in [("instant or sorcery", "Instant", true), ("Arcane", "Instant — Arcane", true), ("Arcane", "Instant", false)] {
                let rule = if native { format!("Splice onto {quality} {{2}}, Pay 2 life") } else {
                    format!("As you cast an {quality} spell, you may reveal this card from your hand and pay {{2}}, Pay 2 life. If you do, add this card's effects to that spell.")
                };
                let mut builder = compiler::CardDefinitionBuilder::new(compiler::ids::CardId::new(), "Added text");
                if !native { builder = builder.with_custom_keyword("Interleave"); }
                let addon = compile_builder_to_runtime_definition(builder, format!("Type: Instant\nMana cost: {{9}}\nYou gain 4 life.\n{rule}"), false).unwrap();
                let mechanism = addon.abilities.iter().find_map(|a| match &a.kind { AbilityKind::Static(a) if a.splice_spec().is_some() => Some(a), _ => None }).unwrap();
                assert_eq!(mechanism.id() == StaticAbilityId::Splice, native);
                let host = compile_to_runtime_definition("Host", &format!("Type: {host_type}\nMana cost: {{1}}\nYou gain 1 life."), false).unwrap();
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let a = PlayerId::from_index(0);
                game.turn.active_player = a;
                game.turn.priority_player = Some(a);
                game.turn.phase = ironsmith::game_state::Phase::FirstMain;
                game.turn.step = None;
                let addon_id = game.create_object_from_definition(&addon, a, Zone::Hand);
                let host_id = game.create_object_from_definition(&host, a, Zone::Hand);
                game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 3);
                let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == host_id)).unwrap();
                let mut dm = Reveal { source: addon_id, accept };
                let mut state = PriorityLoopState::new(2);
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
                for _ in 0..48 {
                    if !state.has_pending_action() { break; }
                    let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}") };
                    progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
                }
                assert!(!state.has_pending_action());
                let paid = accept && matches;
                assert_eq!(game.player(a).unwrap().mana_pool.total(), if paid { 0 } else { 2 });
                assert_eq!(game.player(a).unwrap().life, if paid { 18 } else { 20 });
                assert_eq!(game.object(addon_id).unwrap().zone, Zone::Hand);
                let spell = game.stack[0].object_id;
                let copy = ironsmith::effect::Effect::new(ironsmith::effects::CopySpellEffect::new(ironsmith::target::ChooseSpec::SpecificObject(spell), 1));
                let mut ctx = ironsmith::effects::EffectContext::new_default(spell, a);
                ironsmith::effects::execute_effect(&mut game, &copy, &mut ctx).unwrap();
                assert_eq!(game.stack.len(), 2);
                while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap(); }
                assert_eq!(game.player(a).unwrap().life, if paid { 28 } else { 22 });
                assert_eq!(game.object(addon_id).unwrap().zone, Zone::Hand);
            }
        }
    }
}

#[test]
fn reminder_identity_described_incubator_has_entry_counters_transform_and_no_native_action() {
    use ironsmith::decision::{compute_legal_actions, LegalAction, SelectFirstDecisionMaker};
    use ironsmith::object::CounterType;
    for native in [false, true] {
        for amount in [1, 3] {
            for doubled in [false, true] {
                let text = if native { format!("Incubate {amount}.") } else {
                    format!("Create an Incubator token with {} +1/+1 {} on it and \"{{2}}: Transform this token.\" It transforms into a 0/0 Phyrexian artifact creature.", if amount == 1 { "a" } else { "three" }, if amount == 1 { "counter" } else { "counters" })
                };
                let mut builder = compiler::CardDefinitionBuilder::new(compiler::ids::CardId::new(), "Incubator maker");
                if !native { builder = builder.with_custom_keyword("Nurture"); }
                let card = compile_builder_to_runtime_definition(builder, format!("Type: Sorcery\n{text}"), false).unwrap();
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let a = PlayerId::from_index(0);
                game.turn.active_player = a;
                game.turn.priority_player = Some(a);
                game.turn.phase = ironsmith::game_state::Phase::FirstMain;
                game.turn.step = None;
                let source = game.create_object_from_definition(&card, a, Zone::Stack);
                if doubled {
                    let mut doubler = definition("Doubler", "", None);
                    doubler.abilities.push(Ability::static_ability(StaticAbility::double_token_creation_replacement(ironsmith::target::PlayerFilter::You, "Double tokens".into())));
                    doubler.abilities.push(Ability::static_ability(StaticAbility::double_counters_replacement(ironsmith::target::ObjectFilter::default(), Some(CounterType::PlusOnePlusOne), "Double counters".into())));
                    game.create_object_from_definition(&doubler, a, Zone::Battlefield);
                }
                let mut events = Vec::new();
                let mut ctx = ironsmith::effects::EffectContext::new_default(source, a);
                for effect in card.spell_effect.as_ref().unwrap().all_effects() { events.extend(ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap().events); }
                let tokens: Vec<_> = game.battlefield.iter().copied().filter(|id| game.object(*id).unwrap().subtypes.contains(&ironsmith::types::Subtype::Incubator)).collect();
                assert_eq!(tokens.len(), if doubled { 2 } else { 1 });
                assert_eq!(events.iter().any(|event| event.downcast::<ironsmith::events::KeywordActionEvent>().is_some_and(|event| event.action == ironsmith::events::KeywordActionKind::Incubate)), native);
                for token in &tokens {
                    assert!(!game.current_is_creature(*token));
                    assert_eq!(game.counter_count(*token, CounterType::PlusOnePlusOne), amount * if doubled { 2 } else { 1 });
                }
                let token = tokens[0];
                let can_transform = |game: &GameState| compute_legal_actions(game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == token));
                game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 1);
                assert!(can_transform(&game).is_none());
                game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 1);
                let action = can_transform(&game).unwrap();
                finish_reminder_cast(&mut game, action);
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
                assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
                assert!(game.current_is_creature(token));
                assert!(game.object(token).unwrap().subtypes.contains(&ironsmith::types::Subtype::Phyrexian));
                assert_eq!(game.calculated_power(token), Some((amount * if doubled { 2 } else { 1 }) as i32));
                assert_eq!(game.calculated_toughness(token), game.calculated_power(token));
            }
        }
    }
}

#[test]
fn reminder_identity_face_down_entry_keeps_permission_without_native_provenance() {
    use ironsmith::decision::SelectFirstDecisionMaker;
    use ironsmith::special_actions::{SpecialAction, TurnFaceUpMethod, can_perform_check, perform};
    for native in [false, true] {
        for cloak in [false, true] {
            for underlying in [None, Some("Creature"), Some("Instant")] {
                let action = if cloak { "Cloak" } else { "Manifest" };
                let text = if native { format!("{action} the top card of your library.") } else {
                    format!("Put the top card of your library onto the battlefield face down as a 2/2 creature{}. Turn it face up any time for its mana cost if it's a creature card.", if cloak { " with ward {2}" } else { "" })
                };
                let mut builder = compiler::CardDefinitionBuilder::new(compiler::ids::CardId::new(), "Hidden maker");
                if !native { builder = builder.with_custom_keyword("Obscure"); }
                let card = compile_builder_to_runtime_definition(builder, format!("Type: Sorcery\n{text}"), false).unwrap();
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let a = PlayerId::from_index(0);
                game.turn.priority_player = Some(a);
                let source = game.create_object_from_definition(&card, a, Zone::Stack);
                if let Some(kind) = underlying {
                    let body = if kind == "Creature" { "Power/Toughness: 5/4\nFlying" } else { "You gain 1 life." };
                    let hidden = compile_to_runtime_definition("Hidden card", &format!("Type: {kind}\nMana cost: {{2}}\n{body}"), false).unwrap();
                    game.create_object_from_definition(&hidden, a, Zone::Library);
                }
                let mut events = Vec::new();
                let mut ctx = ironsmith::effects::EffectContext::new_default(source, a);
                for effect in card.spell_effect.as_ref().unwrap().all_effects() { events.extend(ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap().events); }
                assert_eq!(game.battlefield.len(), usize::from(underlying.is_some()));
                assert_eq!(events.iter().any(|event| event.downcast::<ironsmith::events::KeywordActionEvent>().is_some_and(|event| event.action == if cloak { ironsmith::events::KeywordActionKind::Cloak } else { ironsmith::events::KeywordActionKind::Manifest })), native && underlying.is_some());
                let Some(&permanent) = game.battlefield.first() else { continue; };
                assert!(game.is_face_down(permanent));
                assert_eq!(game.is_manifested(permanent), native && !cloak);
                assert_eq!(game.is_cloaked(permanent), native && cloak);
                assert_eq!(game.calculated_power(permanent), Some(2));
                assert_eq!(game.calculated_toughness(permanent), Some(2));
                assert!(!game.current_has_static_ability_id(permanent, StaticAbilityId::Flying));
                assert_eq!(game.current_has_static_ability_id(permanent, StaticAbilityId::Ward), cloak);
                let turn_up = SpecialAction::TurnFaceUp { permanent_id: permanent, method: TurnFaceUpMethod::PrintedManaCost };
                game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 1);
                assert!(can_perform_check(&turn_up, &game, a).is_err());
                game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 1);
                assert_eq!(can_perform_check(&turn_up, &game, a).is_ok(), underlying == Some("Creature"));
                if underlying == Some("Creature") {
                    perform(turn_up, &mut game, a, &mut SelectFirstDecisionMaker).unwrap();
                    assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
                    assert!(!game.is_face_down(permanent));
                    assert!(!game.has_printed_cost_turn_face_up_permission(permanent));
                    assert_eq!(game.calculated_power(permanent), Some(5));
                    assert!(game.current_has_static_ability_id(permanent, StaticAbilityId::Flying));
                } else {
                    let next = game.move_object(permanent, Zone::Hand, ironsmith::events::EventCause::effect()).unwrap();
                    assert!(!game.has_printed_cost_turn_face_up_permission(next));
                    assert!(!game.has_printed_cost_turn_face_up_permission(permanent));
                }
            }
        }
    }
}

#[test]
fn reminder_identity_look_choose_face_down_and_discard_other_is_composed() {
    for cloak in [false, true] {
        for count in 0usize..=2 {
            let text = format!("Look at the top two cards of your library, then put one of them onto the battlefield face down as a 2/2 creature{} and the other into your graveyard. Turn it face up any time for its mana cost if it's a creature card.", if cloak { " with ward {2}" } else { "" });
            let card = compile_to_runtime_definition("Look and hide", &format!("Type: Sorcery\n{text}"), false).unwrap();
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            let source = game.create_object_from_definition(&card, a, Zone::Stack);
            for _ in 0..count { game.create_object_from_definition(&definition("Hidden", "Flying", None), a, Zone::Library); }
            let mut ctx = ironsmith::effects::EffectContext::new_default(source, a);
            for effect in card.spell_effect.as_ref().unwrap().all_effects() {
                let outcome = ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                assert!(outcome.events.iter().all(|event| event.downcast::<ironsmith::events::KeywordActionEvent>().is_none()));
            }
            assert_eq!(game.battlefield.len(), usize::from(count > 0));
            assert_eq!(game.player(a).unwrap().graveyard.len(), count.saturating_sub(1));
            assert!(game.player(a).unwrap().library.is_empty());
            for id in &game.battlefield {
                assert!(game.is_face_down(*id));
                assert!(game.has_printed_cost_turn_face_up_permission(*id));
                assert!(!game.is_manifested(*id) && !game.is_cloaked(*id));
                assert_eq!(game.current_has_static_ability_id(*id, StaticAbilityId::Ward), cloak);
            }
        }
    }
}

#[test]
fn reminder_identity_reveal_land_or_counter_has_native_results_without_explore_event() {
    use ironsmith::decision::DecisionMaker;
    use ironsmith::decisions::context::BooleanContext;
    struct Bury(bool);
    impl DecisionMaker for Bury {
        fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool { self.0 }
    }
    for native in [false, true] {
        for top in [None, Some("Land"), Some("Creature")] {
            for bury in [false, true] {
                for departed in [false, true] {
                    let body = if native { "This creature explores." } else { "Reveal the top card of your library. Put that card into your hand if it's a land. Otherwise, put a +1/+1 counter on this creature, then put the card back or put it into your graveyard." };
                    let card = definition("Explorer", &format!("{{0}}: {body}"), (!native).then_some("Discover"));
                    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                    let a = PlayerId::from_index(0);
                    let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
                    if let Some(kind) = top {
                        let top = compile_to_runtime_definition("Top", &format!("Type: {kind}{}", if kind == "Creature" { "\nPower/Toughness: 1/1" } else { "" }), false).unwrap();
                        game.create_object_from_definition(&top, a, Zone::Library);
                    }
                    let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
                    if departed { game.move_object(source, Zone::Exile, ironsmith::events::EventCause::effect()).unwrap(); }
                    let effects = card.abilities.iter().find_map(|ability| match &ability.kind { AbilityKind::Activated(a) => Some(&a.effects), _ => None }).unwrap();
                    let mut dm = Bury(bury);
                    let mut ctx = ironsmith::effects::EffectContext::new_default(source, a).with_source_snapshot(snapshot).with_decision_maker(&mut dm);
                    let mut events = Vec::new();
                    for effect in effects { events.extend(ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap().events); }
                    assert_eq!(game.player(a).unwrap().hand.len(), usize::from(top == Some("Land")));
                    assert_eq!(game.player(a).unwrap().graveyard.len(), usize::from(top == Some("Creature") && bury));
                    assert_eq!(game.player(a).unwrap().library.len(), usize::from(top == Some("Creature") && !bury));
                    if !departed { assert_eq!(game.counter_count(source, ironsmith::object::CounterType::PlusOnePlusOne), u32::from(top != Some("Land"))); }
                    assert_eq!(events.iter().any(|event| event.downcast::<ironsmith::events::KeywordActionEvent>().is_some_and(|event| event.action == ironsmith::events::KeywordActionKind::Explore)), native);
                }
            }
        }
    }
}

#[test]
fn reminder_identity_next_untap_commitment_is_a_cost_with_fixed_player_duration() {
    use ironsmith::decision::{compute_legal_actions, LegalAction, SelectFirstDecisionMaker};
    fn untap(game: &mut GameState, player: PlayerId) {
        game.turn.turn_number += 1;
        game.turn.active_player = player;
        game.turn.phase = ironsmith::Phase::Beginning;
        game.turn.step = Some(ironsmith::game_state::Step::Untap);
        ironsmith::turn::execute_untap_step_with(game, &mut SelectFirstDecisionMaker).unwrap();
    }
    for native in [false, true] {
        for change_control in [false, true] {
            let cost = if native { "Exert this creature" } else { "Choose to have this creature not untap during your next untap step" };
            let card = definition("Worker", &format!("{{1}}, {{T}}, {cost}: You gain 1 life."), (!native).then_some("Strain"));
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
            game.turn.active_player = a;
            game.turn.priority_player = Some(a);
            let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
            game.remove_summoning_sickness(source);
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 1);
            let activated = card.abilities.iter().find_map(|ability| match &ability.kind { AbilityKind::Activated(a) => Some(a), _ => None }).unwrap();
            let commitment = activated.mana_cost.costs().iter().filter_map(|cost| cost.effect_ref()).find(|effect| effect.downcast_ref::<ironsmith::effects::ExertCostEffect>().is_some()).unwrap();
            // The same payment remains legal while already tapped/committed;
            // only the separate tap cost prevents a second full activation.
            let mut branch = game.clone();
            let mut ctx = ironsmith::effects::EffectContext::new_default(source, a);
            for _ in 0..2 {
                let outcome = ironsmith::effects::execute_effect(&mut branch, commitment, &mut ctx).unwrap();
                assert_eq!(outcome.events.iter().any(|event| event.downcast::<ironsmith::events::KeywordActionEvent>().is_some_and(|event| event.action == ironsmith::events::KeywordActionKind::Exert)), native);
            }
            let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)).unwrap();
            finish_reminder_cast(&mut game, action);
            assert!(game.is_tapped(source));
            assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
            assert_eq!(game.player(a).unwrap().life, 21);
            if change_control { game.set_current_controller(source, b).unwrap(); }
            untap(&mut game, b);
            assert_eq!(game.is_tapped(source), !change_control);
            // Transfer back and tap: the commitment still belongs to A.
            game.set_current_controller(source, a).unwrap();
            game.tap(source);
            untap(&mut game, a);
            assert!(game.is_tapped(source));
            untap(&mut game, a);
            assert!(!game.is_tapped(source));
        }
    }
}

#[test]
fn reminder_identity_combat_history_full_definition_tracks_current_spell_types() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
    for native in [false, true] {
        let body = if native { "Prowl {1}" } else { "You may cast this spell for {1} if you dealt combat damage to a player this turn with a source that shares a creature type with this spell." };
        let mut builder = compiler::CardDefinitionBuilder::new(compiler::ids::CardId::new(), "Changing spell");
        if !native { builder = builder.with_custom_keyword("Afterstrike"); }
        let card = compile_builder_to_runtime_definition(builder, format!("Type: Creature — Sphinx Rogue\nMana cost: {{9}}\nPower/Toughness: 2/2\n{body}"), false).unwrap();
        assert_eq!(card.alternative_casts[0].name() == "Prowl", native);
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
        game.turn.active_player = a;
        game.turn.priority_player = Some(a);
        game.turn.phase = ironsmith::Phase::FirstMain;
        game.turn.step = None;
        let spell = game.create_object_from_definition(&card, a, Zone::Hand);
        let attacker = compile_to_runtime_definition("Bird", "Type: Creature — Bird\nPower/Toughness: 2/2", false).unwrap();
        let attacker = game.create_object_from_definition(&attacker, a, Zone::Battlefield);
        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo { creature: attacker, target: AttackTarget::Player(b) });
        combat.blockers.insert(attacker, vec![]);
        ironsmith::game_loop::execute_combat_damage_step(&mut game, &combat, false);
        game.move_object(attacker, Zone::Graveyard, ironsmith::events::EventCause::effect()).unwrap();
        game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 1);
        let can_cast = |game: &GameState| compute_legal_actions(game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell));
        assert!(can_cast(&game).is_none());
        game.object_mut(spell).unwrap().subtypes = vec![ironsmith::types::Subtype::Bird].into();
        assert!(can_cast(&game).is_some(), "current types must replace the printed reminder's examples");
        let mut reset = game.clone();
        reset.turn_store.turn_history.clear_for_new_turn();
        assert!(can_cast(&reset).is_none());
        let action = can_cast(&game).unwrap();
        finish_reminder_cast(&mut game, action);
        assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
    }
}

#[test]
fn reminder_identity_quality_defense_shares_damage_block_target_and_attachment_rules() {
    use ironsmith::effect::Effect;
    use ironsmith::effects::{execute_effect, EffectContext};
    use ironsmith::target::ChooseSpec;
    for native in [false, true] {
        for (mana, protected) in [("{U}", true), ("{B}", true), ("{R}", false), ("{1}", false), ("{U}{R}", true)] {
            let rules = if native { "Protection from blue and from black" } else { "This creature can't be blocked, targeted, dealt damage, or enchanted by anything blue or black." };
            let card = definition("Protected", rules, (!native).then_some("Azure shelter"));
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
            let target = game.create_object_from_definition(&card, a, Zone::Battlefield);
            assert_eq!(game.current_has_static_ability_id(target, StaticAbilityId::Protection), native);
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&card));
            for owner in [a, b] {
                let source = compile_to_runtime_definition("Source", &format!("Type: Creature\nMana cost: {mana}\nPower/Toughness: 2/2"), false).unwrap();
                let source = game.create_object_from_definition(&source, owner, Zone::Battlefield);
                assert_eq!(ironsmith::targeting::can_target_object(&game, target, source, owner).is_legal(), !protected);
                if owner == b { assert_eq!(ironsmith::rules::combat::can_block(game.object(target).unwrap(), game.object(source).unwrap(), &game), !protected); }
                let mut branch = game.clone();
                let mut ctx = EffectContext::new_default(source, owner);
                execute_effect(&mut branch, &Effect::deal_damage(1, ChooseSpec::SpecificObject(target)), &mut ctx).unwrap();
                assert_eq!(branch.damage_on(target), u32::from(!protected));
                // Untargeted destruction is not part of this protection.
                let mut branch = game.clone();
                execute_effect(&mut branch, &Effect::destroy(ChooseSpec::SpecificObject(target)), &mut ctx).unwrap();
                assert!(branch.object(target).is_none());
            }
            for kind in ["Enchantment — Aura", "Artifact — Equipment"] {
                let body = if kind.contains("Aura") { "Enchant creature" } else { "Equip {0}" };
                let attachment = compile_to_runtime_definition("Attachment", &format!("Type: {kind}\nMana cost: {mana}\n{body}"), false).unwrap();
                let attachment = game.create_object_from_definition(&attachment, a, Zone::Battlefield);
                let mut ctx = EffectContext::new_default(attachment, a);
                execute_effect(&mut game, &Effect::new(ironsmith::effects::AttachToEffect::new(ChooseSpec::SpecificObject(target))), &mut ctx).unwrap();
                assert_eq!(game.object(attachment).unwrap().attached_to.is_some(), !protected);
            }
        }
    }
}

#[test]
fn reminder_identity_discard_replacement_casts_only_from_its_linked_trigger() {
    use ironsmith::decision::{compute_legal_actions, DecisionMaker, LegalAction};
    use ironsmith::decisions::context::BooleanContext;
    use ironsmith::effects::{EffectContext, EffectExecutor, MayCastForMadnessCostEffect};
    struct Accept(bool);
    impl DecisionMaker for Accept {
        fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool { self.0 }
    }
    for native in [false, true] {
        for accept in [false, true] {
            for enough in [false, true] {
                let body = if native { "Madness {1}, Pay 2 life" } else { "If you discard this card, discard it into exile. When you do, cast it for {1}, Pay 2 life or put it into your graveyard." };
                let mut builder = compiler::CardDefinitionBuilder::new(compiler::ids::CardId::new(), "Discard price");
                if !native { builder = builder.with_custom_keyword("Inspiration"); }
                let card = compile_builder_to_runtime_definition(builder, format!("Type: Sorcery\nMana cost: {{9}}\nYou gain 4 life.\n{body}"), false).unwrap();
                assert_eq!(card.alternative_casts[0].is_madness(), native);
                assert_eq!(card.alternative_casts[0].name() == "Madness", native);
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
                game.turn.active_player = b;
                game.turn.priority_player = Some(a);
                game.turn.phase = ironsmith::Phase::Beginning;
                game.turn.step = Some(ironsmith::game_state::Step::Upkeep);
                let hand = game.create_object_from_definition(&card, a, Zone::Hand);
                let unrelated_exile = game.create_object_from_definition(&card, a, Zone::Exile);
                game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, u32::from(enough));
                let mut dm = Accept(accept);
                let discarded = ironsmith::events::processing::execute_discard(&mut game, hand, a, ironsmith::events::EventCause::from_effect(hand, a), false, Default::default(), &mut dm).unwrap().unwrap();
                assert_eq!(discarded.final_zone, Zone::Exile);
                let exiled = discarded.new_id.unwrap();
                assert_eq!(game.effect_store.pending_trigger_entries.len(), 1);
                assert!(!compute_legal_actions(&game, a).unwrap().iter().any(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == exiled || *spell_id == unrelated_exile)));
                // The linked permission overrides normal sorcery timing only
                // while this trigger resolves, retaining the complete price.
                MayCastForMadnessCostEffect::new().execute(&mut game, &mut EffectContext::new_default(exiled, a).with_decision_maker(&mut dm)).unwrap();
                let cast = accept && enough;
                assert_eq!(game.stack.len(), usize::from(cast));
                assert_eq!(game.player(a).unwrap().life, if cast { 18 } else { 20 });
                assert_eq!(game.player(a).unwrap().mana_pool.total(), u32::from(enough && !cast));
                assert_eq!(game.player(a).unwrap().graveyard.len(), usize::from(!cast));
                assert_eq!(game.object(unrelated_exile).unwrap().zone, Zone::Exile);
                if cast {
                    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                    assert_eq!(game.player(a).unwrap().life, 22);
                    assert_eq!(game.player(a).unwrap().graveyard.len(), 1);
                }
            }
        }
    }
}

#[test]
fn reminder_identity_exile_grants_each_owner_a_price_only_for_that_incarnation() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    for native in [false, true] {
        for life_price in [0, 2] {
        if native && life_price != 0 { continue; }
        for count in 0..=2 {
            let body = if native { "Airbend up to two target creatures.".to_owned() } else { format!("Exile up to two target creatures. While it's exiled, its owner may cast it for {{2}}{} rather than its mana cost.", if life_price == 0 { "" } else { ", Pay 2 life" }) };
            let mut builder = compiler::CardDefinitionBuilder::new(compiler::ids::CardId::new(), "Exile and permit");
            if !native { builder = builder.with_custom_keyword("Displace"); }
            let card = compile_builder_to_runtime_definition(builder, format!("Type: Sorcery\n{body}"), false).unwrap();
            let creature = compile_to_runtime_definition("Passenger", "Type: Creature\nMana cost: {9}\nPower/Toughness: 2/2", false).unwrap();
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
            let source = game.create_object_from_definition(&card, a, Zone::Stack);
            let targets: Vec<_> = (0..count).map(|_| game.create_object_from_definition(&creature, b, Zone::Battlefield)).collect();
            // Ownership, not the current controller, receives permission.
            for id in &targets { game.set_current_controller(*id, a).unwrap(); }
            let mut ctx = ironsmith::effects::EffectContext::new_default(source, a).with_targets(targets.iter().copied().map(ironsmith::effects::ResolvedTarget::Object).collect());
            let mut events = Vec::new();
            for effect in card.spell_effect.as_ref().unwrap().all_effects() { events.extend(ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap().events); }
            assert_eq!(game.exile.len(), count);
            assert_eq!(events.iter().filter(|event| event.downcast::<ironsmith::events::KeywordActionEvent>().is_some_and(|event| event.action == ironsmith::events::KeywordActionKind::Airbend)).count(), if native { count } else { 0 });
            for id in targets { assert!(game.object(id).is_none()); }
            let Some(&exiled) = game.exile.first() else { continue; };
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 2);
            game.player_mut(b).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 2);
            game.turn.active_player = a;
            game.turn.priority_player = Some(a);
            game.turn.phase = ironsmith::Phase::FirstMain;
            game.turn.step = None;
            let can_cast = |game: &GameState, player| compute_legal_actions(game, player).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == exiled));
            assert!(can_cast(&game, a).is_none());
            game.turn.priority_player = Some(b);
            assert!(can_cast(&game, b).is_none(), "normal timing applies");
            game.turn.active_player = b;
            assert!(can_cast(&game, b).is_some());
            let mut moved = game.clone();
            let hand = moved.move_object(exiled, Zone::Hand, ironsmith::events::EventCause::effect()).unwrap();
            let new_exile = moved.move_object(hand, Zone::Exile, ironsmith::events::EventCause::effect()).unwrap();
            assert!(!compute_legal_actions(&moved, b).unwrap().into_iter().any(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if spell_id == new_exile)));
            let action = can_cast(&game, b).unwrap();
            finish_reminder_cast(&mut game, action);
            assert_eq!(game.player(b).unwrap().mana_pool.total(), 0);
            assert_eq!(game.stack.len(), 1);
            assert_eq!(game.player(b).unwrap().life, 20 - life_price);
        }
    }
    }
}

#[test]
fn reminder_identity_defined_face_down_cast_and_turn_up_keep_native_identity_separate() {
    use ironsmith::decision::{compute_legal_actions, LegalAction, SelectFirstDecisionMaker};
    use ironsmith::alternative_cast::CastingMethod;
    use ironsmith::special_actions::{SpecialAction, TurnFaceUpMethod, can_perform_check, perform};
    use ironsmith::game_state::FaceDownCastKind;
    for native in [false, true] {
        for family in ["Morph", "Megamorph", "Disguise"] {
            let ward = family == "Disguise";
            let counter = family == "Megamorph";
            let body = if native { format!("{family} {{2}}, Pay 2 life") } else {
                format!("You may cast this card face down for {{3}} as a 2/2 creature{}. Turn it face up any time for {{2}}, Pay 2 life{}.", if ward { " with ward {2}" } else { "" }, if counter { " and put a +1/+1 counter on it" } else { "" })
            };
            let mut builder = compiler::CardDefinitionBuilder::new(compiler::ids::CardId::new(), "Hidden creature");
            if !native { builder = builder.with_custom_keyword("Veilform"); }
            let card = compile_builder_to_runtime_definition(builder, format!("Type: Creature\nMana cost: {{9}}\nPower/Toughness: 4/4\nFlying\n{body}"), false).unwrap();
            let expected_kind = if !native { FaceDownCastKind::Defined { ward } } else if ward { FaceDownCastKind::Disguise } else if counter { FaceDownCastKind::Megamorph } else { FaceDownCastKind::Morph };
            assert_eq!(FaceDownCastKind::of_abilities(&card.abilities), Some(expected_kind));
            assert_eq!(FaceDownCastKind::from_name(expected_kind.as_str()), Some(expected_kind));
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            game.turn.active_player = a;
            game.turn.priority_player = Some(a);
            game.turn.phase = ironsmith::Phase::FirstMain;
            game.turn.step = None;
            let hand = game.create_object_from_definition(&card, a, Zone::Hand);
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 3);
            let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, casting_method: CastingMethod::FaceDown, .. } if *spell_id == hand)).unwrap();
            finish_reminder_cast(&mut game, action);
            assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
            let spell = game.stack[0].object_id;
            assert!(game.is_face_down(spell));
            assert_eq!(game.calculated_power(spell), Some(2));
            assert_eq!(game.current_has_static_ability_id(spell, StaticAbilityId::Ward), ward);
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
            let permanent = game.battlefield[0];
            assert!(game.is_face_down(permanent));
            assert!(!game.current_has_static_ability_id(permanent, StaticAbilityId::Flying));
            let method = if !native { TurnFaceUpMethod::DefinedAbility } else if ward { TurnFaceUpMethod::DisguiseAbility } else if counter { TurnFaceUpMethod::MegamorphAbility } else { TurnFaceUpMethod::TurnFaceUpAbility };
            let action = SpecialAction::TurnFaceUp { permanent_id: permanent, method };
            // A native morph-cost tax must not see a custom identity. It also
            // excludes native Disguise, exactly as its explicit wording says.
            let tax = definition("Tax", "All morph costs cost {1} more.", None);
            game.create_object_from_definition(&tax, a, Zone::Battlefield);
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 2);
            let taxed = native && !ward;
            assert_eq!(can_perform_check(&action, &game, a).is_ok(), !taxed);
            if taxed { game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 1); }
            perform(action, &mut game, a, &mut SelectFirstDecisionMaker).unwrap();
            assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
            assert_eq!(game.player(a).unwrap().life, 18);
            assert!(!game.is_face_down(permanent));
            assert_eq!(game.counter_count(permanent, ironsmith::object::CounterType::PlusOnePlusOne), u32::from(counter));
            assert_eq!(game.calculated_power(permanent), Some(4 + i32::from(counter)));
            assert!(game.current_has_static_ability_id(permanent, StaticAbilityId::Flying));
            let native_id = if ward { StaticAbilityId::Disguise } else if counter { StaticAbilityId::Megamorph } else { StaticAbilityId::Morph };
            assert_eq!(game.current_has_static_ability_id(permanent, native_id), native);
        }
    }
}

#[test]
fn reminder_identity_phyrexian_life_payments_reduce_entry_loyalty_with_replacement_order() {
    use ironsmith::decision::{compute_legal_actions, LegalAction, SelectFirstDecisionMaker};
    use ironsmith::object::CounterType;
    struct HybridChoice { life_remaining: u32 }
    impl ironsmith::decision::DecisionMaker for HybridChoice {
        fn decide_options(&mut self, game: &GameState, ctx: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
            if ctx.options.iter().any(|option| option.description.contains("Phyrexian")) {
                let use_life = self.life_remaining > 0;
                if let Some(option) = ctx.options.iter().find(|option| option.legal && option.description.contains("Phyrexian") == use_life) {
                    if use_life { self.life_remaining -= 1; }
                    return vec![option.index];
                }
            }
            ironsmith::decision::DecisionMaker::decide_options(&mut SelectFirstDecisionMaker, game, ctx)
        }
    }
    for life_pips in 0..=2 {
        for doubled in [false, true] {
            let mut native_loyalty = None;
            for native in [true, false] {
                let text = if native { "Compleated" } else { "If this permanent would enter with one or more loyalty counters on it and the player who cast it paid life for Phyrexian mana symbols in its cost, it instead enters with two fewer loyalty counters for each of those symbols." };
                let mut builder = compiler::CardDefinitionBuilder::new(compiler::ids::CardId::new(), "Phyrexian walker");
                if !native { builder = builder.with_custom_keyword("Lifeformed"); }
                let card = compile_builder_to_runtime_definition(builder, format!("Type: Planeswalker — Jace\nMana cost: {{2}}{{G/P}}{{G/P}}\nLoyalty: 7\n{text}"), false).unwrap();
                assert_eq!(card.abilities.iter().any(|ability| matches!(&ability.kind, AbilityKind::Static(a) if a.keyword_marker_mechanic() == Some(ironsmith_core::KeywordMarkerMechanic::Compleated))), native);
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let a = PlayerId::from_index(0);
                game.turn.active_player = a;
                game.turn.priority_player = Some(a);
                game.turn.phase = ironsmith::Phase::FirstMain;
                game.turn.step = None;
                if doubled {
                    let mut doubler = definition("Doubler", "", None);
                    doubler.abilities.push(Ability::static_ability(StaticAbility::double_counters_replacement(ironsmith::target::ObjectFilter::default(), Some(CounterType::Loyalty), "Double loyalty".into())));
                    game.create_object_from_definition(&doubler, a, Zone::Battlefield);
                }
                let hand = game.create_object_from_definition(&card, a, Zone::Hand);
                game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 2);
                game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Green, 2 - life_pips);
                let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == hand)).unwrap();
                finish_reminder_cast_with(&mut game, action, &mut HybridChoice { life_remaining: life_pips });
                assert_eq!(game.player(a).unwrap().life, 20 - 2 * life_pips as i32, "native={native} life_pips={life_pips} doubled={doubled} mana={:?} stack={:?}", card.card.mana_cost, game.stack);
                assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
                let permanent = *game.battlefield.iter().find(|id| game.object(**id).unwrap().name == "Phyrexian walker").unwrap();
                let loyalty = game.counter_count(permanent, CounterType::Loyalty);
                if native { native_loyalty = Some(loyalty); } else { assert_eq!(Some(loyalty), native_loyalty); }
                if !doubled { assert_eq!(loyalty, 7 - 2 * life_pips); }
                else { assert!([14 - 2 * life_pips, 14 - 4 * life_pips].contains(&loyalty)); }
                let exiled = game.move_object(permanent, Zone::Exile, ironsmith::events::EventCause::effect()).unwrap();
                let returned = enter_reminder_permanent(&mut game, exiled);
                assert_eq!(game.counter_count(returned, CounterType::Loyalty), if doubled { 14 } else { 7 }, "payment belongs to the old incarnation");
            }
        }
    }
}

#[test]
fn reminder_identity_graveyard_price_can_tap_for_power_without_native_harmonize() {
    use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker, compute_legal_actions, LegalAction};
    use ironsmith::decisions::context::SelectOptionsContext;
    struct Tap(bool);
    impl DecisionMaker for Tap {
        fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            if ctx.description == "Choose a cost reduction resource" { return vec![usize::from(self.0)]; }
            SelectFirstDecisionMaker.decide_options(game, ctx)
        }
    }
    for native in [false, true] {
        for power in [-1i32, 0, 3, 8] {
            for tap in [false, true] {
                for colored in [false, true] {
                    let text = if native { "Harmonize {5}{G}, Pay 2 life" } else { "You may cast this card from your graveyard for {5}{G}, Pay 2 life. You may tap a creature you control to reduce that cost by an amount of generic mana equal to its power. Then exile this spell." };
                    let mut builder = compiler::CardDefinitionBuilder::new(compiler::ids::CardId::new(), "Returning spell");
                    if !native { builder = builder.with_custom_keyword("Resonate"); }
                    let card = compile_builder_to_runtime_definition(builder, format!("Type: Sorcery\nMana cost: {{9}}\nYou gain 4 life.\n{text}"), false).unwrap();
                    assert_eq!(card.alternative_casts[0].name() == "Harmonize", native);
                    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                    let a = PlayerId::from_index(0);
                    game.turn.active_player = a;
                    game.turn.priority_player = Some(a);
                    game.turn.phase = ironsmith::Phase::FirstMain;
                    game.turn.step = None;
                    let spell = game.create_object_from_definition(&card, a, Zone::Graveyard);
                    let donor = compile_to_runtime_definition("Donor", &format!("Type: Creature\nPower/Toughness: {power}/2"), false).unwrap();
                    let donor = game.create_object_from_definition(&donor, a, Zone::Battlefield);
                    // This payment has no tap symbol, so summoning sickness
                    // does not prohibit using the newly arrived creature.
                    let reduction = if tap { power.clamp(0, 5) as u32 } else { 0 };
                    game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 5 - reduction);
                    if colored { game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Green, 1); }
                    let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell));
                    assert_eq!(action.is_some(), colored, "the reduction cannot pay colored mana");
                    let Some(action) = action else { continue; };
                    finish_reminder_cast_with(&mut game, action, &mut Tap(tap));
                    assert_eq!(game.is_tapped(donor), tap);
                    assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
                    assert_eq!(game.player(a).unwrap().life, 18);
                    let stack_id = game.stack[0].object_id;
                    let mut bounced = game.clone();
                    let stable = bounced.object(stack_id).unwrap().stable_id;
                    let effect = ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(stack_id), Zone::Hand, false));
                    let mut context = ironsmith::effects::EffectContext::new_default(stack_id, a);
                    ironsmith::effects::execute_effect(&mut bounced, &effect, &mut context).unwrap();
                    let arrival = bounced.find_object_by_stable_id(stable).unwrap();
                    assert_eq!(bounced.object(arrival).unwrap().zone, Zone::Exile);
                    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
                    assert_eq!(game.player(a).unwrap().life, 22);
                    assert!(game.player(a).unwrap().graveyard.is_empty());
                    assert_eq!(game.exile.len(), 1);
                }
            }
        }
    }
}

#[test]
fn reminder_identity_filtered_material_payment_returns_linked_face_to_owner() {
    use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker, compute_legal_actions, LegalAction};
    use ironsmith::decisions::context::SelectObjectsContext;
    struct Materials;
    impl DecisionMaker for Materials {
        fn decide_objects(&mut self, _: &GameState, ctx: &SelectObjectsContext) -> Vec<ironsmith::ObjectId> { ctx.candidates.iter().filter(|c| c.legal).take(ctx.max.unwrap_or(usize::MAX)).map(|c| c.id).collect() }
    }
    for native in [false, true] {
        for extra in [false, true] {
            let text = if native { "Craft with four or more nonlands with activated abilities {8}{U}" } else { "{8}{U}, Exile this artifact, Exile four or more nonlands with activated abilities from among other permanents you control and/or cards in your graveyard: Return this card transformed under its owner's control. Activate only as a sorcery." };
            let mut builder = compiler::CardDefinitionBuilder::new(compiler::ids::CardId::new(), "Forge");
            if !native { builder = builder.with_custom_keyword("Reforge"); }
            let mut front = compile_builder_to_runtime_definition(builder, format!("Type: Artifact\nMana cost: {{U}}\n{text}"), false).unwrap();
            let mut back = compile_to_runtime_definition("Forge awake", "Type: Artifact Creature\nPower/Toughness: 5/5", false).unwrap();
            front.card.other_face = Some(back.card.id);
            front.card.other_face_name = Some(back.card.name.to_string());
            front.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
            front.card.transforming_dfc = true;
            back.card.other_face = Some(front.card.id);
            back.card.other_face_name = Some(front.card.name.to_string());
            back.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
            back.card.transforming_dfc = true;
            let activated = front.abilities.iter().find_map(|ability| match &ability.kind { AbilityKind::Activated(a) => Some(a), _ => None }).unwrap();
            assert_eq!(activated.keyword == Some(ironsmith_core::ActivatedAbilityKeyword::Craft), native);
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
            game.register_linked_face_definition(&front);
            game.register_linked_face_definition(&back);
            game.turn.active_player = a;
            game.turn.priority_player = Some(a);
            game.turn.phase = ironsmith::Phase::FirstMain;
            game.turn.step = None;
            let source = game.create_object_from_definition(&front, b, Zone::Battlefield);
            game.set_current_controller(source, a).unwrap();
            let material = compile_to_runtime_definition("Material", "Type: Artifact\n{0}: You gain 1 life.", false).unwrap();
            let land = compile_to_runtime_definition("Land", "Type: Land\n{T}: Add {C}.", false).unwrap();
            let blank = compile_to_runtime_definition("Blank", "Type: Artifact", false).unwrap();
            let excluded = vec![
                game.create_object_from_definition(&land, a, Zone::Battlefield),
                game.create_object_from_definition(&blank, a, Zone::Graveyard),
                game.create_object_from_definition(&material, b, Zone::Battlefield),
                game.create_object_from_definition(&material, b, Zone::Graveyard),
                game.create_object_from_definition(&material, a, Zone::Hand),
            ];
            let mut paid = vec![game.create_object_from_definition(&material, a, Zone::Battlefield), game.create_object_from_definition(&material, a, Zone::Graveyard)];
            let stolen = game.create_object_from_definition(&material, b, Zone::Battlefield);
            game.set_current_controller(stolen, a).unwrap(); paid.push(stolen);
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue, 9);
            let action = |game: &GameState| compute_legal_actions(game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source));
            assert!(action(&game).is_none(), "the source and excluded cards cannot supply the fourth material");
            paid.push(game.create_object_from_definition(&material, a, Zone::Graveyard));
            if extra { paid.push(game.create_object_from_definition(&material, a, Zone::Battlefield)); }
            game.turn.active_player = b;
            assert!(action(&game).is_none());
            game.turn.active_player = a;
            let action = action(&game).unwrap();
            finish_reminder_cast_with(&mut game, action, &mut Materials);
            assert!(game.object(source).is_none());
            for id in paid { assert!(game.object(id).is_none(), "native={native} extra={extra} remaining={id:?} ability={activated:#?}"); }
            assert_eq!(game.exile.len(), if extra { 6 } else { 5 });
            for id in excluded { assert!(game.object(id).is_some()); }
            assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
            let returned = *game.battlefield.iter().find(|id| game.object(**id).unwrap().name == "Forge awake").unwrap();
            assert_eq!(game.controller_of_id(returned), Some(b));
            assert_eq!(game.calculated_power(returned), Some(5));
            assert_eq!(game.exile.len(), if extra { 5 } else { 4 });
        }
    }
}

#[test]
fn reminder_identity_granted_combat_poison_belongs_to_each_recipient() {
    use ironsmith::events::{DamageEvent, DamageTarget};
    use ironsmith::events::cause::EventCause;
    use ironsmith::triggers::{TriggerEvent, TriggerQueue};
    for native in [false, true] {
        for copies in [1, 2] {
            let rules = if native { "All Sliver creatures have poisonous 1." } else { "All Sliver creatures have \"Whenever this creature deals combat damage to a player, that player gets 1 poison counter.\"" };
            let grant = definition("Poison Grant", rules, (!native).then_some("Venom"));
            let mut sliver = definition("Recipient", "", None);
            sliver.card.subtypes = vec![ironsmith::types::Subtype::Sliver];
            let plain = definition("Unrelated", "", None);
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            let b = PlayerId::from_index(1);
            for _ in 0..copies { game.create_object_from_definition(&grant, a, Zone::Battlefield); }
            let source = game.create_object_from_definition(&sliver, b, Zone::Battlefield);
            let other = game.create_object_from_definition(&plain, b, Zone::Battlefield);
            for (attacker, target, combat, expected) in [
                (source, DamageTarget::Player(a), true, copies),
                (source, DamageTarget::Player(a), false, 0),
                (source, DamageTarget::Object(other), true, 0),
                (other, DamageTarget::Player(a), true, 0),
            ] {
                let event = TriggerEvent::new_with_provenance(DamageEvent::with_cause(attacker, target, 1, combat, if combat { EventCause::combat_damage(attacker) } else { EventCause::effect() }), ironsmith::provenance::ProvNodeId::default());
                let entries = ironsmith::triggers::check_triggers(&game, &event);
                assert_eq!(entries.len(), expected);
                let mut queue = TriggerQueue::new();
                for entry in entries {
                    assert_eq!(entry.source, source);
                    assert_eq!(entry.controller, b, "grantor does not control the recipient's trigger");
                    assert_eq!(entry.ability.presentation_label.is_some(), native, "native={native} ability={:?}", entry.ability);
                    queue.add(entry);
                }
                ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                while !game.stack_is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
            }
            assert_eq!(game.player(a).unwrap().poison_counters, copies as u32);
            assert_eq!(game.player(b).unwrap().poison_counters, 0);
        }
    }
}

#[test]
fn reminder_identity_draw_discard_counter_uses_discard_receipt_and_current_controller() {
    use ironsmith::object::CounterType;
    for native in [false, true] {
        for land in [false, true] {
            for departed in [false, true] {
                let rules = if native { "{0}: This creature connives." } else { "{0}: Draw a card, then discard a card. If you discarded a nonland card, put a +1/+1 counter on this creature." };
                let card = definition("Discerning", rules, (!native).then_some("Discern"));
                let activation = card.abilities.iter().find_map(|a| match &a.kind { AbilityKind::Activated(a) => Some(a), _ => None }).unwrap();
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let a = PlayerId::from_index(0);
                let b = PlayerId::from_index(1);
                let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
                let mut drawn = definition("Drawn", "", None);
                if land { drawn.card.card_types = vec![ironsmith::types::CardType::Land]; }
                game.create_object_from_definition(&drawn, b, Zone::Library);
                let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(source).unwrap(), &game);
                game.set_current_controller(source, b).unwrap();
                let returned = if departed {
                    let hand = game.move_object(source, Zone::Hand, ironsmith::events::EventCause::effect()).unwrap();
                    Some(game.move_object(hand, Zone::Battlefield, ironsmith::events::EventCause::effect()).unwrap())
                } else { None };
                let mut ctx = ironsmith::effects::EffectContext::new_default(source, a).with_source_snapshot(snapshot);
                let mut events = Vec::new();
                for effect in &activation.effects { events.extend(ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap().events); }
                assert!(game.player(b).unwrap().library.is_empty());
                assert!(game.player(b).unwrap().hand.is_empty());
                assert_eq!(game.player(b).unwrap().graveyard.len(), 1);
                assert!(game.player(a).unwrap().graveyard.is_empty());
                let live = returned.unwrap_or(source);
                assert_eq!(game.object(live).unwrap().counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0), u32::from(!land && !departed));
                assert_eq!(events.iter().filter(|event| event.downcast::<ironsmith::events::KeywordActionEvent>().is_some_and(|event| event.action == ironsmith::events::KeywordActionKind::Connive)).count(), usize::from(native));
            }
        }
    }
}

#[test]
fn reminder_identity_day_cycle_transforms_and_restricts_manual_changes_without_keyword_identity() {
    use ironsmith::effects::{EffectContext, TransformEffect, EffectExecutor};
    use ironsmith::ChooseSpec;
    const DAY: &str = "If it's neither day nor night, it becomes day. If it's night, this permanent enters transformed. As it becomes night, if this permanent is front face up, transform it. This permanent can't transform except due to this ability. If a player casts no spells during their own turn, it becomes night next turn.";
    const NIGHT: &str = "If it's neither day nor night and no permanent establishes day, it becomes night. As it becomes day, if this permanent is back face up, transform it. This permanent can't transform except due to this ability. If a player casts at least two spells during their own turn, it becomes day next turn.";
    for native in [false, true] {
        for enters_at_night in [false, true] {
            let mut front = definition("Day Face", if native { "Daybound" } else { DAY }, (!native).then_some("Sunward"));
            let mut back = definition("Night Face", if native { "Nightbound" } else { NIGHT }, (!native).then_some("Moonward"));
            front.card.other_face = Some(back.card.id); front.card.other_face_name = Some(back.card.name.to_string());
            back.card.other_face = Some(front.card.id); back.card.other_face_name = Some(front.card.name.to_string());
            for card in [&mut front.card, &mut back.card] { card.transforming_dfc = true; card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike; }
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            game.register_linked_face_definition(&front); game.register_linked_face_definition(&back);
            if enters_at_night { game.set_daytime(false); }
            let hand = game.create_object_from_definition(&front, a, Zone::Hand);
            let source = game.move_object(hand, Zone::Battlefield, ironsmith::events::EventCause::effect()).unwrap();
            assert!(game.has_day_night());
            assert_eq!(game.is_transformed_permanent(source), enters_at_night);
            for daytime in [false, true, false] {
                game.set_daytime(daytime);
                game.try_apply_day_nightbound_transformations().unwrap();
                assert_eq!(game.is_transformed_permanent(source), !daytime);
                let identity = if daytime { StaticAbilityId::Daybound } else { StaticAbilityId::Nightbound };
                assert_eq!(game.current_has_static_ability_id(source, identity), native);
                let mut ctx = EffectContext::new_default(source, a);
                TransformEffect::new(ChooseSpec::Source).execute(&mut game, &mut ctx).unwrap();
                assert_eq!(game.is_transformed_permanent(source), !daytime, "only the day/night procedure may transform it");
            }
        }
        // The back-face definition can start the designation even without a
        // paired card; it does not claim the official Nightbound identity.
        let back = definition("Night source", if native { "Nightbound" } else { NIGHT }, (!native).then_some("Moonward"));
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.create_object_from_definition(&back, PlayerId::from_index(0), Zone::Battlefield);
        game.try_apply_day_nightbound_transformations().unwrap();
        assert!(game.has_day_night()); assert!(game.is_night);
    }
}

#[test]
fn reminder_identity_library_comparison_preserves_winner_and_reorders_without_clash_events() {
    use ironsmith::decision::DecisionMaker;
    use ironsmith::decisions::context::PartitionContext;
    struct Bottom(bool);
    impl DecisionMaker for Bottom {
        fn decide_partition(&mut self, game: &GameState, ctx: &PartitionContext) -> Vec<ironsmith::ObjectId> {
            if self.0 { game.player(ctx.player).unwrap().library.last().copied().into_iter().collect() } else { Vec::new() }
        }
    }
    for native in [false, true] {
        for values in [(Some(5), Some(2)), (Some(2), Some(5)), (Some(2), Some(2)), (Some(0), None), (None, Some(0)), (None, None)] {
            for bottom in [false, true] {
                let text = if native { "Clash with defending player." } else { "You and defending player each reveal the top card of your library, then each puts that card on their choice of the top or bottom. A player wins if their card had a greater mana value." };
                let card = compile_to_runtime_definition("Compare", &format!("Type: Sorcery\n{text}"), false).unwrap();
                let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
                let a = PlayerId::from_index(0); let b = PlayerId::from_index(1); let c = PlayerId::from_index(2);
                let source = game.create_object_from_definition(&card, a, Zone::Stack);
                let mut top_ids = Vec::new();
                for (player, value) in [(a, values.0), (b, Some(99)), (c, values.1)] {
                    if let Some(value) = value {
                        let lower = definition("Lower", "", None);
                        game.create_object_from_definition(&lower, player, Zone::Library);
                        let upper = compile_to_runtime_definition("Upper", &format!("Type: Artifact\nMana cost: {{{value}}}"), false).unwrap();
                        top_ids.push((player, game.create_object_from_definition(&upper, player, Zone::Library)));
                    }
                }
                let mut dm = Bottom(bottom);
                let mut ctx = ironsmith::effects::EffectContext::new_default(source, a).with_decision_maker(&mut dm);
                ctx.combat.defending_player = Some(c);
                let mut events = Vec::new(); let mut result = None;
                for effect in card.spell_effect.as_ref().unwrap().all_effects() {
                    let outcome = ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
                    events.extend(outcome.events); result = Some(outcome.value);
                }
                let won = match values { (Some(left), Some(right)) => left > right, (Some(_), None) => true, _ => false };
                assert_eq!(result, Some(ironsmith::effect::OutcomeValue::Count(i64::from(won))));
                for (player, top) in top_ids {
                    let library = &game.player(player).unwrap().library;
                    assert_eq!(if bottom && player != b { library.first() } else { library.last() }, Some(&top));
                }
                assert_eq!(events.iter().filter(|event| event.downcast::<ironsmith::events::CardRevealedEvent>().is_some()).count(), usize::from(values.0.is_some()) + usize::from(values.1.is_some()));
                assert_eq!(events.iter().filter(|event| event.downcast::<ironsmith::events::KeywordActionEvent>().is_some_and(|event| event.action == ironsmith::events::KeywordActionKind::Clash)).count(), if native { 2 } else { 0 });
            }
        }
    }
}

#[test]
fn reminder_identity_animated_land_returns_exact_incarnation_to_instruction_controller() {
    use ironsmith::object::CounterType;
    use ironsmith::decision::SelectFirstDecisionMaker;
    for native in [false, true] {
        for destination in [Zone::Graveyard, Zone::Exile, Zone::Hand] {
            for move_again in [false, true] {
                let text = if native { "Earthbend 4." } else { "Target land you control becomes a 0/0 creature with haste that's still a land. Put four +1/+1 counters on it. When it dies or is exiled, return it to the battlefield tapped under your control." };
                let card = compile_to_runtime_definition("Animate land", &format!("Type: Sorcery\n{text}"), false).unwrap();
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
                let source = game.create_object_from_definition(&card, a, Zone::Stack);
                let land = compile_to_runtime_definition("Borrowed land", "Type: Artifact Land\n{T}: Add {G}.", false).unwrap();
                let target = game.create_object_from_definition(&land, b, Zone::Battlefield);
                game.set_current_controller(target, a).unwrap();
                let stable = game.object(target).unwrap().stable_id;
                let mut ctx = ironsmith::effects::EffectContext::new_default(source, a).with_targets(vec![ironsmith::effects::ResolvedTarget::Object(target)]);
                let mut events = Vec::new();
                for effect in card.spell_effect.as_ref().unwrap().all_effects() { events.extend(ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap().events); }
                assert_eq!(game.calculated_power(target), Some(4));
                assert_eq!(game.counter_count(target, CounterType::PlusOnePlusOne), 4);
                assert!(game.current_has_static_ability_id(target, StaticAbilityId::Haste));
                let current = game.current_characteristics(target).unwrap();
                assert!(current.card_types.contains(&ironsmith::types::CardType::Artifact));
                assert!(current.card_types.contains(&ironsmith::types::CardType::Land));
                assert_eq!(events.iter().any(|event| event.downcast::<ironsmith::events::KeywordActionEvent>().is_some_and(|event| event.action == ironsmith::events::KeywordActionKind::Earthbend)), native);
                game.take_pending_trigger_events();
                let departed = game.move_object(target, destination, ironsmith::events::EventCause::effect()).unwrap();
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
                assert_eq!(queue.entries.len(), usize::from(destination != Zone::Hand));
                if move_again { game.move_object(departed, if destination == Zone::Hand { Zone::Exile } else { Zone::Hand }, ironsmith::events::EventCause::effect()); }
                ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                while !game.stack_is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
                let returned = game.battlefield.iter().copied().find(|id| game.object(*id).unwrap().stable_id == stable);
                assert_eq!(returned.is_some(), destination != Zone::Hand && !move_again);
                if let Some(returned) = returned {
                    assert_eq!(game.controller_of_id(returned), Some(a));
                    assert_eq!(game.object(returned).unwrap().owner, b);
                    assert!(game.is_tapped(returned));
                    assert_eq!(game.counter_count(returned, CounterType::PlusOnePlusOne), 0);
                    assert_eq!(game.calculated_power(returned), None);
                }
            }
        }
    }
}

#[test]
fn reminder_identity_other_player_payment_uses_only_generic_mana_and_one_helper() {
    use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker, compute_legal_actions, LegalAction};
    use ironsmith::decisions::context::SelectOptionsContext;
    struct Helper;
    impl DecisionMaker for Helper {
        fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            if ctx.description.starts_with("Choose another player to assist") {
                return ctx.options.iter().find(|o| o.legal && o.index == 2).map(|o| vec![o.index]).unwrap_or_default();
            }
            if ctx.description.starts_with("Choose how much generic mana") {
                assert_eq!(ctx.player, PlayerId::from_index(2));
                return ctx.options.iter().rev().find(|o| o.legal).map(|o| vec![o.index]).unwrap_or_default();
            }
            SelectFirstDecisionMaker.decide_options(game, ctx)
        }
    }
    for native in [false, true] {
        for colored in [false, true] {
            let rules = if native { "Assist" } else { "As you cast this spell, before activating mana abilities, you may choose another player. That player may activate mana abilities before you do and may pay any amount of the generic mana in this spell's total cost before you pay the rest." };
            let mut builder = compiler::CardDefinitionBuilder::new(compiler::ids::CardId::new(), "Shared payment");
            if !native { builder = builder.with_custom_keyword("Collaborate"); }
            let card = compile_builder_to_runtime_definition(builder, format!("Type: Sorcery\nMana cost: {{2}}{{U}}\n{rules}\nYou gain 3 life."), false).unwrap();
            let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
            let a = PlayerId::from_index(0); let b = PlayerId::from_index(1); let c = PlayerId::from_index(2);
            game.turn.active_player = a; game.turn.priority_player = Some(a); game.turn.phase = ironsmith::Phase::FirstMain; game.turn.step = None;
            let spell = game.create_object_from_definition(&card, a, Zone::Hand);
            if colored { game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue, 1); }
            for helper in [b, c] { game.player_mut(helper).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue, 3); }
            assert_eq!(game.current_has_static_ability_id(spell, StaticAbilityId::Assist), native);
            let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell));
            assert_eq!(action.is_some(), colored, "helper's mana cannot satisfy the blue symbol");
            let Some(action) = action else { continue; };
            finish_reminder_cast_with(&mut game, action, &mut Helper);
            assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
            assert_eq!(game.player(b).unwrap().mana_pool.total(), 3);
            assert_eq!(game.player(c).unwrap().mana_pool.total(), 1);
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            assert_eq!(game.player(a).unwrap().life, 23);
        }
    }
}

#[test]
fn reminder_identity_costed_modes_pay_each_selected_price_without_named_identity() {
    use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker, compute_legal_actions, LegalAction};
    use ironsmith::decisions::context::SelectOptionsContext;
    struct Modes(Vec<usize>);
    impl DecisionMaker for Modes {
        fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            if ctx.description.starts_with("Choose mode for") { return self.0.clone(); }
            SelectFirstDecisionMaker.decide_options(game, ctx)
        }
    }
    for native in [false, true] {
        for tiered in [false, true] {
            for modes in [vec![0], vec![1], vec![0, 1]] {
                if tiered && modes.len() > 1 { continue; }
                let header = match (native, tiered) { (true, true) => "Tiered", (true, false) => "Spree", (false, true) => "Choose one additional cost.", (false, false) => "Choose one or more additional costs." };
                let branches = if tiered { "• Small — {1} — You gain 2 life.\n• Large — {2} — You gain 5 life." } else { "+ {1} — You gain 2 life.\n+ {2} — You gain 5 life." };
                let mut builder = compiler::CardDefinitionBuilder::new(compiler::ids::CardId::new(), "Paid modes");
                if !native { builder = builder.with_custom_keyword("Options"); }
                let card = compile_builder_to_runtime_definition(builder, format!("Type: Sorcery\nMana cost: {{R}}\n{header}\n{branches}"), false).unwrap();
                let modal = card.spell_effect.as_ref().unwrap().all_effects().into_iter().find_map(|effect| effect.downcast_ref::<ironsmith::effects::ChooseModeEffect>()).unwrap();
                assert_eq!(modal.descriptive_mode_costs, !native);
                assert_eq!(modal.min_choose_count, ironsmith::Value::Fixed(1));
                assert_eq!(modal.choose_count, ironsmith::Value::Fixed(if tiered { 1 } else { 2 }));
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let a = PlayerId::from_index(0);
                game.turn.active_player = a; game.turn.priority_player = Some(a); game.turn.phase = ironsmith::Phase::FirstMain; game.turn.step = None;
                let spell = game.create_object_from_definition(&card, a, Zone::Hand);
                game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Red, 1);
                let cast = |game: &GameState| compute_legal_actions(game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell));
                assert!(cast(&game).is_none(), "at least one additional price is mandatory");
                let price = modes.iter().map(|mode| *mode as u32 + 1).sum::<u32>();
                game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, price);
                let action = cast(&game).unwrap();
                finish_reminder_cast_with(&mut game, action, &mut Modes(modes.clone()));
                assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
                assert_eq!(game.stack[0].chosen_modes.as_ref(), Some(&modes));
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                assert_eq!(game.player(a).unwrap().life, 20 + modes.iter().map(|mode| if *mode == 0 { 2 } else { 5 }).sum::<i32>());
            }
        }
    }
}

#[test]
fn reminder_identity_single_card_look_and_defined_token_execute_both_parts() {
    use ironsmith::decision::{DecisionMaker, compute_legal_actions, LegalAction};
    use ironsmith::decisions::context::{PartitionContext, SelectObjectsContext};
    struct Bottom(Vec<ironsmith::ObjectId>);
    impl DecisionMaker for Bottom {
        fn decide_partition(&mut self, _: &GameState, _: &PartitionContext) -> Vec<ironsmith::ObjectId> { self.0.clone() }
        fn decide_objects(&mut self, _: &GameState, ctx: &SelectObjectsContext) -> Vec<ironsmith::ObjectId> { self.0.iter().copied().filter(|id| ctx.candidates.iter().any(|c| c.id == *id)).collect() }
    }
    for native in [false, true] {
        for count in 0..=2 {
            for bottom in [false, true] {
                let rules = if native { "Scry 1 and create a Treasure token." } else { "To ponder 1, look at the top card of your library. You may put that card on the bottom. Create a colorless Treasure artifact token with \"{T}, Sacrifice this token: Add one mana of any color.\"." };
                let card = compile_to_runtime_definition("Look and make", &format!("Type: Sorcery\n{rules}"), false).unwrap();
                let a = PlayerId::from_index(0);
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                game.turn.priority_player = Some(a);
                let source = game.create_object_from_definition(&card, a, Zone::Stack);
                for n in 0..count { game.create_object_from_definition(&definition(&format!("Library {n}"), "", None), a, Zone::Library); }
                let top = game.player(a).unwrap().library.last().copied();
                let mut dm = Bottom(if bottom { top.into_iter().collect() } else { Vec::new() });
                let mut ctx = ironsmith::effects::EffectContext::new(source, a, &mut dm);
                let mut events = Vec::new();
                for effect in card.spell_effect.as_ref().unwrap().all_effects() { events.extend(ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap().events); }
                if let Some(top) = top { assert_eq!(if bottom { game.player(a).unwrap().library.first() } else { game.player(a).unwrap().library.last() }, Some(&top)); }
                assert_eq!(game.battlefield.len(), 1);
                let treasure = game.battlefield[0];
                assert!(game.object(treasure).unwrap().subtypes.contains(&ironsmith::types::Subtype::Treasure));
                assert!(events.iter().all(|event| native || !event.downcast::<ironsmith::events::KeywordActionEvent>().is_some_and(|event| event.action == ironsmith::events::KeywordActionKind::Scry)));
                let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source, .. } | LegalAction::ActivateManaAbility { source, .. } if *source == treasure)).unwrap();
                finish_reminder_cast(&mut game, action);
                assert!(!game.battlefield.contains(&treasure));
                assert_eq!(game.player(a).unwrap().mana_pool.total(), 1);
            }
        }
    }
}

#[test]
fn reminder_identity_army_creation_selection_and_growth_use_the_same_transaction() {
    use ironsmith::decision::DecisionMaker;
    use ironsmith::decisions::context::SelectObjectsContext;
    use ironsmith::object::CounterType;
    struct ChooseLast;
    impl DecisionMaker for ChooseLast {
        fn decide_objects(&mut self, _: &GameState, ctx: &SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            ctx.candidates.iter().rev().filter(|candidate| candidate.legal).take(ctx.min).map(|candidate| candidate.id).collect()
        }
    }
    for native in [false, true] {
        for (subtype, plural) in [(ironsmith::types::Subtype::Zombie, "Zombies"), (ironsmith::types::Subtype::Orc, "Orcs")] {
            for amount in [0, 2, 5] {
                for existing in 0..=2 {
                    let rules = if native { format!("Amass {plural} {amount}.") } else { format!("Put {amount} +1/+1 counters on an Army creature you control. It's also a {subtype}. If you don't control an Army creature, create a 0/0 black {subtype} Army creature token first.") };
                    let card = compile_to_runtime_definition("Grow army", &format!("Type: Sorcery\n{rules}"), false).unwrap();
                    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                    let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
                    let source = game.create_object_from_definition(&card, a, Zone::Stack);
                    let mut army = definition("Existing army", "Shroud", None);
                    army.card.subtypes = vec![ironsmith::types::Subtype::Army, ironsmith::types::Subtype::Goblin];
                    let opponent = game.create_object_from_definition(&army, b, Zone::Battlefield);
                    let mut decoy = army.clone(); decoy.card.card_types = vec![ironsmith::types::CardType::Artifact];
                    let decoy = game.create_object_from_definition(&decoy, a, Zone::Battlefield);
                    let own = (0..existing).map(|_| game.create_object_from_definition(&army, a, Zone::Battlefield)).collect::<Vec<_>>();
                    let mut dm = ChooseLast;
                    let mut ctx = ironsmith::effects::EffectContext::new(source, a, &mut dm);
                    let mut events = Vec::new();
                    for effect in card.spell_effect.as_ref().unwrap().all_effects() { events.extend(ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap().events); }
                    let chosen = own.last().copied().unwrap_or_else(|| *game.battlefield.last().unwrap());
                    assert_eq!(game.battlefield.len(), 2 + existing.max(1));
                    assert_eq!(game.counter_count(chosen, CounterType::PlusOnePlusOne), amount as u32);
                    assert!(game.calculated_subtypes(chosen).contains(&subtype));
                    assert_eq!(game.calculated_power(chosen), Some(amount + if existing > 0 { 2 } else { 0 }));
                    if existing > 0 { assert!(game.calculated_subtypes(chosen).contains(&ironsmith::types::Subtype::Goblin)); }
                    for untouched in [opponent, decoy].into_iter().chain(own.iter().copied().filter(|id| *id != chosen)) {
                        assert_eq!(game.counter_count(untouched, CounterType::PlusOnePlusOne), 0);
                        assert!(!game.calculated_subtypes(untouched).contains(&subtype));
                    }
                    assert_eq!(events.iter().filter(|event| event.downcast::<ironsmith::events::KeywordActionEvent>().is_some_and(|event| event.action == ironsmith::events::KeywordActionKind::Amass)).count(), usize::from(native));
                }
            }
        }
    }
}

#[test]
fn reminder_identity_permanent_designation_enables_linked_ability_without_action_identity() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    for native in [false, true] {
        let behavior = if native { "Harness this permanent." } else { "If this permanent isn't harnessed, it becomes harnessed." };
        let mut builder = compiler::CardDefinitionBuilder::new(compiler::ids::CardId::new(), "Designation artifact");
        if !native { builder = builder.with_custom_keyword("Unseal"); }
        let card = compile_builder_to_runtime_definition(builder, format!("Type: Artifact\n{{0}}: {behavior}\n∞ — {{T}}: Add {{G}}."), false).unwrap();
        let activation = card.abilities.iter().find_map(|a| match &a.kind { AbilityKind::Activated(a) => Some(a), _ => None }).unwrap();
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
        game.turn.priority_player = Some(a);
        let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
        let activations = |game: &GameState, player, id| compute_legal_actions(game, player).unwrap().into_iter().filter(|action| matches!(action, LegalAction::ActivateAbility { source, .. } | LegalAction::ActivateManaAbility { source, .. } if *source == id)).collect::<Vec<_>>();
        assert_eq!(activations(&game, a, source).len(), 1);
        for repetition in 0..2 {
            let mut ctx = ironsmith::effects::EffectContext::new_default(source, a);
            let mut events = Vec::new();
            for effect in activation.effects.all_effects() { events.extend(ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap().events); }
            assert!(game.is_harnessed(source));
            assert_eq!(events.iter().filter(|event| event.downcast::<ironsmith::events::KeywordActionEvent>().is_some_and(|event| event.action == ironsmith::events::KeywordActionKind::Harness)).count(), usize::from(native && repetition == 0));
            assert_eq!(activations(&game, a, source).len(), 2);
        }
        game.set_current_controller(source, b).unwrap();
        game.turn.priority_player = Some(b);
        assert!(game.is_harnessed(source));
        assert_eq!(activations(&game, b, source).len(), 2);
        let plain = game.create_object_from_definition(&card, b, Zone::Battlefield);
        assert!(!game.is_harnessed(plain));
        let hand = game.move_object(source, Zone::Hand, ironsmith::events::EventCause::effect()).unwrap();
        let returned = game.move_object(hand, Zone::Battlefield, ironsmith::events::EventCause::effect()).unwrap();
        assert!(!game.is_harnessed(source)); assert!(!game.is_harnessed(returned));
        game.turn.priority_player = Some(a);
        assert_eq!(activations(&game, a, returned).len(), 1);
    }
}

#[test]
fn reminder_identity_chapter_choice_skips_only_entry_turn_thresholds() {
    use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
    use ironsmith::decisions::context::SelectOptionsContext;
    use ironsmith::{Effect, ChooseSpec, StackEntry};
    use ironsmith::effects::{MoveToZoneEffect, RemoveCountersEffect, PutCountersEffect};
    use ironsmith::object::CounterType;
    struct Chapter(usize);
    impl DecisionMaker for Chapter {
        fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            if ctx.options.first().is_some_and(|option| option.description.starts_with("Chapter ")) { return vec![self.0 - 1]; }
            SelectFirstDecisionMaker.decide_options(game, ctx)
        }
    }
    for native in [false, true] {
        for chapter in 1usize..=3 {
            for later_turn in [false, true] {
                let rules = if native { "Read ahead" } else { "As this Saga enters, choose a number between one and its final chapter number. It enters with the chosen number of lore counters. Chapter abilities of this Saga can't trigger the turn it entered unless it has exactly the number of lore counters specified in that chapter's symbol." };
                let card = compile_to_runtime_definition("Chosen chapter", &format!("Type: Enchantment — Saga\n{rules}\nI — You gain 1 life.\nII — You gain 2 life.\nIII — You gain 3 life."), false).unwrap();
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let a = PlayerId::from_index(0);
                let hand = game.create_object_from_definition(&card, a, Zone::Hand);
                let stable = game.object(hand).unwrap().stable_id;
                let source = game.create_object_from_definition(&definition("Mover", "", None), a, Zone::Battlefield);
                game.stack.push(StackEntry::ability(source, a, vec![Effect::new(MoveToZoneEffect::new(ChooseSpec::SpecificObject(hand), Zone::Battlefield, false))]));
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut Chapter(chapter)).unwrap();
                let saga = game.find_object_by_stable_id(stable).unwrap();
                assert_eq!(game.counter_count(saga, CounterType::Lore), chapter as u32);
                assert_eq!(game.current_has_static_ability_id(saga, StaticAbilityId::ReadAhead), native);
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
                assert_eq!(queue.entries.len(), 1);
                ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                while !game.stack_is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
                assert_eq!(game.player(a).unwrap().life, 20 + chapter as i32);
                if later_turn { game.next_turn(); }
                game.stack.push(StackEntry::ability(source, a, vec![
                    Effect::new(RemoveCountersEffect::new(CounterType::Lore, chapter as i32, ChooseSpec::SpecificObject(saga))),
                    Effect::new(PutCountersEffect::new(CounterType::Lore, 3, ChooseSpec::SpecificObject(saga))),
                ]));
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
                assert_eq!(queue.entries.len(), if later_turn { 3 } else { 1 });
                ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                while !game.stack_is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
                assert_eq!(game.player(a).unwrap().life, 20 + chapter as i32 + if later_turn { 6 } else { 3 });
            }
        }
    }
}

#[test]
fn reminder_identity_text_substitution_precedes_target_selection_and_pays_the_full_price() {
    use ironsmith::decision::{DecisionMaker, compute_legal_actions, LegalAction};
    struct Pick(ironsmith::ObjectId);
    impl DecisionMaker for Pick {
        fn decide_targets(&mut self, _: &GameState, _: &ironsmith::decisions::context::TargetsContext) -> Vec<ironsmith::Target> { vec![ironsmith::Target::Object(self.0)] }
    }
    for native in [false, true] {
        for cleave in [false, true] {
            for life_price in [false, true] {
                if native && life_price { continue; }
                let keyword = if cleave { "Cleave" } else { "Overload" };
                let price = if life_price { "{1}, Pay 2 life" } else { "{1}" };
                let rule = if native { format!("{keyword} {{1}}") } else {
                    format!("You may cast this spell for {price}. If you do, {}", if cleave { "remove the words in square brackets." } else { "change its text by replacing all instances of \"target\" with \"each.\"" })
                };
                let body = if cleave { "Return target creature [you control] to its owner's hand." } else { "Return target creature you don't control to its owner's hand." };
                let mut builder = compiler::CardDefinitionBuilder::new(compiler::ids::CardId::new(), "Rewrite spell");
                if !native { builder = builder.with_custom_keyword("Rewrite"); }
                let card = compile_builder_to_runtime_definition(builder, format!("Type: Sorcery\nMana cost: {{9}}\n{rule}\n{body}"), false).unwrap();
                assert_eq!(card.alternative_casts[0].name() == keyword, native);
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
                game.turn.active_player = a; game.turn.priority_player = Some(a); game.turn.phase = ironsmith::Phase::FirstMain; game.turn.step = None;
                let spell = game.create_object_from_definition(&card, a, Zone::Hand);
                let opponent = game.create_object_from_definition(&definition("Opponent", "", None), b, Zone::Battlefield);
                let protected = game.create_object_from_definition(&definition("Protected", "Shroud", None), b, Zone::Battlefield);
                let own = game.create_object_from_definition(&definition("Own", "", None), a, Zone::Battlefield);
                game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 1);
                let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)).unwrap();
                finish_reminder_cast_with(&mut game, action, &mut Pick(opponent));
                assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
                assert_eq!(game.player(a).unwrap().life, if life_price { 18 } else { 20 });
                assert_eq!(game.stack[0].targets.len(), usize::from(cleave));
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                assert!(!game.battlefield.contains(&opponent), "native={native} cleave={cleave} methods={:?}", card.alternative_casts);
                assert_eq!(game.battlefield.contains(&protected), cleave, "the each rewrite removes targeting, including shroud checks");
                assert!(game.battlefield.contains(&own));
            }
        }
    }
}

#[test]
fn reminder_identity_optional_source_copies_belong_to_the_chosen_players() {
    use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker, LegalAction, compute_legal_actions};
    use ironsmith::decisions::context::{BooleanContext, SelectOptionsContext};
    struct Copies(bool);
    impl DecisionMaker for Copies {
        fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool { self.0 }
        fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            if let Some(option) = ctx.options.iter().find(|option| option.legal && option.description == "C") {
                return vec![option.index];
            }
            SelectFirstDecisionMaker.decide_options(game, ctx)
        }
    }
    for native in [false, true] {
        for accept in [false, true] {
            let rules = if native { "Demonstrate" } else {
                "When you cast this spell, you may copy it. If you do, choose an opponent to also copy it. Players may choose new targets for their copies."
            };
            let mut builder = compiler::CardDefinitionBuilder::new(compiler::ids::CardId::new(), "Shared lesson");
            if !native { builder = builder.with_custom_keyword("Teach"); }
            let card = compile_builder_to_runtime_definition(builder,
                format!("Type: Sorcery\nMana cost: {{0}}\nYou gain 3 life.\n{rules}"), false).unwrap();
            let triggered = card.abilities.iter().find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered), _ => None,
            }).unwrap();
            assert_eq!(triggered.presentation_label.as_ref().is_some_and(|label|
                format!("{label:?}").contains("Demonstrate")), native);
            let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
            let a = PlayerId::from_index(0);
            let b = PlayerId::from_index(1);
            let c = PlayerId::from_index(2);
            game.turn.active_player = a;
            game.turn.priority_player = Some(a);
            game.turn.phase = ironsmith::Phase::FirstMain;
            game.turn.step = None;
            let source = game.create_object_from_definition(&card, a, Zone::Hand);
            let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action|
                matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == source)).unwrap();
            finish_reminder_cast(&mut game, action);
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            assert_eq!(game.stack.len(), 2, "only the cast triggers the copy instruction");
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut Copies(accept)).unwrap();
            assert_eq!(game.stack.len(), if accept { 3 } else { 1 });
            if accept {
                let mut controllers = game.stack.iter().map(|entry| game.controller_of_id(entry.object_id).unwrap()).collect::<Vec<_>>();
                controllers.sort_by_key(|player| player.0);
                assert_eq!(controllers, vec![a, a, c]);
            }
            ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
            assert!(queue.entries.is_empty(), "copies are not cast");
            while !game.stack_is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
            assert_eq!(game.player(a).unwrap().life, if accept { 26 } else { 23 });
            assert_eq!(game.player(b).unwrap().life, 20);
            assert_eq!(game.player(c).unwrap().life, if accept { 23 } else { 20 });
        }
    }
}

#[test]
fn reminder_identity_source_copy_retargeting_precedes_the_other_players_copy() {
    use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker, LegalAction, compute_legal_actions};
    use ironsmith::decisions::context::{BooleanContext, SelectOptionsContext, TargetsContext};
    struct Retarget { observations: Vec<(PlayerId, usize)>, choices: usize }
    impl DecisionMaker for Retarget {
        fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool { true }
        fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            if let Some(option) = ctx.options.iter().find(|option| option.legal && option.description == "C") {
                self.choices += 1;
                return vec![option.index];
            }
            SelectFirstDecisionMaker.decide_options(game, ctx)
        }
        fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<ironsmith::Target> {
            self.observations.push((ctx.player, game.stack.len()));
            vec![ironsmith::Target::Player(ctx.player)]
        }
    }
    for native in [false, true] {
        for removed in [false, true] {
            let rules = if native { "Demonstrate" } else {
                "When you cast this spell, you may copy it and you may choose new targets for the copy. If you copy the spell, choose an opponent. That player copies the spell and may choose new targets for that copy."
            };
            let card = compile_to_runtime_definition("Shared lightning",
                &format!("Type: Instant\nMana cost: {{0}}\nShared lightning deals 2 damage to any target.\n{rules}"), false).unwrap();
            let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
            let a = PlayerId::from_index(0);
            let c = PlayerId::from_index(2);
            game.turn.active_player = a;
            game.turn.priority_player = Some(a);
            let source = game.create_object_from_definition(&card, a, Zone::Hand);
            let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action|
                matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == source)).unwrap();
            let mut queue = finish_reminder_cast_with(&mut game, action, &mut Retarget { observations: vec![], choices: 0 });
            let original = game.stack.iter().find(|entry| entry.ability_effects.is_none()).unwrap().object_id;
            ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            if removed {
                game.stack.retain(|entry| entry.ability_effects.is_some() || entry.object_id != original);
                game.move_object(original, Zone::Graveyard, ironsmith::events::EventCause::effect());
            }
            let mut dm = Retarget { observations: vec![], choices: 0 };
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            // CR 608.2h: a source spell can be copied from its last-known
            // stack information even after it was countered.
            {
                assert_eq!(dm.observations.iter().map(|(player, _)| *player).collect::<Vec<_>>(), vec![a, c]);
                assert_eq!(dm.observations[1].1, dm.observations[0].1 + 1,
                    "the opponent's copy is created after the caster retargets");
                assert_eq!(dm.choices, 1);
                while !game.stack_is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
                assert_eq!(game.player(a).unwrap().life, if removed { 18 } else { 16 });
                assert_eq!(game.player(c).unwrap().life, 18);
            }
        }
    }
}

#[test]
fn reminder_identity_choose_controlled_or_reveal_from_hand_preserves_visibility_and_objects() {
    for native in [false, true] {
        for kind in ["Elemental", "Dragon"] {
            for zone in [Zone::Battlefield, Zone::Hand] {
                let rules = if native { format!("Behold an {kind}.") } else {
                    format!("To witness an {kind}, choose an {kind} you control or reveal an {kind} card from your hand.")
                };
                let card = compile_to_runtime_definition("Witness", &format!("Type: Sorcery\n{rules}"), false).unwrap();
                let effects = card.spell_effect.as_ref().unwrap().all_effects();
                let payment = effects.iter().find_map(|effect| effect.downcast_ref::<ironsmith::effects::BeholdEffect>()).unwrap();
                assert_eq!(payment.descriptive, !native);
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let a = PlayerId::from_index(0);
                let b = PlayerId::from_index(1);
                let source = game.create_object_from_definition(&card, a, Zone::Stack);
                let matching = compile_to_runtime_definition("Matching", &format!("Type: Creature — {kind}\nPower/Toughness: 2/2\nShroud"), false).unwrap();
                let selected = game.create_object_from_definition(&matching, a, zone);
                let opponent = game.create_object_from_definition(&matching, b, zone);
                let wrong_zone = game.create_object_from_definition(&matching, a, Zone::Graveyard);
                let wrong_type = game.create_object_from_definition(&definition("Other", "", None), a, zone);
                let outcome = ironsmith::effects::execute_effect(&mut game, effects[0],
                    &mut ironsmith::effects::EffectContext::new_default(source, a)).unwrap();
                assert_eq!(outcome.output_objects(), &[selected]);
                assert_eq!(outcome.events.is_empty(), zone == Zone::Battlefield,
                    "only a hand choice should publish a reveal event");
                assert_eq!(game.object(selected).unwrap().zone, zone);
                assert_eq!(game.object(opponent).unwrap().zone, zone);
                assert_eq!(game.object(wrong_zone).unwrap().zone, Zone::Graveyard);
                assert_eq!(game.object(wrong_type).unwrap().zone, zone);
                assert!(!game.is_tapped(selected));
            }
        }
    }
}

#[test]
fn reminder_identity_controlled_or_revealed_payment_is_validated_before_activation() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    for native in [false, true] {
        for zone in [Zone::Battlefield, Zone::Hand] {
            for available in [false, true] {
                let price = if native { "Behold an Elemental" } else {
                    "Choose an Elemental you control or reveal an Elemental card from your hand"
                };
                let card = definition("Lookout", &format!("{price}: You gain 2 life."), (!native).then_some("Witness"));
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let a = PlayerId::from_index(0);
                let b = PlayerId::from_index(1);
                game.turn.active_player = a;
                game.turn.priority_player = Some(a);
                let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
                let elemental = compile_to_runtime_definition("Elemental", "Type: Creature — Elemental\nPower/Toughness: 1/1", false).unwrap();
                let candidate = game.create_object_from_definition(&elemental, if available { a } else { b }, zone);
                let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action|
                    matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source));
                assert_eq!(action.is_some(), available, "{price} in {zone:?}");
                if let Some(action) = action {
                    finish_reminder_cast(&mut game, action);
                    assert_eq!(game.object(candidate).unwrap().zone, zone, "paying does not move the chosen object");
                    while !game.stack_is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
                    assert_eq!(game.player(a).unwrap().life, 22);
                }
            }
        }
    }
}

#[test]
fn reminder_identity_room_progression_runs_rooms_and_completion_without_venture_identity() {
    use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
    use ironsmith::decisions::context::SelectOptionsContext;
    const DUNGEON: &str = "Reminder Definition Dungeon";
    struct Route;
    impl DecisionMaker for Route {
        fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            if let Some(option) = ctx.options.iter().find(|option| option.legal && (option.description == DUNGEON || option.description == "Right")) {
                return vec![option.index];
            }
            SelectFirstDecisionMaker.decide_options(game, ctx)
        }
    }
    let mut dungeon = compile_to_runtime_definition(DUNGEON, "Type: Dungeon", false).unwrap();
    for (room, exits, life) in [("Entry", vec!["Left", "Right"], 1), ("Left", vec!["Vault"], 2),
        ("Right", vec!["Vault"], 3), ("Vault", vec![], 5)] {
        dungeon.abilities.push(Ability::triggered(
            ironsmith::triggers::Trigger::dungeon_room(room, exits.into_iter().map(str::to_owned).collect()),
            vec![ironsmith::effect::Effect::gain_life(life)]));
    }
    ironsmith::dungeon::register_dungeon_definition(&dungeon).unwrap();
    for native in [false, true] {
        let rules = if native { "Venture into the dungeon." } else { "Enter the first room or advance to the next room." };
        let card = compile_to_runtime_definition("Journey", &format!("Type: Sorcery\n{rules}"), false).unwrap();
        let effect = card.spell_effect.as_ref().unwrap().all_effects()[0];
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0);
        let b = PlayerId::from_index(1);
        let source = game.create_object_from_definition(&card, a, Zone::Stack);
        for (room, life) in [("Entry", 21), ("Right", 24), ("Vault", 29), ("Entry", 30)] {
            let outcome = ironsmith::effects::execute_effect(&mut game, effect,
                &mut ironsmith::effects::EffectContext::new(source, a, &mut Route)).unwrap();
            assert_eq!(game.active_dungeon(a).unwrap().room_name, room);
            assert!(game.active_dungeon(b).is_none());
            assert_eq!(outcome.events.iter().any(|event| event.downcast::<ironsmith::events::KeywordActionEvent>()
                .is_some_and(|action| action.action == ironsmith::events::KeywordActionKind::VentureIntoDungeon)), native);
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
            assert_eq!(queue.entries.len(), 1, "entering a room still triggers its ability");
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            while !game.stack_is_empty() { ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut Route).unwrap(); }
            assert_eq!(game.player(a).unwrap().life, life);
            assert_eq!(game.player(b).unwrap().life, 20);
        }
        assert!(game.has_completed_named_dungeon(a, DUNGEON));
    }
}

#[test]
fn reminder_identity_speed_initialization_keeps_the_inherent_trigger_after_source_leaves() {
    use ironsmith::decision::SelectFirstDecisionMaker;
    use ironsmith::events::EventCause;
    for native in [false, true] {
        let rules = if native { "Start your engines!" } else {
            "If you have no speed while you control this permanent, your speed becomes 1 as a state-based action. Whenever one or more opponents lose life during your turn, if your speed is less than 4, your speed increases by 1. This inherent ability has no source and triggers only once each turn. Your speed remains after this permanent leaves the battlefield. Max speed is 4."
        };
        let card = definition("Momentum", rules, (!native).then_some("Accelerate"));
        let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
        let a = PlayerId::from_index(0);
        let b = PlayerId::from_index(1);
        let c = PlayerId::from_index(2);
        game.turn.active_player = a;
        let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
        assert_eq!(game.player_speed(a), None);
        ironsmith::rules::state_based::apply_state_based_actions_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.player_speed(a), Some(1));
        assert_eq!(game.player_speed(b), None);
        assert_eq!(game.current_has_static_ability_id(source, StaticAbilityId::StartYourEngines), native);
        game.move_object(source, Zone::Graveyard, EventCause::effect());
        for (turn, active, expected_speed) in [(1, b, 1), (2, a, 2), (3, a, 3), (4, a, 4), (5, a, 4)] {
            if turn > 1 { game.next_turn(); }
            game.turn.active_player = active;
            for opponent in [b, c] {
                let loss = ironsmith::effect::Effect::new(ironsmith::effects::LoseLifeEffect::with_filter(1, ironsmith::target::PlayerFilter::Specific(opponent)));
                let outcome = ironsmith::effects::execute_effect(&mut game, &loss,
                    &mut ironsmith::effects::EffectContext::new_default(source, a)).unwrap();
                for event in outcome.events { game.queue_trigger_event(Default::default(), event); }
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
                ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                while !game.stack_is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
            }
            assert_eq!(game.player_speed(a), Some(expected_speed), "turn {turn}: speed persists and increases only once on its owner's turn");
        }
        assert!(game.has_max_speed(a));
    }
}

#[test]
fn reminder_identity_sector_assignment_orders_players_and_retains_source_last_known_behavior() {
    use ironsmith::rules::state_based::{check_state_based_actions, apply_state_based_actions_with, StateBasedAction};
    use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
    use ironsmith::decisions::context::SelectOptionsContext;
    use ironsmith::marker::SectorDesignation::{Alpha, Beta};
    struct Sector;
    impl DecisionMaker for Sector {
        fn decide_options(&mut self, _: &GameState, _: &SelectOptionsContext) -> Vec<usize> { vec![1] }
    }
    for native in [false, true] {
        let rules = if native { "Space sculptor" } else {
            "As a state-based action, each creature without a sector designation is assigned alpha, beta, or gamma sector by its controller. Players who don't control a permanent with this ability assign first, then the other players assign, in active-player order within each group. A permanent keeps its sector until no player controls a permanent with this ability or an ability whose source has this ability. Sector designations aren't copiable."
        };
        let card = compile_to_runtime_definition("Partitions", &format!("Type: Artifact\n{rules}"), false).unwrap();
        let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
        let a = PlayerId::from_index(0);
        let b = PlayerId::from_index(1);
        let c = PlayerId::from_index(2);
        game.turn.active_player = a;
        let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
        let creature_card = definition("Resident", "", None);
        let residents = [a, b, c].map(|player| game.create_object_from_definition(&creature_card, player, Zone::Battlefield));
        let action = check_state_based_actions(&game).into_iter().find(|action| matches!(action, StateBasedAction::SectorDesignationChoices { .. })).unwrap();
        assert_eq!(action, StateBasedAction::SectorDesignationChoices {
            source, creatures: vec![(b, residents[1]), (c, residents[2]), (a, residents[0])],
        });
        assert_eq!(game.current_has_static_ability_id(source, StaticAbilityId::SpaceSculptor), native);
        game.set_sector_designation(residents[0], Alpha);
        apply_state_based_actions_with(&mut game, &mut Sector).unwrap();
        assert_eq!(game.sector_designation(residents[0]), Some(Alpha));
        assert_eq!(game.sector_designation(residents[1]), Some(Beta));
        let copy = game.create_object_from_definition(&creature_card, a, Zone::Battlefield);
        assert_eq!(game.sector_designation(copy), None, "designation is not copied");
        apply_state_based_actions_with(&mut game, &mut Sector).unwrap();
        let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(source).unwrap(), &game);
        game.stack.push(ironsmith::game_state::StackEntry::ability(source, a,
            ironsmith::resolution::ResolutionProgram::default()).with_source_snapshot(snapshot));
        game.move_object(source, Zone::Graveyard, ironsmith::events::EventCause::effect());
        assert!(!check_state_based_actions(&game).contains(&StateBasedAction::ClearSectorDesignations));
        let moved = game.move_object(copy, Zone::Exile, ironsmith::events::EventCause::effect()).unwrap();
        assert_eq!(game.sector_designation(moved), None);
        game.stack.pop();
        assert!(check_state_based_actions(&game).contains(&StateBasedAction::ClearSectorDesignations));
        apply_state_based_actions_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        for resident in residents { assert_eq!(game.sector_designation(resident), None); }
    }
}

#[test]
fn reminder_identity_entry_turn_price_reduction_pays_colored_excess_and_retains_once_limit() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    use ironsmith::mana::ManaSymbol;
    for native in [false, true] {
        for entered_this_turn in [false, true] {
            let rules = if native { "Power-up — {5}{R}{R}: Put two +1/+1 counters on this creature." } else {
                "{5}{R}{R}: Put two +1/+1 counters on this creature. If this permanent entered this turn, this ability's cost is reduced by this permanent's mana cost. Activate this ability only once."
            };
            let card = compile_to_runtime_definition("Growing hero", &format!("Type: Creature\nMana cost: {{2}}{{G}}{{R}}\nPower/Toughness: 2/2\n{rules}"), false).unwrap();
            let activated = card.abilities.iter().find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(activated), _ => None,
            }).unwrap();
            assert_eq!(activated.keyword == Some(ironsmith_core::ActivatedAbilityKeyword::PowerUp), native);
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            let hand = game.create_object_from_definition(&card, a, Zone::Hand);
            let source = game.move_object(hand, Zone::Battlefield, ironsmith::events::EventCause::effect()).unwrap();
            if !entered_this_turn { game.next_turn(); }
            game.turn.active_player = a;
            game.turn.priority_player = Some(a);
            game.turn.phase = ironsmith::Phase::FirstMain;
            game.turn.step = None;
            let generic = if entered_this_turn { 2 } else { 5 };
            let red = if entered_this_turn { 1 } else { 2 };
            game.player_mut(a).unwrap().mana_pool.add(ManaSymbol::Colorless, generic);
            game.player_mut(a).unwrap().mana_pool.add(ManaSymbol::Red, red);
            let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action|
                matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)).unwrap();
            finish_reminder_cast(&mut game, action);
            assert_eq!(game.player(a).unwrap().mana_pool.total(), 0,
                "colored excess reduces generic only; the red symbol still must be paid");
            while !game.stack_is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
            assert_eq!(game.counter_count(source, ironsmith::object::CounterType::PlusOnePlusOne), 2);
            for later_turn in [false, true] {
                if later_turn { game.next_turn(); }
                game.turn.active_player = a;
                game.turn.priority_player = Some(a);
                game.player_mut(a).unwrap().mana_pool.add(ManaSymbol::Colorless, 20);
                game.player_mut(a).unwrap().mana_pool.add(ManaSymbol::Red, 20);
                assert!(!compute_legal_actions(&game, a).unwrap().iter().any(|action|
                    matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)),
                    "the activation limit belongs to this permanent, not the turn");
            }
            let mut permission = definition("Extra activation", "", None);
            permission.abilities.push(Ability::static_ability(StaticAbility::power_up_abilities_additional_time()));
            game.create_object_from_definition(&permission, a, Zone::Battlefield);
            assert_eq!(compute_legal_actions(&game, a).unwrap().iter().any(|action|
                matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)), native,
                "a Power-up-specific permission must not raise the custom ability's limit");
        }
    }
}

#[test]
fn reminder_identity_planeswalker_token_creation_and_loyalty_growth_share_ordinary_programs() {
    use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker, compute_legal_actions, LegalAction};
    use ironsmith::decisions::context::SelectObjectsContext;
    use ironsmith::object::CounterType;
    struct Last;
    impl DecisionMaker for Last {
        fn decide_objects(&mut self, _: &GameState, ctx: &SelectObjectsContext) -> Vec<ironsmith::ids::ObjectId> {
            ctx.candidates.last().map(|candidate| candidate.id).into_iter().collect()
        }
    }
    for native in [false, true] {
        for amount in [0, 3, 5] {
            for existing in [0, 2] {
                let rules = if native { format!("Empower Jace {amount}.") } else {
                    format!("If you don't control a Jace planeswalker token, create a blue Jace planeswalker token with 0 loyalty, \"[-1]: Surveil 1\" and \"[-3]: Draw a card.\" Put {amount} loyalty counters on a Jace planeswalker token you control.")
                };
                let card = compile_to_runtime_definition("Gather wisdom", &format!("Type: Sorcery\n{rules}"), false).unwrap();
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let a = PlayerId::from_index(0);
                let b = PlayerId::from_index(1);
                let source = game.create_object_from_definition(&card, a, Zone::Stack);
                let mut walker = compile_to_runtime_definition("Jace Token", "Type: Planeswalker — Jace\nColor indicator: U\nLoyalty: 1\n[-3]: Draw a card.", false).unwrap();
                walker.card.is_token = true;
                let opponent = game.create_object_from_definition(&walker, b, Zone::Battlefield);
                let own = (0..existing).map(|_| game.create_object_from_definition(&walker, a, Zone::Battlefield)).collect::<Vec<_>>();
                let mut non_token = walker.clone();
                non_token.card.is_token = false;
                let excluded = game.create_object_from_definition(&non_token, a, Zone::Battlefield);
                let mut observations = Vec::new();
                let mut dm = Last;
                let mut ctx = ironsmith::effects::EffectContext::new(source, a, &mut dm);
                for effect in card.spell_effect.as_ref().unwrap().all_effects() {
                    observations.extend(ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap().events);
                }
                assert_eq!(game.battlefield.len(), 2 + existing.max(1));
                let chosen = own.last().copied().unwrap_or_else(|| *game.battlefield.last().unwrap());
                assert_eq!(game.counter_count(chosen, CounterType::Loyalty), amount + u32::from(existing > 0));
                assert_eq!(game.counter_count(opponent, CounterType::Loyalty), 1);
                assert_eq!(game.counter_count(excluded, CounterType::Loyalty), 1);
                if existing == 2 { assert_eq!(game.counter_count(own[0], CounterType::Loyalty), 1); }
                assert_eq!(observations.iter().any(|event| event.downcast::<ironsmith::events::KeywordActionEvent>()
                    .is_some_and(|action| action.action == ironsmith::events::KeywordActionKind::EmpowerJace)), native);
                if existing == 0 && amount >= 3 {
                    let chars = game.current_characteristics(chosen).unwrap();
                    assert!(chars.card_types.contains(&ironsmith::types::CardType::Planeswalker));
                    assert!(chars.subtypes.contains(&ironsmith::types::Subtype::Jace));
                    assert!(!chars.supertypes.contains(&ironsmith::types::Supertype::Legendary));
                    assert_eq!(chars.colors, ironsmith::color::ColorSet::BLUE);
                    game.create_object_from_definition(&definition("Lesson", "", None), a, Zone::Library);
                    game.turn.active_player = a;
                    game.turn.priority_player = Some(a);
                    game.turn.phase = ironsmith::Phase::FirstMain;
                    game.turn.step = None;
                    let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action|
                        matches!(action, LegalAction::ActivateAbility { source: id, ability_index: 1, .. } if *id == chosen)).unwrap();
                    finish_reminder_cast_with(&mut game, action, &mut SelectFirstDecisionMaker);
                    while !game.stack_is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
                    assert_eq!(game.player(a).unwrap().hand.len(), 1, "the token's quoted loyalty ability must execute");
                }
            }
        }
    }
}

#[test]
fn reminder_identity_planeswalker_token_definition_accepts_other_subtypes_and_loyalty() {
    let card = compile_to_runtime_definition("New mentor",
        "Type: Sorcery\nCreate a green Nissa planeswalker token with 4 loyalty and \"[-2]: You gain 3 life.\"", false).unwrap();
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let a = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&card, a, Zone::Stack);
    for effect in card.spell_effect.as_ref().unwrap().all_effects() {
        ironsmith::effects::execute_effect(&mut game, effect,
            &mut ironsmith::effects::EffectContext::new_default(source, a)).unwrap();
    }
    assert_eq!(game.battlefield.len(), 1);
    let token = game.battlefield[0];
    assert_eq!(game.counter_count(token, ironsmith::object::CounterType::Loyalty), 4);
    let chars = game.current_characteristics(token).unwrap();
    assert_eq!(chars.colors, ironsmith::color::ColorSet::GREEN);
    assert!(chars.subtypes.contains(&ironsmith::types::Subtype::Nissa));
    let ability = game.object(token).unwrap().abilities.iter().find_map(|ability| match &ability.kind {
        AbilityKind::Activated(activated) => Some(activated), _ => None,
    }).unwrap();
    assert!(ability.is_loyalty_ability);
    assert_eq!(ability.effects.all_effects().len(), 1);
}

#[test]
fn reminder_identity_optional_attack_payment_taps_eligible_creatures_and_uses_resolution_power() {
    use ironsmith::decision::DecisionMaker;
    use ironsmith::decisions::context::{DecisionContext, SelectObjectsContext};
    use ironsmith::ids::ObjectId;
    struct Donor(Option<ObjectId>);
    impl DecisionMaker for Donor {
        fn decide_objects(&mut self, _: &GameState, _: &SelectObjectsContext) -> Vec<ObjectId> { self.0.into_iter().collect() }
    }
    for native in [false, true] {
        for scenario in 0..3 {
            let rules = if native { "Enlist" } else {
                "As this creature attacks, you may tap up to one untapped creature you control that you didn't choose to attack with and that either has haste or has been under your control continuously since this turn began. When you do, this creature gets +X/+0 until end of turn, where X is the tapped creature's power."
            };
            let card = definition("Leader", rules, (!native).then_some("Rally"));
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            let b = PlayerId::from_index(1);
            let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
            game.remove_summoning_sickness(source);
            let donor_card = compile_to_runtime_definition("Helper", "Type: Creature\nPower/Toughness: 4/4\nShroud", false).unwrap();
            let donor = game.create_object_from_definition(&donor_card, a, Zone::Battlefield);
            game.remove_summoning_sickness(donor);
            let sick = game.create_object_from_definition(&donor_card, a, Zone::Battlefield);
            game.set_summoning_sick(sick);
            let other_attacker = game.create_object_from_definition(&donor_card, a, Zone::Battlefield);
            game.remove_summoning_sickness(other_attacker);
            let opponent = game.create_object_from_definition(&donor_card, b, Zone::Battlefield);
            game.remove_summoning_sickness(opponent);
            let ability = game.current_characteristics(source).unwrap().static_abilities.into_iter()
                .find(|ability| ability.optional_attack_cost_prompt(&game, source, a, &[source, other_attacker]).is_some()).unwrap();
            assert_eq!(ability.id() == StaticAbilityId::EnlistAttack, native);
            let prompt = ability.optional_attack_cost_prompt(&game, source, a, &[source, other_attacker]).unwrap();
            let DecisionContext::SelectObjects(prompt) = prompt else { panic!("expected creature payment"); };
            let candidates = prompt.candidates.iter().map(|candidate| candidate.id).collect::<Vec<_>>();
            assert_eq!(candidates, vec![donor], "excludes attacker, sick creature {sick:?}, and opponent's creature");
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            ability.pay_optional_attack_cost(&mut game, source, a, &[source, other_attacker], &mut queue,
                &mut Donor((scenario != 0).then_some(donor))).unwrap().unwrap();
            assert_eq!(game.is_tapped(donor), scenario != 0);
            assert_eq!(queue.entries.len(), usize::from(scenario != 0));
            if scenario == 0 { continue; }
            let native_event = queue.entries[0].triggering_event.downcast::<ironsmith::events::KeywordActionEvent>();
            assert_eq!(native_event.is_some_and(|event| event.action == ironsmith::events::KeywordActionKind::Enlist), native);
            if scenario == 1 { game.add_counters(donor, ironsmith::object::CounterType::PlusOnePlusOne, 1); }
            else { game.move_object(donor, Zone::Graveyard, ironsmith::events::EventCause::effect()); }
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            while !game.stack_is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
            assert_eq!(game.current_characteristics(source).unwrap().power, Some(if scenario == 1 { 7 } else { 6 }));
            assert_eq!(game.current_characteristics(source).unwrap().toughness, Some(2));
        }
    }
}

#[test]
fn reminder_identity_outside_game_choice_validates_deck_and_pays_for_one_sorcery_speed_action() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    use ironsmith::special_actions::SpecialAction;
    for native in [false, true] {
        let rules = if native { "Companion — Your starting deck contains only cards with even mana values." } else {
            "Before the game begins, you may reveal this card from outside the game as your chosen companion if your starting deck contains only cards with even mana values. Once during the game, during a main phase of your turn while you have priority and the stack is empty, you may pay {3} to put your chosen companion into your hand. This is a special action that doesn't use the stack."
        };
        let card = definition("Traveling scholar", rules, (!native).then_some("Fellowship"));
        let even = compile_to_runtime_definition("Even", "Type: Sorcery\nMana cost: {2}\nDraw a card.", false).unwrap();
        let odd = compile_to_runtime_definition("Odd", "Type: Sorcery\nMana cost: {1}\nDraw a card.", false).unwrap();
        assert!(ironsmith::companion::validate_companion_definition(&card, &[even.clone()], 1).is_ok());
        assert!(ironsmith::companion::validate_companion_definition(&card, &[odd.clone()], 1).is_err());
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&card, a, Zone::OutsideGame);
        let deck = game.create_object_from_definition(&even, a, Zone::Library);
        assert_eq!(game.object(source).unwrap().has_static_ability_id(StaticAbilityId::Companion), native);
        game.designate_companion(a, source, &[deck], 1).unwrap();
        let second = game.create_object_from_definition(&card, a, Zone::OutsideGame);
        assert!(game.designate_companion(a, second, &[deck], 1).is_err());
        game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 3);
        game.turn.active_player = a;
        game.turn.priority_player = Some(a);
        game.turn.phase = ironsmith::Phase::Combat;
        assert!(!compute_legal_actions(&game, a).unwrap().iter().any(|action| matches!(action,
            LegalAction::SpecialAction(SpecialAction::Companion { .. }))));
        game.turn.phase = ironsmith::Phase::FirstMain;
        game.turn.step = None;
        let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action|
            matches!(action, LegalAction::SpecialAction(SpecialAction::Companion { card_id }) if *card_id == source)).unwrap();
        finish_reminder_cast(&mut game, action);
        assert!(game.stack_is_empty(), "this is a special action, not casting or activation");
        assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
        let moved = game.player(a).unwrap().companion.unwrap();
        assert_eq!(game.object(moved).unwrap().zone, Zone::Hand);
        game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 3);
        assert!(!compute_legal_actions(&game, a).unwrap().iter().any(|action| matches!(action,
            LegalAction::SpecialAction(SpecialAction::Companion { .. }))));
    }
}

#[test]
fn reminder_identity_combat_bands_preserve_blocking_and_damage_choice_without_keyword_identity() {
    use ironsmith::combat_state::{CombatState, AttackerInfo, AttackTarget, set_attacking_band, declare_blockers, combat_damage_assignment_player};
    for native in [false, true] {
        let rules = if native { "Banding" } else {
            "As you declare attackers, you may group one or more attacking creatures with this ability and up to one other attacking creature into a band. Each creature can belong to only one band, and all members must attack the same player, planeswalker, or battle. The band lasts for the rest of combat even if this ability is lost; creatures removed from combat leave the band. Members don't share abilities. A creature blocking any member blocks all members, and an effect that blocks one member blocks the whole band. If this creature is blocking an attacker, you choose how that attacker divides its combat damage among its blockers. If this creature is blocked, you choose how its blockers divide their combat damage among the creatures they block."
        };
        let card = definition("Leader", rules, (!native).then_some("Cohort"));
        let plain = definition("Other", "", None);
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0);
        let b = PlayerId::from_index(1);
        game.turn.active_player = a;
        let first = game.create_object_from_definition(&card, a, Zone::Battlefield);
        let second = game.create_object_from_definition(&card, a, Zone::Battlefield);
        let other = game.create_object_from_definition(&plain, a, Zone::Battlefield);
        let extra = game.create_object_from_definition(&plain, a, Zone::Battlefield);
        let blocker = game.create_object_from_definition(&card, b, Zone::Battlefield);
        let mut combat = CombatState { attackers: [first, second, other, extra].into_iter().map(|creature|
            AttackerInfo { creature, target: AttackTarget::Player(b) }).collect(), ..Default::default() };
        assert_eq!(game.current_has_static_ability_id(first, StaticAbilityId::Banding), native);
        assert!(set_attacking_band(&game, &mut combat.clone(), vec![first, other, extra]).is_err());
        let mut different_defenders = combat.clone();
        different_defenders.attackers[1].target = AttackTarget::Player(a);
        assert!(set_attacking_band(&game, &mut different_defenders, vec![first, second]).is_err());
        set_attacking_band(&game, &mut combat, vec![first, second, other]).unwrap();
        assert!(set_attacking_band(&game, &mut combat, vec![first, extra]).is_err());
        declare_blockers(&game, &mut combat, vec![(blocker, other)]).unwrap();
        for attacker in [first, second, other] {
            assert_eq!(combat.blockers.get(&attacker), Some(&vec![blocker]));
            assert_eq!(combat_damage_assignment_player(&game, &combat, attacker), Some(b));
        }
        assert_eq!(combat_damage_assignment_player(&game, &combat, blocker), Some(a));
        game.combat = Some(combat.clone());
        assert!(game.set_combat_damage_assignment_for_player(b, blocker, other, 2).is_err());
        game.set_combat_damage_assignment_for_player(a, blocker, other, 2).unwrap();
        game.object_mut(first).unwrap().abilities_mut().clear();
        game.object_mut(second).unwrap().abilities_mut().clear();
        assert_eq!(combat.attacking_bands, vec![vec![first, second, other]], "announced bands persist after ability loss");
        assert_eq!(combat_damage_assignment_player(&game, &combat, blocker), Some(b), "damage choice still requires the ability at assignment time");
    }
}

#[test]
fn reminder_identity_delayed_exile_price_preserves_payment_owner_permission_and_incarnation() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    use ironsmith::alternative_cast::CastingMethod;
    use ironsmith::events::EventCause;
    for native in [false, true] {
        for departed in [false, true] {
            let rules = if native { "Warp—{B}, Pay 2 life." } else {
                "You may cast this card from your hand for {B}, Pay 2 life rather than its mana cost. If you do, exile the permanent this spell becomes at the beginning of the next end step. Its owner may cast that card after this turn ends for as long as it remains exiled."
            };
            let card = compile_to_runtime_definition("Temporary visitor", &format!("Type: Creature\nMana cost: {{2}}{{B}}\nPower/Toughness: 2/2\n{rules}"), false).unwrap();
            assert_eq!(card.alternative_casts[0].keyword() == Some(ironsmith_core::alternative_cast_model::AlternativeCastKeyword::Warp), native);
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            game.turn.active_player = a;
            game.turn.priority_player = Some(a);
            game.turn.phase = ironsmith::Phase::FirstMain;
            game.turn.step = None;
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Black, 1);
            let grave = game.create_object_from_definition(&card, a, Zone::Graveyard);
            assert!(!compute_legal_actions(&game, a).unwrap().iter().any(|action|
                matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == grave)), "the full official rule grants this price only from hand");
            let hand = game.create_object_from_definition(&card, a, Zone::Hand);
            let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action|
                matches!(action, LegalAction::CastSpell { spell_id, casting_method: CastingMethod::Alternative(0), .. } if *spell_id == hand)).unwrap();
            finish_reminder_cast(&mut game, action);
            assert_eq!(game.player(a).unwrap().life, 18);
            assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
            assert_eq!(game.turn_store.turn_history.spell_was_warped_this_turn(), native);
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            let permanent = game.battlefield[0];
            let returned = if departed {
                let grave = game.move_object(permanent, Zone::Graveyard, EventCause::effect()).unwrap();
                Some(game.move_object(grave, Zone::Battlefield, EventCause::effect()).unwrap())
            } else { None };
            let event = ironsmith::triggers::TriggerEvent::new(ironsmith::events::BeginningOfEndStepEvent::new(a), Default::default());
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            for trigger in ironsmith::triggers::check_delayed_triggers(&mut game, &event) { queue.add(trigger); }
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            while !game.stack_is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
            if let Some(returned) = returned {
                assert!(game.battlefield.contains(&returned));
                assert!(game.exile.is_empty());
                continue;
            }
            let exiled = game.exile[0];
            assert_eq!(game.is_warped(exiled), native, "custom exile permission must not make the card warped");
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Black, 1);
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 2);
            assert!(!compute_legal_actions(&game, a).unwrap().iter().any(|action|
                matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == exiled)));
            game.turn.turn_number += 1;
            game.turn.priority_player = Some(a);
            let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action|
                matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == exiled)).unwrap();
            finish_reminder_cast(&mut game, action);
            assert_eq!(game.player(a).unwrap().mana_pool.total(), 0, "later casting pays the normal price");
            assert_eq!(game.player(a).unwrap().life, 18);
        }
    }
}

#[test]
fn reminder_identity_return_attacker_price_preserves_window_payment_and_defender() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    use ironsmith::alternative_cast::CastingMethod;
    use ironsmith::combat_state::{CombatState, AttackerInfo, AttackTarget};
    for native in [false, true] {
        for countered in [false, true] {
            let rules = if native { "Sneak—{B}, Pay 2 life." } else {
                "During your declare blockers step, any time you could cast an instant, you may cast this spell from your hand for {B}, Pay 2 life and return an unblocked attacker you control to its owner's hand rather than pay this spell's mana cost. It enters tapped and attacking the same player, planeswalker, or battle as the returned creature."
            };
            let card = compile_to_runtime_definition("Replacement", &format!("Type: Creature\nMana cost: {{5}}{{B}}\nPower/Toughness: 3/3\n{rules}"), false).unwrap();
            assert_eq!(card.alternative_casts[0].keyword() == Some(ironsmith_core::alternative_cast_model::AlternativeCastKeyword::Sneak), native);
            let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
            let a = PlayerId::from_index(0);
            let b = PlayerId::from_index(1);
            let c = PlayerId::from_index(2);
            let donor = game.create_object_from_definition(&definition("Attacker", "", None), a, Zone::Battlefield);
            let hand = game.create_object_from_definition(&card, a, Zone::Hand);
            game.combat = Some(CombatState { attackers: vec![AttackerInfo { creature: donor, target: AttackTarget::Player(c) }], block_declaration_complete: true, ..Default::default() });
            game.turn.active_player = a;
            game.turn.priority_player = Some(a);
            game.turn.phase = ironsmith::Phase::Combat;
            game.turn.step = Some(ironsmith::Step::DeclareAttackers);
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Black, 1);
            let available = |game: &GameState| compute_legal_actions(game, a).unwrap().into_iter().find(|action|
                matches!(action, LegalAction::CastSpell { spell_id, casting_method: CastingMethod::Alternative(0), .. } if *spell_id == hand));
            assert!(available(&game).is_none());
            game.turn.step = Some(ironsmith::Step::DeclareBlockers);
            game.turn.active_player = b;
            assert!(available(&game).is_none(), "only during your own declare blockers step");
            game.turn.active_player = a;
            game.combat.as_mut().unwrap().block_declaration_complete = false;
            assert!(available(&game).is_none(), "attackers are not yet unblocked");
            game.combat.as_mut().unwrap().block_declaration_complete = true;
            let action = available(&game).expect("the defined price grants instant timing in this window");
            finish_reminder_cast(&mut game, action);
            assert_eq!(game.player(a).unwrap().life, 18);
            assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
            assert!(!game.battlefield.contains(&donor));
            assert!(game.player(a).unwrap().hand.iter().any(|id| game.object(*id).unwrap().name == "Attacker"));
            if countered {
                game.stack.clear();
                assert!(game.battlefield.is_empty(), "the returned attacker remains paid even when the spell fails to resolve");
                continue;
            }
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            let replacement = game.battlefield[0];
            assert!(game.is_tapped(replacement));
            assert!(game.combat.as_ref().unwrap().attackers.iter().any(|attacker| attacker.creature == replacement && attacker.target == AttackTarget::Player(c)));
        }
    }
}

#[test]
fn reminder_identity_return_attacker_activation_uses_hand_and_command_without_casting() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    use ironsmith::combat_state::{CombatState, AttackerInfo, AttackTarget};
    for native in [false, true] {
        for zone in [Zone::Hand, Zone::Command] {
            for countered in [false, true] {
                let rules = if native { "Commander ninjutsu {U}{B}" } else {
                    "{U}{B}, Return an unblocked attacker you control to its owner's hand: Put this card onto the battlefield from your hand or the command zone tapped and attacking the same player, planeswalker, or battle as the returned creature."
                };
                let card = definition("Ambusher", rules, (!native).then_some("Ambush"));
                let ability = card.abilities.iter().find_map(|ability| match &ability.kind {
                    ironsmith::ability::AbilityKind::Activated(activated) => Some(activated), _ => None }).unwrap();
                assert_eq!(ability.keyword == Some(ironsmith_core::ActivatedAbilityKeyword::Ninjutsu), native);
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let a = PlayerId::from_index(0);
                let b = PlayerId::from_index(1);
                let source = game.create_object_from_definition(&card, a, zone);
                let donor = game.create_object_from_definition(&definition("Attacker", "", None), a, Zone::Battlefield);
                game.combat = Some(CombatState { attackers: vec![AttackerInfo { creature: donor, target: AttackTarget::Player(b) }], block_declaration_complete: false, ..Default::default() });
                game.turn.active_player = a;
                game.turn.priority_player = Some(a);
                game.turn.phase = ironsmith::Phase::Combat;
                game.turn.step = Some(ironsmith::Step::DeclareBlockers);
                game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue, 1);
                game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Black, 1);
                let available = |game: &GameState| compute_legal_actions(game, a).unwrap().into_iter().find(|action|
                    matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source));
                assert!(available(&game).is_none());
                game.combat.as_mut().unwrap().block_declaration_complete = true;
                let action = available(&game).expect("ability works in its defined source zone");
                finish_reminder_cast(&mut game, action);
                assert_eq!(game.object(source).unwrap().zone, zone, "activating does not cast or move the source");
                assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
                assert!(game.stack.iter().all(|entry| entry.is_ability));
                assert_eq!(game.is_revealed_by_pending_ninjutsu(source), zone == Zone::Hand);
                assert!(!game.battlefield.contains(&donor));
                if countered {
                    game.stack.clear();
                    assert!(!game.is_revealed_by_pending_ninjutsu(source));
                    assert_eq!(game.object(source).unwrap().zone, zone);
                } else {
                    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                    let entered = game.battlefield[0];
                    assert!(game.is_tapped(entered));
                    assert!(game.combat.as_ref().unwrap().attackers.iter().any(|attacker| attacker.creature == entered && attacker.target == AttackTarget::Player(b)));
                }
            }
        }
    }
}

#[test]
fn reminder_identity_graveyard_warp_requires_a_separate_named_permission() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    for custom in [false, true] {
        for permission in [false, true] {
            let rules = if custom {
                "You may cast this card from your hand for {B}, Pay 2 life rather than its mana cost. If you do, exile the permanent this spell becomes at the beginning of the next end step. Its owner may cast that card after this turn ends for as long as it remains exiled."
            } else { "Warp—{B}, Pay 2 life." };
            let grant = if permission { "You may cast this card from your graveyard using its warp ability.\n" } else { "" };
            let card = compile_to_runtime_definition("Graveyard visitor", &format!("Type: Creature\nMana cost: {{B}}{{B}}\nPower/Toughness: 2/2\n{grant}{rules}"), false).unwrap();
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            game.turn.active_player = a;
            game.turn.priority_player = Some(a);
            game.turn.phase = ironsmith::Phase::FirstMain;
            game.turn.step = None;
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Black, 1);
            let source = game.create_object_from_definition(&card, a, Zone::Graveyard);
            let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action|
                matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == source));
            assert_eq!(action.is_some(), permission && !custom, "a named Warp permission must not also grant permission to a custom behavior");
            if let Some(action) = action {
                finish_reminder_cast(&mut game, action);
                assert_eq!(game.player(a).unwrap().life, 18);
                assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
                assert!(game.turn_store.turn_history.spell_was_warped_this_turn());
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                assert_eq!(game.battlefield.len(), 1);
            }
        }
    }
}

#[test]
fn reminder_identity_pairing_checks_both_entries_and_intervening_condition() {
    use ironsmith::decision::DecisionMaker;
    use ironsmith::decisions::context::SelectOptionsContext;
    struct PairChoice(bool);
    impl DecisionMaker for PairChoice {
        fn decide_options(&mut self, _: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            if self.0 { ctx.options.iter().find(|option| option.legal).map(|option| vec![option.index]).unwrap_or_default() }
            else { Vec::new() }
        }
    }
    for native in [false, true] {
        for source_entered in [false, true] {
            for scenario in 0..3 {
                let rules = if native { "Soulbond" } else {
                    "You may pair this creature with another unpaired creature when either enters. They remain paired for as long as you control both of them."
                };
                let card = definition("Partner", rules, (!native).then_some("Bond"));
                let other = definition("Other", "Shroud", None);
                let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
                let a = PlayerId::from_index(0);
                let b = PlayerId::from_index(1);
                let source = game.create_object_from_definition(&card, a, if source_entered { Zone::Hand } else { Zone::Battlefield });
                let partner = game.create_object_from_definition(&other, a, if source_entered { Zone::Battlefield } else { Zone::Hand });
                let opponent = game.create_object_from_definition(&other, b, Zone::Hand);
                enter_reminder_permanent(&mut game, opponent);
                assert!(drain_reminder_triggers(&mut game).is_empty());
                let source = if source_entered { enter_reminder_permanent(&mut game, source) } else { source };
                let partner = if !source_entered { enter_reminder_permanent(&mut game, partner) } else { partner };
                let marker = ironsmith::target::ObjectFilter::creature().with_ability_marker("soulbond");
                let ctx = ironsmith::effects::EffectContext::new_default(source, a);
                let matching = ironsmith::effects::helpers::resolve_objects_from_spec(&game, &ironsmith::target::ChooseSpec::All(marker), &ctx).unwrap();
                assert_eq!(matching.contains(&source), native);
                let entries = drain_reminder_triggers(&mut game);
                let event = entries.first().expect("entry trigger").triggering_event.clone();
                assert_eq!(entries.len(), 1);
                let mut queue = ironsmith::triggers::TriggerQueue::new();
                for entry in entries { queue.add(entry); }
                ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                if scenario == 2 { game.move_object(partner, Zone::Graveyard, ironsmith::events::EventCause::effect()); }
                let mut dm = PairChoice(scenario != 0);
                while !game.stack_is_empty() { ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap(); }
                assert_eq!(game.soulbond_partner(source), (scenario == 1).then_some(partner));
                if scenario == 1 {
                    assert!(ironsmith::triggers::check_triggers(&game, &event).is_empty(), "already paired creatures cannot trigger another pairing");
                    game.move_object(partner, Zone::Graveyard, ironsmith::events::EventCause::effect());
                    assert_eq!(game.soulbond_partner(source), None);
                }
            }
        }
    }
}

#[test]
fn reminder_identity_sacrifice_reduced_price_preserves_colors_and_eligible_permanents() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    use ironsmith::alternative_cast::CastingMethod;
    for artifact_type in [false, true] { for native in [false, true] {
        for value in [2, 7] {
            let rules = if native { if artifact_type { "Emerge from artifact {5}{B}{B}" } else { "Emerge {5}{B}{B}" } } else {
                "You may cast this spell from your hand for {5}{B}{B} and sacrifice an artifact rather than pay its mana cost. Reduce that price by an amount of generic mana equal to the sacrificed permanent's mana value."
            };
            let rules = if artifact_type { rules.to_owned() } else { rules.replace("an artifact", "a creature") };
            let card = compile_to_runtime_definition("Emergent", &format!("Type: Creature\nMana cost: {{9}}\nPower/Toughness: 5/5\n{rules}"), false).unwrap();
            assert_eq!(card.alternative_casts[0].name() == "Emerge", native);
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            let b = PlayerId::from_index(1);
            game.turn.active_player = a;
            game.turn.priority_player = Some(a);
            game.turn.phase = ironsmith::Phase::FirstMain;
            game.turn.step = None;
            let source = game.create_object_from_definition(&card, a, Zone::Hand);
            let artifact = compile_to_runtime_definition("Resource", &format!("Type: {}\nMana cost: {{{value}}}\nPower/Toughness: 1/1", if artifact_type { "Artifact" } else { "Creature" }), false).unwrap();
            let opposing = game.create_object_from_definition(&artifact, b, Zone::Battlefield);
            let unrelated = compile_to_runtime_definition("Unrelated", if artifact_type { "Type: Creature\nPower/Toughness: 1/1" } else { "Type: Artifact" }, false).unwrap();
            let creature = game.create_object_from_definition(&unrelated, a, Zone::Battlefield);
            let generic = (5_i32 - value).max(0) as u32;
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Black, 2);
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, generic);
            let available = |game: &GameState| compute_legal_actions(game, a).unwrap().into_iter().find(|action|
                matches!(action, LegalAction::CastSpell { spell_id, casting_method: CastingMethod::Alternative(0), .. } if *spell_id == source));
            assert!(available(&game).is_none(), "neither an opponent's artifact nor your nonartifact creature pays this cost");
            let resource = game.create_object_from_definition(&artifact, a, Zone::Battlefield);
            game.tap(resource);
            let action = available(&game).unwrap_or_else(|| panic!("sacrifice unavailable native={native} artifact={artifact_type} value={value} methods={:?}", card.alternative_casts));
            finish_reminder_cast(&mut game, action);
            assert_eq!(game.player(a).unwrap().mana_pool.total(), 0, "only generic mana is reduced, never the two black symbols");
            assert!(!game.battlefield.contains(&resource));
            assert!(game.battlefield.contains(&opposing));
            assert!(game.battlefield.contains(&creature));
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            assert!(game.battlefield.iter().any(|id| game.object(*id).unwrap().name == "Emergent"));
        }
    }
}
}

#[test]
fn reminder_identity_sacrifice_mana_cost_reduction_grants_timing_and_subtracts_colors() {
    use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker, compute_legal_actions, LegalAction};
    use ironsmith::decisions::context::SelectOptionsContext;
    struct Pay;
    impl DecisionMaker for Pay {
        fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            if ctx.description.starts_with("Choose optional costs") {
                return ctx.options.iter().find(|option| option.legal).map(|option| vec![option.index]).unwrap_or_default();
            }
            SelectFirstDecisionMaker.decide_options(game, ctx)
        }
    }
    for native in [false, true] {
        for colored in [false, true] {
            let rules = if native { "Moonfolk offering" } else {
                "You may cast this spell any time you could cast an instant by sacrificing a Moonfolk and paying the difference in mana costs between this and the sacrificed Moonfolk. Mana cost includes color."
            };
            let card = compile_to_runtime_definition("Offered creature", &format!("Type: Creature\nMana cost: {{4}}{{U}}{{U}}\nPower/Toughness: 5/5\n{rules}"), false).unwrap();
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            let b = PlayerId::from_index(1);
            game.turn.active_player = b;
            game.turn.priority_player = Some(a);
            game.turn.phase = ironsmith::Phase::Combat;
            game.turn.step = Some(ironsmith::Step::DeclareBlockers);
            let source = game.create_object_from_definition(&card, a, Zone::Hand);
            let payment = compile_to_runtime_definition("Payment", &format!("Type: Creature — Moonfolk\nMana cost: {}\nPower/Toughness: 1/1", if colored { "{1}{U}" } else { "{2}" }), false).unwrap();
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue, if colored { 1 } else { 2 });
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, if colored { 3 } else { 2 });
            let available = |game: &GameState| compute_legal_actions(game, a).unwrap().into_iter().find(|action|
                matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == source));
            assert!(available(&game).is_none(), "instant timing requires an eligible sacrifice");
            let opponent_resource = game.create_object_from_definition(&payment, b, Zone::Battlefield);
            assert!(available(&game).is_none());
            let resource = game.create_object_from_definition(&payment, a, Zone::Battlefield);
            let action = available(&game).expect("payment grants instant timing on an opponent's turn");
            finish_reminder_cast_with(&mut game, action, &mut Pay);
            assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
            assert!(!game.battlefield.contains(&resource));
            assert!(game.battlefield.contains(&opponent_resource));
            assert_eq!(game.stack.last().unwrap().optional_costs_paid.was_paid_label("Offering"), native);
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            assert!(game.battlefield.iter().any(|id| game.object(*id).unwrap().name == "Offered creature"));
        }
    }
}

#[test]
fn reminder_identity_scoped_tap_payment_uses_each_artifact_or_creature_once() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    for native in [false, true] {
        for amount in [4, 20] {
            let rules = if native { format!("Exhaust — Waterbend {{{amount}}}: You gain 2 life.") } else {
                format!("Pay {{{amount}}} using any combination of mana and tapping untapped artifacts and creatures you control for {{1}} each: You gain 2 life. Activate this ability only once.")
            };
            let card = definition("Payment source", &rules, (!native).then_some("Channel"));
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            let b = PlayerId::from_index(1);
            game.turn.active_player = a;
            game.turn.priority_player = Some(a);
            game.turn.phase = ironsmith::Phase::FirstMain;
            let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
            let artifact = compile_to_runtime_definition("Artifact", "Type: Artifact", false).unwrap();
            let dual = compile_to_runtime_definition("Dual", "Type: Artifact Creature\nPower/Toughness: 1/1", false).unwrap();
            let first = game.create_object_from_definition(&artifact, a, Zone::Battlefield);
            let both = game.create_object_from_definition(&dual, a, Zone::Battlefield);
            let opponent = game.create_object_from_definition(&artifact, b, Zone::Battlefield);
            let tapped = game.create_object_from_definition(&artifact, a, Zone::Battlefield);
            game.tap(tapped);
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, amount - 4);
            let available = |game: &GameState| compute_legal_actions(game, a).unwrap().into_iter().find(|action|
                matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source));
            assert!(available(&game).is_none(), "a dual-type permanent cannot be counted twice");
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 1);
            let action = available(&game).unwrap();
            finish_reminder_cast(&mut game, action);
            assert!(game.is_tapped(source) && game.is_tapped(first) && game.is_tapped(both), "summoning sickness does not prevent this tap payment");
            assert!(!game.is_tapped(opponent));
            assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
            assert_eq!(game.object_performed_keyword_action_this_turn(source, ironsmith::events::KeywordActionKind::Waterbend), native);
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            assert_eq!(game.player(a).unwrap().life, 22);
            for id in [source, first, both] { game.untap(id); }
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, amount);
            assert!(available(&game).is_none(), "the once-per-object restriction remains after resources refresh");
        }
    }
}

#[test]
fn reminder_identity_target_tax_allows_the_opponents_tap_payment_or_counters() {
    use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
    use ironsmith::decisions::context::{BooleanContext, SelectOptionsContext};
    struct Pay;
    impl DecisionMaker for Pay {
        fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool { true }
        fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> { SelectFirstDecisionMaker.decide_options(game, ctx) }
    }
    for native in [false, true] {
        for affordable in [false, true] {
            let rules = if native { "Ward—Waterbend {4}." } else {
                "Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {4} using any combination of mana and tapping untapped artifacts and creatures you control for {1} each."
            };
            let card = definition("Protected", rules, (!native).then_some("Shield"));
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let a = PlayerId::from_index(0);
            let b = PlayerId::from_index(1);
            let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
            assert_eq!(game.current_has_static_ability_id(source, StaticAbilityId::Ward), native);
            let resource = compile_to_runtime_definition("Resource", "Type: Artifact", false).unwrap();
            let resources = (0..4).map(|_| game.create_object_from_definition(&resource, if affordable { b } else { a }, Zone::Battlefield)).collect::<Vec<_>>();
            let spell = compile_to_runtime_definition("Opponent spell", "Type: Instant\nYou gain 2 life.", false).unwrap();
            let cast = game.create_object_from_definition(&spell, b, Zone::Stack);
            game.push_to_stack(ironsmith::StackEntry::new(cast, b).with_targets(vec![ironsmith::Target::Object(source)]));
            let event = ironsmith::triggers::TriggerEvent::new(ironsmith::events::BecomesTargetedEvent::new(source, cast, b, false), Default::default());
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            let triggers = ironsmith::triggers::check_triggers(&game, &event);
            assert_eq!(triggers.len(), 1);
            for trigger in triggers { queue.add(trigger); }
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut Pay).unwrap();
            assert_eq!(game.stack.len(), usize::from(affordable), "native={native} affordable={affordable} abilities={:?}", card.abilities);
            assert!(resources.iter().all(|id| game.is_tapped(*id) == affordable), "the targeted creature's controller cannot pay using their own resources");
            if affordable {
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                assert_eq!(game.player(b).unwrap().life, 22);
            } else { assert_eq!(game.object(cast).map(|object| object.zone), None); }
        }
    }
}

#[test]
fn reminder_identity_first_draw_price_requires_reveal_and_retains_complete_payment() {
    use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker, LegalAction, compute_legal_actions};
    use ironsmith::decisions::context::{BooleanContext, SelectOptionsContext};
    struct Answers { reveal: bool }
    impl DecisionMaker for Answers {
        fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool { true }
        fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
            if context.options.iter().any(|option| option.description.starts_with("Do not reveal using")) {
                return vec![usize::from(self.reveal)];
            }
            SelectFirstDecisionMaker.decide_options(game, context)
        }
    }
    for native in [false, true] { for reveal in [false, true] { for leave_hand in [false, true] {
        let rules = if native { "Miracle {W}" } else {
            "You may reveal this card from your hand as you draw it if it is the first card you drew this turn. When you reveal it this way, you may cast it for {W}, Pay 2 life rather than pay its mana cost."
        };
        let card = definition("First light", &format!("Mana cost: {{5}}{{W}}\n{rules}"), (!native).then_some("Dawn"));
        assert_eq!(card.alternative_casts.iter().any(|method| method.is_miracle()), native);
        assert!(card.alternative_casts.iter().any(|method| method.uses_first_draw_reveal()));
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0);
        game.turn.active_player = PlayerId::from_index(1);
        game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::White, 1);
        let original = game.create_object_from_definition(&card, a, Zone::Library);
        let source = game.create_object_from_definition(&definition("Draw source", "", None), a, Zone::Battlefield);
        let mut answers = Answers { reveal };
        let mut context = ironsmith::effects::EffectContext::new(source, a, &mut answers);
        let outcome = ironsmith::effects::execute_effect(&mut game, &ironsmith::Effect::target_draws(1, ironsmith::target::PlayerFilter::You), &mut context).unwrap();
        let drawn = game.player(a).unwrap().hand[0];
        assert_ne!(drawn, original);
        assert!(!compute_legal_actions(&game, a).unwrap().iter().any(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == drawn)), "the draw price is unavailable at ordinary priority");
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut count = 0;
        for event in outcome.events {
            for trigger in ironsmith::triggers::check_triggers(&game, &event) { count += 1; queue.add(trigger); }
        }
        assert_eq!(count, usize::from(reveal));
        if leave_hand { game.move_object(drawn, Zone::Graveyard, ironsmith::events::EventCause::effect()); }
        if reveal {
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut answers).unwrap();
        }
        let casts = reveal && !leave_hand;
        assert_eq!(game.stack.len(), usize::from(casts));
        assert_eq!(game.player(a).unwrap().life, if casts && !native { 18 } else { 20 });
        assert_eq!(game.player(a).unwrap().mana_pool.total(), usize::from(!casts) as u32);
        if casts {
            let spell = game.object(game.stack.last().unwrap().object_id).unwrap();
            let paid = spell.cast_alternative_method.as_ref().unwrap();
            assert_eq!(paid.is_miracle(), native);
            assert!(paid.uses_first_draw_reveal());
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            assert_eq!(game.battlefield.len(), 2);
        }
        // A subsequent draw does not offer this price, even after declining it.
        game.create_object_from_definition(&card, a, Zone::Library);
        let mut context = ironsmith::effects::EffectContext::new(source, a, &mut answers);
        let second = ironsmith::effects::execute_effect(&mut game, &ironsmith::Effect::target_draws(1, ironsmith::target::PlayerFilter::You), &mut context).unwrap();
        assert!(second.events.iter().all(|event| ironsmith::triggers::check_triggers(&game, event).is_empty()));
    } } }
}

#[test]
fn reminder_identity_entry_colors_choose_counters_without_type_changing_effects() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    use ironsmith::object::CounterType;
    for native in [false, true] { for creature in [false, true] { for animate in [false, true] {
        let text = if native { "Sunburst" } else { "If this permanent is entering as a creature, ignoring type-changing effects that would affect it, it enters with a +1/+1 counter on it for each color of mana spent to cast it. Otherwise, it enters with a charge counter on it for each color of mana spent to cast it." };
        let mut builder = compiler::CardDefinitionBuilder::new(compiler::ids::CardId::new(), "Prismatic entry");
        if !native { builder = builder.with_custom_keyword("Prism"); }
        let card = compile_builder_to_runtime_definition(builder, format!("Type: Artifact{}\nMana cost: {{3}}\n{}\n{text}", if creature { " Creature" } else { "" }, if creature { "Power/Toughness: 2/2" } else { "" }), false).unwrap();
        assert_eq!(card.abilities.iter().any(|ability| matches!(&ability.kind, AbilityKind::Static(a) if a.keyword_marker_mechanic() == Some(ironsmith_core::KeywordMarkerMechanic::Sunburst))), native);
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0);
        game.turn.active_player = a;
        game.turn.priority_player = Some(a);
        game.turn.phase = ironsmith::Phase::FirstMain;
        game.turn.step = None;
        if animate {
            let animator = definition("Animator", "All artifacts are creatures in addition to their other types.", None);
            game.create_object_from_definition(&animator, a, Zone::Battlefield);
        }
        let hand = game.create_object_from_definition(&card, a, Zone::Hand);
        for color in [ironsmith::mana::ManaSymbol::White, ironsmith::mana::ManaSymbol::Blue, ironsmith::mana::ManaSymbol::Blue] {
            game.player_mut(a).unwrap().mana_pool.add(color, 1);
        }
        let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == hand)).unwrap();
        finish_reminder_cast(&mut game, action);
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        let permanent = *game.battlefield.iter().find(|id| game.object(**id).unwrap().name == "Prismatic entry").unwrap();
        assert_eq!(game.counter_count(permanent, CounterType::PlusOnePlusOne), if creature { 2 } else { 0 });
        assert_eq!(game.counter_count(permanent, CounterType::Charge), if creature { 0 } else { 2 });
        let exiled = game.move_object(permanent, Zone::Exile, ironsmith::events::EventCause::effect()).unwrap();
        let returned = game.move_object(exiled, Zone::Battlefield, ironsmith::events::EventCause::effect()).unwrap();
        assert_eq!(game.counter_count(returned, CounterType::PlusOnePlusOne), 0);
        assert_eq!(game.counter_count(returned, CounterType::Charge), 0);
    } } }
}

#[test]
fn reminder_identity_exiled_spell_grants_combat_copies_only_while_link_survives() {
    use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
    use ironsmith::decisions::context::BooleanContext;
    struct Accept;
    impl DecisionMaker for Accept { fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool { true } }
    for native in [false, true] { for leave_exile in [false, true] { for combat in [false, true] {
        let text = if native { "Cipher" } else { "You may exile this spell card encoded on a creature you control. Whenever that creature deals combat damage to a player, its controller may cast a copy of the encoded card without paying its mana cost." };
        let card = compile_to_runtime_definition("Stored spell", &format!("Type: Sorcery\nMana cost: {{3}}{{U}}\nYou gain 2 life.\n{text}"), false).unwrap();
        assert_eq!(card.spell_effect.as_ref().unwrap().all_effects().iter().filter_map(|effect| effect.downcast_ref::<ironsmith::effects::CipherEffect>()).any(|effect| !effect.descriptive), native);
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
        let bearer = game.create_object_from_definition(&definition("Bearer", "Shroud", None), a, Zone::Battlefield);
        let source = game.create_object_from_definition(&card, a, Zone::Stack);
        game.push_to_stack(ironsmith::StackEntry::new(source, a));
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut Accept).unwrap();
        assert_eq!(game.player(a).unwrap().life, 22);
        assert_eq!(game.exile.len(), 1, "encoding chooses without targeting, so Shroud is irrelevant");
        if leave_exile {
            let card = game.exile[0];
            let grave = game.move_object(card, Zone::Graveyard, ironsmith::events::EventCause::effect()).unwrap();
            game.move_object(grave, Zone::Exile, ironsmith::events::EventCause::effect());
        }
        game.refresh_continuous_state().unwrap();
        let event = ironsmith::triggers::TriggerEvent::new(ironsmith::events::DamageEvent::with_cause(bearer, ironsmith::events::DamageTarget::Player(b), 1, combat, if combat { ironsmith::events::EventCause::combat_damage(bearer) } else { ironsmith::events::EventCause::effect() }), Default::default());
        let triggers = ironsmith::triggers::check_triggers(&game, &event);
        assert_eq!(triggers.len(), usize::from(combat && !leave_exile));
        if !triggers.is_empty() {
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            for trigger in triggers { queue.add(trigger); }
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut Accept).unwrap();
            assert_eq!(game.stack.len(), 1);
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
            assert_eq!(game.player(a).unwrap().life, 24);
            assert_eq!(game.exile.len(), 1, "a spell copy cannot encode itself");
        }
    } } }
}

#[test]
fn reminder_identity_graveyard_copies_assign_each_opponent_and_keep_source_lki() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    for native in [false, true] { for zone in [Zone::Hand, Zone::Graveyard] {
        let text = if native { "Encore {2}" } else { "{2}, Exile this card from your graveyard: For each opponent, create a token copy that attacks that opponent this turn if able. They gain haste. Sacrifice them at the beginning of the next end step. Activate only as a sorcery." };
        let card = definition("Returning host", text, (!native).then_some("Reprise"));
        let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
        let a = PlayerId::from_index(0);
        game.turn.active_player = a; game.turn.priority_player = Some(a); game.turn.phase = ironsmith::Phase::FirstMain; game.turn.step = None;
        let source = game.create_object_from_definition(&card, a, zone);
        game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 2);
        let available = |game: &GameState| compute_legal_actions(game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source));
        assert_eq!(available(&game).is_some(), zone == Zone::Graveyard);
        if zone != Zone::Graveyard { continue; }
        game.turn.phase = ironsmith::Phase::Combat;
        assert!(available(&game).is_none());
        game.turn.phase = ironsmith::Phase::FirstMain;
        let action = available(&game).unwrap(); finish_reminder_cast(&mut game, action);
        assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
        assert_eq!(game.exile.len(), 1);
        let exiled = game.exile[0];
        let moved = game.move_object(exiled, Zone::Hand, ironsmith::events::EventCause::effect()).unwrap();
        game.object_mut(moved).unwrap().base_power = Some(ironsmith::card::PtValue::Fixed(99));
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.battlefield.len(), 2);
        let mut assigned = Vec::new();
        for id in &game.battlefield {
            assert_eq!(game.object(*id).unwrap().power(), Some(2));
            assert!(game.current_has_static_ability_id(*id, StaticAbilityId::Haste));
            assert!(!game.object(*id).unwrap().has_static_ability_id(StaticAbilityId::Haste), "haste is not a copiable characteristic");
            assert!(!game.is_tapped(*id));
            assigned.extend(game.required_attack_players_this_turn(*id));
        }
        assigned.sort(); assigned.dedup(); assert_eq!(assigned.len(), 2);
        assert_eq!(game.effect_store.delayed_triggers.len(), 2);
    } }
}

#[test]
fn reminder_identity_casting_characteristics_are_copiable_and_reset_on_new_incarnation() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    for native in [false, true] { for small in [false, true] {
        let text = if native { "Prototype {1}{U}{U} — 2/1" } else { "You may cast this spell with mana cost {1}{U}{U}, color determined by that mana cost, and base power and toughness 2/1. It keeps its abilities and types." };
        let mut builder = compiler::CardDefinitionBuilder::new(compiler::ids::CardId::new(), "Two forms");
        if !native { builder = builder.with_custom_keyword("Miniature"); }
        let card = compile_builder_to_runtime_definition(builder, format!("Type: Artifact Creature\nMana cost: {{7}}\nPower/Toughness: 7/5\n{text}\nReach"), false).unwrap();
        assert_eq!(card.alternative_casts.iter().any(|method| method.name() == "Prototype"), native);
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0);
        game.turn.active_player = a; game.turn.priority_player = Some(a); game.turn.phase = ironsmith::Phase::FirstMain; game.turn.step = None;
        let hand = game.create_object_from_definition(&card, a, Zone::Hand);
        if small {
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue, 2);
            game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 1);
        } else { game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 7); }
        let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| match action {
            LegalAction::CastSpell { spell_id, casting_method, .. } if *spell_id == hand =>
                matches!(casting_method, ironsmith::alternative_cast::CastingMethod::Alternative(_)) == small,
            _ => false,
        }).unwrap();
        finish_reminder_cast(&mut game, action);
        let stack = game.stack.last().unwrap().object_id;
        assert_eq!(game.current_characteristics(stack).unwrap().power, Some(if small { 2 } else { 7 }));
        assert_eq!(game.object(stack).unwrap().mana_cost.as_ref().unwrap().mana_value(), if small { 3 } else { 7 });
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        let permanent = game.battlefield[0];
        assert_eq!(game.calculated_power(permanent), Some(if small { 2 } else { 7 }));
        assert!(game.current_has_static_ability_id(permanent, StaticAbilityId::Reach));
        let copy = ironsmith::Effect::new(ironsmith::effects::CreateTokenCopyEffect::new(ironsmith::target::ChooseSpec::SpecificObject(permanent), 1, ironsmith::target::PlayerFilter::You));
        let mut context = ironsmith::effects::EffectContext::new_default(permanent, a);
        ironsmith::effects::execute_effect(&mut game, &copy, &mut context).unwrap();
        assert_eq!(game.calculated_power(game.battlefield[1]), Some(if small { 2 } else { 7 }));
        let exiled = game.move_object(permanent, Zone::Exile, ironsmith::events::EventCause::effect()).unwrap();
        let returned = game.move_object(exiled, Zone::Battlefield, ironsmith::events::EventCause::effect()).unwrap();
        assert_eq!(game.calculated_power(returned), Some(7));
        assert_eq!(game.object(returned).unwrap().mana_cost.as_ref().unwrap().mana_value(), 7);
    } }
}

#[test]
fn reminder_identity_entry_grants_only_following_abilities_until_cleanup() {
    for native in [false, true] { for target_self in [false, true] { for source_leaves in [false, true] {
        let text = if native { "Backup 3" } else { "When this creature enters, put three +1/+1 counters on target creature. If that's another creature, it gains the following abilities until end of turn." };
        let card = definition("Mentor", &format!("Flying\n{text}\nReach\nHaste"), (!native).then_some("Teach"));
        let effects = card.abilities.iter().filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(trigger) => Some(trigger.effects.all_effects()), _ => None,
        }).flatten().filter_map(|effect| effect.downcast_ref::<ironsmith::effects::BackupEffect>()).collect::<Vec<_>>();
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].descriptive, !native);
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
        let source = game.create_object_from_definition(&card, a, Zone::Hand);
        let source = enter_reminder_permanent(&mut game, source);
        let other = game.create_object_from_definition(&definition("Pupil", "", None), b, Zone::Battlefield);
        let target = if target_self { source } else { other };
        let entries = drain_reminder_triggers(&mut game);
        assert_eq!(entries.len(), 1);
        if source_leaves && !target_self { game.move_object(source, Zone::Graveyard, ironsmith::events::EventCause::effect()); }
        let mut context = ironsmith::effects::EffectContext::new_default(source, a)
            .with_targets(vec![ironsmith::effects::ResolvedTarget::Object(target)]);
        for effect in entries[0].ability.effects.all_effects() { ironsmith::effects::execute_effect(&mut game, effect, &mut context).unwrap(); }
        assert_eq!(game.counter_count(target, ironsmith::object::CounterType::PlusOnePlusOne), 3);
        assert!(game.current_has_static_ability_id(target, StaticAbilityId::Reach));
        assert!(game.current_has_static_ability_id(target, StaticAbilityId::Haste));
        assert_eq!(game.current_has_static_ability_id(target, StaticAbilityId::Flying), target_self);
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!(game.current_has_static_ability_id(target, StaticAbilityId::Reach), target_self);
        assert_eq!(game.current_has_static_ability_id(target, StaticAbilityId::Haste), target_self);
        assert_eq!(game.counter_count(target, ironsmith::object::CounterType::PlusOnePlusOne), 3);
    } } }
}

#[test]
fn reminder_identity_recurring_copy_uses_first_resolution_by_name_and_survives_zone_changes() {
    use ironsmith::decision::DecisionMaker;
    use ironsmith::decisions::context::BooleanContext;
    struct Accept;
    impl DecisionMaker for Accept { fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool { true } }
    for native in [false, true] { for already_resolved in [false, true] {
        let text = if native { "Paradigm" } else { "If this is the first time a spell you control with this spell's name has resolved this game, at the beginning of each of your precombat main phases for the rest of the game, create a copy of this object in exile. You may cast the copy without paying its mana cost. Exile this spell." };
        let card = compile_to_runtime_definition("Lasting lesson", &format!("Type: Sorcery\nMana cost: {{5}}{{U}}\nYou gain 2 life.\n{text}"), false).unwrap();
        assert_eq!(card.abilities.iter().any(|ability| matches!(&ability.kind, AbilityKind::Static(a) if a.keyword_marker_mechanic() == Some(ironsmith_core::KeywordMarkerMechanic::Paradigm))), native);
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
        if already_resolved {
            let earlier = compile_to_runtime_definition("Lasting lesson", "Type: Sorcery\nYou gain 1 life.", false).unwrap();
            let source = game.create_object_from_definition(&earlier, a, Zone::Stack);
            game.push_to_stack(ironsmith::StackEntry::new(source, a));
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        }
        let source = game.create_object_from_definition(&card, a, Zone::Stack);
        game.push_to_stack(ironsmith::StackEntry::new(source, a));
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.effect_store.delayed_triggers.len(), usize::from(!already_resolved));
        assert_eq!(game.exile.len(), 1);
        let exiled = game.exile[0];
        game.move_object(exiled, Zone::Graveyard, ironsmith::events::EventCause::effect());
        let other = ironsmith::triggers::TriggerEvent::new(ironsmith::events::BeginningOfPrecombatMainPhaseEvent::new(b), Default::default());
        assert!(ironsmith::triggers::check_delayed_triggers(&mut game, &other).is_empty());
        for turn in 2..=3 {
            game.turn.turn_number = turn;
            let event = ironsmith::triggers::TriggerEvent::new(ironsmith::events::BeginningOfPrecombatMainPhaseEvent::new(a), Default::default());
            let entries = ironsmith::triggers::check_delayed_triggers(&mut game, &event);
            assert_eq!(entries.len(), usize::from(!already_resolved));
            let mut queue = ironsmith::triggers::TriggerQueue::new(); for entry in entries { queue.add(entry); }
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut Accept).unwrap(); }
            assert_eq!(game.effect_store.delayed_triggers.len(), usize::from(!already_resolved));
        }
        assert_eq!(game.player(a).unwrap().life, if already_resolved { 23 } else { 26 });
    } }
}

#[test]
fn reminder_identity_exile_preparation_preserves_visibility_timing_and_separate_identity() {
    use ironsmith::decision::{compute_legal_actions, LegalAction, SelectFirstDecisionMaker};
    use ironsmith::special_actions::{SpecialAction, can_perform_check, perform};
    for native in [false, true] { for face_down in [false, true] {
        let text = match (native, face_down) {
            (true, true) => "Foretell {B}", (true, false) => "Plot {2}",
            (false, true) => "During your turn, you may pay {2} and exile this card from your hand face down. Cast it on a later turn for {B}, Pay 2 life.",
            (false, false) => "You may pay {2} and exile this card from your hand. Cast it as a sorcery on a later turn without paying its mana cost. Activate only as a sorcery.",
        };
        let card = definition("Prepared spell", &format!("Mana cost: {{5}}\n{text}"), (!native).then_some("Prepare"));
        assert_eq!(card.alternative_casts[0].name() == if face_down { "Foretell" } else { "Plot" }, native);
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
        game.turn.active_player = a; game.turn.priority_player = Some(a); game.turn.phase = ironsmith::Phase::FirstMain; game.turn.step = None;
        let hand = game.create_object_from_definition(&card, a, Zone::Hand);
        game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 2);
        let action = if face_down { SpecialAction::Foretell { card_id: hand } } else { SpecialAction::Plot { card_id: hand } };
        assert!(can_perform_check(&action, &game, a).is_ok());
        game.turn.active_player = b;
        assert!(can_perform_check(&action, &game, a).is_err());
        game.turn.active_player = a; game.turn.phase = ironsmith::Phase::Combat;
        assert_eq!(can_perform_check(&action, &game, a).is_ok(), face_down);
        game.turn.phase = ironsmith::Phase::FirstMain;
        perform(action, &mut game, a, &mut SelectFirstDecisionMaker).unwrap();
        assert!(game.stack.is_empty(), "preparation is a special action");
        assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
        let exiled = game.exile[0];
        assert_eq!(game.is_face_down(exiled), face_down);
        assert_eq!(game.is_foretold(exiled), face_down && native);
        assert_eq!(game.is_plotted_by(exiled, a), !face_down && native);
        if face_down { assert_eq!(game.has_foretold_this_turn(a), native); }
        if face_down { game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Black, 1); }
        let available = |game: &GameState| compute_legal_actions(game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == exiled));
        assert!(available(&game).is_none(), "preparation never authorizes a same-turn cast");
        game.turn.turn_number += 1;
        let cast = available(&game).expect("the later-turn price is available");
        finish_reminder_cast(&mut game, cast);
        assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
        assert_eq!(game.player(a).unwrap().life, if !native && face_down { 18 } else { 20 });
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.battlefield.len(), 1);
        assert!(!game.is_face_down(game.battlefield[0]));
    } }
}

#[test]
fn reminder_identity_graveyard_arrival_link_uses_permanent_and_spell_branches() {
    use ironsmith::decision::SelectFirstDecisionMaker;
    for native in [false, true] { for permanent in [false, true] { for cancel in [false, true] {
        let text = if native { "Haunt" } else { "On a permanent, this ability means \"When this permanent is put into a graveyard from the battlefield, exile it haunting target creature.\" On an instant or sorcery spell, this ability means \"When this spell is put into a graveyard during its resolution, exile it haunting target creature.\"" };
        let card = compile_to_runtime_definition("Haunting source", &format!("Type: {}\n{text}\nWhen the creature it haunts dies, you gain 3 life.", if permanent { "Artifact" } else { "Sorcery" }), false).unwrap();
        let effects = card.abilities.iter().filter_map(|ability| match &ability.kind { AbilityKind::Triggered(t) => Some(t.effects.all_effects()), _ => None }).flatten().filter_map(|effect| effect.downcast_ref::<ironsmith::effects::HauntExileEffect>()).collect::<Vec<_>>();
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].descriptive, !native);
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
        let bearer = game.create_object_from_definition(&definition("Bearer", "", None), b, Zone::Battlefield);
        let source = game.create_object_from_definition(&card, a, if permanent { Zone::Battlefield } else { Zone::Stack });
        if permanent {
            let destination = if cancel { Zone::Exile } else { Zone::Graveyard };
            let effect = ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(source), destination, false));
            let mut context = ironsmith::effects::EffectContext::new_default(source, a);
            let outcome = ironsmith::effects::execute_effect(&mut game, &effect, &mut context).unwrap();
            for event in outcome.events { game.queue_trigger_event(Default::default(), event); }
        } else {
            game.push_to_stack(ironsmith::StackEntry::new(source, a));
            if cancel {
                let counter = compile_to_runtime_definition("Counter", "Type: Instant\nCounter target spell.", false).unwrap();
                let id = game.create_object_from_definition(&counter, b, Zone::Stack);
                game.push_to_stack(ironsmith::StackEntry::new(id, b).with_targets(vec![ironsmith::game_state::Target::Object(source)]));
            }
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        }
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(queue.entries.len(), usize::from(!cancel));
        if cancel { continue; }
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.exile.len(), 1);
        assert_eq!(game.effect_store.delayed_triggers.len(), 1);
        let kill = ironsmith::Effect::destroy(ironsmith::target::ChooseSpec::SpecificObject(bearer));
        let mut context = ironsmith::effects::EffectContext::new_default(bearer, b);
        let outcome = ironsmith::effects::execute_effect(&mut game, &kill, &mut context).unwrap();
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        for event in outcome.events { game.queue_trigger_event(Default::default(), event); }
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(queue.entries.len(), 1);
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.player(a).unwrap().life, 23);
    } } }
}

#[test]
fn reminder_identity_aura_and_merging_prices_keep_target_fallback_and_keyword_identity_separate() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    use ironsmith::alternative_cast::CastingMethod;
    for native in [false, true] { for aura in [false, true] { for lose_target in [false, true] {
        let text = match (native, aura) {
            (true, true) => "Bestow {2}", (true, false) => "Mutate {2}",
            (false, true) => "You may cast this spell for {2} rather than pay its mana cost as an Aura enchantment spell with enchant creature. If its target is illegal as it begins resolving, it instead resolves using its normal types. If the resulting Aura becomes unattached or phases in unattached, it returns to its normal types.",
            (false, false) => "You may cast this spell for {2} rather than pay its mana cost targeting a non-Human creature with the same owner as this spell. As it resolves, if that target is legal, merge this spell above or below that creature without entering the battlefield. The resulting permanent has the top object's characteristics and all abilities of the merged objects. Otherwise, this spell resolves normally as a creature.",
        };
        let mut builder = compiler::CardDefinitionBuilder::new(compiler::ids::CardId::new(), "Two uses");
        if !native { builder = builder.with_custom_keyword("Join"); }
        let card = compile_builder_to_runtime_definition(builder, format!("Type: {}Creature — Beast\nMana cost: {{6}}\nPower/Toughness: 5/5\nReach\n{text}", if aura { "Enchantment " } else { "" }), false).unwrap();
        let keyword = if aura { ironsmith_core::alternative_cast_model::AlternativeCastKeyword::Bestow } else { ironsmith_core::alternative_cast_model::AlternativeCastKeyword::Mutate };
        assert_eq!(card.alternative_casts[0].keyword() == Some(keyword), native);
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
        game.turn.active_player = a; game.turn.priority_player = Some(a); game.turn.phase = ironsmith::Phase::FirstMain; game.turn.step = None;
        let hand = game.create_object_from_definition(&card, a, Zone::Hand);
        let bearer = game.create_object_from_definition(&definition("Bearer", "Vigilance", None), a, Zone::Battlefield);
        game.set_current_controller(bearer, b).unwrap();
        game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 2);
        let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, casting_method: CastingMethod::Alternative(0), .. } if *spell_id == hand)).unwrap();
        finish_reminder_cast(&mut game, action);
        assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
        if lose_target { game.move_object(bearer, Zone::Exile, ironsmith::events::EventCause::effect()); }
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        if lose_target {
            assert_eq!(game.battlefield.len(), 1);
            let permanent = game.battlefield[0];
            assert_eq!(game.calculated_power(permanent), Some(5));
            assert!(game.object(permanent).unwrap().bestow_cast_state.is_none());
        } else if aura {
            assert_eq!(game.battlefield.len(), 2);
            let attached = *game.battlefield.iter().find(|id| **id != bearer).unwrap();
            assert!(game.object(attached).unwrap().bestow_cast_state.is_some());
            game.detach_object_from_current_target(attached);
            ironsmith::rules::state_based::apply_state_based_actions_with(&mut game, &mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();
            game.refresh_continuous_state().unwrap();
            assert!(game.object(attached).unwrap().bestow_cast_state.is_none());
            assert_eq!(game.calculated_power(attached), Some(5));
        } else {
            assert_eq!(game.battlefield.len(), 1, "merging does not create a second permanent");
            let merged = game.battlefield[0];
            assert!(game.current_has_static_ability_id(merged, StaticAbilityId::Reach));
            assert!(game.current_has_static_ability_id(merged, StaticAbilityId::Vigilance));
            assert_eq!(game.controller_of_id(merged), Some(b));
        }
    } } }
}

#[test]
fn reminder_identity_paid_land_animation_preserves_base_spell_and_optional_target() {
    use ironsmith::alternative_cast::CastingMethod;
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    for native in [false, true] { for paid in [false, true] { for keyword_first in [false, true] {
        let text = if native { "Awaken 3—{2}" } else { "You may cast this spell for {2} rather than pay its mana cost. If you do, put 3 +1/+1 counters on target land you control. That land becomes a 0/0 Elemental creature with haste. It's still a land." };
        let body = if keyword_first { format!("{text}\nYou gain 4 life.") } else { format!("You gain 4 life.\n{text}") };
        let card = compile_to_runtime_definition("Growing terrain", &format!("Type: Sorcery\nMana cost: {{0}}\n{body}"), false).unwrap();
        assert_eq!(card.alternative_casts[0].name() == "Awaken", native);
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0);
        game.turn.active_player = a; game.turn.priority_player = Some(a); game.turn.phase = ironsmith::Phase::FirstMain; game.turn.step = None;
        let land = compile_to_runtime_definition("Land", "Type: Land", false).unwrap();
        let land = game.create_object_from_definition(&land, a, Zone::Battlefield);
        let hand = game.create_object_from_definition(&card, a, Zone::Hand);
        game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 2);
        let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, casting_method, .. } if *spell_id == hand && matches!(casting_method, CastingMethod::Alternative(0)) == paid)).unwrap();
        finish_reminder_cast(&mut game, action);
        assert_eq!(game.stack.last().unwrap().targets.len(), usize::from(paid));
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.player(a).unwrap().life, 24);
        assert_eq!(game.calculated_power(land), if paid { Some(3) } else { None });
        assert_eq!(game.current_has_static_ability_id(land, StaticAbilityId::Haste), paid);
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!(game.calculated_power(land), if paid { Some(3) } else { None }, "animation has no end-of-turn expiry");
    } } }
}

#[test]
fn reminder_identity_combined_halves_require_hand_pay_both_and_resolve_left_first() {
    use ironsmith::alternative_cast::CastingMethod;
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    for native in [false, true] { for in_hand in [false, true] { for funded in [false, true] {
        let text = if native { "Fuse" } else { "You may cast one or both halves of this card from your hand. If you cast both, combine their characteristics and mana costs into one spell, then follow the left half's instructions before the right half's instructions." };
        let mut left = compile_to_runtime_definition("Left instruction", &format!("Type: Instant\nMana cost: {{1}}\nYou gain 3 life.\n{text}"), false).unwrap();
        let mut right = compile_to_runtime_definition("Right instruction", &format!("Type: Sorcery\nMana cost: {{1}}\nDouble your life total.\n{text}"), false).unwrap();
        left.card.other_face = Some(right.card.id); right.card.other_face = Some(left.card.id);
        left.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::Split;
        right.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::Split;
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0);
        game.turn.active_player = a; game.turn.priority_player = Some(a); game.turn.phase = ironsmith::Phase::FirstMain; game.turn.step = None;
        game.register_linked_face_definition(&left); game.register_linked_face_definition(&right);
        let card = game.create_object_from_definition(&left, a, if in_hand { Zone::Hand } else { Zone::Graveyard });
        assert_eq!(game.object(card).unwrap().has_fuse, native);
        game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, if funded { 2 } else { 1 });
        let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, casting_method: CastingMethod::Fuse, .. } if *spell_id == card));
        assert_eq!(action.is_some(), in_hand && funded);
        if let Some(action) = action {
            finish_reminder_cast(&mut game, action);
            assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            assert_eq!(game.player(a).unwrap().life, 46);
        }
    } } }
}

#[test]
fn reminder_identity_timed_exile_counts_down_casts_optionally_and_loses_haste_with_control() {
    use ironsmith::BooleanContext;
    use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
    use ironsmith::special_actions::{perform, SpecialAction};
    struct Answer(bool);
    impl DecisionMaker for Answer { fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool { self.0 } }
    for native in [false, true] { for accept in [false, true] {
        let text = if native { "Suspend 1—{1}" } else { "If you could begin to cast this card from your hand, you may pay {1} and exile it with 1 time counters on it. This action doesn't use the stack. At the beginning of your upkeep, if this card is exiled with a time counter on it, remove a time counter from it. When the last time counter is removed from this card, if it's exiled, you may play it without paying its mana cost. If you cast a creature spell this way, it gains haste until you lose control of the spell or the permanent it becomes." };
        let card = definition("Waiting creature", text, if native { None } else { Some("Await") });
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
        game.turn.active_player = a; game.turn.priority_player = Some(a); game.turn.phase = ironsmith::Phase::FirstMain; game.turn.step = None;
        let card = game.create_object_from_definition(&card, a, Zone::Hand);
        game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 1);
        perform(SpecialAction::Suspend { card_id: card }, &mut game, a, &mut SelectFirstDecisionMaker).unwrap();
        assert!(game.stack.is_empty()); assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
        let exiled = game.exile[0];
        assert_eq!(game.object(exiled).unwrap().counters.get(&ironsmith::object::CounterType::Time), Some(&1));
        for player in [b, a] {
            let event = ironsmith::triggers::TriggerEvent::new(ironsmith::events::BeginningOfUpkeepEvent::new(player), Default::default());
            let entries = ironsmith::triggers::check_triggers(&game, &event);
            assert_eq!(entries.len(), usize::from(player == a));
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            for entry in entries { queue.add(entry); }
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        }
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(queue.entries.len(), 1);
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut Answer(accept)).unwrap();
        assert_eq!(game.stack.len(), usize::from(accept));
        if accept {
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            let creature = game.battlefield[0];
            assert!(game.current_has_static_ability_id(creature, StaticAbilityId::Haste));
            game.set_current_controller(creature, b).unwrap();
            assert!(!game.current_has_static_ability_id(creature, StaticAbilityId::Haste));
        } else { assert_eq!(game.exile.len(), 1); }
    } }
}

#[test]
fn reminder_identity_opponent_benefit_uses_source_timing_and_separate_payment_identity() {
    use ironsmith::SelectOptionsContext;
    use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker, compute_legal_actions, LegalAction};
    struct Pay(bool);
    impl DecisionMaker for Pay {
        fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            if ctx.description.starts_with("Choose optional costs") {
                return if self.0 { ctx.options.iter().find(|o| o.legal).map(|o| vec![o.index]).unwrap_or_default() } else { vec![] };
            }
            SelectFirstDecisionMaker.decide_options(game, ctx)
        }
    }
    for native in [false, true] { for permanent in [false, true] { for pay in [false, true] {
        let text = if native { "Gift a card" } else { "As an additional cost to cast this spell, you may choose an opponent. If this is a permanent, when it enters, if you paid that cost, the chosen player draws a card. If this is an instant or sorcery spell, if you paid that cost, the chosen player draws a card before its other effects." };
        let body = if permanent { text.to_owned() } else { format!("Each opponent discards a card.\n{text}") };
        let card = compile_to_runtime_definition("Benefactor", &format!("Type: {}\nMana cost: {{0}}\n{body}", if permanent { "Artifact" } else { "Sorcery" }), false).unwrap();
        assert_eq!(card.optional_costs[0].kind == ironsmith_core::OptionalCostKind::Gift, native);
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0); let b = PlayerId::from_index(1);
        game.turn.active_player = a; game.turn.priority_player = Some(a); game.turn.phase = ironsmith::Phase::FirstMain; game.turn.step = None;
        game.create_object_from_definition(&definition("Present", "", None), b, Zone::Library);
        let hand = game.create_object_from_definition(&card, a, Zone::Hand);
        let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == hand)).unwrap();
        finish_reminder_cast_with(&mut game, action, &mut Pay(pay));
        assert_eq!(game.stack.last().unwrap().optional_costs_paid.was_paid_label("Gift"), native && pay);
        assert!(game.player(b).unwrap().hand.is_empty());
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        if permanent {
            assert!(game.player(b).unwrap().hand.is_empty(), "benefit waits for entry trigger");
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
            assert_eq!(queue.entries.len(), usize::from(pay));
            ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            if pay { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
            assert_eq!(game.player(b).unwrap().hand.len(), usize::from(pay));
        } else {
            assert!(game.player(b).unwrap().hand.is_empty());
            assert_eq!(game.player(b).unwrap().graveyard.len(), usize::from(pay), "draw occurs before base spell's discard");
        }
    } } }
}

#[test]
fn reminder_identity_shared_type_reveal_accepts_kindred_without_changing_keyword_identity() {
    use ironsmith::decision::DecisionMaker;
    use ironsmith::SelectObjectsContext;
    struct Reveal;
    impl DecisionMaker for Reveal {
        fn decide_objects(&mut self, _: &GameState, ctx: &SelectObjectsContext) -> Vec<ironsmith::ObjectId> { ctx.candidates.iter().filter(|c| c.legal).take(ctx.max.unwrap_or(usize::MAX)).map(|c| c.id).collect() }
    }
    for native in [false, true] {
        let text = if native { "Amplify 1" } else { "As this creature enters, reveal any number of cards from your hand that share a creature type with it. This creature enters with a +1/+1 counter on it for each card revealed this way." };
        let card = compile_to_runtime_definition("Revealer", &format!("Type: Creature — Elf\nPower/Toughness: 2/2\n{text}"), false).unwrap();
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0);
        for types in ["Creature — Elf", "Kindred Sorcery — Elf", "Creature — Human"] {
            let card = compile_to_runtime_definition("Hand card", &format!("Type: {types}\nPower/Toughness: 1/1"), false).unwrap();
            game.create_object_from_definition(&card, a, Zone::Hand);
        }
        let source = game.create_object_from_definition(&card, a, Zone::Stack);
        game.push_to_stack(ironsmith::StackEntry::new(source, a));
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut Reveal).unwrap();
        assert_eq!(game.calculated_power(game.battlefield[0]), Some(4));
        assert_eq!(game.player(a).unwrap().hand.len(), 3, "revealed cards stay in hand");
        assert!(game.stack.is_empty(), "as-enters replacement does not trigger");
    }
}

#[test]
fn reminder_identity_source_token_copy_keeps_types_removes_cost_and_has_no_named_creation() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    for native in [false, true] { for eternal in [false, true] {
        let text = match (native, eternal) {
            (true, true) => "Eternalize {3}", (true, false) => "Embalm {3}",
            (false, true) => "{3}, Exile this card from your graveyard: Create a token that's a copy of this card, except it's 4/4, it's black, it has no mana cost, and it's a Zombie in addition to its other types. Activate only as a sorcery.",
            (false, false) => "{3}, Exile this card from your graveyard: Create a token that's a copy of this card, except it's white, it has no mana cost, and it's a Zombie in addition to its other types. Activate only as a sorcery.",
        };
        let card = compile_to_runtime_definition("Returning token", &format!("Type: Creature — Bird Wizard\nMana cost: {{5}}{{U}}\nPower/Toughness: 1/3\nFlying\n{text}"), false).unwrap();
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0);
        game.turn.active_player = a; game.turn.priority_player = Some(a); game.turn.phase = ironsmith::Phase::FirstMain; game.turn.step = None;
        let source = game.create_object_from_definition(&card, a, Zone::Graveyard);
        game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 3);
        let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)).unwrap();
        finish_reminder_cast(&mut game, action);
        assert_eq!(game.exile.len(), 1);
        assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        let token = game.battlefield[0]; let object = game.object(token).unwrap();
        assert!(object.mana_cost.is_none());
        for subtype in [ironsmith::types::Subtype::Bird, ironsmith::types::Subtype::Wizard, ironsmith::types::Subtype::Zombie] { assert!(object.subtypes.contains(&subtype)); }
        assert_eq!(game.calculated_power(token), Some(if eternal { 4 } else { 1 }));
        assert!(game.current_has_static_ability_id(token, StaticAbilityId::Flying));
        let activated = card.abilities.iter().find_map(|ability| match &ability.kind { AbilityKind::Activated(a) => Some(a), _ => None }).unwrap();
        let copy = activated.effects.all_effects().into_iter().find_map(|e| e.downcast_ref::<ironsmith::effects::CreateTokenCopyEffect>()).unwrap();
        assert_eq!(copy.creation_method.is_some(), native);
        assert!(copy.clear_mana_cost);
    } }
}

#[test]
fn reminder_identity_additional_tap_price_preserves_printed_mana_and_has_no_named_action() {
    use ironsmith::decision::{compute_legal_actions, LegalAction};
    for native in [false, true] {
        let text = if native { "As an additional cost to cast this spell, waterbend {2}." } else { "As an additional cost to cast this spell, pay {2} using any combination of mana and tapping untapped artifacts and creatures you control for {1} each." };
        let card = compile_to_runtime_definition("Tap payment", &format!("Type: Sorcery\nMana cost: {{U}}\n{text}\nYou gain 3 life."), false).unwrap();
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0);
        game.turn.active_player = a; game.turn.priority_player = Some(a); game.turn.phase = ironsmith::Phase::FirstMain; game.turn.step = None;
        let hand = game.create_object_from_definition(&card, a, Zone::Hand);
        let one = game.create_object_from_definition(&definition("One", "", None), a, Zone::Battlefield);
        let two = game.create_object_from_definition(&definition("Two", "", None), a, Zone::Battlefield);
        game.player_mut(a).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue, 1);
        let action = compute_legal_actions(&game, a).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == hand)).unwrap();
        finish_reminder_cast(&mut game, action);
        assert!(game.is_tapped(one) && game.is_tapped(two));
        assert_eq!(game.player(a).unwrap().mana_pool.total(), 0);
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.player(a).unwrap().life, 23);
    }
}

#[test]
fn reminder_identity_cumulative_upkeep_either_payment_keeps_the_permanent() {
    use ironsmith::decision::SelectFirstDecisionMaker;
    // "{G} or {W}" lets each payment use either color; with neither, the
    // permanent is sacrificed (CR 702.24a).
    for (green, white, kept) in [(1, 0, true), (0, 1, true), (0, 0, false)] {
        let card = compile_to_runtime_definition("Either Upkeep", "Type: Enchantment\nMana cost: {G}{W}\nCumulative upkeep {G} or {W}", false).unwrap();
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let a = PlayerId::from_index(0);
        game.turn.active_player = a;
        let source = game.create_object_from_definition(&card, a, Zone::Battlefield);
        let pool = &mut game.player_mut(a).unwrap().mana_pool;
        pool.add(ironsmith::mana::ManaSymbol::Green, green);
        pool.add(ironsmith::mana::ManaSymbol::White, white);
        let event = ironsmith::triggers::TriggerEvent::new(ironsmith::events::BeginningOfUpkeepEvent::new(a), Default::default());
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        for entry in ironsmith::triggers::check_triggers(&game, &event) { queue.add(entry); }
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.battlefield.contains(&source), kept, "green={green} white={white}");
        if kept {
            assert_eq!(game.player(a).unwrap().mana_pool.total(), 0, "one age counter costs one payment");
        }
    }
}

/// The web binder rewrites a custom participle ("if it was blinked") to the
/// payment its definition creates; that reading never acquires Kicker's.
#[test]
fn reminder_identity_custom_participle_tests_its_own_payment_not_kicker() {
    let condition = |card: &ironsmith::cards::CardDefinition| {
        card.abilities.iter().find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered.intervening_if.clone(),
            _ => None,
        }).unwrap()
    };
    let native = definition("Native", "Kicker {1}{U}\nWhen this creature enters, if it was kicked, draw a card.", None);
    let custom = definition(
        "Blinker",
        "As an additional cost to cast this spell, you may pay {1}{U}.\nWhen this creature enters, if its additional cost was paid, draw a card.",
        Some("blink"),
    );
    assert_eq!(condition(&native), ironsmith::ConditionExpr::ThisSpellWasKicked);
    let [optional] = custom.optional_costs.as_slice() else { unreachable!() };
    assert_eq!(condition(&custom), ironsmith::ConditionExpr::ThisSpellPaidLabel(optional.reference.clone()));
    assert_eq!(format!("{:?}", optional.cost), format!("{:?}", native.optional_costs[0].cost));
    let mut paid = ironsmith_core::OptionalCostsPaid::default();
    paid.mark_label_paid(optional.reference.clone());
    assert!(!paid.was_kicked());
    let identities = |card: &ironsmith::cards::CardDefinition| card.abilities.iter().filter_map(|ability| match &ability.kind {
        AbilityKind::Static(ability) => ability.custom_keyword_name().map(str::to_owned),
        _ => None,
    }).collect::<Vec<_>>();
    assert_eq!(identities(&custom), ["blink"]);
    assert!(identities(&native).is_empty());
    // An unbound participle is not vocabulary the compiler guesses at.
    let raw = compile_builder_to_runtime_definition(
        compiler::CardDefinitionBuilder::new(compiler::ids::CardId::new(), "Raw").with_custom_keyword("blink"),
        "Type: Creature\nPower/Toughness: 2/2\nAs an additional cost to cast this spell, you may pay {1}{U}.\nWhen this creature enters, if it was blinked, draw a card.",
        false,
    );
    assert!(raw.is_err());
}
