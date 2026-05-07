import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/conditional/FaceOfDivinityTest.java",
  "tests": [
    {
      "name": "test_BoostCondition",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Face of Divinity",
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
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Aether Tunnel",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {W}",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {W}",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {W}",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Face of Divinity",
          "target": "Balduvian Bears"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "BEGIN_COMBAT",
          "power": 0,
          "toughness": "Balduvian Bears"
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "BEGIN_COMBAT",
          "ability": 0,
          "expected": "Balduvian Bears"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Aether Tunnel",
          "target": "Balduvian Bears"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "END_TURN",
          "power": 0,
          "toughness": "Balduvian Bears"
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "END_TURN",
          "ability": 0,
          "expected": "Balduvian Bears"
        },
        {
          "op": "setStopAt",
          "turn": 1,
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
