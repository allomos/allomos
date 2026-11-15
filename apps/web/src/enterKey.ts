// Task-A00-jackal794: Enter that only confirms an input method's word (Chinese, Japanese) must
// not save a form. WebKit, which runs the macOS app, ends the composition before it sends that
// Enter's keydown, so `isComposing` is already false there; such a keydown still carries keyCode
// 229 ("the input method handled this key"). Checked together, both browsers are covered.

/** The parts of a keydown event this check reads (a React or DOM KeyboardEvent's native event). */
export type EnterKeyEvent = { key: string; isComposing: boolean; keyCode: number };

/** True for an Enter the person meant for the form, not one that confirmed an input method's word. */
export function isPlainEnter(e: EnterKeyEvent): boolean {
  return e.key === "Enter" && !e.isComposing && e.keyCode !== 229;
}
