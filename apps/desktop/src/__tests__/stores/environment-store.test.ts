import { describe, it, expect, beforeEach, vi } from "vitest";
import { useEnvironmentStore } from "@/stores/environment-store";
import { loadEnvironments, getResolvedVariables } from "@/lib/tauri-api";

// Mock the Tauri API
vi.mock("@/lib/tauri-api", () => ({
  loadEnvironments: vi.fn().mockResolvedValue([
    {
      name: "development",
      variables: { baseUrl: "http://localhost:3000", apiKey: "dev-key" },
      secrets: [],
    },
    {
      name: "production",
      variables: { baseUrl: "https://api.prod.com" },
      secrets: ["apiKey"],
    },
  ]),
  getResolvedVariables: vi.fn().mockResolvedValue({
    baseUrl: "http://localhost:3000",
    apiKey: "dev-key",
  }),
  loadRootDotenv: vi.fn().mockResolvedValue({}),
}));

describe("Environment Store", () => {
  beforeEach(() => {
    useEnvironmentStore.setState({
      environments: [],
      activeEnvironmentName: null,
      activeCollectionPath: null,
      runtimeOverrides: {},
      loading: false,
    });
  });

  it("loads environments from collection", async () => {
    await useEnvironmentStore.getState().loadEnvironments("/test/collection");
    const state = useEnvironmentStore.getState();
    expect(state.environments).toHaveLength(2);
    expect(state.environments[0].name).toBe("development");
    expect(state.environments[1].name).toBe("production");
    expect(state.activeCollectionPath).toBe("/test/collection");
  });

  it("auto-selects first environment", async () => {
    await useEnvironmentStore.getState().loadEnvironments("/test/collection");
    expect(useEnvironmentStore.getState().activeEnvironmentName).toBe(
      "development",
    );
  });

  it("sets active environment", async () => {
    await useEnvironmentStore.getState().loadEnvironments("/test/collection");
    useEnvironmentStore.getState().setActiveEnvironment("production");
    expect(useEnvironmentStore.getState().activeEnvironmentName).toBe(
      "production",
    );
  });

  it("applies runtime mutations", () => {
    useEnvironmentStore.getState().applyMutations({
      newVar: "newValue",
      deleteVar: null,
    });
    const overrides = useEnvironmentStore.getState().runtimeOverrides;
    expect(overrides.newVar).toBe("newValue");
  });
  it("does not reuse runtime values across collections or environments", async () => {
    await useEnvironmentStore.getState().loadEnvironments("/first");
    useEnvironmentStore.getState().applyMutations({ token: "synthetic-first" });
    await useEnvironmentStore.getState().loadEnvironments("/second");
    expect(useEnvironmentStore.getState().runtimeOverrides).toEqual({});
    await expect(
      useEnvironmentStore.getState().getResolvedVariables("/first"),
    ).rejects.toThrow("not ready");
    useEnvironmentStore
      .getState()
      .applyMutations({ token: "synthetic-second" });
    useEnvironmentStore.getState().setActiveEnvironment("production");
    expect(useEnvironmentStore.getState().runtimeOverrides).toEqual({});
  });
  it("ignores an older load finishing after a different collection", async () => {
    let finish!: (value: []) => void;
    vi.mocked(loadEnvironments).mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    );
    const old = useEnvironmentStore.getState().loadEnvironments("/old");
    await expect(
      useEnvironmentStore.getState().getResolvedVariables("/old"),
    ).rejects.toThrow("not ready");
    await useEnvironmentStore.getState().loadEnvironments("/current");
    finish([]);
    await old;
    expect(useEnvironmentStore.getState().activeCollectionPath).toBe(
      "/current",
    );
    expect(useEnvironmentStore.getState().environments).toHaveLength(2);
  });
  it("keeps unset variables absent when resolving the next request", async () => {
    await useEnvironmentStore.getState().loadEnvironments("/synthetic");
    useEnvironmentStore.getState().applyMutations({ apiKey: null });
    expect(
      await useEnvironmentStore.getState().getResolvedVariables("/synthetic"),
    ).not.toHaveProperty("apiKey");
    expect(getResolvedVariables).toHaveBeenCalledWith(
      "/synthetic",
      "development",
    );
  });
});
