import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/enters/SkylineCascadeTest.java",
  "tests": [
    {
      "name": "testPreventsTappedCreatureUntapping",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Savannah Lions",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Skyline Cascade",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Savannah Lions",
          "defender": 1
        },
        {
          "op": "playLand",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Skyline Cascade"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Savannah Lions"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Savannah Lions\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Skyline Cascade\", true)"
        }
      ]
    },
    {
      "name": "testDoesNotStopUntappedCreatureUntapping",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Savannah Lions",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Skyline Cascade",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Skyline Cascade"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Savannah Lions"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Savannah Lions\", false)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Skyline Cascade\", true)"
        }
      ]
    }
  ]
});
