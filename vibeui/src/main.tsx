import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

// Global error handlers — log to console before they crash the WebView
window.addEventListener("error", (e) => {
  console.error("[GLOBAL ERROR]", e.message, e.filename, e.lineno, e.error);
});
window.addEventListener("unhandledrejection", (e) => {
  console.error("[UNHANDLED PROMISE]", e.reason);
});

// Error boundary to catch React render crashes
class ErrorBoundary extends React.Component<
  { children: React.ReactNode },
  { error: Error | null }
> {
  state = { error: null as Error | null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error("[REACT CRASH]", error.message, info.componentStack);
  }

  render() {
    if (this.state.error) {
      return (
        <div style={{ padding: 24, color: "#f87171", fontFamily: "monospace", background: "#0d1117", height: "100vh", overflow: "auto" }}>
          <h2>VibeUI crashed</h2>
          <pre style={{ whiteSpace: "pre-wrap", fontSize: 13, color: "#e6edf3" }}>
            {this.state.error.message}
            {"\n\n"}
            {this.state.error.stack}
          </pre>
          <button
            onClick={() => { this.setState({ error: null }); }}
            style={{ marginTop: 16, padding: "8px 16px", background: "#6366f1", color: "#fff", border: "none", borderRadius: 4, cursor: "pointer" }}
          >
            Try to recover
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <ErrorBoundary>
    <App />
  </ErrorBoundary>,
);
