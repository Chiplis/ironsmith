import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/tsp/PhantomWurmTest.java",
  "tests": [
    {
      "name": "test_DoubleBlocked",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Phantom Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Eager Cadet",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Phantom Wurm",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Memnite",
          "attacker": "Phantom Wurm"
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Eager Cadet",
          "attacker": "Phantom Wurm"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "CHOICE_SKIP"
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
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Eager Cadet",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, wurm, 0)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Phantom Wurm",
          "counter": "P1P1",
          "count": 3
        }
      ]
    },
    {
      "name": "test_BlockedByAnotherPhantom",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Phantom Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Phantom Nomad",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Phantom Wurm",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Phantom Nomad",
          "attacker": "Phantom Wurm"
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
          "source": "assertDamageReceived(playerA, wurm, 0)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Phantom Wurm",
          "counter": "P1P1",
          "count": 3
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, \"Phantom Nomad\", 0)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Phantom Nomad",
          "counter": "P1P1",
          "count": 1
        }
      ]
    },
    {
      "name": "test_BlockedByAnotherPhantom_ThenBolt",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Phantom Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Phantom Nomad",
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
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Phantom Wurm",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Phantom Nomad",
          "attacker": "Phantom Wurm"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "player": 1,
          "name": "Lightning Bolt",
          "target": "Phantom Wurm"
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
          "source": "assertDamageReceived(playerA, wurm, 0)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Phantom Wurm",
          "counter": "P1P1",
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, \"Phantom Nomad\", 0)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Phantom Nomad",
          "counter": "P1P1",
          "count": 1
        }
      ]
    },
    {
      "name": "test_DoubleStrike",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Phantom Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Adorned Pouncer",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Phantom Wurm",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Adorned Pouncer",
          "attacker": "Phantom Wurm"
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
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Adorned Pouncer",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, wurm, 0)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Phantom Wurm",
          "counter": "P1P1",
          "count": 2
        }
      ]
    },
    {
      "name": "test_DoubleBlocked_OneFirstStrikeOneNormal",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Phantom Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Goblin Striker",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Phantom Wurm",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Memnite",
          "attacker": "Phantom Wurm"
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Goblin Striker",
          "attacker": "Phantom Wurm"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "CHOICE_SKIP"
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
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Goblin Striker",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, wurm, 0)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Phantom Wurm",
          "counter": "P1P1",
          "count": 2
        }
      ]
    },
    {
      "name": "test_DoubleBlocked_TwoFirstStrike",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Phantom Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Boros Recruit",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Goblin Striker",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Phantom Wurm",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Boros Recruit",
          "attacker": "Phantom Wurm"
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Goblin Striker",
          "attacker": "Phantom Wurm"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "CHOICE_SKIP"
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
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Boros Recruit",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Goblin Striker",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, wurm, 0)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Phantom Wurm",
          "counter": "P1P1",
          "count": 3
        }
      ]
    },
    {
      "name": "test_Blocked_ThenBolt",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Phantom Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Memnite",
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
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Phantom Wurm",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Memnite",
          "attacker": "Phantom Wurm"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "COMBAT_DAMAGE",
          "player": 1,
          "name": "Lightning Bolt",
          "target": "Phantom Wurm"
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
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, wurm, 0)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Phantom Wurm",
          "counter": "P1P1",
          "count": 2
        }
      ]
    },
    {
      "name": "test_Blocked_FirstStrike_ThenBolt",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Phantom Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Boros Recruit",
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
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Phantom Wurm",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Boros Recruit",
          "attacker": "Phantom Wurm"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "FIRST_COMBAT_DAMAGE",
          "player": 1,
          "name": "Lightning Bolt",
          "target": "Phantom Wurm"
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
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Boros Recruit",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, wurm, 0)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Phantom Wurm",
          "counter": "P1P1",
          "count": 2
        }
      ]
    },
    {
      "name": "test_Simultanous_NonCombat",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Phantom Wurm",
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
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Boros Recruit",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Band Together",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Band Together"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite^Boros Recruit"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Phantom Wurm"
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
          "source": "assertDamageReceived(playerA, wurm, 0)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Phantom Wurm",
          "counter": "P1P1",
          "count": 3
        }
      ]
    },
    {
      "name": "test_Bolt_ThenBolt",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Phantom Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Lightning Bolt",
          "target": "Phantom Wurm"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Lightning Bolt",
          "target": "Phantom Wurm"
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
          "source": "assertDamageReceived(playerA, wurm, 0)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Phantom Wurm",
          "counter": "P1P1",
          "count": 2
        }
      ]
    }
  ]
});
