//! "If <event> would happen, <program> instead." (CR 614.1a): a damage,
//! life-change, destruction or zone-change event replaced by an arbitrary
//! effect program, which runs with
//! the replaced event as its context ("that much", "that many", "that
//! player"). Event modifications (amount changes, redirection, prevention)
//! keep their own readers; this reader declines any body that names them.

use super::*;
use ironsmith_core::ReplacedEventSpec;

/// Body words that belong to a modification or prevention of the event
/// rather than to a replacement program, owned by the specialized readers.
const MODIFICATION_WORDS: &[&str] = &["prevent", "double", "twice", "plus", "may"];

/// Additional modification words for the event kinds that own them: an
/// amount change of the same damage ("deals that much damage plus 1") or of
/// the same life gain ("gain twice that much life").
fn event_modification_words(event: &ReplacedEventSpec) -> &'static [&'static str] {
    match event {
        ReplacedEventSpec::DamageToPlayer { .. } | ReplacedEventSpec::DamageToObject { .. } => {
            &["damage"]
        }
        ReplacedEventSpec::LifeGain { .. } => &["gain", "gains"],
        ReplacedEventSpec::LifeLoss { .. } => &["lose", "loses"],
        // Regeneration is its own destruction replacement (CR 701.19).
        ReplacedEventSpec::Destroy { .. } => &["regenerate"],
        ReplacedEventSpec::ZoneChange { .. } => &[],
        ReplacedEventSpec::DrawInstruction { .. } => &[],
        ReplacedEventSpec::SourceDestructionRegenerates => &[],
        ReplacedEventSpec::Untap { .. } => &[],
        ReplacedEventSpec::LifeReducedToZeroOrLess { .. } => &[],
    }
}

/// An object event subject: the source, or an object description
/// ("enchanted land", "a creature you control that's enchanted").
fn object_subject(header: &[OwnedLexToken], subject: &[&str]) -> Option<ObjectFilter> {
    if let Some(filter) = source_subject(subject) {
        return Some(filter);
    }
    // The subject words are the header tokens after "if"; header words and
    // tokens coincide (the header holds only word tokens).
    let subject_tokens = header.get(1..1 + subject.len())?;
    if words_of(subject_tokens) != subject {
        return None;
    }
    let filter = crate::object_filters::parse_object_filter_lexed(subject_tokens, false).ok()?;
    (filter != ObjectFilter::default()).then_some(filter)
}

fn words_of(tokens: &[OwnedLexToken]) -> Vec<&str> {
    crate::lexer::token_word_refs(tokens)
}

/// The players an event subject names.
fn player_subject(words: &[&str]) -> Option<PlayerFilter> {
    match words {
        ["you"] => Some(PlayerFilter::You),
        ["a", "player"] => Some(PlayerFilter::Any),
        ["an", "opponent"] => Some(PlayerFilter::Opponent),
        _ => None,
    }
}

/// The source of the ability ("this creature", a normalized self-name).
fn source_subject(words: &[&str]) -> Option<ObjectFilter> {
    if !crate::util::is_source_reference_words(words) {
        return None;
    }
    let mut filter = ObjectFilter::source();
    filter.source_surface = crate::util::source_reference_surface_for_words(words);
    Some(filter)
}

/// The watched event, read from the clause between "if" and the comma.
fn replaced_event(header: &[OwnedLexToken]) -> Option<ReplacedEventSpec> {
    let words = words_of(header);
    let ["if", rest @ ..] = words.as_slice() else {
        return None;
    };
    let would = rest.iter().position(|word| *word == "would")?;
    let (subject, predicate) = (&rest[..would], &rest[would + 1..]);
    match predicate {
        // "If damage would be dealt to you", "If combat damage would be dealt
        // to this creature".
        ["be", "dealt", "to", recipient @ ..] => {
            let combat_only = match subject {
                ["damage"] => false,
                ["combat", "damage"] => true,
                _ => return None,
            };
            if let Some(player) = player_subject(recipient) {
                return Some(ReplacedEventSpec::DamageToPlayer {
                    player,
                    source_filter: None,
                    combat_only,
                });
            }
            Some(ReplacedEventSpec::DamageToObject {
                target: source_subject(recipient)?,
                source_filter: None,
                combat_only,
            })
        }
        // "If this creature would be dealt damage".
        ["be", "dealt", "damage"] | ["be", "dealt", "combat", "damage"] => {
            Some(ReplacedEventSpec::DamageToObject {
                target: source_subject(subject)?,
                source_filter: None,
                combat_only: predicate.len() == 4,
            })
        }
        // "If Szadek would deal combat damage to a player", "If a Zombie you
        // control would deal combat damage to a player".
        ["deal", "damage", "to", recipient @ ..]
        | ["deal", "combat", "damage", "to", recipient @ ..] => {
            let combat_only = predicate.get(1) == Some(&"combat");
            let player = player_subject(recipient)?;
            let source_filter = match source_subject(subject) {
                Some(filter) => filter,
                None => {
                    // The subject words are the header tokens after "if";
                    // header words and tokens coincide (no punctuation).
                    let subject_tokens = header.get(1..1 + subject.len())?;
                    if words_of(subject_tokens) != subject {
                        return None;
                    }
                    let filter =
                        crate::object_filters::parse_object_filter_lexed(subject_tokens, false)
                            .ok()?;
                    if filter == ObjectFilter::default() {
                        return None;
                    }
                    filter
                }
            };
            Some(ReplacedEventSpec::DamageToPlayer {
                player,
                source_filter: Some(source_filter),
                combat_only,
            })
        }
        // "If an opponent would draw two or more cards" (CR 121.2): the whole
        // draw instruction.
        ["draw", count, "or", "more", "cards"] => {
            let minimum = crate::util::parse_number_word_u32(count)?;
            (minimum >= 2).then_some(ReplacedEventSpec::DrawInstruction {
                player: player_subject(subject)?,
                minimum,
            })
        }
        // "If a permanent with a wind counter on it would untap during its
        // controller's untap step" (CR 502.3).
        ["untap", rest @ ..] => {
            let during_controllers_untap_step = match rest {
                [] => false,
                ["during", "its", "controller's" | "controllers", "untap", "step"]
                | ["during", "your", "untap", "step"] => true,
                _ => return None,
            };
            Some(ReplacedEventSpec::Untap {
                object: object_subject(header, subject)?,
                during_controllers_untap_step,
            })
        }
        // "If an opponent would gain life".
        ["gain", "life"] => Some(ReplacedEventSpec::LifeGain {
            player: player_subject(subject)?,
            from_own_spell_or_ability: false,
        }),
        // "If a spell or ability would cause its controller to gain life"
        // (Rain of Gore): the gain must be that spell or ability's effect,
        // gained by its own controller. "That player" is the gaining player.
        ["cause", "its", "controller", "to", "gain", "life"]
            if matches!(subject, ["a", "spell", "or", "ability"]) =>
        {
            Some(ReplacedEventSpec::LifeGain {
                player: PlayerFilter::Any,
                from_own_spell_or_ability: true,
            })
        }
        // "If your life total would be reduced to 0 or less" (Enduring
        // Angel, CR 119.3).
        ["be", "reduced", "to", "0", "or", "less"] => {
            let player = match subject {
                ["your", "life", "total"] => PlayerFilter::You,
                ["a", "player's" | "players", "life", "total"] => PlayerFilter::Any,
                ["an", "opponent's" | "opponents", "life", "total"] => PlayerFilter::Opponent,
                _ => return None,
            };
            Some(ReplacedEventSpec::LifeReducedToZeroOrLess { player })
        }
        // "If you would lose life".
        ["lose", "life"] => Some(ReplacedEventSpec::LifeLoss {
            player: player_subject(subject)?,
        }),
        // "If enchanted land would be destroyed" (CR 701.8).
        ["be", "destroyed"] => Some(ReplacedEventSpec::Destroy {
            target: object_subject(header, subject)?,
        }),
        // "If this creature would die": put into a graveyard from the
        // battlefield (CR 700.4).
        ["leave", "the", "battlefield"] => Some(ReplacedEventSpec::ZoneChange {
            object: object_subject(header, subject)?,
            from: Some(Zone::Battlefield),
            to: None,
        }),
        ["die"] => Some(ReplacedEventSpec::ZoneChange {
            object: object_subject(header, subject)?,
            from: Some(Zone::Battlefield),
            to: Some(Zone::Graveyard),
        }),
        // "If this would be put into a graveyard from the battlefield", "...
        // from anywhere".
        ["be", "put", "into", "a", "graveyard", origin @ ..] => {
            let from = match origin {
                [] | ["from", "anywhere"] => None,
                ["from", "the", "battlefield"] => Some(Zone::Battlefield),
                _ => return None,
            };
            Some(ReplacedEventSpec::ZoneChange {
                object: object_subject(header, subject)?,
                from,
                to: Some(Zone::Graveyard),
            })
        }
        // "If a card would be put into enchanted player's graveyard from
        // anywhere" (Wheel of Sun and Moon), "... into an opponent's
        // graveyard": the graveyard's owner is the moving card's owner.
        ["be", "put", "into", owner @ .., "graveyard", "from", "anywhere"]
        | ["be", "put", "into", owner @ .., "graveyard"]
            if !owner.is_empty() && !matches!(owner, ["a"]) =>
        {
            let owner = match owner {
                ["your"] => PlayerFilter::You,
                ["an", "opponent's" | "opponents"] => PlayerFilter::Opponent,
                ["enchanted", "player's" | "players"] => PlayerFilter::TaggedPlayer(
                    (crate::tag::CompilerReferenceTag::Enchanted.bind()).into(),
                ),
                _ => return None,
            };
            let mut object = object_subject(header, subject)?;
            object.owner = Some(owner);
            Some(ReplacedEventSpec::ZoneChange {
                object,
                from: None,
                to: Some(Zone::Graveyard),
            })
        }
        _ => None,
    }
}

/// "instead that card is revealed and put on the bottom of that player's
/// library" (Wheel of Sun and Moon): the passive zone-change payload, read as
/// the active "reveal it and put it on the bottom of its owner's library".
/// The replaced card is going to its owner's graveyard, so "that player" is
/// that card's owner.
fn rewrite_passive_zone_change_body(body: &mut Vec<OwnedLexToken>) {
    let words: Vec<String> = words_of(body).iter().map(|word| word.to_string()).collect();
    let words: Vec<&str> = words.iter().map(String::as_str).collect();
    let reference_len = match words.as_slice() {
        ["that", "card", "is", ..] => 2,
        ["it", "is", ..] => 1,
        _ => return,
    };
    let after = &words[reference_len + 1..];
    let (mut rewritten, rest_start) = match after {
        ["revealed", "and", "put", ..] => (
            vec!["reveal", "it", "and", "put", "it"],
            reference_len + 1 + 3,
        ),
        ["put", ..] => (vec!["put", "it"], reference_len + 1 + 1),
        _ => return,
    };
    let mut rest: Vec<&str> = words[rest_start..].to_vec();
    if let Some(index) = rest
        .windows(2)
        .position(|pair| pair == ["that", "player's"] || pair == ["that", "players"])
    {
        rest.splice(index..index + 2, ["its", "owner's"]).for_each(drop);
    }
    rewritten.extend(rest);
    let trailing_period = body.last().is_some_and(|token| token.is_period());
    *body = crate::lexer::synthetic_word_tokens(rewritten.iter().copied());
    if trailing_period {
        body.push(OwnedLexToken::period(crate::diagnostics::TextSpan::synthetic()));
    }
}

/// "<program>. Then if this creature didn't transform this way, <fallback>."
/// (Enduring Angel): the fallback runs exactly when the program's own
/// transform of the source did not happen, so the transform is lowered as the
/// producer of an `IfEffectDidNotHappen` gate rather than read as a state
/// predicate about the creature.
fn parse_did_not_transform_this_way_program(
    body: &[OwnedLexToken],
) -> Option<Vec<crate::cards::builders::EffectAst>> {
    use crate::cards::builders::{
        ConditionalEffectAst, EffectAst, PermanentStateActionAst, SubjectVerbActionAst,
        SubjectVerbEffectAst,
    };
    let period = body.iter().position(|token| token.is_period())?;
    let second = crate::util::trim_edge_punctuation_tokens(&body[period + 1..]);
    let comma = second.iter().position(|token| token.is_comma())?;
    let predicate = words_of(&second[..comma]).join(" ").replace('\'', "");
    let subject = predicate
        .strip_prefix("then if ")
        .or_else(|| predicate.strip_prefix("if "))?
        .strip_suffix(" didnt transform this way")
        .or_else(|| {
            predicate
                .strip_prefix("then if ")
                .or_else(|| predicate.strip_prefix("if "))?
                .strip_suffix(" did not transform this way")
        })?;
    let subject_words: Vec<&str> = subject.split(' ').collect();
    if !(crate::util::is_source_reference_words(&subject_words) || subject_words == ["it"]) {
        return None;
    }
    let fallback_tokens = crate::util::trim_edge_punctuation_tokens(&second[comma + 1..]);
    if fallback_tokens.is_empty() {
        return None;
    }
    let mut effects =
        crate::clause_support::parse_effect_sentences_lexed(&body[..=period]).ok()?;
    let otherwise = crate::clause_support::parse_effect_sentences_lexed(fallback_tokens).ok()?;
    if otherwise.is_empty() {
        return None;
    }
    let is_source_transform = |effect: &EffectAst| match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Transform { target, .. }),
            ..
        }) => match target {
            crate::cards::builders::TargetAst::Source(_) => true,
            crate::cards::builders::TargetAst::Object(filter, ..) => filter.source,
            _ => false,
        },
        _ => false,
    };
    let index = effects.iter().position(is_source_transform)?;
    if effects.iter().filter(|effect| is_source_transform(*effect)).count() != 1 {
        return None;
    }
    let transform = effects.remove(index);
    effects.insert(
        index,
        EffectAst::Conditionals(ConditionalEffectAst::IfEffectDidNotHappen {
            effect: Box::new(transform),
            otherwise,
        }),
    );
    Some(effects)
}

pub fn parse_if_event_would_happen_instead_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let Some(comma) = tokens.iter().position(|token| token.is_comma()) else {
        return Ok(None);
    };
    let header = &tokens[..comma];
    if header.iter().any(|token| token.as_word().is_none()) {
        return Ok(None);
    }
    let Some(event) = replaced_event(header) else {
        return Ok(None);
    };
    // "If this creature would be destroyed, regenerate it.": regeneration is
    // the destruction replacement itself (CR 701.19a), not a shield created
    // after the destruction was replaced.
    if let ReplacedEventSpec::Destroy { target } = &event
        && target.source
        && words_of(crate::util::trim_edge_punctuation_tokens(&tokens[comma + 1..]))
            == ["regenerate", "it"]
    {
        return Ok(Some(StaticAbility::event_replacement_with_effects(
            ReplacedEventSpec::SourceDestructionRegenerates,
            Vec::new(),
            crate::lexer::render_token_slice(tokens),
        )));
    }
    let mut body: Vec<OwnedLexToken> = tokens[comma + 1..].to_vec();
    // Exactly one "instead": leading the program, or closing its first
    // sentence ("..., exile that many cards from your graveyard instead. If
    // you can't, you lose the game.").
    let leading = body.first().is_some_and(|token| token.is_word("instead"));
    if leading {
        body.remove(0);
    }
    let sentence_end = body
        .iter()
        .position(|token| token.is_period())
        .unwrap_or(body.len());
    let trailing = sentence_end > 0 && body[sentence_end - 1].is_word("instead");
    if leading == trailing {
        return Ok(None);
    }
    if trailing {
        body.remove(sentence_end - 1);
    }
    if leading && matches!(event, ReplacedEventSpec::ZoneChange { .. }) {
        rewrite_passive_zone_change_body(&mut body);
    }
    // "If this permanent would be put into a graveyard, you may put it on top
    // of its owner's library instead." (Pulmonic Sliver's granted ability):
    // the permanent's controller may apply the replacement; declined, the
    // permanent goes to the graveyard (CR 614.1a, 616.1). Only a source's own
    // zone change reads this way, so "you" is the moving permanent's
    // controller.
    let optional = matches!(
        &event,
        ReplacedEventSpec::ZoneChange { object, .. } if object.source
    ) && body.len() > 2
        && body[0].is_word("you")
        && body[1].is_word("may");
    if optional {
        body.remove(0);
        body.remove(0);
    }
    let modification_words = event_modification_words(&event);
    if body.iter().any(|token| token.is_word("instead") || token.is_quote())
        || words_of(&body).iter().any(|word| {
            MODIFICATION_WORDS.contains(word) || modification_words.contains(word)
        })
    {
        return Ok(None);
    }
    // The exile-instead readers own "exile it instead" for dying objects and
    // graveyard-bound cards; keep one owner per line.
    if matches!(event, ReplacedEventSpec::ZoneChange { .. })
        && (is_shuffle_into_library_from_graveyard_line_lexed(tokens)
            || matches!(parse_exile_would_die_instead_line(tokens), Ok(Some(_)))
            || matches!(parse_exile_to_exile_instead_of_graveyard_line(tokens), Ok(Some(_)))
            || matches!(
                parse_exile_to_countered_exile_instead_of_graveyard_line(tokens),
                Ok(Some(_))
            ))
    {
        return Ok(None);
    }
    // "+1/+1 counters on it instead" for the source is the specialized
    // put-counter replacement's reading; keep a single owner.
    if matches!(event, ReplacedEventSpec::DamageToObject { .. })
        && words_of(&body).contains(&"+1/+1")
    {
        return Ok(None);
    }
    // "put that many -1/-1 counters on it instead" (Lichenthrope): when the
    // damaged permanent is the source itself, "it" is that source.
    if let ReplacedEventSpec::DamageToObject { target, .. } = &event
        && target.source
        && let Some(last) = body
            .iter()
            .rposition(|token| !token.is_period())
        && body[last].is_word("it")
    {
        let replacement = crate::lexer::synthetic_word_tokens(["this", "permanent"]);
        body.splice(last..=last, replacement);
    }
    let body = crate::util::trim_edge_punctuation_tokens(&body);
    if body.is_empty() {
        return Ok(None);
    }
    // A program this reader cannot read stays unclaimed rather than adding a
    // diagnostic to lines other readers own.
    let effects = if let Some(effects) = parse_did_not_transform_this_way_program(body) {
        effects
    } else {
        let Ok(effects) = crate::clause_support::parse_effect_sentences_lexed(body) else {
            return Ok(None);
        };
        effects
    };
    if effects.is_empty() {
        return Ok(None);
    }
    let display = crate::lexer::render_token_slice(tokens);
    Ok(Some(if optional {
        StaticAbility::optional_event_replacement_with_effects(event, effects, display)
    } else {
        StaticAbility::event_replacement_with_effects(event, effects, display)
    }))
}
