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
///
/// The ledger is local: only peers that held a placeholder when the claim was
/// made record it. Every entry is a public fact (the claim was made in the
/// public decision stream about a card every peer tracks), so exporting it in
/// a sync checkpoint leaks no hidden identity.
#[derive(Debug, Clone)]
pub struct HiddenIdentityObligation {
    pub stable_id: StableId,
    /// The card's owner (public).
    pub owner: PlayerId,
    /// The zone the card was in when the claim was made; the check evaluates
    /// the opened card there.
    pub zone: Zone,
    pub filter: ObjectFilter,
    pub filter_ctx: FilterContext,
    pub description: String,
    pub check: HiddenIdentityCheck,
    /// Set once the card entered a library while the claim was pending: the
    /// key of the [`HiddenLibraryAnchor`] naming the physical card's durable
    /// ziffle ciphertext. From then on the claim no longer follows the
    /// object's stable id (a reshuffle breaks the placeholder-to-card link) and
    /// is checked against the anchor's end-of-match disclosure instead.
    pub library_anchor: Option<String>,
}

impl HiddenIdentityObligation {
    /// Whether this entry still follows the object with `stable_id`.
    fn follows(&self, stable_id: StableId) -> bool {
        self.library_anchor.is_none() && self.stable_id == stable_id
    }

    /// Whether two entries record the same claim (checkpoint merges).
    pub fn same_claim(&self, other: &Self) -> bool {
        self.stable_id == other.stable_id
            && self.owner == other.owner
            && self.zone == other.zone
            && self.check == other.check
            && self.description == other.description
            && self.library_anchor == other.library_anchor
            && self.filter == other.filter
    }
}

/// The durable reference of a hidden card that entered a library while a
/// claim about it was pending (see `anchor_hidden_card_entering_library`).
///
/// Peers never know a placeholder's deck-manifest slot (the ziffle shuffles
/// exist to hide it), but they do know the public ziffle position the card
/// was dealt from: `ziffle:<deck hash>:<position>`, a ciphertext in a verified
/// ceremony record that later shuffles never change. At the end of the match
/// the owner must open that ciphertext (every seat provides its reveal token;
/// the game is over, so revealing is harmless) and every peer checks the
/// anchored claims against the opened card, wherever the card went after it
/// entered the library.
///
/// Anchors are recorded identically on every peer: only public facts decide
/// whether a card is a claim subject.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HiddenLibraryAnchor {
    pub owner: PlayerId,
    /// The library object the card became (bookkeeping for disclosure
    /// reports only; it may no longer exist).
    pub object_id: ObjectId,
    pub slot: u16,
    pub commitment: String,
    pub public_slot: Option<u16>,
    pub public_commitment: Option<String>,
    /// The printed name, when this engine knew the card as it entered the
    /// library (the owner's engine). Never exported to other perspectives.
    pub known_name: Option<String>,
}

impl HiddenLibraryAnchor {
    /// The durable key of the anchored ciphertext: its public ziffle position
    /// commitment when the card had one, else its deck-manifest slot (a card
    /// never shuffled still sits at its committed manifest slot).
    pub fn key(&self) -> String {
        match self.public_commitment.as_deref() {
            Some(commitment) if !commitment.is_empty() => {
                format!("{}:{commitment}", self.owner.index())
            }
            _ => format!(
                "{}:slot:{}:{}",
                self.owner.index(),
                self.slot,
                self.commitment
            ),
        }
    }
}

/// What an obligation claims about the hidden card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HiddenIdentityCheck {
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
    /// Cast face down through an effect's permission ("you may cast that
    /// card face down as a 2/2 creature spell", see
    /// [`FaceDownCastPermission`]) rather than a printed keyword. `source` is
    /// the permission's public source; the claim checked once the card opens
    /// is the permission's filter.
    Permission { source: ObjectId },
}

/// Wire name of [`FaceDownCastKind::Permission`].
pub const FACE_DOWN_CAST_PERMISSION_KIND: &str = "permission";

impl FaceDownCastKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Morph => "morph",
            Self::Megamorph => "megamorph",
            Self::Disguise => "disguise",
            Self::Permission { .. } => FACE_DOWN_CAST_PERMISSION_KIND,
        }
    }

    /// Parse a printed face-down cast kind. A permission kind also needs its
    /// source; see [`FaceDownCastKind::from_wire`].
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "morph" => Some(Self::Morph),
            "megamorph" => Some(Self::Megamorph),
            "disguise" => Some(Self::Disguise),
            _ => None,
        }
    }

    /// Parse the public kind a face-down cast command carries.
    pub fn from_wire(name: &str, permission_source: Option<ObjectId>) -> Option<Self> {
        if name.trim().eq_ignore_ascii_case(FACE_DOWN_CAST_PERMISSION_KIND) {
            return permission_source.map(|source| Self::Permission { source });
        }
        Self::from_name(name)
    }

    /// The permission source of a permission kind.
    pub fn permission_source(self) -> Option<ObjectId> {
        match self {
            Self::Permission { source } => Some(source),
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

    /// Whether `abilities` include an ability of this kind. A permission kind
    /// is never satisfied by printed abilities: its claim is recorded as the
    /// permission's filter instead.
    fn is_allowed_by(self, abilities: &[crate::ability::Ability]) -> bool {
        if matches!(self, Self::Permission { .. }) {
            return false;
        }
        abilities.iter().any(|ability| match &ability.kind {
            crate::ability::AbilityKind::Static(static_ability) => {
                Self::of_static_ability(static_ability) == Some(self)
            }
            _ => false,
        })
    }
}

/// An effect's permission to cast cards face down as 2/2 creature spells
/// without a printed morph, megamorph, or disguise ("you may cast creature
/// spells face down", "you may cast that card face down as a 2/2 creature
/// spell", Illusionary Mask). CR 708.4: the spell is cast face down with the
/// face-down characteristics of CR 708.2, for {3} (CR 702.37c) unless the
/// permission says otherwise.
///
/// Every field is public, so the permission is identical on every peer. A
/// face-down cast through it carries [`FaceDownCastKind::Permission`] with
/// the permission's source; peers that hold only a placeholder accept the
/// cast from that public claim and record the permission's `filter` as a
/// [`HiddenIdentityCheck::Matches`] obligation checked once the card opens.
#[derive(Debug, Clone, PartialEq)]
pub struct FaceDownCastPermission {
    /// The object whose effect grants the permission.
    pub source: ObjectId,
    /// The player who may cast face down.
    pub player: PlayerId,
    /// The zone the cards are cast from.
    pub zone: Zone,
    /// The cards the permission covers ("creature card").
    pub filter: ObjectFilter,
    /// Public description used in obligation reports.
    pub description: String,
    /// The permission ends when its source leaves the battlefield (a static
    /// ability's grant).
    pub requires_source_on_battlefield: bool,
    /// The permission ends after this turn number (an "until end of turn"
    /// effect), if set.
    pub expires_after_turn: Option<u32>,
    /// The permission is used up by one face-down cast.
    pub single_use: bool,
}

impl FaceDownCastPermission {
    fn filter_context(&self) -> FilterContext {
        FilterContext::new(self.player).with_source(self.source)
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
        self.mark_hidden_claim_subjects(chosen.iter().copied());
        let obligations: Vec<_> = chosen
            .iter()
            .filter(|id| self.is_hidden_card_placeholder(**id))
            .filter_map(|id| self.object(*id))
            .map(|object| HiddenIdentityObligation {
                stable_id: object.stable_id,
                owner: object.owner,
                zone: object.zone,
                filter: filter.clone(),
                filter_ctx: filter_ctx.clone(),
                description: description.to_string(),
                check: HiddenIdentityCheck::Matches,
                library_anchor: None,
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
    ///
    /// `withheld_is_public` says whether the withheld list itself is identical
    /// on every peer (a forced reveal's leftovers); only then are its cards
    /// marked as claim subjects here. Callers with a peer-dependent list mark
    /// a symmetric superset themselves.
    pub(crate) fn record_hidden_non_matching_obligations(
        &mut self,
        withheld: &[ObjectId],
        filter: &ObjectFilter,
        filter_ctx: &FilterContext,
        description: &str,
        withheld_is_public: bool,
    ) {
        if !filter_depends_on_card_identity(filter) {
            return;
        }
        if withheld_is_public {
            self.mark_hidden_claim_subjects(withheld.iter().copied());
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
                owner: object.owner,
                zone: object.zone,
                filter: filter.clone(),
                filter_ctx: filter_ctx.clone(),
                description: description.to_string(),
                check: HiddenIdentityCheck::DoesNotMatch,
                library_anchor: None,
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
        // Claim subjects must be identical on every peer, but `offered` is not
        // (the owner offers the matches it knows, other peers offer
        // placeholders) and neither, possibly, is the shortfall itself. Mark a
        // symmetric superset of every card this choice could leave a claim
        // about: each private hand card that passes the identity-free part of
        // the filter.
        if filter_depends_on_card_identity(filter) {
            let generic = identity_free_filter(filter);
            let subjects: Vec<ObjectId> = self
                .all_hand_card_ids()
                .into_iter()
                .filter(|id| self.hidden_identity_is_private(*id))
                .filter(|id| {
                    self.object(*id)
                        .is_some_and(|object| generic.matches(object, filter_ctx, self))
                })
                .collect();
            self.mark_hidden_claim_subjects(subjects);
        }
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
            false,
        );
    }

    /// Record that the face-down spell `id`, cast from a hidden hand with the
    /// public `kind`, must have that keyword once opened. Only placeholders
    /// are recorded.
    ///
    /// A permission kind records the permission's filter instead
    /// ([`HiddenIdentityCheck::Matches`]): the opened card must be one the
    /// permission covers. `permission` is the permission as it stood when
    /// the cast was proposed (it may be used up by the cast).
    pub(crate) fn record_hidden_face_down_cast_obligation(
        &mut self,
        id: ObjectId,
        kind: FaceDownCastKind,
        permission: Option<&FaceDownCastPermission>,
    ) {
        if self.hidden_card_info(id).is_some() {
            self.mark_hidden_claim_subjects([id]);
        }
        if !self.is_hidden_card_placeholder(id) {
            return;
        }
        let Some(object) = self.object(id) else {
            return;
        };
        let (filter, filter_ctx, description, check) = match (kind, permission) {
            (FaceDownCastKind::Permission { .. }, Some(permission)) => (
                permission.filter.clone(),
                permission.filter_context(),
                format!("cast face down using {}", permission.description),
                HiddenIdentityCheck::Matches,
            ),
            // A permission cast whose permission is gone cannot be legal; the
            // claim is recorded so that the opened card is still reported.
            _ => (
                ObjectFilter::default(),
                FilterContext::default(),
                format!("cast face down using {}", kind.as_str()),
                HiddenIdentityCheck::CastFaceDown(kind),
            ),
        };
        let obligation = HiddenIdentityObligation {
            stable_id: object.stable_id,
            owner: object.owner,
            zone: Zone::Hand,
            filter,
            filter_ctx,
            description,
            check,
            library_anchor: None,
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
            .any(|obligation| obligation.follows(stable_id))
    }

    /// Mark `ids` as subjects of a pending public claim. Callers pass only
    /// lists that are identical on every peer, so the marks (and the library
    /// anchors derived from them) are symmetric. Only hidden-tracked cards
    /// are marked.
    fn mark_hidden_claim_subjects(&mut self, ids: impl IntoIterator<Item = ObjectId>) {
        let subjects: Vec<StableId> = ids
            .into_iter()
            .filter(|id| self.hidden_card_info(*id).is_some())
            .filter_map(|id| self.object(id).map(|object| object.stable_id))
            .filter(|stable_id| {
                !self
                    .auxiliary_tracking
                    .hidden_claim_subjects
                    .contains(stable_id)
            })
            .collect();
        if subjects.is_empty() {
            return;
        }
        self.auxiliary_tracking_mut()
            .hidden_claim_subjects
            .extend(subjects);
    }

    /// Stop tracking `stable_id` as a claim subject (its card became public
    /// on every peer). Symmetric: called from zone moves only.
    pub(crate) fn forget_hidden_claim_subject(&mut self, stable_id: StableId) {
        if self
            .auxiliary_tracking
            .hidden_claim_subjects
            .contains(&stable_id)
        {
            self.auxiliary_tracking_mut()
                .hidden_claim_subjects
                .remove(&stable_id);
        }
    }

    /// A hidden card is entering a library (`new_id`, whose hidden info as it
    /// left its previous zone is `info`).
    ///
    /// A library is re-sealed by verified shuffles, after which a placeholder
    /// object no longer stands for the same physical card, so a pending claim
    /// can no longer follow the object. Instead, when the card is the subject
    /// of a public claim, it is anchored to its durable ziffle ciphertext
    /// ([`HiddenLibraryAnchor`], recorded identically on every peer) and this
    /// peer's pending obligations are re-keyed to the anchor. The owner must
    /// open every anchor in its end-of-match disclosure, where the claims are
    /// checked no matter where the card went afterwards.
    pub(crate) fn anchor_hidden_card_entering_library(
        &mut self,
        new_id: ObjectId,
        info: &super::HiddenCardInfo,
    ) {
        let Some(object) = self.object(new_id) else {
            return;
        };
        let stable_id = object.stable_id;
        let subject = self
            .auxiliary_tracking
            .hidden_claim_subjects
            .contains(&stable_id);
        let has_pending = self
            .auxiliary_tracking
            .hidden_identity_obligations
            .iter()
            .any(|obligation| obligation.follows(stable_id));
        if !subject {
            // Not a claim subject on every peer, so no anchor can be demanded
            // symmetrically. Every recording path marks a symmetric superset
            // of its obligations, so this only drops unreachable leftovers.
            if has_pending {
                self.clear_hidden_identity_obligations(new_id);
            }
            return;
        }
        let known_name = object
            .card
            .as_ref()
            .map(|_| object.identity_name().to_string());
        let anchor = HiddenLibraryAnchor {
            owner: info.owner,
            object_id: new_id,
            slot: info.slot,
            commitment: info.commitment.clone(),
            public_slot: info.public_slot,
            public_commitment: info.public_commitment.clone(),
            known_name,
        };
        let key = anchor.key();
        let tracking = self.auxiliary_tracking_mut();
        tracking.hidden_claim_subjects.remove(&stable_id);
        if !tracking
            .hidden_library_anchors
            .iter()
            .any(|existing| existing.key() == key)
        {
            tracking.hidden_library_anchors.push(anchor);
        }
        for obligation in tracking.hidden_identity_obligations.iter_mut() {
            if obligation.follows(stable_id) {
                obligation.library_anchor = Some(key.clone());
            }
        }
    }

    /// Record the public face-down cast kind carried by a face-down cast
    /// command for the hidden hand card `id`. Called identically on every
    /// peer before the command is replayed. A permission kind is accepted only
    /// while that permission lets the card's owner cast face down from the
    /// card's zone (identity-free facts every peer shares); otherwise it is
    /// ignored and the cast is rejected as illegal.
    pub fn set_hidden_face_down_cast_claim(&mut self, id: ObjectId, kind: FaceDownCastKind) {
        if self.hidden_card_info(id).is_none() {
            return;
        }
        if let FaceDownCastKind::Permission { source } = kind {
            let Some(object) = self.object(id) else {
                return;
            };
            if self
                .active_face_down_cast_permission(source, object.owner, object.zone)
                .is_none()
            {
                return;
            }
        }
        self.auxiliary_tracking_mut()
            .hidden_face_down_cast_claims
            .insert(id, kind);
    }

    /// Hidden face-down cast claims (checkpoint sync).
    pub fn hidden_face_down_cast_claims(&self) -> Vec<(ObjectId, FaceDownCastKind)> {
        let mut claims: Vec<_> = self
            .auxiliary_tracking
            .hidden_face_down_cast_claims
            .iter()
            .map(|(id, kind)| (*id, *kind))
            .collect();
        claims.sort_unstable_by_key(|(id, _)| *id);
        claims
    }

    /// Restore hidden face-down cast claims (checkpoint sync).
    pub fn restore_hidden_face_down_cast_claims(
        &mut self,
        claims: impl IntoIterator<Item = (ObjectId, FaceDownCastKind)>,
    ) {
        self.auxiliary_tracking_mut().hidden_face_down_cast_claims = claims.into_iter().collect();
    }

    /// This peer's obligation ledger (checkpoint sync). Entries are public
    /// claims about cards this peer held as placeholders.
    pub fn hidden_identity_obligations(&self) -> &[HiddenIdentityObligation] {
        &self.auxiliary_tracking.hidden_identity_obligations
    }

    /// Replace the obligation ledger (checkpoint sync). Entries whose card is
    /// no longer a hidden placeholder here (known to this peer, or gone) are
    /// dropped unless they are anchored to a library ciphertext or belong to
    /// a card snapshotted as its owner left the game.
    pub fn restore_hidden_identity_obligations(
        &mut self,
        obligations: impl IntoIterator<Item = HiddenIdentityObligation>,
    ) {
        let mut restored: Vec<HiddenIdentityObligation> = Vec::new();
        for obligation in obligations {
            if restored.iter().any(|existing| existing.same_claim(&obligation)) {
                continue;
            }
            let keep = match obligation.library_anchor.as_deref() {
                Some(key) => self
                    .auxiliary_tracking
                    .hidden_library_anchors
                    .iter()
                    .any(|anchor| anchor.key() == key),
                None => {
                    let live_placeholder = self
                        .find_object_by_stable_id(obligation.stable_id)
                        .is_some_and(|id| self.is_hidden_card_placeholder(id));
                    let departed = self
                        .auxiliary_tracking
                        .departed_hidden_cards
                        .iter()
                        .any(|departed| {
                            departed.object.stable_id == obligation.stable_id
                                && departed.object.card.is_none()
                        });
                    live_placeholder || departed
                }
            };
            if keep {
                restored.push(obligation);
            }
        }
        self.auxiliary_tracking_mut().hidden_identity_obligations = restored;
    }

    /// Cards marked as subjects of a pending public claim (checkpoint sync).
    pub fn hidden_claim_subjects(&self) -> Vec<StableId> {
        self.auxiliary_tracking
            .hidden_claim_subjects
            .iter()
            .copied()
            .collect()
    }

    /// Restore the claim subjects (checkpoint sync).
    pub fn restore_hidden_claim_subjects(&mut self, subjects: impl IntoIterator<Item = StableId>) {
        self.auxiliary_tracking_mut().hidden_claim_subjects = subjects.into_iter().collect();
    }

    /// Library anchors (checkpoint sync).
    pub fn hidden_library_anchors(&self) -> &[HiddenLibraryAnchor] {
        &self.auxiliary_tracking.hidden_library_anchors
    }

    /// Restore the library anchors (checkpoint sync).
    pub fn restore_hidden_library_anchors(
        &mut self,
        anchors: impl IntoIterator<Item = HiddenLibraryAnchor>,
    ) {
        self.auxiliary_tracking_mut().hidden_library_anchors = anchors.into_iter().collect();
    }

    /// Hidden cards snapshotted as their owner left the game (checkpoint
    /// sync).
    pub fn departed_hidden_cards(&self) -> &[DepartedHiddenCard] {
        &self.auxiliary_tracking.departed_hidden_cards
    }

    /// Restore the departed hidden-card snapshots (checkpoint sync).
    pub fn restore_departed_hidden_cards(
        &mut self,
        cards: impl IntoIterator<Item = DepartedHiddenCard>,
    ) {
        self.auxiliary_tracking_mut().departed_hidden_cards = cards.into_iter().collect();
    }

    // ------------------------------------------------------------------
    // Face-down cast permissions
    // ------------------------------------------------------------------

    /// Grant a face-down cast permission (see [`FaceDownCastPermission`]).
    /// A permission from the same source for the same player and zone is
    /// replaced.
    pub fn grant_face_down_cast_permission(&mut self, permission: FaceDownCastPermission) {
        let tracking = self.auxiliary_tracking_mut();
        tracking.face_down_cast_permissions.retain(|existing| {
            !(existing.source == permission.source
                && existing.player == permission.player
                && existing.zone == permission.zone)
        });
        tracking.face_down_cast_permissions.push(permission);
    }

    /// Remove every face-down cast permission granted by `source`.
    pub fn revoke_face_down_cast_permissions_from(&mut self, source: ObjectId) {
        if self
            .auxiliary_tracking
            .face_down_cast_permissions
            .iter()
            .any(|permission| permission.source == source)
        {
            self.auxiliary_tracking_mut()
                .face_down_cast_permissions
                .retain(|permission| permission.source != source);
        }
    }

    /// Face-down cast permissions (checkpoint sync).
    pub fn face_down_cast_permissions(&self) -> &[FaceDownCastPermission] {
        &self.auxiliary_tracking.face_down_cast_permissions
    }

    /// Restore face-down cast permissions (checkpoint sync).
    pub fn restore_face_down_cast_permissions(
        &mut self,
        permissions: impl IntoIterator<Item = FaceDownCastPermission>,
    ) {
        self.auxiliary_tracking_mut().face_down_cast_permissions =
            permissions.into_iter().collect();
    }

    fn face_down_cast_permission_is_active(&self, permission: &FaceDownCastPermission) -> bool {
        if permission
            .expires_after_turn
            .is_some_and(|turn| self.turn.turn_number > turn)
        {
            return false;
        }
        !permission.requires_source_on_battlefield
            || self
                .object(permission.source)
                .is_some_and(|source| source.zone == Zone::Battlefield)
    }

    /// The active permission from `source` letting `player` cast face down
    /// from `zone`. Identity-free: every peer agrees on it.
    pub(crate) fn active_face_down_cast_permission(
        &self,
        source: ObjectId,
        player: PlayerId,
        zone: Zone,
    ) -> Option<&FaceDownCastPermission> {
        self.auxiliary_tracking
            .face_down_cast_permissions
            .iter()
            .find(|permission| {
                permission.source == source
                    && permission.player == player
                    && permission.zone == zone
                    && self.face_down_cast_permission_is_active(permission)
            })
    }

    /// The source of an active permission that covers `spell` (read on an
    /// engine that knows the card; a placeholder matches no filter). The
    /// card's owner casts it from its current zone.
    pub(crate) fn face_down_cast_permission_source_for(
        &self,
        spell: &crate::object::Object,
    ) -> Option<ObjectId> {
        let permissions = &self.auxiliary_tracking.face_down_cast_permissions;
        if permissions.is_empty() {
            return None;
        }
        permissions
            .iter()
            .filter(|permission| permission.player == spell.owner && permission.zone == spell.zone)
            .filter(|permission| self.face_down_cast_permission_is_active(permission))
            .find(|permission| {
                permission
                    .filter
                    .matches(spell, &permission.filter_context(), self)
            })
            .map(|permission| permission.source)
    }

    /// Use up a single-use permission after a face-down cast through it.
    pub(crate) fn consume_face_down_cast_permission(
        &mut self,
        source: ObjectId,
        player: PlayerId,
        zone: Zone,
    ) {
        if self
            .auxiliary_tracking
            .face_down_cast_permissions
            .iter()
            .any(|permission| {
                permission.single_use
                    && permission.source == source
                    && permission.player == player
                    && permission.zone == zone
            })
        {
            self.auxiliary_tracking_mut()
                .face_down_cast_permissions
                .retain(|permission| {
                    !(permission.single_use
                        && permission.source == source
                        && permission.player == player
                        && permission.zone == zone)
                });
        }
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
        self.hidden_identity_violation_among(object, def, |obligation| {
            obligation.follows(stable_id)
        })
    }

    /// Evaluate the obligations selected by `selects` against `object`
    /// opened as `def`. Returns the first violation.
    fn hidden_identity_violation_among(
        &self,
        object: &crate::object::Object,
        def: &crate::cards::CardDefinition,
        selects: impl Fn(&HiddenIdentityObligation) -> bool,
    ) -> Option<String> {
        let obligations = &self.auxiliary_tracking.hidden_identity_obligations;
        if !obligations.iter().any(&selects) {
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
            .filter(|obligation| selects(obligation))
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
    /// Obligations anchored to a library ciphertext stay: the opened object
    /// need not be that physical card.
    pub(crate) fn clear_hidden_identity_obligations(&mut self, id: ObjectId) {
        let Some(stable_id) = self.object(id).map(|object| object.stable_id) else {
            return;
        };
        if self
            .auxiliary_tracking
            .hidden_identity_obligations
            .iter()
            .any(|obligation| obligation.follows(stable_id))
        {
            self.auxiliary_tracking_mut()
                .hidden_identity_obligations
                .retain(|obligation| !obligation.follows(stable_id));
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
// hand, every face-down spell or permanent it owns, and every library anchor
// it owns (the durable ziffle ciphertext of a claim subject that entered a
// library, see `HiddenLibraryAnchor`); peers verify the openings against the
// commitments and the obligation ledger. The rest of the library is never
// disclosed. A player who leaves the game has its objects removed
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
    /// The key of the library anchor this card's opening also settles
    /// ([`HiddenLibraryAnchor::key`]). A pure anchor entry has no live object:
    /// its `object_id` is bookkeeping only.
    pub library_anchor: Option<String>,
    /// Whether this entry is only a library anchor (no live or departed
    /// object to open).
    pub anchor_only: bool,
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
    /// snapshotted when it left the game, then its library anchors in the
    /// order they were recorded. Symmetric across peers.
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
                        library_anchor: None,
                        anchor_only: false,
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
                library_anchor: None,
                anchor_only: false,
            });
        }
        cards.sort_unstable_by_key(|card| card.object_id);
        for anchor in &self.auxiliary_tracking.hidden_library_anchors {
            if anchor.owner != player {
                continue;
            }
            let key = anchor.key();
            // The card may still sit at the anchored ciphertext (drawn back
            // without a reshuffle): one opening settles both entries.
            let anchor_info = super::HiddenCardInfo {
                owner: anchor.owner,
                zone: Zone::Library,
                slot: anchor.slot,
                commitment: anchor.commitment.clone(),
                public_slot: anchor.public_slot,
                public_commitment: anchor.public_commitment.clone(),
            };
            let same_ciphertext = cards.iter_mut().find(|card| {
                !card.anchor_only
                    && HiddenLibraryAnchor {
                        owner: card.info.owner,
                        object_id: card.object_id,
                        slot: card.info.slot,
                        commitment: card.info.commitment.clone(),
                        public_slot: card.info.public_slot,
                        public_commitment: card.info.public_commitment.clone(),
                        known_name: None,
                    }
                    .key()
                        == key
            });
            if let Some(card) = same_ciphertext {
                card.library_anchor = Some(key);
                continue;
            }
            if cards
                .iter()
                .any(|card| card.library_anchor.as_deref() == Some(key.as_str()))
            {
                continue;
            }
            cards.push(EndOfMatchDisclosureCard {
                object_id: anchor.object_id,
                info: anchor_info,
                zone: Zone::Library,
                face_down: false,
                known_name: anchor.known_name.clone(),
                library_anchor: Some(key),
                anchor_only: true,
            });
        }
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

    /// Check one end-of-match disclosure entry opened as `def`: the claims
    /// that follow its object, plus every claim anchored to its library
    /// ciphertext. Does not change state.
    pub fn end_of_match_disclosure_card_violation(
        &self,
        card: &EndOfMatchDisclosureCard,
        def: &crate::cards::CardDefinition,
    ) -> Option<String> {
        if !card.anchor_only
            && let Some(violation) = self.end_of_match_disclosure_violation(card.object_id, def)
        {
            return Some(violation);
        }
        let key = card.library_anchor.as_deref()?;
        let anchored = |obligation: &HiddenIdentityObligation| {
            obligation.library_anchor.as_deref() == Some(key)
        };
        let obligations = &self.auxiliary_tracking.hidden_identity_obligations;
        let first = obligations.iter().find(|obligation| anchored(obligation))?;
        // The anchored card has no live object here; evaluate its printed
        // characteristics on a fresh placeholder owned by its owner.
        let mut base = crate::object::Object::new_hidden_card(card.object_id, first.owner, Zone::Library);
        base.stable_id = first.stable_id;
        self.hidden_identity_violation_among(&base, def, anchored)
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
            true,
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
