# Serverless, Cheaterless Magic

Most online games solve cheating by trusting a server. The server owns the
truth, validates player actions, rolls the random numbers, stores hidden state,
and tells every client what it is allowed to see. That is a reasonable default.
It is also expensive, operationally heavy, and not the only possible model.

Ironsmith is an experiment in a different direction: can a browser game for
Magic-like rules run without an authoritative game server, while still making
cheating either impossible to hide or impossible to accept?

The short answer is: for honest peers, yes, if the engine is deterministic, the
hidden information is cryptographically committed, the randomness is jointly
generated, and every state transition is locally replayable. This is not a
claim that a malicious player can be forced to stay online, or that cryptography
can fix bugs in the rules engine. A player can always close a laptop. But a
patched client should not be able to make an honest browser accept an illegal
move, a biased shuffle, a forged reveal, or a different public game state.

That puts Ironsmith at the intersection of two old ideas: mental poker and
machine-readable Magic.

## Mental poker and zero-knowledge proofs, briefly

Mental poker is the name for a family of protocols that let people play a card
game remotely without trusting a dealer. The problem goes back to Shamir,
Rivest, and Adleman in the late 1970s: if Alice and Bob are not in the same
room, and neither trusts the other, how can they shuffle and deal a deck without
one of them stacking it or peeking at it?

The naive solution is to appoint a server as dealer. The mental-poker answer is
more interesting: encrypt the deck, let each player contribute to the shuffle,
and reveal cards only when the rules say they should become visible. No single
player gets unilateral control over the deck order, and no player learns hidden
cards just because they helped shuffle.

The catch is that "I shuffled this encrypted deck fairly" is not something
another client can verify by inspection. A shuffled encrypted deck should look
like random data. If the verifier can see the permutation, the shuffle leaks
information; if the verifier cannot see anything, the shuffler might cheat.

That is where zero-knowledge proofs enter. In this setting, a zero-knowledge
shuffle proof lets a client prove that the output deck is a valid permutation
and re-randomization of the input deck, without revealing the permutation. The
proof says: "I transformed the deck correctly," while hiding: "this old position
became that new position."

This is not theoretical folklore. Libraries such as
[LibTMCG](https://libtmcg.nongnu.org/) implement trusted-third-party-free card
game protocols, and Ironsmith uses the Rust
[ziffle](https://docs.rs/crate/ziffle/latest) mental-poker library for
multi-party shuffles with zero-knowledge proofs.

Poker is the classic example because poker has a simple deck model and a small
rules surface. Magic is the stress test.

## Why Magic is a hostile target

Magic: The Gathering is a famously large card game, but card count is only the
obvious part of the difficulty. The hard part is that cards are tiny programs
written in natural language and interpreted against a living game state.

A card can create a replacement effect. Another can change the rules for when a
player may cast spells. Another can reveal the top card of one library to one
player, all players, or no one depending on a continuous effect. Cards can copy
other cards, change control, change characteristics in layers, set up delayed
triggers, ask players to make choices, move hidden objects without revealing
them, or reveal objects temporarily and hide them again.

The academic results are correspondingly strange. Churchill, Biderman, and
Herrick showed that real Magic is at least as hard as the Halting Problem in
constructed positions, in
[Magic: The Gathering is Turing Complete](https://arxiv.org/abs/1904.09828).
Stella Biderman later pushed further in
[Magic: the Gathering is as Hard as Arithmetic](https://arxiv.org/abs/2003.05119).
Wizards maintains the
[Comprehensive Rules](https://magic.wizards.com/en/rules) as a reference for
the rules and corner cases. A 2026 Scryfall-based analysis counted 33,998 unique
card designs as of April 2026 in
[Thirty Years of Magic Cards, Measured](https://gatheringdata.blog/blog/mtg-distributions/).

Those numbers matter less than the shape of the problem. Magic is not just a
deck plus a hand plus betting rounds. It is a mutable programming language for
game state, with hidden information woven through many of its effects.

## What Ironsmith is

Ironsmith has three major pieces.

The first is the engine. This is the deterministic runtime that tracks players,
zones, objects, the stack, priority, turns, choices, mana, damage, combat,
triggers, replacement effects, prevention effects, continuous effects, and
state-based actions. The engine is the referee. Given the same starting state
and the same command stream, every peer should arrive at the same result.

The second is the parser. Magic cards are written for humans, not machines.
Ironsmith tokenizes oracle text, recognizes common grammar, and turns text such
as "destroy target creature" or "whenever another creature enters" into typed
semantic structures. The parser is not supposed to smuggle behavior through
string labels. The project tries to promote card wording into reusable,
structured facts.

The third is the compiler. Parsed card text is lowered into engine behavior:
spell effects, activated abilities, triggered abilities, static abilities,
replacement effects, target requirements, choices, and continuous
modifications. The goal is not to hand-code every card as a bespoke script. The
goal is to make common Magic language compile into shared engine primitives, so
improving support for a mechanic improves many cards at once.

The browser UI runs the Rust engine through WebAssembly. That matters for
multiplayer because all peers can run the same engine locally. There is no need
for a private server process to decide whether an action was legal. If a client
sends "cast this spell," every other client can replay that command in its own
WASM instance and reject it if the engine says it was not legal.

That deterministic engine is the base layer. The multiplayer protocol adds the
cryptographic layer around it.

## The multiplayer problem: Magic changes who knows what

A serverless Magic client has to answer four questions at the same time:

1. Was the action legal?
2. Did the action produce the same public game state for everyone?
3. Was every newly revealed hidden card the card that had been committed before
   the game?
4. Did each player learn exactly the private information they were entitled to
   learn, and no more?

The first question is an engine question. The other three are audit questions.

In a normal client-server architecture, the server simply knows everything. It
knows the library order, every hand, every face-down card, every random result,
and every private choice. The server can redact information when sending state
to clients.

Ironsmith does not give that power to a server. The lobby host helps assemble
the match and relay messages, but the host is not trusted to sequence the game,
shuffle a library, roll a die, inspect a hand, or decide which action is
canonical. Every browser keeps its own transcript and verifies every action
before mutating local state.

The hard part is that Magic does not have one hidden-information rule. It has a
large family of information transitions.

A card can move from library to hand without becoming public. A card can move
from library to battlefield face up, becoming public. A player can look at the
top card of their own library. All players can reveal cards from the top of a
library until some condition is met. One player can look at another player's
hand. A spell can instruct a player to search a library, reveal one chosen card,
then shuffle. A continuous effect can make the top card of a library visible for
as long as a permanent remains on the battlefield. A replacement effect can
change where a hidden card goes. A delayed trigger can reveal something later.

If the protocol treats all of those as "show the card" or "do not show the
card," it fails. The actual problem is preserving the information boundary
across each engine transition.

Ironsmith handles that by having the engine emit crypto requirements.

When the WASM engine applies a command, it knows what hidden-information
boundary the rules just crossed. It does not merely say "state changed." It can
say, in effect:

- this hidden card became public;
- this hidden card became known to one specific player;
- this group of hidden cards became publicly viewable as a batch;
- this group of hidden cards became privately viewable by one player;
- this hidden object moved while remaining hidden.

The browser audit layer names these cases as `public_open`, `private_open`,
`public_view_window`, `private_view_window`, and `hidden_move`.

That vocabulary is the bridge between Magic's effect system and the
cryptographic transcript. The engine decides what the rules require. The audit
layer decides what proof must be attached before honest peers accept the action.

### Match genesis: committing before anyone acts

Before the game starts, each player creates several public identities for this
match.

There is an audit signing key. It signs genesis records, actions, quorum votes,
randomness commitments, randomness reveals, timeout votes, disconnect votes, and
resync envelopes.

There is an encryption key for private views. It lets a player send hidden-card
material only to the player who is allowed to see it.

There is a ziffle key for mental-poker shuffling. It comes with an ownership
proof bound to the match context, so another player cannot silently swap in a
different key later.

Each deck is also committed. For every main-deck slot, the owner creates a
salted commitment to the card in that slot. The public deck manifest contains
the owner, counts, decklist commitment, commitment root, and one commitment per
slot. The private manifest keeps the card names and salts needed to open those
slots later.

The important property is temporal: the commitments exist before the game
starts. If a player later reveals that slot 17 was Lightning Bolt, every peer
can recompute the commitment from the match id, owner, slot, card name, and
salt. If it does not match the genesis commitment, the reveal is rejected.

This prevents a client from deciding what a hidden card was after seeing how the
game developed.

### Libraries: no trusted shuffle

A committed decklist is not enough. The protocol must also prevent a player from
stacking their library.

For each player's library, Ironsmith runs a ziffle shuffle ceremony. Conceptually,
the deck begins as encrypted original slots. Then every player contributes a
shuffle step in a deterministic player order. Each step produces a new encrypted
deck and a zero-knowledge proof that the new deck is a valid shuffle of the
previous one.

No single player controls the final order. If at least one participant honestly
contributes entropy, the deck order is not chosen by the deck owner. And because
the shuffle proofs hide the permutation, helping shuffle an opponent's library
does not reveal that library.

Later, when a card at a shuffled position must be opened, the relevant ziffle
reveal links the shuffled position back to the committed original slot. The
opening still has to satisfy the ordinary deck commitment. This gives the
protocol two layers of evidence:

- the position came from the jointly verified shuffled deck;
- the original slot opens to the card committed at genesis.

That is the core mental-poker move inside Ironsmith. The deck is not an array
owned by a server. It is a jointly shuffled, proof-carrying commitment structure
that the engine consumes as hidden library objects.

### Actions: signed commands plus local replay

Every game action is wrapped in a signed audit envelope.

The envelope commits to the match id, sequence number, acting player, previous
state hash, exact command, clock audit, hidden-card openings, randomness
reveals, shuffle proofs, private-view proofs, the resulting public checkpoint
hash, and the next audit state hash.

Those bytes are canonicalized before hashing or signing. This matters because
cryptographic protocols fail if two browsers serialize the same logical object
differently. Ironsmith signs canonical JSON: sorted keys, normalized values,
domain-separated hashes, and canonical low-S ECDSA signatures.

When a peer receives an action, it does not ask "did this come from the host?"
It asks a stricter set of questions:

- is this the next sequence number I expected?
- does the signature belong to the player who is allowed to act?
- does the action extend the previous audit hash?
- does the command match the signed payload exactly?
- does the engine accept this command from my current local state?
- did the command emit crypto requirements?
- does the audit envelope satisfy those requirements?
- after I apply it, does my redacted public checkpoint hash equal the signed
  public checkpoint hash?

If any answer is no, the action does not become local state.

This is the main difference between "the client sends a move" and "the client
sends a verifiable transition." A malicious browser can still send arbitrary
bytes. It cannot make another honest browser mutate state unless those bytes
survive signature verification, transcript verification, engine replay, crypto
proof checks, and public checkpoint consensus.

### Public openings: when hidden becomes visible to all

The simplest information transition is `public_open`: a hidden object becomes
public.

Examples include a card being revealed, milled face up, cast from a hidden zone,
or otherwise moved into a public zone with its identity exposed. The engine
emits a requirement saying which hidden object must be opened. The action audit
must include an opening for that object.

For a deck card, that opening contains enough material to prove the identity:
the owner, committed slot, card name, salt, and, when the card came from a
ziffle-shuffled library position, the position-opening proof that links the
runtime library position to the committed slot.

Every peer verifies the opening. If the card says it is Lightning Bolt but the
salted commitment says otherwise, the action is invalid. If the card came from a
library position but the ziffle proof does not link that position to the slot,
the action is invalid. If the opening is missing, the action is invalid.

The public checkpoint then includes the card identity, because everyone is now
entitled to know it.

### Private openings: when only one player may know

Drawing a card is harder than revealing a card.

When Alice draws from her library, Alice must learn the card. Bob must not.
But Bob still needs confidence that Alice did not choose the card, invent the
card, or draw from a different hidden position.

That is a `private_open`. The action transcript includes a proof object that is
publicly bound to the action, but the actual card material is encrypted to the
authorized viewer's encryption key. Alice can decrypt it and verify the card
against the deck commitment. Bob cannot decrypt the card name, but he can verify
the signed envelope, the public parts of the proof, the engine transition, and
the resulting public checkpoint.

This is subtle. The non-viewer is not asked to trust the viewer. The non-viewer
is asked to trust a narrower statement: "the action carried a private-view proof
bound into the signed transcript, addressed to the legal viewer, and the public
state after the action matches my local replay." If the match is later exported
with disclosures, the encrypted private openings can be checked after the fact.

In live play, that separation preserves secrecy. In audit mode, it preserves
accountability.

### View windows: Magic often reveals sets, not single cards

Many Magic effects do not reveal or inspect one card. They create a temporary
view over a set of hidden cards.

"Look at the top three cards of your library." "Reveal cards from the top of
your library until you reveal a creature." "Target opponent reveals their hand."
"You may play with the top card of your library revealed." These are different
rules, but they all require the engine to describe a window of visibility.

Ironsmith separates public and private view windows.

A `public_view_window` means every player is entitled to see the batch. The
audit must include public openings for the relevant objects, and the resulting
public checkpoint can expose them for as long as the rules say they remain
visible.

A `private_view_window` means only a specific viewer is entitled to see the
batch. The proof material is encrypted to that viewer, but the transcript still
commits to the window. Other players can verify that a private view happened
under the signed action and that the public state after the action is the one
their engine replay produced.

This distinction is what lets the protocol model Magic's richer effects without
flattening them into either "everyone sees" or "nobody verifies." The engine
describes who may see what. The cryptographic layer enforces that description.

### Hidden moves: preserving identity without revealing it

Some transitions move a hidden card without revealing it.

A library card can move to hand. A face-down object can move between zones.
Cards can be reordered, tucked, shuffled, manifested, or otherwise transformed
while their identity remains hidden from at least some players.

For these cases, opening the card would be a privacy bug. But doing nothing
would be an audit bug, because the hidden object still needs continuity. The
system has to know that the object after the move is the same committed hidden
object as before the move, or an honestly derived hidden object according to the
engine's rules.

That is the role of `hidden_move`. Instead of revealing card identity, the engine
tracks hidden-card metadata through the transition: owner, hidden slot or
runtime commitment, zone, and the public redaction needed for the checkpoint.
Peers can agree that a hidden object moved without learning what the object is.

The public checkpoint intentionally redacts hidden identities. It is not a full
state dump. It is a public-state fingerprint: enough to prove that everyone
agrees on life totals, visible objects, zones, stack, turn structure, choices,
and redacted hidden-object positions, without leaking card names that are still
private.

### Randomness outside libraries

Libraries are not the only source of randomness. Magic-like games can require
random choices, shuffles of derived groups, or other non-library random events.

Ironsmith handles non-library randomness with signed commit/reveal. Each player
first signs a commitment to a random nonce. After all commitments are present,
players reveal their nonces. The combined seed is derived from all valid
reveals. Since no player knows the other nonces when committing, no single
client can choose the final outcome after seeing everyone else's input.

The reveal transcript is attached to the action that consumes the randomness.
Peers verify the signatures, the nonce-to-commitment matches, and the derived
seed.

Again, the pattern is the same: do not trust one participant to be the source of
truth. Make the thing that changes state carry the evidence needed to verify it.

### Quorum, forks, and what the protocol does not promise

For three- and four-player games, actions also carry peer quorum certificates.
A quorum vote signs the action's match id, sequence, actor, previous hash, next
hash, public checkpoint hash, and action signature. Peers refuse to sign
conflicting votes for the same sequence.

This is not there because the engine cannot validate actions. It is there to
reduce host sequencing authority. The host may relay messages, but the host
should not be able to unilaterally decide which action everyone treats as the
next canonical transition.

There is still an important limit: a peer-to-peer protocol cannot prevent every
form of liveness failure. A malicious player can disconnect. A malicious player
can refuse to provide a reveal. A malicious player can try to fork the action
log by sending different signed messages to different peers. The goal is not to
make those behaviors impossible in the physical sense. The goal is to make them
rejectable or provable.

If a fork appears, the transcript can carry the two signed actions with the same
sequence and previous hash. That is dispute evidence. Honest clients can show
which actor equivocated, and in quorum games, which voters signed both branches.

This is why "cheaterless" has to be understood precisely. Ironsmith does not
make bad network behavior disappear. It makes accepted state transitions
verifiable. If someone cheats by sending invalid data, honest peers reject it.
If someone equivocates, the signatures expose it. If someone disconnects, the
protocol can move toward timeout or forfeit policy, but it cannot force their
machine to keep participating.

### Resync and postgame verification

Peer-to-peer games need resync. Browsers refresh. Connections drop. A player can
fall behind and need the current transcript.

Ironsmith's resync payload is not "trust me, here is the state." It is a signed
transcript segment plus a checkpoint. The receiving client verifies the
transcript hash chain, validates signatures and quorum certificates, verifies
the resync envelope, then replays the actions through its local engine. Only if
the replayed public checkpoint hash matches the signed checkpoint does the
client accept the resync.

The same principle applies after the match. A complete audit transcript can be
verified later, including engine replay and, when available, private-view
disclosures. This is useful for debugging, dispute resolution, and making the
system understandable to people who did not participate in the match.

The transcript is the artifact. The server is not.

### Why the architecture fits Magic

The central design choice is that Ironsmith does not try to build one giant
cryptographic protocol for "a game of Magic." That would be brittle. Magic has
too many effects, too many information boundaries, and too many interactions.

Instead, the deterministic engine remains responsible for rules. It decides
what actions are legal and what information changes when they resolve. The
audit layer remains responsible for evidence. It checks that every hidden-card
opening, private view, shuffle proof, random reveal, action signature, quorum
vote, and checkpoint hash matches the transition the engine produced.

That separation is what makes the problem tractable:

- the parser and compiler turn card text into reusable engine behavior;
- the engine turns player commands into deterministic state transitions and
  crypto requirements;
- the multiplayer audit layer turns those requirements into signed, replayable,
  cryptographically checkable evidence;
- each browser independently verifies the same transition before accepting it.

In other words, Ironsmith does not need a central server because the authority
has been decomposed. Legality comes from deterministic replay. Hidden-card
honesty comes from commitments and openings. Library fairness comes from
multi-party zero-knowledge shuffles. Randomness comes from commit/reveal.
Private information comes from per-viewer encryption. Public consensus comes
from redacted checkpoint hashes. Fork accountability comes from signed
transcripts.

That is the actual shape of serverless anticheat for a complex card game. It is
not one trick. It is a stack of small, specific checks aligned with the places
where Magic changes state and changes knowledge.

The result is a system where the host can help peers find each other, but cannot
secretly become the referee. A patched client can lie, but its lies have to
survive every other player's engine and every cryptographic commitment already
made. That is a much narrower attack surface than "the JavaScript client said
so."

And for a game as weird as Magic, narrowing the attack surface is the whole
game.
