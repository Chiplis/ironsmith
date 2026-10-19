//! Claude package 21 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::events::cause::{CauseAbilityKind, EventCause};
use ironsmith::ids::CardId;
use ironsmith::object::CounterType;
use ironsmith::target::ChooseSpec;
use ironsmith::types::CardType;
use ironsmith::{PlayerId, Zone};

fn p21_creature(game: &mut ironsmith::GameState, name: &str, owner: PlayerId) -> ironsmith::ids::ObjectId {
    let card = CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 4))
        .build();
    game.create_object_from_card(&card, owner, Zone::Battlefield)
}

fn p21_ability_cause(source: ironsmith::ids::ObjectId, controller: PlayerId,
    kind: Option<CauseAbilityKind>) -> EventCause {
    let mut cause = EventCause::from_effect(source, controller);
    cause.resolving_ability = kind;
    cause
}

#[test]
fn master_multiplied_protects_tokens_from_its_controllers_triggered_abilities_only() {
    let definition = compile_to_runtime_definition("The Master, Multiplied",
        "Mana cost: {4}{B}{R}\nType: Legendary Creature — Time Lord Rogue\nPower/Toughness: 4/3\nMyriad\nThe \"legend rule\" doesn't apply to creature tokens you control.\nTriggered abilities you control can't cause you to sacrifice or exile creature tokens you control.", false)
        .expect("The Master, Multiplied should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("BeSacrificedByCause") && debug.contains("BeExiledByCause"), "{debug}");
    assert!(debug.contains("Triggered"), "{debug}");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let master = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let token = p21_creature(&mut game, "Token copy", alice);
    game.object_mut(token).unwrap().kind = ironsmith::object::ObjectKind::Token;
    let nontoken = p21_creature(&mut game, "Nontoken", alice);
    game.refresh_continuous_state().unwrap();
    game.update_cant_effects();
    let triggered = p21_ability_cause(master, alice, Some(CauseAbilityKind::Triggered));
    let modular = p21_ability_cause(master, alice, Some(CauseAbilityKind::ModularTrigger));
    let activated = p21_ability_cause(master, alice, Some(CauseAbilityKind::Activated));
    let opponent_trigger = p21_ability_cause(master, bob, Some(CauseAbilityKind::Triggered));
    assert!(!game.can_be_sacrificed_with_cause(token, &triggered));
    assert!(!game.can_be_sacrificed_with_cause(token, &modular));
    assert!(game.can_be_sacrificed_with_cause(token, &activated));
    assert!(game.can_be_sacrificed_with_cause(token, &opponent_trigger));
    assert!(game.can_be_sacrificed_with_cause(nontoken, &triggered));
    assert!(!game.can_be_exiled_with_cause(token, &triggered));
    assert!(game.can_be_exiled_with_cause(token, &activated));
    assert!(game.can_be_exiled_with_cause(nontoken, &triggered));
    // The exile event itself does nothing to the protected token.
    for (cause, exiled) in [(triggered, false), (activated, true)] {
        let mut ctx = ironsmith::effects::EffectContext::new_default(master, alice).with_cause(cause);
        ironsmith::effects::execute_effect(&mut game,
            &Effect::exile(ChooseSpec::SpecificObject(token)), &mut ctx).unwrap();
        drop(ctx);
        assert_eq!(!game.battlefield.contains(&token), exiled);
    }
}

#[test]
fn zabaz_adds_a_counter_only_to_placements_by_modular_triggers() {
    let definition = compile_to_runtime_definition("Zabaz, the Glimmerwasp",
        "Mana cost: {1}\nType: Legendary Artifact Creature — Insect\nPower/Toughness: 0/0\nModular 1\nIf a modular triggered ability would put one or more +1/+1 counters on a creature you control, that many plus one +1/+1 counters are put on it instead.\n{R}: Destroy target artifact you control.\n{W}: Zabaz gains flying until end of turn.", false)
        .expect("Zabaz should compile");
    let alice = PlayerId::from_index(0);
    for (kind, expected) in [
        (Some(CauseAbilityKind::ModularTrigger), 3),
        (Some(CauseAbilityKind::Triggered), 2),
        (Some(CauseAbilityKind::Activated), 2),
        (None, 2),
    ] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let zabaz = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let recipient = p21_creature(&mut game, "Recipient", alice);
        game.refresh_continuous_state().unwrap();
        let mut ctx = ironsmith::effects::EffectContext::new_default(zabaz, alice)
            .with_cause(p21_ability_cause(zabaz, alice, kind));
        ironsmith::effects::execute_effect(&mut game,
            &Effect::put_counters(CounterType::PlusOnePlusOne, 2, ChooseSpec::SpecificObject(recipient)),
            &mut ctx).unwrap();
        drop(ctx);
        assert_eq!(game.counter_count(recipient, CounterType::PlusOnePlusOne), expected, "{kind:?}");
    }
}

#[test]
fn wolverine_heals_marked_damage_before_the_same_damage_is_dealt() {
    let definition = compile_to_runtime_definition("Wolverine, Fierce Fighter",
        "Mana cost: {2}{R}{G}\nType: Legendary Creature — Mutant Berserker Hero\nPower/Toughness: 3/5\nHaste\nWhen Wolverine enters, he fights up to one other target creature.\nIf damage would be dealt to Wolverine, instead that damage is dealt, but all other damage already dealt to him is healed.", false)
        .expect("Wolverine should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("HealDamageBeforeDamageToSelf"), "{debug}");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let wolverine = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let attacker = p21_creature(&mut game, "Damage source", bob);
    let bystander = p21_creature(&mut game, "Bystander", alice);
    game.refresh_continuous_state().unwrap();
    game.mark_damage(wolverine, 4);
    game.mark_damage(bystander, 1);
    let mut ctx = ironsmith::effects::EffectContext::new_default(attacker, bob);
    ironsmith::effects::execute_effect(&mut game,
        &Effect::deal_damage(2, ChooseSpec::SpecificObject(wolverine)), &mut ctx).unwrap();
    ironsmith::effects::execute_effect(&mut game,
        &Effect::deal_damage(2, ChooseSpec::SpecificObject(bystander)), &mut ctx).unwrap();
    drop(ctx);
    assert_eq!(game.damage_on(wolverine), 2, "old damage is healed, the new damage is dealt");
    assert_eq!(game.damage_on(bystander), 3, "only Wolverine is healed");
}

#[test]
fn ydwen_unblocks_attackers_it_alone_blocked_after_leaving_combat() {
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState, is_blocked};
    let definition = compile_to_runtime_definition("Ydwen Efreet",
        "Mana cost: {R}{R}{R}\nType: Creature — Efreet\nPower/Toughness: 3/6\nWhenever this creature blocks, flip a coin. If you lose the flip, remove this creature from combat and it can't block this turn. Creatures it was blocking that had become blocked by only this creature this combat become unblocked.", false)
        .expect("Ydwen Efreet should compile");
    fn find_unblock(effect: &Effect, found: &mut Option<Effect>) {
        if effect.downcast_ref::<ironsmith::effects::UnblockSoleBlockedAttackersEffect>().is_some() {
            *found = Some(effect.clone());
        }
        effect.visit_child_effects(&mut |child| find_unblock(child, found));
    }
    let mut unblock = None;
    for ability in &definition.abilities {
        if let ironsmith::ability::AbilityKind::Triggered(triggered) = &ability.kind {
            for effect in triggered.effects.all_effects() { find_unblock(effect, &mut unblock); }
        }
    }
    let unblock = unblock.expect("the unblock instruction should compile");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let ydwen = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let partner = p21_creature(&mut game, "Second blocker", alice);
    let alone = p21_creature(&mut game, "Blocked only by Ydwen", bob);
    let shared = p21_creature(&mut game, "Blocked by both", bob);
    let mut combat = CombatState::default();
    combat.attackers.push(AttackerInfo { creature: alone, target: AttackTarget::Player(alice) });
    combat.attackers.push(AttackerInfo { creature: shared, target: AttackTarget::Player(alice) });
    combat.blockers.insert(alone, vec![ydwen]);
    combat.blockers.insert(shared, vec![ydwen, partner]);
    combat.block_declaration_complete = true;
    game.combat = Some(combat);
    for (blocker, attacker) in [(ydwen, alone), (ydwen, shared), (partner, shared)] {
        let event = ironsmith::triggers::TriggerEvent::new_with_provenance(
            ironsmith::events::CreatureBlockedEvent::new(blocker, attacker), ironsmith::ProvNodeId::default());
        game.turn_store.turn_history.record_event(&event, None, None);
    }
    let mut ctx = ironsmith::effects::EffectContext::new_default(ydwen, alice);
    ironsmith::effects::execute_effect(&mut game,
        &Effect::new(ironsmith::effects::RemoveFromCombatEffect::with_spec(ChooseSpec::Source)), &mut ctx).unwrap();
    ironsmith::effects::execute_effect(&mut game, &unblock, &mut ctx).unwrap();
    drop(ctx);
    let combat = game.combat.as_ref().unwrap();
    assert!(!is_blocked(combat, alone), "its sole blocker left, so it becomes unblocked");
    assert!(is_blocked(combat, shared), "another creature also blocked it");
}

#[test]
fn tyvar_grants_a_trigger_on_mana_its_creatures_mana_abilities_produce() {
    let definition = compile_to_runtime_definition("Tyvar the Bellicose",
        "Mana cost: {2}{B}{G}\nType: Legendary Creature — Elf Warrior\nPower/Toughness: 5/4\nWhenever one or more Elves you control attack, they gain deathtouch until end of turn.\nEach creature you control has \"Whenever a mana ability of this creature resolves, put a number of +1/+1 counters on it equal to the amount of mana this creature produced. This ability triggers only once each turn.\"", false)
        .expect("Tyvar should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("SourceManaAbilityResolves"), "{debug}");
    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let _tyvar = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let elf = p21_creature(&mut game, "Mana creature", alice);
    let other = p21_creature(&mut game, "Other creature", alice);
    game.refresh_continuous_state().unwrap();
    use ironsmith::events::ManaAddedEvent;
    use ironsmith::events::mana::ManaProductionProvenance;
    use ironsmith::mana::ManaSymbol;
    let mana_event = |source, provenance| ironsmith::triggers::TriggerEvent::new_with_provenance(
        ManaAddedEvent::new(source, alice, alice, vec![ManaSymbol::Green, ManaSymbol::Green])
            .with_production_provenance(provenance),
        ironsmith::ProvNodeId::default());
    for provenance in [ManaProductionProvenance::TappedSourceForMana, ManaProductionProvenance::ManaAbility] {
        let entries = ironsmith::triggers::check_triggers(&game, &mana_event(elf, provenance));
        let entry = entries.iter().find(|entry| entry.source == elf)
            .unwrap_or_else(|| panic!("{provenance:?} should trigger the granted ability"));
        assert_eq!(entry.event_value_amount, Some(2));
        assert!(entries.iter().all(|entry| entry.source != other));
    }
    let entries = ironsmith::triggers::check_triggers(&game, &mana_event(elf, ManaProductionProvenance::Unknown));
    assert!(entries.iter().all(|entry| entry.source != elf), "mana not from a mana ability doesn't trigger");
}

#[test]
fn verdant_mastery_partitions_the_searched_lands_with_a_gated_opponent_part() {
    let definition = compile_to_runtime_definition("Verdant Mastery",
        "Mana cost: {5}{G}\nType: Sorcery\nYou may pay {3}{G} rather than pay this spell's mana cost.\nSearch your library for up to four basic land cards and reveal them. Put one of them onto the battlefield tapped under an opponent's control if the {3}{G} cost was paid. Put two of them onto the battlefield tapped under your control and the rest into your hand. Then shuffle.", false)
        .expect("Verdant Mastery should compile");
    assert!(!definition.alternative_casts.is_empty());
    let effects = definition.spell_effect.as_ref().unwrap().all_effects();
    let debug = format!("{effects:?}");
    assert!(debug.contains("PutOntoBattlefieldEffect"), "{debug}");
    assert!(debug.contains("Opponent"), "{debug}");
    assert!(debug.matches("ChooseTaggedObjectsInZone").count() >= 2
        || debug.matches("ChooseObjects").count() >= 3, "{debug}");
    assert!(debug.contains("Shuffle"), "{debug}");
}

#[test]
fn tranquil_frillback_bounds_its_payments_and_chooses_up_to_that_many_modes() {
    let definition = compile_to_runtime_definition("Tranquil Frillback",
        "Mana cost: {2}{G}\nType: Creature — Dinosaur\nPower/Toughness: 3/3\nWhen this creature enters, you may pay {G} up to three times. When you pay this cost one or more times, choose up to that many —\n• Destroy target artifact or enchantment.\n• Exile target player's graveyard.\n• You gain 4 life.", false)
        .expect("Tranquil Frillback should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("RepeatProcessEffect"), "{debug}");
    assert!(debug.contains("max_iterations: Some(3)"), "{debug}");
    assert!(debug.contains("ReflexiveTriggerEffect"), "{debug}");
    assert!(debug.contains("EffectValue(EffectId(1000000000))"), "the mode count is the payment count: {debug}");
}

#[test]
fn bounded_repeat_process_stops_at_its_maximum() {
    use ironsmith::effect::{EffectId, EffectPredicate};
    struct AlwaysYes;
    impl ironsmith::decision::DecisionMaker for AlwaysYes {
        fn decide_boolean(&mut self, _game: &ironsmith::GameState,
            _ctx: &ironsmith::decisions::context::BooleanContext) -> bool { true }
    }
    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = p21_creature(&mut game, "Repeater", alice);
    let condition = EffectId(41);
    let repeated = Effect::with_id(condition.0, Effect::may(vec![Effect::gain_life(1)]));
    let process = Effect::new(ironsmith::effects::RepeatProcessEffect::new(
        vec![repeated], condition, EffectPredicate::Happened).with_max_iterations(Some(3)));
    let mut dm = AlwaysYes;
    let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm);
    let outcome = ironsmith::effects::execute_effect(&mut game, &process, &mut ctx).unwrap();
    drop(ctx);
    assert_eq!(game.player(alice).unwrap().life, 23, "three accepted rounds, then it stops");
    assert_eq!(outcome.value.as_count(), Some(3));
}

#[test]
fn uvilda_attaches_its_cast_this_way_reduction_to_the_permitted_cast() {
    let definition = compile_to_runtime_definition("Uvilda, Dean of Perfection",
        "Mana cost: {2}{U}\nType: Legendary Creature — Djinn Wizard\nPower/Toughness: 2/2\n{T}: You may exile an instant or sorcery card from your hand and put three refine counters on it. It gains \"At the beginning of your upkeep, if this card is exiled, remove a refine counter from it\" and \"When the last refine counter is removed from this card, if it's exiled, you may cast it. It costs {4} less to cast this way.\"", false)
        .expect("Uvilda's front face should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("CastTagged"), "{debug}");
    assert!(debug.contains("cost_reduction: Some"), "the {{4}} reduction belongs to the granted cast: {debug}");
}

#[test]
fn vault_112_energy_payment_gates_shuffle_and_exiles_the_paid_amount() {
    let definition = compile_to_runtime_definition("Vault 112: Sadistic Simulation",
        "Mana cost: {3}{B}\nType: Enchantment — Saga\nI, II — Tap up to one target creature and put a stun counter on it. You get {E}{E}.\nIII — Pay any amount of {E}. If you paid one or more {E} this way, shuffle your library, then exile that many cards from the top. You may play one of those cards without paying its mana cost.", false)
        .expect("Vault 112 should compile strictly");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("PayAnyEnergyEffect") && debug.contains("min_amount: 0"), "{debug}");
    assert!(debug.contains("GreaterThan(0)"), "zero energy must not shuffle or exile: {debug}");
    assert!(debug.contains("ShuffleLibraryEffect"), "{debug}");
    assert!(debug.contains("ExileTopOfLibraryEffect"), "{debug}");
    let rendered = ironsmith_text::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(rendered.contains("paid one or more {E} this way"), "{rendered}");
    assert!(rendered.contains("without paying"), "{rendered}");
}

#[test]
fn energy_payment_shuffle_sequence_retains_payment_reference_without_a_comma() {
    for connective in [", then", " then"] {
        let source = format!("Mana cost: {{1}}\nType: Sorcery\nPay any amount of {{E}}. If you paid one or more {{E}} this way, shuffle your library{connective} exile that many cards from the top. You may play one of those cards without paying its mana cost.");
        let definition = compile_to_runtime_definition("Energy sequence probe", &source, false)
            .expect("shuffle then exile should compile");
        let effects = definition.spell_effect.as_ref().unwrap().all_effects();
        fn collect<'a>(effects: &'a [Effect], out: &mut Vec<&'a Effect>) {
            for effect in effects {
                out.push(effect);
                if let Some(with_id) = effect.downcast_ref::<ironsmith::effects::WithIdEffect>() {
                    collect(std::slice::from_ref(&with_id.effect), out);
                } else if let Some(sequence) = effect.downcast_ref::<ironsmith::effects::SequenceEffect>() {
                    collect(&sequence.effects, out);
                } else if let Some(branch) = effect.downcast_ref::<ironsmith::effects::IfEffect>() {
                    collect(&branch.then, out);
                } else if let Some(each) = effect.downcast_ref::<ironsmith::effects::ForEachTaggedEffect>() {
                    collect(&each.effects, out);
                } else if let Some(may) = effect.downcast_ref::<ironsmith::effects::MayEffect>() {
                    collect(&may.effects, out);
                } else if let Some(tag) = effect.downcast_ref::<ironsmith::effects::TaggedEffect>() {
                    collect(std::slice::from_ref(&tag.effect), out);
                } else if let Some(tag) = effect.downcast_ref::<ironsmith::effects::TagAllEffect>() {
                    collect(std::slice::from_ref(&tag.effect), out);
                }
            }
        }
        let mut all = Vec::new();
        for effect in effects {
            collect(std::slice::from_ref(effect), &mut all);
        }
        let payment = all.iter().find_map(|effect| {
            let with_id = effect.downcast_ref::<ironsmith::effects::WithIdEffect>()?;
            with_id.effect.downcast_ref::<ironsmith::effects::PayAnyEnergyEffect>()
                .map(|_| with_id.id)
        }).expect("identified energy payment");
        let branch = all.iter().find_map(|effect| effect.downcast_ref::<ironsmith::effects::IfEffect>())
            .expect("numeric payment gate");
        assert_eq!(branch.condition, payment);
        assert_eq!(branch.predicate, ironsmith::effect::EffectPredicate::Value(ironsmith::effect::Comparison::GreaterThan(0)));
        let exile = all.iter().find_map(|effect| effect.downcast_ref::<ironsmith::effects::ExileTopOfLibraryEffect>())
            .expect("top-library exile");
        assert_eq!(exile.count.unhinted(), &ironsmith::effect::Value::EffectValue(payment),
            "shuffle must not replace the payment as the amount antecedent");
        let cast = all.iter().find_map(|effect| effect.downcast_ref::<ironsmith::effects::CastTaggedEffect>())
            .expect("immediate play of one exiled card");
        assert!(cast.allow_land, "play includes lands");
        assert!(cast.without_paying_mana_cost);
        let choose = all.iter().find_map(|effect| effect.downcast_ref::<ironsmith::effects::ChooseObjectsEffect>())
            .expect("one optional card choice");
        assert_eq!(choose.count.min, 0, "playing the exiled card is optional");
        assert_eq!(choose.count.max, Some(1), "at most one card may be played");
        assert!(!choose.filter.source, "choice must not select the source");
        assert!(choose.filter.tagged_constraints.iter().any(|constraint| exile.moved_tags.contains(&constraint.tag)),
            "choice must reference the actual exiled set");
        let each = all.iter().find_map(|effect| effect.downcast_ref::<ironsmith::effects::ForEachTaggedEffect>())
            .expect("play the selected card only");
        assert_eq!(each.tag, choose.tag);
    }
}
