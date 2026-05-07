import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/lci/AbuelosAwakeningTest.java",
  "tests": [
    {
      "name": "testAbuelosAwakening",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Talisman of Progress",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Abuelo's Awakening",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Abuelo's Awakening",
          "target": "Talisman of Progress"
        },
        {
          "op": "unsupported",
          "source": "setChoiceAmount(playerA, 2)"
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
          "name": "Talisman of Progress",
          "counter": "P1P1",
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "assertType(talisman, CardType.CREATURE, true)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(talisman, SubType.SPIRIT)"
        },
        {
          "op": "unsupported",
          "source": "assertBasePowerToughness(playerA, talisman, 1, 1)"
        }
      ]
    },
    {
      "name": "testAbuelosAwakeningDies",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Talisman of Progress",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Abuelo's Awakening",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Abuelo's Awakening",
          "target": "Talisman of Progress"
        },
        {
          "op": "unsupported",
          "source": "setChoiceAmount(playerA, 2)"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Lightning Bolt",
          "target": "Talisman of Progress"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Talisman of Progress",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Talisman of Progress",
          "count": 1
        }
      ]
    }
  ]
});
