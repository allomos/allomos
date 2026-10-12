// The remembered keys of one collapsible list (Task-A01-26), per project, in the window's storage.

import { useEffect, useState } from "react";
import { foldStorageKey, parseKeys } from "./foldFlow";
import type { FoldList } from "./foldFlow";

export function useFold(root: string, list: FoldList): [string[], (change: (keys: string[]) => string[]) => void] {
  const read = () => {
    try {
      return parseKeys(localStorage.getItem(foldStorageKey(root, list)));
    } catch {
      return [];
    }
  };
  const [state, setState] = useState(() => ({ root, keys: read() }));
  // Another project opened in this window: its own remembered keys.
  useEffect(() => {
    if (state.root !== root) setState({ root, keys: read() });
    // `read` only depends on root and list.
  }, [root, list]);
  const keys = state.root === root ? state.keys : read();
  const change = (f: (keys: string[]) => string[]) =>
    setState((now) => {
      const next = f(now.root === root ? now.keys : read());
      try {
        localStorage.setItem(foldStorageKey(root, list), JSON.stringify(next));
      } catch {
        /* storage unavailable: the choice lasts until the window closes */
      }
      return { root, keys: next };
    });
  return [keys, change];
}
