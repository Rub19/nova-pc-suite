import { describe, expect, it } from "vitest";
import { driverCategory, healthScore, memoryPercent, recommendations, rows, message } from "./lib";
describe("diagnostics réels", () => {
  it("ne transforme pas une donnée absente en mesure rassurante", () => {
    expect(memoryPercent()).toBeNull();
    expect(
      memoryPercent({ memory: { totalBytes: 0, freeBytes: 0 } }),
    ).toBeNull();
    expect(rows({ items: null })).toEqual([]);
  });
  it("recommande une action lorsque la mémoire ou le stockage sont sous pression", () => {
    expect(
      recommendations(
        { memory: { totalBytes: 100, freeBytes: 10 } },
        { volumes: [{ drive: "C:", sizeBytes: 100, freeBytes: 5 }] },
      ).map((x) => x.target),
    ).toEqual(["processes", "storage"]);
  });
  it("ne recommande pas de libérer la mémoire déjà disponible", () => {
    expect(
      recommendations(
        { memory: { totalBytes: 100, freeBytes: 75 } },
        { volumes: [{ sizeBytes: 100, freeBytes: 60 }] },
      ),
    ).toEqual([]);
  });
  it("explique une installation partiellement échouée", () => {
    expect(message("DriverInstallationIncomplete")).toContain(
      "Certains pilotes",
    );
  });
  it("produit un bilan local explicable et borné", () => {
    const health = healthScore(
      { memory: { totalBytes: 100, freeBytes: 5 }, pendingReboot: true },
      { volumes: [{ drive: "C:", sizeBytes: 100, freeBytes: 5 }] },
      { issues: ["AntivirusInventoryUnavailable"] },
      3,
    );
    expect(health.score).toBeGreaterThanOrEqual(0);
    expect(health.score).toBeLessThan(100);
    expect(health.level).toBe("À vérifier");
    expect(health.items.map((item) => item.target)).toEqual([
      "processes",
      "storage",
      "drivers",
      "drivers",
      "security",
    ]);
  });
  it("classe les pilotes sans dépendre d’un fabricant précis", () => {
    expect(driverCategory({ name: "NVIDIA GeForce RTX" })).toBe("Graphique");
    expect(driverCategory({ name: "Realtek PCIe Ethernet" })).toBe("Réseau");
    expect(driverCategory({ name: "USB Audio Device" })).toBe("Audio");
    expect(driverCategory({ name: "AMD SATA Controller" })).toBe("Stockage");
    expect(driverCategory({ name: "Unknown peripheral" })).toBe("Périphérique");
  });
});
