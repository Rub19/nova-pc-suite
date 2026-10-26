// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { beforeEach, afterEach, it, expect, vi } from "vitest";
import AutomaticCleanup from "./AutomaticCleanup";
const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
let host: HTMLDivElement, root: Root;
beforeEach(() => {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  localStorage.clear();
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  mocks.invoke
    .mockReset()
    .mockResolvedValue({
      ok: true,
      data: { deletedCount: 2, preservedCount: 0, freedBytes: 1024 },
    });
});
afterEach(async () => {
  await act(async () => root.unmount());
  host.remove();
  localStorage.clear();
  vi.useRealTimers();
});
it("never cleans before the user enables the option, then runs hourly without dialogs", async () => {
  vi.useFakeTimers();
  await act(async () => root.render(<AutomaticCleanup visible />));
  expect(mocks.invoke).not.toHaveBeenCalled();
  await act(async () => host.querySelector("input")!.click());
  expect(mocks.invoke).toHaveBeenCalledExactlyOnceWith("extended_action", {
    module: "cleanup",
    action: "auto-clean",
    payload: {},
    confirmed: true,
  });
  expect(localStorage.getItem("nova-auto-temp-cleanup")).toBe("enabled");
  expect(host.querySelector("dialog")).toBeNull();
  await act(async () => {
    await vi.advanceTimersByTimeAsync(3600000);
  });
  expect(mocks.invoke).toHaveBeenCalledTimes(2);
  await act(async () => host.querySelector("input")!.click());
  await act(async () => {
    await vi.advanceTimersByTimeAsync(7200000);
  });
  expect(mocks.invoke).toHaveBeenCalledTimes(2);
  expect(localStorage.getItem("nova-auto-temp-cleanup")).toBe("disabled");
});
it("resumes the saved opt-in while navigating other pages", async () => {
  localStorage.setItem("nova-auto-temp-cleanup", "enabled");
  await act(async () => root.render(<AutomaticCleanup visible={false} />));
  expect(mocks.invoke).toHaveBeenCalledTimes(1);
  expect(host.querySelector("section")?.hidden).toBe(true);
});
it("keeps partial results visible and never presents failure as successful cleanup", async () => {
  mocks.invoke.mockResolvedValue({
    ok: false,
    data: { deletedCount: 1, preservedCount: 1, freedBytes: 512 },
    error: "Fixture preserved",
  });
  localStorage.setItem("nova-auto-temp-cleanup", "enabled");
  await act(async () => root.render(<AutomaticCleanup visible />));
  expect(host.querySelector('[role="alert"]')?.textContent).toContain(
    "Fixture preserved",
  );
  expect(host.textContent).toContain("1 supprimé(s), 1 conservé(s)");
});
