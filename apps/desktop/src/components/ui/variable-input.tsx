import {
  forwardRef,
  useId,
  useLayoutEffect,
  useRef,
  useState,
  type InputHTMLAttributes,
} from "react";
import { createPortal } from "react-dom";
import { insertVariable, variableCompletion } from "@/lib/variable-completion";
import { useVariableCompletionStore } from "@/stores/variable-completion-store";

interface Props
  extends Omit<InputHTMLAttributes<HTMLInputElement>, "value" | "onChange"> {
  value: string;
  onValueChange: (value: string) => void;
}

export const VariableInput = forwardRef<HTMLInputElement, Props>(
  function VariableInput(
    { value, onValueChange, onKeyDown, onFocus, onBlur, onSelect, ...props },
    ref,
  ) {
    const names = useVariableCompletionStore((s) => s.names);
    const context = useVariableCompletionStore((s) => s.context);
    const input = useRef<HTMLInputElement | null>(null);
    const list = useRef<HTMLDivElement>(null);
    const id = useId();
    const [cursor, setCursor] = useState(0);
    const [open, setOpen] = useState(false);
    const [selection, setSelection] = useState(0);
    const [bounds, setBounds] = useState({ left: 0, top: 0, width: 0 });
    const completion = variableCompletion(value, cursor, names);
    const candidates = completion?.candidates ?? [];
    const index = Math.min(selection, Math.max(0, candidates.length - 1));
    const visible =
      open && candidates.length > 0 && !props.disabled && !props.readOnly;

    useLayoutEffect(() => {
      setOpen(false);
      setSelection(0);
    }, [context]);
    useLayoutEffect(() => {
      if (!visible) return;
      const reposition = () => {
        const rect = input.current?.getBoundingClientRect();
        if (rect)
          setBounds({
            left: rect.left,
            top: rect.bottom + 4,
            width: rect.width,
          });
      };
      reposition();
      window.addEventListener("resize", reposition);
      window.addEventListener("scroll", reposition, true);
      return () => {
        window.removeEventListener("resize", reposition);
        window.removeEventListener("scroll", reposition, true);
      };
    }, [visible]);
    useLayoutEffect(() => {
      list.current?.children[index]?.scrollIntoView?.({ block: "nearest" });
    }, [index, id, visible]);

    const accept = (name: string) => {
      if (!completion) return;
      const result = insertVariable(value, completion, name);
      onValueChange(result.value);
      setOpen(false);
      // React applies the controlled value before the next frame. Guard a tab switch.
      const target = input.current;
      requestAnimationFrame(() => {
        if (
          target &&
          target === input.current &&
          document.activeElement === target &&
          useVariableCompletionStore.getState().context === context
        )
          target.setSelectionRange(result.cursor, result.cursor);
      });
    };

    return (
      <>
        <input
          {...props}
          ref={(node) => {
            input.current = node;
            if (typeof ref === "function") ref(node);
            else if (ref) ref.current = node;
          }}
          value={value}
          role="combobox"
          aria-autocomplete="list"
          aria-expanded={visible}
          aria-controls={visible ? id : undefined}
          aria-activedescendant={visible ? `${id}-${index}` : undefined}
          autoComplete="off"
          spellCheck={false}
          onChange={(event) => {
            onValueChange(event.target.value);
            setCursor(event.target.selectionStart ?? 0);
            setSelection(0);
            setOpen(!(event.nativeEvent as InputEvent).isComposing);
          }}
          onSelect={(event) => {
            setCursor(event.currentTarget.selectionStart ?? 0);
            onSelect?.(event);
          }}
          onFocus={(event) => {
            setCursor(event.currentTarget.selectionStart ?? 0);
            setOpen(true);
            onFocus?.(event);
          }}
          onBlur={(event) => {
            setOpen(false);
            onBlur?.(event);
          }}
          onKeyDown={(event) => {
            if (
              visible &&
              !event.nativeEvent.isComposing &&
              !event.metaKey &&
              !event.ctrlKey &&
              !event.altKey
            ) {
              if (event.key === "ArrowDown" || event.key === "ArrowUp") {
                event.preventDefault();
                event.stopPropagation();
                setSelection(
                  (index +
                    (event.key === "ArrowDown" ? 1 : candidates.length - 1)) %
                    candidates.length,
                );
                return;
              }
              if (
                (event.key === "Enter" || event.key === "Tab") &&
                !event.shiftKey
              ) {
                event.preventDefault();
                event.stopPropagation();
                accept(candidates[index]);
                return;
              }
              if (event.key === "Escape") {
                event.preventDefault();
                event.stopPropagation();
                setOpen(false);
                return;
              }
            }
            onKeyDown?.(event);
          }}
        />
        {visible &&
          createPortal(
            <div
              ref={list}
              id={id}
              role="listbox"
              aria-label="Variable names"
              className="fixed z-[100] max-h-60 overflow-y-auto rounded-lg border border-[var(--color-border)] bg-[var(--color-elevated)] py-1 shadow-xl"
              style={bounds}
            >
              {candidates.map((name, candidate) => (
                <div
                  key={name}
                  id={`${id}-${candidate}`}
                  role="option"
                  aria-selected={candidate === index}
                  className={`cursor-pointer px-3 py-1.5 font-mono text-xs text-[var(--color-text-primary)] ${candidate === index ? "bg-[var(--color-accent)]/20" : ""}`}
                  onMouseDown={(event) => {
                    event.preventDefault();
                    accept(name);
                  }}
                >
                  {`{{${name}}}`}
                </div>
              ))}
            </div>,
            document.body,
          )}
      </>
    );
  },
);
