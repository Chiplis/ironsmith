//! Semantic-loss fix wave (2026-10-21), package T5B: real-text compile and
//! runtime checks for the behaviour each fix restored.

use super::*;

fn compile(name: &str, card_types: Vec<CardType>, text: &str) -> crate::cards::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name)
        .card_types(card_types)
        .parse_text(text)
        .unwrap_or_else(|err| panic!("{name} should compile: {err:?}"))
}

fn debug(definition: &crate::cards::CardDefinition) -> String {
    format!("{definition:?}")
}

fn text(definition: &crate::cards::CardDefinition) -> String {
    crate::compiled_text::compiled_text_lines(definition).join("\n")
}

/// The `if_true` mode choice of the conditional "choose ... instead" header.
fn conditional_mode_choice(definition: &crate::cards::CardDefinition) -> (Value, Value) {
    let program = definition.spell_effect.as_ref().expect("spell program");
    let conditional = program
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ConditionalEffect>())
        .expect("conditional mode header")
        .clone();
    let choose = conditional.if_true[0]
        .downcast_ref::<crate::effects::ChooseModeEffect>()
        .expect("choose-mode branch");
    (choose.min_choose_count.clone(), choose.choose_count.clone())
}

// --- Modal "choose both instead": mandatory vs. "you may" ---

const MANDATORY_BOTH: &str = "Choose one. If you control three or more creatures, choose both instead.\n• Draw a card.\n• You gain 3 life.";
const OPTIONAL_BOTH: &str = "Choose one. If you control three or more creatures, you may choose both instead.\n• Draw a card.\n• You gain 3 life.";

#[test]
fn mandatory_choose_both_instead_requires_both_modes() {
    let definition = compile("Mandatory Both", vec![CardType::Sorcery], MANDATORY_BOTH);
    assert_eq!(
        conditional_mode_choice(&definition),
        (Value::Fixed(2), Value::Fixed(2))
    );
}

#[test]
fn may_choose_both_instead_keeps_one_or_two_modes() {
    let definition = compile("Optional Both", vec![CardType::Sorcery], OPTIONAL_BOTH);
    assert_eq!(
        conditional_mode_choice(&definition),
        (Value::Fixed(1), Value::Fixed(2))
    );
    assert!(text(&definition).contains("you may choose both instead"), "{}", text(&definition));
}

#[test]
fn triggered_choose_both_has_no_cast_time_qualifier() {
    let definition = compile(
        "Combat Program",
        vec![CardType::Enchantment],
        "At the beginning of combat on your turn, choose one. If you control a commander, you may choose both instead.\n• Draw a card.\n• You gain 3 life.",
    );
    let rendered = text(&definition);
    assert!(rendered.contains("If you control a commander, you may choose both instead"), "{rendered}");
    assert!(!rendered.contains("as you cast this spell"), "{rendered}");
}

// --- "Whenever this creature attacks a battle" ---

#[test]
fn attacks_a_battle_keeps_the_battle_restriction() {
    let definition = compile(
        "Battle Attacker",
        vec![CardType::Creature],
        "Whenever this creature attacks a battle, it gets +1/+1 until end of turn.",
    );
    let debug = debug(&definition);
    assert!(debug.contains("AttacksTrigger"), "{debug}");
    assert!(debug.contains("attacking_battle: true"), "{debug}");
    assert!(text(&definition).contains("Whenever this creature attacks a battle"));
}

// --- "each other player's draw step" ---

#[test]
fn each_other_players_draw_step_excludes_you() {
    let definition = compile(
        "Idea Well",
        vec![CardType::Enchantment],
        "At the beginning of each other player's draw step, that player draws an additional card.",
    );
    let debug = debug(&definition);
    assert!(debug.contains("BeginningOfDrawStepTrigger {\n") || debug.contains("BeginningOfDrawStepTrigger {"));
    assert!(debug.contains("player: NotYou"), "{debug}");
    assert!(text(&definition).contains("each other player's draw step"));
}

// --- graveyard-origin cast trigger keeps the spell filter ---

#[test]
fn graveyard_cast_trigger_keeps_dragon_creature_filter() {
    let definition = compile(
        "Dragon Graveyard",
        vec![CardType::Creature],
        "Whenever you cast a Dragon creature spell from your graveyard, draw a card.",
    );
    let debug = debug(&definition);
    assert!(debug.contains("SpellCastTrigger"), "{debug}");
    assert!(debug.contains("Dragon"), "{debug}");
    assert!(
        text(&definition).contains("Whenever you cast a Dragon creature spell from your graveyard"),
        "{}",
        text(&definition)
    );
}

// --- "a creature left the battlefield under your control this turn" ---

#[test]
fn creature_left_battlefield_condition_counts_only_creatures() {
    let definition = compile(
        "Rough Buddy",
        vec![CardType::Instant],
        "Put a +1/+1 counter on target creature. Put two +1/+1 counters on that creature instead if a creature left the battlefield under your control this turn.",
    );
    let debug = debug(&definition);
    assert!(debug.contains("MovedZones"), "{debug}");
    assert!(!debug.contains("PermanentLeftBattlefieldUnderYourControlThisTurn"), "{debug}");
    assert!(
        text(&definition).contains("a creature left the battlefield under your control this turn"),
        "{}",
        text(&definition)
    );
}

// --- cost/condition timing ---

#[test]
fn opponent_spell_tax_during_your_turn_is_gated_on_your_turn() {
    let definition = compile(
        "Turn Tax",
        vec![CardType::Enchantment],
        "Spells your opponents cast during your turn cost {1} more to cast.",
    );
    let debug = debug(&definition);
    assert!(debug.contains("CostIncrease"), "{debug}");
    assert!(debug.contains("YourTurn"), "{debug}");
}

#[test]
fn cast_another_white_spell_condition_keeps_color_and_excludes_itself() {
    let definition = compile(
        "White Scorn",
        vec![CardType::Instant],
        "If you've cast another white spell this turn, you may cast this spell without paying its mana cost.\nDestroy all enchantments.",
    );
    let debug = debug(&definition);
    assert!(debug.contains("SpellsCastThisTurnMatching"), "{debug}");
    assert!(debug.contains("exclude_source: true"), "{debug}");
    assert!(!debug.contains("YouCastSpellsThisTurnOrMore"), "{debug}");
    assert!(text(&definition).contains("another white spell"));
}

// --- chosen-color damage source ---

#[test]
fn source_of_your_choice_of_the_chosen_color_keeps_the_color_restriction() {
    let definition = compile(
        "Story Ring",
        vec![CardType::Enchantment],
        "As this enchantment enters, choose a color.\n{W}: The next time a source of your choice of the chosen color would deal damage to you this turn, prevent that damage.",
    );
    let debug = debug(&definition);
    assert!(debug.contains("ChoiceMatching"), "{debug}");
    assert!(debug.contains("chosen_color: true"), "{debug}");
    assert!(
        text(&definition).contains("a source of your choice of the chosen color"),
        "{}",
        text(&definition)
    );
}

// --- different-names search ---

#[test]
fn search_for_cards_that_each_have_different_names_sets_distinct_names() {
    let definition = compile(
        "Dragon Search",
        vec![CardType::Sorcery],
        "Search your library for up to five Dragon cards not named Tiamat that each have different names, reveal them, put them into your hand, then shuffle.",
    );
    let debug = debug(&definition);
    assert!(debug.contains("distinct_names: true"), "{debug}");
    let rendered = text(&definition);
    assert!(rendered.contains("Dragon cards not named Tiamat with different names"), "{rendered}");
}

// --- other parse fixes ---

#[test]
fn opponent_graveyard_from_their_library_trigger_has_library_origin() {
    let definition = compile(
        "Library Alchemist",
        vec![CardType::Creature],
        "Whenever a creature card is put into an opponent's graveyard from their library, draw a card.",
    );
    let debug = debug(&definition);
    assert!(debug.contains("from: Specific(\n") || debug.contains("from: Specific("), "{debug}");
    assert!(debug.contains("Library"), "{debug}");
    assert!(!debug.contains("from: Any"), "{debug}");
}

#[test]
fn enters_or_leaves_keeps_the_subject_on_the_leave_arm() {
    let definition = compile(
        "Artifact Strider",
        vec![CardType::Artifact, CardType::Creature],
        "Whenever another artifact you control enters or leaves the battlefield, you gain 1 life.",
    );
    let rendered = text(&definition);
    assert!(
        rendered.contains("or another artifact you control leaves the battlefield"),
        "{rendered}"
    );
}

#[test]
fn spells_theyve_cast_this_turn_counts_cast_history() {
    let definition = compile(
        "Smothering Rug",
        vec![CardType::Artifact],
        "Whenever a player casts a spell, they lose 1 life for each spell they've cast this turn.",
    );
    // Cast history of the triggering caster, not a count of spells on the
    // stack (the pre-fix `Count(ObjectFilter { zone: Stack, .. })`).
    let debug = debug(&definition);
    assert!(
        debug.contains("SpellsCast { player: IteratedPlayer")
            || debug.contains("SpellsCastThisTurn(IteratedPlayer)"),
        "{debug}"
    );
    assert!(!debug.contains("value: Count("), "{debug}");
}

#[test]
fn loses_all_abilities_and_cant_attack_or_block_keeps_the_restriction() {
    let definition = compile(
        "Stasis Aura",
        vec![CardType::Enchantment],
        "Enchant creature\nEnchanted creature loses all abilities and can't attack or block.",
    );
    assert!(
        text(&definition).contains("Enchanted creature loses all abilities and can't attack or block"),
        "{}",
        text(&definition)
    );
}

#[test]
fn additional_combat_rider_is_a_one_shot_delayed_trigger() {
    let definition = compile(
        "Fury Landfall",
        vec![CardType::Creature],
        "Landfall — Whenever a land you control enters, if it's your main phase, there's an additional combat phase after this phase. At the beginning of that combat, untap all creatures you control.",
    );
    let triggered = definition
        .abilities
        .iter()
        .filter(|ability| matches!(ability.kind, crate::ability::AbilityKind::Triggered(_)))
        .count();
    assert_eq!(triggered, 1, "{}", text(&definition));
    let debug = debug(&definition);
    assert!(debug.contains("ScheduleDelayedTriggerEffect"), "{debug}");
    assert!(text(&definition).contains("At the beginning of that combat"));
}
