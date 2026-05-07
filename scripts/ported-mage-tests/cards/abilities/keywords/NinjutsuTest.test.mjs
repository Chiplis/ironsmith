import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/NinjutsuTest.java",
  "tests": [
    {
      "name": "testMultipleUsage",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Seacoast Drake",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Jwari Scuttler",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Moonblade Shinobi",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Seacoast Drake",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Jwari Scuttler",
          "defender": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "DECLARE_BLOCKERS",
          "player": 0,
          "ability": "Ninjutsu"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Seacoast Drake"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "DECLARE_BLOCKERS",
          "player": 0,
          "ability": "Ninjutsu"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Jwari Scuttler"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Moonblade Shinobi",
          "power": 3,
          "toughness": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Illusion Token",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Seacoast Drake",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Jwari Scuttler",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Island",
          "tapped": true,
          "count": 6
        }
      ]
    }
  ]
});
