import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/FightTest.java",
  "tests": [
    {
      "name": "testSimpleFight",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Prey Upon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sunhome Enforcer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mammoth Spider",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Prey Upon"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Sunhome Enforcer"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Mammoth Spider"
        },
        {
          "op": "setStrictChooseMode",
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
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, sunhomeEnforcer, 3)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, mammothSpider, 2)"
        }
      ]
    },
    {
      "name": "testPumpFight",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Savage Swipe",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sunhome Enforcer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Caravan Hurda",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Savage Swipe"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Sunhome Enforcer"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Caravan Hurda"
        },
        {
          "op": "setStrictChooseMode",
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 21
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, sunhomeEnforcer, 1)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, caravanHurda, 4)"
        }
      ]
    },
    {
      "name": "testCounterFight",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Hunt the Weak",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Warrior Angel",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Caravan Hurda",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Hunt the Weak"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Warrior Angel"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Caravan Hurda"
        },
        {
          "op": "setStrictChooseMode",
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
          "op": "assertLife",
          "player": 0,
          "life": 24
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 21
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, warriorAngel, 1)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, caravanHurda, 4)"
        }
      ]
    },
    {
      "name": "testFightTwoTargets",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Rivals' Duel",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mammoth Spider",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Caravan Hurda",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Rivals' Duel",
          "target": "Mammoth Spider^Caravan Hurda"
        },
        {
          "op": "setStrictChooseMode",
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
          "op": "assertLife",
          "player": 0,
          "life": 21
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, mammothSpider, 1)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, caravanHurda, 3)"
        }
      ]
    },
    {
      "name": "testFightEvent",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Foe-Razer Regent",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mammoth Spider",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Foe-Razer Regent"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Mammoth Spider"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
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
          "op": "assertCounterCount",
          "player": 0,
          "name": "Foe-Razer Regent",
          "counter": "P1P1",
          "count": 2
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": "Mammoth Spider",
          "counter": "P1P1",
          "count": 0
        }
      ]
    },
    {
      "name": "testFightSelf",
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
          "name": "Daybreak Ranger",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Dampening Pulse",
          "count": 3
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{R}, {T}: {this} fights target creature."
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Nightfall Predator"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
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
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, nightfallPredator, 2)"
        }
      ]
    }
  ]
});
