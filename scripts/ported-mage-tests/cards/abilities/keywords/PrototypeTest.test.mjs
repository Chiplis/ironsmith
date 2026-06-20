import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/PrototypeTest.java",
  "tests": [
    {
      "name": "testNormal",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "for (Permanent permanent : currentGame.getBattlefield().getActivePermanents( StaticFilters.FILTER_PERMANENT, playerA.getId(), currentGame )) { if (!permanent.getName().equals(automaton)) { continue; } Assert.assertTrue(\"Needs haste\", permanent.getAbilities(currentGame).contains(HasteAbility.getInstance())); Assert.assertEquals(\"Power is wrong\", false ? 3 : 6, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness is wrong\", false ? 2 : 4, permanent.getToughness().getValue()); Assert.assertTrue(\"Color is wrong\", false ? permanent.getColor(currentGame).isRed() : permanent.getColor(currentGame).isColorless() ); Assert.assertEquals(\"Mana cost is wrong\", false ? \"{2}{R}\" : \"{7}\", permanent.getManaCost().getText()); Assert.assertEquals(\"Mana value is wrong\", false ? 3 : 7, permanent.getManaValue()); }"
        }
      ]
    },
    {
      "name": "testPrototype",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton using Prototype"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "for (Permanent permanent : currentGame.getBattlefield().getActivePermanents( StaticFilters.FILTER_PERMANENT, playerA.getId(), currentGame )) { if (!permanent.getName().equals(automaton)) { continue; } Assert.assertTrue(\"Needs haste\", permanent.getAbilities(currentGame).contains(HasteAbility.getInstance())); Assert.assertEquals(\"Power is wrong\", true ? 3 : 6, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness is wrong\", true ? 2 : 4, permanent.getToughness().getValue()); Assert.assertTrue(\"Color is wrong\", true ? permanent.getColor(currentGame).isRed() : permanent.getColor(currentGame).isColorless() ); Assert.assertEquals(\"Mana cost is wrong\", true ? \"{2}{R}\" : \"{7}\", permanent.getManaCost().getText()); Assert.assertEquals(\"Mana value is wrong\", true ? 3 : 7, permanent.getManaValue()); }"
        }
      ]
    },
    {
      "name": "testLeavesBattlefield",
      "operations": [
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
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton using Prototype"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": "Blitz Automaton"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        }
      ]
    },
    {
      "name": "testBlink",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plateau",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cloudshift",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton using Prototype"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Cloudshift",
          "target": "Blitz Automaton"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "for (Permanent permanent : currentGame.getBattlefield().getActivePermanents( StaticFilters.FILTER_PERMANENT, playerA.getId(), currentGame )) { if (!permanent.getName().equals(automaton)) { continue; } Assert.assertTrue(\"Needs haste\", permanent.getAbilities(currentGame).contains(HasteAbility.getInstance())); Assert.assertEquals(\"Power is wrong\", false ? 3 : 6, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness is wrong\", false ? 2 : 4, permanent.getToughness().getValue()); Assert.assertTrue(\"Color is wrong\", false ? permanent.getColor(currentGame).isRed() : permanent.getColor(currentGame).isColorless() ); Assert.assertEquals(\"Mana cost is wrong\", false ? \"{2}{R}\" : \"{7}\", permanent.getManaCost().getText()); Assert.assertEquals(\"Mana value is wrong\", false ? 3 : 7, permanent.getManaValue()); }"
        }
      ]
    },
    {
      "name": "testTriggerColorlessSpell",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility( \"tester\", playerB, new EntersBattlefieldAllTriggeredAbility( new GainLifeEffect(1), filterB, false ) )"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
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
          "life": 21
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 21
        }
      ]
    },
    {
      "name": "testTriggerRedSpell",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility( \"tester\", playerB, new EntersBattlefieldAllTriggeredAbility( new GainLifeEffect(1), filterB, false ) )"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton using Prototype"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
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
          "life": 21
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 21
        }
      ]
    },
    {
      "name": "testTrigger64Spell",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility( \"tester\", playerB, new EntersBattlefieldAllTriggeredAbility( new GainLifeEffect(1), filterB, false ) )"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
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
          "life": 21
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 21
        }
      ]
    },
    {
      "name": "testTrigger32Spell",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility( \"tester\", playerB, new EntersBattlefieldAllTriggeredAbility( new GainLifeEffect(1), filterB, false ) )"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton using Prototype"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
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
          "life": 21
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 21
        }
      ]
    },
    {
      "name": "testTrigger7MVSpell",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility( \"tester\", playerB, new EntersBattlefieldAllTriggeredAbility( new GainLifeEffect(1), filterB, false ) )"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
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
          "life": 21
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 21
        }
      ]
    },
    {
      "name": "testTrigger3MVSpell",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCustomCardWithAbility( \"tester\", playerB, new EntersBattlefieldAllTriggeredAbility( new GainLifeEffect(1), filterB, false ) )"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton using Prototype"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
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
          "life": 21
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 21
        }
      ]
    },
    {
      "name": "testCloneRegular",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 11
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Clone",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Clone"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Blitz Automaton"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "for (Permanent permanent : currentGame.getBattlefield().getActivePermanents( StaticFilters.FILTER_PERMANENT, playerA.getId(), currentGame )) { if (!permanent.getName().equals(automaton)) { continue; } Assert.assertTrue(\"Needs haste\", permanent.getAbilities(currentGame).contains(HasteAbility.getInstance())); Assert.assertEquals(\"Power is wrong\", false ? 3 : 6, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness is wrong\", false ? 2 : 4, permanent.getToughness().getValue()); Assert.assertTrue(\"Color is wrong\", false ? permanent.getColor(currentGame).isRed() : permanent.getColor(currentGame).isColorless() ); Assert.assertEquals(\"Mana cost is wrong\", false ? \"{2}{R}\" : \"{7}\", permanent.getManaCost().getText()); Assert.assertEquals(\"Mana value is wrong\", false ? 3 : 7, permanent.getManaValue()); }"
        }
      ]
    },
    {
      "name": "testClonePrototype",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Clone",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton using Prototype"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Clone"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Blitz Automaton"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "for (Permanent permanent : currentGame.getBattlefield().getActivePermanents( StaticFilters.FILTER_PERMANENT, playerA.getId(), currentGame )) { if (!permanent.getName().equals(automaton)) { continue; } Assert.assertTrue(\"Needs haste\", permanent.getAbilities(currentGame).contains(HasteAbility.getInstance())); Assert.assertEquals(\"Power is wrong\", true ? 3 : 6, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness is wrong\", true ? 2 : 4, permanent.getToughness().getValue()); Assert.assertTrue(\"Color is wrong\", true ? permanent.getColor(currentGame).isRed() : permanent.getColor(currentGame).isColorless() ); Assert.assertEquals(\"Mana cost is wrong\", true ? \"{2}{R}\" : \"{7}\", permanent.getManaCost().getText()); Assert.assertEquals(\"Mana value is wrong\", true ? 3 : 7, permanent.getManaValue()); }"
        }
      ]
    },
    {
      "name": "testTokenCopyRegular",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cackling Counterpart",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Cackling Counterpart",
          "target": "Blitz Automaton"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "for (Permanent permanent : currentGame.getBattlefield().getActivePermanents( StaticFilters.FILTER_PERMANENT, playerA.getId(), currentGame )) { if (!permanent.getName().equals(automaton)) { continue; } Assert.assertTrue(\"Needs haste\", permanent.getAbilities(currentGame).contains(HasteAbility.getInstance())); Assert.assertEquals(\"Power is wrong\", false ? 3 : 6, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness is wrong\", false ? 2 : 4, permanent.getToughness().getValue()); Assert.assertTrue(\"Color is wrong\", false ? permanent.getColor(currentGame).isRed() : permanent.getColor(currentGame).isColorless() ); Assert.assertEquals(\"Mana cost is wrong\", false ? \"{2}{R}\" : \"{7}\", permanent.getManaCost().getText()); Assert.assertEquals(\"Mana value is wrong\", false ? 3 : 7, permanent.getManaValue()); }"
        }
      ]
    },
    {
      "name": "testTokenCopyPrototype",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cackling Counterpart",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton using Prototype"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Cackling Counterpart",
          "target": "Blitz Automaton"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "for (Permanent permanent : currentGame.getBattlefield().getActivePermanents( StaticFilters.FILTER_PERMANENT, playerA.getId(), currentGame )) { if (!permanent.getName().equals(automaton)) { continue; } Assert.assertTrue(\"Needs haste\", permanent.getAbilities(currentGame).contains(HasteAbility.getInstance())); Assert.assertEquals(\"Power is wrong\", true ? 3 : 6, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness is wrong\", true ? 2 : 4, permanent.getToughness().getValue()); Assert.assertTrue(\"Color is wrong\", true ? permanent.getColor(currentGame).isRed() : permanent.getColor(currentGame).isColorless() ); Assert.assertEquals(\"Mana cost is wrong\", true ? \"{2}{R}\" : \"{7}\", permanent.getManaCost().getText()); Assert.assertEquals(\"Mana value is wrong\", true ? 3 : 7, permanent.getManaValue()); }"
        }
      ]
    },
    {
      "name": "testTokenCopyRegularLKI",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 13
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sublime Epiphany",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Sublime Epiphany"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "3"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "4"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "TestPlayer.MODE_SKIP"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Blitz Automaton"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Blitz Automaton"
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "for (Permanent permanent : currentGame.getBattlefield().getActivePermanents( StaticFilters.FILTER_PERMANENT, playerA.getId(), currentGame )) { if (!permanent.getName().equals(automaton)) { continue; } Assert.assertTrue(\"Needs haste\", permanent.getAbilities(currentGame).contains(HasteAbility.getInstance())); Assert.assertEquals(\"Power is wrong\", false ? 3 : 6, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness is wrong\", false ? 2 : 4, permanent.getToughness().getValue()); Assert.assertTrue(\"Color is wrong\", false ? permanent.getColor(currentGame).isRed() : permanent.getColor(currentGame).isColorless() ); Assert.assertEquals(\"Mana cost is wrong\", false ? \"{2}{R}\" : \"{7}\", permanent.getManaCost().getText()); Assert.assertEquals(\"Mana value is wrong\", false ? 3 : 7, permanent.getManaValue()); }"
        }
      ]
    },
    {
      "name": "testTokenCopyPrototypeLKI",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 9
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sublime Epiphany",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton using Prototype"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Sublime Epiphany"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "3"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "4"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "TestPlayer.MODE_SKIP"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Blitz Automaton"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Blitz Automaton"
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
          "op": "assertHandCount",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "for (Permanent permanent : currentGame.getBattlefield().getActivePermanents( StaticFilters.FILTER_PERMANENT, playerA.getId(), currentGame )) { if (!permanent.getName().equals(automaton)) { continue; } Assert.assertTrue(\"Needs haste\", permanent.getAbilities(currentGame).contains(HasteAbility.getInstance())); Assert.assertEquals(\"Power is wrong\", true ? 3 : 6, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness is wrong\", true ? 2 : 4, permanent.getToughness().getValue()); Assert.assertTrue(\"Color is wrong\", true ? permanent.getColor(currentGame).isRed() : permanent.getColor(currentGame).isColorless() ); Assert.assertEquals(\"Mana cost is wrong\", true ? \"{2}{R}\" : \"{7}\", permanent.getManaCost().getText()); Assert.assertEquals(\"Mana value is wrong\", true ? 3 : 7, permanent.getManaValue()); }"
        }
      ]
    },
    {
      "name": "testStackToughnessPrototyped",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Stern Scolding",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton using Prototype"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Stern Scolding"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Blitz Automaton"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 0
        },
        {
          "op": "unsupported",
          "source": "for (Permanent permanent : currentGame.getBattlefield().getActivePermanents( StaticFilters.FILTER_PERMANENT, playerA.getId(), currentGame )) { if (!permanent.getName().equals(automaton)) { continue; } Assert.assertTrue(\"Needs haste\", permanent.getAbilities(currentGame).contains(HasteAbility.getInstance())); Assert.assertEquals(\"Power is wrong\", true ? 3 : 6, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness is wrong\", true ? 2 : 4, permanent.getToughness().getValue()); Assert.assertTrue(\"Color is wrong\", true ? permanent.getColor(currentGame).isRed() : permanent.getColor(currentGame).isColorless() ); Assert.assertEquals(\"Mana cost is wrong\", true ? \"{2}{R}\" : \"{7}\", permanent.getManaCost().getText()); Assert.assertEquals(\"Mana value is wrong\", true ? 3 : 7, permanent.getManaValue()); }"
        }
      ]
    },
    {
      "name": "testStackColorPrototyped",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Douse",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton using Prototype"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{1}{U}: Counter target red spell"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Blitz Automaton"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 0
        },
        {
          "op": "unsupported",
          "source": "for (Permanent permanent : currentGame.getBattlefield().getActivePermanents( StaticFilters.FILTER_PERMANENT, playerA.getId(), currentGame )) { if (!permanent.getName().equals(automaton)) { continue; } Assert.assertTrue(\"Needs haste\", permanent.getAbilities(currentGame).contains(HasteAbility.getInstance())); Assert.assertEquals(\"Power is wrong\", true ? 3 : 6, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness is wrong\", true ? 2 : 4, permanent.getToughness().getValue()); Assert.assertTrue(\"Color is wrong\", true ? permanent.getColor(currentGame).isRed() : permanent.getColor(currentGame).isColorless() ); Assert.assertEquals(\"Mana cost is wrong\", true ? \"{2}{R}\" : \"{7}\", permanent.getManaCost().getText()); Assert.assertEquals(\"Mana value is wrong\", true ? 3 : 7, permanent.getManaValue()); }"
        }
      ]
    },
    {
      "name": "testStackManaValueRegular",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Access Denied",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Access Denied",
          "target": "Blitz Automaton"
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
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Thopter Token",
          "count": 7
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 0
        },
        {
          "op": "unsupported",
          "source": "for (Permanent permanent : currentGame.getBattlefield().getActivePermanents( StaticFilters.FILTER_PERMANENT, playerA.getId(), currentGame )) { if (!permanent.getName().equals(automaton)) { continue; } Assert.assertTrue(\"Needs haste\", permanent.getAbilities(currentGame).contains(HasteAbility.getInstance())); Assert.assertEquals(\"Power is wrong\", false ? 3 : 6, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness is wrong\", false ? 2 : 4, permanent.getToughness().getValue()); Assert.assertTrue(\"Color is wrong\", false ? permanent.getColor(currentGame).isRed() : permanent.getColor(currentGame).isColorless() ); Assert.assertEquals(\"Mana cost is wrong\", false ? \"{2}{R}\" : \"{7}\", permanent.getManaCost().getText()); Assert.assertEquals(\"Mana value is wrong\", false ? 3 : 7, permanent.getManaValue()); }"
        }
      ]
    },
    {
      "name": "testStackManaValuePrototype",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Access Denied",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton using Prototype"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Access Denied",
          "target": "Blitz Automaton"
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
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Thopter Token",
          "count": 3
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 0
        },
        {
          "op": "unsupported",
          "source": "for (Permanent permanent : currentGame.getBattlefield().getActivePermanents( StaticFilters.FILTER_PERMANENT, playerA.getId(), currentGame )) { if (!permanent.getName().equals(automaton)) { continue; } Assert.assertTrue(\"Needs haste\", permanent.getAbilities(currentGame).contains(HasteAbility.getInstance())); Assert.assertEquals(\"Power is wrong\", true ? 3 : 6, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness is wrong\", true ? 2 : 4, permanent.getToughness().getValue()); Assert.assertTrue(\"Color is wrong\", true ? permanent.getColor(currentGame).isRed() : permanent.getColor(currentGame).isColorless() ); Assert.assertEquals(\"Mana cost is wrong\", true ? \"{2}{R}\" : \"{7}\", permanent.getManaCost().getText()); Assert.assertEquals(\"Mana value is wrong\", true ? 3 : 7, permanent.getManaValue()); }"
        }
      ]
    },
    {
      "name": "testManaValueWhenCasting",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Void Winnower",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Fallaji Dragon Engine",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Boulderbranch Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Taiga",
          "count": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Cast Boulderbranch",
          "expected": true
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Taiga"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "label": "Cast Fallaji Dragon Engine using Prototype",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Fallaji Dragon Engine using Prototype"
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Fallaji Dragon Engine",
          "power": 1,
          "toughness": 3
        }
      ]
    },
    {
      "name": "testCopyOnStack",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Frontier Bivouac",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Double Major",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton using Prototype"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Double Major",
          "target": "Blitz Automaton"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "for (Permanent permanent : currentGame.getBattlefield().getActivePermanents( StaticFilters.FILTER_PERMANENT, playerA.getId(), currentGame )) { if (!permanent.getName().equals(automaton)) { continue; } Assert.assertTrue(\"Needs haste\", permanent.getAbilities(currentGame).contains(HasteAbility.getInstance())); Assert.assertEquals(\"Power is wrong\", true ? 3 : 6, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness is wrong\", true ? 2 : 4, permanent.getToughness().getValue()); Assert.assertTrue(\"Color is wrong\", true ? permanent.getColor(currentGame).isRed() : permanent.getColor(currentGame).isColorless() ); Assert.assertEquals(\"Mana cost is wrong\", true ? \"{2}{R}\" : \"{7}\", permanent.getManaCost().getText()); Assert.assertEquals(\"Mana value is wrong\", true ? 3 : 7, permanent.getManaValue()); }"
        }
      ]
    },
    {
      "name": "testHumility",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plateau",
          "count": 9
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Humility",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Disenchant",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Humility"
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
          "name": "Blitz Automaton using Prototype"
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Blitz Automaton",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Disenchant",
          "target": "Humility"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "for (Permanent permanent : currentGame.getBattlefield().getActivePermanents( StaticFilters.FILTER_PERMANENT, playerA.getId(), currentGame )) { if (!permanent.getName().equals(automaton)) { continue; } Assert.assertTrue(\"Needs haste\", permanent.getAbilities(currentGame).contains(HasteAbility.getInstance())); Assert.assertEquals(\"Power is wrong\", true ? 3 : 6, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness is wrong\", true ? 2 : 4, permanent.getToughness().getValue()); Assert.assertTrue(\"Color is wrong\", true ? permanent.getColor(currentGame).isRed() : permanent.getColor(currentGame).isColorless() ); Assert.assertEquals(\"Mana cost is wrong\", true ? \"{2}{R}\" : \"{7}\", permanent.getManaCost().getText()); Assert.assertEquals(\"Mana value is wrong\", true ? 3 : 7, permanent.getManaValue()); }"
        }
      ]
    },
    {
      "name": "testColorCostReduction",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ruby Medallion",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton using Prototype"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "for (Permanent permanent : currentGame.getBattlefield().getActivePermanents( StaticFilters.FILTER_PERMANENT, playerA.getId(), currentGame )) { if (!permanent.getName().equals(automaton)) { continue; } Assert.assertTrue(\"Needs haste\", permanent.getAbilities(currentGame).contains(HasteAbility.getInstance())); Assert.assertEquals(\"Power is wrong\", true ? 3 : 6, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness is wrong\", true ? 2 : 4, permanent.getToughness().getValue()); Assert.assertTrue(\"Color is wrong\", true ? permanent.getColor(currentGame).isRed() : permanent.getColor(currentGame).isColorless() ); Assert.assertEquals(\"Mana cost is wrong\", true ? \"{2}{R}\" : \"{7}\", permanent.getManaCost().getText()); Assert.assertEquals(\"Mana value is wrong\", true ? 3 : 7, permanent.getManaValue()); }"
        },
        {
          "op": "assertTappedCount",
          "name": "Mountain",
          "tapped": true,
          "count": 2
        }
      ]
    },
    {
      "name": "testAbilityRemovalPre",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Dress Down",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Dress Down"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton using Prototype"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Blitz Automaton",
          "power": 3,
          "toughness": 2
        }
      ]
    },
    {
      "name": "testAbilityRemovalPost",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Dress Down",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton using Prototype"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Dress Down"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Blitz Automaton",
          "power": 3,
          "toughness": 2
        }
      ]
    },
    {
      "name": "testEssenceOfWild",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Essence of the Wild",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Pyroclasm",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton using Prototype"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Pyroclasm"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Essence of the Wild",
          "count": 2
        }
      ]
    },
    {
      "name": "testChainer",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chainer, Nightmare Adept",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Discard a card:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Plains"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton using Prototype"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "for (Permanent permanent : currentGame.getBattlefield().getActivePermanents( StaticFilters.FILTER_PERMANENT, playerA.getId(), currentGame )) { if (!permanent.getName().equals(automaton)) { continue; } Assert.assertTrue(\"Needs haste\", permanent.getAbilities(currentGame).contains(HasteAbility.getInstance())); Assert.assertEquals(\"Power is wrong\", true ? 3 : 6, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness is wrong\", true ? 2 : 4, permanent.getToughness().getValue()); Assert.assertTrue(\"Color is wrong\", true ? permanent.getColor(currentGame).isRed() : permanent.getColor(currentGame).isColorless() ); Assert.assertEquals(\"Mana cost is wrong\", true ? \"{2}{R}\" : \"{7}\", permanent.getManaCost().getText()); Assert.assertEquals(\"Mana value is wrong\", true ? 3 : 7, permanent.getManaValue()); }"
        }
      ]
    },
    {
      "name": "testMetamorphCopyA",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 12
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Hulking Metamorph",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton using Prototype"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Hulking Metamorph"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Blitz Automaton"
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Blitz Automaton",
          "power": 3,
          "toughness": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Blitz Automaton",
          "power": 7,
          "toughness": 7
        }
      ]
    },
    {
      "name": "testMetamorphCopyB",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 11
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Hulking Metamorph",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Hulking Metamorph using Prototype"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Blitz Automaton"
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Blitz Automaton",
          "power": 6,
          "toughness": 4
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Blitz Automaton",
          "power": 3,
          "toughness": 3
        }
      ]
    },
    {
      "name": "testReflectionA",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 15
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Goring Warplow",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Infinite Reflection",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton using Prototype"
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
          "name": "Infinite Reflection",
          "target": "Blitz Automaton"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Goring Warplow"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "for (Permanent permanent : currentGame.getBattlefield().getActivePermanents( StaticFilters.FILTER_PERMANENT, playerA.getId(), currentGame )) { if (!permanent.getName().equals(automaton)) { continue; } Assert.assertTrue(\"Needs haste\", permanent.getAbilities(currentGame).contains(HasteAbility.getInstance())); Assert.assertEquals(\"Power is wrong\", true ? 3 : 6, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness is wrong\", true ? 2 : 4, permanent.getToughness().getValue()); Assert.assertTrue(\"Color is wrong\", true ? permanent.getColor(currentGame).isRed() : permanent.getColor(currentGame).isColorless() ); Assert.assertEquals(\"Mana cost is wrong\", true ? \"{2}{R}\" : \"{7}\", permanent.getManaCost().getText()); Assert.assertEquals(\"Mana value is wrong\", true ? 3 : 7, permanent.getManaValue()); }"
        }
      ]
    },
    {
      "name": "testReflectionB",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Underground Sea",
          "count": 15
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Goring Warplow",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Infinite Reflection",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton"
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
          "name": "Infinite Reflection",
          "target": "Blitz Automaton"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Goring Warplow using Prototype"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "for (Permanent permanent : currentGame.getBattlefield().getActivePermanents( StaticFilters.FILTER_PERMANENT, playerA.getId(), currentGame )) { if (!permanent.getName().equals(automaton)) { continue; } Assert.assertTrue(\"Needs haste\", permanent.getAbilities(currentGame).contains(HasteAbility.getInstance())); Assert.assertEquals(\"Power is wrong\", false ? 3 : 6, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness is wrong\", false ? 2 : 4, permanent.getToughness().getValue()); Assert.assertTrue(\"Color is wrong\", false ? permanent.getColor(currentGame).isRed() : permanent.getColor(currentGame).isColorless() ); Assert.assertEquals(\"Mana cost is wrong\", false ? \"{2}{R}\" : \"{7}\", permanent.getManaCost().getText()); Assert.assertEquals(\"Mana value is wrong\", false ? 3 : 7, permanent.getManaValue()); }"
        }
      ]
    },
    {
      "name": "testProgenitor",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Frontier Bivouac",
          "count": 9
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Progenitor Mimic",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton using Prototype"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Progenitor Mimic"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Blitz Automaton"
        },
        {
          "op": "setStopAt",
          "turn": 3,
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 3
        },
        {
          "op": "unsupported",
          "source": "for (Permanent permanent : currentGame.getBattlefield().getActivePermanents( StaticFilters.FILTER_PERMANENT, playerA.getId(), currentGame )) { if (!permanent.getName().equals(automaton)) { continue; } Assert.assertTrue(\"Needs haste\", permanent.getAbilities(currentGame).contains(HasteAbility.getInstance())); Assert.assertEquals(\"Power is wrong\", true ? 3 : 6, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness is wrong\", true ? 2 : 4, permanent.getToughness().getValue()); Assert.assertTrue(\"Color is wrong\", true ? permanent.getColor(currentGame).isRed() : permanent.getColor(currentGame).isColorless() ); Assert.assertEquals(\"Mana cost is wrong\", true ? \"{2}{R}\" : \"{7}\", permanent.getManaCost().getText()); Assert.assertEquals(\"Mana value is wrong\", true ? 3 : 7, permanent.getManaValue()); }"
        }
      ]
    },
    {
      "name": "testInstantaneousLKI",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volcanic Island",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Flowstone Surge",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Drizzt Do'Urden",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Warstorm Surge",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Slimebind",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Slimebind",
          "target": "Drizzt Do'Urden"
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Drizzt Do'Urden",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton using Prototype"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Whenever a creature you control enters"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Drizzt Do'Urden",
          "power": 5,
          "toughness": 5
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 15
        }
      ]
    },
    {
      "name": "testReanimate",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Badlands",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cut Down",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Blitz Automaton using Prototype"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Cut Down",
          "target": "Blitz Automaton"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Reanimate",
          "target": "Blitz Automaton"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Blitz Automaton",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "for (Permanent permanent : currentGame.getBattlefield().getActivePermanents( StaticFilters.FILTER_PERMANENT, playerA.getId(), currentGame )) { if (!permanent.getName().equals(automaton)) { continue; } Assert.assertTrue(\"Needs haste\", permanent.getAbilities(currentGame).contains(HasteAbility.getInstance())); Assert.assertEquals(\"Power is wrong\", false ? 3 : 6, permanent.getPower().getValue()); Assert.assertEquals(\"Toughness is wrong\", false ? 2 : 4, permanent.getToughness().getValue()); Assert.assertTrue(\"Color is wrong\", false ? permanent.getColor(currentGame).isRed() : permanent.getColor(currentGame).isColorless() ); Assert.assertEquals(\"Mana cost is wrong\", false ? \"{2}{R}\" : \"{7}\", permanent.getManaCost().getText()); Assert.assertEquals(\"Mana value is wrong\", false ? 3 : 7, permanent.getManaValue()); }"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 13
        }
      ]
    }
  ]
});
