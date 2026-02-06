use std::collections::HashMap;

use crate::decision::{DecisionMaker, LegalAction, compute_legal_actions};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};

use super::state::{verify_action_propose_for_game, verify_contrib_request_for_game};
use super::{
    ActionKind, ActionPayload, ActionPropose, ContribRequest, CostSpec, Hash32, PeerId, PubKey,
    TargetSpec,
};

/// Incoming network messages consumed by the runtime.
#[derive(Debug, Clone)]
pub enum InboundMessage {
    Action {
        peer: PeerId,
        propose: ActionPropose,
    },
    ContribRequest {
        peer: PeerId,
        request: ContribRequest,
    },
}

/// Synchronous inbox for inbound network messages.
pub trait Inbox {
    /// Blocking receive for the next inbound message.
    fn recv(&mut self) -> InboundMessage;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetRuntimeError {
    UnknownPeer(PeerId),
    UnknownPlayer(PlayerId),
    UnexpectedPeer {
        expected: PeerId,
        got: PeerId,
    },
    PrevStateMismatch {
        expected: Hash32,
        found: Hash32,
    },
    InvalidSignature,
    UnsupportedAction(ActionKind),
    NoMatchingLegalAction {
        player: PlayerId,
        action: ActionKind,
    },
    AmbiguousLegalAction {
        player: PlayerId,
        action: ActionKind,
        count: usize,
    },
    MissingTargets(ActionKind),
    MissingXValue,
    OptionalCostRepeatUnsupported {
        index: usize,
        count: u32,
    },
    UnhandledDecision(&'static str),
    ActionIncomplete(ActionKind),
}

#[derive(Debug, Clone)]
struct PendingAction {
    player: PlayerId,
    action: ActionPayload,
    costs: Option<CostSpec>,
    targets: Vec<Target>,
    consumed_targets: bool,
    consumed_x: bool,
    consumed_optional_costs: bool,
}

impl PendingAction {
    fn kind(&self) -> ActionKind {
        action_kind(&self.action)
    }

    fn expects_targets(&self) -> bool {
        matches!(
            self.action,
            ActionPayload::CastSpell { .. } | ActionPayload::ActivateAbility { .. }
        ) && !self.targets.is_empty()
    }

    fn expects_x(&self) -> bool {
        self.costs.as_ref().and_then(|c| c.x_value).is_some()
    }

    fn expects_optional_costs(&self) -> bool {
        self.costs
            .as_ref()
            .is_some_and(|c| c.optional_costs.iter().any(|n| *n > 0))
    }

    fn is_complete(&self) -> bool {
        (!self.expects_targets() || self.consumed_targets)
            && (!self.expects_x() || self.consumed_x)
            && (!self.expects_optional_costs() || self.consumed_optional_costs)
    }
}

/// Runtime that bridges inbound network actions to the DecisionMaker interface.
///
/// This verifies signatures + prev_state_hash for inbound actions and then
/// maps them to local DecisionMaker responses. Unsupported decision prompts
/// are delegated to a fallback DecisionMaker and recorded as errors.
pub struct NetRuntime<I: Inbox> {
    inbox: I,
    peer_for_player: HashMap<PlayerId, PeerId>,
    pubkeys: HashMap<PeerId, PubKey>,
    pending: Option<PendingAction>,
    errors: Vec<NetRuntimeError>,
    contrib_requests: Vec<(PeerId, ContribRequest)>,
    fallback: Box<dyn DecisionMaker>,
}

impl<I: Inbox> NetRuntime<I> {
    pub fn new(
        inbox: I,
        peer_for_player: HashMap<PlayerId, PeerId>,
        pubkeys: HashMap<PeerId, PubKey>,
    ) -> Self {
        Self::with_fallback(
            inbox,
            peer_for_player,
            pubkeys,
            Box::new(crate::decision::AutoPassDecisionMaker),
        )
    }

    pub fn with_fallback(
        inbox: I,
        peer_for_player: HashMap<PlayerId, PeerId>,
        pubkeys: HashMap<PeerId, PubKey>,
        fallback: Box<dyn DecisionMaker>,
    ) -> Self {
        Self {
            inbox,
            peer_for_player,
            pubkeys,
            pending: None,
            errors: Vec::new(),
            contrib_requests: Vec::new(),
            fallback,
        }
    }

    pub fn take_errors(&mut self) -> Vec<NetRuntimeError> {
        std::mem::take(&mut self.errors)
    }

    pub fn drain_contrib_requests(&mut self) -> Vec<(PeerId, ContribRequest)> {
        std::mem::take(&mut self.contrib_requests)
    }

    fn record_error(&mut self, error: NetRuntimeError) {
        self.errors.push(error);
    }

    fn expected_peer(&self, player: PlayerId) -> Option<PeerId> {
        self.peer_for_player.get(&player).copied()
    }

    fn clear_pending_if_complete(&mut self) {
        if let Some(pending) = &self.pending {
            if !pending.is_complete() {
                self.record_error(NetRuntimeError::ActionIncomplete(pending.kind()));
            }
        }
        self.pending = None;
    }

    fn ensure_action_for(
        &mut self,
        game: &GameState,
        player: PlayerId,
    ) -> Option<&mut PendingAction> {
        let controller = game.controlling_player_for(player);

        if let Some(pending) = &self.pending {
            if pending.player == controller {
                return self.pending.as_mut();
            }
        }

        if self.pending.is_some() {
            self.clear_pending_if_complete();
        }

        let expected_peer = match self.expected_peer(controller) {
            Some(peer) => peer,
            None => {
                self.record_error(NetRuntimeError::UnknownPlayer(controller));
                return None;
            }
        };

        loop {
            match self.inbox.recv() {
                InboundMessage::ContribRequest { peer, request } => {
                    let Some(pubkey) = self.pubkeys.get(&peer).copied() else {
                        self.record_error(NetRuntimeError::UnknownPeer(peer));
                        continue;
                    };
                    match verify_contrib_request_for_game(game, pubkey, &request) {
                        Ok(()) => self.contrib_requests.push((peer, request)),
                        Err(err) => self.record_error(map_contrib_error(err)),
                    }
                }
                InboundMessage::Action { peer, propose } => {
                    if peer != expected_peer {
                        self.record_error(NetRuntimeError::UnexpectedPeer {
                            expected: expected_peer,
                            got: peer,
                        });
                        continue;
                    }

                    let Some(pubkey) = self.pubkeys.get(&peer).copied() else {
                        self.record_error(NetRuntimeError::UnknownPeer(peer));
                        continue;
                    };

                    match verify_action_propose_for_game(game, pubkey, &propose) {
                        Ok(()) => {
                            let pending = pending_from_action(controller, propose.action);
                            self.pending = Some(pending);
                            return self.pending.as_mut();
                        }
                        Err(err) => {
                            self.record_error(map_action_error(err));
                            continue;
                        }
                    }
                }
            }
        }
    }

    fn select_legal_action(
        &mut self,
        game: &GameState,
        player: PlayerId,
        payload: &ActionPayload,
    ) -> Option<LegalAction> {
        match payload {
            ActionPayload::PassPriority { .. } => return Some(LegalAction::PassPriority),
            ActionPayload::CastSpell { card_ref, .. } => {
                let spell_id = ObjectId::from_raw(card_ref.0);
                let matches: Vec<LegalAction> = compute_legal_actions(game, player)
                    .into_iter()
                    .filter(|action| matches!(action, LegalAction::CastSpell { spell_id: id, .. } if *id == spell_id))
                    .collect();
                return match matches.len() {
                    1 => Some(matches[0].clone()),
                    0 => {
                        self.record_error(NetRuntimeError::NoMatchingLegalAction {
                            player,
                            action: ActionKind::CastSpell,
                        });
                        None
                    }
                    count => {
                        self.record_error(NetRuntimeError::AmbiguousLegalAction {
                            player,
                            action: ActionKind::CastSpell,
                            count,
                        });
                        None
                    }
                };
            }
            ActionPayload::ActivateAbility { source_ref, .. } => {
                let source_id = ObjectId::from_raw(source_ref.0);
                let matches: Vec<LegalAction> = compute_legal_actions(game, player)
                    .into_iter()
                    .filter(|action| match action {
                        LegalAction::ActivateAbility { source, .. } => *source == source_id,
                        LegalAction::ActivateManaAbility { source, .. } => *source == source_id,
                        _ => false,
                    })
                    .collect();
                return match matches.len() {
                    1 => Some(matches[0].clone()),
                    0 => {
                        self.record_error(NetRuntimeError::NoMatchingLegalAction {
                            player,
                            action: ActionKind::ActivateAbility,
                        });
                        None
                    }
                    count => {
                        self.record_error(NetRuntimeError::AmbiguousLegalAction {
                            player,
                            action: ActionKind::ActivateAbility,
                            count,
                        });
                        None
                    }
                };
            }
            _ => {
                self.record_error(NetRuntimeError::UnsupportedAction(action_kind(payload)));
                None
            }
        }
    }
}

impl<I: Inbox> DecisionMaker for NetRuntime<I> {
    fn on_action_cancelled(&mut self, _game: &GameState, _reason: &str) {
        self.clear_pending_if_complete();
    }

    fn decide_priority(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::PriorityContext,
    ) -> LegalAction {
        let action_payload = match self.ensure_action_for(game, ctx.player) {
            Some(pending) => pending.action.clone(),
            None => return self.fallback.decide_priority(game, ctx),
        };

        let action = match self.select_legal_action(game, ctx.player, &action_payload) {
            Some(action) => action,
            None => return self.fallback.decide_priority(game, ctx),
        };

        if matches!(action_payload, ActionPayload::PassPriority { .. }) {
            self.pending = None;
        }

        action
    }

    fn decide_targets(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        let (targets, kind, missing) = match self.ensure_action_for(game, ctx.player) {
            Some(pending) => {
                let missing = pending.targets.is_empty();
                if !missing {
                    pending.consumed_targets = true;
                }
                (pending.targets.clone(), pending.kind(), missing)
            }
            None => return self.fallback.decide_targets(game, ctx),
        };

        if missing {
            self.record_error(NetRuntimeError::MissingTargets(kind));
            return self.fallback.decide_targets(game, ctx);
        }

        targets
    }

    fn decide_number(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::NumberContext,
    ) -> u32 {
        if !ctx.is_x_value {
            self.record_error(NetRuntimeError::UnhandledDecision("number"));
            return self.fallback.decide_number(game, ctx);
        }
        let (x_value, has_x) = match self.ensure_action_for(game, ctx.player) {
            Some(pending) => {
                let x_value = pending.costs.as_ref().and_then(|c| c.x_value);
                if x_value.is_some() {
                    pending.consumed_x = true;
                }
                (x_value.unwrap_or(ctx.min), x_value.is_some())
            }
            None => return self.fallback.decide_number(game, ctx),
        };

        if !has_x {
            self.record_error(NetRuntimeError::MissingXValue);
            return self.fallback.decide_number(game, ctx);
        }

        x_value
    }

    fn decide_options(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        if ctx.description.contains("optional cost") {
            let (indices_opt, repeat_errors) = match self.ensure_action_for(game, ctx.player) {
                Some(pending) => {
                    if let Some(costs) = pending.costs.as_ref() {
                        let mut indices = Vec::new();
                        let mut repeat_errors = Vec::new();
                        for (index, count) in costs.optional_costs.iter().enumerate() {
                            if *count > 0 {
                                if *count > 1 {
                                    repeat_errors.push(
                                        NetRuntimeError::OptionalCostRepeatUnsupported {
                                            index,
                                            count: *count,
                                        },
                                    );
                                }
                                indices.push(index);
                            }
                        }
                        pending.consumed_optional_costs = true;
                        (Some(indices), repeat_errors)
                    } else {
                        (None, Vec::new())
                    }
                }
                None => return self.fallback.decide_options(game, ctx),
            };
            let Some(indices) = indices_opt else {
                self.record_error(NetRuntimeError::UnhandledDecision("optional costs"));
                return self.fallback.decide_options(game, ctx);
            };
            for err in repeat_errors {
                self.record_error(err);
            }
            return indices;
        }

        self.record_error(NetRuntimeError::UnhandledDecision("options"));
        self.fallback.decide_options(game, ctx)
    }
}

fn action_kind(action: &ActionPayload) -> ActionKind {
    match action {
        ActionPayload::PassPriority { .. } => ActionKind::PassPriority,
        ActionPayload::CastSpell { .. } => ActionKind::CastSpell,
        ActionPayload::ActivateAbility { .. } => ActionKind::ActivateAbility,
        ActionPayload::ResolveTop => ActionKind::ResolveTop,
        ActionPayload::DrawCard { .. } => ActionKind::DrawCard,
        ActionPayload::ShuffleLibrary => ActionKind::ShuffleLibrary,
        ActionPayload::SearchLibrary { .. } => ActionKind::SearchLibrary,
        ActionPayload::ReorderTopN { .. } => ActionKind::ReorderTopN,
        ActionPayload::MoveCard { .. } => ActionKind::MoveCard,
        ActionPayload::RevealCard { .. } => ActionKind::RevealCard,
    }
}

fn pending_from_action(player: PlayerId, action: ActionPayload) -> PendingAction {
    let (costs, targets, consumed_targets, consumed_x, consumed_optional_costs) = match &action {
        ActionPayload::CastSpell { targets, costs, .. }
        | ActionPayload::ActivateAbility { targets, costs, .. } => (
            Some(costs.clone()),
            targets_from_specs(targets),
            false,
            false,
            false,
        ),
        _ => (None, Vec::new(), true, true, true),
    };

    PendingAction {
        player,
        action,
        costs,
        targets,
        consumed_targets,
        consumed_x,
        consumed_optional_costs,
    }
}

fn targets_from_specs(specs: &[TargetSpec]) -> Vec<Target> {
    specs
        .iter()
        .map(|spec| match spec {
            TargetSpec::Object(obj) => Target::Object(ObjectId::from_raw(obj.0)),
            TargetSpec::Player(player) => Target::Player(PlayerId(player.0)),
        })
        .collect()
}

fn map_action_error(err: super::state::ActionVerifyError) -> NetRuntimeError {
    match err {
        super::state::ActionVerifyError::InvalidSignature => NetRuntimeError::InvalidSignature,
        super::state::ActionVerifyError::PrevState(state_err) => match state_err {
            super::state::StateRootError::Mismatch { expected, found } => {
                NetRuntimeError::PrevStateMismatch { expected, found }
            }
        },
    }
}

fn map_contrib_error(err: super::state::ContribVerifyError) -> NetRuntimeError {
    match err {
        super::state::ContribVerifyError::InvalidSignature => NetRuntimeError::InvalidSignature,
        super::state::ContribVerifyError::PrevState(state_err) => match state_err {
            super::state::StateRootError::Mismatch { expected, found } => {
                NetRuntimeError::PrevStateMismatch { expected, found }
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::GameState;
    use crate::ids::{PlayerId, reset_id_counters};
    use crate::net::crypto::Secp256k1Signer;
    use crate::net::message::build_action_propose_for_game;
    use crate::net::{ActionPayload, ProofBundle};
    use std::collections::VecDeque;

    struct QueueInbox {
        queue: VecDeque<InboundMessage>,
    }

    impl QueueInbox {
        fn new(queue: VecDeque<InboundMessage>) -> Self {
            Self { queue }
        }
    }

    impl Inbox for QueueInbox {
        fn recv(&mut self) -> InboundMessage {
            self.queue.pop_front().expect("inbox empty")
        }
    }

    fn peer_id(byte: u8) -> PeerId {
        PeerId([byte; 32])
    }

    #[test]
    fn net_runtime_accepts_pass_priority() {
        reset_id_counters();
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let signer = Secp256k1Signer::from_secret_bytes([7u8; 32]).expect("signer");
        let pubkey = signer.public_key();

        let propose = build_action_propose_for_game(
            &signer,
            &game,
            ActionPayload::PassPriority { policy_id: None },
            ProofBundle::default(),
            None,
        );

        let alice = PlayerId::from_index(0);
        let p_alice = peer_id(1);

        let inbox = QueueInbox::new(
            [InboundMessage::Action {
                peer: p_alice,
                propose,
            }]
            .into_iter()
            .collect(),
        );

        let mut peer_for_player = HashMap::new();
        peer_for_player.insert(alice, p_alice);

        let mut pubkeys = HashMap::new();
        pubkeys.insert(p_alice, pubkey);

        let mut runtime = NetRuntime::new(inbox, peer_for_player, pubkeys);
        let ctx = crate::decisions::context::PriorityContext::new(
            alice,
            compute_legal_actions(&game, alice),
            Vec::new(),
        );

        let action = runtime.decide_priority(&game, &ctx);
        assert!(matches!(action, LegalAction::PassPriority));
        assert!(runtime.take_errors().is_empty());
    }

    #[test]
    fn net_runtime_records_prev_state_mismatch() {
        reset_id_counters();
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let signer = Secp256k1Signer::from_secret_bytes([9u8; 32]).expect("signer");
        let pubkey = signer.public_key();

        let propose = build_action_propose_for_game(
            &signer,
            &game,
            ActionPayload::PassPriority { policy_id: None },
            ProofBundle::default(),
            None,
        );

        // Mutate game so prev_state_hash no longer matches.
        game.players[0].life -= 1;
        let propose_valid = build_action_propose_for_game(
            &signer,
            &game,
            ActionPayload::PassPriority { policy_id: None },
            ProofBundle::default(),
            None,
        );

        let alice = PlayerId::from_index(0);
        let p_alice = peer_id(2);

        let inbox = QueueInbox::new(
            [
                InboundMessage::Action {
                    peer: p_alice,
                    propose,
                },
                InboundMessage::Action {
                    peer: p_alice,
                    propose: propose_valid,
                },
            ]
            .into_iter()
            .collect(),
        );

        let mut peer_for_player = HashMap::new();
        peer_for_player.insert(alice, p_alice);

        let mut pubkeys = HashMap::new();
        pubkeys.insert(p_alice, pubkey);

        let mut runtime = NetRuntime::new(inbox, peer_for_player, pubkeys);
        let ctx = crate::decisions::context::PriorityContext::new(
            alice,
            compute_legal_actions(&game, alice),
            Vec::new(),
        );

        let _ = runtime.decide_priority(&game, &ctx);
        let errors = runtime.take_errors();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, NetRuntimeError::PrevStateMismatch { .. })),
            "expected prev_state mismatch error, got {:?}",
            errors
        );
    }
}
