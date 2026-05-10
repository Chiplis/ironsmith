use std::borrow::Cow;

/// Types of counters that can be placed on objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CounterType {
    PlusOnePlusOne,
    MinusOneMinusOne,
    PlusOnePlusZero,
    PlusZeroPlusOne,
    PlusOnePlusTwo,
    PlusTwoPlusTwo,
    MinusZeroMinusTwo,
    MinusTwoMinusOne,
    MinusTwoMinusTwo,
    Deathtouch,
    Decayed,
    DoubleStrike,
    FirstStrike,
    Flying,
    Haste,
    Hexproof,
    Indestructible,
    Lifelink,
    Menace,
    Reach,
    Trample,
    Vigilance,
    Loyalty,
    Charge,
    Age,
    Aim,
    Arrow,
    Awakening,
    Blood,
    Brain,
    Bounty,
    Brick,
    Corpse,
    Credit,
    Crystal,
    Cube,
    Currency,
    Death,
    Depletion,
    Despair,
    Devotion,
    Divinity,
    Doom,
    Dream,
    Echo,
    Egg,
    Energy,
    Enlightened,
    Eon,
    Experience,
    Eyeball,
    Fade,
    Fate,
    Feather,
    Filibuster,
    Finality,
    Flame,
    Flood,
    Foreshadow,
    Fungus,
    Fuse,
    Gem,
    Glyph,
    Gold,
    Growth,
    Hatchling,
    Healing,
    Hit,
    Hoofprint,
    Hour,
    Hunger,
    Ice,
    Incarnation,
    Infection,
    Intervention,
    Isolation,
    Javelin,
    Ki,
    Keyword,
    Knowledge,
    Level,
    Lore,
    Luck,
    Magnet,
    Manifestation,
    Mannequin,
    Matrix,
    Mine,
    Mining,
    Mire,
    Music,
    Muster,
    Net,
    Night,
    Oil,
    Omen,
    Ore,
    Page,
    Pain,
    Paralyzation,
    Petal,
    Petrification,
    Phylactery,
    Pin,
    Plague,
    Plot,
    Polyp,
    Poison,
    Pressure,
    Prey,
    Pupa,
    Quest,
    Rad,
    Scream,
    Shield,
    Silver,
    Sleep,
    Slime,
    Slumber,
    Soot,
    Soul,
    Spore,
    Storage,
    Strife,
    Study,
    Stun,
    Void,
    Task,
    Theft,
    Tide,
    Time,
    Tower,
    Training,
    Trap,
    Treasure,
    Unity,
    Velocity,
    Verse,
    Vitality,
    Volatile,
    Voyage,
    Wage,
    Winch,
    Wind,
    Wish,
    Named(&'static str),
}

impl CounterType {
    pub fn pt_delta(&self) -> Option<(i32, i32)> {
        match self {
            CounterType::PlusOnePlusOne => Some((1, 1)),
            CounterType::MinusOneMinusOne => Some((-1, -1)),
            CounterType::PlusOnePlusZero => Some((1, 0)),
            CounterType::PlusZeroPlusOne => Some((0, 1)),
            CounterType::PlusOnePlusTwo => Some((1, 2)),
            CounterType::PlusTwoPlusTwo => Some((2, 2)),
            CounterType::MinusZeroMinusTwo => Some((0, -2)),
            CounterType::MinusTwoMinusOne => Some((-2, -1)),
            CounterType::MinusTwoMinusTwo => Some((-2, -2)),
            _ => None,
        }
    }

    pub fn description(self) -> Cow<'static, str> {
        match self {
            CounterType::PlusOnePlusOne => Cow::Borrowed("+1/+1"),
            CounterType::MinusOneMinusOne => Cow::Borrowed("-1/-1"),
            CounterType::PlusOnePlusZero => Cow::Borrowed("+1/+0"),
            CounterType::PlusZeroPlusOne => Cow::Borrowed("+0/+1"),
            CounterType::PlusOnePlusTwo => Cow::Borrowed("+1/+2"),
            CounterType::PlusTwoPlusTwo => Cow::Borrowed("+2/+2"),
            CounterType::MinusZeroMinusTwo => Cow::Borrowed("-0/-2"),
            CounterType::MinusTwoMinusOne => Cow::Borrowed("-2/-1"),
            CounterType::MinusTwoMinusTwo => Cow::Borrowed("-2/-2"),
            CounterType::DoubleStrike => Cow::Borrowed("double strike"),
            CounterType::FirstStrike => Cow::Borrowed("first strike"),
            CounterType::Named(name) => Cow::Owned(name.to_string()),
            other => Cow::Owned(split_pascal_case_identifier(&format!("{other:?}"))),
        }
    }
}

fn split_pascal_case_identifier(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 4);
    for (idx, ch) in raw.chars().enumerate() {
        if idx > 0 && ch.is_ascii_uppercase() {
            out.push(' ');
        }
        out.push(ch.to_ascii_lowercase());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::CounterType;

    #[test]
    fn counter_type_keeps_pt_deltas() {
        assert_eq!(CounterType::PlusOnePlusOne.pt_delta(), Some((1, 1)));
        assert_eq!(CounterType::MinusTwoMinusTwo.pt_delta(), Some((-2, -2)));
        assert_eq!(CounterType::Loyalty.pt_delta(), None);
    }

    #[test]
    fn counter_type_descriptions_are_stable() {
        assert_eq!(CounterType::Flying.description(), "flying");
        assert_eq!(CounterType::PlusOnePlusOne.description(), "+1/+1");
        assert_eq!(CounterType::Named("hour").description(), "hour");
    }
}
