# Single-source shared-amount damage recipient sets

Status: **UNVALIDATED.** This is source implementation plus authored tests. No builds, compiler runs, or tests were executed.

## Exact proposed coverage

Two frozen full-card identities are in `fixtures/single_source_damage_set.json.fixture`:

- Friendly Fire: the randomly revealed card's mana value, dealt by the spell to the earlier targeted creature and that creature's controller. The revealed card remains a separate prior-result object; it must not replace either damage recipient.
- Volcanic Eruption: the count of Mountains that actually reached a graveyard through the preceding destroy instruction, dealt once to every creature and every player. Protected or replacement-exiled Mountains are not successful graveyard results.

Both previously failed when the shared recipient phrase was split before its suffix damage quantity. Expected future status is strict metadata-bearing, non-lossy full-card compilation and the gameplay behavior below. Together with f42af1cc's four quantities, these are the six reserved candidates; do not count the pair twice.

## Mechanism

`DamageActionAst::DealDamageToRecipients` represents a shared amount with referenced recipients, object groups, and player groups. Its grammar reads the complete suffix-amount clause before generic `and` decomposition. It admits normalized self sources, prior references and quantified groups; it rejects fresh target declarations and different damage sources.

`DealDamageToRecipientsEffect` samples the amount once, resolves the complete union before mutation, deduplicates recipient identities, and calls the existing one-source simultaneous damage pipeline. It does not execute a loop of independent damage effects. Replacement/prevention ordering, per-recipient outcomes and a single source's lifelink result stay in that pipeline. The new typed payload is registered in both artifact decoder paths, card-graph mapping, native encoding, semantic lowering, and rendering.

Reference traversal visits the explicit recipient references and keeps the earlier battlefield creature distinct from a newly revealed hand card. The shared value reader reuses the existing typed prior-result FirstManaValue metric with action Revealed. Volcanic Eruption uses the existing PutIntoGraveyard metric and the destroy executor's actual successful graveyard object memory, rather than requested X or the announced target count.

The new semantic variant is appended; no existing serialized ordinals move.

## Authored, unrun evidence

- Normal compiler-runtime integration `single_source_damage_set`: six tests, exact metadata and direct/JSON artifact materialization; real random reveal with a deterministic one-card hand; both zero and nonzero mana value; original creature/controller versus unrelated objects; real X=3 cast with normal, indestructible and replacement-exiled Mountains; union deduplication; a dynamic life-total amount sampled once despite lifelink; one lifelink event; independent prevention shields; zero/empty set behavior.
- Normal tools integration `single_source_damage_set`: both complete frozen cards aggregated for strict/non-lossy compilation.
- Grammar: typed shared packet rather than sequential effects, exact prior metrics, no accidental fresh target or alternate source, complete operand/tail consumption.

## Deliberate limits

This does not close the separate multiple-source simultaneous-damage family (Coordinated Clobbering, Friendly Rivalry, Tandem Takedown, Terrific Team-Up). Different damage sources require a distinct multi-source packet owner. Existing native numeric bounds remain final correctness limitations, not a claim of arbitrary-precision game rules.

No global prevention-allocation changes were made. The existing limited-shield allocation helper was reviewed, but extending a synthetic pooled player/permanent shield is not justified by merely assuming every multi-recipient shield shares one capacity. Rules 615.7 and 615.11 distinguish a shield on one recipient from separately created shields; damage processing and lifelink event boundaries are governed by 120.4 and 702.15. See the [official September 25, 2026 Comprehensive Rules](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt). The authored scenarios use independent recipient shields.
