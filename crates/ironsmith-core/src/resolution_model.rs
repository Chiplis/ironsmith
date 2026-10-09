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
    /// Expressions explicitly sampled when an ability is activated. Authored
    /// programs contain None; announced stack programs retain the exact sample.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Vec::is_empty"))]
    pub activation_values: Vec<(crate::Value, Option<i32>)>,
    /// Explicit provenance of this ability's linked exile producer/consumer.
    /// Absence does not authorize reading another ability's source-wide links.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub linked_exile_pair: Option<LinkedExilePair>,
    /// Immutable authored trigger occurrence. Runtime acquisitions additionally
    /// bind the exact host incarnation and AbilityOrigin; neither matcher text
    /// nor a later source definition is evidence of this identity.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub trigger_definition: Option<LinkedExileDefinition>,
    /// Composition erased an owner. Even an empty composite cannot later be
    /// mistaken for an unowned accumulator and adopt another trigger's stamp.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "is_false"))]
    unavailable_trigger_definition: bool,
    /// A separately owned linked numeric entry/reselection/read relationship.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub source_number_pair: Option<LinkedExilePair>,
    /// Immutable authored activation occurrence, including its face and costs.
    /// Kept separately from runtime acquisition and from an activation ordinal.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub activation_definition: Option<LinkedExileDefinition>,
    flattened_default_effects: Vec<E>,
    /// A legacy copied object lacked the definition evidence needed for its
    /// spell program. This is incomplete data, never an executable empty body.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "is_false"))]
    unavailable_copied_definition: bool,
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

fn is_false(value: &bool) -> bool { !*value }

impl<E> ResolutionProgram<E> {
    pub fn unavailable_copied_definition() -> Self {
        let mut program = Self::default();
        program.unavailable_copied_definition = true;
        program
    }
    pub fn has_complete_definition(&self) -> bool { !self.unavailable_copied_definition }
    pub fn retained_trigger_definition(&self) -> Option<LinkedExileDefinition> {
        (!self.unavailable_trigger_definition && self.has_complete_definition())
            .then_some(self.trigger_definition).flatten()
    }
}

impl<E> Default for ResolutionProgram<E> {
    fn default() -> Self {
        Self {
            segments: Vec::new(),
            activation_values: Vec::new(),
            linked_exile_pair: None,
            trigger_definition: None,
            unavailable_trigger_definition: false,
            source_number_pair: None,
            activation_definition: None,
            flattened_default_effects: Vec::new(),
            unavailable_copied_definition: false,
        }
    }
}

impl<E: Clone> ResolutionProgram<E> {
    pub fn new(segments: Vec<ResolutionSegment<E>>) -> Self {
        let mut program = Self {
            segments,
            activation_values: Vec::new(),            linked_exile_pair: None,
            trigger_definition: None,
            unavailable_trigger_definition: false,
            source_number_pair: None,
            activation_definition: None,
            flattened_default_effects: Vec::new(),
            unavailable_copied_definition: false,
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

    /// Native authors must explicitly identify the complete authored trigger
    /// occurrence, including its matcher, choices and condition. This is not a
    /// request to derive an identity from the host or the program's wording.
    pub fn with_trigger_definition(mut self, definition: LinkedExileDefinition) -> Self {
        self.trigger_definition = Some(definition);
        self.unavailable_trigger_definition = false;
        self
    }

    pub fn with_source_number_pair(mut self,pair:LinkedExilePair)->Self{
        self.source_number_pair=Some(pair);self
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
        self.unavailable_trigger_definition |= other.unavailable_trigger_definition;
        self.unavailable_copied_definition |= other.unavailable_copied_definition;
        if self.segments.is_empty() {
            self.linked_exile_pair = other.linked_exile_pair;
            self.source_number_pair = other.source_number_pair;
            self.activation_definition = other.activation_definition;
        } else if !other.segments.is_empty()
            && self.linked_exile_pair != other.linked_exile_pair
        {
            // Appending a distinct ability is not evidence that its exile
            // records belong to this pair. Such a composite needs explicit
            // per-owner instructions before it can read linked quantities.
            self.linked_exile_pair = None;
        }
        // An empty but stamped program can still be a complete trigger. Only
        // an unowned, complete empty accumulator adopts another definition.
        if self.segments.is_empty() && self.trigger_definition.is_none()
            && !self.unavailable_copied_definition && !self.unavailable_trigger_definition
        {
            self.trigger_definition = other.trigger_definition;
        } else if self.trigger_definition != other.trigger_definition
            && (!other.segments.is_empty() || other.trigger_definition.is_some()
                || other.unavailable_copied_definition)
        {
            self.trigger_definition = None;
            self.unavailable_trigger_definition = true;
        }
        if self.unavailable_copied_definition || self.unavailable_trigger_definition {
            self.trigger_definition = None;
            self.unavailable_trigger_definition = true;
        }
        if !self.segments.is_empty() && !other.segments.is_empty()
            && self.source_number_pair != other.source_number_pair
        {
            self.source_number_pair = None;
        }
        if !self.segments.is_empty() && !other.segments.is_empty()
            && self.activation_definition != other.activation_definition
        {
            self.activation_definition = None;
        }
        for sample in other.activation_values {
            if !self.activation_values.contains(&sample) { self.activation_values.push(sample); }
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
        mapped.activation_values = self.activation_values;
        mapped.linked_exile_pair = self.linked_exile_pair;
        mapped.trigger_definition = self.trigger_definition;
        mapped.unavailable_trigger_definition = self.unavailable_trigger_definition;
        mapped.source_number_pair = self.source_number_pair;
        mapped.activation_definition = self.activation_definition;
        mapped.unavailable_copied_definition = self.unavailable_copied_definition;
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
        let mut debug = f.debug_struct("ResolutionProgram");
        debug.field("segments", &self.segments)
            .field("linked_exile_pair", &self.linked_exile_pair)
            .field("activation_definition", &self.activation_definition);
        // Historical runtime trigger identities still use Debug for unstamped
        // definitions. Omitted identity metadata must not change their input.
        if self.source_number_pair.is_some() { debug.field("source_number_pair", &self.source_number_pair); }
        if self.trigger_definition.is_some() { debug.field("trigger_definition", &self.trigger_definition); }
        if self.unavailable_trigger_definition { debug.field("unavailable_trigger_definition", &true); }
        debug.field("unavailable_copied_definition", &self.unavailable_copied_definition).finish()
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

#[cfg(test)]
mod copied_program_completeness_tests {
    use super::*;

    #[cfg(feature = "serde")]
    #[test]
    fn incomplete_copy_and_exact_ability_metadata_survive_the_same_native_map_and_wire() {
        let definition = LinkedExileDefinition([61; 32]);
        let pair = LinkedExilePair { definition, pair: 4 };
        let mut program = ResolutionProgram::<u8>::unavailable_copied_definition()
            .with_linked_exile_pair(pair);
        program.activation_definition = Some(LinkedExileDefinition([62; 32]));
        program.replace_segments(vec![ResolutionSegment::from_effects(vec![3])]);
        let mapped = program.try_map_effects(|value| Ok::<_, ()>(u16::from(value))).unwrap();
        let decoded: ResolutionProgram<u16> =
            serde_json::from_value(serde_json::to_value(&mapped).unwrap()).unwrap();
        assert!(!decoded.has_complete_definition());
        assert_eq!(decoded.linked_exile_pair, Some(pair));
        assert_eq!(decoded.activation_definition, Some(LinkedExileDefinition([62; 32])));
        assert_eq!(decoded.flattened_default_effects(), &[3]);
    }

    #[test]
    fn missing_copy_evidence_survives_mapping_and_program_composition() {
        let unknown = ResolutionProgram::<u8>::unavailable_copied_definition();
        assert!(!unknown.has_complete_definition());
        let mut mapped = unknown.try_map_effects(|value| Ok::<_, ()>(u16::from(value))).unwrap();
        mapped.extend(ResolutionProgram::from_effects(vec![7]));
        assert!(!mapped.has_complete_definition());
        mapped.replace_segments(vec![ResolutionSegment::from_effects(vec![9])]);
        assert!(!mapped.has_complete_definition(), "replacement instructions do not supply missing definition evidence");
        let mut before = ResolutionProgram::from_effects(vec![1u8]);
        before.extend(ResolutionProgram::unavailable_copied_definition());
        assert!(!before.has_complete_definition());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn complete_legacy_program_default_and_explicit_missing_evidence_round_trip_distinctly() {
        let complete = ResolutionProgram::<u8>::default();
        let old = serde_json::to_value(&complete).unwrap();
        assert!(old.get("unavailable_copied_definition").is_none());
        assert!(serde_json::from_value::<ResolutionProgram<u8>>(old).unwrap().has_complete_definition());
        let unknown = ResolutionProgram::<u8>::unavailable_copied_definition();
        let wire = serde_json::to_value(&unknown).unwrap();
        assert_eq!(wire["unavailable_copied_definition"], true);
        let decoded: ResolutionProgram<u8> = serde_json::from_value(wire.clone()).unwrap();
        assert!(!decoded.has_complete_definition());
        assert_eq!(serde_json::to_value(decoded).unwrap(), wire);
    }
}

#[cfg(test)]
mod trigger_definition_tests {
    use super::*;
    fn stamp(value: u8) -> LinkedExileDefinition { LinkedExileDefinition([value; 32]) }

    #[test]
    fn mapping_replacement_and_compatible_append_keep_authored_occurrence() {
        let source = ResolutionProgram::from_effects(vec![1u8]).with_trigger_definition(stamp(1));
        let mut changed = source.clone().try_map_effects(|value| Ok::<_, ()>(u16::from(value))).unwrap();
        changed.replace_segments(vec![ResolutionSegment::from_effects(vec![2])]);
        changed.extend(ResolutionProgram::from_effects(vec![3]).with_trigger_definition(stamp(1)));
        assert_eq!(changed.retained_trigger_definition(), Some(stamp(1)));
        assert_eq!(source.flattened_default_effects(), &[1]);
        let mut accumulator = ResolutionProgram::default();
        accumulator.extend(source);
        assert_eq!(accumulator.retained_trigger_definition(), Some(stamp(1)));
    }

    #[test]
    fn incompatible_and_missing_owners_cannot_be_recovered_by_later_append() {
        let mut empty = ResolutionProgram::<u8>::default().with_trigger_definition(stamp(1));
        empty.extend(ResolutionProgram::default().with_trigger_definition(stamp(2)));
        assert_eq!(empty.retained_trigger_definition(), None);
        empty.extend(ResolutionProgram::from_effects(vec![1]).with_trigger_definition(stamp(3)));
        assert_eq!(empty.retained_trigger_definition(), None);
        let mut unknown = ResolutionProgram::<u8>::unavailable_copied_definition();
        unknown.extend(ResolutionProgram::from_effects(vec![1]).with_trigger_definition(stamp(1)));
        assert_eq!(unknown.retained_trigger_definition(), None);
        let mut changed = ResolutionProgram::from_effects(vec![1u8]).with_trigger_definition(stamp(1));
        changed.extend(ResolutionProgram::from_effects(vec![2]));
        assert_eq!(changed.retained_trigger_definition(), None);
        changed.replace_segments(Vec::new());
        changed.extend(ResolutionProgram::from_effects(vec![3]).with_trigger_definition(stamp(1)));
        assert_eq!(changed.retained_trigger_definition(), None);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn legacy_omission_and_explicit_identity_or_composition_hold_round_trip() {
        let legacy = ResolutionProgram::from_effects(vec![1u8]);
        let old = serde_json::to_value(&legacy).unwrap();
        assert!(old.get("trigger_definition").is_none());
        assert!(old.get("unavailable_trigger_definition").is_none());
        let restored: ResolutionProgram<u8> = serde_json::from_value(old).unwrap();
        assert!(restored.has_complete_definition());
        assert_eq!(restored.retained_trigger_definition(), None);
        let retained = legacy.with_trigger_definition(stamp(4));
        let wire = serde_json::to_value(&retained).unwrap();
        let restored: ResolutionProgram<u8> = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(restored.retained_trigger_definition(), Some(stamp(4)));
        assert_eq!(serde_json::to_value(&restored).unwrap(), wire);
        let mut incompatible = retained;
        incompatible.extend(ResolutionProgram::from_effects(vec![2]).with_trigger_definition(stamp(5)));
        let wire = serde_json::to_value(&incompatible).unwrap();
        assert_eq!(wire["unavailable_trigger_definition"], true);
        let restored: ResolutionProgram<u8> = serde_json::from_value(wire).unwrap();
        assert_eq!(restored.retained_trigger_definition(), None);
        assert!(restored.has_complete_definition(), "legacy execution is independent of identity completeness");
    }
}

#[cfg(all(test,feature="serde"))]
mod numeric_pair_codec_tests {
    use super::*;
    #[test]
    fn numeric_pair_is_copied_mapped_and_encoded_without_borrowing_exile_pair(){
        let pair=LinkedExilePair{definition:LinkedExileDefinition([92;32]),pair:3};
        let program=ResolutionProgram::from_effects(vec![4u32]).with_source_number_pair(pair);
        let mapped=program.clone().try_map_effects(|value|Ok::<_,()>(value+1)).unwrap();
        assert_eq!(mapped.source_number_pair,Some(pair));assert_eq!(mapped.linked_exile_pair,None);
        let restored:ResolutionProgram<u32>=serde_json::from_value(serde_json::to_value(&mapped).unwrap()).unwrap();
        assert_eq!(restored.source_number_pair,Some(pair));
        let mut legacy=serde_json::to_value(program).unwrap();legacy.as_object_mut().unwrap().remove("source_number_pair");
        let restored:ResolutionProgram<u32>=serde_json::from_value(legacy).unwrap();assert_eq!(restored.source_number_pair,None);
    }
}


#[cfg(all(test, feature = "serde"))]
mod combined_owner_integration_tests {
    use super::*;

    #[test]
    fn mapping_and_native_codec_preserve_numeric_exile_trigger_and_activation_owners() {
        let numeric = LinkedExilePair { definition: LinkedExileDefinition([31; 32]), pair: 2 };
        let exile = LinkedExilePair { definition: LinkedExileDefinition([32; 32]), pair: 4 };
        let trigger = LinkedExileDefinition([33; 32]);
        let activation = LinkedExileDefinition([34; 32]);
        let mut program = ResolutionProgram::from_effects(vec![1u8])
            .with_source_number_pair(numeric).with_linked_exile_pair(exile).with_trigger_definition(trigger);
        program.activation_definition = Some(activation);
        let mut mapped = program.try_map_effects(|value| Ok::<_, ()>(u16::from(value))).unwrap();
        mapped.replace_segments(vec![ResolutionSegment::from_effects(vec![2])]);
        let decoded: ResolutionProgram<u16> = serde_json::from_value(serde_json::to_value(mapped).unwrap()).unwrap();
        assert_eq!(decoded.source_number_pair, Some(numeric));
        assert_eq!(decoded.linked_exile_pair, Some(exile));
        assert_eq!(decoded.retained_trigger_definition(), Some(trigger));
        assert_eq!(decoded.activation_definition, Some(activation));
    }

    #[test]
    fn absent_numeric_metadata_preserves_historical_debug_and_serialized_inputs() {
        let program = ResolutionProgram::from_effects(vec![7u8]);
        let debug = format!("{program:?}");
        assert!(!debug.contains("source_number_pair"));
        assert!(!debug.contains("trigger_definition"));
        let wire = serde_json::to_value(&program).unwrap();
        assert!(wire.get("source_number_pair").is_none());
        assert!(wire.get("trigger_definition").is_none());
        let decoded: ResolutionProgram<u8> = serde_json::from_value(wire).unwrap();
        assert_eq!(format!("{decoded:?}"), debug);
    }
}
