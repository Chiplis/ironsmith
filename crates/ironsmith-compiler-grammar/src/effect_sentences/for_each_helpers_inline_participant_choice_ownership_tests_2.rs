use crate::cards::builders::ForEachEffectAst;
use super::*;
use crate::lexer::lex_line;

fn parsed_debug(text: &str) -> String {
    let tokens = lex_line(text, 0).expect("participant choice should lex");
    let effect = if text.starts_with("Each opponent") || text.starts_with("For each opponent") {
        parse_for_each_opponent_clause(&tokens)
    } else {
        parse_for_each_player_clause(&tokens)
    }
    .expect("participant choice should parse")
    .expect("participant choice shape should match");
    format!("{effect:#?}")
}

#[test]
fn participant_subject_owns_choice_but_for_each_imperative_does_not() {
    let each_opponent =
        parsed_debug("Each opponent chooses a creature they control and sacrifices it.");
    assert!(each_opponent.contains("player: That"), "{each_opponent}");
    assert!(!each_opponent.contains("player: You"), "{each_opponent}");

    let each_player = parsed_debug("Each player chooses a creature they control.");
    assert!(each_player.contains("player: That"), "{each_player}");

    let controller = parsed_debug("For each opponent, choose a creature they control.");
    assert!(controller.contains("player: You"), "{controller}");
}

#[test]
fn imperative_for_each_keeps_iterated_player_inside_object_filter() {
    let tokens = lex_line(
            "For each opponent, you create a token that's a copy of up to one target creature that player controls.",
            0,
        )
        .expect("quantified token-copy clause should lex");
    let effect = parse_for_each_opponent_clause(&tokens)
        .expect("quantified token-copy clause should parse")
        .expect("quantified token-copy clause should match");
    let EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects }) = effect else {
        panic!("expected opponent iteration, got {effect:#?}");
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopyFromSource { source, player, .. }),
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected one token-copy action, got {effects:#?}");
    };
    let TargetAst::WithCount(source, _) = source else {
        panic!("expected an up-to target, got {source:#?}");
    };
    let TargetAst::Object(filter, _, _) = source.as_ref() else {
        panic!("expected an object-copy target, got {source:#?}");
    };
    assert_eq!(filter.controller, Some(PlayerFilter::IteratedPlayer));
    assert_eq!(*player, PlayerAst::You);

    let parsed = crate::effect_sentences::parse_effect_sentences_lexed(&tokens)
        .expect("public effect parser should keep the quantified program");
    let [EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects })] = parsed.as_slice() else {
        panic!("public parser split the quantified program: {parsed:#?}");
    };
    assert_eq!(effects.len(), 1, "{effects:#?}");
}

#[test]
fn source_attacked_player_subject_keeps_runtime_filter() {
    let tokens = lex_line(
        "Each player this creature attacked this turn loses the game.",
        0,
    )
    .expect("source-relative player clause should lex");
    let effect = parse_for_each_player_clause(&tokens)
        .expect("source-relative player clause should parse")
        .expect("source-relative player clause should match");
    let EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered { filter, effects }) = effect else {
        panic!("expected filtered player iteration, got {effect:#?}");
    };
    assert_eq!(filter, PlayerFilter::AttackedBySourceThisTurn);
    assert!(format!("{effects:#?}").contains("LoseGame"), "{effects:#?}");
}

#[test]
fn named_creature_combat_damage_history_keeps_filtered_participant() {
    let tokens = lex_line(
            "Each opponent dealt combat damage this game by a creature named Gollum, Obsessed Stalker loses life equal to the amount of life you gained this turn.",
            0,
        )
        .expect("combat-history participant clause should lex");
    let effect = parse_for_each_opponent_clause(&tokens)
        .expect("combat-history participant clause should parse")
        .expect("combat-history participant clause should match");
    let EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered { filter, effects }) = effect else {
        panic!("expected filtered player iteration, got {effect:#?}");
    };
    assert!(
        matches!(
            filter,
            PlayerFilter::WasDealtCombatDamageBySourcesThisGame { .. }
        ),
        "{filter:#?}"
    );
    let PlayerFilter::WasDealtCombatDamageBySourcesThisGame { sources, .. } = &filter else {
        unreachable!("typed history variant was already asserted")
    };
    assert_eq!(sources.name.as_deref(), Some("gollum obsessed stalker"));
    assert!(format!("{effects:#?}").contains("LoseLife"), "{effects:#?}");
}

#[test]
fn other_players_copying_triggering_spell_exclude_its_controller() {
    let tokens = lex_line(
            "Each other player may copy that spell and may choose new targets for the copy they control.",
            0,
        )
        .expect("triggering-spell fanout should lex");
    let effect = parse_for_each_player_clause(&tokens)
        .expect("triggering-spell fanout should parse")
        .expect("triggering-spell fanout should match");
    let EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered { filter, effects }) = effect else {
        panic!("expected filtered player iteration, got {effect:#?}");
    };
    assert_eq!(
        filter,
        PlayerFilter::excluding(
            PlayerFilter::Any,
            PlayerFilter::AliasedControllerOf(ObjectRef::tagged("triggering")),
        )
    );
    assert!(matches!(effects.as_slice(), [EffectAst::Permissions(PermissionEffectAst::May { .. })]));
}

#[test]
fn standalone_participant_choices_use_an_aggregate_tag_but_nested_choices_remain_local() {
    let standalone = parsed_debug("Each player chooses a creature they control.");
    assert!(
        standalone.contains("participant_choice_l0_s"),
        "{standalone}"
    );
    assert!(!standalone.contains("\"__it__\""), "{standalone}");

    let nested = parsed_debug("Each opponent chooses a creature they control and sacrifices it.");
    assert!(
        !nested.contains("participant_choice_l0_s"),
        "an immediate per-participant consumer must not share choices across iterations: \
            {nested}"
    );
}

#[test]
fn participant_creature_type_choice_is_not_claimed_as_an_object_choice() {
    let standalone = parsed_debug("Each player chooses a creature type.");
    assert!(standalone.contains("ChooseCreatureType"), "{standalone}");
    assert!(!standalone.contains("ChooseObjects"), "{standalone}");

    let text = "Each player chooses a creature type and returns any number of cards of that type from their graveyard to their hand.";
    let effect = parsed_debug(text);
    assert!(effect.contains("ChooseCreatureType"), "{effect}");
    assert!(!effect.contains("ChooseObjects"), "{effect}");
    assert!(effect.contains("chosen_creature_type: true"), "{effect}");

    let object_choice = parsed_debug(
        "Each player chooses a creature they control and returns it to its owner's hand.",
    );
    assert!(object_choice.contains("ChooseObjects"), "{object_choice}");
    assert!(
        !object_choice.contains("ChooseCreatureType"),
        "{object_choice}"
    );
}

#[test]
fn participant_graveyard_choice_keeps_the_remainder_in_the_same_loop() {
    let tokens = lex_line(
        "Each opponent chooses two cards in their graveyard and exiles the rest.",
        0,
    )
    .expect("participant graveyard choice should lex");
    let effect = parse_for_each_opponent_clause(&tokens)
        .expect("participant graveyard choice should parse")
        .expect("participant graveyard choice should match");
    let EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects }) = effect else {
        panic!("expected opponent loop: {effect:#?}");
    };
    assert!(
        format!("{effects:#?}").contains("ChooseObjects"),
        "{effects:#?}"
    );
    assert!(format!("{effects:#?}").contains("Exile"), "{effects:#?}");
    assert_eq!(
        effects.len(),
        2,
        "choice and remainder must be adjacent: {effects:#?}"
    );
}

#[test]
fn for_each_object_filter_preserves_typed_those_set_surface() {
    let those_tokens = lex_line("those permanents", 0).expect("those filter should lex");
    let those = parse_for_each_object_filter(&those_tokens).expect("those filter should parse");
    assert_eq!(
        those.set_quantifier_surface(),
        Some(ironsmith_core::SetQuantifierSurface::Those)
    );

    let ordinary_tokens =
        lex_line("permanent destroyed this way", 0).expect("ordinary filter should lex");
    let ordinary =
        parse_for_each_object_filter(&ordinary_tokens).expect("ordinary filter should parse");
    assert_eq!(ordinary.set_quantifier_surface(), None);
}

#[test]
fn for_each_object_filter_preserves_owned_exile_counter_scope() {
    let tokens = lex_line(
        "creature card you own in exile with a memory counter on it",
        0,
    )
    .expect("owned exile filter should lex");
    let filter = parse_for_each_object_filter(&tokens).expect("owned exile filter should parse");

    assert_eq!(filter.zone, Some(crate::zone::Zone::Exile));
    assert_eq!(filter.owner, Some(PlayerFilter::You));
    assert_eq!(filter.card_types, [crate::types::CardType::Creature]);
    assert_eq!(
        filter.with_counter,
        Some(crate::filter::CounterConstraint::Typed(
            crate::object::CounterType::Named("memory".into())
        ))
    );
    assert!(filter.has_owner_before_zone_surface());
    assert!(filter.has_counter_requirement_after_zone_surface());
    assert_eq!(
        filter.description(),
        "a creature card you own in exile with a memory counter on it"
    );
}

#[test]
fn for_each_object_filter_restores_only_the_exact_coordinated_stack_domain() {
    let coordinated = lex_line("spell and ability your opponents control", 0)
        .expect("coordinated Stack filter should lex");
    let coordinated =
        parse_for_each_object_filter(&coordinated).expect("coordinated Stack filter should parse");
    assert_eq!(coordinated.zone, Some(crate::zone::Zone::Stack));
    assert_eq!(
        coordinated.stack_kind,
        Some(crate::filter::StackObjectKind::SpellOrAbility)
    );
    assert!(!coordinated.has_mana_cost);
    assert!(coordinated.has_conjunctive_set_surface());

    let ordinary =
        lex_line("spell your opponents control", 0).expect("ordinary spell filter should lex");
    let ordinary =
        parse_for_each_object_filter(&ordinary).expect("ordinary spell filter should parse");
    assert_eq!(
        ordinary.stack_kind,
        Some(crate::filter::StackObjectKind::Spell)
    );
    assert!(ordinary.has_mana_cost);
    assert!(!ordinary.has_conjunctive_set_surface());
}

#[test]
fn relative_participant_count_compares_the_same_set_for_them_and_you() {
    let effect = parsed_debug("Each opponent who controls fewer creatures than you draws a card.");
    let compact: String = effect.chars().filter(|ch| !ch.is_whitespace()).collect();

    assert!(effect.contains("ValueComparison"), "{effect}");
    assert!(effect.contains("operator: LessThan"), "{effect}");
    assert!(
        compact.contains("controller:Some(IteratedPlayer,)"),
        "{effect}"
    );
    assert!(compact.contains("controller:Some(You,)"), "{effect}");
    assert!(!effect.contains("PlayerControls {"), "{effect}");
}
