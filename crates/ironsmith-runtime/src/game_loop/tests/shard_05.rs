#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::shard_02::*;
use super::shard_03::*;
use super::shard_04::*;
use super::shard_06::*;
use super::shard_07::*;
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
#[test]
pub(super) fn roshan_hidden_magister_applies_assassin_subtype_across_zones_for_you_only() {
    use crate::cards::builders::CardDefinitionBuilder;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let roshan = CardDefinitionBuilder::new(CardId::new(), "Roshan, Hidden Magister")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Assassin])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "Other creatures you control are Assassins in addition to their other types. The same is true for creature spells you control and creature cards you own that aren't on the battlefield.\nFace-down creatures you control have menace.\nWhenever a permanent you control is turned face up, you draw a card and you lose 1 life.",
        )
        .expect("Roshan text should parse");
    let _roshan_id = game.create_object_from_definition(&roshan, alice, Zone::Battlefield);

    let draw_probe = CardBuilder::new(CardId::new(), "Roshan Draw Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let _ = game.create_object_from_card(&draw_probe, alice, Zone::Library);

    let ally_creature = CardBuilder::new(CardId::new(), "Roshan Ally")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Bear])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let ally_id = game.create_object_from_card(&ally_creature, alice, Zone::Battlefield);

    let opponent_creature = CardBuilder::new(CardId::new(), "Roshan Opponent")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Bear])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let opponent_id = game.create_object_from_card(&opponent_creature, bob, Zone::Battlefield);

    let ally_gy_card = CardBuilder::new(CardId::new(), "Roshan GY")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Bear])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let ally_gy_id = game.create_object_from_card(&ally_gy_card, alice, Zone::Graveyard);

    assert!(
        game.calculated_subtypes(ally_id)
            .contains(&Subtype::Assassin),
        "other creature you control should gain Assassin"
    );
    assert!(
        game.calculated_subtypes(ally_gy_id)
            .contains(&Subtype::Assassin),
        "creature card you own off battlefield should gain Assassin"
    );
    assert!(
        !game
            .calculated_subtypes(opponent_id)
            .contains(&Subtype::Assassin),
        "opponent creatures should not gain Assassin"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn leyline_of_transformation_applies_chosen_type_across_its_three_scopes() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let leyline = CardDefinitionBuilder::new(CardId::new(), "Leyline of Transformation")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "If this card is in your opening hand, you may begin the game with it on the battlefield.\nAs this enchantment enters, choose a creature type.\nCreatures you control are the chosen type in addition to their other types. The same is true for creature spells you control and creature cards you own that aren't on the battlefield.",
        )
        .expect("Leyline of Transformation text should parse");
    let leyline_in_hand = game.create_object_from_definition(&leyline, alice, Zone::Hand);
    let mut dm = SelectFirstDecisionMaker;
    let leyline_id = game
        .move_object_with_etb_processing_with_dm(leyline_in_hand, Zone::Battlefield, &mut dm)
        .expect("Leyline should enter and record its creature-type choice")
        .new_id;
    let chosen_type = game
        .chosen_creature_type(leyline_id)
        .expect("Leyline should choose a creature type as it enters");

    let ally = CardBuilder::new(CardId::new(), "Leyline Ally")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Bear])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let ally_id = game.create_object_from_card(&ally, alice, Zone::Battlefield);

    let opponent = CardBuilder::new(CardId::new(), "Leyline Opponent")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Bear])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let opponent_id = game.create_object_from_card(&opponent, bob, Zone::Battlefield);

    let creature_spell = CardBuilder::new(CardId::new(), "Leyline Stack Creature")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elf])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let creature_spell_id = game.create_object_from_card(&creature_spell, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(creature_spell_id, alice));

    let noncreature_spell = CardBuilder::new(CardId::new(), "Leyline Stack Instant")
        .card_types(vec![CardType::Instant])
        .build();
    let noncreature_spell_id = game.create_object_from_card(&noncreature_spell, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(noncreature_spell_id, alice));

    let opponent_creature_spell =
        CardBuilder::new(CardId::new(), "Leyline Opposing Stack Creature")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Elf])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
    let opponent_creature_spell_id =
        game.create_object_from_card(&opponent_creature_spell, bob, Zone::Stack);
    game.push_to_stack(StackEntry::new(opponent_creature_spell_id, bob));

    let graveyard_creature = CardBuilder::new(CardId::new(), "Leyline Graveyard Creature")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Wizard])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let graveyard_creature_id =
        game.create_object_from_card(&graveyard_creature, alice, Zone::Graveyard);

    let opponent_graveyard_creature =
        CardBuilder::new(CardId::new(), "Leyline Opposing Graveyard Creature")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Wizard])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
    let opponent_graveyard_creature_id =
        game.create_object_from_card(&opponent_graveyard_creature, bob, Zone::Graveyard);

    let graveyard_noncreature = CardBuilder::new(CardId::new(), "Leyline Graveyard Instant")
        .card_types(vec![CardType::Instant])
        .build();
    let graveyard_noncreature_id =
        game.create_object_from_card(&graveyard_noncreature, alice, Zone::Graveyard);

    assert!(
        game.current_has_subtype(ally_id, chosen_type),
        "creatures you control should gain Leyline's chosen type"
    );
    assert!(
        game.current_has_subtype(ally_id, Subtype::Bear),
        "Leyline should add to, not replace, existing creature types"
    );
    assert!(
        game.current_has_subtype(creature_spell_id, chosen_type),
        "creature spells you control should gain Leyline's chosen type"
    );
    assert!(
        game.current_has_subtype(graveyard_creature_id, chosen_type),
        "creature cards you own off the battlefield should gain Leyline's chosen type"
    );
    assert!(
        !game.current_has_subtype(opponent_id, chosen_type),
        "opposing creatures should not gain Leyline's chosen type"
    );
    assert!(
        !game.current_has_subtype(noncreature_spell_id, chosen_type),
        "noncreature spells should not gain Leyline's chosen type"
    );
    assert!(
        !game.current_has_subtype(opponent_creature_spell_id, chosen_type),
        "creature spells controlled by opponents should not gain Leyline's chosen type"
    );
    assert!(
        !game.current_has_subtype(opponent_graveyard_creature_id, chosen_type),
        "creature cards opponents own off the battlefield should not gain Leyline's chosen type"
    );
    assert!(
        !game.current_has_subtype(graveyard_noncreature_id, chosen_type),
        "noncreature cards off the battlefield should not gain Leyline's chosen type"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn roshan_hidden_magister_face_up_trigger_only_for_your_permanents() {
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::decision::{LegalAction, SelectFirstDecisionMaker};
    use crate::special_actions::TurnFaceUpMethod;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let roshan = CardDefinitionBuilder::new(CardId::new(), "Roshan, Hidden Magister")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Assassin])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "Other creatures you control are Assassins in addition to their other types. The same is true for creature spells you control and creature cards you own that aren't on the battlefield.\nFace-down creatures you control have menace.\nWhenever a permanent you control is turned face up, you draw a card and you lose 1 life.",
        )
        .expect("Roshan text should parse");
    let _roshan_id = game.create_object_from_definition(&roshan, alice, Zone::Battlefield);

    let draw_probe = CardBuilder::new(CardId::new(), "Roshan Draw Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let _ = game.create_object_from_card(&draw_probe, alice, Zone::Library);

    let morph = CardBuilder::new(CardId::new(), "Roshan Morph")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let alice_morph_id = game.create_object_from_card(&morph, alice, Zone::Battlefield);
    let bob_morph_id = game.create_object_from_card(&morph, bob, Zone::Battlefield);
    for id in [alice_morph_id, bob_morph_id] {
        if let Some(obj) = game.object_mut(id) {
            obj.abilities_mut()
                .push(Ability::static_ability(StaticAbility::morph(
                    crate::cost::TotalCost::mana(ManaCost::from_pips(vec![vec![
                        ManaSymbol::Green,
                    ]])),
                )));
        }
        game.set_face_down(id);
    }

    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Green, 1);
    game.player_mut(bob)
        .expect("bob exists")
        .mana_pool
        .add(ManaSymbol::Green, 1);

    let alice_hand_before = game.player(alice).expect("alice exists").hand.len();
    let alice_life_before = game.player(alice).expect("alice exists").life;

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let turn_up_alice = PriorityResponse::PriorityAction(LegalAction::TurnFaceUp {
        creature_id: alice_morph_id,
        method: TurnFaceUpMethod::TurnFaceUpAbility,
    });
    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &turn_up_alice,
        &mut dm,
    )
    .expect("turning your face-down permanent up should succeed");
    resolve_stack_entry(&mut game)
        .expect("Roshan trigger should resolve after your permanent turns up");

    assert_eq!(
        game.player(alice).expect("alice exists").hand.len(),
        alice_hand_before + 1,
        "Roshan should draw one card when your permanent turns face up"
    );
    assert_eq!(
        game.player(alice).expect("alice exists").life,
        alice_life_before - 1,
        "Roshan should make you lose 1 life when your permanent turns face up"
    );

    game.turn.active_player = bob;
    game.turn.priority_player = Some(bob);
    let turn_up_bob = PriorityResponse::PriorityAction(LegalAction::TurnFaceUp {
        creature_id: bob_morph_id,
        method: TurnFaceUpMethod::TurnFaceUpAbility,
    });
    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &turn_up_bob,
        &mut dm,
    )
    .expect("opponent should be able to turn their permanent face up");

    assert!(
        game.stack_is_empty(),
        "Roshan should not trigger when an opponent-controlled permanent is turned face up"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_experiment_twelve_puts_counters_when_itself_is_turned_face_up() {
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::decision::{LegalAction, SelectFirstDecisionMaker};
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::special_actions::TurnFaceUpMethod;
    use crate::static_abilities::StaticAbility;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let experiment = CardDefinitionBuilder::new(CardId::new(), "Experiment Twelve")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)], vec![ManaSymbol::Green]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "Trample\nWhenever this creature or another creature you control is turned face up, put +1/+1 counters on that creature equal to its power.\nDisguise {4}{G}",
        )
        .expect("Experiment Twelve should parse for runtime test");

    let experiment_id = game.create_object_from_definition(&experiment, alice, Zone::Battlefield);
    game.object_mut(experiment_id)
        .expect("Experiment Twelve permanent should exist")
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::morph(
            crate::cost::TotalCost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::Green]])),
        )));
    game.set_face_down(experiment_id);

    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Green, 1);

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let response = PriorityResponse::PriorityAction(LegalAction::TurnFaceUp {
        creature_id: experiment_id,
        method: TurnFaceUpMethod::TurnFaceUpAbility,
    });
    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &response,
        &mut dm,
    )
    .expect("turning Experiment Twelve face up should succeed");

    while !game.stack_is_empty() {
        resolve_stack_entry(&mut game).expect("resolve Experiment Twelve trigger");
    }

    let experiment_obj = game
        .object(experiment_id)
        .expect("Experiment Twelve should remain on battlefield");
    assert_eq!(experiment_obj.power(), Some(8));
    assert_eq!(experiment_obj.toughness(), Some(8));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_experiment_twelve_puts_counters_on_another_turned_face_up_creature() {
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::decision::{LegalAction, SelectFirstDecisionMaker};
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::special_actions::TurnFaceUpMethod;
    use crate::static_abilities::StaticAbility;

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let experiment = CardDefinitionBuilder::new(CardId::new(), "Experiment Twelve")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)], vec![ManaSymbol::Green]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "Trample\nWhenever this creature or another creature you control is turned face up, put +1/+1 counters on that creature equal to its power.\nDisguise {4}{G}",
        )
        .expect("Experiment Twelve should parse for runtime test");
    game.create_object_from_definition(&experiment, alice, Zone::Battlefield);

    let morph_probe = CardDefinitionBuilder::new(CardId::new(), "Face-Up Counter Target")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let morph_id = game.create_object_from_definition(&morph_probe, alice, Zone::Battlefield);
    game.object_mut(morph_id)
        .expect("morph target permanent should exist")
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::morph(
            crate::cost::TotalCost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::Green]])),
        )));
    game.set_face_down(morph_id);

    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Green, 1);

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let response = PriorityResponse::PriorityAction(LegalAction::TurnFaceUp {
        creature_id: morph_id,
        method: TurnFaceUpMethod::TurnFaceUpAbility,
    });
    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &response,
        &mut dm,
    )
    .expect("turning the other creature face up should succeed");

    while !game.stack_is_empty() {
        resolve_stack_entry(&mut game).expect("resolve Experiment Twelve trigger");
    }

    let morph_obj = game
        .object(morph_id)
        .expect("morph target should remain on battlefield");
    assert_eq!(morph_obj.power(), Some(6));
    assert_eq!(morph_obj.toughness(), Some(6));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn ecological_appreciation_puts_two_chosen_cards_back_and_recruits_the_rest() {
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::ids::CardId;

    fn test_creature(name: &str, mana_value: u8) -> crate::cards::CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
                mana_value,
            )]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    struct EcologicalAppreciationDecisionMaker {
        caster: PlayerId,
        opponent: PlayerId,
    }

    impl DecisionMaker for EcologicalAppreciationDecisionMaker {
        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            let legal = ctx
                .candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .collect::<Vec<_>>();

            if ctx.player == self.caster {
                assert_eq!(
                    ctx.min, 0,
                    "Ecological Appreciation search should allow finding fewer than four cards"
                );
                assert_eq!(
                    ctx.max,
                    Some(4),
                    "Ecological Appreciation should look for up to four"
                );
                return legal.into_iter().map(|candidate| candidate.id).collect();
            }

            assert_eq!(
                ctx.player, self.opponent,
                "the opponent should make the divvy choice"
            );
            assert_eq!(ctx.min, 2, "the opponent should choose exactly two cards");
            assert_eq!(
                ctx.max,
                Some(2),
                "the opponent should choose exactly two cards"
            );

            ["Library Alpha", "Graveyard Alpha"]
                .into_iter()
                .map(|wanted_name| {
                    legal
                        .iter()
                        .find(|candidate| {
                            game.object(candidate.id)
                                .is_some_and(|object| object.name == wanted_name)
                        })
                        .map(|candidate| candidate.id)
                        .unwrap_or_else(|| panic!("expected to find {wanted_name} in the divvy"))
                })
                .collect()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let text = "Mana cost: {X}{2}{G}\nType: Sorcery\nSearch your library and graveyard for up to four creature cards with different names that each have mana value X or less and reveal them. An opponent chooses two of those cards. Shuffle the chosen cards into your library and put the rest onto the battlefield. Exile Ecological Appreciation.";
    let ecological_appreciation =
        CardDefinitionBuilder::new(CardId::from_raw(91_001), "Ecological Appreciation")
            .parse_text(text)
            .expect("Ecological Appreciation should parse");

    let source_id =
        game.create_object_from_definition(&ecological_appreciation, alice, Zone::Stack);
    let _library_alpha = game.create_object_from_definition(
        &test_creature("Library Alpha", 1),
        alice,
        Zone::Library,
    );
    let _library_beta =
        game.create_object_from_definition(&test_creature("Library Beta", 2), alice, Zone::Library);
    let _graveyard_alpha = game.create_object_from_definition(
        &test_creature("Graveyard Alpha", 1),
        alice,
        Zone::Graveyard,
    );
    let _graveyard_beta = game.create_object_from_definition(
        &test_creature("Graveyard Beta", 2),
        alice,
        Zone::Graveyard,
    );

    let mut stack_entry = StackEntry::new(source_id, alice);
    stack_entry.x_value = Some(2);
    game.stack.push(stack_entry);

    let mut dm = EcologicalAppreciationDecisionMaker {
        caster: alice,
        opponent: bob,
    };
    resolve_stack_entry_with(&mut game, &mut dm).expect("Ecological Appreciation should resolve");

    let zone_has_name = |ids: &[ObjectId], name: &str| {
        ids.iter()
            .any(|&id| game.object(id).is_some_and(|object| object.name == name))
    };

    assert_eq!(
        zone_has_name(
            &game.player(alice).expect("alice exists").library,
            "Library Alpha"
        ),
        true,
        "the chosen library card should return to the library"
    );
    assert_eq!(
        zone_has_name(
            &game.player(alice).expect("alice exists").library,
            "Graveyard Alpha"
        ),
        true,
        "the chosen graveyard card should also return to the library"
    );
    assert_eq!(
        zone_has_name(&game.battlefield, "Library Beta"),
        true,
        "the unchosen library card should go onto the battlefield"
    );
    assert_eq!(
        zone_has_name(&game.battlefield, "Graveyard Beta"),
        true,
        "the unchosen graveyard card should go onto the battlefield"
    );
    assert_eq!(
        zone_has_name(&game.exile, "Ecological Appreciation"),
        true,
        "Ecological Appreciation should exile itself after resolving"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn elemental_teachings_buries_two_opponent_chosen_lands_and_recruits_the_rest_tapped() {
    struct ElementalTeachingsDecisionMaker {
        caster: PlayerId,
        opponent: PlayerId,
    }

    impl DecisionMaker for ElementalTeachingsDecisionMaker {
        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            let legal = ctx
                .candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .collect::<Vec<_>>();

            if ctx.player == self.caster {
                assert_eq!(
                    ctx.min, 0,
                    "Elemental Teachings searches for up to four lands"
                );
                assert_eq!(
                    ctx.max,
                    Some(4),
                    "Elemental Teachings searches for up to four lands"
                );
                assert!(
                    legal
                        .iter()
                        .all(|candidate| game.object_has_card_type(candidate.id, CardType::Land)),
                    "only land cards should be legal search choices"
                );
                return legal.into_iter().map(|candidate| candidate.id).collect();
            }

            assert_eq!(
                ctx.player, self.opponent,
                "the opponent should make the divvy choice"
            );
            assert_eq!(ctx.min, 2, "the opponent should choose exactly two cards");
            assert_eq!(
                ctx.max,
                Some(2),
                "the opponent should choose exactly two cards"
            );

            ["Plains", "Island"]
                .into_iter()
                .map(|wanted_name| {
                    legal
                        .iter()
                        .find(|candidate| {
                            game.object(candidate.id)
                                .is_some_and(|object| object.name == wanted_name)
                        })
                        .map(|candidate| candidate.id)
                        .unwrap_or_else(|| panic!("expected to find {wanted_name} in the divvy"))
                })
                .collect()
        }
    }

    fn test_land(name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Land])
            .build()
    }

    fn test_creature(name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let text = "Search your library for up to four land cards with different names and reveal them. \
        An opponent chooses two of those cards. Put the chosen cards into your graveyard and the \
        rest onto the battlefield tapped, then shuffle.";

    let elemental_teachings =
        CardDefinitionBuilder::new(CardId::from_raw(91_002), "Elemental Teachings")
            .card_types(vec![CardType::Instant])
            .parse_text(text)
            .expect("Elemental Teachings should parse");

    let source_id = game.create_object_from_definition(&elemental_teachings, alice, Zone::Stack);
    let _plains = game.create_object_from_card(&test_land("Plains"), alice, Zone::Library);
    let _island = game.create_object_from_card(&test_land("Island"), alice, Zone::Library);
    let _swamp = game.create_object_from_card(&test_land("Swamp"), alice, Zone::Library);
    let _forest = game.create_object_from_card(&test_land("Forest"), alice, Zone::Library);
    let _nonland =
        game.create_object_from_card(&test_creature("Elvish Mystic"), alice, Zone::Library);

    game.stack.push(StackEntry::new(source_id, alice));

    let mut dm = ElementalTeachingsDecisionMaker {
        caster: alice,
        opponent: bob,
    };
    resolve_stack_entry_with(&mut game, &mut dm).expect("Elemental Teachings should resolve");

    let zone_has_name = |ids: &[ObjectId], name: &str| {
        ids.iter()
            .any(|&id| game.object(id).is_some_and(|object| object.name == name))
    };
    let battlefield_id = |name: &str| {
        game.battlefield
            .iter()
            .copied()
            .find(|&id| game.object(id).is_some_and(|object| object.name == name))
            .unwrap_or_else(|| panic!("expected {name} on the battlefield"))
    };

    assert!(
        zone_has_name(
            &game.player(alice).expect("alice exists").graveyard,
            "Plains"
        ),
        "the first opponent-chosen land should go to the graveyard"
    );
    assert!(
        zone_has_name(
            &game.player(alice).expect("alice exists").graveyard,
            "Island"
        ),
        "the second opponent-chosen land should go to the graveyard"
    );
    let swamp = battlefield_id("Swamp");
    let forest = battlefield_id("Forest");
    assert!(
        game.is_tapped(swamp),
        "the first unchosen land should enter tapped"
    );
    assert!(
        game.is_tapped(forest),
        "the second unchosen land should enter tapped"
    );
    assert!(
        zone_has_name(
            &game.player(alice).expect("alice exists").library,
            "Elvish Mystic"
        ),
        "nonland cards should remain in the library"
    );
}

// === Target Extraction Tests ===

#[test]
pub(super) fn test_extract_target_spec_single_target() {
    // Destroy effect has single target
    let effect = Effect::destroy(ChooseSpec::creature());

    let extracted = extract_target_spec(&effect).expect("Should extract target");
    assert_eq!(extracted.min_targets, 1);
    assert_eq!(extracted.max_targets, Some(1));
}

#[test]
pub(super) fn test_extract_target_spec_any_number() {
    // Exile with any_number count (using exile_any_number helper)
    let effect = Effect::exile_any_number(ChooseSpec::spell());

    let extracted = extract_target_spec(&effect).expect("Should extract target");
    // ChoiceCount::any_number() returns min: 0, max: None
    assert_eq!(extracted.min_targets, 0, "any_number has min 0");
    assert_eq!(extracted.max_targets, None, "any_number has no max");
}

#[test]
pub(super) fn test_extract_target_spec_no_count() {
    // Exile with no count defaults to single target
    let effect = Effect::exile(ChooseSpec::creature());

    let extracted = extract_target_spec(&effect).expect("Should extract target");
    assert_eq!(extracted.min_targets, 1, "should default to min 1");
    assert_eq!(extracted.max_targets, Some(1), "should default to max 1");
}

#[test]
pub(super) fn test_extract_target_spec_attach_to_iterated_does_not_require_choice() {
    let effect = Effect::attach_objects(
        ChooseSpec::Tagged(crate::tag::TagKey::from("created")),
        ChooseSpec::Iterated,
    );
    assert!(
        extract_target_spec(&effect).is_none(),
        "attaching to an iterated object should resolve automatically without asking for a target"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_extract_target_specs_pump_and_gain_clause_uses_single_target_selection() {
    use crate::cards::CardDefinitionBuilder;

    let def = CardDefinitionBuilder::new(CardId::new(), "Viashino Shanktail Variant")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "{2}{R}, Discard this card: Target attacking creature gets +3/+1 and gains first strike until end of turn.",
            )
            .expect("pump-and-gain clause should parse");

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("expected activated ability");

    let target_specs = activated
        .effects
        .iter()
        .filter_map(extract_target_spec)
        .filter(|extracted| requires_target_selection(extracted.spec))
        .count();

    assert_eq!(
        target_specs, 1,
        "expected a single target selection for combined pump+gain clause, got {target_specs}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_extract_target_specs_target_player_chain_uses_single_shared_target() {
    use crate::cards::CardDefinitionBuilder;

    let def = CardDefinitionBuilder::new(CardId::new(), "Atrocious Experiment Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Target player mills two cards, draws two cards, and loses 2 life.")
        .expect("target-player mill/draw/lose chain should parse");

    let effects = def.spell_effect.expect("expected spell effects");
    let game = setup_game();
    let alice = PlayerId::from_index(0);

    let requirements = extract_target_requirements(&game, &effects, alice, None);
    assert_eq!(
        requirements.len(),
        1,
        "expected exactly one shared target requirement, got {:?}",
        requirements
    );
    assert_eq!(requirements[0].min_targets, 1);
    assert_eq!(requirements[0].max_targets, Some(1));
    assert_eq!(
        requirements[0].legal_targets.len(),
        2,
        "expected both players to be legal targets in a two-player game"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_extract_target_specs_target_player_sacrifice_choice_has_target_requirement() {
    use crate::cards::CardDefinitionBuilder;

    let def = CardDefinitionBuilder::new(CardId::new(), "Sudden Edict Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Target player sacrifices a creature of their choice.")
        .expect("target-player sacrifice-choice clause should parse");

    let effects = def.spell_effect.expect("expected spell effects");
    let game = setup_game();
    let alice = PlayerId::from_index(0);

    let requirements = extract_target_requirements(&game, &effects, alice, None);
    assert_eq!(
        requirements.len(),
        1,
        "expected one target requirement for target-player sacrifice clause, got {:?}",
        requirements
    );
    assert_eq!(requirements[0].min_targets, 1);
    assert_eq!(requirements[0].max_targets, Some(1));
    assert_eq!(
        requirements[0].legal_targets.len(),
        2,
        "expected both players to be legal targets in a two-player game"
    );
}

#[test]
pub(super) fn joint_assault_target_requirement_allows_creature_target() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Joint Assault")
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Green]))
        .card_types(vec![CardType::Instant])
        .parse_text("Target creature gets +2/+2 until end of turn. If it's paired with a creature, that creature also gets +2/+2 until end of turn.")
        .expect("Joint Assault should parse");
    let effects = def.spell_effect.as_ref().expect("expected spell effects");
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let target = CardBuilder::new(CardId::new(), "Elite Vanguard")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 1))
        .build();
    game.create_object_from_card(&target, alice, Zone::Battlefield);

    let requirements =
        extract_target_requirements_from_program_with_modes(&game, effects, alice, None, None);

    assert_eq!(
        requirements.len(),
        1,
        "Joint Assault should require one target creature, got {requirements:?}"
    );
    assert_eq!(requirements[0].legal_targets.len(), 1);
}

#[test]
pub(super) fn joint_assault_is_castable_with_creature_target_and_green_mana() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Joint Assault")
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Green]))
        .card_types(vec![CardType::Instant])
        .parse_text("Target creature gets +2/+2 until end of turn. If it's paired with a creature, that creature also gets +2/+2 until end of turn.")
        .expect("Joint Assault should parse");
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let spell = game.create_object_from_definition(&def, alice, Zone::Hand);
    let target = CardBuilder::new(CardId::new(), "Elite Vanguard")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 1))
        .build();
    game.create_object_from_card(&target, alice, Zone::Battlefield);
    let forest = CardDefinitionBuilder::new(CardId::new(), "Forest")
        .card_types(vec![CardType::Land])
        .parse_text("{T}: Add {G}.")
        .expect("Forest mana ability should parse");
    game.create_object_from_definition(&forest, alice, Zone::Battlefield);

    let effects = game
        .object(spell)
        .and_then(|object| object.spell_effect.as_deref())
        .expect("Joint Assault should have spell effects");
    let requirements = extract_target_requirements(&game, effects, alice, Some(spell));
    assert_eq!(
        requirements.len(),
        1,
        "Joint Assault should require one target creature with source id, got {requirements:?}"
    );
    assert_eq!(
        requirements[0].legal_targets.len(),
        1,
        "Joint Assault should see Elite Vanguard as a legal target with source id, got {requirements:?}"
    );
    let spell_object = game.object(spell).expect("spell should exist");
    let view = crate::derived_view::DerivedGameView::new(&game);
    assert!(
        crate::decision::has_valid_spell_timing_with_view(&game, alice, spell_object, spell, &view),
        "Joint Assault should satisfy spell timing"
    );
    assert!(
        crate::decision::spell_has_legal_targets_for_cast_with_view(
            &game,
            spell_object,
            spell,
            spell_object.spell_effect.as_deref(),
            None,
            alice,
            &view,
        ),
        "Joint Assault should satisfy cast-time target legality"
    );
    let potential = view.potential_mana(alice);
    assert!(
        potential.green >= 1,
        "Forest should provide potential green mana, got {potential:?}"
    );
    assert!(
        crate::decision::can_cast_spell(
            &game,
            alice,
            spell_object,
            &crate::alternative_cast::CastingMethod::Normal,
        ),
        "Joint Assault should pass direct cast legality"
    );

    let actions = crate::decision::compute_legal_actions(&game, alice);

    assert!(
        actions
            .iter()
            .any(|action| matches!(action, crate::decision::LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)),
        "Joint Assault should be castable, actions: {actions:?}"
    );
}

#[test]
pub(super) fn joint_assault_pumps_target_and_its_soulbond_partner() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Joint Assault")
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Green]))
        .card_types(vec![CardType::Instant])
        .parse_text("Target creature gets +2/+2 until end of turn. If it's paired with a creature, that creature also gets +2/+2 until end of turn.")
        .expect("Joint Assault should parse");
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let target = CardBuilder::new(CardId::new(), "Elite Vanguard")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 1))
        .build();
    let partner = CardBuilder::new(CardId::new(), "Trusted Forcemage")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let target_id = game.create_object_from_card(&target, alice, Zone::Battlefield);
    let partner_id = game.create_object_from_card(&partner, alice, Zone::Battlefield);
    game.set_soulbond_pair(target_id, partner_id);

    let spell_id = game.create_object_from_definition(&def, alice, Zone::Stack);
    game.push_to_stack(
        crate::game_state::StackEntry::new(spell_id, alice)
            .with_targets(vec![Target::Object(target_id)])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(
                    crate::filter::ObjectFilter::creature(),
                )),
                range: 0..1,
            }]),
    );

    super::resolve_stack_entry(&mut game).expect("Joint Assault should resolve");

    assert_eq!(game.calculated_power(target_id), Some(4));
    assert_eq!(game.calculated_toughness(target_id), Some(3));
    assert_eq!(game.calculated_power(partner_id), Some(5));
    assert_eq!(game.calculated_toughness(partner_id), Some(5));
}

#[test]
pub(super) fn doom_weaver_grants_dies_trigger_to_soulbond_partner() {
    let doom_weaver = CardDefinitionBuilder::new(CardId::new(), "Doom Weaver")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Spider, Subtype::Horror])
        .power_toughness(PowerToughness::fixed(1, 8))
        .parse_text(
            "Reach\nSoulbond (You may pair this creature with another unpaired creature when either enters. They remain paired for as long as you control both of them.)\nAs long as Doom Weaver is paired with another creature, each of those creatures has \"When this creature dies, draw cards equal to its power.\"",
        )
        .expect("Doom Weaver should parse");

    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    let doom_weaver_id = game.create_object_from_definition(&doom_weaver, alice, Zone::Battlefield);
    for idx in 0..4 {
        let library_card = CardBuilder::new(CardId::new(), format!("Draw Fodder {idx}"))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        game.create_object_from_card(&library_card, alice, Zone::Library);
    }
    let partner = CardBuilder::new(CardId::new(), "Soulbond Partner")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let partner_id = game.create_object_from_card(&partner, alice, Zone::Battlefield);
    game.set_soulbond_pair(doom_weaver_id, partner_id);

    let hand_before = game.player(alice).expect("alice exists").hand.len();
    let moved = game.move_object_by_effect(partner_id, Zone::Graveyard);
    assert!(moved.is_some(), "partner should move to the graveyard");
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "partner death should trigger Doom Weaver grant"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Doom Weaver trigger should be put on stack");

    while !game.stack_is_empty() {
        resolve_stack_entry(&mut game).expect("granted dies trigger should resolve");
    }

    assert_eq!(
        game.player(alice).expect("alice exists").hand.len(),
        hand_before + 3,
        "partner should draw cards equal to its power when it dies while paired"
    );
}

#[test]
pub(super) fn doom_weaver_dies_trigger_not_granted_when_unpaired() {
    let doom_weaver = CardDefinitionBuilder::new(CardId::new(), "Doom Weaver")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Spider, Subtype::Horror])
        .power_toughness(PowerToughness::fixed(1, 8))
        .parse_text(
            "Reach\nSoulbond (You may pair this creature with another unpaired creature when either enters. They remain paired for as long as you control both of them.)\nAs long as Doom Weaver is paired with another creature, each of those creatures has \"When this creature dies, draw cards equal to its power.\"",
        )
        .expect("Doom Weaver should parse");

    let mut game = setup_game();
    let mut trigger_queue = TriggerQueue::new();
    let alice = PlayerId::from_index(0);

    let _doom_weaver_id =
        game.create_object_from_definition(&doom_weaver, alice, Zone::Battlefield);
    for idx in 0..4 {
        let library_card = CardBuilder::new(CardId::new(), format!("No Draw Fodder {idx}"))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        game.create_object_from_card(&library_card, alice, Zone::Library);
    }
    let partner = CardBuilder::new(CardId::new(), "Unpaired Partner")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let partner_id = game.create_object_from_card(&partner, alice, Zone::Battlefield);

    let hand_before = game.player(alice).expect("alice exists").hand.len();
    let moved = game.move_object_by_effect(partner_id, Zone::Graveyard);
    assert!(moved.is_some(), "partner should move to the graveyard");
    drain_pending_trigger_events(&mut game, &mut trigger_queue);
    assert!(
        trigger_queue.entries.is_empty(),
        "unpaired partner death should not trigger draw"
    );
    assert_eq!(
        game.player(alice).expect("alice exists").hand.len(),
        hand_before,
        "unpaired death should not draw cards"
    );
}

#[test]
pub(super) fn test_extract_target_specs_necromentia_uses_one_target_opponent_requirement() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Necromentia")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Choose a card name other than a basic land card name. Search target opponent's graveyard, hand, and library for any number of cards with that name and exile them. That player shuffles, then creates a 2/2 black Zombie creature token for each card exiled from their hand this way.")
        .expect("Necromentia should parse");
    let effects = def.spell_effect.as_ref().expect("expected spell effects");
    let game = setup_game();
    let alice = PlayerId::from_index(0);

    let requirements =
        extract_target_requirements_from_program_with_modes(&game, effects, alice, None, None);

    assert_eq!(
        requirements.len(),
        1,
        "Necromentia should require only its target opponent, got {:?}",
        requirements
    );
    assert_eq!(requirements[0].description, "target");
    assert_eq!(requirements[0].min_targets, 1);
    assert_eq!(requirements[0].max_targets, Some(1));
}

#[test]
pub(super) fn test_extract_target_specs_oracle_target_binding_regressions() {
    struct SpellCase {
        name: &'static str,
        types: Vec<CardType>,
        oracle: &'static str,
        expected_requirements: usize,
        expected_counts: &'static [(usize, Option<usize>)],
    }

    let cases = vec![
        SpellCase {
            name: "Dispossess",
            types: vec![CardType::Sorcery],
            oracle: "Choose an artifact card name. Search target opponent's graveyard, hand, and library for any number of cards with the chosen name and exile them. Then that player shuffles.",
            expected_requirements: 1,
            expected_counts: &[(1, Some(1))],
        },
        SpellCase {
            name: "Cranial Extraction",
            types: vec![CardType::Sorcery],
            oracle: "Choose a nonland card name. Search target player's graveyard, hand, and library for all cards with that name and exile them. Then that player shuffles.",
            expected_requirements: 1,
            expected_counts: &[(1, Some(1))],
        },
        SpellCase {
            name: "Oblation",
            types: vec![CardType::Instant],
            oracle: "The owner of target nonland permanent shuffles it into their library, then draws two cards.",
            expected_requirements: 1,
            expected_counts: &[(1, Some(1))],
        },
        SpellCase {
            name: "Chaos Warp",
            types: vec![CardType::Instant],
            oracle: "The owner of target permanent shuffles it into their library, then reveals the top card of their library. If it's a permanent card, they put it onto the battlefield.",
            expected_requirements: 1,
            expected_counts: &[(1, Some(1))],
        },
        SpellCase {
            name: "Gods Willing",
            types: vec![CardType::Instant],
            oracle: "Target creature you control gains protection from the color of your choice until end of turn. (It can't be blocked, targeted, dealt damage, enchanted, or equipped by anything of that color.)\nScry 1.",
            expected_requirements: 1,
            expected_counts: &[(1, Some(1))],
        },
        SpellCase {
            name: "Arc Trail",
            types: vec![CardType::Sorcery],
            oracle: "Arc Trail deals 2 damage to any target and 1 damage to any other target.",
            expected_requirements: 2,
            expected_counts: &[(1, Some(1)), (1, Some(1))],
        },
        SpellCase {
            name: "Decimate",
            types: vec![CardType::Sorcery],
            oracle: "Destroy target artifact, target creature, target enchantment, and target land. (You can't cast this spell unless you have legal choices for all its targets.)",
            expected_requirements: 4,
            expected_counts: &[(1, Some(1)), (1, Some(1)), (1, Some(1)), (1, Some(1))],
        },
        SpellCase {
            name: "Hex",
            types: vec![CardType::Sorcery],
            oracle: "Destroy six target creatures.",
            expected_requirements: 1,
            expected_counts: &[(6, Some(6))],
        },
    ];

    let alice = PlayerId::from_index(0);

    for case in cases {
        let mut game = setup_game();
        let bob = PlayerId::from_index(1);
        create_creature(&mut game, "Alice Creature", alice, 2, 2);
        for idx in 0..6 {
            create_creature(&mut game, &format!("Bob Creature {idx}"), bob, 2, 2);
        }
        let artifact = CardBuilder::new(CardId::from_raw(1000), "Target Artifact")
            .card_types(vec![CardType::Artifact])
            .build();
        game.create_object_from_card(&artifact, bob, Zone::Battlefield);
        let enchantment = CardBuilder::new(CardId::from_raw(1001), "Target Enchantment")
            .card_types(vec![CardType::Enchantment])
            .build();
        game.create_object_from_card(&enchantment, bob, Zone::Battlefield);
        let land = CardBuilder::new(CardId::from_raw(1002), "Target Land")
            .card_types(vec![CardType::Land])
            .build();
        game.create_object_from_card(&land, bob, Zone::Battlefield);

        let def = CardDefinitionBuilder::new(CardId::new(), case.name)
            .card_types(case.types)
            .parse_text(case.oracle)
            .unwrap_or_else(|err| panic!("{} should parse: {err:?}", case.name));
        let effects = def
            .spell_effect
            .as_ref()
            .unwrap_or_else(|| panic!("{} should have spell effects", case.name));
        let requirements =
            extract_target_requirements_from_program_with_modes(&game, effects, alice, None, None);

        assert_eq!(
            requirements.len(),
            case.expected_requirements,
            "{} should have {} target requirement(s), got {:?}",
            case.name,
            case.expected_requirements,
            requirements
        );
        let counts = requirements
            .iter()
            .map(|requirement| (requirement.min_targets, requirement.max_targets))
            .collect::<Vec<_>>();
        assert_eq!(
            counts, case.expected_counts,
            "{} target counts should match",
            case.name
        );
    }
}

#[test]
pub(super) fn test_extract_target_specs_blood_artist_trigger_uses_one_target_player_binding() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Blood Artist")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever this creature or another creature dies, target player loses 1 life and you gain 1 life.",
        )
        .expect("Blood Artist should parse");
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Blood Artist should have a triggered ability");

    assert_eq!(
        triggered.choices.len(),
        1,
        "Blood Artist should expose one target player choice, got {:?}",
        triggered.choices
    );
    assert_eq!(triggered.choices[0].count().min, 1);
    assert_eq!(triggered.choices[0].count().max, Some(1));
}

pub(super) fn run_exchange_of_words_swapped_myr_moonvessel_dies_trigger_stacks_when_ornithopter_is_sacrificed()
 {
    use crate::ability::AbilityKind;
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::events::zones::EnterBattlefieldEvent;
    use crate::mana::ManaSymbol;
    use crate::provenance::ProvNodeId;

    #[derive(Debug)]
    struct ExchangeTargetsDecisionMaker {
        first: ObjectId,
        second: ObjectId,
    }

    impl DecisionMaker for ExchangeTargetsDecisionMaker {
        fn decide_targets(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            let Some(requirement) = ctx.requirements.first() else {
                return Vec::new();
            };

            [Target::Object(self.first), Target::Object(self.second)]
                .into_iter()
                .filter(|target| requirement.legal_targets.contains(target))
                .collect()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let exchange_def = CardDefinitionBuilder::new(CardId::from_raw(700_201), "Exchange of Words")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "When this enchantment enters, choose two target creatures. For as long as this enchantment remains on the battlefield, exchange the text boxes of those creatures.",
        )
        .expect("Exchange of Words should parse");
    let myr_def = CardDefinitionBuilder::new(CardId::from_raw(700_202), "Myr Moonvessel")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text("When this creature dies, add {C}.")
        .expect("Myr Moonvessel should parse");
    let ornithopter_def = CardDefinitionBuilder::new(CardId::from_raw(700_203), "Ornithopter")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(0, 2))
        .parse_text("Flying")
        .expect("Ornithopter should parse");
    let sacrifice_def = CardDefinitionBuilder::new(CardId::from_raw(700_204), "Sacrifice Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Target player sacrifices a creature of their choice.")
        .expect("targeted sacrifice spell should parse");

    let exchange_trigger = exchange_def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if format!("{:?}", triggered.effects).contains("ExchangeTextBoxesEffect") =>
            {
                Some(triggered.clone())
            }
            _ => None,
        })
        .expect("Exchange of Words should have an ETB text-box exchange trigger");

    let exchange_id = game.create_object_from_definition(&exchange_def, alice, Zone::Battlefield);
    let myr_id = game.create_object_from_definition(&myr_def, alice, Zone::Battlefield);
    let ornithopter_id =
        game.create_object_from_definition(&ornithopter_def, bob, Zone::Battlefield);

    let mut trigger_queue = TriggerQueue::new();
    let etb_event = TriggerEvent::new_with_provenance(
        EnterBattlefieldEvent::new(exchange_id, Zone::Hand),
        ProvNodeId::default(),
    );
    let exchange_stable_id = game
        .object(exchange_id)
        .expect("Exchange of Words should exist")
        .stable_id;
    trigger_queue.add(crate::triggers::TriggeredAbilityEntry {
        source: exchange_id,
        controller: alice,
        x_value: None,
        event_value_amount: None,
        ability: exchange_trigger.clone(),
        triggering_event: etb_event,
        source_stable_id: exchange_stable_id,
        source_name: "Exchange of Words".to_string(),
        source_snapshot: None,
        tagged_objects: std::collections::HashMap::new(),
        source_kind: crate::triggers::TriggeredAbilitySourceKind::Object,
        trigger_identity: crate::triggers::compute_trigger_identity(&exchange_trigger),
    });
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Exchange of Words should queue its ETB trigger"
    );

    let mut dm = ExchangeTargetsDecisionMaker {
        first: myr_id,
        second: ornithopter_id,
    };
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Exchange of Words trigger should go on the stack");
    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("Exchange of Words trigger should resolve");

    game.refresh_continuous_state();

    let ornithopter_chars = game
        .calculated_characteristics(ornithopter_id)
        .expect("Ornithopter should still have calculated characteristics after the exchange");
    assert_eq!(
        ornithopter_chars.compiled_card_text.as_ref(),
        crate::compiled_text::debug_compiled_lines(&myr_def).join("\n"),
        "Ornithopter should now carry Myr Moonvessel's text box"
    );
    assert!(
        ornithopter_chars
            .abilities
            .iter()
            .any(|ability| matches!(ability.kind, AbilityKind::Triggered(_))),
        "Ornithopter should gain the dies trigger from Myr Moonvessel"
    );

    let sacrifice_id = game.create_object_from_definition(&sacrifice_def, alice, Zone::Stack);
    let (sacrifice_stable_id, sacrifice_name) = game
        .object(sacrifice_id)
        .map(|object| (object.stable_id, object.name.to_string()))
        .expect("Sacrifice spell object should exist");
    game.push_to_stack(
        StackEntry::new(sacrifice_id, alice)
            .with_source_info(sacrifice_stable_id, sacrifice_name)
            .with_targets(vec![Target::Player(bob)]),
    );

    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("sacrifice spell should resolve");

    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "sacrificing the swapped Ornithopter should queue its borrowed dies trigger"
    );

    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("borrowed dies trigger should go on the stack");
    assert_eq!(
        game.stack.len(),
        1,
        "Myr Moonvessel's borrowed dies trigger should use the stack"
    );
    assert_eq!(
        game.stack
            .last()
            .and_then(|entry| entry.source_name.as_deref()),
        Some("Ornithopter"),
        "the stacked trigger should come from Ornithopter after the text-box exchange"
    );

    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("borrowed dies trigger should resolve");
    assert_eq!(
        game.player(bob)
            .expect("Bob should exist")
            .mana_pool
            .amount(ManaSymbol::Colorless),
        1,
        "the Ornithopter controller should get {{C}} from the swapped Myr Moonvessel trigger"
    );
}

#[test]
#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn test_exchange_of_words_swapped_myr_moonvessel_dies_trigger_stacks_when_ornithopter_is_sacrificed()
 {
    run_exchange_of_words_swapped_myr_moonvessel_dies_trigger_stacks_when_ornithopter_is_sacrificed(
    );
}

pub(super) fn run_exchange_of_words_cast_from_hand_swapping_alices_yawgmoth_and_ornithopter()
-> (GameState, ObjectId, ObjectId) {
    use crate::ability::AbilityKind;
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::cards::definitions::{ornithopter, yawgmoth_thran_physician};
    use crate::mana::{ManaCost, ManaSymbol};
    use std::sync::OnceLock;

    #[derive(Debug)]
    struct ExchangeTargetsDecisionMaker {
        first: ObjectId,
        second: ObjectId,
    }

    impl DecisionMaker for ExchangeTargetsDecisionMaker {
        fn decide_targets(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            let Some(requirement) = ctx.requirements.first() else {
                return Vec::new();
            };

            [Target::Object(self.first), Target::Object(self.second)]
                .into_iter()
                .filter(|target| requirement.legal_targets.contains(target))
                .collect()
        }
    }

    #[derive(Clone)]
    struct ExchangeOfWordsYawgmothFixture {
        exchange: crate::cards::CardDefinition,
        myr: crate::cards::CardDefinition,
        ornithopter: crate::cards::CardDefinition,
        yawgmoth: crate::cards::CardDefinition,
        omniscience: crate::cards::CardDefinition,
        divination: crate::cards::CardDefinition,
        counterspell: crate::cards::CardDefinition,
    }

    fn exchange_of_words_yawgmoth_fixture() -> &'static ExchangeOfWordsYawgmothFixture {
        static FIXTURE: OnceLock<ExchangeOfWordsYawgmothFixture> = OnceLock::new();
        FIXTURE.get_or_init(|| ExchangeOfWordsYawgmothFixture {
            exchange: CardDefinitionBuilder::new(CardId::from_raw(700_301), "Exchange of Words")
                .mana_cost(ManaCost::from_pips(vec![
                    vec![ManaSymbol::Generic(1)],
                    vec![ManaSymbol::Blue],
                    vec![ManaSymbol::Blue],
                ]))
                .card_types(vec![CardType::Enchantment])
                .parse_text(
                    "When this enchantment enters, choose two target creatures. For as long as this enchantment remains on the battlefield, exchange the text boxes of those creatures.",
                )
                .expect("Exchange of Words should parse"),
            myr: CardDefinitionBuilder::new(CardId::from_raw(700_302), "Myr Moonvessel")
                .card_types(vec![CardType::Artifact, CardType::Creature])
                .power_toughness(PowerToughness::fixed(1, 1))
                .parse_text("When this creature dies, add {C}.")
                .expect("Myr Moonvessel should parse"),
            ornithopter: ornithopter(),
            yawgmoth: yawgmoth_thran_physician(),
            omniscience: CardDefinitionBuilder::new(CardId::from_raw(700_303), "Omniscience")
                .card_types(vec![CardType::Enchantment])
                .parse_text("You may cast spells from your hand without paying their mana costs.")
                .expect("Omniscience should parse"),
            divination: CardDefinitionBuilder::new(CardId::from_raw(700_304), "Divination")
                .mana_cost(ManaCost::from_pips(vec![
                    vec![ManaSymbol::Generic(2)],
                    vec![ManaSymbol::Blue],
                ]))
                .card_types(vec![CardType::Sorcery])
                .parse_text("Draw two cards.")
                .expect("Divination should parse"),
            counterspell: CardDefinitionBuilder::new(CardId::from_raw(700_305), "Counterspell")
                .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue], vec![ManaSymbol::Blue]]))
                .card_types(vec![CardType::Instant])
                .parse_text("Counter target spell.")
                .expect("Counterspell should parse"),
        })
    }

    fn add_ui_opening_battlefield_preset(
        game: &mut GameState,
        player: PlayerId,
        fixture: &ExchangeOfWordsYawgmothFixture,
    ) -> (ObjectId, ObjectId) {
        game.create_object_from_definition(&fixture.omniscience, player, Zone::Battlefield);

        for (idx, name) in [
            "Forest",
            "Plains",
            "Island",
            "Mountain",
            "Swamp",
            "Tropical Island",
            "Volcanic Island",
        ]
        .into_iter()
        .enumerate()
        {
            let land = CardBuilder::new(CardId::from_raw(700_320 + idx as u32), name)
                .card_types(vec![CardType::Land])
                .build();
            game.create_object_from_card(&land, player, Zone::Battlefield);
        }

        let yawgmoth_id =
            game.create_object_from_definition(&fixture.yawgmoth, player, Zone::Battlefield);
        let ornithopter_id =
            game.create_object_from_definition(&fixture.ornithopter, player, Zone::Battlefield);
        game.create_object_from_definition(&fixture.myr, player, Zone::Battlefield);

        (ornithopter_id, yawgmoth_id)
    }

    fn add_ui_opening_hand_preset(
        game: &mut GameState,
        player: PlayerId,
        fixture: &ExchangeOfWordsYawgmothFixture,
    ) {
        let plains = CardBuilder::new(CardId::from_raw(700_340), "Plains")
            .card_types(vec![CardType::Land])
            .build();
        let swamp = CardBuilder::new(CardId::from_raw(700_341), "Swamp")
            .card_types(vec![CardType::Land])
            .build();
        let mountain = CardBuilder::new(CardId::from_raw(700_342), "Mountain")
            .card_types(vec![CardType::Land])
            .build();

        game.create_object_from_card(&plains, player, Zone::Hand);
        game.create_object_from_definition(&fixture.divination, player, Zone::Hand);
        game.create_object_from_card(&swamp, player, Zone::Hand);
        game.create_object_from_definition(&fixture.counterspell, player, Zone::Hand);
        game.create_object_from_definition(&fixture.divination, player, Zone::Hand);
        game.create_object_from_card(&mountain, player, Zone::Hand);
        game.create_object_from_card(&plains, player, Zone::Hand);
    }

    let mut game = GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
            "Diana".to_string(),
        ],
        20,
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let fixture = exchange_of_words_yawgmoth_fixture();
    let (ornithopter_id, yawgmoth_id) =
        add_ui_opening_battlefield_preset(&mut game, alice, fixture);
    let _ = add_ui_opening_battlefield_preset(&mut game, bob, fixture);
    add_ui_opening_hand_preset(&mut game, alice, fixture);
    let exchange_id = game.create_object_from_definition(&fixture.exchange, alice, Zone::Hand);

    let exchange_cast_method = crate::decision::compute_legal_actions(&game, alice)
        .into_iter()
        .find_map(|action| match action {
            crate::decision::LegalAction::CastSpell {
                spell_id,
                from_zone: Zone::Hand,
                casting_method,
            } if spell_id == exchange_id && casting_method != CastingMethod::Normal => {
                Some(casting_method)
            }
            _ => None,
        })
        .expect("Omniscience should offer a free cast for Exchange of Words");

    let stack_id = super::priority_mana::propose_spell_cast(
        &mut game,
        exchange_id,
        Zone::Hand,
        alice,
        &exchange_cast_method,
    )
    .expect("Exchange of Words should move from hand to stack");
    game.stack
        .push(StackEntry::new(stack_id, alice).with_casting_method(exchange_cast_method.clone()));

    let mut trigger_queue = TriggerQueue::new();
    let mut dm = ExchangeTargetsDecisionMaker {
        first: yawgmoth_id,
        second: ornithopter_id,
    };

    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("Exchange of Words spell should resolve from the stack");
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Exchange of Words should queue exactly one ETB trigger after resolving"
    );

    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Exchange of Words ETB trigger should go on the stack");
    resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
        .expect("Exchange of Words ETB trigger should resolve");

    game.refresh_continuous_state();

    let ornithopter_chars = game
        .calculated_characteristics(ornithopter_id)
        .expect("Ornithopter should still have calculated characteristics");
    let yawgmoth_chars = game
        .calculated_characteristics(yawgmoth_id)
        .expect("Yawgmoth should still have calculated characteristics");

    assert_eq!(
        ornithopter_chars.compiled_card_text.as_ref(),
        crate::compiled_text::debug_compiled_lines(&fixture.yawgmoth).join("\n"),
        "Ornithopter should pick up Yawgmoth's text box after the exchange"
    );
    assert_eq!(
        yawgmoth_chars.compiled_card_text.as_ref(),
        crate::compiled_text::debug_compiled_lines(&fixture.ornithopter).join("\n"),
        "Yawgmoth should pick up Ornithopter's text box after the exchange"
    );
    assert!(
        ornithopter_chars
            .abilities
            .iter()
            .any(|ability| matches!(ability.kind, AbilityKind::Activated(_))),
        "Ornithopter should gain Yawgmoth's activated abilities after the exchange"
    );

    (game, ornithopter_id, yawgmoth_id)
}

#[test]
#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn test_exchange_of_words_cast_from_hand_swaps_alices_yawgmoth_and_ornithopter() {
    let _ = run_exchange_of_words_cast_from_hand_swapping_alices_yawgmoth_and_ornithopter();
}

#[test]
#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn test_compute_legal_actions_after_exchange_of_words_sees_borrowed_activated_abilities()
{
    let (game, ornithopter_id, yawgmoth_id) =
        run_exchange_of_words_cast_from_hand_swapping_alices_yawgmoth_and_ornithopter();
    let alice = PlayerId::from_index(0);

    let actions = crate::decision::compute_legal_actions(&game, alice);

    assert!(
        actions.iter().any(|action| {
            matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, .. }
                    if *source == ornithopter_id
            )
        }),
        "Alice should see Ornithopter's borrowed Yawgmoth activation in legal actions after Exchange of Words"
    );
    assert!(
        actions.iter().all(|action| {
            !matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, .. }
                    if *source == yawgmoth_id
            )
        }),
        "Yawgmoth should no longer expose its old activated abilities after its text box is exchanged away"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_extract_target_specs_two_distinct_targets_create_two_requirements() {
    use crate::cards::CardDefinitionBuilder;

    let def = CardDefinitionBuilder::new(CardId::new(), "Spiteful Blow Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Destroy target creature and target land.")
        .expect("two-distinct-target clause should parse");

    let effects = def.spell_effect.expect("expected spell effects");
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let creature_id = create_creature(&mut game, "Target Creature", bob, 2, 2);
    let land_card = CardBuilder::new(CardId::from_raw(2), "Target Land")
        .card_types(vec![CardType::Land])
        .build();
    let land_id = game.create_object_from_card(&land_card, bob, Zone::Battlefield);

    let requirements = extract_target_requirements(&game, &effects, alice, None);
    assert_eq!(
        requirements.len(),
        2,
        "expected two target requirements, got {:?}",
        requirements
    );
    assert!(
        requirements
            .iter()
            .any(|req| req.legal_targets == vec![Target::Object(creature_id)]),
        "expected one requirement to target only the creature, got {:?}",
        requirements
    );
    assert!(
        requirements
            .iter()
            .any(|req| req.legal_targets == vec![Target::Object(land_id)]),
        "expected one requirement to target only the land, got {:?}",
        requirements
    );
    assert!(
        requirements
            .iter()
            .all(|req| req.min_targets == 1 && req.max_targets == Some(1)),
        "expected both requirements to be single-target, got {:?}",
        requirements
    );
}

#[test]
pub(super) fn test_repeated_earthbend_effects_declare_independent_targets() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);

    let swamp = CardBuilder::new(CardId::from_raw(5_010), "Swamp")
        .card_types(vec![CardType::Land])
        .build();
    let forest = CardBuilder::new(CardId::from_raw(5_011), "Forest")
        .card_types(vec![CardType::Land])
        .build();
    let swamp_id = game.create_object_from_card(&swamp, alice, Zone::Battlefield);
    let forest_id = game.create_object_from_card(&forest, alice, Zone::Battlefield);

    let spec = ChooseSpec::target(ChooseSpec::Object(
        crate::filter::ObjectFilter::land().controlled_by(PlayerFilter::You),
    ));
    let effects = vec![
        Effect::new(crate::effects::EarthbendEffect::new(spec.clone(), 1)),
        Effect::new(crate::effects::EarthbendEffect::new(spec.clone(), 1)),
    ];

    let requirements = extract_target_requirements(&game, &effects, alice, None);
    assert_eq!(
        requirements.len(),
        2,
        "each earthbend instruction should declare its own target slot"
    );
    assert!(
        requirements
            .iter()
            .all(|req| req.legal_targets.contains(&Target::Object(swamp_id))
                && req.legal_targets.contains(&Target::Object(forest_id))),
        "both earthbend target prompts should offer the controlled lands, got {requirements:?}"
    );
}

#[test]
pub(super) fn test_repeated_earthbend_trigger_prompts_for_each_target() {
    #[derive(Default)]
    struct RecordingTargetDecisionMaker {
        targets_ctx: Option<crate::decisions::context::TargetsContext>,
    }

    impl DecisionMaker for RecordingTargetDecisionMaker {
        fn decide_targets(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            self.targets_ctx = Some(ctx.clone());
            ctx.requirements
                .iter()
                .filter_map(|requirement| requirement.legal_targets.first().copied())
                .collect()
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let source_id = create_creature(&mut game, "Earthbend Source", alice, 2, 2);
    let source_stable_id = game.object(source_id).expect("source exists").stable_id;

    for name in ["Swamp", "Forest"] {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Land])
            .build();
        game.create_object_from_card(&card, alice, Zone::Battlefield);
    }

    let spec = ChooseSpec::target(ChooseSpec::Object(
        crate::filter::ObjectFilter::land().controlled_by(PlayerFilter::You),
    ));
    let effects = vec![
        Effect::new(crate::effects::EarthbendEffect::new(spec.clone(), 1)),
        Effect::new(crate::effects::EarthbendEffect::new(spec.clone(), 1)),
    ];
    let ability = crate::ability::TriggeredAbility {
        trigger: Trigger::beginning_of_upkeep(PlayerFilter::You),
        effects: crate::resolution::ResolutionProgram::from_effects(effects),
        choices: vec![spec.clone()],
        intervening_if: None,
        presentation_label: None,
    };

    let mut trigger_queue = TriggerQueue::new();
    trigger_queue.add(TriggeredAbilityEntry {
        source: source_id,
        controller: alice,
        x_value: None,
        event_value_amount: None,
        ability: ability.clone(),
        triggering_event: TriggerEvent::new_with_provenance(
            crate::events::phase::BeginningOfUpkeepEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        ),
        source_stable_id,
        source_name: "Earthbend Source".to_string(),
        source_snapshot: None,
        tagged_objects: std::collections::HashMap::new(),
        source_kind: crate::triggers::TriggeredAbilitySourceKind::Object,
        trigger_identity: crate::triggers::compute_trigger_identity(&ability),
    });

    let mut dm = RecordingTargetDecisionMaker::default();
    put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("earthbend trigger should be put on the stack");

    let targets_ctx = dm
        .targets_ctx
        .expect("earthbend trigger should ask for targets");
    assert_eq!(
        targets_ctx.requirements.len(),
        2,
        "repeated earthbend trigger should ask for two independent target slots"
    );
    assert_eq!(
        game.stack
            .last()
            .expect("trigger should be on stack")
            .target_assignments
            .len(),
        2,
        "stack entry should preserve both target assignments"
    );
}

#[test]
pub(super) fn test_distinct_object_target_clauses_resolve_against_their_own_selected_targets() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source_id = game.new_object_id();

    let creature_id = create_creature(&mut game, "Marked Creature", bob, 2, 2);
    let land_card = CardBuilder::new(CardId::from_raw(5_001), "Marked Land")
        .card_types(vec![CardType::Land])
        .build();
    let land_id = game.create_object_from_card(&land_card, bob, Zone::Battlefield);

    let mut ctx = ExecutionContext::new_default(source_id, alice)
        .with_targets(vec![
            crate::effects::ResolvedTarget::Object(creature_id),
            crate::effects::ResolvedTarget::Object(land_id),
        ])
        .with_target_assignments(vec![
            crate::game_state::TargetAssignment {
                spec: ChooseSpec::target(ChooseSpec::creature()),
                range: 0..1,
            },
            crate::game_state::TargetAssignment {
                spec: ChooseSpec::target(ChooseSpec::Object(crate::filter::ObjectFilter::land())),
                range: 1..2,
            },
        ]);

    let destroy_creature =
        Effect::new(crate::effects::DestroyEffect::target(ChooseSpec::creature()));
    let destroy_land = Effect::new(crate::effects::DestroyEffect::target(ChooseSpec::Object(
        crate::filter::ObjectFilter::land(),
    )));

    crate::effects::execute_effect(&mut game, &destroy_creature, &mut ctx)
        .expect("creature-destroy effect should resolve");
    crate::effects::execute_effect(&mut game, &destroy_land, &mut ctx)
        .expect("land-destroy effect should resolve");

    assert!(
        !game.battlefield.contains(&creature_id),
        "the creature target should leave the battlefield after the first clause resolves"
    );
    assert!(
        !game.battlefield.contains(&land_id),
        "the land target should leave the battlefield after the second clause resolves instead of reusing the first object target"
    );
    assert_eq!(
        game.player(bob).expect("bob exists").graveyard.len(),
        2,
        "both selected targets should end up in Bob's graveyard"
    );
}

#[test]
pub(super) fn test_exchange_control_second_target_requirement_includes_artifact_creatures() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let spell_card = CardBuilder::new(CardId::from_raw(5_051), "Legerdemain Probe")
        .card_types(vec![CardType::Sorcery])
        .build();
    let spell_id = game.create_object_from_card(&spell_card, alice, Zone::Stack);
    let ring_card = CardBuilder::new(CardId::from_raw(5_052), "Jinxed Ring")
        .card_types(vec![CardType::Artifact])
        .build();
    let ring_id = game.create_object_from_card(&ring_card, alice, Zone::Battlefield);
    let construct_card = CardBuilder::new(CardId::from_raw(5_053), "Bonded Construct")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 1))
        .build();
    let construct_id = game.create_object_from_card(&construct_card, bob, Zone::Battlefield);

    let first = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::artifact()));
    let second = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::permanent().other()));
    let exchange = crate::effects::ExchangeControlEffect::new(first, second)
        .with_permanent1_reference_tag(crate::tag::TagKey::from("exchange_first"))
        .with_shared_type(crate::effects::SharedTypeConstraint::CardType);
    let effect = Effect::new(crate::effects::TaggedEffect::new(
        "exchanged",
        Effect::new(exchange),
    ));

    let requirements =
        super::targeting::extract_target_requirements(&game, &[effect], alice, Some(spell_id));

    assert_eq!(requirements.len(), 2);
    assert!(
        requirements[0]
            .legal_targets
            .contains(&Target::Object(ring_id))
    );
    assert!(
        requirements[1]
            .legal_targets
            .contains(&Target::Object(construct_id)),
        "the second Legerdemain target must allow an artifact creature, got {:?}",
        requirements[1].legal_targets
    );
}

#[test]
pub(super) fn test_exchange_control_target_requirements_descend_through_may_effect() {
    use crate::Target;
    use crate::target::{ChooseSpec, ObjectFilter, TaggedOpbjectRelation};

    struct YesDecisionMaker;

    impl DecisionMaker for YesDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            true
        }
    }

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let source_card = CardBuilder::new(CardId::from_raw(5_054), "Puca Probe")
        .card_types(vec![CardType::Enchantment])
        .build();
    let source_id = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
    let illusion_card = CardBuilder::new(CardId::from_raw(5_055), "Illusions of Grandeur")
        .mana_cost(ManaCost::from_symbols(vec![
            ManaSymbol::Generic(3),
            ManaSymbol::Blue,
        ]))
        .card_types(vec![CardType::Enchantment])
        .build();
    let illusion_id = game.create_object_from_card(&illusion_card, alice, Zone::Battlefield);
    let celebrant_card = CardBuilder::new(CardId::from_raw(5_056), "Kor Celebrant")
        .mana_cost(ManaCost::from_symbols(vec![
            ManaSymbol::Generic(2),
            ManaSymbol::White,
        ]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 4))
        .build();
    let celebrant_id = game.create_object_from_card(&celebrant_card, bob, Zone::Battlefield);

    let tag = crate::tag::TagKey::from("exchange_first_probe");
    let first = ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::nonland_permanent().you_control(),
    ));
    let second = ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::nonland_permanent()
            .opponent_controls()
            .match_tagged(tag.clone(), TaggedOpbjectRelation::ManaValueLteTagged),
    ));
    let prelude = Effect::new(crate::effects::TargetOnlyEffect::new(second.clone()));
    let exchange = crate::effects::ExchangeControlEffect::new(first, second)
        .with_permanent1_reference_tag(tag);
    let effect = Effect::new(crate::effects::MayEffect::new(vec![Effect::new(
        crate::effects::TaggedEffect::new("exchanged", Effect::new(exchange)),
    )]));
    let program = crate::resolution::ResolutionProgram::from_effects(vec![prelude, effect.clone()]);

    let requirements = super::targeting::extract_target_requirements_from_program_with_modes(
        &game,
        &program,
        alice,
        Some(source_id),
        None,
    );

    assert_eq!(
        requirements.len(),
        2,
        "optional exchange effects still choose their targets while the trigger is put on the stack"
    );
    assert!(
        requirements[0]
            .legal_targets
            .contains(&Target::Object(illusion_id))
    );
    assert!(
        requirements[1]
            .legal_targets
            .contains(&Target::Object(celebrant_id))
    );

    let mut dm = YesDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source_id, alice, &mut dm)
        .with_targets(vec![
            crate::effects::ResolvedTarget::Object(illusion_id),
            crate::effects::ResolvedTarget::Object(celebrant_id),
        ])
        .with_target_assignments(vec![
            crate::game_state::TargetAssignment {
                spec: requirements[0].spec.clone(),
                range: 0..1,
            },
            crate::game_state::TargetAssignment {
                spec: requirements[1].spec.clone(),
                range: 1..2,
            },
        ]);
    super::stack_resolution::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source_id,
        &program,
        None,
        &[
            crate::game_state::TargetAssignment {
                spec: requirements[0].spec.clone(),
                range: 0..1,
            },
            crate::game_state::TargetAssignment {
                spec: requirements[1].spec.clone(),
                range: 1..2,
            },
        ],
    )
    .expect("optional exchange should resolve");
    assert_eq!(game.controller_of_id(illusion_id), Some(bob));
    assert_eq!(game.controller_of_id(celebrant_id), Some(alice));
}

#[test]
pub(super) fn test_exchange_control_resolution_preserves_selected_permanent_when_assignment_spec_is_stale()
 {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let ring_card = CardBuilder::new(CardId::from_raw(5_061), "Jinxed Ring")
        .card_types(vec![CardType::Artifact])
        .build();
    let ring_id = game.create_object_from_card(&ring_card, alice, Zone::Battlefield);
    let construct_card = CardBuilder::new(CardId::from_raw(5_062), "Bonded Construct")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 1))
        .build();
    let construct_id = game.create_object_from_card(&construct_card, bob, Zone::Battlefield);

    let first = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::artifact()));
    let second = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::permanent().other()));
    let exchange = crate::effects::ExchangeControlEffect::new(first.clone(), second)
        .with_permanent1_reference_tag(crate::tag::TagKey::from("exchange_first"))
        .with_shared_type(crate::effects::SharedTypeConstraint::CardType);
    let spell_card = CardBuilder::new(CardId::from_raw(5_063), "Legerdemain Probe")
        .card_types(vec![CardType::Sorcery])
        .build();
    let spell_def = crate::cards::CardDefinition::spell(
        spell_card,
        vec![Effect::new(crate::effects::TaggedEffect::new(
            "exchanged",
            Effect::new(exchange),
        ))],
    );
    let spell_id = game.create_object_from_definition(&spell_def, alice, Zone::Stack);
    let entry = StackEntry::new(spell_id, alice)
        .with_targets(vec![Target::Object(ring_id), Target::Object(construct_id)])
        .with_target_assignments(vec![
            crate::game_state::TargetAssignment {
                spec: first,
                range: 0..1,
            },
            crate::game_state::TargetAssignment {
                spec: ChooseSpec::target(ChooseSpec::Object(ObjectFilter::land())),
                range: 1..2,
            },
        ]);
    let (valid_targets, valid_assignments, all_invalid) =
        super::targeting::validate_stack_entry_targets(&game, &entry);
    assert!(!all_invalid);
    assert_eq!(
        valid_targets,
        vec![
            crate::effects::ResolvedTarget::Object(ring_id),
            crate::effects::ResolvedTarget::Object(construct_id),
        ]
    );
    assert_eq!(valid_assignments[1].range, 1..2);
    game.push_to_stack(entry);

    super::resolve_stack_entry(&mut game).expect("exchange spell should resolve");

    assert_eq!(game.controller_of_id(ring_id), Some(bob));
    assert_eq!(game.controller_of_id(construct_id), Some(alice));
}

#[test]
pub(super) fn test_distinct_player_target_clauses_resolve_against_their_own_selected_targets() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let squirrel = CardDefinitionBuilder::new(CardId::from_raw(5_101), "Squirrel")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Squirrel])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let token_count_before = game
        .battlefield
        .iter()
        .filter(|&&id| {
            game.object(id)
                .is_some_and(|obj| game.controller_of(obj) == alice)
        })
        .count();

    let spell_card = CardBuilder::new(CardId::from_raw(5_102), "Player Split Effects")
        .card_types(vec![CardType::Sorcery])
        .build();
    let spell_def = crate::cards::CardDefinition::spell(
        spell_card,
        vec![
            Effect::create_tokens_player(squirrel, 1, PlayerFilter::target_player()),
            Effect::new(crate::effects::GainLifeEffect::target_player(3)),
        ],
    );
    let spell_id = game.create_object_from_definition(&spell_def, alice, Zone::Stack);
    game.push_to_stack(
        StackEntry::new(spell_id, alice)
            .with_targets(vec![Target::Player(alice), Target::Player(bob)])
            .with_target_assignments(vec![
                crate::game_state::TargetAssignment {
                    spec: ChooseSpec::target_player(),
                    range: 0..1,
                },
                crate::game_state::TargetAssignment {
                    spec: ChooseSpec::target_player(),
                    range: 1..2,
                },
            ]),
    );

    super::resolve_stack_entry(&mut game).expect("spell should resolve");

    let token_count_after = game
        .battlefield
        .iter()
        .filter(|&&id| {
            game.object(id)
                .is_some_and(|obj| game.controller_of(obj) == alice)
        })
        .count();

    assert_eq!(token_count_after, token_count_before + 1);
    assert_eq!(game.player(alice).expect("alice exists").life, 20);
    assert_eq!(game.player(bob).expect("bob exists").life, 23);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_verdant_command_distinct_player_modes_use_their_own_targets() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let token_count_before = game
        .battlefield
        .iter()
        .filter(|&&id| {
            game.object(id)
                .is_some_and(|obj| game.controller_of(obj) == alice)
        })
        .count();

    let verdant_command = CardDefinitionBuilder::new(CardId::from_raw(5_103), "Verdant Command")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Choose two —\n• Target player creates two tapped 1/1 green Squirrel creature tokens.\n• Counter target loyalty ability of a planeswalker.\n• Exile target card from a graveyard.\n• Target player gains 3 life.",
        )
        .expect("Verdant Command should parse");
    let spell_id = game.create_object_from_definition(&verdant_command, alice, Zone::Stack);
    game.push_to_stack(
        StackEntry::new(spell_id, alice)
            .with_chosen_modes(Some(vec![0, 3]))
            .with_targets(vec![Target::Player(alice), Target::Player(bob)])
            .with_target_assignments(vec![
                crate::game_state::TargetAssignment {
                    spec: ChooseSpec::target_player(),
                    range: 0..1,
                },
                crate::game_state::TargetAssignment {
                    spec: ChooseSpec::target_player(),
                    range: 1..2,
                },
            ]),
    );

    super::resolve_stack_entry(&mut game).expect("Verdant Command should resolve");

    let token_count_after = game
        .battlefield
        .iter()
        .filter(|&&id| {
            game.object(id)
                .is_some_and(|obj| game.controller_of(obj) == alice)
        })
        .count();

    assert_eq!(token_count_after, token_count_before + 2);
    assert_eq!(game.player(alice).expect("alice exists").life, 20);
    assert_eq!(game.player(bob).expect("bob exists").life, 23);
}

#[test]
pub(super) fn test_stack_resolution_keeps_distinct_target_clause_assignments_when_one_target_goes_invalid()
 {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let creature_id = create_creature(&mut game, "Marked Creature", bob, 2, 2);
    let land_card = CardBuilder::new(CardId::from_raw(5_003), "Marked Land")
        .card_types(vec![CardType::Land])
        .build();
    let land_id = game.create_object_from_card(&land_card, bob, Zone::Battlefield);

    let spell_card = CardBuilder::new(CardId::from_raw(5_004), "Split Destruction")
        .card_types(vec![CardType::Sorcery])
        .build();
    let spell_def = crate::cards::CardDefinition::spell(
        spell_card,
        vec![
            Effect::new(crate::effects::DestroyEffect::target(ChooseSpec::creature())),
            Effect::new(crate::effects::DestroyEffect::target(ChooseSpec::Object(
                crate::filter::ObjectFilter::land(),
            ))),
        ],
    );
    let spell_id = game.create_object_from_definition(&spell_def, alice, Zone::Stack);
    game.push_to_stack(
        StackEntry::new(spell_id, alice)
            .with_targets(vec![Target::Object(creature_id), Target::Object(land_id)])
            .with_target_assignments(vec![
                crate::game_state::TargetAssignment {
                    spec: ChooseSpec::target(ChooseSpec::creature()),
                    range: 0..1,
                },
                crate::game_state::TargetAssignment {
                    spec: ChooseSpec::target(ChooseSpec::Object(
                        crate::filter::ObjectFilter::land(),
                    )),
                    range: 1..2,
                },
            ]),
    );

    game.move_object_by_effect(land_id, Zone::Graveyard);

    super::resolve_stack_entry(&mut game).expect("spell should resolve");

    assert!(
        !game.battlefield.contains(&creature_id),
        "the still-legal creature target should be destroyed"
    );
    assert_eq!(
        game.player(bob).expect("bob exists").graveyard.len(),
        2,
        "the invalid land target should stay gone and the creature should still be destroyed"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_nightcreep_turns_creatures_black_and_lands_into_swamps_until_end_of_turn() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    let nightcreep = CardDefinitionBuilder::new(CardId::new(), "Nightcreep Probe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text("Until end of turn, all creatures become black and all lands become Swamps.")
        .expect("Nightcreep-style text should parse");
    let spell_id = game.create_object_from_definition(&nightcreep, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell_id, alice));

    let land_def = CardDefinitionBuilder::new(CardId::new(), "Nightcreep Land Probe")
        .card_types(vec![CardType::Land])
        .parse_text("{T}: Add {C}{C}.")
        .expect("test land should parse");
    let land_id = game.create_object_from_definition(&land_def, alice, Zone::Battlefield);

    let green_creature = CardDefinitionBuilder::new(CardId::new(), "Nightcreep Creature Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let creature_id = game.create_object_from_definition(&green_creature, alice, Zone::Battlefield);

    resolve_stack_entry(&mut game).expect("Nightcreep should resolve");

    let creature_colors = game
        .current_colors(creature_id)
        .expect("creature should still have calculated colors");
    assert_eq!(
        creature_colors,
        crate::color::ColorSet::BLACK,
        "Nightcreep should set creature color to black, got {creature_colors:?}"
    );

    let land_subtypes = game.calculated_subtypes(land_id);
    assert!(
        land_subtypes.contains(&crate::types::Subtype::Swamp),
        "Nightcreep should make lands Swamps, got {land_subtypes:?}"
    );

    let land_chars = game
        .calculated_characteristics(land_id)
        .expect("land should still have calculated characteristics");
    let land_mana_symbols: Vec<Vec<ManaSymbol>> = land_chars
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) if activated.is_mana_ability() => {
                Some(activated.mana_symbols().to_vec())
            }
            _ => None,
        })
        .collect();
    assert!(
        land_mana_symbols
            .iter()
            .any(|symbols| symbols == &vec![ManaSymbol::Black]),
        "Nightcreep should give the land black mana, got {land_mana_symbols:?}"
    );
    assert!(
        !land_mana_symbols
            .iter()
            .any(|symbols| symbols == &vec![ManaSymbol::Colorless, ManaSymbol::Colorless]),
        "Nightcreep should replace the land's original colorless mana ability, got {land_mana_symbols:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_extract_target_specs_exactly_two_targets_uses_single_requirement_with_count_two()
{
    use crate::cards::CardDefinitionBuilder;

    let def = CardDefinitionBuilder::new(CardId::new(), "Aether Tradewinds Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Return two target creatures to their owners' hands.")
        .expect("exactly-two-target clause should parse");

    let effects = def.spell_effect.expect("expected spell effects");
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let creature_a = create_creature(&mut game, "Target A", bob, 2, 2);
    let creature_b = create_creature(&mut game, "Target B", bob, 3, 3);

    let requirements = extract_target_requirements(&game, &effects, alice, None);
    assert_eq!(
        requirements.len(),
        1,
        "expected one requirement with count two, got {:?}",
        requirements
    );
    assert_eq!(
        requirements[0].min_targets, 2,
        "expected minimum target count 2, got {:?}",
        requirements
    );
    assert_eq!(
        requirements[0].max_targets,
        Some(2),
        "expected maximum target count 2, got {:?}",
        requirements
    );
    assert!(
        requirements[0]
            .legal_targets
            .contains(&Target::Object(creature_a))
            && requirements[0]
                .legal_targets
                .contains(&Target::Object(creature_b)),
        "expected both creatures to be legal targets, got {:?}",
        requirements
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_beast_within_target_requirements_include_enchantments() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Beast Within")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Destroy target permanent. Its controller creates a 3/3 green Beast creature token.",
        )
        .expect("Beast Within oracle text should compile");
    let effects = def.spell_effect.expect("expected spell effects");

    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let creature_id = create_creature(&mut game, "Target Creature", bob, 2, 2);
    let enchantment = CardBuilder::new(CardId::from_raw(5_003), "Target Enchantment")
        .card_types(vec![CardType::Enchantment])
        .build();
    let enchantment_id = game.create_object_from_card(&enchantment, bob, Zone::Battlefield);

    let requirements = extract_target_requirements(&game, &effects, alice, None);
    assert_eq!(
        requirements.len(),
        1,
        "Beast Within should have exactly one target requirement, got {:?}",
        requirements
    );
    assert!(
        requirements[0]
            .legal_targets
            .contains(&Target::Object(creature_id)),
        "Beast Within should be able to target creatures"
    );
    assert!(
        requirements[0]
            .legal_targets
            .contains(&Target::Object(enchantment_id)),
        "Beast Within should be able to target enchantments, got {:?}",
        requirements[0].legal_targets
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resculpt_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(724_001), "Resculpt")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Exile target artifact or creature. Its controller creates a 4/4 blue and red Elemental creature token.",
        )
        .expect("Resculpt oracle text should parse strictly")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn resculpt_strict_parser_and_compiled_text_regression() {
    let def = resculpt_definition();
    let rendered = crate::compiled_text::compiled_text_lines(&def).join(" ");
    let debug = format!("{:?}", def.spell_effect);

    assert!(
        rendered.contains("Exile target artifact or creature"),
        "Resculpt should render the artifact-or-creature exile target, got {rendered}"
    );
    assert!(
        rendered.contains("Its controller creates a 4/4 blue and red Elemental creature token"),
        "Resculpt should render the target controller token clause, got {rendered}"
    );
    assert!(
        debug.contains("Exile") && debug.contains("CreateTokenEffect"),
        "Resculpt should lower to exile plus token creation effects, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn resculpt_targets_artifacts_and_creatures_but_not_other_permanents() {
    let def = resculpt_definition();
    let effects = def
        .spell_effect
        .as_ref()
        .expect("Resculpt should be a spell");
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let artifact = game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(724_002), "Target Artifact")
            .card_types(vec![CardType::Artifact])
            .build(),
        bob,
        Zone::Battlefield,
    );
    let creature = create_creature(&mut game, "Target Creature", bob, 2, 2);
    let enchantment = game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(724_003), "Target Enchantment")
            .card_types(vec![CardType::Enchantment])
            .build(),
        bob,
        Zone::Battlefield,
    );

    let requirements = extract_target_requirements(&game, effects, alice, None);
    assert_eq!(
        requirements.len(),
        1,
        "Resculpt should have one target requirement, got {requirements:?}"
    );
    let legal_targets = &requirements[0].legal_targets;
    assert!(
        legal_targets.contains(&Target::Object(artifact)),
        "Resculpt should be able to target artifacts, got {legal_targets:?}"
    );
    assert!(
        legal_targets.contains(&Target::Object(creature)),
        "Resculpt should be able to target creatures, got {legal_targets:?}"
    );
    assert!(
        !legal_targets.contains(&Target::Object(enchantment)),
        "Resculpt should not be able to target nonartifact noncreature permanents, got {legal_targets:?}"
    );
    assert!(
        !legal_targets.contains(&Target::Player(bob)),
        "Resculpt should not be able to target players, got {legal_targets:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn resculpt_exiles_target_and_gives_elemental_to_targets_controller() {
    let def = resculpt_definition();
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let spell_id = game.create_object_from_definition(&def, alice, Zone::Stack);
    let target = game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(724_004), "Bob's Relic")
            .card_types(vec![CardType::Artifact])
            .build(),
        bob,
        Zone::Battlefield,
    );
    let target_stable = game.object(target).expect("target exists").stable_id;

    game.push_to_stack(StackEntry::new(spell_id, alice).with_targets(vec![Target::Object(target)]));
    resolve_stack_entry(&mut game).expect("Resculpt should resolve");

    let exiled_target = game
        .find_object_by_stable_id(target_stable)
        .expect("exiled target should still exist");
    assert!(
        game.exile.contains(&exiled_target),
        "Resculpt should exile its artifact target"
    );

    let elemental_tokens: Vec<_> = game
        .objects_in_zone(Zone::Battlefield)
        .into_iter()
        .filter(|id| {
            let object = game.object(*id).expect("battlefield object exists");
            object.kind == ObjectKind::Token
                && object.name == "Elemental"
                && object.subtypes.contains(&Subtype::Elemental)
        })
        .collect();
    assert_eq!(
        elemental_tokens.len(),
        1,
        "Resculpt should create exactly one Elemental token, got {elemental_tokens:?}"
    );
    let token = elemental_tokens[0];
    assert_eq!(
        game.controller_of(game.object(token).expect("token exists")),
        bob,
        "the target's controller should control the Elemental token"
    );
    assert_eq!(game.current_power(token), Some(4));
    assert_eq!(game.current_toughness(token), Some(4));
    assert_eq!(
        game.current_colors(token),
        Some(crate::color::ColorSet::BLUE.union(crate::color::ColorSet::RED)),
        "the Elemental token should be blue and red"
    );
}

#[test]
pub(super) fn test_non_target_put_onto_battlefield_choice_does_not_create_target_requirement() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let land_card = CardBuilder::new(CardId::from_raw(5_002), "Choice Land")
        .card_types(vec![CardType::Land])
        .build();
    game.create_object_from_card(&land_card, alice, Zone::Hand);
    let effects = vec![Effect::new(
        crate::effects::PutOntoBattlefieldEffect::you_control(
            ChooseSpec::Object(crate::filter::ObjectFilter::land().in_zone(Zone::Hand)),
            false,
        ),
    )];

    let requirements = extract_target_requirements(&game, &effects, alice, None);

    assert!(
        requirements.is_empty(),
        "resolution-time choices like 'put a land card from your hand onto the battlefield' should not prompt for targets: {:?}",
        requirements
    );
}

#[test]
pub(super) fn test_spell_has_legal_targets_any_number_with_no_targets() {
    let game = setup_game();
    let alice = PlayerId::from_index(0);

    // Exile any number of target spells
    let effects = vec![Effect::exile_any_number(ChooseSpec::spell())];

    // No spells on stack - but "any number" (min_targets == 0) means 0 targets is valid
    let has_targets = spell_has_legal_targets(&game, &effects, alice, None);
    // "Any number" effects can be cast with 0 targets
    assert!(has_targets, "any_number effects can be cast with 0 targets");
}

#[test]
pub(super) fn test_spell_has_legal_targets_single_target_needs_target() {
    let game = setup_game();
    let alice = PlayerId::from_index(0);

    // Single target exile spell - needs at least one target
    let effects = vec![Effect::exile(ChooseSpec::spell())];

    // No spells on stack
    let has_targets = spell_has_legal_targets(&game, &effects, alice, None);
    assert!(
        !has_targets,
        "Single-target spell needs at least one legal target"
    );
}

#[test]
pub(super) fn test_spell_has_legal_targets_with_may_wrapper_needs_target() {
    let game = setup_game();
    let alice = PlayerId::from_index(0);
    let effects = vec![Effect::may(vec![Effect::counter(ChooseSpec::spell())])];

    let has_targets = spell_has_legal_targets(&game, &effects, alice, None);
    assert!(
        !has_targets,
        "may-wrapped targeted effects must still require legal targets"
    );
}

#[test]
pub(super) fn test_spell_has_legal_targets_with_unless_action_wrapper_needs_target() {
    let game = setup_game();
    let alice = PlayerId::from_index(0);
    let effects = vec![Effect::unless_action(
        vec![Effect::counter(ChooseSpec::spell())],
        vec![Effect::gain_life(1)],
        crate::target::PlayerFilter::You,
    )];

    let has_targets = spell_has_legal_targets(&game, &effects, alice, None);
    assert!(
        !has_targets,
        "unless-action wrapped targeted effects must still require legal targets"
    );
}

#[test]
pub(super) fn test_spell_has_legal_targets_with_sequence_wrapper_needs_target() {
    let game = setup_game();
    let alice = PlayerId::from_index(0);
    let effects = vec![Effect::new(crate::effects::SequenceEffect::new(vec![
        Effect::gain_life(1),
        Effect::counter(ChooseSpec::spell()),
    ]))];

    let has_targets = spell_has_legal_targets(&game, &effects, alice, None);
    assert!(
        !has_targets,
        "sequence-wrapped targeted effects must still require legal targets"
    );
}

#[test]
pub(super) fn test_spell_has_legal_targets_with_choose_mode_allows_non_target_mode() {
    let game = setup_game();
    let alice = PlayerId::from_index(0);
    let effects = vec![Effect::choose_one(vec![
        crate::effect::EffectMode {
            source_text: "Counter target spell".to_string(),
            effects: vec![Effect::counter(ChooseSpec::spell())],
        },
        crate::effect::EffectMode {
            source_text: "Gain 3 life".to_string(),
            effects: vec![Effect::gain_life(3)],
        },
    ])];

    let has_targets = spell_has_legal_targets(&game, &effects, alice, None);
    assert!(
        has_targets,
        "modal spell should be castable when at least one legal mode exists"
    );
}

#[test]
pub(super) fn test_spell_has_legal_targets_with_choose_mode_requires_enough_legal_modes() {
    let game = setup_game();
    let alice = PlayerId::from_index(0);
    let effects = vec![Effect::new(
        crate::effects::ChooseModeEffect::choose_exactly(
            2,
            vec![
                crate::effect::EffectMode {
                    source_text: "Counter target spell".to_string(),
                    effects: vec![Effect::counter(ChooseSpec::spell())],
                },
                crate::effect::EffectMode {
                    source_text: "Gain 3 life".to_string(),
                    effects: vec![Effect::gain_life(3)],
                },
            ],
        ),
    )];

    let has_targets = spell_has_legal_targets(&game, &effects, alice, None);
    assert!(
        !has_targets,
        "choose-exactly modal spell should fail if too few legal modes exist"
    );
}

#[test]
pub(super) fn test_spell_has_legal_targets_with_mode_selection_respects_selected_mode_legality() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let effects = vec![Effect::choose_one(vec![
        crate::effect::EffectMode {
            source_text: "Counter target spell".to_string(),
            effects: vec![Effect::counter(ChooseSpec::spell())],
        },
        crate::effect::EffectMode {
            source_text: "Gain 3 life".to_string(),
            effects: vec![Effect::gain_life(3)],
        },
    ])];

    let counter_mode = [0usize];
    let gain_mode = [1usize];

    assert!(
        !spell_has_legal_targets_with_modes(&game, &effects, alice, None, Some(&counter_mode)),
        "counter mode should be illegal without a spell on the stack"
    );
    assert!(
        spell_has_legal_targets_with_modes(&game, &effects, alice, None, Some(&gain_mode)),
        "non-targeting mode should remain legal"
    );

    let card = CardBuilder::new(CardId::from_raw(999), "Target Spell")
        .card_types(vec![CardType::Instant])
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::Blue,
        ]]))
        .build();
    let target_spell = game.create_object_from_card(&card, bob, Zone::Stack);
    game.push_to_stack(StackEntry::new(target_spell, bob));

    assert!(
        spell_has_legal_targets_with_modes(&game, &effects, alice, None, Some(&counter_mode)),
        "counter mode should become legal when a spell is available to target"
    );
}

#[test]
pub(super) fn test_spell_has_legal_targets_with_mode_preview_allows_partial_choose_two_selection() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let effects = vec![Effect::choose_exactly(
        2,
        vec![
            crate::effect::EffectMode {
                source_text: "Counter target spell".to_string(),
                effects: vec![Effect::counter(ChooseSpec::spell())],
            },
            crate::effect::EffectMode {
                source_text: "Gain 3 life".to_string(),
                effects: vec![Effect::gain_life(3)],
            },
            crate::effect::EffectMode {
                source_text: "Draw a card".to_string(),
                effects: vec![Effect::draw(1)],
            },
        ],
    )];

    let counter_mode = [0usize];
    let gain_mode = [1usize];
    let draw_mode = [2usize];

    assert!(
        spell_has_legal_targets(&game, &effects, alice, None),
        "choose-two modal spell should be castable when two non-targeting modes are available"
    );
    assert!(
        !spell_has_legal_targets_with_mode_preview(&game, &effects, alice, None, &counter_mode),
        "counter mode preview should be illegal without a spell on the stack"
    );
    assert!(
        spell_has_legal_targets_with_mode_preview(&game, &effects, alice, None, &gain_mode),
        "preview should allow picking one legal mode before the full choose-two set is complete"
    );
    assert!(
        spell_has_legal_targets_with_mode_preview(&game, &effects, alice, None, &draw_mode),
        "additional legal non-targeting modes should also preview as selectable"
    );
    assert!(
        !spell_has_legal_targets_with_modes(&game, &effects, alice, None, Some(&gain_mode)),
        "final validation should still reject incomplete choose-two selections"
    );

    let card = CardBuilder::new(CardId::from_raw(1_000), "Target Spell")
        .card_types(vec![CardType::Instant])
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::Blue,
        ]]))
        .build();
    let target_spell = game.create_object_from_card(&card, bob, Zone::Stack);
    game.push_to_stack(StackEntry::new(target_spell, bob));

    let counter_and_gain = [0usize, 1usize];
    assert!(
        spell_has_legal_targets_with_mode_preview(&game, &effects, alice, None, &counter_mode),
        "counter mode preview should become legal once a spell is available to target"
    );
    assert!(
        spell_has_legal_targets_with_modes(&game, &effects, alice, None, Some(&counter_and_gain)),
        "final validation should accept a complete legal choose-two selection"
    );
}

#[test]
pub(super) fn test_active_target_assignments_preserves_stored_target_slot_when_legality_changes() {
    let game = setup_game();
    let effects = vec![Effect::counter(ChooseSpec::spell())];
    let assignment = crate::game_state::TargetAssignment {
        spec: ChooseSpec::spell(),
        range: 0..1,
    };
    let mut consumed_modal_selection = false;
    let mut declared_targets = Vec::new();
    let mut cursor = 0usize;

    let active = super::stack_resolution::active_target_assignments_for_effect(
        &effects[0],
        None,
        &mut consumed_modal_selection,
        &mut declared_targets,
        std::slice::from_ref(&assignment),
        &mut cursor,
    );

    assert_eq!(
        active,
        vec![assignment],
        "resolution should keep the stored target assignment even when no legal targets remain"
    );
    assert_eq!(
        cursor, 1,
        "target cursor should advance by the stored slot count"
    );
    assert!(
        !spell_has_legal_targets(&game, &effects, PlayerId::from_index(0), None),
        "sanity check: the same effect should currently have no legal targets"
    );
}

#[test]
pub(super) fn stack_spell_with_granted_static_ability_still_executes_spell_effect() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bolt = CardDefinitionBuilder::new(CardId::new(), "Lightning Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Lightning Probe deals 3 damage to any target.")
        .expect("damage spell should parse");
    let goblin_card = CardBuilder::new(CardId::new(), "Raging Goblin")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let goblin = game.create_object_from_card(&goblin_card, alice, Zone::Battlefield);
    let bolt_id = game.create_object_from_definition(&bolt, alice, Zone::Stack);
    game.object_mut(bolt_id)
        .expect("spell should be on the stack")
        .abilities_mut()
        .push(Ability::static_ability(
            StaticAbility::cant_be_countered_ability(),
        ));

    let entry = StackEntry::new(bolt_id, alice)
        .with_targets(vec![Target::Object(goblin)])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: crate::target::ChooseSpec::AnyTarget,
            range: 0..1,
        }]);
    game.push_to_stack(entry);

    resolve_stack_entry(&mut game).expect("damage spell should resolve");

    assert_eq!(
        game.damage_on(goblin),
        3,
        "granted static ability on the spell must not suppress its spell effect"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn vexing_shusher_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(489_898), "Vexing Shusher")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Red, ManaSymbol::Green],
            vec![ManaSymbol::Red, ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Goblin, Subtype::Shaman])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text("This spell can't be countered.\n{R/G}: Target spell can't be countered.")
        .expect("Vexing Shusher should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn vexing_shusher_activation_targets_spell_and_stops_countering_it() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let shusher = vexing_shusher_definition();
    let shusher_id = game.create_object_from_definition(&shusher, alice, Zone::Battlefield);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Red, 1);

    assert!(
        !crate::decision::compute_legal_actions(&game, alice)
            .into_iter()
            .any(|action| matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, .. }
                    if source == shusher_id
            )),
        "Vexing Shusher activation should be illegal without a spell target"
    );

    let target_card = CardBuilder::new(CardId::new(), "Target Spell")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .build();
    let target_spell = game.create_object_from_card(&target_card, bob, Zone::Stack);
    game.push_to_stack(StackEntry::new(target_spell, bob));

    let ability_index = game
        .object(shusher_id)
        .expect("Vexing Shusher should exist")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("Vexing Shusher should have an activated ability");
    let activate_action = crate::decision::compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, ability_index: idx }
                    if *source == shusher_id && *idx == ability_index
            )
        })
        .expect("Vexing Shusher activation should be legal with a spell target and mana");

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = SelectFirstDecisionMaker;
    let progress = apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::PriorityAction(activate_action),
        &mut dm,
    )
    .expect("Vexing Shusher activation should start");

    let progress = match progress {
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::HybridChoice(_),
        ) => apply_priority_response_with_dm(
            &mut game,
            &mut trigger_queue,
            &mut state,
            &PriorityResponse::HybridChoice(0),
            &mut dm,
        )
        .expect("choosing the red side of {R/G} should continue activation"),
        other => other,
    };
    match progress {
        crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::Targets(_),
        ) => {}
        other => panic!("expected target selection for Vexing Shusher, got {other:?}"),
    }

    apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &PriorityResponse::Targets(vec![Target::Object(target_spell)]),
        &mut dm,
    )
    .expect("choosing the target spell should complete activation");

    assert_eq!(
        game.player(alice)
            .expect("Alice should exist")
            .mana_pool
            .total(),
        0,
        "Vexing Shusher activation should spend the chosen hybrid mana"
    );

    resolve_stack_entry(&mut game).expect("Vexing Shusher ability should resolve");
    assert!(
        !game.can_be_countered(target_spell),
        "targeted spell should not be counterable after Vexing Shusher resolves"
    );

    let counter_source = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Counter Source")
            .card_types(vec![CardType::Instant])
            .build(),
        bob,
        Zone::Stack,
    );
    let mut counter_dm = SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(counter_source, bob, &mut counter_dm);
    let outcome = crate::effects::execute_effect(
        &mut game,
        &Effect::counter(crate::target::ChooseSpec::SpecificObject(target_spell)),
        &mut ctx,
    )
    .expect("counter attempt should resolve as protected");

    assert_eq!(outcome.status, crate::effect::OutcomeStatus::Protected);
    assert!(
        game.stack
            .iter()
            .any(|entry| entry.object_id == target_spell),
        "protected target spell should remain on the stack after a counter attempt"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn loxodon_smiter_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(290_543), "Loxodon Smiter")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Green],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elephant, Subtype::Soldier])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "This spell can't be countered.\nIf a spell or ability an opponent controls causes you to discard this card, put it onto the battlefield instead of putting it into your graveyard.",
        )
        .expect("Loxodon Smiter should parse for runtime tests")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn loxodon_smiter_spell_cant_be_countered_runtime() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let loxodon = loxodon_smiter_definition();
    let smiter_spell = game.create_object_from_definition(&loxodon, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(smiter_spell, alice));

    game.update_cant_effects();
    assert!(
        !game.can_be_countered(smiter_spell),
        "Loxodon Smiter should be uncounterable while it is a spell on the stack"
    );

    let counter_source = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Counter Source")
            .card_types(vec![CardType::Instant])
            .build(),
        bob,
        Zone::Stack,
    );
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(counter_source, bob, &mut dm);
    let outcome = crate::effects::execute_effect(
        &mut game,
        &Effect::counter(crate::target::ChooseSpec::SpecificObject(smiter_spell)),
        &mut ctx,
    )
    .expect("counter attempt should resolve as protected");

    assert_eq!(outcome.status, crate::effect::OutcomeStatus::Protected);
    assert!(
        game.stack
            .iter()
            .any(|entry| entry.object_id == smiter_spell),
        "Loxodon Smiter should remain on the stack after a counter attempt"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn loxodon_smiter_opponent_effect_discard_replacement_moves_to_battlefield() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let loxodon = loxodon_smiter_definition();
    let smiter = game.create_object_from_definition(&loxodon, alice, Zone::Hand);
    let discard_source = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Opponent Discard Spell")
            .card_types(vec![CardType::Sorcery])
            .build(),
        bob,
        Zone::Stack,
    );
    let mut dm = SelectFirstDecisionMaker;

    let result = crate::events::processing::execute_discard(
        &mut game,
        smiter,
        alice,
        crate::events::cause::EventCause::from_effect(discard_source, bob),
        false,
        crate::provenance::ProvNodeId::default(),
        &mut dm,
    );

    assert_eq!(result.final_zone, Zone::Battlefield);
    let moved = result
        .new_id
        .expect("Loxodon Smiter should have moved to the battlefield");
    assert!(
        game.object(moved).is_some_and(
            |object| object.name == "Loxodon Smiter" && object.zone == Zone::Battlefield
        ),
        "opponent-controlled discard effect should put Loxodon Smiter onto the battlefield"
    );
    assert!(
        !game
            .player(alice)
            .expect("Alice exists")
            .graveyard
            .iter()
            .any(|&id| game
                .object(id)
                .is_some_and(|object| object.name == "Loxodon Smiter")),
        "Loxodon Smiter should not be put into Alice's graveyard when the replacement applies"
    );
}
