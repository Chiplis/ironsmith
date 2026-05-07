import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/conditional/ConditionalContinuousEffectTest.java",
  "tests": [
    {
      "name": "testManorGargoyle",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Manor Gargoyle",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}: Until end of turn"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Mountain\", true)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Manor Gargoyle",
          "ability": "Defender",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Manor Gargoyle",
          "ability": "Indestructible",
          "expected": false
        }
      ]
    }
  ]
});
