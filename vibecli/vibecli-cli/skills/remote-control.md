# Remote Control

Control VibeCLI sessions from mobile devices or web browsers.

## Triggers
- "remote control", "mobile access", "QR code", "phone control"
- "remote session", "browser control", "pair device"

## Usage
```
/remote start          # Start remote control server
/remote pair           # Generate QR code for pairing
/remote clients        # List connected clients
/remote disconnect     # Disconnect a client
```

## Features
- QR code pairing with time-limited tokens
- WebSocket-based real-time communication
- Permission-based access (execute, approve, view history, modify files)
- Device type detection (Phone, Tablet, Desktop, Browser)
- Event buffering for intermittent connections
- 7 command types: Execute, Approve, Reject, Cancel, GetStatus, ScrollHistory, Disconnect
