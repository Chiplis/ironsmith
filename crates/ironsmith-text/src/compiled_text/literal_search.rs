//! Literal substring search for the surface normalizers.
//!
//! `str::find`, `contains`, `replace`, and `split_once` build a Two-Way
//! searcher for every call. The normalizers probe each rendered line with
//! hundreds of fixed phrases, so that setup dominated card rendering. Lines
//! are short, so a byte scan anchored on the needle's first byte is cheaper.
//! Results match the `str` methods exactly: leftmost, non-overlapping matches.

pub(crate) trait LiteralStr {
    fn find_lit(&self, needle: &str) -> Option<usize>;
    fn contains_lit(&self, needle: &str) -> bool;
    fn replace_lit(&self, from: &str, to: &str) -> String;
    fn split_once_lit<'a>(&'a self, delimiter: &str) -> Option<(&'a str, &'a str)>;
}

impl LiteralStr for str {
    fn find_lit(&self, needle: &str) -> Option<usize> {
        let haystack = self.as_bytes();
        let needle = needle.as_bytes();
        let Some((&first, tail)) = needle.split_first() else {
            return Some(0);
        };
        let last = haystack.len().checked_sub(needle.len())?;
        let mut start = 0;
        while start <= last {
            // A valid UTF-8 needle never starts with a continuation byte, so
            // every byte match lies on a character boundary.
            let at = start + memchr::memchr(first, &haystack[start..=last])?;
            if &haystack[at + 1..at + needle.len()] == tail {
                return Some(at);
            }
            start = at + 1;
        }
        None
    }

    fn contains_lit(&self, needle: &str) -> bool {
        self.find_lit(needle).is_some()
    }

    fn replace_lit(&self, from: &str, to: &str) -> String {
        if from.is_empty() {
            return self.replace(from, to);
        }
        let mut result = String::with_capacity(self.len());
        let mut rest = self;
        while let Some(at) = rest.find_lit(from) {
            result.push_str(&rest[..at]);
            result.push_str(to);
            rest = &rest[at + from.len()..];
        }
        result.push_str(rest);
        result
    }

    fn split_once_lit<'a>(&'a self, delimiter: &str) -> Option<(&'a str, &'a str)> {
        let at = self.find_lit(delimiter)?;
        Some((&self[..at], &self[at + delimiter.len()..]))
    }
}

#[cfg(test)]
mod tests {
    use super::LiteralStr;

    #[test]
    fn literal_search_matches_str_methods() {
        let haystacks = [
            "",
            "a",
            "aaa",
            "abcabc",
            "Exile it. It gains haste. It gains haste.",
            "Whenever ~ attacks, you may you attach ~ to target creature.",
            "naïve café — “quoted” — café",
        ];
        let needles = [
            "", "a", "aa", "abc", "c", "bca", "It gains ", " haste.", "you may you attach ",
            "café", "—", "“quoted”", "missing", "aaaa",
        ];
        for haystack in haystacks {
            for needle in needles {
                assert_eq!(haystack.find_lit(needle), haystack.find(needle), "{haystack:?} {needle:?}");
                assert_eq!(haystack.contains_lit(needle), haystack.contains(needle));
                assert_eq!(haystack.split_once_lit(needle), haystack.split_once(needle));
                assert_eq!(haystack.replace_lit(needle, "<>"), haystack.replace(needle, "<>"));
                assert_eq!(haystack.replace_lit(needle, ""), haystack.replace(needle, ""));
            }
        }
    }
}
