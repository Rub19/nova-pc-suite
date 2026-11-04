import { describe, expect, it } from "vitest";
import { arr, bySizeThenUsable, compareFindings, journalDay, bootComparison, findings, formatBytes, formatMeasureValue, headline, measureDelta, Scan } from "./lib";

describe("repères ajoutés", () => {
  it("rend toujours une liste, même si le moteur renvoie autre chose", () => {
    expect(arr(undefined)).toEqual([]);
    expect(arr(null)).toEqual([]);
    expect(arr({ a: 1 })).toEqual([{ a: 1 }]);
    expect(arr([1, 2])).toEqual([1, 2]);
  });
  const finding = (id: string, level: "ok" | "warn" | "alert") => ({ id, level, title: id, detail: "" }) as never;
  it("compare deux analyses sans inventer de nouveauté au premier passage", () => {
    expect(compareFindings(null, [finding("a", "alert")])).toEqual({ fresh: [], solved: 0 });
    expect(compareFindings(["a", "b"], [finding("a", "alert"), finding("c", "warn"), finding("d", "ok")])).toEqual({ fresh: ["c"], solved: 1 });
  });
  it("met les plus gros nettoyages en tête et les indisponibles à la fin", () => {
    const row = (bytes: number, needsAdmin = false) => ({ bytes, needsAdmin, blocked: false });
    expect(bySizeThenUsable([row(5), row(100, true), row(0), row(50)]).map((c) => c.bytes)).toEqual([50, 5, 100, 0]);
  });
  it("écrit le jour du journal en français", () => {
    expect(journalDay(Date.UTC(2026, 10, 4, 12) / 1000)).toContain("novembre");
  });
});

const healthy: Scan = {
  isAdmin: true,
  timestamp: "2026-11-03T10:00:00Z",
  os: { name: "Windows 11", release: "26H2", build: "26300" },
  computer: { manufacturer: "X", model: "Y", board: "Z", bios: "1" },
  cpu: { name: "CPU", cores: 8, threads: 16 },
  memory: { totalBytes: 100, freeBytes: 60, modules: 2, speedMHz: 6000 },
  gpus: [],
  uptimeSeconds: 3600,
  pendingReboot: false,
  systemDrive: "C:",
  volumes: [{ drive: "C:", label: "", sizeBytes: 1000, freeBytes: 500 }],
  disks: [{ name: "SSD", media: "SSD", bus: "NVMe", sizeBytes: 1000, health: "Healthy", temperature: 40, wear: 2, errors: 0 }],
  deviceProblems: 0,
  security: { defender: { running: true, realtime: true, signatureAgeDays: 0 }, otherAntivirus: [], firewallOff: [] },
  stability: { unexpectedShutdowns: 0, blueScreens: 0, gpuHangs: 0, gpuCodes: [], appCrashes: 0, topCrashes: [], lastIncident: null },
  lastUpdate: new Date().toISOString(),
  boots: { available: true, items: [{ date: "2026-11-03T09:00:00Z", ms: 20000 }] },
};
const issues = (scan: Scan) => findings(scan).filter((f) => f.level !== "ok");

describe("findings", () => {
  it("reports nothing on a healthy PC", () => {
    expect(issues(healthy)).toEqual([]);
    expect(headline(findings(healthy))).toBe("Rien à signaler sur ce PC.");
  });
  it("raises space alerts by remaining ratio", () => {
    const low = { ...healthy, volumes: [{ drive: "C:", label: "", sizeBytes: 1000, freeBytes: 50 }] };
    expect(issues(low)[0]).toMatchObject({ id: "space", level: "alert", page: "nettoyage" });
    const mid = { ...healthy, volumes: [{ drive: "C:", label: "", sizeBytes: 1000, freeBytes: 150 }] };
    expect(issues(mid)[0]).toMatchObject({ id: "space", level: "warn" });
  });
  it("names the frequent GPU code and sorts alerts first", () => {
    const scan = {
      ...healthy,
      pendingReboot: true,
      stability: { ...healthy.stability, gpuHangs: 6, gpuCodes: [{ code: "1b8", count: 6 }] },
    };
    const list = issues(scan);
    expect(list[0].id).toBe("gpu");
    expect(list[0].detail).toContain("écran noir");
    expect(list[1].id).toBe("reboot");
    expect(headline(findings(scan))).toBe("2 points à traiter sur ce PC.");
  });
  it("does not warn about Defender when another antivirus runs", () => {
    const scan = { ...healthy, security: { defender: { running: true, realtime: false, signatureAgeDays: 90 }, otherAntivirus: ["Malwarebytes"], firewallOff: [] } };
    expect(issues(scan)).toEqual([]);
    const none = { ...healthy, security: { defender: null, otherAntivirus: [], firewallOff: [] } };
    expect(issues(none)[0]).toMatchObject({ id: "av", level: "alert" });
  });
  it("does not invent facts when the disk list is empty", () => {
    const scan = { ...healthy, disks: [] };
    expect(findings(scan).some((f) => f.id === "disk")).toBe(false);
  });
});

describe("formats", () => {
  it("formats bytes in French", () => {
    expect(formatBytes(1536 * 1024 * 1024)).toBe("1,5 Go");
    expect(formatBytes(null)).toBe("inconnu");
  });
  it("computes real deltas only", () => {
    expect(measureDelta({ label: "x", before: 10, after: 7, unit: "count" })).toBe("−3");
    expect(measureDelta({ label: "x", before: 5e9, after: 5e9, unit: "bytes" })).toBe("sans changement");
    expect(measureDelta({ label: "x", before: "a", after: "b", unit: "text" })).toBeNull();
    expect(formatMeasureValue(null, "bytes")).toBe("inconnu");
  });
  it("compares boots around the last change only when both sides exist", () => {
    const boots = [
      { date: "2026-11-03T10:00:00Z", ms: 20000 },
      { date: "2026-11-01T10:00:00Z", ms: 40000 },
    ];
    const change = [{ timestamp: Date.parse("2026-11-02T10:00:00Z") / 1000 }];
    expect(bootComparison(boots, change)).toEqual({ before: 40000, after: 20000 });
    expect(bootComparison(boots, [])).toBeNull();
    expect(bootComparison(boots.slice(1), change)).toBeNull();
  });
});
