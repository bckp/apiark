import { Component, type ReactNode } from "react";

interface Props {
  children: ReactNode;
  label: string;
}

/** Keep sibling panels usable; never render exception messages or request data. */
export class ErrorBoundary extends Component<Props, { failed: boolean }> {
  state = { failed: false };

  static getDerivedStateFromError() {
    return { failed: true };
  }

  render() {
    if (!this.state.failed) return this.props.children;
    return (
      <div
        role="alert"
        className="flex min-w-0 flex-1 flex-col items-center justify-center gap-3 p-6 text-sm text-[var(--color-text-muted)]"
      >
        <p>{this.props.label} could not be displayed.</p>
        <button
          type="button"
          className="rounded border border-[var(--color-border)] px-3 py-1.5 text-[var(--color-text-primary)]"
          onClick={() => this.setState({ failed: false })}
        >
          Try again
        </button>
      </div>
    );
  }
}
