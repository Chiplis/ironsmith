//! The time-counter parameter of the suspend special action (CR 702.62).
use crate::{ManaCost, tag::TagKeyWalk};

/// Fixed counts keep their legacy numeric serialized representation. An X
/// count binds the special action's announcement, independently of spell X.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TagKeyWalk)]
pub enum SuspendTime {
    Fixed(u32),
    X { minimum: u32 },
}

impl From<u32> for SuspendTime {
    fn from(time: u32) -> Self { Self::Fixed(time) }
}

impl std::fmt::Display for SuspendTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fixed(time) => write!(f, "{time}"),
            Self::X { .. } => f.write_str("X"),
        }
    }
}

impl SuspendTime {
    pub fn minimum_x(self) -> Option<u32> {
        match self { Self::Fixed(_) => None, Self::X { minimum } => Some(minimum) }
    }

    pub fn resolve(self, announced_x: Option<u32>) -> Option<u32> {
        match self {
            Self::Fixed(time) => Some(time),
            Self::X { minimum } => announced_x.filter(|x| *x >= minimum),
        }
    }

    pub fn display_keyword(self, cost: &ManaCost) -> String {
        let mut text = format!("Suspend {self}—{}", cost.to_oracle());
        match self {
            Self::X { minimum: 1 } => text.push_str(". X can't be 0."),
            Self::X { minimum } if minimum > 1 => text.push_str(&format!(". X can't be less than {minimum}.")),
            _ => {}
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suspend_x_is_an_action_bound_parameter() {
        let time = SuspendTime::X { minimum: 1 };
        assert_eq!(time.resolve(None), None);
        assert_eq!(time.resolve(Some(0)), None);
        assert_eq!(time.resolve(Some(7)), Some(7));
        assert_eq!(SuspendTime::Fixed(4).resolve(None), Some(4));
        assert_eq!(SuspendTime::Fixed(4).resolve(Some(7)), Some(4));
    }
}
