import { expect, test } from '@playwright/test';
import { pages } from '../scripts/catalog.mjs';
import { deployment } from '../scripts/deployment.mjs';

const { base } = deployment();
const home = base;
const docs = `${base}docs/`;
const quickstart = `${base}start/quickstart/`;

test('homepage presents the product and navigation', async ({ page }, info) => {
  await page.goto(home);
  await expect(page.locator('html')).toHaveAttribute('data-lang', 'en');
  await expect(page.getByRole('heading', { level: 1, name: /You write the story/ })).toBeVisible();
  await expect(page.locator('#tools')).toContainText("Don't retell");
  if (info.project.name === 'mobile') {
    await page.getByRole('button', { name: 'Open menu' }).click();
    await page
      .getByRole('navigation', { name: 'Mobile navigation' })
      .getByRole('link', { name: 'Docs' })
      .click();
  } else {
    await page
      .getByRole('navigation', { name: 'Primary navigation' })
      .getByRole('link', { name: 'Docs' })
      .click();
  }
  await expect(page).toHaveURL(docs);
  await expect(
    page.getByRole('heading', { level: 1, name: 'Guides and development records' }),
  ).toBeVisible();
});

test('mobile menu opens documentation', async ({ page }, info) => {
  test.skip(info.project.name !== 'mobile', 'mobile menu is tested at 390px');
  await page.goto(home);
  await page.getByRole('button', { name: 'Open menu' }).click();
  await page
    .getByRole('navigation', { name: 'Mobile navigation' })
    .getByRole('link', { name: 'Docs' })
    .click();
  await expect(page).toHaveURL(docs);
});

test('docs catalog, table of contents and adjacent pages', async ({ page }, info) => {
  await page.goto(quickstart);
  const sidebar = page.getByRole('navigation', { name: 'Documentation' });
  await expect(sidebar.getByText('Getting started', { exact: true })).toBeVisible();
  await expect(sidebar.getByRole('link', { name: 'Quick start' })).toBeVisible();
  if (info.project.name !== 'mobile') {
    const toc = page.getByRole('navigation', { name: 'On this page' }).first();
    await expect(toc.getByRole('link', { name: 'Set up the environment' })).toBeVisible();
    await toc.getByRole('link', { name: 'Set up the environment' }).click();
    await expect(page).toHaveURL(/#set-up-the-environment/);
  }
  await page
    .getByRole('navigation', { name: 'Adjacent pages' })
    .getByRole('link', { name: /Workspace/ })
    .click();
  await expect(page.getByRole('heading', { level: 1, name: 'Workspace and SDK' })).toBeVisible();
});

test('search locates documentation by title', async ({ page }) => {
  await page.goto(home);
  await page.getByRole('button', { name: 'Search' }).click();
  const dialog = page.getByRole('dialog', { name: 'Search docs' });
  const box = dialog.getByRole('searchbox', { name: 'Search docs' });
  await expect(box).toBeVisible();
  await box.fill('Quick start');
  const result = dialog.getByRole('link', { name: /Quick start/ });
  await expect(result).toBeVisible();
  await result.click();
  await expect(page).toHaveURL(quickstart);
});

test('theme toggle updates the document theme', async ({ page }) => {
  await page.goto(quickstart);
  const root = page.locator('html');
  const before = await root.getAttribute('data-theme');
  await page.getByRole('button', { name: 'Toggle theme' }).click();
  await expect(root).not.toHaveAttribute('data-theme', before || '');
  const after = await root.getAttribute('data-theme');
  expect(['light', 'dark']).toContain(after);
});

test('language toggle defaults to English and persists Chinese', async ({ page }) => {
  await page.goto(home);
  const root = page.locator('html');
  await expect(root).toHaveAttribute('data-lang', 'en');
  await expect(page.getByRole('heading', { level: 1, name: /You write the story/ })).toBeVisible();
  await page.getByRole('button', { name: 'Switch to Chinese' }).click();
  await expect(root).toHaveAttribute('data-lang', 'zh');
  await expect(page.getByRole('heading', { level: 1, name: /你只管写故事/ })).toBeVisible();
  expect(await page.evaluate(() => localStorage.getItem('renrs-lang'))).toBe('zh');
  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('data-lang', 'zh');
  await expect(page.getByRole('heading', { level: 1, name: /你只管写故事/ })).toBeVisible();
  await page.getByRole('button', { name: 'Switch to English' }).click();
  expect(await page.evaluate(() => localStorage.getItem('renrs-lang'))).toBe('en');
  await expect(page.getByRole('heading', { level: 1, name: /You write the story/ })).toBeVisible();
});

test('documentation body follows the language toggle', async ({ page }) => {
  await page.goto(quickstart);
  await expect(page.getByRole('heading', { name: 'Set up the environment' })).toBeVisible();
  await expect(page.getByText('Install Rust 1.88 or later')).toBeVisible();
  await page.getByRole('button', { name: 'Switch to Chinese' }).click();
  await expect(page.getByRole('heading', { name: '准备环境' })).toBeVisible();
  await expect(page.getByText('安装 Rust 1.88 或更高版本')).toBeVisible();
});

test('docs URL lang query opens Chinese', async ({ page }) => {
  await page.goto(`${quickstart}?lang=zh`);
  await expect(page.locator('html')).toHaveAttribute('data-lang', 'zh');
  await expect(page.getByRole('heading', { name: '准备环境' })).toBeVisible();
});

test('code blocks can be copied', async ({ page, context }) => {
  await context.grantPermissions(['clipboard-read', 'clipboard-write']);
  await page.goto(quickstart);
  const button = page.getByRole('button', { name: 'Copy' }).first();
  await expect(button).toBeVisible();
  await button.click();
  await expect(page.getByRole('button', { name: 'Copied' }).first()).toBeVisible();
});

test('catalog pages render their titles', async ({ page }) => {
  const sample = pages.filter((item) =>
    ['start/quickstart', 'guides/scripting', 'reference/cli', 'project/status'].includes(item.slug),
  );
  for (const doc of sample) {
    await page.goto(`${base}${doc.slug}/`);
    await expect(page.getByRole('heading', { level: 1, name: doc.title.en })).toBeVisible();
  }
});
