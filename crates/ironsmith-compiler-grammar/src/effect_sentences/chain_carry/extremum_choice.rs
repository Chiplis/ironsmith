use super::*;
use crate::cards::builders::DamageActionAst;

fn tie_axis(sentence: &[OwnedLexToken]) -> Option<(&'static str, bool, &str)> {
    let words = crate::lexer::parser_token_word_refs(sentence);
    let rest = words.strip_prefix(&["if", "two", "or", "more"])?;
    let noun = *rest.first()?;
    if !matches!(noun, "creatures" | "cards" | "permanents") {
        return None;
    }
    let rest = rest.get(1..)?.strip_prefix(&["are", "tied", "for"])?;
    let greatest = match *rest.first()? {
        "greatest" => true,
        "least" | "lowest" => false,
        _ => return None,
    };
    let rest = rest.get(1..)?;
    let (axis, rest) = if let Some(rest) = rest.strip_prefix(&["power"]) {
        ("power", rest)
    } else if let Some(rest) = rest.strip_prefix(&["toughness"]) {
        ("toughness", rest)
    } else if let Some(rest) = rest.strip_prefix(&["mana", "value"]) {
        ("mana value", rest)
    } else {
        return None;
    };
    (rest == ["you", "choose", "one", "of", "them"]).then_some((axis, greatest, noun))
}
fn action_target(effect: &mut EffectAst) -> Option<&mut TargetAst> {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect else {
        return None;
    };
    match action {
        SubjectVerbActionAst::ZoneMoves(
            ZoneMoveActionAst::Destroy { target, .. }
            | ZoneMoveActionAst::ReturnToHand { target, .. }
            | ZoneMoveActionAst::MoveToZone { target, .. },
        ) => Some(target),
        SubjectVerbActionAst::Damage(
            DamageActionAst::DealDamage { target, .. }
            | DamageActionAst::DealDamageEqualToPower { target, .. },
        ) => Some(target),
        _ => None,
    }
}
/// The tie sentence clarifies the preceding singular operation's choice; it
/// is not a second operation after destroying/returning/damaging every tie.
/// Match the typed extremum and reject targeted, counted, or unrelated actions.
pub(crate) fn bind(effects: &mut Vec<EffectAst>, sentence: &[OwnedLexToken]) -> bool {
    let Some((axis, greatest, noun)) = tie_axis(sentence) else {
        return false;
    };
    let Some(mut action) = effects.last().cloned() else {
        return false;
    };
    let Some(target) = action_target(&mut action) else {
        return false;
    };
    let TargetAst::Object(filter, None, _) = target else {
        return false;
    };
    if filter.set_quantifier_surface().is_some() {
        return false;
    }
    if noun == "creatures" && !filter.card_types.contains(&CardType::Creature) {
        return false;
    }
    if noun == "cards" && (filter.zone.is_none() || filter.zone == Some(Zone::Battlefield)) {
        return false;
    }
    let comparison = match axis {
        "power" => filter.power.as_ref(),
        "toughness" => filter.toughness.as_ref(),
        _ => filter.mana_value.as_ref(),
    };
    let Some(crate::filter::Comparison::EqualExpr(value)) = comparison else {
        return false;
    };
    let matches = match (axis, greatest, value.unhinted()) {
        ("power", true, Value::GreatestPower(_))
        | ("power", false, Value::LeastPower(_))
        | ("toughness", true, Value::GreatestToughness(_))
        | ("toughness", false, Value::LeastToughness(_))
        | ("mana value", true, Value::GreatestManaValue(_))
        | ("mana value", false, Value::LeastManaValue(_)) => true,
        _ => false,
    };
    if !matches {
        return false;
    }
    let mut filter = filter.clone();
    let slot = match axis {
        "power" => &mut filter.power,
        "toughness" => &mut filter.toughness,
        _ => &mut filter.mana_value,
    };
    if let Some(crate::filter::Comparison::EqualExpr(value)) = slot {
        **value = value
            .as_ref()
            .clone()
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::ExtremumTiedForCharacteristic);
    }
    let tag = crate::util::helper_tag_for_tokens(sentence, "extremum_choice");
    *target = TargetAst::Tagged(tag.clone(), None);
    let choose = EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
        filter,
        count: ChoiceCount::exactly(1),
        count_value: None,
        player: PlayerAst::You,
        tag,
    });
    *effects.last_mut().expect("previous action checked") = EffectAst::Sequence {
        effects: vec![choose, action],
    };
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tie_clarification_chooses_exactly_one_before_the_original_nonregenerating_destroy() {
        let tokens=crate::lexer::lex_line("Destroy the creature with the least power. It can't be regenerated. If two or more creatures are tied for least power, you choose one of them.",0).unwrap();
        let (parsed, loss) = ironsmith_compiler::parse_loss::capture(|| {
            crate::effect_sentences::parse_effect_sentences_lexed(&tokens)
        });
        let parsed = parsed.unwrap();
        assert!(!loss.is_lossy(), "{}", loss.reasons_text());
        let debug = format!("{parsed:#?}");
        assert!(debug.contains("ChooseObjects"));
        assert!(debug.contains("no_regeneration: true"));
        assert!(
            !debug.contains("if_true"),
            "the clarification is not an additive post-destruction conditional"
        );
    }
    #[test]
    fn unrelated_targeted_or_wrong_axis_riders_remain_unclaimed() {
        let mut filter = ObjectFilter::creature();
        filter.power = Some(crate::filter::Comparison::EqualExpr(Box::new(
            Value::LeastPower(ObjectFilter::creature()),
        )));
        let source = EffectAst::subject_verb_destroy(TargetAst::Object(filter.clone(), None, None));
        let mut effects = vec![source.clone()];
        let wrong = crate::lexer::lex_line(
            "If two or more creatures are tied for least toughness, you choose one of them.",
            0,
        )
        .unwrap();
        assert!(!bind(&mut effects, &wrong));
        assert_eq!(effects, vec![source]);
        let targeted = crate::util::parse_target_phrase(
            &crate::lexer::lex_line("target creature with the least power", 0).unwrap(),
        )
        .unwrap();
        let mut effects = vec![EffectAst::subject_verb_destroy(targeted)];
        let before = effects.clone();
        let matching = crate::lexer::lex_line(
            "If two or more creatures are tied for least power, you choose one of them.",
            0,
        )
        .unwrap();
        assert!(!bind(&mut effects, &matching));
        assert_eq!(effects, before);
    }
}
