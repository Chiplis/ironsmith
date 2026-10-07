# NEXT04 evidence-only compatibility boundary

Source-only CLEAR at `0bfe423cabf555b0d59ccf8422574b479fa84025`, tree `2146c272e585781fde1fa4d5ae16d8bed558916c`; all executable scenarios UNRUN / UNVALIDATED.

The boundary remains artifact version 14, public digest version 9, signed audit version 27 and Manabrew version 3. Schema descriptor SHA-256 `292e6db310f90613f13024fb4d135e405483443b81ef85b38f04e6755f6fdd7f` is unchanged from published PR869 and remains the immediate parent descriptor. No new descriptor or version bump is justified: this packet changes only authored tests, fixtures, reports, cfg(test) wiring, dev-dependencies and corresponding lockfile bookkeeping. Production owner and model behavior and serialized schemas are unchanged. Production-path test wiring is not represented as an unchanged whole-file tree.

Historical signed-byte verification does not grant current replay or gameplay admission. Reusing valid signatures does not authenticate the current engine, and relabeling old artifacts is not regeneration. Genuine version 14 artifacts and catalogs, original version 5 provisioning, matching compiler, engine, WASM, glue and layout outputs, native savepoints and exact local images, and authenticated suffix or accepted-genesis replay remain deferred validation gates. Public digest checkpoints are not gameplay restores.

The original source gate remains closed: 1,281 identities are eligible against a threshold of 1,597 out of 3,193 original identities, leaving a gap of 316. All measured outcomes and residual coverage of 146 IDs / 148 entries remain unchanged. Source proposal admission is not executable validation. See [admission](card-failure-next-series-04-source-admission.md).
