use super::*;

/// Combined-axis maxima and per-controller populations retain their complete
/// scope. Both have existing executable Value reductions; no display string
/// supplies the result or changes the player's choice semantics.
pub(super) fn parse(words: &[&str]) -> Option<(Value, usize)> {
    let offset = usize::from(words.first() == Some(&"the"));
    let body = words.get(offset..)?;
    for prefix in [
        &["greatest", "power", "and", "or", "toughness", "among"][..],
        &["greatest", "power", "or", "toughness", "among"][..],
    ] {
        if let Some(scope) = body.strip_prefix(prefix) {
            if scope.is_empty() {
                return None;
            }
            let filter = parse_object_filter_words(scope, false).ok()?;
            let power = Value::GreatestPower(filter.clone());
            let toughness = Value::GreatestToughness(filter);
            // The existing numeric adapter recognizes this exact maximum
            // algebra before its intermediate addition can overflow.
            let minimum = Value::Min(Box::new(power.clone()), Box::new(toughness.clone()));
            let maximum = Value::Add(
                Box::new(Value::Add(Box::new(power), Box::new(toughness))),
                Box::new(Value::Scaled(Box::new(minimum), -1)),
            )
            .with_surface_hint(ValueSurfaceHint::WhicheverIsGreater);
            return Some((maximum, words.len()));
        }
    }
    if let Some(scope) = body.strip_prefix(&["greatest", "number", "of"]) {
        let objects = scope
            .strip_suffix(&["a", "player", "controls"])
            .or_else(|| scope.strip_suffix(&["any", "player", "controls"]))?;
        let mut filter = parse_object_filter_words(objects, false).ok()?;
        if filter.controller.is_some() {
            return None;
        }
        // This exact suffix supplies the partition. Existing specialized
        // "shared creature type" subset reductions keep their own readers.
        filter.controller = Some(PlayerFilter::Any);
        return Some((Value::GreatestCount(filter), words.len()));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn combined_axes_and_per_controller_counts_are_real_typed_reductions() {
        let words = "the greatest power and or toughness among other creatures you control"
            .split_whitespace()
            .collect::<Vec<_>>();
        let (value, used) = parse(&words).unwrap();
        assert_eq!(used, words.len());
        let Value::Add(sum, negative_minimum) = value.unhinted() else {
            panic!("{value:?}")
        };
        assert!(matches!(sum.as_ref(),Value::Add(power,toughness)
            if matches!(power.as_ref(),Value::GreatestPower(filter) if filter.other && filter.controller==Some(PlayerFilter::You))
            && matches!(toughness.as_ref(),Value::GreatestToughness(filter) if filter.other && filter.controller==Some(PlayerFilter::You))));
        assert!(
            matches!(negative_minimum.as_ref(),Value::Scaled(minimum,-1) if matches!(minimum.as_ref(),Value::Min(_, _)))
        );
        let words = "the greatest number of creatures a player controls"
            .split_whitespace()
            .collect::<Vec<_>>();
        let (value, used) = parse(&words).unwrap();
        assert_eq!(used, words.len());
        assert!(
            matches!(value,Value::GreatestCount(filter) if filter.controller==Some(PlayerFilter::Any))
        );
        assert!(parse(&["the", "greatest", "number", "of", "creatures"]).is_none());
    }
}

#[cfg(test)]
mod target_player_extremum_composition_tests {
    use super::*;
    #[test]
    fn plural_target_players_keep_the_authored_count_and_caster_controlled_amount() {
        let tokens=crate::lexer::lex_line("Any number of target players each discard a number of cards equal to the greatest mana value among permanents you control.",0).unwrap();
        let (parsed, loss) = ironsmith_compiler::parse_loss::capture(|| {
            crate::effect_sentences::parse_effect_sentence_lexed(&tokens)
        });
        let parsed = parsed.unwrap();
        assert!(!loss.is_lossy(), "{}", loss.reasons_text());
        let [
            crate::cards::builders::EffectAst::ForEach(
                crate::cards::builders::ForEachEffectAst::ForEachTargetPlayers {
                    count,
                    effects,
                    ..
                },
            ),
        ] = parsed.as_slice()
        else {
            panic!("{parsed:?}")
        };
        assert_eq!(count.min, 0);
        assert_eq!(count.max, None);
        let [
            crate::cards::builders::EffectAst::SubjectVerb(
                crate::cards::builders::SubjectVerbEffectAst {
                    subject,
                    action:
                        crate::cards::builders::SubjectVerbActionAst::ZoneMoves(
                            crate::cards::builders::ZoneMoveActionAst::Discard { count, .. },
                        ),
                },
            ),
        ] = effects.as_slice()
        else {
            panic!("{effects:#?}")
        };
        assert_eq!(subject.player, crate::cards::builders::PlayerAst::That);
        assert!(
            matches!(count.unhinted(),Value::GreatestManaValue(filter) if filter.controller==Some(PlayerFilter::You))
        );
    }
}
