import { registerPortedMageTests } from "../../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/oneshot/exile/RoonOfTheHiddenRealmTest.java",
  "tests": [
    {
      "name": "testReturnToBattlefieldForOwner",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Roon of the Hidden Realm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Perplexing Chimera@chimera",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Silvercoat Lion@lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "castSpell",
          "name": "Silvercoat Lion",
          "player": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "objectAlias": "@lion"
        },
        {
          "op": "assertStackObject",
          "source": "Perplexing Chimera",
          "count": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Triggered"
        },
        {
          "op": "setOptionChoices",
          "kind": "boolean",
          "player": 0,
          "description": "Exchange control of this creature and it",
          "choices": [
            "Yes"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "source": "Silvercoat Lion",
          "count": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "controller": 0
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@lion"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 1,
            "alias": "@chimera"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "activateAbility",
          "source": "Roon of the Hidden Realm",
          "ability": "Exile another target creature",
          "player": 0,
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "target": "@chimera"
        },
        {
          "op": "assertStackObject",
          "source": "Roon of the Hidden Realm",
          "count": 1,
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "abilityKind": "Activated",
          "objectTarget": "Perplexing Chimera"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "@chimera",
          "count": 1
        },
        {
          "op": "assertStackObject",
          "source": "Roon of the Hidden Realm",
          "count": 1,
          "turn": 2,
          "phase": "END_TURN",
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@chimera"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@lion"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertTapped",
          "player": 0,
          "name": "Roon of the Hidden Realm",
          "tapped": true
        }
      ]
    }
  ]
});
