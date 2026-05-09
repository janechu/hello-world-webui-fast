// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { expect, test } from "@playwright/test";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

interface AppState {
    greeting: string;
}

const stateFilePath = resolve(__dirname, "..", "app", "data", "state.json");
const state: AppState = JSON.parse(readFileSync(stateFilePath, "utf-8"));

test.describe("hello-world custom element", () => {
    test("renders the greeting from state.json", async ({ page }) => {
        await page.goto("/");

        const heading = page.locator("hello-world h1");
        await expect(heading).toHaveText(state.greeting);
    });

    test("clicking the button shows an alert reading 'button pressed!'", async ({ page }) => {
        const dialogMessages: string[] = [];
        page.on("dialog", async dialog => {
            dialogMessages.push(dialog.message());
            await dialog.dismiss();
        });

        await page.goto("/");

        await page.waitForFunction(
            () => customElements.get("hello-world") !== undefined,
        );
        await page.evaluate(
            () =>
                new Promise<void>(r =>
                    requestAnimationFrame(() => requestAnimationFrame(() => r())),
                ),
        );

        await page.locator("hello-world button").click();

        await expect.poll(() => dialogMessages.at(-1)).toBe("button pressed!");
    });
});
