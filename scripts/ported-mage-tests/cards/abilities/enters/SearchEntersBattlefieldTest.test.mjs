import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/enters/SearchEntersBattlefieldTest.java",
  "tests": [
    {
      "name": "testLandAfterFetchUntapped",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Verdant Catacombs",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Verdant Catacombs"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}, Pay"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Forest"
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
          "name": "Verdant Catacombs",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Forest\", false)"
        }
      ]
    }
  ]
});
