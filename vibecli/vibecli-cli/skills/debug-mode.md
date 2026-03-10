# Debug Mode

Dedicated debugging workflow with breakpoints, watches, stack inspection, and AI-powered root cause analysis.

## Triggers
- "debug mode", "debug session", "debugger", "breakpoint"
- "step through", "watch variable", "stack trace", "root cause"

## Usage
```
/debug start src/main.rs            # Start debug session
/debug break 42                     # Set line breakpoint
/debug break --cond "x > 10" 42     # Conditional breakpoint
/debug watch "user.name"            # Watch expression
/debug step over                    # Step over
/debug step into                    # Step into
/debug continue                     # Continue execution
/debug inspect frame 0              # Inspect stack frame
/debug hypothesis "NullPointerError" # Generate hypotheses
/debug autofix                      # Suggest fixes
/debug sessions                     # List active sessions
```

## Features
- 3 debug modes: Interactive, Automated, Hybrid
- 4 breakpoint types: Line, Conditional, Exception, Logpoint
- 12 debug actions: StepOver, StepInto, StepOut, Continue, Pause, Evaluate, SetBreakpoint, RemoveBreakpoint, Watch, Unwatch, Inspect, RunToLine
- Stack frame inspection with nested variable trees
- AI-powered hypothesis generation from error + stack trace
- Root cause analysis at crash point
- Auto-fix suggestions based on debug findings
- Multiple concurrent debug sessions
