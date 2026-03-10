# Cloud Autofix Agent

Cloud-based agents that test and propose fixes directly on pull requests.

## Triggers
- "cloud autofix", "autofix PR", "auto fix", "fix PR"
- "bugbot cloud", "cloud agent fix", "test and fix"

## Usage
```
/autofix pr 42                            # Analyze PR #42
/autofix run fix-1                        # Execute fix in cloud sandbox
/autofix propose fix-1                    # Propose fix as PR comment
/autofix stats                            # Show merge rate stats
/autofix sandbox --image node:20          # Configure sandbox
/autofix strategy minimal                 # Set fix strategy
```

## Fix Types
CompileError, TestFailure, LintViolation, SecurityVuln, TypeMismatch, NullCheck, BoundaryCheck, ResourceLeak

## Features
- Cloud sandbox execution with resource limits (CPU, memory, disk)
- 3 fix strategies: Direct, Minimal, Comprehensive
- Automated test execution per fix attempt
- Merge rate tracking (target: 35%+)
- Confidence scoring per fix
- Fix result tracking: Merged, Rejected, Pending, TestFailed, ConflictDetected
- Pipeline stats: total attempts, merged, rejected, avg confidence
