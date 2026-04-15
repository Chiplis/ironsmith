#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedRestrictions {
    pub activation: Vec<String>,
    pub trigger: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestrictionBucket {
    Activation,
    Trigger,
}

impl ParsedRestrictions {
    pub fn push(&mut self, bucket: RestrictionBucket, value: impl Into<String>) {
        match bucket {
            RestrictionBucket::Activation => self.activation.push(value.into()),
            RestrictionBucket::Trigger => self.trigger.push(value.into()),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.activation.is_empty() && self.trigger.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsed_restrictions_bucket_values() {
        let mut restrictions = ParsedRestrictions::default();
        restrictions.push(RestrictionBucket::Activation, "Activate only as a sorcery");
        restrictions.push(RestrictionBucket::Trigger, "This ability triggers only once each turn");

        assert_eq!(restrictions.activation, vec!["Activate only as a sorcery"]);
        assert_eq!(
            restrictions.trigger,
            vec!["This ability triggers only once each turn"]
        );
        assert!(!restrictions.is_empty());
    }
}
