import { useState } from "react";
import { ActionResult, formatDateTime } from "../lib";
import { AdminButton, call, Elapsed, errorText, Failure, Head, Measures, Section, useAction, useApp, useResource } from "../ui";

type Restore = { needsAdmin: boolean; items: { sequence: number; description: string; date: string }[]; protectionOn?: boolean | null };
type Outcome = ActionResult & { output?: string[]; verdict?: string; exitCode?: number; label: string };

const VERDICT: Record<string, string> = {
  ok: "Aucun problème trouvé.",
  repaired: "Problèmes trouvés et réparés.",
  problem: "Problème signalé. Lisez le détail ci-dessous.",
};

const TOOLS = [
  { title: "Image de Windows", detail: "La copie de référence dont Windows se sert pour se réparer.", check: "dism-check", checkLabel: "Analyser", repair: "dism-repair", time: "10 à 30 minutes" },
  { title: "Fichiers système", detail: "Compare les fichiers de Windows à leur version d'origine.", check: "sfc-check", checkLabel: "Vérifier", repair: "sfc-repair", time: "5 à 15 minutes" },
  { title: "Disque système", detail: "Cherche les erreurs du système de fichiers, sans arrêter le PC.", check: "chkdsk-scan", checkLabel: "Analyser", repair: null, time: "quelques minutes" },
];

export default function Reparer() {
  const { ask, toast } = useApp();
  const points = useResource<Restore>("restore", () => call("repair", "restore-list"));
  const { busy, run } = useAction();
  const [outcome, setOutcome] = useState<Outcome | null>(null);
  const launch = async (action: string, label: string) => {
    setOutcome(null);
    const r = await run("repair", action, undefined, { silent: action.endsWith("-check") || action === "chkdsk-scan" });
    if (r) setOutcome({ ...(r as ActionResult), label } as Outcome);
    if (r && ["restore-create", "dism-repair", "sfc-repair", "protection-enable"].includes(action)) void points.reload();
  };
  const repair = async (action: string, title: string, time: string) => {
    const yes = await ask({
      title: `Réparer : ${title}`,
      confirm: "Réparer",
      body: `Nova crée d'abord un point de restauration. Comptez ${time}. Vous pouvez utiliser le PC pendant ce temps, sans l'éteindre.`,
    });
    if (yes) await launch(action, title);
  };
  const cleanStore = async () => {
    const yes = await ask({
      title: "Supprimer les anciens composants",
      confirm: "Supprimer",
      body: "Windows retire les anciennes versions remplacées par des mises à jour. Elles ne pourront plus être restaurées par désinstallation d'une mise à jour déjà ancienne. Comptez plusieurs minutes.",
    });
    if (yes) await launch("store-cleanup", "Anciens composants");
  };
  const openRestore = async () => {
    try {
      await call("repair", "open-restore");
    } catch (e) {
      toast(errorText(e), "alert");
    }
  };
  return (
    <>
      <Head title="Réparer" intro="Quand Windows plante ou se comporte mal. Commencez par une analyse : elle ne modifie rien." />
      {busy && <Elapsed label="Windows travaille. Vous pouvez continuer à utiliser le PC." />}
      {outcome && (
        <div className="ticket" role="status">
          <p><strong>{outcome.label} : {VERDICT[outcome.verdict ?? ""] ?? outcome.summary}</strong></p>
          <Measures items={outcome.measures} />
          {!!outcome.output?.length && <pre className="log">{outcome.output.join("\n")}</pre>}
        </div>
      )}
      <Section title="Vérifications">
        <ul className="panel">
          {TOOLS.map((t) => (
            <li className="row" key={t.check}>
              <div className="grow">
                <strong>{t.title}</strong>
                <p>{t.detail} Durée : {t.time}.</p>
              </div>
              <AdminButton disabled={!!busy} onRun={() => void launch(t.check, t.title)}>{t.checkLabel}</AdminButton>
              {t.repair && <AdminButton disabled={!!busy} onRun={() => void repair(t.repair!, t.title, t.time)}>Réparer</AdminButton>}
            </li>
          ))}
          <li className="row">
            <div className="grow">
              <strong>Anciens composants de Windows</strong>
              <p>Libère l'espace pris par les versions remplacées par des mises à jour.</p>
            </div>
            <AdminButton disabled={!!busy} onRun={() => void cleanStore()}>Supprimer</AdminButton>
          </li>
        </ul>
      </Section>
      <Section
        title="Points de restauration"
        note="Ils ramènent Windows, ses pilotes et ses réglages à une date passée, sans toucher à vos documents."
        actions={
          <>
            <button onClick={() => void openRestore()}>Restaurer le système…</button>
            <AdminButton disabled={!!busy} onRun={() => void launch("restore-create", "Point de restauration")}>Créer un point</AdminButton>
          </>
        }
      >
        {points.error && <Failure text={points.error} retry={() => void points.reload()} />}
        {points.data?.needsAdmin && <p className="note">La liste s'affiche en mode administrateur.</p>}
        {points.data && !points.data.needsAdmin && points.data.items.length === 0 && (
          <div className="failure">
            <span>Aucun point de restauration. Si la protection du système est coupée, Nova ne peut pas installer de pilote ou de mise à jour en sécurité.</span>
            <AdminButton disabled={!!busy} onRun={() => void launch("protection-enable", "Protection du système")}>Activer la protection</AdminButton>
          </div>
        )}
        {!!points.data?.items.length && (
          <ul className="panel">
            {points.data.items.map((p) => (
              <li className="row" key={p.sequence}>
                <div className="grow">
                  <strong>{p.description}</strong>
                  <p>{formatDateTime(p.date)}</p>
                </div>
              </li>
            ))}
          </ul>
        )}
      </Section>
    </>
  );
}
