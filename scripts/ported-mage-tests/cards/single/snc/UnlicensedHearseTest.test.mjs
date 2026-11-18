import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/snc/UnlicensedHearseTest.java",
  "tests": [
    {
      "name": "testExileOneCardFromGraveyard",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Unlicensed Hearse",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Runeclaw Bear",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Forest Bear",
          "count": 1
        },
        {
          "op": "activateAbility",
          "source": "Unlicensed Hearse",
          "ability": "{T}: Exile",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": {
            "groups": [
              [
                "Grizzly Bears"
              ]
            ]
          }
        },
        {
          "op": "assertStackObject",
          "source": "Unlicensed Hearse",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated",
          "targetCount": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Forest Bear",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Forest Bear",
          "count": 1
        },
        {
          "op": "assertType",
          "player": 0,
          "name": "Unlicensed Hearse",
          "cardType": "CREATURE",
          "extra": "false"
        },
        {
          "op": "activateAbility",
          "source": "Unlicensed Hearse",
          "ability": "Crew {2}",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Runeclaw Bear"
        },
        {
          "op": "assertStackObject",
          "source": "Unlicensed Hearse",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertType",
          "player": 0,
          "name": "Unlicensed Hearse",
          "cardType": "CREATURE"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Unlicensed Hearse",
          "power": 1,
          "toughness": 1
        }
      ]
    },
    {
      "name": "testExileTwoCardsFromGraveyard",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Unlicensed Hearse",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Runeclaw Bear",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Forest Bear",
          "count": 1
        },
        {
          "op": "activateAbility",
          "source": "Unlicensed Hearse",
          "ability": "{T}: Exile",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": {
            "groups": [
              [
                "Grizzly Bears",
                "Forest Bear"
              ]
            ]
          }
        },
        {
          "op": "assertStackObject",
          "source": "Unlicensed Hearse",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated",
          "targetCount": 2
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Forest Bear",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Forest Bear",
          "count": 0
        },
        {
          "op": "assertType",
          "player": 0,
          "name": "Unlicensed Hearse",
          "cardType": "CREATURE",
          "extra": "false"
        },
        {
          "op": "activateAbility",
          "source": "Unlicensed Hearse",
          "ability": "Crew {2}",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Runeclaw Bear"
        },
        {
          "op": "assertStackObject",
          "source": "Unlicensed Hearse",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertType",
          "player": 0,
          "name": "Unlicensed Hearse",
          "cardType": "CREATURE"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Unlicensed Hearse",
          "power": 2,
          "toughness": 2
        }
      ]
    }
  ]
});
