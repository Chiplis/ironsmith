import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/MerfolkTricksterTest.java",
  "tests": [
    {
      "name": "test_TricksterAndFlyer_FlyingRemoved",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Flying Men",
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
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Merfolk Trickster",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Flying Men",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "DECLARE_BLOCKERS",
          "player": 1,
          "name": "Merfolk Trickster"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
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
          "life": 19
        },
        {
          "op": "assertTappedCount",
          "name": "Island",
          "tapped": true,
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Flying Men\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertAbilities(playerA, \"Flying Men\", noAbilities)"
        },
        {
          "op": "unsupported",
          "source": "assertAbilities(playerB, mTrickster, flashAbility)"
        }
      ]
    },
    {
      "name": "test_TricksterAndFlyerBlocked_FlyingRemovedAndBlocked",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Flying Men",
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
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Merfolk Trickster",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Flying Men",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 1,
          "name": "Merfolk Trickster"
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Merfolk Trickster",
          "attacker": "Flying Men"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
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
          "op": "assertTappedCount",
          "name": "Island",
          "tapped": true,
          "count": 2
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Flying Men",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Merfolk Trickster",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, mTrickster, 1)"
        }
      ]
    },
    {
      "name": "test_TricksterBlocksFootlightFiend_Survives",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Footlight Fiend",
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
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Merfolk Trickster",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Footlight Fiend",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 1,
          "name": "Merfolk Trickster"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Footlight Fiend"
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Merfolk Trickster",
          "attacker": "Footlight Fiend"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
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
          "op": "assertTappedCount",
          "name": "Island",
          "tapped": true,
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Merfolk Trickster",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, mTrickster, 1)"
        }
      ]
    },
    {
      "name": "test_TricksterBlocksTibaltToken_Survives",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tibalt, Rakish Instigator",
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
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Merfolk Trickster",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "-2:"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Devil Token",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "DECLARE_ATTACKERS",
          "player": 1,
          "name": "Merfolk Trickster"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Devil Token"
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Merfolk Trickster",
          "attacker": "Devil Token"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
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
          "name": "Tibalt, Rakish Instigator",
          "counter": "LOYALTY",
          "count": 3
        },
        {
          "op": "assertTappedCount",
          "name": "Island",
          "tapped": true,
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Merfolk Trickster",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, mTrickster, 1)"
        }
      ]
    }
  ]
});
