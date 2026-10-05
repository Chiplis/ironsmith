# Rope: live maximum-blocker rule checkpoint

Parent checkpoint: `d2018867afdddc5243c0852c7a12446028ade5f7`. The earlier six-card review remains separate and unchanged in history.

This checkpoint closes the known source-layer prerequisite for Rope. `Restriction::MaximumBlockers { filter, maximum }` is appended to the existing enum, preserving prior serialized discriminants. Its filter participates in the existing tag/reference, iterated-player, resolution, and text-rendering paths. Static rules populate a derived per-attacker maximum, merged by taking the lowest bound and cleared on each restriction rebuild. Combat declaration and legal-blocker queries combine this rule bound with intrinsic/granted maximum-blocker abilities.

The unquoted compound-tail and keyword-plus-limit owners now emit a restriction owned by the source. They retain the live attached-recipient filter and any whole-clause condition. Quoted granted abilities retain their original ownership. Rope's reach can disappear when its recipient loses abilities while its +1/+2 modifier and blocking rule remain. Removing/phasing the Equipment or removing its static ability removes the rule.

The frozen Rope body continues through the independent direct and serialized-artifact routes authored in `compound_static_bodies.rs`. Native scenarios now exercise recipient ability loss, one-vs-two actual blocker declarations, source phasing and source ability loss, multiple independent Ropes, reattachment, controller changes, source departure, real equip payment, and sacrifice/draw. Existing parser assertions now require a source-owned typed restriction instead of a removable grant.

No builds, tests, compiler probes, formatters, corpus runs, or matrix edits were performed. These are authored, unexecuted scenarios and source inspection, not verified runtime recovery. Rope is now a complete-body source candidate for independent review.
