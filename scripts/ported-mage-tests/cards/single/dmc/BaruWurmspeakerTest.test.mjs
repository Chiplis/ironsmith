import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/dmc/BaruWurmspeakerTest.java",
  "tests": [
    {
      "name": "test_no_wurm",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Baru, Wurmspeaker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 8
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{7}{G},"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(baru, true)"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Wurm Token",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Forest",
          "tapped": true,
          "count": 8
        }
      ]
    },
    {
      "name": "test_wurm",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Baru, Wurmspeaker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Voracious Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{7}{G},"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(baru, true)"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Wurm Token",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Forest",
          "tapped": true,
          "count": 4
        }
      ]
    }
  ]
});
