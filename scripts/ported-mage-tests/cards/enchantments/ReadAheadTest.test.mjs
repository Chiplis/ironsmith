import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/enchantments/ReadAheadTest.java",
  "tests": [
    {
      "name": "testElderDragonWarChapter1",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The Elder Dragon War",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "The Elder Dragon War"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=1"
        },
        {
          "op": "setStopAt",
          "turn": 1,
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
          "player": 1,
          "life": 18
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "The Elder Dragon War",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "The Elder Dragon War",
          "count": 0
        }
      ]
    },
    {
      "name": "testElderDragonWarChapter2",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The Elder Dragon War",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ancestral Recall",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "The Elder Dragon War"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=2"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Ancestral Recall"
        },
        {
          "op": "setStopAt",
          "turn": 1,
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
          "player": 1,
          "life": 20
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Ancestral Recall",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Ancestral Recall",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "The Elder Dragon War",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "The Elder Dragon War",
          "count": 0
        }
      ]
    },
    {
      "name": "testElderDragonWarChapter3",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The Elder Dragon War",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "The Elder Dragon War"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=3"
        },
        {
          "op": "setStopAt",
          "turn": 1,
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
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Dragon Token",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "The Elder Dragon War",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "The Elder Dragon War",
          "count": 1
        }
      ]
    },
    {
      "name": "testBarbaraWrightChapter1",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Barbara Wright",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Rite of Belzenlok",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Rite of Belzenlok"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=1"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Rite of Belzenlok",
          "counter": "LORE",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cleric Token",
          "count": 2
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Rite of Belzenlok",
          "counter": "LORE",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cleric Token",
          "count": 4
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Rite of Belzenlok",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Rite of Belzenlok",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cleric Token",
          "count": 4
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Demon Token",
          "count": 1
        }
      ]
    },
    {
      "name": "testBarbaraWrightChapter2",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Barbara Wright",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Rite of Belzenlok",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Rite of Belzenlok"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=2"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Rite of Belzenlok",
          "counter": "LORE",
          "count": 2
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Rite of Belzenlok",
          "ability": "ReadAhead",
          "expected": true
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cleric Token",
          "count": 2
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Rite of Belzenlok",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Rite of Belzenlok",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cleric Token",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Demon Token",
          "count": 1
        }
      ]
    },
    {
      "name": "testBarbaraWrightChapter3",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Barbara Wright",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Rite of Belzenlok",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Rite of Belzenlok"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=3"
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
          "name": "Rite of Belzenlok",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Rite of Belzenlok",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Cleric Token",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Demon Token",
          "count": 1
        }
      ]
    }
  ]
});
