//! Claude package 76 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/claude2/BRIEF.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

fn p76_debug(name: &str, text: &str) -> String {
    let definition = compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|error| panic!("{name} should compile: {error:?}"));
    format!("{definition:?}")
}

/// The text from `start` (first occurrence of `marker`) of at most `len` bytes.
fn p76_window<'a>(debug: &'a str, marker: &str, len: usize) -> &'a str {
    let start = debug
        .find(marker)
        .unwrap_or_else(|| panic!("missing {marker}: {debug}"));
    let mut end = (start + len).min(debug.len());
    while !debug.is_char_boundary(end) {
        end -= 1;
    }
    &debug[start..end]
}

#[test]
fn p76_doc_aurlock_reduces_spells_cast_from_graveyard_or_exile() {
    let debug = p76_debug(
        "Doc Aurlock, Grizzled Genius",
        "Mana cost: {G}{U}\nType: Legendary Creature — Bear Druid\nPower/Toughness: 2/3\nSpells you cast from your graveyard or from exile cost {2} less to cast.\nPlotting cards from your hand costs {2} less.",
    );
    let reduction = p76_window(&debug, "CostReduction(CostReduction", 12000);
    assert!(
        reduction.contains("zone: Some(Graveyard)") && reduction.contains("zone: Some(Exile)"),
        "both cast origins reduce the spell: {reduction}"
    );
}

#[test]
fn p76_done_for_the_day_intervening_if_keeps_the_whole_control_list() {
    let debug = p76_debug(
        "Done for the Day",
        "Mana cost: {2}{G}\nType: Enchantment\nAt the beginning of your end step, if you control an Employee, a Performer, or a Robot, you may get {TK} or create a Treasure token. If you control all three, you may put a sticker on a nonland permanent you own.",
    );
    let condition = p76_window(&debug, "intervening_if: Some(", 8000);
    for subtype in ["Employee", "Performer", "Robot"] {
        assert!(condition.contains(subtype), "{subtype} satisfies the intervening if: {condition}");
    }
}

#[test]
fn p76_depala_puts_the_unchosen_revealed_cards_on_the_bottom() {
    let debug = p76_debug(
        "Depala, Pilot Exemplar",
        "Mana cost: {1}{R}{W}\nType: Legendary Creature — Dwarf Pilot\nPower/Toughness: 3/3\nOther Dwarves you control get +1/+1.\nEach Vehicle you control gets +1/+1 as long as it's a creature.\nWhenever Depala becomes tapped, you may pay {X}. If you do, reveal the top X cards of your library, put all Dwarf and Vehicle cards from among them into your hand, then put the rest on the bottom of your library in a random order.",
    );
    assert!(debug.contains("Dwarf") && debug.contains("Vehicle"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnLibraryBottomEffect"),
        "the rest goes to the bottom of the library: {debug}"
    );
}

#[test]
fn p76_emissary_retargets_only_a_matching_stack_object() {
    let debug = p76_debug(
        "Emissary of Grudges",
        "Mana cost: {5}{R}\nType: Creature — Efreet\nPower/Toughness: 6/5\nFlying, haste\nAs this creature enters, secretly choose an opponent.\nReveal the player you chose: Choose new targets for target spell or ability if it's controlled by the chosen player and if it targets you or a permanent you control. Activate only once.",
    );
    assert!(debug.contains("RetargetStackObjectEffect"), "the ability retargets: {debug}");
    assert!(
        !debug.contains("SourceMatches("),
        "the trailing if qualifies the target, not this creature: {debug}"
    );
}

#[test]
fn p76_everything_pizza_each_opponent_discards() {
    let debug = p76_debug(
        "Everything Pizza",
        "Mana cost: {2}\nType: Artifact — Food\nWhen this artifact enters, search your library for a basic land card, reveal it, put it into your hand, then shuffle.\n{2}{W}{U}{B}{R}{G}, {T}, Sacrifice this artifact: Target player gains 3 life and draws a card. Each of your opponents discards a card. This artifact deals 3 damage to any target. Put three +1/+1 counters on up to one target creature.",
    );
    let discard = p76_window(&debug, "DiscardEffect", 400);
    assert!(discard.contains("player: IteratedPlayer"), "every opponent discards: {discard}");
}

#[test]
fn p76_eye_of_singularity_destroys_same_named_permanents_and_sees_nonbasic_lands() {
    let debug = p76_debug(
        "Eye of Singularity",
        "Mana cost: {3}{W}\nType: World Enchantment\nWhen this enchantment enters, destroy each permanent with the same name as another permanent, except for basic lands. They can't be regenerated.\nWhenever a permanent other than a basic land enters, destroy all other permanents with that name. They can't be regenerated.",
    );
    assert!(
        debug.contains("SameNameAsTagged") && debug.contains("IsNotTaggedObject"),
        "only the other permanents sharing the entering permanent's name: {debug}"
    );
    assert!(
        !debug.contains("excluded_card_types: [Land]"),
        "a nonbasic land entering still triggers: {debug}"
    );
}

#[test]
fn p76_gimli_third_resolution_gate_reports_its_match() {
    let debug = p76_debug(
        "Gimli, Mournful Avenger",
        "Mana cost: {1}{R}{G}\nType: Legendary Creature — Dwarf Warrior\nPower/Toughness: 3/2\nGimli has indestructible as long as two or more creatures died under your control this turn.\nWhenever another creature you control dies, put a +1/+1 counter on Gimli. When this ability resolves for the third time this turn, Gimli fights up to one target creature you don't control.",
    );
    let gate = p76_window(&debug, "ThisAbilityResolvedThisTurnExactly", 600);
    assert!(
        gate.contains("capture_condition_result: true"),
        "the reflexive fight reads whether the third-resolution gate matched: {gate}"
    );
}

#[test]
fn p76_delina_copy_exiles_itself_at_end_of_combat() {
    let debug = p76_debug(
        "Delina, Wild Mage",
        r#"Mana cost: {3}{R}
Type: Legendary Creature — Elf Shaman
Power/Toughness: 3/2
Whenever Delina attacks, choose target creature you control, then roll a d20.
1—14 | Create a tapped and attacking token that's a copy of that creature, except it's not legendary and it has "At end of combat, exile this token."
15—20 | Create one of those tokens. You may roll again."#,
    );
    assert!(
        debug.contains("exile_at_end_of_combat: true"),
        "the quoted end-of-combat exile is kept: {debug}"
    );
}

#[test]
fn p76_etrata_counts_every_owned_exiled_hit_card() {
    let debug = p76_debug(
        "Etrata, the Silencer",
        "Mana cost: {2}{U}{B}\nType: Legendary Creature — Vampire Assassin\nPower/Toughness: 3/5\nEtrata can't be blocked.\nWhenever Etrata deals combat damage to a player, exile target creature that player controls and put a hit counter on that card. That player loses the game if they own three or more exiled cards with hit counters on them. Etrata's owner shuffles Etrata into their library.",
    );
    let condition = p76_window(&debug, "PlayerHasAtLeast", 9000);
    let condition = &condition[..condition.find("LoseTheGameEffect").unwrap_or(condition.len())];
    assert!(condition.contains("Hit"), "{condition}");
    assert!(
        !condition.contains("__source_exiled__"),
        "hit cards exiled by an earlier Etrata object still count: {condition}"
    );
}

#[test]
fn p76_extrapolate_opponent_chooses_from_the_outside_game_pool() {
    let debug = p76_debug(
        "Extrapolate the Impossible",
        "Mana cost: {1}{B}\nType: Sorcery\nYou may reveal exactly two cards you own with different names from outside the game. An opponent chooses one of them. You put that card into your hand.",
    );
    let choice = p76_window(&debug, "ChoosePlayerEffect", 20000);
    let choice = &choice[..choice.find("MoveToZoneEffect").unwrap_or(choice.len())];
    assert!(
        choice.contains("OutsideGame"),
        "the opponent picks among the revealed outside-game cards: {choice}"
    );
}

#[test]
fn p76_frantic_firebolt_counts_adventure_cards_in_your_graveyard() {
    let debug = p76_debug(
        "Frantic Firebolt",
        "Mana cost: {2}{R}\nType: Instant\nFrantic Firebolt deals X damage to target creature, where X is 2 plus the number of cards in your graveyard that are instant cards, sorcery cards, and/or have an Adventure.",
    );
    let adventure = debug.find("Adventure]").expect("adventure branch");
    let branch_start = debug[..adventure]
        .rfind("ObjectFilter { zone:")
        .expect("adventure branch filter");
    let branch = &debug[branch_start..adventure];
    assert!(
        branch.starts_with("ObjectFilter { zone: Some(Graveyard)"),
        "the Adventure branch counts graveyard cards: {branch}"
    );
}

#[test]
fn p76_enlightened_confidant_returns_the_surveilled_card_not_itself() {
    let debug = p76_debug(
        "Enlightened Confidant",
        "Mana cost: {1}{W}\nType: Creature — Kor Cleric\nPower/Toughness: 2/1\nLifelink\nAt the beginning of your end step, if you gained life this turn, surveil 1. If you put a card with mana value less than or equal to the amount of life you gained this turn into your graveyard this way, put that card into your hand.",
    );
    let moved = p76_window(&debug, "MoveToZoneEffect", 300);
    assert!(
        moved.contains("__surveil_put_into_graveyard__") && !moved.contains("target: Source"),
        "the card the surveil put into the graveyard returns: {moved}"
    );
}

#[test]
fn p76_guardian_archon_protects_you_and_one_target_permanent() {
    let debug = p76_debug(
        "Guardian Archon",
        "Mana cost: {4}{W}{W}\nType: Creature — Archon\nPower/Toughness: 5/5\nFlying\nAs this creature enters, secretly choose an opponent.\nReveal the player you chose: You and target permanent you control each gain protection from the chosen player until end of turn. Activate only once.",
    );
    assert!(
        !debug.contains("target: Filter("),
        "only the one target permanent gains protection, not every permanent: {debug}"
    );
}

#[test]
fn p76_giant_oyster_counters_go_on_the_target_and_cleanup_is_delayed() {
    let debug = p76_debug(
        "Giant Oyster",
        "Mana cost: {2}{U}{U}\nType: Creature — Oyster\nPower/Toughness: 0/3\nYou may choose not to untap this creature during your untap step.\n{T}: For as long as this creature remains tapped, target tapped creature doesn't untap during its controller's untap step, and at the beginning of each of your draw steps, put a -1/-1 counter on that creature. When this creature leaves the battlefield or becomes untapped, remove all -1/-1 counters from the creature.",
    );
    let draw_step = p76_window(&debug, "BeginningOfDrawStepTrigger", 1500);
    let draw_step = &draw_step[..draw_step.find("duration:").unwrap_or(draw_step.len())];
    assert!(
        draw_step.contains("targeted_0") && !draw_step.contains("target: Source"),
        "each draw step's counter goes on the targeted creature: {draw_step}"
    );
    assert!(
        debug.matches("ScheduleDelayedTriggerEffect").count() >= 2,
        "the leaves/untaps cleanup is its own delayed trigger: {debug}"
    );
}

#[test]
fn p76_dog_umbra_otherwise_branch_is_this_auras_conditional_umbra_armor() {
    let debug = p76_debug(
        "Dog Umbra",
        "Mana cost: {1}{W}\nType: Enchantment — Aura\nFlash\nEnchant creature\nAs long as another player controls enchanted creature, it can't attack or block. Otherwise, this Aura has umbra armor.",
    );
    assert!(debug.contains("UmbraArmor"), "{debug}");
    assert!(
        !debug.contains("GrantObjectAbilityForFilter"),
        "umbra armor belongs to this Aura, not to every Aura: {debug}"
    );
    assert!(
        debug.matches("Not(").count() >= 2,
        "the restriction and the umbra armor are gated by opposite conditions: {debug}"
    );
}

#[test]
fn p76_galactus_keeps_the_other_cards_literal_name() {
    let debug = p76_debug(
        "Galactus, Devourer of Worlds",
        "Mana cost: {10}\nType: Legendary Creature — Elder Alien\nPower/Toughness: 12/12\nFlying, trample, indestructible\nWhen Galactus enters, exile target permanent.\nInsatiable Hunger — Galactus attacks an opponent with the most life among your opponents each combat if able unless you control a creature named Silver Surfer, Galactus's Herald.",
    );
    assert!(
        !debug.contains("this creatures herald") && !debug.contains("this creature's herald"),
        "the named card is Silver Surfer, Galactus's Herald: {debug}"
    );
}
