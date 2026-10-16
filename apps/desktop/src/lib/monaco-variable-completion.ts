import type * as Monaco from "monaco-editor";
import { variableCompletion } from "@/lib/variable-completion";
import { useVariableCompletionStore } from "@/stores/variable-completion-store";

let registered = false;
export function registerVariableCompletions(monaco: typeof Monaco) {
  if (registered) return;
  registered = true;
  for (const language of [
    "json",
    "xml",
    "plaintext",
    "graphql",
    "javascript",
    "typescript",
    "yaml",
    "html",
    "css",
  ]) {
    monaco.languages.registerCompletionItemProvider(language, {
      triggerCharacters: ["{", "$"],
      provideCompletionItems(model, position) {
        const range = variableCompletion(
          model.getLineContent(position.lineNumber),
          position.column - 1,
          useVariableCompletionStore.getState().names,
        );
        if (!range) return { suggestions: [] };
        return {
          suggestions: range.candidates.map((name) => ({
            label: `{{${name}}}`,
            kind: monaco.languages.CompletionItemKind.Variable,
            insertText: `{{${name}}}`,
            filterText: `{{${name}}}`,
            range: {
              startLineNumber: position.lineNumber,
              endLineNumber: position.lineNumber,
              startColumn: range.start + 1,
              endColumn: range.end + 1,
            },
          })),
        };
      },
    });
  }
}
