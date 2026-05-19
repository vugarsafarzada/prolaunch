import { useState, useRef, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ask } from "@tauri-apps/plugin-dialog";
import WelcomeScreen from "./components/WelcomeScreen";
import ProjectWorkspace from "./components/ProjectWorkspace";
import TabBar from "./components/TabBar";
import type { ProjectInfo, Tab } from "./types";
import "./App.css";

function App() {
  const [tabs, setTabs] = useState<Tab[]>([]);
  const [activeTabId, setActiveTabId] = useState<string | null>(null);
  const [addingTab, setAddingTab] = useState(false);
  const [runningProjects, setRunningProjects] = useState<Set<string>>(new Set());
  const closedTabsRef = useRef<Tab[]>([]);
  const tabsRef = useRef(tabs);
  const activeIdRef = useRef(activeTabId);
  const runningByPathRef = useRef<Record<string, boolean>>({});
  tabsRef.current = tabs;
  activeIdRef.current = activeTabId;

  const closeTab = async (tabId: string, skipConfirm?: boolean) => {
    const tab = tabsRef.current.find((t) => t.id === tabId);
    if (!tab) return;

    if (!skipConfirm && runningByPathRef.current[tab.project.path]) {
      const confirmed = await ask(
        "This tab has running scripts. Closing will stop them. Continue?",
        { title: "Stop running scripts?", kind: "warning" },
      );
      if (!confirmed) return;
    }

    try {
      await invoke("kill_project_scripts", {
        projectPath: tab.project.path,
      });
    } catch {}

    runningByPathRef.current[tab.project.path] = false;
    setRunningProjects((prev) => {
      if (!prev.has(tab.project.path)) {
        return prev;
      }

      const next = new Set(prev);
      next.delete(tab.project.path);
      return next;
    });

    closedTabsRef.current.push(tab);
    setTabs((prev) => {
      const remaining = prev.filter((t) => t.id !== tabId);
      if (activeIdRef.current === tabId) {
        if (remaining.length > 0) {
          setActiveTabId(remaining[remaining.length - 1].id);
        } else {
          setActiveTabId(null);
          setAddingTab(false);
        }
      }
      return remaining;
    });
  };

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      if (!mod) return;

      if (e.key === "t" && !e.shiftKey) {
        e.preventDefault();
        setAddingTab(true);
        setActiveTabId(null);
        return;
      }

      if (e.key === "t" && e.shiftKey) {
        e.preventDefault();
        const stack = closedTabsRef.current;
        if (stack.length === 0) return;
        const tab = stack.pop()!;
        setTabs((t) => [...t, tab]);
        setActiveTabId(tab.id);
        setAddingTab(false);
        return;
      }

      if (e.key === "w") {
        e.preventDefault();
        const id = activeIdRef.current;
        if (id) closeTab(id);
        return;
      }
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, []);

  const handleProjectOpen = (project: ProjectInfo) => {
    const existing = tabs.find((t) => t.project.path === project.path);
    if (existing) {
      setActiveTabId(existing.id);
    } else {
      const newTab: Tab = { id: crypto.randomUUID(), project };
      setTabs((prev) => [...prev, newTab]);
      setActiveTabId(newTab.id);
    }
    setAddingTab(false);
    invoke("save_recent_project", { projectPath: project.path }).catch(() => {});
  };

  const handleRunningChange = useCallback((projectPath: string, hasRunning: boolean) => {
    runningByPathRef.current[projectPath] = hasRunning;
    setRunningProjects((prev) => {
      if (hasRunning === prev.has(projectPath)) {
        return prev;
      }

      const next = new Set(prev);
      if (hasRunning) {
        next.add(projectPath);
      } else {
        next.delete(projectPath);
      }
      return next;
    });
  }, []);

  const handleTabSelect = (tabId: string) => {
    setActiveTabId(tabId);
    setAddingTab(false);
  };

  const showOverlay = tabs.length === 0 || addingTab;

  return (
    <div className="app">
      {tabs.length > 0 && (
        <TabBar
          tabs={tabs}
          activeTabId={activeTabId}
          runningProjects={runningProjects}
          onTabSelect={handleTabSelect}
          onTabClose={closeTab}
          onAddTab={() => { setAddingTab(true); setActiveTabId(null); }}
        />
      )}
      <div className="app-content">
        {tabs.map((tab) => (
          <div
            key={tab.id}
            className={`tab-workspace ${tab.id === activeTabId && !addingTab ? "active" : ""}`}
          >
            <ProjectWorkspace
              project={tab.project}
              onRunningChange={handleRunningChange}
            />
          </div>
        ))}
        {showOverlay && (
          <div className="welcome-overlay">
            <WelcomeScreen onProjectOpen={handleProjectOpen} />
          </div>
        )}
      </div>
    </div>
  );
}

export default App;
