import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/other/SoulConduitTest.java",
  "tests": [
    {
      "name": "basicTwoPlayerGame_SoulConduitEffect",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 9
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Soul Conduit",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Live Fast",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Live Fast",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Live Fast",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Live Fast",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 18
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 0,
          "count": 2,
          "counter": "energy"
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": 1,
          "count": 0,
          "counter": "energy"
        },
        {
          "op": "activateAbility",
          "source": "Soul Conduit",
          "ability": "Two target players exchange life totals",
          "player": 0,
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "target": {
            "groups": [[0, 1]]
          }
        },
        {
          "op": "assertStackObject",
          "source": "Soul Conduit",
          "count": 1,
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "abilityKind": "Activated",
          "targetCount": 2
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "Soul Conduit",
          "tapped": true
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        }
      ]
    }
  ]
});
