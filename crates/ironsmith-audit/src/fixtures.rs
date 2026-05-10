use ed25519_dalek::{Signer, SigningKey};

use crate::{
    ActionEnvelope, ActionSigningPayload, AuditCommand, AuditTranscript, CardOpening, DeckCeremony,
    PlayerInfo, RngReveal, ShuffleProof, ShuffleStep, TRANSCRIPT_VERSION, Visibility,
    action_state_hash, backend::ZIFFLE_0_1_BACKEND, canonical_bytes, card_commitment,
    decklist_hash, encode_hex, encrypted_deck_hash, entropy_commitment, initial_state_hash,
    rng_commitment, shuffle_proof_transcript_hash,
};

pub fn fair_transcript() -> Result<AuditTranscript, String> {
    build_transcript(false)
}

pub fn cheating_transcript() -> Result<AuditTranscript, String> {
    build_transcript(true)
}

fn build_transcript(cheat: bool) -> Result<AuditTranscript, String> {
    let match_id = "fixture-four-player-audit-v1".to_string();
    let signing_keys = signing_keys();
    let players = player_infos(&signing_keys);
    let decklists = fixture_decklists();
    let deck_ceremonies = decklists
        .iter()
        .enumerate()
        .map(|(seat, deck)| build_deck_ceremony(&match_id, seat as u8, deck))
        .collect::<Result<Vec<_>, _>>()?;

    let initial_state_hash = initial_state_hash(&match_id, &[0, 1, 2, 3]);
    let mut state_hash = initial_state_hash.clone();
    let mut actions = Vec::new();

    push_signed_action(
        &mut actions,
        &match_id,
        &signing_keys,
        &mut state_hash,
        1,
        0,
        AuditCommand::PassPriority,
        Vec::new(),
        Vec::new(),
    )?;

    push_signed_action(
        &mut actions,
        &match_id,
        &signing_keys,
        &mut state_hash,
        2,
        1,
        AuditCommand::DrawCards {
            player: 1,
            count: 1,
        },
        vec![opening(
            &match_id,
            1,
            0,
            &decklists[1][0],
            Visibility::OwnerOnly,
        )],
        Vec::new(),
    )?;

    let player_two_opening = if cheat {
        let wrong_card = "Time Walk";
        let salt = opening_salt(2, 0);
        CardOpening {
            owner: 2,
            slot: 0,
            card: wrong_card.to_string(),
            salt: salt.clone(),
            commitment: card_commitment(&match_id, 2, 0, wrong_card, &salt),
            visibility: Visibility::OwnerOnly,
        }
    } else {
        opening(&match_id, 2, 0, &decklists[2][0], Visibility::OwnerOnly)
    };
    push_signed_action(
        &mut actions,
        &match_id,
        &signing_keys,
        &mut state_hash,
        3,
        2,
        AuditCommand::DrawCards {
            player: 2,
            count: 1,
        },
        vec![player_two_opening],
        Vec::new(),
    )?;

    let rng_reveals = (0..4)
        .map(|player| {
            let event_id = "shuffle-p3-library-seq4";
            let opening = format!("rng-open-p{player}-seq4");
            RngReveal {
                event_id: event_id.to_string(),
                player,
                commitment: rng_commitment(&match_id, event_id, player, &opening),
                opening,
            }
        })
        .collect();
    push_signed_action(
        &mut actions,
        &match_id,
        &signing_keys,
        &mut state_hash,
        4,
        3,
        AuditCommand::ShuffleLibrary { player: 3 },
        Vec::new(),
        rng_reveals,
    )?;

    let rng_reveals = all_player_rng_reveals(&match_id, "search-p0-library", 5);
    push_signed_action(
        &mut actions,
        &match_id,
        &signing_keys,
        &mut state_hash,
        5,
        0,
        AuditCommand::SearchLibrary {
            searcher: 0,
            library_owner: 0,
            filter: "creature".to_string(),
            selected_slot: Some(1),
        },
        vec![opening(
            &match_id,
            0,
            1,
            &decklists[0][1],
            Visibility::Viewer { viewer: 0 },
        )],
        rng_reveals,
    )?;

    Ok(AuditTranscript {
        version: TRANSCRIPT_VERSION,
        match_id,
        players,
        initial_state_hash,
        deck_ceremonies,
        actions,
    })
}

fn all_player_rng_reveals(match_id: &str, event_prefix: &str, seq: u64) -> Vec<RngReveal> {
    (0..4)
        .map(|player| {
            let event_id = format!("{event_prefix}-seq{seq}");
            let opening = format!("rng-open-p{player}-seq{seq}");
            RngReveal {
                event_id: event_id.to_string(),
                player,
                commitment: rng_commitment(match_id, &event_id, player, &opening),
                opening,
            }
        })
        .collect()
}

fn signing_keys() -> Vec<SigningKey> {
    (0..4)
        .map(|seat| {
            let mut seed = [0u8; 32];
            seed[0] = 0x51;
            seed[1] = seat + 1;
            seed[31] = 0xa0 + seat;
            SigningKey::from_bytes(&seed)
        })
        .collect()
}

fn player_infos(signing_keys: &[SigningKey]) -> Vec<PlayerInfo> {
    signing_keys
        .iter()
        .enumerate()
        .map(|(seat, key)| PlayerInfo {
            seat: seat as u8,
            name: format!("Player {}", seat + 1),
            verifying_key: encode_hex(key.verifying_key().as_bytes()),
        })
        .collect()
}

fn fixture_decklists() -> Vec<Vec<String>> {
    vec![
        vec!["Forest", "Llanowar Elves", "Giant Growth", "Elvish Mystic"],
        vec!["Island", "Opt", "Counterspell", "Ponder"],
        vec![
            "Mountain",
            "Lightning Bolt",
            "Monastery Swiftspear",
            "Shock",
        ],
        vec!["Swamp", "Thoughtseize", "Dark Ritual", "Doom Blade"],
    ]
    .into_iter()
    .map(|deck| deck.into_iter().map(str::to_string).collect())
    .collect()
}

fn build_deck_ceremony(
    match_id: &str,
    owner: u8,
    cards: &[String],
) -> Result<DeckCeremony, String> {
    let deck_id = format!("p{owner}-library");
    let declared_decklist_hash = decklist_hash(match_id, owner, cards)?;
    let initial_encrypted_deck_hash = encrypted_deck_hash(
        match_id,
        &deck_id,
        "owner-encrypted-deck",
        &declared_decklist_hash,
    );
    let required_shufflers = vec![0, 1, 2, 3];
    let mut previous = initial_encrypted_deck_hash.clone();
    let mut steps = Vec::new();
    for shuffler in &required_shufflers {
        let opening = format!("shuffle-open-p{shuffler}-deck-owner-{owner}");
        let commitment = entropy_commitment(match_id, &deck_id, *shuffler, &opening);
        let output = encrypted_deck_hash(
            match_id,
            &deck_id,
            &format!("shuffle-by-p{shuffler}-{opening}"),
            &previous,
        );
        let proof_transcript_hash = shuffle_proof_transcript_hash(
            match_id,
            &deck_id,
            *shuffler,
            &previous,
            &output,
            &commitment,
        );
        steps.push(ShuffleStep {
            shuffler: *shuffler,
            input_deck_hash: previous,
            output_deck_hash: output.clone(),
            entropy_commitment: commitment,
            entropy_opening: opening,
            shuffle_proof: ShuffleProof::BayerGrothMentalPokerV1 {
                proof_transcript_hash,
                backend: ZIFFLE_0_1_BACKEND.to_string(),
            },
        });
        previous = output;
    }

    let slot_commitments = cards
        .iter()
        .enumerate()
        .map(|(slot, card)| {
            card_commitment(
                match_id,
                owner,
                slot as u16,
                card,
                &opening_salt(owner, slot as u16),
            )
        })
        .collect();

    Ok(DeckCeremony {
        owner,
        deck_id,
        declared_decklist_hash,
        initial_encrypted_deck_hash,
        required_shufflers,
        steps,
        final_encrypted_deck_hash: previous,
        slot_commitments,
    })
}

fn opening(
    match_id: &str,
    owner: u8,
    slot: u16,
    card: &str,
    visibility: Visibility,
) -> CardOpening {
    let salt = opening_salt(owner, slot);
    CardOpening {
        owner,
        slot,
        card: card.to_string(),
        commitment: card_commitment(match_id, owner, slot, card, &salt),
        salt,
        visibility,
    }
}

fn opening_salt(owner: u8, slot: u16) -> String {
    format!("fixture-card-salt-p{owner}-slot{slot}")
}

fn push_signed_action(
    actions: &mut Vec<ActionEnvelope>,
    match_id: &str,
    signing_keys: &[SigningKey],
    state_hash: &mut String,
    seq: u64,
    actor: u8,
    command: AuditCommand,
    openings: Vec<CardOpening>,
    rng_reveals: Vec<RngReveal>,
) -> Result<(), String> {
    let prev_state_hash = state_hash.clone();
    let next_state_hash = action_state_hash(
        match_id,
        seq,
        &prev_state_hash,
        &command,
        &openings,
        &rng_reveals,
    )?;
    let payload = ActionSigningPayload {
        match_id: match_id.to_string(),
        seq,
        actor,
        prev_state_hash: prev_state_hash.clone(),
        command: command.clone(),
        openings: openings.clone(),
        rng_reveals: rng_reveals.clone(),
        next_state_hash: next_state_hash.clone(),
    };
    let payload_bytes = canonical_bytes(&payload)?;
    let signature = signing_keys
        .get(actor as usize)
        .ok_or_else(|| format!("missing signing key for actor {actor}"))?
        .sign(&payload_bytes);
    actions.push(ActionEnvelope {
        seq,
        actor,
        prev_state_hash,
        command,
        openings,
        rng_reveals,
        next_state_hash: next_state_hash.clone(),
        signature: encode_hex(&signature.to_bytes()),
    });
    *state_hash = next_state_hash;
    Ok(())
}
