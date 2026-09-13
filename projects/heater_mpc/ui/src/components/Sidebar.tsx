import type { ReactNode } from "react";

export type Page = "simulation" | "notes";

export interface NotesSection {
  id: string;
  title: string;
}

interface Props {
  page: Page;
  sections: NotesSection[];
  activeSection: string | null;
  status: ReactNode;
  onPage: (page: Page) => void;
  onSection: (id: string) => void;
}

export function Sidebar({ page, sections, activeSection, status, onPage, onSection }: Props) {
  return (
    <aside className="sidebar">
      <div className="sidebar-brand">
        <h1>Heater MPC</h1>
        <p className="muted">基板ヒーターの PID と MPC のシミュレータ</p>
        {status}
      </div>
      <nav>
        <button type="button" className={`nav-item ${page === "simulation" ? "active" : ""}`} onClick={() => onPage("simulation")}>
          シミュレーション
          <span className="nav-sub">設定、実行、結果の表示</span>
        </button>
        <button type="button" className={`nav-item ${page === "notes" ? "active" : ""}`} onClick={() => onPage("notes")}>
          ノート
          <span className="nav-sub">モデルと制御の解説、デモ</span>
        </button>
        <ul className={`nav-sections ${page === "notes" ? "" : "collapsed"}`}>
          {sections.map((s) => (
            <li key={s.id}>
              <button type="button" className={`nav-section ${activeSection === s.id ? "active" : ""}`} onClick={() => onSection(s.id)}>
                {s.title}
              </button>
            </li>
          ))}
        </ul>
      </nav>
    </aside>
  );
}
