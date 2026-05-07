import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/dies/ChronozoaTest.java",
  "tests": [
    {
      "name": "testTriggerOtherCreature",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Chronozoa",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Chronozoa",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Viscera Seer",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Chronozoa"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "Sacrifice a creature"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Viscera Seer"
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
          "name": "Viscera Seer",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Chronozoa",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Chronozoa",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "count": 0,
          "name": 5
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 0
        }
      ]
    }
  ]
});
