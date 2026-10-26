import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Activity, ShieldCheck, Wifi, MemoryStick } from "lucide-react";
import {
  memoryDecision,
  type MemorySample,
  type MemoryTarget,
} from "./memory-policy";
type Row = Record<string, unknown>;
type Reply = { ok: boolean; data?: Row; error?: string };
const list = (v: unknown): Row[] =>
  Array.isArray(v)
    ? (v.filter((x) => x && typeof x === "object") as Row[])
    : [];
const size = (v: unknown) =>
  typeof v === "number"
    ? `${(v / 1024 ** 2).toLocaleString("fr-FR", { maximumFractionDigits: 0 })} Mio`
    : "Indisponible";
const errors: Record<string, string> = {
  MemoryTargetUnavailable:
    "Ce processus a changé, s’est arrêté ou ne peut pas être modifié.",
  ForegroundApplicationSkipped:
    "L’application au premier plan a été conservée.",
  MemoryTrimFailed: "Windows a refusé la réduction du working set.",
  ProcessHasNoClosableWindow:
    "Cette application n’a pas de fenêtre disponible pour une fermeture normale. Aucun arrêt forcé n’a été effectué.",
  MemoryDiagnosticUnavailable: "Mesures mémoire indisponibles.",
  NetworkDiagnosticUnavailable:
    "Certains diagnostics réseau sont indisponibles. Vérifiez les permissions Windows.",
};
async function call(
  module: string,
  action: string,
  payload: Row = {},
  confirmed = false,
) {
  const r = await invoke<Reply>("extended_action", {
    module,
    action,
    payload,
    confirmed,
  });
  if (!r.ok)
    throw new Error(
      errors[r.error ?? ""] ?? r.error ?? "Opération indisponible",
    );
  return r.data ?? {};
}

export function NetworkTools() {
  const [busy, setBusy] = useState(false),
    [result, setResult] = useState<Row>(),
    [error, setError] = useState("");
  const guard = useRef(false);
  async function run(action: string) {
    if (guard.current) return;
    guard.current = true;
    setBusy(true);
    setError("");
    try {
      setResult(await call("network", action));
    } catch (e) {
      setError(String(e));
    } finally {
      guard.current = false;
      setBusy(false);
    }
  }
  return (
    <section className="workbench">
      <div className="info-strip">
        <Wifi size={24} />
        <div>
          <strong>Comprendre les lenteurs et les connexions</strong>
          <span>
            Diagnostic local, suivi des compteurs et tests courts. Les résultats
            permettent de repérer des indices, sans conclure automatiquement à
            une fuite de données.
          </span>
        </div>
      </div>
      <div className="feature-grid">
        {[
          [
            "network-audit",
            "État du réseau",
            "Interfaces, DNS configurés, passerelles, proxy et indices de VPN. Les compteurs d’erreurs sont cumulés.",
          ],
          [
            "network-connections",
            "Connexions et ports ouverts",
            "Connexions TCP et processus associés, ports en écoute sur toutes les interfaces. Aucun trafic réseau n’est intercepté.",
          ],
          [
            "network-probe",
            "Latence, pertes & DNS",
            "Envoie 5 pings à la passerelle et à 1.1.1.1 (Cloudflare), puis résout example.com via le DNS configuré. Ces services reçoivent le trafic de test.",
          ],
        ].map(([action, title, description]) => (
          <article className="feature" key={action}>
            <h3>{title}</h3>
            <p>{description}</p>
            <button disabled={busy} onClick={() => void run(action)}>
              Lancer le diagnostic
            </button>
          </article>
        ))}
      </div>
      {busy && <p role="status">Diagnostic réseau en cours…</p>}
      {error && (
        <p role="alert" className="alert error">
          {error}
        </p>
      )}
      {result && (
        <section className="panel">
          <h2>Résultats du diagnostic</h2>
          {typeof result.leakStatus === "string" && <p>{result.leakStatus}</p>}
          {typeof result.limitations === "string" && (
            <p>{result.limitations}</p>
          )}
          {typeof result.interpretation === "string" && (
            <p>{result.interpretation}</p>
          )}
          {list(result.probes).map((p, i) => (
            <div className="history-row" key={i}>
              <strong>{String(p.target)}</strong>
              <span>
                {String(p.averageMs ?? "—")} ms moyens ·{" "}
                {String(p.jitterMs ?? "—")} ms de variation ·{" "}
                {String(p.lossPercent)} % sans réponse ({String(p.received)}/5)
              </span>
            </div>
          ))}
          {"dnsResolved" in result && (
            <p>
              Résolution DNS :{" "}
              {result.dnsResolved ? "réussie" : "non confirmée"} ·{" "}
              {String(result.dnsElapsedMs)} ms
            </p>
          )}
          {list(result.items).length > 0 && (
            <div className="table-container">
              <table>
                <thead>
                  <tr>
                    <th>Processus</th>
                    <th>Destination</th>
                    <th>Port</th>
                    <th>État</th>
                  </tr>
                </thead>
                <tbody>
                  {list(result.items).map((c, i) => (
                    <tr key={i}>
                      <td>
                        {String(c.process ?? "Inconnu")} ({String(c.pid)})
                      </td>
                      <td>{String(c.remoteAddress)}</td>
                      <td>{String(c.remotePort)}</td>
                      <td>{String(c.state)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
          {list(result.adapters).map((a, i) => (
            <div className="history-row" key={i}>
              <strong>{String(a.Name)}</strong>
              <span>
                {String(a.Status)} · {String(a.LinkSpeed)}
              </span>
            </div>
          ))}
          <details className="wb-details">
            <summary>Détails locaux du réseau</summary>
            <pre className="wb-output">{JSON.stringify(result, null, 2)}</pre>
          </details>
        </section>
      )}
      <section className="panel">
        <h2>Améliorations selon le diagnostic</h2>
        <p>
          Erreurs qui augmentent : vérifier le câble, le signal Wi-Fi et le
          pilote de la carte. Latence élevée vers la passerelle : examiner
          d’abord le réseau local. Passerelle correcte mais internet lent :
          examiner la connexion du fournisseur et les transferts en cours. Un
          test ICMP seul ne suffit pas à attribuer la panne.
        </p>
        <p>
          En cas de VPN, comparez les interfaces et serveurs DNS configurés. La
          présence d’un DNS externe ou d’une connexion inconnue ne prouve pas
          une fuite ; une analyse approfondie exige un test dédié.
        </p>
      </section>
    </section>
  );
}

export function MemoryGuardian({ visible }: { visible: boolean }) {
  const [sample, setSample] = useState<MemorySample>(),
    [selected, setSelected] = useState<MemoryTarget[]>([]),
    [auto, setAuto] = useState(false),
    [closing, setClosing] = useState(false),
    [query, setQuery] = useState(""),
    [sort, setSort] = useState("ram"),
    [manageableOnly, setManageableOnly] = useState(false),
    [pending, setPending] = useState<"auto" | MemoryTarget | null>(null),
    [error, setError] = useState(""),
    [notice, setNotice] = useState(""),
    [busy, setBusy] = useState(false),
    [samples, setSamples] = useState<number[]>([]);
  const guard = useRef(false),
    pressure = useRef(0),
    lastTrim = useRef(0),
    dialog = useRef<HTMLDialogElement>(null),
    session = useRef(0);
  useEffect(() => {
    void invoke("set_tray_memory_status", { active: auto }).catch(() => {});
  }, [auto]);
  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen("nova:memory-stop", () => {
      if (disposed) return;
      session.current++;
      setAuto(false);
      pressure.current = 0;
      setNotice(
        "Surveillance automatique suspendue depuis la zone de notification.",
      );
    })
      .then((stop) => {
        if (disposed) stop();
        else unlisten = stop;
      })
      .catch(() => {});
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);
  useEffect(() => {
    if (pending) dialog.current?.showModal();
    else dialog.current?.close();
  }, [pending]);
  async function refresh(automatic = false) {
    if (guard.current) return;
    guard.current = true;
    setBusy(true);
    const generation = session.current;
    try {
      const current = (await call(
        "memory",
        "memory-snapshot",
      )) as unknown as MemorySample;
      setSample(current);
      setSamples((old) => [...old, current.usedPercent].slice(-20));
      pressure.current = current.usedPercent >= 90 ? pressure.current + 1 : 0;
      if (automatic && generation === session.current) {
        const decision = memoryDecision(
          current,
          selected,
          pressure.current,
          Date.now() - lastTrim.current,
        );
        if (decision) {
          lastTrim.current = Date.now();
          await call(
            "memory",
            "memory-trim",
            { pid: decision.pid, started: decision.started },
            true,
          );
          setNotice(
            `Entretien du working set demandé pour ${decision.name}. Vérifiez la réactivité : les pages peuvent revenir en RAM.`,
          );
        }
      }
    } catch (e) {
      if (!automatic) setError(String(e));
      else
        setNotice(
          "Surveillance en attente : une autre opération ou une mesure indisponible empêche ce relevé.",
        );
    } finally {
      guard.current = false;
      setBusy(false);
    }
  }
  useEffect(() => {
    if (!auto) return;
    void refresh(true);
    const timer = setInterval(() => void refresh(true), 30000);
    return () => {
      clearInterval(timer);
      session.current++;
    };
  }, [auto, selected]);
  async function trim(target: MemoryTarget, close = false) {
    if (guard.current) return;
    guard.current = true;
    setBusy(true);
    setError("");
    try {
      const result = await call(
        "memory",
        close ? "process-close" : "memory-trim",
        { pid: target.pid, started: target.started },
        true,
      );
      setNotice(
        close
          ? result.exited === true
            ? `${target.name} a quitté.`
            : `Fermeture demandée pour ${target.name}. Vérifiez sa fenêtre : une confirmation ou un enregistrement peut être nécessaire.`
          : `Windows a reçu la demande pour ${target.name}. Il s’agit d’une réduction du working set, pas de la mémoire privée allouée.`,
      );
      if (close) {
        setSelected((old) =>
          old.filter(
            (t) => t.pid !== target.pid || t.started !== target.started,
          ),
        );
        if (result.exited === true)
          setSample((old) =>
            old
              ? {
                  ...old,
                  items: old.items.filter(
                    (t) => t.pid !== target.pid || t.started !== target.started,
                  ),
                }
              : old,
          );
      } else lastTrim.current = Date.now();
    } catch (e) {
      setError(String(e));
    } finally {
      guard.current = false;
      setBusy(false);
    }
  }
  const targets = (sample?.items ?? [])
    .filter(
      (t) =>
        (!manageableOnly || t.manageable) &&
        `${t.name} ${t.pid}`
          .toLocaleLowerCase()
          .includes(query.trim().toLocaleLowerCase()),
    )
    .slice()
    .sort((a, b) =>
      sort === "name"
        ? a.name.localeCompare(b.name)
        : sort === "private"
          ? b.privateBytes - a.privateBytes
          : b.workingSetBytes - a.workingSetBytes,
    );
  return (
    <section hidden={!visible} className="workbench">
      <div className="info-strip">
        <MemoryStick size={24} />
        <div>
          <strong>Surveillance mémoire intelligente</strong>
          <span>
            Mesure de la pression RAM et entretien optionnel des seules
            applications choisies. Fermeture normale disponible sur demande
            explicite.
          </span>
        </div>
      </div>
      <div className="toolbar">
        <button
          className="button secondary"
          disabled={busy}
          onClick={() => void refresh()}
        >
          Mesurer la mémoire
        </button>
        <button
          className="button primary"
          disabled={!selected.length || busy}
          onClick={() => {
            if (auto) {
              session.current++;
              setAuto(false);
              pressure.current = 0;
            } else {
              setClosing(false);
              setPending("auto");
            }
          }}
        >
          {auto
            ? "Arrêter la surveillance automatique"
            : "Activer la surveillance automatique"}
        </button>
      </div>
      {auto && (
        <p role="status" className="alert success">
          <ShieldCheck size={18} />
          Surveillance active tant que Nova reste ouvert, même sur un autre
          écran. {selected.length} processus autorisé(s).
        </p>
      )}
      {sample && (
        <section className="panel">
          <h2>
            {sample.usedPercent} % utilisés · {size(sample.freeBytes)}{" "}
            disponibles
          </h2>
          <p>
            Pagination disque : {sample.pageReadsPerSecond ?? "indisponible"}{" "}
            lecture(s)/seconde.
          </p>
          <svg
            viewBox="0 0 400 70"
            width="100%"
            height="70"
            aria-label="Évolution des dernières mesures de pression mémoire"
            role="img"
          >
            <polyline
              points={samples
                .map(
                  (n, i) =>
                    `${(i * 400) / Math.max(1, samples.length - 1)},${70 - n * 0.7}`,
                )
                .join(" ")}
              fill="none"
              stroke="var(--blue)"
              strokeWidth="2"
            />
          </svg>
        </section>
      )}
      {busy && <p role="status">Mesure ou entretien mémoire en cours…</p>}
      {error && (
        <p role="alert" className="alert error">
          {error}
        </p>
      )}
      {notice && (
        <p role="status" className="wb-note">
          {notice}
        </p>
      )}
      <section className="panel">
        <h2>Applications consommatrices</h2>
        <div className="toolbar">
          <label>
            Rechercher une application ou un PID{" "}
            <input
              type="search"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
          </label>
          <label>
            Trier par{" "}
            <select
              className="nova-select"
              value={sort}
              onChange={(e) => setSort(e.target.value)}
            >
              <option value="ram">RAM utilisée</option>
              <option value="private">Mémoire privée</option>
              <option value="name">Nom</option>
            </select>
          </label>
          <label>
            <input
              type="checkbox"
              checked={manageableOnly}
              onChange={(e) => setManageableOnly(e.target.checked)}
            />{" "}
            Applications gérables uniquement
          </label>
        </div>
        {sample && (
          <p>
            {targets.length} application(s) affichée(s) sur{" "}
            {sample.items.length} · {selected.length} autorisée(s) pour
            l’entretien automatique.
          </p>
        )}
        <p>
          Sélectionnez les processus autorisés pour l’entretien automatique. Les
          processus système et certaines protections sont exclus. La liste est
          limitée aux 30 processus de plus de 100 Mio les plus consommateurs.
        </p>
        {targets.map((t) => (
          <div className="history-row" key={`${t.pid}-${t.started}`}>
            <label className="grow">
              <input
                type="checkbox"
                disabled={!t.manageable || busy || auto}
                checked={selected.some(
                  (s) => s.pid === t.pid && s.started === t.started,
                )}
                onChange={(e) =>
                  setSelected((old) =>
                    e.target.checked
                      ? [...old, t]
                      : old.filter(
                          (s) => !(s.pid === t.pid && s.started === t.started),
                        ),
                  )
                }
              />{" "}
              {t.name} · {size(t.workingSetBytes)} en RAM ·{" "}
              {size(t.privateBytes)} privés
            </label>
            <button
              className="button secondary"
              disabled={!t.manageable || busy}
              onClick={() => {
                setClosing(false);
                setPending(t);
              }}
            >
              Entretenir
            </button>
            <button
              className="button secondary"
              disabled={!t.manageable || busy || auto}
              onClick={() => {
                setClosing(true);
                setPending(t);
              }}
            >
              Fermer
            </button>
          </div>
        ))}
      </section>
      <p className="footnote">
        Automatique : 3 relevés consécutifs à au moins 90 % de RAM utilisée, peu
        de pagination, un seul processus autorisé de plus de 200 Mio par
        intervalle de 10 minutes. L’application au premier plan est ignorée. Une
        réduction de working set peut causer des rechargements et ne corrige pas
        une fuite mémoire. Windows reste responsable de la gestion de la
        mémoire.
      </p>
      <dialog
        ref={dialog}
        aria-labelledby="memory-confirm"
        onCancel={() => setPending(null)}
      >
        <h2 id="memory-confirm">
          {pending === "auto"
            ? "Activer l’entretien mémoire automatique ?"
            : closing
              ? `Fermer ${pending?.name ?? "cette application"} ?`
              : "Entretenir cette application ?"}
        </h2>
        <p>
          {pending === "auto"
            ? "Nova pourra demander une réduction du working set aux seuls processus que vous avez sélectionnés, lorsque les critères de pression sont remplis. Cette autorisation dure jusqu’à l’arrêt du mode ou la fermeture de Nova."
            : closing
              ? "Enregistrez votre travail avant de continuer. Nova demande une fermeture normale de cette application uniquement. Elle peut demander confirmation ou refuser ; les processus système sont exclus et aucun arrêt forcé n’est effectué."
              : "Windows pourra retirer des pages du working set de cette application en arrière-plan. Des défauts de page peuvent augmenter lors de sa prochaine utilisation."}
        </p>
        <div className="dialog-actions">
          <button className="button secondary" onClick={() => setPending(null)}>
            Annuler
          </button>
          <button
            className="button primary"
            onClick={() => {
              const target = pending;
              setPending(null);
              if (target === "auto") {
                pressure.current = 0;
                session.current++;
                setAuto(true);
              } else if (target) void trim(target, closing);
            }}
          >
            Confirmer
          </button>
        </div>
      </dialog>
    </section>
  );
}
