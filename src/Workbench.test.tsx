// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { beforeEach, afterEach, describe, it, expect, vi } from "vitest";
import Workbench from "./Workbench";
const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: mocks.invoke,
  isTauri: () => true,
}));
let host: HTMLDivElement, root: Root;
function button(text: string) {
  const found = [...host.querySelectorAll("button")].find((b) =>
    b.textContent?.includes(text),
  );
  if (!found) throw new Error("Button missing: " + text);
  return found;
}
async function click(text: string) {
  await act(async () => button(text).click());
}
beforeEach(() => {
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
  mocks.invoke.mockReset();
});
afterEach(async () => {
  await act(async () => root.unmount());
  host.remove();
});
describe("confirmations et résultats de maintenance", () => {
  it("n’applique pas de réglage avant la confirmation", async () => {
    mocks.invoke.mockResolvedValue({ ok: true, data: { enabled: true } });
    await act(async () => root.render(<Workbench section="automation" />));
    await click("Exécuter");
    expect(mocks.invoke).toHaveBeenCalledWith(
      "extended_action",
      expect.objectContaining({ action: "schedule-status", confirmed: false }),
    );
    mocks.invoke.mockClear();
    const buttons = [...host.querySelectorAll("button")].filter(
      (b) => b.textContent === "Exécuter",
    );
    await act(async () => buttons[1].click());
    expect(mocks.invoke).not.toHaveBeenCalled();
    await click("Annuler");
    expect(mocks.invoke).not.toHaveBeenCalled();
  });
  it("conserve les résultats partiels quand Windows retourne une erreur", async () => {
    mocks.invoke.mockResolvedValue({
      ok: false,
      error: "NativeCommandFailed",
      data: { exitCode: 87 },
    });
    await act(async () => root.render(<Workbench section="maintenance" />));
    await click("Exécuter");
    expect(host.textContent).toContain("87");
    expect(host.textContent).toContain("Windows a signalé une erreur");
  });
  it("exige un consentement distinct avant la purge irréversible", async () => {
    mocks.invoke.mockResolvedValue({
      ok: true,
      data: { batches: [{ id: "abc", totalBytes: 100, count: 1 }] },
    });
    await act(async () => root.render(<Workbench section="cleanup" />));
    await click("Actualiser les lots");
    mocks.invoke.mockClear();
    await click("Supprimer");
    expect(button("Confirmer").disabled).toBe(true);
    expect(mocks.invoke).not.toHaveBeenCalled();
    await click("Annuler");
    expect(mocks.invoke).not.toHaveBeenCalled();
  });
});
