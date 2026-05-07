import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/AngelsTombTest.java",
  "tests": [
    {
      "name": "testUnsummonToAnimatedArtifact",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angel's Tomb",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Unsummon",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Llanowar Elves"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Unsummon",
          "target": "Angel's Tomb"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Angel's Tomb"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Unsummon",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angel's Tomb",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Angel's Tomb",
          "power": 0,
          "toughness": 0
        }
      ]
    },
    {
      "name": "testUnsummonToAnimatedArtifact2",
      "operations": [
        {
          "op": "addCard",
          "zone": "Constants.Zone.BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "Constants.Zone.BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "Constants.Zone.BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "Constants.Zone.BATTLEFIELD",
          "player": 0,
          "name": "Angel's Tomb",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "Constants.Zone.HAND",
          "player": 0,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "Constants.Zone.HAND",
          "player": 0,
          "name": "Unsummon",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "Constants.PhaseStep.PRECOMBAT_MAIN",
          "player": 0,
          "name": "Llanowar Elves"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "Constants.PhaseStep.PRECOMBAT_MAIN",
          "player": 0,
          "name": "Unsummon",
          "target": "Angel's Tomb"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "Constants.PhaseStep.PRECOMBAT_MAIN",
          "player": 0,
          "name": "Angel's Tomb"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "Constants.PhaseStep.BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Unsummon",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angel's Tomb",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Angel's Tomb",
          "power": 0,
          "toughness": 0
        }
      ]
    }
  ]
});
