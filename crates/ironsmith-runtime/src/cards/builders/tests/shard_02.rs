#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
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
use super::shard_18::*;
use super::shard_19::*;
use super::shard_20::*;
use super::shard_21::*;
use super::shard_22::*;
use super::shard_23::*;
use super::*;

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn shiny_impetus_stops_buffing_and_goading_when_aura_leaves_battlefield() {
    let shiny_impetus = parse_oracle_card_definition("Shiny Impetus");
    let creature = CardDefinitionBuilder::new(CardId::from_raw(91_123), "Grizzly Bears")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = crate::game_state::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );

    let enchanted_creature = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
    let aura = game.create_object_from_definition(&shiny_impetus, alice, Zone::Battlefield);
    assert!(game.attach_object_to_target(
        aura,
        crate::object::AttachmentTarget::Object(enchanted_creature),
    ));

    assert_eq!(game.current_power(enchanted_creature), Some(4));
    assert_eq!(game.current_toughness(enchanted_creature), Some(4));
    assert!(game.is_goaded(enchanted_creature));

    game.move_object_by_effect(aura, Zone::Graveyard)
        .expect("Shiny Impetus should move to graveyard");

    assert_eq!(game.current_power(enchanted_creature), Some(2));
    assert_eq!(game.current_toughness(enchanted_creature), Some(2));
    assert!(
        !game.is_goaded(enchanted_creature),
        "Shiny Impetus should stop goading after it leaves the battlefield"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn shiny_impetus_creates_treasure_when_enchanted_creature_attacks() {
    let shiny_impetus = parse_oracle_card_definition("Shiny Impetus");
    let creature = CardDefinitionBuilder::new(CardId::from_raw(91_122), "Grizzly Bears")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let mut game = crate::game_state::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );

    let enchanted_creature = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
    let aura = game.create_object_from_definition(&shiny_impetus, alice, Zone::Battlefield);
    assert!(game.attach_object_to_target(
        aura,
        crate::object::AttachmentTarget::Object(enchanted_creature),
    ));
    game.remove_summoning_sickness(enchanted_creature);
    game.turn.active_player = bob;

    let mut combat = crate::combat_state::CombatState::default();
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    let attack = [crate::AttackerDeclaration {
        creature: enchanted_creature,
        target: crate::combat_state::AttackTarget::Player(charlie),
    }];
    crate::game_loop::apply_attacker_declarations(
        &mut game,
        &mut combat,
        &mut trigger_queue,
        &attack,
    )
    .expect("enchanted creature should be able to attack a non-goading player");
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Shiny Impetus should queue exactly one attack trigger"
    );
    crate::game_loop::put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Shiny Impetus attack trigger should go on the stack");
    assert_eq!(
        game.stack.len(),
        1,
        "Shiny Impetus should create exactly one attack trigger"
    );

    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Shiny Impetus attack trigger should resolve");
    let treasure_count = game
        .battlefield
        .iter()
        .filter(|&&id| {
            game.object(id).is_some_and(|object| {
                object.name == "Treasure" && game.controller_of(object) == alice
            })
        })
        .count();
    assert_eq!(
        treasure_count, 1,
        "Shiny Impetus should create one Treasure controlled by the Aura controller"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn shiny_impetus_allows_attacking_aura_controller_when_no_other_player_is_attackable() {
    let shiny_impetus = parse_oracle_card_definition("Shiny Impetus");
    let creature = CardDefinitionBuilder::new(CardId::from_raw(91_121), "Grizzly Bears")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let mut game = crate::game_state::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );

    let enchanted_creature = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
    let aura = game.create_object_from_definition(&shiny_impetus, alice, Zone::Battlefield);
    assert!(game.attach_object_to_target(
        aura,
        crate::object::AttachmentTarget::Object(enchanted_creature),
    ));
    game.remove_summoning_sickness(enchanted_creature);
    game.effect_store
        .cant_effects
        .add_cant_attack_defenders(enchanted_creature, [charlie]);
    game.turn.active_player = bob;

    let combat = crate::combat_state::CombatState::default();
    let options = crate::decision::compute_legal_attackers(&game, &combat);
    let attacker_option = options
        .iter()
        .find(|option| option.creature == enchanted_creature)
        .expect("enchanted creature should still be able to attack the Aura controller");
    assert!(
        attacker_option.must_attack,
        "goad should still require attacking if only the goading player can be attacked"
    );
    assert_eq!(
        attacker_option.valid_targets,
        vec![crate::combat_state::AttackTarget::Player(alice)],
        "when no non-goading player is attackable, goad should allow attacking the Aura controller"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_lantern_of_insight_public_top_library_static() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Lantern of Insight Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Players play with the top card of their libraries revealed.\n{T}, Sacrifice this artifact: Target player shuffles.",
        )
        .expect("Lantern of Insight text should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("AllPlayersLookAtTopCardsOfLibraries"),
        "expected public top-library static ability, got {debug}"
    );
    assert!(
        debug.contains("ShuffleLibraryEffect"),
        "expected target-player shuffle activation, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Players play with the top card of their libraries revealed")
            && rendered.contains("Target player shuffles"),
        "expected Lantern oracle wording to survive compilation, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn fae_offering_keeps_both_spell_gate_and_token_bundle() {
    let line = "At the beginning of each end step, if you've cast both a creature spell and a noncreature spell this turn, create a Clue token, a Food token, and a Treasure token.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Fae Offering Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(line)
        .expect("Fae Offering text should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("And(")
            && debug.contains("card_types: [Creature]")
            && debug.contains("excluded_card_types: [Creature]"),
        "expected separate creature and noncreature spell-cast gates, got {debug}"
    );
    assert_eq!(
        unprocessed_compiled_lines(&def),
        vec![line.to_string()],
        "expected Fae Offering oracle wording to survive compilation"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sphinx_ambassador_keeps_search_name_choice_condition() {
    let triggered = "Whenever this creature deals combat damage to a player, search that player's library for a card, then that player chooses a card name. If you searched for a creature card that doesn't have that name, you may put it onto the battlefield under your control. Then that player shuffles.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Sphinx Ambassador Variant")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Sphinx])
        .power_toughness(PowerToughness::fixed(5, 5))
        .parse_text(&format!("Flying\n{triggered}"))
        .expect("Sphinx Ambassador search/name sequence should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ChooseCardNameEffect") && debug.contains("SameNameAsTagged"),
        "expected name choice and chosen-name condition, got {debug}"
    );

    let lines = unprocessed_compiled_lines(&def);
    assert_eq!(
        lines,
        vec!["Flying".to_string(), triggered.to_string()],
        "expected oracle-style Sphinx wording to survive compilation"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn emet_selch_keeps_graveyard_cost_and_life_loss_may_cast_trigger() {
    let triggered = "Whenever one or more opponents lose life, you may cast target instant or sorcery card from your graveyard. If that spell would be put into your graveyard, exile it instead.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Emet-Selch of the Third Seat Variant")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elder, Subtype::Wizard])
        .power_toughness(PowerToughness::fixed(3, 4))
        .parse_text(&format!(
            "Spells you cast from your graveyard cost {{2}} less to cast.\n{triggered} Do this only once each turn."
        ))
        .expect("Emet-Selch graveyard cast trigger should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("PlayerLosesLife") && debug.contains("CastTaggedEffect"),
        "expected opponent life-loss trigger with cast-tagged effect, got {debug}"
    );
    assert!(
        debug.contains("RegisterFutureZoneReplacementEffect"),
        "expected one-shot graveyard-to-exile replacement for cast spell, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("Spells you cast from your graveyard cost {2} less to cast")
            && rendered.contains(triggered),
        "expected Emet-Selch oracle wording to survive compilation, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn pantlaza_keeps_do_this_only_once_each_turn_condition() {
    let oracle = "Whenever Pantlaza or another Dinosaur you control enters, you may discover X, where X is that creature's toughness. Do this only once each turn.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Pantlaza, Sun-Favored")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Dinosaur])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(oracle)
        .expect("Pantlaza trigger should parse");

    let debug = format!("{:#?}", def.abilities);
    let has_do_this_cap = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Triggered(triggered)
                if triggered.intervening_if
                    == Some(crate::ConditionExpr::DoThisMaxTimesEachTurn(1))
        )
    });
    assert!(
        has_do_this_cap,
        "expected Pantlaza to keep a do-this-once-per-turn condition, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("Do this only once each turn"),
        "expected Pantlaza scored text to preserve the do-this suffix, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn eye_of_doom_keeps_doom_counter_choice_and_destroy_filter() {
    let lines = [
        "When this artifact enters, each player chooses a nonland permanent and puts a doom counter on it.",
        "{2}, {T}, Sacrifice this artifact: Destroy each permanent with a doom counter on it.",
    ];
    let def = CardDefinitionBuilder::new(CardId::new(), "Eye of Doom Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(&lines.join("\n"))
        .expect("Eye of Doom counter choice/destroy text should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ForPlayersEffect") && debug.contains("PutCountersEffect"),
        "expected each-player counter placement, got {debug}"
    );
    assert!(
        debug.contains("with_counter: Some(Typed(Named(\"doom\")))"),
        "expected destroy filter to require a doom counter, got {debug}"
    );

    assert_eq!(
        unprocessed_compiled_lines(&def),
        vec![lines[0].to_string(), lines[1].to_string()],
        "expected Eye of Doom oracle wording to survive compilation"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn dream_thiefs_bandana_keeps_look_exile_and_while_exiled_permission() {
    let trigger = "Whenever equipped creature deals combat damage to a player, look at the top card of their library, then exile it face down. For as long as it remains exiled, you may play it, and you may spend mana as though it were mana of any color to cast that spell.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Dream-Thief's Bandana Variant")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .parse_text(&format!("{trigger}\nEquip {{1}}"))
        .expect("Dream-Thief's Bandana text should parse");

    let debug = format!("{:#?}", def.abilities);
    assert!(debug.contains("LookAtTopCardsEffect"), "{debug}");
    assert!(debug.contains("ExileEffect"), "{debug}");
    assert!(debug.contains("face_down: true"), "{debug}");
    assert!(debug.contains("GrantPlayTaggedEffect"), "{debug}");
    assert!(debug.contains("ForAsLongAsExiled"), "{debug}");
    assert!(debug.contains("allow_any_color_for_cast: true"), "{debug}");
    assert_eq!(
        unprocessed_compiled_lines(&def),
        vec![trigger.to_string(), "Equip {1}".to_string()],
        "expected Dream-Thief's Bandana oracle wording to survive compilation"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cogwork_librarian_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Cogwork Librarian");

    assert_eq!(
        canonical_compiled_lines(&def),
        vec![
            "Draft this card face up.".to_string(),
            "As you draft a card, you may draft an additional card from that booster pack. If you do, put this card into that booster pack."
                .to_string(),
        ],
        "Cogwork Librarian should preserve its draft face-up and booster-pack clauses"
    );
    assert!(
        def.abilities.iter().all(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => {
                static_ability.id() == StaticAbilityId::DraftRuleText
            }
            _ => false,
        }),
        "Cogwork Librarian's draft-only text should compile as static rule text, got {:#?}",
        def.abilities
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cogwork_librarian_draft_rules_do_not_create_runtime_effects() {
    let oracle = oracle_text_by_name()
        .get("Cogwork Librarian")
        .expect("Cogwork Librarian oracle text should be available")
        .clone();
    let def = CardDefinitionBuilder::new(CardId::new(), "Cogwork Librarian")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Construct])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text(oracle)
        .expect("Cogwork Librarian should parse strictly");

    let alice = PlayerId::from_index(0);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let librarian = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    assert_eq!(game.current_power(librarian), Some(3));
    assert_eq!(game.current_toughness(librarian), Some(3));
    assert!(game.object_has_card_type(librarian, CardType::Artifact));
    assert!(game.object_has_card_type(librarian, CardType::Creature));

    let draft_static_abilities = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(draft_static_abilities.len(), 2);
    for ability in draft_static_abilities {
        assert_eq!(ability.id(), StaticAbilityId::DraftRuleText);
        assert!(ability.generate_effects(librarian, alice, &game).is_empty());
        assert!(
            ability
                .generate_replacement_effect(librarian, alice)
                .is_none()
        );
        assert!(ability.pregame_action_kind().is_none());
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn pyretic_hunter_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Pyretic Hunter");

    assert_eq!(
        canonical_compiled_lines(&def),
        vec![
            "Reveal this card as you draft it and note how many cards you've drafted this draft round, including this card."
                .to_string(),
            "Menace".to_string(),
            "This creature enters with X +1/+1 counters on it, where X is the highest number you noted for cards named Pyretic Hunter."
                .to_string(),
        ],
        "Pyretic Hunter should preserve its draft note and noted-number counter clause"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn pyretic_hunter_draft_note_and_counter_value_are_structural() {
    let def = parse_oracle_card_definition("Pyretic Hunter");
    let debug = format!("{:#?}", def.abilities);

    assert!(
        def.abilities.iter().any(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => {
                static_ability.id() == StaticAbilityId::DraftRuleText
            }
            _ => false,
        }),
        "Pyretic Hunter should compile the reveal-as-you-draft line as draft-only rule text, got {debug}"
    );
    assert!(
        debug.contains("DraftNotedHighestNumber") && debug.contains("pyretic hunter"),
        "Pyretic Hunter should model the highest noted number as a structured counter value, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn pyretic_hunter_without_tracked_draft_notes_enters_with_zero_counters() {
    let oracle = oracle_text_by_name()
        .get("Pyretic Hunter")
        .expect("Pyretic Hunter oracle text should be available")
        .clone();
    let def = CardDefinitionBuilder::new(CardId::new(), "Pyretic Hunter")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elemental, Subtype::Cat])
        .power_toughness(PowerToughness::fixed(0, 0))
        .parse_text(oracle)
        .expect("Pyretic Hunter should parse strictly");

    let alice = PlayerId::from_index(0);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let hunter_in_hand = game.create_object_from_definition(&def, alice, Zone::Hand);
    let hunter = game
        .move_object_with_etb_processing(hunter_in_hand, Zone::Battlefield)
        .expect("Pyretic Hunter should enter")
        .new_id;
    let hunter_obj = game.object(hunter).expect("Pyretic Hunter should exist");

    assert_eq!(
        hunter_obj
            .counters
            .get(&crate::object::CounterType::PlusOnePlusOne)
            .copied()
            .unwrap_or(0),
        0,
        "without tracked draft-note state, Pyretic Hunter should enter with zero +1/+1 counters"
    );
    assert_eq!(game.current_power(hunter), Some(0));
    assert_eq!(game.current_toughness(hunter), Some(0));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn pyretic_hunter_uses_tracked_highest_draft_note_for_entering_counters() {
    let oracle = oracle_text_by_name()
        .get("Pyretic Hunter")
        .expect("Pyretic Hunter oracle text should be available")
        .clone();
    let def = CardDefinitionBuilder::new(CardId::new(), "Pyretic Hunter")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elemental, Subtype::Cat])
        .power_toughness(PowerToughness::fixed(0, 0))
        .parse_text(oracle)
        .expect("Pyretic Hunter should parse strictly");

    let alice = PlayerId::from_index(0);
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    game.set_draft_noted_highest_number(alice, "Pyretic Hunter", 4);
    let hunter_in_hand = game.create_object_from_definition(&def, alice, Zone::Hand);
    let hunter = game
        .move_object_with_etb_processing(hunter_in_hand, Zone::Battlefield)
        .expect("Pyretic Hunter should enter")
        .new_id;
    let hunter_obj = game.object(hunter).expect("Pyretic Hunter should exist");

    assert_eq!(
        hunter_obj
            .counters
            .get(&crate::object::CounterType::PlusOnePlusOne)
            .copied()
            .unwrap_or(0),
        4,
        "Pyretic Hunter should enter with the highest noted number of +1/+1 counters"
    );
    assert_eq!(game.current_power(hunter), Some(4));
    assert_eq!(game.current_toughness(hunter), Some(4));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn firkraag_cunning_instigator_strict_parser_and_text_regression() {
    let def = parse_oracle_card_definition("Firkraag, Cunning Instigator");
    let rendered = canonical_compiled_lines(&def).join("\n");

    assert!(
        rendered.contains(
            "Whenever one or more Dragons you control attack an opponent, goad target creature that player controls."
        ),
        "Firkraag should bind the attacked opponent for the goad target, got {rendered}"
    );
    assert!(
        rendered.contains("if that creature had to attack this combat"),
        "Firkraag should preserve its combat-damage intervening-if condition, got {rendered}"
    );

    let debug = format!("{:#?}", def.abilities);
    let debug_compact = debug.split_whitespace().collect::<String>();
    assert!(
        debug_compact.contains("AttacksTrigger")
            && debug_compact
                .contains("attacking_player_or_planeswalker_controlled_by:Some(Opponent,)")
            && debug_compact.contains("controller:Some(Defending,)"),
        "Firkraag attack trigger should structurally bind the defending opponent, got {debug}"
    );
    assert!(
        debug.contains("TriggeringObjectHadToAttackThisCombat"),
        "Firkraag combat-damage trigger should carry the had-to-attack condition, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn happily_ever_after_keeps_life_draw_and_win_gate() {
    let lines = [
        "When this enchantment enters, each player gains 5 life and draws a card.",
        "At the beginning of your upkeep, if there are five colors among permanents you control, there are six or more card types among permanents you control and/or cards in your graveyard, and your life total is greater than or equal to your starting life total, you win the game.",
    ];
    let def = CardDefinitionBuilder::new(CardId::new(), "Happily Ever After Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(&lines.join("\n"))
        .expect("Happily Ever After text should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("intervening_if: Some")
            && debug.contains("ColorsAmong")
            && debug.contains("CardTypesAmong")
            && debug.contains("StartingLifeTotal"),
        "expected Happily Ever After's upkeep trigger to keep the full modeled gate, got {debug}"
    );

    assert_eq!(
        unprocessed_compiled_lines(&def),
        vec![lines[0].to_string(), lines[1].to_string()],
        "expected Happily Ever After oracle wording to survive compilation"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sigardas_splendor_strict_parser_compiled_text_and_model_regression() {
    assert_oracle_card_parses_strict("Sigarda's Splendor");

    let def = parse_oracle_card_definition("Sigarda's Splendor");
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    let debug = format!("{def:?}");

    assert!(
        rendered.contains("As this enchantment enters, note your life total."),
        "expected as-enters note text, got {rendered}"
    );
    assert!(
        rendered.contains(
            "if your life total is greater than or equal to the last noted life total for this enchantment"
        ) && rendered.contains("Note your life total."),
        "expected conditional upkeep draw plus note update, got {rendered}"
    );
    assert!(
        rendered.contains("Whenever you cast a white spell, you gain 1 life."),
        "expected white spell life-gain trigger text, got {rendered}"
    );
    assert!(
        debug.contains("NoteLifeTotalAsEnters")
            && debug.contains("NoteLifeTotalEffect")
            && debug.contains("LastNotedLifeTotal")
            && debug.contains("ConditionalEffect"),
        "expected Sigarda's Splendor to model note, compare, conditional draw, and note update, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sigardas_splendor_notes_life_and_draws_conditionally_at_runtime() {
    fn sigarda_upkeep_trigger(def: &CardDefinition) -> &crate::ability::TriggeredAbility {
        def.abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered)
                    if triggered
                        .trigger
                        .display()
                        .to_ascii_lowercase()
                        .contains("upkeep") =>
                {
                    Some(triggered)
                }
                _ => None,
            })
            .expect("Sigarda's Splendor should have an upkeep trigger")
    }

    fn add_library_card(game: &mut crate::game_state::GameState, player: PlayerId, name: &str) {
        let filler = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_definition(&filler, player, Zone::Library);
    }

    fn resolve_upkeep_trigger(
        game: &mut crate::game_state::GameState,
        source: ObjectId,
        controller: PlayerId,
        triggered: &crate::ability::TriggeredAbility,
    ) {
        let mut ctx = crate::effects::ExecutionContext::new_default(source, controller);
        for effect in triggered.effects.flattened_default_effects() {
            crate::effects::execute_effect(game, effect, &mut ctx)
                .expect("Sigarda's Splendor upkeep trigger should resolve");
        }
    }

    let def = parse_oracle_card_definition("Sigarda's Splendor");
    let upkeep = sigarda_upkeep_trigger(&def);
    let alice = PlayerId::from_index(0);

    let mut equal_game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let equal_sigarda = equal_game.create_object_from_definition(&def, alice, Zone::Battlefield);
    add_library_card(&mut equal_game, alice, "Sigarda Equal Branch Draw");
    assert_eq!(
        equal_game.noted_life_total_for_source(equal_sigarda),
        Some(20),
        "as-enters ability should note the controller's current life total"
    );

    resolve_upkeep_trigger(&mut equal_game, equal_sigarda, alice, upkeep);
    assert_eq!(
        equal_game.player(alice).expect("alice exists").hand.len(),
        1,
        "upkeep trigger should draw when life is at least the noted life total"
    );
    assert_eq!(
        equal_game.noted_life_total_for_source(equal_sigarda),
        Some(20),
        "upkeep trigger should note the current life total after resolving"
    );

    let mut lower_game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let lower_sigarda = lower_game.create_object_from_definition(&def, alice, Zone::Battlefield);
    add_library_card(&mut lower_game, alice, "Sigarda Lower Branch Draw");
    lower_game.lose_life(alice, 1);

    resolve_upkeep_trigger(&mut lower_game, lower_sigarda, alice, upkeep);
    assert_eq!(
        lower_game.player(alice).expect("alice exists").hand.len(),
        0,
        "upkeep trigger should not draw when life is below the noted life total"
    );
    assert_eq!(
        lower_game.noted_life_total_for_source(lower_sigarda),
        Some(19),
        "upkeep trigger should still update the noted life total when the draw branch is false"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sigardas_splendor_gains_life_when_controller_casts_white_spell() {
    fn spell_cast_event(spell: ObjectId, caster: PlayerId) -> crate::triggers::TriggerEvent {
        crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::spells::SpellCastEvent::new(spell, caster, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        )
    }

    let def = parse_oracle_card_definition("Sigarda's Splendor");
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let sigarda_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    let blue_spell = CardDefinitionBuilder::new(CardId::new(), "Blue Spell")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Instant])
        .build();
    let blue_spell_id = game.create_object_from_definition(&blue_spell, alice, Zone::Stack);
    let blue_event = spell_cast_event(blue_spell_id, alice);
    assert!(
        crate::triggers::check_triggers(&game, &blue_event)
            .into_iter()
            .all(|entry| entry.source != sigarda_id),
        "Sigarda's Splendor should not trigger for nonwhite spells"
    );

    let white_spell = CardDefinitionBuilder::new(CardId::new(), "White Spell")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
        .card_types(vec![CardType::Instant])
        .build();
    let white_spell_id = game.create_object_from_definition(&white_spell, alice, Zone::Stack);
    let white_event = spell_cast_event(white_spell_id, alice);
    let triggered = crate::triggers::check_triggers(&game, &white_event);
    let entry = triggered
        .iter()
        .find(|entry| entry.source == sigarda_id)
        .expect("Sigarda's Splendor should trigger when its controller casts a white spell");

    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(sigarda_id, alice, &mut dm)
        .with_triggering_event(entry.triggering_event.clone());
    for effect in &entry.ability.effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Sigarda's Splendor white-spell trigger should resolve");
    }

    assert_eq!(
        game.player(alice).expect("alice exists").life,
        21,
        "controller should gain 1 life from Sigarda's Splendor's white-spell trigger"
    );
}

#[test]
pub(super) fn test_creature_with_etb() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "ETB Creature")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .with_etb(vec![Effect::draw(1)])
        .build();

    assert_eq!(def.abilities.len(), 1);
    let ability = &def.abilities[0];
    // Check that the trigger is an ETB trigger (now using Trigger struct)
    if let AbilityKind::Triggered(t) = &ability.kind {
        assert!(t.trigger.display().contains("enters"));
    } else {
        panic!("Expected triggered ability");
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_deep_gnome_terramancer_ignores_played_lands() {
    let deep_gnome = CardDefinitionBuilder::new(CardId::from_raw(1), "Deep Gnome Terramancer")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Gnome, Subtype::Wizard])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "Flash\nMold Earth — Whenever one or more lands enter under an opponent's control without being played, you may search your library for a Plains card, put it onto the battlefield tapped, then shuffle. Do this only once each turn.",
        )
        .expect("deep gnome terramancer text should parse");

    let abilities_debug = format!("{:?}", deep_gnome.abilities);
    assert!(
        abilities_debug.contains("Flash"),
        "expected flash to remain present, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("cause_filter: Some")
            && abilities_debug.contains("Not(SpecialAction)"),
        "expected cause restriction in compiled trigger, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("count_mode: OneOrMore"),
        "expected one-or-more trigger count mode, got {abilities_debug}"
    );

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let gnome_id =
        game.create_object_from_definition(&deep_gnome, alice, crate::zone::Zone::Battlefield);

    let land_def = CardDefinitionBuilder::new(CardId::from_raw(2), "Test Land")
        .card_types(vec![CardType::Land])
        .build();
    let land_id =
        game.create_object_from_definition(&land_def, bob, crate::zone::Zone::Battlefield);

    let effect_event = crate::events::RawEvent::new(
        crate::events::ZoneChangeEvent::with_cause(
            land_id,
            crate::zone::Zone::Hand,
            crate::zone::Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            None,
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let effect_triggered = crate::triggers::check_triggers(&game, &effect_event);
    assert_eq!(
        effect_triggered
            .iter()
            .filter(|entry| entry.source == gnome_id)
            .count(),
        1,
        "expected Deep Gnome Terramancer to trigger for a land entering without being played"
    );

    let played_event = crate::events::RawEvent::new(
        crate::events::ZoneChangeEvent::with_cause(
            land_id,
            crate::zone::Zone::Hand,
            crate::zone::Zone::Battlefield,
            crate::events::cause::EventCause::from_special_action(Some(land_id), bob),
            None,
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let played_triggered = crate::triggers::check_triggers(&game, &played_event);
    assert!(
        played_triggered
            .iter()
            .all(|entry| entry.source != gnome_id),
        "expected Deep Gnome Terramancer not to trigger for a played land, got {played_triggered:#?}"
    );
}

#[test]
pub(super) fn test_protection_from_color() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Protected")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .protection_from(ColorSet::from(Color::Red))
        .build();

    assert_eq!(def.abilities.len(), 1);
}

#[test]
pub(super) fn test_land_with_mana_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Forest")
        .supertypes(vec![Supertype::Basic])
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Forest])
        .taps_for(ManaSymbol::Green)
        .build();

    assert!(def.card.is_land());
    assert_eq!(def.abilities.len(), 1);
    assert!(def.abilities[0].is_mana_ability());
}

#[test]
pub(super) fn test_complex_creature() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Complex Creature")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Vampire])
        .power_toughness(PowerToughness::fixed(2, 3))
        .flying()
        .deathtouch()
        .lifelink()
        .build();

    assert_eq!(def.abilities.len(), 3);
    assert!(def.is_creature());
}

#[test]
pub(super) fn test_builder_mentor_creates_targeted_attack_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mentor Test")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .mentor()
        .build();

    assert_eq!(def.abilities.len(), 1);
    let ability = &def.abilities[0];
    match &ability.kind {
        AbilityKind::Triggered(triggered) => {
            assert!(triggered.trigger.display().contains("attacks"));
            assert_eq!(triggered.choices.len(), 1);
            let choices_debug = format!("{:?}", triggered.choices);
            assert!(
                choices_debug.contains("attacking: true")
                    && choices_debug.contains("power_relative_to_source: Some(LessThanSource)"),
                "expected mentor target restriction, got {choices_debug}"
            );
        }
        _ => panic!("expected triggered ability"),
    }
}

#[test]
pub(super) fn test_builder_afterlife_creates_dies_trigger_with_tokens() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Afterlife Test")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .afterlife(2)
        .build();

    assert_eq!(def.abilities.len(), 1);
    let ability = &def.abilities[0];
    match &ability.kind {
        AbilityKind::Triggered(triggered) => {
            assert!(triggered.trigger.display().contains("dies"));
            let effects_debug = format!("{:?}", triggered.effects);
            assert!(
                effects_debug.contains("CreateTokenEffect"),
                "expected token creation effect, got {effects_debug}"
            );
        }
        _ => panic!("expected triggered ability"),
    }
}

#[test]
pub(super) fn test_builder_fabricate_creates_etb_modal_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Fabricate Test")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .fabricate(1)
        .build();

    assert_eq!(def.abilities.len(), 1);
    let ability = &def.abilities[0];
    match &ability.kind {
        AbilityKind::Triggered(triggered) => {
            assert!(triggered.trigger.display().contains("enters"));
            let effects_debug = format!("{:?}", triggered.effects);
            assert!(
                effects_debug.contains("ChooseModeEffect"),
                "expected modal fabricate effect, got {effects_debug}"
            );
        }
        _ => panic!("expected triggered ability"),
    }
}

#[test]
pub(super) fn test_builder_soulshift_creates_dies_trigger_with_graveyard_target() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Soulshift Test")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .soulshift(3)
        .build();

    assert_eq!(def.abilities.len(), 1);
    let ability = &def.abilities[0];
    match &ability.kind {
        AbilityKind::Triggered(triggered) => {
            assert!(triggered.trigger.display().contains("dies"));
            assert_eq!(triggered.choices.len(), 1);
            let debug = format!("{:?}", triggered.effects);
            assert!(
                debug.contains("ReturnFromGraveyardToHandEffect"),
                "expected soulshift recursion effect, got {debug}"
            );
        }
        _ => panic!("expected triggered soulshift ability"),
    }
}

#[test]
pub(super) fn test_builder_outlast_creates_sorcery_speed_activated_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Outlast Test")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .outlast(ManaCost::from_symbols(vec![ManaSymbol::White]))
        .build();

    assert_eq!(def.abilities.len(), 1);
    let ability = &def.abilities[0];
    match &ability.kind {
        AbilityKind::Activated(activated) => {
            assert_eq!(
                activated.timing,
                crate::ability::ActivationTiming::SorcerySpeed
            );
            let cost_text = activated.mana_cost.display().to_ascii_lowercase();
            assert!(
                cost_text.contains("{w}") && cost_text.contains("{t}"),
                "expected outlast mana+tap cost, got {cost_text}"
            );
            let debug = format!("{:?}", activated.effects);
            assert!(
                debug.contains("PutCountersEffect"),
                "expected +1/+1 counter effect, got {debug}"
            );
        }
        _ => panic!("expected activated outlast ability"),
    }
}

#[test]
pub(super) fn test_builder_extort_creates_spell_cast_trigger_with_optional_payment() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Extort Test")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .extort()
        .build();

    assert_eq!(def.abilities.len(), 1);
    let ability = &def.abilities[0];
    match &ability.kind {
        AbilityKind::Triggered(triggered) => {
            assert!(triggered.trigger.display().contains("you cast"));
            let debug = format!("{:?}", triggered.effects);
            assert!(
                debug.contains("PayManaEffect"),
                "expected extort payment effect, got {debug}"
            );
            assert!(
                debug.contains("ForPlayersEffect"),
                "expected extort opponent-drain loop, got {debug}"
            );
        }
        _ => panic!("expected triggered extort ability"),
    }
}

#[test]
pub(super) fn test_builder_partner_creates_keyword_static_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Partner Test")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .partner()
        .build();

    assert_eq!(def.abilities.len(), 1);
    let ability = &def.abilities[0];
    match &ability.kind {
        AbilityKind::Static(static_ability) => {
            assert_eq!(static_ability.id(), StaticAbilityId::Partner);
        }
        _ => panic!("expected static partner ability"),
    }
}

#[test]
pub(super) fn test_builder_assist_creates_keyword_static_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Assist Test")
        .card_types(vec![CardType::Sorcery])
        .assist()
        .build();

    assert_eq!(def.abilities.len(), 1);
    let ability = &def.abilities[0];
    match &ability.kind {
        AbilityKind::Static(static_ability) => {
            assert_eq!(static_ability.id(), StaticAbilityId::Assist);
        }
        _ => panic!("expected static assist ability"),
    }
}

#[test]
pub(super) fn test_builder_modular_creates_enters_counter_and_dies_transfer() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Modular Test")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .modular(2)
        .build();

    assert_eq!(def.abilities.len(), 2);
    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("EntersWithCounters"),
        "expected enters-with-counters ability, got {debug}"
    );
    assert!(
        debug.contains("ZoneChangeTrigger") && debug.contains("PutCountersEffect"),
        "expected dies transfer trigger for modular, got {debug}"
    );
}

#[test]
pub(super) fn test_builder_graft_creates_enters_counter_and_etb_move_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Graft Test")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(0, 0))
        .graft(2)
        .build();

    assert_eq!(def.abilities.len(), 2);
    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("EntersWithCounters"),
        "expected enters-with-counters ability, got {debug}"
    );
    assert!(
        debug.contains("ZoneChangeTrigger") && debug.contains("MoveCountersEffect"),
        "expected graft move-counter trigger, got {debug}"
    );
}

#[test]
pub(super) fn test_builder_sunburst_creature_uses_plus_one_counters() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sunburst Creature Test")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(0, 0))
        .sunburst()
        .build();

    assert_eq!(def.abilities.len(), 2);
    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("EntersWithCounters"),
        "expected enters-with-counters replacement, got {debug}"
    );
    assert!(
        debug.contains("ColorsOfManaSpentToCastThisSpell"),
        "expected sunburst to scale from colors spent to cast, got {debug}"
    );
    assert!(
        debug.contains("PlusOnePlusOne"),
        "expected creature sunburst to use +1/+1 counters, got {debug}"
    );
}

#[test]
pub(super) fn test_builder_sunburst_noncreature_uses_charge_counters() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sunburst Artifact Test")
        .card_types(vec![CardType::Artifact])
        .sunburst()
        .build();

    assert_eq!(def.abilities.len(), 2);
    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("EntersWithCounters"),
        "expected enters-with-counters replacement, got {debug}"
    );
    assert!(
        debug.contains("ColorsOfManaSpentToCastThisSpell"),
        "expected sunburst to scale from colors spent to cast, got {debug}"
    );
    assert!(
        debug.contains("Charge"),
        "expected noncreature sunburst to use charge counters, got {debug}"
    );
}

#[test]
pub(super) fn test_builder_arcbound_wanderer_modular_sunburst_renders_full_semantics() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Arcbound Wanderer")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(0, 0))
        .modular_sunburst()
        .build();

    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("modular—sunburst"),
        "expected modular sunburst keyword wording, got {rendered}"
    );
    assert!(
        !rendered.contains("modular_triggering_object"),
        "expected modular sunburst render to hide internal trigger tags, got {rendered}"
    );

    let modular_target = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered.choices.iter().find_map(|choice| {
                let ChooseSpec::Target(inner) = choice else {
                    return None;
                };
                let ChooseSpec::Object(filter) = inner.as_ref() else {
                    return None;
                };
                Some(filter)
            }),
            _ => None,
        })
        .expect("modular sunburst death trigger should target an artifact creature");
    assert!(
        modular_target.all_card_types.contains(&CardType::Artifact)
            && modular_target.all_card_types.contains(&CardType::Creature),
        "expected modular sunburst death trigger to target artifact creatures, got {modular_target:?}"
    );
}

#[test]
pub(super) fn parser_treats_artifacts_and_or_creatures_as_type_union_in_dies_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Seer Test")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 3))
        .parse_text(
            "Whenever one or more artifacts and/or creatures you control are put into a graveyard from the battlefield, surveil 1.",
        )
        .expect("parse artifacts-and/or-creatures dies trigger");

    let filter = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .trigger
                .downcast_ref::<crate::triggers::ZoneChangeTrigger>()
                .map(|trigger| &trigger.object_filter),
            _ => None,
        })
        .expect("expected a zone-change trigger");

    assert_eq!(
        filter.card_types,
        vec![CardType::Artifact, CardType::Creature]
    );
    assert!(
        filter.all_card_types.is_empty(),
        "artifacts and/or creatures should match either type, got {filter:?}"
    );
}

#[test]
pub(super) fn test_builder_fading_creates_counter_upkeep_and_sacrifice_triggers() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Fading Test")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .fading(2)
        .build();

    assert_eq!(def.abilities.len(), 3);
    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("EntersWithCounters") && debug.contains("Fade"),
        "expected fading ETB fade counters, got {debug}"
    );
    assert!(
        debug.contains("BeginningOfUpkeepTrigger") && debug.contains("RemoveCountersEffect"),
        "expected fading upkeep counter removal trigger, got {debug}"
    );
    assert!(
        debug.contains("CounterRemovedFromTrigger")
            && debug.contains("SourceHasNoCounter(Fade)")
            && debug.contains("SacrificeTargetEffect"),
        "expected fading last-counter sacrifice trigger, got {debug}"
    );
}

#[test]
pub(super) fn parse_tangle_wire_strictly_and_renders_fade_counter_tap_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(3_694), "Tangle Wire")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
        .card_types(vec![CardType::Artifact])
        .parse_text(concat!(
            "Fading 4 (This artifact enters with four fade counters on it. ",
            "At the beginning of your upkeep, remove a fade counter from it. ",
            "If you can't, sacrifice it.)\n",
            "At the beginning of each player's upkeep, that player taps an untapped artifact, ",
            "creature, or land they control for each fade counter on this artifact."
        ))
        .expect("Tangle Wire should parse strictly");

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("Fading 4"),
        "expected fading keyword line, got {rendered}"
    );
    assert!(
        rendered.contains(concat!(
            "At the beginning of each player's upkeep, that player taps an untapped artifact, ",
            "creature, or land they control for each fade counter on this artifact"
        )),
        "expected exact Tangle Wire dynamic tap clause, got {rendered}"
    );

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("WithCountValue")
            && debug.contains("Fade")
            && debug.contains("ThisPermanentType")
            && debug.contains("this artifact")
            && debug.contains("BeginningOfUpkeepTrigger"),
        "expected Tangle Wire to lower to an upkeep tap choice counted by fade counters on this artifact, got {debug}"
    );
}

#[test]
pub(super) fn test_builder_vanishing_creates_counter_upkeep_and_sacrifice_triggers() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Vanishing Test")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .vanishing(3)
        .build();

    assert_eq!(def.abilities.len(), 3);
    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("EntersWithCounters") && debug.contains("Time"),
        "expected vanishing ETB time counters, got {debug}"
    );
    assert!(
        debug.contains("BeginningOfUpkeepTrigger") && debug.contains("RemoveCountersEffect"),
        "expected vanishing upkeep counter removal trigger, got {debug}"
    );
    assert!(
        debug.contains("CounterRemovedFromTrigger")
            && debug.contains("SourceHasNoCounter(Time)")
            && debug.contains("SacrificeTargetEffect"),
        "expected vanishing last-counter sacrifice trigger, got {debug}"
    );
}

#[test]
pub(super) fn test_builder_devour_creates_etb_triggered_effect_without_marker_fallback() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Devour Test")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .devour(2)
        .build();

    assert_eq!(def.abilities.len(), 1);
    let ability = &def.abilities[0];
    match &ability.kind {
        AbilityKind::Triggered(triggered) => {
            assert!(triggered.trigger.display().contains("enters"));
            let debug = format!("{:?}", triggered.effects);
            assert!(
                debug.contains("DevourEffect"),
                "expected explicit devour runtime effect, got {debug}"
            );
            assert!(
                !debug.contains("KeywordMarker"),
                "devour should not lower to a keyword marker, got {debug}"
            );
        }
        _ => panic!("expected triggered devour ability"),
    }
}

#[test]
pub(super) fn test_builder_bloodthirst_creates_real_static_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bloodthirst Test")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .bloodthirst(3)
        .build();

    assert_eq!(def.abilities.len(), 1);
    let ability = &def.abilities[0];
    let debug = format!("{ability:?}");
    assert!(
        debug.contains("Bloodthirst"),
        "expected bloodthirst static ability, got {debug}"
    );
    assert!(
        !debug.contains("KeywordMarker"),
        "bloodthirst should not lower to a keyword marker, got {debug}"
    );
}

#[test]
pub(super) fn test_builder_rampage_preserves_keyword_marker_and_runtime_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Rampage Test")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 5))
        .rampage(4)
        .build();

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("KeywordMarker") && debug.contains("rampage 4"),
        "expected rampage keyword marker to survive in compiled definition, got {debug}"
    );
    assert!(
        debug.contains("ThisBecomesBlockedTrigger")
            && debug.contains("BlockersBeyondFirst")
            && debug.contains("multiplier: 4"),
        "expected rampage runtime trigger/effect to survive with marker, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_soulshift_keyword_line_compiles_keyword_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Soulshift Parse Test")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Soulshift 2 (When this creature dies, you may return target Spirit card with mana value 2 or less from your graveyard to your hand.)",
        )
        .expect("soulshift keyword line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Soulshift 2"),
        "expected soulshift keyword render, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_outlast_keyword_line_compiles_keyword_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Outlast Parse Test")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Outlast {W} ({W}, {T}: Put a +1/+1 counter on this creature. Activate only as a sorcery.)",
        )
        .expect("outlast keyword line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Outlast {W}"),
        "expected outlast keyword render, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_devour_keyword_line_compiles_without_unsupported_marker() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Devour Parse Test")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Devour 2 (As this creature enters, you may sacrifice any number of creatures. This creature enters with twice that many +1/+1 counters on it.)",
        )
        .expect("devour keyword line should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    let debug = format!("{def:?}");
    assert!(
        rendered.contains("devour 2"),
        "expected devour keyword render, got {rendered}"
    );
    assert!(
        !debug.contains("KeywordMarker")
            && !debug.contains("RuleFallbackText")
            && !debug.contains("UnsupportedParserLine"),
        "devour parse should avoid unsupported placeholders, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_afflict_keyword_line_compiles_keyword_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Afflict Parse Test")
        .card_types(vec![CardType::Creature])
        .parse_text("Afflict 1")
        .expect("afflict keyword line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Afflict 1"),
        "expected afflict keyword render, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_afflict_equivalent_rules_text_does_not_render_keyword() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Vedalken Ghoul Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Whenever this creature becomes blocked, defending player loses 4 life.")
        .expect("afflict-equivalent triggered text should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Whenever this creature becomes blocked, defending player loses 4 life")
            && !rendered.contains("Afflict 4"),
        "expected explicit trigger text rather than afflict keyword promotion, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn lost_monarch_of_ifnir_parses_afflict_grant_and_second_main_condition() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(692_108), "Lost Monarch of Ifnir")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie, Subtype::Noble])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "Afflict 3 (Whenever this creature becomes blocked, defending player loses 3 life.)\n\
             Other Zombies you control have afflict 3.\n\
             At the beginning of your second main phase, if a player was dealt combat damage by a Zombie this turn, mill three cards, then you may return a creature card from your graveyard to your hand.",
        )
        .expect("Lost Monarch of Ifnir should parse strictly");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Other Zombies you control have afflict 3"),
        "expected granted afflict keyword text, got {rendered}"
    );
    assert!(
        rendered.contains("At the beginning of your second main phase")
            && rendered.contains("if a player was dealt combat damage by a Zombie this turn"),
        "expected second-main Zombie combat-damage condition, got {rendered}"
    );

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("GrantObjectAbilityForFilter")
            && debug.contains("PlayerWasDealtCombatDamageByCreatureSubtypeThisTurn")
            && debug.contains("phase_type: Postcombat")
            && !debug.contains("BeginningOfCombat"),
        "expected structural afflict grant and postcombat-main condition, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_bloodthirst_keyword_line_compiles_without_unsupported_marker() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bloodthirst Parse Test")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Bloodthirst 3 (If an opponent was dealt damage this turn, this creature enters with three +1/+1 counters on it.)",
        )
        .expect("bloodthirst keyword line should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    let debug = format!("{def:#?}");
    assert!(
        rendered.contains("bloodthirst 3"),
        "expected bloodthirst keyword render, got {rendered}"
    );
    assert!(
        !debug.contains("KeywordMarker")
            && !debug.contains("RuleFallbackText")
            && !debug.contains("UnsupportedParserLine"),
        "bloodthirst parse should avoid unsupported placeholders, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_backup_keyword_line_compiles_to_explicit_etb_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Backup Parse Test")
        .card_types(vec![CardType::Creature])
        .parse_text("Backup 1\nFlying")
        .expect("backup keyword line should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    let debug = format!("{def:#?}");
    assert!(
        rendered.contains("backup 1"),
        "expected backup keyword render, got {rendered}"
    );
    assert!(
        debug.contains("BackupEffect"),
        "expected backup to lower to an explicit effect, got {debug}"
    );
    assert!(
        !debug.contains("KeywordMarker"),
        "backup parse should not leave a placeholder marker, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_repeated_backup_keyword_line_compiles_each_instance() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Repeated Backup Parse Test")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Backup 1, backup 1, backup 1 (When this creature enters, put a +1/+1 counter on target creature. If that's another creature, it gains the following abilities until end of turn. Each backup ability triggers separately.)\nFlying",
        )
        .expect("repeated backup keyword line should parse");

    let debug = format!("{def:#?}");
    let backup_effect_count = debug.matches("BackupEffect").count();
    assert_eq!(
        backup_effect_count, 3,
        "expected three backup triggers, got {debug}"
    );
    assert!(
        !debug.contains("KeywordFallbackText"),
        "repeated backup should not remain a fallback marker, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_backup_copy_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mirror-Shield Hoplite")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever a creature you control becomes the target of a backup ability, copy that ability. You may choose new targets for the copy. This ability triggers only once each turn.",
        )
        .expect("backup-copy trigger should parse");

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("BecomesTargetedObjectByStackObject")
            && debug.contains("CopySpellEffect")
            && debug.contains("TagTriggeringSourceEffect"),
        "expected backup-copy trigger to target and copy the triggering ability, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_plain_vanishing_keyword_line_compiles_without_marker() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Vanishing Parse Test")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Vanishing")
        .expect("plain vanishing keyword line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let debug = format!("{def:#?}");
    assert!(
        rendered.contains("Vanishing"),
        "expected vanishing keyword render, got {rendered}"
    );
    assert!(
        !debug.contains("KeywordMarker"),
        "plain vanishing should not remain a marker, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_extort_keyword_line_compiles_keyword_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Extort Parse Test")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Extort (Whenever you cast a spell, you may pay {W/B}. If you do, each opponent loses 1 life and you gain that much life.)",
        )
        .expect("extort keyword line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Extort"),
        "expected extort keyword render, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_partner_keyword_line_compiles_keyword_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Partner Parse Test")
        .card_types(vec![CardType::Creature])
        .parse_text("Partner")
        .expect("partner keyword line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Partner"),
        "expected partner keyword render, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_partner_with_keyword_line_lowers_keyword_and_search_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Partner With Parse Test")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Partner with Proud Mentor (When this creature enters, target player may put Proud Mentor into their hand from their library, then shuffle.)",
        )
        .expect("partner-with keyword line should parse");
    match &def.abilities[0].kind {
        AbilityKind::Static(static_ability) => {
            assert_eq!(static_ability.id(), StaticAbilityId::PartnerWith);
        }
        other => panic!("expected partner-with marker static ability, got {other:?}"),
    }
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Partner with Proud Mentor")
            && !rendered.contains("card named proud mentor")
            && !rendered.contains("target player"),
        "expected partner-with to render as a single keyword line, got {rendered}"
    );
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("choices: [\n                        Target(\n                            Player(\n                                Any")
            && ((debug.contains("SearchLibraryEffect") && debug.contains("search_mode: Exact"))
                || (debug.contains("ChooseObjectsEffect") && debug.contains("is_search: true"))),
        "expected partner-with to keep exactly one target player and a real library-search effect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_partner_with_attack_value_renders_oracle_surface() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Impetuous Protege")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Warrior])
        .parse_text(
            "Partner with Proud Mentor\nWhenever this creature attacks, it gets +X/+0 until end of turn, where X is the greatest power among tapped creatures your opponents control.",
        )
        .expect("impetuous protege should parse");

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("Partner with Proud Mentor")
            && rendered.contains(
                "where X is the greatest power among tapped creatures your opponents control"
            )
            && !rendered.contains("opponent's tapped creatures"),
        "expected partner-with and opponent-controlled value surface, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_leonardo_the_balance_strictly_parses_character_select_partner_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Leonardo, the Balance")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Mutant, Subtype::Ninja, Subtype::Turtle])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text(
            "Whenever a token you control enters, you may put a +1/+1 counter on each creature you control. Do this only once each turn.\n{W}{U}{B}{R}{G}: Creatures you control gain menace, trample, and lifelink until end of turn.\nPartner—Character select (You can have two commanders if both have this ability.)",
        )
        .expect("Leonardo, the Balance should parse");

    let rendered_lines = unprocessed_compiled_lines(&def);
    let rendered = rendered_lines.join("\n");
    let debug = format!("{:#?}", def);
    assert!(
        rendered_lines
            .iter()
            .any(|line| line.eq_ignore_ascii_case("Partner—Character select"))
            && rendered.contains("Do this only once each turn")
            && rendered.contains(
                "Creatures you control gain menace, trample, and lifelink until end of turn"
            )
            && debug.contains("id: Some(\n                            Partner,")
            && debug.contains("DoThisMaxTimesEachTurn")
            && debug.contains("Menace")
            && debug.contains("Trample")
            && debug.contains("Lifelink")
            && !debug.contains("id: Some(\n                            PartnerWith,"),
        "expected Leonardo output to preserve the partner variant label and keep once-per-turn and grant semantics, got rendered={rendered}; debug={debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_partner_variant_does_not_override_partner_with() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Partner Variant Guard")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Partner with Proud Mentor (When this creature enters, target player may put Proud Mentor into their hand from their library, then shuffle.)",
        )
        .expect("partner-with should still parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Partner with Proud Mentor"),
        "expected partner-with line to remain intact, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_start_your_engines_and_max_speed_graveyard_cast_permission() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Speed Parse Test")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Enchant creature or Vehicle\nStart your engines!\nEnchanted permanent gets +1/+1 and has vigilance.\nMax speed — You may cast this card from your graveyard.",
        )
        .expect("start-your-engines and max-speed graveyard cast line should parse");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Start your engines!")
            && rendered.contains("You may cast this card from your graveyard")
            && rendered.contains("as long as you have max speed"),
        "expected speed keyword plus max-speed graveyard cast permission, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_marang_river_prowler_graveyard_cast_condition() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Marang River Prowler")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie, Subtype::Fish])
        .power_toughness(PowerToughness::fixed(2, 1))
        .parse_text(
            "Skulk (This creature can't be blocked by creatures with greater power.)\nYou may cast this card from your graveyard as long as you control a black or green permanent.",
        )
        .expect("Marang River Prowler should parse");

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("Skulk")
            && rendered.contains("You may cast this card from your graveyard")
            && rendered.contains(
                "as long as you control a black permanent or you control a green permanent"
            ),
        "expected graveyard-cast condition for Marang River Prowler, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_eelectrocute_roll_six_graveyard_cast_condition_and_exile_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Eelectrocute")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Eelectrocute deals 2 damage to any target.\nYou may cast this card from your graveyard as long as you've rolled a 6 this turn. If you cast it this way and it would be put into your graveyard, exile it instead.",
        )
        .expect("Eelectrocute should parse");

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    let debug = format!("{def:#?}");
    assert!(
        rendered.contains("Eelectrocute deals 2 damage to any target")
            && rendered.contains(
                "You may cast this card from your graveyard as long as you've rolled a 6 this turn"
            )
            && rendered.contains(
                "If you cast it this way and it would be put into your graveyard, exile it instead"
            )
            && debug.contains("PlayerRolledResultThisTurn")
            && debug.contains("exiles_after_resolution: true"),
        "expected Eelectrocute parser/text output to preserve roll-six graveyard casting and exile-after-resolution semantics, got rendered={rendered}; debug={debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_maestros_ascendancy_graveyard_cast_cost_and_exile_clause() {
    assert_oracle_card_parses_strict("Maestros Ascendancy");
    let def = parse_oracle_card_definition("Maestros Ascendancy");

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    let debug = format!("{def:#?}");
    assert!(
        rendered.contains(
            "Once during each of your turns, you may cast an instant or sorcery spell from your graveyard by sacrificing a creature in addition to paying its other costs"
        ) && rendered.contains(
            "If a spell cast this way would be put into your graveyard, exile it instead"
        ),
        "expected Maestros Ascendancy to render its graveyard cast permission, sacrifice additional cost, and exile clause, got {rendered}"
    );
    assert!(
        debug.contains("GraveyardCastFromCardManaCost")
            && debug.contains("OnceDuringEachOfYourTurns")
            && debug.contains("Sacrifice")
            && debug.contains("Creature")
            && debug.contains("exiles_after_resolution: true"),
        "expected Maestros Ascendancy to lower to a once-per-turn graveyard cast grant with creature sacrifice and exile-after-resolution, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_demilich_graveyard_cast_additional_exile_cost() {
    assert_oracle_card_parses_strict("Demilich");
    let def = parse_oracle_card_definition("Demilich");

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    let debug = format!("{def:#?}");
    assert!(
        rendered.contains(
            "This spell costs {U} less to cast for each instant and sorcery spell you've cast this turn"
        ),
        "expected Demilich to render its dynamic blue cost reduction, got {rendered}"
    );
    assert!(
        rendered.contains(
            "You may cast this card from your graveyard by exiling four instant and/or sorcery cards from your graveyard in addition to paying its other costs"
        ),
        "expected Demilich to render its graveyard-cast additional exile cost, got {rendered}"
    );
    assert!(
        debug.contains("GraveyardCastFromCardManaCost")
            && debug.contains("ExileEffect")
            && debug.contains("min: 4")
            && debug.contains("Instant")
            && debug.contains("Sorcery"),
        "expected Demilich to lower to a graveyard-cast grant with a four-card instant/sorcery exile cost, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_squee_the_immortal_graveyard_or_exile_cast_permission() {
    assert_oracle_card_parses_strict("Squee, the Immortal");
    let def = parse_oracle_card_definition("Squee, the Immortal");
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    let debug = format!("{def:#?}");

    assert!(
        rendered.contains("You may cast this card from your graveyard or from exile")
            && debug.contains("grantable: PlayFrom")
            && debug.contains("zone: Graveyard")
            && debug.contains("zone: Exile"),
        "expected Squee to parse into source play-from-graveyard and play-from-exile grants, got rendered={rendered}; debug={debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_max_speed_draw_replacement_static_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Vnwxt Parse Test")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Start your engines!\nYou have no maximum hand size.\nMax speed — If you would draw a card, draw two cards instead.",
        )
        .expect("max-speed draw replacement line should parse");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Start your engines!")
            && rendered.contains("no maximum hand size")
            && rendered.contains("If you would draw a card, draw two cards instead")
            && rendered.contains("max speed"),
        "expected max-speed draw replacement static ability, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_blood_scrivener_strictly_parses_conditional_draw_replacement() {
    assert_oracle_card_parses_strict("Blood Scrivener");
    let def = parse_oracle_card_definition("Blood Scrivener");
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert!(
        def.abilities.iter().any(|ability| matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::ConditionalDrawReplacement
        )),
        "Blood Scrivener should compile to a conditional draw replacement static ability, got {def:#?}"
    );
    assert!(
        def.spell_effect.is_none(),
        "Blood Scrivener's replacement text should not lower as a spell effect"
    );
    assert!(
        rendered.contains("If you would draw a card while you have no cards in hand")
            && rendered.contains("instead draw two cards and you lose 1 life"),
        "expected Blood Scrivener conditional draw-replacement text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn add_blood_scrivener_library_cards(
    game: &mut crate::game_state::GameState,
    player: PlayerId,
    count: u32,
) {
    for index in 0..count {
        game.create_object_from_card(
            &crate::card::CardBuilder::new(
                CardId::from_raw(20_000 + index),
                &format!("Blood Scrivener Library Card {index}"),
            )
            .card_types(vec![CardType::Creature])
            .build(),
            player,
            Zone::Library,
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn blood_scrivener_replaces_draw_when_controller_has_no_cards_in_hand() {
    let def = parse_oracle_card_definition("Blood Scrivener");
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let blood_scrivener = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    add_blood_scrivener_library_cards(&mut game, alice, 3);

    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(blood_scrivener, alice, &mut dm);
    let result = DrawCardsEffect::you(1)
        .execute(&mut game, &mut ctx)
        .expect("Blood Scrivener replacement draw should resolve");

    assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
    assert_eq!(game.player(alice).unwrap().hand.len(), 2);
    assert_eq!(game.player(alice).unwrap().library.len(), 1);
    assert_eq!(game.life_total(alice), 19);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn blood_scrivener_does_not_replace_draw_when_controller_has_cards_in_hand() {
    let def = parse_oracle_card_definition("Blood Scrivener");
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let blood_scrivener = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::from_raw(20_100), "Blood Scrivener Hand Card")
            .card_types(vec![CardType::Creature])
            .build(),
        alice,
        Zone::Hand,
    );
    add_blood_scrivener_library_cards(&mut game, alice, 2);

    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(blood_scrivener, alice, &mut dm);
    let result = DrawCardsEffect::you(1)
        .execute(&mut game, &mut ctx)
        .expect("normal draw should resolve");

    assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
    assert_eq!(game.player(alice).unwrap().hand.len(), 2);
    assert_eq!(game.player(alice).unwrap().library.len(), 1);
    assert_eq!(game.life_total(alice), 20);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_exploit_keyword_line_lowers_to_keyword_action_event() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Exploit Parse Test")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Exploit (When this creature enters, you may sacrifice a creature.)\nWhen this creature exploits a creature, target player draws two cards and loses 2 life.",
        )
        .expect("exploit keyword line and exploit trigger should parse");
    let debug = format!("{def:#?}");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        debug.contains("MayEffect")
            && debug.contains("SacrificeEffect")
            && debug.contains("EmitKeywordActionEffect")
            && debug.contains("Exploit"),
        "expected exploit to lower to optional sacrifice plus keyword event, got {debug}"
    );
    assert!(
        rendered.contains("Exploit")
            && rendered.contains("Whenever this creature exploits a creature")
            && !rendered.contains("If you do,."),
        "expected exploit keyword and exploit trigger to render cleanly, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_exploit_grant_and_creature_you_control_exploit_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Exploit Grant Parse Test")
        .card_types(vec![CardType::Creature])
        .supertypes(vec![Supertype::Legendary])
        .parse_text(
            "Other legendary creatures you control have exploit.\nWhenever a creature you control exploits a creature, put a +1/+1 counter on each creature you control.",
        )
        .expect("exploit grant and filtered exploit trigger should parse");
    let debug = format!("{def:#?}");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        debug.contains("GrantObjectAbilityForFilter")
            && debug.contains("EmitKeywordActionEffect")
            && debug.contains("KeywordActionTrigger")
            && debug.contains("Exploit"),
        "expected exploit grant to lower to granted triggered ability and filtered keyword-action trigger, got {debug}"
    );
    assert!(
        rendered.contains("Whenever a creature you control exploits a creature"),
        "expected filtered exploit trigger to render, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_exploit_trigger_filters_exploited_object() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Exploit Object Filter Parse Test")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Exploit (When this creature enters, you may sacrifice a creature.)\nWhenever a creature you control exploits a nontoken creature, create a 2/2 black Zombie creature token.",
        )
        .expect("exploit trigger object filter should parse");
    let debug = format!("{def:#?}");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        debug.contains("tagged_object_filter")
            && debug.contains("exploited")
            && debug.contains("nontoken: true"),
        "expected exploit trigger to filter the exploited object, got {debug}"
    );
    assert!(
        rendered.contains("Whenever a creature you control exploits a nontoken creature"),
        "expected exploited object filter to render, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_exploit_payoff_owner_chooses_top_or_bottom_library() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Exploit Top Bottom Parse Test")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Exploit (When this creature enters, you may sacrifice a creature.)\nWhen this creature exploits a creature, target creature's owner puts it on their choice of the top or bottom of their library.",
        )
        .expect("exploit payoff top-or-bottom library choice should parse");
    let debug = format!("{def:#?}");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        debug.contains("MoveToLibraryTopOrBottomChoiceEffect"),
        "expected top-or-bottom library choice effect, got {debug}"
    );
    assert!(
        rendered.contains(
            "target creature's owner puts it on their choice of the top or bottom of their library"
        ),
        "expected owner-choice library placement to render, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_exploit_payoff_references_exploited_creature_toughness() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Exploit Toughness Parse Test")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Exploit (When this creature enters, you may sacrifice a creature.)\nWhen this creature exploits a creature, return to their owners' hands all creatures your opponents control with toughness less than the exploited creature's toughness.",
        )
        .expect("exploit payoff should parse exploited creature toughness reference");
    let debug = format!("{def:#?}");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        debug.contains("object_tags")
            && debug.contains("exploited")
            && debug.contains("ToughnessOf")
            && debug.contains("ReturnToHandEffect"),
        "expected exploit to tag sacrificed memory and payoff to compare toughness, got {debug}"
    );
    assert!(
        rendered.contains("toughness less than the exploited creature's toughness"),
        "expected exploited creature toughness comparison to render, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_exploit_conditional_checks_source_exploited_triggering_creature() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Exploit Conditional Parse Test")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Flying\nExploit (When this creature enters, you may sacrifice a creature.)\nWhenever another creature you control dies, put a +1/+1 counter on this creature. It gains haste until end of turn if it exploited that creature.",
        )
        .expect("exploit conditional should parse");
    let debug = format!("{def:#?}");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        debug.contains("exploited")
            && debug.contains("exploiter")
            && debug.contains("TaggedObjectMatches"),
        "expected exploit conditional to use exploited/exploiter event tags, got {debug}"
    );
    assert!(
        rendered.contains("If it exploited that creature, it gains haste until end of turn"),
        "expected exploit conditional to render, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_assist_keyword_line_compiles_keyword_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Assist Parse Test")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Assist")
        .expect("assist keyword line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Assist"),
        "expected assist keyword render, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_modular_keyword_line_compiles_keyword_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Modular Parse Test")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .parse_text("Modular 1 (This creature enters with a +1/+1 counter on it.)")
        .expect("modular keyword line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Modular 1"),
        "expected modular keyword render, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_modular_sunburst_keyword_line_lowers_to_full_semantics() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Modular Sunburst Parse Test")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(0, 0))
        .parse_text("Modular—Sunburst")
        .expect("modular sunburst keyword line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("modular—sunburst"),
        "expected modular sunburst keyword render, got {rendered}"
    );
    assert!(
        !rendered_lower.contains("modular_triggering_object"),
        "expected modular sunburst render to hide internal trigger tags, got {rendered}"
    );

    let modular_target = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered.choices.iter().find_map(|choice| {
                let ChooseSpec::Target(inner) = choice else {
                    return None;
                };
                let ChooseSpec::Object(filter) = inner.as_ref() else {
                    return None;
                };
                Some(filter)
            }),
            _ => None,
        })
        .expect("modular sunburst death trigger should target an artifact creature");
    assert!(
        modular_target.all_card_types.contains(&CardType::Artifact)
            && modular_target.all_card_types.contains(&CardType::Creature),
        "expected modular sunburst death trigger to target artifact creatures, got {modular_target:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_graft_keyword_line_compiles_keyword_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Graft Parse Test")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Graft 2 (This creature enters with two +1/+1 counters on it. Whenever another creature enters, you may move a +1/+1 counter from this creature onto it.)",
        )
        .expect("graft keyword line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Graft 2"),
        "expected graft keyword render, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_sunburst_keyword_line_compiles_keyword_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sunburst Parse Test")
        .card_types(vec![CardType::Artifact])
        .parse_text("Sunburst")
        .expect("sunburst keyword line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Sunburst"),
        "expected sunburst keyword render, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_fading_keyword_line_compiles_keyword_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Fading Parse Test")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Fading 2 (This creature enters with two fade counters on it. At the beginning of your upkeep, remove a fade counter from it. If you can't, sacrifice it.)",
        )
        .expect("fading keyword line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Fading 2"),
        "expected fading keyword render, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_cant_gain_life_from_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "No Life")
        .parse_text("Players can't gain life.")
        .expect("parse players can't gain life");

    let has_cant_gain = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(ability) if ability.id() == StaticAbilityId::PlayersCantGainLife
        )
    });

    assert!(has_cant_gain);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_cant_get_additional_poison_counters_from_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Poison Shield")
        .parse_text("You can't get additional poison counters this turn.")
        .expect("parse poison-counter restriction");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("you can't get additional poison counters this turn")
            || rendered.contains("you cant get additional poison counters this turn"),
        "expected poison-counter restriction text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_deafening_silence_noncreature_cast_limit() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Deafening Silence Variant")
        .parse_text("Each player can't cast more than one noncreature spell each turn.")
        .expect("parse each-player noncreature cast limit");

    let has_rule_restriction = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(ability) if ability.id() == StaticAbilityId::RuleRestriction
        )
    });
    assert!(
        has_rule_restriction,
        "expected a rule restriction static ability"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("each player can't cast more than one noncreature spell each turn")
            || rendered.contains("each player cant cast more than one noncreature spell each turn"),
        "expected deafening silence cast-limit text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_you_cant_cast_more_than_one_spell_each_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Moderation Variant")
        .parse_text("You can't cast more than one spell each turn.")
        .expect("parse you-cant-cast-more-than-one-spell restriction");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("you can't cast more than one spell each turn")
            || rendered.contains("you cant cast more than one spell each turn"),
        "expected player-scoped cast-limit text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_enchanted_player_cant_cast_more_than_one_spell_each_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Curse Cast Limit Variant")
        .parse_text("Enchant player\nEnchanted player can't cast more than one spell each turn.")
        .expect("parse enchanted-player cast-limit restriction");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("enchanted player can't cast more than one spell each turn")
            || rendered.contains("enchanted player cant cast more than one spell each turn"),
        "expected enchanted-player cast-limit text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_each_player_cant_cast_more_than_one_spell_each_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Arcane Laboratory Variant")
        .parse_text("Each player can't cast more than one spell each turn.")
        .expect("parse each-player one-spell cast limit");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("each player can't cast more than one spell each turn")
            || rendered.contains("each player cant cast more than one spell each turn"),
        "expected each-player cast-limit text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_noncreature_spells_cant_be_cast_restrictions() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Gaddock Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Noncreature spells with mana value 4 or greater can't be cast.\n\
             Noncreature spells with {X} in their mana costs can't be cast.",
        )
        .expect("parse noncreature spell cast restrictions");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("CastSpellsMatching")
            && debug.contains("GreaterThanOrEqual(4)")
            && debug.contains("excluded_card_types")
            && debug.contains("has_x_in_cost: true"),
        "expected Gaddock-style cast restrictions, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_players_cant_cast_more_than_one_spell_each_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Rule Of Law Variant")
        .parse_text("Players can't cast more than one spell each turn.")
        .expect("parse players one-spell cast limit");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("players can't cast more than one spell each turn")
            || rendered.contains("players cant cast more than one spell each turn"),
        "expected players cast-limit text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_canonist_style_nonartifact_cast_limit() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Canonist Variant")
        .parse_text("Each player who has cast a nonartifact spell this turn can't cast additional nonartifact spells.")
        .expect("parse canonist-style nonartifact cast limit");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "each player who has cast a nonartifact spell this turn can't cast additional nonartifact spells"
        )
            || rendered.contains(
                "each player who has cast a nonartifact spell this turn cant cast additional nonartifact spells"
            )
            || rendered.contains("each player can't cast more than one nonartifact spell each turn")
            || rendered.contains("each player cant cast more than one nonartifact spell each turn"),
        "expected canonist-style cast-limit normalization, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_each_opponent_with_poison_counter_threshold() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Corrupted Atlas Variant")
        .parse_text(
            "Corrupted - Whenever this artifact becomes tapped, each opponent who has three or more poison counters loses 1 life.",
        )
        .expect("parse each-opponent poison threshold trigger");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("each opponent who has three or more poison counters loses 1 life")
            || rendered.contains("for each opponent, if that player has 3 or more poison counters, that player loses 1 life"),
        "expected poison-threshold opponent life-loss trigger, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_your_opponents_nonartifact_cast_limit() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Lavinia Variant")
        .parse_text("Your opponents can't cast more than one nonartifact spell each turn.")
        .expect("parse your-opponents nonartifact cast limit");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("your opponents can't cast more than one nonartifact spell each turn")
            || rendered
                .contains("your opponents cant cast more than one nonartifact spell each turn"),
        "expected your-opponents nonartifact cast-limit text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_nonphyrexian_cast_limit() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Phyrexian Censor Variant")
        .parse_text("Each player can't cast more than one non-Phyrexian spell each turn.")
        .expect("parse non-phyrexian cast limit");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("each player can't cast more than one non-phyrexian spell each turn")
            || rendered
                .contains("each player cant cast more than one non-phyrexian spell each turn"),
        "expected non-phyrexian cast-limit text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_uncounterable_from_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "No Counter")
        .parse_text("This spell can't be countered.")
        .expect("parse this spell can't be countered");

    let has_uncounterable = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(ability) if ability.id() == StaticAbilityId::CantBeCountered
        )
    });

    assert!(has_uncounterable);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn vexing_shusher_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Vexing Shusher");
    let rendered = unprocessed_compiled_lines(&def).join("\n");

    assert!(
        rendered.contains("This spell can't be countered."),
        "expected static uncounterable line for Vexing Shusher, got {rendered}"
    );
    assert!(
        rendered.contains("{R/G}: Target spell can't be countered."),
        "expected targeted activated uncounterable line for Vexing Shusher, got {rendered}"
    );

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Vexing Shusher should have an activated ability");
    let default_effects = activated
        .effects
        .segments
        .iter()
        .flat_map(|segment| segment.default_effects.iter())
        .collect::<Vec<_>>();
    fn find_target_only(effect: &crate::effect::Effect) -> Option<&TargetOnlyEffect> {
        effect.downcast_ref::<TargetOnlyEffect>().or_else(|| {
            effect
                .downcast_ref::<TaggedEffect>()
                .and_then(|tagged| find_target_only(&tagged.effect))
        })
    }
    fn find_cant_effect(effect: &crate::effect::Effect) -> Option<&crate::effects::CantEffect> {
        effect
            .downcast_ref::<crate::effects::CantEffect>()
            .or_else(|| {
                effect
                    .downcast_ref::<TaggedEffect>()
                    .and_then(|tagged| find_cant_effect(&tagged.effect))
            })
    }

    let target_only = default_effects
        .iter()
        .find_map(|effect| find_target_only(effect))
        .expect("Vexing Shusher activation should expose a target-only spell choice");
    assert_eq!(
        target_only.target.inner(),
        &ChooseSpec::spell(),
        "Vexing Shusher activation should target a spell, got {:?}",
        target_only.target
    );

    let cant_effect = default_effects
        .iter()
        .find_map(|effect| find_cant_effect(effect))
        .expect("Vexing Shusher activation should apply a cant-be-countered effect");
    match &cant_effect.restriction {
        crate::effect::Restriction::BeCountered(filter) => {
            assert_eq!(filter.zone, Some(Zone::Stack));
            assert_eq!(
                filter.stack_kind,
                Some(crate::filter::StackObjectKind::Spell)
            );
            assert!(
                filter.tagged_constraints.iter().any(|constraint| {
                    constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                }),
                "Vexing Shusher's restriction should be tied to the chosen spell target, got {filter:?}"
            );
        }
        other => panic!("expected Vexing Shusher be-countered restriction, got {other:?}"),
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn loxodon_smiter_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Loxodon Smiter");
    let rendered = unprocessed_compiled_lines(&def).join("\n");

    assert!(
        rendered.contains("This spell can't be countered."),
        "expected Loxodon Smiter to render its uncounterable spell line, got {rendered}"
    );
    assert!(
        rendered.contains("If a spell or ability an opponent controls causes you to discard this card, put it onto the battlefield instead of putting it into your graveyard"),
        "expected Loxodon Smiter to render its opponent-caused discard replacement, got {rendered}"
    );

    assert!(
        def.abilities.iter().any(|ability| matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::CantBeCountered
                    && ability.functions_in(&Zone::Stack)
        )),
        "Loxodon Smiter's uncounterable ability should function on the stack"
    );
    assert!(
        def.abilities.iter().any(|ability| matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id()
                    == StaticAbilityId::OpponentEffectDiscardThisToBattlefieldReplacement
                    && ability.functions_in(&Zone::Hand)
        )),
        "Loxodon Smiter's discard replacement should function from hand"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_spells_cant_be_countered_as_rule_restriction() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Global No Counter")
        .parse_text("Spells can't be countered.")
        .expect("parse spells can't be countered");

    let has_rule_restriction = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(ability) if ability.id() == StaticAbilityId::RuleRestriction
        )
    });

    assert!(has_rule_restriction);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_cavern_of_souls_generic_mana_usage_restriction() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Cavern of Souls")
        .card_types(vec![CardType::Land])
        .parse_text(
            "As this land enters, choose a creature type.\n{T}: Add {C}.\n{T}: Add one mana of any color. Spend this mana only to cast a creature spell of the chosen type, and that spell can't be countered.",
        )
        .expect("cavern of souls style mana restriction should parse");

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) if !activated.mana_usage_restrictions.is_empty() => {
                Some(activated)
            }
            _ => None,
        })
        .expect("expected colored mana ability with typed spend restriction");

    assert_eq!(
        activated.mana_usage_restrictions,
        vec![crate::ability::ManaUsageRestriction::CastSpell {
            card_types: vec![CardType::Creature],
            subtype_requirement: Some(
                crate::ability::ManaUsageSubtypeRequirement::ChosenTypeOfSource,
            ),
            restrict_to_matching_spell: true,
            grant_uncounterable: true,
            enters_with_counters: vec![],
            granted_abilities: vec![],
        }]
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_nonsource_cant_block_specific_attacker_restriction() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cowardly Rule")
        .parse_text("Cowards can't block Warriors.")
        .expect("parse cowards can't block warriors");

    let has_rule_restriction = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(ability) if ability.id() == StaticAbilityId::RuleRestriction
        )
    });

    assert!(has_rule_restriction);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_multi_color_cant_block_this_turn_restriction() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Flash of Defiance Test")
        .parse_text("Green creatures and white creatures can't block this turn.")
        .expect("parse multicolor cant-block-this-turn restriction");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("green creatures can't block this turn")
            || rendered.contains("white creatures can't block this turn"),
        "expected at least one parsed color cant-block-this-turn clause, got {rendered}"
    );
    assert!(
        rendered.contains("green creatures") && rendered.contains("white creatures"),
        "expected both colors in parsed restriction output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_bare_cant_be_blocked_by_more_than_one_creature() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bare Unblockable Limit")
        .parse_text("Can't be blocked by more than one creature.")
        .expect("parse bare cant-be-blocked-by-more-than clause");

    let has_limit = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(ability)
                if ability.id() == StaticAbilityId::CantBeBlockedByMoreThan
        )
    });

    assert!(has_limit);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_enchanted_creature_cant_attack_or_block() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Arrest Test")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Enchant creature\nEnchanted creature can't attack or block.")
        .expect("parse enchanted creature cant attack or block");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("enchanted creature can't attack or block")
            || rendered.contains("enchanted creature cant attack or block"),
        "expected enchanted attack/block restriction text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_enchanted_creature_cant_activate_abilities() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Arrest Plus Test")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Enchant creature\nEnchanted creature can't attack or block, and its activated abilities can't be activated.",
        )
        .expect("parse enchanted creature activated-abilities restriction");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("enchanted creature can't attack or block")
            && (rendered.contains("its activated abilities can't be activated")
                || rendered.contains("enchanted creature activated abilities can't be activated")),
        "expected arrest-style restriction text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_strip_bare_normalizes_destroy_attached_auras_and_equipment() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Strip Bare")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
        .card_types(vec![CardType::Instant])
        .parse_text("Destroy all Auras and Equipment attached to target creature.")
        .expect("Strip Bare text should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("destroy all auras and equipment attached to target creature"),
        "expected Strip Bare's attached-permanent text to normalize cleanly, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_soul_nova_parses_and_renders_attached_equipment_exile_bundle() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(48_101), "Soul Nova")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::White],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text("Exile target attacking creature and all Equipment attached to it.")
        .expect("Soul Nova text should parse strictly");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert_eq!(
        rendered, "Exile target attacking creature and all Equipment attached to it.",
        "Soul Nova should render the attached Equipment exile bundle as one structural clause"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_deadlock_trap_its_activated_abilities_cant_be_activated_this_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Deadlock Trap Test")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "This artifact enters tapped.\n{T}, Pay {E}: Tap target creature or planeswalker. Its activated abilities can't be activated this turn.",
        )
        .expect("parse deadlock-trap style activated-abilities clause");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("its activated abilities can't be activated this turn")
            || rendered.contains("activated abilities of permanent can't be activated this turn")
            || rendered.contains("its activated abilities cant be activated this turn"),
        "expected deadlock-trap restriction text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_activated_abilities_with_t_in_costs_cant_be_activated() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Serra Bestiary Test")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Enchant creature\nEnchanted creature's activated abilities with {T} in their costs can't be activated.",
        )
        .expect("parse tap-cost activated-ability restriction");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("activated abilities with {t} in their costs can't be activated")
            || rendered.contains(
                "enchanted creatures activated abilities with t in their costs can't be activated"
            )
            || rendered.contains("activated abilities with t in their costs cant be activated"),
        "expected tap-cost activated-ability restriction text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_enchanted_permanent_cant_attack_or_block() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bound In Gold Test")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Enchant permanent\nEnchanted permanent can't attack or block.")
        .expect("parse enchanted permanent cant attack or block");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("enchanted permanent can't attack or block")
            || rendered.contains("enchanted permanent cant attack or block"),
        "expected attached cant attack or block text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_target_creature_you_dont_control_gets_minus_two_minus_two() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Downsize Test")
        .parse_text("Target creature you don't control gets -2/-2 until end of turn.")
        .expect("parse target creature you dont control gets -2/-2");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("target creature you don't control gets -2/-2 until end of turn")
            || rendered.contains("target creature you dont control gets -2/-2 until end of turn"),
        "expected parsed pump effect, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_destination_first_return_all_to_hand_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Return To Hand Test")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Return to your hand all creature cards in your graveyard that were put there from the battlefield this turn.",
        )
        .expect("parse destination-first return-to-hand clause");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("in your graveyard") && rendered.contains("to your hand"),
        "expected destination-first return-to-hand text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_split_the_party_chooses_target_player_and_half_their_creatures() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Split the Party")
        .card_types(vec![CardType::Sorcery])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .parse_text("Choose target player. Return half the creatures they control to their owner's hand, rounded up.")
        .expect("Split the Party should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("choose target player")
            && (rendered.contains(
                "return half the creatures they control to their owner's hand, rounded up"
            ) || rendered.contains(
                "return half the creatures that player controls to their owner's hand, rounded up"
            ))
            && rendered.contains("hand"),
        "expected choose-player plus half-creature return text, got {rendered}"
    );

    let debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    assert!(
        debug.contains("TargetOnlyEffect")
            && debug.contains("ChooseObjectsEffect")
            && debug.contains("ReturnToHandEffect")
            && debug.contains("HalfRoundedDown"),
        "expected Split the Party to lower to target-player, choose-objects, and return-to-hand effects, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_destination_first_return_all_to_battlefield_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Return To Battlefield Test")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Return to the battlefield all permanent cards in your graveyard that were put there from the battlefield this turn.",
        )
        .expect("parse destination-first return-to-battlefield clause");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        (rendered.contains("in your graveyard") || rendered.contains("from your graveyard"))
            && rendered.contains("to the battlefield"),
        "expected destination-first return-to-battlefield text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_choose_color_as_enters_for_nonland_subjects() {
    let creature_def = CardDefinitionBuilder::new(CardId::from_raw(1), "Color Creature")
        .card_types(vec![CardType::Creature])
        .parse_text("As this creature enters, choose a color.")
        .expect("parse as this creature enters choose a color");
    let enchantment_def = CardDefinitionBuilder::new(CardId::from_raw(2), "Color Enchantment")
        .card_types(vec![CardType::Enchantment])
        .parse_text("As this enchantment enters, choose a color.")
        .expect("parse as this enchantment enters choose a color");

    let creature_has = creature_def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(ability) if ability.id() == StaticAbilityId::ChooseColorAsEnters
        )
    });
    let enchantment_has = enchantment_def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(ability) if ability.id() == StaticAbilityId::ChooseColorAsEnters
        )
    });

    assert!(creature_has);
    assert!(enchantment_has);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_choose_basic_land_type_as_enters_for_aura() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Convincing Mirage Variant")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text("As this aura enters, choose a basic land type.")
        .expect("parse as this aura enters choose a basic land type");

    let ids: Vec<StaticAbilityId> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(ids.contains(&StaticAbilityId::ChooseBasicLandTypeAsEnters));
    assert!(
        !ids.contains(&StaticAbilityId::RuleFallbackText),
        "expected typed basic-land-type-as-enters static ability, got {ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_choose_land_type_as_enters_for_aura() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Traveler's Cloak Variant")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(
            "Enchant creature.\nAs this aura enters, choose a land type.\nEnchanted creature has landwalk of the chosen type.",
        )
        .expect("parse as this aura enters choose a land type");

    let ids: Vec<StaticAbilityId> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(ids.contains(&StaticAbilityId::ChooseLandTypeAsEnters));
    assert!(ids.contains(&StaticAbilityId::AttachedChosenLandwalkGrant));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_choose_player_as_enters_without_placeholder() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Player Choice Artifact")
        .card_types(vec![CardType::Artifact])
        .parse_text("As this artifact enters, choose a player.")
        .expect("parse as this artifact enters choose a player");

    let ids: Vec<StaticAbilityId> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(ids.contains(&StaticAbilityId::ChoosePlayerAsEnters));
    assert!(
        !ids.contains(&StaticAbilityId::RuleFallbackText),
        "expected typed choose-player-as-enters static ability, got {ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_enchanted_land_is_chosen_type_without_placeholder() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Phantasmal Terrain Variant")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(
            "Enchant land.\nAs this aura enters, choose a basic land type.\nEnchanted land is the chosen type.",
        )
        .expect("parse chosen basic land type Aura lines");

    let ids: Vec<StaticAbilityId> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(ids.contains(&StaticAbilityId::ChooseBasicLandTypeAsEnters));
    assert!(ids.contains(&StaticAbilityId::EnchantedLandIsChosenType));
    assert!(
        !ids.contains(&StaticAbilityId::RuleFallbackText)
            && !ids.contains(&StaticAbilityId::UnsupportedParserLine),
        "expected typed chosen-type Aura static abilities, got {ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_aura_chosen_basic_land_type_sets_enchanted_land_subtype() {
    let aura_def = CardDefinitionBuilder::new(CardId::from_raw(1), "Convincing Mirage Variant")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(
            "Enchant land.\nAs this aura enters, choose a basic land type.\nEnchanted land is the chosen type.",
        )
        .expect("parse chosen basic land type Aura");

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let land_card = crate::card::CardBuilder::new(CardId::from_raw(2), "Test Forest")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Forest])
        .build();
    let land_id = game.create_object_from_card(&land_card, alice, crate::zone::Zone::Battlefield);

    let aura_id_in_hand =
        game.create_object_from_definition(&aura_def, alice, crate::zone::Zone::Hand);
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let result = game
        .move_object_with_etb_processing_with_dm(
            aura_id_in_hand,
            crate::zone::Zone::Battlefield,
            &mut dm,
        )
        .expect("aura should enter and attach to the available land");
    let aura_id = result.new_id;

    assert_eq!(
        game.chosen_basic_land_type(aura_id),
        Some(Subtype::Plains),
        "select-first decision maker should choose Plains"
    );
    assert_eq!(
        game.object(aura_id).and_then(|obj| obj.attached_to),
        Some(crate::object::AttachmentTarget::Object(land_id)),
        "aura should attach to the only legal land"
    );

    let land_chars = game
        .calculated_characteristics(land_id)
        .expect("land should have calculated characteristics");
    assert!(
        land_chars.subtypes.contains(&Subtype::Plains),
        "enchanted land should become the chosen type"
    );
    assert!(
        !land_chars.subtypes.contains(&Subtype::Forest),
        "set-subtype effect should replace prior land subtypes"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn lithoform_blight_removes_land_types_and_old_abilities_then_grants_both_mana_abilities()
 {
    let blight = parse_oracle_card_definition("Lithoform Blight");
    let forest = crate::card::CardBuilder::new(CardId::from_raw(92_001), "Test Forest")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Forest])
        .build();
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let land = game.create_object_from_card(&forest, alice, Zone::Battlefield);
    let aura = game.create_object_from_definition(&blight, alice, Zone::Battlefield);
    assert!(game.attach_object_to_target(aura, crate::object::AttachmentTarget::Object(land),));
    game.refresh_continuous_state();

    let subtypes = game.calculated_subtypes(land);
    for basic_type in [
        Subtype::Plains,
        Subtype::Island,
        Subtype::Swamp,
        Subtype::Mountain,
        Subtype::Forest,
    ] {
        assert!(
            !subtypes.contains(&basic_type),
            "Lithoform Blight should remove every land type, got {subtypes:?}"
        );
    }

    let abilities = game
        .current_abilities(land)
        .expect("enchanted land should have calculated abilities");
    assert_eq!(
        abilities.len(),
        2,
        "expected exactly the two granted abilities: {abilities:#?}"
    );
    assert!(
        abilities
            .iter()
            .all(|ability| matches!(ability.kind, AbilityKind::Activated(_))),
        "both Lithoform Blight grants should be activated mana abilities: {abilities:#?}"
    );
    let debug = format!("{abilities:#?}");
    assert!(debug.contains("Colorless"), "{debug}");
    assert!(debug.contains("Life"), "{debug}");
    assert!(debug.contains("AddManaOfAnyColorEffect"), "{debug}");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_roaming_throne_variant_parses_without_placeholders() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Roaming Throne Variant")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Golem])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "Ward {2}\nAs this creature enters, choose a creature type.\nThis creature is the chosen type in addition to its other types.\nIf a triggered ability of another creature you control of the chosen type triggers, it triggers an additional time.",
        )
        .expect("parse roaming throne variant");

    let ids: Vec<StaticAbilityId> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(ids.contains(&StaticAbilityId::Ward));
    assert!(ids.contains(&StaticAbilityId::ChooseCreatureTypeAsEnters));
    assert!(ids.contains(&StaticAbilityId::AddChosenCreatureType));
    assert!(ids.contains(&StaticAbilityId::DuplicateMatchingTriggeredAbilities));
    assert!(
        !ids.contains(&StaticAbilityId::RuleFallbackText)
            && !ids.contains(&StaticAbilityId::UnsupportedParserLine),
        "expected typed Roaming Throne static abilities, got {ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_chosen_creature_type_static_adds_selected_subtype() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Roaming Throne Variant")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Golem])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "As this creature enters, choose a creature type.\nThis creature is the chosen type in addition to its other types.",
        )
        .expect("parse chosen creature type addition lines");

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let throne_id = game.create_object_from_definition(&def, alice, crate::zone::Zone::Battlefield);
    game.set_chosen_creature_type(throne_id, Subtype::Wall);

    let chars = game
        .calculated_characteristics(throne_id)
        .expect("roaming throne variant should have calculated characteristics");
    assert!(
        chars.subtypes.contains(&Subtype::Golem),
        "expected original subtype to remain"
    );
    assert!(
        chars.subtypes.contains(&Subtype::Wall),
        "expected chosen subtype to be added"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn painters_servant_definition() -> CardDefinition {
    parse_oracle_card_definition("Painter's Servant")
}

#[test]
pub(super) fn realmwright_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Realmwright");

    let def = parse_oracle_card_definition("Realmwright");
    let ids: Vec<StaticAbilityId> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(ids.contains(&StaticAbilityId::ChooseBasicLandTypeAsEnters));
    assert!(ids.contains(&StaticAbilityId::AddChosenBasicLandType));
    assert!(
        !ids.contains(&StaticAbilityId::RuleFallbackText)
            && !ids.contains(&StaticAbilityId::UnsupportedParserLine),
        "expected strict Realmwright static abilities, got {ids:?}"
    );

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("As this creature enters, choose a basic land type."),
        "expected Realmwright choose-basic-land-type as-enters wording, got {rendered}"
    );
    assert!(
        rendered
            .contains("Lands you control are the chosen type in addition to their other types."),
        "expected Realmwright chosen basic land type static wording, got {rendered}"
    );
}

#[test]
pub(super) fn realmwright_adds_chosen_basic_land_type_to_lands_you_control_only() {
    let def = parse_oracle_card_definition("Realmwright");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let alice_land_card = crate::card::CardBuilder::new(CardId::from_raw(2), "Alice Forest")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Forest])
        .build();
    let bob_land_card = crate::card::CardBuilder::new(CardId::from_raw(3), "Bob Island")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Island])
        .build();
    let alice_land =
        game.create_object_from_card(&alice_land_card, alice, crate::zone::Zone::Battlefield);
    let bob_land =
        game.create_object_from_card(&bob_land_card, bob, crate::zone::Zone::Battlefield);

    let realmwright_in_hand =
        game.create_object_from_definition(&def, alice, crate::zone::Zone::Hand);
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let result = game
        .move_object_with_etb_processing_with_dm(
            realmwright_in_hand,
            crate::zone::Zone::Battlefield,
            &mut dm,
        )
        .expect("Realmwright should enter and choose a basic land type");
    let realmwright = result.new_id;

    assert_eq!(
        game.chosen_basic_land_type(realmwright),
        Some(Subtype::Plains),
        "select-first decision maker should choose Plains"
    );

    let alice_land_chars = game
        .calculated_characteristics(alice_land)
        .expect("Alice's land should have calculated characteristics");
    assert!(
        alice_land_chars.subtypes.contains(&Subtype::Forest),
        "Realmwright should preserve original land subtypes"
    );
    assert!(
        alice_land_chars.subtypes.contains(&Subtype::Plains),
        "Realmwright should add the chosen basic land type to lands Alice controls"
    );

    let bob_land_chars = game
        .calculated_characteristics(bob_land)
        .expect("Bob's land should have calculated characteristics");
    assert!(
        bob_land_chars.subtypes.contains(&Subtype::Island),
        "opposing lands should keep their original subtype"
    );
    assert!(
        !bob_land_chars.subtypes.contains(&Subtype::Plains),
        "Realmwright should not affect lands controlled by opponents"
    );
}
