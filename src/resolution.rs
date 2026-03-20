use crate::effect::{Condition, Effect};

#[derive(Clone, Default, PartialEq)]
pub struct ResolutionProgram {
    pub segments: Vec<ResolutionSegment>,
    flattened_default_effects: Vec<Effect>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResolutionSegment {
    pub default_effects: Vec<Effect>,
    pub self_replacements: Vec<SelfReplacementBranch>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelfReplacementBranch {
    pub condition: Condition,
    pub replacement_effects: Vec<Effect>,
}

impl ResolutionProgram {
    pub fn new(segments: Vec<ResolutionSegment>) -> Self {
        let mut program = Self {
            segments,
            flattened_default_effects: Vec::new(),
        };
        program.refresh_flattened_defaults();
        program
    }

    pub fn from_effects(effects: Vec<Effect>) -> Self {
        if effects.is_empty() {
            Self::default()
        } else {
            Self::new(vec![ResolutionSegment::from_effects(effects)])
        }
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty() || self.flattened_default_effects.is_empty()
    }

    pub fn push_segment(&mut self, segment: ResolutionSegment) {
        self.flattened_default_effects
            .extend(segment.default_effects.iter().cloned());
        self.segments.push(segment);
    }

    pub fn push(&mut self, effect: Effect) {
        self.flattened_default_effects.push(effect.clone());
        if let Some(segment) = self.segments.last_mut() {
            segment.default_effects.push(effect);
        } else {
            self.segments
                .push(ResolutionSegment::from_effects(vec![effect]));
        }
    }

    pub fn pop(&mut self) -> Option<Effect> {
        let effect = self.segments.last_mut()?.default_effects.pop()?;
        self.flattened_default_effects.pop();
        if self.segments.last().is_some_and(|segment| {
            segment.default_effects.is_empty() && segment.self_replacements.is_empty()
        }) {
            self.segments.pop();
        }
        Some(effect)
    }

    pub fn insert(&mut self, index: usize, effect: Effect) {
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

    pub fn last_segment_mut(&mut self) -> Option<&mut ResolutionSegment> {
        self.segments.last_mut()
    }

    pub fn all_effects(&self) -> Vec<&Effect> {
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

    pub fn all_effects_owned(&self) -> Vec<Effect> {
        self.all_effects().into_iter().cloned().collect()
    }

    pub fn flattened_default_effects(&self) -> &[Effect] {
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

impl From<Vec<Effect>> for ResolutionProgram {
    fn from(value: Vec<Effect>) -> Self {
        Self::from_effects(value)
    }
}

impl ResolutionSegment {
    pub fn from_effects(effects: Vec<Effect>) -> Self {
        Self {
            default_effects: effects,
            self_replacements: Vec::new(),
        }
    }
}

impl SelfReplacementBranch {
    pub fn new(condition: Condition, replacement_effects: Vec<Effect>) -> Self {
        Self {
            condition,
            replacement_effects,
        }
    }
}

impl std::ops::Deref for ResolutionProgram {
    type Target = [Effect];

    fn deref(&self) -> &Self::Target {
        self.flattened_default_effects()
    }
}

impl<'a> IntoIterator for &'a ResolutionProgram {
    type Item = &'a Effect;
    type IntoIter = std::slice::Iter<'a, Effect>;

    fn into_iter(self) -> Self::IntoIter {
        self.flattened_default_effects().iter()
    }
}

impl IntoIterator for ResolutionProgram {
    type Item = Effect;
    type IntoIter = std::vec::IntoIter<Effect>;

    fn into_iter(self) -> Self::IntoIter {
        self.flattened_default_effects.into_iter()
    }
}

impl std::fmt::Debug for ResolutionProgram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolutionProgram")
            .field("segments", &self.segments)
            .finish()
    }
}
