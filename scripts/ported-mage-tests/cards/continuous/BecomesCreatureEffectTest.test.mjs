import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/BecomesCreatureEffectTest.java",
  "tests": [
    {
      "name": "testBecomesCreatureAllEffect",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Dryad Arbor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ambush Commander",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ambush Commander"
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
      "name": "testBecomesCreatureAttachedEffect",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Dryad Arbor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Frogify",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Frogify",
          "target": "Dryad Arbor"
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
          "source": "assertAbilities(playerA, dryadArbor, Collections.emptyList())"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Dryad Arbor",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(dryadArbor, CardType.CREATURE, SubType.FROG)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(dryadArbor, SubType.DRYAD)"
        },
        {
          "op": "unsupported",
          "source": "assertNotType(dryadArbor, CardType.LAND)"
        },
        {
          "op": "unsupported",
          "source": "assertColor(playerA, dryadArbor, ObjectColor.BLUE, true)"
        },
        {
          "op": "unsupported",
          "source": "assertColor(playerA, dryadArbor, ObjectColor.GREEN, false)"
        }
      ]
    }
  ]
});
