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
pub(super) fn alms_beast_keeps_lifelink_grant_for_creatures_in_combat_with_source() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Alms Beast Variant")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Beast])
        .power_toughness(PowerToughness::fixed(6, 6))
        .parse_text("Creatures blocking or blocked by this creature have lifelink.")
        .expect("Alms Beast should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("creatures blocking or blocked by this creature have lifelink"),
        "expected Alms Beast to keep its in-combat grant wording, got {rendered}"
    );

    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        (debug.contains("grantability") || debug.contains("grantobjectabilityforfilter"))
            && debug.contains("lifelink")
            && debug.contains("in_combat_with_source: true"),
        "expected Alms Beast definition to grant lifelink to creatures in combat with the source, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn named_source_blocking_or_blocked_by_filter_marks_in_combat_with_source() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sisters of Stone Death")
        .card_types(vec![CardType::Creature])
        .parse_text("Exile target creature blocking or blocked by Sisters of Stone Death.")
        .expect("named source in-combat target should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("target creature blocking or blocked by this creature"),
        "expected named source target to render as in-combat-with-source, got {rendered}"
    );

    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        debug.contains("in_combat_with_source: true"),
        "expected named source target to mark in_combat_with_source, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn metalcraft_keyword_grant_keeps_label_in_oracle_like_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Spiraling Duelist Variant")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Berserker])
        .power_toughness(PowerToughness::fixed(3, 1))
        .parse_text(
            "Metalcraft — This creature has double strike as long as you control three or more artifacts.",
        )
        .expect("Spiraling Duelist should parse");

    let lines = unprocessed_compiled_lines(&def);
    assert_eq!(
        lines,
        vec![
            "This creature has double strike as long as you control three or more artifacts."
                .to_string()
        ],
        "expected metalcraft condition to survive debug-safe rendering, got {lines:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sand_golem_keeps_opponent_discard_trigger_and_delayed_return_counter() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sand Golem Variant")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Golem])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text(
            "When a spell or ability an opponent controls causes you to discard this card, return this card from your graveyard to the battlefield with a +1/+1 counter on it at the beginning of the next end step.",
        )
        .expect("Sand Golem should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("opponent controls causes you to discard this card")
            && rendered.contains("beginning of the next end step")
            && rendered.contains("return this card from your graveyard to the battlefield")
            && rendered.contains("+1/+1 counter"),
        "expected Sand Golem to keep discard trigger plus delayed return/counter text, got {rendered}"
    );

    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        debug.contains("youdiscardcardtrigger")
            && debug.contains("cause_controller: some(")
            && debug.contains("opponent")
            && debug.contains("effect_like_only: true")
            && debug.contains("scheduledelayedtriggereffect")
            && debug.contains("putcounterseffect"),
        "expected Sand Golem to keep caused-discard trigger and delayed return counter sequence, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn nissas_encouragement_keeps_three_exact_named_searches() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Nissa's Encouragement Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Search your library and graveyard for a card named Forest, a card named Brambleweft Behemoth, and a card named Nissa, Genesis Mage. Reveal those cards, put them into your hand, then shuffle.",
        )
        .expect("Nissa's Encouragement should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("forest")
            && rendered.contains("brambleweft behemoth")
            && rendered.contains("nissa, genesis mage")
            && rendered.contains("reveal those cards")
            && rendered.contains("put them into your hand")
            && rendered.contains("then shuffle"),
        "expected Nissa's Encouragement to keep the three exact named searches, got {rendered}"
    );

    let debug = format!("{:?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        debug.contains("searchlibraryslotseffect")
            && debug.contains("forest")
            && debug.contains("brambleweft behemoth")
            && debug.contains("nissa, genesis mage")
            && debug.contains("destination: hand")
            && debug.contains("reveal: true"),
        "expected Nissa's Encouragement to compile as one typed slot search, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn reveka_activation_keeps_self_skip_next_untap_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Reveka Variant")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Dwarf, Subtype::Wizard])
        .supertypes(vec![Supertype::Legendary])
        .power_toughness(PowerToughness::fixed(0, 1))
        .parse_text("{T}: Reveka deals 2 damage to any target and doesn't untap during your next untap step.")
        .expect("Reveka should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("deals 2 damage to any target")
            && rendered.contains("doesn't untap during your next untap step"),
        "expected Reveka to keep self-untap restriction wording, got {rendered}"
    );

    let debug = format!("{:?}", def.abilities).to_ascii_lowercase();
    assert!(
        debug.contains("controllersnextuntapstep")
            && debug.contains("source: true")
            && debug.contains("dealdamageeffect"),
        "expected Reveka to keep its damage plus next-untap restriction structure, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn reveka_activation_runtime_keeps_source_tapped_for_next_untap() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Reveka Variant")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Dwarf, Subtype::Wizard])
        .supertypes(vec![Supertype::Legendary])
        .power_toughness(PowerToughness::fixed(0, 1))
        .parse_text(
            "{T}: Reveka deals 2 damage to any target and doesn't untap during your next untap step.",
        )
        .expect("Reveka should parse");
    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Reveka should have an activated ability");
    let cant_effect = activated.effects.segments[0]
        .default_effects
        .iter()
        .find(|effect| {
            effect
                .downcast_ref::<crate::effects::CantEffect>()
                .is_some()
        })
        .expect("Reveka activation should include a next-untap restriction");

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let reveka_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.tap(reveka_id);

    let mut ctx = crate::effects::ExecutionContext::new_default(reveka_id, alice);
    cant_effect
        .0
        .execute(&mut game, &mut ctx)
        .expect("Reveka next-untap restriction should resolve");

    game.turn.active_player = alice;
    game.turn.phase = crate::game_state::Phase::Beginning;
    game.turn.step = Some(crate::game_state::Step::Untap);
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    crate::turn::execute_untap_step_with(&mut game, &mut dm);

    assert!(
        game.is_tapped(reveka_id),
        "Reveka should remain tapped during its controller's next untap step"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn mad_dog_trigger_checks_attack_or_entered_this_turn_before_sacrificing() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mad Dog Variant")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Dog])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "At the beginning of your end step, if this creature didn't attack or come under your control this turn, sacrifice it.",
        )
        .expect("Mad Dog should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("if this creature didn't attack or come under your control this turn")
            && (rendered.contains("sacrifice it") || rendered.contains("sacrifice this creature")),
        "expected Mad Dog to preserve its conditional self-sacrifice wording, got {rendered}"
    );

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Mad Dog should have a triggered ability");
    let condition = triggered
        .intervening_if
        .as_ref()
        .expect("Mad Dog trigger should have an intervening condition");
    let effects_debug = format!("{:?}", triggered.effects);
    assert!(
        effects_debug.contains("SacrificeTargetEffect") && effects_debug.contains("Source"),
        "expected Mad Dog to sacrifice itself without chooser scaffolding, got {effects_debug}"
    );
    assert!(
        triggered.choices.is_empty(),
        "Mad Dog should not leave any chooser scaffolding, got {:?}",
        triggered.choices
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let old_mad_dog = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.turn_store.turn_history.clear_for_new_turn();
    let ctx = crate::effects::ExecutionContext::new_default(old_mad_dog, alice);
    assert!(
        crate::condition_eval::evaluate_condition_resolution(&game, condition, &ctx)
            .expect("Mad Dog condition should evaluate"),
        "Mad Dog should sacrifice when it neither attacked nor entered this turn"
    );

    game.mark_creature_attacked_this_turn(old_mad_dog);
    assert!(
        !crate::condition_eval::evaluate_condition_resolution(&game, condition, &ctx)
            .expect("Mad Dog attacked condition should evaluate"),
        "Mad Dog should not sacrifice after it attacked this turn"
    );

    game.turn_store.turn_history.clear_for_new_turn();
    let new_mad_dog = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let new_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(new_mad_dog)
            .expect("new Mad Dog should exist on the battlefield"),
        &game,
    );
    let entry_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::with_cause(
            new_mad_dog,
            Zone::Hand,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            Some(new_snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    game.record_turn_history_event(&entry_event);
    let new_ctx = crate::effects::ExecutionContext::new_default(new_mad_dog, alice);
    assert!(
        !crate::condition_eval::evaluate_condition_resolution(&game, condition, &new_ctx)
            .expect("Mad Dog entered condition should evaluate"),
        "Mad Dog should not sacrifice if it came under your control this turn"
    );

    game.turn_store.turn_history.clear_for_new_turn();
    let stolen_mad_dog = game.create_object_from_definition(&def, bob, Zone::Battlefield);
    let mut control_ctx = crate::effects::ExecutionContext::new_default(old_mad_dog, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(stolen_mad_dog)]);
    let control_outcome =
        crate::effects::control::GainControlEffect::until_end_of_turn(ChooseSpec::creature())
            .execute(&mut game, &mut control_ctx)
            .expect("gain-control effect should resolve");
    for event in &control_outcome.events {
        game.record_turn_history_event(event);
    }
    assert_eq!(
        game.current_controller(stolen_mad_dog),
        Some(alice),
        "Alice should control the stolen Mad Dog after the control-change effect"
    );
    let stolen_ctx = crate::effects::ExecutionContext::new_default(stolen_mad_dog, alice);
    assert!(
        !crate::condition_eval::evaluate_condition_resolution(&game, condition, &stolen_ctx)
            .expect("Mad Dog control-change condition should evaluate"),
        "Mad Dog should not sacrifice if it came under your control through a control-change effect this turn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn source_scoped_intervening_if_binds_bare_it_sacrifice_to_source() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Tapped Self Sacrifice Variant")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text("At the beginning of your upkeep, if this creature is tapped, sacrifice it.")
        .expect("source-scoped self-sacrifice trigger should parse");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("variant should have a triggered ability");
    let debug = format!("{:?}", triggered.effects);
    assert!(
        debug.contains("SacrificeTargetEffect") && debug.contains("Source"),
        "expected bare it under source-scoped condition to sacrifice the source, got {debug}"
    );
    assert!(
        triggered.choices.is_empty(),
        "source-scoped self-sacrifice should not require a chooser, got {:?}",
        triggered.choices
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn source_scoped_choose_then_sacrifice_it_keeps_chosen_object() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Chosen Sacrifice Variant")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "At the beginning of your upkeep, if this creature is tapped, choose a creature you control. Sacrifice it.",
        )
        .expect("source-scoped choose/sacrifice trigger should parse");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("variant should have a triggered ability");
    let debug = format!("{:?}", triggered.effects);
    assert!(
        debug.contains("ChooseObjectsEffect")
            && debug.contains("SacrificeTargetEffect")
            && debug.contains("Tagged"),
        "expected choose/sacrifice sequence to stay object-scoped, got {debug}"
    );
    assert!(
        !(debug.contains("SacrificeTargetEffect") && debug.contains("Source")),
        "source-scoped choose/sacrifice should not collapse to sacrificing the source, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_if_you_do_search_library_clause_keeps_full_tail_effect() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Blood Speaker Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "At the beginning of your upkeep, you may sacrifice this creature. If you do, search your library for a Demon card, reveal that card, put it into your hand, then shuffle.",
        )
        .expect("if-you-do search clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("if you do")
            && rendered.contains("search your library for a demon card")
            && rendered.contains("put it into your hand")
            && rendered.contains("shuffle"),
        "expected if-you-do gate plus full search/reveal/put/shuffle tail, got {rendered}"
    );
    assert!(
        rendered.contains("search your library for a demon card")
            && rendered.contains("put it into your hand")
            && rendered.contains("shuffle"),
        "expected full search/reveal/put/shuffle tail to remain after if-you-do split, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_trigger_with_comma_separated_list_does_not_split_early() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sram Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Whenever you cast an Aura, Equipment, or Vehicle spell, draw a card.")
        .expect("comma-separated trigger list should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("aura") && rendered.contains("equipment") && rendered.contains("vehicle"),
        "expected trigger list to include aura/equipment/vehicle, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_trigger_with_and_or_subtype_list_keeps_effect_split_on_trigger_delimiter() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Vaan Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Whenever one or more Scouts, Pirates, and/or Rogues you control deal combat damage to a player, exile the top card of that player's library. You may cast it. If you don't, create a Treasure token.")
        .expect("and/or subtype trigger list should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("exile the top card of that player's library")
            && rendered.contains("you may cast it")
            && rendered.contains("create a treasure token"),
        "expected exile/create sequence to remain on the triggered effect, got {rendered}"
    );

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        !abilities_debug.contains("unimplemented_trigger"),
        "expected no fallback custom trigger for and/or subtype list, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_other_mice_anthem_renders_irregular_plural() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mabel Anthem Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Other Mice you control get +1/+1.")
        .expect("mice anthem should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("other mice you control get +1/+1"),
        "expected irregular 'mice' plural in rendered anthem, got {rendered}"
    );
    assert!(
        !rendered.contains("mouses"),
        "expected not to render as 'mouses', got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_mabel_token_preserves_colorless_and_equipment_payload() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mabel Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When Mabel enters, create Cragflame, a legendary colorless Equipment artifact token with \"Equipped creature gets +1/+1 and has vigilance, trample, and haste\" and equip {2}.",
        )
        .expect("Mabel token payload should parse");
    let rendered = format!("{def:?}").to_ascii_lowercase();
    assert!(
        rendered.contains("name: \"cragflame\"")
            && rendered.contains("makecolorless")
            && rendered.contains("equipment")
            && rendered.contains("attachtoeffect"),
        "expected parsed Mabel token payload, got {rendered}"
    );

    let rendered_text = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered_text.contains(
            "create Cragflame, a legendary colorless Equipment artifact token with \"Equipped creature gets +1/+1 and has vigilance, trample, and haste\" and equip {2}"
        ),
        "expected Mabel token payload to render as a compact Equipment token ability, got {rendered_text}"
    );
}

#[test]
pub(super) fn parse_toggo_token_preserves_named_rock_and_activated_payload() {
    let def = parse_oracle_card_definition("Toggo, Goblin Weaponsmith");
    let rendered = format!("{def:?}").to_ascii_lowercase();
    assert!(
        rendered.contains("name: \"rock\"")
            && rendered.contains("attachedabilitygrant")
            && rendered.contains("sacrificeeffect")
            && rendered.contains("dealdamageeffect")
            && rendered.contains("attachtoeffect"),
        "expected Toggo's Rock token payload to preserve its named activated ability, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_that_creature_gets_and_gains_uses_single_tagged_target() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ogre Battledriver Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever another creature you control enters, that creature gets +2/+0 and gains haste until end of turn.",
        )
        .expect("that-creature gets-and-gains clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("that creature gets +2/+0") || rendered.contains("it gets +2/+0"),
        "expected pump to stay on the single triggering creature, got {rendered}"
    );
    assert!(
        !rendered.contains("creatures get +2/+0"),
        "expected not to broaden to all creatures, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_the_player_may_clause_preserves_that_player_decider() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Gate Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "At the beginning of each player's upkeep, that player reveals the top card of their library. If it's an artifact, creature, enchantment, or land card, the player may put it onto the battlefield.",
        )
        .expect("the-player-may conditional clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("that player may put it onto the battlefield"),
        "expected 'that player may' to be preserved, got {rendered}"
    );
    assert!(
        !rendered.contains("you may put it onto the battlefield"),
        "expected decision actor not to collapse to source controller, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_return_with_multiple_counters_on_it_sentence() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Perennation Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Return target permanent card from your graveyard to the battlefield with a hexproof counter and an indestructible counter on it.",
        )
        .expect("return-with-multiple-counters clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("return target")
            && rendered.contains("battlefield")
            && rendered.contains("hexproof counter")
            && rendered.contains("indestructible counter"),
        "expected returned permanent to receive both counters, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_sacrifice_any_number_sentence_keeps_open_count() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Landslide Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Sacrifice any number of Mountains. Landslide deals that much damage to target player or planeswalker.",
        )
        .expect("sacrifice-any-number clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    let debug = format!("{:?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        rendered.contains("any number"),
        "expected sacrifice count to remain open-ended, got {rendered}"
    );
    assert!(
        debug.contains("choicecount { min: 0, max: none")
            && (debug.contains("sacrificeeffect") || debug.contains("sacrificeplayereffect"))
            && debug.contains("count(objectfilter")
            && (debug.contains("tagkey(\"sacrificed_") || debug.contains("tagkey(\"__it__\")")),
        "expected the any-number sacrifice to keep a tagged choose-and-sacrifice chain, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_gain_ability_until_next_upkeep_uses_non_end_of_turn_duration() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Erhnam Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "At the beginning of your upkeep, target non-Wall creature an opponent controls gains forestwalk until your next upkeep.",
        )
        .expect("gain-until-next-upkeep clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("until your next"),
        "expected duration to remain next-turn scoped, got {rendered}"
    );
    assert!(
        !rendered.contains("until end of turn"),
        "expected duration not to collapse to end of turn, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_minion_reflector_copy_clause_keeps_haste_and_end_step_sacrifice() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Minion Reflector Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Whenever a nontoken creature you control enters, you may pay {2}. If you do, create a token that's a copy of that creature, except it has haste and \"At the beginning of the end step, sacrifice this permanent.\"",
        )
        .expect("copy-with-inline-haste-and-end-step clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("copy of it"),
        "expected token-copy clause to remain present, got {rendered}"
    );
    assert!(
        rendered.contains("haste"),
        "expected haste modifier to remain present, got {rendered}"
    );
    assert!(
        rendered.contains("next end step") || rendered.contains("the end step"),
        "expected delayed end-step sacrifice clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_not_dead_after_all_keeps_role_creation_and_attachment_in_granted_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Not Dead After All Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Until end of turn, target creature you control gains \"When this creature dies, return it to the battlefield tapped under its owner's control, then create a Wicked Role token attached to it.\"",
        )
        .expect("wicked-role-on-return clause should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("CreateTokenEffect"),
        "expected granted trigger to keep Wicked Role token creation, got {debug}"
    );
    assert!(
        debug.contains("AttachObjectsEffect"),
        "expected created role token to be attached, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_face_down_target_filter_for_destroy_effect() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Nosy Goblin Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}, Sacrifice this creature: Destroy target face-down creature.")
        .expect("face-down target destroy should parse");

    let debug = format!("{def:?}");
    assert!(
        debug.contains("face_down: Some") && debug.contains("true"),
        "expected face-down target filter, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_face_down_static_anthem_filter() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Secret Plans Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Face-down creatures you control get +0/+1.\nWhenever a permanent you control is turned face up, draw a card.",
        )
        .expect("face-down anthem line should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("face-down creatures you control get +0/+1"),
        "expected face-down qualifier preserved on anthem, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_you_may_look_at_face_down_creatures_you_dont_control_any_time() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Keeper of the Lens Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("You may look at face-down creatures you don't control any time.")
        .expect("face-down visibility static line should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("you may look at face-down creatures you don't control any time"),
        "expected face-down visibility line to be preserved, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_player_sacrifices_trigger_preserves_another_qualifier() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Furnace Celebration Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever you sacrifice another permanent, you may pay {2}. If you do, this enchantment deals 2 damage to any target.",
        )
        .expect("player-sacrifices trigger with another qualifier should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("other: true"),
        "expected sacrifice trigger filter to keep 'another' qualifier, got {abilities_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("sacrifice another permanent"),
        "expected rendered trigger to preserve 'another permanent', got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_rhystic_lightning_unless_payment_then_reduced_damage() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Rhystic Lightning Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "This spell deals 4 damage to any target unless that permanent's controller or that player pays {2}. If they do, this spell deals 2 damage to the permanent or player.",
        )
        .expect("rhystic unless-payment clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("unless") && rendered.contains("pays {2}"),
        "expected unless-payment branch to remain explicit, got {rendered}"
    );
    assert!(
        (rendered.contains("if they do")
            || rendered.contains("if that doesn't happen")
            || rendered.contains("if that doesnt happen"))
            && rendered.contains("deal 2 damage"),
        "expected reduced-damage paid branch to remain explicit, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_slivercycling_grant_clause_as_static_grant_not_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Homing Grant Variant")
        .parse_text("Each Sliver card in each player's hand has slivercycling {3}.")
        .expect("slivercycling grant clause should parse as a static grant ability");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("sliver cards") && rendered.contains("have \"slivercycling {3}"),
        "expected rendered static grant clause, got {rendered}"
    );
    assert!(
        !rendered.starts_with("keyword ability 1: slivercycling {3}."),
        "expected no standalone keyword-only parse for grant clause, got {rendered}"
    );
}

#[test]
pub(super) fn goddric_cloaked_reveler_keeps_conditional_dragon_animation() {
    let def = parse_oracle_card_definition("Goddric, Cloaked Reveler");
    let abilities_debug = format!("{:#?}", def.abilities);
    let oracle_like = unprocessed_compiled_lines(&def);

    assert!(
        !abilities_debug.contains("KeywordFallbackText")
            && !abilities_debug.contains("RuleFallbackText")
            && !abilities_debug.contains("UnsupportedParserLine"),
        "expected no fallback static abilities, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("CountComparison"),
        "expected celebration count comparison condition, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("SetCreatureSubtypes"),
        "expected creature-type replacement effect, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("SetBasePowerToughnessForFilter"),
        "expected base power/toughness setter, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("GrantObjectAbilityForFilter"),
        "expected granted activated ability effect, got {abilities_debug}"
    );
    assert!(
        abilities_debug.to_ascii_lowercase().contains("flying"),
        "expected flying grant in lowered abilities, got {abilities_debug}"
    );

    let rendered = oracle_like.join(" ").to_ascii_lowercase();
    assert!(
        rendered.contains("as long as two or more nonland permanents entered the battlefield under your control this turn"),
        "expected celebration condition in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains("dragon")
            && rendered.contains("base power and toughness 4/4")
            && rendered.contains("flying"),
        "expected dragon animation payload in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains("dragons you control get +1/+0 until end of turn"),
        "expected granted activated ability payload in compiled text, got {rendered}"
    );
    assert!(
        !rendered.contains("loses all other creature types"),
        "expected stripped reminder text to omit subtype replacement note, got {rendered}"
    );
    let celebration_lines = oracle_like
        .iter()
        .filter(|line| {
            let lower = line.to_ascii_lowercase();
            lower.contains("two or more nonland permanents entered the battlefield under your control this turn")
                && lower.contains("dragons you control get +1/+0 until end of turn")
        })
        .count();
    assert_eq!(
        celebration_lines, 1,
        "expected Goddric celebration text to render once, got {oracle_like:#?}"
    );

    let (_oracle_cov, _compiled_cov, similarity, _delta, _mismatch) =
        crate::semantic_compare::compare_card_semantics_scored(
            "Goddric, Cloaked Reveler",
            &crate::compiled_text::debug_compiled_lines(&def).join("\n"),
            &oracle_like,
            crate::semantic_compare::report_embedding_config(),
        );
    assert!(
        similarity >= 0.68,
        "expected Goddric to reach oracle-like similarity, got {similarity}"
    );
    assert!(
        !crate::cards::generated_definition_has_unimplemented_content(&def),
        "expected Goddric to compile without unimplemented fallback content: {abilities_debug}"
    );
}

pub(super) fn herald_of_leshrac_definition() -> CardDefinition {
    let oracle = oracle_text_by_name()
        .get("Herald of Leshrac")
        .expect("missing Herald of Leshrac oracle text")
        .clone();
    CardDefinitionBuilder::new(CardId::new(), "Herald of Leshrac")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(6)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Avatar])
        .power_toughness(PowerToughness::fixed(2, 4))
        .parse_text(oracle)
        .expect("Herald of Leshrac should parse strictly")
}

pub(super) struct PayCumulativeUpkeep;

impl crate::decision::DecisionMaker for PayCumulativeUpkeep {
    fn decide_boolean(
        &mut self,
        _game: &crate::game_state::GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        true
    }
}

pub(super) fn herald_upkeep_trigger(def: &CardDefinition) -> &crate::ability::TriggeredAbility {
    def.abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if format!("{triggered:?}").contains("CumulativeUpkeepEffect") =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Herald of Leshrac should have a cumulative upkeep trigger")
}

pub(super) fn herald_leaves_trigger(def: &CardDefinition) -> &crate::ability::TriggeredAbility {
    def.abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if format!("{:?}", triggered.effects)
                    .contains("ChangeControllerToPlayer(IteratedPlayer)") =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Herald of Leshrac should have a leaves-the-battlefield trigger")
}

pub(super) fn test_land_definition(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Land])
        .build()
}

pub(super) fn execute_herald_upkeep(
    game: &mut crate::game_state::GameState,
    herald: crate::ids::ObjectId,
    upkeep: &crate::ability::TriggeredAbility,
    controller: PlayerId,
) {
    let mut dm = PayCumulativeUpkeep;
    let mut ctx = crate::effects::ExecutionContext::new(herald, controller, &mut dm);
    crate::game_loop::execute_resolution_program(
        game,
        &mut ctx,
        controller,
        herald,
        &upkeep.effects,
        None,
        &[],
    )
    .expect("Herald cumulative upkeep should resolve");
}

#[test]
pub(super) fn herald_of_leshrac_strict_parser_and_compiled_text_regression() {
    let def = herald_of_leshrac_definition();
    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("CumulativeUpkeepEffect")
            && abilities_debug.contains("ChangeControllerToEffectController")
            && abilities_debug.contains("ChangeControllerToPlayer(IteratedPlayer)"),
        "expected Herald control-changing cumulative upkeep and leaves trigger, got {abilities_debug}"
    );
    assert!(
        !crate::cards::generated_definition_has_unimplemented_content(&def),
        "Herald of Leshrac should not contain unsupported markers: {abilities_debug}"
    );

    let rendered_lines = unprocessed_compiled_lines(&def);
    let rendered = rendered_lines.join("\n");
    assert!(
        rendered.contains("Cumulative upkeep—Gain control of a land you don't control."),
        "expected cumulative upkeep control cost in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains("This creature gets +1/+1 for each land you control but don't own."),
        "expected unowned-land scaling text, got {rendered}"
    );
    assert!(
        rendered.contains("When this creature leaves the battlefield, each player gains control of each land they own that you control."),
        "expected owner-restoration leaves trigger text, got {rendered}"
    );

    let oracle = oracle_text_by_name()
        .get("Herald of Leshrac")
        .expect("missing Herald oracle text");
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        crate::semantic_compare::compare_card_semantics_scored(
            "Herald of Leshrac",
            oracle,
            &rendered_lines,
            crate::semantic_compare::report_embedding_config(),
        );
    assert!(
        similarity >= 0.99 && !mismatch,
        "expected Herald compiled text to match oracle, got similarity={similarity}, mismatch={mismatch}, text={rendered}"
    );
}

#[test]
pub(super) fn herald_of_leshrac_cumulative_upkeep_gains_land_and_scales_power() {
    let def = herald_of_leshrac_definition();
    let upkeep = herald_upkeep_trigger(&def);
    let land_def = test_land_definition("Stolen Land");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let herald = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let bob_land = game.create_object_from_definition(&land_def, bob, Zone::Battlefield);

    execute_herald_upkeep(&mut game, herald, upkeep, alice);

    assert_eq!(
        game.counter_count(herald, crate::object::CounterType::Age),
        1,
        "cumulative upkeep should add one age counter"
    );
    assert_eq!(
        game.current_controller(bob_land),
        Some(alice),
        "paying Herald's upkeep should gain control of a land you don't control"
    );
    assert_eq!(
        game.calculated_power(herald),
        Some(3),
        "Herald should get +1/+1 for the land Alice controls but doesn't own"
    );
    assert_eq!(game.calculated_toughness(herald), Some(5));
}

#[test]
pub(super) fn herald_of_leshrac_repeated_cumulative_upkeep_pays_once_per_age_counter() {
    let def = herald_of_leshrac_definition();
    let upkeep = herald_upkeep_trigger(&def);
    let land_def = test_land_definition("Stolen Land");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let herald = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let bob_land_one = game.create_object_from_definition(&land_def, bob, Zone::Battlefield);
    let bob_land_two = game.create_object_from_definition(&land_def, bob, Zone::Battlefield);
    let bob_land_three = game.create_object_from_definition(&land_def, bob, Zone::Battlefield);

    execute_herald_upkeep(&mut game, herald, upkeep, alice);
    execute_herald_upkeep(&mut game, herald, upkeep, alice);

    assert_eq!(
        game.counter_count(herald, crate::object::CounterType::Age),
        2,
        "second cumulative upkeep should leave two age counters"
    );
    for land in [bob_land_one, bob_land_two, bob_land_three] {
        assert_eq!(
            game.current_controller(land),
            Some(alice),
            "two age counters should require gaining control of two additional lands"
        );
    }
    assert_eq!(
        game.calculated_power(herald),
        Some(5),
        "Herald should count every land Alice controls but doesn't own"
    );
    assert_eq!(game.calculated_toughness(herald), Some(7));
}

#[test]
pub(super) fn herald_of_leshrac_repeated_cumulative_upkeep_fails_atomically_when_short_on_lands() {
    let def = herald_of_leshrac_definition();
    let upkeep = herald_upkeep_trigger(&def);
    let land_def = test_land_definition("Stolen Land");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let herald = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let first_land = game.create_object_from_definition(&land_def, bob, Zone::Battlefield);
    let only_remaining_land = game.create_object_from_definition(&land_def, bob, Zone::Battlefield);

    execute_herald_upkeep(&mut game, herald, upkeep, alice);
    assert_eq!(game.current_controller(first_land), Some(alice));

    execute_herald_upkeep(&mut game, herald, upkeep, alice);

    assert!(
        !game.battlefield.contains(&herald),
        "Herald should be sacrificed when the full repeated upkeep cost cannot be paid"
    );
    assert_eq!(
        game.current_controller(only_remaining_land),
        Some(bob),
        "failed repeated upkeep should not keep a partial controller-change payment"
    );
}

#[test]
pub(super) fn herald_of_leshrac_cumulative_upkeep_sacrifices_when_no_land_can_be_gained() {
    let def = herald_of_leshrac_definition();
    let upkeep = herald_upkeep_trigger(&def);

    let alice = PlayerId::from_index(0);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let herald = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    execute_herald_upkeep(&mut game, herald, upkeep, alice);

    assert!(
        !game.battlefield.contains(&herald),
        "Herald should leave the battlefield when its cumulative upkeep cost cannot be paid"
    );
    let graveyard_object = game
        .player(alice)
        .and_then(|player| player.graveyard.first().copied())
        .and_then(|id| game.object(id));
    assert_eq!(
        graveyard_object.map(|object| object.name.as_str()),
        Some("Herald of Leshrac"),
        "Herald should be sacrificed to its owner's graveyard when its cumulative upkeep cost cannot be paid"
    );
}

#[test]
pub(super) fn herald_of_leshrac_leaves_trigger_returns_lands_to_their_owners() {
    let def = herald_of_leshrac_definition();
    let leaves = herald_leaves_trigger(&def);
    let land_def = test_land_definition("Recovered Land");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let herald = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let alice_land = game.create_object_from_definition(&land_def, alice, Zone::Battlefield);
    let bob_land = game.create_object_from_definition(&land_def, bob, Zone::Battlefield);
    game.set_current_controller(bob_land, alice);
    assert_eq!(game.current_controller(bob_land), Some(alice));

    let mut dm = PayCumulativeUpkeep;
    let mut ctx = crate::effects::ExecutionContext::new(herald, alice, &mut dm);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        herald,
        &leaves.effects,
        None,
        &[],
    )
    .expect("Herald leaves trigger should resolve");

    assert_eq!(
        game.current_controller(bob_land),
        Some(bob),
        "Bob should regain each land they own that Alice controls"
    );
    assert_eq!(
        game.current_controller(alice_land),
        Some(alice),
        "Alice should retain lands they already own and control"
    );
}

#[test]
pub(super) fn bello_bard_of_the_brambles_compacts_conditional_animation_bundle() {
    let def = parse_oracle_card_definition("Bello, Bard of the Brambles");
    let lines = unprocessed_compiled_lines(&def);
    let rendered = lines.join(" ");
    let lower = rendered.to_ascii_lowercase();

    assert!(
        !lower.contains("non-auran"),
        "non-Aura should not be corrupted by article cleanup, got {rendered}"
    );
    assert!(
        lower.contains("during your turn")
            && lower
                .contains("each non-equipment artifact and non-aura enchantment you control with mana value 4 or greater"),
        "expected Bello's filtered turn condition in one rendered surface, got {rendered}"
    );
    assert!(
        lower.contains("4/4 elemental creature")
            && lower.contains("indestructible")
            && lower.contains("haste")
            && lower
                .contains("whenever this creature deals combat damage to a player, draw a card"),
        "expected Bello's animation payload and granted abilities to stay bundled, got {rendered}"
    );

    let animation_lines = lines
        .iter()
        .filter(|line| {
            let lower = line.to_ascii_lowercase();
            lower.contains("4/4 elemental creature")
                && lower.contains("indestructible")
                && lower.contains("haste")
                && lower.contains("combat damage to a player")
        })
        .count();
    assert_eq!(
        animation_lines, 1,
        "expected one compact Bello animation line, got {lines:#?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_gideon_planeswalker_predicate_keeps_subtype_constraint() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Gideon Predicate Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Exile target white creature that's attacking or blocking. If it was a Gideon planeswalker, you gain 5 life.")
        .expect("gideon-planeswalker predicate should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("gideon"),
        "expected subtype constraint to remain in rendered predicate, got {rendered}"
    );
    assert!(
        rendered.contains("planeswalker"),
        "expected planeswalker card type to remain in rendered predicate, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_permanent_card_target_in_graveyard_sets_permanent_card_types() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Jailbreak Permanent Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Return target permanent card in an opponent's graveyard to the battlefield under their control.",
        )
        .expect("permanent-card graveyard target should parse");

    let spell_debug = format!("{:#?}", def.spell_effect);
    assert!(
        spell_debug.contains("Artifact")
            && spell_debug.contains("Creature")
            && spell_debug.contains("Enchantment")
            && spell_debug.contains("Land")
            && spell_debug.contains("Planeswalker")
            && spell_debug.contains("Battle"),
        "expected permanent card-type expansion for graveyard target, got {spell_debug}"
    );
    assert!(
        !spell_debug.contains("Instant") && !spell_debug.contains("Sorcery"),
        "expected nonpermanent types to stay excluded, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_one_or_more_subject_with_attack_verb_is_not_custom_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "One Or More Attack Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Whenever one or more Phyrexians you control attack, draw a card.")
        .expect("one-or-more attack trigger should parse as attacks trigger");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        !abilities_debug.contains("unimplemented_trigger"),
        "expected no fallback custom trigger for singular 'attack' wording, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_one_or_more_attack_trigger_preserves_one_or_more_compiled_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "One Or More Attack Render Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Whenever one or more Phyrexians you control attack, draw a card.")
        .expect("one-or-more attack trigger should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("one or more phyrexian")
            && rendered.contains("you control")
            && rendered.contains("attack"),
        "expected one-or-more attack wording to remain explicit, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_mount_or_vehicle_attack_trigger_keeps_both_subjects() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mount Vehicle Attack Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Whenever a Mount or Vehicle you control attacks, draw a card.")
        .expect("mount-or-vehicle attack trigger should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        !abilities_debug.contains("unimplemented_trigger"),
        "expected no fallback custom trigger for mount-or-vehicle attack clause, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("Mount") && abilities_debug.contains("Vehicle"),
        "expected both subtypes in attack trigger filter, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_one_or_more_enters_trigger_uses_batch_count_mode() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "One Or More Enter Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever one or more tokens you control enter, put a +1/+1 counter on this creature.",
        )
        .expect("one-or-more enters trigger should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("count_mode: OneOrMore"),
        "expected ETB trigger to compile in one-or-more mode, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_due_respect_variant_renders_permanents_enter_tapped_compactly() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Due Respect Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Permanents enter tapped this turn.\nDraw a card.")
        .expect("due-respect style line should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("permanents enter tapped"),
        "expected compact permanent enter-tapped wording, got {rendered}"
    );
    assert!(
        !rendered.contains("artifacts, creatures, enchantments, lands, planeswalkers, and battles"),
        "expected no expanded permanent type list in enter-tapped wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_creatures_entering_dont_trigger_static_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Torpor Orb Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text("Creatures entering don't cause abilities to trigger.")
        .expect("torpor-orb static line should parse");

    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        static_ids.contains(
            &crate::static_abilities::StaticAbilityId::CreaturesEnteringDontCauseAbilitiesToTrigger
        ),
        "expected ETB trigger suppression static ability, got {static_ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_serum_powder_compiles_to_mulligan_redraw_pregame_action() {
    let def = parse_oracle_card_definition("Serum Powder");
    let static_abilities: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability),
            _ => None,
        })
        .collect();

    assert!(
        static_abilities.iter().any(|ability| matches!(
            ability.pregame_action_kind(),
            Some(crate::static_abilities::PregameActionKind::MulliganExileHandDrawSameCount)
        )),
        "expected oracle Serum Powder to compile to mulligan redraw pregame action, got {static_abilities:#?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_clara_oswald_full_text_compiles() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Clara Oswald")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Impossible Girl - If Clara Oswald is your commander, choose a color before the game begins.\nClara Oswald is the chosen color.\nIf a triggered ability of a Doctor you control triggers, that ability triggers an additional time.\nDoctor's companion (You can have two commanders if the other is the Doctor.)",
        )
        .expect("Clara Oswald should compile");

    let static_abilities: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.clone()),
            _ => None,
        })
        .collect();
    let ids: Vec<_> = static_abilities
        .iter()
        .map(|ability| ability.id())
        .collect();
    assert!(
        ids.contains(&StaticAbilityId::PregameAction),
        "expected pregame choose-color action, got {ids:?}"
    );
    assert!(
        static_abilities.iter().any(|ability| matches!(
            ability.pregame_action_kind(),
            Some(crate::static_abilities::PregameActionKind::ChooseColor)
        )),
        "expected typed choose-color pregame action, got {static_abilities:#?}"
    );
    assert!(
        ids.contains(&StaticAbilityId::SetChosenColor),
        "expected chosen-color static ability, got {ids:?}"
    );
    assert!(
        ids.contains(&StaticAbilityId::DuplicateMatchingTriggeredAbilities),
        "expected trigger duplication ability, got {ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_clara_oswald_combined_pregame_line_compiles() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Clara Oswald")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Impossible Girl - If Clara Oswald is your commander, choose a color before the game begins. Clara Oswald is the chosen color.\nIf a triggered ability of a Doctor you control triggers, that ability triggers an additional time.\nDoctor's companion (You can have two commanders if the other is the Doctor.)",
        )
        .expect("Clara Oswald combined pregame line should compile");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered
            .to_ascii_lowercase()
            .contains("choose a color before the game begins"),
        "expected pregame choose-color text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_elesh_norn_full_text_compiles_with_generic_trigger_suppression() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Elesh Norn, Mother of Machines")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Vigilance\nIf a permanent entering causes a triggered ability of a permanent you control to trigger, that ability triggers an additional time.\nPermanents entering don't cause abilities of permanents your opponents control to trigger.",
        )
        .expect("Elesh Norn should compile");

    let abilities_debug = format!("{:#?}", def.abilities);
    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        static_ids.contains(&StaticAbilityId::DuplicateMatchingTriggeredAbilities),
        "expected ETB trigger duplication ability, got {static_ids:?}"
    );
    assert!(
        static_ids.contains(&StaticAbilityId::SuppressMatchingTriggeredAbilities),
        "expected generic trigger suppression ability, got {static_ids:?}"
    );
    assert!(
        abilities_debug.contains("Opponent"),
        "expected suppression source filter to target opponents, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_vantress_visions_copy_triggered_ability_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Vantress Visions")
        .card_types(vec![CardType::Instant])
        .parse_text("Copy target activated or triggered ability you control. You may choose new targets for the copy.")
        .expect("Vantress Visions copy clause should compile");

    let spell_debug = format!("{:#?}", def.spell_effect);
    assert!(
        spell_debug.contains("CopySpellEffect"),
        "expected ability-copy clause to lower through CopySpellEffect, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("is_ability: true") || spell_debug.contains("TriggeredAbility"),
        "expected copied target to remain ability-shaped, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) const CHANDRAS_REGULATOR_TEXT: &str = "Whenever you activate a loyalty ability of a Chandra planeswalker, you may pay {1}. If you do, copy that ability. You may choose new targets for the copy.\n{1}, {T}, Discard a Mountain card or a red card: Draw a card.";

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn chandras_regulator_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(1), "Chandra's Regulator")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Red],
        ]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Artifact])
        .parse_text(CHANDRAS_REGULATOR_TEXT)
        .expect("Chandra's Regulator should parse strictly")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_chandras_regulator_strict_and_render_oracle_text() {
    let def = chandras_regulator_definition();
    let rendered = unprocessed_compiled_lines(&def).join("\n");

    assert_eq!(rendered, CHANDRAS_REGULATOR_TEXT);
    assert!(
        !crate::cards::generated_definition_has_unimplemented_content(&def),
        "Chandra's Regulator should not contain unsupported runtime markers: {def:#?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_chandras_regulator_models_loyalty_copy_and_filtered_discard_cost() {
    let def = chandras_regulator_definition();
    let debug = format!("{def:#?}");

    assert!(
        debug.contains("AbilityActivatedTrigger")
            && debug.contains("loyalty_only: true")
            && debug.contains("Chandra")
            && debug.contains("Planeswalker"),
        "expected Chandra's Regulator to trigger on Chandra loyalty abilities, got {debug}"
    );
    assert!(
        debug.contains("MayEffect")
            && debug.contains("PayManaEffect")
            && debug.contains("IfEffect")
            && debug.contains("CopySpellEffect")
            && debug.contains("triggering_source")
            && (debug.contains("ChooseNewTargets") || debug.contains("RetargetStackObjectEffect")),
        "expected optional-pay copy and retarget effects, got {debug}"
    );
    assert!(
        debug.contains("DiscardEffect")
            && debug.contains("Mountain")
            && debug.contains("ColorSet(")
            && debug.contains("any_of"),
        "expected discard cost to allow a Mountain card or a red card, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn chandras_regulator_runtime_triggers_only_for_chandra_loyalty_abilities() {
    let regulator = chandras_regulator_definition();
    let chandra = CardDefinitionBuilder::new(CardId::from_raw(2), "Chandra Test Walker")
        .card_types(vec![CardType::Planeswalker])
        .subtypes(vec![Subtype::Chandra])
        .build();
    let other_planeswalker = CardDefinitionBuilder::new(CardId::from_raw(3), "Other Test Walker")
        .card_types(vec![CardType::Planeswalker])
        .build();

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let regulator_id = game.create_object_from_definition(&regulator, alice, Zone::Battlefield);
    let chandra_id = game.create_object_from_definition(&chandra, alice, Zone::Battlefield);
    let other_id =
        game.create_object_from_definition(&other_planeswalker, alice, Zone::Battlefield);

    let chandra_loyalty_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::AbilityActivatedEvent::new(chandra_id, alice, false)
            .with_loyalty_ability(true),
        crate::provenance::ProvNodeId::default(),
    );
    let triggers = crate::triggers::check_triggers(&game, &chandra_loyalty_event);
    assert_eq!(
        triggers
            .iter()
            .filter(|entry| entry.source == regulator_id)
            .count(),
        1,
        "Chandra's Regulator should trigger for a Chandra loyalty ability"
    );

    let chandra_non_loyalty_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::AbilityActivatedEvent::new(chandra_id, alice, false),
        crate::provenance::ProvNodeId::default(),
    );
    assert!(
        crate::triggers::check_triggers(&game, &chandra_non_loyalty_event)
            .iter()
            .all(|entry| entry.source != regulator_id),
        "Chandra's Regulator must not trigger for non-loyalty Chandra abilities"
    );

    let other_loyalty_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::AbilityActivatedEvent::new(other_id, alice, false)
            .with_loyalty_ability(true),
        crate::provenance::ProvNodeId::default(),
    );
    assert!(
        crate::triggers::check_triggers(&game, &other_loyalty_event)
            .iter()
            .all(|entry| entry.source != regulator_id),
        "Chandra's Regulator must not trigger for non-Chandra planeswalkers"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn chandras_regulator_runtime_copies_triggering_loyalty_ability() {
    let regulator = chandras_regulator_definition();
    let chandra = CardDefinitionBuilder::new(CardId::from_raw(2), "Chandra Test Walker")
        .card_types(vec![CardType::Planeswalker])
        .subtypes(vec![Subtype::Chandra])
        .build();

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let regulator_source = game.create_object_from_definition(&regulator, alice, Zone::Battlefield);
    let chandra_id = game.create_object_from_definition(&chandra, alice, Zone::Battlefield);
    game.stack.push(crate::game_state::StackEntry::ability(
        chandra_id,
        alice,
        vec![crate::effect::Effect::draw(1)],
    ));

    let triggering_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::AbilityActivatedEvent::new(chandra_id, alice, false)
            .with_loyalty_ability(true),
        crate::provenance::ProvNodeId::default(),
    );
    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(regulator_source, alice, &mut dm);
    ctx.triggering_event = Some(triggering_event);

    crate::effects::TagTriggeringSourceEffect::new(crate::tag::TagKey::from("triggering_source"))
        .execute(&mut game, &mut ctx)
        .expect("ability activation should expose its source for tagging");
    let copy_outcome = crate::effects::CopySpellEffect::single(ChooseSpec::Tagged(
        crate::tag::TagKey::from("triggering_source"),
    ))
    .execute(&mut game, &mut ctx)
    .expect("Chandra's Regulator should copy the triggered loyalty ability");

    let copied_id = copy_outcome
        .objects()
        .and_then(|objects| objects.first().copied())
        .expect("copy effect should report the copied ability object");
    let copied_entry = game
        .stack
        .iter()
        .find(|entry| entry.object_id == copied_id)
        .expect("copied ability should be on the stack");
    assert!(copied_entry.is_ability, "copy should remain an ability");
    assert!(
        copied_entry.ability_effects.is_some(),
        "copy should preserve the original ability effects"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn chandras_regulator_test_chandra_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(4), "Chandra Test Walker")
        .card_types(vec![CardType::Planeswalker])
        .subtypes(vec![Subtype::Chandra])
        .loyalty(3)
        .parse_text("+1: Draw a card.")
        .expect("test Chandra loyalty ability should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn chandras_regulator_main_phase_game() -> (crate::game_state::GameState, PlayerId) {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;
    game.turn.phase = crate::game_state::Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(alice);
    (game, alice)
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn chandras_regulator_drive_activation<D: crate::decision::DecisionMaker>(
    game: &mut crate::game_state::GameState,
    trigger_queue: &mut crate::triggers::TriggerQueue,
    state: &mut crate::game_loop::PriorityLoopState,
    mut progress: crate::decision::GameProgress,
    decision_maker: &mut D,
    discard_choice: Option<ObjectId>,
) -> crate::decision::GameProgress {
    for _ in 0..12 {
        progress = match progress {
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectOptions(ctx),
            ) => {
                let option = ctx
                    .options
                    .iter()
                    .find(|option| {
                        option.legal
                            && discard_choice.is_some()
                            && option.description.to_ascii_lowercase().contains("discard")
                    })
                    .or_else(|| ctx.options.iter().find(|option| option.legal))
                    .unwrap_or_else(|| panic!("expected a legal activation option, got {ctx:?}"));
                let description = ctx.description.to_ascii_lowercase();
                let response = if description.contains("mana payment") {
                    crate::game_loop::PriorityResponse::ManaPayment(option.index)
                } else {
                    crate::game_loop::PriorityResponse::NextCostChoice(option.index)
                };
                crate::game_loop::apply_priority_response_with_dm(
                    game,
                    trigger_queue,
                    state,
                    &response,
                    decision_maker,
                )
                .expect("activation option should apply")
            }
            crate::decision::GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::SelectObjects(_),
            ) => crate::game_loop::apply_priority_response_with_dm(
                game,
                trigger_queue,
                state,
                &crate::game_loop::PriorityResponse::CardCostChoice(
                    discard_choice.expect("discard cost should choose a hand card"),
                ),
                decision_maker,
            )
            .expect("discard choice should apply"),
            other => return other,
        };
    }

    panic!("activation did not finish after expected cost prompts; last progress was {progress:?}");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn chandras_regulator_game_loop_copies_chandra_loyalty_after_paying_one() {
    let regulator = chandras_regulator_definition();
    let chandra = chandras_regulator_test_chandra_definition();
    let (mut game, alice) = chandras_regulator_main_phase_game();
    game.create_object_from_definition(&regulator, alice, Zone::Battlefield);
    let chandra_id = game.create_object_from_definition(&chandra, alice, Zone::Battlefield);
    for idx in 0..2 {
        let draw_card = CardBuilder::new(CardId::new(), format!("Draw Card {idx}"))
            .card_types(vec![CardType::Artifact])
            .build();
        game.create_object_from_card(&draw_card, alice, Zone::Library);
    }

    let ability_index = game
        .object(chandra_id)
        .expect("test Chandra should exist")
        .abilities
        .iter()
        .position(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated.is_loyalty_ability(),
            _ => false,
        })
        .expect("test Chandra should have a loyalty ability");
    let activate_action = crate::decision::compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, ability_index: idx }
                    if *source == chandra_id && *idx == ability_index
            )
        })
        .expect("Chandra loyalty ability should be activatable through legal actions");

    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    let mut state = crate::game_loop::PriorityLoopState::new(game.players_in_game());
    let mut activation_dm = crate::decision::SelectFirstDecisionMaker;
    let progress = crate::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &crate::game_loop::PriorityResponse::PriorityAction(activate_action),
        &mut activation_dm,
    )
    .expect("activating Chandra's loyalty ability should start");
    chandras_regulator_drive_activation(
        &mut game,
        &mut trigger_queue,
        &mut state,
        progress,
        &mut activation_dm,
        None,
    );

    crate::game_loop::put_triggers_on_stack_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut activation_dm,
    )
    .expect("Chandra's Regulator trigger should be put on the stack");
    assert_eq!(
        game.stack.len(),
        2,
        "the original Chandra ability and Regulator trigger should be stacked"
    );

    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Red, 1);
    let mut trigger_dm = crate::decision::SelectFirstDecisionMaker;
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut trigger_dm)
        .expect("Regulator trigger should resolve, pay {1}, and copy the loyalty ability");
    assert_eq!(
        game.stack.iter().filter(|entry| entry.is_ability).count(),
        2,
        "paying for Chandra's Regulator should leave the original and copied loyalty abilities on the stack"
    );
    assert_eq!(
        game.player(alice)
            .expect("Alice should exist")
            .mana_pool
            .total(),
        0,
        "Regulator trigger should spend the {{1}} payment"
    );

    crate::game_loop::resolve_stack_entry_with(&mut game, &mut trigger_dm)
        .expect("copied Chandra loyalty ability should resolve");
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut trigger_dm)
        .expect("original Chandra loyalty ability should resolve");
    assert_eq!(
        game.player(alice).expect("Alice should exist").hand.len(),
        2,
        "the copied and original Chandra abilities should each draw a card"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn chandras_regulator_activate_draw_with_discard(
    discard_card: crate::card::Card,
) -> crate::game_state::GameState {
    let regulator = chandras_regulator_definition();
    let (mut game, alice) = chandras_regulator_main_phase_game();
    let regulator_id = game.create_object_from_definition(&regulator, alice, Zone::Battlefield);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Red, 1);
    let discard_id = game.create_object_from_card(&discard_card, alice, Zone::Hand);
    let discard_stable_id = game
        .object(discard_id)
        .expect("discard card should exist")
        .stable_id;
    let draw_card = CardBuilder::new(CardId::new(), "Drawn Card")
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_card(&draw_card, alice, Zone::Library);

    let ability_index = game
        .object(regulator_id)
        .expect("Chandra's Regulator should exist")
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
        .expect("Chandra's Regulator should have an activated draw ability");
    let activate_action = crate::decision::compute_legal_actions(&game, alice)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, ability_index: idx }
                    if *source == regulator_id && *idx == ability_index
            )
        })
        .expect("Regulator draw ability should be legal with matching discard fuel");

    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    let mut state = crate::game_loop::PriorityLoopState::new(game.players_in_game());
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let progress = crate::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut trigger_queue,
        &mut state,
        &crate::game_loop::PriorityResponse::PriorityAction(activate_action),
        &mut dm,
    )
    .expect("Regulator draw activation should start");
    chandras_regulator_drive_activation(
        &mut game,
        &mut trigger_queue,
        &mut state,
        progress,
        &mut dm,
        Some(discard_id),
    );

    assert!(
        game.is_tapped(regulator_id),
        "activation cost should tap Regulator"
    );
    let discarded_id = game
        .find_object_by_stable_id(discard_stable_id)
        .expect("discarded card should still be tracked by stable id");
    assert!(
        game.player(alice)
            .expect("Alice should exist")
            .graveyard
            .contains(&discarded_id),
        "activation cost should move the chosen card to the graveyard"
    );

    crate::game_loop::resolve_stack_entry_with(&mut game, &mut dm)
        .expect("Regulator draw ability should resolve");
    assert_eq!(
        game.player(alice).expect("Alice should exist").hand.len(),
        1,
        "Regulator draw ability should draw after paying the discard cost"
    );
    game
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn chandras_regulator_draw_activation_can_discard_a_mountain_card() {
    let mountain = CardBuilder::new(CardId::new(), "Test Mountain")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Mountain])
        .build();

    chandras_regulator_activate_draw_with_discard(mountain);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn chandras_regulator_draw_activation_can_discard_a_red_card() {
    let red_card = CardBuilder::new(CardId::new(), "Red Test Card")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();

    chandras_regulator_activate_draw_with_discard(red_card);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn chandras_regulator_draw_activation_rejects_nonred_nonmountain_discard() {
    let regulator = chandras_regulator_definition();
    let (mut game, alice) = chandras_regulator_main_phase_game();
    let regulator_id = game.create_object_from_definition(&regulator, alice, Zone::Battlefield);
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::Red, 1);
    let blue_card = CardBuilder::new(CardId::new(), "Blue Test Card")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    game.create_object_from_card(&blue_card, alice, Zone::Hand);

    assert!(
        !crate::decision::compute_legal_actions(&game, alice)
            .into_iter()
            .any(|action| matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, .. }
                    if source == regulator_id
            )),
        "Regulator draw ability should be illegal without a Mountain card or red card to discard"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_panoptic_projektor_full_text_compiles() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Panoptic Projektor")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "{T}: The next face-down creature spell you cast this turn costs {3} less to cast.\nIf turning a face-down permanent face up causes a triggered ability of a permanent you control to trigger, that ability triggers an additional time.",
        )
        .expect("Panoptic Projektor should compile");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.to_ascii_lowercase().contains(
            "the next face-down creature spell you cast this turn costs {3} less to cast"
        ),
        "expected next-spell cost text in compiled output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_next_spell_cascade_family_compiles() {
    for (name, card_types, text) in [
        (
            "Dark Apostle",
            vec![CardType::Creature],
            "Gift of Chaos — {3}, {T}: The next noncreature spell you cast this turn has cascade. (When you cast that spell, exile cards from the top of your library until you exile a nonland card that costs less. You may cast it without paying its mana cost. Put the exiled cards on the bottom in a random order.)",
        ),
        (
            "Sloppity Bilepiper",
            vec![CardType::Creature],
            "Jolly Gutpipes — {2}, {T}, Sacrifice a creature: The next creature spell you cast this turn has cascade. (When you next cast a creature spell, exile cards from the top of your library until you exile a nonland card that costs less. You may cast it without paying its mana cost. Put the exiled cards on the bottom in a random order.)",
        ),
        (
            "Smoldering Stagecoach",
            vec![CardType::Creature],
            "Smoldering Stagecoach's power is equal to the number of instant and sorcery cards in your graveyard.\nWhenever Smoldering Stagecoach attacks, the next instant spell and the next sorcery spell you cast this turn each have cascade.",
        ),
        (
            "Bigger on the Inside",
            vec![CardType::Enchantment],
            "Enchant artifact or land\nEnchanted permanent has \"{T}: Target player adds two mana of any one color. The next spell they cast this turn has cascade.\" (When they cast their next spell, they exile cards from the top of their library until they exile a nonland card that costs less. They may cast it without paying its mana cost. They put the exiled cards on the bottom in a random order.)",
        ),
    ] {
        CardDefinitionBuilder::new(CardId::from_raw(1), name)
            .card_types(card_types)
            .parse_text(text)
            .unwrap_or_else(|err| panic!("{name} should compile, got {err:?}"));
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_next_spell_cascade_family_renders_cleanly() {
    let dark_apostle = CardDefinitionBuilder::new(CardId::from_raw(1), "Dark Apostle")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Gift of Chaos — {3}, {T}: The next noncreature spell you cast this turn has cascade. (When you cast that spell, exile cards from the top of your library until you exile a nonland card that costs less. You may cast it without paying its mana cost. Put the exiled cards on the bottom in a random order.)",
        )
        .expect("Dark Apostle should compile");
    let dark_rendered = unprocessed_compiled_lines(&dark_apostle).join(" ");
    assert!(
        dark_rendered
            .to_ascii_lowercase()
            .contains("the next noncreature spell you cast this turn has cascade."),
        "expected clean next-spell render for Dark Apostle, got {dark_rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_wayta_trainer_prodigy_full_text_compiles() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Wayta, Trainer Prodigy")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Haste\n{2}{G}, {T}: Target creature you control fights another target creature. This ability costs {2} less to activate if it targets two creatures you control.\nIf a creature you control being dealt damage causes a triggered ability of a permanent you control to trigger, that ability triggers an additional time.",
        )
        .expect("Wayta should compile");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains(
            "this ability costs {2} less to activate if it targets two creatures you control"
        ) || rendered_lower.contains(
            "this ability costs 2 less to activate if it targets two creatures you control"
        ),
        "expected inline activated cost text in compiled output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_windcrag_siege_full_text_compiles() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Windcrag Siege")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "As this enchantment enters, choose Mardu or Jeskai.\nMardu - If a creature attacking causes a triggered ability of a permanent you control to trigger, that ability triggers an additional time.\nJeskai - At the beginning of your upkeep, create a 1/1 red Goblin creature token. It gains lifelink and haste until end of turn.",
        )
        .expect("Windcrag Siege should compile");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered
            .to_ascii_lowercase()
            .contains("choose mardu or jeskai"),
        "expected named choice text in compiled output, got {rendered}"
    );
    assert!(
        rendered
            .to_ascii_lowercase()
            .contains("triggers an additional time"),
        "expected Mardu trigger-doubling text in compiled output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_barrensteppe_siege_full_text_compiles_and_renders_choice_bullets() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Barrensteppe Siege")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "As this enchantment enters, choose Abzan or Mardu.\n• Abzan — At the beginning of your end step, put a +1/+1 counter on each creature you control.\n• Mardu — At the beginning of your end step, if a creature died under your control this turn, each opponent sacrifices a creature of their choice.",
        )
        .expect("Barrensteppe Siege should compile strictly");

    let rendered = crate::compiled_text::canonical_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("As this enchantment enters, choose abzan or mardu."),
        "expected named-option as-enters choice text, got:\n{rendered}"
    );
    assert!(
        rendered.contains("• Abzan — At the beginning of your end step, put a +1/+1 counter on each creature you control."),
        "expected Abzan bullet trigger without redundant chosen-option condition, got:\n{rendered}"
    );
    assert!(
        rendered.contains("• Mardu — At the beginning of your end step, if a creature died under your control this turn, each opponent sacrifices a creature of their choice."),
        "expected Mardu bullet trigger with compact controller-qualified death condition, got:\n{rendered}"
    );
    assert!(
        !rendered.contains("chosen option"),
        "choice bullet rendering should not repeat the structural chosen-option gate, got:\n{rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_as_long_as_its_enchanted_condition_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Fledgling Osprey Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature has flying as long as it's enchanted.")
        .expect("as-long-as-its-enchanted static line should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("flying"),
        "expected flying in rendered static text, got {rendered}"
    );
    assert!(
        rendered.contains("as long as this creature is enchanted"),
        "expected enchanted condition in rendered static text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_as_long_as_it_attacked_this_turn_condition_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Agent Frank Horrigan")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Trample\nThis creature has indestructible as long as it attacked this turn.\nWhenever this creature enters or attacks, proliferate twice.",
        )
        .expect("attacked-this-turn static line should parse");

    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        !static_ids.contains(&StaticAbilityId::RuleFallbackText),
        "attacked-this-turn static line should not fall back to placeholder static ability: {static_ids:?}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("indestructible"),
        "expected indestructible in rendered static text, got {rendered}"
    );
    assert!(
        rendered.contains("as long as this creature attacked this turn"),
        "expected attacked-this-turn condition in rendered static text, got {rendered}"
    );
    assert!(
        rendered.contains("proliferate twice"),
        "expected proliferate trigger to remain in compiled output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_as_long_as_enchanted_permanent_is_a_creature_condition_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Rune of Flight Variant")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(
            "Enchant permanent\nAs long as enchanted permanent is a creature, it has flying.",
        )
        .expect("enchanted-permanent creature condition line should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("has flying"),
        "expected attached keyword grant in rendered static text, got {rendered}"
    );
    assert!(
        rendered.contains("as long as enchanted permanent is a creature"),
        "expected enchanted-permanent creature condition in rendered static text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_each_creature_assigns_combat_damage_with_toughness_static_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Doran Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Each creature assigns combat damage equal to its toughness rather than its power.",
        )
        .expect("global toughness-combat-damage static line should parse");

    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        static_ids.contains(
            &crate::static_abilities::StaticAbilityId::CreaturesAssignCombatDamageUsingToughness
        ),
        "expected global toughness combat-damage static ability, got {static_ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_each_creature_you_control_assigns_combat_damage_with_toughness_static_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Brontodon Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Each creature you control assigns combat damage equal to its toughness rather than its power.",
        )
        .expect("you-control toughness-combat-damage static line should parse");

    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        static_ids.contains(
            &crate::static_abilities::StaticAbilityId::CreaturesYouControlAssignCombatDamageUsingToughness
        ),
        "expected controller-scoped toughness combat-damage static ability, got {static_ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_this_creature_assigns_combat_damage_with_toughness_static_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Streetwise Negotiator")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature assigns combat damage equal to its toughness rather than its power.",
        )
        .expect("source-scoped toughness-combat-damage static line should parse");

    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        static_ids.contains(
            &crate::static_abilities::StaticAbilityId::ThisCreatureAssignsCombatDamageUsingToughness
        ),
        "expected source-scoped toughness combat-damage static ability, got {static_ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_defensive_formation_strict_and_renders_combat_assignment_clause() {
    assert_oracle_card_parses_strict("Defensive Formation");
    let def = parse_oracle_card_definition("Defensive Formation");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        static_ids.contains(
            &crate::static_abilities::StaticAbilityId::YouAssignCombatDamageOfCreaturesAttackingYou
        ),
        "expected Defensive Formation to lower to defending-player combat-damage assignment static ability, got {static_ids:?}"
    );
    assert!(
        rendered.contains(
            "Rather than the attacking player, you assign the combat damage of each creature attacking you"
        ) && rendered.contains(
            "You can divide that creature's combat damage as you choose among any of the creatures blocking it"
        ),
        "Defensive Formation should render both combat-damage assignment clauses, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn defensive_formation_defending_player_orders_damage_assignment_for_creatures_attacking_them()
 {
    let mut game = crate::game_state::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let formation = parse_oracle_card_definition("Defensive Formation");
    game.create_object_from_definition(&formation, alice, Zone::Battlefield);

    let attacker = create_winds_test_creature(&mut game, "Bob Attacker", bob, 4, 4);
    let alice_blocker_a = create_winds_test_creature(&mut game, "Alice Blocker A", alice, 1, 4);
    let alice_blocker_b = create_winds_test_creature(&mut game, "Alice Blocker B", alice, 1, 4);
    let charlie_blocker_a =
        create_winds_test_creature(&mut game, "Charlie Blocker A", charlie, 1, 4);
    let charlie_blocker_b =
        create_winds_test_creature(&mut game, "Charlie Blocker B", charlie, 1, 4);
    let planeswalker = {
        let card = crate::card::CardBuilder::new(CardId::new(), "Alice Planeswalker")
            .card_types(vec![CardType::Planeswalker])
            .loyalty(3)
            .build();
        game.create_object_from_card(&card, alice, Zone::Battlefield)
    };

    let mut attacking_alice = crate::combat_state::CombatState::default();
    attacking_alice
        .attackers
        .push(crate::combat_state::AttackerInfo {
            creature: attacker,
            target: crate::combat_state::AttackTarget::Player(alice),
        });
    attacking_alice
        .blockers
        .insert(attacker, vec![alice_blocker_a, alice_blocker_b]);

    assert_eq!(
        crate::game_loop::combat_damage_assignment_player_for_attacker(
            &game,
            &attacking_alice,
            attacker
        ),
        Some(alice),
        "Defensive Formation's controller should assign damage for creatures attacking them"
    );
    let order_context =
        crate::game_loop::get_blocker_order_decision(&game, &attacking_alice, attacker)
            .expect("multiple blockers should need an order decision");
    let crate::decisions::context::DecisionContext::Order(order_context) = order_context else {
        panic!("expected order decision context");
    };
    assert_eq!(
        order_context.player, alice,
        "Defensive Formation should move the combat damage assignment decision to the defending player"
    );

    let mut attacking_charlie = crate::combat_state::CombatState::default();
    attacking_charlie
        .attackers
        .push(crate::combat_state::AttackerInfo {
            creature: attacker,
            target: crate::combat_state::AttackTarget::Player(charlie),
        });
    attacking_charlie
        .blockers
        .insert(attacker, vec![charlie_blocker_a, charlie_blocker_b]);

    assert_eq!(
        crate::game_loop::combat_damage_assignment_player_for_attacker(
            &game,
            &attacking_charlie,
            attacker
        ),
        Some(bob),
        "Defensive Formation should not affect creatures attacking another player"
    );

    let mut attacking_planeswalker = crate::combat_state::CombatState::default();
    attacking_planeswalker
        .attackers
        .push(crate::combat_state::AttackerInfo {
            creature: attacker,
            target: crate::combat_state::AttackTarget::Planeswalker(planeswalker),
        });
    attacking_planeswalker
        .blockers
        .insert(attacker, vec![alice_blocker_a, alice_blocker_b]);

    assert_eq!(
        crate::game_loop::combat_damage_assignment_player_for_attacker(
            &game,
            &attacking_planeswalker,
            attacker
        ),
        Some(bob),
        "Defensive Formation should not affect creatures attacking a planeswalker its controller controls"
    );

    let alice_trampler = create_winds_test_creature(&mut game, "Bob Trampler", bob, 5, 5);
    if let Some(object) = game.object_mut(alice_trampler) {
        object.abilities_mut().push(Ability::static_ability(
            crate::static_abilities::StaticAbility::trample(),
        ));
    }
    let alice_tough_blocker_a =
        create_winds_test_creature(&mut game, "Alice Tough Blocker A", alice, 0, 6);
    let alice_tough_blocker_b =
        create_winds_test_creature(&mut game, "Alice Tough Blocker B", alice, 0, 6);
    let mut formation_combat = crate::combat_state::CombatState::default();
    formation_combat
        .attackers
        .push(crate::combat_state::AttackerInfo {
            creature: alice_trampler,
            target: crate::combat_state::AttackTarget::Player(alice),
        });
    formation_combat.blockers.insert(
        alice_trampler,
        vec![alice_tough_blocker_a, alice_tough_blocker_b],
    );
    game.set_combat_damage_assignment(alice_trampler, alice_tough_blocker_a, 1);
    game.set_combat_damage_assignment(alice_trampler, alice_tough_blocker_b, 1);

    let alice_life_before = game.player(alice).expect("Alice exists").life;
    let formation_damage_events =
        crate::game_loop::execute_combat_damage_step(&mut game, &formation_combat, false);
    assert_eq!(
        game.damage_on(alice_tough_blocker_a),
        1,
        "Defensive Formation should honor the defending player's chosen damage to the first blocker"
    );
    assert_eq!(
        game.damage_on(alice_tough_blocker_b),
        4,
        "Defensive Formation should keep any remaining assigned damage among blockers"
    );
    assert_eq!(
        game.player(alice).expect("Alice exists").life,
        alice_life_before,
        "Defensive Formation should keep unassigned trample damage from being assigned to the attacked player, got {formation_damage_events:?}"
    );

    let charlie_trampler = create_winds_test_creature(&mut game, "Bob Other Trampler", bob, 5, 5);
    if let Some(object) = game.object_mut(charlie_trampler) {
        object.abilities_mut().push(Ability::static_ability(
            crate::static_abilities::StaticAbility::trample(),
        ));
    }
    let charlie_tough_blocker_a =
        create_winds_test_creature(&mut game, "Charlie Tough Blocker A", charlie, 0, 6);
    let charlie_tough_blocker_b =
        create_winds_test_creature(&mut game, "Charlie Tough Blocker B", charlie, 0, 6);
    let mut normal_combat = crate::combat_state::CombatState::default();
    normal_combat
        .attackers
        .push(crate::combat_state::AttackerInfo {
            creature: charlie_trampler,
            target: crate::combat_state::AttackTarget::Player(charlie),
        });
    normal_combat.blockers.insert(
        charlie_trampler,
        vec![charlie_tough_blocker_a, charlie_tough_blocker_b],
    );
    game.set_combat_damage_assignment(charlie_trampler, charlie_tough_blocker_a, 1);
    game.set_combat_damage_assignment(charlie_trampler, charlie_tough_blocker_b, 1);

    let charlie_life_before = game.player(charlie).expect("Charlie exists").life;
    crate::game_loop::execute_combat_damage_step(&mut game, &normal_combat, false);
    assert_eq!(
        game.player(charlie).expect("Charlie exists").life,
        charlie_life_before - 3,
        "Defensive Formation controlled by Alice should not change trample assignment for creatures attacking Charlie"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_return_up_to_one_subtype_list_target_stays_single_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Thwart Return Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Return target creature card and up to one target Cleric, Rogue, Warrior, or Wizard creature card from your graveyard to the battlefield.",
        )
        .expect("subtype-list return clause should parse without splitting into multiple returns");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("up to one target cleric or rogue or warrior or wizard creature card"),
        "expected subtype list to remain on a single return target clause, got {rendered}"
    );
    assert!(
        !rendered.contains("return card rogue from your graveyard")
            && !rendered.contains("return card warrior from your graveyard"),
        "expected no synthetic per-subtype return clauses, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_draw_second_card_each_turn_trigger_is_not_custom() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Second Draw Trigger Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever you draw your second card each turn, target Detective can't be blocked this turn.",
        )
        .expect("second-card draw trigger should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        !abilities_debug.contains("unimplemented_trigger"),
        "expected no fallback custom trigger for second-card draw trigger, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("PlayerDrawsNthCardEachTurnTrigger")
            || abilities_debug.contains("draws their second card each turn"),
        "expected nth-card draw trigger matcher, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_draw_third_card_each_turn_trigger_supports_higher_ordinals() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Third Draw Trigger Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Whenever you draw your third card each turn, draw a card.")
        .expect("third-card draw trigger should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("card_number: 3"),
        "expected third-card ordinal to compile as card_number=3, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_orcish_bowmasters_draw_exception_clause_compiles_noncustom_draw_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Orcish Bowmasters Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Flash\nWhen this creature enters and whenever an opponent draws a card except the first one they draw in each of their draw steps, this creature deals 1 damage to any target. Then amass Orcs 1.",
        )
        .expect("orcish bowmasters-style trigger should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        !abilities_debug.contains("unimplemented_trigger"),
        "expected no fallback custom trigger, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("OrTrigger"),
        "expected ETB-or-draw trigger composition, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("PlayerDrawsCardExceptFirstInDrawStepTrigger")
            || abilities_debug
                .contains("except the first one they draw in each of their draw steps"),
        "expected draw-exception clause to compile as draw-step-aware trigger, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("AmassEffect"),
        "expected triggered effect list to include AmassEffect, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_xyris_shared_draw_clause_keeps_both_players() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Xyris Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Flying\nWhenever an opponent draws a card except the first one they draw in each of their draw steps, create a 1/1 green Snake creature token.\nWhenever this creature deals combat damage to a player, you and that player each draw that many cards.",
        )
        .expect("xyris-style triggered abilities should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("PlayerDrawsCardExceptFirstInDrawStepTrigger"),
        "expected typed extra-draw trigger, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("ThisDealsCombatDamageToPlayerTrigger"),
        "expected combat-damage trigger, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("DrawCardsEffect")
            && abilities_debug.contains("player: You")
            && abilities_debug.contains("player: DamagedPlayer"),
        "expected combat-damage trigger to draw for you and the damaged player, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_xyris_shared_draw_clause_uses_oracle_style_surface() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Xyris Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Flying\nWhenever an opponent draws a card except the first one they draw in each of their draw steps, create a 1/1 green Snake creature token.\nWhenever this creature deals combat damage to a player, you and that player each draw that many cards.",
        )
        .expect("xyris-style triggered abilities should render cleanly");

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains(
            "Whenever this creature deals combat damage to a player, you and that player each draw that many cards."
        ),
        "expected oracle-like shared draw surface, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_tataru_taru_off_turn_draw_trigger_is_typed_and_capped() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Tataru Taru Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, you draw a card and target opponent may draw a card.\nScions' Secretary — Whenever an opponent draws a card, if it isn't that player's turn, create a tapped Treasure token. This ability triggers only once each turn.",
        )
        .expect("tataru taru-style trigger should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("PlayerDrawsCardTrigger"),
        "expected typed draw trigger matcher, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("not_during_turn: Some(")
            || abilities_debug.contains("if it isn't that player's turn"),
        "expected off-turn draw restriction in trigger matcher, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("MaxTimesEachTurn"),
        "expected once-each-turn cap to survive lowering, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_tataru_taru_draw_trigger_keeps_explicit_once_each_turn_suffix() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Tataru Taru Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, you draw a card and target opponent may draw a card.\nScions' Secretary — Whenever an opponent draws a card, if it isn't that player's turn, create a tapped Treasure token. This ability triggers only once each turn.",
        )
        .expect("tataru taru-style trigger should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("whenever an opponent draws a card")
            && rendered.contains("this ability triggers only once each turn")
            && rendered.contains("create a tapped treasure token"),
        "expected render to keep explicit once-each-turn suffix, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_intuition_target_opponent_divvy_bundle() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Intuition Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Search your library for three cards and reveal them. Target opponent chooses one. Put that card into your hand and the rest into your graveyard. Then shuffle.",
        )
        .expect("intuition-style divvy spell should parse");

    let spell_debug = format!("{:#?}", def.spell_effect);
    assert!(
        spell_debug.contains("divvy_chosen"),
        "expected chosen-card tagging for opponent split, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("divvy_source"),
        "expected original revealed set to stay tagged for rest-to-graveyard handling, got {spell_debug}"
    );
    let spell_effect = def.spell_effect.as_ref().expect("spell effects");
    let search = spell_effect.segments[0].default_effects[0]
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()
        .expect("first Intuition effect should be the library search");
    assert_eq!(search.count, crate::effect::ChoiceCount::exactly(3));
    assert_eq!(
        search.search_mode,
        crate::effect::SearchSelectionMode::Exact
    );
    assert!(
        spell_debug.contains("ShuffleLibraryEffect"),
        "expected shuffle to remain in spell effect bundle, got {spell_debug}"
    );
}

#[test]
pub(super) fn parse_oracle_death_or_glory_divvy_surface_regression() {
    let def = parse_oracle_card_definition("Death or Glory");
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert!(
        rendered.contains(
            "Separate all creature cards in your graveyard into two piles. Exile the pile of an opponent's choice and return the other to the battlefield."
        ),
        "expected Death or Glory to render its two-pile graveyard divvy wording, got {rendered}"
    );

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("divvy_chosen") && debug.contains("ReturnAllToBattlefieldEffect"),
        "expected Death or Glory to keep the underlying divvy/exile/return structure, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_ecological_appreciation_divvy_surface_regression() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ecological Appreciation")
        .parse_text(
            "Mana cost: {X}{2}{G}\nType: Sorcery\nSearch your library and graveyard for up to four creature cards with different names that each have mana value X or less and reveal them. An opponent chooses two of those cards. Shuffle the chosen cards into your library and put the rest onto the battlefield. Exile Ecological Appreciation.",
        )
        .expect("Ecological Appreciation should parse");
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert!(
        rendered.contains(
            "Search your library and graveyard for up to four creature cards with different names that each have mana value X or less and reveal them. An opponent chooses two of those cards. Shuffle the chosen cards into your library and put the rest onto the battlefield. Exile Ecological Appreciation."
        ),
        "expected Ecological Appreciation to render oracle-like two-pile search wording, got {rendered}"
    );

    let debug_rendered = debug_compiled_lines(&def).join(" ");
    assert!(
        debug_rendered.contains(
            "Search your library and graveyard for up to four creature cards with different names that each have mana value X or less and reveal them. An opponent chooses two of those cards. Shuffle the chosen cards into your library and put the rest onto the battlefield. Exile Ecological Appreciation."
        ),
        "expected Ecological Appreciation debug text to compact from structured divvy effects, got {debug_rendered}"
    );

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("divvy_chosen") && debug.contains("divvy_source"),
        "expected Ecological Appreciation to preserve the underlying tagged divvy bundle, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_elemental_teachings_divvy_surface_regression() {
    let def = parse_oracle_card_definition("Elemental Teachings");
    let rendered = debug_compiled_lines(&def).join(" ");

    assert_eq!(
        rendered,
        "Search your library for up to 4 land cards with different names and reveal them. An opponent chooses two of those cards. Put the chosen cards into your graveyard and the rest onto the battlefield tapped. Then shuffle.",
        "expected Elemental Teachings to render its exact opponent-choice land divvy wording"
    );

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("divvy_chosen")
            && debug.contains("divvy_source")
            && debug.contains("distinct_names: true")
            && debug.contains("enters_tapped: true"),
        "expected Elemental Teachings to preserve the tagged divvy bundle, distinct-name search, and tapped battlefield move, got {debug}"
    );

    let oracle = oracle_text_by_name()
        .get("Elemental Teachings")
        .expect("missing Elemental Teachings oracle text");
    let rendered_lines = vec![rendered.clone()];
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        crate::semantic_compare::compare_card_semantics_scored(
            "Elemental Teachings",
            oracle,
            &rendered_lines,
            crate::semantic_compare::report_embedding_config(),
        );
    assert!(
        similarity >= 0.99,
        "expected >=0.99 similarity, got {similarity}"
    );
    assert!(!mismatch, "expected no semantic mismatch, got {rendered}");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_battle_for_bretagard_strict_parser_and_text_regression() {
    let def = parse_oracle_card_definition("Battle for Bretagard");
    let rendered = debug_compiled_lines(&def).join(" ").to_ascii_lowercase();

    assert!(
        rendered.contains(
            "choose any number of artifact tokens and/or creature tokens you control with different names. for each of them, create a token that's a copy of it"
        ),
        "expected Battle for Bretagard chapter III to render its distinct-name token-copy choice, got {rendered}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ChooseObjectsEffect")
            && debug.contains("distinct_names: true")
            && debug.contains("token: true")
            && debug.contains("ForEachTaggedEffect")
            && debug.contains("CreateTokenCopyEffect"),
        "expected Battle for Bretagard to preserve a structural distinct-name token choice and per-object copy, got {debug}"
    );

    let oracle = oracle_text_by_name()
        .get("Battle for Bretagard")
        .expect("missing Battle for Bretagard oracle text");
    let rendered_lines = debug_compiled_lines(&def);
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        crate::semantic_compare::compare_card_semantics_scored(
            "Battle for Bretagard",
            oracle,
            &rendered_lines,
            crate::semantic_compare::report_embedding_config(),
        );
    assert!(
        similarity >= 0.99,
        "expected >=0.99 similarity, got {similarity}"
    );
    assert!(!mismatch, "expected no semantic mismatch, got {rendered}");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_split_the_spoils_divvy_uses_splitter_then_opponent_choice() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Split the Spoils")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Exile up to five target permanent cards from your graveyard and separate them into two piles. An opponent chooses one of those piles. Put that pile into your hand and the other into your graveyard.",
        )
        .expect("Split the Spoils should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("an opponent chooses one of those piles")
            && rendered.contains("put that pile into your hand and the other into your graveyard"),
        "expected Split the Spoils to keep its oracle-like pile-choice wording, got {rendered}"
    );

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("UnlessActionEffect"),
        "expected Split the Spoils to preserve opponent pile choice as a branch, got {debug}"
    );
    assert!(
        debug.contains("chooser: You")
            && debug.contains("tag: TagKey(\n                                \"divvy_pile\""),
        "expected caster-owned pile split tagging, got {debug}"
    );
    assert!(
        debug.contains("player: Opponent"),
        "expected the opponent to own the final pile-selection branch, got {debug}"
    );

    let canonical = unprocessed_compiled_lines(&def).join(" ");
    assert_eq!(
        canonical,
        "Exile up to five target permanent cards from your graveyard and separate them into two piles. An opponent chooses one of those piles. Put that pile into your hand and the other into your graveyard."
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_unesh_criosphinx_sovereign_reveal_top_opponent_split_piles() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(91_102), "Unesh, Criosphinx Sovereign")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Sphinx])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "Flying\n\
             Sphinx spells you cast cost {2} less to cast.\n\
             Whenever Unesh or another Sphinx you control enters, reveal the top four cards of your library. An opponent separates those cards into two piles. Put one pile into your hand and the other into your graveyard.",
        )
        .expect("Unesh, Criosphinx Sovereign should parse strictly");

    let rendered = compiled_text_lines(&def).join("\n");
    assert_eq!(
        rendered,
        "Flying\nSphinx spells you cast cost {2} less to cast.\nWhenever Unesh or another Sphinx you control enters, reveal the top four cards of your library. An opponent separates those cards into two piles. Put one pile into your hand and the other into your graveyard."
    );

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("LookAtTopCardsEffect")
            && abilities_debug.contains("divvy_source")
            && abilities_debug.contains("divvy_pile")
            && abilities_debug.contains("chooser: Opponent")
            && abilities_debug.contains("player: You"),
        "expected Unesh's reveal-and-piles trigger to preserve the opponent split and caster pile choice structurally, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_make_an_example_preserves_choose_then_sacrifice_surface() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Make an Example")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Each opponent separates the creatures they control into two piles. For each opponent, you choose one of their piles. Each opponent sacrifices the creatures in their chosen pile. (Piles can be empty.)",
        )
        .expect("Make an Example should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert_eq!(
        rendered,
        "each opponent separates the creatures they control into two piles. for each opponent, you choose one of their piles. each opponent sacrifices the creatures in their chosen pile."
    );

    let spell_debug = format!("{:#?}", def.spell_effect);
    let spell_debug_compact = spell_debug.split_whitespace().collect::<String>();
    assert!(
        spell_debug_compact.contains("\"divvy_pile\"")
            && spell_debug_compact.contains("chooser:IteratedPlayer"),
        "expected each opponent to split their creatures into a pile, got {spell_debug}"
    );
    assert!(
        spell_debug_compact.contains("ChooseModeEffect")
            && spell_debug.contains("Choose the separated pile")
            && spell_debug.contains("Choose the other pile"),
        "expected the caster to choose which pile is used through a modal choice, got {spell_debug}"
    );
    assert!(
        spell_debug_compact.contains("relation:IsTaggedObject")
            && spell_debug_compact.contains("relation:IsNotTaggedObject"),
        "expected chosen-pile and other-pile branches to be represented structurally, got {spell_debug}"
    );

    let canonical = canonical_compiled_lines(&def).join(" ");
    assert_eq!(
        canonical,
        "Each opponent separates the creatures they control into two piles. For each opponent, you choose one of their piles. Each opponent sacrifices the creatures in their chosen pile."
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_exile_top_card_of_target_library_preserves_top_card_selection() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Top Card Exile Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}: Exile the top card of target player's library.")
        .expect("top-card exile should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("exile the top card of target player's library")
            || rendered.contains("target player exiles the top card of target player's library"),
        "expected top-card wording to remain explicit, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_lose_all_abilities_except_mana_static_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Blood Sun Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text("All lands lose all abilities except mana abilities.")
        .expect("lose-all-except-mana clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("lose all abilities except mana abilities"),
        "expected explicit except-mana wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_put_counters_equal_to_that_creatures_power() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "First Responder Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "At the beginning of your end step, you may return another creature you control to its owner's hand, then put a number of +1/+1 counters equal to that creature's power on this creature.",
        )
        .expect("dynamic +1/+1 counter count should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("PowerOf") || abilities_debug.contains("that creature's power"),
        "expected dynamic power-based counter amount, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_lose_life_equal_to_power_plus_toughness() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Phthisis Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Destroy target creature. Its controller loses life equal to its power plus its toughness.")
        .expect("power-plus-toughness life amount should parse");

    let abilities_debug = format!("{:#?}", def.spell_effect);
    assert!(
        abilities_debug.contains("Add")
            && abilities_debug.contains("PowerOf")
            && abilities_debug.contains("ToughnessOf"),
        "expected additive power+toughness life amount, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_creature_tapped_to_pay_additional_cost_targets_tap_cost_tag() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Swallow Whole Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "As an additional cost to cast this spell, tap an untapped creature you control.\nExile target tapped creature. Put a +1/+1 counter on the creature tapped to pay this spell's additional cost.",
        )
        .expect("cost-linked tapped creature reference should parse");

    let spell_debug = format!("{:#?}", def.spell_effect);
    let cost_debug = format!("{:#?}", def.additional_cost);
    assert!(
        spell_debug.contains("tapped_0") && cost_debug.contains("tapped_0"),
        "expected the additional-cost producer and follow-up target to share tapped_0, got cost {cost_debug} and spell {spell_debug}"
    );
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("tap an untapped creature you control")
            && rendered.contains(
                "Put a +1/+1 counter on the creature tapped to pay this spell's additional cost"
            ),
        "expected exact cost-linked creature surface, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_enchanted_base_pt_and_indestructible_without_nested_grant_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Almost Perfect Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Enchant creature\nEnchanted creature has base power and toughness 9/10 and has indestructible.",
        )
        .expect("base P/T + keyword aura clause should parse");

    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        static_ids.contains(&StaticAbilityId::SetBasePowerToughnessForFilter),
        "expected set-base-power/toughness static ability, got {static_ids:?}"
    );
    assert_eq!(
        static_ids
            .iter()
            .filter(|id| **id == StaticAbilityId::GrantAbility)
            .count(),
        1,
        "expected exactly one keyword grant static ability, got {static_ids:?}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("base power and toughness 9/10"),
        "expected base P/T wording in compiled output, got {rendered}"
    );
    assert!(
        rendered.contains("indestructible"),
        "expected granted indestructible wording in compiled output, got {rendered}"
    );
    assert!(
        !rendered.contains("has permanents with base power and toughness"),
        "expected no nested grant phrasing in compiled output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_target_creature_becomes_colorless_until_end_of_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ancient Kavu Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{1}: Target creature becomes colorless until end of turn.")
        .expect("becomes-colorless clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("target creature becomes colorless until end of turn"),
        "expected make-colorless wording in compiled output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_target_creature_becomes_single_color_until_end_of_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Swirling Spriggan Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{1}: Target creature becomes red until end of turn.")
        .expect("becomes-color clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("target creature becomes red until end of turn"),
        "expected set-colors wording in compiled output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_target_creature_becomes_color_of_your_choice_until_end_of_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Color Choice Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{1}: Target creature becomes the color of your choice until end of turn.")
        .expect("becomes-color-of-choice clause should parse");

    let abilities_debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        abilities_debug.contains("becomecolorchoiceeffect"),
        "expected become-color-choice effect in activated ability, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_target_creature_becomes_color_or_colors_of_your_choice_until_end_of_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Swirling Spriggan Choice Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{G/U}: Target creature you control becomes the color or colors of your choice until end of turn.")
        .expect("becomes-color-or-colors-of-choice clause should parse");

    let abilities_debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        abilities_debug.contains("becomecolorchoiceeffect"),
        "expected become-color-choice effect in activated ability, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_this_creature_becomes_creature_type_of_your_choice_until_end_of_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mistform Dreamer Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{1}: This creature becomes the creature type of your choice until end of turn.",
        )
        .expect("becomes-creature-type-of-choice clause should parse");

    let abilities_debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        abilities_debug.contains("becomecreaturetypechoiceeffect"),
        "expected become-creature-type-choice effect in activated ability, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_choose_creature_type_then_target_becomes_that_type() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Imagecrafter Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}: Choose a creature type other than Wall. Target creature becomes that type until end of turn.")
        .expect("choose-creature-type then target becomes that type should parse");

    let abilities_debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        abilities_debug.contains("becomecreaturetypechoiceeffect"),
        "expected become-creature-type-choice effect in activated ability, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_choose_creature_type_then_each_creature_becomes_that_type() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Standardize Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Choose a creature type other than Wall. Each creature becomes that type until end of turn.")
        .expect("choose-creature-type then each creature becomes that type should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered
            == "choose a creature type other than wall. each creature becomes that type until end of turn.",
        "expected oracle-shaped creature-type choice text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_elsewhere_flask_strictly_compiles_choose_basic_land_type_then_that_type_clause()
{
    let def = CardDefinitionBuilder::new(CardId::from_raw(610_618), "Elsewhere Flask")
        .card_types(vec![CardType::Artifact])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
        .parse_text(
            "When this artifact enters, draw a card.\nSacrifice this artifact: Choose a basic land type. Each land you control becomes that type until end of turn.",
        )
        .expect("Elsewhere Flask should parse strictly");

    let abilities_debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        abilities_debug.contains("becomebasiclandtypechoiceeffect"),
        "expected Elsewhere Flask to lower to become-basic-land-type-choice effect, got {abilities_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("sacrifice this artifact")
            && rendered.contains("choose a basic land type")
            && rendered.contains("each land you control becomes that type until end of turn"),
        "expected compiled text to cover Elsewhere Flask's land-type-changing clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_standalone_choose_creature_type_effect() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Creature Type Choice Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Choose a creature type.")
        .expect("standalone choose-creature-type clause should parse");

    let spell_debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        spell_debug.contains("choosecreaturetypeeffect"),
        "expected standalone creature-type choice effect, got {spell_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert_eq!(rendered, "You choose a creature type.");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_raise_the_palisade_uses_chosen_creature_type_filter() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Raise the Palisade")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Choose a creature type. Return all creatures that aren't of the chosen type to their owners' hands.",
        )
        .expect("Raise the Palisade should parse");

    let spell_debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        spell_debug.contains("choosecreaturetypeeffect"),
        "expected typed creature-type choice effect, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("excluded_chosen_creature_type: true"),
        "expected return filter to exclude the chosen creature type, got {spell_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("choose a creature type")
            && rendered.contains("return all creatures")
            && rendered.contains("chosen type"),
        "expected rendered text to preserve chosen-type bounce wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_creature_type_choice_pump_sentence_uses_shared_choice_effect() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Alpha Status Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Creatures of the creature type of your choice get +2/+2 and gain trample until end of turn.")
        .expect("creature-type choice pump sentence should parse");

    let spell_debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        spell_debug.contains("choosecreaturetypeeffect"),
        "expected shared creature-type choice effect, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("chosen_creature_type: true"),
        "expected chosen-type filter on follow-up effects, got {spell_debug}"
    );
    assert!(
        !spell_debug.contains("chosen_creature_type_ref"),
        "expected no tagged-creature scaffolding after cleanup, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_return_target_of_creature_type_of_choice_uses_shared_choice_effect() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Peer Pressure Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Return target creature of the creature type of your choice to its owner's hand.",
        )
        .expect("creature-type choice return sentence should parse");

    let spell_debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        spell_debug.contains("choosecreaturetypeeffect"),
        "expected shared creature-type choice effect, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("chosen_creature_type: true"),
        "expected chosen-type constraint on return target, got {spell_debug}"
    );
    assert!(
        !spell_debug.contains("chosen_creature_type_ref"),
        "expected no tagged-creature scaffolding after cleanup, got {spell_debug}"
    );
}
