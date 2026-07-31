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
pub(super) fn parse_persecute_discards_all_cards_of_chosen_color() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Persecute Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Choose a color. Target player reveals their hand and discards all cards of that color.")
        .expect("persecute discard-all chosen-color clause should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let debug = format!("{:?}", def.spell_effect);
    assert!(
        rendered.to_ascii_lowercase().contains("choose a color")
            && rendered.to_ascii_lowercase().contains("target player"),
        "expected choose-color setup and target-player discard render, got {rendered}"
    );
    assert!(
        debug.contains("DiscardEffect")
            && debug.contains("Count(")
            && debug.contains("chosen_color: true")
            && debug.contains("zone: Some(Hand)"),
        "expected discard-all chosen-color lowering, got {debug}"
    );
}

#[test]
pub(super) fn hand_reveal_choice_effects_render_oracle_style_surfaces() {
    let cases = [
        (
            "Never Happened Variant",
            "Target opponent reveals their hand. You choose a nonland card from that player's graveyard or hand and exile it.",
            "Target opponent reveals their hand. You choose a nonland card from that player's graveyard or hand and exile it.",
        ),
        (
            "Memory Theft Variant",
            "Target opponent reveals their hand. You choose a nonland card from it. That player discards that card. You may put a card that has an Adventure that player owns from exile into that player's graveyard.",
            "Target opponent reveals their hand. You choose a nonland card from it. That player discards that card. You may put a card that has an Adventure that player owns from exile into that player's graveyard.",
        ),
        (
            "Persecute Variant",
            "Choose a color. Target player reveals their hand and discards all cards of that color.",
            "Choose a color. Target player reveals their hand and discards all cards of that color.",
        ),
        (
            "Harsh Scrutiny Variant",
            "Target opponent reveals their hand. You choose a creature card from it. That player discards that card. Scry 1.",
            "Target opponent reveals their hand. You choose a creature card from it. That player discards that card. Scry 1.",
        ),
        (
            "Appetite Variant",
            "Target opponent reveals their hand. You choose a card from it with mana value 4 or greater and exile that card.",
            "Target opponent reveals their hand. You choose a card from it with mana value 4 or greater and exile that card.",
        ),
    ];

    for (name, oracle, expected) in cases {
        let def = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Sorcery])
            .parse_text(oracle)
            .unwrap_or_else(|err| panic!("{name} should parse: {err:?}"));
        let rendered = compiled_text_lines(&def).join("\n");
        assert_eq!(rendered, expected, "{name} rendered unexpectedly");
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn discard_up_to_two_permanents_then_draw_that_many_still_fails_loudly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Tersa Negative Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, discard up to two permanents, then draw that many cards.",
        )
        .expect_err("unsupported discard noun should fail loudly");

    let message = format!("{err:?}");
    assert!(
        message.contains("missing card keyword") || message.contains("unsupported discard"),
        "expected loud discard-qualifier failure, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_broadside_bombardiers_boast_damage_formula() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Broadside Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Boast — Sacrifice another creature or artifact: This creature deals damage equal to 2 plus the sacrificed permanent's mana value to any target. (Activate only if this creature attacked this turn and only once each turn.)",
        )
        .expect("broadside boast damage formula should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        (rendered.contains("2 plus the sacrificed permanent's mana value")
            || rendered.contains("2 plus its mana value"))
            && rendered.contains("any target"),
        "expected boast damage formula rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_reveal_top_card_then_if_land_else_hand_sequence() {
    CardDefinitionBuilder::new(CardId::new(), "Nadu Variant")
        .parse_text(
            "Creatures you control have \"Whenever this creature becomes the target of a spell or ability, reveal the top card of your library. If it's a land card, put it onto the battlefield. Otherwise, put it into your hand. This ability triggers only twice each turn.\"",
        )
        .expect("nadu-style reveal top card sequence should parse");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn reveal_top_card_sequence_still_fails_loudly_for_unsupported_tail() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Unsupported Reveal Tail Variant")
        .parse_text(
            "Reveal the top card of your library and put that card into your hand. Repeat this process.",
        )
        .expect_err("unsupported repeat-this-process tail should still fail");

    let rendered = format!("{err:?}").to_ascii_lowercase();
    assert!(
        rendered.contains("repeat this process")
            || rendered.contains("could not find verb in effect clause"),
        "expected loud unsupported-tail failure, got {rendered}"
    );
    assert!(
        !rendered.contains("missing reveal count in reveal-top matching split clause"),
        "expected reveal-top helper to decline unrelated top-card text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_ad_nauseam_style_optional_repeat_process() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ad Nauseam Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Reveal the top card of your library and put that card into your hand. You lose life equal to its mana value. You may repeat this process any number of times.",
        )
        .expect("ad nauseam style optional repeat process should parse");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Reveal the top card of your library and put that card into your hand"),
        "expected ad nauseam reveal-and-draw clause to stay oracle-like, got {rendered}"
    );
    assert!(
        rendered.contains("You may repeat this process any number of times"),
        "expected optional repeat-process text in render output, got {rendered}"
    );
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("RepeatProcess"),
        "expected repeat-process lowering in compiled card definition, got {debug}"
    );
    assert!(
        !debug.to_ascii_lowercase().contains("unsupported"),
        "expected ad nauseam style repeat process to avoid unsupported placeholders, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_birgi_front_face_support_lines() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Birgi Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever you cast a spell, add {R}. Until end of turn, you don't lose this mana as steps and phases end.\nCreatures you control can boast twice during each of your turns rather than once.",
        )
        .expect("birgi front-face rules text should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower
            .contains("until end of turn, you don't lose this mana as steps and phases end"),
        "expected mana-retention text in render output, got {rendered}"
    );
    assert!(
        rendered.contains(
            "Creatures you control can boast twice during each of your turns rather than once"
        ),
        "expected boast-frequency text in render output, got {rendered}"
    );
    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        !debug.contains("unsupported"),
        "expected birgi front-face parse to avoid unsupported placeholders, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_phyrexian_metamorph_style_enter_as_copy_with_added_card_type() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Phyrexian Metamorph Variant")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .parse_text(
            "You may have this creature enter as a copy of any artifact or creature on the battlefield except it's an artifact in addition to its other types.",
        )
        .expect("metamorph copy-as-enters text should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("copy of any artifact or creature on the battlefield except it's an artifact in addition to its other types"),
        "expected copy-as-enters text in render output, got {rendered}"
    );
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("added_card_types"),
        "expected copy-as-enters lowering to record added card type support, got {debug}"
    );
    assert!(
        !debug.to_ascii_lowercase().contains("unsupported"),
        "expected metamorph copy-as-enters parse to avoid unsupported placeholders, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_the_mimeoplasm_linked_graveyard_copy_replacement() {
    let def = CardDefinitionBuilder::new(CardId::new(), "The Mimeoplasm")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Ooze])
        .power_toughness(crate::card::PowerToughness::fixed(0, 0))
        .parse_text(
            "As The Mimeoplasm enters, you may exile two creature cards from graveyards. If you do, it enters as a copy of one of those cards with a number of additional +1/+1 counters on it equal to the power of the other card.",
        )
        .expect("The Mimeoplasm linked graveyard copy replacement should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("you may exile two creature cards from graveyards")
            && rendered.contains("it enters as a copy of one of those cards")
            && rendered.contains("additional +1/+1 counters"),
        "expected The Mimeoplasm compiled text to preserve linked exile/copy/counter clause, got {rendered}"
    );
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("linked_exile_pair")
            && debug.contains("PlusOnePlusOne")
            && debug.contains("nontoken: true"),
        "expected The Mimeoplasm lowering to record linked exile pair with +1/+1 counters for nontoken creature cards, got {debug}"
    );
    assert!(
        !debug.to_ascii_lowercase().contains("unsupported"),
        "expected The Mimeoplasm parse to avoid unsupported placeholders, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_omni_changeling_copy_exception_stays_localized() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Omni-Changeling")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Shapeshifter])
        .power_toughness(crate::card::PowerToughness::fixed(0, 0))
        .parse_text(
            "Changeling (This card is every creature type.)\nConvoke (Your creatures can help cast this spell. Each creature you tap while casting this spell pays for {1} or one mana of that creature's color.)\nYou may have this creature enter as a copy of any creature on the battlefield, except it has changeling.",
        )
        .expect("omni-changeling copy exception should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "You may have this creature enter as a copy of any creature on the battlefield, except it has changeling."
        ),
        "expected localized copy-exception text in render output, got {rendered}"
    );
    assert!(
        !rendered.contains("All creatures have changeling."),
        "expected copy exception to avoid degrading into a global grant, got {rendered}"
    );
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("added_abilities"),
        "expected copy-as-enters lowering to record added ability support, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_auton_soldier_copy_exception_with_nonlegendary_artifact_and_myriad() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Auton Soldier")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Robot, Subtype::Soldier])
        .power_toughness(crate::card::PowerToughness::fixed(4, 4))
        .parse_text(
            "You may have this creature enter as a copy of any creature on the battlefield, except it isn't legendary, is an artifact in addition to its other types, and has myriad.",
        )
        .expect("Auton Soldier copy exception should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("isn't legendary")
            && rendered.contains("is an artifact in addition to its other types")
            && rendered.contains("has myriad"),
        "expected Auton Soldier copy exception details in render output, got {rendered}"
    );

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("removed_supertypes") && debug.contains("Legendary"),
        "expected copy-as-enters lowering to remove legendary, got {debug}"
    );
    assert!(
        debug.contains("added_card_types") && debug.contains("Artifact"),
        "expected copy-as-enters lowering to add artifact type, got {debug}"
    );
    assert!(
        debug.contains("added_abilities") && debug.to_ascii_lowercase().contains("myriad"),
        "expected copy-as-enters lowering to add myriad, got {debug}"
    );
    assert!(
        !debug.contains("StaticAbilityId::KeywordMarker")
            || !debug.to_ascii_lowercase().contains("myriad"),
        "expected myriad to lower as functional triggered ability, got {debug}"
    );
    assert!(
        !debug.to_ascii_lowercase().contains("unsupported"),
        "expected Auton Soldier parse to avoid unsupported placeholders, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_sakashimas_student_copy_exception_adds_ninja_subtype() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sakashima's Student")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Ninja])
        .power_toughness(crate::card::PowerToughness::fixed(0, 0))
        .parse_text(
            "Ninjutsu {1}{U} ({1}{U}, Return an unblocked attacker you control to hand: Put this card onto the battlefield from your hand tapped and attacking.)\nYou may have this creature enter as a copy of any creature on the battlefield, except it's a Ninja in addition to its other creature types.",
        )
        .expect("Sakashima's Student should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("ninjutsu {1}{u}")
            && rendered_lower.contains(
                "copy of any creature on the battlefield except it's a ninja in addition to its other creature types"
            ),
        "expected Sakashima's Student compiled text to include compact ninjutsu and the Ninja copy exception, got {rendered}"
    );

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("NinjutsuCostEffect") && debug.contains("NinjutsuEffect"),
        "expected Sakashima's Student to lower ninjutsu as a hand activated ability, got {debug}"
    );
    assert!(
        debug.contains("added_subtypes") && debug.contains("Ninja"),
        "expected copy-as-enters lowering to record the added Ninja subtype, got {debug}"
    );
    assert!(
        !debug.to_ascii_lowercase().contains("unsupported"),
        "expected Sakashima's Student parse to avoid unsupported placeholders, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_reveal_card_this_way_trigger_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Primitive Etchings Variant")
        .parse_text(
            "Reveal the first card you draw each turn. Whenever you reveal a creature card this way, draw a card.",
        )
        .expect("reveal-card-this-way trigger clause should parse");

    assert!(def.spell_effect.is_none());
    assert!(def.abilities.iter().any(|ability| matches!(
        &ability.kind,
        AbilityKind::Static(static_ability)
            if static_ability.id() == StaticAbilityId::RevealFirstCardYouDrawEachTurn
    )));
    assert!(
        def.abilities
            .iter()
            .any(|ability| matches!(&ability.kind, AbilityKind::Triggered(_)))
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_optional_reveal_first_draw_trigger_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Kefnet Variant")
        .parse_text(
            "You may reveal the first card you draw each turn as you draw it. Whenever you reveal a creature card this way, draw a card.",
        )
        .expect("optional reveal-first-draw trigger clause should parse");

    assert!(def.spell_effect.is_none());
    assert!(def.abilities.iter().any(|ability| matches!(
        &ability.kind,
        AbilityKind::Static(static_ability)
            if static_ability.id() == StaticAbilityId::RevealFirstCardYouDrawEachTurn
    )));
    assert!(
        def.abilities
            .iter()
            .any(|ability| matches!(&ability.kind, AbilityKind::Triggered(_)))
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_sacellum_godspeaker_reveals_any_number_from_hand_and_counts_revealed_cards() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sacellum Godspeaker")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elf, Subtype::Druid])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "{T}: Reveal any number of creature cards with power 5 or greater from your hand. Add {G} for each card revealed this way.",
        )
        .expect("Sacellum Godspeaker should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("ChooseObjectsEffect")
            && abilities_debug.contains("RevealTaggedEffect")
            && abilities_debug.contains("AddScaledManaEffect")
            && abilities_debug.contains("min: 0")
            && abilities_debug.contains("max: None"),
        "expected any-number reveal-from-hand lowering with scaled green mana, got {abilities_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("reveal any number of creature cards")
            && rendered.contains("add {g}")
            && !rendered.contains("reveal it"),
        "expected Sacellum Godspeaker oracle-like text to stay on the revealed-card wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_god_eternal_kefnet_reveal_copy_cost_reduction_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "God-Eternal Kefnet Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Flying\nYou may reveal the first card you draw each turn as you draw it. Whenever you reveal an instant or sorcery card this way, copy that card and you may cast the copy. That copy costs {2} less to cast.",
        )
        .expect("god-eternal kefnet reveal trigger should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("PlayerRevealsCardTrigger"),
        "expected reveal trigger lowering, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("CastTaggedEffect"),
        "expected immediate cast-the-copy lowering, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("cost_reduction: Some"),
        "expected inline copy cost reduction, got {abilities_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("That copy costs {2} less to cast"),
        "expected compiled text to preserve Kefnet copy reduction, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_full_god_eternal_kefnet_oracle() {
    let def = CardDefinitionBuilder::new(CardId::new(), "God-Eternal Kefnet")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Flying\nYou may reveal the first card you draw each turn as you draw it. Whenever you reveal an instant or sorcery card this way, copy that card and you may cast the copy. That copy costs {2} less to cast.\nWhen God-Eternal Kefnet dies or is put into exile from the battlefield, you may put it into its owner's library third from the top.",
        )
        .expect("full god-eternal kefnet oracle should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("OrTrigger")
            && abilities_debug.contains("MoveToLibraryNthFromTopEffect"),
        "expected dies-or-exiled trigger with third-from-top move, got {abilities_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("third from the top"),
        "expected third-from-top library wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn god_eternal_rhonas_definition() -> CardDefinition {
    let oracle = oracle_text_by_name()
        .get("God-Eternal Rhonas")
        .expect("God-Eternal Rhonas oracle text")
        .clone();
    CardDefinitionBuilder::new(CardId::new(), "God-Eternal Rhonas")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Green],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie, Subtype::God])
        .power_toughness(PowerToughness::fixed(5, 5))
        .parse_text(oracle)
        .expect("God-Eternal Rhonas should parse strictly")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn vanilla_creature_definition(
    name: &str,
    power: i32,
    toughness: i32,
) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness))
        .build()
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn god_eternal_rhonas_trigger_for_event(
    game: &crate::game_state::GameState,
    event: &crate::triggers::TriggerEvent,
    source: ObjectId,
) -> crate::triggers::TriggeredAbilityEntry {
    let triggers = crate::triggers::check_triggers(game, event);
    let matching = triggers
        .into_iter()
        .filter(|entry| entry.source == source)
        .collect::<Vec<_>>();
    assert_eq!(
        matching.len(),
        1,
        "expected exactly one God-Eternal Rhonas trigger for event"
    );
    matching.into_iter().next().expect("checked one trigger")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_full_god_eternal_rhonas_oracle_and_compiled_text() {
    assert_oracle_card_parses_strict("God-Eternal Rhonas");
    let def = god_eternal_rhonas_definition();
    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("ForEachObject")
            && abilities_debug.contains("PowerOf")
            && abilities_debug.contains("Vigilance")
            && abilities_debug.contains("OrTrigger")
            && abilities_debug.contains("MoveToLibraryNthFromTopEffect"),
        "expected Rhonas ETB and dies/exile trigger structures, got {abilities_debug}"
    );
    assert!(
        abilities_debug.matches("FullName(").count() >= 3,
        "the ETB trigger and both dies-or-exile branches should retain the named source surface: {abilities_debug}"
    );

    let oracle = oracle_text_by_name()
        .get("God-Eternal Rhonas")
        .expect("God-Eternal Rhonas oracle text");
    assert_eq!(unprocessed_compiled_lines(&def).join("\n"), oracle.as_str());
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn god_eternal_rhonas_etb_doubles_other_creatures_and_grants_vigilance() {
    let rhonas = god_eternal_rhonas_definition();
    let ally = vanilla_creature_definition("Allied Bear", 2, 2);
    let second_ally = vanilla_creature_definition("Allied Soldier", 1, 3);
    let enemy = vanilla_creature_definition("Enemy Bear", 4, 4);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let rhonas_id = game.create_object_from_definition(&rhonas, alice, Zone::Battlefield);
    let ally_id = game.create_object_from_definition(&ally, alice, Zone::Battlefield);
    let second_ally_id = game.create_object_from_definition(&second_ally, alice, Zone::Battlefield);
    let enemy_id = game.create_object_from_definition(&enemy, bob, Zone::Battlefield);

    let snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(rhonas_id).expect("Rhonas exists"),
        &game,
    );
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::with_cause(
            rhonas_id,
            Zone::Stack,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            Some(snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let entry = god_eternal_rhonas_trigger_for_event(&game, &event, rhonas_id);
    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(rhonas_id, alice, &mut dm)
        .with_triggering_event(entry.triggering_event.clone());
    for effect in &entry.ability.effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Rhonas ETB trigger should resolve");
    }

    assert_eq!(game.calculated_power(ally_id), Some(4));
    assert_eq!(game.calculated_toughness(ally_id), Some(2));
    assert!(game.object_has_static_ability_id(ally_id, StaticAbilityId::Vigilance));
    assert_eq!(game.calculated_power(second_ally_id), Some(2));
    assert_eq!(game.calculated_toughness(second_ally_id), Some(3));
    assert!(game.object_has_static_ability_id(second_ally_id, StaticAbilityId::Vigilance));
    assert_eq!(game.calculated_power(rhonas_id), Some(5));
    assert!(
        !game.object_has_static_ability_id(rhonas_id, StaticAbilityId::Vigilance),
        "Rhonas should not grant vigilance to itself"
    );
    assert_eq!(game.calculated_power(enemy_id), Some(4));
    assert!(
        !game.object_has_static_ability_id(enemy_id, StaticAbilityId::Vigilance),
        "Rhonas should not affect opponents' creatures"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn god_eternal_rhonas_death_or_exile_trigger_may_put_third_from_top() {
    struct AcceptMay;
    impl crate::decision::DecisionMaker for AcceptMay {
        fn decide_boolean(
            &mut self,
            _game: &crate::game_state::GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            true
        }
    }

    for destination in [Zone::Graveyard, Zone::Exile] {
        let rhonas = god_eternal_rhonas_definition();
        let filler = vanilla_creature_definition("Library Filler", 1, 1);
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        for _ in 0..4 {
            game.create_object_from_definition(&filler, alice, Zone::Library);
        }
        let rhonas_id = game.create_object_from_definition(&rhonas, alice, Zone::Battlefield);
        let snapshot = crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(rhonas_id).expect("Rhonas exists before moving"),
            &game,
        );
        let lookback_source_snapshots = game.trigger_source_lookback_snapshots();
        let moved_id = game
            .move_object_by_effect(rhonas_id, destination)
            .expect("Rhonas should move zones");
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::zones::ZoneChangeEvent::with_results(
                rhonas_id,
                vec![moved_id],
                Zone::Battlefield,
                destination,
                crate::events::cause::EventCause::effect(),
                Some(snapshot),
            ),
            crate::provenance::ProvNodeId::default(),
        )
        .with_lookback_source_snapshots(lookback_source_snapshots);
        let entry = god_eternal_rhonas_trigger_for_event(&game, &event, rhonas_id);
        let mut dm = AcceptMay;
        let mut ctx = crate::effects::ExecutionContext::new(rhonas_id, alice, &mut dm)
            .with_triggering_event(entry.triggering_event.clone());
        for effect in &entry.ability.effects {
            crate::effects::execute_effect(&mut game, effect, &mut ctx)
                .expect("Rhonas dies/exile trigger should resolve");
        }

        let third_from_top = game
            .player(alice)
            .expect("Alice exists")
            .library
            .iter()
            .rev()
            .nth(2)
            .copied()
            .expect("library should have a third card");
        assert!(
            game.object(third_from_top)
                .is_some_and(|object| object.name == "God-Eternal Rhonas"),
            "Rhonas should be third from the top after moving from {destination:?}"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn god_eternal_rhonas_optional_library_trigger_can_be_declined() {
    let rhonas = god_eternal_rhonas_definition();
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let rhonas_id = game.create_object_from_definition(&rhonas, alice, Zone::Battlefield);
    let snapshot = crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
        game.object(rhonas_id).expect("Rhonas exists before exile"),
        &game,
    );
    let lookback_source_snapshots = game.trigger_source_lookback_snapshots();
    let exile_id = game
        .move_object_by_effect(rhonas_id, Zone::Exile)
        .expect("Rhonas should move to exile");
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::with_results(
            rhonas_id,
            vec![exile_id],
            Zone::Battlefield,
            Zone::Exile,
            crate::events::cause::EventCause::effect(),
            Some(snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    )
    .with_lookback_source_snapshots(lookback_source_snapshots);
    let entry = god_eternal_rhonas_trigger_for_event(&game, &event, rhonas_id);
    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(rhonas_id, alice, &mut dm)
        .with_triggering_event(entry.triggering_event.clone());
    for effect in &entry.ability.effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("declined Rhonas trigger should resolve");
    }

    assert!(
        game.object(exile_id)
            .is_some_and(|object| object.zone == Zone::Exile),
        "declining the may ability should leave Rhonas in exile"
    );
    assert!(
        !game
            .player(alice)
            .expect("Alice exists")
            .library
            .iter()
            .any(|id| game
                .object(*id)
                .is_some_and(|object| object.name == "God-Eternal Rhonas")),
        "declining the may ability should not put Rhonas into the library"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_long_term_plans_searches_then_puts_card_third_from_top() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Long-Term Plans")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Search your library for a card, then shuffle and put that card third from the top.",
        )
        .expect("Long-Term Plans style search placement should parse");

    let spell_debug = format!("{:#?}", def.spell_effect);
    let normalized_debug = spell_debug.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        normalized_debug.contains("SearchLibraryEffect")
            && normalized_debug.contains("library_position_from_top: Some( Fixed( 3"),
        "expected search-library effect with third-from-top placement, got {spell_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("third from the top"),
        "expected third-from-top search-library wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_reveal_cards_in_library_clause() {
    CardDefinitionBuilder::new(CardId::new(), "Guided Passage Variant")
        .parse_text("Reveal the cards in your library.")
        .expect("reveal-all-library cards clause should parse");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_target_creature_attacks_or_blocks_if_able() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Hustle Variant")
        .parse_text("Target creature attacks or blocks this turn if able.")
        .expect("target creature attacks-or-blocks clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("target creature attacks or blocks this turn if able"),
        "expected attack/block-if-able grants, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_target_creature_becomes_red_and_attacks_if_able() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Incite Variant")
        .parse_text("Target creature becomes red until end of turn and attacks this turn if able.")
        .expect("incite-style color-change plus attacks-if-able clause should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("SetColors"),
        "expected explicit color-set effect, got {debug}"
    );
    assert!(
        debug.contains("MustAttack"),
        "expected attacks-if-able grant on the same target, got {debug}"
    );
    assert!(
        debug.contains("Tagged("),
        "expected follow-up must-attack effect to reference prior target by tag, got {debug}"
    );
    assert!(
        !debug.contains("colors: Some("),
        "should not reinterpret subject as an already-red filter, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_target_creature_can_block_any_number_fails_strictly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Valor Variant")
        .parse_text("Target creature can block any number of creatures this turn.")
        .expect_err("unsupported target-only combat-action clause should fail");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported target-only restriction clause"),
        "expected strict target-only restriction error, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_target_creature_blocks_this_turn_if_able() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Culling Mark Variant")
        .parse_text("Target creature blocks this turn if able.")
        .expect("target creature blocks-if-able clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("target creature blocks this turn if able"),
        "expected must-block effect, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_target_creature_blocks_this_creature_this_turn_if_able() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Rampant Elephant Variant")
        .parse_text("{G}: Target creature blocks this creature this turn if able.")
        .expect("target creature blocks this creature should parse");

    let debug = format!("{:#?}", def);
    assert!(
        debug.contains("TargetOnlyEffect") && debug.contains("MustBlockSpecificAttacker"),
        "expected target selection plus attacker-specific must-block restriction, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("{g}: target creature blocks this creature this turn if able"),
        "expected targeted blocks-this-creature effect, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_each_creature_opponents_control_blocks_this_turn_if_able() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Predatory Rampage Variant")
        .parse_text("Each creature your opponents control blocks this turn if able.")
        .expect("each-creature blocks-if-able clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("each creature your opponents control blocks this turn if able"),
        "expected must-block effect for filtered creatures, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_play_that_card_from_exile_this_turn_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Play From Exile Variant")
        .parse_text(
            "Exile target card from a graveyard. You may play that card from exile this turn.",
        )
        .expect("play-that-card-from-exile clause should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("GrantPlayTaggedEffect") && debug.contains("UntilEndOfTurn"),
        "expected end-of-turn play permission effect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_play_an_additional_land_this_turn_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Explore Variant")
        .parse_text("You may play an additional land this turn. Draw a card.")
        .expect("additional land play clause should parse");

    let debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    assert!(
        debug.contains("AdditionalLandPlaysEffect") && debug.contains("duration: EndOfTurn"),
        "expected temporary additional-land-play effect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_newline_additional_land_this_turn_clause_stays_a_spell_effect() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Explore")
        .parse_text("You may play an additional land this turn.\nDraw a card.")
        .expect("newline additional land play clause should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    let spell_debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    assert!(
        !abilities_debug.contains("AdditionalLandPlay"),
        "temporary additional land play should not become a battlefield static ability: {abilities_debug}"
    );
    assert!(
        spell_debug.contains("AdditionalLandPlaysEffect")
            && spell_debug.contains("duration: EndOfTurn"),
        "expected end-of-turn additional-land-play spell effect, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_additional_land_this_turn_clause_is_not_wrapped_in_may() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Explore")
        .parse_text("You may play an additional land this turn.\nDraw a card.")
        .expect("explore-style text should parse");

    let spell_debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    assert!(
        !spell_debug.contains("MayEffect"),
        "permission-granting land-play text should not become a MayEffect: {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn compiled_text_keeps_additional_land_this_turn_duration() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Explore")
        .parse_text("You may play an additional land this turn.\nDraw a card.")
        .expect("explore-style text should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("You may play an additional land this turn"),
        "compiled text should keep temporary land-play duration, got {rendered}"
    );
    assert!(
        rendered.to_ascii_lowercase().contains("draw a card"),
        "compiled text should preserve draw effect, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_spell_next_upkeep_trigger_stays_in_spell_effects() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Pact Variant")
        .parse_text(
            "Search your library for a green creature card, reveal it, put it into your hand, then shuffle. At the beginning of your next upkeep, pay {2}{G}{G}. If you don't, you lose the game.",
        )
        .expect("next-upkeep pact line should parse");

    let spell_debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    let ability_debug = format!("{:?}", def.abilities);
    assert!(
        spell_debug.contains("ScheduleDelayedTriggerEffect")
            && spell_debug.contains("BeginningOfUpkeepTrigger")
            && spell_debug.contains("start_next_turn: true"),
        "expected next-upkeep delayed trigger in spell effects, got {spell_debug}"
    );
    assert!(
        !ability_debug.contains("BeginningOfUpkeepTrigger"),
        "delayed next-upkeep clause should not become a printed triggered ability: {ability_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_multiline_spell_next_upkeep_trigger_stays_in_spell_effects() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Pact Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Search your library for a green creature card, reveal it, put it into your hand, then shuffle.\nAt the beginning of your next upkeep, pay {2}{G}{G}. If you don't, you lose the game.",
        )
        .expect("multiline next-upkeep pact line should parse");

    let spell_debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    let ability_debug = format!("{:?}", def.abilities);
    assert!(
        spell_debug.contains("ScheduleDelayedTriggerEffect")
            && spell_debug.contains("BeginningOfUpkeepTrigger")
            && spell_debug.contains("start_next_turn: true"),
        "expected multiline next-upkeep delayed trigger in spell effects, got {spell_debug}"
    );
    assert!(
        !ability_debug.contains("BeginningOfUpkeepTrigger"),
        "multiline delayed next-upkeep clause should not become a printed triggered ability: {ability_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_multiline_spell_when_you_do_followup_stays_in_spell_effects() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Followup Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Sacrifice a creature.\nWhen you do, draw two cards.")
        .expect("multiline when-you-do spell should parse");

    let spell_debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    let ability_debug = format!("{:?}", def.abilities);
    assert!(
        spell_debug.contains("IfEffect") || spell_debug.contains("WithIdEffect"),
        "expected multiline follow-up clause to stay in spell effects, got {spell_debug}"
    );
    assert!(
        !ability_debug.contains("Triggered"),
        "multiline when-you-do follow-up should not become a printed triggered ability: {ability_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_fastbond_additional_land_permission_is_explicitly_unsupported() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Fastbond Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text("You may play any number of lands on each of your turns.")
        .expect_err("additional land play permission should stay unsupported");

    let debug = format!("{err:?}").to_ascii_lowercase();
    assert!(
        debug.contains("unsupported additional-land-play permission clause"),
        "expected explicit additional-land-play permission error, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_release_to_the_wind_owner_while_exiled_free_cast_permission() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Release to the Wind Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Exile target nonland permanent. For as long as that card remains exiled, its owner may cast it without paying its mana cost.",
        )
        .expect("Release to the Wind owner while-exiled free-cast permission should parse");

    let debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        debug.contains("grantplaytaggedeffect")
            && debug.contains("granttaggedspellfreecastuntilendofturneffect")
            && debug.contains("foraslongasexiled")
            && debug.contains("ownerof"),
        "expected Release to the Wind owner while-exiled free-cast permission, got {debug}"
    );
}

#[test]
pub(super) fn release_to_the_wind_oracle_parses_and_renders_while_exiled_free_cast_permission() {
    let def = parse_oracle_card_definition("Release to the Wind");

    let spell_debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        spell_debug.contains("movetozoneeffect")
            && spell_debug.contains("excluded_card_types")
            && spell_debug.contains("land")
            && spell_debug.contains("grantplaytaggedeffect")
            && spell_debug.contains("granttaggedspellfreecastuntilendofturneffect")
            && spell_debug.contains("foraslongasexiled")
            && spell_debug.contains("ownerof"),
        "Release to the Wind should structurally exile a nonland permanent and grant its owner a while-exiled free-cast permission, got {spell_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert_eq!(
        rendered,
        "Exile target nonland permanent. For as long as that card remains exiled, its owner may cast it without paying its mana cost."
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_temporary_free_play_permission_is_explicitly_unsupported() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Golos Variant")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .parse_text(
            "{2}{W}{U}{B}{R}{G}: Exile the top three cards of your library. You may play them this turn without paying their mana costs.",
        )
        .expect_err("temporary free play permission should stay unsupported");

    let debug = format!("{err:?}").to_ascii_lowercase();
    assert!(
        debug.contains("unsupported temporary play/cast permission clause with alternative cost"),
        "expected explicit temporary free play permission error, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_omniscience_static_free_cast_permission() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Omniscience Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text("You may cast spells from your hand without paying their mana costs.")
        .expect("Omniscience static permission should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered
            .to_ascii_lowercase()
            .contains("you may cast spells from your hand without paying their mana costs"),
        "expected omniscience wording in compiled output, got {rendered}"
    );

    let has_free_cast_grant = def.abilities.iter().any(|ability| {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            return false;
        };
        let Some(spec) = static_ability.grant_spec() else {
            return false;
        };
        matches!(
            spec.grantable,
            crate::grant::Grantable::AlternativeCast(
                crate::alternative_cast::AlternativeCastingMethod::Composed { .. }
            )
        ) && spec.zone == Zone::Hand
    });

    assert!(
        has_free_cast_grant,
        "expected a hand free-cast grant in parsed Omniscience ability"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_brain_in_a_jar_strictly_and_renders_counter_gated_free_cast() {
    fn find_matching_cast_filter(
        effect: &crate::effect::Effect,
    ) -> Option<crate::filter::ObjectFilter> {
        if let Some(cast) =
            effect.downcast_ref::<crate::effects::MayCastMatchingSpellWithoutPayingManaCostEffect>()
        {
            return Some(cast.filter.clone());
        }

        let mut found = None;
        effect.visit_child_effects(&mut |child| {
            if found.is_none() {
                found = find_matching_cast_filter(child);
            }
        });
        found
    }

    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Brain in a Jar")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "{1}, {T}: Put a charge counter on this artifact, then you may cast an instant or sorcery spell with mana value equal to the number of charge counters on this artifact from your hand without paying its mana cost.\n{3}, {T}, Remove X charge counters from this artifact: Scry X.",
        )
        .expect("Brain in a Jar should parse strictly");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains(
            "you may cast an instant or sorcery spell from your hand with mana value equal to the number of charge counters on this artifact without paying its mana cost"
        ) || rendered_lower.contains(
            "you may cast an instant or sorcery spell with mana value equal to the number of charge counters on this artifact from your hand without paying its mana cost"
        ),
        "expected counter-gated free-cast clause in compiled output, got {rendered}"
    );

    let counter_gated_filter = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(&activated.effects),
            _ => None,
        })
        .flat_map(|program| program.all_effects())
        .find_map(find_matching_cast_filter)
        .expect("Brain in a Jar should lower to a matching-spell cast effect");
    let has_counter_gate = counter_gated_filter.mana_value_eq_counters_on_source
        == Some(crate::object::CounterType::Charge)
        || matches!(
            counter_gated_filter.mana_value.as_ref(),
            Some(crate::filter::Comparison::EqualExpr(value))
                if matches!(
                    value.unhinted(),
                    crate::effect::Value::CountersOn(spec, Some(crate::object::CounterType::Charge))
                        if matches!(spec.base(), ChooseSpec::Source)
                )
        );
    assert!(
        has_counter_gate,
        "expected Brain in a Jar to lower to a charge-counter-gated free-cast effect, got {counter_gated_filter:#?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_surtland_elementalist_strictly_and_renders_hand_free_cast() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Surtland Elementalist")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(5)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Giant, Subtype::Wizard])
        .power_toughness(PowerToughness::fixed(8, 8))
        .parse_text(
            "As an additional cost to cast this spell, reveal a Giant card from your hand or pay {2}.\nWhenever this creature attacks, you may cast an instant or sorcery spell from your hand without paying its mana cost.",
        )
        .expect("Surtland Elementalist should parse strictly");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains(
            "as an additional cost to cast this spell, reveal a giant card from your hand or pay {2}"
        ) || (rendered_lower.contains(
            "as an additional cost to cast this spell, choose a giant card"
        ) && rendered_lower.contains("reveal it or pay {2}")),
        "expected reveal-or-pay additional cost in compiled output, got {rendered}"
    );
    assert!(
        rendered_lower.contains("whenever this creature attacks, you may cast an instant or sorcery spell from your hand without paying its mana cost"),
        "expected attack-triggered hand free-cast clause in compiled output, got {rendered}"
    );

    let ability_debug = format!("{:?}", def.abilities);
    assert!(
        ability_debug.contains("ThisAttacksTrigger")
            && ability_debug.contains("MayCastMatchingSpellWithoutPayingManaCostEffect")
            && ability_debug.contains("Instant")
            && ability_debug.contains("Sorcery"),
        "expected Surtland Elementalist to lower to an attack-triggered instant/sorcery free-cast effect, got {ability_debug}"
    );
    let cost_debug = format!("{:?}", def.additional_cost);
    assert!(
        cost_debug.contains("ChooseModeEffect")
            && cost_debug.contains("RevealTaggedEffect")
            && cost_debug.contains("Giant")
            && cost_debug.contains("PayManaEffect"),
        "expected Surtland Elementalist to keep reveal-or-pay additional cost structurally, got {cost_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_kentaro_static_mana_value_permission() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Kentaro Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "You may pay {X} rather than pay the mana cost for Samurai spells you cast, where X is that spell's mana value.",
        )
        .expect("Kentaro static permission should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "you may pay {x} rather than pay the mana cost for samurai spells you cast, where x is that spell's mana value"
        ),
        "expected Kentaro wording in compiled output, got {rendered}"
    );

    let has_mana_value_grant = def.abilities.iter().any(|ability| {
        let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
            return false;
        };
        let Some(spec) = static_ability.grant_spec() else {
            return false;
        };
        matches!(
            spec.grantable,
            crate::grant::Grantable::DerivedAlternativeCast(
                crate::grant::DerivedAlternativeCast::ManaValueAsGenericFromHand
            )
        ) && spec.zone == Zone::Hand
            && spec.filter.subtypes.contains(&Subtype::Samurai)
    });

    assert!(
        has_mana_value_grant,
        "expected a Samurai hand grant that uses mana value as an alternative cost"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_rooftop_storm_static_free_zombie_permission() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Rooftop Storm")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "You may pay {0} rather than pay the mana cost for Zombie creature spells you cast.",
        )
        .expect("Rooftop Storm static permission should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "you may pay {0} rather than pay the mana cost for zombie creature spells you cast"
        ),
        "expected Rooftop Storm wording in compiled output, got {rendered}"
    );

    let has_zombie_grant = def.abilities.iter().any(|ability| {
        let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
            return false;
        };
        let Some(spec) = static_ability.grant_spec() else {
            return false;
        };
        matches!(
            spec.grantable,
            crate::grant::Grantable::AlternativeCast(
                crate::alternative_cast::AlternativeCastingMethod::Composed { .. }
            )
        ) && spec.zone == Zone::Hand
            && spec.filter.card_types.contains(&CardType::Creature)
            && spec.filter.subtypes.contains(&Subtype::Zombie)
    });

    assert!(
        has_zombie_grant,
        "expected a Zombie creature hand grant that uses a fixed alternative mana cost"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_demon_of_fates_design_life_cost_and_sacrifice_pump_regression() {
    let def = parse_oracle_card_definition("Demon of Fate's Design");

    let rendered = unprocessed_compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "once during each of your turns, you may cast an enchantment spell by paying life equal to its mana value rather than paying its mana cost"
        ),
        "expected Demon of Fate's Design life-cost cast permission, got {rendered}"
    );
    assert!(
        rendered.contains(
            "sacrifice another enchantment: this creature gets +x/+0 until end of turn, where x is the sacrificed enchantment's mana value"
        ),
        "expected Demon of Fate's Design sacrificed-enchantment pump wording, got {rendered}"
    );

    let has_life_cost_grant = def.abilities.iter().any(|ability| {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            return false;
        };
        let Some(spec) = static_ability.grant_spec() else {
            return false;
        };
        matches!(
            spec.grantable,
            crate::grant::Grantable::DerivedAlternativeCast(
                crate::grant::DerivedAlternativeCast::LifeEqualManaValueFromHand {
                    usage_limit: Some(crate::grant::GrantUsageLimit::OnceDuringEachOfYourTurns)
                }
            )
        ) && spec.zone == Zone::Hand
            && spec.filter.card_types == [CardType::Enchantment]
    });
    assert!(
        has_life_cost_grant,
        "expected Demon of Fate's Design to grant a once-per-turn enchantment life-cost alternative cast"
    );

    let activated_debug = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(format!("{activated:?}")),
            _ => None,
        })
        .expect("Demon of Fate's Design should have an activated ability");
    assert!(
        activated_debug.contains("sacrifice_cost_0")
            && activated_debug.contains("ManaValueOf")
            && activated_debug.contains("WhereXIs"),
        "expected sacrificed enchantment mana value to drive the pump amount, got {activated_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_eye_of_duskmantle_surveilled_graveyard_permission() {
    let def = parse_oracle_card_definition("Eye of Duskmantle");

    let rendered = unprocessed_compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "you may play lands and cast spells from among cards in your graveyard you've surveilled this turn"
        ),
        "expected Eye of Duskmantle surveilled graveyard play permission, got {rendered}"
    );
    assert!(
        rendered.contains(
            "if you cast a spell this way, you pay life equal to its mana value rather than paying its mana cost"
        ),
        "expected Eye of Duskmantle life-cost replacement wording, got {rendered}"
    );

    let mut has_play_grant = false;
    let mut has_life_cost_grant = false;
    for ability in &def.abilities {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            continue;
        };
        let Some(spec) = static_ability.grant_spec() else {
            continue;
        };
        if matches!(spec.grantable, crate::grant::Grantable::PlayFrom)
            && spec.zone == Zone::Graveyard
            && spec.filter.surveilled_this_turn
            && spec.filter.owner == Some(PlayerFilter::You)
        {
            has_play_grant = true;
        }
        if matches!(
            spec.grantable,
            crate::grant::Grantable::DerivedAlternativeCast(
                crate::grant::DerivedAlternativeCast::LifeEqualManaValueFromZone {
                    zone: Zone::Graveyard,
                    usage_limit: None
                }
            )
        ) && spec.zone == Zone::Graveyard
            && spec.filter.surveilled_this_turn
            && spec.filter.excluded_card_types.contains(&CardType::Land)
        {
            has_life_cost_grant = true;
        }
    }

    assert!(
        has_play_grant,
        "expected Eye of Duskmantle to grant play-from-graveyard permission for surveilled cards"
    );
    assert!(
        has_life_cost_grant,
        "expected Eye of Duskmantle to grant a graveyard life-cost alternative cast"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_edgar_graveyard_cast_permission_with_tapped_this_way_grant() {
    let def = parse_oracle_card_definition("Edgar, Master Machinist");

    let rendered = unprocessed_compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "once during each of your turns, you may cast an artifact spell from your graveyard"
        ),
        "expected Edgar graveyard artifact-cast permission, got {rendered}"
    );
    assert!(
        rendered.contains("if you cast an artifact spell this way, that artifact enters tapped"),
        "expected Edgar cast-this-way tapped suffix, got {rendered}"
    );

    let mut has_graveyard_cast_grant = false;
    let mut has_tapped_this_way_grant = false;
    for ability in &def.abilities {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            continue;
        };
        let Some(spec) = static_ability.grant_spec() else {
            continue;
        };
        if matches!(
            spec.grantable,
            crate::grant::Grantable::DerivedAlternativeCast(
                crate::grant::DerivedAlternativeCast::GraveyardCastFromCardManaCost {
                    additional_costs: ref costs,
                    usage_limit: Some(crate::grant::GrantUsageLimit::OnceDuringEachOfYourTurns),
                    condition: None,
                    exiles_after_resolution: false,
                }
            ) if costs.is_empty()
        ) && spec.zone == Zone::Graveyard
            && spec.filter.card_types == [CardType::Artifact]
        {
            has_graveyard_cast_grant = true;
            has_tapped_this_way_grant = spec
                .cast_this_way_grants
                .iter()
                .any(|grant| grant.id() == StaticAbilityId::EntersTapped);
        }
    }

    assert!(
        has_graveyard_cast_grant,
        "expected Edgar to grant a once-per-turn artifact graveyard alternative cast"
    );
    assert!(
        has_tapped_this_way_grant,
        "expected Edgar's graveyard cast grant to carry an enters-tapped cast-this-way grant"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_creature_spell_emerge_grant_uses_hand_derived_alternative_cast() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Herigast Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Each creature spell you cast has emerge. The emerge cost is equal to its mana cost.",
        )
        .expect("emerge grant should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered
            .contains("each creature spells has emerge. the emerge cost is equal to its mana cost")
            || rendered.contains(
                "each creature spell you cast has emerge. the emerge cost is equal to its mana cost"
            ),
        "expected emerge grant wording in compiled output, got {rendered}"
    );

    let has_emerge_grant = def.abilities.iter().any(|ability| {
        let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
            return false;
        };
        let Some(spec) = static_ability.grant_spec() else {
            return false;
        };
        matches!(
            spec.grantable,
            crate::grant::Grantable::DerivedAlternativeCast(
                crate::grant::DerivedAlternativeCast::EmergeFromCardManaCost
            )
        ) && spec.zone == Zone::Hand
            && spec.filter.card_types.contains(&CardType::Creature)
    });

    assert!(
        has_emerge_grant,
        "expected a creature hand grant that derives emerge from mana cost"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_put_land_card_from_hand_onto_battlefield_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Scout Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}: You may put a land card from your hand onto the battlefield.")
        .expect("put-land-from-hand clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("put a land card from your hand onto the battlefield"),
        "expected put-land wording in compiled output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_recommission_text_parses_typed_counter_followup() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Recommission Variant")
        .parse_text(
            "Return target artifact or creature card with mana value 3 or less from your graveyard to the battlefield. If a creature enters this way, it enters with an additional +1/+1 counter on it.",
        )
        .expect("mixed return+enters-with-counters clause should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("BattlefieldEntryCounterSpec")
            && debug.contains("IfObjectEntersThisWay")
            && debug.contains("PlusOnePlusOne")
            && !debug.contains("PutCountersEffect"),
        "expected fused conditional battlefield-entry counter, got {debug}"
    );
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
        "recommission followup should not emit static placeholder fallback: {static_ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_teferis_time_twist_text_parses_typed_counter_followup() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Teferi Time Twist Variant")
        .parse_text(
            "Exile target permanent you control. Return that card to the battlefield under its owner's control at the beginning of the next end step. If it enters as a creature, it enters with an additional +1/+1 counter on it.",
        )
        .expect("mixed exile/return+enters-with-counters clause should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("ScheduleDelayedTriggerEffect")
            && debug.contains("BattlefieldEntryCounterSpec")
            && debug.contains("IfObjectEntersThisWay")
            && debug.contains("PlusOnePlusOne")
            && !debug.contains("PutCountersEffect"),
        "expected typed delayed conditional battlefield-entry counter, got {debug}"
    );
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
        "time-twist followup should not emit static placeholder fallback: {static_ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_aberrant_return_oracle_parses_and_renders_enters_with_counters() {
    assert_oracle_card_parses_strict("Aberrant Return");
    let def = parse_oracle_card_definition("Aberrant Return");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("MoveToZoneEffect")
            && debug.contains("WithCount")
            && debug.contains("min: 1")
            && debug.contains("max: Some(3)")
            && debug.contains("BattlefieldEntryCounterSpec")
            && debug.contains("EachOfThemEnters")
            && debug.contains("MinusOneMinusOne"),
        "Aberrant Return should lower to one-to-three targeted graveyard returns with entry-time -1/-1 counters, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert_eq!(
        rendered,
        "Put one, two, or three target creature cards from graveyards onto the battlefield under your control. Each of them enters with an additional -1/-1 counter on it.",
        "Aberrant Return compiled text should preserve target count and enter-with-counter wording"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn create_aberrant_return_graveyard_permanent(
    game: &mut crate::game_state::GameState,
    name: &str,
    owner: PlayerId,
    card_types: Vec<CardType>,
) -> ObjectId {
    let is_creature = card_types.contains(&CardType::Creature);
    let mut builder = crate::card::CardBuilder::new(CardId::new(), name).card_types(card_types);
    if is_creature {
        builder = builder.power_toughness(PowerToughness::fixed(2, 2));
    }
    game.create_object_from_card(&builder.build(), owner, Zone::Graveyard)
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn battlefield_object_named(
    game: &crate::game_state::GameState,
    name: &str,
) -> Option<ObjectId> {
    game.battlefield.iter().copied().find(|id| {
        game.object(*id)
            .is_some_and(|object| object.name == name && object.zone == Zone::Battlefield)
    })
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn replay_aberrant_return_returns_three_target_creatures_with_minus_counters() {
    let def = parse_oracle_card_definition("Aberrant Return");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let alice_creature = create_aberrant_return_graveyard_permanent(
        &mut game,
        "Alice Aberrant Bear",
        alice,
        vec![CardType::Creature],
    );
    let bob_creature = create_aberrant_return_graveyard_permanent(
        &mut game,
        "Bob Aberrant Bear",
        bob,
        vec![CardType::Creature],
    );
    let second_bob_creature = create_aberrant_return_graveyard_permanent(
        &mut game,
        "Bob Aberrant Soldier",
        bob,
        vec![CardType::Creature],
    );
    create_aberrant_return_graveyard_permanent(
        &mut game,
        "Ignored Aberrant Relic",
        bob,
        vec![CardType::Artifact],
    );

    let spell_id = game.create_object_from_definition(&def, alice, Zone::Stack);
    game.push_to_stack(
        crate::game_state::StackEntry::new(spell_id, alice).with_targets(vec![
            crate::game_state::Target::Object(alice_creature),
            crate::game_state::Target::Object(bob_creature),
            crate::game_state::Target::Object(second_bob_creature),
        ]),
    );

    crate::game_loop::resolve_stack_entry_with(
        &mut game,
        &mut crate::decision::SelectFirstDecisionMaker,
    )
    .expect("Aberrant Return should resolve with three legal creature-card targets");

    for name in [
        "Alice Aberrant Bear",
        "Bob Aberrant Bear",
        "Bob Aberrant Soldier",
    ] {
        let returned = battlefield_object_named(&game, name)
            .unwrap_or_else(|| panic!("{name} should be returned to the battlefield"));
        let object = game.object(returned).expect("returned object exists");
        assert_eq!(
            game.controller_of(object),
            alice,
            "Aberrant Return should put {name} onto the battlefield under its caster's control"
        );
        assert_eq!(
            game.counter_count(returned, crate::object::CounterType::MinusOneMinusOne),
            1,
            "{name} should enter with exactly one -1/-1 counter"
        );
    }

    assert!(
        battlefield_object_named(&game, "Ignored Aberrant Relic").is_none(),
        "noncreature graveyard cards should not be returned by Aberrant Return"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn aberrant_return_target_requirements_reject_noncreatures_and_cap_at_three() {
    let def = parse_oracle_card_definition("Aberrant Return");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let creature_targets = (0..4)
        .map(|idx| {
            create_aberrant_return_graveyard_permanent(
                &mut game,
                &format!("Aberrant Extra Target {idx}"),
                alice,
                vec![CardType::Creature],
            )
        })
        .collect::<Vec<_>>();
    let artifact = create_aberrant_return_graveyard_permanent(
        &mut game,
        "Aberrant Illegal Relic",
        alice,
        vec![CardType::Artifact],
    );

    let spell_id = game.create_object_from_definition(&def, alice, Zone::Stack);
    let requirements = crate::game_loop::extract_target_requirements_from_program_with_modes(
        &game,
        def.spell_effect
            .as_ref()
            .expect("Aberrant Return should have spell effects"),
        alice,
        Some(spell_id),
        None,
    );

    assert_eq!(
        requirements.len(),
        1,
        "Aberrant Return should have one target group"
    );
    assert_eq!(
        requirements[0].min_targets, 1,
        "Aberrant Return requires at least one target"
    );
    assert_eq!(
        requirements[0].max_targets,
        Some(3),
        "Aberrant Return allows at most three targets"
    );
    for creature in creature_targets {
        assert!(
            requirements[0]
                .legal_targets
                .contains(&crate::game_state::Target::Object(creature)),
            "creature cards in graveyards should be legal Aberrant Return targets"
        );
    }
    assert!(
        !requirements[0]
            .legal_targets
            .contains(&crate::game_state::Target::Object(artifact)),
        "noncreature graveyard cards should not be legal Aberrant Return targets"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_named_enters_tapped_and_doesnt_untap_fails_strictly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Grimgrin Variant")
        .parse_text("Grimgrin enters tapped and doesn't untap during your untap step.")
        .expect_err("mixed enters-tapped/negated-untap should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported mixed enters-tapped and negated-untap clause"),
        "expected strict mixed enters-tapped parse error, got {message}"
    );
}

#[cfg(ironsmith_runtime_removed_parser_helper_unit_tests)]
#[test]
pub(super) fn parse_at_trigger_intro_ignores_look_at_the_top_clause() {
    let tokens = tokenize_line("Look at the top seven cards of your library.", 0);
    assert!(
        !is_at_trigger_intro(&tokens, 1),
        "look-at clause must not be treated as trigger intro"
    );
}

#[cfg(ironsmith_runtime_removed_parser_helper_unit_tests)]
#[test]
pub(super) fn parse_at_trigger_intro_matches_beginning_clause() {
    let tokens = tokenize_line("At the beginning of your upkeep, draw a card.", 0);
    assert!(
        is_at_trigger_intro(&tokens, 0),
        "'at the beginning' should be treated as trigger intro"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_enchanted_creature_doesnt_untap_during_controller_untap_step() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sleep Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Enchant creature\nEnchanted creature doesn't untap during its controller's untap step.",
        )
        .expect("attached negated untap clause should parse as static grant");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&StaticAbilityId::AttachedAbilityGrant)
            || ids.contains(&StaticAbilityId::RuleRestriction),
        "expected attached ability grant static ability, got {ids:?}"
    );

    let compiled = unprocessed_compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        compiled.contains("enchanted creature don't untap during their controllers' untap steps")
            || compiled
                .contains("enchanted creature doesn't untap during its controller's untap step")
            || compiled
                .contains("enchanted creature doesnt untap during its controllers untap step"),
        "expected compiled text to keep attached untap restriction, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn fall_from_favor_runtime_game() -> (
    crate::game_state::GameState,
    PlayerId,
    PlayerId,
    ObjectId,
    ObjectId,
) {
    let fall_from_favor = parse_oracle_card_definition("Fall from Favor");
    let creature = CardDefinitionBuilder::new(CardId::new(), "Grizzly Bears")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let enchanted_creature = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
    let aura = game.create_object_from_definition(&fall_from_favor, alice, Zone::Battlefield);
    assert!(game.attach_object_to_target(
        aura,
        crate::object::AttachmentTarget::Object(enchanted_creature),
    ));
    (game, alice, bob, aura, enchanted_creature)
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn fall_from_favor_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Fall from Favor");
    let def = parse_oracle_card_definition("Fall from Favor");
    let rendered = canonical_compiled_lines(&def).join("\n");

    assert_eq!(
        rendered,
        "Enchant creature\nWhen this Aura enters, tap enchanted creature and you become the monarch.\nEnchanted creature doesnt untap during its controllers untap step unless that player is the monarch.",
        "Fall from Favor should preserve its Aura target, ETB tap/monarch trigger, and monarch-gated untap restriction"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn fall_from_favor_enters_taps_enchanted_creature_and_makes_controller_monarch() {
    let (mut game, alice, _bob, aura, enchanted_creature) = fall_from_favor_runtime_game();

    let enters_event = crate::events::RawEvent::new(
        crate::events::ZoneChangeEvent::with_cause(
            aura,
            Zone::Hand,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            None,
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let trigger_count = resolve_triggers_for_source(&mut game, aura, &enters_event);

    assert_eq!(
        trigger_count, 1,
        "Fall from Favor entering should create exactly one trigger"
    );
    assert!(
        game.is_tapped(enchanted_creature),
        "Fall from Favor should tap the enchanted creature when its trigger resolves"
    );
    assert_eq!(
        game.monarch,
        Some(alice),
        "Fall from Favor's controller should become the monarch"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn fall_from_favor_keeps_enchanted_creature_tapped_when_controller_is_not_monarch() {
    let (mut game, alice, bob, _aura, enchanted_creature) = fall_from_favor_runtime_game();
    game.set_monarch(Some(alice));
    game.tap(enchanted_creature);
    game.turn.active_player = bob;
    game.turn.phase = crate::game_state::Phase::Beginning;
    game.turn.step = Some(crate::game_state::Step::Untap);

    let mut dm = crate::decision::SelectFirstDecisionMaker;
    crate::turn::execute_untap_step_with(&mut game, &mut dm);

    assert!(
        game.is_tapped(enchanted_creature),
        "Fall from Favor should stop the enchanted creature from untapping when its controller is not the monarch"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn fall_from_favor_allows_enchanted_creature_to_untap_when_controller_is_monarch() {
    let (mut game, _alice, bob, _aura, enchanted_creature) = fall_from_favor_runtime_game();
    game.set_monarch(Some(bob));
    game.tap(enchanted_creature);
    game.turn.active_player = bob;
    game.turn.phase = crate::game_state::Phase::Beginning;
    game.turn.step = Some(crate::game_state::Step::Untap);
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    crate::turn::execute_untap_step_with(&mut game, &mut dm);

    assert!(
        !game.is_tapped(enchanted_creature),
        "Fall from Favor should allow the enchanted creature to untap when its controller is the monarch"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn animate_wall_strict_parser_compiled_text_and_model_regression() {
    assert_oracle_card_parses_strict("Animate Wall");

    let def = parse_oracle_card_definition("Animate Wall");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&StaticAbilityId::AttachedAbilityGrant),
        "Animate Wall should parse as an attached static ability grant, got {ids:?}"
    );
    let Some(AuraAttachmentFilter::Object(filter)) = &def.aura_attach_filter else {
        panic!(
            "Animate Wall should parse Enchant Wall as an object attachment filter, got {:?}",
            def.aura_attach_filter
        );
    };
    assert!(
        filter.subtypes.contains(&Subtype::Wall),
        "Animate Wall's attachment filter should require Wall subtype, got {filter:?}"
    );

    let compiled = compiled_text_lines(&def).join("\n");
    assert!(
        compiled.contains("Enchant Wall"),
        "Animate Wall should preserve its Wall enchant restriction, got {compiled}"
    );
    assert!(
        compiled
            .to_ascii_lowercase()
            .contains("enchanted wall can attack as though it didn't have defender"),
        "Animate Wall should preserve the as-though defender attack clause, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn animate_wall_allows_only_enchanted_wall_to_attack_through_defender() {
    let animate_wall = parse_oracle_card_definition("Animate Wall");
    let wall = CardDefinitionBuilder::new(CardId::new(), "Test Wall")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Wall])
        .power_toughness(PowerToughness::fixed(0, 4))
        .defender()
        .build();
    let non_wall = CardDefinitionBuilder::new(CardId::new(), "Defender Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .defender()
        .build();

    let alice = PlayerId::from_index(0);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let wall_id = game.create_object_from_definition(&wall, alice, Zone::Battlefield);
    let non_wall_id = game.create_object_from_definition(&non_wall, alice, Zone::Battlefield);
    let aura_id = game.create_object_from_definition(&animate_wall, alice, Zone::Battlefield);
    game.remove_summoning_sickness(wall_id);
    game.remove_summoning_sickness(non_wall_id);

    assert!(
        !crate::rules::combat::can_attack(
            game.object(wall_id).expect("wall exists before attachment"),
            &game,
        ),
        "a Wall with defender should not attack before Animate Wall is attached"
    );
    assert!(
        !crate::effects::permanents::attachment_can_attach_to_target(
            &game,
            aura_id,
            crate::object::AttachmentTarget::Object(non_wall_id),
        ),
        "Animate Wall's enchant restriction should reject non-Wall targets"
    );
    assert!(
        crate::effects::permanents::attach_battlefield_object_to_target(
            &mut game,
            aura_id,
            crate::object::AttachmentTarget::Object(wall_id),
        )
    );

    assert!(
        crate::rules::combat::can_attack(
            game.object(wall_id).expect("wall exists after attachment"),
            &game,
        ),
        "Animate Wall should let the enchanted Wall attack as though it didn't have defender"
    );
    assert!(
        !crate::rules::combat::can_attack(
            game.object(non_wall_id).expect("non-Wall defender exists"),
            &game,
        ),
        "Animate Wall should not let an unattached non-Wall defender attack"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_choose_not_to_untap_artifact_line_as_static_ability() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Endoskeleton Untap Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text("You may choose not to untap this artifact during your untap step.")
        .expect("choose-not-to-untap artifact clause should parse as static ability");

    assert!(
        def.spell_effect
            .as_ref()
            .map(|effects| effects.is_empty())
            .unwrap_or(true),
        "expected no spell effects for choose-not-to-untap static line"
    );

    let static_displays: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.display()),
            _ => None,
        })
        .collect();
    let lower_static = static_displays
        .iter()
        .map(|display| display.to_ascii_lowercase())
        .collect::<Vec<_>>();

    assert!(
        lower_static.iter().any(|display| {
            display.contains("you may choose not to untap this artifact during your untap step")
        }),
        "expected choose-not-to-untap static display, got {static_displays:?}"
    );
    assert!(
        !lower_static
            .iter()
            .any(|display| display.contains("unsupported parser line fallback")),
        "expected real parser static ability, not unsupported fallback marker: {static_displays:?}"
    );

    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        static_ids.contains(&StaticAbilityId::MayChooseNotToUntapDuringUntapStep),
        "expected typed optional-untap static ability, got {static_ids:?}"
    );
    assert!(
        !static_ids.contains(&StaticAbilityId::RuleFallbackText),
        "optional untap should not parse as placeholder static ability: {static_ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_choose_not_to_untap_line_and_activated_line_without_spurious_untap_effect() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Endoskeleton Pair Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "You may choose not to untap this artifact during your untap step.\n{2}, {T}: Target creature gets +0/+3 for as long as this artifact remains tapped.",
        )
        .expect("endoskeleton-style untap + activated lines should parse");

    let compiled = unprocessed_compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        compiled.contains("you may choose not to untap this artifact during your untap step"),
        "expected compiled output to retain choose-not-to-untap static line, got {compiled}"
    );
    assert!(
        !compiled.contains("spell effects: you may untap target artifact"),
        "unexpected untap-target spell effect leak in compiled output: {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_named_source_optional_untap_and_control_duration_surface() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Rubinia Soulsinger")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 3))
        .parse_text(
            "You may choose not to untap Rubinia Soulsinger during your untap step.\n{T}: Gain control of target creature for as long as you control Rubinia Soulsinger and Rubinia Soulsinger remains tapped.",
        )
        .expect("named source optional untap and control duration should parse");

    let compiled = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        compiled.contains("You may choose not to untap Rubinia Soulsinger during your untap step"),
        "expected named-source optional untap static line, got {compiled}"
    );
    assert!(
        compiled.contains(
            "{T}: Gain control of target creature for as long as you control Rubinia Soulsinger and Rubinia Soulsinger remains tapped"
        ),
        "expected named-source compound control duration, got {compiled}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("MayChooseNotToUntapDuringUntapStep")
            && debug.contains("ObjectControlledBy")
            && debug.contains("ObjectTapped")
            && debug.contains("Rubinia Soulsinger"),
        "expected typed optional untap plus sourced control duration, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_untap_during_each_other_players_untap_step_as_static_ability() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Seedborn Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Untap all permanents you control during each other player's untap step.")
        .expect("Seedborn Muse untap static ability should parse");
    let compiled = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        compiled.contains("untap all permanents you control during each other player's untap step"),
        "expected supported each-other-player untap rendering, got {compiled}"
    );
    let static_ids = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        static_ids.contains(&StaticAbilityId::UntapDuringEachOtherPlayersUntapStep),
        "expected dedicated untap-during-other-players-step static ability, got {static_ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_victory_chimes_keeps_singular_untap_step_line_static() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Victory Chimes")
        .card_types(vec![CardType::Artifact])
        .parse_text("Untap this artifact during each other player's untap step.\n{T}: A player of your choice adds {C}.")
        .expect("Victory Chimes should parse");

    assert!(
        def.spell_effect.is_none(),
        "victory chimes untap line should not lower as a spell effect: {:#?}",
        def.spell_effect
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("untap this artifact during each other player's untap step"),
        "expected static untap line in oracle-like output, got {rendered}"
    );
    assert!(
        rendered.contains("A player of your choice adds {C}."),
        "expected chosen-player mana wording in oracle-like output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_dark_deal_that_many_minus_one_keeps_prior_effect_reference() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dark Deal Variant")
        .parse_text("Each player discards all the cards in their hand, then draws that many cards minus one.")
        .expect("dark deal style clause should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("EffectValueOffset") || debug.contains("EffectMetricOffset"),
        "expected draw count to reference prior effect with offset, got {debug}"
    );
    assert!(
        debug.contains("-1"),
        "expected minus one offset in draw count, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_where_x_is_count_minus_fixed_preserves_negative_offset() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ivory Tower Variant")
        .parse_text("At the beginning of your upkeep, you gain X life, where X is the number of cards in your hand minus 4.")
        .expect("where-x count-minus-fixed clause should parse");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected upkeep triggered ability");
    let gain = triggered
        .effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<GainLifeEffect>())
        .expect("expected gain-life effect");

    let crate::effect::Value::Add(left, right) = gain.amount.unhinted() else {
        panic!("expected additive life gain amount, got {:?}", gain.amount);
    };
    assert!(
        matches!(
            left.unhinted(),
            crate::effect::Value::CardsInHand(PlayerFilter::You)
        ) || matches!(
            left.unhinted(),
            crate::effect::Value::Count(filter)
                if filter.zone == Some(Zone::Hand)
                    && filter.owner == Some(PlayerFilter::You)
        ),
        "expected left side to count cards in hand, got {left:?}"
    );
    assert!(
        matches!(right.as_ref(), crate::effect::Value::Fixed(-4)),
        "expected minus-four offset, got {right:?}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("minus 4"),
        "expected compiled text to preserve minus-four offset, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_hellion_eruption_that_many_keeps_prior_effect_reference() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Hellion Eruption Variant")
        .parse_text("Sacrifice all creatures you control, then create that many 4/4 red Hellion creature tokens.")
        .expect("hellion eruption style clause should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("CreateTokenEffect"),
        "expected token creation effect, got {debug}"
    );
    assert!(
        debug.contains("EffectValue"),
        "expected token count to reference prior effect result, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_can_block_only_creatures_with_flying_static_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Cloud Djinn Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Flying\nThis creature can block only creatures with flying.")
        .expect("can-block-only-flying static clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::CanBlockOnlyFlying),
        "expected can-block-only-flying static ability, got {ids:?}"
    );

    let compiled = crate::compiled_text::unprocessed_compiled_lines(&def).join("\n");
    assert!(
        compiled
            .to_ascii_lowercase()
            .contains("can block only creatures with flying"),
        "expected compiled text to keep can-block-only-flying clause, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_dragon_hunter_strict_and_renders_reach_blocking_clause() {
    assert_oracle_card_parses_strict("Dragon Hunter");
    let def = parse_oracle_card_definition("Dragon Hunter");

    assert!(
        def.abilities.iter().any(|ability| matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.can_block_as_though_reach_subtype() == Some(Subtype::Dragon)
        )),
        "expected Dragon Hunter to block Dragons as though it had reach, got {:?}",
        def.abilities
    );

    let compiled = canonical_compiled_lines(&def).join("\n");
    assert!(
        compiled
            .to_ascii_lowercase()
            .contains("this creature can block dragons as though it had reach"),
        "expected compiled text to keep Dragon Hunter's reach-blocking clause, got {compiled}"
    );
}

#[test]
pub(super) fn dragon_hunter_blocks_flying_dragons_but_not_other_fliers() {
    let dragon_hunter = parse_oracle_card_definition("Dragon Hunter");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);

    let hunter_id = game.create_object_from_definition(&dragon_hunter, bob, Zone::Battlefield);
    let flying_dragon = CardDefinitionBuilder::new(CardId::new(), "Flying Dragon Attacker")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Dragon])
        .flying()
        .build();
    let flying_dragon_id =
        game.create_object_from_definition(&flying_dragon, alice, Zone::Battlefield);
    let flying_bird = CardDefinitionBuilder::new(CardId::new(), "Flying Bird Attacker")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Bird])
        .flying()
        .build();
    let flying_bird_id = game.create_object_from_definition(&flying_bird, alice, Zone::Battlefield);

    let hunter = game
        .object(hunter_id)
        .expect("Dragon Hunter exists")
        .clone();
    let dragon = game
        .object(flying_dragon_id)
        .expect("flying Dragon exists")
        .clone();
    let bird = game
        .object(flying_bird_id)
        .expect("flying Bird exists")
        .clone();

    assert!(
        crate::rules::combat::can_block(&dragon, &hunter, &game),
        "Dragon Hunter should block flying Dragons as though it had reach"
    );
    assert!(
        !crate::rules::combat::can_block(&bird, &hunter, &game),
        "Dragon Hunter should not get reach against non-Dragon fliers"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_cant_be_blocked_by_creatures_with_power_or_less_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Arlinn's Wolf Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't be blocked by creatures with power 2 or less.")
        .expect("cant-be-blocked-by-power clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::CantBeBlockedByPowerOrLess),
        "expected cant-be-blocked-by-power static ability, got {ids:?}"
    );

    let compiled = crate::compiled_text::unprocessed_compiled_lines(&def).join("\n");
    assert!(
        compiled.to_ascii_lowercase().contains("power 2 or less"),
        "expected compiled text to include power threshold, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_cant_be_blocked_by_creatures_with_power_or_greater_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Amrou Kithkin Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't be blocked by creatures with power 3 or greater.")
        .expect("cant-be-blocked-by-power clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::CantBeBlockedByPowerOrGreater),
        "expected cant-be-blocked-by-power static ability, got {ids:?}"
    );

    let compiled = crate::compiled_text::unprocessed_compiled_lines(&def).join("\n");
    assert!(
        compiled.to_ascii_lowercase().contains("power 3 or greater"),
        "expected compiled text to include power threshold, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_wandering_wolf_relative_power_blocking_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Wandering Wolf")
        .card_types(vec![CardType::Creature])
        .parse_text("Creatures with power less than this creature's power can't block it.")
        .expect("wandering wolf blocking clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(
            &crate::static_abilities::StaticAbilityId::CantBeBlockedByLowerPowerThanSource
        ),
        "expected relative-power blocking static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::Skulk),
        "wandering wolf text must not collapse into skulk, got {ids:?}"
    );

    let compiled = crate::compiled_text::unprocessed_compiled_lines(&def).join("\n");
    assert!(
        compiled
            .to_ascii_lowercase()
            .contains("creatures with power less than this creature's power can't block it"),
        "expected compiled text to preserve wandering wolf wording, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_cant_attack_unless_defending_player_controls_island_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Deep-Sea Serpent Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't attack unless defending player controls an Island.")
        .expect("cant-attack-unless-defending-controls-island should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::CantAttackUnlessCondition),
        "expected generic cant-attack-unless condition restriction, got {ids:?}"
    );
    assert!(
        !ids.contains(&StaticAbilityId::RuleFallbackText),
        "defending-player-land-subtype restriction should not emit rule text placeholders, got {ids:?}"
    );

    let compiled = crate::compiled_text::unprocessed_compiled_lines(&def).join("\n");
    assert!(
        compiled
            .to_ascii_lowercase()
            .contains("defending player controls an island")
            || compiled
                .to_ascii_lowercase()
                .contains("defending player controls island"),
        "expected compiled text to include defending-player island condition, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_cant_attack_unless_youve_cast_creature_spell_this_turn_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Goblin Cohort Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't attack unless you've cast a creature spell this turn.")
        .expect("cant-attack-unless-youve-cast-creature-spell should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(
            &crate::static_abilities::StaticAbilityId::CantAttackUnlessControllerCastCreatureSpellThisTurn
        ),
        "expected cast-creature-spell attack restriction, got {ids:?}"
    );
    assert!(
        !ids.contains(&StaticAbilityId::RuleFallbackText),
        "cast-creature-spell attack restriction should not emit rule text placeholders, got {ids:?}"
    );

    let compiled = crate::compiled_text::unprocessed_compiled_lines(&def).join("\n");
    assert!(
        compiled
            .to_ascii_lowercase()
            .contains("can't attack unless you've cast a creature spell this turn")
            || compiled
                .to_ascii_lowercase()
                .contains("cant attack unless youve cast a creature spell this turn"),
        "expected compiled text to include cast-creature-spell condition, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_cant_attack_unless_youve_cast_noncreature_spell_this_turn_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mercurial Spelldancer Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't attack unless you've cast a noncreature spell this turn.")
        .expect("cant-attack-unless-youve-cast-noncreature-spell should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(
            &crate::static_abilities::StaticAbilityId::CantAttackUnlessControllerCastNonCreatureSpellThisTurn
        ),
        "expected cast-noncreature-spell attack restriction, got {ids:?}"
    );
    assert!(
        !ids.contains(&StaticAbilityId::RuleFallbackText),
        "cast-noncreature-spell attack restriction should not emit rule text placeholders, got {ids:?}"
    );

    let compiled = crate::compiled_text::unprocessed_compiled_lines(&def).join("\n");
    assert!(
        compiled
            .to_ascii_lowercase()
            .contains("can't attack unless you've cast a noncreature spell this turn")
            || compiled
                .to_ascii_lowercase()
                .contains("cant attack unless youve cast a noncreature spell this turn"),
        "expected compiled text to include cast-noncreature-spell condition, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_cant_attack_unless_control_more_creatures_than_defending_player_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Bog Hoodlums Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature can't attack unless you control more creatures than defending player.",
        )
        .expect("cant-attack-unless-control-more-creatures should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::CantAttackUnlessCondition),
        "expected typed cant-attack-unless condition static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&StaticAbilityId::RuleFallbackText),
        "control-more-creatures restriction should not emit placeholders, got {ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_cant_attack_or_block_unless_you_control_seven_or_more_lands_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Topiary Stomper Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't attack or block unless you control seven or more lands.")
        .expect("cant-attack-or-block-unless-control-seven-lands should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("can't attack or block unless you control seven or more lands")
            || rendered.contains("cant attack or block unless you control seven or more lands"),
        "expected conditioned attack/block restriction text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_cant_attack_unless_defending_player_is_poisoned_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Skullsnatcher Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't attack unless defending player is poisoned.")
        .expect("cant-attack-unless-defending-player-is-poisoned should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::CantAttackUnlessCondition),
        "expected typed cant-attack-unless condition static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&StaticAbilityId::RuleFallbackText),
        "defending-player-poisoned restriction should not emit placeholders, got {ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_cant_attack_unless_black_or_green_creature_also_attacks_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Goblin War Drums Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't attack unless a black or green creature also attacks.")
        .expect("cant-attack-unless-black-or-green-creature-also-attacks should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::CantAttackUnlessCondition),
        "expected typed cant-attack-unless condition static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&StaticAbilityId::RuleFallbackText),
        "also-attacks restriction should not emit placeholders, got {ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_cant_attack_unless_sacrifice_a_land_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Exalted Dragon Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't attack unless you sacrifice a land.")
        .expect("cant-attack-unless-sacrifice-land should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::CantAttackUnlessCondition),
        "expected typed cant-attack-unless condition static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&StaticAbilityId::RuleFallbackText),
        "sacrifice-land restriction should not emit placeholders, got {ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_cant_attack_unless_sacrifice_two_islands_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Leviathan Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't attack unless you sacrifice two islands.")
        .expect("cant-attack-unless-sacrifice-two-islands should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::CantAttackUnlessCondition),
        "expected typed cant-attack-unless condition static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&StaticAbilityId::RuleFallbackText),
        "sacrifice-two-islands restriction should not emit placeholders, got {ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_cant_attack_unless_pay_per_plus_one_plus_one_counter_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Phyrexian Marauder Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't attack unless you pay {1} for each +1/+1 counter on it.")
        .expect("cant-attack-unless-pay-per-counter should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::CantAttackUnlessCondition),
        "expected typed cant-attack-unless condition static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&StaticAbilityId::RuleFallbackText),
        "pay-per-counter restriction should not emit placeholders, got {ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_cant_attack_unless_defending_player_is_the_monarch_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Crown-Hunter Hireling Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't attack unless defending player is the monarch.")
        .expect("cant-attack-unless-defending-player-is-the-monarch should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::CantAttackUnlessCondition),
        "expected typed cant-attack-unless condition static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&StaticAbilityId::RuleFallbackText),
        "monarch restriction should not emit placeholders, got {ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_collective_restraint_domain_attack_tax_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Collective Restraint Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Creatures can't attack you unless their controller pays {X} for each creature they control that's attacking you, where X is the number of basic land types among lands you control.",
        )
        .expect("collective restraint domain attack tax line should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(
            &crate::static_abilities::StaticAbilityId::CantAttackYouUnlessControllerPaysPerAttackerBasicLandTypesAmongLandsYouControl
        ),
        "expected collective-restraint attack tax static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&StaticAbilityId::RuleFallbackText),
        "collective-restraint line should not emit rule text placeholders, got {ids:?}"
    );

    let compiled = crate::compiled_text::unprocessed_compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        compiled.contains(
            "unless their controller pays {x} for each creature they control thats attacking you"
        ) || compiled.contains(
            "unless their controller pays {x} for each creature they control that's attacking you"
        ),
        "expected compiled text to include collective-restraint tax clause, got {compiled}"
    );
    assert!(
        compiled.contains("basic land types among lands you control"),
        "expected compiled text to include domain clause, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_fixed_attack_tax_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ghostly Prison Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Creatures can't attack you unless their controller pays {2} for each creature they control that's attacking you.",
        )
        .expect("fixed attack tax line should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(
            &crate::static_abilities::StaticAbilityId::CantAttackYouUnlessControllerPaysPerAttacker
        ),
        "expected fixed attack-tax static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&StaticAbilityId::RuleFallbackText),
        "fixed attack-tax line should not emit rule text placeholders, got {ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_resolving_attack_tax_as_a_source_independent_temporary_restriction() {
    let oracle = "When this creature enters, until your next turn, creatures can't attack you or planeswalkers you control unless their controller pays {2} for each of those creatures.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Forbidding Spirit Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(oracle)
        .expect("a triggered temporary per-attacker tax should parse");

    assert_eq!(unprocessed_compiled_lines(&def), vec![oracle.to_string()]);
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("AttackYouUnlessControllerPaysPerAttacker")
            && debug.contains("2,")
            && debug.contains("duration: YourNextTurn")
            && debug.contains("LeadingUntilYourNextTurn"),
        "expected a typed temporary attack-tax restriction with its authored duration: {debug}"
    );
    assert!(
        !debug.contains("UnlessPaysEffect") && !debug.contains("ApplyContinuousEffect"),
        "the attack tax is a resolving rule, not a payment made during the ETB trigger or an ability granted to its source: {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_orzhov_advokist_strictly_and_renders_attack_defender_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Orzhov Advokist")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Advisor])
        .power_toughness(PowerToughness::fixed(1, 4))
        .parse_text(
            "At the beginning of your upkeep, each player may put two +1/+1 counters on a creature they control. If a player does, creatures that player controls can't attack you or planeswalkers you control until your next turn.",
        )
        .expect("Orzhov Advokist should parse strictly");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        !ids.contains(&StaticAbilityId::RuleFallbackText)
            && !ids.contains(&StaticAbilityId::UnsupportedParserLine),
        "Orzhov Advokist should not emit parser fallback placeholders, got {ids:?}"
    );
    assert!(
        def.abilities
            .iter()
            .any(|ability| matches!(ability.kind, AbilityKind::Triggered(_))),
        "Orzhov Advokist should compile to a triggered upkeep ability"
    );

    let compiled = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        compiled.contains("if a player does") || compiled.contains("if that player does"),
        "expected compiled text to preserve the per-player conditional, got {compiled}"
    );
    assert!(
        compiled.contains(
            "creatures that player controls can't attack you or planeswalkers you control until your next turn"
        ),
        "expected compiled text to include the attack-defender restriction, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_morph_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Morph Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Morph {3}{R}")
        .expect("morph keyword line should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::Morph),
        "expected morph static ability, got {ids:?}"
    );

    let compiled = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        compiled.to_ascii_lowercase().contains("morph") && compiled.contains("{3}{R}"),
        "expected morph line in compiled text, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_megamorph_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Megamorph Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Megamorph {5}{G}")
        .expect("megamorph keyword line should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::Megamorph),
        "expected megamorph static ability, got {ids:?}"
    );

    let compiled = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        compiled.to_ascii_lowercase().contains("megamorph") && compiled.contains("{5}{G}"),
        "expected megamorph line in compiled text, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_disguise_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Disguise Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Disguise {R/W}{R/W}")
        .expect("disguise keyword line should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::Disguise),
        "expected disguise static ability, got {ids:?}"
    );

    let compiled = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        compiled.to_ascii_lowercase().contains("disguise") && compiled.contains("{R/W}{R/W}"),
        "expected disguise line in compiled text, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_morph_keyword_line_with_trailing_clause_fails() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Morph Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Morph {3}{R} reveal this card")
        .expect_err("morph keyword with trailing clause should fail");

    let message = format!("{err:?}").to_ascii_lowercase();
    assert!(
        message.contains("unsupported morph cost clause"),
        "expected trailing morph clause parse error, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_zombie_cutthroat_morph_life_cost_stays_static() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Zombie Cutthroat")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 4))
        .parse_text(
            "Morph—Pay 5 life. (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)",
        )
        .expect("Zombie Cutthroat morph cost should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::Morph),
        "expected Zombie Cutthroat to compile as a morph static ability, got {ids:?}"
    );
    assert!(
        def.spell_effect.is_none(),
        "morph life cost should not lower into a top-level spell effect: {:?}",
        def.spell_effect
    );

    let compiled = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        compiled.contains("Morph") && compiled.contains("Pay 5 life"),
        "expected Zombie Cutthroat compiled text to keep its morph life cost, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_morph_keyword_line_with_sacrifice_cost() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Morph Sacrifice Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Morph—Sacrifice a creature.")
        .expect("morph sacrifice cost should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::Morph),
        "expected morph sacrifice cost to compile as morph, got {ids:?}"
    );

    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        debug.contains("sacrificeeffect"),
        "expected morph sacrifice cost to use a checked payment effect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_morph_keyword_line_with_non_cost_effect_fails_loudly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Morph Draw Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Morph—Draw a card.")
        .expect_err("non-cost morph payment should fail loudly");

    let err = format!("{err:?}").to_ascii_lowercase();
    assert!(
        err.contains("cost") || err.contains("cost-executable"),
        "expected loud morph cost error, got {err}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_banding_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Banding Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Banding")
        .expect("banding should parse as a supported combat keyword");

    assert!(
        def.abilities.iter().any(|ability| matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::Banding
        )),
        "expected banding to lower to StaticAbilityId::Banding"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_umbra_armor_keyword_line_lowers_to_static_ability() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Umbra Armor Variant")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text("Umbra armor")
        .expect("umbra armor keyword line should parse");

    let compiled = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        compiled.contains("Umbra armor"),
        "expected umbra armor text in compiled output, got {compiled}"
    );
    let debug = format!("{def:?}");
    assert!(
        !debug.contains("KeywordMarker") && !debug.contains("MarkerText"),
        "umbra armor should lower to a real static ability, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_filter_power_numeric_comparison_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Power Filter Variant")
        .parse_text("Destroy target creature with power 2 or less.")
        .expect("numeric power comparison should parse");

    let effects = def.spell_effect.as_ref().expect("spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("power: Some(LessThanOrEqual(2))"),
        "expected parsed power comparison constraint, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_counter_spell_with_power_or_toughness_filter() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Stern Scolding Variant")
        .parse_text("Counter target creature spell with power or toughness 2 or less.")
        .expect("power-or-toughness spell filter should parse");

    let effects = def.spell_effect.as_ref().expect("spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("any_of:")
            && debug.contains("power: Some(LessThanOrEqual(2))")
            && debug.contains("toughness: Some(LessThanOrEqual(2))"),
        "expected disjunctive power/toughness spell filter, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_filter_dynamic_power_comparison_preserves_typed_operand() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dynamic Power Filter Variant")
        .parse_text("Exile target creature with power greater than or equal to your life total.")
        .expect("dynamic power comparison should parse");
    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("GreaterThanOrEqualExpr") && debug.contains("LifeTotal(You)"),
        "expected typed dynamic-power comparison, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_return_up_to_x_target_creatures_preserves_dynamic_optional_count() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dynamic Return Count Variant")
        .parse_text(
            "Return up to X target creatures to their owners' hands, where X is one plus the number of cards named Aether Burst in all graveyards as you cast this spell.",
        )
        .expect("up-to-X target return clause should parse");
    let message = format!("{:?}", def.spell_effect);
    assert!(
        message.contains("dynamic_x: true") && message.contains("up_to_x: true"),
        "expected optional dynamic target-count in compiled effect, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_destroy_up_to_x_other_targets_preserves_dynamic_count() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dynamic Destroy Count Variant")
            .parse_text(
                "Destroy target creature and up to X other target creatures, where X is the number of Attractions you've visited this turn.",
            )
            .expect("dynamic multi-target destroy should parse");
    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.matches("DestroyEffect").count() == 2
            && debug.contains("dynamic_x: true")
            && debug.contains("up_to_x: true")
            && debug.contains("Attraction")
            && debug.contains("other: true"),
        "expected a fixed target plus up-to-X other targets, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_loses_all_abilities_and_becomes_effect_fails_instead_of_partial_parse() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Lose Abilities Becomes Effect Variant")
            .parse_text(
                "Until end of turn, target creature loses all abilities and becomes a blue Frog with base power and toughness 1/1.",
            )
            .expect_err("unsupported lose-all-abilities+becomes effect should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported loses-all-abilities with becomes clause")
            || message.contains("unsupported lose-all-abilities static becomes clause"),
        "expected strict loses-all-abilities+becomes effect parse error, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_loses_all_abilities_and_becomes_static_lowers_all_characteristics() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Lose Abilities Becomes Static Variant")
            .parse_text(
                "Each noncreature artifact loses all abilities and becomes an artifact creature with power and toughness each equal to its mana value.",
            )
            .expect("static lose-all-abilities and becomes clause should parse");
    let static_ids = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        static_ids.contains(&StaticAbilityId::RemoveAllAbilitiesForFilter)
            && static_ids.contains(&StaticAbilityId::SetCardTypes)
            && static_ids.contains(&StaticAbilityId::SetBasePowerToughnessForFilter),
        "expected all three typed characteristic-setting abilities, got {static_ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_each_player_exile_sacrifice_return_this_way_keeps_exact_result_set() {
    let oracle = "Each player exiles all creature cards from their graveyard, then sacrifices all creatures they control, then puts all cards they exiled this way onto the battlefield.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Living Death Variant")
        .parse_text(oracle)
        .expect("each-player exile/sacrifice/return should parse");
    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("TaggedEffect")
            && debug.contains("PutOntoBattlefieldEffect")
            && debug.matches("__sentence_helper_exiled_this_way").count() >= 2,
        "the return must consume the exact tagged exile result, got {debug}"
    );
    assert_eq!(unprocessed_compiled_lines(&def).join(" "), oracle);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_combat_damage_to_creature_trigger_parses_with_damaged_creature_reference() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Combat Damage Creature Trigger Variant")
        .parse_text(
            "Whenever this creature deals combat damage to a creature, you gain 2 life unless that creature's controller pays {2}.",
        )
        .expect("combat-damage-to-creature trigger should parse");
    let joined = unprocessed_compiled_lines(&def).join(" ").to_lowercase();
    assert!(
        joined.contains("deals combat damage to creature")
            || joined.contains("deals combat damage to a creature"),
        "expected combat-damage-to-creature trigger text, got {joined}"
    );
    assert!(
        joined.contains("unless its controller pays {2}")
            || joined.contains("unless that object's controller pays {2}"),
        "expected damaged-creature controller reference to be preserved, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_target_player_loses_the_game_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Lose Game Target Variant")
        .parse_text("{W}{W}, {T}: Target player loses the game.")
        .expect("target-player lose-game clause should parse");
    let joined = unprocessed_compiled_lines(&def).join(" ").to_lowercase();
    assert!(
        joined.contains("target player loses the game"),
        "expected target-player lose-game text, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_trigger_target_opponent_creates_treasure_tokens() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Target Opponent Creates Token Variant")
        .parse_text("When this creature dies, target opponent creates two Treasure tokens.")
        .expect("target-opponent create-token trigger should parse");
    let joined = unprocessed_compiled_lines(&def).join(" ").to_lowercase();
    assert!(
        joined.contains("target opponent creates 2 treasure tokens")
            || joined.contains("target opponent creates two treasure tokens")
            || joined.contains("create two treasure tokens under target opponent's control"),
        "expected targeted opponent token creation text, got {joined}"
    );
}
