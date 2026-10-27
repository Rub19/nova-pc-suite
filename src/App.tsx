import {
  useEffect,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
} from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import Workbench from "./Workbench";
import AboutPC from "./AboutPC";
import AutomaticCleanup from "./AutomaticCleanup";
import { MemoryGuardian, NetworkTools } from "./HealthTools";
import {
  Activity,
  ArrowDownToLine,
  ArrowRight,
  Check,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Clock3,
  Cpu,
  Download,
  HardDrive,
  History,
  LayoutDashboard,
  LoaderCircle,
  Moon,
  MoreHorizontal,
  Play,
  RefreshCw,
  Search,
  Settings2,
  ShieldCheck,
  Sparkles,
  Sun,
  Zap,
  X,
  CircleHelp,
  MemoryStick,
  ExternalLink,
} from "lucide-react";
import {
  action,
  bytes,
  driverAuditCategory,
  driverCategory,
  elevate,
  Event,
  getHistory,
  healthScore,
  healthReport,
  isCriticalDriverUpdate,
  labels,
  memoryPercent,
  message,
  openSettings,
  recommendations,
  Reply,
  Row,
  rows,
  text,
} from "./lib";

const navGroups = [
  {
    id: "space",
    label: "Espace",
    items: [
      { id: "overview", label: "Vue d’ensemble", icon: LayoutDashboard },
      { id: "about-pc", label: "À propos de ce PC", icon: Cpu },
      { id: "measurements", label: "Avant / après", icon: Activity },
    ],
  },
  {
    id: "maintenance",
    label: "Entretien",
    items: [
      { id: "drivers", label: "Pilotes & mises à jour", icon: Download },
      { id: "optimize", label: "Optimisation", icon: Zap },
      { id: "network", label: "Diagnostic réseau", icon: Activity },
      { id: "memory", label: "Mémoire intelligente", icon: MemoryStick },
      { id: "cleanup", label: "Nettoyage récupérable", icon: HardDrive },
    ],
  },
  {
    id: "system",
    label: "Outils système",
    items: [
      { id: "applications", label: "Applications", icon: Download },
      { id: "storage", label: "Stockage", icon: HardDrive },
      { id: "startup", label: "Démarrage", icon: Play },
      { id: "processes", label: "Moniteur d’activité", icon: Activity },
      { id: "security", label: "Sécurité", icon: ShieldCheck },
    ],
  },
  {
    id: "nova",
    label: "Nova",
    items: [
      { id: "assistant", label: "Assistant Nova", icon: Sparkles },
      { id: "suggestions", label: "Suggestions Nova", icon: CircleHelp },
      { id: "automation", label: "Planification & Nova", icon: Clock3 },
      { id: "driver-sources", label: "Fabricants détectés", icon: Cpu },
      { id: "history", label: "Historique", icon: History },
    ],
  },
];
const nav = navGroups.flatMap((group) => group.items);
const navGroupIds = new Set(navGroups.map((group) => group.id));

function readCollapsedGroups() {
  try {
    const saved = JSON.parse(
      localStorage.getItem("nova-collapsed-nav") ?? "[]",
    );
    return Array.isArray(saved)
      ? saved.filter(
          (group): group is string =>
            typeof group === "string" && navGroupIds.has(group),
        )
      : [];
  } catch {
    return [];
  }
}
type Confirm = {
  action: string;
  description: string;
  ids?: string[];
  eulas?: { title: string; text: string }[];
};
const settings: Record<string, string> = {
  drivers: "windows-update",
  storage: "storage",
  startup: "startup",
  security: "security",
};

export default function App() {
  const savedPage = localStorage.getItem("nova-last-page");
  const [page, setPage] = useState(
    nav.some((item) => item.id === savedPage) ? savedPage! : "overview",
  );
  const [collapsedGroups, setCollapsedGroups] = useState(readCollapsedGroups);
  const [query, setQuery] = useState("");
  const [activeSearchResult, setActiveSearchResult] = useState(-1);
  const [data, setData] = useState<Record<string, Row>>({});
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [events, setEvents] = useState<Event[]>([]);
  const [confirm, setConfirm] = useState<Confirm | null>(null);
  const [accepted, setAccepted] = useState(false);
  const [restoreBeforeDrivers, setRestoreBeforeDrivers] = useState(true);
  const [dark, setDark] = useState(
    localStorage.getItem("nova-theme") !== "light",
  );
  const [compact, setCompact] = useState(
    () => localStorage.getItem("nova-compact-ui") === "enabled",
  );
  const [lastScan, setLastScan] = useState<Date | null>(null);
  const [healthHistory, setHealthHistory] = useState<
    { score: number; at: number }[]
  >(() => {
    try {
      const saved = JSON.parse(
        localStorage.getItem("nova-health-history") ?? "[]",
      );
      return Array.isArray(saved)
        ? saved
            .filter(
              (item) =>
                typeof item?.score === "number" && typeof item?.at === "number",
            )
            .slice(-20)
        : [];
    } catch {
      return [];
    }
  });
  const [question, setQuestion] = useState("");
  const [answer, setAnswer] = useState("");
  const [aiBusy, setAiBusy] = useState(false);
  const dialog = useRef<HTMLDialogElement>(null);
  const searchInput = useRef<HTMLInputElement>(null);
  const operation = useRef(false);
  const snapshot = data.snapshot;
  const memory = snapshot?.memory as Row | undefined;
  const os = snapshot?.os as Row | undefined;
  const cpu = rows(snapshot, "cpu")[0];
  const usedMemory = memoryPercent(snapshot);
  const volumes = rows(data.storage, "volumes");
  const installed = rows(data.drivers);
  const updates = rows(data.updates);
  const driverAudit = data["driver-audit"];
  const driverProblems = rows(driverAudit, "problemDevices");
  const missingDriverDevices = rows(driverAudit, "missingDriverDevices");
  const criticalDriverProblems = rows(driverAudit, "criticalProblemDevices");
  const driverAuditCategories = rows(driverAudit, "categories");
  const [driverSelection, setDriverSelection] = useState<string[]>([]);
  const [driverCategoryFilter, setDriverCategoryFilter] = useState("Tout");
  const [driverAuditFilter, setDriverAuditFilter] = useState<
    "all" | "critical" | "missing"
  >("all");
  useEffect(() => {
    setDriverSelection(rows(data.updates).map((u) => text(u.id)));
  }, [data.updates]);
  const selectedUpdates = updates.filter((u) =>
    driverSelection.includes(text(u.id)),
  );
  const driverInstallRows = rows(data["install-drivers"], "results").map(
    (result) => {
      const update = rows(data["install-drivers"], "items").find(
        (item) => text(item.id) === text(result.id),
      );
      return {
        ...result,
        source: "Windows Update",
        title: update ? text(update.title) : "Pilote Windows Update",
        category: update ? text(update.driverClass) : "Indisponible",
      };
    },
  );
  const visibleDriverProblems =
    driverAuditFilter === "missing"
      ? missingDriverDevices
      : driverAuditFilter === "critical"
        ? criticalDriverProblems
        : driverProblems;
  const advice = recommendations(snapshot, data.storage);
  const health = healthScore(
    snapshot,
    data.storage,
    data.security,
    updates.length,
  );
  const attentionItems = [
    snapshot?.pendingReboot && {
      title: "Redémarrage recommandé",
      detail: "Windows signale une opération en attente.",
      target: "optimize",
    },
    updates.length > 0 && {
      title: `${updates.length} pilote${updates.length > 1 ? "s" : ""} à examiner`,
      detail: "Windows Update propose des mises à jour applicables.",
      target: "drivers",
    },
    missingDriverDevices.length > 0 && {
      title: `${missingDriverDevices.length} pilote${missingDriverDevices.length > 1 ? "s" : ""} manquant${missingDriverDevices.length > 1 ? "s" : ""}`,
      detail: "Windows a signalé un périphérique sans pilote.",
      target: "drivers",
    },
  ].filter(Boolean) as { title: string; detail: string; target: string }[];
  function downloadHealthReport(format: "txt" | "json") {
    const report = healthReport(health);
    const content = format === "txt" ? report.text : report.json;
    const blob = new Blob([content], {
      type: format === "txt" ? "text/plain;charset=utf-8" : "application/json",
    });
    const link = document.createElement("a");
    link.href = URL.createObjectURL(blob);
    link.download = `nova-bilan-sante.${format}`;
    link.click();
    URL.revokeObjectURL(link.href);
  }
  function downloadDriverReport(format: "txt" | "json") {
    const report = {
      generatedAt: new Date().toISOString(),
      audit: driverAudit ?? {},
      installed: installed.map((item) => ({
        name: item.name,
        manufacturer: item.manufacturer,
        version: item.version,
        date: item.date,
        signed: item.signed,
      })),
      updates: updates.map((item) => ({
        title: item.title,
        driverClass: item.driverClass,
        driverManufacturer: item.driverManufacturer,
        driverModel: item.driverModel,
        driverVerDate: item.driverVerDate,
      })),
      privacy: "Exclut les identifiants matériels.",
    };
    const content =
      format === "json"
        ? JSON.stringify(report, null, 2)
        : [
            "Rapport pilotes Nova",
            `Généré : ${report.generatedAt}`,
            `Appareils en erreur : ${driverProblems.length}`,
            `Pilotes manquants : ${missingDriverDevices.length}`,
            `Mises à jour Windows Update : ${updates.length}`,
          ].join("\n");
    const link = document.createElement("a");
    link.href = URL.createObjectURL(
      new Blob([content], {
        type:
          format === "json" ? "application/json" : "text/plain;charset=utf-8",
      }),
    );
    link.download = `nova-pilotes.${format}`;
    link.click();
    setTimeout(() => URL.revokeObjectURL(link.href), 1000);
  }
  function clearHealthHistory() {
    localStorage.removeItem("nova-health-history");
    setHealthHistory([]);
  }
  useEffect(() => {
    if (!snapshot) return;
    setHealthHistory((previous) => {
      const next = [...previous, { score: health.score, at: Date.now() }].slice(
        -20,
      );
      localStorage.setItem("nova-health-history", JSON.stringify(next));
      return next;
    });
  }, [snapshot]);
  const suggestionTools = [
    {
      icon: Download,
      title: updates.length
        ? `${updates.length} mise${updates.length > 1 ? "s" : ""} à jour à examiner`
        : "Vérifier les pilotes et mises à jour",
      detail: updates.length
        ? "Des mises à jour applicables sont prêtes à être examinées avant toute installation."
        : "Recherchez les mises à jour proposées pour votre matériel par Windows Update.",
      target: "drivers",
      label: "Voir les pilotes",
    },
    {
      icon: Download,
      title: "Gérer les logiciels",
      detail:
        "Consultez vos applications et leurs mises à jour disponibles, sans désinstaller ni modifier quoi que ce soit automatiquement.",
      target: "applications",
      label: "Voir les logiciels",
    },
    {
      icon: Play,
      title: "Alléger le démarrage",
      detail:
        "Identifiez les applications lancées à l’ouverture de session et désactivez uniquement celles que vous choisissez.",
      target: "startup",
      label: "Gérer le démarrage",
    },
    {
      icon: HardDrive,
      title: "Préserver votre espace disque",
      detail:
        "Analysez le stockage et les fichiers temporaires avant toute opération de nettoyage.",
      target: "storage",
      label: "Analyser le stockage",
    },
    {
      icon: ShieldCheck,
      title: "Contrôler les protections Windows",
      detail:
        "Vérifiez antivirus et pare-feu depuis Nova, puis ouvrez les réglages Windows lorsque c’est nécessaire.",
      target: "security",
      label: "Voir la sécurité",
    },
  ];
  const guidedSteps = [
    {
      title: "Diagnostic du PC",
      detail: "Matériel, mémoire et état Windows",
      target: "overview",
      complete: Boolean(snapshot),
    },
    {
      title: "Stockage",
      detail: "Volumes locaux et espace disponible",
      target: "storage",
      complete: Boolean(data.storage),
    },
    {
      title: "Pilotes",
      detail: "Inventaire local et mises à jour applicables",
      target: "drivers",
      complete: Boolean(data.drivers || data.updates),
    },
    {
      title: "Protections",
      detail: "Antivirus et profils de pare-feu",
      target: "security",
      complete: Boolean(data.security),
    },
  ];
  useEffect(() => {
    document.documentElement.dataset.theme = dark ? "dark" : "light";
    localStorage.setItem("nova-theme", dark ? "dark" : "light");
  }, [dark]);
  useEffect(() => {
    localStorage.setItem("nova-compact-ui", compact ? "enabled" : "disabled");
  }, [compact]);
  useEffect(() => {
    localStorage.setItem("nova-last-page", page);
  }, [page]);
  useEffect(() => {
    localStorage.setItem("nova-collapsed-nav", JSON.stringify(collapsedGroups));
  }, [collapsedGroups]);
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        searchInput.current?.focus();
      }
      if (event.key === "F5" && !(event.target instanceof HTMLInputElement)) {
        event.preventDefault();
        void run("snapshot");
      }
      if (
        event.key === "Escape" &&
        document.activeElement === searchInput.current
      ) {
        setQuery("");
        searchInput.current?.blur();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);
  useEffect(() => {
    setAccepted(false);
    if (confirm) dialog.current?.showModal();
    else dialog.current?.close();
  }, [confirm]);
  async function run(
    id: string,
    approved = false,
    ids: string[] = [],
    acceptEulas = false,
  ) {
    if (operation.current) return;
    operation.current = true;
    setBusy(id);
    setError("");
    setNotice("");
    try {
      const reply = await action(id, approved, ids, acceptEulas);
      if (reply.data) setData((old) => ({ ...old, [id]: reply.data! }));
      if (!reply.ok) setError(message(reply.error));
      else {
        if (id === "snapshot") setLastScan(new Date());
        setNotice(`${labels[id] ?? id} : terminé.`);
      }
      setEvents(await getHistory());
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
    } finally {
      operation.current = false;
      setBusy(null);
    }
  }
  async function runCompleteDriverScan() {
    if (operation.current) return;
    operation.current = true;
    setBusy("driver-complete");
    setError("");
    setNotice("");
    const actions = ["drivers", "driver-audit", "updates"];
    const failures: string[] = [];
    try {
      for (const id of actions) {
        const reply = await action(id, false, [], false);
        if (reply.data) setData((old) => ({ ...old, [id]: reply.data! }));
        if (!reply.ok) failures.push(message(reply.error));
      }
      if (failures.length) setError(failures.join(" "));
      else
        setNotice(
          "Analyse complète des pilotes terminée. Examinez les propositions Windows Update avant installation.",
        );
      setEvents(await getHistory());
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
    } finally {
      operation.current = false;
      setBusy(null);
    }
  }
  async function runProtectedDriverInstall(confirm: Confirm) {
    if (operation.current) return;
    operation.current = true;
    setBusy("install-drivers");
    setError("");
    setNotice("");
    try {
      if (restoreBeforeDrivers) {
        const restore = await action("restore", true, [], false);
        if (!restore.ok) {
          setError(
            `Point de restauration non créé : ${message(restore.error)}. Aucun pilote n’a été installé.`,
          );
          return;
        }
        if (restore.data)
          setData((old) => ({ ...old, restore: restore.data! }));
      }
      const installed = await action(
        "install-drivers",
        true,
        confirm.ids,
        accepted,
      );
      if (installed.data)
        setData((old) => ({ ...old, "install-drivers": installed.data! }));
      if (!installed.ok) setError(message(installed.error));
      else
        setNotice(
          "Installation terminée. Consultez le bilan et redémarrez Windows si nécessaire.",
        );
      setEvents(await getHistory());
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
    } finally {
      operation.current = false;
      setBusy(null);
    }
  }
  useEffect(() => {
    void run("snapshot");
    if (isTauri())
      void invoke<string>("startup_page")
        .then((id) => {
          if (nav.some((item) => item.id === id)) setPage(id);
        })
        .catch(() => {});
  }, []);
  useEffect(() => {
    if (!isTauri()) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen<string>("nova:navigate", ({ payload }) => {
      if (!disposed && nav.some((item) => item.id === payload))
        changePage(payload);
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
  async function settingsOpen(target: string) {
    try {
      await openSettings(target);
    } catch (e) {
      setError(String(e));
    }
  }
  async function admin() {
    try {
      const result = await elevate(page);
      if (!result.ok) setError(message(result.error));
      else
        setNotice(
          "Transition vers Nova administrateur en cours. La fenêtre actuelle sera remplacée.",
        );
    } catch (e) {
      setError(String(e));
    }
  }
  function changePage(id: string) {
    const group = navGroups.find((candidate) =>
      candidate.items.some((item) => item.id === id),
    );
    if (group)
      setCollapsedGroups((current) =>
        current.filter((groupId) => groupId !== group.id),
      );
    setPage(id);
    setQuery("");
    setError("");
    setNotice("");
    if (id === "history") void getHistory().then(setEvents);
  }
  const filteredNav = query
    ? navGroups.flatMap((group) =>
        group.items
          .filter((item) =>
            item.label.toLowerCase().includes(query.toLowerCase()),
          )
          .map((item) => ({ ...item, group: group.label })),
      )
    : [];
  function selectSearchResult(index: number) {
    const result = filteredNav[index];
    if (result) changePage(result.id);
  }
  function handleSearchKeyDown(event: ReactKeyboardEvent<HTMLInputElement>) {
    if (!filteredNav.length) return;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setActiveSearchResult((current) =>
        current >= filteredNav.length - 1 ? 0 : current + 1,
      );
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      setActiveSearchResult((current) =>
        current <= 0 ? filteredNav.length - 1 : current - 1,
      );
    }
    if (event.key === "Enter" && activeSearchResult >= 0) {
      event.preventDefault();
      selectSearchResult(activeSearchResult);
    }
  }
  function filtered(items: Row[]) {
    return items.filter((x) =>
      Object.values(x)
        .map(text)
        .join(" ")
        .toLowerCase()
        .includes(query.toLowerCase()),
    );
  }
  const metric = (
    label: string,
    value: string,
    subtitle: string,
    Icon: typeof Cpu,
    percent?: number | null,
    pendingLabel?: string,
  ) => (
    <article className="metric">
      <div className="metric-title">
        <span>{label}</span>
        <Icon size={18} />
      </div>
      {pendingLabel ? (
        <strong className="metric-pending">{pendingLabel}</strong>
      ) : (
        <strong>{value}</strong>
      )}
      <span className="muted">{subtitle}</span>
      {percent != null && (
        <progress max="100" value={percent} aria-label={label} />
      )}
    </article>
  );
  const scanButton = (id: string, label = "Analyser") => (
    <button
      className="button secondary"
      onClick={() => void run(id)}
      disabled={!!busy}
    >
      {busy === id ? (
        <LoaderCircle className="spin" size={16} />
      ) : (
        <RefreshCw size={16} />
      )}{" "}
      {label}
    </button>
  );
  const heading = nav.find((n) => n.id === page)?.label ?? "Nova";

  return (
    <div className={`app${compact ? " compact-ui" : ""}`}>
      <a className="skip-link" href="#main-content">
        Aller au contenu principal
      </a>
      <aside className="sidebar">
        <a
          className="brand"
          href="#"
          onClick={(e) => {
            e.preventDefault();
            changePage("overview");
          }}
        >
          <span className="brand-mark">
            n<span />
          </span>
          <span>
            nova<small>PC SUITE</small>
          </span>
        </a>
        <nav aria-label="Navigation principale">
          {navGroups.map((group) => {
            const collapsed = collapsedGroups.includes(group.id);
            return (
              <section className="nav-group" key={group.id}>
                <button
                  className="nav-group-toggle"
                  aria-expanded={!collapsed}
                  onClick={() =>
                    setCollapsedGroups((current) =>
                      current.includes(group.id)
                        ? current.filter((id) => id !== group.id)
                        : [...current, group.id],
                    )
                  }
                >
                  <span>{group.label}</span>
                  <ChevronDown size={14} aria-hidden="true" />
                </button>
                {!collapsed &&
                  group.items.map(({ id, label, icon: Icon }) => (
                    <button
                      key={id}
                      className={`nav-item ${page === id ? "active" : ""}`}
                      aria-current={page === id ? "page" : undefined}
                      onClick={() => changePage(id)}
                    >
                      <Icon size={19} />
                      <span>{label}</span>
                      {id === "drivers" && updates.length > 0 && (
                        <span className="count">{updates.length}</span>
                      )}
                    </button>
                  ))}
              </section>
            );
          })}
        </nav>
        <div className="sidebar-bottom">
          <div className="local-badge">
            <span className="status-dot" />
            <span>
              Vos données restent locales<small>Aucune télémétrie</small>
            </span>
          </div>
          <button
            className="admin-button"
            onClick={() => void admin()}
            disabled={!!busy || snapshot?.isAdmin === true}
          >
            <ShieldCheck size={17} />
            {snapshot?.isAdmin
              ? "Session administrateur"
              : "Passer en administrateur"}
            <ChevronRight size={14} />
          </button>
          <div className="sidebar-footer">
            <span>
              Nova <span className="version">v0.2.13</span>
            </span>
            <button
              aria-label={
                dark ? "Activer le thème clair" : "Activer le thème sombre"
              }
              onClick={() => setDark(!dark)}
            >
              {dark ? <Sun size={18} /> : <Moon size={18} />}
            </button>
            <button
              aria-label={
                compact
                  ? "Désactiver le mode compact"
                  : "Activer le mode compact"
              }
              onClick={() => setCompact(!compact)}
            >
              <MoreHorizontal size={18} />
            </button>
          </div>
        </div>
      </aside>
      <div className="main-shell">
        <header className="topbar">
          <div className="breadcrumb">
            Mon espace <ChevronRight size={14} />
            <span>{heading}</span>
          </div>
          <div className="search-wrap">
            <Search size={17} />
            <input
              ref={searchInput}
              aria-label="Rechercher dans Nova et les résultats"
              aria-autocomplete="list"
              aria-controls="nova-search-results"
              aria-expanded={Boolean(query)}
              aria-activedescendant={
                activeSearchResult >= 0
                  ? `nova-search-result-${activeSearchResult}`
                  : undefined
              }
              placeholder="Rechercher dans Nova…"
              value={query}
              onChange={(e) => {
                setQuery(e.target.value);
                setActiveSearchResult(-1);
              }}
              onKeyDown={handleSearchKeyDown}
            />
            {!query && <kbd aria-hidden="true">Ctrl K</kbd>}
            {query && (
              <button
                aria-label="Effacer la recherche"
                onClick={() => setQuery("")}
              >
                <X size={14} />
              </button>
            )}
            {query && (
              <div
                className="search-results"
                id="nova-search-results"
                role="listbox"
              >
                {filteredNav.map((n, index) => (
                  <button
                    id={`nova-search-result-${index}`}
                    key={n.id}
                    role="option"
                    aria-selected={activeSearchResult === index}
                    className={
                      activeSearchResult === index ? "active" : undefined
                    }
                    onMouseEnter={() => setActiveSearchResult(index)}
                    onClick={() => selectSearchResult(index)}
                  >
                    <span>
                      {n.label}
                      <small>{n.group}</small>
                    </span>
                    <ArrowRight size={15} />
                  </button>
                ))}
                {filteredNav.length === 0 && (
                  <p className="search-empty">Aucune rubrique trouvée.</p>
                )}
              </div>
            )}
          </div>
        </header>
        <main id="main-content" tabIndex={-1}>
          <div className="page-heading">
            <div>
              <div className="eyebrow">
                {page === "overview"
                  ? "UN PEU D’ATTENTION. BEAUCOUP DE PERFORMANCE."
                  : "VOTRE CENTRE DE MAINTENANCE"}
              </div>
              <h1>
                {page === "overview"
                  ? "Votre PC, au meilleur de lui-même."
                  : heading}
              </h1>
              <p>
                {page === "overview"
                  ? "Prenez soin de votre ordinateur. On vous accompagne à chaque étape."
                  : page === "drivers"
                    ? "Les bons pilotes, pour le bon matériel. Directement depuis Windows Update."
                    : page === "optimize"
                      ? "Des actions concrètes pour entretenir Windows et votre matériel."
                      : "Des informations locales et des actions à portée de main."}
              </p>
            </div>
            {page === "overview" && (
              <button
                className="button primary"
                onClick={() => void run("snapshot")}
                disabled={!!busy}
              >
                <Activity size={17} />
                Analyser mon PC
              </button>
            )}
          </div>
          {busy && (
            <div className="running" role="status">
              <LoaderCircle className="spin" size={18} />
              <div>
                <strong>{labels[busy]} en cours</strong>
                <span>
                  {busy === "updates" || busy === "install-drivers"
                    ? "Windows Update peut prendre plusieurs minutes. Gardez Nova ouvert."
                    : busy === "repair"
                      ? "DISM puis SFC : cette opération peut prendre jusqu’à deux heures."
                      : "Collecte des informations Windows…"}
                </span>
              </div>
              <span className="live-pill">EN COURS</span>
            </div>
          )}
          {error && (
            <div className="alert error" role="alert">
              <CircleHelp size={19} />
              <span>{error}</span>
              <button
                aria-label="Fermer le message"
                onClick={() => setError("")}
              >
                <X size={16} />
              </button>
            </div>
          )}
          {notice && !busy && (
            <div className="alert success" role="status">
              <CheckCircle2 size={18} />
              <span>{notice}</span>
              <button
                aria-label="Fermer le message"
                onClick={() => setNotice("")}
              >
                <X size={16} />
              </button>
            </div>
          )}

          {page === "overview" && (
            <>
              {attentionItems.length > 0 && (
                <section
                  className="attention-center"
                  aria-label="Points à examiner"
                >
                  <div>
                    <span className="tag neutral">À EXAMINER</span>
                    <h2>Vos priorités</h2>
                  </div>
                  {attentionItems.slice(0, 3).map((item) => (
                    <button
                      key={item.title}
                      onClick={() => changePage(item.target)}
                    >
                      <span>
                        <strong>{item.title}</strong>
                        <small>{item.detail}</small>
                      </span>
                      <ArrowRight size={16} />
                    </button>
                  ))}
                </section>
              )}
              <section className="hero">
                <div className="hero-main">
                  <div className="hero-icon">
                    <Activity size={30} />
                  </div>
                  <div>
                    <span className="tag">DIAGNOSTIC LOCAL</span>
                    <h2>
                      {snapshot
                        ? "Tout commence par un état des lieux."
                        : "Faisons connaissance avec votre PC."}
                    </h2>
                    <p>
                      {snapshot
                        ? `${text(os?.caption)} · ${text(os?.architecture)} · ${text(cpu?.cores)} cœurs`
                        : "Lancez une analyse pour consulter les informations réelles de votre système."}
                    </p>
                    <div className="hero-meta">
                      <Clock3 size={14} />
                      {lastScan
                        ? `Dernière analyse à ${lastScan.toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" })}`
                        : "Aucune analyse terminée"}
                    </div>
                  </div>
                </div>
                <div className="hero-side">
                  <span className="signal">
                    <span />
                    <span />
                    <span />
                    <span />
                  </span>
                  <span>
                    Une base saine.
                    <br />
                    <strong>Un PC qui dure.</strong>
                  </span>
                </div>
              </section>
              <section
                className="health-summary"
                aria-label="Bilan de santé local"
              >
                <div
                  className={`health-score ${health.score < 60 ? "attention" : ""}`}
                >
                  <strong>{health.score}</strong>
                  <span>/ 100</span>
                </div>
                <div className="health-copy">
                  <span className="tag neutral">BILAN LOCAL</span>
                  <h2>{health.level}</h2>
                  <p>
                    Score indicatif calculé uniquement avec les derniers
                    diagnostics Nova. Il ne remplace pas un diagnostic matériel.
                  </p>
                </div>
                <div className="health-actions">
                  {health.items.length ? (
                    health.items.slice(0, 2).map((item) => (
                      <button
                        key={item.title}
                        onClick={() => changePage(item.target)}
                      >
                        <span>{item.title}</span>
                        <ArrowRight size={15} />
                      </button>
                    ))
                  ) : (
                    <span className="health-clear">
                      <CheckCircle2 size={16} /> Aucun point prioritaire détecté
                    </span>
                  )}
                  <div className="health-export">
                    <button onClick={() => downloadHealthReport("txt")}>
                      Rapport .txt
                    </button>
                    <button onClick={() => downloadHealthReport("json")}>
                      Rapport .json
                    </button>
                  </div>
                </div>
              </section>
              {healthHistory.length > 1 && (
                <section
                  className="health-trend"
                  aria-label="Évolution du bilan de santé"
                >
                  <span>Évolution des derniers bilans</span>
                  <svg
                    viewBox="0 0 200 32"
                    role="img"
                    aria-label="Historique du score santé"
                  >
                    <polyline
                      points={healthHistory
                        .map(
                          (item, index) =>
                            `${(index * 200) / Math.max(1, healthHistory.length - 1)},${32 - item.score * 0.32}`,
                        )
                        .join(" ")}
                      fill="none"
                      stroke="var(--blue)"
                      strokeWidth="2"
                    />
                  </svg>
                  <button className="quiet-action" onClick={clearHealthHistory}>
                    Effacer l’historique
                  </button>
                </section>
              )}
              <section className="metrics" aria-label="Mesures système">
                {metric(
                  "Processeur",
                  cpu?.loadPercent != null ? `${text(cpu.loadPercent)} %` : "—",
                  cpu ? text(cpu.name) : "En attente du diagnostic",
                  Cpu,
                  cpu?.loadPercent != null ? Number(cpu.loadPercent) : null,
                )}
                {metric(
                  "Mémoire",
                  usedMemory !== null ? `${usedMemory} %` : "—",
                  memory
                    ? `${bytes(memory.freeBytes)} disponibles sur ${bytes(memory.totalBytes)}`
                    : "En attente du diagnostic",
                  MemoryStick,
                  usedMemory,
                )}
                {metric(
                  "Stockage",
                  volumes.length
                    ? bytes(
                        volumes.reduce(
                          (sum, v) => sum + Number(v.freeBytes),
                          0,
                        ),
                      )
                    : "—",
                  volumes.length
                    ? "Espace disponible sur les volumes locaux"
                    : "Lancez l’analyse du stockage",
                  HardDrive,
                  null,
                  volumes.length ? undefined : "Analyse requise",
                )}
                {metric(
                  "Pilotes installés",
                  data.drivers ? String(installed.length) : "—",
                  data.drivers
                    ? "Inventoriés sur votre ordinateur"
                    : "Lancez l’inventaire des pilotes",
                  Download,
                  null,
                  data.drivers ? undefined : "Inventaire requis",
                )}
              </section>
              <div className="section-heading">
                <div>
                  <h2>Un ordinateur bien entretenu.</h2>
                  <p>Vos essentiels, réunis au même endroit.</p>
                </div>
                <span className="quiet-label">À VOTRE RYTHME</span>
              </div>
              <section className="feature-grid">
                <Feature
                  icon={Download}
                  tone="blue"
                  title="Gardez une longueur d’avance."
                  text="Recherchez les pilotes compatibles proposés pour votre PC, puis installez-les en une seule opération."
                  label="Vérifier mes pilotes"
                  onClick={() => changePage("drivers")}
                />
                <Feature
                  icon={Zap}
                  tone="amber"
                  title="Retrouvez le bon équilibre."
                  text="Réparez Windows, entretenez votre SSD et choisissez le profil d’alimentation adapté à votre usage."
                  label="Explorer les optimisations"
                  onClick={() => changePage("optimize")}
                />
                <Feature
                  icon={HardDrive}
                  tone="green"
                  title="Faites de la place à l’essentiel."
                  text="Repérez les volumes qui manquent d’espace et accédez aux outils de nettoyage de Windows."
                  label="Analyser mon stockage"
                  onClick={() => changePage("storage")}
                />
              </section>
              <div className="bottom-grid">
                <section className="panel">
                  <div className="panel-head">
                    <h2>Vos prochaines étapes</h2>
                    <span className="tag neutral">PERSONNALISÉ</span>
                  </div>
                  {advice.length ? (
                    advice.map((a) => (
                      <button
                        className="step"
                        key={a.title}
                        onClick={() => changePage(a.target)}
                      >
                        <span className="step-dot" />
                        <div>
                          <strong>{a.title}</strong>
                          <small>{a.detail}</small>
                        </div>
                        <ArrowRight size={17} />
                      </button>
                    ))
                  ) : (
                    <>
                      <button
                        className="step"
                        onClick={() => changePage("drivers")}
                      >
                        <span className="step-number">1</span>
                        <div>
                          <strong>Vérifiez vos mises à jour de pilotes</strong>
                          <small>Une recherche dédiée à votre matériel.</small>
                        </div>
                        <ArrowRight size={17} />
                      </button>
                      <button
                        className="step"
                        onClick={() => changePage("security")}
                      >
                        <span className="step-number">2</span>
                        <div>
                          <strong>Faites le point sur vos protections</strong>
                          <small>Antivirus et profils de pare-feu.</small>
                        </div>
                        <ArrowRight size={17} />
                      </button>
                    </>
                  )}
                </section>
                <section className="assistant-teaser">
                  <Sparkles size={23} />
                  <span className="tag">ASSISTANT NOVA</span>
                  <h2>Comprendre, c’est déjà améliorer.</h2>
                  <p>
                    Des conseils locaux fondés sur vos mesures. Une IA locale
                    peut aussi répondre via Ollama.
                  </p>
                  <button onClick={() => changePage("assistant")}>
                    Faire le point <ArrowRight size={17} />
                  </button>
                </section>
              </div>
            </>
          )}

          {page === "drivers" && (
            <>
              <section className="panel action-banner">
                <div className="feature-icon blue">
                  <Download size={25} />
                </div>
                <div className="grow">
                  <h2>Votre matériel mérite les bons pilotes.</h2>
                  <p>
                    Windows Update recherche les mises à jour applicables. Nova
                    compare aussi votre inventaire local et détecte les
                    fabricants présents pour vous guider vers leur support
                    officiel.
                  </p>
                </div>
                <div className="action-banner-actions">
                  <button
                    className="button primary"
                    onClick={() => void runCompleteDriverScan()}
                    disabled={!!busy}
                  >
                    {busy === "driver-complete" ? (
                      <LoaderCircle className="spin" size={16} />
                    ) : (
                      <Search size={16} />
                    )}
                    Analyser tous les pilotes
                  </button>
                  {scanButton("updates", "Rechercher les mises à jour")}
                  <button
                    className="button secondary"
                    onClick={() => changePage("driver-sources")}
                  >
                    Fabricants détectés
                    <ArrowRight size={15} />
                  </button>
                </div>
              </section>
              <section
                className="panel driver-audit"
                aria-label="Audit local des appareils"
              >
                <div>
                  <span className="tag neutral">LECTURE SEULE</span>
                  <h2>État des appareils et des pilotes</h2>
                  <p>
                    Vérifie les erreurs déclarées par Windows et les métadonnées
                    de signature. Cet audit ne télécharge, n’installe ni ne
                    remplace aucun pilote.
                  </p>
                </div>
                <div className="driver-audit-actions">
                  {scanButton("driver-audit", "Auditer les appareils")}
                  <button
                    className="button secondary"
                    disabled={!!busy}
                    onClick={() => void runCompleteDriverScan()}
                  >
                    Réanalyser après installation
                  </button>
                  <button
                    className="button secondary"
                    disabled={!driverAudit}
                    onClick={() => downloadDriverReport("txt")}
                  >
                    Rapport .txt
                  </button>
                  <button
                    className="button secondary"
                    disabled={!driverAudit}
                    onClick={() => downloadDriverReport("json")}
                  >
                    Rapport .json
                  </button>
                  <button
                    className="button secondary"
                    onClick={() => changePage("driver-sources")}
                  >
                    Supports officiels <ArrowRight size={15} />
                  </button>
                </div>
                {driverAudit && (
                  <div className="driver-audit-result" aria-live="polite">
                    <span>
                      {text(driverAudit.deviceCount)} appareils relevés
                    </span>
                    <span>
                      {text(driverAudit.signedDriverCount)} pilotes déclarés
                      signés
                    </span>
                    <span>
                      {text(driverAudit.unsignedDriverCount)} à vérifier
                    </span>
                    <span>
                      {missingDriverDevices.length} pilote
                      {missingDriverDevices.length > 1 ? "s" : ""} manquant
                      {missingDriverDevices.length > 1 ? "s" : ""} signalé
                      {missingDriverDevices.length > 1 ? "s" : ""}
                    </span>
                    {driverAuditCategories.length > 0 && (
                      <div
                        className="driver-category-summary"
                        aria-label="Répartition des appareils par catégorie"
                      >
                        {driverAuditCategories.map((category) => (
                          <span key={text(category.category)}>
                            {text(category.category)} · {text(category.count)}
                          </span>
                        ))}
                      </div>
                    )}
                    {driverProblems.length ? (
                      <div className="driver-problems">
                        <div className="driver-audit-actions">
                          <button
                            className="button secondary"
                            onClick={() => setDriverAuditFilter("all")}
                          >
                            Tous
                          </button>
                          <button
                            className="button secondary"
                            onClick={() => setDriverAuditFilter("critical")}
                          >
                            Critiques
                          </button>
                          <button
                            className="button secondary"
                            onClick={() => setDriverAuditFilter("missing")}
                          >
                            Pilotes manquants
                          </button>
                        </div>
                        <strong>
                          {driverProblems.length} appareil
                          {driverProblems.length > 1 ? "s" : ""} demande
                          {driverProblems.length > 1 ? "nt" : ""} une attention
                        </strong>
                        {visibleDriverProblems
                          .slice(0, 10)
                          .map((device, index) => (
                            <p key={`${text(device.name)}-${index}`}>
                              {driverAuditCategory(device)} ·{" "}
                              {text(device.name) || "Appareil non nommé"} · code
                              Windows {text(device.errorCode)}
                            </p>
                          ))}
                        {criticalDriverProblems.length > 0 && (
                          <small>
                            Priorité : {criticalDriverProblems.length}{" "}
                            problème(s) concerne(nt) le graphique, réseau,
                            stockage ou chipset.
                          </small>
                        )}
                        <small>
                          Ouvrez les supports officiels ou recherchez les mises
                          à jour Windows avant toute modification.
                        </small>
                      </div>
                    ) : (
                      <span className="health-clear">
                        <CheckCircle2 size={16} /> Aucun appareil en erreur
                        signalé par Windows
                      </span>
                    )}
                  </div>
                )}
              </section>
              <div className="toolbar">
                <div>
                  <h2>
                    Mises à jour disponibles{" "}
                    {data.updates && (
                      <span className="inline-count">{updates.length}</span>
                    )}
                  </h2>
                  <p>La liste est vérifiée à nouveau avant l’installation.</p>
                </div>
                <button
                  className="button primary"
                  disabled={!!busy || !selectedUpdates.length}
                  onClick={() =>
                    setConfirm({
                      action: "install-drivers",
                      ids: selectedUpdates.map((u) => text(u.id)),
                      eulas: selectedUpdates
                        .filter((u) => u.eulaAccepted !== true)
                        .map((u) => ({
                          title: text(u.title),
                          text: text(u.eulaText),
                        })),
                      description: `Installer les ${selectedUpdates.length} pilotes sélectionnés via Windows Update. Une session administrateur est nécessaire. Un point de restauration est conseillé. Un redémarrage peut être requis ; Nova ne le déclenche pas.`,
                    })
                  }
                >
                  <ArrowDownToLine size={16} />
                  {selectedUpdates.length === updates.length
                    ? "Tout installer"
                    : `Installer la sélection (${selectedUpdates.length})`}
                </button>
                <button
                  className="button secondary"
                  disabled={!!busy || !updates.length}
                  onClick={() =>
                    setDriverSelection(
                      updates
                        .filter(isCriticalDriverUpdate)
                        .map((u) => text(u.id)),
                    )
                  }
                >
                  Sélectionner les pilotes critiques
                </button>
              </div>
              {data.updates ? (
                updates.length ? (
                  <div className="table-container">
                    <table>
                      <thead>
                        <tr>
                          <th>Sélection</th>
                          <th>Mise à jour</th>
                          <th>Catégorie</th>
                          <th>Fabricant / modèle</th>
                          <th>Date proposée</th>
                          <th>Licence acceptée</th>
                          <th>Téléchargé</th>
                        </tr>
                      </thead>
                      <tbody>
                        {filtered(updates).map((u) => (
                          <tr key={text(u.id)}>
                            <td>
                              <input
                                type="checkbox"
                                aria-label={`Sélectionner ${text(u.title)}`}
                                checked={driverSelection.includes(text(u.id))}
                                disabled={!!busy}
                                onChange={(e) =>
                                  setDriverSelection((old) =>
                                    e.target.checked
                                      ? [...old, text(u.id)]
                                      : old.filter((id) => id !== text(u.id)),
                                  )
                                }
                              />
                            </td>
                            <td>{text(u.title)}</td>
                            <td>{text(u.driverClass)}</td>
                            <td>
                              {text(u.driverManufacturer)} ·{" "}
                              {text(u.driverModel)}
                            </td>
                            <td>{text(u.driverVerDate)}</td>
                            <td>{text(u.eulaAccepted)}</td>
                            <td>{text(u.downloaded)}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                ) : (
                  <Empty
                    icon={CheckCircle2}
                    title="Aucune mise à jour proposée"
                    description="Windows Update ne propose actuellement aucun pilote applicable. Cela ne garantit pas l’absence d’une version plus récente chez le constructeur."
                  />
                )
              ) : (
                <Empty
                  icon={Search}
                  title="Prêt à rechercher"
                  description="Lancez une recherche en ligne pour connaître les pilotes proposés par Windows Update."
                />
              )}
              <div className="toolbar">
                <div>
                  <h2>Pilotes installés</h2>
                  <p>
                    Inventaire local, catégorie, version et signature numérique.
                  </p>
                </div>
                <div className="action-banner-actions">
                  <label className="inline-select">
                    Catégorie
                    <select
                      className="nova-select"
                      value={driverCategoryFilter}
                      onChange={(event) =>
                        setDriverCategoryFilter(event.target.value)
                      }
                    >
                      {[
                        "Tout",
                        "Graphique",
                        "Réseau",
                        "Audio",
                        "Stockage",
                        "Chipset",
                        "Périphérique",
                      ].map((item) => (
                        <option key={item}>{item}</option>
                      ))}
                    </select>
                  </label>
                  {scanButton("drivers", "Actualiser l’inventaire")}
                </div>
              </div>
              {data.drivers ? (
                <DataTable
                  items={filtered(
                    installed.filter(
                      (driver) =>
                        driverCategoryFilter === "Tout" ||
                        driverCategory(driver) === driverCategoryFilter,
                    ),
                  ).map((driver) => ({
                    ...driver,
                    category: driverCategory(driver),
                  }))}
                  columns={[
                    ["category", "Catégorie"],
                    ["name", "Périphérique"],
                    ["manufacturer", "Fabricant"],
                    ["version", "Version"],
                    ["date", "Date"],
                    ["signed", "Signé"],
                  ]}
                />
              ) : (
                <Empty
                  icon={Cpu}
                  title="Votre inventaire est à un clic"
                  description="L’analyse de l’inventaire est locale et ne modifie aucun pilote."
                />
              )}
              {data["install-drivers"] && (
                <section className="panel result-panel">
                  <h2>Résultats de l’installation</h2>
                  <p>
                    {data["install-drivers"].rebootRequired
                      ? "Un redémarrage est requis. Enregistrez votre travail puis redémarrez Windows."
                      : "Aucun redémarrage signalé par cette opération."}
                  </p>
                  <p>
                    Codes Windows : 2 = réussi, 3 = partiellement réussi, 4 =
                    échec, 5 = annulé.
                  </p>
                  <DataTable
                    items={driverInstallRows}
                    columns={[
                      ["title", "Pilote"],
                      ["category", "Catégorie"],
                      ["source", "Source"],
                      ["resultCode", "Résultat"],
                      ["hresult", "Code Windows"],
                      ["rebootRequired", "Redémarrage"],
                    ]}
                  />
                </section>
              )}
              <button
                className="text-button"
                onClick={() => void settingsOpen("windows-update")}
              >
                Ouvrir Windows Update <ExternalLink size={14} />
              </button>
            </>
          )}

          {page === "optimize" && (
            <>
              <div className="info-strip">
                <ShieldCheck size={20} />
                <div>
                  <strong>Préparez votre filet de sécurité.</strong>
                  <span>
                    Créez un point de restauration avant une installation de
                    pilotes ou une réparation.
                  </span>
                </div>
                <button
                  className="button secondary"
                  disabled={!!busy}
                  onClick={() =>
                    setConfirm({
                      action: "restore",
                      description:
                        "Créer et vérifier un point de restauration Windows. La protection système doit être active. Windows peut limiter la fréquence de création. Droits administrateur requis.",
                    })
                  }
                >
                  Créer un point
                </button>
              </div>
              <div className="optimization-list">
                {[
                  {
                    id: "repair",
                    icon: Settings2,
                    title: "Réparer les composants de Windows",
                    detail:
                      "Exécute DISM RestoreHealth puis SFC pour vérifier et réparer les composants et fichiers système.",
                    impact: "Intégrité système",
                    description:
                      "Lancer DISM puis SFC. La réparation peut durer longtemps et télécharger des composants Microsoft. Un point de restauration est conseillé. Les codes de sortie seront affichés ; ils ne constituent pas à eux seuls une preuve que toute corruption a été corrigée.",
                  },
                  {
                    id: "trim",
                    icon: HardDrive,
                    title: "Entretenir le SSD système",
                    detail:
                      "Demande un ReTrim du volume système sur un stockage compatible. Les fichiers personnels sont conservés.",
                    impact: "Entretien du stockage",
                    description:
                      "Demander un ReTrim du volume système. Windows refusera si le volume ne prend pas en charge cette fonction. Cette action nécessite les droits administrateur.",
                  },
                  {
                    id: "energy-balanced",
                    icon: Activity,
                    title: "Privilégier l’équilibre",
                    detail:
                      "Active le profil Windows Équilibré pour adapter consommation et performances à la charge.",
                    impact: "Énergie & confort",
                    description:
                      "Activer le profil Équilibré Windows. Vous pouvez changer de profil à tout moment dans les paramètres Windows.",
                  },
                  {
                    id: "energy-performance",
                    icon: Zap,
                    title: "Privilégier la performance",
                    detail:
                      "Active le profil Performances élevées, si disponible. Peut augmenter la consommation, la chauffe et le bruit.",
                    impact: "Pour un usage exigeant",
                    description:
                      "Activer le profil Performances élevées. Ce profil peut accroître la chauffe et la consommation ; il ne garantit pas de gain de vitesse. Vous pourrez revenir au mode Équilibré.",
                  },
                ].map(
                  ({ id, icon: Icon, title, detail, impact, description }) => (
                    <article className="optimization-item" key={id}>
                      <div className="feature-icon blue">
                        <Icon size={22} />
                      </div>
                      <div className="grow">
                        <span className="eyebrow">{impact}</span>
                        <h2>{title}</h2>
                        <p>{detail}</p>
                        {data[id] && (
                          <p className="result">
                            Dernier résultat :{" "}
                            {Object.entries(data[id])
                              .map(([k, v]) => `${k} : ${text(v)}`)
                              .join(" · ")}
                          </p>
                        )}
                      </div>
                      <button
                        className="button secondary"
                        disabled={!!busy}
                        onClick={() => setConfirm({ action: id, description })}
                      >
                        Configurer
                        <ArrowRight size={15} />
                      </button>
                    </article>
                  ),
                )}
              </div>
              <p className="footnote">
                Les gains dépendent de l’état initial du PC. Aucune augmentation
                arbitraire de score ou promesse de FPS.
              </p>
            </>
          )}

          {page === "storage" && (
            <>
              <div className="toolbar">
                <div>
                  <h2>Vos volumes locaux</h2>
                  <p>Espace disponible et taux d’occupation.</p>
                </div>
                {scanButton("storage", "Analyser le stockage")}
              </div>
              {data.storage ? (
                <div className="volume-grid">
                  {volumes.map((v) => (
                    <article className="panel volume" key={text(v.drive)}>
                      <HardDrive size={26} />
                      <h2>Disque {text(v.drive)}</h2>
                      <p>{text(v.fileSystem)}</p>
                      <progress
                        value={Number(v.sizeBytes) - Number(v.freeBytes)}
                        max={Number(v.sizeBytes)}
                        aria-label={`Occupation de ${text(v.drive)}`}
                      />
                      <strong>{bytes(v.freeBytes)} libres</strong>
                      <p>sur {bytes(v.sizeBytes)}</p>
                    </article>
                  ))}
                </div>
              ) : (
                <Empty
                  icon={HardDrive}
                  title="Votre espace, en toute clarté"
                  description="Analysez les volumes locaux pour voir où l’espace manque."
                />
              )}
              <section className="panel action-banner">
                <Settings2 size={28} />
                <div className="grow">
                  <h2>Nettoyage et Assistant Stockage</h2>
                  <p>
                    Examinez les fichiers temporaires, les applications
                    volumineuses et les suggestions de nettoyage dans Windows.
                  </p>
                </div>
                <button
                  className="button secondary"
                  onClick={() => void settingsOpen("storage")}
                >
                  Ouvrir le nettoyage Windows
                  <ExternalLink size={15} />
                </button>
              </section>
            </>
          )}

          {(page === "startup" ||
            page === "processes" ||
            page === "security") && (
            <>
              <div className="toolbar">
                <div>
                  <h2>
                    {page === "startup"
                      ? "Applications lancées au démarrage"
                      : page === "processes"
                        ? "Processus et mémoire utilisée"
                        : "État des protections Windows"}
                  </h2>
                  <p>
                    {page === "startup"
                      ? "Réduisez le temps de démarrage en vérifiant les applications superflues."
                      : page === "processes"
                        ? "Mesures ponctuelles. Actualisez pour obtenir un nouvel état."
                        : "Inventaire des produits de sécurité et des profils de pare-feu."}
                  </p>
                </div>
                {scanButton(page, "Actualiser")}
              </div>
              {!data[page] ? (
                <Empty
                  icon={page === "security" ? ShieldCheck : Activity}
                  title="Lancez votre analyse"
                  description="Les résultats sont collectés sur ce PC à votre demande."
                />
              ) : page === "security" ? (
                <>
                  <DataTable
                    items={rows(data.security, "antivirus")}
                    columns={[
                      ["name", "Antivirus détecté"],
                      ["productState", "État Windows brut"],
                    ]}
                  />
                  <p className="footnote">
                    La présence d’un antivirus ne garantit pas sa protection
                    active. Vérifiez son état dans Sécurité Windows.
                  </p>
                  <DataTable
                    items={rows(data.security, "firewall")}
                    columns={[
                      ["name", "Profil pare-feu"],
                      ["enabled", "Activé (1 = oui, 2 = non)"],
                    ]}
                  />
                  {rows(data.security, "issues").length > 0 && (
                    <p>Certains contrôles sont indisponibles.</p>
                  )}
                  {Array.isArray(data.security.issues) &&
                    (data.security.issues as string[]).map((x) => (
                      <p className="footnote" key={x}>
                        {x}
                      </p>
                    ))}
                </>
              ) : (
                <DataTable
                  items={filtered(
                    page === "processes"
                      ? rows(data.processes).sort(
                          (a, b) =>
                            Number(b.workingSetBytes) -
                            Number(a.workingSetBytes),
                        )
                      : rows(data.startup),
                  )}
                  columns={
                    page === "processes"
                      ? [
                          ["name", "Processus"],
                          ["pid", "PID"],
                          ["workingSetBytes", "Mémoire"],
                          ["threads", "Threads"],
                        ]
                      : [["name", "Application"]]
                  }
                />
              )}
              {settings[page] && (
                <button
                  className="button secondary"
                  onClick={() => void settingsOpen(settings[page])}
                >
                  Ouvrir les paramètres Windows
                  <ExternalLink size={15} />
                </button>
              )}
            </>
          )}

          {page === "suggestions" && (
            <>
              <section className="suggestion-hero">
                <div className="feature-icon blue">
                  <Sparkles size={24} />
                </div>
                <div>
                  <span className="tag">GUIDE LOCAL</span>
                  <h2>Suggestions pour votre PC</h2>
                  <p>
                    Une feuille de route claire pour entretenir votre
                    ordinateur. Nova vous propose des outils ; aucune action
                    n’est appliquée sans votre choix.
                  </p>
                </div>
              </section>
              {advice.length > 0 && (
                <section className="panel suggestion-priority">
                  <div className="panel-head">
                    <div>
                      <h2>À regarder en priorité</h2>
                      <p>Basé sur le dernier diagnostic local.</p>
                    </div>
                    <span className="tag neutral">
                      {advice.length} CONSEIL{advice.length > 1 ? "S" : ""}
                    </span>
                  </div>
                  {advice.map((item) => (
                    <button
                      className="step"
                      key={item.title}
                      onClick={() => changePage(item.target)}
                    >
                      <span className="step-dot" />
                      <div>
                        <strong>{item.title}</strong>
                        <small>{item.detail}</small>
                      </div>
                      <ArrowRight size={17} />
                    </button>
                  ))}
                </section>
              )}
              <section className="panel guided-setup">
                <div className="panel-head">
                  <div>
                    <h2>Parcours conseillé</h2>
                    <p>
                      {guidedSteps.filter((step) => step.complete).length} /{" "}
                      {guidedSteps.length} vérifications effectuées dans cette
                      session.
                    </p>
                  </div>
                  <span className="tag neutral">À VOTRE RYTHME</span>
                </div>
                {guidedSteps.map((step, index) => (
                  <button
                    className="step"
                    key={step.title}
                    onClick={() => changePage(step.target)}
                  >
                    <span
                      className={step.complete ? "guided-done" : "step-number"}
                    >
                      {step.complete ? <Check size={14} /> : index + 1}
                    </span>
                    <div>
                      <strong>{step.title}</strong>
                      <small>
                        {step.complete
                          ? "Vérifié dans cette session"
                          : step.detail}
                      </small>
                    </div>
                    <ArrowRight size={17} />
                  </button>
                ))}
              </section>
              <section className="suggestion-grid" aria-label="Outils suggérés">
                {suggestionTools.map(
                  ({ icon: Icon, title, detail, target, label }) => (
                    <article className="suggestion-card" key={target}>
                      <Icon size={20} />
                      <h2>{title}</h2>
                      <p>{detail}</p>
                      <button onClick={() => changePage(target)}>
                        {label}
                        <ArrowRight size={15} />
                      </button>
                    </article>
                  ),
                )}
              </section>
            </>
          )}

          {page === "assistant" && (
            <>
              <section className="assistant-intro">
                <Sparkles size={30} />
                <h2>Faisons le point, ensemble.</h2>
                <p>
                  Nova interprète vos mesures localement. Pour une conversation
                  avec une IA, connectez un modèle Ollama installé sur votre PC.
                </p>
              </section>
              <section className="panel">
                <h2>Votre bilan local</h2>
                {!snapshot ? (
                  <p>Lancez d’abord le diagnostic depuis la vue d’ensemble.</p>
                ) : (
                  <>
                    <p>
                      {text(os?.caption)} · {text(cpu?.name)} ·{" "}
                      {bytes(memory?.totalBytes)} de mémoire
                    </p>
                    {advice.length ? (
                      advice.map((a) => (
                        <div className="advice" key={a.title}>
                          <h3>{a.title}</h3>
                          <p>{a.detail}</p>
                          <button
                            className="text-button"
                            onClick={() => changePage(a.target)}
                          >
                            Voir les détails
                            <ArrowRight size={14} />
                          </button>
                        </div>
                      ))
                    ) : (
                      <p>
                        Aucun seuil de pression mémoire dépassé dans la dernière
                        mesure. Analysez aussi le stockage et les pilotes pour
                        compléter ce bilan.
                      </p>
                    )}
                  </>
                )}
              </section>
              <section className="panel ai-panel">
                <h2>Poser une question à l’IA locale</h2>
                <p>
                  Ollama doit être lancé sur ce PC avec au moins un modèle
                  installé. Seules votre question et les mesures synthétiques
                  ci-dessus sont transmises à ce service local.
                </p>
                <form
                  onSubmit={async (e) => {
                    e.preventDefault();
                    if (!question.trim() || aiBusy) return;
                    setAiBusy(true);
                    setError("");
                    try {
                      if (!isTauri())
                        throw new Error(
                          "Disponible dans l’application Windows.",
                        );
                      const result = await invoke<string>("ask_local_ai", {
                        question,
                        context: JSON.stringify({
                          os: os?.caption,
                          cpu: cpu?.name,
                          memoryUsedPercent: usedMemory,
                          advice,
                        }),
                      });
                      setAnswer(result);
                    } catch (e) {
                      setError(String(e));
                    } finally {
                      setAiBusy(false);
                    }
                  }}
                >
                  <label htmlFor="question">Votre question</label>
                  <textarea
                    id="question"
                    maxLength={2000}
                    value={question}
                    onChange={(e) => setQuestion(e.target.value)}
                    placeholder="Comment réduire les ralentissements de mon PC ?"
                  />
                  <button
                    className="button primary"
                    disabled={aiBusy || !question.trim()}
                  >
                    {aiBusy ? (
                      <LoaderCircle size={16} className="spin" />
                    ) : (
                      <Sparkles size={16} />
                    )}
                    Demander à l’IA locale
                  </button>
                </form>
                {answer && <div className="ai-answer">{answer}</div>}
              </section>
            </>
          )}

          {page === "history" && (
            <>
              <section className="panel action-banner">
                <History size={26} />
                <div className="grow">
                  <h2>Votre maintenance, en toute transparence.</h2>
                  <p>
                    Les 100 dernières opérations de cette installation. Les
                    résultats détaillés restent affichés pendant la session.
                  </p>
                </div>
                <button
                  className="button secondary"
                  onClick={() => {
                    const blob = new Blob([JSON.stringify(events, null, 2)], {
                      type: "application/json",
                    });
                    const url = URL.createObjectURL(blob);
                    const a = document.createElement("a");
                    a.href = url;
                    a.download = "nova-historique.json";
                    a.click();
                    setTimeout(() => URL.revokeObjectURL(url), 1000);
                  }}
                >
                  <Download size={16} />
                  Exporter
                </button>
              </section>
              {events.length ? (
                <div className="panel">
                  {events.map((e, i) => (
                    <div className="history-row" key={`${e.timestamp}-${i}`}>
                      <span
                        className={`history-status ${e.ok ? "ok" : "failed"}`}
                      >
                        {e.ok ? <Check size={17} /> : <X size={17} />}
                      </span>
                      <div className="grow">
                        <strong>{labels[e.action] ?? e.action}</strong>
                        <small>
                          {new Date(e.timestamp * 1000).toLocaleString("fr-FR")}
                        </small>
                      </div>
                      <span>{e.ok ? "Terminé" : "À vérifier"}</span>
                      {e.result && (
                        <details className="wb-details">
                          <summary>Résultat enregistré</summary>
                          <pre className="wb-output">
                            {JSON.stringify(e.result, null, 2)}
                          </pre>
                        </details>
                      )}
                    </div>
                  ))}
                </div>
              ) : (
                <Empty
                  icon={History}
                  title="Une nouvelle histoire commence"
                  description="Vos opérations et leurs résultats apparaîtront ici."
                />
              )}
            </>
          )}
          {page === "network" && <NetworkTools />}
          <MemoryGuardian visible={page === "memory"} />
          {page === "about-pc" && <AboutPC visible />}
          <AutomaticCleanup visible={page === "cleanup"} />
          {page === "optimize" && <Workbench section="maintenance" />}
          {page === "startup" && <Workbench section="startup-manage" />}
          {[
            "cleanup",
            "applications",
            "automation",
            "measurements",
            "driver-sources",
          ].includes(page) && <Workbench section={page} />}
          <footer className="main-footer">
            <span>
              <ShieldCheck size={14} />
              Conçu pour Windows 10 & 11
            </span>
            <span>Votre PC. Vos décisions.</span>
          </footer>
        </main>
      </div>
      <dialog
        ref={dialog}
        onCancel={() => setConfirm(null)}
        aria-labelledby="confirm-title"
      >
        <div className="dialog-head">
          <ShieldCheck size={25} />
          <button aria-label="Fermer" onClick={() => setConfirm(null)}>
            <X size={20} />
          </button>
        </div>
        <h2 id="confirm-title">
          {confirm ? labels[confirm.action] : "Confirmer"}
        </h2>
        <p>{confirm?.description}</p>
        {!!confirm?.eulas?.length && (
          <div className="eula-section">
            {confirm.eulas.map((e, i) => (
              <details key={i}>
                <summary>{e.title} — licence</summary>
                <pre>{e.text}</pre>
              </details>
            ))}
            <label>
              <input
                type="checkbox"
                checked={accepted}
                onChange={(e) => setAccepted(e.target.checked)}
              />{" "}
              J’ai lu et j’accepte les licences des pilotes affichés ci-dessus.
            </label>
          </div>
        )}
        {confirm?.action === "install-drivers" && (
          <label className="confirm-option">
            <input
              type="checkbox"
              checked={restoreBeforeDrivers}
              onChange={(event) =>
                setRestoreBeforeDrivers(event.target.checked)
              }
            />
            Créer et vérifier un point de restauration avant l’installation
          </label>
        )}
        <div className="dialog-actions">
          <button className="button secondary" onClick={() => setConfirm(null)}>
            Annuler
          </button>
          <button
            className="button primary"
            disabled={!!confirm?.eulas?.length && !accepted}
            onClick={() => {
              if (confirm) {
                const c = confirm;
                setConfirm(null);
                if (c.action === "install-drivers")
                  void runProtectedDriverInstall(c);
                else void run(c.action, true, c.ids, accepted);
              }
            }}
          >
            Confirmer l’action
            <ArrowRight size={16} />
          </button>
        </div>
      </dialog>
    </div>
  );
}
function Feature({
  icon: Icon,
  tone,
  title,
  text: body,
  label,
  onClick,
}: {
  icon: typeof Cpu;
  tone: string;
  title: string;
  text: string;
  label: string;
  onClick: () => void;
}) {
  return (
    <article className="feature">
      <div className={`feature-icon ${tone}`}>
        <Icon size={23} />
      </div>
      <h3>{title}</h3>
      <p>{body}</p>
      <button onClick={onClick}>
        {label}
        <ArrowRight size={16} />
      </button>
    </article>
  );
}
function Empty({
  icon: Icon,
  title,
  description,
}: {
  icon: typeof Cpu;
  title: string;
  description: string;
}) {
  return (
    <div className="empty">
      <Icon size={30} />
      <h3>{title}</h3>
      <p>{description}</p>
    </div>
  );
}
function DataTable({
  items,
  columns,
}: {
  items: Row[];
  columns: [string, string][];
}) {
  const [limit, setLimit] = useState(100);
  return (
    <div className="table-container">
      <table>
        <thead>
          <tr>
            {columns.map(([key, label]) => (
              <th key={key}>{label}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {items.slice(0, limit).map((row, i) => (
            <tr key={i}>
              {columns.map(([key]) => (
                <td key={key}>
                  {key === "workingSetBytes" ? bytes(row[key]) : text(row[key])}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
      {!items.length && <p className="table-empty">Aucun résultat.</p>}
      {items.length > limit && (
        <button className="text-button" onClick={() => setLimit(limit + 100)}>
          Afficher 100 résultats supplémentaires ({items.length - limit}{" "}
          restants)
        </button>
      )}
    </div>
  );
}
