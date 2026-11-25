// The Monitor ready tasks dialog's model and reasoning effort choices (A03-alto709): the online
// model list, read when the dialog opens and at Refresh, and for each checked agent a Model and
// an Effort list whose first choice, Default, is chosen until the person picks another.

import { useCallback, useEffect, useState } from "react";
import { useI18n } from "./i18n";
import type { Messenger } from "./messenger";
import { listNotice, optionsFor } from "./agentModelsFlow";
import type { ModelsRead, Picks } from "./agentModelsFlow";
import type { TerminalAgent } from "./monitorFlow";

/** The list: read once when the dialog opens; `refresh` reads it again. */
export function useAgentModels(messenger: Messenger) {
  const [read, setRead] = useState<ModelsRead | null>(null);
  const [reading, setReading] = useState(true);
  const refresh = useCallback(() => {
    setReading(true);
    return messenger
      .agentModels()
      .then(setRead, () => setRead({ agents: null, from: "none", failed: "fetch", url: "" }))
      .finally(() => setReading(false));
  }, [messenger]);
  useEffect(() => {
    void refresh();
  }, [refresh]);
  return { read, reading, refresh };
}

/** The list's line: where the choices come from, Refresh, and why the web could not be read. */
export function ModelListLine({ read, reading, onRefresh }: { read: ModelsRead | null; reading: boolean; onRefresh: () => void }) {
  const { t } = useI18n();
  const notice = listNotice(read);
  return (
    <p className="muted small monitor-models-line">
      {t("monitor.models_note")}{" "}
      <button type="button" className="link" disabled={reading} onClick={onRefresh}>
        {reading ? t("monitor.models_reading") : t("monitor.models_refresh")}
      </button>
      {notice && !reading && <span className="hint own-line">{t(notice)}</span>}
    </p>
  );
}

/** One agent's Model and Effort lists; a list the online list offers nothing for shows Default
 * only. The efforts offered follow the chosen model (A03-pony586): a model that takes no reasoning
 * effort leaves Default as the only choice, and an effort the new model does not offer falls back
 * to Default. */
export function ModelPicker({ agent, name, read, picks, onChange }: { agent: TerminalAgent; name: string; read: ModelsRead | null; picks: Picks; onChange: (picks: Picks) => void }) {
  const { t } = useI18n();
  const { models, efforts } = optionsFor(read, agent, picks.model);
  const model = models.some((m) => m.value === picks.model) ? picks.model! : "";
  const effort = efforts.includes(picks.effort ?? "") ? picks.effort! : "";
  return (
    <span className="monitor-sessions monitor-models small" data-agent={agent}>
      <span className="muted">{t("monitor.model_label")}</span>{" "}
      <select
        aria-label={t("monitor.model_aria", { agent: name })}
        value={model}
        onChange={(e) => {
          const next = optionsFor(read, agent, e.target.value || undefined);
          const keep = next.efforts.includes(picks.effort ?? "");
          onChange({ model: e.target.value || undefined, ...(keep && picks.effort ? { effort: picks.effort } : {}) });
        }}
      >
        <option value="">{t("monitor.choice_default")}</option>
        {models.map((m) => (
          <option key={m.value} value={m.value}>
            {m.label}
          </option>
        ))}
      </select>{" "}
      <span className="muted">{t("monitor.effort_label")}</span>{" "}
      <select aria-label={t("monitor.effort_aria", { agent: name })} value={effort} onChange={(e) => onChange({ ...picks, effort: e.target.value || undefined })}>
        <option value="">{t("monitor.choice_default")}</option>
        {efforts.map((e) => (
          <option key={e} value={e}>
            {e}
          </option>
        ))}
      </select>
    </span>
  );
}
