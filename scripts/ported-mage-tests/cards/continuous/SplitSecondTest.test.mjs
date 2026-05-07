import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/SplitSecondTest.java",
  "tests": [
    {
      "name": "testCounterSpell",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sudden Shock",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Counterspell",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Sudden Shock",
          "target": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Counterspell",
          "target": "Sudden Shock"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "unsupported",
          "source": "try { execute(); Assert.fail(\"must throw exception on execute\"); } catch (Throwable e) { if (!e.getMessage().contains(\"Cast Counterspell$target=Sudden Shock\")) { Assert.fail(\"Should have thrown error about trying to use Counterspell, but got:\\n\" + e.getMessage()); } }"
        }
      ]
    },
    {
      "name": "testCopiedSpell",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Doublecast",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sudden Shock",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Raging Goblin",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Doublecast"
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
          "name": "Sudden Shock",
          "target": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Raging Goblin"
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
          "op": "assertLife",
          "player": 1,
          "life": 16
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Raging Goblin",
          "count": 1
        }
      ]
    },
    {
      "name": "testMoltenDisasterUnkicked",
      "operations": [
        {
          "op": "unsupported",
          "source": "setupMoltenDisaster()"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Molten Disaster"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=1"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Shock",
          "target": "Fortress Crab"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Sacrifice"
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
          "op": "assertLife",
          "player": 0,
          "life": 22
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 19
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, crab, 1 + 2)"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Bottle Gnomes",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, bear, 1)"
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, drake, 0)"
        }
      ]
    },
    {
      "name": "testMoltenDisasterKickedNoSpell",
      "operations": [
        {
          "op": "unsupported",
          "source": "setupMoltenDisaster()"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Molten Disaster"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=1"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Shock",
          "target": "Fortress Crab"
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
          "op": "unsupported",
          "source": "try { execute(); throw new AssertionError(\"expected failure to cast Shock\"); } catch (AssertionError e) { Assert.assertTrue(e.getMessage().contains(\"Can't find ability to activate command: Cast Shock$target=Fortress Crab\")); }"
        }
      ]
    },
    {
      "name": "testMoltenDisasterKickedNoAbility",
      "operations": [
        {
          "op": "unsupported",
          "source": "setupMoltenDisaster()"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Molten Disaster"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=1"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Sacrifice"
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
          "op": "unsupported",
          "source": "try { execute(); throw new AssertionError(\"expected failure to activate sacrifice ability\"); } catch (AssertionError e) { Assert.assertTrue(e.getMessage().contains(\"Can't find ability to activate command: Sacrifice\")); }"
        }
      ]
    }
  ]
});
