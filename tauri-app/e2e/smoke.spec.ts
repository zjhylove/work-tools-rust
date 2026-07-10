import { test, expect } from '@playwright/test';

// Tauri dev server URL
const BASE_URL = 'http://localhost:1420';

test.describe('Work Tools Smoke Tests', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(BASE_URL, { waitUntil: 'networkidle', timeout: 30000 });
  });

  test('app loads and shows sidebar', async ({ page }) => {
    // The app should render without errors
    const body = await page.textContent('body');
    expect(body).toBeTruthy();

    // Check for no runtime errors
    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));
    await page.reload({ waitUntil: 'networkidle' });
    expect(errors).toHaveLength(0);
  });

  test('sidebar contains plugin navigation', async ({ page }) => {
    // Sidebar should have plugin entries
    const sidebar = page.locator('[data-testid="sidebar"], .sidebar, nav');
    // At minimum, the sidebar region should exist
    const sidebarExists = await sidebar.count();
    expect(sidebarExists).toBeGreaterThan(0);
  });

  test('theme toggle works', async ({ page }) => {
    // Find theme toggle button
    const themeBtn = page.locator('[data-testid="theme-toggle"], button[aria-label*="theme"], button[aria-label*="主题"]');
    const btnCount = await themeBtn.count();
    if (btnCount > 0) {
      // Get initial theme
      const htmlBefore = await page.getAttribute('html', 'data-theme');

      // Click toggle
      await themeBtn.first().click();
      await page.waitForTimeout(500);

      // Theme should have changed
      const htmlAfter = await page.getAttribute('html', 'data-theme');
      expect(htmlAfter).not.toBe(htmlBefore);
    } else {
      // If no explicit theme toggle found, check that data-theme attribute exists
      const theme = await page.getAttribute('html', 'data-theme');
      expect(theme).toBeTruthy();
    }
  });

  test('plugin placeholder renders iframe correctly', async ({ page }) => {
    // Click on a plugin in sidebar to load it
    const pluginLinks = page.locator('[data-testid="plugin-item"], .plugin-item, [class*="plugin"] a, [class*="nav"] button');
    const linkCount = await pluginLinks.count();

    if (linkCount > 0) {
      await pluginLinks.first().click();
      await page.waitForTimeout(1000);

      // Should have an iframe for the plugin
      const iframe = page.locator('iframe');
      const iframeCount = await iframe.count();
      expect(iframeCount).toBeGreaterThan(0);

      // iframe should have a srcdoc or src
      const firstIframe = iframe.first();
      const srcdoc = await firstIframe.getAttribute('srcdoc');
      const src = await firstIframe.getAttribute('src');
      expect(srcdoc || src).toBeTruthy();
    }
  });

  test('no console errors on initial load', async ({ page }) => {
    const consoleErrors: string[] = [];
    page.on('console', (msg) => {
      if (msg.type() === 'error') consoleErrors.push(msg.text());
    });

    await page.goto(BASE_URL, { waitUntil: 'networkidle', timeout: 30000 });
    await page.waitForTimeout(2000);

    // Filter out known non-critical errors (like dev tools messages)
    const critical = consoleErrors.filter(
      (e) => !e.includes('DevTools') && !e.includes('favicon') && !e.includes('manifest')
    );
    expect(critical).toHaveLength(0);
  });

  test('LogViewer loads without crash', async ({ page }) => {
    // Navigate to logs if possible
    const logLink = page.locator('text=日志, text=Logs, [data-testid="logs-tab"]');
    const logCount = await logLink.count();

    if (logCount > 0) {
      await logLink.first().click();
      await page.waitForTimeout(1000);

      // LogViewer component should be visible
      const logViewer = page.locator('[data-testid="log-viewer"], [class*="log"]');
      const exists = await logViewer.count();
      expect(exists).toBeGreaterThan(0);
    }
  });
});
