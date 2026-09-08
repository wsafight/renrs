import { expect, test } from '@playwright/test';
import { advance } from './reading-helpers';

test('video advances, freezes in saves and restores its remaining time', async ({
  page,
}, testInfo) => {
  test.skip(!process.env.RENRS_MEDIA_TEST, 'Requires the generated media fixture at /media/');
  await page.setViewportSize({ width: 390, height: 844 });
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto('/media/');
  await expect(page.locator('#text')).toContainText('Media checkpoint');
  await advance(page);
  await expect
    .poll(() => page.locator('#background').getAttribute('src'))
    .toContain('/clips/intro/');
  await expect(page.locator('#background')).toHaveCSS('object-fit', 'contain');
  await expect(page.locator('#sprites')).toBeHidden();
  const first = await page.locator('#background').getAttribute('src');
  await expect.poll(() => page.locator('#background').getAttribute('src')).not.toBe(first);
  await page.getByRole('button', { name: 'Saves', exact: true }).click();
  await page.getByRole('textbox', { name: 'Slot 1 note' }).fill('During video');
  await page.getByRole('button', { name: 'Save', exact: true }).first().click();
  await expect(page.getByText('During video', { exact: true })).toBeVisible();
  const paused = await page.locator('#background').getAttribute('src');
  await page.waitForTimeout(2100);
  expect(await page.locator('#background').getAttribute('src')).toBe(paused);
  await page.getByRole('button', { name: 'Close', exact: true }).click();
  await page.screenshot({ path: testInfo.outputPath('video.png'), fullPage: true });
  await expect(page.locator('#text')).toContainText('Video finished');
  await page.getByRole('button', { name: 'Saves', exact: true }).click();
  page.once('dialog', (dialog) => dialog.accept());
  await page.getByRole('button', { name: 'Load', exact: true }).first().click();
  await expect(page.locator('#modal')).not.toBeVisible();
  expect(await page.locator('#background').getAttribute('src')).toBe(paused);
  await expect(page.locator('#text')).toContainText('Video finished');
  await advance(page);
  await page.getByRole('button', { name: 'Finish', exact: true }).click();
  await page.getByRole('button', { name: 'Collection', exact: true }).click();
  await expect(page.getByText('Media complete', { exact: true })).toBeVisible();
  await expect
    .poll(() =>
      page
        .getByRole('img', { name: 'Rooftop', exact: true })
        .evaluate<number, void, HTMLImageElement>((image) => image.naturalWidth),
    )
    .toBeGreaterThan(0);
  await page.screenshot({ path: testInfo.outputPath('collection.png'), fullPage: true });
  expect(errors).toEqual([]);
});
