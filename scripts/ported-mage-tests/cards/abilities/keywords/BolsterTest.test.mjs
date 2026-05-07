import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/BolsterTest.java",
  "tests": [
    {
      "name": "Basic1Test",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Elite Scaleguard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 5
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Elite Scaleguard"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Elite Scaleguard",
          "counter": "P1P1",
          "count": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Elite Scaleguard",
          "power": 4,
          "toughness": 5
        }
      ]
    },
    {
      "name": "Basic2Test",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Elite Scaleguard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Elite Scaleguard"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Elite Scaleguard",
          "power": 2,
          "toughness": 3
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "counter": "P1P1",
          "count": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Silvercoat Lion",
          "power": 4,
          "toughness": 4
        }
      ]
    },
    {
      "name": "EliteScaleguardTriggerTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Elite Scaleguard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Pillarfield Ox",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Elite Scaleguard"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Silvercoat Lion",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Pillarfield Ox"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Elite Scaleguard",
          "power": 2,
          "toughness": 3
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "counter": "P1P1",
          "count": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Silvercoat Lion",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Silvercoat Lion\", true)"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Pillarfield Ox",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Pillarfield Ox\", true)"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 16
        }
      ]
    },
    {
      "name": "EliteScaleguardTriggerTwiceTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Elite Scaleguard",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Pillarfield Ox",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Elite Scaleguard"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Silvercoat Lion",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Pillarfield Ox"
        },
        {
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "Silvercoat Lion",
          "defender": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Pillarfield Ox"
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Elite Scaleguard",
          "power": 2,
          "toughness": 3
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "counter": "P1P1",
          "count": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Silvercoat Lion",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Silvercoat Lion\", true)"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Pillarfield Ox",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Pillarfield Ox\", true)"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 12
        }
      ]
    }
  ]
});
