import { CircleAlert, CircleCheck, TriangleAlert } from "lucide-react";
import { useMemo } from "react";
import { Finding, findings, formatBytes, formatDate, formatDuration, headline, plural, reportText, Scan } from "../lib";
import { call, errorText, Failure, Section, Skeleton, useApp, useResource } from "../ui";

const ICON = { alert: CircleAlert, warn: TriangleAlert, ok: CircleCheck };

function Row({ item }: { item: Finding }) {
  const { go } = useApp();
  const Icon = ICON[item.level];
  return (
    <li className={`row finding ${item.level}`}>
      <Icon size={18} aria-hidden className="finding-icon" />
      <div className="grow">
        <strong>{item.title}</strong>
        <p>{item.detail}</p>
      </div>
      {item.page && <button onClick={() => go(item.page!)}>{item.cta}</button>}
    </li>
  );
}

function Machine({ scan }: { scan: Scan }) {
  const facts: [string, string][] = [
    ["Système", `${scan.os.name.replace("Microsoft ", "")} ${scan.os.release}, build ${scan.os.build}`],
    ["Carte mère", `${scan.computer.board}, BIOS ${scan.computer.bios}`],
    ["Processeur", `${scan.cpu.name}, ${scan.cpu.cores} cœurs, ${scan.cpu.threads} threads`],
    ["Mémoire", `${formatBytes(scan.memory.totalBytes)}, ${plural(scan.memory.modules, "barrette", "barrettes")}${scan.memory.speedMHz ? ` à ${scan.memory.speedMHz} MT/s` : ""}`],
    ...scan.gpus.map((g): [string, string] => ["Graphique", `${g.name}, pilote ${g.driverVersion}${g.driverDate ? ` du ${formatDate(g.driverDate)}` : ""}`]),
    ...scan.disks.map((d): [string, string] => ["Disque", `${d.name}, ${formatBytes(d.sizeBytes)}, ${d.bus} ${d.media}${d.temperature ? `, ${d.temperature} °C` : ""}`]),
    ["Allumé depuis", formatDuration(scan.uptimeSeconds)],
  ];
  return (
    <dl className="facts">
      {facts.map(([k, v], i) => (
        <div key={i}>
          <dt>{k}</dt>
          <dd>{v}</dd>
        </div>
      ))}
    </dl>
  );
}

export default function Bilan() {
  const { toast } = useApp();
  const { data, loading, error, reload } = useResource<Scan>("scan", () => call<Scan>("diag", "scan"));
  const items = useMemo(() => (data ? findings(data) : []), [data]);
  const issues = items.filter((f) => f.level !== "ok");
  const fine = items.filter((f) => f.level === "ok");
  const copy = async () => {
    if (!data) return;
    try {
      await navigator.clipboard.writeText(reportText(data, items));
      toast("Bilan copié");
    } catch (e) {
      toast(errorText(e), "alert");
    }
  };
  return (
    <>
      <header className="verdict">
        <h1>{data ? headline(items) : "Analyse de ce PC en cours."}</h1>
        <div className="head-actions">
          {data && !loading && <button onClick={() => void copy()}>Copier le bilan</button>}
          <button disabled={loading} onClick={() => void reload()}>
            {loading ? "Analyse…" : "Relancer l'analyse"}
          </button>
        </div>
      </header>
      {error && <Failure text={error} retry={() => void reload()} />}
      {!data && loading && <Skeleton rows={5} />}
      {issues.length > 0 && (
        <ul className="panel reveal">
          {issues.map((f) => (
            <Row item={f} key={f.id} />
          ))}
        </ul>
      )}
      {data && !data.boots.available && (
        <p className="note">Le temps de démarrage et l'état détaillé des disques ne sont lisibles qu'en mode administrateur.</p>
      )}
      {fine.length > 0 && (
        <Section title="Ce qui va bien">
          <ul className="panel">
            {fine.map((f) => (
              <Row item={f} key={f.id} />
            ))}
          </ul>
        </Section>
      )}
      {data && (
        <Section title="Ce PC">
          <Machine scan={data} />
        </Section>
      )}
    </>
  );
}
