import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mh3/TheCreationOfAvacynTest.java",
  "tests": [
    {
      "name": "test_Creature_PutIntoPlay",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The Creation of Avacyn",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Avacyn, Angel of Hope",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "The Creation of Avacyn"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Avacyn, Angel of Hope"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
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
          "op": "assertLife",
          "player": 0,
          "life": 12
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Avacyn, Angel of Hope",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Creature_DontPutIntoPlay",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The Creation of Avacyn",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Avacyn, Angel of Hope",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "The Creation of Avacyn"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Avacyn, Angel of Hope"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
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
          "op": "assertLife",
          "player": 0,
          "life": 12
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Avacyn, Angel of Hope",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Avacyn, Angel of Hope",
          "count": 1
        }
      ]
    },
    {
      "name": "test_NotCreature",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The Creation of Avacyn",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Helvault",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "The Creation of Avacyn"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Helvault"
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Helvault",
          "count": 1
        }
      ]
    },
    {
      "name": "test_StrionicResonator_DoubleCreature",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": false
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Strionic Resonator",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The Creation of Avacyn",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Avacyn, Angel of Hope",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Griselbrand",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "The Creation of Avacyn"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{2}"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Avacyn, Angel of Hope"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Griselbrand"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
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
          "op": "assertLife",
          "player": 0,
          "life": 4
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Avacyn, Angel of Hope",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Griselbrand",
          "count": 1
        }
      ]
    },
    {
      "name": "test_StrionicResonator_CreatureAndPermanent",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": false
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Strionic Resonator",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The Creation of Avacyn",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Avacyn, Angel of Hope",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Helvault",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "The Creation of Avacyn"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{2}"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Avacyn, Angel of Hope"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Helvault"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
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
          "op": "assertLife",
          "player": 0,
          "life": 9
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Avacyn, Angel of Hope",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Helvault",
          "count": 1
        }
      ]
    },
    {
      "name": "test_StrionicResonator_CreatureAndSpell",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": false
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Strionic Resonator",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The Creation of Avacyn",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Avacyn, Angel of Hope",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "The Creation of Avacyn"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{2}"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Avacyn, Angel of Hope"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Lightning Bolt"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
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
          "op": "assertLife",
          "player": 0,
          "life": 11
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Avacyn, Angel of Hope",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        }
      ]
    }
  ]
});
