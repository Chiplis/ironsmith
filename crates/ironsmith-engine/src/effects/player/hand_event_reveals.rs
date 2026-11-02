//! Event-time hand reveal windows with captured complete replacement prices.
//! The window is frozen with the original event; revealing is answered before
//! subsequent instructions/additions, and casting waits for its own trigger.
use crate::{GameState,ObjectId,PlayerId,Zone};
use crate::effects::{ExecutionContext,ExecutionError};
use crate::snapshot::ObjectSnapshot;
use crate::triggers::{Trigger,TriggerContext,TriggerEvent,TriggeredAbilityEntry,TriggeredAbilitySourceKind};
use crate::alternative_cast::AlternativeCastingMethod;
use crate::grant::Grantable;
use crate::filter::ObjectFilterExt as _;
#[derive(Debug,Clone)]
pub(crate) struct HandRevealWindow {
    event:TriggerEvent,
    frame:HandRevealFrame,
}
#[derive(Debug,Clone)]
pub(crate) struct HandRevealFrame {
    /// Orders retained frames independently of a batch's object-vector order.
    frozen_revision:u64,
    hands:Vec<(PlayerId,Vec<ObjectSnapshot>)>,
    opportunities:Vec<Opportunity>,
    /// The event-time world validates all captured filters, including public
    /// cards, after grantors/modifiers depart. Peers can import authenticated
    /// hidden definitions here. It contains no nested reveal windows.
    identity_world:Option<Box<GameState>>,
}
#[derive(Debug,Clone)]
struct Opportunity {
    card:ObjectSnapshot,player:PlayerId,method:AlternativeCastingMethod,
    intrinsic:Option<usize>,
    required_face_name:Option<String>,
    eligibility:Option<(crate::target::ObjectFilter,crate::filter::FilterContext)>,
}
fn opportunities(game:&GameState,hands:&[(PlayerId,Vec<ObjectSnapshot>)])->Result<Vec<Opportunity>,ExecutionError> {
    let mut result=Vec::new();
    for (player,cards) in hands {for card in cards {
        let Some(object)=game.object(card.object_id).filter(|object|object.zone==Zone::Hand && object.stable_id==card.stable_id && object.owner==*player) else{continue};
        let mut methods=object.alternative_casts.iter().cloned().enumerate().map(|(index,method)|(method,Some(index),None,None)).collect::<Vec<_>>();
        for grant in game.effect_store.grant_registry.potential_hand_reveal_grants(game,card.object_id,Zone::Hand,*player) {
            if let Grantable::AlternativeCast(method)=&grant.grantable {
                let source=grant.source.source_id();
                let context=crate::grant_registry::grant_filter_context(&game.filter_context_for(*player,None),&grant,game).with_source_snapshot(ObjectSnapshot::from_object_id(game,source));
                methods.push((method.clone(),None,grant.filter.clone().map(|filter|(filter,context)),grant.required_face_name.clone()));
            }
        }
        for (method,intrinsic,eligibility,required_face_name) in methods {
            if method.battlefield_instructions().hand_reveal_event.is_none() {continue;}
            // Freeze authority, not a decision against one proposal's event.
            // The completed simultaneous event may contain other matching
            // departures. Each distinct grant instance remains independent.
            result.push(Opportunity {card:ObjectSnapshot::from_object_id(game,card.object_id).unwrap_or_else(||card.clone()),player:*player,method:method.clone(),intrinsic,required_face_name:required_face_name.clone(),eligibility:eligibility.clone()});
        }
    }}
    Ok(result)
}
fn expand_occurrences(game:&GameState,event:&TriggerEvent,offers:Vec<Opportunity>)->Result<Vec<Opportunity>,ExecutionError> {
    let mut expanded=Vec::new();
    for offer in offers {
        let Some(model)=offer.method.battlefield_instructions().hand_reveal_event else{continue};
        let matcher=Trigger::from_model(*model).map_err(|error|ExecutionError::Impossible(error.to_string()))?;
        let context=TriggerContext::new(offer.card.object_id,offer.player,crate::target::FilterContext::new(offer.player).with_source(offer.card.object_id).with_source_snapshot(Some(offer.card.clone())).with_active_player(game.turn.active_player),game);
        if matcher.matches(event,&context) {for _ in 0..matcher.trigger_count_with_context(event,&context) {expanded.push(offer.clone());}}
    }
    Ok(expanded)
}

/// Freeze before a battlefield departure is proposed. The packet is published
/// only if a real departure subsequently completes; a prevented proposal never
/// reveals a card. Grants lost in this same departure remain in this packet.
pub(crate) fn freeze_hand_reveal_frame(game:&GameState,_event:&TriggerEvent)->Result<HandRevealFrame,ExecutionError> {
    let hands=game.players.iter().map(|player|(player.id,player.hand.iter().filter_map(|id|ObjectSnapshot::from_object_id(game,*id)).collect::<Vec<_>>())).collect::<Vec<_>>();
    let opportunities=opportunities(game,&hands)?;
    let identity_world=if !opportunities.is_empty() || hands.iter().any(|(_,cards)|cards.iter().any(|card|game.hidden_identity_is_private(card.object_id))) {
        let mut world=game.clone();
        world.effect_store.pending_hand_reveal_windows.clear();
        world.effect_store.departure_hand_reveal_frames.clear();
        world.effect_store.hand_reveal_events_captured.clear();
        Some(Box::new(world))
    } else {None};
    Ok(HandRevealFrame {frozen_revision:game.mutation_revision(),hands,opportunities,identity_world})
}

/// Called by every canonical capture owner, including turn-based block events.
/// An empty captured hand is authoritative; cards acquired later cannot join it.
pub(crate) fn capture_hand_reveal_window(game:&mut GameState,event:&TriggerEvent)->Result<(),ExecutionError> {
    if game.action_observations_suppressed() {return Ok(());}
    // The shared reader currently admits these existing event operations.
    if !matches!(event.kind(),crate::events::EventKind::ZoneChange|crate::events::EventKind::CreatureBlocked|crate::events::EventKind::CreatureBecameBlocked) {return Ok(());}
    let turn=game.turn.turn_number;
    // Pointer identity is authoritative only while a retained Arc keeps its
    // occurrence alive. Retain each token for this turn, and discard old turns.
    game.effect_store.hand_reveal_events_captured.retain(|(captured_turn,_),_|*captured_turn==turn);
    let key=(turn,event.occurrence_key());
    if game.effect_store.hand_reveal_events_captured.insert(key,event.clone()).is_some() {return Ok(());}
    let frame=if let Some(change)=event.downcast::<crate::events::ZoneChangeEvent>() {
        let retained=change.objects.iter().filter_map(|object|game.effect_store.departure_hand_reveal_frames.get(object))
            .min_by_key(|frame|frame.frozen_revision).cloned();
        for object in &change.objects {game.effect_store.departure_hand_reveal_frames.remove(object);}
        match retained {Some(frame)=>frame,None=>freeze_hand_reveal_frame(game,event)?}
    } else {freeze_hand_reveal_frame(game,event)?};
    if !frame.opportunities.is_empty() || frame.hands.iter().any(|(_,cards)|cards.iter().any(|card|game.hidden_identity_is_private(card.object_id))) {
        game.effect_store.pending_hand_reveal_windows.push(HandRevealWindow {event:event.clone(),frame});
    }
    Ok(())
}
/// Replace unanswered singleton windows when canonical zone-event owners
/// regroup their completed occurrences. Keep the earliest pre-departure world
/// for the aggregate, including grants lost in the same simultaneous action.
/// Retain occurrence dedup tokens so aliases of retired events stay retired.
pub(crate) fn regroup_hand_reveal_windows(game:&mut GameState,retired:&[TriggerEvent]) {
    let keys=retired.iter().map(TriggerEvent::occurrence_key).collect::<std::collections::HashSet<_>>();
    let windows=std::mem::take(&mut game.effect_store.pending_hand_reveal_windows);
    for window in windows {
        if !keys.contains(&window.event.occurrence_key()) {
            game.effect_store.pending_hand_reveal_windows.push(window);
            continue;
        }
        if let Some(change)=window.event.downcast::<crate::events::ZoneChangeEvent>() {
            for object in &change.objects {
                let slot=game.effect_store.departure_hand_reveal_frames.entry(*object).or_insert_with(||window.frame.clone());
                if window.frame.frozen_revision<slot.frozen_revision {*slot=window.frame.clone();}
            }
        }
    }
}

pub(crate) fn answer_hand_reveal_windows(game:&mut GameState,ctx:&mut ExecutionContext)->Result<(),ExecutionError> {
    if game.action_observations_suppressed() || game.has_open_simultaneous_action() {return Ok(());}
    let windows=std::mem::take(&mut game.effect_store.pending_hand_reveal_windows);
    for mut window in windows {
        let mut opened=std::collections::HashSet::new();
        // Hidden identities are answered solely by their owners. A voluntarily
        // disclosed card acquires a rule receipt only after its typed matcher
        // validates this original event; disclosure itself fabricates no action.
        for (player,cards) in &window.frame.hands {
            let private=cards.iter().filter(|card|game.object(card.object_id).is_some_and(|object|object.zone==Zone::Hand && object.stable_id==card.stable_id) && game.hidden_identity_is_private(card.object_id)).map(|card|card.object_id).collect::<Vec<_>>();
            if !private.is_empty() {
                let Some(revealed)=game.reveal_private_hidden_cards_publicly(ctx.decision_maker,*player,ctx.source,&private,"You may reveal cards with a casting permission for this event",true) else{return Ok(())};
                opened.extend(revealed);
            }
        }
        if let Some(world)=window.frame.identity_world.as_mut() {
            for id in &opened {
                if world.is_hidden_card_placeholder(*id) {
                    let definition=game.object(*id).and_then(|object|object.card).and_then(|card|game.retained_card_definition(card)).cloned()
                        .ok_or_else(||ExecutionError::IncompleteEvidence("authenticated hand opening lacks its immutable definition".into()))?;
                    world.reveal_hidden_card_with_definition(*id,&definition).ok_or_else(||ExecutionError::IncompleteEvidence("event-time hand placeholder lost its authenticated identity".into()))?;
                }
            }
            world.refresh_continuous_state().map_err(ExecutionError::ContinuousDiscovery)?;
        }
        let event_world=window.frame.identity_world.as_deref().unwrap_or(game);
        let mut offers=window.frame.opportunities;
        for offer in &mut offers {
            if opened.contains(&offer.card.object_id) {
                offer.card=ObjectSnapshot::from_object_id(event_world,offer.card.object_id).ok_or_else(||ExecutionError::IncompleteEvidence("event-time opened card snapshot is missing".into()))?;
            }
        }
        if !opened.is_empty() {
            let opened_hands=window.frame.hands.iter().map(|(player,cards)|(*player,cards.iter().filter(|card|opened.contains(&card.object_id)).cloned().collect())).collect::<Vec<_>>();
            let new=opportunities(event_world,&opened_hands)?;
            let old=offers.iter().filter_map(|offer|offer.intrinsic.map(|index|(offer.card.object_id,index))).collect::<std::collections::HashSet<_>>();
            offers.extend(new.into_iter().filter(|offer|offer.intrinsic.is_some_and(|index|!old.contains(&(offer.card.object_id,index)))));
        }
        let offers=expand_occurrences(event_world,&window.event,offers)?;
        let offers=offers.into_iter().filter(|offer| {
            offer.required_face_name.as_ref().is_none_or(|face|offer.card.name.as_str()==face)
                && offer.eligibility.as_ref().is_none_or(|(filter,context)|filter.matches_snapshot(&offer.card,context,event_world))
        }).collect::<Vec<_>>();
        for offer in offers {
            if game.hidden_identity_is_private(offer.card.object_id) {continue;}
            let Some(object)=game.object(offer.card.object_id).filter(|object|object.zone==Zone::Hand && object.owner==offer.player && object.stable_id==offer.card.stable_id) else{continue};
            let name=object.name.to_string();
            {
                let question=crate::decisions::context::BooleanContext::new(offer.player,Some(offer.card.object_id),"Reveal this card to use its event-linked casting permission?").with_source_name(&name);
                let yes=ctx.decision_maker.decide_boolean(game,&question);
                if ctx.decision_maker.awaiting_choice() {return Ok(());}
                if !yes {continue;}
            }
            // Eligibility is event-time evidence; the reveal itself observes
            // the card's characteristics now, after any departure modifiers.
            let revealed_card=ObjectSnapshot::try_from_object_id(game,offer.card.object_id)?
                .ok_or_else(||ExecutionError::IncompleteEvidence("revealed hand incarnation disappeared".into()))?;
            // This rule reveal is an independent instruction, not a child
            // of the interrupted spell's discourse/cost/activation scope.
            let reveal={
                let mut reveal_ctx=ExecutionContext::new(offer.card.object_id,offer.player,&mut *ctx.decision_maker)
                    .with_provenance(window.event.provenance());
                reveal_ctx.source_snapshot=Some(revealed_card.clone());
                crate::effects::cards::reveal_objects_with_outputs(game,&mut reveal_ctx,vec![revealed_card.clone()],Some(offer.player),"Reveal a card for its event-linked casting permission",None)
            }?;
            if ctx.decision_maker.awaiting_choice() {return Ok(());}
            // A prevented/incomplete disclosure cannot create a cast trigger.
            let Some(revealed)=reveal.outcome.events.iter().find(|event|event.downcast::<crate::events::CardRevealedEvent>().is_some_and(|event|event.card==offer.card.object_id)).cloned() else{continue};
            let Some(cost)=offer.method.total_cost().cloned() else{return Err(ExecutionError::IncompleteEvidence("event-linked reveal price lacks complete cost".into()))};
            let tag=crate::tag::TagKey::from("hand_event_cast");
            let mut cast=crate::effects::CastTaggedEffect::new(tag.clone(),crate::target::PlayerFilter::You).from_zone(Zone::Hand).with_alternative_cost(cost);
            cast.alternative_cost_label=Some(offer.method.name().to_string());
            let ability=crate::ability::TriggeredAbility {
                trigger:Trigger::player_reveals_card(crate::target::PlayerFilter::You,crate::target::ObjectFilter::source(),true),
                effects:crate::resolution::ResolutionProgram::from_effects(vec![crate::Effect::new(crate::effects::MayEffect::new(vec![crate::Effect::new(cast)]))]),
                choices:Vec::new(),intervening_if:None,presentation_label:None,
            };
            let identity=crate::triggers::compute_trigger_identity(&ability);
            let entry=TriggeredAbilityEntry {linked_exile_owner:None,source_number_owner:None,source:offer.card.object_id,controller:offer.player,x_value:None,event_value_amount:None,
                ability,triggering_event:revealed,source_stable_id:offer.card.stable_id,source_name:name,source_snapshot:Some(revealed_card.clone()),
                tagged_objects:std::collections::HashMap::from([(tag,vec![revealed_card])]),source_kind:TriggeredAbilitySourceKind::Object,trigger_identity:identity};
            game.defer_trigger_entries(vec![entry]);
            for event in reveal.outcome.events {game.queue_trigger_event(event.provenance(),event);}
        }
    }
    Ok(())
}
