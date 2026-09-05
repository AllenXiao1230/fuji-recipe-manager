import { describe, expect, it } from "vitest";
import { isSafeExternalUrl } from "./externalUrl";

describe("isSafeExternalUrl", () => {
  it("permits only normal web attribution links", () => {
    expect(isSafeExternalUrl("https://fujifilm-x.com/en-us/")).toBe(true);
    expect(isSafeExternalUrl("http://localhost:5173/reference")).toBe(true);
    expect(isSafeExternalUrl("file:///tmp/camera-notes")).toBe(false);
    expect(isSafeExternalUrl("javascript:alert(1)")).toBe(false);
    expect(isSafeExternalUrl(undefined)).toBe(false);
  });
});
