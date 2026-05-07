import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/TwoHeadedSliverTest.java",
  "tests": [
    {
      "name": "testCantBeBlockedByOneEffectAbility",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Two-Headed Sliver",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Two-Headed Sliver"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Two-Headed Sliver",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Silvercoat Lion",
          "attacker": "Two-Headed Sliver"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "unsupported",
          "source": "try { execute(); fail(\"Expected exception not thrown\"); } catch (UnsupportedOperationException e) { assertEquals(\"Two-Headed Sliver is blocked by 1 creature(s). It has to be blocked by 2 or more.\", e.getMessage()); }"
        }
      ]
    },
    {
      "name": "testCanBeBlockedByTwoEffectAbility",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Two-Headed Sliver",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Coral Barrier",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Two-Headed Sliver"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Two-Headed Sliver",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Silvercoat Lion",
          "attacker": "Two-Headed Sliver"
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Coral Barrier",
          "attacker": "Two-Headed Sliver"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Two-Headed Sliver",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Coral Barrier",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Two-Headed Sliver",
          "count": 1
        }
      ]
    }
  ]
});
