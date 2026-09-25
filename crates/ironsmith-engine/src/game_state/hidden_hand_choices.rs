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

/// A pending check on a card whose identity was hidden from this peer when a
/// public decision depended on it. It is evaluated once the card is opened
/// (cast, discarded, revealed, moved to a public zone, or disclosed at the end
/// of the match) against the opened card's printed characteristics.
#[derive(Debug, Clone)]
pub(crate) struct HiddenIdentityObligation {
    pub(crate) stable_id: StableId,
    /// The zone the card was in when the claim was made; the check evaluates
    /// the opened card there.
    pub(crate) zone: Zone,
    pub(crate) filter: ObjectFilter,
    pub(crate) filter_ctx: FilterContext,
    pub(crate) description: String,
    pub(crate) check: HiddenIdentityCheck,
}

/// What an obligation claims about the hidden card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HiddenIdentityCheck {
    /// The card was chosen for the filter, so it must match it.
    Matches,
    /// The owner withheld the card from a forced reveal or claimed it was
    /// not a match ("reveal every creature card", "no creature card to
    /// discard"), so it must not match the filter.
    ///
    /// Limit: the filter is re-evaluated against the card's printed
    /// characteristics with the recorded filter context; qualities that
    /// depended on transient state at the time of the claim (effects that
    /// modified the card in hand, dynamic values that changed since) are
    /// judged as they stand when the card is opened.
    DoesNotMatch,
    /// The card was cast face down with this public kind, so its printed
    /// abilities must include that keyword (CR 702.37a, 702.168a).
    CastFaceDown(FaceDownCastKind),
}

/// The public kind of a face-down cast (CR 702.37, 702.37b, 702.168).
///
/// A face-down cast command carries it, so every peer (including those that
/// hold only a hidden-card placeholder) agrees the cast is legal and whether
/// the face-down spell has disguise's ward {2}. Peers holding a placeholder
/// record it as a [`HiddenIdentityCheck::CastFaceDown`] obligation checked
/// when the card is opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FaceDownCastKind {
    Morph,
    Megamorph,
    Disguise,
}

impl FaceDownCastKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Morph => "morph",
            Self::Megamorph => "megamorph",
            Self::Disguise => "disguise",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "morph" => Some(Self::Morph),
            "megamorph" => Some(Self::Megamorph),
            "disguise" => Some(Self::Disguise),
            _ => None,
        }
    }

    fn of_static_ability(ability: &crate::static_abilities::StaticAbility) -> Option<Self> {
        ability.turn_face_up_cost()?;
        Some(if ability.is_disguise() {
            Self::Disguise
        } else if ability.is_megamorph() {
            Self::Megamorph
        } else {
            Self::Morph
        })
    }

    /// The face-down cast kind `abilities` allow: the first morph, megamorph,
    /// or disguise ability.
    pub fn of_abilities(abilities: &[crate::ability::Ability]) -> Option<Self> {
        abilities.iter().find_map(|ability| match &ability.kind {
            crate::ability::AbilityKind::Static(static_ability) => {
                Self::of_static_ability(static_ability)
            }
            _ => None,
        })
    }

    /// Whether `abilities` include an ability of this kind.
    fn is_allowed_by(self, abilities: &[crate::ability::Ability]) -> bool {
        abilities.iter().any(|ability| match &ability.kind {
            crate::ability::AbilityKind::Static(static_ability) => {
                Self::of_static_ability(static_ability) == Some(self)
            }
            _ => false,
        })
    }
}

/// A "reveal the first card you draw each turn" reveal (Primitive Etchings,
/// Keranos, God-Eternal Kefnet, ...) whose drawn card was private when drawn.
///
/// The reveal's own "whenever you reveal a creature card this way" trigger
/// reads the card's characteristics, which peers holding a placeholder do not
/// know. Draw-step draws cannot pause for the owner's answer mid-step, so the
/// reveal is deferred to the draw reveal windows answered before triggers are
/// put on the stack: the owner reveals the card publicly (the peer front end
/// opens it on every peer before replaying the answer) and only then is the
/// reveal event emitted and its triggers checked, identically on every peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingAutomaticDrawReveal {
    pub player: PlayerId,
    pub card: ObjectId,
    /// The permanent whose static ability reveals the card.
    pub source: ObjectId,
    /// "You may reveal ..." (the owner may decline).
    pub optional: bool,
}

/// Prefix of every obligation violation message. The peer front end treats
/// failures carrying it as a detected cheat.
pub const HIDDEN_IDENTITY_VIOLATION_PREFIX: &str = "Hidden identity obligation violated";

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
                check: HiddenIdentityCheck::Matches,
            })
            .collect();
        if obligations.is_empty() {
            return;
        }
        self.auxiliary_tracking_mut()
            .hidden_identity_obligations
            .extend(obligations);
    }

    /// Record that the placeholders among `withheld` must *not* satisfy
    /// `filter` once opened: their owner left them out of a forced reveal of
    /// every matching card, or claimed it had no (more) matching card.
    ///
    /// Completeness of such an answer cannot be checked while the cards stay
    /// hidden (it would take a zero-knowledge non-membership proof), so it is
    /// checked later, whenever each card is opened; a violation is reported
    /// through the same failed-verification path as any other bad opening.
    /// Only placeholders are recorded, so the owner (who knows its cards)
    /// records nothing.
    pub(crate) fn record_hidden_non_matching_obligations(
        &mut self,
        withheld: &[ObjectId],
        filter: &ObjectFilter,
        filter_ctx: &FilterContext,
        description: &str,
    ) {
        if !filter_depends_on_card_identity(filter) {
            return;
        }
        let mut seen = Vec::new();
        let obligations: Vec<_> = withheld
            .iter()
            .copied()
            .filter(|id| {
                let fresh = !seen.contains(id);
                seen.push(*id);
                fresh
            })
            .filter(|id| self.is_hidden_card_placeholder(*id))
            .filter_map(|id| self.object(id))
            .filter(|object| object.zone.is_hidden())
            .map(|object| HiddenIdentityObligation {
                stable_id: object.stable_id,
                zone: object.zone,
                filter: filter.clone(),
                filter_ctx: filter_ctx.clone(),
                description: description.to_string(),
                check: HiddenIdentityCheck::DoesNotMatch,
            })
            .collect();
        if obligations.is_empty() {
            return;
        }
        self.auxiliary_tracking_mut()
            .hidden_identity_obligations
            .extend(obligations);
    }

    /// A hidden hand choice answered with fewer cards than the rules require
    /// (`required_min`) claims that no other offered card matches: record a
    /// [`HiddenIdentityCheck::DoesNotMatch`] obligation for every offered
    /// placeholder that was not chosen. Choices of "up to" a number
    /// (`required_min == 0`) or answered in full claim nothing.
    pub(crate) fn record_hidden_shortfall_obligations(
        &mut self,
        offered: &[ObjectId],
        chosen: &[ObjectId],
        required_min: usize,
        filter: &ObjectFilter,
        filter_ctx: &FilterContext,
        description: &str,
    ) {
        if chosen.len() >= required_min {
            return;
        }
        let withheld: Vec<ObjectId> = offered
            .iter()
            .copied()
            .filter(|id| !chosen.contains(id))
            .filter(|id| self.is_hidden_tracked_hand_card(*id))
            .collect();
        self.record_hidden_non_matching_obligations(
            &withheld,
            filter,
            filter_ctx,
            &format!("claimed no further match for \"{description}\""),
        );
    }

    /// Record that the face-down spell `id`, cast from a hidden hand with the
    /// public `kind`, must have that keyword once opened. Only placeholders
    /// are recorded.
    pub(crate) fn record_hidden_face_down_cast_obligation(
        &mut self,
        id: ObjectId,
        kind: FaceDownCastKind,
    ) {
        if !self.is_hidden_card_placeholder(id) {
            return;
        }
        let Some(object) = self.object(id) else {
            return;
        };
        let obligation = HiddenIdentityObligation {
            stable_id: object.stable_id,
            zone: Zone::Hand,
            filter: ObjectFilter::default(),
            filter_ctx: FilterContext::default(),
            description: format!("cast face down using {}", kind.as_str()),
            check: HiddenIdentityCheck::CastFaceDown(kind),
        };
        self.auxiliary_tracking_mut()
            .hidden_identity_obligations
            .push(obligation);
    }

    /// Whether this peer still holds an unchecked obligation for `id`.
    pub fn has_hidden_identity_obligation(&self, id: ObjectId) -> bool {
        let Some(stable_id) = self.object(id).map(|object| object.stable_id) else {
            return false;
        };
        self.auxiliary_tracking
            .hidden_identity_obligations
            .iter()
            .any(|obligation| obligation.stable_id == stable_id)
    }

    /// Drop the obligations of a hidden card that entered a library.
    ///
    /// A library is re-sealed by verified shuffles, after which a placeholder
    /// object no longer necessarily stands for the same physical card, so a
    /// claim about it could no longer be checked soundly (a documented limit
    /// of the deferred checks).
    pub(crate) fn forget_hidden_identity_obligations_for_library(&mut self, id: ObjectId) {
        if self
            .object(id)
            .is_some_and(|object| object.zone == Zone::Library)
        {
            self.clear_hidden_identity_obligations(id);
        }
    }

    /// Record the public face-down cast kind carried by a face-down cast
    /// command for the hidden hand card `id`. Called identically on every
    /// peer before the command is replayed.
    pub fn set_hidden_face_down_cast_claim(&mut self, id: ObjectId, kind: FaceDownCastKind) {
        if self.hidden_card_info(id).is_none() {
            return;
        }
        self.auxiliary_tracking_mut()
            .hidden_face_down_cast_claims
            .insert(id, kind);
    }

    /// The public face-down cast kind claimed for `id`, if any.
    pub fn hidden_face_down_cast_claim(&self, id: ObjectId) -> Option<FaceDownCastKind> {
        self.auxiliary_tracking
            .hidden_face_down_cast_claims
            .get(&id)
            .copied()
    }

    pub(crate) fn clear_hidden_face_down_cast_claim(&mut self, id: ObjectId) {
        if self
            .auxiliary_tracking
            .hidden_face_down_cast_claims
            .contains_key(&id)
        {
            self.auxiliary_tracking_mut()
                .hidden_face_down_cast_claims
                .remove(&id);
        }
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
        self.hidden_identity_obligation_violation_for_object(object, def)
    }

    fn hidden_identity_obligation_violation_for_object(
        &self,
        object: &crate::object::Object,
        def: &crate::cards::CardDefinition,
    ) -> Option<String> {
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
                let satisfied = match obligation.check {
                    HiddenIdentityCheck::Matches => obligation.filter.matches_non_recursive(
                        &candidate,
                        &obligation.filter_ctx,
                        self,
                    ),
                    HiddenIdentityCheck::DoesNotMatch => !obligation.filter.matches_non_recursive(
                        &candidate,
                        &obligation.filter_ctx,
                        self,
                    ),
                    HiddenIdentityCheck::CastFaceDown(kind) => {
                        kind.is_allowed_by(&candidate.abilities)
                    }
                };
                (!satisfied).then(|| {
                    format!(
                        "{HIDDEN_IDENTITY_VIOLATION_PREFIX}: {} does not satisfy the hidden choice \"{}\"",
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
// End-of-match disclosure
// ============================================================================
//
// Deferred claims (face-down cast kinds, "did not match" answers) about cards
// that are never opened during the game would otherwise go unchecked. When the
// match ends each player publicly opens every hidden card it still owns in its
// hand and every face-down spell or permanent it owns; peers verify the
// openings against the commitments and the obligation ledger. Libraries are
// never disclosed. A player who leaves the game has its objects removed
// (CR 800.4a), so the cards it must disclose are snapshotted as it leaves.

/// A hidden card that left the game with its owner (CR 800.4a), kept for the
/// end-of-match disclosure.
#[derive(Debug, Clone)]
pub struct DepartedHiddenCard {
    pub object: crate::object::Object,
    pub info: super::HiddenCardInfo,
    pub face_down: bool,
}

/// One card a player must open at the end of the match.
#[derive(Debug, Clone)]
pub struct EndOfMatchDisclosureCard {
    pub object_id: ObjectId,
    pub info: super::HiddenCardInfo,
    pub zone: Zone,
    pub face_down: bool,
    /// The printed name, when this engine knows the card (the owner's).
    pub known_name: Option<String>,
}

impl GameState {
    fn must_disclose_at_match_end(object: &crate::object::Object, face_down: bool) -> bool {
        object.zone == Zone::Hand
            || (face_down && matches!(object.zone, Zone::Battlefield | Zone::Stack))
    }

    /// Snapshot the cards `player` must disclose at the end of the match
    /// before its objects are removed as it leaves the game. Symmetric
    /// across peers (zones, face-down flags, hidden tracking).
    pub(crate) fn note_departing_hidden_cards(&mut self, player: PlayerId) {
        let mut departing: Vec<DepartedHiddenCard> = self
            .auxiliary_tracking
            .hidden_cards
            .iter()
            .filter(|(_, info)| info.owner == player)
            .filter_map(|(id, info)| {
                let object = self.object(*id)?;
                let face_down = self.is_face_down(*id);
                Self::must_disclose_at_match_end(object, face_down).then(|| DepartedHiddenCard {
                    object: object.clone(),
                    info: info.clone(),
                    face_down,
                })
            })
            .collect();
        if departing.is_empty() {
            return;
        }
        departing.sort_unstable_by_key(|card| card.object.id);
        self.auxiliary_tracking_mut()
            .departed_hidden_cards
            .extend(departing);
    }

    /// The hidden cards `player` must open at the end of the match, in object
    /// order: its live hand and face-down spells and permanents, plus those
    /// snapshotted when it left the game.
    pub fn end_of_match_disclosure_cards(&self, player: PlayerId) -> Vec<EndOfMatchDisclosureCard> {
        let known_name = |object: &crate::object::Object| {
            object
                .card
                .as_ref()
                .map(|_| object.identity_name().to_string())
        };
        let mut cards: Vec<EndOfMatchDisclosureCard> = self
            .auxiliary_tracking
            .hidden_cards
            .iter()
            .filter(|(_, info)| info.owner == player)
            .filter_map(|(id, info)| {
                let object = self.object(*id)?;
                let face_down = self.is_face_down(*id);
                Self::must_disclose_at_match_end(object, face_down).then(|| {
                    EndOfMatchDisclosureCard {
                        object_id: *id,
                        info: info.clone(),
                        zone: object.zone,
                        face_down,
                        known_name: known_name(object),
                    }
                })
            })
            .collect();
        for departed in &self.auxiliary_tracking.departed_hidden_cards {
            if departed.info.owner != player
                || cards
                    .iter()
                    .any(|card| card.object_id == departed.object.id)
            {
                continue;
            }
            cards.push(EndOfMatchDisclosureCard {
                object_id: departed.object.id,
                info: departed.info.clone(),
                zone: departed.object.zone,
                face_down: departed.face_down,
                known_name: known_name(&departed.object),
            });
        }
        cards.sort_unstable_by_key(|card| card.object_id);
        cards
    }

    /// Check a card disclosed at the end of the match against the obligation
    /// ledger (the live object, or its snapshot if its owner left the game).
    /// Does not change state.
    pub fn end_of_match_disclosure_violation(
        &self,
        id: ObjectId,
        def: &crate::cards::CardDefinition,
    ) -> Option<String> {
        if let Some(object) = self.object(id) {
            return self.hidden_identity_obligation_violation_for_object(object, def);
        }
        self.auxiliary_tracking
            .departed_hidden_cards
            .iter()
            .find(|departed| departed.object.id == id)
            .and_then(|departed| {
                self.hidden_identity_obligation_violation_for_object(&departed.object, def)
            })
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
    /// hidden-tracked card in its owner's hand (private, or already opened
    /// publicly by another owner-answered reveal).
    pub(crate) fn next_pending_hidden_draw_reveal(&mut self) -> Option<(PlayerId, ObjectId)> {
        loop {
            let next = self
                .auxiliary_tracking
                .pending_hidden_draw_reveals
                .first()
                .copied()?;
            let (player, card) = next;
            // A card another owner-answered reveal already opened publicly
            // (e.g. "reveal the first card you draw each turn") stays in the
            // window: its draw is re-checked without asking again.
            let still_in_hand = self
                .object(card)
                .is_some_and(|object| object.zone == Zone::Hand && object.owner == player)
                && self.hidden_card_info(card).is_some();
            if still_in_hand {
                return Some(next);
            }
            self.auxiliary_tracking_mut()
                .pending_hidden_draw_reveals
                .remove(0);
        }
    }

    /// Defer a "reveal the first card you draw" reveal of a private hidden
    /// card until the owner opens it publicly (see
    /// [`PendingAutomaticDrawReveal`]).
    pub(crate) fn defer_hidden_automatic_draw_reveal(&mut self, pending: PendingAutomaticDrawReveal) {
        if self
            .auxiliary_tracking
            .pending_hidden_automatic_draw_reveals
            .contains(&pending)
        {
            return;
        }
        self.auxiliary_tracking_mut()
            .pending_hidden_automatic_draw_reveals
            .push(pending);
    }

    /// Take the next deferred automatic draw reveal whose card is still in
    /// its owner's hand. Deferred reveals of cards that left the hand are
    /// dropped (their object no longer exists). Symmetric across peers.
    pub(crate) fn next_pending_hidden_automatic_draw_reveal(
        &mut self,
    ) -> Option<PendingAutomaticDrawReveal> {
        loop {
            let next = self
                .auxiliary_tracking
                .pending_hidden_automatic_draw_reveals
                .first()
                .copied()?;
            let in_hand = self.object(next.card).is_some_and(|object| {
                object.zone == Zone::Hand && object.owner == next.player
            });
            if in_hand {
                return Some(next);
            }
            self.auxiliary_tracking_mut()
                .pending_hidden_automatic_draw_reveals
                .remove(0);
        }
    }

    /// Remove a deferred automatic draw reveal once it has been answered.
    pub(crate) fn finish_pending_hidden_automatic_draw_reveal(
        &mut self,
        pending: &PendingAutomaticDrawReveal,
    ) {
        self.auxiliary_tracking_mut()
            .pending_hidden_automatic_draw_reveals
            .retain(|entry| entry != pending);
    }

    /// Deferred automatic draw reveals not yet answered (checkpoint sync).
    pub fn pending_hidden_automatic_draw_reveals(&self) -> Vec<PendingAutomaticDrawReveal> {
        self.auxiliary_tracking
            .pending_hidden_automatic_draw_reveals
            .clone()
    }

    /// Restore deferred automatic draw reveals (checkpoint sync).
    pub fn restore_pending_hidden_automatic_draw_reveals(
        &mut self,
        pending: Vec<PendingAutomaticDrawReveal>,
    ) {
        self.auxiliary_tracking_mut()
            .pending_hidden_automatic_draw_reveals = pending;
    }

    /// Close the draw reveal window for `card`.
    pub(crate) fn finish_pending_hidden_draw_reveal(&mut self, card: ObjectId) {
        self.auxiliary_tracking_mut()
            .pending_hidden_draw_reveals
            .retain(|(_, pending)| *pending != card);
    }

    /// Settle the pool of a random choice among hand cards matching `filter`
    /// ("discard a creature card at random", "exile a nonland card at random
    /// from your hand").
    ///
    /// The owner's engine sees which of its hidden cards match while peers
    /// hold placeholders, so a local pool (and the shuffle drawn over it)
    /// differs between peers. Under the rules the random pick is made among
    /// the qualifying cards, so every peer must agree on that set: each owner
    /// of private candidates in `hand_ids` is asked (the same identity-free
    /// question on every peer) to reveal publicly every such card that
    /// matches. The revealed cards are opened on every peer before the answer
    /// is replayed; revealed cards that do not match are left out, and every
    /// withheld card is recorded as a "does not match" obligation checked
    /// when that card is opened later (or disclosed at the end of the match).
    /// `candidates` is then rebuilt identically on every peer: its
    /// non-private entries (evaluated the same everywhere) followed by the
    /// revealed matches in hand order.
    ///
    /// Leak tradeoff: every qualifying card is revealed, not only the one
    /// picked. That is the minimum a shared random pick over a private hand
    /// needs without a zero-knowledge membership proof.
    ///
    /// Returns `false` while an owner's answer is awaited (the caller must
    /// stop). Never prompts outside hidden-information matches or when the
    /// filter states no quality of the card.
    pub(crate) fn settle_hidden_hand_random_pool(
        &mut self,
        decision_maker: &mut (impl crate::decision::DecisionMaker + ?Sized),
        source: ObjectId,
        filter: &ObjectFilter,
        filter_ctx: &FilterContext,
        hand_ids: &[ObjectId],
        candidates: &mut Vec<ObjectId>,
    ) -> bool {
        if !filter_depends_on_card_identity(filter) {
            return true;
        }
        let generic = identity_free_filter(filter);
        let private: Vec<ObjectId> = hand_ids
            .iter()
            .copied()
            .filter(|id| self.hidden_identity_is_private(*id))
            .filter(|id| {
                self.object(*id).is_some_and(|object| {
                    object.zone == Zone::Hand && generic.matches(object, filter_ctx, self)
                })
            })
            .fold(Vec::new(), |mut unique, id| {
                if !unique.contains(&id) {
                    unique.push(id);
                }
                unique
            });
        if private.is_empty() {
            return true;
        }
        let owners: Vec<PlayerId> = self
            .players
            .iter()
            .map(|player| player.id)
            .filter(|owner| {
                private.iter().any(|id| {
                    self.object(*id)
                        .is_some_and(|object| object.owner == *owner)
                })
            })
            .collect();
        let description = format!(
            "Reveal every {} in your hand (one is chosen at random)",
            filter.description()
        );
        for owner in owners {
            let owned: Vec<ObjectId> = private
                .iter()
                .copied()
                .filter(|id| {
                    self.object(*id)
                        .is_some_and(|object| object.owner == owner)
                })
                .collect();
            if self
                .reveal_private_hidden_cards_publicly(
                    decision_maker,
                    owner,
                    source,
                    &owned,
                    &description,
                    true,
                )
                .is_none()
            {
                return false;
            }
        }
        // Cards the owners did not reveal are claimed not to match: checked
        // when each is opened later (see `record_hidden_non_matching_obligations`).
        let withheld: Vec<ObjectId> = private
            .iter()
            .copied()
            .filter(|id| !self.is_publicly_revealed_hidden_card(*id))
            .collect();
        self.record_hidden_non_matching_obligations(
            &withheld,
            filter,
            filter_ctx,
            &description,
        );
        candidates.retain(|id| !private.contains(id));
        for id in private {
            let revealed_match = self.is_publicly_revealed_hidden_card(id)
                && self
                    .object(id)
                    .is_some_and(|object| filter.matches(object, filter_ctx, self));
            if revealed_match && !candidates.contains(&id) {
                candidates.push(id);
            }
        }
        true
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
        // A forced reveal is a determined set: the decision requires every
        // listed card (min == max == all candidates), the command validator
        // rejects short or repeated answers on every peer, and the peer front
        // end rejects an answer whose publicly revealed selections it was not
        // given an opening for. An optional reveal's withheld cards need no
        // claim (the owner may decline).
        self.mark_hidden_cards_publicly_revealed(&revealed);
        Some(revealed)
    }
}
