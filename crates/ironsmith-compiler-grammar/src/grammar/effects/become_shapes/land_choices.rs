//! Complete basic-land alternatives and explicit type retention.
use crate::types::Subtype;
#[derive(Debug, Clone, PartialEq)]
pub struct BasicLandChoiceTemplate {
    pub allowed_subtypes: Vec<Subtype>,
    pub preserve_other_types: bool,
}
pub fn parse_basic_land_choice_template(words: &[&str]) -> Option<BasicLandChoiceTemplate> {
    let (words, preserve_other_types) = super::strip_become_addition_tail_words(words);
    let words = words.strip_prefix(&["the"]).unwrap_or(words);
    if words == ["basic", "land", "type", "of", "your", "choice"] {
        return Some(BasicLandChoiceTemplate { allowed_subtypes: Vec::new(), preserve_other_types });
    }
    let mut subtypes = Vec::new();
    for part in words.split(|word| *word == "or") {
        let part = part.strip_prefix(&["a"]).or_else(|| part.strip_prefix(&["an"])).unwrap_or(part);
        let [word] = part else { return None; };
        let subtype = super::super::super::leaf::parse_leaf_subtype_flexible_complete(word).ok()?;
        if !subtype.is_basic_land_type() || subtypes.contains(&subtype) { return None; }
        subtypes.push(subtype);
    }
    // The older fixed-basic-land production owns the unchanged simple form.
    (preserve_other_types || subtypes.len() > 1).then_some(BasicLandChoiceTemplate {
        allowed_subtypes: subtypes, preserve_other_types,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn basic_land_choices_keep_the_exact_set_and_addition_mode() {
        assert_eq!(parse_basic_land_choice_template(&["a","plains","or","an","island"]).unwrap().allowed_subtypes, [Subtype::Plains,Subtype::Island]);
        let addition=parse_basic_land_choice_template(&["basic","land","type","of","your","choice","in","addition","to","its","other","types"]).unwrap();
        assert!(addition.preserve_other_types && addition.allowed_subtypes.is_empty());
        assert_eq!(parse_basic_land_choice_template(&["an","island","in","addition","to","its","other","types"]).unwrap().allowed_subtypes,[Subtype::Island]);
        for bad in [vec!["plains","or","island","and","forest"],vec!["plains","or","desert"],vec!["plains","or","plains"],vec!["basic","land","type","of","your","choice","and","draw"]] {assert!(parse_basic_land_choice_template(&bad).is_none());}
    }
}
