//! Claude package 27 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

#[test]
fn p27_conduit_of_storms_trailing_main_phase_mana_delay() {
    let definition = compile_to_runtime_definition("Conduit of Storms",
        "Mana cost: {2}{R}\nType: Creature — Werewolf Horror\nWhenever this creature attacks, add {R} at the beginning of your next main phase this turn.\n{3}{R}{R}: Transform this creature.", false)
        .expect("Conduit of Storms should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("BeginningOfMainPhase"), "the mana waits for your next main phase: {debug}");
    assert!(debug.contains("ScheduleDelayedTrigger"), "the trigger schedules a delay instead of adding mana now: {debug}");
    assert!(debug.contains("AddMana") || debug.contains("Mana"), "the delayed payload adds mana: {debug}");
}

#[test]
fn p27_conjurers_ban_prohibits_casting_and_playing_the_chosen_name() {
    let definition = compile_to_runtime_definition("Conjurer's Ban",
        "Mana cost: {W}{B}\nType: Sorcery\nChoose a card name. Until your next turn, spells with the chosen name can't be cast and lands with the chosen name can't be played.\nDraw a card.", false)
        .expect("Conjurer's Ban should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("CastSpellsMatching"), "spells with the name can't be cast: {debug}");
    assert!(debug.contains("PlayLandsMatching"), "lands with the name can't be played: {debug}");
    assert!(debug.matches("{chosen name}").count() >= 2, "both read the chosen name: {debug}");
}

#[test]
fn p27_clowning_around_compares_the_roll_with_your_robot_count() {
    let definition = compile_to_runtime_definition("Clowning Around",
        "Mana cost: {1}{W}\nType: Sorcery\nCreate two 1/1 white Clown Robot artifact creature tokens, then roll a six-sided die. If the result is equal to or less than the number of Robots you control, create a 1/1 white Clown Robot artifact creature token.", false)
        .expect("Clowning Around should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("LessThanOrEqual"), "the roll must be at most the count: {debug}");
    assert!(debug.contains("Rolled"), "the left side is the completed roll: {debug}");
    assert!(debug.contains("Robot"), "the right side counts Robots you control: {debug}");
}

#[test]
fn p27_command_the_stage_reads_noncombat_damage_from_the_previous_turn() {
    let definition = compile_to_runtime_definition("Command the Stage",
        "Mana cost: {2}{R}\nType: Sorcery\nCreate a 2/2 colorless Wizard Soldier creature token named Cadet, then put a +1/+1 counter on each other Wizard token you control.\nAt the beginning of each upkeep, if an opponent was dealt noncombat damage last turn, return this card from your graveyard to your hand.", false)
        .expect("Command the Stage should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("PlayerWasDealtNoncombatDamageLastTurn(Opponent)"),
        "the intervening if reads the previous turn's noncombat damage to opponents: {debug}");
    assert!(debug.contains("Cadet"), "the token is named Cadet: {debug}");
}

#[test]
fn p27_commanders_insight_draws_x_plus_one_per_commander_cast() {
    let definition = compile_to_runtime_definition("Commander's Insight",
        "Mana cost: {X}{U}{U}{U}\nType: Instant\nTarget player draws X cards plus an additional card for each time they've cast a commander from the command zone this game.", false)
        .expect("Commander's Insight should compile");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("Add(") && debug.contains("CommanderCastCount"),
        "the draw count is X plus the commander-cast count: {debug}");
    assert!(!debug.contains("CommanderCastCount(IteratedPlayer)"),
        "\"they\" is the targeted drawing player, not an unbound iterated player: {debug}");
}

#[test]
fn p27_clash_of_elements_negated_put_restatement_reads_as_if_they_dont() {
    let definition = compile_to_runtime_definition("Clash of Elements",
        "Mana cost: {1}{U}{R}\nType: Instant\nChoose target nonland permanent. Its owner may put it on top of their library. If they do, Clash of Elements deals 2 damage to them. If they didn't put the card on top of their library, they put it on the bottom.", false)
        .expect("Clash of Elements should compile");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("DidNotHappen"), "the bottom put runs only when the optional top put did not: {debug}");
    assert!(debug.contains("DealDamage"), "the accepted put is punished with damage: {debug}");
}

#[test]
fn p27_common_cause_pumps_only_while_every_nonartifact_creature_shares_a_color() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::ids::{CardId, PlayerId};
    use ironsmith::mana::{ManaCost, ManaSymbol};
    use ironsmith::types::CardType;
    use ironsmith::zone::Zone;
    let definition = compile_to_runtime_definition("Common Cause",
        "Mana cost: {2}{W}\nType: Enchantment\nNonartifact creatures get +2/+2 as long as they all share a color.", false)
        .expect("Common Cause should compile");
    let alice = PlayerId::from_index(0);
    let creature = |name: &str, symbol: ManaSymbol| CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .mana_cost(ManaCost::from_symbols(vec![symbol]))
        .power_toughness(PowerToughness::fixed(1, 1)).build();
    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let first = game.create_object_from_card(&creature("White One", ManaSymbol::White), alice, Zone::Battlefield);
    game.create_object_from_card(&creature("White Two", ManaSymbol::White), alice, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.calculated_power(first), Some(3), "two white creatures share white");
    game.create_object_from_card(&creature("Red One", ManaSymbol::Red), alice, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.calculated_power(first), Some(1), "a red creature breaks the shared color");
}

#[test]
fn p27_clouds_limit_break_blade_beam_targets_need_different_controllers() {
    let definition = compile_to_runtime_definition("Cloud's Limit Break",
        "Mana cost: {1}{W}\nType: Instant\nTiered (Choose one additional cost.)\n• Cross-Slash — {0} — Destroy target tapped creature.\n• Blade Beam — {1} — Destroy any number of target tapped creatures with different controllers.\n• Omnislash — {3}{W} — Destroy all tapped creatures.", false)
        .expect("Cloud's Limit Break should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("target_set_different_controllers: true"),
        "Blade Beam's targets must each have a different controller: {debug}");
}

#[test]
fn p27_cloud_planets_champion_needs_your_turn_and_equipment() {
    let definition = compile_to_runtime_definition("Cloud, Planet's Champion",
        "Mana cost: {3}{R}{W}\nType: Legendary Creature — Human Soldier Mercenary\nPower/Toughness: 4/4\nDuring your turn, as long as Cloud is equipped, it has double strike and indestructible. (This creature deals both first-strike and regular combat damage. Damage and effects that say \"destroy\" don't destroy this creature.)\nEquip abilities you activate that target Cloud cost {2} less to activate.", false)
        .expect("Cloud, Planet's Champion should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("DuringYourTurn"), "the grant is limited to your turn: {debug}");
    assert!(debug.contains("SourceIsEquipped") || debug.contains("Equipped"),
        "and to while Cloud is equipped: {debug}");
    assert!(debug.contains("DoubleStrike") && debug.contains("Indestructible"), "{debug}");
}

#[test]
fn p27_circuits_act_creates_one_token_per_different_die_result() {
    use ironsmith::ids::PlayerId;
    use ironsmith::zone::Zone;
    let definition = compile_to_runtime_definition("Circuits Act",
        "Mana cost: {2}{R}\nType: Sorcery\nRoll three six-sided dice. For each different result, create a 1/1 white Clown Robot artifact creature token.", false)
        .expect("Circuits Act should compile");
    let debug = format!("{:?}", definition.spell_effect);
    assert!(debug.contains("DistinctDieResults"), "the token count reads the different results: {debug}");

    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut ctx = ironsmith::effects::EffectContext::new_default(spell, alice);
    for effect in definition.spell_effect.as_ref().expect("spell effect").all_effects() {
        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    let tokens = game.battlefield.iter()
        .filter(|id| game.object(**id).is_some_and(|object| object.kind == ironsmith::object::ObjectKind::Token))
        .count();
    assert!((1..=3).contains(&tokens), "three dice show one to three different results, got {tokens}");
}

#[test]
fn p27_clay_champion_counts_complete_symbol_pairs_spent() {
    let definition = compile_to_runtime_definition("Clay Champion",
        "Mana cost: {X}{4}\nType: Artifact Creature — Construct\nPower/Toughness: 0/0\nThis creature enters with three +1/+1 counters on it for each {G}{G} spent to cast it.\nWhen this creature enters, choose up to two other target creatures you control. For each {W}{W} spent to cast this creature, put a +1/+1 counter on each of them.", false)
        .expect("Clay Champion should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("DividedRoundedDown"), "a pair of symbols counts once: {debug}");
    assert!(debug.contains("symbol: Green"), "the entry counters read {{G}} spent: {debug}");
    assert!(debug.contains("symbol: White"), "the trigger reads {{W}} spent: {debug}");
}

#[test]
fn p27_coiling_rebirth_copies_only_with_gift_and_nonlegendary() {
    let definition = compile_to_runtime_definition("Coiling Rebirth",
        "Mana cost: {3}{B}{B}\nType: Sorcery\nGift a card (You may promise an opponent a gift as you cast this spell. If you do, they draw a card before its other effects.)\nReturn target creature card from your graveyard to the battlefield. Then if the gift was promised and that creature isn't legendary, create a token that's a copy of that creature, except it's 1/1.", false)
        .expect("Coiling Rebirth should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("Gift"), "the copy needs the promised gift: {debug}");
    assert!(debug.contains("Legendary"), "and a nonlegendary returned creature: {debug}");
    assert!(debug.contains("CreateTokenCopy"), "the copy is a token: {debug}");
}

#[test]
fn p27_creative_technique_shuffles_then_reveals_until_nonland() {
    let definition = compile_to_runtime_definition("Creative Technique",
        "Mana cost: {4}{R}\nType: Sorcery\nDemonstrate (When you cast this spell, you may copy it. If you do, choose an opponent to also copy it.)\nShuffle your library, then reveal cards from the top of it until you reveal a nonland card. Exile that card and put the rest on the bottom of your library in a random order. You may cast the exiled card without paying its mana cost.", false)
        .expect("Creative Technique should compile");
    let debug = format!("{:?}", definition.spell_effect);
    let shuffle = debug.find("ShuffleLibrary").expect("the library is shuffled first");
    let reveal = debug.find("RevealUntil").or_else(|| debug.find("Consult")).expect("then a reveal-until procedure");
    assert!(shuffle < reveal, "shuffle precedes the reveal: {debug}");
    assert!(debug.contains("without_paying") || debug.contains("WithoutPaying"), "the hit is cast free: {debug}");
}

#[test]
fn p27_concerted_effort_copies_landwalk_and_protection_variants() {
    let definition = compile_to_runtime_definition("Concerted Effort",
        "Mana cost: {2}{W}{W}\nType: Enchantment\nAt the beginning of each upkeep, creatures you control gain flying until end of turn if a creature you control has flying. The same is true for fear, first strike, double strike, landwalk, protection, trample, and vigilance.", false)
        .expect("Concerted Effort should compile");
    let debug = format!("{definition:?}");
    assert_eq!(debug.matches("CopyStaticAbilityVariants").count() >= 2, true,
        "landwalk and protection copy the variants your creatures have: {debug}");
    assert!(debug.contains("Any(Landwalk)") && debug.contains("Any(Protection)"), "{debug}");
    assert!(debug.contains("Fear") && debug.contains("Vigilance"), "plain keywords keep their conditional grants: {debug}");
}

#[test]
fn p27_conspiracy_replaces_creature_types_on_and_off_the_battlefield() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::ids::{CardId, PlayerId};
    use ironsmith::types::{CardType, Subtype};
    use ironsmith::zone::Zone;
    let definition = compile_to_runtime_definition("Conspiracy",
        "Mana cost: {3}{B}{B}\nType: Enchantment\nAs this enchantment enters, choose a creature type.\nCreatures you control are the chosen type. The same is true for creature spells you control and creature cards you own that aren't on the battlefield.", false)
        .expect("Conspiracy should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("replace_existing: true"), "the chosen type replaces creature types: {debug}");

    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let conspiracy = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.set_chosen_creature_type(conspiracy, Subtype::Elf);
    let goblin = CardBuilder::new(CardId::new(), "Goblin Witness").card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Goblin]).power_toughness(PowerToughness::fixed(1, 1)).build();
    let on_field = game.create_object_from_card(&goblin, alice, Zone::Battlefield);
    let in_hand = game.create_object_from_card(&goblin, alice, Zone::Hand);
    game.refresh_continuous_state().unwrap();
    for id in [on_field, in_hand] {
        let subtypes = game.calculated_subtypes(id);
        assert!(subtypes.contains(&Subtype::Elf) && !subtypes.contains(&Subtype::Goblin),
            "the chosen type replaces Goblin: {subtypes:?}");
    }
}

#[test]
fn p27_court_of_cunning_monarch_mills_ten_instead_for_each_target() {
    let definition = compile_to_runtime_definition("Court of Cunning",
        "Mana cost: {1}{U}{U}\nType: Enchantment\nWhen this enchantment enters, you become the monarch.\nAt the beginning of your upkeep, any number of target players each mill two cards. If you're the monarch, each of those players mills ten cards instead. (To mill a card, a player puts the top card of their library into their graveyard.)", false)
        .expect("Court of Cunning should compile");
    let debug = format!("{definition:?}");
    assert_eq!(debug.matches("TargetOnlyEffect").count(), 1, "the target players are declared once: {debug}");
    assert!(debug.contains("Fixed(\n") || debug.contains("Fixed(10)") || debug.contains("10"), "{debug}");
    assert!(debug.contains("Monarch"), "the ten-card mill is gated on being the monarch: {debug}");
}

#[test]
fn p27_coveted_falcon_draws_per_permanent_whose_control_changed() {
    let definition = compile_to_runtime_definition("Coveted Falcon",
        "Mana cost: {1}{U}{U}\nType: Artifact Creature — Bird\nPower/Toughness: 1/4\nFlying\nWhenever this creature attacks, gain control of target permanent you own but don't control.\nDisguise {1}{U}\nWhen this creature is turned face up, target opponent gains control of any number of target permanents you control. Draw a card for each one they gained control of this way.", false)
        .expect("Coveted Falcon should compile");
    let debug = format!("{definition:?}");
    assert!(!debug.contains("PendingEffectMetric") && !debug.contains("PendingPriorEffectMetric"),
        "the draw count is bound to the control change: {debug}");
    assert!(debug.contains("DrawCardsEffect"), "{debug}");
}

#[test]
fn p27_codie_ravenous_codex_copies_prepared_spells() {
    use ironsmith::ids::{CardId, ObjectId, PlayerId};
    use ironsmith::zone::Zone;
    let definition = compile_to_runtime_definition("Codie, Ravenous Codex",
        "Mana cost: {3}\nType: Legendary Artifact Creature — Book Construct\nPower/Toughness: 1/4\nWhenever you cast a prepared spell, copy it. You may choose new targets for the copy.\n{W}{U}{B}{R}{G}, {T}: Each creature you control becomes prepared. (Only creatures with prepare spells can become prepared.)", false)
        .expect("Codie, Ravenous Codex should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("prepared: true"), "the cast trigger only matches prepared spells: {debug}");
    assert!(debug.contains("CopySpell"), "it copies the spell: {debug}");

    // The engine records the stack object of a prepare-copy cast.
    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let bolt = ironsmith::card::CardBuilder::new(CardId::new(), "Witness Bolt")
        .card_types(vec![ironsmith::types::CardType::Instant]).build();
    let spell = game.create_object_from_card(&bolt, alice, Zone::Stack);
    assert!(!game.was_cast_as_prepared_spell(spell), "an ordinary spell isn't prepared");
    game.record_prepared_spell_cast(spell);
    assert!(game.was_cast_as_prepared_spell(spell), "a spell cast from a prepare copy is");
    let _ = ObjectId::from_raw(0);
}

#[test]
fn p27_clandestine_chameleon_borrows_owned_ability_sticker_abilities() {
    let definition = compile_to_runtime_definition("Clandestine Chameleon",
        "Mana cost: {3}{G}\nType: Creature — Lizard Guest\nPower/Toughness: 3/3\nWhen Clandestine Chameleon enters, you get {TK}{TK}, then you may put a sticker on a nonland permanent you own.\nClandestine Chameleon has all abilities of ability stickers on other permanents you own and cards in your graveyard.", false)
        .expect("Clandestine Chameleon should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("SourceHasAbilityStickerAbilitiesOf"), "the static borrows sticker abilities: {debug}");
    assert!(debug.contains("Graveyard"), "graveyard cards are included: {debug}");
}

#[test]
fn p27_city_in_a_bottle_sacrifices_and_bans_arabian_nights_names() {
    let definition = compile_to_runtime_definition("City in a Bottle",
        "Mana cost: {2}\nType: Artifact\nWhenever one or more other nontoken permanents with a name originally printed in the Arabian Nights expansion are on the battlefield, their controllers sacrifice them.\nPlayers can't cast spells or play lands with a name originally printed in the Arabian Nights expansion.", false)
        .expect("City in a Bottle should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("StateTrigger"), "a state trigger watches the battlefield: {debug}");
    assert!(debug.contains("ForPlayersEffect") && debug.contains("Sacrifice"),
        "each controller sacrifices all of theirs: {debug}");
    assert!(debug.matches("Arabian Nights").count() >= 3, "trigger, sacrifice and both bans read the set: {debug}");
    assert!(debug.contains("CastSpellsMatching") && debug.contains("PlayLandsMatching"), "{debug}");
}
