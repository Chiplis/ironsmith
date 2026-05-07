import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/uds/OpalescenceTest.java",
  "tests": [
    {
      "name": "testOpalescenceApplies",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Dockside Chef",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Alms",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Opalescence",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Amok",
          "count": 1
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Dockside Chef",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Alms\", CardType.CREATURE, true)"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Alms",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Amok\", CardType.CREATURE, true)"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Amok",
          "power": 2,
          "toughness": 2
        }
      ]
    },
    {
      "name": "testOpalescenceEffectEnds",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Dockside Chef",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Alms",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Opalescence",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Vindicate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Scrubland",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Amok",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Opalescence"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Vindicate",
          "target": "Opalescence"
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
          "source": "assertType(\"Dockside Chef\", CardType.CREATURE, true)"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Dockside Chef",
          "power": 1,
          "toughness": 2
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Alms\", CardType.CREATURE, false)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Amok\", CardType.CREATURE, false)"
        }
      ]
    }
  ]
});
