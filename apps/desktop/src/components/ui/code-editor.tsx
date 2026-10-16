import {lazy,Suspense} from "react";
import type {CodeEditorProps} from "./code-editor-impl";
const Editor=lazy(()=>import("./code-editor-impl").then(m=>({default:m.CodeEditorImpl})));
export function CodeEditor(props:CodeEditorProps){
    return <Suspense fallback={<div className="p-3 text-sm text-[var(--color-text-muted)]">Loading editor…</div>}><Editor {...props}/></Suspense>;
}
