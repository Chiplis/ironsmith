import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/watchers/AureliasFuryTest.java",
  "tests": [
    {
      "name": "testAureliasFury",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plateau",
          "count": 13
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kraken Hatchling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Glimmerbell",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Fortress Crab",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Aurelia's Fury",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ardent Elementalist",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Aurelia's Fury"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=3"
        },
        {
          "op": "addTargetAmount",
          "player": 0,
          "target": "Kraken Hatchling",
          "amount": 2
        },
        {
          "op": "addTargetAmount",
          "player": 0,
          "target": "Glimmerbell",
          "amount": 1
        },
        {
          "op": "assertDamageReceived",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Kraken Hatchling",
          "damage": 2,
          "seatIndependent": true
        },
        {
          "op": "assertDamageReceived",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 1,
          "name": "Glimmerbell",
          "damage": 1,
          "seatIndependent": true
        },
        {
          "op": "assertTappedCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Kraken Hatchling",
          "tapped": true,
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertTappedCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 1,
          "name": "Glimmerbell",
          "tapped": true,
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertTappedCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 1,
          "name": "Fortress Crab",
          "tapped": false,
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "END_COMBAT",
          "player": 1,
          "ability": "{1}{U}: Untap"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Ardent Elementalist"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Aurelia's Fury"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Aurelia's Fury"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=2"
        },
        {
          "op": "addTargetAmount",
          "player": 0,
          "target": "Fortress Crab",
          "amount": 1
        },
        {
          "op": "addTargetAmount",
          "player": 0,
          "target": 1,
          "amount": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertDamageReceived",
          "player": 0,
          "name": "Kraken Hatchling",
          "damage": 2
        },
        {
          "op": "assertDamageReceived",
          "player": 1,
          "name": "Glimmerbell",
          "damage": 1
        },
        {
          "op": "assertDamageReceived",
          "player": 1,
          "name": "Fortress Crab",
          "damage": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 19
        },
        {
          "op": "assertTapped",
          "name": "Kraken Hatchling",
          "tapped": true
        },
        {
          "op": "assertTapped",
          "name": "Glimmerbell",
          "tapped": false
        },
        {
          "op": "assertTapped",
          "name": "Fortress Crab",
          "tapped": true
        }
      ]
    }
  ]
});
