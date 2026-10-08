use crate::zone::Zone;
pub(crate) mod price_routes;
pub mod play_permission;
pub mod blind_play;
pub use ironsmith_core::{AlternativeCastRequirements, TrapCondition};

pub type AlternativeCastingMethod = ironsmith_core::AlternativeCastingMethod<
    crate::effect::Effect,
    crate::costs::Cost,
    crate::static_abilities::ThisSpellCostCondition,
>;

/// Route alternatives that first take a special action from hand. Ordinary
/// alternative casts use the generic spell-casting pipeline. Keep this match
/// exhaustive so a new mechanic cannot silently miss hand-action discovery.
pub(crate) fn hand_special_action(
    method: &AlternativeCastingMethod,
    card_id: crate::ids::ObjectId,
) -> Option<crate::special_actions::SpecialAction> {
    use crate::special_actions::SpecialAction;
    match method {
        AlternativeCastingMethod::Plot { .. } => Some(SpecialAction::Plot { card_id }),
        AlternativeCastingMethod::Foretell { .. } => Some(SpecialAction::Foretell { card_id }),
        AlternativeCastingMethod::Suspend { .. } => Some(SpecialAction::Suspend { card_id }),
        AlternativeCastingMethod::Dash { .. }
        | AlternativeCastingMethod::Blitz { .. }
        | AlternativeCastingMethod::Warp { .. }
        | AlternativeCastingMethod::Disturb { .. }
        | AlternativeCastingMethod::Overload { .. }
        | AlternativeCastingMethod::Cleave { .. }
        | AlternativeCastingMethod::Awaken { .. }
        | AlternativeCastingMethod::Flashback { .. }
        | AlternativeCastingMethod::Harmonize { .. }
        | AlternativeCastingMethod::Retrace { .. }
        | AlternativeCastingMethod::JumpStart { .. }
        | AlternativeCastingMethod::Escape { .. }
        | AlternativeCastingMethod::Madness { .. }
        | AlternativeCastingMethod::Miracle { .. }
        | AlternativeCastingMethod::FlashWithAdditionalCost { .. }
        | AlternativeCastingMethod::Composed { .. }
        | AlternativeCastingMethod::FromZone { .. }
        | AlternativeCastingMethod::Trap { .. }
        | AlternativeCastingMethod::Bestow { .. }
        | AlternativeCastingMethod::Mutate { .. } => None,
    }
}

/// A granted alternative keyword has the same battlefield abilities as an
/// intrinsic instance (CR 702.109a, 702.152a). Carry those abilities with the
/// resolving spell, without copying its paid-cost state onto permanent copies.
pub(crate) fn ensure_alternative_battlefield_abilities(
    object: &mut crate::object::Object,
    method: &AlternativeCastingMethod,
    current_turn: u32,
) {
    let (name, blitz) = match method {
        AlternativeCastingMethod::Dash { .. } => ("Dash", false),
        AlternativeCastingMethod::Blitz { .. } => ("Blitz", true),
        _ => return,
    };
    let condition = crate::ConditionExpr::ThisSpellPaidLabel(name.into());
    let haste = crate::ability::Ability::static_ability(crate::static_abilities::StaticAbility::haste());
    let death_draw = crate::ability::Ability::triggered(
        crate::triggers::Trigger::this_dies(),
        vec![crate::effect::Effect::target_draws(1, crate::target::PlayerFilter::You)],
    );
    let abilities = object.abilities.iter().filter_map(|ability| match &ability.kind {
        crate::ability::AbilityKind::Static(ability) => Some(ability.clone()),
        _ => None,
    }).chain(object.temporary_static_ability_grants.iter().filter(|grant| !grant.is_expired(current_turn)).filter_map(|grant| grant.materialize()));
    let mut has_haste = false;
    let mut has_death_draw = false;
    for ability in abilities {
        if ability.granted_inline_condition() != Some(&condition) { continue; }
        for granted in ability.source_granted_inline_abilities() {
            has_haste |= granted == &haste;
            has_death_draw |= is_blitz_death_draw_ability(granted);
        }
    }
    let mut missing = Vec::new();
    if !has_haste { missing.push(haste); }
    if blitz && !has_death_draw { missing.push(death_draw); }
    if missing.is_empty() { return; }
    let mut grant = crate::static_abilities::GrantObjectAbilityForFilter::new(
        crate::target::ObjectFilter::source(),
        missing.remove(0),
        format!("{name} battlefield abilities"),
    ).with_condition(condition);
    grant.additional_abilities = missing;
    let ability = crate::static_abilities::StaticAbility::new(grant);
    object.temporary_static_ability_grants.push(crate::object::TemporaryStaticAbilityGrant {
        ability: ability.id(), ability_payload: Some(ability), expires_end_of_turn: Some(u32::MAX),
    });
}

/// Identify Blitz's ordinary death ability structurally. Runtime effects do
/// not implement semantic `PartialEq`, so comparing complete abilities would
/// fail to recognize an already-defined component and grant it twice.
pub fn is_blitz_death_draw_ability(ability: &crate::ability::Ability) -> bool {
    let crate::ability::AbilityKind::Triggered(triggered) = &ability.kind else { return false; };
    if ability.functional_zones.as_slice() != [Zone::Battlefield]
        || !triggered.choices.is_empty()
        || triggered.intervening_if.is_some()
        || triggered.trigger.downcast_ref::<crate::triggers::ZoneChangeTrigger>()
            != Some(&crate::triggers::ZoneChangeTrigger::this_dies())
    {
        return false;
    }
    let [segment] = triggered.effects.segments.as_slice() else { return false; };
    if !segment.self_replacements.is_empty() { return false; }
    let [effect] = segment.default_effects.as_slice() else { return false; };
    effect.downcast_ref::<crate::effects::DrawCardsEffect>().is_some_and(|draw| {
        draw.count == crate::effect::Value::Fixed(1)
            && draw.player == crate::target::PlayerFilter::You
    })
}

/// Exact native permission plus the source and ordinal used by public action
/// references. The ordinal is local to the checked announcement snapshot;
/// execution validates the identity, never retargets by ordinal after payment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantSelection {
    pub identity: crate::grant_registry::GrantPermissionIdentity,
    pub source: crate::ids::ObjectId,
    pub index: usize,
}

/// Which method is being used to cast a spell.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serialization", derive(serde::Serialize, serde::Deserialize))]
pub enum CastingMethod {
    #[default]
    Normal,
    FaceDown,
    SplitOtherHalf,
    Fuse,
    Alternative(usize),
    GrantedEscape {
        source: crate::ids::ObjectId,
        exile_count: u32,
    },
    GrantedFlashback,
    PlayFrom {
        source: crate::ids::ObjectId,
        zone: Zone,
        use_alternative: Option<usize>,
    },
    SplitOtherHalfPlayFrom {
        source: crate::ids::ObjectId,
        zone: Zone,
        use_alternative: Option<usize>,
    },
    /// A printed morph/disguise (or separately granted face-down) cast using
    /// this exact zone permission. Appended for serialized ordinal stability.
    FaceDownPlayFrom {
        source: crate::ids::ObjectId,
        zone: Zone,
    },
    /// A separately authorized origin plus one independently selected price.
    /// Nested price routes and origins that already replace the mana cost are
    /// rejected before announcement. Both identities are locked at selection.
    #[cfg_attr(feature = "serialization", serde(skip))]
    AlternativePrice {
        origin: Box<CastingMethod>,
        origin_permission: Option<GrantSelection>,
        price: GrantSelection,
        /// Exact prototype characteristic choice on the selected face. This
        /// is independent of the replacement price (CR 718.3).
        prototype: Option<usize>,
    },
    /// An exact origin permission, independent of a replacement casting price.
    /// Native identity is never reconstructed from a public ordinal alone.
    #[cfg_attr(feature = "serialization", serde(skip))]
    ExactPermission {
        origin: Box<CastingMethod>,
        permission: GrantSelection,
    },
}

impl CastingMethod {
    /// The underlying spell face/origin, without discarding its price receipt.
    /// Validation admits only one layer; this intentionally does not recurse.
    pub fn origin_method(&self) -> &Self {
        match self { Self::AlternativePrice { origin, .. } | Self::ExactPermission { origin, .. } => origin, _ => self }
    }

    /// Preserve a separate price wrapper while inspecting the printed origin
    /// behavior of one exact-permission wrapper. Admission rejects nesting.
    pub fn without_exact_permission(&self) -> &Self {
        match self { Self::ExactPermission { origin, .. } => origin, _ => self }
    }

    pub fn is_alternative(&self) -> bool {
        if let Self::ExactPermission { origin, .. } = self { return origin.is_alternative(); }
        matches!(self, Self::Alternative(_) | Self::FaceDown | Self::FaceDownPlayFrom { .. } | Self::AlternativePrice { .. })
    }

    pub fn exiles_after_resolution(&self) -> bool {
        if let Self::ExactPermission { origin, .. } = self { return origin.exiles_after_resolution(); }
        matches!(
            self,
            Self::GrantedFlashback | Self::SplitOtherHalfPlayFrom { use_alternative: Some(_), .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mana::{ManaCost, ManaSymbol};

    #[test]
    fn test_flashback_properties() {
        let flashback = AlternativeCastingMethod::Flashback {
            x_minimum: 0,
            total_cost: crate::cost::TotalCost::mana(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(2)],
                vec![ManaSymbol::Blue],
            ])),
        };

        assert_eq!(flashback.cast_from_zone(), Zone::Graveyard);
        assert!(flashback.exiles_after_resolution());
        assert!(flashback.mana_cost().is_some());
        assert_eq!(flashback.name(), "Flashback");
    }

    #[test]
    fn test_jump_start_properties() {
        let jump_start = AlternativeCastingMethod::JumpStart {
            additional_cost: crate::cost::TotalCost::from_cost(crate::costs::Cost::discard(
                1, None,
            )),
        };

        assert_eq!(jump_start.cast_from_zone(), Zone::Graveyard);
        assert!(jump_start.exiles_after_resolution());
        assert!(jump_start.mana_cost().is_none());
        assert_eq!(jump_start.name(), "Jump-start");
    }

    #[test]
    fn test_escape_properties() {
        let escape = AlternativeCastingMethod::Escape {
            cost: Some(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::Black],
                vec![ManaSymbol::Black],
            ])),
            exile_count: 4,
            additional_cost: crate::cost::TotalCost::from_cost(
                crate::costs::Cost::exile_from_graveyard(4, None),
            ),
        };

        assert_eq!(escape.cast_from_zone(), Zone::Graveyard);
        assert!(!escape.exiles_after_resolution());
        assert!(
            !CastingMethod::GrantedEscape {
                source: crate::ids::ObjectId(1),
                exile_count: 3
            }
            .exiles_after_resolution()
        );
        assert!(escape.mana_cost().is_some());
        assert_eq!(escape.name(), "Escape");
    }

    #[test]
    fn test_casting_method() {
        let normal = CastingMethod::Normal;
        let alternative = CastingMethod::Alternative(0);

        assert!(!normal.is_alternative());
        assert!(alternative.is_alternative());
        assert_eq!(CastingMethod::default(), CastingMethod::Normal);
    }
}
