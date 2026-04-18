# VibeUI Design-System Audit

_Generated 2026-04-18 by `vibeui/scripts/audit-design-system.mjs`._

Scanned **235** panels in `vibeui/src/components/*Panel.tsx`.

- ✅ Clean (zero violations): **149**
- ⚠ With violations: **86**

## Violations by rule

| Severity | Rule | Total occurrences | Affected files |
|---|---|---:|---:|
| 🟠 med | height: 100% (rule 1: use flex: 1, minHeight: 0) | 63 | 48 |
| 🟡 low | 'Loading…' text not wrapped in .panel-loading | 8 | 8 |
| 🟡 low | Empty/no-items text not wrapped in .panel-empty | 5 | 5 |
| 🟡 low | Custom tab styling instead of .panel-tab-bar / .panel-tab | 38 | 38 |

## Worst offenders (top 30 by weighted score)

Score = sum of (occurrences × severity-weight). High=3, Med=2, Low=1.

| Score | Panel | High | Med | Low |
|---:|---|---:|---:|---:|
| 8 | `AgilePanel.tsx` | 0 | 4 | 0 |
| 7 | `CloudAutofixPanel.tsx` | 0 | 3 | 1 |
| 7 | `VoiceVocabPanel.tsx` | 0 | 3 | 1 |
| 6 | `EnvDispatchPanel.tsx` | 0 | 2 | 2 |
| 5 | `QuantumComputingPanel.tsx` | 0 | 2 | 1 |
| 5 | `TeamOnboardingPanel.tsx` | 0 | 2 | 1 |
| 4 | `ColorConverterPanel.tsx` | 0 | 2 | 0 |
| 4 | `FineTuningPanel.tsx` | 0 | 2 | 0 |
| 4 | `IdpPanel.tsx` | 0 | 2 | 0 |
| 4 | `SpecPipelinePanel.tsx` | 0 | 2 | 0 |
| 4 | `WorkflowPanel.tsx` | 0 | 2 | 0 |
| 3 | `AstEditPanel.tsx` | 0 | 1 | 1 |
| 3 | `AutoDeployPanel.tsx` | 0 | 1 | 1 |
| 3 | `KnowledgeGraphPanel.tsx` | 0 | 1 | 1 |
| 3 | `SecurityScanPanel.tsx` | 0 | 1 | 1 |
| 3 | `SessionMemoryPanel.tsx` | 0 | 1 | 1 |
| 2 | `A2aPanel.tsx` | 0 | 1 | 0 |
| 2 | `AgentTeamsPanel.tsx` | 0 | 1 | 0 |
| 2 | `AiCodeReviewPanel.tsx` | 0 | 0 | 2 |
| 2 | `AppBuilderPanel.tsx` | 0 | 1 | 0 |
| 2 | `ArchitectureSpecPanel.tsx` | 0 | 1 | 0 |
| 2 | `AutoResearchPanel.tsx` | 0 | 1 | 0 |
| 2 | `BlueTeamPanel.tsx` | 0 | 1 | 0 |
| 2 | `BrowserPanel.tsx` | 0 | 1 | 0 |
| 2 | `CompliancePanel.tsx` | 0 | 1 | 0 |
| 2 | `CostPanel.tsx` | 0 | 1 | 0 |
| 2 | `DiagramGeneratorPanel.tsx` | 0 | 1 | 0 |
| 2 | `EncodingPanel.tsx` | 0 | 1 | 0 |
| 2 | `InfiniteContextPanel.tsx` | 0 | 1 | 0 |
| 2 | `LoadTestPanel.tsx` | 0 | 1 | 0 |

## Per-panel detail (panels with violations only)

### `AgilePanel.tsx` — score 8

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 4 |

### `CloudAutofixPanel.tsx` — score 7

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 3 |
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `VoiceVocabPanel.tsx` — score 7

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 3 |
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `EnvDispatchPanel.tsx` — score 6

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 2 |
| Empty/no-items text not wrapped in .panel-empty | 1 |
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `QuantumComputingPanel.tsx` — score 5

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 2 |
| 'Loading…' text not wrapped in .panel-loading | 1 |

### `TeamOnboardingPanel.tsx` — score 5

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 2 |
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `ColorConverterPanel.tsx` — score 4

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 2 |

### `FineTuningPanel.tsx` — score 4

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 2 |

### `IdpPanel.tsx` — score 4

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 2 |

### `SpecPipelinePanel.tsx` — score 4

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 2 |

### `WorkflowPanel.tsx` — score 4

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 2 |

### `AstEditPanel.tsx` — score 3

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `AutoDeployPanel.tsx` — score 3

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `KnowledgeGraphPanel.tsx` — score 3

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `SecurityScanPanel.tsx` — score 3

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `SessionMemoryPanel.tsx` — score 3

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |
| 'Loading…' text not wrapped in .panel-loading | 1 |

### `A2aPanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `AgentTeamsPanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `AiCodeReviewPanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| 'Loading…' text not wrapped in .panel-loading | 1 |
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `AppBuilderPanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `ArchitectureSpecPanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `AutoResearchPanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `BlueTeamPanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `BrowserPanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `CompliancePanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `CostPanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `DiagramGeneratorPanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `EncodingPanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `InfiniteContextPanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `LoadTestPanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `McpLazyPanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `McpPanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `OnDevicePanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| Empty/no-items text not wrapped in .panel-empty | 1 |
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `OpenMemoryPanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `OrchestrationPanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `OrgContextPanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `RenderOptimizePanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `ResiliencePanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `ReviewPanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `SessionBrowserPanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `SettingsPanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `SpawnAgentPanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `SpecPanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `SuperBrainPanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `SweBenchPanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `TestPanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `TrainingPanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `TransformPanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `UsageMeteringPanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `WorkManagementPanel.tsx` — score 2

| Rule | Occurrences |
|---|---:|
| height: 100% (rule 1: use flex: 1, minHeight: 0) | 1 |

### `AdminPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `AgentRecordingPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| 'Loading…' text not wrapped in .panel-loading | 1 |

### `CloudSandboxPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `CompanyDocumentsPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `DemoPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| 'Loading…' text not wrapped in .panel-loading | 1 |

### `DesignHubPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| 'Loading…' text not wrapped in .panel-loading | 1 |

### `DrawioEditorPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `GpuTerminalPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Empty/no-items text not wrapped in .panel-empty | 1 |

### `HardProblemPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `HealthScorePanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `IdeBridgePanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `ImageGenPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| 'Loading…' text not wrapped in .panel-loading | 1 |

### `IntentRefactorPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `JwtPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `LogPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| 'Loading…' text not wrapped in .panel-loading | 1 |

### `LongContextPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `McpGovernancePanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `MemoryPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `MsafPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `NestedAgentsPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `PencilPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `PenpotPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `PlanDocumentPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `PolicyEnginePanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `ProfilerPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Empty/no-items text not wrapped in .panel-empty | 1 |

### `QaValidationPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `RemoteControlPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `ReproAgentPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `ReviewProtocolPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `SelfReviewPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `SkillDistillationPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `ThoughtStreamPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `VibeSqlPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `WatchManagementPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |

### `WebSocketPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Empty/no-items text not wrapped in .panel-empty | 1 |

### `WebhookPanel.tsx` — score 1

| Rule | Occurrences |
|---|---:|
| Custom tab styling instead of .panel-tab-bar / .panel-tab | 1 |
