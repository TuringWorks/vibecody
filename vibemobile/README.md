# VibeCody Mobile

Flutter companion app for remote management of VibeCody CLI and desktop sessions from iOS, Android, macOS, Linux, Windows, and Web.

## Features

- **QR Code Pairing** — Scan a QR code from VibeCLI (`/pair`) or VibeUI to connect
- **Manual Connect** — Enter host:port directly for remote machines
- **Remote Chat** — Send messages to AI providers through connected VibeCody instances
- **Machine Management** — Register, monitor health, view metrics for multiple machines
- **Session Browser** — View and manage active agent sessions
- **Push Notifications** — Get notified when agent tasks complete
- **Dark/Light Theme** — Material Design 3 with custom VibeCody color palette
- **Secure Storage** — API tokens stored in platform keychain (iOS Keychain, Android Keystore)

## Architecture

```
lib/
├── main.dart                    # App entry with Provider setup
├── models/
│   └── machine.dart             # Machine, PairedDevice, Session models
├── screens/
│   ├── home_screen.dart         # Dashboard with machine summary
│   ├── onboarding_screen.dart   # First-run setup flow
│   ├── pair_screen.dart         # QR code scanner for pairing
│   ├── manual_connect_screen.dart # Manual host:port entry
│   ├── machines_screen.dart     # Machine list with status
│   ├── machine_detail_screen.dart # Machine metrics and actions
│   ├── chat_screen.dart         # Remote AI chat interface
│   ├── sessions_screen.dart     # Agent session browser
│   └── settings_screen.dart     # App preferences
├── services/
│   ├── api_client.dart          # HTTP client for VibeCody serve API
│   ├── auth_service.dart        # Authentication and pairing management
│   └── notification_service.dart # Push notification handling
└── theme/
    └── app_theme.dart           # Light/dark Material 3 theming
```

## Getting Started

### Prerequisites

- Flutter SDK ≥3.2.0
- Dart SDK ≥3.2.0
- Xcode (for iOS/macOS)
- Android Studio (for Android)

### Development

```bash
cd vibemobile
flutter pub get
flutter run                # Run on connected device/emulator
flutter run -d chrome      # Run in web browser
flutter run -d macos       # Run on macOS desktop
```

### Production Build

```bash
flutter build ios          # iOS .ipa
flutter build apk          # Android .apk
flutter build appbundle    # Android .aab (Play Store)
flutter build macos        # macOS .app
flutter build web          # Web (deploy to any static host)
```

## Connecting to VibeCody

### Via QR Code (Recommended)

1. Start VibeCLI in serve mode: `vibecli serve --port 7879`
2. Run `/pair` in the REPL to generate a QR code
3. Open VibeMobile → tap "Pair" → scan the QR code
4. The app connects and authenticates automatically

### Via Manual Entry

1. Start VibeCLI in serve mode: `vibecli serve --port 7879`
2. Open VibeMobile → tap "Connect Manually"
3. Enter `<host>:<port>` (e.g., `192.168.1.100:7879`)
4. Enter the API token from VibeCLI

## Dependencies

| Package | Purpose |
|---------|---------|
| `provider` | State management |
| `http` | REST API client |
| `shared_preferences` | Local settings storage |
| `flutter_secure_storage` | Keychain/Keystore for tokens |
| `mobile_scanner` | QR code scanning |

## Backend API

VibeMobile communicates with VibeCody's HTTP serve API (`vibecli serve`). Key endpoints:

- `POST /chat` — Send chat message
- `GET /sessions` — List agent sessions
- `POST /pair` — Register device pairing
- `GET /health` — Server health check
- `GET /status` — Provider and model info

See [VibeCLI Server Mode](../docs/vibecli.md#server-mode) for full API documentation.

## License

MIT — see [LICENSE](../LICENSE) in the repository root.
