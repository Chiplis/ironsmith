import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/MeldTest.java",
  "tests": [
    {
      "name": "testMeldAndRestrict",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bruna, the Fading Light",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gisela, the Broken Blade",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bruna, the Fading Light"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Silvercoat Lion"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gisela, the Broken Blade"
        },
        {
          "op": "castSpell",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Silvercoat Lion"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "unsupported",
          "source": "try { execute(); Assert.fail(\"must throw exception on execute\"); } catch (Throwable e) { if (!e.getMessage().contains(\"Cast Silvercoat Lion\")) { Assert.fail(\"Should have thrown error about casting Silvercoat Lion, but got:\\n\" + e.getMessage()); } } assertPermanentCount(playerA, brisela, 1)"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Bruna, the Fading Light",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Gisela, the Broken Blade",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        }
      ]
    },
    {
      "name": "testMeldAndStopRestrictIfMeldCreatureLeftBattlefield",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bruna, the Fading Light",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gisela, the Broken Blade",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 4
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
          "name": "Silvercoat Lion",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Vanish into Memory",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bruna, the Fading Light"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Silvercoat Lion"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gisela, the Broken Blade"
        },
        {
          "op": "castSpell",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Vanish into Memory",
          "target": "Brisela, Voice of Nightmares"
        },
        {
          "op": "castSpell",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Silvercoat Lion"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Bruna, the Fading Light",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Gisela, the Broken Blade",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Brisela, Voice of Nightmares",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Vanish into Memory",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1,
          "name": 11
        }
      ]
    },
    {
      "name": "testMeld3",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bruna, the Fading Light",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gisela, the Broken Blade",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 4
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
          "name": "Silvercoat Lion",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Vanish into Memory",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bruna, the Fading Light"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Silvercoat Lion"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gisela, the Broken Blade"
        },
        {
          "op": "castSpell",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Vanish into Memory",
          "target": "Brisela, Voice of Nightmares"
        },
        {
          "op": "waitStackResolved",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Silvercoat Lion"
        },
        {
          "op": "setStopAt",
          "turn": 8,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Bruna, the Fading Light",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Gisela, the Broken Blade",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Brisela, Voice of Nightmares",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Vanish into Memory",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1,
          "name": 1
        }
      ]
    },
    {
      "name": "testReturnToHand",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hanweir Garrison",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hanweir Battlements",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Unsummon",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{3}{R}{R}"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Unsummon",
          "target": "Hanweir Garrison"
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
          "player": 1,
          "name": "Unsummon",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Hanweir Battlements",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Hanweir Garrison",
          "count": 1
        }
      ]
    },
    {
      "name": "testUnmeldAfterRollback",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hanweir Garrison",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hanweir Battlements",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Unsummon",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{3}{R}{R}"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Unsummon",
          "target": "Hanweir, the Writhing Township"
        },
        {
          "op": "unsupported",
          "source": "rollbackTurns(2, PhaseStep.BEGIN_COMBAT, playerB, 0)"
        },
        {
          "op": "unsupported",
          "source": "rollbackAfterActionsStart()"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Unsummon",
          "target": "Hanweir, the Writhing Township"
        },
        {
          "op": "unsupported",
          "source": "rollbackAfterActionsEnd()"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Unsummon",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Hanweir Battlements",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Hanweir Garrison",
          "count": 1
        }
      ]
    },
    {
      "name": "testMeldManaValue",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 11
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bruna, the Fading Light",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gisela, the Broken Blade",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Clone",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Bruna, the Fading Light"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Gisela, the Broken Blade"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Bruna, the Fading Light"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Brisela, Voice of Nightmares",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Clone"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Brisela, Voice of Nightmares"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Brisela, Voice of Nightmares",
          "count": 1
        }
      ]
    },
    {
      "name": "testMeldManaValue_Urza",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Urza, Lord Protector",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "The Mightstone and Weakstone",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Mythos of Illuna",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{7}: If you both own and control {this} and"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "The Mightstone and Weakstone"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Urza, Planeswalker",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Mythos of Illuna",
          "target": "Urza, Planeswalker"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Urza, Planeswalker",
          "count": 1
        }
      ]
    },
    {
      "name": "testMeld_Urza_Eliminate",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Urza, Lord Protector",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "The Mightstone and Weakstone",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Eliminate",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{7}: If you both own and control {this} and"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "The Mightstone and Weakstone"
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
          "name": "Urza, Planeswalker",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 1,
          "name": "Eliminate",
          "target": "Urza, Planeswalker"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "unsupported",
          "source": "try { execute(); Assert.fail(\"must throw exception on execute\"); } catch (Throwable e) { Assert.assertEquals(\"Player PlayerB must have 0 actions but found 1\", e.getMessage()); }"
        }
      ]
    },
    {
      "name": "testMeld_Urza_Eliminate_After_Rollback",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Urza, Lord Protector",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "The Mightstone and Weakstone",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{7}: If you both own and control {this} and"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "The Mightstone and Weakstone"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Urza, Planeswalker",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "rollbackTurns(2, PhaseStep.POSTCOMBAT_MAIN, playerA, 0)"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Urza, Planeswalker",
          "count": 1
        }
      ]
    }
  ]
});
