#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    White,
    Blue,
    Black,
    Red,
    Green,
}

impl Color {
    pub const ALL: [Color; 5] = [
        Color::White,
        Color::Blue,
        Color::Black,
        Color::Red,
        Color::Green,
    ];
}

/// A set of colors represented as bitflags for efficient operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ColorSet(u8);

impl ColorSet {
    pub const COLORLESS: Self = Self(0);
    pub const WHITE: Self = Self(1 << 0);
    pub const BLUE: Self = Self(1 << 1);
    pub const BLACK: Self = Self(1 << 2);
    pub const RED: Self = Self(1 << 3);
    pub const GREEN: Self = Self(1 << 4);

    /// Creates a new empty ColorSet.
    pub const fn new() -> Self {
        Self(0)
    }

    /// Creates a ColorSet from a single color.
    pub const fn from_color(color: Color) -> Self {
        match color {
            Color::White => Self::WHITE,
            Color::Blue => Self::BLUE,
            Color::Black => Self::BLACK,
            Color::Red => Self::RED,
            Color::Green => Self::GREEN,
        }
    }

    /// Returns true if this set contains no colors.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns true if this set contains the given color.
    pub const fn contains(self, color: Color) -> bool {
        self.0 & Self::from_color(color).0 != 0
    }

    /// Returns true if this set contains all colors in the other set.
    pub const fn contains_all(self, other: ColorSet) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns the union of two color sets.
    pub const fn union(self, other: ColorSet) -> Self {
        Self(self.0 | other.0)
    }

    /// Returns the intersection of two color sets.
    pub const fn intersection(self, other: ColorSet) -> Self {
        Self(self.0 & other.0)
    }

    /// Returns the number of colors in this set.
    pub const fn count(self) -> u32 {
        self.0.count_ones()
    }

    /// Adds a color to this set, returning the new set.
    pub const fn with(self, color: Color) -> Self {
        self.union(Self::from_color(color))
    }

    /// Removes a color from this set, returning the new set.
    pub const fn without(self, color: Color) -> Self {
        Self(self.0 & !Self::from_color(color).0)
    }
}

impl From<Color> for ColorSet {
    fn from(color: Color) -> Self {
        Self::from_color(color)
    }
}

impl FromIterator<Color> for ColorSet {
    fn from_iter<T: IntoIterator<Item = Color>>(iter: T) -> Self {
        iter.into_iter()
            .fold(ColorSet::COLORLESS, |set, color| set.with(color))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_set_empty() {
        let set = ColorSet::new();
        assert!(set.is_empty());
        assert_eq!(set.count(), 0);
    }

    #[test]
    fn test_color_set_single_color() {
        let set = ColorSet::WHITE;
        assert!(!set.is_empty());
        assert!(set.contains(Color::White));
        assert!(!set.contains(Color::Blue));
        assert_eq!(set.count(), 1);
    }

    #[test]
    fn test_color_set_union() {
        let azorius = ColorSet::WHITE.union(ColorSet::BLUE);
        assert!(azorius.contains(Color::White));
        assert!(azorius.contains(Color::Blue));
        assert!(!azorius.contains(Color::Black));
        assert_eq!(azorius.count(), 2);
    }

    #[test]
    fn test_color_set_intersection() {
        let azorius = ColorSet::WHITE.union(ColorSet::BLUE);
        let boros = ColorSet::WHITE.union(ColorSet::RED);
        let intersection = azorius.intersection(boros);
        assert!(intersection.contains(Color::White));
        assert!(!intersection.contains(Color::Blue));
        assert!(!intersection.contains(Color::Red));
        assert_eq!(intersection.count(), 1);
    }

    #[test]
    fn test_color_set_contains_all() {
        let jeskai = ColorSet::WHITE.union(ColorSet::BLUE).union(ColorSet::RED);
        let azorius = ColorSet::WHITE.union(ColorSet::BLUE);
        assert!(jeskai.contains_all(azorius));
        assert!(!azorius.contains_all(jeskai));
    }

    #[test]
    fn test_color_set_with_without() {
        let set = ColorSet::new().with(Color::Green).with(Color::White);
        assert_eq!(set.count(), 2);

        let set = set.without(Color::Green);
        assert!(set.contains(Color::White));
        assert!(!set.contains(Color::Green));
        assert_eq!(set.count(), 1);
    }

    #[test]
    fn test_color_set_from_iter() {
        let colors = vec![Color::White, Color::Blue, Color::Black];
        let set: ColorSet = colors.into_iter().collect();
        assert_eq!(set.count(), 3);
        assert!(set.contains(Color::White));
        assert!(set.contains(Color::Blue));
        assert!(set.contains(Color::Black));
    }

    #[test]
    fn test_color_set_from_color() {
        let set: ColorSet = Color::Red.into();
        assert!(set.contains(Color::Red));
        assert_eq!(set.count(), 1);
    }
}
