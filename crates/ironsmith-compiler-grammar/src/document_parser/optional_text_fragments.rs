//! Finite optional-text documents, compiled before any runtime materialization.
use super::*;
use crate::model::{OptionalTextVariantBranch, OptionalTextVariants};

fn casting_marker(tokens: &[OwnedLexToken]) -> bool {
    let words=crate::lexer::parser_token_word_refs(tokens).into_iter().map(|word|word.replace(['\'', '’'], "")).collect::<Vec<_>>();
    words.iter().map(String::as_str).collect::<Vec<_>>()==["for","each","time","you","pay","this","spells","optional","additional","cost","remove","one","bracketed","text","fragment","from","it"]
}
fn removes_fragment(tokens:&[OwnedLexToken])->bool {
    let mut quoted=false; let mut parentheses=0usize;
    for (index,token) in tokens.iter().enumerate() {
        if token.kind==TokenKind::Quote {quoted=!quoted;}
        if !quoted && token.kind==TokenKind::LParen {parentheses+=1;}
        if !quoted && token.kind==TokenKind::RParen {parentheses=parentheses.saturating_sub(1);}
        if !quoted && parentheses==0 && tokens.get(index..index+5).is_some_and(|slice|slice.iter().zip(["remove","one","bracketed","text","fragment"]).all(|(token,word)|token.is_word(word))) {return true;}
    }
    false
}
#[derive(Clone,Copy)]
enum Bracket { Literal, Fragment(usize) }
#[derive(Clone)]
struct LineBrackets { starts:std::collections::HashMap<usize,Bracket>, ends:std::collections::HashMap<usize,Bracket> }
fn literal_bracket(tokens:&[OwnedLexToken],start:usize)->bool {
    // Loyalty costs retain both their symbols and their brackets in every plan.
    if start==0 {
        if let Some(end)=tokens.iter().enumerate().skip(start+1).find_map(|(index,token)|(token.kind==TokenKind::RBracket).then_some(index)) {
            let body=&tokens[start+1..end];
            if tokens.get(end+1).is_some_and(|token|token.kind==TokenKind::Colon)
                && ((body.len()==2 && matches!(body[0].kind,TokenKind::Plus|TokenKind::Dash) && (body[1].kind==TokenKind::Number || body[1].is_word("x")))
                    || (body.len()==1 && body[0].parser_text().parse::<i32>().is_ok())) {return true;}
        }
    }
    // Compiler program boundaries are delimiters, not removable Oracle text.
    if start>0 && tokens[start-1].kind==TokenKind::Colon {
        let words=crate::lexer::parser_token_word_refs(&tokens[..start-1]);
        return words.ends_with(&["do","the","following"])
            || (words.starts_with(&["repeat","the","following","process"]) && words.last()==Some(&"times"));
    }
    false
}
fn line_brackets(line:&PreprocessedLine,labels:&mut Vec<String>)->Result<LineBrackets,CardTextError> {
    let mut starts=std::collections::HashMap::new();let mut ends=std::collections::HashMap::new();let mut stack=Vec::new(); let mut parentheses=0usize;
    for (index,token) in line.tokens.iter().enumerate() {
        if token.kind==TokenKind::LParen {parentheses+=1;}
        if token.kind==TokenKind::RParen {parentheses=parentheses.saturating_sub(1);}
        match token.kind {
            TokenKind::LBracket=>{
                let kind=if parentheses>0 || literal_bracket(&line.tokens,index){Bracket::Literal}else{
                    if labels.len()>=12 {return Err(CardTextError::ParseError("optional text supports at most twelve independently bracketed fragments".into()));}
                    let bit=labels.len();labels.push(String::new());Bracket::Fragment(bit)
                };
                starts.insert(index,kind);stack.push((index,kind));
            }
            TokenKind::RBracket=>{
                let Some((start,kind))=stack.pop() else{return Err(CardTextError::ParseError("optional text has an unmatched closing bracket".into()));};
                if let Bracket::Fragment(bit)=kind {labels[bit]=render_token_slice(&line.tokens[start+1..index]);}
                ends.insert(index,kind);
            }
            _=>{},
        }
    }
    if !stack.is_empty(){return Err(CardTextError::ParseError("optional text has an unmatched opening bracket".into()));}
    Ok(LineBrackets{starts,ends})
}
fn rewrite_document(document:&PreprocessedDocument,lines:&[Option<LineBrackets>],removed:u64)->PreprocessedDocument {
    let mut rewritten=document.clone();let mut items=Vec::new();
    for (item_index,item) in document.items.iter().enumerate() {
        let PreprocessedItem::Line(line)=item else{items.push(item.clone());continue;};
        if casting_marker(&line.tokens){continue;}
        let Some(shape)=&lines[item_index] else{items.push(item.clone());continue;};
        let mut stack=Vec::<bool>::new();let mut tokens=Vec::new();
        for (index,token) in line.tokens.iter().enumerate(){
            if let Some(kind)=shape.starts.get(&index){
                let outer_removed=stack.iter().any(|removed|*removed);
                let remove=matches!(kind,Bracket::Fragment(bit) if removed&(1u64<<*bit)!=0);
                stack.push(remove);
                if !outer_removed && !remove && matches!(kind,Bracket::Literal){tokens.push(token.clone());}
            }else if let Some(kind)=shape.ends.get(&index){
                let suppressed=stack.iter().any(|removed|*removed);stack.pop();
                if !suppressed && matches!(kind,Bracket::Literal){tokens.push(token.clone());}
            }else if !stack.iter().any(|removed|*removed){tokens.push(token.clone());}
        }
        if !tokens.iter().any(|token|matches!(token.kind,TokenKind::Word|TokenKind::Number|TokenKind::ManaGroup)){continue;}
        let mut line=rewrite_line_tokens(line,&tokens);
        // The variant's parsed source is authoritative: source recovery may
        // not reintroduce text deleted by this exact subset.
        line.info.source_tokens=tokens;
        items.push(PreprocessedItem::Line(line));
    }
    rewritten.items=items;rewritten
}

pub(super) fn read(document:&PreprocessedDocument)->Result<Option<OptionalTextVariants<PreprocessedItem>>,CardTextError>{
    let casting_payment=document.items.iter().any(|item|matches!(item,PreprocessedItem::Line(line) if casting_marker(&line.tokens)));
    if !casting_payment && !document.items.iter().any(|item|matches!(item,PreprocessedItem::Line(line) if removes_fragment(&line.tokens))){return Ok(None);}
    let mut labels=Vec::new();let mut lines=Vec::new();
    for item in &document.items {
        match item {
            PreprocessedItem::Line(line) if !casting_marker(&line.tokens)=>lines.push(Some(line_brackets(line,&mut labels)?)),
            PreprocessedItem::Line(line)=>{crate::word_claims::claim_tokens(&line.tokens);lines.push(None);}
            _=>lines.push(None),
        }
    }
    if labels.is_empty(){return Err(CardTextError::ParseError("optional text removal requires a bracketed text fragment".into()));}
    let branches=(0..(1u64<<labels.len())).map(|removed|OptionalTextVariantBranch{removed,items:rewrite_document(document,&lines,removed).items}).collect();
    Ok(Some(OptionalTextVariants{labels,branches,casting_payment}))
}
