import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/copy/ReflectionOfKikiJikiTest.java",
  "tests": [
    {
      "name": "testTokenNotSacrificedIfNotControlled",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blustersquall",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Fable of the Mirror-Breaker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Humble Defector",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 6,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Blustersquall",
          "target": "Humble Defector"
        },
        {
          "op": "activateAbility",
          "turn": 6,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{1}, {T}: Create a token that's a copy of another target nonlegendary creature you control",
          "target": "Humble Defector"
        },
        {
          "op": "activateAbility",
          "turn": 6,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "ability": "{T}: Draw two cards. Target opponent gains control"
        },
        {
          "op": "setStopAt",
          "turn": 7,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "count": 5
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Blustersquall",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Humble Defector",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Humble Defector",
          "count": 1
        }
      ]
    }
  ]
});
