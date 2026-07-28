#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::shard_02::*;
use super::shard_03::*;
use super::shard_04::*;
use super::shard_05::*;
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
use super::shard_19::*;
use super::shard_20::*;
use super::shard_21::*;
use super::shard_22::*;
use super::shard_23::*;
use super::*;

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn path_of_the_pyromancer_runtime_discards_adds_mana_draws_and_planeswalks() {
    let (game, events) = resolve_path_of_the_pyromancer_with_votes(vec![0, 0]);
    let alice = PlayerId::from_index(0);
    let player = game.player(alice).expect("Alice should exist");

    assert_eq!(player.hand.len(), 4, "discard 3, then draw 4 cards");
    assert_eq!(player.library.len(), 0, "four cards should be drawn");
    assert_eq!(
        player.mana_pool.red, 3,
        "three discarded cards should add {{R}}{{R}}{{R}}"
    );
    assert_eq!(
        keyword_action_count(&events, crate::events::KeywordActionKind::Planeswalk),
        1,
        "planeswalk should happen when planeswalk gets more votes"
    );
    assert_eq!(
        keyword_action_count(&events, crate::events::KeywordActionKind::ChaosEnsues),
        0,
        "chaos should not happen when planeswalk gets more votes"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn path_of_the_pyromancer_runtime_tied_vote_chaos_branch() {
    let (_game, events) = resolve_path_of_the_pyromancer_with_votes(vec![0, 1]);

    assert_eq!(
        keyword_action_count(&events, crate::events::KeywordActionKind::Planeswalk),
        0,
        "planeswalk should not happen when its vote is tied"
    );
    assert_eq!(
        keyword_action_count(&events, crate::events::KeywordActionKind::ChaosEnsues),
        1,
        "chaos should happen when the vote is tied"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn path_of_the_pyromancer_runtime_chaos_more_votes_branch() {
    let (_game, events) = resolve_path_of_the_pyromancer_with_votes(vec![1, 1]);

    assert_eq!(
        keyword_action_count(&events, crate::events::KeywordActionKind::Planeswalk),
        0,
        "planeswalk should not happen when chaos gets more votes"
    );
    assert_eq!(
        keyword_action_count(&events, crate::events::KeywordActionKind::ChaosEnsues),
        1,
        "chaos should happen when chaos gets more votes"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn expert_level_safe_strict_parse_and_render_secret_choice_match() {
    assert_oracle_card_parses_strict("Expert-Level Safe");
    let def = parse_oracle_card_definition("Expert-Level Safe");
    let rendered = unprocessed_compiled_lines(&def).join("\n");

    assert!(
        rendered.contains(
            "You and target opponent each secretly choose 1, 2, or 3. Then those choices are revealed. If they match, sacrifice this artifact and put all cards exiled with it into their owners' hands. Otherwise, exile the top card of your library face down"
        ),
        "expected Expert-Level Safe to render the scoped secret-choice branch, got {rendered}"
    );

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("SecretChoiceEffect")
            && debug.contains("participants")
            && debug.contains("SecretChoicesMatch")
            && debug.contains("__source_exiled__"),
        "expected Expert-Level Safe to keep scoped secret choice, match condition, and source-exiled return structurally, got {debug}"
    );
}

#[test]
pub(super) fn pinnacle_starcage_strict_parser_and_text_regression() {
    let def = parse_oracle_card_definition("Pinnacle Starcage");
    let lines = canonical_compiled_lines(&def);
    let expected_enters = "When this artifact enters, exile all artifacts and creatures with mana value 2 or less until this artifact leaves the battlefield.";
    let expected_activated = "{6}{W}{W}: Put each card exiled with this artifact into its owner's graveyard, then create a 2/2 colorless Robot artifact creature token for each card put into a graveyard this way. Sacrifice this artifact.";

    assert_eq!(lines, vec![expected_enters, expected_activated]);

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("ExileUntilEffect")
            && debug.contains("SourceLeavesBattlefield")
            && debug.contains("MoveToZoneEffect")
            && debug.contains("WithIdEffect")
            && debug.contains("EffectMetric")
            && debug.contains("AffectedObjects")
            && debug.contains("CreateTokenEffect")
            && debug.contains("SacrificeTargetEffect"),
        "expected Pinnacle Starcage to keep source-leaves exile, affected-object token count, and source sacrifice, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct ExpertLevelSafeDecisionMaker {
    pub(super) votes: Vec<usize>,
}

#[cfg(ironsmith_runtime_parser_tests)]
impl crate::decision::DecisionMaker for ExpertLevelSafeDecisionMaker {
    fn decide_options(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        if !self.votes.is_empty() {
            vec![self.votes.remove(0)]
        } else {
            ctx.options
                .iter()
                .filter(|option| option.legal)
                .map(|option| option.index)
                .take(ctx.min)
                .collect()
        }
    }

    fn decide_objects(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        ctx.candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .take(ctx.min)
            .collect()
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn expert_level_safe_filler(id: u32, name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(id), name)
        .card_types(vec![CardType::Creature])
        .build()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resolve_expert_level_safe_with_votes(
    votes: Vec<usize>,
) -> (crate::game_state::GameState, ObjectId) {
    let def = parse_oracle_card_definition("Expert-Level Safe");
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Expert-Level Safe should have an enters trigger");
    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Expert-Level Safe should have an activated ability");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    for idx in 0..3 {
        let filler = expert_level_safe_filler(91_600 + idx, &format!("Safe Filler {idx}"));
        game.create_object_from_definition(&filler, alice, Zone::Library);
    }

    let mut dm = ExpertLevelSafeDecisionMaker { votes: Vec::new() };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        &triggered.effects,
        None,
        &[],
    )
    .expect("Expert-Level Safe enters trigger should resolve");
    assert_eq!(
        game.get_exiled_with_source_links(source).len(),
        2,
        "enters trigger should exile and link the top two library cards"
    );
    for &exiled in game.get_exiled_with_source_links(source) {
        assert!(
            game.is_face_down(exiled),
            "linked exiled cards should be face down"
        );
    }

    let mut dm = ExpertLevelSafeDecisionMaker { votes };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm)
        .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)]);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        &activated.effects,
        None,
        &[],
    )
    .expect("Expert-Level Safe activated ability should resolve");
    (game, source)
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn pinnacle_starcage_enter_exiles_all_matching_permanents_until_source_leaves() {
    let def = parse_oracle_card_definition("Pinnacle Starcage");
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Pinnacle Starcage should have an enters trigger");

    let alice = PlayerId::from_index(0);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let cheap_artifact = CardDefinitionBuilder::new(CardId::from_raw(91_197), "Cheap Artifact")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .card_types(vec![CardType::Artifact])
        .build();
    let cheap_creature = CardDefinitionBuilder::new(CardId::from_raw(91_198), "Cheap Creature")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let expensive_creature =
        CardDefinitionBuilder::new(CardId::from_raw(91_199), "Expensive Creature")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 3))
            .build();
    game.create_object_from_definition(&cheap_artifact, alice, Zone::Battlefield);
    game.create_object_from_definition(&cheap_creature, alice, Zone::Battlefield);
    game.create_object_from_definition(&expensive_creature, alice, Zone::Battlefield);

    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        &triggered.effects,
        None,
        &[],
    )
    .expect("Pinnacle Starcage enters trigger should resolve");

    let exile_names = game
        .exile
        .iter()
        .filter_map(|&id| game.object(id).map(|object| object.name.as_str()))
        .collect::<Vec<_>>();
    assert!(exile_names.contains(&"Cheap Artifact"));
    assert!(exile_names.contains(&"Cheap Creature"));
    assert!(!exile_names.contains(&"Expensive Creature"));

    game.move_object_by_effect(source, Zone::Graveyard);
    game.return_exiled_for_source_leave(source);

    let battlefield_names = game
        .battlefield
        .iter()
        .filter_map(|&id| game.object(id).map(|object| object.name.as_str()))
        .collect::<Vec<_>>();
    assert!(battlefield_names.contains(&"Cheap Artifact"));
    assert!(battlefield_names.contains(&"Cheap Creature"));
    assert!(battlefield_names.contains(&"Expensive Creature"));
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resolve_pinnacle_starcage_activation_with_exiled_cards(
    exiled_count: usize,
) -> crate::game_state::GameState {
    use crate::effects::ExecutionContext;
    use crate::object::ObjectKind;

    let def = parse_oracle_card_definition("Pinnacle Starcage");
    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Pinnacle Starcage should have an activated ability");

    let alice = PlayerId::from_index(0);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let exiled_card = CardDefinitionBuilder::new(CardId::from_raw(91_200), "Exiled Trinket")
        .card_types(vec![CardType::Artifact])
        .build();
    for _ in 0..exiled_count {
        let exiled_id = game.create_object_from_definition(&exiled_card, alice, Zone::Exile);
        game.add_exiled_with_source_link(source, exiled_id);
    }

    let mut ctx = ExecutionContext::new_default(source, alice);
    for effect in &activated.effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Pinnacle Starcage activation effect should resolve");
    }

    let robot_count = game
        .battlefield
        .iter()
        .filter_map(|&id| game.object(id))
        .filter(|object| object.kind == ObjectKind::Token && object.name == "Robot")
        .count();
    assert_eq!(
        robot_count, exiled_count,
        "Pinnacle Starcage should create one Robot for each card put into a graveyard this way"
    );

    game
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn expert_level_safe_runtime_matching_choices_sacrifice_and_return_exiled_cards() {
    let (game, source) = resolve_expert_level_safe_with_votes(vec![0, 0]);
    let alice = PlayerId::from_index(0);

    assert!(
        game.get_exiled_with_source_links(source).is_empty(),
        "matching choices should return every card exiled with Expert-Level Safe"
    );
    assert_eq!(
        game.player(alice).expect("Alice should exist").hand.len(),
        2,
        "matching choices should put the two linked exiled cards into their owner's hand"
    );
    assert!(
        game.objects_in_zone(Zone::Graveyard).into_iter().any(|id| {
            game.object(id)
                .is_some_and(|object| object.name == "Expert-Level Safe")
        }),
        "matching choices should sacrifice Expert-Level Safe"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn pinnacle_starcage_activation_moves_exiled_cards_creates_robots_and_sacrifices_source()
{
    use crate::object::ObjectKind;

    let game = resolve_pinnacle_starcage_activation_with_exiled_cards(2);
    let alice = PlayerId::from_index(0);

    let graveyard_names = game
        .player(alice)
        .expect("Alice should exist")
        .graveyard
        .iter()
        .filter_map(|&id| game.object(id).map(|object| object.name.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(
        graveyard_names
            .iter()
            .filter(|name| **name == "Exiled Trinket")
            .count(),
        2,
        "activation should put each exiled-with-source card into its owner's graveyard"
    );
    assert!(
        graveyard_names.contains(&"Pinnacle Starcage"),
        "activation should sacrifice Pinnacle Starcage after creating tokens"
    );
    assert_eq!(
        game.battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|object| object.kind == ObjectKind::Token && object.name == "Robot")
            .count(),
        2,
        "expected two Robot tokens from two moved exiled cards"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn expert_level_safe_runtime_nonmatching_choices_exile_another_card() {
    let (game, source) = resolve_expert_level_safe_with_votes(vec![0, 1]);
    let alice = PlayerId::from_index(0);

    assert_eq!(
        game.get_exiled_with_source_links(source).len(),
        3,
        "nonmatching choices should keep the two original cards and exile one more linked card"
    );
    assert_eq!(
        game.player(alice).expect("Alice should exist").hand.len(),
        0,
        "nonmatching choices should not return the linked exiled cards"
    );
    assert_eq!(
        game.player(alice)
            .expect("Alice should exist")
            .library
            .len(),
        0,
        "nonmatching choices should exile the last library card"
    );
    assert!(
        game.objects_in_zone(Zone::Battlefield)
            .into_iter()
            .any(|id| id == source),
        "nonmatching choices should leave Expert-Level Safe on the battlefield"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn pinnacle_starcage_activation_returns_mixed_owner_cards_to_their_own_graveyards() {
    use crate::effects::ExecutionContext;
    use crate::object::ObjectKind;

    let def = parse_oracle_card_definition("Pinnacle Starcage");
    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Pinnacle Starcage should have an activated ability");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let alice_card = CardDefinitionBuilder::new(CardId::from_raw(91_201), "Alice Trinket")
        .card_types(vec![CardType::Artifact])
        .build();
    let bob_card = CardDefinitionBuilder::new(CardId::from_raw(91_202), "Bob Trinket")
        .card_types(vec![CardType::Artifact])
        .build();
    let alice_exiled = game.create_object_from_definition(&alice_card, alice, Zone::Exile);
    let bob_exiled = game.create_object_from_definition(&bob_card, bob, Zone::Exile);
    game.add_exiled_with_source_link(source, alice_exiled);
    game.add_exiled_with_source_link(source, bob_exiled);

    let mut ctx = ExecutionContext::new_default(source, alice);
    for effect in &activated.effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Pinnacle Starcage activation effect should resolve");
    }

    let alice_graveyard_names = game
        .player(alice)
        .expect("Alice should exist")
        .graveyard
        .iter()
        .filter_map(|&id| game.object(id).map(|object| object.name.as_str()))
        .collect::<Vec<_>>();
    let bob_graveyard_names = game
        .player(bob)
        .expect("Bob should exist")
        .graveyard
        .iter()
        .filter_map(|&id| game.object(id).map(|object| object.name.as_str()))
        .collect::<Vec<_>>();

    assert!(alice_graveyard_names.contains(&"Alice Trinket"));
    assert!(!alice_graveyard_names.contains(&"Bob Trinket"));
    assert!(bob_graveyard_names.contains(&"Bob Trinket"));
    assert!(!bob_graveyard_names.contains(&"Alice Trinket"));
    assert_eq!(
        game.battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|object| object.kind == ObjectKind::Token && object.name == "Robot")
            .count(),
        2,
        "expected one Robot for each card put into its owner's graveyard"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn pinnacle_starcage_activation_creates_no_robots_when_no_cards_are_moved_this_way() {
    let game = resolve_pinnacle_starcage_activation_with_exiled_cards(0);
    let alice = PlayerId::from_index(0);

    assert!(
        game.player(alice)
            .expect("Alice should exist")
            .graveyard
            .iter()
            .filter_map(|&id| game.object(id).map(|object| object.name.as_str()))
            .any(|name| name == "Pinnacle Starcage"),
        "activation should still sacrifice Pinnacle Starcage when no exiled cards move"
    );
    assert!(
        game.battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .all(|object| object.name != "Robot"),
        "activation should not create Robot tokens when no cards were put into a graveyard this way"
    );
}

pub(super) fn vanilla_creature_for_when_we_were_young(id: u32, name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(id), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build()
}

pub(super) fn resolve_when_we_were_young_with_condition(
    controls_enchantment: bool,
) -> (crate::game_state::GameState, ObjectId, ObjectId, ObjectId) {
    struct NoopDecisionMaker;
    impl crate::decision::DecisionMaker for NoopDecisionMaker {}

    let def = parse_oracle_card_definition("When We Were Young");
    let program = def
        .spell_effect
        .as_ref()
        .expect("When We Were Young should compile to spell effects");
    let target_spec = (*program.segments[0].default_effects[0]
        .target_selection_profile()
        .expect("When We Were Young should require targets")
        .spec)
        .clone();

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = game.create_object_from_definition(&def, alice, Zone::Stack);
    let first = game.create_object_from_definition(
        &vanilla_creature_for_when_we_were_young(91_500, "First Target"),
        alice,
        Zone::Battlefield,
    );
    let second = game.create_object_from_definition(
        &vanilla_creature_for_when_we_were_young(91_501, "Second Target"),
        bob,
        Zone::Battlefield,
    );
    let unselected = game.create_object_from_definition(
        &vanilla_creature_for_when_we_were_young(91_502, "Unselected Creature"),
        bob,
        Zone::Battlefield,
    );

    game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_503), "Condition Artifact")
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Battlefield,
    );
    if controls_enchantment {
        game.create_object_from_definition(
            &CardDefinitionBuilder::new(CardId::from_raw(91_504), "Condition Enchantment")
                .card_types(vec![CardType::Enchantment])
                .build(),
            alice,
            Zone::Battlefield,
        );
    }

    let mut dm = NoopDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
    ctx.targets = vec![
        crate::effects::ResolvedTarget::Object(first),
        crate::effects::ResolvedTarget::Object(second),
    ];
    let assignments = vec![crate::game_state::TargetAssignment {
        spec: target_spec,
        range: 0..2,
    }];
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        program,
        None,
        &assignments,
    )
    .expect("When We Were Young should resolve");

    (game, first, second, unselected)
}

#[test]
pub(super) fn when_we_were_young_pumps_two_targets_and_conditionally_grants_lifelink() {
    let (game, first, second, unselected) = resolve_when_we_were_young_with_condition(true);

    assert_eq!(game.current_power(first), Some(3));
    assert_eq!(game.current_toughness(first), Some(3));
    assert_eq!(game.current_power(second), Some(3));
    assert_eq!(game.current_toughness(second), Some(3));
    assert!(game.current_has_static_ability_id(first, StaticAbilityId::Lifelink));
    assert!(game.current_has_static_ability_id(second, StaticAbilityId::Lifelink));

    assert_eq!(game.current_power(unselected), Some(1));
    assert_eq!(game.current_toughness(unselected), Some(1));
    assert!(!game.current_has_static_ability_id(unselected, StaticAbilityId::Lifelink));
}

#[test]
pub(super) fn when_we_were_young_without_artifact_enchantment_condition_only_pumps_targets() {
    let (game, first, second, unselected) = resolve_when_we_were_young_with_condition(false);

    assert_eq!(game.current_power(first), Some(3));
    assert_eq!(game.current_toughness(first), Some(3));
    assert_eq!(game.current_power(second), Some(3));
    assert_eq!(game.current_toughness(second), Some(3));
    assert!(!game.current_has_static_ability_id(first, StaticAbilityId::Lifelink));
    assert!(!game.current_has_static_ability_id(second, StaticAbilityId::Lifelink));

    assert_eq!(game.current_power(unselected), Some(1));
    assert_eq!(game.current_toughness(unselected), Some(1));
    assert!(!game.current_has_static_ability_id(unselected, StaticAbilityId::Lifelink));
}

#[test]
pub(super) fn guardian_of_the_ages_strict_parser_and_text_regression() {
    let def = parse_oracle_card_definition("Guardian of the Ages");

    let rendered_lines = canonical_compiled_lines(&def);
    assert_eq!(
        rendered_lines,
        vec![
            "Defender".to_string(),
            "Whenever a creature attacks you or a planeswalker you control, if this creature has defender, this creature loses defender and this creature gains trample.".to_string(),
        ],
        "Guardian of the Ages should render the source-keyword intervening-if clause"
    );

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Guardian of the Ages should have an attack trigger");
    assert!(
        matches!(
            triggered.intervening_if.as_ref(),
            Some(crate::ConditionExpr::SourceMatches(filter))
                if filter.card_types.is_empty()
                    && filter.static_abilities.as_slice() == [StaticAbilityId::Defender]
        ),
        "Guardian of the Ages should gate the trigger on the source having defender, got {:?}",
        triggered.intervening_if
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn valiant_endeavor_strict_parser_and_text_regression() {
    let def = parse_oracle_card_definition("Valiant Endeavor");

    let rendered_lines = canonical_compiled_lines(&def);
    assert_eq!(
        rendered_lines,
        vec![
            "Roll two d6 and choose one result. Destroy each creature with power greater than or equal to that result. Create a number of 2/2 white Knight creature tokens with vigilance equal to the other result.".to_string(),
        ],
        "Valiant Endeavor should render the dice choice, destroy threshold, and other-result token count"
    );
}

#[test]
pub(super) fn calamity_bearer_strict_parser_and_text_regression() {
    let def = parse_oracle_card_definition("Calamity Bearer");
    let rendered_lines = canonical_compiled_lines(&def);

    assert!(
        def.abilities.iter().any(|ability| matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::ModifyDamageAmountReplacement
        )),
        "Calamity Bearer should compile to a double-damage replacement static ability"
    );
    assert!(
        rendered_lines.iter().any(|line| line
            == "If a giant source you control would deal damage to a permanent or player, it deals double that damage to that permanent or player instead."),
        "Calamity Bearer should render its double-damage replacement clause, got {rendered_lines:?}"
    );
}

#[test]
pub(super) fn zilortha_strength_incarnate_strict_parser_and_text_regression() {
    let def = parse_oracle_card_definition("Zilortha, Strength Incarnate");
    let rendered_lines = canonical_compiled_lines(&def);

    assert!(
        def.abilities.iter().any(|ability| matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::LethalDamageToCreaturesYouControlUsesPower
        )),
        "Zilortha, Strength Incarnate should compile to a lethal-damage power static ability"
    );
    assert!(
        rendered_lines.iter().any(|line| line
            == "Lethal damage dealt to creatures you control is determined by their power rather than their toughness."),
        "Zilortha, Strength Incarnate should render its lethal-damage clause, got {rendered_lines:?}"
    );
}

#[test]
pub(super) fn calamity_bearer_runtime_doubles_giant_source_damage_to_players_and_permanents() {
    let def = parse_oracle_card_definition("Calamity Bearer");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let giant_source = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_300), "Alice Giant")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Giant])
            .power_toughness(PowerToughness::fixed(3, 3))
            .build(),
        alice,
        Zone::Battlefield,
    );
    let target_permanent = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_301), "Target Permanent")
            .card_types(vec![CardType::Artifact])
            .build(),
        bob,
        Zone::Battlefield,
    );
    let controller_permanent = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_304), "Controller Permanent")
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Battlefield,
    );

    let player_damage = crate::events::processing::process_damage_assignments_with_event(
        &mut game,
        giant_source,
        crate::events::DamageTarget::Player(bob),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(player_damage.assignments.len(), 1);
    assert_eq!(player_damage.assignments[0].amount, 6);

    let controller_damage = crate::events::processing::process_damage_assignments_with_event(
        &mut game,
        giant_source,
        crate::events::DamageTarget::Player(alice),
        1,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(controller_damage.assignments.len(), 1);
    assert_eq!(controller_damage.assignments[0].amount, 2);

    let permanent_damage = crate::events::processing::process_damage_assignments_with_event(
        &mut game,
        giant_source,
        crate::events::DamageTarget::Object(target_permanent),
        2,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(permanent_damage.assignments.len(), 1);
    assert_eq!(permanent_damage.assignments[0].amount, 4);

    let controller_permanent_damage =
        crate::events::processing::process_damage_assignments_with_event(
            &mut game,
            giant_source,
            crate::events::DamageTarget::Object(controller_permanent),
            1,
            false,
            crate::events::cause::EventCause::effect(),
        );
    assert_eq!(controller_permanent_damage.assignments.len(), 1);
    assert_eq!(controller_permanent_damage.assignments[0].amount, 2);
}

#[test]
pub(super) fn calamity_bearer_runtime_ignores_non_giant_and_opposing_giant_sources() {
    let def = parse_oracle_card_definition("Calamity Bearer");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let non_giant_source = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_302), "Alice Soldier")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Soldier])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        alice,
        Zone::Battlefield,
    );
    let opposing_giant_source = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_303), "Bob Giant")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Giant])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        bob,
        Zone::Battlefield,
    );

    let non_giant_damage = crate::events::processing::process_damage_assignments_with_event(
        &mut game,
        non_giant_source,
        crate::events::DamageTarget::Player(bob),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(non_giant_damage.assignments.len(), 1);
    assert_eq!(non_giant_damage.assignments[0].amount, 3);

    let opposing_giant_damage = crate::events::processing::process_damage_assignments_with_event(
        &mut game,
        opposing_giant_source,
        crate::events::DamageTarget::Player(alice),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(opposing_giant_damage.assignments.len(), 1);
    assert_eq!(opposing_giant_damage.assignments[0].amount, 3);
}

#[test]
pub(super) fn torture_pit_adds_two_to_noncombat_damage_to_opponents_only() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(91_306), "Torture Pit Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "If a source you control would deal noncombat damage to an opponent, it deals that much damage plus 2 instead.",
        )
        .expect("Torture Pit-style additive noncombat replacement should parse");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let debug = format!("{:#?}", def.abilities);

    assert!(
        rendered.contains(
            "If a source you control would deal noncombat damage to an opponent, it deals that much damage plus 2 instead"
        ),
        "Torture Pit should render additive noncombat damage replacement, got {rendered}"
    );
    assert!(
        debug.contains("ModifyDamageAmountReplacement")
            && debug.contains("delta: 2")
            && debug.contains("noncombat_only: true")
            && debug.contains("target_player_filter: Some"),
        "Torture Pit should compile to noncombat-only +2 damage replacement, got {debug}"
    );

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let source = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_307), "Alice Damage Source")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        alice,
        Zone::Battlefield,
    );

    let noncombat_to_opponent = crate::events::processing::process_damage_assignments_with_event(
        &mut game,
        source,
        crate::events::DamageTarget::Player(bob),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(noncombat_to_opponent.assignments.len(), 1);
    assert_eq!(noncombat_to_opponent.assignments[0].amount, 5);

    let combat_to_opponent = crate::events::processing::process_damage_assignments_with_event(
        &mut game,
        source,
        crate::events::DamageTarget::Player(bob),
        3,
        true,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(combat_to_opponent.assignments.len(), 1);
    assert_eq!(combat_to_opponent.assignments[0].amount, 3);

    let noncombat_to_controller = crate::events::processing::process_damage_assignments_with_event(
        &mut game,
        source,
        crate::events::DamageTarget::Player(alice),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(noncombat_to_controller.assignments.len(), 1);
    assert_eq!(noncombat_to_controller.assignments[0].amount, 3);
}

#[test]
pub(super) fn aftermath_grants_source_cast_from_graveyard_and_exiles_after_resolution() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(91_308), "Aftermath Probe")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Aftermath (Cast this spell only from your graveyard. Then exile it.)\n\
             Target creature you control fights target creature an opponent controls.",
        )
        .expect("aftermath keyword line should parse");
    let debug = format!("{:#?}", def.abilities);
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let aftermath_grant = def
        .abilities
        .iter()
        .find(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id() == crate::static_abilities::StaticAbilityId::Grants
            )
        })
        .expect("expected aftermath to lower to a Grants ability");

    assert!(
        debug.contains("GraveyardCastFromCardManaCost")
            && debug.contains("exiles_after_resolution: true"),
        "Aftermath should lower to a graveyard self-cast grant active from graveyard, got {debug}"
    );
    assert_eq!(aftermath_grant.functional_zones, vec![Zone::Graveyard]);
    assert!(
        rendered.contains("Aftermath")
            && !rendered.contains("You may cast this card from your graveyard")
            && rendered.contains("fights target creature an opponent controls"),
        "Aftermath compiled text should preserve the keyword surface while the model carries graveyard/exile semantics, got {rendered}"
    );
}

#[test]
pub(super) fn charging_tuskodon_strict_parser_compiled_text_and_model_regression() {
    assert_oracle_card_parses_strict("Charging Tuskodon");
    let def = parse_oracle_card_definition("Charging Tuskodon");
    let rendered_lines = canonical_compiled_lines(&def);

    assert!(
        rendered_lines.iter().any(|line| line == "Trample"),
        "Charging Tuskodon should render trample, got {rendered_lines:?}"
    );
    assert!(
        rendered_lines.iter().any(|line| line
            == "If this creature would deal combat damage to a player, it deals double that damage to that player instead."),
        "Charging Tuskodon should render its combat-damage replacement clause, got {rendered_lines:?}"
    );

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("DoubleDamageAmountReplacement")
            && debug.contains("factor: 2")
            && debug.contains("combat_only: true")
            && debug.contains("target_player_filter: Some")
            && debug.contains("target_object_filter: None"),
        "Charging Tuskodon should compile to a combat-only player damage multiplier, got {debug}"
    );
}

#[test]
pub(super) fn charging_tuskodon_runtime_doubles_only_its_combat_damage_to_players() {
    let def = parse_oracle_card_definition("Charging Tuskodon");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let tuskodon = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let other_creature = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_305), "Other Attacker")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        alice,
        Zone::Battlefield,
    );
    let target_permanent = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_306), "Bob Permanent")
            .card_types(vec![CardType::Artifact])
            .build(),
        bob,
        Zone::Battlefield,
    );

    let combat_to_player = crate::events::processing::process_damage_assignments_with_event(
        &mut game,
        tuskodon,
        crate::events::DamageTarget::Player(bob),
        4,
        true,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(combat_to_player.assignments.len(), 1);
    assert_eq!(combat_to_player.assignments[0].amount, 8);

    let noncombat_to_player = crate::events::processing::process_damage_assignments_with_event(
        &mut game,
        tuskodon,
        crate::events::DamageTarget::Player(bob),
        4,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(noncombat_to_player.assignments.len(), 1);
    assert_eq!(noncombat_to_player.assignments[0].amount, 4);

    let combat_to_permanent = crate::events::processing::process_damage_assignments_with_event(
        &mut game,
        tuskodon,
        crate::events::DamageTarget::Object(target_permanent),
        4,
        true,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(combat_to_permanent.assignments.len(), 1);
    assert_eq!(combat_to_permanent.assignments[0].amount, 4);

    let other_source_combat_to_player =
        crate::events::processing::process_damage_assignments_with_event(
            &mut game,
            other_creature,
            crate::events::DamageTarget::Player(bob),
            4,
            true,
            crate::events::cause::EventCause::effect(),
        );
    assert_eq!(other_source_combat_to_player.assignments.len(), 1);
    assert_eq!(other_source_combat_to_player.assignments[0].amount, 4);
}

#[test]
pub(super) fn rebbec_architect_of_ascension_strict_parser_and_text_regression() {
    let def = parse_oracle_card_definition("Rebbec, Architect of Ascension");

    let rendered_lines = canonical_compiled_lines(&def);
    assert_eq!(
        rendered_lines,
        vec![
            "Artifacts you control have protection from each mana value among artifacts you control."
                .to_string(),
            "Partner".to_string(),
        ],
        "Rebbec, Architect of Ascension should render mana-value protection exactly"
    );

    let ability_debug = format!("{:?}", def.abilities);
    assert!(
        ability_debug.contains("GrantObjectAbilityForFilter")
            && ability_debug.contains("EachManaValueAmong")
            && ability_debug.contains("Artifact"),
        "Rebbec, Architect of Ascension should structurally grant artifact mana-value protection, got {ability_debug}"
    );
}

pub(super) fn rebbec_runtime_test_card(
    name: &str,
    card_types: Vec<CardType>,
    mana_value: u8,
    power_toughness: Option<(i32, i32)>,
) -> CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(card_types)
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
            mana_value,
        )]]));
    if let Some((power, toughness)) = power_toughness {
        builder = builder.power_toughness(PowerToughness::fixed(power, toughness));
    }
    builder.build()
}

#[test]
pub(super) fn rebbec_architect_of_ascension_runtime_targeting_uses_controller_artifact_mana_values()
{
    let rebbec = parse_oracle_card_definition("Rebbec, Architect of Ascension");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);

    game.create_object_from_definition(&rebbec, alice, Zone::Battlefield);
    let protected_artifact = rebbec_runtime_test_card(
        "Alice Rebbec-Protected Mana Rock",
        vec![CardType::Artifact],
        2,
        None,
    );
    let protected_id =
        game.create_object_from_definition(&protected_artifact, alice, Zone::Battlefield);
    let matching_spell = rebbec_runtime_test_card(
        "Mana Value Two Targeting Spell",
        vec![CardType::Instant],
        2,
        None,
    );
    let matching_spell_id = game.create_object_from_definition(&matching_spell, bob, Zone::Stack);
    let nonmatching_spell = rebbec_runtime_test_card(
        "Mana Value Three Targeting Spell",
        vec![CardType::Instant],
        3,
        None,
    );
    let nonmatching_spell_id =
        game.create_object_from_definition(&nonmatching_spell, bob, Zone::Stack);
    let opponent_artifact = rebbec_runtime_test_card(
        "Opponent Artifact With Mana Value Three",
        vec![CardType::Artifact],
        3,
        None,
    );
    game.create_object_from_definition(&opponent_artifact, bob, Zone::Battlefield);
    let bob_artifact = rebbec_runtime_test_card(
        "Bob Artifact With Matching Mana Value",
        vec![CardType::Artifact],
        2,
        None,
    );
    let bob_artifact_id = game.create_object_from_definition(&bob_artifact, bob, Zone::Battlefield);

    assert!(
        !crate::targeting::can_target_object(&game, protected_id, matching_spell_id, bob)
            .is_legal(),
        "Rebbec, Architect of Ascension should make Alice's artifact illegal to target from sources whose mana value matches Alice's artifacts"
    );
    assert!(
        crate::targeting::can_target_object(&game, protected_id, nonmatching_spell_id, bob)
            .is_legal(),
        "Rebbec, Architect of Ascension should not count Bob's artifacts when checking Alice's protected artifact"
    );
    assert!(
        crate::targeting::can_target_object(&game, bob_artifact_id, matching_spell_id, bob)
            .is_legal(),
        "Rebbec, Architect of Ascension should not grant protection to artifacts controlled by another player"
    );
}

#[test]
pub(super) fn rebbec_architect_of_ascension_runtime_blocking_uses_controller_artifact_mana_values()
{
    let rebbec = parse_oracle_card_definition("Rebbec, Architect of Ascension");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);

    game.create_object_from_definition(&rebbec, alice, Zone::Battlefield);
    let attacker_def = rebbec_runtime_test_card(
        "Alice Rebbec-Protected Artifact Creature",
        vec![CardType::Artifact, CardType::Creature],
        2,
        Some((2, 2)),
    );
    let attacker_id = game.create_object_from_definition(&attacker_def, alice, Zone::Battlefield);
    let matching_blocker_def = rebbec_runtime_test_card(
        "Mana Value Two Blocker",
        vec![CardType::Creature],
        2,
        Some((2, 2)),
    );
    let matching_blocker_id =
        game.create_object_from_definition(&matching_blocker_def, bob, Zone::Battlefield);
    let nonmatching_blocker_def = rebbec_runtime_test_card(
        "Mana Value Three Blocker",
        vec![CardType::Creature],
        3,
        Some((3, 3)),
    );
    let nonmatching_blocker_id =
        game.create_object_from_definition(&nonmatching_blocker_def, bob, Zone::Battlefield);
    let opponent_artifact = rebbec_runtime_test_card(
        "Opponent Artifact With Mana Value Three",
        vec![CardType::Artifact],
        3,
        None,
    );
    game.create_object_from_definition(&opponent_artifact, bob, Zone::Battlefield);

    let attacker = game.object(attacker_id).expect("attacker exists").clone();
    let matching_blocker = game
        .object(matching_blocker_id)
        .expect("matching blocker exists")
        .clone();
    let nonmatching_blocker = game
        .object(nonmatching_blocker_id)
        .expect("nonmatching blocker exists")
        .clone();

    assert!(
        !crate::rules::combat::can_block(&attacker, &matching_blocker, &game),
        "Rebbec, Architect of Ascension should stop blockers whose mana value matches Alice's artifacts"
    );
    assert!(
        crate::rules::combat::can_block(&attacker, &nonmatching_blocker, &game),
        "Rebbec, Architect of Ascension should not count Bob's artifacts when checking Alice's protected attacker"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn trystan_faces_strict_parser_and_text_regression() {
    let front_oracle = "Deathtouch\n\
        Whenever this creature enters or transforms into Trystan, Callous Cultivator, mill three cards. Then if there is an Elf card in your graveyard, you gain 2 life.\n\
        At the beginning of your first main phase, you may pay {B}. If you do, transform Trystan.";
    let front = CardDefinitionBuilder::new(CardId::new(), "Trystan, Callous Cultivator")
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .parse_text(front_oracle)
        .expect("Trystan, Callous Cultivator should parse strictly");
    let front_rendered = canonical_compiled_lines(&front).join("\n");
    assert!(
        front_rendered.contains(
            "Whenever this creature enters or transforms into Trystan, Callous Cultivator"
        ) && front_rendered.contains("mill three cards")
            && front_rendered
                .contains("If there is an Elf card in your graveyard, you gain 2 life"),
        "expected front-face transform trigger and Elf-card graveyard condition, got {front_rendered}"
    );

    let back_oracle = "Deathtouch\n\
        Whenever this creature transforms into Trystan, Penitent Culler, mill three cards, then you may exile an Elf card from your graveyard. If you do, each opponent loses 2 life.\n\
        At the beginning of your first main phase, you may pay {G}. If you do, transform Trystan.";
    let back = CardDefinitionBuilder::new(CardId::new(), "Trystan, Penitent Culler")
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .parse_text(back_oracle)
        .expect("Trystan, Penitent Culler should parse strictly");
    let back_rendered = canonical_compiled_lines(&back).join("\n");
    assert!(
        back_rendered.contains("Whenever this creature transforms into Trystan, Penitent Culler")
            && back_rendered.contains("mill three cards")
            && back_rendered.contains("You may exile an Elf card")
            && back_rendered.contains("If you do, each opponent loses 2 life")
            && back_rendered.contains("At the beginning of your first main phase, you may pay {G}"),
        "expected back-face transform trigger, optional Elf exile, opponent life loss, and green transform trigger, got {back_rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_sporeweb_weaver_strict_regression() {
    assert_oracle_card_parses_strict("Sporeweb Weaver");
    let def = parse_oracle_card_definition("Sporeweb Weaver");
    let rendered_lines = canonical_compiled_lines(&def);

    assert!(
        def.abilities.iter().any(|ability| matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::Reach
        )),
        "expected Sporeweb Weaver to have reach"
    );
    let hexproof_from = def
        .abilities
        .iter()
        .find_map(|ability| {
            if let AbilityKind::Static(static_ability) = &ability.kind {
                if static_ability.id() == StaticAbilityId::HexproofFrom {
                    return static_ability.hexproof_from_filter();
                }
            }
            None
        })
        .expect("expected Sporeweb Weaver to have hexproof from blue");
    assert_eq!(
        hexproof_from.colors,
        Some(crate::color::ColorSet::BLUE),
        "expected Sporeweb Weaver hexproof-from filter to be exactly blue"
    );
    assert_eq!(rendered_lines[0], "Reach, hexproof from blue");
    assert!(
        rendered_lines[1].contains("Whenever this creature is dealt damage")
            && rendered_lines[1].contains("gain 1 life")
            && rendered_lines[1].contains("create a 1/1 green Saproling creature token"),
        "unexpected Sporeweb Weaver compiled text: {rendered_lines:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sporeweb_weaver_hexproof_from_blue_blocks_only_opposing_blue_sources() {
    let def = parse_oracle_card_definition("Sporeweb Weaver");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let weaver_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let blue_source = CardDefinitionBuilder::new(CardId::from_raw(91_200), "Blue Source")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Instant])
        .build();
    let red_source = CardDefinitionBuilder::new(CardId::from_raw(91_201), "Red Source")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .card_types(vec![CardType::Instant])
        .build();
    let opposing_blue = game.create_object_from_definition(&blue_source, bob, Zone::Battlefield);
    let opposing_red = game.create_object_from_definition(&red_source, bob, Zone::Battlefield);
    let own_blue = game.create_object_from_definition(&blue_source, alice, Zone::Battlefield);

    assert_eq!(
        crate::targeting::can_target_object(&game, weaver_id, opposing_blue, bob),
        crate::targeting::TargetingResult::Invalid(
            crate::targeting::TargetingInvalidReason::HasHexproofFrom
        ),
        "opposing blue source should be unable to target Sporeweb Weaver"
    );
    assert!(
        crate::targeting::can_target_object(&game, weaver_id, opposing_red, bob).is_legal(),
        "opposing nonblue source should be able to target Sporeweb Weaver"
    );
    assert!(
        crate::targeting::can_target_object(&game, weaver_id, own_blue, alice).is_legal(),
        "controller's blue source should be able to target their own Sporeweb Weaver"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn breaker_of_creation_strict_parser_text_and_structure_regression() {
    assert_oracle_card_parses_strict("Breaker of Creation");
    let def = parse_oracle_card_definition("Breaker of Creation");
    let rendered_lines = canonical_compiled_lines(&def);
    let rendered = rendered_lines.join("\n");

    assert!(
        rendered.contains(
            "When you cast this spell, you gain 1 life for each colorless permanent you control"
        ) && rendered.contains("Hexproof from each color")
            && rendered.contains("annihilator 2"),
        "expected Breaker of Creation cast trigger, hexproof-from-each-color, and annihilator text, got {rendered}"
    );
    assert!(
        !rendered.to_ascii_lowercase().contains("unsupported"),
        "strict Breaker of Creation render should not include unsupported markers, got {rendered}"
    );

    let all_colors = crate::color::ColorSet::WHITE
        .union(crate::color::ColorSet::BLUE)
        .union(crate::color::ColorSet::BLACK)
        .union(crate::color::ColorSet::RED)
        .union(crate::color::ColorSet::GREEN);
    let hexproof_from = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => {
                if static_ability.id() == StaticAbilityId::HexproofFrom {
                    static_ability.hexproof_from_filter()
                } else {
                    None
                }
            }
            _ => None,
        })
        .expect("Breaker of Creation should have hexproof from each color");
    let mut expected_filter = crate::target::ObjectFilter::default();
    expected_filter.colors = Some(all_colors);
    assert_eq!(
        hexproof_from, &expected_filter,
        "hexproof from each color should lower to exactly an all-colors source filter"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn breaker_of_creation_hexproof_from_each_color_blocks_only_opposing_colored_sources() {
    let def = parse_oracle_card_definition("Breaker of Creation");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let breaker_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    for (idx, symbol) in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
    ]
    .into_iter()
    .enumerate()
    {
        let source_def = CardDefinitionBuilder::new(
            CardId::from_raw(92_000 + idx as u32),
            format!("Opposing {symbol:?} Source"),
        )
        .mana_cost(ManaCost::from_pips(vec![vec![symbol]]))
        .card_types(vec![CardType::Instant])
        .build();
        let source_id = game.create_object_from_definition(&source_def, bob, Zone::Battlefield);
        assert_eq!(
            crate::targeting::can_target_object(&game, breaker_id, source_id, bob),
            crate::targeting::TargetingResult::Invalid(
                crate::targeting::TargetingInvalidReason::HasHexproofFrom
            ),
            "opposing {symbol:?} source should be unable to target Breaker of Creation"
        );
    }

    let colorless_source = CardDefinitionBuilder::new(CardId::from_raw(92_100), "Colorless Source")
        .card_types(vec![CardType::Artifact])
        .build();
    let opposing_colorless =
        game.create_object_from_definition(&colorless_source, bob, Zone::Battlefield);
    assert!(
        crate::targeting::can_target_object(&game, breaker_id, opposing_colorless, bob).is_legal(),
        "opposing colorless source should be able to target Breaker of Creation"
    );

    let own_red_source = CardDefinitionBuilder::new(CardId::from_raw(92_101), "Own Red Source")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .card_types(vec![CardType::Instant])
        .build();
    let own_red = game.create_object_from_definition(&own_red_source, alice, Zone::Battlefield);
    assert!(
        crate::targeting::can_target_object(&game, breaker_id, own_red, alice).is_legal(),
        "controller's colored source should be able to target their own Breaker of Creation"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn breaker_of_creation_cast_trigger_counts_only_your_colorless_permanents() {
    let def = parse_oracle_card_definition("Breaker of Creation");
    let cast_trigger = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => {
                if ability.functional_zones == [Zone::Stack]
                    && triggered.trigger.display() == "When you cast this spell"
                {
                    Some(triggered)
                } else {
                    None
                }
            }
            _ => None,
        })
        .expect("Breaker of Creation should have a cast trigger on the stack");

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source_id = game.create_object_from_definition(&def, alice, Zone::Stack);
    let colorless_artifact =
        CardDefinitionBuilder::new(CardId::from_raw(92_200), "Colorless Artifact")
            .card_types(vec![CardType::Artifact])
            .build();
    let colorless_land = CardDefinitionBuilder::new(CardId::from_raw(92_201), "Colorless Land")
        .card_types(vec![CardType::Land])
        .build();
    let green_creature = CardDefinitionBuilder::new(CardId::from_raw(92_202), "Green Creature")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .card_types(vec![CardType::Creature])
        .build();

    game.create_object_from_definition(&colorless_artifact, alice, Zone::Battlefield);
    game.create_object_from_definition(&colorless_land, alice, Zone::Battlefield);
    game.create_object_from_definition(&green_creature, alice, Zone::Battlefield);
    game.create_object_from_definition(&colorless_artifact, bob, Zone::Battlefield);

    let mut ctx = crate::effects::ExecutionContext::new_default(source_id, alice);
    for effect in &cast_trigger.effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Breaker of Creation cast trigger should resolve");
    }

    assert_eq!(
        game.life_total(alice),
        22,
        "cast trigger should gain life only for Alice's two colorless battlefield permanents"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn breaker_of_creation_annihilator_sacrifices_two_defending_player_permanents() {
    let def = parse_oracle_card_definition("Breaker of Creation");
    let annihilator = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => {
                if ability.functional_zones == [Zone::Battlefield]
                    && triggered.trigger.display() == "Whenever this creature attacks"
                {
                    Some(triggered)
                } else {
                    None
                }
            }
            _ => None,
        })
        .expect("Breaker of Creation should have annihilator");

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let breaker_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let permanent = CardDefinitionBuilder::new(CardId::from_raw(92_300), "Sacrifice Permanent")
        .card_types(vec![CardType::Artifact])
        .build();
    let bob_first = game.create_object_from_definition(&permanent, bob, Zone::Battlefield);
    let bob_second = game.create_object_from_definition(&permanent, bob, Zone::Battlefield);
    let bob_third = game.create_object_from_definition(&permanent, bob, Zone::Battlefield);
    let alice_permanent = game.create_object_from_definition(&permanent, alice, Zone::Battlefield);

    let mut ctx = crate::effects::ExecutionContext::new_default(breaker_id, alice)
        .with_defending_player(bob)
        .with_targets(vec![
            crate::effects::ResolvedTarget::Object(bob_first),
            crate::effects::ResolvedTarget::Object(bob_second),
        ]);
    for effect in &annihilator.effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Breaker of Creation annihilator should resolve");
    }

    let battlefield_after = game.objects_in_zone(Zone::Battlefield);
    assert!(
        !battlefield_after.contains(&bob_first),
        "first chosen defending permanent should leave the battlefield"
    );
    assert!(
        !battlefield_after.contains(&bob_second),
        "second chosen defending permanent should leave the battlefield"
    );
    assert_eq!(
        game.player(bob).expect("bob").graveyard.len(),
        2,
        "annihilator should put exactly two defending permanents into Bob's graveyard"
    );
    assert_eq!(
        game.object(bob_third).expect("bob third").zone,
        Zone::Battlefield
    );
    assert_eq!(
        game.object(alice_permanent).expect("alice permanent").zone,
        Zone::Battlefield,
        "annihilator should affect the defending player, not the attacker"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sporeweb_weaver_damage_trigger_gains_life_and_creates_saproling() {
    let def = parse_oracle_card_definition("Sporeweb Weaver");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let weaver_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let damage_source = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_202), "Damage Source")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        bob,
        Zone::Battlefield,
    );

    let damage_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            damage_source,
            crate::events::DamageTarget::Object(weaver_id),
            2,
            false,
            crate::events::cause::EventCause::effect(),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    for entry in crate::triggers::check_triggers(&game, &damage_event) {
        trigger_queue.add(entry);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("put Sporeweb Weaver trigger on stack");
    assert_eq!(game.stack.len(), 1, "expected one Weaver trigger on stack");

    crate::game_loop::resolve_stack_entry(&mut game).expect("resolve Weaver trigger");

    assert_eq!(
        game.player(alice).expect("alice exists").life,
        21,
        "Sporeweb Weaver trigger should gain 1 life"
    );
    let saprolings: Vec<_> = game
        .battlefield
        .iter()
        .filter_map(|&id| game.object(id))
        .filter(|obj| obj.owner == alice && obj.name == "Saproling")
        .collect();
    assert_eq!(saprolings.len(), 1, "expected one Saproling token");
    let saproling = saprolings[0];
    assert_eq!(saproling.kind, crate::object::ObjectKind::Token);
    assert_eq!(saproling.power(), Some(1));
    assert_eq!(saproling.toughness(), Some(1));
    assert!(saproling.subtypes.contains(&Subtype::Saproling));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sporeweb_weaver_damage_trigger_ignores_other_damaged_creatures() {
    let def = parse_oracle_card_definition("Sporeweb Weaver");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let damage_source = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_203), "Damage Source")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        bob,
        Zone::Battlefield,
    );
    let other_creature = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_204), "Other Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        alice,
        Zone::Battlefield,
    );

    let damage_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            damage_source,
            crate::events::DamageTarget::Object(other_creature),
            2,
            false,
            crate::events::cause::EventCause::effect(),
        ),
        crate::provenance::ProvNodeId::default(),
    );

    assert!(
        crate::triggers::check_triggers(&game, &damage_event).is_empty(),
        "Sporeweb Weaver should not trigger when another creature is dealt damage"
    );
    assert_eq!(game.player(alice).expect("alice exists").life, 20);
    assert!(
        game.battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .all(|obj| obj.name != "Saproling"),
        "no Saproling token should be created without a Weaver trigger"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_xanthic_statue_strict_regression() {
    assert_oracle_card_parses_strict("Xanthic Statue");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_living_conundrum_strict_regression() {
    assert_oracle_card_parses_strict("Living Conundrum");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn living_conundrum_compiled_text_keeps_empty_library_clauses() {
    let def = parse_oracle_card_definition("Living Conundrum");
    let lines = canonical_compiled_lines(&def);

    assert!(
        lines.iter().any(|line| {
            line == "If you would draw a card while your library has no cards in it, skip that draw instead."
        }),
        "Living Conundrum should render the empty-library draw-skip replacement, got {lines:?}"
    );
    assert!(
        lines.iter().any(|line| {
            line.contains("base power and toughness 10/10")
                && line.contains("flying")
                && line.contains("vigilance")
                && line.contains("no cards in your library")
        }),
        "Living Conundrum should render the empty-library characteristic bonus, got {lines:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn living_conundrum_empty_library_replacement_and_bonus_are_active() {
    let def = parse_oracle_card_definition("Living Conundrum");
    let alice = PlayerId::from_index(0);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let source_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    assert_eq!(
        game.player(alice)
            .expect("alice should exist")
            .library
            .len(),
        0
    );
    game.update_replacement_effects();
    assert!(
        game.effect_store
            .replacement_effects
            .effects()
            .iter()
            .any(|replacement| {
                replacement.source == source_id
                    && matches!(
                        replacement.replacement,
                        crate::replacement::ReplacementAction::Skip
                    )
                    && replacement
                        .matcher
                        .as_ref()
                        .is_some_and(|matcher| matcher.display().contains("no cards in it"))
            }),
        "Living Conundrum should register an empty-library draw-skip replacement"
    );

    let mut ctx = crate::effects::ExecutionContext::new_default(source_id, alice);
    let outcome = crate::effects::DrawCardsEffect::you(1)
        .execute(&mut game, &mut ctx)
        .expect("Living Conundrum draw replacement should resolve");
    assert_eq!(outcome.count_or_zero(), 0);
    assert_eq!(
        game.player(alice).expect("alice should exist").hand.len(),
        0
    );

    assert_eq!(game.calculated_power(source_id), Some(10));
    assert_eq!(game.calculated_toughness(source_id), Some(10));
    assert!(game.object_has_static_ability_id(source_id, StaticAbilityId::Flying));
    assert!(game.object_has_static_ability_id(source_id, StaticAbilityId::Vigilance));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn living_conundrum_nonempty_library_does_not_skip_or_gain_bonus() {
    let def = parse_oracle_card_definition("Living Conundrum");
    let alice = PlayerId::from_index(0);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let source_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let library_card = CardDefinitionBuilder::new(CardId::new(), "Library Card")
        .card_types(vec![CardType::Instant])
        .build();
    game.create_object_from_definition(&library_card, alice, Zone::Library);

    assert_ne!(game.calculated_power(source_id), Some(10));
    assert_ne!(game.calculated_toughness(source_id), Some(10));
    assert!(!game.object_has_static_ability_id(source_id, StaticAbilityId::Flying));
    assert!(!game.object_has_static_ability_id(source_id, StaticAbilityId::Vigilance));

    let mut ctx = crate::effects::ExecutionContext::new_default(source_id, alice);
    let outcome = crate::effects::DrawCardsEffect::you(1)
        .execute(&mut game, &mut ctx)
        .expect("nonempty-library draw should proceed normally");
    assert_eq!(outcome.count_or_zero(), 1);
    assert_eq!(
        game.player(alice).expect("alice should exist").hand.len(),
        1
    );
    assert_eq!(
        game.player(alice)
            .expect("alice should exist")
            .library
            .len(),
        0
    );
    assert_eq!(game.calculated_power(source_id), Some(10));
    assert_eq!(game.calculated_toughness(source_id), Some(10));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_sages_of_the_anima_strict_regression() {
    assert_oracle_card_parses_strict("Sages of the Anima");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sages_of_the_anima_compiled_text_keeps_draw_replacement_clause() {
    let def = parse_oracle_card_definition("Sages of the Anima");
    let rendered = canonical_compiled_lines(&def).join("\n");
    let static_ids = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(
        static_ids.contains(&StaticAbilityId::DrawReplacementRevealTopMatchingToHandRestBottom),
        "expected Sages of the Anima draw-replacement static ability, got {static_ids:?}"
    );
    assert!(
        rendered.contains(
            "If you would draw a card, instead reveal the top three cards of your library"
        ),
        "Sages of the Anima compiled text should keep the draw replacement and instead marker, got {rendered}"
    );
    assert!(
        rendered.contains(
            "Put all creature cards revealed this way into your hand and the rest on the bottom of your library in any order"
        ),
        "Sages of the Anima compiled text should keep the creature/rest split, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sages_of_the_anima_draw_replacement_moves_creatures_and_bottoms_rest() {
    let def = parse_oracle_card_definition("Sages of the Anima");
    let alice = PlayerId::from_index(0);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let source_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    for (name, card_types) in [
        ("Library Creature One", vec![CardType::Creature]),
        ("Library Instant", vec![CardType::Instant]),
        ("Library Creature Two", vec![CardType::Creature]),
    ] {
        let card = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(card_types)
            .build();
        game.create_object_from_definition(&card, alice, Zone::Library);
    }

    let mut ctx = crate::effects::ExecutionContext::new_default(source_id, alice);
    crate::effects::DrawCardsEffect::you(1)
        .execute(&mut game, &mut ctx)
        .expect("Sages of the Anima draw replacement should resolve");

    let player = game.player(alice).expect("alice should exist");
    let hand_names = player
        .hand
        .iter()
        .filter_map(|id| game.object(*id).map(|object| object.name.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(
        hand_names.len(),
        2,
        "only revealed creature cards should move to hand, got {hand_names:?}"
    );
    assert!(
        hand_names.contains(&"Library Creature One"),
        "{hand_names:?}"
    );
    assert!(
        hand_names.contains(&"Library Creature Two"),
        "{hand_names:?}"
    );

    let library_names = player
        .library
        .iter()
        .filter_map(|id| game.object(*id).map(|object| object.name.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(
        library_names,
        vec!["Library Instant"],
        "noncreature revealed cards should stay in the library on bottom"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_wave_of_rats_strict_regression() {
    assert_oracle_card_parses_strict("Wave of Rats");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_cloud_ex_soldier_strict_regression() {
    assert_oracle_card_parses_strict("Cloud, Ex-SOLDIER");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_cephalid_vandal_strict_regression() {
    assert_oracle_card_parses_strict("Cephalid Vandal");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cephalid_vandal_compiled_text_keeps_shred_counter_mill_clause() {
    let def = parse_oracle_card_definition("Cephalid Vandal");
    let rendered = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("at the beginning of your upkeep")
            && rendered.contains("put a shred counter on this creature")
            && rendered.contains("mill a card for each shred counter on this creature"),
        "expected Cephalid Vandal upkeep shred-counter mill clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cephalid_vandal_upkeep_trigger_adds_shred_counter_then_mills_one() {
    let def = parse_oracle_card_definition("Cephalid Vandal");
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered.clone()),
            _ => None,
        })
        .expect("Cephalid Vandal should have an upkeep triggered ability");

    let effects = triggered.effects.flattened_default_effects();
    let alice = PlayerId::from_index(0);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let source_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    let library_card = CardDefinitionBuilder::new(CardId::new(), "Library Card")
        .card_types(vec![CardType::Land])
        .build();
    for _ in 0..3 {
        game.create_object_from_definition(&library_card, alice, Zone::Library);
    }
    let library_before = game
        .player(alice)
        .expect("alice should exist")
        .library
        .len();

    let mut ctx = crate::effects::ExecutionContext::new_default(source_id, alice);
    for effect in effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Cephalid Vandal upkeep effects should resolve");
    }

    assert_eq!(
        game.counter_count(source_id, crate::object::CounterType::Named("shred")),
        1,
        "Cephalid Vandal should gain one shred counter during upkeep"
    );
    assert_eq!(
        game.player(alice)
            .expect("alice should exist")
            .library
            .len(),
        library_before - 1,
        "Cephalid Vandal should mill one card after the first shred counter"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cephalid_vandal_upkeep_trigger_mill_count_scales_with_existing_shred_counters() {
    let def = parse_oracle_card_definition("Cephalid Vandal");
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered.clone()),
            _ => None,
        })
        .expect("Cephalid Vandal should have an upkeep triggered ability");

    let effects = triggered.effects.flattened_default_effects();
    let alice = PlayerId::from_index(0);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let source_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    let library_card = CardDefinitionBuilder::new(CardId::new(), "Library Card")
        .card_types(vec![CardType::Land])
        .build();
    for _ in 0..5 {
        game.create_object_from_definition(&library_card, alice, Zone::Library);
    }

    game.add_counters_with_source(
        source_id,
        crate::object::CounterType::Named("shred"),
        1,
        None,
        None,
    );
    let library_before = game
        .player(alice)
        .expect("alice should exist")
        .library
        .len();

    let mut ctx = crate::effects::ExecutionContext::new_default(source_id, alice);
    for effect in effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Cephalid Vandal upkeep effects should resolve");
    }

    assert_eq!(
        game.counter_count(source_id, crate::object::CounterType::Named("shred")),
        2,
        "Cephalid Vandal should add a second shred counter during upkeep"
    );
    assert_eq!(
        game.player(alice)
            .expect("alice should exist")
            .library
            .len(),
        library_before - 2,
        "Cephalid Vandal should mill two cards when it has two shred counters"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cloud_ex_soldier_compiled_text_keeps_power_threshold_treasure_clause() {
    let def = parse_oracle_card_definition("Cloud, Ex-SOLDIER");
    let rendered = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("whenever cloud attacks")
            && rendered.contains("draw a card for each equipped attacking creature you control")
            && rendered.contains("if this has power 7 or greater")
            && rendered.contains("create two treasure tokens"),
        "expected Cloud, Ex-SOLDIER attack trigger and power-threshold treasure clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn wave_of_rats_compiled_text_keeps_combat_damage_condition_clause() {
    let def = parse_oracle_card_definition("Wave of Rats");
    let rendered = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("when this creature dies")
            && rendered.contains("if it dealt combat damage to a player this turn")
            && rendered.contains("return it to the battlefield under its owner's control"),
        "expected Wave of Rats dies condition and return clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn wave_of_rats_dies_trigger_uses_source_combat_damage_condition() {
    let def = parse_oracle_card_definition("Wave of Rats");
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered.clone()),
            _ => None,
        })
        .expect("Wave of Rats should have a dies triggered ability");

    let trigger_debug = format!("{:?}", triggered).to_ascii_lowercase();
    assert!(
        trigger_debug.contains("sourcedealtcombatdamagetoplayerthisturn")
            && trigger_debug.contains("movetozoneeffect")
            && trigger_debug.contains("battlefield"),
        "expected Wave of Rats dies trigger to gate return on source combat damage, got {trigger_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn xanthic_statue_compiled_text_keeps_until_end_of_turn_becomes_clause() {
    let def = parse_oracle_card_definition("Xanthic Statue");
    let rendered = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("until end of turn")
            && rendered.contains("this artifact becomes an 8/8")
            && rendered.contains("golem artifact creature")
            && rendered.contains("trample"),
        "expected Xanthic Statue become-until-end clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn xanthic_statue_activation_sets_base_pt_and_trample_on_source_artifact() {
    use crate::ability::AbilityKind;

    let def = parse_oracle_card_definition("Xanthic Statue");
    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated.clone()),
            _ => None,
        })
        .expect("Xanthic Statue should have an activated ability");

    assert_eq!(
        activated.mana_cost.display(),
        "{5}",
        "expected Xanthic Statue activation cost to remain {{5}}"
    );

    let effects_debug = format!("{:?}", activated.effects).to_ascii_lowercase();
    assert!(
        effects_debug.contains("until: endofturn")
            && effects_debug.contains("creature")
            && effects_debug.contains("artifact")
            && effects_debug.contains("setpowertoughness")
            && effects_debug.contains("power: fixed(8)")
            && effects_debug.contains("toughness: fixed(8)")
            && effects_debug.contains("addsubtypes([golem])")
            && effects_debug.contains("label: \"trample\""),
        "expected Xanthic Statue lowering to include until-end continuous become effect, got {effects_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn vastwood_animist_strict_parse_and_compiled_text_include_dynamic_land_animation() {
    let def = parse_oracle_card_definition("Vastwood Animist");
    let rendered = canonical_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();

    assert!(
        rendered_lower.contains(
            "target land you control becomes an x/x elemental creature until end of turn"
        ) && rendered_lower.contains("where x is the number of allies you control")
            && rendered_lower.contains("it's still a land"),
        "expected Vastwood Animist compiled text to preserve dynamic X/X land animation, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oran_rief_the_vastwood_strict_regression() {
    assert_oracle_card_parses_strict("Oran-Rief, the Vastwood");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_james_wandering_dad_follow_him_strict_regression() {
    assert_oracle_card_parses_strict("James, Wandering Dad // Follow Him");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_raphael_tag_team_tough_strict_regression() {
    assert_oracle_card_parses_strict("Raphael, Tag Team Tough");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_rankle_master_of_pranks_strict_regression() {
    assert_oracle_card_parses_strict("Rankle, Master of Pranks");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_wild_roads_strict_regression() {
    assert_oracle_card_parses_strict("Wild Roads");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_demonic_torment_strict_regression() {
    assert_oracle_card_parses_strict("Demonic Torment");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn demonic_torment_compiled_text_keeps_combat_prevention_clause() {
    let def = parse_oracle_card_definition("Demonic Torment");
    let rendered = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("prevent all combat damage that would be dealt by enchanted creature"),
        "expected Demonic Torment compiled text to keep combat-only prevention clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn demonic_torment_grants_combat_only_damage_prevention_to_enchanted_creature() {
    let def = parse_oracle_card_definition("Demonic Torment");
    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("PreventAllCombatDamageDealtByThisPermanent"),
        "expected Demonic Torment to grant combat-only prevention, got {debug}"
    );
    assert!(
        !debug.contains("PreventAllDamageDealtByThisPermanent"),
        "expected Demonic Torment to avoid all-damage prevention, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_cloudspire_coordinator_strict_regression() {
    assert_oracle_card_parses_strict("Cloudspire Coordinator");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cloudspire_coordinator_compiled_text_keeps_dynamic_pilot_token_clause() {
    let def = parse_oracle_card_definition("Cloudspire Coordinator");
    let rendered = canonical_compiled_lines(&def).join("\n");

    assert!(
        rendered.contains("Create X 1/1 colorless Pilot creature tokens")
            && rendered.contains("where X is the number of Mounts and/or Vehicles that entered the battlefield under your control this turn")
            && rendered.contains(
                "\"This token saddles Mounts and crews Vehicles as though its power were 2 greater.\""
            ),
        "expected Cloudspire Coordinator compiled text to preserve the dynamic Pilot token count and saddle/crew clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cloudspire_coordinator_creates_pilot_tokens_for_your_entered_mounts_and_vehicles() {
    let def = parse_oracle_card_definition("Cloudspire Coordinator");
    let create_effect = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated
                .effects
                .flattened_default_effects()
                .into_iter()
                .find(|effect| effect.downcast_ref::<CreateTokenEffect>().is_some())
                .cloned(),
            _ => None,
        })
        .expect("Cloudspire Coordinator should have an activation that creates Pilot tokens");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    let mount = CardDefinitionBuilder::new(CardId::new(), "Mount Probe")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Mount])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let vehicle = CardDefinitionBuilder::new(CardId::new(), "Vehicle Probe")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Vehicle])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    let bear = CardDefinitionBuilder::new(CardId::new(), "Bear Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();

    let record_entered = |game: &mut crate::game_state::GameState, object_id: ObjectId| {
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::EnterBattlefieldEvent::new(object_id, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        );
        game.record_turn_history_event(&event);
    };

    let alice_mount = game.create_object_from_definition(&mount, alice, Zone::Battlefield);
    record_entered(&mut game, alice_mount);
    let alice_vehicle = game.create_object_from_definition(&vehicle, alice, Zone::Battlefield);
    record_entered(&mut game, alice_vehicle);
    let bob_vehicle = game.create_object_from_definition(&vehicle, bob, Zone::Battlefield);
    record_entered(&mut game, bob_vehicle);
    let alice_bear = game.create_object_from_definition(&bear, alice, Zone::Battlefield);
    record_entered(&mut game, alice_bear);
    game.create_object_from_definition(&mount, alice, Zone::Battlefield);

    let pilot_tokens = |game: &crate::game_state::GameState| {
        game.battlefield
            .iter()
            .copied()
            .filter(|id| {
                game.object(*id).is_some_and(|obj| {
                    obj.name == "Pilot"
                        && obj.zone == Zone::Battlefield
                        && matches!(obj.kind, crate::object::ObjectKind::Token)
                })
            })
            .collect::<Vec<_>>()
    };

    let before = pilot_tokens(&game).len();
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    crate::effects::execute_effect(&mut game, &create_effect, &mut ctx)
        .expect("Cloudspire Coordinator token creation should resolve");
    let pilots = pilot_tokens(&game);
    assert_eq!(
        pilots.len() - before,
        2,
        "Cloudspire Coordinator should count only Alice's Mount and Vehicle that entered this turn, not Bob's Vehicle, a Bear, or an unrecorded Mount"
    );

    assert!(
        pilots.iter().all(|id| game.object(*id).is_some()),
        "sanity check: created Pilot tokens should remain on the battlefield"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cloudspire_coordinator_pilot_token_power_bonus_applies_to_saddle_and_crew_costs() {
    let def = parse_oracle_card_definition("Cloudspire Coordinator");
    let pilot = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated
                .effects
                .flattened_default_effects()
                .into_iter()
                .find_map(|effect| effect.downcast_ref::<CreateTokenEffect>())
                .map(|create| create.token.clone()),
            _ => None,
        })
        .expect("Cloudspire Coordinator should create Pilot tokens");

    let marker = pilot
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::KeywordMarker =>
            {
                Some(static_ability.display())
            }
            _ => None,
        })
        .expect("Cloudspire Coordinator Pilot token should keep its saddle/crew marker");
    assert_eq!(
        marker,
        "this token saddles mounts and crews vehicles as though its power were 2 greater."
    );

    let alice = PlayerId::from_index(0);
    let vehicle = CardDefinitionBuilder::new(CardId::new(), "Vehicle Probe")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Vehicle])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    let mount = CardDefinitionBuilder::new(CardId::new(), "Mount Probe")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Mount])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let vanilla = CardDefinitionBuilder::new(CardId::new(), "Vanilla 1/1")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();

    let crew_cost = crate::effects::CrewCostEffect { required_power: 3 };
    let mut crew_game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    crew_game.create_object_from_definition(&pilot, alice, Zone::Battlefield);
    let crew_vehicle = crew_game.create_object_from_definition(&vehicle, alice, Zone::Battlefield);
    crate::effects::CostExecutableEffect::can_execute_as_cost(
        &crew_cost,
        &crew_game,
        crew_vehicle,
        alice,
    )
    .expect("Cloudspire Coordinator Pilot token should crew as though its power were 2 greater");

    let saddle_cost = crate::effects::SaddleCostEffect::new(3);
    let mut saddle_game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    saddle_game.create_object_from_definition(&pilot, alice, Zone::Battlefield);
    let saddle_mount = saddle_game.create_object_from_definition(&mount, alice, Zone::Battlefield);
    crate::effects::CostExecutableEffect::can_execute_as_cost(
        &saddle_cost,
        &saddle_game,
        saddle_mount,
        alice,
    )
    .expect("Cloudspire Coordinator Pilot token should saddle as though its power were 2 greater");

    let mut baseline_crew = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    baseline_crew.create_object_from_definition(&vanilla, alice, Zone::Battlefield);
    let baseline_vehicle =
        baseline_crew.create_object_from_definition(&vehicle, alice, Zone::Battlefield);
    assert!(
        crate::effects::CostExecutableEffect::can_execute_as_cost(
            &crew_cost,
            &baseline_crew,
            baseline_vehicle,
            alice,
        )
        .is_err(),
        "baseline 1/1 without Cloudspire Coordinator Pilot marker should not satisfy crew 3"
    );

    let mut baseline_saddle = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    baseline_saddle.create_object_from_definition(&vanilla, alice, Zone::Battlefield);
    let baseline_mount =
        baseline_saddle.create_object_from_definition(&mount, alice, Zone::Battlefield);
    assert!(
        crate::effects::CostExecutableEffect::can_execute_as_cost(
            &saddle_cost,
            &baseline_saddle,
            baseline_mount,
            alice,
        )
        .is_err(),
        "baseline 1/1 without Cloudspire Coordinator Pilot marker should not satisfy saddle 3"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cloudspire_coordinator_creates_no_tokens_without_entered_mounts_or_vehicles() {
    let def = parse_oracle_card_definition("Cloudspire Coordinator");
    let create_effect = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated
                .effects
                .flattened_default_effects()
                .into_iter()
                .find(|effect| effect.downcast_ref::<CreateTokenEffect>().is_some())
                .cloned(),
            _ => None,
        })
        .expect("Cloudspire Coordinator should have an activation that creates Pilot tokens");

    let alice = PlayerId::from_index(0);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    crate::effects::execute_effect(&mut game, &create_effect, &mut ctx)
        .expect("Cloudspire Coordinator zero-token branch should resolve");

    let pilot_tokens = game
        .battlefield
        .iter()
        .filter(|id| {
            game.object(**id).is_some_and(|obj| {
                obj.name == "Pilot" && matches!(obj.kind, crate::object::ObjectKind::Token)
            })
        })
        .count();
    assert_eq!(
        pilot_tokens, 0,
        "Cloudspire Coordinator should create no Pilot tokens when no Mounts or Vehicles entered under your control this turn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn wild_roads_compiled_text_keeps_pilot_saddle_and_crew_clause() {
    let def = parse_oracle_card_definition("Wild Roads");
    let rendered = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("saddles mounts and crews vehicles as though its power were 2 greater"),
        "expected Wild Roads compiled text to preserve Pilot token saddle/crew clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn wild_roads_pilot_token_power_bonus_applies_to_saddle_and_crew_costs() {
    let def = parse_oracle_card_definition("Wild Roads");
    let pilot = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated
                .effects
                .segments
                .iter()
                .flat_map(|segment| segment.default_effects.iter())
                .find_map(|effect| effect.downcast_ref::<CreateTokenEffect>())
                .map(|create| create.token.clone()),
            _ => None,
        })
        .expect("Wild Roads should have an activated ability that creates a Pilot token");

    let alice = PlayerId::from_index(0);

    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let pilot_id = game.create_object_from_definition(&pilot, alice, Zone::Battlefield);
    let vehicle = CardDefinitionBuilder::new(CardId::new(), "Vehicle Probe")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Vehicle])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    let vehicle_id = game.create_object_from_definition(&vehicle, alice, Zone::Battlefield);
    let mount = CardDefinitionBuilder::new(CardId::new(), "Mount Probe")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Mount])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();

    let crew_cost = crate::effects::CrewCostEffect { required_power: 3 };
    crate::effects::CostExecutableEffect::can_execute_as_cost(&crew_cost, &game, vehicle_id, alice)
        .expect("Wild Roads Pilot token should crew as though its power were 2 greater");

    let mount_id = game.create_object_from_definition(&mount, alice, Zone::Battlefield);
    let saddle_cost = crate::effects::SaddleCostEffect::new(3);
    crate::effects::CostExecutableEffect::can_execute_as_cost(&saddle_cost, &game, mount_id, alice)
        .expect("Wild Roads Pilot token should saddle as though its power were 2 greater");

    let vanilla = CardDefinitionBuilder::new(CardId::new(), "Vanilla 1/1")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let mut baseline_crew = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    baseline_crew.create_object_from_definition(&vanilla, alice, Zone::Battlefield);
    let baseline_vehicle =
        baseline_crew.create_object_from_definition(&vehicle, alice, Zone::Battlefield);

    assert!(
        crate::effects::CostExecutableEffect::can_execute_as_cost(
            &crew_cost,
            &baseline_crew,
            baseline_vehicle,
            alice,
        )
        .is_err(),
        "baseline 1/1 without Wild Roads Pilot marker should not satisfy crew 3"
    );

    let mut baseline_saddle = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    baseline_saddle.create_object_from_definition(&vanilla, alice, Zone::Battlefield);
    let baseline_mount =
        baseline_saddle.create_object_from_definition(&mount, alice, Zone::Battlefield);

    assert!(
        crate::effects::CostExecutableEffect::can_execute_as_cost(
            &saddle_cost,
            &baseline_saddle,
            baseline_mount,
            alice,
        )
        .is_err(),
        "baseline 1/1 without Wild Roads Pilot marker should not satisfy saddle 3"
    );

    assert!(
        game.object(pilot_id).is_some(),
        "sanity check: Wild Roads Pilot token should exist on battlefield"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_deathless_pilot_strict_regression() {
    assert_oracle_card_parses_strict("Deathless Pilot");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn deathless_pilot_compiled_text_keeps_saddle_crew_and_graveyard_return() {
    let def = parse_oracle_card_definition("Deathless Pilot");
    let rendered = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("saddles mounts and crews vehicles as though its power were 2 greater"),
        "expected Deathless Pilot compiled text to preserve saddle/crew power clause, got {rendered}"
    );
    assert!(
        rendered.contains("{3}{b}: return this card from your graveyard to your hand"),
        "expected Deathless Pilot compiled text to preserve graveyard self-return activation, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn deathless_pilot_power_bonus_applies_to_saddle_and_crew_costs() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Deathless Pilot")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie, Subtype::Pilot])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            oracle_text_by_name()
                .get("Deathless Pilot")
                .expect("Deathless Pilot oracle text should be available")
                .clone(),
        )
        .expect("Deathless Pilot should parse for runtime cost test");
    let alice = PlayerId::from_index(0);

    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    game.create_object_from_definition(&def, alice, Zone::Battlefield);

    let vehicle = CardDefinitionBuilder::new(CardId::new(), "Vehicle Probe")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Vehicle])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    let vehicle_id = game.create_object_from_definition(&vehicle, alice, Zone::Battlefield);
    let crew_cost = crate::effects::CrewCostEffect { required_power: 4 };
    crate::effects::CostExecutableEffect::can_execute_as_cost(&crew_cost, &game, vehicle_id, alice)
        .expect("Deathless Pilot should crew 4 as though its power were 2 greater");

    let mount = CardDefinitionBuilder::new(CardId::new(), "Mount Probe")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Mount])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    let mount_id = game.create_object_from_definition(&mount, alice, Zone::Battlefield);
    let saddle_cost = crate::effects::SaddleCostEffect::new(4);
    crate::effects::CostExecutableEffect::can_execute_as_cost(&saddle_cost, &game, mount_id, alice)
        .expect("Deathless Pilot should saddle 4 as though its power were 2 greater");

    let vanilla = CardDefinitionBuilder::new(CardId::new(), "Vanilla 2/2")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let mut baseline_crew = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    baseline_crew.create_object_from_definition(&vanilla, alice, Zone::Battlefield);
    let baseline_vehicle =
        baseline_crew.create_object_from_definition(&vehicle, alice, Zone::Battlefield);
    assert!(
        crate::effects::CostExecutableEffect::can_execute_as_cost(
            &crew_cost,
            &baseline_crew,
            baseline_vehicle,
            alice,
        )
        .is_err(),
        "baseline 2/2 without Deathless Pilot marker should not satisfy crew 4"
    );

    let mut baseline_saddle = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    baseline_saddle.create_object_from_definition(&vanilla, alice, Zone::Battlefield);
    let baseline_mount =
        baseline_saddle.create_object_from_definition(&mount, alice, Zone::Battlefield);
    assert!(
        crate::effects::CostExecutableEffect::can_execute_as_cost(
            &saddle_cost,
            &baseline_saddle,
            baseline_mount,
            alice,
        )
        .is_err(),
        "baseline 2/2 without Deathless Pilot marker should not satisfy saddle 4"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn deathless_pilot_graveyard_activation_returns_only_from_graveyard() {
    let def = parse_oracle_card_definition("Deathless Pilot");
    let (ability, activated) = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some((ability, activated)),
            _ => None,
        })
        .expect("Deathless Pilot should have a graveyard self-return activated ability");
    assert_eq!(ability.functional_zones, vec![Zone::Graveyard]);
    assert_eq!(
        activated.mana_cost.display(),
        "{3}{B}",
        "Deathless Pilot self-return should keep its activation cost"
    );

    let alice = PlayerId::from_index(0);
    let mut graveyard_game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let graveyard_pilot =
        graveyard_game.create_object_from_definition(&def, alice, Zone::Graveyard);
    let mut graveyard_ctx = crate::effects::ExecutionContext::new_default(graveyard_pilot, alice);
    for effect in activated.effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut graveyard_game, effect, &mut graveyard_ctx)
            .expect("Deathless Pilot graveyard activation should resolve");
    }
    assert_eq!(
        graveyard_game.player(alice).expect("Alice").graveyard.len(),
        0,
        "Deathless Pilot should leave the graveyard when its activated ability resolves"
    );
    assert_eq!(
        graveyard_game.player(alice).expect("Alice").hand.len(),
        1,
        "Deathless Pilot should return to its owner's hand from the graveyard"
    );

    let mut battlefield_game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let battlefield_pilot =
        battlefield_game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let mut battlefield_ctx =
        crate::effects::ExecutionContext::new_default(battlefield_pilot, alice);
    for effect in activated.effects.flattened_default_effects() {
        let _ = crate::effects::execute_effect(&mut battlefield_game, effect, &mut battlefield_ctx);
    }
    assert!(
        battlefield_game
            .objects_in_zone(Zone::Battlefield)
            .contains(&battlefield_pilot),
        "Deathless Pilot should not be returned by its graveyard ability while on the battlefield"
    );
    assert!(
        battlefield_game
            .player(alice)
            .expect("Alice")
            .hand
            .is_empty(),
        "Deathless Pilot graveyard ability should not move a battlefield object to hand"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_interface_ace_strict_regression() {
    assert_oracle_card_parses_strict("Interface Ace");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn interface_ace_compiled_text_keeps_saddle_and_crew_toughness_clause() {
    let def = parse_oracle_card_definition("Interface Ace");
    let rendered = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains(
            "saddles mounts and crews vehicles using its toughness rather than its power"
        ),
        "expected Interface Ace compiled text to preserve toughness-based saddle/crew clause, got {rendered}"
    );
    assert!(
        rendered.contains("whenever this creature becomes tapped during your turn")
            && rendered.contains("untap it")
            && rendered.contains("this ability triggers only once each turn"),
        "expected Interface Ace compiled text to preserve tap trigger and once-per-turn cap, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn interface_ace_uses_toughness_for_saddle_and_crew_costs() {
    let alice = PlayerId::from_index(0);
    let def = CardDefinitionBuilder::new(CardId::new(), "Interface Ace")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Robot, Subtype::Pilot])
        .power_toughness(PowerToughness::fixed(0, 4))
        .parse_text(
            "This creature saddles Mounts and crews Vehicles using its toughness rather than its power.\n\
             Whenever this creature becomes tapped during your turn, untap it. This ability triggers only once each turn.",
        )
        .expect("Interface Ace should parse for runtime cost test");

    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    game.create_object_from_definition(&def, alice, Zone::Battlefield);

    let vehicle = CardDefinitionBuilder::new(CardId::new(), "Vehicle Probe")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Vehicle])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    let vehicle_id = game.create_object_from_definition(&vehicle, alice, Zone::Battlefield);
    let crew_cost = crate::effects::CrewCostEffect { required_power: 4 };
    crate::effects::CostExecutableEffect::can_execute_as_cost(&crew_cost, &game, vehicle_id, alice)
        .expect("Interface Ace should crew 4 using its toughness rather than 0 power");

    let mount = CardDefinitionBuilder::new(CardId::new(), "Mount Probe")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Mount])
        .power_toughness(PowerToughness::fixed(4, 4))
        .build();
    let mount_id = game.create_object_from_definition(&mount, alice, Zone::Battlefield);
    let saddle_cost = crate::effects::SaddleCostEffect::new(4);
    crate::effects::CostExecutableEffect::can_execute_as_cost(&saddle_cost, &game, mount_id, alice)
        .expect("Interface Ace should saddle 4 using its toughness rather than 0 power");

    let vanilla = CardDefinitionBuilder::new(CardId::new(), "Vanilla 0/4")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(0, 4))
        .build();
    let mut baseline_crew = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    baseline_crew.create_object_from_definition(&vanilla, alice, Zone::Battlefield);
    let baseline_vehicle =
        baseline_crew.create_object_from_definition(&vehicle, alice, Zone::Battlefield);
    assert!(
        crate::effects::CostExecutableEffect::can_execute_as_cost(
            &crew_cost,
            &baseline_crew,
            baseline_vehicle,
            alice,
        )
        .is_err(),
        "baseline 0/4 without Interface Ace marker should not satisfy crew 4"
    );

    let mut baseline_saddle = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    baseline_saddle.create_object_from_definition(&vanilla, alice, Zone::Battlefield);
    let baseline_mount =
        baseline_saddle.create_object_from_definition(&mount, alice, Zone::Battlefield);
    assert!(
        crate::effects::CostExecutableEffect::can_execute_as_cost(
            &saddle_cost,
            &baseline_saddle,
            baseline_mount,
            alice,
        )
        .is_err(),
        "baseline 0/4 without Interface Ace marker should not satisfy saddle 4"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_brambleback_brute_strict_regression() {
    assert_oracle_card_parses_strict("Brambleback Brute");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn james_wandering_dad_follow_him_compiled_text_keeps_spend_this_mana_only_clause() {
    let def = parse_oracle_card_definition("James, Wandering Dad // Follow Him");
    let rendered = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("spend this mana only to activate abilities"),
        "expected James, Wandering Dad // Follow Him compiled text to preserve spend restriction, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn raphael_tag_team_tough_compiled_text_keeps_additional_combat_phase_clause() {
    let def = parse_oracle_card_definition("Raphael, Tag Team Tough");
    let rendered = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("there is an additional combat phase"),
        "expected Raphael, Tag Team Tough compiled text to preserve additional combat clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn rankle_master_of_pranks_compiled_text_keeps_choose_any_number_modal_header() {
    let def = parse_oracle_card_definition("Rankle, Master of Pranks");
    let rendered = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("choose any number"),
        "expected Rankle, Master of Pranks compiled text to preserve choose-any-number modal header, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn brambleback_brute_compiled_text_keeps_unspecified_remove_counter_activation_cost() {
    let def = parse_oracle_card_definition("Brambleback Brute");
    let rendered = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("remove a counter from this creature"),
        "expected Brambleback Brute compiled text to keep unspecified remove-counter cost, got {rendered}"
    );
    assert!(
        rendered.contains("target creature can't block this turn"),
        "expected Brambleback Brute compiled text to keep can't-block effect, got {rendered}"
    );
    assert!(
        rendered.contains("activate only as a sorcery"),
        "expected Brambleback Brute compiled text to keep sorcery-speed activation rider, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn brambleback_brute_activation_cost_and_effect_runtime_regression() {
    let def = parse_oracle_card_definition("Brambleback Brute");
    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Brambleback Brute should have one activated ability");

    assert_eq!(
        activated.timing,
        crate::ability::ActivationTiming::SorcerySpeed,
        "Brambleback Brute activation must stay sorcery speed"
    );

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);

    let brute_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let target_id = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Blocking Target")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        bob,
        Zone::Battlefield,
    );

    assert!(
        game.add_counters(brute_id, crate::object::CounterType::MinusOneMinusOne, 2)
            .is_some(),
        "Brambleback Brute should be on battlefield"
    );

    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 4);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Red, 2);

    let mut dm = crate::decision::SelectFirstDecisionMaker;
    crate::special_actions::pay_total_cost_with_choice(
        &mut game,
        alice,
        brute_id,
        &activated.mana_cost,
        crate::costs::PaymentReason::ActivateAbility,
        &mut dm,
    )
    .expect("first Brambleback Brute activation cost should be payable");

    let counters_after_first = game
        .object(brute_id)
        .expect("Brambleback Brute should still be on battlefield")
        .counters
        .get(&crate::object::CounterType::MinusOneMinusOne)
        .copied()
        .unwrap_or(0);
    assert_eq!(
        counters_after_first, 1,
        "first activation should remove exactly one counter from Brambleback Brute"
    );

    let mut resolve_ctx = crate::effects::ExecutionContext::new(brute_id, alice, &mut dm)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target_id)]);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut resolve_ctx,
        alice,
        brute_id,
        &activated.effects,
        None,
        &[],
    )
    .expect("Brambleback Brute ability effect should resolve");
    assert!(
        !game.can_block(target_id),
        "target creature should be unable to block this turn after Brambleback Brute resolves"
    );

    crate::special_actions::pay_total_cost_with_choice(
        &mut game,
        alice,
        brute_id,
        &activated.mana_cost,
        crate::costs::PaymentReason::ActivateAbility,
        &mut dm,
    )
    .expect("second Brambleback Brute activation cost should be payable");

    let counters_after_second = game
        .object(brute_id)
        .expect("Brambleback Brute should still be on battlefield")
        .counters
        .get(&crate::object::CounterType::MinusOneMinusOne)
        .copied()
        .unwrap_or(0);
    assert_eq!(
        counters_after_second, 0,
        "second activation should remove the last counter from Brambleback Brute"
    );

    let pay_third = crate::special_actions::pay_total_cost_with_choice(
        &mut game,
        alice,
        brute_id,
        &activated.mana_cost,
        crate::costs::PaymentReason::ActivateAbility,
        &mut dm,
    );
    assert!(
        pay_third.is_err(),
        "activation should fail once Brambleback Brute has no counters left"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn rankle_master_of_pranks_models_zero_to_all_modal_selection_bounds() {
    let def = parse_oracle_card_definition("Rankle, Master of Pranks");
    let modal = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .effects
                .iter()
                .find_map(|effect| effect.downcast_ref::<ChooseModeEffect>()),
            _ => None,
        })
        .expect("Rankle, Master of Pranks should include a triggered modal choice effect");

    assert_eq!(modal.min_choose_count, Value::Fixed(0));
    assert_eq!(modal.modes.len(), 3);
    assert_eq!(modal.choose_count, Value::Fixed(3));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn james_wandering_dad_follow_him_models_activate_only_mana_usage_restriction() {
    let def = parse_oracle_card_definition("James, Wandering Dad // Follow Him");
    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) if activated.mana_output.is_some() => Some(activated),
            _ => None,
        })
        .expect("James, Wandering Dad should have a mana ability");

    assert!(
        activated
            .mana_usage_restrictions
            .contains(&crate::ability::ManaUsageRestriction::ActivateAbility),
        "expected James, Wandering Dad mana ability to keep activate-abilities usage restriction"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn jasmine_dragon_tea_shop_strict_parser_compiled_text_and_model_regression() {
    assert_oracle_card_parses_strict("Jasmine Dragon Tea Shop");
    let def = parse_oracle_card_definition("Jasmine Dragon Tea Shop");
    let rendered = canonical_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();

    assert!(
        rendered_lower.contains(
            "spend this mana only to cast an ally spell or activate an ability of an ally source",
        ),
        "expected Jasmine Dragon Tea Shop compiled text to preserve cast-or-activate spend restriction, got {rendered}"
    );

    let mana_activated = def
        .abilities
        .iter()
        .find_map(|ability| {
            let AbilityKind::Activated(activated) = &ability.kind else {
                return None;
            };
            activated
                .mana_usage_restrictions
                .iter()
                .any(|restriction| {
                    matches!(
                        restriction,
                        crate::ability::ManaUsageRestriction::CastSpellOrActivateAbilitySourceMatching {
                            ..
                        }
                    )
                })
                .then_some(activated)
        })
        .expect("Jasmine Dragon Tea Shop should have restricted Ally mana ability");

    let has_ally_cast_or_ability_restriction =
        mana_activated
            .mana_usage_restrictions
            .iter()
            .any(|restriction| {
                matches!(
                    restriction,
                    crate::ability::ManaUsageRestriction::CastSpellOrActivateAbilitySourceMatching {
                        spell_filter,
                        ability_source_filter,
                    } if spell_filter.subtypes == vec![Subtype::Ally]
                        && ability_source_filter.subtypes == vec![Subtype::Ally]
                )
            });
    assert!(
        has_ally_cast_or_ability_restriction,
        "Jasmine Dragon Tea Shop mana should be restricted to Ally spells or Ally-source abilities"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn creeping_peeper_strict_parser_compiled_text_and_model_regression() {
    assert_oracle_card_parses_strict("Creeping Peeper");
    let def = parse_oracle_card_definition("Creeping Peeper");
    let rendered = canonical_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();

    assert!(
        rendered_lower.contains(
            "spend this mana only to cast an enchantment spell, unlock a door, or turn a permanent face up",
        ),
        "expected Creeping Peeper compiled text to preserve the full restricted-mana clause, got {rendered}"
    );

    let mana_activated = def
        .abilities
        .iter()
        .find_map(|ability| {
            let AbilityKind::Activated(activated) = &ability.kind else {
                return None;
            };
            activated
                .mana_usage_restrictions
                .iter()
                .any(|restriction| {
                    matches!(
                        restriction,
                        crate::ability::ManaUsageRestriction::CastSpellOrUnlockDoorOrTurnFaceUp {
                            ..
                        }
                    )
                })
                .then_some(activated)
        })
        .expect("Creeping Peeper should have a restricted mana ability");

    let has_enchantment_unlock_turn_up_restriction = mana_activated
        .mana_usage_restrictions
        .iter()
        .any(|restriction| {
            matches!(
                restriction,
                crate::ability::ManaUsageRestriction::CastSpellOrUnlockDoorOrTurnFaceUp {
                    spell_filter,
                } if spell_filter.card_types == vec![CardType::Enchantment]
            )
        });
    assert!(
        has_enchantment_unlock_turn_up_restriction,
        "Creeping Peeper mana should be restricted to enchantment spells, unlocking doors, or turning permanents face up"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn creeping_peeper_restricted_mana_runtime_branches() {
    let def = parse_oracle_card_definition("Creeping Peeper");
    let restriction =
        def.abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => activated
                    .mana_usage_restrictions
                    .iter()
                    .find(|restriction| {
                        matches!(
                        restriction,
                        crate::ability::ManaUsageRestriction::CastSpellOrUnlockDoorOrTurnFaceUp {
                            ..
                        }
                    )
                    })
                    .cloned(),
                _ => None,
            })
            .expect("Creeping Peeper should carry its special mana usage restriction");

    let alice = PlayerId::from_index(0);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let peeper_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.player_mut(alice)
        .expect("alice exists")
        .add_restricted_mana(crate::ability::RestrictedManaUnit {
            symbol: ManaSymbol::Blue,
            source: peeper_id,
            source_chosen_creature_type: None,
            restrictions: vec![restriction.clone()],
        });

    let blue_cost = ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]);
    let enchantment_spell = CardDefinitionBuilder::new(CardId::new(), "Enchantment Spell")
        .card_types(vec![CardType::Enchantment])
        .build();
    let enchantment_spell_id =
        game.create_object_from_definition(&enchantment_spell, alice, Zone::Stack);
    assert!(
        game.can_pay_mana_cost_with_reason(
            alice,
            Some(enchantment_spell_id),
            &blue_cost,
            0,
            crate::costs::PaymentReason::CastSpell,
        ),
        "Creeping Peeper mana should pay for enchantment spells"
    );

    let instant_spell = CardDefinitionBuilder::new(CardId::new(), "Instant Spell")
        .card_types(vec![CardType::Instant])
        .build();
    let instant_spell_id = game.create_object_from_definition(&instant_spell, alice, Zone::Stack);
    assert!(
        !game.can_pay_mana_cost_with_reason(
            alice,
            Some(instant_spell_id),
            &blue_cost,
            0,
            crate::costs::PaymentReason::CastSpell,
        ),
        "Creeping Peeper mana should not pay for non-enchantment spells"
    );

    let ability_source = CardDefinitionBuilder::new(CardId::new(), "Ordinary Ability Source")
        .card_types(vec![CardType::Artifact])
        .parse_text("{U}: You gain 1 life.")
        .expect("ordinary ability source should parse");
    let ability_source_id =
        game.create_object_from_definition(&ability_source, alice, Zone::Battlefield);
    assert!(
        !game.can_pay_mana_cost_with_reason(
            alice,
            Some(ability_source_id),
            &blue_cost,
            0,
            crate::costs::PaymentReason::ActivateAbility,
        ),
        "Creeping Peeper mana should not pay for unrelated activated abilities"
    );

    let non_lockable_room = CardDefinitionBuilder::new(CardId::new(), "Non-Lockable Room Probe")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Room])
        .build();
    let non_lockable_room_id =
        game.create_object_from_definition(&non_lockable_room, alice, Zone::Battlefield);
    assert!(
        !game.can_pay_mana_cost_with_reason(
            alice,
            Some(non_lockable_room_id),
            &blue_cost,
            0,
            crate::costs::PaymentReason::UnlockDoor,
        ),
        "Creeping Peeper mana should not pay unlock-door costs for a Room with no locked door"
    );

    let room_front_id = CardId::from_raw(571_600_001);
    let room_back_id = CardId::from_raw(571_600_002);
    let room = CardDefinitionBuilder::new(room_front_id, "Locked Door Probe")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Room])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
        .other_face(room_back_id)
        .other_face_name("Other Locked Door Probe")
        .linked_face_layout(crate::card::LinkedFaceLayout::Split)
        .build();
    let other_room_door = CardDefinitionBuilder::new(room_back_id, "Other Locked Door Probe")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Room])
        .mana_cost(blue_cost.clone())
        .other_face(room_front_id)
        .other_face_name("Locked Door Probe")
        .linked_face_layout(crate::card::LinkedFaceLayout::Split)
        .build();
    game.register_linked_face_definition(&other_room_door);
    let room_id = game.create_object_from_definition(&room, alice, Zone::Battlefield);
    assert!(
        game.can_pay_mana_cost_with_reason(
            alice,
            Some(room_id),
            &blue_cost,
            0,
            crate::costs::PaymentReason::UnlockDoor,
        ),
        "Creeping Peeper mana should pay to unlock a genuinely locked Room door"
    );
    assert!(
        !game.can_pay_mana_cost_with_reason(
            alice,
            Some(room_id),
            &blue_cost,
            0,
            crate::costs::PaymentReason::ActivateAbility,
        ),
        "Creeping Peeper mana should not pay ordinary activated costs, even from a Room source"
    );

    let ordinary_room = CardDefinitionBuilder::new(CardId::new(), "Ordinary Room Ability Probe")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Room])
        .parse_text("{U}: You gain 1 life.")
        .expect("ordinary Room activated ability should parse");
    let ordinary_room_id =
        game.create_object_from_definition(&ordinary_room, alice, Zone::Battlefield);
    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    let ordinary_room_ability_index = game
        .object(ordinary_room_id)
        .expect("ordinary Room should exist")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("ordinary Room should have an activated ability");
    let ordinary_room_action = crate::decision::compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, ability_index }
                    if *source == ordinary_room_id && *ability_index == ordinary_room_ability_index
            )
        })
        .expect("ordinary Room activation should enter the real activation path");
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    let mut state = crate::game_loop::PriorityLoopState::new(game.players_in_game());
    let room_activation = crate::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &crate::PriorityResponse::PriorityAction(ordinary_room_action),
        &mut dm,
    );
    let room_activation = match room_activation {
        Ok(crate::decision::GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(ctx),
        )) => {
            let finish_index = ctx
                .options
                .iter()
                .find(|option| option.description == "Finish activating mana abilities")
                .map(|option| option.index)
                .expect("activation payment should expose the mana-ability-window finish action");
            crate::game_loop::apply_priority_response_with_dm(
                &mut game,
                &mut trigger_queue,
                &mut state,
                &crate::PriorityResponse::ManaPayment(finish_index),
                &mut dm,
            )
        }
        other => other,
    };
    assert!(
        room_activation.is_err(),
        "Creeping Peeper mana must not be offered for ordinary activated abilities on Room permanents: {room_activation:?}"
    );
    assert!(
        game.can_pay_mana_cost_with_reason(
            alice,
            Some(room_id),
            &blue_cost,
            0,
            crate::costs::PaymentReason::UnlockDoor,
        ),
        "failed Room activation should leave Creeping Peeper mana available for a real unlock-door payment"
    );

    let unlock_room_action = crate::decision::compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                crate::decision::LegalAction::SpecialAction(
                    crate::special_actions::SpecialAction::UnlockRoomDoor { room_id: action_room }
                ) if *action_room == room_id
            )
        })
        .expect("locked Room should expose a real unlock-door special action");
    crate::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &crate::PriorityResponse::PriorityAction(unlock_room_action),
        &mut dm,
    )
    .expect("Creeping Peeper mana should be spendable through the real unlock-door action path");
    assert!(
        !game.can_pay_mana_cost_with_reason(
            alice,
            Some(room_id),
            &blue_cost,
            0,
            crate::costs::PaymentReason::UnlockDoor,
        ),
        "fully unlocked Rooms should no longer accept unlock-door restricted mana"
    );

    game.player_mut(alice)
        .expect("alice exists")
        .add_restricted_mana(crate::ability::RestrictedManaUnit {
            symbol: ManaSymbol::Blue,
            source: peeper_id,
            source_chosen_creature_type: None,
            restrictions: vec![restriction],
        });

    let face_up_probe = CardDefinitionBuilder::new(CardId::new(), "Face-Up Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let face_up_probe_id =
        game.create_object_from_definition(&face_up_probe, alice, Zone::Battlefield);
    game.object_mut(face_up_probe_id)
        .expect("face-up probe should exist")
        .abilities_mut()
        .push(Ability::static_ability(
            crate::static_abilities::StaticAbility::morph(crate::cost::TotalCost::mana(
                blue_cost.clone(),
            )),
        ));
    game.set_face_down(face_up_probe_id);
    assert_eq!(
        crate::special_actions::turn_face_up_cost_display(
            &game,
            face_up_probe_id,
            crate::special_actions::TurnFaceUpMethod::TurnFaceUpAbility,
        )
        .as_deref(),
        Some("{U}")
    );
    assert!(
        game.can_pay_mana_cost_with_reason(
            alice,
            Some(face_up_probe_id),
            &blue_cost,
            0,
            crate::costs::PaymentReason::TurnFaceUp,
        ),
        "Creeping Peeper mana should be eligible for the face-down permanent's turn-face-up cost"
    );
    crate::special_actions::perform(
        crate::special_actions::SpecialAction::TurnFaceUp {
            permanent_id: face_up_probe_id,
            method: crate::special_actions::TurnFaceUpMethod::TurnFaceUpAbility,
        },
        &mut game,
        alice,
        &mut dm,
    )
    .expect("Creeping Peeper mana should pay to turn a permanent face up");
    assert!(
        !game.is_face_down(face_up_probe_id),
        "turn-face-up special action should leave the permanent face up"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sphinxs_tutelage_preserves_counted_shared_color_mill_gate() {
    let definition = parse_oracle_card_definition("Sphinx's Tutelage");
    let lines = canonical_compiled_lines(&definition);
    let expected = "Whenever you draw a card, target opponent mills two cards. If two nonland cards that share a color were milled this way, repeat this process.";
    assert!(
        lines.iter().any(|line| line == expected),
        "expected the exact counted shared-color mill gate, got {lines:#?}"
    );

    let debug = format!("{:#?}", definition);
    assert!(
        debug.contains("required_count: Some(\n")
            && debug.contains("shared_characteristic: Some(\n")
            && debug.contains("Color"),
        "expected typed count and shared-color semantics, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cemetery_prowler_preserves_shared_card_type_cost_reduction() {
    let definition = parse_oracle_card_definition("Cemetery Prowler");
    let lines = canonical_compiled_lines(&definition);
    assert_eq!(
        lines,
        vec![
            "Vigilance",
            "Whenever this creature enters or attacks, exile a card from a graveyard.",
            "Spells you cast cost {1} less to cast for each card type they share with cards exiled with this creature.",
        ]
    );

    let debug = format!("{:#?}", definition);
    assert!(
        debug.contains("characteristic_intersection: Some(")
            && debug.contains("characteristic: CardType")
            && debug.contains("cards exiled with this creature"),
        "expected typed spell/comparison-set card-type intersection, got {debug}"
    );
}
