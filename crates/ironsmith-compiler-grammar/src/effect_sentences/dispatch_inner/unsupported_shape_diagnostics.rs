use crate::cards::builders::LifeResourceActionAst;
pub fn parse_gain_life_equal_to_power_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::parse_gain_life_equal_power_tokens(tokens) else {
        return Ok(None);
    };
    let subject = if !shape.subject_tokens.is_empty() {
        Some(parse_subject(shape.subject_tokens))
    } else {
        None
    };
    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    let amount = Value::PowerOf(Box::new(ChooseSpec::Tagged((crate::tag::CompilerReferenceTag::It.bind()).into())));
    Ok(Some(vec![EffectAst::subject_verb(
        SubjectVerbRoleAst::AffectedPlayer,
        player,
        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife { amount }),
    )]))
}

pub fn parse_prevent_damage_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if let Some(effect) = effect_grammar::parse_prevent_damage_sentence_lexed(tokens)? {
        return Ok(Some(effect));
    }
    parse_prevent_all_damage_clause(tokens)
}

pub fn parse_gain_x_plus_life_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::parse_gain_x_plus_life_tokens(tokens) else {
        return Ok(None);
    };
    let player = match parse_subject(shape.subject_tokens) {
        SubjectAst::Player(player) => player,
        _ => PlayerAst::Implicit,
    };

    let trailing_tokens = trim_commas(shape.trailing_tokens);
    let x_value = if trailing_tokens.is_empty() {
        Value::X
    } else if let Some(where_x) = parse_value_binding_clause(&trailing_tokens) {
        where_x
    } else {
        return Err(CardTextError::ParseError(format!(
            "unsupported gain-x-plus-life trailing clause (clause: '{}')",
            render_token_slice(tokens)
        )));
    };
    let amount = Value::Add(
        Box::new(x_value),
        Box::new(Value::Fixed(shape.bonus as i32)),
    );
    let effects = vec![EffectAst::subject_verb(
        SubjectVerbRoleAst::AffectedPlayer,
        player,
        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife { amount }),
    )];

    Ok(Some(effects))
}
