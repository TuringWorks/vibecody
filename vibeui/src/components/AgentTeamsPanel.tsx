import React, { useState } from "react";

type AgentRole = "Lead" | "Teammate" | "Observer";
type AgentStatus = "Active" | "Idle" | "Busy" | "Offline";
type TaskStatus = "Todo" | "In Progress" | "Done" | "Blocked";
type TaskPriority = "High" | "Medium" | "Low";

interface Agent {
  id: string;
  name: string;
  role: AgentRole;
  status: AgentStatus;
  specialty: string;
}

interface TeamMessage {
  id: string;
  from: string;
  to: string;
  type: "Info" | "Request" | "Update" | "Alert";
  text: string;
  timestamp: string;
}

interface Task {
  id: string;
  title: string;
  status: TaskStatus;
  assignee: string;
  priority: TaskPriority;
}

const AgentTeamsPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<string>("team");
  const [newAgentName, setNewAgentName] = useState("");
  const [newAgentRole, setNewAgentRole] = useState<AgentRole>("Teammate");
  const [newMsgText, setNewMsgText] = useState("");
  const [newMsgTo, setNewMsgTo] = useState("All");
  const [newMsgType, setNewMsgType] = useState<"Info" | "Request" | "Update" | "Alert">("Info");
  const [newTaskTitle, setNewTaskTitle] = useState("");
  const [newTaskPriority, setNewTaskPriority] = useState<TaskPriority>("Medium");

  const [agents, setAgents] = useState<Agent[]>([
    { id: "a1", name: "Architect", role: "Lead", status: "Active", specialty: "System design" },
    { id: "a2", name: "Coder", role: "Teammate", status: "Busy", specialty: "Implementation" },
    { id: "a3", name: "Reviewer", role: "Teammate", status: "Idle", specialty: "Code review" },
    { id: "a4", name: "Tester", role: "Teammate", status: "Active", specialty: "QA & testing" },
    { id: "a5", name: "Watcher", role: "Observer", status: "Active", specialty: "Monitoring" },
  ]);

  const [messages, setMessages] = useState<TeamMessage[]>([
    { id: "msg1", from: "Architect", to: "All", type: "Info", text: "Starting API module redesign.", timestamp: "10:00 AM" },
    { id: "msg2", from: "Coder", to: "Architect", type: "Request", text: "Need schema clarification for User model.", timestamp: "10:05 AM" },
    { id: "msg3", from: "Reviewer", to: "Coder", type: "Update", text: "PR #42 approved with minor suggestions.", timestamp: "10:12 AM" },
    { id: "msg4", from: "Tester", to: "All", type: "Alert", text: "Integration tests failing on auth module.", timestamp: "10:18 AM" },
  ]);

  const [tasks, setTasks] = useState<Task[]>([
    { id: "t1", title: "Design API schema", status: "Done", assignee: "Architect", priority: "High" },
    { id: "t2", title: "Implement user endpoints", status: "In Progress", assignee: "Coder", priority: "High" },
    { id: "t3", title: "Write auth middleware", status: "Todo", assignee: "Coder", priority: "Medium" },
    { id: "t4", title: "Review database migrations", status: "Blocked", assignee: "Reviewer", priority: "Medium" },
    { id: "t5", title: "Add integration tests", status: "In Progress", assignee: "Tester", priority: "Low" },
  ]);

  const containerStyle: React.CSSProperties = {
    padding: "16px", color: "var(--text-primary)",
    backgroundColor: "var(--bg-primary)",
    fontFamily: "inherit", fontSize: "13px",
    height: "100%", overflow: "auto",
  };
  const tabBarStyle: React.CSSProperties = {
    display: "flex", gap: "4px", marginBottom: "16px",
    borderBottom: "1px solid var(--border-color)", paddingBottom: "8px",
  };
  const tabStyle = (active: boolean): React.CSSProperties => ({
    padding: "6px 14px", cursor: "pointer", border: "none",
    backgroundColor: active ? "var(--accent-color)" : "transparent",
    color: active ? "white" : "var(--text-primary)",
    borderRadius: "4px", fontSize: "13px",
  });
  const inputStyle: React.CSSProperties = {
    width: "100%", padding: "6px 10px", boxSizing: "border-box",
    backgroundColor: "var(--bg-secondary)", color: "var(--text-primary)",
    border: "1px solid var(--border-color)", borderRadius: "4px",
  };
  const btnStyle: React.CSSProperties = {
    padding: "6px 14px", cursor: "pointer", border: "none", borderRadius: "4px",
    backgroundColor: "var(--accent-color)", color: "white",
  };
  const btnSmall: React.CSSProperties = { ...btnStyle, padding: "3px 8px", fontSize: "11px" };
  const cardStyle: React.CSSProperties = {
    padding: "10px", marginBottom: "8px", borderRadius: "4px",
    backgroundColor: "var(--bg-secondary)",
    border: "1px solid var(--border-color)",
  };
  const badgeStyle = (color: string): React.CSSProperties => ({
    display: "inline-block", padding: "2px 8px", borderRadius: "10px",
    fontSize: "11px", fontWeight: 600, backgroundColor: color, color: "var(--text-primary)",
  });

  const roleColors: Record<AgentRole, string> = { Lead: "#6a1b9a", Teammate: "#1565c0", Observer: "#757575" };
  const statusColors: Record<AgentStatus, string> = { Active: "#2e7d32", Idle: "#f57f17", Busy: "#e65100", Offline: "#757575" };
  const msgTypeColors = { Info: "#1565c0", Request: "#6a1b9a", Update: "#2e7d32", Alert: "#c62828" };
  const taskStatusColors: Record<TaskStatus, string> = { Todo: "#757575", "In Progress": "#1565c0", Done: "#2e7d32", Blocked: "#c62828" };
  const priorityColors: Record<TaskPriority, string> = { High: "#c62828", Medium: "#f57f17", Low: "#757575" };

  const handleAddAgent = () => {
    if (!newAgentName.trim()) return;
    setAgents(prev => [...prev, {
      id: `a-${Date.now()}`, name: newAgentName, role: newAgentRole, status: "Idle", specialty: "General",
    }]);
    setNewAgentName("");
  };

  const handleSendMessage = () => {
    if (!newMsgText.trim()) return;
    setMessages(prev => [...prev, {
      id: `msg-${Date.now()}`, from: "You", to: newMsgTo, type: newMsgType, text: newMsgText,
      timestamp: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
    }]);
    setNewMsgText("");
  };

  const handleAddTask = () => {
    if (!newTaskTitle.trim()) return;
    setTasks(prev => [...prev, {
      id: `t-${Date.now()}`, title: newTaskTitle, status: "Todo", assignee: "Unassigned", priority: newTaskPriority,
    }]);
    setNewTaskTitle("");
  };

  const cycleTaskStatus = (id: string) => {
    const order: TaskStatus[] = ["Todo", "In Progress", "Done", "Blocked"];
    setTasks(prev => prev.map(t => {
      if (t.id !== id) return t;
      const idx = order.indexOf(t.status);
      return { ...t, status: order[(idx + 1) % order.length] };
    }));
  };

  const renderTeam = () => (
    <div>
      <div style={{ fontSize: "12px", opacity: 0.7, marginBottom: "8px" }}>{agents.length} agents</div>
      {agents.map(a => (
        <div key={a.id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <div>
            <div style={{ display: "flex", gap: "8px", alignItems: "center", marginBottom: "4px" }}>
              <strong>{a.name}</strong>
              <span style={badgeStyle(roleColors[a.role])}>{a.role}</span>
            </div>
            <div style={{ fontSize: "12px", opacity: 0.7 }}>{a.specialty}</div>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
            <span style={{ width: "8px", height: "8px", borderRadius: "50%",
              backgroundColor: statusColors[a.status], display: "inline-block" }} />
            <span style={{ fontSize: "12px" }}>{a.status}</span>
          </div>
        </div>
      ))}
      <div style={{ display: "flex", gap: "8px", marginTop: "12px" }}>
        <input style={{ ...inputStyle, flex: 1 }} value={newAgentName} onChange={e => setNewAgentName(e.target.value)}
          placeholder="Agent name..." onKeyDown={e => e.key === "Enter" && handleAddAgent()} />
        <select style={{ ...inputStyle, width: "120px" }} value={newAgentRole}
          onChange={e => setNewAgentRole(e.target.value as AgentRole)}>
          {(["Lead", "Teammate", "Observer"] as AgentRole[]).map(r => <option key={r} value={r}>{r}</option>)}
        </select>
        <button style={btnStyle} onClick={handleAddAgent}>Add</button>
      </div>
    </div>
  );

  const renderMessages = () => (
    <div>
      {messages.map(m => (
        <div key={m.id} style={cardStyle}>
          <div style={{ display: "flex", justifyContent: "space-between", marginBottom: "4px" }}>
            <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
              <strong>{m.from}</strong>
              <span style={{ opacity: 0.5 }}>&rarr;</span>
              <span>{m.to}</span>
              <span style={badgeStyle(msgTypeColors[m.type])}>{m.type}</span>
            </div>
            <span style={{ fontSize: "11px", opacity: 0.6 }}>{m.timestamp}</span>
          </div>
          <div style={{ fontSize: "13px" }}>{m.text}</div>
        </div>
      ))}
      <div style={{ display: "flex", gap: "8px", marginTop: "12px" }}>
        <select style={{ ...inputStyle, width: "100px" }} value={newMsgTo} onChange={e => setNewMsgTo(e.target.value)}>
          <option value="All">All</option>
          {agents.map(a => <option key={a.id} value={a.name}>{a.name}</option>)}
        </select>
        <select style={{ ...inputStyle, width: "100px" }} value={newMsgType}
          onChange={e => setNewMsgType(e.target.value as "Info" | "Request" | "Update" | "Alert")}>
          {(["Info", "Request", "Update", "Alert"] as const).map(t => <option key={t} value={t}>{t}</option>)}
        </select>
        <input style={{ ...inputStyle, flex: 1 }} value={newMsgText} onChange={e => setNewMsgText(e.target.value)}
          placeholder="Send a message..." onKeyDown={e => e.key === "Enter" && handleSendMessage()} />
        <button style={btnStyle} onClick={handleSendMessage}>Send</button>
      </div>
    </div>
  );

  const renderTasks = () => (
    <div>
      {tasks.map(t => (
        <div key={t.id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <div style={{ flex: 1 }}>
            <div style={{ display: "flex", gap: "8px", alignItems: "center", marginBottom: "4px" }}>
              <span style={{ fontWeight: 600 }}>{t.title}</span>
              <span style={badgeStyle(priorityColors[t.priority])}>{t.priority}</span>
            </div>
            <div style={{ fontSize: "12px", opacity: 0.7 }}>Assigned to: {t.assignee}</div>
          </div>
          <div style={{ display: "flex", gap: "6px", alignItems: "center" }}>
            <span style={badgeStyle(taskStatusColors[t.status])}>{t.status}</span>
            <button style={btnSmall} onClick={() => cycleTaskStatus(t.id)} title="Cycle status">
              &rarr;
            </button>
          </div>
        </div>
      ))}
      <div style={{ display: "flex", gap: "8px", marginTop: "12px" }}>
        <input style={{ ...inputStyle, flex: 1 }} value={newTaskTitle} onChange={e => setNewTaskTitle(e.target.value)}
          placeholder="New task..." onKeyDown={e => e.key === "Enter" && handleAddTask()} />
        <select style={{ ...inputStyle, width: "100px" }} value={newTaskPriority}
          onChange={e => setNewTaskPriority(e.target.value as TaskPriority)}>
          {(["High", "Medium", "Low"] as TaskPriority[]).map(p => <option key={p} value={p}>{p}</option>)}
        </select>
        <button style={btnStyle} onClick={handleAddTask}>Add</button>
      </div>
    </div>
  );

  return (
    <div style={containerStyle}>
      <h2 style={{ margin: "0 0 12px" }}>Agent Teams</h2>
      <div style={tabBarStyle}>
        {[["team", "Team"], ["messages", "Messages"], ["tasks", "Tasks"]].map(([id, label]) => (
          <button key={id} style={tabStyle(activeTab === id)} onClick={() => setActiveTab(id)}>{label}</button>
        ))}
      </div>
      {activeTab === "team" && renderTeam()}
      {activeTab === "messages" && renderMessages()}
      {activeTab === "tasks" && renderTasks()}
    </div>
  );
};

export default AgentTeamsPanel;
