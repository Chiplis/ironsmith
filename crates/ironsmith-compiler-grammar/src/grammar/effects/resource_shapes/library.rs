use super::*;

pub fn parse_resource_shuffle_shape(
    tokens: &[OwnedLexToken],
    default_player: PlayerAst,
) -> Option<ResourceShuffleShape> {
    let clause = trimmed(tokens);
    if let Some((into_idx, (), after_into)) =
        primitives::find_prefix(clause, || primitives::kw("into").void())
    {
        let target = trimmed(&clause[..into_idx]);
        let normalized_destination = without_articles(trimmed(after_into));
        if exact_unit(target, tagged_reference)
            && let Some((destination_player, rest)) =
                primitives::parse_prefix(&normalized_destination, destination)
            && supported_source_tail(trimmed(rest))
        {
            return Some(ResourceShuffleShape::TaggedIntoLibrary {
                player: resolve_destination(destination_player, default_player),
                to_bottom: false,
            });
        }
        if consult_remainder(target)
            && let Some((destination_player, rest)) =
                primitives::parse_prefix(&normalized_destination, destination)
            && supported_source_tail(trimmed(rest))
        {
            return Some(ResourceShuffleShape::ShuffleLibrary {
                player: resolve_destination(destination_player, default_player),
            });
        }
    }

    if matches!(default_player, PlayerAst::ItsOwner)
        && exact_unit(clause, tagged_into_their_library)
    {
        return Some(ResourceShuffleShape::TaggedIntoLibrary {
            player: PlayerAst::ItsOwner,
            to_bottom: true,
        });
    }
    if required_shuffle_markers(clause) {
        return None;
    }

    let normalized = without_articles(clause);
    let (destination_player, rest) = primitives::parse_prefix(&normalized, destination)?;
    if !trimmed(rest).is_empty() {
        return None;
    }
    let _ = destination_player;
    Some(ResourceShuffleShape::SimpleLibrary)
}
