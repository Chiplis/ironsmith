import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/MasterOfCrueltiesTest.java",
  "tests": [
    {
      "name": "testMasterWasAleshaAnimated",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Alesha, Who Smiles at Death",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Master of Cruelties",
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
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Alesha, Who Smiles at Death",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Silvercoat Lion",
          "attacker": "Master of Cruelties"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Master of Cruelties",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Master of Cruelties\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Alesha, Who Smiles at Death\", true)"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 17
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        }
      ]
    }
  ]
});
