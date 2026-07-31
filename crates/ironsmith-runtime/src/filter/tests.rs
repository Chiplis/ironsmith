use super::*;

#[test]
fn test_comparison() {
    assert!(Comparison::Equal(5).satisfies(5));
    assert!(!Comparison::Equal(5).satisfies(4));

    assert!(Comparison::LessThanOrEqual(2).satisfies(2));
    assert!(Comparison::LessThanOrEqual(2).satisfies(1));
    assert!(!Comparison::LessThanOrEqual(2).satisfies(3));

    assert!(Comparison::GreaterThan(3).satisfies(4));
    assert!(!Comparison::GreaterThan(3).satisfies(3));
}

#[test]
fn test_creature_filter() {
    let filter = ObjectFilter::creature();
    assert_eq!(filter.zone, Some(Zone::Battlefield));
    assert_eq!(filter.card_types, vec![CardType::Creature]);
}

#[test]
fn branch_scoped_type_union_applies_shared_controller_and_mana_value_once() {
    use crate::card::CardBuilder;
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};

    fn permanent(
        id: u32,
        name: &str,
        card_type: CardType,
        subtype: Option<Subtype>,
        mana_value: u8,
    ) -> crate::card::Card {
        let mut builder = CardBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![card_type])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(
                mana_value,
            )]));
        if let Some(subtype) = subtype {
            builder = builder.subtypes(vec![subtype]);
        }
        builder.build()
    }

    let you = PlayerId::from_index(0);
    let opponent = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["You".to_string(), "Opponent".to_string()], 20);
    let artifact = game.create_object_from_card(
        &permanent(47_260, "Relic", CardType::Artifact, None, 4),
        you,
        Zone::Battlefield,
    );
    let equipment = game.create_object_from_card(
        &permanent(
            47_261,
            "Equipment",
            CardType::Artifact,
            Some(Subtype::Equipment),
            4,
        ),
        you,
        Zone::Battlefield,
    );
    let enchantment = game.create_object_from_card(
        &permanent(47_262, "Blessing", CardType::Enchantment, None, 4),
        you,
        Zone::Battlefield,
    );
    let aura = game.create_object_from_card(
        &permanent(
            47_263,
            "Aura",
            CardType::Enchantment,
            Some(Subtype::Aura),
            4,
        ),
        you,
        Zone::Battlefield,
    );
    let cheap_artifact = game.create_object_from_card(
        &permanent(47_264, "Cheap Relic", CardType::Artifact, None, 3),
        you,
        Zone::Battlefield,
    );
    let opposing_artifact = game.create_object_from_card(
        &permanent(47_265, "Opposing Relic", CardType::Artifact, None, 4),
        opponent,
        Zone::Battlefield,
    );

    let mut filter = ObjectFilter::default()
        .in_zone(Zone::Battlefield)
        .controlled_by(PlayerFilter::You)
        .with_mana_value(Comparison::GreaterThanOrEqual(4));
    filter.any_of = vec![
        ObjectFilter::default()
            .with_type(CardType::Artifact)
            .without_subtype(Subtype::Equipment),
        ObjectFilter::default()
            .with_type(CardType::Enchantment)
            .without_subtype(Subtype::Aura),
    ];
    filter.set_conjunctive_set_surface(true);
    let context = FilterContext::new(you);

    assert_eq!(
        filter.description(),
        "a non-equipment artifact and non-aura enchantment you control with mana value 4 or greater"
    );
    assert!(filter.matches(game.object(artifact).unwrap(), &context, &game));
    assert!(filter.matches(game.object(enchantment).unwrap(), &context, &game));
    assert!(!filter.matches(game.object(equipment).unwrap(), &context, &game));
    assert!(!filter.matches(game.object(aura).unwrap(), &context, &game));
    assert!(!filter.matches(game.object(cheap_artifact).unwrap(), &context, &game));
    assert!(!filter.matches(game.object(opposing_artifact).unwrap(), &context, &game));
}

#[test]
fn compound_subtype_filter_requires_every_authored_subtype() {
    let you = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["You".to_string()], 20);
    let both = crate::card::CardBuilder::new(crate::ids::CardId::from_raw(47_250), "Both")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Eldrazi, Subtype::Spawn])
        .build();
    let eldrazi_only =
        crate::card::CardBuilder::new(crate::ids::CardId::from_raw(47_251), "Eldrazi")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Eldrazi])
            .build();
    let both = game.create_object_from_card(&both, you, Zone::Battlefield);
    let eldrazi_only = game.create_object_from_card(&eldrazi_only, you, Zone::Battlefield);
    let context = FilterContext::new(you);
    let filter = ObjectFilter::creature()
        .with_all_subtype(Subtype::Eldrazi)
        .with_all_subtype(Subtype::Spawn);

    assert_eq!(filter.description(), "Eldrazi Spawn creature");
    assert!(filter.matches(game.object(both).unwrap(), &context, &game));
    assert!(
        !filter.matches(game.object(eldrazi_only).unwrap(), &context, &game),
        "a compound subtype phrase must not behave like an inclusive subtype list"
    );
}

#[test]
fn your_team_controller_filter_preserves_team_surface_and_membership() {
    let you = PlayerId::from_index(0);
    let teammate = PlayerId::from_index(1);
    let opponent = PlayerId::from_index(2);
    let mut context = FilterContext::new(you);
    context.teammates = vec![teammate];
    context.opponents = vec![opponent];

    let your_team = PlayerFilter::your_team();
    assert!(your_team.matches_player(you, &context));
    assert!(your_team.matches_player(teammate, &context));
    assert!(!your_team.matches_player(opponent, &context));

    let warriors = ObjectFilter::default()
        .with_subtype(Subtype::Warrior)
        .controlled_by(your_team);
    assert_eq!(warriors.description(), "a Warrior your team controls");
}

#[test]
fn negative_attack_or_entry_history_filter_checks_both_turn_records() {
    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let creature_card =
        crate::card::CardBuilder::new(crate::ids::CardId::from_raw(47_190), "History Creature")
            .card_types(vec![CardType::Creature])
            .build();
    let eligible = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);
    let entered = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);
    let attacked = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);

    let entered_snapshot =
        ObjectSnapshot::from_object(game.object(entered).expect("entered creature"), &game);
    let entry_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::with_cause(
            entered,
            Zone::Hand,
            Zone::Battlefield,
            crate::events::EventCause::effect(),
            Some(entered_snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    game.record_turn_history_event(&entry_event);
    game.mark_creature_attacked_this_turn(attacked);

    let filter = ObjectFilter {
        zone: Some(Zone::Battlefield),
        controller: Some(PlayerFilter::You),
        card_types: vec![CardType::Creature],
        didnt_attack_this_turn: true,
        didnt_enter_battlefield_this_turn: true,
        ..ObjectFilter::default()
    };
    let context = FilterContext::new(alice);

    assert!(filter.matches(game.object(eligible).unwrap(), &context, &game));
    assert!(!filter.matches(game.object(entered).unwrap(), &context, &game));
    assert!(!filter.matches(game.object(attacked).unwrap(), &context, &game));
}

#[test]
fn last_known_attachment_relation_ignores_later_current_attachment_state() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let bob = PlayerId::from_index(1);
    let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Attachment Host")
        .card_types(vec![CardType::Creature])
        .build();
    let equipment =
        crate::card::CardBuilder::new(crate::ids::CardId::new(), "Provenance Equipment")
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Equipment])
            .build();
    let former_host = game.create_object_from_card(&creature, bob, Zone::Battlefield);
    let later_host = game.create_object_from_card(&creature, bob, Zone::Battlefield);
    let former_attachment = game.create_object_from_card(&equipment, bob, Zone::Battlefield);
    let later_attachment = game.create_object_from_card(&equipment, bob, Zone::Battlefield);

    assert!(
        crate::effects::permanents::attach_battlefield_object_to_target(
            &mut game,
            former_attachment,
            crate::object::AttachmentTarget::Object(former_host),
        )
    );
    let former_host_snapshot =
        ObjectSnapshot::from_object(game.object(former_host).expect("former host exists"), &game);
    assert!(
        crate::effects::permanents::attach_battlefield_object_to_target(
            &mut game,
            former_attachment,
            crate::object::AttachmentTarget::Object(later_host),
        )
    );
    assert!(
        crate::effects::permanents::attach_battlefield_object_to_target(
            &mut game,
            later_attachment,
            crate::object::AttachmentTarget::Object(former_host),
        )
    );

    let tag = TagKey::from("former_host");
    let context =
        FilterContext::new(bob).with_tagged_objects(&std::collections::HashMap::from([(
            tag.clone(),
            vec![former_host_snapshot],
        )]));
    let filter = ObjectFilter::default()
        .with_subtype(Subtype::Equipment)
        .in_zone(Zone::Battlefield)
        .match_tagged(tag, TaggedOpbjectRelation::WasAttachedToTaggedObject);

    assert!(
        filter.matches(
            game.object(former_attachment)
                .expect("former attachment exists"),
            &context,
            &game,
        ),
        "the snapshot's former attachment must match even after moving elsewhere"
    );
    assert!(
        !filter.matches(
            game.object(later_attachment)
                .expect("later attachment exists"),
            &context,
            &game,
        ),
        "a host's later current attachment must not enter the LKI attachment set"
    );
}

#[test]
fn original_printing_set_filter_matches_card_metadata_and_snapshots() {
    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let antiquities_card =
        crate::card::CardBuilder::new(crate::ids::CardId::from_raw(47_200), "Antiquities Relic")
            .first_printed_set_name("Antiquities")
            .card_types(vec![CardType::Artifact])
            .build();
    let unannotated_card =
        crate::card::CardBuilder::new(crate::ids::CardId::from_raw(47_201), "Unknown Relic")
            .card_types(vec![CardType::Artifact])
            .build();
    let antiquities_id = game.create_object_from_card(&antiquities_card, alice, Zone::Battlefield);
    let unannotated_id = game.create_object_from_card(&unannotated_card, alice, Zone::Battlefield);
    let context = FilterContext::new(alice);
    let filter = ObjectFilter {
        zone: Some(Zone::Battlefield),
        name_originally_printed_in_set: Some("antiquities".to_string()),
        ..ObjectFilter::default()
    };

    let antiquities_object = game
        .object(antiquities_id)
        .expect("annotated permanent should exist");
    assert!(filter.matches(antiquities_object, &context, &game));
    assert!(filter.matches_snapshot(
        &ObjectSnapshot::from_object(antiquities_object, &game),
        &context,
        &game
    ));
    assert!(
        !filter.matches(
            game.object(unannotated_id)
                .expect("unannotated permanent should exist"),
            &context,
            &game
        )
    );
}

#[test]
fn created_with_source_filter_matches_stable_creation_provenance() {
    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let source_card = crate::card::CardBuilder::new(crate::ids::CardId::from_raw(47_210), "Source")
        .card_types(vec![CardType::Enchantment])
        .build();
    let candidate_card =
        crate::card::CardBuilder::new(crate::ids::CardId::from_raw(47_211), "Candidate")
            .card_types(vec![CardType::Creature])
            .build();
    let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
    let linked = game.create_object_from_card(&candidate_card, alice, Zone::Battlefield);
    let unrelated = game.create_object_from_card(&candidate_card, alice, Zone::Battlefield);
    let source_stable = game.object(source).unwrap().stable_id;
    let linked_stable = game.object(linked).unwrap().stable_id;
    game.add_token_created_with_source_link(source_stable, linked_stable);

    let filter = ObjectFilter {
        created_with_source: true,
        ..ObjectFilter::default()
    };
    let context = FilterContext::new(alice).with_source(source);
    let linked_object = game.object(linked).unwrap();
    assert!(filter.matches(linked_object, &context, &game));
    assert!(filter.matches_snapshot(
        &ObjectSnapshot::from_object(linked_object, &game),
        &context,
        &game
    ));
    assert!(!filter.matches(game.object(unrelated).unwrap(), &context, &game));
}

#[test]
fn blocked_by_tagged_filter_matches_current_combat_relationship() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let attacker_card = crate::card::CardBuilder::new(crate::ids::CardId::from_raw(1), "Attacker")
        .card_types(vec![CardType::Creature])
        .build();
    let blocker_card = crate::card::CardBuilder::new(crate::ids::CardId::from_raw(2), "Blocker")
        .card_types(vec![CardType::Creature])
        .build();
    let attacker = Object::from_card(
        ObjectId::from_raw(1),
        &attacker_card,
        alice,
        Zone::Battlefield,
    );
    let blocker = Object::from_card(ObjectId::from_raw(2), &blocker_card, bob, Zone::Battlefield);
    game.add_object(attacker.clone());
    game.add_object(blocker.clone());
    game.combat = Some(crate::combat_state::CombatState {
        attackers: vec![crate::combat_state::AttackerInfo {
            creature: attacker.id,
            target: crate::combat_state::AttackTarget::Player(bob),
        }],
        blockers: std::collections::HashMap::from([(attacker.id, vec![blocker.id])]),
        damage_assignment_order: std::collections::HashMap::new(),
        attacking_bands: Vec::new(),
        had_to_attack_this_combat: Default::default(),
    });

    let blocker_snapshot =
        ObjectSnapshot::from_object_with_calculated_characteristics(&blocker, &game);
    let ctx = FilterContext::new(alice).with_tagged_objects(&std::collections::HashMap::from([(
        TagKey::from("chosen_blockers"),
        vec![blocker_snapshot],
    )]));
    let filter = ObjectFilter {
        zone: Some(Zone::Battlefield),
        card_types: vec![CardType::Creature],
        blocked_by: Some(ObjectRef::Tagged(TagKey::from("chosen_blockers"))),
        ..ObjectFilter::default()
    };

    assert!(filter.matches(game.object(attacker.id).unwrap(), &ctx, &game));
}

#[test]
fn historical_block_partner_filter_matches_both_roles_using_declaration_lki() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let creature_card =
        crate::card::CardBuilder::new(crate::ids::CardId::from_raw(47_230), "Candidate")
            .card_types(vec![CardType::Creature])
            .build();
    let zombie_card = crate::card::CardBuilder::new(crate::ids::CardId::from_raw(47_231), "Zombie")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie])
        .build();

    let candidate_attacker = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);
    let zombie_blocker = game.create_object_from_card(&zombie_card, bob, Zone::Battlefield);
    let zombie_attacker = game.create_object_from_card(&zombie_card, bob, Zone::Battlefield);
    let candidate_blocker = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);
    let unrelated = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);

    for (blocker, attacker) in [
        (zombie_blocker, candidate_attacker),
        (candidate_blocker, zombie_attacker),
    ] {
        let blocker_snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(blocker).expect("blocker"),
            &game,
        );
        let attacker_snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(attacker).expect("attacker"),
            &game,
        );
        let event = crate::triggers::TriggerEvent::new(
            crate::events::combat::CreatureBlockedEvent::with_snapshots(
                blocker,
                attacker,
                blocker_snapshot,
                attacker_snapshot,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        game.record_turn_history_event(&event);
    }

    // The partner permanents no longer exist under their declaration-time
    // object IDs. Matching must use the snapshots captured by the block
    // events rather than current battlefield characteristics.
    game.move_object_by_effect(zombie_blocker, Zone::Graveyard)
        .expect("move Zombie blocker");
    game.move_object_by_effect(zombie_attacker, Zone::Graveyard)
        .expect("move Zombie attacker");

    let mut filter = ObjectFilter::creature();
    filter.blocked_or_was_blocked_by_this_turn = Some(Box::new(
        ObjectFilter::creature().with_subtype(Subtype::Zombie),
    ));
    let ctx = FilterContext::new(alice);

    assert!(filter.matches(game.object(candidate_attacker).unwrap(), &ctx, &game));
    assert!(filter.matches(game.object(candidate_blocker).unwrap(), &ctx, &game));
    assert!(!filter.matches(game.object(unrelated).unwrap(), &ctx, &game));
}

#[test]
fn test_filter_chaining() {
    let filter = ObjectFilter::creature()
        .you_control()
        .other()
        .with_power(Comparison::GreaterThanOrEqual(3));

    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert!(filter.other);
    assert!(filter.power.is_some());
}

#[test]
fn test_nonland_filter() {
    let filter = ObjectFilter::nonland();
    assert!(filter.excluded_card_types.contains(&CardType::Land));
}

#[test]
fn test_filter_with_subtypes() {
    let filter = ObjectFilter::creature()
        .with_subtype(crate::types::Subtype::Elf)
        .with_subtype(crate::types::Subtype::Warrior);

    assert_eq!(filter.subtypes.len(), 2);
}

#[test]
fn test_adventure_subtype_filter_matches_front_face_linked_to_adventure() {
    use crate::card::{LinkedFaceLayout, PowerToughness};
    use crate::cards::CardDefinitionBuilder;
    use crate::ids::CardId;
    use crate::snapshot::ObjectSnapshot;
    use crate::zone::Zone;

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let front_id = CardId::from_raw(47_100);
    let adventure_id = CardId::from_raw(47_101);
    let front = CardDefinitionBuilder::new(front_id, "Linked Adventure Creature")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human])
        .power_toughness(PowerToughness::fixed(2, 2))
        .other_face(adventure_id)
        .other_face_name("Linked Adventure Spell")
        .linked_face_layout(LinkedFaceLayout::TransformLike)
        .build();
    let adventure = CardDefinitionBuilder::new(adventure_id, "Linked Adventure Spell")
        .card_types(vec![CardType::Sorcery])
        .subtypes(vec![Subtype::Adventure])
        .other_face(front_id)
        .other_face_name("Linked Adventure Creature")
        .linked_face_layout(LinkedFaceLayout::TransformLike)
        .build();
    game.register_linked_face_definition(&front);
    game.register_linked_face_definition(&adventure);

    let object_id = game.create_object_from_definition(&front, alice, Zone::Hand);
    let object = game
        .object(object_id)
        .expect("linked adventure creature should exist");
    let ctx = FilterContext::new(alice);
    let adventure_filter = ObjectFilter::default().with_subtype(Subtype::Adventure);

    assert!(adventure_filter.matches(object, &ctx, &game));

    let snapshot = ObjectSnapshot::from_object(object, &game);
    assert!(adventure_filter.matches_snapshot(&snapshot, &ctx, &game));
    assert!(
        !ObjectFilter::default()
            .without_subtype(Subtype::Adventure)
            .matches(object, &ctx, &game)
    );
}

#[test]
fn test_spell_zone_filter_matches_stack_spell_cast_from_graveyard() {
    use crate::alternative_cast::CastingMethod;
    use crate::card::CardBuilder;
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::zone::Zone;

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let spell = CardBuilder::new(CardId::from_raw(1), "Graveyard Cast Probe")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .build();
    let graveyard_id = game.create_object_from_card(&spell, alice, Zone::Graveyard);
    let stack_id = game
        .move_object_by_effect(graveyard_id, Zone::Stack)
        .expect("move probe spell to stack");
    game.push_to_stack(
        crate::game_state::StackEntry::new(stack_id, alice).with_casting_method(
            CastingMethod::PlayFrom {
                source: stack_id,
                zone: Zone::Graveyard,
                use_alternative: None,
            },
        ),
    );
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new(stack_id, alice, Zone::Graveyard),
        crate::provenance::ProvNodeId::default(),
    );
    game.stage_turn_history_event(&event);

    let filter = ObjectFilter::spell().in_zone(Zone::Graveyard);
    let ctx = FilterContext::new(alice);
    let object = game.object(stack_id).expect("stack spell should exist");
    assert!(
        filter.matches(object, &ctx, &game),
        "spell cast from graveyard should satisfy graveyard origin filter"
    );
}

#[test]
fn test_spell_zone_filter_matches_stack_spell_with_graveyard_alternative_cast() {
    use crate::alternative_cast::{AlternativeCastingMethod, CastingMethod};
    use crate::card::CardBuilder;
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::zone::Zone;

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let spell = CardBuilder::new(CardId::from_raw(2), "Flashback Probe")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .build();
    let graveyard_id = game.create_object_from_card(&spell, alice, Zone::Graveyard);
    let stack_id = game
        .move_object_by_effect(graveyard_id, Zone::Stack)
        .expect("move flashback probe to stack");
    game.object_mut(stack_id)
        .expect("stack spell should exist")
        .alternative_casts
        .push(AlternativeCastingMethod::Flashback {
            total_cost: crate::cost::TotalCost::mana(ManaCost::default()),
        });
    game.push_to_stack(
        crate::game_state::StackEntry::new(stack_id, alice)
            .with_casting_method(CastingMethod::Alternative(0)),
    );
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new(stack_id, alice, Zone::Graveyard),
        crate::provenance::ProvNodeId::default(),
    );
    game.stage_turn_history_event(&event);

    let filter = ObjectFilter::spell().in_zone(Zone::Graveyard);
    let ctx = FilterContext::new(alice);
    let object = game.object(stack_id).expect("stack spell should exist");
    assert!(
        filter.matches(object, &ctx, &game),
        "spell cast with a graveyard alternative method should satisfy graveyard origin filter"
    );
}

#[test]
fn excluded_cast_origin_zone_distinguishes_hand_and_nonhand_spells() {
    use crate::alternative_cast::CastingMethod;
    use crate::card::CardBuilder;
    use crate::game_state::StackEntry;
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};

    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let spell = CardBuilder::new(CardId::from_raw(47_260), "Cast Origin Probe")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .build();

    let hand_id = game.create_object_from_card(&spell, alice, Zone::Hand);
    let hand_stack_id = game
        .move_object_by_effect(hand_id, Zone::Stack)
        .expect("move hand spell to stack");
    game.push_to_stack(
        StackEntry::new(hand_stack_id, alice).with_casting_method(CastingMethod::Normal),
    );

    let graveyard_id = game.create_object_from_card(&spell, alice, Zone::Graveyard);
    let graveyard_stack_id = game
        .move_object_by_effect(graveyard_id, Zone::Stack)
        .expect("move graveyard spell to stack");
    game.push_to_stack(
        StackEntry::new(graveyard_stack_id, alice).with_casting_method(CastingMethod::PlayFrom {
            source: graveyard_stack_id,
            zone: Zone::Graveyard,
            use_alternative: None,
        }),
    );

    let filter = ObjectFilter::spell().not_cast_from_zone(Zone::Hand);
    let context = FilterContext::new(alice);
    assert_eq!(
        filter.description(),
        "spell that wasn't cast from its owner's hand"
    );
    assert!(
        !filter.matches(
            game.object(hand_stack_id).expect("hand spell on stack"),
            &context,
            &game,
        ),
        "a spell cast from hand must not satisfy the negative hand-origin filter"
    );
    assert!(
        filter.matches(
            game.object(graveyard_stack_id)
                .expect("graveyard spell on stack"),
            &context,
            &game,
        ),
        "a spell cast from a graveyard should satisfy the negative hand-origin filter"
    );
}

#[test]
fn test_graveyard_filter_uses_current_subtypes() {
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::cards::CardDefinitionBuilder;
    use crate::ids::CardId;
    use crate::static_abilities::StaticAbility;
    use crate::zone::Zone;

    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let _beacon_id = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(30), "Graveyard Beacon")
            .card_types(vec![CardType::Artifact])
            .with_ability(Ability::static_ability(StaticAbility::add_subtypes(
                ObjectFilter::default()
                    .in_zone(Zone::Graveyard)
                    .owned_by(PlayerFilter::You)
                    .with_type(CardType::Creature),
                vec![Subtype::Wizard],
            )))
            .build(),
        alice,
        Zone::Battlefield,
    );

    let graveyard_id = game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(31), "Vanilla Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        alice,
        Zone::Graveyard,
    );

    let filter = ObjectFilter::default()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::You)
        .with_type(CardType::Creature)
        .with_subtype(Subtype::Wizard);
    let ctx = FilterContext::new(alice);
    let object = game
        .object(graveyard_id)
        .expect("graveyard card should exist");

    assert!(
        filter.matches(object, &ctx, &game),
        "off-battlefield subtype filters should use current characteristics"
    );
}

#[test]
fn test_filter_cast_by_matches_context_caster_for_nonstack_cards() {
    use crate::card::CardBuilder;
    use crate::ids::CardId;
    use crate::zone::Zone;

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let spell = CardBuilder::new(CardId::from_raw(3), "Borrowed Probe")
        .card_types(vec![CardType::Instant])
        .build();
    let spell_id = game.create_object_from_card(&spell, bob, Zone::Exile);
    let object = game.object(spell_id).expect("spell card should exist");

    let filter = ObjectFilter::default()
        .with_type(CardType::Instant)
        .cast_by_you();
    let alice_casting_ctx = FilterContext::new(alice).with_caster(Some(alice));
    assert!(
        filter.matches(object, &alice_casting_ctx, &game),
        "cast-by filter should use context caster for non-stack card objects"
    );

    let bob_casting_ctx = FilterContext::new(alice).with_caster(Some(bob));
    assert!(
        !filter.matches(object, &bob_casting_ctx, &game),
        "cast-by filter should reject when context caster does not match"
    );

    let no_caster_ctx = FilterContext::new(alice);
    assert!(
        !filter.matches(object, &no_caster_ctx, &game),
        "cast-by filter should not match non-stack cards without explicit caster context"
    );
}

#[test]
fn test_filter_cast_by_uses_stack_controller_when_caster_missing() {
    use crate::alternative_cast::CastingMethod;
    use crate::card::CardBuilder;
    use crate::game_state::StackEntry;
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::zone::Zone;

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let spell = CardBuilder::new(CardId::from_raw(4), "Stack Probe")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .build();
    let hand_id = game.create_object_from_card(&spell, alice, Zone::Hand);
    let stack_id = game
        .move_object_by_effect(hand_id, Zone::Stack)
        .expect("move spell to stack");
    game.push_to_stack(StackEntry::new(stack_id, alice).with_casting_method(CastingMethod::Normal));
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new(stack_id, alice, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    game.stage_turn_history_event(&event);

    let filter = ObjectFilter::spell().cast_by_you();
    let object = game.object(stack_id).expect("stack spell should exist");
    let alice_ctx = FilterContext::new(alice);
    assert!(
        filter.matches(object, &alice_ctx, &game),
        "cast-by filter should fall back to stack controller when caster context is absent"
    );
    let bob_ctx = FilterContext::new(bob);
    assert!(
        !filter.matches(object, &bob_ctx, &game),
        "cast-by filter should respect 'you' against the stack spell controller"
    );
}

#[test]
fn test_filter_description_includes_positive_colors() {
    let filter =
        ObjectFilter::creature().with_colors(ColorSet::from_color(crate::color::Color::Blue));
    assert_eq!(filter.description(), "blue creature");
}

#[test]
fn required_colors_match_all_members_and_render_as_both() {
    use crate::card::{CardBuilder, PowerToughness};
    use crate::ids::CardId;

    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let blue = ColorSet::BLUE;
    let blue_black = blue.union(ColorSet::BLACK);
    let blue_creature = CardBuilder::new(CardId::from_raw(1), "Blue")
        .card_types(vec![CardType::Creature])
        .color_indicator(blue)
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let both_creature = CardBuilder::new(CardId::from_raw(2), "Both")
        .card_types(vec![CardType::Creature])
        .color_indicator(blue_black)
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let blue_id = game.create_object_from_card(&blue_creature, alice, Zone::Battlefield);
    let both_id = game.create_object_from_card(&both_creature, alice, Zone::Battlefield);
    let mut filter = ObjectFilter::creature();
    filter.required_colors = Some(blue_black);
    let ctx = FilterContext::new(alice);

    assert!(!filter.matches(game.object(blue_id).unwrap(), &ctx, &game));
    assert!(filter.matches(game.object(both_id).unwrap(), &ctx, &game));
    assert_eq!(filter.description(), "both blue and black creature");
}

#[test]
fn sticker_filter_uses_stable_object_annotation() {
    use crate::card::{CardBuilder, PowerToughness};
    use crate::ids::CardId;

    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let card = CardBuilder::new(CardId::from_raw(3), "Sticker target")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let object_id = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let mut filter = ObjectFilter::creature();
    filter.sticker = Some(crate::events::KeywordActionKind::ArtSticker);
    let ctx = FilterContext::new(alice);

    assert!(!filter.matches(game.object(object_id).unwrap(), &ctx, &game));
    game.put_sticker_on_object(object_id, crate::events::KeywordActionKind::ArtSticker);
    assert!(filter.matches(game.object(object_id).unwrap(), &ctx, &game));
    assert_eq!(filter.description(), "creature with an art sticker on it");
}

#[test]
fn test_filter_description_includes_tapped_state() {
    let filter = ObjectFilter::creature().tapped();
    assert_eq!(filter.description(), "tapped creature");
}

#[test]
fn test_filter_description_includes_modified_state() {
    let filter = ObjectFilter::creature().modified();
    assert_eq!(filter.description(), "modified creature");
}

#[test]
fn test_filter_description_includes_face_down_state() {
    let filter = ObjectFilter::creature().face_down();
    assert_eq!(filter.description(), "face-down creature");
}

#[test]
fn test_filter_matches_face_down_state() {
    use crate::card::{CardBuilder, PowerToughness};
    use crate::game_state::GameState;
    use crate::ids::CardId;

    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let controller = PlayerId::from_index(0);
    let card = CardBuilder::new(CardId::from_raw(1), "Face-Down Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let object_id = game.create_object_from_card(&card, controller, Zone::Battlefield);

    let ctx = FilterContext::new(controller).with_source(object_id);
    let face_down_filter = ObjectFilter::creature().face_down();
    let face_up_filter = ObjectFilter::creature().face_up();

    let object = game.object(object_id).expect("created object should exist");
    assert!(
        face_up_filter.matches(object, &ctx, &game),
        "face-up filter should match by default"
    );
    assert!(
        !face_down_filter.matches(object, &ctx, &game),
        "face-down filter should not match a face-up object"
    );

    game.set_face_down(object_id);
    let object = game.object(object_id).expect("created object should exist");
    assert!(
        face_down_filter.matches(object, &ctx, &game),
        "face-down filter should match after object is set face down"
    );
    assert!(
        !face_up_filter.matches(object, &ctx, &game),
        "face-up filter should not match a face-down object"
    );
}

#[test]
fn foretold_filter_does_not_match_arbitrary_face_down_exile_cards() {
    use crate::card::CardBuilder;
    use crate::game_state::GameState;
    use crate::ids::CardId;

    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let owner = PlayerId::from_index(0);
    let card = CardBuilder::new(CardId::from_raw(2), "Foretell State Probe").build();
    let object_id = game.create_object_from_card(&card, owner, Zone::Exile);
    game.set_face_down(object_id);

    let filter = ObjectFilter {
        zone: Some(Zone::Exile),
        owner: Some(PlayerFilter::You),
        foretold: true,
        ..ObjectFilter::default()
    };
    let ctx = FilterContext::new(owner);
    assert_eq!(filter.description(), "a foretold card you own in exile");
    assert!(
        !filter.matches(game.object(object_id).unwrap(), &ctx, &game),
        "face-down exile alone must not satisfy the foretold predicate"
    );

    game.set_foretold(object_id);
    assert!(filter.matches(game.object(object_id).unwrap(), &ctx, &game));
}

#[test]
fn test_filter_description_includes_all_card_types() {
    let filter = ObjectFilter::default()
        .with_all_type(CardType::Artifact)
        .with_all_type(CardType::Creature);
    assert_eq!(filter.description(), "artifact creature");
}

#[test]
fn test_filter_description_includes_excluded_subtypes() {
    let filter = ObjectFilter::creature()
        .without_subtype(crate::types::Subtype::Vampire)
        .without_subtype(crate::types::Subtype::Werewolf)
        .without_subtype(crate::types::Subtype::Zombie);
    assert_eq!(
        filter.description(),
        "non-vampire non-werewolf non-zombie creature"
    );
}

#[test]
fn filter_description_preserves_relative_and_independently_negated_unions() {
    let mut relative = ObjectFilter::creature()
        .you_control()
        .with_subtype(crate::types::Subtype::Fungus)
        .with_subtype(crate::types::Subtype::Saproling)
        .with_union_connective(ironsmith_core::ObjectFilterUnionConnective::AndOr);
    relative.set_relative_characteristic_list_surface(true);
    assert_eq!(
        ObjectFilterExt::description(&relative),
        "a creature you control that's a Fungus and/or Saproling"
    );

    let mut controller_qualified = ObjectFilter::creature()
        .you_control()
        .with_subtype(crate::types::Subtype::Vehicle)
        .with_union_connective(ironsmith_core::ObjectFilterUnionConnective::AndOr);
    controller_qualified.type_or_subtype_union = true;
    controller_qualified.set_subtype_before_card_type_union_surface(true);
    assert_eq!(
        ObjectFilterExt::description(&controller_qualified),
        "a Vehicle and/or creature you control"
    );

    let mut independently_negated = ObjectFilter {
        zone: Some(Zone::Graveyard),
        owner: Some(PlayerFilter::You),
        any_of: vec![
            ObjectFilter::default().with_type(CardType::Artifact),
            ObjectFilter {
                card_types: vec![CardType::Enchantment],
                excluded_subtypes: vec![crate::types::Subtype::Aura],
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    independently_negated.set_explicit_card_noun(true);
    assert_eq!(
        ObjectFilterExt::description(&independently_negated),
        "artifact or non-aura enchantment card in your graveyard"
    );
}

#[test]
fn filter_description_compacts_mirrored_owner_or_controller_branches() {
    let permanent = ObjectFilter::permanent();
    let union = ObjectFilter {
        any_of: vec![
            permanent.clone().owned_by(PlayerFilter::You),
            permanent.controlled_by(PlayerFilter::You),
        ],
        ..Default::default()
    };

    assert_eq!(union.description(), "a permanent you own or control");
}

#[test]
fn filter_description_keeps_distinct_owner_or_controller_branches_expanded() {
    let distinct_selectors = ObjectFilter {
        any_of: vec![
            ObjectFilter::permanent().owned_by(PlayerFilter::You),
            ObjectFilter::creature().controlled_by(PlayerFilter::You),
        ],
        ..Default::default()
    };
    assert_eq!(
        distinct_selectors.description(),
        "a permanent you own or a creature you control"
    );

    let distinct_players = ObjectFilter {
        any_of: vec![
            ObjectFilter::permanent().owned_by(PlayerFilter::You),
            ObjectFilter::permanent().controlled_by(PlayerFilter::Opponent),
        ],
        ..Default::default()
    };
    assert_eq!(
        distinct_players.description(),
        "a permanent you own or a permanent an opponent controls"
    );
}

#[test]
fn filter_description_compacts_the_complete_owned_nonbattlefield_zone_union() {
    let branches = [
        Zone::Hand,
        Zone::Library,
        Zone::Graveyard,
        Zone::Exile,
        Zone::Command,
    ]
    .into_iter()
    .map(|zone| {
        let mut branch = ObjectFilter {
            zone: Some(zone),
            owner: Some(PlayerFilter::You),
            card_types: vec![CardType::Creature],
            ..Default::default()
        };
        branch.set_explicit_card_noun(true);
        branch
    })
    .collect();
    let union = ObjectFilter {
        any_of: branches,
        ..Default::default()
    };

    assert_eq!(
        union.description(),
        "creature cards you own that aren't on the battlefield"
    );
}

#[test]
fn owned_nonbattlefield_union_compaction_rejects_an_extra_controller_constraint() {
    let mut branches = [
        Zone::Hand,
        Zone::Library,
        Zone::Graveyard,
        Zone::Exile,
        Zone::Command,
    ]
    .into_iter()
    .map(|zone| {
        let mut branch = ObjectFilter {
            zone: Some(zone),
            owner: Some(PlayerFilter::You),
            card_types: vec![CardType::Creature],
            ..Default::default()
        };
        branch.set_explicit_card_noun(true);
        branch
    })
    .collect::<Vec<_>>();
    branches[0].controller = Some(PlayerFilter::You);
    let union = ObjectFilter {
        any_of: branches,
        ..Default::default()
    };

    assert_ne!(
        union.description(),
        "creature cards you own that aren't on the battlefield"
    );
}

#[test]
fn filter_description_compacts_controlled_and_owned_nonbattlefield_domains() {
    let mut battlefield = ObjectFilter {
        zone: Some(Zone::Battlefield),
        controller: Some(PlayerFilter::You),
        card_types: vec![CardType::Land],
        ..Default::default()
    };
    battlefield.set_plural_object_noun_surface(true);
    let mut branches = vec![battlefield];
    branches.extend(
        [
            Zone::Hand,
            Zone::Library,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Command,
        ]
        .into_iter()
        .map(|zone| {
            let mut branch = ObjectFilter {
                zone: Some(zone),
                owner: Some(PlayerFilter::You),
                card_types: vec![CardType::Land],
                ..Default::default()
            };
            branch.set_explicit_card_noun(true);
            branch.set_plural_object_noun_surface(true);
            branch
        }),
    );
    let mut union = ObjectFilter {
        any_of: branches,
        ..Default::default()
    };
    union.set_conjunctive_set_surface(true);

    assert_eq!(
        union.description(),
        "lands you control and land cards you own that aren't on the battlefield"
    );
}

#[test]
fn test_filter_description_compacts_full_outlaw_subtype_pack() {
    let filter = ObjectFilter::creature()
        .with_subtype(crate::types::Subtype::Assassin)
        .with_subtype(crate::types::Subtype::Mercenary)
        .with_subtype(crate::types::Subtype::Pirate)
        .with_subtype(crate::types::Subtype::Rogue)
        .with_subtype(crate::types::Subtype::Warlock);
    assert_eq!(filter.description(), "outlaw creature");
}

#[test]
fn test_filter_description_compacts_outlaw_pack_with_extra_subtypes() {
    let filter = ObjectFilter::creature()
        .with_subtype(crate::types::Subtype::Assassin)
        .with_subtype(crate::types::Subtype::Mercenary)
        .with_subtype(crate::types::Subtype::Pirate)
        .with_subtype(crate::types::Subtype::Rogue)
        .with_subtype(crate::types::Subtype::Warlock)
        .with_subtype(crate::types::Subtype::Wizard);
    assert_eq!(filter.description(), "outlaw or Wizard creature");
}

#[test]
fn test_filter_description_includes_skulk() {
    let mut filter = ObjectFilter::creature();
    filter.static_abilities.push(StaticAbilityId::Skulk);
    assert_eq!(filter.description(), "creature with skulk");
}

#[test]
fn test_filter_description_includes_excluded_colors() {
    let filter = ObjectFilter::creature().without_colors(
        ColorSet::from_color(crate::color::Color::Black)
            .union(ColorSet::from_color(crate::color::Color::Red)),
    );
    assert_eq!(filter.description(), "nonblack nonred creature");
}

#[test]
fn test_filter_description_includes_chosen_color_clause() {
    let filter = ObjectFilter::spell().of_chosen_color();
    assert_eq!(filter.description(), "spell of the chosen color");
}

#[test]
fn test_filter_description_includes_entered_since_last_turn_ended_clause() {
    let filter = ObjectFilter {
        card_types: vec![CardType::Creature],
        entered_since_your_last_turn_ended: true,
        ..Default::default()
    };
    assert_eq!(
        filter.description(),
        "creature that entered since your last turn ended"
    );
}

#[test]
fn test_filter_description_includes_commander_owner_and_controller_distinction() {
    let filter = ObjectFilter::creature()
        .commander()
        .owned_by(PlayerFilter::You)
        .controlled_by(PlayerFilter::Opponent);
    assert_eq!(
        filter.description(),
        "a commander creature you own but an opponent controls"
    );
}

fn setup_modified_filter_game() -> crate::game_state::GameState {
    crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
}

fn create_modified_test_creature(
    game: &mut crate::game_state::GameState,
    controller: PlayerId,
) -> ObjectId {
    use crate::card::{CardBuilder, PowerToughness};
    use crate::ids::CardId;
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    let card = CardBuilder::new(CardId::from_raw(1), "Test Creature")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Bear])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    game.create_object_from_card(&card, controller, Zone::Battlefield)
}

fn create_modified_test_equipment(
    game: &mut crate::game_state::GameState,
    controller: PlayerId,
) -> ObjectId {
    use crate::card::CardBuilder;
    use crate::ids::CardId;
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    let card = CardBuilder::new(CardId::from_raw(2), "Test Equipment")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .build();
    game.create_object_from_card(&card, controller, Zone::Battlefield)
}

fn create_modified_test_aura(
    game: &mut crate::game_state::GameState,
    controller: PlayerId,
) -> ObjectId {
    use crate::card::CardBuilder;
    use crate::ids::CardId;
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    let card = CardBuilder::new(CardId::from_raw(3), "Test Aura")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .build();
    game.create_object_from_card(&card, controller, Zone::Battlefield)
}

#[test]
fn test_filter_matches_modified_by_counter() {
    let mut game = setup_modified_filter_game();
    let alice = PlayerId::from_index(0);
    let creature_id = create_modified_test_creature(&mut game, alice);

    let ctx = FilterContext::new(alice).with_source(creature_id);
    let filter = ObjectFilter::creature().you_control().modified();

    let creature = game.object(creature_id).expect("creature exists");
    assert!(
        !filter.matches(creature, &ctx, &game),
        "unmodified creature should not match"
    );

    game.object_mut(creature_id)
        .expect("creature exists")
        .counters
        .insert(CounterType::PlusOnePlusOne, 1);
    let creature = game.object(creature_id).expect("creature exists");
    assert!(
        filter.matches(creature, &ctx, &game),
        "creature with a counter should match"
    );
}

#[test]
fn counter_threshold_filter_matches_at_and_above_the_minimum() {
    let mut game = setup_modified_filter_game();
    let alice = PlayerId::from_index(0);
    let creature_id = create_modified_test_creature(&mut game, alice);
    let ctx = FilterContext::new(alice).with_source(creature_id);
    let filter = ObjectFilter {
        with_counter: Some(CounterConstraint::AtLeast {
            counter_type: Some(CounterType::PlusOnePlusOne),
            count: 3,
        }),
        ..ObjectFilter::creature().you_control()
    };

    for (counter_count, expected) in [(2, false), (3, true), (4, true)] {
        game.object_mut(creature_id)
            .expect("creature exists")
            .counters
            .insert(CounterType::PlusOnePlusOne, counter_count);
        let creature = game.object(creature_id).expect("creature exists");
        assert_eq!(
            filter.matches(creature, &ctx, &game),
            expected,
            "{counter_count} counters should have threshold match {expected}"
        );
        assert_eq!(
            filter.matches_snapshot(&ObjectSnapshot::from_object(creature, &game), &ctx, &game,),
            expected,
            "snapshot with {counter_count} counters should have threshold match {expected}"
        );
    }
}

#[test]
fn test_filter_matches_modified_by_equipment() {
    let mut game = setup_modified_filter_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let creature_id = create_modified_test_creature(&mut game, alice);
    let equipment_id = create_modified_test_equipment(&mut game, bob);

    game.object_mut(creature_id)
        .expect("creature exists")
        .attachments
        .push(equipment_id);

    let ctx = FilterContext::new(alice).with_source(creature_id);
    let filter = ObjectFilter::creature().you_control().modified();
    let creature = game.object(creature_id).expect("creature exists");
    assert!(
        filter.matches(creature, &ctx, &game),
        "equipped creature should match regardless of equipment controller"
    );
}

#[test]
fn test_filter_matches_intrinsically_equipped_creature_without_tag_context() {
    let mut game = setup_modified_filter_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let creature_id = create_modified_test_creature(&mut game, alice);
    let equipment_id = create_modified_test_equipment(&mut game, bob);

    game.object_mut(creature_id)
        .expect("creature exists")
        .attachments
        .push(equipment_id);

    let mut filter = ObjectFilter::creature().you_control();
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from("equipped"),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    let ctx = FilterContext::new(alice).with_source(creature_id);
    let creature = game.object(creature_id).expect("creature exists");
    assert!(
        filter.matches(creature, &ctx, &game),
        "unbound equipped adjective should match a creature with Equipment attached"
    );
}

#[test]
fn test_filter_matches_intrinsically_equipped_snapshot_without_tag_context() {
    let mut game = setup_modified_filter_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let creature_id = create_modified_test_creature(&mut game, alice);
    let equipment_id = create_modified_test_equipment(&mut game, bob);

    game.object_mut(creature_id)
        .expect("creature exists")
        .attachments
        .push(equipment_id);

    let mut filter = ObjectFilter::creature().you_control();
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from("equipped"),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    let ctx = FilterContext::new(alice).with_source(creature_id);
    let snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(
        game.object(creature_id).expect("creature exists"),
        &game,
    );
    assert!(
        filter.matches_snapshot(&snapshot, &ctx, &game),
        "unbound equipped adjective should match LKI for a creature with Equipment attached"
    );
}

#[test]
fn test_filter_matches_modified_by_controlled_aura() {
    let mut game = setup_modified_filter_game();
    let alice = PlayerId::from_index(0);
    let creature_id = create_modified_test_creature(&mut game, alice);
    let aura_id = create_modified_test_aura(&mut game, alice);

    game.object_mut(creature_id)
        .expect("creature exists")
        .attachments
        .push(aura_id);

    let ctx = FilterContext::new(alice).with_source(creature_id);
    let filter = ObjectFilter::creature().you_control().modified();
    let creature = game.object(creature_id).expect("creature exists");
    assert!(
        filter.matches(creature, &ctx, &game),
        "creature enchanted by an Aura you control should match"
    );
}

#[test]
fn test_filter_does_not_match_modified_by_opponent_aura() {
    let mut game = setup_modified_filter_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let creature_id = create_modified_test_creature(&mut game, alice);
    let aura_id = create_modified_test_aura(&mut game, bob);

    game.object_mut(creature_id)
        .expect("creature exists")
        .attachments
        .push(aura_id);

    let ctx = FilterContext::new(alice).with_source(creature_id);
    let filter = ObjectFilter::creature().you_control().modified();
    let creature = game.object(creature_id).expect("creature exists");
    assert!(
        !filter.matches(creature, &ctx, &game),
        "Aura controlled by opponent should not make creature modified"
    );
}

#[test]
fn test_filter_matches_permanent_attached_to_player() {
    let mut game = setup_modified_filter_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let aura_id = create_modified_test_aura(&mut game, bob);

    game.object_mut(aura_id).expect("aura exists").attached_to =
        Some(crate::object::AttachmentTarget::Player(alice));

    let mut filter = ObjectFilter::permanent().with_subtype(Subtype::Aura);
    filter.attached_to_player = Some(PlayerFilter::You);

    let aura = game.object(aura_id).expect("aura exists");
    assert!(
        filter.matches(aura, &FilterContext::new(alice), &game),
        "Aura attached to Alice should match attached_to_player=You from Alice's context"
    );
    assert!(
        !filter.matches(aura, &FilterContext::new(bob), &game),
        "Aura attached to Alice should not match attached_to_player=You from Bob's context"
    );
}

#[test]
fn test_filter_matches_aura_attached_to_source_creature() {
    let mut game = setup_modified_filter_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source_id = create_modified_test_creature(&mut game, alice);
    let other_id = create_modified_test_creature(&mut game, bob);
    let aura_id = create_modified_test_aura(&mut game, alice);

    game.object_mut(aura_id).expect("aura exists").attached_to =
        Some(crate::object::AttachmentTarget::Object(source_id));

    let mut filter = ObjectFilter::permanent().with_subtype(Subtype::Aura);
    filter.attached_to_object = Some(Box::new(ObjectFilter::source()));

    let aura = game.object(aura_id).expect("aura exists");
    assert!(
        filter.matches(
            aura,
            &FilterContext::new(alice).with_source(source_id),
            &game
        ),
        "Aura attached to the source should satisfy the intrinsic relation"
    );
    assert!(
        !filter.matches(
            aura,
            &FilterContext::new(alice).with_source(other_id),
            &game
        ),
        "Aura attached elsewhere must not satisfy an attached-to-source selector"
    );
}

#[test]
fn test_filter_matches_aura_attached_to_creature_you_control() {
    let mut game = setup_modified_filter_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let your_creature_id = create_modified_test_creature(&mut game, alice);
    let opposing_creature_id = create_modified_test_creature(&mut game, bob);
    let your_aura_id = create_modified_test_aura(&mut game, alice);
    let opposing_aura_id = create_modified_test_aura(&mut game, alice);

    game.object_mut(your_aura_id)
        .expect("your aura exists")
        .attached_to = Some(crate::object::AttachmentTarget::Object(your_creature_id));
    game.object_mut(opposing_aura_id)
        .expect("opposing aura exists")
        .attached_to = Some(crate::object::AttachmentTarget::Object(
        opposing_creature_id,
    ));

    let mut filter = ObjectFilter::permanent().with_subtype(Subtype::Aura);
    filter.attached_to_object = Some(Box::new(ObjectFilter::creature().you_control()));
    let ctx = FilterContext::new(alice);

    assert!(filter.matches(
        game.object(your_aura_id).expect("your aura exists"),
        &ctx,
        &game
    ));
    assert!(!filter.matches(
        game.object(opposing_aura_id).expect("opposing aura exists"),
        &ctx,
        &game
    ));
}

#[test]
fn test_attachment_filter_matches_outer_aura_not_its_permanent_host() {
    let mut game = setup_modified_filter_game();
    let alice = PlayerId::from_index(0);
    let host_id = create_modified_test_creature(&mut game, alice);
    let aura_id = create_modified_test_aura(&mut game, alice);
    game.object_mut(aura_id).expect("aura exists").attached_to =
        Some(crate::object::AttachmentTarget::Object(host_id));

    let mut filter = ObjectFilter::permanent().with_subtype(Subtype::Aura);
    filter.attached_to_object = Some(Box::new(ObjectFilter::permanent().you_control()));
    let ctx = FilterContext::new(alice);

    assert!(filter.matches(game.object(aura_id).expect("aura exists"), &ctx, &game));
    assert!(
        !filter.matches(game.object(host_id).expect("host exists"), &ctx, &game),
        "the host must not be substituted for the selected Aura"
    );
}

#[test]
fn with_attached_aura_filter_keeps_host_and_aura_controllers_independent() {
    let mut game = setup_modified_filter_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let matching_host = create_modified_test_creature(&mut game, bob);
    let wrong_host_controller = create_modified_test_creature(&mut game, alice);
    let wrong_aura_controller = create_modified_test_creature(&mut game, bob);
    let unattached_host = create_modified_test_creature(&mut game, bob);
    let your_aura_on_opponent = create_modified_test_aura(&mut game, alice);
    let your_aura_on_your_creature = create_modified_test_aura(&mut game, alice);
    let opponent_aura = create_modified_test_aura(&mut game, bob);

    game.object_mut(matching_host)
        .expect("matching host exists")
        .attachments
        .push(your_aura_on_opponent);
    game.object_mut(wrong_host_controller)
        .expect("wrong-controller host exists")
        .attachments
        .push(your_aura_on_your_creature);
    game.object_mut(wrong_aura_controller)
        .expect("wrong-aura host exists")
        .attachments
        .push(opponent_aura);

    let mut filter = ObjectFilter::creature().controlled_by(PlayerFilter::Opponent);
    filter.with_attached_object = Some(Box::new(
        ObjectFilter::default()
            .with_subtype(Subtype::Aura)
            .controlled_by(PlayerFilter::You),
    ));
    let ctx = FilterContext::new(alice).with_opponents(vec![bob]);

    assert!(
        filter.matches(
            game.object(matching_host).expect("matching host exists"),
            &ctx,
            &game
        ),
        "an opponent's creature enchanted by your Aura should match"
    );
    assert!(
        !filter.matches(
            game.object(wrong_host_controller)
                .expect("wrong-controller host exists"),
            &ctx,
            &game
        ),
        "your own creature must fail the outer opponent-controller constraint"
    );
    assert!(
        !filter.matches(
            game.object(wrong_aura_controller)
                .expect("wrong-aura host exists"),
            &ctx,
            &game
        ),
        "an opponent-controlled Aura must fail the nested Aura-controller constraint"
    );
    assert!(
        !filter.matches(
            game.object(unattached_host)
                .expect("unattached opponent host exists"),
            &ctx,
            &game
        ),
        "an opponent's creature without an attached Aura must not match"
    );
}

#[test]
fn different_name_from_tagged_excludes_all_tagged_names() {
    use crate::card::CardBuilder;
    use crate::ids::CardId;
    use crate::snapshot::ObjectSnapshot;

    let mut game = setup_modified_filter_game();
    let alice = PlayerId::from_index(0);
    let tag = TagKey::from("attached_curses");

    let attached_misfortunes = CardBuilder::new(CardId::from_raw(10), "Curse of Misfortunes")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura, Subtype::Curse])
        .build();
    let attached_thirst = CardBuilder::new(CardId::from_raw(11), "Curse of Thirst")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura, Subtype::Curse])
        .build();
    let candidate_same = CardBuilder::new(CardId::from_raw(12), "Curse of Misfortunes")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura, Subtype::Curse])
        .build();
    let candidate_different = CardBuilder::new(CardId::from_raw(13), "Curse of Death's Hold")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura, Subtype::Curse])
        .build();

    let attached_misfortunes_id =
        game.create_object_from_card(&attached_misfortunes, alice, Zone::Battlefield);
    let attached_thirst_id =
        game.create_object_from_card(&attached_thirst, alice, Zone::Battlefield);
    let candidate_same_id = game.create_object_from_card(&candidate_same, alice, Zone::Library);
    let candidate_different_id =
        game.create_object_from_card(&candidate_different, alice, Zone::Library);

    let mut ctx = FilterContext::new(alice);
    ctx.tagged_objects.insert(
        tag.clone(),
        vec![
            ObjectSnapshot::from_object(
                game.object(attached_misfortunes_id)
                    .expect("attached object"),
                &game,
            ),
            ObjectSnapshot::from_object(
                game.object(attached_thirst_id).expect("attached object"),
                &game,
            ),
        ],
    );

    let mut filter = ObjectFilter::default().with_subtype(Subtype::Curse);
    filter.zone = Some(Zone::Library);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag,
        relation: TaggedOpbjectRelation::DifferentNameFromTagged,
    });

    assert!(
        !filter.matches(
            game.object(candidate_same_id).expect("same-name candidate"),
            &ctx,
            &game
        ),
        "candidate sharing any tagged Curse name should not match"
    );
    assert!(
        filter.matches(
            game.object(candidate_different_id)
                .expect("different-name candidate"),
            &ctx,
            &game
        ),
        "candidate with a name different from every tagged Curse should match"
    );
}

#[test]
fn test_player_filter_matching() {
    let you = PlayerId::from_index(0);
    let opponent = PlayerId::from_index(1);

    let ctx = FilterContext::new(you).with_opponents(vec![opponent]);

    assert!(PlayerFilter::Any.matches_player(you, &ctx));
    assert!(PlayerFilter::Any.matches_player(opponent, &ctx));

    assert!(PlayerFilter::You.matches_player(you, &ctx));
    assert!(!PlayerFilter::You.matches_player(opponent, &ctx));

    assert!(!PlayerFilter::Opponent.matches_player(you, &ctx));
    assert!(PlayerFilter::Opponent.matches_player(opponent, &ctx));

    assert!(PlayerFilter::Specific(you).matches_player(you, &ctx));
    assert!(!PlayerFilter::Specific(you).matches_player(opponent, &ctx));
}

#[test]
fn test_player_filter_controller_of_target_uses_target_snapshot() {
    use crate::card::CardBuilder;
    use crate::ids::CardId;
    use crate::snapshot::ObjectSnapshot;

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = crate::tests::test_helpers::setup_two_player_game();

    let land = CardBuilder::new(CardId::from_raw(1001), "Target Forest")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Forest])
        .build();
    let land_id = game.create_object_from_card(&land, bob, Zone::Battlefield);
    let snapshot =
        ObjectSnapshot::from_object(game.object(land_id).expect("target land exists"), &game);

    let ctx = FilterContext::new(alice).with_target_objects(vec![snapshot]);
    let controller_filter = PlayerFilter::ControllerOf(ObjectRef::Target);
    let owner_filter = PlayerFilter::OwnerOf(ObjectRef::Target);

    assert!(controller_filter.matches_player(bob, &ctx));
    assert!(!controller_filter.matches_player(alice, &ctx));
    assert!(owner_filter.matches_player(bob, &ctx));
    assert!(!owner_filter.matches_player(alice, &ctx));
}

#[test]
fn test_excluded_supertypes_builder() {
    use crate::types::Supertype;

    let filter = ObjectFilter::land().without_supertype(Supertype::Basic);
    assert_eq!(filter.excluded_supertypes, vec![Supertype::Basic]);
}

#[test]
fn test_nonbasic_shorthand() {
    use crate::types::Supertype;

    let filter = ObjectFilter::land().nonbasic();
    assert_eq!(filter.excluded_supertypes, vec![Supertype::Basic]);
}

#[test]
fn nonbasic_land_type_filter_checks_subtypes_not_the_basic_supertype() {
    use crate::card::CardBuilder;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId};
    use crate::object::Object;
    use crate::types::{Subtype, Supertype};

    let player = PlayerId::from_index(0);
    let game = GameState::new(vec!["Alice".to_string()], 20);
    let filter = ObjectFilter {
        has_nonbasic_land_type: true,
        ..Default::default()
    };
    let desert = CardBuilder::new(CardId::from_raw(11), "Desert")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Desert])
        .build();
    let dual = CardBuilder::new(CardId::from_raw(12), "Steam Vents")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Island, Subtype::Mountain])
        .build();
    let basic = CardBuilder::new(CardId::from_raw(13), "Forest")
        .card_types(vec![CardType::Land])
        .supertypes(vec![Supertype::Basic])
        .subtypes(vec![Subtype::Forest])
        .build();
    let desert = Object::from_card(ObjectId::from_raw(11), &desert, player, Zone::Battlefield);
    let dual = Object::from_card(ObjectId::from_raw(12), &dual, player, Zone::Battlefield);
    let basic = Object::from_card(ObjectId::from_raw(13), &basic, player, Zone::Battlefield);
    let ctx = FilterContext::new(player);

    assert!(filter.matches(&desert, &ctx, &game));
    assert!(!filter.matches(&dual, &ctx, &game));
    assert!(!filter.matches(&basic, &ctx, &game));
}

#[test]
fn basic_land_type_filter_checks_subtypes_not_the_basic_supertype() {
    use crate::card::CardBuilder;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId};
    use crate::object::Object;
    use crate::types::{Subtype, Supertype};

    let player = PlayerId::from_index(0);
    let game = GameState::new(vec!["Alice".to_string()], 20);
    let filter = ObjectFilter {
        has_basic_land_type: true,
        ..Default::default()
    };
    let desert = CardBuilder::new(CardId::from_raw(21), "Desert")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Desert])
        .build();
    let dual = CardBuilder::new(CardId::from_raw(22), "Steam Vents")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Island, Subtype::Mountain])
        .build();
    let basic = CardBuilder::new(CardId::from_raw(23), "Forest")
        .card_types(vec![CardType::Land])
        .supertypes(vec![Supertype::Basic])
        .subtypes(vec![Subtype::Forest])
        .build();
    let desert = Object::from_card(ObjectId::from_raw(21), &desert, player, Zone::Battlefield);
    let dual = Object::from_card(ObjectId::from_raw(22), &dual, player, Zone::Battlefield);
    let basic = Object::from_card(ObjectId::from_raw(23), &basic, player, Zone::Battlefield);
    let ctx = FilterContext::new(player);

    assert!(!filter.matches(&desert, &ctx, &game));
    assert!(filter.matches(&dual, &ctx, &game));
    assert!(filter.matches(&basic, &ctx, &game));
}

#[test]
fn test_excluded_supertypes_matching() {
    use crate::card::CardBuilder;
    use crate::game_state::GameState;
    use crate::ids::CardId;
    use crate::object::Object;
    use crate::types::Supertype;

    let p0 = PlayerId::from_index(0);

    // Create a basic land
    let basic_forest_card = CardBuilder::new(CardId::from_raw(1), "Forest")
        .card_types(vec![CardType::Land])
        .supertypes(vec![Supertype::Basic])
        .subtypes(vec![crate::types::Subtype::Forest])
        .build();
    let basic_forest = Object::from_card(
        crate::ids::ObjectId::from_raw(1),
        &basic_forest_card,
        p0,
        Zone::Battlefield,
    );

    // Create a nonbasic land
    let nonbasic_land_card = CardBuilder::new(CardId::from_raw(2), "Steam Vents")
        .card_types(vec![CardType::Land])
        .subtypes(vec![
            crate::types::Subtype::Island,
            crate::types::Subtype::Mountain,
        ])
        .build();
    let nonbasic_land = Object::from_card(
        crate::ids::ObjectId::from_raw(2),
        &nonbasic_land_card,
        p0,
        Zone::Battlefield,
    );

    // Filter for nonbasic lands (excludes Basic supertype)
    let nonbasic_filter = ObjectFilter::land().nonbasic();
    let ctx = FilterContext::new(p0);
    let game = GameState::new(vec!["Alice".to_string()], 20);

    // Basic land should NOT match (has Basic supertype which is excluded)
    assert!(
        !nonbasic_filter.matches(&basic_forest, &ctx, &game),
        "Basic Forest should not match nonbasic filter"
    );

    // Nonbasic land SHOULD match (doesn't have Basic supertype)
    assert!(
        nonbasic_filter.matches(&nonbasic_land, &ctx, &game),
        "Steam Vents should match nonbasic filter"
    );
}

#[test]
fn test_blood_moon_filter_for_nonbasic_lands() {
    use crate::card::CardBuilder;
    use crate::game_state::GameState;
    use crate::ids::CardId;
    use crate::types::Supertype;

    let p0 = PlayerId::from_index(0);

    // Blood Moon filter: nonbasic lands on the battlefield
    let blood_moon_filter = ObjectFilter {
        zone: Some(Zone::Battlefield),
        card_types: vec![CardType::Land],
        excluded_supertypes: vec![Supertype::Basic],
        ..Default::default()
    };

    // Create basic Plains
    let plains_card = CardBuilder::new(CardId::from_raw(1), "Plains")
        .card_types(vec![CardType::Land])
        .supertypes(vec![Supertype::Basic])
        .subtypes(vec![crate::types::Subtype::Plains])
        .build();
    let plains = Object::from_card(
        crate::ids::ObjectId::from_raw(1),
        &plains_card,
        p0,
        Zone::Battlefield,
    );

    // Create Breeding Pool (nonbasic)
    let breeding_pool_card = CardBuilder::new(CardId::from_raw(2), "Breeding Pool")
        .card_types(vec![CardType::Land])
        .subtypes(vec![
            crate::types::Subtype::Forest,
            crate::types::Subtype::Island,
        ])
        .build();
    let breeding_pool = Object::from_card(
        crate::ids::ObjectId::from_raw(2),
        &breeding_pool_card,
        p0,
        Zone::Battlefield,
    );

    let ctx = FilterContext::new(p0);
    let game = GameState::new(vec!["Alice".to_string()], 20);

    // Blood Moon should NOT affect basic Plains
    assert!(
        !blood_moon_filter.matches(&plains, &ctx, &game),
        "Blood Moon filter should not match basic Plains"
    );

    // Blood Moon SHOULD affect Breeding Pool
    assert!(
        blood_moon_filter.matches(&breeding_pool, &ctx, &game),
        "Blood Moon filter should match Breeding Pool"
    );
}

#[test]
fn test_commander_filter_matches_true_commander_regardless_of_ctx_owner_list() {
    use crate::card::CardBuilder;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId};
    use crate::object::Object;

    let you = PlayerId::from_index(0);
    let opponent = PlayerId::from_index(1);

    let commander_card = CardBuilder::new(CardId::from_raw(99), "Opponent Commander")
        .card_types(vec![CardType::Creature])
        .build();
    let commander_obj = Object::from_card(
        ObjectId::from_raw(99),
        &commander_card,
        opponent,
        Zone::Battlefield,
    );

    let mut game = GameState::new(vec!["You".to_string(), "Opponent".to_string()], 20);
    game.add_object(commander_obj.clone());
    game.set_as_commander(commander_obj.id, opponent);

    let filter = ObjectFilter::creature().commander();
    let ctx = FilterContext::new(you).with_your_commanders(Vec::new());
    assert!(
        filter.matches(&commander_obj, &ctx, &game),
        "commander filter should rely on game commander identity, not ctx.your_commanders"
    );
}

#[test]
fn test_historic_and_nonhistoric_filters_match_correctly() {
    use crate::card::CardBuilder;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId};
    use crate::object::Object;

    let you = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["You".to_string()], 20);

    let artifact_card = CardBuilder::new(CardId::from_raw(1), "Mox")
        .card_types(vec![CardType::Artifact])
        .build();
    let artifact_obj = Object::from_card(
        ObjectId::from_raw(1),
        &artifact_card,
        you,
        Zone::Battlefield,
    );
    game.add_object(artifact_obj.clone());

    let creature_card = CardBuilder::new(CardId::from_raw(2), "Bear")
        .card_types(vec![CardType::Creature])
        .build();
    let creature_obj = Object::from_card(
        ObjectId::from_raw(2),
        &creature_card,
        you,
        Zone::Battlefield,
    );
    game.add_object(creature_obj.clone());

    let ctx = FilterContext::new(you);
    assert!(
        ObjectFilter::permanent()
            .historic()
            .matches(&artifact_obj, &ctx, &game)
    );
    assert!(
        !ObjectFilter::permanent()
            .historic()
            .matches(&creature_obj, &ctx, &game)
    );
    assert!(
        ObjectFilter::permanent()
            .nonhistoric()
            .matches(&creature_obj, &ctx, &game)
    );
    assert!(
        !ObjectFilter::permanent()
            .nonhistoric()
            .matches(&artifact_obj, &ctx, &game)
    );
}

#[test]
fn test_shares_color_with_tagged_constraint() {
    use crate::card::CardBuilder;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId};
    use crate::object::Object;
    use crate::snapshot::ObjectSnapshot;
    use crate::tag::TagKey;

    let you = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["You".to_string()], 20);

    let red_card = CardBuilder::new(CardId::from_raw(10), "Red Creature")
        .card_types(vec![CardType::Creature])
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::Red,
        ]]))
        .build();
    let red_obj = Object::from_card(ObjectId::from_raw(10), &red_card, you, Zone::Battlefield);
    game.add_object(red_obj.clone());

    let blue_card = CardBuilder::new(CardId::from_raw(11), "Blue Creature")
        .card_types(vec![CardType::Creature])
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::Blue,
        ]]))
        .build();
    let blue_obj = Object::from_card(ObjectId::from_raw(11), &blue_card, you, Zone::Battlefield);
    game.add_object(blue_obj.clone());

    let mut tagged = std::collections::HashMap::new();
    tagged.insert(
        TagKey::from("it"),
        vec![ObjectSnapshot::from_object(&red_obj, &game)],
    );
    let ctx = FilterContext::new(you).with_tagged_objects(&tagged);
    let filter = ObjectFilter::creature().shares_color_with_tagged("it");

    assert!(filter.matches(&red_obj, &ctx, &game));
    assert!(!filter.matches(&blue_obj, &ctx, &game));
}

#[test]
fn shares_subtype_with_each_tagged_requires_a_match_for_every_snapshot() {
    use crate::card::CardBuilder;
    use crate::ids::CardId;
    use crate::snapshot::ObjectSnapshot;

    let you = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["You".to_string()], 20);
    let elf_warrior = CardBuilder::new(CardId::from_raw(20), "Elf Warrior")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elf, Subtype::Warrior])
        .build();
    let human_wizard = CardBuilder::new(CardId::from_raw(21), "Human Wizard")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Wizard])
        .build();
    let elf_wizard = CardBuilder::new(CardId::from_raw(22), "Elf Wizard")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elf, Subtype::Wizard])
        .build();
    let elf_only = CardBuilder::new(CardId::from_raw(23), "Elf Only")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elf])
        .build();

    let elf_warrior_id = game.create_object_from_card(&elf_warrior, you, Zone::Battlefield);
    let human_wizard_id = game.create_object_from_card(&human_wizard, you, Zone::Battlefield);
    let elf_wizard_id = game.create_object_from_card(&elf_wizard, you, Zone::Hand);
    let elf_only_id = game.create_object_from_card(&elf_only, you, Zone::Hand);
    let tag = TagKey::from("tap_cost_0");
    let tagged = std::collections::HashMap::from([(
        tag.clone(),
        vec![
            ObjectSnapshot::from_object(game.object(elf_warrior_id).unwrap(), &game),
            ObjectSnapshot::from_object(game.object(human_wizard_id).unwrap(), &game),
        ],
    )]);
    let ctx = FilterContext::new(you).with_tagged_objects(&tagged);
    let any_filter = ObjectFilter::default()
        .in_zone(Zone::Hand)
        .with_type(CardType::Creature)
        .shares_subtype_with_tagged(tag.clone());
    let each_filter = ObjectFilter::default()
        .in_zone(Zone::Hand)
        .with_type(CardType::Creature)
        .shares_subtype_with_each_tagged(tag);

    assert!(
        each_filter.matches(game.object(elf_wizard_id).unwrap(), &ctx, &game),
        "a candidate may share a different subtype with each tapped creature"
    );
    assert!(
        any_filter.matches(game.object(elf_only_id).unwrap(), &ctx, &game),
        "the legacy relation should still match the union of tagged subtypes"
    );
    assert!(
        !each_filter.matches(game.object(elf_only_id).unwrap(), &ctx, &game),
        "sharing a subtype with only one tagged creature is insufficient"
    );
}

#[test]
fn characteristic_relations_compare_against_the_filtered_object_set() {
    use crate::card::CardBuilder;
    use crate::ids::CardId;

    let you = PlayerId::from_index(0);
    let opponent = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["You".to_string(), "Opponent".to_string()], 20);
    let your_human = CardBuilder::new(CardId::from_raw(40_200), "Your Human")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human])
        .build();
    let opposing_elf = CardBuilder::new(CardId::from_raw(40_201), "Opposing Elf")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elf])
        .build();
    let human_candidate = CardBuilder::new(CardId::from_raw(40_202), "Human Candidate")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human])
        .build();
    let elf_candidate = CardBuilder::new(CardId::from_raw(40_203), "Elf Candidate")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elf])
        .build();

    game.create_object_from_card(&your_human, you, Zone::Battlefield);
    game.create_object_from_card(&opposing_elf, opponent, Zone::Battlefield);
    let human_candidate = game.create_object_from_card(&human_candidate, you, Zone::Hand);
    let elf_candidate = game.create_object_from_card(&elf_candidate, you, Zone::Hand);
    let ctx = FilterContext::new(you);
    let characteristics = vec![ObjectCharacteristic::Subtype(SubtypeFamily::Creature)];
    let comparison = ObjectFilter::creature().you_control();
    let shares = ObjectFilter::default()
        .in_zone(Zone::Hand)
        .with_type(CardType::Creature)
        .sharing_characteristics_with(characteristics.clone(), comparison.clone());
    let shares_none = ObjectFilter::default()
        .in_zone(Zone::Hand)
        .with_type(CardType::Creature)
        .sharing_no_characteristics_with(characteristics, comparison);

    assert!(shares.matches(game.object(human_candidate).unwrap(), &ctx, &game));
    assert!(
        !shares.matches(game.object(elf_candidate).unwrap(), &ctx, &game),
        "an Elf controlled by another player must not satisfy the nested you-control filter"
    );
    assert!(!shares_none.matches(game.object(human_candidate).unwrap(), &ctx, &game));
    assert!(shares_none.matches(game.object(elf_candidate).unwrap(), &ctx, &game));
}

#[test]
fn negated_land_type_relation_uses_only_land_subtypes() {
    use crate::card::CardBuilder;
    use crate::ids::CardId;

    let you = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["You".to_string()], 20);
    let forest = CardBuilder::new(CardId::from_raw(40_210), "Forest")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Forest])
        .build();
    let forest_candidate = CardBuilder::new(CardId::from_raw(40_211), "Forest Candidate")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Forest])
        .build();
    let island_candidate = CardBuilder::new(CardId::from_raw(40_212), "Island Candidate")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Island])
        .build();

    game.create_object_from_card(&forest, you, Zone::Battlefield);
    let forest_candidate = game.create_object_from_card(&forest_candidate, you, Zone::Library);
    let island_candidate = game.create_object_from_card(&island_candidate, you, Zone::Library);
    let ctx = FilterContext::new(you);
    let filter = ObjectFilter::default()
        .in_zone(Zone::Library)
        .with_type(CardType::Land)
        .sharing_no_characteristics_with(
            vec![ObjectCharacteristic::Subtype(SubtypeFamily::Land)],
            ObjectFilter::land().you_control(),
        );

    assert!(!filter.matches(game.object(forest_candidate).unwrap(), &ctx, &game));
    assert!(filter.matches(game.object(island_candidate).unwrap(), &ctx, &game));
}

#[test]
fn characteristic_relation_treats_color_and_mana_value_as_alternatives() {
    use crate::card::CardBuilder;
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};

    let you = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["You".to_string()], 20);
    let comparison = CardBuilder::new(CardId::from_raw(40_220), "Red Three")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_symbols(vec![
            ManaSymbol::Generic(2),
            ManaSymbol::Red,
        ]))
        .build();
    let blue_three = CardBuilder::new(CardId::from_raw(40_221), "Blue Three")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_symbols(vec![
            ManaSymbol::Generic(2),
            ManaSymbol::Blue,
        ]))
        .build();
    let red_one = CardBuilder::new(CardId::from_raw(40_222), "Red One")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Red]))
        .build();
    let blue_one = CardBuilder::new(CardId::from_raw(40_223), "Blue One")
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue]))
        .build();

    game.create_object_from_card(&comparison, you, Zone::Battlefield);
    let blue_three = game.create_object_from_card(&blue_three, you, Zone::Hand);
    let red_one = game.create_object_from_card(&red_one, you, Zone::Hand);
    let blue_one = game.create_object_from_card(&blue_one, you, Zone::Hand);
    let ctx = FilterContext::new(you);
    let filter = ObjectFilter::default()
        .in_zone(Zone::Hand)
        .sharing_characteristics_with(
            vec![ObjectCharacteristic::Color, ObjectCharacteristic::ManaValue],
            ObjectFilter::creature().you_control(),
        );

    assert!(filter.matches(game.object(blue_three).unwrap(), &ctx, &game));
    assert!(filter.matches(game.object(red_one).unwrap(), &ctx, &game));
    assert!(!filter.matches(game.object(blue_one).unwrap(), &ctx, &game));
}

#[test]
fn test_base_power_builder_sets_reference() {
    let filter = ObjectFilter::creature().with_base_power(Comparison::LessThanOrEqual(2));
    assert_eq!(filter.power, Some(Comparison::LessThanOrEqual(2)));
    assert_eq!(filter.power_reference, PtReference::Base);
    assert_eq!(filter.description(), "creature with base power 2 or less");
}

#[test]
fn test_filter_description_places_controller_before_power_toughness_relation() {
    let filter = ObjectFilter::creature()
        .controlled_by(PlayerFilter::You)
        .with_power_toughness_relation(PowerToughnessRelation::ToughnessGreaterThanPower);

    assert_eq!(
        filter.description(),
        "a creature you control with toughness greater than its power"
    );
}

#[test]
fn test_filter_description_places_controller_before_static_ability_qualifier() {
    let filter = ObjectFilter::creature()
        .controlled_by(PlayerFilter::You)
        .with_static_ability(StaticAbilityId::Deathtouch);

    assert_eq!(
        filter.description(),
        "a creature you control with deathtouch"
    );
}

#[test]
fn test_filter_description_places_controller_before_mana_value_qualifier() {
    let filter = ObjectFilter::spell()
        .controlled_by(PlayerFilter::You)
        .with_mana_value(Comparison::LessThanOrEqual(2));

    assert_eq!(
        filter.description(),
        "a spell you control with mana value 2 or less"
    );
}

#[test]
fn test_filter_description_places_controller_before_name_qualifier() {
    let filter = ObjectFilter::creature()
        .controlled_by(PlayerFilter::You)
        .named("Example");

    assert_eq!(filter.description(), "a creature you control named Example");
}

#[test]
fn test_filter_can_match_base_vs_effective_power() {
    use crate::card::{CardBuilder, PowerToughness};
    use crate::game_state::GameState;
    use crate::ids::CardId;
    use crate::object::CounterType;

    let you = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["You".to_string()], 20);

    let card = CardBuilder::new(CardId::from_raw(30), "Counter Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let object_id = game.create_object_from_card(&card, you, Zone::Battlefield);
    if let Some(obj) = game.object_mut(object_id) {
        obj.counters.insert(CounterType::PlusOnePlusOne, 1);
    }

    let obj = game.object(object_id).expect("object should exist");
    let ctx = FilterContext::new(you);

    let effective_filter = ObjectFilter::creature().with_power(Comparison::GreaterThanOrEqual(3));
    let base_filter = ObjectFilter::creature().with_base_power(Comparison::GreaterThanOrEqual(3));

    assert!(
        effective_filter.matches(obj, &ctx, &game),
        "effective power should include +1/+1 counters"
    );
    assert!(
        !base_filter.matches(obj, &ctx, &game),
        "base power should ignore +1/+1 counters"
    );
}

#[test]
fn test_non_recursive_match_avoids_calculated_power() {
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::game_state::GameState;
    use crate::ids::CardId;
    use crate::static_abilities::StaticAbility;

    let you = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["You".to_string()], 20);

    let card = CardBuilder::new(CardId::from_raw(31), "Anthem Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let object_id = game.create_object_from_card(&card, you, Zone::Battlefield);
    if let Some(obj) = game.object_mut(object_id) {
        obj.abilities_mut()
            .push(Ability::static_ability(StaticAbility::anthem(
                ObjectFilter::source(),
                2,
                0,
            )));
    }

    let obj = game.object(object_id).expect("object should exist");
    let ctx = FilterContext::new(you);
    let filter = ObjectFilter::creature().with_power(Comparison::GreaterThanOrEqual(4));

    assert!(
        filter.matches(obj, &ctx, &game),
        "regular matching should use calculated power"
    );
    assert!(
        !filter.matches_non_recursive(obj, &ctx, &game),
        "non-recursive matching should avoid layer-calculated power"
    );
}

#[test]
fn extremum_power_filter_matches_every_tied_maximum_and_only_the_minimum() {
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::Value;
    use crate::game_state::GameState;
    use crate::ids::CardId;

    let you = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["You".to_string()], 20);
    let make_creature = |id, name, power, toughness| {
        CardBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build()
    };
    let small = game.create_object_from_card(
        &make_creature(40_100, "Small", 2, 4),
        you,
        Zone::Battlefield,
    );
    let tied_max_a = game.create_object_from_card(
        &make_creature(40_101, "Tied Max A", 5, 3),
        you,
        Zone::Battlefield,
    );
    let tied_max_b = game.create_object_from_card(
        &make_creature(40_102, "Tied Max B", 5, 1),
        you,
        Zone::Battlefield,
    );
    let ctx = FilterContext::new(you);
    let scope = ObjectFilter::creature();
    let greatest = ObjectFilter::creature().with_power(Comparison::EqualExpr(Box::new(
        Value::GreatestPower(scope.clone()),
    )));
    let least = ObjectFilter::creature()
        .with_power(Comparison::EqualExpr(Box::new(Value::LeastPower(scope))));

    assert!(!greatest.matches(game.object(small).unwrap(), &ctx, &game));
    assert!(greatest.matches(game.object(tied_max_a).unwrap(), &ctx, &game));
    assert!(greatest.matches(game.object(tied_max_b).unwrap(), &ctx, &game));
    assert!(least.matches(game.object(small).unwrap(), &ctx, &game));
    assert!(!least.matches(game.object(tied_max_a).unwrap(), &ctx, &game));
    assert!(!least.matches(game.object(tied_max_b).unwrap(), &ctx, &game));
}

#[test]
fn tagged_extremum_mana_value_filter_uses_the_tagged_snapshot_set() {
    use crate::card::CardBuilder;
    use crate::effect::Value;
    use crate::game_state::GameState;
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};

    let you = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["You".to_string()], 20);
    let cheap_card = CardBuilder::new(CardId::from_raw(40_110), "Cheap Card")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
        .build();
    let expensive_card = CardBuilder::new(CardId::from_raw(40_111), "Expensive Card")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(5)]]))
        .build();
    let cheap_id = game.create_object_from_card(&cheap_card, you, Zone::Graveyard);
    let expensive_id = game.create_object_from_card(&expensive_card, you, Zone::Graveyard);
    let cheap = ObjectSnapshot::from_object(game.object(cheap_id).unwrap(), &game);
    let expensive = ObjectSnapshot::from_object(game.object(expensive_id).unwrap(), &game);
    let tag = TagKey::from("discarded_cards");
    let tagged_constraint = TaggedObjectConstraint {
        tag: tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    };
    let scope = ObjectFilter {
        tagged_constraints: vec![tagged_constraint.clone()],
        ..ObjectFilter::default()
    };
    let filter = ObjectFilter {
        mana_value: Some(Comparison::EqualExpr(Box::new(Value::GreatestManaValue(
            scope,
        )))),
        tagged_constraints: vec![tagged_constraint],
        ..ObjectFilter::default()
    };
    let ctx = FilterContext::new(you).with_tagged_objects(&std::collections::HashMap::from([(
        tag,
        vec![cheap.clone(), expensive.clone()],
    )]));

    assert!(!filter.matches_snapshot(&cheap, &ctx, &game));
    assert!(filter.matches_snapshot(&expensive, &ctx, &game));
}

#[test]
fn dynamic_comparison_resolves_surface_hinted_source_counter_count() {
    use crate::card::CardBuilder;
    use crate::effect::Value;
    use crate::game_state::GameState;
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::CounterType;
    use crate::target::{ChooseSpec, ChooseSpecSurfaceHint, SourceReferenceSurface};

    let you = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["You".to_string()], 20);
    let source = CardBuilder::new(CardId::from_raw(40_120), "Counter Source")
        .card_types(vec![CardType::Artifact])
        .build();
    let one_mana = CardBuilder::new(CardId::from_raw(40_121), "One-Mana Spell")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]))
        .build();
    let two_mana = CardBuilder::new(CardId::from_raw(40_122), "Two-Mana Spell")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(2)]))
        .build();
    let source_id = game.create_object_from_card(&source, you, Zone::Battlefield);
    let one_mana_id = game.create_object_from_card(&one_mana, you, Zone::Hand);
    let two_mana_id = game.create_object_from_card(&two_mana, you, Zone::Hand);
    game.add_counters(source_id, CounterType::Charge, 1)
        .expect("the source should receive a charge counter");

    let source_spec = ChooseSpec::Source.with_surface_hint(ChooseSpecSurfaceHint::SourceReference(
        SourceReferenceSurface::ThisPermanentType("this artifact".to_string()),
    ));
    let filter = ObjectFilter {
        mana_value: Some(Comparison::EqualExpr(Box::new(Value::CountersOn(
            Box::new(source_spec),
            Some(CounterType::Charge),
        )))),
        ..ObjectFilter::default()
    };
    let ctx = FilterContext::new(you).with_source(source_id);

    assert!(filter.matches(game.object(one_mana_id).unwrap(), &ctx, &game));
    assert!(!filter.matches(game.object(two_mana_id).unwrap(), &ctx, &game));
}

#[test]
fn dynamic_comparison_describes_player_counter_count() {
    let filter = ObjectFilter {
        mana_value: Some(Comparison::GreaterThanExpr(Box::new(
            crate::effect::Value::PlayerCounters(PlayerFilter::You, crate::CounterType::Experience),
        ))),
        ..ObjectFilter::default()
    };

    assert!(
        filter
            .description()
            .contains("mana value greater than the number of experience counters you have"),
        "{}",
        filter.description()
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn test_filter_matches_earthbent_land_as_creature() {
    use crate::card::{CardBuilder, PowerToughness};
    use crate::cards::definitions::basic_mountain;
    use crate::effect::Effect;
    use crate::effects::EarthbendEffect;
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::game_state::GameState;
    use crate::ids::CardId;
    use crate::target::ChooseSpec;

    let you = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["You".to_string()], 20);

    let source_card = CardBuilder::new(CardId::from_raw(32), "Earthbend Source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let source_id = game.create_object_from_card(&source_card, you, Zone::Battlefield);
    let land_id = game.create_object_from_definition(&basic_mountain(), you, Zone::Battlefield);

    let effect = Effect::new(EarthbendEffect::new(ChooseSpec::SpecificObject(land_id), 8));
    let mut exec_ctx = ExecutionContext::new_default(source_id, you);
    execute_effect(&mut game, &effect, &mut exec_ctx).expect("earthbend should resolve");

    let filter_ctx = FilterContext::new(you).with_source(source_id);
    let land = game.object(land_id).expect("earthbent land should exist");

    assert!(
        ObjectFilter::creature().matches(land, &filter_ctx, &game),
        "calculated creature type should make the animated land match creature filters"
    );
    assert!(
        !ObjectFilter::creature().matches_non_recursive(land, &filter_ctx, &game),
        "non-recursive matching should keep using base types for layer calculations"
    );
}

#[test]
fn test_filter_matches_creature_dealt_damage_this_turn() {
    use crate::card::{CardBuilder, PowerToughness};
    use crate::game_state::GameState;
    use crate::ids::CardId;

    let you = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["You".to_string()], 20);

    let card = CardBuilder::new(CardId::from_raw(40), "Damaged Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let creature_id = game.create_object_from_card(&card, you, Zone::Battlefield);
    let ctx = FilterContext::new(you);

    let mut filter = ObjectFilter::creature();
    filter.was_dealt_damage_this_turn = true;

    let creature = game.object(creature_id).expect("creature should exist");
    assert!(!filter.matches(creature, &ctx, &game));

    let damage_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::DamageEvent {
            source: ObjectId::from_raw(500),
            target: crate::events::DamageTarget::Object(creature_id),
            amount: 1,
            is_combat: false,
            is_unpreventable: false,
            cause: crate::events::cause::EventCause::effect(),
            remainder: None,
            target_snapshot: None,
        },
        crate::provenance::ProvNodeId::default(),
    );
    game.record_turn_history_event(&damage_event);
    let creature = game.object(creature_id).expect("creature should exist");
    assert!(filter.matches(creature, &ctx, &game));
}
