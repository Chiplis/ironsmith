# Timed draw-replacement programs

Status: source-authored, UNVALIDATED. No build, compilation, test or replay was
run. Seven exact full-body candidates are retained in
`fixtures/timed_draw_replacements.json.fixture`: Words of War, Words of Waste,
Words of Wilding, Words of Wind, Words of Worship, Plagiarize, and Urabrask,
Heretic Praetor. Their recovery prerequisite was integrated and independently source-reviewed
in stage43. Words of Wilding also uses the source-reviewed shared typed
token-resource owner. All corresponding runtime validation remains deferred. No measured recovery is asserted.

## Complete grammar and registration

A strict typed instruction reader owns either the next single draw this turn
or every matching draw until cleanup. Leading and trailing `instead` forms are
mutually exclusive. The entire following program remains inside registration;
Urabrask's later play-permission sentence must not execute prematurely. The
legacy triggered-line wrapper cannot wrap an already registered program twice.

The new typed player target is announced with the registering spell/ability.
Targets inside the future program are also announced at that time, through its
normal child-effect metadata. The resolving registration captures the resolved
player and original targets, assignments, X, and object/player tags. The program
has its own future execution scope and retains the current replacement-history
keys, original source/controller, actual future drawer and event values. It does
not borrow the interrupted draw instruction's unrelated target or tag context.

One-shot registrations use the existing replacement-consumption mechanism and
end-turn cleanup. Several Words activations remain independent choices; applying
one does not consume the others. Plagiarize remains multi-use through cleanup,
including when its chosen player is also its controller. The applied replacement
identity prevents recursive self-application to the resulting draw.

## Target and source lifetime

Words of War's target is chosen on activation and checked when that ability
resolves. If the ability fails to resolve, no replacement is installed. Once
installed, a later missing/non-damageable recipient produces the existing legal
zero-damage outcome; the replaced draw is not restored and no new target is
chosen. Actual damage still uses protection/prevention and the original damage
source. Source control changes do not change the controller captured by the
registration. Current source characteristics prevail while it exists; otherwise
the latest zone-departure, leave-game, or phase-out event supplies LKI, with the resolving
source snapshot as the final fallback.

These scopes follow [CR 115, 121.6, 608.2, and 614–616](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt).
The seven bodies replace individual draw proposals. Alms Collector's whole
multi-card instruction, first-draw-per-turn latches, optional replacement offers,
and ordered face-down exile piles remain separate and are not counted here.
Aladdin's Lamp and Ring of Ma'rûf are also not proposed by this batch.

## Authored evidence, all unrun

Three grammar scenarios cover exact header/duration ownership, nested later
permissions, and rejection of batch/optional/first-time/missing-duration forms.
Fourteen runtime scenarios use complete frozen direct/restored artifacts, real
Words activation/payment/announcement, actual Plagiarize spell resolution and
both Urabrask upkeep triggers. Cases cover independent consumption, cleanup,
controller changes, missing targets before/after registration, source departure
and phasing LKI, actual discard/return/token actions, self-Plagiarize, future
exile/play permissions and pending replacement-choice rollback. The bound child
uses the shared per-instruction replacement executor: prefix observers are
captured before later children can remove them, with pending/error rollback
remaining owned by DrawCards. Follow-up scenarios cover a stolen Words of War
whose owner leaves after it gained lifelink, prefix receipts surviving observer
destruction exactly once (including pending replay), and preservation of the
actual future draw event rather than the registration-time event.

The final build, full corpus, native/Wasm/browser and regression gates remain
deferred. These authored expectations are not executed evidence.
