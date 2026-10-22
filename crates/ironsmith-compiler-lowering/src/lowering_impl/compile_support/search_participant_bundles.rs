//! Correlated search offers with a later reward and one final shuffle per searcher.
use super::*;
use crate::cards::builders::{ForEachEffectAst, PermissionEffectAst, LibraryActionAst};

fn is_search(effect: &EffectAst) -> bool {
    matches!(effect, EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SearchLibrary { .. }), ..
    }))
}

fn capture_searches(effect: &Effect, tag: &TagKey) -> Result<Effect, CardTextError> {
    if let Some(search) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() {
        let mut search = search.clone();
        if search.is_search { search.search_participant_tag = Some(tag.clone()); }
        return Ok(Effect::new(search));
    }
    if let Some(search) = effect.downcast_ref::<crate::effects::SearchLibraryEffect>() {
        let mut search = search.clone(); search.participant_tag = Some(tag.clone());
        return Ok(Effect::new(search));
    }
    macro_rules! body {
        ($kind:ty) => { if let Some(wrapper) = effect.downcast_ref::<$kind>() {
            let mut wrapper = wrapper.clone();
            wrapper.effects = wrapper.effects.iter().map(|effect| capture_searches(effect, tag)).collect::<Result<_, _>>()?;
            return Ok(Effect::new(wrapper));
        } };
    }
    body!(crate::effects::SequenceEffect);
    body!(crate::effects::ForPlayersEffect<Effect>);
    body!(crate::effects::MayEffect<Effect>);
    if let Some(wrapper) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        let mut wrapper = wrapper.clone(); wrapper.effect = Box::new(capture_searches(&wrapper.effect, tag)?);
        return Ok(Effect::new(wrapper));
    }
    // The recognized program permits no other search-bearing control flow.
    // Do not silently miss a search if its lowering starts using a new wrapper.
    let mut nested_search = false;
    effect.visit_child_effects(&mut |child| {
        fn contains(effect: &Effect) -> bool {
            if effect.downcast_ref::<crate::effects::SearchLibraryEffect>().is_some()
                || effect.downcast_ref::<crate::effects::ChooseObjectsEffect>().is_some_and(|choice| choice.is_search) { return true; }
            let mut found = false; effect.visit_child_effects(&mut |child| found |= contains(child)); found
        }
        nested_search |= contains(child);
    });
    if nested_search { return Err(CardTextError::ParseError("search participant capture encountered an unsupported control wrapper".into())); }
    Ok(effect.clone())
}

pub(super) fn compile_search_participant_bundle(
    annotated: &AnnotatedEffectSequence,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let [initial, offers, reward, final_action] = annotated.effects.as_slice() else { return Ok(None); };
    if !is_search(&initial.effect) { return Ok(None); }
    let EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects: offer_body }) = &offers.effect else { return Ok(None); };
    let [EffectAst::Permissions(PermissionEffectAst::MayByPlayer { effects: optional_searches, .. })] = offer_body.as_slice() else { return Ok(None); };
    if optional_searches.is_empty() || !optional_searches.iter().all(is_search) { return Ok(None); }
    let EffectAst::ForEach(ForEachEffectAst::ForEachOpponentDid {
        effects: rewards, predicate: None, result_predicate: IfResultPredicate::SearchedLibrary,
    }) = &reward.effect else { return Ok(None); };
    if rewards.is_empty() || !rewards.iter().all(is_search) { return Ok(None); }
    let EffectAst::ForEach(ForEachEffectAst::ForEachPlayerDid {
        effects: final_effects, predicate: None, result_predicate: IfResultPredicate::SearchedLibrary,
    }) = &final_action.effect else { return Ok(None); };
    if !matches!(final_effects.as_slice(), [EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::Library(LibraryActionAst::ShuffleLibrary), ..
    })]) { return Ok(None); }
    let tag = ctx.next_tag("search_participants");
    // Each execution of the correlated instruction starts a new collection,
    // including executions inside a repeat process.
    let mut compiled = vec![Effect::new(crate::effects::TagPlayersEffect::new(
        PlayerFilter::excluding(PlayerFilter::Any, PlayerFilter::Any), tag.clone(),
    ))];
    let mut choices = Vec::new();
    for clause in [&initial.effect, &offers.effect] {
        let (effects, extra) = compile_effect(clause, ctx)?;
        compiled.extend(effects.iter().map(|effect| capture_searches(effect, &tag)).collect::<Result<Vec<_>, _>>()?);
        for choice in extra { push_choice(&mut choices, choice); }
    }
    // Finish every opponent's offer before any controller reward search.
    // The participant set is frozen by the player iterator before rewards add
    // further searches to the same deduplicated final-shuffle collection.
    let (rewards, extra) = compile_effects(rewards, ctx)?;
    for choice in extra { push_choice(&mut choices, choice); }
    let rewards = rewards.iter().map(|effect| capture_searches(effect, &tag)).collect::<Result<Vec<_>, _>>()?;
    let reward_loop = crate::effects::ForPlayersEffect {
        filter: PlayerFilter::excluding(PlayerFilter::TaggedPlayer(tag.clone()), PlayerFilter::your_team()),
        effects: rewards, sequential: true, starting_with_controller: false, stop_after_first_happened: false, announcement_condition: None,
    };
    compiled.push(Effect::new(reward_loop));
    compiled.push(Effect::new(crate::effects::ForEachTaggedPlayerEffect {
        tag, effects: vec![Effect::shuffle_library_player(PlayerFilter::IteratedPlayer)], require_evidence: true,
    }));
    Ok(Some((compiled, choices)))
}
