import { invoke } from "@tauri-apps/api/core";
import { Undo2 } from "lucide-react";
import { Fragment, useEffect, useState } from "react";
import { formatDateTime, journalDay, Measure, Undo } from "../lib";
import { call, Empty, errorText, Head, Measures, useApp } from "../ui";

type Entry = { timestamp: number; module: string; action: string; ok: boolean; summary?: string; measures?: Measure[]; undo?: Undo; restorePoint?: { description: string } };

const undoTarget = (u: Undo) => {
  const p = u.payload as { source?: string; name?: string } | undefined;
  return [u.module, u.action, p?.source, p?.name].join("|");
};
const stillUndoable = (entries: Entry[], i: number) => {
  const { undo, ok, timestamp } = entries[i];
  if (!ok || !undo) return false;
  if (undo.action === "reboot-cancel" && Date.now() / 1000 - timestamp > 60) return false;
  return !entries.slice(0, i).some((e) => e.ok && e.undo && undoTarget(e.undo) === undoTarget(undo));
};

export default function Journal() {
  const { toast } = useApp();
  const [entries, setEntries] = useState<Entry[] | null>(null);
  const [undone, setUndone] = useState<number[]>([]);
  const [filter, setFilter] = useState<"all" | "failed" | "undo">("all");
  const load = () =>
    invoke<Entry[]>("journal")
      .then(setEntries)
      .catch(() => setEntries([]));
  useEffect(() => {
    void load();
  }, []);
  const undo = async (entry: Entry) => {
    if (!entry.undo) return;
    try {
      const r = await call(entry.undo.module, entry.undo.action, entry.undo.payload);
      toast(r.summary ?? "Annulé");
      setUndone((u) => [...u, entry.timestamp]);
      await load();
    } catch (e) {
      toast(errorText(e), "alert");
    }
  };
  const canUndo = (i: number) => {
    const entry = entries![i];
    return stillUndoable(entries!, i) && !!entry.undo && !undone.includes(entry.timestamp);
  };
  const rows = (entries ?? []).map((entry, index) => ({ entry, index })).filter(({ entry, index }) => (filter === "all" ? true : filter === "failed" ? !entry.ok : canUndo(index)));
  const copy = async () => {
    const text = rows.map(({ entry }) => `${formatDateTime(entry.timestamp * 1000)} : ${entry.ok ? "" : "Échec : "}${entry.summary ?? entry.action}`).join("\n");
    try {
      await navigator.clipboard.writeText(text);
      toast("Journal copié");
    } catch (e) {
      toast(errorText(e), "alert");
    }
  };
  return (
    <>
      <Head title="Journal" intro="Tout ce que Nova a changé sur ce PC, avec les mesures avant et après. Ce journal reste sur ce PC.">
        {!!entries?.length && <button disabled={!rows.length} onClick={() => void copy()}>Copier</button>}
      </Head>
      {!!entries?.length && (
        <div className="filters" role="group" aria-label="Filtrer le journal">
          {([["all", "Tout"], ["failed", "Échecs"], ["undo", "Annulables"]] as const).map(([id, label]) => (
            <button key={id} className={filter === id ? "on" : ""} aria-pressed={filter === id} onClick={() => setFilter(id)}>{label}</button>
          ))}
        </div>
      )}
      {entries && !entries.length && <Empty>Nova n'a encore rien changé sur ce PC.</Empty>}
      {!!entries?.length && !rows.length && <Empty>Rien pour ce filtre.</Empty>}
      {rows.length > 0 && (
        <ol className="journal">
          {rows.map(({ entry: e, index: i }, position) => (
            <Fragment key={`${e.timestamp}-${i}`}>
            {(position === 0 || journalDay(rows[position - 1].entry.timestamp) !== journalDay(e.timestamp)) && <li className="journal-day" aria-hidden>{journalDay(e.timestamp)}</li>}
            <li className={e.ok ? "" : "failed"}>
              <time className="tabular">{formatDateTime(e.timestamp * 1000)}</time>
              <div className="grow">
                <strong>{e.ok ? e.summary ?? e.action : `Échec : ${e.summary ?? e.action}`}</strong>
                {e.restorePoint && <p>Point de restauration : {e.restorePoint.description}</p>}
                <Measures items={e.measures} />
              </div>
              {canUndo(i) && e.undo && (
                <button onClick={() => void undo(e)}>
                  <Undo2 size={15} aria-hidden /> {e.undo.label}
                </button>
              )}
            </li>
            </Fragment>
          ))}
        </ol>
      )}
    </>
  );
}
