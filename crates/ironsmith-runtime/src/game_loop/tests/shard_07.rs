#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::shard_02::*;
use super::shard_03::*;
use super::shard_04::*;
use super::shard_05::*;
use super::shard_06::*;
use super::shard_08::*;
use super::shard_09::*;
use super::shard_10::*;
use super::shard_11::*;
use super::shard_12::*;
use super::shard_13::*;
use super::shard_14::*;
use super::shard_15::*;
use super::shard_16::*;
use super::shard_17::*;
use super::*;

#[cfg(ironsmith_runtime_parser_tests)]
struct SelectNamedObjectGroups {
    groups: std::collections::VecDeque<Vec<&'static str>>,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl SelectNamedObjectGroups {
    fn new(groups: impl IntoIterator<Item = Vec<&'static str>>) -> Self {
        Self {
            groups: groups.into_iter().collect(),
        }
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for SelectNamedObjectGroups {
    fn decide_objects(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        let Some(names) = self.groups.pop_front() else {
            return ctx
                .candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .take(ctx.min)
                .collect();
        };
        let selected = names
            .iter()
            .map(|name| {
                ctx.candidates
                    .iter()
                    .find(|candidate| {
                        candidate.legal
                            && game
                                .object(candidate.id)
                                .is_some_and(|object| object.name == *name)
                    })
                    .unwrap_or_else(|| panic!("missing legal looked-card choice {name}"))
                    .id
            })
            .collect::<Vec<_>>();
        assert!(
            selected.len() >= ctx.min && ctx.max.is_none_or(|max| selected.len() <= max),
            "scripted looked-card choice must satisfy the requested count"
        );
        selected
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
fn add_partition_library_card(
    game: &mut GameState,
    owner: PlayerId,
    id: u32,
    name: &str,
) -> ObjectId {
    game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![CardType::Artifact])
            .build(),
        owner,
        Zone::Library,
    )
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn dark_bargain_moves_two_chosen_looked_cards_and_only_the_remainder_to_graveyard() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let def = CardDefinitionBuilder::new(CardId::from_raw(82_050), "Dark Bargain")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Look at the top three cards of your library. Put two of them into your hand and the other into your graveyard. Dark Bargain deals 2 damage to you.",
        )
        .expect("Dark Bargain should parse");

    add_partition_library_card(&mut game, alice, 82_051, "Unseen Sentinel");
    add_partition_library_card(&mut game, alice, 82_052, "Dark Remainder");
    add_partition_library_card(&mut game, alice, 82_053, "Dark Hand A");
    add_partition_library_card(&mut game, alice, 82_054, "Dark Hand B");

    let spell_id = game.create_object_from_definition(&def, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell_id, alice));
    let mut decisions = SelectNamedObjectGroups::new([vec!["Dark Hand A", "Dark Hand B"]]);
    resolve_stack_entry_with(&mut game, &mut decisions).expect("Dark Bargain should resolve");

    let player = game.player(alice).expect("Alice should exist");
    let hand_names = player
        .hand
        .iter()
        .filter_map(|id| game.object(*id).map(|object| object.name.to_string()))
        .collect::<Vec<_>>();
    assert_eq!(hand_names.len(), 2);
    assert!(hand_names.contains(&"Dark Hand A".to_string()));
    assert!(hand_names.contains(&"Dark Hand B".to_string()));
    let graveyard_names = player
        .graveyard
        .iter()
        .filter_map(|id| game.object(*id).map(|object| object.name.to_string()))
        .collect::<Vec<_>>();
    assert!(graveyard_names.contains(&"Dark Remainder".to_string()));
    assert!(!graveyard_names.contains(&"Dark Hand A".to_string()));
    assert!(!graveyard_names.contains(&"Dark Hand B".to_string()));
    assert_eq!(
        player.library.len(),
        1,
        "only the unseen card should remain"
    );
    assert_eq!(player.life, 18, "Dark Bargain should still deal its damage");
    assert!(
        decisions.groups.is_empty(),
        "all scripted choices should be used"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn telling_time_preserves_independent_hand_top_and_bottom_choices() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let def = CardDefinitionBuilder::new(CardId::from_raw(82_060), "Telling Time")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Look at the top three cards of your library. Put one of those cards into your hand, one on top of your library, and one on the bottom of your library.",
        )
        .expect("Telling Time should parse");

    add_partition_library_card(&mut game, alice, 82_061, "Unseen Middle Sentinel");
    add_partition_library_card(&mut game, alice, 82_062, "Chosen Bottom");
    add_partition_library_card(&mut game, alice, 82_063, "Chosen Top");
    add_partition_library_card(&mut game, alice, 82_064, "Chosen Hand");

    let spell_id = game.create_object_from_definition(&def, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell_id, alice));
    let mut decisions = SelectNamedObjectGroups::new([vec!["Chosen Hand"], vec!["Chosen Top"]]);
    resolve_stack_entry_with(&mut game, &mut decisions).expect("Telling Time should resolve");

    let player = game.player(alice).expect("Alice should exist");
    let hand_names = player
        .hand
        .iter()
        .filter_map(|id| game.object(*id).map(|object| object.name.to_string()))
        .collect::<Vec<_>>();
    assert_eq!(hand_names, vec!["Chosen Hand".to_string()]);
    let library_names = player
        .library
        .iter()
        .filter_map(|id| game.object(*id).map(|object| object.name.to_string()))
        .collect::<Vec<_>>();
    assert_eq!(
        library_names,
        vec![
            "Chosen Bottom".to_string(),
            "Unseen Middle Sentinel".to_string(),
            "Chosen Top".to_string(),
        ],
        "library vectors are bottom-to-top, so the two chosen placements must remain independent"
    );
    assert!(
        decisions.groups.is_empty(),
        "the hand and top choices should be used; the last looked-at card goes to the bottom deterministically"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn illicit_auction_high_bidder_can_bid_more_life_than_they_have_and_gains_control_indefinitely()
 {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let creature_id = create_creature(&mut game, "Auctioned Bear", alice, 2, 2);

    put_illicit_auction_on_stack(&mut game, alice, creature_id);
    let mut bids = ScriptedLifeBids { bids: vec![25, 0] };
    resolve_stack_entry_with(&mut game, &mut bids).expect("Illicit Auction should resolve");
    game.refresh_continuous_state();

    assert_eq!(
        game.player(bob).expect("Bob should exist").life,
        -5,
        "high bidder can bid more life than they have, then loses life equal to the high bid"
    );
    assert_eq!(
        game.current_controller(creature_id),
        Some(bob),
        "high bidder should gain control of the target creature"
    );

    crate::turn::execute_cleanup_step(&mut game);
    game.refresh_continuous_state();
    assert_eq!(
        game.current_controller(creature_id),
        Some(bob),
        "Illicit Auction's control effect should last indefinitely"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn illicit_auction_zero_bid_stands_for_controller_when_no_player_tops_it() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let creature_id = create_creature(&mut game, "Unbid Bear", bob, 2, 2);

    put_illicit_auction_on_stack(&mut game, alice, creature_id);
    let mut bids = ScriptedLifeBids { bids: vec![0] };
    resolve_stack_entry_with(&mut game, &mut bids).expect("Illicit Auction should resolve");
    game.refresh_continuous_state();

    assert_eq!(
        game.player(alice).expect("Alice should exist").life,
        20,
        "the standing zero bid should not make the controller lose life"
    );
    assert_eq!(
        game.current_controller(creature_id),
        Some(alice),
        "the initial high bidder should gain control when no one tops the zero bid"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn illicit_auction_bidding_uses_turn_order_until_high_bid_stands() {
    let mut game = setup_three_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    game.turn_store.turn_order = vec![bob, alice, charlie];
    let creature_id = create_creature(&mut game, "Turn Order Bear", alice, 2, 2);

    put_illicit_auction_on_stack(&mut game, bob, creature_id);
    let mut bids = RecordingLifeBids::new(vec![
        (alice, Some(3)),
        (charlie, None),
        (bob, Some(5)),
        (alice, None),
        (charlie, None),
    ]);
    resolve_stack_entry_with(&mut game, &mut bids).expect("Illicit Auction should resolve");
    game.refresh_continuous_state();

    assert_eq!(
        bids.prompted_players,
        vec![alice, charlie, bob, alice, charlie],
        "bidding should follow turn order from the spell controller through multiple rounds"
    );
    assert!(
        bids.responses.is_empty(),
        "the high bid should stand only after every other player declines to top it"
    );
    assert_eq!(
        game.player(bob).expect("Bob should exist").life,
        15,
        "the final high bidder should lose life equal to the final high bid"
    );
    assert_eq!(
        game.current_controller(creature_id),
        Some(bob),
        "the final high bidder should gain control of the target creature"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn beamsplitter_mage_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(82_200), "Beamsplitter Mage")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Vedalken, crate::types::Subtype::Wizard])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "Whenever you cast an instant or sorcery spell that targets only this creature, if you control one or more other creatures that spell could target, choose one of those creatures. Copy that spell. The copy targets the chosen creature.",
        )
        .expect("Beamsplitter Mage should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn zada_hedron_grinder_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(82_202), "Zada, Hedron Grinder")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Goblin, crate::types::Subtype::Ally])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text(
            "Whenever you cast an instant or sorcery spell that targets only Zada, copy that spell for each other creature you control that the spell could target. Each copy targets a different one of those creatures.",
        )
        .expect("Zada, Hedron Grinder should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn feather_radiant_arbiter_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(82_203), "Feather, Radiant Arbiter")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Red],
            vec![ManaSymbol::White],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Angel])
        .power_toughness(PowerToughness::fixed(4, 3))
        .parse_text(
            "Flying, lifelink\nWhenever you cast a noncreature spell that targets only Feather, you may choose any number of other creatures that spell could target and pay {2} for each of those creatures. If you do, for each of those creatures, copy that spell. The copy targets that creature.",
        )
        .expect("Feather, Radiant Arbiter should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn nonartifact_creature_spell_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(82_201), "Friendly Calibration")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Instant])
        .parse_text("Target nonartifact creature you control.")
        .expect("single-target spell should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn any_nonartifact_creature_spell_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(82_204), "Open Calibration")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Instant])
        .parse_text("Target nonartifact creature.")
        .expect("single-target spell should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn artifact_creature_card(name: &str) -> crate::card::Card {
    CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn stack_beamsplitter_probe_spell(
    game: &mut GameState,
    controller: PlayerId,
    target: ObjectId,
) -> ObjectId {
    let spell_def = nonartifact_creature_spell_definition();
    let spell_id = game.create_object_from_definition(&spell_def, controller, Zone::Stack);
    game.push_to_stack(
        StackEntry::new(spell_id, controller).with_targets(vec![Target::Object(target)]),
    );
    spell_id
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn stack_any_nonartifact_creature_probe_spell(
    game: &mut GameState,
    controller: PlayerId,
    target: ObjectId,
) -> ObjectId {
    let spell_def = any_nonartifact_creature_spell_definition();
    let spell_id = game.create_object_from_definition(&spell_def, controller, Zone::Stack);
    game.push_to_stack(
        StackEntry::new(spell_id, controller).with_targets(vec![Target::Object(target)]),
    );
    spell_id
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn spell_cast_event_for_stack_object(
    game: &GameState,
    spell_id: ObjectId,
    caster: PlayerId,
) -> TriggerEvent {
    let snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(spell_id)
            .expect("spell object should exist on the stack"),
        game,
    );
    TriggerEvent::new_with_provenance(
        SpellCastEvent::new_with_snapshot(spell_id, caster, Zone::Hand, snapshot),
        crate::provenance::ProvNodeId::default(),
    )
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct ChooseSpecificObjectDecisionMaker {
    pub(super) desired: ObjectId,
    pub(super) seen_candidates: Vec<ObjectId>,
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct ChooseObjectByOnlyLegalSetDecisionMaker {
    pub(super) allowed: Vec<ObjectId>,
    pub(super) chosen: Vec<ObjectId>,
    pub(super) seen_candidate_sets: Vec<Vec<ObjectId>>,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl ChooseSpecificObjectDecisionMaker {
    pub(super) fn new(desired: ObjectId) -> Self {
        Self {
            desired,
            seen_candidates: Vec::new(),
        }
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
impl ChooseObjectByOnlyLegalSetDecisionMaker {
    pub(super) fn new(allowed: Vec<ObjectId>) -> Self {
        Self {
            allowed,
            chosen: Vec::new(),
            seen_candidate_sets: Vec::new(),
        }
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for ChooseSpecificObjectDecisionMaker {
    fn decide_targets(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        self.seen_candidates = ctx
            .requirements
            .iter()
            .flat_map(|requirement| requirement.legal_targets.iter())
            .filter_map(|target| match target {
                Target::Object(id) => Some(*id),
                Target::Player(_) => None,
            })
            .collect();
        assert!(
            ctx.requirements.iter().any(|requirement| requirement
                .legal_targets
                .contains(&Target::Object(self.desired))),
            "expected desired object target to be legal, got {:?}",
            ctx.requirements
        );
        vec![Target::Object(self.desired)]
    }

    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        self.seen_candidates = ctx
            .candidates
            .iter()
            .map(|candidate| candidate.id)
            .collect();
        assert!(
            ctx.candidates
                .iter()
                .any(|candidate| candidate.id == self.desired && candidate.legal),
            "expected desired Beamsplitter retarget candidate to be legal, got {:?}",
            ctx.candidates
        );
        vec![self.desired]
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for ChooseObjectByOnlyLegalSetDecisionMaker {
    fn decide_targets(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        let candidates: Vec<ObjectId> = ctx
            .requirements
            .iter()
            .flat_map(|requirement| requirement.legal_targets.iter())
            .filter_map(|target| match target {
                Target::Object(id) => Some(*id),
                Target::Player(_) => None,
            })
            .collect();
        self.seen_candidate_sets.push(candidates.clone());
        let chosen = candidates
            .iter()
            .copied()
            .find(|candidate| self.allowed.contains(candidate))
            .expect("expected one allowed object target to be legal");
        self.chosen.push(chosen);
        vec![Target::Object(chosen)]
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn beamsplitter_mage_copies_spell_and_retargets_copy_to_chosen_creature() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let beamsplitter = beamsplitter_mage_definition();
    let mage_id = game.create_object_from_definition(&beamsplitter, alice, Zone::Battlefield);
    let legal_ally = create_creature(&mut game, "Legal Ally", alice, 2, 2);
    let legal_ally_two = create_creature(&mut game, "Second Legal Ally", alice, 2, 2);
    let illegal_artifact = game.create_object_from_card(
        &artifact_creature_card("Alloy Ally"),
        alice,
        Zone::Battlefield,
    );
    let bob_creature = create_creature(&mut game, "Bob Creature", bob, 2, 2);

    let spell_id = stack_beamsplitter_probe_spell(&mut game, alice, mage_id);
    let triggering_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(spell_id)
            .expect("triggering spell should exist on the stack"),
        &game,
    );
    let mut filter_ctx = game.filter_context_for(alice, Some(mage_id));
    filter_ctx
        .tagged_objects
        .insert(crate::TagKey::from("triggering"), vec![triggering_snapshot]);
    let targetable_other_creature = crate::target::ObjectFilter::creature()
        .controlled_by(PlayerFilter::You)
        .could_be_targeted_by(ObjectRef::tagged("triggering"));
    assert!(
        game.object(legal_ally)
            .is_some_and(|obj| { targetable_other_creature.matches(obj, &filter_ctx, &game) }),
        "targetability primitive should include the legal ally"
    );
    assert!(
        game.object(illegal_artifact)
            .is_some_and(|obj| { !targetable_other_creature.matches(obj, &filter_ctx, &game) }),
        "targetability primitive should exclude same-controller artifact creatures"
    );
    assert!(
        game.object(bob_creature)
            .is_some_and(|obj| { !targetable_other_creature.matches(obj, &filter_ctx, &game) }),
        "targetability primitive should exclude opponent creatures for this spell"
    );

    let event = spell_cast_event_for_stack_object(&game, spell_id, alice);
    let triggers = check_triggers(&game, &event);
    assert_eq!(
        triggers.len(),
        1,
        "Beamsplitter should trigger for an instant targeting only itself"
    );

    let mut trigger_queue = TriggerQueue::new();
    for trigger in triggers {
        trigger_queue.add(trigger);
    }
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Beamsplitter trigger should go on the stack");
    let trigger_entry = game
        .stack
        .last()
        .expect("trigger should be the top stack entry");
    let intervening_if = trigger_entry
        .intervening_if
        .as_ref()
        .expect("Beamsplitter trigger should carry its intervening-if condition");
    assert!(
        crate::triggers::verify_intervening_if(
            &game,
            intervening_if,
            trigger_entry.controller,
            trigger_entry
                .triggering_event
                .as_ref()
                .expect("trigger entry should retain the spell-cast event"),
            trigger_entry.object_id,
            None,
            None,
        ),
        "resolution-time intervening-if should still see another legal targetable creature"
    );

    let mut dm = ChooseSpecificObjectDecisionMaker::new(legal_ally);
    resolve_stack_entry_with(&mut game, &mut dm).expect("Beamsplitter trigger should resolve");
    assert_eq!(
        game.stack.len(),
        2,
        "resolving the trigger should leave the original spell plus a copy on the stack, got {:?}",
        game.stack
    );

    assert!(
        dm.seen_candidates.contains(&legal_ally),
        "the legal ally should be offered as the chosen creature"
    );
    assert!(
        dm.seen_candidates.contains(&legal_ally_two),
        "the second legal ally should also be offered as a targetable creature"
    );
    assert!(
        !dm.seen_candidates.contains(&illegal_artifact),
        "same-controller artifact creature should be excluded by spell targetability"
    );
    assert!(
        !dm.seen_candidates.contains(&bob_creature),
        "opponent's creature should be excluded by spell targetability"
    );

    let original_entry = game
        .stack
        .iter()
        .find(|entry| entry.object_id == spell_id)
        .expect("original spell should remain on the stack");
    assert_eq!(
        original_entry.targets,
        vec![Target::Object(mage_id)],
        "the original spell should still target Beamsplitter"
    );

    let copy_entry = game
        .stack
        .iter()
        .find(|entry| entry.object_id != spell_id)
        .expect("copy should be on the stack");
    assert_eq!(
        copy_entry.targets,
        vec![Target::Object(legal_ally)],
        "the copy should target the chosen legal creature"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn beamsplitter_mage_does_not_trigger_without_another_legal_creature() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let beamsplitter = beamsplitter_mage_definition();
    let mage_id = game.create_object_from_definition(&beamsplitter, alice, Zone::Battlefield);
    create_creature(&mut game, "Bob Creature", bob, 2, 2);

    let spell_id = stack_beamsplitter_probe_spell(&mut game, alice, mage_id);
    let event = spell_cast_event_for_stack_object(&game, spell_id, alice);
    let triggers = check_triggers(&game, &event);

    assert!(
        triggers.is_empty(),
        "Beamsplitter should not trigger when no other creature could be targeted by the spell"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn zada_copies_spell_for_each_other_legal_creature_with_distinct_targets() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let zada = zada_hedron_grinder_definition();
    let zada_id = game.create_object_from_definition(&zada, alice, Zone::Battlefield);
    let legal_ally = create_creature(&mut game, "Legal Ally", alice, 2, 2);
    let second_ally = create_creature(&mut game, "Second Legal Ally", alice, 2, 2);
    let artifact_ally = game.create_object_from_card(
        &artifact_creature_card("Alloy Ally"),
        alice,
        Zone::Battlefield,
    );
    let bob_creature = create_creature(&mut game, "Bob Creature", bob, 2, 2);

    let spell_id = stack_beamsplitter_probe_spell(&mut game, alice, zada_id);
    let event = spell_cast_event_for_stack_object(&game, spell_id, alice);
    let triggers = check_triggers(&game, &event);
    assert_eq!(
        triggers.len(),
        1,
        "Zada should trigger for an instant targeting only itself"
    );

    let mut trigger_queue = TriggerQueue::new();
    for trigger in triggers {
        trigger_queue.add(trigger);
    }
    put_triggers_on_stack(&mut game, &mut trigger_queue).expect("Zada trigger should stack");

    let mut dm = SelectFirstDecisionMaker;
    resolve_stack_entry_with(&mut game, &mut dm).expect("Zada trigger should resolve");
    assert_eq!(
        game.stack.len(),
        3,
        "Zada trigger should leave the original spell plus one copy per legal other creature"
    );

    let original_entry = game
        .stack
        .iter()
        .find(|entry| entry.object_id == spell_id)
        .expect("original spell should remain on the stack");
    assert_eq!(original_entry.targets, vec![Target::Object(zada_id)]);

    let copy_targets: std::collections::HashSet<Target> = game
        .stack
        .iter()
        .filter(|entry| entry.object_id != spell_id)
        .map(|entry| entry.targets[0])
        .collect();
    assert_eq!(
        copy_targets,
        std::collections::HashSet::from([Target::Object(legal_ally), Target::Object(second_ally)])
    );
    assert!(!copy_targets.contains(&Target::Object(zada_id)));
    assert!(!copy_targets.contains(&Target::Object(artifact_ally)));
    assert!(!copy_targets.contains(&Target::Object(bob_creature)));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[derive(Default)]
pub(super) struct ChooseAllDecisionMaker {
    pub(super) object_choice_candidates: Vec<Vec<ObjectId>>,
    pub(super) object_choices: Vec<Vec<ObjectId>>,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for ChooseAllDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        true
    }

    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        self.object_choice_candidates.push(
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .collect(),
        );
        let mut legal: Vec<ObjectId> = ctx
            .candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .collect();
        if let Some(max) = ctx.max {
            legal.truncate(max);
        }
        self.object_choices.push(legal.clone());
        legal
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn feather_radiant_arbiter_copies_spell_for_each_paid_chosen_creature() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let feather = feather_radiant_arbiter_definition();
    let feather_id = game.create_object_from_definition(&feather, alice, Zone::Battlefield);
    let legal_ally = create_creature(&mut game, "Legal Ally", alice, 2, 2);
    let bob_creature = create_creature(&mut game, "Bob Creature", bob, 2, 2);
    let artifact_ally = game.create_object_from_card(
        &artifact_creature_card("Alloy Ally"),
        alice,
        Zone::Battlefield,
    );
    game.player_mut(alice)
        .expect("alice should exist")
        .mana_pool
        .add(ManaSymbol::Colorless, 4);

    let spell_id = stack_any_nonartifact_creature_probe_spell(&mut game, alice, feather_id);
    let event = spell_cast_event_for_stack_object(&game, spell_id, alice);
    let triggers = check_triggers(&game, &event);
    assert_eq!(
        triggers.len(),
        1,
        "Feather should trigger for a noncreature spell targeting only itself"
    );

    let mut trigger_queue = TriggerQueue::new();
    for trigger in triggers {
        trigger_queue.add(trigger);
    }
    put_triggers_on_stack(&mut game, &mut trigger_queue).expect("Feather trigger should stack");

    let mut dm = ChooseAllDecisionMaker::default();
    resolve_stack_entry_with(&mut game, &mut dm).expect("Feather trigger should resolve");
    assert!(
        dm.object_choice_candidates
            .iter()
            .any(|candidates| candidates.contains(&legal_ally)
                && candidates.contains(&bob_creature)
                && !candidates.contains(&feather_id)
                && !candidates.contains(&artifact_ally)),
        "Feather should offer the other legal nonartifact-creature targets somewhere in its choices, got {:?}",
        dm.object_choice_candidates
    );
    assert!(
        dm.object_choices
            .iter()
            .any(|choices| choices.contains(&legal_ally) && choices.contains(&bob_creature)),
        "Feather choose-all helper should select both legal choices, got {:?}",
        dm.object_choices
    );
    assert_eq!(
        game.stack.len(),
        3,
        "Feather should leave the original spell plus one copy per paid chosen creature"
    );
    assert_eq!(
        game.player(alice)
            .expect("alice should exist")
            .mana_pool
            .total(),
        0,
        "Feather should pay {{2}} once for each chosen creature"
    );

    let original_entry = game
        .stack
        .iter()
        .find(|entry| entry.object_id == spell_id)
        .expect("original spell should remain on the stack");
    assert_eq!(original_entry.targets, vec![Target::Object(feather_id)]);

    let copy_targets: std::collections::HashSet<Target> = game
        .stack
        .iter()
        .filter(|entry| entry.object_id != spell_id)
        .map(|entry| entry.targets[0])
        .collect();
    assert_eq!(
        copy_targets,
        std::collections::HashSet::from([Target::Object(legal_ally), Target::Object(bob_creature)])
    );
    assert!(!copy_targets.contains(&Target::Object(feather_id)));
    assert!(!copy_targets.contains(&Target::Object(artifact_ally)));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn feather_radiant_arbiter_creates_no_copy_when_no_other_legal_creature_exists() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let feather = feather_radiant_arbiter_definition();
    let feather_id = game.create_object_from_definition(&feather, alice, Zone::Battlefield);
    let artifact_ally = game.create_object_from_card(
        &artifact_creature_card("Alloy Ally"),
        alice,
        Zone::Battlefield,
    );
    game.player_mut(alice)
        .expect("alice should exist")
        .mana_pool
        .add(ManaSymbol::Colorless, 4);

    let spell_id = stack_any_nonartifact_creature_probe_spell(&mut game, alice, feather_id);
    let event = spell_cast_event_for_stack_object(&game, spell_id, alice);
    let triggers = check_triggers(&game, &event);
    assert_eq!(
        triggers.len(),
        1,
        "Feather should still trigger before discovering there are no other legal choices"
    );

    let mut trigger_queue = TriggerQueue::new();
    for trigger in triggers {
        trigger_queue.add(trigger);
    }
    put_triggers_on_stack(&mut game, &mut trigger_queue).expect("Feather trigger should stack");

    let mut dm = ChooseAllDecisionMaker::default();
    resolve_stack_entry_with(&mut game, &mut dm).expect("Feather trigger should resolve");
    assert_eq!(
        game.stack.len(),
        1,
        "Feather should not create a copy when there are no other legal targets"
    );
    assert_eq!(
        game.player(alice)
            .expect("alice should exist")
            .mana_pool
            .total(),
        4,
        "Feather should not pay mana when no objects were chosen"
    );

    let original_entry = game
        .stack
        .iter()
        .find(|entry| entry.object_id == spell_id)
        .expect("original spell should remain on the stack");
    assert_eq!(original_entry.targets, vec![Target::Object(feather_id)]);
    assert!(!game.stack.iter().any(|entry| {
        entry.object_id != spell_id && entry.targets.contains(&Target::Object(artifact_ally))
    }));
}

pub(super) fn record_battlefield_entry_this_turn(game: &mut GameState, object_id: ObjectId) {
    use crate::events::cause::EventCause;
    use crate::events::zones::ZoneChangeEvent;
    use crate::provenance::ProvNodeId;
    use crate::snapshot::ObjectSnapshot;
    use crate::triggers::TriggerEvent;

    let snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(object_id)
            .expect("battlefield entry event object should exist"),
        game,
    );
    let event = TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_cause(
            object_id,
            Zone::Hand,
            Zone::Battlefield,
            EventCause::effect(),
            Some(snapshot),
        ),
        ProvNodeId::default(),
    );
    game.record_turn_history_event(&event);
}

pub(super) fn setup_goddric_fixture() -> (GameState, PlayerId, PlayerId, ObjectId) {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let goddric = CardDefinitionBuilder::new(CardId::new(), "Goddric, Cloaked Reveler")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![
            crate::types::Subtype::Human,
            crate::types::Subtype::Noble,
        ])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text(
            "Haste\nCelebration — As long as two or more nonland permanents entered the battlefield under your control this turn, this creature is a Dragon with base power and toughness 4/4, flying, and \"{R}: Dragons you control get +1/+0 until end of turn.\" (It loses all other creature types.)",
        )
        .expect("Goddric, Cloaked Reveler should compile");
    let goddric_id = game.create_object_from_definition(&goddric, alice, Zone::Battlefield);

    (game, alice, bob, goddric_id)
}

pub(super) fn create_delayed_reanimator(
    game: &mut GameState,
    owner: PlayerId,
    name: &str,
) -> ObjectId {
    let card = CardBuilder::new(CardId::from_raw(9010), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let id = game.create_object_from_card(&card, owner, Zone::Battlefield);
    if let Some(obj) = game.object_mut(id) {
        obj.abilities_mut().push(Ability::triggered(
            Trigger::this_dies(),
            vec![
                Effect::tag_triggering_object("triggering"),
                Effect::new(crate::effects::ScheduleDelayedTriggerEffect::new(
                    Trigger::beginning_of_end_step(crate::target::PlayerFilter::Any),
                    vec![Effect::return_from_graveyard_to_battlefield(
                        ChooseSpec::Tagged("triggering".into()),
                        false,
                    )],
                    true,
                    Vec::new(),
                    crate::target::PlayerFilter::You,
                )),
            ],
        ));
    }
    id
}

pub(super) fn undying_effects() -> Vec<Effect> {
    let trigger_tag = "undying_trigger";
    let return_tag = "undying_return";
    let returned_tag = "undying_returned";

    let filter = ObjectFilter::default()
        .in_zone(Zone::Graveyard)
        .same_stable_id_as_tagged(trigger_tag);

    let choose = Effect::choose_objects(filter, 1, crate::target::PlayerFilter::You, return_tag);
    let move_to_battlefield = Effect::move_to_zone(
        ChooseSpec::Tagged(return_tag.into()),
        Zone::Battlefield,
        true,
    )
    .tag(returned_tag);
    let counters = Effect::for_each_tagged(
        returned_tag,
        vec![Effect::put_counters(
            CounterType::PlusOnePlusOne,
            1,
            ChooseSpec::Iterated,
        )],
    );

    vec![
        Effect::tag_triggering_object(trigger_tag),
        choose,
        move_to_battlefield,
        counters,
    ]
}

#[test]
#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn test_goddric_celebration_inactive_without_two_nonland_entries() {
    let (mut game, alice, _bob, goddric_id) = setup_goddric_fixture();

    game.refresh_continuous_state();

    assert_eq!(
        game.calculated_power(goddric_id),
        Some(3),
        "Goddric should keep printed power before celebration is active"
    );
    assert_eq!(
        game.calculated_toughness(goddric_id),
        Some(3),
        "Goddric should keep printed toughness before celebration is active"
    );
    assert!(
        game.current_has_subtype(goddric_id, crate::types::Subtype::Human),
        "Goddric should still be Human before celebration is active"
    );
    assert!(
        game.current_has_subtype(goddric_id, crate::types::Subtype::Noble),
        "Goddric should still be Noble before celebration is active"
    );
    assert!(
        !game.current_has_subtype(goddric_id, crate::types::Subtype::Dragon),
        "Goddric should not be Dragon before celebration is active"
    );
    assert!(
        !game.object_has_static_ability_id(
            goddric_id,
            crate::static_abilities::StaticAbilityId::Flying
        ),
        "Goddric should not have flying before celebration is active"
    );
    assert!(
        !crate::decision::compute_legal_actions(&game, alice)
            .iter()
            .any(|action| matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, .. } if *source == goddric_id
            )),
        "Goddric should not expose the granted red activation before celebration is active"
    );
}

#[test]
#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn test_goddric_celebration_grants_dragon_stats_flying_and_activation() {
    let (mut game, alice, _bob, goddric_id) = setup_goddric_fixture();

    let celebrant_one = CardBuilder::new(CardId::new(), "Celebration Relic")
        .card_types(vec![CardType::Artifact])
        .build();
    let celebrant_two = CardBuilder::new(CardId::new(), "Celebration Familiar")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Construct])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let celebrant_one_id = game.create_object_from_card(&celebrant_one, alice, Zone::Battlefield);
    let celebrant_two_id = game.create_object_from_card(&celebrant_two, alice, Zone::Battlefield);
    record_battlefield_entry_this_turn(&mut game, celebrant_one_id);
    record_battlefield_entry_this_turn(&mut game, celebrant_two_id);
    game.refresh_continuous_state();

    let subtypes = game.calculated_subtypes(goddric_id);
    assert_eq!(
        game.calculated_power(goddric_id),
        Some(4),
        "Goddric should become a 4-power Dragon once celebration is active"
    );
    assert_eq!(
        game.calculated_toughness(goddric_id),
        Some(4),
        "Goddric should become a 4-toughness Dragon once celebration is active"
    );
    assert!(
        subtypes.contains(&crate::types::Subtype::Dragon),
        "Goddric should gain Dragon subtype once celebration is active: {subtypes:?}"
    );
    assert!(
        !subtypes.contains(&crate::types::Subtype::Human)
            && !subtypes.contains(&crate::types::Subtype::Noble),
        "Goddric should lose its other creature types once celebration is active: {subtypes:?}"
    );
    assert!(
        game.object_has_static_ability_id(
            goddric_id,
            crate::static_abilities::StaticAbilityId::Flying
        ),
        "Goddric should have flying once celebration is active"
    );
    assert!(
        crate::decision::compute_legal_actions(&game, alice)
            .iter()
            .any(|action| matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, .. } if *source == goddric_id
            )),
        "Goddric should expose the granted red activation once celebration is active"
    );
}

#[test]
#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn test_celebration_nonland_count_uses_current_type_effects_for_current_permanents() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let ashaya = CardDefinitionBuilder::new(CardId::new(), "Ashaya, Soul of the Wild")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::new(PtValue::Star, PtValue::Star))
        .parse_text(
            "Ashaya's power and toughness are each equal to the number of lands you control.\nNontoken creatures you control are Forest lands in addition to their other types.",
        )
        .expect("Ashaya should compile");
    let ashaya_id = game.create_object_from_definition(&ashaya, alice, Zone::Battlefield);
    record_battlefield_entry_this_turn(&mut game, ashaya_id);

    let lotus = CardDefinitionBuilder::new(CardId::new(), "Black Lotus")
        .card_types(vec![CardType::Artifact])
        .parse_text("{T}, Sacrifice this artifact: Add three mana of any one color.")
        .expect("Black Lotus should compile");
    let lotus_id = game.create_object_from_definition(&lotus, alice, Zone::Graveyard);
    record_battlefield_entry_this_turn(&mut game, lotus_id);

    let mice = CardDefinitionBuilder::new(CardId::new(), "Armory Mice")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Mouse])
        .power_toughness(PowerToughness::fixed(3, 1))
        .parse_text(
            "Celebration — This creature gets +0/+2 as long as two or more nonland permanents entered the battlefield under your control this turn.",
        )
        .expect("Armory Mice should compile");
    let mice_id = game.create_object_from_definition(&mice, alice, Zone::Battlefield);
    record_battlefield_entry_this_turn(&mut game, mice_id);
    game.refresh_continuous_state();

    assert!(
        game.object_has_card_type(ashaya_id, CardType::Land),
        "Ashaya should count its own type-changing effect and become a land"
    );
    assert_eq!(
        game.calculated_toughness(ashaya_id),
        Some(2),
        "Ashaya's characteristic-defining toughness should count lands after layer-4 effects"
    );
    crate::rules::state_based::apply_state_based_actions(&mut game);
    assert!(
        game.object(ashaya_id)
            .is_some_and(|object| object.zone == Zone::Battlefield),
        "Ashaya should survive state-based actions once it counts itself as a land"
    );
    assert!(
        game.object_has_card_type(mice_id, CardType::Land),
        "Ashaya should make Armory Mice a land"
    );
    assert_eq!(
        game.calculated_toughness(mice_id),
        Some(1),
        "Armory Mice should not count itself as a nonland permanent while Ashaya makes it a land"
    );
}

#[test]
#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn test_goddric_celebration_granted_ability_buffs_only_dragons() {
    use crate::decision::LegalAction;

    let (mut game, alice, _bob, goddric_id) = setup_goddric_fixture();

    let dragon_ally = CardBuilder::new(CardId::new(), "Dragon Ally")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Dragon])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let human_ally = CardBuilder::new(CardId::new(), "Human Ally")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Human])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let celebration_artifact = CardBuilder::new(CardId::new(), "Celebration Bauble")
        .card_types(vec![CardType::Artifact])
        .build();

    let dragon_ally_id = game.create_object_from_card(&dragon_ally, alice, Zone::Battlefield);
    let human_ally_id = game.create_object_from_card(&human_ally, alice, Zone::Battlefield);
    let artifact_id = game.create_object_from_card(&celebration_artifact, alice, Zone::Battlefield);
    record_battlefield_entry_this_turn(&mut game, dragon_ally_id);
    record_battlefield_entry_this_turn(&mut game, artifact_id);
    game.refresh_continuous_state();

    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(crate::mana::ManaSymbol::Red, 1);

    let activate_action = crate::decision::compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::ActivateAbility { source, .. } if *source == goddric_id
            )
        })
        .expect("celebrating Goddric should expose its granted red activation");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = AutoPassDecisionMaker;
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(activate_action),
        &mut dm,
    )
    .expect("Goddric's granted ability should activate");

    match progress {
        crate::decision::GameProgress::Continue
        | crate::decision::GameProgress::StackResolved
        | crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Priority(_),
        ) => {}
        other => panic!("unexpected progress while activating Goddric: {other:?}"),
    }

    assert_eq!(
        game.stack.len(),
        1,
        "Goddric's granted ability should be placed on the stack"
    );

    resolve_stack_entry(&mut game).expect("Goddric's granted ability should resolve");
    game.refresh_continuous_state();

    assert_eq!(
        game.calculated_power(goddric_id),
        Some(5),
        "Goddric should pump itself once it is a Dragon"
    );
    assert_eq!(
        game.calculated_power(dragon_ally_id),
        Some(3),
        "other Dragons you control should get +1/+0"
    );
    assert_eq!(
        game.calculated_power(human_ally_id),
        Some(2),
        "non-Dragons you control should not get pumped"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn kjeldoran_elite_guard_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Kjeldoran Elite Guard")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Soldier])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "{T}: Target creature gets +2/+2 until end of turn. When that creature leaves the battlefield this turn, sacrifice this creature. Activate only during combat.",
        )
        .expect("Kjeldoran Elite Guard should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn zone_contains_named(game: &GameState, zone: Zone, name: &str) -> bool {
    game.objects_in_zone(zone).into_iter().any(|id| {
        game.object(id)
            .is_some_and(|object| object.name.eq_ignore_ascii_case(name))
    })
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn kjeldoran_elite_guard_delayed_trigger_tracks_only_targeted_creature() {
    use crate::decision::{LegalAction, compute_legal_actions};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::BeginCombat);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let guard = kjeldoran_elite_guard_definition();
    let guard_id = game.create_object_from_definition(&guard, alice, Zone::Battlefield);
    game.remove_summoning_sickness(guard_id);

    let target = CardBuilder::new(CardId::new(), "Chosen Target")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let decoy = CardBuilder::new(CardId::new(), "Decoy Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let target_id = game.create_object_from_card(&target, bob, Zone::Battlefield);
    let decoy_id = game.create_object_from_card(&decoy, bob, Zone::Battlefield);

    let activate_action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                LegalAction::ActivateAbility { source, .. } if *source == guard_id
            )
        })
        .expect("Kjeldoran Elite Guard should be activatable during combat");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = ChooseSpecificObjectDecisionMaker::new(target_id);
    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(activate_action),
        &mut dm,
    )
    .expect("Kjeldoran Elite Guard activation should choose the target and go on the stack");

    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Targets(vec![Target::Object(target_id)]),
        &mut dm,
    )
    .expect("Kjeldoran Elite Guard target choice should complete activation");

    if !game.stack.is_empty() {
        resolve_stack_entry(&mut game).expect("Kjeldoran Elite Guard activation should resolve");
    }
    game.refresh_continuous_state();

    assert_eq!(
        game.calculated_power(target_id),
        Some(4),
        "the chosen target should get +2/+2 until end of turn"
    );
    assert_eq!(
        game.calculated_power(decoy_id),
        Some(2),
        "an untargeted creature should not get the pump"
    );

    game.move_object(
        decoy_id,
        Zone::Graveyard,
        crate::events::cause::EventCause::effect(),
    )
    .expect("decoy should move to graveyard");
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    assert!(
        trigger_queue.entries.is_empty(),
        "a nontarget creature leaving should not trigger the delayed sacrifice"
    );
    assert!(
        zone_contains_named(&game, Zone::Battlefield, "Kjeldoran Elite Guard"),
        "Kjeldoran Elite Guard should remain after the decoy leaves"
    );

    game.move_object(
        target_id,
        Zone::Graveyard,
        crate::events::cause::EventCause::effect(),
    )
    .expect("target should move to graveyard");
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "the targeted creature leaving should trigger the delayed sacrifice"
    );

    let mut trigger_dm = SelectFirstDecisionMaker;
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut trigger_dm)
        .expect("delayed sacrifice trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("delayed sacrifice trigger should resolve");

    assert!(
        zone_contains_named(&game, Zone::Graveyard, "Kjeldoran Elite Guard"),
        "Kjeldoran Elite Guard should be sacrificed when the targeted creature leaves"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_root_greevil_activation_reaches_stack_and_resolves_with_color_choice() {
    use crate::decision::{LegalAction, compute_legal_actions};

    #[derive(Default)]
    struct ChooseBlueModeDecisionMaker;

    impl DecisionMaker for ChooseBlueModeDecisionMaker {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            let legal_options: Vec<_> = ctx.options.iter().filter(|option| option.legal).collect();
            ctx.options
                .iter()
                .find(|option| {
                    option.legal && option.description.to_ascii_lowercase().contains("blue")
                })
                .or_else(|| legal_options.get(1).copied())
                .or_else(|| legal_options.first().copied())
                .map(|option| vec![option.index])
                .unwrap_or_else(|| {
                    legal_options
                        .into_iter()
                        .map(|option| option.index)
                        .take(ctx.min)
                        .collect()
                })
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    if let Some(player) = game.player_mut(alice) {
        player.mana_pool.add(ManaSymbol::Colorless, 2);
        player.mana_pool.add(ManaSymbol::Green, 1);
    }

    let root_def = CardDefinitionBuilder::new(CardId::new(), "Root Greevil Variant")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Beast])
        .power_toughness(PowerToughness::fixed(2, 3))
        .parse_text(
            "{2}{G}, {T}, Sacrifice this creature: Destroy all enchantments of the color of your choice.",
        )
        .expect("Root Greevil should parse");
    let root_id = game.create_object_from_definition(&root_def, alice, Zone::Battlefield);
    game.remove_summoning_sickness(root_id);

    let blue_one = CardBuilder::new(CardId::new(), "Azure Sigil")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Enchantment])
        .build();
    let blue_two = CardBuilder::new(CardId::new(), "Tidal Sigil")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Enchantment])
        .build();
    let green_enchantment = CardBuilder::new(CardId::new(), "Verdant Sigil")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .card_types(vec![CardType::Enchantment])
        .build();

    game.create_object_from_card(&blue_one, alice, Zone::Battlefield);
    game.create_object_from_card(&blue_two, bob, Zone::Battlefield);
    game.create_object_from_card(&green_enchantment, bob, Zone::Battlefield);

    let ability_index = game
        .object(root_id)
        .expect("Root Greevil should exist")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("Root Greevil should have an activated ability");

    let activate_action = compute_legal_actions(&game, alice)
        .into_iter()
        .find(
            |action| matches!(action, LegalAction::ActivateAbility { source, ability_index: idx } if *source == root_id && *idx == ability_index),
        )
        .expect("Root Greevil's ability should be activatable");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = AutoPassDecisionMaker;

    let mut progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(activate_action),
        &mut dm,
    )
    .expect("Root Greevil activation should succeed");
    while let crate::decision::GameProgress::NeedsDecisionCtx(decision) = progress {
        progress = match decision {
            crate::decisions::context::DecisionContext::SelectOptions(ctx) => {
                let choice = ctx
                    .options
                    .iter()
                    .find(|option| {
                        option.legal
                            && option
                                .description
                                .to_ascii_lowercase()
                                .contains("sacrifice")
                    })
                    .or_else(|| ctx.options.iter().find(|option| option.legal))
                    .map(|option| option.index)
                    .expect("Root Greevil should offer a legal activation-cost choice");
                apply_priority_response_with_dm(
                    &mut game,
                    &mut trigger_queue,
                    &mut state,
                    &PriorityResponse::NextCostChoice(choice),
                    &mut dm,
                )
                .expect("Root Greevil cost choice should continue activation")
            }
            crate::decisions::context::DecisionContext::Modes(ctx) => {
                let choice = ctx
                    .spec
                    .modes
                    .iter()
                    .find(|mode| {
                        mode.legal && mode.description.to_ascii_lowercase().contains("blue")
                    })
                    .or_else(|| ctx.spec.modes.iter().find(|mode| mode.legal))
                    .map(|mode| mode.index)
                    .expect("Root Greevil should offer a legal color mode");
                apply_priority_response_with_dm(
                    &mut game,
                    &mut trigger_queue,
                    &mut state,
                    &PriorityResponse::Modes(vec![choice]),
                    &mut dm,
                )
                .expect("Root Greevil mode choice should continue activation")
            }
            crate::decisions::context::DecisionContext::Priority(_) => break,
            other => panic!(
                "unexpected decision while activating Root Greevil: {:?}",
                other
            ),
        };
    }

    assert!(
        game.object(root_id)
            .map(|object| object.zone != Zone::Battlefield)
            .unwrap_or(true),
        "sacrificing Root Greevil should remove it from the battlefield as part of the activation cost"
    );
    assert_eq!(
        game.stack.len(),
        1,
        "the activated ability should be waiting on the stack"
    );

    let mut dm = ChooseBlueModeDecisionMaker::default();
    resolve_stack_entry_with(&mut game, &mut dm).expect("Root Greevil ability should resolve");
    assert!(
        game.stack.is_empty(),
        "Root Greevil should finish resolving cleanly"
    );
}

pub(super) fn persist_effects() -> Vec<Effect> {
    let trigger_tag = "persist_trigger";
    let return_tag = "persist_return";
    let returned_tag = "persist_returned";

    let filter = ObjectFilter::default()
        .in_zone(Zone::Graveyard)
        .same_stable_id_as_tagged(trigger_tag);

    let choose = Effect::choose_objects(filter, 1, crate::target::PlayerFilter::You, return_tag);
    let move_to_battlefield = Effect::move_to_zone(
        ChooseSpec::Tagged(return_tag.into()),
        Zone::Battlefield,
        true,
    )
    .tag(returned_tag);
    let counters = Effect::for_each_tagged(
        returned_tag,
        vec![Effect::put_counters(
            CounterType::MinusOneMinusOne,
            1,
            ChooseSpec::Iterated,
        )],
    );

    vec![
        Effect::tag_triggering_object(trigger_tag),
        choose,
        move_to_battlefield,
        counters,
    ]
}

// === Stack Resolution Tests ===

#[test]
pub(super) fn test_resolve_empty_stack() {
    let mut game = setup_game();
    let result = resolve_stack_entry(&mut game);
    assert!(result.is_err());
}

#[test]
pub(super) fn test_resolve_stack_entry_basic() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    // Create a simple instant
    let card = CardBuilder::new(CardId::from_raw(1), "Test Instant")
        .card_types(vec![CardType::Instant])
        .build();
    let spell_id = game.create_object_from_card(&card, alice, Zone::Stack);

    // Put on stack
    let entry = StackEntry::new(spell_id, alice);
    game.push_to_stack(entry);

    // Resolve
    let result = resolve_stack_entry(&mut game);
    assert!(result.is_ok());

    // Stack should be empty
    assert!(game.stack_is_empty());

    // Spell should be in graveyard
    let player = game.player(alice).unwrap();
    assert_eq!(player.graveyard.len(), 1);
}

#[test]
pub(super) fn resolving_spell_can_insert_additional_combat_and_main_phases() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.active_player = alice;
    game.turn.phase = Phase::NextMain;
    game.turn.step = None;

    let card = CardBuilder::new(CardId::from_raw(91_001), "Extra Combat Test")
        .card_types(vec![CardType::Sorcery])
        .build();
    let spell_id = game.create_object_from_card(&card, alice, Zone::Stack);
    game.object_mut(spell_id).expect("spell").spell_effect = Some(
        crate::resolution::ResolutionProgram::from_effects(vec![Effect::new(
            crate::effects::AdditionalPhasesEffect::combat_then_main(),
        )])
        .into(),
    );

    game.push_to_stack(StackEntry::new(spell_id, alice));
    resolve_stack_entry(&mut game).expect("spell should resolve");

    crate::turn::advance_phase(&mut game).expect("advance to inserted combat");
    assert_eq!(game.turn.phase, Phase::Combat);
    assert_eq!(game.turn.step, Some(crate::game_state::Step::BeginCombat));

    game.turn.step = None;
    crate::turn::advance_phase(&mut game).expect("advance to inserted main");
    assert_eq!(game.turn.phase, Phase::NextMain);
}

#[test]
pub(super) fn resolving_spell_with_tag_and_untap_then_additional_phases_runs_all_effects() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.active_player = alice;
    game.turn.phase = Phase::NextMain;
    game.turn.step = None;

    let creature = CardBuilder::new(CardId::from_raw(91_002), "Test Creature")
        .card_types(vec![CardType::Creature])
        .build();
    game.create_object_from_card(&creature, alice, Zone::Battlefield);

    let card = CardBuilder::new(CardId::from_raw(91_003), "Fury Shape Test")
        .card_types(vec![CardType::Sorcery])
        .build();
    let spell_id = game.create_object_from_card(&card, alice, Zone::Stack);
    let filter = crate::filter::ObjectFilter {
        zone: Some(Zone::Battlefield),
        card_types: vec![CardType::Creature],
        ..Default::default()
    };
    game.object_mut(spell_id).expect("spell").spell_effect = Some(
        crate::resolution::ResolutionProgram::from_effects(vec![
            Effect::new(crate::effects::TagMatchingObjectsEffect::new(
                filter.clone(),
                crate::tag::TagKey::from("untapped_0"),
            )),
            Effect::new(crate::effects::UntapEffect::with_spec(
                crate::target::ChooseSpec::All(filter),
            )),
            Effect::new(crate::effects::AdditionalPhasesEffect::combat_then_main()),
        ])
        .into(),
    );

    game.push_to_stack(StackEntry::new(spell_id, alice));
    resolve_stack_entry(&mut game).expect("spell should resolve");

    crate::turn::advance_phase(&mut game).expect("advance to inserted combat");
    assert_eq!(game.turn.phase, Phase::Combat);
}

#[test]
pub(super) fn full_throttle_inserts_two_additional_combats_and_reaches_normal_next_main() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.active_player = alice;
    game.turn.phase = Phase::NextMain;
    game.turn.step = None;

    let full_throttle = CardDefinitionBuilder::new(CardId::new(), "Full Throttle")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "After this main phase, there are two additional combat phases.\nAt the beginning of each combat this turn, untap all creatures that attacked this turn.",
        )
        .expect("Full Throttle should parse for runtime test");

    let spell_id = game.create_object_from_definition(&full_throttle, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell_id, alice));
    resolve_stack_entry(&mut game).expect("Full Throttle should resolve");

    crate::turn::advance_phase(&mut game).expect("advance to first inserted combat");
    assert_eq!(game.turn.phase, Phase::Combat);
    assert_eq!(game.turn.step, Some(crate::game_state::Step::BeginCombat));

    game.turn.step = None;
    crate::turn::advance_phase(&mut game).expect("advance to second inserted combat");
    assert_eq!(game.turn.phase, Phase::Combat);
    assert_eq!(game.turn.step, Some(crate::game_state::Step::BeginCombat));

    game.turn.step = None;
    crate::turn::advance_phase(&mut game)
        .expect("advance to normal phase order after added combats");
    assert_eq!(game.turn.phase, Phase::Ending);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_resolve_stack_entry_uses_self_replacement_branch_when_condition_is_true() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let spell_def = CardDefinitionBuilder::new(CardId::new(), "Refined Analysis")
        .card_types(vec![CardType::Instant])
        .parse_text("Draw a card. If you control an artifact, draw two cards instead.")
        .expect("self-replacement spell should parse");
    let artifact_def = CardDefinitionBuilder::new(CardId::new(), "Proof Artifact")
        .card_types(vec![CardType::Artifact])
        .build();
    let library_card = CardBuilder::new(CardId::new(), "Library Card A")
        .card_types(vec![CardType::Artifact])
        .build();
    let second_library_card = CardBuilder::new(CardId::new(), "Library Card B")
        .card_types(vec![CardType::Artifact])
        .build();

    game.create_object_from_definition(&artifact_def, alice, Zone::Battlefield);
    game.create_object_from_card(&library_card, alice, Zone::Library);
    game.create_object_from_card(&second_library_card, alice, Zone::Library);

    let hand_before = game.player(alice).expect("alice exists").hand.len();
    let library_before = game.player(alice).expect("alice exists").library.len();

    let spell_id = game.create_object_from_definition(&spell_def, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell_id, alice));

    resolve_stack_entry(&mut game).expect("spell should resolve");

    assert_eq!(
        game.player(alice).expect("alice exists").hand.len(),
        hand_before + 2,
        "the self-replacement branch should replace draw one with draw two"
    );
    assert_eq!(
        game.player(alice).expect("alice exists").library.len(),
        library_before - 2,
        "the replacement branch should consume two cards from the library"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_resolve_stack_entry_uses_default_effect_when_self_replacement_condition_is_false()
 {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let spell_def = CardDefinitionBuilder::new(CardId::new(), "Refined Analysis")
        .card_types(vec![CardType::Instant])
        .parse_text("Draw a card. If you control an artifact, draw two cards instead.")
        .expect("self-replacement spell should parse");
    let library_card = CardBuilder::new(CardId::new(), "Library Card A")
        .card_types(vec![CardType::Artifact])
        .build();
    let second_library_card = CardBuilder::new(CardId::new(), "Library Card B")
        .card_types(vec![CardType::Artifact])
        .build();

    game.create_object_from_card(&library_card, alice, Zone::Library);
    game.create_object_from_card(&second_library_card, alice, Zone::Library);

    let hand_before = game.player(alice).expect("alice exists").hand.len();
    let library_before = game.player(alice).expect("alice exists").library.len();

    let spell_id = game.create_object_from_definition(&spell_def, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell_id, alice));

    resolve_stack_entry(&mut game).expect("spell should resolve");

    assert_eq!(
        game.player(alice).expect("alice exists").hand.len(),
        hand_before + 1,
        "the default segment should resolve when the self-replacement condition is false"
    );
    assert_eq!(
        game.player(alice).expect("alice exists").library.len(),
        library_before - 1,
        "the default segment should draw exactly one card"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_echo_upkeep_trigger_without_payment_sacrifices_source() {
    use crate::ability::AbilityKind;
    use crate::cards::CardDefinitionBuilder;
    use crate::ids::CardId;
    use crate::object::CounterType;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let mogg_war_marshal = CardDefinitionBuilder::new(CardId::new(), "Mogg War Marshal")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .parse_text(
                "Echo {1}{R} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)",
            )
            .expect("echo ability should parse");
    let marshal_id =
        game.create_object_from_definition(&mogg_war_marshal, alice, Zone::Battlefield);
    game.object_mut(marshal_id)
        .expect("mogg war marshal should exist")
        .counters
        .insert(CounterType::Echo, 1);

    let echo_effects = game
        .object(marshal_id)
        .expect("mogg war marshal should exist")
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered.effects.clone()),
            _ => None,
        })
        .expect("echo trigger effects should exist");
    game.push_to_stack(StackEntry::ability(marshal_id, alice, echo_effects));

    let mut dm = crate::decision::SelectFirstDecisionMaker;
    resolve_stack_entry_with(&mut game, &mut dm).expect("echo trigger should resolve");

    let still_on_battlefield = game.battlefield.iter().any(|id| {
        game.object(*id)
            .is_some_and(|obj| obj.name == "Mogg War Marshal")
    });
    assert!(
        !still_on_battlefield,
        "Mogg War Marshal should be sacrificed when echo is unpaid"
    );
    let in_graveyard = game.player(alice).is_some_and(|player| {
        player.graveyard.iter().any(|id| {
            game.object(*id)
                .is_some_and(|obj| obj.name == "Mogg War Marshal")
        })
    });
    assert!(
        in_graveyard,
        "Mogg War Marshal should end up in graveyard after unpaid echo"
    );
}

#[test]
pub(super) fn test_echo_trigger_ignores_source_after_zone_change() {
    use crate::object::CounterType;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let echo_card = CardDefinitionBuilder::new(CardId::new(), "Blinked Echo")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .echo(crate::cost::TotalCost::mana(ManaCost::from_pips(vec![
            vec![ManaSymbol::Red],
        ])))
        .build();
    let original_id = game.create_object_from_definition(&echo_card, alice, Zone::Battlefield);
    game.object_mut(original_id)
        .expect("echo permanent should exist")
        .counters
        .insert(CounterType::Echo, 1);

    let source_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(original_id)
            .expect("echo permanent should still exist"),
        &game,
    );
    let echo_effects = game
        .object(original_id)
        .expect("echo permanent should exist")
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered.effects.clone()),
            _ => None,
        })
        .expect("echo trigger effects should exist");
    let entry = StackEntry::ability(original_id, alice, echo_effects)
        .with_source_info(source_snapshot.stable_id, source_snapshot.name.to_string())
        .with_source_snapshot(source_snapshot);

    let exiled_id = game
        .move_object_by_effect(original_id, Zone::Exile)
        .expect("echo permanent should move to exile");
    let returned_id = game
        .move_object_by_effect(exiled_id, Zone::Battlefield)
        .expect("echo permanent should return to battlefield");
    game.object_mut(returned_id)
        .expect("returned echo permanent should exist")
        .counters
        .insert(CounterType::Echo, 1);
    game.push_to_stack(entry);

    let mut dm = SelectFirstDecisionMaker;
    resolve_stack_entry_with(&mut game, &mut dm).expect("stale echo trigger should resolve");

    let returned = game
        .object(returned_id)
        .expect("returned echo permanent should remain");
    assert_eq!(
        returned.zone,
        Zone::Battlefield,
        "stale echo trigger should not sacrifice the returned object"
    );
    assert_eq!(
        returned
            .counters
            .get(&CounterType::Echo)
            .copied()
            .unwrap_or(0),
        1,
        "stale echo trigger should not remove counters from the returned object"
    );
}

#[test]
pub(super) fn test_enter_as_copy_applies_copied_enters_with_echo_counter() {
    use crate::object::CounterType;
    use crate::static_abilities::EnterAsCopyAsEntersSpec;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let echo_source = CardDefinitionBuilder::new(CardId::new(), "Echo Source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .echo(crate::cost::TotalCost::mana(ManaCost::from_pips(vec![
            vec![ManaSymbol::Green],
        ])))
        .build();
    game.create_object_from_definition(&echo_source, alice, Zone::Battlefield);

    let clone = CardDefinitionBuilder::new(CardId::new(), "Entering Clone")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(0, 0))
        .with_ability(Ability::static_ability(
            StaticAbility::with_enter_as_copy_as_enters(
                EnterAsCopyAsEntersSpec {
                    filter: crate::target::ObjectFilter::creature(),
                    affected_filter: None,
                    may: false,
                    enters_tapped_if_chosen: false,
                    copy_duration: None,
                    linked_exile_pair: None,
                    copy_source_self: false,
                    copy_source_enchanted: false,
                    name_override: None,
                    added_card_types: Vec::new(),
                    removed_supertypes: Vec::new(),
                    added_subtypes: Vec::new(),
                    added_abilities: Vec::new(),
                    set_base_power_toughness: None,
                    set_base_power_toughness_from_self: false,
                },
                "You may have this creature enter as a copy of any creature on the battlefield."
                    .to_string(),
            ),
        ))
        .build();
    let clone_id = game.create_object_from_definition(&clone, alice, Zone::Hand);

    let result = game
        .move_object_with_etb_processing(clone_id, Zone::Battlefield)
        .expect("clone should enter the battlefield");
    let copied = game
        .object(result.new_id)
        .expect("copied permanent should exist");

    assert_eq!(copied.name, "Echo Source");
    assert_eq!(
        copied
            .counters
            .get(&CounterType::Echo)
            .copied()
            .unwrap_or(0),
        1,
        "a clone entering as an echo permanent should apply the copied enters-with-counter ability"
    );
}

#[test]
pub(super) fn test_enter_as_copy_can_set_base_power_toughness_from_entering_object() {
    use crate::static_abilities::EnterAsCopyAsEntersSpec;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source = CardDefinitionBuilder::new(CardId::new(), "Copy Source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    game.create_object_from_definition(&source, alice, Zone::Battlefield);

    let metamorph = CardDefinitionBuilder::new(CardId::new(), "Metamorph Probe")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(7, 7))
        .with_ability(Ability::static_ability(
            StaticAbility::with_enter_as_copy_as_enters(
                EnterAsCopyAsEntersSpec {
                    filter: crate::target::ObjectFilter::creature(),
                    affected_filter: None,
                    may: false,
                    enters_tapped_if_chosen: false,
                    copy_duration: None,
                    linked_exile_pair: None,
                    copy_source_self: false,
                    copy_source_enchanted: false,
                    name_override: None,
                    added_card_types: Vec::new(),
                    removed_supertypes: Vec::new(),
                    added_subtypes: Vec::new(),
                    added_abilities: Vec::new(),
                    set_base_power_toughness: None,
                    set_base_power_toughness_from_self: true,
                },
                "You may have this creature enter as a copy of any creature on the battlefield, except its power and toughness are equal to this creature's power and toughness."
                    .to_string(),
            ),
        ))
        .build();
    let metamorph_id = game.create_object_from_definition(&metamorph, alice, Zone::Hand);

    let result = game
        .move_object_with_etb_processing(metamorph_id, Zone::Battlefield)
        .expect("metamorph should enter the battlefield");
    let copied = game
        .object(result.new_id)
        .expect("copied permanent should exist");

    assert_eq!(copied.name, "Copy Source");
    assert_eq!(copied.base_power, Some(crate::card::PtValue::Fixed(7)));
    assert_eq!(copied.base_toughness, Some(crate::card::PtValue::Fixed(7)));
}

#[test]
pub(super) fn test_enter_as_copy_can_set_base_power_toughness_from_entering_stack_object() {
    use crate::static_abilities::EnterAsCopyAsEntersSpec;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source = CardDefinitionBuilder::new(CardId::new(), "Copy Source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    game.create_object_from_definition(&source, alice, Zone::Battlefield);

    let metamorph = CardDefinitionBuilder::new(CardId::new(), "Metamorph Probe")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(7, 7))
        .with_ability(Ability::static_ability(
            StaticAbility::with_enter_as_copy_as_enters(
                EnterAsCopyAsEntersSpec {
                    filter: crate::target::ObjectFilter::creature(),
                    affected_filter: None,
                    may: false,
                    enters_tapped_if_chosen: false,
                    copy_duration: None,
                    linked_exile_pair: None,
                    copy_source_self: false,
                    copy_source_enchanted: false,
                    name_override: None,
                    added_card_types: Vec::new(),
                    removed_supertypes: Vec::new(),
                    added_subtypes: Vec::new(),
                    added_abilities: Vec::new(),
                    set_base_power_toughness: None,
                    set_base_power_toughness_from_self: true,
                },
                "You may have this creature enter as a copy of any creature on the battlefield, except its power and toughness are equal to this creature's power and toughness."
                    .to_string(),
            ),
        ))
        .build();
    let metamorph_id = game.create_object_from_definition(&metamorph, alice, Zone::Stack);

    let result = game
        .move_object_with_etb_processing(metamorph_id, Zone::Battlefield)
        .expect("metamorph should enter the battlefield");
    let copied = game
        .object(result.new_id)
        .expect("copied permanent should exist");

    assert_eq!(copied.name, "Copy Source");
    assert_eq!(copied.base_power, Some(crate::card::PtValue::Fixed(7)));
    assert_eq!(copied.base_toughness, Some(crate::card::PtValue::Fixed(7)));
}

#[test]
pub(super) fn test_static_source_can_make_matching_creatures_enter_as_copy_of_itself() {
    use crate::static_abilities::EnterAsCopyAsEntersSpec;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let essence = CardDefinitionBuilder::new(CardId::new(), "Essence Source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(6, 6))
        .with_ability(Ability::static_ability(
            StaticAbility::with_enter_as_copy_as_enters(
                EnterAsCopyAsEntersSpec {
                    filter: crate::target::ObjectFilter::source(),
                    affected_filter: Some(crate::target::ObjectFilter::creature().you_control()),
                    may: false,
                    enters_tapped_if_chosen: false,
                    copy_duration: None,
                    linked_exile_pair: None,
                    copy_source_self: true,
                    copy_source_enchanted: false,
                    name_override: None,
                    added_card_types: Vec::new(),
                    removed_supertypes: Vec::new(),
                    added_subtypes: Vec::new(),
                    added_abilities: Vec::new(),
                    set_base_power_toughness: None,
                    set_base_power_toughness_from_self: false,
                },
                "Creatures you control enter as a copy of this creature.".to_string(),
            ),
        ))
        .build();
    game.create_object_from_definition(&essence, alice, Zone::Battlefield);

    let bear = CardDefinitionBuilder::new(CardId::new(), "Entering Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let bear_id = game.create_object_from_definition(&bear, alice, Zone::Hand);

    let result = game
        .move_object_with_etb_processing(bear_id, Zone::Battlefield)
        .expect("creature should enter the battlefield");
    let copied = game
        .object(result.new_id)
        .expect("copied permanent should exist");

    assert_eq!(copied.name, "Essence Source");
    assert_eq!(copied.base_power, Some(crate::card::PtValue::Fixed(6)));
    assert_eq!(copied.base_toughness, Some(crate::card::PtValue::Fixed(6)));
}

#[test]
pub(super) fn test_enter_as_copy_can_remove_legendary_add_artifact_and_add_myriad() {
    use crate::static_abilities::EnterAsCopyAsEntersSpec;
    use crate::types::Supertype;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let legendary_source = CardDefinitionBuilder::new(CardId::new(), "Legendary Copy Source")
        .card_types(vec![CardType::Creature])
        .supertypes(vec![Supertype::Legendary])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    game.create_object_from_definition(&legendary_source, alice, Zone::Battlefield);

    let auton_like = CardDefinitionBuilder::new(CardId::new(), "Auton Soldier")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .with_ability(Ability::static_ability(
            StaticAbility::with_enter_as_copy_as_enters(
                EnterAsCopyAsEntersSpec {
                    filter: crate::target::ObjectFilter::creature(),
                    affected_filter: None,
                    may: false,
                    enters_tapped_if_chosen: false,
                    copy_duration: None,
                    linked_exile_pair: None,
                    copy_source_self: false,
                    copy_source_enchanted: false,
                    name_override: None,
                    added_card_types: vec![CardType::Artifact],
                    removed_supertypes: vec![Supertype::Legendary],
                    added_subtypes: Vec::new(),
                    added_abilities: vec![Ability::triggered(
                        Trigger::this_attacks(),
                        vec![Effect::for_players(
                            PlayerFilter::excluding(
                                PlayerFilter::Opponent,
                                PlayerFilter::Defending,
                            ),
                            vec![Effect::may(vec![Effect::new(
                                crate::effects::CreateTokenCopyEffect::new(
                                    ChooseSpec::Source,
                                    1,
                                    PlayerFilter::You,
                                )
                                .enters_tapped(true)
                                .attacking_player_or_planeswalker_controlled_by(
                                    PlayerFilter::IteratedPlayer,
                                )
                                .exile_at_eoc(true),
                            )])],
                        )],
                    )],
                    set_base_power_toughness: None,
                    set_base_power_toughness_from_self: false,
                },
                "You may have this creature enter as a copy of any creature on the battlefield, except it isn't legendary, is an artifact in addition to its other types, and has myriad."
                    .to_string(),
            ),
        ))
        .build();
    let auton_like_id = game.create_object_from_definition(&auton_like, alice, Zone::Hand);

    let result = game
        .move_object_with_etb_processing(auton_like_id, Zone::Battlefield)
        .expect("auton-like permanent should enter the battlefield");
    let copied = game
        .object(result.new_id)
        .expect("copied permanent should exist");

    assert_eq!(copied.name, "Legendary Copy Source");
    assert!(
        !copied.supertypes.contains(&Supertype::Legendary),
        "copied permanent should lose legendary"
    );
    assert!(
        copied.card_types.contains(&CardType::Artifact),
        "copied permanent should be an artifact in addition to copied types"
    );
    let abilities_debug = format!("{:?}", copied.abilities);
    assert!(
        abilities_debug.contains("CreateTokenCopyEffect")
            && abilities_debug.contains("ForPlayersEffect")
            && abilities_debug.contains("MayEffect")
            && !abilities_debug.contains("StaticAbilityId::KeywordMarker"),
        "copied permanent should gain functional myriad trigger, got {abilities_debug}"
    );
}

#[test]
pub(super) fn test_enter_as_copy_with_no_candidates_keeps_original_characteristics() {
    use crate::static_abilities::EnterAsCopyAsEntersSpec;
    use crate::types::Supertype;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let auton_like = CardDefinitionBuilder::new(CardId::new(), "Auton Soldier")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .with_ability(Ability::static_ability(
            StaticAbility::with_enter_as_copy_as_enters(
                EnterAsCopyAsEntersSpec {
                    filter: crate::target::ObjectFilter::creature(),
                    affected_filter: None,
                    may: false,
                    enters_tapped_if_chosen: false,
                    copy_duration: None,
                    linked_exile_pair: None,
                    copy_source_self: false,
                    copy_source_enchanted: false,
                    name_override: None,
                    added_card_types: vec![CardType::Artifact],
                    removed_supertypes: vec![Supertype::Legendary],
                    added_subtypes: Vec::new(),
                    added_abilities: vec![Ability::triggered(
                        Trigger::this_attacks(),
                        vec![Effect::for_players(
                            PlayerFilter::excluding(
                                PlayerFilter::Opponent,
                                PlayerFilter::Defending,
                            ),
                            vec![Effect::may(vec![Effect::new(
                                crate::effects::CreateTokenCopyEffect::new(
                                    ChooseSpec::Source,
                                    1,
                                    PlayerFilter::You,
                                )
                                .enters_tapped(true)
                                .attacking_player_or_planeswalker_controlled_by(
                                    PlayerFilter::IteratedPlayer,
                                )
                                .exile_at_eoc(true),
                            )])],
                        )],
                    )],
                    set_base_power_toughness: None,
                    set_base_power_toughness_from_self: false,
                },
                "You may have this creature enter as a copy of any creature on the battlefield, except it isn't legendary, is an artifact in addition to its other types, and has myriad."
                    .to_string(),
            ),
        ))
        .build();
    let auton_like_id = game.create_object_from_definition(&auton_like, alice, Zone::Hand);

    let result = game
        .move_object_with_etb_processing(auton_like_id, Zone::Battlefield)
        .expect("auton-like permanent should enter the battlefield");
    let entered = game
        .object(result.new_id)
        .expect("entered permanent should exist");

    assert_eq!(entered.name, "Auton Soldier");
    assert_eq!(entered.base_power, Some(crate::card::PtValue::Fixed(4)));
    assert_eq!(entered.base_toughness, Some(crate::card::PtValue::Fixed(4)));
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn sakashimas_student_test_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Sakashima's Student")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Ninja])
        .power_toughness(PowerToughness::fixed(0, 0))
        .parse_text(
            "Ninjutsu {1}{U} ({1}{U}, Return an unblocked attacker you control to hand: Put this card onto the battlefield from your hand tapped and attacking.)\nYou may have this creature enter as a copy of any creature on the battlefield, except it's a Ninja in addition to its other creature types.",
        )
        .expect("Sakashima's Student should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct ChooseSakashimaCopySourceDecisionMaker {
    pub(super) source_name: &'static str,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for ChooseSakashimaCopySourceDecisionMaker {
    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        ctx.options
            .iter()
            .find(|option| option.legal && option.description.contains(self.source_name))
            .map(|option| vec![option.index])
            .unwrap_or_default()
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct PanicOnSakashimaCopyPrompt;

#[cfg(ironsmith_runtime_parser_tests)]
impl DecisionMaker for PanicOnSakashimaCopyPrompt {
    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        panic!(
            "Sakashima's Student should not offer copy choices without another creature: {:?}",
            ctx.options
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sakashimas_student_ninjutsu_cost_returns_unblocked_attacker_and_records_target() {
    use crate::combat_state::{AttackerInfo, CombatState};
    use crate::effect::OutcomeStatus;
    use crate::effects::{EffectExecutor as _, ExecutionContext, NinjutsuCostEffect};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let student = sakashimas_student_test_definition();
    let student_id = game.create_object_from_definition(&student, alice, Zone::Hand);
    let attacker = CardDefinitionBuilder::new(CardId::new(), "Unblocked Attacker")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let attacker_id = game.create_object_from_definition(&attacker, alice, Zone::Battlefield);
    game.remove_summoning_sickness(attacker_id);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::DeclareBlockers);
    game.combat = Some(CombatState {
        attackers: vec![AttackerInfo {
            creature: attacker_id,
            target: AttackTarget::Player(bob),
        }],
        ..CombatState::default()
    });

    let mut ctx = ExecutionContext::new_default(student_id, alice);
    let result = NinjutsuCostEffect::new()
        .execute(&mut game, &mut ctx)
        .expect("Sakashima's Student ninjutsu cost should resolve");

    assert!(matches!(result.status, OutcomeStatus::Succeeded));
    assert!(
        game.player(alice)
            .expect("Alice exists")
            .hand
            .iter()
            .filter_map(|id| game.object(*id))
            .any(|obj| obj.name == "Unblocked Attacker"),
        "ninjutsu cost should return the unblocked attacker to hand"
    );
    assert!(
        game.combat
            .as_ref()
            .is_some_and(|combat| combat.attackers.is_empty()),
        "returned attacker should be removed from combat"
    );
    assert_eq!(
        game.last_ninjutsu_attack_target(student_id).cloned(),
        Some(AttackTarget::Player(bob)),
        "ninjutsu cost should remember the original attack target for the entering Student"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sakashimas_student_ninjutsu_enters_tapped_attacking_as_copy_with_added_ninja_type() {
    use crate::effect::OutcomeValue;
    use crate::effects::{EffectExecutor as _, ExecutionContext, NinjutsuEffect};

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let copy_source = CardDefinitionBuilder::new(CardId::new(), "Runeclaw Bear")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Bear])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    game.create_object_from_definition(&copy_source, alice, Zone::Battlefield);

    let student = sakashimas_student_test_definition();
    let student_id = game.create_object_from_definition(&student, alice, Zone::Hand);
    game.record_ninjutsu_attack_target(student_id, AttackTarget::Player(bob));
    game.combat = Some(crate::combat_state::CombatState::default());
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::CombatDamage);

    let mut dm = ChooseSakashimaCopySourceDecisionMaker {
        source_name: "Runeclaw Bear",
    };
    let mut ctx = ExecutionContext::new_default(student_id, alice).with_decision_maker(&mut dm);
    let result = NinjutsuEffect::new()
        .execute(&mut game, &mut ctx)
        .expect("Sakashima's Student ninjutsu effect should resolve");
    let entered_id = match result.value {
        OutcomeValue::Objects(ids) => ids[0],
        other => panic!("expected Sakashima's Student to enter, got {other:?}"),
    };

    let entered = game
        .object(entered_id)
        .expect("Sakashima's Student permanent should exist");
    assert_eq!(entered.name, "Runeclaw Bear");
    assert_eq!(entered.base_power, Some(PtValue::Fixed(2)));
    assert_eq!(entered.base_toughness, Some(PtValue::Fixed(2)));
    assert!(entered.subtypes.contains(&Subtype::Bear));
    assert!(
        entered.subtypes.contains(&Subtype::Ninja),
        "copy exception should add Ninja to the copied creature types"
    );
    assert!(
        !entered.subtypes.contains(&Subtype::Human),
        "copying the Bear should replace original Human subtype before adding Ninja"
    );
    assert!(game.is_tapped(entered_id));
    assert!(
        game.combat
            .as_ref()
            .is_some_and(|combat| combat.attackers.iter().any(|info| {
                info.creature == entered_id && info.target == AttackTarget::Player(bob)
            })),
        "ninjutsu should leave Sakashima's Student tapped and attacking the recorded player"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sakashimas_student_declined_copy_enters_with_its_own_characteristics() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let copy_source = CardDefinitionBuilder::new(CardId::new(), "Runeclaw Bear")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Bear])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    game.create_object_from_definition(&copy_source, alice, Zone::Battlefield);

    let student = sakashimas_student_test_definition();
    let student_id = game.create_object_from_definition(&student, alice, Zone::Hand);
    let mut dm = AutoPassDecisionMaker;
    let result = game
        .move_object_with_etb_processing_with_dm(student_id, Zone::Battlefield, &mut dm)
        .expect("Sakashima's Student should enter when its optional copy is declined");

    let entered = game
        .object(result.new_id)
        .expect("Sakashima's Student permanent should exist");
    assert_eq!(entered.name, "Sakashima's Student");
    assert_eq!(entered.base_power, Some(PtValue::Fixed(0)));
    assert_eq!(entered.base_toughness, Some(PtValue::Fixed(0)));
    assert!(entered.subtypes.contains(&Subtype::Human));
    assert!(entered.subtypes.contains(&Subtype::Ninja));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sakashimas_student_without_copy_candidate_enters_without_prompt() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let student = sakashimas_student_test_definition();
    let student_id = game.create_object_from_definition(&student, alice, Zone::Hand);
    let mut dm = PanicOnSakashimaCopyPrompt;
    let result = game
        .move_object_with_etb_processing_with_dm(student_id, Zone::Battlefield, &mut dm)
        .expect("Sakashima's Student should enter without another creature to copy");

    let entered = game
        .object(result.new_id)
        .expect("Sakashima's Student permanent should exist");
    assert_eq!(entered.name, "Sakashima's Student");
    assert_eq!(entered.base_power, Some(PtValue::Fixed(0)));
    assert_eq!(entered.base_toughness, Some(PtValue::Fixed(0)));
    assert!(entered.subtypes.contains(&Subtype::Human));
    assert!(entered.subtypes.contains(&Subtype::Ninja));
}

pub(super) fn the_mimeoplasm_test_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "The Mimeoplasm")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Ooze])
        .power_toughness(PowerToughness::fixed(0, 0))
        .parse_text(
            "As The Mimeoplasm enters, you may exile two creature cards from graveyards. If you do, it enters as a copy of one of those cards with a number of additional +1/+1 counters on it equal to the power of the other card.",
        )
        .expect("The Mimeoplasm should parse for runtime tests")
}

pub(super) struct ChooseMimeoplasmPairDecisionMaker {
    pub(super) copy_name: &'static str,
    pub(super) counter_id: ObjectId,
}

impl DecisionMaker for ChooseMimeoplasmPairDecisionMaker {
    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        ctx.options
            .iter()
            .find(|option| {
                option.legal
                    && option.description.contains(self.copy_name)
                    && option
                        .related_object_ids
                        .as_ref()
                        .is_some_and(|ids| ids.contains(&self.counter_id))
            })
            .map(|option| vec![option.index])
            .unwrap_or_else(|| vec![0])
    }
}

pub(super) struct PanicOnMimeoplasmReplacementPrompt;

impl DecisionMaker for PanicOnMimeoplasmReplacementPrompt {
    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        panic!(
            "The Mimeoplasm should not offer replacement choices without two graveyard creatures: {:?}",
            ctx.options
        );
    }
}

#[test]
pub(super) fn the_mimeoplasm_exiles_two_graveyard_creatures_copies_one_and_gets_other_power_counters()
 {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let copy_source = CardDefinitionBuilder::new(CardId::new(), "Copy Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let counter_source = CardDefinitionBuilder::new(CardId::new(), "Counter Wurm")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(6, 6))
        .build();
    game.create_object_from_definition(&copy_source, alice, Zone::Graveyard);
    let counter_id = game.create_object_from_definition(&counter_source, bob, Zone::Graveyard);

    let mimeoplasm = the_mimeoplasm_test_definition();
    let mimeoplasm_id = game.create_object_from_definition(&mimeoplasm, alice, Zone::Hand);
    let mut dm = ChooseMimeoplasmPairDecisionMaker {
        copy_name: "Copy Bear",
        counter_id,
    };
    let result = game
        .move_object_with_etb_processing_with_dm(mimeoplasm_id, Zone::Battlefield, &mut dm)
        .expect("The Mimeoplasm should enter");

    let entered = game
        .object(result.new_id)
        .expect("The Mimeoplasm permanent should exist");
    assert_eq!(entered.name, "Copy Bear");
    assert_eq!(entered.base_power, Some(PtValue::Fixed(2)));
    assert_eq!(entered.base_toughness, Some(PtValue::Fixed(2)));
    assert_eq!(
        entered
            .counters
            .get(&crate::object::CounterType::PlusOnePlusOne)
            .copied()
            .unwrap_or(0),
        6,
        "The Mimeoplasm should get +1/+1 counters equal to the other exiled card's power"
    );

    let linked_ids = game.get_exiled_with_source_links(result.new_id);
    assert!(
        linked_ids.iter().all(|id| game
            .object(*id)
            .is_some_and(|object| object.zone == Zone::Exile)),
        "The Mimeoplasm should link only exiled objects, got {linked_ids:?}"
    );

    let linked_names = linked_ids
        .iter()
        .filter_map(|id| game.object(*id).map(|object| object.name.as_str()))
        .collect::<Vec<_>>();
    assert!(
        linked_names.contains(&"Copy Bear") && linked_names.contains(&"Counter Wurm"),
        "The Mimeoplasm should exile and link both chosen graveyard cards, got {linked_names:?}"
    );
}

#[test]
pub(super) fn the_mimeoplasm_declined_optional_exile_enters_as_itself_and_leaves_graveyards_unchanged()
 {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let first = CardDefinitionBuilder::new(CardId::new(), "First Graveyard Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let second = CardDefinitionBuilder::new(CardId::new(), "Second Graveyard Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    let first_id = game.create_object_from_definition(&first, alice, Zone::Graveyard);
    let second_id = game.create_object_from_definition(&second, alice, Zone::Graveyard);

    let mimeoplasm = the_mimeoplasm_test_definition();
    let mimeoplasm_id = game.create_object_from_definition(&mimeoplasm, alice, Zone::Hand);
    let mut dm = AutoPassDecisionMaker;
    let result = game
        .move_object_with_etb_processing_with_dm(mimeoplasm_id, Zone::Battlefield, &mut dm)
        .expect("The Mimeoplasm should enter even when declined");

    let entered = game
        .object(result.new_id)
        .expect("The Mimeoplasm permanent should exist");
    assert_eq!(entered.name, "The Mimeoplasm");
    assert!(entered.counters.is_empty());
    assert!(
        game.object(first_id)
            .is_some_and(|object| object.zone == Zone::Graveyard)
    );
    assert!(
        game.object(second_id)
            .is_some_and(|object| object.zone == Zone::Graveyard)
    );
    assert!(game.get_exiled_with_source_links(result.new_id).is_empty());
}

#[test]
pub(super) fn the_mimeoplasm_needs_two_graveyard_creature_cards_to_apply_copy_replacement() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let lone = CardDefinitionBuilder::new(CardId::new(), "Lone Graveyard Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 5))
        .build();
    let lone_id = game.create_object_from_definition(&lone, alice, Zone::Graveyard);

    let mimeoplasm = the_mimeoplasm_test_definition();
    let mimeoplasm_id = game.create_object_from_definition(&mimeoplasm, alice, Zone::Hand);
    let mut dm = PanicOnMimeoplasmReplacementPrompt;
    let result = game
        .move_object_with_etb_processing_with_dm(mimeoplasm_id, Zone::Battlefield, &mut dm)
        .expect("The Mimeoplasm should enter without enough graveyard creature cards");

    let entered = game
        .object(result.new_id)
        .expect("The Mimeoplasm permanent should exist");
    assert_eq!(entered.name, "The Mimeoplasm");
    assert!(entered.counters.is_empty());
    assert!(
        game.object(lone_id)
            .is_some_and(|object| object.zone == Zone::Graveyard)
    );
    assert!(game.get_exiled_with_source_links(result.new_id).is_empty());
}

#[test]
pub(super) fn the_mimeoplasm_does_not_count_noncreature_or_token_graveyard_objects_for_its_pair() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let creature = CardDefinitionBuilder::new(CardId::new(), "Only Graveyard Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 5))
        .build();
    let noncreature = CardDefinitionBuilder::new(CardId::new(), "Graveyard Spell")
        .card_types(vec![CardType::Instant])
        .build();
    let token_creature = CardDefinitionBuilder::new(CardId::new(), "Graveyard Token Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(9, 9))
        .build();
    let creature_id = game.create_object_from_definition(&creature, alice, Zone::Graveyard);
    let noncreature_id = game.create_object_from_definition(&noncreature, alice, Zone::Graveyard);
    let token_id = game.create_object_from_definition(&token_creature, alice, Zone::Graveyard);
    game.object_mut(token_id)
        .expect("test token object should exist")
        .kind = ObjectKind::Token;

    let mimeoplasm = the_mimeoplasm_test_definition();
    let mimeoplasm_id = game.create_object_from_definition(&mimeoplasm, alice, Zone::Hand);
    let mut dm = PanicOnMimeoplasmReplacementPrompt;
    let result = game
        .move_object_with_etb_processing_with_dm(mimeoplasm_id, Zone::Battlefield, &mut dm)
        .expect(
            "The Mimeoplasm should enter without counting noncreature or token graveyard objects",
        );

    let entered = game
        .object(result.new_id)
        .expect("The Mimeoplasm permanent should exist");
    assert_eq!(entered.name, "The Mimeoplasm");
    assert!(entered.counters.is_empty());
    assert!(
        game.object(creature_id)
            .is_some_and(|object| object.zone == Zone::Graveyard)
    );
    assert!(
        game.object(noncreature_id)
            .is_some_and(|object| object.zone == Zone::Graveyard)
    );
    assert!(
        game.object(token_id)
            .is_some_and(|object| object.zone == Zone::Graveyard)
    );
    assert!(game.get_exiled_with_source_links(result.new_id).is_empty());
}

#[test]
pub(super) fn resolving_ability_from_spell_on_stack_does_not_move_source_spell() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let spell = CardDefinitionBuilder::new(CardId::new(), "Stack Ability Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .card_types(vec![CardType::Instant])
        .with_spell_effect(vec![Effect::gain_life(3)])
        .build();
    let spell_id = game.create_object_from_definition(&spell, alice, Zone::Stack);

    game.push_to_stack(StackEntry::new(spell_id, alice));
    game.push_to_stack(StackEntry::ability(
        spell_id,
        alice,
        vec![Effect::gain_life(2)],
    ));

    let mut dm = SelectFirstDecisionMaker;
    resolve_stack_entry_with(&mut game, &mut dm).expect("stack ability should resolve");

    assert_eq!(game.player(alice).expect("Alice should exist").life, 22);
    assert!(
        game.object(spell_id)
            .is_some_and(|object| object.zone == Zone::Stack),
        "source spell should remain on the stack after its ability resolves"
    );
    assert_eq!(game.stack.len(), 1);
    assert!(!game.stack[0].is_ability);

    resolve_stack_entry_with(&mut game, &mut dm).expect("spell should resolve");

    assert_eq!(game.player(alice).expect("Alice should exist").life, 25);
    assert!(
        game.player(alice).is_some_and(|player| {
            player.graveyard.iter().any(|id| {
                game.object(*id)
                    .is_some_and(|object| object.name == "Stack Ability Probe")
            })
        }),
        "spell should move to its owner's graveyard after the spell entry resolves"
    );
}

#[test]
pub(super) fn test_resolve_stack_entry_with_graveyard_object_target() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source_card = CardBuilder::new(CardId::from_raw(9001), "Reanimation Source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let source_id = game.create_object_from_card(&source_card, alice, Zone::Battlefield);

    let target_card = CardBuilder::new(CardId::from_raw(9002), "Graveyard Target")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let target_id = game.create_object_from_card(&target_card, alice, Zone::Graveyard);

    let target_spec = ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::permanent().in_zone(Zone::Graveyard),
    ));
    let effects = vec![Effect::return_from_graveyard_to_battlefield(
        target_spec,
        false,
    )];

    let entry = StackEntry::ability(source_id, alice, effects)
        .with_targets(vec![Target::Object(target_id)]);
    game.push_to_stack(entry);

    resolve_stack_entry(&mut game).expect("stack entry should resolve");

    assert!(
        game.players[0].graveyard.is_empty(),
        "target card should leave graveyard"
    );
    assert!(
        game.battlefield.iter().any(|id| {
            game.object(*id)
                .is_some_and(|obj| obj.name == "Graveyard Target")
        }),
        "target card should be returned to battlefield"
    );
}

#[test]
pub(super) fn test_resolution_target_validation_uses_source_lki_for_protection() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let source_card = CardBuilder::new(CardId::from_raw(9003), "Departed Red Source")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .card_types(vec![CardType::Instant])
        .build();
    let source_id = game.create_object_from_card(&source_card, alice, Zone::Stack);
    let source_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(source_id).expect("source should exist"),
        &game,
    );

    let protected_id = create_creature(&mut game, "Protected Creature", bob, 2, 2);
    game.object_mut(protected_id)
        .expect("protected creature should exist")
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::protection(
            crate::ability::ProtectionFrom::Color(crate::color::ColorSet::RED),
        )));
    game.refresh_continuous_state();

    game.remove_object(source_id);

    let entry = StackEntry::ability(
        source_id,
        alice,
        vec![Effect::deal_damage(1, ChooseSpec::AnyTarget)],
    )
    .with_targets(vec![Target::Object(protected_id)])
    .with_source_snapshot(source_snapshot);

    let (valid_targets, _, all_targets_invalid) = validate_stack_entry_targets(&game, &entry);
    assert!(
        valid_targets.is_empty(),
        "protection from red should make the target illegal using the departed source's LKI"
    );
    assert!(
        all_targets_invalid,
        "the ability should fizzle when its only target is illegal under source LKI"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn emrakul_the_world_anew_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(124_010), "Emrakul, the World Anew")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(12)]]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Eldrazi])
        .power_toughness(PowerToughness::fixed(12, 12))
        .parse_text(
            "When you cast this spell, gain control of all creatures target player controls.\n\
             Flying, protection from spells and from permanents that were cast this turn\n\
             When Emrakul leaves the battlefield, sacrifice all creatures you control.\n\
             Madness—Pay six {C}.",
        )
        .expect("Emrakul, the World Anew should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn stage_spell_cast_for_test(
    game: &mut GameState,
    object_id: ObjectId,
    caster: PlayerId,
) {
    let event = TriggerEvent::new_with_provenance(
        SpellCastEvent::new(object_id, caster, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    game.stage_turn_history_event(&event);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn emrakul_the_world_anew_cast_trigger_gains_control_of_target_players_creatures() {
    #[derive(Debug)]
    struct ChoosePlayerDecisionMaker {
        player: PlayerId,
    }

    impl DecisionMaker for ChoosePlayerDecisionMaker {
        fn decide_targets(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            ctx.requirements
                .iter()
                .map(|requirement| {
                    let target = Target::Player(self.player);
                    assert!(
                        requirement.legal_targets.contains(&target),
                        "chosen Emrakul target player should be legal"
                    );
                    target
                })
                .collect()
        }
    }

    let mut game = setup_three_player_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);

    let bob_first = create_creature(&mut game, "Bob Creature One", bob, 2, 2);
    let bob_second = create_creature(&mut game, "Bob Creature Two", bob, 3, 3);
    let charlie_creature = create_creature(&mut game, "Charlie Creature", charlie, 4, 4);
    let alice_creature = create_creature(&mut game, "Alice Creature", alice, 1, 1);

    let emrakul = emrakul_the_world_anew_definition();
    let emrakul_id = game.create_object_from_definition(&emrakul, alice, Zone::Stack);
    let (emrakul_stable_id, emrakul_name) = game
        .object(emrakul_id)
        .map(|object| (object.stable_id, object.name.to_string()))
        .expect("Emrakul spell should exist on the stack");
    game.push_to_stack(
        StackEntry::new(emrakul_id, alice).with_source_info(emrakul_stable_id, emrakul_name),
    );

    let event = TriggerEvent::new_with_provenance(
        SpellCastEvent::new(emrakul_id, alice, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    queue_triggers_from_event(&mut game, &mut trigger_queue, event, false);
    put_triggers_on_stack_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut ChoosePlayerDecisionMaker { player: bob },
    )
    .expect("Emrakul's cast trigger should go on the stack");

    resolve_stack_entry(&mut game).expect("Emrakul's cast trigger should resolve");
    game.refresh_continuous_state();

    assert_eq!(game.current_controller(bob_first), Some(alice));
    assert_eq!(game.current_controller(bob_second), Some(alice));
    assert_eq!(game.current_controller(charlie_creature), Some(charlie));
    assert_eq!(game.current_controller(alice_creature), Some(alice));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn emrakul_the_world_anew_leaves_trigger_sacrifices_only_your_creatures() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let emrakul = emrakul_the_world_anew_definition();
    let emrakul_id = game.create_object_from_definition(&emrakul, alice, Zone::Battlefield);
    let alice_first = create_creature(&mut game, "Alice Creature One", alice, 2, 2);
    let alice_second = create_creature(&mut game, "Alice Creature Two", alice, 3, 3);
    let bob_creature = create_creature(&mut game, "Bob Creature", bob, 4, 4);

    game.move_object_by_effect(emrakul_id, Zone::Graveyard)
        .expect("Emrakul should move to the graveyard");
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Emrakul's leaves trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("Emrakul's leaves trigger should resolve");

    assert!(!game.battlefield.contains(&alice_first));
    assert!(!game.battlefield.contains(&alice_second));
    assert!(game.battlefield.contains(&bob_creature));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn emrakul_the_world_anew_protection_rejects_spell_targets() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let emrakul = emrakul_the_world_anew_definition();
    let emrakul_id = game.create_object_from_definition(&emrakul, bob, Zone::Battlefield);
    game.refresh_continuous_state();

    let spell = CardDefinitionBuilder::new(CardId::from_raw(124_011), "Targeting Spell")
        .card_types(vec![CardType::Instant])
        .parse_text("Destroy target creature.")
        .expect("targeted spell should parse");
    let spell_id = game.create_object_from_definition(&spell, alice, Zone::Stack);
    assert!(
        crate::targeting::has_protection_from_source(&game, emrakul_id, spell_id),
        "Emrakul should have protection from spell sources"
    );
    let entry = StackEntry::new(spell_id, alice).with_targets(vec![Target::Object(emrakul_id)]);

    let (valid_targets, _, all_targets_invalid) = validate_stack_entry_targets(&game, &entry);
    assert!(
        valid_targets.is_empty(),
        "Emrakul should have protection from spells"
    );
    assert!(
        all_targets_invalid,
        "a spell with only Emrakul as target should fizzle"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn emrakul_the_world_anew_protection_only_rejects_permanents_cast_this_turn() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let emrakul = emrakul_the_world_anew_definition();
    let emrakul_id = game.create_object_from_definition(&emrakul, bob, Zone::Battlefield);
    game.refresh_continuous_state();

    let cast_source = create_creature(&mut game, "Fresh Ability Source", alice, 2, 2);
    stage_spell_cast_for_test(&mut game, cast_source, alice);
    let cast_entry = StackEntry::ability(
        cast_source,
        alice,
        vec![Effect::deal_damage(1, ChooseSpec::AnyTarget)],
    )
    .with_targets(vec![Target::Object(emrakul_id)]);
    let (cast_valid_targets, _, cast_all_invalid) =
        validate_stack_entry_targets(&game, &cast_entry);
    assert!(
        cast_valid_targets.is_empty(),
        "Emrakul should have protection from permanents that were cast this turn"
    );
    assert!(cast_all_invalid);

    let old_source = create_creature(&mut game, "Old Ability Source", alice, 2, 2);
    let old_entry = StackEntry::ability(
        old_source,
        alice,
        vec![Effect::deal_damage(1, ChooseSpec::AnyTarget)],
    )
    .with_targets(vec![Target::Object(emrakul_id)]);
    let (old_valid_targets, _, old_all_invalid) = validate_stack_entry_targets(&game, &old_entry);
    assert_eq!(
        old_valid_targets,
        vec![crate::effects::ResolvedTarget::Object(emrakul_id)],
        "Emrakul should not have protection from permanents that were not cast this turn"
    );
    assert!(!old_all_invalid);
}

#[test]
pub(super) fn test_resolution_player_target_validation_uses_source_lki_for_source_filter() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let source_card = CardBuilder::new(CardId::from_raw(9004), "Departed Red Spell")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .card_types(vec![CardType::Instant])
        .build();
    let source_id = game.create_object_from_card(&source_card, alice, Zone::Stack);
    let source_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(source_id).expect("source should exist"),
        &game,
    );
    game.effect_store
        .cant_effects
        .cant_target_players_from
        .push(crate::game_state::PlayerCantBeTargetedFrom {
            player: bob,
            source_filter: ObjectFilter::default().with_colors(crate::color::ColorSet::RED),
            controller: bob,
        });

    game.remove_object(source_id);

    let entry = StackEntry::ability(
        source_id,
        alice,
        vec![Effect::deal_damage(
            1,
            ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Any)),
        )],
    )
    .with_targets(vec![Target::Player(bob)])
    .with_source_snapshot(source_snapshot);

    let (valid_targets, _, all_targets_invalid) = validate_stack_entry_targets(&game, &entry);
    assert!(
        valid_targets.is_empty(),
        "player target restrictions from red sources should use departed source LKI"
    );
    assert!(all_targets_invalid);
}

#[test]
pub(super) fn test_stack_entry_captures_source_lki_when_ability_source_leaves_before_resolution() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let source_card = CardBuilder::new(CardId::from_raw(9005), "Red Ability Source")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let source_id = game.create_object_from_card(&source_card, alice, Zone::Battlefield);

    let protected_id = create_creature(&mut game, "Protected Creature", bob, 2, 2);
    game.object_mut(protected_id)
        .expect("protected creature should exist")
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::protection(
            crate::ability::ProtectionFrom::Color(crate::color::ColorSet::RED),
        )));
    game.refresh_continuous_state();

    let entry = StackEntry::ability(
        source_id,
        alice,
        vec![Effect::deal_damage(1, ChooseSpec::AnyTarget)],
    )
    .with_targets(vec![Target::Object(protected_id)]);
    game.push_to_stack(entry);

    game.move_object_by_effect(source_id, Zone::Graveyard)
        .expect("source should leave before its ability resolves");

    resolve_stack_entry(&mut game).expect("ability should resolve using source LKI");

    assert_eq!(
        game.damage_on(protected_id),
        0,
        "protection from red should make the target illegal using source LKI captured when the ability went on the stack"
    );
}

#[test]
pub(super) fn test_resolution_uses_source_lki_from_when_source_left_expected_zone() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let source_id = create_creature(&mut game, "Late Deathtouch Source", alice, 1, 1);
    let target_id = create_creature(&mut game, "Large Target", bob, 3, 3);

    let entry = StackEntry::ability(
        source_id,
        alice,
        vec![Effect::deal_damage(1, ChooseSpec::AnyTarget)],
    )
    .with_targets(vec![Target::Object(target_id)]);
    game.push_to_stack(entry);

    game.object_mut(source_id)
        .expect("source should still be on the battlefield")
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::deathtouch()));
    game.refresh_continuous_state();
    game.move_object_by_effect(source_id, Zone::Graveyard)
        .expect("source should leave before its ability resolves");

    resolve_stack_entry(&mut game).expect("ability should resolve using source LKI");

    let mut trigger_queue = TriggerQueue::new();
    check_and_apply_sbas(&mut game, &mut trigger_queue).expect("SBAs should apply");

    assert!(
        !game.battlefield.contains(&target_id),
        "113.7a/608.2h and 702.2e require the damage to use source LKI from when it left, including the late deathtouch"
    );
}

#[test]
pub(super) fn test_resolution_uses_source_lki_for_generic_power_of_source() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source_id = create_creature(&mut game, "Departed Power Source", alice, 4, 4);
    let entry = StackEntry::ability(
        source_id,
        alice,
        vec![Effect::gain_life(Value::PowerOf(Box::new(
            ChooseSpec::Source,
        )))],
    );
    game.push_to_stack(entry);

    game.move_object_by_effect(source_id, Zone::Graveyard)
        .expect("source should leave before its ability resolves");

    resolve_stack_entry(&mut game).expect("ability should resolve using source LKI");

    assert_eq!(
        game.player(alice).expect("Alice should exist").life,
        24,
        "608.2h requires generic source-referential object values to use source LKI after the source leaves"
    );
}

#[test]
pub(super) fn test_resolution_uses_source_lki_for_dynamic_token_count() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let source_id = create_creature(&mut game, "Departed Token Source", alice, 1, 1);
    game.object_mut(source_id)
        .expect("source should exist")
        .add_counters(crate::object::CounterType::PlusOnePlusOne, 1);
    let token = CardDefinitionBuilder::new(CardId::new(), "Vampire")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Vampire])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let entry = StackEntry::ability(
        source_id,
        alice,
        vec![Effect::new(crate::effects::CreateTokenEffect::you(
            token,
            Value::PowerOf(Box::new(ChooseSpec::Source)),
        ))],
    );
    game.push_to_stack(entry);

    game.move_object_by_effect(source_id, Zone::Graveyard)
        .expect("source should leave before its ability resolves");

    resolve_stack_entry(&mut game).expect("ability should resolve using source LKI");

    let tokens = game
        .battlefield
        .iter()
        .filter(|id| game.object(**id).is_some_and(|obj| obj.name == "Vampire"))
        .count();
    assert_eq!(
        tokens, 2,
        "dynamic token counts should resolve PowerOf(Source) from source LKI after the source leaves"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parsed_dies_token_count_uses_source_lki_after_prior_counter_trigger() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let def = CardDefinitionBuilder::new(CardId::new(), "Elenda Probe")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Vampire])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "Whenever another creature dies, put a +1/+1 counter on Elenda Probe.\nWhen this creature dies, create X 1/1 white Vampire creature tokens with lifelink, where X is Elenda Probe's power.",
        )
        .expect("Elenda-like text should parse");

    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let other = create_creature(&mut game, "Other Creature", bob, 2, 2);

    game.move_object_by_effect(other, Zone::Graveyard)
        .expect("other creature should die");
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    put_triggers_on_stack(&mut game, &mut trigger_queue).expect("counter trigger should stack");
    while !game.stack_is_empty() {
        resolve_stack_entry(&mut game).expect("counter trigger should resolve");
    }
    assert_eq!(
        game.object(source)
            .expect("source should remain on battlefield")
            .counters
            .get(&crate::object::CounterType::PlusOnePlusOne)
            .copied(),
        Some(1),
        "the first dies trigger should put a +1/+1 counter on the source"
    );

    game.move_object_by_effect(source, Zone::Graveyard)
        .expect("source should die");
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    put_triggers_on_stack(&mut game, &mut trigger_queue).expect("dies token trigger should stack");
    assert_eq!(
        game.stack
            .last()
            .and_then(|entry| entry.source_snapshot.as_ref())
            .and_then(|snapshot| snapshot.power),
        Some(2),
        "the queued dies trigger should carry source LKI including counters"
    );
    while !game.stack_is_empty() {
        resolve_stack_entry(&mut game).expect("token trigger should resolve");
    }

    let tokens = game
        .battlefield
        .iter()
        .filter(|id| game.object(**id).is_some_and(|obj| obj.name == "Vampire"))
        .count();
    assert_eq!(
        tokens, 2,
        "Elenda-like dies triggers should create tokens from the source's LKI power"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn boss_s_chauffeur_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(671_495), "Boss's Chauffeur")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elf, Subtype::Citizen])
        .power_toughness(PowerToughness::fixed(0, 0))
        .parse_text(
            "This creature enters with a number of +1/+1 counters on it equal to one plus the number of other creatures you control.\n\
             Alliance — Whenever another creature you control enters, put a +1/+1 counter on this creature.\n\
             When this creature dies, create a 1/1 green and white Citizen creature token for each +1/+1 counter on it.",
        )
        .expect("Boss's Chauffeur should parse strictly")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn plus_one_counters(game: &GameState, object_id: ObjectId) -> u32 {
    game.object(object_id)
        .expect("object should exist")
        .counters
        .get(&crate::object::CounterType::PlusOnePlusOne)
        .copied()
        .unwrap_or(0)
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn citizen_token_count(game: &GameState) -> usize {
    game.battlefield
        .iter()
        .filter(|id| {
            game.object(**id)
                .is_some_and(|object| object.name == "Citizen")
        })
        .count()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn put_boss_s_chauffeur_onto_battlefield(
    game: &mut GameState,
    definition: &crate::cards::CardDefinition,
    controller: PlayerId,
) -> ObjectId {
    let object_id = game.create_object_from_definition(definition, controller, Zone::Hand);
    game.move_object_with_etb_processing(object_id, Zone::Battlefield)
        .expect("Boss's Chauffeur should move onto the battlefield")
        .new_id
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn put_plain_creature_onto_battlefield(
    game: &mut GameState,
    name: &str,
    controller: PlayerId,
) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let object_id = game.create_object_from_card(&card, controller, Zone::Hand);
    game.move_object_with_etb_processing(object_id, Zone::Battlefield)
        .expect("test creature should move onto the battlefield")
        .new_id
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn boss_s_chauffeur_enters_with_one_plus_other_creatures_you_control() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let chauffeur = boss_s_chauffeur_definition();

    create_creature(&mut game, "Alice Creature One", alice, 2, 2);
    create_creature(&mut game, "Alice Creature Two", alice, 2, 2);
    create_creature(&mut game, "Bob Creature", bob, 2, 2);
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("no Boss's Chauffeur triggers should exist before it enters");
    assert!(game.stack_is_empty());

    let chauffeur_id = put_boss_s_chauffeur_onto_battlefield(&mut game, &chauffeur, alice);
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Boss's Chauffeur should not trigger on itself entering");

    assert!(
        game.stack_is_empty(),
        "Boss's Chauffeur's Alliance trigger should not count its own enter event"
    );
    assert_eq!(
        plus_one_counters(&game, chauffeur_id),
        3,
        "Boss's Chauffeur should enter with one plus Alice's two other creatures, ignoring itself and Bob's creature"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn boss_s_chauffeur_alliance_triggers_only_for_another_creature_you_control() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let chauffeur = boss_s_chauffeur_definition();
    let chauffeur_id = put_boss_s_chauffeur_onto_battlefield(&mut game, &chauffeur, alice);

    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Boss's Chauffeur should not trigger on itself entering");
    assert!(game.stack_is_empty());
    assert_eq!(plus_one_counters(&game, chauffeur_id), 1);

    put_plain_creature_onto_battlefield(&mut game, "Opponent Creature", bob);
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("opponent creature enter should not create an Alliance trigger");
    assert!(
        game.stack_is_empty(),
        "Boss's Chauffeur should not trigger for a creature an opponent controls"
    );
    assert_eq!(plus_one_counters(&game, chauffeur_id), 1);

    put_plain_creature_onto_battlefield(&mut game, "Alice Followup Creature", alice);
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Alice's other creature should create an Alliance trigger");
    assert!(
        !game.stack_is_empty(),
        "Boss's Chauffeur should trigger for another creature Alice controls"
    );
    while !game.stack_is_empty() {
        resolve_stack_entry(&mut game).expect("Alliance trigger should resolve");
    }

    assert_eq!(
        plus_one_counters(&game, chauffeur_id),
        2,
        "Boss's Chauffeur should get one +1/+1 counter from the Alliance trigger"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn boss_s_chauffeur_dies_creates_citizens_for_each_plus_one_counter_on_it() {
    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);
    let chauffeur = boss_s_chauffeur_definition();

    create_creature(&mut game, "Alice Creature One", alice, 2, 2);
    create_creature(&mut game, "Alice Creature Two", alice, 2, 2);
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("setup creatures should not create Boss's Chauffeur triggers");
    assert!(game.stack_is_empty());

    let chauffeur_id = put_boss_s_chauffeur_onto_battlefield(&mut game, &chauffeur, alice);
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Boss's Chauffeur should not trigger on itself entering");
    assert_eq!(plus_one_counters(&game, chauffeur_id), 3);

    put_plain_creature_onto_battlefield(&mut game, "Alice Followup Creature", alice);
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    put_triggers_on_stack(&mut game, &mut trigger_queue).expect("Alliance trigger should stack");
    while !game.stack_is_empty() {
        resolve_stack_entry(&mut game).expect("Alliance trigger should resolve");
    }
    assert_eq!(plus_one_counters(&game, chauffeur_id), 4);

    game.move_object_by_effect(chauffeur_id, Zone::Graveyard)
        .expect("Boss's Chauffeur should die");
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Boss's Chauffeur dies trigger should stack");
    while !game.stack_is_empty() {
        resolve_stack_entry(&mut game).expect("dies trigger should resolve");
    }

    assert_eq!(
        citizen_token_count(&game),
        4,
        "Boss's Chauffeur should create one Citizen token for each +1/+1 counter it had using source LKI"
    );
}
