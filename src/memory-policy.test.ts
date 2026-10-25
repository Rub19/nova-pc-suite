import { describe, it, expect } from "vitest";
import {
  memoryDecision,
  type MemorySample,
  type MemoryTarget,
} from "./memory-policy";
const target: MemoryTarget = {
  pid: 42,
  name: "Fixture",
  started: "1234",
  workingSetBytes: 400 * 1024 ** 2,
  privateBytes: 600 * 1024 ** 2,
  manageable: true,
};
const sample: MemorySample = {
  usedPercent: 95,
  freeBytes: 100,
  totalBytes: 2000,
  pageReadsPerSecond: 0,
  items: [target],
};
describe("entretien mémoire opt-in", () => {
  it("exige une pression soutenue et une application autorisée", () => {
    expect(memoryDecision(sample, [], 3, 700000)).toBeNull();
    expect(memoryDecision(sample, [target], 2, 700000)).toBeNull();
    expect(memoryDecision(sample, [target], 3, 700000)).toEqual(target);
  });
  it("refuse le PID recyclé et les processus protégés", () => {
    expect(
      memoryDecision(
        { ...sample, items: [{ ...target, started: "different" }] },
        [target],
        3,
        700000,
      ),
    ).toBeNull();
    expect(
      memoryDecision(
        { ...sample, items: [{ ...target, manageable: false }] },
        [target],
        3,
        700000,
      ),
    ).toBeNull();
  });
  it("respecte le délai et évite la pression de pagination", () => {
    expect(memoryDecision(sample, [target], 3, 10)).toBeNull();
    expect(
      memoryDecision(
        { ...sample, pageReadsPerSecond: 99 },
        [target],
        3,
        700000,
      ),
    ).toBeNull();
    expect(
      memoryDecision(
        { ...sample, pageReadsPerSecond: null },
        [target],
        3,
        700000,
      ),
    ).toBeNull();
    expect(
      memoryDecision({ ...sample, usedPercent: 30 }, [target], 3, 700000),
    ).toBeNull();
  });
});
