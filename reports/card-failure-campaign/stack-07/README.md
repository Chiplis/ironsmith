# First repair-stack measurement

The exact frozen 32,209-entry corpus was replayed at source `bc9e56e2cf228f158060631927cab8f42d9b6ebc` (the verified tree of draft #772). Forty previously unresolved entries, also forty unique Oracle cards, pass the authoritative strict/non-lossy gate. 3,198 entries / 3,193 unique baseline cards remain. No previously supported card regressed in compile acceptance or semantic scoring. The run took 9m26s.

Eleven entries now expose the explicit allow-unsupported fallback and remain failures. They are not counted among the forty recoveries. Complete linked-face coverage and runtime closure remain open.

`milestone.json` records the compact measurement, source and corpus digests, and full-snapshot digest. `comparison.json` preserves every recovered and still-unresolved entry without duplicating the large raw snapshot or volatile allocation-ID list. The frozen corpus and original baseline remain in fixtures; rerun the existing campaign harness to reproduce this measurement.

The raw Debug-definition hash changed for 13,833 entries. For 13,775, the only difference is a numeric StaticAbilityInstanceId allocation. The remaining 58 reconcile to forty recoveries, eleven permissive fallbacks, six supported landwalk corrections, and a Boseiju Channel resolution-segment boundary change. This one-off normalization does not alter the raw snapshot or acceptance gate. Boseiju receives a gameplay comparison against the former one-segment shape before closing semantic review.

Focused scenarios and broad unit logs are archived with individual SHA-256 values. Broad grammar failures exactly match the 38 frozen-main failures; the runtime library's existing Bilious Skulldweller/Toxic assertion also reproduces on frozen main. These are explicit pre-existing failures, not a claimed all-green suite. Later-batch tests and fixes have separate evidence.

The current stack is draft #766 through #772, each based on its predecessor. No merges or deployments are part of this campaign.
