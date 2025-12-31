import { useEffect, useCallback } from "react";

interface ShortcutAction {
  key: string;
  ctrl?: boolean;
  alt?: boolean;
  shift?: boolean;
  meta?: boolean;
  action: () => void;
  description: string;
}

interface UseKeyboardShortcutsOptions {
  enabled?: boolean;
}

/**
 * Hook for handling keyboard shortcuts
 *
 * @example
 * ```tsx
 * useKeyboardShortcuts([
 *   { key: 'n', ctrl: true, action: () => newChat(), description: 'New chat' },
 *   { key: 'Escape', action: () => closeModal(), description: 'Close modal' },
 * ]);
 * ```
 */
export function useKeyboardShortcuts(
  shortcuts: ShortcutAction[],
  options: UseKeyboardShortcutsOptions = {}
) {
  const { enabled = true } = options;

  const handleKeyDown = useCallback(
    (event: KeyboardEvent) => {
      if (!enabled) return;

      // Don't trigger shortcuts when typing in inputs
      const target = event.target as HTMLElement;
      const isInputField =
        target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.isContentEditable;

      for (const shortcut of shortcuts) {
        const keyMatches =
          event.key.toLowerCase() === shortcut.key.toLowerCase();
        const ctrlMatches = shortcut.ctrl ? event.ctrlKey || event.metaKey : !event.ctrlKey && !event.metaKey;
        const altMatches = shortcut.alt ? event.altKey : !event.altKey;
        const shiftMatches = shortcut.shift ? event.shiftKey : !event.shiftKey;
        const metaMatches = shortcut.meta ? event.metaKey : true; // Allow meta to be optional

        // Special keys like Escape should work even in input fields
        const isSpecialKey = ["Escape", "F1", "F2", "F3"].includes(event.key);

        if (
          keyMatches &&
          ctrlMatches &&
          altMatches &&
          shiftMatches &&
          (isSpecialKey || !isInputField || shortcut.ctrl || shortcut.alt)
        ) {
          event.preventDefault();
          shortcut.action();
          return;
        }
      }
    },
    [shortcuts, enabled]
  );

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);
}

/**
 * Format shortcut for display
 */
export function formatShortcut(shortcut: Omit<ShortcutAction, "action">): string {
  const parts: string[] = [];
  const isMac = navigator.platform.toUpperCase().indexOf("MAC") >= 0;

  if (shortcut.ctrl) {
    parts.push(isMac ? "Cmd" : "Ctrl");
  }
  if (shortcut.alt) {
    parts.push(isMac ? "Option" : "Alt");
  }
  if (shortcut.shift) {
    parts.push("Shift");
  }

  // Format special keys
  const keyDisplay =
    shortcut.key === " "
      ? "Space"
      : shortcut.key.length === 1
      ? shortcut.key.toUpperCase()
      : shortcut.key;

  parts.push(keyDisplay);

  return parts.join("+");
}

/**
 * Common keyboard shortcuts configuration
 */
export const commonShortcuts = {
  newChat: { key: "n", ctrl: true, description: "New chat" },
  search: { key: "k", ctrl: true, description: "Search" },
  settings: { key: ",", ctrl: true, description: "Open settings" },
  closeModal: { key: "Escape", description: "Close modal" },
  submit: { key: "Enter", ctrl: true, description: "Send message" },
  toggleSidebar: { key: "b", ctrl: true, description: "Toggle sidebar" },
  help: { key: "?", shift: true, description: "Show help" },
  navigateModels: { key: "1", ctrl: true, description: "Go to Models" },
  navigateChat: { key: "2", ctrl: true, description: "Go to Chat" },
  navigateSettings: { key: "3", ctrl: true, description: "Go to Settings" },
} as const;

export default useKeyboardShortcuts;
