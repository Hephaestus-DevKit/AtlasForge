import { useEffect } from "react";
import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import { Layout } from "./components/Layout";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { Dashboard } from "./pages/Dashboard";
import { Assets } from "./pages/Assets";
import { Repositories } from "./pages/Repositories";
import { Tasks } from "./pages/Tasks";
import { Knowledge } from "./pages/Knowledge";
import { Automations } from "./pages/Automations";
import { Settings } from "./pages/Settings";
import { tickScheduler } from "./api/ipc";

function App() {
  useEffect(() => {
    const tick = () => {
      void tickScheduler().catch(() => {
        // Browser-only development and tests do not expose the Tauri IPC bridge.
      });
    };
    tick();
    const timer = window.setInterval(tick, 60_000);
    return () => window.clearInterval(timer);
  }, []);

  return (
    <ErrorBoundary>
      <BrowserRouter>
        <Routes>
          <Route path="/" element={<Layout />}>
            <Route index element={<Dashboard />} />
            <Route path="assets" element={<Assets />} />
            <Route path="repositories" element={<Repositories />} />
            <Route path="tasks" element={<Tasks />} />
            <Route path="knowledge" element={<Knowledge />} />
            <Route path="automations" element={<Automations />} />
            <Route path="settings" element={<Settings />} />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Route>
        </Routes>
      </BrowserRouter>
    </ErrorBoundary>
  );
}

export default App;
