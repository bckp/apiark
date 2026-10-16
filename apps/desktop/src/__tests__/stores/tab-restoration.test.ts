import { beforeEach, describe, expect, it, vi } from "vitest";
import { loadPersistedState, readRequestFile } from "@/lib/tauri-api";

vi.mock("@/lib/tauri-api", () => ({
  loadPersistedState: vi.fn(),
  readRequestFile: vi.fn(),
}));
vi.mock("@/stores/collection-store", () => ({
  useCollectionStore: {
    getState: () => ({ openCollection: vi.fn().mockResolvedValue(undefined) }),
  },
}));
vi.mock("@/stores/toast-store", () => ({
  useToastStore: { getState: () => ({ showWarning: vi.fn() }) },
}));

const request = {
  name: "Synthetic",
  method: "GET" as const,
  url: "https://example.test",
  headers: {},
};
const persisted = (paths: string[], activeTabIndex = 0) => ({
  tabs: paths.map((filePath) => ({ filePath, collectionPath: "/synthetic" })),
  activeTabIndex,
});

beforeEach(() => {
  vi.resetModules();
  vi.mocked(loadPersistedState).mockReset();
  vi.mocked(readRequestFile).mockReset();
});

describe("Session restoration", () => {
  it("shares concurrent completion and only restores once per session", async () => {
    const { useTabStore } = await import("@/stores/tab-store");
    let finish!: (value: ReturnType<typeof persisted>) => void;
    vi.mocked(loadPersistedState).mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    );
    vi.mocked(readRequestFile).mockResolvedValue(request);
    const first = useTabStore.getState().restoreTabs();
    const second = useTabStore.getState().restoreTabs();
    expect(first).toBe(second);
    finish(
      persisted(
        ["/synthetic/a.yaml", "/synthetic/a.yaml", "/synthetic/b.yaml"],
        2,
      ),
    );
    await second;
    await useTabStore.getState().restoreTabs();
    expect(loadPersistedState).toHaveBeenCalledTimes(1);
    expect(useTabStore.getState().tabs).toHaveLength(2);
    expect(
      useTabStore
        .getState()
        .tabs.find((t) => t.id === useTabStore.getState().activeTabId)
        ?.filePath,
    ).toBe("/synthetic/b.yaml");
  });

  it("preserves a tab and selection created while a file is loading", async () => {
    const { useTabStore } = await import("@/stores/tab-store");
    vi.mocked(loadPersistedState).mockResolvedValue(
      persisted(["/synthetic/a.yaml"]),
    );
    let finish!: (value: typeof request) => void;
    vi.mocked(readRequestFile).mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    );
    const pending = useTabStore.getState().restoreTabs();
    await vi.waitFor(() => expect(readRequestFile).toHaveBeenCalled());
    useTabStore.getState().newTab({ filePath: "/synthetic/a.yaml" });
    const active = useTabStore.getState().activeTabId;
    finish(request);
    await pending;
    expect(useTabStore.getState().tabs).toHaveLength(1);
    expect(useTabStore.getState().activeTabId).toBe(active);
  });

  it("allows a retry after persistence loading fails", async () => {
    const { useTabStore } = await import("@/stores/tab-store");
    vi.mocked(loadPersistedState)
      .mockRejectedValueOnce(new Error("unavailable"))
      .mockResolvedValueOnce(persisted(["/synthetic/a.yaml"]));
    vi.mocked(readRequestFile).mockResolvedValue(request);
    await useTabStore.getState().restoreTabs();
    await useTabStore.getState().restoreTabs();
    expect(loadPersistedState).toHaveBeenCalledTimes(2);
    expect(useTabStore.getState().tabs).toHaveLength(1);
  });
});
