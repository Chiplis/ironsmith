use super::*;

pub(super) fn try_parse_level_header_block(
    preprocessed: &PreprocessedDocument,
    idx: usize,
    line: &PreprocessedLine,
    allow_unsupported: bool,
) -> Result<Option<(RewriteLineCst, usize)>, CardTextError> {
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
        match parse_level_item_cst(next_line) {
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
                RewriteLineCst::Unsupported(UnsupportedLineCst {
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
        RewriteLineCst::LevelHeader(LevelHeaderCst {
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
) -> Result<Option<(RewriteLineCst, usize)>, CardTextError> {
    let mut bullet_modes = Vec::new();
    let mut probe_idx = idx + 1;
    while let Some(PreprocessedItem::Line(next_line)) = preprocessed.items.get(probe_idx) {
        if !is_bullet_line(next_line.info.raw_line.as_str()) {
            break;
        }
        bullet_modes.push(parse_modal_mode_cst(next_line)?);
        probe_idx += 1;
    }

    if bullet_modes.is_empty() {
        return Ok(None);
    }

    Ok(Some((
        RewriteLineCst::Modal(ModalBlockCst {
            header: line.info.clone(),
            modes: bullet_modes,
        }),
        probe_idx,
    )))
}
