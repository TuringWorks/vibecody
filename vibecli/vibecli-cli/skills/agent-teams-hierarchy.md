# Agent Teams with Lead/Teammate Hierarchy

Multi-agent teams with lead coordination, peer-to-peer messaging, and shared task lists.

## Triggers
- "agent team", "lead agent", "teammate", "team hierarchy"
- "peer messaging", "delegate task", "team coordination"

## Usage
```
/team create "Backend Refactor Squad"     # Create team
/team add-lead agent-1                    # Assign lead
/team add-member agent-2 "testing"        # Add teammate with capability
/team delegate agent-1 agent-2 "Write tests" # Lead delegates
/team msg agent-2 agent-3 "Need API schema" # Peer-to-peer message
/team broadcast agent-1 "Starting phase 2" # Lead broadcasts
/team escalate agent-3 "Blocked on auth"  # Escalate to lead
/team status                              # Show team status
/team tasks                               # Show shared task list
```

## Features
- 3 agent roles: Lead, Teammate, Observer
- 7 message types: TaskAssignment, StatusUpdate, PeerRequest, PeerResponse, Broadcast, DirectMessage, Escalation
- Shared task list with assignment, priorities, and dependencies
- Lead delegates tasks to teammates
- Peer-to-peer direct messaging between teammates
- Escalation from teammate to lead
- Team status overview (agents, tasks, messages)
