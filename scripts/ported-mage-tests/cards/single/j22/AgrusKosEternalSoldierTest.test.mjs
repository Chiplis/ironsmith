import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/j22/AgrusKosEternalSoldierTest.java",
  "tests": [
    {
      "name": "testTriggerCopiesAbility",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Agrus Kos, Eternal Soldier",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Horned Turtle",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kor Firewalker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Cunning Sparkmage",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plateau",
          "count": 2
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{T}: {this} deals",
          "target": "Agrus Kos, Eternal Soldier"
        },
        {
          "op": "setChoice",
          "player": 0,
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
          "op": "unsupported",
          "source": "assertTapped(sparkmage, true)"
        },
        {
          "op": "assertTappedCount",
          "name": "Plateau",
          "tapped": true,
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, agrus, 1)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, turtle, 1)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, firewalker, 0)"
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
        }
      ]
    },
    {
      "name": "testTriggerDoesntCopySpell",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Agrus Kos, Eternal Soldier",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Horned Turtle",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kor Firewalker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Shock",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plateau",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Shock",
          "target": "Agrus Kos, Eternal Soldier"
        },
        {
          "op": "setChoice",
          "player": 0,
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
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, agrus, 2)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, turtle, 0)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, firewalker, 0)"
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
        }
      ]
    },
    {
      "name": "testCopiedTriggerAbility",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Agrus Kos, Eternal Soldier",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Horned Turtle",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Kor Firewalker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plateau",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Panharmonicon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Smoldering Werewolf",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Smoldering Werewolf"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "When {this} enters, it deals"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Agrus Kos, Eternal Soldier"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Agrus Kos, Eternal Soldier"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Whenever {this} becomes"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
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
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, agrus, 2)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, turtle, 2)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, firewalker, 0)"
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
        }
      ]
    }
  ]
});
