import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/StationTest.java",
  "tests": [
    {
      "name": "testNoStation",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Galvanizing Sawship",
          "count": 1
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
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Galvanizing Sawship",
          "counter": "CHARGE",
          "count": 0
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Galvanizing Sawship",
          "ability": "Haste",
          "expected": "0 >= 3"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Galvanizing Sawship",
          "ability": "Flying",
          "expected": "0 >= 3"
        },
        {
          "op": "unsupported",
          "source": "assertType(sawship, CardType.ARTIFACT, SubType.SPACECRAFT)"
        },
        {
          "op": "unsupported",
          "source": "assertType(sawship, CardType.CREATURE, isLeveled)"
        },
        {
          "op": "unsupported",
          "source": "if (isLeveled) { assertPowerToughness(playerA, sawship, 6, 5); }"
        }
      ]
    },
    {
      "name": "testStationInsufficient",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Galvanizing Sawship",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Riot Devils",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Station"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Riot Devils"
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
        },
        {
          "op": "unsupported",
          "source": "assertTapped(devils, true)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Galvanizing Sawship",
          "counter": "CHARGE",
          "count": 2
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Galvanizing Sawship",
          "ability": "Haste",
          "expected": "2 >= 3"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Galvanizing Sawship",
          "ability": "Flying",
          "expected": "2 >= 3"
        },
        {
          "op": "unsupported",
          "source": "assertType(sawship, CardType.ARTIFACT, SubType.SPACECRAFT)"
        },
        {
          "op": "unsupported",
          "source": "assertType(sawship, CardType.CREATURE, isLeveled)"
        },
        {
          "op": "unsupported",
          "source": "if (isLeveled) { assertPowerToughness(playerA, sawship, 6, 5); }"
        }
      ]
    },
    {
      "name": "testStationSufficient",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Galvanizing Sawship",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hill Giant",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Station"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Hill Giant"
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
        },
        {
          "op": "unsupported",
          "source": "assertTapped(giant, true)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Galvanizing Sawship",
          "counter": "CHARGE",
          "count": 3
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Galvanizing Sawship",
          "ability": "Haste",
          "expected": "3 >= 3"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Galvanizing Sawship",
          "ability": "Flying",
          "expected": "3 >= 3"
        },
        {
          "op": "unsupported",
          "source": "assertType(sawship, CardType.ARTIFACT, SubType.SPACECRAFT)"
        },
        {
          "op": "unsupported",
          "source": "assertType(sawship, CardType.CREATURE, isLeveled)"
        },
        {
          "op": "unsupported",
          "source": "if (isLeveled) { assertPowerToughness(playerA, sawship, 6, 5); }"
        }
      ]
    },
    {
      "name": "testTapestryWarden",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Galvanizing Sawship",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tapestry Warden",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Riot Devils",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Station"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Riot Devils"
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
        },
        {
          "op": "unsupported",
          "source": "assertTapped(devils, true)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Galvanizing Sawship",
          "counter": "CHARGE",
          "count": 3
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Galvanizing Sawship",
          "ability": "Haste",
          "expected": "3 >= 3"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Galvanizing Sawship",
          "ability": "Flying",
          "expected": "3 >= 3"
        },
        {
          "op": "unsupported",
          "source": "assertType(sawship, CardType.ARTIFACT, SubType.SPACECRAFT)"
        },
        {
          "op": "unsupported",
          "source": "assertType(sawship, CardType.CREATURE, isLeveled)"
        },
        {
          "op": "unsupported",
          "source": "if (isLeveled) { assertPowerToughness(playerA, sawship, 6, 5); }"
        }
      ]
    },
    {
      "name": "testEntropicBattlecruiser",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Entropic Battlecruiser",
          "count": 1
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
          "name": "Specter's Wail",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Wastes",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Station"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Balduvian Bears"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Specter's Wail"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        }
      ]
    }
  ]
});
