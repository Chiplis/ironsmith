import test from 'node:test';
import assert from 'node:assert/strict';
import { findAbilityPreference } from '../src/lib/ability-preferences.js';
const exalted = { source_ability_text:'Exalted', automation_identity:'trigger:123' };
test('keyword and full rule text identify only an unambiguous native trigger', () => {
 assert.equal(findAbilityPreference([exalted],['exalted']),exalted);
 assert.equal(findAbilityPreference([exalted],['Exalted (Whenever a creature you control attacks alone...)']),exalted);
 assert.equal(findAbilityPreference([exalted],['Flying']),null);
 assert.equal(findAbilityPreference([exalted,{...exalted,automation_identity:'trigger:other'}],['Exalted']),null);
});
