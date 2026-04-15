#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Zone {
    Library,
    Hand,
    Battlefield,
    Graveyard,
    Stack,
    Exile,
    Command,
}

impl Zone {
    pub fn name(self) -> &'static str {
        match self {
            Zone::Library => "library",
            Zone::Hand => "hand",
            Zone::Battlefield => "battlefield",
            Zone::Graveyard => "graveyard",
            Zone::Stack => "stack",
            Zone::Exile => "exile",
            Zone::Command => "command",
        }
    }

    /// Returns true if objects in this zone are public (visible to all players).
    pub fn is_public(&self) -> bool {
        matches!(
            self,
            Zone::Battlefield | Zone::Graveyard | Zone::Stack | Zone::Exile | Zone::Command
        )
    }

    /// Returns true if objects in this zone are hidden (private to owner).
    pub fn is_hidden(&self) -> bool {
        matches!(self, Zone::Library | Zone::Hand)
    }

    /// Returns true if cards in this zone are ordered (order matters).
    pub fn is_ordered(&self) -> bool {
        matches!(self, Zone::Library | Zone::Stack)
    }
}

impl std::fmt::Display for Zone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zone_visibility() {
        assert!(Zone::Battlefield.is_public());
        assert!(Zone::Graveyard.is_public());
        assert!(Zone::Stack.is_public());
        assert!(Zone::Exile.is_public());
        assert!(Zone::Command.is_public());

        assert!(Zone::Library.is_hidden());
        assert!(Zone::Hand.is_hidden());
    }

    #[test]
    fn test_zone_ordering() {
        assert!(Zone::Library.is_ordered());
        assert!(Zone::Stack.is_ordered());

        assert!(!Zone::Hand.is_ordered());
        assert!(!Zone::Battlefield.is_ordered());
        assert!(!Zone::Graveyard.is_ordered());
    }
}
