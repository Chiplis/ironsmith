use std::collections::HashMap;

use ironsmith::{
    CardBuilder, CardDefinition, CardId, CardType, CommanderDraftBooster, CommanderDraftProduct,
    CommanderDraftState, DraftSelection, PlayerId, Supertype,
};

fn card(name: &str) -> CardDefinition {
    CardDefinition::new(CardBuilder::new(CardId::new(), name).build())
}

fn basic(name: &str) -> CardDefinition {
    CardDefinition::new(
        CardBuilder::new(CardId::new(), name)
            .supertypes(vec![Supertype::Basic])
            .card_types(vec![CardType::Land])
            .build(),
    )
}

fn players() -> Vec<PlayerId> {
    (0..3).map(PlayerId::from_index).collect()
}

fn boosters(
    prefix: &str,
    cards_per_pack: usize,
    product: CommanderDraftProduct,
) -> Vec<CommanderDraftBooster> {
    (1..=3)
        .map(|round| CommanderDraftBooster {
            product,
            cards: (0..cards_per_pack)
                .map(|index| card(&format!("{prefix} R{round} C{index}")))
                .collect(),
        })
        .collect()
}

fn selections(draft: &CommanderDraftState, players: &[PlayerId]) -> Vec<DraftSelection> {
    players
        .iter()
        .filter_map(|player| {
            let pack = draft.current_pack_view(*player, *player);
            (!pack.is_empty()).then(|| DraftSelection {
                player: *player,
                card_ids: pack.iter().take(2).map(|card| card.id).collect(),
                exchange_face_up: None,
                public_notes: HashMap::new(),
            })
        })
        .collect()
}

fn draft_with_product(
    cards_per_pack: usize,
    product: CommanderDraftProduct,
) -> CommanderDraftState {
    let seats = players();
    CommanderDraftState::new(
        seats.clone(),
        vec![
            (seats[0], boosters("Alice", cards_per_pack, product)),
            (seats[1], boosters("Bob", cards_per_pack, product)),
            (seats[2], boosters("Cara", cards_per_pack, product)),
        ],
    )
    .unwrap()
}

fn finish(draft: &mut CommanderDraftState) {
    let seats = players();
    while !draft.is_complete() {
        draft.draft_step(selections(draft, &seats)).unwrap();
    }
}

#[test]
fn u077_two_card_and_tail_picks_pass_left_right_left_and_finalize_pools() {
    let seats = players();
    let mut draft = draft_with_product(5, CommanderDraftProduct::CommanderLegends);

    draft.draft_step(selections(&draft, &seats)).unwrap();
    assert_eq!(
        draft.current_pack_view(seats[1], seats[1])[0]
            .name
            .as_deref(),
        Some("Alice R1 C2")
    );
    draft.draft_step(selections(&draft, &seats)).unwrap();
    assert_eq!(
        draft.current_pack_view(seats[2], seats[2])[0]
            .name
            .as_deref(),
        Some("Alice R1 C4")
    );
    draft.draft_step(selections(&draft, &seats)).unwrap();
    assert_eq!(draft.round(), 2, "the one-card tail finishes round one");

    draft.draft_step(selections(&draft, &seats)).unwrap();
    assert_eq!(
        draft.current_pack_view(seats[2], seats[2])[0]
            .name
            .as_deref(),
        Some("Alice R2 C2"),
        "round two passes right"
    );

    finish(&mut draft);
    assert!(draft.is_complete());
    for player in seats {
        assert_eq!(draft.card_pool(player).unwrap().len(), 15);
    }
}

#[test]
fn u077_pack_and_picks_are_private_and_illegal_batches_roll_back_atomically() {
    let seats = players();
    let mut draft = draft_with_product(4, CommanderDraftProduct::CommanderLegends);
    let alice_pack = draft.current_pack_view(seats[0], seats[0]);
    let opponent_view = draft.current_pack_view(seats[1], seats[0]);
    assert!(alice_pack.iter().all(|card| card.name.is_some()));
    assert!(opponent_view.iter().all(|card| card.name.is_none()));

    let error = draft
        .draft_step(vec![
            DraftSelection {
                player: seats[0],
                card_ids: vec![alice_pack[0].id],
                ..Default::default()
            },
            selections(&draft, &seats)[1].clone(),
            selections(&draft, &seats)[2].clone(),
        ])
        .unwrap_err();
    assert!(error.contains("must draft 2"));
    assert_eq!(draft.current_pack_view(seats[0], seats[0]).len(), 4);
    assert!(draft.drafted_view(seats[0], seats[0]).is_empty());

    draft.draft_step(selections(&draft, &seats)).unwrap();
    assert_eq!(draft.drafted_view(seats[0], seats[0]).len(), 2);
    assert!(
        draft
            .drafted_view(seats[1], seats[0])
            .iter()
            .all(|card| card.name.is_none())
    );
}

#[test]
fn u077_pool_validation_allows_external_basics_and_has_no_sixty_card_maximum() {
    let alice = players()[0];
    let mut draft = draft_with_product(22, CommanderDraftProduct::CommanderLegends);
    finish(&mut draft);
    let pool = draft.card_pool(alice).unwrap();
    let commander = vec![pool[0].clone()];
    let mut legal = pool[1..].to_vec();
    legal.extend(std::iter::repeat_n(basic("Plains"), 10));
    assert!(legal.len() + commander.len() > 60);
    draft
        .validate_pool_and_size(alice, &legal, &commander)
        .unwrap();

    let mut too_short = vec![basic("Plains"); 58];
    assert!(
        draft
            .validate_pool_and_size(alice, &too_short, &commander)
            .unwrap_err()
            .contains("at least 60")
    );
    too_short.push(card("Undrafted Spell"));
    assert!(
        draft
            .validate_pool_and_size(alice, &too_short, &commander)
            .unwrap_err()
            .contains("not in")
    );
}

#[test]
fn u077_product_additions_are_limited_to_two_matching_commander_slots() {
    let alice = players()[0];
    let mut legends = draft_with_product(1, CommanderDraftProduct::CommanderLegends);
    finish(&mut legends);
    let basics = vec![basic("Island"); 58];
    legends
        .validate_pool_and_size(
            alice,
            &basics,
            &[card("The Prismatic Piper"), card("The Prismatic Piper")],
        )
        .unwrap();
    assert!(
        legends
            .validate_pool_and_size(
                alice,
                &basics,
                &[card("Faceless One"), card("Faceless One")]
            )
            .is_err()
    );

    let mut baldurs_gate = draft_with_product(1, CommanderDraftProduct::BattleForBaldursGate);
    finish(&mut baldurs_gate);
    baldurs_gate
        .validate_pool_and_size(
            alice,
            &basics,
            &[card("Faceless One"), card("Faceless One")],
        )
        .unwrap();
    assert!(
        baldurs_gate
            .validate_pool_and_size(
                alice,
                &basics,
                &[card("The Prismatic Piper"), card("The Prismatic Piper")],
            )
            .is_err()
    );
}
