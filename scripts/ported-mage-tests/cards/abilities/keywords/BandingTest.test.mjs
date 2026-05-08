import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/BandingTest.java",
  "tests": [
    {
      "name": "BandingAttackSimple",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Squire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Benalish Infantry",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Eager Cadet",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Naga Eternal",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Squire",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Benalish Infantry",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Eager Cadet",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Squire"
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Naga Eternal",
          "attacker": "Squire"
        },
        {
          "op": "unsupported",
          "source": "setChoiceAmount(playerA, 1, 2)"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, \"Squire\", 1)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, \"Benalish Infantry\", 2)"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 19
        }
      ]
    },
    {
      "name": "BandingBlockSimple",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Alpine Grizzly",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Squire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Sanctuary Cat",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Benalish Infantry",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Alpine Grizzly",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Squire",
          "attacker": "Alpine Grizzly"
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Sanctuary Cat",
          "attacker": "Alpine Grizzly"
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Benalish Infantry",
          "attacker": "Alpine Grizzly"
        },
        {
          "op": "unsupported",
          "source": "setChoiceAmount(playerB, 1, 1, 2)"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "count": 0
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, \"Squire\", 1)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, \"Sanctuary Cat\", 1)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, \"Benalish Infantry\", 2)"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        }
      ]
    },
    {
      "name": "DoubleBanding",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Benalish Infantry",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Fortress Crab",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Eager Cadet",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Catacomb Slug",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "War Elephant",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Benalish Infantry",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Fortress Crab",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Eager Cadet",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Fortress Crab"
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Catacomb Slug",
          "attacker": "Benalish Infantry"
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "War Elephant",
          "attacker": "Benalish Infantry"
        },
        {
          "op": "unsupported",
          "source": "setChoiceAmount(playerB, 0, 1)"
        },
        {
          "op": "unsupported",
          "source": "setChoiceAmount(playerB, 0, 1)"
        },
        {
          "op": "unsupported",
          "source": "setChoiceAmount(playerA, 1, 1)"
        },
        {
          "op": "unsupported",
          "source": "setChoiceAmount(playerA, 0, 2)"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 0
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, \"Benalish Infantry\", 1)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, \"Fortress Crab\", 3)"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "War Elephant",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, \"Catacomb Slug\", 0)"
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
