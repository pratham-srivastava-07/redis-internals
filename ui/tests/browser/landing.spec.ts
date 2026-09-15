import { test, expect } from "@playwright/test";

test("desktop landing, traffic controls, commands, and install", async ({
  page,
  context,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.setViewportSize({ width: 1440, height: 1000 });
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await page.goto("/", { waitUntil: "networkidle" });
  await expect(page).toHaveTitle(/Vynk/);
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute(
    "href",
    "http://localhost:3000",
  );
  await expect(page.locator('meta[property="og:title"]')).toHaveAttribute(
    "content",
    /Vynk/,
  );
  await expect(page.locator('meta[name="twitter:card"]')).toHaveAttribute(
    "content",
    "summary_large_image",
  );
  await expect(page.locator("h1")).toContainText("Make room for");
  await expect(page.locator("body")).not.toContainText("Not yet published");
  await page.screenshot({ path: "test-results/desktop-hero.png" });
  await page.getByRole("button", { name: "See result" }).click();
  await expect(
    page.locator(".admission .policy-metrics strong").nth(0),
  ).toHaveText("24");
  await expect(page.locator(".lru .policy-metrics strong").nth(0)).toHaveText(
    "12",
  );
  await page
    .locator("#playground")
    .screenshot({ path: "test-results/playground.png" });
  await page.getByRole("button", { name: "Popular keys", exact: true }).click();
  await page.getByRole("button", { name: "See result" }).click();
  await expect(
    page.locator(".admission .policy-metrics strong").nth(0),
  ).toHaveText("42");
  await page
    .getByRole("button", { name: "Changing demand", exact: true })
    .click();
  await page.getByRole("button", { name: "Run traffic" }).click();
  await expect(
    page.getByRole("button", { name: "Pause", exact: true }),
  ).toBeVisible();
  await page.waitForTimeout(600);
  await page.getByRole("button", { name: "Pause", exact: true }).click();
  const paused = await page.locator(".traffic-count").innerText();
  await page.waitForTimeout(500);
  await expect(page.locator(".traffic-count")).toHaveText(paused);
  await page.getByRole("button", { name: "Reset", exact: false }).click();
  await expect(page.locator(".traffic-count")).toContainText("00 /");
  await page.getByRole("tab", { name: "Cache admission" }).click();
  await expect(page.getByRole("tabpanel")).toContainText("CACHE.PUT");
  await page.getByRole("tab", { name: "Cache admission" }).press("ArrowRight");
  await expect(page.getByRole("tab", { name: "Expiration" })).toBeFocused();
  await expect(page.getByRole("tabpanel")).toContainText("EXPIRE");
  await page
    .locator("summary")
    .filter({ hasText: "Does admission control change SET?" })
    .click();
  await expect(page.locator(".faq-list details[open]")).toContainText(
    "Normal SET",
  );
  await page
    .locator("#start")
    .getByRole("button", { name: "Copy command" })
    .first()
    .click();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(
    "cargo install vynk --locked",
  );
  await page.locator("footer").screenshot({ path: "test-results/footer.png" });
  await page.screenshot({
    path: "test-results/desktop-full.png",
    fullPage: true,
  });
  expect(errors).toEqual([]);
});

test("mobile layout and navigation work without horizontal overflow", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/", { waitUntil: "networkidle" });
  await page.screenshot({ path: "test-results/mobile-hero.png" });
  await page.getByRole("button", { name: "Menu" }).click();
  await page
    .getByRole("navigation")
    .getByRole("link", { name: "Playground" })
    .click();
  await expect(page.getByRole("button", { name: "Menu" })).toHaveAttribute(
    "aria-expanded",
    "false",
  );
  await page.getByRole("button", { name: "See result" }).click();
  await expect(
    page.locator(".admission .policy-metrics strong").first(),
  ).toHaveText("24");
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: "test-results/mobile-full.png",
    fullPage: true,
  });
  await page.setViewportSize({ width: 320, height: 740 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= window.innerWidth,
    ),
  ).toBe(true);
});

test("reduced motion keeps the page usable", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/");
  expect(
    await page
      .locator(".art-tile")
      .first()
      .evaluate((element) => getComputedStyle(element).animationName),
  ).toBe("none");
  await page.getByRole("button", { name: "See result" }).click();
  await expect(page.locator(".scenario-note")).toContainText("Trace complete");
});

test("FAQ answers animate open and closed", async ({ page }) => {
  await page.goto("/");
  const faq = page
    .locator(".faq-list details")
    .filter({ hasText: "Does admission control change SET?" });
  const summary = faq.locator("summary");

  await summary.click();
  await page.waitForTimeout(80);
  const opening = await faq.evaluate((element) => {
    const style = getComputedStyle(element, "::details-content");
    return {
      height: parseFloat(style.blockSize),
      opacity: Number(style.opacity),
    };
  });
  expect(opening.height).toBeGreaterThan(0);
  expect(opening.opacity).toBeGreaterThan(0);
  expect(
    await faq.evaluate(
      (element) =>
        getComputedStyle(element, "::details-content").transitionDuration,
    ),
  ).toContain("0.24s");

  await page.waitForTimeout(220);
  await summary.click();
  await page.waitForTimeout(80);
  const closing = await faq.evaluate((element) => {
    const style = getComputedStyle(element, "::details-content");
    return {
      height: parseFloat(style.blockSize),
      opacity: Number(style.opacity),
    };
  });
  expect(closing.height).toBeGreaterThan(0);
  expect(closing.opacity).toBeGreaterThan(0);
  expect(closing.opacity).toBeLessThan(1);

  await page.waitForTimeout(220);
  await expect(faq).not.toHaveAttribute("open", "");
});

test("copy uses the fallback when the Clipboard API fails", async ({
  page,
}) => {
  await page.addInitScript(() => {
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText: () => Promise.reject(new Error("unavailable")) },
    });
    document.execCommand = (command) => {
      document.documentElement.dataset.copyFallback = command;
      return command === "copy";
    };
  });
  await page.goto("/");
  await page
    .locator("#start")
    .getByRole("button", { name: "Copy command" })
    .first()
    .click();
  await expect(
    page.getByRole("button", { name: "Copied" }).first(),
  ).toBeVisible();
  await expect(page.locator("html")).toHaveAttribute(
    "data-copy-fallback",
    "copy",
  );
});

test("mobile text and footer targets meet accessibility thresholds", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");
  const audit = await page.evaluate(() => {
    const rgba = (value: string) => {
      const parts = value.match(/[\d.]+/g)?.map(Number);
      return parts && parts.length >= 3
        ? [parts[0], parts[1], parts[2], parts[3] ?? 1]
        : null;
    };
    const luminance = (rgb: number[]) => {
      const channels = rgb.slice(0, 3).map((value) => {
        const channel = value / 255;
        return channel <= 0.04045
          ? channel / 12.92
          : ((channel + 0.055) / 1.055) ** 2.4;
      });
      return channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722;
    };
    const contrast = (first: number[], second: number[]) => {
      const values = [luminance(first), luminance(second)].sort(
        (a, b) => b - a,
      );
      return (values[0] + 0.05) / (values[1] + 0.05);
    };
    const background = (element: Element) => {
      let current: Element | null = element;
      while (current) {
        const color = rgba(getComputedStyle(current).backgroundColor);
        if (color?.[3] === 1) return color;
        current = current.parentElement;
      }
      return [255, 255, 255, 1];
    };
    const visible = (element: Element) => {
      const style = getComputedStyle(element);
      const rect = element.getBoundingClientRect();
      return rect.width > 0 && rect.height > 0 && style.visibility !== "hidden";
    };
    const textFailures = [...document.querySelectorAll("body *")].flatMap(
      (element) => {
        const text = [...element.childNodes]
          .filter((node) => node.nodeType === Node.TEXT_NODE)
          .map((node) => node.textContent ?? "")
          .join(" ")
          .trim();
        if (
          !text ||
          !visible(element) ||
          element.closest('[aria-hidden="true"], [role="img"]')
        )
          return [];
        const style = getComputedStyle(element);
        const color = rgba(style.color);
        if (!color) return [];
        const size = Number.parseFloat(style.fontSize);
        const weight = Number.parseInt(style.fontWeight, 10) || 400;
        const required =
          size >= 24 || (size >= 18.66 && weight >= 700) ? 3 : 4.5;
        const ratio = contrast(color, background(element));
        return size < 12 || ratio + 0.01 < required
          ? [
              `${element.tagName.toLowerCase()}.${element.className} "${text.slice(0, 35)}": ${size}px, ${ratio.toFixed(2)}:1`,
            ]
          : [];
      },
    );
    const smallTargets = [...document.querySelectorAll("footer a")]
      .filter(visible)
      .flatMap((element) => {
        const rect = element.getBoundingClientRect();
        return rect.height < 24 ? [element.textContent?.trim() ?? "link"] : [];
      });
    return { textFailures, smallTargets };
  });
  expect(audit.textFailures).toEqual([]);
  expect(audit.smallTargets).toEqual([]);
});
