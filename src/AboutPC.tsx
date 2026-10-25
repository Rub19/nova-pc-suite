import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Cpu,
  MemoryStick,
  Monitor,
  CircuitBoard,
  HardDrive,
  Wifi,
  Volume2,
  Battery,
  Laptop,
  RefreshCw,
  Copy,
  Download,
} from "lucide-react";
import "./about-pc.css";
type Row = Record<string, unknown>;
type Field = [
  string,
  string,
  ("bytes" | "date" | "bool" | "positive" | "memoryType")?,
];
type Section = {
  id: string;
  title: string;
  icon: typeof Cpu;
  rows: Row[];
  fields: Field[];
  unavailable: boolean;
};
const object = (v: unknown): Row =>
  v && typeof v === "object" && !Array.isArray(v) ? (v as Row) : {};
const rows = (v: unknown): Row[] =>
  Array.isArray(v)
    ? (v.filter(
        (x) => x && typeof x === "object" && !Array.isArray(x),
      ) as Row[])
    : [];
export function hardwareValue(value: unknown, format?: Field[2]): string {
  if (value === null || value === undefined || value === "")
    return "Non disponible";
  if (format === "bool")
    return typeof value === "boolean"
      ? value
        ? "Oui"
        : "Non"
      : "Non disponible";
  if (format === "date") {
    const date = new Date(String(value));
    return Number.isNaN(date.getTime())
      ? "Non disponible"
      : date.toLocaleString("fr-FR");
  }
  if (format === "memoryType")
    return (
      (
        {
          24: "DDR3",
          26: "DDR4",
          34: "DDR5",
          30: "LPDDR4",
          35: "LPDDR5",
        } as Record<string, string>
      )[String(value)] ??
      (Number(value) > 2 ? `Type SMBIOS ${value}` : "Non disponible")
    );
  if (format === "bytes") {
    const n = Number(value);
    if (!Number.isFinite(n) || n <= 0) return "Non disponible";
    const unit = Math.min(4, Math.max(0, Math.floor(Math.log2(n) / 10)));
    return `${(n / 1024 ** unit).toLocaleString("fr-FR", { maximumFractionDigits: 1 })} ${["o", "Kio", "Mio", "Gio", "Tio"][unit]}`;
  }
  if (
    format === "positive" &&
    (Number(value) <= 0 || !Number.isFinite(Number(value)))
  )
    return "Non disponible";
  return typeof value === "number"
    ? value.toLocaleString("fr-FR")
    : String(value).trim() || "Non disponible";
}
export function hardwareSections(info: Row): Section[] {
  const unavailable = Array.isArray(info.unavailable) ? info.unavailable : [];
  const section = (
    id: string,
    title: string,
    icon: typeof Cpu,
    source: string,
    fields: Field[],
    classes: string[],
  ): Section => ({
    id,
    title,
    icon,
    fields,
    rows: Array.isArray(info[source])
      ? rows(info[source])
      : [object(info[source])],
    unavailable: classes.some((c) => unavailable.includes(c)),
  });
  return [
    section(
      "computer",
      "Ordinateur",
      Laptop,
      "computer",
      [
        ["manufacturer", "Fabricant"],
        ["model", "Modèle"],
        ["systemType", "Type de système"],
        ["installedMemoryBytes", "RAM physique déclarée", "bytes"],
        ["logicalProcessors", "Processeurs logiques", "positive"],
        ["hypervisorPresent", "Hyperviseur présent", "bool"],
      ],
      ["Win32_ComputerSystem"],
    ),
    section(
      "windows",
      "Windows",
      Monitor,
      "windows",
      [
        ["name", "Édition"],
        ["version", "Version"],
        ["build", "Build"],
        ["architecture", "Architecture"],
        ["installedUtc", "Date d’installation déclarée", "date"],
        ["lastBootUtc", "Dernier démarrage", "date"],
        ["usableMemoryBytes", "RAM utilisable par Windows", "bytes"],
      ],
      ["Win32_OperatingSystem"],
    ),
    section(
      "cpu",
      "Processeur",
      Cpu,
      "cpu",
      [
        ["name", "Modèle"],
        ["manufacturer", "Fabricant"],
        ["socket", "Socket"],
        ["cores", "Cœurs physiques", "positive"],
        ["threads", "Processeurs logiques", "positive"],
        ["maxClockMHz", "Fréquence maximale déclarée (MHz)", "positive"],
        ["l2CacheKiB", "Cache L2 (Kio)", "positive"],
        ["l3CacheKiB", "Cache L3 (Kio)", "positive"],
        ["virtualizationFirmware", "Virtualisation firmware déclarée", "bool"],
      ],
      ["Win32_Processor"],
    ),
    section(
      "memory",
      "Barrettes de mémoire",
      MemoryStick,
      "memory",
      [
        ["slot", "Emplacement"],
        ["bank", "Banque"],
        ["manufacturer", "Fabricant"],
        ["partNumber", "Référence"],
        ["capacityBytes", "Capacité", "bytes"],
        ["configuredClockMHz", "Fréquence configurée SMBIOS (MHz)", "positive"],
        ["smbiosType", "Type de mémoire", "memoryType"],
      ],
      ["Win32_PhysicalMemory"],
    ),
    section(
      "gpu",
      "Cartes graphiques et affichage",
      Monitor,
      "gpu",
      [
        ["name", "Carte graphique"],
        ["processor", "Puce"],
        ["driverVersion", "Version du pilote"],
        ["driverDateUtc", "Date du pilote", "date"],
        ["width", "Largeur déclarée (px)", "positive"],
        ["height", "Hauteur déclarée (px)", "positive"],
        ["refreshHz", "Rafraîchissement déclaré (Hz)", "positive"],
        ["status", "État déclaré"],
      ],
      ["Win32_VideoController"],
    ),
    section(
      "motherboard",
      "Carte mère",
      CircuitBoard,
      "motherboard",
      [
        ["manufacturer", "Fabricant"],
        ["product", "Modèle"],
        ["version", "Version"],
      ],
      ["Win32_BaseBoard"],
    ),
    section(
      "bios",
      "BIOS / firmware",
      CircuitBoard,
      "bios",
      [
        ["manufacturer", "Fabricant"],
        ["version", "Version"],
        ["dateUtc", "Date du firmware", "date"],
      ],
      ["Win32_BIOS"],
    ),
    section(
      "disks",
      "Disques physiques",
      HardDrive,
      "disks",
      [
        ["model", "Modèle"],
        ["sizeBytes", "Capacité", "bytes"],
        ["interface", "Interface déclarée"],
        ["media", "Support déclaré"],
        ["firmware", "Firmware"],
        ["status", "État déclaré (ne remplace pas SMART)"],
      ],
      ["Win32_DiskDrive"],
    ),
    section(
      "volumes",
      "Volumes et espace libre",
      HardDrive,
      "volumes",
      [
        ["drive", "Volume"],
        ["fileSystem", "Système de fichiers"],
        ["sizeBytes", "Capacité", "bytes"],
        ["freeBytes", "Espace libre", "bytes"],
      ],
      ["Win32_LogicalDisk"],
    ),
    section(
      "network",
      "Cartes réseau physiques",
      Wifi,
      "network",
      [
        ["name", "Carte"],
        ["manufacturer", "Fabricant"],
        ["enabled", "Connexion activée", "bool"],
        ["speedBitsPerSecond", "Débit de liaison déclaré (bit/s)", "positive"],
      ],
      ["Win32_NetworkAdapter"],
    ),
    section(
      "audio",
      "Périphériques audio",
      Volume2,
      "audio",
      [
        ["name", "Périphérique"],
        ["manufacturer", "Fabricant"],
        ["status", "État déclaré"],
      ],
      ["Win32_SoundDevice"],
    ),
    section(
      "batteries",
      "Batterie",
      Battery,
      "batteries",
      [
        ["name", "Batterie"],
        ["chargePercent", "Charge estimée (%)"],
        ["statusCode", "État batterie (code Windows)"],
      ],
      ["Win32_Battery"],
    ),
  ];
}
export default function AboutPC({ visible }: { visible: boolean }) {
  const [info, setInfo] = useState<Row>(),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [notice, setNotice] = useState(""),
    [query, setQuery] = useState("");
  const guard = useRef(false);
  async function scan() {
    if (guard.current) return;
    guard.current = true;
    setBusy(true);
    setError("");
    setNotice("");
    try {
      const reply = await invoke<{ ok: boolean; data?: Row }>(
        "extended_action",
        {
          module: "hardware",
          action: "hardware-info",
          payload: {},
          confirmed: false,
        },
      );
      if (!reply.ok || !reply.data)
        throw new Error(
          "L’inventaire matériel est indisponible. Réessayez l’analyse.",
        );
      setInfo(reply.data);
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
    } finally {
      guard.current = false;
      setBusy(false);
    }
  }
  useEffect(() => {
    if (visible && !info) void scan();
  }, [visible]);
  const sections = info
    ? hardwareSections(info).filter((s) =>
        `${s.title} ${s.rows.map((r) => s.fields.map(([k, label, f]) => `${label} ${hardwareValue(r[k], f)}`).join(" ")).join(" ")}`
          .toLocaleLowerCase("fr")
          .includes(query.trim().toLocaleLowerCase("fr")),
      )
    : [];
  function report() {
    return [
      "Nova — À propos de ce PC",
      `Relevé : ${hardwareValue(info?.timestampUtc, "date")}`,
      ...hardwareSections(info ?? {}).flatMap((s) => [
        "",
        s.title,
        ...s.rows.flatMap((r, i) => [
          ...(s.rows.length > 1 ? [`Élément ${i + 1}`] : []),
          ...s.fields.map(
            ([k, label, f]) => `${label} : ${hardwareValue(r[k], f)}`,
          ),
        ]),
        ...(s.unavailable
          ? ["Source indisponible"]
          : s.rows.length === 0
            ? ["Aucun élément déclaré"]
            : []),
      ]),
      "",
      ...(Array.isArray(info?.limitations) ? info.limitations.map(String) : []),
    ].join("\n");
  }
  async function copy() {
    try {
      await navigator.clipboard.writeText(report());
      setNotice("Rapport copié.");
    } catch {
      setError("Copie indisponible. Utilisez Exporter le rapport.");
    }
  }
  function download() {
    const url = URL.createObjectURL(
      new Blob([report()], { type: "text/plain;charset=utf-8" }),
    );
    const link = document.createElement("a");
    link.href = url;
    link.download = "Nova-informations-PC.txt";
    link.click();
    setTimeout(() => URL.revokeObjectURL(url), 1500);
    setNotice("Export du rapport demandé.");
  }
  return (
    <section
      hidden={!visible}
      className="about-pc"
      aria-label="À propos de ce PC"
    >
      <div className="info-strip">
        <Laptop size={26} />
        <div>
          <strong>
            {info
              ? [
                  object(info.computer).manufacturer,
                  object(info.computer).model,
                ]
                  .filter(Boolean)
                  .join(" ") || "Votre ordinateur"
              : "Votre fiche technique"}
          </strong>
          <span>
            Composants et configuration déclarés par Windows · relevé local en
            lecture seule.
          </span>
        </div>
      </div>
      <div className="toolbar">
        <button
          className="button primary"
          disabled={busy}
          onClick={() => void scan()}
        >
          <RefreshCw size={16} />
          {busy ? "Analyse en cours…" : "Actualiser les informations"}
        </button>
        <button
          className="button secondary"
          disabled={!info || busy}
          onClick={() => void copy()}
        >
          <Copy size={16} />
          Copier le rapport
        </button>
        <button
          className="button secondary"
          disabled={!info || busy}
          onClick={download}
        >
          <Download size={16} />
          Exporter le rapport
        </button>
      </div>
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
      {busy && <p role="status">Lecture des composants Windows…</p>}
      {info && (
        <>
          <div className="about-filter">
            <label>
              Rechercher un composant ou une information
              <input
                type="search"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="Processeur, RAM, pilote graphique…"
              />
            </label>
            <p>
              Relevé du {hardwareValue(info.timestampUtc, "date")} ·{" "}
              {sections.length} rubrique(s)
            </p>
          </div>
          <nav className="about-links" aria-label="Rubriques matérielles">
            {sections.map((s) => (
              <a href={`#pc-${s.id}`} key={s.id}>
                {s.title}
              </a>
            ))}
          </nav>
          <div className="about-grid">
            {sections.map(
              ({ id, title, icon: Icon, rows: items, fields, unavailable }) => (
                <section className="panel about-card" id={`pc-${id}`} key={id}>
                  <h2>
                    <Icon size={19} />
                    {title}
                  </h2>
                  {unavailable ? (
                    <p className="wb-note">
                      Cette source Windows est indisponible. Les valeurs
                      manquantes ne permettent pas de conclure à l’absence du
                      composant.
                    </p>
                  ) : null}
                  {items.length ? (
                    items.map((item, i) => (
                      <div className="about-device" key={i}>
                        {items.length > 1 && (
                          <h3>
                            {title} · {i + 1}
                          </h3>
                        )}
                        <dl>
                          {fields.map(([key, label, format]) => (
                            <div key={key}>
                              <dt>{label}</dt>
                              <dd>{hardwareValue(item[key], format)}</dd>
                            </div>
                          ))}
                        </dl>
                      </div>
                    ))
                  ) : (
                    <p>
                      {unavailable
                        ? "Informations non disponibles."
                        : "Aucun élément déclaré par cette source Windows."}
                    </p>
                  )}
                </section>
              ),
            )}
          </div>
          {!sections.length && (
            <p>Aucune rubrique ne correspond à la recherche.</p>
          )}
          <section className="panel">
            <h2>Précision des informations</h2>
            {Array.isArray(info.limitations) &&
              info.limitations.map((line, i) => <p key={i}>{String(line)}</p>)}
            <p>
              Le rapport reprend l’ensemble des rubriques, même lorsqu’un filtre
              est actif. Numéros de série, clés produit et adresses IP/MAC ne
              sont pas collectés par cet inventaire.
            </p>
          </section>
        </>
      )}
    </section>
  );
}
