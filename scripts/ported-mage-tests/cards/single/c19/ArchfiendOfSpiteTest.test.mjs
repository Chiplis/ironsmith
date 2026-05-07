import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/c19/ArchfiendOfSpiteTest.java",
  "tests": [
    {
      "name": "damageTriggerTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Archfiend of Spite",
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
          "zone": "HAND",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "UNTAP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Archfiend of Spite",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Lightning Bolt",
          "target": "Archfiend of Spite"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Archfiend of Spite",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, \"Archfiend of Spite\", 3)"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        }
      ]
    }
  ]
});
