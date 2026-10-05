# Static top-card visibility during announcement

UNVALIDATED source correction; no build, compilation or tests were executed.

The earlier scoped-prohibition proposal **Experimental Frenzy** and the five
pending filtered-library candidates share this prerequisite. An ongoing static
look/reveal permission must not disclose a changed top card during a spell,
activation or land-play announcement/payment chain. The
[official MKM release notes](https://magic.wizards.com/en/news/feature/murders-at-karlov-manor-release-notes)
state this boundary in the Assemble the Players entry. This patch adds no
whole-card proposal or measured recovery.

The Wasm host derives the boundary from its existing exact
`pending_action_checkpoint`, with the native pending-action checkpoint as a
fallback. The window compares the per-player top revision and object identity.
A shuffle additionally invalidates the earlier hidden-position knowledge even
if an offline permutation happens to leave the same card on top. A pending
action lacking its original snapshot cannot authorize a new static peek.
Unchanged tops and unrelated players' unchanged libraries stay available.

The same window gates both production crypto opening/view-window preparation
and both production snapshot encoders. It reaches the persistent-look list,
the top-card display, and the permission flag before any of those are built.
A window hash invalidates the UI cache when the action completes, even if no
further gameplay mutation occurs. Explicit instruction-driven views keep their
existing separate owner. No peer claim, unsigned hash, new checkpoint carrier,
or unverified opening grants authority.

Completion exposes the new top through the normal authenticated opening path;
rollback restores the original state and clears the pending owner. The existing
same-attempt disclosure journal/recovery verification is unchanged. Four unrun
production-path scenarios cover private/public views before and after completion,
unchanged/other-player tops, shuffle with the same final top, rollback, and a
missing checkpoint. They call the opening-preparation owner and the native JSON
snapshot encoder, rather than checking only local cached names.

## Native announcement follow-up

A bounded review found that the host-only checkpoint was insufficient: casts
started inside a resolving effect use a local priority state, and land plays
are intentionally outside the host's cancelable-action checkpoint. The native
follow-up captures exact per-player top identity/revision and shuffle chronology
inside GameState at each cast, activation, mana-ability payment and both land-play
owners. Exact decision/runtime clones retain these independent nested boundaries.
Each successful owner removes only its boundary; pending/error rollback restores
the prior state. Land entry additions remain inside the land-play boundary.

Wasm now consults those native boundaries before its host fallback. Active
boundaries refuse wire checkpoint export, including a pending decision clone;
old imports lacking the new local completeness field fail closed. This field is
not a peer assertion authorizing recovery: the verified signed-transcript replay
route remains the recovery authority. Exact runtime savepoints retain the frames.

Two additional unrun production-path scenarios start a real CastSource operation
and both native land-play routes with an entry draw/choice program, capture their
actual suspended decision states, and inspect opening preparation/UI with no host
pending action. The cast starts after the resolving instruction has already
changed the library, proving the boundary comes from the nested announcement.
The source entry point for effect-authorized casts is private and separate from
ordinary UI actions, so a legitimate immediate cast needs no standing grant while
an ordinary named-source action still does.
