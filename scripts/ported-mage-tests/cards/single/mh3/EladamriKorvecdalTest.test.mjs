import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mh3/EladamriKorvecdalTest.java",
  "tests": [
    {
      "name": "test_Activation_FromHand_NonCreature",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Eladamri, Korvecdal",
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
          "name": "Ornithopter",
          "count": 1
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
          "zone": "LIBRARY",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{G}, {T}"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Memnite^Ornithopter"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Swamp"
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
          "op": "assertTappedCount",
          "name": "Forest",
          "tapped": true,
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Eladamri, Korvecdal",
          "tapped": true,
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Memnite",
          "tapped": true,
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Ornithopter",
          "tapped": true,
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Swamp",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Activation_FromHand_Creature",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Eladamri, Korvecdal",
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
          "name": "Ornithopter",
          "count": 1
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
          "zone": "LIBRARY",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{G}, {T}"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Memnite^Ornithopter"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Grizzly Bears"
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
          "op": "assertTappedCount",
          "name": "Forest",
          "tapped": true,
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Eladamri, Korvecdal",
          "tapped": true,
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Memnite",
          "tapped": true,
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Ornithopter",
          "tapped": true,
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Activation_FromTop_Creature",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Eladamri, Korvecdal",
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
          "name": "Ornithopter",
          "count": 1
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
          "zone": "HAND",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{G}, {T}"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Memnite^Ornithopter"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
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
          "op": "assertTappedCount",
          "name": "Forest",
          "tapped": true,
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Eladamri, Korvecdal",
          "tapped": true,
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Memnite",
          "tapped": true,
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Ornithopter",
          "tapped": true,
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Activation_FromTop_NonCreature",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Eladamri, Korvecdal",
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
          "name": "Ornithopter",
          "count": 1
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
          "zone": "LIBRARY",
          "player": 0,
          "name": "Taiga",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{G}, {T}"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Memnite^Ornithopter"
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
          "op": "assertTappedCount",
          "name": "Forest",
          "tapped": true,
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Eladamri, Korvecdal",
          "tapped": true,
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Memnite",
          "tapped": true,
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Ornithopter",
          "tapped": true,
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Taiga",
          "count": 0
        }
      ]
    }
  ]
});
