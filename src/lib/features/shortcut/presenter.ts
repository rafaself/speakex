export interface ShortcutKeyboardEventLike {
  key: string;
  code?: string;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
}

export type ShortcutCaptureResult =
  | { kind: "ignored" }
  | { kind: "incomplete"; message: string }
  | { kind: "invalid"; message: string }
  | { kind: "shortcut"; value: string };

const modifierKeys = new Set(["Alt", "AltGraph", "Control", "Fn", "Meta", "OS", "Shift"]);
const namedKeys = new Map([
  ["ArrowDown", "ArrowDown"],
  ["ArrowLeft", "ArrowLeft"],
  ["ArrowRight", "ArrowRight"],
  ["ArrowUp", "ArrowUp"],
  ["Backspace", "Backspace"],
  ["Delete", "Delete"],
  ["End", "End"],
  ["Enter", "Enter"],
  ["Escape", "Escape"],
  ["Home", "Home"],
  ["Insert", "Insert"],
  ["PageDown", "PageDown"],
  ["PageUp", "PageUp"],
  ["Spacebar", "Space"],
  ["Tab", "Tab"],
  [" ", "Space"]
]);
const displayTokenLabels = new Map([
  ["Alt", "Alt"],
  ["ArrowDown", "Down"],
  ["ArrowLeft", "Left"],
  ["ArrowRight", "Right"],
  ["ArrowUp", "Up"],
  ["Backspace", "Backspace"],
  ["CommandOrControl", "Ctrl"],
  ["Ctrl", "Ctrl"],
  ["Delete", "Delete"],
  ["End", "End"],
  ["Enter", "Enter"],
  ["Escape", "Esc"],
  ["Home", "Home"],
  ["Insert", "Ins"],
  ["Meta", "Meta"],
  ["PageDown", "PgDn"],
  ["PageUp", "PgUp"],
  ["Shift", "Shift"],
  ["Space", "Space"],
  ["Tab", "Tab"]
]);

export function captureShortcutFromKeyboardEvent(
  event: ShortcutKeyboardEventLike
): ShortcutCaptureResult {
  const modifiers = buildModifierTokens(event);
  const primaryKey = normalizePrimaryKey(event);

  if (primaryKey === null) {
    return modifierKeys.has(event.key)
      ? {
          kind: "incomplete",
          message: "Add one more key to finish the shortcut."
        }
      : {
          kind: "invalid",
          message: "Use a letter, number, function key, or navigation key."
        };
  }

  if (modifiers.length === 0 && !/^F\d{1,2}$/u.test(primaryKey)) {
    return {
      kind: "incomplete",
      message: "Add Ctrl/Cmd, Alt, or Shift so the shortcut does not conflict with normal typing."
    };
  }

  return {
    kind: "shortcut",
    value: [...modifiers, primaryKey].join("+")
  };
}

export function getRecordingShortcutDisplayTokens(
  shortcut: string | null | undefined,
  platform = getCurrentPlatform()
): string[] {
  const normalizedShortcut = shortcut?.trim();

  if (!normalizedShortcut) {
    return [];
  }

  const applePlatform = isApplePlatform(platform);

  return normalizedShortcut
    .split("+")
    .map((token) => formatShortcutToken(token, applePlatform))
    .filter((token) => token.length > 0);
}

export function formatRecordingShortcutForText(
  shortcut: string | null | undefined,
  platform = getCurrentPlatform()
): string | null {
  const tokens = getRecordingShortcutDisplayTokens(shortcut, platform);

  return tokens.length > 0 ? tokens.join(" + ") : null;
}

export function isApplePlatform(platform = getCurrentPlatform()): boolean {
  return /Mac|iPad|iPhone/iu.test(platform);
}

function buildModifierTokens(event: ShortcutKeyboardEventLike): string[] {
  const tokens: string[] = [];

  if (event.ctrlKey && event.metaKey) {
    tokens.push("Ctrl", "Meta");
  } else if (event.ctrlKey || event.metaKey) {
    tokens.push("CommandOrControl");
  }

  if (event.altKey) {
    tokens.push("Alt");
  }

  if (event.shiftKey) {
    tokens.push("Shift");
  }

  return tokens;
}

function normalizePrimaryKey(event: ShortcutKeyboardEventLike): string | null {
  if (modifierKeys.has(event.key)) {
    return null;
  }

  if (typeof event.code === "string") {
    const keyCodeMatch = /^Key([A-Z])$/u.exec(event.code);

    if (keyCodeMatch) {
      return keyCodeMatch[1];
    }

    const digitCodeMatch = /^Digit([0-9])$/u.exec(event.code);

    if (digitCodeMatch) {
      return digitCodeMatch[1];
    }
  }

  if (/^F\d{1,2}$/iu.test(event.key)) {
    return event.key.toUpperCase();
  }

  if (namedKeys.has(event.key)) {
    return namedKeys.get(event.key) ?? null;
  }

  if (/^[A-Z0-9]$/iu.test(event.key)) {
    return event.key.toUpperCase();
  }

  return null;
}

function formatShortcutToken(token: string, applePlatform: boolean): string {
  if (token === "CommandOrControl") {
    return applePlatform ? "Cmd" : "Ctrl";
  }

  if (token === "Meta") {
    return applePlatform ? "Cmd" : "Meta";
  }

  if (token === "Alt") {
    return applePlatform ? "Option" : "Alt";
  }

  return displayTokenLabels.get(token) ?? token;
}

function getCurrentPlatform(): string {
  return typeof navigator === "undefined" ? "" : navigator.platform;
}
