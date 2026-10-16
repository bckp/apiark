import {
  useEffect,
  useState,
  useRef,
  useCallback,
  lazy,
  Suspense,
} from "react";
import {
  ActivityBar,
  type ActivityView,
} from "@/components/layout/activity-bar";
import { SidePanel } from "@/components/layout/side-panel";
import { TabBar } from "@/components/tabs/tab-bar";
import { UrlBar } from "@/components/request/url-bar";
import { RequestPanel } from "@/components/request/request-panel";
import { ResponsePanel } from "@/components/response/response-panel";
import { ConsoleBottomBar } from "@/components/console/console-panel";
import { ToastContainer } from "@/components/ui/toast-container";
import { PanelDivider } from "@/components/ui/panel-divider";
import { ErrorBoundary } from "@/components/ui/error-boundary";
import {
  useTabStore,
  useActiveTab,
  initWindowStateTracker,
} from "@/stores/tab-store";
import { useSettingsStore } from "@/stores/settings-store";
import { useCollectionStore } from "@/stores/collection-store";
import { useEnvironmentStore } from "@/stores/environment-store";
import { useConsoleStore } from "@/stores/console-store";
import { useTheme } from "@/hooks/use-theme";
import { useAutoSave } from "@/hooks/use-auto-save";
import { useFileWatcher } from "@/hooks/use-file-watcher";
import { useVariableCompletion } from "@/hooks/use-variable-completion";
import {
  PanelLeft,
  Plus,
  Save,
  Play,
  Upload,
  Settings,
  Maximize2,
} from "lucide-react";

const SettingsDialog = lazy(() =>
  import("@/components/settings/settings-dialog").then((m) => ({
    default: m.SettingsDialog,
  })),
);
const ImportDialog = lazy(() =>
  import("@/components/import/import-dialog").then((m) => ({
    default: m.ImportDialog,
  })),
);
const CurlImportDialog = lazy(() =>
  import("@/components/request/curl-import-dialog").then((m) => ({
    default: m.CurlImportDialog,
  })),
);
const CollectionRunnerDialog = lazy(() =>
  import("@/components/runner/collection-runner-dialog").then((m) => ({
    default: m.CollectionRunnerDialog,
  })),
);
const SaveAsDialog = lazy(() =>
  import("@/components/request/save-as-dialog").then((m) => ({
    default: m.SaveAsDialog,
  })),
);
const ResponseDiffDialog = lazy(() =>
  import("@/components/response/response-diff-dialog").then((m) => ({
    default: m.ResponseDiffDialog,
  })),
);

export default function App() {
  const active = useActiveTab();
  const tabs = useTabStore((s) => s.tabs);
  const activeId = useTabStore((s) => s.activeTabId);
  const collections = useCollectionStore((s) => s.collections);
  const environmentCollection =
    active?.collectionPath ?? collections[0]?.path ?? null;
  useVariableCompletion(
    activeId,
    environmentCollection,
    active?.filePath ?? null,
    Boolean(active?.collectionPath),
  );
  useEffect(() => {
    if (environmentCollection)
      void useEnvironmentStore
        .getState()
        .loadEnvironments(environmentCollection);
    else useEnvironmentStore.getState().setActiveCollectionPath(null);
  }, [environmentCollection]);
  const settings = useSettingsStore((s) => s.settings);
  const { autoSaveError } = useAutoSave();
  const [view, setView] = useState<ActivityView>("collections");
  const [sidebar, setSidebar] = useState(true);
  const [zen, setZen] = useState(false);
  const [dialog, setDialog] = useState<
    "settings" | "import" | "curl" | "runner" | "save" | null
  >(null);
  const panel = useRef<HTMLDivElement>(null);
  const container = useRef<HTMLDivElement>(null);
  useTheme();
  useFileWatcher();
  useEffect(() => {
    initWindowStateTracker();
    void useSettingsStore.getState().loadSettings();
    void useTabStore
      .getState()
      .restoreTabs()
      .then(() => {
        if (!useTabStore.getState().tabs.length)
          useTabStore.getState().newTab();
      });
    const curl = () => setDialog("curl");
    const save = () => setDialog("save");
    window.addEventListener("apiark:open-curl-import", curl);
    window.addEventListener("apiark:open-save-as", save);
    return () => {
      window.removeEventListener("apiark:open-curl-import", curl);
      window.removeEventListener("apiark:open-save-as", save);
    };
  }, []);
  useEffect(() => {
    const timer = setTimeout(() => useTabStore.getState().persistTabs(), 500);
    return () => clearTimeout(timer);
  }, [tabs, activeId]);
  useEffect(() => {
    const before = (e: BeforeUnloadEvent) => {
      useTabStore.getState().persistTabs();
      if (useTabStore.getState().hasUnsavedNewTabs()) e.preventDefault();
    };
    window.addEventListener("beforeunload", before);
    return () => window.removeEventListener("beforeunload", before);
  }, []);
  const save = useCallback(() => {
    const state = useTabStore.getState();
    const tab = state.tabs.find((t) => t.id === state.activeTabId);
    if (tab?.filePath) void state.save();
    else setDialog("save");
  }, []);
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey)) return;
      if (e.key === "Enter") {
        e.preventDefault();
        void useTabStore.getState().send();
      } else if (e.key.toLowerCase() === "s") {
        e.preventDefault();
        save();
      } else if (e.key.toLowerCase() === "n") {
        e.preventDefault();
        useTabStore.getState().newTab();
      } else if (e.key.toLowerCase() === "b") {
        e.preventDefault();
        setSidebar((v) => !v);
      } else if (e.key === ",") {
        e.preventDefault();
        setDialog("settings");
      } else if (e.shiftKey && e.key.toLowerCase() === "z") {
        e.preventDefault();
        setZen((v) => !v);
      }
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [save]);
  const horizontal = settings.layout === "horizontal";
  const ratio = Math.min(0.8, Math.max(0.2, settings.panelRatio ?? 0.5));
  return (
    <div className="flex h-screen flex-col overflow-hidden bg-[var(--color-bg)] text-[var(--color-text-primary)]">
      <header
        data-tauri-drag-region
        className="flex h-[52px] shrink-0 items-center gap-1 border-b border-[var(--color-border)] bg-[var(--color-surface)] pl-[106px] pr-3"
      >
        <button
          className="p-2"
          title="Toggle sidebar (⌘B)"
          onClick={() => setSidebar((v) => !v)}
        >
          <PanelLeft size={16} />
        </button>
        <button
          className="p-2"
          title="New request (⌘N)"
          onClick={() => useTabStore.getState().newTab()}
        >
          <Plus size={16} />
        </button>
        <button className="p-2" title="Save request (⌘S)" onClick={save}>
          <Save size={16} />
        </button>
        <button
          className="p-2"
          title="Import Postman Collection v2.1"
          onClick={() => setDialog("import")}
        >
          <Upload size={16} />
        </button>
        <button
          className="p-2"
          title="Run collection"
          onClick={() => setDialog("runner")}
        >
          <Play size={16} />
        </button>
        <div data-tauri-drag-region className="h-full flex-1" />
        <button
          className="p-2"
          title="Zen mode"
          onClick={() => setZen((v) => !v)}
        >
          <Maximize2 size={16} />
        </button>
        <button
          className="p-2"
          title="Settings (⌘,)"
          onClick={() => setDialog("settings")}
        >
          <Settings size={16} />
        </button>
      </header>
      {autoSaveError && (
        <div role="alert" className="bg-red-950 px-4 py-1 text-sm">
          {autoSaveError}
        </div>
      )}
      <div className="flex min-h-0 flex-1">
        {!zen && (
          <ActivityBar
            activeView={view}
            onViewChange={(v) => {
              setView(v);
              setSidebar(true);
            }}
            onOpenSettings={() => setDialog("settings")}
            onToggleConsole={() => useConsoleStore.getState().toggle()}
          />
        )}
        {!zen && sidebar && (
          <ErrorBoundary key={view} label="Sidebar">
            <SidePanel
              activeView={view}
              onOpenImport={() => setDialog("import")}
            />
          </ErrorBoundary>
        )}
        <main className="flex min-w-0 flex-1 flex-col">
          <TabBar />
          {active ? (
            <>
              <ErrorBoundary key={`url-${activeId}`} label="Request URL">
                <UrlBar />
              </ErrorBoundary>
              <div
                ref={container}
                className={`flex min-h-0 flex-1 ${horizontal ? "flex-row" : "flex-col"}`}
              >
                <div
                  ref={panel}
                  className="flex min-h-0 min-w-0 shrink-0 flex-col"
                  style={
                    horizontal
                      ? { width: `${ratio * 100}%` }
                      : { height: `${ratio * 100}%` }
                  }
                >
                  <ErrorBoundary key={activeId} label="Request editor">
                    <RequestPanel />
                  </ErrorBoundary>
                </div>
                <PanelDivider
                  direction={horizontal ? "horizontal" : "vertical"}
                  panelRef={panel}
                  containerRef={container}
                  onResizeEnd={(panelRatio) =>
                    void useSettingsStore
                      .getState()
                      .updateSettings({ panelRatio })
                  }
                  onDoubleClick={() =>
                    void useSettingsStore
                      .getState()
                      .updateSettings({ panelRatio: 0.5 })
                  }
                />
                <ErrorBoundary key={activeId} label="Response">
                  <ResponsePanel />
                </ErrorBoundary>
              </div>
            </>
          ) : (
            <div className="m-auto text-sm text-[var(--color-text-muted)]">
              Create or open a request to begin.
            </div>
          )}
          <ErrorBoundary label="Console">
            <ConsoleBottomBar />
          </ErrorBoundary>
        </main>
      </div>
      <ErrorBoundary key={dialog} label="Dialog">
        <Suspense fallback={null}>
          {dialog === "settings" && (
            <SettingsDialog open onOpenChange={(v) => !v && setDialog(null)} />
          )}
          {dialog === "import" && (
            <ImportDialog open onOpenChange={(v) => !v && setDialog(null)} />
          )}
          {dialog === "curl" && (
            <CurlImportDialog
              open
              onOpenChange={(v) => !v && setDialog(null)}
            />
          )}
          {dialog === "runner" && (
            <CollectionRunnerDialog
              open
              onOpenChange={(v) => !v && setDialog(null)}
            />
          )}
          {dialog === "save" && (
            <SaveAsDialog
              open
              onOpenChange={(v) => !v && setDialog(null)}
              defaultName={active?.name ?? ""}
              onSave={async (collectionPath, dir, filename, name) => {
                const target = useTabStore.getState().activeTabId;
                if (!target) return;
                try {
                  const filePath = await useCollectionStore
                    .getState()
                    .createRequest(dir, filename, name, collectionPath);
                  useTabStore.setState((state) => ({
                    tabs: state.tabs.map((t) =>
                      t.id === target
                        ? {
                            ...t,
                            filePath,
                            collectionPath,
                            name,
                            isDirty: true,
                          }
                        : t,
                    ),
                  }));
                  await useTabStore.getState().save(target);
                  setDialog(null);
                } catch (error) {
                  const { useToastStore } = await import(
                    "@/stores/toast-store"
                  );
                  useToastStore.getState().showError(String(error));
                }
              }}
            />
          )}
          <ResponseDiffDialog />
        </Suspense>
      </ErrorBoundary>
      <ToastContainer />
    </div>
  );
}
