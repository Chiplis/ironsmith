//! Authenticate a flat history of encrypted collections before composing a new
//! shuffle input. No wire field can manufacture a verified ciphertext handle.
use super::*;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct CiphertextRef {
    epoch: usize,
    position: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct InputEpoch {
    deck_count: usize,
    context: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sources: Option<Vec<CiphertextRef>>,
    steps: Vec<ZiffleShuffleStepInput>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ZiffleInputDeck {
    universe_count: usize,
    epochs: Vec<InputEpoch>,
    sources: Vec<CiphertextRef>,
}

pub(super) type ManifestRevealer = Rc<dyn Fn(AggregateRevealToken, MaskedCard) -> Option<usize>>;

#[derive(Clone)]
pub(super) struct PreparedInput {
    pub fingerprint: [u8; 32],
    pub cards: Vec<Verified<MaskedCard>>,
    pub universe_count: usize,
    pub reveal_manifest_card: ManifestRevealer,
    pub root_deck_hash: String,
    pub root_context: String,
}

#[derive(Clone)]
pub(super) struct VerifiedDeckExport {
    pub cards: Vec<Verified<MaskedCard>>,
    pub universe_count: usize,
    pub reveal_manifest_card: ManifestRevealer,
    pub root_deck_hash: String,
    pub root_context: String,
}

#[derive(Clone)]
struct VerifiedGraph {
    scope: [u8; 32],
    epoch_hashes: Vec<[u8; 32]>,
    decks: Vec<Rc<VerifiedDeckExport>>,
    consumed: HashSet<CiphertextRef>,
}

const MAX_EPOCHS: usize = 256;
const GRAPH_CACHE_CAPACITY: usize = 16;
type GraphCache = VecDeque<([u8; 32], Rc<VerifiedGraph>)>;
thread_local! {
    static PREPARED_INPUTS: RefCell<Vec<PreparedInput>> = const { RefCell::new(Vec::new()) };
    static VERIFIED_DECK_EXPORT: RefCell<Option<VerifiedDeckExport>> = const { RefCell::new(None) };
    static VERIFIED_GRAPHS: RefCell<GraphCache> = const { RefCell::new(VecDeque::new()) };
}

fn fingerprint(input: &ZiffleInputDeck) -> Result<[u8; 32], VerifierError> {
    Ok(Sha256::digest(encode(&("ironsmith-ziffle-input-deck-v1", input))?).into())
}

pub(super) fn prepared_input(input: &ZiffleInputDeck) -> Result<PreparedInput, VerifierError> {
    let key = fingerprint(input)?;
    PREPARED_INPUTS
        .with(|inputs| {
            inputs
                .borrow()
                .last()
                .filter(|prepared| prepared.fingerprint == key)
                .cloned()
        })
        .ok_or_else(|| {
            VerifierError::new(
                "authenticated input deck requires the chain-aware verifier dispatcher",
            )
        })
}

pub(super) fn export_verified_deck(deck: VerifiedDeckExport) {
    VERIFIED_DECK_EXPORT.with(|value| *value.borrow_mut() = Some(deck));
}

struct PreparedGuard;
impl Drop for PreparedGuard {
    fn drop(&mut self) {
        PREPARED_INPUTS.with(|inputs| {
            inputs.borrow_mut().pop();
        });
    }
}

fn with_prepared<T>(prepared: PreparedInput, call: impl FnOnce() -> T) -> T {
    PREPARED_INPUTS.with(|inputs| inputs.borrow_mut().push(prepared));
    let _guard = PreparedGuard;
    call()
}

fn select_cards(
    sources: &[CiphertextRef],
    count: usize,
    decks: &[Rc<VerifiedDeckExport>],
    consumed: &HashSet<CiphertextRef>,
) -> Result<Vec<Verified<MaskedCard>>, VerifierError> {
    if sources.len() != count {
        return Err(VerifierError::new(
            "authenticated source count does not match deck count",
        ));
    }
    let mut unique = HashSet::with_capacity(count);
    sources
        .iter()
        .map(|source| {
            if !unique.insert(*source) {
                return Err(VerifierError::new(
                    "duplicate authenticated ciphertext source",
                ));
            }
            if consumed.contains(source) {
                return Err(VerifierError::new(
                    "authenticated ciphertext source was already consumed",
                ));
            }
            decks
                .get(source.epoch)
                .and_then(|deck| deck.cards.get(source.position))
                .copied()
                .ok_or_else(|| {
                    VerifierError::new(
                        "authenticated ciphertext source is missing or refers forwards",
                    )
                })
        })
        .collect()
}

fn prepare_from_graph(
    input: &ZiffleInputDeck,
    count: usize,
    graph: &VerifiedGraph,
) -> Result<PreparedInput, VerifierError> {
    let root = graph
        .decks
        .first()
        .ok_or_else(|| VerifierError::new("authenticated input deck has no root"))?;
    Ok(PreparedInput {
        fingerprint: fingerprint(input)?,
        cards: select_cards(&input.sources, count, &graph.decks, &graph.consumed)?,
        universe_count: input.universe_count,
        reveal_manifest_card: Rc::clone(&root.reveal_manifest_card),
        root_deck_hash: root.root_deck_hash.clone(),
        root_context: root.root_context.clone(),
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChainRequest {
    deck_count: usize,
    context: String,
    #[serde(default)]
    key_context: String,
    keys: Vec<ZifflePublicKeyInput>,
    #[serde(default)]
    input_deck: Option<ZiffleInputDeck>,
}

/// Route each parent collection through the existing size-specialized proof
/// verifier, then make only its verified ciphertexts available to the current
/// operation. `dispatch` must call the ordinary shard dispatcher (not recurse
/// through this wrapper). Initial ceremony calls need no parent graph.
pub fn execute_with_input_chain(
    operation: Operation,
    input: &[u8],
    dispatch: impl Fn(Operation, &[u8]) -> Result<Vec<u8>, VerifierError>,
) -> Result<Vec<u8>, VerifierError> {
    if operation == Operation::Keygen {
        return execute_keygen(input);
    }
    let request: ChainRequest = decode(input, "ziffle chain")?;
    let Some(input_deck) = request.input_deck else {
        return dispatch(operation, input);
    };
    if !(2..=100).contains(&input_deck.universe_count)
        || !(2..=input_deck.universe_count).contains(&request.deck_count)
        || input_deck.epochs.is_empty()
        || input_deck.epochs.len() > MAX_EPOCHS
    {
        return Err(VerifierError::new(
            "invalid authenticated input universe or epoch count",
        ));
    }
    let key_context = ziffle_key_context(&request.key_context, &request.context);
    let scope: [u8; 32] = Sha256::digest(encode(&(
        "ironsmith-ziffle-verified-graph-scope-v1",
        input_deck.universe_count,
        key_context,
        &request.keys,
    ))?)
    .into();
    let epoch_hashes: Vec<[u8; 32]> = input_deck
        .epochs
        .iter()
        .map(|epoch| encode(epoch).map(|bytes| Sha256::digest(bytes).into()))
        .collect::<Result<_, _>>()?;
    let graph_key: [u8; 32] = Sha256::digest(encode(&(
        "ironsmith-ziffle-verified-graph-v1",
        scope,
        &epoch_hashes,
    ))?)
    .into();
    let cached = VERIFIED_GRAPHS.with(|cache| {
        let mut cache = cache.borrow_mut();
        let index = cache.iter().position(|(key, _)| *key == graph_key)?;
        let entry = cache.remove(index)?;
        let graph = Rc::clone(&entry.1);
        cache.push_back(entry);
        Some(graph)
    });
    let graph = if let Some(graph) = cached {
        graph
    } else {
        // A successful cached prefix is already authenticated. Reusing only
        // exact node hashes under the same roster, match and universe avoids
        // rechecking an ever-growing history on every appended shuffle. The
        // remainder is still checked node-by-node, including the consumption
        // ledger; a mutated parent cannot inherit trust from a valid sibling.
        let prefix = VERIFIED_GRAPHS.with(|cache| {
            cache
                .borrow()
                .iter()
                .map(|(_, graph)| graph)
                .filter(|graph| {
                    graph.scope == scope && epoch_hashes.starts_with(&graph.epoch_hashes)
                })
                .max_by_key(|graph| graph.epoch_hashes.len())
                .cloned()
        });
        let mut graph = prefix.as_deref().cloned().unwrap_or_else(|| VerifiedGraph {
            scope,
            epoch_hashes: Vec::new(),
            decks: Vec::new(),
            consumed: HashSet::new(),
        });
        for (index, epoch) in input_deck.epochs.iter().enumerate().skip(graph.decks.len()) {
            if !(2..=input_deck.universe_count).contains(&epoch.deck_count)
                || epoch.steps.len() != request.keys.len()
            {
                return Err(VerifierError::new(
                    "authenticated parent epoch has invalid size or incomplete shuffle",
                ));
            }
            let mut node = serde_json::json!({
                "deckCount": epoch.deck_count,
                "context": epoch.context,
                "keyContext": key_context,
                "keys": request.keys,
                "steps": epoch.steps,
            });
            let prepared = if index == 0 {
                if epoch.deck_count != input_deck.universe_count || epoch.sources.is_some() {
                    return Err(VerifierError::new(
                        "authenticated root must be the complete canonical manifest ceremony",
                    ));
                }
                None
            } else {
                let sources = epoch.sources.as_ref().ok_or_else(|| {
                    VerifierError::new("authenticated parent epoch has no sources")
                })?;
                let prefix = ZiffleInputDeck {
                    universe_count: input_deck.universe_count,
                    epochs: input_deck.epochs[..index].to_vec(),
                    sources: sources.clone(),
                };
                let prepared = prepare_from_graph(&prefix, epoch.deck_count, &graph)?;
                node["inputDeck"] = serde_json::to_value(&prefix).map_err(VerifierError::new)?;
                Some(prepared)
            };
            let node_bytes = encode(&node)?;
            VERIFIED_DECK_EXPORT.with(|value| *value.borrow_mut() = None);
            if let Some(prepared) = prepared {
                with_prepared(prepared, || dispatch(Operation::VerifyShuffle, &node_bytes))?;
            } else {
                dispatch(Operation::VerifyShuffle, &node_bytes)?;
            }
            let verified = VERIFIED_DECK_EXPORT
                .with(|value| value.borrow_mut().take())
                .ok_or_else(|| {
                    VerifierError::new("parent dispatcher did not export a verified deck")
                })?;
            if verified.cards.len() != epoch.deck_count
                || verified.universe_count != input_deck.universe_count
            {
                return Err(VerifierError::new(
                    "authenticated parent deck metadata mismatch",
                ));
            }
            if let Some(sources) = &epoch.sources {
                graph.consumed.extend(sources.iter().copied());
            }
            graph.decks.push(Rc::new(verified));
            graph.epoch_hashes.push(epoch_hashes[index]);
        }
        let graph = Rc::new(graph);
        VERIFIED_GRAPHS.with(|cache| {
            let mut cache = cache.borrow_mut();
            if cache.len() == GRAPH_CACHE_CAPACITY {
                cache.pop_front();
            }
            cache.push_back((graph_key, Rc::clone(&graph)));
        });
        graph
    };
    let prepared = prepare_from_graph(&input_deck, request.deck_count, &graph)?;
    with_prepared(prepared, || dispatch(operation, input))
}
