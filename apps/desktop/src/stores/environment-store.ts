import { create } from "zustand";
import type { EnvironmentData } from "@apiark/types";
import {
  loadEnvironments as loadEnvironmentsApi,
  getResolvedVariables as getResolvedVariablesApi,
  loadRootDotenv,
} from "@/lib/tauri-api";

interface EnvironmentState {
  environments: EnvironmentData[];
  activeEnvironmentName: string | null;
  activeCollectionPath: string | null;
  /** Runtime variable overrides from scripts (not persisted to disk) */
  runtimeOverrides: Record<string, string | null>;
  loading: boolean;

  loadEnvironments: (collectionPath: string) => Promise<void>;
  setActiveEnvironment: (name: string | null) => void;
  setActiveCollectionPath: (path: string | null) => void;
  getResolvedVariables: (
    expectedCollectionPath?: string | null,
  ) => Promise<Record<string, string>>;
  applyMutations: (mutations: Record<string, string | null>) => void;
}

let loadGeneration = 0;
export const useEnvironmentStore = create<EnvironmentState>((set, get) => ({
  environments: [],
  activeEnvironmentName: null,
  activeCollectionPath: null,
  runtimeOverrides: {},
  loading: false,

  loadEnvironments: async (collectionPath) => {
    const generation = ++loadGeneration;
    const sameCollection = get().activeCollectionPath === collectionPath;
    set({
      activeCollectionPath: collectionPath,
      loading: true,
      ...(sameCollection
        ? {}
        : {
            environments: [],
            activeEnvironmentName: null,
            runtimeOverrides: {},
          }),
    });
    try {
      const envs = await loadEnvironmentsApi(collectionPath);
      if (
        generation !== loadGeneration ||
        get().activeCollectionPath !== collectionPath
      )
        return;
      set({
        environments: envs,
        activeCollectionPath: collectionPath,
        // Auto-select first environment if none selected
        activeEnvironmentName:
          sameCollection &&
          (get().activeEnvironmentName === null ||
            envs.some((e) => e.name === get().activeEnvironmentName))
            ? get().activeEnvironmentName
            : (envs[0]?.name ?? null),
      });
    } catch (err) {
      if (generation !== loadGeneration) return;
      import("@/stores/toast-store").then(({ useToastStore }) =>
        useToastStore
          .getState()
          .showError(`Failed to load environments: ${err}`),
      );
    } finally {
      if (generation === loadGeneration) set({ loading: false });
    }
  },

  setActiveEnvironment: (name) => {
    set({ activeEnvironmentName: name, runtimeOverrides: {} });
  },

  setActiveCollectionPath: (path) => {
    ++loadGeneration;
    set({
      activeCollectionPath: path,
      environments: [],
      activeEnvironmentName: null,
      runtimeOverrides: {},
      loading: false,
    });
  },

  getResolvedVariables: async (expectedCollectionPath) => {
    const { activeCollectionPath, activeEnvironmentName, runtimeOverrides } =
      get();
    if (
      get().loading ||
      (expectedCollectionPath &&
        expectedCollectionPath !== activeCollectionPath)
    )
      throw new Error(
        "Environment for this request's collection is not ready. Select the collection's environment and retry.",
      );
    const withOverrides = (base: Record<string, string>) => {
      const resolved = { ...base };
      for (const [key, value] of Object.entries(runtimeOverrides)) {
        if (value === null) delete resolved[key];
        else resolved[key] = value;
      }
      return resolved;
    };
    if (!activeCollectionPath) {
      return withOverrides({});
    }
    if (!activeEnvironmentName) {
      // No environment selected — still load root .env variables
      try {
        const rootVars = await loadRootDotenv(activeCollectionPath);
        return withOverrides(rootVars);
      } catch (error) {
        import("@/stores/toast-store").then(({ useToastStore }) =>
          useToastStore.getState().showWarning("Could not load .env file"),
        );
        throw error;
      }
    }
    try {
      const resolved = await getResolvedVariablesApi(
        activeCollectionPath,
        activeEnvironmentName,
      );
      return withOverrides(resolved);
    } catch (err) {
      import("@/stores/toast-store").then(({ useToastStore }) =>
        useToastStore
          .getState()
          .showError(`Failed to resolve variables: ${err}`),
      );
      throw err;
    }
  },

  applyMutations: (mutations) => {
    set((state) => {
      const overrides = { ...state.runtimeOverrides };
      for (const [key, value] of Object.entries(mutations)) {
        overrides[key] = value;
      }
      return { runtimeOverrides: overrides };
    });
  },
}));
