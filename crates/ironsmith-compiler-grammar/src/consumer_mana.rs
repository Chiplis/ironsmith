//! Consumer-side spending requirements. Producer restrictions ("spend this
//! mana only ...") belong to the mana-producing ability and are separate.
use crate::word_primitives::WordWindows as _;
use crate::lexer::{OwnedLexToken, TokenWordView};
use ironsmith_core::mana::{ManaProducerFilter, ManaSpendingRestriction};

pub(crate) fn source_spending_rule(
    tokens: &[OwnedLexToken],
    alternative_only: bool,
) -> Option<ManaSpendingRestriction> {
    if !alternative_only && crate::word_claims::speculative_keep_if(|| read_bounded_counter_payment(tokens), |matched| *matched) { return Some(ManaSpendingRestriction::GenericManaByMinusOneCounters); }
    let words = TokenWordView::new(tokens).word_refs();
    let body = words.strip_prefix(&["spend", "only", "mana", "produced", "by"])?;
    let producer = if alternative_only {
        body.strip_suffix(&["to", "cast", "it", "this", "way"])?
    } else {
        body.strip_suffix(&["to", "cast", "this", "spell"])?
    };
    Some(ManaSpendingRestriction::ProducedBy(mana_producer_filter(producer)?))
}

/// The producer a "mana produced by <producers>" rule names: optional
/// supertypes ("basic", "snow", "legendary") then one plural card type or
/// subtype ("lands", "creatures", "Treasures", "Forests"). The rule judges
/// the producer's characteristics as the mana was produced.
fn mana_producer_filter(words: &[&str]) -> Option<ManaProducerFilter> {
    let (noun, modifiers) = words.split_last()?;
    let mut parts = Vec::new();
    if let Some(card_type) = crate::util::parse_card_type(noun) {
        parts.push(ManaProducerFilter::CardType(card_type));
    } else if let Some(subtype) = crate::util::parse_subtype_flexible(noun) {
        parts.push(ManaProducerFilter::Subtype(subtype));
    } else {
        return None;
    }
    for modifier in modifiers {
        if let Some(supertype) = crate::util::parse_supertype_word(modifier) {
            parts.push(ManaProducerFilter::Supertype(supertype));
        } else if let Some(card_type) = crate::util::parse_card_type(modifier) {
            parts.push(ManaProducerFilter::CardType(card_type));
        } else if let Some(subtype) = crate::util::parse_subtype_flexible(modifier) {
            parts.push(ManaProducerFilter::Subtype(subtype));
        } else {
            return None;
        }
    }
    Some(if parts.len() == 1 {
        parts.remove(0)
    } else {
        ManaProducerFilter::All(parts)
    })
}

/// The mana a per-unit spending rule admits: "mana of the chosen color",
/// "black and/or red mana", "colored mana", "colorless mana", "snow mana",
/// "mana produced by <producers>".
fn unit_spending_requirement(words: &[&str]) -> Option<ManaSpendingRestriction> {
    use ironsmith_core::color::{Color, ColorSet};
    Some(match words {
        ["mana", "of", "the", "chosen", "color"] | ["mana", "of", "chosen", "color"] => {
            ManaSpendingRestriction::SourceChosenColor
        }
        ["colored", "mana"] => {
            ManaSpendingRestriction::Colors(Color::ALL.into_iter().map(ColorSet::from_color).fold(ColorSet::new(), ColorSet::union))
        }
        ["colorless", "mana"] => ManaSpendingRestriction::Colorless,
        ["snow", "mana"] => ManaSpendingRestriction::Snow,
        ["mana", "produced", "by", producer @ ..] => {
            ManaSpendingRestriction::ProducedBy(mana_producer_filter(producer)?)
        }
        [colors @ .., "mana"] if !colors.is_empty() => {
            // "black and/or red" lexes its connective as "and or"; a list is
            // colors separated by one or more connective words.
            let mut set = ColorSet::new();
            let mut expect_color = true;
            for word in colors.iter() {
                if matches!(*word, "and" | "or" | "and/or") {
                    if set.is_empty() {
                        return None;
                    }
                    expect_color = true;
                } else if expect_color {
                    set = set.with(Color::from_name(word)?);
                    expect_color = false;
                } else {
                    return None;
                }
            }
            if expect_color {
                return None;
            }
            ManaSpendingRestriction::Colors(set)
        }
        _ => return None,
    })
}

/// "Spend only mana of the chosen color to activate this ability." (Throne
/// of Eldraine), "Spend only black mana to activate this ability.": a
/// consumer-side rule every mana unit paid for this ability's cost obeys.
pub(crate) fn activation_spending_rule(tokens: &[OwnedLexToken]) -> Option<ManaSpendingRestriction> {
    crate::word_claims::speculative_keep_if(
        || {
            let clean = crate::util::trim_edge_punctuation_tokens(tokens);
            let words = TokenWordView::new(clean).word_refs();
            if let Some(body) = words.strip_prefix(&["spend", "mana", "of", "at", "least"])
                .and_then(|body| body.strip_suffix(&["different", "colors", "to", "activate", "this", "ability"]))
                && let [number] = body
                && let Some(minimum) = crate::util::parse_number_word_i32(number).and_then(|n| u8::try_from(n).ok()).filter(|n| (1..=5).contains(n))
            {
                crate::word_claims::claim_words(&words);
                return Some(ManaSpendingRestriction::AtLeastColors(minimum));
            }
            let body = words
                .strip_prefix(&["spend", "only"])?
                .strip_suffix(&["to", "activate", "this", "ability"])?;
            let rule = if matches!(body,
                ["mana", "of", "a", "color", "that", "hasn't" | "hasnt", "been", "spent", "this", "way"]
                | ["mana", "of", "a", "color", "that", "has", "not", "been", "spent", "this", "way"])
            { ManaSpendingRestriction::UnspentActivationColors }
            else { unit_spending_requirement(body)? };
            // A complete sentence match: every word was read.
            crate::word_claims::claim_words(&words);
            Some(rule)
        },
        Option::is_some,
    )
}

/// One complete intrinsic rule, reusable by nested quoted static abilities.
/// The opaque payment name is consumed only by the bounded label reader.
pub(crate) fn named_tap_prohibition_ability(
    tokens: &[OwnedLexToken],
) -> Option<crate::model::CompilerStaticAbilityCore> {
    named_tap_prohibition(tokens).map(crate::model::CompilerStaticAbilityCore::generic_mana_tap_payment_prohibition)
}

pub(crate) fn spell_source_spending_ability(
    tokens: &[OwnedLexToken],
) -> Option<crate::model::CompilerStaticAbilityCore> {
    if let Some(ability)=named_tap_prohibition_ability(tokens){return Some(ability);}
    if let Some((spells, activated_abilities, exclude_source, name)) = source_generic_tap_payment(tokens) {
        let mut rule=crate::model::CompilerStaticAbilityCore::generic_mana_tap_payment(
            spells, activated_abilities, exclude_source, crate::lexer::render_token_slice(tokens));
        if let ironsmith_core::StaticAbilityPayload::GenericManaTapPayment{name:slot,..}=&mut rule.payload{*slot=name;}
        return Some(rule);
    }
    let rule = source_spending_rule(tokens, false)
        .or_else(|| x_spending_rule(tokens))
        .or_else(|| no_mana_spending_rule(tokens))?;
    Some(
        crate::model::CompilerStaticAbilityCore::spell_mana_spending_restriction(
            rule,
            crate::lexer::render_token_slice(tokens)
                .trim()
                .trim_end_matches('.'),
        ),
    )
}

/// "You can't spend mana to cast this spell." (Hogaak, Arisen Necropolis):
/// the whole cost must be paid without spending mana (convoke, delve).
pub(crate) fn no_mana_spending_rule(tokens: &[OwnedLexToken]) -> Option<ManaSpendingRestriction> {
    let words = TokenWordView::new(tokens).word_refs();
    matches!(
        words.as_slice(),
        ["you", "can't" | "cant" | "cannot", "spend", "mana", "to", "cast", "this", "spell"]
    )
    .then_some(ManaSpendingRestriction::NoMana)
}

/// "For each mana in this ability's activation cost, you may tap an untapped
/// creature you control rather than pay that mana." (Heirloom Epic): the
/// activation price admits a creature tap per generic mana. It is an
/// alternative way to pay the cost, never a resolving instruction.
pub(crate) fn is_creature_tap_payment_sentence(tokens: &[OwnedLexToken]) -> bool {
    let mut words = TokenWordView::new(tokens).word_refs();
    // The payment planner already limits this method to generic pips.
    // Retain the explicit authored restriction instead of requiring the
    // older abbreviated surface that merely says "each mana".
    if words.starts_with(&["for", "each", "generic", "mana"]) {words.remove(2);}
    let recognized = matches!(
        words.as_slice(),
        [
            "for", "each", "mana", "in", "this", "ability's" | "abilitys" | "ability",
            "activation", "cost", "you", "may", "tap", "an", "untapped", "creature", "you",
            "control", "rather", "than", "pay", "that", "mana",
        ] | [
            "for", "each", "mana", "in", "this", "ability's" | "abilitys" | "ability", "s",
            "activation", "cost", "you", "may", "tap", "an", "untapped", "creature", "you",
            "control", "rather", "than", "pay", "that", "mana",
        ]
    );
    if recognized { crate::word_claims::claim_tokens(tokens); }
    recognized
}

/// The artifact-only counterpart of the generic activation tap payment.
pub(crate) fn is_artifact_tap_payment_sentence(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    let body = words.strip_prefix(&["for", "each", "mana", "in", "this"]);
    let recognized = matches!(body, Some(["ability's" | "abilitys" | "ability", "activation", "cost", "you", "may", "tap", "an", "untapped", "artifact", "you", "control", "rather", "than", "pay", "that", "mana"]) | Some(["ability's" | "abilitys" | "ability", "s", "activation", "cost", "you", "may", "tap", "an", "untapped", "artifact", "you", "control", "rather", "than", "pay", "that", "mana"]));
    if recognized { crate::word_claims::claim_tokens(tokens); }
    recognized
}
pub(crate) fn allow_artifact_tap_payment(
    cost: ironsmith_core::TotalCost<crate::model::CompilerCost>,
) -> ironsmith_core::TotalCost<crate::model::CompilerCost> {
    cost.try_map(|component| Ok::<_, std::convert::Infallible>(match component {
        crate::model::CompilerCost::Mana(cost) => crate::model::CompilerCost::Mana(cost.with_artifact_tap_payment()),
        other => other,
    })).unwrap_or_else(|never| match never {})
}

/// Let every mana component of an activation price be paid by tapping
/// untapped creatures the activator controls, one {1} each.
pub(crate) fn allow_creature_tap_payment(
    cost: ironsmith_core::TotalCost<crate::model::CompilerCost>,
) -> ironsmith_core::TotalCost<crate::model::CompilerCost> {
    cost.try_map(|component| Ok::<_, std::convert::Infallible>(match component {
        crate::model::CompilerCost::Mana(cost) => {
            crate::model::CompilerCost::Mana(cost.with_creature_tap_payment())
        }
        other => other,
    })).unwrap_or_else(|never| match never {})
}

/// A restriction on actual mana spent on X. It is never lowered as a colored
/// mana symbol or as a resolution effect.
pub(crate) fn x_spending_rule(tokens: &[OwnedLexToken]) -> Option<ManaSpendingRestriction> {
    use ironsmith_core::color::{Color, ColorSet};
    let words = TokenWordView::new(tokens).word_refs();
    let body = words.strip_prefix(&["spend", "only"])?;
    let split = body.word_windows(3).position(|part| part == ["mana", "on", "x"])?;
    let colors = match &body[..split] {
        ["colored"] => Color::ALL.into_iter().collect(),
        [single] => ColorSet::from_color(Color::from_name(single)?),
        [first, "and/or", second] | [first, "or", second] =>
            ColorSet::from_color(Color::from_name(first)?).with(Color::from_name(second)?),
        [first, "and", "or", second] =>
            ColorSet::from_color(Color::from_name(first)?).with(Color::from_name(second)?),
        _ => return None,
    };
    let maximum_per_color = match &body[split + 3..] {
        [] => None,
        ["no", "more", "than", "one" | "1", "mana", "of", "each", "color", "may", "be", "spent", "this", "way"] => Some(1),
        _ => return None,
    };
    // A complete sentence match: every word was read.
    crate::word_claims::claim_words(&words);
    Some(ManaSpendingRestriction::OnX { colors, maximum_per_color })
}

/// Split only complete recognized spending sentences. The enclosing ability
/// or modal owner must consume the returned rules as announcement costs.
pub(crate) fn split_x_spending_sentences(tokens: &[OwnedLexToken])
    -> (Vec<OwnedLexToken>, Vec<ManaSpendingRestriction>) {
    let mut kept = Vec::new();
    let mut rules = Vec::new();
    for sentence in crate::lexer::split_lexed_sentences(tokens) {
        if let Some(rule) = x_spending_rule(sentence) {
            if !rules.contains(&rule) { rules.push(rule); }
        } else { kept.push(sentence.to_vec()); }
    }
    (crate::util::join_sentences_with_period(&kept), rules)
}

pub(crate) fn constrain_activation_cost(
    cost: ironsmith_core::TotalCost<crate::model::CompilerCost>,
    rules: &[ManaSpendingRestriction],
) -> ironsmith_core::TotalCost<crate::model::CompilerCost> {
    cost.try_map(|component| Ok::<_, std::convert::Infallible>(match component {
        crate::model::CompilerCost::Mana(mut cost) => {
            for rule in rules { cost = cost.with_spending_restriction(rule.clone()); }
            crate::model::CompilerCost::Mana(cost)
        },
        other => other,
    })).unwrap_or_else(|never| match never {})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tokens(text: &str) -> Vec<OwnedLexToken> { crate::lexer::lex_line(text, 0).unwrap() }
    #[test]
    fn artifact_activation_tap_payment_requires_exact_owner_and_scope() {
        assert!(is_artifact_tap_payment_sentence(&tokens("For each mana in this ability's activation cost, you may tap an untapped artifact you control rather than pay that mana.")));
        for text in [
            "For each mana in this spell's activation cost, you may tap an untapped artifact you control rather than pay that mana.",
            "For each mana in this ability's activation cost, you may tap an untapped creature you control rather than pay that mana.",
            "For each mana in this ability's activation cost, you may tap an untapped artifact an opponent controls rather than pay that mana.",
        ] { assert!(!is_artifact_tap_payment_sentence(&tokens(text)), "{text}"); }
    }
    #[test]
    fn x_spending_reader_requires_the_exact_scope_and_color_relationship() {
        use ironsmith_core::color::ColorSet;
        assert_eq!(x_spending_rule(&tokens("Spend only black and/or red mana on X.")),
            Some(ManaSpendingRestriction::OnX { colors: ColorSet::BLACK.union(ColorSet::RED), maximum_per_color: None }));
        assert!(matches!(x_spending_rule(&tokens("Spend only colored mana on X. No more than one mana of each color may be spent this way.")),
            Some(ManaSpendingRestriction::OnX { maximum_per_color: Some(1), .. })));
        for unsupported in ["Spend only black mana to cast this spell.", "Spend this mana only on X.",
            "Spend only black mana on Y.", "Spend only red mana on X if you attacked this turn."] {
            assert!(x_spending_rule(&tokens(unsupported)).is_none(), "{unsupported}");
        }
    }
}

#[cfg(test)]
mod minimum_actual_color_tests {
    use super::*;
    #[test]
    fn minimum_colors_reads_only_complete_bounded_activation_requirement() {
        let read = |text| activation_spending_rule(&crate::lexer::lex_line(text, 0).unwrap());
        assert_eq!(read("Spend mana of at least three different colors to activate this ability."), Some(ManaSpendingRestriction::AtLeastColors(3)));
        assert_eq!(read("Spend mana of at least 2 different colors to activate this ability."), Some(ManaSpendingRestriction::AtLeastColors(2)));
        assert_eq!(read("Spend mana of at least six different colors to activate this ability."), None);
        assert_eq!(read("Spend mana of at least three different colors to activate another ability."), None);
        assert_eq!(read("Spend mana of at least three different colors to activate this ability and draw a card."), None);
    }
}

/// Read the generic-pip substitution, counter operand, recipient selector,
/// and independent remaining-toughness bound. No keyword identity is inferred.
fn read_bounded_counter_payment(tokens: &[OwnedLexToken]) -> bool {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let Some(put) = tokens.iter().position(|token| token.is_word("put")) else { return false; };
    let words = TokenWordView::new(&tokens[..put]).word_refs();
    if !matches!(words.as_slice(),
        ["for", "each", "generic", "mana", "in", "this", "spell's" | "spells", "total", "cost", "you", "may"]
        | ["for", "each", "generic", "mana", "in", "this", "spell", "s", "total", "cost", "you", "may"])
    { return false; }
    let Some(counter) = tokens.iter().enumerate().skip(put+1).find_map(|(i,t)|t.is_word("counter").then_some(i)) else { return false; };
    let start = put + 1 + usize::from(tokens.get(put+1).is_some_and(|token| token.is_word("a")));
    if crate::grammar::filters::parse_counter_type_from_tokens(&tokens[start..counter]) != Some(ironsmith_core::CounterType::MinusOneMinusOne)
        || !tokens.get(counter+1).is_some_and(|token|token.is_word("on")) { return false; }
    let Some(bound) = tokens.iter().enumerate().skip(counter+2).find_map(|(i,t)|t.is_any_word(&["with","up"]).then_some(i)) else { return false; };
    let Ok(filter) = crate::object_filters::parse_object_filter_lexed(&tokens[counter+2..bound],false) else { return false; };
    if filter != ironsmith_core::ObjectFilter::creature().controlled_by(ironsmith_core::PlayerFilter::You) { return false; }
    let Some(rather) = tokens.iter().enumerate().skip(bound).find_map(|(i,t)|t.is_word("rather").then_some(i)) else { return false; };
    if TokenWordView::new(&tokens[rather..]).word_refs() != ["rather","than","pay","that","mana"] { return false; }
    let bounds = &tokens[bound..rather];
    let short = TokenWordView::new(bounds).word_refs() == ["up","to","its","remaining","toughness"];
    let long = bounds.first().is_some_and(|t|t.is_word("with"))
        && bounds.get(1).is_some_and(|t|t.is_word("toughness"))
        && bounds.iter().position(|t|t.is_word("or")).is_some_and(|or| {
            crate::util::parse_number_word_u32(&crate::lexer::render_token_slice(&bounds[2..or])) == Some(1)
                && TokenWordView::new(&bounds[or..]).word_refs() == ["or","greater","and","less","damage","marked","on","it","than","its","toughness"]
        });
    if !short && !long { return false; }
    crate::word_claims::claim_tokens(tokens);
    true
}

#[cfg(test)]
mod counter_substitution_tests {
    use super::*;
    #[test]
    fn bounded_counter_payment_reads_operands_and_declines_broadened_counter_or_recipient() {
        let text = "For each generic mana in this spell's total cost, you may put a -1/-1 counter on a creature you control with toughness 1 or greater and less damage marked on it than its toughness rather than pay that mana.";
        let read = |text: &str| source_spending_rule(&crate::lexer::lex_line(text,0).unwrap(),false);
        assert_eq!(read(text),Some(ManaSpendingRestriction::GenericManaByMinusOneCounters));
        assert_eq!(read(&text.replace("-1/-1","+1/+1")),None);
        assert_eq!(read(&text.replace("you control","an opponent controls")),None);
        assert_eq!(read(&text.replace("generic mana","colored mana")),None);
        assert_eq!(read(&text.replace("toughness 1 or greater and less damage marked on it than its toughness","flying")),None);
    }
}

/// Source-wide generic-only tap payment. The ordinary mana planner owns tapping.
fn source_generic_tap_payment(tokens: &[OwnedLexToken]) -> Option<(bool,bool,bool,Option<String>)> {
    let sentences=crate::lexer::split_lexed_sentences(tokens);
    let mut combined=(false,false,false);let mut name=None;
    for sentence in sentences {
        if let Some(label)=named_tap_label(sentence,&["this","generic","tap","payment","is","named"]){
            if name.replace(label).is_some(){return None}continue;
        }
        let (spells,activated,exclude)=source_generic_tap_payment_sentence(sentence)?;
        combined.0|=spells;combined.1|=activated;combined.2|=exclude;
    }
    (combined.0||combined.1).then_some((combined.0,combined.1,combined.2,name))
}
fn source_generic_tap_payment_sentence(tokens: &[OwnedLexToken]) -> Option<(bool,bool,bool)> {
    let view=TokenWordView::new(tokens);
    let words=view.word_refs().into_iter().map(|word|word.replace(['\'', '’'], "")).filter(|word|word!="s").collect::<Vec<_>>();
    let words=words.iter().map(String::as_str).collect::<Vec<_>>();
    let result=match words.as_slice() {
        ["for","each","generic","mana","in","this","spells" | "spell","total","cost","you","may","tap","an","untapped","creature","you","control","rather","than","pay","that","mana"] => (true,false,false),
        ["for","each","generic","mana","in","the","total","cost","of","this","permanents" | "permanent","abilities","you","may","tap","another","untapped","creature","you","control","rather","than","pay","that","mana"] => (false,true,true),
        ["for","each","generic","mana","in","the","total","cost","of","this","permanents" | "permanent","abilities","you","may","tap","an","untapped","creature","you","control","rather","than","pay","that","mana"] => (false,true,false),
        _=>return None,
    };
    crate::word_claims::claim_tokens(tokens);Some(result)
}

fn named_tap_label(tokens:&[OwnedLexToken],prefix:&[&str])->Option<String>{
    let tokens=crate::util::trim_edge_punctuation_tokens(tokens);let view=TokenWordView::new(&tokens);let words=view.word_refs();
    if !words.starts_with(prefix)||words.len()<=prefix.len(){return None}
    let start=*view.token_start_indices().get(prefix.len())?;
    let label=crate::lexer::render_literal_token_slice(&tokens[start..]).trim().to_ascii_lowercase();
    if label.is_empty()||label.len()>64||!label.chars().all(|ch|ch.is_ascii_alphabetic()||matches!(ch,' '|'-'|'\'')){return None}
    crate::word_claims::claim_tokens(&tokens);Some(label)
}
fn named_tap_prohibition(tokens:&[OwnedLexToken])->Option<String>{
    for subject in ["creature","permanent"]{
        for negative in ["can't","cannot"]{
            if let Some(name)=named_tap_label(tokens,&["this",subject,negative,"be","tapped","to","pay","the","generic","tap","payment","named"]){return Some(name)}
        }
    }
    None
}

#[cfg(test)]
mod explicit_generic_creature_payment {
    use super::*;
    #[test]
    fn explicit_generic_scope_uses_the_existing_generic_only_method() {
        let text="For each generic mana in this ability's activation cost, you may tap an untapped creature you control rather than pay that mana.";
        let tokens=crate::lexer::lex_line(text,0).unwrap();
        assert!(is_creature_tap_payment_sentence(&tokens));
        let tokens=crate::lexer::lex_line(&text.replace("generic mana","colored mana"),0).unwrap();
        assert!(!is_creature_tap_payment_sentence(&tokens));
    }
}
