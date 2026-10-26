// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";

const mocks = vi.hoisted(() => ({ action: vi.fn(), history: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => false, invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));
vi.mock("./AutomaticCleanup", () => ({ default: () => <div>Nettoyage automatique</div> }));
vi.mock("./AboutPC", () => ({ default: () => <div>FICHE_MATERIELLE_MOCK</div> }));
vi.mock("./HealthTools", () => ({
  MemoryGuardian: () => <div>Mémoire</div>,
  NetworkTools: () => <div>Réseau</div>,
}));
vi.mock("./Workbench", () => ({ default: () => <div>Atelier</div> }));
vi.mock("./lib", () => ({
  action: mocks.action,
  getHistory: mocks.history,
  labels: { snapshot: "Analyse" },
  rows: () => [],
  text: (value: unknown) => String(value ?? ""),
  bytes: () => "—",
  memoryPercent: () => null,
  message: (value: unknown) => String(value ?? ""),
  recommendations: () => [],
  healthScore: () => ({ score: 100, level: "Bon état", items: [] }),
  healthReport: () => ({ text: "Bilan Nova", json: "{}" }),
  driverCategory: () => "Périphérique",
  elevate: vi.fn(),
  openSettings: vi.fn(),
}));

let host: HTMLDivElement;
let root: Root;

beforeEach(() => {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  HTMLDialogElement.prototype.close = function () {
    this.open = false;
  };
  HTMLDialogElement.prototype.showModal = function () {
    this.open = true;
  };
  localStorage.clear();
  mocks.action.mockReset();
  mocks.history.mockReset();
  mocks.action.mockResolvedValue({ ok: true, data: {} });
  mocks.history.mockResolvedValue([]);
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
});
afterEach(async () => {
  await act(async () => root.unmount());
  host.remove();
});

describe("raccourcis d'interface", () => {
  it("restaure la dernière page et la conserve lors de la navigation", async () => {
    localStorage.setItem("nova-last-page", "cleanup");
    await act(async () => root.render(<App />));
    expect(host.textContent).toContain("Nettoyage automatique");
    const overview = [...host.querySelectorAll("button")].find((button) =>
      button.textContent?.includes("Vue d’ensemble"),
    )!;
    await act(async () => overview.click());
    expect(localStorage.getItem("nova-last-page")).toBe("overview");
  });

  it("ne monte la fiche matérielle que lorsque sa page est ouverte", async () => {
    await act(async () => root.render(<App />));
    expect(host.textContent).not.toContain("FICHE_MATERIELLE_MOCK");
    const about = [...host.querySelectorAll("button")].find((button) =>
      button.textContent?.includes("À propos de ce PC"),
    )!;
    await act(async () => about.click());
    expect(host.textContent).toContain("FICHE_MATERIELLE_MOCK");
  });

  it("offre un lien d’évitement vers le contenu principal", async () => {
    await act(async () => root.render(<App />));
    const skipLink = host.querySelector<HTMLAnchorElement>("a.skip-link")!;
    expect(skipLink.textContent).toContain("Aller au contenu principal");
    expect(skipLink.getAttribute("href")).toBe("#main-content");
    expect(host.querySelector("main#main-content")).not.toBeNull();
  });

  it("ouvre la recherche avec Ctrl+K et l'efface avec Échap", async () => {
    await act(async () => root.render(<App />));
    const input = host.querySelector<HTMLInputElement>("input[aria-label^='Rechercher']")!;
    await act(async () =>
      window.dispatchEvent(new KeyboardEvent("keydown", { key: "k", ctrlKey: true })),
    );
    expect(document.activeElement).toBe(input);
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(
        input,
        "pilotes",
      );
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await act(async () => window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" })));
    expect(input.value).toBe("");
    expect(document.activeElement).not.toBe(input);
  });

  it("permet de choisir un résultat de recherche au clavier", async () => {
    await act(async () => root.render(<App />));
    const input = host.querySelector<HTMLInputElement>("input[aria-label^='Rechercher']")!;
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(
        input,
        "pilotes",
      );
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
    await act(async () =>
      input.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true })),
    );
    expect(host.querySelector("[role='option'][aria-selected='true']")?.textContent).toContain(
      "Pilotes & mises à jour",
    );
    await act(async () =>
      input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true })),
    );
    expect(input.value).toBe("");
    expect(host.textContent).toContain("Pilotes & mises à jour");
  });

  it("permet de replier un groupe sans retirer ses pages de la recherche", async () => {
    await act(async () => root.render(<App />));
    const maintenance = [...host.querySelectorAll("button")].find((button) =>
      button.textContent?.trim() === "Entretien",
    )!;
    expect(maintenance.getAttribute("aria-expanded")).toBe("true");
    await act(async () => maintenance.click());
    expect(maintenance.getAttribute("aria-expanded")).toBe("false");
    expect(localStorage.getItem("nova-collapsed-nav")).toBe('["maintenance"]');
    expect(host.textContent).not.toContain("Pilotes & mises à jour");
    const input = host.querySelector<HTMLInputElement>("input[aria-label^='Rechercher']")!;
    await act(async () => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(
        input,
        "pilotes",
      );
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
    expect(host.textContent).toContain("Pilotes & mises à jour");
    const driverResult = [...host.querySelectorAll(".search-results button")].find(
      (button) => button.textContent?.includes("Pilotes & mises à jour"),
    )!;
    await act(async () => (driverResult as HTMLButtonElement).click());
    expect(maintenance.getAttribute("aria-expanded")).toBe("true");
    expect(localStorage.getItem("nova-collapsed-nav")).toBe("[]");
  });

  it("présente des suggestions sans lancer de maintenance automatiquement", async () => {
    await act(async () => root.render(<App />));
    const suggestions = [...host.querySelectorAll("button")].find((button) =>
      button.textContent?.includes("Suggestions Nova"),
    )!;
    await act(async () => suggestions.click());
    expect(host.textContent).toContain("Suggestions pour votre PC");
    expect(host.textContent).toContain("Gérer les logiciels");
    expect(host.textContent).toContain("Parcours conseillé");
    expect(host.textContent).toContain("1 / 4 vérifications effectuées");
    expect(mocks.action).toHaveBeenCalledTimes(1);
  });

  it("oriente la recherche de pilotes vers les fabricants détectés", async () => {
    await act(async () => root.render(<App />));
    const drivers = [...host.querySelectorAll("button")].find((button) =>
      button.textContent?.includes("Pilotes & mises à jour"),
    )!;
    await act(async () => drivers.click());
    const sources = [...host.querySelectorAll("button")].find((button) =>
      button.textContent?.includes("Fabricants détectés"),
    )!;
    await act(async () => sources.click());
    expect(host.textContent).toContain("Atelier");
  });

  it("lance un audit de pilotes en lecture seule à la demande", async () => {
    await act(async () => root.render(<App />));
    const drivers = [...host.querySelectorAll("button")].find((button) =>
      button.textContent?.includes("Pilotes & mises à jour"),
    )!;
    await act(async () => drivers.click());
    const audit = [...host.querySelectorAll("button")].find((button) =>
      button.textContent?.includes("Auditer les appareils"),
    )!;
    await act(async () => audit.click());
    expect(mocks.action).toHaveBeenLastCalledWith("driver-audit", false, [], false);
  });

  it("affiche une attente explicite pour les mesures non encore analysées", async () => {
    await act(async () => root.render(<App />));
    expect(host.textContent).toContain("Analyse requise");
    expect(host.textContent).toContain("Inventaire requis");
    expect(host.textContent).not.toContain("Stockage—");
  });

  it("affiche un bilan local sans lancer d’action de maintenance", async () => {
    await act(async () => root.render(<App />));
    expect(host.textContent).toContain("BILAN LOCAL");
    expect(host.textContent).toContain("Bon état");
    expect(mocks.action).toHaveBeenCalledTimes(1);
  });

  it("permet d’effacer uniquement l’historique local des scores", async () => {
    localStorage.setItem(
      "nova-health-history",
      JSON.stringify([{ score: 72, at: 1 }, { score: 80, at: 2 }]),
    );
    await act(async () => root.render(<App />));
    const clear = [...host.querySelectorAll("button")].find((button) =>
      button.textContent?.includes("Effacer l’historique"),
    )!;
    await act(async () => clear.click());
    expect(localStorage.getItem("nova-health-history")).toBeNull();
    expect(host.textContent).not.toContain("Évolution des derniers bilans");
  });
});
