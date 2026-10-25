export type MemoryTarget = {
  pid: number;
  name: string;
  started: string;
  workingSetBytes: number;
  privateBytes: number;
  manageable: boolean;
};
export type MemorySample = {
  usedPercent: number;
  freeBytes: number;
  totalBytes: number;
  pageReadsPerSecond: number | null;
  items: MemoryTarget[];
};
export function memoryDecision(
  sample: MemorySample,
  approved: MemoryTarget[],
  consecutive: number,
  sinceLastMs: number,
): MemoryTarget | null {
  if (
    !Number.isFinite(sample.usedPercent) ||
    sample.usedPercent < 90 ||
    consecutive < 3 ||
    sinceLastMs < 600_000 ||
    sample.pageReadsPerSecond === null ||
    !Number.isFinite(sample.pageReadsPerSecond) ||
    sample.pageReadsPerSecond > 2
  )
    return null;
  return (
    sample.items
      .filter(
        (t) =>
          t.manageable &&
          t.workingSetBytes > 200 * 1024 ** 2 &&
          approved.some((a) => a.pid === t.pid && a.started === t.started),
      )
      .sort((a, b) => b.workingSetBytes - a.workingSetBytes)[0] ?? null
  );
}
