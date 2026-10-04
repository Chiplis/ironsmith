//! Complete affirmative/negative characteristic assertions, excluding abilities.
use crate::lexer::{OwnedLexToken,parser_token_word_positions,parser_token_word_refs};
use crate::types::{CardType,Supertype};
#[derive(Debug,Clone,Copy,PartialEq)]
pub enum Property { Supertype(Supertype), CardType(CardType) }
#[derive(Debug,Clone)]
pub struct Assertion<'a> { pub subject:&'a [OwnedLexToken], pub property:Property, pub remove:bool }
fn property(words:&[&str])->Option<Property> {
    let words=words.strip_prefix(&["a"]).or_else(||words.strip_prefix(&["an"])).unwrap_or(words);
    let [word]=words else{return None};
    crate::util::parse_supertype_word(word).map(Property::Supertype)
        .or_else(||crate::util::parse_card_type(word).map(Property::CardType))
}
fn copula_tail(words:&[&str])->Option<(Property,bool)> {
    let (tail,remove)=match words {
        ["isn't"|"isnt"|"isn’t"|"aren't"|"arent"|"aren’t",rest @ ..] =>(rest,true),
        ["is"|"are","not",rest @ ..] | ["is"|"are","no","longer",rest @ ..] =>(rest,true),
        ["is"|"are",rest @ ..] =>(rest,false),
        _=>return None,
    };
    Some((property(tail)?,remove))
}
pub fn parse(tokens:&[OwnedLexToken])->Option<Assertion<'_>> {
    if tokens.iter().any(OwnedLexToken::is_quote) {return None;}
    let positions=parser_token_word_positions(tokens);let words=parser_token_word_refs(tokens);
    let verb=words.iter().position(|word|matches!(*word,"is"|"are"|"isn't"|"isnt"|"isn’t"|"aren't"|"arent"|"aren’t"))?;
    if verb==0 {return None;}
    let (property,remove)=copula_tail(&words[verb..])?;
    Some(Assertion{subject:&tokens[..positions[verb].0],property,remove})
}
pub fn color_then_remove_card_type(tokens:&[OwnedLexToken])->Option<(crate::color::ColorSet,CardType)> {
    if tokens.iter().any(OwnedLexToken::is_quote) {return None;}
    let words=parser_token_word_refs(tokens);
    for split in 1..words.len() {
        if words[split]!="and" {continue;}
        let Some((Property::CardType(kind),true))=copula_tail(&words[split+1..]) else{continue};
        let colors=super::become_shapes::parse_become_color_words(&words[..split])?;
        return Some((colors,kind));
    }
    None
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_negations_keep_subjects_and_reject_other_predicates_and_tails() {
        for text in ["Target snow land is no longer snow.","Target snow permanent isn't snow.","Target artifact creature is not an artifact."] {
            let tokens=crate::lexer::lex_line(text,0).unwrap();assert!(parse(&tokens).unwrap().remove);
        }
        for text in ["Target creature is not attacking", "Target land is snow and draw a card", "This creature has \"This land is snow.\""] {
            assert!(parse(&crate::lexer::lex_line(text,0).unwrap()).is_none(),"{text}");
        }
        let tokens=crate::lexer::lex_line("blue and isn't an artifact",0).unwrap();
        assert_eq!(color_then_remove_card_type(&tokens),Some((crate::color::ColorSet::BLUE,CardType::Artifact)));
    }
}
