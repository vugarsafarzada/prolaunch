import { useState, useRef } from "react";
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
  const runningByPathRef = useRef<Record<string, boolean>>({});

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
  };

  const handleAddTab = () => {
    setAddingTab(true);
    setActiveTabId(null);
  };

  const handleTabClose = async (tabId: string) => {
    const tab = tabs.find((t) => t.id === tabId);
    if (!tab) return;

    if (runningByPathRef.current[tab.project.path]) {
      const confirmed = await ask(
        "This tab has running scripts. Closing will stop them. Continue?",
        { title: "Stop running scripts?", kind: "warning" },
      );
      if (!confirmed) return;
    }

    for (const script of tab.project.scripts) {
      try {
        await invoke("kill_script", {
          projectPath: tab.project.path,
          scriptName: script.name,
        });
      } catch {}
    }

    const remaining = tabs.filter((t) => t.id !== tabId);
    setTabs(remaining);

    if (activeTabId === tabId) {
      if (remaining.length > 0) {
        setActiveTabId(remaining[remaining.length - 1].id);
      } else {
        setActiveTabId(null);
        setAddingTab(false);
      }
    }
  };

  const handleRunningChange = (projectPath: string, hasRunning: boolean) => {
    runningByPathRef.current[projectPath] = hasRunning;
  };

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
          onTabSelect={handleTabSelect}
          onTabClose={handleTabClose}
          onAddTab={handleAddTab}
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
