//! English cardinal and ordinal spellings share the tens prefix in compounds.

const TENS: &[&str] = &[
    "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
];

pub(super) const CARDINAL: NumberWords = NumberWords {
    first: 0,
    small: &[
        "zero",
        "one",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
    ],
    tens: TENS,
    hundred: "one hundred",
};

pub(super) const ORDINAL: NumberWords = NumberWords {
    first: 1,
    small: &[
        "first",
        "second",
        "third",
        "fourth",
        "fifth",
        "sixth",
        "seventh",
        "eighth",
        "ninth",
        "tenth",
        "eleventh",
        "twelfth",
        "thirteenth",
        "fourteenth",
        "fifteenth",
        "sixteenth",
        "seventeenth",
        "eighteenth",
        "nineteenth",
    ],
    tens: &[
        "twentieth",
        "thirtieth",
        "fortieth",
        "fiftieth",
        "sixtieth",
        "seventieth",
        "eightieth",
        "ninetieth",
    ],
    hundred: "one hundredth",
};

pub(super) struct NumberWords {
    first: u32,
    small: &'static [&'static str],
    tens: &'static [&'static str],
    hundred: &'static str,
}

fn index_of(words: &[&str], word: &str) -> Option<u32> {
    words
        .iter()
        .position(|s| s.eq_ignore_ascii_case(word))
        .map(|i| i as u32)
}

impl NumberWords {
    fn small(&self, n: u32) -> Option<&'static str> {
        self.small.get(n.checked_sub(self.first)? as usize).copied()
    }

    pub(super) fn spell(&self, n: u32) -> Option<String> {
        match n {
            0..=19 => Some(self.small(n)?.to_owned()),
            20..=99 => {
                let tens = (n / 10 - 2) as usize;
                match n % 10 {
                    0 => Some(self.tens[tens].to_owned()),
                    ones => Some(format!("{}-{}", TENS[tens], self.small(ones)?)),
                }
            }
            100 => Some(self.hundred.to_owned()),
            _ => None,
        }
    }

    pub(super) fn compound(&self, tens: &str, ones: &str) -> Option<u32> {
        let tens = index_of(TENS, tens)? + 2;
        let ones = index_of(self.small, ones)? + self.first;
        (1..=9).contains(&ones).then_some(10 * tens + ones)
    }

    pub(super) fn read(&self, word: &str) -> Option<u32> {
        if word.eq_ignore_ascii_case(self.hundred) {
            return Some(100);
        }
        if let Some((tens, ones)) = word.split_once('-') {
            return self.compound(tens, ones);
        }
        index_of(self.small, word)
            .map(|i| i + self.first)
            .or_else(|| index_of(self.tens, word).map(|i| (i + 2) * 10))
    }
}
