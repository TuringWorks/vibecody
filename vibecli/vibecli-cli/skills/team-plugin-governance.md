# Team Plugin Marketplace Governance

Admin controls for sharing private plugins within teams with approval workflows and compliance checking.

## Triggers
- "team governance", "plugin approval", "team marketplace"
- "plugin policy", "governance controls", "private plugins"

## Usage
```
/governance register my-plugin            # Register plugin
/governance submit my-plugin              # Submit for approval
/governance approve plugin-1              # Approve (reviewer)
/governance reject plugin-1 "Needs tests" # Reject with reason
/governance policy set require-approval   # Set team policy
/governance audit                         # View audit log
/governance compliance plugin-1           # Check compliance
/governance list --team my-team           # List team plugins
```

## Features
- 4 visibility levels: Private, TeamOnly, Organization, Public
- 4 approval statuses: Pending, Approved, Rejected, Deprecated
- Team governance policies (require approval, allowed/blocked categories, size limits, SHA pinning)
- Compliance checking with issue reporting
- Full audit trail (actions, actors, timestamps)
- Multi-reviewer approval workflow
