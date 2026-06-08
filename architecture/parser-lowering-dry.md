# Parser and Lowering DRY Checklist

Use this checklist when touching Ironsmith parser, semantic AST, preparation, or lowering code.

## Boundaries

- Surface text recognition belongs in the front end. Lowering should consume CST, `RewriteSemanticItem`, `LineAst`, `EffectAst`, predicates, references, and typed metadata.
- Parser facts that change behavior must be typed. Do not smuggle them through `ParseAnnotations`, presentation labels, or arbitrary strings.
- `ParseAnnotations` is for diagnostics, source spans, original/normalized text maps, and presentation support.
- Parser and semantic phases should not construct executable runtime objects unless the code is legacy. Prefer declarative AST shapes and let lowering build runtime `Ability`, `StaticAbility`, and `Effect` values.

## Effect AST

- Recursive `EffectAst` walks should use the shared traversal helpers or a shared helper built on them.
- Do not duplicate wrapper-variant lists in feature code.
- Prefer reusable game-operation primitives over whole oracle-text recipe variants in `SubjectVerbActionAst`.
- Add bespoke AST variants only when the behavior is truly atomic or cannot be represented without semantic loss.

## Parser Rules

- New parser special cases should be named rules in the closest rule registry, with head hints where possible.
- Post-parse repairs are legacy escape hatches. Add one only when the shape genuinely needs cross-sentence context.
- Repeated phrase alternatives are acceptable when they keep grammar local and diagnostics clear. The DRY target is duplicated semantic decisions, not every repeated English phrase.

## Tests

- Add parser tests for the local grammar shape.
- Add lowering/runtime tests for behavior-changing AST or lowering changes.
- Add or update workspace-boundary tests only for high-signal mechanical rules with low false positives.
