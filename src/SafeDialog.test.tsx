import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SafeDialog } from "./SafeDialog";

describe("SafeDialog", () => {
  afterEach(cleanup);
  it("closes on Escape only when closing is enabled", () => {
    const onClose = vi.fn();
    render(
      <SafeDialog labelledBy="dialog-title" onClose={onClose}>
        <h2 id="dialog-title">Protected operation</h2>
        <button type="button">Cancel</button>
      </SafeDialog>,
    );
    fireEvent.keyDown(document, { key: "Escape" });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("keeps a camera operation open while it is busy", () => {
    const onClose = vi.fn();
    render(
      <SafeDialog labelledBy="dialog-title" onClose={onClose} closeDisabled>
        <h2 id="dialog-title">Protected operation</h2>
        <button type="button">Cancel</button>
      </SafeDialog>,
    );
    fireEvent.keyDown(document, { key: "Escape" });
    expect(onClose).not.toHaveBeenCalled();
    expect(screen.getByRole("dialog").getAttribute("aria-modal")).toBe("true");
  });
});
