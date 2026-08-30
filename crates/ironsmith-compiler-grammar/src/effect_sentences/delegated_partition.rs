use super::*;
use crate::cards::builders::{
    ChoiceCount, EffectAst, PlayerAst, PredicateAst, ReturnControllerAst, SubjectVerbActionAst,
    SubjectVerbRoleAst, TargetAst,
};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::CardType;
use crate::zone::Zone;

pub(super) fn parse_source_exiled_delegated_partition_program(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let shape = crate::grammar::effects::delegated_partition_shapes::parse_source_exiled_delegated_partition_shape(tokens)?;
    let pool_tag = crate::tag::CompilerReferenceTag::SourceExiled.key();
    let subset_tag = crate::tag::CompilerDerivedTag::DelegatedSubset.key(&pool_tag);
    let choose_subset = EffectAst::ChooseObjects {
        filter: ObjectFilter::tagged(pool_tag.clone()),
        count: ChoiceCount::exactly(shape.subset_count),
        count_value: None,
        player: PlayerAst::Opponent,
        tag: subset_tag.clone(),
    };
    let move_subset = EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(subset_tag.clone(), None),
        Zone::Library,
        false,
        ReturnControllerAst::Preserve,
        false,
        None,
    );
    let return_complement = EffectAst::subject_verb_return_to_battlefield(
        TargetAst::Object(
            ObjectFilter::tagged(pool_tag).not_tagged(subset_tag),
            None,
            None,
        ),
        true,
        false,
        false,
        ReturnControllerAst::Preserve,
        None,
    );
    Some(vec![choose_subset, move_subset, return_complement])
}

pub(super) fn parse_revealed_top_delegated_partition_program(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let shape = crate::grammar::effects::delegated_partition_shapes::parse_revealed_top_delegated_partition_shape(tokens)?;
    let pool_count = i32::try_from(shape.pool_count).ok()?;
    let pool_tag = crate::util::helper_tag_for_tokens(tokens, "delegated_collection_pool");
    let subset_tag = crate::tag::CompilerDerivedTag::DelegatedSubset.key(&pool_tag);
    let exiled_tag = crate::util::helper_tag_for_tokens(tokens, "delegated_complement_exiled");

    let reveal = EffectAst::subject_verb_reveal_top_cards(
        PlayerAst::You,
        crate::effect::Value::Fixed(pool_count),
        pool_tag.clone(),
    );
    let choose_subset = EffectAst::ChooseObjects {
        filter: ObjectFilter::tagged(pool_tag.clone()),
        count: ChoiceCount::exactly(shape.subset_count),
        count_value: None,
        player: PlayerAst::Opponent,
        tag: subset_tag.clone(),
    };
    let move_subset =
        EffectAst::subject_verb_return_to_hand(TargetAst::Tagged(subset_tag.clone(), None), false);
    let exile_complement = EffectAst::TagAffected {
        effect: Box::new(EffectAst::subject_verb_exile(
            TargetAst::Object(
                ObjectFilter::tagged(pool_tag).not_tagged(subset_tag),
                None,
                None,
            ),
            false,
        )),
        tag: exiled_tag.clone(),
    };
    let mark_exiled = EffectAst::subject_verb_put_counters(
        crate::object::CounterType::Silver,
        crate::effect::Value::Fixed(1),
        TargetAst::Tagged(exiled_tag, None),
        None,
        false,
    );

    Some(vec![
        reveal,
        choose_subset,
        move_subset,
        exile_complement,
        mark_exiled,
    ])
}

fn tagged_graveyard_target_pool(
    tokens: &[OwnedLexToken],
    count: usize,
    filter: ObjectFilter,
    tag: crate::tag::TagKey,
) -> EffectAst {
    let target = TargetAst::WithCount(
        Box::new(TargetAst::Object(
            filter.in_zone(Zone::Graveyard).owned_by(PlayerFilter::You),
            crate::util::span_from_tokens(tokens),
            None,
        )),
        ChoiceCount::up_to(count),
    );
    EffectAst::TagAffected {
        effect: Box::new(EffectAst::subject_verb_explicit_target_only(target)),
        tag,
    }
}

pub(super) fn parse_delegated_graveyard_pair_partition_program(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let shape = crate::grammar::effects::delegated_partition_shapes::parse_delegated_graveyard_pair_partition_shape(tokens)?;
    let sentences = crate::lexer::split_lexed_sentences(tokens);
    let pool_tag = crate::util::helper_tag_for_tokens(tokens, "delegated_collection_pool");
    let subset_tag = crate::tag::CompilerDerivedTag::DelegatedSubset.key(&pool_tag);
    let pool = tagged_graveyard_target_pool(
        sentences[0],
        shape.pool_count,
        ObjectFilter::default().with_type(CardType::Creature),
        pool_tag.clone(),
    );
    let choose_subset = EffectAst::ChooseObjects {
        filter: ObjectFilter::tagged(pool_tag.clone()),
        count: ChoiceCount::exactly(shape.subset_count),
        count_value: None,
        player: PlayerAst::Opponent,
        tag: subset_tag.clone(),
    };
    let return_subset =
        EffectAst::subject_verb_return_to_hand(TargetAst::Tagged(subset_tag.clone(), None), false);
    let return_complement = EffectAst::subject_verb_return_to_battlefield(
        TargetAst::Object(
            ObjectFilter::tagged(pool_tag).not_tagged(subset_tag),
            None,
            None,
        ),
        false,
        false,
        false,
        ReturnControllerAst::You,
        None,
    );
    Some(vec![pool, choose_subset, return_subset, return_complement])
}

pub(super) fn parse_conditional_delegated_graveyard_partition_program(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let shape = crate::grammar::effects::delegated_partition_shapes::parse_conditional_delegated_graveyard_partition_shape(tokens)?;
    let sentences = crate::lexer::split_lexed_sentences(tokens);
    let pool_tag = crate::util::helper_tag_for_tokens(tokens, "delegated_collection_pool");
    let subset_tag = crate::tag::CompilerDerivedTag::DelegatedSubset.key(&pool_tag);

    let condition = sentences[1];
    let return_idx = condition.iter().position(|token| token.is_word("return"))?;
    let predicate = crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(
        &condition[1..return_idx],
    )
    .ok()?;

    let pool = tagged_graveyard_target_pool(
        sentences[0],
        shape.pool_count,
        ObjectFilter::default(),
        pool_tag.clone(),
    );

    let mut subset_filter = ObjectFilter::tagged(pool_tag.clone());
    subset_filter.zone = None;
    let choose_subset = EffectAst::ChooseObjects {
        filter: subset_filter,
        count: ChoiceCount::exactly(shape.subset_count),
        count_value: None,
        player: PlayerAst::Opponent,
        tag: subset_tag.clone(),
    };
    let move_remainder = EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::PutTaggedRemainderInZone {
            tag: pool_tag.clone(),
            keep_tagged: subset_tag,
            zone: Zone::Hand,
            surface: ironsmith_core::LibraryRemainderSurface::Rest,
        },
    );
    let conditional = EffectAst::Conditional {
        predicate,
        if_true: vec![EffectAst::subject_verb_return_to_hand(
            TargetAst::Tagged(pool_tag, None),
            false,
        )],
        if_false: vec![choose_subset, move_remainder],
    };

    Some(vec![pool, conditional])
}
