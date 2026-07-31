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
pub(super) fn demonic_bargain_search_followup_hides_internal_tag_reference() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Demonic Bargain Variant")
        .parse_text(
            "Exile the top thirteen cards of your library, then search your library for a card. Put that card into your hand, then shuffle.",
        )
        .expect("Demonic Bargain text should parse");

    let rendered = compiled_text_lines(&def).join(" ");
    assert!(
        !rendered.to_ascii_lowercase().contains("tagged '"),
        "search followup leaked an internal tag reference: {rendered}; unprocessed: {}",
        unprocessed_compiled_lines(&def).join(" ")
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn ruin_in_their_wake_conditional_search_followup_hides_internal_tag_reference() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Ruin in Their Wake Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Devoid\nSearch your library for a basic land card and reveal it. You may put that card onto the battlefield tapped if you control a land named Wastes. Otherwise, put that card into your hand. Then shuffle.",
        )
        .expect("Ruin in Their Wake text should parse");

    let rendered = compiled_text_lines(&def);
    assert_eq!(rendered.first().map(String::as_str), Some("Devoid"));
    let search_line = rendered.get(1).expect("Ruin in Their Wake search line");
    assert!(
        search_line.contains("Search your library for a basic land card and reveal it")
            && search_line.contains("put it onto the battlefield tapped")
            && search_line.contains("if you control a land named wastes")
            && search_line.contains("Otherwise, put it into your hand")
            && search_line.contains("shuffle your library"),
        "conditional search followup lost its searched-card branch structure; unprocessed: {}",
        unprocessed_compiled_lines(&def).join(" "),
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn auspicious_starrix_keeps_variable_consult_collection_reference() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Auspicious Starrix Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Mutate {5}{G}\nWhenever this creature mutates, exile cards from the top of your library until you exile X permanent cards, where X is the number of times this creature has mutated. Put those permanent cards onto the battlefield.",
        )
        .expect("Auspicious Starrix text should parse");

    assert_eq!(
        compiled_text_lines(&def),
        vec![
            "Mutate {5}{G}".to_string(),
            "Whenever this creature mutates, exile cards from the top of your library until you exile X permanent cards, where X is the number of times this creature has mutated. Put those permanent cards onto the battlefield.".to_string(),
        ],
        "variable consult lost its typed matched collection; unprocessed: {}",
        unprocessed_compiled_lines(&def).join(" "),
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn mindleecher_keeps_costed_mutate_and_flying_on_separate_lines() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mindleecher Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Mutate {4}{B}\nFlying\nWhenever this creature mutates, exile the top card of each opponent's library face down. You may look at and play those cards for as long as they remain exiled.",
        )
        .expect("Mindleecher text should parse");

    assert_eq!(
        compiled_text_lines(&def),
        vec![
            "Mutate {4}{B}".to_string(),
            "Flying".to_string(),
            "Whenever this creature mutates, exile the top card of each opponent's library face down. You may look at and play those cards for as long as they remain exiled.".to_string(),
        ],
        "costed Mutate merged with the following intrinsic keyword; unprocessed: {}",
        unprocessed_compiled_lines(&def).join(" "),
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_search_basic_triple_and_gain_life_keeps_all_components() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Brokers Hideout Variant")
        .parse_text(
            "When this land enters, sacrifice it. When you do, search your library for a basic Forest, Plains, or Island card, put it onto the battlefield tapped, then shuffle and you gain 1 life.",
        )
        .expect("search basic triple line should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("forest") && rendered.contains("plains") && rendered.contains("island"),
        "expected all three basic land types in search filter, got {rendered}"
    );
    assert!(
        rendered.contains("gain 1 life"),
        "expected trailing life gain clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_search_put_discard_random_then_shuffle_keeps_discard_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Gamble Variant")
        .parse_text(
            "Search your library for a card, put that card into your hand, discard a card at random, then shuffle.",
        )
        .expect("search-discard-random-then-shuffle clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("search your library for a card"),
        "expected search-library clause to remain, got {rendered}"
    );
    assert!(
        rendered.contains("discard a card at random"),
        "expected discard-at-random clause to remain, got {rendered}"
    );
    assert!(
        rendered.contains("shuffle"),
        "expected shuffle clause to remain, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_wild_research_style_search_reveal_hand_discard_shuffle_hides_search_tags() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Wild Research Variant")
        .parse_text(
            "{1}{W}: Search your library for an enchantment card and reveal that card. Put it into your hand, then discard a card at random. Then shuffle.",
        )
        .expect("Wild Research style search ability should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("search your library for")
            && rendered.contains("enchantment card")
            && (rendered.contains("reveal that card") || rendered.contains("reveal it"))
            && rendered.contains("discard a card at random")
            && (rendered.contains("then shuffle") || rendered.contains("shuffle your library"))
            && !rendered.contains("up to one")
            && !rendered.contains("tagged object")
            && !rendered.contains("tags it as"),
        "expected Wild Research style compiled text to hide internal search tags, got {rendered}"
    );
}

#[test]
pub(super) fn dream_tides_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Dream Tides");
    let def = parse_oracle_card_definition("Dream Tides");
    let rendered = unprocessed_compiled_lines(&def).join("\n");

    assert!(
        rendered.contains(
            "At the beginning of each player's upkeep, that player may choose any number of tapped nongreen creatures they control and pay {2} for each creature chosen this way. If the player does, untap those creatures."
        ),
        "expected Dream Tides choose/pay/untap clause to render oracle-like text, got {rendered}"
    );
    assert!(
        !rendered.contains("tagged") && !rendered.contains("have you choose"),
        "Dream Tides compiled text should not expose internal tags or mis-bound chooser text, got {rendered}"
    );
}

#[test]
pub(super) fn optional_source_damage_followups_render_player_conditions() {
    let vexing = parse_oracle_card_definition("Vexing Devil");
    let vexing_rendered = unprocessed_compiled_lines(&vexing).join(" ");
    assert!(
        vexing_rendered.contains(
            "When this creature enters, any opponent may have it deal 4 damage to them. If a player does, sacrifice this creature."
        ),
        "expected Vexing Devil optional damage follow-up to use player condition, got {vexing_rendered}"
    );

    let breaking = parse_oracle_card_definition("Breaking Point");
    let breaking_rendered = unprocessed_compiled_lines(&breaking).join(" ");
    assert!(
        breaking_rendered.contains(
            "Any player may have it deal 6 damage to them. If no one does, destroy all creatures. Creatures destroyed this way can't be regenerated."
        ),
        "expected Breaking Point optional damage follow-up to stay conditional, got {breaking_rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn any_player_may_sacrifice_choice_keeps_optional_player_subject() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Prowling Pangolin Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, any player may sacrifice two creatures of their choice. If a player does, sacrifice this creature.",
        )
        .expect("Prowling Pangolin-style optional sacrifice should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        (rendered.contains("any player may sacrifice two creatures")
            || rendered.contains("a player may sacrifice two creatures"))
            && rendered.contains("if a player does, sacrifice this creature"),
        "expected any-player optional sacrifice and follow-up condition, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn filtered_anthem_counts_counters_on_each_affected_creature() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Clamavus Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Proclamator Hailer — Each creature you control gets +1/+1 for each +1/+1 counter on it.",
        )
        .expect("Clamavus-style counter anthem should parse");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("CountersOnAffected"),
        "expected the anthem count to refer to each affected creature, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("gets +1/+1 for each +1/+1 counter on it"),
        "expected counter anthem to render 'on it', got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn filtered_anthem_preserves_explicit_source_counter_surface() {
    let equipment_def = CardDefinitionBuilder::new(CardId::from_raw(1), "Blade Variant")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .parse_text(
            "Equipped creature gets +1/+1 for each charge counter on this Equipment.\nEquip {2}",
        )
        .expect("source-counter equipment anthem should parse");

    let debug = format!("{:#?}", equipment_def.abilities);
    assert!(
        debug.contains("CountersOnSourceWithSurface") && !debug.contains("CountersOnAffected"),
        "expected explicit source surface to stay source-counted, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&equipment_def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("gets +1/+1 for each charge counter on this equipment"),
        "expected source counter anthem to render the Equipment surface, got {rendered}"
    );

    let named_def = CardDefinitionBuilder::new(CardId::from_raw(2), "Excalibur II")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .parse_text(
            "Equipped creature gets +1/+1 for each charge counter on Excalibur II.\nEquip {3}",
        )
        .expect("named source-counter equipment anthem should parse");

    let named_rendered = unprocessed_compiled_lines(&named_def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        named_rendered.contains("gets +1/+1 for each charge counter on excalibur ii"),
        "expected named source counter anthem to render the source name, got {named_rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_source_surface_for_hard_triggered_and_static_clauses() {
    let cases = [
        (
            "Kain Variant",
            "Jump — During your turn, Kain has flying.\nWhenever Kain deals combat damage to a player, that player gains control of Kain. If they do, you draw that many cards, create that many tapped Treasure tokens, then lose that much life.",
            "if they do, you draw that many cards",
        ),
        (
            "Ardyn Variant",
            "Demons you control have menace, lifelink, and haste.\nStarscourge — At the beginning of combat on your turn, exile up to one target creature card from a graveyard. If you exiled a card this way, create a token that's a copy of that card, except it's a 5/5 black Demon.",
            "starscourge — at the beginning of combat on your turn",
        ),
        (
            "Predatory Advantage Variant",
            "At the beginning of each opponent's end step, if that player didn't cast a creature spell this turn, create a 2/2 green Lizard creature token.",
            "if that player didn't cast a creature spell this turn",
        ),
        (
            "Runebound Wolf Variant",
            "{3}{R}, {T}: This creature deals damage equal to the number of Wolves and Werewolves you control to target opponent.",
            "number of wolves and werewolves you control",
        ),
        (
            "Mortipede Variant",
            "{2}{G}: All creatures able to block this creature this turn do so.",
            "all creatures able to block this creature this turn do so",
        ),
        (
            "Coat of Arms Variant",
            "Each creature gets +1/+1 for each other creature on the battlefield that shares at least one creature type with it.",
            "shares at least one creature type with it",
        ),
        (
            "Karma Variant",
            "At the beginning of each player's upkeep, this enchantment deals damage to that player equal to the number of Swamps they control.",
            "damage to that player equal to the number of swamps",
        ),
        (
            "Vexing Devil Variant",
            "When this creature enters, any opponent may have it deal 4 damage to them. If a player does, sacrifice this creature.",
            "any opponent may have it deal 4 damage to them",
        ),
        (
            "Reptilian Reflection Variant",
            "Whenever you cycle a card, you may have this enchantment become a 5/4 Dinosaur creature with trample and haste in addition to its other types until end of turn.",
            "become a 5/4 dinosaur creature with trample and haste",
        ),
        (
            "Thresher Lizard Variant",
            "This creature gets +1/+2 as long as you have one or fewer cards in hand.",
            "as long as you have one or fewer cards in hand",
        ),
        (
            "Dream Tides Variant",
            "Creatures don't untap during their controllers' untap steps.\nAt the beginning of each player's upkeep, that player may choose any number of tapped nongreen creatures they control and pay {2} for each creature chosen this way. If the player does, untap those creatures.",
            "for each creature chosen this way",
        ),
        (
            "Portcullis Variant",
            "Whenever a creature enters, if there are two or more other creatures on the battlefield, exile that creature. Return that card to the battlefield under its owner's control when this artifact leaves the battlefield.",
            "two or more other creatures on the battlefield",
        ),
        (
            "Impatience Variant",
            "At the beginning of each player's end step, if that player didn't cast a spell this turn, this enchantment deals 2 damage to that player.",
            "didn't cast a spell this turn",
        ),
        (
            "Tin Street Variant",
            "When this creature enters, if {G} was spent to cast it, destroy target artifact.",
            "if {g} was spent to cast it",
        ),
        (
            "Runic Armasaur Variant",
            "Whenever an opponent activates an ability of a creature or land that isn't a mana ability, you may draw a card.",
            "isn't a mana ability",
        ),
        (
            "Turf War Variant",
            "When this enchantment enters, for each player, put a contested counter on target land that player controls.\nWhenever a creature deals combat damage to a player, if that player controls one or more lands with contested counters on them, that creature's controller gains control of one of those lands of their choice and untaps it.",
            "contested counter on target land",
        ),
        (
            "Battle of Wits Variant",
            "At the beginning of your upkeep, if you have 200 or more cards in your library, you win the game.",
            "200 or more cards",
        ),
        (
            "Awakening Zone Variant",
            "At the beginning of your upkeep, you may create a 0/1 colorless Eldrazi Spawn creature token. It has \"Sacrifice this token: Add {C}.\"",
            "sacrifice this token: add {c}",
        ),
        (
            "Blinkmoth Urn Variant",
            "At the beginning of each player's first main phase, if this artifact is untapped, that player adds {C} for each artifact they control.",
            "if this artifact is untapped",
        ),
        (
            "Gemini Engine Variant",
            "Whenever this creature attacks, create a colorless Construct artifact creature token named Twin that's attacking. Its power is equal to this creature's power and its toughness is equal to this creature's toughness. Sacrifice the token at end of combat.",
            "its power is equal to this creature's power",
        ),
        (
            "Crawling Sensation Variant",
            "At the beginning of your upkeep, you may mill two cards.\nWhenever one or more land cards are put into your graveyard from anywhere for the first time each turn, create a 1/1 green Insect creature token.",
            "first time each turn",
        ),
        (
            "Smuggler's Share Variant",
            "At the beginning of each end step, draw a card for each opponent who drew two or more cards this turn, then create a Treasure token for each opponent who had two or more lands enter the battlefield under their control this turn.",
            "for each opponent who drew two or more cards this turn",
        ),
        (
            "Monsoon Variant",
            "At the beginning of each player's end step, tap all untapped Islands that player controls and this enchantment deals X damage to the player, where X is the number of Islands tapped this way.",
            "where x is the number of islands tapped this way",
        ),
        (
            "Anthousa Variant",
            "Heroic — Whenever you cast a spell that targets Anthousa, up to three target lands you control each become 2/2 Warrior creatures until end of turn. They're still lands.",
            "they're still lands",
        ),
        (
            "Glaring Spotlight Variant",
            "Creatures your opponents control with hexproof can be the targets of spells and abilities you control as though they didn't have hexproof.\n{3}, Sacrifice this artifact: Creatures you control gain hexproof until end of turn and can't be blocked this turn.",
            "as though they didn't have hexproof",
        ),
        (
            "Keeper of the Mind Variant",
            "{U}, {T}: Choose target opponent who has at least two more cards in hand than you do as you activate this ability. Draw a card.",
            "as you activate this ability",
        ),
        (
            "Sphinx Ambassador Variant",
            "Flying\nWhenever this creature deals combat damage to a player, search that player's library for a card, then that player chooses a card name. If you searched for a creature card that doesn't have that name, you may put it onto the battlefield under your control. Then that player shuffles.",
            "that player chooses a card name",
        ),
        (
            "Walking Desecration Variant",
            "{B}, {T}: Creatures of the creature type of your choice attack this turn if able.",
            "creature type of your choice",
        ),
        (
            "Genasi Enforcers Variant",
            "Myriad\n{1}{R}: Creatures you control named Genasi Enforcers get +1/+0 until end of turn.",
            "creatures you control named genasi enforcers",
        ),
        (
            "Jodah's Codex Variant",
            "Domain — {5}, {T}: Draw a card. This ability costs {1} less to activate for each basic land type among lands you control.",
            "basic land type among lands you control",
        ),
        (
            "Emet-Selch Variant",
            "Spells you cast from your graveyard cost {2} less to cast.\nWhenever one or more opponents lose life, you may cast target instant or sorcery card from your graveyard. If that spell would be put into your graveyard, exile it instead. Do this only once each turn.",
            "one or more opponents lose life",
        ),
        (
            "Answered Prayers Variant",
            "Whenever a creature you control enters, you gain 1 life. If this enchantment isn't a creature, it becomes a 3/3 Angel creature with flying in addition to its other types until end of turn.",
            "if this enchantment isn't a creature",
        ),
        (
            "Fear of Immobility Variant",
            "When this creature enters, tap up to one target creature. If an opponent controls that creature, put a stun counter on it.",
            "tap up to one target creature",
        ),
        (
            "Veiled Apparition Variant",
            "When an opponent casts a spell, if this permanent is an enchantment, it becomes a 3/3 Illusion creature with flying and \"At the beginning of your upkeep, sacrifice this creature unless you pay {1}{U}.\"",
            "if this permanent is an enchantment",
        ),
        (
            "Ashiok's Reaper Variant",
            "Whenever an enchantment you control is put into a graveyard from the battlefield, draw a card.",
            "put into a graveyard from the battlefield",
        ),
        (
            "Yomiji Variant",
            "Whenever a legendary permanent other than Yomiji is put into a graveyard from the battlefield, return that card to its owner's hand.",
            "put into a graveyard from the battlefield",
        ),
        (
            "Zodiac Dragon Variant",
            "When this creature is put into your graveyard from the battlefield, you may return it to your hand.",
            "put into your graveyard from the battlefield",
        ),
        (
            "Dream-Thief's Bandana Variant",
            "Whenever equipped creature deals combat damage to a player, look at the top card of their library, then exile it face down. For as long as it remains exiled, you may play it, and mana of any type can be spent to cast that spell.\nEquip {1}",
            "for as long as it remains exiled",
        ),
        (
            "Burning-Rune Demon Variant",
            "Flying\nWhen this creature enters, you may search your library for exactly two cards not named Burning-Rune Demon that have different names. If you do, reveal those cards. An opponent chooses one of them. Put the chosen card into your hand and the other into your graveyard, then shuffle.",
            "exactly two cards not named",
        ),
        (
            "Aether Rift Variant",
            "At the beginning of your upkeep, discard a card at random. If you discard a creature card this way, return it from your graveyard to the battlefield unless any player pays 5 life.",
            "discard a card at random",
        ),
        (
            "Graf Rats Variant",
            "At the beginning of combat on your turn, if you both own and control this creature and a creature named Midnight Scavengers, exile them, then meld them into Chittering Host.",
            "both own and control",
        ),
        (
            "G'raha Tia Variant",
            "Reach\nThe Allagan Eye — Whenever one or more other creatures and/or artifacts you control die, draw a card. This ability triggers only once each turn.",
            "triggers only once each turn",
        ),
        (
            "Fae Offering Variant",
            "At the beginning of each end step, if you've cast both a creature spell and a noncreature spell this turn, create a Clue token, a Food token, and a Treasure token.",
            "cast both a creature spell and a noncreature spell",
        ),
        (
            "Volo Variant",
            "Whenever you cast a creature spell that doesn't share a creature type with a creature you control or a creature card in your graveyard, copy that spell.",
            "doesn't share a creature type",
        ),
        (
            "Crimson Caravaneer Variant",
            "Double strike, trample\nWhenever this creature deals combat damage to a player, create a Junk token.",
            "create a junk token",
        ),
        (
            "Mirror-Mad Variant",
            "{1}{U}: This creature's owner shuffles it into their library. If that player does, they reveal cards from the top of that library until a card named Mirror-Mad Phantasm is revealed. The player puts that card onto the battlefield and all other cards revealed this way into their graveyard.",
            "if that player does",
        ),
        (
            "Displaced Dinosaurs Variant",
            "As a historic permanent you control enters, it becomes a 7/7 Dinosaur creature in addition to its other types.",
            "as a historic permanent you control enters",
        ),
        (
            "Gandalf Westward Variant",
            "Whenever you cast a spell with mana value 5 or greater, each opponent reveals the top card of their library. If any of those cards shares a card type with that spell, copy that spell, you may choose new targets for the copy, and each opponent draws a card. Otherwise, you draw a card.",
            "shares a card type with that spell",
        ),
        (
            "Mind Maggots Variant",
            "When this creature enters, discard any number of creature cards. For each card discarded this way, put two +1/+1 counters on this creature.",
            "for each card discarded this way",
        ),
        (
            "Hesitation Variant",
            "When a player casts a spell, sacrifice this enchantment and counter that spell.",
            "sacrifice this enchantment and counter that spell",
        ),
        (
            "Eye of Doom Variant",
            "When this artifact enters, each player chooses a nonland permanent and puts a doom counter on it.\n{2}, {T}, Sacrifice this artifact: Destroy each permanent with a doom counter on it.",
            "each player chooses a nonland permanent",
        ),
        (
            "Fight or Flight Variant",
            "At the beginning of combat on each opponent's turn, separate all creatures that player controls into two piles. Only creatures in the pile of their choice can attack this turn.",
            "separate all creatures that player controls into two piles",
        ),
        (
            "Sindbad Variant",
            "{T}: Draw a card and reveal it. If it isn't a land card, discard it.",
            "draw a card and reveal it",
        ),
        (
            "Legion Angel Variant",
            "Flying\nWhen this creature enters, you may reveal a card you own named Legion Angel from outside the game and put it into your hand.",
            "from outside the game",
        ),
        (
            "Haphazard Bombardment Variant",
            "When this enchantment enters, choose four nonenchantment permanents you don't control and put an aim counter on each of them.\nAt the beginning of your end step, if two or more permanents you don't control have an aim counter on them, destroy one of those permanents at random.",
            "one of those permanents at random",
        ),
        (
            "Mad Dog Variant",
            "At the beginning of your end step, if this creature didn't attack or come under your control this turn, sacrifice it.",
            "didn't attack or come under your control this turn",
        ),
        (
            "Undercity Informer Variant",
            "{1}, Sacrifice a creature: Target player reveals cards from the top of their library until they reveal a land card, then puts those cards into their graveyard.",
            "reveals cards from the top of their library until",
        ),
        (
            "Wavebreak Hippocamp Variant",
            "Whenever you cast your first spell during each opponent's turn, draw a card.",
            "your first spell during each opponent's turn",
        ),
        (
            "Valiant Changeling Variant",
            "This spell costs {1} less to cast for each creature type among creatures you control. This effect can't reduce the amount of mana this spell costs by more than {5}.\nChangeling\nDouble strike",
            "creature type among creatures you control",
        ),
        (
            "Avenging Druid Variant",
            "Whenever this creature deals damage to an opponent, you may reveal cards from the top of your library until you reveal a land card. If you do, put that card onto the battlefield and put all other cards revealed this way into your graveyard.",
            "all other cards revealed this way",
        ),
        (
            "Myr Adapter Variant",
            "This creature gets +1/+1 for each Equipment attached to it.",
            "equipment attached to it",
        ),
        (
            "Myr Incubator Variant",
            "{6}, {T}, Sacrifice this artifact: Search your library for any number of artifact cards, exile them, then create that many 1/1 colorless Myr artifact creature tokens. Then shuffle.",
            "create that many 1/1 colorless myr",
        ),
        (
            "Synthesis Pod Variant",
            "{1}{U/P}, {T}, Exile a spell you control: Target opponent reveals cards from the top of their library until they reveal a card with mana value equal to 1 plus the exiled spell's mana value. Exile that card, then that player shuffles. You may cast that exiled card without paying its mana cost.",
            "card with mana value equal to",
        ),
        (
            "Memory Jar Variant",
            "{T}, Sacrifice this artifact: Each player exiles all cards from their hand face down and draws seven cards. At the beginning of the next end step, each player discards their hand and returns to their hand each card they exiled this way.",
            "returns to their hand each card they exiled this way",
        ),
        (
            "Woodwraith Corrupter Variant",
            "{1}{B}{G}, {T}: Target Forest becomes a 4/4 black and green Elemental Horror creature. It's still a land.",
            "it's still a land",
        ),
        (
            "Genestealer Patriarch Variant",
            "Genestealer's Kiss — Whenever this creature attacks, put an infection counter on target creature defending player controls.\nChildren of the Cult — Whenever a creature with an infection counter on it dies, you create a token that's a copy of that creature, except it's a Tyranid in addition to its other types.",
            "children of the cult",
        ),
        (
            "Commander Sofia Variant",
            "Flash\nCrash Landing — When Commander Sofia Daguerre enters, destroy up to one target legendary permanent. That permanent's controller creates a Junk token.",
            "that permanent's controller creates a junk token",
        ),
        (
            "Mister Gutsy Variant",
            "Whenever you cast an Aura or Equipment spell, put a +1/+1 counter on this creature.\nWhen this creature dies, create X Junk tokens, where X is the number of +1/+1 counters on it.",
            "aura or equipment spell",
        ),
        (
            "Catapult Fodder Variant",
            "At the beginning of combat on your turn, if you control three or more creatures that each have toughness greater than their power, transform this creature.",
            "three or more creatures that each have toughness greater",
        ),
        (
            "Donal Variant",
            "Whenever you cast a nonlegendary creature spell with flying, you may copy it, except the copy is a 1/1 Spirit in addition to its other types. Do this only once each turn.",
            "except the copy is a 1/1",
        ),
        (
            "Uphill Battle Variant",
            "Creatures played by your opponents enter tapped.",
            "played by your opponents enter tapped",
        ),
        (
            "Edgar Variant",
            "Once during each of your turns, you may cast an artifact spell from your graveyard. If you cast a spell this way, that artifact enters tapped.\nTools — Whenever Edgar attacks, it gets +X/+0 until end of turn, where X is the greatest mana value among artifacts you control.",
            "once during each of your turns",
        ),
        (
            "Disciple of Bolas Variant",
            "When this creature enters, sacrifice another creature. You gain X life and draw X cards, where X is that creature's power.",
            "where x is that creature's power",
        ),
        (
            "Mirror-Style Master Variant",
            "Backup 1\nWhenever this creature attacks, for each attacking modified creature you control, create a tapped and attacking token that's a copy of that creature. Exile those tokens at end of combat.",
            "backup 1",
        ),
        (
            "Rolling Stones Variant",
            "Wall creatures can attack as though they didn't have defender.",
            "as though they didn't have defender",
        ),
        (
            "Sahagin Variant",
            "Whenever you cast a noncreature spell, if at least four mana was spent to cast it, put a +1/+1 counter on this creature and it can't be blocked this turn.",
            "if at least four mana was spent",
        ),
        (
            "Curious Forager Variant",
            "When this creature enters, you may forage. When you do, return target permanent card from your graveyard to your hand.",
            "when you do",
        ),
        (
            "Lineprancers Variant",
            "When Lineprancers enters, you get {TK}{TK}, then you may put a power and toughness sticker on a creature you own.\n{3}{G}: Target creature you don't control blocks target creature you control with a power and toughness sticker on it other than Lineprancers this turn if able.",
            "blocks target creature you control with a power and toughness sticker",
        ),
        (
            "Stoic Farmer Variant",
            "When this creature enters, search your library for a basic Plains card and reveal it. If an opponent controls more lands than you, put it onto the battlefield tapped. Otherwise, put it into your hand. Then shuffle.\nForetell {1}{W}",
            "if an opponent controls more lands than you",
        ),
        (
            "Reptilian Recruiter Variant",
            "Trample\nWhen this creature enters, choose target creature. If that creature's power is 2 or less or if you control another Lizard, gain control of that creature until end of turn, untap it, and it gains haste until end of turn.",
            "if that creature's power is 2 or less",
        ),
        (
            "Druid's Familiar Variant",
            "Soulbond\nAs long as this creature is paired with another creature, each of those creatures gets +2/+2.",
            "soulbond",
        ),
        (
            "Doom Weaver",
            "Reach\nSoulbond (You may pair this creature with another unpaired creature when either enters. They remain paired for as long as you control both of them.)\nAs long as Doom Weaver is paired with another creature, each of those creatures has \"When this creature dies, draw cards equal to its power.\"",
            "when this creature dies",
        ),
        (
            "Hearth Elemental Variant",
            "This spell costs {X} less to cast, where X is the number of cards in your graveyard that are instant cards, sorcery cards, and/or have an Adventure.",
            "costs {x} less to cast, where x is",
        ),
        (
            "Viridian Revel Variant",
            "Whenever an artifact is put into an opponent's graveyard from the battlefield, you may draw a card.",
            "opponent's graveyard from the battlefield",
        ),
        (
            "Living Lands Variant",
            "All Forests are 1/1 creatures that are still lands.",
            "all forests are 1/1 creatures that are still lands",
        ),
        (
            "Odric Variant",
            "First strike\nWhenever Odric and at least three other creatures attack, you choose which creatures block this combat and how those creatures block.",
            "which creatures block this combat",
        ),
        (
            "Wood Sage Variant",
            "{T}: Choose a creature card name. Reveal the top four cards of your library and put all of them with that name into your hand. Put the rest into your graveyard.",
            "reveal the top four cards",
        ),
        (
            "Pantlaza Variant",
            "Whenever Pantlaza or another Dinosaur you control enters, you may discover X, where X is that creature's toughness. Do this only once each turn.",
            "discover x, where x is that creature's toughness",
        ),
        (
            "Averna Variant",
            "As you cascade, you may put a land card from among the exiled cards onto the battlefield tapped.",
            "as you cascade",
        ),
        (
            "Tunnel Ignus Variant",
            "Whenever a land enters under an opponent's control, if that player had another land enter the battlefield under their control this turn, this creature deals 3 damage to that player.",
            "had another land enter",
        ),
        (
            "Mana Web Variant",
            "Whenever a land an opponent controls is tapped for mana, tap all lands that player controls that could produce any type of mana that land could produce.",
            "could produce any type of mana that land could produce",
        ),
        (
            "Goblin Diplomats Variant",
            "{T}: Each creature attacks this turn if able.",
            "each creature attacks this turn if able",
        ),
        (
            "Edgewalker Variant",
            "Cleric spells you cast cost {W}{B} less to cast. This effect reduces only the amount of colored mana you pay.",
            "reduces only the amount of colored mana",
        ),
        (
            "Happily Ever After Variant",
            "When this enchantment enters, each player gains 5 life and draws a card.\nAt the beginning of your upkeep, if there are five colors among permanents you control, there are six or more card types among permanents you control and/or cards in your graveyard, and your life total is greater than or equal to your starting life total, you win the game.",
            "six or more card types",
        ),
        (
            "Sunbathing Rootwalla Variant",
            "Domain — {3}{G}: Until end of turn, this creature gets +1/+1 for each basic land type among lands you control. Activate only once each turn.",
            "gets +1/+1 for each basic land type",
        ),
        (
            "Void Mirror Variant",
            "Whenever a player casts a spell, if no colored mana was spent to cast it, counter that spell.",
            "if no colored mana was spent",
        ),
        (
            "Wood Elemental Variant",
            "As this creature enters, sacrifice any number of untapped Forests.\nWood Elemental's power and toughness are each equal to the number of Forests sacrificed as it entered.",
            "sacrificed as it entered",
        ),
        (
            "Bruna Variant",
            "Flying, vigilance\nWhenever Bruna attacks or blocks, you may attach to it any number of Auras on the battlefield and you may put onto the battlefield attached to it any number of Aura cards that could enchant it from your graveyard and/or hand.",
            "any number of auras",
        ),
        (
            "Kashi Variant",
            "Whenever this creature deals combat damage to a creature, tap that creature and it doesn't untap during its controller's next untap step.",
            "doesn't untap during its controller's next untap step",
        ),
        (
            "Pawn Variant",
            "Whenever this creature or another nontoken creature you control dies, you may create a 0/1 colorless Eldrazi Spawn creature token. It has \"Sacrifice this token: Add {C}.\"",
            "sacrifice this token: add {c}",
        ),
        (
            "Soulflayer Variant",
            "Delve\nIf a creature card with flying was exiled with this creature's delve ability, this creature has flying. The same is true for first strike, double strike, deathtouch, haste, hexproof, indestructible, lifelink, reach, trample, and vigilance.",
            "the same is true",
        ),
        (
            "Wary Farmer Variant",
            "At the beginning of your end step, if another creature entered the battlefield under your control this turn, surveil 1.",
            "another creature entered the battlefield",
        ),
        (
            "Druid of Purification Variant",
            "When this creature enters, starting with you, each player may choose an artifact or enchantment you don't control. Destroy each permanent chosen this way.",
            "starting with you, each player may choose",
        ),
    ];

    for (name, text, _expected) in cases {
        stacker::maybe_grow(1024 * 1024, 64 * 1024 * 1024, || {
            let mut builder = CardDefinitionBuilder::new(CardId::from_raw(1), name);
            if name == "Lineprancers Variant" {
                builder = builder
                    .card_types(vec![CardType::Creature])
                    .power_toughness(PowerToughness::fixed(2, 2));
            }
            let def = builder
                .parse_text(text)
                .unwrap_or_else(|err| panic!("{name} should parse: {err:?}"));

            let rendered = unprocessed_compiled_lines(&def)
                .join(" ")
                .to_ascii_lowercase();
            assert!(
                !rendered.contains("unsupported"),
                "expected debug-safe AST rendering without unsupported markers for {name}, got {rendered}"
            );
        });
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn hearth_elemental_shared_graveyard_domain_round_trips_exactly() {
    let oracle = "This spell costs {X} less to cast, where X is the number of cards in your graveyard that are instant cards, sorcery cards, and/or have an Adventure.";
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Hearth Elemental Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(oracle)
        .expect("Hearth Elemental cost reduction should parse");

    assert_eq!(unprocessed_compiled_lines(&def), vec![oracle.to_string()]);
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn legion_leadership_keeps_both_coordinated_duration_actions() {
    let oracle = "Until end of turn, double target creature's power and it gains first strike.";
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Legion Leadership Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(oracle)
        .expect("Legion Leadership coordinated duration should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("double target creature's power")
            && rendered.contains("gains first strike")
            && !rendered.contains("Each creature"),
        "expected both coordinated target actions, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn soulflayer_delve_exiled_keywords_grant_to_source() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Soulflayer Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Delve\nIf a creature card with flying was exiled with this creature's delve ability, this creature has flying. The same is true for first strike, double strike, deathtouch, haste, hexproof, indestructible, lifelink, reach, trample, and vigilance.",
        )
        .expect("Soulflayer-style delve ability chain should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("this creature has flying")
            && rendered.contains("exiled with this creature")
            && (rendered.contains("this creature has vigilance")
                || (rendered.contains("the same is true") && rendered.contains("vigilance"))),
        "expected Soulflayer keywords to be granted to the source conditionally, got {rendered}"
    );
    assert!(
        !rendered.contains("all creature cards exiled with this creature"),
        "Soulflayer should not grant keywords to the exiled cards, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn kain_variant_keeps_control_change_if_do_and_life_loss() {
    let def = parse_oracle_card_definition("Kain, Traitorous Dragoon");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        (rendered.contains("during your turn") || rendered.contains("your turn"))
            && (rendered.contains("kain has flying")
                || rendered.contains("this creature has flying"))
            && (rendered.contains("gains control of this creature")
                || rendered.contains("gain control of this creature")
                || rendered.contains("gains control of kain")
                || rendered.contains("gain control of kain"))
            && rendered.contains("if they do")
            && rendered.contains("draw that many cards")
            && rendered.contains("create that many tapped treasure tokens")
            && rendered.contains("lose that much life"),
        "expected the rendered Kain text to preserve the control-change chain, got {rendered}"
    );

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("ChangeControllerToPlayer") && debug.contains("LoseLife"),
        "expected Kain's lowered ability to keep gain-control and life-loss effects, got {debug}"
    );
    assert!(
        debug.contains("IfEffect") || debug.contains("IfResult") || debug.contains("Conditional"),
        "expected Kain's lowered ability to keep the If they do clause, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn short_name_source_references_are_preserved_as_surface_hints() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Kain, Traitorous Dragoon")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}: Untap Kain.")
        .expect("short-name source reference should parse structurally");

    let AbilityKind::Activated(activated) = &def.abilities[0].kind else {
        panic!("expected activated ability, got {:#?}", def.abilities);
    };
    let untap = activated.effects.flattened_default_effects()[0]
        .downcast_ref::<UntapEffect>()
        .expect("expected untap effect");
    assert_eq!(
        untap.target.source_reference_surface(),
        Some(&SourceReferenceSurface::ShortName("Kain".to_string()))
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn this_creature_source_references_are_preserved_as_surface_hints() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Surface Source Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{T}: Untap this creature.")
        .expect("this-creature source reference should parse");

    let AbilityKind::Activated(activated) = &def.abilities[0].kind else {
        panic!("expected activated ability, got {:#?}", def.abilities);
    };
    let untap = activated.effects.flattened_default_effects()[0]
        .downcast_ref::<UntapEffect>()
        .expect("expected untap effect");
    assert_eq!(
        untap.target.source_reference_surface(),
        Some(&SourceReferenceSurface::ThisPermanentType(
            "this creature".to_string()
        ))
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_answered_prayers_keeps_life_gain_and_angel_animation() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Answered Prayers Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever a creature you control enters, you gain 1 life. If this enchantment isn't a creature, it becomes a 3/3 Angel creature with flying in addition to its other types until end of turn.",
        )
        .expect("Answered Prayers text should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("GainLifeEffect"),
        "expected the enter trigger to include life gain, got {debug}"
    );
    assert!(
        debug.contains("target: Source") || debug.contains("target_spec: Some(Source)"),
        "expected the animation clause to target the source permanent, got {debug}"
    );
    assert!(
        debug.contains("AddSubtypes") && debug.contains("Angel"),
        "expected the animation clause to add Angel, got {debug}"
    );
    assert!(
        debug.contains("flying"),
        "expected the animation clause to grant flying, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("gain 1 life") && rendered.contains("3/3 angel creature with flying"),
        "expected oracle-like rendered text for Answered Prayers, got {rendered}"
    );
    assert!(
        rendered.contains("in addition to its other types"),
        "expected the source animation to preserve existing types, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_returned_object_pronoun_static_followup_stays_in_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Returned Angel Followup Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever a nontoken, non-Angel creature you control dies, return that card to the battlefield under its owner's control with a +1/+1 counter on it. It has flying and is an Angel in addition to its other types.",
        )
        .expect("returned-object pronoun follow-up should parse inside the trigger");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected a triggered ability");
    let effects_debug = format!("{:?}", triggered.effects);
    assert!(
        effects_debug.contains("ApplyContinuousEffect")
            && effects_debug.contains("returned_")
            && effects_debug.contains("AddSubtypes")
            && effects_debug.contains("Angel")
            && effects_debug.contains("flying"),
        "expected returned object to receive flying and Angel modifications inside the trigger, got {effects_debug}"
    );
    assert!(
        !def.abilities.iter().any(|ability| matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == crate::static_abilities::StaticAbilityId::AddSubtypes
        )),
        "returned-object Angel follow-up must not become a detached static ability: {:?}",
        def.abilities
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered.contains("with a +1/+1 counter")
            && rendered_lower.contains("flying")
            && rendered_lower.contains("angel"),
        "expected returned-object modifications in compiled text, got {rendered}"
    );
    assert_eq!(
        rendered,
        "Whenever a nontoken non-angel creature you control dies, return that card to the battlefield under its owner's control with a +1/+1 counter on it. It has flying and is an Angel in addition to its other types."
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_returned_object_color_subtype_static_followup_stays_in_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Returned Zombie Followup Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever a creature you don't control dies, return it to the battlefield under your control with an additional +1/+1 counter on it at the beginning of the next end step. That creature is a black Zombie in addition to its other colors and types.",
        )
        .expect("returned-object color/type follow-up should parse inside the trigger");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected a triggered ability");
    let effects_debug = format!("{:?}", triggered.effects);
    assert!(
        effects_debug.contains("ScheduleDelayedTriggerEffect")
            && effects_debug.contains("AddColors")
            && effects_debug.contains("AddSubtypes")
            && effects_debug.contains("Zombie"),
        "expected returned object to receive color and Zombie modifications inside the trigger, got {effects_debug}"
    );
    assert!(
        !def.abilities.iter().any(|ability| matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == crate::static_abilities::StaticAbilityId::AddSubtypes
        )),
        "returned-object Zombie follow-up must not become a detached static ability: {:?}",
        def.abilities
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        (rendered_lower.contains("with a +1/+1 counter")
            || rendered_lower.contains("with an additional +1/+1 counter"))
            && rendered_lower.contains("black zombie")
            && rendered_lower.contains("beginning of the next end step"),
        "expected returned-object color/type modifications in compiled text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_your_turn_keyword_grants_preserve_during_vs_as_long_surface() {
    let during = CardDefinitionBuilder::new(CardId::new(), "During Turn Keyword Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("During your turn, this creature has first strike.")
        .expect("during-your-turn keyword grant should parse");
    let during_debug = format!("{:?}", during.abilities);
    assert!(
        during_debug.contains("ActivationTiming")
            && during_debug.contains("DuringYourTurn")
            && !during_debug.contains("condition: YourTurn"),
        "expected during-your-turn source to preserve an activation-timing condition, got {during_debug}"
    );
    let during_rendered = unprocessed_compiled_lines(&during).join(" ");
    assert_eq!(
        during_rendered,
        "During your turn, this creature has first strike."
    );

    let as_long = CardDefinitionBuilder::new(CardId::new(), "As Long Keyword Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("As long as it's your turn, this creature has first strike.")
        .expect("as-long-as-your-turn keyword grant should parse");
    let as_long_debug = format!("{:?}", as_long.abilities);
    assert!(
        as_long_debug.contains("condition: Some(YourTurn)"),
        "expected as-long source to keep the plain YourTurn condition, got {as_long_debug}"
    );
    let as_long_rendered = unprocessed_compiled_lines(&as_long).join(" ");
    assert!(
        as_long_rendered == "As long as it's your turn, this creature has first strike."
            || as_long_rendered == "During your turn, this creature has first strike.",
        "expected as-long keyword grant to render as an active-your-turn surface, got {as_long_rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn separately_authored_during_turn_keyword_statics_keep_their_line_boundaries() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Turn Static Boundary Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "During your turn, this creature has first strike.\nDuring your turn, creatures you control with +1/+1 counters on them have first strike.",
        )
        .expect("separate during-turn statics should parse");

    let debug = format!("{def:#?}");
    assert_eq!(
        unprocessed_compiled_lines(&def),
        vec![
            "During your turn, this creature has first strike.".to_string(),
            "During your turn, creatures you control with +1/+1 counters on them have first strike."
                .to_string(),
        ],
        "separately authored static lines must not be fused or share subjects: {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_veiled_apparition_uses_source_gate_and_granted_upkeep_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Veiled Apparition Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "When an opponent casts a spell, if this permanent is an enchantment, it becomes a 3/3 Illusion creature with flying and \"At the beginning of your upkeep, sacrifice this creature unless you pay {1}{U}.\"",
        )
        .expect("Veiled Apparition text should parse");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Veiled Apparition should have a triggered ability");

    assert!(
        matches!(
            triggered.intervening_if.as_ref(),
            Some(crate::ConditionExpr::SourceMatches(filter))
                if filter.card_types.as_slice() == [CardType::Enchantment]
        ),
        "the enchantment check should be an intervening-if on the source, got {:?}",
        triggered.intervening_if
    );

    let default_effects = &triggered.effects.segments[0].default_effects;
    assert!(
        default_effects.iter().all(|effect| effect
            .downcast_ref::<crate::effects::UnlessPaysEffect>()
            .is_none()),
        "the upkeep sacrifice-unless-pay clause must not resolve in the spell-cast trigger: {default_effects:?}"
    );

    let animation = default_effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ApplyContinuousEffect>())
        .expect("spell-cast trigger should animate the source");
    assert!(
        matches!(animation.target_spec.as_ref(), Some(ChooseSpec::Source)),
        "animation should apply to the source permanent, got {:?}",
        animation.target_spec
    );
    assert!(
        matches!(
            animation.modification.as_ref(),
            Some(crate::continuous::Modification::SetCardTypes(card_types))
                if card_types.as_slice() == [CardType::Creature]
        ),
        "animation should set the source's creature type, got {:?}",
        animation.modification
    );

    let granted_upkeep = animation
        .additional_modifications
        .iter()
        .find_map(|modification| match modification {
            crate::continuous::Modification::AddAbilityGeneric(ability) => match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            },
            _ => None,
        })
        .expect("animation should grant the quoted upkeep triggered ability");
    assert!(
        format!("{:?}", granted_upkeep.trigger).contains("BeginningOfUpkeep"),
        "granted ability should trigger at the beginning of upkeep, got {:?}",
        granted_upkeep.trigger
    );
    assert!(
        granted_upkeep.effects.iter().any(|effect| effect
            .downcast_ref::<crate::effects::UnlessPaysEffect>()
            .is_some()),
        "granted upkeep ability should contain the sacrifice-unless-pay effect: {granted_upkeep:?}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        !rendered.contains("spell or enchantment")
            && rendered.contains("opponent casts a spell")
            && rendered.contains("illusion creature")
            && (rendered.contains("base power and toughness 3/3")
                || rendered.contains("3/3 illusion creature"))
            && rendered.contains("flying"),
        "expected source-gated spell trigger rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn answered_prayers_trigger_gains_life_when_a_creature_enters() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Answered Prayers Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever a creature you control enters, you gain 1 life. If this enchantment isn't a creature, it becomes a 3/3 Angel creature with flying in addition to its other types until end of turn.",
        )
        .expect("Answered Prayers text should parse");

    let trigger = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Answered Prayers should have a triggered ability");

    let gain_effect = trigger
        .effects
        .iter()
        .find(|effect| format!("{:?}", effect).contains("GainLifeEffect"))
        .expect("trigger should include a gain-life effect");

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let answered_prayers_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    let creature_def = CardDefinitionBuilder::new(CardId::from_raw(2), "Answer Test Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let entering_id = game.create_object_from_definition(&creature_def, alice, Zone::Battlefield);
    let snapshot =
        crate::snapshot::ObjectSnapshot::from_object(game.object(entering_id).unwrap(), &game);
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::with_cause(
            entering_id,
            Zone::Hand,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            Some(snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    );

    let triggered = crate::triggers::check_triggers(&game, &event);
    assert!(
        triggered
            .iter()
            .any(|entry| entry.source == answered_prayers_id),
        "Answered Prayers should trigger when a creature you control enters"
    );

    let starting_life = game.player(alice).unwrap().life;
    let mut ctx = crate::effects::ExecutionContext::new_default(answered_prayers_id, alice);
    gain_effect
        .0
        .execute(&mut game, &mut ctx)
        .expect("gain life effect should resolve");
    assert_eq!(
        game.player(alice).unwrap().life,
        starting_life + 1,
        "Answered Prayers should gain 1 life when the trigger resolves"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_source_surface_for_hard_spell_effect_clauses() {
    let cases = [
        (
            "Haunting Echoes Variant",
            "Exile all cards from target player's graveyard other than basic land cards. For each card exiled this way, search that player's library for all cards with the same name as that card and exile them. Then that player shuffles.",
            "other than basic land cards",
        ),
        (
            "Fulgent Distraction Variant",
            "Choose two target creatures. Tap those creatures, then unattach all Equipment from them.",
            "unattach all equipment from them",
        ),
        (
            "Second Guess Variant",
            "Counter target spell that's the second spell cast this turn.",
            "counter target spell that's the second spell cast this turn",
        ),
        (
            "Tip the Scales Variant",
            "Sacrifice a creature. When you do, all creatures get -X/-X until end of turn, where X is the sacrificed creature's toughness.",
            "when you do, all creatures get -x/-x",
        ),
        (
            "Decimate Variant",
            "Destroy target artifact, target creature, target enchantment, and target land.",
            "destroy target artifact, target creature, target enchantment, and target land",
        ),
        (
            "Crush Underfoot Variant",
            "Choose a Giant creature you control. It deals damage equal to its power to target creature.",
            "choose a giant creature you control",
        ),
        (
            "Urza's Ruinous Blast Variant",
            "Exile all nonland permanents that aren't legendary.",
            "all nonland permanents that aren't legendary",
        ),
        (
            "Trapfinder's Trick Variant",
            "Target player reveals their hand and discards all Trap cards.",
            "discards all trap cards",
        ),
        (
            "Amnesia Variant",
            "Target player reveals their hand and discards all nonland cards.",
            "discards all nonland cards",
        ),
        (
            "Elemental Uprising Variant",
            "Target land you control becomes a 4/4 Elemental creature with haste until end of turn. It's still a land. It must be blocked this turn if able.",
            "it must be blocked this turn if able",
        ),
        (
            "Divine Smite Variant",
            "Target creature or planeswalker an opponent controls phases out. If that permanent is black, exile it instead.",
            "if that permanent is black, exile it instead",
        ),
        (
            "Aggravate Variant",
            "Aggravate deals 1 damage to each creature target player controls. Each creature dealt damage this way attacks this turn if able.",
            "each creature dealt damage this way attacks this turn if able",
        ),
        (
            "Yamabushi's Storm Variant",
            "Yamabushi's Storm deals 1 damage to each creature. If a creature dealt damage this way would die this turn, exile it instead.",
            "would die this turn, exile it instead",
        ),
        (
            "Capricious Efreet Variant",
            "At the beginning of your upkeep, choose target nonland permanent you control and up to two target nonland permanents you don't control. Destroy one of them at random.",
            "destroy one of them at random",
        ),
        (
            "Eternal Flame Variant",
            "Eternal Flame deals X damage to target opponent or planeswalker and half X damage, rounded up, to you, where X is the number of Mountains you control.",
            "half x damage, rounded up",
        ),
        (
            "Doomsday Variant",
            "Search your library and graveyard for five cards and exile the rest. Put the chosen cards on top of your library in any order. You lose half your life, rounded up.",
            "search your library and graveyard for five cards",
        ),
        (
            "Decree Variant",
            "Exile all artifacts, creatures, and lands from the battlefield, all cards from all graveyards, and all cards from all hands.\nCycling {5}{R}{R}\nWhen you cycle this card, destroy all lands.",
            "exile all artifacts, creatures, and lands",
        ),
        (
            "Kruphix Insight Variant",
            "Reveal the top six cards of your library. Put up to three enchantment cards from among them into your hand and the rest of the revealed cards into your graveyard.",
            "reveal the top six cards",
        ),
        (
            "Minions Murmurs Variant",
            "You draw X cards and you lose X life, where X is the number of creatures you control.",
            "you draw x cards and you lose x life",
        ),
        (
            "Strategic Betrayal Variant",
            "Target opponent exiles a creature they control and their graveyard.",
            "target opponent exiles a creature they control and their graveyard",
        ),
        (
            "The Fall of Kroog Variant",
            "Choose target opponent. Destroy target land that player controls. The Fall of Kroog deals 3 damage to that player and 1 damage to each creature they control.",
            "choose target opponent",
        ),
        (
            "Self-Destruct Variant",
            "Target creature you control deals X damage to any other target and X damage to itself, where X is its power.",
            "where x is its power",
        ),
        (
            "Skyreaping Variant",
            "Skyreaping deals damage to each creature with flying equal to your devotion to green.",
            "devotion to green",
        ),
        (
            "Cathartic Parting Variant",
            "The owner of target artifact or enchantment an opponent controls shuffles it into their library. You may shuffle up to four target cards from your graveyard into your library.",
            "owner of target",
        ),
        (
            "Officious Interrogation Variant",
            "This spell costs {W}{U} more to cast for each target beyond the first.\nChoose any number of target players. Investigate X times, where X is the total number of creatures those players control.",
            "investigate x times",
        ),
        (
            "Peak Eruption Variant",
            "Destroy target Mountain. Peak Eruption deals 3 damage to that land's controller.",
            "that land's controller",
        ),
        (
            "Life Variant",
            "All lands you control become 1/1 creatures until end of turn. They're still lands.",
            "they're still lands",
        ),
        (
            "Expressive Iteration Variant",
            "Look at the top three cards of your library. Put one of them into your hand, put one of them on the bottom of your library, and exile one of them. You may play the exiled card this turn.",
            "one of them into your hand",
        ),
        (
            "Hustle Variant",
            "Target creature attacks or blocks this turn if able.",
            "attacks or blocks this turn if able",
        ),
        (
            "Cut Down Variant",
            "Destroy target creature with total power and toughness 5 or less.",
            "total power and toughness 5 or less",
        ),
        (
            "Incriminate Variant",
            "Choose two target creatures controlled by the same player. That player sacrifices one of them of their choice.",
            "controlled by the same player",
        ),
        (
            "Mass Polymorph Variant",
            "Exile all creatures you control, then reveal cards from the top of your library until you reveal that many creature cards. Put all creature cards revealed this way onto the battlefield, then shuffle the rest of the revealed cards into your library.",
            "that many creature cards",
        ),
        (
            "Consign to the Pit Variant",
            "Destroy target creature. Consign to the Pit deals 2 damage to that creature's controller.",
            "that creature's controller",
        ),
        (
            "Wild Ricochet Variant",
            "You may choose new targets for target instant or sorcery spell. Then copy that spell. You may choose new targets for the copy.",
            "choose new targets",
        ),
        (
            "Prismatic Undercurrents Variant",
            "Vivid — When this enchantment enters, search your library for up to X basic land cards, where X is the number of colors among permanents you control. Reveal those cards, put them into your hand, then shuffle.\nYou may play an additional land on each of your turns.",
            "where x is the number of color among permanents",
        ),
        (
            "Sumala Rumblers Variant",
            "Sumala Rumblers's power is equal to the number of creatures you control.\nMyriad",
            "myriad",
        ),
        (
            "Regal Sliver Variant",
            "Sliver creatures you control have \"When this creature enters, Slivers you control get +1/+1 until end of turn if you're the monarch. Otherwise, you become the monarch.\"",
            "otherwise, you become the monarch",
        ),
        (
            "Deploy to the Front Variant",
            "Create X 1/1 white Soldier creature tokens, where X is the number of creatures on the battlefield.",
            "where x is the number of creatures",
        ),
        (
            "Culling Mark Variant",
            "Target creature blocks this turn if able.",
            "blocks this turn if able",
        ),
        (
            "Brace for Impact Variant",
            "Prevent all damage that would be dealt to target multicolored creature this turn. For each 1 damage prevented this way, put a +1/+1 counter on that creature.",
            "for each 1 damage prevented this way",
        ),
        (
            "Cryoclasm Variant",
            "Destroy target Plains or Island. Cryoclasm deals 3 damage to that land's controller.",
            "that land's controller",
        ),
        (
            "Coax Variant",
            "You may reveal an Eldrazi card you own from outside the game or choose a face-up Eldrazi card you own in exile. Put that card into your hand.",
            "outside the game or choose a face-up",
        ),
        (
            "Mercy Killing Variant",
            "Target creature's controller sacrifices it, then creates X 1/1 green and white Elf Warrior creature tokens, where X is that creature's power.",
            "target creature's controller sacrifices it",
        ),
        (
            "Boon of Safety Variant",
            "Put a shield counter on target creature.\nScry 1.",
            "scry 1",
        ),
        (
            "Gods Willing Variant",
            "Target creature you control gains protection from the color of your choice until end of turn.\nScry 1.",
            "scry 1",
        ),
        (
            "Capital Punishment Variant",
            "Council's dilemma — Starting with you, each player votes for death or taxes. Each opponent sacrifices a creature of their choice for each death vote and discards a card for each taxes vote.",
            "council's dilemma",
        ),
        (
            "Boneyard Parley Variant",
            "Exile up to five target creature cards from graveyards. An opponent separates those cards into two piles. Put all cards from the pile of your choice onto the battlefield under your control and the rest into their owners' graveyards.",
            "opponent separates those cards into two piles",
        ),
        (
            "Tribal Unity Variant",
            "Creatures of the creature type of your choice get +X/+X until end of turn.",
            "creature type of your choice",
        ),
        (
            "Graceful Reprieve Variant",
            "When target creature dies this turn, return that card to the battlefield under its owner's control.",
            "when target creature dies this turn",
        ),
        (
            "Tendrils Variant",
            "Tendrils of Corruption deals X damage to target creature and you gain X life, where X is the number of Swamps you control.",
            "where x is the number of swamps",
        ),
        (
            "Remove Enchantments Variant",
            "Return to your hand all enchantments you both own and control, all Auras you own attached to permanents you control, and all Auras you own attached to attacking creatures your opponents control. Then destroy all other enchantments you control, all other Auras attached to permanents you control, and all other Auras attached to attacking creatures your opponents control.",
            "return to your hand all enchantments",
        ),
        (
            "Wash Out Variant",
            "Return all permanents of the color of your choice to their owners' hands.",
            "return all permanents of the color of your choice",
        ),
        (
            "Rise of the Dark Realms Variant",
            "Put all creature cards from all graveyards onto the battlefield under your control.",
            "all creature cards from all graveyards",
        ),
        (
            "Breaking Point Variant",
            "Any player may have Breaking Point deal 6 damage to them. If no one does, destroy all creatures. Creatures destroyed this way can't be regenerated.",
            "any player may have",
        ),
        (
            "Inferno Trap Variant",
            "If you've been dealt damage by two or more creatures this turn, you may pay {R} rather than pay this spell's mana cost.\nInferno Trap deals 4 damage to target creature.",
            "inferno trap deals 4 damage",
        ),
        (
            "Espers to Magicite Variant",
            "Exile each opponent's graveyard. When you do, choose up to one target creature card exiled this way. Create a token that's a copy of that card, except it's an artifact and it loses all other card types.",
            "loses all other card types",
        ),
        (
            "Blossoming Wreath Variant",
            "You gain life equal to the number of creature cards in your graveyard.",
            "gain life equal to the number of creature cards",
        ),
        (
            "Ill-Timed Explosion Variant",
            "Draw two cards. Then you may discard two cards. When you do, Ill-Timed Explosion deals X damage to each creature, where X is the greatest mana value among cards discarded this way.",
            "greatest mana value among cards discarded this way",
        ),
        (
            "Mythos Variant",
            "Destroy target nonland permanent if it's a creature or if {G}{W} was spent to cast this spell.",
            "if {g}{w} was spent to cast this spell",
        ),
        (
            "Never Happened Variant",
            "Target opponent reveals their hand. You choose a nonland card from that player's graveyard or hand and exile it.",
            "graveyard or hand",
        ),
        (
            "Essence Harvest Variant",
            "Target player loses X life and you gain X life, where X is the greatest power among creatures you control.",
            "where x is the greatest power",
        ),
        (
            "Extinction Variant",
            "Destroy all creatures of the creature type of your choice.",
            "creature type of your choice",
        ),
        (
            "Double Trouble Variant",
            "Double the power of each creature you control until end of turn.",
            "double the power of each creature",
        ),
        (
            "Memoricide Variant",
            "Choose a nonland card name. Search target player's graveyard, hand, and library for any number of cards with that name and exile them. Then that player shuffles.",
            "search target player's graveyard, hand, and library",
        ),
        (
            "Gelatinous Genesis Variant",
            "Create X X/X green Ooze creature tokens.",
            "create x x/x green ooze",
        ),
        (
            "Primal Surge Variant",
            "Exile the top card of your library. If it's a permanent card, you may put it onto the battlefield. If you do, repeat this process.",
            "repeat this process",
        ),
        (
            "Destructive Revelry Variant",
            "Destroy target artifact or enchantment. Destructive Revelry deals 2 damage to that permanent's controller.",
            "that permanent's controller",
        ),
        (
            "Council's Judgment Variant",
            "Will of the council — Starting with you, each player votes for a nonland permanent you don't control. Exile each permanent with the most votes or tied for most votes.",
            "most votes",
        ),
        (
            "Lucid Dreams Variant",
            "Draw X cards, where X is the number of card types among cards in your graveyard.",
            "where x is the number of card types",
        ),
        (
            "Rivals' Duel Variant",
            "Choose two target creatures that share no creature types. Those creatures fight each other.",
            "share no creature types",
        ),
        (
            "Hellish Rebuke Variant",
            "Until end of turn, permanents your opponents control gain \"When this permanent deals damage to the player who cast Hellish Rebuke, sacrifice this permanent. You lose 2 life.\"",
            "gain \"when this permanent deals damage",
        ),
        (
            "Smash to Smithereens Variant",
            "Destroy target artifact. Smash to Smithereens deals 3 damage to that artifact's controller.",
            "that artifact's controller",
        ),
        (
            "Dwarven Catapult Variant",
            "Dwarven Catapult deals X damage divided evenly, rounded down, among all creatures target opponent controls.",
            "divided evenly, rounded down",
        ),
        (
            "Breaking Wave Variant",
            "You may cast this spell as though it had flash if you pay {2} more to cast it.\nSimultaneously untap all tapped creatures and tap all untapped creatures.",
            "as though it had flash",
        ),
        (
            "Golden Wish",
            "You may reveal an artifact or enchantment card you own from outside the game and put it into your hand. Exile Golden Wish.",
            "from outside the game",
        ),
        (
            "All Is Dust Variant",
            "Each player sacrifices all permanents they control that are one or more colors.",
            "one or more color",
        ),
        (
            "Aether Burst Variant",
            "Return up to X target creatures to their owners' hands, where X is one plus the number of cards named Aether Burst in all graveyards as you cast this spell.",
            "where x is one plus",
        ),
        (
            "Grim Reminder Variant",
            "Search your library for a nonland card and reveal it. Each opponent who cast a spell this turn with the same name as that card loses 6 life. Then shuffle.\n{B}{B}: Return this card from your graveyard to your hand. Activate only during your upkeep.",
            "each opponent who cast a spell this turn",
        ),
        (
            "Nissa's Pilgrimage Variant",
            "Search your library for up to two basic Forest cards, reveal those cards, and put one onto the battlefield tapped and the rest into your hand. Then shuffle.\nSpell mastery — If there are two or more instant and/or sorcery cards in your graveyard, search your library for up to three basic Forest cards instead of two.",
            "up to two basic forest",
        ),
        (
            "Ruin in Their Wake Variant",
            "Devoid\nSearch your library for a basic land card and reveal it. You may put that card onto the battlefield tapped if you control a land named Wastes. Otherwise, put that card into your hand. Then shuffle.",
            "if you control a land named wastes",
        ),
        (
            "Perish the Thought Variant",
            "Target opponent reveals their hand. You choose a card from it. That player shuffles that card into their library.",
            "shuffles that card into their library",
        ),
    ];

    for (name, text, _expected) in cases {
        stacker::maybe_grow(1024 * 1024, 64 * 1024 * 1024, || {
            let def = CardDefinitionBuilder::new(CardId::from_raw(1), name)
                .parse_text(text)
                .unwrap_or_else(|err| panic!("{name} should parse: {err:?}"));

            let rendered = unprocessed_compiled_lines(&def)
                .join(" ")
                .to_ascii_lowercase();
            assert!(
                !rendered.contains("unsupported"),
                "expected debug-safe AST rendering without unsupported markers for {name}, got {rendered}"
            );
        });
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn doomsday_keeps_the_cross_zone_partition_and_exact_surface() {
    let oracle = "Search your library and graveyard for five cards and exile the rest. Put the chosen cards on top of your library in any order. You lose half your life, rounded up.";
    let definition = CardDefinitionBuilder::new(CardId::from_raw(1), "Doomsday Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(oracle)
        .expect("cross-zone search partition should parse");
    let debug = format!("{:#?}", definition.spell_effect);

    assert!(debug.contains("ChooseObjectsEffect"), "{debug}");
    assert!(debug.contains("IsNotTaggedObject"), "{debug}");
    assert!(debug.contains("HalfLifeTotalRoundedUp"), "{debug}");
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![oracle.to_string()]
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_haunting_echoes_exception_and_target_library_search() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Haunting Echoes Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Exile all cards from target player's graveyard other than basic land cards. For each card exiled this way, search that player's library for all cards with the same name as that card and exile them. Then that player shuffles.",
        )
        .expect("Haunting Echoes text should parse");

    let rendered = debug_compiled_lines(&def).join(" ");
    assert!(
        rendered
            .contains("Exile all cards from target player's graveyard other than basic land cards"),
        "expected basic-land exception to render from structure, got {rendered}"
    );
    assert!(
        rendered.contains(
            "search that player's library for all cards with the same name as that card and exile them"
        ),
        "expected same-name target-library search to render, got {rendered}"
    );

    let program = def.spell_effect.as_ref().expect("spell effect");
    let effects = &program.segments[0].default_effects;
    let tagged_exile = effects[0]
        .downcast_ref::<TaggedEffect>()
        .expect("initial exile should tag cards exiled this way");
    let exile = tagged_exile
        .effect
        .downcast_ref::<crate::effects::ExileEffect>()
        .expect("first effect should exile a graveyard subset");
    let ChooseSpec::All(exile_filter) = &exile.spec else {
        panic!("initial exile should use an all-object filter");
    };
    assert_eq!(
        exile_filter.any_of.len(),
        2,
        "basic land exception should lower as not-land or not-basic, got {exile_filter:#?}"
    );
    assert!(
        exile_filter
            .any_of
            .iter()
            .any(|branch| branch.excluded_card_types.contains(&CardType::Land))
            && exile_filter
                .any_of
                .iter()
                .any(|branch| branch.excluded_supertypes.contains(&Supertype::Basic)),
        "expected structural non-basic-land exclusion, got {exile_filter:#?}"
    );

    let iteration_effect = program
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .find(|effect| {
            effect
                .downcast_ref::<crate::effects::ForEachObject>()
                .is_some()
                || effect
                    .downcast_ref::<crate::effects::ForEachTaggedEffect>()
                    .is_some()
        })
        .expect("a later segment should iterate cards exiled this way");
    let iterated_effects =
        if let Some(for_each) = iteration_effect.downcast_ref::<crate::effects::ForEachObject>() {
            for_each.effects.as_slice()
        } else if let Some(for_each) =
            iteration_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()
        {
            assert!(
                for_each.tag == tagged_exile.tag
                    || for_each.tag.as_str() == crate::tag::SOURCE_EXILED_TAG,
                "tagged iteration should use the cards exiled by the first effect, got {:?}",
                for_each.tag
            );
            for_each.effects.as_slice()
        } else {
            panic!("second effect should iterate cards exiled this way");
        };
    let iterated_effects = match iterated_effects {
        [sequence_effect] => sequence_effect
            .downcast_ref::<crate::effects::SequenceEffect>()
            .map_or(iterated_effects, |sequence| sequence.effects.as_slice()),
        _ => iterated_effects,
    };
    let search = iterated_effects[0]
        .downcast_ref::<ChooseObjectsEffect>()
        .expect("iterated effect should search the target player's library");
    assert!(
        matches!(
            search.filter.owner,
            Some(PlayerFilter::IteratedPlayer)
                | Some(PlayerFilter::Target(_))
                | Some(PlayerFilter::AliasedTarget(_))
        ),
        "expected the search owner to remain linked to the exiled target player's cards, got {:?}",
        search.filter.owner
    );
    assert_eq!(
        search.search_mode,
        crate::effect::SearchSelectionMode::AllMatching
    );
    assert!(
        search.filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::SameNameAsTagged
                && constraint.tag.as_str() == "__it__"
        }),
        "expected search filter to compare each library card name to the iterated exiled card"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_total_power_toughness_target_filter_as_single_target_constraint() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cut Down Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Destroy target creature with total power and toughness 5 or less.")
        .expect("Cut Down target filter should parse");

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("total_power_toughness: Some")
            && debug.contains("LessThanOrEqual")
            && debug.contains("5,")
            && debug.contains("toughness: None"),
        "expected one target filter with total power/toughness <= 5, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("destroy target creature with total power and toughness 5 or less"),
        "expected Cut Down source surface, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_once_each_turn_play_from_exile_line_is_rejected() {
    let err = CardDefinitionBuilder::new(CardId::from_raw(1), "Evelyn Variant")
        .parse_text(
            "Once each turn, you may play a card from exile with a collection counter on it if it was exiled by an ability you controlled, and you may spend mana as though it were mana of any color to cast it.",
        )
        .expect_err("once-each-turn play-from-exile fallback line should be rejected");
    let debug = format!("{err:?}").to_ascii_lowercase();
    assert!(
        debug.contains("unsupported static clause"),
        "expected unsupported static clause error, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_manabond_reveal_hand_put_lands_from_it() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Manabond Variant")
        .parse_text(
            "At the beginning of your end step, you may reveal your hand and put all land cards from it onto the battlefield. If you do, discard your hand.",
        )
        .expect("manabond reveal/put-from-it clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("reveal your hand"),
        "expected reveal-hand rendering, got {rendered}"
    );
    assert!(
        rendered.contains("from your hand")
            || rendered.contains("cards in your hand")
            || rendered.contains("your hand to the battlefield"),
        "expected lands to be moved from hand, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_each_player_puts_card_from_hand_on_top() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sadistic Variant")
        .parse_text(
            "When this creature dies, each player puts a card from their hand on top of their library.",
        )
        .expect("sadistic-augermage style clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("each player puts a card from their hand on top of their library"),
        "expected compact each-player puts wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_conditional_doesnt_untap_static_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Alirios Variant")
        .parse_text(
            "This creature doesn't untap during your untap step if you control a Reflection.",
        )
        .expect("conditional doesn't-untap line should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("doesn't untap during your untap step if you control a reflection")
            || rendered.contains("doesnt untap during your untap step if you control a reflection"),
        "expected untap condition to be preserved, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_agathas_soul_cauldron_static_lines() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Agatha's Soul Cauldron")
        .parse_text(
            "You may spend mana as though it were mana of any color to activate abilities of creatures you control.\n\
Creatures you control with +1/+1 counters on them have all activated abilities of all creature cards exiled with Agatha's Soul Cauldron.\n\
{T}: Exile target card from a graveyard. When a creature card is exiled this way, put a +1/+1 counter on target creature you control.",
        )
        .expect("Agatha's Soul Cauldron text should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "spend mana as though it were mana of any color to activate abilities of creatures you control"
        ),
        "expected filtered mana-spend permission, got {rendered}"
    );
    assert!(
        rendered.contains("have all activated abilities of all creature cards exiled with this"),
        "expected granted copied activated abilities clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn necrotic_ooze_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Necrotic Ooze");
    let ability_debug = format!("{:#?}", def.abilities);
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        ability_debug.contains("CopyActivatedAbilities")
            && ability_debug.contains("SourceIsInZone(Battlefield)")
            && ability_debug.contains("Graveyard")
            && ability_debug.contains("Creature"),
        "Necrotic Ooze should structurally model a battlefield-gated graveyard-creature activated-ability copy effect, got {ability_debug}"
    );
    assert!(
        rendered.contains("as long as")
            && rendered.contains("this creature is on the battlefield")
            && rendered.contains("all activated abilities")
            && rendered.contains("creature cards")
            && rendered.contains("graveyards"),
        "expected Necrotic Ooze compiled text to preserve the source-on-battlefield activated-ability copy clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_existing_mana_spend_as_any_color_static_patterns() {
    let lattice = CardDefinitionBuilder::new(CardId::from_raw(1), "Mycosynth Lattice Variant")
        .parse_text("Players may spend mana as though it were mana of any color.")
        .expect("global mana-spend permission should parse");
    let lattice_rendered = unprocessed_compiled_lines(&lattice)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        lattice_rendered.contains("players may spend mana as though it were mana of any color"),
        "expected global mana-spend permission, got {lattice_rendered}"
    );

    let refractor = CardDefinitionBuilder::new(CardId::from_raw(1), "Manascape Refractor")
        .parse_text(
            "You may spend mana as though it were mana of any color to pay the activation costs of Manascape Refractor's abilities.",
        )
        .expect("source activation mana-spend permission should parse");
    let refractor_rendered = unprocessed_compiled_lines(&refractor)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        refractor_rendered
            .contains("spend mana as though it were mana of any color to pay the activation costs"),
        "expected source activation mana-spend permission, got {refractor_rendered}"
    );
}

#[test]
pub(super) fn celestial_dawn_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Celestial Dawn");
    let def = parse_oracle_card_definition("Celestial Dawn");
    let rendered = canonical_compiled_lines(&def).join("\n");
    let rendered_lower = rendered.to_ascii_lowercase();
    let ability_debug = format!("{:#?}", def.abilities);

    assert!(
        rendered_lower.contains("lands you control are plains"),
        "Celestial Dawn should render the Plains-changing static ability, got {rendered}"
    );
    assert!(
        rendered_lower.contains("nonland permanents you control are white. the same is true for spells you control and nonland cards you own that aren't on the battlefield"),
        "Celestial Dawn should render the same-is-true color static ability, got {rendered}"
    );
    assert!(
        rendered_lower.contains("you may spend white mana as though it were mana of any color. you may spend other mana only as though it were colorless mana"),
        "Celestial Dawn should render the white-mana spend permission and other-mana restriction, got {rendered}"
    );
    assert!(
        ability_debug.contains("ManaSpendPermission")
            && ability_debug.contains("any_color_mana_symbol: Some(White)")
            && ability_debug.contains("other_mana_only_as_colorless: true"),
        "Celestial Dawn should structurally lower its mana-spend rule, got {ability_debug}"
    );
}

#[test]
pub(super) fn celestial_dawn_mana_spend_runtime_uses_white_as_any_color_only() {
    let def = parse_oracle_card_definition("Celestial Dawn");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.update_cant_effects();

    let blue_spell =
        CardDefinitionBuilder::new(CardId::from_raw(700_900), "Celestial Dawn Blue Cost Probe")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
            .card_types(vec![CardType::Sorcery])
            .build();
    let blue_spell_id = game.create_object_from_definition(&blue_spell, alice, Zone::Stack);
    let blue_cost = ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]);

    game.player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::White, 1);
    let view = crate::derived_view::DerivedGameView::new(&game);
    assert!(
        view.can_potentially_pay_with_reason(
            alice,
            Some(blue_spell_id),
            &blue_cost,
            0,
            crate::costs::PaymentReason::CastSpell,
        ),
        "Celestial Dawn should also make the derived affordability path see white mana as usable for blue"
    );
    assert!(
        game.can_pay_mana_cost(alice, Some(blue_spell_id), &blue_cost, 0),
        "Celestial Dawn should let white mana pay a blue pip"
    );
    assert!(
        game.try_pay_mana_cost(alice, Some(blue_spell_id), &blue_cost, 0),
        "white mana should actually be spent as blue"
    );
    assert_eq!(
        game.player(alice).expect("Alice exists").mana_pool.total(),
        0,
        "white mana should be consumed by the blue-pip payment"
    );
}

#[test]
pub(super) fn celestial_dawn_mana_spend_runtime_treats_other_mana_as_colorless_only() {
    let def = parse_oracle_card_definition("Celestial Dawn");
    let alice = PlayerId::from_index(0);

    let blue_cost = ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]);
    let colorless_cost = ManaCost::from_pips(vec![vec![ManaSymbol::Colorless]]);
    let spell = CardDefinitionBuilder::new(CardId::from_raw(700_901), "Celestial Dawn Cost Probe")
        .mana_cost(blue_cost.clone())
        .card_types(vec![CardType::Sorcery])
        .build();

    let mut nonwhite_for_blue =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    nonwhite_for_blue.create_object_from_definition(&def, alice, Zone::Battlefield);
    nonwhite_for_blue.update_cant_effects();
    let blue_spell_id = nonwhite_for_blue.create_object_from_definition(&spell, alice, Zone::Stack);
    nonwhite_for_blue
        .player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::Red, 1);
    let nonwhite_view = crate::derived_view::DerivedGameView::new(&nonwhite_for_blue);
    assert!(
        !nonwhite_view.can_potentially_pay_with_reason(
            alice,
            Some(blue_spell_id),
            &blue_cost,
            0,
            crate::costs::PaymentReason::CastSpell,
        ),
        "Celestial Dawn should also make the derived affordability path reject nonwhite mana for colored pips"
    );
    assert!(
        !nonwhite_for_blue.can_pay_mana_cost(alice, Some(blue_spell_id), &blue_cost, 0),
        "Celestial Dawn should not let nonwhite mana pay colored pips"
    );

    let mut nonwhite_for_colorless =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    nonwhite_for_colorless.create_object_from_definition(&def, alice, Zone::Battlefield);
    nonwhite_for_colorless.update_cant_effects();
    let colorless_spell_id =
        nonwhite_for_colorless.create_object_from_definition(&spell, alice, Zone::Stack);
    nonwhite_for_colorless
        .player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::Red, 1);
    assert!(
        nonwhite_for_colorless.can_pay_mana_cost(
            alice,
            Some(colorless_spell_id),
            &colorless_cost,
            0,
        ),
        "Celestial Dawn should let nonwhite mana be spent as colorless"
    );
    assert!(
        nonwhite_for_colorless.try_pay_mana_cost(
            alice,
            Some(colorless_spell_id),
            &colorless_cost,
            0,
        ),
        "nonwhite mana should actually be spendable as colorless"
    );

    let mut white_for_colorless =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    white_for_colorless.create_object_from_definition(&def, alice, Zone::Battlefield);
    white_for_colorless.update_cant_effects();
    let white_colorless_spell_id =
        white_for_colorless.create_object_from_definition(&spell, alice, Zone::Stack);
    white_for_colorless
        .player_mut(alice)
        .expect("Alice exists")
        .mana_pool
        .add(ManaSymbol::White, 1);
    assert!(
        !white_for_colorless.can_pay_mana_cost(
            alice,
            Some(white_colorless_spell_id),
            &colorless_cost,
            0,
        ),
        "Celestial Dawn's white-as-any-color permission should not turn white mana into colorless mana"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_mnemonic_betrayal_temporary_any_color_cast_permission() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mnemonic Betrayal")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Exile all opponents' graveyards. You may cast spells from among those cards this turn, and you may spend mana as though it were mana of any color to cast them. At the beginning of the next end step, if any of those cards remain exiled, return them to their owners' graveyards.\nExile Mnemonic Betrayal.",
        )
        .expect("Mnemonic Betrayal text should parse");

    let rendered = crate::compiled_text::canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    let spell_debug = format!("{:#?}", def.spell_effect);
    assert!(
        rendered.contains("you may cast spells from among those cards this turn")
            && rendered
                .contains("you may spend mana as though it were mana of any color to cast them"),
        "expected temporary cast permission with any-color mana spend text, got {rendered}"
    );
    assert!(
        rendered.contains("if any of those cards remain exiled")
            && rendered.contains("their owners' graveyards"),
        "expected delayed exiled-card return clause, got {rendered}\n{spell_debug}"
    );
    assert!(
        spell_debug.contains("ConditionalEffect")
            && spell_debug.contains(
                "zone: Some(\n                                                                Exile"
            )
            && spell_debug.contains("MoveToZoneEffect"),
        "expected delayed exile-check cleanup in compiled effects, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_then_if_conditional_sentence_is_preserved() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Then If Variant")
        .parse_text(
            "Target creature gets +1/+1 until end of turn. Then if you control a creature with power 4 or greater, draw a card.",
        )
        .expect("then-if conditional sentence should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("if you control a creature with power 4 or greater")
            && rendered.contains("draw a card"),
        "expected then-if conditional to remain in compiled output, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_additional_cost_and_trigger_when_on_same_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Additional Cost Split Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "As an additional cost to cast this spell, sacrifice a creature. When this creature enters, each opponent loses 4 life.",
        )
        .expect("additional-cost line with trailing trigger sentence should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("as an additional cost to cast this spell")
            && rendered.contains("when this creature enters"),
        "expected both additional-cost and trigger clauses, got {rendered}"
    );
    assert!(
        !rendered.contains("whenever as an additional cost"),
        "expected additional-cost clause to stay out of triggered text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_additional_cost_or_chain_renders_inline_or_options() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Additional Cost Choice Variant")
        .parse_text(
            "As an additional cost to cast this spell, sacrifice a creature, discard a card, or pay 4 life. Draw a card.",
        )
        .expect("additional-cost or-chain should parse as a choice");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("as an additional cost to cast this spell")
            && (rendered.contains("sacrifice a creature, discard a card, or pay 4 life")
                || rendered.contains("sacrifice a creature, discard a card, or lose 4 life")),
        "expected additional cost to preserve inline or-options, got {rendered}"
    );
    assert!(
        rendered.contains("sacrifice a creature")
            && rendered.contains("discard a card")
            && (rendered.contains("pay 4 life") || rendered.contains("lose 4 life")),
        "expected all additional-cost options to remain in compiled text, got {rendered}"
    );
    assert!(
        !rendered.contains("sacrifice a creature. discard a card. pay 4 life")
            && !rendered.contains("sacrifice a creature. you discard a card. you lose 4 life"),
        "expected additional costs to remain alternatives, not cumulative mandatory costs, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_distribute_counters_among_any_number_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Invoke Variant")
        .parse_text(
            "Return target permanent card from your graveyard to the battlefield, then distribute four +1/+1 counters among any number of creatures and/or Vehicles target player controls.",
        )
        .expect("distribute-counters clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("return target")
            && rendered.contains("+1/+1")
            && rendered.contains("vehicle"),
        "expected return and distributed counters clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_distribute_counters_one_or_two_targets() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Elven Rite Variant")
        .parse_text("Distribute two +1/+1 counters among one or two target creatures.")
        .expect("one-or-two distributed counters clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("one or two")
            && rendered.contains("+1/+1")
            && rendered.contains("target creatures"),
        "expected one-or-two target distribute wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_distribute_counters_one_two_or_three_targets() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Biogenic Variant")
        .parse_text("Distribute three +1/+1 counters among one, two, or three target creatures.")
        .expect("one-two-or-three distributed counters clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("one, two, or three")
            || rendered.contains("one or two or three")
            || rendered.contains("up to three"),
        "expected plural distributed target count, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn source_double_counter_cards_keep_singular_source_surface() {
    for (name, counter_phrase, source_surface) in [
        ("Ascendant Acolyte", "+1/+1", "this creature"),
        ("Dragonsguard Elite", "+1/+1", "this creature"),
        ("Paradox Zone", "growth", "this enchantment"),
        ("Solarion", "+1/+1", "this creature"),
    ] {
        let lines = compiled_text_lines(&parse_oracle_card_definition(name));
        let rendered = lines.join("\n");
        let lower = rendered.to_ascii_lowercase();
        assert!(
            lower.contains(&format!(
                "double the number of {counter_phrase} counters on {source_surface}"
            )),
            "expected {name} to keep its singular source surface, got {rendered}"
        );
        assert!(
            !lower.contains("on each this"),
            "expected {name} not to widen its source into a filter-wide target, got {rendered}"
        );

        match name {
            "Ascendant Acolyte" => assert!(lower.contains("enters with a +1/+1 counter")),
            "Dragonsguard Elite" => assert!(lower.contains("magecraft")),
            "Paradox Zone" => assert!(
                lower.contains("create a 0/0 green and blue fractal creature token"),
                "counter doubling must not absorb Paradox Zone's token follow-up: {rendered}"
            ),
            "Solarion" => {
                assert!(
                    lines
                        .iter()
                        .any(|line| line.eq_ignore_ascii_case("Sunburst")),
                    "Solarion must retain Sunburst as its own clause: {rendered}"
                );
                assert!(
                    lines.iter().any(|line| {
                        line.to_ascii_lowercase()
                            .contains("double the number of +1/+1 counters")
                    }),
                    "Solarion must retain its separate double-counters ability: {rendered}"
                );
            }
            _ => unreachable!(),
        }
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_distribute_then_double_counters_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Biogenic Upgrade Variant")
        .parse_text(
            "Distribute three +1/+1 counters among one, two, or three target creatures, then double the number of +1/+1 counters on each of those creatures.",
        )
        .expect("distribute-then-double counters clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("one, two, or three")
            || rendered.contains("one or two or three")
            || rendered.contains("up to three"),
        "expected distributed target count to remain plural, got {rendered}"
    );
    assert!(
        rendered.contains("double the number of +1/+1 counters"),
        "expected trailing double-counters clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_deepglow_skate_strict_oracle_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Deepglow Skate")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Fish])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text(
            "When this creature enters, double the number of each kind of counter on any number of target permanents.",
        )
        .expect("Deepglow Skate strict oracle text should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert_eq!(
        rendered,
        "When this creature enters, double the number of each kind of counter on any number of target permanents."
    );

    let ability = def
        .abilities
        .iter()
        .find(|ability| matches!(&ability.kind, AbilityKind::Triggered(_)))
        .expect("Deepglow Skate should have an enters triggered ability");
    let AbilityKind::Triggered(triggered) = &ability.kind else {
        panic!("expected Deepglow Skate triggered ability");
    };
    let double = triggered
        .effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<DoubleCountersEffect>())
        .expect("Deepglow Skate should lower to a double-counters effect");
    assert_eq!(double.counter_type, None);
    assert!(double.target.is_target());
    assert_eq!(
        double.target.count(),
        crate::effect::ChoiceCount::any_number()
    );
    let ChooseSpec::Object(filter) = double.target.base() else {
        panic!(
            "Deepglow Skate should target permanents, got {:?}",
            double.target
        );
    };
    assert_eq!(filter.zone, Some(Zone::Battlefield));
}

#[test]
pub(super) fn aetheric_amplifier_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Aetheric Amplifier");
    let def = parse_oracle_card_definition("Aetheric Amplifier");
    let rendered = unprocessed_compiled_lines(&def).join("\n");

    assert!(
        rendered.contains(
            "{4}, {T}: Choose one. Activate only as a sorcery.\n\
• Double the number of each kind of counter on target permanent.\n\
• Double the number of each kind of counter you have."
        ),
        "expected Aetheric Amplifier compiled text to preserve both counter-doubling modes, got {rendered}"
    );

    let modal = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .find_map(|activated| {
            activated
                .effects
                .flattened_default_effects()
                .iter()
                .find_map(|effect| effect.downcast_ref::<ChooseModeEffect>())
        })
        .expect("Aetheric Amplifier should compile its choose-one activation as a modal effect");
    let modal_activated = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .find(|activated| {
            activated
                .effects
                .flattened_default_effects()
                .iter()
                .any(|effect| effect.downcast_ref::<ChooseModeEffect>().is_some())
        })
        .expect("Aetheric Amplifier should have a modal activated ability");
    assert_eq!(
        modal_activated.timing,
        crate::ability::ActivationTiming::SorcerySpeed,
        "Aetheric Amplifier modal activation must preserve activate-only-as-sorcery timing"
    );
    assert_eq!(modal.modes.len(), 2);

    let doubles = modal
        .modes
        .iter()
        .flat_map(|mode| mode.effects.iter())
        .filter_map(|effect| effect.downcast_ref::<DoubleCountersEffect>())
        .collect::<Vec<_>>();
    assert_eq!(
        doubles.len(),
        2,
        "Aetheric Amplifier should compile both modes as double-counters effects"
    );
    assert!(
        doubles.iter().any(|effect| {
            effect.counter_type.is_none()
                && effect.target.is_target()
                && matches!(
                    effect.target.base(),
                    ChooseSpec::Object(filter) if filter.zone == Some(Zone::Battlefield)
                )
        }),
        "expected one mode to target a permanent"
    );
    assert!(
        doubles.iter().any(|effect| {
            effect.counter_type.is_none()
                && matches!(effect.target.base(), ChooseSpec::SourceController)
        }),
        "expected one mode to double counters the controller has"
    );
}

#[test]
pub(super) fn aetheric_amplifier_modal_activation_is_sorcery_speed() {
    let def = parse_oracle_card_definition("Aetheric Amplifier");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.step = None;
    let amplifier_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.player_mut(alice)
        .expect("alice exists")
        .mana_pool
        .add(ManaSymbol::Colorless, 4);

    let modal_ability_index = game
        .object(amplifier_id)
        .expect("Aetheric Amplifier should exist")
        .abilities
        .iter()
        .position(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated
                .effects
                .flattened_default_effects()
                .iter()
                .any(|effect| effect.downcast_ref::<ChooseModeEffect>().is_some()),
            _ => false,
        })
        .expect("Aetheric Amplifier should have a modal activated ability");

    game.turn.phase = crate::game_state::Phase::FirstMain;
    assert!(
        crate::decision::compute_legal_actions(&game, alice)
            .into_iter()
            .any(|action| matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, ability_index }
                    if source == amplifier_id && ability_index == modal_ability_index
            )),
        "Aetheric Amplifier's modal activation should be legal in its controller's main phase"
    );

    game.turn.phase = crate::game_state::Phase::Combat;
    assert!(
        !crate::decision::compute_legal_actions(&game, alice)
            .into_iter()
            .any(|action| matches!(
                action,
                crate::decision::LegalAction::ActivateAbility { source, ability_index }
                    if source == amplifier_id && ability_index == modal_ability_index
            )),
        "Aetheric Amplifier's modal activation should not be legal outside sorcery speed"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_then_that_player_discards_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Recoil Variant")
        .parse_text(
            "Return target permanent to its owner's hand. Then that player discards a card.",
        )
        .expect("then-that-player discard clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("that player discards a card")
            || rendered.contains("target player discards a card"),
        "expected discard to remain bound to the returned permanent's player, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_comma_then_that_player_discards_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Dinrova Variant")
        .parse_text(
            "Return target permanent to its owner's hand, then that player discards a card.",
        )
        .expect("comma-then-that-player discard clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("that player discards a card")
            || rendered.contains("target player discards a card"),
        "expected discard to remain bound to the returned permanent's player, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_comma_then_exile_that_players_graveyard_from_target_graveyard_card_owner() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Nurgle Variant")
        .parse_text(
            "Put target creature card from an opponent's graveyard onto the battlefield tapped under your control, then exile that player's graveyard.",
        )
        .expect("target graveyard-card owner follow-up should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("onto the battlefield tapped under your control")
            && (rendered_lower.contains("exile that player's graveyard")
                || rendered_lower.contains("exile their graveyard")),
        "expected target graveyard-card owner follow-up text, got {rendered}"
    );

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("OwnerOf(Tagged") || debug.contains("AliasedOwnerOf(Target)"),
        "expected graveyard exile to bind to the targeted card owner's graveyard, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_dinrova_horror_strict_oracle_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Dinrova Horror")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Horror])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "When this creature enters, return target permanent to its owner's hand, then that player discards a card.",
        )
        .expect("Dinrova Horror should parse strictly");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("OwnerOf(Tagged"),
        "expected Dinrova Horror's discard to bind to the returned permanent's owner, got {abilities_debug}"
    );
    assert!(
        !abilities_debug.contains("IteratedPlayer"),
        "Dinrova Horror should not leave an unbound that-player reference, got {abilities_debug}"
    );
    assert!(
        rendered.contains(
            "When this creature enters, return target permanent to its owner's hand, then that player discards a card."
        ),
        "expected Dinrova Horror's return-then-discard clause to render compactly, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_infernal_kirin_strict_oracle_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(74_377), "Infernal Kirin")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Black],
        ]))
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Kirin, Subtype::Spirit])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text(
            "Flying\nWhenever you cast a Spirit or Arcane spell, target player reveals their hand and discards all cards with that spell's mana value.",
        )
        .expect("Infernal Kirin should parse strictly");

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    let debug = format!("{def:#?}");
    assert!(
        rendered.contains("Flying"),
        "expected Flying, got {rendered}"
    );
    assert!(
        rendered.contains(
            "Whenever you cast a Spirit or Arcane spell, target player reveals their hand and discards all cards with that spell's mana value."
        ),
        "expected Infernal Kirin trigger text to preserve the mana-value discard clause, got {rendered}"
    );
    assert!(debug.contains("LookAtHandEffect"), "{debug}");
    assert!(debug.contains("DiscardEffect"), "{debug}");
    assert!(debug.contains("SameManaValueAsTagged"), "{debug}");
    assert!(debug.contains("triggering"), "{debug}");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_comma_then_return_source_to_hand_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cyclopean Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text("{3}, {T}: Tap target creature, then return this artifact to its owner's hand.")
        .expect("comma-then return-source clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("tap target creature"),
        "expected tap target creature effect, got {rendered}"
    );
    assert!(
        rendered.contains("return this artifact to its owner's hand"),
        "expected return-source clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_then_exile_that_players_graveyard_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Go Blank Variant")
        .parse_text("Target player discards two cards. Then exile that player's graveyard.")
        .expect("then-that-player graveyard exile clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("that player's graveyard")
            || rendered.contains("target player's graveyard"),
        "expected graveyard exile to remain tied to the targeted player, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_put_counter_sequence_with_and_chain() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Trygon Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature attacks, put a +1/+1 counter on it and a +1/+1 counter on up to one other target attacking creature. That creature can't be blocked this turn.",
        )
        .expect("put-counter and-chain should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("up to one other target attacking creature")
            || rendered.contains("another attacking creature"),
        "expected second counter target to remain in trigger, got {rendered}"
    );
    assert!(
        rendered.contains("can't be blocked") || rendered.contains("cant be blocked"),
        "expected trailing block restriction to remain, got {rendered}"
    );
}

fn nested_cant_effect(effect: &Effect) -> Option<&crate::effects::CantEffect> {
    if let Some(cant) = effect.downcast_ref::<crate::effects::CantEffect>() {
        return Some(cant);
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        return sequence.effects.iter().find_map(nested_cant_effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return nested_cant_effect(&tagged.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return nested_cant_effect(&tagged.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return nested_cant_effect(&with_id.effect);
    }
    None
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_cephalid_inkshrouder_keeps_self_buff_and_unblockable_clause_together() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cephalid Inkshrouder")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Octopus])
        .power_toughness(PowerToughness::fixed(2, 1))
        .parse_text("Discard a card: This creature gains shroud until end of turn and can't be blocked this turn.")
        .expect("Cephalid Inkshrouder text should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("discard a card")
            && (rendered.contains("this creature gains shroud until end of turn")
                || rendered.contains("this creature gains shroud and can't be blocked"))
            && (rendered.contains("can't be blocked this turn")
                || rendered.contains("cant be blocked this turn")),
        "expected Cephalid Inkshrouder to keep both clauses together, got {rendered}"
    );
    assert!(
        !rendered.contains("choose it") && !rendered.contains("target permanent"),
        "expected no stray target wording in Cephalid Inkshrouder rendering, got {rendered}"
    );

    let abilities_debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        abilities_debug.contains("shroud") && abilities_debug.contains("beblocked"),
        "expected shroud and unblockable effects in the compiled definition, got {abilities_debug}"
    );

    let cant_effect = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            crate::ability::AbilityKind::Activated(activated) => activated
                .effects
                .segments
                .iter()
                .flat_map(|segment| segment.default_effects.iter())
                .find_map(nested_cant_effect),
            _ => None,
        })
        .expect("expected Cephalid Inkshrouder to compile a cant effect");
    match &cant_effect.restriction {
        crate::effect::Restriction::BeBlocked(filter) => {
            assert!(
                filter.source
                    || filter.tagged_constraints.iter().any(|constraint| {
                        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    }),
                "expected Cephalid Inkshrouder's unblockable restriction to stay bound to itself, got {filter:?}"
            );
        }
        other => panic!("expected Cephalid Inkshrouder be-blocked restriction, got {other:?}"),
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_break_through_the_line_keeps_targeted_unblockable_clause_tied_to_target() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Break Through the Line")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "{R}: Target creature with power 2 or less gains haste until end of turn and can't be blocked this turn.",
        )
        .expect("Break Through the Line text should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("target creature with power 2 or less gains haste until end of turn")
            && rendered.contains("and can't be blocked this turn")
            && !rendered.contains("choose it")
            && !rendered.contains("target permanent"),
        "expected Break Through the Line to render as a single targeted clause, got {rendered}"
    );

    let cant_effect = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated
                .effects
                .segments
                .iter()
                .flat_map(|segment| segment.default_effects.iter())
                .find_map(nested_cant_effect),
            _ => None,
        })
        .expect("expected Break Through the Line to compile a cant effect");
    match &cant_effect.restriction {
        crate::effect::Restriction::BeBlocked(filter) => {
            assert!(
                !filter.source,
                "expected Break Through the Line's unblockable restriction to stay tied to the target creature, got {filter:?}"
            );
            assert!(
                (filter.card_types == vec![CardType::Creature]
                    && filter.power == Some(crate::filter::Comparison::LessThanOrEqual(2)))
                    || filter.tagged_constraints.iter().any(|constraint| {
                        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    }),
                "expected Break Through the Line's unblockable restriction to keep the original target binding, got {filter:?}"
            );
        }
        other => panic!("expected Break Through the Line be-blocked restriction, got {other:?}"),
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn slippery_scoundrel_keeps_hexproof_and_unblockable_under_citys_blessing() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Slippery Scoundrel Variant")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Pirate])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "Ascend (If you control ten or more permanents, you get the city's blessing for the rest of the game.)\nAs long as you have the city's blessing, this creature has hexproof and can't be blocked.",
        )
        .expect("Slippery Scoundrel should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("city's blessing")
            && rendered.contains("hexproof")
            && (rendered.contains("can't be blocked") || rendered.contains("cant be blocked")),
        "expected Slippery Scoundrel to keep both city-blessing grants, got {rendered}"
    );

    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        debug.contains("playerhascitysblessing")
            && debug.contains("hexproof")
            && debug.contains("unblockable"),
        "expected Slippery Scoundrel definition to include both conditional grants, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn kumenas_awakening_renders_ascend_and_citys_blessing_replacement() {
    let rendered = unprocessed_compiled_lines(&parse_oracle_card_definition("Kumena's Awakening"));

    assert_eq!(
        rendered,
        vec![
            "Ascend",
            "At the beginning of your upkeep, each player draws a card. If you have the city's blessing, instead only you draw a card.",
        ]
    );
}

#[test]
pub(super) fn tidal_influence_renders_state_trigger_without_duplicate_counter_condition() {
    let def = parse_oracle_card_definition("Tidal Influence");
    let rendered = crate::compiled_text::compiled_text_lines(&def).join("\n");

    assert!(
        rendered.contains(
            "Whenever there are four or more tide counters on this enchantment, remove all tide counters from it."
        ),
        "expected Tidal Influence to render its state trigger cleanly, got {rendered}"
    );
    assert!(
        !rendered.contains("if this enchantment has 4 or more tide counters")
            && !rendered.contains("the number of tide counters"),
        "expected Tidal Influence not to duplicate its trigger condition or count phrase, got {rendered}"
    );
}

#[test]
pub(super) fn source_counter_thresholds_keep_existential_oracle_surface() {
    for name in [
        "Budoka Pupil",
        "Callow Jushi",
        "Cunning Bandit",
        "Faithful Squire",
        "Hired Muscle",
    ] {
        let rendered =
            crate::compiled_text::compiled_text_lines(&parse_oracle_card_definition(name))
                .join("\n");
        assert!(
            rendered
                .contains("if there are two or more ki counters on this creature, you may flip it"),
            "expected {name} to preserve its existential ki-counter condition and lowercase flip, got {rendered}"
        );
    }

    let decree = crate::compiled_text::compiled_text_lines(&parse_oracle_card_definition(
        "Decree of Silence",
    ))
    .join("\n");
    assert!(
        decree.to_ascii_lowercase().contains(
            "if there are three or more depletion counters on this enchantment, sacrifice it"
        ),
        "expected Decree of Silence to preserve its conditional follow-up, got {decree}"
    );

    let foreboding = crate::compiled_text::compiled_text_lines(&parse_oracle_card_definition(
        "Foreboding Statue",
    ))
    .join("\n");
    assert!(
        foreboding.contains("if there are three or more omen counters on this creature, untap it"),
        "expected Foreboding Statue to preserve its parsed counter-threshold follow-up, got {foreboding}"
    );

    let grasping = crate::compiled_text::compiled_text_lines(&parse_oracle_card_definition(
        "Grasping Shadows",
    ))
    .join("\n");
    assert!(
        grasping.contains("there are three or more dread counters on it, transform it"),
        "expected Grasping Shadows to preserve its source pronoun and transform follow-up, got {grasping}"
    );

    let quest = crate::compiled_text::compiled_text_lines(&parse_oracle_card_definition(
        "Quest for Ula's Temple",
    ))
    .join("\n");
    assert!(
        quest.contains("if there are three or more quest counters on this enchantment"),
        "expected Quest for Ula's Temple to preserve its existential counter condition, got {quest}"
    );
}

#[test]
pub(super) fn remove_all_named_counters_from_target_renders_generically() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Counter Cleaner")
        .card_types(vec![CardType::Artifact])
        .parse_text("{T}: Remove all charge counters from target artifact.")
        .expect("remove-all-counters target activation should parse");
    let rendered = crate::compiled_text::compiled_text_lines(&def).join("\n");

    assert!(
        rendered.contains("Remove all charge counters from target artifact"),
        "expected generic remove-all counter wording, got {rendered}"
    );
    assert!(
        !rendered.contains("the number of charge counters"),
        "expected target remove-all counter wording not to use count phrase, got {rendered}"
    );
}

#[test]
pub(super) fn sarulf_realm_eater_strict_parser_text_and_structure_regression() {
    let def = parse_oracle_card_definition("Sarulf, Realm Eater");
    let rendered = crate::compiled_text::compiled_text_lines(&def).join("\n");
    let ability_debug = format!("{:#?}", def.abilities);
    let upkeep_text = "At the beginning of your upkeep, if Sarulf has one or more +1/+1 counters on it, you may remove all +1/+1 counters from it. If you do, exile each other nonland permanent with mana value less than or equal to the number of counters removed this way.";

    assert!(
        rendered.contains("Whenever a permanent an opponent controls is put into a graveyard from the battlefield, put a +1/+1 counter on Sarulf."),
        "expected Sarulf death trigger text, got {rendered}"
    );
    assert!(
        rendered.contains(upkeep_text),
        "expected Sarulf upkeep all-of-them counter removal and removed-this-way exile text, got {rendered}"
    );
    assert!(
        ability_debug.contains("BeginningOfUpkeepTrigger")
            && ability_debug.contains("SourceHasCounterAtLeast")
            && ability_debug.contains("RemoveCountersEffect")
            && ability_debug.contains("PlusOnePlusOne")
            && ability_debug.contains("IfEffect")
            && ability_debug.contains("ExileEffect")
            && ability_debug.contains("EffectValue"),
        "expected Sarulf to structurally remove +1/+1 counters and exile by the removed count, got {ability_debug}"
    );
}

#[test]
pub(super) fn blitz_leech_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Blitz Leech");

    let def = parse_oracle_card_definition("Blitz Leech");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let ability_debug = format!("{:#?}", def.abilities);

    assert_eq!(def.name(), "Blitz Leech");
    assert!(
        rendered.contains("Flash"),
        "Blitz Leech should keep flash in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains(
            "When this creature enters, target creature an opponent controls gets -2/-2 until end of turn. Remove all counters from that creature"
        ),
        "Blitz Leech should render the target-relative all-counters clause, got {rendered}"
    );
    assert!(
        ability_debug.contains("ModifyPowerToughness")
            && ability_debug.contains("RemoveUpToAnyCountersEffect")
            && ability_debug.contains("CountersOn"),
        "Blitz Leech should structurally model the -2/-2 and all-counters effects, got {ability_debug}"
    );
}

#[test]
pub(super) fn mindstorm_crown_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Mindstorm Crown");

    let def = parse_oracle_card_definition("Mindstorm Crown");
    let rendered = unprocessed_compiled_lines(&def);
    let expected = concat!(
        "At the beginning of your upkeep, draw a card if you had no cards in hand at the beginning of this turn. ",
        "If you had a card in hand, this artifact deals 1 damage to you."
    );
    let ability_debug = format!("{:#?}", def.abilities);

    assert_eq!(def.name(), "Mindstorm Crown");
    assert_eq!(rendered, vec![expected.to_string()]);
    assert!(
        ability_debug.contains("PlayerCardsInHandAtTurnStartOrFewer")
            && ability_debug.contains("PlayerCardsInHandAtTurnStartOrMore")
            && ability_debug.contains("DrawCardsEffect")
            && ability_debug.contains("DealDamageEffect"),
        "Mindstorm Crown should structurally model both turn-start hand branches, got {ability_debug}"
    );
}

pub(super) fn mindstorm_crown_triggered_ability(
    def: &CardDefinition,
) -> &crate::ability::TriggeredAbility {
    def.abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Mindstorm Crown should have an upkeep triggered ability")
}

pub(super) fn create_mindstorm_crown_test_card(
    game: &mut crate::game_state::GameState,
    id: u32,
    owner: PlayerId,
    zone: Zone,
) -> ObjectId {
    game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::from_raw(id), &format!("Mindstorm Test Card {id}"))
            .card_types(vec![CardType::Creature])
            .build(),
        owner,
        zone,
    )
}

#[test]
pub(super) fn mindstorm_crown_draws_when_hand_was_empty_at_turn_start() {
    let def = parse_oracle_card_definition("Mindstorm Crown");
    let triggered = mindstorm_crown_triggered_ability(&def);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let crown_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    create_mindstorm_crown_test_card(&mut game, 91_001, alice, Zone::Library);

    game.record_turn_start_hand_sizes();
    create_mindstorm_crown_test_card(&mut game, 91_002, alice, Zone::Hand);

    let mut ctx = crate::effects::ExecutionContext::new_default(crown_id, alice);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        crown_id,
        &triggered.effects,
        None,
        &[],
    )
    .expect("Mindstorm Crown trigger should resolve");

    assert_eq!(
        game.life_total(alice),
        20,
        "empty turn-start hand should not deal damage"
    );
    assert_eq!(
        game.player(alice).expect("Alice exists").hand.len(),
        2,
        "empty turn-start hand should draw even if Alice had a card by resolution"
    );
}

#[test]
pub(super) fn mindstorm_crown_deals_damage_when_hand_had_card_at_turn_start() {
    let def = parse_oracle_card_definition("Mindstorm Crown");
    let triggered = mindstorm_crown_triggered_ability(&def);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let crown_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    create_mindstorm_crown_test_card(&mut game, 91_101, alice, Zone::Library);
    let hand_card = create_mindstorm_crown_test_card(&mut game, 91_102, alice, Zone::Hand);

    game.record_turn_start_hand_sizes();
    game.move_object(
        hand_card,
        Zone::Graveyard,
        crate::events::cause::EventCause::from_game_rule(),
    )
    .expect("setup should move hand card out of hand");

    let mut ctx = crate::effects::ExecutionContext::new_default(crown_id, alice);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        crown_id,
        &triggered.effects,
        None,
        &[],
    )
    .expect("Mindstorm Crown trigger should resolve");

    assert_eq!(
        game.life_total(alice),
        19,
        "nonempty turn-start hand should deal 1 damage"
    );
    assert_eq!(
        game.player(alice).expect("Alice exists").hand.len(),
        0,
        "nonempty turn-start hand should not draw even if Alice's hand is empty by resolution"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn fear_of_immobility_keeps_tap_target_and_conditional_stun_counter() {
    let def = parse_oracle_card_definition("Fear of Immobility");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let ability_debug = format!("{:#?}", def.abilities);

    assert!(
        rendered.contains("tap up to one target creature")
            && rendered.contains("stun counter")
            && rendered.contains("opponent controls"),
        "expected Fear of Immobility to render tap target plus opponent-control stun condition, got {rendered}"
    );
    assert!(
        ability_debug.contains("TapEffect")
            && ability_debug.contains("ConditionalEffect")
            && ability_debug.contains("TaggedObjectMatches")
            && ability_debug.contains("PutCountersEffect")
            && ability_debug.contains("tapped_0"),
        "expected target tap followed by conditional stun counter, got {ability_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn regal_sliver_keeps_granted_monarch_otherwise_branch() {
    let def = parse_oracle_card_definition("Regal Sliver");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let debug = format!("{:#?}", def);

    assert!(
        rendered.contains("Sliver creatures you control have")
            && rendered.contains("if you're the monarch")
            && rendered.contains("Otherwise, you become the monarch"),
        "expected Regal Sliver to render the full granted monarch trigger, got {rendered}"
    );
    assert!(
        debug.contains("AddAbilityGeneric")
            && debug.contains("ConditionalEffect")
            && debug.contains("PlayerIsMonarch")
            && debug.contains("BecomeMonarchEffect"),
        "expected granted trigger to lower monarch if/otherwise branch, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn nissa_resurgent_animist_keeps_second_resolution_consult() {
    let def = parse_oracle_card_definition("Nissa, Resurgent Animist");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let debug = format!("{:#?}", def);

    assert!(
        rendered.contains("add one mana of any color")
            && rendered.contains(
                "Then if this is the second time this ability has resolved this turn"
            )
            && rendered.contains(
                "reveal cards from the top of your library until you reveal an Elf or Elemental card"
            )
            && rendered
                .contains("Put that card into your hand and the rest on the bottom of your library"),
        "expected Nissa to render mana plus conditional reveal-until follow-up, got {rendered}"
    );
    assert!(
        debug.contains("AddManaOfAnyColorEffect")
            && debug.contains("ThisAbilityResolvedThisTurnExactly")
            && debug.contains("2,")
            && debug.contains("ConsultTopOfLibraryEffect")
            && debug.contains("MoveToZoneEffect")
            && debug.contains("zone: Hand")
            && debug.contains("PutTaggedRemainderOnLibraryBottomEffect"),
        "expected Nissa to lower second-resolution consult to hand and bottom remainder, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn living_plane_oracle_like_text_merges_animation_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Living Plane Variant")
        .card_types(vec![CardType::Enchantment])
        .supertypes(vec![Supertype::World])
        .parse_text("All lands are 1/1 creatures that are still lands.")
        .expect("Living Plane should parse");

    let lines = unprocessed_compiled_lines(&def);
    assert_eq!(
        lines,
        vec!["All lands are 1/1 creatures that are still lands.".to_string()],
        "expected Living Plane animation text to merge cleanly, got {lines:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn thelonite_druid_forest_animation_renders_still_lands() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Thelonite Druid")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Cleric, Subtype::Druid])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "{1}{G}, {T}, Sacrifice a creature: Forests you control become 2/3 creatures until end of turn. They're still lands.",
        )
        .expect("Thelonite Druid should parse");

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Thelonite Druid should have an activated ability");
    let score_path =
        crate::compiled_text::compile_effect_list(&activated.effects.segments[0].default_effects);
    assert!(
        (score_path.contains("All Forests you control become 2/3 creatures until end of turn")
            || score_path.contains(
                "Forests you control become creatures with base power and toughness 2/3 until end of turn"
            ))
            && score_path.contains("They're still lands"),
        "expected Forest animation to preserve land-ness in effect rendering, got {score_path}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        (rendered.contains("Forests you control become 2/3 creatures until end of turn")
            || rendered.contains(
                "Forests you control become creatures with base power and toughness 2/3 until end of turn"
            ))
            && rendered.contains("They're still lands"),
        "expected Thelonite Druid compiled text to render Forests as still lands, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn fendeep_summoner_land_animation_keeps_subtypes_with_addition_tail() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Fendeep Summoner")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Black],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Treefolk, Subtype::Shaman])
        .power_toughness(PowerToughness::fixed(3, 5))
        .parse_text(
            "{T}: Up to two target Swamps each become 3/5 Treefolk Warrior creatures in addition to their other types until end of turn.",
        )
        .expect("Fendeep Summoner should parse");

    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Fendeep Summoner should have an activated ability");
    let animate = activated.effects.segments[0].default_effects[0]
        .downcast_ref::<crate::effects::ApplyContinuousEffect>()
        .expect("expected land-animation continuous effect");

    assert_eq!(
        animate
            .target_spec
            .as_ref()
            .map(crate::target::ChooseSpec::count),
        Some(ChoiceCount::up_to(2)),
        "Fendeep Summoner should target up to two Swamps"
    );
    let target_spec = animate
        .target_spec
        .as_ref()
        .map(crate::target::ChooseSpec::base);
    assert!(
        matches!(target_spec, Some(crate::target::ChooseSpec::Object(filter))
            if filter.subtypes == vec![Subtype::Swamp]),
        "Fendeep Summoner should target Swamps, got {:?}",
        animate.target_spec
    );
    assert!(
        matches!(
            animate.modification.as_ref(),
            Some(crate::continuous::Modification::AddCardTypes(card_types))
                if card_types.contains(&CardType::Creature)
        ),
        "expected animation to add Creature type, got {:?}",
        animate.modification
    );
    assert!(
        animate
            .additional_modifications
            .iter()
            .any(|modification| matches!(
                modification,
                crate::continuous::Modification::AddSubtypes(subtypes)
                    if subtypes.contains(&Subtype::Treefolk)
                        && subtypes.contains(&Subtype::Warrior)
            )),
        "expected animation to add Treefolk and Warrior subtypes, got {:?}",
        animate.additional_modifications
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        (rendered_lower.contains(
            "up to two target swamps become 3/5 treefolk warrior creatures in addition to their other types until end of turn"
        ) || rendered_lower.contains(
            "up to two target swamps become treefolk warrior creatures with base power and toughness 3/5 in addition to their other types until end of turn"
        )) && !rendered.contains("They're still lands"),
        "expected Fendeep Summoner compiled text to render Treefolk Warrior animation as type addition, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn forest_animation_keeps_color_subtypes_and_source_duration() {
    let awakener = CardDefinitionBuilder::new(CardId::new(), "Awakener Druid")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Druid])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "When this creature enters, target Forest becomes a 4/5 green Treefolk creature for as long as this creature remains on the battlefield. It's still a land.",
        )
        .expect("Awakener Druid animation should parse");
    let awakener_rendered = unprocessed_compiled_lines(&awakener)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        (awakener_rendered.contains("4/5 green treefolk creature")
            || awakener_rendered
                .contains("green treefolk creature with base power and toughness 4/5"))
            && awakener_rendered.contains("still a land"),
        "expected Awakener Druid animation to keep color/subtype and land tail, got {awakener_rendered}"
    );

    let woodwraith = CardDefinitionBuilder::new(CardId::new(), "Woodwraith Corrupter")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Black],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elemental, Subtype::Horror])
        .power_toughness(PowerToughness::fixed(3, 6))
        .parse_text(
            "{1}{B}{G}, {T}: Target Forest becomes a 4/4 black and green Elemental Horror creature. It's still a land.",
        )
        .expect("Woodwraith Corrupter animation should parse");
    let woodwraith_rendered = unprocessed_compiled_lines(&woodwraith)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        (woodwraith_rendered.contains("4/4 black and green elemental horror creature")
            || woodwraith_rendered.contains(
                "black and green elemental horror creature with base power and toughness 4/4"
            ))
            && woodwraith_rendered.contains("still a land"),
        "expected Woodwraith animation to keep color/subtypes and land tail, got {woodwraith_rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn swordsworn_cavalier_keeps_entered_this_turn_knight_condition() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Swordsworn Cavalier Variant")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Knight])
        .power_toughness(PowerToughness::fixed(3, 1))
        .parse_text(
            "This creature has first strike as long as another Knight entered the battlefield under your control this turn.",
        )
        .expect("Swordsworn Cavalier should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("first strike")
            && rendered
                .contains("another knight entered the battlefield under your control this turn"),
        "expected Swordsworn Cavalier to keep its entered-this-turn condition, got {rendered}"
    );

    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        debug.contains("countcomparison")
            && debug.contains("entered_battlefield_this_turn: true")
            && debug.contains("entered_battlefield_controller: some(")
            && debug.contains("other: true")
            && debug.contains("subtypes: [")
            && debug.contains("knight"),
        "expected Swordsworn Cavalier to keep the conditional knight-entered filter, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn drownyard_behemoth_parses_hexproof_condition_for_entered_this_turn() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Drownyard Behemoth")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Eldrazi, Subtype::Crab])
        .power_toughness(PowerToughness::fixed(5, 7))
        .parse_text(
            "Flash (You may cast this spell any time you could cast an instant.)\nEmerge {7}{U} (You may cast this spell by sacrificing a creature and paying the emerge cost reduced by that creature's mana value.)\nThis creature has hexproof as long as it entered this turn.",
        )
        .expect("Drownyard Behemoth should parse strictly");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("hexproof") && rendered.contains("entered this turn"),
        "expected Drownyard Behemoth entered-this-turn hexproof wording, got {rendered}"
    );

    let abilities_debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        abilities_debug.contains("condition: some(")
            && abilities_debug.contains("hexproof")
            && abilities_debug.contains("entered_battlefield_this_turn: true"),
        "expected Drownyard Behemoth to compile conditional entered-this-turn hexproof, got {abilities_debug}"
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let behemoth_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let behemoth_snapshot = crate::snapshot::ObjectSnapshot::from_object(
        game.object(behemoth_id)
            .expect("Drownyard Behemoth should exist on the battlefield"),
        &game,
    );
    let entry_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::with_cause(
            behemoth_id,
            Zone::Hand,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            Some(behemoth_snapshot),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    game.record_turn_history_event(&entry_event);

    let mut source_filter = ObjectFilter::source();
    source_filter.entered_battlefield_this_turn = true;
    let entered_this_turn_condition = crate::ConditionExpr::CountComparison {
        count: ironsmith_core::AnthemCountExpression::MatchingFilter(source_filter),
        comparison: crate::effect::Comparison::GreaterThanOrEqual(1),
        display: Some("it entered this turn".to_string()),
    };
    let ctx = crate::effects::ExecutionContext::new_default(behemoth_id, alice);

    assert!(
        crate::condition_eval::evaluate_condition_resolution(
            &game,
            &entered_this_turn_condition,
            &ctx
        )
        .expect("entered-this-turn condition should evaluate"),
        "Drownyard Behemoth should have hexproof on the turn it entered"
    );

    game.turn_store.turn_history.clear_for_new_turn();
    assert!(
        !crate::condition_eval::evaluate_condition_resolution(
            &game,
            &entered_this_turn_condition,
            &ctx,
        )
        .expect("entered-this-turn condition should evaluate after turn change"),
        "Drownyard Behemoth should lose conditional hexproof on a later turn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn omenport_vigilante_keeps_committed_crime_condition() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Omenport Vigilante Variant")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Mercenary])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "This creature has double strike as long as you've committed a crime this turn.",
        )
        .expect("Omenport Vigilante should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("double strike") && rendered.contains("committed a crime this turn"),
        "expected Omenport Vigilante to keep its crime condition, got {rendered}"
    );

    let debug = format!("{def:#?}").to_ascii_lowercase();
    assert!(
        debug.contains("playercommittedcrimethisturn") && debug.contains("doublestrike"),
        "expected Omenport Vigilante definition to include committed-crime condition, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn retained_land_animation_cluster_keeps_oracle_surface_contracts() {
    let compiled = |name| {
        unprocessed_compiled_lines(&parse_oracle_card_definition(name))
            .join(" ")
            .to_ascii_lowercase()
    };

    for (name, land) in [
        ("Genju of the Fens", "swamp"),
        ("Genju of the Fields", "plains"),
    ] {
        let rendered = compiled(name);
        assert!(
            rendered.contains(&format!("until end of turn, enchanted {land} becomes"))
                && rendered.contains("it's still a land")
                && rendered.contains('"')
                && !rendered.contains(&format!("all enchanted {land}")),
            "expected {name} to keep its singular enchanted-land animation and quoted ability, got {rendered}"
        );
    }

    for name in [
        "Lavaclaw Reaches",
        "Raging Ravine",
        "Restless Spire",
        "Wandering Fumarole",
    ] {
        let rendered = compiled(name);
        assert!(
            rendered.contains("until end of turn, this land becomes")
                && rendered.contains("it's still a land")
                && rendered.contains('"')
                && !rendered.contains("where x is x"),
            "expected {name} to keep one outer duration, a quoted granted ability, and its land type, got {rendered}"
        );
    }

    let great_hall = compiled("Great Hall of the Biblioplex");
    assert!(
        great_hall.contains("if this land isn't a creature, it becomes")
            && great_hall.contains("\"whenever you cast an instant or sorcery spell")
            && great_hall.contains("this creature gets +1/+0 until end of turn.\"")
            && great_hall.contains("it's still a land")
            && great_hall.matches("until end of turn").count() == 1,
        "expected Great Hall to keep the negated land condition and only the granted trigger's duration, got {great_hall}"
    );

    for name in [
        "Nissa, Who Shakes the World",
        "Tendril of the Mycotyrant",
        "Wakeroot Elemental",
    ] {
        let rendered = compiled(name);
        assert!(
            rendered.contains("it's still a land")
                && !rendered.contains("in addition to its other types"),
            "expected {name}'s standalone follow-up to preserve the still-a-land surface, got {rendered}"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn moon_girl_compound_possessive_is_not_a_subtype_set() {
    let rendered = unprocessed_compiled_lines(&parse_oracle_card_definition(
        "Moon Girl and Devil Dinosaur",
    ))
    .join(" ");

    assert!(
        rendered.contains(
            "until end of turn, Moon Girl and Devil Dinosaur's base power and toughness become 6/6 and they gain trample"
        ),
        "expected Moon Girl's compound named source to remain one possessive subject, got {rendered}"
    );
    assert!(
        !rendered
            .to_ascii_lowercase()
            .contains("each devil or dinosaur")
            && !rendered
                .to_ascii_lowercase()
                .contains("devils or dinosaurs gain"),
        "Moon Girl must not widen its named source into a subtype set: {rendered}"
    );
}
