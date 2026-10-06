//! Source-relative host references survive Aura/Equipment copy changes.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
use ironsmith::events::combat::CreatureAttackedEvent;
use ironsmith::game_loop::{
    extract_target_requirements_from_program_with_modes, put_triggers_on_stack_with_dm,
    resolve_stack_entry_with,
};
use ironsmith::game_state::{StackEntry, TargetAssignment};
use ironsmith::object::{AttachmentTarget, CounterType};
use ironsmith::provenance::ProvNodeId;
use ironsmith::snapshot::ObjectSnapshot;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::target::{FilterContext, ObjectFilter, TaggedOpbjectRelation};
use ironsmith::triggers::{
    AttackEventTarget, AttacksTrigger, TriggerContext, TriggerEvent, TriggerMatcher, TriggerQueue,
    check_triggers,
};
use ironsmith::{AbilityKind, CardType, GameState, ObjectId, PlayerId, Subtype, Target, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

fn definition(name: &str, text: &str) -> CardDefinition {
    compile_to_runtime_definition(name, text, false).unwrap()
}
fn bears() -> CardDefinition {
    definition(
        "Grizzly Bears",
        "Mana cost: {1}{G}\nType: Creature — Bear\nPower/Toughness: 2/2",
    )
}
fn umbra() -> CardDefinition {
    definition(
        "Hyena Umbra",
        "Mana cost: {W}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature gets +1/+1 and has first strike.\nUmbra armor",
    )
}
fn fireshrieker() -> CardDefinition {
    definition(
        "Fireshrieker",
        "Mana cost: {3}\nType: Artifact — Equipment\nEquipped creature has double strike.\nEquip {2}",
    )
}
fn sword() -> CardDefinition {
    definition(
        "Sword of the Animist",
        "Mana cost: {2}\nType: Legendary Artifact — Equipment\nEquipped creature gets +1/+1.\nWhenever equipped creature attacks, you may search your library for a basic land card, put it onto the battlefield tapped, then shuffle.\nEquip {2}",
    )
}
fn ordeal() -> CardDefinition {
    definition(
        "Ordeal of Heliod",
        "Mana cost: {1}{W}\nType: Enchantment — Aura\nEnchant creature\nWhenever enchanted creature attacks, put a +1/+1 counter on it. Then if it has three or more +1/+1 counters on it, sacrifice this Aura.\nWhen you sacrifice this Aura, you gain 10 life.",
    )
}
fn game() -> GameState {
    GameState::new(vec!["Alice".into(), "Bob".into()], 20)
}
fn make_artifact(game: &mut GameState, target: ObjectId) {
    let coating = definition(
        "Liquimetal Coating",
        "Mana cost: {2}\nType: Artifact\n{T}: Target permanent becomes an artifact in addition to its other types until end of turn.",
    );
    let source = game.create_object_from_definition(&coating, A, Zone::Battlefield);
    let AbilityKind::Activated(ability) = &coating.abilities[0].kind else {
        panic!("activated Coating")
    };
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx =
        EffectContext::new(source, A, &mut dm).with_targets(vec![ResolvedTarget::Object(target)]);
    for effect in ability.effects.all_effects() {
        execute_effect(game, effect, &mut ctx).unwrap();
    }
    assert!(game.object_has_card_type(target, CardType::Artifact));
}
fn polymorph(game: &mut GameState, guest: ObjectId, copied: ObjectId) {
    let spell = definition(
        "True Polymorph",
        "Mana cost: {4}{U}{U}\nType: Instant\nTarget artifact or creature becomes a copy of another target artifact or creature.",
    );
    let source = game.create_object_from_definition(&spell, A, Zone::Stack);
    let requirements = extract_target_requirements_from_program_with_modes(
        game,
        spell.spell_effect.as_ref().unwrap(),
        A,
        Some(source),
        None,
    );
    assert_eq!(requirements.len(), 2);
    let assignments = requirements
        .iter()
        .enumerate()
        .map(|(index, requirement)| TargetAssignment {
            spec: requirement.spec.clone(),
            range: index..index + 1,
        })
        .collect();
    game.push_to_stack(
        StackEntry::new(source, A)
            .with_targets(vec![Target::Object(guest), Target::Object(copied)])
            .with_target_assignments(assignments),
    );
    resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap();
}
fn assert_host_aliases(game: &GameState, guest: ObjectId, host: ObjectId) {
    let context = game.filter_context_for(A, Some(guest));
    for tag in ["enchanted", "equipped"] {
        assert_eq!(context.tagged_objects[tag][0].object_id, host, "{tag}");
    }
    assert_eq!(
        game.object(guest).unwrap().attached_to,
        Some(AttachmentTarget::Object(host))
    );
    assert!(ironsmith::rules::check_state_based_actions(game).is_empty());
}
fn attack_event(host: ObjectId) -> TriggerEvent {
    TriggerEvent::new_with_provenance(
        CreatureAttackedEvent::new(host, AttackEventTarget::Player(B)),
        ProvNodeId::default(),
    )
}
fn resolve_host_attack(game: &mut GameState, guest: ObjectId, host: ObjectId) {
    let triggered = check_triggers(game, &attack_event(host));
    assert_eq!(
        triggered
            .iter()
            .filter(|entry| entry.source == guest)
            .count(),
        1
    );
    let mut queue = TriggerQueue::new();
    for entry in triggered {
        queue.add(entry);
    }
    let mut dm = SelectFirstDecisionMaker;
    put_triggers_on_stack_with_dm(game, &mut queue, &mut dm).unwrap();
    assert_eq!(game.stack.len(), 1);
    resolve_stack_entry_with(game, &mut dm).unwrap();
}

#[test]
fn aura_copied_into_fireshrieker_keeps_host_and_grants_double_strike() {
    let mut game = game();
    let host = game.create_object_from_definition(&bears(), A, Zone::Battlefield);
    let guest = game.create_object_from_definition(&umbra(), A, Zone::Battlefield);
    let copied = game.create_object_from_definition(&fireshrieker(), B, Zone::Battlefield);
    game.attach_object_to_target(guest, AttachmentTarget::Object(host));
    assert_eq!(game.current_power(host), Some(3));
    make_artifact(&mut game, guest);
    polymorph(&mut game, guest, copied);
    assert_host_aliases(&game, guest, host);
    assert_eq!(game.current_power(host), Some(2));
    assert!(game.object_has_static_ability_id(host, StaticAbilityId::DoubleStrike));
    assert!(!game.object_has_static_ability_id(host, StaticAbilityId::FirstStrike));
}

#[test]
fn aura_copied_into_sword_of_the_animist_triggers_and_searches_on_host_attack() {
    let mut game = game();
    let host = game.create_object_from_definition(&bears(), A, Zone::Battlefield);
    let guest = game.create_object_from_definition(&umbra(), A, Zone::Battlefield);
    // The opponent's original avoids a same-controller legend-rule conflict.
    let copied = game.create_object_from_definition(&sword(), B, Zone::Battlefield);
    let forest = definition("Forest", "Type: Basic Land — Forest");
    let land = game.create_object_from_definition(&forest, A, Zone::Library);
    let land_stable_id = game.object(land).unwrap().stable_id;
    game.attach_object_to_target(guest, AttachmentTarget::Object(host));
    make_artifact(&mut game, guest);
    polymorph(&mut game, guest, copied);
    assert_host_aliases(&game, guest, host);
    assert_eq!(game.current_power(host), Some(3));
    // An unrelated creature's attack must not trigger the copied Equipment.
    let other = game.create_object_from_definition(&bears(), A, Zone::Battlefield);
    assert!(check_triggers(&game, &attack_event(other)).is_empty());
    resolve_host_attack(&mut game, guest, host);
    let land = game
        .battlefield
        .iter()
        .filter_map(|id| game.object(*id))
        .find(|object| object.stable_id == land_stable_id)
        .expect("searched Forest on battlefield");
    assert!(game.is_tapped(land.id));
}

#[test]
fn equipment_copied_into_ordeal_of_heliod_triggers_and_puts_counter_on_host() {
    let mut game = game();
    let host = game.create_object_from_definition(&bears(), A, Zone::Battlefield);
    let guest = game.create_object_from_definition(&fireshrieker(), A, Zone::Battlefield);
    let copied = game.create_object_from_definition(&ordeal(), B, Zone::Battlefield);
    let copied_host = game.create_object_from_definition(&bears(), B, Zone::Battlefield);
    game.attach_object_to_target(copied, AttachmentTarget::Object(copied_host));
    game.attach_object_to_target(guest, AttachmentTarget::Object(host));
    make_artifact(&mut game, copied);
    polymorph(&mut game, guest, copied);
    assert_host_aliases(&game, guest, host);
    assert!(!game.object_has_static_ability_id(host, StaticAbilityId::DoubleStrike));
    resolve_host_attack(&mut game, guest, host);
    assert_eq!(
        game.object(host)
            .unwrap()
            .counters
            .get(&CounterType::PlusOnePlusOne),
        Some(&1)
    );
}

#[test]
fn unbound_and_historical_host_aliases_ignore_guest_subtype_but_keep_host_identity() {
    for guest_definition in [umbra(), fireshrieker()] {
        let mut game = game();
        let host = game.create_object_from_definition(&bears(), A, Zone::Battlefield);
        let other = game.create_object_from_definition(&bears(), A, Zone::Battlefield);
        let guest = game.create_object_from_definition(&guest_definition, A, Zone::Battlefield);
        game.attach_object_to_target(guest, AttachmentTarget::Object(host));
        let source_snapshot = ObjectSnapshot::from_object(game.object(guest).unwrap(), &game);
        for historical in [false, true] {
            if historical {
                game.remove_object(guest);
            }
            let mut context = FilterContext::new(A).with_source(guest);
            if historical {
                context.source_snapshot = Some(source_snapshot.clone());
            }
            let ctx = TriggerContext::new(guest, A, context, &game);
            for tag in ["enchanted", "equipped"] {
                let positive = AttacksTrigger::new(
                    ObjectFilter::creature()
                        .match_tagged(tag, TaggedOpbjectRelation::IsTaggedObject),
                );
                let negative = AttacksTrigger::new(
                    ObjectFilter::creature()
                        .match_tagged(tag, TaggedOpbjectRelation::IsNotTaggedObject),
                );
                assert!(
                    positive.matches(&attack_event(host), &ctx),
                    "{tag}, historical={historical}"
                );
                assert!(!positive.matches(&attack_event(other), &ctx));
                assert!(!negative.matches(&attack_event(host), &ctx));
                assert!(negative.matches(&attack_event(other), &ctx));
            }
        }
    }
}

#[test]
fn intrinsic_attachment_predicates_still_distinguish_auras_from_equipment() {
    let mut game = game();
    let aura_host = game.create_object_from_definition(&bears(), A, Zone::Battlefield);
    let equipment_host = game.create_object_from_definition(&bears(), A, Zone::Battlefield);
    let aura = game.create_object_from_definition(&umbra(), A, Zone::Battlefield);
    let equipment = game.create_object_from_definition(&fireshrieker(), A, Zone::Battlefield);
    game.attach_object_to_target(aura, AttachmentTarget::Object(aura_host));
    game.attach_object_to_target(equipment, AttachmentTarget::Object(equipment_host));
    let ctx = TriggerContext::new(aura, A, game.filter_context_for(A, Some(aura)), &game);
    for (subtype, expected_host) in [
        (Subtype::Aura, aura_host),
        (Subtype::Equipment, equipment_host),
    ] {
        let mut filter = ObjectFilter::creature();
        filter.with_attached_object = Some(Box::new(ObjectFilter::default().with_subtype(subtype)));
        let predicate = AttacksTrigger::new(filter);
        assert!(predicate.matches(&attack_event(expected_host), &ctx));
        let other = if expected_host == aura_host {
            equipment_host
        } else {
            aura_host
        };
        assert!(!predicate.matches(&attack_event(other), &ctx));
    }
}

#[test]
fn lookback_death_triggers_bind_both_host_aliases_after_attachment_is_cleared() {
    for (type_line, tag) in [
        ("Enchantment — Aura", "equipped"),
        ("Artifact — Equipment", "enchanted"),
    ] {
        let mut game = game();
        let host = game.create_object_from_definition(&bears(), A, Zone::Battlefield);
        // The reference's spelling need not agree with the guest's current
        // subtype. A type/copy change can leave this exact combination behind.
        let observer = definition(
            "Attachment observer",
            &format!("Type: {type_line}\nWhen {tag} creature dies, draw a card."),
        );
        let guest = game.create_object_from_definition(&observer, A, Zone::Battlefield);
        game.attach_object_to_target(guest, AttachmentTarget::Object(host));
        let host_snapshot = ObjectSnapshot::from_object(game.object(host).unwrap(), &game);
        let guest_snapshot = ObjectSnapshot::from_object(game.object(guest).unwrap(), &game);
        let mut death = ironsmith::events::zones::ZoneChangeEvent::with_cause(
            host,
            Zone::Battlefield,
            Zone::Graveyard,
            ironsmith::events::EventCause::effect(),
            Some(host_snapshot),
        );
        death
            .object_tags
            .insert("attached_source".into(), vec![guest_snapshot.clone()]);
        game.remove_object(host);
        game.remove_object(guest);
        let event = TriggerEvent::new_with_provenance(death, ProvNodeId::default())
            .with_lookback_source_snapshots(vec![guest_snapshot]);
        let triggered = check_triggers(&game, &event);
        assert_eq!(
            triggered
                .iter()
                .filter(|entry| entry.source == guest)
                .count(),
            1,
            "{tag}"
        );
    }
}

#[test]
fn host_aliases_follow_reattachment_and_stop_matching_when_detached() {
    let mut game = game();
    let former = game.create_object_from_definition(&bears(), A, Zone::Battlefield);
    let current = game.create_object_from_definition(&bears(), A, Zone::Battlefield);
    let guest = game.create_object_from_definition(&umbra(), A, Zone::Battlefield);
    game.attach_object_to_target(guest, AttachmentTarget::Object(former));
    game.attach_object_to_target(guest, AttachmentTarget::Object(current));
    for attached in [true, false] {
        if !attached {
            game.detach_object_from_current_target(guest);
        }
        let ctx = TriggerContext::new(guest, A, game.filter_context_for(A, Some(guest)), &game);
        for tag in ["enchanted", "equipped"] {
            let trigger = AttacksTrigger::new(
                ObjectFilter::creature().match_tagged(tag, TaggedOpbjectRelation::IsTaggedObject),
            );
            assert_eq!(trigger.matches(&attack_event(current), &ctx), attached);
            assert!(!trigger.matches(&attack_event(former), &ctx));
        }
    }
}
