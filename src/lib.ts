import { invoke, isTauri } from "@tauri-apps/api/core";

export type Row = Record<string, unknown>;
export type Reply = { ok: boolean; data?: Row; error?: string };
export type Event = {
  timestamp: number;
  action: string;
  ok: boolean;
  result?: Reply;
};
export const labels: Record<string, string> = {
  snapshot: "Diagnostic système",
  drivers: "Inventaire des pilotes",
  updates: "Recherche des mises à jour",
  "install-drivers": "Installation groupée des pilotes",
  startup: "Applications au démarrage",
  security: "État des protections",
  storage: "Analyse du stockage",
  processes: "Processus actifs",
  restore: "Point de restauration",
  repair: "Réparation Windows",
  trim: "Optimisation TRIM du SSD",
  "energy-balanced": "Mode équilibré",
  "energy-performance": "Mode performances élevées",
};
const errors: Record<string, string> = {
  AutomationUnavailable:
    "Windows n’a pas pu appliquer ce réglage de planification ou contacter GitHub.",
  MaintenanceBusy: "Une maintenance est déjà en cours dans Nova.",
  BaselineMissing: "Enregistrez d’abord une mesure de référence.",
  StorageVolumeUnsupported:
    "L’outil prend en charge le volume système fixe NTFS déclaré sain par Windows. Aucun autre volume n’a été modifié.",
  BootDiagnosticsUnavailable:
    "Le journal de performance du démarrage est indisponible ou son accès est refusé. Cela ne signifie pas que le démarrage est sain.",
  NativeCommandFailed:
    "Windows a signalé une erreur. Consultez le code et les diagnostics retournés.",
  MaintenanceFailed:
    "Cette opération n’est pas disponible ou n’a pas abouti sur ce PC.",
  InvalidBaseline:
    "La référence enregistrée est invalide. Enregistrez une nouvelle mesure.",
  WingetUnavailable:
    "WinGet est indisponible. Vérifiez l’installation de Programme d’installation d’application Microsoft.",
  WingetOperationFailed:
    "WinGet n’a pas terminé la demande. Consultez sa réponse ; des licences ou sources peuvent demander une intervention.",
  AppsOperationFailed:
    "Windows n’a pas pu terminer l’opération sur les applications.",
  StartupConflict:
    "Une entrée de démarrage du même nom existe déjà. Nova la conserve.",
  StartupChanged: "L’entrée a changé depuis la lecture. Actualisez la liste.",
  InvalidBackup:
    "La sauvegarde ne peut pas être validée. L’opération a été refusée.",
  OperationInProgress: "Une autre opération sur les applications est en cours.",
  PackageNotInstalledOrQueryFailed:
    "WinGet n’a pas confirmé ce paquet installé. Vérifiez l’identifiant exact et les sources.",
  MaintenanceAlreadyRunning:
    "Une autre fenêtre Nova effectue déjà une maintenance. Attendez sa fin.",
  PreviousMaintenanceInterrupted:
    "Une précédente maintenance a été interrompue. Vérifiez Windows Update et les journaux Windows avant de relancer.",
  AdministratorRequired:
    "Ouvrez Nova en administrateur avec le bouton en bas à gauche, puis relancez cette action.",
  DriverSearchIncomplete:
    "Windows Update n’a pas terminé la recherche. Vérifiez votre connexion et réessayez.",
  UnacceptedDriverEula:
    "Certains pilotes nécessitent une acceptation de licence. Installez-les depuis Windows Update.",
  DriverDownloadFailed:
    "Le téléchargement de certains pilotes a échoué. Aucune installation n’a été lancée.",
  RebootRequiredBeforeInstallation:
    "Windows attend un redémarrage avant de pouvoir installer les pilotes.",
  DriverInstallationIncomplete:
    "Certains pilotes n’ont pas pu être installés. Consultez les résultats individuels.",
  RepairCommandFailed:
    "La réparation Windows a signalé une erreur. Consultez les codes de résultat.",
  UnsupportedSystemVolume: "Ce volume ne permet pas cette opération TRIM.",
  PowerSchemeUnavailable:
    "Ce profil d’alimentation n’est pas disponible sur ce PC.",
  RestorePointNotCreated:
    "Le point de restauration n’a pas été confirmé. Vérifiez la protection du système ou la limite de fréquence Windows.",
  ApprovedUpdatesUnavailable:
    "La liste des mises à jour a changé. Relancez une recherche avant de confirmer.",
  InvalidApprovedIds:
    "Relancez une recherche et sélectionnez les pilotes proposés.",
  ApprovedDriverUnavailable:
    "La liste des mises à jour a changé. Relancez la recherche avant de confirmer.",
  ApprovedIdsRequired:
    "Relancez une recherche pour confirmer la liste de pilotes à installer.",
  TooManyApprovedIds:
    "La liste dépasse 200 pilotes. Utilisez Windows Update pour cette opération.",
  InvalidApprovedId:
    "Un identifiant de mise à jour est invalide. Relancez une recherche.",
  ActionFailed:
    "Windows n’a pas pu terminer cette opération. Vérifiez les permissions et la disponibilité du service.",
};
export function message(error?: string) {
  return errors[error ?? ""] ?? error ?? "Opération indisponible.";
}
export function rows(data: Row | undefined, key = "items"): Row[] {
  const value = data?.[key];
  return Array.isArray(value)
    ? value.filter((x): x is Row => typeof x === "object" && x !== null)
    : [];
}
export function text(value: unknown): string {
  return value === null || value === undefined
    ? "Indisponible"
    : typeof value === "boolean"
      ? value
        ? "Oui"
        : "Non"
      : typeof value === "object"
        ? JSON.stringify(value)
        : String(value);
}
export function bytes(value: unknown) {
  const n = Number(value);
  if (!Number.isFinite(n) || value == null) return "Indisponible";
  return (
    new Intl.NumberFormat("fr-FR", { maximumFractionDigits: 1 }).format(
      n / 1024 ** 3,
    ) + " Go"
  );
}
export function memoryPercent(data?: Row) {
  const memory = data?.memory as Row | undefined;
  const total = Number(memory?.totalBytes);
  const free = Number(memory?.freeBytes);
  return total > 0 && Number.isFinite(free)
    ? Math.max(0, Math.min(100, Math.round((1 - free / total) * 100)))
    : null;
}
export async function action(
  action: string,
  confirmed = false,
  approvedIds: string[] = [],
  acceptEulas = false,
): Promise<Reply> {
  if (!isTauri())
    throw new Error(
      "Les diagnostics nécessitent l’application Windows. Cet aperçu affiche uniquement l’interface.",
    );
  return invoke("system_action", {
    action,
    confirmed,
    approvedIds,
    acceptEulas,
  });
}
export async function getHistory(): Promise<Event[]> {
  return isTauri() ? invoke("history") : [];
}
export async function openSettings(target: string) {
  if (!isTauri()) throw new Error("Disponible dans l’application Windows.");
  const reply = await invoke<Reply>("open_settings", { target });
  if (!reply.ok) throw new Error("Impossible d’ouvrir ce panneau Windows.");
}
export async function elevate(page = "overview"): Promise<Reply> {
  if (!isTauri()) throw new Error("Disponible dans l’application Windows.");
  return invoke("elevate", { page });
}
export function recommendations(snapshot?: Row, storage?: Row) {
  const items: { title: string; detail: string; target: string }[] = [];
  const used = memoryPercent(snapshot);
  if (used !== null && used > 80)
    items.push({
      title: "Mémoire fortement sollicitée",
      detail: `${used} % de la mémoire est utilisée. Identifiez les applications les plus gourmandes.`,
      target: "processes",
    });
  for (const v of rows(storage, "volumes")) {
    const total = Number(v.sizeBytes);
    const free = Number(v.freeBytes);
    if (total > 0 && free / total < 0.15)
      items.push({
        title: `Espace faible sur ${text(v.drive)}`,
        detail: `${bytes(free)} disponibles. Examinez les catégories de stockage avant de nettoyer.`,
        target: "storage",
      });
  }
  if (snapshot?.pendingReboot === true)
    items.push({
      title: "Un redémarrage est en attente",
      detail:
        "Enregistrez votre travail et redémarrez depuis Windows pour terminer les mises à jour.",
      target: "drivers",
    });
  return items;
}

export type HealthItem = {
  title: string;
  detail: string;
  target: string;
  penalty: number;
};

export function healthScore(
  snapshot?: Row,
  storage?: Row,
  security?: Row,
  updateCount = 0,
) {
  const items: HealthItem[] = [];
  const used = memoryPercent(snapshot);
  if (used !== null && used > 80)
    items.push({
      title: "Mémoire sous pression",
      detail: `${used} % de la RAM est utilisée dans le dernier relevé.`,
      target: "processes",
      penalty: 15,
    });
  if (rows(storage, "volumes").some((volume) => {
    const total = Number(volume.sizeBytes);
    const free = Number(volume.freeBytes);
    return total > 0 && free / total < 0.15;
  }))
    items.push({
      title: "Espace disque faible",
      detail: "Un volume local dispose de moins de 15 % d’espace libre.",
      target: "storage",
      penalty: 20,
    });
  if (snapshot?.pendingReboot === true)
    items.push({
      title: "Redémarrage requis",
      detail: "Windows attend un redémarrage pour terminer une opération.",
      target: "drivers",
      penalty: 10,
    });
  if (updateCount > 0)
    items.push({
      title: "Pilotes à examiner",
      detail: `${updateCount} mise${updateCount > 1 ? "s" : ""} à jour proposée${updateCount > 1 ? "s" : ""} par Windows Update.`,
      target: "drivers",
      penalty: 5,
    });
  if (Array.isArray(security?.issues) && security.issues.length > 0)
    items.push({
      title: "État des protections incomplet",
      detail: "Au moins une protection Windows n’a pas pu être vérifiée.",
      target: "security",
      penalty: 20,
    });
  const score = Math.max(0, 100 - items.reduce((total, item) => total + item.penalty, 0));
  return {
    score,
    level: score >= 85 ? "Bon état" : score >= 60 ? "À surveiller" : "À vérifier",
    items,
  };
}

export function driverCategory(driver: Row) {
  const value = `${driver.name ?? ""} ${driver.manufacturer ?? ""}`.toLocaleLowerCase("fr");
  if (/nvidia|geforce|radeon|graphics|display|video|gpu/.test(value)) return "Graphique";
  if (/ethernet|wi-?fi|wireless|network|bluetooth|realtek.*(pcie|gbe)/.test(value)) return "Réseau";
  if (/audio|sound|speaker|microphone/.test(value)) return "Audio";
  if (/sata|nvme|storage|disk|raid|ahci/.test(value)) return "Stockage";
  if (/chipset|pci express|system management|i2c|smbus/.test(value)) return "Chipset";
  return "Périphérique";
}

export function healthReport(health: { score: number; level: string; items: HealthItem[] }) {
  const report = {
    generatedAt: new Date().toISOString(),
    score: health.score,
    level: health.level,
    findings: health.items.map(({ title, detail, penalty }) => ({ title, detail, penalty })),
    privacy: "Exclut les identifiants matériel, noms de processus, adresses réseau et numéros de série.",
  };
  return {
    json: JSON.stringify(report, null, 2),
    text: [
      "Bilan Nova — local",
      `Score : ${report.score}/100 (${report.level})`,
      "",
      ...(report.findings.length ? report.findings.map((item) => `- ${item.title} : ${item.detail}`) : ["- Aucun point prioritaire détecté."]),
      "",
      report.privacy,
    ].join("\n"),
  };
}
