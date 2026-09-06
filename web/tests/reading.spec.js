import {test, expect} from '@playwright/test';

test('partial reveal, modal pause, portable save progress and click-to-complete', async ({page}, info) => {
  await page.addInitScript(() => {
    window.readWrites = 0;
    const original = Storage.prototype.setItem;
    Storage.prototype.setItem = function(key, value) {
      if (key.endsWith(':read')) window.readWrites++;
      return original.call(this, key, value);
    };
    localStorage.setItem('renrs:org.renrs.signal-at-dusk:settings', JSON.stringify({text_speed: 10}));
  });
  await page.goto('/');
  const text = page.locator('#text');
  await expect(text).toHaveAttribute('data-revealing', 'true');
  await expect.poll(async () => Number(await text.getAttribute('data-visible-characters'))).toBeGreaterThan(1);
  expect(await page.evaluate(() => window.readWrites)).toBe(0);
  const bounds = await text.boundingBox();
  await page.locator('#saves').click();
  const visible = await text.getAttribute('data-visible-characters');
  await page.waitForTimeout(300);
  await expect(text).toHaveAttribute('data-visible-characters', visible);
  await page.getByRole('button', {name: 'Save', exact: true}).first().click();
  expect(await page.evaluate(() => window.readWrites)).toBe(1);
  const downloading = page.waitForEvent('download');
  await page.getByRole('button', {name: 'Export', exact: true}).first().click();
  const stream = await (await downloading).createReadStream();
  let data = ''; for await (const part of stream) data += part;
  expect(JSON.parse(data).presentation.visible_characters).toBe(Number(visible));
  await page.locator('#close').click();
  await page.locator('#next').click();
  await expect(text).toHaveAttribute('data-revealing', 'false');
  expect(await text.boundingBox()).toEqual(bounds);
  await page.screenshot({path: info.outputPath('revealed.png'), fullPage: true});
  await page.locator('#next').click();
  await expect(page.getByRole('button', {name: 'Let Mira explain', exact: true})).toBeVisible();
  await page.locator('#saves').click();
  page.once('dialog', dialog => dialog.accept());
  await page.getByRole('button', {name: 'Load', exact: true}).first().click();
  await expect(text).toHaveAttribute('data-revealing', 'true');
  expect(Number(await text.getAttribute('data-visible-characters'))).toBeLessThan(Number(visible) + 4);
  await expect.poll(async () => Number(await text.getAttribute('data-visible-characters'))).toBeGreaterThan(Number(visible));
  await page.evaluate(() => dispatchEvent(new PageTransitionEvent('pagehide')));
  expect(await page.evaluate(() => window.readWrites)).toBe(1);
});

test('auto advance waits for reveal and its post-reveal delay', async ({page}) => {
  await page.addInitScript(() => localStorage.setItem('renrs:org.renrs.signal-at-dusk:settings', JSON.stringify({text_speed: 10, auto: true, auto_delay: .8, wait_voice: false})));
  await page.goto('/');
  await expect(page.locator('#text')).toHaveAttribute('data-revealing', 'true');
  await page.waitForTimeout(1000);
  await expect(page.locator('#text')).toHaveAttribute('data-revealing', 'true');
  await page.locator('#next').click();
  await expect(page.locator('#text')).toHaveAttribute('data-revealing', 'false');
  await page.waitForTimeout(200);
  await expect(page.getByRole('button', {name: 'Let Mira explain', exact: true})).toHaveCount(0);
  await expect(page.getByRole('button', {name: 'Let Mira explain', exact: true})).toBeVisible();
});

test('Unicode and ruby reveal keep complete scalar values and pause in the background', async ({page}) => {
  await page.route('**/project.json', async route => {
    const response = await route.fetch(), data = await response.json();
    const program = JSON.parse(data.program_json);
    program.instructions.find(item => item.kind.Dialogue).kind.Dialogue.text = 'A\u{1f600}{ruby=han}\u6c49{/ruby}Z';
    data.program_json = JSON.stringify(program);
    await route.fulfill({json: data});
  });
  await page.addInitScript(() => localStorage.setItem('renrs:org.renrs.signal-at-dusk:settings', JSON.stringify({text_speed: 1})));
  await page.goto('/');
  const text = page.locator('#text');
  await expect(text).toHaveAttribute('data-visible-characters', '2');
  const shown = () => text.evaluate(node => {
    const walker = document.createTreeWalker(node, NodeFilter.SHOW_TEXT); let result = '', current;
    while (current = walker.nextNode()) if (getComputedStyle(current.parentElement).visibility !== 'hidden') result += current.textContent;
    return result;
  });
  expect(await shown()).toBe('A\u{1f600}');
  const bounds = await text.boundingBox();
  await page.evaluate(() => { Object.defineProperty(document, 'hidden', {configurable: true, value: true}); document.dispatchEvent(new Event('visibilitychange')); });
  await page.waitForTimeout(1200); expect(await shown()).toBe('A\u{1f600}');
  await page.evaluate(() => { Object.defineProperty(document, 'hidden', {configurable: true, value: false}); document.dispatchEvent(new Event('visibilitychange')); });
  await expect(text).toHaveAttribute('data-revealing', 'false');
  expect(await shown()).toBe('A\u{1f600}\u6c49hanZ');
  expect(await text.boundingBox()).toEqual(bounds);
});
