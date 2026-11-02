//! Match completed labelled operations using immutable event-time evidence.
use crate::events::{EventKind, NamedActionEvent};
use crate::filter::{ObjectFilterExt as _, player_filter_matches_game};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::{TriggerEvent, matcher_trait::{TriggerContext, TriggerMatcher}};
#[derive(Debug, Clone, PartialEq)]
pub struct NamedActionTrigger {
    pub label: String,
    pub player: PlayerFilter,
    pub source_filter: ObjectFilter,
    pub object_filter: Option<ObjectFilter>,
    pub minimum_amount: u32,
}
impl TriggerMatcher for NamedActionTrigger {
    fn canonical_model(&self) -> Option<ironsmith_core::trigger_model::Trigger> {
        Some(ironsmith_core::trigger_model::Trigger::named_action_with_minimum_amount(self.label.clone(), self.player.clone(), self.source_filter.clone(), self.object_filter.clone(), self.minimum_amount))
    }
    fn looks_back_for_source(&self, event: &TriggerEvent) -> bool {
        event.downcast::<NamedActionEvent>().is_some_and(|action| action.complete_source_lookback)
    }
    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> { Some(vec![EventKind::KeywordAction]) }
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        let Some(action) = event.downcast::<NamedActionEvent>() else { return false; };
        if action.amount < self.minimum_amount || action.label != self.label || !player_filter_matches_game(&self.player, action.player, ctx.game, &ctx.filter_ctx) { return false; }
        if action.player_only_performer && self.source_filter != ObjectFilter::default() { return false; }
        // The stable physical card may survive a move, but this incarnation's
        // source-local ability cannot observe a different incarnation acting.
        if self.source_filter.source && action.source != ctx.source_id { return false; }
        if !self.source_filter.matches_snapshot(&action.snapshot, &ctx.filter_ctx, ctx.game) { return false; }
        self.object_filter.as_ref().is_none_or(|filter| action.objects.iter().any(|object| filter.matches_snapshot(object, &ctx.filter_ctx, ctx.game)))
    }
    fn event_value_amount(&self, event: &TriggerEvent, _: &TriggerContext) -> Option<i32> {
        event.downcast::<NamedActionEvent>().map(|action| action.amount.min(i32::MAX as u32) as i32)
    }
    fn display(&self) -> String {
        let objects = self.object_filter.as_ref().map_or(String::new(), |filter| format!(" with {}", filter.description()));
        let (subject, verb) = if self.source_filter == ObjectFilter::default() {
            match self.player {
                PlayerFilter::You => ("you".to_string(), "perform"),
                PlayerFilter::Opponent => ("an opponent".to_string(), "performs"),
                _ => ("a player".to_string(), "performs"),
            }
        } else { (self.source_filter.description(), "performs") };
        let amount = if self.minimum_amount == 0 { String::new() } else { format!(" with an amount of {} or more", self.minimum_amount) };
        if self.source_filter != ObjectFilter::default() && self.player != PlayerFilter::Any {
            let (actor, actor_verb) = match self.player {
                PlayerFilter::You => ("you".to_string(), "perform"),
                PlayerFilter::Opponent => ("an opponent".to_string(), "performs"),
                _ => (self.player.description(), "performs"),
            };
            return format!("Whenever {actor} {actor_verb} the {} action{amount}{objects} from {}", self.label, self.source_filter.description());
        }
        format!("Whenever {subject} {verb} the {} action{amount}{}", self.label, objects)
    }
}

/// A labelled operation's affected-object role, distinct from its performer.
#[derive(Debug, Clone, PartialEq)]
pub struct NamedActionParticipantTrigger {
    pub label: String, pub player: PlayerFilter, pub participant_filter: ObjectFilter,
    pub performer_filter: Option<ObjectFilter>, pub one_or_more: bool,
}
impl TriggerMatcher for NamedActionParticipantTrigger {
    fn canonical_model(&self)->Option<ironsmith_core::trigger_model::Trigger>{Some(ironsmith_core::trigger_model::Trigger::named_action_participant(self.label.clone(),self.player.clone(),self.participant_filter.clone(),self.performer_filter.clone(),self.one_or_more))}
    fn looks_back_for_source(&self,event:&TriggerEvent)->bool{event.downcast::<NamedActionEvent>().is_some()}
    fn subscribed_kinds(&self)->Option<Vec<EventKind>>{
        // Zone changes capture the observer packet later consumed by an
        // affected-object LTB action; matching a raw move still returns false.
        Some(vec![EventKind::KeywordAction,EventKind::ZoneChange])
    }
    fn matches(&self,event:&TriggerEvent,ctx:&TriggerContext)->bool{
        let Some(action)=event.downcast::<NamedActionEvent>()else{return false;};
        if std::env::var_os("IRONSMITH_TRACE_NAMED_PARTICIPANTS").is_some() {
            eprintln!("named-participant match label={} expected={} source={:?} source_filter={} objects={:?} player_ok={}", action.label,self.label,ctx.source_id,self.participant_filter.source,action.objects.iter().take(8).map(|object|(object.object_id,object.zone,self.participant_filter.matches_snapshot(object,&ctx.filter_ctx,ctx.game))).collect::<Vec<_>>(),player_filter_matches_game(&self.player,action.player,ctx.game,&ctx.filter_ctx));
        }
        action.label==self.label && player_filter_matches_game(&self.player,action.player,ctx.game,&ctx.filter_ctx)
            && self.performer_filter.as_ref().is_none_or(|filter|!action.player_only_performer && filter.matches_snapshot(&action.snapshot,&ctx.filter_ctx,ctx.game))
            && action.objects.iter().any(|object|(!self.participant_filter.source||object.object_id==ctx.source_id)
                && self.participant_filter.matches_snapshot(object,&ctx.filter_ctx,ctx.game))
    }
    fn event_value_amount(&self,event:&TriggerEvent,ctx:&TriggerContext)->Option<i32>{event.downcast::<NamedActionEvent>().map(|action|action.objects.iter().filter(|object|(!self.participant_filter.source||object.object_id==ctx.source_id)&&self.participant_filter.matches_snapshot(object,&ctx.filter_ctx,ctx.game)).count().min(i32::MAX as usize)as i32)}
    fn simultaneous_trigger_key(&self,_:&TriggerEvent)->Option<crate::triggers::matcher_trait::SimultaneousTriggerKey>{self.one_or_more.then_some(crate::triggers::matcher_trait::SimultaneousTriggerKey::NamedActionBatch)}
    fn display(&self)->String{format!("Whenever {} participates in the {} action",self.participant_filter.description(),self.label)}
}
