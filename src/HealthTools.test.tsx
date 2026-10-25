// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { beforeEach, afterEach, it, expect, vi } from "vitest";
import { MemoryGuardian } from "./HealthTools";
const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
let host: HTMLDivElement, root: Root;
async function click(label: string) {
  const b = [...host.querySelectorAll("button")].find(
    (b) => b.textContent === label,
  );
  if (!b) throw new Error(`Missing ${label}`);
  await act(async () => b.click());
}
beforeEach(async () => {
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
  mocks.invoke
    .mockReset()
    .mockResolvedValue({
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
});
it("never requests closure on measurement, opening a dialog, or cancel", async () => {
  await click("Fermer");
  expect(host.querySelector("dialog")?.open).toBe(true);
  expect(mocks.invoke).not.toHaveBeenCalled();
  await click("Annuler");
  expect(mocks.invoke).not.toHaveBeenCalled();
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
