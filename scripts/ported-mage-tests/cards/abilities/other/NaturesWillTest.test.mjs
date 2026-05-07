import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/other/NaturesWillTest.java",
  "tests": [
    {
      "name": "testAttackMultiplePlayers",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Nature's Will",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Suntail Hawk",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Island",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Nature's Will"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Grizzly Bears",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Suntail Hawk",
          "defender": "playerC"
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
          "op": "assertTappedCount",
          "name": "Forest",
          "tapped": false,
          "count": 4
        },
        {
          "op": "assertTappedCount",
          "name": "Mountain",
          "tapped": true,
          "count": 4
        },
        {
          "op": "assertTappedCount",
          "name": "Island",
          "tapped": true,
          "count": 4
        }
      ]
    },
    {
      "name": "testAttackOnePlayer",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Nature's Will",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Island",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Nature's Will"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Grizzly Bears",
          "defender": 1
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
          "op": "assertTappedCount",
          "name": "Forest",
          "tapped": false,
          "count": 4
        },
        {
          "op": "assertTappedCount",
          "name": "Mountain",
          "tapped": true,
          "count": 4
        },
        {
          "op": "assertTappedCount",
          "name": "Island",
          "tapped": false,
          "count": 4
        }
      ]
    }
  ]
});
