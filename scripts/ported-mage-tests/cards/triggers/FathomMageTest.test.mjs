import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/FathomMageTest.java",
  "tests": [
    {
      "name": "testDrawCardsAddedCounters",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blessings of Nature",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fathom Mage",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 5
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blessings of Nature",
          "target": "Fathom Mage"
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
          "name": "Fathom Mage",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Fathom Mage",
          "power": 5,
          "toughness": 5
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 4
        }
      ]
    },
    {
      "name": "testDrawCardsEntersTheBattlefield",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Fathom Mage",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Master Biomancer",
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
          "name": "Island",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Fathom Mage",
          "holdStack": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Fathom Mage",
          "count": 1,
          "isSpell": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Fathom Mage",
          "counter": "+1/+1",
          "count": 2
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Fathom Mage",
          "count": 2,
          "abilityKind": "Triggered"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
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
          "name": "Fathom Mage",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Fathom Mage",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 2
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Fathom Mage"
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Mutant",
              "expected": true
            }
          ]
        }
      ]
    }
  ]
});
