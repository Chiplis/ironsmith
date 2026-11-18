import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/copy/DeceiverOfFormTest.java",
  "tests": [
    {
      "name": "testCopyEndsEndOfTurn",
      "operations": [
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Affa Protector",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Deceiver of Form",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 3
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "source": "Deceiver of Form",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "boolean",
          "choices": [
            "Yes"
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "boolean",
          "choices": [
            "No"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "name": "Affa Protector",
          "count": 3,
          "player": 0,
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Deceiver of Form",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Affa Protector",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Affa Protector",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Affa Protector",
          "defender": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Deceiver of Form",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 3
        },
        {
          "op": "assertPermanentCount",
          "name": "Affa Protector",
          "count": 0,
          "player": 0
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Silvercoat Lion",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 9
        }
      ]
    }
  ]
});
