import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/rules/CantDrawTest.java",
  "tests": [
    {
      "name": "testCardsNotDrawnFromLibraryEffect",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Leovold, Emissary of Trest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sylvan Library",
          "count": 1
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "DRAW",
          "source": "Sylvan Library",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "assertChoiceMenu",
          "player": 0,
          "kind": "select_options",
          "description": "Draw two additional cards",
          "absent": true
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "DRAW"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20,
          "rulesReference": "CR 121.2b prevents choosing the additional draws after the normal draw."
        }
      ]
    },
    {
      "name": "testCardsNotDrawnFromLibraryEffectNoUse",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Leovold, Emissary of Trest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sylvan Library",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        }
      ]
    }
  ]
});
