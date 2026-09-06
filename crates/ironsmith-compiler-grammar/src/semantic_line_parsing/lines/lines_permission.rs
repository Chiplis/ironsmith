use crate::cards::builders::LibraryActionAst;
use crate::cards::builders::GrantActionAst;
use super::*;

pub(super) fn exact_dynamic_exile_permission_bundle(
    effect_parse_tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    // Do not call the aggregate typed-bundle dispatcher from this CST proof.
    // The public triggered-line router asks this predicate before choosing a
    // split candidate, while that dispatcher can in turn enter the same
    // public routing path.  Parse only the reusable two-sentence rule whose
    // shape this guard is proving.
    let sentences = split_lexed_sentences(effect_parse_tokens)
        .into_iter()
        .map(crate::effect_sentences::SentenceInput::from_lexed)
        .collect::<Vec<_>>();
    if sentences.len() != 2 {
        return None;
    }
    let effects = crate::effect_sentences::parse_dynamic_exile_top_then_play_for_as_long_as_exiled(
        &sentences, 0,
    )
    .ok()??;
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject:
                SubjectVerbSubjectAst {
                    player: PlayerAst::ItsOwner,
                    ..
                },
            action:
                SubjectVerbActionAst::Library(LibraryActionAst::ExileTopOfLibrary {
                    count,
                    tags,
                    face_down: false,
                    ..
                }),
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedForAsLongAsExiled {
                    tag,
                    allow_land: false,
                    ..
                }),
            ..
        }),
    ] = effects.as_slice()
    else {
        return None;
    };
    if tags != std::slice::from_ref(tag)
        || !matches!(
            count.unhinted(),
            Value::PowerOf(spec)
                if matches!(spec.as_ref(), ChooseSpec::Tagged(tag) if tag.as_str() == "triggering")
        )
    {
        return None;
    }
    Some(effects)
}

/// Lexical CST proof for the dynamic two-sentence exile permission. At this
/// boundary, contextual `its` references have not yet been bound to the
/// triggering object, so the stronger typed proof above is intentionally too
/// early. The semantic lowering pass still has to produce the exact typed
/// PowerOf/ItsOwner/shared-tag bundle before this route has any effect.
pub fn is_authored_dynamic_exile_permission_bundle(effect_parse_tokens: &[OwnedLexToken]) -> bool {
    let sentences = split_lexed_sentences(effect_parse_tokens);
    let [exile, permission] = sentences.as_slice() else {
        return false;
    };
    matches!(
        crate::lexer::parser_token_word_refs(exile).as_slice(),
        [
            "exile", "cards", "equal", "to", "its", "power", "from", "the", "top", "of", "its",
            "owners", "library"
        ]
    ) && matches!(
        crate::lexer::parser_token_word_refs(permission).as_slice(),
        [
            "you", "may", "cast", "spells", "from", "among", "those", "cards", "for", "as", "long",
            "as", "they", "remain", "exiled", "and", "mana", "of", "any", "type", "can", "be",
            "spent", "to", "cast", "them"
        ]
    )
}
