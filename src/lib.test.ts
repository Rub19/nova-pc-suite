import { describe, expect, it } from "vitest";
import { memoryPercent, recommendations, rows, message } from "./lib";
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
});
