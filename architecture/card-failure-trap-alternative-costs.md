# Trap alternative costs and completed-action history

Status: eleven source-complete proposals, **UNVALIDATED**. No compilation, build or test execution. `fixtures/trap_alternative_costs.json.fixture` preserves exact frozen oracle IDs, metadata and complete bodies.

## Cohort and typed implementation

- Archive Trap: an opponent actually searched their own library this turn. Searcher and library owner are separate roles; failed-to-find remains a completed search.
- Arrow Volley Trap, Lethargy Trap, Nemesis Trap, Pitfall Trap, Slingbow Trap: quantified current attacking-object filters, including exact count, color and flying constraints.
- Baloth Cage Trap, Permafrost Trap, Whiplash Trap: event-time entry characteristics and controller. Whiplash takes the maximum for a single opponent, never the sum across opponents.
- Cobra Trap: successful destruction of a matching permanent, with its pre-destruction controller and frozen action cause. Sacrifice, an own-controller effect, indestructibility and ordinary creature destruction do not prove this condition.
- Summoning Trap: exact spell incarnation was cast by the player this turn and subsequently countered by an opponent's spell/ability. The original caster is separate from the spell's controller when countered, and an uncast copy cannot borrow its original's cast event.

All predicates enter the existing `ThisSpellCostCondition::ConditionExpr` and composed alternative-cost path. They do not grant a new casting zone, combine alternative prices or replace normal cost selection. Normal taxes are added after the chosen price, and the announced price remains locked through the mana-ability/payment window.

Four `TurnHistoryCount` variants are appended after the current frontier. Typed filter/reference walkers, relative-player validation, runtime scalar resolution and text visitors handle them. The new event cause fields are optional; legacy constructors remain valid and cannot prove a cause they never captured.

## Required producer corrections

Completed `DestroyEvent` and `SpellCounteredEvent` notifications retain `EventCause`, including the action controller. They preserve source LKI when the source is already gone. Historical source predicates inspect retained snapshots, never a new live incarnation.

A successful destruction can have its graveyard destination replaced by exile. The destroy producer now emits the completed action for such a move while preserving existing UI behavior. This distinction is stated in the official [Spider-Man release notes](https://www.magic.wizards.com/en/news/feature/spider-man-release-notes), including the Rest in Peace and Saw in Half rulings. Regeneration replaces the destruction itself; indestructibility prevents it.

Battlefield entry commonly emits both a zone-change notification and a specialized ETB notification. The fallible completed-operation owner now freezes the whole original batch after every entrant and simultaneous timestamp choice, before deferred replacement programs. It covers queued entries, reported token events, and both land-play paths. All raw entry rows are staged first; a checked continuous query snapshot supplies the completed frames. Discovery failure propagates through the original transaction's rollback instead of publishing a partial frame. Simultaneous player-action holds defer this capture until the outer owner finishes the group.

`ZoneChangeEvent.destination_snapshots` and `destination_snapshot(ObjectId)` expose the same exact destination receipt to trigger predicates. Existing `snapshot`/`snapshots` remain origin LKI, and per-object split views retain only their own destinations. The history reader deduplicates exact destination ObjectIds; later leave/reentry remains a different entry.

Alternative-cost discovery and self-cost pricing use an explicit prospective caster. A foreign-owned exiled card's stored controller cannot supply its “you” or “opponent” context; legacy on-stack callers retain their controller-derived wrapper.

## Full bodies

Source review found existing executable primitives for Arrow Volley's announced damage allocation and Nemesis's copy from departed-target LKI with next-end-step token exile. The other bodies use existing mill, token creation, mass temporary stats, targeted destruction, tap/next-untap restriction, library choice and return primitives. None is omitted or replaced by a placeholder.

## Authored regressions

`crates/ironsmith-compiler-runtime/tests/trap_alternative_costs.rs` retains all eleven full programs through direct strict compilation and typed-artifact JSON, followed by real announcement, mana payment and stack resolution.

Scenarios cover every complete body, actual attack declarations, 3/2 divided damage, per-opponent three-player entry counts, A-owned/B-controlled entry and reentry, qualifying-source departure, searcher/owner/turn negatives, destroyed-to-exile, own effects/sacrifice/indestructibility negatives, original caster versus stolen spell controller, countering an uncast copy, token copiable values and delayed exile, normal versus alternative price plus taxes, and a mana ability sacrificing an attacker after the alternative cost locks. Review regressions add foreign-owned Archive Trap permission, both chosen timestamp orders for simultaneous color-changing entrants, colorless tokens under a continuous color effect that departs in the next instruction, and typed continuous-discovery failure with instruction/history rollback.

Deferred command:

`cargo test -p ironsmith-compiler-runtime --test trap_alternative_costs -- --nocapture`

The count remains a source proposal pending the shared regression build and frozen full replay. Game Over and unrelated spell-cost quantity/predicate families remain in their existing lanes.

### Outer simultaneous owner correction

An open simultaneous-action scope also defers entry freezing, independently of trigger-matching holds. The entry helper closes only its own scope; an enclosing each-player owner freezes all reported and queued entries unconditionally after its scope/lookback closes. The each-player ReturnAll proposal now locks its selected exact card IDs at preparation and returns a completion receipt for added programs, following the existing Mill interface. Its additions cannot run during the first player's original commit. An authored regression returns Bob's colorless creature before Carol's global green setter and checks Bob's completed green entry receipt.

The same split now covers the CreateToken simultaneous proposal. Its player/count/template are locked before participants mutate state; creation replacements prepare without executing appended programs; original tokens and entry receipts commit before the outer owner freezes anything. A completion then runs entry additions followed by token-creation additions. Trigger capture will not drain an open simultaneous action. Authored negative scenarios attach an addition to Bob's token creation or entry: it must count all three original tokens, then remove their shared color source, while all entry receipts still record green.

The split token proposal carries one charged instruction permit through preparation, commit and completion. Nesting is guarded only while that participant executes a phase; sibling preparation never accumulates nesting depth. Native ForPlayers execution now opens the shared resource transaction, while dispatched execution reuses its existing meter. Direct CreateToken execution keeps its original single charge. Authored native/dispatcher tests cover exhausted 0/1-instruction budgets, three siblings at nesting limit 1, and a replacement-created token requiring nesting level 2 during completion.
