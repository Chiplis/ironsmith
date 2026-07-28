use crate::{Condition, PresentationLabel};

#[derive(Clone, PartialEq)]
pub struct ResolutionProgram<E> {
    pub segments: Vec<ResolutionSegment<E>>,
    flattened_default_effects: Vec<E>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResolutionSegment<E> {
    pub default_effects: Vec<E>,
    pub self_replacements: Vec<SelfReplacementBranch<E>>,
    /// This segment begins on a new authored Oracle line. Resolution semantics
    /// are unchanged; card-level rendering uses this provenance to avoid
    /// collapsing distinct spell instructions onto one line.
    pub starts_new_source_line: bool,
}

#[derive(Debug, Clone, PartialEq)]
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
}

impl<E> Default for ResolutionProgram<E> {
    fn default() -> Self {
        Self {
            segments: Vec::new(),
            flattened_default_effects: Vec::new(),
        }
    }
}

impl<E: Clone> ResolutionProgram<E> {
    pub fn new(segments: Vec<ResolutionSegment<E>>) -> Self {
        let mut program = Self {
            segments,
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
        Ok(ResolutionProgram::new(segments))
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
            .finish()
    }
}
