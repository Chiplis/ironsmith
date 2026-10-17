//! Claude package 36 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

fn p36_compile(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    let (definition, trace) = ironsmith_compiler::parse_trace::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    definition.unwrap_or_else(|error| panic!("{name}: {error:?}\n{}", trace.render()))
}

#[test]
fn kianne_flash_permissions_follow_the_parity_of_its_power() {
    let definition = p36_compile("Kianne, Corrupted Memory",
        "Mana cost: {2}{G}{U}\nType: Legendary Creature — Illusion\nPower/Toughness: 2/2\nAs long as Kianne's power is even, you may cast noncreature spells as though they had flash.\nAs long as Kianne's power is odd, you may cast creature spells as though they had flash.\nWhenever you draw a card, put a +1/+1 counter on Kianne.");
    let debug = format!("{:?}", definition.abilities);
    assert_eq!(debug.matches("CountParity").count(), 2, "{debug}");
    assert!(debug.contains("count: SourcePower, even: true"), "{debug}");
    assert!(debug.contains("count: SourcePower, even: false"), "{debug}");
}

#[test]
fn lightning_runner_pays_a_counted_single_energy_symbol() {
    let definition = p36_compile("Lightning Runner",
        "Mana cost: {3}{R}{R}\nType: Creature — Human Warrior\nPower/Toughness: 2/2\nDouble strike, haste\nWhenever this creature attacks, you get {E}{E} (two energy counters), then you may pay eight {E}. If you pay, untap all creatures you control, and after this phase, there is an additional combat phase.");
    let debug = format!("{:?}", definition.abilities);
    let pay = debug.find("PayEnergyEffect").expect("energy payment");
    assert!(debug[pay..].contains("Fixed(8)"), "pays eight energy: {debug}");
}

#[test]
fn lathiel_distributes_up_to_the_life_gained_this_turn() {
    let definition = p36_compile("Lathiel, the Bounteous Dawn",
        "Mana cost: {2}{G}{W}\nType: Legendary Creature — Unicorn\nPower/Toughness: 2/2\nLifelink\nAt the beginning of each end step, if you gained life this turn, distribute up to that many +1/+1 counters among any number of other target creatures.");
    let debug = format!("{:?}", definition.abilities);
    let put = debug.find("PutCountersEffect").expect("counter placement");
    let tail = &debug[put..];
    assert!(tail.contains("LifeGainedThisTurn(You)"), "that many = life gained: {tail}");
    assert!(tail.contains("UpTo"), "up to: {tail}");
    assert!(tail.contains("distributed: true"), "{tail}");
    assert!(!debug.contains("EventValue(Amount)"), "{debug}");
}

#[test]
fn lantern_flare_bracketed_x_definition_binds_the_uncleaved_x() {
    let definition = p36_compile("Lantern Flare",
        "Mana cost: {1}{W}\nType: Instant\nCleave {X}{R}{W} (You may cast this spell for its cleave cost. If you do, remove the words in square brackets.)\nLantern Flare deals X damage to target creature or planeswalker and you gain X life. [X is the number of creatures you control.]");
    let debug = format!("{definition:?}");
    assert!(debug.contains("DealDamageEffect"), "{debug}");
    assert!(debug.contains("GainLifeEffect"), "{debug}");
    assert!(debug.contains("Count("), "printed X counts creatures you control: {debug}");
}

#[test]
fn legion_loyalist_grants_both_keywords_and_a_token_blocking_restriction() {
    let definition = p36_compile("Legion Loyalist",
        "Mana cost: {R}\nType: Creature — Goblin Soldier\nPower/Toughness: 1/1\nHaste\nBattalion — Whenever this creature and at least two other creatures attack, creatures you control gain first strike and trample until end of turn and can't be blocked by creature tokens this turn.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("FirstStrike"), "{debug}");
    assert!(debug.contains("Trample"), "{debug}");
    assert!(debug.contains("token"), "blocked-by-tokens restriction: {debug}");
}

#[test]
fn knollspine_dragon_draws_the_damage_dealt_to_the_target_opponent_this_turn() {
    let definition = p36_compile("Knollspine Dragon",
        "Mana cost: {5}{R}{R}\nType: Creature — Dragon\nPower/Toughness: 7/5\nFlying\nWhen this creature enters, you may discard your hand and draw cards equal to the damage dealt to target opponent this turn.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("DamageDealtToPlayersThisTurn(Target(Opponent))"), "{debug}");
    assert!(debug.contains("DrawCardsEffect"), "{debug}");
}

#[test]
fn lich_knights_conquest_sacrifices_from_one_serial_and_or_noun_list() {
    let definition = p36_compile("Lich-Knights' Conquest",
        "Mana cost: {4}{B}\nType: Sorcery\nSacrifice any number of artifacts, enchantments, and/or tokens. Return that many creature cards from your graveyard to the battlefield.");
    let debug = format!("{definition:?}");
    assert_eq!(debug.matches("SacrificeEffect").count(), 1, "one sacrifice choice: {debug}");
    assert!(debug.contains("Artifact") && debug.contains("Enchantment"), "{debug}");
    assert!(debug.contains("token"), "tokens are a third arm: {debug}");
}

#[test]
fn the_first_creature_or_card_names_the_triggering_object() {
    let rig = p36_compile("Laccolith Rig",
        "Mana cost: {R}\nType: Enchantment — Aura\nEnchant creature\nWhenever enchanted creature becomes blocked, you may have it deal damage equal to its power to target creature. If you do, the first creature assigns no combat damage this turn.");
    let debug = format!("{:?}", rig.abilities);
    let assign = debug.find("AssignNoCombatDamageEffect").expect("no combat damage");
    assert!(debug[assign..].contains("\"triggering\""), "the first creature is the blocked enchanted creature: {debug}");
    let lifeline = p36_compile("Lifeline",
        "Mana cost: {5}\nType: Artifact\nWhenever a creature dies, if another creature is on the battlefield, return the first card to the battlefield under its owner's control at the beginning of the next end step.");
    let debug = format!("{:?}", lifeline.abilities);
    assert!(debug.contains("TagTriggeringObjectEffect"), "{debug}");
    assert!(debug.contains("\"triggering\""), "{debug}");
}

#[test]
fn lashknife_barrier_reduces_damage_to_creatures_you_control_by_one() {
    let definition = p36_compile("Lashknife Barrier",
        "Mana cost: {2}{W}\nType: Enchantment\nWhen this enchantment enters, draw a card.\nIf a source would deal damage to a creature you control, it deals that much damage minus 1 to that creature instead.");
    let debug = format!("{:?}", definition.abilities);
    let replacement = debug.find("EventAmountReplacement").expect("damage amount replacement");
    let tail = &debug[replacement..];
    assert!(tail.contains("Subtract(1)"), "{tail}");
    assert!(tail.contains("controller: Some(You)"), "only creatures you control: {tail}");
}

#[test]
fn kopala_taxes_opponent_abilities_that_target_its_merfolk() {
    let definition = p36_compile("Kopala, Warden of Waves",
        "Mana cost: {1}{U}{U}\nType: Legendary Creature — Merfolk Wizard\nPower/Toughness: 2/2\nSpells your opponents cast that target a Merfolk you control cost {2} more to cast.\nAbilities your opponents activate that target a Merfolk you control cost {2} more to activate.");
    let debug = format!("{:?}", definition.abilities);
    let increase = debug.find("ActivatedAbilityCostIncrease").expect("ability tax");
    let tail = &debug[increase..];
    assert!(tail.contains("activator: Some(Opponent)"), "{tail}");
    assert!(tail.contains("TargetsExactly"), "only targeting activations: {tail}");
    assert!(tail.contains("Merfolk"), "{tail}");
}

#[test]
fn lethal_throwdown_draws_when_the_sacrificed_creature_was_modified() {
    let definition = p36_compile("Lethal Throwdown",
        "Mana cost: {B}\nType: Sorcery\nAs an additional cost to cast this spell, sacrifice a creature or sacrifice a modified creature. (Equipment, Auras you control, and counters are modifications.)\nDestroy target creature or planeswalker. If the modified creature was sacrificed, draw a card.");
    let debug = format!("{definition:?}");
    assert!(debug.contains("TaggedMatchedLastKnown"), "{debug}");
    assert!(debug.contains("modified: true"), "{debug}");
    assert!(debug.contains("DrawCardsEffect"), "{debug}");
}

#[test]
fn legions_end_exiles_same_named_cards_from_hand_and_graveyard() {
    let definition = p36_compile("Legion's End",
        "Mana cost: {1}{B}\nType: Sorcery\nExile target creature an opponent controls with mana value 2 or less and all other creatures that player controls with the same name as that creature. Then that player reveals their hand and exiles all cards with that name from their hand and graveyard.");
    let debug = format!("{definition:?}");
    assert!(debug.contains("Hand"), "{debug}");
    assert!(debug.contains("Graveyard"), "{debug}");
    assert!(debug.contains("SameNameAsTagged"), "{debug}");
}

#[test]
fn kharn_prevents_damage_to_itself_and_hands_itself_to_an_opponent() {
    let definition = p36_compile("Khârn the Betrayer",
        "Mana cost: {3}{R}\nType: Legendary Creature — Astartes Berserker\nPower/Toughness: 5/1\nBerzerker — Khârn the Betrayer attacks or blocks each combat if able.\nSigil of Corruption — When you lose control of Khârn the Betrayer, draw two cards.\nThe Betrayer — If damage would be dealt to Khârn the Betrayer, prevent that damage and an opponent of your choice gains control of it.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("PreventionFollowUp") || debug.contains("prevent"), "{debug}");
    assert!(debug.contains("GainControl"), "control change follow-up: {debug}");
}

#[test]
fn kor_chant_and_dirge_redirect_a_chosen_sources_damage_all_turn_to_another_target() {
    for (name, cost) in [("Kor Chant", "{2}{W}"), ("Kor Dirge", "{2}{B}")] {
        let definition = p36_compile(name, &format!(
            "Mana cost: {cost}\nType: Instant\nAll damage that would be dealt this turn to target creature you control by a source of your choice is dealt to another target creature instead."));
        let debug = format!("{definition:?}");
        let redirect = debug.find("RedirectNextTimeDamageToSourceEffect").expect("redirect");
        let tail = &debug[redirect..];
        assert!(tail.contains("source: Choice"), "{tail}");
        assert!(tail.contains("TargetObject"), "{tail}");
        assert!(tail.contains("all_this_turn: true"), "{tail}");
    }
}

#[test]
fn kytheon_transforms_only_when_it_and_two_other_creatures_attacked() {
    let definition = p36_compile("Kytheon, Hero of Akros",
        "Mana cost: {W}\nType: Legendary Creature — Human Soldier\nPower/Toughness: 2/1\nAt end of combat, if Kytheon and at least two other creatures attacked this combat, exile Kytheon, then return him to the battlefield transformed under his owner's control.\n{2}{W}: Kytheon gains indestructible until end of turn.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("SourceAttackedThisCombat"), "{debug}");
    assert!(debug.contains("GreaterThanOrEqual(2)"), "{debug}");
    assert!(debug.contains("attacking: true"), "{debug}");
}

#[test]
fn kotis_exiles_and_casts_up_to_the_combat_damage_dealt() {
    let definition = p36_compile("Kotis, the Fangkeeper",
        "Mana cost: {1}{B}{G}{U}\nType: Legendary Creature — Zombie Warrior\nPower/Toughness: 2/1\nIndestructible\nWhenever Kotis deals combat damage to a player, exile the top X cards of their library, where X is the amount of damage dealt. You may cast any number of spells with mana value X or less from among them without paying their mana costs.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("EventValue(Amount)"), "X is the damage dealt: {debug}");
    assert!(debug.contains("LessThanOrEqualExpr"), "{debug}");
}

#[test]
fn light_paws_searches_for_an_aura_no_costlier_than_the_entering_aura() {
    let definition = p36_compile("Light-Paws, Emperor's Voice",
        "Mana cost: {1}{W}\nType: Legendary Creature — Fox Advisor\nPower/Toughness: 2/2\nWhenever an Aura you control enters, if you cast it, you may search your library for an Aura card with mana value less than or equal to that Aura and with a different name than each Aura you control, put that card onto the battlefield attached to Light-Paws, then shuffle.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("LessThanOrEqualExpr(ManaValueOf(Tagged("), "{debug}");
    assert!(debug.contains("\"triggering\""), "{debug}");
}

#[test]
fn kylox_casts_from_among_cards_exiled_with_it() {
    let definition = p36_compile("Kylox's Voltstrider",
        "Mana cost: {1}{U}{R}\nType: Artifact — Vehicle\nPower/Toughness: 6/6\nCollect evidence 6: This Vehicle becomes an artifact creature until end of turn.\nWhenever this Vehicle attacks, you may cast an instant or sorcery spell from among cards exiled with it. If that spell would be put into a graveyard, put it on the bottom of its owner's library instead.\nCrew 2");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("source_exiled") || debug.contains("SourceExiled"), "{debug}");
}

#[test]
fn leonardo_graveyard_casts_during_your_turn_enter_with_finality_counters() {
    let definition = p36_compile("Leonardo, Sewer Samurai",
        "Mana cost: {3}{W}\nType: Legendary Creature — Mutant Ninja Turtle Samurai\nPower/Toughness: 3/3\nSneak {2}{W}{W}\nDouble strike\nDuring your turn, you may cast creature spells with power or toughness 1 or less from your graveyard. If you cast a spell this way, that creature enters with a finality counter on it. (If a creature with a finality counter on it would die, exile it instead.)");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("DuringYourTurns"), "{debug}");
    assert!(debug.contains("Finality"), "{debug}");
}

#[test]
fn kotose_may_play_one_exiled_card_while_you_control_it_with_any_color_mana() {
    let definition = p36_compile("Kotose, the Silent Spider",
        "Mana cost: {3}{U}{B}\nType: Legendary Creature — Human Ninja\nPower/Toughness: 4/4\nWhen Kotose enters, exile target card other than a basic land card from an opponent's graveyard. Search that player's graveyard, hand, and library for any number of cards with the same name as that card and exile them. Then that player shuffles. For as long as you control Kotose, you may play one of the exiled cards, and you may spend mana as though it were mana of any color to cast it.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("AnyColor"), "{debug}");
    assert!(debug.contains("max_plays: Some(1)"), "one of the exiled cards: {debug}");
}

#[test]
fn lara_croft_may_play_one_discovery_card_from_exile_this_turn() {
    let definition = p36_compile("Lara Croft, Tomb Raider",
        "Mana cost: {G}{U}{R}\nType: Legendary Creature — Human Ranger\nPower/Toughness: 3/3\nFirst strike, reach\nWhenever Lara Croft attacks, exile up to one target legendary artifact card or legendary land card from a graveyard and put a discovery counter on it. You may play a card from exile with a discovery counter on it this turn.\nRaid — At end of combat on your turn, if you attacked this turn, create a Treasure token.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("OnceEachTurn"), "one play: {debug}");
    assert!(debug.to_lowercase().contains("discovery"), "{debug}");
}

#[test]
fn keeper_of_the_dead_targets_an_opponent_two_creature_cards_behind_in_the_graveyard() {
    let definition = p36_compile("Keeper of the Dead",
        "Mana cost: {B}{B}\nType: Creature — Human Wizard\nPower/Toughness: 1/2\n{B}, {T}: Choose target opponent who has at least two fewer creature cards in their graveyard than you do as you activate this ability. Destroy target nonblack creature that player controls.");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("OpponentWithMoreControlledObjectsThan"), "{debug}");
    assert!(debug.contains("fewer: true"), "{debug}");
    assert!(debug.contains("as_you_activate: true"), "{debug}");
    assert!(debug.contains("minimum_difference: 2"), "{debug}");
}
