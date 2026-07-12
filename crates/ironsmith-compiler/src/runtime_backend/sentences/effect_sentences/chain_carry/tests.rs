use crate::cards::builders::{
    CardDefinitionBuilder, EffectAst, PlayerAst, SubjectVerbActionAst, SubjectVerbEffectAst,
};
use crate::ids::CardId;
use crate::zone::Zone;

use super::super::super::lexer::lex_line;
use super::{
    parse_effect_chain_lexed, parse_effect_clause_with_trailing_if_lexed,
    parse_effect_sentence_lexed, parse_leading_player_may_lexed, starts_like_create_fragment_lexed,
};

#[test]
fn leading_may_land_play_permission_does_not_lower_to_may_effect() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Explore")
        .parse_text("You may play an additional land this turn.\nDraw a card.")
        .expect("explore-style text should parse");

    let spell_debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    assert!(
        super::string_contains(&spell_debug, "AdditionalLandPlaysEffect")
            || super::string_contains(&spell_debug, "additional_land_plays"),
        "expected Explore-style permission text to lower to additional land plays, got {spell_debug}"
    );
}

#[test]
fn create_fragment_probe_accepts_capitalized_pt_token_clauses() {
    let tokens = lex_line("Two 1/1 white Soldier creature tokens", 0)
        .expect("rewrite lexer should classify create-fragment text");

    assert!(starts_like_create_fragment_lexed(&tokens));
}

#[test]
fn create_fragment_probe_accepts_named_token_appositive_clauses() {
    let tokens = lex_line(
        "a legendary 2/1 black Skeleton creature token with \"Jumblebones can't block\"",
        0,
    )
    .expect("rewrite lexer should classify named-token appositive text");

    assert!(starts_like_create_fragment_lexed(&tokens));
}

#[test]
fn parses_named_token_appositive_with_quoted_trigger_rules() {
    let tokens = lex_line(
        "Create Jumblebones, a legendary 2/1 black Skeleton creature token with \"Jumblebones can't block\" and \"When Jumblebones leaves the battlefield, return target card named Ozox, the Clattering King from your graveyard to your hand.\"",
        0,
    )
    .expect("named-token appositive should lex");

    parse_effect_chain_lexed(&tokens)
        .expect("named-token appositive with nested token trigger should parse");
}

#[test]
fn parses_target_card_type_list_with_lte_mana_value_reference() {
    let tokens = lex_line(
        "Exile target enchantment, instant, or sorcery card with equal or lesser mana value than that spell from an opponent's graveyard",
        0,
    )
    .expect("target list clause should lex");

    parse_effect_chain_lexed(&tokens).expect("target list clause should parse");
}

#[test]
fn coordinated_tap_set_stays_one_antecedent_for_then_them() {
    let tokens = lex_line(
        "Tap this creature and all creatures named Kobolds of Kher Keep, then an opponent gains control of them.",
        0,
    )
    .expect("coordinated tap chain should lex");

    let effects = parse_effect_chain_lexed(&tokens).expect("coordinated tap chain should parse");
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::TapAll { filter },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::GainControl { .. },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected tap-union then gain-control effects, got {effects:#?}");
    };
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    assert!(filter.any_of[0].source, "{filter:#?}");
    assert_eq!(
        filter.any_of[1].name.as_deref(),
        Some("kobolds of kher keep")
    );
}

#[test]
fn chain_entrypoint_accepts_nonverb_additional_phase_clause() {
    let tokens = lex_line("There's an additional combat phase after this phase.", 0)
        .expect("additional phase clause should lex");

    let effects = parse_effect_chain_lexed(&tokens).expect("additional phase should parse");
    assert!(
        matches!(
            effects.as_slice(),
            [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::AdditionalPhases { .. },
                ..
            })]
        ),
        "{effects:#?}"
    );
}

#[test]
fn copy_then_gain_clause_keeps_the_explicit_gain_duration() {
    let tokens = lex_line(
        "Each land you control of that type becomes a copy of target creature you control until end of turn and gains haste until end of turn.",
        0,
    )
    .expect("copy-and-gain clause should lex");

    let effects = parse_effect_chain_lexed(&tokens).expect("copy-and-gain clause should parse");
    let gain = effects
        .iter()
        .find_map(|effect| match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::GrantAbilitiesAll { duration, .. },
                ..
            }) => Some(duration),
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected an all-lands haste grant, got {effects:#?}"));
    assert_eq!(*gain, crate::effect::Until::EndOfTurn, "{effects:#?}");
}

#[test]
fn trailing_if_keeps_relative_target_spell_controller_predicate() {
    let tokens = lex_line(
        "Counter target spell if you control more creatures than that spell's controller.",
        0,
    )
    .expect("relative counter condition should lex");

    let effect = parse_effect_clause_with_trailing_if_lexed(&tokens)
        .expect("relative counter condition should parse");
    assert!(
        matches!(
            effect,
            EffectAst::Conditional {
                predicate: crate::cards::builders::PredicateAst::YouControlMoreCreaturesThanTargetSpellController,
                ..
            }
        ),
        "{effect:#?}"
    );
}

#[test]
fn source_linked_exile_reveal_keeps_nonpermanents_face_up_and_moves_only_permanents() {
    let tokens = lex_line(
        "Each player turns face up all cards they own exiled with this artifact, then puts all permanent cards among them onto the battlefield.",
        0,
    )
    .expect("source-linked exile sequence should lex");

    let effects = parse_effect_chain_lexed(&tokens).expect("sequence should parse");
    let sentence_effects =
        parse_effect_sentence_lexed(&tokens).expect("sentence entrypoint should parse");
    assert_eq!(sentence_effects, effects);
    let [EffectAst::ForEachPlayer { effects: nested }] = effects.as_slice() else {
        panic!("expected per-player source-linked sequence, got {effects:#?}");
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::TurnFaceUp { target },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ReturnAllToBattlefield { filter, .. },
            ..
        }),
    ] = nested.as_slice()
    else {
        panic!("expected reveal then permanent-return effects, got {nested:#?}");
    };
    let crate::cards::builders::TargetAst::Object(reveal_filter, None, None) = target else {
        panic!("expected non-target reveal filter, got {target:#?}");
    };
    for candidate in [reveal_filter, filter] {
        assert_eq!(candidate.zone, Some(Zone::Exile));
        assert_eq!(
            candidate.owner,
            Some(crate::target::PlayerFilter::IteratedPlayer)
        );
        assert!(
            candidate
                .tagged_constraints
                .iter()
                .any(|constraint| { constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG })
        );
    }
    assert!(reveal_filter.card_types.is_empty(), "{reveal_filter:#?}");
    assert_eq!(filter.card_types.len(), 6, "{filter:#?}");
}

#[test]
fn leading_player_may_probe_accepts_capitalized_opponent_clauses() {
    let tokens = lex_line("An opponent may cast it", 0)
        .expect("rewrite lexer should classify player-may text");

    assert_eq!(
        parse_leading_player_may_lexed(&tokens),
        Some(PlayerAst::Opponent)
    );
}

#[test]
fn leading_player_may_probe_accepts_then_target_player_clauses() {
    let tokens = lex_line("Then target player may draw a card", 0)
        .expect("rewrite lexer should classify target-player may text");

    assert_eq!(
        parse_leading_player_may_lexed(&tokens),
        Some(PlayerAst::Target)
    );
}

#[test]
fn leading_player_may_probe_accepts_possessive_controller_clauses() {
    let tokens = lex_line("That creature's controller may cast it", 0)
        .expect("rewrite lexer should classify possessive controller text");

    assert_eq!(
        parse_leading_player_may_lexed(&tokens),
        Some(PlayerAst::ItsController)
    );
}

#[test]
fn leading_player_may_probe_accepts_that_attacking_player_clauses() {
    let tokens = lex_line("That attacking player may create a tapped Zombie token", 0)
        .expect("rewrite lexer should classify attacking-player may text");

    assert_eq!(
        parse_leading_player_may_lexed(&tokens),
        Some(PlayerAst::Attacking)
    );
}

#[test]
fn leading_player_may_probe_accepts_that_player_or_target_controller_clauses() {
    let tokens = lex_line(
        "That player or that permanent's controller may draw a card",
        0,
    )
    .expect("rewrite lexer should classify split controller text");

    assert_eq!(
        parse_leading_player_may_lexed(&tokens),
        Some(PlayerAst::ThatPlayerOrTargetController)
    );
}

#[test]
fn top_cards_then_put_counted_into_hand_rest_graveyard_chain_parses() {
    let tokens = lex_line(
        "Look at the top three cards of your library, then put one of them into your hand and the rest into your graveyard",
        0,
    )
    .expect("looked-cards split clause should lex");

    let effects =
        parse_effect_chain_lexed(&tokens).expect("looked-cards split clause should parse");

    match effects.as_slice() {
        [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::LookAtTopCards { .. },
                ..
            }),
            EffectAst::SnapshotLastObjectTag { .. },
            EffectAst::ChooseTaggedObjectsInZone {
                player,
                count,
                zone: Zone::Library,
                ..
            },
            EffectAst::MoveTaggedGroupToZone {
                zone: Zone::Hand, ..
            },
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::PutTaggedRemainderInZone {
                        zone: Zone::Graveyard,
                        ..
                    },
                ..
            }),
        ] => {
            assert_eq!(*player, PlayerAst::You);
            assert_eq!(*count, crate::effect::ChoiceCount::exactly(1));
        }
        other => panic!("expected composed looked-cards split effects, got {other:?}"),
    }
}

#[test]
fn exile_then_shuffle_graveyard_chain_keeps_both_effects() {
    let tokens = lex_line(
        "Exile all cards from your library face down, then shuffle all cards from your graveyard into your library.",
        0,
    )
    .expect("rewrite lexer should classify exile-then-shuffle text");
    let effects = parse_effect_chain_lexed(&tokens).expect("chain should parse");
    let debug = format!("{effects:?}");

    assert!(
        debug.contains("ExileAll")
            && debug.contains("face_down: true")
            && debug.contains("ShuffleGraveyardIntoLibrary"),
        "expected exile-all face-down and graveyard shuffle effects, got {debug}"
    );
    assert!(
        effects.iter().any(|effect| matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ExileAll {
                    face_down: true,
                    ..
                },
                ..
            })
        )),
        "expected a face-down exile-all effect in the parsed chain: {debug}"
    );
    assert!(
        effects.iter().any(|effect| {
            matches!(
                effect,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::ShuffleGraveyardIntoLibrary,
                    ..
                })
            )
        }),
        "expected a graveyard shuffle effect in the parsed chain: {debug}"
    );
}

#[test]
fn or_action_clause_preserves_secondary_or_inside_sacrifice_filter() {
    let tokens = lex_line(
        "Discard two cards or sacrifice a creature or planeswalker of your choice",
        0,
    )
    .expect("or-action text should lex");

    let parsed = super::parse_or_action_clause_lexed(&tokens)
        .expect("or-action parse should succeed")
        .expect("or-action clause should be recognized");

    let debug = format!("{parsed:?}");
    assert!(
        debug.contains("UnlessAction"),
        "expected or-action lowering to use unless-action AST, got {debug}"
    );
    assert!(
        debug.contains("Discard"),
        "expected discard branch in or-action AST, got {debug}"
    );
    assert!(
        debug.contains("Sacrifice"),
        "expected sacrifice branch in or-action AST, got {debug}"
    );
    assert!(
        debug.contains("Planeswalker"),
        "expected sacrifice filter to keep planeswalker branch, got {debug}"
    );
}
