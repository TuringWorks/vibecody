import { Component, ErrorInfo, ReactNode } from "react";

interface Props {
  children: ReactNode;
  fallback?: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("ErrorBoundary caught:", error, info.componentStack);
  }

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) return this.props.fallback;
      return (
        <div style={{ padding: 16, color: "var(--text-danger)", background: "var(--error-bg)", borderRadius: 6, margin: 4 }}>
          <strong>Something went wrong</strong>
          <p style={{ fontSize: 12, opacity: 0.8, marginTop: 4 }}>{this.state.error?.message}</p>
          <button
            onClick={() => this.setState({ hasError: false, error: null })}
            style={{ marginTop: 8, padding: "4px 12px", cursor: "pointer", background: "var(--bg-tertiary)", color: "var(--text-secondary)", border: "1px solid var(--border-color)", borderRadius: 4 }}
          >
            Retry
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
