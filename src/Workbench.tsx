import { useEffect, useId, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { message } from "./lib";
import "./workbench.css";

type Row = Record<string, unknown>;
type Reply = { ok: boolean; data?: unknown; error?: string };
type Module = "optimize" | "apps" | "cleanup" | "automation";
type Job = {
  module: Module;
  action: string;
  label: string;
  payload?: Row;
  confirmation?: string;
  permanent?: boolean;
};
type Entry = {
  action: string;
  label: string;
  description: string;
  category: string;
  admin?: boolean;
  changes?: boolean;
};
const catalog: Entry[] = [
  {
    action: "storage-analyze",
    label: "Analyser l’optimisation du disque",
    category: "Stockage",
    admin: true,
    description:
      "Analyser le volume système NTFS avec Windows. Affiche les statistiques sans lancer d’optimisation.",
  },
  {
    action: "storage-optimize",
    label: "Optimiser le disque avec Windows",
    category: "Stockage",
    admin: true,
    changes: true,
    description:
      "Optimiser le volume système NTFS en laissant Windows choisir l’opération adaptée au support. Peut durer longtemps et solliciter le disque ; à utiliser après l’analyse.",
  },
  {
    action: "boot-slowdowns",
    label: "Repérer les lenteurs au démarrage",
    category: "Diagnostic",
    description:
      "Lire jusqu’à 30 événements Windows des 7 derniers jours concernant les applications, pilotes et services au démarrage. Aucun élément désactivé automatiquement.",
  },
  {
    action: "power-requests",
    label: "Qui empêche la veille ?",
    category: "Énergie",
    admin: true,
    description:
      "Afficher les demandes des applications et pilotes qui maintiennent le PC ou l’écran éveillé. Utile pour comprendre la consommation au repos.",
  },
  {
    action: "sleep-states",
    label: "Vérifier les modes de veille",
    category: "Énergie",
    description:
      "Afficher les états de veille disponibles et les raisons Windows de leur indisponibilité.",
  },
  {
    action: "wake-sources",
    label: "Comprendre les réveils du PC",
    category: "Énergie",
    admin: true,
    description:
      "Afficher la dernière source de réveil et les minuteurs actifs, sans changer les périphériques ni les tâches planifiées.",
  },
  {
    action: "opti-audit",
    label: "Audit de maintenance",
    description:
      "Lire les informations utiles au diagnostic, sans appliquer de réglages.",
    category: "Diagnostic",
  },
  {
    action: "dism-analyze",
    label: "Analyser le magasin Windows",
    description:
      "Examiner le magasin de composants et les possibilités de nettoyage DISM.",
    category: "Windows",
    admin: true,
  },
  {
    action: "dism-cleanup",
    label: "Nettoyer les composants Windows",
    description:
      "Supprimer les composants remplacés via DISM. Cette opération peut durer plusieurs minutes.",
    category: "Windows",
    admin: true,
    changes: true,
  },
  {
    action: "system-check",
    label: "Vérifier les fichiers système",
    description:
      "Vérifier l’intégrité des fichiers système Windows. Consulter le résultat pour les anomalies détectées.",
    category: "Windows",
    admin: true,
  },
  {
    action: "disk-scan",
    label: "Analyser le disque système",
    description:
      "Rechercher les erreurs du système de fichiers ; une réparation peut nécessiter une autre intervention.",
    category: "Diagnostic",
    admin: true,
    changes: true,
  },
  {
    action: "dns-flush",
    label: "Vider le cache DNS",
    description:
      "Effacer les résolutions DNS locales. Les prochaines requêtes devront être résolues à nouveau.",
    category: "Réseau",
    admin: true,
    changes: true,
  },
  {
    action: "energy-saver",
    label: "Activer le profil économe",
    description:
      "Changer le profil d’alimentation ; les performances peuvent être réduites.",
    category: "Énergie",
    admin: true,
    changes: true,
  },
  {
    action: "hibernate-on",
    label: "Activer la veille prolongée",
    description:
      "Autoriser la veille prolongée et son fichier système sur le disque.",
    category: "Énergie",
    admin: true,
    changes: true,
  },
  {
    action: "hibernate-off",
    label: "Désactiver la veille prolongée",
    description:
      "Retirer la veille prolongée ; le démarrage rapide peut aussi devenir indisponible.",
    category: "Énergie",
    admin: true,
    changes: true,
  },
  {
    action: "power-report",
    label: "Rapport d’énergie",
    description:
      "Générer un rapport Windows sur la consommation et les paramètres d’alimentation.",
    category: "Énergie",
    admin: true,
  },
  {
    action: "battery-report",
    label: "Rapport de batterie",
    description:
      "Générer l’historique et les capacités de batterie disponibles sur cet appareil.",
    category: "Énergie",
  },
];
const titles: Record<string, string> = {
  maintenance: "Maintenance",
  cleanup: "Nettoyage réversible",
  applications: "Applications",
  "startup-manage": "Gestion du démarrage",
  automation: "Automatisation",
  measurements: "Mesures comparées",
  "driver-sources": "Fabricants détectés",
};
const asRow = (value: unknown): Row =>
  value !== null && typeof value === "object" && !Array.isArray(value)
    ? (value as Row)
    : {};
function list(value: unknown, key = "items"): Row[] {
  const source = Array.isArray(value) ? value : asRow(value)[key];
  return Array.isArray(source)
    ? (source.filter(
        (item) =>
          item !== null && typeof item === "object" && !Array.isArray(item),
      ) as Row[])
    : [];
}
function display(value: unknown): string {
  if (value === undefined || value === null) return "—";
  if (typeof value === "boolean") return value ? "Oui" : "Non";
  if (typeof value === "number")
    return value.toLocaleString("fr-FR", { maximumFractionDigits: 3 });
  return typeof value === "object" ? JSON.stringify(value) : String(value);
}
function byteLabel(value: unknown): string {
  if (typeof value !== "number" || !Number.isFinite(value)) return "—";
  const units = ["o", "Kio", "Mio", "Gio", "Tio"];
  const power = Math.min(
    4,
    Math.max(0, Math.floor(Math.log2(Math.max(1, Math.abs(value))) / 10)),
  );
  return `${(value / 1024 ** power).toLocaleString("fr-FR", { maximumFractionDigits: 1 })} ${units[power]}`;
}
const numericLabels: Record<string, string> = {
  freeMemoryBytes: "Mémoire disponible (octets)",
  cpuLoadPercent: "Charge CPU (%)",
  processCount: "Processus actifs",
  bootDurationMs: "Dernier démarrage mesuré (ms)",
  freeDiskBytes: "Espace disque disponible (octets)",
  cpu: "Processeur",
  cpuPercent: "CPU (%)",
  cpu_percent: "CPU (%)",
  memory: "Mémoire",
  memoryPercent: "Mémoire (%)",
  memory_percent: "Mémoire (%)",
  usedBytes: "Mémoire utilisée (octets)",
  availableBytes: "Disponible (octets)",
  freeBytes: "Libre (octets)",
};
function numericRows(value: unknown, path = ""): [string, number][] {
  return Object.entries(asRow(value)).flatMap(([key, item]) => {
    const label = path
      ? `${path} / ${numericLabels[key] ?? key}`
      : (numericLabels[key] ?? key);
    return typeof item === "number"
      ? [[label, item] as [string, number]]
      : numericRows(item, label);
  });
}
function Result({ data, action }: { data: unknown; action: string }) {
  const raw = asRow(data);
  const object =
    action.startsWith("snapshot-") && raw.delta && !raw.deltas
      ? { ...raw, deltas: raw.delta }
      : raw;
  const entries = Object.entries(object).filter(
    ([key]) => !["output", "diagnostics"].includes(key),
  );
  const measured = action.startsWith("snapshot-");
  return (
    <>
      {typeof raw.output === "string" && (
        <pre className="wb-output">{raw.output}</pre>
      )}
      {action === "boot-slowdowns" && (
        <div>
          <p>{display(raw.interpretation)}</p>
          {list(data).length ? (
            <div className="table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>Élément signalé</th>
                    <th>Type</th>
                    <th>Durée</th>
                    <th>Dégradation</th>
                    <th>Date UTC</th>
                  </tr>
                </thead>
                <tbody>
                  {list(data).map((item, i) => (
                    <tr key={i}>
                      <td>{display(item.name)}</td>
                      <td>
                        {(
                          {
                            101: "Application",
                            102: "Pilote",
                            103: "Service",
                          } as Record<string, string>
                        )[String(item.eventId)] ?? "Autre"}
                      </td>
                      <td>{display(item.durationMs)} ms</td>
                      <td>{display(item.degradationMs)} ms</td>
                      <td>{display(item.timestampUtc)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <p>
              Aucun événement correspondant retourné. Cela ne garantit pas un
              démarrage optimal.
            </p>
          )}
        </div>
      )}
      {typeof asRow(raw.diagnostics).text === "string" && (
        <pre className="wb-output">{String(asRow(raw.diagnostics).text)}</pre>
      )}
      {measured && (
        <p className="wb-note">
          CPU et mémoire sont des mesures instantanées, sensibles aux tâches en
          cours. Une variation ne prouve pas une amélioration.
        </p>
      )}
      {measured ? (
        (["baseline", "current", "deltas"].filter((key) => key in object).length
          ? ["baseline", "current", "deltas"].filter((key) => key in object)
          : ["snapshot"]
        ).map((key) => {
          const values = numericRows(key === "snapshot" ? data : object[key]);
          return (
            <div key={key}>
              <h3>
                {
                  {
                    baseline: "Référence",
                    current: "Mesure actuelle",
                    deltas: "Écarts retournés",
                    snapshot: "Mesure de référence",
                  }[key]
                }
              </h3>
              {values.length ? (
                <dl className="wb-values">
                  {values.map(([label, value]) => (
                    <div key={label}>
                      <dt>{label}</dt>
                      <dd>{display(value)}</dd>
                    </div>
                  ))}
                </dl>
              ) : (
                <p>Aucune mesure numérique dans cette réponse.</p>
              )}
            </div>
          );
        })
      ) : typeof data === "string" ? (
        <pre className="wb-output">{data}</pre>
      ) : entries.length ? (
        <dl className="wb-values">
          {entries.map(([key, value]) => (
            <div key={key}>
              <dt>
                {(
                  {
                    enabled: "Activé",
                    exitCode: "Code Windows",
                    rebootRequired: "Redémarrage requis",
                    readOnly: "Lecture seule",
                    activePowerScheme: "Profil d’alimentation",
                    automaticPagefile: "Fichier d’échange automatique",
                    physicalDisks: "Disques physiques",
                    storageReliability: "État du stockage",
                    networkAdapters: "Interfaces réseau",
                    sample: "Mesures instantanées",
                    reportPath: "Rapport local",
                    movedBytes: "Octets déplacés",
                    freedBytes: "Octets supprimés",
                    available: "Disponible",
                    tag: "Version publiée",
                    publishedAt: "Date de publication",
                    lastReport: "Dernier bilan",
                    nextRun: "Prochaine analyse",
                    count: "Nombre",
                    results: "Résultats détaillés",
                    timestamp: "Date",
                    notifications: "Notifications",
                  } as Record<string, string>
                )[key] ?? key}
              </dt>
              <dd>
                {typeof value === "object" && value !== null
                  ? Array.isArray(value)
                    ? `${value.length} élément(s)`
                    : `${Object.keys(asRow(value)).length} propriété(s)`
                  : display(value)}
              </dd>
            </div>
          ))}
        </dl>
      ) : (
        <p>
          {data == null
            ? "Opération terminée sans données supplémentaires."
            : display(data)}
        </p>
      )}
      {data != null && (
        <details className="wb-details">
          <summary>Données complètes · lecture seule</summary>
          <pre className="wb-output" tabIndex={0}>
            {JSON.stringify(data, null, 2)}
          </pre>
        </details>
      )}
    </>
  );
}

export default function Workbench({ section }: { section: string }) {
  const guard = useRef(false);
  const [busy, setBusy] = useState(false);
  // Remount the view on navigation so lists, selections and confirmations never cross sections.
  return (
    <WorkbenchView
      key={section}
      section={section}
      guard={guard}
      busy={busy}
      setBusy={setBusy}
    />
  );
}
function WorkbenchView({
  section,
  guard,
  busy,
  setBusy,
}: {
  section: string;
  guard: { current: boolean };
  busy: boolean;
  setBusy: (value: boolean) => void;
}) {
  const mounted = useRef(true);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [results, setResults] = useState<Record<string, unknown>>({});
  const [last, setLast] = useState<{
    action: string;
    label: string;
    data: unknown;
  } | null>(null);
  const [pending, setPending] = useState<Job | null>(null);
  const [accepted, setAccepted] = useState(false);
  const [phrase, setPhrase] = useState("");
  const [search, setSearch] = useState("");
  const [category, setCategory] = useState("Tout");
  const [selected, setSelected] = useState<string[]>([]);
  const [appId, setAppId] = useState("");
  const [startupSearch, setStartupSearch] = useState("");
  const [startupState, setStartupState] = useState("all");
  const dialog = useRef<HTMLDialogElement>(null);
  const uid = useId();
  useEffect(() => {
    mounted.current = true;
    if (section === "driver-sources")
      void run({
        module: "automation",
        action: "detect-sources",
        label: "Détection des fabricants",
      });
    return () => {
      mounted.current = false;
    };
  }, []);
  useEffect(() => {
    if (pending && !dialog.current?.open) dialog.current?.showModal();
  }, [pending]);
  async function run(job: Job, confirmed = false) {
    if (guard.current) return;
    guard.current = true;
    setBusy(true);
    setError("");
    setNotice("");
    try {
      const reply = await invoke<Reply>("extended_action", {
        module: job.module,
        action: job.action,
        payload: job.payload ?? {},
        confirmed,
      });
      if (!mounted.current) return;
      if (reply.data != null) {
        setResults((previous) => ({ ...previous, [job.action]: reply.data }));
        setLast({ action: job.action, label: job.label, data: reply.data });
      }
      if (
        job.module === "cleanup" &&
        ["quarantine", "restore", "purge", "delete-selected"].includes(
          job.action,
        )
      ) {
        setSelected([]);
        setResults((previous) => {
          const next = { ...previous };
          delete next.preview;
          delete next["cleanup-batches"];
          return next;
        });
      }
      if (!reply.ok)
        throw new Error(
          message(reply.error) ||
            "Le service n’a pas pu terminer cette opération.",
        );
      setResults((previous) => ({ ...previous, [job.action]: reply.data }));
      setLast({ action: job.action, label: job.label, data: reply.data });
      setNotice(`${job.label} : terminé.`);
      if (
        job.module === "apps" &&
        ["startup-enable", "startup-disable"].includes(job.action)
      ) {
        setResults((previous) => {
          const next = { ...previous };
          delete next["startup-list"];
          return next;
        });
      }
    } catch (reason) {
      if (mounted.current)
        setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      guard.current = false;
      setBusy(false);
    }
  }
  function request(job: Job) {
    if (guard.current || pending) return;
    if (job.confirmation) {
      setAccepted(false);
      setPhrase("");
      setPending(job);
    } else void run(job);
  }
  function closeDialog() {
    dialog.current?.close();
    setPending(null);
  }
  const button = (job: Job, label = job.label, disabled = false) => (
    <button
      type="button"
      className="wb-button"
      disabled={busy || !!pending || disabled}
      onClick={() => request(job)}
    >
      {label}
    </button>
  );
  const actionRow = (job: Job, description: string, tag?: string) => (
    <div className="wb-row" key={job.action}>
      <div>
        <h3>{job.label}</h3>
        <p>{description}</p>
        {tag && <span className="wb-tag">{tag}</span>}
      </div>
      {button(job, "Exécuter")}
    </div>
  );
  const preview = list(results.preview);
  const batches = list(results["cleanup-batches"], "batches").length
    ? list(results["cleanup-batches"], "batches")
    : list(results["cleanup-batches"]);
  const apps = list(results["apps-list"], "apps").length
    ? list(results["apps-list"], "apps")
    : list(results["apps-list"]);
  const upgrades = asRow(results["app-upgrades"]);
  const startup = list(results["startup-list"]);
  const validAppId = /^[A-Za-z0-9][A-Za-z0-9._-]{1,127}$/.test(appId);
  const has = (action: string) =>
    Object.prototype.hasOwnProperty.call(results, action);
  return (
    <section className="workbench" aria-labelledby={`${uid}-title`}>
      <header className="wb-header">
        <span className="wb-eyebrow">OUTILS SYSTÈME</span>
        <h2 id={`${uid}-title`}>{titles[section] ?? "Outils indisponibles"}</h2>
        <p>
          Choisissez une opération et consultez son résultat avant de
          poursuivre.
        </p>
      </header>
      <div role="alert" className={error ? "wb-alert" : "wb-sr-only"}>
        {error}
      </div>
      <div role="status" aria-live="polite" className="wb-status">
        {busy ? "Opération en cours…" : notice}
      </div>
      <div className="wb-layout">
        <div className="wb-panel" aria-busy={busy}>
          {section === "maintenance" && (
            <>
              <div className="wb-tools">
                <label>
                  Rechercher
                  <input
                    type="search"
                    value={search}
                    onChange={(e) => setSearch(e.target.value)}
                  />
                </label>
                <label>
                  Profil / catégorie
                  <select
                    className="nova-select"
                    value={category}
                    onChange={(e) => setCategory(e.target.value)}
                  >
                    {[
                      "Tout",
                      "Diagnostic",
                      "Windows",
                      "Stockage",
                      "Réseau",
                      "Énergie",
                    ].map((item) => (
                      <option key={item}>{item}</option>
                    ))}
                  </select>
                </label>
              </div>
              {catalog
                .filter(
                  (item) =>
                    (category === "Tout" || category === item.category) &&
                    `${item.label} ${item.description}`
                      .toLocaleLowerCase("fr")
                      .includes(search.toLocaleLowerCase("fr")),
                )
                .map((item) =>
                  actionRow(
                    {
                      module: "optimize",
                      action: item.action,
                      label: item.label,
                      confirmation: item.changes
                        ? `${item.description} Confirmer cette modification ?`
                        : undefined,
                    },
                    item.description,
                    item.admin ? "Administrateur requis" : undefined,
                  ),
                )}
              {!catalog.some(
                (item) =>
                  (category === "Tout" || category === item.category) &&
                  `${item.label} ${item.description}`
                    .toLocaleLowerCase("fr")
                    .includes(search.toLocaleLowerCase("fr")),
              ) && (
                <p className="wb-empty">
                  Aucune opération ne correspond aux filtres.
                </p>
              )}
            </>
          )}
          {section === "measurements" && (
            <>
              {actionRow(
                {
                  module: "optimize",
                  action: "snapshot-baseline",
                  label: "Enregistrer une référence",
                  confirmation:
                    "Enregistrer une mesure de référence ? Elle remplacera la référence précédente sur le disque.",
                },
                "Capturer l’état actuel comme point de comparaison. Une nouvelle référence remplace la précédente.",
              )}
              {actionRow(
                {
                  module: "optimize",
                  action: "snapshot-compare",
                  label: "Comparer à la référence",
                },
                "Demander une nouvelle mesure et les écarts calculés par le service.",
              )}
              <p className="wb-note">
                Comparez dans des conditions similaires. CPU et mémoire
                fluctuent ; ces relevés ne démontrent aucun gain de
                performances.
              </p>
            </>
          )}
          {section === "cleanup" && (
            <>
              <div className="wb-intro">
                <h2>Fichiers temporaires</h2>
                <p>
                  Analyse limitée aux fichiers de plus de 7 jours à la racine
                  des temporaires utilisateur (1 000 maximum). La quarantaine
                  déplace les fichiers : elle ne libère pas d’octets. Seule une
                  suppression définitive peut libérer de l’espace.
                </p>
                <div className="wb-actions">
                  {button({
                    module: "cleanup",
                    action: "preview",
                    label: "Analyser les fichiers",
                  })}
                  {button({
                    module: "cleanup",
                    action: "cleanup-batches",
                    label: "Actualiser les lots",
                  })}
                </div>
              </div>
              {has("preview") && (
                <>
                  <p className="wb-note">
                    Total annoncé :{" "}
                    {byteLabel(asRow(results.preview).totalBytes)} ·{" "}
                    {selected.length} sélection(s)
                  </p>
                  {preview.map((item, index) => (
                    <label
                      className="wb-check-row"
                      key={`${display(item.id)}-${index}`}
                    >
                      <input
                        type="checkbox"
                        disabled={
                          busy || typeof item.id !== "string" || !item.id
                        }
                        checked={
                          typeof item.id === "string" &&
                          selected.includes(item.id)
                        }
                        onChange={(e) => {
                          const id = String(item.id);
                          setSelected((old) =>
                            e.target.checked
                              ? [...old, id]
                              : old.filter((value) => value !== id),
                          );
                        }}
                      />
                      <span>
                        <strong>{display(item.name)}</strong>
                        <small>
                          {byteLabel(item.bytes)} · {display(item.ageDays)}{" "}
                          jour(s)
                        </small>
                      </span>
                    </label>
                  ))}
                  {!preview.length && (
                    <p className="wb-empty">
                      Aucun fichier proposé. Consultez les données complètes si
                      le format de réponse diffère.
                    </p>
                  )}
                  <div className="wb-actions">
                    <button
                      className="button secondary"
                      disabled={busy || !preview.length}
                      onClick={() =>
                        setSelected(
                          preview
                            .map((item) => String(item.id))
                            .filter(Boolean),
                        )
                      }
                    >
                      Tout sélectionner
                    </button>
                    <button
                      className="button secondary"
                      disabled={busy || !selected.length}
                      onClick={() => setSelected([])}
                    >
                      Tout désélectionner
                    </button>
                    {button(
                      {
                        module: "cleanup",
                        action: "delete-selected",
                        label: "Supprimer les temporaires sélectionnés",
                        payload: { ids: selected },
                        confirmation: `Supprimer définitivement les ${selected.length} temporaires sélectionnés ? Les fichiers occupés ou modifiés seront conservés.`,
                        permanent: true,
                      },
                      "Supprimer définitivement",
                      !selected.length,
                    )}
                    {button(
                      {
                        module: "cleanup",
                        action: "quarantine",
                        label: "Mettre en quarantaine",
                        payload: { ids: selected },
                        confirmation: `Déplacer les ${selected.length} fichier(s) sélectionné(s) en quarantaine ? Ce déplacement ne libère pas d’octets. Les fichiers pourront être restaurés depuis leur lot.`,
                      },
                      "Mettre en quarantaine",
                      !selected.length,
                    )}
                  </div>
                </>
              )}
              <div className="wb-intro">
                <h2>Lots en quarantaine</h2>
                <p>
                  Après une opération, actualisez les lots et l’analyse pour
                  obtenir l’état courant.
                </p>
              </div>
              {batches.map((batch, index) => {
                const id =
                  typeof batch.id === "string"
                    ? batch.id
                    : typeof batch.batchId === "string"
                      ? batch.batchId
                      : "";
                return (
                  <div className="wb-row" key={`${id}-${index}`}>
                    <div>
                      <h3>Lot {id || "sans identifiant"}</h3>
                      <p>
                        {display(
                          batch.name ?? batch.createdAt ?? batch.timestamp,
                        )}{" "}
                        · {byteLabel(batch.totalBytes ?? batch.bytes)}
                      </p>
                      <small>
                        {Array.isArray(batch.items)
                          ? `${batch.items.length} fichier(s)`
                          : "Détails dans la réponse complète"}
                      </small>
                    </div>
                    <div className="wb-actions">
                      {button(
                        {
                          module: "cleanup",
                          action: "restore",
                          label: "Restaurer",
                          payload: { batchId: id },
                          confirmation: `Restaurer les fichiers du lot ${id} à leur emplacement d’origine ?`,
                        },
                        "Restaurer",
                        !id,
                      )}
                      {button(
                        {
                          module: "cleanup",
                          action: "purge",
                          label: "Supprimer définitivement",
                          payload: { batchId: id },
                          permanent: true,
                          confirmation: `Supprimer définitivement le lot ${id} et ses fichiers ? Cette opération est irréversible ; Nova ne pourra plus les restaurer.`,
                        },
                        "Supprimer",
                        !id,
                      )}
                    </div>
                  </div>
                );
              })}
              {has("cleanup-batches") && !batches.length && (
                <p className="wb-empty">
                  Aucun lot disponible dans cette réponse.
                </p>
              )}
            </>
          )}
          {section === "applications" && (
            <>
              <div className="wb-intro">
                <h2>Applications installées</h2>
                <p>
                  La recherche interroge les sources WinGet configurées.
                  Certaines sources ou licences peuvent nécessiter une
                  acceptation dans l’outil officiel avant de continuer.
                </p>
                <div className="wb-actions">
                  {button({
                    module: "apps",
                    action: "apps-list",
                    label: "Charger les applications",
                  })}
                  {button({
                    module: "apps",
                    action: "app-upgrades",
                    label: "Rechercher les mises à jour",
                  })}
                </div>
                <label>
                  Filtrer par nom, version ou éditeur
                  <input
                    type="search"
                    value={search}
                    onChange={(e) => setSearch(e.target.value)}
                  />
                </label>
              </div>
              {apps
                .filter((app) =>
                  [app.name, app.version, app.publisher].some((value) =>
                    display(value)
                      .toLocaleLowerCase("fr")
                      .includes(search.toLocaleLowerCase("fr")),
                  ),
                )
                .map((app, index) => (
                  <div className="wb-row" key={index}>
                    <div>
                      <h3>{display(app.name)}</h3>
                      <p>
                        {display(app.version)} · {display(app.publisher)}
                      </p>
                    </div>
                  </div>
                ))}
              {has("apps-list") && !apps.length && (
                <p className="wb-empty">
                  Aucune application disponible dans cette réponse.
                </p>
              )}
              {has("app-upgrades") && (
                <section className="wb-note wb-upgrades" aria-live="polite">
                  <strong>Dernière recherche WinGet</strong>
                  <p>
                    {upgrades.available === true
                      ? upgrades.exitCode === 0
                        ? "Recherche terminée. Consultez les paquets proposés avant toute mise à jour."
                        : "WinGet a terminé avec un code à vérifier."
                      : "WinGet n’est pas disponible sur ce PC."}
                  </p>
                  {typeof upgrades.output === "string" && upgrades.output.trim() && (
                    <details className="wb-details">
                      <summary>Résultats proposés</summary>
                      <pre className="wb-output">{upgrades.output}</pre>
                    </details>
                  )}
                  {upgrades.truncated === true && (
                    <small>Le résultat est tronqué pour préserver les performances.</small>
                  )}
                </section>
              )}
              <div className="wb-intro">
                <h2>Mettre à jour un paquet précis</h2>
                <p>
                  Copiez l’identifiant exact affiché par la recherche. Aucune
                  acceptation automatique de licence (EULA) n’est demandée ; si
                  une licence est requise, utilisez l’outil officiel.
                </p>
                <label>
                  Identifiant exact du paquet
                  <input
                    value={appId}
                    onChange={(e) => setAppId(e.target.value)}
                    spellCheck={false}
                    autoCapitalize="none"
                    aria-describedby={`${uid}-id-help`}
                  />
                </label>
                <small id={`${uid}-id-help`}>
                  2 à 128 caractères : lettres ASCII, chiffres, point, tiret ou
                  soulignement. Aucun espace ni argument de commande.
                </small>
                <div className="wb-actions">
                  {button(
                    {
                      module: "apps",
                      action: "app-upgrade",
                      label: "Mettre à jour",
                      payload: { id: appId },
                      confirmation: `Demander la mise à jour du paquet exact « ${appId} » ? Son programme d’installation peut modifier votre système. Aucune licence ne sera acceptée automatiquement.`,
                    },
                    "Mettre à jour ce paquet",
                    !validAppId,
                  )}
                </div>
              </div>
            </>
          )}
          {section === "startup-manage" && (
            <>
              <div className="wb-intro">
                <h2>Éléments de démarrage</h2>
                <p>
                  Seuls les éléments signalés comme gérables peuvent être
                  modifiés. La réactivation restaure l’élément depuis sa
                  sauvegarde.
                </p>
                {button({
                  module: "apps",
                  action: "startup-list",
                  label: "Actualiser le démarrage",
                })}
              </div>
              <div className="toolbar">
                <label>
                  Rechercher au démarrage{" "}
                  <input
                    type="search"
                    value={startupSearch}
                    onChange={(e) => setStartupSearch(e.target.value)}
                  />
                </label>
                <label>
                  Afficher{" "}
                  <select
                    className="nova-select"
                    value={startupState}
                    onChange={(e) => setStartupState(e.target.value)}
                  >
                    <option value="all">Tous les éléments</option>
                    <option value="enabled">Activés</option>
                    <option value="disabled">Désactivés</option>
                    <option value="manageable">Gérables dans Nova</option>
                  </select>
                </label>
              </div>
              {has("startup-list") && (
                <p>
                  {startup.filter((item) => item.enabled === true).length}{" "}
                  activé(s) ·{" "}
                  {startup.filter((item) => item.enabled === false).length}{" "}
                  désactivé(s). Désactiver au démarrage ne ferme pas une
                  application déjà ouverte.
                </p>
              )}
              {startup
                .filter(
                  (item) =>
                    display(item.name)
                      .toLocaleLowerCase("fr")
                      .includes(startupSearch.trim().toLocaleLowerCase("fr")) &&
                    (startupState === "all" ||
                      (startupState === "manageable" &&
                        item.manageable === true) ||
                      (startupState === "enabled" && item.enabled === true) ||
                      (startupState === "disabled" && item.enabled === false)),
                )
                .map((item, index) => {
                  const id = typeof item.id === "string" ? item.id : "";
                  const enabled = item.enabled;
                  return (
                    <div className="wb-row" key={`${id}-${index}`}>
                      <div>
                        <h3>{display(item.name)}</h3>
                        <p>
                          {enabled === true
                            ? "Activé"
                            : enabled === false
                              ? "Désactivé"
                              : "État inconnu"}{" "}
                          ·{" "}
                          {item.manageable
                            ? "Gérable dans Nova"
                            : "Lecture seule"}
                        </p>
                      </div>
                      {button(
                        {
                          module: "apps",
                          action:
                            enabled === true
                              ? "startup-disable"
                              : "startup-enable",
                          label:
                            enabled === true
                              ? "Désactiver"
                              : "Restaurer la sauvegarde",
                          payload: { id },
                          confirmation:
                            enabled === true
                              ? `Désactiver « ${display(item.name)} » au démarrage (identifiant ${id}) ?`
                              : `Restaurer la sauvegarde de « ${display(item.name)} » (identifiant ${id}) pour le réactiver au démarrage ?`,
                        },
                        enabled === true ? "Désactiver" : "Réactiver",
                        !id ||
                          item.manageable !== true ||
                          typeof enabled !== "boolean",
                      )}
                    </div>
                  );
                })}
              {has("startup-list") && !startup.length && (
                <p className="wb-empty">
                  Aucun élément disponible dans cette réponse.
                </p>
              )}
            </>
          )}
          {section === "automation" && (
            <>
              <p className="wb-note">
                Les tâches planifiées effectuent uniquement des audits en
                lecture seule, avec l’utilisateur connecté : à 9 h chaque jour
                ou le lundi à 9 h chaque semaine. Conservez Nova installé au
                même emplacement. Les notifications Windows sont proposées au
                mieux selon les autorisations du système ; le bilan reste
                consultable ici.
              </p>
              {[
                [
                  "schedule-status",
                  "État de la planification",
                  "Consulter la tâche actuellement configurée.",
                  false,
                ],
                [
                  "schedule-daily",
                  "Audit quotidien",
                  "Planifier un audit chaque jour à 9 h, lorsque l’utilisateur est connecté.",
                  true,
                ],
                [
                  "schedule-weekly",
                  "Audit hebdomadaire",
                  "Planifier un audit le lundi à 9 h, lorsque l’utilisateur est connecté.",
                  true,
                ],
                [
                  "schedule-disable",
                  "Désactiver la planification",
                  "Arrêter les audits planifiés.",
                  true,
                ],
                [
                  "check-release",
                  "Vérifier la version de Nova",
                  "Rechercher les informations de publication disponibles.",
                  false,
                ],
                [
                  "open-release",
                  "Ouvrir la publication",
                  "Ouvrir la page de publication dans votre navigateur.",
                  false,
                ],
                [
                  "notifications-on",
                  "Activer les notifications",
                  "Autoriser les notifications de Nova.",
                  true,
                ],
                [
                  "notifications-off",
                  "Désactiver les notifications",
                  "Désactiver les notifications de Nova.",
                  true,
                ],
              ].map(([action, label, description, changes]) =>
                actionRow(
                  {
                    module: "automation",
                    action: String(action),
                    label: String(label),
                    confirmation: changes
                      ? `${description} Confirmer ce réglage ?`
                      : undefined,
                  },
                  String(description),
                ),
              )}
            </>
          )}
          {section === "driver-sources" && (
            <>
              <div className="wb-intro">
                <h2>Support officiel des fabricants</h2>
                <p>
                  Nova détecte les fabricants depuis votre ordinateur, carte
                  mère, CPU, GPU et périphériques. Les composants identifiés
                  sont regroupés ci-dessous avec les supports officiels
                  disponibles. La recherche des mises à jour applicables reste
                  dans Pilotes et mises à jour.
                </p>
                {button({
                  module: "automation",
                  action: "detect-sources",
                  label: "Actualiser la détection",
                })}
              </div>
              {list(results["detect-sources"], "providers").map((provider) => (
                <div className="wb-row" key={String(provider.id)}>
                  <div>
                    <h3>{display(provider.name)}</h3>
                    {Array.isArray(provider.devices) &&
                      provider.devices.map((device, i) => (
                        <p key={i}>{display(device)}</p>
                      ))}
                  </div>
                  {button(
                    {
                      module: "automation",
                      action: "open-source",
                      label: `Support ${display(provider.name)}`,
                      payload: { id: provider.id },
                    },
                    "Ouvrir ↗",
                  )}
                </div>
              ))}
              {has("detect-sources") &&
                !list(results["detect-sources"], "providers").length && (
                  <p>
                    Aucun fabricant avec support intégré identifié. Consultez
                    aussi Windows Update et les autres fabricants ci-dessous.
                  </p>
                )}
              {list(results["detect-sources"], "otherManufacturers").length >
                0 && (
                <section>
                  <h2>Autres fabricants détectés</h2>
                  <p>
                    Ces fabricants sont identifiés ; aucun lien de
                    téléchargement n’est intégré pour eux. Windows Update reste
                    disponible pour leurs pilotes applicables.
                  </p>
                  {list(results["detect-sources"], "otherManufacturers").map(
                    (item, i) => (
                      <div className="wb-row" key={i}>
                        <div>
                          <h3>{display(item.manufacturer)}</h3>
                          {Array.isArray(item.devices) &&
                            item.devices.map((device, j) => (
                              <p key={j}>{display(device)}</p>
                            ))}
                        </div>
                      </div>
                    ),
                  )}
                </section>
              )}
            </>
          )}
          {!titles[section] && (
            <p className="wb-empty">Cette section n’est pas prise en charge.</p>
          )}
        </div>
        <aside className="wb-panel wb-result" aria-labelledby={`${uid}-result`}>
          <div className="wb-intro">
            <span className="wb-eyebrow">RÉSULTAT</span>
            <h2 id={`${uid}-result`}>
              {last?.label ?? "En attente d’une opération"}
            </h2>
          </div>
          <div className="wb-result-body">
            {last ? (
              <Result data={last.data} action={last.action} />
            ) : (
              <p>
                Les données retournées par le service apparaîtront ici. Aucune
                opération n’est lancée automatiquement.
              </p>
            )}
          </div>
        </aside>
      </div>
      <dialog
        ref={dialog}
        className="wb-dialog"
        aria-labelledby={`${uid}-confirm`}
        aria-describedby={`${uid}-description`}
        onCancel={(event) => {
          event.preventDefault();
          closeDialog();
        }}
        onClose={() => setPending(null)}
      >
        <h2 id={`${uid}-confirm`}>{pending?.label}</h2>
        <p id={`${uid}-description`}>{pending?.confirmation}</p>
        {pending?.permanent && (
          <>
            <label className="wb-check-row">
              <input
                type="checkbox"
                checked={accepted}
                onChange={(e) => setAccepted(e.target.checked)}
              />
              <span>
                Je comprends que les fichiers seront définitivement supprimés.
              </span>
            </label>
            <label>
              Saisissez SUPPRIMER pour confirmer
              <input
                value={phrase}
                onChange={(e) => setPhrase(e.target.value)}
                autoComplete="off"
                spellCheck={false}
              />
            </label>
          </>
        )}
        <div className="wb-actions">
          <button
            type="button"
            className="wb-button"
            autoFocus
            onClick={closeDialog}
          >
            Annuler
          </button>
          <button
            type="button"
            className={`wb-button ${pending?.permanent ? "wb-danger" : "wb-primary"}`}
            disabled={
              busy ||
              (pending?.permanent && (!accepted || phrase !== "SUPPRIMER"))
            }
            onClick={() => {
              const job = pending;
              closeDialog();
              if (job) void run(job, true);
            }}
          >
            Confirmer
          </button>
        </div>
      </dialog>
    </section>
  );
}
