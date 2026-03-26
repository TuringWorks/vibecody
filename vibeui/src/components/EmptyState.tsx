import React from "react";

interface EmptyStateProps {
  icon?: React.ReactNode;
  title: string;
  description?: string;
  action?: { label: string; onClick: () => void };
}

export function EmptyState({ icon, title, description, action }: EmptyStateProps) {
  return (
    <div
      role="status"
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        padding: '48px 24px',
        textAlign: 'center',
        gap: '12px',
        height: '100%',
      }}
    >
      {icon && <span style={{ fontSize: '32px', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>{icon}</span>}
      <div style={{ fontWeight: 600, fontSize: '14px', color: 'var(--text-primary)' }}>{title}</div>
      {description && (
        <div style={{ fontSize: '13px', color: 'var(--text-secondary)', maxWidth: '280px' }}>{description}</div>
      )}
      {action && (
        <button
          className="btn-primary"
          onClick={action.onClick}
          style={{ marginTop: '8px' }}
        >
          {action.label}
        </button>
      )}
    </div>
  );
}
