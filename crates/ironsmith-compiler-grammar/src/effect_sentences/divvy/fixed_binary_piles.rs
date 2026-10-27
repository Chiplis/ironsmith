//! Freeze both piles before asking a different player to choose between them.
use super::*;
use crate::cards::builders::ChooseOneModeAst;
use crate::tag::{CompilerReferenceTag, TagRef};

fn captured(tag: &TagRef, zone: Zone) -> ObjectFilter {
    ObjectFilter::tagged(tag.clone())
        .in_zone(zone)
        .match_tagged(tag.clone(), TaggedOpbjectRelation::SameObjectId)
}

fn partition(filter: ObjectFilter, zone: Zone, player: PlayerAst) -> Vec<EffectAst> {
    let pool = CompilerReferenceTag::DivvySource.bind();
    let first = CompilerReferenceTag::DivvyPile.bind();
    let second = CompilerReferenceTag::DivvyChosen.bind();
    vec![
        EffectAst::subject_verb_tag_matching_objects(filter, vec![zone], pool.clone()),
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones {
            optional_search_zones: false,
            filter: captured(&pool, zone),
            count: ChoiceCount::any_number(),
            count_value: None,
            player,
            tag: first.clone(),
            zones: vec![zone],
            search_mode: None,
        }),
        EffectAst::subject_verb_tag_matching_objects(
            captured(&pool, zone).not_tagged(first),
            vec![zone],
            second,
        ),
    ]
}

fn choose(
    chooser: PlayerFilter,
    effects: impl Fn(&TagRef, &TagRef) -> Vec<EffectAst>,
) -> EffectAst {
    let first = CompilerReferenceTag::DivvyPile.bind();
    let second = CompilerReferenceTag::DivvyChosen.bind();
    let modes = [(&first, &second), (&second, &first)]
        .into_iter()
        .enumerate()
        .map(|(index, (chosen, other))| ChooseOneModeAst {
            description: format!("Choose pile {}", index + 1),
            effects: effects(chosen, other),
        })
        .collect();
    EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseOneOf { chooser, modes })
}

pub(super) fn destroy_creature_pile() -> Vec<EffectAst> {
    let mut effects = vec![EffectAst::subject_verb_explicit_target_only(
        TargetAst::Player(PlayerFilter::Any, None),
    )];
    effects.extend(partition(
        ObjectFilter::creature().controlled_by(PlayerFilter::target_player()),
        Zone::Battlefield,
        PlayerAst::You,
    ));
    effects.push(choose(PlayerFilter::target_player(), |chosen, _| {
        vec![EffectAst::subject_verb_destroy_no_regeneration(
            TargetAst::Object(captured(chosen, Zone::Battlefield), None, None),
        )]
    }));
    effects
}

pub(super) fn return_exiled_creature_pile() -> Vec<EffectAst> {
    let mut effects = partition(
        ObjectFilter::tagged(CompilerReferenceTag::It.bind()),
        Zone::Exile,
        PlayerAst::That,
    );
    let opponent = CompilerReferenceTag::DivvyOpponent.bind();
    let separation = effects.remove(1);
    effects.insert(
        1,
        EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
            sequential: false,
            filter: PlayerFilter::TaggedPlayer(opponent.clone().into()),
            effects: vec![separation],
        }),
    );
    effects.insert(
        0,
        EffectAst::subject_verb_choose_player(
            PlayerAst::You,
            PlayerFilter::Opponent,
            opponent,
            false,
            0,
        ),
    );
    effects.push(choose(PlayerFilter::You, |chosen, other| {
        let mut union = ObjectFilter::default().in_zone(Zone::Exile);
        union.any_of = vec![captured(chosen, Zone::Exile), captured(other, Zone::Exile)];
        vec![
            EffectAst::subject_verb_move_all_to_zone(
                TargetAst::Object(union, None, None),
                Zone::Battlefield,
                false,
                ReturnControllerAst::You,
                false,
                None,
            )
            .with_tagged_destinations(vec![
                (chosen.clone(), Zone::Battlefield),
                (other.clone(), Zone::Graveyard),
            ]),
        ]
    }));
    effects
}
