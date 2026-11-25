import { useState } from "react";

import { SettingsPage } from "./settings/SettingsPage";
import { WorkPage } from "./pages/WorkPage";
import { formatElapsed, useWorkbench } from "./state/useWorkbench";

type Page = "work" | "settings";

const PAGE_TABS: readonly { id: Page; text: string }[] = [
  { id: "work", text: "工作" },
  { id: "settings", text: "设置" },
];

/**
 * 两页壳层：工作页负责"看和用"，设置页负责"配"。
 *
 * 顶栏只留三件事：这场会是谁、开了多久、要不要暂停提示。其余入口都在页内，
 * 免得开会时视线来回扫。
 */
export default function App() {
  const wb = useWorkbench();
  const [page, setPage] = useState<Page>("work");
  const [hintOn, setHintOn] = useState(true);

  const meetingLabel =
    wb.meeting === null
      ? "未检测到会议"
      : wb.meeting.title === ""
        ? wb.meeting.app
        : `${wb.meeting.app} · ${wb.meeting.title}`;

  return (
    <div className={`shell labels-${wb.labelMode}`}>
      <header className="topbar">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true" />
          <div className="brand-copy">
            <strong>会议副驾</strong>
            <span className="brand-sub">Meeting Copilot</span>
          </div>
        </div>

        <div className="topbar-status">
          <span className="meeting-pill" data-live={wb.meeting !== null}>
            <span className="meeting-dot" aria-hidden="true" />
            {meetingLabel}
          </span>
          <span className="timer">
            {wb.meeting === null ? "--:--" : formatElapsed(wb.elapsedSeconds)}
          </span>
        </div>

        <div className="topbar-actions">
          <nav className="seg" aria-label="页面切换">
            {PAGE_TABS.map((tab) => (
              <button
                key={tab.id}
                className="seg-btn"
                type="button"
                aria-pressed={page === tab.id}
                onClick={() => {
                  setPage(tab.id);
                }}
              >
                {tab.text}
              </button>
            ))}
          </nav>
          <button
            className={hintOn ? "btn btn-ghost btn-sm" : "btn btn-ghost btn-sm btn-on"}
            type="button"
            aria-pressed={!hintOn}
            onClick={() => {
              setHintOn((prev) => !prev);
            }}
          >
            {hintOn ? "暂停提示" : "恢复提示"}
          </button>
        </div>
      </header>

      {page === "work" ? <WorkPage wb={wb} /> : <SettingsPage wb={wb} />}
    </div>
  );
}
