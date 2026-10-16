import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ErrorBoundary } from "@/components/ui/error-boundary";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("Panel error boundary", () => {
  it("keeps siblings available, hides exception data and supports retry", () => {
    vi.spyOn(console, "error").mockImplementation(() => {});
    let broken = true;
    function Panel() {
      if (broken) throw new Error("Authorization: synthetic-private-value");
      return <p>Recovered panel</p>;
    }
    const view = render(
      <>
        <button>Other tab</button>
        <ErrorBoundary label="Response">
          <Panel />
        </ErrorBoundary>
      </>,
    );
    expect(screen.getByRole("button", { name: "Other tab" })).toBeVisible();
    expect(screen.getByRole("alert")).toHaveTextContent(
      "Response could not be displayed",
    );
    expect(view.container.innerHTML).not.toContain("synthetic-private-value");
    broken = false;
    fireEvent.click(screen.getByRole("button", { name: "Try again" }));
    expect(screen.getByText("Recovered panel")).toBeVisible();
  });
});
