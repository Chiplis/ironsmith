use super::*;

pub fn parse_transfigure(tokens: &[OwnedLexToken]) -> Result<Option<ParsedAbility>, CardTextError> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    if !permission_shapes::prefix_words(&words, &["transfigure"])
        || words.iter().any(|word| matches!(*word, "has" | "have"))
    {
        return Ok(None);
    }
    let mana_prefix = leaf::parse_leaf_mana_cost_prefix_tokens(&tokens[1..]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "transfigure keyword missing mana cost (clause: '{}')",
            words.join(" ")
        ))
    })?;
    let base_mana_cost = mana_prefix.cost;
    let mana_cost = ironsmith_core::TotalCost::from_costs(vec![
        CompilerCost::Mana(base_mana_cost.clone()),
        CompilerCost::SacrificeSelf { surface: None },
    ]);
    let filter = ObjectFilter::default()
        .with_type(crate::types::CardType::Creature)
        .with_mana_value(Comparison::EqualExpr(Box::new(Value::ManaValueOf(
            Box::new(ChooseSpec::Source),
        ))));
    let text = format!("Transfigure {}", base_mana_cost.to_oracle());
    let effects_ast = vec![
        crate::cards::builders::EffectAst::subject_verb_search_library(
            filter,
            Zone::Battlefield,
            crate::cards::builders::PlayerAst::You,
            crate::cards::builders::PlayerAst::You,
            crate::effect::SearchSelectionMode::Exact,
            false,
            None,
            true,
            crate::effect::ChoiceCount::exactly(1),
            None,
            None,
            crate::effect::SearchResultReferenceSurface::ThatCard,
            false,
            false,
            false,
        ),
    ];
    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(
                crate::model::compiler_semantic::CompilerActivatedAbilityCore {
                    mana_cost,
                    effects: ironsmith_core::ResolutionProgram::default(),
                    choices: Vec::new(),
                    timing: ActivationTiming::SorcerySpeed,
                    additional_restrictions: Vec::new(),
                    activation_restrictions: Vec::new(),
                    mana_output: None,
                    activation_condition: None,
                    mana_usage_restrictions: vec![],
                    is_loyalty_ability: false,
                },
            ),
            functional_zones: vec![Zone::Battlefield],
        }
        .into(),
        text: Some(text),
        effects_ast: Some(effects_ast),
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    }))
}

pub(super) fn parse_fixed_word(word: &str) -> Option<u32> {
    leaf::parse_leaf_number_prefix_words(&[word])?
        .into_fixed()
        .map(|(value, _)| value)
}

pub(super) fn first_kind_after(
    tokens: &[OwnedLexToken],
    start: usize,
    kind: TokenKind,
) -> Option<usize> {
    tokens
        .iter()
        .enumerate()
        .skip(start)
        .find_map(|(index, token)| (token.kind == kind).then_some(index))
}

pub(super) fn trim_edge_commas(mut tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    while tokens.first().is_some_and(OwnedLexToken::is_comma) {
        tokens = &tokens[1..];
    }
    while tokens.last().is_some_and(OwnedLexToken::is_comma) {
        tokens = &tokens[..tokens.len() - 1];
    }
    tokens
}

pub(super) fn unsupported_escape(words: &[&str]) -> CardTextError {
    CardTextError::ParseError(format!(
        "unsupported escape clause tail (clause: '{}')",
        words.join(" ")
    ))
}
