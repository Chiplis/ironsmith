import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/cmr/AkromasWillTest.java",
  "tests": [
    {
      "name": "test_OneMode",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Akroma's Will",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kitesail Corsair",
          "count": 1
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "ability": 0,
          "expected": "Kitesail Corsair"
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "ability": 0,
          "expected": "Kitesail Corsair"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Akroma's Will"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "1"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "ability": 0,
          "expected": "Kitesail Corsair"
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "ability": 0,
          "expected": "Kitesail Corsair"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "test_MultiModesOnCommander",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Akroma's Will",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "COMMAND",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kitesail Corsair",
          "count": 1
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "ability": 0,
          "expected": "Kitesail Corsair"
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "ability": 0,
          "expected": "Kitesail Corsair"
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Akroma's Will"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "2"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "1"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "ability": 0,
          "expected": "Kitesail Corsair"
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "ability": 0,
          "expected": "Kitesail Corsair"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ]
    }
  ]
});
