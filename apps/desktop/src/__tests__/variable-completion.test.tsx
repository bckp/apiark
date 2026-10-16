import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  renderHook,
  screen,
  act,
  waitFor,
} from "@testing-library/react";
import { VariableInput } from "@/components/ui/variable-input";
import { insertVariable, variableCompletion } from "@/lib/variable-completion";
import { useVariableCompletionStore } from "@/stores/variable-completion-store";
import { getVariableNames } from "@/lib/tauri-api";
import { registerVariableCompletions } from "@/lib/monaco-variable-completion";
import { useVariableCompletion } from "@/hooks/use-variable-completion";
import { useEnvironmentStore } from "@/stores/environment-store";
import type * as Monaco from "monaco-editor";

vi.mock("@/lib/tauri-api", () => ({ getVariableNames: vi.fn() }));
beforeEach(() => {
  useVariableCompletionStore.getState().clear();
  vi.mocked(getVariableNames).mockReset();
});
afterEach(cleanup);

function Input({
  onKeyDown = vi.fn(),
}: {
  onKeyDown?: React.KeyboardEventHandler<HTMLInputElement>;
}) {
  const [value, setValue] = useState("");
  return (
    <VariableInput
      aria-label="URL"
      value={value}
      onValueChange={setValue}
      onKeyDown={onKeyDown}
    />
  );
}
function type(text: string, cursor = text.length) {
  const input = screen.getByRole("combobox");
  fireEvent.focus(input);
  fireEvent.change(input, { target: { value: text, selectionStart: cursor } });
  return input;
}

describe("Names-only variable completion", () => {
  it("replaces a partial token, a closed token and its remaining suffix", () => {
    for (const [text, cursor, expected] of [
      ["https://{{to", 12, "https://{{token}}"],
      ["{{token}}/path", 4, "{{token}}/path"],
      ["{{token/path", 4, "{{token}}/path"],
      ['{"value":"{{to}}"}', 14, '{"value":"{{token}}"}'],
    ] as const) {
      const range = variableCompletion(text, cursor, ["token"]);
      expect(range).not.toBeNull();
      expect(insertVariable(text, range!, "token").value).toBe(expected);
    }
    expect(variableCompletion("plain", 5, ["token"])).toBeNull();
    expect(
      variableCompletion("{{TOKEN", 7, ["token", "token", "bad\nname"])
        ?.candidates,
    ).toEqual(["token"]);
  });

  it("renders only names and accepts using arrows, Enter or mouse", () => {
    useVariableCompletionStore.setState({
      names: ["apiToken", "baseUrl"],
      context: "request-a",
    });
    const shortcut = vi.fn();
    render(<Input onKeyDown={shortcut} />);
    const input = type("{{");
    expect(
      screen.getAllByRole("option").map((item) => item.textContent),
    ).toEqual(["{{apiToken}}", "{{baseUrl}}"]);
    expect(screen.getByRole("listbox").innerHTML).not.toMatch(
      /title=|synthetic-private-value/,
    );
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(input).toHaveValue("{{baseUrl}}");
    expect(shortcut).not.toHaveBeenCalled();
    type("Bearer {{api");
    fireEvent.mouseDown(screen.getByRole("option"));
    expect(input).toHaveValue("Bearer {{apiToken}}");
  });

  it("handles Escape, Tab, blur and request shortcuts without accidental acceptance", () => {
    useVariableCompletionStore.setState({
      names: ["token"],
      context: "request-a",
    });
    const shortcut = vi.fn();
    render(<Input onKeyDown={shortcut} />);
    const input = type("{{to");
    fireEvent.keyDown(input, { key: "Enter", metaKey: true });
    expect(shortcut).toHaveBeenCalledTimes(1);
    expect(input).toHaveValue("{{to");
    fireEvent.keyDown(input, { key: "Escape" });
    expect(screen.queryByRole("listbox")).toBeNull();
    type("{{tok");
    fireEvent.keyDown(input, { key: "Tab" });
    expect(input).toHaveValue("{{token}}");
    type("{{t");
    fireEvent.blur(input);
    expect(screen.queryByRole("listbox")).toBeNull();
  });

  it("ignores a previous context completing late and fails closed on errors", async () => {
    let finish!: (names: string[]) => void;
    vi.mocked(getVariableNames)
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            finish = resolve;
          }),
      )
      .mockResolvedValueOnce(["newName"]);
    const old = useVariableCompletionStore
      .getState()
      .load("a", "/a", null, null, [], []);
    await useVariableCompletionStore
      .getState()
      .load("b", "/b", null, null, ["runtimeName"], []);
    finish(["oldName"]);
    await old;
    expect(useVariableCompletionStore.getState().names).toEqual([
      "newName",
      "runtimeName",
    ]);
    vi.mocked(getVariableNames).mockRejectedValueOnce(
      new Error("synthetic-private-value"),
    );
    await useVariableCompletionStore
      .getState()
      .load("c", "/c", null, null, [], []);
    expect(useVariableCompletionStore.getState()).toMatchObject({
      names: [],
      failed: true,
    });
    expect(JSON.stringify(useVariableCompletionStore.getState())).not.toContain(
      "synthetic-private-value",
    );
  });

  it("loads names once for all fields without passing runtime or environment values", async () => {
    useEnvironmentStore.setState({
      activeCollectionPath: "/synthetic",
      activeEnvironmentName: "test",
      loading: false,
      environments: [
        {
          name: "test",
          variables: { token: "synthetic-environment-value" },
          secrets: [],
        },
      ],
      runtimeOverrides: {
        runtimeKey: "synthetic-private-value",
        deletedKey: null,
      },
    });
    vi.mocked(getVariableNames).mockResolvedValue(["token"]);
    const hook = renderHook(
      ({ request }) => useVariableCompletion(request, "/synthetic", null),
      { initialProps: { request: "a" } },
    );
    await waitFor(() =>
      expect(useVariableCompletionStore.getState().names).toEqual([
        "token",
        "runtimeKey",
      ]),
    );
    expect(getVariableNames).toHaveBeenCalledExactlyOnceWith(
      "/synthetic",
      null,
      "test",
      true,
      ["deletedKey"],
    );
    expect(JSON.stringify(useVariableCompletionStore.getState())).not.toContain(
      "synthetic-private-value",
    );
    hook.rerender({ request: "b" });
    await waitFor(() => expect(getVariableNames).toHaveBeenCalledTimes(2));
    hook.unmount();
    expect(useVariableCompletionStore.getState().names).toEqual([]);
  });

  it("closes a visible list when the active request or environment changes", () => {
    useVariableCompletionStore.setState({
      names: ["token"],
      context: "request-a",
    });
    render(<Input />);
    type("{{to");
    expect(screen.getByRole("listbox")).toBeVisible();
    act(() =>
      useVariableCompletionStore.setState({
        names: ["otherToken"],
        context: "request-b",
      }),
    );
    expect(screen.queryByRole("listbox")).toBeNull();
  });

  it("Monaco suggestions contain placeholders without details, documentation or values", () => {
    const providers: Monaco.languages.CompletionItemProvider[] = [];
    const monaco = {
      languages: {
        CompletionItemKind: { Variable: 4 },
        registerCompletionItemProvider: (
          _: string,
          provider: Monaco.languages.CompletionItemProvider,
        ) => providers.push(provider),
      },
    };
    registerVariableCompletions(monaco as unknown as typeof Monaco);
    registerVariableCompletions(monaco as unknown as typeof Monaco);
    expect(providers).toHaveLength(9);
    useVariableCompletionStore.setState({ names: ["apiToken"] });
    const model = { getLineContent: () => '"{{api}}"' };
    const result = providers[0].provideCompletionItems(
      model as unknown as Monaco.editor.ITextModel,
      { lineNumber: 1, column: 7 },
      {} as Monaco.languages.CompletionContext,
      {} as Monaco.CancellationToken,
    ) as Monaco.languages.CompletionList;
    expect(result.suggestions).toEqual([
      {
        label: "{{apiToken}}",
        insertText: "{{apiToken}}",
        filterText: "{{apiToken}}",
        kind: 4,
        range: {
          startLineNumber: 1,
          endLineNumber: 1,
          startColumn: 2,
          endColumn: 9,
        },
      },
    ]);
  });
});
