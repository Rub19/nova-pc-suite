import { invoke } from "@tauri-apps/api/core";
import { Undo2 } from "lucide-react";
import { useEffect, useState } from "react";
import { formatDateTime, Measure, Undo } from "../lib";
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
  return (
    <>
      <Head title="Journal" intro="Tout ce que Nova a changé sur ce PC, avec les mesures avant et après. Ce journal reste sur ce PC." />
      {entries && !entries.length && <Empty>Nova n'a encore rien changé sur ce PC.</Empty>}
      {!!entries?.length && (
        <ol className="journal">
          {entries.map((e, i) => (
            <li key={`${e.timestamp}-${i}`} className={e.ok ? "" : "failed"}>
              <time className="tabular">{formatDateTime(e.timestamp * 1000)}</time>
              <div className="grow">
                <strong>{e.ok ? e.summary ?? e.action : `Échec : ${e.summary ?? e.action}`}</strong>
                {e.restorePoint && <p>Point de restauration : {e.restorePoint.description}</p>}
                <Measures items={e.measures} />
              </div>
              {stillUndoable(entries, i) && e.undo && !undone.includes(e.timestamp) && (
                <button onClick={() => void undo(e)}>
                  <Undo2 size={15} aria-hidden /> {e.undo.label}
                </button>
              )}
            </li>
          ))}
        </ol>
      )}
    </>
  );
}
