import { create } from "zustand";
import { getVariableNames } from "@/lib/tauri-api";

let generation = 0;
interface CompletionState {
  names: string[];
  context: string;
  failed: boolean;
  clear: () => void;
  load: (
    context: string,
    collection: string | null,
    source: string | null,
    environment: string | null,
    added: string[],
    removed: string[],
    includeScopes?: boolean,
  ) => Promise<void>;
}

export const useVariableCompletionStore = create<CompletionState>((set) => ({
  names: [],
  context: "",
  failed: false,
  clear: () => {
    ++generation;
    set({ names: [], context: "", failed: false });
  },
  load: async (
    context,
    collection,
    source,
    environment,
    added,
    removed,
    includeScopes = true,
  ) => {
    const current = ++generation;
    set({ names: [], context, failed: false });
    try {
      const names = await getVariableNames(
        collection,
        source,
        environment,
        includeScopes,
        removed,
      );
      if (current !== generation) return;
      set({
        names: [...new Set([...names, ...added])],
      });
    } catch {
      if (current === generation) set({ names: [], failed: true });
    }
  },
}));
