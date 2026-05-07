import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/damage/ObNixilisCaptiveKingpinTest.java",
  "tests": [
    {
      "name": "damageController1Point",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ob Nixilis, Captive Kingpin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": "playerD",
          "attacker": "Memnite",
          "defender": 0
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ob Nixilis, Captive Kingpin",
          "counter": "P1P1",
          "count": 0
        }
      ]
    },
    {
      "name": "damage1Opp1Point",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ob Nixilis, Captive Kingpin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Memnite",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ob Nixilis, Captive Kingpin",
          "counter": "P1P1",
          "count": 1
        }
      ]
    },
    {
      "name": "damage1Opp2Points",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ob Nixilis, Captive Kingpin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Expedition Envoy",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Expedition Envoy",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ob Nixilis, Captive Kingpin",
          "counter": "P1P1",
          "count": 0
        }
      ]
    },
    {
      "name": "damage2Opp1Point",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ob Nixilis, Captive Kingpin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
          "count": 2
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Memnite",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Memnite",
          "defender": "playerC"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ob Nixilis, Captive Kingpin",
          "counter": "P1P1",
          "count": 1
        }
      ]
    },
    {
      "name": "damage2Opp2Points",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ob Nixilis, Captive Kingpin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Expedition Envoy",
          "count": 2
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Expedition Envoy",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Expedition Envoy",
          "defender": "playerC"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ob Nixilis, Captive Kingpin",
          "counter": "P1P1",
          "count": 0
        }
      ]
    },
    {
      "name": "damage2Opp1Point1Opp2Points",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ob Nixilis, Captive Kingpin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Expedition Envoy",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Expedition Envoy",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Memnite",
          "defender": "playerC"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ob Nixilis, Captive Kingpin",
          "counter": "P1P1",
          "count": 0
        }
      ]
    },
    {
      "name": "damage1Opp1PointCont2Points",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ob Nixilis, Captive Kingpin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Expedition Envoy",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Withstand Death",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Deadly Tempest",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Ob Nixilis, Captive Kingpin"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Withstand Death"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Deadly Tempest"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": "currentGame.getStartingLife() - 2"
        },
        {
          "op": "assertLife",
          "player": "playerD",
          "life": "currentGame.getStartingLife() - 1"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ob Nixilis, Captive Kingpin",
          "counter": "P1P1",
          "count": 1
        }
      ]
    },
    {
      "name": "payLife1Opp1Point",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ob Nixilis, Captive Kingpin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Arid Mesa",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{T}, Pay 1 life"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Mountain"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ob Nixilis, Captive Kingpin",
          "counter": "P1P1",
          "count": 1
        }
      ]
    },
    {
      "name": "payLife1Opp2Point",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ob Nixilis, Captive Kingpin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Book of Rass",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{2}, Pay 2 life"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ob Nixilis, Captive Kingpin",
          "counter": "P1P1",
          "count": 0
        }
      ]
    },
    {
      "name": "loseLife1Opp1Point",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ob Nixilis, Captive Kingpin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Acolyte of Xathrid",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "playerC"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}{B}, {T}"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ob Nixilis, Captive Kingpin",
          "counter": "P1P1",
          "count": 1
        }
      ]
    },
    {
      "name": "loseLife1Opp2Point",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ob Nixilis, Captive Kingpin",
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
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blood Pact",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "playerD"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blood Pact"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ob Nixilis, Captive Kingpin",
          "counter": "P1P1",
          "count": 0
        }
      ]
    },
    {
      "name": "loseLifeAll1Point",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ob Nixilis, Captive Kingpin",
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
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Orzhov Guildmage",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{2}{B}"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ob Nixilis, Captive Kingpin",
          "counter": "P1P1",
          "count": 1
        }
      ]
    },
    {
      "name": "loseLifeAll2Point",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ob Nixilis, Captive Kingpin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Crushing Disappointment",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Crushing Disappointment"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Ob Nixilis, Captive Kingpin",
          "counter": "P1P1",
          "count": 0
        }
      ]
    }
  ]
});
