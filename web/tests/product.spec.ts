import { expect, test } from '@playwright/test';
import { advance } from './reading-helpers';

for (const width of [1280, 390]) {
  test(`custom screens, preferences and portable saves at ${width}`, async ({ page }, info) => {
    await page.setViewportSize({ width, height: 844 });
    const errors: string[] = [];
    page.on('pageerror', (error) => errors.push(error.message));
    await page.goto('/product/');
    await page
      .locator('[data-screen=main_menu]')
      .getByRole('button', { name: 'New game', exact: true })
      .click();
    await expect(
      page.locator('.custom-dialogue strong').filter({ hasText: 'receiver' }),
    ).toBeVisible();
    await page.getByLabel('Name', { exact: true }).fill('Alex');
    await page
      .locator('[data-screen=dialogue]')
      .getByRole('button', { name: 'Save', exact: true })
      .click();
    await page.getByRole('textbox', { name: 'Slot 1 note' }).fill('Portable');
    await page.locator('#modal').getByRole('button', { name: 'Save', exact: true }).first().click();
    const downloading = page.waitForEvent('download');
    await page.getByRole('button', { name: 'Export', exact: true }).first().click();
    const stream = await (await downloading).createReadStream();
    let text = '';
    for await (const part of stream) text += part;
    expect(text).toContain('9007199254740993');
    expect(JSON.parse(text).container_version).toBe(2);
    await page.locator('input[type=file]').setInputFiles({
      name: 'desktop.json',
      mimeType: 'application/json',
      buffer: Buffer.from(text),
    });
    await expect(page.getByText('Portable', { exact: true })).toHaveCount(2);
    await page.getByRole('button', { name: 'Close', exact: true }).click();
    await advance(
      page,
      page.locator('.custom-dialogue').getByRole('button', { name: 'Continue', exact: true }),
    );
    await expect(page.locator('[data-screen=choices]')).toContainText('Alex');
    await page.getByRole('button', { name: 'Reply', exact: true }).click();
    await expect(page.locator('.custom-dialogue')).toContainText('Alex: 2');
    await page.getByRole('button', { name: 'Settings', exact: true }).click();
    await page.getByLabel('High contrast').check();
    await page.getByRole('button', { name: 'Pack map', exact: true }).click();
    await expect(page.locator('#modal')).toContainText('Bag: ["key","map"]');
    await expect(page.locator('body')).toHaveClass(/contrast/);
    await page.screenshot({ path: info.outputPath('custom-settings.png'), fullPage: true });
    await page.locator('#close').click();
    await page.screenshot({ path: info.outputPath('custom-story.png'), fullPage: true });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(
      true,
    );
    expect(errors).toEqual([]);
  });
}

for (const width of [1280, 390]) {
  test(`parallel animation, journal pages and soundtracks at ${width}`, async ({ page }, info) => {
    test.setTimeout(45000);
    await page.setViewportSize({ width, height: 844 });
    const errors: string[] = [];
    page.on('pageerror', (error) => errors.push(error.message));
    await page.addInitScript(() => {
      const Original = window.Audio;
      window.testAudio = [];
      window.Audio = new Proxy(Original, {
        construct(Target, args) {
          const audio = new Target(args[0] as string | undefined);
          window.testAudio.push(audio);
          return audio;
        },
      });
    });
    await page.goto('/reading/');
    await expect(page.locator('#text')).toContainText('journal is ready');
    const sprite = page.locator('#sprites img').first();
    const before = await sprite.boundingBox();
    await advance(page);
    await page.waitForTimeout(550);
    const moving = await sprite.boundingBox();
    if (!before || !moving) throw new Error('Expected the animated sprite to have bounds');
    expect(moving.x).toBeGreaterThan(before.x + 10);
    await page.screenshot({ path: info.outputPath('parallel.png'), fullPage: true });
    await expect(page.locator('#text ruby')).toContainText('Signal');
    await advance(page);
    await expect(page.locator('#text')).toContainText('Two frequencies');
    await expect(page.locator('#text')).toContainText('Signal');
    await page.screenshot({ path: info.outputPath('journal.png'), fullPage: true });
    await advance(page);
    await expect(page.locator('#text')).toHaveText('Mira: A new page.');
    await page.locator('#rollback').click();
    await expect(page.locator('#text')).toContainText('Two frequencies');
    await advance(page);
    await advance(page);
    await page.locator('#saves').click();
    await page.getByRole('button', { name: 'Save', exact: true }).nth(5).click();
    await page.locator('#close').click();
    for (const kind of ['Frame', 'Stream']) {
      if (kind === 'Stream') {
        await page.locator('#saves').click();
        page.once('dialog', (dialog) => dialog.accept());
        await page.getByRole('button', { name: 'Load', exact: true }).last().click();
      }
      await page.getByRole('button', { name: `${kind} soundtrack`, exact: true }).click();
      const clock = () =>
        page.evaluate(
          () =>
            window.testAudio
              .slice()
              .reverse()
              .find((audio) => audio.src.endsWith('/audio.wav'))?.currentTime || 0,
        );
      await expect.poll(clock).toBeGreaterThan(0.2);
      await page.locator('#saves').click();
      await expect(page.locator('#modal')).toBeVisible();
      await expect
        .poll(() =>
          page.evaluate(
            () =>
              window.testAudio
                .slice()
                .reverse()
                .find((audio) => audio.src.endsWith('/audio.wav'))?.paused,
          ),
        )
        .toBe(true);
      const paused = await clock();
      await page.waitForTimeout(250);
      expect(Math.abs((await clock()) - paused)).toBeLessThan(0.05);
      await page
        .getByRole('button', { name: 'Save', exact: true })
        .nth(kind === 'Frame' ? 0 : 1)
        .click();
      await page.locator('#close').click();
      await expect(page.locator('#text')).toContainText('Playback complete');
      await page.locator('#saves').click();
      page.once('dialog', (dialog) => dialog.accept());
      await page
        .getByRole('button', { name: 'Load', exact: true })
        .nth(kind === 'Frame' ? 0 : 1)
        .click();
      await expect.poll(clock).toBeGreaterThanOrEqual(paused - 0.06);
      expect(await clock()).toBeLessThan(paused + 0.6);
      await expect(page.locator('#text')).toContainText('Playback complete');
    }
    expect(errors).toEqual([]);
  });
}
