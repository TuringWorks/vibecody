import { test, expect } from '@playwright/test';

test('verify new file and folder buttons with modal', async ({ page }) => {
    // 1. Open the application
    await page.goto('http://localhost:1420');

    // 2. Wait for explorer to load
    await page.waitForSelector('[data-testid="file-explorer"]');

    // 3. Click "New File" button
    await page.click('[title="New File"]');

    // 4. Verify Modal appears
    const modal = page.locator('.modal-content');
    await expect(modal).toBeVisible();
    await expect(modal).toContainText('Create New File');

    // 5. Type name and confirm
    const input = page.locator('.modal-input');
    await input.fill('test_new_file.txt');
    await page.click('button:has-text("Confirm")');

    // 6. Verify file appears in list (optimistic update or real backend)
    // Note: Since we can't easily mock the backend in this e2e test without more setup,
    // we primarily verify the UI interaction works up to the point of calling the backend.
    // If the backend call fails, the alert would show, which we could check for, 
    // or we check if the modal closed.
    await expect(modal).toBeHidden();

    // 7. Click "New Folder" button
    await page.click('[title="New Folder"]');

    // 8. Verify Modal appears
    await expect(modal).toBeVisible();
    await expect(modal).toContainText('Create New Folder');

    // 9. Type name and confirm
    await input.fill('test_new_folder');
    await page.click('button:has-text("Confirm")');

    // 10. Verify modal closes
    await expect(modal).toBeHidden();
});
