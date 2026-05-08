import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/rules/NamePredicateTest.java",
  "tests": [
    {
      "name": "test_SearchPermanentsByName",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Pine Walker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Pine Walker using Morph"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertPermanentCount",
          "player": 0,
          "count": 4
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "EmptyNames.FACE_DOWN_CREATURE.getTestCommand()",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertNamePredicate(\"by rules - empty choice must return zero\", 0, \"\", false)"
        },
        {
          "op": "unsupported",
          "source": "assertNamePredicate(\"by rules - face down choice must return zero\", 0, EmptyNames.FACE_DOWN_CREATURE.getTestCommand(), false)"
        },
        {
          "op": "unsupported",
          "source": "assertNamePredicate(\"by rules - non existing name must return zero\", 0, \"Island\", false)"
        },
        {
          "op": "unsupported",
          "source": "assertNamePredicate(\"by rules - existing name must work\", 3, \"Forest\", false)"
        },
        {
          "op": "unsupported",
          "source": "assertNamePredicate(\"by inner - non existing name must return zero\", 0, \"Island\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertNamePredicate(\"by inner - existing name must work\", 3, \"Forest\", true)"
        }
      ]
    },
    {
      "name": "testCityInABottle",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Camel",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "City in a Bottle",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Desert Nomads",
          "count": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Desert Nomads",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "City in a Bottle"
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Camel",
          "count": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "label": "Cast Desert Nomads",
          "expected": false
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
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "City in a Bottle",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Mountain",
          "count": 5
        }
      ]
    }
  ]
});
