#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::shard_02::*;
use super::shard_03::*;
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
pub(super) fn test_parse_hideaway_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Hideaway Probe")
        .card_types(vec![CardType::Land])
        .parse_text("Hideaway 4")
        .expect("hideaway should lower to real ETB semantics");

    let debug = format!("{def:#?}");
    assert!(
        !debug.contains("KeywordFallbackText") && !debug.contains("unsupported"),
        "hideaway should avoid unsupported placeholders, got {debug}"
    );
    assert!(
        !debug.contains("EntersTapped") && !debug.contains("enters_tapped"),
        "hideaway no longer implies enters tapped, got {debug}"
    );
    assert!(
        debug.contains("LookAtTopCards"),
        "expected hideaway ETB trigger to look at top cards, got {debug}"
    );
    assert!(
        debug.contains("face_down: true"),
        "expected hideaway to exile the chosen card face down, got {debug}"
    );
    assert!(
        debug.contains("PutTaggedRemainderOnLibraryBottom")
            || debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "expected hideaway to put the rest on the bottom, got {debug}"
    );
    assert!(
        debug.contains("Random"),
        "expected hideaway to bottom the rest in random order, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_partner_with_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Partner Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Partner with Bebop, Skull & Crossbones")
        .expect("partner-with should lower to a PartnerWith marker plus a real ETB search trigger");
    match &def.abilities[0].kind {
        AbilityKind::Static(static_ability) => {
            assert_eq!(static_ability.id(), StaticAbilityId::PartnerWith);
        }
        other => panic!("expected partner-with marker static ability, got {other:?}"),
    }

    let rendered = unprocessed_compiled_lines(&def);
    assert!(
        rendered
            .iter()
            .any(|line| line == "Partner with Bebop, Skull & Crossbones"),
        "expected partner-with keyword surface, got {rendered:?}"
    );
    let search_lines = rendered
        .iter()
        .filter(|line| {
            line.to_ascii_lowercase()
                .contains("bebop, skull & crossbones")
        })
        .count();
    assert_eq!(
        search_lines, 1,
        "expected exactly one partner-with line mentioning the named partner, got {rendered:?}"
    );
    let debug = format!("{def:#?}");
    assert!(
        ((debug.contains("SearchLibraryEffect") && debug.contains("search_mode: Exact"))
            || (debug.contains("ChooseObjectsEffect") && debug.contains("is_search: true")))
            && !debug.contains("KeywordFallbackText"),
        "expected real library-search effect without fallback, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_reveal_opponent_exiles_rest_hand_then_may_cast() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Allure Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Reveal the top six cards of your library. An opponent exiles a nonland card from among them, then you put the rest into your hand. That opponent may cast the exiled card without paying its mana cost.",
        )
        .expect("opponent-exiles reveal/rest/cast sequence should parse");

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("top six cards of your library")
            && rendered.contains("may cast")
            && rendered.contains("without paying its mana cost"),
        "expected reveal/rest/cast sequence to render, got {rendered}"
    );
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("ChooseObjects")
            && debug.contains("MoveToZone")
            && debug.contains("CastTagged")
            && debug.contains("reveal: true")
            && debug.contains("IsNotTaggedObject")
            && !debug.contains("KeywordFallbackText")
            && !debug.contains("unsupported"),
        "expected real tagged choose/move/cast effects without fallback, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_implicit_become_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Implicit Become Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("It's an enchantment.")
        .expect("implicit become clause should parse");

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("SetCardTypes") && debug.contains("Enchantment"),
        "expected a set-card-types effect for the implicit become clause, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_split_second_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Split Second Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Split second\nDraw a card.")
        .expect("split second line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Split second"),
        "expected split second marker in render output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_cascade_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cascade Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Cascade\nDraw a card.")
        .expect("cascade line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Cascade"),
        "expected cascade keyword in render output, got {rendered}"
    );
    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        debug.contains("staticability(") && debug.contains("cascade"),
        "expected cascade static ability id, got {debug}"
    );
    assert!(
        !debug.contains("staticabilityid::keywordmarker")
            && !debug.contains("staticabilityid::rulefallbacktext"),
        "expected cascade to compile without placeholder static abilities, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_as_you_cascade_land_drop_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Averna Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "As you cascade, you may put a land card from among the exiled cards onto the battlefield tapped.",
        )
        .expect("Averna cascade land-drop line should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "as you cascade, you may put a land card from among the exiled cards onto the battlefield tapped"
        ),
        "expected Averna static ability in render output, got {rendered}"
    );
    assert!(
        def.spell_effect.is_none(),
        "Averna line should not become a spell effect"
    );
    assert!(
        def.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id() == StaticAbilityId::CascadeLandDrop
            )
        }),
        "expected CascadeLandDrop static ability, got {:#?}",
        def.abilities
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_spells_you_cast_have_cascade_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Imoti Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Spells you cast with mana value 6 or greater have cascade.")
        .expect("spell-grant cascade line should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("spells you cast with mana value 6 or greater have cascade"),
        "expected cascade grant in render output, got {rendered}"
    );
    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        debug.contains("grantobjectabilityforfilter")
            && debug.contains("cascade")
            && debug.contains("cast_by: some")
            && debug.contains("you"),
        "expected granted cascade static ability, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_colorless_spells_from_hand_have_double_cascade_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Zhulodok Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Colorless spells you cast from your hand with mana value 7 or greater have \"Cascade, cascade.\"",
        )
        .expect("double-cascade spell grant line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Cascade, cascade")
            || rendered
                .to_ascii_lowercase()
                .contains("cascade and cascade"),
        "expected doubled cascade text in render output, got {rendered}"
    );
    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        debug.matches("cascade").count() >= 2,
        "expected two granted cascade abilities, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_riot_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Riot Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Riot")
        .expect("riot line should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("ChooseModeEffect"),
        "expected riot to compile into a modal ETB trigger, got {abilities_debug}"
    );
    assert!(
        !abilities_debug.contains("StaticAbilityId::KeywordMarker")
            && !abilities_debug.contains("StaticAbilityId::RuleFallbackText"),
        "riot should not remain a placeholder marker ability, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_unleash_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Unleash Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Unleash")
        .expect("unleash line should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("ZoneChangeTrigger")
            || abilities_debug.contains("ThisEntersBattlefield"),
        "expected unleash ETB trigger, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("Unleash"),
        "expected unleash restriction ability, got {abilities_debug}"
    );
    assert!(
        !abilities_debug.contains("StaticAbilityId::KeywordMarker")
            && !abilities_debug.contains("StaticAbilityId::RuleFallbackText"),
        "unleash should not remain a placeholder marker ability, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_zhur_taa_goblin_riot_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Zhur-Taa Goblin")
        .card_types(vec![CardType::Creature])
        .parse_text("Riot")
        .expect("zhur-taa goblin riot line should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("ChooseModeEffect"),
        "expected riot to compile into a modal ETB choice, got {abilities_debug}"
    );
    assert!(
        !abilities_debug.contains("StaticAbilityId::KeywordMarker")
            && !abilities_debug.contains("StaticAbilityId::RuleFallbackText"),
        "zhur-taa goblin riot should not remain a placeholder marker ability, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_training_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Training Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Training (Whenever this creature attacks with another creature with greater power, put a +1/+1 counter on this creature.)",
        )
        .expect("training line should parse as typed trigger");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Training"),
        "expected training keyword render in output, got {rendered}"
    );
    let compiled = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        compiled.contains("Training"),
        "expected compiled text to preserve training keyword, got {compiled}"
    );
    assert!(
        !compiled
            .contains("Whenever this creature attacks with another creature with greater power"),
        "expected compiled text to avoid expanding training reminder text, got {compiled}"
    );
    assert!(
        !rendered.contains("EmitKeywordActionEffect"),
        "training render should hide runtime keyword-action instrumentation, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_parish_blade_trainee_keeps_keyword_and_counter_transfer_surface() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Parish-Blade Trainee")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Training\nWhen this creature dies, put its counters on target creature you control.",
        )
        .expect("Parish-Blade Trainee-style text should parse");

    let compiled = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        compiled.contains("Training"),
        "expected compiled text to preserve training keyword, got {compiled}"
    );
    assert!(
        compiled.contains("put its counters on target creature you control"),
        "expected source counter-transfer wording, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_ravenous_keyword_line_keeps_keyword_surface() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ravenous Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Ravenous (This creature enters with X +1/+1 counters on it. If X is 5 or more, draw a card when it enters.)",
        )
        .expect("ravenous line should parse as typed keyword support");

    let compiled = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        compiled.contains("Ravenous"),
        "expected compiled text to preserve ravenous keyword, got {compiled}"
    );
    assert!(
        !compiled.contains("This creature enters with X +1/+1 counters")
            && !compiled.contains("if X is 5 or more"),
        "expected compiled text to avoid expanding ravenous helper abilities, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_vanishing_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Vanishing Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Vanishing 3 (This creature enters with three time counters on it. At the beginning of your upkeep, remove a time counter from it. When the last is removed, sacrifice it.)",
        )
        .expect("vanishing line should parse as explicit mechanic");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let debug = format!("{def:#?}");
    assert!(
        rendered.contains("Vanishing 3"),
        "expected vanishing render output, got {rendered}"
    );
    assert!(
        !debug.contains("KeywordMarker"),
        "expected vanishing to avoid keyword markers, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn oracle_like_enchant_keyword_grant_does_not_duplicate_keyword_tail() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Aura Keyword Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Enchant creature\nEnchanted creature gets +1/+1 and has flying.")
        .expect("aura keyword grant line should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        !rendered.contains("and flying. and flying"),
        "expected duplicate keyword tail to be collapsed, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn oracle_like_enchant_anthem_keywords_and_subtype_addition_parse_together() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Angelic Aura Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Enchant creature\nEnchanted creature gets +4/+4, has flying and first strike, and is an Angel in addition to its other types.",
        )
        .expect("aura anthem, keyword grants, and subtype addition should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("+4/+4")
            && rendered.contains("flying")
            && rendered.contains("first strike")
            && rendered.contains("angel")
            && rendered.contains("in addition to its other types"),
        "expected composed aura static effect in rendered output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn oracle_like_basic_land_type_aura_sets_land_type_instead_of_adding_it() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Land Type Aura Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Enchant land\nEnchanted land is an Island.")
        .expect("basic land type aura line should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("enchanted land is an island"),
        "expected aura to set the enchanted land's basic land type, got {rendered}"
    );
    assert!(
        !rendered.contains("in addition to its other types"),
        "expected basic land type setting not subtype addition, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn oracle_like_attached_land_put_into_graveyard_trigger_parses() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Genju Probe")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(
            "Enchant Mountain\nWhen enchanted Mountain is put into a graveyard, you may return this card from your graveyard to your hand.",
        )
        .expect("attached land graveyard trigger should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("enchanted mountain is put into a graveyard from the battlefield")
            && (rendered.contains("return this card from your graveyard to your hand")
                || rendered.contains("return this aura from a graveyard to its owner's hand")),
        "expected attached-object graveyard trigger and return effect, got {rendered}"
    );
    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("functional_zones: [Battlefield]"),
        "attached-object graveyard triggers should function from the battlefield, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn oracle_like_return_to_hand_unless_target_opponent_pays_life_parses() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Passage Probe")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever another creature you own dies, return it to your hand unless target opponent pays 3 life.",
        )
        .expect("target opponent unless-pay trigger should parse");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");
    let debug = format!("{:?}", triggered);
    assert!(
        debug.contains("UnlessPaysEffect") && debug.contains("Target(Opponent)"),
        "expected unless-pay effect to target an opponent, got {debug}"
    );
    assert!(
        debug.contains("Player(Opponent)"),
        "expected trigger choices to include an opponent player target, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn oracle_like_cycling_uses_braced_mana_symbols() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cycling Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Cycling {2}")
        .expect("cycling line should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("cycling {2}"),
        "expected braced cycling mana cost in render output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_unearth_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Unearth Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{T}: You may tap or untap another target permanent.\n\
Unearth {U} ({U}: Return this card from your graveyard to the battlefield. It gains haste. Exile it at the beginning of the next end step or if it would leave the battlefield. Unearth only as a sorcery.)",
        )
        .expect("unearth keyword line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Unearth {U}")
            || rendered.contains("UnearthEffect")
            || rendered.contains("Unearth"),
        "expected unearth keyword in render output, got {rendered}"
    );
    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        !debug.contains("staticabilityid::custom") && !debug.contains("keyword_marker"),
        "expected unearth to compile without placeholder marker static abilities, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_echo_keyword_line_with_mana_cost() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Echo Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Echo {2}{R} (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)",
        )
        .expect("echo keyword line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Echo {2}{R}"),
        "expected echo keyword render in output, got {rendered}"
    );

    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        !debug.contains("staticabilityid::custom") && !debug.contains("keyword_marker"),
        "expected echo to compile without placeholder marker static abilities, got {debug}"
    );
    assert!(
        debug.contains("counter_type: echo"),
        "expected echo to track an internal echo counter, got {debug}"
    );
    assert!(
        debug.contains("paymanaeffect"),
        "expected echo mana variant to include a mana payment effect, got {debug}"
    );
    assert!(
        debug.contains("withideffect"),
        "expected echo trigger to track counter removal outcome with WithIdEffect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_echo_keyword_line_with_non_mana_cost() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Echo Discard Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Flying, haste\nEcho—Discard a card. (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)",
        )
        .expect("echo keyword line with non-mana cost should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.to_ascii_lowercase().contains("discard a card"),
        "expected non-mana echo payment text in render output, got {rendered}"
    );

    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        !debug.contains("staticabilityid::custom") && !debug.contains("keyword_marker"),
        "expected echo to compile without placeholder marker static abilities, got {debug}"
    );
    assert!(
        debug.contains("counter_type: echo"),
        "expected echo to track an internal echo counter, got {debug}"
    );
    assert!(
        debug.contains("unlessactioneffect"),
        "expected echo non-mana variant to use unless-action payment flow, got {debug}"
    );
    assert!(
        debug.contains("withideffect"),
        "expected echo trigger to track counter removal outcome with WithIdEffect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_echo_keyword_line_with_life_cost() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Echo Life Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Echo—Pay 3 life. (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)",
        )
        .expect("echo keyword line with life cost should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Echo—Pay 3 life"),
        "expected life echo payment text in stored ability text, got {rendered}"
    );

    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        debug.contains("counter_type: echo") && debug.contains("loselifeeffect"),
        "expected echo life variant to compile as a checked payment effect, got {debug}"
    );
    assert!(
        !debug.contains("keyword_marker") && !debug.contains("keywordfallbacktext"),
        "echo life cost should not fall back to marker text, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_echo_keyword_line_with_sacrifice_cost() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Echo Sacrifice Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Echo—Sacrifice a creature. (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)",
        )
        .expect("echo keyword line with sacrifice cost should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Echo—Sacrifice a creature"),
        "expected sacrifice echo payment text in stored ability text, got {rendered}"
    );

    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        debug.contains("counter_type: echo") && debug.contains("sacrificeeffect"),
        "expected echo sacrifice variant to compile as a checked payment effect, got {debug}"
    );
    assert!(
        !debug.contains("keyword_marker") && !debug.contains("keywordfallbacktext"),
        "echo sacrifice cost should not fall back to marker text, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_echo_keyword_line_with_non_cost_effect_fails_loudly() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Echo Draw Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Echo—Draw a card. (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)",
        )
        .expect_err("non-cost echo payment should fail loudly");

    let err = format!("{err:?}").to_ascii_lowercase();
    assert!(
        err.contains("echo") && (err.contains("cost") || err.contains("cost-executable")),
        "expected loud echo cost error, got {err}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_escape_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Escape Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Escape—{3}{B}{B}, exile four other cards from your graveyard.")
        .expect("escape keyword line should parse");

    assert_eq!(def.alternative_casts.len(), 1);
    match &def.alternative_casts[0] {
        AlternativeCastingMethod::Escape {
            cost, exile_count, ..
        } => {
            assert_eq!(*exile_count, 4);
            let cost = cost
                .as_ref()
                .expect("escape should carry explicit mana cost");
            assert_eq!(cost.to_oracle(), "{3}{B}{B}");
        }
        other => panic!("expected escape alternative cast, got {other:?}"),
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_flashback_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Flashback Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Flashback {1}{U}")
        .expect("flashback keyword line should parse");

    assert_eq!(def.alternative_casts.len(), 1);
    match &def.alternative_casts[0] {
        AlternativeCastingMethod::Flashback { total_cost } => {
            let cost = total_cost
                .mana_cost()
                .expect("flashback should include mana cost");
            assert_eq!(cost.to_oracle(), "{1}{U}");
            let costs = def.alternative_casts[0].non_mana_costs();
            assert!(
                costs.is_empty(),
                "expected flashback test probe to have no extra non-mana costs, got {costs:?}"
            );
        }
        other => panic!("expected flashback alternative cast, got {other:?}"),
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_bestow_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bestow Probe")
        .card_types(vec![CardType::Enchantment, CardType::Creature])
        .subtypes(vec![Subtype::Insect])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text("Bestow {3}{W}\nLifelink\nEnchanted creature gets +1/+1 and has lifelink.")
        .expect("bestow keyword line should parse");

    assert_eq!(def.alternative_casts.len(), 1);
    match &def.alternative_casts[0] {
        AlternativeCastingMethod::Bestow { total_cost } => {
            let cost = total_cost
                .mana_cost()
                .expect("bestow should include mana cost");
            assert_eq!(cost.to_oracle(), "{3}{W}");
            let costs = def.alternative_casts[0].non_mana_costs();
            assert!(
                costs.is_empty(),
                "expected mana-only bestow cost for probe, got {costs:?}"
            );
        }
        other => panic!("expected bestow alternative cast, got {other:?}"),
    }

    let static_ids = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        !static_ids.contains(&StaticAbilityId::KeywordMarker)
            && !static_ids.contains(&StaticAbilityId::RuleFallbackText)
            && !static_ids.contains(&StaticAbilityId::UnsupportedParserLine),
        "bestow line should compile without placeholder static abilities, got {static_ids:?}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Bestow {3}{W}"),
        "expected compiled text to include bestow line, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_bestow_keyword_line_with_extra_cost_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Bestow Extra Cost Probe")
        .card_types(vec![CardType::Enchantment, CardType::Creature])
        .subtypes(vec![Subtype::Insect])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "Bestow—{R}, Collect evidence 6.\nFlying\nEnchanted creature gets +2/+2 and has flying.",
        )
        .expect("bestow line with extra clause should parse");

    assert_eq!(def.alternative_casts.len(), 1);
    match &def.alternative_casts[0] {
        AlternativeCastingMethod::Bestow { total_cost } => {
            let cost = total_cost
                .mana_cost()
                .expect("bestow should include mana cost");
            assert_eq!(cost.to_oracle(), "{R}");
        }
        other => panic!("expected bestow alternative cast, got {other:?}"),
    }

    let debug = format!("{def:?}");
    assert!(
        !debug.contains("KeywordMarker")
            && !debug.contains("RuleFallbackText")
            && !debug.contains("UnsupportedParserLine"),
        "bestow extra-cost line should avoid placeholder fallback, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_buyback_keyword_line_compiles_to_optional_cost() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Buyback Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Buyback {3}\nDraw a card.")
        .expect("buyback keyword line should parse");

    assert_eq!(def.optional_costs.len(), 1);
    let buyback = &def.optional_costs[0];
    assert_eq!(buyback.source_label, "Buyback");
    assert!(buyback.returns_to_hand);
    let mana = buyback
        .cost
        .mana_cost()
        .expect("buyback should preserve mana cost");
    assert_eq!(mana.to_oracle(), "{3}");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_kicker_keyword_line_compiles_to_optional_cost() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Kicker Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("Kicker {1}{U}\nDraw a card.")
        .expect("kicker keyword line should parse");

    assert_eq!(def.optional_costs.len(), 1);
    let kicker = &def.optional_costs[0];
    assert_eq!(kicker.source_label, "Kicker");
    assert!(!kicker.repeatable, "kicker should not be repeatable");
    let mana = kicker
        .cost
        .mana_cost()
        .expect("kicker should preserve mana cost");
    assert_eq!(mana.to_oracle(), "{1}{U}");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_kicker_keyword_line_with_reminder_text_strips_reminder_tail() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Kicker Reminder Probe")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Kicker {2}{R} (You may pay an additional {2}{R} as you cast this spell.)\nDraw a card.",
        )
        .expect("kicker keyword with reminder text should parse");

    assert_eq!(def.optional_costs.len(), 1);
    let kicker = &def.optional_costs[0];
    assert_eq!(kicker.source_label, "Kicker");
    let mana = kicker
        .cost
        .mana_cost()
        .expect("kicker should preserve mana cost");
    assert_eq!(mana.to_oracle(), "{2}{R}");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn nightscape_battlemage_strict_parser_text_and_structure_regression() {
    let def = parse_oracle_card_definition("Nightscape Battlemage");

    assert_eq!(def.optional_costs.len(), 2);
    assert_eq!(def.optional_costs[0].source_label, "Kicker {2}{U}");
    assert_eq!(def.optional_costs[1].source_label, "Kicker {2}{R}");
    assert_eq!(
        def.optional_costs[0]
            .cost
            .mana_cost()
            .expect("blue kicker should be a mana cost")
            .to_oracle(),
        "{2}{U}"
    );
    assert_eq!(
        def.optional_costs[1]
            .cost
            .mana_cost()
            .expect("red kicker should be a mana cost")
            .to_oracle(),
        "{2}{R}"
    );

    let rendered = compiled_text_lines(&def).join("\n");
    assert!(
        rendered.contains("Kicker {2}{U} and/or {2}{R}"),
        "expected split kicker costs to render as one and/or keyword line, got {rendered}"
    );
    assert!(
        rendered.contains(
            "When this creature enters, if it was kicked with its {2}{U} kicker, return up to two target nonblack creatures to their owners' hands."
        ),
        "expected blue kicker ETB clause in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains(
            "When this creature enters, if it was kicked with its {2}{R} kicker, destroy target land."
        ),
        "expected red kicker ETB clause in compiled text, got {rendered}"
    );

    let conditions: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered.intervening_if.as_ref(),
            _ => None,
        })
        .collect();
    assert!(
        conditions.contains(&&crate::effect::Condition::ThisSpellPaidLabel(
            "Kicker {2}{U}".into()
        )),
        "expected blue kicker paid-label condition, got {conditions:?}"
    );
    assert!(
        conditions.contains(&&crate::effect::Condition::ThisSpellPaidLabel(
            "Kicker {2}{R}".into()
        )),
        "expected red kicker paid-label condition, got {conditions:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn nightscape_battlemage_runtime_branches_use_matching_kicker_labels() {
    fn trigger_effects_for_label(
        def: &CardDefinition,
        label: &str,
    ) -> crate::resolution::ResolutionProgram {
        def.abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered)
                    if triggered.intervening_if.as_ref()
                        == Some(&crate::effect::Condition::ThisSpellPaidLabel(label.into())) =>
                {
                    Some(triggered.effects.clone())
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing Nightscape Battlemage trigger for {label}"))
    }

    fn paid(def: &CardDefinition, index: usize) -> crate::cost::OptionalCostsPaid {
        let mut paid = crate::cost::OptionalCostsPaid::from_costs(&def.optional_costs);
        paid.pay(index);
        paid
    }

    let def = parse_oracle_card_definition("Nightscape Battlemage");
    let blue_effects = trigger_effects_for_label(&def, "Kicker {2}{U}");
    let red_effects = trigger_effects_for_label(&def, "Kicker {2}{R}");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let etb_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::EnterBattlefieldEvent::new(source, Zone::Stack),
        crate::provenance::ProvNodeId::default(),
    );
    let nonblack_def = CardDefinitionBuilder::new(CardId::new(), "Nonblack Target")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let other_nonblack_def = CardDefinitionBuilder::new(CardId::new(), "Other Nonblack Target")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let land_def = CardDefinitionBuilder::new(CardId::new(), "Target Land")
        .card_types(vec![CardType::Land])
        .build();
    let first_creature = game.create_object_from_definition(&nonblack_def, bob, Zone::Battlefield);
    let second_creature =
        game.create_object_from_definition(&other_nonblack_def, bob, Zone::Battlefield);
    let land = game.create_object_from_definition(&land_def, bob, Zone::Battlefield);

    game.push_to_stack(
        crate::game_state::StackEntry::ability(source, alice, blue_effects.clone())
            .with_targets(vec![
                crate::game_state::Target::Object(first_creature),
                crate::game_state::Target::Object(second_creature),
            ])
            .with_optional_costs_paid(paid(&def, 0))
            .with_intervening_if(crate::effect::Condition::ThisSpellPaidLabel(
                "Kicker {2}{U}".into(),
            )),
    );
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("blue Nightscape Battlemage trigger should resolve");
    assert!(
        !game.battlefield.contains(&first_creature) && !game.battlefield.contains(&second_creature),
        "blue kicker trigger should remove both nonblack creatures from the battlefield"
    );
    let bob_hand_names: Vec<_> = game
        .player(bob)
        .expect("Bob exists")
        .hand
        .iter()
        .filter_map(|id| game.object(*id))
        .map(|object| object.name.as_str())
        .collect();
    assert!(
        bob_hand_names.contains(&"Nonblack Target")
            && bob_hand_names.contains(&"Other Nonblack Target"),
        "blue kicker trigger should return both nonblack creatures to hand, got {bob_hand_names:?}"
    );

    assert!(
        !crate::triggers::verify_intervening_if(
            &game,
            &crate::effect::Condition::ThisSpellPaidLabel("Kicker {2}{R}".into()),
            alice,
            &etb_event,
            source,
            None,
            Some(&paid(&def, 0)),
        ),
        "red kicker intervening-if must be false when only the blue kicker was paid"
    );
    assert!(
        game.battlefield.contains(&land),
        "checking the unpaid red branch must not move the target land"
    );

    assert!(
        crate::triggers::verify_intervening_if(
            &game,
            &crate::effect::Condition::ThisSpellPaidLabel("Kicker {2}{R}".into()),
            alice,
            &etb_event,
            source,
            None,
            Some(&paid(&def, 1)),
        ),
        "red kicker intervening-if should be true when the red kicker was paid"
    );

    game.push_to_stack(
        crate::game_state::StackEntry::ability(source, alice, red_effects)
            .with_targets(vec![crate::game_state::Target::Object(land)])
            .with_optional_costs_paid(paid(&def, 1))
            .with_intervening_if(crate::effect::Condition::ThisSpellPaidLabel(
                "Kicker {2}{R}".into(),
            )),
    );
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("red Nightscape Battlemage trigger should resolve when red kicker was paid");
    assert!(
        !game.battlefield.contains(&land),
        "red kicker trigger should destroy the target land"
    );
    let bob_graveyard_names: Vec<_> = game
        .player(bob)
        .expect("Bob exists")
        .graveyard
        .iter()
        .filter_map(|id| game.object(*id))
        .map(|object| object.name.as_str())
        .collect();
    assert!(
        bob_graveyard_names.contains(&"Target Land"),
        "destroyed land should move to its owner's graveyard, got {bob_graveyard_names:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_dash_kicker_with_typed_discard_cost_compiles_to_optional_cost() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Dash Kicker Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Kicker—{2}{B}, Discard a creature card.\nFlying")
        .expect("dash kicker with discard cost should parse");

    assert_eq!(def.optional_costs.len(), 1);
    assert!(
        def.spell_effect.is_none(),
        "discard cost must not become a spell effect"
    );
    let kicker = &def.optional_costs[0];
    assert_eq!(kicker.source_label, "Kicker");
    let costs = kicker.cost.costs();
    assert_eq!(costs.len(), 2);
    assert_eq!(
        costs[0]
            .mana_cost_ref()
            .expect("first component should be mana")
            .to_oracle(),
        "{2}{B}"
    );
    match costs[1].processing_mode() {
        crate::costs::CostProcessingMode::DiscardCards { count, card_types } => {
            assert_eq!(count, 1);
            assert_eq!(card_types, vec![CardType::Creature]);
        }
        other => panic!("expected typed discard cost, got {other:?}"),
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_multikicker_and_entwine_keyword_lines_compile_to_optional_costs() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Multi Optional Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Multikicker {1}{G}\nEntwine {2}\nDraw a card.")
        .expect("multikicker/entwine keyword lines should parse");

    assert_eq!(def.optional_costs.len(), 2);

    let multikicker = &def.optional_costs[0];
    assert_eq!(multikicker.source_label, "Multikicker");
    assert!(multikicker.repeatable, "multikicker should be repeatable");
    let mana = multikicker
        .cost
        .mana_cost()
        .expect("multikicker should preserve mana cost");
    assert_eq!(mana.to_oracle(), "{1}{G}");

    let entwine = &def.optional_costs[1];
    assert_eq!(entwine.source_label, "Entwine");
    assert!(!entwine.repeatable, "entwine should not be repeatable");
    let mana = entwine
        .cost
        .mana_cost()
        .expect("entwine should preserve mana cost");
    assert_eq!(mana.to_oracle(), "{2}");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_strength_of_the_tajuru_strict_and_renders_kicked_targets() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Strength of the Tajuru")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::X],
            vec![ManaSymbol::Green],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Multikicker {1} (You may pay an additional {1} any number of times as you cast this spell.)\n\
             Choose target creature, then choose another target creature for each time this spell was kicked. Put X +1/+1 counters on each of them.",
        )
        .expect("Strength of the Tajuru should parse strictly");

    assert_eq!(def.optional_costs.len(), 1);
    assert_eq!(def.optional_costs[0].source_label, "Multikicker");
    assert!(
        def.optional_costs[0].repeatable,
        "Strength of the Tajuru's multikicker should be repeatable"
    );

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("WithCountValue") && debug.contains("KickCount"),
        "expected target count to be structurally tied to the multikicker count, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.to_ascii_lowercase().contains("multikicker {1}"),
        "expected Strength of the Tajuru compiled text to preserve kicked target clause, got {rendered}"
    );
    assert!(
        rendered.contains("Put X +1/+1 counters on each target creature"),
        "Strength of the Tajuru should keep the counter effect tied to the chosen creatures, got {rendered}\n{debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_spell_contortion_strict_and_renders_kicked_draw_count() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Spell Contortion")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Multikicker {1}{U} (You may pay an additional {1}{U} any number of times as you cast this spell.)\n\
             Counter target spell unless its controller pays {2}. Draw a card for each time Spell Contortion was kicked.",
        )
        .expect("Spell Contortion should parse strictly");

    assert_eq!(def.optional_costs.len(), 1);
    assert_eq!(def.optional_costs[0].source_label, "Multikicker");
    assert!(
        def.optional_costs[0].repeatable,
        "Spell Contortion's multikicker should be repeatable"
    );

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("UnlessPaysEffect")
            && debug.contains("DrawCardsEffect")
            && debug.contains("KickCount"),
        "expected Spell Contortion to model counter-unless and draw per kick count, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("Counter target spell unless its controller pays {2}"),
        "expected Spell Contortion compiled text to preserve counter-unless clause, got {rendered}"
    );
    assert!(
        rendered.contains(
            "Counter target spell unless its controller pays {2}. Draw a card for each time this spell was kicked"
        ),
        "expected Spell Contortion compiled text to preserve kicked draw count, got {rendered}"
    );

    let scored = compiled_text_lines(&def).join("\n");
    assert!(
        scored.contains(
            "Counter target spell unless its controller pays {2}. Draw a card for each time Spell Contortion was kicked"
        ),
        "expected Spell Contortion scored text to use the source name for kicked draw count, got {scored}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_replicate_keyword_line_compiles_to_repeatable_optional_cost() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Replicate Probe")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Replicate {1} (When you cast this spell, copy it for each time you paid its replicate cost. You may choose new targets for the copies.)\nDraw a card.",
        )
        .expect("replicate keyword line should parse");

    assert_eq!(def.optional_costs.len(), 1);
    let replicate = &def.optional_costs[0];
    assert_eq!(replicate.source_label, "Replicate");
    assert!(replicate.repeatable, "replicate should be repeatable");
    let mana = replicate
        .cost
        .mana_cost()
        .expect("replicate should preserve mana cost");
    assert_eq!(mana.to_oracle(), "{1}");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Replicate {1}"),
        "expected replicate optional-cost line, got {rendered}"
    );
    assert!(
        !rendered.contains("UnsupportedParserLine"),
        "replicate keyword line should not rely on unsupported parser fallback: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_squad_keyword_line_compiles_to_optional_cost_and_etb_copy_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Squad Test")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Soldier])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text("Squad {2}\nFlying")
        .expect("squad keyword line should parse");

    assert_eq!(def.optional_costs.len(), 1, "expected one squad cost");
    let squad = &def.optional_costs[0];
    assert_eq!(squad.source_label, "Squad");
    assert!(squad.repeatable, "squad should be repeatable");
    let mana = squad
        .cost
        .mana_cost()
        .expect("squad should preserve mana cost");
    assert_eq!(mana.to_oracle(), "{2}");

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("Squad {2}.") && !rendered.contains("optional cost 'Squad' was paid"),
        "expected squad optional-cost line, got {rendered}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("CreateTokenCopyEffect")
            && debug.contains("TimesPaidLabel(OptionalCostRef { kind: Squad"),
        "expected squad ETB copy trigger, got {debug}"
    );
    assert!(
        !debug.contains("KeywordMarker") && !debug.contains("MarkerText(\"Squad"),
        "squad should not fall back to a marker, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_squad_trigger_creates_token_copies_equal_to_times_paid() {
    use crate::ability::AbilityKind;
    use crate::cost::OptionalCostsPaid;
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::object::ObjectKind;
    use crate::tests::test_helpers::setup_two_player_game;
    use crate::zone::Zone;

    let mut game = setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let def = CardDefinitionBuilder::new(CardId::new(), "Squad Test")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Soldier])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text("Squad {2}\nFlying")
        .expect("squad keyword line should parse");

    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let mut paid = OptionalCostsPaid::from_costs(&def.optional_costs);
    paid.pay_times(0, 2);
    game.object_mut(source)
        .expect("source object exists")
        .optional_costs_paid = paid.clone();

    let effects = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) if triggered.trigger.display().contains("enters") => {
                Some(triggered.effects.clone())
            }
            _ => None,
        })
        .expect("squad ETB trigger should exist");

    let mut ctx = ExecutionContext::new_default(source, alice).with_optional_costs_paid(paid);
    for effect in &effects {
        execute_effect(&mut game, effect, &mut ctx).expect("squad effect should resolve");
    }

    let squad_objects: Vec<_> = game
        .battlefield
        .iter()
        .filter_map(|&id| game.object(id))
        .filter(|obj| game.controller_of(obj) == alice && obj.name == "Squad Test")
        .collect();
    assert_eq!(
        squad_objects.len(),
        3,
        "expected original plus two squad copies"
    );

    let token_count = squad_objects
        .iter()
        .filter(|obj| obj.kind == ObjectKind::Token)
        .count();
    assert_eq!(token_count, 2, "expected two squad-created tokens");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_thrill_kill_disciple_compiles_squad_as_optional_cost_and_death_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Thrill-Kill Disciple")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Mercenary])
        .power_toughness(PowerToughness::fixed(3, 2))
        .parse_text("Squad—{1}, Discard a card.\nWhen this creature dies, create a Junk token.")
        .expect("thrill-kill disciple should parse");

    assert_eq!(def.optional_costs.len(), 1, "expected one squad cost");
    let squad = &def.optional_costs[0];
    assert_eq!(squad.source_label, "Squad");
    assert!(squad.repeatable, "squad should be repeatable");

    let costs = squad.cost.costs();
    assert_eq!(costs.len(), 2, "expected mana plus discard squad cost");
    assert_eq!(
        costs[0].mana_cost_ref().map(|mana| mana.to_oracle()),
        Some("{1}".to_string()),
        "expected squad mana payment first, got {costs:?}"
    );
    assert_eq!(
        costs[1].discard_details(),
        Some((1, None)),
        "expected squad discard payment second, got {costs:?}"
    );
    assert!(
        def.spell_effect.is_none(),
        "squad discard should not become a spell effect"
    );

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("Squad—{1}, Discard a card")
            && rendered.contains("When this creature dies, create a Junk token"),
        "expected Thrill-Kill Disciple text to preserve squad and dies trigger, got {rendered}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("TimesPaidLabel(OptionalCostRef { kind: Squad"),
        "expected squad copy trigger, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_offspring_keyword_line_compiles_to_optional_cost_and_etb_copy_trigger() {
    use crate::ability::AbilityKind;
    use crate::effect::Condition;

    let def = CardDefinitionBuilder::new(CardId::new(), "Offspring Test")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Rabbit])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text("Offspring {2}\nFlying")
        .expect("offspring keyword line should parse");

    assert_eq!(def.optional_costs.len(), 1, "expected one offspring cost");
    let offspring = &def.optional_costs[0];
    assert_eq!(offspring.source_label, "Offspring");
    assert!(!offspring.repeatable, "offspring should not be repeatable");
    let mana = offspring
        .cost
        .mana_cost()
        .expect("offspring should preserve mana cost");
    assert_eq!(mana.to_oracle(), "{2}");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("offspring {2}"),
        "expected offspring optional-cost line, got {rendered}"
    );

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) if triggered.trigger.display().contains("enters") => {
                Some(triggered)
            }
            _ => None,
        })
        .expect("offspring ETB trigger should exist");
    assert_eq!(
        triggered.intervening_if.as_ref(),
        Some(&Condition::ThisSpellPaidLabel("Offspring".into())),
        "offspring should use an intervening-if cost-paid check",
    );

    let debug = format!("{:?}", triggered.effects);
    assert!(
        debug.contains("CreateTokenCopyEffect")
            && debug.contains("WasPaidLabel(OptionalCostRef { kind: Offspring")
            && debug.contains("set_base_power_toughness: Some((1, 1))"),
        "expected offspring ETB copy effect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_conspire_keyword_line_compiles_to_optional_cost() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Conspire Test")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Conspire\nDraw a card.")
        .expect("conspire keyword line should parse");

    assert_eq!(def.optional_costs.len(), 1, "expected one conspire cost");
    let conspire = &def.optional_costs[0];
    assert_eq!(conspire.source_label, "Conspire");
    assert!(!conspire.repeatable, "conspire should not be repeatable");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered.contains("Conspire"),
        "expected conspire keyword in compiled output, got {rendered}"
    );
    assert!(
        rendered_lower.contains("draw a card"),
        "expected spell body to remain intact, got {rendered}"
    );
    assert!(
        rendered_lower.find("draw a card") < rendered_lower.find("conspire"),
        "expected spell text to render before conspire reminder, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn excavation_technique_parses_demonstrate_and_renders_keyword_text() {
    let def = parse_oracle_card_definition("Excavation Technique");

    let lines = unprocessed_compiled_lines(&def);
    let rendered = lines.join("\n");
    assert!(
        rendered.contains("Demonstrate"),
        "expected Excavation Technique to render the demonstrate keyword, got {rendered}"
    );
    assert_eq!(
        lines,
        vec![
            "Destroy target nonland permanent. Its controller creates 2 Treasure tokens.",
            "Demonstrate.",
        ],
        "expected Excavation Technique compiled text to preserve spell text and keyword identity"
    );
    assert!(
        !rendered.to_ascii_lowercase().contains("unsupported"),
        "strict parser should not emit unsupported fallback text, got {rendered}"
    );

    let demonstrate = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if ability.functional_zones == [Zone::Stack]
                    && triggered.trigger.display() == "When you cast this spell" =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Excavation Technique should compile demonstrate as a stack trigger");
    let debug = format!("{:#?}", demonstrate.effects);
    assert!(
        debug.contains("MayEffect")
            && debug.contains("CopySpellEffect")
            && debug.contains("ChoosePlayerEffect")
            && debug.contains("ChooseNewTargetsEffect")
            && debug.contains("Opponent"),
        "expected demonstrate trigger to copy for you and a chosen opponent with retarget choices, got {debug}"
    );
}

#[test]
pub(super) fn excavation_technique_demonstrate_decline_creates_no_spell_copies() {
    use crate::decision::DecisionMaker;
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::game_state::Target;

    #[derive(Default)]
    struct DeclineDemonstrate;
    impl DecisionMaker for DeclineDemonstrate {}

    let (mut game, source, triggered) = setup_excavation_technique_demonstrate_runtime();
    let mut dm = DeclineDemonstrate;
    let mut ctx = ExecutionContext::new_default(source, PlayerId::from_index(0))
        .with_targets(vec![crate::effects::ResolvedTarget::Object(
            game.stack
                .iter()
                .find(|entry| entry.object_id == source)
                .and_then(|entry| entry.targets.first())
                .and_then(|target| match target {
                    Target::Object(id) => Some(*id),
                    Target::Player(_) => None,
                })
                .expect("original spell should have a target"),
        )])
        .with_decision_maker(&mut dm);

    for effect in &triggered.effects {
        execute_effect(&mut game, effect, &mut ctx).expect("declining demonstrate should resolve");
    }

    assert_eq!(
        stack_entries_named(&game, "Excavation Technique").len(),
        1,
        "declining demonstrate should leave only the original spell on the stack"
    );
    assert!(
        game.stack.iter().all(|entry| entry.object_id == source),
        "no copy stack entries should be created when demonstrate is declined"
    );
}

pub(super) fn setup_excavation_technique_demonstrate_runtime() -> (
    crate::game_state::GameState,
    ObjectId,
    crate::ability::TriggeredAbility,
) {
    use crate::game_state::{StackEntry, Target};

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let original_target = crate::card::CardBuilder::new(CardId::from_raw(60_001), "Alice Relic")
        .card_types(vec![CardType::Artifact])
        .build();
    let replacement_target =
        crate::card::CardBuilder::new(CardId::from_raw(60_002), "Bob Replacement Relic")
            .card_types(vec![CardType::Artifact])
            .build();
    let original_target = game.create_object_from_card(&original_target, alice, Zone::Battlefield);
    game.create_object_from_card(&replacement_target, bob, Zone::Battlefield);

    let def = CardDefinitionBuilder::new(CardId::from_raw(60_003), "Excavation Technique")
        .card_types(vec![CardType::Sorcery])
        .demonstrate()
        .with_spell_effect(vec![Effect::destroy(ChooseSpec::target(
            ChooseSpec::Object(ObjectFilter::nonland_permanent()),
        ))])
        .build();
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered.clone()),
            _ => None,
        })
        .expect("demonstrate trigger should exist");

    let source = game.create_object_from_definition(&def, alice, Zone::Stack);
    let mut entry = StackEntry::new(source, alice);
    entry.targets = vec![Target::Object(original_target)];
    game.push_to_stack(entry);

    (game, source, triggered)
}

pub(super) fn stack_entries_named(
    game: &crate::game_state::GameState,
    name: &str,
) -> Vec<crate::game_state::StackEntry> {
    game.stack
        .iter()
        .filter(|entry| {
            game.object(entry.object_id)
                .is_some_and(|object| object.name == name)
        })
        .cloned()
        .collect()
}

#[test]
pub(super) fn excavation_technique_demonstrate_creates_player_and_opponent_copies_with_retargets() {
    use crate::decision::DecisionMaker;
    use crate::decisions::context::{BooleanContext, SelectOptionsContext, TargetsContext};
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::game_state::{GameState, Target};

    struct AcceptDemonstrateAndRetarget {
        replacement_target: ObjectId,
        boolean_calls: usize,
    }

    impl DecisionMaker for AcceptDemonstrateAndRetarget {
        fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
            self.boolean_calls += 1;
            true
        }

        fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            ctx.options
                .iter()
                .filter(|option| option.legal)
                .map(|option| option.index)
                .take(ctx.min)
                .collect()
        }

        fn decide_targets(&mut self, _game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
            assert_eq!(
                ctx.requirements.len(),
                1,
                "expected one copied spell target choice"
            );
            assert!(
                ctx.requirements[0]
                    .legal_targets
                    .contains(&Target::Object(self.replacement_target)),
                "replacement target should be legal for the copied Excavation Technique"
            );
            vec![Target::Object(self.replacement_target)]
        }
    }

    let (mut game, source, triggered) = setup_excavation_technique_demonstrate_runtime();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let replacement_target = game
        .battlefield
        .iter()
        .copied()
        .find(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Bob Replacement Relic")
        })
        .expect("replacement target should exist");
    let original_target = game
        .stack
        .iter()
        .find(|entry| entry.object_id == source)
        .and_then(|entry| entry.targets.first().copied())
        .expect("original spell should have a target");

    let mut dm = AcceptDemonstrateAndRetarget {
        replacement_target,
        boolean_calls: 0,
    };
    let mut ctx = ExecutionContext::new_default(source, alice)
        .with_targets(vec![match original_target {
            Target::Object(id) => crate::effects::ResolvedTarget::Object(id),
            Target::Player(id) => crate::effects::ResolvedTarget::Player(id),
        }])
        .with_decision_maker(&mut dm);

    for effect in &triggered.effects {
        execute_effect(&mut game, effect, &mut ctx).expect("demonstrate should resolve");
    }

    let entries = stack_entries_named(&game, "Excavation Technique");
    assert_eq!(
        entries.len(),
        3,
        "accepting demonstrate should leave the original plus two spell copies on the stack"
    );

    let alice_copies = entries
        .iter()
        .filter(|entry| entry.object_id != source && entry.controller == alice)
        .collect::<Vec<_>>();
    let bob_copies = entries
        .iter()
        .filter(|entry| entry.object_id != source && entry.controller == bob)
        .collect::<Vec<_>>();
    assert_eq!(
        alice_copies.len(),
        1,
        "Alice should control one demonstrate copy"
    );
    assert_eq!(
        bob_copies.len(),
        1,
        "chosen opponent should control one demonstrate copy"
    );
    assert_eq!(
        alice_copies[0].targets,
        vec![Target::Object(replacement_target)],
        "Alice should be able to choose new targets for her demonstrate copy"
    );
    assert_eq!(
        bob_copies[0].targets,
        vec![Target::Object(replacement_target)],
        "the chosen opponent should be able to choose new targets for their demonstrate copy"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_offspring_trigger_creates_one_one_copy_when_paid() {
    use crate::ability::AbilityKind;
    use crate::cost::OptionalCostsPaid;
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::object::ObjectKind;
    use crate::tests::test_helpers::setup_two_player_game;
    use crate::zone::Zone;

    let mut game = setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let def = CardDefinitionBuilder::new(CardId::new(), "Offspring Test")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Rabbit])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text("Offspring {2}\nFlying")
        .expect("offspring keyword line should parse");

    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let mut paid = OptionalCostsPaid::from_costs(&def.optional_costs);
    paid.pay(0);
    game.object_mut(source)
        .expect("source object exists")
        .optional_costs_paid = paid.clone();

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) if triggered.trigger.display().contains("enters") => {
                Some(triggered.clone())
            }
            _ => None,
        })
        .expect("offspring ETB trigger should exist");

    let mut ctx = ExecutionContext::new_default(source, alice).with_optional_costs_paid(paid);
    for effect in &triggered.effects {
        execute_effect(&mut game, effect, &mut ctx).expect("offspring effect should resolve");
    }

    let offspring_objects: Vec<_> = game
        .battlefield
        .iter()
        .filter_map(|&id| game.object(id))
        .filter(|obj| game.controller_of(obj) == alice && obj.name == "Offspring Test")
        .collect();
    assert_eq!(
        offspring_objects.len(),
        2,
        "expected original plus one offspring token"
    );

    let token = offspring_objects
        .iter()
        .find(|obj| obj.kind == ObjectKind::Token)
        .expect("expected an offspring token");
    assert_eq!(token.power(), Some(1), "offspring token should be 1/1");
    assert_eq!(token.toughness(), Some(1), "offspring token should be 1/1");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_scavenge_keyword_line_compiles_to_graveyard_activated_ability() {
    use crate::zone::Zone;

    let def = CardDefinitionBuilder::new(CardId::new(), "Scavenge Test")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text("Scavenge {2}{G}")
        .expect("scavenge keyword line should parse");

    let (ability, activated) = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some((ability, activated)),
            _ => None,
        })
        .expect("scavenge should compile to an activated ability");

    assert_eq!(ability.functional_zones, vec![Zone::Graveyard]);

    let debug = format!("{:?}", activated);
    assert!(
        debug.contains("SourcePower")
            && debug.contains("PlusOnePlusOne")
            && debug.contains("ExileEffect"),
        "expected scavenge cost/effect lowering, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_embalm_keyword_line_compiles_to_graveyard_token_copy_activation() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Embalm Test")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Angel])
        .power_toughness(PowerToughness::fixed(3, 4))
        .parse_text(
            "Embalm {5}{W} ({5}{W}, Exile this card from your graveyard: Create a token that's a copy of it, except it's a white Zombie Angel with no mana cost. Embalm only as a sorcery.)",
        )
        .expect("embalm keyword line should parse");

    let (ability, activated) = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some((ability, activated)),
            _ => None,
        })
        .expect("embalm should compile to an activated ability");

    assert_eq!(ability.functional_zones, vec![Zone::Graveyard]);
    assert_eq!(activated.choices.len(), 0);

    let flattened = activated.effects.flattened_default_effects();
    let [effect] = flattened else {
        panic!("embalm should have exactly one default effect");
    };
    let create = effect
        .downcast_ref::<crate::effects::CreateTokenCopyEffect>()
        .expect("embalm should create a token copy");
    assert!(matches!(&create.target, ChooseSpec::Source));
    assert_eq!(create.count, Value::Fixed(1));
    assert_eq!(create.controller, PlayerFilter::You);
    assert_eq!(create.set_colors, Some(crate::color::ColorSet::WHITE));
    assert!(create.added_subtypes.contains(&Subtype::Zombie));
    assert!(create.clear_mana_cost);

    let lines = unprocessed_compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains("Embalm {5}{W}"),
        "expected embalm keyword label in compiled text, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_eternalize_keyword_line_renders_keyword_activation() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Eternalize Test")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie, Subtype::Jackal])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text("Eternalize {4}{W}{W} ({4}{W}{W}, Exile this card from your graveyard: Create a token that's a copy of it, except it's a 4/4 black Zombie Jackal with no mana cost. Eternalize only as a sorcery.)")
        .expect("eternalize keyword line should parse");

    let (ability, activated) = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some((ability, activated)),
            _ => None,
        })
        .expect("eternalize should compile to an activated ability");

    assert_eq!(ability.functional_zones, vec![Zone::Graveyard]);
    assert_eq!(activated.choices.len(), 0);

    let flattened = activated.effects.flattened_default_effects();
    let [effect] = flattened else {
        panic!("eternalize should have exactly one default effect");
    };
    let create = effect
        .downcast_ref::<crate::effects::CreateTokenCopyEffect>()
        .expect("eternalize should create a token copy");
    assert!(matches!(&create.target, ChooseSpec::Source));
    assert_eq!(create.count, Value::Fixed(1));
    assert_eq!(create.controller, PlayerFilter::You);
    assert_eq!(create.set_colors, Some(crate::color::ColorSet::BLACK));
    assert!(create.added_subtypes.contains(&Subtype::Zombie));
    assert_eq!(create.set_base_power_toughness, Some((4, 4)));
    assert!(create.clear_mana_cost);

    let rendered = crate::compiled_text::ability_surface_text(ability);
    assert_eq!(rendered, "Eternalize {4}{W}{W}");

    let joined = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        joined.contains("Eternalize {4}{W}{W}"),
        "expected eternalize keyword label in compiled text, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn eternalize_keeps_an_authored_additional_activation_cost() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Eternalize Additional Cost Test")
        .card_types(vec![CardType::Creature])
        .parse_text("Eternalize—{2}{W}{W}, Discard a card.")
        .expect("eternalize with an additional discard cost should parse");

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("eternalize should compile to an activated ability");
    let cost_debug = format!("{:#?}", activated.mana_cost);
    assert!(cost_debug.contains("DiscardEffect"), "{cost_debug}");
    assert!(cost_debug.contains("ExileEffect"), "{cost_debug}");
    assert_eq!(
        unprocessed_compiled_lines(&def),
        vec!["Eternalize {2}{W}{W}, Discard a card.".to_string()]
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_emerge_keyword_line_compiles_to_hand_alternative_cast() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Emerge Test")
        .card_types(vec![CardType::Creature])
        .parse_text("Emerge {5}{U} (You may cast this spell by sacrificing a creature and paying the emerge cost reduced by that creature's mana value.)")
        .expect("emerge keyword line should parse");

    assert_eq!(def.alternative_casts.len(), 1);
    match &def.alternative_casts[0] {
        AlternativeCastingMethod::Composed {
            name, total_cost, ..
        } => {
            assert_eq!(*name, "Emerge");
            let cost = total_cost
                .mana_cost()
                .expect("emerge should include mana cost");
            assert_eq!(cost.to_oracle(), "{5}{U}");
            let costs = def.alternative_casts[0].non_mana_costs();
            assert_eq!(costs.len(), 1);
            let filter = costs[0]
                .sacrifice_filter()
                .expect("emerge should include a sacrifice cost");
            assert!(filter.card_types.contains(&CardType::Creature));
            assert_eq!(filter.controller, Some(PlayerFilter::You));
        }
        other => panic!("expected emerge composed alternative cast, got {other:?}"),
    }

    let lines = unprocessed_compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains("Emerge {5}{U}"),
        "expected emerge keyword label in compiled text, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_scavenge_uses_source_snapshot_power_after_source_is_exiled() {
    use crate::ability::AbilityKind;
    use crate::effects::{ExecutionContext, ResolvedTarget, execute_effect};
    use crate::snapshot::ObjectSnapshot;
    use crate::zone::Zone;

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);

    let scavenge_def = CardDefinitionBuilder::new(CardId::new(), "Scavenge Test")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zombie])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text("Scavenge {2}{G}")
        .expect("scavenge keyword line should parse");
    let source = game.create_object_from_definition(&scavenge_def, alice, Zone::Graveyard);

    let target_def = CardDefinitionBuilder::new(CardId::new(), "Target Grizzly")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Bear])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let target = game.create_object_from_definition(&target_def, alice, Zone::Battlefield);

    let effect = scavenge_def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated.effects.first().cloned(),
            _ => None,
        })
        .expect("scavenge activated effect should exist");

    let source_snapshot =
        ObjectSnapshot::from_object(game.object(source).expect("source object exists"), &game);
    let moved_source = game
        .move_object_by_effect(source, Zone::Exile)
        .expect("source should move to exile to simulate paying scavenge");
    assert!(
        game.object(moved_source)
            .is_some_and(|obj| obj.zone == Zone::Exile),
        "source should be exiled after paying scavenge"
    );

    let mut ctx = ExecutionContext::new_default(source, alice)
        .with_targets(vec![ResolvedTarget::Object(target)])
        .with_source_snapshot(source_snapshot);
    execute_effect(&mut game, &effect, &mut ctx).expect("scavenge effect should resolve");

    let target_obj = game.object(target).expect("target creature exists");
    assert_eq!(
        target_obj
            .counters
            .get(&crate::object::CounterType::PlusOnePlusOne),
        Some(&4),
        "scavenge should use the exiled source card's power via source snapshot"
    );
}

#[test]
pub(super) fn unprocessed_compiled_lines_render_bannerhide_krushok_keywords_and_clear_similarity_floor()
 {
    let def = parse_oracle_card_definition("Bannerhide Krushok");
    let lines = unprocessed_compiled_lines(&def);
    let joined = lines.join("\n");

    assert!(
        joined.contains("Reinforce 2—{1}{G}"),
        "expected reinforce keyword label in canonical compiled text, got {joined}"
    );
    assert!(
        joined.contains("Scavenge {5}{G}{G}"),
        "expected scavenge keyword label in canonical compiled text, got {joined}"
    );

    let (_oracle_cov, _compiled_cov, similarity, _delta, _mismatch) =
        crate::semantic_compare::compare_card_semantics_scored(
            "Bannerhide Krushok",
            &crate::compiled_text::debug_compiled_lines(&def).join("\n"),
            &lines,
            crate::semantic_compare::report_embedding_config(),
        );
    assert!(
        similarity >= 0.95,
        "expected Bannerhide Krushok to clear the 0.95 similarity floor after keyword rendering, got {similarity}"
    );
}

#[test]
pub(super) fn unprocessed_compiled_lines_compact_expanded_mechanic_asts() {
    let storm = CardDefinitionBuilder::new(CardId::from_raw(1), "Storm Probe")
        .card_types(vec![CardType::Instant])
        .with_spell_effect(vec![Effect::gain_life(3)])
        .storm()
        .build();
    let storm_rendered = unprocessed_compiled_lines(&storm).join("\n");
    assert!(
        storm_rendered.contains("Storm")
            && !storm_rendered.contains("copy this spell for each spell cast before it this turn"),
        "expected storm AST to render as the compact keyword, got {storm_rendered}"
    );

    let mobilize = CardDefinitionBuilder::new(CardId::from_raw(2), "Mobilize Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(0, 4))
        .vigilance()
        .mobilize(3)
        .build();
    let mobilize_rendered = unprocessed_compiled_lines(&mobilize).join("\n");
    assert!(
        mobilize_rendered
            .to_ascii_lowercase()
            .contains("mobilize 3")
            && !mobilize_rendered.contains("Warrior creature tokens that are tapped and attacking"),
        "expected mobilize AST to render as the compact keyword, got {mobilize_rendered}"
    );

    let soulbond = CardDefinitionBuilder::new(CardId::from_raw(3), "Soulbond Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .soulbond()
        .build();
    let soulbond_rendered = unprocessed_compiled_lines(&soulbond).join("\n");
    assert_eq!(
        soulbond_rendered, "Soulbond",
        "expected soulbond AST to render as the compact keyword"
    );

    let undaunted = CardDefinitionBuilder::new(CardId::from_raw(4), "Undaunted Probe")
        .card_types(vec![CardType::Instant])
        .undaunted()
        .build();
    let undaunted_rendered = unprocessed_compiled_lines(&undaunted).join("\n");
    assert_eq!(
        undaunted_rendered, "Undaunted",
        "expected undaunted AST to render as the compact keyword"
    );

    let suspend = CardDefinitionBuilder::new(CardId::from_raw(5), "Suspend Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 5))
        .suspend(5, ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .build();
    let suspend_rendered = unprocessed_compiled_lines(&suspend).join("\n");
    assert!(
        suspend_rendered.contains("Suspend 5")
            && suspend_rendered.contains("{G}")
            && !suspend_rendered
                .to_ascii_lowercase()
                .contains("time counter")
            && !suspend_rendered
                .to_ascii_lowercase()
                .contains("cast this card from exile"),
        "expected suspend AST to render only the compact keyword line, got {suspend_rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_mobilize_keyword_line_compiles_to_attack_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mobilize Test")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Warrior])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text("Mobilize 2")
        .expect("mobilize keyword line should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ThisAttacksTrigger")
            && debug.contains("CreateTokenEffect")
            && debug.contains("sacrifice_at_next_end_step: true"),
        "expected mobilize attack trigger lowering, got {debug}"
    );
    assert!(
        !debug.contains("MarkerText(\"Mobilize 2\")"),
        "mobilize should not fall back to marker text, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_mobilize_trigger_creates_attacking_warriors() {
    use crate::ability::AbilityKind;
    use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::zone::Zone;

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let def = CardDefinitionBuilder::new(CardId::new(), "Mobilize Test")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Warrior])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text("Mobilize 2")
        .expect("mobilize keyword line should parse");
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    game.combat = Some(CombatState {
        attackers: vec![AttackerInfo {
            creature: source,
            target: AttackTarget::Player(bob),
        }],
        ..CombatState::default()
    });

    let effects = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered.trigger.display().contains("attacks") =>
            {
                Some(triggered.effects.clone())
            }
            _ => None,
        })
        .expect("mobilize trigger should exist");

    let mut ctx = ExecutionContext::new_default(source, alice);
    for effect in &effects {
        execute_effect(&mut game, effect, &mut ctx).expect("mobilize effect should resolve");
    }

    let warrior_ids: Vec<_> = game
        .battlefield
        .iter()
        .copied()
        .filter(|&id| {
            game.object(id)
                .is_some_and(|obj| game.controller_of(obj) == alice && obj.name == "Warrior")
        })
        .collect();
    assert_eq!(warrior_ids.len(), 2, "expected two mobilize tokens");
    assert!(warrior_ids.iter().all(|&id| game.is_tapped(id)));

    let combat = game.combat.as_ref().expect("combat should exist");
    let warrior_attackers = combat
        .attackers
        .iter()
        .filter(|attacker| warrior_ids.contains(&attacker.creature))
        .count();
    assert_eq!(
        warrior_attackers, 2,
        "mobilize tokens should enter attacking"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_named_counter_types_fall_back_to_named_counter() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Named Counter Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text("At the beginning of your upkeep, put a spore counter on this creature.")
        .expect("named counter types should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("Named(\"spore\")"),
        "expected CounterType::Named(\"spore\") in parsed ability, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_plus_zero_plus_one_counter_type() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "PT Counter Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Put a +0/+1 counter on target creature.")
        .expect("+0/+1 counter type should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("PlusZeroPlusOne"),
        "expected +0/+1 to map to CounterType::PlusZeroPlusOne, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_switch_power_toughness_until_eot() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Switch Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 4))
        .parse_text("{U}: Switch this creature's power and toughness until end of turn.")
        .expect("switch P/T clause should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("SwitchPowerToughness"),
        "expected continuous switch P/T modification, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_suspend_keyword_line_with_reminder_text_keeps_suspend_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Suspend Probe")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Suspend 3—{0} (Rather than cast this card from your hand, pay {0} and exile it with three time counters on it. At the beginning of your upkeep, remove a time counter. When the last is removed, you may cast it without paying its mana cost.)",
        )
        .expect("suspend keyword with reminder text should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("suspend 3—{0}") || rendered.contains("suspend 3 {0}"),
        "expected suspend keyword text in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "suspend keyword line should not rely on unsupported fallback marker: {rendered}"
    );
    assert_eq!(def.alternative_casts.len(), 1);
    match &def.alternative_casts[0] {
        AlternativeCastingMethod::Suspend { cost, time } => {
            assert_eq!(*time, 3);
            assert_eq!(cost.to_oracle(), "{0}");
        }
        other => panic!("expected suspend metadata, got {other:?}"),
    }
    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        !debug.contains("unsupported"),
        "suspend parse should avoid unsupported placeholders, got {debug}"
    );
}

#[test]
pub(super) fn jhoira_of_the_ghitu_strict_parser_text_and_suspend_grant_regression() {
    assert_oracle_card_parses_strict("Jhoira of the Ghitu");

    let def = parse_oracle_card_definition("Jhoira of the Ghitu");
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert_eq!(
        rendered,
        "{2}, Exile a nonland card from your hand: Put four time counters on the exiled card. If it doesn't have suspend, it gains suspend.",
        "Jhoira should keep the exact tagged-card suspend condition"
    );

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("ExileEffect")
            && debug.contains("excluded_card_types")
            && debug.contains("Land")
            && debug.contains("PutCountersEffect")
            && debug.contains("presentation_label")
            && debug.contains("Keyword")
            && debug.contains("Suspend")
            && debug.contains("AddAbilityGeneric"),
        "expected Jhoira to lower exile-from-hand, time counters, and real suspend triggers, got {debug}"
    );
    assert!(
        !debug.contains("KeywordFallbackText") && !debug.contains("unsupported"),
        "Jhoira should not rely on unsupported fallback text, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn doom_time_platform_exiles_with_time_counters_before_granting_suspend() {
    assert_oracle_card_parses_strict("Doom's Time Platform");

    let def = parse_oracle_card_definition("Doom's Time Platform");
    let debug = format!("{def:?}");
    assert!(
        debug.contains("MoveToZoneEffect")
            && debug.contains("zone: Exile")
            && debug.contains("PutCountersEffect")
            && debug.contains("counter_type: Time")
            && debug.contains("amount: Fixed(2)")
            && debug.contains("alternative_cast: Some(Suspend)"),
        "expected exile, two time counters, and the conditional suspend grant, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "exile target nonland card from your graveyard with two time counters on it. If it doesn't have suspend, it gains suspend"
        ),
        "expected compact exile-with-counters suspend wording, got {rendered}"
    );
    assert!(
        !rendered.contains("Then if not"),
        "suspend implementation details should stay structural, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sibylline_soothsayer_exiles_consult_match_with_time_counters() {
    assert_oracle_card_parses_strict("Sibylline Soothsayer");

    let def = parse_oracle_card_definition("Sibylline Soothsayer");
    let debug = format!("{def:?}");
    assert!(
        debug.contains("ConsultTopOfLibraryEffect")
            && debug.contains("mode: Reveal")
            && debug.contains("PutCountersEffect")
            && debug.contains("counter_type: Time")
            && debug.contains("amount: Fixed(3)")
            && debug.contains("PutTaggedRemainderOnLibraryBottomEffect"),
        "expected reveal consult, exile, three time counters, and random bottoming, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("greater. Exile that card with three time counters on it")
            && rendered.contains(
            "Exile that card with three time counters on it. If it doesn't have suspend, it gains suspend"
        ) && rendered.contains(
            "Put the rest of the revealed cards on the bottom of your library in a random order"
        ),
        "expected compact matched-card suspend setup and revealed remainder wording, got {rendered}"
    );
    assert!(
        !rendered.contains("Then if not")
            && !rendered.contains("greater, exile that card, and put")
            && !rendered.contains("remaining tagged cards"),
        "consult and suspend internals should not leak, got {rendered}"
    );
}

#[test]
pub(super) fn taigam_flurry_exiles_copied_spell_with_suspend_setup() {
    assert_oracle_card_parses_strict("Taigam, Master Opportunist");

    let def = parse_oracle_card_definition("Taigam, Master Opportunist");
    let rendered = compiled_text_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "Flurry — Whenever you cast your second spell each turn, copy it, then exile the spell you cast with four time counters on it. If it doesn't have suspend, it gains suspend"
        ),
        "expected Taigam's Flurry text to render compactly, got {rendered}"
    );

    let debug = format!("{:?}", def);
    assert!(
        debug.contains("CopySpellEffect")
            && debug.contains("target: Tagged")
            && debug.contains("MoveToZoneEffect")
            && debug.contains("zone: Exile")
            && debug.contains("PutCountersEffect")
            && debug.contains("counter_type: Time")
            && debug.contains("amount: Fixed(4)")
            && debug.contains("Keyword(Suspend)")
            && debug.contains("AddAbilityGeneric"),
        "expected Taigam to copy, exile, add time counters, and grant real suspend triggers, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_the_face_of_boe_suspend_cost_activation() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(73_100), "The Face of Boe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::White],
        ]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Alien, Subtype::Advisor])
        .power_toughness(PowerToughness::fixed(0, 4))
        .parse_text(
            "{T}: You may cast a spell with suspend from your hand. If you do, pay its suspend cost rather than its mana cost. Activate only as a sorcery.",
        )
        .expect("The Face of Boe should parse strictly");

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("You may cast a spell with suspend from your hand. If you do, pay its suspend cost rather than its mana cost"),
        "expected compiled text to preserve the suspend-cost cast clause, got {rendered}"
    );
    assert!(
        rendered.contains("Activate only as a sorcery"),
        "expected compiled text to preserve sorcery-speed activation, got {rendered}"
    );

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("The Face of Boe should have an activated ability");
    assert!(matches!(
        activated.timing,
        crate::ability::ActivationTiming::SorcerySpeed
    ));
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("MayCastMatchingSpellWithoutPayingManaCostEffect")
            && debug.contains("AlternativeCost")
            && debug.contains("Suspend"),
        "expected The Face of Boe to lower to a suspend-cost one-shot cast effect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_compile_lotus_bloom_raw_definition_keeps_suspend_and_no_mana_cost() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Lotus Bloom")
        .parse_text(
            "Type: Artifact\n\
             Suspend 3—{0} (Rather than cast this card from your hand, pay {0} and exile it with three time counters on it. At the beginning of your upkeep, remove a time counter. When the last is removed, you may cast it without paying its mana cost.)\n\
             {T}, Sacrifice this artifact: Add three mana of any one color.",
        )
        .expect("Lotus Bloom raw definition should compile");

    assert!(
        def.card.mana_cost.is_none(),
        "Lotus Bloom should keep its missing mana cost"
    );
    assert_eq!(def.card.card_types, vec![CardType::Artifact]);
    assert_eq!(def.alternative_casts.len(), 1);
    match &def.alternative_casts[0] {
        AlternativeCastingMethod::Suspend { cost, time } => {
            assert_eq!(*time, 3);
            assert_eq!(cost.to_oracle(), "{0}");
        }
        other => panic!("expected Lotus Bloom suspend metadata, got {other:?}"),
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_plot_keyword_line_compiles_to_alternative_cast() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Plot Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Plot {2}{R} (You may pay {2}{R} and exile this card from your hand. Cast it as a sorcery on a later turn without paying its mana cost. Plot only as a sorcery.)",
        )
        .expect("plot keyword line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Plot {2}{R}"),
        "expected plot cost in render output, got {rendered}"
    );
    assert_eq!(def.alternative_casts.len(), 1);
    match &def.alternative_casts[0] {
        AlternativeCastingMethod::Plot { cost } => {
            assert_eq!(cost.to_oracle(), "{2}{R}");
        }
        other => panic!("expected plot alternative cast, got {other:?}"),
    }
    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        !debug.contains("unsupported"),
        "plot parse should avoid unsupported placeholders, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_foretell_keyword_line_compiles_to_alternative_cast() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Foretell Probe")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Foretell {1}{U} (During your turn, you may pay {2} and exile this card from your hand face down. Cast it on a later turn for its foretell cost.)",
        )
        .expect("foretell keyword line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Foretell {1}{U}"),
        "expected foretell cost in render output, got {rendered}"
    );
    assert_eq!(def.alternative_casts.len(), 1);
    match &def.alternative_casts[0] {
        AlternativeCastingMethod::Foretell { cost } => {
            assert_eq!(cost.to_oracle(), "{1}{U}");
        }
        other => panic!("expected foretell alternative cast, got {other:?}"),
    }
    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        !debug.contains("unsupported"),
        "foretell parse should avoid unsupported placeholders, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_harmonize_keyword_line_compiles_to_alternative_cast() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Harmonize Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Search your library for a creature card with mana value X or less, put it onto the battlefield, then shuffle.\nHarmonize {X}{G}{G}{G}{G} (You may cast this card from your graveyard for its harmonize cost. You may tap a creature you control to reduce that cost by an amount of generic mana equal to its power. Then exile this spell.)",
        )
        .expect("harmonize keyword line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Harmonize {X}{G}{G}{G}{G}"),
        "expected harmonize cost in render output, got {rendered}"
    );
    assert_eq!(def.alternative_casts.len(), 1);
    match &def.alternative_casts[0] {
        AlternativeCastingMethod::Harmonize { total_cost } => {
            assert_eq!(
                total_cost
                    .mana_cost()
                    .expect("harmonize should keep mana cost")
                    .to_oracle(),
                "{X}{G}{G}{G}{G}"
            );
        }
        other => panic!("expected harmonize alternative cast, got {other:?}"),
    }
    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        !debug.contains("unsupported"),
        "harmonize parse should avoid unsupported placeholders, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_spectacle_keyword_line_compiles_to_alternative_cast() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Spectacle Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Spectacle {R} (You may cast this spell for its spectacle cost rather than its mana cost if an opponent lost life this turn.)",
        )
        .expect("spectacle keyword line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Spectacle") || rendered.contains("opponent lost life this turn"),
        "expected spectacle render output, got {rendered}"
    );
    assert_eq!(def.alternative_casts.len(), 1);
    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        debug.contains("spectacle") && !debug.contains("unsupported"),
        "spectacle parse should lower to a real alternative cost, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_rix_maadi_reveler_parses_with_spectacle_paid_predicate() {
    let def = parse_oracle_card_definition("Rix Maadi Reveler");
    let debug = format!("{def:#?}");

    assert!(
        !debug.to_ascii_lowercase().contains("unsupported"),
        "expected Rix Maadi Reveler to parse without unsupported placeholders, got {debug}"
    );
    assert!(
        debug.contains("condition: ThisSpellPaidLabel(") && debug.contains("\"Spectacle\""),
        "expected spectacle-paid predicate in compiled definition, got {debug}"
    );

    let rendered = canonical_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("discard a card, then draw a card")
            && rendered.contains(
                "If this spell's spectacle cost was paid, discard your hand, then draw three cards instead"
            ),
        "expected Rix Maadi Reveler compiled text to keep both ETB branches, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_rix_maadi_reveler_etb_uses_base_branch_when_spectacle_not_paid() {
    use crate::ability::AbilityKind;
    use crate::condition_eval::evaluate_condition_resolution;
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::tests::test_helpers::setup_two_player_game;
    use crate::zone::Zone;

    let mut game = setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let def = parse_oracle_card_definition("Rix Maadi Reveler");

    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let filler = CardDefinitionBuilder::new(CardId::new(), "Filler")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();

    for _ in 0..2 {
        game.create_object_from_definition(&filler, alice, Zone::Hand);
    }
    for _ in 0..3 {
        game.create_object_from_definition(&filler, alice, Zone::Library);
    }

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) if triggered.trigger.display().contains("enters") => {
                Some(triggered.clone())
            }
            _ => None,
        })
        .expect("Rix Maadi Reveler ETB trigger should exist");

    let mut ctx = ExecutionContext::new_default(source, alice);
    let segment = &triggered.effects.segments[0];
    let replacement = &segment.self_replacements[0];
    assert!(
        !evaluate_condition_resolution(&game, &replacement.condition, &ctx)
            .expect("condition evaluation should succeed"),
        "spectacle branch should be false when spectacle was not paid"
    );
    for effect in &segment.default_effects {
        execute_effect(&mut game, effect, &mut ctx).expect("Rix Maadi Reveler ETB should resolve");
    }

    assert_eq!(
        game.player(alice).expect("player exists").hand.len(),
        2,
        "without spectacle payment, ETB should discard 1 then draw 1"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_rix_maadi_reveler_etb_uses_spectacle_branch_when_paid() {
    use crate::ability::AbilityKind;
    use crate::condition_eval::evaluate_condition_resolution;
    use crate::cost::OptionalCostsPaid;
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::tests::test_helpers::setup_two_player_game;
    use crate::zone::Zone;

    let mut game = setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let def = parse_oracle_card_definition("Rix Maadi Reveler");

    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let filler = CardDefinitionBuilder::new(CardId::new(), "Filler")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();

    for _ in 0..2 {
        game.create_object_from_definition(&filler, alice, Zone::Hand);
    }
    for _ in 0..4 {
        game.create_object_from_definition(&filler, alice, Zone::Library);
    }

    let paid = OptionalCostsPaid {
        costs: vec![("Spectacle".into(), 1)],
        cast_at_sorcery_timing: false,
    };
    game.object_mut(source)
        .expect("source object exists")
        .optional_costs_paid = paid.clone();

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) if triggered.trigger.display().contains("enters") => {
                Some(triggered.clone())
            }
            _ => None,
        })
        .expect("Rix Maadi Reveler ETB trigger should exist");

    let mut ctx = ExecutionContext::new_default(source, alice).with_optional_costs_paid(paid);
    let segment = &triggered.effects.segments[0];
    let replacement = &segment.self_replacements[0];
    assert!(
        evaluate_condition_resolution(&game, &replacement.condition, &ctx)
            .expect("condition evaluation should succeed"),
        "spectacle branch should be true when spectacle was paid"
    );
    for effect in &replacement.replacement_effects {
        execute_effect(&mut game, effect, &mut ctx).expect("Rix Maadi Reveler ETB should resolve");
    }

    assert_eq!(
        game.player(alice).expect("player exists").hand.len(),
        3,
        "with spectacle payment, ETB should discard hand then draw three"
    );
}

#[test]
pub(super) fn flycatcher_giraffid_strict_parser_compiled_text_and_model_regression() {
    assert_oracle_card_parses_strict("Flycatcher Giraffid");
    let def = parse_oracle_card_definition("Flycatcher Giraffid");

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("EntersWithCounterChoice")
            && debug.contains("Vigilance")
            && debug.contains("Reach")
            && !debug.contains("KeywordFallbackText")
            && !debug.contains("RuleFallbackText")
            && !debug.contains("UnsupportedParserLine"),
        "expected Flycatcher Giraffid to lower its ETB counter choice without unsupported placeholders, got {debug}"
    );

    let rendered = compiled_text_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "This creature enters with your choice of a vigilance counter or a reach counter on it"
        ),
        "expected Flycatcher Giraffid compiled text to preserve its counter-choice clause, got {rendered}"
    );
}

#[test]
pub(super) fn flycatcher_giraffid_enters_with_chosen_vigilance_counter() {
    use crate::tests::test_helpers::setup_two_player_game;

    struct ChooseCounter(usize);
    impl crate::decision::DecisionMaker for ChooseCounter {
        fn decide_options(
            &mut self,
            _game: &crate::game_state::GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            assert!(
                ctx.description.contains("Flycatcher Giraffid"),
                "counter choice should name the entering creature, got {:?}",
                ctx.description
            );
            assert_eq!(ctx.min, 1);
            assert_eq!(ctx.max, 1);
            assert_eq!(ctx.options.len(), 2);
            vec![self.0]
        }
    }

    let def = parse_oracle_card_definition("Flycatcher Giraffid");
    let mut game = setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let stack_id = game.create_object_from_definition(&def, alice, Zone::Stack);
    let mut dm = ChooseCounter(0);

    let result = game
        .move_object_with_etb_processing_with_dm(stack_id, Zone::Battlefield, &mut dm)
        .expect("Flycatcher Giraffid should enter the battlefield");
    let giraffid_id = result.new_id;

    assert_eq!(
        game.counter_count(giraffid_id, crate::object::CounterType::Vigilance),
        1,
        "choosing the first option should put one vigilance counter on Flycatcher Giraffid"
    );
    assert_eq!(
        game.counter_count(giraffid_id, crate::object::CounterType::Reach),
        0,
        "choosing vigilance should not also put a reach counter on Flycatcher Giraffid"
    );
    let chars = game
        .calculated_characteristics(giraffid_id)
        .expect("Flycatcher Giraffid should have calculated characteristics");
    assert!(
        chars
            .static_abilities
            .iter()
            .any(|ability| ability.id() == StaticAbilityId::Vigilance),
        "a vigilance counter should grant vigilance to Flycatcher Giraffid"
    );
    assert!(
        chars
            .static_abilities
            .iter()
            .all(|ability| ability.id() != StaticAbilityId::Reach),
        "choosing vigilance should not grant reach to Flycatcher Giraffid"
    );
}

#[test]
pub(super) fn flycatcher_giraffid_enters_with_chosen_reach_counter() {
    use crate::tests::test_helpers::setup_two_player_game;

    struct ChooseCounter(usize);
    impl crate::decision::DecisionMaker for ChooseCounter {
        fn decide_options(
            &mut self,
            _game: &crate::game_state::GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            assert!(
                ctx.options
                    .iter()
                    .any(|option| option.description == "reach counter"),
                "counter choice should include a reach-counter option, got {:?}",
                ctx.options
            );
            vec![self.0]
        }
    }

    let def = parse_oracle_card_definition("Flycatcher Giraffid");
    let mut game = setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let stack_id = game.create_object_from_definition(&def, alice, Zone::Stack);
    let mut dm = ChooseCounter(1);

    let result = game
        .move_object_with_etb_processing_with_dm(stack_id, Zone::Battlefield, &mut dm)
        .expect("Flycatcher Giraffid should enter the battlefield");
    let giraffid_id = result.new_id;

    assert_eq!(
        game.counter_count(giraffid_id, crate::object::CounterType::Reach),
        1,
        "choosing the second option should put one reach counter on Flycatcher Giraffid"
    );
    assert_eq!(
        game.counter_count(giraffid_id, crate::object::CounterType::Vigilance),
        0,
        "choosing reach should not also put a vigilance counter on Flycatcher Giraffid"
    );
    let chars = game
        .calculated_characteristics(giraffid_id)
        .expect("Flycatcher Giraffid should have calculated characteristics");
    assert!(
        chars
            .static_abilities
            .iter()
            .any(|ability| ability.id() == StaticAbilityId::Reach),
        "a reach counter should grant reach to Flycatcher Giraffid"
    );
    assert!(
        chars
            .static_abilities
            .iter()
            .all(|ability| ability.id() != StaticAbilityId::Vigilance),
        "choosing reach should not grant vigilance to Flycatcher Giraffid"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn thunder_brute_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Thunder Brute");
    let def = parse_oracle_card_definition("Thunder Brute");

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("Tribute")
            && debug.contains("Tribute(3)")
            && debug.contains("ThisSpellPaidLabel")
            && !debug.contains("KeywordMarker")
            && !debug.contains("KeywordFallbackText")
            && !debug.contains("RuleFallbackText")
            && !debug.contains("UnsupportedParserLine"),
        "expected Thunder Brute to lower tribute and its paid condition without unsupported placeholders, got {debug}"
    );

    let rendered = canonical_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Tribute 3")
            && rendered.contains("When this creature enters, if tribute wasn't paid")
            && rendered.contains("it gains haste until end of turn"),
        "expected Thunder Brute compiled text to preserve tribute and the unpaid branch, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn thunder_brute_etb_grants_haste_when_tribute_not_paid() {
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::tests::test_helpers::setup_two_player_game;

    struct DeclineTribute;
    impl crate::decision::DecisionMaker for DeclineTribute {}

    let def = parse_oracle_card_definition("Thunder Brute");
    let mut game = setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let stack_id = game.create_object_from_definition(&def, alice, Zone::Stack);
    let mut dm = DeclineTribute;
    let result = game
        .move_object_with_etb_processing_with_dm(stack_id, Zone::Battlefield, &mut dm)
        .expect("Thunder Brute should enter the battlefield");
    let thunder_brute_id = result.new_id;

    assert_eq!(
        game.counter_count(thunder_brute_id, crate::object::CounterType::PlusOnePlusOne),
        0,
        "Thunder Brute should not get tribute counters when the opponent declines"
    );
    assert!(
        !game
            .object(thunder_brute_id)
            .expect("Thunder Brute exists")
            .optional_costs_paid
            .was_paid_label("Tribute"),
        "tribute should not be marked paid when the opponent declines"
    );

    let snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(thunder_brute_id).expect("Thunder Brute exists"),
        &game,
    );
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::with_cause(
            thunder_brute_id,
            Zone::Stack,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            Some(snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    );

    let triggered = crate::triggers::check_triggers(&game, &event);
    assert_eq!(
        triggered.len(),
        1,
        "Thunder Brute should trigger when tribute was not paid"
    );

    let entry = &triggered[0];
    let mut ctx = ExecutionContext::new_default(thunder_brute_id, alice)
        .with_triggering_event(entry.triggering_event.clone());
    for effect in &entry.ability.effects {
        execute_effect(&mut game, effect, &mut ctx).expect("Thunder Brute ETB should resolve");
    }

    assert!(
        game.current_has_static_ability_id(thunder_brute_id, StaticAbilityId::Haste),
        "Thunder Brute should gain haste until end of turn when tribute was not paid"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn thunder_brute_etb_does_not_trigger_when_tribute_paid() {
    use crate::tests::test_helpers::setup_two_player_game;

    struct AcceptTribute;
    impl crate::decision::DecisionMaker for AcceptTribute {
        fn decide_boolean(
            &mut self,
            _game: &crate::game_state::GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            true
        }
    }

    let def = parse_oracle_card_definition("Thunder Brute");
    let mut game = setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let graveyard_id = game.create_object_from_definition(&def, alice, Zone::Graveyard);
    let mut dm = AcceptTribute;
    let result = game
        .move_object_with_etb_processing_with_dm(graveyard_id, Zone::Battlefield, &mut dm)
        .expect("Thunder Brute should enter the battlefield");
    let thunder_brute_id = result.new_id;

    assert_eq!(
        game.counter_count(thunder_brute_id, crate::object::CounterType::PlusOnePlusOne),
        3,
        "Thunder Brute should enter with three tribute counters when the opponent accepts"
    );
    assert!(
        game.object(thunder_brute_id)
            .expect("Thunder Brute exists")
            .optional_costs_paid
            .was_paid_label("Tribute"),
        "tribute should be marked paid when the opponent accepts"
    );

    let snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(thunder_brute_id).expect("Thunder Brute exists"),
        &game,
    );
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::with_cause(
            thunder_brute_id,
            Zone::Graveyard,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            Some(snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    );

    let triggered = crate::triggers::check_triggers(&game, &event);
    assert!(
        triggered.is_empty(),
        "Thunder Brute should not trigger when tribute was paid"
    );
    assert!(
        !game.current_has_static_ability_id(thunder_brute_id, StaticAbilityId::Haste),
        "Thunder Brute should not gain haste when tribute was paid"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn thunder_brute_tribute_uses_controller_chosen_opponent() {
    struct ChooseCharlieThenAccept {
        chose_opponent: bool,
        prompted_chosen_opponent: bool,
    }

    impl crate::decision::DecisionMaker for ChooseCharlieThenAccept {
        fn decide_options(
            &mut self,
            _game: &crate::game_state::GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            assert_eq!(ctx.player, PlayerId::from_index(0));
            self.chose_opponent = true;
            let charlie = ctx
                .options
                .iter()
                .find(|option| option.description == "Charlie")
                .expect("Charlie should be a tribute opponent option");
            vec![charlie.index]
        }

        fn decide_boolean(
            &mut self,
            _game: &crate::game_state::GameState,
            ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.prompted_chosen_opponent = ctx.player == PlayerId::from_index(2);
            self.prompted_chosen_opponent
        }
    }

    let def = parse_oracle_card_definition("Thunder Brute");
    let mut game = crate::game_state::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );
    let alice = PlayerId::from_index(0);
    let stack_id = game.create_object_from_definition(&def, alice, Zone::Stack);
    let mut dm = ChooseCharlieThenAccept {
        chose_opponent: false,
        prompted_chosen_opponent: false,
    };

    let result = game
        .move_object_with_etb_processing_with_dm(stack_id, Zone::Battlefield, &mut dm)
        .expect("Thunder Brute should enter the battlefield");
    let thunder_brute_id = result.new_id;

    assert!(
        dm.chose_opponent,
        "controller should choose a tribute opponent"
    );
    assert!(
        dm.prompted_chosen_opponent,
        "chosen opponent should receive the tribute payment choice"
    );
    assert_eq!(
        game.counter_count(thunder_brute_id, crate::object::CounterType::PlusOnePlusOne),
        3,
        "Thunder Brute should get counters when the chosen opponent accepts tribute"
    );
    assert!(
        game.object(thunder_brute_id)
            .expect("Thunder Brute exists")
            .optional_costs_paid
            .was_paid_label("Tribute"),
        "tribute should be marked paid when the chosen opponent accepts"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_disturb_keyword_line_compiles_to_alternative_cast() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Disturb Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Disturb {1}{W} (You may cast this card from your graveyard transformed for its disturb cost.)",
        )
        .expect("disturb keyword line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Disturb {1}{W}"),
        "expected disturb render output, got {rendered}"
    );
    assert_eq!(def.alternative_casts.len(), 1);
    match &def.alternative_casts[0] {
        AlternativeCastingMethod::Disturb { cost } => {
            assert_eq!(cost.to_oracle(), "{1}{W}");
        }
        other => panic!("expected disturb alternative cast, got {other:?}"),
    }
    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        !debug.contains("unsupported"),
        "disturb parse should avoid unsupported placeholders, got {debug}"
    );

    let compact = CardDefinitionBuilder::new(CardId::from_raw(2), "Compact Disturb Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Disturb {1}{W}.")
        .expect("compact disturb keyword line should parse");
    assert_eq!(compact.alternative_casts.len(), 1);
    match &compact.alternative_casts[0] {
        AlternativeCastingMethod::Disturb { cost } => {
            assert_eq!(cost.to_oracle(), "{1}{W}");
        }
        other => panic!("expected compact disturb alternative cast, got {other:?}"),
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_cipher_keyword_line_compiles_to_real_spell_effect() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cipher Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Draw a card.\nCipher")
        .expect("cipher keyword line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Cipher"),
        "expected cipher render output, got {rendered}"
    );
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("CipherEffect"),
        "expected cipher to compile to a first-class effect, got {debug}"
    );
    assert!(
        !debug.contains("KeywordMarker"),
        "cipher should not remain a keyword marker, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_overload_keyword_line_compiles_to_alternative_cast_and_rewritten_effects()
{
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Overload Probe")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Target creature you control gets +1/+0 until end of turn.\nOverload {1}{R} (You may cast this spell for its overload cost. If you do, change \"target\" in its text to \"each.\")",
        )
        .expect("overload keyword line should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Overload {1}{R}"),
        "expected overload render output, got {rendered}"
    );
    assert_eq!(def.alternative_casts.len(), 1);
    match &def.alternative_casts[0] {
        AlternativeCastingMethod::Overload { cost, effects } => {
            assert_eq!(cost.to_oracle(), "{1}{R}");
            assert!(
                !effects.is_empty(),
                "expected overload to compile a rewritten effect tree"
            );
            let debug = format!("{effects:?}");
            assert!(
                !debug.contains("Target("),
                "expected overloaded effects to be non-targeted, got {debug}"
            );
        }
        other => panic!("expected overload alternative cast, got {other:?}"),
    }
    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        !debug.contains("unsupported"),
        "overload parse should avoid unsupported placeholders, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_evolving_door_compiles_color_count_search_and_may_cast() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Evolving Door Probe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "{1}, {T}, Sacrifice a creature: Count the colors of the sacrificed creature, then search your library for a creature card that's exactly that many colors plus one. Exile that card, then shuffle. You may cast the exiled card. Activate only as a sorcery.",
        )
        .expect("evolving door should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    let debug = format!("{:#?}", def.abilities);
    assert!(
        rendered.contains("search your library for a creature card")
            && rendered
                .contains("color count equal to the number of colors among permanent plus 1")
            && rendered.contains("you may cast that card"),
        "expected Evolving Door compiled search and may-cast wording, got {rendered}\n{debug}"
    );
    assert!(
        !rendered.contains("you searches")
            && !rendered.contains("cast the tagged object")
            && !rendered.contains("that many color plus one")
            && !rendered.contains(".."),
        "expected Evolving Door to normalize the awkward compiled phrasing, got {rendered}"
    );

    let oracle_rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        oracle_rendered.contains("search your library for a creature card")
            && oracle_rendered
                .contains("color count equal to the number of colors among permanent plus 1")
            && oracle_rendered.contains("you may cast that card"),
        "expected Evolving Door oracle-like wording, got {oracle_rendered}"
    );

    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        debug.contains("color_count") && debug.contains("colorsamong") && debug.contains("may"),
        "expected Evolving Door to compile a color-count comparison and a may wrapper, got {debug}"
    );
    assert!(
        !debug.contains("unsupported"),
        "Evolving Door parse should avoid unsupported placeholders, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_doubling_chant_compiles_search_put_onto_battlefield_and_shuffle() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Doubling Chant Probe")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(5)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "For each creature you control, you may search your library for a creature card with the same name as that creature. Put those cards onto the battlefield, then shuffle.",
        )
        .expect("doubling chant should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("search your library for")
            && rendered.contains("same name")
            && rendered.contains("onto the battlefield")
            && rendered.contains("shuffle"),
        "expected Doubling Chant compiled search/battlefield/shuffle wording, got {rendered}"
    );
    assert!(
        !rendered.contains("tags it as 'searched'")
            && !rendered.contains("cast the tagged object")
            && !rendered.contains("you searches"),
        "expected Doubling Chant to avoid debug-style search scaffolding, got {rendered}"
    );

    let oracle_rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        oracle_rendered.contains("search your library for")
            && oracle_rendered.contains("same name")
            && oracle_rendered.contains("onto the battlefield")
            && oracle_rendered.contains("shuffle"),
        "expected Doubling Chant oracle-like wording, got {oracle_rendered}"
    );

    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        debug.contains("foreachobject")
            && debug.contains("mayeffect")
            && debug.contains("samenameastagged")
            && debug.contains("shufflelibraryeffect"),
        "expected Doubling Chant to keep per-creature same-name search structure, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_feral_encounter_avoids_tagged_play_marker_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Feral Encounter")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Green],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Look at the top five cards of your library. You may exile a creature card from among them. Put the rest on the bottom of your library in a random order. You may cast the exiled card this turn. At the beginning of the next combat phase this turn, target creature you control deals damage equal to its power to up to one target creature you don't control.",
        )
        .expect("feral encounter should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("you may cast that card this turn")
            || rendered.contains("you may cast the exiled card this turn"),
        "expected Feral Encounter cast permission text, got {rendered}"
    );
    assert!(
        rendered.contains("you may exile a creature card from among them")
            && rendered.contains("put the rest on the bottom of your library in a random order"),
        "expected Feral Encounter optional exile and library-bottom wording, got {rendered}"
    );
    assert!(
        rendered.contains(
            "deals damage equal to its power to up to one target creature you don't control"
        ),
        "expected Feral Encounter damage text, got {rendered}"
    );
    assert!(
        !rendered.contains("tagged object")
            && !rendered.contains("tagged '")
            && !rendered.contains("tagged cards")
            && !rendered.contains("you choose up to one creature cards"),
        "expected Feral Encounter to avoid internal tagged markers, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_triggered_explore_clause_without_fallback_marker() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Explore Trigger Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, it explores. (Reveal the top card of your library. Put that card into your hand if it's a land. Otherwise, put a +1/+1 counter on this creature, then put the card back or put it into your graveyard.)",
        )
        .expect("explore trigger should parse as an explicit mechanic effect");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("explores"),
        "expected explore text in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "explore trigger should not rely on unsupported fallback marker: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_parse_explore_trigger_subject_without_fallback_marker() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Wildgrowth Walker")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 3))
        .parse_text(
            "Whenever a creature you control explores, put a +1/+1 counter on this creature and you gain 3 life.",
        )
        .expect("explore subject trigger should parse as an explicit keyword-action trigger");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("whenever a creature you control explores"),
        "expected explore trigger text in oracle-like output, got {rendered}"
    );
    assert!(
        !rendered.contains("unsupported parser line fallback"),
        "explore subject trigger should not rely on unsupported fallback marker: {rendered}"
    );

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("KeywordAction") && debug.contains("Explore"),
        "expected explore keyword-action trigger in parsed definition, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn green_suns_twilight_keeps_revealed_pool_and_typed_destination_replacement() {
    let def = parse_oracle_card_definition("Green Sun's Twilight");
    let program = def
        .spell_effect
        .as_ref()
        .expect("Green Sun's Twilight should have a spell resolution program");
    let [segment] = program.segments.as_slice() else {
        panic!("expected one linked resolution segment, got {program:#?}");
    };
    let [replacement] = segment.self_replacements.as_slice() else {
        panic!("expected one X-threshold self-replacement, got {segment:#?}");
    };
    assert!(matches!(
        &replacement.condition,
        crate::ConditionExpr::XValueAtLeast(5)
    ));

    let [
        default_look,
        default_creature,
        default_land,
        default_move,
        default_rest,
    ] = segment.default_effects.as_slice()
    else {
        panic!("expected linked default reveal/choice/disposition pipeline: {segment:#?}");
    };
    let [
        replacement_look,
        replacement_creature,
        replacement_land,
        destination_choice,
        replacement_rest,
    ] = replacement.replacement_effects.as_slice()
    else {
        panic!("expected linked replacement reveal/choice/disposition pipeline: {replacement:#?}");
    };

    let look = default_look
        .downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        .expect("default branch should reveal the top cards");
    let replacement_look = replacement_look
        .downcast_ref::<crate::effects::LookAtTopCardsEffect>()
        .expect("replacement branch should retain the same reveal");
    assert!(look.reveal);
    assert_eq!(look, replacement_look);
    assert_eq!(
        look.count,
        crate::effect::Value::Add(
            Box::new(crate::effect::Value::X),
            Box::new(crate::effect::Value::Fixed(1)),
        )
    );

    let default_choices = [default_creature, default_land].map(|effect| {
        effect
            .downcast_ref::<ChooseObjectsEffect>()
            .expect("default branch should keep independent typed choices")
    });
    let replacement_choices = [replacement_creature, replacement_land].map(|effect| {
        effect
            .downcast_ref::<ChooseObjectsEffect>()
            .expect("replacement branch should keep independent typed choices")
    });
    assert_eq!(default_choices, replacement_choices);
    assert_eq!(default_choices[0].tag, default_choices[1].tag);
    assert_eq!(default_choices[0].filter.card_types, [CardType::Creature]);
    assert_eq!(default_choices[1].filter.card_types, [CardType::Land]);
    for choose in default_choices {
        assert_eq!(choose.count.min, 0);
        assert_eq!(choose.count.max, Some(1));
        assert_eq!(choose.filter.zone, Some(Zone::Library));
        assert!(choose.filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == look.tag
                && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        }));
        assert!(choose.filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == choose.tag
                && constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
        }));
    }

    let default_move = default_move
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()
        .expect("default selected set should move as one tagged group");
    assert_eq!(default_move.tag, default_choices[0].tag);
    let [default_move_one] = default_move.effects.as_slice() else {
        panic!("expected one iterated default move, got {default_move:#?}");
    };
    let default_move_one = default_move_one
        .downcast_ref::<MoveToZoneEffect>()
        .expect("default group move should be typed");
    assert_eq!(default_move_one.zone, Zone::Hand);
    assert!(matches!(
        default_move_one.target.base(),
        ChooseSpec::Iterated
    ));

    let destination_choice = destination_choice
        .downcast_ref::<ChooseModeEffect>()
        .expect("X-threshold branch should choose one shared destination");
    assert_eq!(destination_choice.modes.len(), 2);
    let destinations = destination_choice
        .modes
        .iter()
        .map(|mode| {
            let [move_group] = mode.effects.as_slice() else {
                panic!("destination mode should contain one group move: {mode:#?}");
            };
            let move_group = move_group
                .downcast_ref::<crate::effects::ForEachTaggedEffect>()
                .expect("destination should move the selected tagged group");
            assert_eq!(move_group.tag, default_choices[0].tag);
            let [move_one] = move_group.effects.as_slice() else {
                panic!("destination group should contain one iterated move: {move_group:#?}");
            };
            move_one
                .downcast_ref::<MoveToZoneEffect>()
                .expect("destination move should be typed")
                .zone
        })
        .collect::<Vec<_>>();
    assert_eq!(destinations, [Zone::Battlefield, Zone::Hand]);

    let default_rest = default_rest
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()
        .expect("default branch should preserve the exact revealed complement");
    let replacement_rest = replacement_rest
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()
        .expect("replacement branch should preserve the exact revealed complement");
    assert_eq!(default_rest, replacement_rest);
    assert_eq!(default_rest.tag, look.tag);
    assert_eq!(
        default_rest.keep_tagged.as_ref(),
        Some(&default_choices[0].tag)
    );

    assert_eq!(
        compiled_text_lines(&def).join("\n"),
        "Reveal the top X plus one cards of your library. Choose a creature card and/or a land card from among them. Put those cards into your hand and the rest on the bottom of your library in a random order. If X is 5 or more, instead put the chosen cards onto the battlefield or into your hand and the rest on the bottom of your library in a random order."
    );
}
