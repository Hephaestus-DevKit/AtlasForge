import { useEffect, useState, type ReactNode } from "react";
import { Layout } from "./components/Layout";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { Dashboard } from "./pages/Dashboard";
import { Assets } from "./pages/Assets";
import { Repositories } from "./pages/Repositories";
import { Tasks } from "./pages/Tasks";
import { Knowledge } from "./pages/Knowledge";
import { Automations } from "./pages/Automations";
import { Settings } from "./pages/Settings";
import { currentRoute, type AppRoute } from "./routing";

const pages: Record<AppRoute, ReactNode> = {
  "/": <Dashboard />,
  "/assets": <Assets />,
  "/repositories": <Repositories />,
  "/tasks": <Tasks />,
  "/knowledge": <Knowledge />,
  "/automations": <Automations />,
  "/settings": <Settings />,
};

function App() {
  const [route, setRoute] = useState<AppRoute>(() => currentRoute());

  useEffect(() => {
    const updateRoute = () => setRoute(currentRoute());
    window.addEventListener("hashchange", updateRoute);
    return () => window.removeEventListener("hashchange", updateRoute);
  }, []);

  return (
    <ErrorBoundary>
      <Layout currentRoute={route}>{pages[route]}</Layout>
    </ErrorBoundary>
  );
}

export default App;
