import { useState } from "react";
import { formatDate, plural } from "../lib";
import { AdminButton, call, Elapsed, Empty, errorText, Failure, Head, Measures, Section, useAction, useApp, useResource } from "../ui";
import { WuList } from "./Pilotes";

type Upgrade = { name: string; id: string; version: string; available: string };

function Software() {
  const { toast } = useApp();
  const list = useResource<{ items: Upgrade[] }>("winget", () => call("updates", "winget-list"));
  const { busy, result, run } = useAction();
  const [queue, setQueue] = useState<string | null>(null);
  const items = list.data?.items ?? [];
  const upgrade = async (item: Upgrade) => {
    setQueue(item.name);
    await run("updates", "winget-upgrade", { id: item.id }, { key: item.id });
    setQueue(null);
    void list.reload();
  };
  const upgradeAll = async () => {
    let done = 0;
    for (const item of items) {
      setQueue(item.name);
      try {
        await call("updates", "winget-upgrade", { id: item.id });
        done++;
      } catch (e) {
        toast(errorText(e), "alert");
      }
    }
    setQueue(null);
    toast(`${plural(done, "logiciel mis à jour", "logiciels mis à jour")} sur ${items.length}`);
    void list.reload();
  };
  return (
    <Section
      title="Logiciels"
      note="Via WinGet, le gestionnaire de Microsoft. Les licences des logiciels mis à jour sont acceptées."
      actions={
        <>
          <button disabled={list.loading || !!queue} onClick={() => void list.reload()}>{list.loading ? "Lecture…" : "Actualiser"}</button>
          {items.length > 1 && <button disabled={!!queue} onClick={() => void upgradeAll()}>Tout mettre à jour ({items.length})</button>}
        </>
      }
    >
      {list.error && <Failure text={list.error} retry={() => void list.reload()} />}
      {queue && <Elapsed label={`Mise à jour de ${queue}.`} />}
      {list.data && !items.length && <Empty>Vos logiciels connus de WinGet sont à jour.</Empty>}
      {items.length > 0 && (
        <ul className="panel">
          {items.map((i) => (
            <li className="row" key={i.id}>
              <div className="grow">
                <strong>{i.name}</strong>
                <p className="tabular">{i.version} › {i.available}</p>
              </div>
              <button disabled={!!queue || !!busy} onClick={() => void upgrade(i)}>Mettre à jour</button>
            </li>
          ))}
        </ul>
      )}
      <Measures items={result?.measures} />
    </Section>
  );
}

function History() {
  const history = useResource<{ items: { title: string; date: string; ok: boolean }[] }>("wu-history", () => call("updates", "wu-history"));
  return (
    <Section title="Dernières installations de Windows">
      {history.error && <Failure text={history.error} retry={() => void history.reload()} />}
      {history.data && (
        <ul className="panel">
          {history.data.items.map((h, i) => (
            <li className="row" key={i}>
              <div className="grow">
                <strong>{h.title}</strong>
                <p>{formatDate(h.date)}{h.ok ? "" : ", échec"}</p>
              </div>
            </li>
          ))}
        </ul>
      )}
    </Section>
  );
}

export default function MiseAJour() {
  const { ask } = useApp();
  const { busy, run } = useAction();
  const reboot = async () => {
    const yes = await ask({
      title: "Redémarrer le PC",
      body: "Windows redémarre dans 60 secondes. Enregistrez votre travail. Vous pourrez annuler pendant ce délai.",
      confirm: "Redémarrer dans 60 s",
      danger: true,
    });
    if (yes) await run("updates", "reboot");
  };
  return (
    <>
      <Head title="Mises à jour" intro="Correctifs de Windows et de vos logiciels, avec un point de restauration avant toute installation Windows.">
        <AdminButton needsAdmin={false} disabled={!!busy} onRun={() => void reboot()}>Redémarrer le PC</AdminButton>
      </Head>
      <Section title="Windows">
        <WuList kind="Software" cacheKey="wu-software" />
      </Section>
      <Software />
      <History />
    </>
  );
}
