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
use super::shard_18::*;
use super::shard_19::*;
use super::shard_20::*;
use super::shard_21::*;
use super::shard_23::*;
use super::*;

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn alien_invasion_preserves_source_counter_count_and_created_token_target() {
    let def = parse_oracle_card_definition("Alien Invasion");
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Alien Invasion should have a combat trigger");
    let effects = triggered.effects.flattened_default_effects();

    let (created_tag, create) = effects
        .iter()
        .find_map(|effect| {
            let tagged = effect.downcast_ref::<TaggedEffect>()?;
            let create = tagged.effect.downcast_ref::<CreateTokenEffect>()?;
            Some((&tagged.tag, create))
        })
        .expect("Alien Invasion should tag the created Alien for its follow-up");
    assert!(
        format!("{:#?}", create.token.abilities).contains("MustAttack"),
        "the Alien token must retain its quoted attacks-each-combat ability"
    );

    let plus_one = effects
        .iter()
        .filter_map(|effect| {
            effect
                .downcast_ref::<crate::effects::PutCountersEffect>()
                .or_else(|| {
                    effect
                        .downcast_ref::<TaggedEffect>()?
                        .effect
                        .downcast_ref::<crate::effects::PutCountersEffect>()
                })
        })
        .find(|put| put.counter_type == CounterType::PlusOnePlusOne)
        .expect("Alien Invasion should put +1/+1 counters on the created token");
    assert!(
        matches!(plus_one.target.unhinted(), ChooseSpec::Tagged(tag) if tag == created_tag),
        "the +1/+1 counter target should be the created Alien, got {:?}",
        plus_one.target
    );
    let Value::CountersOn(source, Some(CounterType::Named("invasion"))) =
        plus_one.amount.unhinted()
    else {
        panic!(
            "the +1/+1 amount should count invasion counters on the source, got {:?}",
            plus_one.amount
        );
    };
    assert!(matches!(source.unhinted(), ChooseSpec::Source));
    assert_eq!(
        source
            .source_reference_surface()
            .map(SourceReferenceSurface::display_text),
        Some("this enchantment".to_string())
    );

    let rendered = compiled_text_lines(&def).join(" ");
    assert!(
        rendered.contains("This token attacks each combat if able")
            && rendered.contains(
                "Put a +1/+1 counter on it for each invasion counter on this enchantment"
            ),
        "Alien Invasion should preserve both the nested token rule and source-counter basis: {rendered}"
    );
    assert!(
        !rendered.contains("for each enchantment on the battlefield"),
        "the invasion-counter count must not degrade into a battlefield filter: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_vanguard_seraph_preserves_first_time_trigger_surface() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Vanguard Seraph")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Flying\nWhenever you gain life for the first time each turn, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)",
        )
        .expect("Vanguard Seraph should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Whenever you gain life for the first time each turn, surveil 1.")
            || (rendered.contains("Whenever you gain life, surveil 1.")
                && rendered.contains("This ability triggers only once each turn")),
        "expected render to preserve first-time trigger surface, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_vengeful_warchief_keeps_first_life_loss_trigger_shape() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Vengeful Warchief")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Orc, Subtype::Warrior])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "Whenever you lose life for the first time each turn, put a +1/+1 counter on this creature.",
        )
        .expect("Vengeful Warchief should parse");

    let ability = def
        .abilities
        .iter()
        .find(|ability| matches!(ability.kind, AbilityKind::Triggered(_)))
        .expect("Vengeful Warchief should have a triggered ability");
    let AbilityKind::Triggered(triggered) = &ability.kind else {
        unreachable!("ability was filtered to triggered");
    };
    let life_loss_trigger = triggered
        .trigger
        .downcast_ref::<crate::triggers::PlayerLosesLifeTrigger>()
        .expect("expected a player loses life trigger");
    assert_eq!(
        life_loss_trigger.player,
        crate::target::PlayerFilter::You,
        "expected Vengeful Warchief to watch you losing life"
    );
    assert!(
        life_loss_trigger.during_turn.is_none(),
        "expected Vengeful Warchief to trigger any turn"
    );
    assert_eq!(
        triggered.intervening_if,
        Some(crate::ConditionExpr::FirstTimeThisTurn),
        "expected Vengeful Warchief to preserve the first-time-each-turn trigger condition"
    );
    let [segment] = triggered.effects.segments.as_slice() else {
        panic!("expected one resolution segment");
    };
    let [effect] = segment.default_effects.as_slice() else {
        panic!("expected one default effect");
    };
    let put_counters = effect
        .downcast_ref::<crate::effects::PutCountersEffect>()
        .expect("expected the trigger to put counters");
    assert_eq!(
        put_counters.counter_type,
        crate::object::CounterType::PlusOnePlusOne,
        "expected the trigger to place +1/+1 counters"
    );
    assert_eq!(
        put_counters.amount,
        crate::effect::Value::Fixed(1),
        "expected the trigger to add exactly one counter"
    );
    assert_eq!(
        *put_counters.target.unhinted(),
        crate::target::ChooseSpec::Source,
        "expected the trigger to counter this creature"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert_eq!(
        rendered,
        "Whenever you lose life for the first time each turn, put a +1/+1 counter on this creature.",
        "expected debug-safe compiled text to preserve the first-time wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn coax_from_the_blind_eternities_lowers_to_face_up_exile_choice_bundle() {
    let def = CardDefinitionBuilder::new(
        CardId::from_raw(1),
        "Coax from the Blind Eternities",
    )
    .mana_cost(ManaCost::from_pips(vec![
        vec![ManaSymbol::Generic(2)],
        vec![ManaSymbol::Blue],
    ]))
    .card_types(vec![CardType::Sorcery])
    .parse_text(
        "You may reveal an Eldrazi card you own from outside the game or choose a face-up Eldrazi card you own in exile. Put that card into your hand.",
    )
    .expect("Coax from the Blind Eternities should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("MayEffect")
            && debug.contains("ChooseObjectsEffect")
            && debug.contains("zone: Some(OutsideGame)")
            && debug.contains("additional_zones: [Exile]")
            && debug.contains("face_down: Some(false)")
            && debug.contains("RevealTaggedEffect")
            && debug.contains("MoveToZoneEffect"),
        "expected Coax to lower into a may/choose-across-zones/reveal/move bundle, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert_eq!(
        rendered,
        "You may reveal an Eldrazi card you own from outside the game or choose a face-up Eldrazi card you own in exile. Put that card into your hand.",
        "expected Coax to render both outside-game and exile choice surfaces"
    );
}

pub(super) struct ChooseFaceUpEldraziDecisionMaker {
    pub(super) expected: ObjectId,
}

impl crate::decision::DecisionMaker for ChooseFaceUpEldraziDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &crate::game_state::GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        true
    }

    fn decide_objects(
        &mut self,
        _game: &crate::game_state::GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        assert_eq!(
            ctx.candidates.len(),
            1,
            "expected only the face-up Eldrazi exile candidate, got {:?}",
            ctx.candidates
        );
        assert_eq!(ctx.candidates[0].id, self.expected);
        assert!(
            ctx.candidates[0].legal,
            "expected the candidate to be legal"
        );
        vec![self.expected]
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn coax_from_the_blind_eternities_puts_the_face_up_exiled_eldrazi_into_hand() {
    let coax_def = CardDefinitionBuilder::new(
        CardId::from_raw(1),
        "Coax from the Blind Eternities",
    )
    .mana_cost(ManaCost::from_pips(vec![
        vec![ManaSymbol::Generic(2)],
        vec![ManaSymbol::Blue],
    ]))
    .card_types(vec![CardType::Sorcery])
    .parse_text(
        "You may reveal an Eldrazi card you own from outside the game or choose a face-up Eldrazi card you own in exile. Put that card into your hand.",
    )
    .expect("Coax from the Blind Eternities should parse");

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);

    let hidden_titan_def = CardDefinitionBuilder::new(CardId::from_raw(2), "Hidden Titan")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Eldrazi])
        .build();
    let emrakul_id = game.create_object_from_definition(
        &crate::cards::definitions::emrakul_the_promised_end(),
        alice,
        crate::zone::Zone::Exile,
    );
    let emrakul_stable_id = game.object(emrakul_id).expect("Emrakul exists").stable_id;
    let hidden_titan_id =
        game.create_object_from_definition(&hidden_titan_def, alice, crate::zone::Zone::Exile);
    game.set_face_down(hidden_titan_id);
    let source = game.create_object_from_definition(&coax_def, alice, crate::zone::Zone::Stack);

    let mut dm = ChooseFaceUpEldraziDecisionMaker {
        expected: emrakul_id,
    };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);

    let program = coax_def.spell_effect.as_ref().expect("spell effect");
    for effect in &program.segments[0].default_effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Coax from the Blind Eternities effect should resolve");
    }

    let current_emrakul_id = game
        .find_object_by_stable_id(emrakul_stable_id)
        .expect("the chosen Eldrazi should still be tracked by stable id");
    assert_eq!(
        game.object(current_emrakul_id).map(|object| object.zone),
        Some(crate::zone::Zone::Hand),
        "expected the face-up Eldrazi card to move to hand"
    );
    assert_eq!(
        game.object(hidden_titan_id).map(|object| object.zone),
        Some(crate::zone::Zone::Exile),
        "expected the face-down Eldrazi card to stay in exile"
    );
    assert!(
        game.is_face_down(hidden_titan_id),
        "expected the hidden Titan to remain face down"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_cranial_ram_keeps_only_x_dynamic() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cranial Ram Variant")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .parse_text(
            "Living weapon (When this Equipment enters, create a 0/0 black Phyrexian Germ creature token, then attach this to it.)\n\
             Equipped creature gets +X/+1, where X is the number of artifacts you control.\n\
             Equip {2}",
        )
        .expect("Cranial Ram text should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("power: PerCount")
            && abilities_debug.contains("toughness: Fixed(1)"),
        "expected Cranial Ram to keep only power dynamic, got {abilities_debug}"
    );

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains(
            "equipped creature gets +x/+1, where x is the number of artifacts you control"
        ) && joined.contains("equip {2}"),
        "expected Cranial Ram to preserve the mixed X/+1 wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_stunted_growth_keeps_random_hand_reveal_and_top_of_library_link() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Stunted Growth")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Green],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Target player chooses three cards from their hand and puts them on top of their library in any order.",
        )
        .expect("Stunted Growth text should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("target player chooses three cards from their hand")
            && rendered.contains("puts them on top of their library")
            && !rendered.contains("that object on top of its owner's library"),
        "expected the Stunted Growth compile surface to stay oracle-like, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_mortuary_preserves_graveyard_origin_and_your_library_link() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mortuary")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever a creature is put into your graveyard from the battlefield, put that card on top of your library.",
        )
        .expect("Mortuary text should parse");

    let ability_debug = format!("{:#?}", def.abilities);
    let ability_debug_compact = ability_debug.split_whitespace().collect::<String>();
    assert!(
        ability_debug_compact.contains("ZoneChangeTrigger")
            && ability_debug_compact.contains("from:Specific(Battlefield)")
            && ability_debug_compact.contains("to:Specific(Graveyard)")
            && ability_debug_compact.contains("owner:Some(You")
            && ability_debug_compact.contains("card_types:[Creature")
            && ability_debug_compact.contains("MoveToZoneEffect")
            && ability_debug_compact.contains("zone:Library")
            && ability_debug_compact.contains("to_top:true"),
        "expected Mortuary to lower to an owned-creature battlefield-to-graveyard trigger that moves the triggering card to library top, got {ability_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered.contains(
            "Whenever a creature is put into your graveyard from the battlefield, put that card on top of your library"
        ) && !rendered_lower.contains("creature you own dies")
            && !rendered_lower.contains("its owner's library"),
        "expected Mortuary to preserve the graveyard origin and your-library destination, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_return_x_target_creatures_of_creature_type_of_choice_targets_not_all() {
    // Selective Snare pattern: "Return X target creatures of the creature type
    // of your choice to their owner's hand."
    // The parser must produce a targeted ReturnToHand with an X-count spec,
    // not a mass ReturnAllToHand.
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Selective Snare")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::X],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Return X target creatures of the creature type of your choice to their owner's hand.",
        )
        .expect("Selective Snare text should parse");

    let spell_debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();

    // Must have a ChooseCreatureType effect up front.
    assert!(
        spell_debug.contains("choosecreaturetypeeffect"),
        "expected a ChooseCreatureTypeEffect for creature-type selection, got {spell_debug}"
    );

    // Must use targeted return-to-hand, not a mass return-all.
    assert!(
        spell_debug.contains("returntohandeffect"),
        "expected ReturnToHandEffect (targeted), got {spell_debug}"
    );

    // Must be a target spec, not an All spec.
    assert!(
        spell_debug.contains("target("),
        "expected Target(...) spec for targeting X creatures, got {spell_debug}"
    );
    assert!(
        !spell_debug.contains("all("),
        "must NOT use All(...) spec — the card targets X creatures, not all, got {spell_debug}"
    );

    // Must constrain targets to the chosen creature type.
    assert!(
        spell_debug.contains("chosen_creature_type: true"),
        "expected chosen_creature_type constraint on the target filter, got {spell_debug}"
    );

    // The count must be dynamic X (from the mana cost).
    assert!(
        spell_debug.contains("dynamic_x: true"),
        "expected dynamic X target count, got {spell_debug}"
    );

    // Verify the compiled text surface mentions the right oracle phrasing.
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "return x target creatures of the creature type of your choice to their owners' hands"
        ),
        "expected the dynamic chosen-type target program to recover its one-sentence oracle surface, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_each_player_choose_type_return_cards_of_that_type() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Grave Sifter Variant")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(5)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elemental, Subtype::Beast])
        .power_toughness(PowerToughness::fixed(5, 7))
        .parse_text(
            "When this creature enters, each player chooses a creature type and returns any number of cards of that type from their graveyard to their hand.",
        )
        .expect("Grave Sifter-style chosen-type return should parse");

    let debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    let debug_compact = debug.split_whitespace().collect::<String>();
    assert!(
        debug.contains("choosecreaturetypeeffect")
            && debug.contains("chosen_creature_type: true")
            && debug_compact.contains("owner:some(iteratedplayer"),
        "expected return filter to use the per-player chosen creature type, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "when this creature enters, each player chooses a creature type and returns any number of cards of that type from their graveyard to their hand"
        ),
        "expected compact each-player chosen-type return surface, got {rendered}"
    );
}

// ── Chandra's Outburst tests ──────────────────────────────────────────

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn chandras_outburst_compiled_text_uses_card_not_permanent() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Chandra's Outburst")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Chandra's Outburst deals 4 damage to target player or planeswalker.\nSearch your library and/or graveyard for a card named Chandra, Bold Pyromancer, reveal it, and put it into your hand. If you search your library this way, shuffle.",
        )
        .expect("Chandra's Outburst should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    // The compiled text must say "card" (not "permanent") for the search target.
    assert!(
        rendered.contains("card named"),
        "multi-zone search filter should produce 'card' not 'permanent', got {rendered}"
    );
    assert!(
        !rendered.contains("permanent named"),
        "multi-zone search filter must not say 'permanent named', got {rendered}"
    );
}

// ---------------------------------------------------------------------------
// Abundant Harvest tests
// ---------------------------------------------------------------------------

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_abundant_harvest_uses_choice_and_consult_lowering() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Abundant Harvest")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Choose land or nonland. Reveal cards from the top of your library until you reveal a card of the chosen kind. Put that card into your hand and the rest on the bottom of your library in a random order.",
        )
        .expect("Abundant Harvest should parse");

    let spell_debug = format!("{:?}", def.spell_effect);
    assert!(
        spell_debug.contains("ChooseNamedOptionEffect"),
        "expected Abundant Harvest to lower to an explicit land/nonland choice, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("ConsultTopOfLibraryEffect")
            && spell_debug.contains("ConditionalEffect"),
        "expected Abundant Harvest to lower to consult branches keyed off the choice, got {spell_debug}"
    );
    assert!(
        !spell_debug.contains("ChooseObjectsEffect"),
        "Abundant Harvest should not fall back to battlefield object choice, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_abundant_harvest_compiled_text_mentions_land_or_nonland_choice() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Abundant Harvest")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Choose land or nonland. Reveal cards from the top of your library until you reveal a card of the chosen kind. Put that card into your hand and the rest on the bottom of your library in a random order.",
        )
        .expect("Abundant Harvest should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("choose land or nonland"),
        "expected Abundant Harvest compiled text to preserve the land/nonland choice, got {rendered}"
    );
    assert!(
        rendered.contains("reveal cards from the top of your library")
            && rendered.contains("bottom of your library in a random order"),
        "expected Abundant Harvest compiled text to preserve the consult and bottoming clauses, got {rendered}"
    );
    assert!(
        rendered.contains("put that card into your hand")
            && !rendered.contains("return it to its owner's hand")
            && !rendered.contains("remaining tagged cards"),
        "expected Abundant Harvest compiled text to use consult hand wording without internal fallback phrasing, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_treasure_hunt_reveals_until_nonland_and_puts_all_revealed_into_hand() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Treasure Hunt")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Reveal cards from the top of your library until you reveal a nonland card, then put all cards revealed this way into your hand.",
        )
        .expect("Treasure Hunt should parse");

    let spell_debug = format!("{:?}", def.spell_effect);
    assert!(
        spell_debug.contains("ConsultTopOfLibraryEffect")
            && spell_debug.contains("MoveToZoneEffect")
            && spell_debug.contains("revealed"),
        "expected Treasure Hunt to lower to consult plus all-revealed move to hand, got {spell_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered
            .contains("reveal cards from the top of your library until you reveal a nonland card")
            && rendered.contains("put all cards revealed this way into your hand"),
        "expected Treasure Hunt compiled text to preserve all-revealed-to-hand wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_etali_attack_exiles_each_players_top_card_and_casts_any_number() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Etali, Primal Storm")
        .mana_cost(ManaCost::from_pips(vec![vec![
            ManaSymbol::Generic(4),
            ManaSymbol::Red,
            ManaSymbol::Red,
        ]]))
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever this creature attacks, exile the top card of each player's library, then you may cast any number of spells from among those cards without paying their mana costs.",
        )
        .expect("Etali attack trigger should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("ForPlayersEffect")
            && abilities_debug.contains("ExileTopOfLibraryEffect")
            && abilities_debug.contains("ForEachObject")
            && abilities_debug.contains("CastTaggedEffect"),
        "expected Etali to lower to per-player exile and per-exiled-card free cast, got {abilities_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("exile the top card of each player's library")
            && rendered.contains("you may cast any number of spells from among those cards without paying their mana costs"),
        "expected Etali compiled text to preserve each-player exile and any-number cast wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn villainous_wealth_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Villainous Wealth");

    let spell_debug = format!("{:?}", def.spell_effect);
    assert!(
        spell_debug.contains("ExileTopOfLibraryEffect")
            && spell_debug.contains("ForEachObject")
            && spell_debug.contains("CastTaggedEffect")
            && spell_debug.contains("LessThanOrEqualExpr")
            && spell_debug.contains("X"),
        "expected Villainous Wealth to lower to exile-top plus mana-value-capped free casts, got {spell_debug}"
    );

    let rendered = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("exile the top x cards of target opponent's library")
            && rendered.contains("for each nonland card with mana value x or less exiled this way")
            && rendered.contains("you may cast that card without paying its mana cost"),
        "expected Villainous Wealth compiled text to preserve the capped any-number free-cast clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn mindleech_mass_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Mindleech Mass");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("LookAtHandEffect")
            && abilities_debug.contains("IfEffect")
            && abilities_debug.contains("MayCastMatchingSpellWithoutPayingManaCostEffect")
            && abilities_debug.contains("zone_owner: DamagedPlayer"),
        "expected Mindleech Mass to lower to look-at-hand followed by a free cast from that player's hand, got {abilities_debug}"
    );

    let rendered = canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert_eq!(def.card.name, "Mindleech Mass");
    assert!(
        rendered
            .contains("you may cast a spell from among those cards without paying its mana cost"),
        "expected Mindleech Mass compiled text to preserve the hand free-cast clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn geode_golem_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Geode Golem");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Geode Golem should have a combat-damage trigger");
    assert_eq!(
        triggered.trigger.display(),
        "Whenever this creature deals combat damage to a player"
    );

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("MayCastMatchingSpellWithoutPayingManaCostEffect")
            && abilities_debug.contains("zone: Command")
            && abilities_debug.contains("is_commander: true"),
        "expected Geode Golem to lower to command-zone commander free-cast effect, got {abilities_debug}"
    );

    let rendered = canonical_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("trample")
            && rendered_lower.contains("whenever this creature deals combat damage to a player")
            && rendered_lower.contains(
                "you may cast your commander from the command zone without paying its mana cost"
            ),
        "expected Geode Golem compiled text to preserve the command-zone commander free-cast clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn minds_dilation_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Mind's Dilation");

    let abilities_debug = format!("{:?}", def.abilities);
    assert_eq!(def.card.name, "Mind's Dilation");
    assert!(
        abilities_debug.contains("SpellCastTrigger")
            && abilities_debug.contains("exact_spells_this_turn: Some(1)")
            && abilities_debug.contains("ControllerOf(Tagged(TagKey(\"triggering\")))")
            && abilities_debug.contains("ExileTopOfLibraryEffect")
            && abilities_debug.contains("ConditionalEffect")
            && abilities_debug.contains("CastTaggedEffect")
            && abilities_debug.contains("__sentence_helper_exiled_"),
        "expected Mind's Dilation to lower to first-spell trigger, triggering-player library exile, and conditional free cast, got {abilities_debug}"
    );

    let rendered = canonical_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("whenever an opponent casts their first spell each turn")
            && rendered_lower.contains("exile the top card of their library")
            && rendered_lower.contains("if a nonland card was exiled this way")
            && rendered_lower.contains("you may cast that card without paying its mana cost"),
        "expected Mind's Dilation compiled text to preserve the triggering-player nonland free-cast clause, got {rendered}"
    );
    assert!(
        !rendered_lower.contains("tagged object") && !rendered_lower.contains("tagged '"),
        "Mind's Dilation compiled text should not expose internal tagged references, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn villainous_wealth_runtime_casts_only_exiled_nonland_spells_with_mana_value_at_most_x()
{
    let def = parse_oracle_card_definition("Villainous Wealth");
    let spell = def
        .spell_effect
        .as_ref()
        .expect("Villainous Wealth should produce spell effects")
        .clone();

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&def, alice, Zone::Stack);
    game.object_mut(source)
        .expect("Villainous Wealth source should exist")
        .x_value = Some(3);

    game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::from_raw(80_001), "Bottom Filler")
            .card_types(vec![CardType::Artifact])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .build(),
        bob,
        Zone::Library,
    );
    game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::from_raw(80_002), "Cheap Sorcery")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .build(),
        bob,
        Zone::Library,
    );
    game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::from_raw(80_003), "Expensive Sorcery")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
            .build(),
        bob,
        Zone::Library,
    );
    game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::from_raw(80_004), "Forest")
            .card_types(vec![CardType::Land])
            .subtypes(vec![Subtype::Forest])
            .build(),
        bob,
        Zone::Library,
    );

    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let target_assignment = crate::game_state::TargetAssignment {
        spec: ChooseSpec::Player(PlayerFilter::Opponent),
        range: 0..1,
    };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm)
        .with_x(3)
        .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)])
        .with_target_assignments(vec![target_assignment.clone()]);

    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        &spell,
        None,
        &[target_assignment],
    )
    .expect("Villainous Wealth should resolve");

    let stack_names: Vec<_> = game
        .stack
        .iter()
        .filter_map(|entry| game.object(entry.object_id).map(|obj| obj.name.to_string()))
        .collect();
    assert!(
        stack_names.iter().any(|name| name == "Cheap Sorcery"),
        "the exiled spell with mana value at most X should be cast without paying mana, got stack {stack_names:?}"
    );
    assert!(
        !stack_names.iter().any(|name| name == "Expensive Sorcery")
            && !stack_names.iter().any(|name| name == "Forest"),
        "expensive spells and lands should not be cast by Villainous Wealth, got stack {stack_names:?}"
    );

    let exile_names: Vec<_> = game
        .exile
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.to_string()))
        .collect();
    assert!(
        exile_names.iter().any(|name| name == "Expensive Sorcery")
            && exile_names.iter().any(|name| name == "Forest"),
        "nonmatching exiled cards should remain in exile, got {exile_names:?}"
    );

    let library_names: Vec<_> = game
        .player(bob)
        .expect("bob exists")
        .library
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.to_string()))
        .collect();
    assert_eq!(
        library_names,
        vec!["Bottom Filler".to_string()],
        "only the top X cards should be exiled from the target opponent, got library {library_names:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn villainous_wealth_runtime_may_decline_casting_exiled_spells() {
    let def = parse_oracle_card_definition("Villainous Wealth");
    let spell = def
        .spell_effect
        .as_ref()
        .expect("Villainous Wealth should produce spell effects")
        .clone();

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&def, alice, Zone::Stack);

    game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::from_raw(80_011), "Declined Sorcery")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .build(),
        bob,
        Zone::Library,
    );

    let mut dm = crate::decision::AutoPassDecisionMaker;
    let target_assignment = crate::game_state::TargetAssignment {
        spec: ChooseSpec::Player(PlayerFilter::Opponent),
        range: 0..1,
    };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm)
        .with_x(1)
        .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)])
        .with_target_assignments(vec![target_assignment.clone()]);

    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        &spell,
        None,
        &[target_assignment],
    )
    .expect("Villainous Wealth should resolve when its may choice is declined");

    let stack_names: Vec<_> = game
        .stack
        .iter()
        .filter_map(|entry| game.object(entry.object_id).map(|obj| obj.name.to_string()))
        .collect();
    assert!(
        !stack_names.iter().any(|name| name == "Declined Sorcery"),
        "declining Villainous Wealth's may choice should not cast the exiled spell, got stack {stack_names:?}"
    );
    assert!(
        game.exile.iter().any(|&id| game
            .object(id)
            .is_some_and(|obj| obj.name == "Declined Sorcery")),
        "declined spell should remain exiled"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_day_of_black_sun_destroy_those_creatures_reuses_ability_loss_filter() {
    let def = parse_oracle_card_definition("Day of Black Sun");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("each creature with mana value x or less loses all abilities")
            && rendered.contains("destroy all creatures with mana value x or less")
            && !rendered.contains("lesses"),
        "expected Day of Black Sun compiled text to preserve the destroy followup, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_delayed_life_loss_return_source_to_hand_as_single_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Brood of Cockroaches Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature is put into your graveyard from the battlefield, at the beginning of the next end step, you lose 1 life and return this card to your hand.",
        )
        .expect("Brood-style delayed return trigger should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("at the beginning of the next end step, you lose 1 life and return this card to your hand")
            && !rendered.contains("return this creature to its owner's hand"),
        "expected Brood-style delayed return to keep the life-loss and return in one clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn abundant_harvest_land_choice_puts_first_land_into_hand() {
    struct ChooseLandOptionDecisionMaker;

    impl crate::decision::DecisionMaker for ChooseLandOptionDecisionMaker {
        fn decide_options(
            &mut self,
            _game: &crate::game_state::GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            ctx.options
                .iter()
                .find(|option| option.description.eq_ignore_ascii_case("land"))
                .map(|option| vec![option.index])
                .unwrap_or_else(|| vec![0])
        }
    }

    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Abundant Harvest")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Choose land or nonland. Reveal cards from the top of your library until you reveal a card of the chosen kind. Put that card into your hand and the rest on the bottom of your library in a random order.",
        )
        .expect("Abundant Harvest should parse");

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let harvest_id = game.create_object_from_definition(&def, alice, Zone::Stack);

    game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::from_raw(40), "Bottom Card")
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Library,
    );
    game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::from_raw(41), "Forest")
            .card_types(vec![CardType::Land])
            .subtypes(vec![Subtype::Forest])
            .build(),
        alice,
        Zone::Library,
    );
    game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::from_raw(42), "Creature Above")
            .card_types(vec![CardType::Creature])
            .build(),
        alice,
        Zone::Library,
    );

    let mut dm = ChooseLandOptionDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(harvest_id, alice, &mut dm);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        harvest_id,
        def.spell_effect
            .as_ref()
            .expect("Abundant Harvest should have a spell effect"),
        None,
        &[],
    )
    .expect("Abundant Harvest should resolve for land choice");

    let hand_names: Vec<_> = game
        .player(alice)
        .expect("alice exists")
        .hand
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.to_string()))
        .collect();
    assert!(
        hand_names.iter().any(|name| name == "Forest"),
        "land choice should put the first revealed land into hand, got {hand_names:?}"
    );
    assert!(
        !hand_names.iter().any(|name| name == "Creature Above"),
        "land choice should not put nonlands into hand, got {hand_names:?}"
    );

    let library_names: Vec<_> = game
        .player(alice)
        .expect("alice exists")
        .library
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.to_string()))
        .collect();
    assert_eq!(
        library_names.len(),
        2,
        "land choice should leave the unrevealed bottom card plus the revealed nonland in library, got {library_names:?}"
    );
    assert!(
        library_names.iter().any(|name| name == "Bottom Card")
            && library_names.iter().any(|name| name == "Creature Above"),
        "land choice should put only the remainder back on the bottom, got {library_names:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn abundant_harvest_nonland_choice_skips_lands_until_nonland() {
    struct ChooseNonlandOptionDecisionMaker;

    impl crate::decision::DecisionMaker for ChooseNonlandOptionDecisionMaker {
        fn decide_options(
            &mut self,
            _game: &crate::game_state::GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            ctx.options
                .iter()
                .find(|option| option.description.eq_ignore_ascii_case("nonland"))
                .map(|option| vec![option.index])
                .unwrap_or_else(|| vec![0])
        }
    }

    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Abundant Harvest")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Choose land or nonland. Reveal cards from the top of your library until you reveal a card of the chosen kind. Put that card into your hand and the rest on the bottom of your library in a random order.",
        )
        .expect("Abundant Harvest should parse");

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let harvest_id = game.create_object_from_definition(&def, alice, Zone::Stack);

    game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::from_raw(50), "Bottom Card")
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Library,
    );
    game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::from_raw(51), "Merfolk")
            .card_types(vec![CardType::Creature])
            .build(),
        alice,
        Zone::Library,
    );
    game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::from_raw(52), "Island")
            .card_types(vec![CardType::Land])
            .subtypes(vec![Subtype::Island])
            .build(),
        alice,
        Zone::Library,
    );

    let mut dm = ChooseNonlandOptionDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(harvest_id, alice, &mut dm);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        harvest_id,
        def.spell_effect
            .as_ref()
            .expect("Abundant Harvest should have a spell effect"),
        None,
        &[],
    )
    .expect("Abundant Harvest should resolve for nonland choice");

    let hand_names: Vec<_> = game
        .player(alice)
        .expect("alice exists")
        .hand
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.to_string()))
        .collect();
    assert!(
        hand_names.iter().any(|name| name == "Merfolk"),
        "nonland choice should put the first revealed nonland into hand, got {hand_names:?}"
    );
    assert!(
        !hand_names.iter().any(|name| name == "Island"),
        "nonland choice should not put lands into hand, got {hand_names:?}"
    );

    let library_names: Vec<_> = game
        .player(alice)
        .expect("alice exists")
        .library
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.to_string()))
        .collect();
    assert_eq!(
        library_names.len(),
        2,
        "nonland choice should leave the unrevealed bottom card plus the revealed land in library, got {library_names:?}"
    );
    assert!(
        library_names.iter().any(|name| name == "Bottom Card")
            && library_names.iter().any(|name| name == "Island"),
        "nonland choice should put only the nonmatching revealed land back on the bottom, got {library_names:?}"
    );
}

// ---------------------------------------------------------------------------
// Hermit Druid tests
// ---------------------------------------------------------------------------

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_hermit_druid_uses_consult_basic_land_lowering() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Hermit Druid")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Druid])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "{G}, {T}: Reveal cards from the top of your library until you reveal a basic land card. Put that card into your hand and all other cards revealed this way into your graveyard.",
        )
        .expect("Hermit Druid should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("ConsultTopOfLibraryEffect"),
        "expected Hermit Druid to lower to consult effect, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("MoveToZoneEffect"),
        "expected Hermit Druid to lower to a move-to-zone effect, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("zone: Hand"),
        "expected Hermit Druid to move the matched card to hand, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("zone: Graveyard"),
        "expected Hermit Druid to move non-matching cards to graveyard, got {abilities_debug}"
    );
    assert!(
        !abilities_debug.contains("RevealTopEffect"),
        "expected Hermit Druid to avoid the generic reveal-top fallback, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_mirror_mad_uses_consult_named_card_lowering() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mirror-Mad Phantasm")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Spirit])
        .power_toughness(PowerToughness::fixed(5, 1))
        .parse_text(
            "{1}{U}: This creature's owner shuffles it into their library. If that player does, they reveal cards from the top of that library until a card named Mirror-Mad Phantasm is revealed. The player puts that card onto the battlefield and all other cards revealed this way into their graveyard.",
        )
        .expect("Mirror-Mad Phantasm should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("ConsultTopOfLibraryEffect"),
        "expected Mirror-Mad to lower to consult effect, got {abilities_debug}"
    );
    assert!(
        abilities_debug
            .to_ascii_lowercase()
            .contains("name: some(\"mirror mad phantasm\")"),
        "expected named-card stop filter, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("zone: Battlefield")
            && abilities_debug.contains("zone: Graveyard"),
        "expected matched card to battlefield and other revealed cards to graveyard, got {abilities_debug}"
    );
    assert!(
        !abilities_debug.contains("RevealTopEffect"),
        "expected Mirror-Mad to avoid the generic reveal-top fallback, got {abilities_debug}"
    );
    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    // Honest surface after removing the hand-written compaction gate; the
    // compact oracle-style rendering is F-series renderer work.
    assert!(
        rendered.contains("until they reveal a card named mirror mad phantasm")
            && rendered.contains("put it onto the battlefield"),
        "expected Mirror-Mad consult rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn bruenor_battlehammer_anthem_parses_attached_to_affected_not_source() {
    // Verify that "Each creature you control gets +2/+0 for each Equipment
    // attached to it." parses with AttachedToAffected (not AttachedToSource),
    // since "it" refers to the affected creature, not Bruenor.
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bruenor Battlehammer")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Dwarf, Subtype::Warrior])
        .power_toughness(PowerToughness::fixed(5, 3))
        .parse_text("Each creature you control gets +2/+0 for each Equipment attached to it.")
        .expect("Bruenor anthem text should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("AttachedToAffected"),
        "expected AttachedToAffected in anthem, got {abilities_debug}"
    );
    assert!(
        !abilities_debug.contains("AttachedToSource"),
        "should not contain AttachedToSource for multi-creature anthem, got {abilities_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("creature you control gets +2/+0 for each equipment attached to it")
            || rendered
                .contains("creatures you control get +2/+0 for each equipment attached to it"),
        "expected Bruenor anthem to render oracle-like, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn bruenor_battlehammer_equip_cost_alternative_parses_as_static() {
    // Verify that "You may pay {0} rather than pay the equip cost of the first
    // equip ability you activate each turn." parses as a static ability, not a
    // spell effect that drops the "rather than pay ..." clause.
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bruenor Battlehammer")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Dwarf, Subtype::Warrior])
        .power_toughness(PowerToughness::fixed(5, 3))
        .parse_text(
            "You may pay {0} rather than pay the equip cost of the first equip ability you activate each turn.",
        )
        .expect("Bruenor equip cost alternative text should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("FirstEquipCostAlternative"),
        "expected FirstEquipCostAlternative static ability, got {abilities_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("rather than pay the equip cost"),
        "expected compiled text to preserve 'rather than pay the equip cost', got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn first_equip_cost_alternative_parses_for_during_each_of_your_turns_variant() {
    // Forge Anew variant: "during each of your turns" instead of "each turn"
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Forge Anew")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "You may pay {0} rather than pay the equip cost of the first equip ability you activate during each of your turns.",
        )
        .expect("Forge Anew equip cost alternative text should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("FirstEquipCostAlternative"),
        "expected FirstEquipCostAlternative static ability for Forge Anew variant, got {abilities_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("during each of your turns"),
        "expected compiled text to preserve 'during each of your turns', got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn chandras_outburst_compiled_text_preserves_shuffle() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Chandra's Outburst")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Chandra's Outburst deals 4 damage to target player or planeswalker.\nSearch your library and/or graveyard for a card named Chandra, Bold Pyromancer, reveal it, and put it into your hand. If you search your library this way, shuffle.",
        )
        .expect("Chandra's Outburst should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    // The structural renderer still preserves the search/shuffle operation after
    // debug-only text reconciliation was removed.
    assert!(
        rendered.contains("if you search your library this way, shuffle"),
        "multi-zone search should preserve the shuffle clause, got {rendered}"
    );
    // Must not contain "shuffle target player's library" or similar unconditional form.
    assert!(
        !rendered.contains("shuffle target"),
        "shuffle should not reference 'target player', got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn auditore_ambush_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Auditore Ambush");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let lower = rendered.to_ascii_lowercase();
    let modal = def
        .spell_effect
        .as_ref()
        .and_then(|effects| {
            effects
                .iter()
                .find_map(|effect| effect.downcast_ref::<ChooseModeEffect>())
        })
        .expect("Auditore Ambush should lower to a modal spell effect");

    assert_eq!(
        modal.modes.len(),
        2,
        "Auditore Ambush should have two modes"
    );
    assert!(
        matches!(modal.min_choose_count, crate::effect::Value::Fixed(1))
            && matches!(modal.choose_count, crate::effect::Value::Fixed(2)),
        "choose one or both should allow choosing one or two modes, got {modal:?}"
    );
    assert!(
        lower.contains("target player searches their library and/or graveyard")
            && lower.contains("if they search their library this way, they shuffle"),
        "expected target-player multi-zone search and conditional shuffle text, got {rendered}"
    );

    let search_mode = &modal.modes[1];
    fn contains_search(effect: &crate::effect::Effect) -> bool {
        if effect
            .downcast_ref::<ChooseObjectsEffect>()
            .is_some_and(|choose| choose.is_search)
        {
            return true;
        }
        let mut found = false;
        effect.visit_child_effects(&mut |child| {
            found |= contains_search(child);
        });
        found
    }

    fn tracked_search_id(effect: &crate::effect::Effect) -> Option<crate::effect::EffectId> {
        if let Some(with_id) = effect.downcast_ref::<WithIdEffect>()
            && contains_search(&with_id.effect)
        {
            return Some(with_id.id);
        }
        let mut found = None;
        effect.visit_child_effects(&mut |child| {
            if found.is_none() {
                found = tracked_search_id(child);
            }
        });
        found
    }

    fn nested_if_effect(effect: &crate::effect::Effect) -> Option<&IfEffect> {
        if let Some(if_effect) = effect.downcast_ref::<IfEffect>() {
            return Some(if_effect);
        }
        if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
            return sequence.effects.iter().find_map(nested_if_effect);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return nested_if_effect(&tagged.effect);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
            return nested_if_effect(&tagged.effect);
        }
        if let Some(with_id) = effect.downcast_ref::<WithIdEffect>() {
            return nested_if_effect(&with_id.effect);
        }
        None
    }

    let search_id = search_mode
        .effects
        .iter()
        .find_map(tracked_search_id)
        .expect("library search should be effect-id tracked");
    let shuffle_condition = search_mode
        .effects
        .iter()
        .find_map(nested_if_effect)
        .expect("conditional library shuffle should lower to IfEffect");
    assert_eq!(
        shuffle_condition.condition, search_id,
        "the shuffle condition must be keyed to the library search, not to moving a found card"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn disciple_of_perdition_modal_effect(def: &CardDefinition) -> &crate::effect::Effect {
    let ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Disciple of Perdition should have a dies triggered ability");

    ability
        .effects
        .segments
        .first()
        .and_then(|segment| segment.default_effects.first())
        .expect("Disciple of Perdition trigger should contain a modal effect")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn disciple_of_perdition_modal_choice(def: &CardDefinition) -> &ChooseModeEffect {
    let conditional = disciple_of_perdition_modal_effect(def)
        .downcast_ref::<crate::effects::ConditionalEffect>()
        .expect("Disciple choose-both clause should lower to a conditional modal effect");

    conditional.if_true[0]
        .downcast_ref::<ChooseModeEffect>()
        .expect("Disciple exact-life branch should contain modal choices")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn disciple_of_perdition_exile_effect(
    effect: &crate::effect::Effect,
) -> Option<&crate::effects::ExileEffect> {
    effect
        .downcast_ref::<crate::effects::ExileEffect>()
        .or_else(|| {
            effect
                .downcast_ref::<TaggedEffect>()
                .and_then(|tagged| tagged.effect.downcast_ref::<crate::effects::ExileEffect>())
        })
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn disciple_of_perdition_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Disciple of Perdition");

    let def = parse_oracle_card_definition("Disciple of Perdition");
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    let lower = rendered.to_ascii_lowercase();
    let modal_effect = disciple_of_perdition_modal_effect(&def);
    let conditional = modal_effect
        .downcast_ref::<crate::effects::ConditionalEffect>()
        .expect("Disciple choose-both clause should lower to a conditional modal effect");

    assert!(
        lower.contains("when this creature dies, choose one. if you have exactly 13 life, you may choose both instead"),
        "expected compiled text to preserve the exact-life choose-both-instead clause, got {rendered}"
    );
    assert_eq!(
        &conditional.condition,
        &crate::effect::Condition::ValueComparison {
            left: crate::effect::Value::LifeTotal(PlayerFilter::You),
            operator: crate::effect::ValueComparisonOperator::Equal,
            right: crate::effect::Value::Fixed(13),
        },
        "expected exact 13 life condition, got {conditional:?}"
    );

    let modal = disciple_of_perdition_modal_choice(&def);
    let exile = modal.modes[1]
        .effects
        .iter()
        .find_map(disciple_of_perdition_exile_effect)
        .expect("Disciple graveyard mode should contain a graveyard exile effect");
    let ChooseSpec::All(filter) = &exile.spec else {
        panic!(
            "Disciple should exile all cards in the targeted opponent's graveyard, got {exile:?}"
        );
    };
    assert_eq!(filter.zone, Some(Zone::Graveyard));
    assert_eq!(
        filter.owner,
        Some(PlayerFilter::Target(Box::new(PlayerFilter::Opponent))),
        "Disciple should model target opponent's graveyard as all graveyard cards owned by the targeted opponent, got {filter:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn disciple_of_perdition_runtime_allows_both_modes_at_exactly_13_life() {
    let def = parse_oracle_card_definition("Disciple of Perdition");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.lose_life(alice, 7);

    let modal_spec = disciple_of_perdition_modal_effect(&def)
        .0
        .get_modal_spec_with_context(&game, alice, source)
        .expect("Disciple trigger should expose modal choices");

    assert_eq!(modal_spec.min_modes, crate::effect::Value::Fixed(1));
    assert_eq!(modal_spec.max_modes, crate::effect::Value::Fixed(2));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn disciple_of_perdition_runtime_stays_choose_one_when_not_at_exactly_13_life() {
    let def = parse_oracle_card_definition("Disciple of Perdition");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    let modal_spec = disciple_of_perdition_modal_effect(&def)
        .0
        .get_modal_spec_with_context(&game, alice, source)
        .expect("Disciple trigger should expose modal choices");

    assert_eq!(modal_spec.min_modes, crate::effect::Value::Fixed(1));
    assert_eq!(modal_spec.max_modes, crate::effect::Value::Fixed(1));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn disciple_of_perdition_runtime_modes_apply_draw_life_loss_and_graveyard_exile() {
    let def = parse_oracle_card_definition("Disciple of Perdition");
    let modal = disciple_of_perdition_modal_choice(&def);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let _library_card = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_601), "Disciple Draw Card").build(),
        alice,
        Zone::Library,
    );
    let alice_graveyard_card = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_602), "Disciple Alice Graveyard Card")
            .build(),
        alice,
        Zone::Graveyard,
    );
    let _bob_graveyard_card_one = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_603), "Disciple Bob Graveyard Card One")
            .build(),
        bob,
        Zone::Graveyard,
    );
    let _bob_graveyard_card_two = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_604), "Disciple Bob Graveyard Card Two")
            .build(),
        bob,
        Zone::Graveyard,
    );

    let mut draw_ctx = crate::effects::ExecutionContext::new_default(source, alice);
    for effect in &modal.modes[0].effects {
        crate::effects::execute_effect(&mut game, effect, &mut draw_ctx)
            .expect("Disciple draw/life-loss mode should resolve");
    }

    assert_eq!(game.player(alice).expect("alice").life, 19);
    assert_eq!(game.player(alice).expect("alice").library.len(), 0);
    assert!(
        game.player(alice).expect("alice").hand.len() == 1,
        "the first mode should draw the top card"
    );

    let mut exile_ctx = crate::effects::ExecutionContext::new_default(source, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)]);
    for effect in &modal.modes[1].effects {
        crate::effects::execute_effect(&mut game, effect, &mut exile_ctx)
            .expect("Disciple graveyard-exile mode should resolve");
    }

    assert_eq!(game.player(bob).expect("bob").life, 19);
    assert_eq!(
        game.player(bob).expect("bob").graveyard.len(),
        0,
        "the second mode should empty the targeted opponent's whole graveyard"
    );
    assert_eq!(
        game.player(alice).expect("alice").graveyard.len(),
        1,
        "the second mode should not exile a non-targeted player's graveyard"
    );
    assert_eq!(
        game.objects_in_zone(Zone::Exile).len(),
        2,
        "the second mode should exile each card in the targeted opponent's graveyard"
    );
    assert_eq!(
        game.object(alice_graveyard_card)
            .expect("alice graveyard card")
            .zone,
        Zone::Graveyard
    );
    let exile_names: Vec<_> = game
        .objects_in_zone(Zone::Exile)
        .into_iter()
        .filter_map(|id| game.object(id).map(|object| object.name.to_string()))
        .collect();
    assert!(
        exile_names.contains(&"Disciple Bob Graveyard Card One".to_string())
            && exile_names.contains(&"Disciple Bob Graveyard Card Two".to_string()),
        "the second mode should exile both cards from Bob's graveyard, got {exile_names:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn auditore_ambush_runtime_returns_target_creature() {
    use crate::effects::ResolvedTarget;

    let def = parse_oracle_card_definition("Auditore Ambush");
    let modal = def
        .spell_effect
        .as_ref()
        .and_then(|effects| {
            effects
                .iter()
                .find_map(|effect| effect.downcast_ref::<ChooseModeEffect>())
        })
        .expect("Auditore Ambush should lower to a modal spell effect");
    let return_mode: crate::resolution::ResolutionProgram = modal.modes[0].effects.clone().into();

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let spell_source = game.create_object_from_definition(&def, alice, Zone::Stack);
    let target_creature = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(91_501), "Ambushed Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        bob,
        Zone::Battlefield,
    );

    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(spell_source, alice, &mut dm)
        .with_targets(vec![ResolvedTarget::Object(target_creature)])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: crate::target::ChooseSpec::target_creature(),
            range: 0..1,
        }]);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        spell_source,
        &return_mode,
        None,
        &[],
    )
    .expect("Auditore Ambush return mode should resolve");

    let bob_hand_names: Vec<_> = game
        .player(bob)
        .expect("bob exists")
        .hand
        .iter()
        .filter_map(|id| game.object(*id).map(|object| object.name.to_string()))
        .collect();
    assert!(
        bob_hand_names
            .iter()
            .any(|name| name == "Ambushed Creature"),
        "return mode should put the target creature into its owner's hand, got {bob_hand_names:?}"
    );
    assert!(
        !game.battlefield.contains(&target_creature),
        "return mode should remove the target creature from the battlefield"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn auditore_ambush_runtime_target_player_searches_for_ezio() {
    use crate::effects::ResolvedTarget;

    let def = parse_oracle_card_definition("Auditore Ambush");
    let modal = def
        .spell_effect
        .as_ref()
        .and_then(|effects| {
            effects
                .iter()
                .find_map(|effect| effect.downcast_ref::<ChooseModeEffect>())
        })
        .expect("Auditore Ambush should lower to a modal spell effect");
    let search_mode: crate::resolution::ResolutionProgram = modal.modes[1].effects.clone().into();

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let spell_source = game.create_object_from_definition(&def, alice, Zone::Stack);
    game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::from_raw(91_502), "Ezio, Blade of Vengeance")
            .card_types(vec![CardType::Creature])
            .build(),
        bob,
        Zone::Library,
    );

    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(spell_source, alice, &mut dm)
        .with_targets(vec![ResolvedTarget::Player(bob)])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: crate::target::ChooseSpec::target_player(),
            range: 0..1,
        }]);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        spell_source,
        &search_mode,
        None,
        &[],
    )
    .expect("Auditore Ambush search mode should resolve");

    let bob_hand_names: Vec<_> = game
        .player(bob)
        .expect("bob exists")
        .hand
        .iter()
        .filter_map(|id| game.object(*id).map(|object| object.name.to_string()))
        .collect();
    assert!(
        bob_hand_names
            .iter()
            .any(|name| name == "Ezio, Blade of Vengeance"),
        "search mode should put the named Ezio card into the target player's hand, got {bob_hand_names:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn auditore_ambush_runtime_search_mode_allows_no_matching_card() {
    use crate::effects::ResolvedTarget;

    let def = parse_oracle_card_definition("Auditore Ambush");
    let modal = def
        .spell_effect
        .as_ref()
        .and_then(|effects| {
            effects
                .iter()
                .find_map(|effect| effect.downcast_ref::<ChooseModeEffect>())
        })
        .expect("Auditore Ambush should lower to a modal spell effect");
    let search_mode: crate::resolution::ResolutionProgram = modal.modes[1].effects.clone().into();

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let spell_source = game.create_object_from_definition(&def, alice, Zone::Stack);
    game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::from_raw(91_503), "Not Ezio")
            .card_types(vec![CardType::Creature])
            .build(),
        bob,
        Zone::Library,
    );

    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(spell_source, alice, &mut dm)
        .with_targets(vec![ResolvedTarget::Player(bob)])
        .with_target_assignments(vec![crate::game_state::TargetAssignment {
            spec: crate::target::ChooseSpec::target_player(),
            range: 0..1,
        }]);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        spell_source,
        &search_mode,
        None,
        &[],
    )
    .expect("Auditore Ambush search mode should resolve without a matching card");

    let bob_hand_names: Vec<_> = game
        .player(bob)
        .expect("bob exists")
        .hand
        .iter()
        .filter_map(|id| game.object(*id).map(|object| object.name.to_string()))
        .collect();
    assert!(
        bob_hand_names.is_empty(),
        "search mode should not move a nonmatching card into hand, got {bob_hand_names:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_hermit_druid_compiled_text_matches_oracle() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Hermit Druid")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Druid])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "{G}, {T}: Reveal cards from the top of your library until you reveal a basic land card. Put that card into your hand and all other cards revealed this way into your graveyard.",
        )
        .expect("Hermit Druid should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let lower = rendered.to_ascii_lowercase();
    assert!(
        lower.contains(
            "reveal cards from the top of your library until you reveal a basic land card"
        ),
        "expected Hermit Druid compiled text to contain reveal-until clause, got {rendered}"
    );
    assert!(
        lower.contains("hand") && lower.contains("graveyard"),
        "expected Hermit Druid compiled text to mention hand and graveyard destinations, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn chandras_outburst_compiled_text_no_internal_tags() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Chandra's Outburst")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Chandra's Outburst deals 4 damage to target player or planeswalker.\nSearch your library and/or graveyard for a card named Chandra, Bold Pyromancer, reveal it, and put it into your hand. If you search your library this way, shuffle.",
        )
        .expect("Chandra's Outburst should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    // Internal tag names like "searched_multi_zone" must not leak into the compiled text.
    assert!(
        !rendered.contains("searched_multi_zone"),
        "internal tag name should not appear in compiled text, got {rendered}"
    );
    assert!(
        !rendered.contains("for each tagged"),
        "generic 'for each tagged' rendering should not appear, got {rendered}"
    );
    assert!(
        !rendered.contains("tags it"),
        "internal tagging description should not appear, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_hermit_druid_reveals_until_basic_land_and_graveyards_others() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Hermit Druid")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Druid])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "{G}, {T}: Reveal cards from the top of your library until you reveal a basic land card. Put that card into your hand and all other cards revealed this way into your graveyard.",
        )
        .expect("Hermit Druid should parse");

    let ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Hermit Druid should have an activated ability");

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let druid_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    // Library from bottom to top: Unseen Bottom -> Basic Forest -> Nonland Filler
    // Top of library is last pushed, so push order: bottom first.
    game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::from_raw(10), "Unseen Bottom")
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Library,
    );
    game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::from_raw(11), "Basic Forest")
            .supertypes(vec![Supertype::Basic])
            .card_types(vec![CardType::Land])
            .subtypes(vec![Subtype::Forest])
            .build(),
        alice,
        Zone::Library,
    );
    game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::from_raw(12), "Nonland Filler")
            .card_types(vec![CardType::Creature])
            .build(),
        alice,
        Zone::Library,
    );

    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(druid_id, alice, &mut dm);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        druid_id,
        &ability.effects,
        None,
        &[],
    )
    .expect("Hermit Druid effect should resolve");

    // The basic land should be in hand.
    let hand_names: Vec<_> = game
        .player(alice)
        .expect("alice exists")
        .hand
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.to_string()))
        .collect();
    assert!(
        hand_names.iter().any(|name| name == "Basic Forest"),
        "Hermit Druid should put the first basic land into hand, got {hand_names:?}"
    );
    assert!(
        !hand_names.iter().any(|name| name == "Nonland Filler"),
        "Hermit Druid should not put non-land cards into hand, got {hand_names:?}"
    );

    // The non-land filler should be in graveyard.
    let graveyard_names: Vec<_> = game
        .player(alice)
        .expect("alice exists")
        .graveyard
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.to_string()))
        .collect();
    assert!(
        graveyard_names.iter().any(|name| name == "Nonland Filler"),
        "Hermit Druid should put revealed non-matching cards into graveyard, got {graveyard_names:?}"
    );
    assert!(
        !graveyard_names.iter().any(|name| name == "Basic Forest"),
        "Hermit Druid should keep the matching basic land out of graveyard, got {graveyard_names:?}"
    );

    // The unseen bottom card should remain in library (not revealed).
    let library_names: Vec<_> = game
        .player(alice)
        .expect("alice exists")
        .library
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.to_string()))
        .collect();
    assert_eq!(
        library_names,
        vec!["Unseen Bottom".to_string()],
        "Hermit Druid should stop revealing at the first basic land, leaving unseen cards alone"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn chandras_outburst_compiled_text_has_4_damage() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Chandra's Outburst")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Chandra's Outburst deals 4 damage to target player or planeswalker.\nSearch your library and/or graveyard for a card named Chandra, Bold Pyromancer, reveal it, and put it into your hand. If you search your library this way, shuffle.",
        )
        .expect("Chandra's Outburst should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    // The damage clause must reference 4 damage to a target player or planeswalker.
    assert!(
        rendered.contains("4 damage") && rendered.contains("target player or planeswalker"),
        "expected 4-damage-to-target clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_hermit_druid_basic_land_on_top_goes_straight_to_hand() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Hermit Druid")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Druid])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "{G}, {T}: Reveal cards from the top of your library until you reveal a basic land card. Put that card into your hand and all other cards revealed this way into your graveyard.",
        )
        .expect("Hermit Druid should parse");

    let ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Hermit Druid should have an activated ability");

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let druid_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    // Library: just a basic land on top.
    game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::from_raw(20), "Another Card")
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Library,
    );
    game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::from_raw(21), "Island")
            .supertypes(vec![Supertype::Basic])
            .card_types(vec![CardType::Land])
            .subtypes(vec![Subtype::Island])
            .build(),
        alice,
        Zone::Library,
    );

    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(druid_id, alice, &mut dm);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        druid_id,
        &ability.effects,
        None,
        &[],
    )
    .expect("Hermit Druid effect should resolve");

    let hand_names: Vec<_> = game
        .player(alice)
        .expect("alice exists")
        .hand
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.to_string()))
        .collect();
    assert!(
        hand_names.iter().any(|name| name == "Island"),
        "When basic land is on top, Hermit Druid should put it directly into hand, got {hand_names:?}"
    );

    let graveyard_names: Vec<_> = game
        .player(alice)
        .expect("alice exists")
        .graveyard
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.to_string()))
        .collect();
    assert!(
        graveyard_names.is_empty(),
        "When basic land is on top, nothing else should go to graveyard, got {graveyard_names:?}"
    );

    let library_names: Vec<_> = game
        .player(alice)
        .expect("alice exists")
        .library
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.to_string()))
        .collect();
    assert_eq!(
        library_names,
        vec!["Another Card".to_string()],
        "Cards below the top basic land should remain in library"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn chandras_outburst_compiled_text_reveal_and_hand() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Chandra's Outburst")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Chandra's Outburst deals 4 damage to target player or planeswalker.\nSearch your library and/or graveyard for a card named Chandra, Bold Pyromancer, reveal it, and put it into your hand. If you search your library this way, shuffle.",
        )
        .expect("Chandra's Outburst should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    // The compiled text must mention revealing and putting into hand.
    assert!(
        rendered.contains("reveal it"),
        "expected 'reveal it' clause, got {rendered}"
    );
    assert!(
        rendered.contains("your hand"),
        "expected 'your hand' destination clause, got {rendered}"
    );
}

/// Generic multi-zone search: any "search your library and/or graveyard for a <type>
/// card named ..." must produce "card named" (not "permanent named") in the compiled
/// text, regardless of the specific card name.
#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn multi_zone_search_named_card_uses_card_noun() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Multi Zone Test")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Search your library and/or graveyard for a card named Example Card, reveal it, and put it into your hand. If you search your library this way, shuffle.",
        )
        .expect("multi-zone named search should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("card named"),
        "multi-zone named-card search should say 'card named', got {rendered}"
    );
    assert!(
        !rendered.contains("permanent named"),
        "multi-zone named-card search must not say 'permanent named', got {rendered}"
    );
    assert!(
        rendered.contains("if you search your library this way, shuffle"),
        "multi-zone search should produce conditional shuffle, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_hermit_druid_no_basic_land_mills_entire_library() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Hermit Druid")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Druid])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "{G}, {T}: Reveal cards from the top of your library until you reveal a basic land card. Put that card into your hand and all other cards revealed this way into your graveyard.",
        )
        .expect("Hermit Druid should parse");

    let ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Hermit Druid should have an activated ability");

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let druid_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    // Library has no basic lands at all - only nonland cards.
    for idx in 0..4 {
        game.create_object_from_card(
            &crate::card::CardBuilder::new(CardId::from_raw(30 + idx), format!("Spell {idx}"))
                .card_types(vec![CardType::Instant])
                .build(),
            alice,
            Zone::Library,
        );
    }

    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(druid_id, alice, &mut dm);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        druid_id,
        &ability.effects,
        None,
        &[],
    )
    .expect("Hermit Druid effect should resolve even with no basic lands");
    let hand_names: Vec<_> = game
        .player(alice)
        .expect("alice exists")
        .hand
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.to_string()))
        .collect();
    assert!(
        hand_names.is_empty(),
        "With no basic lands, nothing should end up in hand, got {hand_names:?}"
    );

    let graveyard_names: Vec<_> = game
        .player(alice)
        .expect("alice exists")
        .graveyard
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.to_string()))
        .collect();
    assert_eq!(
        graveyard_names.len(),
        4,
        "With no basic lands, all library cards should go to graveyard, got {graveyard_names:?}"
    );

    let library = &game.player(alice).expect("alice exists").library;
    assert!(
        library.is_empty(),
        "With no basic lands, the entire library should be empty after Hermit Druid resolves"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn bruenor_source_only_anthem_keeps_attached_to_source() {
    // Verify that a source-only anthem like "This creature gets +2/+0 for
    // each Equipment attached to it" still uses AttachedToSource.
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Self-Equip Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text("This creature gets +2/+0 for each Equipment attached to it.")
        .expect("source-only anthem text should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("AttachedToSource"),
        "source-only anthem should keep AttachedToSource, got {abilities_debug}"
    );
    assert!(
        !abilities_debug.contains("AttachedToAffected"),
        "source-only anthem should not use AttachedToAffected, got {abilities_debug}"
    );
}

// ====================================================================
// Union of the Third Path tests
// ====================================================================

/// Union of the Third Path — {2}{W} Instant
/// Oracle: "Draw a card, then you gain life equal to the number of cards in your hand."
///
/// Structure test: verify that the card compiles with the correct effects
/// (DrawCardsEffect followed by GainLifeEffect) and that the compiled text
/// uses the oracle-style "Draw a card, then you gain life equal to the number of
/// cards in your hand" phrasing.
#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn union_of_the_third_path_compiles_with_draw_then_gain_life() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Union of the Third Path")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Instant])
        .oracle_text("Draw a card, then you gain life equal to the number of cards in your hand.")
        .parse_text("Draw a card, then you gain life equal to the number of cards in your hand.")
        .expect("Union of the Third Path text should parse");

    // Verify the spell effect has both DrawCards and GainLife effects.
    let spell = def.spell_effect.as_ref().expect("should have spell effect");
    assert!(!spell.is_empty(), "spell should have at least one segment");
    let effects = &spell.segments[0].default_effects;
    assert_eq!(
        effects.len(),
        2,
        "should have exactly 2 effects (draw + gain life)"
    );

    let draw = effects[0]
        .downcast_ref::<crate::effects::DrawCardsEffect>()
        .expect("first effect should be DrawCardsEffect");
    assert_eq!(
        draw.count,
        crate::effect::Value::Fixed(1),
        "draw count should be 1"
    );
    assert_eq!(draw.player, PlayerFilter::You, "draw player should be You");

    let gain = effects[1]
        .downcast_ref::<GainLifeEffect>()
        .expect("second effect should be GainLifeEffect");
    assert!(
        matches!(gain.amount.unhinted(), crate::effect::Value::Count(_)),
        "gain amount should be Count (cards in hand), got {:?}",
        gain.amount
    );
    assert!(
        matches!(gain.player, ChooseSpec::Player(PlayerFilter::You)),
        "gain player should be You"
    );
}

/// Verify the canonical compiled text matches the oracle phrasing:
/// "Draw a card, then you gain life equal to the number of cards in your hand."
#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn union_of_the_third_path_canonical_text_uses_then_and_equal_to() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Union of the Third Path")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Instant])
        .oracle_text("Draw a card, then you gain life equal to the number of cards in your hand.")
        .parse_text("Draw a card, then you gain life equal to the number of cards in your hand.")
        .expect("Union of the Third Path text should parse");

    let canonical = crate::compiled_text::canonical_compiled_lines(&def).join("\n");
    let canonical_lower = canonical.to_ascii_lowercase();

    // The canonical text must use ", then" (not ". " or " and ") between draw and gain.
    assert!(
        canonical_lower.contains("draw a card, then you gain"),
        "canonical text should use ', then' connector, got: {canonical}"
    );

    // The canonical text must use "life equal to the number of" (not "1 life for each").
    assert!(
        canonical_lower.contains("life equal to the number of"),
        "canonical text should use 'life equal to the number of' phrasing, got: {canonical}"
    );

    // It must mention "cards in your hand".
    assert!(
        canonical_lower.contains("cards in your hand"),
        "canonical text should mention 'cards in your hand', got: {canonical}"
    );
}

/// Scenario test: resolve Union of the Third Path's spell effect directly
/// and verify the life gained equals the hand size AFTER drawing.
///
/// Setup: Alice has 3 cards in hand, 3 cards in library.
/// After drawing 1 (hand -> 4), she should gain 4 life.
#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn union_of_the_third_path_gains_life_equal_to_hand_size_after_draw() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Union of the Third Path")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text("Draw a card, then you gain life equal to the number of cards in your hand.")
        .expect("Union of the Third Path should parse");

    let spell_effect = def.spell_effect.as_ref().expect("should have spell effect");

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let union_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    // Give Alice 3 cards in hand
    for i in 0..3 {
        game.create_object_from_card(
            &crate::card::CardBuilder::new(CardId::from_raw(10 + i), &format!("Hand Card {i}"))
                .card_types(vec![CardType::Creature])
                .build(),
            alice,
            Zone::Hand,
        );
    }
    // Give Alice 3 cards in library to draw from
    for i in 0..3 {
        game.create_object_from_card(
            &crate::card::CardBuilder::new(CardId::from_raw(20 + i), &format!("Library Card {i}"))
                .card_types(vec![CardType::Creature])
                .build(),
            alice,
            Zone::Library,
        );
    }

    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(union_id, alice, &mut dm);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        union_id,
        spell_effect,
        None,
        &[],
    )
    .expect("Union effect should resolve");

    // After drawing 1 card, hand should be 4.
    let hand_size = game.player(alice).unwrap().hand.len();
    assert_eq!(
        hand_size, 4,
        "Alice should have 4 cards in hand (3 + 1 drawn)"
    );

    // Life should be 20 + 4 (hand size after draw) = 24.
    let life = game.life_total(alice);
    assert_eq!(
        life, 24,
        "Alice should have 24 life (20 starting + 4 from hand size). Got {life}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn stand_or_fall_probe_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(1), "Stand or Fall Probe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "At the beginning of combat on your turn, for each defending player, separate all creatures that player controls into two piles and that player chooses one. Only creatures in the chosen piles can block this turn.",
        )
        .expect("Stand or Fall should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn fight_or_flight_probe_definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(5), "Fight or Flight Probe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "At the beginning of combat on each opponent's turn, separate all creatures that player controls into two piles. Only creatures in the pile of their choice can attack this turn.",
        )
        .expect("Fight or Flight should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn fight_or_flight_compiled_text_uses_pile_choice_language() {
    let def = fight_or_flight_probe_definition();

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("BeginningOfCombatTrigger")
            && abilities_debug.contains("ChooseObjectsEffect")
            && abilities_debug.contains("CantEffect"),
        "expected Fight or Flight to keep a beginning-of-combat pile-splitting restriction, got {abilities_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("at the beginning of combat on each opponent's turn")
            && rendered.contains("separate all creatures that player controls into two piles")
            && rendered.contains("only creatures in the pile of their choice can attack this turn"),
        "expected Fight or Flight text to render as a pile-choice attack restriction, got {rendered}"
    );
    assert!(
        !rendered.contains("choose any number a creature")
            && !rendered.contains("tags it as 'divvy_chosen'"),
        "expected the compiled text to avoid the generic choose/tag fallback, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn fight_or_flight_keeps_only_the_chosen_pile_legal_to_attack() {
    use crate::decision::{DecisionMaker, LegalAction};
    use crate::rules::combat::can_attack;

    struct ChooseNamedPileDecisionMaker {
        chosen_name: &'static str,
    }

    impl DecisionMaker for ChooseNamedPileDecisionMaker {
        fn decide_priority(
            &mut self,
            _game: &crate::game_state::GameState,
            _ctx: &crate::decisions::context::PriorityContext,
        ) -> LegalAction {
            LegalAction::PassPriority
        }

        fn decide_objects(
            &mut self,
            game: &crate::game_state::GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            ctx.candidates
                .iter()
                .find(|candidate| {
                    candidate.legal
                        && game
                            .current_name(candidate.id)
                            .is_some_and(|name| name == self.chosen_name)
                })
                .map(|candidate| vec![candidate.id])
                .expect("expected to find the named chosen pile")
        }
    }

    let def = fight_or_flight_probe_definition();
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let fight_or_flight_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let chosen_id = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(6), "Chosen Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        bob,
        Zone::Battlefield,
    );
    let other_id = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(7), "Other Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        bob,
        Zone::Battlefield,
    );
    game.remove_summoning_sickness(chosen_id);
    game.remove_summoning_sickness(other_id);

    let mut dm = ChooseNamedPileDecisionMaker {
        chosen_name: "Chosen Bear",
    };

    let triggered = match &def.abilities[0].kind {
        AbilityKind::Triggered(triggered) => triggered,
        other => {
            panic!("expected Fight or Flight to compile to a triggered ability, got {other:?}")
        }
    };
    let mut ctx = crate::effects::ExecutionContext::new(fight_or_flight_id, alice, &mut dm)
        .with_defending_player(bob);
    ctx.iteration.iterated_player = Some(bob);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        fight_or_flight_id,
        &triggered.effects,
        None,
        &[],
    )
    .expect("Fight or Flight's combat trigger should resolve");

    assert!(
        can_attack(
            game.object(chosen_id).expect("chosen creature exists"),
            &game
        ),
        "creatures in the chosen pile should remain able to attack"
    );
    assert!(
        !can_attack(game.object(other_id).expect("other creature exists"), &game),
        "creatures outside the chosen pile should be unable to attack"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn stand_or_fall_compiled_text_uses_pile_choice_language() {
    let def = stand_or_fall_probe_definition();

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("ForPlayersEffect")
            && abilities_debug.contains("ChooseObjectsEffect")
            && abilities_debug.contains("CantEffect"),
        "expected pile-splitting structure to remain intact, got {abilities_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("for each defending player")
            && rendered.contains("separate all creatures that player controls into two piles")
            && rendered.contains("that player chooses one")
            && rendered.contains("only creatures in the chosen piles can block this turn"),
        "expected Stand or Fall text to render as a pile-choice block restriction, got {rendered}"
    );
    assert!(
        !rendered.contains("choose any number a creature")
            && !rendered.contains("tags it as 'divvy_chosen'"),
        "expected the compiled text to avoid the generic choose/tag fallback, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn stand_or_fall_keeps_only_the_chosen_pile_legal_to_block() {
    use crate::combat_state::AttackTarget;
    use crate::decision::{DecisionMaker, LegalAction};

    struct ChooseNamedPileDecisionMaker {
        chosen_name: &'static str,
        attacker_name: &'static str,
    }

    impl DecisionMaker for ChooseNamedPileDecisionMaker {
        fn decide_priority(
            &mut self,
            _game: &crate::game_state::GameState,
            _ctx: &crate::decisions::context::PriorityContext,
        ) -> LegalAction {
            LegalAction::PassPriority
        }

        fn decide_objects(
            &mut self,
            game: &crate::game_state::GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            ctx.candidates
                .iter()
                .find(|candidate| {
                    candidate.legal
                        && game
                            .current_name(candidate.id)
                            .is_some_and(|name| name == self.chosen_name)
                })
                .map(|candidate| vec![candidate.id])
                .expect("expected to find the named chosen pile")
        }

        fn decide_attackers(
            &mut self,
            game: &crate::game_state::GameState,
            ctx: &crate::decisions::context::AttackersContext,
        ) -> Vec<crate::decisions::spec::AttackerDeclaration> {
            let attacker = ctx
                .attacker_options
                .iter()
                .find(|option| {
                    game.current_name(option.creature)
                        .is_some_and(|name| name == self.attacker_name)
                })
                .expect("expected the named attacker to be legal");
            let target = attacker
                .valid_targets
                .iter()
                .find_map(|target| match target {
                    AttackTarget::Player(player) => Some(*player),
                    _ => None,
                })
                .expect("expected a player attack target");

            vec![crate::decisions::spec::AttackerDeclaration {
                creature: attacker.creature,
                target: AttackTarget::Player(target),
            }]
        }

        fn decide_blockers(
            &mut self,
            game: &crate::game_state::GameState,
            ctx: &crate::decisions::context::BlockersContext,
        ) -> Vec<crate::decisions::spec::BlockerDeclaration> {
            assert_eq!(
                ctx.blocker_options.len(),
                1,
                "expected one attacker to block"
            );
            let option = &ctx.blocker_options[0];
            let legal_blockers = option
                .valid_blockers
                .iter()
                .map(|(id, _)| {
                    game.current_name(*id)
                        .unwrap_or_else(|| format!("object-{id:?}"))
                })
                .collect::<Vec<_>>();
            assert_eq!(
                legal_blockers,
                vec![self.chosen_name.to_string()],
                "only the chosen pile should be legal to block"
            );

            let blocker = option
                .valid_blockers
                .iter()
                .find(|(id, _)| {
                    game.current_name(*id)
                        .is_some_and(|name| name == self.chosen_name)
                })
                .map(|(id, _)| *id)
                .expect("expected chosen blocker to remain legal");

            vec![crate::decisions::spec::BlockerDeclaration {
                blocker,
                blocking: option.attacker,
            }]
        }
    }

    let def = stand_or_fall_probe_definition();
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let stand_or_fall_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let attacker_id = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(2), "Attacking Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        alice,
        Zone::Battlefield,
    );
    game.remove_summoning_sickness(attacker_id);

    let chosen_id = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(3), "Chosen Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        bob,
        Zone::Battlefield,
    );
    let other_id = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(4), "Other Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        bob,
        Zone::Battlefield,
    );

    let mut dm = ChooseNamedPileDecisionMaker {
        chosen_name: "Chosen Bear",
        attacker_name: "Attacking Bear",
    };

    let triggered = match &def.abilities[0].kind {
        AbilityKind::Triggered(triggered) => triggered,
        other => panic!("expected Stand or Fall to compile to a triggered ability, got {other:?}"),
    };
    let mut ctx = crate::effects::ExecutionContext::new(stand_or_fall_id, alice, &mut dm)
        .with_defending_player(bob);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        stand_or_fall_id,
        &triggered.effects,
        None,
        &[],
    )
    .expect("Stand or Fall's combat trigger should resolve");

    assert!(
        game.battlefield.contains(&stand_or_fall_id),
        "Stand or Fall should remain on the battlefield after the turn"
    );
    assert!(
        game.can_block(chosen_id),
        "creatures in the chosen pile should remain able to block"
    );
    assert!(
        !game.can_block(other_id),
        "creatures outside the chosen pile should be unable to block"
    );
}

#[test]
pub(super) fn incriminate_keeps_same_controller_choice_sacrifice() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Incriminate Probe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Choose two target creatures controlled by the same player. That player sacrifices one of them of their choice.",
        )
        .expect("parse Incriminate text");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "Choose two target creatures controlled by the same player. That player sacrifices one of them of their choice"
        ),
        "expected same-controller choice-sacrifice wording, got {rendered}"
    );

    let effects = def.spell_effect.as_ref().expect("spell effect");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("ChooseObjectsEffect")
            && debug.contains("ControllerOf")
            && debug.contains("target_set_same_controller: true")
            && debug.contains("SacrificeTargetEffect"),
        "expected target set, controller choice, and sacrifice, got {debug}"
    );
}

#[test]
pub(super) fn barrins_spite_keeps_same_controller_choice_sacrifice_return_other() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Barrin's Spite Probe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Choose two target creatures controlled by the same player. Their controller chooses and sacrifices one of them. Return the other to its owner's hand.",
        )
        .expect("parse Barrin's Spite text");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "Choose two target creatures controlled by the same player. Their controller chooses and sacrifices one of them. Return the other to its owner's hand"
        ),
        "expected same-controller choice-sacrifice-return wording, got {rendered}"
    );

    let effects = def.spell_effect.as_ref().expect("spell effect");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("target_set_same_controller: true")
            && debug.contains("ChooseObjectsEffect")
            && debug.contains("ControllerOf")
            && debug.contains("SacrificeTargetEffect")
            && debug.contains("ReturnToHandEffect")
            && debug.contains("IsNotTaggedObject"),
        "expected target set, controller choice, sacrifice, and return-other, got {debug}"
    );
}

#[test]
pub(super) fn parse_oracle_satyrs_cunning_keeps_escape_exile_clause_in_scored_text() {
    let def = parse_oracle_card_definition("Satyr's Cunning");
    let rendered = crate::compiled_text::compiled_text_lines(&def).join("\n");

    assert!(
        rendered.contains("Create a 1/1 red Satyr creature token with \"This token can't block.\"")
            && rendered.contains("Escape")
            && rendered.contains("{2}{R}")
            && rendered.contains("Exile two other cards from your graveyard"),
        "expected Satyr's Cunning scored text to keep the token and escape clauses, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_axgard_artisan_keeps_first_time_each_turn_in_scored_text() {
    let def = parse_oracle_card_definition("Axgard Artisan");
    let rendered = crate::compiled_text::compiled_text_lines(&def).join("\n");

    assert!(
        rendered.contains("for the first time each turn")
            && !rendered.contains("This ability triggers only once each turn"),
        "expected Axgard Artisan scored text to preserve first-time-each-turn wording, got {rendered}"
    );
}

#[test]
pub(super) fn parse_the_sixth_doctor_keeps_once_each_turn_in_scored_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(81_601), "The Sixth Doctor")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Green],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .supertypes(vec![crate::types::Supertype::Legendary])
        .subtypes(vec![crate::types::Subtype::Doctor])
        .power_toughness(crate::card::PowerToughness::fixed(3, 3))
        .parse_text(
            "Time Lord's Prerogative — Whenever you cast a historic spell, copy it, except the copy isn't legendary. This ability triggers only once each turn.",
        )
        .expect("The Sixth Doctor should parse");
    let rendered = crate::compiled_text::compiled_text_lines(&def).join("\n");

    assert!(
        rendered.contains("This ability triggers only once each turn")
            && !rendered.contains("for the first time each turn"),
        "expected The Sixth Doctor scored text to preserve the once-per-turn suffix, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_grasping_current_keeps_named_multi_zone_search_surface() {
    let def = parse_oracle_card_definition("Grasping Current");
    let rendered = crate::compiled_text::compiled_text_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains(
            "search your library and/or graveyard for a card named jace ingenious mind mage"
        ) && rendered.contains("reveal it")
            && rendered.contains("put it into your hand")
            && (rendered.contains("if you search your library this way, shuffle")
                || rendered.contains("if you searched your library this way, shuffle")),
        "expected Grasping Current scored text to keep the multi-zone search wording, got {rendered}"
    );
    assert!(
        !rendered.contains("searched_multi_zone") && !rendered.contains("for each tagged"),
        "expected Grasping Current scored text to avoid internal helper leakage, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_burning_rune_demon_keeps_two_card_divvy_surface() {
    let def = parse_oracle_card_definition("Burning-Rune Demon");
    let rendered = crate::compiled_text::compiled_text_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains(
            "search your library for exactly two cards not named burning rune demon that have different names"
        ) && rendered.contains("an opponent chooses one of them")
            && rendered.contains("put the chosen card into your hand and the other into your graveyard")
            && (rendered.contains("then shuffle") || rendered.contains("shuffle your library")),
        "expected Burning-Rune Demon scored text to keep the full divvy wording, got {rendered}"
    );
    assert!(
        !rendered.contains("divvy_source")
            && !rendered.contains("divvy_chosen")
            && !rendered.contains("tags it as"),
        "expected Burning-Rune Demon scored text to avoid helper-tag leakage, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_gemini_engine_keeps_named_twin_pt_surface() {
    let def = parse_oracle_card_definition("Gemini Engine");
    let rendered = crate::compiled_text::compiled_text_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("token named twin that's attacking")
            && rendered.contains("its power is equal to this creature's power")
            && rendered.contains("its toughness is equal to this creature's toughness")
            && rendered.contains("sacrifice it at end of combat"),
        "expected Gemini Engine scored text to keep Twin's attacking and dynamic P/T wording, got {rendered}"
    );
    assert!(
        !rendered.contains("artifact you control")
            && !rendered.contains("base power and toughness"),
        "expected Gemini Engine scored text to avoid construct-CDA and raw base-PT leakage, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_consult_the_star_charts_keeps_kicker_choice_override_surface() {
    let def = parse_oracle_card_definition("Consult the Star Charts");
    let rendered = crate::compiled_text::compiled_text_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains(
            "look at the top x cards of your library, where x is the number of lands you control"
        ) && rendered.contains("put one of those cards into your hand")
            && rendered.contains(
                "if this spell was kicked, put two of those cards into your hand instead"
            )
            && rendered.contains("put the rest on the bottom of your library"),
        "expected Consult the Star Charts scored text to keep the kicked count override, got {rendered}"
    );
    assert!(
        !rendered.contains("__sentence_helper") && !rendered.contains("tags it as"),
        "expected Consult the Star Charts scored text to avoid helper-tag leakage, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_cream_of_the_crop_keeps_power_scaled_rearrange_surface() {
    let def = parse_oracle_card_definition("Cream of the Crop");
    let rendered = crate::compiled_text::compiled_text_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    let abilities_debug = format!("{:#?}", def.abilities).to_ascii_lowercase();

    assert!(
        rendered.contains("whenever a creature you control enters")
            && rendered.contains("look at the top x cards of your library")
            && rendered.contains("where x is its power")
            && rendered.contains("if you do")
            && rendered.contains("put one of those cards on top of your library")
            && rendered.contains("the rest on the bottom of your library in any order"),
        "expected Cream of the Crop scored text to keep the full top/bottom rearrange wording, got {rendered}"
    );
    assert!(
        abilities_debug.contains("lookattopcardseffect")
            && abilities_debug.contains("powerof")
            && abilities_debug.contains("chooseobjectseffect")
            && abilities_debug.contains("puttaggedremainderonlibrarybottomeffect"),
        "expected Cream of the Crop definition to preserve the source-power look count and tagged top/rest rearrangement, got {abilities_debug}"
    );
}

#[test]
pub(super) fn parse_oracle_divergent_transformations_keeps_reveal_until_creature_surface() {
    let def = parse_oracle_card_definition("Divergent Transformations");
    let spell_debug = format!("{:#?}", def.spell_effect);
    assert!(
        spell_debug.contains("ConsultTopOfLibraryEffect"),
        "expected consult-top-of-library lowering, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("ShuffleLibraryEffect"),
        "expected library shuffle follow-up, got {spell_debug}"
    );
    assert!(
        !spell_debug.contains("RevealTopEffect"),
        "expected reveal-until lowering instead of top-card fallback, got {spell_debug}"
    );

    let rendered = crate::compiled_text::compiled_text_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("exile two target creatures")
            && rendered.contains("for each card exiled this way")
            && rendered.contains("until they reveal a creature card")
            && rendered.contains("puts that card onto the battlefield")
            && rendered.contains("then shuffles"),
        "expected oracle-like Divergent Transformations surface, got {rendered}\n{spell_debug}"
    );
}

#[test]
pub(super) fn score_surface_normalizes_granted_death_return_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Granted Death Trigger Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Until end of turn, target creature gains \"When this creature dies, return it to the battlefield tapped under its owner's control with a +1/+1 counter on it.\"",
        )
        .expect("granted death trigger should parse");

    let rendered = crate::compiled_text::compiled_text_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "Until end of turn, target creature gains \"When this creature dies, return it to the battlefield tapped under its owner's control with a +1/+1 counter on it.\""
        ),
        "expected oracle-like granted death trigger surface, got {rendered}"
    );
}

#[test]
pub(super) fn score_surface_normalizes_attached_count_anthem() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Attached Count Anthem Variant")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Myr])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text("This creature gets +1/+1 for each Equipment attached to it.")
        .expect("attached count anthem should parse");

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("AttachedToSource") && !abilities_debug.contains("MatchingFilter"),
        "expected the source anthem to count Equipment attached to itself, got {abilities_debug}"
    );
    let rendered = crate::compiled_text::compiled_text_lines(&def).join(" ");
    assert!(
        rendered.contains("This creature gets +1/+1 for each Equipment attached to it"),
        "expected attached-count anthem to render with per-object +1/+1 wording, got {rendered}\n{abilities_debug}"
    );
}

#[test]
pub(super) fn score_surface_compacts_each_player_sacrifice_sequence() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Each Player Sacrifice Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Each player loses 1 life, discards a card, sacrifices a creature of their choice, then sacrifices a land of their choice.",
        )
        .expect("each-player sacrifice sequence should parse");

    let rendered = crate::compiled_text::compiled_text_lines(&def).join(" ");
    let effects_debug = format!("{:#?}", def.spell_effect);
    assert!(
        rendered.contains(
            "Each player loses 1 life, discards a card, sacrifices a creature of their choice, then sacrifices a land of their choice"
        ),
        "expected compact each-player sacrifice sequence, got {rendered}\n{effects_debug}"
    );
}

#[test]
pub(super) fn score_surface_compacts_jar_hand_exchange_sequence() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Jar Hand Exchange Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "{T}, Sacrifice this artifact: Each player exiles all cards from their hand face down and draws seven cards. At the beginning of the next end step, each player discards their hand and returns to their hand each card they exiled this way.",
        )
        .expect("jar hand exchange should parse");

    let rendered = crate::compiled_text::compiled_text_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "Each player exiles all cards from their hand face down and draws seven cards. At the beginning of the next end step, each player discards their hand and returns to their hand each card they exiled this way"
        ),
        "expected compact jar hand exchange surface, got {rendered}"
    );
}

#[test]
pub(super) fn score_surface_normalizes_exiled_flashback_return() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Exiled Flashback Return Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Return target exiled card with flashback you own to your hand.")
        .expect("exiled flashback return should parse");

    let rendered = crate::compiled_text::compiled_text_lines(&def).join(" ");
    assert!(
        rendered.contains("Return target exiled card with flashback you own to your hand"),
        "expected exiled flashback return surface, got {rendered}"
    );
}

#[test]
pub(super) fn card_fixer_parse_sacrifice_unless_it_escaped_keeps_escape_condition() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Escape Sacrifice Variant")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text(
            "When this creature enters, sacrifice it unless it escaped.\nEscape—{R}, Exile one other card from your graveyard.",
        )
        .expect("escape sacrifice condition should parse");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("ThisSpellEscaped"),
        "expected sacrifice condition to lower through ThisSpellEscaped, got {debug}"
    );

    let rendered = crate::compiled_text::compiled_text_lines(&def).join("\n");
    assert!(
        rendered.contains("sacrifice it unless it escaped"),
        "expected escape sacrifice surface, got {rendered}"
    );
}

#[test]
pub(super) fn card_fixer_parse_escapes_with_counter_line_lowers_to_escape_condition() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Escape Counter Variant")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 1))
        .parse_text(
            "Haste\nEscape—{R}, Exile three other cards from your graveyard.\nThis creature escapes with a +1/+1 counter on it.",
        )
        .expect("escape enters-with-counter line should parse");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("EnterWithCountersIfCondition"),
        "expected conditional enters-with-counters static ability, got {debug}"
    );
    assert!(
        debug.contains("ThisSpellEscaped"),
        "expected escape condition to lower through ThisSpellEscaped, got {debug}"
    );

    let rendered = crate::compiled_text::compiled_text_lines(&def).join("\n");
    assert!(
        rendered.contains("escapes with a +1/+1 counter on it"),
        "expected escaped conditional enters-with-counter surface, got {rendered}"
    );
}

#[test]
pub(super) fn card_fixer_parse_characteristic_defining_domain_counts_basic_land_types() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Domain Kavu Variant")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::new(
            crate::card::PtValue::Star,
            crate::card::PtValue::Star,
        ))
        .parse_text(
            "Domain — This creature's power and toughness are each equal to the number of basic land types among lands you control.",
        )
        .expect("domain characteristic-defining P/T should parse");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("BasicLandTypesAmong"),
        "expected domain CDA to count distinct basic land types, got {debug}"
    );

    let rendered = crate::compiled_text::compiled_text_lines(&def).join("\n");
    assert!(
        rendered.contains("basic land types among lands you control"),
        "expected domain CDA surface to mention basic land types, got {rendered}"
    );
}

#[test]
pub(super) fn card_fixer_parse_color_conditional_keyword_grants_merge_to_oracle_surface() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Color Conditional Grants Variant")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .subtypes(vec![Subtype::Dragon])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "Each creature you control has vigilance if it's white, hexproof if it's blue, lifelink if it's black, first strike if it's red, and trample if it's green.",
        )
        .expect("color-conditional keyword grants should parse");

    let rendered = crate::compiled_text::compiled_text_lines(&def).join("\n");
    assert!(
        rendered.contains(
            "Each creature you control has vigilance if it's white, hexproof if it's blue, lifelink if it's black, first strike if it's red, and trample if it's green"
        ),
        "expected merged color-conditional grant surface, got {rendered}"
    );
}

#[test]
pub(super) fn esper_origins_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Esper Origins // Summon: Esper Maduin");

    let front = parse_oracle_card_definition("Esper Origins // Summon: Esper Maduin");
    let rendered = crate::compiled_text::compiled_text_lines(&front).join("\n");
    let debug = format!("{:#?}", front.spell_effect);
    assert!(
        rendered.contains(
            "exile it, then put it onto the battlefield transformed under its owner's control with a finality counter on it"
        ),
        "Esper Origins should render the transformed finality-counter return clause, got {rendered}"
    );
    assert!(
        debug.contains("ThisSpellWasCastFromZone")
            && debug.contains("Graveyard")
            && debug.contains("enters_transformed: true")
            && debug.contains("Finality"),
        "Esper Origins should structurally lower graveyard-cast transform and finality counter effects, got {debug}"
    );

    let back = summon_esper_maduin_test_definition();
    let back_rendered = crate::compiled_text::compiled_text_lines(&back).join("\n");
    assert!(
        back_rendered.contains(
            "I — Reveal the top card of your library. If it's a permanent card, put it into your hand."
        ) && back_rendered.contains(
            "III — Other creatures you control get +2/+2 and gain trample until end of turn."
        ),
        "Summon: Esper Maduin should render oracle-style Saga chapters, got {back_rendered}"
    );
}

#[test]
pub(super) fn hundred_battle_veteran_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Hundred-Battle Veteran");

    let def = parse_oracle_card_definition("Hundred-Battle Veteran");
    let rendered = crate::compiled_text::compiled_text_lines(&def).join("\n");
    let debug = format!("{def:#?}");
    assert_eq!(
        rendered,
        "As long as there are three or more different kinds of counters among creatures you control, this creature gets +2/+4.\nYou may cast this card from your graveyard. If you do, it enters with a finality counter on it.",
        "Hundred-Battle Veteran should render its exact counter-threshold anthem and graveyard-cast finality clause"
    );
    assert!(
        debug.contains("DistinctCounterTypesAmong")
            && debug.contains("cast_this_way_grants")
            && debug.contains("EnterWithCounters")
            && debug.contains("Finality"),
        "Hundred-Battle Veteran should structurally lower distinct counter kinds and cast-this-way finality counter grant, got {debug}"
    );
}

#[test]
pub(super) fn esper_origins_graveyard_cast_condition_moves_source_with_finality_counter() {
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::tests::test_helpers::setup_two_player_game;

    let (def, back_def) = esper_origins_linked_test_definitions();
    let conditional = def
        .spell_effect
        .as_ref()
        .expect("Esper Origins spell effect")
        .flattened_default_effects()
        .into_iter()
        .find(|effect| {
            effect
                .downcast_ref::<crate::effects::ConditionalEffect>()
                .is_some()
        })
        .expect("Esper Origins graveyard-cast conditional");

    let mut normal_game = setup_two_player_game();
    normal_game.register_linked_face_definition(&back_def);
    let alice = PlayerId::from_index(0);
    let normal_source = normal_game.create_object_from_definition(&def, alice, Zone::Stack);
    let mut normal_ctx = ExecutionContext::new_default(normal_source, alice);
    execute_effect(&mut normal_game, conditional, &mut normal_ctx)
        .expect("normal-cast conditional should resolve");
    assert_eq!(
        normal_game
            .object(normal_source)
            .expect("source exists")
            .zone,
        Zone::Stack,
        "Esper Origins should not move itself when it was not cast from a graveyard"
    );
    assert!(
        normal_game.objects_in_zone(Zone::Battlefield).is_empty(),
        "normal-cast condition should not put Esper Origins onto the battlefield"
    );

    let mut flashback_game = setup_two_player_game();
    flashback_game.register_linked_face_definition(&back_def);
    let flashback_source = flashback_game.create_object_from_definition(&def, alice, Zone::Stack);
    let mut flashback_ctx = ExecutionContext::new_default(flashback_source, alice)
        .with_casting_method(crate::alternative_cast::CastingMethod::GrantedFlashback);
    execute_effect(&mut flashback_game, conditional, &mut flashback_ctx)
        .expect("graveyard-cast conditional should resolve");
    let battlefield = flashback_game.objects_in_zone(Zone::Battlefield);
    assert_eq!(
        battlefield.len(),
        1,
        "graveyard-cast Esper Origins should put itself onto the battlefield, got {battlefield:?}"
    );
    let returned = flashback_game
        .object(battlefield[0])
        .expect("graveyard-cast Esper Origins should remain on the battlefield");
    assert_eq!(returned.name, "Summon: Esper Maduin");
    assert!(returned.card_types.contains(&CardType::Enchantment));
    assert!(returned.card_types.contains(&CardType::Creature));
    assert_eq!(
        flashback_game.controller_of(returned),
        alice,
        "graveyard-cast Esper Origins should return transformed under its owner's control"
    );
    assert_eq!(
        flashback_game.counter_count(battlefield[0], crate::object::CounterType::Finality),
        1,
        "graveyard-cast Esper Origins should return with one finality counter"
    );
}

pub(super) fn esper_origins_linked_test_definitions() -> (CardDefinition, CardDefinition) {
    let front_id = CardId::from_raw(605_190_001);
    let back_id = CardId::from_raw(605_190_002);
    let front_text = oracle_text_by_name()
        .get("Esper Origins")
        .expect("Esper Origins front-face oracle text")
        .clone();
    let front = CardDefinitionBuilder::new(front_id, "Esper Origins")
        .card_types(vec![CardType::Sorcery])
        .other_face(back_id)
        .other_face_name("Summon: Esper Maduin")
        .linked_face_layout(crate::card::LinkedFaceLayout::TransformLike)
        .parse_text(front_text)
        .expect("Esper Origins front face should parse");
    let mut back = summon_esper_maduin_test_definition();
    back.card.id = back_id;
    back.card.other_face = Some(front_id);
    back.card.other_face_name = Some("Esper Origins".to_string());
    back.card.linked_face_layout = crate::card::LinkedFaceLayout::TransformLike;
    (front, back)
}

#[test]
pub(super) fn summon_esper_maduin_saga_chapters_resolve_their_branches() {
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::tests::test_helpers::setup_two_player_game;

    fn execute_chapter(
        game: &mut crate::game_state::GameState,
        ability: &crate::ability::TriggeredAbility,
        source: ObjectId,
        controller: PlayerId,
    ) {
        let mut ctx = ExecutionContext::new_default(source, controller);
        for effect in ability.effects.flattened_default_effects() {
            if effect
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_some()
            {
                continue;
            }
            execute_effect(game, effect, &mut ctx).expect("Saga chapter effect should resolve");
        }
    }

    let saga = summon_esper_maduin_test_definition();
    let chapters = saga
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(chapters.len(), 3, "expected three Saga chapter abilities");

    let alice = PlayerId::from_index(0);
    let permanent = CardDefinitionBuilder::new(CardId::new(), "Library Permanent")
        .card_types(vec![CardType::Enchantment])
        .build();
    let mut permanent_game = setup_two_player_game();
    let permanent_source =
        permanent_game.create_object_from_definition(&saga, alice, Zone::Battlefield);
    permanent_game.create_object_from_definition(&permanent, alice, Zone::Library);
    execute_chapter(&mut permanent_game, chapters[0], permanent_source, alice);
    assert_eq!(
        permanent_game.objects_in_zone(Zone::Hand).len(),
        1,
        "chapter I should put a revealed permanent card into your hand"
    );

    let instant = CardDefinitionBuilder::new(CardId::new(), "Library Instant")
        .card_types(vec![CardType::Instant])
        .build();
    let mut instant_game = setup_two_player_game();
    let instant_source =
        instant_game.create_object_from_definition(&saga, alice, Zone::Battlefield);
    instant_game.create_object_from_definition(&instant, alice, Zone::Library);
    execute_chapter(&mut instant_game, chapters[0], instant_source, alice);
    assert!(
        instant_game.objects_in_zone(Zone::Hand).is_empty(),
        "chapter I should not put a revealed nonpermanent card into your hand"
    );

    let creature = CardDefinitionBuilder::new(CardId::new(), "Other Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let mut pump_game = setup_two_player_game();
    let pump_source = pump_game.create_object_from_definition(&saga, alice, Zone::Battlefield);
    let other_creature =
        pump_game.create_object_from_definition(&creature, alice, Zone::Battlefield);
    execute_chapter(&mut pump_game, chapters[2], pump_source, alice);
    assert_eq!(pump_game.calculated_power(other_creature), Some(3));
    assert_eq!(pump_game.calculated_toughness(other_creature), Some(3));
    assert!(
        pump_game.current_has_static_ability_id(other_creature, StaticAbilityId::Trample),
        "chapter III should grant trample to other creatures you control"
    );
    assert!(
        !pump_game.current_has_static_ability_id(pump_source, StaticAbilityId::Trample),
        "chapter III should not grant trample to Summon: Esper Maduin itself"
    );
}

pub(super) fn summon_esper_maduin_test_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Summon: Esper Maduin")
        .card_types(vec![CardType::Enchantment, CardType::Creature])
        .subtypes(vec![Subtype::Saga, Subtype::Elemental])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "I — Reveal the top card of your library. If it's a permanent card, put it into your hand.\n\
             II — Add {G}{G}.\n\
             III — Other creatures you control get +2/+2 and gain trample until end of turn.",
        )
        .expect("Summon: Esper Maduin back face should parse")
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn grouped_dice_roll_cards_preserve_one_or_more_trigger_surface() {
    for name in [
        "Brazen Dwarf",
        "Feywild Trickster",
        "Vrondiss, Rage of Ancients",
    ] {
        let def = parse_oracle_card_definition(name);
        let rendered = unprocessed_compiled_lines(&def).join("\n");
        assert!(
            rendered.contains("Whenever you roll one or more dice"),
            "{name} should preserve its grouped dice-roll trigger surface, got {rendered}"
        );

        let debug = format!("{:#?}", def.abilities);
        assert!(
            debug.contains("PlayerRollsDieTrigger") && debug.contains("one_or_more: true"),
            "{name} should lower to the grouped typed die-roll matcher, got {debug}"
        );
    }
}
