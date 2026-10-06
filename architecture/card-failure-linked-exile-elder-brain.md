# Elder Brain: original hand arrivals and exact casting permission

Status: source implementation and authored scenarios, UNVALIDATED and unrun.
No builds, compiler probes, tests, formatters or engine/corpus execution.
Depends on the source-cleared exact casting prerequisite through 2a181efc3
(tree 2492023c9ebb831e3da61f7717148259ca9e2f7e). The local equivalent base is
944d26862. The original frozen corpus and failed-card baseline are unchanged.

The complete frozen body is retained in linked_exile_static_permissions.json.fixture,
oracle identity e537bd23-7368-459b-badf-b7b7c112c88a. Menace remains an ordinary
keyword. The frozen body binds both the hand owner and draw recipient through the
attack trigger's captured attacked-player reference;
attacking a planeswalker does not satisfy an attacks-a-player trigger.

The new grammar composition recognizes one hand-exile instruction with an
explicit owner filter, a relative-player draw referring to its count, and one persistent play permission
with an explicit cast-this-way mana rider. It accepts the canonical sentence
boundaries and inline mana-rider form independently of the original phrasing.
The compound “play lands and cast spells” target and conditional mana rider
are grammar facts. No card name, display label, Debug representation or
source-wide union determines the relation.

The draw is a PriorEffectMetric over the exact producer's OriginalZoneMoveCards
with destination Exile. ExileEffect now publishes those existing typed facts
before replacement additions, including an explicit known-empty receipt.
Redirected/prevented movements do not count; another replacement instruction's
exiles do not count; an original card that subsequently leaves exile still
counts for the draw. This extends the existing destination-qualified query
owner without changing the old ExileEffect numeric outcome or cost behavior.

A semantic permission_bound_mana flag survives reference resolution and
lowering into the reviewed tagged grant. Lowering requires the same exact
producer collection. Actual card incarnations still in exile receive both land
and spell authority. Their casting conversion is AnyColor, with native exact
selection and a frozen casting receipt. A red mana unit cannot pay a colorless
pip. Ordinary timing, land limits, printed mana costs and additional costs
remain required. The beneficiary is fixed when the trigger resolves; later
source control changes, phasing or departure do not expire this for-as-long-as-
exiled duration. Exile departure ends authority for that card incarnation.

The standalone grammar helper does not migrate unrelated legacy tagged play
routes. Intellect Devourer and Rogue Class remain partial and uncounted; the
separate unsupported shuffled face-down exile-pile interaction remains held.
This packet itself remains uncounted pending independent full-body source
review. No execution success is claimed.

Authored full-body direct/artifact/native scenarios cover: Menace and illegal
single blocking; player versus planeswalker attacks; empty and nonempty hands;
multiplayer actor scope; actual-arrival draw counts under redirected or added
replacement instructions; a departed arrival; normal timing, lands, additional
discard, colored and colorless mana; copied pending triggers/source blink;
fixed-beneficiary control/phasing lifetime; card leave/reentry; rollback and
native clone recovery after a replacement failure; missing receipt/tag errors;
and independent canonical render/reparse. Grammar positives and bounded
negative variants are also authored. All remain unrun.
