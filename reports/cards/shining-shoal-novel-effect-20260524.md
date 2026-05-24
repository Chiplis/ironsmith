## Shining Shoal - single-card worker failure report

- Card: `Shining Shoal`
- Date (UTC): `2026-05-24`
- Worker mode: AWS fleet single-card pass

### Parse failure reproduced

Command:

```bash
cargo run -p ironsmith-tools --bin compile_oracle_text -- --name "Shining Shoal" --compare-text
```

Observed error:

```text
parse failed for Shining Shoal: parser does not yet support line family:
'The next X damage that a source of your choice would deal to you and/or
creatures you control this turn is dealt to any target instead.'
```

### Why parser-only is insufficient

This clause needs a reusable runtime replacement capability that does not exist in
current primitives:

1. Choose a source on resolution (`a source of your choice`).
2. Create an amount-limited shield (`next X damage`).
3. Match damage to a mixed protected set (`you and/or creatures you control`).
4. Redirect matched damage to a chosen destination (`any target`).

Existing redirect/prevention support is close but not enough:

- `RedirectNextDamageToTargetEffect` currently assumes damage to `this creature`
  (self-only matcher).
- `RedirectNextTimeDamageToSourceEffect` handles next-time source-constrained
  redirection for one protected target, not next-X across player + creatures.
- Existing prevent-next-time effects do not provide this combined
  source-constrained redirect model.

### Required follow-up capability

Implement a reusable runtime effect family for source-constrained,
amount-limited redirection that can match damage to controller and creatures they
control, then redirect to a chosen legal target.

This is beyond parser-only single-card scope, so no parser/runtime approximation
was committed.
