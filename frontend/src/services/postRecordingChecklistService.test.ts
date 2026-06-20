// @ts-nocheck
import assert from 'node:assert/strict';
import test from 'node:test';

import {
  completePostRecordingChecklistItem,
  getPostRecordingChecklistState,
  skipPostRecordingChecklist,
} from './postRecordingChecklistService.ts';

function installLocalStorage() {
  const values = new Map();
  globalThis.window = {
    localStorage: {
      getItem: (key) => values.get(key) ?? null,
      setItem: (key, value) => values.set(key, value),
      removeItem: (key) => values.delete(key),
      clear: () => values.clear(),
    },
  };
}

test('stores checklist completion per meeting without duplicating items', () => {
  installLocalStorage();

  completePostRecordingChecklistItem('meeting-1', 'screenshots');
  completePostRecordingChecklistItem('meeting-1', 'screenshots');
  completePostRecordingChecklistItem('meeting-1', 'notes');

  const state = getPostRecordingChecklistState('meeting-1');
  assert.equal(state.meetingId, 'meeting-1');
  assert.deepEqual(state.completedItemIds, ['screenshots', 'notes']);
  assert.equal(state.skipped, false);
});

test('stores skipped state separately for each meeting', () => {
  installLocalStorage();

  skipPostRecordingChecklist('meeting-1');
  completePostRecordingChecklistItem('meeting-2', 'agents');

  assert.equal(getPostRecordingChecklistState('meeting-1').skipped, true);
  assert.equal(getPostRecordingChecklistState('meeting-2').skipped, false);
  assert.deepEqual(getPostRecordingChecklistState('meeting-2').completedItemIds, ['agents']);
});
