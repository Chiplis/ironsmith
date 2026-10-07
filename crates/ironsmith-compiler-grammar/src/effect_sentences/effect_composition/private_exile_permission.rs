use super::*;

/// Keep exact-card readers inside the optional producer's result branch.
/// Declining the payment never executes readers with a missing antecedent.
pub(super) fn parse_optional_private_exile_play_bundle(
    sentences: &[&[OwnedLexToken]],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let [payment, conditional, inspection, permission] = sentences else { return Ok(None); };
    let Some(mut inspection) = crate::permission_helpers::parse_look_tagged_exile_permission(inspection)? else { return Ok(None); };
    let Some(mut permission) = parse_cast_or_play_tagged_clause(permission)? else { return Ok(None); };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst { action: SubjectVerbActionAst::Grants(
        GrantActionAst::GrantPlayTaggedWhileSourceOnBattlefield { tag: permission_tag, .. }), .. }) = &mut permission else { return Ok(None); };
    if permission_tag.as_str() != crate::tag::CompilerReferenceTag::It.as_str() { return Ok(None); }
    let Some(prefix) = crate::grammar::structure::split_leading_result_prefix_lexed(conditional) else { return Ok(None); };
    if prefix.kind != crate::grammar::structure::LeadingResultPrefixKind::If || prefix.predicate != IfResultPredicate::Did { return Ok(None); }
    let mut payments = effect_sentences::parse_effect_sentence_lexed(payment)?;
    let [optional] = payments.as_slice() else { return Ok(None); };
    let inner = match optional {
        EffectAst::Permissions(PermissionEffectAst::May { effects }) => effects,
        EffectAst::Permissions(PermissionEffectAst::MayByPlayer { player: PlayerAst::You, effects }) => effects,
        _ => return Ok(None),
    };
    if !matches!(inner.as_slice(), [EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::Mana(crate::cards::builders::ManaActionAst::PayMana { x_value: None, x_maximum: None, .. }), ..
    })]) { return Ok(None); }
    let mut exiles = effect_sentences::parse_effect_sentence_lexed(prefix.trailing_tokens)?;
    let [EffectAst::SubjectVerb(SubjectVerbEffectAst { subject, action: SubjectVerbActionAst::Library(
        LibraryActionAst::ExileTopOfLibrary { count, tags, accumulated_tags, face_down: true, .. }) })] = exiles.as_mut_slice() else { return Ok(None); };
    if count.unhinted() != &Value::Fixed(1) || !matches!(subject.player, PlayerAst::You | PlayerAst::Implicit) || !accumulated_tags.is_empty() { return Ok(None); }
    let tag = helper_tag_for_tokens(prefix.trailing_tokens, "exiled");
    tags.clear(); tags.push(crate::tag::TagRef::of(tag.clone()));
    *permission_tag = crate::tag::TagRef::of(tag.clone());
    let EffectAst::SubjectVerb(SubjectVerbEffectAst { action: SubjectVerbActionAst::RevealLook(
        RevealLookActionAst::LookAtObjects { filter, permit_while_exiled: true }), .. }) = &mut inspection else { return Ok(None); };
    let [constraint] = filter.tagged_constraints.as_mut_slice() else { return Ok(None); };
    constraint.tag = tag.into();
    exiles.extend([inspection, permission]);
    payments.push(EffectAst::Conditionals(ConditionalEffectAst::IfResult { predicate: IfResultPredicate::Did, effects: exiles }));
    Ok(Some(payments))
}
