import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/bng/ThunderousMightTest.java",
  "tests": [
    {
      "name": "testLockedIn",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Runeclaw Bear",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Duergar Assailant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Thunderous Might",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Thunderous Might",
          "target": "Runeclaw Bear"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Runeclaw Bear",
          "defender": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": null
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "name": "Runeclaw Bear",
          "power": 4,
          "toughness": 2
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "DECLARE_BLOCKERS",
          "player": 0,
          "ability": "Sacrifice",
          "target": "Runeclaw Bear"
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Runeclaw Bear",
          "power": 4,
          "toughness": 2
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, bear, 1)"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 16
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 1
        }
      ]
    }
  ]
});
