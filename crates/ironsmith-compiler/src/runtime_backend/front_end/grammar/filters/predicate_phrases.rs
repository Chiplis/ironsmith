use super::*;
use crate::runtime_backend::sentences::effect_sentences::clause_pattern_helpers::{
    ClauseShape, clause_shape,
};

const OUTLAW_SHORTHAND_FILTER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["outlaw"],
            &["outlaws"],
            &["outlaw", "creature"],
            &["outlaws", "creatures"],
        ]
);
const NO_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["no"]);
const SACRIFICED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["sacrificed"]);
const PERMANENT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["permanent"]);
const CREATURE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["creature"]);
const SOURCE_EXILED_WITH_COUNTER_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["this", "card", "is", "exiled", "with"],
            &["this", "source", "is", "exiled", "with"],
        ]
);
const COUNTER_OR_COUNTERS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["counter"], &["counters"]]);
const COUNTER_ON_SOURCE_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["on", "it"], &["on", "this"], &["on", "them"]]);
const THAT_SPELL_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["that", "spell"]);
const SPELL_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["spell"]);
const IT_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["it"]);
const TARGETS_ONLY_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["targets", "only"]);
const TARGET_THIS_CREATURE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["this", "creature"]);
const TARGET_THIS_ARTIFACT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["this", "artifact"]);
const TARGET_THIS_ENCHANTMENT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["this", "enchantment"]);
const TARGET_THIS_LAND_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["this", "land"]);
const TARGET_THIS_PERMANENT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["this", "permanent"]);
const TARGET_SOURCE_REFERENCE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["this", "source"], &["it"]]);
const PERMANENTS_YOU_CONTROL_SCOPE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["permanent", "you", "control"],
            &["permanent", "you", "controls"],
            &["permanents", "you", "control"],
            &["permanents", "you", "controls"],
        ]
);
const CARDS_IN_YOUR_GRAVEYARD_SCOPE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["card", "in", "your", "graveyard"],
            &["cards", "in", "your", "graveyard"],
        ]
);
const PERMANENTS_AND_OR_GRAVEYARD_CONNECTOR_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["and/or"]);
const PERMANENTS_AND_OR_SPLIT_CONNECTOR_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["and", "or"]);
const THERE_ARE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["there", "are"]);
const THERE_ARE_OR_WERE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["there", "are"], &["there", "were"]]);
const THERE_IS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["there", "is"]);
const OR_IF_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["or", "if"]);
const AND_YOUR_LIFE_TOTAL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["and", "your", "life", "total"]);
const COLOR_OR_COLORS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["color"], &["colors"]]);
const AMONG_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["among"]);
const CARD_TYPES_AMONG_CARDS_IN_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["card", "type", "among", "card", "in"],
            &["card", "type", "among", "cards", "in"],
            &["card", "types", "among", "card", "in"],
            &["card", "types", "among", "cards", "in"],
        ]
);
const TYPE_OR_TYPES_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["type"], &["types"]]);
const SACRIFICED_OR_SACRIFICED_TAG_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["sacrificed"], &["sacrificed_0"]]);
const PERMANENT_OR_PERMANENTS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["permanent"], &["permanents"]]);
const LIFE_TOTAL_AT_LEAST_STARTING_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "your", "life", "total", "is", "greater", "than", "or", "equal", "to", "your",
            "starting", "life", "total",
        ]
);
const OR_MORE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["or", "more"]);
const HAS_OR_HAVE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["has"], &["have"]]);
const IN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["in"]);
const INSTEAD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["instead"]);
const GRAVEYARD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["graveyard"]);
const MORE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["more"]);
const OTHER_OR_ANOTHER_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["another"], &["other"]]);
const OR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["or"]);
const CHOSEN_NAME_TAG: &str = "__chosen_name__";
const PUT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["put"]);
const YOU_REVEALED_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["you", "revealed"]);
const BEHOLD_CAST_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["as", "you", "cast", "this", "spell"]);
const CONTROL_OR_CONTROLLED_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["control"], &["controlled"]]);
const CARD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["card"]);
const CARD_OR_CARDS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["card"], &["cards"]]);
const IN_YOUR_GRAVEYARD_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["in", "your", "graveyard"],
            &["in", "graveyard"],
            &["in", "the", "graveyard"],
        ]
);
const IT_EXPLOITED_TRIGGERING_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["it", "exploited", "that", "creature"],
            &["it", "exploited", "that", "object"],
        ]
);
const SOURCE_IN_HAND_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "is", "in", "your", "hand"],
            &["this", "card", "is", "in", "your", "hand"],
        ]
);
const SOURCE_IN_GRAVEYARD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "is", "in", "your", "graveyard"],
            &["this", "card", "is", "in", "your", "graveyard"],
            &["this", "creature", "is", "in", "your", "graveyard"],
            &["this", "permanent", "is", "in", "your", "graveyard"],
            &["this", "object", "is", "in", "your", "graveyard"],
        ]
);
const SOURCE_IN_LIBRARY_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "is", "in", "your", "library"],
            &["this", "card", "is", "in", "your", "library"],
        ]
);
const SOURCE_IN_EXILE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "is", "in", "exile"],
            &["this", "card", "is", "in", "exile"],
        ]
);
const SOURCE_IN_COMMAND_ZONE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "is", "in", "the", "command", "zone"],
            &["this", "card", "is", "in", "the", "command", "zone"],
        ]
);
const COST_PAID_INSTEAD_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["cost", "was", "paid"], &["cost", "wasnt", "paid"]]);
const COST_NOT_PAID_INSTEAD_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["cost", "was", "not", "paid"]);
const GETS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["gets"]);
const MORE_VOTES_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["more", "votes"]);
const MORE_VOTES_OR_TIED_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["more", "votes", "or", "vote", "is", "tied"]);
const NO_WORDS_GOT_VOTES_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["no"]; suffix & ["got", "votes"]);
const MELD_ATTACKING_OWN_CONTROL_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "are",
            "attacking",
            "and",
            "you",
            "both",
            "own",
            "and",
            "control",
            "them",
        ]
);
const YOU_ATTACKED_THIS_TURN_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["you", "attacked", "this", "turn"]);
const SOURCE_IS_YOUR_RING_BEARER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "is", "your", "ring", "bearer"],
            &["this", "creature", "is", "your", "ring", "bearer"],
        ]
);
const RING_HAS_TEMPTED_YOU_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(
        prefix_any
            & [
                &["ring", "has", "tempted", "you"],
                &["the", "ring", "has", "tempted", "you"],
            ]
    );
const TIMES_THIS_GAME_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["times", "this", "game"], &["time", "this", "game"]]);
const TRIGGERING_OBJECT_HAD_TO_ATTACK_THIS_COMBAT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["that", "creature", "had", "to", "attack", "this", "combat"],
            &["it", "had", "to", "attack", "this", "combat"],
            &["that", "creature", "must", "attack", "this", "combat"],
            &["it", "must", "attack", "this", "combat"],
        ]
);
const YOU_ATTACKED_WITH_EXACTLY_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["you", "attacked", "with", "exactly"]);
const OTHER_CREATURES_THIS_COMBAT_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["other", "creature", "this", "combat"],
            &["other", "creatures", "this", "combat"],
            &["others", "creature", "this", "combat"],
            &["others", "creatures", "this", "combat"],
        ]
);
const SOURCE_ATTACKED_OR_BLOCKED_THIS_TURN_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "this", "creature", "attacked", "or", "blocked", "this", "turn",
            ],
            &[
                "this",
                "permanent",
                "attacked",
                "or",
                "blocked",
                "this",
                "turn",
            ],
            &["this", "attacked", "or", "blocked", "this", "turn"],
            &["it", "attacked", "or", "blocked", "this", "turn"],
        ]
);
const YOU_CAST_SOURCE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["you", "cast", "it"], &["you", "cast", "this", "spell"]]);
const TAGGED_WAS_CAST_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["it", "was", "cast"],
            &["that", "creature", "was", "cast"],
            &["that", "permanent", "was", "cast"],
            &["that", "object", "was", "cast"],
        ]
);
const THIS_SPELL_WAS_CAST_FROM_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["this", "spell", "was", "cast", "from"]);
const NO_SPELLS_CAST_LAST_TURN_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["no", "spells", "were", "cast", "last", "turn"],
            &["no", "spell", "was", "cast", "last", "turn"],
        ]
);
const THIS_SPELL_WAS_KICKED_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "spell", "was", "kicked"],
            &["this", "creature", "was", "kicked"],
            &["this", "permanent", "was", "kicked"],
        ]
);
const THIS_SPELL_WAS_BARGAINED_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "spell", "was", "bargained"],
            &["it", "was", "bargained"],
        ]
);
const GIFT_PROMISED_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["gift", "was", "promised"]);
const GIFT_NOT_PROMISED_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["gift", "wasnt", "promised"],
            &["gift", "was", "not", "promised"],
        ]
);
const COST_WAS_PAID_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["cost", "was", "paid"]);
const COST_WASNT_PAID_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["cost", "wasnt", "paid"]);
const COST_WAS_NOT_PAID_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["cost", "was", "not", "paid"]);
const ARTICLE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["a"], &["an"]]);
const DEFINITE_ARTICLE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["the"]);
const WAS_OR_WERE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["was"], &["were"]]);
const WAS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["was"]);
const BEHELD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["beheld"]);
const THIS_POSSESSIVE_PAID_LABEL_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["this"]; suffix & ["cost", "was", "paid"]);
const THIS_POSSESSIVE_PAID_SUBJECT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["spell's"],
            &["spells"],
            &["card's"],
            &["cards"],
            &["creature's"],
            &["creatures"],
            &["permanent's"],
            &["permanents"],
        ]
);
const IT_WAS_KICKED_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["it", "was", "kicked"]);
const THAT_WAS_KICKED_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["that", "was", "kicked"]);
const MANA_SPENT_TO_CAST_THIS_SPELL_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["was", "spent", "to", "cast", "this", "spell"],
            &["were", "spent", "to", "cast", "this", "spell"],
        ]
);
const YOU_CONTROLLED_TAGGED_PERMANENT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["you", "controlled", "that", "permanent"],
            &["you", "control", "that", "permanent"],
        ]
);
const TAGGED_ENTERED_UNDER_YOUR_CONTROL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["it", "entered", "under", "your", "control"],
            &["that", "card", "entered", "under", "your", "control"],
            &["that", "permanent", "entered", "under", "your", "control"],
        ]
);
const YOU_PUT_ONTO_BATTLEFIELD_THIS_WAY_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["you", "put"]; suffix & ["onto", "the", "battlefield", "this", "way"]);
const IS_PUT_ONTO_BATTLEFIELD_THIS_WAY_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["is", "put", "onto", "battlefield", "this", "way"]);
const YOU_DIDNT_PUT_TAGGED_INTO_HAND_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["you", "dont", "put", "the", "card", "into", "your", "hand"],
            &["you", "didnt", "put", "the", "card", "into", "your", "hand"],
            &[
                "you", "did", "not", "put", "the", "card", "into", "your", "hand",
            ],
            &["you", "dont", "put", "card", "into", "your", "hand"],
            &["you", "didnt", "put", "card", "into", "your", "hand"],
            &["you", "did", "not", "put", "card", "into", "your", "hand"],
            &["you", "dont", "put", "it", "into", "your", "hand"],
            &["you", "didnt", "put", "it", "into", "your", "hand"],
            &["you", "did", "not", "put", "it", "into", "your", "hand"],
        ]
);
const YOU_DIDNT_PUT_TAGGED_ONTO_BATTLEFIELD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["you", "dont", "put", "the", "card", "onto", "battlefield"],
            &["you", "didnt", "put", "the", "card", "onto", "battlefield"],
            &[
                "you",
                "did",
                "not",
                "put",
                "the",
                "card",
                "onto",
                "battlefield"
            ],
            &["you", "dont", "put", "card", "onto", "battlefield"],
            &["you", "didnt", "put", "card", "onto", "battlefield"],
            &["you", "did", "not", "put", "card", "onto", "battlefield"],
            &["you", "dont", "put", "that", "card", "onto", "battlefield"],
            &["you", "didnt", "put", "that", "card", "onto", "battlefield",],
            &[
                "you",
                "did",
                "not",
                "put",
                "that",
                "card",
                "onto",
                "battlefield",
            ],
            &["you", "dont", "put", "it", "onto", "battlefield"],
            &["you", "didnt", "put", "it", "onto", "battlefield"],
            &["you", "did", "not", "put", "it", "onto", "battlefield"],
        ]
);
const TAGGED_WASNT_BLOCKING_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["it", "wasnt", "blocking"],
            &["it", "was", "not", "blocking"],
            &["that", "creature", "wasnt", "blocking"],
        ]
);
const NO_CREATURES_ON_BATTLEFIELD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["no", "creatures", "are", "on", "battlefield"]);
const YOU_OR_DEFENDING_PLAYER_HAS_INITIATIVE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "you",
                "or",
                "player",
                "youre",
                "attacking",
                "has",
                "initiative",
            ],
            &[
                "you",
                "or",
                "a",
                "player",
                "youre",
                "attacking",
                "has",
                "the",
                "initiative",
            ],
        ]
);
const IT_IS_NIGHT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["its", "night"], &["it", "is", "night"], &["it", "night"]]);
const FIRST_COMBAT_PHASE_OF_TURN_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "its", "the", "first", "combat", "phase", "of", "the", "turn",
            ],
            &[
                "it", "is", "the", "first", "combat", "phase", "of", "the", "turn",
            ],
            &["it", "first", "combat", "phase", "of", "turn"],
            &["it", "the", "first", "combat", "phase", "of", "the", "turn"],
        ]
);
const SOURCE_DEALT_COMBAT_DAMAGE_TO_PLAYER_THIS_TURN_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "it", "dealt", "combat", "damage", "to", "player", "this", "turn",
            ],
            &[
                "it", "dealt", "combat", "damage", "to", "a", "player", "this", "turn",
            ],
        ]
);
const PLAYER_WAS_DEALT_COMBAT_DAMAGE_BY_SUBTYPE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["a", "player", "was", "dealt", "combat", "damage", "by",],
            &["player", "was", "dealt", "combat", "damage", "by",],
            &["an", "opponent", "was", "dealt", "combat", "damage", "by",],
            &["opponent", "was", "dealt", "combat", "damage", "by",],
        ]
);
const CAST_THIS_SPELL_DURING_YOUR_MAIN_PHASE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "you", "cast", "this", "spell", "during", "your", "main", "phase",
        ]
);
const YOU_CONTROL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["you", "control"], &["you", "controls"]]);
const YOU_CONTROL_NO_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["you", "control", "no"],
            &["you", "controls", "no"],
            &["you", "control", "neither"],
            &["you", "controls", "neither"],
        ]
);
const PLAYER_CONTROLS_NO_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["player", "control", "no"], &["player", "controls", "no"]]);
const YOU_DONT_CONTROL_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["you", "dont", "control"],
            &["you", "dont", "controls"],
            &["you", "don't", "control"],
            &["you", "don't", "controls"],
        ]
);
const YOU_DO_NOT_CONTROL_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["you", "do", "not", "control"],
            &["you", "do", "not", "controls"]
        ]
);
const THAT_PLAYER_CONTROLS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["that", "player", "control"],
            &["that", "player", "controls"],
            &["that", "players", "control"],
            &["that", "players", "controls"],
        ]
);
const WITH_DIFFERENT_POWERS_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["with", "different", "powers"],
            &["with", "different", "power"],
        ]
);
const NOT_TOKEN_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["not", "token"]);
const THAT_ENCHANTMENT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["that", "enchantment"]);
const EQUIPPED_CREATURE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["equipped", "creature"]);
const ENCHANTED_CREATURE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["enchanted", "creature"]);
const YOUR_GRAVEYARD_WORDS_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["your", "graveyard"]);
const YOU_BOTH_OWN_AND_CONTROL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["you", "both", "own", "and"]);
const AND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["and"]);
const THIS_WAY_SUFFIX_PATTERN: ClauseShape<'static> = clause_shape!(suffix & ["this", "way"]);
const PASSIVE_THIS_WAY_COPULA_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["is"], &["are"], &["was"], &["were"]]);
const PASSIVE_THIS_WAY_VERB_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["countered"],
            &["destroyed"],
            &["discarded"],
            &["exiled"],
            &["milled"],
            &["returned"],
            &["revealed"],
            &["sacrificed"],
        ]
);
const IT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["it"]);
const THAT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["that"]);
const PREDICATE_REFERENCE_NOUN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["artifact"],
            &["card"],
            &["creature"],
            &["creatures"],
            &["enchantment"],
            &["land"],
            &["object"],
            &["permanent"],
            &["source"],
            &["spell"],
            &["token"],
        ]
);
const OR_COMPARISON_TAIL_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["more"], &["fewer"], &["less"], &["greater"], &["equal"]]);
const ITS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["its"], &["it's"]]);
const IT_S_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["it", "s"]);
const YOUR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["your"]);
const THEIR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["their"]);
const HAVE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["have"]);
const YOU_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["you"]);
const WHILE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["while"]);
const MANA_VALUE_HEAD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["mana", "value"]);
const COLORS_SPENT_TO_CAST_SOURCE_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "less", "than", "or", "equal", "to", "number", "of", "colors", "of", "mana",
                "spent", "to", "cast", "this", "spell",
            ],
            &[
                "less", "than", "or", "equal", "to", "number", "of", "color", "of", "mana",
                "spent", "to", "cast", "this", "spell",
            ],
        ]
);
const TOTAL_POWER_TOUGHNESS_HEAD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["total", "power", "and", "toughness"]);
const POWER_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["power"]);
const POWER_OR_TOUGHNESS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["power"], &["toughness"]]);
const HAS_OR_HAVE_TOXIC_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["has", "toxic"], &["have", "toxic"]]);
const NEITHER_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["neither"]);
const THERE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["there"]);
const ONTO_BATTLEFIELD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["onto", "battlefield"], &["onto", "the", "battlefield"]]);
const MOST_COMMON_COLOR_AMONG_ALL_PERMANENTS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["most", "common", "color", "among", "all", "permanents"]);
const SOURCE_TAPPED_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["this", "tapped"], &["thiss", "tapped"]]);
const SOURCE_UNTAPPED_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "untapped"],
            &["thiss", "untapped"],
            &["this", "is", "untapped"],
            &["this", "creature", "is", "untapped"],
            &["this", "permanent", "is", "untapped"],
        ]
);
const SOURCE_OR_SOURCE_POSSESSIVE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["this"], &["thiss"]]);
const TAPPED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["tapped"]);
const UNTAPPED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["untapped"]);
const SOURCE_NOT_SADDLED_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "creature", "isnt", "saddled"],
            &["this", "permanent", "isnt", "saddled"],
            &["this", "isnt", "saddled"],
            &["it", "isnt", "saddled"],
        ]
);
const SOURCE_SADDLED_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "creature", "is", "saddled"],
            &["this", "permanent", "is", "saddled"],
            &["this", "is", "saddled"],
            &["it", "is", "saddled"],
        ]
);
const IS_OR_ARE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["is"], &["are"]]);
const BE_VERB_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["is"], &["are"], &["was"], &["were"]]);
const MANA_SYMBOL_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["w"], &["u"], &["b"], &["r"], &["g"], &["c"], &["s"]]);
const SOURCE_FILTER_STATE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["is"],
            &["are"],
            &["isnt"],
            &["isn't"],
            &["arent"],
            &["aren't"],
        ]
);
const NEGATED_STATE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["isnt"], &["isn't"], &["arent"], &["aren't"]]);
const NOT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["not"]);
const SOURCE_FILTER_IGNORED_DESCRIPTOR_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["attached"], &["tapped"], &["untapped"], &["saddled"]]);
const SOURCE_REFERENCE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["it"], &["its"]]);
const ENCHANTED_BY_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["enchanted", "by"]);
const AURA_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["aura"], &["auras"]]);
const CONTROL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["control"]);
const CONTROL_OR_CONTROLS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["control"], &["controls"]]);
const ZONE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["graveyard"], &["hand"], &["exile"], &["library"]]);
const OPPONENT_CONTROLS_IT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["an", "opponent", "controls", "it"],
            &["an", "opponent", "controls", "that", "creature"],
            &["an", "opponent", "controls", "that", "permanent"],
            &["opponent", "controls", "it"],
            &["opponent", "controls", "that", "creature"],
            &["opponent", "controls", "that", "permanent"],
        ]
);
const OPPONENT_CONTROLS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["opponent", "controls"]);
const AN_OPPONENT_CONTROLS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["an", "opponent", "controls"]);
const SOURCE_DIDNT_ATTACK_OR_ENTER_CONTROL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "this", "creature", "didnt", "attack", "or", "come", "under", "your", "control",
                "this", "turn",
            ],
            &[
                "this", "creature", "didnt", "attack", "or", "came", "under", "your", "control",
                "this", "turn",
            ],
        ]
);
const THERE_ARE_NO_COUNTERS_ON_SOURCE_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix & ["there", "are", "no"];
    contains_words & ["counters", "on"];
    contains_any_words & [&["this", "it", "them"]]
);
const THIS_HAS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["this", "has"]);
const THIS_TYPED_HAS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["this", "creature", "has"],
            &["this", "permanent", "has"],
            &["this", "artifact", "has"],
            &["this", "enchantment", "has"],
            &["this", "land", "has"],
            &["this", "planeswalker", "has"],
            &["this", "battle", "has"],
        ]
);
const COUNTER_ON_SOURCE_PRONOUN_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix & ["on"];
    contains_any_words & [&["it", "him", "her", "them", "this", "that"]]
);
const IT_HAD_NO_COUNTER_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["it", "had", "no"]);
const TYPED_OBJECT_HAD_NO_COUNTER_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["this", "creature", "had", "no"],
            &["that", "creature", "had", "no"],
            &["this", "permanent", "had", "no"],
            &["that", "permanent", "had", "no"],
        ]
);
const IT_HAD_COUNTER_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["it", "had"]);
const TYPED_OBJECT_HAD_COUNTER_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["this", "creature", "had"],
            &["that", "creature", "had"],
            &["this", "permanent", "had"],
            &["that", "permanent", "had"],
        ]
);
const COUNTER_ON_TRIGGERING_OBJECT_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix & ["on"];
    contains_any_words & [&["it", "them", "this", "that", "itself"]]
);
const COUNTER_ON_SOURCE_TAIL_ANY_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["on", "it"],
            &["on", "this"],
            &["on", "this", "artifact"],
            &["on", "this", "creature"],
            &["on", "this", "enchantment"],
            &["on", "this", "land"],
            &["on", "this", "permanent"],
        ]
);
const SOURCE_POWER_IS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["this", "creature", "power", "is"],
            &["this", "creatures", "power", "is"],
            &["this", "permanent", "power", "is"],
            &["this", "permanents", "power", "is"],
        ]
);
const SOURCE_HAS_POWER_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["this", "has", "power"]);
const BASIC_LAND_TYPES_AMONG_LANDS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["basic", "land", "type", "among", "land"],
            &["basic", "land", "type", "among", "lands"],
            &["basic", "land", "types", "among", "land"],
            &["basic", "land", "types", "among", "lands"],
        ]
);
const THAT_PLAYER_CONTROLS_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["that", "player", "controls"],
            &["that", "player", "control"],
            &["that", "players", "controls"],
        ]
);
const YOU_CONTROL_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["you", "control"], &["you", "controls"]]);
const ON_BATTLEFIELD_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix_any & [&["on", "the", "battlefield"], &["on", "battlefield"]]);
const ON_THE_BATTLEFIELD_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["on", "the", "battlefield"]);
const YOUR_GRAVEYARD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["your", "graveyard"]);
const THAT_PLAYER_GRAVEYARD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["that", "player", "graveyard"],
            &["that", "players", "graveyard"],
        ]
);
const TARGET_PLAYER_GRAVEYARD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["target", "player", "graveyard"],
            &["target", "players", "graveyard"],
        ]
);
const TARGET_OPPONENT_GRAVEYARD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["target", "opponent", "graveyard"],
            &["target", "opponents", "graveyard"],
        ]
);
const OPPONENT_GRAVEYARD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["opponent", "graveyard"], &["opponents", "graveyard"]]);
const YOU_HAVE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["you", "have"]);
const THAT_PLAYER_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["that", "player"]);
const TARGET_PLAYER_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["target", "player"]);
const TARGET_OPPONENT_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["target", "opponent"]);
const EACH_OPPONENT_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["each", "opponent"]);
const A_OR_ANY_PLAYER_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["a", "player"], &["any", "player"]]);
const DEFENDING_PLAYER_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["defending", "player"]);
const ATTACKING_PLAYER_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["attacking", "player"]);
const OPPONENT_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["opponent"], &["opponents"]]);
const PLAYER_WHO_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["player", "who"]);
const PLAYER_SUBJECT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["player"]);
const YOUR_LIFE_TOTAL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["your", "life", "total"]);
const THEIR_LIFE_TOTAL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["their", "life", "total"]);
const THAT_PLAYERS_LIFE_TOTAL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["that", "players", "life", "total"]);
const TARGET_PLAYERS_LIFE_TOTAL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["target", "players", "life", "total"]);
const TARGET_OPPONENTS_LIFE_TOTAL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["target", "opponents", "life", "total"]);
const OPPONENT_LIFE_TOTAL_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["opponents", "life", "total"],
            &["opponent", "life", "total"]
        ]
);
const DEFENDING_PLAYERS_LIFE_TOTAL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["defending", "players", "life", "total"]);
const ATTACKING_PLAYERS_LIFE_TOTAL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["attacking", "players", "life", "total"]);
const HALF_STARTING_LIFE_TOTAL_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["half", "your", "starting", "life", "total"],
            &["half", "their", "starting", "life", "total"],
            &["half", "that", "players", "starting", "life", "total"],
            &["half", "target", "players", "starting", "life", "total"],
            &["half", "target", "opponents", "starting", "life", "total"],
            &["half", "opponents", "starting", "life", "total"],
            &["half", "defending", "players", "starting", "life", "total"],
            &["half", "attacking", "players", "starting", "life", "total"],
        ]
);
const LESS_THAN_OR_EQUAL_TO_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["less", "than", "or", "equal", "to"]);
const LESS_THAN_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["less", "than"]);
const THAN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["than"]);
const THAN_YOU_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["than", "you"], &["than", "you", "do"]]);
const THIS_TURN_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["this", "turn"]);

fn source_zone_from_words(words: &[&str]) -> Option<Zone> {
    if SOURCE_IN_HAND_PATTERN.matches_words(words) {
        Some(Zone::Hand)
    } else if SOURCE_IN_GRAVEYARD_PATTERN.matches_words(words) {
        Some(Zone::Graveyard)
    } else if SOURCE_IN_LIBRARY_PATTERN.matches_words(words) {
        Some(Zone::Library)
    } else if SOURCE_IN_EXILE_PATTERN.matches_words(words) {
        Some(Zone::Exile)
    } else if SOURCE_IN_COMMAND_ZONE_PATTERN.matches_words(words) {
        Some(Zone::Command)
    } else {
        None
    }
}

fn parse_outlaw_shorthand_filter(words: &[&str]) -> Option<ObjectFilter> {
    let trimmed = strip_leading_article_word_refs(words);
    if !OUTLAW_SHORTHAND_FILTER_PATTERN.matches_words(trimmed) {
        return None;
    }

    let mut filter = ObjectFilter::default();
    push_outlaw_subtypes(&mut filter.subtypes);
    filter.card_types.push(CardType::Creature);
    Some(filter)
}

fn parse_attachment_quantity_prefix(
    tokens: &[OwnedLexToken],
) -> Result<(crate::effect::Comparison, usize), CardTextError> {
    parse_quantity_comparison_prefix(tokens, false, false, "attachment-count predicate")
}

fn parse_source_exiled_with_counter_predicate(
    raw_words: &[&str],
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let with_idx = if SOURCE_EXILED_WITH_COUNTER_PREFIX_PATTERN.matches_words(raw_words) {
        4
    } else {
        return None;
    };
    let counter_idx = find_index(&raw_words[with_idx + 1..], |word| {
        COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(word)
    })? + with_idx
        + 1;
    if !raw_words
        .get(counter_idx + 1..)
        .is_some_and(|tail| COUNTER_ON_SOURCE_TAIL_PATTERN.matches_words(tail))
    {
        return None;
    }

    let counter_type = parse_counter_type_from_tokens(&tokens[with_idx + 1..=counter_idx])?;
    let count = parse_number(&tokens[with_idx + 1..counter_idx])
        .map(|(count, _)| count)
        .unwrap_or(1);
    Some(PredicateAst::And(
        Box::new(PredicateAst::SourceIsInZone(Zone::Exile)),
        Box::new(PredicateAst::SourceHasCounterAtLeast {
            counter_type,
            count,
        }),
    ))
}

fn parse_source_is_your_ring_bearer_predicate(words: &[&str]) -> Option<PredicateAst> {
    if SOURCE_IS_YOUR_RING_BEARER_PATTERN.matches_words(words) {
        Some(PredicateAst::SourceIsRingBearer {
            player: PlayerAst::You,
        })
    } else {
        None
    }
}

fn parse_ring_has_tempted_you_this_game_predicate(
    words: &[&str],
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    if !RING_HAS_TEMPTED_YOU_PREFIX_PATTERN.matches_words(words) {
        return None;
    }
    let count_start = if words.first() == Some(&"the") { 5 } else { 4 };
    let (count, used) = parse_number(tokens.get(count_start..)?)?;
    let tail = words.get(count_start + used..)?;
    if tail.len() == 5 && OR_MORE_PREFIX_PATTERN.matches_words(&tail[..2]) {
        if TIMES_THIS_GAME_TAIL_PATTERN.matches_words(&tail[2..]) {
            return Some(PredicateAst::PlayerRingTemptedThisGameOrMore {
                player: PlayerAst::You,
                count,
            });
        }
    }
    None
}

fn parse_ring_bearer_temptation_predicate(
    words: &[&str],
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    if let Some(predicate) = parse_source_is_your_ring_bearer_predicate(words) {
        return Some(predicate);
    }
    if let Some(predicate) = parse_ring_has_tempted_you_this_game_predicate(words, tokens) {
        return Some(predicate);
    }

    let and_idx = find_index(words, |word| AND_WORD_PATTERN.matches_word(word))?;
    let left_words = &words[..and_idx];
    let right_words = &words[and_idx + 1..];
    if left_words.is_empty() || right_words.is_empty() {
        return None;
    }
    let left = parse_source_is_your_ring_bearer_predicate(left_words)?;
    let right =
        parse_ring_has_tempted_you_this_game_predicate(right_words, &tokens[and_idx + 1..])?;
    Some(PredicateAst::And(Box::new(left), Box::new(right)))
}

fn parse_stack_object_targets_only_source_predicate(filtered: &[&str]) -> Option<PredicateAst> {
    let tail = if THAT_SPELL_PREFIX_PATTERN.matches_words(filtered) {
        &filtered[2..]
    } else if SPELL_PREFIX_PATTERN.matches_words(filtered)
        || IT_PREFIX_PATTERN.matches_words(filtered)
    {
        &filtered[1..]
    } else {
        return None;
    };

    if !TARGETS_ONLY_PREFIX_PATTERN.matches_words(tail) {
        return None;
    }

    let target_words = &tail[2..];
    let mut target_filter = if TARGET_THIS_CREATURE_PATTERN.matches_words(target_words) {
        ObjectFilter::creature()
    } else if TARGET_THIS_ARTIFACT_PATTERN.matches_words(target_words) {
        ObjectFilter::artifact()
    } else if TARGET_THIS_ENCHANTMENT_PATTERN.matches_words(target_words) {
        ObjectFilter::enchantment()
    } else if TARGET_THIS_LAND_PATTERN.matches_words(target_words) {
        ObjectFilter::land()
    } else if TARGET_THIS_PERMANENT_PATTERN.matches_words(target_words) {
        ObjectFilter::default().in_zone(Zone::Battlefield)
    } else if TARGET_SOURCE_REFERENCE_PATTERN.matches_words(target_words) {
        ObjectFilter::source()
    } else {
        return None;
    };
    target_filter.source = true;

    Some(PredicateAst::ItMatches(
        ObjectFilter::spell()
            .targeting_only_object(target_filter)
            .target_count_exact(1),
    ))
}

fn mana_cost_label_from_words(words: &[&str]) -> Option<String> {
    if words.is_empty() {
        return None;
    }

    let mut label = String::new();
    for word in words {
        if word.chars().all(|ch| ch.is_ascii_digit()) {
            label.push('{');
            label.push_str(word);
            label.push('}');
            continue;
        }
        if parse_mana_symbol(word).is_ok() {
            label.push('{');
            label.push_str(&word.to_ascii_uppercase());
            label.push('}');
            continue;
        }
        return None;
    }

    Some(label)
}

fn ordinal_number_word(word: &str) -> Option<u32> {
    ironsmith_core::parse_ordinal_word(word).or_else(|| parse_named_number(word))
}

fn predicate_quantity_prefix(words: &[&str]) -> Option<(crate::effect::Comparison, usize)> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    parse_quantity_comparison_prefix(&tokens, false, false, "predicate quantity").ok()
}

fn predicate_number_prefix(words: &[&str]) -> Option<(u32, usize)> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    parse_number(&tokens)
}

fn predicate_at_least_quantity_prefix(words: &[&str]) -> Option<(u32, usize)> {
    let (comparison, used) = predicate_quantity_prefix(words)?;
    let count = comparison_to_strict_at_least_threshold(&comparison)?;
    Some((count, used))
}

fn control_predicate_quantity(
    words: &[&str],
    prefix_len: usize,
) -> (Option<u32>, Option<u32>, usize) {
    let mut filter_start = prefix_len;
    let mut min_count = None;
    let mut exact_count = None;

    if let Some((comparison, used)) =
        predicate_quantity_prefix(words.get(prefix_len..).unwrap_or_default())
    {
        if let crate::effect::Comparison::Equal(count) = comparison
            && count >= 0
        {
            exact_count = Some(count as u32);
            filter_start = prefix_len + used;
        } else if let Some(threshold) = comparison_to_at_least_threshold(&comparison) {
            min_count = Some(threshold);
            filter_start = prefix_len + used;
        }
    }

    (min_count, exact_count, filter_start)
}

fn parse_player_controls_predicate(
    words: &[&str],
    player: PlayerAst,
    controller: Option<PlayerFilter>,
    prefix_len: usize,
    allow_outlaw_shorthand: bool,
    allow_different_powers: bool,
) -> Result<Option<PredicateAst>, CardTextError> {
    let control_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    if let Some(control_condition) =
        crate::runtime_backend::grammar::conditions::parse_control_condition(
            &control_tokens,
            crate::runtime_backend::grammar::conditions::ControlConditionOptions {
                allow_that_player: player == PlayerAst::That,
                allow_opponent_players: false,
                bind_filter_controller_to_subject: controller.is_some(),
                allow_different_powers_tail: allow_different_powers,
                default_filter_zone: None,
            },
        )
    {
        return Ok(Some(predicate_from_control_condition(control_condition)));
    }

    let (min_count, exact_count, filter_start) = control_predicate_quantity(words, prefix_len);
    let mut control_words = words[filter_start..].to_vec();
    if control_words.is_empty() {
        return Ok(None);
    }

    let mut requires_different_powers = false;
    if allow_different_powers
        && WITH_DIFFERENT_POWERS_TAIL_PATTERN
            .matches_words(&control_words[control_words.len().saturating_sub(3)..])
    {
        requires_different_powers = true;
        control_words.truncate(control_words.len().saturating_sub(3));
    }

    let control_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&control_words);
    let other = control_tokens
        .first()
        .is_some_and(|token| OTHER_OR_ANOTHER_WORD_PATTERN.matches_token(token));
    let parsed_filter = parse_object_filter(&control_tokens, other).or_else(|_| {
        if allow_outlaw_shorthand {
            parse_outlaw_shorthand_filter(&control_words)
                .ok_or_else(|| CardTextError::ParseError("unsupported control filter".to_string()))
        } else {
            Err(CardTextError::ParseError(
                "unsupported control filter".to_string(),
            ))
        }
    });
    let Ok(mut filter) = parsed_filter else {
        return Ok(None);
    };
    if let Some(controller) = controller {
        filter.controller = Some(controller);
    }

    if let Some(count) = exact_count {
        return Ok(Some(PredicateAst::PlayerControlsExactly {
            player,
            filter,
            count,
        }));
    }
    if let Some(count) = min_count
        && count > 1
    {
        if requires_different_powers {
            return Ok(Some(
                PredicateAst::PlayerControlsAtLeastWithDifferentPowers {
                    player,
                    filter,
                    count,
                },
            ));
        }
        return Ok(Some(PredicateAst::PlayerControlsAtLeast {
            player,
            filter,
            count,
        }));
    }
    Ok(Some(PredicateAst::PlayerControls { player, filter }))
}

fn predicate_from_control_condition(
    control_condition: crate::runtime_backend::grammar::conditions::ControlConditionAst,
) -> PredicateAst {
    match control_condition.comparison {
        crate::effect::Comparison::Equal(count) if count >= 0 => {
            PredicateAst::PlayerControlsExactly {
                player: control_condition.player,
                filter: control_condition.filter,
                count: count as u32,
            }
        }
        _ => {
            let Some(count) = control_condition.at_least_count() else {
                return PredicateAst::PlayerControls {
                    player: control_condition.player,
                    filter: control_condition.filter,
                };
            };
            if count > 1 {
                if control_condition.requires_different_powers {
                    return PredicateAst::PlayerControlsAtLeastWithDifferentPowers {
                        player: control_condition.player,
                        filter: control_condition.filter,
                        count,
                    };
                }
                return PredicateAst::PlayerControlsAtLeast {
                    player: control_condition.player,
                    filter: control_condition.filter,
                    count,
                };
            }
            PredicateAst::PlayerControls {
                player: control_condition.player,
                filter: control_condition.filter,
            }
        }
    }
}

fn parse_this_ability_resolution_count_predicate(filtered: &[&str]) -> Option<PredicateAst> {
    let count = match filtered {
        [
            "this",
            "is",
            count,
            "time",
            "this",
            "ability",
            "has",
            "resolved",
            "this",
            "turn",
        ]
        | [
            "this",
            "is",
            count,
            "time",
            "this",
            "ability",
            "resolved",
            "this",
            "turn",
        ]
        | [
            "this",
            "ability",
            "has",
            "resolved",
            "for",
            count,
            "time",
            "this",
            "turn",
        ]
        | [
            "this",
            "ability",
            "resolved",
            "for",
            count,
            "time",
            "this",
            "turn",
        ] => ordinal_number_word(count)?,
        ["it's", count, "time"] | ["its", count, "time"] | ["it", "s", count, "time"] => {
            ordinal_number_word(count)?
        }
        _ => return None,
    };

    Some(PredicateAst::ThisAbilityResolvedThisTurnExactly(count))
}

fn predicate_tokens_from_words(words: &[&str]) -> Vec<OwnedLexToken> {
    crate::runtime_backend::lexer::synthetic_word_tokens(words)
}

fn parse_color_only_object_filter_words(words: &[&str]) -> Option<ObjectFilter> {
    let mut filter = ObjectFilter::default();
    let mut saw_color = false;
    for word in words {
        if AND_WORD_PATTERN.matches_word(word) || OR_WORD_PATTERN.matches_word(word) {
            continue;
        }
        if let Some(color) = parse_color(word) {
            let existing = filter.colors.unwrap_or(ColorSet::new());
            filter.colors = Some(existing.union(color));
            saw_color = true;
            continue;
        }
        if let Some(color) = parse_non_color(word) {
            filter.excluded_colors = filter.excluded_colors.union(color);
            saw_color = true;
            continue;
        }
        return None;
    }
    saw_color.then_some(filter)
}

fn parse_this_way_object_filter_words(words: &[&str]) -> Option<ObjectFilter> {
    let (words, needs_chosen_name) = if let Some(base_words) =
        crate::runtime_backend::lexer::word_slice_strip_suffix(words, &["with", "chosen", "name"])
    {
        (base_words, true)
    } else {
        (words, false)
    };
    let has_card_noun = words
        .last()
        .is_some_and(|word| CARD_OR_CARDS_WORD_PATTERN.matches_word(word));
    let candidates = [
        (words, has_card_noun),
        (
            crate::runtime_backend::lexer::word_slice_strip_suffix(words, &["card"])
                .unwrap_or(words),
            true,
        ),
        (
            crate::runtime_backend::lexer::word_slice_strip_suffix(words, &["cards"])
                .unwrap_or(words),
            true,
        ),
    ];
    for (candidate, stripped_card_noun) in candidates {
        if candidate.is_empty() {
            let mut filter = ObjectFilter::default();
            if needs_chosen_name {
                filter.tagged_constraints.push(TaggedObjectConstraint {
                    tag: TagKey::from(CHOSEN_NAME_TAG),
                    relation: TaggedOpbjectRelation::SameNameAsTagged,
                });
            }
            return Some(filter);
        }
        let tokens = predicate_tokens_from_words(candidate);
        if let Ok(mut filter) = parse_object_filter(&tokens, false) {
            if stripped_card_noun {
                filter.zone = None;
            }
            if needs_chosen_name {
                filter.tagged_constraints.push(TaggedObjectConstraint {
                    tag: TagKey::from(CHOSEN_NAME_TAG),
                    relation: TaggedOpbjectRelation::SameNameAsTagged,
                });
            }
            return Some(filter);
        }
        if let Some(mut filter) = parse_color_only_object_filter_words(candidate) {
            if stripped_card_noun {
                filter.zone = None;
            }
            if needs_chosen_name {
                filter.tagged_constraints.push(TaggedObjectConstraint {
                    tag: TagKey::from(CHOSEN_NAME_TAG),
                    relation: TaggedOpbjectRelation::SameNameAsTagged,
                });
            }
            return Some(filter);
        }
    }
    None
}

fn parse_passive_this_way_tagged_object_predicate(
    filtered: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    if filtered.len() < 5 || !THIS_WAY_SUFFIX_PATTERN.matches_words(filtered) {
        return Ok(None);
    }
    let verb_idx = filtered.len() - 3;
    let copula_idx = verb_idx.saturating_sub(1);
    if copula_idx == 0
        || !PASSIVE_THIS_WAY_COPULA_WORD_PATTERN.matches_word(filtered[copula_idx])
        || !PASSIVE_THIS_WAY_VERB_WORD_PATTERN.matches_word(filtered[verb_idx])
    {
        return Ok(None);
    }

    let filter_words = &filtered[..copula_idx];
    let Some(filter) = parse_this_way_object_filter_words(filter_words) else {
        return Ok(None);
    };
    Ok(Some(PredicateAst::TaggedMatches(
        TagKey::from(IT_TAG),
        filter,
    )))
}

fn parse_repeated_if_or_predicate(
    filtered: &[&str],
) -> Result<Option<PredicateAst>, CardTextError> {
    let Some(or_idx) = OR_IF_PATTERN.find_exact_window_range(filtered, 2, 2) else {
        return Ok(None);
    };
    if or_idx == 0 || or_idx + 2 >= filtered.len() {
        return Ok(None);
    }

    let left_tokens = predicate_tokens_from_words(&filtered[..or_idx]);
    let right_tokens = predicate_tokens_from_words(&filtered[or_idx + 2..]);
    let left = match parse_predicate(&left_tokens) {
        Ok(predicate) => predicate,
        Err(_) => return Ok(None),
    };
    let right = parse_predicate(&right_tokens)?;
    Ok(Some(PredicateAst::Or(Box::new(left), Box::new(right))))
}

fn predicate_reference_prefix<'a>(words: &'a [&'a str]) -> Option<&'a [&'a str]> {
    if words
        .first()
        .is_some_and(|word| IT_WORD_PATTERN.matches_word(word))
    {
        return Some(&words[..1]);
    }
    if words.len() >= 2
        && THAT_WORD_PATTERN.matches_word(words[0])
        && PREDICATE_REFERENCE_NOUN_WORD_PATTERN.matches_word(words[1])
    {
        return Some(&words[..2]);
    }
    None
}

fn predicate_words_start_with_reference(words: &[&str]) -> bool {
    matches!(
        words.first().copied(),
        Some(
            "it" | "its"
                | "this"
                | "that"
                | "you"
                | "your"
                | "opponent"
                | "player"
                | "target"
                | "source"
        )
    )
}

fn parse_single_card_type_card_descriptor(words: &[&str]) -> Option<ObjectFilter> {
    if words.len() == 2
        && CARD_OR_CARDS_WORD_PATTERN.matches_word(words[1])
        && let Some(card_type) = parse_card_type(words[0])
    {
        return Some(ObjectFilter {
            card_types: vec![card_type],
            ..Default::default()
        });
    }
    None
}

fn parse_or_predicate(filtered: &[&str]) -> Result<Option<PredicateAst>, CardTextError> {
    let Some(or_idx) = rfind_index_with(filtered, |idx, word| {
        if !OR_WORD_PATTERN.matches_word(word) || idx == 0 || idx + 1 >= filtered.len() {
            return false;
        }
        if filtered
            .get(idx + 1)
            .is_some_and(|word| OR_COMPARISON_TAIL_WORD_PATTERN.matches_word(word))
        {
            return false;
        }
        true
    }) else {
        return Ok(None);
    };

    let left_words = &filtered[..or_idx];
    let right_words = &filtered[or_idx + 1..];
    let left_tokens = predicate_tokens_from_words(left_words);
    let right_tokens = predicate_tokens_from_words(right_words);
    let left = parse_predicate(&left_tokens)?;
    let right = match parse_predicate(&right_tokens) {
        Ok(predicate) => predicate,
        Err(original_err) => {
            let Some(reference_prefix) = predicate_reference_prefix(left_words) else {
                return Err(original_err);
            };
            if predicate_words_start_with_reference(right_words) {
                return Err(original_err);
            }
            let prefixed_words = reference_prefix
                .iter()
                .copied()
                .chain(right_words.iter().copied())
                .collect::<Vec<_>>();
            let prefixed_tokens = predicate_tokens_from_words(&prefixed_words);
            parse_predicate(&prefixed_tokens).map_err(|_| original_err)?
        }
    };
    Ok(Some(PredicateAst::Or(Box::new(left), Box::new(right))))
}

fn player_filter_for_turn_value(player: PlayerAst) -> Option<PlayerFilter> {
    match player {
        PlayerAst::You | PlayerAst::Implicit => Some(PlayerFilter::You),
        PlayerAst::Any => Some(PlayerFilter::Any),
        PlayerAst::Chosen => Some(PlayerFilter::ChosenPlayer),
        PlayerAst::Defending => Some(PlayerFilter::Defending),
        PlayerAst::Attacking => Some(PlayerFilter::Attacking),
        PlayerAst::MostCardsInHand => Some(PlayerFilter::MostCardsInHand),
        PlayerAst::MostLifeTied => Some(PlayerFilter::MostLifeTied),
        PlayerAst::LowestLifeTied => Some(PlayerFilter::LowestLifeTied),
        PlayerAst::Target => Some(PlayerFilter::target_player()),
        PlayerAst::TargetOpponent => Some(PlayerFilter::target_opponent()),
        PlayerAst::Opponent => Some(PlayerFilter::Opponent),
        PlayerAst::NotYou => Some(PlayerFilter::NotYou),
        PlayerAst::That => Some(PlayerFilter::IteratedPlayer),
        PlayerAst::ThatPlayerOrTargetController => {
            Some(PlayerFilter::TargetPlayerOrControllerOfTarget)
        }
        PlayerAst::ItsController | PlayerAst::ItsOwner => None,
    }
}

fn player_ast_from_status_player_filter(player: PlayerFilter) -> Option<PlayerAst> {
    match player {
        PlayerFilter::You => Some(PlayerAst::You),
        PlayerFilter::Any => Some(PlayerAst::Any),
        PlayerFilter::Defending => Some(PlayerAst::Defending),
        PlayerFilter::Attacking => Some(PlayerAst::Attacking),
        PlayerFilter::Opponent => Some(PlayerAst::Opponent),
        PlayerFilter::IteratedPlayer => Some(PlayerAst::That),
        PlayerFilter::Target(base) if *base == PlayerFilter::Opponent => {
            Some(PlayerAst::TargetOpponent)
        }
        PlayerFilter::Target(_) => Some(PlayerAst::Target),
        _ => None,
    }
}

fn parse_player_status_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let status =
        crate::runtime_backend::grammar::conditions::parse_player_status_condition(&tokens)?;
    match status.status {
        crate::runtime_backend::grammar::conditions::PlayerStatusAst::Monarch => {
            Some(PredicateAst::PlayerIsMonarch {
                player: player_ast_from_status_player_filter(status.player)?,
            })
        }
        crate::runtime_backend::grammar::conditions::PlayerStatusAst::Initiative => {
            Some(PredicateAst::PlayerHasInitiative {
                player: player_ast_from_status_player_filter(status.player)?,
            })
        }
        crate::runtime_backend::grammar::conditions::PlayerStatusAst::MaxSpeed => {
            Some(PredicateAst::ValueComparison {
                left: Value::Speed(status.player),
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(4),
            })
        }
    }
}

fn parse_player_achievement_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let achievement =
        crate::runtime_backend::grammar::conditions::parse_player_achievement_condition(&tokens)?;
    let player = player_ast_from_status_player_filter(achievement.player)?;
    let predicate = match achievement.achievement {
        crate::runtime_backend::grammar::conditions::PlayerAchievementAst::CitysBlessing => {
            Some(PredicateAst::PlayerHasCitysBlessing { player })
        }
        crate::runtime_backend::grammar::conditions::PlayerAchievementAst::CompletedDungeon {
            dungeon_name,
        } => Some(PredicateAst::PlayerCompletedDungeon {
            player,
            dungeon_name,
        }),
        crate::runtime_backend::grammar::conditions::PlayerAchievementAst::FullParty => {
            if player == PlayerAst::You {
                Some(PredicateAst::YouHaveFullParty)
            } else {
                None
            }
        }
    }?;
    if achievement.negated {
        Some(PredicateAst::Not(Box::new(predicate)))
    } else {
        Some(predicate)
    }
}

fn parse_player_cards_in_hand_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let condition =
        crate::runtime_backend::grammar::conditions::parse_player_cards_in_hand_condition(&tokens)?;
    let player = player_ast_from_status_player_filter(condition.player.clone())?;
    if player == PlayerAst::You && condition.is_no_cards_in_hand() {
        return Some(PredicateAst::YouHaveNoCardsInHand);
    }
    match condition.comparison {
        crate::effect::Comparison::GreaterThanOrEqual(count) if count >= 0 => {
            Some(PredicateAst::PlayerCardsInHandOrMore {
                player,
                count: count as u32,
            })
        }
        crate::effect::Comparison::GreaterThan(count) if count >= -1 => {
            Some(PredicateAst::PlayerCardsInHandOrMore {
                player,
                count: (count + 1) as u32,
            })
        }
        crate::effect::Comparison::LessThanOrEqual(count) if count >= 0 => {
            Some(PredicateAst::PlayerCardsInHandOrFewer {
                player,
                count: count as u32,
            })
        }
        crate::effect::Comparison::LessThan(count) if count > 0 => {
            Some(PredicateAst::PlayerCardsInHandOrFewer {
                player,
                count: (count - 1) as u32,
            })
        }
        _ => None,
    }
}

fn parse_player_life_total_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let condition =
        crate::runtime_backend::grammar::conditions::parse_player_life_total_condition(&tokens)?;
    let (operator, amount) = comparison_to_value_comparison_operator(condition.comparison)?;
    Some(PredicateAst::ValueComparison {
        left: crate::effect::Value::LifeTotal(condition.player),
        operator,
        right: crate::effect::Value::Fixed(amount),
    })
}

fn parse_player_life_relation_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let relation =
        crate::runtime_backend::grammar::conditions::parse_player_life_relation_condition(&tokens)?;
    let player = player_ast_from_status_player_filter(relation.player)?;
    match relation.relation {
        crate::runtime_backend::grammar::conditions::PlayerLifeRelationAst::HasMoreLifeThanYou => {
            Some(PredicateAst::PlayerHasMoreLifeThanYou { player })
        }
        crate::runtime_backend::grammar::conditions::PlayerLifeRelationAst::HasLessLifeThanYou => {
            Some(PredicateAst::PlayerHasLessLifeThanYou { player })
        }
        crate::runtime_backend::grammar::conditions::PlayerLifeRelationAst::HasNoOpponentWithMoreLifeThan => {
            Some(PredicateAst::PlayerHasNoOpponentWithMoreLifeThan { player })
        }
        crate::runtime_backend::grammar::conditions::PlayerLifeRelationAst::HasMoreLifeThanEachOtherPlayer => {
            Some(PredicateAst::PlayerHasMoreLifeThanEachOtherPlayer { player })
        }
    }
}

fn parse_player_cards_in_hand_relation_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let relation =
        crate::runtime_backend::grammar::conditions::parse_player_cards_in_hand_relation_condition(
            &tokens,
        )?;
    let player = player_ast_from_status_player_filter(relation.player)?;
    match relation.relation {
        crate::runtime_backend::grammar::conditions::PlayerCardsInHandRelationAst::HasMoreCardsInHandThanYou => {
            Some(PredicateAst::PlayerHasMoreCardsInHandThanYou { player })
        }
        crate::runtime_backend::grammar::conditions::PlayerCardsInHandRelationAst::HasMoreCardsInHandThanEachOtherPlayer => {
            Some(PredicateAst::PlayerHasMoreCardsInHandThanEachOtherPlayer { player })
        }
    }
}

fn parse_player_turn_event_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let condition =
        crate::runtime_backend::grammar::conditions::parse_player_turn_event_condition(&tokens)?;
    let (operator, count) = comparison_to_value_comparison_operator(condition.comparison)?;
    let left = match condition.event {
        crate::runtime_backend::grammar::conditions::PlayerTurnEventAst::CardsDrawn => {
            Value::MaxCardsDrawnThisTurn(condition.player)
        }
        crate::runtime_backend::grammar::conditions::PlayerTurnEventAst::LandsEnteredBattlefieldUnderControl => {
            Value::LandsEnteredBattlefieldThisTurn(condition.player)
        }
    };

    Some(PredicateAst::ValueComparison {
        left,
        operator,
        right: Value::Fixed(count),
    })
}

fn parse_spell_context_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let condition =
        crate::runtime_backend::grammar::conditions::parse_spell_context_condition(&tokens)?;
    match condition {
        crate::runtime_backend::grammar::conditions::SpellContextConditionAst::ControllerIsPoisoned {
            ..
        } => Some(PredicateAst::TargetSpellControllerIsPoisoned),
        crate::runtime_backend::grammar::conditions::SpellContextConditionAst::NoManaSpentToCast {
            ..
        } => Some(PredicateAst::TargetSpellNoManaSpentToCast),
        crate::runtime_backend::grammar::conditions::SpellContextConditionAst::YouControlMoreCreaturesThanController {
            ..
        } => Some(PredicateAst::YouControlMoreCreaturesThanTargetSpellController),
    }
}

fn parse_player_spell_cast_this_turn_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let condition =
        crate::runtime_backend::grammar::conditions::parse_player_spell_cast_this_turn_condition(
            &tokens,
        )?;
    match condition {
        crate::runtime_backend::grammar::conditions::PlayerSpellCastThisTurnConditionAst::CountAtLeast {
            player,
            count,
        } => Some(PredicateAst::PlayerCastSpellsThisTurnOrMore {
            player: player_ast_from_status_player_filter(player)?,
            count,
        }),
        crate::runtime_backend::grammar::conditions::PlayerSpellCastThisTurnConditionAst::MatchingFilters {
            player,
            filters,
            negated,
        } => {
            let mut predicates = filters.into_iter().map(|filter| {
                PredicateAst::ValueComparison {
                    left: Value::SpellsCastThisTurnMatching {
                        player: player.clone(),
                        filter,
                        exclude_source: false,
                    },
                    operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::Fixed(1),
                }
            });
            let first = predicates.next()?;
            let predicate = predicates
                .fold(first, |left, right| PredicateAst::And(Box::new(left), Box::new(right)));
            if negated {
                Some(PredicateAst::Not(Box::new(predicate)))
            } else {
                Some(predicate)
            }
        }
    }
}

fn parse_player_life_change_this_turn_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let condition =
        crate::runtime_backend::grammar::conditions::parse_player_life_change_this_turn_condition(
            &tokens,
        )?;
    match condition.direction {
        crate::runtime_backend::grammar::conditions::PlayerLifeChangeDirectionAst::Gained => {
            let count = comparison_to_strict_at_least_threshold(&condition.comparison)?;
            Some(PredicateAst::PlayerGainedLifeThisTurnOrMore {
                player: player_ast_from_status_player_filter(condition.player)?,
                count,
            })
        }
        crate::runtime_backend::grammar::conditions::PlayerLifeChangeDirectionAst::Lost
            if condition.player == PlayerFilter::Opponent
                && comparison_to_strict_at_least_threshold(&condition.comparison) == Some(1) =>
        {
            Some(PredicateAst::OpponentLostLifeThisTurn)
        }
        crate::runtime_backend::grammar::conditions::PlayerLifeChangeDirectionAst::Lost => {
            let (operator, count) = comparison_to_value_comparison_operator(condition.comparison)?;
            Some(PredicateAst::ValueComparison {
                left: Value::LifeLostThisTurn(condition.player),
                operator,
                right: Value::Fixed(count),
            })
        }
    }
}

fn parse_object_death_this_turn_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let condition =
        crate::runtime_backend::grammar::conditions::parse_object_death_this_turn_condition(
            &tokens,
        )?;
    match condition.event {
        crate::runtime_backend::grammar::conditions::ObjectDeathThisTurnEventAst::Died => {
            let count = comparison_to_strict_at_least_threshold(&condition.comparison)?;
            if count <= 1 {
                Some(PredicateAst::CreatureDiedThisTurn)
            } else {
                Some(PredicateAst::CreatureDiedThisTurnOrMore(count))
            }
        }
        crate::runtime_backend::grammar::conditions::ObjectDeathThisTurnEventAst::PutIntoYourGraveyardFromAnywhere => {
            Some(PredicateAst::CreatureCardPutIntoYourGraveyardThisTurn)
        }
    }
}

fn parse_player_would_action_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let condition =
        crate::runtime_backend::grammar::conditions::parse_player_would_action_condition(&tokens)?;
    let player = player_ast_from_status_player_filter(condition.player)?;
    match condition.action {
        crate::runtime_backend::grammar::conditions::PlayerWouldActionAst::DrawCard => {
            Some(PredicateAst::PlayerWouldDrawCard { player })
        }
        crate::runtime_backend::grammar::conditions::PlayerWouldActionAst::Proliferate => {
            Some(PredicateAst::PlayerWouldProliferate { player })
        }
        crate::runtime_backend::grammar::conditions::PlayerWouldActionAst::BeginExtraTurn => {
            Some(PredicateAst::PlayerWouldBeginExtraTurn { player })
        }
    }
}

fn parse_battlefield_entry_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let condition =
        crate::runtime_backend::grammar::conditions::parse_battlefield_entry_condition(&tokens)?;
    match condition {
        crate::runtime_backend::grammar::conditions::BattlefieldEntryConditionAst::ObjectEntered {
            filter,
            window:
                crate::runtime_backend::grammar::conditions::BattlefieldEntryTurnWindowAst::ThisTurn,
        } => Some(PredicateAst::ObjectEnteredBattlefieldThisTurn(filter)),
        crate::runtime_backend::grammar::conditions::BattlefieldEntryConditionAst::ObjectEntered {
            filter,
            window:
                crate::runtime_backend::grammar::conditions::BattlefieldEntryTurnWindowAst::LastTurn,
        } => Some(PredicateAst::ObjectEnteredBattlefieldLastTurn(filter)),
        crate::runtime_backend::grammar::conditions::BattlefieldEntryConditionAst::LandEnteredUnderYourControlThisTurn {
            player,
        } => Some(PredicateAst::PlayerHadLandEnterBattlefieldThisTurn { player }),
    }
}

fn parse_battlefield_change_this_turn_predicate(words: &[&str]) -> Option<PredicateAst> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words);
    let condition =
        crate::runtime_backend::grammar::conditions::parse_battlefield_change_this_turn_condition(
            &tokens,
        )?;
    match condition {
        crate::runtime_backend::grammar::conditions::BattlefieldChangeThisTurnConditionAst::PermanentLeftBattlefield {
            negated,
        } => {
            let predicate = PredicateAst::PermanentLeftBattlefieldThisTurn;
            if negated {
                Some(PredicateAst::Not(Box::new(predicate)))
            } else {
                Some(predicate)
            }
        }
        crate::runtime_backend::grammar::conditions::BattlefieldChangeThisTurnConditionAst::PermanentLeftBattlefieldUnderYourControl => {
            Some(PredicateAst::PermanentLeftBattlefieldUnderYourControlThisTurn)
        }
        crate::runtime_backend::grammar::conditions::BattlefieldChangeThisTurnConditionAst::ObjectPutIntoGraveyardFromBattlefield {
            filter,
        } => Some(PredicateAst::ObjectPutIntoGraveyardFromBattlefieldThisTurn(filter)),
    }
}

fn graveyard_possessive_matches_subject(player: PlayerAst, possessive: &str) -> bool {
    match player {
        PlayerAst::You | PlayerAst::Implicit => YOUR_WORD_PATTERN.matches_word(possessive),
        _ => THEIR_WORD_PATTERN.matches_word(possessive),
    }
}

fn permanents_you_control_scope(words: &[&str]) -> Option<ObjectFilter> {
    if PERMANENTS_YOU_CONTROL_SCOPE_PATTERN.matches_words(words) {
        return Some(ObjectFilter::permanent().you_control());
    }
    None
}

fn cards_in_your_graveyard_scope(words: &[&str]) -> Option<ObjectFilter> {
    if CARDS_IN_YOUR_GRAVEYARD_SCOPE_PATTERN.matches_words(words) {
        return Some(
            ObjectFilter::default()
                .in_zone(Zone::Graveyard)
                .owned_by(PlayerFilter::You),
        );
    }
    None
}

fn permanents_and_your_graveyard_scope(words: &[&str]) -> Option<ObjectFilter> {
    let graveyard_start = if words.len() == 8
        && words
            .get(3..4)
            .is_some_and(|tail| PERMANENTS_AND_OR_GRAVEYARD_CONNECTOR_PATTERN.matches_words(tail))
    {
        4
    } else if words.len() == 9
        && words
            .get(3..5)
            .is_some_and(|tail| PERMANENTS_AND_OR_SPLIT_CONNECTOR_PATTERN.matches_words(tail))
    {
        5
    } else {
        return None;
    };
    let battlefield = permanents_you_control_scope(&words[..3])?;
    let graveyard = cards_in_your_graveyard_scope(&words[graveyard_start..])?;
    let mut filter = ObjectFilter::default();
    filter.any_of = vec![battlefield, graveyard];
    Some(filter)
}

fn parse_colors_among_predicate(words: &[&str]) -> Option<PredicateAst> {
    if words.len() >= 7
        && THERE_ARE_OR_WERE_PREFIX_PATTERN.matches_words(words)
        && let Some((count, used)) = predicate_number_prefix(&words[2..])
        && words
            .get(2 + used)
            .is_some_and(|word| COLOR_OR_COLORS_WORD_PATTERN.matches_word(word))
        && words
            .get(3 + used)
            .is_some_and(|word| AMONG_WORD_PATTERN.matches_word(word))
        && let Some(filter) = permanents_you_control_scope(&words[4 + used..])
    {
        return Some(PredicateAst::ValueComparison {
            left: Value::ColorsAmong(filter),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(count as i32),
        });
    }
    None
}

fn parse_card_types_among_predicate(words: &[&str]) -> Option<PredicateAst> {
    if words.len() >= 9
        && THERE_ARE_OR_WERE_PREFIX_PATTERN.matches_words(words)
        && let Some((count, rest_start)) = predicate_at_least_quantity_prefix(&words[2..])
        && words
            .get(2 + rest_start)
            .is_some_and(|word| CARD_OR_CARDS_WORD_PATTERN.matches_word(word))
        && words
            .get(3 + rest_start)
            .is_some_and(|word| TYPE_OR_TYPES_WORD_PATTERN.matches_word(word))
        && words
            .get(4 + rest_start)
            .is_some_and(|word| AMONG_WORD_PATTERN.matches_word(word))
        && words
            .get(5 + rest_start)
            .is_some_and(|word| SACRIFICED_OR_SACRIFICED_TAG_WORD_PATTERN.matches_word(word))
        && (words
            .get(6 + rest_start)
            .is_some_and(|word| PERMANENT_OR_PERMANENTS_WORD_PATTERN.matches_word(word))
            || words.len() == 6 + rest_start)
    {
        return Some(PredicateAst::ValueComparison {
            left: Value::CardTypesAmong(ObjectFilter::tagged("sacrificed_0")),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(count as i32),
        });
    }

    if words.len() >= 13
        && THERE_ARE_OR_WERE_PREFIX_PATTERN.matches_words(words)
        && let Some((count, rest_start)) = predicate_at_least_quantity_prefix(&words[2..])
        && words
            .get(2 + rest_start)
            .is_some_and(|word| CARD_OR_CARDS_WORD_PATTERN.matches_word(word))
        && words
            .get(3 + rest_start)
            .is_some_and(|word| TYPE_OR_TYPES_WORD_PATTERN.matches_word(word))
        && words
            .get(4 + rest_start)
            .is_some_and(|word| AMONG_WORD_PATTERN.matches_word(word))
        && let Some(filter) = permanents_and_your_graveyard_scope(&words[5 + rest_start..])
    {
        return Some(PredicateAst::ValueComparison {
            left: Value::CardTypesAmong(filter),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(count as i32),
        });
    }
    None
}

fn parse_life_total_at_least_starting_predicate(words: &[&str]) -> Option<PredicateAst> {
    if LIFE_TOTAL_AT_LEAST_STARTING_PATTERN.matches_words(words) {
        return Some(PredicateAst::ValueComparison {
            left: Value::LifeTotal(PlayerFilter::You),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::StartingLifeTotal(PlayerFilter::You),
        });
    }
    None
}

fn parse_counted_objects_have_counter_predicate(words: &[&str]) -> Option<PredicateAst> {
    if words.len() < 7 {
        return None;
    }
    let (comparison, used) = predicate_quantity_prefix(words)?;
    let count = comparison_to_strict_at_least_threshold(&comparison)?;
    let have_idx = find_index(words, |word| HAS_OR_HAVE_WORD_PATTERN.matches_word(word))?;
    if have_idx <= used {
        return None;
    }
    let object_words = &words[used..have_idx];
    let counter_words = &words[have_idx + 1..];
    if object_words.is_empty() || counter_words.is_empty() {
        return None;
    }
    let (counter_constraint, consumed) = parse_filter_counter_constraint_words(counter_words)?;
    if consumed != counter_words.len() {
        return None;
    }

    let object_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(object_words);
    let other = object_tokens
        .first()
        .is_some_and(|token| OTHER_OR_ANOTHER_WORD_PATTERN.matches_token(token));
    let mut filter = parse_object_filter(&object_tokens, other).ok()?;
    filter.with_counter = Some(counter_constraint);
    if filter.zone.is_none()
        && filter.card_types.iter().any(|card_type| {
            matches!(
                card_type,
                CardType::Artifact
                    | CardType::Creature
                    | CardType::Enchantment
                    | CardType::Land
                    | CardType::Planeswalker
                    | CardType::Battle
            )
        })
    {
        filter.zone = Some(Zone::Battlefield);
    }

    Some(PredicateAst::ValueComparison {
        left: Value::Count(filter),
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(count as i32),
    })
}

fn parse_happily_style_conjoined_predicate(words: &[&str]) -> Option<PredicateAst> {
    let cleaned = word_refs_except(words, &[","]);
    let words = cleaned.as_slice();
    let second_there_idx = THERE_ARE_PREFIX_PATTERN
        .find_exact_window_range(&words[1..], 2, 2)
        .map(|idx| idx + 1)?;
    let life_idx = AND_YOUR_LIFE_TOTAL_PATTERN
        .find_exact_window_range(&words[second_there_idx + 1..], 4, 4)
        .map(|idx| idx + second_there_idx + 1)?;

    let first = parse_colors_among_predicate(&words[..second_there_idx])?;
    let second = parse_card_types_among_predicate(&words[second_there_idx..life_idx])?;
    let third = parse_life_total_at_least_starting_predicate(&words[life_idx + 1..])?;

    Some(PredicateAst::And(
        Box::new(PredicateAst::And(Box::new(first), Box::new(second))),
        Box::new(third),
    ))
}

fn parse_revealed_or_controlled_subtype_predicate(words: &[&str]) -> Option<PredicateAst> {
    let suffix_len = usize::from(BEHOLD_CAST_SUFFIX_PATTERN.matches_words(words)) * 5;
    let core_words = if suffix_len > 0 {
        &words[..words.len().saturating_sub(suffix_len)]
    } else {
        words
    };

    if core_words.len() != 7
        || !core_words
            .get(0..2)
            .is_some_and(|prefix| YOU_REVEALED_PREFIX_PATTERN.matches_words(prefix))
        || parse_subtype_word(core_words[2]).is_none()
        || !CARD_WORD_PATTERN.matches_word(core_words[3])
        || !OR_WORD_PATTERN.matches_word(core_words[4])
        || !CONTROL_OR_CONTROLLED_WORD_PATTERN.matches_word(core_words[5])
        || parse_subtype_word(core_words[6]).is_none()
        || core_words[2] != core_words[6]
    {
        return None;
    }

    Some(PredicateAst::Or(
        Box::new(PredicateAst::ThisSpellPaidLabel("Behold".to_string())),
        Box::new(PredicateAst::PlayerControls {
            player: PlayerAst::You,
            filter: ObjectFilter::default().with_subtype(parse_subtype_word(core_words[2])?),
        }),
    ))
}

fn parse_card_in_your_graveyard_predicate(words: &[&str]) -> Option<PredicateAst> {
    if words.len() < 6 || !THERE_IS_PREFIX_PATTERN.matches_words(words) {
        return None;
    }

    let in_idx = IN_WORD_PATTERN.find_word(&words[2..]).map(|idx| idx + 2)?;
    if in_idx <= 2 {
        return None;
    }
    if !IN_YOUR_GRAVEYARD_TAIL_PATTERN.matches_words(&words[in_idx..]) {
        return None;
    }

    let descriptor_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&words[2..in_idx]);
    let mut filter = parse_object_filter(&descriptor_tokens, false).ok()?;
    filter.zone = Some(Zone::Graveyard);
    filter.owner = Some(PlayerFilter::You);

    Some(PredicateAst::PlayerControls {
        player: PlayerAst::You,
        filter,
    })
}

fn parse_object_on_battlefield_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let suffix_len = if word_slice_ends_with(&words, &["is", "on", "the", "battlefield"])
        || word_slice_ends_with(&words, &["are", "on", "the", "battlefield"])
    {
        4
    } else if word_slice_ends_with(&words, &["is", "on", "battlefield"])
        || word_slice_ends_with(&words, &["are", "on", "battlefield"])
    {
        3
    } else {
        return Ok(None);
    };
    let object_token_end = tokens.len().saturating_sub(suffix_len);
    if object_token_end == 0 {
        return Ok(None);
    }

    let object_tokens = &tokens[..object_token_end];
    let mut filter = parse_object_filter(object_tokens, false)?;
    if filter.name.is_some()
        && let Some(named_idx) = object_tokens
            .iter()
            .position(|token| token.is_word("named"))
    {
        let object_words = object_tokens
            .iter()
            .filter_map(OwnedLexToken::as_word)
            .collect::<Vec<_>>();
        let name_end = find_name_clause_end(&object_words, named_idx + 1);
        let name = object_tokens[named_idx + 1..name_end]
            .iter()
            .filter_map(OwnedLexToken::as_word)
            .collect::<Vec<_>>()
            .join(" ");
        if !name.is_empty() {
            filter.name = Some(name);
        }
    }
    filter.zone = Some(Zone::Battlefield);

    Ok(Some(PredicateAst::ValueComparison {
        left: Value::Count(filter),
        operator: crate::effect::ValueComparisonOperator::GreaterThan,
        right: Value::Fixed(0),
    }))
}

pub(crate) fn parse_predicate(tokens: &[OwnedLexToken]) -> Result<PredicateAst, CardTextError> {
    let raw_words_view = GrammarFilterNormalizedWords::new(tokens);
    let raw_words = raw_words_view.to_word_refs();
    let mut filtered = non_article_word_refs(&raw_words);

    if filtered.is_empty() {
        return Err(CardTextError::ParseError(
            "empty predicate in if clause".to_string(),
        ));
    }
    if filtered.first().copied() == Some("if") {
        filtered.remove(0);
    }
    if filtered.is_empty() {
        return Err(CardTextError::ParseError(
            "empty predicate in if clause".to_string(),
        ));
    }
    if ITS_WORD_PATTERN.matches_word(filtered[0]) {
        filtered[0] = "it";
    }
    if IT_S_PREFIX_PATTERN.matches_words(&filtered) {
        filtered.remove(1);
    }
    if let Some(instead_idx) = INSTEAD_WORD_PATTERN.find_word(&filtered)
        && instead_idx > 0
    {
        let maybe_predicate = &filtered[..instead_idx];
        let paid_tail = maybe_predicate.len() >= 3
            && COST_PAID_INSTEAD_TAIL_PATTERN
                .matches_words(&maybe_predicate[maybe_predicate.len() - 3..]);
        let unpaid_tail = maybe_predicate.len() >= 4
            && COST_NOT_PAID_INSTEAD_TAIL_PATTERN
                .matches_words(&maybe_predicate[maybe_predicate.len() - 4..]);
        if paid_tail || unpaid_tail {
            filtered.truncate(instead_idx);
        }
    }

    if let Some(predicate) = parse_repeated_if_or_predicate(&filtered)? {
        return Ok(predicate);
    }
    if let Some(gets_idx) = find_index(&filtered, |word| GETS_WORD_PATTERN.matches_word(word))
        && gets_idx > 0
        && MORE_VOTES_OR_TIED_TAIL_PATTERN.matches_words(&filtered[gets_idx + 1..])
    {
        return Ok(PredicateAst::VoteOptionGetsMoreVotesOrTied {
            option: filtered[..gets_idx].join(" "),
        });
    }

    if let Some(predicate) = parse_passive_this_way_tagged_object_predicate(&filtered)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_this_ability_resolution_count_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_stack_object_targets_only_source_predicate(&filtered) {
        return Ok(predicate);
    }

    if IT_EXPLOITED_TRIGGERING_PATTERN.matches_words(&filtered) {
        return Ok(PredicateAst::And(
            Box::new(PredicateAst::TaggedMatches(
                TagKey::from(crate::tag::EXPLOITED_TAG),
                ObjectFilter::tagged("triggering"),
            )),
            Box::new(PredicateAst::TaggedMatches(
                TagKey::from(crate::tag::EXPLOITER_TAG),
                ObjectFilter::source(),
            )),
        ));
    }

    if let Some(zone) = source_zone_from_words(&filtered) {
        return Ok(PredicateAst::SourceIsInZone(zone));
    }

    if let Some(predicate) = parse_source_exiled_with_counter_predicate(&raw_words, tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_happily_style_conjoined_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_revealed_or_controlled_subtype_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_graveyard_threshold_predicate(&filtered)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_card_in_your_graveyard_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_object_on_battlefield_predicate(tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_colors_among_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_card_types_among_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_life_total_at_least_starting_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_status_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_counted_objects_have_counter_predicate(&filtered) {
        return Ok(predicate);
    }

    if filtered.len() >= 4 && filtered.get(0..2) == Some(&["you", "have"]) {
        let tail_words = &filtered[2..];
        if tail_words.last().copied() == Some("life") {
            let quantity_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(
                &tail_words[..tail_words.len() - 1],
            );
            if let Some((amount, used)) = parse_less_than_or_equal_quantity_prefix(
                &quantity_tokens,
                false,
                false,
                "life-total predicate",
            )
            .ok()
            .flatten()
                && used == tail_words.len() - 1
            {
                return Ok(PredicateAst::ValueComparison {
                    left: Value::LifeTotal(PlayerFilter::You),
                    operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
                    right: Value::Fixed(amount as i32),
                });
            }
        }
    }
    if filtered.len() >= 6 && filtered.get(0..4) == Some(&["your", "life", "total", "is"]) {
        let quantity_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&filtered[4..]);
        if let Some((amount, used)) = parse_less_than_or_equal_quantity_prefix(
            &quantity_tokens,
            false,
            false,
            "life-total predicate",
        )
        .ok()
        .flatten()
            && used == filtered.len() - 4
        {
            return Ok(PredicateAst::ValueComparison {
                left: Value::LifeTotal(PlayerFilter::You),
                operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
                right: Value::Fixed(amount as i32),
            });
        }
    }

    if let Some(has_idx) = find_index(&filtered, |word| {
        HAS_OR_HAVE_WORD_PATTERN.matches_word(word)
    }) && has_idx > 0
        && has_idx + 1 < filtered.len()
        && filtered[..has_idx]
            .iter()
            .any(|word| CONTROL_WORD_PATTERN.matches_word(word))
        && let Some((constraint, consumed)) =
            parse_filter_keyword_constraint_words(&filtered[has_idx + 1..])
        && has_idx + 1 + consumed == filtered.len()
    {
        let mut subject_words = filtered[..has_idx].to_vec();
        subject_words.retain(|word| {
            !YOU_WORD_PATTERN.matches_word(word)
                && !CONTROL_OR_CONTROLS_WORD_PATTERN.matches_word(word)
        });
        let subject_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(subject_words);
        let mut filter = parse_object_filter(&subject_tokens, false)?;
        apply_filter_keyword_constraint(&mut filter, constraint, false);
        filter.controller = Some(PlayerFilter::You);
        return Ok(PredicateAst::PlayerControls {
            player: PlayerAst::You,
            filter,
        });
    }

    if let Some(has_idx) = find_index(&filtered, |word| {
        HAS_OR_HAVE_WORD_PATTERN.matches_word(word)
    }) && has_idx > 0
        && has_idx + 1 < filtered.len()
        && filtered[..has_idx]
            .iter()
            .any(|word| ZONE_WORD_PATTERN.matches_word(word))
        && let Some((constraint, consumed)) =
            parse_filter_keyword_constraint_words(&filtered[has_idx + 1..])
        && has_idx + 1 + consumed == filtered.len()
    {
        let subject_tokens =
            crate::runtime_backend::lexer::synthetic_word_tokens(&filtered[..has_idx]);
        let mut filter = parse_object_filter(&subject_tokens, false)?;
        apply_filter_keyword_constraint(&mut filter, constraint, false);
        if filter.owner.is_none() {
            filter.owner = Some(PlayerFilter::You);
        }
        return Ok(PredicateAst::PlayerControls {
            player: PlayerAst::You,
            filter,
        });
    }

    if OPPONENT_CONTROLS_IT_PATTERN.matches_words(&filtered) {
        let mut filter = ObjectFilter {
            controller: Some(PlayerFilter::Opponent),
            ..Default::default()
        };
        if filtered
            .last()
            .is_some_and(|word| CREATURE_WORD_PATTERN.matches_word(word))
        {
            filter.card_types.push(CardType::Creature);
        }
        return Ok(PredicateAst::ItMatches(filter));
    }

    if filtered.len() >= 3
        && OPPONENT_CONTROLS_PREFIX_PATTERN.matches_words(&filtered)
        && !(filtered[2] == "more" && word_slice_contains_word(&filtered[3..], "than"))
    {
        let control_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&filtered[2..]);
        let other = control_tokens
            .first()
            .is_some_and(|token| OTHER_OR_ANOTHER_WORD_PATTERN.matches_token(token));
        if let Ok(mut filter) = parse_object_filter(&control_tokens, other) {
            filter.controller = Some(PlayerFilter::Opponent);
            return Ok(PredicateAst::PlayerControls {
                player: PlayerAst::Opponent,
                filter,
            });
        }
    }

    if raw_words.len() >= 4
        && AN_OPPONENT_CONTROLS_PREFIX_PATTERN.matches_words(&raw_words)
        && !(raw_words[3] == "more" && word_slice_contains_word(&raw_words[4..], "than"))
    {
        let control_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&raw_words[3..]);
        let other = control_tokens
            .first()
            .is_some_and(|token| OTHER_OR_ANOTHER_WORD_PATTERN.matches_token(token));
        if let Ok(mut filter) = parse_object_filter(&control_tokens, other) {
            filter.controller = Some(PlayerFilter::Opponent);
            return Ok(PredicateAst::PlayerControls {
                player: PlayerAst::Opponent,
                filter,
            });
        }
    }

    if let Some(gets_idx) = find_index(&filtered, |word| GETS_WORD_PATTERN.matches_word(word))
        && gets_idx > 0
        && MORE_VOTES_TAIL_PATTERN.matches_words(&filtered[gets_idx + 1..])
    {
        return Ok(PredicateAst::VoteOptionGetsMoreVotes {
            option: filtered[..gets_idx].join(" "),
        });
    }

    if filtered.len() >= 4 && NO_WORDS_GOT_VOTES_PATTERN.matches_words(&filtered) {
        let filter_tokens =
            crate::runtime_backend::lexer::synthetic_word_tokens(&filtered[1..filtered.len() - 2]);
        let filter = parse_object_filter(&filter_tokens, false)?;
        return Ok(PredicateAst::NoVoteObjectsMatched { filter });
    }

    if let Some(attacking_idx) = (0..filtered.len())
        .find(|idx| MELD_ATTACKING_OWN_CONTROL_TAIL_PATTERN.matches_words(&filtered[*idx..]))
        && let Some(and_idx) = find_meld_subject_split(&filtered[..attacking_idx])
    {
        let left_words = &filtered[..and_idx];
        let right_words = &filtered[and_idx + 1..attacking_idx];
        if !left_words.is_empty() && !right_words.is_empty() {
            let mut left_filter = parse_meld_subject_filter(left_words).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported attacking meld predicate subject (predicate: '{}')",
                    filtered.join(" ")
                ))
            })?;
            left_filter.controller = Some(PlayerFilter::You);
            left_filter.attacking = true;

            let mut right_filter = parse_meld_subject_filter(right_words).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported attacking meld predicate tail (predicate: '{}')",
                    filtered.join(" ")
                ))
            })?;
            right_filter.controller = Some(PlayerFilter::You);
            right_filter.attacking = true;

            return Ok(PredicateAst::And(
                Box::new(PredicateAst::PlayerControls {
                    player: PlayerAst::You,
                    filter: left_filter,
                }),
                Box::new(PredicateAst::PlayerControls {
                    player: PlayerAst::You,
                    filter: right_filter,
                }),
            ));
        }
    }

    if filtered.len() >= 8
        && YOU_BOTH_OWN_AND_CONTROL_PREFIX_PATTERN.matches_words(&filtered)
        && filtered
            .get(4)
            .is_some_and(|word| CONTROL_OR_CONTROLS_WORD_PATTERN.matches_word(word))
        && let Some(and_idx) = find_meld_subject_split(&filtered[5..])
    {
        let and_idx = 5 + and_idx;
        if and_idx > 5 && and_idx + 1 < filtered.len() {
            let mut left_filter =
                parse_meld_subject_filter(&filtered[5..and_idx]).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported own-and-control predicate subject (predicate: '{}')",
                        filtered.join(" ")
                    ))
                })?;
            left_filter.controller = Some(PlayerFilter::You);
            let mut right_filter =
                parse_meld_subject_filter(&filtered[and_idx + 1..]).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported own-and-control predicate tail (predicate: '{}')",
                        filtered.join(" ")
                    ))
                })?;
            right_filter.controller = Some(PlayerFilter::You);
            return Ok(PredicateAst::And(
                Box::new(PredicateAst::PlayerControls {
                    player: PlayerAst::You,
                    filter: left_filter,
                }),
                Box::new(PredicateAst::PlayerControls {
                    player: PlayerAst::You,
                    filter: right_filter,
                }),
            ));
        }
    }

    if let Some(and_idx) = find_index(&filtered, |word| AND_WORD_PATTERN.matches_word(word))
        && and_idx > 0
        && and_idx + 1 < filtered.len()
    {
        let right_first = filtered.get(and_idx + 1).copied();
        if right_first.is_some_and(|word| {
            HAVE_WORD_PATTERN.matches_word(word) || YOU_WORD_PATTERN.matches_word(word)
        }) {
            let left_words = &filtered[..and_idx];
            let mut right_words = filtered[and_idx + 1..].to_vec();
            if right_words
                .first()
                .is_some_and(|word| HAVE_WORD_PATTERN.matches_word(word))
            {
                right_words.insert(0, "you");
            }
            let left_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(left_words);
            let right_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(right_words);
            let left = parse_predicate(&left_tokens)?;
            let right = parse_predicate(&right_tokens)?;
            return Ok(PredicateAst::And(Box::new(left), Box::new(right)));
        }
    }

    if let Some(while_idx) = find_index(&filtered, |word| WHILE_WORD_PATTERN.matches_word(word))
        && while_idx > 0
        && while_idx + 1 < filtered.len()
    {
        let left_tokens =
            crate::runtime_backend::lexer::synthetic_word_tokens(&filtered[..while_idx]);
        let right_tokens =
            crate::runtime_backend::lexer::synthetic_word_tokens(&filtered[while_idx + 1..]);
        let left = parse_predicate(&left_tokens)?;
        let right = parse_predicate(&right_tokens)?;
        if matches!(
            left,
            PredicateAst::ManaSpentToCastThisSpellAtLeast { .. }
                | PredicateAst::SameColorManaSpentToCastThisSpellAtLeast(_)
        ) {
            return Err(CardTextError::ParseError(format!(
                "unsupported mana-spent predicate tail (predicate: '{}')",
                filtered.join(" ")
            )));
        }
        return Ok(PredicateAst::And(Box::new(left), Box::new(right)));
    }

    if SOURCE_TAPPED_PATTERN.matches_words(&filtered)
        || (filtered
            .first()
            .is_some_and(|word| SOURCE_OR_SOURCE_POSSESSIVE_WORD_PATTERN.matches_word(word))
            && filtered
                .last()
                .is_some_and(|word| TAPPED_WORD_PATTERN.matches_word(word)))
    {
        return Ok(PredicateAst::SourceIsTapped);
    }

    if SOURCE_UNTAPPED_PATTERN.matches_words(&filtered)
        || (filtered
            .first()
            .is_some_and(|word| SOURCE_OR_SOURCE_POSSESSIVE_WORD_PATTERN.matches_word(word))
            && filtered
                .last()
                .is_some_and(|word| UNTAPPED_WORD_PATTERN.matches_word(word)))
    {
        return Ok(PredicateAst::Not(Box::new(PredicateAst::SourceIsTapped)));
    }

    if SOURCE_NOT_SADDLED_PATTERN.matches_words(&filtered) {
        return Ok(PredicateAst::Not(Box::new(PredicateAst::SourceIsSaddled)));
    }

    if SOURCE_SADDLED_PATTERN.matches_words(&filtered) {
        return Ok(PredicateAst::SourceIsSaddled);
    }

    if let Some(is_idx) = find_index(&filtered, |word| IS_OR_ARE_WORD_PATTERN.matches_word(word)) {
        let subject_words = &filtered[..is_idx];
        let is_source_subject = is_source_reference_words(subject_words)
            || SOURCE_REFERENCE_WORD_PATTERN.matches_words(subject_words);
        if is_source_subject && ENCHANTED_BY_PREFIX_PATTERN.matches_words(&filtered[is_idx + 1..]) {
            let attachment_tokens =
                crate::runtime_backend::lexer::synthetic_word_tokens(&filtered[is_idx + 3..]);
            let (comparison, used) = parse_attachment_quantity_prefix(&attachment_tokens)?;
            let filter_tokens = &attachment_tokens[used..];
            if !filter_tokens.is_empty() {
                let filter = parse_object_filter(filter_tokens, false).or_else(|_| {
                    let filter_words = crate::runtime_backend::token_word_refs(filter_tokens);
                    if AURA_WORD_PATTERN.matches_words(&filter_words) {
                        Ok(ObjectFilter::default().with_subtype(Subtype::Aura))
                    } else {
                        Err(CardTextError::ParseError(format!(
                            "unsupported attachment-count predicate tail (predicate: '{}')",
                            filtered.join(" ")
                        )))
                    }
                })?;
                return Ok(PredicateAst::SourceHasAttachmentsMatching {
                    filter,
                    comparison,
                    display: filtered.join(" "),
                });
            }
        }
    }

    let source_filter_predicate = {
        let predicate_idx = find_index(&filtered, |word| {
            SOURCE_FILTER_STATE_WORD_PATTERN.matches_word(word)
        });
        predicate_idx.and_then(|idx| {
            let subject_words = &filtered[..idx];
            let is_source_subject = is_source_reference_words(subject_words);
            if !is_source_subject {
                return None;
            }

            let mut negative = NEGATED_STATE_WORD_PATTERN.matches_word(filtered[idx]);
            let mut tail_start = idx + 1;
            if filtered
                .get(tail_start)
                .is_some_and(|word| NOT_WORD_PATTERN.matches_word(word))
            {
                negative = true;
                tail_start += 1;
            }
            let descriptor_words = &filtered[tail_start..];
            if descriptor_words.is_empty()
                || descriptor_words
                    .iter()
                    .any(|word| SOURCE_FILTER_IGNORED_DESCRIPTOR_WORD_PATTERN.matches_word(word))
            {
                return None;
            }

            let descriptor_tokens =
                crate::runtime_backend::lexer::synthetic_word_tokens(descriptor_words);
            let Ok(filter) = parse_object_filter(&descriptor_tokens, false) else {
                return None;
            };
            let has_identity = !filter.card_types.is_empty()
                || !filter.all_card_types.is_empty()
                || !filter.subtypes.is_empty()
                || !filter.supertypes.is_empty()
                || filter.colors.is_some()
                || filter.token
                || filter.nontoken
                || !filter.excluded_card_types.is_empty()
                || !filter.excluded_subtypes.is_empty();
            has_identity.then_some((filter, negative))
        })
    };
    if let Some((filter, negative)) = source_filter_predicate {
        let predicate = PredicateAst::SourceMatches(filter);
        return Ok(if negative {
            PredicateAst::Not(Box::new(predicate))
        } else {
            predicate
        });
    }

    if let Some(has_idx) = find_index(&filtered, |word| {
        HAS_OR_HAVE_WORD_PATTERN.matches_word(word)
    }) && has_idx > 0
        && has_idx + 1 < filtered.len()
    {
        let subject_words = &filtered[..has_idx];
        let is_source_subject = is_source_reference_words(subject_words)
            || SOURCE_REFERENCE_WORD_PATTERN.matches_words(subject_words);
        if is_source_subject
            && let Some((constraint, consumed)) =
                parse_filter_keyword_constraint_words(&filtered[has_idx + 1..])
            && has_idx + 1 + consumed == filtered.len()
        {
            let mut filter = ObjectFilter::default();
            apply_filter_keyword_constraint(&mut filter, constraint, false);
            return Ok(PredicateAst::SourceMatches(filter));
        }
    }

    if SOURCE_DIDNT_ATTACK_OR_ENTER_CONTROL_PATTERN.matches_words(&filtered) {
        return Ok(PredicateAst::And(
            Box::new(PredicateAst::Not(Box::new(
                PredicateAst::SourceAttackedThisTurn,
            ))),
            Box::new(PredicateAst::Not(Box::new(
                PredicateAst::SourceCameUnderYourControlThisTurn,
            ))),
        ));
    }

    if THERE_ARE_NO_COUNTERS_ON_SOURCE_PATTERN.matches_words(&filtered)
        && let Some(counters_idx) = find_index(&raw_words, |word| {
            COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(word)
        })
        && counters_idx >= 4
        && let Some(counter_type) = parse_counter_type_from_tokens(&tokens[..=counters_idx])
    {
        return Ok(PredicateAst::SourceHasNoCounter(counter_type));
    }

    let source_has_counter_prefix_len = if THIS_HAS_PREFIX_PATTERN.matches_words(&raw_words) {
        Some(2)
    } else if raw_words.len() >= 3 && THIS_TYPED_HAS_PREFIX_PATTERN.matches_words(&raw_words) {
        Some(3)
    } else {
        None
    };
    if let Some(prefix_len) = source_has_counter_prefix_len
        && raw_words.len() >= prefix_len + 4
        && NO_WORD_PATTERN.matches_word(raw_words[prefix_len])
        && let Some(counter_type) = parse_counter_type_word(raw_words[prefix_len + 1])
        && COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(raw_words[prefix_len + 2])
        && raw_words
            .get(prefix_len + 3..)
            .is_some_and(|tail| COUNTER_ON_SOURCE_PRONOUN_TAIL_PATTERN.matches_words(tail))
    {
        return Ok(PredicateAst::SourceHasNoCounter(counter_type));
    }

    if let Some(prefix_len) = source_has_counter_prefix_len
        && raw_words.len() >= prefix_len + 4
        && !OR_MORE_PREFIX_PATTERN.matches_words(&raw_words[prefix_len + 1..])
        && let Some(counter_idx) = find_index(&raw_words[prefix_len..], |word| {
            COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(word)
        })
        && counter_idx > 0
        && let Some(counter_type) =
            parse_counter_type_from_tokens(&tokens[prefix_len..=prefix_len + counter_idx])
        && raw_words
            .get(prefix_len + counter_idx + 1..)
            .is_some_and(|tail| COUNTER_ON_SOURCE_PRONOUN_TAIL_PATTERN.matches_words(tail))
    {
        return Ok(PredicateAst::SourceHasCounterAtLeast {
            counter_type,
            count: 1,
        });
    }

    let triggering_object_had_no_counter_prefix_len =
        if IT_HAD_NO_COUNTER_PREFIX_PATTERN.matches_words(&raw_words) {
            Some(3)
        } else if TYPED_OBJECT_HAD_NO_COUNTER_PREFIX_PATTERN.matches_words(&raw_words) {
            Some(4)
        } else {
            None
        };
    if let Some(prefix_len) = triggering_object_had_no_counter_prefix_len
        && raw_words.len() >= prefix_len + 4
        && let Some(counter_type) = parse_counter_type_word(raw_words[prefix_len])
        && COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(raw_words[prefix_len + 1])
        && raw_words
            .get(prefix_len + 2..)
            .is_some_and(|tail| COUNTER_ON_TRIGGERING_OBJECT_TAIL_PATTERN.matches_words(tail))
    {
        return Ok(PredicateAst::TriggeringObjectHadNoCounter(counter_type));
    }

    let triggering_object_had_counter_prefix_len =
        if IT_HAD_COUNTER_PREFIX_PATTERN.matches_words(&raw_words) {
            Some(2)
        } else if TYPED_OBJECT_HAD_COUNTER_PREFIX_PATTERN.matches_words(&raw_words) {
            Some(3)
        } else {
            None
        };
    if let Some(prefix_len) = triggering_object_had_counter_prefix_len
        && raw_words.len() >= prefix_len + 4
        && let Some(counter_idx) = find_index(&raw_words[prefix_len..], |word| {
            COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(word)
        })
        && counter_idx > 0
        && let Some(counter_type) =
            parse_counter_type_from_tokens(&tokens[prefix_len..=prefix_len + counter_idx])
        && raw_words
            .get(prefix_len + counter_idx + 1..)
            .is_some_and(|tail| COUNTER_ON_TRIGGERING_OBJECT_TAIL_PATTERN.matches_words(tail))
    {
        return Ok(PredicateAst::TriggeringObjectHadCounterAtLeast {
            counter_type,
            count: 1,
        });
    }

    if THERE_ARE_PREFIX_PATTERN.matches_words(&raw_words)
        && raw_words
            .iter()
            .any(|w| COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(w))
        && let Some((comparison, used)) = predicate_quantity_prefix(&raw_words[2..])
        && let Some(count) = comparison_to_at_least_threshold(&comparison)
    {
        let rest = &tokens[2 + used..];
        let rest_words = crate::runtime_backend::token_word_refs(rest);
        if let Some(counter_idx) = find_index(rest_words.as_slice(), |word| {
            COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(word)
        }) {
            let consumed_source_tail =
                COUNTER_ON_SOURCE_TAIL_ANY_PATTERN.matches_words(&rest_words[counter_idx + 1..]);
            if counter_idx == 0 && consumed_source_tail {
                return Ok(PredicateAst::SourceHasCountersAtLeast(count));
            }
            if counter_idx > 0
                && let Some(counter_type) = parse_counter_type_from_tokens(&rest[..=counter_idx])
                && consumed_source_tail
            {
                return Ok(PredicateAst::SourceHasCounterAtLeast {
                    counter_type,
                    count,
                });
            }
        }
    }

    if let Some(prefix_len) = source_has_counter_prefix_len
        && raw_words.len() >= prefix_len + 6
        && let Some((comparison, used)) = predicate_quantity_prefix(&raw_words[prefix_len..])
        && let Some(count) = comparison_to_at_least_threshold(&comparison)
        && let Some(counter_idx) = find_index(&raw_words[prefix_len + used..], |word| {
            COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(word)
        })
        && counter_idx > 0
        && let Some(counter_type) = parse_counter_type_from_tokens(
            &tokens[prefix_len + used..=prefix_len + used + counter_idx],
        )
        && raw_words
            .get(prefix_len + used + counter_idx + 1..)
            .is_some_and(|tail| COUNTER_ON_SOURCE_PRONOUN_TAIL_PATTERN.matches_words(tail))
    {
        return Ok(PredicateAst::SourceHasCounterAtLeast {
            counter_type,
            count,
        });
    }

    if filtered.len() == 7
        && SOURCE_POWER_IS_PREFIX_PATTERN.matches_words(&filtered)
        && let Some((comparison, used)) = predicate_quantity_prefix(&filtered[4..])
        && used == filtered.len() - 4
        && let Some(count) = comparison_to_at_least_threshold(&comparison)
    {
        return Ok(PredicateAst::SourcePowerAtLeast(count));
    }

    if filtered.len() == 6
        && SOURCE_HAS_POWER_PREFIX_PATTERN.matches_words(&filtered)
        && let Some((comparison, used)) = predicate_quantity_prefix(&filtered[3..])
        && used == filtered.len() - 3
        && let Some(count) = comparison_to_at_least_threshold(&comparison)
    {
        return Ok(PredicateAst::SourcePowerAtLeast(count));
    }

    if filtered.len() >= 10 && THERE_ARE_PREFIX_PATTERN.matches_words(&filtered) {
        if let Some((comparison, idx)) = predicate_quantity_prefix(&filtered[2..])
            .map(|(comparison, used)| (comparison, 2 + used))
            && let Some(count) = comparison_to_at_least_threshold(&comparison)
        {
            let looks_like_basic_land_type_clause =
                BASIC_LAND_TYPES_AMONG_LANDS_PREFIX_PATTERN.matches_words(&filtered[idx..]);
            if looks_like_basic_land_type_clause {
                let tail = &filtered[idx + 5..];
                let player = if THAT_PLAYER_CONTROLS_TAIL_PATTERN.matches_words(tail) {
                    PlayerAst::That
                } else if YOU_CONTROL_TAIL_PATTERN.matches_words(tail) {
                    PlayerAst::You
                } else {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported basic-land-types predicate tail (predicate: '{}')",
                        filtered.join(" ")
                    )));
                };

                return Ok(PredicateAst::PlayerControlsBasicLandTypesAmongLandsOrMore {
                    player,
                    count,
                });
            }
        }
    }

    if filtered.len() >= 7
        && THERE_ARE_PREFIX_PATTERN.matches_words(&filtered)
        && let Some((count, idx)) = predicate_at_least_quantity_prefix(&filtered[2..])
            .map(|(count, used)| (count, 2 + used))
    {
        let battlefield_suffix_len =
            if ON_BATTLEFIELD_SUFFIX_PATTERN.matches_words(&filtered[idx..]) {
                if ON_THE_BATTLEFIELD_SUFFIX_PATTERN.matches_words(&filtered) {
                    Some(3usize)
                } else {
                    Some(2usize)
                }
            } else {
                None
            };
        if let Some(battlefield_suffix_len) = battlefield_suffix_len {
            let raw_filter_words = &filtered[idx..filtered.len() - battlefield_suffix_len];
            let other = raw_filter_words
                .first()
                .is_some_and(|word| OTHER_OR_ANOTHER_WORD_PATTERN.matches_word(word));
            let filter_words = if other {
                &raw_filter_words[1..]
            } else {
                raw_filter_words
            };
            if !filter_words.is_empty() {
                let filter_tokens =
                    crate::runtime_backend::lexer::synthetic_word_tokens(filter_words);
                if let Ok(mut filter) = parse_object_filter(&filter_tokens, other) {
                    filter.zone = Some(Zone::Battlefield);

                    return Ok(PredicateAst::ValueComparison {
                        left: Value::Count(filter),
                        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                        right: Value::Fixed(count as i32),
                    });
                }
            }
        }
    }

    let parse_graveyard_card_types_subject = |words: &[&str]| -> Option<PlayerAst> {
        if YOUR_GRAVEYARD_PATTERN.matches_words(words) {
            Some(PlayerAst::You)
        } else if THAT_PLAYER_GRAVEYARD_PATTERN.matches_words(words) {
            Some(PlayerAst::That)
        } else if TARGET_PLAYER_GRAVEYARD_PATTERN.matches_words(words) {
            Some(PlayerAst::Target)
        } else if TARGET_OPPONENT_GRAVEYARD_PATTERN.matches_words(words) {
            Some(PlayerAst::TargetOpponent)
        } else if OPPONENT_GRAVEYARD_PATTERN.matches_words(words) {
            Some(PlayerAst::Opponent)
        } else {
            None
        }
    };
    if filtered.len() >= 11 {
        let (count_idx, subject_start, constrained_player) =
            if THERE_ARE_PREFIX_PATTERN.matches_words(&filtered) {
                (2usize, 10usize, None)
            } else if YOU_HAVE_PREFIX_PATTERN.matches_words(&filtered) {
                (2usize, 10usize, Some(PlayerAst::You))
            } else {
                (usize::MAX, usize::MAX, None)
            };
        if count_idx != usize::MAX
            && let Some((count, used)) = predicate_at_least_quantity_prefix(&filtered[count_idx..])
            && CARD_TYPES_AMONG_CARDS_IN_PREFIX_PATTERN.matches_words(&filtered[count_idx + used..])
            && subject_start <= filtered.len()
            && let Some(player) = parse_graveyard_card_types_subject(&filtered[subject_start..])
            && constrained_player.map_or(true, |expected| expected == player)
        {
            return Ok(PredicateAst::PlayerHasCardTypesInGraveyardOrMore { player, count });
        }
    }

    let parse_comparison_player_subject = |words: &[&str]| -> Option<(PlayerAst, usize)> {
        if THAT_PLAYER_SUBJECT_PREFIX_PATTERN.matches_words(words) {
            Some((PlayerAst::That, 2))
        } else if TARGET_PLAYER_SUBJECT_PREFIX_PATTERN.matches_words(words) {
            Some((PlayerAst::Target, 2))
        } else if TARGET_OPPONENT_SUBJECT_PREFIX_PATTERN.matches_words(words) {
            Some((PlayerAst::TargetOpponent, 2))
        } else if EACH_OPPONENT_SUBJECT_PREFIX_PATTERN.matches_words(words) {
            Some((PlayerAst::Opponent, 2))
        } else if A_OR_ANY_PLAYER_SUBJECT_PREFIX_PATTERN.matches_words(words) {
            Some((PlayerAst::Any, 2))
        } else if DEFENDING_PLAYER_SUBJECT_PREFIX_PATTERN.matches_words(words) {
            Some((PlayerAst::Defending, 2))
        } else if ATTACKING_PLAYER_SUBJECT_PREFIX_PATTERN.matches_words(words) {
            Some((PlayerAst::Attacking, 2))
        } else if words
            .first()
            .is_some_and(|word| YOU_WORD_PATTERN.matches_word(word))
        {
            Some((PlayerAst::You, 1))
        } else if OPPONENT_SUBJECT_PREFIX_PATTERN.matches_words(words) {
            Some((PlayerAst::Opponent, 1))
        } else if PLAYER_WHO_SUBJECT_PREFIX_PATTERN.matches_words(words) {
            Some((PlayerAst::That, 1))
        } else if words
            .first()
            .is_some_and(|word| PLAYER_SUBJECT_WORD_PATTERN.matches_word(word))
        {
            Some((PlayerAst::Any, 1))
        } else {
            None
        }
    };
    let parse_life_total_subject = |words: &[&str]| -> Option<(PlayerAst, usize)> {
        if YOUR_LIFE_TOTAL_PREFIX_PATTERN.matches_words(words) {
            Some((PlayerAst::You, 3))
        } else if THEIR_LIFE_TOTAL_PREFIX_PATTERN.matches_words(words) {
            Some((PlayerAst::That, 3))
        } else if THAT_PLAYERS_LIFE_TOTAL_PREFIX_PATTERN.matches_words(words) {
            Some((PlayerAst::That, 4))
        } else if TARGET_PLAYERS_LIFE_TOTAL_PREFIX_PATTERN.matches_words(words) {
            Some((PlayerAst::Target, 4))
        } else if TARGET_OPPONENTS_LIFE_TOTAL_PREFIX_PATTERN.matches_words(words) {
            Some((PlayerAst::TargetOpponent, 4))
        } else if OPPONENT_LIFE_TOTAL_PREFIX_PATTERN.matches_words(words) {
            Some((PlayerAst::Opponent, 3))
        } else if DEFENDING_PLAYERS_LIFE_TOTAL_PREFIX_PATTERN.matches_words(words) {
            Some((PlayerAst::Defending, 4))
        } else if ATTACKING_PLAYERS_LIFE_TOTAL_PREFIX_PATTERN.matches_words(words) {
            Some((PlayerAst::Attacking, 4))
        } else {
            None
        }
    };
    if let Some((player, subject_len)) = parse_life_total_subject(&filtered)
        && filtered
            .get(subject_len)
            .is_some_and(|word| IS_OR_ARE_WORD_PATTERN.matches_word(word))
    {
        let tail = &filtered[subject_len + 1..];
        if LESS_THAN_OR_EQUAL_TO_PREFIX_PATTERN.matches_words(tail)
            && HALF_STARTING_LIFE_TOTAL_TAIL_PATTERN.matches_words(&tail[5..])
        {
            return Ok(PredicateAst::PlayerLifeAtMostHalfStartingLifeTotal { player });
        }
        if LESS_THAN_PREFIX_PATTERN.matches_words(tail)
            && HALF_STARTING_LIFE_TOTAL_TAIL_PATTERN.matches_words(&tail[2..])
        {
            return Ok(PredicateAst::PlayerLifeLessThanHalfStartingLifeTotal { player });
        }
    }
    if let Some((player, subject_len)) = parse_comparison_player_subject(&filtered)
        && filtered
            .get(subject_len)
            .is_some_and(|word| HAS_OR_HAVE_WORD_PATTERN.matches_word(word))
        && let Some((comparison, used)) = predicate_quantity_prefix(&filtered[subject_len + 1..])
        && let Some((operator, count)) = comparison_to_value_comparison_operator(comparison)
        && filtered
            .get(subject_len + 1 + used)
            .is_some_and(|word| CARD_OR_CARDS_WORD_PATTERN.matches_word(word))
        && filtered
            .get(subject_len + 2 + used)
            .is_some_and(|word| IN_WORD_PATTERN.matches_word(word))
        && let Some(possessive) = filtered.get(subject_len + 3 + used).copied()
        && graveyard_possessive_matches_subject(player, possessive)
        && filtered
            .get(subject_len + 4 + used)
            .is_some_and(|word| GRAVEYARD_WORD_PATTERN.matches_word(word))
        && filtered.len() == subject_len + 5 + used
        && let Some(player_filter) = player_filter_for_turn_value(player)
    {
        return Ok(PredicateAst::ValueComparison {
            left: Value::CardsInGraveyard(player_filter),
            operator,
            right: Value::Fixed(count),
        });
    }
    if let Some((player, subject_len)) = parse_comparison_player_subject(&filtered)
        && filtered
            .get(subject_len)
            .is_some_and(|word| CONTROL_OR_CONTROLS_WORD_PATTERN.matches_word(word))
        && filtered
            .get(subject_len + 1)
            .is_some_and(|word| MORE_WORD_PATTERN.matches_word(word))
        && let Some(than_offset) = find_index(&filtered[subject_len + 2..], |word| {
            THAN_WORD_PATTERN.matches_word(word)
        })
    {
        let than_idx = subject_len + 2 + than_offset;
        let tail = &filtered[than_idx..];
        if THAN_YOU_TAIL_PATTERN.matches_words(tail) {
            let filter_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(
                &filtered[subject_len + 2..than_idx],
            );
            if !filter_tokens.is_empty() {
                let other = filter_tokens
                    .first()
                    .is_some_and(|token| OTHER_OR_ANOTHER_WORD_PATTERN.matches_token(token));
                if let Ok(filter) = parse_object_filter(&filter_tokens, other)
                    && filter != ObjectFilter::default()
                {
                    return Ok(PredicateAst::PlayerControlsMoreThanYou { player, filter });
                }
            }
        }
    }

    if let Some(predicate) = parse_player_life_relation_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_life_total_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_cards_in_hand_relation_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_cards_in_hand_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_turn_event_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_would_action_predicate(&filtered) {
        return Ok(predicate);
    }

    if matches!(
        filtered.as_slice(),
        ["it", "your", "turn"] | ["its", "your", "turn"] | ["your", "turn"]
    ) {
        return Ok(PredicateAst::YourTurn);
    }

    if matches!(
        filtered.as_slice(),
        ["it", "not", "your", "turn"]
            | ["its", "not", "your", "turn"]
            | ["it", "is", "not", "your", "turn"]
            | ["its", "is", "not", "your", "turn"]
            | ["not", "your", "turn"]
    ) {
        return Ok(PredicateAst::Not(Box::new(PredicateAst::YourTurn)));
    }

    if let Some(predicate) = parse_player_life_change_this_turn_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_object_death_this_turn_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_battlefield_change_this_turn_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_battlefield_entry_predicate(&filtered) {
        return Ok(predicate);
    }

    if YOU_ATTACKED_THIS_TURN_PATTERN.matches_words(&filtered) {
        return Ok(PredicateAst::YouAttackedThisTurn);
    }

    if TRIGGERING_OBJECT_HAD_TO_ATTACK_THIS_COMBAT_PATTERN.matches_words(&filtered) {
        return Ok(PredicateAst::TriggeringObjectHadToAttackThisCombat);
    }

    if filtered.len() >= 9
        && YOU_ATTACKED_WITH_EXACTLY_PREFIX_PATTERN.matches_words(&filtered)
        && let Some((count, used)) = predicate_number_prefix(&filtered[4..])
        && OTHER_CREATURES_THIS_COMBAT_TAIL_PATTERN.matches_words(&filtered[4 + used..])
    {
        return Ok(PredicateAst::YouAttackedWithExactlyNOtherCreaturesThisCombat(count));
    }

    if SOURCE_ATTACKED_OR_BLOCKED_THIS_TURN_PATTERN.matches_words(&filtered) {
        return Ok(PredicateAst::SourceAttackedOrBlockedThisTurn);
    }

    if YOU_CAST_SOURCE_PATTERN.matches_words(&filtered) {
        return Ok(PredicateAst::SourceWasCast);
    }
    if TAGGED_WAS_CAST_PATTERN.matches_words(&filtered) {
        return Ok(PredicateAst::TaggedWasCast(TagKey::from(IT_TAG)));
    }

    if filtered.len() >= 6 && THIS_SPELL_WAS_CAST_FROM_PREFIX_PATTERN.matches_words(&filtered) {
        let zone_words = &filtered[5..];
        if zone_words == ["anywhere", "other", "than", "your", "hand"] {
            return Ok(PredicateAst::ThisSpellWasCastFromNonHand);
        }
        let zone = if zone_words.len() == 1 {
            parse_zone_word(zone_words[0])
        } else if zone_words.len() == 2 && is_article(zone_words[0]) {
            parse_zone_word(zone_words[1])
        } else if zone_words.len() == 2 && DEFINITE_ARTICLE_WORD_PATTERN.matches_word(zone_words[0])
        {
            parse_zone_word(zone_words[1])
        } else {
            None
        };

        if let Some(zone) = zone {
            return Ok(PredicateAst::ThisSpellWasCastFromZone(zone));
        }
    }

    if NO_SPELLS_CAST_LAST_TURN_PATTERN.matches_words(&filtered) {
        return Ok(PredicateAst::NoSpellsWereCastLastTurn);
    }
    if THIS_SPELL_WAS_KICKED_PATTERN.matches_words(&filtered) {
        return Ok(PredicateAst::ThisSpellWasKicked);
    }
    if THIS_SPELL_WAS_BARGAINED_PATTERN.matches_words(&filtered) {
        return Ok(PredicateAst::ThisSpellPaidLabel("Bargain".to_string()));
    }
    if filtered.len() == 4
        && ARTICLE_WORD_PATTERN.matches_word(filtered[0])
        && parse_subtype_word(filtered[1]).is_some()
        && WAS_OR_WERE_WORD_PATTERN.matches_word(filtered[2])
        && BEHELD_WORD_PATTERN.matches_word(filtered[3])
    {
        return Ok(PredicateAst::ThisSpellPaidLabel("Behold".to_string()));
    }
    if filtered.len() == 3
        && parse_subtype_word(filtered[0]).is_some()
        && WAS_OR_WERE_WORD_PATTERN.matches_word(filtered[1])
        && BEHELD_WORD_PATTERN.matches_word(filtered[2])
    {
        return Ok(PredicateAst::ThisSpellPaidLabel("Behold".to_string()));
    }
    if GIFT_PROMISED_PATTERN.matches_words(&filtered) {
        return Ok(PredicateAst::ThisSpellPaidLabel("Gift".to_string()));
    }
    if GIFT_NOT_PROMISED_PATTERN.matches_words(&filtered) {
        return Ok(PredicateAst::Not(Box::new(
            PredicateAst::ThisSpellPaidLabel("Gift".to_string()),
        )));
    }
    if filtered.len() >= 4
        && COST_WAS_PAID_TAIL_PATTERN.matches_words(&filtered[filtered.len() - 3..])
    {
        let start = usize::from(DEFINITE_ARTICLE_WORD_PATTERN.matches_word_at(&filtered, 0));
        if let Some(label) = mana_cost_label_from_words(&filtered[start..filtered.len() - 3]) {
            return Ok(PredicateAst::ThisSpellPaidLabel(label));
        }
    }
    if filtered.len() >= 4
        && COST_WASNT_PAID_TAIL_PATTERN.matches_words(&filtered[filtered.len() - 3..])
    {
        let start = usize::from(DEFINITE_ARTICLE_WORD_PATTERN.matches_word_at(&filtered, 0));
        if let Some(label) = mana_cost_label_from_words(&filtered[start..filtered.len() - 3]) {
            return Ok(PredicateAst::Not(Box::new(
                PredicateAst::ThisSpellPaidLabel(label),
            )));
        }
    }
    if filtered.len() >= 5
        && COST_WAS_NOT_PAID_TAIL_PATTERN.matches_words(&filtered[filtered.len() - 4..])
    {
        let start = usize::from(DEFINITE_ARTICLE_WORD_PATTERN.matches_word_at(&filtered, 0));
        if let Some(label) = mana_cost_label_from_words(&filtered[start..filtered.len() - 4]) {
            return Ok(PredicateAst::Not(Box::new(
                PredicateAst::ThisSpellPaidLabel(label),
            )));
        }
    }
    if filtered.len() == 6
        && THIS_POSSESSIVE_PAID_LABEL_PATTERN.matches_words(&filtered)
        && THIS_POSSESSIVE_PAID_SUBJECT_WORD_PATTERN.matches_word(filtered[1])
    {
        let mut chars = filtered[2].chars();
        let Some(first) = chars.next() else {
            return Err(CardTextError::ParseError(
                "missing paid-cost label in predicate".to_string(),
            ));
        };
        let label = format!(
            "{}{}",
            first.to_ascii_uppercase(),
            chars.as_str().to_ascii_lowercase()
        );
        return Ok(PredicateAst::ThisSpellPaidLabel(label));
    }
    if IT_WAS_KICKED_PATTERN.matches_words(&filtered) {
        return Ok(PredicateAst::ThisSpellWasKicked);
    }
    if THAT_WAS_KICKED_PATTERN.matches_words(&filtered) {
        return Ok(PredicateAst::TargetWasKicked);
    }

    if let Some(predicate) = parse_spell_context_predicate(&filtered) {
        return Ok(predicate);
    }
    if filtered.len() == 7
        && MANA_SYMBOL_WORD_PATTERN.matches_word(filtered[0])
        && MANA_SPENT_TO_CAST_THIS_SPELL_TAIL_PATTERN.matches_words(&filtered[1..])
        && let Ok(symbol) = parse_mana_symbol(filtered[0])
    {
        return Ok(PredicateAst::ManaSpentToCastThisSpellAtLeast {
            amount: 1,
            symbol: Some(symbol),
        });
    }
    if filtered.len() >= 8
        && MANA_SPENT_TO_CAST_THIS_SPELL_TAIL_PATTERN.matches_words(&filtered[filtered.len() - 6..])
        && filtered[..filtered.len() - 6]
            .iter()
            .all(|word| MANA_SYMBOL_WORD_PATTERN.matches_word(word))
    {
        let mut predicates = filtered[..filtered.len() - 6]
            .iter()
            .filter_map(|word| parse_mana_symbol(word).ok())
            .map(|symbol| PredicateAst::ManaSpentToCastThisSpellAtLeast {
                amount: 1,
                symbol: Some(symbol),
            });
        if let Some(first) = predicates.next() {
            return Ok(predicates.fold(first, |left, right| {
                PredicateAst::And(Box::new(left), Box::new(right))
            }));
        }
    }

    if let Some(amount) = parse_same_color_mana_spent_to_cast_predicate(&filtered) {
        return Ok(PredicateAst::SameColorManaSpentToCastThisSpellAtLeast(
            amount,
        ));
    }

    if let Some((amount, symbol)) = parse_mana_spent_to_cast_predicate(&filtered) {
        return Ok(PredicateAst::ManaSpentToCastThisSpellAtLeast { amount, symbol });
    }

    if filtered.len() >= 5
        && matches!(
            filtered.as_slice(),
            ["this", "permanent", "attached", "to", ..]
                | ["that", "permanent", "attached", "to", ..]
                | ["this", "permanent", "is", "attached", "to", ..]
                | ["that", "permanent", "is", "attached", "to", ..]
        )
    {
        let attached_start = if IS_OR_ARE_WORD_PATTERN.matches_word_at(&filtered, 2) {
            5
        } else {
            4
        };
        let attached_tokens =
            crate::runtime_backend::lexer::synthetic_word_tokens(&filtered[attached_start..]);
        let mut filter = parse_object_filter(&attached_tokens, false)?;
        if filter.card_types.is_empty() {
            filter.card_types.push(CardType::Creature);
        }
        return Ok(PredicateAst::TaggedMatches(
            TagKey::from("enchanted"),
            filter,
        ));
    }

    let sacrificed_idx = if SACRIFICED_WORD_PATTERN.matches_word_at(&filtered, 0) {
        Some(0usize)
    } else if filtered.len() >= 2
        && matches!(filtered[0], "the" | "a" | "an")
        && SACRIFICED_WORD_PATTERN.matches_word_at(&filtered, 1)
    {
        Some(1usize)
    } else {
        None
    };
    if let Some(sacrificed_idx) = sacrificed_idx
        && filtered.len() >= sacrificed_idx + 4
        && WAS_WORD_PATTERN.matches_word_at(&filtered, sacrificed_idx + 2)
    {
        let sacrificed_head = filtered[sacrificed_idx + 1];
        let subject_card_type =
            parse_card_type(sacrificed_head).filter(|card_type| is_permanent_type(*card_type));
        let subject_is_permanent =
            PERMANENT_WORD_PATTERN.matches_word(sacrificed_head) || subject_card_type.is_some();

        if subject_is_permanent {
            let descriptor_tokens =
                crate::runtime_backend::lexer::synthetic_word_tokens(
                    &filtered[sacrificed_idx + 3..],
                );
            let mut filter = match parse_object_filter(&descriptor_tokens, false) {
                Ok(filter) => filter,
                Err(err) => parse_color_only_object_filter_words(&filtered[sacrificed_idx + 3..])
                    .ok_or(err)?,
            };
            if filter.card_types.is_empty() {
                if let Some(card_type) = subject_card_type {
                    filter.card_types.push(card_type);
                }
            }
            if filter.zone.is_none() && PERMANENT_WORD_PATTERN.matches_word(sacrificed_head) {
                filter.zone = Some(Zone::Battlefield);
            }
            return Ok(PredicateAst::ItMatches(filter));
        }
    }

    if matches!(
        filtered.as_slice(),
        ["any", "of", "those", "cards", "remain", "exiled"]
            | ["those", "cards", "remain", "exiled"]
            | ["that", "card", "remains", "exiled"]
            | ["it", "remains", "exiled"]
    ) {
        return Ok(PredicateAst::TaggedMatches(
            TagKey::from(IT_TAG),
            ObjectFilter::default().in_zone(Zone::Exile),
        ));
    }

    if ITS_WORD_PATTERN.matches_word_at(&filtered, 0) {
        filtered[0] = "it";
    }
    if IT_S_PREFIX_PATTERN.matches_words(&filtered) {
        filtered.remove(1);
    }

    let demonstrative_reference_len = if IT_WORD_PATTERN.matches_word_at(&filtered, 0) {
        Some(1usize)
    } else if filtered.len() >= 2
        && THAT_WORD_PATTERN.matches_word_at(&filtered, 0)
        && PREDICATE_REFERENCE_NOUN_WORD_PATTERN.matches_word_at(&filtered, 1)
    {
        Some(2usize)
    } else {
        None
    };

    let is_it_soulbond_paired = matches!(
        filtered.as_slice(),
        ["it", "paired", "with", "creature"]
            | ["it", "paired", "with", "another", "creature"]
            | ["it", "s", "paired", "with", "creature"]
            | ["it", "s", "paired", "with", "another", "creature"]
    );
    if is_it_soulbond_paired {
        return Ok(PredicateAst::ItIsSoulbondPaired);
    }

    if filtered.len() >= 2 {
        let tag = if EQUIPPED_CREATURE_PREFIX_PATTERN.matches_words(&filtered) {
            Some("equipped")
        } else if ENCHANTED_CREATURE_PREFIX_PATTERN.matches_words(&filtered) {
            Some("enchanted")
        } else {
            None
        };
        if let Some(tag) = tag {
            let remainder = filtered[2..].to_vec();
            let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(remainder);
            let mut filter = parse_object_filter(&tokens, false)?;
            if filter.card_types.is_empty() {
                filter.card_types.push(CardType::Creature);
            }
            return Ok(PredicateAst::TaggedMatches(TagKey::from(tag), filter));
        }
    }

    let onto_battlefield_idx = ONTO_BATTLEFIELD_PATTERN.find_exact_window_range(&filtered, 2, 3);
    if filtered.len() >= 7
        && YOU_WORD_PATTERN.matches_word_at(&filtered, 0)
        && PUT_WORD_PATTERN.matches_word_at(&filtered, 1)
        && THIS_WAY_SUFFIX_PATTERN.matches_words(&filtered)
        && let Some(onto_idx) = onto_battlefield_idx
    {
        let filter_words = &filtered[2..onto_idx];
        let filter_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filter_words);
        let mut filter = parse_object_filter(&filter_tokens, false)?;
        if filter.zone.is_none() {
            filter.zone = Some(Zone::Battlefield);
        }
        return Ok(PredicateAst::PlayerTaggedObjectMatches {
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
            filter,
        });
    }

    let is_it = demonstrative_reference_len == Some(1);
    let has_card = demonstrative_reference_len
        .map(|reference_len| {
            filtered[reference_len..]
                .iter()
                .any(|word| CARD_WORD_PATTERN.matches_word(word))
        })
        .unwrap_or(false);

    if is_it {
        if filtered
            .get(1)
            .is_some_and(|word| HAS_OR_HAVE_WORD_PATTERN.matches_word(word))
        {
            filtered.remove(1);
        }
        if filtered
            .get(1..3)
            .is_some_and(|words| MANA_VALUE_HEAD_PATTERN.matches_words(words))
        {
            let mana_value_tail = if filtered
                .get(3)
                .is_some_and(|word| BE_VERB_WORD_PATTERN.matches_word(word))
            {
                &filtered[4..]
            } else {
                &filtered[3..]
            };
            let compares_to_colors_spent =
                COLORS_SPENT_TO_CAST_SOURCE_TAIL_PATTERN.matches_words(mana_value_tail);
            if compares_to_colors_spent {
                return Ok(PredicateAst::TargetManaValueLteColorsSpentToCastThisSpell);
            }

            if let Some((cmp, _consumed)) =
                parse_filter_comparison_tokens("mana value", mana_value_tail, &filtered)?
            {
                return Ok(PredicateAst::ItMatches(ObjectFilter {
                    mana_value: Some(cmp),
                    ..Default::default()
                }));
            }
        }

        if filtered.len() >= 5
            && filtered
                .get(1..5)
                .is_some_and(|words| TOTAL_POWER_TOUGHNESS_HEAD_PATTERN.matches_words(words))
            && let Some((cmp, _consumed)) =
                parse_filter_comparison_tokens("power", &filtered[5..], &filtered)?
        {
            return Ok(PredicateAst::ItMatches(ObjectFilter {
                total_power_toughness: Some(cmp),
                ..Default::default()
            }));
        }

        if filtered.len() >= 3 && POWER_OR_TOUGHNESS_WORD_PATTERN.matches_word(filtered[1]) {
            let axis = filtered[1];
            let value_tail = &filtered[2..];
            if let Some((cmp, _consumed)) =
                parse_filter_comparison_tokens(axis, value_tail, &filtered)?
            {
                let mut filter = ObjectFilter::default();
                if POWER_WORD_PATTERN.matches_word(axis) {
                    filter.power = Some(cmp);
                } else {
                    filter.toughness = Some(cmp);
                }
                return Ok(PredicateAst::ItMatches(filter));
            }
        }
    }

    if demonstrative_reference_len.is_some()
        && filtered
            .iter()
            .any(|word| OR_WORD_PATTERN.matches_word(word))
        && MOST_COMMON_COLOR_AMONG_ALL_PERMANENTS_PATTERN
            .find_exact_window_range(&filtered, 6, 6)
            .is_none()
        && let Some(predicate) = parse_or_predicate(&filtered)?
    {
        return Ok(predicate);
    }

    if let Some(reference_len) = demonstrative_reference_len {
        let mut descriptor_words = filtered[reference_len..].to_vec();
        if descriptor_words.len() >= 2
            && POWER_OR_TOUGHNESS_WORD_PATTERN.matches_word(descriptor_words[0])
        {
            let axis = descriptor_words[0];
            let value_tail = if descriptor_words
                .get(1)
                .is_some_and(|word| BE_VERB_WORD_PATTERN.matches_word(word))
            {
                &descriptor_words[2..]
            } else {
                &descriptor_words[1..]
            };
            if let Some((cmp, _consumed)) =
                parse_filter_comparison_tokens(axis, value_tail, &filtered)?
            {
                let mut filter = ObjectFilter::default();
                if POWER_WORD_PATTERN.matches_word(axis) {
                    filter.power = Some(cmp);
                } else {
                    filter.toughness = Some(cmp);
                }
                return Ok(PredicateAst::ItMatches(filter));
            }
        }
        if HAS_OR_HAVE_TOXIC_PATTERN.matches_words(&descriptor_words) {
            let mut filter = ObjectFilter::default().with_ability_marker("toxic");
            if CREATURE_WORD_PATTERN.matches_word_at(&filtered, 1) {
                filter.card_types.push(CardType::Creature);
            }
            return Ok(PredicateAst::ItMatches(filter));
        }
        if descriptor_words
            .first()
            .is_some_and(|word| IS_OR_ARE_WORD_PATTERN.matches_word(word))
        {
            descriptor_words.remove(0);
        }
        if matches!(
            descriptor_words.as_slice(),
            ["shares", "a", "card", "type", "with", "that", "spell"]
                | ["shares", "card", "type", "with", "that", "spell"]
        ) {
            return Ok(PredicateAst::ItMatches(
                ObjectFilter::default().shares_card_type_with_tagged("triggering"),
            ));
        }
        if matches!(
            descriptor_words.as_slice(),
            [
                "shares",
                "a",
                "color",
                "with",
                "the",
                "most",
                "common",
                "color",
                "among",
                "all",
                "permanents",
                "or",
                "a",
                "color",
                "tied",
                "for",
                "most",
                "common"
            ] | [
                "shares",
                "color",
                "with",
                "most",
                "common",
                "color",
                "among",
                "all",
                "permanents",
                "or",
                "color",
                "tied",
                "for",
                "most",
                "common"
            ]
        ) {
            return Ok(PredicateAst::ItMatches(
                ObjectFilter::default().shares_most_common_permanent_color(),
            ));
        }
        if NOT_TOKEN_PREFIX_PATTERN.matches_words(&descriptor_words) {
            descriptor_words.drain(0..2);
            descriptor_words.insert(0, "nontoken");
        }
        if !descriptor_words.is_empty() {
            if let Some(filter) = parse_single_card_type_card_descriptor(&descriptor_words) {
                return Ok(PredicateAst::ItMatches(filter));
            }
            let descriptor_tokens =
                crate::runtime_backend::lexer::synthetic_word_tokens(descriptor_words);
            if let Ok(filter) = parse_object_filter_lexed(&descriptor_tokens, false)
                && filter != ObjectFilter::default()
            {
                if has_card
                    && filter.card_types.len() == 1
                    && filter.card_types[0] == CardType::Land
                    && filter.subtypes.is_empty()
                    && !filter.nontoken
                    && filter.excluded_card_types.is_empty()
                {
                    return Ok(PredicateAst::ItIsLandCard);
                }
                if THAT_ENCHANTMENT_PREFIX_PATTERN.matches_words(&filtered) {
                    return Ok(PredicateAst::TaggedMatches(
                        TagKey::from("triggering"),
                        filter,
                    ));
                }
                return Ok(PredicateAst::ItMatches(filter));
            }
        }
    }

    if filtered.len() >= 3 && YOU_CONTROL_NO_PREFIX_PATTERN.matches_words(&filtered) {
        let control_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&filtered[3..]);
        if let Ok(mut filter) = parse_object_filter(&control_tokens, false) {
            filter.controller = Some(PlayerFilter::You);
            if NEITHER_WORD_PATTERN.matches_word(filtered[2]) {
                filter = filter
                    .match_tagged(TagKey::from(IT_TAG), TaggedOpbjectRelation::IsTaggedObject);
            }
            return Ok(PredicateAst::PlayerControlsNo {
                player: PlayerAst::You,
                filter,
            });
        }
    }

    if filtered.len() >= 4 && PLAYER_CONTROLS_NO_PREFIX_PATTERN.matches_words(&filtered) {
        let control_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&filtered[3..]);
        if let Ok(mut filter) = parse_object_filter(&control_tokens, false) {
            filter.controller = Some(PlayerFilter::Any);
            return Ok(PredicateAst::PlayerControlsNo {
                player: PlayerAst::Any,
                filter,
            });
        }
    }

    let you_dont_control_filter_start = if filtered.len() >= 4
        && YOU_DONT_CONTROL_PREFIX_PATTERN.matches_words(&filtered)
    {
        Some(3usize)
    } else if filtered.len() >= 5 && YOU_DO_NOT_CONTROL_PREFIX_PATTERN.matches_words(&filtered) {
        Some(4usize)
    } else {
        None
    };
    if let Some(filter_start) = you_dont_control_filter_start {
        let control_tokens =
            crate::runtime_backend::lexer::synthetic_word_tokens(&filtered[filter_start..]);
        let other = control_tokens
            .first()
            .is_some_and(|token| OTHER_OR_ANOTHER_WORD_PATTERN.matches_token(token));
        if let Ok(mut filter) = parse_object_filter(&control_tokens, other) {
            filter.controller = Some(PlayerFilter::You);
            return Ok(PredicateAst::PlayerControlsNo {
                player: PlayerAst::You,
                filter,
            });
        }
    }

    if filtered.len() >= 7
        && YOU_CONTROL_PREFIX_PATTERN.matches_words(&filtered)
        && let Some(or_idx) = find_index(&filtered, |word| OR_WORD_PATTERN.matches_word(word))
        && or_idx > 2
    {
        let left_tokens =
            crate::runtime_backend::lexer::synthetic_word_tokens(&filtered[2..or_idx]);
        let mut right_words = filtered[or_idx + 1..].to_vec();
        if right_words
            .first()
            .is_some_and(|word| THERE_WORD_PATTERN.matches_word(word))
        {
            right_words = right_words[1..].to_vec();
        }
        if YOUR_GRAVEYARD_WORDS_PATTERN.matches_words(&right_words) {
            let right_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(right_words);
            if let (Ok(mut control_filter), Ok(mut graveyard_filter)) = (
                parse_object_filter(&left_tokens, false),
                parse_object_filter(&right_tokens, false),
            ) {
                control_filter.controller = Some(PlayerFilter::You);
                if graveyard_filter.zone.is_none() {
                    graveyard_filter.zone = Some(Zone::Graveyard);
                }
                if graveyard_filter.owner.is_none() {
                    graveyard_filter.owner = Some(PlayerFilter::You);
                }
                return Ok(PredicateAst::PlayerControlsOrHasCardInGraveyard {
                    player: PlayerAst::You,
                    control_filter,
                    graveyard_filter,
                });
            }
        }
    }

    if filtered.len() >= 3 && YOU_CONTROL_PREFIX_PATTERN.matches_words(&filtered) {
        if let Some(and_idx) =
            find_index(&filtered[2..], |word| AND_WORD_PATTERN.matches_word(word))
        {
            let and_idx = 2 + and_idx;
            if and_idx > 2 && and_idx + 1 < filtered.len() {
                let left_tokens =
                    crate::runtime_backend::lexer::synthetic_word_tokens(&filtered[2..and_idx]);
                let right_tokens =
                    crate::runtime_backend::lexer::synthetic_word_tokens(&filtered[and_idx + 1..]);
                if let (Ok(mut left_filter), Ok(mut right_filter)) = (
                    parse_object_filter(&left_tokens, false),
                    parse_object_filter(&right_tokens, false),
                ) {
                    left_filter.controller = Some(PlayerFilter::You);
                    right_filter.controller = Some(PlayerFilter::You);
                    return Ok(PredicateAst::And(
                        Box::new(PredicateAst::PlayerControls {
                            player: PlayerAst::You,
                            filter: left_filter,
                        }),
                        Box::new(PredicateAst::PlayerControls {
                            player: PlayerAst::You,
                            filter: right_filter,
                        }),
                    ));
                }
            }
        }

        if let Some(predicate) = parse_player_controls_predicate(
            &filtered,
            PlayerAst::You,
            Some(PlayerFilter::You),
            2,
            true,
            true,
        )? {
            return Ok(predicate);
        }
    }

    if filtered.len() >= 4 && THAT_PLAYER_CONTROLS_PREFIX_PATTERN.matches_words(&filtered) {
        if let Some(predicate) =
            parse_player_controls_predicate(&filtered, PlayerAst::That, None, 3, false, false)?
        {
            return Ok(predicate);
        }
    }

    if YOU_CONTROLLED_TAGGED_PERMANENT_PATTERN.matches_words(&filtered) {
        return Ok(PredicateAst::PlayerTaggedObjectMatches {
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
            filter: ObjectFilter::default(),
        });
    }

    if TAGGED_ENTERED_UNDER_YOUR_CONTROL_PATTERN.matches_words(&filtered) {
        return Ok(PredicateAst::PlayerTaggedObjectEnteredBattlefieldThisTurn {
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
        });
    }

    if filtered.len() >= 8 && YOU_PUT_ONTO_BATTLEFIELD_THIS_WAY_PATTERN.matches_words(&filtered) {
        let filter_words = &filtered[2..filtered.len() - 5];
        let filter_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filter_words);
        let filter = parse_object_filter(&filter_tokens, false)?;
        return Ok(PredicateAst::PlayerTaggedObjectMatches {
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
            filter,
        });
    }

    if filtered.len() >= 7 && IS_PUT_ONTO_BATTLEFIELD_THIS_WAY_TAIL_PATTERN.matches_words(&filtered)
    {
        let filter_words = &filtered[..filtered.len() - 6];
        let filter_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(filter_words);
        let filter = parse_object_filter(&filter_tokens, false)?;
        return Ok(PredicateAst::TaggedMatches(TagKey::from(IT_TAG), filter));
    }

    if YOU_DIDNT_PUT_TAGGED_INTO_HAND_PATTERN.matches_words(&filtered) {
        return Ok(PredicateAst::Not(Box::new(
            PredicateAst::PlayerTaggedObjectMatches {
                player: PlayerAst::You,
                tag: TagKey::from(IT_TAG),
                filter: ObjectFilter::default().in_zone(Zone::Hand),
            },
        )));
    }

    if YOU_DIDNT_PUT_TAGGED_ONTO_BATTLEFIELD_PATTERN.matches_words(&filtered) {
        return Ok(PredicateAst::Not(Box::new(
            PredicateAst::PlayerTaggedObjectMatches {
                player: PlayerAst::You,
                tag: TagKey::from(IT_TAG),
                filter: ObjectFilter::default().in_zone(Zone::Battlefield),
            },
        )));
    }

    if TAGGED_WASNT_BLOCKING_PATTERN.matches_words(&filtered) {
        return Ok(PredicateAst::TaggedMatches(
            TagKey::from(IT_TAG),
            ObjectFilter {
                nonblocking: true,
                ..Default::default()
            },
        ));
    }

    if NO_CREATURES_ON_BATTLEFIELD_PATTERN.matches_words(&filtered) {
        return Ok(PredicateAst::PlayerControlsNo {
            player: PlayerAst::Any,
            filter: ObjectFilter::creature(),
        });
    }

    if let Some(predicate) = parse_player_achievement_predicate(&filtered) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_ring_bearer_temptation_predicate(&filtered, tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_status_predicate(&filtered) {
        return Ok(predicate);
    }

    if YOU_OR_DEFENDING_PLAYER_HAS_INITIATIVE_PATTERN.matches_words(&filtered) {
        return Ok(PredicateAst::Or(
            Box::new(PredicateAst::PlayerHasInitiative {
                player: PlayerAst::You,
            }),
            Box::new(PredicateAst::PlayerHasInitiative {
                player: PlayerAst::Defending,
            }),
        ));
    }

    if IT_IS_NIGHT_PATTERN.matches_words(&filtered) {
        return Ok(PredicateAst::ItIsNight);
    }

    if FIRST_COMBAT_PHASE_OF_TURN_PATTERN.matches_words(&filtered) {
        return Ok(PredicateAst::FirstCombatPhaseOfTurn);
    }

    if SOURCE_DEALT_COMBAT_DAMAGE_TO_PLAYER_THIS_TURN_PATTERN.matches_words(&filtered) {
        return Ok(PredicateAst::SourceDealtCombatDamageToPlayerThisTurn);
    }

    if THIS_TURN_TAIL_PATTERN.matches_words(
        filtered
            .get(filtered.len().saturating_sub(2)..)
            .unwrap_or_default(),
    ) && PLAYER_WAS_DEALT_COMBAT_DAMAGE_BY_SUBTYPE_PREFIX_PATTERN.matches_words(&filtered)
    {
        let subtype_idx = filtered.len().saturating_sub(3);
        let subtype = parse_subtype_word(filtered[subtype_idx]).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported combat-damage source subtype predicate: {}",
                filtered.join(" ")
            ))
        })?;
        let player =
            if filtered.first() == Some(&"opponent") || filtered.get(1) == Some(&"opponent") {
                PlayerAst::Opponent
            } else {
                PlayerAst::Any
            };
        return Ok(
            PredicateAst::PlayerWasDealtCombatDamageByCreatureSubtypeThisTurn { player, subtype },
        );
    }

    if CAST_THIS_SPELL_DURING_YOUR_MAIN_PHASE_PATTERN.matches_words(&filtered) {
        return Ok(PredicateAst::ThisSpellPaidLabel(
            "CastDuringYourMainPhase".to_string(),
        ));
    }

    if let Some(predicate) = parse_player_spell_cast_this_turn_predicate(&filtered) {
        return Ok(predicate);
    }

    if filtered.len() >= 4
        && filtered.first() == Some(&"x")
        && filtered.get(1) == Some(&"is")
        && let Some((comparison, used)) = predicate_quantity_prefix(&filtered[2..])
        && used + 2 == filtered.len()
        && let Some((operator, amount)) = comparison_to_value_comparison_operator(comparison)
    {
        return Ok(PredicateAst::ValueComparison {
            left: Value::X,
            operator,
            right: Value::Fixed(amount),
        });
    }

    if let Some(predicate) = parse_or_predicate(&filtered)? {
        return Ok(predicate);
    }

    Err(CardTextError::ParseError(format!(
        "unsupported predicate (predicate: '{}')",
        filtered.join(" ")
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::ValueComparisonOperator;
    use crate::runtime_backend::front_end::lexer::lex_line;

    const IF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["if"]);

    fn predicate_tokens_after_if(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
        tokens
            .iter()
            .filter(|token| !IF_WORD_PATTERN.matches_token(token))
            .cloned()
            .collect()
    }

    #[test]
    fn parse_predicate_accepts_unapostrophed_spell_paid_label() -> Result<(), CardTextError> {
        let tokens = lex_line("If this spells surge cost was paid", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::ThisSpellPaidLabel("Surge".to_string())
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_accepts_paid_label_with_trailing_instead_effect_tail()
    -> Result<(), CardTextError> {
        let tokens = lex_line(
            "If this creature's spectacle cost was paid instead discard your hand",
            0,
        )?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::ThisSpellPaidLabel("Spectacle".to_string())
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_opponent_would_begin_extra_turn() -> Result<(), CardTextError> {
        let tokens = lex_line("If an opponent would begin an extra turn", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::PlayerWouldBeginExtraTurn {
                player: PlayerAst::Opponent,
            }
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_you_or_player_youre_attacking_has_initiative()
    -> Result<(), CardTextError> {
        let tokens = lex_line("If you or a player you're attacking has the initiative", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::Or(
                Box::new(PredicateAst::PlayerHasInitiative {
                    player: PlayerAst::You,
                }),
                Box::new(PredicateAst::PlayerHasInitiative {
                    player: PlayerAst::Defending,
                }),
            )
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_player_statuses_use_shared_capture_parser() -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "If you're monarch",
                PredicateAst::PlayerIsMonarch {
                    player: PlayerAst::You,
                },
            ),
            (
                "If you have the initiative",
                PredicateAst::PlayerHasInitiative {
                    player: PlayerAst::You,
                },
            ),
            (
                "If you have maximum speed",
                PredicateAst::ValueComparison {
                    left: Value::Speed(PlayerFilter::You),
                    operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::Fixed(4),
                },
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_player_achievements_use_shared_capture_parser() -> Result<(), CardTextError>
    {
        for (text, expected) in [
            (
                "If you have city's blessing",
                PredicateAst::PlayerHasCitysBlessing {
                    player: PlayerAst::You,
                },
            ),
            (
                "If you've completed a dungeon",
                PredicateAst::PlayerCompletedDungeon {
                    player: PlayerAst::You,
                    dungeon_name: None,
                },
            ),
            (
                "If you have completed Lost Mine of Phandelver",
                PredicateAst::PlayerCompletedDungeon {
                    player: PlayerAst::You,
                    dungeon_name: Some("lost mine of phandelver".to_string()),
                },
            ),
            (
                "If you haven't completed Lost Mine of Phandelver",
                PredicateAst::Not(Box::new(PredicateAst::PlayerCompletedDungeon {
                    player: PlayerAst::You,
                    dungeon_name: Some("lost mine of phandelver".to_string()),
                })),
            ),
            ("If you have a full party", PredicateAst::YouHaveFullParty),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_its_night() -> Result<(), CardTextError> {
        let tokens = lex_line("If it's night", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, PredicateAst::ItIsNight);
        Ok(())
    }

    #[test]
    fn parse_predicate_accepts_first_combat_phase_of_turn() -> Result<(), CardTextError> {
        let tokens = lex_line("If it's the first combat phase of the turn", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, PredicateAst::FirstCombatPhaseOfTurn);
        Ok(())
    }

    #[test]
    fn parse_predicate_inherits_it_for_bare_or_descriptor_tail() -> Result<(), CardTextError> {
        let tokens = lex_line("If it's a creature or planeswalker card", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        match parsed {
            PredicateAst::Or(left, right) => {
                assert!(
                    matches!(*left, PredicateAst::ItMatches(ref filter) if filter.card_types == vec![CardType::Creature]),
                    "expected creature left predicate, got {left:?}"
                );
                assert!(
                    matches!(*right, PredicateAst::ItMatches(ref filter) if filter.card_types == vec![CardType::Planeswalker]),
                    "expected planeswalker right predicate, got {right:?}"
                );
            }
            other => panic!("expected inherited-reference or predicate, got {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_if_you_dont_put_card_into_your_hand() -> Result<(), CardTextError> {
        let tokens = lex_line("If you don't put the card into your hand", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::Not(Box::new(PredicateAst::PlayerTaggedObjectMatches {
                player: PlayerAst::You,
                tag: TagKey::from(IT_TAG),
                filter: ObjectFilter::default().in_zone(Zone::Hand),
            }))
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_it_dealt_combat_damage_to_player_this_turn()
    -> Result<(), CardTextError> {
        let tokens = lex_line("if it dealt combat damage to a player this turn", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::SourceDealtCombatDamageToPlayerThisTurn
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_you_cast_this_spell_during_your_main_phase()
    -> Result<(), CardTextError> {
        let tokens = lex_line("If you cast this spell during your main phase", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::ThisSpellPaidLabel("CastDuringYourMainPhase".to_string())
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_if_you_dont_put_it_into_your_hand() -> Result<(), CardTextError> {
        let tokens = lex_line("If you don't put it into your hand", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::Not(Box::new(PredicateAst::PlayerTaggedObjectMatches {
                player: PlayerAst::You,
                tag: TagKey::from(IT_TAG),
                filter: ObjectFilter::default().in_zone(Zone::Hand),
            }))
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_if_equipment_is_put_onto_the_battlefield_this_way()
    -> Result<(), CardTextError> {
        let tokens = lex_line("If an Equipment is put onto the battlefield this way", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;
        let equipment_filter_tokens = lex_line("an Equipment", 0)?;
        let equipment_filter = parse_object_filter(&equipment_filter_tokens, false)?;

        assert_eq!(
            parsed,
            PredicateAst::TaggedMatches(TagKey::from(IT_TAG), equipment_filter)
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_if_aura_is_put_onto_the_battlefield_this_way()
    -> Result<(), CardTextError> {
        let tokens = lex_line("If an Aura is put onto the battlefield this way", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;
        let aura_filter_tokens = lex_line("an Aura", 0)?;
        let aura_filter = parse_object_filter(&aura_filter_tokens, false)?;

        assert_eq!(
            parsed,
            PredicateAst::TaggedMatches(TagKey::from(IT_TAG), aura_filter)
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_you_would_draw_card() -> Result<(), CardTextError> {
        let tokens = lex_line("If you would draw a card", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;
        assert_eq!(
            parsed,
            PredicateAst::PlayerWouldDrawCard {
                player: PlayerAst::You
            }
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_player_would_actions_use_shared_capture_parser() -> Result<(), CardTextError>
    {
        for (text, expected) in [
            (
                "If you would draw a card",
                PredicateAst::PlayerWouldDrawCard {
                    player: PlayerAst::You,
                },
            ),
            (
                "If an opponent would draw card",
                PredicateAst::PlayerWouldDrawCard {
                    player: PlayerAst::Opponent,
                },
            ),
            (
                "If opponent would proliferate",
                PredicateAst::PlayerWouldProliferate {
                    player: PlayerAst::Opponent,
                },
            ),
            (
                "If an opponent would begin an extra turn",
                PredicateAst::PlayerWouldBeginExtraTurn {
                    player: PlayerAst::Opponent,
                },
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_would_draw_while_no_cards_in_hand() -> Result<(), CardTextError> {
        let tokens = lex_line(
            "If you would draw a card while you have no cards in hand",
            0,
        )?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::And(
                Box::new(PredicateAst::PlayerWouldDrawCard {
                    player: PlayerAst::You,
                }),
                Box::new(PredicateAst::YouHaveNoCardsInHand),
            )
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_cards_in_hand_counts_use_shared_capture_parser() -> Result<(), CardTextError>
    {
        for (text, expected) in [
            (
                "If you have no cards in hand",
                PredicateAst::YouHaveNoCardsInHand,
            ),
            (
                "If you have one or fewer cards in hand",
                PredicateAst::PlayerCardsInHandOrFewer {
                    player: PlayerAst::You,
                    count: 1,
                },
            ),
            (
                "If an opponent has three or more cards in hand",
                PredicateAst::PlayerCardsInHandOrMore {
                    player: PlayerAst::Opponent,
                    count: 3,
                },
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_cards_in_hand_relations_use_shared_capture_parser()
    -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "If an opponent has more cards in hand than you",
                PredicateAst::PlayerHasMoreCardsInHandThanYou {
                    player: PlayerAst::Opponent,
                },
            ),
            (
                "If a player has more cards in hand than each other player",
                PredicateAst::PlayerHasMoreCardsInHandThanEachOtherPlayer {
                    player: PlayerAst::Any,
                },
            ),
            (
                "If that player has more cards in their hand than you do",
                PredicateAst::PlayerHasMoreCardsInHandThanYou {
                    player: PlayerAst::That,
                },
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_turn_event_counts_use_shared_capture_parser() -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "If you drew two or more cards this turn",
                PredicateAst::ValueComparison {
                    left: Value::MaxCardsDrawnThisTurn(PlayerFilter::You),
                    operator: ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::Fixed(2),
                },
            ),
            (
                "If an opponent has drawn three cards this turn",
                PredicateAst::ValueComparison {
                    left: Value::MaxCardsDrawnThisTurn(PlayerFilter::Opponent),
                    operator: ValueComparisonOperator::Equal,
                    right: Value::Fixed(3),
                },
            ),
            (
                "If that player had two or fewer lands entered battlefield under their control this turn",
                PredicateAst::ValueComparison {
                    left: Value::LandsEnteredBattlefieldThisTurn(PlayerFilter::IteratedPlayer),
                    operator: ValueComparisonOperator::LessThanOrEqual,
                    right: Value::Fixed(2),
                },
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_spell_context_uses_shared_capture_parser() -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "If that spells controller poisoned",
                PredicateAst::TargetSpellControllerIsPoisoned,
            ),
            (
                "If no mana was spent to cast that spell",
                PredicateAst::TargetSpellNoManaSpentToCast,
            ),
            (
                "If you control more creatures than its controller",
                PredicateAst::YouControlMoreCreaturesThanTargetSpellController,
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_spell_cast_this_turn_uses_shared_capture_parser() -> Result<(), CardTextError>
    {
        let tokens = lex_line("If you cast another spell this turn", 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
        assert_eq!(
            parsed,
            PredicateAst::PlayerCastSpellsThisTurnOrMore {
                player: PlayerAst::You,
                count: 2,
            }
        );

        let tokens = lex_line("If opponent has cast a creature spell this turn", 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
        let PredicateAst::ValueComparison {
            left:
                Value::SpellsCastThisTurnMatching {
                    player,
                    filter,
                    exclude_source,
                },
            operator: ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(1),
        } = parsed
        else {
            panic!("expected spell-cast matching predicate, got {parsed:?}");
        };
        assert_eq!(player, PlayerFilter::Opponent);
        assert!(!exclude_source);
        assert!(filter.card_types.contains(&CardType::Creature));

        let tokens = lex_line("If you didnt cast a noncreature spell this turn", 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
        assert!(
            matches!(&parsed, PredicateAst::Not(inner) if matches!(
                inner.as_ref(),
                PredicateAst::ValueComparison {
                    left: Value::SpellsCastThisTurnMatching { player: PlayerFilter::You, .. },
                    operator: ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::Fixed(1),
                }
            )),
            "expected negated spell-cast matching predicate, got {parsed:?}"
        );

        Ok(())
    }

    #[test]
    fn parse_predicate_supports_you_would_proliferate() -> Result<(), CardTextError> {
        let tokens = lex_line("If you would proliferate", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;
        assert_eq!(
            parsed,
            PredicateAst::PlayerWouldProliferate {
                player: PlayerAst::You
            }
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_you_have_more_life_than_opponent() -> Result<(), CardTextError> {
        let tokens = lex_line("if you have more life than an opponent", 0)?;

        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::PlayerHasLessLifeThanYou {
                player: PlayerAst::Opponent,
            }
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_life_relations_use_shared_capture_parser() -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "if an opponent has more life than you",
                PredicateAst::PlayerHasMoreLifeThanYou {
                    player: PlayerAst::Opponent,
                },
            ),
            (
                "if you have more life than each opponent",
                PredicateAst::PlayerHasMoreLifeThanEachOtherPlayer {
                    player: PlayerAst::You,
                },
            ),
            (
                "if no opponent has more life than that player",
                PredicateAst::PlayerHasNoOpponentWithMoreLifeThan {
                    player: PlayerAst::That,
                },
            ),
            (
                "if a player has more life than each other player",
                PredicateAst::PlayerHasMoreLifeThanEachOtherPlayer {
                    player: PlayerAst::Any,
                },
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_life_totals_use_shared_capture_parser() -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "If you have five or less life",
                PredicateAst::ValueComparison {
                    left: crate::effect::Value::LifeTotal(PlayerFilter::You),
                    operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
                    right: crate::effect::Value::Fixed(5),
                },
            ),
            (
                "If an opponent has ten or more life",
                PredicateAst::ValueComparison {
                    left: crate::effect::Value::LifeTotal(PlayerFilter::Opponent),
                    operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                    right: crate::effect::Value::Fixed(10),
                },
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_life_change_this_turn_uses_shared_capture_parser()
    -> Result<(), CardTextError> {
        for (text, expected) in [
            (
                "If you gained life this turn",
                PredicateAst::PlayerGainedLifeThisTurnOrMore {
                    player: PlayerAst::You,
                    count: 1,
                },
            ),
            (
                "If you gained three or more life this turn",
                PredicateAst::PlayerGainedLifeThisTurnOrMore {
                    player: PlayerAst::You,
                    count: 3,
                },
            ),
            (
                "If you lost two or more life this turn",
                PredicateAst::ValueComparison {
                    left: Value::LifeLostThisTurn(PlayerFilter::You),
                    operator: ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::Fixed(2),
                },
            ),
            (
                "If one or more opponents lost life this turn",
                PredicateAst::OpponentLostLifeThisTurn,
            ),
        ] {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_ring_bearer_temptation_gate() -> Result<(), CardTextError> {
        let tokens = lex_line(
            "If this is your Ring-bearer and the Ring has tempted you two or more times this game",
            0,
        )?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::And(
                Box::new(PredicateAst::SourceIsRingBearer {
                    player: PlayerAst::You,
                }),
                Box::new(PredicateAst::PlayerRingTemptedThisGameOrMore {
                    player: PlayerAst::You,
                    count: 2,
                })
            )
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_creature_card_put_into_your_graveyard_this_turn()
    -> Result<(), CardTextError> {
        let tokens = lex_line(
            "If a creature card was put into your graveyard from anywhere this turn",
            0,
        )?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::CreatureCardPutIntoYourGraveyardThisTurn
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_battlefield_change_this_turn_uses_shared_capture_parser()
    -> Result<(), CardTextError> {
        let cases = [
            (
                "If no permanents left battlefield this turn",
                PredicateAst::Not(Box::new(PredicateAst::PermanentLeftBattlefieldThisTurn)),
            ),
            (
                "If a permanent left battlefield this turn",
                PredicateAst::PermanentLeftBattlefieldThisTurn,
            ),
            (
                "If creatures left battlefield under your control this turn",
                PredicateAst::PermanentLeftBattlefieldUnderYourControlThisTurn,
            ),
            (
                "If lands you controlled were put into graveyard from battlefield this turn",
                PredicateAst::ObjectPutIntoGraveyardFromBattlefieldThisTurn(
                    ObjectFilter::land().controlled_by(PlayerFilter::You),
                ),
            ),
        ];

        for (text, expected) in cases {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_object_death_this_turn_uses_shared_capture_parser()
    -> Result<(), CardTextError> {
        let cases = [
            (
                "If a creature died this turn",
                PredicateAst::CreatureDiedThisTurn,
            ),
            (
                "If seven or more creatures died this turn",
                PredicateAst::CreatureDiedThisTurnOrMore(7),
            ),
            (
                "If a creature card was put into your graveyard from anywhere this turn",
                PredicateAst::CreatureCardPutIntoYourGraveyardThisTurn,
            ),
        ];

        for (text, expected) in cases {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_battlefield_entry_uses_shared_capture_parser() -> Result<(), CardTextError> {
        let cases = [
            (
                "If you had another creature entered the battlefield under your control last turn",
                PredicateAst::ObjectEnteredBattlefieldLastTurn(
                    ObjectFilter::creature()
                        .controlled_by(PlayerFilter::You)
                        .other(),
                ),
            ),
            (
                "If artifacts entered battlefield under your control this turn",
                PredicateAst::ObjectEnteredBattlefieldThisTurn(
                    ObjectFilter::artifact().controlled_by(PlayerFilter::You),
                ),
            ),
            (
                "If you had lands entered battlefield under your control this turn",
                PredicateAst::PlayerHadLandEnterBattlefieldThisTurn {
                    player: PlayerAst::You,
                },
            ),
        ];

        for (text, expected) in cases {
            let tokens = lex_line(text, 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(parsed, expected, "{text}");
        }
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_card_in_your_graveyard_existence() -> Result<(), CardTextError> {
        let tokens = lex_line("If there is an Elf card in your graveyard", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        let mut expected_filter = ObjectFilter::default()
            .with_subtype(parse_subtype_word("elf").expect("elf subtype"))
            .in_zone(Zone::Graveyard);
        expected_filter.owner = Some(PlayerFilter::You);
        assert_eq!(
            parsed,
            PredicateAst::PlayerControls {
                player: PlayerAst::You,
                filter: expected_filter,
            }
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_behold_or_controlled_subtype_as_cast() -> Result<(), CardTextError>
    {
        let tokens = lex_line(
            "If you revealed a Dragon card or controlled a Dragon as you cast this spell",
            0,
        )?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::Or(
                Box::new(PredicateAst::ThisSpellPaidLabel("Behold".to_string())),
                Box::new(PredicateAst::PlayerControls {
                    player: PlayerAst::You,
                    filter: ObjectFilter::default()
                        .with_subtype(parse_subtype_word("dragon").expect("dragon subtype")),
                }),
            )
        );
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_this_has_power_or_greater() -> Result<(), CardTextError> {
        let tokens = lex_line("If this has power 7 or greater", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, PredicateAst::SourcePowerAtLeast(7));
        Ok(())
    }

    #[test]
    fn parse_predicate_supports_source_has_keyword() -> Result<(), CardTextError> {
        let tokens = lex_line("If this creature has defender", 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        let mut expected_filter = ObjectFilter::default();
        expected_filter
            .static_abilities
            .push(crate::static_abilities::StaticAbilityId::Defender);
        assert_eq!(parsed, PredicateAst::SourceMatches(expected_filter));
        Ok(())
    }
}
