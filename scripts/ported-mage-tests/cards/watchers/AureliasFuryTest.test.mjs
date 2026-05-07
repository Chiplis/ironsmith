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
          "op": "unsupported",
          "source": "addTargetAmount(playerA, hatchling, 2)"
        },
        {
          "op": "unsupported",
          "source": "addTargetAmount(playerA, glimmerbell, 1)"
        },
        {
          "op": "unsupported",
          "source": "checkDamage(\"first cast\", 1, PhaseStep.BEGIN_COMBAT, playerA, hatchling, 2)"
        },
        {
          "op": "unsupported",
          "source": "checkDamage(\"first cast\", 1, PhaseStep.BEGIN_COMBAT, playerB, glimmerbell, 1)"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentTapped(\"first cast\", 1, PhaseStep.BEGIN_COMBAT, playerA, hatchling, true, 1)"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentTapped(\"first cast\", 1, PhaseStep.BEGIN_COMBAT, playerB, glimmerbell, true, 1)"
        },
        {
          "op": "unsupported",
          "source": "checkPermanentTapped(\"first cast\", 1, PhaseStep.BEGIN_COMBAT, playerB, crab, false, 1)"
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
          "op": "unsupported",
          "source": "addTargetAmount(playerA, crab, 1)"
        },
        {
          "op": "unsupported",
          "source": "addTargetAmount(playerA, playerB, 1)"
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
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, hatchling, 2)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, glimmerbell, 1)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, crab, 1)"
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
          "op": "unsupported",
          "source": "assertTapped(hatchling, true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(glimmerbell, false)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(crab, true)"
        }
      ]
    }
  ]
});
