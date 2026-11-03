import { useEffect, useState } from "react";
import { ActionResult, formatBytes, plural } from "../lib";
import { call, Elapsed, Failure, Head, Measures, Skeleton, useAction, useApp, useResource, AdminButton } from "../ui";

type Category = { id: string; label: string; detail: string; bytes: number; files: number; selected: boolean; needsAdmin: boolean; blocked: boolean };
type ScanResult = { items: Category[]; freeBytes: number };
type Cleaned = ActionResult & {
  removedBytes?: number;
  skippedFiles?: number;
  results?: { id: string; label: string; bytes: number; files: number; skipped: number; note: string | null }[];
};

export default function Nettoyage() {
  const { ask, elevate } = useApp();
  const scan = useResource<ScanResult>("cleanup", () => call("cleanup", "scan"));
  const { busy, result, run } = useAction();
  const [chosen, setChosen] = useState<string[]>([]);
  const [touched, setTouched] = useState(false);
  const items = scan.data?.items ?? [];
  const usable = items.filter((c) => !c.needsAdmin && !c.blocked && c.bytes > 0);
  useEffect(() => {
    if (!touched) setChosen(usable.filter((c) => c.selected).map((c) => c.id));
  }, [scan.data]);
  const picked = usable.filter((c) => chosen.includes(c.id));
  const total = picked.reduce((sum, c) => sum + c.bytes, 0);
  const locked = items.some((c) => c.needsAdmin);
  const clean = async () => {
    const yes = await ask({
      title: `Supprimer ${formatBytes(total)}`,
      danger: true,
      confirm: "Supprimer",
      body: (
        <>
          <p>Ces fichiers seront supprimés définitivement. Il n'y a pas d'annulation, mais aucun document personnel n'est concerné.</p>
          <ul className="plain">
            {picked.map((c) => (
              <li key={c.id}>{c.label} : {formatBytes(c.bytes)}</li>
            ))}
          </ul>
        </>
      ),
    });
    if (!yes) return;
    const r = await run("cleanup", "clean", { ids: chosen.filter((id) => picked.some((c) => c.id === id)) });
    if (r) void scan.reload();
  };
  const done = result as Cleaned | null;
  return (
    <>
      <Head title="Nettoyage" intro="Seulement ce que Windows et vos applications savent recréer. Vos documents ne sont jamais touchés." />
      {scan.error && <Failure text={scan.error} retry={() => void scan.reload()} />}
      {!scan.data && scan.loading && <Skeleton rows={7} />}
      {scan.data && (
        <>
          <div className="total">
            <div>
              <strong className="big tabular">{formatBytes(total)}</strong>
              <span>à libérer sur {formatBytes(scan.data.freeBytes)} déjà libres</span>
            </div>
            <div className="head-actions">
              <button disabled={scan.loading || !!busy} onClick={() => void scan.reload()}>{scan.loading ? "Analyse…" : "Analyser à nouveau"}</button>
              <button className="primary" disabled={!picked.length || !!busy} onClick={() => void clean()}>Nettoyer</button>
            </div>
          </div>
          {busy && <Elapsed label="Suppression en cours." />}
          {done && (
            <div className="ticket">
              <p><strong>{formatBytes(done.removedBytes ?? 0)} supprimés.</strong>{done.skippedFiles ? ` ${plural(done.skippedFiles, "fichier ouvert laissé", "fichiers ouverts laissés")} en place.` : ""}</p>
              <Measures items={done.measures} />
              <ul className="plain">
                {done.results?.filter((r) => r.note).map((r) => <li key={r.id}>{r.label} : {r.note}</li>)}
              </ul>
            </div>
          )}
          {locked && (
            <p className="note">
              Certains emplacements de Windows demandent les droits administrateur.
              <AdminButton className="link" onRun={() => void elevate()}>Les inclure</AdminButton>
            </p>
          )}
          <ul className="panel">
            {items.map((c) => {
              const off = c.needsAdmin || c.blocked || c.bytes === 0;
              return (
                <li className={`row ${off ? "dim" : ""}`} key={c.id}>
                  <label className="check grow">
                    <input
                      type="checkbox"
                      disabled={off || !!busy}
                      checked={!off && chosen.includes(c.id)}
                      onChange={(e) => {
                        setTouched(true);
                        setChosen((s) => (e.target.checked ? [...s, c.id] : s.filter((x) => x !== c.id)));
                      }}
                    />
                    <span>
                      <strong>{c.label}</strong>
                      <small>{c.blocked ? "Reporté : une mise à jour attend un redémarrage." : c.detail}</small>
                    </span>
                  </label>
                  <span className="size tabular">{c.needsAdmin ? "droits requis" : c.bytes ? formatBytes(c.bytes) : "vide"}</span>
                </li>
              );
            })}
          </ul>
        </>
      )}
    </>
  );
}
