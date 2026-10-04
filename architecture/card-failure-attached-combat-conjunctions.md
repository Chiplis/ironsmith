# Attached stat, keyword, and attack-rule conjunctions

Source-only proposal for the six Vow Auras: Duty, Flight, Lightning, Malice, Torment, and Wildness. No builds, tests, compilation probes, or corpus replay were run.

The existing complete continuing-anthem reader now accepts an unquoted `can't attack you [or planeswalkers you control]` segment. It retains the stat modifier, each keyword, and the full restriction. Unknown extra tails are rejected. The rule is a static restriction on the anthem's affected filter, controlled by the granter, rather than an ability granted to the enchanted creature. The exact attached-source predicate prevents one Aura from affecting another Aura's host. Existing source conditions remain conjunctive.

Attack legality now passes the actual attack-target kind through declaration, preview, and the pending attack-cost diagnostic owner. A battle's protector remains its defending player for ordinary defending-player ability restrictions, opponent relations and direction, but is not treated as an attacked player/planeswalker for this narrower prohibition. The older defender-only API retains its documented player/planeswalker meaning.

Authored direct/artifact scenarios exercise each paid Aura cast and actual attachment, stat/keyword preservation, controller changes, reattachment, departure, host versus Aura ability loss, phasing, and preview/real-declaration consistency for players, planeswalkers and battles. All scenarios are unrun. This fixture remains partial until bounded independent source review.

Independent bounded source review cleared `c26435d84` plus `943e96764`. The correction retains raw quote ownership before segment edge trimming, so a quoted restriction can never use the new granter-owned shortcut. Six exact identities are proposed/unvalidated in stage52; all execution remains deferred.
