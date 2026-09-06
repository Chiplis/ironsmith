use super::*;

pub(super) fn parse_player_chooses_source_excluded_permanent_then_exiles(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let words = TokenWordView::new(trim_lexed_commas(tokens)).word_refs();
    if !crate::word_primitives::parse_sequence_complete(
        &words,
        &[
            "an",
            "opponent",
            "chooses",
            "a",
            "permanent",
            "you",
            "control",
            "other",
            "than",
            "this",
            "creature",
            "and",
            "exiles",
            "it",
        ],
    ) {
        return None;
    }
    let tag = crate::util::helper_tag_for_tokens(tokens, "chosen");
    let mut filter = ObjectFilter::permanent().you_control();
    filter.other = true;
    filter.source_surface = Some(SourceReferenceSurface::ThisPermanentType(
        "this creature".to_string(),
    ));
    Some(vec![
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::Opponent,
            tag: crate::tag::TagRef::of(tag.clone()),
        }),
        EffectAst::subject_verb_exile(TargetAst::Tagged(crate::tag::TagRef::of(tag), None), false),
    ])
}

pub(super) fn explicit_target_choose_spec(target: &TargetAst) -> Option<ChooseSpec> {
    match target {
        TargetAst::Object(filter, Some(_), _) => Some(ChooseSpec::Target(Box::new(
            ChooseSpec::Object(filter.clone()),
        ))),
        TargetAst::WithCount(inner, count) if count.is_single() => {
            explicit_target_choose_spec(inner)
        }
        TargetAst::WithCountValue(inner, count, _) if count.is_single() => {
            explicit_target_choose_spec(inner)
        }
        _ => None,
    }
}

pub(super) fn normalize_imperative_choose_player(effect: &mut EffectAst) -> bool {
    let player = match effect {
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { player, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsWithAggregateConstraint { player, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones { player, .. }) => player,
        _ => return false,
    };

    if matches!(
        player,
        PlayerAst::Implicit | PlayerAst::Target | PlayerAst::TargetOpponent | PlayerAst::That
    ) {
        *player = PlayerAst::You;
        return true;
    }
    false
}
