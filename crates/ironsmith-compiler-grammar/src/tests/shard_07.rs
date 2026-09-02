use super::*;
#[cfg(test)]
use ironsmith_compiler::ParseCardText;
#[cfg(test)]
use ironsmith_compiler_lowering::CardDefinitionBuilder;

#[test]
pub(super) fn turn_history_values_compile_for_exact_card_surfaces() {
    let floating_dream = CardDefinitionBuilder::new(CardId::new(), "Floating-Dream Zubera")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Zubera, Subtype::Spirit])
        .parse_text("When this creature dies, draw a card for each Zubera that died this turn.")
        .expect("Floating-Dream Zubera's draw count should use death history");
    let floating_debug = format!("{:#?}", floating_dream.abilities);
    assert!(
        floating_debug.contains("TurnHistoryCount"),
        "{floating_debug}"
    );
    assert!(floating_debug.contains("Died"), "{floating_debug}");
    assert!(floating_debug.contains("Zubera"), "{floating_debug}");

    let fraying = CardDefinitionBuilder::new(CardId::new(), "Fraying Sanity")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura, Subtype::Curse])
        .parse_text(
            "Enchant player\nAt the beginning of each end step, enchanted player mills X cards, where X is the number of cards put into their graveyard from anywhere this turn.",
        )
        .expect("Fraying Sanity's mill count should use enchanted-player graveyard history");
    let fraying_debug = format!("{:#?}", fraying.abilities);
    assert!(
        fraying_debug.contains("PutIntoGraveyard"),
        "{fraying_debug}"
    );
    assert!(
        fraying_debug.contains("TaggedPlayer") && fraying_debug.contains("enchanted"),
        "relative `their` must bind to enchanted player: {fraying_debug}"
    );
    assert!(fraying_debug.contains("from: []"), "{fraying_debug}");

    let surge = CardDefinitionBuilder::new(CardId::new(), "Surge of Brilliance")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Paradox — Draw a card for each spell you've cast this turn from anywhere other than your hand.\nForetell {1}{U}",
        )
        .expect("Surge of Brilliance's draw count should use spell-cast history");
    let surge_debug = format!("{:#?}", surge.spell_effect);
    assert!(surge_debug.contains("TurnHistoryCount"), "{surge_debug}");
    assert!(surge_debug.contains("SpellsCast"), "{surge_debug}");
    assert!(
        surge_debug.contains("from_outside_hand: true"),
        "{surge_debug}"
    );

    let impending = CardDefinitionBuilder::new(CardId::new(), "Impending Flux")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Paradox — Impending Flux deals X damage to each opponent and each creature they control, where X is 1 plus the number of spells you've cast from anywhere other than your hand this turn.\nForetell {1}{R}{R}",
        )
        .expect("Impending Flux's damage count should use spell-cast history");
    let impending_debug = format!("{:#?}", impending.spell_effect);
    assert!(
        impending_debug.contains("TurnHistoryCount"),
        "{impending_debug}"
    );
    assert!(impending_debug.contains("SpellsCast"), "{impending_debug}");
    assert!(
        impending_debug.contains("from_outside_hand: true"),
        "{impending_debug}"
    );
    let impending_compact = impending_debug.split_whitespace().collect::<String>();
    assert!(impending_compact.contains("Fixed(1,"), "{impending_debug}");

    let welcome = CardDefinitionBuilder::new(CardId::new(), "Welcome the Dead")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Draw two cards, then discard a card and you lose 2 life. Create X tapped 2/2 black Zombie Druid creature tokens, where X is the number of cards that were put into your graveyard from your hand or library this turn.\nFlashback {5}{B}",
        )
        .expect("Welcome the Dead's token count should use graveyard history");
    let welcome_debug = format!("{:#?}", welcome.spell_effect);
    assert!(
        welcome_debug.contains("TurnHistoryCount"),
        "{welcome_debug}"
    );
    assert!(
        welcome_debug.contains("PutIntoGraveyard"),
        "{welcome_debug}"
    );
    assert!(welcome_debug.contains("Hand"), "{welcome_debug}");
    assert!(welcome_debug.contains("Library"), "{welcome_debug}");
}

#[test]
pub(super) fn spell_history_floor_cards_compile_to_turn_totals_and_trigger_boundaries() {
    let rionya = CardDefinitionBuilder::new(CardId::new(), "Rionya, Fire Dancer")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Wizard])
        .parse_text(
            "At the beginning of combat on your turn, create X tokens that are copies of another target creature you control, where X is one plus the number of instant and sorcery spells you've cast this turn. They gain haste. Exile them at the beginning of the next end step.",
        )
        .expect("Rionya should compile its token count from the full turn's spell history");
    let rionya_debug = format!("{:#?}", rionya.abilities);
    assert!(rionya_debug.contains("SpellsCast"), "{rionya_debug}");
    assert!(
        rionya_debug.contains("before_triggering_spell: false"),
        "{rionya_debug}"
    );
    let rionya_compact = rionya_debug.split_whitespace().collect::<String>();
    assert!(rionya_compact.contains("Fixed(1,"), "{rionya_debug}");
    assert!(
        rionya_debug.contains("Instant") && rionya_debug.contains("Sorcery"),
        "{rionya_debug}"
    );
    assert!(
        rionya_debug.contains("CreateTokenCopyEffect"),
        "Rionya must create token copies rather than copy a spell: {rionya_debug}"
    );
    assert!(
        rionya_debug.contains("has_haste: true")
            && rionya_debug.contains("exile_at_next_end_step: true"),
        "Rionya's follow-up sentences must remain attached to the created set: {rionya_debug}"
    );
    assert!(
        !rionya_debug.contains("CopySpellEffect"),
        "a battlefield creature is not a spell on the stack: {rionya_debug}"
    );

    let thunder = CardDefinitionBuilder::new(CardId::new(), "Thunder Salvo")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Thunder Salvo deals X damage to target creature, where X is 2 plus the number of other spells you've cast this turn.",
        )
        .expect("Thunder Salvo should compile its damage from the full turn's other spells");
    let thunder_debug = format!("{:#?}", thunder.spell_effect);
    assert!(thunder_debug.contains("SpellsCast"), "{thunder_debug}");
    assert!(
        thunder_debug.contains("before_triggering_spell: false"),
        "{thunder_debug}"
    );
    assert!(
        thunder_debug.contains("exclude_source: true"),
        "{thunder_debug}"
    );
    let thunder_compact = thunder_debug.split_whitespace().collect::<String>();
    assert!(thunder_compact.contains("Fixed(2,"), "{thunder_debug}");

    let sentinel = CardDefinitionBuilder::new(CardId::new(), "Sentinel Tower")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "Whenever an instant or sorcery spell is cast during your turn, this artifact deals damage to any target equal to 1 plus the number of instant and sorcery spells cast before that spell this turn.",
        )
        .expect("Sentinel Tower should compile its passive cast trigger and history boundary");
    let sentinel_debug = format!("{:#?}", sentinel.abilities);
    assert!(sentinel_debug.contains("SpellCast"), "{sentinel_debug}");
    assert!(
        sentinel_debug.contains("Instant") && sentinel_debug.contains("Sorcery"),
        "{sentinel_debug}"
    );
    assert!(
        sentinel_debug.contains("before_triggering_spell: true"),
        "{sentinel_debug}"
    );
    assert!(sentinel_debug.contains("player: Any"), "{sentinel_debug}");
    let sentinel_compact = sentinel_debug.split_whitespace().collect::<String>();
    assert!(sentinel_compact.contains("Fixed(1,"), "{sentinel_debug}");

    let thousand_year = CardDefinitionBuilder::new(CardId::new(), "Thousand-Year Storm")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Whenever you cast an instant or sorcery spell, copy it for each other instant and sorcery spell you've cast before it this turn. You may choose new targets for the copies.",
        )
        .expect("Thousand-Year Storm should compile its copy count against the triggering cast");
    let thousand_year_debug = format!("{:#?}", thousand_year.abilities);
    assert!(
        thousand_year_debug.contains("SpellCast"),
        "{thousand_year_debug}"
    );
    assert!(
        thousand_year_debug.contains("before_triggering_spell: true"),
        "{thousand_year_debug}"
    );
    assert!(
        thousand_year_debug.contains("player: You"),
        "{thousand_year_debug}"
    );
    assert!(
        thousand_year_debug.contains("CopySpell") && thousand_year_debug.contains("Retarget"),
        "{thousand_year_debug}"
    );
}

#[test]
pub(super) fn explicit_your_hand_discard_is_not_rebound_to_the_damaged_player() {
    let fateful = CardDefinitionBuilder::new(CardId::new(), "Fateful Showdown")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Fateful Showdown deals damage to any target equal to the number of cards in your hand. Discard all the cards in your hand, then draw that many cards.",
        )
        .expect("Fateful Showdown should keep the explicit hand owner as the discard actor");
    let debug = format!("{:#?}", fateful.spell_effect);
    let discard = debug
        .split("DiscardEffect")
        .nth(1)
        .expect("compiled program should contain a discard effect");
    assert!(discard.contains("player: You"), "{debug}");
    assert!(!discard.contains("player: DamagedPlayer"), "{debug}");
}

#[test]
pub(super) fn token_copy_followups_cross_source_sentence_boundaries_inside_loops() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Looped Token Followups")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Choose a creature type. For each creature you control of the chosen type, create a token that's a copy of that creature. Those tokens gain haste. Exile them at the beginning of the next end step.",
        )
        .expect("token-copy followups should bind through the source-sentence wrapper");
    let debug = format!("{:#?}", definition.spell_effect);

    assert!(debug.contains("CreateTokenCopyEffect"), "{debug}");
    assert!(debug.contains("has_haste: true"), "{debug}");
    assert!(debug.contains("exile_at_next_end_step: true"), "{debug}");
    assert!(
        !debug.contains("ApplyContinuousEffect") && !debug.contains("ScheduleDelayedTriggerEffect"),
        "the followups must stay attached to the created tokens rather than becoming broad effects: {debug}"
    );
}

#[test]
pub(super) fn created_token_temporary_haste_remains_a_duration_scoped_effect() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Temporary Token Haste")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Create two 1/1 red Goblin creature tokens. Those tokens gain haste until end of turn.",
        )
        .expect("temporary token haste should compile as a follow-up effect");
    let debug = format!("{:#?}", definition.spell_effect);
    let compact = debug.split_whitespace().collect::<String>();

    assert!(compact.contains("CreateTokenEffect"), "{debug}");
    assert!(compact.contains("count:Fixed(2,)"), "{debug}");
    assert!(compact.contains("ApplyContinuousEffect"), "{debug}");
    assert!(compact.contains("until:EndOfTurn"), "{debug}");
    assert!(
        !compact.contains("ability_presentation:Some(Standalone"),
        "duration-scoped haste must not become a permanent token-definition ability: {debug}"
    );
}

#[test]
pub(super) fn unqualified_target_base_characteristics_are_indefinite() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Indefinite Base Characteristics")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Any number of target Shapeshifter creatures you control have base power and toughness 4/4.",
        )
        .expect("an unqualified base-characteristic effect should compile");
    let debug = format!("{:#?}", definition.spell_effect);

    assert!(debug.contains("SetPowerToughness"), "{debug}");
    assert!(
        debug.contains("until: Forever"),
        "a missing duration means the continuous effect is indefinite: {debug}"
    );
    assert!(!debug.contains("until: EndOfTurn"), "{debug}");
}

#[test]
pub(super) fn remove_a_counter_is_mandatory_when_one_is_available() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Exact Counter Removal")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Remove a counter from target permanent.")
        .expect("untyped counter removal should compile");
    let debug = format!("{:#?}", definition.spell_effect);

    assert!(debug.contains("RemoveUpToAnyCountersEffect"), "{debug}");
    assert!(debug.contains("up_to: false"), "{debug}");
}
