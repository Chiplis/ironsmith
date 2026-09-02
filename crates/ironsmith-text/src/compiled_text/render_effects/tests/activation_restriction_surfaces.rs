use super::*;
use ironsmith_compiler::ParseCardText;

fn compile_with_types(
    text: &str,
    card_types: Vec<CardType>,
    subtypes: Vec<Subtype>,
) -> crate::cards::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Activation Restriction Probe")
        .card_types(card_types)
        .subtypes(subtypes)
        .parse_text(text)
        .expect("the restricted activated ability should compile")
}

fn render_with_types(text: &str, card_types: Vec<CardType>, subtypes: Vec<Subtype>) -> String {
    let definition = compile_with_types(text, card_types, subtypes);
    crate::compiled_text::compiled_text_lines(&definition).join("\n")
}

fn render_activated(text: &str, subtypes: Vec<Subtype>) -> String {
    render_with_types(text, vec![CardType::Artifact], subtypes)
}

#[test]
fn mana_ability_preserves_trailing_once_per_turn_restriction() {
    let text = "{1}: Add one mana of any color. Activate only once each turn.";
    assert_eq!(render_activated(text, vec![]), text);
}

#[test]
fn equip_preserves_trailing_once_per_turn_in_addition_to_implied_sorcery_timing() {
    let text = "Equip {0}. Activate only once each turn.";
    assert_eq!(
        render_activated(text, vec![Subtype::Equipment]),
        "Equip {0}. Activate only once each turn"
    );
}

#[test]
fn additional_discard_cost_keeps_the_sentence_leading_discard_count() {
    let text = "As an additional cost to cast this spell, discard a card.\nDraw two cards.";
    assert_eq!(
        render_with_types(text, vec![CardType::Instant], vec![]),
        text
    );
}

#[test]
fn scaled_unless_payment_renders_the_multiplier_as_mana_per_prior_result() {
    let text = "Discard any number of cards. Counter target spell unless its controller pays {3} for each card discarded this way.";
    assert_eq!(
        render_with_types(text, vec![CardType::Instant], vec![]),
        "You discard any number of cards. Counter target spell unless its controller pays {3} for each card discarded this way."
    );
}

#[test]
fn heterogeneous_additional_cost_actions_remain_independent_costs() {
    let text = "As an additional cost to cast this spell, discard a card and sacrifice a creature.\nTwo target creatures each get -13/-13 until end of turn.";
    assert_eq!(
        render_with_types(text, vec![CardType::Sorcery], vec![]),
        text
    );
}

#[test]
fn attacks_while_saddled_keeps_the_bare_typed_state_predicate() {
    let text =
        "Whenever this creature attacks while saddled, it gets +2/+2 until end of turn.\nSaddle 1.";
    assert_eq!(
        render_with_types(text, vec![CardType::Creature], vec![Subtype::Mount]),
        text
    );
}

#[test]
fn waterbend_preserves_the_alternative_total_cost_during_lowering() {
    let text = "Waterbend {3}: Sacrifice this artifact. If you do, scry 2.";
    assert_eq!(render_activated(text, vec![]), text);
}

#[test]
fn ward_waterbend_materializes_and_preserves_its_keyword_cost() {
    let text = "Ward—Waterbend {4}.";
    let definition = compile_with_types(text, vec![CardType::Creature], vec![]);
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition),
        [text]
    );
    let debug = format!("{definition:#?}");
    assert!(debug.contains("Ward"), "{debug}");
    assert!(debug.contains("OneOf"), "{debug}");
}

#[test]
fn morph_preserves_a_creature_subtype_discard_cost() {
    let text = "Morph—Discard a Zombie card.";
    let definition = compile_with_types(text, vec![CardType::Creature], vec![Subtype::Zombie]);
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition),
        [text]
    );
    let debug = format!("{definition:#?}");
    assert!(debug.contains("subtypes: ["), "{debug}");
    assert!(debug.contains("Zombie"), "{debug}");
}

#[test]
fn instant_timing_payment_until_end_of_turn_is_a_repeatable_special_action() {
    let text = "Prevent the next X damage that would be dealt to any target this turn. Until end of turn, you may pay {1} any time you could cast an instant. If you do, prevent the next 1 damage that would be dealt to that permanent or player this turn.";
    let definition = compile_with_types(text, vec![CardType::Instant], vec![]);

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join(" "),
        text
    );
    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("GrantRepeatableManaPaymentActionUntilEndOfTurnEffect"),
        "{debug}"
    );
    assert!(!debug.contains("MayEffect"), "{debug}");
    assert!(!debug.contains("IfEffect"), "{debug}");
}

#[test]
fn coordinated_target_player_resources_share_one_activated_ability_target() {
    let text =
        "{4}, {T}: Target opponent loses 2 life, gets a poison counter, then mills six cards.";
    let definition = compile_with_types(text, vec![CardType::Artifact], vec![]);
    let debug = format!("{definition:#?}");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition),
        [text],
        "{debug}"
    );
    assert!(debug.contains("LoseLifeEffect"), "{debug}");
    assert!(debug.contains("PoisonCountersEffect"), "{debug}");
    assert!(debug.contains("MillEffect"), "{debug}");
    assert_eq!(debug.matches("TargetOnlyEffect").count(), 1, "{debug}");
}

#[test]
fn exert_reflexive_trigger_owns_its_target_choice() {
    let text = "You may exert this creature as it attacks. When you do, target creature can't block this turn.";
    assert_eq!(
        render_with_types(text, vec![CardType::Creature], vec![]),
        text
    );
}

#[test]
fn enter_as_copy_granted_ability_owns_its_target_choice() {
    let text = "You may have this creature enter as a copy of any creature on the battlefield, except it has \"{U}{B}, {T}: Destroy target creature with the same name as this creature.\"";
    let rendered = render_with_types(text, vec![CardType::Creature], vec![]);
    assert!(
        rendered.contains("Destroy target creature with the same name as this creature"),
        "targeted granted ability was lost during nested lowering: {rendered}"
    );
}

#[test]
fn conditional_zone_rewrite_preserves_instead_surface() {
    let text = "Search your library for a basic land card, reveal it, put it into your hand, then shuffle.\nYou may put that card onto the battlefield instead of putting it into your hand if a creature died this turn.";
    assert_eq!(
        render_with_types(text, vec![CardType::Sorcery], vec![]),
        text
    );
}

#[test]
fn bargained_local_zone_rewrite_preserves_instead_surface() {
    let text = "Bargain (You may sacrifice an artifact, enchantment, or token as you cast this spell.)\nReturn up to two target creature cards from your graveyard to your hand. If this spell was bargained, you may put one of those cards with mana value 4 or less onto the battlefield instead of putting it into your hand.";
    let definition = compile_with_types(text, vec![CardType::Sorcery], vec![]);
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        "Bargain.\nReturn up to two target creature cards from your graveyard to your hand. If this spell was bargained, you may put one of those cards with mana value 4 or less onto the battlefield instead of putting it into your hand."
    );
}

#[test]
fn self_replacement_presentation_survives_the_artifact_boundary() {
    let text = "Search your library for a basic land card, reveal it, put it into your hand, then shuffle.\nYou may put that card onto the battlefield instead of putting it into your hand if a creature died this turn.";
    let compiled = ironsmith_compiler::CompilerFacade::new()
        .compile_definition(
            ironsmith_compiler::CardDefinitionBuilder::new(
                crate::ids::CardId::new(),
                "Artifact Replacement Probe",
            )
            .card_types(vec![ironsmith_compiler::types::CardType::Sorcery]),
            text.to_string(),
            ironsmith_compiler::CompilePolicy {
                allow_unsupported: false,
            },
        )
        .expect("the compiler-side replacement should compile");
    let wire = ironsmith_compiled_artifact::wire_definition_from_serializable(&compiled.definition)
        .expect("the replacement should serialize");
    let runtime = ironsmith_runtime_catalog::artifact_materializer::materialize_definition(wire)
        .expect("the replacement should materialize");
    let branch = runtime
        .spell_effect
        .as_ref()
        .and_then(|program| program.segments.first())
        .and_then(|segment| segment.self_replacements.first())
        .expect("the replacement branch should survive materialization");
    assert!(branch.condition_after_replacement, "{branch:#?}");
    assert!(branch.starts_new_source_line, "{branch:#?}");
}

#[test]
fn kicked_modal_count_override_preserves_instead_surface() {
    let text = "Kicker {2}{U}{U}\nChoose one. If this spell was kicked, choose any number instead.\n• Return up to two target creatures to their owners' hands.\n• Scry 2, then draw two cards.\n• Target player creates an X/X blue Illusion creature token, where X is the number of cards in their hand.";
    assert_eq!(
        render_with_types(text, vec![CardType::Sorcery], vec![]),
        text
    );
}
