//! Replacement effect system.
//!
//! Replacement effects modify or replace events as they happen.
//! Per MTG rule 614, they use "instead" or "as [event]" or "skip".

use crate::ability::Ability;
use crate::effect::{Effect, Value};
use crate::events::cards::matchers::WouldDrawCardMatcher;
use crate::events::damage::matchers::{DamageFromSourceMatcher, DamageToPlayerMatcher};
use crate::events::life::matchers::WouldGainLifeMatcher;
use crate::events::permanents::matchers::ThisWouldBeDestroyedMatcher;
use crate::events::zones::matchers::{
    ThisWouldDieMatcher, ThisWouldEnterBattlefieldMatcher, WouldChangeZoneMatcher,
    WouldEnterBattlefieldMatcher,
};
use crate::events::{ReplacementMatcher, ReplacementPriority};
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;
use crate::static_abilities::StaticAbilityInstanceId;
use crate::target::ChooseSpec;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;
use ironsmith_core::AdditionalTokenKind;

/// A replacement effect that modifies events.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialization", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialization", serde(deny_unknown_fields, bound(deserialize = "A: serde::Deserialize<'de>, M: serde::Deserialize<'de>, S: serde::Deserialize<'de>, O: serde::Deserialize<'de>")))]
pub struct ReplacementEffect<A = ReplacementAction, M = Box<dyn ReplacementMatcher>, S = StaticAbilityInstanceId, O = ReplacementAbilityOrigin> {
    /// Unique identifier for this effect
    pub id: ReplacementEffectId,

    /// Identity of a persistent manager registration. Static/ephemeral
    /// effects leave this unset because their transient IDs can change.
    #[cfg_attr(feature = "serialization", serde(deserialize_with = "deserialize_present_descriptor_option"))]
    registration_id: Option<ReplacementEffectId>,

    /// The source that created this effect
    pub source: ObjectId,

    /// The controller of this effect
    pub controller: PlayerId,

    /// What happens instead
    pub replacement: A,

    /// Optional explicit priority bucket override per CR 616.1.
    #[cfg_attr(feature = "serialization", serde(deserialize_with = "deserialize_present_descriptor_option"))]
    pub priority_override: Option<ReplacementPriority>,

    /// Trait-based matcher for checking if this effect applies.
    #[cfg_attr(feature = "serialization", serde(deserialize_with = "deserialize_present_descriptor_option"))]
    pub matcher: Option<M>,

    /// Stable identity of the static ability that generated this effect.
    /// Resolution-created effects leave this unset.
    #[cfg_attr(feature = "serialization", serde(deserialize_with = "deserialize_present_descriptor_option"))]
    pub static_ability_instance: Option<S>,
    /// Stable originating occurrence plus generated branch, when produced
    /// from an object's abilities. Controller and mutable payload are not identity.
    #[cfg_attr(feature = "serialization", serde(deserialize_with = "deserialize_present_descriptor_option"))]
    pub ability_origin: Option<O>,

    /// Whether the affected player may decline this replacement effect.
    /// Optional effects are expanded into an explicit no-op CR 616 choice
    /// carrying the declined effect's stable application key.
    pub optional: bool,
}

/// Decode an explicitly present optional descriptor field. Serde's ordinary
/// Option handling would otherwise accept an omitted capture as None.
#[cfg(feature = "serialization")]
fn deserialize_present_descriptor_option<'de, T: serde::Deserialize<'de>, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    <Option<T> as serde::Deserialize>::deserialize(deserializer)
}

impl<A, M, S, O> ReplacementEffect<A, M, S, O> {
    /// Translate every executable and occurrence payload without changing the
    /// captured controller, source, registration, ordering or optionality.
    /// Each converter is mandatory and fallible. The owning codec must bind
    /// nested world/card references and validate its complete imported world.
    pub fn try_map_payloads<A2, M2, S2, O2, Error>(
        self,
        action: impl FnOnce(A) -> Result<A2, Error>,
        matcher: impl FnOnce(M) -> Result<M2, Error>,
        static_ability: impl FnOnce(S) -> Result<S2, Error>,
        origin: impl FnOnce(O) -> Result<O2, Error>,
    ) -> Result<ReplacementEffect<A2, M2, S2, O2>, Error> {
        // Exhaustive destructuring makes a newly added runtime field require an
        // explicit transport decision rather than silently disappearing.
        let Self { id, registration_id, source, controller, replacement,
            priority_override, matcher: native_matcher, static_ability_instance,
            ability_origin, optional } = self;
        Ok(ReplacementEffect {
            id, registration_id, source, controller,
            replacement: action(replacement)?, priority_override,
            matcher: native_matcher.map(matcher).transpose()?,
            static_ability_instance: static_ability_instance.map(static_ability).transpose()?,
            ability_origin: ability_origin.map(origin).transpose()?, optional,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialization", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialization", serde(deny_unknown_fields, bound(deserialize = "I: serde::Deserialize<'de>, C: serde::Deserialize<'de>")))]
pub struct ReplacementAbilityOrigin<I = StaticAbilityInstanceId, C = crate::ids::CardId> {
    pub ability: crate::continuous::AbilityOrigin<I, C>,
    #[cfg_attr(feature = "serialization", serde(deserialize_with = "deserialize_present_descriptor_option"))]
    pub printed_face: Option<C>,
    pub branch: usize,
}

impl<I, C> ReplacementAbilityOrigin<I, C> {
    /// Bind all nested generating occurrences through the owning ability table.
    pub fn try_map_static_instances<J, Error, F>(
        self, map: &mut F,
    ) -> Result<ReplacementAbilityOrigin<J, C>, Error>
    where F: FnMut(I) -> Result<J, Error> + ?Sized {
        let Self { ability, printed_face, branch } = self;
        Ok(ReplacementAbilityOrigin {
            ability: ability.try_map_static_instances(map)?, printed_face, branch,
        })
    }

    /// Bind every nested face, including level and borrowed/granted ancestry.
    /// The enclosing codec must supply visibility-approved definition roots.
    pub fn try_map_card_ids<D, Error, F>(
        self, map: &mut F,
    ) -> Result<ReplacementAbilityOrigin<I, D>, Error>
    where F: FnMut(C) -> Result<D, Error> + ?Sized {
        let Self { ability, printed_face, branch } = self;
        Ok(ReplacementAbilityOrigin {
            ability: ability.try_map_card_ids(map)?,
            printed_face: printed_face.map(&mut *map).transpose()?, branch,
        })
    }
}

/// Unique identifier for a replacement effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialization", derive(serde::Serialize, serde::Deserialize))]
pub struct ReplacementEffectId(pub u64);

impl ReplacementEffectId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Stable identity for a replacement effect across static-effect regeneration.
///
/// Static ability replacement effects are cleared and re-added during game-state
/// refreshes, which gives them fresh transient IDs. Event processing still needs
/// to recognize the same replacement effect for CR 614.5, especially when a
/// replacement creates nested events that move objects and refresh state.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialization", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialization", serde(deny_unknown_fields, bound(deserialize = "I: serde::Deserialize<'de>, C: serde::Deserialize<'de>")))]
pub enum ReplacementEffectKey<I = StaticAbilityInstanceId, C = crate::ids::CardId> {
    /// Separate resolutions remain separate even when their source and text match.
    Registered(ReplacementEffectId),
    /// The same ability occurrence survives regeneration and mutable bindings.
    Ability { source: ObjectId, origin: ReplacementAbilityOrigin<I, C> },
    /// Regenerated and event-local effects retain their structural identity.
    Regenerated {
        source: ObjectId,
        controller: PlayerId,
        #[cfg_attr(feature = "serialization", serde(deserialize_with = "deserialize_present_descriptor_option"))]
        static_ability_instance: Option<I>,
        #[cfg_attr(feature = "serialization", serde(deserialize_with = "deserialize_present_descriptor_option"))]
        matcher: Option<String>,
        replacement: String,
    },
}

impl<I, C> ReplacementEffectKey<I, C> {
    /// Remap occurrence references in both authoritative ability keys and the
    /// legacy structural key. Registered identities are manager-local and stay
    /// unchanged with the complete registered manager frame.
    pub fn try_map_static_instances<J, Error, F>(
        self, map: &mut F,
    ) -> Result<ReplacementEffectKey<J, C>, Error>
    where F: FnMut(I) -> Result<J, Error> + ?Sized {
        Ok(match self {
            Self::Registered(id) => ReplacementEffectKey::Registered(id),
            Self::Ability { source, origin } => ReplacementEffectKey::Ability {
                source, origin: origin.try_map_static_instances(map)?,
            },
            Self::Regenerated { source, controller, static_ability_instance, matcher, replacement } =>
                ReplacementEffectKey::Regenerated { source, controller,
                    static_ability_instance: static_ability_instance.map(&mut *map).transpose()?,
                    matcher, replacement },
        })
    }

    /// Remap all captured printed faces. Legacy structural identity strings are
    /// preserved as identity data, never interpreted as executable descriptors.
    /// Portability of legacy keys after payload/world rebinding remains an
    /// owning integration requirement, not a guarantee from this mapper.
    pub fn try_map_card_ids<D, Error, F>(
        self, map: &mut F,
    ) -> Result<ReplacementEffectKey<I, D>, Error>
    where F: FnMut(C) -> Result<D, Error> + ?Sized {
        Ok(match self {
            Self::Registered(id) => ReplacementEffectKey::Registered(id),
            Self::Ability { source, origin } => ReplacementEffectKey::Ability {
                source, origin: origin.try_map_card_ids(map)?,
            },
            Self::Regenerated { source, controller, static_ability_instance, matcher, replacement } =>
                ReplacementEffectKey::Regenerated { source, controller, static_ability_instance, matcher, replacement },
        })
    }
}

impl ReplacementEffect {
    pub fn with_ability_origin(mut self, ability: crate::continuous::AbilityOrigin,
        printed_face: Option<crate::ids::CardId>, branch: usize) -> Self {
        let origin = ReplacementAbilityOrigin { ability, printed_face, branch };
        // Accepting/declining and copy alternatives retain one parent identity,
        // even when the decline carrier was constructed before origin binding.
        if let ReplacementAction::DeclineOptional(key) = &mut self.replacement {
            *key = ReplacementEffectKey::Ability { source: self.source, origin: origin.clone() };
        }
        self.ability_origin = Some(origin);
        self
    }
    pub fn application_key(&self) -> ReplacementEffectKey {
        // Accepting and declining are two choices for the same effect, not
        // independent opportunities to replace the event.
        if let ReplacementAction::DeclineOptional(key) = &self.replacement {
            return key.clone();
        }
        if let Some(id) = self.registration_id {
            return ReplacementEffectKey::Registered(id);
        }
        if let Some(origin) = &self.ability_origin {
            return ReplacementEffectKey::Ability { source: self.source, origin: origin.clone() };
        }
        ReplacementEffectKey::Regenerated {
            source: self.source,
            controller: self.controller,
            static_ability_instance: self.static_ability_instance,
            matcher: self.matcher.as_ref().map(|matcher| matcher.display()),
            replacement: format!("{:?}", self.replacement),
        }
    }
}

/// What happens instead when a replacement triggers.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialization", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialization", serde(deny_unknown_fields))]
pub enum ReplacementAction<E = Effect, A = Ability, P = crate::resolution::ResolutionProgram, K = ReplacementEffectKey> {
    /// Prevent the event entirely
    Prevent,

    /// Prevent an entire damage event and emit the CR 615.13 application event.
    PreventDamage,

    /// Prevent up to the specified amount of damage and emit the CR 615.13 event.
    PreventDamageAmount(u32),

    /// Prevent half of the damage (rounded up or down) and emit the CR 615.13 event.
    PreventHalfDamage { round_up: bool },

    /// Prevent one point of damage for each matching counter available on this
    /// replacement effect's source, then remove exactly that many counters.
    PreventDamageByRemovingSourceCounters { counter_type: CounterType },

    /// Prevent the damage, then perform the prevention effect's additional part.
    ///
    /// The additional part still happens with an amount of zero when the damage
    /// can't be prevented (CR 615.12), while CR 615.13 is emitted only when the
    /// application actually prevents damage.
    PreventDamageThen(Vec<E>),

    /// Apply one prevention shield to a matching damage event.
    ///
    /// Damage processing exposes live shields as ordinary CR 616 candidates;
    /// this action consumes only the shield the affected player chooses.
    PreventWithShield {
        shield_id: crate::prevention::PreventionShieldId,
        /// Batch-level CR 615.7 allocation cap for this source event.
        #[cfg_attr(feature = "serialization", serde(deserialize_with = "deserialize_present_descriptor_option"))]
        max_amount: Option<u32>,
    },

    /// Apply the event but modified
    Modify(EventModification),

    /// Do something different instead
    Instead(Vec<E>),

    /// Redirect to a different target.
    /// Use `which` to specify which target to redirect for multi-target events.
    Redirect {
        target: RedirectTarget,
        /// Which target to redirect (default: First).
        which: RedirectWhich,
    },

    /// Redirect up to a fixed amount of damage to a different target.
    ///
    /// This is used by effects like:
    /// "The next 1 damage that would be dealt to this creature this turn is dealt to target creature instead."
    ///
    /// If the event's damage amount is larger than `amount`, only `amount` is redirected and
    /// the remainder stays on the original target.
    RedirectDamageAmount {
        target: RedirectTarget,
        /// Which redirectable target to rewrite (default: First).
        which: RedirectWhich,
        /// Maximum damage to redirect from a single matching event.
        amount: u32,
    },

    /// Change the zone an object would go to
    ChangeDestination(Zone),

    /// Exile a discarded card through its Madness replacement (CR 702.35a).
    /// The discard event retains this provenance for the linked trigger.
    DiscardWithMadness,

    /// The replacement's controller draws instead of the event's player
    /// ("that player skips that draw and you draw a card", Notion Thief).
    RedirectDrawToController,

    /// Move the object to a replacement zone and put counters on it.
    MoveToZoneWithCounters {
        zone: Zone,
        counters: Vec<(CounterType, u32)>,
    },

    /// Exile the object and record it as exiled with the replacement source.
    ExileWithSourceLink,

    /// Exile the object, record it as exiled with the replacement source, then
    /// execute follow-up effects from that source.
    ExileWithSourceLinkThen(Vec<E>),

    /// Exile the object with counters, record it as exiled with the replacement
    /// source, then execute follow-up effects from that source.
    ExileWithSourceLinkCountersThen {
        counters: Vec<(CounterType, u32)>,
        effects: Vec<E>,
    },

    /// Enter with the prospective printed number specified by an intrinsic rule.
    EnterWithIntrinsicStartingCounters(ironsmith_core::IntrinsicStartingCounter),

    /// Enter with additional counters
    EnterWithCounters {
        counter_type: CounterType,
        count: Value,
        /// Selects `count` when true and `otherwise_count` when false. The
        /// condition is evaluated with the entering object as its source.
        #[cfg_attr(feature = "serialization", serde(deserialize_with = "deserialize_present_descriptor_option"))]
        count_condition: Option<crate::ConditionExpr>,
        #[cfg_attr(feature = "serialization", serde(deserialize_with = "deserialize_present_descriptor_option"))]
        otherwise_count: Option<Value>,
        added_subtypes: Vec<Subtype>,
        added_abilities: Vec<A>,
    },

    /// Enter with the controller's choice of one counter type.
    EnterWithCounterChoice {
        counter_types: Vec<CounterType>,
        count: Value,
    },

    /// As this enters, an opponent may put counters on it and mark a keyword paid.
    Tribute {
        counter_type: CounterType,
        count: u32,
        paid_label: String,
    },

    /// Enter tapped
    EnterTapped,

    /// Enter untapped
    EnterUntapped,

    /// Enter under the specified player's control.
    EnterUnderControl(PlayerId),

    /// The affected permanent controller chooses its entering controller.
    EnterUnderChosenControl {
        players: crate::target::PlayerFilter,
    },

    /// Enter as a copy of something
    EnterAsCopy {
        source: ObjectId,
        enters_tapped: bool,
        #[cfg_attr(feature = "serialization", serde(deserialize_with = "deserialize_present_descriptor_option"))]
        copy_duration: Option<crate::effect::Until>,
        linked_exile_objects: Vec<ObjectId>,
        additional_counters: Vec<(CounterType, u32)>,
        #[cfg_attr(feature = "serialization", serde(deserialize_with = "deserialize_present_descriptor_option"))]
        name_override: Option<String>,
        added_colors: crate::color::ColorSet,
        added_card_types: Vec<CardType>,
        removes_other_card_types: bool,
        added_supertypes: Vec<Supertype>,
        removed_supertypes: Vec<Supertype>,
        added_subtypes: Vec<Subtype>,
        added_abilities: Vec<A>,
        #[cfg_attr(feature = "serialization", serde(deserialize_with = "deserialize_present_descriptor_option"))]
        set_base_power_toughness: Option<(i32, i32)>,
        /// What else happens once this copy is chosen.
        copy_followups: Vec<ironsmith_core::EnterAsCopyFollowup>,
    },

    /// Enter with permanent characteristic changes.
    EnterWithCharacteristics {
        added_card_types: Vec<CardType>,
        added_subtypes: Vec<Subtype>,
        #[cfg_attr(feature = "serialization", serde(deserialize_with = "deserialize_present_descriptor_option"))]
        set_base_power_toughness: Option<(i32, i32)>,
    },

    /// Double the effect (e.g., double damage, double counters)
    Double,

    /// Double counters of the matching type on counter-placement events.
    DoubleCounters {
        #[cfg_attr(feature = "serialization", serde(deserialize_with = "deserialize_present_descriptor_option"))]
        counter_type: Option<CounterType>,
    },

    /// Halve (rounded down) counters of the matching type on counter-placement
    /// events ("they put half that many ... instead, rounded down").
    HalveCounters {
        #[cfg_attr(feature = "serialization", serde(deserialize_with = "deserialize_present_descriptor_option"))]
        counter_type: Option<CounterType>,
    },

    /// Add extra counters of the matching type to counter-placement events
    /// ("that many plus one ... counters are put on it instead").
    AddCountersToPlacement {
        #[cfg_attr(feature = "serialization", serde(deserialize_with = "deserialize_present_descriptor_option"))]
        counter_type: Option<CounterType>,
        additional: i64,
    },

    /// Replace one player-counter event with a fixed amount and establish a
    /// turn-scoped prohibition on additional counters of that type.
    SetPlayerCountersAndLockForTurn {
        counter_type: CounterType,
        amount: u32,
    },

    /// Execute a selected as-enters program, then reconsider the pending entry.
    AsEntersProgram(P),

    /// Add an additional effect
    Additionally(Vec<E>),

    /// Explicitly decline one optional replacement for this event.
    DeclineOptional(K),

    /// Add separately defined tokens to a token-creation event.
    AddTokens {
        token: AdditionalTokenKind,
        count: u32,
    },

    /// Add one separately defined token per token being created ("those
    /// tokens plus that many 1/1 green Squirrel creature tokens", Chatterfang).
    AddTokensPerCreated { token: AdditionalTokenKind },

    /// Add one of each listed token kind other than the kind being created
    /// ("If you would create a Clue, Food, or Treasure token, instead create
    /// one of each", Academy Manufactor).
    AddTokensOfOtherKinds { kinds: Vec<AdditionalTokenKind> },

    /// Replace the mana produced by a matching mana event.
    ReplaceMana(Vec<crate::mana::ManaSymbol>),

    /// Replace the produced mana with exactly these symbols, changing the
    /// amount as well as the type ("produces {C} instead of any other type
    /// and amount", Damping Sphere).
    ReplaceManaExact(Vec<crate::mana::ManaSymbol>),

    /// Skip (for "skip your draw step" etc.)
    Skip,

    /// Interactive: Discard a card matching filter, or redirect to a different zone.
    ///
    /// Used by Mox Diamond: "If Mox Diamond would enter the battlefield, you may discard
    /// a land card instead. If you do, put Mox Diamond onto the battlefield. If you don't,
    /// put it into its owner's graveyard."
    ///
    /// When this applies:
    /// 1. Check if controller has any cards in hand matching the filter
    /// 2. If no matching cards, automatically redirect to redirect_zone
    /// 3. If matching cards exist, prompt the player to choose one or decline
    /// 4. If player discards a matching card, the permanent enters the battlefield
    /// 5. If player declines, the permanent goes to redirect_zone instead
    InteractiveDiscardOrRedirect {
        /// Filter for cards that can be discarded to satisfy the replacement.
        filter: ObjectFilter,
        /// Where the permanent goes if no card is discarded.
        redirect_zone: Zone,
    },

    /// Interactive: Sacrifice an exact number of matching controlled
    /// permanents, or redirect the entering permanent to another zone.
    InteractiveSacrificeOrRedirect {
        filter: ObjectFilter,
        count: u32,
        redirect_zone: Zone,
    },

    /// Interactive: Pay life or enter tapped.
    ///
    /// Used by shock lands (Godless Shrine, etc.): "As ~ enters the battlefield,
    /// you may pay 2 life. If you don't, it enters the battlefield tapped."
    ///
    /// When this applies:
    /// 1. Prompt the player if they want to pay life_cost life
    /// 2. If player pays, the permanent enters untapped
    /// 3. If player declines (or can't pay), the permanent enters tapped
    InteractivePayLifeOrEnterTapped {
        /// The amount of life to pay.
        life_cost: u32,
    },

    /// Interactive: Reveal a matching card from hand or enter tapped.
    ///
    /// Used by the SOI shadow lands and STX snarls: "As this land enters, you
    /// may reveal a Plains or Island card from your hand. If you don't, this
    /// land enters tapped."
    ///
    /// When this applies:
    /// 1. If the controller has no matching card in hand, the permanent enters tapped
    /// 2. Otherwise prompt the controller to reveal one matching card
    /// 3. If they reveal, the permanent enters untapped; if they decline, tapped
    InteractiveRevealCardOrEnterTapped {
        /// The hand card that may be revealed.
        filter: crate::target::ObjectFilter,
    },

    /// Interactive: Choose alternate destination for a zone-changing event.
    ///
    /// Used by Library of Leng: "If an effect causes you to discard a card,
    /// you may put it on top of your library instead of into your graveyard."
    ///
    /// When this applies:
    /// 1. Prompt the player with the choice of destinations
    /// 2. If player chooses the alternate destination, modify the event
    /// 3. If player declines, the event proceeds with its original destination
    ///
    /// This is a generic version that could work for various "instead of X, you may Y"
    /// effects involving zone changes.
    InteractiveChooseDestination {
        /// The destinations the player can choose from.
        /// The first destination is typically the default (original destination).
        destinations: Vec<Zone>,
        /// Description for the choice prompt.
        description: String,
    },
    /// General typed damage prevention; append to preserve wire variant ordinals.
    PreventDamageByRule(ironsmith_core::StaticDamagePreventionAmount),
}


impl<E, A, P, K> ReplacementAction<E, A, P, K> {
    /// Encode all executable bodies, added abilities, selected programs and
    /// declined-parent keys. Scalar event semantics remain unchanged. Each
    /// occurrence is converted in order, including identical independent bodies.
    /// The owning encoder supplies atomic table/reference binding and codecs.
    pub fn try_map_payloads<E2, A2, P2, K2, Error>(
        self, mut effect: impl FnMut(E) -> Result<E2, Error>,
        mut ability: impl FnMut(A) -> Result<A2, Error>,
        program: impl FnOnce(P) -> Result<P2, Error>,
        key: impl FnOnce(K) -> Result<K2, Error>,
    ) -> Result<ReplacementAction<E2, A2, P2, K2>, Error> {
        Ok(match self {
            Self::Prevent => ReplacementAction::Prevent,
            Self::PreventDamage => ReplacementAction::PreventDamage,
            Self::PreventDamageAmount(value) => ReplacementAction::PreventDamageAmount(value),
            Self::PreventDamageByRule(value) => ReplacementAction::PreventDamageByRule(value),
            Self::PreventHalfDamage { round_up } => ReplacementAction::PreventHalfDamage { round_up },
            Self::PreventDamageByRemovingSourceCounters { counter_type } => ReplacementAction::PreventDamageByRemovingSourceCounters { counter_type },
            Self::PreventDamageThen(value) => ReplacementAction::PreventDamageThen(value.into_iter().map(&mut effect).collect::<Result<Vec<_>, _>>()?),
            Self::PreventWithShield { shield_id, max_amount } => ReplacementAction::PreventWithShield { shield_id, max_amount },
            Self::Modify(value) => ReplacementAction::Modify(value),
            Self::Instead(value) => ReplacementAction::Instead(value.into_iter().map(&mut effect).collect::<Result<Vec<_>, _>>()?),
            Self::Redirect { target, which } => ReplacementAction::Redirect { target, which },
            Self::RedirectDamageAmount { target, which, amount } => ReplacementAction::RedirectDamageAmount { target, which, amount },
            Self::ChangeDestination(value) => ReplacementAction::ChangeDestination(value),
            Self::DiscardWithMadness => ReplacementAction::DiscardWithMadness,
            Self::RedirectDrawToController => ReplacementAction::RedirectDrawToController,
            Self::MoveToZoneWithCounters { zone, counters } => ReplacementAction::MoveToZoneWithCounters { zone, counters },
            Self::ExileWithSourceLink => ReplacementAction::ExileWithSourceLink,
            Self::ExileWithSourceLinkThen(value) => ReplacementAction::ExileWithSourceLinkThen(value.into_iter().map(&mut effect).collect::<Result<Vec<_>, _>>()?),
            Self::ExileWithSourceLinkCountersThen { counters, effects } => ReplacementAction::ExileWithSourceLinkCountersThen { counters, effects: effects.into_iter().map(&mut effect).collect::<Result<Vec<_>, _>>()? },
            Self::EnterWithCounters { counter_type, count, count_condition, otherwise_count, added_subtypes, added_abilities } => ReplacementAction::EnterWithCounters { counter_type, count, count_condition, otherwise_count, added_subtypes, added_abilities: added_abilities.into_iter().map(&mut ability).collect::<Result<Vec<_>, _>>()? },
            Self::EnterWithIntrinsicStartingCounters(rule) => ReplacementAction::EnterWithIntrinsicStartingCounters(rule),
            Self::EnterWithCounterChoice { counter_types, count } => ReplacementAction::EnterWithCounterChoice { counter_types, count },
            Self::Tribute { counter_type, count, paid_label } => ReplacementAction::Tribute { counter_type, count, paid_label },
            Self::EnterTapped => ReplacementAction::EnterTapped,
            Self::EnterUntapped => ReplacementAction::EnterUntapped,
            Self::EnterUnderControl(value) => ReplacementAction::EnterUnderControl(value),
            Self::EnterUnderChosenControl { players } => ReplacementAction::EnterUnderChosenControl { players },
            Self::EnterAsCopy { source, enters_tapped, copy_duration, linked_exile_objects, additional_counters, name_override, added_colors, added_card_types, removes_other_card_types, added_supertypes, removed_supertypes, added_subtypes, added_abilities, set_base_power_toughness, copy_followups } => ReplacementAction::EnterAsCopy { source, enters_tapped, copy_duration, linked_exile_objects, additional_counters, name_override, added_colors, added_card_types, removes_other_card_types, added_supertypes, removed_supertypes, added_subtypes, added_abilities: added_abilities.into_iter().map(&mut ability).collect::<Result<Vec<_>, _>>()?, set_base_power_toughness, copy_followups },
            Self::EnterWithCharacteristics { added_card_types, added_subtypes, set_base_power_toughness } => ReplacementAction::EnterWithCharacteristics { added_card_types, added_subtypes, set_base_power_toughness },
            Self::Double => ReplacementAction::Double,
            Self::DoubleCounters { counter_type } => ReplacementAction::DoubleCounters { counter_type },
            Self::HalveCounters { counter_type } => ReplacementAction::HalveCounters { counter_type },
            Self::AddCountersToPlacement { counter_type, additional } => ReplacementAction::AddCountersToPlacement { counter_type, additional },
            Self::SetPlayerCountersAndLockForTurn { counter_type, amount } => ReplacementAction::SetPlayerCountersAndLockForTurn { counter_type, amount },
            Self::AsEntersProgram(value) => ReplacementAction::AsEntersProgram(program(value)?),
            Self::Additionally(value) => ReplacementAction::Additionally(value.into_iter().map(&mut effect).collect::<Result<Vec<_>, _>>()?),
            Self::DeclineOptional(value) => ReplacementAction::DeclineOptional(key(value)?),
            Self::AddTokens { token, count } => ReplacementAction::AddTokens { token, count },
            Self::AddTokensPerCreated { token } => ReplacementAction::AddTokensPerCreated { token },
            Self::AddTokensOfOtherKinds { kinds } => ReplacementAction::AddTokensOfOtherKinds { kinds },
            Self::ReplaceMana(value) => ReplacementAction::ReplaceMana(value),
            Self::ReplaceManaExact(value) => ReplacementAction::ReplaceManaExact(value),
            Self::Skip => ReplacementAction::Skip,
            Self::InteractiveDiscardOrRedirect { filter, redirect_zone } => ReplacementAction::InteractiveDiscardOrRedirect { filter, redirect_zone },
            Self::InteractiveSacrificeOrRedirect { filter, count, redirect_zone } => ReplacementAction::InteractiveSacrificeOrRedirect { filter, count, redirect_zone },
            Self::InteractivePayLifeOrEnterTapped { life_cost } => ReplacementAction::InteractivePayLifeOrEnterTapped { life_cost },
            Self::InteractiveRevealCardOrEnterTapped { filter } => ReplacementAction::InteractiveRevealCardOrEnterTapped { filter },
            Self::InteractiveChooseDestination { destinations, description } => ReplacementAction::InteractiveChooseDestination { destinations, description },
        })
    }
}

/// How to modify an event.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialization", derive(serde::Serialize, serde::Deserialize))]
pub enum EventModification {
    /// Multiply by a factor (e.g., double strike)
    Multiply(u32),

    /// Add to the value
    Add(i32),

    /// Subtract from the value (minimum 0)
    Subtract(u32),

    /// Set to a specific value
    SetTo(u32),

    /// Set to a dynamic value if the event amount is lower.
    SetToAtLeast(crate::effect::Value),

    /// Reduce to zero (prevent)
    ReduceToZero,
}

/// Where to redirect an effect.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialization", derive(serde::Serialize, serde::Deserialize))]
pub enum RedirectTarget {
    /// Redirect to this permanent's controller
    ToController,

    /// Redirect to a specific player
    ToPlayer(PlayerId),

    /// Redirect to a specific object
    ToObject(ObjectId),

    /// Redirect to the source of the effect
    ToSource,

    /// Redirect to the controller of the event source.
    ToSourceController,
}

/// Which target to redirect in a multi-target event.
///
/// For events like `MoveCounters` that have multiple redirectable targets
/// (e.g., "source" and "destination"), this specifies which one to redirect.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serialization", derive(serde::Serialize, serde::Deserialize))]
pub enum RedirectWhich {
    /// Redirect the first (or only) redirectable target.
    /// This is the default behavior.
    #[default]
    First,

    /// Redirect the Nth target (0-indexed).
    Index(usize),

    /// Redirect targets matching this description.
    /// The description is matched against `RedirectableTarget::description`.
    /// E.g., "counter source" or "counter destination" for MoveCounters.
    ByDescription(String),
}

/// Source type for replacement effects - distinguishes between static abilities
/// and resolution-based effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serialization", derive(serde::Serialize, serde::Deserialize))]
pub enum ReplacementEffectSource {
    /// Effect from a static ability (regenerated each state refresh)
    StaticAbility,
    /// Effect from a resolved spell/ability (persists until removed)
    Resolution,
}

/// Shared builder for zone-change replacement effects.
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneReplacementSpec {
    pub filter: ObjectFilter,
    pub from_zone: Option<Zone>,
    pub to_zone: Option<Zone>,
    pub replacement_zone: Zone,
    pub follow_up_effects: Vec<Effect>,
}

impl ZoneReplacementSpec {
    pub fn new(filter: ObjectFilter, replacement_zone: Zone) -> Self {
        Self {
            filter,
            from_zone: None,
            to_zone: None,
            replacement_zone,
            follow_up_effects: Vec::new(),
        }
    }

    pub fn from_zone(mut self, zone: Zone) -> Self {
        self.from_zone = Some(zone);
        self
    }

    pub fn to_zone(mut self, zone: Zone) -> Self {
        self.to_zone = Some(zone);
        self
    }

    pub fn with_follow_up_effects(mut self, effects: Vec<Effect>) -> Self {
        self.follow_up_effects = effects;
        self
    }

    pub fn build(self, source: ObjectId, controller: PlayerId) -> ReplacementEffect {
        let replacement = if self.follow_up_effects.is_empty() {
            ReplacementAction::ChangeDestination(self.replacement_zone)
        } else {
            let move_effect = if self.replacement_zone == Zone::Exile {
                Effect::new(crate::effects::ExileEffect::with_spec(ChooseSpec::Source))
            } else {
                Effect::move_to_zone(ChooseSpec::Source, self.replacement_zone, true)
            };
            let mut effects = vec![move_effect];
            effects.extend(self.follow_up_effects);
            ReplacementAction::Instead(effects)
        };

        ReplacementEffect::with_matcher(
            source,
            controller,
            WouldChangeZoneMatcher::new(self.filter, self.from_zone, self.to_zone),
            replacement,
        )
    }
}

/// A replacement effect temporarily taken out of a [`ReplacementEffectManager`].
#[derive(Debug, Clone)]
pub struct SuspendedReplacementEffect {
    index: usize,
    effect: ReplacementEffect,
    source: Option<ReplacementEffectSource>,
    one_shot: bool,
    batch_one_shot: bool,
    pending_batch_one_shot: bool,
    until_end_of_turn: bool,
}

/// Complete persistent replacement registrations. Static descriptors are rebuilt
/// from the restored object world; their allocated identity gaps remain reserved.
/// A codec must convert every executable descriptor, not its display string.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialization", derive(serde::Serialize, serde::Deserialize))]
pub struct RegisteredReplacementEffectState<E = ReplacementEffect> {
    pub effects: Vec<E>,
    pub next_id: u64,
    pub effect_sources: Vec<(ReplacementEffectId, ReplacementEffectSource)>,
    pub one_shot_effects: Vec<ReplacementEffectId>,
    pub batch_one_shot_effects: Vec<ReplacementEffectId>,
    pub pending_batch_one_shot_effects: Vec<ReplacementEffectId>,
    pub until_end_of_turn_effects: Vec<ReplacementEffectId>,
    pub until_next_turn_effects: Vec<(ReplacementEffectId, (PlayerId, u32, Option<u32>))>,
}

impl<E> RegisteredReplacementEffectState<E> {
    pub fn try_map_effects<E2, Error>(
        self, convert: impl FnMut(E) -> Result<E2, Error>,
    ) -> Result<RegisteredReplacementEffectState<E2>, Error> {
        let Self { effects, next_id, effect_sources, one_shot_effects,
            batch_one_shot_effects, pending_batch_one_shot_effects,
            until_end_of_turn_effects, until_next_turn_effects } = self;
        Ok(RegisteredReplacementEffectState {
            effects: effects.into_iter().map(convert).collect::<Result<Vec<_>, _>>()?,
            next_id, effect_sources, one_shot_effects, batch_one_shot_effects,
            pending_batch_one_shot_effects, until_end_of_turn_effects, until_next_turn_effects,
        })
    }
}

/// Manages all replacement effects in the game.
#[derive(Debug, Clone, Default)]
pub struct ReplacementEffectManager {
    /// All active replacement effects
    effects: Vec<ReplacementEffect>,

    /// Source type for each effect (by ID)
    effect_sources: std::collections::HashMap<u64, ReplacementEffectSource>,

    /// One-shot effects that are consumed after a single use (e.g., regeneration shields).
    /// These are removed after being applied once.
    one_shot_effects: std::collections::HashSet<ReplacementEffectId>,

    /// One-shot entry replacements that apply to every matching member of one
    /// simultaneous ETB batch, then are consumed together after proposals for
    /// that batch have been prepared.
    batch_one_shot_effects: std::collections::HashSet<ReplacementEffectId>,

    /// Batch-scoped one-shots applied by at least one proposal in the current
    /// ETB batch. These remain live while sibling proposals are evaluated.
    pending_batch_one_shot_effects: std::collections::HashSet<ReplacementEffectId>,

    /// Temporary replacement effects that expire during cleanup.
    until_end_of_turn_effects: std::collections::HashSet<ReplacementEffectId>,

    /// Resolved replacements ending at a player's actual next turn start.
    until_next_turn_effects:
        std::collections::HashMap<ReplacementEffectId, (PlayerId, u32, Option<u32>)>,

    /// Next effect ID to assign
    next_id: u64,
}

impl ReplacementEffectManager {
    /// Capture full persistent state, rejecting inconsistent native bookkeeping
    /// rather than dropping it from an apparently successful checkpoint.
    pub fn registered_state(&self) -> Result<RegisteredReplacementEffectState, String> {
        // Exhaustive destructuring makes new manager state require an explicit
        // checkpoint decision rather than silently omitting it.
        let Self { effects, effect_sources, one_shot_effects, batch_one_shot_effects,
            pending_batch_one_shot_effects, until_end_of_turn_effects,
            until_next_turn_effects, next_id } = self;
        let mut allocated_ids = std::collections::HashSet::new();
        for effect in effects {
            if effect.id.0 >= *next_id || !allocated_ids.insert(effect.id) {
                return Err("invalid replacement manager allocation".into());
            }
        }
        let static_ids: std::collections::HashSet<_> = effects.iter()
            .filter(|effect| effect.registration_id.is_none()
                && effect_sources.get(&effect.id.0) == Some(&ReplacementEffectSource::StaticAbility))
            .map(|effect| effect.id).collect();
        let sorted_ids = |ids: &std::collections::HashSet<ReplacementEffectId>| {
            let mut ids: Vec<_> = ids.iter().copied().collect(); ids.sort_by_key(|id| id.0); ids
        };
        let mut sources: Vec<_> = effect_sources.iter()
            .filter(|(id, _)| !static_ids.contains(&ReplacementEffectId(**id)))
            .map(|(id, source)| (ReplacementEffectId(*id), *source)).collect();
        sources.sort_by_key(|(id, _)| id.0);
        let mut next_turn: Vec<_> = until_next_turn_effects.iter()
            .map(|(id, anchor)| (*id, *anchor)).collect();
        next_turn.sort_by_key(|(id, _)| id.0);
        let state = RegisteredReplacementEffectState {
            effects: effects.iter().filter(|effect| !static_ids.contains(&effect.id)).cloned().collect(),
            next_id: *next_id, effect_sources: sources,
            one_shot_effects: sorted_ids(one_shot_effects),
            batch_one_shot_effects: sorted_ids(batch_one_shot_effects),
            pending_batch_one_shot_effects: sorted_ids(pending_batch_one_shot_effects),
            until_end_of_turn_effects: sorted_ids(until_end_of_turn_effects),
            until_next_turn_effects: next_turn,
        };
        Self::new().restore_registered_state(state.clone())?;
        Ok(state)
    }

    /// Validate all identities and lifetime memberships before publishing. The
    /// owning importer must validate object/player references and regenerate
    /// static descriptors; departed sources remain valid stored identities.
    pub fn restore_registered_state(&mut self, state: RegisteredReplacementEffectState) -> Result<(), String> {
        if state.next_id == u64::MAX {
            return Err("serialized replacement allocator cannot advance".into());
        }
        let mut ids = std::collections::HashSet::new();
        for effect in &state.effects {
            if effect.registration_id != Some(effect.id) || effect.id.0 >= state.next_id || !ids.insert(effect.id) {
                return Err("invalid registered replacement identity".into());
            }
        }
        let mut sources = std::collections::HashMap::new();
        for (id, source) in state.effect_sources {
            if !ids.contains(&id) || source == ReplacementEffectSource::StaticAbility || sources.insert(id.0, source).is_some() {
                return Err("invalid registered replacement source".into());
            }
        }
        let checked_ids = |entries: Vec<ReplacementEffectId>| -> Result<std::collections::HashSet<ReplacementEffectId>, String> {
            let mut result = std::collections::HashSet::new();
            for id in entries {
                if !ids.contains(&id) || !result.insert(id) { return Err("invalid registered replacement lifetime membership".into()); }
            }
            Ok(result)
        };
        let one_shot = checked_ids(state.one_shot_effects)?;
        let batch = checked_ids(state.batch_one_shot_effects)?;
        if !one_shot.is_disjoint(&batch) { return Err("replacement cannot be both ordinary and batch one-shot".into()); }
        let pending = checked_ids(state.pending_batch_one_shot_effects)?;
        if !pending.is_subset(&batch) { return Err("pending replacement consumption is not a batch one-shot".into()); }
        let cleanup = checked_ids(state.until_end_of_turn_effects)?;
        let mut next_turn = std::collections::HashMap::new();
        for (id, anchor) in state.until_next_turn_effects {
            if !ids.contains(&id) || next_turn.insert(id, anchor).is_some() {
                return Err("invalid registered replacement next-turn anchor".into());
            }
        }
        *self = Self {
            effects: state.effects, effect_sources: sources, next_id: state.next_id,
            one_shot_effects: one_shot, batch_one_shot_effects: batch,
            pending_batch_one_shot_effects: pending, until_end_of_turn_effects: cleanup,
            until_next_turn_effects: next_turn,
        };
        Ok(())
    }

    /// Create a new empty manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot active replacement effects.
    pub fn effects(&self) -> &[ReplacementEffect] {
        &self.effects
    }

    /// Snapshot effect sources in deterministic order.
    pub fn effect_sources_snapshot(&self) -> Vec<(u64, ReplacementEffectSource)> {
        let mut entries: Vec<(u64, ReplacementEffectSource)> = self
            .effect_sources
            .iter()
            .map(|(id, source)| (*id, *source))
            .collect();
        entries.sort_by_key(|(id, _)| *id);
        entries
    }

    /// Snapshot one-shot effect ids in deterministic order.
    pub fn one_shot_effects_snapshot(&self) -> Vec<u64> {
        let mut entries: Vec<u64> = self
            .one_shot_effects
            .iter()
            .chain(&self.batch_one_shot_effects)
            .map(|id| id.0)
            .collect();
        entries.sort();
        entries
    }

    /// Snapshot cleanup-scoped effect ids in deterministic order.
    pub fn until_end_of_turn_effects_snapshot(&self) -> Vec<u64> {
        let mut entries: Vec<u64> = self
            .until_end_of_turn_effects
            .iter()
            .map(|id| id.0)
            .collect();
        entries.sort();
        entries
    }

    /// Get the next effect id (for deterministic state hashing).
    pub fn next_id(&self) -> u64 {
        self.next_id
    }

    /// Add a new replacement effect.
    pub fn add_effect(&mut self, effect: ReplacementEffect) -> ReplacementEffectId {
        self.register_effect(effect, true)
    }

    fn register_effect(
        &mut self,
        mut effect: ReplacementEffect,
        persistent: bool,
    ) -> ReplacementEffectId {
        let id = ReplacementEffectId::new(self.next_id);
        self.next_id += 1;
        effect.id = id;
        effect.registration_id = persistent.then_some(id);
        self.effects.push(effect);
        id
    }

    /// Remove an effect by ID.
    pub fn remove_effect(&mut self, id: ReplacementEffectId) {
        self.effects.retain(|e| e.id != id);
        self.effect_sources.remove(&id.0);
        self.one_shot_effects.remove(&id);
        self.batch_one_shot_effects.remove(&id);
        self.pending_batch_one_shot_effects.remove(&id);
        self.until_end_of_turn_effects.remove(&id);
        self.until_next_turn_effects.remove(&id);
    }

    /// Remove all effects from a specific source.
    pub fn remove_effects_from_source(&mut self, source: ObjectId) {
        let ids: Vec<_> = self.effects.iter()
            .filter(|effect| effect.source == source)
            .map(|effect| effect.id)
            .collect();
        for id in ids {
            self.remove_effect(id);
        }
    }

    /// Take the regeneration shields protecting `source` out of play for one
    /// destruction that can't be regenerated (CR 701.19c). Pass the result to
    /// [`Self::restore_suspended_effects`] if the permanent survives: the
    /// shields weren't used up.
    pub fn suspend_regeneration_shields_from_source(
        &mut self,
        source: ObjectId,
    ) -> Vec<SuspendedReplacementEffect> {
        let mut suspended = Vec::new();
        let mut index = 0;
        while index < self.effects.len() {
            let effect = &self.effects[index];
            let id = effect.id;
            let is_shield = effect.source == source
                && effect
                    .matcher
                    .as_ref()
                    .is_some_and(|matcher| matcher.is_regeneration_shield())
                && (self.one_shot_effects.contains(&id) || self.batch_one_shot_effects.contains(&id));
            if !is_shield {
                index += 1;
                continue;
            }
            let effect = self.effects.remove(index);
            suspended.push(SuspendedReplacementEffect {
                index: index + suspended.len(),
                effect,
                source: self.effect_sources.remove(&id.0),
                one_shot: self.one_shot_effects.remove(&id),
                batch_one_shot: self.batch_one_shot_effects.remove(&id),
                pending_batch_one_shot: self.pending_batch_one_shot_effects.remove(&id),
                until_end_of_turn: self.until_end_of_turn_effects.remove(&id),
            });
        }
        suspended
    }

    /// Put effects taken out by [`Self::suspend_regeneration_shields_from_source`]
    /// back in their original order and with their original identities.
    pub fn restore_suspended_effects(&mut self, suspended: Vec<SuspendedReplacementEffect>) {
        for entry in suspended {
            let id = entry.effect.id;
            let index = entry.index.min(self.effects.len());
            self.effects.insert(index, entry.effect);
            if let Some(source) = entry.source {
                self.effect_sources.insert(id.0, source);
            }
            if entry.one_shot {
                self.one_shot_effects.insert(id);
            }
            if entry.batch_one_shot {
                self.batch_one_shot_effects.insert(id);
            }
            if entry.pending_batch_one_shot {
                self.pending_batch_one_shot_effects.insert(id);
            }
            if entry.until_end_of_turn {
                self.until_end_of_turn_effects.insert(id);
            }
        }
    }

    /// Remove all one-shot effects from a specific source.
    ///
    /// Primarily used to ignore regeneration shields for "can't be regenerated"
    /// destroy effects.
    /// Remove the regeneration shields protecting `source` (CR 701.19c).
    /// Other one-shot replacements sourced from the object are unaffected.
    pub fn remove_regeneration_shields_from_source(&mut self, source: ObjectId) {
        let ids: Vec<_> = self
            .effects
            .iter()
            .filter(|e| {
                e.source == source
                    && e.matcher
                        .as_ref()
                        .is_some_and(|matcher| matcher.is_regeneration_shield())
                    && (self.one_shot_effects.contains(&e.id)
                        || self.batch_one_shot_effects.contains(&e.id))
            })
            .map(|e| e.id)
            .collect();
        for id in ids {
            self.remove_effect(id);
        }
    }

    pub fn remove_one_shot_effects_from_source(&mut self, source: ObjectId) {
        let ids: Vec<_> = self
            .effects
            .iter()
            .filter(|e| {
                e.source == source
                    && (self.one_shot_effects.contains(&e.id)
                        || self.batch_one_shot_effects.contains(&e.id))
            })
            .map(|e| e.id)
            .collect();
        for id in ids {
            self.remove_effect(id);
        }
    }

    /// Get all effects that might apply to a damage event.
    /// All effects are returned and filtered at runtime via matches_event().
    pub fn get_damage_replacements(&self) -> Vec<&ReplacementEffect> {
        self.effects
            .iter()
            .filter(|e| e.matcher.is_some())
            .collect()
    }

    /// Get all effects that might apply to a zone change.
    /// All effects are returned and filtered at runtime via matches_event().
    pub fn get_zone_change_replacements(&self) -> Vec<&ReplacementEffect> {
        self.effects
            .iter()
            .filter(|e| e.matcher.is_some())
            .collect()
    }

    /// Get all effects that might apply to drawing cards.
    /// All effects are returned and filtered at runtime via matches_event().
    pub fn get_draw_replacements(&self) -> Vec<&ReplacementEffect> {
        self.effects
            .iter()
            .filter(|e| e.matcher.is_some())
            .collect()
    }

    /// Get a replacement effect by its ID.
    pub fn get_effect(&self, id: ReplacementEffectId) -> Option<&ReplacementEffect> {
        self.effects.iter().find(|e| e.id == id)
    }

    /// Add a replacement effect from a static ability.
    ///
    /// These effects are regenerated each state refresh, so they are tracked
    /// separately from resolution-based effects.
    pub fn add_static_ability_effect(&mut self, effect: ReplacementEffect) -> ReplacementEffectId {
        let id = self.register_effect(effect, false);
        self.effect_sources
            .insert(id.0, ReplacementEffectSource::StaticAbility);
        id
    }

    /// Add a replacement effect from a resolved spell/ability.
    pub fn add_resolution_effect(&mut self, effect: ReplacementEffect) -> ReplacementEffectId {
        let id = self.add_effect(effect);
        self.effect_sources
            .insert(id.0, ReplacementEffectSource::Resolution);
        id
    }

    /// Clear all effects from static abilities.
    ///
    /// Called before regenerating static ability effects during state refresh.
    pub fn clear_static_ability_effects(&mut self) {
        let static_ids: Vec<ReplacementEffectId> = self
            .effects
            .iter()
            .filter(|e| {
                self.effect_sources
                    .get(&e.id.0)
                    .map(|s| *s == ReplacementEffectSource::StaticAbility)
                    .unwrap_or(false)
            })
            .map(|e| e.id)
            .collect();

        for id in static_ids {
            self.remove_effect(id);
        }
    }

    /// Get discard replacement effects that might apply.
    /// All effects are returned and filtered at runtime via matches_event().
    pub fn get_discard_replacements(&self) -> Vec<&ReplacementEffect> {
        self.effects
            .iter()
            .filter(|e| e.matcher.is_some())
            .collect()
    }

    // =========================================================================
    // One-Shot Effect Management
    // =========================================================================

    /// Add a one-shot replacement effect that will be consumed after use.
    ///
    /// One-shot effects are used for things like regeneration shields, which
    /// replace destruction once and then are removed. The effect is registered
    /// and tracked separately from static ability effects.
    ///
    /// Returns the ID of the added effect.
    pub fn add_one_shot_effect(&mut self, effect: ReplacementEffect) -> ReplacementEffectId {
        let id = self.add_effect(effect);
        self.one_shot_effects.insert(id);
        id
    }

    /// Add an entry replacement consumed after the next simultaneous ETB
    /// batch containing at least one matching object. Cleanup clears an
    /// unused effect just like an ordinary turn-scoped one-shot.
    pub fn add_batch_one_shot_effect(&mut self, effect: ReplacementEffect) -> ReplacementEffectId {
        let id = self.add_effect(effect);
        self.batch_one_shot_effects.insert(id);
        id
    }

    /// Add a replacement effect that lasts until cleanup.
    pub fn add_until_end_of_turn_effect(
        &mut self,
        effect: ReplacementEffect,
    ) -> ReplacementEffectId {
        let id = self.add_effect(effect);
        self.until_end_of_turn_effects.insert(id);
        id
    }

    /// Keep a resolved replacement live across any intervening or skipped turns.
    pub fn add_until_next_turn_effect(
        &mut self,
        effect: ReplacementEffect,
        player: PlayerId,
        created_turn: u32,
    ) -> ReplacementEffectId {
        let id = self.add_resolution_effect(effect);
        self.until_next_turn_effects
            .insert(id, (player, created_turn, None));
        id
    }

    pub fn prepare_for_departing_player(&mut self, player: PlayerId, boundary: u32) {
        for (duration_player, _, departure_boundary) in self.until_next_turn_effects.values_mut() {
            if *duration_player == player {
                *departure_boundary = Some(boundary);
            }
        }
    }

    /// Called after the next active players are chosen and before their turn begins.
    pub fn expire_at_turn_start(&mut self, turn: u32, active_players: &[PlayerId]) {
        let expired: Vec<_> = self
            .until_next_turn_effects
            .iter()
            .filter_map(|(id, (player, created_turn, departure_boundary))| {
                let expired = departure_boundary.map_or(
                    turn > *created_turn && active_players.contains(player),
                    |boundary| turn >= boundary,
                );
                expired.then_some(*id)
            })
            .collect();
        for id in expired {
            self.remove_effect(id);
        }
    }

    /// Mark a one-shot effect as used and remove it.
    ///
    /// Returns true if the effect was found and removed, false if it wasn't
    /// a one-shot effect or didn't exist.
    /// Consume `redirected` damage from a one-shot "the next N damage ...
    /// is dealt to ... instead" redirection shield. The shield stays until
    /// all N damage has been redirected; it is removed once exhausted.
    /// Returns false when `id` is not such a shield (callers then consume it
    /// normally).
    pub fn consume_redirect_damage_amount(
        &mut self,
        id: ReplacementEffectId,
        redirected: u32,
    ) -> bool {
        if !self.one_shot_effects.contains(&id) {
            return false;
        }
        let Some(effect) = self.effects.iter_mut().find(|effect| effect.id == id) else {
            return false;
        };
        let ReplacementAction::RedirectDamageAmount { amount, .. } = &mut effect.replacement
        else {
            return false;
        };
        if redirected < *amount {
            *amount -= redirected;
            return true;
        }
        self.mark_effect_used(id)
    }

    pub fn mark_effect_used(&mut self, id: ReplacementEffectId) -> bool {
        if self.batch_one_shot_effects.contains(&id) {
            self.pending_batch_one_shot_effects.insert(id);
            return true;
        }
        if self.one_shot_effects.remove(&id) {
            self.remove_effect(id);
            true
        } else {
            false
        }
    }

    /// Check if an effect is a one-shot effect.
    pub fn is_one_shot(&self, id: ReplacementEffectId) -> bool {
        self.one_shot_effects.contains(&id) || self.batch_one_shot_effects.contains(&id)
    }

    /// Consume every batch-scoped one-shot applied while preparing the current
    /// simultaneous ETB event. Called only after all sibling proposals have
    /// had a chance to see the same replacement.
    pub fn consume_pending_batch_one_shot_effects(&mut self) {
        let used: Vec<_> = self.pending_batch_one_shot_effects.drain().collect();
        for id in used {
            self.remove_effect(id);
        }
    }

    /// Clear all one-shot effects (e.g., at end of turn).
    pub fn clear_one_shot_effects(&mut self) {
        let one_shot_ids: Vec<_> = self
            .one_shot_effects
            .iter()
            .chain(&self.batch_one_shot_effects)
            .copied()
            .collect();
        for id in one_shot_ids {
            self.remove_effect(id);
        }
        self.one_shot_effects.clear();
        self.batch_one_shot_effects.clear();
        self.pending_batch_one_shot_effects.clear();
    }

    /// Clear all replacement effects that expire during cleanup.
    pub fn clear_until_end_of_turn_effects(&mut self) {
        let ids: Vec<_> = self.until_end_of_turn_effects.iter().copied().collect();
        for id in ids {
            self.remove_effect(id);
        }
        self.until_end_of_turn_effects.clear();
    }

    /// Get the count of one-shot effects from a specific source.
    ///
    /// This is useful for checking how many regeneration shields a creature has.
    pub fn count_one_shot_effects_from_source(&self, source: ObjectId) -> u32 {
        self.effects
            .iter()
            .filter(|e| {
                e.source == source
                    && (self.one_shot_effects.contains(&e.id)
                        || self.batch_one_shot_effects.contains(&e.id))
            })
            .count() as u32
    }
}

impl ReplacementEffect {
    /// Create a new replacement effect using a trait-based matcher.
    pub fn with_matcher<M: ReplacementMatcher + 'static>(
        source: ObjectId,
        controller: PlayerId,
        matcher: M,
        replacement: ReplacementAction,
    ) -> Self {
        Self {
            id: ReplacementEffectId(0),
            registration_id: None,
            source,
            controller,
            replacement,
            priority_override: None,
            matcher: Some(Box::new(matcher)),
            static_ability_instance: None,
            ability_origin: None,
            optional: false,
        }
    }

    /// Create a new replacement effect using a boxed trait-based matcher.
    pub fn with_boxed_matcher(
        source: ObjectId,
        controller: PlayerId,
        matcher: Box<dyn ReplacementMatcher>,
        replacement: ReplacementAction,
    ) -> Self {
        Self {
            id: ReplacementEffectId(0),
            registration_id: None,
            source,
            controller,
            replacement,
            priority_override: None,
            matcher: Some(matcher),
            static_ability_instance: None,
            ability_origin: None,
            optional: false,
        }
    }

    /// Set a trait-based matcher on this effect.
    pub fn with_trait_matcher<M: ReplacementMatcher + 'static>(mut self, matcher: M) -> Self {
        self.matcher = Some(Box::new(matcher));
        self
    }

    /// Override the natural priority bucket used when applying this effect.
    pub fn with_priority_override(mut self, priority: ReplacementPriority) -> Self {
        self.priority_override = Some(priority);
        self
    }

    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    /// Build the alternative for declining this same effect. For persistent
    /// effects, derive this from the registered effect so it carries the
    /// registration identity rather than the pre-registration fingerprint.
    pub fn optional_decline_effect(&self) -> Option<Self> {
        self.optional.then(|| Self {
            id: ReplacementEffectId(0),
            registration_id: None,
            source: self.source,
            controller: self.controller,
            replacement: ReplacementAction::DeclineOptional(self.application_key()),
            priority_override: self.priority_override,
            matcher: self.matcher.as_ref().map(|matcher| matcher.clone_box()),
            static_ability_instance: self.static_ability_instance,
            ability_origin: self.ability_origin.clone(),
            optional: false,
        })
    }

    /// Create a damage prevention effect.
    pub fn prevent_damage(source: ObjectId, controller: PlayerId, amount: u32) -> Self {
        Self::with_matcher(
            source,
            controller,
            DamageToPlayerMatcher::to_you(),
            ReplacementAction::PreventDamageAmount(amount),
        )
    }

    /// Create a "can't gain life" effect.
    pub fn cant_gain_life(source: ObjectId, controller: PlayerId) -> Self {
        Self::with_matcher(
            source,
            controller,
            WouldGainLifeMatcher::any_player(),
            ReplacementAction::Prevent,
        )
    }

    /// Create an "enters tapped" effect.
    pub fn enters_tapped(source: ObjectId, controller: PlayerId, filter: ObjectFilter) -> Self {
        Self::with_matcher(
            source,
            controller,
            WouldEnterBattlefieldMatcher::new(filter),
            ReplacementAction::EnterTapped,
        )
    }

    /// Create a "this enters with N counters" effect.
    pub fn enters_with_counters(
        source: ObjectId,
        controller: PlayerId,
        counter_type: CounterType,
        count: Value,
    ) -> Self {
        Self::with_matcher(
            source,
            controller,
            ThisWouldEnterBattlefieldMatcher,
            ReplacementAction::EnterWithCounters {
                counter_type,
                count,
                count_condition: None,
                otherwise_count: None,
                added_subtypes: Vec::new(),
                added_abilities: Vec::new(),
            },
        )
    }

    /// Create a "if this would die, exile it instead" effect.
    pub fn exile_instead_of_dying(source: ObjectId, controller: PlayerId) -> Self {
        Self::with_matcher(
            source,
            controller,
            ThisWouldDieMatcher,
            ReplacementAction::ChangeDestination(Zone::Exile),
        )
    }

    /// Create a "double damage" effect.
    pub fn double_damage(
        source: ObjectId,
        controller: PlayerId,
        from_filter: ObjectFilter,
    ) -> Self {
        Self::with_matcher(
            source,
            controller,
            DamageFromSourceMatcher::new(from_filter),
            ReplacementAction::Double,
        )
    }

    /// Create a "skip draw step" effect.
    pub fn skip_draw(source: ObjectId, controller: PlayerId, player: PlayerFilter) -> Self {
        Self::with_matcher(
            source,
            controller,
            WouldDrawCardMatcher::new(player),
            ReplacementAction::Skip,
        )
    }

    // Note: "Can't lose the game" (Platinum Angel) is implemented via CantEffects,
    // not as a replacement effect. See game_state.rs CantEffects::can_lose_game().

    /// Create an indestructible effect.
    pub fn indestructible(source: ObjectId, controller: PlayerId) -> Self {
        Self::with_matcher(
            source,
            controller,
            ThisWouldBeDestroyedMatcher,
            ReplacementAction::Prevent,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registered_transport_fixture() -> (ReplacementEffectManager, [ReplacementEffectId; 5]) {
        let source = ObjectId::from_raw(71);
        let player = PlayerId::from_index(1);
        let mut manager = ReplacementEffectManager::new();
        let effect = ReplacementEffect::with_matcher(source, player, WouldGainLifeMatcher::you(), ReplacementAction::Double);
        let gap = manager.add_resolution_effect(effect.clone()); manager.remove_effect(gap);
        manager.add_static_ability_effect(effect.clone());
        let persistent = manager.add_resolution_effect(effect.clone());
        let one = manager.add_one_shot_effect(effect.clone());
        let batch = manager.add_batch_one_shot_effect(effect.clone());
        manager.mark_effect_used(batch);
        let cleanup = manager.add_until_end_of_turn_effect(effect.clone());
        let turn = manager.add_until_next_turn_effect(effect, player, 4);
        manager.prepare_for_departing_player(player, 9);
        (manager, [persistent, one, batch, cleanup, turn])
    }

    fn complete_descriptor_fixture() -> ReplacementEffect {
        let mut effect = ReplacementEffect::with_matcher(
            ObjectId::from_raw(173), PlayerId::from_index(1),
            WouldGainLifeMatcher::you(), ReplacementAction::Double,
        ).with_ability_origin(crate::continuous::AbilityOrigin::Printed(4), Some(crate::ids::CardId::new()), 7);
        effect.id = ReplacementEffectId(29);
        effect.registration_id = Some(ReplacementEffectId(29));
        effect.priority_override = Some(ReplacementPriority::SelfReplacement);
        effect.static_ability_instance = Some(crate::static_abilities::StaticAbility::enters_tapped_ability().instance_id());
        effect.optional = true;
        effect
    }

    #[test]
    fn replacement_action_payload_conversion_keeps_order_repeats_and_failures() {
        use std::cell::Cell;
        type Action = ReplacementAction<u8, u8, u8, u8>;
        let calls = Cell::new(0);
        let converted = Action::Instead(vec![3, 3, 7]).try_map_payloads(
            |value| { calls.set(calls.get() + 1); Ok::<_, u8>(value + 10) },
            Ok, Ok, Ok,
        ).unwrap();
        assert_eq!(converted, Action::Instead(vec![13, 13, 17]));
        assert_eq!(calls.get(), 3, "equal occurrences are independent");
        let calls = Cell::new(0);
        let failed = Action::PreventDamageThen(vec![3, 7, 9]).try_map_payloads(
            |value| { calls.set(calls.get() + 1); if value == 7 { Err(value) } else { Ok(value) } },
            Ok, Ok, Ok,
        );
        assert_eq!(failed.unwrap_err(), 7);
        assert_eq!(calls.get(), 2, "stop at the actual failed body");
        assert_eq!(Action::AsEntersProgram(19).try_map_payloads(Ok, Ok,
            |_| Err::<u8, _>("program"), Ok).unwrap_err(), "program");
        assert_eq!(Action::DeclineOptional(23).try_map_payloads(Ok, Ok, Ok,
            |_| Err::<u8, _>("key")).unwrap_err(), "key");
    }

    #[cfg(feature = "serialization")]
    #[test]
    fn replacement_action_wire_retains_redirect_description_and_dynamic_entry_counters() {
        type Action = ReplacementAction<u8, u8, u8, u8>;
        let redirect = Action::RedirectDamageAmount {
            target: RedirectTarget::ToPlayer(PlayerId::from_index(1)),
            which: RedirectWhich::ByDescription(String::from("damage recipient")), amount: 5,
        };
        let restored: Action = serde_json::from_str(&serde_json::to_string(&redirect).unwrap()).unwrap();
        assert_eq!(restored, redirect);
        let counters = Action::EnterWithCounters {
            counter_type: CounterType::PlusOnePlusOne,
            count: Value::EventValue(crate::effect::EventValueSpec::Amount),
            count_condition: None, otherwise_count: Some(Value::Fixed(7)),
            added_subtypes: vec![Subtype::Elf], added_abilities: vec![2, 2, 5],
        };
        let mapped = counters.clone().try_map_payloads(Ok::<_, String>, |value| Ok(value + 10), Ok, Ok).unwrap();
        let json = serde_json::to_value(&mapped).unwrap();
        assert_eq!(serde_json::from_value::<Action>(json.clone()).unwrap(), mapped);
        let restored = mapped.try_map_payloads(Ok::<_, String>, |value| Ok(value - 10), Ok, Ok).unwrap();
        assert_eq!(restored, counters);
        for field in ["count_condition", "otherwise_count", "count", "added_abilities", "added_subtypes", "counter_type"] {
            let mut missing = json.clone(); missing["EnterWithCounters"].as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<Action>(missing).is_err(), "missing {field}");
        }
        let mut unknown = json; unknown["EnterWithCounters"]["unknown_body"] = serde_json::json!(3);
        assert!(serde_json::from_value::<Action>(unknown).is_err());
        for variant in ["DoubleCounters", "HalveCounters"] {
            let mut absent = serde_json::Map::new(); absent.insert(variant.into(), serde_json::json!({}));
            assert!(serde_json::from_value::<Action>(serde_json::Value::Object(absent)).is_err());
            let mut explicit = serde_json::Map::new(); explicit.insert(variant.into(), serde_json::json!({"counter_type": null}));
            assert!(serde_json::from_value::<Action>(serde_json::Value::Object(explicit)).is_ok());
        }
    }

    #[test]
    fn complete_replacement_descriptor_mapping_preserves_every_capture() {
        let original = complete_descriptor_fixture();
        let mapped = original.clone().try_map_payloads(
            Ok::<_, String>, Ok, Ok, Ok,
        ).unwrap();
        assert_eq!(mapped.id, original.id);
        assert_eq!(mapped.registration_id, original.registration_id);
        assert_eq!(mapped.source, original.source);
        assert_eq!(mapped.controller, original.controller);
        assert_eq!(mapped.replacement, original.replacement);
        assert_eq!(mapped.priority_override, original.priority_override);
        assert_eq!(mapped.static_ability_instance, original.static_ability_instance);
        assert_eq!(mapped.ability_origin, original.ability_origin);
        assert_eq!(mapped.optional, original.optional);
        assert_eq!(mapped.application_key(), original.application_key());
        assert_eq!(mapped.matcher.as_ref().unwrap().as_ref().downcast_ref::<WouldGainLifeMatcher>().unwrap().player_filter,
            original.matcher.as_ref().unwrap().as_ref().downcast_ref::<WouldGainLifeMatcher>().unwrap().player_filter);
        let mut absent = original;
        absent.matcher = None; absent.static_ability_instance = None; absent.ability_origin = None;
        let mapped: ReplacementEffect = absent.try_map_payloads(Ok::<_, String>, |_| panic!("absent matcher"),
            |_| panic!("absent static occurrence"), |_| panic!("absent origin")).unwrap();
        assert!(mapped.matcher.is_none());
        assert!(mapped.static_ability_instance.is_none());
        assert!(mapped.ability_origin.is_none());
    }

    #[test]
    fn complete_replacement_descriptor_mapping_propagates_each_payload_failure() {
        use std::cell::Cell;
        for failing in 0..4 {
            let visited = Cell::new(0usize);
            let record = |stage, value| {
                visited.set(visited.get() + 1);
                if stage == failing { Err(stage) } else { Ok(value) }
            };
            let result = complete_descriptor_fixture().try_map_payloads(
                |_| record(0, 0u8), |_| record(1, 1u8),
                |_| record(2, 2u8), |_| record(3, 3u8),
            );
            assert_eq!(result.unwrap_err(), failing);
            assert_eq!(visited.get(), failing + 1, "do not publish or continue after a failed payload");
        }
    }

    #[cfg(feature = "serialization")]
    #[test]
    fn complete_replacement_descriptor_schema_requires_all_fields_and_explicit_options() {
        type Encoded = ReplacementEffect<u8, u8, u8, u8>;
        let encoded: Encoded = complete_descriptor_fixture().try_map_payloads(
            |_| Ok::<_, String>(10), |_| Ok(11), |_| Ok(12), |_| Ok(13),
        ).unwrap();
        let json = serde_json::to_value(&encoded).unwrap();
        assert_eq!(serde_json::from_value::<Encoded>(json.clone()).unwrap(), encoded);
        let fields = json.as_object().unwrap();
        assert_eq!(fields.len(), 10);
        for field in fields.keys() {
            let mut missing = json.clone(); missing.as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<Encoded>(missing).is_err(), "missing {field}");
        }
        let mut unknown = json.clone(); unknown["unknown_payload"] = serde_json::json!(1);
        assert!(serde_json::from_value::<Encoded>(unknown).is_err());
        let mut explicit_null = json;
        for field in ["registration_id", "priority_override", "matcher", "static_ability_instance", "ability_origin"] {
            explicit_null[field] = serde_json::Value::Null;
        }
        let restored = serde_json::from_value::<Encoded>(explicit_null).unwrap();
        assert!(restored.registration_id.is_none() && restored.priority_override.is_none());
        assert!(restored.matcher.is_none() && restored.static_ability_instance.is_none() && restored.ability_origin.is_none());
        assert_eq!(restored.source, encoded.source);
        assert_eq!(restored.controller, encoded.controller);
        assert_eq!(restored.replacement, encoded.replacement);
        assert!(restored.optional);
    }

    #[test]
    fn registered_replacement_transport_preserves_consumption_expiry_and_identity() {
        let (original, ids) = registered_transport_fixture();
        let state = original.registered_state().unwrap();
        assert_eq!(state.effects.iter().map(|effect| effect.id).collect::<Vec<_>>(), ids);
        let keys = state.effects.iter().map(ReplacementEffect::application_key).collect::<Vec<_>>();
        let mut restored = ReplacementEffectManager::new();
        restored.add_static_ability_effect(ReplacementEffect::indestructible(ObjectId::from_raw(91), PlayerId::from_index(0)));
        restored.restore_registered_state(state).unwrap();
        assert_eq!(restored.effects().iter().map(ReplacementEffect::application_key).collect::<Vec<_>>(), keys);
        assert_eq!(restored.next_id(), original.next_id());
        assert!(restored.mark_effect_used(ids[1]));
        restored.consume_pending_batch_one_shot_effects();
        assert!(restored.get_effect(ids[1]).is_none());
        assert!(restored.get_effect(ids[2]).is_none());
        restored.clear_until_end_of_turn_effects();
        assert!(restored.get_effect(ids[3]).is_none());
        restored.expire_at_turn_start(8, &[PlayerId::from_index(0)]);
        assert!(restored.get_effect(ids[4]).is_some());
        restored.expire_at_turn_start(9, &[PlayerId::from_index(0)]);
        assert!(restored.get_effect(ids[4]).is_none(), "departed player's actual next-turn boundary retained");
        assert!(restored.get_effect(ids[0]).is_some());
        let next = restored.add_resolution_effect(original.get_effect(ids[0]).unwrap().clone());
        assert_eq!(next.0, original.next_id());
    }

    #[test]
    fn registered_replacement_action_mapping_executes_each_retained_instead_body() {
        use crate::effects::{EffectExecutor, ExecutionContext};
        use std::cell::Cell;
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let body = Effect::gain_life_player(3, ChooseSpec::Player(PlayerFilter::Opponent));
        let mut original = ReplacementEffectManager::new();
        let id = original.add_until_end_of_turn_effect(ReplacementEffect::with_matcher(
            source, bob, WouldGainLifeMatcher::you(), ReplacementAction::Instead(vec![body.clone(), body]),
        ));
        let calls = Cell::new(0);
        let state = original.registered_state().unwrap().try_map_effects(|effect|
            effect.try_map_payloads(|action| action.try_map_payloads(
                |body| { calls.set(calls.get() + 1); Ok::<_, String>(body) }, Ok, Ok, Ok,
            ), Ok, Ok, Ok)
        ).unwrap();
        assert_eq!(calls.get(), 2, "retain both equal independent executable occurrences");
        assert_eq!(state.effects[0].application_key(), ReplacementEffectKey::Registered(id));
        game.effect_store.replacement_effects.restore_registered_state(state).unwrap();
        let effect = crate::effects::GainLifeEffect::new(2, ChooseSpec::Player(PlayerFilter::You));
        let mut context = ExecutionContext::new_default(source, bob);
        let outcome = effect.execute(&mut game, &mut context).unwrap();
        assert_eq!(game.player(bob).unwrap().life, 20, "do not also commit the original gain");
        assert_eq!(game.player(alice).unwrap().life, 26, "both bodies use the captured Bob controller");
        let gains = outcome.events.iter().filter_map(|event|
            event.downcast::<crate::events::LifeGainEvent>()
        ).collect::<Vec<_>>();
        assert_eq!(gains.len(), 2);
        assert!(gains.iter().all(|event| event.player == alice && event.amount == 3));
        game.effect_store.replacement_effects.clear_until_end_of_turn_effects();
        effect.execute(&mut game, &mut context).unwrap();
        assert_eq!(game.player(bob).unwrap().life, 22, "retained replacement expires normally");
        assert_eq!(game.player(alice).unwrap().life, 26);
    }

    #[test]
    fn registered_replacement_transport_executes_captured_controller_after_restore() {
        use crate::effects::{EffectExecutor, ExecutionContext};
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut original = ReplacementEffectManager::new();
        original.add_until_next_turn_effect(ReplacementEffect::with_matcher(
            source, bob, WouldGainLifeMatcher::you(), ReplacementAction::Double,
        ), bob, 4);
        let state = original.registered_state().unwrap().try_map_effects(|effect|
            effect.try_map_payloads(Ok::<_, String>, Ok, Ok, Ok)
        ).unwrap();
        game.effect_store.replacement_effects.restore_registered_state(state).unwrap();
        let effect = crate::effects::GainLifeEffect::new(2, ChooseSpec::Player(PlayerFilter::You));
        let mut bob_ctx = ExecutionContext::new_default(source, bob);
        effect.execute(&mut game, &mut bob_ctx).unwrap();
        assert_eq!(game.player(bob).unwrap().life, 24);
        let mut alice_ctx = ExecutionContext::new_default(source, alice);
        effect.execute(&mut game, &mut alice_ctx).unwrap();
        assert_eq!(game.player(alice).unwrap().life, 22);
        game.effect_store.replacement_effects.expire_at_turn_start(5, &[alice]);
        effect.execute(&mut game, &mut bob_ctx).unwrap();
        assert_eq!(game.player(bob).unwrap().life, 28);
        game.effect_store.replacement_effects.expire_at_turn_start(6, &[bob]);
        effect.execute(&mut game, &mut bob_ctx).unwrap();
        assert_eq!(game.player(bob).unwrap().life, 30);
    }

    #[test]
    fn registered_replacement_transport_capture_diagnoses_inconsistent_native_state() {
        let (valid, ids) = registered_transport_fixture();
        for case in 0..4 {
            let mut manager = valid.clone();
            match case {
                0 => manager.effects.iter_mut().find(|effect| effect.id == ids[0]).unwrap().registration_id = None,
                1 => { manager.one_shot_effects.insert(ReplacementEffectId(manager.next_id + 1)); },
                2 => { manager.effect_sources.insert(manager.next_id + 1, ReplacementEffectSource::StaticAbility); },
                3 => manager.next_id = ids[4].0,
                _ => unreachable!(),
            }
            let before = format!("{manager:?}");
            assert!(manager.registered_state().is_err(), "inconsistent native state {case} cannot disappear from export");
            assert_eq!(format!("{manager:?}"), before);
        }
    }

    #[test]
    fn registered_replacement_transport_rejects_malformed_state_atomically() {
        let (mut manager, ids) = registered_transport_fixture();
        let valid = manager.registered_state().unwrap();
        let before = format!("{manager:?}");
        for case in 0..15 {
            let mut state = valid.clone();
            let foreign = ReplacementEffectId(state.next_id + 1);
            match case {
                0 => state.next_id = u64::MAX,
                1 => state.effects.push(state.effects[0].clone()),
                2 => state.effects[0].registration_id = None,
                3 => state.effects[0].registration_id = Some(ids[1]),
                4 => state.next_id = ids[4].0,
                5 => state.effect_sources.push(state.effect_sources[0]),
                6 => state.effect_sources.push((foreign, ReplacementEffectSource::Resolution)),
                7 => state.effect_sources[0].1 = ReplacementEffectSource::StaticAbility,
                8 => state.one_shot_effects.push(foreign),
                9 => state.one_shot_effects.push(ids[1]),
                10 => state.batch_one_shot_effects.push(ids[1]),
                11 => state.pending_batch_one_shot_effects.push(ids[0]),
                12 => state.until_end_of_turn_effects.push(foreign),
                13 => state.until_next_turn_effects.push(state.until_next_turn_effects[0]),
                14 => state.until_next_turn_effects.push((foreign, (PlayerId::from_index(1), 4, None))),
                _ => unreachable!(),
            }
            assert!(manager.restore_registered_state(state).is_err(), "malformed case {case}");
            assert_eq!(format!("{manager:?}"), before, "failure must not publish any field: {case}");
        }
        manager.restore_registered_state(valid).unwrap();
        manager.consume_pending_batch_one_shot_effects();
        assert!(manager.get_effect(ids[2]).is_none());
    }

    #[test]
    fn source_removal_clears_lifetime_state_without_touching_other_registrations() {
        let mut manager = ReplacementEffectManager::new();
        let player = PlayerId::from_index(0);
        let removed_source = ObjectId::from_raw(71);
        let kept_source = ObjectId::from_raw(72);
        let register = |manager: &mut ReplacementEffectManager, source| {
            let effect = ReplacementEffect::with_matcher(
                source, player, WouldGainLifeMatcher::you(), ReplacementAction::Double,
            );
            let static_id = manager.add_static_ability_effect(effect.clone());
            let resolution = manager.add_resolution_effect(effect.clone());
            let one_shot = manager.add_one_shot_effect(effect.clone());
            let batch = manager.add_batch_one_shot_effect(effect.clone());
            assert!(manager.mark_effect_used(batch));
            let cleanup = manager.add_until_end_of_turn_effect(effect.clone());
            let next_turn = manager.add_until_next_turn_effect(effect, player, 4);
            [static_id, resolution, one_shot, batch, cleanup, next_turn]
        };
        let removed = register(&mut manager, removed_source);
        let kept = register(&mut manager, kept_source);
        manager.prepare_for_departing_player(player, 9);
        let next_id = manager.next_id();
        manager.remove_effects_from_source(removed_source);
        for id in removed {
            assert!(manager.get_effect(id).is_none());
            assert!(!manager.effect_sources.contains_key(&id.0), "removed source provenance retained");
            assert!(!manager.one_shot_effects.contains(&id), "removed one-shot retained");
            assert!(!manager.batch_one_shot_effects.contains(&id), "removed batch one-shot retained");
            assert!(!manager.pending_batch_one_shot_effects.contains(&id), "removed pending consumption retained");
            assert!(!manager.until_end_of_turn_effects.contains(&id), "removed cleanup scope retained");
            assert!(!manager.until_next_turn_effects.contains_key(&id), "removed next-turn anchor retained");
        }
        assert_eq!(manager.next_id(), next_id);
        assert_eq!(manager.effects().iter().map(|effect| effect.id).collect::<Vec<_>>(), kept);
        assert_eq!(manager.effect_sources.get(&kept[0].0), Some(&ReplacementEffectSource::StaticAbility));
        assert_eq!(manager.effect_sources.get(&kept[1].0), Some(&ReplacementEffectSource::Resolution));
        assert_eq!(manager.until_next_turn_effects.get(&kept[5]), Some(&(player, 4, Some(9))));
        assert!(manager.is_one_shot(kept[2]));
        assert!(manager.is_one_shot(kept[3]));
        manager.remove_effects_from_source(removed_source);
        manager.consume_pending_batch_one_shot_effects();
        assert!(manager.get_effect(kept[3]).is_none());
        assert!(manager.mark_effect_used(kept[2]));
        manager.clear_until_end_of_turn_effects();
        assert!(manager.get_effect(kept[4]).is_none());
        manager.expire_at_turn_start(8, &[player]);
        assert!(manager.get_effect(kept[5]).is_some());
        manager.expire_at_turn_start(9, &[]);
        assert!(manager.get_effect(kept[5]).is_none());
        manager.clear_static_ability_effects();
        assert_eq!(manager.effects().iter().map(|effect| effect.id).collect::<Vec<_>>(), vec![kept[1]]);
        let replacement = manager.add_resolution_effect(ReplacementEffect::with_matcher(
            removed_source, player, WouldGainLifeMatcher::you(), ReplacementAction::Double,
        ));
        assert_eq!(replacement.0, next_id);
        assert!(!manager.is_one_shot(replacement));
    }


    #[test]
    fn registration_identity_distinguishes_instances_and_survives_payload_changes() {
        let mut manager = ReplacementEffectManager::new();
        let effect = ReplacementEffect::with_matcher(
            ObjectId::from_raw(1),
            PlayerId::from_index(0),
            WouldGainLifeMatcher::you(),
            ReplacementAction::Double,
        );
        let first = manager.add_resolution_effect(effect.clone());
        let second = manager.add_resolution_effect(effect.clone());
        let key = manager.get_effect(first).unwrap().application_key();
        assert_ne!(key, manager.get_effect(second).unwrap().application_key());
        let mut changed = manager.get_effect(first).unwrap().clone();
        changed.replacement = ReplacementAction::Prevent;
        assert_eq!(key, changed.application_key());

        let static_id = manager.add_static_ability_effect(effect.clone());
        let static_key = manager.get_effect(static_id).unwrap().application_key();
        manager.clear_static_ability_effects();
        let regenerated_id = manager.add_static_ability_effect(effect);
        assert_ne!(static_id, regenerated_id);
        assert_eq!(
            static_key,
            manager
                .get_effect(regenerated_id)
                .unwrap()
                .application_key()
        );
        assert_eq!(key, manager.get_effect(first).unwrap().application_key());
    }

    #[test]
    fn optional_alternatives_share_only_their_own_registered_parent_identity() {
        let mut manager = ReplacementEffectManager::new();
        let effect = ReplacementEffect::with_matcher(
            ObjectId::from_raw(1), PlayerId::from_index(0),
            WouldGainLifeMatcher::you(), ReplacementAction::Double,
        ).optional();
        let first = manager.add_resolution_effect(effect.clone());
        let second = manager.add_resolution_effect(effect);
        let decline = manager.get_effect(first).unwrap().optional_decline_effect().unwrap();
        let declined = manager.add_resolution_effect(decline);
        assert_eq!(manager.get_effect(first).unwrap().application_key(), manager.get_effect(declined).unwrap().application_key());
        assert_ne!(manager.get_effect(second).unwrap().application_key(), manager.get_effect(declined).unwrap().application_key());
    }

    /// CR 701.19c: "can't be regenerated" removes regeneration shields only,
    /// not other one-shot replacements sourced from the same permanent.
    #[test]
    fn removing_regeneration_shields_keeps_other_one_shot_effects() {
        let source = ObjectId::from_raw(7);
        let controller = PlayerId::from_index(0);
        let mut manager = ReplacementEffectManager::new();
        let shield = manager.add_one_shot_effect(ReplacementEffect::with_matcher(
            source,
            controller,
            crate::events::permanents::matchers::RegenerationShieldMatcher::new(source),
            ReplacementAction::Instead(Vec::new()),
        ));
        let other = manager.add_one_shot_effect(ReplacementEffect::with_matcher(
            source,
            controller,
            crate::events::permanents::matchers::ThisWouldBeDestroyedMatcher,
            ReplacementAction::Instead(Vec::new()),
        ));
        manager.remove_regeneration_shields_from_source(source);
        assert!(manager.get_effect(shield).is_none(), "regeneration shield removed");
        assert!(manager.get_effect(other).is_some(), "unrelated replacement kept");
    }

    #[test]
    fn test_damage_prevention() {
        let effect =
            ReplacementEffect::prevent_damage(ObjectId::from_raw(1), PlayerId::from_index(0), 3);

        assert!(
            effect.matcher.is_some(),
            "prevent_damage should use trait-based matcher"
        );
        assert!(matches!(
            effect.replacement,
            ReplacementAction::PreventDamageAmount(3)
        ));
    }

    #[test]
    fn test_enters_with_counters() {
        let effect = ReplacementEffect::enters_with_counters(
            ObjectId::from_raw(1),
            PlayerId::from_index(0),
            CounterType::PlusOnePlusOne,
            Value::Fixed(3),
        );

        assert_eq!(effect.priority_override, None);
        assert!(
            effect.matcher.is_some(),
            "enters_with_counters should use trait-based matcher"
        );
        assert!(matches!(
            effect.replacement,
            ReplacementAction::EnterWithCounters { .. }
        ));
    }

    #[test]
    fn test_exile_instead_of_dying() {
        let effect = ReplacementEffect::exile_instead_of_dying(
            ObjectId::from_raw(1),
            PlayerId::from_index(0),
        );

        assert_eq!(effect.priority_override, None);
        assert!(
            effect.matcher.is_some(),
            "exile_instead_of_dying should use trait-based matcher"
        );
        assert!(matches!(
            effect.replacement,
            ReplacementAction::ChangeDestination(Zone::Exile)
        ));
    }

    #[test]
    fn test_replacement_manager() {
        let mut manager = ReplacementEffectManager::new();

        let effect1 =
            ReplacementEffect::prevent_damage(ObjectId::from_raw(1), PlayerId::from_index(0), 3);
        let effect2 = ReplacementEffect::enters_with_counters(
            ObjectId::from_raw(2),
            PlayerId::from_index(0),
            CounterType::PlusOnePlusOne,
            Value::Fixed(2),
        );

        let id1 = manager.add_effect(effect1);
        let id2 = manager.add_effect(effect2);

        // Effects are tracked
        assert_eq!(manager.effects().len(), 2);

        // Both effects have matchers (used by the new trait-based system)
        assert!(manager.effects().iter().all(|e| e.matcher.is_some()));

        // Remove one effect
        manager.remove_effect(id1);
        assert_eq!(manager.effects().len(), 1);
        assert_eq!(manager.effects()[0].id, id2);
    }

    #[test]
    fn test_priority_override() {
        let effect = ReplacementEffect::with_matcher(
            ObjectId::from_raw(1),
            PlayerId::from_index(0),
            ThisWouldEnterBattlefieldMatcher,
            ReplacementAction::EnterTapped,
        )
        .with_priority_override(ReplacementPriority::CopyEffect);

        assert_eq!(
            effect.priority_override,
            Some(ReplacementPriority::CopyEffect)
        );
    }
}

#[cfg(test)]
mod ability_origin_identity_tests {
    use super::*;
    #[test]
    fn originating_occurrences_survive_controller_payload_and_optional_changes() {
        let source=ObjectId::from_raw(90100); let alice=PlayerId::from_index(0);
        let ability=crate::static_abilities::StaticAbility::double_life_change_replacement(
            PlayerFilter::You, false, "Double life gain");
        let first=ability.generate_replacement_effect(source, alice).unwrap()
            .with_ability_origin(crate::continuous::AbilityOrigin::Printed(0), None, 0);
        let second=ability.generate_replacement_effect(source, alice).unwrap()
            .with_ability_origin(crate::continuous::AbilityOrigin::Printed(1), None, 0);
        assert_ne!(first.application_key(), second.application_key(), "cloned static values are distinct originating slots");
        let mut changed=first.clone(); changed.controller=PlayerId::from_index(1);
        changed.replacement=ReplacementAction::Prevent;
        assert_eq!(changed.application_key(), first.application_key());
        let decline=first.clone().optional().optional_decline_effect().unwrap();
        assert_eq!(decline.application_key(), first.application_key());
        let sibling=first.clone().with_ability_origin(crate::continuous::AbilityOrigin::Printed(0), None, 1);
        assert_ne!(sibling.application_key(), first.application_key(), "generated siblings retain branch identity");
    }
}

// Pure production rewrites are exposed independently of event matching. This
// does not assert that a replacement matches, or that it is safe to reorder.
impl<E, A, P, K> ReplacementAction<E, A, P, K> {
    pub fn mana_transformation(&self) -> Option<crate::events::mana::ManaTransformation<'_>> {
        use crate::events::mana::ManaTransformation;
        match self {
            Self::ReplaceMana(symbols) => Some(ManaTransformation::ReplaceTypes(symbols)),
            Self::ReplaceManaExact(symbols) => Some(ManaTransformation::ReplaceExact(symbols)),
            Self::Modify(EventModification::Multiply(factor)) => Some(ManaTransformation::Multiply(*factor)),
            _ => None,
        }
    }
}

#[cfg(all(test, feature = "serialization"))]
mod replacement_identity_binding_tests {
    use super::*;
    use crate::continuous::{AbilityEffectOrigin, AbilityOrigin, ContinuousAbilityOrigin,
        ContinuousEffect, EffectTarget, Modification};
    use crate::ids::CardId;
    use crate::static_abilities::StaticAbility;

    fn origin_fixture() -> (ReplacementAbilityOrigin, [StaticAbilityInstanceId; 2], [CardId; 2]) {
        let source = ObjectId::from_raw(71);
        // Equal semantic abilities still have independent native occurrences.
        let first = StaticAbility::hexproof(); let second = StaticAbility::hexproof();
        assert_ne!(first.instance_id(), second.instance_id());
        let faces = [CardId::new(), CardId::new()];
        let inner = ContinuousEffect::new(source, PlayerId::from_index(1),
            EffectTarget::AllPermanents, Modification::AddAbility(second.clone()))
            .with_originating_static_ability(second.clone());
        let mut outer = ContinuousEffect::new(source, PlayerId::from_index(1),
            EffectTarget::AllPermanents, Modification::AddAbility(first.clone()))
            .with_originating_static_ability(first.clone());
        outer.originating_ability = Some(Box::new(ContinuousAbilityOrigin {
            host: ObjectId::from_raw(72),
            ability: AbilityOrigin::Level { printed_face: Some(faces[1]),
                parent: Box::new(AbilityOrigin::Effect { effect: AbilityEffectOrigin::from(&inner), slot: 8 }),
                tier: 2, slot: 3 },
            printed_face: Some(faces[0]), branch: 5,
        }));
        (ReplacementAbilityOrigin {
            ability: AbilityOrigin::Borrowed { effect: AbilityEffectOrigin::from(&outer),
                source: ObjectId::from_raw(73),
                origin: Box::new(AbilityOrigin::Level { printed_face: Some(faces[1]),
                    parent: Box::new(AbilityOrigin::Printed(4)), tier: 6, slot: 7 }) },
            printed_face: Some(faces[0]), branch: 9,
        }, [first.instance_id(), second.instance_id()], faces)
    }

    #[test]
    fn replacement_identity_binding_rebinds_nested_keys_and_decline_parent() {
        let (origin, instances, faces) = origin_fixture();
        let original = ReplacementEffect::with_matcher(ObjectId::from_raw(74),
            PlayerId::from_index(1), WouldGainLifeMatcher::you(), ReplacementAction::Double)
            .with_ability_origin(origin.ability.clone(), origin.printed_face, origin.branch).optional();
        let old_key = original.application_key();
        let mut static_visits = Vec::new(); let mut face_visits = Vec::new();
        let wire: ReplacementEffectKey<u32, u32> = old_key.clone()
            .try_map_static_instances(&mut |id| {
                static_visits.push(id);
                if id == instances[0] { Ok::<_, String>(3) }
                else if id == instances[1] { Ok(5) } else { Err("unbound occurrence".into()) }
            }).unwrap()
            .try_map_card_ids(&mut |face| {
                face_visits.push(face);
                if face == faces[0] { Ok::<_, String>(11) }
                else if face == faces[1] { Ok(13) } else { Err("unbound face".into()) }
            }).unwrap();
        assert_eq!(static_visits, instances);
        assert_eq!(face_visits.len(), 4);
        assert_eq!(face_visits.iter().filter(|face| **face == faces[0]).count(), 2);
        assert_eq!(face_visits.iter().filter(|face| **face == faces[1]).count(), 2);
        let bytes = serde_json::to_vec(&wire).unwrap();
        let decoded: ReplacementEffectKey<u32, u32> = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded, wire);
        let fresh_instances = [StaticAbility::hexproof().instance_id(), StaticAbility::hexproof().instance_id()];
        let fresh_faces = [CardId::new(), CardId::new()];
        assert!(fresh_instances.iter().all(|id| !instances.contains(id)));
        let restored: ReplacementEffectKey = decoded
            .try_map_static_instances(&mut |id| match id { 3 => Ok::<_, String>(fresh_instances[0]),
                5 => Ok(fresh_instances[1]), _ => Err("unknown occurrence reference".into()) }).unwrap()
            .try_map_card_ids(&mut |id| match id { 11 => Ok::<_, String>(fresh_faces[0]),
                13 => Ok(fresh_faces[1]), _ => Err("unknown card reference".into()) }).unwrap();
        let ReplacementEffectKey::Ability { source, origin: restored_origin } = &restored else { panic!("ability key") };
        let descriptor = ReplacementEffect::with_matcher(*source, original.controller,
            WouldGainLifeMatcher::you(), ReplacementAction::Double)
            .with_ability_origin(restored_origin.ability.clone(), restored_origin.printed_face,
                restored_origin.branch).optional();
        assert_eq!(restored, descriptor.application_key());
        assert_eq!(restored, descriptor.optional_decline_effect().unwrap().application_key());
        assert_ne!(restored, old_key, "native allocations changed, so stale keys cannot substitute");
        let rebound = restored.try_map_static_instances(&mut |id|
            if id == fresh_instances[0] { Ok::<_, String>(3) }
            else if id == fresh_instances[1] { Ok(5) } else { Err("unbound fresh occurrence".into()) }
        ).unwrap().try_map_card_ids(&mut |id|
            if id == fresh_faces[0] { Ok::<_, String>(11) }
            else if id == fresh_faces[1] { Ok(13) } else { Err("unbound fresh face".into()) }
        ).unwrap();
        assert_eq!(rebound, wire, "every nested capture survives fresh-native reconstruction");
    }

    #[test]
    fn replacement_identity_binding_rejects_unbound_nested_references() {
        let (origin, instances, faces) = origin_fixture();
        let key = ReplacementEffectKey::Ability { source: ObjectId::from_raw(74), origin };
        let failed = key.clone().try_map_static_instances(&mut |id|
            if id == instances[0] { Ok(3u32) } else { Err("nested occurrence") });
        assert_eq!(failed.unwrap_err(), "nested occurrence");
        let failed = key.try_map_card_ids(&mut |face|
            if face == faces[0] { Ok(11u32) } else { Err("nested face") });
        assert_eq!(failed.unwrap_err(), "nested face");
    }

    #[test]
    fn replacement_identity_binding_schema_requires_explicit_capture_fields() {
        type Key = ReplacementEffectKey<u32, u32>;
        let key = Key::Regenerated { source: ObjectId::from_raw(88), controller: PlayerId::from_index(1),
            static_ability_instance: Some(7), matcher: Some("identity text".into()), replacement: "identity only".into() };
        let json = serde_json::to_value(&key).unwrap();
        assert_eq!(serde_json::from_value::<Key>(json.clone()).unwrap(), key);
        for field in ["source", "controller", "static_ability_instance", "matcher", "replacement"] {
            let mut missing = json.clone(); missing["Regenerated"].as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<Key>(missing).is_err(), "missing {field}");
        }
        let origin = ReplacementAbilityOrigin::<u32, u32> {
            ability: AbilityOrigin::Printed(4), printed_face: None, branch: 8,
        };
        let json = serde_json::to_value(&origin).unwrap();
        for field in ["ability", "printed_face", "branch"] {
            let mut missing = json.clone(); missing.as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<ReplacementAbilityOrigin<u32, u32>>(missing).is_err(), "missing {field}");
        }
        let mut unknown = json; unknown["unknown_origin"] = serde_json::json!(1);
        assert!(serde_json::from_value::<ReplacementAbilityOrigin<u32, u32>>(unknown).is_err());
        let registered = Key::Registered(ReplacementEffectId(19));
        let mut visits = 0;
        let mapped = registered.try_map_static_instances(&mut |value| { visits += 1; Ok::<_, String>(value) }).unwrap()
            .try_map_card_ids(&mut |value| { visits += 1; Ok::<_, String>(value) }).unwrap();
        assert_eq!(visits, 0);
        assert_eq!(mapped, Key::Registered(ReplacementEffectId(19)));
    }
}
