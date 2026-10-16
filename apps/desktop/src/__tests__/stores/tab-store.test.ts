import { describe, it, expect, beforeEach, vi } from "vitest";

vi.mock("@/lib/tauri-api", () => ({
  sendRequest: vi.fn().mockResolvedValue({}),
  sendRequestWithScripts: vi.fn().mockResolvedValue({}),
  readRequestFile: vi.fn().mockResolvedValue({}),
  saveRequestFile: vi.fn().mockResolvedValue(undefined),
  loadPersistedState: vi.fn().mockResolvedValue({ tabs: [], activeTabId: null }),
  savePersistedState: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@/stores/environment-store", () => ({
  useEnvironmentStore: { getState: () => ({ getResolvedVariables: vi.fn().mockResolvedValue({}) }) },
}));

vi.mock("@/stores/settings-store", () => ({
  useSettingsStore: { getState: () => ({ settings: {} }) },
}));

vi.mock("@/stores/console-store", () => ({
  useConsoleStore: { getState: () => ({ log: vi.fn() }) },
}));

import { useTabStore } from "@/stores/tab-store";
import { sendRequestWithScripts } from "@/lib/tauri-api";

function scriptedResponse(text: string) {
  return {status:200,statusText:"OK",headers:[],cookies:[],body:text,timeMs:1,sizeBytes:1,
    testResults:[],assertionResults:[],consoleOutput:[],envMutations:{},visualizations:[{type:"text" as const,name:"Result",data:text}]};
}

describe("Response visualization ownership", () => {
  beforeEach(() => {useTabStore.setState({tabs:[],activeTabId:null});vi.mocked(sendRequestWithScripts).mockReset();});
  it("keeps an in-flight result on its original tab", async () => {
    let finish!: (value: ReturnType<typeof scriptedResponse>) => void;
    vi.mocked(sendRequestWithScripts).mockImplementationOnce(() => new Promise((resolve)=>{finish=resolve;}));
    useTabStore.getState().newTab();useTabStore.getState().setUrl("https://example.test");useTabStore.getState().setPostResponseScript("ark.visualize('text',{data:'a'})");
    const original=useTabStore.getState().activeTabId;
    const pending=useTabStore.getState().send();await Promise.resolve();await Promise.resolve();
    useTabStore.getState().newTab();finish(scriptedResponse("original"));await pending;
    expect(useTabStore.getState().tabs.find((t)=>t.id===original)?.response?.visualizations?.[0]).toMatchObject({data:"original"});
    expect(useTabStore.getState().tabs.at(-1)?.response).toBeNull();
  });
  it("clears outputs on rerun and ignores an older completion", async () => {
    let oldFinish!: (value: ReturnType<typeof scriptedResponse>) => void;
    vi.mocked(sendRequestWithScripts).mockImplementationOnce(()=>new Promise((resolve)=>{oldFinish=resolve;})).mockResolvedValueOnce(scriptedResponse("new"));
    useTabStore.getState().newTab();useTabStore.getState().setUrl("https://example.test");useTabStore.getState().setPostResponseScript("script");
    const old=useTabStore.getState().send();await Promise.resolve();await Promise.resolve();
    const latest=useTabStore.getState().send();expect(useTabStore.getState().tabs[0].response).toBeNull();await latest;
    oldFinish(scriptedResponse("old"));await old;
    expect(useTabStore.getState().tabs[0].response?.body).toBe("new");
  });
  it("clears old outputs if a script/transport fails", async () => {
    vi.mocked(sendRequestWithScripts).mockResolvedValueOnce(scriptedResponse("first")).mockRejectedValueOnce("script failed");
    useTabStore.getState().newTab();useTabStore.getState().setUrl("https://example.test");useTabStore.getState().setPostResponseScript("script");
    await useTabStore.getState().send();await useTabStore.getState().send();
    expect(useTabStore.getState().tabs[0].response).toBeNull();expect(useTabStore.getState().tabs[0].error?.message).toBe("script failed");
  });
});

describe("Tab Store", () => {
  beforeEach(() => {
    useTabStore.setState({
      tabs: [],
      activeTabId: null,
    });
  });

  it("creates a new HTTP tab", () => {
    useTabStore.getState().newTab();
    const state = useTabStore.getState();
    expect(state.tabs).toHaveLength(1);
    expect(state.tabs[0].protocol).toBe("http");
    expect(state.tabs[0].method).toBe("GET");
    expect(state.tabs[0].name).toBe("Untitled Request");
    expect(state.activeTabId).toBe(state.tabs[0].id);
  });






  it("closes a tab", () => {
    useTabStore.getState().newTab();
    useTabStore.getState().newTab();
    const firstId = useTabStore.getState().tabs[0].id;
    expect(useTabStore.getState().tabs).toHaveLength(2);

    useTabStore.getState().closeTab(firstId);
    expect(useTabStore.getState().tabs).toHaveLength(1);
    expect(useTabStore.getState().tabs[0].id).not.toBe(firstId);
  });

  it("sets active tab", () => {
    useTabStore.getState().newTab();
    useTabStore.getState().newTab();
    const tabs = useTabStore.getState().tabs;

    useTabStore.getState().setActiveTab(tabs[0].id);
    expect(useTabStore.getState().activeTabId).toBe(tabs[0].id);

    useTabStore.getState().setActiveTab(tabs[1].id);
    expect(useTabStore.getState().activeTabId).toBe(tabs[1].id);
  });

  it("updates method on active tab", () => {
    useTabStore.getState().newTab();
    useTabStore.getState().setMethod("POST");
    expect(useTabStore.getState().tabs[0].method).toBe("POST");
  });

  it("updates URL on active tab", () => {
    useTabStore.getState().newTab();
    useTabStore.getState().setUrl("https://api.example.com/users");
    expect(useTabStore.getState().tabs[0].url).toBe("https://api.example.com/users");
  });

  it("marks tab as dirty after changes", () => {
    useTabStore.getState().newTab();
    expect(useTabStore.getState().tabs[0].isDirty).toBe(false);

    useTabStore.getState().setUrl("https://changed.com");
    expect(useTabStore.getState().tabs[0].isDirty).toBe(true);
  });

  it("supports undo/redo", () => {
    useTabStore.getState().newTab();
    useTabStore.getState().setUrl("https://first.com");
    useTabStore.getState().setUrl("https://second.com");
    expect(useTabStore.getState().tabs[0].url).toBe("https://second.com");

    useTabStore.getState().undoTab();
    expect(useTabStore.getState().tabs[0].url).toBe("https://first.com");

    useTabStore.getState().redoTab();
    expect(useTabStore.getState().tabs[0].url).toBe("https://second.com");
  });

  it("toggles pin on tab", () => {
    useTabStore.getState().newTab();
    const tabId = useTabStore.getState().tabs[0].id;
    expect(useTabStore.getState().tabs[0].pinned).toBe(false);

    useTabStore.getState().togglePin(tabId);
    expect(useTabStore.getState().tabs[0].pinned).toBe(true);

    useTabStore.getState().togglePin(tabId);
    expect(useTabStore.getState().tabs[0].pinned).toBe(false);
  });

  it("duplicates a tab", () => {
    useTabStore.getState().newTab();
    useTabStore.getState().setUrl("https://dup.test");
    const tabId = useTabStore.getState().tabs[0].id;

    useTabStore.getState().duplicateTab(tabId);
    const tabs = useTabStore.getState().tabs;
    expect(tabs).toHaveLength(2);
    expect(tabs[1].url).toBe("https://dup.test");
    expect(tabs[1].id).not.toBe(tabs[0].id);
  });

  it("closes other tabs", () => {
    useTabStore.getState().newTab();
    useTabStore.getState().newTab();
    useTabStore.getState().newTab();
    const keepId = useTabStore.getState().tabs[1].id;

    useTabStore.getState().closeOtherTabs(keepId);
    const tabs = useTabStore.getState().tabs;
    expect(tabs).toHaveLength(1);
    expect(tabs[0].id).toBe(keepId);
  });

  it("closes all tabs", () => {
    useTabStore.getState().newTab();
    useTabStore.getState().newTab();

    useTabStore.getState().closeAllTabs();
    expect(useTabStore.getState().tabs).toHaveLength(0);
    expect(useTabStore.getState().activeTabId).toBeNull();
  });
});
