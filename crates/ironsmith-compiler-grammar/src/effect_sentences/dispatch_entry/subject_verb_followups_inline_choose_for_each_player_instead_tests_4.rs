use super::*;

fn choice_players(effect: &EffectAst) -> Vec<PlayerAst> {
    let EffectAst::ForEachPlayer { effects } = effect else {
        panic!("expected per-player procedure: {effect:#?}");
    };
    let effects = match effects.as_slice() {
        [EffectAst::CommaThen { effects }] => effects,
        _ => effects,
    };
    effects
        .iter()
        .filter_map(|effect| match effect {
            EffectAst::ChooseObjects { player, .. } => Some(*player),
            _ => None,
        })
        .collect()
}

#[test]
fn paid_colors_can_replace_only_the_per_player_chooser() {
    let tokens = crate::lexer::lex_line(
            "Each player chooses an artifact, a creature, an enchantment, and a planeswalker from among the nonland permanents they control, then sacrifices the rest. If {B}{R} was spent to cast this spell, you choose the permanents for each player instead.",
            0,
        )
        .expect("chooser replacement should lex");
    let parsed = parse_effect_sentences_lexed(&tokens).expect("chooser replacement should parse");
    let [
        EffectAst::SelfReplacement {
            predicate,
            if_true,
            if_false,
            ..
        },
    ] = parsed.as_slice()
    else {
        panic!("expected one self replacement: {parsed:#?}");
    };
    assert!(format!("{predicate:#?}").contains("Mana"), "{predicate:#?}");
    assert_eq!(choice_players(&if_true[0]), vec![PlayerAst::You; 4]);
    assert_eq!(choice_players(&if_false[0]), vec![PlayerAst::That; 4]);
}

#[test]
fn chooser_rewrite_rejects_a_non_complement_procedure() {
    let mut effect = EffectAst::ForEachPlayer {
        effects: vec![EffectAst::ChooseObjects {
            filter: ObjectFilter::creature().controlled_by(PlayerFilter::IteratedPlayer),
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::That,
            tag: crate::tag::CompilerReferenceTag::Chosen.bind(),
        }],
    };
    assert!(!rewrite_each_player_choice_complement_chooser(&mut effect));
    assert_eq!(choice_players(&effect), vec![PlayerAst::That]);
}
