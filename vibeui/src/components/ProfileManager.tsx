/**
 * ProfileManager — Manage settings profiles.
 * Can be embedded in the Settings panel or used standalone.
 */
import { useState, useEffect, useCallback } from "react";
import {
  listProfiles,
  createProfile,
  deleteProfile,
  setDefaultProfile,
  exportProfile,
  importProfile,
  ProfileInfo,
} from "../hooks/usePanelSettings";

export function ProfileManager() {
  const [profiles, setProfiles] = useState<ProfileInfo[]>([]);
  const [newId, setNewId] = useState("");
  const [newName, setNewName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const list = await listProfiles();
      setProfiles(list);
    } catch (e: any) {
      setError(typeof e === "string" ? e : "Failed to load profiles");
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  const handleCreate = async () => {
    if (!newId.trim() || !newName.trim()) return;
    try {
      await createProfile(newId.trim(), newName.trim());
      setNewId("");
      setNewName("");
      setSuccess(`Profile "${newName}" created`);
      setTimeout(() => setSuccess(null), 3000);
      await load();
    } catch (e: any) {
      setError(typeof e === "string" ? e : "Failed to create profile");
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await deleteProfile(id);
      await load();
    } catch (e: any) {
      setError(typeof e === "string" ? e : "Cannot delete this profile");
    }
  };

  const handleSetDefault = async (id: string) => {
    try {
      await setDefaultProfile(id);
      await load();
    } catch (e: any) {
      setError(typeof e === "string" ? e : "Failed to set default");
    }
  };

  const handleExport = async (id: string) => {
    try {
      const data = await exportProfile(id);
      const blob = new Blob([JSON.stringify(data, null, 2)], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `vibeui-profile-${id}.json`;
      a.click();
      URL.revokeObjectURL(url);
      setSuccess("Profile exported");
      setTimeout(() => setSuccess(null), 3000);
    } catch (e: any) {
      setError(typeof e === "string" ? e : "Export failed");
    }
  };

  const handleImport = async (id: string) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".json";
    input.onchange = async () => {
      const file = input.files?.[0];
      if (!file) return;
      try {
        const text = await file.text();
        const data = JSON.parse(text);
        const count = await importProfile(id, data);
        setSuccess(`Imported ${count} settings`);
        setTimeout(() => setSuccess(null), 3000);
      } catch (e: any) {
        setError(typeof e === "string" ? e : "Import failed");
      }
    };
    input.click();
  };


  return (
    <div className="panel-container">
      <div className="panel-header"><h3>Settings Profiles</h3></div>
      <div className="panel-body">
      <p style={{ fontSize: "var(--font-size-base)", color: "var(--text-secondary)", margin: "0 0 12px" }}>
        Profiles let you save and switch between different panel configurations. All settings are encrypted and stored locally.
      </p>

      {error && (
        <div className="panel-error" style={{ marginBottom: 8 }}>
          {error}
          <button onClick={() => setError(null)} style={{ float: "right", background: "none", border: "none", color: "inherit", cursor: "pointer" }}>x</button>
        </div>
      )}
      {success && (
        <div style={{ padding: "6px 10px", marginBottom: 8, borderRadius: "var(--radius-xs-plus)", background: "var(--success-bg)", color: "var(--success-color)", fontSize: "var(--font-size-base)" }}>
          {success}
        </div>
      )}

      {/* Profile list */}
      <div style={{ display: "flex", flexDirection: "column", gap: 6, marginBottom: 16 }}>
        {profiles.map((p) => (
          <div key={p.id} className="panel-card" style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <div style={{ flex: 1 }}>
              <span style={{ fontWeight: 600, fontSize: "var(--font-size-md)" }}>{p.name}</span>
              <span style={{ marginLeft: 6, fontSize: "var(--font-size-sm)", color: "var(--text-secondary)" }}>({p.id})</span>
              {p.is_default && (
                <span style={{ marginLeft: 6, padding: "1px 6px", borderRadius: 3, fontSize: "var(--font-size-xs)", background: "var(--accent-color)", color: "var(--btn-primary-fg)" }}>default</span>
              )}
            </div>
            <div style={{ display: "flex", gap: 4 }}>
              {!p.is_default && <button className="panel-btn panel-btn-secondary" onClick={() => handleSetDefault(p.id)}>Set Default</button>}
              <button className="panel-btn panel-btn-secondary" onClick={() => handleExport(p.id)}>Export</button>
              <button className="panel-btn panel-btn-secondary" onClick={() => handleImport(p.id)}>Import</button>
              {p.id !== "default" && (
                <button className="panel-btn panel-btn-danger" onClick={() => handleDelete(p.id)}>Delete</button>
              )}
            </div>
          </div>
        ))}
      </div>

      {/* Create profile form */}
      <div className="panel-card">
        <div style={{ fontWeight: 600, fontSize: "var(--font-size-base)", marginBottom: 8 }}>New Profile</div>
        <div style={{ display: "flex", gap: 6 }}>
          <input className="panel-input" style={{ width: 120 }} placeholder="Profile ID" value={newId} onChange={(e) => setNewId(e.target.value)} />
          <input className="panel-input" style={{ flex: 1 }} placeholder="Profile Name" value={newName} onChange={(e) => setNewName(e.target.value)} onKeyDown={(e) => e.key === "Enter" && handleCreate()} />
          <button className="panel-btn panel-btn-primary" onClick={handleCreate}>Create</button>
        </div>
      </div>
      </div>
    </div>
  );
}
