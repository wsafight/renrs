import {test} from 'node:test';
import assert from 'node:assert/strict';
import {ReadStore} from '../src/read-store.js';
import {RevealClock} from '../src/reveal.js';

test('read history batches thousands of statements, ignores duplicates and retries failed storage', () => {
  const writes = [], errors = []; let fail = false;
  const storage = {getItem: () => '["already-read"]', setItem: (_key, data) => {
    if (fail) throw new Error('storage full'); writes.push(JSON.parse(data));
  }};
  const store = new ReadStore(storage, 'test', error => errors.push(error));
  store.add('already-read'); store.flush(); assert.equal(writes.length, 0);
  for (let i = 0; i < 10000; i++) store.add(`line-${i}`);
  assert.equal(writes.length, 0); store.flush();
  assert.equal(writes.length, 1); assert.equal(writes[0].length, 10001);
  fail = true; store.add('retry'); assert.equal(store.flush(), false);
  assert.equal(store.dirty, true); assert.deepEqual(errors, ['storage full']);
  fail = false; assert.equal(store.flush(), true); assert.equal(writes[1].at(-1), 'retry');
});

test('reveal clock pauses without catching up and restores partial progress at a new speed', () => {
  const clock = new RevealClock(); clock.reset(30);
  clock.tick(0, 10); clock.tick(550, 10); assert.equal(clock.visible, 5);
  clock.pause(); clock.tick(10000, 10); assert.equal(clock.visible, 5);
  clock.tick(10500, 20); assert.equal(clock.visible, 15);
  clock.reset(30, clock.visible); clock.tick(11000, 20); clock.tick(11250, 20);
  assert.equal(clock.visible, 20); clock.finish(); assert.equal(clock.complete, true);
  clock.tick(50000, 100); assert.equal(clock.visible, 30);
});
