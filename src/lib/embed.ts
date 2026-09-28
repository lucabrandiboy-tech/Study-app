/** True when the app runs inside a hosted page frame (e.g. a shared claude.ai link), where pop-up dialogs, downloads and file pickers are blocked. */
export const embedded = (() => { try { return window.self !== window.top; } catch { return true; } })();

/** confirm() that still works when pop-up dialogs are blocked (hosted link): the click itself counts as the confirmation. */
export const ask = (msg: string) => (embedded ? true : window.confirm(msg));
