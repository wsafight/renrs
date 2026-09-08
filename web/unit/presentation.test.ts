import { expect, test } from 'vitest';
import { storyAnnouncement } from '../src/presentation';

test('dialogue announcement preserves speaker context for assistive technology', () => {
  expect(storyAnnouncement({ speaker_name: 'Mira', text: 'The signal is clear.' })).toBe(
    'Mira: The signal is clear.',
  );
  expect(storyAnnouncement({ text: 'A quiet morning.' })).toBe('A quiet morning.');
  expect(storyAnnouncement(null)).toBe('');
});
