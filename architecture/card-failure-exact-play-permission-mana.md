# Exact play-permission mana prerequisite

Status: staged source work, UNVALIDATED. No builds, compiler probes, tests,
formatters or engine/corpus execution. Base:
098c0aec81116bcbd754ee91ca06fef2bc38ebe3.

This first packet defines the permission-local mana mode and a fallible exact
selection/receipt owner. GrantSpec mappings and stored/static grant collection
retain the mode. Normal is omitted from serialized default carriers, preserving
old payload shapes. The exact resolver checks the selected index against its
immutable GrantPermissionIdentity, source, player, actual face/origin and use
limit. It cannot adopt a different reader on the same host. Its native receipt
freezes the payer, origin ObjectId, source, identity and constraints before a
provider or live grant list can change.

This is an unactivated prerequisite, not a complete card proposal. No grammar
emits a non-normal permission-local mode yet, and the existing tagged-grant
runtime continues its old unmarked mana route. The new receipt is not yet
installed into casting or recovery state. No new casting/action encoding or
public audit format is introduced in this packet. Artifact 6, public checkpoint
3 and audit 19 remain unchanged here.

The next packet must connect exact selection to display/prospective legality,
announcement, selected-cost computation, actual mana payment, native root and
inactive-lane savepoints, and accepted signed replay. A public action encoding
change requires an explicit compatibility decision backed by the actual
carrier. Only after those owners and each complete frozen body are reviewed may
Intellect Devourer, Rogue Class or Elder Brain be proposed. Those three remain
partial and uncounted.

Authored unrun native scenarios cover two grants on one host with independent
modes, stale positions, wrong player, missing identity, exile reincarnation,
receipt immutability after provider removal, native clone recovery, and omitted
old/default versus explicit new-mode serialization. These scenarios validate
the prerequisite contract only; they do not claim a complete cast or payment.
