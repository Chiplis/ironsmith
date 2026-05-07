import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/planeswalker/VivienTest.java",
  "tests": [
    {
      "name": "test_Distribute_NoTargets",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Vivien, Arkbow Ranger",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Vivien, Arkbow Ranger"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1: Distribute"
        },
        {
          "op": "unsupported",
          "source": "addTargetAmount(playerA, TestPlayer.TARGET_SKIP)"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Vivien, Arkbow Ranger",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Vivien, Arkbow Ranger",
          "counter": "LOYALTY",
          "count": 5
        }
      ]
    },
    {
      "name": "test_Distribute_OneTarget",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Vivien, Arkbow Ranger",
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
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Vivien, Arkbow Ranger"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1: Distribute"
        },
        {
          "op": "unsupported",
          "source": "addTargetAmount(playerA, \"Silvercoat Lion\", 2)"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Vivien, Arkbow Ranger",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Vivien, Arkbow Ranger",
          "counter": "LOYALTY",
          "count": 5
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Silvercoat Lion",
          "power": 4,
          "toughness": 4
        }
      ]
    },
    {
      "name": "testVivienArkbowRangerAbilityOnePossibleTargetWithTwo",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Vivien, Arkbow Ranger",
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
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Vivien, Arkbow Ranger"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1: Distribute"
        },
        {
          "op": "unsupported",
          "source": "addTargetAmount(playerA, \"Silvercoat Lion\", 2)"
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
          "name": "Vivien, Arkbow Ranger",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Vivien, Arkbow Ranger",
          "counter": "LOYALTY",
          "count": 5
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Silvercoat Lion",
          "power": 4,
          "toughness": 4
        }
      ]
    },
    {
      "name": "testVivienArkbowRangerAbility1OneOwnPossibleTarget",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Vivien, Arkbow Ranger",
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
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Vivien, Arkbow Ranger"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1: Distribute"
        },
        {
          "op": "unsupported",
          "source": "addTargetAmount(playerA, \"Silvercoat Lion\", 2)"
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
          "name": "Vivien, Arkbow Ranger",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Vivien, Arkbow Ranger",
          "counter": "LOYALTY",
          "count": 5
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Silvercoat Lion",
          "power": 4,
          "toughness": 4
        }
      ]
    },
    {
      "name": "testVivienArkbowRangerAbility1TwoOwnPossibleTarget",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Vivien, Arkbow Ranger",
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
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Pillarfield Ox",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Vivien, Arkbow Ranger"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1: Distribute"
        },
        {
          "op": "unsupported",
          "source": "addTargetAmount(playerA, \"Silvercoat Lion\", 1)"
        },
        {
          "op": "unsupported",
          "source": "addTargetAmount(playerA, \"Pillarfield Ox\", 1)"
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
          "name": "Vivien, Arkbow Ranger",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Vivien, Arkbow Ranger",
          "counter": "LOYALTY",
          "count": 5
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Silvercoat Lion",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Pillarfield Ox",
          "power": 3,
          "toughness": 5
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Silvercoat Lion",
          "ability": "Trample",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Pillarfield Ox",
          "ability": "Trample",
          "expected": true
        }
      ]
    }
  ]
});
