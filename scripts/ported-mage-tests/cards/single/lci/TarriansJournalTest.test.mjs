import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/lci/TarriansJournalTest.java",
  "tests": [
    {
      "name": "testFunctionality",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tarrian's Journal@journal",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Edgar Markov",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Kraken Hatchling@hatchling",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{2}, {T}, Discard your hand: Transform",
          "source": "Tarrian's Journal"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Kraken Hatchling",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "The Tomb of Aclazotz",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: You may cast a creature spell from your graveyard this turn. ",
          "source": "The Tomb of Aclazotz"
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "source": "The Tomb of Aclazotz",
          "count": 1,
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Kraken Hatchling",
          "holdStack": true,
          "objectAlias": "@hatchling",
          "permissionSource": "@journal"
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "source": "Kraken Hatchling",
          "count": 1,
          "isSpell": true
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "source": "Edgar Markov",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kraken Hatchling",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Vampire Token",
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@hatchling"
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Vampire",
              "expected": true
            }
          ]
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Kraken Hatchling",
          "counter": "FINALITY",
          "count": 1
        }
      ]
    }
  ]
});
