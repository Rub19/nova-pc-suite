import { invoke } from "@tauri-apps/api/core";
import { ShieldCheck, Undo2, X } from "lucide-react";
import {
  Component,
  createContext,
  ReactNode,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react";
import { ActionResult, formatMeasureValue, Measure, measureDelta, Page, Undo } from "./lib";

type Raw = { ok: boolean; data?: unknown; error?: string };

export async function call<T = ActionResult>(
  module: string,
  action: string,
  payload?: Record<string, unknown>,
): Promise<T> {
  const raw = await invoke<Raw>("run", { module, action, payload: payload ?? {} });
  if (!raw.ok) throw new Error(raw.error || "Windows a refusé cette opération.");
  return raw.data as T;
}

export function errorText(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

const cache = new Map<string, unknown>();
const clearResources = () => cache.clear();

export function useResource<T>(key: string, loader: () => Promise<T>, auto = true) {
  const [data, setData] = useState<T | null>((cache.get(key) as T) ?? null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const loaderRef = useRef(loader);
  loaderRef.current = loader;
  const reload = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const value = await loaderRef.current();
      cache.set(key, value);
      setData(value);
      return value;
    } catch (e) {
      setError(errorText(e));
      return null;
    } finally {
      setLoading(false);
    }
  }, [key]);
  useEffect(() => {
    if (auto && !cache.has(key)) void reload();
  }, [auto, key, reload]);
  return { data, loading, error, reload, setData };
}

type Toast = { id: number; text: string; tone: "ok" | "alert"; undo?: Undo | null; leaving?: boolean };
type ConfirmOptions = {
  title: string;
  body: ReactNode;
  confirm: string;
  danger?: boolean;
};
type Ctx = {
  isAdmin: boolean;
  page: Page;
  go: (page: Page) => void;
  toast: (text: string, tone?: "ok" | "alert", undo?: Undo | null) => void;
  ask: (options: ConfirmOptions) => Promise<boolean>;
  elevate: () => Promise<void>;
};
const AppContext = createContext<Ctx>(null as unknown as Ctx);
export const useApp = () => useContext(AppContext);

export function AppProvider({
  isAdmin,
  page,
  go,
  children,
}: {
  isAdmin: boolean;
  page: Page;
  go: (page: Page) => void;
  children: ReactNode;
}) {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [dialog, setDialog] = useState<(ConfirmOptions & { resolve: (v: boolean) => void }) | null>(null);
  const dialogRef = useRef<HTMLDialogElement>(null);
  const nextId = useRef(1);

  const dismiss = (id: number) => {
    setToasts((all) => all.map((t) => (t.id === id ? { ...t, leaving: true } : t)));
    window.setTimeout(() => setToasts((all) => all.filter((t) => t.id !== id)), 170);
  };
  const toast = useCallback<Ctx["toast"]>((text, tone = "ok", undo) => {
    const id = nextId.current++;
    setToasts((all) => [...all.slice(-2), { id, text, tone, undo }]);
    if (tone !== "alert") window.setTimeout(() => dismiss(id), undo ? 12000 : 5000);
  }, []);
  const ask = useCallback<Ctx["ask"]>(
    (options) => new Promise((resolve) => setDialog({ ...options, resolve })),
    [],
  );
  useEffect(() => {
    if (dialog) dialogRef.current?.showModal();
  }, [dialog]);
  const close = (value: boolean) => {
    dialog?.resolve(value);
    const node = dialogRef.current;
    const finish = () => {
      node?.close();
      setDialog(null);
    };
    if (!node || document.documentElement.hasAttribute("data-keynav")) return finish();
    node.classList.add("closing");
    window.setTimeout(finish, 120);
  };
  const elevate = useCallback(async () => {
    const yes = await ask({
      title: "Relancer Nova en administrateur",
      body: "Cette action touche à Windows. Nova va se fermer et se rouvrir sur cette page. Windows vous demandera votre accord.",
      confirm: "Relancer",
    });
    if (!yes) return;
    try {
      const result = await invoke<Raw>("elevate", { page });
      if (!result.ok) toast(result.error ?? "Passage administrateur annulé.", "alert");
    } catch (e) {
      toast(errorText(e), "alert");
    }
  }, [ask, page, toast]);
  const applyUndo = async (t: Toast) => {
    if (!t.undo) return;
    dismiss(t.id);
    try {
      const r = await call(t.undo.module, t.undo.action, t.undo.payload);
      toast(r.summary ?? "Annulé");
      window.dispatchEvent(new CustomEvent("nova:changed", { detail: t.undo.module }));
    } catch (e) {
      toast(errorText(e), "alert");
    }
  };

  return (
    <AppContext.Provider value={{ isAdmin, page, go, toast, ask, elevate }}>
      {children}
      <div className="toasts" role="status" aria-live="polite">
        {toasts.map((t) => (
          <div className={`toast ${t.tone}${t.leaving ? " leaving" : ""}`} key={t.id}>
            <span>{t.text}</span>
            {t.undo && (
              <button className="link" onClick={() => void applyUndo(t)}>
                <Undo2 size={15} aria-hidden /> {t.undo.label}
              </button>
            )}
            <button className="icon" aria-label="Fermer ce message" onClick={() => dismiss(t.id)}>
              <X size={15} aria-hidden />
            </button>
          </div>
        ))}
      </div>
      {dialog && (
        <dialog ref={dialogRef} className="sheet" onCancel={() => close(false)}>
          <h2>{dialog.title}</h2>
          <div className="sheet-body">{dialog.body}</div>
          <div className="sheet-actions">
            <button onClick={() => close(false)}>Annuler</button>
            <button className={dialog.danger ? "danger" : "primary"} autoFocus onClick={() => close(true)}>
              {dialog.confirm}
            </button>
          </div>
        </dialog>
      )}
    </AppContext.Provider>
  );
}

export function useAction() {
  const { toast } = useApp();
  const [busy, setBusy] = useState<string | null>(null);
  const [result, setResult] = useState<ActionResult | null>(null);
  const run = useCallback(
    async (
      module: string,
      action: string,
      payload?: Record<string, unknown>,
      options?: { key?: string; silent?: boolean },
    ) => {
      setBusy(options?.key ?? action);
      setResult(null);
      try {
        const r = await call(module, action, payload);
        setResult(r);
        if (!options?.silent && r.summary) toast(r.summary, "ok", r.undo);
        return r;
      } catch (e) {
        toast(errorText(e), "alert");
        return null;
      } finally {
        setBusy(null);
      }
    },
    [toast],
  );
  return { busy, result, run, setResult };
}

export function Head({ title, intro, children }: { title: string; intro?: string; children?: ReactNode }) {
  return (
    <header className="head">
      <div>
        <h1>{title}</h1>
        {intro && <p>{intro}</p>}
      </div>
      {children && <div className="head-actions">{children}</div>}
    </header>
  );
}

export function Section({
  title,
  note,
  actions,
  children,
}: {
  title: string;
  note?: string;
  actions?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="section">
      <div className="section-head">
        <div>
          <h2>{title}</h2>
          {note && <p>{note}</p>}
        </div>
        {actions && <div className="section-actions">{actions}</div>}
      </div>
      {children}
    </section>
  );
}

export class ScreenGuard extends Component<{ children: ReactNode }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() {
    return { failed: true };
  }
  retry = () => {
    clearResources();
    this.setState({ failed: false });
  };
  render() {
    if (!this.state.failed) return this.props.children;
    return <Failure text="Cet écran a reçu des données illisibles. Le reste de Nova fonctionne : changez d'écran ou réessayez." retry={this.retry} />;
  }
}

export function Elapsed({ label }: { label: string }) {
  const [seconds, setSeconds] = useState(0);
  useEffect(() => {
    const timer = window.setInterval(() => setSeconds((s) => s + 1), 1000);
    return () => window.clearInterval(timer);
  }, []);
  const text = seconds >= 60 ? `${Math.floor(seconds / 60)} min ${seconds % 60} s` : `${seconds} s`;
  return (
    <div className="working" role="status">
      <span className="bar" aria-hidden />
      <span>{label}</span>
      <span className="muted tabular">{text}</span>
    </div>
  );
}

export function Failure({ text, retry }: { text: string; retry?: () => void }) {
  return (
    <div className="failure" role="alert">
      <span>{text}</span>
      {retry && <button onClick={retry}>Réessayer</button>}
    </div>
  );
}

export function Skeleton({ rows = 4 }: { rows?: number }) {
  return (
    <div className="panel" aria-hidden>
      {Array.from({ length: rows }).map((_, i) => (
        <div className="row skeleton" key={i}>
          <span />
        </div>
      ))}
    </div>
  );
}

export function Empty({ children }: { children: ReactNode }) {
  return <div className="empty">{children}</div>;
}

export function Measures({ items }: { items?: Measure[] }) {
  if (!items?.length) return null;
  return (
    <dl className="measures">
      {items.map((m) => {
        const delta = measureDelta(m);
        return (
          <div key={m.label}>
            <dt>{m.label}</dt>
            <dd>
              <span className="muted">{formatMeasureValue(m.before, m.unit)}</span>
              <span aria-label="devient" className="arrow">›</span>
              <strong>{formatMeasureValue(m.after, m.unit)}</strong>
              {delta && <em>{delta}</em>}
            </dd>
          </div>
        );
      })}
    </dl>
  );
}

export function AdminButton({
  needsAdmin = true,
  onRun,
  disabled,
  className,
  children,
}: {
  needsAdmin?: boolean;
  onRun: () => void;
  disabled?: boolean;
  className?: string;
  children: ReactNode;
}) {
  const { isAdmin, elevate } = useApp();
  const locked = needsAdmin && !isAdmin;
  return (
    <button
      className={className}
      disabled={disabled}
      onClick={locked ? () => void elevate() : onRun}
      title={locked ? "Demande les droits administrateur" : undefined}
    >
      {locked && <ShieldCheck size={15} aria-label="Droits administrateur requis" />}
      {children}
    </button>
  );
}
