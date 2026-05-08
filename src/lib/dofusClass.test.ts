import { describe, expect, it } from "vitest";
import { KNOWN_DOFUS_CLASSES, avatarUrlFor, classDisplayName } from "./dofusClass";

describe("avatarUrlFor", () => {
  it("returns the avatar path for every known class", () => {
    for (const slug of KNOWN_DOFUS_CLASSES) {
      expect(avatarUrlFor(slug)).toBe(`/avatars/${slug}.jpg`);
    }
  });

  it("returns null for unknown or missing class", () => {
    expect(avatarUrlFor(null)).toBeNull();
    expect(avatarUrlFor(undefined)).toBeNull();
    expect(avatarUrlFor("")).toBeNull();
    expect(avatarUrlFor("not-a-class")).toBeNull();
  });
});

describe("classDisplayName", () => {
  it("returns localized names for known classes", () => {
    expect(classDisplayName("cra")).toBe("Crâ");
    expect(classDisplayName("feca")).toBe("Féca");
    expect(classDisplayName("iop")).toBe("Iop");
  });

  it("falls back to the slug for unmapped classes", () => {
    expect(classDisplayName("custom")).toBe("custom");
  });

  it("returns null for missing class", () => {
    expect(classDisplayName(null)).toBeNull();
    expect(classDisplayName(undefined)).toBeNull();
  });
});
