use super::*;

pub(super) fn find_no_defender_phrase(
    tokens: &[OwnedLexToken],
    with_condition_tail: bool,
) -> Option<(usize, usize)> {
    const BASE_IT: &[&[&str]] = &[
        &[
            "can", "attack", "as", "though", "it", "didnt", "have", "defender",
        ],
        &[
            "can", "attack", "as", "though", "it", "didn't", "have", "defender",
        ],
    ];
    const BASE_THEY: &[&[&str]] = &[
        &[
            "can", "attack", "as", "though", "they", "didnt", "have", "defender",
        ],
        &[
            "can", "attack", "as", "though", "they", "didn't", "have", "defender",
        ],
    ];
    const CONDITIONAL: &[&[&str]] = &[
        &[
            "can", "attack", "as", "though", "it", "didnt", "have", "defender", "as", "long", "as",
        ],
        &[
            "can", "attack", "as", "though", "it", "didn't", "have", "defender", "as", "long", "as",
        ],
    ];
    let phrases = if with_condition_tail {
        CONDITIONAL
    } else {
        BASE_IT
    };
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    loop {
        let start = initial_len.saturating_sub(input.len());
        let mut candidate = input.clone();
        if primitives::any_phrase(phrases)
            .parse_next(&mut candidate)
            .is_ok()
        {
            return Some((start, initial_len.saturating_sub(candidate.len())));
        }
        if !with_condition_tail {
            let mut plural_candidate = input.clone();
            if primitives::any_phrase(BASE_THEY)
                .parse_next(&mut plural_candidate)
                .is_ok()
            {
                return Some((start, initial_len.saturating_sub(plural_candidate.len())));
            }
        }
        take_token(&mut input).ok()?;
    }
}
