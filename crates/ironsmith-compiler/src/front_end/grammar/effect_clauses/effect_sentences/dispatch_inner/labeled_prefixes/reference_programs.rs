use super::*;

pub fn parse_subject_verb_extension_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    macro_rules! one {
        ($route:literal, $parser:expr) => {{
            if let Some(effect) = $parser? {
                crate::parse_trace::event(concat!("effect-route: subject-verb ", $route));
                return Ok(Some(vec![effect]));
            }
        }};
    }
    macro_rules! many {
        ($route:literal, $parser:expr) => {{
            if let Some(effects) = $parser? {
                crate::parse_trace::event(concat!("effect-route: subject-verb ", $route));
                return Ok(Some(effects));
            }
        }};
    }

    one!(
        "verb=Take subject=implicit recognizer=extra-turn-after-anchor",
        parse_take_extra_turn_sentence(tokens)
    );
    one!(
        "verb=Prevent subject=implicit recognizer=damage-prevention",
        parse_prevent_damage_sentence(tokens)
    );
    one!(
        "verb=Monstrosity subject=implicit recognizer=keyword-action",
        parse_monstrosity_sentence(tokens)
    );
    many!(
        "verb=Earthbend subject=implicit recognizer=keyword-action",
        parse_earthbend_subject_verb_sentence(tokens)
    );
    one!(
        "verb=Enchant subject=implicit recognizer=aura-attachment",
        super::super::search_library::parse_enchant_sentence(tokens)
    );
    one!(
        "verb=Play subject=explicit recognizer=zone-permission",
        parse_play_permission_subject_verb(tokens)
    );
    one!(
        "verb=Exile subject=implicit recognizer=instead-replacement",
        parse_zone_replacement_subject_verb(tokens)
    );
    many!(
        "verb=Is subject=explicit recognizer=passive-color-type-addition",
        parse_passive_color_type_addition_sentence(tokens)
    );
    many!(
        "verb=When subject=implicit recognizer=delayed-trigger-this-turn",
        parse_sentence_delayed_trigger_this_turn(tokens)
    );
    one!(
        "verb=Deal subject=triggering-spell recognizer=spell-count-opponent-damage",
        parse_triggered_spell_opponent_damage_subject_verb(tokens)
    );
    one!(
        "verb=Choose subject=explicit recognizer=choice-complement-sacrifice",
        parse_choice_complement_subject_verb(tokens)
    );
    many!(
        "verb=Gain subject=explicit recognizer=life-equal-stat",
        parse_gain_life_equal_to_power_sentence(tokens)
    );
    one!(
        "verb=Get subject=explicit recognizer=last-effect-counter-loop",
        parse_for_each_counter_removed_sentence(tokens)
    );
    many!(
        "verb=Exile subject=explicit recognizer=exile-return-same-object",
        parse_exile_then_return_same_object_sentence(tokens)
    );
    if let Some(effects) =
        super::super::chain_carry::parse_return_it_then_loses_all_abilities_lexed(tokens)?
    {
        crate::parse_trace::event(
            "effect-route: subject-verb verb=Return subject=explicit recognizer=return-then-lose-abilities",
        );
        return Ok(Some(effects));
    }
    let ability_candidates =
        effect_grammar::labeled_dispatch::parse_ability_candidate_shape(tokens);
    if ability_candidates.simple_source_gain {
        many!(
            "verb=Gain subject=implicit recognizer=source-ability-grant",
            parse_gain_ability_to_source_subject_verb_sentence(tokens)
        );
    }
    if ability_candidates.simple_gain {
        many!(
            "verb=Gain subject=explicit recognizer=ability-grant",
            parse_gain_ability_subject_verb_sentence(tokens)
        );
    }
    many!(
        "verb=Choose subject=explicit recognizer=opponent-decline-loop",
        parse_for_each_opponent_doesnt_subject_verb_sentence(tokens)
    );
    many!(
        "verb=Vote subject=explicit recognizer=vote-affinity",
        parse_vote_affinity_subject_verb(tokens)
    );
    one!(
        "verb=Vote subject=explicit recognizer=vote-procedure",
        parse_vote_subject_verb(tokens)
    );

    Ok(None)
}

pub(super) fn parse_earthbend_subject_verb_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(earthbend) = super::super::search_library::parse_earthbend_sentence(tokens)? else {
        return Ok(None);
    };

    let Some((_, used)) = parse_number(&tokens[1..]) else {
        return Ok(Some(vec![earthbend]));
    };
    let mut tail = trim_commas(&tokens[1 + used..]).to_vec();
    while token_slice_first_is(&tail, "then") {
        tail.remove(0);
    }
    if tail.is_empty() {
        return Ok(Some(vec![earthbend]));
    }

    let mut effects = vec![earthbend];
    if token_slice_first_is(&tail, "earthbend")
        && let Some(mut tail_effects) = parse_earthbend_subject_verb_sentence(&tail)?
    {
        effects.append(&mut tail_effects);
        return Ok(Some(effects));
    }
    effects.extend(parse_effect_chain_lexed(&tail)?);
    Ok(Some(effects))
}

pub(super) fn parse_gain_ability_to_source_subject_verb_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(
        super::super::gain_ability::parse_gain_ability_to_source_sentence(tokens)?
            .map(|effect| vec![effect]),
    )
}

pub(super) fn parse_gain_ability_subject_verb_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    super::super::gain_ability::parse_gain_ability_sentence(tokens)
}

pub(super) fn parse_for_each_opponent_doesnt_subject_verb_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(
        super::super::conditionals::parse_for_each_opponent_doesnt(tokens)?
            .map(|effect| vec![effect]),
    )
}
