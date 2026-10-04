# Exact fight operands and completed simultaneous damage

Source-only correction; normal runtime/native scenarios are authored and unrun.
No independent new card coverage is claimed by this common-owner correction.

`FightEffect` now treats a saved target assignment, including an empty range, as
authoritative. It cannot select another surviving target to replace an illegal
or unchosen fighter. Exact assignments take priority over compatible descriptors;
an ambiguous fallback is an explicit error. Repeated identical saved slots keep
their operand order. A mutual tagged group with one surviving member does not
silently become a self-fight. Legacy flat-pair support is limited to old untagged,
untargeted descriptor invocations; explicit Source/Tagged/Specific identities do
not inherit unrelated targets.

Both powers, current source characteristics, controllers and recipients are
captured before the damage event. The two real assignments enter the existing
completed damage batch, retaining replacement order, occurrence identities,
lifelink and original-before-addition semantics. A genuine self-fight remains
one assignment for twice the power. Native representation overflow produces a
typed incomplete error rather than saturation. Keyword fight observers are
captured once before added programs can remove them. Whole-game/context
checkpoints roll back every original and receipt on a pause or error.

Authored cases include illegal first/second saved slots with overlapping friendly
filters, identical target descriptors, an unchosen optional second operand,
mutual groups with a departed member, self-fight overflow and a paused damage
replacement-order choice. Public direct/artifact spell scenarios change control
or blink the first fighter before resolution, and remove the first fighter and a
damage observer through a first-side replacement addition. The latter requires
both original damage receipts, one lifelink gain, and both already-matched
recipient triggers. Existing infect/wither, calculated power, noncreature and
self-fight tests continue to exercise the same owner.

The direct Fight owner establishes a checked control/characteristic discovery
frame before operand/type/power reads. Its transaction shares the existing native
resource meter across both sides and all replacement additions. A nonconvergent
self-regrant/power graph must return `ContinuousDiscovery`, preserve game and
context, and permit a clean retry after the graph is corrected. Both direct and
generic execution routes have authored, unrun regression cases.
