import { Routes, Route, Navigate } from "react-router-dom";
import Layout from "@/components/Layout";
import Dashboard from "@/pages/Dashboard";
import Settings from "@/pages/Settings";
import Logs from "@/pages/Logs";
import ModelMappings from "@/pages/ModelMappings";

export default function App() {
  return (
    <Routes>
      <Route path="/" element={<Layout />}>
        <Route index element={<Navigate to="/dashboard" replace />} />
        <Route path="dashboard" element={<Dashboard />} />
        <Route path="logs" element={<Logs />} />
        <Route path="mappings" element={<Navigate to="/mappings/claude" replace />} />
        <Route path="mappings/:client" element={<ModelMappings />} />
        <Route path="settings" element={<Settings />} />
      </Route>
    </Routes>
  );
}
