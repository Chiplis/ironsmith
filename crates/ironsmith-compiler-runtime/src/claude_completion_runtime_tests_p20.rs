//! Claude package 20 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;
use ironsmith::{PlayerId, Zone};

fn p20_all_effects(definition: &ironsmith::cards::CardDefinition) -> Vec<Effect> {
    fn walk(effect: &Effect, out: &mut Vec<Effect>) {
        out.push(effect.clone());
        effect.visit_child_effects(&mut |child| walk(child, out));
    }
    let mut out = Vec::new();
    if let Some(program) = &definition.spell_effect {
        for effect in program.all_effects() {
            walk(effect, &mut out);
        }
    }
    for ability in &definition.abilities {
        match &ability.kind {
            ironsmith::ability::AbilityKind::Activated(activated) => {
                for effect in activated.effects.all_effects() {
                    walk(effect, &mut out);
                }
            }
            ironsmith::ability::AbilityKind::Triggered(triggered) => {
                for effect in triggered.effects.all_effects() {
                    walk(effect, &mut out);
                }
            }
            _ => {}
        }
    }
    out
}

const PALIANO_TEXT: &str = "Type: Legendary Land\nReveal this card as you draft it. The player to your right chooses a color, you choose another color, then the player to your left chooses a third color.\n{T}: Add one mana of any color chosen as you drafted cards named Paliano, the High City.";

/// The Conspiracy draft records Paliano's three revealed color choices, the
/// game reads them back per drafter, and the mana ability offers only those.
#[test]
fn paliano_adds_only_colors_its_controller_chose_while_drafting_it() {
    use ironsmith::color::{Color, ColorSet};
    let paliano = compile_to_runtime_definition("Paliano, the High City", PALIANO_TEXT, false)
        .expect("Paliano, the High City should compile");
    let add = p20_all_effects(&paliano)
        .into_iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::AddManaOfAnyColorEffect>().cloned())
        .expect("drafted-color mana production");
    assert!(
        add.available_colors_chosen_while_drafting_named
            .as_deref()
            .is_some_and(|name| name.to_ascii_lowercase().replace(',', "") == "paliano the high city"),
        "{add:?}"
    );

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let filler = |name: &str| {
        ironsmith::cards::CardDefinition::new(
            ironsmith::CardBuilder::new(ironsmith::ids::CardId::new(), name).build(),
        )
    };
    let packs = |first: ironsmith::cards::CardDefinition, prefix: &str| {
        vec![vec![first], vec![filler(&format!("{prefix} R2"))], vec![filler(&format!("{prefix} R3"))]]
    };
    let new_draft = || {
        ironsmith::ConspiracyDraftState::new(
            vec![alice, bob],
            vec![(alice, packs(paliano.clone(), "Alice")), (bob, packs(filler("Bob R1"), "Bob"))],
        )
        .unwrap()
    };
    let pick = |draft: &ironsmith::ConspiracyDraftState, player: PlayerId, note: Option<&str>| {
        let id = draft.current_pack_view(player, player)[0].id;
        ironsmith::DraftSelection {
            player,
            card_ids: vec![id],
            exchange_face_up: None,
            public_notes: note
                .map(|note| std::collections::HashMap::from([(id, note.to_string())]))
                .unwrap_or_default(),
        }
    };

    // Repeated or too few colors are not a legal Paliano record.
    for bad in ["red, red, white", "red, green"] {
        let mut draft = new_draft();
        let selections = vec![pick(&draft, alice, Some(bad)), pick(&draft, bob, None)];
        assert!(draft.draft_step(selections).is_err(), "{bad}");
    }

    let mut draft = new_draft();
    let selections = vec![pick(&draft, alice, Some("Red, green, then white")), pick(&draft, bob, None)];
    draft.draft_step(selections).unwrap();
    while !draft.is_complete() {
        let selections = vec![pick(&draft, alice, None), pick(&draft, bob, None)];
        draft.draft_step(selections).unwrap();
    }
    let chosen = ColorSet::from_color(Color::Red)
        .union(ColorSet::from_color(Color::Green))
        .union(ColorSet::from_color(Color::White));
    assert_eq!(draft.draft_chosen_colors(alice, "Paliano, the High City"), chosen);
    assert!(draft.draft_chosen_colors(bob, "Paliano, the High City").is_empty());

    let ability = paliano
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("mana ability");
    for (controller, expected_total) in [(alice, 1), (bob, 0)] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.record_conspiracy_draft_choices(&draft);
        assert_eq!(game.draft_chosen_colors(alice, "PALIANO THE HIGH CITY"), chosen);
        let source = game.create_object_from_definition(&paliano, controller, Zone::Battlefield);
        let mut ctx = ironsmith::effects::EffectContext::new_default(source, controller);
        for effect in ability.effects.all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        let pool = &game.player(controller).unwrap().mana_pool;
        assert_eq!(pool.total_wide(), expected_total, "{controller:?}");
        assert_eq!(pool.blue + pool.black, 0, "only drafted colors are offered");
        if expected_total == 1 {
            assert_eq!(pool.white + pool.red + pool.green, 1);
        }
    }
}

const QUAKEBRINGER_TEXT: &str = "Mana cost: {3}{R}{R}\nType: Creature — Giant Berserker\nPower/Toughness: 5/4\nYour opponents can't gain life.\nAt the beginning of your upkeep, Quakebringer deals 2 damage to each opponent. This ability triggers only if Quakebringer is on the battlefield or if Quakebringer is in your graveyard and you control a Giant.\nForetell {2}{R}{R}";

/// "This ability triggers only if C" gates the trigger event but, unlike an
/// intervening if, is not rechecked when the ability resolves.
#[test]
fn quakebringer_trigger_only_condition_is_not_rechecked_on_resolution() {
    use ironsmith::condition_eval::{
        ExternalEvaluationContext, ExternalEvaluationOptions, evaluate_condition_external_checked,
    };
    let definition = compile_to_runtime_definition("Quakebringer", QUAKEBRINGER_TEXT, false)
        .expect("Quakebringer should compile");
    let ability = definition
        .abilities
        .iter()
        .find(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Triggered(_)))
        .expect("upkeep trigger");
    assert!(ability.functional_zones.contains(&Zone::Battlefield), "{:?}", ability.functional_zones);
    assert!(ability.functional_zones.contains(&Zone::Graveyard), "{:?}", ability.functional_zones);
    let ironsmith::ability::AbilityKind::Triggered(triggered) = &ability.kind else { unreachable!() };
    let condition = triggered.intervening_if.clone().expect("trigger-only gate");
    assert!(
        format!("{condition:?}").starts_with("TriggerEventOnly(Or(SourceIsInZone(Battlefield), And(SourceIsInZone(Graveyard)"),
        "{condition:?}"
    );

    let alice = PlayerId::from_index(0);
    for (zone, as_triggered) in [(Zone::Battlefield, true), (Zone::Graveyard, false)] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, alice, zone);
        let at = |ignore_trigger_limits: bool| ExternalEvaluationContext {
            controller: alice,
            source,
            options: ExternalEvaluationOptions {
                triggering_object_current: ignore_trigger_limits,
                ignore_trigger_limits,
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(
            evaluate_condition_external_checked(&game, &condition, &at(false), None).unwrap(),
            as_triggered,
            "{zone:?}: checked as the upkeep begins"
        );
        assert!(
            evaluate_condition_external_checked(&game, &condition, &at(true), None).unwrap(),
            "{zone:?}: never rechecked on resolution"
        );
    }
}

const ORACLE_EN_VEC_TEXT: &str = "Mana cost: {1}{W}\nType: Creature — Human Wizard\nPower/Toughness: 1/1\n{T}: Target opponent chooses any number of creatures they control. During that player's next turn, the chosen creatures attack if able, and other creatures can't attack. At the beginning of that turn's end step, destroy each of the chosen creatures that didn't attack this turn. Activate only during your turn.";

/// The opponent's chosen set must attack during that player's next turn,
/// every other creature can't, and a delayed end-step destroy follows.
#[test]
fn oracle_en_vec_binds_next_turn_attack_rules_to_the_chosen_creatures() {
    use ironsmith::effect::{Restriction, RestrictionStart};
    use ironsmith::target::TaggedOpbjectRelation;
    let definition = compile_to_runtime_definition("Oracle en-Vec", ORACLE_EN_VEC_TEXT, false)
        .expect("Oracle en-Vec should compile");
    let effects = p20_all_effects(&definition);
    let rules: Vec<_> = effects
        .iter()
        .filter_map(|effect| effect.downcast_ref::<ironsmith::effects::CantEffect>())
        .collect();
    let relation_of = |filter: &ironsmith::target::ObjectFilter| {
        filter.tagged_constraints.first().map(|constraint| constraint.relation.clone())
    };
    let must = rules
        .iter()
        .find(|rule| matches!(&rule.restriction, Restriction::MustAttack(filter) if relation_of(filter) == Some(TaggedOpbjectRelation::IsTaggedObject)))
        .unwrap_or_else(|| panic!("chosen creatures must attack: {rules:?}"));
    let others = rules
        .iter()
        .find(|rule| matches!(&rule.restriction, Restriction::Attack(filter) if relation_of(filter) == Some(TaggedOpbjectRelation::IsNotTaggedObject)))
        .unwrap_or_else(|| panic!("other creatures can't attack: {rules:?}"));
    for rule in [must, others] {
        assert!(matches!(rule.start, RestrictionStart::NextTurn(_)), "{rule:?}");
        assert!(
            !matches!(rule.start, RestrictionStart::NextTurn(ironsmith::target::PlayerFilter::IteratedPlayer)),
            "'that player' must be bound to the target opponent: {rule:?}"
        );
    }
    let delayed = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::ScheduleDelayedTriggerEffect>())
        .expect("end-step destroy");
    assert!(format!("{delayed:?}").contains("DestroyEffect"), "{delayed:?}");
}

const PSYCHIC_PAPER_TEXT: &str = "Mana cost: {2}\nType: Artifact — Equipment\nAs this Equipment becomes attached to a creature, choose a creature card name and a creature type.\nEquipped creature has ward {1}, it can't be blocked, and its name and creature type are the last chosen name and creature type.\nEquip {2}";

/// The attach-time choices are stored on Psychic Paper, and the equipped
/// creature takes the last chosen name and exactly the last creature type.
#[test]
fn psychic_paper_equipped_creature_takes_the_last_chosen_name_and_creature_type() {
    use ironsmith::static_abilities::StaticAbilityId;
    use ironsmith::types::Subtype;
    let definition = compile_to_runtime_definition("Psychic Paper", PSYCHIC_PAPER_TEXT, false)
        .expect("Psychic Paper should compile");
    let ids: Vec<_> = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Static(ability) => Some(ability.id()),
            _ => None,
        })
        .collect();
    for id in [
        StaticAbilityId::ChooseCreatureCardNameAndTypeAsBecomesAttached,
        StaticAbilityId::AttachedHasSourceChosenNameAndCreatureType,
    ] {
        assert!(ids.contains(&id), "{id:?} missing from {ids:?}");
    }
    assert!(format!("{:?}", definition.abilities).contains("Ward"), "{ids:?}");

    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let bear = ironsmith::CardBuilder::new(ironsmith::ids::CardId::new(), "Test Bear")
        .card_types(vec![ironsmith::types::CardType::Creature])
        .subtypes(vec![Subtype::Bear])
        .power_toughness(ironsmith::PowerToughness::fixed(2, 2))
        .build();
    let creature = game.create_object_from_card(&bear, alice, Zone::Battlefield);
    let paper = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    assert!(game.attach_object_to_target(paper, ironsmith::object::AttachmentTarget::Object(creature)));
    game.refresh_continuous_state().unwrap();
    let chars = game.current_characteristics(creature).unwrap();
    assert_eq!(String::from(&chars.name), "Test Bear", "no choice yet: unchanged");
    assert!(chars.subtypes.contains(&Subtype::Bear));
    assert!(!game.can_be_blocked(creature), "the equipped creature cannot be blocked");

    game.set_chosen_named_option(paper, "Grizzly Bears".to_string());
    game.set_chosen_creature_type(paper, Subtype::Elf);
    game.refresh_continuous_state().unwrap();
    let chars = game.current_characteristics(creature).unwrap();
    assert_eq!(String::from(&chars.name), "Grizzly Bears");
    assert!(chars.subtypes.contains(&Subtype::Elf), "{:?}", chars.subtypes);
    assert!(!chars.subtypes.contains(&Subtype::Bear), "{:?}", chars.subtypes);
    let paper_chars = game.current_characteristics(paper).unwrap();
    assert_eq!(String::from(&paper_chars.name), "Psychic Paper", "only the equipped creature changes");
}

const PIRACY_TEXT: &str = "Mana cost: {U}{U}\nType: Sorcery\nUntil end of turn, you may tap lands you don't control for mana. Spend this mana only to cast spells.";

/// Piracy lets its caster activate {T} mana abilities of an opponent's land
/// this turn; the mana goes to the caster and keeps "only to cast spells".
#[test]
fn piracy_lets_its_caster_tap_an_opponents_land_for_spell_only_mana() {
    use ironsmith::special_actions::{SpecialAction, can_perform_check, perform};
    struct Picker;
    impl ironsmith::decision::DecisionMaker for Picker {}

    let piracy = compile_to_runtime_definition("Piracy", PIRACY_TEXT, false)
        .expect("Piracy should compile");
    let effects = p20_all_effects(&piracy);
    assert!(
        effects.iter().any(|effect| effect.downcast_ref::<ironsmith::effects::ManaRestrictedEffect>().is_some()),
        "the spending restriction wraps the permission: {effects:?}"
    );
    assert!(effects
        .iter()
        .any(|effect| effect.downcast_ref::<ironsmith::effects::GrantTapUncontrolledLandsForManaEffect>().is_some()));
    let forest = ironsmith::cards::builders::CardDefinitionBuilder::new(
        ironsmith::ids::CardId::new(), "Mana land witness")
        .card_types(vec![ironsmith::types::CardType::Land])
        .with_ability(ironsmith::ability::Ability::mana(
            ironsmith::TotalCost::free(), vec![ironsmith::mana::ManaSymbol::Green]))
        .build();
    let ability_index = forest
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_)))
        .expect("Forest mana ability");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let bobs_forest = game.create_object_from_definition(&forest, bob, Zone::Battlefield);
    let action = || SpecialAction::ActivateManaAbility { permanent_id: bobs_forest, ability_index };
    assert!(can_perform_check(&action(), &game, alice).is_err(), "no permission yet");

    let source = game.create_object_from_definition(&piracy, alice, Zone::Stack);
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    for effect in piracy.spell_effect.as_ref().expect("spell effect").all_effects() {
        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    assert!(game.has_uncontrolled_land_mana_permission(alice));
    assert!(!game.has_uncontrolled_land_mana_permission(bob));
    assert!(can_perform_check(&action(), &game, alice).is_ok());

    perform(action(), &mut game, alice, &mut Picker).unwrap();
    assert!(game.is_tapped(bobs_forest));
    let alice_state = game.player(alice).unwrap();
    // The pool includes restricted mana; its separate units govern spending.
    assert_eq!(alice_state.mana_pool.total_wide(), 1);
    assert_eq!(alice_state.restricted_mana.len(), 1);
    assert!(!alice_state.restricted_mana[0].restrictions.is_empty());
    assert_eq!(game.player(bob).unwrap().restricted_mana.len(), 0);
    assert_eq!(game.player(bob).unwrap().mana_pool.total_wide(), 0);
    let cost = ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::mana::ManaSymbol::Green]);
    assert!(!game.can_pay_mana_cost_with_reason(alice, Some(source), &cost, 0,
        ironsmith::costs::PaymentReason::ActivateAbility));
    assert!(game.can_pay_mana_cost_with_reason(alice, Some(source), &cost, 0,
        ironsmith::costs::PaymentReason::CastSpell));
    assert!(game.try_pay_mana_cost_with_reason(alice, Some(source), &cost, 0,
        ironsmith::costs::PaymentReason::CastSpell).unwrap());
    assert_eq!(game.player(alice).unwrap().mana_pool.total_wide(), 0);
    assert!(game.player(alice).unwrap().restricted_mana.is_empty());
}

const PSYCHIC_BATTLE_TEXT: &str = "Mana cost: {3}{U}{U}\nType: Enchantment\nWhenever a player chooses one or more targets, each player reveals the top card of their library. The player who reveals the card with the greatest mana value may change the target or targets. If two or more cards are tied for greatest, the target or targets remain unchanged. Changing targets this way doesn't trigger abilities of permanents named Psychic Battle.";

/// The trigger fires once per targeting spell or ability, a sole reveal
/// winner may retarget it, and its own target changes don't retrigger it.
#[test]
fn psychic_battle_reveal_winner_may_retarget_without_retriggering_itself() {
    use ironsmith::events::spells::BecomesTargetedEvent;
    use ironsmith::triggers::{TriggerEvent, check_triggers};
    let definition = compile_to_runtime_definition("Psychic Battle", PSYCHIC_BATTLE_TEXT, false)
        .expect("Psychic Battle should compile");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("targets-chosen trigger");
    let trigger = format!("{:?}", triggered.trigger);
    assert!(trigger.contains("PlayerOrObjectBecomesTargetedBySourceControllerTrigger"), "{trigger}");
    assert!(trigger.contains("once_per_stack_object: true"), "{trigger}");
    let effects = p20_all_effects(&definition);
    assert!(effects
        .iter()
        .any(|effect| effect.downcast_ref::<ironsmith::effects::KeepGreatestManaValuePlayersEffect>().is_some()));
    let retarget = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::RetargetStackObjectEffect>())
        .expect("winner's retarget");
    assert!(retarget.doesnt_trigger_same_named_sources, "{retarget:?}");
    assert!(!retarget.require_change, "the winner may keep the targets: {retarget:?}");
    assert_eq!(retarget.chooser, ironsmith::target::PlayerFilter::IteratedPlayer);

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let battle = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let bear = ironsmith::CardBuilder::new(ironsmith::ids::CardId::new(), "Target Bear")
        .card_types(vec![ironsmith::types::CardType::Creature])
        .power_toughness(ironsmith::PowerToughness::fixed(2, 2))
        .build();
    let target = game.create_object_from_card(&bear, alice, Zone::Battlefield);
    let spell = ironsmith::CardBuilder::new(ironsmith::ids::CardId::new(), "Targeting Spell")
        .card_types(vec![ironsmith::types::CardType::Instant])
        .build();
    let spell = game.create_object_from_card(&spell, bob, Zone::Stack);
    game.refresh_continuous_state().unwrap();
    let event = |name: Option<&str>| {
        TriggerEvent::new_with_provenance(
            BecomesTargetedEvent::new(target, spell, bob, false)
                .with_untriggered_source_name(name.map(str::to_string)),
            ironsmith::provenance::ProvNodeId::default(),
        )
    };
    let from_battle = |entries: Vec<ironsmith::triggers::TriggeredAbilityEntry>| {
        entries.into_iter().filter(|entry| entry.source == battle).count()
    };
    assert_eq!(from_battle(check_triggers(&game, &event(None))), 1, "any chosen target triggers it");
    assert_eq!(
        from_battle(check_triggers(&game, &event(Some("Psychic Battle")))),
        0,
        "its own target changes don't trigger permanents named Psychic Battle"
    );
}
