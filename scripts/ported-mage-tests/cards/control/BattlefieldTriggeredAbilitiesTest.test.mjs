import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/control/BattlefieldTriggeredAbilitiesTest.java",
  "tests": [
    {
      "name": "testBeguilerofWillsAndPrimevalTitan",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Primeval Titan",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Mountain",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Beguiler of Wills",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Arrogant Bloodlord",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Mountain@first",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Mountain@second",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Mountain",
          "count": 8
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{T}: Gain control",
          "target": "Primeval Titan"
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Primeval Titan",
          "defender": 0
        },
        {
          "op": "setOptionChoices",
          "player": 1,
          "kind": "boolean",
          "description": "Search your library for up to two land cards, put them onto the battlefield tapped, then shuffle",
          "choices": [
            "Yes"
          ]
        },
        {
          "op": "setObjectChoices",
          "player": 1,
          "description": "objects",
          "choices": [
            {
              "alias": "@first"
            },
            {
              "alias": "@second"
            }
          ]
        },
        {
          "op": "assertStackObject",
          "turn": 4,
          "phase": "DECLARE_ATTACKERS",
          "source": "Primeval Titan",
          "count": 1,
          "abilityKind": "Triggered",
          "controller": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 4,
          "phase": "DECLARE_ATTACKERS"
        },
        {
          "op": "assertTappedCount",
          "player": 1,
          "name": "Mountain",
          "count": 2,
          "tapped": true
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 14
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Beguiler of Wills",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Arrogant Bloodlord",
          "count": 5
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Primeval Titan",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Mountain",
          "count": 0
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "cardType": "Land"
          },
          "count": 0,
          "checks": []
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 1,
            "cardType": "Land"
          },
          "count": 2,
          "checks": []
        }
      ]
    }
  ]
});
