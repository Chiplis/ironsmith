import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mh3/FangsOfKaloniaTest.java",
  "tests": [
    {
      "name": "test_Basic",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Fangs of Kalonia",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Fangs of Kalonia",
          "target": "Grizzly Bears"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "power": 0,
          "toughness": "Grizzly Bears"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Fangs of Kalonia",
          "target": "Grizzly Bears"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Grizzly Bears",
          "power": 8,
          "toughness": 8
        }
      ]
    },
    {
      "name": "test_Overload",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Fangs of Kalonia",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Arcbound Lancer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 6
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Fangs of Kalonia with overload"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Grizzly Bears",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Arcbound Lancer",
          "power": 10,
          "toughness": 10
        }
      ]
    },
    {
      "name": "test_Vorinclex",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Fangs of Kalonia",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Arcbound Lancer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 11
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Vorinclex, Monstrous Raider",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Arcbound Lancer"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "power": 0,
          "toughness": "Arcbound Lancer"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Fangs of Kalonia",
          "target": "Grizzly Bears"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Fangs of Kalonia",
          "target": "Arcbound Lancer"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Grizzly Bears",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Arcbound Lancer",
          "power": 2,
          "toughness": 2
        }
      ]
    },
    {
      "name": "test_HardenedScales",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Fangs of Kalonia",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Arcbound Lancer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 11
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
          "name": "Arcbound Lancer"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "power": 0,
          "toughness": "Arcbound Lancer"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Fangs of Kalonia",
          "target": "Grizzly Bears"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Fangs of Kalonia",
          "target": "Arcbound Lancer"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Grizzly Bears",
          "power": 7,
          "toughness": 7
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Arcbound Lancer",
          "power": 15,
          "toughness": 15
        }
      ]
    },
    {
      "name": "test_DoubleReplacement1",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Fangs of Kalonia",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Arcbound Lancer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 9
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hardened Scales",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Vorinclex, Monstrous Raider",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Arcbound Lancer"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Hardened Scales"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "power": 0,
          "toughness": "Arcbound Lancer"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Fangs of Kalonia",
          "target": "Arcbound Lancer"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Hardened Scales"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Arcbound Lancer",
          "power": 5,
          "toughness": 5
        }
      ]
    },
    {
      "name": "test_DoubleReplacement2",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Fangs of Kalonia",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Arcbound Lancer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 9
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hardened Scales",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Vorinclex, Monstrous Raider",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Arcbound Lancer"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Vorinclex, Monstrous Raider"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "power": 0,
          "toughness": "Arcbound Lancer"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Fangs of Kalonia",
          "target": "Arcbound Lancer"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Vorinclex, Monstrous Raider"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Arcbound Lancer",
          "power": 3,
          "toughness": 3
        }
      ]
    },
    {
      "name": "test_VorinclexOverload",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Fangs of Kalonia",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Arcbound Lancer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 13
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Vorinclex, Monstrous Raider",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Arcbound Lancer"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "power": 0,
          "toughness": "Arcbound Lancer"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Fangs of Kalonia with overload"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Grizzly Bears",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Arcbound Lancer",
          "power": 2,
          "toughness": 2
        }
      ]
    },
    {
      "name": "test_HardenedScalesOverload",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Fangs of Kalonia",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Arcbound Lancer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 13
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
          "name": "Arcbound Lancer"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "power": 0,
          "toughness": "Arcbound Lancer"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Fangs of Kalonia with overload"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Grizzly Bears",
          "power": 7,
          "toughness": 7
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Arcbound Lancer",
          "power": 15,
          "toughness": 15
        }
      ]
    }
  ]
});
