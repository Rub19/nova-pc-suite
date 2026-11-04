import { arr, formatBytes, formatDate } from "../lib";
import { AdminButton, call, Elapsed, Failure, Head, Measures, Section, Skeleton, useAction, useApp, useResource } from "../ui";

type Overview = {
  plans: { guid: string; name: string; active: boolean }[];
  volumes: { letter: string; label: string; sizeBytes: number; freeBytes: number; media: string | null; lastOptimized: string | null; system: boolean }[];
  hibernation: { enabled: boolean; fileBytes: number };
  gpu: { name: string; temperature: number; usePercent: number; memoryUsedMiB: number; memoryTotalMiB: number; powerW: number | null; driver: string } | null;
};
type Proc = { name: string; count: number; cpuPercent: number; ramBytes: number; closable: boolean };
type Net = { probes: { role: string; target: string; lossPercent: number; averageMs: number | null; worstMs: number | null }[]; dnsOk: boolean; dnsMs: number };

function Processes() {
  const list = useResource<{ items: Proc[] }>("processes", () => call("perf", "processes"));
  const { busy, result, run } = useAction();
  const close = async (p: Proc) => {
    await run("perf", "process-close", { name: p.name }, { key: p.name });
    void list.reload();
  };
  return (
    <Section
      title="Ce qui occupe le PC en ce moment"
      note="Fermer revient à cliquer sur la croix du programme : il peut demander d'enregistrer. Nova n'arrête jamais un programme de force."
      actions={<button disabled={list.loading} onClick={() => void list.reload()}>{list.loading ? "Mesure…" : "Mesurer à nouveau"}</button>}
    >
      {list.error && <Failure text={list.error} retry={() => void list.reload()} />}
      {!list.data && list.loading && <Skeleton rows={6} />}
      {list.data && (
        <div className="table-wrap">
          <table>
            <thead>
              <tr><th>Programme</th><th className="num">Processeur</th><th className="num">Mémoire</th><th aria-label="Action" /></tr>
            </thead>
            <tbody>
              {arr(list.data.items).map((p) => (
                <tr key={p.name}>
                  <td>{p.name}{p.count > 1 && <span className="muted"> ({p.count} processus)</span>}</td>
                  <td className="num tabular">{p.cpuPercent.toLocaleString("fr-FR")} %</td>
                  <td className="num tabular">{formatBytes(p.ramBytes)}</td>
                  <td className="num">{p.closable && <button disabled={!!busy} onClick={() => void close(p)}>{busy === p.name ? "Fermeture…" : "Fermer"}</button>}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      <Measures items={result?.measures} />
    </Section>
  );
}

function Network() {
  const test = useResource<Net>("network", () => call("perf", "network-test"), false);
  const { busy, result, run } = useAction();
  return (
    <Section
      title="Connexion"
      note="Cinq envois vers votre box et vers Internet. Ne mesure pas le débit."
      actions={
        <>
          <AdminButton disabled={!!busy} onRun={() => void run("perf", "dns-flush")}>Vider le cache DNS</AdminButton>
          <button className={test.data ? "" : "primary"} disabled={test.loading} onClick={() => void test.reload()}>{test.loading ? "Test…" : "Tester"}</button>
        </>
      }
    >
      {test.error && <Failure text={test.error} />}
      {test.data && (
        <ul className="panel">
          {arr(test.data.probes).map((p) => (
            <li className="row" key={p.target}>
              <div className="grow"><strong>{p.role}</strong></div>
              <span className="tabular">{p.averageMs === null ? "aucune réponse" : `${p.averageMs.toLocaleString("fr-FR")} ms, ${p.lossPercent} % de perte`}</span>
            </li>
          ))}
          <li className="row">
            <div className="grow"><strong>Résolution des noms (DNS)</strong></div>
            <span className="tabular">{test.data.dnsOk ? `${test.data.dnsMs} ms` : "échec"}</span>
          </li>
        </ul>
      )}
      <Measures items={result?.measures} />
    </Section>
  );
}

export default function Performances() {
  const { ask } = useApp();
  const { data, loading, error, reload } = useResource<Overview>("perf", () => call("perf", "overview"));
  const { busy, result, run } = useAction();
  const setPlan = async (guid: string) => {
    if (await run("perf", "power-apply", { guid })) void reload();
  };
  const optimize = async (letter: string) => {
    if (await run("perf", "volume-optimize", { letter }, { key: letter })) void reload();
  };
  const hibernate = async (enabled: boolean) => {
    const yes =
      enabled ||
      (await ask({
        title: "Désactiver la veille prolongée",
        body: `Libère ${formatBytes(data?.hibernation.fileBytes ?? 0)} sur le disque. Le démarrage rapide de Windows sera aussi désactivé : le PC peut démarrer un peu plus lentement. Vous pourrez la réactiver ici.`,
        confirm: "Désactiver",
      }));
    if (yes && (await run("perf", "hibernate-set", { enabled }))) void reload();
  };
  return (
    <>
      <Head title="Performances" intro="Les réglages qui changent vraiment quelque chose, et ce qui consomme vos ressources." />
      {error && <Failure text={error} retry={() => void reload()} />}
      {!data && loading && <Skeleton rows={5} />}
      {busy && <Elapsed label="Opération en cours." />}
      <Measures items={result?.measures} />
      {data && (
        <>
          <Section title="Mode d'alimentation" note="Nova mesure le calcul du processeur avant et après. Sur un PC de bureau récent, l'écart est souvent faible.">
            <div className="segments" role="radiogroup" aria-label="Mode d'alimentation">
              {arr(data.plans).map((p) => (
                <button key={p.guid} role="radio" aria-checked={p.active} className={p.active ? "on" : ""} disabled={!!busy} onClick={() => !p.active && void setPlan(p.guid)}>
                  {p.name}
                </button>
              ))}
            </div>
          </Section>
          {data.gpu && (
            <Section title={data.gpu.name} note={`Pilote ${data.gpu.driver}. Relevé au moment de l'ouverture de cette page.`}>
              <dl className="stats">
                <div><dt>Température</dt><dd className="tabular">{Math.round(data.gpu.temperature)} °C</dd></div>
                <div><dt>Utilisation</dt><dd className="tabular">{Math.round(data.gpu.usePercent)} %</dd></div>
                <div><dt>Mémoire vidéo</dt><dd className="tabular">{formatBytes(data.gpu.memoryUsedMiB * 1048576)} sur {formatBytes(data.gpu.memoryTotalMiB * 1048576)}</dd></div>
                {data.gpu.powerW !== null && <div><dt>Puissance</dt><dd className="tabular">{Math.round(data.gpu.powerW)} W</dd></div>}
              </dl>
            </Section>
          )}
          <Section title="Disques" note="Windows choisit lui-même : TRIM pour un SSD, défragmentation pour un disque dur.">
            <ul className="panel">
              {arr(data.volumes).map((v) => (
                <li className="row" key={v.letter}>
                  <div className="grow">
                    <strong>{v.letter}:{v.label ? ` ${v.label}` : ""}</strong>
                    <p>{[v.media ?? "Support inconnu", `${formatBytes(v.freeBytes)} libres sur ${formatBytes(v.sizeBytes)}`, `dernière optimisation : ${formatDate(v.lastOptimized)}`].join(", ")}</p>
                  </div>
                  <AdminButton disabled={!!busy} onRun={() => void optimize(v.letter)}>Optimiser</AdminButton>
                </li>
              ))}
            </ul>
          </Section>
          <Section title="Veille prolongée">
            <ul className="panel">
              <li className="row">
                <div className="grow">
                  <strong>{data.hibernation.enabled ? "Activée" : "Désactivée"}</strong>
                  <p>{data.hibernation.enabled ? `Le fichier hiberfil.sys occupe ${formatBytes(data.hibernation.fileBytes)}.` : "Le démarrage rapide de Windows est aussi désactivé."}</p>
                </div>
                <AdminButton disabled={!!busy} onRun={() => void hibernate(!data.hibernation.enabled)}>{data.hibernation.enabled ? "Désactiver" : "Réactiver"}</AdminButton>
              </li>
            </ul>
          </Section>
        </>
      )}
      <Processes />
      <Network />
    </>
  );
}
