import { describe, expect, it } from "vitest";

import {
  captureShortcutFromKeyboardEvent,
  formatRecordingShortcutForText,
  getRecordingShortcutDisplayTokens
} from "./presenter";

const baseKeyboardEvent = {
  key: "",
  code: "",
  ctrlKey: false,
  metaKey: false,
  altKey: false,
  shiftKey: false
};

describe("shortcut presenter", () => {
  it("captures primary modifier shortcuts with a letter key", () => {
    expect(
      captureShortcutFromKeyboardEvent({
        ...baseKeyboardEvent,
        key: "a",
        code: "KeyA",
        ctrlKey: true,
        altKey: true
      })
    ).toEqual({
      kind: "shortcut",
      value: "CommandOrControl+Alt+A"
    });
  });

  it("allows function keys without modifiers", () => {
    expect(
      captureShortcutFromKeyboardEvent({
        ...baseKeyboardEvent,
        key: "F8",
        code: "F8"
      })
    ).toEqual({
      kind: "shortcut",
      value: "F8"
    });
  });

  it("asks for one more key when only modifiers are pressed", () => {
    expect(
      captureShortcutFromKeyboardEvent({
        ...baseKeyboardEvent,
        key: "Shift",
        code: "ShiftLeft",
        shiftKey: true
      })
    ).toEqual({
      kind: "incomplete",
      message: "Add one more key to finish the shortcut."
    });
  });

  it("requires modifiers for regular typing keys", () => {
    expect(
      captureShortcutFromKeyboardEvent({
        ...baseKeyboardEvent,
        key: "a",
        code: "KeyA"
      })
    ).toEqual({
      kind: "incomplete",
      message: "Add Ctrl/Cmd, Alt, or Shift so the shortcut does not conflict with normal typing."
    });
  });

  it("formats stored shortcuts for the current platform", () => {
    expect(getRecordingShortcutDisplayTokens("CommandOrControl+Alt+A", "Linux x86_64")).toEqual([
      "Ctrl",
      "Alt",
      "A"
    ]);
    expect(formatRecordingShortcutForText("CommandOrControl+Alt+A", "MacIntel")).toBe(
      "Cmd + Option + A"
    );
  });
});
