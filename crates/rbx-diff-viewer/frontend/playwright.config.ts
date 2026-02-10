import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './tests/e2e',
  fullyParallel: false, // Sequential for server stability
  webServer: [
    {
      command: 'cd .. && cargo run --release',
      url: 'http://localhost:3000/api/status',
      reuseExistingServer: true,
      timeout: 60000,
    },
    {
      command: 'npm run dev',
      url: 'http://localhost:5173',
      reuseExistingServer: true,
    },
  ],
  use: {
    baseURL: 'http://localhost:5173',
    screenshot: 'on',
    trace: 'on-first-retry',
  },
  outputDir: './test-results',
});
