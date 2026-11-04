import { invoke } from "@tauri-apps/api/core";
import { Lock } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { arr, bootComparison, formatBytes, formatDate, plural } from "../lib";
import { call, errorText, Failure, Head, Section, Skeleton, useApp, useResource } from "../ui";

type Entry = { source: string; sourceLabel: string; name: string; enabled: boolean; path: string | null; publisher: string | null; exists: boolean; ramBytes: number; running: boolean; protected: boolean; manageable: boolean };
type Boots = { available: boolean; items: { date: string; ms: number }[] };

function BootChart({ boots, changes }: { boots: Boots; changes: { timestamp: number }[] }) {
  const { elevate } = useApp();
  if (!boots.available) {
    return (
      <p className="note">
        Le temps de démarrage est enregistré par Windows, mais il faut les droits administrateur pour le lire.
        <button className="link" onClick={() => void elevate()}>Relancer en administrateur</button>
      </p>
    );
  }
  if (!arr(boots.items).length) return <p className="note">Windows n'a pas encore enregistré de temps de démarrage.</p>;
  const ordered = [...arr(boots.items)].reverse();
  const top = Math.max(...ordered.map((b) => b.ms));
  const compare = bootComparison(boots.items, changes);
  return (
    <>
      <ol className="bars" aria-label="Durée des derniers démarrages">
        {ordered.map((b) => (
          <li key={b.date}>
            <span className="bar-value tabular">{Math.round(b.ms / 1000)} s</span>
            <span className="bar-fill" style={{ height: `${(b.ms / top) * 100}%` }} />
            <span className="bar-date">{formatDate(b.date).replace(/ \d{4}$/, "")}</span>
          </li>
        ))}
      </ol>
      {compare && (
        <p className="note">
          Depuis votre dernier changement : {Math.round(compare.before / 1000)} s › <strong>{Math.round(compare.after / 1000)} s</strong> en moyenne.
        </p>
      )}
    </>
  );
}

export default function Demarrage() {
  const { toast, isAdmin, elevate } = useApp();
  const list = useResource<{ items: Entry[] }>("startup", () => call("startup", "list"));
  const boots = useResource<Boots>("boots", () => call("diag", "boots"));
  const [pending, setPending] = useState<string | null>(null);
  const [changes, setChanges] = useState<{ timestamp: number }[]>([]);
  const refreshChanges = () =>
    invoke<{ timestamp: number; module: string; ok: boolean }[]>("journal")
      .then((all) => setChanges(all.filter((e) => e.module === "startup" && e.ok)))
      .catch(() => undefined);
  useEffect(() => {
    void refreshChanges();
    const listener = (e: Event) => {
      if ((e as CustomEvent).detail === "startup") void list.reload();
    };
    window.addEventListener("nova:changed", listener);
    return () => window.removeEventListener("nova:changed", listener);
  }, []);
  const items = arr(list.data?.items);
  const active = useMemo(() => items.filter((i) => i.enabled), [items]);
  const key = (i: Entry) => `${i.source}|${i.name}`;
  const toggle = async (i: Entry) => {
    setPending(key(i));
    try {
      const r = await call("startup", "set", { source: i.source, name: i.name, enabled: !i.enabled });
      toast(r.summary ?? "Fait", "ok", r.undo);
      await list.reload();
      void refreshChanges();
    } catch (e) {
      toast(errorText(e), "alert");
    } finally {
      setPending(null);
    }
  };
  return (
    <>
      <Head title="Démarrage" intro="Chaque programme lancé avec Windows allonge le démarrage et occupe de la mémoire. Le désactiver ne le désinstalle pas : vous pouvez toujours le lancer à la main ou le réactiver ici." />
      {boots.data && (
        <Section title="Durée des derniers démarrages">
          <BootChart boots={boots.data} changes={changes} />
        </Section>
      )}
      {list.error && <Failure text={list.error} retry={() => void list.reload()} />}
      {!list.data && list.loading && <Skeleton rows={7} />}
      {list.data && (
        <Section
          title={`${plural(active.length, "programme lancé", "programmes lancés")} au démarrage`}
          note={isAdmin ? undefined : "Les éléments communs à tous les utilisateurs demandent les droits administrateur."}
          actions={<button disabled={list.loading} onClick={() => void list.reload()}>{list.loading ? "Lecture…" : "Actualiser"}</button>}
        >
          <ul className="panel">
            {items.map((i) => (
              <li className={`row ${i.enabled ? "" : "dim"}`} key={key(i)}>
                <div className="grow">
                  <strong>{i.name.replace(/^.*\\/, "")}</strong>
                  <p>
                    {[i.publisher ?? "Éditeur inconnu", i.running ? `${formatBytes(i.ramBytes)} de mémoire en ce moment` : "", i.sourceLabel].filter(Boolean).join(", ")}
                  </p>
                  {!i.exists && <p className="warn-text">Le programme n'existe plus sur le disque. Cette entrée ne sert à rien.</p>}
                </div>
                {i.protected ? (
                  <span className="tag" title="Nova ne désactive pas les logiciels de sécurité"><Lock size={13} aria-hidden /> Protégé</span>
                ) : i.manageable ? (
                  <button
                    role="switch"
                    className="switch"
                    aria-checked={i.enabled}
                    aria-label={`${i.name} au démarrage`}
                    disabled={!!pending}
                    onClick={() => void toggle(i)}
                  >
                    <span />
                  </button>
                ) : (
                  <button className="link" onClick={() => void elevate()}>Droits requis</button>
                )}
              </li>
            ))}
          </ul>
        </Section>
      )}
    </>
  );
}
