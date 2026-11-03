import { ExternalLink } from "lucide-react";
import { useMemo, useState } from "react";
import { DEVICE_CODES, formatBytes, formatDate, plural } from "../lib";
import { AdminButton, call, Elapsed, Empty, errorText, Failure, Head, Measures, Section, Skeleton, useAction, useApp, useResource } from "../ui";

type Devices = {
  drivers: { name: string; manufacturer: string; version: string; deviceClass: string; signed: boolean; date: string | null }[];
  problems: { name: string; manufacturer: string; deviceClass: string; code: number }[];
  vendors: { id: string; name: string }[];
};
export type WuItem = {
  id: string;
  title: string;
  description: string;
  kb: string[];
  sizeBytes: number;
  severity: string;
  eula: string | null;
  eulaAccepted: boolean;
  rebootLikely: boolean;
  driverClass: string;
  driverProvider: string;
  driverDate: string | null;
};
export type WuResults = {
  results?: { id: string; title: string; ok: boolean; hresult: string; rebootRequired: boolean }[];
  changedDrivers?: { name: string; before: string; after: string }[];
  rebootRequired?: boolean;
  restorePoint?: { created: boolean; description: string; date: string };
};

export function WuOutcome({ data }: { data: WuResults }) {
  return (
    <div className="ticket">
      {data.restorePoint && (
        <p className="muted">
          {data.restorePoint.created ? "Point de restauration créé" : "Point de restauration récent conservé"} : {data.restorePoint.description}.
        </p>
      )}
      <ul className="plain">
        {data.results?.map((r) => (
          <li key={r.id} className={r.ok ? "good" : "bad"}>
            {r.ok ? "Installé" : `Échec ${r.hresult}`} : {r.title}
          </li>
        ))}
      </ul>
      {data.changedDrivers?.map((d) => (
        <p key={d.name}>
          {d.name} : {d.before} › <strong>{d.after}</strong>
        </p>
      ))}
      {data.rebootRequired && <p><strong>Redémarrez le PC pour terminer.</strong></p>}
    </div>
  );
}

export function WuList({ kind, cacheKey }: { kind: "Driver" | "Software"; cacheKey: string }) {
  const { ask } = useApp();
  const search = useResource<{ items: WuItem[]; searchedAt: string }>(cacheKey, () => call("updates", "wu-search", { kind }), false);
  const [selected, setSelected] = useState<string[]>([]);
  const { busy, result, run } = useAction();
  const items = search.data?.items ?? [];
  const chosen = items.filter((i) => selected.includes(i.id));

  const install = async () => {
    const eulas = chosen.filter((i) => !i.eulaAccepted && i.eula);
    const yes = await ask({
      title: `Installer ${plural(chosen.length, "mise à jour", "mises à jour")}`,
      confirm: eulas.length ? "Accepter et installer" : "Installer",
      body: (
        <>
          <p>Nova crée d'abord un point de restauration, pour pouvoir revenir en arrière. Le PC peut ensuite demander un redémarrage.</p>
          {eulas.map((i) => (
            <details key={i.id}>
              <summary>Licence : {i.title}</summary>
              <pre className="eula">{i.eula}</pre>
            </details>
          ))}
        </>
      ),
    });
    if (!yes) return;
    const r = await run("updates", "wu-install", { kind, ids: selected, acceptEulas: eulas.length > 0 });
    if (r) {
      setSelected([]);
      void search.reload();
    }
  };

  return (
    <>
      <div className="toolbar">
        <button className={search.data ? "" : "primary"} disabled={search.loading || !!busy} onClick={() => void search.reload()}>
          {search.loading ? "Recherche…" : search.data ? "Chercher à nouveau" : "Chercher sur Windows Update"}
        </button>
        {items.length > 0 && (
          <AdminButton className="primary" disabled={!chosen.length || !!busy} onRun={() => void install()}>
            Installer la sélection ({chosen.length})
          </AdminButton>
        )}
        {items.length > 1 && (
          <button disabled={!!busy} onClick={() => setSelected(selected.length === items.length ? [] : items.map((i) => i.id))}>
            {selected.length === items.length ? "Tout décocher" : "Tout cocher"}
          </button>
        )}
      </div>
      {search.loading && <Elapsed label="Windows Update cherche. Cela peut prendre quelques minutes." />}
      {busy && <Elapsed label="Point de restauration, téléchargement puis installation. Laissez le PC allumé." />}
      {search.error && <Failure text={search.error} retry={() => void search.reload()} />}
      {search.data && !search.loading && !items.length && (
        <Empty>Windows Update ne propose rien pour le moment.</Empty>
      )}
      {items.length > 0 && (
        <ul className="panel">
          {items.map((i) => (
            <li className="row" key={i.id}>
              <label className="check grow">
                <input
                  type="checkbox"
                  checked={selected.includes(i.id)}
                  disabled={!!busy}
                  onChange={(e) => setSelected((s) => (e.target.checked ? [...s, i.id] : s.filter((x) => x !== i.id)))}
                />
                <span>
                  <strong>{i.title}</strong>
                  <small>
                    {[i.driverProvider, i.driverDate && `pilote du ${formatDate(i.driverDate)}`, i.kb.join(", "), i.severity, i.sizeBytes ? formatBytes(i.sizeBytes) : "", i.rebootLikely ? "redémarrage probable" : ""].filter(Boolean).join(", ")}
                  </small>
                </span>
              </label>
            </li>
          ))}
        </ul>
      )}
      {result && (
        <>
          <Measures items={result.measures} />
          <WuOutcome data={result as WuResults} />
        </>
      )}
    </>
  );
}

function Inventory({ drivers }: { drivers: Devices["drivers"] }) {
  const [query, setQuery] = useState("");
  const [all, setAll] = useState(false);
  const rows = useMemo(() => {
    const seen = new Set<string>();
    const q = query.trim().toLowerCase();
    return drivers
      .filter((d) => all || !/^(microsoft|\(standard|standard|\(générique|generic)/i.test(d.manufacturer ?? ""))
      .filter((d) => !q || `${d.name} ${d.manufacturer} ${d.version}`.toLowerCase().includes(q))
      .filter((d) => {
        const key = `${d.name}|${d.version}`;
        if (seen.has(key)) return false;
        seen.add(key);
        return true;
      })
      .sort((a, b) => (a.date ?? "9").localeCompare(b.date ?? "9"));
  }, [drivers, query, all]);
  return (
    <>
      <div className="toolbar">
        <input type="search" aria-label="Filtrer les pilotes" placeholder="Filtrer par nom, fabricant, version" value={query} onChange={(e) => setQuery(e.target.value)} />
        <label className="check">
          <input type="checkbox" checked={all} onChange={(e) => setAll(e.target.checked)} />
          <span>Inclure les pilotes de Microsoft</span>
        </label>
      </div>
      <div className="table-wrap">
        <table>
          <thead>
            <tr><th>Appareil</th><th>Fabricant</th><th>Version</th><th>Date</th></tr>
          </thead>
          <tbody>
            {rows.map((d, i) => (
              <tr key={i}>
                <td>{d.name}{!d.signed && <span className="tag warn">non signé</span>}</td>
                <td>{d.manufacturer}</td>
                <td className="tabular">{d.version}</td>
                <td className="tabular">{formatDate(d.date)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </>
  );
}

export default function Pilotes() {
  const { toast } = useApp();
  const { data, loading, error, reload } = useResource<Devices>("devices", () => call<Devices>("updates", "devices"));
  const openVendor = async (id: string) => {
    try {
      await call("updates", "open-vendor", { id });
    } catch (e) {
      toast(errorText(e), "alert");
    }
  };
  return (
    <>
      <Head title="Pilotes" intro="Les pilotes font le lien entre Windows et le matériel. Installez seulement ce que Windows Update ou le fabricant proposent." />
      {error && <Failure text={error} retry={() => void reload()} />}
      {!data && loading && <Skeleton />}
      {data && (
        <Section title="Appareils en erreur" actions={<button disabled={loading} onClick={() => void reload()}>{loading ? "Lecture…" : "Actualiser"}</button>}>
          {data.problems.length === 0 ? (
            <Empty>Tous les appareils fonctionnent.</Empty>
          ) : (
            <ul className="panel">
              {data.problems.map((p, i) => (
                <li className="row" key={i}>
                  <div className="grow">
                    <strong>{p.name || "Appareil sans nom"}</strong>
                    <p>{DEVICE_CODES[p.code] ?? `Windows signale l'erreur ${p.code}.`}</p>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </Section>
      )}
      <Section title="Mises à jour proposées par Windows" note="Sources officielles Microsoft. Elles peuvent être plus anciennes que celles du fabricant.">
        <WuList kind="Driver" cacheKey="wu-driver" />
      </Section>
      {data && data.vendors.length > 0 && (
        <Section title="Sites des fabricants de ce PC" note="Pour la version la plus récente, surtout pour la carte graphique. S'ouvre dans votre navigateur.">
          <div className="toolbar">
            {data.vendors.map((v) => (
              <button key={v.id} onClick={() => void openVendor(v.id)}>
                {v.name} <ExternalLink size={14} aria-hidden />
              </button>
            ))}
          </div>
        </Section>
      )}
      {data && (
        <Section title="Pilotes installés" note="Du plus ancien au plus récent.">
          <Inventory drivers={data.drivers} />
        </Section>
      )}
    </>
  );
}
