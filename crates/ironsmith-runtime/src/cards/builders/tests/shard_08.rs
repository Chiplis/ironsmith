#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::shard_02::*;
use super::shard_03::*;
use super::shard_04::*;
use super::shard_05::*;
use super::shard_06::*;
use super::shard_07::*;
use super::shard_09::*;
use super::shard_10::*;
use super::shard_11::*;
use super::shard_12::*;
use super::shard_13::*;
use super::shard_14::*;
use super::shard_15::*;
use super::shard_16::*;
use super::shard_17::*;
use super::shard_18::*;
use super::shard_19::*;
use super::shard_20::*;
use super::shard_21::*;
use super::shard_22::*;
use super::shard_23::*;
use super::*;

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_tap_untapped_creatures_cost_preserves_tap_filter_cost() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Hand of Justice Variant")
        .parse_text("{T}, Tap three untapped white creatures you control: Destroy target creature.")
        .expect("tap-untapped-creatures cost should parse");

    let ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("expected activated ability");
    let debug = format!("{:?}", ability.mana_cost);
    assert!(
        debug.contains("ChooseObjectsEffect"),
        "expected choose-objects tap cost in mana cost, got {debug}"
    );
    assert!(
        debug.contains("untapped: true"),
        "expected untapped filter requirement in tap cost, got {debug}"
    );
    assert!(
        debug.contains("count: ChoiceCount { min: 3, max: Some(3)")
            && debug.contains("dynamic_x: false"),
        "expected exactly-three tap cost selection, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
fn ordered_graveyard_compiled_text(name: &str, text: &str) -> String {
    let def = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .parse_text(text)
        .expect("ordered-graveyard card should parse");
    crate::compiled_text::unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase()
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn ordered_graveyard_alms_round_trips_top_card_activation_cost() {
    let rendered = ordered_graveyard_compiled_text(
        "Alms",
        "{1}, Exile the top card of your graveyard: Prevent the next 1 damage that would be dealt to target creature this turn.",
    );
    assert!(
        rendered.contains("exile the top card of your graveyard"),
        "expected Alms to preserve its ordered graveyard cost, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn ordered_graveyard_barrow_ghoul_round_trips_top_creature_unless_action() {
    let rendered = ordered_graveyard_compiled_text(
        "Barrow Ghoul",
        "At the beginning of your upkeep, sacrifice this creature unless you exile the top creature card of your graveyard.",
    );
    assert!(
        rendered.contains("unless you exile the top creature card of your graveyard"),
        "expected Barrow Ghoul to preserve its ordered graveyard alternative, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn ordered_graveyard_necratog_round_trips_top_creature_activation_cost() {
    let rendered = ordered_graveyard_compiled_text(
        "Necratog",
        "Exile the top creature card of your graveyard: This creature gets +2/+2 until end of turn.",
    );
    assert!(
        rendered.contains("exile the top creature card of your graveyard"),
        "expected Necratog to preserve its ordered graveyard cost, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn ordered_graveyard_soldevi_digger_round_trips_bottom_library_move() {
    let rendered = ordered_graveyard_compiled_text(
        "Soldevi Digger",
        "{2}: Put the top card of your graveyard on the bottom of your library.",
    );
    assert!(
        rendered.contains("put the top card of your graveyard on the bottom of your library"),
        "expected Soldevi Digger to preserve its ordered graveyard move, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn ordered_graveyard_zombie_scavengers_round_trips_top_creature_activation_cost() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Zombie Scavengers Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Exile the top creature card of your graveyard: Regenerate this creature.")
        .expect("exile-graveyard cost activated ability should parse");

    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let activated_line = lines.join(" ").to_ascii_lowercase();
    assert!(
        activated_line.contains("exile the top creature card of your graveyard")
            && activated_line.contains("regenerate"),
        "expected Zombie Scavengers to preserve its ordered cost and effect, got {activated_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_exile_source_cost_activated_line_preserves_followup_effect() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Selfless Glyphweaver Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Exile this creature: Creatures you control gain indestructible until end of turn.",
        )
        .expect("exile-source cost activated ability should parse");

    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let activated_line = lines.join(" ");
    assert!(
        activated_line.contains("Exile"),
        "expected exile cost to remain in activated ability text, got {activated_line}"
    );
    assert!(
        activated_line
            .to_ascii_lowercase()
            .contains("indestructible"),
        "expected post-colon indestructible effect to remain, got {activated_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_exile_this_card_from_graveyard_cost_uses_source_and_graveyard_zone() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ghoulcaller Variant")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "{3}{B}, Exile this card from your graveyard: Create a 2/2 black Zombie creature token. Activate only as a sorcery.",
            )
            .expect("exile-this-card-from-graveyard cost should parse as source exile");

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some((ability, activated)),
            _ => None,
        })
        .expect("expected activated ability");

    assert_eq!(
        activated.0.functional_zones,
        vec![Zone::Graveyard],
        "expected graveyard functional zone for self-exile from graveyard"
    );

    let mana_cost_debug = format!("{:?}", activated.1.mana_cost);
    assert!(
        mana_cost_debug.contains("ExileEffect") && mana_cost_debug.contains("Source"),
        "expected source exile in activation cost, got {mana_cost_debug}"
    );
    assert!(
        !mana_cost_debug.contains("exile_cost_0"),
        "self exile from graveyard should not route through tagged choose-cost, got {mana_cost_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_exile_spell_activation_cost_uses_stack_spell_filter() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Nivmagus Variant")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 2))
        .parse_text(
            "Exile an instant or sorcery spell you control: Put two +1/+1 counters on this creature.",
        )
        .expect("exile-spell activation cost should parse");

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("expected activated ability");
    let mana_cost_debug = format!("{:#?}", activated.mana_cost);
    let mana_cost_debug_compact = mana_cost_debug.split_whitespace().collect::<String>();
    assert!(
        mana_cost_debug_compact.contains("zone:Some(Stack"),
        "expected exile-spell cost to choose from the stack, got {mana_cost_debug}"
    );
    assert!(
        mana_cost_debug_compact.contains("stack_kind:Some(Spell"),
        "expected exile-spell cost to require a spell stack object, got {mana_cost_debug}"
    );
    assert!(
        !mana_cost_debug_compact.contains("zone:Some(Battlefield"),
        "exile-spell costs must not target battlefield objects, got {mana_cost_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "Exile an instant or sorcery spell you control: Put two +1/+1 counters on this creature."
        ),
        "expected debug-safe text to compact the stack-spell exile cost, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_each_player_may_choose_destroy_chosen_this_way_binds_iterated_choices() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Druid Variant")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 3))
        .parse_text(
            "When this creature enters, each player may choose an artifact or enchantment you don't control. Destroy each permanent chosen this way.",
        )
        .expect("each-player choose/destroy-chosen pattern should parse");

    let debug = format!("{def:#?}");
    let debug_compact = debug.split_whitespace().collect::<String>();
    assert!(
        debug_compact.contains("chooser:IteratedPlayer"),
        "each player should make their own choice, got {debug}"
    );
    assert!(
        debug_compact.contains("relation:IsTaggedObject"),
        "destroy-chosen follow-up should bind to chosen objects, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "When this creature enters, each player may choose an artifact or enchantment you don't control. Destroy each permanent chosen this way."
        ),
        "expected debug-safe text to compact the each-player choice and destroy follow-up, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_targeted_exile_activation_cost_fails_strictly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Targeted Exile Cost Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text("Exile target creature card from a graveyard: Draw a card.")
        .expect_err("targeted exile cost should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported targeted exile cost segment"),
        "expected strict targeted-exile-cost parse error, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_granted_activated_ability_to_non_source_compiles_as_grant() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Quicksmith Rebel Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, target artifact you control gains \"{T}: This artifact deals 2 damage to any target\" for as long as you control this creature.",
        )
        .expect("non-source granted activated ability should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("target artifact you control gains")
            && rendered.contains("{t}")
            && rendered.contains("this artifact deals 2 damage to any target"),
        "expected granted activated ability wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_put_target_creature_on_top_of_owner_library() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Griptide Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Put target creature on top of its owner's library.")
        .expect("put-on-top-of-library clause should parse");
    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let joined = lines.join("\n").to_ascii_lowercase();
    assert!(
        joined.contains("top of") && joined.contains("library"),
        "expected top-of-library move wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_controlled_creature_to_owner_library_does_not_infer_your_destination() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Nulltread Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, put a creature you control on top of its owner's library.",
        )
        .expect("owner-library placement should parse");
    let joined = unprocessed_compiled_lines(&def).join(" ");
    let debug = format!("{def:#?}");
    assert!(
        joined.contains("put a creature you control on top of its owner's library"),
        "expected owner-library destination wording, got {joined}"
    );
    assert!(
        debug.contains("destination_player_surface: None"),
        "target-side 'you control' must not become a destination player: {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_draw_then_put_source_on_top_of_library() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sensei Top Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text("{T}: Draw a card, then put this artifact on top of its owner's library.")
        .expect("draw-then-put-self-on-top clause should parse");
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("draw a card") && joined.contains("top of its owner's library"),
        "expected draw-then-put-self wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn gomazoa_activated_ability(def: &CardDefinition) -> &crate::ability::ActivatedAbility {
    def.abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Gomazoa should have an activated ability")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn execute_gomazoa_ability(
    game: &mut crate::game_state::GameState,
    source: ObjectId,
    controller: PlayerId,
    activated: &crate::ability::ActivatedAbility,
) -> crate::effect::EffectOutcome {
    let mut ctx = crate::effects::ExecutionContext::new_default(source, controller);
    let mut outcomes = Vec::new();
    for effect in activated.effects.flattened_default_effects() {
        outcomes.push(
            crate::effects::execute_effect(game, effect, &mut ctx)
                .expect("Gomazoa activated ability effect should resolve"),
        );
    }
    crate::effect::EffectOutcome::aggregate_summing_counts(outcomes)
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn library_contains_card_name(
    game: &crate::game_state::GameState,
    player: PlayerId,
    name: &str,
) -> bool {
    game.player(player)
        .expect("player exists")
        .library
        .iter()
        .any(|id| game.object(*id).is_some_and(|object| object.name == name))
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn shuffle_event_count_for_player(
    outcome: &crate::effect::EffectOutcome,
    player: PlayerId,
) -> usize {
    outcome
        .events
        .iter()
        .filter(|event| {
            event
                .downcast::<crate::events::ShuffleLibraryEvent>()
                .is_some_and(|shuffle| shuffle.player == player)
        })
        .count()
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_gomazoa_strict_and_renders_blocked_creatures_shuffle_clause() {
    let def = parse_oracle_card_definition("Gomazoa");
    let activated = gomazoa_activated_ability(&def);
    assert!(activated.has_tap_cost(), "Gomazoa should keep its tap cost");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("defender")
            && rendered.contains("flying")
            && rendered
                .contains("for each this creature or creature blocked by this creature this turn")
            && rendered.contains("put it on top of its owner's library")
            && rendered.contains("shuffle its owner's library"),
        "expected Gomazoa compiled text to preserve the blocking top-library shuffle clause, got {rendered}"
    );

    let ability_debug = format!("{activated:#?}").to_ascii_lowercase();
    assert!(
        ability_debug.contains("foreachobject")
            && ability_debug.contains("blocked_by_source: true")
            && ability_debug.contains("movetozoneeffect")
            && ability_debug.contains("shufflelibraryeffect"),
        "expected Gomazoa ability to structurally move source plus blocked creatures and shuffle owners, got {ability_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn gomazoa_runtime_moves_source_and_creature_it_is_blocking_to_owners_libraries() {
    let def = parse_oracle_card_definition("Gomazoa");
    let activated = gomazoa_activated_ability(&def);
    let vanilla = CardDefinitionBuilder::new(CardId::new(), "Attacking Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let gomazoa = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let attacker = game.create_object_from_definition(&vanilla, bob, Zone::Battlefield);
    let mut combat = crate::combat_state::CombatState::default();
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: attacker,
        target: crate::combat_state::AttackTarget::Player(alice),
    });
    combat.blockers.insert(attacker, vec![gomazoa]);
    game.combat = Some(combat);

    execute_gomazoa_ability(&mut game, gomazoa, alice, activated);

    assert!(
        !game.battlefield.contains(&gomazoa) && !game.battlefield.contains(&attacker),
        "Gomazoa and the creature it blocked should leave the battlefield"
    );
    assert!(
        library_contains_card_name(&game, alice, "Gomazoa")
            && library_contains_card_name(&game, bob, "Attacking Bear"),
        "Gomazoa and its blocked attacker should be in their owners' libraries"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn gomazoa_runtime_does_not_move_creature_blocking_it_when_source_attacks() {
    let def = parse_oracle_card_definition("Gomazoa");
    let activated = gomazoa_activated_ability(&def);
    let blocker_def = CardDefinitionBuilder::new(CardId::new(), "Blocking Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let gomazoa = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let blocker = game.create_object_from_definition(&blocker_def, bob, Zone::Battlefield);
    let mut combat = crate::combat_state::CombatState::default();
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: gomazoa,
        target: crate::combat_state::AttackTarget::Player(bob),
    });
    combat.blockers.insert(gomazoa, vec![blocker]);
    game.combat = Some(combat);

    execute_gomazoa_ability(&mut game, gomazoa, alice, activated);

    assert!(
        !game.battlefield.contains(&gomazoa),
        "Gomazoa should move itself even if it is attacking"
    );
    assert!(
        game.battlefield.contains(&blocker)
            && !library_contains_card_name(&game, bob, "Blocking Bear"),
        "a creature blocking Gomazoa is not a creature Gomazoa is blocking"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn gomazoa_runtime_shuffles_each_affected_owner_once_after_all_moves() {
    let def = parse_oracle_card_definition("Gomazoa");
    let activated = gomazoa_activated_ability(&def);
    let bear = CardDefinitionBuilder::new(CardId::new(), "Owned Attacking Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let elk = CardDefinitionBuilder::new(CardId::new(), "Owned Attacking Elk")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let gomazoa = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let bear = game.create_object_from_definition(&bear, alice, Zone::Battlefield);
    let elk = game.create_object_from_definition(&elk, alice, Zone::Battlefield);
    let mut combat = crate::combat_state::CombatState::default();
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: bear,
        target: crate::combat_state::AttackTarget::Player(bob),
    });
    combat.attackers.push(crate::combat_state::AttackerInfo {
        creature: elk,
        target: crate::combat_state::AttackTarget::Player(bob),
    });
    combat.blockers.insert(bear, vec![gomazoa]);
    combat.blockers.insert(elk, vec![gomazoa]);
    game.combat = Some(combat);

    let outcome = execute_gomazoa_ability(&mut game, gomazoa, alice, activated);

    assert!(
        library_contains_card_name(&game, alice, "Gomazoa")
            && library_contains_card_name(&game, alice, "Owned Attacking Bear")
            && library_contains_card_name(&game, alice, "Owned Attacking Elk"),
        "all affected permanents owned by Alice should move to her library"
    );
    assert_eq!(
        shuffle_event_count_for_player(&outcome, alice),
        1,
        "Gomazoa should make each affected owner shuffle once, not once per affected object"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_draw_then_put_source_third_from_top() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Sensei Top Third Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "{T}: Draw a card, then put this artifact third from the top of its owner's library.",
        )
        .expect_err("third-from-top library-position tail remains unsupported");
    let message = format!("{err:?}").to_ascii_lowercase();
    assert!(
        message.contains("unsupported put clause"),
        "expected strict unsupported third-from-top clause, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_put_target_beneath_top_x_cards() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Unexpectedly Absent Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Put target nonland permanent into its owner's library just beneath the top X cards of that library.",
        )
        .expect("beneath-top-x library-position clause should parse");
    let message = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        message.contains("just beneath the top x cards"),
        "expected beneath-top-x wording, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_put_target_third_from_bottom_still_fails_loudly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Library Bottom Negative Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Put target nonland permanent into its owner's library third from the bottom.")
        .expect_err("unsupported bottom-position clause should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported put clause"),
        "expected strict unsupported put-clause error, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_triggered_put_into_graveyard_from_anywhere() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Worldspine Trigger Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature is put into a graveyard from anywhere, shuffle it into its owner's library.")
        .expect("put-into-graveyard-from-anywhere trigger should parse");
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    let debug = format!("{def:#?}");
    assert!(
        joined.contains("is put into a graveyard from anywhere")
            && joined.contains("shuffle it into its owner's library"),
        "expected graveyard-from-anywhere trigger wording, got {joined}"
    );
    assert!(
        debug.contains("owner_library_destination: true") && debug.contains("player: OwnerOf("),
        "expected structural owner-library relation, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_possessive_target_owner_shuffle_preserves_subject_surface() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Possessive Owner Shuffle Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Target creature's owner shuffles it into their library.")
        .expect("possessive target-owner shuffle should parse");
    assert_eq!(
        unprocessed_compiled_lines(&def),
        ["Target creature's owner shuffles it into their library."]
    );
    let debug = format!("{def:#?}");
    let compact_debug = debug.split_whitespace().collect::<String>();
    assert!(
        compact_debug.contains("possessive_owner_subject:true")
            && compact_debug.contains("player:OwnerOf(Target,"),
        "expected typed possessive owner surface, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_triggered_put_into_exile_from_anywhere() {
    let def = CardDefinitionBuilder::new(CardId::new(), "From Anywhere Exile Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature is put into exile from anywhere, shuffle it into its owner's library.")
        .expect("put-into-exile-from-anywhere trigger should parse");
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    let debug = format!("{:#?}", def.abilities);
    assert!(
        joined.contains("put into exile")
            && joined.contains("shuffle")
            && joined.contains("library"),
        "expected exile-from-anywhere trigger wording, got {joined}"
    );
    assert!(
        debug.contains("ZoneChangeTrigger")
            && debug.contains("from: Any")
            && debug.contains("Exile"),
        "expected exile-from-anywhere trigger model, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_add_any_color_for_each_removed_counter() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Coalition Relic Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "At the beginning of your first main phase, remove all charge counters from this artifact. Add one mana of any color for each charge counter removed this way.",
        )
        .expect("dynamic removed-counter mana clause should parse");
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("remove all charge counters from it")
            && joined.contains("add one mana of any color for each counter removed this way"),
        "expected removed-counter mana wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_add_any_color_for_each_removed_counter_with_unsupported_tail_fails_strictly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Coalition Relic Negative Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "At the beginning of your first main phase, remove all charge counters from this artifact. Add one mana of any color for each charge counter removed this way unless it's your turn.",
        )
        .expect_err("unsupported removed-counter mana tail should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported trailing mana clause"),
        "expected strict trailing-mana error, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_touch_of_the_eternal_counted_permanents_life_total_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(278_197), "Touch of the Eternal")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(5)],
            vec![ManaSymbol::White],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "At the beginning of your upkeep, count the number of permanents you control. Your life total becomes that number.",
        )
        .expect("Touch of the Eternal should parse strictly");

    let debug = format!("{def:#?}");
    let compact_debug = debug.split_whitespace().collect::<String>();
    assert!(debug.contains("SetLifeTotalEffect"), "{debug}");
    assert!(
        compact_debug.contains("Count(") && compact_debug.contains("controller:Some(You"),
        "expected Touch of the Eternal to count permanents you control, got {debug}"
    );

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("at the beginning of your upkeep")
            && joined.contains("count the number of permanents you control")
            && joined.contains("your life total becomes that number"),
        "expected counted-permanents life-total wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_starting_life_total_amount_in_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Endstone Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "At the beginning of your end step, your life total becomes half your starting life total, rounded up.",
        )
        .expect("starting-life-total amount should parse");
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("half your starting life total, rounded up"),
        "expected starting-life-total wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_starting_life_total_amount_with_extra_math_fails_strictly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Endstone Negative Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "At the beginning of your end step, your life total becomes half your starting life total plus one.",
        )
        .expect_err("unsupported starting-life-total math should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("missing life total amount"),
        "expected strict missing-life-total-amount error, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_deep_water_mana_replacement_clause_strictly() {
    let def = deep_water_test_definition();
    let activated = deep_water_activated_ability(&def);
    assert_eq!(
        activated.mana_cost.display(),
        "{U}",
        "Deep Water activation cost should parse as {{U}}"
    );

    let debug = format!("{:?}", activated.effects);
    assert!(
        debug.contains("RegisterManaReplacement"),
        "Deep Water should lower to a mana replacement registration, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn deep_water_compiled_text_renders_mana_replacement_clause() {
    let def = deep_water_test_definition();
    let joined = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        joined.contains(
            "Until end of turn, if you tap a land you control for mana, it produces {U} instead of any other type"
        ),
        "expected Deep Water replacement clause in compiled text, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn deep_water_replaces_mana_from_land_you_control_until_end_of_turn() {
    let alice = PlayerId::from_index(0);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let deep_water_id =
        game.create_object_from_definition(&deep_water_test_definition(), alice, Zone::Battlefield);

    resolve_deep_water_activation(&mut game, deep_water_id, alice, &mut dm);

    let land_id = create_deep_water_test_land(&mut game, alice, "Deep Water Test Swamp");
    crate::special_actions::perform_activate_mana_ability(&mut game, alice, land_id, 0, &mut dm)
        .expect("land mana ability should activate");

    let pool = &game.player(alice).expect("alice").mana_pool;
    assert_eq!(
        pool.blue, 1,
        "Deep Water should replace the land's mana with blue"
    );
    assert_eq!(
        pool.black, 0,
        "the land's original black mana should not be produced"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn deep_water_preserves_amount_from_multi_mana_land() {
    let alice = PlayerId::from_index(0);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let deep_water_id =
        game.create_object_from_definition(&deep_water_test_definition(), alice, Zone::Battlefield);

    resolve_deep_water_activation(&mut game, deep_water_id, alice, &mut dm);

    let land_id = create_deep_water_test_land_with_mana(
        &mut game,
        alice,
        "Deep Water Multi-Mana Test Land",
        vec![ManaSymbol::Black, ManaSymbol::Colorless],
    );
    crate::special_actions::perform_activate_mana_ability(&mut game, alice, land_id, 0, &mut dm)
        .expect("multi-mana land ability should activate");

    let pool = &game.player(alice).expect("alice").mana_pool;
    assert_eq!(
        pool.blue, 2,
        "Deep Water should preserve the amount of mana while making it all blue"
    );
    assert_eq!(
        pool.black, 0,
        "the original black mana should not be produced"
    );
    assert_eq!(
        pool.colorless, 0,
        "the original colorless mana should not be produced"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn deep_water_replaces_effect_based_mana_from_tapped_land_you_control() {
    let alice = PlayerId::from_index(0);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let deep_water_id =
        game.create_object_from_definition(&deep_water_test_definition(), alice, Zone::Battlefield);

    resolve_deep_water_activation(&mut game, deep_water_id, alice, &mut dm);

    let land_id = create_deep_water_test_land_with_mana_effect(
        &mut game,
        alice,
        "Deep Water Effect-Mana Test Land",
        vec![ManaSymbol::Black],
    );
    let events =
        crate::special_actions::perform_activate_mana_ability_restricted_colors_with_events(
            &mut game, alice, land_id, 0, None, &mut dm,
        )
        .expect("effect-based land mana ability should activate");

    let pool = &game.player(alice).expect("alice").mana_pool;
    assert_eq!(
        pool.blue, 1,
        "Deep Water should replace effect-produced land mana with blue"
    );
    assert_eq!(
        pool.black, 0,
        "the effect's original black mana should not be produced"
    );

    let event = events
        .iter()
        .find_map(|event| event.downcast::<crate::events::ManaAddedEvent>())
        .expect("effect-based mana ability should emit a ManaAddedEvent");
    assert_eq!(event.source, land_id);
    assert_eq!(event.controller, alice);
    assert_eq!(event.player, alice);
    assert_eq!(
        event.provenance,
        crate::events::mana::ManaProductionProvenance::TappedSourceForMana,
        "the emitted mana event should record that the source was tapped for mana"
    );
    assert_eq!(
        event.mana,
        vec![ManaSymbol::Blue],
        "the emitted mana event should carry the replaced mana"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn deep_water_does_not_replace_effect_based_mana_from_free_land_ability() {
    let alice = PlayerId::from_index(0);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let deep_water_id =
        game.create_object_from_definition(&deep_water_test_definition(), alice, Zone::Battlefield);

    resolve_deep_water_activation(&mut game, deep_water_id, alice, &mut dm);

    let land_id = create_deep_water_test_land_with_free_mana_effect(
        &mut game,
        alice,
        "Deep Water Free Effect-Mana Test Land",
        vec![ManaSymbol::Black],
    );
    let events =
        crate::special_actions::perform_activate_mana_ability_restricted_colors_with_events(
            &mut game, alice, land_id, 0, None, &mut dm,
        )
        .expect("free effect-based land mana ability should activate");

    let pool = &game.player(alice).expect("alice").mana_pool;
    assert_eq!(
        pool.black, 1,
        "free effect-produced land mana should keep its original black mana"
    );
    assert_eq!(
        pool.blue, 0,
        "Deep Water should only replace mana produced by tapping a land for mana"
    );
    assert!(
        !game.is_tapped(land_id),
        "the free mana ability should not tap the land"
    );

    let event = events
        .iter()
        .find_map(|event| event.downcast::<crate::events::ManaAddedEvent>())
        .expect("free effect-based mana ability should emit a ManaAddedEvent");
    assert_eq!(
        event.provenance,
        crate::events::mana::ManaProductionProvenance::Unknown,
        "the emitted mana event should not claim the source was tapped for mana"
    );
    assert_eq!(
        event.mana,
        vec![ManaSymbol::Black],
        "the emitted mana event should carry unreplaced mana"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn deep_water_mana_replacement_expires_at_cleanup() {
    let alice = PlayerId::from_index(0);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let deep_water_id =
        game.create_object_from_definition(&deep_water_test_definition(), alice, Zone::Battlefield);

    resolve_deep_water_activation(&mut game, deep_water_id, alice, &mut dm);
    crate::turn::execute_cleanup_step(&mut game);

    let land_id = create_deep_water_test_land(&mut game, alice, "Post-Cleanup Test Swamp");
    crate::special_actions::perform_activate_mana_ability(&mut game, alice, land_id, 0, &mut dm)
        .expect("land mana ability should activate after cleanup");

    let pool = &game.player(alice).expect("alice").mana_pool;
    assert_eq!(
        pool.black, 1,
        "the original black mana should be produced after cleanup"
    );
    assert_eq!(
        pool.blue, 0,
        "Deep Water's replacement should expire at cleanup"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn deep_water_does_not_replace_mana_from_nonland_you_control() {
    let alice = PlayerId::from_index(0);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let deep_water_id =
        game.create_object_from_definition(&deep_water_test_definition(), alice, Zone::Battlefield);

    resolve_deep_water_activation(&mut game, deep_water_id, alice, &mut dm);

    let artifact_id = create_deep_water_test_mana_artifact(&mut game, alice);
    crate::special_actions::perform_activate_mana_ability(
        &mut game,
        alice,
        artifact_id,
        0,
        &mut dm,
    )
    .expect("artifact mana ability should activate");

    let pool = &game.player(alice).expect("alice").mana_pool;
    assert_eq!(
        pool.black, 1,
        "nonland mana should keep its original black mana"
    );
    assert_eq!(
        pool.blue, 0,
        "Deep Water should only replace mana from lands"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn deep_water_does_not_replace_mana_from_land_you_do_not_control() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let deep_water_id =
        game.create_object_from_definition(&deep_water_test_definition(), alice, Zone::Battlefield);

    resolve_deep_water_activation(&mut game, deep_water_id, alice, &mut dm);

    let land_id = create_deep_water_test_land(&mut game, bob, "Bob's Deep Water Test Swamp");
    crate::special_actions::perform_activate_mana_ability(&mut game, bob, land_id, 0, &mut dm)
        .expect("opponent land mana ability should activate");

    let pool = &game.player(bob).expect("bob").mana_pool;
    assert_eq!(
        pool.black, 1,
        "Bob's land should keep its original black mana"
    );
    assert_eq!(
        pool.blue, 0,
        "Deep Water should not replace mana from lands Alice doesn't control"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn deep_water_does_not_replace_effect_based_mana_from_nonland_or_uncontrolled_land() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let deep_water_id =
        game.create_object_from_definition(&deep_water_test_definition(), alice, Zone::Battlefield);

    resolve_deep_water_activation(&mut game, deep_water_id, alice, &mut dm);

    let artifact_id = create_deep_water_test_mana_artifact_with_effect(&mut game, alice);
    crate::special_actions::perform_activate_mana_ability(
        &mut game,
        alice,
        artifact_id,
        0,
        &mut dm,
    )
    .expect("effect-based artifact mana ability should activate");

    let bob_land_id = create_deep_water_test_land_with_mana_effect(
        &mut game,
        bob,
        "Bob's Deep Water Effect-Mana Test Land",
        vec![ManaSymbol::Black],
    );
    crate::special_actions::perform_activate_mana_ability(&mut game, bob, bob_land_id, 0, &mut dm)
        .expect("opponent effect-based land mana ability should activate");

    let alice_pool = &game.player(alice).expect("alice").mana_pool;
    assert_eq!(
        alice_pool.black, 1,
        "effect-based nonland mana should keep its original black mana"
    );
    assert_eq!(
        alice_pool.blue, 0,
        "Deep Water should not replace effect-based nonland mana"
    );

    let bob_pool = &game.player(bob).expect("bob").mana_pool;
    assert_eq!(
        bob_pool.black, 1,
        "effect-based mana from an uncontrolled land should keep its original black mana"
    );
    assert_eq!(
        bob_pool.blue, 0,
        "Deep Water should not replace effect-based land mana Alice does not control"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn deep_water_test_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Deep Water")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue], vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "{U}: Until end of turn, if you tap a land you control for mana, it produces {U} instead of any other type.",
        )
        .expect("Deep Water should parse strictly")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn deep_water_activated_ability(
    def: &CardDefinition,
) -> &crate::ability::ActivatedAbility {
    def.abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Deep Water should have an activated ability")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resolve_deep_water_activation(
    game: &mut crate::game_state::GameState,
    deep_water_id: ObjectId,
    controller: PlayerId,
    dm: &mut dyn crate::decision::DecisionMaker,
) {
    let effects = {
        let object = game.object(deep_water_id).expect("Deep Water should exist");
        let activated = object
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(activated),
                _ => None,
            })
            .expect("Deep Water should have an activated ability");
        activated.effects.clone()
    };
    let mut ctx = crate::effects::ExecutionContext::new(deep_water_id, controller, dm);
    crate::game_loop::execute_resolution_program(
        game,
        &mut ctx,
        controller,
        deep_water_id,
        &effects,
        None,
        &[],
    )
    .expect("Deep Water activation should register a mana replacement");
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn create_deep_water_test_land(
    game: &mut crate::game_state::GameState,
    controller: PlayerId,
    name: &str,
) -> ObjectId {
    create_deep_water_test_land_with_mana(game, controller, name, vec![ManaSymbol::Black])
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn create_deep_water_test_land_with_mana(
    game: &mut crate::game_state::GameState,
    controller: PlayerId,
    name: &str,
    mana: Vec<ManaSymbol>,
) -> ObjectId {
    let land = crate::card::CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Land])
        .build();
    let land_id = game.create_object_from_card(&land, controller, Zone::Battlefield);
    game.object_mut(land_id)
        .expect("land should exist")
        .abilities_mut()
        .push(crate::ability::Ability::mana(
            crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
            mana,
        ));
    land_id
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn create_deep_water_test_land_with_mana_effect(
    game: &mut crate::game_state::GameState,
    controller: PlayerId,
    name: &str,
    mana: Vec<ManaSymbol>,
) -> ObjectId {
    let land = crate::card::CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Land])
        .build();
    let land_id = game.create_object_from_card(&land, controller, Zone::Battlefield);
    game.object_mut(land_id)
        .expect("land should exist")
        .abilities_mut()
        .push(crate::ability::Ability::mana_with_effects(
            crate::cost::TotalCost::free(),
            vec![crate::effect::Effect::add_mana(mana)],
        ));
    land_id
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn create_deep_water_test_land_with_free_mana_effect(
    game: &mut crate::game_state::GameState,
    controller: PlayerId,
    name: &str,
    mana: Vec<ManaSymbol>,
) -> ObjectId {
    let land = crate::card::CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Land])
        .build();
    let land_id = game.create_object_from_card(&land, controller, Zone::Battlefield);
    game.object_mut(land_id)
        .expect("land should exist")
        .abilities_mut()
        .push(crate::ability::Ability::activated(
            crate::cost::TotalCost::free(),
            vec![crate::effect::Effect::add_mana(mana)],
        ));
    land_id
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn create_deep_water_test_mana_artifact(
    game: &mut crate::game_state::GameState,
    controller: PlayerId,
) -> ObjectId {
    let artifact = crate::card::CardBuilder::new(CardId::new(), "Deep Water Test Mana Rock")
        .card_types(vec![CardType::Artifact])
        .build();
    let artifact_id = game.create_object_from_card(&artifact, controller, Zone::Battlefield);
    game.object_mut(artifact_id)
        .expect("artifact should exist")
        .abilities_mut()
        .push(crate::ability::Ability::mana(
            crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
            vec![ManaSymbol::Black],
        ));
    artifact_id
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn create_deep_water_test_mana_artifact_with_effect(
    game: &mut crate::game_state::GameState,
    controller: PlayerId,
) -> ObjectId {
    let artifact = crate::card::CardBuilder::new(CardId::new(), "Deep Water Effect-Mana Test Rock")
        .card_types(vec![CardType::Artifact])
        .build();
    let artifact_id = game.create_object_from_card(&artifact, controller, Zone::Battlefield);
    game.object_mut(artifact_id)
        .expect("artifact should exist")
        .abilities_mut()
        .push(crate::ability::Ability::mana_with_effects(
            crate::cost::TotalCost::free(),
            vec![crate::effect::Effect::add_mana(vec![ManaSymbol::Black])],
        ));
    artifact_id
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_mana_replacement_clause_harvest_mage_fails_instead_of_partial_tap() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Harvest Mage Variant")
            .parse_text(
                "{G}, {T}, Discard a card: Until end of turn, if you tap a land for mana, it produces one mana of a color of your choice instead of any other type and amount.",
            )
            .expect_err("unsupported mana replacement clause should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported mana replacement clause")
            || message.contains("unsupported until-end-of-turn permission clause")
            || message.contains("could not find verb in effect clause"),
        "expected strict mana replacement parse error, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_mana_replacement_clause_with_taps_plural_fails_strictly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Pale Moon Variant")
            .parse_text(
                "Until end of turn, if a player taps a nonbasic land for mana, it produces colorless mana instead of any other type.",
            )
            .expect_err("unsupported mana replacement clause should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported mana replacement clause")
            || message.contains("unsupported until-end-of-turn permission clause")
            || message.contains("could not find verb in effect clause"),
        "expected strict mana replacement parse error, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_mana_trigger_additional_clause_high_tide_fails_strictly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "High Tide Variant")
            .parse_text(
                "Until end of turn, whenever a player taps an Island for mana, that player adds an additional {U}.",
            )
            .expect_err("unsupported mana-triggered additional-mana clause should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported mana-triggered additional-mana clause")
            || message.contains("unsupported until-end-of-turn permission clause"),
        "expected strict mana-triggered parse error, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_add_mana_chosen_color_tail() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Thriving Mana Variant")
        .card_types(vec![CardType::Land])
        .parse_text("{T}: Add {B} or one mana of the chosen color.")
        .expect("chosen-color mana tail should parse");
    let lines = unprocessed_compiled_lines(&def);
    let mana_line = lines.join(" ");
    assert!(
        mana_line.contains("Add {B} or one mana of the chosen color"),
        "expected chosen-color mana render, got {mana_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_chosen_color_mana_for_each_different_power() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Selvala Mana Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{T}: Choose a color. Add one mana of that color for each different power among creatures you control.",
        )
        .expect("chosen-color mana scaled by distinct powers should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        (rendered.contains("Choose a color") || rendered.contains("You choose a color"))
            && (rendered.contains(
                "Add one mana of that color for each different power among creatures you control",
            ) || rendered.contains(
                "Add one mana of the chosen color for each different power among creatures you control",
            )),
        "expected distinct-power chosen-color mana render, got {rendered}"
    );
    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("AddManaOfChosenColorEffect") && debug.contains("DistinctPowers"),
        "expected chosen-color mana effect scaled by distinct powers, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_urzas_tower_conditional_mana_output() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Urza's Tower Variant")
        .card_types(vec![CardType::Land])
        .parse_text(
            "{T}: Add {C}. If you control an Urza's Mine and an Urza's Power-Plant, add {C}{C}{C} instead.",
        )
        .expect("urza tower mana followup should parse");

    let mana_line = unprocessed_compiled_lines(&def).join(" ");
    let mana_lower = mana_line.to_ascii_lowercase();
    assert!(
        mana_lower.contains("if you control")
            && mana_lower.contains("add {c}{c}{c}")
            && mana_lower.contains("add {c}"),
        "expected conditional tron mana render, got {mana_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_activated_ability_instead_followup_builds_stack_self_replacement_program() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Draw Relay Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text("{T}: Draw a card. If you control an artifact, draw two cards instead.")
        .expect("activated ability instead followup should parse");

    let ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("expected activated ability");
    let entry = crate::game_state::StackEntry::ability(
        ObjectId::from_raw(1),
        PlayerId::from_index(0),
        ability.effects.clone(),
    );
    let program = entry
        .ability_effects
        .as_ref()
        .expect("stack ability program");
    assert_eq!(program.segments.len(), 1);
    assert_eq!(program.segments[0].self_replacements.len(), 1);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_urza_tron_other_lands_conditional_mana_followups() {
    for (name, text, _other_a, _other_b) in [
        (
            "Urza's Mine Variant",
            "{T}: Add {C}. If you control an Urza's Power-Plant and an Urza's Tower, add {C}{C} instead.",
            "Urza's Power-Plant",
            "Urza's Tower",
        ),
        (
            "Urza's Power-Plant Variant",
            "{T}: Add {C}. If you control an Urza's Mine and an Urza's Tower, add {C}{C} instead.",
            "Urza's Mine",
            "Urza's Tower",
        ),
    ] {
        let def = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Land])
            .parse_text(text)
            .expect("other tron land mana followup should parse");
        let mana_line = unprocessed_compiled_lines(&def).join(" ");
        let mana_lower = mana_line.to_ascii_lowercase();
        assert!(
            mana_lower.contains("if you control")
                && mana_lower.contains("add {c}{c}")
                && mana_lower.contains("add {c}"),
            "expected conditional tron mana render, got {mana_line}"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_metalcraft_mana_activation_condition() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mox Opal Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "{T}: Add one mana of any color. Activate only if you control three or more artifacts.",
        )
        .expect("metalcraft mana activation condition should parse");

    let lines = unprocessed_compiled_lines(&def);
    let mana_line = lines.join(" ");
    assert!(
        mana_line.contains("Add one mana of any color"),
        "expected mana production text in compiled output, got {mana_line}"
    );
    assert!(
        mana_line.contains("Activate only if you control 3 or more artifacts")
            || mana_line.contains("Activate only if you control three or more artifacts"),
        "expected rendered activation restriction, got {mana_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_land_count_mana_activation_condition() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Temple Variant")
        .card_types(vec![CardType::Land])
        .parse_text("{T}: Add {C}{C}. Activate only if you control five or more lands.")
        .expect("land-count mana activation condition should parse");

    let lines = unprocessed_compiled_lines(&def);
    let mana_line = lines.join(" ");
    assert!(
        mana_line.contains("Add {C}{C}"),
        "expected mana amount in compiled output, got {mana_line}"
    );
    assert!(
        mana_line.contains("Activate only if you control 5 or more lands")
            || mana_line.contains("Activate only if you control five or more lands"),
        "expected rendered activation restriction, got {mana_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_graveyard_card_mana_activation_condition() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Elf Tomb Variant")
        .card_types(vec![CardType::Land])
        .parse_text("{T}: Add {G}{G}. Activate only if there is an Elf card in your graveyard.")
        .expect("graveyard-card mana activation condition should parse");

    let lines = unprocessed_compiled_lines(&def);
    let mana_line = lines.join(" ");
    assert!(
        mana_line.contains("Add {G}{G}"),
        "expected mana amount in compiled output, got {mana_line}"
    );
    assert!(
        mana_line.contains("Activate only if there is an elf card in your graveyard"),
        "expected rendered activation restriction, got {mana_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_creature_power_mana_activation_condition() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ferocious Mana Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{T}: Add {G}{G}. Activate only if you control a creature with power 4 or greater.",
        )
        .expect("creature-power mana activation condition should parse");

    let lines = unprocessed_compiled_lines(&def);
    let mana_line = lines.join(" ");
    assert!(
        mana_line.contains("Add {G}{G}"),
        "expected mana amount in compiled output, got {mana_line}"
    );
    assert!(
        mana_line.contains("Activate only if you control a creature with power 4 or greater")
            || mana_line.contains("Activate only if you control creature with power 4 or greater"),
        "expected rendered activation restriction, got {mana_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_total_power_mana_activation_condition() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Formidable Mana Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}: Add {C}{C}{C}. Activate only if creatures you control have total power 8 or greater.")
        .expect("total-power mana activation condition should parse");

    let lines = unprocessed_compiled_lines(&def);
    let mana_line = lines.join(" ");
    assert!(
        mana_line.contains("Add {C}{C}{C}"),
        "expected mana amount in compiled output, got {mana_line}"
    );
    assert!(
        mana_line.contains("Activate only if creatures you control have total power 8 or greater"),
        "expected rendered activation restriction, got {mana_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_inline_whenever_clause_keeps_its_controller_subject() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Noxious Assault Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Creatures you control get +2/+2 until end of turn. Whenever a creature blocks this turn, its controller gets a poison counter.",
        )
        .expect("inline whenever clause with its-controller subject should parse");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("controller gets a poison counter")
            || joined.contains("that object's controller gets a poison counter"),
        "expected controller-based poison counter wording, got {joined}"
    );
    assert!(
        !joined.contains("you get 1 poison counter"),
        "did not expect implicit-you poison counter wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_until_end_of_turn_whenever_clause_as_temporary_grant() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mountain Titan Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{1}{R}{R}: Until end of turn, whenever you cast a black spell, put a +1/+1 counter on this creature.")
        .expect("until-end-of-turn whenever grant should parse as a temporary delayed trigger");
    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("expected activated ability");
    let schedule = activated
        .effects
        .flattened_default_effects()
        .into_iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>())
        .expect("temporary whenever clause should schedule a delayed trigger");
    assert!(schedule.until_end_of_turn);
    assert_eq!(
        schedule.duration,
        ironsmith_core::DelayedTriggerDuration::EndOfTurn
    );
    assert!(
        schedule.leading_duration_surface,
        "the authored leading duration should survive lowering"
    );
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("until end of turn, whenever you cast a black spell")
            && rendered.contains("put a +1/+1 counter on this creature"),
        "expected temporary delayed-trigger surface, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_rejects_marker_keyword_with_non_keyword_tail() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Ninjutsu Tail Variant")
        .parse_text("Ninjutsu abilities you activate cost {1} less to activate.")
        .expect_err("non-keyword ninjutsu tail should not parse as a bare keyword");
    let message = format!("{err:?}");
    assert!(
        message.contains("could not find verb")
            || message.contains("unsupported")
            || message.contains("parse"),
        "expected parse failure for non-keyword tail, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_ninjutsu_keyword_line_builds_hand_activated_ability() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ninjutsu Probe")
        .parse_text("Ninjutsu {1}{B}")
        .expect("ninjutsu keyword line should parse");

    let ability = def
        .abilities
        .iter()
        .find(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("expected activated ninjutsu ability");
    assert!(
        ability.functional_zones.contains(&Zone::Hand),
        "ninjutsu should function from hand"
    );
    let AbilityKind::Activated(activated) = &ability.kind else {
        panic!("expected activated ability");
    };
    assert_eq!(
        activated.timing,
        crate::ability::ActivationTiming::DuringCombat,
        "ninjutsu should use during-combat timing"
    );

    let cost_debug = format!("{:?}", activated.mana_cost);
    assert!(
        cost_debug.contains("NinjutsuCostEffect"),
        "expected ninjutsu return-attacker cost effect, got {cost_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("ninjutsu {1}{b}"),
        "expected compact ninjutsu keyword surface, got {rendered}"
    );
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("NinjutsuCostEffect") && debug.contains("NinjutsuEffect"),
        "expected compiled model to keep the ninjutsu effect pipeline, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn satoru_umezawa_keeps_named_ninjutsu_trigger_and_look_sequence() {
    let def = parse_oracle_card_definition("Satoru Umezawa");
    let rendered = unprocessed_compiled_lines(&def);
    assert_eq!(
        rendered.first().map(String::as_str),
        Some(
            "Whenever you activate a ninjutsu ability, look at the top three cards of your library. Put one of them into your hand and the rest on the bottom of your library in any order. This ability triggers only once each turn."
        )
    );

    let debug = format!("{def:#?}");
    let compact_debug: String = debug.chars().filter(|ch| !ch.is_whitespace()).collect();
    assert!(
        debug.contains("AbilityActivatedTrigger")
            && compact_debug.contains("ability_markers:[\"ninjutsu\"")
            && debug.contains("LookAtTopCardsEffect")
            && debug.contains("PutTaggedRemainderOnLibraryBottomEffect"),
        "Satoru should retain the named trigger and complete looked-card pipeline: {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn splinters_technique_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Splinter's Technique");

    let sneak = def
        .alternative_casts
        .iter()
        .find(|method| method.name().eq_ignore_ascii_case("Sneak"))
        .expect("Splinter's Technique should compile with a sneak alternative cost");
    assert_eq!(
        sneak.mana_cost().map(|cost| cost.to_oracle()),
        Some("{1}{B}".to_string()),
        "Sneak should preserve its printed mana cost"
    );
    assert!(
        sneak.non_mana_costs().iter().any(|cost| {
            cost.effect_ref().is_some_and(|effect| {
                effect
                    .downcast_ref::<crate::effects::SneakCostEffect>()
                    .is_some()
            })
        }),
        "Sneak should require returning an unblocked attacker as a real cost"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered.contains("Sneak {1}{B}"),
        "compiled text should preserve the sneak keyword cost, got {rendered}"
    );
    assert!(
        rendered_lower.contains("search your library for a card")
            && (rendered_lower.contains("put that card into your hand")
                || rendered_lower.contains("put it into your hand"))
            && rendered_lower.contains("then shuffle"),
        "Splinter's Technique should keep its tutor effect, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn kitsunes_technique_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Kitsune's Technique");

    let sneak = def
        .alternative_casts
        .iter()
        .find(|method| method.name().eq_ignore_ascii_case("Sneak"))
        .expect("Kitsune's Technique should compile with a sneak alternative cost");
    assert_eq!(
        sneak.mana_cost().map(|cost| cost.to_oracle()),
        Some("{1}{U}".to_string()),
        "Kitsune's Technique should preserve its printed Sneak cost"
    );
    assert!(
        sneak.non_mana_costs().iter().any(|cost| {
            cost.effect_ref().is_some_and(|effect| {
                effect
                    .downcast_ref::<crate::effects::SneakCostEffect>()
                    .is_some()
            })
        }),
        "Kitsune's Technique Sneak should require returning an unblocked attacker as a real cost"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Sneak {1}{U}"),
        "compiled text should preserve the Kitsune's Technique sneak keyword cost, got {rendered}"
    );
    assert!(
        rendered.contains("Target opponent mills half their library, rounded up"),
        "compiled text should render the rounded-up half-library mill clause, got {rendered}"
    );

    let spell_debug = format!("{:#?}", def.spell_effect);
    assert!(
        spell_debug.contains("MillEffect")
            && spell_debug.contains("HalfRoundedDown")
            && spell_debug.contains("CardsInLibrary"),
        "Kitsune's Technique should lower to a dynamic half-library MillEffect, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn permanent_sneak_form_compiles_with_sneak_cost() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(80_491), "Elektra, Daughter of the Hand")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text(
            "Sneak {1}{B}{B} (You may cast this spell for {1}{B}{B} if you also return an \
             unblocked attacker you control to hand during the declare blockers step. She enters \
             tapped and attacking.)\n\
             When Elektra enters, destroy target creature an opponent controls with power 3 or less.",
        )
        .expect("permanent Sneak should compile");

    let sneak = def
        .alternative_casts
        .iter()
        .find(|method| method.name().eq_ignore_ascii_case("Sneak"))
        .expect("Elektra should have a Sneak alternative cost");
    assert_eq!(
        sneak.mana_cost().map(|cost| cost.to_oracle()),
        Some("{1}{B}{B}".to_string())
    );
    assert!(
        sneak.non_mana_costs().iter().any(|cost| {
            cost.effect_ref().is_some_and(|effect| {
                effect
                    .downcast_ref::<crate::effects::SneakCostEffect>()
                    .is_some()
            })
        }),
        "permanent Sneak should keep the real return-unblocked-attacker cost"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn requested_parse_gap_cards_compile() {
    let cases = [
        (
            "Cryogen Relic",
            CardType::Artifact,
            "When this artifact enters or leaves the battlefield, draw a card.\n\
             {1}{U}, Sacrifice this artifact: Put a stun counter on up to one target tapped creature.",
        ),
        (
            "Stormchaser's Talent",
            CardType::Enchantment,
            "(Gain the next level as a sorcery to add its ability.)\n\
             When this Class enters, create a 1/1 blue and red Otter creature token with prowess.\n\
             {3}{U}: Level 2\n\
             When this Class becomes level 2, return target instant or sorcery card from your graveyard to your hand.\n\
             {5}{U}: Level 3\n\
             Whenever you cast an instant or sorcery spell, create a 1/1 blue and red Otter creature token with prowess.",
        ),
        (
            "Nowhere to Run",
            CardType::Enchantment,
            "Flash\n\
             When this enchantment enters, target creature an opponent controls gets -3/-3 until end of turn.\n\
             Creatures your opponents control can be the targets of spells and abilities as though they didn't have hexproof. Ward abilities of those creatures don't trigger.",
        ),
        (
            "Momentum Breaker",
            CardType::Enchantment,
            "Start your engines!\n\
             When this enchantment enters, each opponent sacrifices a creature or Vehicle of their choice. Each opponent who can't discards a card.\n\
             {2}, Sacrifice this enchantment: You gain life equal to your speed.",
        ),
        (
            "Fire Lord Sozin",
            CardType::Creature,
            "Menace, firebending 3 (Whenever this creature attacks, add {R}{R}{R}. This mana lasts until end of combat.)\n\
             Whenever Fire Lord Sozin deals combat damage to a player, you may pay {X}. When you do, put any number of target creature cards with total mana value X or less from that player's graveyard onto the battlefield under your control.",
        ),
        (
            "The Mind Stone",
            CardType::Artifact,
            "Indestructible\n\
             {T}: Add {W}.\n\
             {5}{W}, {T}: Harness The Mind Stone.\n\
             ∞ — At the beginning of your end step, exile up to one other target nonland permanent you control, then return that card to the battlefield under its owner's control.",
        ),
        (
            "Erode",
            CardType::Instant,
            "Destroy target creature or planeswalker. Its controller may search their library for a basic land card, put it onto the battlefield tapped, then shuffle.",
        ),
    ];

    for (name, card_type, text) in cases {
        let def = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![card_type])
            .parse_text(text)
            .unwrap_or_else(|err| panic!("{name} should parse: {err}"));
        let rendered = unprocessed_compiled_lines(&def).join(" ");
        assert!(
            !rendered.trim().is_empty(),
            "{name} should produce compiled text"
        );
        if name == "Cryogen Relic" {
            let lowered = rendered.to_ascii_lowercase();
            assert!(
                lowered.contains("enters") && lowered.contains("leaves"),
                "Cryogen Relic should retain both trigger arms, got {rendered}"
            );
        }
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn source_cost_compiled_text_does_not_leak_placeholder_surfaces() {
    let memory_jar = CardDefinitionBuilder::new(CardId::new(), "Memory Jar Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "{T}, Sacrifice this artifact: Each player exiles all cards from their hand face down and draws seven cards. At the beginning of the next end step, each player discards their hand and returns to their hand each card they exiled this way.",
        )
        .expect("Memory Jar text should parse");
    let memory_jar_text = crate::compiled_text::compiled_text_lines(&memory_jar).join(" ");
    assert!(
        memory_jar_text.contains("Sacrifice this artifact"),
        "source sacrifice cost should render with card subject, got {memory_jar_text}"
    );
    assert!(
        memory_jar_text.contains("returns to their hand each card they exiled this way"),
        "Memory Jar should keep compact oracle-shaped text, got {memory_jar_text}"
    );

    let spirit_guide = CardDefinitionBuilder::new(CardId::new(), "Spirit Guide Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Exile this card from your hand: Add {R}.")
        .expect("Spirit Guide text should parse");
    let spirit_guide_text = crate::compiled_text::compiled_text_lines(&spirit_guide).join(" ");
    assert!(
        spirit_guide_text.contains("Exile this card from your hand: Add {R}"),
        "source exile hand cost should render as a card-from-hand cost, got {spirit_guide_text}"
    );

    let awakening_zone = CardDefinitionBuilder::new(CardId::new(), "Awakening Zone Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "At the beginning of your upkeep, you may create a 0/1 colorless Eldrazi Spawn creature token. It has \"Sacrifice this token: Add {C}.\"",
        )
        .expect("Awakening Zone text should parse");
    let awakening_zone_text = crate::compiled_text::compiled_text_lines(&awakening_zone).join(" ");
    assert!(
        awakening_zone_text.contains("\"Sacrifice this token: Add {C}.\""),
        "token-granted source sacrifice cost should render as this token, got {awakening_zone_text}"
    );

    let fixed_counter_cost = CardDefinitionBuilder::new(CardId::new(), "Counter Cost Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text("{2}, {T}, Put a blood counter on this artifact: Draw a card.")
        .expect("source put-counter cost should parse");
    let fixed_counter_cost_text =
        crate::compiled_text::compiled_text_lines(&fixed_counter_cost).join(" ");
    assert!(
        fixed_counter_cost_text.contains("Put a blood counter on this artifact"),
        "source put-counter cost should render with card subject, got {fixed_counter_cost_text}"
    );

    let variable_counter_cost = CardDefinitionBuilder::new(CardId::new(), "Mana Battery Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "{2}, {T}: Put a charge counter on this artifact.\n{T}, Remove any number of charge counters from this artifact: Add {R}.",
        )
        .expect("source variable remove-counter cost should parse");
    let variable_counter_cost_text =
        crate::compiled_text::compiled_text_lines(&variable_counter_cost).join(" ");
    assert!(
        variable_counter_cost_text
            .contains("Remove any number of charge counters from this artifact"),
        "source variable remove-counter cost should render with card subject, got {variable_counter_cost_text}"
    );

    for rendered in [
        memory_jar_text,
        spirit_guide_text,
        awakening_zone_text,
        fixed_counter_cost_text,
        variable_counter_cost_text,
    ] {
        assert!(
            !rendered.contains('~'),
            "source placeholder should not leak into compiled text, got {rendered}"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn source_counter_removal_and_sacrifice_cost_renders_as_one_source_cost() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ior Ruin Expedition Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Landfall — Whenever a land you control enters, you may put a quest counter on this enchantment.\nRemove three quest counters from this enchantment and sacrifice it: Draw two cards.",
        )
        .expect("Ior Ruin Expedition text should parse");

    let rendered = crate::compiled_text::compiled_text_lines(&def).join(" ");
    assert!(
        rendered.contains("Remove three quest counters from this enchantment and sacrifice it"),
        "source counter-removal/sacrifice cost should stay oracle-shaped, got {rendered}"
    );
    assert!(
        !rendered.contains('~'),
        "source placeholder should not leak into compiled text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn source_hinted_loyalty_costs_render_as_loyalty_prefixes() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Loyalty Variant")
        .card_types(vec![CardType::Planeswalker])
        .parse_text("+1: Add {R}.\n−2: Draw a card.\n−X: Draw X cards.")
        .expect("loyalty abilities should parse");

    let rendered = crate::compiled_text::compiled_text_lines(&def).join(" ");
    assert!(
        rendered.contains("+1: Add {R}"),
        "source-hinted put-loyalty cost should render as +1, got {rendered}"
    );
    assert!(
        rendered.contains("−2: You draw a card") || rendered.contains("−2: Draw a card"),
        "source-hinted remove-loyalty cost should render as −2, got {rendered}"
    );
    assert!(
        rendered.contains("−X: You draw X cards") || rendered.contains("−X: Draw X cards"),
        "source-hinted X remove-loyalty cost should render as −X, got {rendered}"
    );
    assert!(
        !rendered.contains("loyalty counter") && !rendered.contains('~'),
        "loyalty costs should not render as counter-removal text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_each_player_discard_then_draw_keeps_each_player_scope() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Wheel Variant")
        .parse_text("Each player discards their hand, then draws seven cards.")
        .expect("each-player discard-then-draw should parse");

    let compiled = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        compiled.contains("Each player discards their hand, then draws 7 cards")
            || compiled.contains("Each player discards their hand, then draws seven cards"),
        "expected each-player scope to carry into draw clause, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn leading_then_conjugated_draw_keeps_each_player_scope() {
    let (def, trace) = ironsmith_compiler::parse_trace::capture(|| {
        CardDefinitionBuilder::new(CardId::new(), "Day Undoing Variant")
            .card_types(vec![CardType::Sorcery])
            .parse_text(
                "Each player shuffles their hand and graveyard into their library, then draws seven cards. If it's your turn, end the turn.",
            )
            .expect("leading-then conjugated draw should retain each-player scope")
    });

    let debug = format!("{:#?}", def.spell_effect);
    let compact = debug.split_whitespace().collect::<String>();
    assert!(
        debug.contains("ForPlayersEffect")
            && debug.contains("filter: Any")
            && debug.contains("ShuffleHandAndGraveyardIntoLibraryEffect")
            && debug.contains("DrawCardsEffect")
            && debug.contains("count: Fixed")
            && debug.contains("player: IteratedPlayer"),
        "shuffle and conjugated draw must share each-player scope: {debug}\ntrace:\n{}",
        trace.render()
    );
    assert!(
        debug.contains("ConditionalEffect")
            && debug.contains("EndTurnEffect")
            && debug.contains("player: You")
            && !compact.contains("DrawCardsEffect{count:Fixed(7,),player:You")
            && !compact.contains("EndTurnEffect{player:IteratedPlayer"),
        "the turn gate must apply only to your end-turn action: {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "Each player shuffles their hand and graveyard into their library, then draws 7 cards"
        ) || rendered.contains(
            "Each player shuffles their hand and graveyard into their library, then draws seven cards"
        ),
        "expected oracle-like each-player chain, got {rendered}"
    );
    assert!(
        rendered.contains("If it's your turn, end the turn")
            && !rendered.contains("draw seven cards if it's your turn")
            && !rendered.contains("each player ends the turn"),
        "conditional end-turn scope must remain separate, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_each_player_may_shuffle_hand_and_graveyard_keeps_player_scope() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Step Variant")
        .parse_text(
            "Each player may shuffle their hand and graveyard into their library. Each player who does draws seven cards.",
        )
        .expect("each-player may-shuffle clause should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("MayEffect") && debug.contains("ShuffleHandAndGraveyardIntoLibraryEffect"),
        "{debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("may shuffle their hand and graveyard into their library"),
        "{rendered}\n{debug}"
    );
    assert!(
        rendered.contains("that player draws 7 cards")
            || rendered.contains("that player draws seven cards"),
        "{rendered}"
    );
    assert!(
        !rendered.contains("if you do, draw 7 cards")
            && !rendered.contains("if you do, draw seven cards"),
        "{rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_each_player_may_search_their_library_then_shuffle_keeps_player_scope() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Noble Variant")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "When this creature dies, each player may search their library for a card and put that card into their hand. Then each player who searched their library this way shuffles.",
        )
        .expect("each-player may-search clause should parse");

    let debug = format!("{:?}", def.abilities);
    let compact_debug = debug.split_whitespace().collect::<String>();
    assert!(
        compact_debug.contains("ForPlayersEffect")
            && compact_debug.contains("MayEffect")
            && compact_debug.contains("decider:Some(IteratedPlayer")
            && compact_debug.contains("owner:Some(IteratedPlayer")
            && compact_debug.contains("chooser:IteratedPlayer")
            && compact_debug.contains("ShuffleLibraryEffect")
            && compact_debug.contains("player:IteratedPlayer"),
        "expected iterated-player search and shuffle scope, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        (rendered.contains(
            "each player may search their library for a card and put that card into their hand"
        ) || rendered.contains(
            "each player may search their library for a card, put that card into their hand"
        )) && rendered.contains("each player who searched their library this way shuffles"),
        "expected oracle-like per-player search rendering, got {rendered}"
    );
    assert!(
        !rendered.contains("you may search your library"),
        "search should not default to you, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_controller_may_search_their_library_does_not_default_to_you() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Path Variant")
        .parse_text(
            "Exile target creature. Its controller may search their library for a basic land card, put that card onto the battlefield tapped, then shuffle.",
        )
        .expect("controller may-search clause should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("owner: Some(ControllerOf"),
        "expected searched library owner to be the target object's controller, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("controller may search their library")
            && rendered.contains("battlefield tapped"),
        "expected controller-owned search rendering, got {rendered}"
    );
    assert!(
        !rendered.contains("search your library"),
        "controller search should not default to you, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_destroyed_land_controller_may_search_their_library_keeps_controller_scope() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Rampage Variant")
        .parse_text(
            "Destroy target artifact, enchantment, or land. If a land was destroyed this way, its controller may search their library for up to two basic land cards, put them onto the battlefield tapped, then shuffle. Otherwise, its controller may search their library for a basic land card, put it onto the battlefield tapped, then shuffle.",
        )
        .expect("destroyed-land controller may-search clause should parse");

    let debug = format!("{:?}", def.spell_effect);
    let compact_debug = debug.split_whitespace().collect::<String>();
    assert!(
        compact_debug.contains("owner:Some(ControllerOf(")
            && compact_debug.contains("chooser:ControllerOf("),
        "expected destroyed land's controller to own and choose the search, got {debug}"
    );
    assert!(
        !compact_debug.contains("owner:Some(You)") && !compact_debug.contains("chooser:You"),
        "controller search should not lower as a you-owned search, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("controller may search their library")
            && rendered.contains("up to two basic land cards"),
        "expected controller-owned destroyed-land search rendering, got {rendered}"
    );
    assert!(
        !rendered.contains("may have you search your library")
            && !rendered.contains("search your library"),
        "destroyed-land controller search should not default to you, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_non_outlaw_creature_filter_excludes_outlaw_subtypes() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Shoot Variant")
        .parse_text("Destroy target non-outlaw creature.")
        .expect("non-outlaw target filter should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("excluded_subtypes: [Assassin, Mercenary, Pirate, Rogue, Warlock]")
            || debug.contains("excluded_subtypes: [Assassin, Mercenary, Pirate, Rogue, Warlock,"),
        "expected outlaw subtype exclusions in parsed filter, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_for_each_object_subject_wraps_create_effect() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Predation Variant")
        .parse_text(
            "For each creature your opponents control, create a 4/4 green Beast creature token.",
        )
        .expect("for-each object create clause should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("ForEachObject"),
        "expected ForEachObject lowering, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_create_for_each_tail_wraps_create_effect() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Pack Variant")
        .parse_text("Create a 1/1 white Soldier creature token for each creature you control.")
        .expect("create-for-each tail should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("CreateTokenEffect") && debug.contains("count: Count("),
        "expected counted token creation based on controlled creatures, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_earthbend_then_untap_keeps_tail_effect() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Earthbend Variant")
        .parse_text("Earthbend 8, then untap that land.")
        .expect("earthbend with tail clause should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("EarthbendEffect") && debug.contains("UntapEffect"),
        "expected earthbend and untap effects, got {debug}"
    );
    assert!(
        debug.contains("earthbend_0"),
        "expected earthbend target tag to carry into tail untap, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_instead_if_control_keeps_prior_damage_target() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Steer Clear Variant")
        .parse_text(
            "Steer Clear deals 2 damage to target attacking or blocking creature. Steer Clear deals 4 damage to that creature instead if you controlled a Mount as you cast this spell.",
        )
        .expect("instead-if damage clause should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("deal 2 damage to target attacking or blocking creature")
            && rendered_lower.contains("it deals 4 damage instead")
            && rendered_lower.contains("if you control a mount"),
        "expected instead-if render with the original creature target, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn foul_tongue_invocation_strict_parse_regression() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Foul-Tongue Invocation")
        .parse_text(
            "As an additional cost to cast this spell, you may reveal a Dragon card from your hand.\n\
Target player sacrifices a creature of their choice. If you revealed a Dragon card or controlled a Dragon as you cast this spell, you gain 4 life.",
        )
        .expect("Foul-Tongue Invocation should parse strictly");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("target player sacrifices a creature")
            && rendered.contains(
                "if you revealed a dragon card or controlled a dragon as you cast this spell"
            )
            && rendered.contains("you gain 4 life"),
        "expected Foul-Tongue Invocation to render behold-or-control condition, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn dragon_reveal_additional_cost_payoffs_keep_authored_cast_history_surface() {
    for name in ["Draconic Roar", "Orator of Ojutai"] {
        let def = parse_oracle_card_definition(name);
        let rendered = unprocessed_compiled_lines(&def).join("\n");
        let rendered_lower = rendered.to_ascii_lowercase();
        assert!(
            rendered_lower.contains(
                "if you revealed a dragon card or controlled a dragon as you cast this spell"
            ) && !rendered_lower.contains("behold cost"),
            "expected {name} to retain the typed Dragon reveal/control cast-history condition, got {rendered}"
        );
    }
}

#[test]
pub(super) fn foul_tongue_invocation_condition_fails_without_behold_or_dragon_control() {
    let def = parse_oracle_card_definition("Foul-Tongue Invocation");
    let condition = def
        .spell_effect
        .as_ref()
        .and_then(|effects| {
            effects.iter().find_map(|effect| {
                effect
                    .downcast_ref::<crate::effects::ConditionalEffect>()
                    .map(|conditional| conditional.condition.clone())
            })
        })
        .expect("Foul-Tongue Invocation should lower a conditional life-gain clause");

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = game.create_object_from_definition(
        &def,
        crate::PlayerId::from_index(0),
        crate::zone::Zone::Stack,
    );
    let condition_matches = crate::condition_eval::evaluate_condition_cast_time(
        &game,
        &condition,
        crate::PlayerId::from_index(0),
        source,
    );

    assert!(
        !condition_matches,
        "Foul-Tongue Invocation life-gain condition should fail without a behold payment or controlled Dragon"
    );
}

#[test]
pub(super) fn foul_tongue_invocation_condition_matches_when_behold_label_was_paid() {
    let def = parse_oracle_card_definition("Foul-Tongue Invocation");
    let condition = def
        .spell_effect
        .as_ref()
        .and_then(|effects| {
            effects.iter().find_map(|effect| {
                effect
                    .downcast_ref::<crate::effects::ConditionalEffect>()
                    .map(|conditional| conditional.condition.clone())
            })
        })
        .expect("Foul-Tongue Invocation should lower a conditional life-gain clause");

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = game.create_object_from_definition(
        &def,
        crate::PlayerId::from_index(0),
        crate::zone::Zone::Stack,
    );
    game.object_mut(source)
        .expect("source spell object should exist")
        .optional_costs_paid
        .mark_label_paid("Behold Dragon");
    let condition_matches = crate::condition_eval::evaluate_condition_cast_time(
        &game,
        &condition,
        crate::PlayerId::from_index(0),
        source,
    );

    assert!(
        condition_matches,
        "Foul-Tongue Invocation life-gain condition should pass when its Behold label is marked paid"
    );
}

#[test]
pub(super) fn kaya_orzhov_usurper_strict_parse_regression() {
    let def = parse_oracle_card_definition("Kaya, Orzhov Usurper");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("exile up to two target cards")
            && rendered.contains("if it's a creature card")
            && rendered.contains("you gain 2 life"),
        "expected Kaya, Orzhov Usurper to parse with conditional life gain on the +1 ability, got {rendered}"
    );
}

#[test]
pub(super) fn kaya_orzhov_usurper_plus_one_conditional_life_gain_branch_false_without_creature_tagged()
 {
    let def = parse_oracle_card_definition("Kaya, Orzhov Usurper");
    let conditional = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated
                .effects
                .segments
                .iter()
                .flat_map(|segment| segment.default_effects.iter())
                .find_map(|effect| effect.downcast_ref::<crate::effects::ConditionalEffect>()),
            _ => None,
        })
        .expect("Kaya, Orzhov Usurper +1 should lower a conditional life-gain effect");
    let tag = match &conditional.condition {
        Condition::TaggedObjectMatches(tag, _) => tag.clone(),
        other => panic!("expected tagged-object condition for Kaya +1, got {other:?}"),
    };

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&def, alice, crate::zone::Zone::Battlefield);

    let noncreature_def = CardDefinitionBuilder::new(CardId::from_raw(777001), "Tagged Relic")
        .card_types(vec![CardType::Artifact])
        .build();
    let tagged_id =
        game.create_object_from_definition(&noncreature_def, alice, crate::zone::Zone::Exile);
    let tagged_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(tagged_id)
            .expect("tagged noncreature should exist"),
        &game,
    );

    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice).with_tagged_objects(
        std::collections::HashMap::from([(tag, vec![tagged_snapshot])]),
    );
    crate::effects::execute_effect(
        &mut game,
        &crate::effect::Effect::new(conditional.clone()),
        &mut ctx,
    )
    .expect("Kaya, Orzhov Usurper conditional +1 effect should resolve");

    assert_eq!(
        game.life_total(alice),
        20,
        "controller should not gain life when no tagged creature card was exiled"
    );
}

#[test]
pub(super) fn kaya_orzhov_usurper_plus_one_conditional_life_gain_branch_true_with_creature_tagged()
{
    let def = parse_oracle_card_definition("Kaya, Orzhov Usurper");
    let conditional = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated
                .effects
                .segments
                .iter()
                .flat_map(|segment| segment.default_effects.iter())
                .find_map(|effect| effect.downcast_ref::<crate::effects::ConditionalEffect>()),
            _ => None,
        })
        .expect("Kaya, Orzhov Usurper +1 should lower a conditional life-gain effect");
    let tag = match &conditional.condition {
        Condition::TaggedObjectMatches(tag, _) => tag.clone(),
        other => panic!("expected tagged-object condition for Kaya +1, got {other:?}"),
    };

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&def, alice, crate::zone::Zone::Battlefield);

    let creature_def = CardDefinitionBuilder::new(CardId::from_raw(777002), "Tagged Witness")
        .card_types(vec![CardType::Creature])
        .build();
    let tagged_id =
        game.create_object_from_definition(&creature_def, alice, crate::zone::Zone::Exile);
    let tagged_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(tagged_id)
            .expect("tagged creature should exist"),
        &game,
    );

    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice).with_tagged_objects(
        std::collections::HashMap::from([(tag, vec![tagged_snapshot])]),
    );
    crate::effects::execute_effect(
        &mut game,
        &crate::effect::Effect::new(conditional.clone()),
        &mut ctx,
    )
    .expect("Kaya, Orzhov Usurper conditional +1 effect should resolve");

    assert_eq!(
        game.life_total(alice),
        22,
        "controller should gain 2 life when a tagged creature card was exiled"
    );
}

#[test]
pub(super) fn kaya_orzhov_usurper_plus_one_conditional_life_gain_branch_true_with_mixed_tagged_cards()
 {
    let def = parse_oracle_card_definition("Kaya, Orzhov Usurper");
    let conditional = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated
                .effects
                .segments
                .iter()
                .flat_map(|segment| segment.default_effects.iter())
                .find_map(|effect| effect.downcast_ref::<crate::effects::ConditionalEffect>()),
            _ => None,
        })
        .expect("Kaya, Orzhov Usurper +1 should lower a conditional life-gain effect");
    let tag = match &conditional.condition {
        Condition::TaggedObjectMatches(tag, _) => tag.clone(),
        other => panic!("expected tagged-object condition for Kaya +1, got {other:?}"),
    };

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&def, alice, crate::zone::Zone::Battlefield);

    let noncreature_def = CardDefinitionBuilder::new(CardId::from_raw(777003), "Tagged Relic")
        .card_types(vec![CardType::Artifact])
        .build();
    let noncreature_id =
        game.create_object_from_definition(&noncreature_def, alice, crate::zone::Zone::Exile);
    let noncreature_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(noncreature_id)
            .expect("tagged noncreature should exist"),
        &game,
    );

    let creature_def = CardDefinitionBuilder::new(CardId::from_raw(777004), "Tagged Witness")
        .card_types(vec![CardType::Creature])
        .build();
    let creature_id =
        game.create_object_from_definition(&creature_def, alice, crate::zone::Zone::Exile);
    let creature_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(creature_id)
            .expect("tagged creature should exist"),
        &game,
    );

    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice).with_tagged_objects(
        std::collections::HashMap::from([(tag, vec![noncreature_snapshot, creature_snapshot])]),
    );
    crate::effects::execute_effect(
        &mut game,
        &crate::effect::Effect::new(conditional.clone()),
        &mut ctx,
    )
    .expect("Kaya, Orzhov Usurper conditional +1 effect should resolve");

    assert_eq!(
        game.life_total(alice),
        22,
        "controller should gain life when at least one tagged exiled card is a creature"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_instead_if_control_omitted_target_reuses_prior_damage_target() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Invasive Maneuvers Variant")
        .parse_text(
            "Invasive Maneuvers deals 3 damage to target creature. It deals 5 damage instead if you control a Spacecraft.",
        )
        .expect("instead-if followup sentence should reuse prior target");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("deal 3 damage to target creature")
            && rendered_lower.contains("deals 5 damage to that creature instead")
            && rendered_lower.contains("if you control a spacecraft"),
        "expected conditional to preserve the original creature target, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_instead_if_control_omitted_target_reuses_prior_damage_target_with_or_filter() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Chandra's Triumph Variant")
        .parse_text(
            "Chandra's Triumph deals 3 damage to target creature or planeswalker an opponent controls. Chandra's Triumph deals 5 damage instead if you control a Chandra planeswalker.",
        )
        .expect("instead-if followup sentence should reuse prior target");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower
            .contains("deal 3 damage to target creature or planeswalker an opponent controls")
            && rendered_lower.contains("deals 5 damage to that permanent instead")
            && rendered_lower.contains("if you control a chandra planeswalker"),
        "expected conditional to preserve the original creature-or-planeswalker target, got {rendered}"
    );
}

#[test]
pub(super) fn take_the_fall_strict_parse_regression() {
    let def = parse_oracle_card_definition("Take the Fall");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("target creature gets -1/-0 until end of turn")
            && rendered
                .contains("it gets -4/-0 until end of turn instead if you control an outlaw")
            && rendered.contains("draw a card"),
        "expected Take the Fall to keep its instead-if outlaw clause in compiled text, got {rendered}"
    );
}

#[test]
pub(super) fn take_the_fall_condition_fails_without_controlled_outlaw() {
    let def = parse_oracle_card_definition("Take the Fall");
    let condition = def
        .spell_effect
        .as_ref()
        .and_then(|program| {
            program.segments.iter().find_map(|segment| {
                segment
                    .self_replacements
                    .first()
                    .map(|branch| branch.condition.clone())
            })
        })
        .expect("Take the Fall should lower an outlaw self-replacement clause");

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = crate::PlayerId::from_index(0);
    let source = game.create_object_from_definition(&def, alice, crate::zone::Zone::Stack);

    let non_outlaw = CardDefinitionBuilder::new(CardId::from_raw(778001), "Calm Bear")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Bear])
        .build();
    game.create_object_from_definition(&non_outlaw, alice, crate::zone::Zone::Battlefield);

    let condition_matches =
        crate::condition_eval::evaluate_condition_cast_time(&game, &condition, alice, source);

    assert!(
        !condition_matches,
        "Take the Fall outlaw condition should fail when only non-outlaw creatures are controlled"
    );
}

#[test]
pub(super) fn take_the_fall_condition_matches_with_controlled_outlaw() {
    let def = parse_oracle_card_definition("Take the Fall");
    let condition = def
        .spell_effect
        .as_ref()
        .and_then(|program| {
            program.segments.iter().find_map(|segment| {
                segment
                    .self_replacements
                    .first()
                    .map(|branch| branch.condition.clone())
            })
        })
        .expect("Take the Fall should lower an outlaw self-replacement clause");

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = crate::PlayerId::from_index(0);
    let source = game.create_object_from_definition(&def, alice, crate::zone::Zone::Stack);

    let outlaw = CardDefinitionBuilder::new(CardId::from_raw(778002), "Sneaky Rogue")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Rogue])
        .build();
    game.create_object_from_definition(&outlaw, alice, crate::zone::Zone::Battlefield);

    let condition_matches =
        crate::condition_eval::evaluate_condition_cast_time(&game, &condition, alice, source);

    assert!(
        condition_matches,
        "Take the Fall outlaw condition should pass when an outlaw creature is controlled"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_spell_line_instead_followup_merges_into_prior_spell_effect() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Galvanic Blast Variant")
        .parse_text(
            "Galvanic Blast deals 2 damage to any target.\nMetalcraft — Galvanic Blast deals 4 damage instead if you control three or more artifacts.",
        )
        .expect("metalcraft instead followup line should merge into prior spell effect");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("deal 2 damage to any target")
            && rendered_lower.contains("deals 4 damage to that permanent or player instead")
            && rendered_lower.contains("if you control three or more artifacts"),
        "expected metalcraft line to replace prior damage amount and reuse target, got {rendered}"
    );

    let program = def.spell_effect.as_ref().expect("spell effect");
    assert_eq!(program.segments.len(), 1);
    assert_eq!(program.segments[0].default_effects.len(), 1);
    assert_eq!(program.segments[0].self_replacements.len(), 1);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_spell_line_instead_followup_merges_non_control_predicate() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Cackling Flames Variant")
        .parse_text(
            "Cackling Flames deals 3 damage to any target.\nHellbent — Cackling Flames deals 5 damage instead if you have no cards in hand.",
        )
        .expect("hellbent instead followup line should merge into prior spell effect");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("deal 3 damage to any target")
            && rendered_lower.contains("deals 5 damage to that permanent or player instead")
            && rendered_lower.contains("if you have no cards in hand"),
        "expected hellbent line to replace prior damage amount and reuse target, got {rendered}"
    );

    let program = def.spell_effect.as_ref().expect("spell effect");
    assert_eq!(program.segments.len(), 1);
    assert_eq!(program.segments[0].self_replacements.len(), 1);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn gibbering_descent_parses_hellbent_skip_upkeep_static_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Gibbering Descent")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "At the beginning of each player's upkeep, that player loses 1 life and discards a card.\n\
             Hellbent — Skip your upkeep step if you have no cards in hand.\n\
             Madness {2}{B}{B} (If you discard this card, discard it into exile. When you do, cast it for its madness cost or put it into your graveyard.)",
        )
        .expect("Gibbering Descent should parse strictly");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "At the beginning of each player's upkeep, that player loses 1 life and discards a card"
        ),
        "expected compact lose-life-and-discard trigger text, got {rendered}"
    );
    assert!(
        rendered.contains("Hellbent — Skip your upkeep step if you have no cards in hand"),
        "expected hellbent skip-upkeep clause in compiled text, got {rendered}"
    );
    assert!(
        def.abilities.iter().any(|ability| {
            matches!(&ability.kind, AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::PlayersSkipUpkeep)
        }),
        "expected Gibbering Descent to compile a skip-upkeep static ability, got {:?}",
        def.abilities
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_cabal_ritual_threshold_instead_compiles_to_self_replacement_branch() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Cabal Ritual Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Add {B}{B}{B}. Threshold — Add {B}{B}{B}{B}{B} instead if there are seven or more cards in your graveyard.",
        )
        .expect("cabal ritual threshold instead should parse");

    let program = def.spell_effect.as_ref().expect("spell effect");
    assert_eq!(program.segments.len(), 1);
    assert_eq!(program.segments[0].default_effects.len(), 1);
    assert_eq!(program.segments[0].self_replacements.len(), 1);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_triggered_instead_followup_preserves_default_branch() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Aerith Trigger Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "At the beginning of your end step, return target creature card from your graveyard to your hand. If you gained 7 or more life this turn, return that card to the battlefield instead.",
        )
        .expect("triggered instead followup should parse");

    let ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");
    assert_eq!(ability.effects.segments.len(), 1);
    assert_eq!(ability.effects.segments[0].self_replacements.len(), 1);

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_lowercase();
    assert!(
        rendered_lower.contains("if you gained 7 or more life this turn")
            && rendered_lower.contains("return that card to the battlefield")
            && rendered_lower
                .contains("return target creature card from your graveyard to your hand"),
        "expected rendered trigger to keep both default and replacement branches, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_landfall_instead_followup_preserves_default_branch() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Akoum Hellkite Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Landfall — Whenever a land you control enters, this creature deals 1 damage to any target. If that land is a Mountain, this creature deals 2 damage instead.",
        )
        .expect("landfall instead followup should parse");

    let ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");
    assert_eq!(ability.effects.segments.len(), 1);
    assert_eq!(ability.effects.segments[0].self_replacements.len(), 1);
    assert!(matches!(
        ability.effects.segments[0].self_replacements[0].condition,
        Condition::TaggedObjectMatches(ref tag, _)
            if tag.as_str() == "triggering"
    ));

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("deals 1 damage to any target")
            && rendered.contains("deals 2 damage instead"),
        "expected rendered trigger to keep both damage branches, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_triggered_instead_followup_with_toxic_condition_preserves_default_branch() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Porcelain Zealot Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "At the beginning of combat on your turn, target creature you control gets +1/+1 until end of turn. If that creature has toxic, instead it gets +2/+2 until end of turn.",
        )
        .expect("toxic instead followup should parse");

    let ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");
    assert_eq!(ability.effects.segments.len(), 1);
    assert_eq!(ability.effects.segments[0].self_replacements.len(), 1);

    let rendered = unprocessed_compiled_lines(&def).join(" ").to_lowercase();
    assert!(
        rendered.contains("gets +1/+1 until end of turn")
            && rendered.contains("gets +2/+2 until end of turn"),
        "expected rendered trigger to keep both pump branches, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_triggered_instead_followup_with_creatures_died_count_preserves_default_branch()
{
    let def = CardDefinitionBuilder::new(CardId::new(), "Tallyman Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "At the beginning of your end step, if a creature died this turn, you draw a card and you lose 1 life. If seven or more creatures died this turn, instead you draw seven cards and you lose 7 life.",
        )
        .expect("died-count instead followup should parse");

    let ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");
    assert_eq!(ability.effects.segments.len(), 1);
    assert_eq!(ability.effects.segments[0].self_replacements.len(), 1);
    assert!(matches!(
        ability.effects.segments[0].self_replacements[0].condition,
        Condition::CreatureDiedThisTurnOrMore(7)
    ));

    let rendered = unprocessed_compiled_lines(&def).join(" ").to_lowercase();
    assert!(
        rendered.contains("draw a card")
            && rendered.contains("lose 1 life")
            && rendered.contains("draw seven cards")
            && rendered.contains("lose 7 life"),
        "expected rendered trigger to keep both tallyman branches, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_triggered_instead_followup_with_full_party_preserves_default_branch() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Destined Warrior Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "At the beginning of combat on your turn, creatures you control get +1/+0 until end of turn. If you have a full party, creatures you control get +3/+0 until end of turn instead.",
        )
        .expect("full-party instead followup should parse");

    let ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");
    assert_eq!(ability.effects.segments.len(), 1);
    assert_eq!(ability.effects.segments[0].self_replacements.len(), 1);
    assert!(matches!(
        ability.effects.segments[0].self_replacements[0].condition,
        Condition::YouHaveFullParty
    ));

    let rendered = unprocessed_compiled_lines(&def).join(" ").to_lowercase();
    assert!(
        rendered.contains("get +1/+0 until end of turn")
            && rendered.contains("get +3/+0 until end of turn"),
        "expected rendered trigger to keep both party branches, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_instead_followup_without_prior_spell_segment_fails_loudly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Broken Self-Replacement Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("If you control an artifact, draw two cards instead.")
        .expect_err("unanchored self-replacement followup should fail");

    assert!(matches!(err, CardTextError::UnsupportedLine(_)));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_deal_damage_with_trailing_if_clause_emits_conditional() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Kami's Flare Variant")
        .parse_text(
            "Kami's Flare deals 3 damage to target creature or planeswalker. Kami's Flare also deals 2 damage to that permanent's controller if you control a modified creature. (Equipment, Auras you control, and counters are modifications.)",
        )
        .expect("trailing if control clause should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("deal 3 damage to target creature or planeswalker")
            && rendered_lower.contains("if you control a modified creature")
            && (rendered_lower.contains("damage to that object's controller")
                || rendered_lower.contains("damage to that permanent's controller")),
        "expected conditional damage followup, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_damage_to_that_creatures_controller_targets_player() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Chandra Variant")
        .parse_text(
            "Chandra's Outrage deals 4 damage to target creature and 2 damage to that creature's controller.",
        )
        .expect("damage to that creature's controller should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("that object's controller")
            || rendered.contains("that creature's controller")
            || rendered.contains("that permanent's controller"),
        "expected controller-target damage wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_dingus_egg_keeps_the_source_and_controller_linked() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dingus Egg")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Whenever a land is put into a graveyard from the battlefield, this artifact deals 2 damage to that land's controller.",
        )
        .expect("Dingus Egg should parse");

    assert_eq!(def.abilities.len(), 1);
    let ability = &def.abilities[0];
    let AbilityKind::Triggered(triggered) = &ability.kind else {
        panic!("expected triggered ability");
    };

    let effects_debug = format!("{:#?}", triggered.effects);
    let trigger_debug = format!("{:#?}", triggered.trigger);
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    let canonical = crate::compiled_text::canonical_compiled_lines(&def).join(" ");
    assert!(
        (rendered.contains("whenever a land is put into a graveyard from the battlefield")
            || rendered.contains("whenever a land dies"))
            && (rendered.contains("deal 2 damage to that object's controller")
                || rendered.contains("deal 2 damage to that land's controller")
                || rendered.contains("deals 2 damage to that object's controller")
                || rendered.contains("deals 2 damage to that land's controller")),
        "expected Dingus Egg to keep the damage clause attached, got {rendered}"
    );
    assert!(
        trigger_debug.contains("ZoneChangeTrigger")
            || trigger_debug.contains("PutIntoGraveyardFromZone")
            || trigger_debug.contains("to: Graveyard"),
        "expected Dingus Egg to lower into a battlefield-to-graveyard trigger, got {trigger_debug}"
    );
    assert!(
        effects_debug.contains("DealDamageEffect") || effects_debug.contains("DealDamage"),
        "expected Dingus Egg to lower into a damage effect, got {effects_debug}"
    );
    assert_eq!(
        canonical,
        "Whenever a land is put into a graveyard from the battlefield, this artifact deals 2 damage to that land's controller."
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn dingus_egg_deals_damage_to_the_land_controller_on_graveyard_entry() {
    let dingus_egg = CardDefinitionBuilder::new(CardId::new(), "Dingus Egg")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Whenever a land is put into a graveyard from the battlefield, this artifact deals 2 damage to that land's controller.",
        )
        .expect("Dingus Egg should parse");
    let land = CardDefinitionBuilder::new(CardId::new(), "Test Land")
        .card_types(vec![CardType::Land])
        .build();

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let egg_id =
        game.create_object_from_definition(&dingus_egg, alice, crate::zone::Zone::Battlefield);
    let land_id = game.create_object_from_definition(&land, bob, crate::zone::Zone::Battlefield);
    let snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(land_id).expect("land should exist"),
        &game,
    );
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_cause(
            land_id,
            crate::zone::Zone::Battlefield,
            crate::zone::Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
            Some(snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    )
    .with_lookback_source_snapshots(game.trigger_source_lookback_snapshots());

    let triggered = crate::triggers::check_triggers(&game, &event);
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    for entry in triggered.into_iter().filter(|entry| entry.source == egg_id) {
        trigger_queue.add(entry);
    }
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "expected Dingus Egg to trigger once when a land dies"
    );

    crate::game_loop::put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Dingus Egg trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game).expect("Dingus Egg trigger should resolve");

    assert_eq!(
        game.life_total(bob),
        18,
        "the land's controller should take 2 damage"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn profane_memento_compiled_text_and_trigger_model_regression() {
    let def = parse_oracle_card_definition("Profane Memento");
    let rendered = canonical_compiled_lines(&def).join(" ");
    let rendered_lc = rendered.to_ascii_lowercase();
    assert!(
        rendered_lc
            .contains("whenever a creature card is put into an opponent's graveyard from anywhere"),
        "expected opponent-owned nontoken creature graveyard trigger text, got {rendered}"
    );
    assert!(
        rendered_lc.contains("you gain 1 life"),
        "expected life gain effect text, got {rendered}"
    );

    let trigger = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .trigger
                .downcast_ref::<crate::triggers::ZoneChangeTrigger>(
            ),
            _ => None,
        })
        .expect("Profane Memento should compile to a zone-change trigger");

    assert_eq!(
        trigger.player,
        crate::triggers::zone_changes::PlayerRelation::Any
    );
    assert_eq!(
        trigger.object_filter.owner,
        Some(PlayerFilter::Opponent),
        "triggered card should be owned by an opponent"
    );
    assert_eq!(trigger.object_filter.card_types, vec![CardType::Creature]);
    assert!(
        trigger.object_filter.nontoken,
        "creature card trigger must exclude tokens"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn profane_memento_triggers_for_opponents_creature_cards_only() {
    let profane_memento = parse_oracle_card_definition("Profane Memento");
    let vanilla_creature = CardDefinitionBuilder::new(CardId::new(), "Test Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let memento_id =
        game.create_object_from_definition(&profane_memento, alice, crate::zone::Zone::Battlefield);

    let bob_hand_creature = game.create_object_from_definition(&vanilla_creature, bob, Zone::Hand);
    let bob_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(bob_hand_creature)
            .expect("opponent creature should exist"),
        &game,
    );
    let bob_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_cause(
            bob_hand_creature,
            Zone::Hand,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
            Some(bob_snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    );

    let triggered = crate::triggers::check_triggers(&game, &bob_event);
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    for entry in triggered
        .into_iter()
        .filter(|entry| entry.source == memento_id)
    {
        trigger_queue.add(entry);
    }
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Profane Memento should trigger when an opponent's creature card goes to their graveyard"
    );

    crate::game_loop::put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Profane Memento trigger should go on stack");
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Profane Memento trigger should resolve");
    assert_eq!(game.life_total(alice), 21, "controller should gain 1 life");

    let alice_library_creature =
        game.create_object_from_definition(&vanilla_creature, alice, Zone::Library);
    let alice_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(alice_library_creature)
            .expect("controller creature should exist"),
        &game,
    );
    let alice_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_cause(
            alice_library_creature,
            Zone::Library,
            Zone::Graveyard,
            crate::events::cause::EventCause::effect(),
            Some(alice_snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    );

    let triggered = crate::triggers::check_triggers(&game, &alice_event);
    assert!(
        triggered.iter().all(|entry| entry.source != memento_id),
        "Profane Memento must not trigger for your own creature cards"
    );
    assert_eq!(
        game.life_total(alice),
        21,
        "life total should be unchanged when trigger condition is not met"
    );
}

#[test]
pub(super) fn burn_the_accursed_regression_uses_oracle_like_damage_and_die_replacement_text() {
    let def = parse_oracle_card_definition("Burn the Accursed");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "deal 5 damage to target creature and 2 damage to that creature's controller"
        ) && rendered.contains("if that creature would die this turn, exile it instead"),
        "expected Burn the Accursed to keep its linked damage and die-replacement wording, got {rendered}"
    );
}

#[test]
pub(super) fn gloomshrieker_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Gloomshrieker");
    let def = parse_oracle_card_definition("Gloomshrieker");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let lower = rendered.to_ascii_lowercase();
    let abilities_debug = format!("{:#?}", def.abilities);

    assert!(
        lower.contains("menace"),
        "expected Gloomshrieker menace text, got {rendered}"
    );
    assert!(
        lower.contains(
            "when this creature enters, return target permanent card from your graveyard to your hand"
        ),
        "expected Gloomshrieker ETB return text, got {rendered}"
    );
    assert!(
        lower.contains("if this creature would die, exile it instead"),
        "expected self death replacement text, got {rendered}"
    );
    assert!(
        abilities_debug.contains("ExileWouldDieInstead")
            && abilities_debug.contains("source: true")
            && abilities_debug.contains("Creature"),
        "expected source-scoped creature death replacement, got {abilities_debug}"
    );
}

#[test]
pub(super) fn gloomshrieker_enters_returns_target_permanent_card_from_your_graveyard_to_hand() {
    let def = parse_oracle_card_definition("Gloomshrieker");
    let nonpermanent = CardDefinitionBuilder::new(CardId::new(), "Gloomshrieker Test Instant")
        .card_types(vec![CardType::Instant])
        .build();
    let permanent = CardDefinitionBuilder::new(CardId::new(), "Gloomshrieker Test Artifact")
        .card_types(vec![CardType::Artifact])
        .build();

    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    game.create_object_from_definition(&nonpermanent, alice, Zone::Graveyard);
    game.create_object_from_definition(&permanent, alice, Zone::Graveyard);
    let gloom_in_hand = game.create_object_from_definition(&def, alice, Zone::Hand);
    let gloom_snapshot =
        crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(gloom_in_hand)
                .expect("Gloomshrieker should be in hand before entering"),
            &game,
        );
    let gloom = game
        .move_object_with_etb_processing(gloom_in_hand, Zone::Battlefield)
        .expect("Gloomshrieker should enter")
        .new_id;

    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::ZoneChangeEvent::with_results(
            gloom,
            vec![gloom],
            Zone::Hand,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            Some(gloom_snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let triggered = crate::triggers::check_triggers(&game, &event);
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    for entry in triggered.into_iter().filter(|entry| entry.source == gloom) {
        trigger_queue.add(entry);
    }
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Gloomshrieker should trigger once when it enters"
    );

    let mut dm = crate::decision::SelectFirstDecisionMaker;
    crate::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("Gloomshrieker trigger should go on the stack with a legal target");
    crate::game_loop::resolve_stack_entry(&mut game).expect("Gloomshrieker trigger should resolve");

    let hand_names: Vec<String> = game
        .objects_in_zone(Zone::Hand)
        .into_iter()
        .filter_map(|id| game.object(id).map(|object| object.name.to_string()))
        .collect();
    let graveyard_names: Vec<String> = game
        .objects_in_zone(Zone::Graveyard)
        .into_iter()
        .filter_map(|id| game.object(id).map(|object| object.name.to_string()))
        .collect();

    assert!(
        hand_names
            .iter()
            .any(|name| name == "Gloomshrieker Test Artifact"),
        "the permanent card target should move to hand, hand={hand_names:?}"
    );
    assert!(
        graveyard_names
            .iter()
            .any(|name| name == "Gloomshrieker Test Instant"),
        "the nonpermanent card should not be a legal target, graveyard={graveyard_names:?}"
    );
}

#[test]
pub(super) fn gloomshrieker_death_replacement_exiles_only_gloomshrieker() {
    let def = parse_oracle_card_definition("Gloomshrieker");
    let decoy_def = CardDefinitionBuilder::new(CardId::new(), "Gloomshrieker Test Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let gloom = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let decoy = game.create_object_from_definition(&decoy_def, alice, Zone::Battlefield);
    game.update_replacement_effects();

    let decoy_destination = crate::events::processing::process_zone_change_with_event(
        &mut game,
        decoy,
        Zone::Battlefield,
        Zone::Graveyard,
        crate::events::cause::EventCause::from_sba(),
    )
    .expect("decoy death should not be prevented");
    assert_eq!(
        decoy_destination,
        Zone::Graveyard,
        "Gloomshrieker's replacement must not affect other creatures"
    );
    game.move_object_by_sba(decoy, decoy_destination)
        .expect("decoy should move to graveyard");

    let gloom_destination = crate::events::processing::process_zone_change_with_event(
        &mut game,
        gloom,
        Zone::Battlefield,
        Zone::Graveyard,
        crate::events::cause::EventCause::from_sba(),
    )
    .expect("Gloomshrieker death should not be prevented");
    assert_eq!(
        gloom_destination,
        Zone::Exile,
        "Gloomshrieker should be exiled instead of going to the graveyard"
    );
    game.move_object_by_sba(gloom, gloom_destination)
        .expect("Gloomshrieker should move to exile");

    let exile_names: Vec<String> = game
        .objects_in_zone(Zone::Exile)
        .into_iter()
        .filter_map(|id| game.object(id).map(|object| object.name.to_string()))
        .collect();
    let graveyard_names: Vec<String> = game
        .objects_in_zone(Zone::Graveyard)
        .into_iter()
        .filter_map(|id| game.object(id).map(|object| object.name.to_string()))
        .collect();

    assert!(
        exile_names.iter().any(|name| name == "Gloomshrieker"),
        "Gloomshrieker should end in exile, exile={exile_names:?}"
    );
    assert!(
        graveyard_names
            .iter()
            .any(|name| name == "Gloomshrieker Test Bear"),
        "the decoy creature should end in the graveyard, graveyard={graveyard_names:?}"
    );
    assert!(
        !graveyard_names.iter().any(|name| name == "Gloomshrieker"),
        "Gloomshrieker should not be in the graveyard, graveyard={graveyard_names:?}"
    );
}

#[test]
pub(super) fn moira_and_teshar_regression_keeps_leave_battlefield_instead_marker() {
    let def = parse_oracle_card_definition("Moira and Teshar");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("if it would leave the battlefield, exile it instead"),
        "expected leave-battlefield replacement text to keep 'instead', got {rendered}"
    );
}

#[test]
pub(super) fn isareth_lowers_returned_creature_leave_replacement_to_persistent_engine_effect() {
    let def = parse_oracle_card_definition("Isareth the Awakener");
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Isareth should have an attack trigger");
    let reflexive = triggered
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ReflexiveTriggerEffect>())
        .expect("Isareth's when-you-do clause should lower as a reflexive trigger");
    let replacement = reflexive
        .effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::RegisterZoneReplacementEffect>())
        .expect("the leave-battlefield replacement must resolve inside the reflexive trigger");
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert_eq!(replacement.from_zone, Some(Zone::Battlefield));
    assert_eq!(replacement.to_zone, None);
    assert_eq!(replacement.replacement_zone, Zone::Exile);
    assert_eq!(
        replacement.mode,
        crate::effects::ReplacementApplyMode::Resolution
    );
    assert!(
        matches!(&replacement.target, ChooseSpec::Tagged(tag)
            if tag.as_str().starts_with("moved_") || tag.as_str().starts_with("returned_")),
        "expected Isareth's persistent replacement to follow the object returned inside the reflexive trigger, got {:?}",
        replacement.target
    );
    assert!(
        rendered
            .to_ascii_lowercase()
            .contains("would leave the battlefield, exile it instead of putting it anywhere else"),
        "expected Isareth's leave-battlefield replacement to render from the engine effect, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn mana_ability_render_uses_colon_separator() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mana Separator Variant")
        .card_types(vec![CardType::Land])
        .parse_text("{T}: Add {W}.")
        .expect("basic mana ability should parse");

    let line = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        line.contains("{T}: Add {W}") && !line.contains("{T}, Add {W}"),
        "expected colon-separated mana text, got {line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn metadata_land_dual_mana_line_stays_a_mana_ability() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Tundra Variant")
        .parse_text("Type: Land\n{T}: Add {W} or {U}.")
        .expect("metadata-driven dual mana land should parse");

    let line = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        line.contains("{T}: Add {W} or {U}"),
        "expected dual mana output to stay a mana ability, got {line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn metadata_basic_typed_dual_land_mana_line_stays_a_mana_ability() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Tundra")
        .parse_text("Type: Land — Plains Island\n{T}: Add {W} or {U}.")
        .expect("typed dual land should parse");

    let lines = unprocessed_compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains("{T}: Add {W} or {U}"),
        "expected typed dual land output to stay a mana ability, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_reveal_hand_clause_with_trailing_effect_keeps_tagged_same_name_condition() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Retraced Image Variant")
        .parse_text(
            "Reveal a card in your hand, then put that card onto the battlefield if it has the same name as a permanent.",
        )
        .expect("reveal-hand clause and its conditional follow-up should parse");
    let debug = format!("{def:?}");
    assert!(
        debug.contains("ChooseObjectsEffect")
            && debug.contains("RevealTaggedEffect")
            && debug.contains("ConditionalEffect")
            && debug.contains("TaggedObjectMatches")
            && debug.contains("SameNameAsTagged")
            && debug.contains("MoveToZoneEffect"),
        "expected a tagged reveal followed by a typed same-name battlefield move, got {debug}"
    );
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "reveal it, then put it onto the battlefield if it has the same name as a permanent"
        ),
        "expected the supported reveal and conditional move to render, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_reveal_hand_clause_with_colon_tail_fails_strictly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Sasaya Variant")
        .parse_text(
            "Reveal your hand: If you have seven or more land cards in your hand, flip Sasaya.",
        )
        .expect_err("partial reveal-hand parsing should fail");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported reveal-hand clause"),
        "expected strict reveal-hand parse error, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_reveal_any_number_of_cards_in_your_hand_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Scent Variant")
        .parse_text("Reveal any number of red cards in your hand.")
        .expect("reveal-any-number-in-hand clause should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("ChooseObjectsEffect") && debug.contains("zone: Some(Hand)"),
        "expected choose-from-hand reveal setup, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_reveal_x_cards_in_your_hand_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Nightshade Assassin Variant")
        .parse_text("Reveal X black cards in your hand.")
        .expect("reveal-x-in-hand clause should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("ChooseObjectsEffect") && debug.contains("zone: Some(Hand)"),
        "expected x-count choose-from-hand reveal setup, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_reveal_single_card_in_your_hand_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Assembly Hall Variant")
        .parse_text("Reveal a creature card in your hand.")
        .expect("reveal-single-card-in-hand clause should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("ChooseObjectsEffect") && debug.contains("zone: Some(Hand)"),
        "expected single-card choose-from-hand reveal setup, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_reveal_top_plural_cards_clause() {
    CardDefinitionBuilder::new(CardId::new(), "Top Reveal Variant")
        .parse_text("Reveal the top five cards of your library.")
        .expect("reveal-top plural cards clause should parse");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_reveal_top_card_clause_without_library_suffix() {
    CardDefinitionBuilder::new(CardId::new(), "Top Card Reveal Variant")
        .parse_text("Reveal the top card.")
        .expect("reveal top-card shorthand should parse");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_reveal_top_card_then_lose_life_followup() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dark Confidant Variant")
        .parse_text(
            "At the beginning of your upkeep, reveal the top card of your library and put that card into your hand. You lose life equal to its mana value.",
        )
        .expect("dark confidant-style reveal followup should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("reveal the top card of your library")
            && (rendered.contains("lose life equal to its mana value")
                || rendered.contains("lose life equal to that card's mana value")),
        "expected reveal and life-loss followup, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_discard_up_to_two_then_draw_that_many() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Tersa Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, discard up to two cards, then draw that many cards.",
        )
        .expect("discard-up-to-two then draw-that-many should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("Discard")
            && debug.contains("Fixed(2)")
            && (debug.contains("EventValue(Amount)")
                || debug.contains("EffectValue(EffectId(")
                || (debug.contains("EffectMetric") && debug.contains("metric: Count"))),
        "expected discard-count and draw-that-many lowering, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
fn processor_activation_compiled_text(name: &str, text: &str) -> String {
    let def = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .parse_text(text)
        .expect("processor activation should parse");
    crate::compiled_text::unprocessed_compiled_lines(&def).join("\n")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cryptic_cruiser_preserves_processor_activation_cost_surface() {
    let rendered = processor_activation_compiled_text(
        "Cryptic Cruiser",
        "Devoid\n{2}{U}, Put a card an opponent owns from exile into that player's graveyard: Tap target creature.",
    );
    assert!(
        rendered.contains(
            "{2}{U}, Put a card an opponent owns from exile into that player's graveyard: Tap target creature"
        ),
        "expected Cryptic Cruiser's processor cost, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn oracle_of_dust_preserves_processor_activation_cost_surface() {
    let rendered = processor_activation_compiled_text(
        "Oracle of Dust",
        "Devoid\n{2}, Put a card an opponent owns from exile into that player's graveyard: Draw a card, then discard a card.",
    );
    assert!(
        rendered.contains(
            "{2}, Put a card an opponent owns from exile into that player's graveyard: Draw a card, then discard a card"
        ),
        "expected Oracle of Dust's processor cost, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn void_attendant_preserves_processor_activation_cost_surface() {
    let rendered = processor_activation_compiled_text(
        "Void Attendant",
        "Devoid\n{1}{G}, Put a card an opponent owns from exile into that player's graveyard: Create a 1/1 colorless Eldrazi Scion creature token. It has \"Sacrifice this token: Add {C}.\"",
    );
    assert!(
        rendered.contains(
            "{1}{G}, Put a card an opponent owns from exile into that player's graveyard: Create a 1/1 colorless Eldrazi Scion creature token"
        ),
        "expected Void Attendant's processor cost, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
fn count_scaled_pump_compiled_text(
    name: &str,
    card_types: Vec<CardType>,
    subtypes: Vec<Subtype>,
    text: &str,
) -> String {
    let def = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(card_types)
        .subtypes(subtypes)
        .parse_text(text)
        .expect("count-scaled pump should parse");
    crate::compiled_text::unprocessed_compiled_lines(&def).join("\n")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn empyrial_armor_preserves_plus_one_for_each_hand_card_surface() {
    let rendered = count_scaled_pump_compiled_text(
        "Empyrial Armor",
        vec![CardType::Enchantment],
        vec![Subtype::Aura],
        "Enchant creature\nEnchanted creature gets +1/+1 for each card in your hand.",
    );
    assert_eq!(
        rendered,
        "Enchant creature\nEnchanted creature gets +1/+1 for each card in your hand."
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn empyrial_plate_preserves_plus_one_for_each_hand_card_surface() {
    let rendered = count_scaled_pump_compiled_text(
        "Empyrial Plate",
        vec![CardType::Artifact],
        vec![Subtype::Equipment],
        "Equipped creature gets +1/+1 for each card in your hand.\nEquip {2}",
    );
    assert_eq!(
        rendered,
        "Equipped creature gets +1/+1 for each card in your hand.\nEquip {2}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn gran_pulse_ochu_preserves_plus_one_for_each_graveyard_permanent_surface() {
    let rendered = count_scaled_pump_compiled_text(
        "Gran Pulse Ochu",
        vec![CardType::Creature],
        vec![],
        "Deathtouch\n{8}: Until end of turn, this creature gets +1/+1 for each permanent card in your graveyard.",
    );
    assert_eq!(
        rendered,
        "Deathtouch\n{8}: This creature gets +1/+1 until end of turn for each permanent card in your graveyard."
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
fn adventure_creature_spell_trigger_compiled_text(name: &str, text: &str) -> String {
    let def = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .parse_text(text)
        .expect("Adventure creature spell trigger should parse");
    crate::compiled_text::unprocessed_compiled_lines(&def).join("\n")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn edgewall_innkeeper_uses_rules_characteristic_adventure_surface() {
    let rendered = adventure_creature_spell_trigger_compiled_text(
        "Edgewall Innkeeper",
        "Whenever you cast a creature spell that has an Adventure, draw a card.",
    );
    assert_eq!(
        rendered,
        "Whenever you cast a creature spell that has an Adventure, draw a card."
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn garenbrig_squire_uses_rules_characteristic_adventure_surface() {
    let rendered = adventure_creature_spell_trigger_compiled_text(
        "Garenbrig Squire",
        "Whenever you cast a creature spell that has an Adventure, this creature gets +1/+1 until end of turn.",
    );
    assert_eq!(
        rendered,
        "Whenever you cast a creature spell that has an Adventure, this creature gets +1/+1 until end of turn."
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn wandermare_uses_rules_characteristic_adventure_surface() {
    let rendered = adventure_creature_spell_trigger_compiled_text(
        "Wandermare",
        "Whenever you cast a creature spell that has an Adventure, put a +1/+1 counter on this creature.",
    );
    assert_eq!(
        rendered,
        "Whenever you cast a creature spell that has an Adventure, put a +1/+1 counter on this creature."
    );
}
