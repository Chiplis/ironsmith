//! Source-authored regression assertions; execution is deferred.
use super::*;

fn lex(text: &str) -> Vec<OwnedLexToken> {
    crate::lexer::lex_line(text, 0).expect("authored body should lex")
}

#[test]
fn rejected_animation_ownership_probe_does_not_report_subject_recovery() {
    let tokens = lex("If this card is in your opening hand, you may begin the game with it on the battlefield.");
    // Establish the adversarial witness: the legacy reader reaches a lossy
    // subject, then rejects the non-P/T predicate. This is not a committed AST.
    let (legacy, speculative_loss) = crate::parse_loss::capture(|| {
        parse_filter_is_pt_creature_in_addition_line(&tokens)
    });
    assert!(legacy.unwrap().is_none());
    assert!(speculative_loss.diagnostics().iter().any(|loss| {
        loss.code == "suffix_object_filter_recovery"
    }));
    let (candidate, loss) = crate::parse_loss::capture(|| {
        parse_conditional_copular_creature_line(&tokens)
    });
    assert!(candidate.unwrap().is_none());
    assert!(!loss.is_lossy(), "{loss:?}");
}

#[test]
fn nested_probe_preserves_outer_losses_and_later_committed_suffix_loss() {
    let tokens = lex("If this card is in your opening hand, you may begin the game with it on the battlefield.");
    let (_, loss) = crate::parse_loss::capture(|| {
        crate::parse_loss::record("outer_before", "must survive nested capture");
        assert!(parse_conditional_copular_creature_line(&tokens).unwrap().is_none());
        // A caller that actually accepts a recovered suffix must still be
        // diagnosed. The fix must not disable record(), replay(), or the gate.
        assert!(parse_best_object_filter_suffix(&lex("if this card")).is_some());
        crate::parse_loss::record("outer_after", "must survive nested capture");
    });
    let codes: Vec<_> = loss.diagnostics().iter().map(|loss| loss.code.as_str()).collect();
    assert_eq!(codes, ["outer_before", "suffix_object_filter_recovery", "outer_after"]);
}

#[test]
fn complete_pregame_body_keeps_its_owner_without_probe_loss() {
    for (body, nonstarting, exiled) in [
        ("If this card is in your opening hand, you may begin the game with it on the battlefield.", false, 0),
        ("If this card is in your opening hand and you're not the starting player, you may begin the game with it on the battlefield with a luck counter on it. If you do, exile a card from your hand.", true, 1),
    ] {
        let (parsed, loss) = crate::parse_loss::capture(|| {
            super::super::parse_static_ability_ast_line_lexed(&lex(body))
        });
        let abilities = parsed.unwrap().expect("complete pregame owner");
        assert!(!loss.is_lossy(), "{body}: {loss:?}");
        assert_eq!(abilities.len(), 1, "{abilities:?}");
        let StaticAbilityAst::Static(ability) = &abilities[0] else {
            panic!("pregame cannot be reinterpreted as a battlefield conditional: {abilities:?}");
        };
        let Some(crate::static_abilities::PregameActionKind::BeginOnBattlefield(spec)) = ability.pregame_action_kind() else {
            panic!("expected pregame owner: {ability:?}");
        };
        assert_eq!(spec.require_not_starting_player, nonstarting);
        assert_eq!(spec.exile_cards_from_hand, exiled);
        assert_eq!(spec.counters.is_empty(), !nonstarting);
    }
}

#[test]
fn established_sized_animation_is_deferred_and_still_available_to_its_owner() {
    let tokens = lex("This artifact is a 3/3 Golem creature in addition to its other types.");
    let (deferred, loss) = crate::parse_loss::capture(|| {
        parse_conditional_copular_creature_line(&tokens)
    });
    assert!(deferred.unwrap().is_none());
    assert!(!loss.is_lossy(), "{loss:?}");
    let (committed, loss) = crate::parse_loss::capture(|| {
        parse_filter_is_pt_creature_in_addition_line(&tokens)
    });
    assert!(committed.unwrap().is_some());
    assert!(!loss.is_lossy(), "{loss:?}");
}

#[test]
fn deferring_a_lossy_sized_match_does_not_clean_the_committed_owner() {
    // Deliberately malformed source condition: the legacy owner's suffix
    // recovery can build an AST, but the support gate must still see its loss.
    let tokens = lex("If this card is a 3/3 Golem creature in addition to its other types.");
    let (deferred, probe_loss) = crate::parse_loss::capture(|| {
        parse_conditional_copular_creature_line(&tokens)
    });
    assert!(deferred.unwrap().is_none());
    assert!(!probe_loss.is_lossy(), "{probe_loss:?}");
    let (committed, committed_loss) = crate::parse_loss::capture(|| {
        parse_filter_is_pt_creature_in_addition_line(&tokens)
    });
    assert!(committed.unwrap().is_some());
    assert!(committed_loss.diagnostics().iter().any(|loss| {
        loss.code == "suffix_object_filter_recovery"
    }), "{committed_loss:?}");
}
