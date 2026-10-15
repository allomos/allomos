// Which task the task pane shows after a click (Task-A01-46). Plain logic, tested in tests/paneFlow.test.ts.

/**
 * The task the pane shows after a task is clicked in the list.
 * Clicking the task already shown keeps it shown (it used to close the pane);
 * clicking another task switches to it. The pane closes only with its own × or Esc.
 */
export function afterClick(_shown: string | null, clicked: string): string {
  return clicked;
}
