import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/AlmsBeastTest.java",
  "tests": [
    {
      "name": "testLifelink",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Alms Beast",
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
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Silvercoat Lion",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Alms Beast",
          "attacker": "Silvercoat Lion"
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
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 22
        }
      ]
    },
    {
      "name": "testNoLifelinkAfterCombat",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Rootwater Hunter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Shielded Passage",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Alms Beast",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Alms Beast",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Rootwater Hunter",
          "attacker": "Alms Beast"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "DECLARE_BLOCKERS",
          "player": 0,
          "name": "Shielded Passage",
          "target": "Rootwater Hunter"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: ",
          "target": 1
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Shielded Passage",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Rootwater Hunter",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 21
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 19
        }
      ]
    }
  ]
});
