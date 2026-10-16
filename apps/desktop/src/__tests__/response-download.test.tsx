import { afterEach, describe, expect, it, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { save } from "@tauri-apps/plugin-dialog";
import { writeFile, writeTextFile } from "@tauri-apps/plugin-fs";
import { ResponseBodyActions } from "@/components/response/response-panel";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("Binary response download", () => {
  it("saves the response that opened the dialog even after switching or rerunning", async () => {
    let select!: (path: string | null) => void;
    vi.mocked(save).mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          select = resolve;
        }),
    );
    const { rerender } = render(
      <ResponseBodyActions body="first binary" bodyBase64="AP+A" />,
    );
    fireEvent.click(screen.getByTitle("response.saveToFile"));
    rerender(<ResponseBodyActions body="new response" bodyBase64="AQID" />);
    select("/synthetic/selected.bin");
    await waitFor(() =>
      expect(writeFile).toHaveBeenCalledWith(
        "/synthetic/selected.bin",
        new Uint8Array([0, 255, 128]),
      ),
    );
    expect(writeTextFile).not.toHaveBeenCalled();
  });
  it("does not write a cancelled download and preserves an empty binary response", async () => {
    vi.mocked(save)
      .mockResolvedValueOnce(null)
      .mockResolvedValueOnce("/synthetic/empty.bin");
    render(<ResponseBodyActions body="empty binary" bodyBase64="" />);
    fireEvent.click(screen.getByTitle("response.saveToFile"));
    await waitFor(() => expect(save).toHaveBeenCalledTimes(1));
    expect(writeFile).not.toHaveBeenCalled();
    fireEvent.click(screen.getByTitle("response.saveToFile"));
    await waitFor(() =>
      expect(writeFile).toHaveBeenCalledWith(
        "/synthetic/empty.bin",
        new Uint8Array(),
      ),
    );
  });
});
