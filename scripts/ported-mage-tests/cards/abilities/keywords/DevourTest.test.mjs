import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/DevourTest.java",
  "tests": [
    {
      "name": "Wurm_NoDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gorger Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gorger Wurm"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        }
      ]
    },
    {
      "name": "Wurm_OneDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gorger Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gorger Wurm"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 24
        }
      ]
    },
    {
      "name": "Wurm_TwoDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gorger Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gorger Wurm"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 24
        }
      ]
    },
    {
      "name": "Wurm_ThreeDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gorger Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gorger Wurm"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 24
        }
      ]
    },
    {
      "name": "Wurm_IllegalDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gorger Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gorger Wurm"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Darksteel Relic"
        },
        {
          "op": "unsupported",
          "source": "try { setStopAt(1, PhaseStep.BEGIN_COMBAT); execute(); } catch (AssertionError e) { if (e.getMessage().startsWith(\"PlayerA - Targets list was setup by addTarget with [\" + \"Darksteel Relic\" + \"], but not used\")) { legal = false; } } finally { assert !legal; }"
        }
      ]
    },
    {
      "name": "Thromok_NoDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Thromok the Insatiable",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Thromok the Insatiable"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        }
      ]
    },
    {
      "name": "Thromok_OneDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Thromok the Insatiable",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Thromok the Insatiable"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 24
        }
      ]
    },
    {
      "name": "Thromok_TwoDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Thromok the Insatiable",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Thromok the Insatiable"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 24
        }
      ]
    },
    {
      "name": "Thromok_ThreeDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Thromok the Insatiable",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Thromok the Insatiable"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 24
        }
      ]
    },
    {
      "name": "Thromok_IllegalDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Thromok the Insatiable",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Thromok the Insatiable"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Darksteel Relic"
        },
        {
          "op": "unsupported",
          "source": "try { setStopAt(1, PhaseStep.BEGIN_COMBAT); execute(); } catch (AssertionError e) { if (e.getMessage().startsWith(\"PlayerA - Targets list was setup by addTarget with [\" + \"Darksteel Relic\" + \"], but not used\")) { legal = false; } } finally { assert !legal; }"
        }
      ]
    },
    {
      "name": "Hobbit_NoDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Feasting Hobbit",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Feasting Hobbit"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        }
      ]
    },
    {
      "name": "Hobbit_OneDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Feasting Hobbit",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Feasting Hobbit"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        }
      ]
    },
    {
      "name": "Hobbit_IllegalDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Feasting Hobbit",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Feasting Hobbit"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Enatu Golem"
        },
        {
          "op": "unsupported",
          "source": "try { setStopAt(1, PhaseStep.BEGIN_COMBAT); execute(); } catch (AssertionError e) { if (e.getMessage().startsWith(\"PlayerA - Targets list was setup by addTarget with [\" + \"Enatu Golem\" + \"], but not used\")) { legal = false; } } finally { assert !legal; }"
        }
      ]
    },
    {
      "name": "Caprichrome_NoDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Caprichrome",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Caprichrome"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        }
      ]
    },
    {
      "name": "Caprichrome_OneDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Caprichrome",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Caprichrome"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 24
        }
      ]
    },
    {
      "name": "Caprichrome_TwoDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Caprichrome",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Caprichrome"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 24
        }
      ]
    },
    {
      "name": "Caprichrome_ThreeDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Caprichrome",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Caprichrome"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 24
        }
      ]
    },
    {
      "name": "Caprichrome_IllegalDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Caprichrome",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Caprichrome"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Silvercoat Lion"
        },
        {
          "op": "unsupported",
          "source": "try { setStopAt(1, PhaseStep.BEGIN_COMBAT); execute(); } catch (AssertionError e) { if (e.getMessage().startsWith(\"PlayerA - Targets list was setup by addTarget with [\" + \"Silvercoat Lion\" + \"], but not used\")) { legal = false; } } finally { assert !legal; }"
        }
      ]
    },
    {
      "name": "Hatchling_NoDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Hellkite Hatchling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Hellkite Hatchling"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Hellkite Hatchling",
          "ability": "Flying",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Hellkite Hatchling",
          "ability": "Trample",
          "expected": false
        }
      ]
    },
    {
      "name": "Hatchling_OneDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Hellkite Hatchling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Hellkite Hatchling"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 24
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Hellkite Hatchling",
          "ability": "Flying",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Hellkite Hatchling",
          "ability": "Trample",
          "expected": true
        }
      ]
    },
    {
      "name": "Hatchling_TwoDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Hellkite Hatchling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Hellkite Hatchling"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 24
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Hellkite Hatchling",
          "ability": "Flying",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Hellkite Hatchling",
          "ability": "Trample",
          "expected": true
        }
      ]
    },
    {
      "name": "Hatchling_ThreeDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Hellkite Hatchling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Hellkite Hatchling"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 24
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Hellkite Hatchling",
          "ability": "Flying",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Hellkite Hatchling",
          "ability": "Trample",
          "expected": true
        }
      ]
    },
    {
      "name": "Hatchling_IllegalDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Hellkite Hatchling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Hellkite Hatchling"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Darksteel Relic"
        },
        {
          "op": "unsupported",
          "source": "try { setStopAt(1, PhaseStep.BEGIN_COMBAT); execute(); } catch (AssertionError e) { if (e.getMessage().startsWith(\"PlayerA - Targets list was setup by addTarget with [\" + \"Darksteel Relic\" + \"], but not used\")) { legal = false; } } finally { assert !legal; }"
        }
      ]
    },
    {
      "name": "Chomper_NoDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Marrow Chomper",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Marrow Chomper"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        }
      ]
    },
    {
      "name": "Chomper_OneDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Marrow Chomper",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Marrow Chomper"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 26
        }
      ]
    },
    {
      "name": "Chomper_TwoDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Marrow Chomper",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Marrow Chomper"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 28
        }
      ]
    },
    {
      "name": "Chomper_ThreeDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Marrow Chomper",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Marrow Chomper"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 30
        }
      ]
    },
    {
      "name": "Chomper_IllegalDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Marrow Chomper",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Marrow Chomper"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Darksteel Relic"
        },
        {
          "op": "unsupported",
          "source": "try { setStopAt(1, PhaseStep.BEGIN_COMBAT); execute(); } catch (AssertionError e) { if (e.getMessage().startsWith(\"PlayerA - Targets list was setup by addTarget with [\" + \"Darksteel Relic\" + \"], but not used\")) { legal = false; } } finally { assert !legal; }"
        }
      ]
    },
    {
      "name": "hellionNoDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Devouring Hellion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Devouring Hellion"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        }
      ]
    },
    {
      "name": "hellionOneDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Devouring Hellion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Devouring Hellion"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        }
      ]
    },
    {
      "name": "hellionTwoDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Devouring Hellion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Devouring Hellion"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 24
        }
      ]
    },
    {
      "name": "hellionThreeDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Devouring Hellion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Devouring Hellion"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Enatu Golem",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gingerbrute",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": "true ? 0 : 1"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "false ? 1 : 0"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": "false ? 0 : 1"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 24
        }
      ]
    },
    {
      "name": "hellionIllegalDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Devouring Hellion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Devouring Hellion"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Darksteel Relic"
        },
        {
          "op": "unsupported",
          "source": "try { setStopAt(1, PhaseStep.BEGIN_COMBAT); execute(); } catch (AssertionError e) { if (e.getMessage().startsWith(\"PlayerA - Targets list was setup by addTarget with [\" + \"Darksteel Relic\" + \"], but not used\")) { legal = false; } } finally { assert !legal; }"
        }
      ]
    }
  ]
});
