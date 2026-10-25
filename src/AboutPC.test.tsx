// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { beforeEach, afterEach, it, expect, vi } from "vitest";
import AboutPC, { hardwareValue, hardwareSections } from "./AboutPC";
const mocks = vi.hoisted(() => ({ invoke: vi.fn(), copy: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
let host: HTMLDivElement, root: Root;
beforeEach(() => {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  mocks.invoke.mockReset();
  mocks.copy.mockReset().mockResolvedValue(undefined);
  Object.defineProperty(navigator, "clipboard", {
    configurable: true,
    value: { writeText: mocks.copy },
  });
});
afterEach(async () => {
  await act(async () => root.unmount());
  host.remove();
});
it("only scans when the hardware page is opened and preserves the inventory between pages", async () => {
  mocks.invoke.mockResolvedValue({
    ok: true,
    data: {
      computer: { manufacturer: "Fixture", model: "PC" },
      windows: { name: "Windows fixture" },
      cpu: [{ name: "Fixture CPU", cores: 6 }],
      memory: [{ capacityBytes: 8589934592 }],
      unavailable: [],
      limitations: [],
    },
  });
  await act(async () => root.render(<AboutPC visible={false} />));
  expect(mocks.invoke).not.toHaveBeenCalled();
  await act(async () => root.render(<AboutPC visible />));
  expect(mocks.invoke).toHaveBeenCalledExactlyOnceWith("extended_action", {
    module: "hardware",
    action: "hardware-info",
    payload: {},
    confirmed: false,
  });
  expect(host.textContent).toContain("Fixture CPU");
  expect(host.textContent).toContain("8 Gio");
  await act(async () => root.render(<AboutPC visible={false} />));
  await act(async () => root.render(<AboutPC visible />));
  expect(mocks.invoke).toHaveBeenCalledTimes(1);
  const copy = [...host.querySelectorAll("button")].find((b) =>
    b.textContent?.includes("Copier le rapport"),
  )!;
  await act(async () => copy.click());
  expect(mocks.copy).toHaveBeenCalledWith(
    expect.stringContaining("Fixture CPU"),
  );
  expect(mocks.copy).toHaveBeenCalledWith(
    expect.stringContaining("Windows fixture"),
  );
});
it("distinguishes an unavailable provider from an absent component", async () => {
  mocks.invoke.mockResolvedValue({
    ok: true,
    data: { gpu: [], unavailable: ["Win32_VideoController"], limitations: [] },
  });
  await act(async () => root.render(<AboutPC visible />));
  expect(host.querySelector("#pc-gpu")?.textContent).toContain(
    "ne permettent pas de conclure à l’absence",
  );
  expect(
    hardwareSections({ gpu: [], unavailable: ["Win32_VideoController"] }).find(
      (s) => s.id === "gpu",
    )?.unavailable,
  ).toBe(true);
});
it("keeps unknown measurements distinct from false and empty batteries", () => {
  expect(hardwareValue(null, "bytes")).toBe("Non disponible");
  expect(hardwareValue(0, "positive")).toBe("Non disponible");
  expect(hardwareValue(false, "bool")).toBe("Non");
  expect(hardwareValue(0)).toBe("0");
  expect(
    hardwareSections({ batteries: [], unavailable: [] }).find(
      (s) => s.id === "batteries",
    )?.rows,
  ).toHaveLength(0);
});
