use super::*;

pub(super) fn parse_choose_each_graveyard_then_owner_shuffle_bundle(
    choice_sentence: &[OwnedLexToken],
    shuffle_sentence: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if bundle_grammar::parse_each_graveyard_owner_shuffle_shape(choice_sentence, shuffle_sentence)
        .is_none()
    {
        return Ok(None);
    }

    let Some((chooser, mut filter, count, count_value)) =
        parse_you_choose_objects_clause_with_count_value(choice_sentence)?
    else {
        return Ok(None);
    };
    if !matches!(chooser, PlayerAst::Implicit | PlayerAst::You)
        || filter.zone != Some(Zone::Graveyard)
    {
        return Ok(None);
    }

    // "Each graveyard" partitions the choice by graveyard owner. The
    // resolving ability's controller makes every choice, while the surrounding
    // player loop binds the graveyard whose cards are eligible.
    filter.controller = None;
    filter.owner = Some(PlayerFilter::IteratedPlayer);
    filter.single_graveyard = false;

    // This is a collection selected inside a player loop, not the loop's
    // implicit `__it__` object. Give it a distinct tag so lowering preserves
    // the collection instead of rewriting the reference to `Iterated`.
    let chosen_tag = crate::tag::CompilerReferenceTag::EachGraveyardChosen.bind();
    let chosen_target = TargetAst::Tagged(chosen_tag.clone(), span_from_tokens(shuffle_sentence));

    Ok(Some(vec![EffectAst::ForEachPlayer {
        effects: vec![
            EffectAst::ChooseObjects {
                filter,
                count,
                count_value,
                player: PlayerAst::You,
                tag: chosen_tag,
            },
            EffectAst::subject_verb_shuffle_objects_into_library(
                PlayerAst::ItsOwner,
                chosen_target,
            ),
        ],
    }]))
}
