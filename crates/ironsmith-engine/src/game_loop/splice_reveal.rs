//! Replay-safe source reveal offers during the ordinary splice announcement.
use super::*;
use crate::decisions::context::{DecisionContext, BooleanContext};
use crate::effects::cards::RevealedCardActionOffer;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub(crate) enum SpliceOfferAnswer {
    Boolean(bool),
    Number(u32),
    Text(String),
    Objects(Vec<ObjectId>),
    Options(Vec<usize>),
    Order(Vec<ObjectId>),
    Colors(Vec<crate::color::Color>),
    Counters(Vec<(crate::object::CounterType, u32)>),
    Partition(Vec<ObjectId>),
    Proliferate(crate::decisions::specs::ProliferateResponse),
    Targets(Vec<crate::game_state::Target>),
    Distribute(Vec<(crate::game_state::Target, u32)>),
    Priority(crate::decision::LegalAction),
    Attackers(Vec<crate::decisions::spec::AttackerDeclaration>),
    Blockers(Vec<crate::decisions::spec::BlockerDeclaration>),
    ManaPayment(crate::mana_payment::ManaPaymentResponse),
}

#[derive(Debug, Clone)]
struct SpliceOfferDecisionMaker {
    capture_mana_payment: bool,
    answers: Vec<SpliceOfferAnswer>,
    next: usize,
    pending_prompt: Option<DecisionContext>,
}

impl SpliceOfferDecisionMaker {
    fn new(answers: Vec<SpliceOfferAnswer>) -> Self {
        Self {
            capture_mana_payment: true,
            answers,
            next: 0,
            pending_prompt: None,
        }
    }
    fn next_answer(&mut self) -> Option<SpliceOfferAnswer> {
        let answer = self.answers.get(self.next).cloned();
        self.next += 1;
        answer
    }
}

impl DecisionMaker for SpliceOfferDecisionMaker {
    fn decide_priority(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::PriorityContext,
    ) -> crate::decision::LegalAction {
        match self.next_answer() {
            Some(SpliceOfferAnswer::Priority(answer)) => answer,
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::Priority(ctx.clone()));
                crate::decision::LegalAction::PassPriority
            }
        }
    }

    fn decide_attackers(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::AttackersContext,
    ) -> Vec<crate::decisions::spec::AttackerDeclaration> {
        match self.next_answer() {
            Some(SpliceOfferAnswer::Attackers(answer)) => answer,
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::Attackers(ctx.clone()));
                Vec::new()
            }
        }
    }

    fn decide_blockers(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::BlockersContext,
    ) -> Vec<crate::decisions::spec::BlockerDeclaration> {
        match self.next_answer() {
            Some(SpliceOfferAnswer::Blockers(answer)) => answer,
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::Blockers(ctx.clone()));
                Vec::new()
            }
        }
    }

    fn decide_number(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::NumberContext,
    ) -> u32 {
        match self.next_answer() {
            Some(SpliceOfferAnswer::Number(answer)) => answer,
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::Number(ctx.clone()));
                ctx.min
            }
        }
    }

    fn decide_text(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::TextInputContext,
    ) -> String {
        match self.next_answer() {
            Some(SpliceOfferAnswer::Text(answer)) => answer,
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::TextInput(ctx.clone()));
                ctx.initial_value.clone().unwrap_or_default()
            }
        }
    }

    fn decide_colors(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::ColorsContext,
    ) -> Vec<crate::color::Color> {
        match self.next_answer() {
            Some(SpliceOfferAnswer::Colors(answer)) => answer,
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::Colors(ctx.clone()));
                Vec::new()
            }
        }
    }

    fn decide_counters(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::CountersContext,
    ) -> Vec<(crate::object::CounterType, u32)> {
        match self.next_answer() {
            Some(SpliceOfferAnswer::Counters(answer)) => answer,
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::Counters(ctx.clone()));
                Vec::new()
            }
        }
    }

    fn decide_partition(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::PartitionContext,
    ) -> Vec<ObjectId> {
        match self.next_answer() {
            Some(SpliceOfferAnswer::Partition(answer)) => answer,
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::Partition(ctx.clone()));
                Vec::new()
            }
        }
    }

    fn decide_proliferate(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::ProliferateContext,
    ) -> crate::decisions::specs::ProliferateResponse {
        match self.next_answer() {
            Some(SpliceOfferAnswer::Proliferate(answer)) => answer,
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::Proliferate(ctx.clone()));
                Default::default()
            }
        }
    }

    fn decide_targets(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<crate::game_state::Target> {
        match self.next_answer() {
            Some(SpliceOfferAnswer::Targets(answer)) => answer,
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::Targets(ctx.clone()));
                Vec::new()
            }
        }
    }

    fn decide_distribute(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::DistributeContext,
    ) -> Vec<(crate::game_state::Target, u32)> {
        match self.next_answer() {
            Some(SpliceOfferAnswer::Distribute(answer)) => answer,
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::Distribute(ctx.clone()));
                Vec::new()
            }
        }
    }

    fn decide_mana_payment(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::ManaPaymentContext,
    ) -> crate::mana_payment::ManaPaymentResponse {
        use crate::mana_payment::ManaPaymentResponse;
        if !self.capture_mana_payment {
            let options = crate::decisions::context::SelectOptionsContext::new(
                ctx.player,
                Some(ctx.source),
                format!("Confirm mana payment for {}", ctx.subject),
                vec![
                    crate::decisions::context::SelectableOption::new(1, "Confirm payment"),
                    crate::decisions::context::SelectableOption::new(0, "Cancel"),
                ],
                1,
                1,
            );
            return if self.decide_options(game, &options).first().copied() == Some(1) {
                ManaPaymentResponse::Confirm {
                    plan_id: ctx.plan.id,
                    request_hash: ctx.plan.request_hash,
                }
            } else {
                ManaPaymentResponse::Cancel
            };
        }
        match self.next_answer() {
            Some(SpliceOfferAnswer::ManaPayment(response)) => response,
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::ManaPayment(ctx.clone()));
                ManaPaymentResponse::Cancel
            }
        }
    }
    fn awaiting_choice(&self) -> bool {
        self.pending_prompt.is_some()
    }
    fn decide_boolean(&mut self, _game: &GameState, ctx: &BooleanContext) -> bool {
        match self.next_answer() {
            Some(SpliceOfferAnswer::Boolean(answer)) => answer,
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::Boolean(ctx.clone()));
                false
            }
        }
    }
    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        match self.next_answer() {
            Some(SpliceOfferAnswer::Objects(objects)) => objects,
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::SelectObjects(ctx.clone()));
                Vec::new()
            }
        }
    }
    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        match self.next_answer() {
            Some(SpliceOfferAnswer::Options(options)) => options,
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::SelectOptions(ctx.clone()));
                Vec::new()
            }
        }
    }
    fn decide_order(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::OrderContext,
    ) -> Vec<ObjectId> {
        // "In any order" (Abundance's bottom-of-library cards): the chooser
        // orders them (CR 401.4 / 608.2d); a trivial order needs no prompt.
        let default_order = || ctx.items.iter().map(|(id, _)| *id).collect::<Vec<_>>();
        if ctx.items.len() < 2 {
            return default_order();
        }
        match self.next_answer() {
            Some(SpliceOfferAnswer::Order(order)) => {
                let mut remaining = default_order();
                let mut ordered = Vec::with_capacity(remaining.len());
                for id in order {
                    if let Some(index) = remaining.iter().position(|item| *item == id) {
                        ordered.push(remaining.remove(index));
                    }
                }
                ordered.extend(remaining);
                ordered
            }
            _ => {
                self.pending_prompt
                    .get_or_insert_with(|| DecisionContext::Order(ctx.clone()));
                default_order()
            }
        }
    }
}


#[derive(Debug, Clone)]
struct SpliceRevealBoundary {
    event: crate::triggers::TriggerEvent,
    offers: Vec<RevealedCardActionOffer>,
    next: usize,
    answers: Vec<SpliceOfferAnswer>,
    prompt: Option<DecisionContext>,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingSpliceReveal {
    /// Original selection order, never reconstructed from a child response.
    pub(crate) selected: Vec<ObjectId>,
    pub(crate) additions: Vec<(crate::ids::StableId, crate::cost::TotalCost, crate::resolution::ResolutionProgram)>,
    added_abilities: Vec<crate::ability::Ability>,
    boundaries: VecDeque<SpliceRevealBoundary>,
}

impl PendingSpliceReveal {
    pub(super) fn new(selected: Vec<ObjectId>, additions: Vec<(crate::ids::StableId, crate::cost::TotalCost, crate::resolution::ResolutionProgram)>, added_abilities: Vec<crate::ability::Ability>) -> Self {
        Self {selected,additions,added_abilities,boundaries:VecDeque::new()}
    }
    pub(super) fn push_boundary(&mut self,event:crate::triggers::TriggerEvent,offers:Vec<RevealedCardActionOffer>) {
        self.boundaries.push_back(SpliceRevealBoundary{event,offers,next:0,answers:Vec::new(),prompt:None});
    }
}

pub(super) fn continue_reveal(
    game:&mut GameState,trigger_queue:&mut TriggerQueue,state:&mut PriorityLoopState,
    mut pending:PendingCast,decision_maker:&mut impl DecisionMaker,
)->Result<GameProgress,GameLoopError>{
    let mut replay=pending.splice_reveal.take().ok_or_else(||GameLoopError::InvalidState("No captured splice reveal".into()))?;
    while let Some(boundary)=replay.boundaries.front_mut(){
        while let Some(offer)=boundary.offers.get(boundary.next).cloned(){
            if !crate::effects::cards::reveal_action_offer_is_available(game,&offer){
                boundary.next+=1;boundary.answers.clear();boundary.prompt=None;continue;
            }
            if let Some(prompt)=boundary.prompt.clone(){
                pending.splice_reveal=Some(replay);state.pending_cast=Some(pending);
                return Ok(GameProgress::NeedsDecisionCtx(prompt));
            }
            // Child decisions replay only on a private world. Earlier
            // completed offers and the original splice reveal are never replayed.
            let mut hypothetical=game.clone();
            let mut decisions=SpliceOfferDecisionMaker::new(boundary.answers.clone());
            let accepted=decisions.decide_boolean(&hypothetical,&crate::effects::cards::reveal_action_offer_boolean_context(&offer));
            if decisions.pending_prompt.is_none(){
                crate::effects::cards::complete_reveal_action_offer(&mut hypothetical,&offer,accepted,&mut decisions)?;
            }
            if let Some(prompt)=decisions.pending_prompt.take(){
                boundary.prompt=Some(prompt.clone());
                pending.splice_reveal=Some(replay);state.pending_cast=Some(pending);
                return Ok(GameProgress::NeedsDecisionCtx(prompt));
            }
            *game=hypothetical;
            boundary.next+=1;boundary.answers.clear();boundary.prompt=None;
        }
        crate::effects::cards::finish_public_reveal_offer_boundary(game,&boundary.event);
        game.queue_trigger_event(pending.provenance,boundary.event.clone());
        replay.boundaries.pop_front();
    }
    super::priority_cast::finish_splice_response(game,trigger_queue,state,pending,replay.additions,replay.added_abilities,decision_maker)
}

fn read_answer(game:&GameState,ctx:&DecisionContext,dm:&mut impl DecisionMaker)->Result<SpliceOfferAnswer,GameLoopError>{
    Ok(match ctx {
        DecisionContext::Boolean(ctx)=>SpliceOfferAnswer::Boolean(dm.decide_boolean(game,ctx)),
        DecisionContext::Number(ctx)=>SpliceOfferAnswer::Number(dm.decide_number(game,ctx)),
        DecisionContext::TextInput(ctx)=>SpliceOfferAnswer::Text(dm.decide_text(game,ctx)),
        DecisionContext::SelectObjects(ctx)=>SpliceOfferAnswer::Objects(dm.decide_objects(game,ctx)),
        DecisionContext::SelectOptions(ctx)=>SpliceOfferAnswer::Options(dm.decide_options(game,ctx)),
        DecisionContext::Order(ctx)=>SpliceOfferAnswer::Order(dm.decide_order(game,ctx)),
        DecisionContext::Colors(ctx)=>SpliceOfferAnswer::Colors(dm.decide_colors(game,ctx)),
        DecisionContext::Counters(ctx)=>SpliceOfferAnswer::Counters(dm.decide_counters(game,ctx)),
        DecisionContext::Partition(ctx)=>SpliceOfferAnswer::Partition(dm.decide_partition(game,ctx)),
        DecisionContext::Proliferate(ctx)=>SpliceOfferAnswer::Proliferate(dm.decide_proliferate(game,ctx)),
        DecisionContext::Targets(ctx)=>SpliceOfferAnswer::Targets(dm.decide_targets(game,ctx)),
        DecisionContext::Distribute(ctx)=>SpliceOfferAnswer::Distribute(dm.decide_distribute(game,ctx)),
        DecisionContext::Priority(ctx)=>SpliceOfferAnswer::Priority(dm.decide_priority(game,ctx)),
        DecisionContext::Attackers(ctx)=>SpliceOfferAnswer::Attackers(dm.decide_attackers(game,ctx)),
        DecisionContext::Blockers(ctx)=>SpliceOfferAnswer::Blockers(dm.decide_blockers(game,ctx)),
        DecisionContext::ManaPayment(ctx)=>SpliceOfferAnswer::ManaPayment(dm.decide_mana_payment(game,ctx)),
        DecisionContext::Modes(_)|DecisionContext::HybridChoice(_)=>return Err(GameLoopError::InvalidState("Splice offer emitted an unadapted composite decision".into())),
    })
}

pub(super) fn apply_response(
    game:&mut GameState,trigger_queue:&mut TriggerQueue,state:&mut PriorityLoopState,
    requested:&DecisionContext,decision_maker:&mut impl DecisionMaker,
)->Result<GameProgress,GameLoopError>{
    let mut pending=state.pending_cast.take().ok_or_else(||GameLoopError::InvalidState("No pending splice response".into()))?;
    let Some(boundary)=pending.splice_reveal.as_mut().and_then(|replay|replay.boundaries.front_mut())else{
        state.pending_cast=Some(pending);return Err(GameLoopError::InvalidState("No pending splice reveal boundary".into()));
    };
    let Some(prompt)=boundary.prompt.clone()else{
        state.pending_cast=Some(pending);return Err(GameLoopError::InvalidState("No pending splice reveal decision".into()));
    };
    if std::mem::discriminant(&prompt)!=std::mem::discriminant(requested)
        || prompt.player()!=requested.player() || prompt.source()!=requested.source()
        || prompt.prompt_text()!=requested.prompt_text(){
        state.pending_cast=Some(pending);return Err(GameLoopError::InvalidState("Response does not match the pending splice reveal decision".into()));
    }
    // Read through the authoritative saved context, never caller-supplied
    // options. The ordinary instruction validates the selected response on replay.
    let answer=match read_answer(game,&prompt,decision_maker){
        Ok(answer)=>answer,
        Err(error)=>{state.pending_cast=Some(pending);return Err(error);}
    };
    if decision_maker.awaiting_choice(){
        state.pending_cast=Some(pending);return Ok(GameProgress::NeedsDecisionCtx(prompt));
    }
    boundary.answers.push(answer);boundary.prompt=None;
    continue_reveal(game,trigger_queue,state,pending,decision_maker)
}

#[cfg(test)]
mod tests;
