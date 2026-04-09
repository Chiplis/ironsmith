use super::*;
use crate::ability::AbilityKind;
use crate::card::{CardBuilder, PowerToughness};
use crate::cards::builders::CardDefinitionBuilder;
use crate::color::Color;
use crate::decision::DecisionMaker;
use crate::executor::{ExecutionContext, execute_effect};
use crate::game_event::DamageTarget;
use crate::ids::CardId;
use crate::mana::ManaSymbol;
use crate::provenance::ProvNodeId;
use crate::types::CardType;
use crate::zone::Zone;
use std::collections::VecDeque;

fn setup_three_player_game() -> GameState {
    GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    )
}

fn parse_creature_definition(
    name: &str,
    power: i32,
    toughness: i32,
    oracle_text: &str,
) -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness))
        .parse_text(oracle_text)
        .unwrap_or_else(|err| panic!("{name} should parse: {err:?}"))
}

fn parse_sorcery_definition(name: &str, oracle_text: &str) -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Sorcery])
        .parse_text(oracle_text)
        .unwrap_or_else(|err| panic!("{name} should parse: {err:?}"))
}

fn register_spell_cast_this_turn_for_test(
    game: &mut GameState,
    spell_id: ObjectId,
    caster: PlayerId,
) {
    let event = TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new(spell_id, caster, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    game.stage_turn_history_event(&event);
}

fn resolve_spell_definition_with_dm(
    game: &mut GameState,
    definition: &crate::cards::CardDefinition,
    caster: PlayerId,
    decision_maker: &mut impl DecisionMaker,
) -> ObjectId {
    let spell_id = game.create_object_from_definition(definition, caster, Zone::Stack);
    register_spell_cast_this_turn_for_test(game, spell_id, caster);

    let effects = definition.spell_effect.as_ref().expect("spell effects");
    let mut ctx =
        ExecutionContext::new_default(spell_id, caster).with_decision_maker(decision_maker);
    let mut trigger_queue = TriggerQueue::new();

    for effect in effects {
        let outcome =
            execute_effect(game, effect, &mut ctx).expect("spell effect should resolve cleanly");
        for event in outcome.events {
            queue_triggers_from_event(game, &mut trigger_queue, event, false);
        }
    }

    if !trigger_queue.entries.is_empty() {
        put_triggers_on_stack(game, &mut trigger_queue).expect("spell triggers should stack");
        while !game.stack.is_empty() {
            resolve_stack_entry(game).expect("triggered stack entry should resolve");
        }
    }

    spell_id
}

fn create_creature(
    game: &mut GameState,
    name: &str,
    owner: PlayerId,
    power: i32,
    toughness: i32,
) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness))
        .build();
    game.create_object_from_card(&card, owner, Zone::Battlefield)
}

fn move_definition_to_battlefield_with_dm(
    game: &mut GameState,
    definition: &crate::cards::CardDefinition,
    owner: PlayerId,
    decision_maker: &mut impl DecisionMaker,
) -> ObjectId {
    let old_id = game.create_object_from_definition(definition, owner, Zone::Hand);
    game.move_object_with_etb_processing_with_dm(old_id, Zone::Battlefield, decision_maker)
        .expect("object should enter the battlefield")
        .new_id
}

fn count_battlefield_name(game: &GameState, owner: PlayerId, name: &str) -> usize {
    game.battlefield
        .iter()
        .filter(|&&id| {
            game.object(id)
                .is_some_and(|obj| obj.controller == owner && obj.name == name)
        })
        .count()
}

fn first_triggered_ability(
    definition: &crate::cards::CardDefinition,
) -> &crate::ability::TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected a triggered ability")
}

fn first_activated_ability(
    definition: &crate::cards::CardDefinition,
) -> &crate::ability::ActivatedAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("expected an activated ability")
}

#[derive(Default)]
struct ScriptedDecisionMaker {
    option_matches: VecDeque<String>,
    object_matches: VecDeque<String>,
    color_matches: VecDeque<String>,
}

impl ScriptedDecisionMaker {
    fn new(option_matches: &[&str], object_matches: &[&str]) -> Self {
        Self::with_colors(option_matches, object_matches, &[])
    }

    fn with_colors(
        option_matches: &[&str],
        object_matches: &[&str],
        color_matches: &[&str],
    ) -> Self {
        Self {
            option_matches: option_matches
                .iter()
                .map(|value| value.to_ascii_lowercase())
                .collect(),
            object_matches: object_matches
                .iter()
                .map(|value| value.to_ascii_lowercase())
                .collect(),
            color_matches: color_matches
                .iter()
                .map(|value| value.to_ascii_lowercase())
                .collect(),
        }
    }
}

impl DecisionMaker for ScriptedDecisionMaker {
    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        if let Some(needle) = self.option_matches.pop_front()
            && let Some(option) = ctx.options.iter().find(|option| {
                option.legal && option.description.to_ascii_lowercase().contains(&needle)
            })
        {
            return vec![option.index];
        }

        ctx.options
            .iter()
            .filter(|option| option.legal)
            .map(|option| option.index)
            .take(ctx.min)
            .collect()
    }

    fn decide_objects(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        if let Some(needle) = self.object_matches.pop_front()
            && let Some(candidate) = ctx.candidates.iter().find(|candidate| {
                candidate.legal
                    && game
                        .object(candidate.id)
                        .is_some_and(|object| object.name.to_ascii_lowercase().contains(&needle))
            })
        {
            return vec![candidate.id];
        }

        ctx.candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .take(ctx.min)
            .collect()
    }

    fn decide_colors(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::ColorsContext,
    ) -> Vec<Color> {
        fn parse_color(value: &str) -> Option<Color> {
            match value {
                "white" => Some(Color::White),
                "blue" => Some(Color::Blue),
                "black" => Some(Color::Black),
                "red" => Some(Color::Red),
                "green" => Some(Color::Green),
                _ => None,
            }
        }

        let chosen = self
            .color_matches
            .pop_front()
            .and_then(|value| parse_color(&value))
            .or_else(|| {
                ctx.available_colors
                    .as_ref()
                    .and_then(|colors| colors.first().copied())
            })
            .unwrap_or(Color::Green);

        vec![chosen; ctx.count as usize]
    }
}

#[test]
fn choose_player_true_name_nemesis_stores_choice_and_prevents_blocking() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let true_name = parse_creature_definition(
        "True-Name Nemesis",
        3,
        1,
        "As this creature enters, choose a player.\nThis creature has protection from the chosen player.",
    );
    let mut dm = ScriptedDecisionMaker::new(&["Bob"], &[]);
    let attacker_id = move_definition_to_battlefield_with_dm(&mut game, &true_name, alice, &mut dm);
    let blocker_id = create_creature(&mut game, "Bob Blocker", bob, 2, 2);

    assert_eq!(game.chosen_player(attacker_id), Some(bob));

    let attacker = game.object(attacker_id).expect("attacker should exist");
    let blocker = game.object(blocker_id).expect("blocker should exist");
    assert!(
        !crate::rules::combat::can_block(attacker, blocker, &game),
        "the chosen player's creatures should not be able to block True-Name Nemesis"
    );
}

#[test]
fn choose_player_stuffy_doll_redirects_damage_to_chosen_player() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let stuffy_doll = parse_creature_definition(
        "Stuffy Doll",
        0,
        1,
        "Indestructible\nAs this creature enters, choose a player.\nWhenever this creature is dealt damage, it deals that much damage to the chosen player.\n{T}: This creature deals 1 damage to itself.",
    );
    let mut dm = ScriptedDecisionMaker::new(&["Bob"], &[]);
    let doll_id = move_definition_to_battlefield_with_dm(&mut game, &stuffy_doll, alice, &mut dm);
    let source_id = create_creature(&mut game, "Prodder", alice, 1, 1);

    let bob_life_before = game.player(bob).expect("bob exists").life;
    let damage_event = TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            source_id,
            DamageTarget::Object(doll_id),
            3,
            false,
            crate::events::cause::EventCause::effect(),
        ),
        ProvNodeId::default(),
    );
    let mut trigger_queue = TriggerQueue::new();
    queue_triggers_from_event(&mut game, &mut trigger_queue, damage_event, false);

    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "damage to Stuffy Doll should queue exactly one trigger"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue).expect("trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("stuffy doll trigger should resolve");

    assert_eq!(
        game.player(bob).expect("bob exists").life,
        bob_life_before - 3,
        "Stuffy Doll should deal the same amount of damage to the chosen player"
    );
}

#[test]
fn choose_player_spectral_searchlight_gives_mana_to_chosen_player() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let searchlight = CardDefinitionBuilder::new(CardId::new(), "Spectral Searchlight")
        .card_types(vec![CardType::Artifact])
        .parse_text("{T}: Choose a player. That player adds one mana of any color they choose.")
        .expect("Spectral Searchlight should parse");
    let searchlight_id = game.create_object_from_definition(&searchlight, alice, Zone::Battlefield);
    let activated = first_activated_ability(&searchlight);

    let mut dm = ScriptedDecisionMaker::with_colors(&["Bob"], &[], &["White"]);
    let mut ctx = ExecutionContext::new_default(searchlight_id, alice).with_decision_maker(&mut dm);
    for effect in &activated.effects {
        execute_effect(&mut game, effect, &mut ctx).expect("Searchlight effect should resolve");
    }

    assert_eq!(
        game.player(alice).expect("alice exists").mana_pool.total(),
        0,
        "Spectral Searchlight should not add mana to the controller when another player is chosen"
    );
    assert_eq!(
        game.player(bob)
            .expect("bob exists")
            .mana_pool
            .amount(ManaSymbol::White),
        1,
        "the chosen player should receive the chosen color of mana"
    );
}

#[test]
fn choose_player_gluntch_keeps_first_second_and_third_players_distinct() {
    let mut game = setup_three_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);

    let gluntch = parse_creature_definition(
        "Gluntch, the Bestower",
        0,
        5,
        "Flying\nAt the beginning of your end step, choose a player. They put two +1/+1 counters on a creature they control. Choose a second player to draw a card. Then choose a third player to create two Treasure tokens.",
    );
    let gluntch_id = game.create_object_from_definition(&gluntch, alice, Zone::Battlefield);
    let bob_creature_id = create_creature(&mut game, "Bob Bear", bob, 2, 2);

    let library_card = crate::card::CardBuilder::new(CardId::new(), "Charlie Draw")
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_card(&library_card, charlie, Zone::Library);

    let hand_before = game.player(charlie).expect("charlie exists").hand.len();
    let treasures_before = count_battlefield_name(&game, alice, "Treasure");

    let triggered = first_triggered_ability(&gluntch);
    let mut dm = ScriptedDecisionMaker::new(&["Bob", "Charlie", "Alice"], &["Bob Bear"]);
    let mut ctx = ExecutionContext::new_default(gluntch_id, alice).with_decision_maker(&mut dm);
    for effect in &triggered.effects {
        execute_effect(&mut game, effect, &mut ctx).expect("Gluntch trigger should resolve");
    }

    let bob_creature = game
        .object(bob_creature_id)
        .expect("Bob creature should exist");
    assert_eq!(
        bob_creature
            .counters
            .get(&crate::object::CounterType::PlusOnePlusOne),
        Some(&2),
        "the first chosen player should put two counters on a creature they control"
    );
    assert_eq!(
        game.player(charlie).expect("charlie exists").hand.len(),
        hand_before + 1,
        "the second chosen player should draw a card"
    );
    assert_eq!(
        count_battlefield_name(&game, alice, "Treasure"),
        treasures_before + 2,
        "the third chosen player should create two Treasure tokens"
    );
}

#[test]
fn choose_player_split_the_party_returns_half_the_chosen_players_creatures_rounded_up() {
    let mut game = setup_three_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let split_the_party = parse_sorcery_definition(
        "Split the Party",
        "Choose target player. Return half the creatures they control to their owner's hand, rounded up.",
    );
    create_creature(&mut game, "Bob Creature A", bob, 2, 2);
    create_creature(&mut game, "Bob Creature B", bob, 2, 2);
    create_creature(&mut game, "Bob Creature C", bob, 2, 2);

    let bob_hand_before = game.player(bob).expect("bob exists").hand.len();
    let bob_battlefield_before = count_battlefield_name(&game, bob, "Bob Creature A")
        + count_battlefield_name(&game, bob, "Bob Creature B")
        + count_battlefield_name(&game, bob, "Bob Creature C");

    let mut dm = ScriptedDecisionMaker::new(&["Bob"], &[]);
    resolve_spell_definition_with_dm(&mut game, &split_the_party, alice, &mut dm);

    assert_eq!(
        game.player(bob).expect("bob exists").hand.len(),
        bob_hand_before + 2,
        "Split the Party should return half of Bob's three creatures, rounded up"
    );
    let bob_battlefield_after = count_battlefield_name(&game, bob, "Bob Creature A")
        + count_battlefield_name(&game, bob, "Bob Creature B")
        + count_battlefield_name(&game, bob, "Bob Creature C");
    assert_eq!(
        bob_battlefield_after,
        bob_battlefield_before - 2,
        "Split the Party should leave exactly one of Bob's three creatures on the battlefield"
    );
}

#[test]
fn choose_player_saskia_redirects_combat_damage_to_the_chosen_player() {
    let mut game = setup_three_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);

    let saskia = parse_creature_definition(
        "Saskia the Unyielding",
        3,
        4,
        "Vigilance, haste\nAs this creature enters, choose a player.\nWhenever a creature you control deals combat damage to a player, it deals that much damage to the chosen player.",
    );
    let mut dm = ScriptedDecisionMaker::new(&["Bob"], &[]);
    let saskia_id = move_definition_to_battlefield_with_dm(&mut game, &saskia, alice, &mut dm);
    let attacker_id = create_creature(&mut game, "Alice Attacker", alice, 4, 4);

    let bob_life_before = game.player(bob).expect("bob exists").life;
    let damage_event = TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            attacker_id,
            DamageTarget::Player(charlie),
            4,
            true,
            crate::events::cause::EventCause::combat_damage(attacker_id),
        ),
        ProvNodeId::default(),
    );
    let mut trigger_queue = TriggerQueue::new();
    queue_triggers_from_event(&mut game, &mut trigger_queue, damage_event, false);

    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "combat damage from your creature should trigger Saskia"
    );
    assert_eq!(
        trigger_queue.entries[0].source, saskia_id,
        "the queued trigger should come from Saskia"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue).expect("Saskia trigger should stack");
    resolve_stack_entry(&mut game).expect("Saskia trigger should resolve");

    assert_eq!(
        game.player(bob).expect("bob exists").life,
        bob_life_before - 4,
        "Saskia should deal the same combat damage amount to the chosen player"
    );
}

#[test]
fn choose_player_sewer_nemesis_only_triggers_for_the_chosen_players_spells() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let sewer_nemesis = parse_creature_definition(
        "Sewer Nemesis",
        0,
        0,
        "As this creature enters, choose a player.\nSewer Nemesis's power and toughness are each equal to the number of cards in the chosen player's graveyard.\nWhenever the chosen player casts a spell, that player mills a card.",
    );
    let mut dm = ScriptedDecisionMaker::new(&["Bob"], &[]);
    let sewer_id =
        move_definition_to_battlefield_with_dm(&mut game, &sewer_nemesis, alice, &mut dm);

    let bob_library_card = crate::card::CardBuilder::new(CardId::new(), "Bob Mill Card")
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_card(&bob_library_card, bob, Zone::Library);

    let bob_spell = crate::card::CardBuilder::new(CardId::new(), "Bob Spell")
        .card_types(vec![CardType::Sorcery])
        .build();
    let bob_spell_id = game.create_object_from_card(&bob_spell, bob, Zone::Stack);

    let mut trigger_queue = TriggerQueue::new();
    let bob_cast_event = TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new(bob_spell_id, bob, Zone::Hand),
        ProvNodeId::default(),
    );
    queue_triggers_from_event(&mut game, &mut trigger_queue, bob_cast_event, false);
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "the chosen player's spell should trigger Sewer Nemesis"
    );

    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Sewer Nemesis trigger should go on the stack");
    resolve_stack_entry(&mut game).expect("Sewer Nemesis trigger should resolve");

    assert_eq!(
        game.player(bob).expect("bob exists").graveyard.len(),
        1,
        "the chosen player should mill one card"
    );

    let alice_spell = crate::card::CardBuilder::new(CardId::new(), "Alice Spell")
        .card_types(vec![CardType::Sorcery])
        .build();
    let alice_spell_id = game.create_object_from_card(&alice_spell, alice, Zone::Stack);
    let alice_cast_event = TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new(alice_spell_id, alice, Zone::Hand),
        ProvNodeId::default(),
    );
    let mut second_queue = TriggerQueue::new();
    queue_triggers_from_event(&mut game, &mut second_queue, alice_cast_event, false);

    assert!(
        second_queue.entries.is_empty(),
        "a non-chosen player's spell should not trigger Sewer Nemesis"
    );
    assert_eq!(
        game.chosen_player(sewer_id),
        Some(bob),
        "the chosen player should stay linked to Sewer Nemesis after the trigger resolves"
    );
}

#[test]
fn choose_player_backdraft_uses_the_selected_sorcerys_damage_history() {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let big_sorcery =
        parse_sorcery_definition("Big Sorcery", "This spell deals 5 damage to each opponent.");
    let small_sorcery = parse_sorcery_definition(
        "Small Sorcery",
        "This spell deals 2 damage to each opponent.",
    );
    let backdraft = parse_sorcery_definition(
        "Backdraft",
        "Choose a player who cast one or more sorcery spells this turn. Backdraft deals damage to that player equal to half the damage dealt by one of those sorcery spells this turn, rounded down.",
    );

    let mut auto_dm = crate::decision::SelectFirstDecisionMaker;
    let big_sorcery_id =
        resolve_spell_definition_with_dm(&mut game, &big_sorcery, bob, &mut auto_dm);
    let small_sorcery_id =
        resolve_spell_definition_with_dm(&mut game, &small_sorcery, bob, &mut auto_dm);
    assert_eq!(
        game.turn_history
            .damage_dealt_by_spell_this_turn(&game.provenance_graph, big_sorcery_id),
        5,
        "the first sorcery's dealt damage should be queryable from turn history"
    );
    assert_eq!(
        game.turn_history
            .damage_dealt_by_spell_this_turn(&game.provenance_graph, small_sorcery_id),
        2,
        "the second sorcery's dealt damage should be queryable from turn history"
    );

    let bob_life_before = game.player(bob).expect("bob exists").life;
    let mut dm = ScriptedDecisionMaker::new(&["Bob", "cast #2"], &[]);
    let backdraft_id = game.create_object_from_definition(&backdraft, alice, Zone::Stack);
    register_spell_cast_this_turn_for_test(&mut game, backdraft_id, alice);
    let effects = backdraft.spell_effect.as_ref().expect("Backdraft effects");
    let choose_player = effects[0]
        .downcast_ref::<crate::effects::ChoosePlayerEffect>()
        .expect("first Backdraft effect should choose a player");
    let choose_spell = effects[1]
        .downcast_ref::<crate::effects::ChooseSpellCastHistoryEffect>()
        .expect("second Backdraft effect should choose a historical spell");

    let mut ctx = ExecutionContext::new_default(backdraft_id, alice).with_decision_maker(&mut dm);
    execute_effect(&mut game, &effects[0], &mut ctx).expect("Backdraft should choose a player");
    let tagged_players = ctx
        .get_tagged_players(choose_player.tag.as_str())
        .expect("Backdraft should tag the chosen qualifying player");
    assert_eq!(tagged_players.as_slice(), &[bob]);

    execute_effect(&mut game, &effects[1], &mut ctx)
        .expect("Backdraft should choose one of that player's sorcery spells");
    let chosen_spell = ctx
        .get_tagged(choose_spell.tag.as_str())
        .expect("Backdraft should tag the chosen spell-cast snapshot");
    assert_eq!(
        chosen_spell.cast_order_this_turn,
        Some(2),
        "Backdraft should keep the selected spell-cast instance, not just the player"
    );

    let damage_outcome = execute_effect(&mut game, &effects[2], &mut ctx)
        .expect("Backdraft should deal damage from the chosen spell history");
    assert_eq!(
        damage_outcome.count_or_zero(),
        1,
        "Backdraft's damage effect should resolve for the chosen spell's halved damage"
    );

    assert_eq!(
        game.player(bob).expect("bob exists").life,
        bob_life_before - 1,
        "Backdraft should deal half the damage of the selected sorcery spell, rounded down"
    );
}
