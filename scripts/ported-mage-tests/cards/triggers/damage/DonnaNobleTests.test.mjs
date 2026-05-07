import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/damage/DonnaNobleTests.java",
  "tests": [
    {
      "name": "PairedCreature2To1Test",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Donna Noble",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Impervious Greatwurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 10
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Impervious Greatwurm"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
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
          "name": "Expedition Envoy",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "Impervious Greatwurm",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 5,
          "player": 1,
          "blocker": "Memnite",
          "attacker": "Impervious Greatwurm"
        },
        {
          "op": "block",
          "turn": 5,
          "player": 1,
          "blocker": "Expedition Envoy",
          "attacker": "Impervious Greatwurm"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "CHOICE_SKIP"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "setStopAt",
          "turn": 5,
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
          "life": "currentGame.getStartingLife()"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": "currentGame.getStartingLife() - 3"
        }
      ]
    },
    {
      "name": "DonnaAndPairedBothDamagedSingleSourceTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Donna Noble",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Impervious Greatwurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 10
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Impervious Greatwurm"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cinderclasm",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Cinderclasm"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Whenever"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
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
          "op": "assertLife",
          "player": 0,
          "life": "currentGame.getStartingLife()"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": "currentGame.getStartingLife() - 4"
        }
      ]
    },
    {
      "name": "DonnaAndPairedBothDamagedDiffSourceTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Donna Noble",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Impervious Greatwurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 10
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Impervious Greatwurm"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
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
          "name": "Expedition Envoy",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Memnite",
          "defender": 0
        },
        {
          "op": "attack",
          "turn": 4,
          "player": 1,
          "attacker": "Expedition Envoy",
          "defender": 0
        },
        {
          "op": "block",
          "turn": 4,
          "player": 0,
          "blocker": "Impervious Greatwurm",
          "attacker": "Memnite"
        },
        {
          "op": "block",
          "turn": 4,
          "player": 0,
          "blocker": "Donna Noble",
          "attacker": "Expedition Envoy"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Whenever"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "setStopAt",
          "turn": 4,
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
          "life": "currentGame.getStartingLife()"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": "currentGame.getStartingLife() - 3"
        }
      ]
    }
  ]
});
