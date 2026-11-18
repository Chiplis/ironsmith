//! Input bindings for every phase of a prepared object iteration.

use crate::effects::{ExecutionContext, ExecutionError};
#[cfg(test)]
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;

/// Own tag installation/removal and restoration for the whole selected
/// iteration or one captured phase. None keeps an absent binding absent;
/// it is distinct from installing an empty set. Reverse restoration also
/// preserves the existing precedence of repeated tag names.
pub(crate) fn with_iteration_tags<T>(
    ctx: &mut ExecutionContext,
    tags: impl IntoIterator<Item = (TagKey, Option<Vec<ObjectSnapshot>>)>,
    body: impl FnOnce(&mut ExecutionContext) -> Result<T, ExecutionError>,
) -> Result<T, ExecutionError> {
    let scope = IterationTagScope::enter(ctx, tags);
    let result = body(ctx);
    scope.leave(ctx);
    result
}

/// A retained tag frame uses the same installation and reverse restoration
/// owner as synchronous iteration helpers. Body writes remain visible until
/// this frame leaves; phase re-entry never re-installs stale input tags.
pub(super) struct IterationTagScope(Vec<(TagKey, Option<Vec<ObjectSnapshot>>, bool)>);
impl IterationTagScope {
    pub(super) fn enter(
        ctx: &mut ExecutionContext,
        tags: impl IntoIterator<Item = (TagKey, Option<Vec<ObjectSnapshot>>)>,
    ) -> Self {
        Self(
            tags.into_iter()
                .map(|(tag, snapshots)| {
                    let previous_exact=ctx.exact_object_reference_tags.remove(&tag);
                    let previous = ctx.tagged_objects.remove(&tag);
                    let same_exact_input=previous_exact && snapshots.as_ref().zip(previous.as_ref()).is_some_and(|(new,old)|new.iter().map(|snapshot|snapshot.object_id).eq(old.iter().map(|snapshot|snapshot.object_id)));
                    if let Some(snapshots) = snapshots {
                        ctx.set_tagged_objects(tag.clone(), snapshots);
                        if same_exact_input {ctx.exact_object_reference_tags.insert(tag.clone());}
                    }
                    (tag, previous, previous_exact)
                })
                .collect(),
        )
    }
    pub(super) fn leave(self, ctx: &mut ExecutionContext) {
        for (tag, previous, previous_exact) in self.0.into_iter().rev() {
            ctx.exact_object_reference_tags.remove(&tag);
            if previous_exact {ctx.exact_object_reference_tags.insert(tag.clone());}
            match previous {
                Some(snapshots) => {
                    ctx.tagged_objects.insert(tag, snapshots);
                }
                None => {
                    ctx.tagged_objects.remove(&tag);
                }
            }
        }
    }
}

/// Selected object/player bindings live for the complete iteration body in
/// a participant frame. Only the installed fields are restored on completion.
pub(super) struct IterationScope {
    object: Option<Option<ObjectId>>,
    player: Option<Option<PlayerId>>,
    tags: IterationTagScope,
}
impl IterationScope {
    pub(super) fn enter(
        ctx: &mut ExecutionContext,
        object: Option<ObjectId>,
        player: Option<PlayerId>,
        mut tags: Vec<(TagKey, Vec<ObjectSnapshot>)>,
    ) -> Self {
        let inherited_exact=tags.iter().any(|(tag,_)|tag.as_str()==ironsmith_core::tag::CURRENT_ITERATED_OBJECT_TAG);
        if let Some(object) = object {
            let snapshot=tags.iter().rev().find(|(tag,_)|tag.as_str()=="__it__")
                .and_then(|(_,snapshots)|snapshots.iter().find(|snapshot|snapshot.object_id==object)).cloned()
                .or_else(||ctx.get_tagged_all("__it__").and_then(|snapshots|snapshots.iter().find(|snapshot|snapshot.object_id==object)).cloned());
            // Empty hides any outer iteration when no exact input snapshot
            // is available; it never borrows a different enclosing object.
            tags.push((TagKey::from(ironsmith_core::tag::CURRENT_ITERATED_OBJECT_TAG),snapshot.into_iter().collect()));
        }
        let tags = IterationTagScope::enter(
            ctx,
            tags.into_iter()
                .map(|(tag, snapshots)| (tag, Some(snapshots))),
        );
        if object.is_some(){
            let current=TagKey::from(ironsmith_core::tag::CURRENT_ITERATED_OBJECT_TAG);
            if inherited_exact{ctx.exact_object_reference_tags.insert(current);}else{ctx.exact_object_reference_tags.remove(&current);}
        }
        let object = object
            .map(|object| std::mem::replace(&mut ctx.iteration.iterated_object, Some(object)));
        let player = player
            .map(|player| std::mem::replace(&mut ctx.iteration.iterated_player, Some(player)));
        Self {
            object,
            player,
            tags,
        }
    }
    pub(super) fn leave(self, ctx: &mut ExecutionContext) {
        if let Some(player) = self.player {
            ctx.iteration.iterated_player = player;
        }
        if let Some(object) = self.object {
            ctx.iteration.iterated_object = object;
        }
        self.tags.leave(ctx);
    }
}

/// Ordinary children and prepared phases share the same iteration-local
/// binding owner. Other successful child context outputs stay visible.
pub(crate) fn with_object_iteration<T>(
    ctx: &mut ExecutionContext,
    object: ObjectId,
    player: PlayerId,
    tags: Vec<(TagKey, Vec<ObjectSnapshot>)>,
    body: impl FnOnce(&mut ExecutionContext) -> Result<T, ExecutionError>,
) -> Result<T, ExecutionError> {
    let scope = IterationScope::enter(ctx, Some(object), Some(player), tags);
    let result = body(ctx);
    scope.leave(ctx);
    result
}

#[cfg(test)]
mod current_iteration_operand_tests {
    use super::*;
    #[test]
    fn current_iteration_reference_survives_choices_and_restores_nested_frames() {
        let player=PlayerId(0);let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let card=crate::card::CardBuilder::new(crate::CardId::new(),"Iteration input").card_types(vec![crate::CardType::Sorcery]).build();
        let ids=(0..3).map(|_|game.create_object_from_card(&card,player,crate::Zone::Exile)).collect::<Vec<_>>();
        let snapshots=ids.iter().map(|id|ObjectSnapshot::from_object(game.object(*id).unwrap(),&game)).collect::<Vec<_>>();
        let key=ironsmith_core::tag::CURRENT_ITERATED_OBJECT_TAG;
        let mut ctx=ExecutionContext::new_default(ids[2],player);ctx.set_tagged_objects(key,vec![snapshots[2].clone()]);
        with_object_iteration(&mut ctx,ids[0],player,vec![(TagKey::from("__it__"),vec![snapshots[0].clone()])],|ctx| {
            assert_eq!(ctx.get_tagged(key).unwrap().object_id,ids[0]);
            ctx.set_tagged_objects("__it__",vec![snapshots[1].clone()]);
            assert_eq!(ctx.get_tagged(key).unwrap().object_id,ids[0],"ordinary it can change without rebinding input operand");
            with_object_iteration(ctx,ids[1],player,vec![(TagKey::from("__it__"),vec![snapshots[1].clone()])],|ctx| {
                assert_eq!(ctx.get_tagged(key).unwrap().object_id,ids[1]);Ok(())
            })?;
            assert_eq!(ctx.get_tagged(key).unwrap().object_id,ids[0]);Ok(())
        }).unwrap();
        assert_eq!(ctx.get_tagged(key).unwrap().object_id,ids[2]);
    }
}

#[cfg(test)]
mod captured_iteration_cast_tests {
    use super::*;
    #[test]
    fn later_iteration_card_reexile_obeys_captured_domain_policy_and_local_move_exception(){
        for captured_domain in [false,true] {
        let player=PlayerId(0);let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let card=crate::card::CardBuilder::new(crate::CardId::new(),"Iteration copy").card_types(vec![crate::CardType::Sorcery]).build();
        let first=game.create_object_from_card(&card,player,crate::Zone::Exile);let second=game.create_object_from_card(&card,player,crate::Zone::Exile);let second_stable=game.object(second).unwrap().stable_id;
        let snapshots=[first,second].iter().map(|id|ObjectSnapshot::from_object(game.object(*id).unwrap(),&game)).collect();
        let mut ctx=ExecutionContext::new_default(first,player);ctx.set_tagged_objects("pool",snapshots);if captured_domain{ctx.exact_object_reference_tags.insert(TagKey::from("pool"));}
        let effects=vec![
            crate::effect::Effect::new(crate::effects::MoveToZoneEffect::new(crate::target::ChooseSpec::SpecificObject(second),crate::Zone::Hand,false)).tag_all("moved_later"),
            crate::effect::Effect::new(crate::effects::MoveToZoneEffect::new(crate::target::ChooseSpec::Tagged(TagKey::from("moved_later")),crate::Zone::Exile,false)),
            crate::effect::Effect::new(crate::effects::CastTaggedEffect::new(ironsmith_core::tag::CURRENT_ITERATED_OBJECT_TAG,crate::target::PlayerFilter::You).as_copy().without_paying_mana_cost()),
        ];
        crate::effects::execute_effect(&mut game,&crate::effect::Effect::new(crate::effects::ForEachTaggedEffect::new("pool",effects)),&mut ctx).unwrap();
        assert_ne!(game.find_object_by_stable_id(second_stable),Some(second));assert_eq!(game.stack.len(),if captured_domain{1}else{2},"captured domain cannot revive a stale member; a local iteration keeps the ordinary same-resolution move exception");
        }
    }
}

#[cfg(test)]
mod nested_local_iteration_cast_tests {
    use super::*;
    #[test]
    fn nested_local_same_object_loop_does_not_inherit_outer_captured_cast_policy(){
        let player=PlayerId(0);let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let card=crate::card::CardBuilder::new(crate::CardId::new(),"Nested loop copy").card_types(vec![crate::CardType::Sorcery]).build();
        let object=game.create_object_from_card(&card,player,crate::Zone::Exile);let source=game.create_object_from_card(&card,player,crate::Zone::Exile);let snapshot=ObjectSnapshot::from_object(game.object(object).unwrap(),&game);let key=TagKey::from(ironsmith_core::tag::CURRENT_ITERATED_OBJECT_TAG);
        let mut ctx=ExecutionContext::new_default(source,player);
        with_object_iteration(&mut ctx,object,player,vec![(TagKey::from("__it__"),vec![snapshot.clone()]),(key.clone(),vec![snapshot.clone()])],|ctx|{
            assert!(ctx.exact_object_reference_tags.contains(&key));
            with_object_iteration(ctx,object,player,vec![(TagKey::from("__it__"),vec![snapshot.clone()])],|ctx|{
                assert!(!ctx.exact_object_reference_tags.contains(&key));
                let moved=crate::effect::Effect::new(crate::effects::MoveToZoneEffect::new(crate::target::ChooseSpec::SpecificObject(object),crate::Zone::Hand,false)).tag_all("local_move");
                crate::effects::execute_effect(&mut game,&moved,ctx)?;
                crate::effects::execute_effect(&mut game,&crate::effect::Effect::new(crate::effects::MoveToZoneEffect::new(crate::target::ChooseSpec::Tagged(TagKey::from("local_move")),crate::Zone::Exile,false)),ctx)?;
                crate::effects::execute_effect(&mut game,&crate::effect::Effect::new(crate::effects::CastTaggedEffect::new(key.clone(),crate::target::PlayerFilter::You).as_copy().without_paying_mana_cost()),ctx)?;
                Ok(())
            })?;
            assert!(ctx.exact_object_reference_tags.contains(&key));assert_eq!(game.stack.len(),1,"local frame retained ordinary same-resolution move exception");
            let stale=crate::effects::execute_effect(&mut game,&crate::effect::Effect::new(crate::effects::CastTaggedEffect::new(key.clone(),crate::target::PlayerFilter::You).as_copy().without_paying_mana_cost()),ctx)?;
            assert_eq!(stale.status,crate::effect::OutcomeStatus::TargetInvalid);assert_eq!(game.stack.len(),1);Ok(())
        }).unwrap();
    }
}
