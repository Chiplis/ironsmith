import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/c19/ChainerNightmareAdeptTest.java",
  "tests": [
    {
      "name": "testChainer",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chainer, Nightmare Adept",
          "count": 1
        },
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
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Feral Maaka",
          "count": 2
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Discard"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Mountain"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Feral Maaka"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Feral Maaka",
          "expected": false
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Feral Maaka",
          "defender": 1
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Feral Maaka",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Feral Maaka",
          "count": 1
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Feral Maaka",
          "ability": "Haste",
          "expected": true
        }
      ]
    },
    {
      "name": "testChainerTwice",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chainer, Nightmare Adept",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Rings of Brighthearth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Feral Maaka",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Defiant Khenra",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Discard"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Mountain"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "0"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "0"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Feral Maaka"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Defiant Khenra"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Feral Maaka",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Defiant Khenra",
          "defender": 1
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Feral Maaka",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(maaka, true)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Feral Maaka",
          "ability": "Haste",
          "expected": true
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Feral Maaka",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Defiant Khenra",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(khenra, true)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Defiant Khenra",
          "ability": "Haste",
          "expected": true
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Defiant Khenra",
          "count": 0
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 16
        }
      ]
    },
    {
      "name": "testChainerOpponent",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chainer, Nightmare Adept",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Reanimate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Feral Maaka",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Reanimate",
          "target": "Feral Maaka"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Feral Maaka",
          "defender": 1
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Feral Maaka",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Feral Maaka",
          "count": 0
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Feral Maaka",
          "ability": "Haste",
          "expected": true
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 18
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        }
      ]
    }
  ]
});
