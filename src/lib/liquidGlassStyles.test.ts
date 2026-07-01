import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const styles = readFileSync(new URL("../styles.css", import.meta.url), "utf8");

function cssRule(selector: string) {
  const start = styles.indexOf(selector);
  expect(start).toBeGreaterThanOrEqual(0);
  const end = styles.indexOf("\n}", start);
  expect(end).toBeGreaterThan(start);
  return styles.slice(start, end);
}

describe("liquid glass style contracts", () => {
  it("keeps select menus readable over refracted backgrounds", () => {
    const contentRule = cssRule(':root[data-app-theme="liquid-glass-light"] .select-content');
    const itemRule = cssRule(':root[data-app-theme="liquid-glass-light"] .select-item {');
    const checkedRule = cssRule(
      ':root[data-app-theme="liquid-glass-light"] .select-item[data-state="checked"]',
    );

    expect(contentRule).toContain("rgba(255, 255, 255, 0.92)");
    expect(contentRule).toContain("backdrop-filter: blur(26px)");
    expect(itemRule).toContain("#111614");
    expect(checkedRule).toContain("rgba(207, 255, 53, 0.28)");
  });
});
