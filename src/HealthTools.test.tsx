// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { beforeEach, afterEach, it, expect, vi } from "vitest";
import { MemoryGuardian } from "./HealthTools";
const mocks = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: mocks.listen }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
let host: HTMLDivElement, root: Root;
let stopFromTray: () => void;
async function click(label: string) {
  const b = [...host.querySelectorAll("button")].find(
    (b) => b.textContent === label,
  );
  if (!b) throw new Error(`Missing ${label}`);
  await act(async () => b.click());
}
beforeEach(async () => {
  mocks.listen.mockImplementation(async (_event, handler) => {
    stopFromTray = handler;
    return () => {};
  });
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  HTMLDialogElement.prototype.showModal = function () {
    this.open = true;
  };
  HTMLDialogElement.prototype.close = function () {
    this.open = false;
  };
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  mocks.invoke.mockReset().mockResolvedValue({
    ok: true,
    data: {
      usedPercent: 80,
      freeBytes: 100,
      items: [
        {
          pid: 42,
          started: "123",
          name: "Fixture",
          manageable: true,
          workingSetBytes: 200e6,
          privateBytes: 300e6,
        },
      ],
    },
  });
  await act(async () => root.render(<MemoryGuardian visible />));
  await click("Mesurer la mémoire");
  mocks.invoke.mockClear();
});
afterEach(async () => {
  await act(async () => root.unmount());
  host.remove();
  vi.useRealTimers();
});
it("automatically trims only approved targets under sustained pressure and can pause from the tray", async () => {
  vi.useFakeTimers();
  const sample = {
    usedPercent: 95,
    freeBytes: 100,
    pageReadsPerSecond: 1,
    items: [
      {
        pid: 42,
        started: "123",
        name: "Fixture",
        manageable: true,
        workingSetBytes: 300 * 1024 ** 2,
        privateBytes: 400 * 1024 ** 2,
      },
    ],
  };
  mocks.invoke.mockImplementation(async (command, args) =>
    command === "set_tray_memory_status"
      ? {}
      : { ok: true, data: args.action === "memory-snapshot" ? sample : {} },
  );
  await click("Mesurer la mémoire");
  const label = [...host.querySelectorAll("label")].find((l) =>
    l.textContent?.includes("Fixture"),
  )!;
  await act(async () =>
    (label.querySelector("input") as HTMLInputElement).click(),
  );
  await click("Activer la surveillance automatique");
  await click("Confirmer");
  await act(async () => {
    await vi.advanceTimersByTimeAsync(60000);
  });
  const trims = mocks.invoke.mock.calls.filter(
    ([command, args]) =>
      command === "extended_action" && args.action === "memory-trim",
  );
  expect(trims).toHaveLength(1);
  expect(trims[0][1]).toMatchObject({
    payload: { pid: 42, started: "123" },
    confirmed: true,
  });
  expect(
    mocks.invoke.mock.calls.some(
      ([, args]) => args?.action === "process-close",
    ),
  ).toBe(false);
  await act(async () => stopFromTray());
  const count = mocks.invoke.mock.calls.length;
  await act(async () => {
    await vi.advanceTimersByTimeAsync(600000);
  });
  expect(mocks.invoke.mock.calls).toHaveLength(count);
  expect(host.textContent).toContain(
    "suspendue depuis la zone de notification",
  );
});
it("never requests closure on measurement, opening a dialog, or cancel", async () => {
  await click("Fermer");
  expect(host.querySelector("dialog")?.open).toBe(true);
  expect(mocks.invoke).not.toHaveBeenCalled();
  await click("Annuler");
  expect(mocks.invoke).not.toHaveBeenCalled();
});
it("conserve un sélecteur de tri accessible avec une présentation Nova", () => {
  const select = host.querySelector<HTMLSelectElement>("select")!;
  expect(select.classList.contains("nova-select")).toBe(true);
  expect(select.value).toBe("ram");
  expect([...select.options].map((option) => option.value)).toEqual([
    "ram",
    "private",
    "name",
  ]);
});
it("only closes the explicitly confirmed PID and reports pending application confirmation", async () => {
  mocks.invoke.mockResolvedValue({
    ok: true,
    data: { closeRequested: true, exited: false },
  });
  await click("Fermer");
  await click("Confirmer");
  expect(mocks.invoke).toHaveBeenCalledExactlyOnceWith("extended_action", {
    module: "memory",
    action: "process-close",
    payload: { pid: 42, started: "123" },
    confirmed: true,
  });
  expect(host.textContent).toContain(
    "une confirmation ou un enregistrement peut être nécessaire",
  );
});
it("keeps protected processes unavailable for closure", async () => {
  mocks.invoke.mockResolvedValue({
    ok: true,
    data: {
      usedPercent: 80,
      freeBytes: 100,
      items: [
        {
          pid: 4,
          started: "123",
          name: "Protected fixture",
          manageable: false,
          workingSetBytes: 200e6,
          privateBytes: 300e6,
        },
      ],
    },
  });
  await click("Mesurer la mémoire");
  const close = [...host.querySelectorAll("button")].find(
    (b) => b.textContent === "Fermer",
  );
  expect(close?.disabled).toBe(true);
  mocks.invoke.mockClear();
  await act(async () => close?.click());
  expect(mocks.invoke).not.toHaveBeenCalled();
});
it("reports a refused closure without claiming that the application exited", async () => {
  mocks.invoke.mockResolvedValue({
    ok: false,
    error: "ProcessHasNoClosableWindow",
  });
  await click("Fermer");
  await click("Confirmer");
  expect(host.querySelector('[role="alert"]')?.textContent).toContain(
    "Aucun arrêt forcé",
  );
  expect(host.textContent).not.toContain("Fixture a quitté");
});
