// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";

const mocks = vi.hoisted(() => ({ action: vi.fn(), history: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => false, invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));
vi.mock("./AutomaticCleanup", () => ({ default: () => <div>Nettoyage automatique</div> }));
vi.mock("./AboutPC", () => ({ default: () => <div>À propos</div> }));
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
});
