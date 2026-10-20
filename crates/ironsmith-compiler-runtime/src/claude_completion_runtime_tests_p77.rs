//! Claude package 77 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/claude2/BRIEF.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

/// "instant and/or sorcery spells" is a disjunction of card types: the cast
/// pool must not require a card to be both an instant and a sorcery.
#[test]
fn and_or_spell_type_list_is_a_disjunction_in_collection_casts() {
    for (name, text) in [
        (
            "Invoke Calamity",
            "Mana cost: {1}{R}{R}{R}{R}\nType: Instant\nYou may cast up to two instant and/or sorcery spells with total mana value 6 or less from your graveyard and/or hand without paying their mana costs. If those spells would be put into your graveyard, exile them instead. Exile Invoke Calamity.",
        ),
        (
            "Chandra Ablaze",
            "Mana cost: {4}{R}{R}\nType: Legendary Planeswalker — Chandra\nLoyalty: 5\n+1: Discard a card. If a red card is discarded this way, Chandra deals 4 damage to any target.\n−2: Each player discards their hand, then draws three cards.\n−7: Cast any number of red instant and/or sorcery cards from your graveyard without paying their mana costs.",
        ),
    ] {
        let definition = compile_to_runtime_definition(name, text, false)
            .unwrap_or_else(|error| panic!("{name} should compile: {error:?}"));
        let debug = format!("{definition:?}");
        assert!(
            !debug.contains("all_card_types: [Instant, Sorcery]"),
            "{name}: an and/or type list is not a conjunction: {debug}"
        );
        assert!(debug.contains("card_types: [Instant, Sorcery]"), "{name}: {debug}");
        assert!(debug.contains("CastTaggedEffect"), "{name}: {debug}");
    }
}

/// "You and target player exchange control of the creature you each control
/// with the greatest mana value.": the target player's pick is among the
/// permanents that player controls, never among the caster's.
#[test]
fn juxtapose_target_player_chooses_among_their_own_greatest_permanents() {
    let definition = compile_to_runtime_definition(
        "Juxtapose",
        "Mana cost: {3}{U}\nType: Sorcery\nYou and target player exchange control of the creature you each control with the greatest mana value. Then exchange control of artifacts the same way. If two or more permanents a player controls are tied for greatest, their controller chooses one of them.",
        false,
    )
    .expect("Juxtapose should compile");
    let debug = format!("{definition:?}");
    let theirs: Vec<_> = debug.match_indices("ChooseObjectsEffect").map(|(index, _)| index).collect();
    assert_eq!(theirs.len(), 4, "{debug}");
    for (position, start) in theirs.iter().enumerate() {
        let end = theirs.get(position + 1).copied().unwrap_or(debug.len());
        let choice = &debug[*start..end];
        let chooser_is_target = choice.contains("chooser: Target(");
        if chooser_is_target {
            assert!(
                choice.contains("controller: Some(Target(Any))"),
                "the target player's pool is their own permanents: {choice}"
            );
            assert!(!choice.contains("controller: Some(You)"), "{choice}");
        } else {
            assert!(choice.contains("controller: Some(You)"), "{choice}");
        }
    }
    assert_eq!(debug.matches("ExchangeControlEffect").count(), 2, "{debug}");
}

/// "Whenever an Aura you control enters, if you cast it, ...": the cast check
/// names the entering Aura, not the source; and "with a different name than
/// each Aura you control" excludes every Aura name you control.
#[test]
fn light_paws_checks_the_entering_aura_was_cast_and_excludes_each_controlled_aura_name() {
    let definition = compile_to_runtime_definition(
        "Light-Paws, Emperor's Voice",
        "Mana cost: {1}{W}\nType: Legendary Creature — Fox Advisor\nPower/Toughness: 2/2\nWhenever an Aura you control enters, if you cast it, you may search your library for an Aura card with mana value less than or equal to that Aura and with a different name than each Aura you control, put that card onto the battlefield attached to Light-Paws, then shuffle.",
        false,
    )
    .expect("Light-Paws should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("TriggeringObjectWasCastByController"), "cast actor must be retained: {debug}");
    assert!(!debug.contains("intervening_if: Some(SourceWasCast)"), "{debug}");
    assert!(debug.contains("TagMatchingObjectsEffect"), "the controlled Auras are tagged: {debug}");
    assert!(debug.contains("DifferentNameFromTagged"), "{debug}");
    assert!(debug.contains("ManaValueOf(Tagged(TagKey(\"triggering\")))"), "{debug}");
    assert!(debug.contains("AttachObjectsEffect"), "{debug}");

    let sibsig = compile_to_runtime_definition(
        "The Sibsig Ceremony",
        "Mana cost: {B}{B}{B}\nType: Legendary Enchantment\nCreature spells you cast cost {2} less to cast.\nWhenever a creature you control enters, if you cast it, destroy that creature, then create a 2/2 black Zombie Druid creature token.",
        false,
    )
    .expect("The Sibsig Ceremony should compile");
    let debug = format!("{:?}", sibsig.abilities);
    assert!(debug.contains("TriggeringObjectWasCastByController"), "the entering creature was cast by you: {debug}");
    assert!(!debug.contains("intervening_if: Some(SourceWasCast)"), "{debug}");
}

/// The return clause is installed while the ETB ability resolves. It remains
/// live after the source leaves and watches only cards exiled by that ability.
#[test]
fn source_until_exile_return_compiles_as_delayed_one_shot_watchers() {
    let definition = compile_to_runtime_definition(
        "Lagrella, the Magpie",
        "Mana cost: {G}{W}{U}\nType: Legendary Creature — Human Soldier\nPower/Toughness: 2/3\nWhen Lagrella enters, exile any number of other target creatures controlled by different players until Lagrella leaves the battlefield. When an exiled card enters under your control this way, put two +1/+1 counters on it.",
        false,
    )
    .expect("linked exile and its delayed return should compile");
    let debug = format!("{:#?}", definition.abilities);
    assert_eq!(debug.matches("TriggeredAbility").count(), 1, "{debug}");
    assert!(debug.contains("ExileUntilEffect"), "{debug}");
    assert!(debug.contains("target_set_different_controllers: true"), "{debug}");
    assert!(debug.contains("ScheduleDelayedTriggerEffect"), "{debug}");
    assert!(debug.contains("duration_return_from_source: true"), "{debug}");
    assert!(debug.contains("one_shot: true"), "{debug}");
    assert!(debug.contains("watch_ability_source: true"), "{debug}");
    let text = ironsmith_text::compiled_text_lines(&definition).join("\n");
    assert!(
        text.contains("When an exiled card enters under your control this way, put two +1/+1 counters on it"),
        "the printed delayed clause must preserve the duration-return provenance: {text}"
    );
}

/// "If an opponent ... would put a land onto the battlefield": the replaced
/// zone change is matched against the land before it moves (in a hand or a
/// library), so the event filter must not require the battlefield.
#[test]
fn land_equilibrium_matches_lands_entering_from_any_zone() {
    let definition = compile_to_runtime_definition(
        "Land Equilibrium",
        "Mana cost: {2}{U}{U}\nType: Enchantment\nIf an opponent who controls at least as many lands as you do would put a land onto the battlefield, that player instead puts that land onto the battlefield then sacrifices a land of their choice.",
        false,
    )
    .expect("Land Equilibrium should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(
        debug.contains("event: ZoneChange { object: ObjectFilter { zone: None"),
        "the entering land is matched in its origin zone: {debug}"
    );
    assert!(!debug.contains("event: ZoneChange { object: ObjectFilter { zone: Some(Battlefield)"), "{debug}");
    assert!(debug.contains("owner: Some(Opponent)"), "{debug}");
    assert!(debug.contains("to: Some(Battlefield)"), "{debug}");
}

/// "exile up to one target legendary artifact card or legendary land card
/// from a graveyard and put a discovery counter on it": the `or` joins two
/// nouns of the target phrase; it is not a choice between two actions.
#[test]
fn lara_croft_exiles_a_legendary_artifact_or_land_card_from_a_graveyard_and_marks_it() {
    let definition = compile_to_runtime_definition(
        "Lara Croft, Tomb Raider",
        "Mana cost: {G}{U}{R}\nType: Legendary Creature — Human Ranger\nPower/Toughness: 3/4\nFirst strike, reach\nWhenever Lara Croft attacks, exile up to one target legendary artifact card or legendary land card from a graveyard and put a discovery counter on it. You may play a card from exile with a discovery counter on it this turn.\nRaid — At end of combat on your turn, if you attacked this turn, create a Treasure token.",
        false,
    )
    .expect("Lara Croft should compile");
    let debug = format!("{:?}", definition.abilities);
    let attack = debug.find("AttacksTrigger").unwrap_or_else(|| panic!("{debug}"));
    let raid = debug[attack..].find("EndOfCombatTrigger").map_or(debug.len(), |end| attack + end);
    let attack_ability = &debug[attack..raid];
    assert!(!attack_ability.contains("ChooseModeEffect"), "no modal split: {attack_ability}");
    assert!(attack_ability.contains("zone: Some(Graveyard)"), "the target is a graveyard card: {attack_ability}");
    assert!(attack_ability.contains("Land"), "legendary land cards are legal targets: {attack_ability}");
    assert!(attack_ability.contains("Artifact"), "{attack_ability}");
    assert!(attack_ability.contains("ExileEffect"), "{attack_ability}");
    assert!(attack_ability.contains("Named(\"discovery\")"), "{attack_ability}");
    assert!(
        !attack_ability.contains("PutCountersEffect { counter_type: Named(\"discovery\"), amount: Fixed(1), target: Tagged(TagKey(\"triggering\"))"),
        "the counter goes on the exiled card, not on Lara: {attack_ability}"
    );
    assert!(attack_ability.contains("PlayFrom"), "{attack_ability}");
}

/// "Whenever Kotis deals combat damage to a player, exile the top X cards of
/// their library": "their" is the damaged player, not Kotis's controller.
#[test]
fn kotis_exiles_from_the_damaged_players_library() {
    let definition = compile_to_runtime_definition(
        "Kotis, the Fangkeeper",
        "Mana cost: {1}{B}{G}{U}\nType: Legendary Creature — Zombie Warrior\nPower/Toughness: 2/1\nIndestructible\nWhenever Kotis deals combat damage to a player, exile the top X cards of their library, where X is the amount of damage dealt. You may cast any number of spells with mana value X or less from among them without paying their mana costs.",
        false,
    )
    .expect("Kotis should compile");
    let debug = format!("{:?}", definition.abilities);
    let exile = debug.find("ExileTopOfLibraryEffect").unwrap_or_else(|| panic!("{debug}"));
    let exile = &debug[exile..];
    let exile = &exile[..exile.find("moved_tags").unwrap_or(exile.len())];
    assert!(exile.contains("player: DamagedPlayer"), "the damaged player's library: {exile}");
    assert!(!exile.contains("ControllerOf"), "{exile}");
    assert!(debug.contains("CastTaggedEffect"), "{debug}");
}

/// "−7: Each creature you control deals damage equal to its power to each
/// opponent.": each creature deals its own damage; the source is the
/// iterated creature, never a fresh choice among creatures you control.
#[test]
fn lukka_ultimate_damage_source_is_each_iterated_creature() {
    let definition = compile_to_runtime_definition(
        "Lukka, Coppercoat Outcast",
        "Mana cost: {3}{R}{R}\nType: Legendary Planeswalker — Lukka\nLoyalty: 5\n+1: Exile the top three cards of your library. Creature cards exiled this way gain \"You may cast this card from exile as long as you control a Lukka planeswalker.\"\n−2: Exile target creature you control, then reveal cards from the top of your library until you reveal a creature card with greater mana value. Put that card onto the battlefield and the rest on the bottom of your library in a random order.\n−7: Each creature you control deals damage equal to its power to each opponent.",
        false,
    )
    .expect("Lukka should compile");
    let debug = format!("{:?}", definition.abilities);
    let ultimate = debug.find("Fixed(7)").unwrap_or_else(|| panic!("{debug}"));
    let ultimate = &debug[ultimate..];
    assert!(ultimate.contains("ForEachObject"), "{ultimate}");
    assert!(ultimate.contains("ExecuteWithSourceEffect { source: Iterated"), "{ultimate}");
    assert!(!ultimate.contains("ExecuteWithSourceEffect { source: Object("), "{ultimate}");
}

/// Journey to the Lost City: "those cards" in the die branches are the four
/// exiled cards. The 1-9 land choice looks in exile (not the library), and the
/// 10-19 counter count reads creature cards among the exiled cards, not the
/// freshly created Wolf token.
#[test]
fn journey_to_the_lost_city_branches_read_the_exiled_cards() {
    let definition = compile_to_runtime_definition(
        "Journey to the Lost City",
        "Mana cost: {3}{G}\nType: Enchantment\nAt the beginning of your upkeep, exile the top four cards of your library, then roll a d20.\n1—9 | You may put a land card from among those cards onto the battlefield.\n10—19 | Create a 2/2 green Wolf creature token, then put a +1/+1 counter on it for each creature card among those cards.\n20 | Put all permanent cards exiled with this enchantment onto the battlefield, then sacrifice it.",
        false,
    )
    .expect("Journey to the Lost City should compile");
    let debug = format!("{:?}", definition.abilities);
    let low = debug.find("BetweenInclusive(1, 9)").unwrap_or_else(|| panic!("{debug}"));
    let mid = debug.find("BetweenInclusive(10, 19)").unwrap_or_else(|| panic!("{debug}"));
    let land_branch = &debug[low..mid];
    assert!(land_branch.contains("ChooseObjectsEffect"), "{land_branch}");
    assert!(land_branch.contains("zone: Some(Exile)"), "the land is chosen in exile: {land_branch}");
    assert!(!land_branch.contains("zone: Some(Library)"), "{land_branch}");
    let top = debug[mid..].find("Equal(20)").map_or(debug.len(), |end| mid + end);
    let wolf_branch = &debug[mid..top];
    let count = wolf_branch.find("PutCountersEffect").unwrap_or_else(|| panic!("{wolf_branch}"));
    let count = &wolf_branch[count..];
    let amount = &count[..count.find("target:").unwrap_or(count.len())];
    assert!(amount.contains("__sentence_helper_exiled"), "counted among the exiled cards: {amount}");
    assert!(!amount.contains("TagKey(\"created_"), "not the created Wolf: {amount}");
}

/// Immortal Obligation: the duty-counter sentence continues the returned
/// creature's instruction (goad plus two restrictions for as long as it has a
/// duty counter); it must never become static abilities of the instant.
#[test]
fn immortal_obligation_goads_and_restricts_the_returned_creature_as_spell_effects() {
    let definition = compile_to_runtime_definition(
        "Immortal Obligation",
        "Mana cost: {1}{W}\nType: Instant\nReturn target creature card from an opponent's graveyard to the battlefield under their control with a duty counter on it. For as long as that creature has a duty counter on it, it is goaded, can't attack you or a permanent you control, and can't block creatures you control.",
        false,
    )
    .expect("Immortal Obligation should compile");
    let abilities = format!("{:?}", definition.abilities);
    assert!(!abilities.contains("RuleRestriction"), "no statics on an instant: {abilities}");
    let spell = format!("{:?}", definition.spell_effect);
    assert!(spell.contains("ReturnFromGraveyardToBattlefieldEffect"), "{spell}");
    assert!(spell.contains("GoadEffect"), "the returned creature is goaded: {spell}");
    assert!(spell.contains("Named(\"duty\")"), "{spell}");
    assert!(spell.matches("CantEffect").count() >= 2, "attack and block restrictions: {spell}");
}
