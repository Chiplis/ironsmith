import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/SerraAscendantTest.java",
  "tests": [
    {
      "name": "testSilence",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Serra Ascendant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Martyr of Sands",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Stomping Ground",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Wild Nacatl",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Plains"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Serra Ascendant"
        },
        {
          "op": "playLand",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Stomping Ground"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Wild Nacatl"
        },
        {
          "op": "playLand",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Plains"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Martyr of Sands"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}, Reveal X white cards from your hand"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Silvercoat Lion"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Silvercoat Lion"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Silvercoat Lion"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Serra Ascendant",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Wild Nacatl",
          "attacker": "Serra Ascendant"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Martyr of Sands",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 30
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Wild Nacatl",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Serra Ascendant",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Serra Ascendant",
          "power": 6,
          "toughness": 6
        }
      ]
    }
  ]
});
