use crate::diagnostics::TextSpan;
use crate::model::ClauseVerbAst;
use crate::recognition::{ParseDiagnostic, ParseExpectation, ParseOutcome, RuleId};
use crate::front_end::lexer::{OwnedLexToken, TokenWordView};

const TYPED_CLAUSE_HEAD_RULE: RuleId = RuleId::new("typed-effect-clause-head");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ClauseActorHeadAst {
    Implicit,
    Controller,
    Player,
    Object,
    Iterated,
    Reference,
    Structural,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ClauseHeadFormAst {
    Action(ClauseVerbAst),
    Conditional,
    Iteration,
    Permission,
    Repetition,
    Structural,
}

/// The only lexical head classification used by effect-sentence registries.
///
/// It borrows the normalized token spellings so registries can use their
/// existing single/pair indexes without re-tokenizing or rescanning a whole
/// sentence for every candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TypedClauseHeadAst<'a> {
    pub first_word: &'a str,
    pub second_word: Option<&'a str>,
    pub actor: ClauseActorHeadAst,
    pub form: ClauseHeadFormAst,
    pub span: Option<TextSpan>,
}

impl TypedClauseHeadAst<'_> {
    pub(crate) fn permits_action_fallback(self) -> bool {
        matches!(
            self.form,
            ClauseHeadFormAst::Action(_) | ClauseHeadFormAst::Structural
        )
    }
}

pub(crate) fn classify_typed_clause_head<'a>(
    tokens: &'a [OwnedLexToken],
) -> ParseOutcome<TypedClauseHeadAst<'a>> {
    let words = TokenWordView::new(tokens).to_word_refs();
    let Some(first_word) = words.first().copied() else {
        return ParseOutcome::NoMatch;
    };
    let second_word = words.get(1).copied();
    let span = clause_span(tokens);
    let actor = classify_actor(first_word, second_word);

    if let Some(action) = words.iter().find_map(|word| classify_action(word)) {
        return ParseOutcome::matched(
            TypedClauseHeadAst {
                first_word,
                second_word,
                actor,
                form: ClauseHeadFormAst::Action(action),
                span,
            },
            span,
        );
    }

    if words.iter().any(|word| is_structural_action(word)) {
        return ParseOutcome::matched(
            TypedClauseHeadAst {
                first_word,
                second_word,
                actor,
                form: ClauseHeadFormAst::Structural,
                span,
            },
            span,
        );
    }

    let form = match first_word {
        "if" | "unless" | "when" | "whenever" | "at" => ClauseHeadFormAst::Conditional,
        "for" | "each" => ClauseHeadFormAst::Iteration,
        "may" => ClauseHeadFormAst::Permission,
        "repeat" => ClauseHeadFormAst::Repetition,
        "x" | "the" | "a" | "an" | "all" | "and" | "then" | "there" | "until" | "mana" | "roll" => {
            ClauseHeadFormAst::Structural
        }
        _ if actor_commits_to_action(actor) => {
            return ParseOutcome::Error(ParseDiagnostic::malformed(
                TYPED_CLAUSE_HEAD_RULE,
                span,
                [ParseExpectation::new("effect action")],
                format!("recognized {actor:?} clause head but found no supported action"),
            ));
        }
        _ => return ParseOutcome::NoMatch,
    };

    ParseOutcome::matched(
        TypedClauseHeadAst {
            first_word,
            second_word,
            actor,
            form,
            span,
        },
        span,
    )
}

fn classify_actor(first: &str, second: Option<&str>) -> ClauseActorHeadAst {
    match (first, second) {
        ("you", _) | ("your", _) => ClauseActorHeadAst::Controller,
        ("target", Some("player" | "opponent"))
        | ("that", Some("player" | "opponent"))
        | ("each", Some("player" | "opponent"))
        | ("an", Some("opponent")) => ClauseActorHeadAst::Player,
        ("opponent", _) => ClauseActorHeadAst::Player,
        ("each" | "for", _) => ClauseActorHeadAst::Iterated,
        (
            "it" | "it's" | "it’s" | "its" | "they" | "they're" | "they’re" | "theyre" | "those"
            | "them" | "that",
            _,
        ) => ClauseActorHeadAst::Reference,
        ("target" | "this" | "enchanted" | "equipped" | "creatures", _) => {
            ClauseActorHeadAst::Object
        }
        ("if" | "unless" | "when" | "whenever" | "at" | "repeat", _) => {
            ClauseActorHeadAst::Structural
        }
        _ if classify_action(first).is_some() => ClauseActorHeadAst::Implicit,
        _ => ClauseActorHeadAst::Structural,
    }
}

fn actor_commits_to_action(actor: ClauseActorHeadAst) -> bool {
    matches!(
        actor,
        ClauseActorHeadAst::Controller
            | ClauseActorHeadAst::Player
            | ClauseActorHeadAst::Object
            | ClauseActorHeadAst::Iterated
            | ClauseActorHeadAst::Reference
    )
}

fn classify_action(word: &str) -> Option<ClauseVerbAst> {
    Some(match word {
        "add" | "adds" => ClauseVerbAst::Add,
        "attach" | "attaches" => ClauseVerbAst::Attach,
        "become" | "becomes" | "is" | "are" | "it's" | "it’s" | "they're" | "they’re"
        | "theyre" | "isnt" | "isn't" | "arent" | "aren't" | "get" | "gets" => {
            ClauseVerbAst::Become
        }
        "cast" | "casts" => ClauseVerbAst::Cast,
        "choose" | "chooses" => ClauseVerbAst::Choose,
        "control" | "controls" => ClauseVerbAst::Control,
        "copy" | "copies" => ClauseVerbAst::Copy,
        "counter" | "counters" => ClauseVerbAst::Counter,
        "create" | "creates" => ClauseVerbAst::Create,
        "damage" | "deal" | "deals" => ClauseVerbAst::DealDamage,
        "destroy" | "destroys" => ClauseVerbAst::Destroy,
        "discard" | "discards" => ClauseVerbAst::Discard,
        "draw" | "draws" => ClauseVerbAst::Draw,
        "exchange" | "exchanges" => ClauseVerbAst::Exchange,
        "exile" | "exiles" => ClauseVerbAst::Exile,
        "fight" | "fights" => ClauseVerbAst::Fight,
        "gain" | "gains" | "has" | "have" => ClauseVerbAst::Gain,
        "give" | "gives" => ClauseVerbAst::Give,
        "look" | "looks" => ClauseVerbAst::Look,
        "lose" | "loses" => ClauseVerbAst::Lose,
        "mill" | "mills" => ClauseVerbAst::Mill,
        "move" | "moves" => ClauseVerbAst::Move,
        "pay" | "pays" => ClauseVerbAst::Pay,
        "play" | "plays" => ClauseVerbAst::Play,
        "prevent" | "prevents" | "cant" | "can't" | "cannot" => ClauseVerbAst::Prevent,
        "put" | "puts" => ClauseVerbAst::Put,
        "remove" | "removes" => ClauseVerbAst::Remove,
        "return" | "returns" => ClauseVerbAst::Return,
        "reveal" | "reveals" => ClauseVerbAst::Reveal,
        "sacrifice" | "sacrifices" => ClauseVerbAst::Sacrifice,
        "search" | "searches" => ClauseVerbAst::Search,
        "shuffle" | "shuffles" => ClauseVerbAst::Shuffle,
        "tap" | "taps" => ClauseVerbAst::Tap,
        "transform" | "transforms" | "convert" | "converts" => ClauseVerbAst::Transform,
        "untap" | "untaps" => ClauseVerbAst::Untap,
        _ => return None,
    })
}

fn is_structural_action(word: &str) -> bool {
    matches!(
        word,
        "adapt"
            | "amass"
            | "assemble"
            | "bolster"
            | "clash"
            | "cloak"
            | "connive"
            | "detain"
            | "discover"
            | "distribute"
            | "double"
            | "end"
            | "endure"
            | "exert"
            | "explore"
            | "goad"
            | "incubate"
            | "investigate"
            | "learn"
            | "manifest"
            | "open"
            | "populate"
            | "proliferate"
            | "regenerate"
            | "reorder"
            | "roll"
            | "scry"
            | "skip"
            | "support"
            | "surveil"
            | "suspect"
            | "venture"
    )
}

fn clause_span(tokens: &[OwnedLexToken]) -> Option<TextSpan> {
    let first = tokens.first()?;
    let last = tokens.last()?;
    (first.span.line == last.span.line).then_some(TextSpan {
        line: first.span.line,
        start: first.span.start,
        end: last.span.end,
    })
}
