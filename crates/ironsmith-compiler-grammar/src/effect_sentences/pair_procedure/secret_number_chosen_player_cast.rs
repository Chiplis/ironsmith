//! "Exile the top three cards of your library. Each opponent secretly chooses
//! a number 0 or greater. Then those numbers are revealed. Choose an opponent
//! with the highest number. <This> deals that much damage to them, then they
//! may cast a spell from among those cards without paying its mana cost. You
//! put a card from among them that wasn't cast this way into your hand."
//! (Itazura, Lingering Wick)
//!
//! The exiled cards form one tagged collection. The secret numbers are
//! collected before any is recorded (CR 101.4 hidden simultaneous choice);
//! the controller then chooses one player among those tied for the extreme
//! number. That player is dealt damage equal to that number and may cast one
//! nonland card of the collection without paying its mana cost (CR 118.9).
//! Afterwards the controller puts one of the collection's cards still in
//! exile (a cast card left exile for the stack) into their hand.
use super::*;
use crate::cards::builders::{ForEachEffectAst, ObjectChoiceEffectAst, PermissionEffectAst};
use crate::target::{TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::types::CardType;

fn words(sentence: &SentenceInput) -> Vec<&str> {
    crate::lexer::parser_token_word_refs(crate::lexer::trim_lexed_commas(sentence.lowered()))
}

pub(super) fn read(
    sentences: &[SentenceInput],
    index: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(group) = sentences.get(index..index + 6) else {
        return Ok(None);
    };
    // 1. "Exile the top N cards of your library."
    let count = match words(&group[0]).as_slice() {
        ["exile", "the", "top", count, "cards", "of", "your", "library"] => {
            let Some(count) = crate::util::parse_number_word_u32(count) else {
                return Ok(None);
            };
            count
        }
        _ => return Ok(None),
    };
    // 2. "Each opponent secretly chooses a number 0 or greater." 3. reveal.
    let Some(choice) = super::super::player_option_choice_procedure::secret_number_opening(&group[1])
    else {
        return Ok(None);
    };
    if !super::super::player_option_choice_procedure::is_choice_reveal_sentence(&group[2]) {
        return Ok(None);
    }
    // 4. "Choose an opponent|a player with the highest|lowest number."
    let highest = match words(&group[3]).as_slice() {
        ["choose", "an", "opponent", "with", "the", extreme, "number"]
            if choice.participants == PlayerFilter::Opponent =>
        {
            *extreme
        }
        ["choose", "a", "player", "with", "the", extreme, "number"] => *extreme,
        _ => return Ok(None),
    };
    let highest = match highest {
        "highest" => true,
        "lowest" => false,
        _ => return Ok(None),
    };
    // 5. "<this> deals that much damage to them, then they may cast a spell
    //    from among those cards without paying its mana cost."
    let fifth = words(&group[4]);
    let Some(deals) = fifth.iter().position(|word| *word == "deals") else {
        return Ok(None);
    };
    if deals == 0
        || fifth[deals + 1..]
            != [
                "that", "much", "damage", "to", "them", "then", "they", "may", "cast", "a",
                "spell", "from", "among", "those", "cards", "without", "paying", "its", "mana",
                "cost",
            ]
    {
        return Ok(None);
    }
    // 6. "You put a card from among them that wasn't cast this way into your
    //    hand."
    if !matches!(
        words(&group[5]).as_slice(),
        [
            "you", "put", "a", "card", "from", "among", "them", "that", "wasnt" | "wasn't",
            "cast", "this", "way", "into", "your", "hand"
        ] | [
            "you", "put", "a", "card", "from", "among", "them", "that", "was", "not", "cast",
            "this", "way", "into", "your", "hand"
        ]
    ) {
        return Ok(None);
    }

    let exiled_tag = helper_tag_for_tokens(group[0].lowered(), "exiled");
    let chosen_player_tag = helper_tag_for_tokens(group[3].lowered(), "chosen_player");
    let put_tag = helper_tag_for_tokens(group[5].lowered(), "put_into_hand");
    // The collection's cards that are still in exile; a card cast this way
    // has left exile for the stack.
    let still_exiled = || {
        let mut filter = ObjectFilter::default().in_zone(Zone::Exile);
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: exiled_tag.clone().into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        filter
    };
    let mut castable = still_exiled();
    castable.excluded_card_types.push(CardType::Land);
    let chosen_players = PlayerFilter::TaggedPlayer(chosen_player_tag.clone().into());
    Ok(Some(vec![
        EffectAst::subject_verb_exile_top_of_library(
            PlayerAst::You,
            Value::Fixed(count as i32),
            vec![crate::tag::TagRef::of(exiled_tag.clone())],
            Vec::new(),
        ),
        EffectAst::ChoosePlayerOption(choice),
        EffectAst::subject_verb_choose_player(
            PlayerAst::You,
            PlayerFilter::TaggedPlayer(ironsmith_core::player_number_extreme_tag(highest)),
            crate::tag::TagRef::of(chosen_player_tag),
            false,
            0,
        ),
        EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
            sequential: false,
            filter: chosen_players,
            effects: vec![
                EffectAst::subject_verb_damage(
                    Value::PlayersChosenNumberExtreme { highest, guess: false },
                    TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                ),
                EffectAst::Permissions(
                    PermissionEffectAst::MayCastMatchingSpellWithoutPayingManaCost {
                        player: PlayerAst::That,
                        zone_owner: PlayerAst::You,
                        filter: castable,
                        zone: Zone::Exile,
                        payment: ironsmith_core::MayCastMatchingSpellPayment::WithoutPayingManaCost,
                    },
                ),
            ],
        }),
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter: still_exiled(),
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: put_tag.clone(),
        }),
        EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
            tag: put_tag,
            effects: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
                Zone::Hand,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            )],
        }),
    ]))
}
