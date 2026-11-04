export type Page =
  | "bilan"
  | "pilotes"
  | "miseajour"
  | "nettoyage"
  | "demarrage"
  | "performances"
  | "reparer"
  | "journal";

export type Unit = "bytes" | "count" | "number" | "percent" | "text" | "date";
export type Measure = {
  label: string;
  before: number | string | null;
  after: number | string | null;
  unit: Unit;
};
export type Undo = {
  module: string;
  action: string;
  payload?: Record<string, unknown>;
  label: string;
};
export type ActionResult = {
  summary?: string;
  measures?: Measure[];
  undo?: Undo | null;
  [key: string]: unknown;
};

export type Scan = {
  isAdmin: boolean;
  timestamp: string;
  os: { name: string; release: string; build: string };
  computer: { manufacturer: string; model: string; board: string; bios: string };
  cpu: { name: string; cores: number; threads: number };
  memory: {
    totalBytes: number;
    freeBytes: number;
    modules: number;
    speedMHz: number | null;
  };
  gpus: { name: string; driverVersion: string; driverDate: string | null }[];
  uptimeSeconds: number;
  pendingReboot: boolean;
  systemDrive: string;
  volumes: { drive: string; label: string; sizeBytes: number; freeBytes: number }[];
  disks: {
    name: string;
    media: string;
    bus: string;
    sizeBytes: number;
    health: string;
    temperature: number | null;
    wear: number | null;
    errors: number | null;
  }[];
  deviceProblems: number;
  security: {
    defender: { running: boolean; realtime: boolean; signatureAgeDays: number } | null;
    otherAntivirus: string[];
    firewallOff: string[] | null;
  };
  stability: {
    unexpectedShutdowns: number;
    blueScreens: number;
    gpuHangs: number;
    gpuCodes: { code: string; count: number }[];
    appCrashes: number;
    topCrashes: { name: string; count: number }[];
    lastIncident: string | null;
  };
  lastUpdate: string | null;
  boots: { available: boolean; items: { date: string; ms: number }[] };
};

export type Level = "alert" | "warn" | "ok";
export type Finding = {
  id: string;
  level: Level;
  title: string;
  detail: string;
  page?: Page;
  cta?: string;
};

export const plural = (count: number, one: string, many: string) => `${count} ${count > 1 ? many : one}`;

const fr = new Intl.NumberFormat("fr-FR", { maximumFractionDigits: 1 });

export function formatBytes(value: number | null | undefined): string {
  if (value === null || value === undefined || !Number.isFinite(value)) return "inconnu";
  const units = ["o", "Ko", "Mo", "Go", "To"];
  let size = Math.abs(value);
  let unit = 0;
  while (size >= 1024 && unit < units.length - 1) {
    size /= 1024;
    unit++;
  }
  const digits = unit >= 3 && size < 100 ? 1 : 0;
  return `${new Intl.NumberFormat("fr-FR", { maximumFractionDigits: digits }).format(size)} ${units[unit]}`;
}

export function formatDuration(seconds: number): string {
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (days > 0) return `${days} j ${hours} h`;
  if (hours > 0) return `${hours} h ${minutes} min`;
  return `${minutes} min`;
}

export function formatDate(iso: string | null | undefined): string {
  if (!iso) return "inconnue";
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return date.toLocaleDateString("fr-FR", { day: "numeric", month: "short", year: "numeric" });
}

export function arr<T>(value: T[] | T | null | undefined): T[] {
  if (value === null || value === undefined) return [];
  return Array.isArray(value) ? value : [value];
}

export function compareFindings(previous: string[] | null, items: Finding[]) {
  const now = items.filter((item) => item.level !== "ok").map((item) => item.id);
  if (!previous) return { fresh: [] as string[], solved: 0 };
  return { fresh: now.filter((id) => !previous.includes(id)), solved: previous.filter((id) => !now.includes(id)).length };
}

export function journalDay(seconds: number): string {
  return new Date(seconds * 1000).toLocaleDateString("fr-FR", { weekday: "long", day: "numeric", month: "long" });
}

export function bySizeThenUsable<T extends { bytes: number; needsAdmin: boolean; blocked: boolean }>(list: T[]): T[] {
  const rank = (item: T) => (item.needsAdmin || item.blocked || item.bytes === 0 ? 1 : 0);
  return [...list].sort((a, b) => rank(a) - rank(b) || b.bytes - a.bytes);
}

export function formatDateTime(iso: string | number): string {
  const date = new Date(iso);
  return `${date.toLocaleDateString("fr-FR", { day: "numeric", month: "short" })} à ${date.toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" })}`;
}

export function formatMeasureValue(value: number | string | null, unit: Unit): string {
  if (value === null || value === undefined) return "inconnu";
  if (typeof value === "string" && unit !== "date") return value;
  switch (unit) {
    case "bytes":
      return formatBytes(Number(value));
    case "percent":
      return `${fr.format(Number(value))} %`;
    case "date":
      return typeof value === "string" && !/^\d{4}-/.test(value) ? value : formatDateTime(String(value));
    default:
      return fr.format(Number(value));
  }
}

export function measureDelta(measure: Measure): string | null {
  const { before, after, unit } = measure;
  if (typeof before !== "number" || typeof after !== "number" || unit === "text" || unit === "date") return null;
  const delta = after - before;
  if (Math.abs(delta) < (unit === "bytes" ? 1024 * 1024 : 0.5)) return "sans changement";
  const sign = delta > 0 ? "+" : "−";
  return `${sign}${formatMeasureValue(Math.abs(delta), unit)}`;
}

export const GPU_CODES: Record<string, string> = {
  "117": "pilote relancé après un blocage (TDR)",
  "141": "moteur graphique bloqué",
  "142": "délai dépassé par le pilote",
  "193": "capture de diagnostic graphique",
  "1a1": "blocage graphique en cours de fonctionnement",
  "1a8": "écran noir détecté par Windows",
  "1b8": "écran noir détecté dans le pilote",
};

export const DEVICE_CODES: Record<number, string> = {
  1: "Mal configuré. Réinstallez le pilote.",
  3: "Le pilote est peut-être endommagé, ou la mémoire manque.",
  10: "L'appareil ne démarre pas. Éteignez complètement le PC 30 secondes. Si l'erreur revient, mettez à jour son pilote.",
  12: "Pas assez de ressources libres pour cet appareil.",
  14: "Redémarrez le PC pour terminer l'installation.",
  18: "Réinstallez le pilote de cet appareil.",
  19: "Sa configuration dans le registre est endommagée.",
  21: "Windows est en train de retirer cet appareil. Redémarrez.",
  24: "Appareil absent, ou mal branché.",
  28: "Aucun pilote n'est installé pour cet appareil.",
  31: "Windows ne parvient pas à charger le pilote.",
  32: "Le service de cet appareil est désactivé.",
  37: "Le pilote a refusé de démarrer.",
  39: "Le pilote est absent ou endommagé.",
  41: "Le pilote est chargé mais l'appareil est introuvable.",
  43: "Windows a arrêté cet appareil après un problème signalé.",
  45: "Appareil débranché.",
  48: "Pilote bloqué car incompatible avec Windows.",
  52: "La signature du pilote n'a pas pu être vérifiée.",
};

export function findings(scan: Scan): Finding[] {
  const list: Finding[] = [];
  const push = (id: string, level: Level, title: string, detail: string, page?: Page, cta?: string) =>
    list.push({ id, level, title, detail, page, cta });

  const system = scan.volumes.find((v) => v.drive === scan.systemDrive);
  if (system && system.sizeBytes > 0) {
    const ratio = system.freeBytes / system.sizeBytes;
    const text = `${formatBytes(system.freeBytes)} libres sur ${formatBytes(system.sizeBytes)}.`;
    if (ratio < 0.1) push("space", "alert", `Le disque ${system.drive} est presque plein`, `${text} Windows ralentit et les mises à jour peuvent échouer.`, "nettoyage", "Nettoyer");
    else if (ratio < 0.2) push("space", "warn", `Le disque ${system.drive} se remplit`, text, "nettoyage", "Nettoyer");
    else push("space", "ok", `Espace disponible sur ${system.drive}`, text);
  }

  const badDisk = scan.disks.find((d) => d.health && d.health !== "Healthy");
  const errorDisk = scan.disks.find((d) => (d.errors ?? 0) > 0);
  const hotDisk = scan.disks.find((d) => (d.temperature ?? 0) >= 70);
  const wornDisk = scan.disks.find((d) => (d.wear ?? 0) >= 90);
  if (badDisk) push("disk", "alert", `${badDisk.name} signale un problème`, "Windows ne le considère plus comme sain. Sauvegardez vos fichiers maintenant.", "reparer", "Analyser");
  else if (errorDisk) push("disk", "alert", `${errorDisk.name} a des erreurs de lecture`, `${errorDisk.errors} erreurs non corrigées. Sauvegardez vos fichiers.`, "reparer", "Analyser");
  else if (hotDisk) push("disk", "warn", `${hotDisk.name} chauffe`, `${hotDisk.temperature} °C. Vérifiez la ventilation du boîtier.`);
  else if (wornDisk) push("disk", "warn", `${wornDisk.name} est usé`, `${wornDisk.wear} % de sa durée de vie consommée. Prévoyez son remplacement.`);
  else if (scan.disks.length) push("disk", "ok", "Disques sains", `${plural(scan.disks.length, "disque déclaré sain", "disques déclarés sains")} par Windows.`);

  const s = scan.stability;
  if (s.gpuHangs > 0) {
    const top = s.gpuCodes[0];
    const named = top ? ` Le plus fréquent : ${GPU_CODES[top.code] ?? `code ${top.code}`}.` : "";
    push("gpu", s.gpuHangs >= 5 ? "alert" : "warn", `La carte graphique a décroché ${s.gpuHangs} fois en 30 jours`, `Windows a enregistré ces blocages.${named} Essayez d'abord le dernier pilote du fabricant.`, "pilotes", "Voir les pilotes");
  }
  if (s.blueScreens > 0) push("bsod", "alert", `${plural(s.blueScreens, "écran bleu", "écrans bleus")} en 30 jours`, "Vérifiez les pilotes et les fichiers système.", "reparer", "Vérifier Windows");
  if (s.unexpectedShutdowns > 0) push("power", s.unexpectedShutdowns >= 3 ? "alert" : "warn", `${plural(s.unexpectedShutdowns, "arrêt brutal", "arrêts brutaux")} en 30 jours`, "Le PC s'est éteint sans passer par l'arrêt normal, à cause d'un plantage ou d'une coupure.", "reparer", "Vérifier Windows");
  if (!s.gpuHangs && !s.blueScreens && !s.unexpectedShutdowns) push("stable", "ok", "Aucun plantage système", "Aucun écran bleu ni arrêt brutal en 30 jours.");
  if (s.appCrashes >= 10) {
    const top = s.topCrashes[0];
    push("apps", "warn", `${s.appCrashes} plantages d'applications en 30 jours`, top ? `${top.name} revient le plus souvent (${top.count} fois).` : "");
  }

  if (scan.deviceProblems > 0) push("devices", "warn", `${plural(scan.deviceProblems, "appareil", "appareils")} en erreur`, "Windows ne les fait pas fonctionner correctement.", "pilotes", "Voir les appareils");
  else push("devices", "ok", "Tous les appareils fonctionnent", "Aucun appareil signalé en erreur.");

  if (scan.pendingReboot) push("reboot", "warn", "Un redémarrage est en attente", "Une mise à jour ne sera terminée qu'après le redémarrage.", "miseajour", "Voir les mises à jour");
  else if (scan.uptimeSeconds > 14 * 86400) push("uptime", "warn", `Allumé depuis ${formatDuration(scan.uptimeSeconds)}`, "Un redémarrage libère la mémoire et termine les opérations en attente.");

  if (scan.lastUpdate) {
    const days = (Date.now() - new Date(scan.lastUpdate).getTime()) / 86400000;
    if (days > 30) push("updates", "warn", "Windows n'a rien installé depuis plus d'un mois", `Dernière installation : ${formatDate(scan.lastUpdate)}.`, "miseajour", "Chercher");
    else push("updates", "ok", "Windows à jour récemment", `Dernière installation : ${formatDate(scan.lastUpdate)}.`);
  }

  const d = scan.security.defender;
  const third = scan.security.otherAntivirus.length > 0;
  if (!third && (!d || !d.running || !d.realtime)) push("av", "alert", "Protection antivirus inactive", "Aucun antivirus ne surveille ce PC en temps réel.");
  else if (d && d.realtime && d.signatureAgeDays > 7) push("av", "warn", "Définitions antivirus anciennes", `Dernière mise à jour il y a ${d.signatureAgeDays} jours.`, "miseajour", "Chercher");
  else push("av", "ok", "Antivirus actif", third ? scan.security.otherAntivirus.join(", ") : "Microsoft Defender surveille ce PC.");
  if (scan.security.firewallOff?.length) push("firewall", "warn", "Pare-feu partiellement coupé", `Profil désactivé : ${scan.security.firewallOff.join(", ")}.`);

  const used = 1 - scan.memory.freeBytes / scan.memory.totalBytes;
  if (used >= 0.9) push("memory", "warn", `Mémoire utilisée à ${Math.round(used * 100)} %`, "Fermez ce qui ne sert pas pour éviter les ralentissements.", "performances", "Voir les programmes");

  const boot = scan.boots.items[0];
  if (boot && boot.ms > 60000) push("boot", "warn", `Démarrage en ${Math.round(boot.ms / 1000)} s`, "Réduisez les programmes lancés au démarrage.", "demarrage", "Voir le démarrage");

  const order: Record<Level, number> = { alert: 0, warn: 1, ok: 2 };
  return list.sort((a, b) => order[a.level] - order[b.level]);
}

export function headline(items: Finding[]): string {
  const issues = items.filter((f) => f.level !== "ok").length;
  if (issues === 0) return "Rien à signaler sur ce PC.";
  return issues === 1 ? "Un point à traiter sur ce PC." : `${issues} points à traiter sur ce PC.`;
}

export function reportText(scan: Scan, items: Finding[]): string {
  return [
    `Nova PC Suite, bilan du ${formatDateTime(scan.timestamp)}`,
    `${scan.computer.manufacturer} ${scan.computer.model}`,
    `${scan.os.name} ${scan.os.release} (build ${scan.os.build})`,
    `${scan.cpu.name}, ${formatBytes(scan.memory.totalBytes)} de mémoire`,
    ...scan.gpus.map((g) => `${g.name}, pilote ${g.driverVersion}`),
    "",
    ...items.map((f) => `[${f.level === "ok" ? "OK" : f.level === "warn" ? "À voir" : "Urgent"}] ${f.title}. ${f.detail}`),
  ].join("\n");
}

export function bootComparison(
  boots: { date: string; ms: number }[],
  changes: { timestamp: number }[],
): { before: number; after: number } | null {
  if (!changes.length) return null;
  const change = Math.max(...changes.map((c) => c.timestamp * 1000));
  const average = (list: { ms: number }[]) => list.reduce((sum, b) => sum + b.ms, 0) / list.length;
  const before = boots.filter((b) => new Date(b.date).getTime() < change);
  const after = boots.filter((b) => new Date(b.date).getTime() > change);
  if (!before.length || !after.length) return null;
  return { before: average(before), after: average(after) };
}
