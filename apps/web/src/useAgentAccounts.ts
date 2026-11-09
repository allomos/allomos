// The extra accounts of the terminal agents, read from the app (Task-A03-oven548): the dialog,
// the monitor's list and its status bar use it to show one row or label per account.

import { useCallback, useEffect, useState } from "react";
import type { Messenger } from "./messenger";
import { rememberAccounts } from "./agentAccounts";
import type { AgentAccounts } from "./monitorFlow";

export function useAgentAccounts(messenger: Messenger): { accounts: AgentAccounts | null; reload: () => Promise<void> } {
  const [accounts, setAccounts] = useState<AgentAccounts | null>(null);
  const reload = useCallback(async () => {
    try {
      const list = await messenger.agentAccounts();
      rememberAccounts(list.accounts);
      setAccounts(list);
    } catch {
      // Without the list every agent shows as one row, as before accounts existed.
    }
  }, [messenger]);
  useEffect(() => {
    void reload();
  }, [reload]);
  return { accounts, reload };
}
