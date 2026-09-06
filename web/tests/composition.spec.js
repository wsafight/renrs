import {test, expect} from '@playwright/test';

for (const width of [1280, 390]) {
  test(`nested viewports and persistent data controls at ${width}`, async ({page}, info) => {
    await page.setViewportSize({width, height: 844});
    const errors = []; page.on('pageerror', error => errors.push(error.message));
    await page.goto('/composition/');
    await page.locator('#settings').click();
    await page.getByRole('button', {name: 'Map', exact: true}).click();
    await expect(page.locator('#modal')).toContainText('Selected: Map');
    await page.getByRole('button', {name: 'Pack selected item', exact: true}).click();
    await page.getByRole('button', {name: 'Travel bag', exact: true}).click();
    await expect(page.locator('#modal')).toContainText('Packed: Map');
    await page.getByRole('button', {name: 'Collect reward', exact: true}).click();
    await expect(page.locator('#modal')).toContainText('Reward: 1');
    if (width === 1280) {
      await page.getByRole('button', {name: 'Radio', exact: true}).click();
      await page.getByRole('button', {name: 'Pack selected item', exact: true}).dragTo(page.getByRole('button', {name: 'Travel bag', exact: true}));
      await expect(page.locator('#modal')).toContainText('Packed: Radio');
    }
    const notes = page.getByLabel('notes', {exact: true});
    await notes.evaluate(node => { node.scrollTop = node.scrollHeight; });
    await expect.poll(() => notes.evaluate(node => node.scrollTop)).toBeGreaterThan(0);
    await page.screenshot({path: info.outputPath('composition.png'), fullPage: true});
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await page.locator('#close').click();
    await page.locator('#saves').click();
    await page.getByRole('button', {name: 'Save', exact: true}).first().click();
    const summaries = await page.evaluate(() => new Promise((resolve, reject) => {
      const request = indexedDB.open('renrs-saves');
      request.onsuccess = () => {
        const db = request.result, tx = db.transaction('summaries');
        const rows = tx.objectStore('summaries').getAll(); rows.onsuccess = () => {resolve(rows.result); db.close();}; rows.onerror = () => reject(rows.error);
      }; request.onerror = () => reject(request.error);
    }));
    expect(summaries.length).toBeGreaterThan(0);
    expect(summaries.every(row => !('snapshot' in row))).toBe(true);
    page.once('dialog', dialog => dialog.accept());
    await page.getByRole('button', {name: 'Load', exact: true}).first().click();
    await page.locator('#settings').click();
    await expect(page.locator('#modal')).toContainText(width === 1280 ? 'Packed: Radio' : 'Packed: Map');
    await expect(page.locator('#modal')).toContainText('Reward: 1');
    expect(errors).toEqual([]);
  });
}
