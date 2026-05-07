import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/watchers/IanMalcolmChaoticianTests.java",
  "tests": [
    {
      "name": "testManaCostsandWatcher",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ian Malcolm, Chaotician",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Avaricious Dragon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Avaricious Dragon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Avaricious Dragon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Avaricious Dragon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Horde of Notions",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Fusion Elemental",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": "playerC",
          "name": "Chromanticore",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": "playerD",
          "name": "Garth One-Eye",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 15
        },
        {
          "op": "assertPlayableAbility",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Fusion Elemental",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Chromanticore",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Garth One-Eye",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Horde of Notions",
          "expected": false
        },
        {
          "op": "castSpell",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Fusion Elemental"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Chromanticore",
          "expected": false
        },
        {
          "op": "assertPlayableAbility",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Garth One-Eye",
          "expected": false
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "testBlink",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ian Malcolm, Chaotician",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Heightened Awareness",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Heightened Awareness",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Heightened Awareness",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Heightened Awareness",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Horde of Notions",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Ephemerate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Fusion Elemental",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": "playerC",
          "name": "Chromanticore",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": "playerD",
          "name": "Garth One-Eye",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 15
        },
        {
          "op": "assertPlayableAbility",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Fusion Elemental",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Chromanticore",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Garth One-Eye",
          "expected": true
        },
        {
          "op": "assertPlayableAbility",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Horde of Notions",
          "expected": false
        },
        {
          "op": "castSpell",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ephemerate"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Ian Malcolm, Chaotician"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Fusion Elemental",
          "expected": false
        },
        {
          "op": "assertPlayableAbility",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Chromanticore",
          "expected": false
        },
        {
          "op": "assertPlayableAbility",
          "turn": 5,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Garth One-Eye",
          "expected": false
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        }
      ]
    }
  ]
});
