import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/dies/ReefWormTest.java",
  "tests": [
    {
      "name": "testDiesTriggeredAbility",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Reef Worm",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "Reef Worm"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "Fish Token"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "Whale Token"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "Whale Token"
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 4
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Fish Token",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Whale Token",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Kraken Token",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Reef Worm",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 4
        }
      ]
    }
  ]
});
