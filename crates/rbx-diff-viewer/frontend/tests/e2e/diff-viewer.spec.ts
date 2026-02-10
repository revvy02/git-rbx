import { test, expect } from '@playwright/test';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FIXTURES_DIR = path.join(__dirname, '../../../tests/fixtures');

// Helper to ensure we're on the diff view (upload if needed)
async function ensureDiffView(page: import('@playwright/test').Page) {
  await page.goto('/');

  // Wait a moment for page to settle
  await page.waitForTimeout(500);

  // Check if diff view is already showing (files already loaded in backend)
  const diffView = page.locator('.diff-view');
  if (await diffView.isVisible().catch(() => false)) {
    return; // Already on diff view
  }

  // Need to upload files
  const oldInput = page.locator('.upload-box:first-child input[type="file"]');
  const newInput = page.locator('.upload-box:last-child input[type="file"]');
  await oldInput.setInputFiles(path.join(FIXTURES_DIR, 'house_no_primary_part.rbxm'));
  await newInput.setInputFiles(path.join(FIXTURES_DIR, 'house_insane_case.rbxm'));
  await page.click('.compare-btn');

  // Wait for diff view
  await expect(page.locator('.diff-view')).toBeVisible({ timeout: 15000 });
}

test.describe('Diff Viewer', () => {
  test('full workflow', async ({ page }) => {
    await ensureDiffView(page);

    // Screenshot 1: Initial diff view
    await page.screenshot({ path: 'test-results/diff-view.png', fullPage: true });

    // Check icons are present
    const icons = page.locator('.class-icon[src^="/images/"]');
    const iconCount = await icons.count();
    console.log(`Found ${iconCount} class icons`);

    // Expand a node
    const expandIcons = page.locator('.expand-icon:has-text("▶")');
    if ((await expandIcons.count()) > 0) {
      await expandIcons.first().click();
      await page.waitForTimeout(500);
    }
    await page.screenshot({ path: 'test-results/expanded-tree.png', fullPage: true });

    // Select a Model node to show properties (not DataModel which has no properties)
    const modelNodes = page.locator('.file-panel:first-child .node-row:has-text("[Model]")');
    if ((await modelNodes.count()) > 0) {
      await modelNodes.first().click();
      await page.waitForTimeout(500);
    }
    await page.screenshot({ path: 'test-results/properties.png', fullPage: true });
  });
});
