import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/all/SoldeviSentryTest.java",
  "tests": [
    {
      "name": "test_ReflexiveOnRegenerate",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Soldevi Sentry",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Doom Blade",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "ability": "{1}: Choose",
          "target": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Doom Blade",
          "target": "Soldevi Sentry"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Soldevi Sentry",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(sentry, true)"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1,
          "name": 1
        }
      ]
    }
  ]
});
