import type { Tab } from "../types";

interface Props {
  tabs: Tab[];
  activeTabId: string | null;
  onTabSelect: (tabId: string) => void;
  onTabClose: (tabId: string) => void;
  onAddTab: () => void;
}

function TabBar({ tabs, activeTabId, onTabSelect, onTabClose, onAddTab }: Props) {
  return (
    <div className="tab-bar">
      <div className="tab-list">
        {tabs.map((tab) => (
          <div
            key={tab.id}
            className={`tab ${tab.id === activeTabId ? "active" : ""}`}
            onClick={() => onTabSelect(tab.id)}
          >
            <span className="tab-label">{tab.project.name}</span>
            <button
              className="tab-close"
              onClick={(e) => {
                e.stopPropagation();
                onTabClose(tab.id);
              }}
            >
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <line x1="18" y1="6" x2="6" y2="18" />
                <line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          </div>
        ))}
      </div>
      <button className="tab-add" onClick={onAddTab} title="Open another project">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <line x1="12" y1="5" x2="12" y2="19" />
          <line x1="5" y1="12" x2="19" y2="12" />
        </svg>
      </button>
    </div>
  );
}

export default TabBar;
