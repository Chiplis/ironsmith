use ironsmith_core::{EffectId, TagKey};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefState<T> {
    Known(T),
    Unknown,
    Ambiguous,
}

impl<T: Clone + PartialEq> RefState<T> {
    pub fn from_option(value: Option<T>) -> Self {
        match value {
            Some(value) => Self::Known(value),
            None => Self::Unknown,
        }
    }

    pub fn into_option(self) -> Option<T> {
        match self {
            Self::Known(value) => Some(value),
            Self::Unknown | Self::Ambiguous => None,
        }
    }

    pub fn join(left: &Self, right: &Self) -> Self {
        match (left, right) {
            (Self::Known(left), Self::Known(right)) if left == right => Self::Known(left.clone()),
            (Self::Unknown, Self::Unknown) => Self::Unknown,
            (Self::Ambiguous, _) | (_, Self::Ambiguous) => Self::Ambiguous,
            (Self::Known(_), Self::Known(_)) => Self::Ambiguous,
            (Self::Known(_), Self::Unknown) | (Self::Unknown, Self::Known(_)) => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReferenceFrame<PlayerFilter> {
    pub last_effect_id: Option<EffectId>,
    pub last_object_tag: Option<TagKey>,
    pub last_player_filter: Option<PlayerFilter>,
    pub source_object_antecedent: bool,
    pub recent_player_choice_tags: Vec<TagKey>,
    pub iterated_player: bool,
    pub auto_tag_object_targets: bool,
    pub force_auto_tag_object_targets: bool,
    pub allow_life_event_value: bool,
    pub bind_unbound_x_to_last_effect: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceImports<PlayerFilter> {
    pub last_object_tag: Option<TagKey>,
    pub last_player_filter: Option<PlayerFilter>,
    pub source_object_antecedent: bool,
    pub last_effect_id: Option<EffectId>,
}

impl<PlayerFilter> Default for ReferenceImports<PlayerFilter> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<PlayerFilter> ReferenceImports<PlayerFilter> {
    pub fn empty() -> Self {
        Self {
            last_object_tag: None,
            last_player_filter: None,
            source_object_antecedent: false,
            last_effect_id: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.last_object_tag.is_none()
            && self.last_player_filter.is_none()
            && !self.source_object_antecedent
            && self.last_effect_id.is_none()
    }

    pub fn with_last_object_tag(tag: impl Into<TagKey>) -> Self {
        Self {
            last_object_tag: Some(tag.into()),
            ..Self::empty()
        }
    }
}

impl<PlayerFilter: Clone> ReferenceImports<PlayerFilter> {
    pub fn from_frame(frame: &ReferenceFrame<PlayerFilter>) -> Self {
        Self {
            last_object_tag: frame.last_object_tag.clone(),
            last_player_filter: frame.last_player_filter.clone(),
            source_object_antecedent: frame.source_object_antecedent,
            last_effect_id: frame.last_effect_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceEnv<PlayerFilter> {
    pub last_object_tag: RefState<TagKey>,
    pub last_player_filter: RefState<PlayerFilter>,
    pub source_object_antecedent: bool,
    pub last_effect_id: RefState<EffectId>,
    pub iterated_player: bool,
    pub allow_life_event_value: bool,
    pub bind_unbound_x_to_last_effect: bool,
}

impl<PlayerFilter> Default for ReferenceEnv<PlayerFilter> {
    fn default() -> Self {
        Self {
            last_object_tag: RefState::Unknown,
            last_player_filter: RefState::Unknown,
            source_object_antecedent: false,
            last_effect_id: RefState::Unknown,
            iterated_player: false,
            allow_life_event_value: false,
            bind_unbound_x_to_last_effect: false,
        }
    }
}

impl<PlayerFilter: Clone + PartialEq> ReferenceEnv<PlayerFilter> {
    pub fn from_imports(
        imports: &ReferenceImports<PlayerFilter>,
        iterated_player: bool,
        allow_life_event_value: bool,
        bind_unbound_x_to_last_effect: bool,
        initial_last_effect_id: Option<EffectId>,
    ) -> Self {
        Self {
            last_object_tag: RefState::from_option(imports.last_object_tag.clone()),
            last_player_filter: RefState::from_option(imports.last_player_filter.clone()),
            source_object_antecedent: imports.source_object_antecedent,
            last_effect_id: RefState::from_option(
                imports.last_effect_id.or(initial_last_effect_id),
            ),
            iterated_player,
            allow_life_event_value,
            bind_unbound_x_to_last_effect,
        }
    }

    pub fn from_frame(frame: &ReferenceFrame<PlayerFilter>) -> Self {
        Self {
            last_object_tag: RefState::from_option(frame.last_object_tag.clone()),
            last_player_filter: RefState::from_option(frame.last_player_filter.clone()),
            source_object_antecedent: frame.source_object_antecedent,
            last_effect_id: RefState::from_option(frame.last_effect_id),
            iterated_player: frame.iterated_player,
            allow_life_event_value: frame.allow_life_event_value,
            bind_unbound_x_to_last_effect: frame.bind_unbound_x_to_last_effect,
        }
    }

    pub fn to_frame(
        &self,
        auto_tag_object_targets: bool,
        force_auto_tag_object_targets: bool,
    ) -> ReferenceFrame<PlayerFilter> {
        ReferenceFrame {
            last_effect_id: self.last_effect_id.clone().into_option(),
            last_object_tag: self.last_object_tag.clone().into_option(),
            last_player_filter: self.last_player_filter.clone().into_option(),
            source_object_antecedent: self.source_object_antecedent,
            recent_player_choice_tags: Vec::new(),
            iterated_player: self.iterated_player,
            auto_tag_object_targets: auto_tag_object_targets || force_auto_tag_object_targets,
            force_auto_tag_object_targets,
            allow_life_event_value: self.allow_life_event_value,
            bind_unbound_x_to_last_effect: self.bind_unbound_x_to_last_effect,
        }
    }

    pub fn known_last_object_tag(&self) -> Option<&TagKey> {
        match &self.last_object_tag {
            RefState::Known(tag) => Some(tag),
            RefState::Unknown | RefState::Ambiguous => None,
        }
    }

    pub fn known_last_player_filter(&self) -> Option<&PlayerFilter> {
        match &self.last_player_filter {
            RefState::Known(filter) => Some(filter),
            RefState::Unknown | RefState::Ambiguous => None,
        }
    }

    pub fn known_last_effect_id(&self) -> Option<EffectId> {
        match self.last_effect_id {
            RefState::Known(id) => Some(id),
            RefState::Unknown | RefState::Ambiguous => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceExports<PlayerFilter> {
    pub last_object_tag: RefState<TagKey>,
    pub last_player_filter: RefState<PlayerFilter>,
    pub source_object_antecedent: bool,
    pub last_effect_id: RefState<EffectId>,
    pub iterated_player: bool,
}

impl<PlayerFilter> Default for ReferenceExports<PlayerFilter> {
    fn default() -> Self {
        Self {
            last_object_tag: RefState::Unknown,
            last_player_filter: RefState::Unknown,
            source_object_antecedent: false,
            last_effect_id: RefState::Unknown,
            iterated_player: false,
        }
    }
}

impl<PlayerFilter: Clone + PartialEq> ReferenceExports<PlayerFilter> {
    pub fn from_env(env: &ReferenceEnv<PlayerFilter>) -> Self {
        Self {
            last_object_tag: env.last_object_tag.clone(),
            last_player_filter: env.last_player_filter.clone(),
            source_object_antecedent: env.source_object_antecedent,
            last_effect_id: env.last_effect_id.clone(),
            iterated_player: env.iterated_player,
        }
    }

    pub fn join(left: &Self, right: &Self) -> Self {
        Self {
            last_object_tag: RefState::join(&left.last_object_tag, &right.last_object_tag),
            last_player_filter: RefState::join(&left.last_player_filter, &right.last_player_filter),
            source_object_antecedent: left.source_object_antecedent
                && right.source_object_antecedent,
            last_effect_id: RefState::join(&left.last_effect_id, &right.last_effect_id),
            iterated_player: left.iterated_player && right.iterated_player,
        }
    }

    pub fn to_imports(&self) -> ReferenceImports<PlayerFilter> {
        ReferenceImports {
            last_object_tag: self.last_object_tag.clone().into_option(),
            last_player_filter: self.last_player_filter.clone().into_option(),
            source_object_antecedent: self.source_object_antecedent,
            last_effect_id: self.last_effect_id.clone().into_option(),
        }
    }

    pub fn known_last_effect_id(&self) -> Option<EffectId> {
        match self.last_effect_id {
            RefState::Known(id) => Some(id),
            RefState::Unknown | RefState::Ambiguous => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoweredEffects<Program, Choice, PlayerFilter> {
    pub effects: Program,
    pub choices: Vec<Choice>,
    pub exports: ReferenceExports<PlayerFilter>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotatedEffect<Effect, PlayerFilter> {
    pub effect: Effect,
    pub in_env: ReferenceEnv<PlayerFilter>,
    pub out_env: ReferenceEnv<PlayerFilter>,
    pub assigned_effect_id: Option<EffectId>,
    pub auto_tag_object_targets: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotatedEffectSequence<Effect, PlayerFilter> {
    pub effects: Vec<AnnotatedEffect<Effect, PlayerFilter>>,
    pub final_env: ReferenceEnv<PlayerFilter>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_exports_join_known_and_unknown_state() {
        let left = ReferenceExports {
            last_object_tag: RefState::Known(TagKey::from("it")),
            last_player_filter: RefState::Known("you".to_string()),
            source_object_antecedent: true,
            last_effect_id: RefState::Known(EffectId(7)),
            iterated_player: true,
        };
        let right = ReferenceExports {
            last_object_tag: RefState::Known(TagKey::from("it")),
            last_player_filter: RefState::Unknown,
            source_object_antecedent: true,
            last_effect_id: RefState::Known(EffectId(7)),
            iterated_player: false,
        };

        let joined = ReferenceExports::join(&left, &right);

        assert!(matches!(joined.last_object_tag, RefState::Known(_)));
        assert!(matches!(joined.last_player_filter, RefState::Unknown));
        assert!(joined.source_object_antecedent);
        assert_eq!(joined.known_last_effect_id(), Some(EffectId(7)));
        assert!(!joined.iterated_player);
    }

    #[test]
    fn reference_env_round_trips_frame_state() {
        let frame = ReferenceFrame {
            last_effect_id: Some(EffectId(9)),
            last_object_tag: Some(TagKey::from("that")),
            last_player_filter: Some("opponent".to_string()),
            source_object_antecedent: true,
            recent_player_choice_tags: vec![crate::tag::CompilerReferenceTag::Chosen.key()],
            iterated_player: true,
            auto_tag_object_targets: false,
            force_auto_tag_object_targets: true,
            allow_life_event_value: true,
            bind_unbound_x_to_last_effect: true,
        };

        let env = ReferenceEnv::from_frame(&frame);
        let rebuilt = env.to_frame(false, true);

        assert_eq!(env.known_last_effect_id(), Some(EffectId(9)));
        assert_eq!(rebuilt.last_object_tag, Some(TagKey::from("that")));
        assert_eq!(rebuilt.last_player_filter.as_deref(), Some("opponent"));
        assert!(rebuilt.source_object_antecedent);
        assert!(rebuilt.force_auto_tag_object_targets);
    }
}
