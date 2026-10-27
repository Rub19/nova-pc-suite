import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
type Result = {
  deletedCount?: number;
  preservedCount?: number;
  freedBytes?: number;
};
export default function AutomaticCleanup({ visible }: { visible: boolean }) {
  const [enabled, setEnabled] = useState(
    () => localStorage.getItem("nova-auto-temp-cleanup") === "enabled",
  );
  const [busy, setBusy] = useState(false),
    [result, setResult] = useState<Result>(),
    [last, setLast] = useState<Date>(),
    [error, setError] = useState("");
  const active = useRef(false),
    guard = useRef(false);
  async function clean() {
    if (!active.current || guard.current) return;
    guard.current = true;
    setBusy(true);
    setError("");
    try {
      const reply = await invoke<{
        ok: boolean;
        data?: Result;
        error?: string;
      }>("extended_action", {
        module: "cleanup",
        action: "auto-clean",
        payload: {},
        confirmed: true,
      });
      if (reply.data) setResult(reply.data);
      setLast(new Date());
      if (!reply.ok)
        setError(reply.error ?? "Certains temporaires ont été conservés.");
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
    } finally {
      guard.current = false;
      setBusy(false);
    }
  }
  useEffect(() => {
    localStorage.setItem(
      "nova-auto-temp-cleanup",
      enabled ? "enabled" : "disabled",
    );
    active.current = enabled;
    if (!enabled) return;
    void clean();
    const timer = setInterval(() => void clean(), 3600000);
    return () => {
      active.current = false;
      clearInterval(timer);
    };
  }, [enabled]);
  useEffect(() => {
    let disposed = false;
    const unlisten: Array<() => void> = [];
    void listen("nova:temp-cleanup-toggle", () => {
      if (!disposed) setEnabled((current) => !current);
    }).then((stop) => {
      if (disposed) stop();
      else unlisten.push(stop);
    });
    void listen("nova:temp-clean-now", () => {
      if (active.current) void clean();
    }).then((stop) => {
      if (disposed) stop();
      else unlisten.push(stop);
    });
    return () => {
      disposed = true;
      unlisten.forEach((stop) => stop());
    };
  }, []);
  return (
    <section
      hidden={!visible}
      className="panel auto-cleanup"
      aria-label="Nettoyage automatique des temporaires"
    >
      <h2>Nettoyage automatique des temporaires</h2>
      <label className="wb-check-row auto-cleanup-toggle">
        <input
          type="checkbox"
          checked={enabled}
          onChange={(e) => setEnabled(e.target.checked)}
        />
        <span>
          <strong>Supprimer automatiquement les temporaires anciens</strong>
          <small>
            Suppression définitive des fichiers éligibles, sans nouvelle
            confirmation.
          </small>
        </span>
      </label>
      <p>
        Au lancement de Nova puis toutes les heures tant qu’il reste ouvert.
        Fichiers de plus de 7 jours uniquement, à la racine des temporaires
        utilisateur ; 1 000 propositions maximum par cycle. Les dossiers, liens
        et fichiers occupés ou modifiés sont conservés.
      </p>
      <p role="status">
        {enabled ? "Automatisation activée" : "Automatisation désactivée"}
        {busy ? " · Nettoyage en cours…" : ""}
      </p>
      {last && (
        <p>
          Dernier passage : {last.toLocaleString("fr-FR")} ·{" "}
          {result?.deletedCount ?? 0} supprimé(s), {result?.preservedCount ?? 0}{" "}
          conservé(s) · {(result?.freedBytes ?? 0).toLocaleString("fr-FR")}{" "}
          octets logiques supprimés.
        </p>
      )}
      {error && (
        <p role="alert" className="alert error">
          {error} Le prochain passage reste planifié tant que l’option est
          activée.
        </p>
      )}
      <p className="footnote">
        Le choix est conservé sur ce PC. Désactiver l’option empêche les
        prochains passages ; une suppression déjà envoyée se termine. Un arrêt
        imprévu peut laisser des fichiers récupérables dans un lot de
        quarantaine.
      </p>
    </section>
  );
}
