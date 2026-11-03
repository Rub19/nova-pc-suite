import { invoke } from "@tauri-apps/api/core";
import {
  Brush,
  ClipboardList,
  Cpu,
  Download,
  Gauge,
  LucideIcon,
  Power,
  Stethoscope,
  Wrench,
} from "lucide-react";
import { ComponentType, useEffect, useRef, useState } from "react";
import { formatBytes, Page } from "./lib";
import { AppProvider, useApp } from "./ui";
import Bilan from "./pages/Bilan";
import Pilotes from "./pages/Pilotes";
import MiseAJour from "./pages/MiseAJour";
import Nettoyage from "./pages/Nettoyage";
import Demarrage from "./pages/Demarrage";
import Performances from "./pages/Performances";
import Reparer from "./pages/Reparer";
import Journal from "./pages/Journal";

const NAV: { id: Page; label: string; icon: LucideIcon }[] = [
  { id: "bilan", label: "Bilan", icon: Stethoscope },
  { id: "pilotes", label: "Pilotes", icon: Cpu },
  { id: "miseajour", label: "Mises à jour", icon: Download },
  { id: "nettoyage", label: "Nettoyage", icon: Brush },
  { id: "demarrage", label: "Démarrage", icon: Power },
  { id: "performances", label: "Performances", icon: Gauge },
  { id: "reparer", label: "Réparer", icon: Wrench },
  { id: "journal", label: "Journal", icon: ClipboardList },
];

const SCREENS: Record<Page, ComponentType> = {
  bilan: Bilan,
  pilotes: Pilotes,
  miseajour: MiseAJour,
  nettoyage: Nettoyage,
  demarrage: Demarrage,
  performances: Performances,
  reparer: Reparer,
  journal: Journal,
};

type Sample = { cpu: number | null; disk: number | null; memoryUsedBytes: number; memoryTotalBytes: number };

function Trace({ label, value, points, max = 100 }: { label: string; value: string; points: number[]; max?: number }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const node = canvas.current;
    const context = node?.getContext("2d");
    if (!node || !context) return;
    const ratio = window.devicePixelRatio || 1;
    const { width, height } = node.getBoundingClientRect();
    node.width = width * ratio;
    node.height = height * ratio;
    context.scale(ratio, ratio);
    context.clearRect(0, 0, width, height);
    if (points.length < 2) return;
    const step = width / 59;
    const x = (i: number) => width - (points.length - 1 - i) * step;
    const y = (v: number) => height - 1 - (Math.min(v, max) / max) * (height - 3);
    context.beginPath();
    points.forEach((v, i) => (i ? context.lineTo(x(i), y(v)) : context.moveTo(x(i), y(v))));
    context.strokeStyle = getComputedStyle(node).color;
    context.lineWidth = 1.5;
    context.lineJoin = "round";
    context.stroke();
  }, [points, max]);
  return (
    <div className="trace">
      <div className="trace-text">
        <span>{label}</span>
        <strong>{value}</strong>
      </div>
      <canvas ref={canvas} aria-hidden />
    </div>
  );
}

function Vitals() {
  const [history, setHistory] = useState<{ cpu: number[]; ram: number[]; disk: number[] }>({ cpu: [], ram: [], disk: [] });
  const [last, setLast] = useState<Sample | null>(null);
  useEffect(() => {
    const tick = async () => {
      if (document.hidden) return;
      const s = await invoke<Sample>("vitals_sample").catch(() => null);
      if (!s) return;
      setLast(s);
      const push = (list: number[], v: number | null) => (v === null ? list : [...list.slice(-59), v]);
      setHistory((h) => ({
        cpu: push(h.cpu, s.cpu),
        disk: push(h.disk, s.disk),
        ram: push(h.ram, s.memoryTotalBytes ? (s.memoryUsedBytes / s.memoryTotalBytes) * 100 : null),
      }));
    };
    void tick();
    const timer = window.setInterval(() => void tick(), 1500);
    return () => window.clearInterval(timer);
  }, []);
  const pct = (v: number | null | undefined) => (v === null || v === undefined ? "n/d" : `${Math.round(v)} %`);
  return (
    <div className="vitals" aria-label="Activité de ce PC en direct">
      <Trace label="Processeur" value={pct(last?.cpu)} points={history.cpu} />
      <Trace label="Mémoire" value={last ? formatBytes(last.memoryUsedBytes) : "n/d"} points={history.ram} />
      <Trace label="Disque" value={pct(last?.disk)} points={history.disk} />
    </div>
  );
}

function AdminStatus() {
  const { isAdmin, elevate } = useApp();
  if (isAdmin) return <p className="mode">Mode administrateur</p>;
  return (
    <button className="mode-button" onClick={() => void elevate()}>
      Relancer en administrateur
    </button>
  );
}

function Shell() {
  const { page, go } = useApp();
  const Screen = SCREENS[page];
  const main = useRef<HTMLElement>(null);
  useEffect(() => {
    main.current?.scrollTo(0, 0);
  }, [page]);
  return (
    <div className="app">
      <nav className="rail" aria-label="Navigation principale">
        <div className="brand">
          <span className="brand-mark" aria-hidden />
          Nova
        </div>
        <ul>
          {NAV.map(({ id, label, icon: Icon }) => (
            <li key={id}>
              <button className={page === id ? "active" : ""} aria-current={page === id ? "page" : undefined} onClick={() => go(id)}>
                <Icon size={17} aria-hidden />
                {label}
              </button>
            </li>
          ))}
        </ul>
        <Vitals />
        <AdminStatus />
      </nav>
      <main ref={main} className="content">
        <div className="content-inner">
          <Screen />
        </div>
      </main>
    </div>
  );
}

export default function App() {
  const [page, setPage] = useState<Page>("bilan");
  const [isAdmin, setIsAdmin] = useState(false);
  useEffect(() => {
    void invoke<boolean>("is_admin").then(setIsAdmin).catch(() => undefined);
    void invoke<string>("startup_page")
      .then((p) => setPage(p as Page))
      .catch(() => undefined)
      .finally(() => void invoke("app_ready").catch(() => undefined));
  }, []);
  return (
    <AppProvider isAdmin={isAdmin} page={page} go={setPage}>
      <Shell />
    </AppProvider>
  );
}
