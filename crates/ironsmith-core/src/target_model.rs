use crate::{ChoiceCount, ObjectFilter, ObjectId, PlayerFilter, PlayerId, TagKey, Zone};

/// Specifies what can be chosen or targeted by an effect.
#[derive(Debug, Clone, PartialEq)]
pub enum ChooseSpec {
    Target(Box<ChooseSpec>),
    Player(PlayerFilter),
    Object(ObjectFilter),
    SpecificObject(ObjectId),
    SpecificPlayer(PlayerId),
    AnyTarget,
    AnyOtherTarget,
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
}

impl ChooseSpec {
    pub fn target(inner: ChooseSpec) -> Self {
        if inner.is_target() {
            inner
        } else {
            Self::Target(Box::new(inner))
        }
    }

    pub fn is_target(&self) -> bool {
        match self {
            Self::Target(_)
            | Self::AnyTarget
            | Self::AnyOtherTarget
            | Self::PlayerOrPlaneswalker(_) => true,
            Self::WithCount(inner, _) => inner.is_target(),
            _ => false,
        }
    }

    pub fn inner(&self) -> &ChooseSpec {
        match self {
            Self::Target(inner) => inner.as_ref(),
            Self::WithCount(inner, _) => inner.inner(),
            _ => self,
        }
    }

    pub fn base(&self) -> &ChooseSpec {
        match self {
            Self::Target(inner) => inner.base(),
            Self::WithCount(inner, _) => inner.base(),
            _ => self,
        }
    }

    pub fn with_count(self, count: ChoiceCount) -> Self {
        match self {
            Self::WithCount(inner, _) => Self::WithCount(inner, count),
            other => Self::WithCount(Box::new(other), count),
        }
    }

    pub fn count(&self) -> ChoiceCount {
        match self {
            Self::WithCount(_, count) => *count,
            Self::Target(inner) => inner.count(),
            _ => ChoiceCount::default(),
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
        matches!(self, Self::All(_) | Self::EachPlayer(_))
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
        assert!(target_creature.inner().is_target() == false);
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
}
