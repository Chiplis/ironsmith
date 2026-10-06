use crate::tag::TagKeyWalk;

use crate::{Condition, PresentationLabel};

/// Immutable identity of the executable rules definition. This is deliberately
/// independent of caller-local CardId values and physical card identities.
/// Compilers use a SHA-256 of the typed executable definition; native authors
/// explicitly provide a stable identity for their paired rules definition.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TagKeyWalk)]
pub struct LinkedExileDefinition(pub [u8; 32]);

/// A compiler/native-authored pair of linked executable abilities. This
/// definition identity is copied with the program, rather than read from the
/// current host card. Runtime acquisitions supply a separate occurrence key.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TagKeyWalk)]
pub struct LinkedExilePair {
    pub definition: LinkedExileDefinition,
    pub pair: u32,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, PartialEq, TagKeyWalk)]
pub struct ResolutionProgram<E> {
    pub segments: Vec<ResolutionSegment<E>>,
    /// Explicit provenance of this ability's linked exile producer/consumer.
    /// Absence does not authorize reading another ability's source-wide links.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub linked_exile_pair: Option<LinkedExilePair>,
    /// Immutable authored activation occurrence, including its face and costs.
    /// Kept separately from runtime acquisition and from an activation ordinal.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub activation_definition: Option<LinkedExileDefinition>,
    flattened_default_effects: Vec<E>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, PartialEq, TagKeyWalk)]
pub struct ResolutionSegment<E> {
    pub default_effects: Vec<E>,
    pub self_replacements: Vec<SelfReplacementBranch<E>>,
    /// This segment begins on a new authored Oracle line. Resolution semantics
    /// are unchanged; card-level rendering uses this provenance to avoid
    /// collapsing distinct spell instructions onto one line.
    pub starts_new_source_line: bool,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub struct SelfReplacementBranch<E> {
    pub condition: Condition,
    pub replacement_effects: Vec<E>,
    pub presentation_label: Option<PresentationLabel>,
    pub condition_after_replacement: bool,
    /// Preserve an authored leading replacement connective:
    /// "If ..., instead [actions]" rather than "[actions] instead".
    ///
    /// This is presentation provenance only; replacement semantics are
    /// already carried by the branch itself.
    pub leading_instead_surface: bool,
    /// This replacement was authored on a new Oracle source line even though
    /// it semantically replaces the effects in the preceding segment.
    /// Presentation only; resolution behavior is unchanged.
    pub starts_new_source_line: bool,
}

impl<E> Default for ResolutionProgram<E> {
    fn default() -> Self {
        Self {
            segments: Vec::new(),
            linked_exile_pair: None,
            activation_definition: None,
            flattened_default_effects: Vec::new(),
        }
    }
}

impl<E: Clone> ResolutionProgram<E> {
    pub fn new(segments: Vec<ResolutionSegment<E>>) -> Self {
        let mut program = Self {
            segments,
            linked_exile_pair: None,
            activation_definition: None,
            flattened_default_effects: Vec::new(),
        };
        program.refresh_flattened_defaults();
        program
    }

    pub fn from_effects(effects: Vec<E>) -> Self {
        if effects.is_empty() {
            Self::default()
        } else {
            Self::new(vec![ResolutionSegment::from_effects(effects)])
        }
    }

    pub fn with_linked_exile_pair(mut self, pair: LinkedExilePair) -> Self {
        self.linked_exile_pair = Some(pair);
        self
    }

    /// Replace instructions while retaining the declared ability owner.
    pub fn replace_segments(&mut self, segments: Vec<ResolutionSegment<E>>) {
        self.segments = segments;
        self.refresh_flattened_defaults();
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty() || self.flattened_default_effects.is_empty()
    }

    pub fn push_segment(&mut self, segment: ResolutionSegment<E>) {
        self.flattened_default_effects
            .extend(segment.default_effects.iter().cloned());
        self.segments.push(segment);
    }

    pub fn push(&mut self, effect: E) {
        self.flattened_default_effects.push(effect.clone());
        if let Some(segment) = self.segments.last_mut() {
            segment.default_effects.push(effect);
        } else {
            self.segments
                .push(ResolutionSegment::from_effects(vec![effect]));
        }
    }

    pub fn pop(&mut self) -> Option<E> {
        let effect = self.segments.last_mut()?.default_effects.pop()?;
        self.flattened_default_effects.pop();
        if self.segments.last().is_some_and(|segment| {
            segment.default_effects.is_empty() && segment.self_replacements.is_empty()
        }) {
            self.segments.pop();
        }
        Some(effect)
    }

    pub fn insert(&mut self, index: usize, effect: E) {
        self.flattened_default_effects.insert(index, effect.clone());
        if self.segments.is_empty() {
            self.segments
                .push(ResolutionSegment::from_effects(vec![effect]));
            return;
        }

        let mut offset = 0usize;
        for segment in &mut self.segments {
            let next = offset + segment.default_effects.len();
            if index <= next {
                segment.default_effects.insert(index - offset, effect);
                return;
            }
            offset = next;
        }

        self.segments
            .last_mut()
            .expect("checked non-empty above")
            .default_effects
            .push(effect);
    }

    pub fn extend(&mut self, other: Self) {
        if self.segments.is_empty() {
            self.linked_exile_pair = other.linked_exile_pair;
            self.activation_definition = other.activation_definition;
        } else if !other.segments.is_empty()
            && self.linked_exile_pair != other.linked_exile_pair
        {
            // Appending a distinct ability is not evidence that its exile
            // records belong to this pair. Such a composite needs explicit
            // per-owner instructions before it can read linked quantities.
            self.linked_exile_pair = None;
        }
        if !self.segments.is_empty() && !other.segments.is_empty()
            && self.activation_definition != other.activation_definition
        {
            self.activation_definition = None;
        }
        for segment in other.segments {
            self.push_segment(segment);
        }
    }

    pub fn last_segment_mut(&mut self) -> Option<&mut ResolutionSegment<E>> {
        self.segments.last_mut()
    }

    pub fn all_effects(&self) -> Vec<&E> {
        let mut effects = Vec::new();
        for segment in &self.segments {
            for effect in &segment.default_effects {
                effects.push(effect);
            }
            for branch in &segment.self_replacements {
                for effect in &branch.replacement_effects {
                    effects.push(effect);
                }
            }
        }
        effects
    }

    pub fn all_effects_owned(&self) -> Vec<E> {
        self.all_effects().into_iter().cloned().collect()
    }

    pub fn flattened_default_effects(&self) -> &[E] {
        &self.flattened_default_effects
    }

    fn refresh_flattened_defaults(&mut self) {
        self.flattened_default_effects.clear();
        for segment in &self.segments {
            self.flattened_default_effects
                .extend(segment.default_effects.iter().cloned());
        }
    }
}

impl<E> ResolutionProgram<E> {
    pub fn try_map_effects<U: Clone, Err>(
        self,
        mut f: impl FnMut(E) -> Result<U, Err>,
    ) -> Result<ResolutionProgram<U>, Err> {
        let mut segments = Vec::with_capacity(self.segments.len());
        for segment in self.segments {
            segments.push(segment.try_map_effects(&mut f)?);
        }
        let mut mapped = ResolutionProgram::new(segments);
        mapped.linked_exile_pair = self.linked_exile_pair;
        mapped.activation_definition = self.activation_definition;
        Ok(mapped)
    }
}

impl<E> ResolutionSegment<E> {
    pub fn try_map_effects<U, Err>(
        self,
        f: &mut impl FnMut(E) -> Result<U, Err>,
    ) -> Result<ResolutionSegment<U>, Err> {
        let mut default_effects = Vec::with_capacity(self.default_effects.len());
        for effect in self.default_effects {
            default_effects.push(f(effect)?);
        }

        let mut self_replacements = Vec::with_capacity(self.self_replacements.len());
        for branch in self.self_replacements {
            self_replacements.push(branch.try_map_effects(f)?);
        }

        Ok(ResolutionSegment {
            default_effects,
            self_replacements,
            starts_new_source_line: self.starts_new_source_line,
        })
    }
}

impl<E> SelfReplacementBranch<E> {
    pub fn try_map_effects<U, Err>(
        self,
        f: &mut impl FnMut(E) -> Result<U, Err>,
    ) -> Result<SelfReplacementBranch<U>, Err> {
        let mut replacement_effects = Vec::with_capacity(self.replacement_effects.len());
        for effect in self.replacement_effects {
            replacement_effects.push(f(effect)?);
        }

        Ok(SelfReplacementBranch {
            condition: self.condition,
            replacement_effects,
            presentation_label: self.presentation_label,
            condition_after_replacement: self.condition_after_replacement,
            leading_instead_surface: self.leading_instead_surface,
            starts_new_source_line: self.starts_new_source_line,
        })
    }
}

impl<E: Clone> From<Vec<E>> for ResolutionProgram<E> {
    fn from(value: Vec<E>) -> Self {
        Self::from_effects(value)
    }
}

impl<E> ResolutionSegment<E> {
    pub fn from_effects(effects: Vec<E>) -> Self {
        Self {
            default_effects: effects,
            self_replacements: Vec::new(),
            starts_new_source_line: false,
        }
    }
}

impl<E> SelfReplacementBranch<E> {
    pub fn new(condition: Condition, replacement_effects: Vec<E>) -> Self {
        Self {
            condition,
            replacement_effects,
            presentation_label: None,
            condition_after_replacement: false,
            leading_instead_surface: false,
            starts_new_source_line: false,
        }
    }

    pub fn with_presentation_label(
        mut self,
        presentation_label: Option<PresentationLabel>,
    ) -> Self {
        self.presentation_label = presentation_label;
        self
    }

    pub fn with_leading_instead_surface(mut self, leading_instead_surface: bool) -> Self {
        self.leading_instead_surface = leading_instead_surface;
        self
    }

    pub fn with_starts_new_source_line(mut self, starts_new_source_line: bool) -> Self {
        self.starts_new_source_line = starts_new_source_line;
        self
    }
}

impl<E> std::ops::Deref for ResolutionProgram<E> {
    type Target = [E];

    fn deref(&self) -> &Self::Target {
        self.flattened_default_effects.as_slice()
    }
}

impl<'a, E> IntoIterator for &'a ResolutionProgram<E> {
    type Item = &'a E;
    type IntoIter = std::slice::Iter<'a, E>;

    fn into_iter(self) -> Self::IntoIter {
        self.flattened_default_effects.iter()
    }
}

impl<E> IntoIterator for ResolutionProgram<E> {
    type Item = E;
    type IntoIter = std::vec::IntoIter<E>;

    fn into_iter(self) -> Self::IntoIter {
        self.flattened_default_effects.into_iter()
    }
}

impl<E: std::fmt::Debug> std::fmt::Debug for ResolutionProgram<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolutionProgram")
            .field("segments", &self.segments)
            .field("linked_exile_pair", &self.linked_exile_pair)
            .field("activation_definition", &self.activation_definition)
            .finish()
    }
}


#[cfg(test)]
mod linked_exile_pair_tests {
    use super::*;

    fn pair(slot: u32) -> LinkedExilePair {
        LinkedExilePair { definition: LinkedExileDefinition([120; 32]), pair: slot }
    }

    #[test]
    fn mapping_and_instruction_replacement_preserve_pair_identity() {
        let program = ResolutionProgram::from_effects(vec![1u32]).with_linked_exile_pair(pair(0));
        let mut mapped = program.try_map_effects(|value| Ok::<_, ()>(u64::from(value))).unwrap();
        mapped.replace_segments(vec![ResolutionSegment::from_effects(vec![2u64])]);
        assert_eq!(mapped.linked_exile_pair, Some(pair(0)));
        assert_eq!(mapped.flattened_default_effects(), &[2]);
    }

    #[test]
    fn extending_different_owners_does_not_coalesce_pairs() {
        let mut first = ResolutionProgram::from_effects(vec![1u32]).with_linked_exile_pair(pair(0));
        first.extend(ResolutionProgram::from_effects(vec![2]).with_linked_exile_pair(pair(1)));
        assert_eq!(first.linked_exile_pair, None);
        let mut empty = ResolutionProgram::default();
        empty.extend(ResolutionProgram::from_effects(vec![3u32]).with_linked_exile_pair(pair(1)));
        assert_eq!(empty.linked_exile_pair, Some(pair(1)));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn unknown_legacy_program_and_explicit_pair_round_trip() {
        let legacy = ResolutionProgram::from_effects(vec![1u32]);
        let json = serde_json::to_value(&legacy).unwrap();
        assert!(json.get("linked_exile_pair").is_none());
        let restored: ResolutionProgram<u32> = serde_json::from_value(json).unwrap();
        assert_eq!(restored.linked_exile_pair, None);
        let paired = legacy.with_linked_exile_pair(pair(2));
        let restored: ResolutionProgram<u32> = serde_json::from_str(&serde_json::to_string(&paired).unwrap()).unwrap();
        assert_eq!(restored.linked_exile_pair, Some(pair(2)));
    }
}

#[cfg(all(test, feature = "serde"))]
mod activation_definition_tests {
    use super::*;
    #[test]
    fn absent_legacy_identity_stays_absent_and_mapping_preserves_a_retained_identity() {
        let legacy = ResolutionProgram::from_effects(vec![1_u32]);
        let json = serde_json::to_value(&legacy).unwrap();
        assert!(json.get("activation_definition").is_none());
        let restored: ResolutionProgram<u32> = serde_json::from_value(json).unwrap();
        assert_eq!(restored.activation_definition, None);
        let mut retained = restored;
        retained.activation_definition = Some(LinkedExileDefinition([37; 32]));
        let mapped = retained.clone().try_map_effects(|value| Ok::<_, ()>(u64::from(value))).unwrap();
        assert_eq!(mapped.activation_definition, retained.activation_definition);
        let restored: ResolutionProgram<u64> = serde_json::from_value(serde_json::to_value(mapped).unwrap()).unwrap();
        assert_eq!(restored.activation_definition, retained.activation_definition);
        retained.replace_segments(vec![ResolutionSegment::from_effects(vec![2])]);
        assert_eq!(retained.activation_definition, Some(LinkedExileDefinition([37; 32])));
        let mut other = ResolutionProgram::from_effects(vec![3]);
        other.activation_definition = Some(LinkedExileDefinition([38; 32]));
        retained.extend(other);
        assert_eq!(retained.activation_definition, None);
    }
}
