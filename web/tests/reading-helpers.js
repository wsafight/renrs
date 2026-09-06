export async function advance(page, button = page.locator('#next')) {
  if (await page.locator('#text').getAttribute('data-revealing') === 'true') await button.click();
  await button.click();
}
