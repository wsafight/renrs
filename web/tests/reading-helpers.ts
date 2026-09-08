import type { Locator, Page } from '@playwright/test';

export async function advance(page: Page, button: Locator = page.locator('#next')) {
  if ((await page.locator('#text').getAttribute('data-revealing')) === 'true') await button.click();
  await button.click();
}
