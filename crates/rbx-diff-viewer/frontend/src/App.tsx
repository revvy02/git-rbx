import { useState, useCallback } from 'react';
import { AppProvider, useAppContext } from './context/AppContext';
import { ExplorerPanel } from './components/ExplorerPanel';
import { ResizeHandle } from './components/ResizeHandle';
import './styles/index.css';

function DiffViewer() {
  const { state } = useAppContext();
  // Use ratios for flexible panel sizing: [old, new, diff]
  const [panelRatios, setPanelRatios] = useState([1, 1, 1.5]);

  const handleResize = useCallback((panelIndex: number) => (delta: number) => {
    setPanelRatios(prev => {
      const totalWidth = window.innerWidth;
      const deltaRatio = delta / totalWidth * (prev[0] + prev[1] + prev[2]);
      const newRatios = [...prev];

      // Adjust the panel being resized and the one after it
      newRatios[panelIndex] = Math.max(0.3, newRatios[panelIndex] + deltaRatio);
      newRatios[panelIndex + 1] = Math.max(0.3, newRatios[panelIndex + 1] - deltaRatio);

      return newRatios;
    });
  }, []);

  // Show loading while data is being initialized
  if (!state.isLoaded) {
    return (
      <div className="loading-screen">
        <div className="loading">Loading diff data...</div>
      </div>
    );
  }

  return (
    <div className="diff-view">
      <div className="header">
        <h1>rbx-diff-viewer</h1>
        <span className="file-names">
          {state.meta?.old_name} → {state.meta?.new_name}
        </span>
        <div className="summary-badge">
          <span className="added">+{state.meta?.summary.added ?? 0} added</span>
          <span className="removed">-{state.meta?.summary.removed ?? 0} removed</span>
          <span className="modified">~{state.meta?.summary.modified ?? 0} modified</span>
        </div>
      </div>
      <div className="container">
        <div className="panel file-panel" style={{ flex: panelRatios[0] }}>
          <ExplorerPanel side="old" title="OLD FILE" />
        </div>
        <ResizeHandle direction="vertical" onResize={handleResize(0)} />
        <div className="panel file-panel" style={{ flex: panelRatios[1] }}>
          <ExplorerPanel side="new" title="NEW FILE" />
        </div>
        <ResizeHandle direction="vertical" onResize={handleResize(1)} />
        <div className="panel diff-panel" style={{ flex: panelRatios[2] }}>
          <ExplorerPanel side="diff" title="CHANGES" />
        </div>
      </div>
    </div>
  );
}

function App() {
  return (
    <AppProvider>
      <DiffViewer />
    </AppProvider>
  );
}

export default App;
