//! Audition (custom CR 701.101) is a named action with a resolution-time menu.
use crate::cards::builders::{CardTextError, ChooseOneModeAst, EffectAst, KeywordActionAst, ObjectChoiceEffectAst, PlayerAst, SubjectAst, SubjectVerbActionAst, SubjectVerbRoleAst, TargetAst};
use crate::color::ColorSet;
use crate::effect::{ChoiceCount, Value};
use crate::grammar::token_definitions::{BuiltinTokenShape, TokenDefinitionSpec, TokenDescriptionTextRoles, fixed_creature};
use crate::lexer::OwnedLexToken;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::{CardType, Subtype};

fn roles(color: &OwnedLexToken) -> Option<[BuiltinTokenShape; 2]> {
    use BuiltinTokenShape::{ClericRole, ScoutRole, WarlockRole, WarriorRole, WizardRole};
    Some(match color.parser_text() {
        "white" => [ClericRole, ScoutRole],
        "blue" => [WarlockRole, WizardRole],
        "black" => [ClericRole, WarlockRole],
        "red" => [WarriorRole, WizardRole],
        "green" => [ScoutRole, WarriorRole],
        _ => return None,
    })
}

pub(super) fn parse(tokens: &[OwnedLexToken], subject: Option<SubjectAst>) -> Result<EffectAst, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let operand = match tokens {
        [rest @ .., last] if last.is_word("again") => rest,
        _ => tokens,
    };
    let invalid = || CardTextError::ParseError(format!("unsupported audition colors '{}'", crate::lexer::render_token_slice(tokens)));
    let mut roles = match operand {
        [for_word, color] if for_word.is_word("for") => roles(color).ok_or_else(invalid)?.to_vec(),
        [for_word, first, or, second_for, second] if for_word.is_word("for") && or.is_word("or") && second_for.is_word("for") => {
            let mut options = roles(first).ok_or_else(invalid)?.to_vec();
            options.extend(roles(second).ok_or_else(invalid)?);
            options
        }
        _ => return Err(invalid()),
    };
    roles.sort_by_key(|role| role.fixed_name());
    roles.dedup();

    let actor = ironsmith_core::custom_subtypes::declare_type_line_subtype("actor", ironsmith_core::SubtypeFamily::Creature);
    let name = if actor.is_some() { "Construct Actor" } else { "Construct" };
    let subtypes = std::iter::once(Subtype::Construct).chain(actor).collect();
    let definition = fixed_creature(name, subtypes, ColorSet::COLORLESS, (1, 1),
        Some(TokenDescriptionTextRoles::authored(ironsmith_core::TokenNameTextRole::SubtypeDerived)));
    let mode = |name: String, definition, attachment: Option<TargetAst>| {
        let description = if attachment.is_some() {
            format!("create a {name} token attached to a creature you control")
        } else { format!("create a {name} token") };
        ChooseOneModeAst { description, effects: vec![super::simple_token_creation(name, definition, Value::Fixed(1), PlayerAst::Implicit, false, attachment)] }
    };
    let mut modes = vec![mode("1/1 colorless Construct Actor creature".into(), definition, None)];
    let mut host = ObjectFilter::creature().you_control();
    host.set_explicit_card_type_noun(Some(CardType::Creature));
    for role in roles {
        let name = role.fixed_name().expect("audition Roles have a fixed name");
        let target = TargetAst::WithCount(Box::new(TargetAst::Object(host.clone(), None, None)), ChoiceCount::exactly(1));
        modes.push(mode(format!("{name} Role"), TokenDefinitionSpec::Builtin(role), Some(target)));
    }
    let menu = EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseOneOf { chooser: PlayerFilter::You, modes });
    crate::word_claims::claim_tokens(tokens);
    Ok(EffectAst::subject_verb(SubjectVerbRoleAst::Actor, super::clause_pattern_helpers::extract_subject_player(subject).unwrap_or(PlayerAst::Implicit),
        SubjectVerbActionAst::KeywordActions(KeywordActionAst::PerformNamedAction {
            label: "audition".into(), program_completion: ironsmith_core::effect::NamedActionProgramCompletion::ProgramCompleted,
            performer: None, objects: None, prior_objects: None, program: Some(vec![menu]),
        })))
}
