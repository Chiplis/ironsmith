# Complete attached control subjects

Source proposal: Domineer, Steal Enchantment and In Bolas's Clutches. All
scenarios are authored and unrun; no build, compiler probe, test, formatter or
corpus execution occurred. No coverage credit before independent review.

Independent review is complete through `649cba7a391479e7cad5c1da48dd2fc168316076`;
integration at `57bbf8f495802a1a3e2a56dd76fa589fcf176537` preserves all five reviewed
file contents exactly. The three identities are source proposals only.

The shared attached-subject grammar retains complete `enchanted artifact
creature` and `enchanted enchantment` nouns. The longer compound is recognized
before the artifact prefix. Full sentence consumption remains required. The
existing ControlAttachedPermanent model supplies live layer-two control from
the Aura's controller and exact attachment; display wording does not decide
runtime behavior. No new runtime payload or wire enum is introduced.

In Bolas's Clutches additionally uses the already implemented complete supertype
assertion owner for its attached permanent, plus its own printed Legendary
supertype. The three separate Enchant domains remain the spell's complete
target/attachment restrictions. Control does not change ownership or write a
lasting resolved spell-control effect.

Native/direct/artifact scenarios cover real paid Aura casting and attachment,
the compound Artifact+Creature target constraint, unrelated target kinds,
live controller and attachment changes, phasing and source departure, native
cloning, target blink before resolution and legendary removal with the source.
The ability-loss witness explicitly invokes state-based cleanup after losing
Enchant; it does not infer a layer-two rollback directly from layer-six loss.
Unsupported trailing control clauses remain errors. These witnesses and existing
shared control/layer/attachment owners remain subject to deferred execution.

Stage95 also records nine known source holds found after stage94 publication:
Tester of the Tangential and Cytoplast Manipulator await the corrected shared
counter-transfer original/completion owner and its compatibility boundary;
Adipose Offspring, Baloth Cage Trap, Cobra Trap, Camellia, the Seedmiser,
Belisarius Cawl, Brood Birthing and Oni-Cult Anvil await corrected subtype-derived
token names. The three additions and nine holds yield 1,203 proposed unique
identities and 1,208 entries (the previous publication snapshot was 1,209/1,214).
Measured recoveries remain 40 and failures 3,193. No hold is treated as fixed by
this Aura-only code increment. The artifact7 / public digest3 / signed audit20
boundary is unchanged.
