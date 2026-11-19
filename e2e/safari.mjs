import assert from "node:assert/strict";
import { after, before, test } from "node:test";
import { Builder, By, until } from "selenium-webdriver";
import safari from "selenium-webdriver/safari.js";
import { createServer } from "vite";

let server, service, driver, base;
before(async () => {
  server = await createServer({ server: { host: "127.0.0.1", port: 0 } });
  await server.listen();
  base = server.resolvedUrls.local[0];
  service = new safari.ServiceBuilder("/usr/bin/safaridriver").build();
  driver = await new Builder().disableEnvironmentOverrides().usingServer(await service.start()).forBrowser("safari").build();
});
after(async () => {
  try { await driver?.quit(); } finally {
    try { await service?.kill(); } finally { await server?.close(); }
  }
});

async function visible(xpath) {
  const element = await driver.wait(until.elementLocated(By.xpath(xpath)), 10000);
  return driver.wait(until.elementIsVisible(element), 10000);
}
function heading(name) {
  return visible(`//*[self::h1 or self::h2 or self::h3][normalize-space(.)='${name}']`);
}
test("Dashboard and app name are visible", async () => {
  await driver.get(base);
  await visible("//body//*[normalize-space(.)='AtlasForge']");
  await heading("Dashboard");
});
for (const page of ["Assets", "Settings"]) {
  test(`Navigate to ${page}`, async () => {
    await driver.get(base);
    await (await visible(`//a[normalize-space(.)='${page}']`)).click();
    await driver.wait(until.urlContains(page.toLowerCase()), 10000);
    await heading(page);
  });
}
test("App shell renders a nonempty body", async () => {
  await driver.get(base);
  assert.ok(await driver.executeScript("return document.body.childNodes.length"));
});
test("Automations exposes only implemented scheduled notifications", async () => {
  await driver.get(base + "#/automations");
  await heading("Automations");
  await (await visible("//button[normalize-space(.)='New Rule']")).click();
  for (const [label, value] of [["Trigger", "Schedule"], ["Action", "Notification"], ["Interval (minutes)", "60"]]) {
    const element = await visible(`//label[normalize-space(.)='${label}']`);
    const id = await element.getAttribute("for");
    assert.equal(await (await driver.findElement(By.id(id))).getAttribute("value"), value);
  }
  assert.equal((await driver.findElements(By.xpath("//*[contains(text(),'Auto-apply fixes when possible')]"))).length, 0);
});
