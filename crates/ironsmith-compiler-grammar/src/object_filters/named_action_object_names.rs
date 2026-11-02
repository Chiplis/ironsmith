//! A named operation's captured objects form a global historical name set.
use crate::lexer::{OwnedLexToken,TokenKind,TokenWordView};
pub(crate) fn split_clause(tokens:&[OwnedLexToken])->Option<(Vec<OwnedLexToken>,ironsmith_core::NamedActionObjectNameCondition)>{
 let tokens=crate::util::trim_edge_punctuation_tokens(tokens);
 let view=TokenWordView::new(tokens);let words=view.word_refs();
 let phrase=["sharing","a","name","with","a","card","from","your"];
 let start=words.windows(phrase.len()).position(|part|part==phrase)?;
 let tail=&words[start+phrase.len()..];
 let (tail,this_turn)=tail.strip_suffix(&["this","turn"]).map_or((tail,false),|tail|(tail,true));
 let label=tail.strip_suffix(&["action"])?;
 if label.is_empty(){return None}
 let starts=view.token_start_indices();let at=starts[start];
 if tokens[at..].iter().any(|token|token.kind!=TokenKind::Word){return None}
 crate::word_claims::claim_tokens(&tokens[at..]);
 Some((tokens[..at].to_vec(),ironsmith_core::NamedActionObjectNameCondition{label:label.join(" "),player:ironsmith_core::PlayerFilter::You,this_turn}))
}
