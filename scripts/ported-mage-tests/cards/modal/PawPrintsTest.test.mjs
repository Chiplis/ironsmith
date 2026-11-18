import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/modal/PawPrintsTest.java",
  "tests": [
    {
      "name": "test_Choose113",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Season of Gathering",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Season of Gathering"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "modes",
          "description": "Choose mode for Season of Gathering",
          "choices": [
            "Put a +1/+1 counter on a creature you control. It gains vigilance and trample until end of turn.",
            "Put a +1/+1 counter on a creature you control. It gains vigilance and trample until end of turn.",
            "Draw cards equal to the greatest power among creatures you control."
          ]
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Season of Gathering",
          "count": 1
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose exactly 1 creature",
          "choices": [
            "Memnite"
          ]
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose exactly 1 creature",
          "choices": [
            "Memnite"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
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
          "op": "assertHandCount",
          "player": 0,
          "count": 3
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Memnite",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Memnite",
          "counter": "P1P1",
          "count": 2
        }
      ]
    },
    {
      "name": "test_Choose123",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Season of Gathering",
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
          "name": "Hardened Scales",
          "count": 1
        },
        {
          "op": "assertRejectedProposal",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Season of Gathering",
          "modes": [
            1,
            2,
            3
          ],
          "silentRejection": true
        }
      ]
    },
    {
      "name": "test_Choose2111",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Season of Gathering",
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
          "name": "Hardened Scales",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Season of Gathering"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "modes",
          "description": "Choose mode for Season of Gathering",
          "choices": [
            "Choose artifact or enchantment. Destroy all permanents of the chosen type.",
            "Put a +1/+1 counter on a creature you control. It gains vigilance and trample until end of turn.",
            "Put a +1/+1 counter on a creature you control. It gains vigilance and trample until end of turn.",
            "Put a +1/+1 counter on a creature you control. It gains vigilance and trample until end of turn."
          ]
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Season of Gathering",
          "count": 1
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose exactly 1 creature",
          "choices": [
            "Memnite"
          ]
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose exactly 1 creature",
          "choices": [
            "Memnite"
          ]
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose exactly 1 creature",
          "choices": [
            "Memnite"
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose one",
          "choices": [
            "enchantment"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Memnite",
          "power": 7,
          "toughness": 7
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Memnite",
          "counter": "P1P1",
          "count": 6
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Hardened Scales",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Hardened Scales",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Choose1x5",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Season of Gathering",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Season of Gathering"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "modes",
          "description": "Choose mode for Season of Gathering",
          "choices": [
            "Put a +1/+1 counter on a creature you control. It gains vigilance and trample until end of turn.",
            "Put a +1/+1 counter on a creature you control. It gains vigilance and trample until end of turn.",
            "Put a +1/+1 counter on a creature you control. It gains vigilance and trample until end of turn.",
            "Put a +1/+1 counter on a creature you control. It gains vigilance and trample until end of turn.",
            "Put a +1/+1 counter on a creature you control. It gains vigilance and trample until end of turn."
          ]
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Season of Gathering",
          "count": 1
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose exactly 1 creature",
          "choices": [
            "Memnite"
          ]
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose exactly 1 creature",
          "choices": [
            "Memnite"
          ]
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose exactly 1 creature",
          "choices": [
            "Memnite"
          ]
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose exactly 1 creature",
          "choices": [
            "Memnite"
          ]
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose exactly 1 creature",
          "choices": [
            "Memnite"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Memnite",
          "power": 6,
          "toughness": 6
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Memnite",
          "counter": "P1P1",
          "count": 5
        }
      ]
    },
    {
      "name": "test_Choose23",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Season of Gathering",
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
          "name": "Hardened Scales",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Season of Gathering"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "modes",
          "description": "Choose mode for Season of Gathering",
          "choices": [
            "Draw cards equal to the greatest power among creatures you control.",
            "Choose artifact or enchantment. Destroy all permanents of the chosen type."
          ]
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Season of Gathering",
          "count": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose one",
          "choices": [
            "artifact"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
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
          "name": "Hardened Scales",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0
        }
      ]
    },
    {
      "name": "test_Choose122",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Season of Gathering",
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
          "name": "Hardened Scales",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Season of Gathering"
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "modes",
          "description": "Choose mode for Season of Gathering",
          "choices": [
            "Put a +1/+1 counter on a creature you control. It gains vigilance and trample until end of turn.",
            "Choose artifact or enchantment. Destroy all permanents of the chosen type.",
            "Choose artifact or enchantment. Destroy all permanents of the chosen type."
          ]
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Season of Gathering",
          "count": 1
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose exactly 1 creature",
          "choices": [
            "Memnite"
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose one",
          "choices": [
            "artifact"
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "options",
          "description": "Choose one",
          "choices": [
            "enchantment"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Hardened Scales",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Memnite",
          "count": 1
        }
      ]
    }
  ]
});
