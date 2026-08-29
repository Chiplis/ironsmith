use super::*;

#[test]
fn chosen_permanents_and_sacrifice_results_keep_distinct_typed_sets() {
    let tokens = crate::lexer::lex_line(
            "For each player, choose target permanent that player controls. Those players sacrifice those permanents. Each player who sacrificed a permanent this way reveals the top card of their library, then puts it onto the battlefield if it's a permanent card.",
            0,
        )
        .expect("correlated each-player sequence should lex");
    let effects = parse_effect_sentences_lexed(&tokens)
        .expect("correlated each-player sequence should parse");

    let [
        EffectAst::ForEachPlayer { .. },
        EffectAst::ForEachPlayer {
            effects: sacrifice_effects,
        },
        EffectAst::ForEachPlayerDid {
            effects: followups,
            predicate: Some(_),
            ..
        },
    ] = effects.as_slice()
    else {
        panic!("expected target, tagged sacrifice, and correlated follow-up: {effects:#?}");
    };
    let [EffectAst::TagAffected { effect, tag }] = sacrifice_effects.as_slice() else {
        panic!("the plural sacrifice must export its actual result set: {sacrifice_effects:#?}");
    };
    assert!(
        tag.as_str().starts_with("__sentence_helper_sacrificed_"),
        "{tag:?}"
    );
    assert!(matches!(
        effect.as_ref(),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::SacrificeAll { .. },
            ..
        })
    ));

    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::RevealTop,
            ..
        }),
        EffectAst::TrailingIf {
            effects: move_effects,
            ..
        },
    ] = followups.as_slice()
    else {
        panic!("the reveal and conditional move must both survive: {followups:#?}");
    };
    assert!(matches!(
        move_effects.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::MoveToZone {
                zone: Zone::Battlefield,
                ..
            },
            ..
        })]
    ));
}

#[test]
fn wave_of_vitriol_keeps_sacrificed_lands_partitioned_by_snapshot_controller() {
    let tokens = crate::lexer::lex_line(
            "Each player sacrifices all artifacts, enchantments, and nonbasic lands they control. For each land sacrificed this way, its controller may search their library for a basic land card and put it onto the battlefield tapped. Then each player who searched their library this way shuffles.",
            0,
        )
        .expect("Wave of Vitriol should lex");
    let effects =
        parse_effect_sentences_lexed(&tokens).expect("Wave of Vitriol should parse structurally");

    let [
        EffectAst::TagAffected {
            effect: sacrifice,
            tag: sacrificed_tag,
        },
        EffectAst::ForEachTagged {
            tag: iterated_tag,
            effects: land_effects,
        },
        EffectAst::ForEachPlayerDid { .. },
    ] = effects.as_slice()
    else {
        panic!("expected tagged sacrifice, typed iterator, and searched-player gate: {effects:#?}");
    };
    assert_eq!(sacrificed_tag, iterated_tag);
    let EffectAst::ForEachPlayer {
        effects: sacrifice_effects,
    } = sacrifice.as_ref()
    else {
        panic!("the tagged producer must remain an each-player loop: {sacrifice:#?}");
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::SacrificeAll { filter: union },
            ..
        }),
    ] = sacrifice_effects.as_slice()
    else {
        panic!("the each-player loop must contain one all-set sacrifice: {sacrifice_effects:#?}");
    };
    assert_eq!(union.any_of.len(), 3, "{union:#?}");
    let artifact = union
        .any_of
        .iter()
        .find(|branch| branch.card_types == [crate::types::CardType::Artifact])
        .expect("artifact union arm");
    let enchantment = union
        .any_of
        .iter()
        .find(|branch| branch.card_types == [crate::types::CardType::Enchantment])
        .expect("enchantment union arm");
    let nonbasic_land = union
        .any_of
        .iter()
        .find(|branch| branch.card_types == [crate::types::CardType::Land])
        .expect("land union arm");
    assert!(artifact.excluded_supertypes.is_empty());
    assert!(enchantment.excluded_supertypes.is_empty());
    assert_eq!(
        nonbasic_land.excluded_supertypes,
        [crate::types::Supertype::Basic]
    );
    let [
        EffectAst::Conditional {
            predicate: PredicateAst::ItMatchedLastKnown(filter),
            if_true,
            if_false,
        },
    ] = land_effects.as_slice()
    else {
        panic!("sacrifice iterator must gate each LKI snapshot by type: {land_effects:#?}");
    };
    assert_eq!(filter.card_types.as_slice(), [crate::types::CardType::Land]);
    assert!(if_false.is_empty());
    assert!(if_true.iter().any(|effect| matches!(
        effect,
        EffectAst::MayByPlayer {
            player: PlayerAst::That,
            ..
        }
    )));
}
