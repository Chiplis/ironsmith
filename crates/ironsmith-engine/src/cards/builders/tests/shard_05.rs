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
use super::shard_18::*;
use super::shard_19::*;
use super::shard_20::*;
use super::shard_21::*;
use super::shard_22::*;
use super::shard_23::*;
use super::*;

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_merfolk_cave_diver_strictly_parses_pump_unblockable_explore_trigger() {
    let def = parse_oracle_card_definition("Merfolk Cave-Diver");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert_eq!(
        rendered,
        "Whenever a creature you control explores, this creature gets +1/+0 until end of turn and this creature can't be blocked this turn."
    );

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("KeywordAction")
            && debug.contains("Explore")
            && debug.contains("ModifyPowerToughness")
            && debug.contains("CantEffect")
            && debug.contains("BeBlocked"),
        "expected Merfolk Cave-Diver to lower to explore-triggered pump plus can't-be-blocked effects, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_explore_trigger_revealed_card_filter() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Nicanzil Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever a creature you control explores a nonland card, put a +1/+1 counter on this creature.",
        )
        .expect("explore nonland trigger should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("explores a nonland card"),
        "expected revealed-card filter in rendered trigger, got {rendered}"
    );
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("KeywordActionTaggedObject")
            || (debug.contains("tagged_object_filter") && debug.contains("__public_revealed")),
        "expected explore trigger to filter on the revealed card tag, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_repeated_explore_clauses() {
    let repeated = CardDefinitionBuilder::new(CardId::from_raw(1), "Jadelight Ranger Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature enters, it explores, then it explores again.")
        .expect("explores again should parse");
    let repeated_debug = format!("{:?}", repeated.abilities);
    assert!(
        repeated_debug.matches("ExploreEffect").count() >= 2,
        "expected two explore effects, got {repeated_debug}"
    );
    assert_eq!(
        unprocessed_compiled_lines(&repeated),
        vec!["When this creature enters, it explores, then it explores again.".to_string()],
        "repeated typed explore effects should retain the authored repeat surface"
    );

    let returned = CardDefinitionBuilder::new(CardId::from_raw(3), "Defossilize Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Return target creature card from your graveyard to the battlefield. That creature explores, then it explores again.",
        )
        .expect("a returned creature should remain the repeated explore subject");
    let returned_debug = format!("{:#?}", returned.spell_effect);
    assert!(
        returned_debug.matches("ExploreEffect").count() >= 2
            && returned_debug.contains("explored_0"),
        "the second explore should reference the first explore's object result: {returned_debug}"
    );
    assert_eq!(
        unprocessed_compiled_lines(&returned),
        vec![
            "Return target creature card from your graveyard to the battlefield. That creature explores, then it explores again.".to_string(),
        ]
    );

    let x_times = CardDefinitionBuilder::new(CardId::from_raw(2), "Jadelight Spelunker Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature enters, it explores X times.")
        .expect("explores X times should parse");
    let x_debug = format!("{:?}", x_times.abilities);
    assert!(
        x_debug.contains("RepeatEffectsEffect") && x_debug.contains("ExploreEffect"),
        "expected dynamic repeat of explore, got {x_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_open_attraction_clause_without_fallback_marker() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Open Attraction Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, open an Attraction. (Put the top card of your Attraction deck onto the battlefield.)",
        )
        .expect("open attraction trigger should parse as an explicit mechanic effect");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("open an attraction"),
        "expected open-attraction text in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "open-attraction trigger should not rely on unsupported fallback marker: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn empty_the_laboratory_keeps_dynamic_sacrifice_and_consult_sequence() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Empty the Laboratory Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Sacrifice X Zombies, then reveal cards from the top of your library until you reveal a number of Zombie creature cards equal to the number of Zombies sacrificed this way. Put those cards onto the battlefield and the rest on the bottom of your library in a random order.",
        )
        .expect("Empty the Laboratory should parse");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    let spell_debug = format!("{:#?}", def.spell_effect);
    assert!(
        joined.contains("sacrifice x zombies")
            && joined.contains(
                "reveal cards from the top of your library until you reveal a number of zombie creature cards equal to the number of zombies sacrificed this way",
            )
            && joined.contains("put those cards onto the battlefield")
            && joined.contains("the rest on the bottom of your library in a random order"),
        "expected Empty the Laboratory search/reveal/bottom wording, got {joined}\n{spell_debug}"
    );

    let debug = format!("{:?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        debug.contains("choicecount { min: 0, max: none, dynamic_x: true")
            && debug.contains("sacrificeplayereffect")
            && debug.contains("matchcount(prioreffectmetric")
            && debug.contains("sacrificed")
            && debug.contains("puttaggedremainderonlibrarybottomeffect"),
        "expected Empty the Laboratory to keep dynamic sacrifice, consult, battlefield move, and random bottoming, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn revealed_battlefield_complement_keeps_typed_graveyard_surface() {
    for (selection, name) in [
        ("permanent cards", "Genesis Wave Variant"),
        ("artifact cards", "Saheeli's Directive Variant"),
    ] {
        let oracle = format!(
            "Reveal the top X cards of your library. You may put any number of {selection} with mana value X or less from among them onto the battlefield. Then put all cards revealed this way that weren't put onto the battlefield into your graveyard."
        );
        let def = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Sorcery])
            .parse_text(&oracle)
            .expect("revealed battlefield partition should parse");

        let joined = unprocessed_compiled_lines(&def)
            .join(" ")
            .to_ascii_lowercase();
        assert!(
            joined.contains(
                "then put all cards revealed this way that weren't put onto the battlefield into your graveyard",
            ),
            "expected explicit revealed-card complement in {name}, got {joined}"
        );

        let debug = format!("{:?}", def.spell_effect);
        assert!(
            debug.contains("remainder_surface: Some(RevealedCardsNotPutOntoBattlefield)"),
            "expected typed revealed-card remainder provenance in {name}, got {debug}"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sensational_spider_man_distributes_stun_counter_removal_and_keeps_metric_surface() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sensational Spider-Man")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever Sensational Spider-Man attacks, tap target creature defending player controls and put a stun counter on it. Then you may remove up to three stun counters from among all permanents. Draw cards equal to the number of stun counters removed this way.",
        )
        .expect("distributed stun-counter trigger should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered
            .contains("then you may remove up to three stun counters from among all permanents",)
            && rendered
                .contains("draw cards equal to the number of stun counters removed this way",),
        "expected distributed removal and typed prior-count wording, got {rendered}"
    );
    assert!(!rendered.contains("from a permanent"), "{rendered}");
    assert!(!rendered.contains("draw that many cards"), "{rendered}");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("RemoveUpToCountersEffect")
            && debug.contains("counter_type: Stun")
            && debug.contains("target: All(")
            && debug.contains("PriorEffectMetric")
            && debug.contains("action: Some(Removed)")
            && debug.contains("counter_type: Some(Stun)"),
        "expected all-matching runtime target and typed removed-count query, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn open_the_way_parses_x_cap_and_reveal_x_lands() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Open the Way")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::X],
            vec![ManaSymbol::Green],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "X can't be greater than the number of players in the game.\n\
             Reveal cards from the top of your library until you reveal X land cards. Put those land cards onto the battlefield tapped and the rest on the bottom of your library in a random order.",
        )
        .expect("Open the Way should parse strictly");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("X can't be greater than the number of players in the game")
            && rendered.contains("until you reveal X land cards")
            && rendered.contains("Put those land cards onto the battlefield tapped")
            && rendered.contains("the rest on the bottom of your library in a random order"),
        "expected Open the Way X cap and reveal/battlefield/bottom text, got {rendered}"
    );

    let debug = format!("{:?}", def);
    assert!(
        debug.contains("ThisSpellXMaximum")
            && debug.contains("CountPlayers")
            && debug.contains("MatchCount(X)")
            && debug.contains("ConsultTopOfLibraryEffect")
            && debug.contains("enters_tapped: true")
            && debug.contains("PutTaggedRemainderOnLibraryBottomEffect"),
        "expected Open the Way to lower to an X cap plus X-count consult/move/bottom effects, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn venture_forth_exile_until_land_uses_consult_and_suspend() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Venture Forth")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Exile cards from the top of your library until you exile a land card. Put that card onto the battlefield and the rest on the bottom of your library in a random order. Exile Venture Forth with three time counters on it.\nSuspend 3—{1}{G}",
        )
        .expect("Venture Forth should parse strictly");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("exile cards from the top of your library until you exile a land card")
            && rendered.contains("put that card onto the battlefield")
            && rendered.contains("the rest on the bottom of your library in a random order")
            && rendered.contains("exile venture forth with three time counters on it")
            && rendered.contains("suspend 3"),
        "expected Venture Forth consult, battlefield, remainder, and suspend wording, got {rendered}"
    );
    assert!(
        !rendered.contains("choose the top card"),
        "expected Venture Forth to avoid top-card chooser fallback, got {rendered}"
    );

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("ConsultTopOfLibraryEffect")
            && debug.contains("mode: Exile")
            && debug.contains("MoveToZoneEffect")
            && debug.contains("zone: Battlefield")
            && debug.contains("PutTaggedRemainderOnLibraryBottomEffect"),
        "expected Venture Forth to lower to exile consult, battlefield move, and bottom remainder, got {debug}"
    );
    assert!(
        matches!(
            def.alternative_casts.as_slice(),
            [AlternativeCastingMethod::Suspend { time: 3, .. }]
        ),
        "expected Venture Forth suspend metadata, got {:?}",
        def.alternative_casts
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn costume_shop_keeps_visit_sticker_effect() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Costume Shop")
        .card_types(vec![CardType::Artifact])
        .parse_text("Visit — You may put a sticker on a nonland permanent you own.")
        .expect("Costume Shop should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("you may put a sticker on a nonland permanent you own")
            && !rendered.contains("investigate 0")
            && !rendered.contains("unsupported effect"),
        "expected Costume Shop to keep its sticker visit effect, got {rendered}"
    );

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("PutStickerEffect") && debug.contains("Sticker"),
        "expected Costume Shop to lower to a sticker effect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn eldrazi_guacamole_tightrope_ticket_sticker_lines_parse_strictly() {
    let text = concat!(
        "Type: Stickers\n",
        "{TK}{TK} — Haste\n",
        "{TK}{TK}{TK}{TK}{TK} — You may cast this card from your graveyard by ",
        "paying 2 life in addition to paying its other costs.\n",
        "{TK}{TK} — 1/4\n",
        "{TK}{TK}{TK} — 5/3",
    );
    let def = CardDefinitionBuilder::new(CardId::from_raw(58_3538), "Eldrazi Guacamole Tightrope")
        .parse_text(text)
        .expect("Eldrazi Guacamole Tightrope should parse strictly");

    let rendered = unprocessed_compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("{tk}{tk}{tk}{tk}{tk} — you may cast this card from your graveyard by paying 2 life in addition to paying its other costs"),
        "expected ticket graveyard-cast sticker row to be preserved, got {rendered}"
    );
    assert!(
        rendered.contains("{tk}{tk} — haste")
            && rendered.contains("{tk}{tk} — 1/4")
            && rendered.contains("{tk}{tk}{tk} — 5/3"),
        "expected all Eldrazi Guacamole Tightrope sticker rows to render, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported") && !rendered.contains("chosen option"),
        "sticker rows should compile as marker text without fallback or chosen-option conditions: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn eldrazi_guacamole_tightrope_sticker_marker_does_not_grant_graveyard_casting() {
    let text = concat!(
        "Type: Stickers\n",
        "{TK}{TK} — Haste\n",
        "{TK}{TK}{TK}{TK}{TK} — You may cast this card from your graveyard by ",
        "paying 2 life in addition to paying its other costs.\n",
        "{TK}{TK} — 1/4\n",
        "{TK}{TK}{TK} — 5/3",
    );
    let def = CardDefinitionBuilder::new(CardId::from_raw(58_3539), "Eldrazi Guacamole Tightrope")
        .parse_text(text)
        .expect("Eldrazi Guacamole Tightrope should parse strictly");

    let debug = format!("{:?}", def.abilities).to_ascii_lowercase();
    assert!(
        debug.contains("keywordmarker") && debug.contains("paying 2 life"),
        "expected graveyard-cast sticker row to remain marker text, got {debug}"
    );
    assert!(
        !debug.contains("grantstaticability") && !debug.contains("graveyardcastfromcardmanacost"),
        "sticker marker text should not create an intrinsic graveyard-cast permission: {debug}"
    );

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let card_id = game.create_object_from_definition(&def, alice, Zone::Graveyard);

    assert!(
        !game.effect_store.grant_registry.card_can_play_from_zone(
            &game,
            card_id,
            Zone::Graveyard,
            alice
        ),
        "sticker marker text should not grant cast-from-graveyard runtime permission"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_adapt_activation_with_reminder_text_without_fallback_marker() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Adapt Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{2}{U}: Adapt 2. (If this creature has no +1/+1 counters on it, put two +1/+1 counters on it.)",
        )
        .expect("adapt activated line should parse as an explicit mechanic effect");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("adapt 2"),
        "expected adapt text in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "adapt activation should not rely on unsupported fallback marker: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_rageform_parses_and_renders_aura_become_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Rageform")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "When this enchantment enters, it becomes an Aura with enchant creature. Manifest the top card of your library and attach this enchantment to it. (To manifest a card, put it onto the battlefield face down as a 2/2 creature. Turn it face up any time for its mana cost if it's a creature card.)\nEnchanted creature has double strike. (It deals both first-strike and regular combat damage.)",
        )
        .expect("Rageform should parse strictly");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("becomes an aura with enchant creature"),
        "expected aura become + enchant restriction clauses in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains("manifest the top card of your library"),
        "expected manifest clause in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains("attach this enchantment to it"),
        "expected attach clause in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains("enchanted creature has double strike"),
        "expected enchanted-creature static line, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "Rageform should not rely on parser fallback markers: {rendered}"
    );

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Rageform should have an enters trigger");
    let effects = triggered.effects.flattened_default_effects();

    fn manifested_tag(effect: &crate::effect::Effect) -> Option<crate::TagKey> {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            if tagged
                .effect
                .downcast_ref::<crate::effects::ManifestTopCardOfLibraryEffect>()
                .is_some()
            {
                return Some(tagged.tag.clone());
            }
            return manifested_tag(&tagged.effect);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
            if tagged
                .effect
                .downcast_ref::<crate::effects::ManifestTopCardOfLibraryEffect>()
                .is_some()
            {
                return Some(tagged.tag.clone());
            }
            return manifested_tag(&tagged.effect);
        }
        effect
            .downcast_ref::<crate::effects::SequenceEffect>()
            .and_then(|sequence| sequence.effects.iter().find_map(manifested_tag))
    }

    let manifested_tag = effects.iter().find_map(manifested_tag).unwrap_or_else(|| {
        panic!("manifest should export the manifested permanent under a tag: {effects:#?}")
    });
    fn attachment_specs(effect: &crate::effect::Effect) -> Option<(ChooseSpec, ChooseSpec)> {
        if let Some(attach) = effect.downcast_ref::<crate::effects::AttachObjectsEffect>() {
            return Some((attach.objects.clone(), attach.target.clone()));
        }
        let mut found = None;
        effect.visit_child_effects(&mut |child| {
            if found.is_none() {
                found = attachment_specs(child);
            }
        });
        found
    }
    let (attach_objects, attach_target) = effects
        .iter()
        .find_map(attachment_specs)
        .expect("Rageform should attach itself after manifesting");
    assert_eq!(attach_objects, ChooseSpec::Source);
    assert_eq!(attach_target, ChooseSpec::Tagged(manifested_tag));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cryptic_coat_cloaks_then_attaches_to_the_cloaked_card() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cryptic Coat")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .parse_text(
            "When this Equipment enters, cloak the top card of your library, then attach this Equipment to it. (To cloak a card, put it onto the battlefield face down as a 2/2 creature with ward {2}. Turn it face up any time for its mana cost if it's a creature card.)\nEquipped creature gets +1/+0 and can't be blocked.\n{1}{U}: Return this Equipment to its owner's hand.",
        )
        .expect("Cryptic Coat should parse strictly");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("cloak the top card of your library"),
        "expected cloak action in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains("attach this equipment to it"),
        "expected attachment follow-up in compiled text, got {rendered}"
    );

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Cryptic Coat should have an enters trigger");
    let effects = triggered.effects.flattened_default_effects();

    fn cloaked_tag(effect: &crate::effect::Effect) -> Option<crate::TagKey> {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>()
            && tagged
                .effect
                .downcast_ref::<crate::effects::ManifestTopCardOfLibraryEffect>()
                .is_some_and(|cloak| cloak.cloak)
        {
            return Some(tagged.tag.clone());
        }

        let mut found = None;
        effect.visit_child_effects(&mut |child| {
            if found.is_none() {
                found = cloaked_tag(child);
            }
        });
        found
    }

    let cloaked_tag = effects
        .iter()
        .find_map(cloaked_tag)
        .expect("cloak should export the cloaked permanent under a tag");

    fn attachment_specs(effect: &crate::effect::Effect) -> Option<(ChooseSpec, ChooseSpec)> {
        if let Some(attach) = effect.downcast_ref::<crate::effects::AttachObjectsEffect>() {
            return Some((attach.objects.clone(), attach.target.clone()));
        }

        let mut found = None;
        effect.visit_child_effects(&mut |child| {
            if found.is_none() {
                found = attachment_specs(child);
            }
        });
        found
    }

    let (attach_objects, attach_target) = effects
        .iter()
        .find_map(attachment_specs)
        .expect("Cryptic Coat should attach itself after cloaking");
    assert_eq!(attach_objects, ChooseSpec::Source);
    assert_eq!(attach_target, ChooseSpec::Tagged(cloaked_tag));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn pre_war_formalwear_attaches_to_the_returned_creature() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Pre-War Formalwear")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .parse_text(
            "When this Equipment enters, return target creature card with mana value 3 or less from your graveyard to the battlefield and attach this Equipment to it.",
        )
        .expect("Pre-War Formalwear should parse strictly");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Pre-War Formalwear should have an enters trigger");
    let effects = triggered.effects.flattened_default_effects();
    fn find_returned_object_tag(effect: &crate::effect::Effect) -> Option<crate::tag::TagKey> {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            let is_battlefield_return = tagged
                .effect
                .downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
                .is_some();
            let is_generic_battlefield_move = tagged
                .effect
                .downcast_ref::<crate::effects::MoveToZoneEffect>()
                .is_some_and(|moved| moved.zone == Zone::Battlefield);
            if is_battlefield_return || is_generic_battlefield_move {
                return Some(tagged.tag.clone());
            }
        }

        let mut found = None;
        effect.visit_child_effects(&mut |child| {
            if found.is_none() {
                found = find_returned_object_tag(child);
            }
        });
        found
    }
    let returned_tag = effects
        .iter()
        .find_map(find_returned_object_tag)
        .expect("the returned creature should be tagged after its zone change");
    fn find_attachment_specs(
        effect: &crate::effect::Effect,
    ) -> Option<(crate::target::ChooseSpec, crate::target::ChooseSpec)> {
        if let Some(attach) = effect.downcast_ref::<crate::effects::AttachObjectsEffect>() {
            return Some((attach.objects.clone(), attach.target.clone()));
        }

        let mut found = None;
        effect.visit_child_effects(&mut |child| {
            if found.is_none() {
                found = find_attachment_specs(child);
            }
        });
        found
    }
    let (attach_objects, attach_target) = effects
        .iter()
        .find_map(find_attachment_specs)
        .expect("Pre-War Formalwear should attach itself after the return");
    assert_eq!(attach_objects, ChooseSpec::Source);
    assert_eq!(attach_target, ChooseSpec::Tagged(returned_tag));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn runes_of_the_deus_strict_parse_and_compiled_text_conditions() {
    let oracle = oracle_text_by_name()
        .get("Runes of the Deus")
        .expect("missing oracle text for Runes of the Deus")
        .clone();
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Runes of the Deus")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(oracle)
        .expect("Runes of the Deus should parse strictly");

    let rendered = unprocessed_compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("enchant creature"),
        "expected Aura enchant restriction in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains(
            "as long as enchanted creature is red, enchanted creature gets +1/+1 and has double strike"
        ),
        "expected red conditional double-strike grant in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains(
            "as long as enchanted creature is green, enchanted creature gets +1/+1 and has trample"
        ),
        "expected green conditional trample grant in compiled text, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "Runes of the Deus should not rely on parser fallback markers: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn runes_of_the_deus_runtime_applies_color_condition_branches() {
    fn assert_runes_branch(
        target_colors: crate::color::ColorSet,
        expected_power: i32,
        expected_toughness: i32,
        has_double_strike: bool,
        has_trample: bool,
    ) {
        let oracle = oracle_text_by_name()
            .get("Runes of the Deus")
            .expect("missing oracle text for Runes of the Deus")
            .clone();
        let runes = CardDefinitionBuilder::new(CardId::new(), "Runes of the Deus")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .parse_text(oracle)
            .expect("Runes of the Deus should parse strictly");
        let enchanted = CardDefinitionBuilder::new(CardId::new(), "Enchanted Test Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .color_indicator(target_colors)
            .build();

        let alice = PlayerId::from_index(0);
        let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
        let creature_id = game.create_object_from_definition(&enchanted, alice, Zone::Battlefield);
        let runes_id = game.create_object_from_definition(&runes, alice, Zone::Battlefield);
        game.object_mut(runes_id)
            .expect("Runes of the Deus object should exist")
            .attached_to = Some(crate::object::AttachmentTarget::Object(creature_id));
        game.object_mut(creature_id)
            .expect("enchanted creature should exist")
            .attachments
            .push(runes_id);

        assert_eq!(game.calculated_power(creature_id), Some(expected_power));
        assert_eq!(
            game.calculated_toughness(creature_id),
            Some(expected_toughness)
        );
        assert_eq!(
            game.object_has_static_ability_id(creature_id, StaticAbilityId::DoubleStrike),
            has_double_strike
        );
        assert_eq!(
            game.object_has_static_ability_id(creature_id, StaticAbilityId::Trample),
            has_trample
        );
    }

    assert_runes_branch(crate::color::ColorSet::RED, 3, 3, true, false);
    assert_runes_branch(crate::color::ColorSet::GREEN, 3, 3, false, true);
    assert_runes_branch(
        crate::color::ColorSet::RED.union(crate::color::ColorSet::GREEN),
        4,
        4,
        true,
        true,
    );
    assert_runes_branch(crate::color::ColorSet::BLUE, 2, 2, false, false);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_manifest_dread_trigger_without_fallback_marker() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Manifest Dread Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature dies, manifest dread. (Look at the top two cards of your library. Put one onto the battlefield face down as a 2/2 creature and the other into your graveyard. Turn it face up any time for its mana cost if it's a creature card.)",
        )
        .expect("manifest-dread trigger should parse as an explicit mechanic effect");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("manifest dread"),
        "expected manifest-dread text in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "manifest-dread trigger should not rely on unsupported fallback marker: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn paranormal_analyst_manifest_dread_observer_round_trips_exactly() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Paranormal Analyst Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever you manifest dread, put a card you put into your graveyard this way into your hand.",
        )
        .expect("manifest-dread observer should parse strictly");

    assert_eq!(
        unprocessed_compiled_lines(&def).join(" "),
        "Whenever you manifest dread, put a card you put into your graveyard this way into your hand."
    );
    let debug = format!("{:#?}", def.abilities);
    assert!(debug.contains("ManifestDread"), "{debug}");
    assert!(debug.contains("TagTriggeringObjectEffect"), "{debug}");
    assert!(debug.contains("MoveToZoneEffect"), "{debug}");
    assert!(
        debug.contains(crate::tag::MANIFEST_DREAD_GRAVEYARD_TAG),
        "{debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn they_came_from_the_pipes_strict_parse_includes_manifest_dread_twice_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "They Came from the Pipes")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "When this enchantment enters, manifest dread twice. (To manifest dread, look at the top two cards of your library. Put one onto the battlefield face down as a 2/2 creature and the other into your graveyard. Turn it face up any time for its mana cost if it's a creature card.)\nWhenever a face-down creature you control enters, draw a card.",
        )
        .expect("They Came from the Pipes should parse strictly");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("manifest dread twice") && !rendered.contains("repeat manifest dread"),
        "expected compiled text to preserve the manifest-dread-twice clause, got {rendered}"
    );
    assert!(
        rendered.contains("whenever a face-down creature you control enters, draw a card"),
        "expected face-down ETB draw trigger in compiled text, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "They Came from the Pipes should not rely on unsupported parser fallback: {rendered}"
    );

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("RepeatEffects") && abilities_debug.contains("Fixed(2)"),
        "expected manifest dread twice to lower to a repeat effect with count 2, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn standalone_manifest_dread_card_parses_as_a_keyword_action_statement() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Manifest Dread")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Manifest dread.")
        .expect("a bare manifest-dread instruction should parse strictly");

    assert_eq!(
        unprocessed_compiled_lines(&def).join(" "),
        "Manifest dread."
    );
    let debug = format!("{:#?}", def.spell_effect);
    assert!(debug.contains("ManifestDreadEffect"), "{debug}");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn valgavoths_onslaught_counters_only_the_permanents_manifested_by_the_repeat() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Valgavoth's Onslaught")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Manifest dread X times, then put X +1/+1 counters on each of those creatures.")
        .expect("Valgavoth's Onslaught should parse strictly");
    let spell_effects = def.spell_effect.clone().expect("spell effects");
    let debug = format!("{spell_effects:#?}");
    assert!(
        debug.contains("TaggedEffect")
            && debug.contains("RepeatEffectsEffect")
            && debug.contains("manifested_")
            && debug.contains("IsTaggedObject"),
        "the repeated action and plural followup should share tagged provenance: {debug}"
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let unrelated = CardDefinitionBuilder::new(CardId::from_raw(20), "Unrelated Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let unrelated_id = game.create_object_from_definition(&unrelated, alice, Zone::Battlefield);
    for index in 0..4 {
        let library_card =
            CardDefinitionBuilder::new(CardId::from_raw(30 + index), format!("Dread Card {index}"))
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(1, 1))
                .build();
        game.create_object_from_definition(&library_card, alice, Zone::Library);
    }

    let source = game.new_object_id();
    let mut decisions = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice)
        .with_x(2)
        .with_decision_maker(&mut decisions);
    for effect in &spell_effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Valgavoth's Onslaught effect should resolve");
    }

    assert_eq!(
        game.object(unrelated_id)
            .and_then(|object| object.counters.get(&CounterType::PlusOnePlusOne))
            .copied()
            .unwrap_or(0),
        0,
        "an unrelated battlefield creature must not receive counters"
    );
    let manifested = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| *id != unrelated_id)
        .collect::<Vec<_>>();
    assert_eq!(manifested.len(), 2, "X=2 should manifest two permanents");
    for object_id in manifested {
        let object = game.object(object_id).expect("manifested permanent");
        assert!(
            game.is_face_down(object_id),
            "manifest dread should create a face-down permanent"
        );
        assert_eq!(
            object
                .counters
                .get(&CounterType::PlusOnePlusOne)
                .copied()
                .unwrap_or(0),
            2,
            "each manifested permanent should receive X counters"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn they_came_from_the_pipes_face_down_trigger_respects_controller() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "They Came from the Pipes")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "When this enchantment enters, manifest dread twice. (To manifest dread, look at the top two cards of your library. Put one onto the battlefield face down as a 2/2 creature and the other into your graveyard. Turn it face up any time for its mana cost if it's a creature card.)\nWhenever a face-down creature you control enters, draw a card.",
        )
        .expect("They Came from the Pipes should parse strictly");

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let source_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let vanilla_creature = CardDefinitionBuilder::new(CardId::from_raw(2), "Vanilla Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();

    let alice_face_down =
        game.create_object_from_definition(&vanilla_creature, alice, Zone::Battlefield);
    game.set_face_down(alice_face_down);
    let alice_etb = crate::events::RawEvent::new(
        crate::events::ZoneChangeEvent::with_cause(
            alice_face_down,
            Zone::Hand,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            None,
        ),
        crate::provenance::ProvNodeId::default(),
    );

    let alice_trigger_count = crate::triggers::check_triggers(&game, &alice_etb)
        .iter()
        .filter(|entry| entry.source == source_id)
        .count();
    assert_eq!(
        alice_trigger_count, 1,
        "expected They Came from the Pipes to trigger for your own face-down creature ETB"
    );

    let bob_face_down =
        game.create_object_from_definition(&vanilla_creature, bob, Zone::Battlefield);
    game.set_face_down(bob_face_down);
    let bob_etb = crate::events::RawEvent::new(
        crate::events::ZoneChangeEvent::with_cause(
            bob_face_down,
            Zone::Hand,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            None,
        ),
        crate::provenance::ProvNodeId::default(),
    );

    let bob_trigger_count = crate::triggers::check_triggers(&game, &bob_etb)
        .iter()
        .filter(|entry| entry.source == source_id)
        .count();
    assert_eq!(
        bob_trigger_count, 0,
        "expected They Came from the Pipes not to trigger for opponent face-down creature ETB"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_manifest_top_card_of_your_library_without_fallback_marker() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Manifest Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature dies, manifest the top card of your library. (Put that card onto the battlefield face down as a 2/2 creature. Turn it face up any time for its mana cost if it's a creature card.)",
        )
        .expect("manifest trigger should parse as an explicit mechanic effect");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("manifest the top card of your library"),
        "expected manifest text in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "manifest trigger should not rely on unsupported fallback marker: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn scroll_of_fate_parses_strictly() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Scroll of Fate")
        .card_types(vec![CardType::Artifact])
        .parse_text("{T}: Manifest a card from your hand.")
        .expect("Scroll of Fate should parse strictly");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "Scroll of Fate should not rely on unsupported fallback marker: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn scroll_of_fate_compiled_text_keeps_manifest_from_hand_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Scroll of Fate")
        .card_types(vec![CardType::Artifact])
        .parse_text("{T}: Manifest a card from your hand.")
        .expect("Scroll of Fate ability should parse strictly");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("manifest a card from your hand"),
        "expected Scroll of Fate compiled text to keep manifest-from-hand clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_vault_101_birthday_party_parses_strictly() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Vault 101: Birthday Party")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "(As this Saga enters and after your draw step, add a lore counter. Sacrifice after III.)\nI - Create a 1/1 white Human Soldier creature token and a Food token. (A Food token is an artifact with \"{2}, {T}, Sacrifice this token: You gain 3 life.\")\nII, III - You may put an Aura or Equipment card from your hand or graveyard onto the battlefield. If an Equipment is put onto the battlefield this way, you may attach it to a creature you control.",
        )
        .expect("Vault 101: Birthday Party should parse strictly");

    let lines = unprocessed_compiled_lines(&def);
    let rendered = lines.join(" ").to_ascii_lowercase();
    assert!(
        !rendered.contains("unsupported predicate") && !rendered.contains("unsupported effect"),
        "Vault 101: Birthday Party should parse without unsupported markers, got {rendered}"
    );
    let chapter_one = lines
        .iter()
        .find(|line| line.trim_start().starts_with("I —"))
        .unwrap_or_else(|| panic!("expected Vault 101 chapter I, got {lines:#?}"))
        .to_ascii_lowercase();
    assert!(
        chapter_one.contains("human soldier creature token") && chapter_one.contains("food token"),
        "Vault 101 chapter I should retain both coordinated token creations, got {chapter_one}"
    );
    assert!(
        !chapter_one.contains("sacrifice"),
        "Food reminder text must not become a Vault 101 chapter effect, got {chapter_one}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_vault_101_birthday_party_renders_equipment_only_attach_branch() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Vault 101: Birthday Party")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "II, III - You may put an Aura or Equipment card from your hand or graveyard onto the battlefield. If an Equipment is put onto the battlefield this way, you may attach it to a creature you control.",
        )
        .expect("Vault 101: Birthday Party chapter II/III line should parse strictly");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "you may put an aura or equipment card from your hand or graveyard onto the battlefield. if an equipment is put onto the battlefield this way, you may attach it to a creature you control"
        ),
        "expected exact multi-zone move and conditional attach surface, got {rendered}"
    );

    let effect_debug = format!("{def:?}");
    assert!(
        effect_debug.contains("IfEffect")
            && (effect_debug.contains("TaggedObjectMatches")
                || effect_debug.contains("TaggedObjectConstraint"))
            && effect_debug.contains("Equipment")
            && effect_debug.contains("AttachObjectsEffect"),
        "expected parsed chapter line to gate attach behavior to moved Equipment objects, got {effect_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_manifest_top_card_of_that_players_library_without_fallback_marker() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Manifest Theft Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever this creature deals combat damage to a player, manifest the top card of that player's library. (Put that card onto the battlefield face down as a 2/2 creature. Turn it face up any time for its mana cost if it's a creature card.)",
        )
        .expect("manifest that-player trigger should parse as an explicit mechanic effect");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("manifest the top card of that player's library"),
        "expected manifest-that-player text in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "manifest that-player trigger should not rely on unsupported fallback marker: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_manifest_chain_after_create_token_keeps_both_effects() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Orochi Manifest Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever one or more creatures you control deal combat damage to a player, create a Treasure token and manifest the top card of that player's library. (Put that card onto the battlefield face down as a 2/2 creature. Turn it face up any time for its mana cost if it's a creature card.)",
        )
        .expect("create-plus-manifest trigger should keep the full effect chain");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("create a treasure token"),
        "expected Treasure creation in oracle-like output, got {rendered}"
    );
    assert!(
        rendered.contains("manifest the top card of that player's library"),
        "expected manifest tail in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "create-plus-manifest trigger should not rely on unsupported fallback marker: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_manifest_dread_then_multi_counter_followup_keeps_full_chain() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Manifest Door Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "When you unlock this door, manifest dread, then put two +1/+1 counters and a trample counter on that creature.",
        )
        .expect("manifest-dread then counter follow-up should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let lower = rendered.to_ascii_lowercase();
    assert!(
        lower.contains("manifest dread"),
        "expected manifest dread in compiled text, got {rendered}"
    );
    assert!(
        lower.contains("+1/+1 counter"),
        "expected +1/+1 counters in compiled text, got {rendered}"
    );
    assert!(
        lower.contains("trample counter"),
        "expected trample counter in compiled text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_legendary_trigger_uses_card_name_when_oracle_uses_name() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Name Trigger Probe")
        .card_types(vec![CardType::Creature])
        .supertypes(vec![crate::types::Supertype::Legendary])
        .parse_text("Whenever Name Trigger Probe attacks, draw a card.")
        .expect("name-based trigger should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Whenever Name Trigger Probe attacks"),
        "expected rendered trigger to keep the oracle name surface, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn massacre_girl_keeps_named_self_reference_and_other_than_filter() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Massacre Girl")
        .card_types(vec![CardType::Creature])
        .supertypes(vec![crate::types::Supertype::Legendary])
        .parse_text(
            "Menace\n\
             When Massacre Girl enters, each other creature gets -1/-1 until end of turn. Whenever a creature dies this turn, each creature other than Massacre Girl gets -1/-1 until end of turn.",
        )
        .expect("Massacre Girl should parse with named source surfaces");

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("When Massacre Girl enters"),
        "ETB trigger should keep Massacre Girl's oracle name surface, got {rendered}"
    );
    assert!(
        rendered.contains(
            "Whenever a creature dies, each other creature gets -1/-1 until end of turn."
        ),
        "death trigger should target creatures other than Massacre Girl, got {rendered}"
    );
    assert!(
        !rendered.contains("When this creature enters")
            && !rendered.contains("for each other creature")
            && !rendered.contains("this creature gets -1/-1"),
        "Massacre Girl should not fall back to structural or source-targeted wording, got {rendered}"
    );

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("FullName")
            && debug.contains("\"Massacre Girl\"")
            && !debug.contains("ForEachObject"),
        "expected named source surface on an object-filter pump, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_alchemy_prefixed_name_still_resolves_self_reference_triggers() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "A-Oran-Rief Ooze")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When Oran-Rief Ooze enters, put a +1/+1 counter on target creature you control.",
        )
        .expect("alchemy-prefixed source name should normalize to self reference");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("enters, put a +1/+1 counter on target creature you control"),
        "expected enters trigger body to stay intact, got {rendered}"
    );
    assert!(
        !rendered.contains("Whenever a Ooze enters"),
        "alchemy prefix should not degrade source trigger to subtype filter: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn slash_bearing_source_name_resolves_self_reference_triggers() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "SP//dr, Piloted by Peni")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .parse_text(
            "Vigilance
When SP//dr enters, put a +1/+1 counter on target creature.
Whenever a modified creature you control deals combat damage to a player, draw a card.",
        )
        .expect("SP//dr should parse strictly");

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("When SP//dr enters, put a +1/+1 counter on target creature."),
        "slash-bearing source ETB should render as a self-reference, got {rendered}"
    );
    assert!(
        rendered.contains(
            "Whenever a modified creature you control deals combat damage to a player, draw a card."
        ),
        "modified combat-damage draw trigger should render intact, got {rendered}"
    );
    assert!(
        !rendered.contains("When a creature enters"),
        "slash-bearing source ETB should not degrade to a generic creature trigger: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_multiword_name_first_word_still_resolves_self_reference_triggers() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Loran of the Third Path")
        .card_types(vec![CardType::Creature])
        .parse_text("When Loran enters, destroy up to one target artifact or enchantment.")
        .expect("multiword source name shorthand should normalize to self reference");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("When Loran enters")
            || rendered.contains("When Loran of the Third Path enters"),
        "expected self-reference trigger render, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn legendary_self_reference_uses_oracle_short_name_when_oracle_shortens() {
    // A comma-less legendary whose oracle shortens its own name ("Bramblewood")
    // should keep that short form on every self-reference, not just the trigger
    // that captured it — driven by the captured oracle surface, not the card name.
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bramblewood of the Deep Vale")
        .card_types(vec![CardType::Creature])
        .supertypes(vec![crate::types::Supertype::Legendary])
        .power_toughness(PowerToughness::fixed(0, 0))
        .parse_text(
            "When Bramblewood enters, draw a card.\nBramblewood's power and toughness are each equal to the number of Forests you control.",
        )
        .expect("self-referencing legendary should parse");

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("When Bramblewood enters"),
        "ETB should keep the oracle short name, got {rendered}"
    );
    assert!(
        rendered.contains("Bramblewood's power and toughness are each equal to"),
        "the static self-reference should reuse the oracle short name, got {rendered}"
    );
    assert!(
        !rendered.contains("Bramblewood of the Deep Vale's power"),
        "the static self-reference should not fall back to the full card name, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_bolster_trigger_without_fallback_marker() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bolster Trigger Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, bolster 2. (Choose a creature with the least toughness among creatures you control and put two +1/+1 counters on it.)",
        )
        .expect("bolster trigger should parse as an explicit mechanic effect");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("bolster 2"),
        "expected bolster text in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "bolster trigger should not rely on unsupported fallback marker: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_bolster_spell_clause_without_fallback_marker() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bolster Spell Probe")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Target player sacrifices an enchantment of their choice. Bolster 1. (Choose a creature with the least toughness among creatures you control and put a +1/+1 counter on it.)",
        )
        .expect("bolster spell clause should parse as an explicit mechanic effect");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("bolster 1"),
        "expected bolster text in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "bolster spell clause should not rely on unsupported fallback marker: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_support_trigger_without_fallback_marker() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Support Trigger Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, support 3. (Put a +1/+1 counter on each of up to three other target creatures.)",
        )
        .expect("support trigger should parse as an explicit mechanic effect");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("support 3"),
        "expected support text in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "support trigger should not rely on unsupported fallback marker: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_support_spell_clause_without_fallback_marker() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Support Spell Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Support 2. (Put a +1/+1 counter on each of up to two target creatures.) Draw a card.",
        )
        .expect("support spell clause should parse as an explicit mechanic effect");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("support 2"),
        "expected support text in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "support spell clause should not rely on unsupported fallback marker: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_counter_target_activated_or_triggered_ability_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Stifle Probe")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Counter target activated or triggered ability. (Mana abilities can't be targeted.)",
        )
        .expect("counter activated/triggered ability clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("counter target activated ability")
            && rendered.contains("triggered ability"),
        "expected counter-ability text in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "counter-ability clause should not rely on unsupported fallback marker: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_counter_target_spell_activated_or_triggered_ability_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Disallow Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Counter target spell, activated ability, or triggered ability.")
        .expect("counter spell-or-ability clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("counter target spell")
            && rendered.contains("activated ability")
            && rendered.contains("triggered ability"),
        "expected counter spell-or-ability text in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "counter spell-or-ability clause should not rely on unsupported fallback marker: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_counter_target_activated_ability_from_artifact_source_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Rust Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Counter target activated ability from an artifact source.")
        .expect("counter activated-ability from artifact source clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("counter target"),
        "expected counter text in oracle-like output, got {rendered}"
    );
    assert!(
        rendered.contains("artifact"),
        "expected artifact source constraint in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "counter activated-ability from artifact source should not rely on unsupported fallback marker: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_ouphe_vandals_preserves_type_line_and_artifact_source_target() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ouphe Vandals")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Ouphe, crate::types::Subtype::Rogue])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "{G}, Sacrifice this creature: Counter target activated ability from an artifact source and destroy that artifact if it's on the battlefield. (Mana abilities can't be targeted.)",
        )
        .expect("Ouphe Vandals should parse");

    assert!(
        def.card.subtypes.contains(&crate::types::Subtype::Ouphe),
        "expected Ouphe subtype to survive type-line parsing, got {:?}",
        def.card.subtypes
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("counter target activated ability from an artifact source"),
        "expected activated-ability-from-artifact-source wording in oracle-like output, got {rendered}"
    );
    assert!(
        rendered.contains("destroy that artifact if it")
            || rendered.contains("if it matches permanent, destroy that artifact"),
        "expected battlefield-gated destroy clause in oracle-like output, got {rendered}\n{def:#?}"
    );
    assert!(
        !rendered.contains("counter target artifact spell"),
        "expected oracle-like output to avoid collapsing Ouphe Vandals to artifact spell wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_counter_target_ability_or_legendary_spell_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Tales End Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Counter target activated ability, triggered ability, or legendary spell.")
        .expect("counter activated/triggered ability or legendary spell clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("legendary spell"),
        "expected legendary spell selector in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "counter activated/triggered ability or legendary spell should not rely on unsupported fallback marker: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_counter_target_ability_or_noncreature_spell_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Louisoix Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Counter target activated ability, triggered ability, or noncreature spell.")
        .expect("counter activated/triggered ability or noncreature spell clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("noncreature") && rendered.contains("spell"),
        "expected noncreature spell selector in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "counter activated/triggered ability or noncreature spell should not rely on unsupported fallback marker: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_counter_target_triggered_ability_or_colorless_spell_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Colorless Stifle Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Counter target triggered ability or colorless spell.")
        .expect("counter triggered ability or colorless spell clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("triggered ability") && rendered.contains("colorless spell"),
        "expected triggered ability and colorless spell selector in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "counter triggered ability or colorless spell should not rely on unsupported fallback marker: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_counter_up_to_one_target_activated_or_triggered_ability_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Tidebinder Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Counter up to one target activated or triggered ability.")
        .expect("counter up-to-one activated/triggered ability clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("up to one target"),
        "expected up-to-one target selector in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "counter up-to-one activated/triggered ability should not rely on unsupported fallback marker: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_counter_target_ability_you_dont_control_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Obstructionist Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Counter target activated or triggered ability you don't control.")
        .expect("counter activated/triggered ability you don't control clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("you don't control") || rendered.contains("opponents"),
        "expected controller restriction in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "counter activated/triggered ability you don't control should not rely on unsupported fallback marker: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_counter_target_activated_ability_from_permanent_source_unless_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ayesha Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{T}: Counter target activated ability from an artifact, creature, enchantment, or land unless that ability's controller pays {W}.",
        )
        .expect("counter activated ability from permanent source unless clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("unless"),
        "expected unless payment clause in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "counter activated ability from permanent source unless should not rely on unsupported fallback marker: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_counter_target_instant_or_sorcery_spell_or_ability_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sister Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, counter target instant spell, sorcery spell, activated ability, or triggered ability.",
        )
        .expect("counter instant/sorcery spell or activated/triggered ability clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("instant spell"),
        "expected instant-spell selector in oracle-like output, got {rendered}"
    );
    assert!(
        rendered.contains("sorcery spell"),
        "expected sorcery-spell selector in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "counter instant/sorcery spell or activated/triggered ability should not rely on unsupported fallback marker: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_prevent_all_damage_to_creatures_static_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bubble Matrix Probe")
        .card_types(vec![CardType::Artifact])
        .parse_text("Prevent all damage that would be dealt to creatures.")
        .expect("prevent-all damage to creatures clause should parse as static ability");

    let has_prevent_all_to_creatures = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::PreventAllDamageDealtToCreatures
        )
    });
    assert!(
        has_prevent_all_to_creatures,
        "expected PreventAllDamageDealtToCreatures static ability in parsed card"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_prevent_all_damage_duration_before_target_order_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sivvi Prevention Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Prevent all damage that would be dealt this turn to creatures you control.")
        .expect("prevent-all damage clause with duration-before-target order should parse");

    let spell_debug = format!("{:?}", def.spell_effect);
    assert!(
        spell_debug.contains("PreventAllDamageEffect")
            && spell_debug.contains("PermanentsMatching"),
        "expected non-targeted prevent-all-damage effect in parsed spell text, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_prevent_all_damage_to_explicit_target_stays_targeted() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Targeted Prevention Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Prevent all damage that would be dealt this turn to target creature.")
        .expect("targeted prevent-all damage clause should parse");

    let spell_debug = format!("{:#?}", def.spell_effect);
    assert!(
        spell_debug.contains("PreventAllDamageToTarget"),
        "expected targeted prevent-all-damage effect in parsed spell text, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_prevent_all_damage_from_non_human_sources_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Repel Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Prevent all damage that would be dealt this turn by non-Human sources.")
        .expect("prevent-all damage from source filter clause should parse");

    let spell_debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        spell_debug.contains("preventalldamageeffect")
            && spell_debug.contains("from_source")
            && spell_debug.contains("excluded_subtypes")
            && spell_debug.contains("human"),
        "expected non-Human source-filter prevent-all-damage effect, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_prevent_all_damage_from_opponents_creatures_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Thwart Probe")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Prevent all damage that would be dealt this turn by creatures your opponents control.",
        )
        .expect("prevent-all damage from opponent-controlled creatures clause should parse");

    let spell_debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        spell_debug.contains("preventalldamageeffect")
            && spell_debug.contains("from_source")
            && spell_debug.contains("card_types")
            && spell_debug.contains("creature")
            && spell_debug.contains("opponent"),
        "expected source-filter prevent-all-damage effect for opponent-controlled creatures, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_cant_be_blocked_as_long_as_defending_player_controls_artifact_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bouncing Beebles Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature can't be blocked as long as defending player controls an artifact.",
        )
        .expect("defending-player artifact unblockable clause should parse");

    let has_conditional_unblockable = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id()
                    == StaticAbilityId::CantBeBlockedAsLongAsDefendingPlayerControlsCardType
        )
    });
    assert!(
        has_conditional_unblockable,
        "expected defending-player artifact unblockable static ability in parsed card"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_cant_be_blocked_as_long_as_defending_player_controls_artifact_land_clause()
{
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Tanglewalker Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature can't be blocked as long as defending player controls an artifact land.",
        )
        .expect("defending-player artifact-land unblockable clause should parse");

    let has_multi_type_conditional_unblockable = def.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id()
                    == StaticAbilityId::CantBeBlockedAsLongAsDefendingPlayerControlsCardTypes
        )
    });
    assert!(
        has_multi_type_conditional_unblockable,
        "expected defending-player artifact-land unblockable static ability in parsed card"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_add_any_type_that_land_produced_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Heartbeat Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever a player taps a land for mana, that player adds one mana of any type that land produced.",
        )
        .expect("land-produced mana clause should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert_eq!(
        rendered,
        "Whenever a player taps a land for mana, that player adds one mana of any type that land produced."
    );

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("TapForManaTrigger")
            && debug.contains("TriggeringEventProduced")
            && debug.contains("player: IteratedPlayer"),
        "expected actual-event mana semantics, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_tap_multiple_land_types_for_actual_mana_uses_oracle_list_surface() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Keeper Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever a player taps a Mountain, Forest, or Plains for mana, that player adds one mana of any type that land produced.",
        )
        .expect("multi-subtype actual-mana trigger should parse");

    assert_eq!(
        unprocessed_compiled_lines(&def).join(" "),
        "Whenever a player taps a Mountain, Forest, or Plains for mana, that player adds one mana of any type that land produced."
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_raid_conditional_with_attacked_this_turn_without_fallback() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Raid Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Raid — When this creature enters, if you attacked this turn, this creature deals 2 damage to any target.",
        )
        .expect("raid conditional should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("if you attacked this turn"),
        "expected attacked-this-turn predicate in rendered text, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "raid conditional should not rely on unsupported fallback marker: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) const HOWL_OF_THE_HORDE_TEXT: &str = "When you next cast an instant or sorcery spell this turn, copy that spell. You may choose new targets for the copy.\nRaid — If you attacked this turn, when you next cast an instant or sorcery spell this turn, copy that spell an additional time. You may choose new targets for the copy.";

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn howl_of_the_horde_parses_as_spell_effect_delayed_triggers() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Howl of the Horde")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(HOWL_OF_THE_HORDE_TEXT)
        .expect("Howl of the Horde should parse strictly");

    assert!(
        def.abilities.is_empty(),
        "Howl's next-cast clauses should be spell effects, not battlefield triggers: {:?}",
        def.abilities
    );
    let spell_debug = format!("{:#?}", def.spell_effect.as_ref().expect("spell effects"));
    assert!(
        spell_debug.matches("ScheduleDelayedTriggerEffect").count() >= 2
            && spell_debug.contains("one_shot: true")
            && spell_debug.contains("until_end_of_turn: true")
            && spell_debug.contains("AttackedThisTurn")
            && spell_debug.contains("CopySpellEffect")
            && spell_debug.contains("RetargetStackObjectEffect"),
        "expected unconditional and raid-gated one-shot delayed copy triggers, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn howl_of_the_horde_compiled_text_preserves_raid_next_cast_copy_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Howl of the Horde")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(HOWL_OF_THE_HORDE_TEXT)
        .expect("Howl of the Horde should parse strictly");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("When you next cast an instant or sorcery spell this turn")
            && rendered.contains("Raid — If you attacked this turn")
            && rendered.matches("copy it").count() >= 2
            && rendered
                .matches("you may choose new targets for the copy")
                .count()
                >= 2,
        "expected Howl's next-cast raid copy text to render structurally, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) const REPEATED_REVERBERATION_TEXT: &str = "When you next cast an instant spell, cast a sorcery spell, or activate a loyalty ability this turn, copy that spell or ability twice. You may choose new targets for the copies.";

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn repeated_reverberation_parses_as_spell_effect_delayed_spell_or_loyalty_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Repeated Reverberation")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(REPEATED_REVERBERATION_TEXT)
        .expect("Repeated Reverberation should parse strictly");

    assert!(
        def.abilities.is_empty(),
        "Repeated's next-stack-object clause should be a spell effect, not a battlefield trigger: {:?}",
        def.abilities
    );
    let spell_debug = format!("{:#?}", def.spell_effect.as_ref().expect("spell effects"));
    assert!(
        spell_debug.contains("ScheduleDelayedTriggerEffect")
            && spell_debug.contains("one_shot: true")
            && spell_debug.contains("until_end_of_turn: true")
            && spell_debug.contains("OrTrigger")
            && spell_debug.contains("AbilityActivatedTrigger")
            && spell_debug.contains("CopySpellEffect")
            && spell_debug.contains("RetargetStackObjectEffect"),
        "expected one-shot delayed spell-or-loyalty copy trigger, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn repeated_reverberation_compiled_text_preserves_next_spell_or_loyalty_copy_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Repeated Reverberation")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(REPEATED_REVERBERATION_TEXT)
        .expect("Repeated Reverberation should parse strictly");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "When you next cast an instant spell, cast a sorcery spell, or activate a loyalty ability this turn, copy that spell or ability twice"
        ) && rendered.contains("You may choose new targets for the copies"),
        "expected Repeated's delayed spell-or-loyalty copy text to render structurally, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_x_target_lands_clause_without_fallback() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "X Untap Probe")
        .card_types(vec![CardType::Artifact])
        .parse_text("{X}, {T}: Untap X target lands.")
        .expect("x-target untap clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("target lands"),
        "expected target-lands wording in rendered text, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "x-target untap clause should not rely on unsupported fallback marker: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_exile_cards_from_single_graveyard_without_fallback() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Single Graveyard Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Exile up to three target cards from a single graveyard.")
        .expect("single-graveyard exile clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("single graveyard"),
        "expected single-graveyard wording in rendered text, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "single-graveyard exile clause should not rely on unsupported fallback marker: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_each_of_them_gets_clause_targets_selected_objects() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Hope and Glory Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Untap two target creatures. Each of them gets +1/+1 until end of turn.")
        .expect("each-of-them gets clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("untap two target creatures"),
        "expected untap-target-creatures clause in rendered text, got {rendered}"
    );
    assert!(
        !rendered.contains("this spell gets +1/+1"),
        "selected creatures should be pumped, not the spell itself: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_return_cards_at_random_from_graveyard_to_hand() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Make a Wish Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Return two cards at random from your graveyard to your hand.")
        .expect("return-at-random clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("return two cards at random from your graveyard to your hand"),
        "expected random graveyard return wording in rendered text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_ignite_memories_keeps_random_hand_reveal_and_damage_link() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ignite Memories")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Target player reveals a card at random from their hand. Ignite Memories deals damage to that player equal to that card's mana value.",
        )
        .expect("Ignite Memories should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("ChooseObjectsEffect")
            && debug.contains("random: true")
            && debug.contains("zone: Some(Hand)")
            && debug.contains("RevealTaggedEffect")
            && debug.contains("DealDamageEffect")
            && debug.contains("ManaValueOf"),
        "expected random hand selection, reveal, and mana-value damage linkage, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("target player reveals a card at random from their hand")
            && rendered.contains("deal damage to that player equal to that card's mana value")
            && !rendered.contains("choose exactly 1 at random")
            && !rendered.contains("tags it as"),
        "expected Ignite Memories compiled text to use the cleaner random hand reveal wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_singe_mind_ogre_keeps_random_hand_reveal_and_life_loss_link() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Singe-Mind Ogre")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Ogre, Subtype::Mutant])
        .power_toughness(PowerToughness::fixed(3, 2))
        .parse_text(
            "When this creature enters, target player reveals a card at random from their hand, then loses life equal to that card's mana value.",
        )
        .expect("Singe-Mind Ogre should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ChooseObjectsEffect")
            && debug.contains("random: true")
            && debug.contains("zone: Some(Hand)")
            && debug.contains("RevealTaggedEffect")
            && debug.contains("LoseLifeEffect")
            && debug.contains("ManaValueOf"),
        "expected random hand selection, reveal, and mana-value life-loss linkage, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("when this creature enters")
            && rendered.contains("target player reveals a card at random from their hand")
            && rendered.contains("that player loses life equal to that card's mana value")
            && !rendered.contains("reveal it")
            && !rendered.contains("choose exactly 1 at random"),
        "expected Singe-Mind Ogre compiled text to preserve the random reveal and life-loss link, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_word_of_blasting_uses_destroyed_wall_mana_value_for_damage() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Word of Blasting")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Destroy target Wall. It can't be regenerated. Word of Blasting deals damage equal to that Wall's mana value to the Wall's controller.",
        )
        .expect("Word of Blasting should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("DestroyNoRegenerationEffect")
            && debug.contains("DealDamageEffect")
            && debug.contains("ManaValueOf"),
        "expected destroy/no-regeneration plus mana-value damage linkage, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("destroy target wall")
            && rendered.contains("it can't be regenerated")
            && rendered.contains(
                "word of blasting deals damage to that object's controller equal to its mana value"
            ),
        "expected Word of Blasting compiled text to preserve wall mana-value damage clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_scent_of_cinder_uses_source_damage_surface() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Scent of Cinder")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Reveal any number of red cards in your hand. Scent of Cinder deals X damage to any target, where X is the number of cards revealed this way.",
        )
        .expect("Scent of Cinder should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("reveal any number of red cards in your hand")
            && rendered.contains("scent of cinder deals x damage to any target")
            && rendered.contains("where x is the number of cards revealed this way"),
        "expected Scent of Cinder to keep its source-linked reveal-count damage text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_merfolk_spy_keeps_random_hand_reveal_surface() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Merfolk Spy")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Merfolk, Subtype::Rogue])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "Islandwalk (This creature can't be blocked as long as defending player controls an Island.)\nWhenever this creature deals combat damage to a player, that player reveals a card at random from their hand.",
        )
        .expect("Merfolk Spy should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("that player reveals a card at random from their hand"),
        "expected Merfolk Spy to render the random hand reveal surface cleanly, got {rendered}"
    );
    assert!(
        !rendered.contains("choose exactly 1 at random"),
        "expected Merfolk Spy to avoid the raw choose-and-tag surface, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_one_word_verb_card_name_does_not_break_clause_parsing() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Regenerate")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Regenerate target creature. (The next time that creature would be destroyed this turn, instead tap it, remove it from combat, and heal all damage on it.)",
        )
        .expect("verb-named card should still parse regenerate clause");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("regenerate target creature"),
        "expected regenerate clause in rendered text, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "verb-named card should not rely on unsupported fallback marker: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_debt_of_loyalty_regenerate_control_followup() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Debt of Loyalty")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::White],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Regenerate target creature. You gain control of that creature if it regenerates this way.",
        )
        .expect("Debt of Loyalty should parse strictly");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("regenerate target creature"),
        "expected Debt of Loyalty to render the regenerate clause, got {rendered}"
    );
    assert!(
        rendered_lower.contains("gain control of that creature if it regenerates this way"),
        "expected Debt of Loyalty to render the regenerate-this-way control clause, got {rendered}"
    );
    assert!(
        !rendered_lower.contains("unsupported parser line fallback"),
        "Debt of Loyalty should not rely on unsupported fallback marker: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_render_enters_with_single_counter_uses_singular_wording() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Single Counter Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature enters with a +1/+1 counter on it.")
        .expect("single-counter enters clause should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("enters with a +1/+1 counter on it"),
        "expected singular enters-with-counter wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_tayam_oracle_text_regression() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Tayam, Luminous Enigma")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Each other creature you control enters with an additional vigilance counter on it.\n\
             {3}, Remove three counters from among creatures you control: Mill three cards, then return a permanent card with mana value 3 or less from your graveyard to the battlefield.",
        )
        .expect("tayam oracle text should parse");

    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        static_ids.contains(&StaticAbilityId::EnterWithCountersForFilter),
        "expected ETB counter replacement static ability, got {static_ids:?}"
    );
    assert!(
        !static_ids.contains(&StaticAbilityId::RuleFallbackText),
        "tayam oracle text should not fall back to placeholder static ability: {static_ids:?}"
    );

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("expected activated ability");

    let cost_debug = format!("{:?}", activated.mana_cost);
    assert!(
        cost_debug.contains("CostEffect")
            && cost_debug.contains("RemoveAnyCountersAmongEffect")
            && cost_debug.contains("count: 3"),
        "expected effect-backed remove-three-counters-among cost, got {cost_debug}"
    );
    assert!(
        activated
            .mana_cost
            .non_mana_costs()
            .any(|cost| cost.effect_ref().is_some_and(|effect| effect
                .downcast_ref::<crate::effects::RemoveAnyCountersAmongEffect>()
                .is_some())),
        "expected Tayam activation to expose an effect-backed staged remove-counters-among cost"
    );

    let effects_debug = format!("{:?}", activated.effects);
    assert!(
        effects_debug.contains("MillEffect"),
        "expected mill effect in tayam activated ability, got {effects_debug}"
    );
    assert!(
        effects_debug.contains("ChooseObjectsEffect"),
        "expected runtime graveyard choice in tayam activated ability, got {effects_debug}"
    );
    assert!(
        effects_debug.contains("ReturnFromGraveyardToBattlefieldEffect"),
        "expected return-from-graveyard effect in tayam activated ability, got {effects_debug}"
    );
    assert!(
        effects_debug.contains("mana_value: Some(")
            && effects_debug.contains("LessThanOrEqual")
            && effects_debug.contains("Artifact")
            && effects_debug.contains("Creature")
            && effects_debug.contains("Enchantment")
            && effects_debug.contains("Land")
            && effects_debug.contains("Planeswalker"),
        "expected permanent-card mana-value<=3 filter in return effect, got {effects_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_enters_with_counter_if_you_attacked_this_turn_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Goblin Boarders Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature enters with a +1/+1 counter on it if you attacked this turn.")
        .expect("raid enters-with-counter clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCountersIfCondition),
        "expected conditional enters-with-counters ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleFallbackText),
        "raid enters-with-counter should not fall back to placeholder static ability: {ids:?}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("if you attacked this turn"),
        "expected raid condition text in rendered output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_enters_with_x_plus_one_counters_line_is_typed_static() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Endless One Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature enters with X +1/+1 counters on it.")
        .expect("x enters-with-counters clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCounters),
        "expected typed enters-with-counters static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleFallbackText),
        "x enters-with-counters should not fall back to placeholder static ability: {ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_enters_with_counter_if_opponent_lost_life_is_typed_static() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Frilled Sparkshooter Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with a +1/+1 counter on it if an opponent lost life this turn.",
        )
        .expect("opponent-life-loss conditional enters-with-counters should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCountersIfCondition),
        "expected conditional enters-with-counters ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleFallbackText),
        "opponent-life-loss conditional variant should not use placeholder fallback: {ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_enters_with_counter_if_creature_died_this_turn_is_typed_static() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Moldering Reclaimer Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with two +1/+1 counters on it if a creature died this turn.",
        )
        .expect("creature-died conditional enters-with-counters should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCountersIfCondition),
        "expected conditional enters-with-counters ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleFallbackText),
        "creature-died conditional variant should not use placeholder fallback: {ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_enters_with_counter_if_permanent_left_under_your_control_is_typed_static() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Fountainport Charmer Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with two +1/+1 counters on it if a permanent left the battlefield under your control this turn.",
        )
        .expect("permanent-left conditional enters-with-counters should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCountersIfCondition),
        "expected conditional enters-with-counters ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleFallbackText),
        "permanent-left conditional variant should not use placeholder fallback: {ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_enters_with_counter_for_each_creature_that_died_this_turn_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bloodcrazed Paladin Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature enters with a +1/+1 counter on it for each creature that died this turn.")
        .expect("for-each-creature-died enters-with-counter clause should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("CreaturesDiedThisTurn")
            || debug.contains("for each creature that died this turn"),
        "expected creatures-died-this-turn value in static ability, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_enters_with_counter_for_each_color_of_mana_spent_to_cast_it_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Springmantle Cleric Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with a +1/+1 counter on it for each color of mana spent to cast it.",
        )
        .expect("spent-to-cast enters-with-counter clause should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ColorsOfManaSpentToCastThisSpell")
            || debug.contains("for each color of mana spent to cast it"),
        "expected spent-to-cast color value in static ability, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_enters_with_counter_for_each_time_it_was_kicked_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Apex Hawks Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature enters with a +1/+1 counter on it for each time it was kicked.")
        .expect("for-each-time-kicked enters-with-counter clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCounters),
        "expected typed enters-with-counters static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleFallbackText),
        "for-each-time-kicked variant should not use placeholder fallback: {ids:?}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("KickCount"),
        "expected kick-count value in static ability, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_create_token_for_each_time_it_regenerated_this_turn_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Spiny Starfish Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "At the beginning of each end step, if this creature regenerated this turn, create a 0/1 blue Starfish creature token for each time it regenerated this turn.",
        )
        .expect("for-each-regenerated-this-turn token clause should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        !debug.contains("RuleFallbackText"),
        "for-each-regenerated-this-turn variant should not use placeholder fallback: {debug}"
    );
    assert!(
        debug.contains("SourceRegeneratedThisTurnCount"),
        "expected regenerated-this-turn value in triggered ability, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_enters_with_counter_for_each_creature_card_in_your_graveyard_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Golgari Raiders Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with a +1/+1 counter on it for each creature card in your graveyard.",
        )
        .expect("for-each-creature-card-in-graveyard enters-with-counter clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCounters),
        "expected typed enters-with-counters static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleFallbackText),
        "graveyard-count enters-with-counter variant should not use placeholder fallback: {ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_enters_with_counter_equal_to_number_of_creature_cards_in_your_graveyard_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Rhizome Lurcher Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with a number of +1/+1 counters on it equal to the number of creature cards in your graveyard.",
        )
        .expect("equal-to-number-of-creature-cards enters-with-counter clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCounters),
        "expected typed enters-with-counters static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleFallbackText),
        "equal-to-count enters-with-counter variant should not use placeholder fallback: {ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_enters_with_counter_if_you_control_creature_with_power_four_or_greater_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Frontier Mastodon Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with a +1/+1 counter on it if you control a creature with power 4 or greater.",
        )
        .expect("control-power conditional enters-with-counter clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCountersIfCondition),
        "expected conditional enters-with-counters ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleFallbackText),
        "control-power conditional variant should not use placeholder fallback: {ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_enters_with_counter_for_each_other_creature_and_or_artifact_you_control_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Luxknight Breacher Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with a +1/+1 counter on it for each other creature and/or artifact you control.",
        )
        .expect("for-each-other-creature-and-or-artifact enters-with-counter clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCounters),
        "expected typed enters-with-counters static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleFallbackText),
        "for-each-other-creature-and-or-artifact variant should not use placeholder fallback: {ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_enters_with_counter_if_x_is_five_or_more_additional_x_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Apocalypse Hydra Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with X +1/+1 counters on it. If X is 5 or more, it enters with an additional X +1/+1 counters on it.",
        )
        .expect("x-threshold additional enters-with-counters clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCounters),
        "expected baseline enters-with-counters static ability, got {ids:?}"
    );
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCountersIfCondition),
        "expected conditional additional enters-with-counters static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleFallbackText),
        "x-threshold additional enters-with-counters variant should not use placeholder fallback: {ids:?}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("XValueAtLeast(\n                                                            5,\n                                                        )")
            || debug.contains("XValueAtLeast(5)"),
        "expected X-threshold condition in static ability, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_enters_with_counter_where_x_is_total_life_lost_by_opponents_this_turn_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cryptborn Horror Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with X +1/+1 counters on it, where X is the total life lost by your opponents this turn.",
        )
        .expect("where-x-total-life-lost enters-with-counters clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCounters),
        "expected typed enters-with-counters static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleFallbackText),
        "where-x-total-life-lost variant should not use placeholder fallback: {ids:?}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("LifeLostThisTurn(\n                                                            Opponent,\n                                                        )")
            || debug.contains("LifeLostThisTurn(Opponent)"),
        "expected life-lost-this-turn value in static ability, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_each_opponent_loses_life_equal_to_that_players_life_lost_this_turn() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Archfiend Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "At the beginning of each end step, each opponent loses life equal to the life that player lost this turn.",
        )
        .expect("that-player-life-lost-this-turn amount should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("LifeLostThisTurn(\n                                                            IteratedPlayer,\n                                                        )")
            || debug.contains("LifeLostThisTurn(IteratedPlayer)"),
        "expected iterated-player life-lost-this-turn value, got {debug}"
    );
    assert!(
        debug.contains("ForPlayersEffect") && debug.contains("filter: Opponent"),
        "expected effect to iterate over opponents, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_tainted_sigil_strictly_parses_and_renders_total_life_lost() {
    assert_oracle_card_parses_strict("Tainted Sigil");

    let def = parse_oracle_card_definition("Tainted Sigil");
    let rendered_lines = canonical_compiled_lines(&def);
    assert_eq!(
        rendered_lines,
        vec![
            "{T}, Sacrifice this artifact: You gain life equal to the total life lost by all players this turn."
                .to_string(),
        ],
        "expected Tainted Sigil to render the all-players life-lost amount"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("LifeLostThisTurn") && debug.contains("Any"),
        "expected Tainted Sigil to structurally use all players' life lost this turn, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) struct DustAnimusNoopDecisionMaker;

#[cfg(ironsmith_runtime_parser_tests)]
impl crate::decision::DecisionMaker for DustAnimusNoopDecisionMaker {}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_dust_animus_strictly_parses_and_renders_conditional_counters() {
    assert_oracle_card_parses_strict("Dust Animus");

    let def = parse_oracle_card_definition("Dust Animus");
    let rendered = canonical_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("Flying"),
        "expected Dust Animus to render flying, got {rendered}"
    );
    assert!(
        rendered.contains(
            "If you control five or more untapped lands, this creature enters with two +1/+1 counters and a lifelink counter on it"
        ),
        "expected Dust Animus to render the combined conditional counter clause, got {rendered}"
    );
    assert!(
        rendered.contains("Plot {1}{W}"),
        "expected Dust Animus to keep plot text, got {rendered}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug
            .matches("StaticAbility(StaticAbilityModelInterpreter { id: Some(EnterWithCounters)")
            .count()
            >= 2
            && debug.contains("PlusOnePlusOne")
            && debug.contains("Lifelink")
            && debug.contains("untapped: true"),
        "expected Dust Animus to structurally model both conditional counters against untapped lands, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn dust_animus_enters_with_both_counters_when_untapped_land_condition_is_met() {
    let def = parse_oracle_card_definition("Dust Animus");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let mut dm = DustAnimusNoopDecisionMaker;

    for idx in 0..5 {
        let land = CardDefinitionBuilder::new(CardId::new(), format!("Untapped Land {idx}"))
            .card_types(vec![CardType::Land])
            .build();
        game.create_object_from_definition(&land, alice, Zone::Battlefield);
    }

    let old_id = game.create_object_from_definition(&def, alice, Zone::Hand);
    let animus_id = game
        .move_object_with_etb_processing_with_dm(old_id, Zone::Battlefield, &mut dm)
        .expect("Dust Animus should enter")
        .new_id;

    let animus = game.object(animus_id).expect("Dust Animus should exist");
    assert_eq!(
        animus
            .counters
            .get(&CounterType::PlusOnePlusOne)
            .copied()
            .unwrap_or_default(),
        2,
        "Dust Animus should enter with two +1/+1 counters when the land condition is met"
    );
    assert_eq!(
        animus
            .counters
            .get(&CounterType::Lifelink)
            .copied()
            .unwrap_or_default(),
        1,
        "Dust Animus should enter with a lifelink counter when the land condition is met"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn dust_animus_enters_without_counters_when_fewer_than_five_lands_are_untapped() {
    let def = parse_oracle_card_definition("Dust Animus");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let mut dm = DustAnimusNoopDecisionMaker;

    for idx in 0..5 {
        let land = CardDefinitionBuilder::new(CardId::new(), format!("Untapped Land {idx}"))
            .card_types(vec![CardType::Land])
            .build();
        let land_id = game.create_object_from_definition(&land, alice, Zone::Battlefield);
        if idx == 4 {
            game.tap(land_id);
        }
    }

    let old_id = game.create_object_from_definition(&def, alice, Zone::Hand);
    let animus_id = game
        .move_object_with_etb_processing_with_dm(old_id, Zone::Battlefield, &mut dm)
        .expect("Dust Animus should enter")
        .new_id;

    let animus = game.object(animus_id).expect("Dust Animus should exist");
    assert_eq!(
        animus
            .counters
            .get(&CounterType::PlusOnePlusOne)
            .copied()
            .unwrap_or_default(),
        0,
        "Dust Animus should not get +1/+1 counters with only four untapped lands"
    );
    assert_eq!(
        animus
            .counters
            .get(&CounterType::Lifelink)
            .copied()
            .unwrap_or_default(),
        0,
        "Dust Animus should not get a lifelink counter with only four untapped lands"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oracle_final_punishment_strictly_parses_and_renders_damage_dealt_this_turn() {
    assert_oracle_card_parses_strict("Final Punishment");

    let def = parse_oracle_card_definition("Final Punishment");
    let rendered_lines = canonical_compiled_lines(&def);
    assert_eq!(
        rendered_lines,
        vec![
            "Target player loses life equal to the damage already dealt to that player this turn."
                .to_string(),
        ],
        "expected Final Punishment to render its target player's prior damage amount"
    );

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("DamageDealtToPlayersThisTurn") && debug.contains("Target"),
        "expected Final Punishment to structurally use damage dealt to the target player this turn, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_one_or_more_etb_trigger_binds_that_much_to_zone_change_count() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Artillerist Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever one or more artifacts you control enter, this creature deals that much damage to each opponent.",
        )
        .expect("one-or-more ETB that-much amount should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("count_mode: OneOrMore"),
        "expected one-or-more enters trigger, got {debug}"
    );
    assert!(
        debug.contains("EventValue(\n                                                                    Amount,\n                                                                )")
            || debug.contains("EventValue(Amount)"),
        "expected event-derived amount to remain bound to the trigger, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_one_or_more_dies_trigger_uses_batch_count_mode() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Townsfolk Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever one or more other creatures you control die, put a +1/+1 counter on this creature.",
        )
        .expect("one-or-more dies trigger should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("count_mode: OneOrMore"),
        "expected one-or-more dies trigger, got {debug}"
    );
    assert!(
        debug.contains("other: true"),
        "expected other-creature filter to be preserved, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_implicit_choose_creature_does_not_default_to_you_control() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Duneblast Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Choose up to one creature. Destroy the rest.")
        .expect("implicit choose creature should parse");

    let effects = def
        .spell_effect
        .as_ref()
        .expect("spell should have effects")
        .all_effects();
    let choose = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ChooseObjectsEffect>())
        .expect("expected choose-object effect");
    assert_eq!(choose.count.min, 0);
    assert_eq!(choose.count.max, Some(1));
    assert_eq!(
        choose.filter.controller, None,
        "implicit choose should allow any controller's creature"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_sacrificed_permanents_card_type_condition() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Baba Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{T}, Sacrifice up to three permanents: If there were three or more card types among sacrificed permanents, each opponent loses 3 life and you gain 3 life.",
        )
        .expect("sacrificed-permanents card-type condition should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("CardTypesAmong") && debug.contains("sacrificed_0"),
        "expected condition to count card types among sacrificed permanents, got {debug}"
    );
    assert!(
        debug.contains("min: 0") && debug.contains("max: Some(3)"),
        "expected sacrifice cost to choose up to three permanents, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_sacrifice_any_number_then_draw_that_many_uses_sacrifice_result_count() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Witchkite Variant")
        .parse_text(
            "Sacrifice any number of artifacts, enchantments, and/or tokens, then draw that many cards.",
        )
        .expect("sacrifice-any-number then draw-that-many should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("ChoiceCount") && debug.contains("max: None"),
        "expected any-number choice before sacrifice, got {debug}"
    );
    assert!(
        debug.contains("Artifact")
            && debug.contains("Enchantment")
            && debug.contains("token: true"),
        "expected sacrifice choice to include artifacts, enchantments, and tokens, got {debug}"
    );
    assert!(
        debug.contains("EffectValue"),
        "expected draw count to reference sacrifice result count, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_sacrifice_any_number_then_return_that_many_uses_sacrifice_result_count() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Lich Variant")
        .parse_text(
            "Sacrifice any number of artifacts, enchantments, and/or tokens. Return that many creature cards from your graveyard to the battlefield.",
        )
        .expect("sacrifice-any-number then return-that-many should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("any_of")
            && debug.contains("Artifact")
            && debug.contains("Enchantment")
            && debug.contains("token: true"),
        "expected sacrifice choice to include artifacts, enchantments, or tokens, got {debug}"
    );
    assert!(
        debug.contains("EffectValue") && debug.contains("ReturnFromGraveyardToBattlefieldEffect"),
        "expected return count to reference sacrifice result count, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "Sacrifice any number of artifacts or enchantments or tokens. Return that many creature cards from your graveyard to the battlefield"
        ),
        "expected oracle-like sacrifice/return rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_relic_amulet_remove_all_counters_cost_binds_pronoun_that_much_to_cost_x() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Relic Amulet")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "{2}, {T}, Remove all charge counters from this artifact: It deals that much damage to target creature.",
        )
        .expect("Relic Amulet's remove-all cost should bind pronoun that-much damage to cost X");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("remove_all: true"),
        "expected remove-all counter cost, got {debug}"
    );
    assert!(
        debug.contains("DealDamage")
            && (debug.contains("amount: X") || debug.contains("amount: X,")),
        "expected that-much damage to bind to cost X, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_enters_with_counter_unless_two_or_more_colors_of_mana_were_spent_to_cast_it_line()
 {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Steel Exemplar Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with two +1/+1 counters on it unless two or more colors of mana were spent to cast it.",
        )
        .expect("unless-colors-spent enters-with-counters clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCountersIfCondition),
        "expected conditional enters-with-counters static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleFallbackText),
        "unless-colors-spent variant should not use placeholder fallback: {ids:?}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ColorsOfManaSpentToCastThisSpellOrMore(\n                                                                    2,\n                                                                )")
            || debug.contains("ColorsOfManaSpentToCastThisSpellOrMore(2)"),
        "expected distinct-colors-spent condition in static ability, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_enters_with_counter_if_youve_cast_two_or_more_spells_this_turn_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Effortless Master Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with two +1/+1 counters on it if you've cast two or more spells this turn.",
        )
        .expect("cast-two-spells conditional enters-with-counters clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCountersIfCondition),
        "expected conditional enters-with-counters static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleFallbackText),
        "cast-two-spells conditional variant should not use placeholder fallback: {ids:?}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("PlayerCastSpellsThisTurnOrMore")
            && (debug.contains("count: 2")
                || debug.contains(
                    "count:\n                                                                    2"
                )),
        "expected spells-cast-this-turn threshold condition in static ability, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_enters_with_counter_equal_to_greatest_number_of_cards_an_opponent_has_drawn_this_turn_line()
 {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Thought Sponge Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with a number of +1/+1 counters on it equal to the greatest number of cards an opponent has drawn this turn.",
        )
        .expect("equal-to-greatest-cards-drawn enters-with-counters clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCounters),
        "expected typed enters-with-counters static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleFallbackText),
        "equal-to-greatest-cards-drawn variant should not use placeholder fallback: {ids:?}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("MaxCardsDrawnThisTurn(\n                                                            Opponent,\n                                                        )")
            || debug.contains("MaxCardsDrawnThisTurn(Opponent)"),
        "expected max-cards-drawn-this-turn value in static ability, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_enters_with_counter_plus_additional_for_each_other_creature_you_control_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sheriff of Safe Passage Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with a +1/+1 counter on it plus an additional +1/+1 counter on it for each other creature you control.",
        )
        .expect("plus-additional-for-each enters-with-counters clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCounters),
        "expected typed enters-with-counters static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleFallbackText),
        "plus-additional-for-each variant should not use placeholder fallback: {ids:?}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("Add(") && debug.contains("other: true"),
        "expected additive counter value with 'other creature' filter in static ability, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_enters_with_counter_for_each_magic_game_you_lost_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Gus Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with a +1/+1 counter on it for each Magic game you have lost to one of your opponents since you last won a game against them.",
        )
        .expect("match-history enters-with-counters clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCounters),
        "expected typed enters-with-counters static ability, got {ids:?}"
    );
    assert!(
        !ids.contains(&crate::static_abilities::StaticAbilityId::RuleFallbackText),
        "match-history variant should not use placeholder fallback: {ids:?}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("MagicGamesLostToOpponentsSinceLastWin"),
        "expected dedicated match-history counter value in static ability, got {debug}"
    );
}

#[cfg(ironsmith_runtime_removed_parser_helper_unit_tests)]
#[test]
pub(super) fn parse_enters_with_counter_additional_x_if_threshold_direct_clause() {
    let tokens = tokenize_line(
        "it enters with an additional x +1/+1 counters on it if x is 5 or more",
        0,
    );
    let parsed = crate::cards::builders::parse_enters_with_counters_line(&tokens)
        .expect("direct additional-x clause should not error")
        .expect("direct additional-x clause should parse as a static ability");

    assert_eq!(
        parsed.id(),
        crate::static_abilities::StaticAbilityId::EnterWithCountersIfCondition,
        "expected direct additional-x clause to compile to conditional enters-with-counters"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_as_this_land_enters_reveal_if_you_dont_enters_tapped_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Secluded Glen Variant")
        .card_types(vec![CardType::Land])
        .parse_text(
            "As this land enters, you may reveal a Faerie card from your hand. If you don't, this land enters tapped.",
        )
        .expect("reveal-if-you-dont land ETB clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&StaticAbilityId::EntersTappedUnlessCondition),
        "expected generic enters-tapped-unless replacement, got {ids:?}"
    );
    assert!(
        !ids.contains(&StaticAbilityId::RuleFallbackText),
        "reveal-if-you-dont clause should not emit placeholder static ability: {ids:?}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("YouHaveCardInHandMatching"),
        "expected hand-match condition in replacement ability, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_as_this_land_enters_reveal_unless_revealed_or_control_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Temple of the Dragon Queen Variant")
        .card_types(vec![CardType::Land])
        .parse_text(
            "As this land enters, you may reveal a Dragon card from your hand. This land enters tapped unless you revealed a Dragon card this way or you control a Dragon.",
        )
        .expect("reveal-unless-or-control land ETB clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&StaticAbilityId::EntersTappedUnlessCondition),
        "expected generic enters-tapped-unless replacement, got {ids:?}"
    );
    assert!(
        !ids.contains(&StaticAbilityId::RuleFallbackText),
        "reveal-unless-or-control clause should not emit placeholder static ability: {ids:?}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("YouHaveCardInHandMatching") && debug.contains("YouControl"),
        "expected OR condition combining hand-match and you-control checks, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_as_this_creature_enters_reveal_cards_counted_for_counters_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Arsenal Thresher Variant")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .parse_text(
            "As this creature enters, you may reveal any number of other artifact cards from your hand. This creature enters with a +1/+1 counter on it for each card revealed this way.",
        )
        .expect("as-enters reveal counted by counters should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&StaticAbilityId::AsEntersEffectProgram)
            && ids.contains(&StaticAbilityId::EnterWithCounters),
        "expected reveal-as-enters plus enters-with-counters static abilities, got {ids:?}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    let def_debug = format!("{def:#?}");
    assert!(
        rendered_lower.contains(
            "as this creature enters, you may reveal any number of other artifact cards from your hand"
        ) && rendered_lower.contains(
            "this creature enters with a +1/+1 counter on it for each card revealed this way"
        ),
        "expected Arsenal-style static reveal/counter wording, got {rendered}\n{def_debug}"
    );

    let program = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => {
                static_ability
                    .compiled_model()
                    .and_then(|model| match &model.payload {
                        ironsmith_core::StaticAbilityPayload::AsEntersEffectProgram {
                            program,
                            ..
                        } => Some(program),
                        _ => None,
                    })
            }
            _ => None,
        })
        .expect("expected typed as-enters reveal program");
    let may = program.segments[0].default_effects[0]
        .downcast_ref::<crate::effects::MayEffect>()
        .expect("as-enters reveal should remain optional");
    let choose = may.effects[0]
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()
        .expect("as-enters reveal should choose cards from hand");
    let reveal = may.effects[1]
        .downcast_ref::<crate::effects::RevealTaggedEffect>()
        .expect("as-enters reveal should reveal the chosen cards");
    assert_eq!(choose.filter.zone, Some(Zone::Hand));
    assert_eq!(choose.filter.owner, Some(PlayerFilter::You));
    assert!(choose.filter.other);
    assert_eq!(choose.filter.card_types, vec![CardType::Artifact]);
    assert_eq!(choose.count, ChoiceCount::any_number());
    assert_eq!(reveal.tag, choose.tag);

    let debug = format!("{def:?}");
    assert!(
        debug.contains("__public_revealed"),
        "the enters-with-counters value should count cards revealed by the as-enters program: {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_starting_town_first_three_turns_etb_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Starting Town")
        .card_types(vec![CardType::Land])
        .parse_text(
            "This land enters tapped unless it's your first, second, or third turn of the game.\n{T}: Add {R}.",
        )
        .expect("Starting Town ETB clause should parse");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&StaticAbilityId::EntersTappedUnlessCondition),
        "expected generic enters-tapped-unless replacement, got {ids:?}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("YourFirstTurnsOfTheGameOrFewer(3)"),
        "expected first-three-turns condition in replacement ability, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_render_sacrifice_unless_you_pay_uses_pay_verb() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Conversion Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "At the beginning of your upkeep, sacrifice this enchantment unless you pay {W}{W}.",
        )
        .expect("sacrifice-unless-pay upkeep clause should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("unless you pay {W}{W}"),
        "expected 'you pay' wording in rendered text, got {rendered}"
    );
    assert!(
        !rendered.contains("you pays"),
        "renderer should never emit 'you pays', got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_render_leading_unless_payment_clause_keeps_unless_structure() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Demonic Hordes Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "At the beginning of your upkeep, unless you pay {B}{B}{B}, tap this creature and sacrifice a land.",
        )
        .expect("leading-unless upkeep clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("unless you pay {b}{b}{b}"),
        "expected unless-payment wording, got {rendered}"
    );
    assert!(
        rendered.contains("tap this creature"),
        "expected tap effect in unless branch, got {rendered}"
    );
    assert!(
        rendered.contains("sacrifice a land"),
        "expected sacrifice effect in unless branch, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_rhystic_study_unless_that_player_does_not_flip_to_you() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Rhystic Study")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever an opponent casts a spell, you may draw a card unless that player pays {1}.",
        )
        .expect("rhystic-study style unless-payment clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        !rendered.contains("unless you pay {1}"),
        "payer should not collapse to you in rhystic-style trigger, got {rendered}"
    );
    assert!(
        rendered.contains("unless that player pays {1}")
            || rendered.contains("unless they pay {1}")
            || rendered.contains("unless an opponent pays {1}"),
        "expected non-you payer in rhystic-style trigger, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_creatures_without_flying_cant_attack_static_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Moat Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Creatures without flying can't attack.")
        .expect("creatures-without-flying restriction should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("creatures without flying can't attack"),
        "expected static restriction text in oracle-like output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_this_creature_cant_attack_alone_static_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bonded Construct Probe")
        .card_types(vec![CardType::Artifact])
        .parse_text("This creature can't attack alone.")
        .expect("cant-attack-alone restriction should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("this creature can't attack alone"),
        "expected cant-attack-alone text in oracle-like output, got {rendered}"
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
        static_ids.contains(&StaticAbilityId::RuleRestriction),
        "expected rule-restriction static ability id, got {static_ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_this_token_cant_attack_or_block_alone_static_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Token Restriction Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, create a 4/4 white Beast creature token with \"This token can't attack or block alone.\"",
        )
        .expect("token cant-attack-or-block-alone restriction should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("this token can't attack or block alone"),
        "expected token cant-attack-or-block-alone text in oracle-like output, got {rendered}"
    );

    assert!(
        rendered.contains("can't attack or block alone"),
        "expected token self-restriction text in render output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_explicit_token_controller_keeps_adjacent_quoted_rules_intrinsic() {
    let oracle = "Whenever this creature deals combat damage to a player, that player creates a 0/1 colorless Goblin Construct artifact creature token with \"This token can't block\" and \"At the beginning of your upkeep, this token deals 1 damage to you.\"";
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Relic Robber Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(oracle)
        .expect("combat-damage token creation should parse");

    assert_eq!(canonical_compiled_lines(&def), vec![oracle]);
    let debug = format!("{:#?}", def.abilities);
    assert_eq!(
        debug.matches("DealDamageEffect").count(),
        1,
        "the upkeep damage must exist only inside the generated token's triggered ability: {debug}"
    );
    assert!(debug.contains("CantBlock"), "{debug}");
    assert!(debug.contains("BeginningOfUpkeepTrigger"), "{debug}");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_activated_abilities_of_artifacts_cant_be_activated_static_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Collector Ouphe Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Activated abilities of artifacts can't be activated.")
        .expect("activated-abilities-of restriction should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("activated abilities of artifacts can't be activated"),
        "expected activated-abilities-of restriction text, got {rendered}"
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
        static_ids.contains(&StaticAbilityId::RuleRestriction),
        "expected rule-restriction static ability id, got {static_ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_activated_abilities_of_artifacts_and_creatures_unless_mana_static_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Damping Matrix Probe")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Activated abilities of artifacts and creatures can't be activated unless they're mana abilities.",
        )
        .expect("matrix-style restriction should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("activated abilities of artifacts and creatures can't be activated unless they're mana abilities"),
        "expected matrix-style restriction text, got {rendered}"
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
        static_ids.contains(&StaticAbilityId::RuleRestriction),
        "expected rule-restriction static ability id, got {static_ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_lands_dont_untap_during_controllers_steps_static_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Rising Waters Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Lands don't untap during their controllers' untap steps.")
        .expect("lands-dont-untap restriction should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("lands don't untap during their controllers' untap steps"),
        "expected lands-dont-untap text in oracle-like output, got {rendered}"
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
        static_ids.contains(&StaticAbilityId::RuleRestriction),
        "expected rule-restriction static ability id, got {static_ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_flying_only_restriction_does_not_widen_to_reach() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Treetop Restriction Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't be blocked except by creatures with flying.")
        .expect("flying-only block restriction should parse");

    let static_ids = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        static_ids.contains(&crate::static_abilities::StaticAbilityId::FlyingOnlyRestriction),
        "expected flying-only restriction id, got {static_ids:?}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("except by creatures with flying"),
        "expected flying-only text in render, got {rendered}"
    );
    assert!(
        !rendered.contains("flying or reach"),
        "flying-only restriction must not widen to reach, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn ox_drover_parses_oxen_block_restriction_and_trigger_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ox Drover")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Peasant])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "Vigilance\n\
             This creature can't be blocked by Oxen.\n\
             Whenever this creature enters or attacks, target opponent creates a 2/4 white Ox creature token and you draw a card.",
        )
        .expect("Ox Drover should parse strictly");

    let rendered = unprocessed_compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("vigilance"),
        "expected vigilance in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains("can't be blocked by oxen"),
        "expected Oxen block restriction in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains("target opponent creates a 2/4 white ox creature token")
            && rendered.contains("draw a card"),
        "expected token-and-draw trigger text in compiled text, got {rendered}"
    );

    let ability_debug = format!("{:?}", def.abilities);
    assert!(
        ability_debug.contains("subtypes: [Ox]"),
        "expected Oxen to lower to the Ox subtype in the block restriction, got {ability_debug}"
    );
    assert!(
        ability_debug.contains("CreateTokenEffect") && ability_debug.contains("DrawCardsEffect"),
        "expected Ox Drover trigger to create a token and draw a card, got {ability_debug}"
    );
}
