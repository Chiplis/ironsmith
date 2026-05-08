import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/lrw/AquitectsWillTest.java",
  "tests": [
    {
      "name": "testProduceBlueDuringCast",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Aquitect's Will",
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
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Aquitect's Will",
          "target": "Mountain"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Ancestral Recall",
          "target": 0
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
          "op": "assertHandCount",
          "player": 0,
          "count": 3
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Aquitect's Will",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Ancestral Recall",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Mountain",
          "counter": "FLOOD",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(mountain, SubType.MOUNTAIN)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(mountain, SubType.ISLAND)"
        }
      ]
    },
    {
      "name": "testProduceBlueOutsideCast",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Aquitect's Will",
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
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Aquitect's Will",
          "target": "Mountain"
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {U}",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Ancestral Recall",
          "target": 0
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
          "op": "assertHandCount",
          "player": 0,
          "count": 3
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Aquitect's Will",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Ancestral Recall",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Mountain",
          "counter": "FLOOD",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(mountain, SubType.MOUNTAIN)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(mountain, SubType.ISLAND)"
        }
      ]
    },
    {
      "name": "testEffectTiedToCounter",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Vampire Hexmage",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Aquitect's Will",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Aquitect's Will",
          "target": "Mountain"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "Sacrifice",
          "target": "Mountain"
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
          "name": "Aquitect's Will",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Vampire Hexmage",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Mountain",
          "counter": "FLOOD",
          "count": 0
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(mountain, SubType.MOUNTAIN)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(mountain, SubType.ISLAND)"
        }
      ]
    }
  ]
});
