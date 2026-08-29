use super::*;

pub(super) fn try_parse_level_header_block(
    preprocessed: &PreprocessedDocument,
    idx: usize,
    line: &PreprocessedLine,
    allow_unsupported: bool,
) -> Result<Option<(RecognizedLine, usize)>, CardTextError> {
    let Some((min_level, max_level)) = parse_level_header(&line.info.normalized.normalized) else {
        return Ok(None);
    };

    let mut pt = None;
    let mut items = Vec::new();
    let mut probe_idx = idx + 1;
    while let Some(PreprocessedItem::Line(next_line)) = preprocessed.items.get(probe_idx) {
        if parse_level_header(&next_line.info.normalized.normalized).is_some() {
            break;
        }
        if parse_saga_chapter_prefix(&next_line.info.normalized.normalized).is_some() {
            break;
        }
        if let Some(parsed_pt) = parse_power_toughness(&next_line.info.normalized.normalized)
            && let (PtValue::Fixed(power), PtValue::Fixed(toughness)) =
                (parsed_pt.power, parsed_pt.toughness)
        {
            pt = Some((power, toughness));
            probe_idx += 1;
            continue;
        }
        match recognize_level_item(&preprocessed.builder, next_line) {
            Ok(Some(item)) => {
                items.push(item);
                probe_idx += 1;
            }
            Ok(None) => {
                if allow_unsupported {
                    break;
                }
                return Err(CardTextError::ParseError(format!(
                    "unsupported level ability line: '{}'",
                    next_line.info.raw_line
                )));
            }
            Err(_) if allow_unsupported => break,
            Err(err) => return Err(err),
        }
    }

    if pt.is_none() && items.is_empty() && preprocessed.items.get(idx + 1).is_some() {
        if allow_unsupported {
            return Ok(Some((
                RecognizedLine::Unsupported(RecognizedUnsupportedLine {
                    info: line.info.clone(),
                    reason_code: "level-header-not-yet-supported",
                }),
                idx + 1,
            )));
        }
        return Err(CardTextError::ParseError(format!(
            "parser does not yet support level header: '{}'",
            line.info.raw_line
        )));
    }

    Ok(Some((
        RecognizedLine::LevelHeader(RecognizedLevelHeader {
            min_level,
            max_level,
            pt,
            items,
        }),
        probe_idx,
    )))
}

pub(super) fn try_parse_modal_bullet_block(
    preprocessed: &PreprocessedDocument,
    idx: usize,
    line: &PreprocessedLine,
) -> Result<Option<(RecognizedLine, usize)>, CardTextError> {
    if is_bullet_line(line) {
        return Ok(None);
    }
    if is_named_option_as_enters_choice_header(line) {
        return Ok(None);
    }

    let normalized_header = line.info.raw_line.trim_start().to_ascii_lowercase();
    let spree_header = crate::string_primitives::starts_with(&normalized_header, "spree");
    let next_line_is_mode = preprocessed
        .items
        .get(idx + 1)
        .is_some_and(|item| match item {
            PreprocessedItem::Line(next_line) => {
                is_bullet_line(next_line)
                    || (spree_header
                        && next_line
                            .tokens
                            .first()
                            .is_some_and(|token| token.kind == TokenKind::Plus))
            }
            PreprocessedItem::Metadata(_) => false,
        });
    if !next_line_is_mode {
        return Ok(None);
    }

    // Parsing a modal header can probe a common action suffix. Only perform
    // that work after the following physical line proves this is a modal
    // block; ordinary multi-sentence abilities may contain the same choice
    // words and internal commas but have no bullet modes to inherit a suffix.
    let header_has_common_target_suffix =
        super::super::modal_support::parse_modal_header(&line.info, &line.tokens)?
            .is_some_and(|header| !header.common_suffix_effects_ast.is_empty());
    let mut bullet_modes = Vec::new();
    let mut probe_idx = idx + 1;
    while let Some(PreprocessedItem::Line(next_line)) = preprocessed.items.get(probe_idx) {
        let is_spree_mode = spree_header
            && next_line
                .tokens
                .first()
                .is_some_and(|token| token.kind == TokenKind::Plus);
        if !is_bullet_line(next_line) && !is_spree_mode {
            break;
        }
        bullet_modes.push(recognize_modal_mode(
            next_line,
            header_has_common_target_suffix,
        )?);
        probe_idx += 1;
    }

    if bullet_modes.is_empty() {
        return Ok(None);
    }

    Ok(Some((
        RecognizedLine::Modal(RecognizedModalBlock {
            header: line.info.clone(),
            header_tokens: line.tokens.clone(),
            modes: bullet_modes,
        }),
        probe_idx,
    )))
}
