use crate::ChooseSpec;
use crate::cards::builders::EffectAst;
use crate::effect::EffectId;
use crate::{PlayerFilter, TagKey};

use super::shared_types::LoweringFrame;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RefState<T> {
    Known(T),
    Unknown,
    Ambiguous,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ReferenceFrame {
    pub(crate) last_effect_id: Option<EffectId>,
    pub(crate) last_library_search_effect_id: Option<EffectId>,
    pub(crate) last_object_tag: Option<String>,
    pub(crate) snapshot_tag_aliases: Vec<(String, String)>,
    pub(crate) last_it_choice_is_set: bool,
    pub(crate) last_player_filter: Option<PlayerFilter>,
    pub(crate) source_object_antecedent: bool,
    pub(crate) recent_player_choice_tags: Vec<String>,
    pub(crate) iterated_player: bool,
    pub(crate) iterated_object: bool,
    pub(crate) auto_tag_object_targets: bool,
    pub(crate) force_auto_tag_object_targets: bool,
    pub(crate) allow_life_event_value: bool,
    pub(crate) bind_unbound_x_to_last_effect: bool,
}

impl ReferenceFrame {
    pub(crate) fn from_lowering_frame(frame: &LoweringFrame) -> Self {
        Self {
            last_effect_id: frame.last_effect_id,
            last_library_search_effect_id: frame.last_library_search_effect_id,
            last_object_tag: frame.last_object_tag.clone(),
            snapshot_tag_aliases: frame.snapshot_tag_aliases.clone(),
            last_it_choice_is_set: frame.last_it_choice_is_set,
            last_player_filter: frame.last_player_filter.clone(),
            source_object_antecedent: frame.source_object_antecedent,
            recent_player_choice_tags: frame.recent_player_choice_tags.clone(),
            iterated_player: frame.iterated_player,
            iterated_object: frame.iterated_object,
            auto_tag_object_targets: frame.auto_tag_object_targets,
            force_auto_tag_object_targets: frame.force_auto_tag_object_targets,
            allow_life_event_value: frame.allow_life_event_value,
            bind_unbound_x_to_last_effect: frame.bind_unbound_x_to_last_effect,
        }
    }

    pub(crate) fn to_lowering_frame(&self) -> LoweringFrame {
        LoweringFrame {
            last_effect_id: self.last_effect_id,
            last_library_search_effect_id: self.last_library_search_effect_id,
            last_object_tag: self.last_object_tag.clone(),
            snapshot_tag_aliases: self.snapshot_tag_aliases.clone(),
            last_it_choice_is_set: self.last_it_choice_is_set,
            last_revealed_tag: None,
            last_revealed_zone: None,
            last_revealed_player_filter: None,
            last_exiled_collection_tag: None,
            last_exiled_collection_is_plural: false,
            last_player_filter: self.last_player_filter.clone(),
            source_object_antecedent: self.source_object_antecedent,
            recent_player_choice_tags: self.recent_player_choice_tags.clone(),
            iterated_player: self.iterated_player,
            iterated_object: self.iterated_object,
            auto_tag_object_targets: self.auto_tag_object_targets,
            force_auto_tag_object_targets: self.force_auto_tag_object_targets,
            allow_life_event_value: self.allow_life_event_value,
            bind_unbound_x_to_last_effect: self.bind_unbound_x_to_last_effect,
        }
    }
}

impl<T: Clone + PartialEq> RefState<T> {
    pub(crate) fn from_option(value: Option<T>) -> Self {
        match value {
            Some(value) => Self::Known(value),
            None => Self::Unknown,
        }
    }

    pub(crate) fn into_option(self) -> Option<T> {
        match self {
            Self::Known(value) => Some(value),
            Self::Unknown | Self::Ambiguous => None,
        }
    }

    pub(crate) fn join(left: &Self, right: &Self) -> Self {
        match (left, right) {
            (Self::Known(left), Self::Known(right)) if left == right => Self::Known(left.clone()),
            (Self::Unknown, Self::Unknown) => Self::Unknown,
            (Self::Ambiguous, _) | (_, Self::Ambiguous) => Self::Ambiguous,
            (Self::Known(_), Self::Known(_)) => Self::Ambiguous,
            (Self::Known(_), Self::Unknown) | (Self::Unknown, Self::Known(_)) => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct ReferenceImports {
    pub(crate) last_object_tag: Option<TagKey>,
    pub(crate) last_it_choice_is_set: bool,
    pub(crate) iterated_object: bool,
    pub(crate) last_player_filter: Option<PlayerFilter>,
    pub(crate) source_object_antecedent: bool,
    pub(crate) last_effect_id: Option<EffectId>,
    pub(crate) last_library_search_effect_id: Option<EffectId>,
}

impl ReferenceImports {
    pub(crate) fn is_empty(&self) -> bool {
        self.last_object_tag.is_none()
            && !self.last_it_choice_is_set
            && !self.iterated_object
            && self.last_player_filter.is_none()
            && !self.source_object_antecedent
            && self.last_effect_id.is_none()
            && self.last_library_search_effect_id.is_none()
    }

    pub(crate) fn with_last_object_tag(tag: impl Into<TagKey>) -> Self {
        Self {
            last_object_tag: Some(tag.into()),
            last_it_choice_is_set: false,
            iterated_object: false,
            ..Default::default()
        }
    }

    pub(crate) fn from_frame(frame: &ReferenceFrame) -> Self {
        Self {
            last_object_tag: frame.last_object_tag.as_ref().map(TagKey::from),
            last_it_choice_is_set: frame.last_it_choice_is_set,
            iterated_object: frame.iterated_object,
            last_player_filter: frame.last_player_filter.clone(),
            source_object_antecedent: frame.source_object_antecedent,
            last_effect_id: frame.last_effect_id,
            last_library_search_effect_id: frame.last_library_search_effect_id,
        }
    }

    pub(crate) fn from_lowering_frame(frame: &LoweringFrame) -> Self {
        Self::from_frame(&ReferenceFrame::from_lowering_frame(frame))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReferenceEnv {
    pub(crate) last_object_tag: RefState<TagKey>,
    /// Parse-time tag aliases bound by `SnapshotLastObjectTag`, mapping a stable
    /// parse-time placeholder tag to the concrete tag captured from
    /// `last_object_tag` at snapshot time. Survives later `last_object_tag`
    /// clobbers so composed effects can still reference an earlier looked pool.
    pub(crate) snapshot_tag_aliases: Vec<(String, String)>,
    pub(crate) last_it_choice_is_set: bool,
    pub(crate) last_player_filter: RefState<PlayerFilter>,
    pub(crate) source_object_antecedent: bool,
    pub(crate) last_effect_id: RefState<EffectId>,
    pub(crate) last_library_search_effect_id: RefState<EffectId>,
    pub(crate) iterated_player: bool,
    pub(crate) iterated_object: bool,
    pub(crate) allow_life_event_value: bool,
    pub(crate) bind_unbound_x_to_last_effect: bool,
}

impl Default for ReferenceEnv {
    fn default() -> Self {
        Self {
            last_object_tag: RefState::Unknown,
            snapshot_tag_aliases: Vec::new(),
            last_it_choice_is_set: false,
            last_player_filter: RefState::Unknown,
            source_object_antecedent: false,
            last_effect_id: RefState::Unknown,
            last_library_search_effect_id: RefState::Unknown,
            iterated_player: false,
            iterated_object: false,
            allow_life_event_value: false,
            bind_unbound_x_to_last_effect: false,
        }
    }
}

impl ReferenceEnv {
    pub(crate) fn from_imports(
        imports: &ReferenceImports,
        iterated_player: bool,
        allow_life_event_value: bool,
        bind_unbound_x_to_last_effect: bool,
        initial_last_effect_id: Option<EffectId>,
    ) -> Self {
        Self {
            last_object_tag: RefState::from_option(imports.last_object_tag.clone()),
            snapshot_tag_aliases: Vec::new(),
            last_it_choice_is_set: imports.last_it_choice_is_set,
            last_player_filter: RefState::from_option(imports.last_player_filter.clone()),
            source_object_antecedent: imports.source_object_antecedent,
            last_effect_id: RefState::from_option(
                imports.last_effect_id.or(initial_last_effect_id),
            ),
            last_library_search_effect_id: RefState::from_option(
                imports.last_library_search_effect_id,
            ),
            iterated_player,
            iterated_object: imports.iterated_object,
            allow_life_event_value,
            bind_unbound_x_to_last_effect,
        }
    }

    pub(crate) fn from_frame(frame: &ReferenceFrame) -> Self {
        Self {
            last_object_tag: RefState::from_option(
                frame.last_object_tag.as_ref().map(TagKey::from),
            ),
            snapshot_tag_aliases: frame.snapshot_tag_aliases.clone(),
            last_it_choice_is_set: frame.last_it_choice_is_set,
            last_player_filter: RefState::from_option(frame.last_player_filter.clone()),
            source_object_antecedent: frame.source_object_antecedent,
            last_effect_id: RefState::from_option(frame.last_effect_id),
            last_library_search_effect_id: RefState::from_option(
                frame.last_library_search_effect_id,
            ),
            iterated_player: frame.iterated_player,
            iterated_object: frame.iterated_object,
            allow_life_event_value: frame.allow_life_event_value,
            bind_unbound_x_to_last_effect: frame.bind_unbound_x_to_last_effect,
        }
    }

    pub(crate) fn from_lowering_frame(frame: &LoweringFrame) -> Self {
        Self::from_frame(&ReferenceFrame::from_lowering_frame(frame))
    }

    pub(crate) fn to_frame(
        &self,
        auto_tag_object_targets: bool,
        force_auto_tag_object_targets: bool,
    ) -> ReferenceFrame {
        ReferenceFrame {
            last_effect_id: self.last_effect_id.clone().into_option(),
            last_library_search_effect_id: self.last_library_search_effect_id.clone().into_option(),
            last_object_tag: self
                .last_object_tag
                .clone()
                .into_option()
                .map(|tag| tag.as_str().to_string()),
            snapshot_tag_aliases: self.snapshot_tag_aliases.clone(),
            last_it_choice_is_set: self.last_it_choice_is_set,
            last_player_filter: self.last_player_filter.clone().into_option(),
            source_object_antecedent: self.source_object_antecedent,
            recent_player_choice_tags: Vec::new(),
            iterated_player: self.iterated_player,
            iterated_object: self.iterated_object,
            auto_tag_object_targets: auto_tag_object_targets || force_auto_tag_object_targets,
            force_auto_tag_object_targets,
            allow_life_event_value: self.allow_life_event_value,
            bind_unbound_x_to_last_effect: self.bind_unbound_x_to_last_effect,
        }
    }

    pub(crate) fn to_lowering_frame(
        &self,
        auto_tag_object_targets: bool,
        force_auto_tag_object_targets: bool,
    ) -> LoweringFrame {
        self.to_frame(auto_tag_object_targets, force_auto_tag_object_targets)
            .to_lowering_frame()
    }

    pub(crate) fn known_last_object_tag(&self) -> Option<&TagKey> {
        match &self.last_object_tag {
            RefState::Known(tag) => Some(tag),
            RefState::Unknown | RefState::Ambiguous => None,
        }
    }

    pub(crate) fn known_last_player_filter(&self) -> Option<&PlayerFilter> {
        match &self.last_player_filter {
            RefState::Known(filter) => Some(filter),
            RefState::Unknown | RefState::Ambiguous => None,
        }
    }

    pub(crate) fn has_source_object_antecedent(&self) -> bool {
        self.source_object_antecedent
    }

    pub(crate) fn known_last_effect_id(&self) -> Option<EffectId> {
        match self.last_effect_id {
            RefState::Known(id) => Some(id),
            RefState::Unknown | RefState::Ambiguous => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReferenceExports {
    pub(crate) last_object_tag: RefState<TagKey>,
    pub(crate) last_it_choice_is_set: bool,
    pub(crate) last_player_filter: RefState<PlayerFilter>,
    pub(crate) source_object_antecedent: bool,
    pub(crate) last_effect_id: RefState<EffectId>,
    pub(crate) last_library_search_effect_id: RefState<EffectId>,
    pub(crate) iterated_player: bool,
}

impl Default for ReferenceExports {
    fn default() -> Self {
        Self {
            last_object_tag: RefState::Unknown,
            last_it_choice_is_set: false,
            last_player_filter: RefState::Unknown,
            source_object_antecedent: false,
            last_effect_id: RefState::Unknown,
            last_library_search_effect_id: RefState::Unknown,
            iterated_player: false,
        }
    }
}

impl ReferenceExports {
    pub(crate) fn from_env(env: &ReferenceEnv) -> Self {
        Self {
            last_object_tag: env.last_object_tag.clone(),
            last_it_choice_is_set: env.last_it_choice_is_set,
            last_player_filter: env.last_player_filter.clone(),
            source_object_antecedent: env.source_object_antecedent,
            last_effect_id: env.last_effect_id.clone(),
            last_library_search_effect_id: env.last_library_search_effect_id.clone(),
            iterated_player: env.iterated_player,
        }
    }

    pub(crate) fn join(left: &Self, right: &Self) -> Self {
        Self {
            last_object_tag: RefState::join(&left.last_object_tag, &right.last_object_tag),
            last_it_choice_is_set: left.last_it_choice_is_set && right.last_it_choice_is_set,
            last_player_filter: RefState::join(&left.last_player_filter, &right.last_player_filter),
            source_object_antecedent: left.source_object_antecedent
                && right.source_object_antecedent,
            last_effect_id: RefState::join(&left.last_effect_id, &right.last_effect_id),
            last_library_search_effect_id: RefState::join(
                &left.last_library_search_effect_id,
                &right.last_library_search_effect_id,
            ),
            iterated_player: left.iterated_player && right.iterated_player,
        }
    }

    pub(crate) fn to_imports(&self) -> ReferenceImports {
        ReferenceImports {
            last_object_tag: self.last_object_tag.clone().into_option(),
            last_it_choice_is_set: self.last_it_choice_is_set,
            iterated_object: false,
            last_player_filter: self.last_player_filter.clone().into_option(),
            source_object_antecedent: self.source_object_antecedent,
            last_effect_id: self.last_effect_id.clone().into_option(),
            last_library_search_effect_id: self.last_library_search_effect_id.clone().into_option(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LoweredEffects {
    pub(crate) effects: crate::resolution::ResolutionProgram,
    pub(crate) choices: Vec<ChooseSpec>,
    pub(crate) exports: ReferenceExports,
}

#[derive(Debug, Clone)]
pub(crate) struct AnnotatedEffect {
    pub(crate) effect: EffectAst,
    pub(crate) in_env: ReferenceEnv,
    pub(crate) out_env: ReferenceEnv,
    pub(crate) assigned_effect_id: Option<EffectId>,
    pub(crate) auto_tag_object_targets: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct AnnotatedEffectSequence {
    pub(crate) effects: Vec<AnnotatedEffect>,
    pub(crate) final_env: ReferenceEnv,
}
