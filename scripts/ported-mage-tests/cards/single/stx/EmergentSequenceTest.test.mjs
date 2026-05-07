import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/stx/EmergentSequenceTest.java",
  "tests": [
    {
      "name": "test_PlayFirst",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
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
          "zone": "HAND",
          "player": 0,
          "name": "Emergent Sequence",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Emergent Sequence"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Swamp"
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
          "source": "assertType(\"Swamp\", CardType.CREATURE, true)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Swamp",
          "counter": "P1P1",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Swamp",
          "power": 1,
          "toughness": 1
        }
      ]
    },
    {
      "name": "test_PlayAfterLand",
      "operations": [
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
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
          "zone": "HAND",
          "player": 0,
          "name": "Emergent Sequence",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Island"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Emergent Sequence"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Swamp"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Swamp\", CardType.CREATURE, true)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Swamp",
          "counter": "P1P1",
          "count": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Swamp",
          "power": 2,
          "toughness": 2
        }
      ]
    }
  ]
});
