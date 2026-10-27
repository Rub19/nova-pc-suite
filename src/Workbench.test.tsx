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
  it("détecte les fabricants à l’ouverture sans lancer de téléchargement", async () => {
    mocks.invoke.mockResolvedValue({
      ok: true,
      data: {
        providers: [
          { id: "intel", name: "Intel", devices: ["Fixture chipset"] },
        ],
        otherManufacturers: [
          { manufacturer: "Other fixture", devices: ["Fixture device"] },
        ],
        firmwareTargets: [
          { kind: "BIOS / UEFI", manufacturer: "Fixture", model: "F1" },
        ],
      },
    });
    await act(async () => root.render(<Workbench section="driver-sources" />));
    expect(mocks.invoke).toHaveBeenCalledExactlyOnceWith("extended_action", {
      module: "automation",
      action: "detect-sources",
      payload: {},
      confirmed: false,
    });
    expect(host.textContent).toContain("Fixture chipset");
    expect(host.textContent).toContain("Other fixture");
    expect(host.textContent).toContain("Firmware détecté");
    expect(host.textContent).toContain("BIOS / UEFI");
    expect(
      mocks.invoke.mock.calls.some(([, args]) => args.action === "open-source"),
    ).toBe(false);
  });
  it("affiche les lenteurs de démarrage identifiées dans un tableau lisible", async () => {
    mocks.invoke.mockResolvedValue({
      ok: true,
      data: {
        items: [
          {
            name: "Fixture startup",
            eventId: 101,
            durationMs: 1234,
            degradationMs: 567,
            timestampUtc: "2026-10-25T12:00:00Z",
          },
        ],
        count: 1,
        interpretation: "Événements Windows récents",
      },
    });
    await act(async () => root.render(<Workbench section="maintenance" />));
    const heading = [...host.querySelectorAll("h3")].find(
      (h) => h.textContent === "Repérer les lenteurs au démarrage",
    )!;
    await act(async () =>
      (
        heading.closest(".wb-row")!.querySelector("button") as HTMLButtonElement
      ).click(),
    );
    expect(mocks.invoke).toHaveBeenCalledWith(
      "extended_action",
      expect.objectContaining({ action: "boot-slowdowns", confirmed: false }),
    );
    expect(host.querySelector("table")?.textContent).toContain(
      "Fixture startup",
    );
    expect(
      host.querySelector("table")?.textContent?.replace(/\s/g, ""),
    ).toContain("1234ms");
    expect(host.querySelector("table")?.textContent).toContain("Application");
  });
  it("permet de filtrer le stockage et exige une confirmation pour l’optimiser", async () => {
    mocks.invoke.mockResolvedValue({
      ok: true,
      data: { completed: true, readOnly: false },
    });
    await act(async () => root.render(<Workbench section="maintenance" />));
    const category = host.querySelector("select")!;
    expect([...category.options].some((o) => o.value === "Stockage")).toBe(
      true,
    );
    await act(async () => {
      category.value = "Stockage";
      category.dispatchEvent(new Event("change", { bubbles: true }));
    });
    const heading = [...host.querySelectorAll("h3")].find(
      (h) => h.textContent === "Optimiser le disque avec Windows",
    )!;
    await act(async () =>
      (
        heading.closest(".wb-row")!.querySelector("button") as HTMLButtonElement
      ).click(),
    );
    expect(mocks.invoke).not.toHaveBeenCalled();
    await click("Confirmer");
    expect(mocks.invoke).toHaveBeenCalledWith(
      "extended_action",
      expect.objectContaining({
        module: "optimize",
        action: "storage-optimize",
        confirmed: true,
      }),
    );
  });
  it("désactive une entrée de démarrage uniquement après confirmation", async () => {
    mocks.invoke.mockResolvedValue({
      ok: true,
      data: {
        items: [
          {
            id: "fixture-id",
            name: "Fixture startup",
            enabled: true,
            manageable: true,
          },
        ],
      },
    });
    await act(async () => root.render(<Workbench section="startup-manage" />));
    await click("Actualiser le démarrage");
    mocks.invoke.mockClear();
    await click("Désactiver");
    expect(mocks.invoke).not.toHaveBeenCalled();
    mocks.invoke.mockResolvedValue({ ok: true, data: {} });
    await click("Confirmer");
    expect(mocks.invoke).toHaveBeenCalledWith(
      "extended_action",
      expect.objectContaining({
        module: "apps",
        action: "startup-disable",
        payload: { id: "fixture-id" },
        confirmed: true,
      }),
    );
  });
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
  it("rend visible le bilan WinGet après une recherche de logiciels", async () => {
    mocks.invoke.mockResolvedValue({
      ok: true,
      data: {
        available: true,
        exitCode: 0,
        output: "Fixture update",
        truncated: false,
      },
    });
    await act(async () => root.render(<Workbench section="applications" />));
    await click("Rechercher les mises à jour");
    expect(host.textContent).toContain("Dernière recherche WinGet");
    expect(host.textContent).toContain("Fixture update");
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
