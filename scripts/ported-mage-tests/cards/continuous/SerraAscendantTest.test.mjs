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
          "player": 0,
          "name": "Plains",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "castSpell",
          "name": "Serra Ascendant",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Serra Ascendant",
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
          "op": "playLand",
          "player": 1,
          "name": "Stomping Ground",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "castSpell",
          "name": "Wild Nacatl",
          "player": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Wild Nacatl",
          "count": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Wild Nacatl",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "playLand",
          "player": 0,
          "name": "Plains",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "castSpell",
          "name": "Martyr of Sands",
          "player": 0,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Martyr of Sands",
          "count": 1,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=3"
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
          "op": "activateAbility",
          "source": "Martyr of Sands",
          "ability": "Reveal X white cards",
          "player": 0,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Martyr of Sands",
          "count": 1,
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 29
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Martyr of Sands",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Serra Ascendant",
          "power": 1,
          "toughness": 1
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
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 30
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
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
