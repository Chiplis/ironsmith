use crate::runtime_backend::grammar::permission_shapes;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CantPattern {
    IfPrefix,
    MaxSpeedAttackOrBlock,
    DirectTemporaryCastRestriction,
    OpponentsCantCastSpellsWith,
    OpponentsCantBlockWithCreaturesWith,
    EvenCountersOnItSuffix,
    ThisCantAttackOrBlockUnlessPrefix,
    ThisCreatureCantAttackOrBlockUnlessPrefix,
    ThisSelfCantAttackOrBlockUnlessPrefix,
    IfSourceDoubleManaValueInstead,
    IfPlayerWouldGainNoLifeInstead,
    PlayersCantGainLife,
    PlayersCantSearchLibraries,
    DamageCantBePrevented,
    YouCantLoseGame,
    OpponentsCantWinGame,
    YourLifeTotalCantChange,
    OpponentsCantCastSpells,
    OpponentsCantDrawExtraCards,
    CantHaveCountersPlaced,
    ThisSpellCantBeCountered,
    SourceCantAttack,
    SourceCantBlock,
    SourceCantAttackItsOwner,
    PermanentsYouControlCantBeSacrificed,
    SourceCantBeBlocked,
    TemporaryUnblockable,
    SourceCantAttackAlone,
    SourceCantAttackOrBlock,
    SourceCantAttackOrBlockAlone,
    LoseUnspentManaAsStepsEnd,
    LoseThisManaAsStepsEnd,
    AttackOrBlockTailPrefix,
    CantRestrictionOrTail,
    ThisCantAttackPrefix,
    ThisCantAttackUnlessPrefix,
    ThisCreatureCantAttackUnlessPrefix,
    ThisSelfCantAttackUnlessPrefix,
    CastCreatureSpellThisTurnUnlessSuffix,
    CastNoncreatureSpellThisTurnUnlessSuffix,
    CollectiveRestraintAttackTax,
    CantBeBlockedByPrefix,
    ThisCreatureCantBeBlockedByPrefix,
    ThisCantBeBlockedByPrefix,
    CantBeBlockedExceptByPrefix,
    ThisCreatureCantBeBlockedExceptByPrefix,
    ThisCantBeBlockedExceptByPrefix,
    WithPowerPrefix,
    WithFlying,
    YouControlPrefix,
    ControlMoreCreaturesThanDefendingPlayer,
    ControlMoreLandsThanDefendingPlayer,
    ControlAnotherCreatureWithPowerPrefix,
    ControlACreatureWithPowerPrefix,
    CardsInYourGraveyard,
    IslandsOnBattlefield,
    CardsInTheirGraveyard,
    CardsInExile,
    MountainOnBattlefield,
    DefendingPlayerPoisoned,
    DefendingPlayerControlsEnchantment,
    OtherCreaturesAttack,
    CreatureWithGreaterPowerAlsoAttacks,
    BlackOrGreenCreatureAlsoAttacks,
    OpponentDealtDamageThisTurn,
    SacrificeLandAttackCost,
    ReturnEnchantmentAttackCost,
    PayPerPlusOneCounterAttackCost,
}

pub(crate) fn parse_cant_pattern(words: &[&str], pattern: CantPattern) -> Option<CantPattern> {
    use CantPattern::*;
    let recognized = match pattern {
        IfPrefix => prefix(words, &["if"]),
        MaxSpeedAttackOrBlock => exact_any(
            words,
            &[
                &[
                    "this", "cant", "attack", "or", "block", "unless", "you", "have", "max",
                    "speed",
                ],
                &[
                    "this", "creature", "cant", "attack", "or", "block", "unless", "you", "have",
                    "max", "speed",
                ],
            ],
        ),
        DirectTemporaryCastRestriction => {
            prefix_any(
                words,
                &[
                    &["your", "opponents", "cant", "cast"],
                    &["each", "opponent", "cant", "cast"],
                    &["each", "player", "cant", "cast"],
                    &["players", "cant", "cast"],
                    &["target", "player", "cant", "cast"],
                    &["you", "cant", "cast"],
                ],
            ) && contains_all(words, &["this", "turn"])
        }
        OpponentsCantCastSpellsWith => prefix(
            words,
            &["your", "opponents", "cant", "cast", "spells", "with"],
        ),
        OpponentsCantBlockWithCreaturesWith => prefix(
            words,
            &[
                "your",
                "opponents",
                "cant",
                "block",
                "with",
                "creatures",
                "with",
            ],
        ),
        EvenCountersOnItSuffix => suffix(words, &["even", "number", "of", "counters", "on", "it"]),
        ThisCantAttackOrBlockUnlessPrefix => prefix_any(
            words,
            &[
                &[
                    "this", "creature", "cant", "attack", "or", "block", "unless",
                ],
                &["this", "cant", "attack", "or", "block", "unless"],
            ],
        ),
        ThisCreatureCantAttackOrBlockUnlessPrefix => prefix(
            words,
            &[
                "this", "creature", "cant", "attack", "or", "block", "unless",
            ],
        ),
        ThisSelfCantAttackOrBlockUnlessPrefix => {
            prefix(words, &["this", "cant", "attack", "or", "block", "unless"])
        }
        IfSourceDoubleManaValueInstead => {
            prefix(words, &["if", "source", "you", "control", "with"])
                && suffix(words, &["instead"])
                && contains_all(words, &["mana", "value", "double"])
        }
        IfPlayerWouldGainNoLifeInstead => exact_any(
            words,
            &[
                &[
                    "if", "a", "player", "would", "gain", "life", "that", "player", "gains", "no",
                    "life", "instead",
                ],
                &[
                    "if", "a", "player", "would", "gain", "life", "they", "gain", "no", "life",
                    "instead",
                ],
            ],
        ),
        PlayersCantGainLife => exact(words, &["players", "cant", "gain", "life"]),
        PlayersCantSearchLibraries => exact(words, &["players", "cant", "search", "libraries"]),
        DamageCantBePrevented => exact(words, &["damage", "cant", "be", "prevented"]),
        YouCantLoseGame => exact(words, &["you", "cant", "lose", "the", "game"]),
        OpponentsCantWinGame => exact(words, &["your", "opponents", "cant", "win", "the", "game"]),
        YourLifeTotalCantChange => exact(words, &["your", "life", "total", "cant", "change"]),
        OpponentsCantCastSpells => exact(words, &["your", "opponents", "cant", "cast", "spells"]),
        OpponentsCantDrawExtraCards => exact(
            words,
            &[
                "your",
                "opponents",
                "cant",
                "draw",
                "more",
                "than",
                "one",
                "card",
                "each",
                "turn",
            ],
        ),
        CantHaveCountersPlaced => exact(
            words,
            &["counters", "cant", "be", "put", "on", "this", "permanent"],
        ),
        ThisSpellCantBeCountered => exact(words, &["this", "spell", "cant", "be", "countered"]),
        SourceCantAttack => exact_any(
            words,
            &[
                &["this", "creature", "cant", "attack"],
                &["this", "token", "cant", "attack"],
                &["this", "cant", "attack"],
            ],
        ),
        SourceCantBlock => exact_any(
            words,
            &[
                &["this", "creature", "cant", "block"],
                &["this", "token", "cant", "block"],
                &["this", "cant", "block"],
            ],
        ),
        SourceCantAttackItsOwner => exact(
            words,
            &["this", "creature", "cant", "attack", "its", "owner"],
        ),
        PermanentsYouControlCantBeSacrificed => exact(
            words,
            &["permanents", "you", "control", "cant", "be", "sacrificed"],
        ),
        SourceCantBeBlocked => exact_any(
            words,
            &[
                &["this", "creature", "cant", "be", "blocked"],
                &["this", "token", "cant", "be", "blocked"],
                &["this", "cant", "be", "blocked"],
                &["cant", "be", "blocked"],
            ],
        ),
        TemporaryUnblockable => exact_any(
            words,
            &[
                &["this", "creature", "cant", "be", "blocked", "this", "turn"],
                &["this", "cant", "be", "blocked", "this", "turn"],
                &["cant", "be", "blocked", "this", "turn"],
            ],
        ),
        SourceCantAttackAlone => exact_any(
            words,
            &[
                &["this", "creature", "cant", "attack", "alone"],
                &["this", "token", "cant", "attack", "alone"],
                &["this", "cant", "attack", "alone"],
            ],
        ),
        SourceCantAttackOrBlock => exact_any(
            words,
            &[
                &["this", "creature", "cant", "attack", "or", "block"],
                &["this", "token", "cant", "attack", "or", "block"],
                &["this", "cant", "attack", "or", "block"],
            ],
        ),
        SourceCantAttackOrBlockAlone => exact_any(
            words,
            &[
                &["this", "creature", "cant", "attack", "or", "block", "alone"],
                &["this", "token", "cant", "attack", "or", "block", "alone"],
                &["this", "cant", "attack", "or", "block", "alone"],
            ],
        ),
        LoseUnspentManaAsStepsEnd => {
            prefix(words, &["lose", "unspent"]) && contains(words, &["mana", "as", "steps"])
        }
        LoseThisManaAsStepsEnd => prefix(words, &["lose", "this", "mana", "as", "steps"]),
        AttackOrBlockTailPrefix => prefix(words, &["attack", "or", "block"]),
        CantRestrictionOrTail => prefix_any(
            words,
            &[&["cast"], &["activate"], &["attack"], &["block"], &["be"]],
        ),
        ThisCantAttackPrefix => prefix_any(
            words,
            &[
                &["this", "creature", "cant", "attack"],
                &["this", "cant", "attack"],
            ],
        ),
        ThisCantAttackUnlessPrefix => prefix_any(
            words,
            &[
                &["this", "creature", "cant", "attack", "unless"],
                &["this", "cant", "attack", "unless"],
            ],
        ),
        ThisCreatureCantAttackUnlessPrefix => {
            prefix(words, &["this", "creature", "cant", "attack", "unless"])
        }
        ThisSelfCantAttackUnlessPrefix => prefix(words, &["this", "cant", "attack", "unless"]),
        CastCreatureSpellThisTurnUnlessSuffix => suffix_any(
            words,
            &[
                &[
                    "unless", "youve", "cast", "a", "creature", "spell", "this", "turn",
                ],
                &[
                    "unless", "you", "ve", "cast", "a", "creature", "spell", "this", "turn",
                ],
                &[
                    "unless", "youve", "cast", "creature", "spell", "this", "turn",
                ],
                &[
                    "unless", "you", "ve", "cast", "creature", "spell", "this", "turn",
                ],
            ],
        ),
        CastNoncreatureSpellThisTurnUnlessSuffix => suffix_any(
            words,
            &[
                &[
                    "unless",
                    "youve",
                    "cast",
                    "a",
                    "noncreature",
                    "spell",
                    "this",
                    "turn",
                ],
                &[
                    "unless",
                    "you",
                    "ve",
                    "cast",
                    "a",
                    "noncreature",
                    "spell",
                    "this",
                    "turn",
                ],
                &[
                    "unless",
                    "youve",
                    "cast",
                    "noncreature",
                    "spell",
                    "this",
                    "turn",
                ],
                &[
                    "unless",
                    "you",
                    "ve",
                    "cast",
                    "noncreature",
                    "spell",
                    "this",
                    "turn",
                ],
            ],
        ),
        CollectiveRestraintAttackTax => {
            prefix(
                words,
                &[
                    "creatures",
                    "cant",
                    "attack",
                    "you",
                    "unless",
                    "their",
                    "controller",
                    "pays",
                    "x",
                    "for",
                    "each",
                    "creature",
                    "they",
                    "control",
                    "thats",
                    "attacking",
                    "you",
                ],
            ) && suffix_any(
                words,
                &[
                    &[
                        "where", "x", "is", "the", "number", "of", "basic", "land", "types",
                        "among", "lands", "you", "control",
                    ],
                    &[
                        "where", "x", "is", "the", "number", "of", "basic", "land", "type",
                        "among", "lands", "you", "control",
                    ],
                ],
            )
        }
        CantBeBlockedByPrefix => prefix_any(
            words,
            &[
                &["this", "creature", "cant", "be", "blocked", "by"],
                &["this", "cant", "be", "blocked", "by"],
                &["cant", "be", "blocked", "by"],
            ],
        ),
        ThisCreatureCantBeBlockedByPrefix => {
            prefix(words, &["this", "creature", "cant", "be", "blocked", "by"])
        }
        ThisCantBeBlockedByPrefix => prefix(words, &["this", "cant", "be", "blocked", "by"]),
        CantBeBlockedExceptByPrefix => prefix_any(
            words,
            &[
                &["this", "creature", "cant", "be", "blocked", "except", "by"],
                &["this", "cant", "be", "blocked", "except", "by"],
                &["cant", "be", "blocked", "except", "by"],
            ],
        ),
        ThisCreatureCantBeBlockedExceptByPrefix => prefix(
            words,
            &["this", "creature", "cant", "be", "blocked", "except", "by"],
        ),
        ThisCantBeBlockedExceptByPrefix => {
            prefix(words, &["this", "cant", "be", "blocked", "except", "by"])
        }
        WithPowerPrefix => prefix(words, &["with", "power"]),
        WithFlying => exact(words, &["with", "flying"]),
        YouControlPrefix => prefix(words, &["you", "control"]),
        ControlMoreCreaturesThanDefendingPlayer => exact(
            words,
            &[
                "you",
                "control",
                "more",
                "creatures",
                "than",
                "defending",
                "player",
            ],
        ),
        ControlMoreLandsThanDefendingPlayer => exact(
            words,
            &[
                "you",
                "control",
                "more",
                "lands",
                "than",
                "defending",
                "player",
            ],
        ),
        ControlAnotherCreatureWithPowerPrefix => prefix(
            words,
            &["you", "control", "another", "creature", "with", "power"],
        ),
        ControlACreatureWithPowerPrefix => {
            prefix(words, &["you", "control", "a", "creature", "with", "power"])
        }
        CardsInYourGraveyard => exact(words, &["cards", "in", "your", "graveyard"]),
        IslandsOnBattlefield => exact_any(
            words,
            &[
                &["islands", "on", "the", "battlefield"],
                &["islands", "on", "battlefield"],
            ],
        ),
        CardsInTheirGraveyard => exact(words, &["cards", "in", "their", "graveyard"]),
        CardsInExile => exact(words, &["cards", "in", "exile"]),
        MountainOnBattlefield => exact_any(
            words,
            &[
                &["there", "is", "a", "mountain", "on", "the", "battlefield"],
                &["there", "is", "a", "mountain", "on", "battlefield"],
                &["there", "is", "mountain", "on", "battlefield"],
            ],
        ),
        DefendingPlayerPoisoned => exact(words, &["defending", "player", "is", "poisoned"]),
        DefendingPlayerControlsEnchantment => exact_any(
            words,
            &[
                &[
                    "defending",
                    "player",
                    "controls",
                    "an",
                    "enchantment",
                    "or",
                    "an",
                    "enchanted",
                    "permanent",
                ],
                &[
                    "defending",
                    "player",
                    "controls",
                    "enchantment",
                    "or",
                    "enchanted",
                    "permanent",
                ],
            ],
        ),
        OtherCreaturesAttack => exact(words, &["other", "creatures", "attack"]),
        CreatureWithGreaterPowerAlsoAttacks => exact(
            words,
            &[
                "a", "creature", "with", "greater", "power", "also", "attacks",
            ],
        ),
        BlackOrGreenCreatureAlsoAttacks => exact(
            words,
            &["a", "black", "or", "green", "creature", "also", "attacks"],
        ),
        OpponentDealtDamageThisTurn => exact(
            words,
            &[
                "an", "opponent", "has", "been", "dealt", "damage", "this", "turn",
            ],
        ),
        SacrificeLandAttackCost => exact_any(
            words,
            &[
                &["you", "sacrifice", "a", "land"],
                &["you", "sacrifice", "land"],
            ],
        ),
        ReturnEnchantmentAttackCost => exact_any(
            words,
            &[
                &[
                    "you",
                    "return",
                    "an",
                    "enchantment",
                    "you",
                    "control",
                    "to",
                    "its",
                    "owners",
                    "hand",
                ],
                &[
                    "you",
                    "return",
                    "enchantment",
                    "you",
                    "control",
                    "to",
                    "its",
                    "owners",
                    "hand",
                ],
                &[
                    "you",
                    "return",
                    "an",
                    "enchantment",
                    "you",
                    "control",
                    "to",
                    "its",
                    "owner",
                    "s",
                    "hand",
                ],
            ],
        ),
        PayPerPlusOneCounterAttackCost => exact_any(
            words,
            &[
                &[
                    "you", "pay", "1", "for", "each", "+1/+1", "counter", "on", "it",
                ],
                &[
                    "you", "pay", "1", "for", "each", "1/1", "counter", "on", "it",
                ],
            ],
        ),
    };
    recognized.then_some(pattern)
}

pub(crate) fn matches_cant_pattern(words: &[&str], pattern: CantPattern) -> bool {
    parse_cant_pattern(words, pattern).is_some()
}

fn exact(words: &[&str], expected: &[&str]) -> bool {
    permission_shapes::exact_words(words, expected)
}

fn exact_any(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives.iter().any(|expected| exact(words, expected))
}

fn prefix(words: &[&str], expected: &[&str]) -> bool {
    permission_shapes::prefix_words(words, expected)
}

fn prefix_any(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives.iter().any(|expected| prefix(words, expected))
}

fn suffix(words: &[&str], expected: &[&str]) -> bool {
    permission_shapes::suffix_words(words, expected)
}

fn suffix_any(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives.iter().any(|expected| suffix(words, expected))
}

fn contains(words: &[&str], expected: &[&str]) -> bool {
    permission_shapes::find_words(words, expected).is_some()
}

fn contains_all(words: &[&str], expected: &[&str]) -> bool {
    expected
        .iter()
        .all(|word| permission_shapes::find_words(words, &[*word]).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_typed_attack_restriction_surfaces() {
        let words = [
            "this", "creature", "cant", "attack", "unless", "you", "control", "a", "goblin",
        ];
        assert_eq!(
            parse_cant_pattern(&words, CantPattern::ThisCantAttackUnlessPrefix),
            Some(CantPattern::ThisCantAttackUnlessPrefix)
        );
        assert_eq!(
            parse_cant_pattern(&words, CantPattern::SourceCantAttack),
            None
        );
    }
}
