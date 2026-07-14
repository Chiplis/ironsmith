use crate::runtime_backend::grammar::permission_shapes;
use crate::runtime_backend::lexer::OwnedLexToken;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TargetSurface {
    AllReferencedWithThatName,
    AnyOtherTarget,
    AnyPlayer,
    AnyTarget,
    CardsShorthand,
    ChosenPlayer,
    ControllerOrOwnerPluralWord,
    CreatureTappedForSpellCost,
    DefendingPlayerChoice,
    DefendingPlayerEdge,
    EnchantedPlayer,
    EquippedObject,
    HimOrHer,
    InsteadThisWayWord,
    Itself,
    ItsOrTheirController,
    ItsOrTheirOwner,
    ItInsteadThisWay,
    ItOrThemWith,
    ItOrThemWord,
    ItWord,
    LibraryWord,
    MixedPlayerPlaneswalkerToken,
    OneOfYourOpponents,
    Opponent,
    OrWord,
    PlayerOnYourTeam,
    Rest,
    SourcePtPrefix,
    SourcePtReference,
    Spell,
    TaggedObject,
    TargetNoun,
    ThatOpponent,
    ThatOrTheWord,
    ThatPlayer,
    ThatWord,
    Them,
    TokenCreatedThisWay,
    TopCardShorthand,
    TriggeringSpellOrAbility,
    TriggeringSpell,
    WithWord,
    YourOpponents,
    YouOrYour,
}

pub(super) const ALL_REFERENCED_WITH_THAT_NAME_PATTERN: TargetSurface =
    TargetSurface::AllReferencedWithThatName;
pub(super) const ANY_OTHER_TARGET_PATTERN: TargetSurface = TargetSurface::AnyOtherTarget;
pub(super) const ANY_PLAYER_TARGET_PATTERN: TargetSurface = TargetSurface::AnyPlayer;
pub(super) const ANY_TARGET_PATTERN: TargetSurface = TargetSurface::AnyTarget;
pub(super) const CARDS_TARGET_SHORTHAND_PATTERN: TargetSurface = TargetSurface::CardsShorthand;
pub(super) const CHOSEN_PLAYER_TARGET_PATTERN: TargetSurface = TargetSurface::ChosenPlayer;
pub(super) const CONTROLLER_OR_OWNER_PLURAL_WORD_PATTERN: TargetSurface =
    TargetSurface::ControllerOrOwnerPluralWord;
pub(super) const CREATURE_TAPPED_FOR_THIS_SPELL_COST_PATTERN: TargetSurface =
    TargetSurface::CreatureTappedForSpellCost;
pub(super) const DEFENDING_PLAYER_CHOICE_TARGET_PATTERN: TargetSurface =
    TargetSurface::DefendingPlayerChoice;
pub(super) const DEFENDING_PLAYER_EDGE_PATTERN: TargetSurface = TargetSurface::DefendingPlayerEdge;
pub(super) const ENCHANTED_PLAYER_TARGET_PATTERN: TargetSurface = TargetSurface::EnchantedPlayer;
pub(super) const EQUIPPED_OBJECT_TARGET_PATTERN: TargetSurface = TargetSurface::EquippedObject;
pub(super) const HIM_OR_HER_TARGET_PATTERN: TargetSurface = TargetSurface::HimOrHer;
pub(super) const INSTEAD_THIS_WAY_WORD_PATTERN: TargetSurface = TargetSurface::InsteadThisWayWord;
pub(super) const ITSELF_TARGET_PATTERN: TargetSurface = TargetSurface::Itself;
pub(super) const ITS_OR_THEIR_CONTROLLER_TARGET_PATTERN: TargetSurface =
    TargetSurface::ItsOrTheirController;
pub(super) const ITS_OR_THEIR_OWNER_TARGET_PATTERN: TargetSurface = TargetSurface::ItsOrTheirOwner;
pub(super) const IT_INSTEAD_THIS_WAY_PREFIX_PATTERN: TargetSurface =
    TargetSurface::ItInsteadThisWay;
pub(super) const IT_OR_THEM_WITH_PREFIX_PATTERN: TargetSurface = TargetSurface::ItOrThemWith;
pub(super) const IT_OR_THEM_WORD_PATTERN: TargetSurface = TargetSurface::ItOrThemWord;
pub(super) const IT_WORD_PATTERN: TargetSurface = TargetSurface::ItWord;
pub(super) const LIBRARY_WORD_PATTERN: TargetSurface = TargetSurface::LibraryWord;
pub(super) const MIXED_PLAYER_PLANESWALKER_TOKEN_PATTERN: TargetSurface =
    TargetSurface::MixedPlayerPlaneswalkerToken;
pub(super) const ONE_OF_YOUR_OPPONENTS_TARGET_PATTERN: TargetSurface =
    TargetSurface::OneOfYourOpponents;
pub(super) const OPPONENT_TARGET_PATTERN: TargetSurface = TargetSurface::Opponent;
pub(super) const OR_WORD_PATTERN: TargetSurface = TargetSurface::OrWord;
pub(super) const PLAYER_ON_YOUR_TEAM_TARGET_PATTERN: TargetSurface =
    TargetSurface::PlayerOnYourTeam;
pub(super) const REST_TARGET_PATTERN: TargetSurface = TargetSurface::Rest;
pub(super) const SOURCE_PT_REFERENCE_PREFIX_PATTERN: TargetSurface = TargetSurface::SourcePtPrefix;
pub(super) const SOURCE_PT_REFERENCE_TARGET_PATTERN: TargetSurface =
    TargetSurface::SourcePtReference;
pub(super) const SPELL_TARGET_PATTERN: TargetSurface = TargetSurface::Spell;
pub(super) const TAGGED_OBJECT_TARGET_PATTERN: TargetSurface = TargetSurface::TaggedObject;
pub(super) const TARGET_OR_TARGETS_WORD_PATTERN: TargetSurface = TargetSurface::TargetNoun;
pub(super) const THAT_OPPONENT_TARGET_PATTERN: TargetSurface = TargetSurface::ThatOpponent;
pub(super) const THAT_OR_THE_WORD_PATTERN: TargetSurface = TargetSurface::ThatOrTheWord;
pub(super) const THAT_PLAYER_TARGET_PATTERN: TargetSurface = TargetSurface::ThatPlayer;
pub(super) const THAT_WORD_PATTERN: TargetSurface = TargetSurface::ThatWord;
pub(super) const THEM_TARGET_PATTERN: TargetSurface = TargetSurface::Them;
pub(super) const TOKEN_CREATED_THIS_WAY_TARGET_PATTERN: TargetSurface =
    TargetSurface::TokenCreatedThisWay;
pub(super) const TOP_CARD_TARGET_SHORTHAND_PATTERN: TargetSurface = TargetSurface::TopCardShorthand;
pub(super) const TRIGGERING_SPELL_OR_ABILITY_TARGET_PATTERN: TargetSurface =
    TargetSurface::TriggeringSpellOrAbility;
pub(super) const TRIGGERING_SPELL_TARGET_PATTERN: TargetSurface = TargetSurface::TriggeringSpell;
pub(super) const WITH_WORD_PATTERN: TargetSurface = TargetSurface::WithWord;
pub(super) const YOUR_OPPONENTS_TARGET_PATTERN: TargetSurface = TargetSurface::YourOpponents;
pub(super) const YOU_OR_YOUR_PREFIX_PATTERN: TargetSurface = TargetSurface::YouOrYour;

pub(super) fn matches_surface(words: &[&str], surface: TargetSurface) -> bool {
    match surface {
        TargetSurface::AllReferencedWithThatName => exact_one_of(
            words,
            &[
                &["all", "of", "them", "with", "that", "name"],
                &["all", "of", "those", "with", "that", "name"],
            ],
        ),
        TargetSurface::AnyOtherTarget => exact_one_of(
            words,
            &[
                &["any", "other"],
                &["any", "other", "target"],
                &["any", "other", "targets"],
                &["other"],
                &["the", "other"],
            ],
        ),
        TargetSurface::AnyPlayer => exact_one_of(words, &[&["player"], &["players"]]),
        TargetSurface::AnyTarget => {
            exact_one_of(words, &[&["any"], &["any", "target"], &["any", "targets"]])
        }
        TargetSurface::CardsShorthand => permission_shapes::exact_words(words, &["cards"]),
        TargetSurface::ChosenPlayer => {
            exact_one_of(words, &[&["chosen", "player"], &["chosen", "players"]])
        }
        TargetSurface::ControllerOrOwnerPluralWord => exact_one_of(
            words,
            &[&["controller"], &["controllers"], &["owner"], &["owners"]],
        ),
        TargetSurface::CreatureTappedForSpellCost => {
            permission_shapes::prefix_words(words, &["creature", "tapped", "to", "pay", "this"])
                && suffix_one_of(words, &[&["additional", "cost"], &["additional", "costs"]])
                && has_one_of_words(words, &["spell", "spell's", "spell’s", "spells"])
        }
        TargetSurface::DefendingPlayerChoice => {
            has_all_words(words, &["defending", "player", "choice"])
        }
        TargetSurface::DefendingPlayerEdge => {
            permission_shapes::prefix_words(words, &["defending", "player"])
        }
        TargetSurface::EnchantedPlayer => exact_one_of(
            words,
            &[&["enchanted", "player"], &["enchanted", "players"]],
        ),
        TargetSurface::EquippedObject => exact_one_of(
            words,
            &[&["equipped", "creature"], &["equipped", "permanent"]],
        ),
        TargetSurface::HimOrHer => exact_one_of(words, &[&["him"], &["her"]]),
        TargetSurface::InsteadThisWayWord => {
            exact_one_of(words, &[&["instead"], &["this"], &["way"]])
        }
        TargetSurface::Itself => permission_shapes::exact_words(words, &["itself"]),
        TargetSurface::ItsOrTheirController => prefix_one_of(
            words,
            &[
                &["its", "controller"],
                &["its", "controllers"],
                &["their", "controller"],
                &["their", "controllers"],
            ],
        ),
        TargetSurface::ItsOrTheirOwner => prefix_one_of(
            words,
            &[
                &["its", "owner"],
                &["its", "owners"],
                &["their", "owner"],
                &["their", "owners"],
            ],
        ),
        TargetSurface::ItInsteadThisWay => permission_shapes::prefix_words(words, &["it"]),
        TargetSurface::ItOrThemWith => prefix_one_of(words, &[&["it", "with"], &["them", "with"]]),
        TargetSurface::ItOrThemWord => exact_one_of(words, &[&["it"], &["them"]]),
        TargetSurface::ItWord => permission_shapes::exact_words(words, &["it"]),
        TargetSurface::LibraryWord => permission_shapes::exact_words(words, &["library"]),
        TargetSurface::MixedPlayerPlaneswalkerToken => {
            has_all_words(words, &["player", "planeswalker", "token"])
        }
        TargetSurface::OneOfYourOpponents => exact_one_of(
            words,
            &[
                &["one", "of", "your", "opponents"],
                &["one", "of", "your", "opponent"],
            ],
        ),
        TargetSurface::Opponent => exact_one_of(words, &[&["opponent"], &["opponents"]]),
        TargetSurface::OrWord => permission_shapes::exact_words(words, &["or"]),
        TargetSurface::PlayerOnYourTeam => exact_one_of(
            words,
            &[
                &["player", "on", "your", "team"],
                &["players", "on", "your", "team"],
            ],
        ),
        TargetSurface::Rest => exact_one_of(
            words,
            &[
                &["rest"],
                &["the", "rest"],
                &["rest", "of", "revealed", "cards"],
                &["the", "rest", "of", "revealed", "cards"],
            ],
        ),
        TargetSurface::SourcePtPrefix => prefix_one_of(
            words,
            &[
                &["thiss", "power", "and", "toughness"],
                &["this", "power", "and", "toughness"],
            ],
        ),
        TargetSurface::SourcePtReference => exact_one_of(
            words,
            &[
                &["thiss", "power"],
                &["this", "power"],
                &["thiss", "toughness"],
                &["this", "toughness"],
                &["thiss", "base", "power", "and", "toughness"],
                &["this", "base", "power", "and", "toughness"],
            ],
        ),
        TargetSurface::Spell => exact_one_of(words, &[&["spell"], &["spells"]]),
        TargetSurface::TaggedObject => exact_one_of(words, &[&["the", "card"], &["it"]]),
        TargetSurface::TargetNoun => exact_one_of(words, &[&["target"], &["targets"]]),
        TargetSurface::ThatOpponent => permission_shapes::exact_words(words, &["that", "opponent"]),
        TargetSurface::ThatOrTheWord => exact_one_of(words, &[&["that"], &["the"]]),
        TargetSurface::ThatPlayer => permission_shapes::exact_words(words, &["that", "player"]),
        TargetSurface::ThatWord => permission_shapes::exact_words(words, &["that"]),
        TargetSurface::Them => permission_shapes::exact_words(words, &["them"]),
        TargetSurface::TokenCreatedThisWay => exact_one_of(
            words,
            &[
                &["token", "created", "this", "way"],
                &["tokens", "created", "this", "way"],
                &["that", "token", "created", "this", "way"],
                &["those", "tokens", "created", "this", "way"],
            ],
        ),
        TargetSurface::TopCardShorthand => exact_one_of(words, &[&["top", "card"], &["card"]]),
        TargetSurface::TriggeringSpellOrAbility => exact_one_of(
            words,
            &[
                &["that", "spell", "or", "ability"],
                &["that", "ability", "or", "spell"],
            ],
        ),
        TargetSurface::TriggeringSpell => {
            exact_one_of(words, &[&["that", "spell"], &["those", "spells"]])
        }
        TargetSurface::WithWord => permission_shapes::exact_words(words, &["with"]),
        TargetSurface::YourOpponents => {
            exact_one_of(words, &[&["your", "opponents"], &["opponents"]])
        }
        TargetSurface::YouOrYour => prefix_one_of(words, &[&["you"], &["your"]]),
    }
}

pub(super) fn matches_surface_word(word: &str, surface: TargetSurface) -> bool {
    matches_surface(&[word], surface)
}

pub(super) fn token_matches_surface(token: &OwnedLexToken, surface: TargetSurface) -> bool {
    token
        .as_word()
        .is_some_and(|word| matches_surface_word(word, surface))
}

fn exact_one_of(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|expected| permission_shapes::exact_words(words, expected))
}

fn prefix_one_of(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|expected| permission_shapes::prefix_words(words, expected))
}

fn suffix_one_of(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|expected| permission_shapes::suffix_words(words, expected))
}

fn has_one_of_words(words: &[&str], alternatives: &[&str]) -> bool {
    alternatives
        .iter()
        .any(|word| permission_shapes::find_words(words, &[*word]).is_some())
}

fn has_all_words(words: &[&str], required: &[&str]) -> bool {
    required
        .iter()
        .all(|word| permission_shapes::find_words(words, &[*word]).is_some())
}
