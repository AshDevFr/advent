// Capture the documentation's screenshots from the built site (site/: the
// demo and the showcases), so they always match the current UI. Requests
// are answered from the folder directly: no server is started.
//
// Usage: node scripts/screenshots.mjs <site dir> <output dir>

import { mkdirSync } from "node:fs";
import { join, resolve } from "node:path";
import { chromium } from "playwright";

const [siteArg, outArg] = process.argv.slice(2);
if (!siteArg || !outArg) {
  console.error("usage: screenshots.mjs <site dir> <output dir>");
  process.exit(2);
}
const site = resolve(siteArg);
const out = resolve(outArg);
mkdirSync(out, { recursive: true });

// https, so the page is a secure context and WebCrypto is available.
const origin = "https://advent.test";
const desktop = { width: 1280, height: 900 };
const phone = { width: 390, height: 844 };
const sampleWords = ["light", "returns", "after", "the", "longest", "winter", "night"];

const browser = await chromium.launch();

async function open(path, { viewport = desktop, scheme = "dark", words } = {}) {
  const context = await browser.newContext({
    viewport,
    colorScheme: scheme,
    deviceScaleFactor: 2,
  });
  await context.route(`${origin}/**`, async (route) => {
    let path = new URL(route.request().url()).pathname;
    if (path.endsWith("/")) path += "index.html";
    try {
      await route.fulfill({ path: join(site, decodeURIComponent(path)) });
    } catch {
      await route.fulfill({ status: 404, body: "not found" });
    }
  });
  if (words) {
    await context.addInitScript((progress) => {
      localStorage.setItem("advent-progress", JSON.stringify(progress));
    }, {
      state: {
        events: {
          [words.event]: {
            words: Object.fromEntries(words.list.map((w, i) => [i + 1, w])),
            hintsUsed: {},
            attempts: {},
            name: "",
          },
        },
      },
      version: 1,
    });
  }
  const page = await context.newPage();
  await page.goto(`${origin}${path}`);
  await page.getByRole("list", { name: "Calendar doors" }).waitFor();
  await page.evaluate(() => document.fonts.ready);
  return page;
}

async function shot(page, name, locator) {
  const file = join(out, `${name}.png`);
  if (locator) await locator.screenshot({ path: file });
  else await page.screenshot({ path: file });
  console.log(`wrote ${file}`);
  await page.context().close();
}

let page = await open("/northwind/holiday-hunt/", { scheme: "light" });
await shot(page, "northwind-light");

page = await open("/christmas/christmas-2026/");
await shot(page, "christmas-dark");

page = await open("/christmas/christmas-2026/", { viewport: phone });
await shot(page, "christmas-phone");

page = await open("/demo/sample/");
await page.getByRole("button", { name: "Day 4, open" }).click();
await page.getByRole("dialog").getByLabel("Your answer").waitFor();
await shot(page, "door", page.getByRole("dialog"));

page = await open("/demo/sample/", {
  scheme: "light",
  words: { event: "sample", list: sampleWords },
});
await page.getByLabel("Name for the certificate").fill("Ada Lovelace");
await page.getByRole("button", { name: "Unlock certificate" }).click();
const certificate = page.getByRole("img", { name: /Certificate for Ada Lovelace/ });
await certificate.waitFor({ timeout: 20000 });
await page.waitForTimeout(300);
await shot(page, "certificate", certificate);

await browser.close();
