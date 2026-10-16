import { useLayoutEffect } from "react";
import { useEnvironmentStore } from "@/stores/environment-store";
import { useVariableCompletionStore } from "@/stores/variable-completion-store";

/** One loader for all inputs; clear stale names before painting another context. */
export function useVariableCompletion(
  requestId: string | null,
  collection: string | null,
  source: string | null,
  includeScopes = true,
) {
  const path = useEnvironmentStore((s) => s.activeCollectionPath);
  const environment = useEnvironmentStore((s) => s.activeEnvironmentName);
  const loading = useEnvironmentStore((s) => s.loading);
  const environments = useEnvironmentStore((s) => s.environments);
  const overrides = useEnvironmentStore((s) => s.runtimeOverrides);
  useLayoutEffect(() => {
    const store = useVariableCompletionStore.getState();
    if (loading || path !== collection) {
      store.clear();
      return;
    }
    const keys = Object.keys(overrides);
    void store.load(
      JSON.stringify([
        requestId,
        collection,
        source,
        environment,
        includeScopes,
      ]),
      collection,
      source,
      environment,
      keys.filter((key) => overrides[key] !== null),
      keys.filter((key) => overrides[key] === null),
      includeScopes,
    );
    return () => store.clear();
  }, [
    requestId,
    collection,
    source,
    path,
    environment,
    loading,
    environments,
    overrides,
    includeScopes,
  ]);
}
