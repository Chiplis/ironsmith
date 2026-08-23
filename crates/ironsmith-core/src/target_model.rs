use crate::{ChoiceCount, ObjectFilter, ObjectId, PlayerFilter, PlayerId, TagKey, Zone};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceReferenceSurface {
    FullName(String),
    ShortName(String),
    ThisPermanentType(String),
}

/// Oracle-facing noun used when an effect refers back to an object that was
/// sacrificed earlier in the same cost or resolution sequence.
///
/// This is presentation metadata only. Object identity is still carried by a
/// tagged snapshot so runtime characteristic and controller lookups use LKI.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SacrificedObjectKind {
    Creature,
    Artifact,
    Enchantment,
    Permanent,
}

impl SacrificedObjectKind {
    pub const fn noun(self) -> &'static str {
        match self {
            Self::Creature => "creature",
            Self::Artifact => "artifact",
            Self::Enchantment => "enchantment",
            Self::Permanent => "permanent",
        }
    }
}

impl SourceReferenceSurface {
    pub fn display_text(&self) -> String {
        match self {
            Self::FullName(text) | Self::ShortName(text) | Self::ThisPermanentType(text) => {
                text.clone()
            }
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChooseSpecSurfaceHint {
    SourceReference(SourceReferenceSurface),
    SacrificedObject(SacrificedObjectKind),
}

/// Specifies what can be chosen or targeted by an effect.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum ChooseSpec {
    SurfaceHinted {
        spec: Box<ChooseSpec>,
        hints: Vec<ChooseSpecSurfaceHint>,
    },
    Target(Box<ChooseSpec>),
    Player(PlayerFilter),
    Object(ObjectFilter),
    SpecificObject(ObjectId),
    SpecificPlayer(PlayerId),
    AnyTarget,
    AnyOtherTarget,
    /// A single choice from either the matching objects or players.
    ///
    /// Wrap this in [`ChooseSpec::Target`] when the choice is a target.
    ObjectOrPlayer(ObjectFilter, PlayerFilter),
    PlayerOrPlaneswalker(PlayerFilter),
    AttackedPlayerOrPlaneswalker,
    Source,
    SourceController,
    SourceOwner,
    Tagged(TagKey),
    All(ObjectFilter),
    EachPlayer(PlayerFilter),
    Iterated,
    WithCount(Box<ChooseSpec>, ChoiceCount),
    WithCountValue(Box<ChooseSpec>, ChoiceCount, crate::Value),
}

impl ChooseSpec {
    pub fn with_surface_hint(self, hint: ChooseSpecSurfaceHint) -> Self {
        self.with_surface_hints([hint])
    }

    pub fn with_surface_hints(
        self,
        hints: impl IntoIterator<Item = ChooseSpecSurfaceHint>,
    ) -> Self {
        let mut hints_to_add: Vec<ChooseSpecSurfaceHint> = hints.into_iter().collect();
        match self {
            Self::SurfaceHinted {
                spec,
                hints: mut existing,
            } => {
                for hint in hints_to_add.drain(..) {
                    if !existing.contains(&hint) {
                        existing.push(hint);
                    }
                }
                Self::SurfaceHinted {
                    spec,
                    hints: existing,
                }
            }
            spec => Self::SurfaceHinted {
                spec: Box::new(spec),
                hints: hints_to_add,
            },
        }
    }

    pub fn surface_hints(&self) -> &[ChooseSpecSurfaceHint] {
        match self {
            Self::SurfaceHinted { hints, .. } => hints,
            _ => &[],
        }
    }

    pub fn source_reference_surface(&self) -> Option<&SourceReferenceSurface> {
        self.surface_hints().iter().find_map(|hint| match hint {
            ChooseSpecSurfaceHint::SourceReference(surface) => Some(surface),
            ChooseSpecSurfaceHint::SacrificedObject(_) => None,
        })
    }

    pub fn sacrificed_object_kind(&self) -> Option<SacrificedObjectKind> {
        self.surface_hints().iter().find_map(|hint| match hint {
            ChooseSpecSurfaceHint::SacrificedObject(kind) => Some(*kind),
            ChooseSpecSurfaceHint::SourceReference(_) => None,
        })
    }

    pub fn unhinted(&self) -> &ChooseSpec {
        match self {
            Self::SurfaceHinted { spec, .. } => spec.unhinted(),
            spec => spec,
        }
    }

    pub fn into_unhinted(self) -> ChooseSpec {
        match self {
            Self::SurfaceHinted { spec, .. } => spec.into_unhinted(),
            spec => spec,
        }
    }

    pub fn target(inner: ChooseSpec) -> Self {
        match inner {
            // Presentation metadata describes the chosen object, not the
            // target wrapper. Keep it discoverable at the outside just as
            // `with_count` does when adding another semantic wrapper.
            Self::SurfaceHinted { spec, hints } => Self::SurfaceHinted {
                spec: Box::new(Self::target(*spec)),
                hints,
            },
            inner if inner.is_target() => inner,
            inner => Self::Target(Box::new(inner)),
        }
    }

    pub fn is_target(&self) -> bool {
        match self {
            Self::SurfaceHinted { spec, .. } => spec.is_target(),
            Self::Target(_)
            | Self::AnyTarget
            | Self::AnyOtherTarget
            | Self::PlayerOrPlaneswalker(_) => true,
            Self::WithCount(inner, _) | Self::WithCountValue(inner, _, _) => inner.is_target(),
            _ => false,
        }
    }

    pub fn inner(&self) -> &ChooseSpec {
        match self {
            Self::SurfaceHinted { spec, .. } => spec.inner(),
            Self::Target(inner) => inner.as_ref(),
            Self::WithCount(inner, _) | Self::WithCountValue(inner, _, _) => inner.inner(),
            _ => self,
        }
    }

    pub fn base(&self) -> &ChooseSpec {
        match self {
            Self::SurfaceHinted { spec, .. } => spec.base(),
            Self::Target(inner) => inner.base(),
            Self::WithCount(inner, _) | Self::WithCountValue(inner, _, _) => inner.base(),
            _ => self,
        }
    }

    pub fn with_count(self, count: ChoiceCount) -> Self {
        match self {
            Self::SurfaceHinted { spec, hints } => Self::SurfaceHinted {
                spec: Box::new(spec.with_count(count)),
                hints,
            },
            Self::WithCount(inner, _) | Self::WithCountValue(inner, _, _) => {
                Self::WithCount(inner, count)
            }
            other => Self::WithCount(Box::new(other), count),
        }
    }

    pub fn with_count_value(self, count: ChoiceCount, value: crate::Value) -> Self {
        match self {
            Self::SurfaceHinted { spec, hints } => Self::SurfaceHinted {
                spec: Box::new(spec.with_count_value(count, value)),
                hints,
            },
            Self::WithCount(inner, _) | Self::WithCountValue(inner, _, _) => {
                Self::WithCountValue(inner, count, value)
            }
            other => Self::WithCountValue(Box::new(other), count, value),
        }
    }

    pub fn count(&self) -> ChoiceCount {
        match self {
            Self::SurfaceHinted { spec, .. } => spec.count(),
            Self::WithCount(_, count) | Self::WithCountValue(_, count, _) => *count,
            Self::Target(inner) => inner.count(),
            _ => ChoiceCount::default(),
        }
    }

    pub fn count_value(&self) -> Option<&crate::Value> {
        match self {
            Self::SurfaceHinted { spec, .. } | Self::Target(spec) | Self::WithCount(spec, _) => {
                spec.count_value()
            }
            Self::WithCountValue(_, _, value) => Some(value),
            _ => None,
        }
    }

    /// Constraint authored for the selected target set, if any.
    ///
    /// The object filter still describes which individual objects are legal;
    /// this metadata constrains the selection as a whole.
    pub fn target_set_aggregate_constraint(&self) -> Option<&crate::ChoiceAggregateConstraint> {
        match self {
            Self::SurfaceHinted { spec, .. }
            | Self::Target(spec)
            | Self::WithCount(spec, _)
            | Self::WithCountValue(spec, _, _) => spec.target_set_aggregate_constraint(),
            Self::Object(filter) | Self::ObjectOrPlayer(filter, _) => {
                filter.target_set_aggregate_constraint.as_deref()
            }
            _ => None,
        }
    }

    pub fn is_single(&self) -> bool {
        self.count().is_single()
    }

    pub fn all(filter: ObjectFilter) -> Self {
        Self::All(filter)
    }

    pub fn all_creatures() -> Self {
        Self::All(ObjectFilter::creature())
    }

    pub fn all_permanents() -> Self {
        Self::All(ObjectFilter::permanent())
    }

    pub fn each_player(filter: PlayerFilter) -> Self {
        Self::EachPlayer(filter)
    }

    pub fn each_opponent() -> Self {
        Self::EachPlayer(PlayerFilter::Opponent)
    }

    pub fn iterated() -> Self {
        Self::Iterated
    }

    pub fn is_all(&self) -> bool {
        matches!(self.base(), Self::All(_) | Self::EachPlayer(_))
    }

    pub fn creature() -> Self {
        Self::Object(ObjectFilter::creature())
    }

    pub fn permanent() -> Self {
        Self::Object(ObjectFilter::permanent())
    }

    pub fn spell() -> Self {
        Self::Object(ObjectFilter::spell())
    }

    pub fn card_in_zone(zone: Zone) -> Self {
        Self::Object(ObjectFilter::default().in_zone(zone))
    }

    pub fn any_player() -> Self {
        Self::Player(PlayerFilter::Any)
    }

    pub fn object_or_player(object: ObjectFilter, player: PlayerFilter) -> Self {
        Self::ObjectOrPlayer(object, player)
    }

    pub fn opponent() -> Self {
        Self::Player(PlayerFilter::Opponent)
    }

    pub fn you() -> Self {
        Self::Player(PlayerFilter::You)
    }

    pub fn tagged(tag: impl Into<TagKey>) -> Self {
        Self::Tagged(tag.into())
    }

    pub fn target_creature() -> Self {
        Self::target(Self::creature())
    }

    pub fn target_permanent() -> Self {
        Self::target(Self::permanent())
    }

    pub fn target_player() -> Self {
        Self::target(Self::any_player())
    }

    pub fn target_opponent() -> Self {
        Self::target(Self::opponent())
    }

    pub fn target_spell() -> Self {
        Self::target(Self::spell())
    }
}

impl From<ObjectFilter> for ChooseSpec {
    fn from(value: ObjectFilter) -> Self {
        Self::Object(value)
    }
}

#[cfg(test)]
mod tests {
    use super::ChooseSpec;
    use crate::{CardType, FilterComparison, ObjectFilter, PlayerFilter, Zone};

    #[test]
    fn target_wrapper_and_count_are_stable() {
        let target_creature = ChooseSpec::target_creature().with_count(2usize.into());
        assert!(target_creature.is_target());
        assert!(!target_creature.inner().is_target());
        assert_eq!(target_creature.count(), 2usize.into());
    }

    #[test]
    fn choose_spec_builders_keep_shape() {
        assert!(matches!(ChooseSpec::creature(), ChooseSpec::Object(_)));
        assert!(matches!(
            ChooseSpec::opponent(),
            ChooseSpec::Player(PlayerFilter::Opponent)
        ));
        assert_eq!(
            ChooseSpec::card_in_zone(Zone::Graveyard),
            ChooseSpec::Object(ObjectFilter::default().in_zone(Zone::Graveyard))
        );
    }

    #[test]
    fn choose_spec_uses_core_filter_builders() {
        let filter = ObjectFilter::creature().with_power(FilterComparison::GreaterThanOrEqual(3));
        let choose = ChooseSpec::Object(filter.clone());
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(choose, ChooseSpec::Object(filter));
    }

    #[test]
    fn object_or_player_only_targets_when_wrapped() {
        let choice = ChooseSpec::object_or_player(ObjectFilter::creature(), PlayerFilter::Opponent);
        assert!(!choice.is_target());
        assert!(ChooseSpec::target(choice).is_target());
    }
}
