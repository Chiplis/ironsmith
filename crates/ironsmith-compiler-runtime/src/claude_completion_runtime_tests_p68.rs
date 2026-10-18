//! Claude package 68 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/claude2/BRIEF.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

fn p68_compile(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|error| panic!("{name}: {error:?}"))
}

fn p68_text(definition: &ironsmith::cards::CardDefinition) -> String {
    ironsmith_text::compiled_text_lines(definition).join("\n")
}

#[test]
fn p68_opponent_split_piles_render_past_the_group_into_piles_record() {
    for (name, text) in [
        ("Sphinx of Uthuun",
         "Mana cost: {5}{U}{U}\nType: Creature — Sphinx\nPower/Toughness: 5/6\nFlying\nWhen this creature enters, reveal the top five cards of your library. An opponent separates those cards into two piles. Put one pile into your hand and the other into your graveyard."),
        ("Unesh, Criosphinx Sovereign",
         "Mana cost: {4}{U}{U}\nType: Legendary Creature — Sphinx\nPower/Toughness: 4/4\nFlying\nSphinx spells you cast cost {2} less to cast.\nWhenever Unesh or another Sphinx you control enters, reveal the top four cards of your library. An opponent separates those cards into two piles. Put one pile into your hand and the other into your graveyard."),
    ] {
        let definition = p68_compile(name, text);
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("GroupIntoPiles"), "{name}: pile record kept: {debug}");
        let surface = p68_text(&definition);
        assert!(!surface.contains("tagged"), "{name}: {surface}");
        assert!(
            surface.contains("An opponent separates those cards into two piles. Put one pile into your hand and the other into your graveyard"),
            "{name}: {surface}"
        );
    }
}

#[test]
fn p68_rendered_internal_references_use_their_authored_wording() {
    let study = p68_compile("Stinging Study",
        "Mana cost: {4}{B}\nType: Instant\nYou draw X cards and you lose X life, where X is the mana value of a commander you own on the battlefield or in the command zone.");
    let surface = p68_text(&study);
    assert!(!surface.contains("tagged"), "{surface}");
    assert!(surface.contains("the mana value of a commander you own on the battlefield or in the command zone"), "{surface}");

    let meeting = p68_compile("Meeting of the Five",
        "Mana cost: {3}{W}{U}{B}{R}{G}\nType: Sorcery\nExile the top ten cards of your library. You may cast spells with exactly three colors from among them this turn. Add {W}{W}{U}{U}{B}{B}{R}{R}{G}{G}. Spend this mana only to cast spells with exactly three colors.");
    let surface = p68_text(&meeting);
    assert!(!surface.contains("tagged"), "{surface}");
    assert!(surface.contains("from among them this turn"), "{surface}");
    assert!(surface.contains("exactly three colors"), "{surface}");

    let vessel = p68_compile("Memory Vessel",
        "Mana cost: {3}{R}{R}\nType: Artifact\n{T}, Exile this artifact: Each player exiles the top seven cards of their library. Until your next turn, players may play cards they exiled this way, and they can't play cards from their hand. Activate only as a sorcery.");
    let surface = p68_text(&vessel);
    assert!(!surface.contains("tagged"), "{surface}");
    assert!(surface.contains("the cards they exiled this way"), "{surface}");
}

#[test]
fn p68_keeper_of_the_dead_names_the_as_you_activate_opponent_and_its_creature() {
    let definition = p68_compile("Keeper of the Dead",
        "Mana cost: {B}{B}\nType: Creature — Human Wizard\nPower/Toughness: 1/2\n{B}, {T}: Choose target opponent who has at least two fewer creature cards in their graveyard than you do as you activate this ability. Destroy target nonblack creature that player controls.");
    let surface = p68_text(&definition);
    assert!(surface.contains("target opponent who has at least two fewer creature cards in their graveyard than you do as you activate this ability"), "{surface}");
    assert!(surface.contains("that player controls"), "{surface}");
}

#[test]
fn p68_kotose_grants_one_lasting_play_with_any_color_mana() {
    let definition = p68_compile("Kotose, the Silent Spider",
        "Mana cost: {3}{U}{B}\nType: Legendary Creature — Human Ninja\nPower/Toughness: 4/4\nWhen Kotose enters, exile target card other than a basic land card from an opponent's graveyard. Search that player's graveyard, hand, and library for any number of cards with the same name as that card and exile them. Then that player shuffles. For as long as you control Kotose, you may play one of the exiled cards, and you may spend mana as though it were mana of any color to cast it.");
    let debug = format!("{:?}", definition.abilities);
    let grant = debug.find("GrantPlayTaggedEffect").expect("lasting play permission");
    let tail = &debug[grant..];
    assert!(tail.contains("ForAsLongAsYouControlSource"), "{tail}");
    assert!(tail.contains("AnyColor"), "{tail}");
    assert!(tail.contains("max_plays: Some(1)"), "{tail}");
    assert!(!debug.contains("CastTaggedEffect"), "no immediate play: {debug}");
    assert!(p68_text(&definition).contains("as though"), "{}", p68_text(&definition));
}

#[test]
fn p68_restriction_and_trigger_surfaces_keep_their_conditions() {
    let lurker = p68_compile("Lurker",
        "Mana cost: {2}{G}\nType: Creature — Beast\nPower/Toughness: 2/3\nThis creature can't be the target of spells unless it attacked or blocked this turn.");
    let surface = p68_text(&lurker);
    assert!(surface.contains("unless it attacked or blocked this turn"), "{surface}");

    let reyhan = p68_compile("Reyhan, Last of the Abzan",
        "Mana cost: {1}{B}{G}\nType: Legendary Creature — Human Warrior\nPower/Toughness: 0/0\nReyhan enters with three +1/+1 counters on it.\nWhenever a creature you control dies or is put into the command zone, if it had one or more +1/+1 counters on it, you may put that many +1/+1 counters on target creature.\nPartner");
    let surface = p68_text(&reyhan);
    assert!(surface.contains("put into the command zone"), "{surface}");

    let river = p68_compile("Raging River",
        "Mana cost: {R}{R}\nType: Enchantment\nWhenever one or more creatures you control attack, each defending player divides all creatures without flying they control into a \"left\" pile and a \"right\" pile. Then, for each attacking creature you control, choose \"left\" or \"right.\" That creature can't be blocked this combat except by creatures with flying and creatures in a pile with the chosen label.");
    let surface = p68_text(&river);
    assert!(surface.contains("can't be blocked this combat except by creatures with flying"), "{surface}");
}

#[test]
fn p68_minamo_and_pin_collection_render_their_specific_selections() {
    let meddling = p68_compile("Minamo's Meddling",
        "Mana cost: {2}{U}{U}\nType: Instant\nCounter target spell. That spell's controller reveals their hand, then discards each card with the same name as a card spliced onto that spell.");
    let surface = p68_text(&meddling);
    assert!(surface.contains("same name as a card spliced onto that spell"), "{surface}");

    let pins = p68_compile("Pin Collection",
        "Mana cost: {X}{W}\nType: Artifact — Equipment\nWhen this Equipment enters, you may put an ability sticker with ticket cost X or less on it without paying that sticker's ticket cost.\nEquipped creature gets +1/+1 and has all abilities of ability stickers on this Equipment.\nEquip {2}");
    let surface = p68_text(&pins);
    assert!(surface.contains("with ticket cost X or less"), "{surface}");
    assert!(surface.contains("without paying that sticker's ticket cost"), "{surface}");
}

#[test]
fn p68_rakdos_augermage_second_reveal_belongs_to_the_choosing_opponent() {
    let definition = p68_compile("Rakdos Augermage",
        "Mana cost: {B}{B}{R}\nType: Creature — Human Wizard\nPower/Toughness: 3/2\nFirst strike\n{T}: Reveal your hand and discard a card of target opponent's choice. Then that player reveals their hand and discards a card of your choice. Activate only as a sorcery.");
    let debug = format!("{:?}", definition.abilities);
    assert_eq!(debug.matches("LookAtHandEffect { target: Player(You)").count(), 1,
        "only the first reveal is yours: {debug}");
    let surface = p68_text(&definition);
    assert!(surface.contains("reveals their hand"), "{surface}");
}

#[test]
fn p68_cast_spells_are_exiled_as_they_resolve_not_on_cast() {
    for (name, text) in [
        ("Rod of Absorption",
         "Mana cost: {2}{U}\nType: Artifact\nWhenever a player casts an instant or sorcery spell, exile it instead of putting it into a graveyard as it resolves.\n{X}, {T}, Sacrifice this artifact: You may cast any number of spells from among cards exiled with this artifact with total mana value X or less without paying their mana costs."),
        ("River Song's Diary",
         "Mana cost: {3}\nType: Artifact — Book\nImprint — Whenever a player casts an instant or sorcery spell from their hand, exile it instead of putting it into a graveyard as it resolves.\nAt the beginning of your upkeep, if there are four or more cards exiled with this artifact, choose one of them at random. You may cast it without paying its mana cost."),
    ] {
        let definition = p68_compile(name, text);
        let debug = format!("{:?}", definition.abilities);
        let replacement = debug.find("RegisterFutureZoneReplacementEffect").unwrap_or_else(|| panic!("{name}: {debug}"));
        let tail = &debug[replacement..];
        assert!(tail.contains("link_exiled_to_source: true"), "{name}: {tail}");
        assert!(!debug.contains("ExileEffect { spec: Tagged(TagKey(\"triggering\"))"), "{name}: never exiled on cast: {debug}");
        let surface = p68_text(&definition);
        assert!(surface.contains("instead of putting it into a graveyard as it resolves"), "{name}: {surface}");
    }
}

#[test]
fn p68_scarlet_spider_counters_read_the_linked_web_slinging_cast() {
    let definition = p68_compile("Scarlet Spider, Ben Reilly",
        "Mana cost: {1}{R}{G}\nType: Legendary Creature — Spider Human Hero\nPower/Toughness: 4/3\nWeb-slinging {R}{G}\nTrample\nSensational Save — If Scarlet Spider was cast using web-slinging, he enters with X +1/+1 counters on him, where X is the mana value of the returned creature.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("EntersWithCountersIfCondition"), "{debug}");
    assert!(!debug.contains("CustomUnsupported"), "{debug}");
    assert!(debug.contains("AlternativeCast"), "{debug}");
}

#[test]
fn p68_sorrows_path_exchanges_the_two_declared_same_opponent_blockers() {
    let definition = p68_compile("Sorrow's Path",
        "Type: Land\n{T}: Choose two target blocking creatures controlled by the same opponent. If each of those creatures could block all creatures that the other is blocking, remove both of them from combat. Each one then blocks all creatures the other was blocking.\nWhenever this land becomes tapped, it deals 2 damage to you and each creature you control.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("target_set_same_controller: true"), "{debug}");
    let exchange = debug.find("ExchangeBlocksEffect").expect("exchange");
    assert!(!debug[exchange..].starts_with("ExchangeBlocksEffect { target: SurfaceHinted { spec: SurfaceHinted { spec: Source"),
        "the pair is the declared targets, not this land: {debug}");
    assert!(debug[exchange..].contains("Tagged("), "{debug}");
    let surface = p68_text(&definition);
    assert!(surface.contains("controlled by the same opponent"), "{surface}");
}

#[test]
fn p68_kotose_search_and_shuffle_follow_the_exiled_cards_owner() {
    let definition = p68_compile("Kotose, the Silent Spider",
        "Mana cost: {3}{U}{B}\nType: Legendary Creature — Human Ninja\nPower/Toughness: 4/4\nWhen Kotose enters, exile target card other than a basic land card from an opponent's graveyard. Search that player's graveyard, hand, and library for any number of cards with the same name as that card and exile them. Then that player shuffles. For as long as you control Kotose, you may play one of the exiled cards, and you may spend mana as though it were mana of any color to cast it.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("AliasedOwnerOf"), "that player = the exiled card's owner: {debug}");
    assert!(!debug.contains("ShuffleLibraryEffect { player: Opponent }"), "{debug}");
}

#[test]
fn p68_hama_grants_a_lasting_waterbend_priced_cast() {
    let definition = p68_compile("Hama, the Bloodbender",
        "Mana cost: {2}{U/B}{U/B}{U/B}\nType: Legendary Creature — Human Warlock\nPower/Toughness: 3/3\nWhen Hama enters, target opponent mills three cards. Exile up to one noncreature, nonland card from that player's graveyard. For as long as you control Hama, you may cast the exiled card during your turn by waterbending {X} rather than paying its mana cost, where X is its mana value.");
    let debug = format!("{:?}", definition.abilities);
    let grant = debug.find("GrantPlayTaggedEffect").expect("lasting cast permission");
    let tail = &debug[grant..];
    assert!(tail.contains("ForAsLongAsYouControlSource"), "{tail}");
    assert!(tail.contains("during_your_turns: true"), "{tail}");
    assert!(tail.contains("alternative_cost: Some"), "{tail}");
}
