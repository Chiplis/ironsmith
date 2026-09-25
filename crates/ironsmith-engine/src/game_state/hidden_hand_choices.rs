//! Choices among cards in a hidden hand in peer (mental-poker) matches.
//!
//! Every peer runs its own engine. The owner of a hidden hand card knows its
//! identity; the other peers hold a "Hidden Card" placeholder with no
//! characteristics. A choice such as "put a creature card from your hand onto
//! the battlefield" therefore sees real matches on the owner and nothing on the
//! other peers. Anything decided from that filter result — whether a prompt
//! exists, its bounds, auto-picking a lone candidate, skipping an empty choice —
//! desyncs the positional decision stream.
//!
//! The rule used here mirrors hidden library searches:
//!
//! * Whether a choice "depends on hidden identity" is judged only from facts
//!   every peer shares: the filter states a quality (CR 701.19b) and the hand
//!   holds cards tracked by the mental-poker layer (`hidden_card_info`, which is
//!   created for every dealt card on every peer and survives the owner's
//!   private opening).
//! * Such a choice is always offered, may be completed partially, and is never
//!   auto-picked or skipped because of the local filter result.
//! * Peers offer placeholders (whose filter result they cannot evaluate) next to
//!   the cards they know match. A chosen placeholder is recorded as an
//!   obligation and checked against the filter once its identity is opened
//!   (the chosen card normally becomes public right away: it moves to a public
//!   zone or is revealed, and the peer front end opens command-referenced cards
//!   before replaying the command, so the choice is validated by the replay
//!   itself). A cheating choice of a non-matching card is rejected either way.

use super::GameState;
use crate::filter::{FilterContext, ObjectFilter, ObjectFilterExt as _};
use crate::ids::{ObjectId, PlayerId, StableId};
use crate::zone::Zone;

/// A pending check that a card chosen while it was a hidden placeholder
/// actually satisfied the choice's filter.
#[derive(Debug, Clone)]
pub(crate) struct HiddenIdentityObligation {
    pub(crate) stable_id: StableId,
    /// The zone the card was chosen from; the check evaluates it there.
    pub(crate) zone: Zone,
    pub(crate) filter: ObjectFilter,
    pub(crate) filter_ctx: FilterContext,
    pub(crate) description: String,
}

/// Whether `filter` states a quality of the card itself (CR 701.19b) rather
/// than naming objects already known to every peer (specific or tagged
/// objects, the source).
fn filter_depends_on_card_identity(filter: &ObjectFilter) -> bool {
    filter.has_search_stated_quality()
        && filter.specific.is_none()
        && !filter.source
        && filter.tagged_constraints.is_empty()
        && !filter
            .any_of
            .iter()
            .any(|branch| branch.specific.is_some() || !branch.tagged_constraints.is_empty())
}

/// The part of `filter` that does not depend on card identity.
pub(crate) fn identity_free_filter(filter: &ObjectFilter) -> ObjectFilter {
    let mut generic = ObjectFilter::default();
    generic.zone = filter.zone;
    generic.owner = filter.owner.clone();
    generic.controller = filter.controller.clone();
    generic
}

impl GameState {
    /// Whether a hand card's identity is tracked by the mental-poker layer,
    /// i.e. it is (or was) hidden from at least one peer. This is the same on
    /// every peer, unlike "do I know this card".
    pub(crate) fn is_hidden_tracked_hand_card(&self, id: ObjectId) -> bool {
        self.hidden_card_info(id).is_some()
            && self
                .object(id)
                .is_some_and(|object| object.zone == Zone::Hand)
    }

    /// Whether choosing hand cards with `filter` among `hand_ids` depends on
    /// identities that some peer cannot see. Symmetric across peers.
    pub(crate) fn hand_choice_depends_on_hidden_identity(
        &self,
        filter: &ObjectFilter,
        hand_ids: impl IntoIterator<Item = ObjectId>,
    ) -> bool {
        filter_depends_on_card_identity(filter)
            && hand_ids
                .into_iter()
                .any(|id| self.is_hidden_tracked_hand_card(id))
    }

    /// Whether a choice of hand cards with `filter` (any player's hand, the
    /// filter's owner/controller restricting it) depends on hidden identity.
    /// Symmetric across peers.
    pub(crate) fn hidden_hand_choice_for_filter(
        &self,
        filter: &ObjectFilter,
        filter_ctx: &FilterContext,
    ) -> bool {
        if filter.zone != Some(Zone::Hand) || !filter_depends_on_card_identity(filter) {
            return false;
        }
        let generic = identity_free_filter(filter);
        self.players
            .iter()
            .flat_map(|player| player.hand.iter().copied())
            .filter(|id| self.is_hidden_tracked_hand_card(*id))
            .any(|id| {
                self.object(id)
                    .is_some_and(|object| generic.matches(object, filter_ctx, self))
            })
    }

    /// Every player's hand cards, for callers that restrict by filter.
    pub(crate) fn all_hand_card_ids(&self) -> Vec<ObjectId> {
        self.players
            .iter()
            .flat_map(|player| player.hand.iter().copied())
            .collect()
    }

    /// Placeholders among `hand_ids` that pass the identity-free part of
    /// `filter`. Their full filter result is unknown locally, so they stay
    /// choosable (the owner, who knows them, never sees placeholders).
    pub(crate) fn hidden_hand_placeholder_candidates(
        &self,
        filter: &ObjectFilter,
        filter_ctx: &FilterContext,
        hand_ids: impl IntoIterator<Item = ObjectId>,
    ) -> Vec<ObjectId> {
        let generic = identity_free_filter(filter);
        hand_ids
            .into_iter()
            .filter(|id| self.is_hidden_card_placeholder(*id))
            .filter(|id| {
                self.object(*id).is_some_and(|object| {
                    object.zone == Zone::Hand && generic.matches(object, filter_ctx, self)
                })
            })
            .collect()
    }

    /// Record that `chosen` placeholders must satisfy `filter` once opened.
    pub(crate) fn record_hidden_identity_obligations(
        &mut self,
        chosen: &[ObjectId],
        filter: &ObjectFilter,
        filter_ctx: &FilterContext,
        description: &str,
    ) {
        if !filter.has_search_stated_quality() {
            return;
        }
        let obligations: Vec<_> = chosen
            .iter()
            .filter(|id| self.is_hidden_card_placeholder(**id))
            .filter_map(|id| self.object(*id))
            .map(|object| HiddenIdentityObligation {
                stable_id: object.stable_id,
                zone: object.zone,
                filter: filter.clone(),
                filter_ctx: filter_ctx.clone(),
                description: description.to_string(),
            })
            .collect();
        if obligations.is_empty() {
            return;
        }
        self.auxiliary_tracking_mut()
            .hidden_identity_obligations
            .extend(obligations);
    }

    /// Check a hidden card about to be opened with `def` against the choices
    /// made while it was a placeholder. Returns a description of the first
    /// violated choice. Does not change state.
    pub fn hidden_identity_obligation_violation(
        &self,
        id: ObjectId,
        def: &crate::cards::CardDefinition,
    ) -> Option<String> {
        let object = self.object(id)?;
        let stable_id = object.stable_id;
        let obligations = &self.auxiliary_tracking.hidden_identity_obligations;
        if !obligations
            .iter()
            .any(|obligation| obligation.stable_id == stable_id)
        {
            return None;
        }
        let mut opened = object.clone();
        opened.face_down_cast_state = None;
        opened.apply_card_definition_with_shared(
            def,
            &crate::object::CardSharedHandles::from_definition(def),
        );
        obligations
            .iter()
            .filter(|obligation| obligation.stable_id == stable_id)
            .find_map(|obligation| {
                let mut candidate = opened.clone();
                candidate.zone = obligation.zone;
                (!obligation
                    .filter
                    .matches_non_recursive(&candidate, &obligation.filter_ctx, self))
                .then(|| {
                    format!(
                        "{} does not satisfy the hidden choice \"{}\"",
                        def.card.name, obligation.description
                    )
                })
            })
    }

    /// Drop the obligations of a card whose identity has now been opened.
    pub(crate) fn clear_hidden_identity_obligations(&mut self, id: ObjectId) {
        let Some(stable_id) = self.object(id).map(|object| object.stable_id) else {
            return;
        };
        if self
            .auxiliary_tracking
            .hidden_identity_obligations
            .iter()
            .any(|obligation| obligation.stable_id == stable_id)
        {
            self.auxiliary_tracking_mut()
                .hidden_identity_obligations
                .retain(|obligation| obligation.stable_id != stable_id);
        }
    }
}

// ============================================================================
// Owner-answered public reveals of hidden hand cards
// ============================================================================
//
// Some rules consult a hand card's identity at the moment it moves or is
// drawn: Madness replaces where a discarded card goes (CR 702.35a), "when you
// discard this card" / "when an opponent causes you to discard this card"
// abilities trigger from the discarded card, and Miracle triggers only if the
// owner reveals the card as it is drawn (CR 702.94a). On peers that hold a
// placeholder those checks see nothing while the owner's engine sees the real
// card, so the owner alone would apply the replacement or put the trigger on
// the stack.
//
// The fix is a sequenced, owner-answered reveal: before the identity-dependent
// step, every peer asks the owner the same select-objects question about the
// same (identity-free) cards. The decision's selection reveal policy is
// `Public`, and the peer front end opens every publicly revealed selected card
// on every peer *before* it replays the owner's answer. Once the answer is
// replayed, all engines know the chosen identities and continue identically.
// Which cards were revealed this way is recorded here, identically on every
// peer, so hand-functioning triggers of those cards may be checked again.

impl GameState {
    /// Whether every peer learned this hidden-tracked card's identity through
    /// an owner-answered public reveal. Symmetric across peers.
    pub fn is_publicly_revealed_hidden_card(&self, id: ObjectId) -> bool {
        self.auxiliary_tracking
            .publicly_revealed_hidden_cards
            .contains(&id)
    }

    /// Whether the identity of this hidden-zone card is known to its owner
    /// only (some peer holds a placeholder for it). Symmetric across peers.
    pub(crate) fn hidden_identity_is_private(&self, id: ObjectId) -> bool {
        self.hidden_card_info(id).is_some()
            && !self.is_publicly_revealed_hidden_card(id)
            && self
                .object(id)
                .is_some_and(|object| object.zone.is_hidden())
    }

    /// Record that `ids` were opened publicly on every peer. Only cards the
    /// mental-poker layer tracks are recorded.
    pub(crate) fn mark_hidden_cards_publicly_revealed(&mut self, ids: &[ObjectId]) {
        let tracked: Vec<ObjectId> = ids
            .iter()
            .copied()
            .filter(|id| self.hidden_card_info(*id).is_some())
            .filter(|id| !self.is_publicly_revealed_hidden_card(*id))
            .collect();
        if tracked.is_empty() {
            return;
        }
        self.auxiliary_tracking_mut()
            .publicly_revealed_hidden_cards
            .extend(tracked);
    }

    /// Forget the public-reveal mark of an object that left its zone.
    pub(crate) fn forget_public_hidden_card_reveal(&mut self, id: ObjectId) {
        if self.is_publicly_revealed_hidden_card(id) {
            self.auxiliary_tracking_mut()
                .publicly_revealed_hidden_cards
                .remove(&id);
        }
        if self
            .auxiliary_tracking
            .pending_hidden_draw_reveals
            .iter()
            .any(|(_, card)| *card == id)
        {
            self.auxiliary_tracking_mut()
                .pending_hidden_draw_reveals
                .retain(|(_, card)| *card != id);
        }
    }

    /// Hidden-tracked cards revealed publicly on every peer (checkpoint sync).
    pub fn publicly_revealed_hidden_cards(&self) -> Vec<ObjectId> {
        self.auxiliary_tracking
            .publicly_revealed_hidden_cards
            .iter()
            .copied()
            .collect()
    }

    /// Restore the publicly revealed hidden cards (checkpoint sync).
    pub fn restore_publicly_revealed_hidden_cards(
        &mut self,
        ids: impl IntoIterator<Item = ObjectId>,
    ) {
        self.auxiliary_tracking_mut().publicly_revealed_hidden_cards = ids.into_iter().collect();
    }

    /// Players for whom drawing a hidden card opens an owner reveal window.
    ///
    /// Set once at match setup from public information (open decklists), so it
    /// is identical on every peer.
    pub fn set_hidden_draw_reveal_players(&mut self, players: impl IntoIterator<Item = PlayerId>) {
        self.auxiliary_tracking_mut().hidden_draw_reveal_players = players.into_iter().collect();
    }

    pub fn hidden_draw_reveal_players(&self) -> Vec<PlayerId> {
        self.auxiliary_tracking
            .hidden_draw_reveal_players
            .iter()
            .copied()
            .collect()
    }

    /// Draw reveal windows not yet answered (checkpoint sync).
    pub fn pending_hidden_draw_reveals(&self) -> Vec<(PlayerId, ObjectId)> {
        self.auxiliary_tracking.pending_hidden_draw_reveals.clone()
    }

    /// Restore unanswered draw reveal windows (checkpoint sync).
    pub fn restore_pending_hidden_draw_reveals(&mut self, pending: Vec<(PlayerId, ObjectId)>) {
        self.auxiliary_tracking_mut().pending_hidden_draw_reveals = pending;
    }

    /// Open a draw reveal window for a hidden card drawn by an eligible player.
    ///
    /// Only the first card a player draws each turn can have a draw-reveal
    /// trigger (Miracle, CR 702.94a; no printed card triggers from the hand on
    /// later draws), so only that card is offered. Everything consulted here is
    /// identity-free, so every peer opens the same window.
    pub(crate) fn note_hidden_draw_for_reveal_window(
        &mut self,
        event: &crate::triggers::TriggerEvent,
    ) {
        let Some(drawn) = event.downcast::<crate::events::other::CardsDrawnEvent>() else {
            return;
        };
        if !drawn.is_first_this_turn
            || !self
                .auxiliary_tracking
                .hidden_draw_reveal_players
                .contains(&drawn.player)
        {
            return;
        }
        let Some(card) = drawn.first_card() else {
            return;
        };
        let owned_hand_card = self
            .object(card)
            .is_some_and(|object| object.zone == Zone::Hand && object.owner == drawn.player);
        if !owned_hand_card || !self.hidden_identity_is_private(card) {
            return;
        }
        if self
            .auxiliary_tracking
            .pending_hidden_draw_reveals
            .iter()
            .any(|(_, pending)| *pending == card)
        {
            return;
        }
        self.auxiliary_tracking_mut()
            .pending_hidden_draw_reveals
            .push((drawn.player, card));
    }

    /// Take the next unanswered draw reveal window whose card is still a
    /// private card in its owner's hand.
    pub(crate) fn next_pending_hidden_draw_reveal(&mut self) -> Option<(PlayerId, ObjectId)> {
        loop {
            let next = self
                .auxiliary_tracking
                .pending_hidden_draw_reveals
                .first()
                .copied()?;
            let (player, card) = next;
            let still_private = self
                .object(card)
                .is_some_and(|object| object.zone == Zone::Hand && object.owner == player)
                && self.hidden_identity_is_private(card);
            if still_private {
                return Some(next);
            }
            self.auxiliary_tracking_mut()
                .pending_hidden_draw_reveals
                .remove(0);
        }
    }

    /// Close the draw reveal window for `card`.
    pub(crate) fn finish_pending_hidden_draw_reveal(&mut self, card: ObjectId) {
        self.auxiliary_tracking_mut()
            .pending_hidden_draw_reveals
            .retain(|(_, pending)| *pending != card);
    }

    /// Ask `owner` to publicly reveal the private hidden cards among `cards`
    /// before an identity-dependent step.
    ///
    /// With `optional == false` every private card must be revealed (the cards
    /// are about to become public anyway, e.g. discarded); with `optional ==
    /// true` the owner may reveal any of them (Miracle). Cards already public
    /// on every peer, or not tracked by the mental-poker layer, are not asked
    /// about, so outside hidden-information matches this never prompts.
    ///
    /// Returns the cards revealed by this call, or `None` while the decision
    /// is awaiting the owner's answer (the caller must stop and let the answer
    /// be replayed).
    pub(crate) fn reveal_private_hidden_cards_publicly(
        &mut self,
        decision_maker: &mut (impl crate::decision::DecisionMaker + ?Sized),
        owner: PlayerId,
        source: ObjectId,
        cards: &[ObjectId],
        description: &str,
        optional: bool,
    ) -> Option<Vec<ObjectId>> {
        use crate::decisions::context::SelectionRevealPolicy;
        use crate::decisions::{make_decision, specs::ChooseObjectsSpec};

        let mut private = Vec::new();
        for &card in cards {
            let owned = self
                .object(card)
                .is_some_and(|object| object.owner == owner);
            if owned && self.hidden_identity_is_private(card) && !private.contains(&card) {
                private.push(card);
            }
        }
        if private.is_empty() {
            return Some(Vec::new());
        }
        let required = if optional { 0 } else { private.len() };
        let spec = ChooseObjectsSpec::new(
            source,
            description.to_string(),
            private.clone(),
            required,
            Some(private.len()),
        )
        .require_explicit_choice()
        .with_selection_reveal_policy(SelectionRevealPolicy::Public);
        let chosen: Vec<ObjectId> = make_decision(self, decision_maker, owner, Some(source), spec);
        if decision_maker.awaiting_choice() {
            return None;
        }
        let mut revealed = Vec::new();
        for id in chosen {
            if private.contains(&id) && !revealed.contains(&id) {
                revealed.push(id);
            }
        }
        // A forced reveal answered with fewer cards is a protocol violation:
        // the unrevealed cards stay private here, and a peer whose engine
        // then diverges detects it through the public checkpoint hash.
        self.mark_hidden_cards_publicly_revealed(&revealed);
        Some(revealed)
    }
}
