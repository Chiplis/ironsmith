use super::*;

pub fn parse_dealt_damage_by_source_subject_words(
    words: &[&str],
) -> Option<DealtDamageBySourceSubject> {
    use ironsmith_core::DamagedBySource;

    let alternatives: &[(&[&str], DamagedBySource)] = &[
        (
            &["dealt", "damage", "by", "this", "creature", "this", "turn"],
            DamagedBySource::ThisCreature,
        ),
        (
            &[
                "that", "was", "dealt", "damage", "by", "this", "creature", "this", "turn",
            ],
            DamagedBySource::ThisCreature,
        ),
        (
            &[
                "that", "were", "dealt", "damage", "by", "this", "creature", "this", "turn",
            ],
            DamagedBySource::ThisCreature,
        ),
        (
            &[
                "dealt", "damage", "by", "equipped", "creature", "this", "turn",
            ],
            DamagedBySource::EquippedCreature,
        ),
        (
            &[
                "that", "was", "dealt", "damage", "by", "equipped", "creature", "this", "turn",
            ],
            DamagedBySource::EquippedCreature,
        ),
        (
            &[
                "that", "were", "dealt", "damage", "by", "equipped", "creature", "this", "turn",
            ],
            DamagedBySource::EquippedCreature,
        ),
        (
            &[
                "dealt",
                "damage",
                "by",
                "enchanted",
                "creature",
                "this",
                "turn",
            ],
            DamagedBySource::EnchantedCreature,
        ),
        (
            &[
                "that",
                "was",
                "dealt",
                "damage",
                "by",
                "enchanted",
                "creature",
                "this",
                "turn",
            ],
            DamagedBySource::EnchantedCreature,
        ),
        (
            &[
                "that",
                "were",
                "dealt",
                "damage",
                "by",
                "enchanted",
                "creature",
                "this",
                "turn",
            ],
            DamagedBySource::EnchantedCreature,
        ),
    ];

    alternatives.iter().find_map(|(suffix, damager)| {
        let base_word_count = words.len().checked_sub(suffix.len())?;
        let matched =
            base_word_count > 0 && crate::word_primitives::parse_sequence_suffix(words, suffix);
        if matched {
            // The damage-history suffix ("dealt damage by this creature this
            // turn") read whole.
            crate::word_claims::claim_words(&words[base_word_count..]);
        }
        matched.then_some(DealtDamageBySourceSubject {
                base_word_count,
                damager: *damager,
            })
    })
}
