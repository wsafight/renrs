import assert from 'node:assert/strict';
import { test } from 'vitest';
import { ReadStore } from '../src/read-store';
import { RevealClock } from '../src/reveal';

test('read history batches thousands of statements, ignores duplicates and retries failed storage', () => {
  const writes: string[][] = [],
    errors: string[] = [];
  let fail = false;
  const storage = {
    getItem: () => '["already-read"]',
    setItem: (_key: string, data: string) => {
      if (fail) throw new Error('storage full');
      writes.push(JSON.parse(data) as string[]);
    },
  };
  const store = new ReadStore(storage, 'test', (error) => errors.push(String(error)));
  store.add('already-read');
  store.flush();
  assert.equal(writes.length, 0);
  for (let i = 0; i < 10000; i++) store.add(`line-${i}`);
  assert.equal(writes.length, 0);
  store.flush();
  assert.equal(writes.length, 1);
  assert.equal(writes[0].length, 10001);
  fail = true;
  store.add('retry');
  assert.equal(store.flush(), false);
  assert.equal(store.dirty, true);
  assert.deepEqual(errors, ['storage full']);
  fail = false;
  assert.equal(store.flush(), true);
  assert.equal(writes[1].at(-1), 'retry');
});

test('reveal clock pauses without catching up and restores partial progress at a new speed', () => {
  const clock = new RevealClock();
  clock.reset(30);
  clock.tick(0, 10);
  clock.tick(550, 10);
  assert.equal(clock.visible, 5);
  clock.pause();
  clock.tick(10000, 10);
  assert.equal(clock.visible, 5);
  clock.tick(10500, 20);
  assert.equal(clock.visible, 15);
  clock.reset(30, clock.visible);
  clock.tick(11000, 20);
  clock.tick(11250, 20);
  assert.equal(clock.visible, 20);
  clock.finish();
  assert.equal(clock.complete, true);
  clock.tick(50000, 100);
  assert.equal(clock.visible, 30);
});

test('reveal clock stops once per cue and fast-forwards to the next blocking cue', () => {
  const clock = new RevealClock();
  clock.reset(20, 0, [
    { position: 4, delayMs: null, fast: false },
    { position: 8, delayMs: null, fast: true },
    { position: 12, delayMs: 100, fast: false },
  ]);
  clock.tick(0, 10);
  clock.tick(500, 10);
  assert.equal(clock.visible, 4);
  assert.equal(clock.blocked, true);
  assert.equal(clock.skip(), true);
  clock.tick(500, 10);
  clock.tick(1000, 10);
  assert.equal(clock.visible, 12);
  assert.equal(clock.blocked, true);
  clock.tick(1050, 10);
  assert.equal(clock.blocked, true);
  clock.tick(1100, 10);
  assert.equal(clock.blocked, false);
  assert.equal(clock.skip(), true);
  assert.equal(clock.visible, 20);
});
