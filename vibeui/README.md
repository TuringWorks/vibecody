# VibeUI - AI-Powered Code Editor

A modern, high-performance code editor built with Rust and Tauri, featuring AI-powered code completion and chat capabilities.

## Features

### Core Editor
- ✅ **Efficient Text Buffer**: Built on the `ropey` rope data structure for fast editing of large files
- ✅ **File System Operations**: Async file I/O with file watching capabilities
- ✅ **Multi-Workspace Support**: Work with multiple project folders simultaneously
- 🚧 **Syntax Highlighting**: Monaco Editor integration (in progress)
- 🚧 **LSP Support**: Language Server Protocol client for intelligent code features

### AI Integration
- ✅ **Ollama Support**: Local AI models for code completion and chat
- 🚧 **Claude**: Anthropic Claude API integration (planned)
- 🚧 **ChatGPT/OpenAI**: OpenAI API integration (planned)
- 🚧 **Gemini**: Google Gemini API integration (planned)
- 🚧 **Grok**: xAI Grok API integration (planned)

### Architecture
- **Backend**: Rust with Tauri 2.0
- **Frontend**: React + TypeScript with Monaco Editor
- **Text Processing**: Ropey rope data structure
- **AI Providers**: Modular plugin system
- **Extension System**: VSCode-compatible API (planned)

## Project Structure

```
vibeUI/
├── src/                    # React frontend
├── src-tauri/             # Tauri Rust backend
├── crates/
│   ├── vibe-core/         # Text buffer, file system, workspace
│   ├── vibe-lsp/          # LSP client implementation
│   ├── vibe-ai/           # AI provider abstraction
│   └── vibe-extensions/   # Extension system
└── docs/                  # Documentation
```

## Getting Started

### Prerequisites
- Rust (latest stable)
- Node.js 18+
- npm or pnpm

### Development

1. **Install dependencies**:
   ```bash
   npm install
   ```

2. **Run in development mode**:
   ```bash
   npm run tauri dev
   ```

3. **Build for production**:
   ```bash
   npm run tauri build
   ```

### Testing the Rust Backend

```bash
# Run all tests
cargo test --workspace

# Check compilation
cargo check --workspace

# Run specific crate tests
cargo test -p vibe-core
cargo test -p vibe-ai
```

## AI Provider Configuration

### Ollama (Local AI)

1. Install Ollama from [ollama.ai](https://ollama.ai)
2. Pull a code model:
   ```bash
   ollama pull codellama
   ```
3. The editor will automatically detect and use Ollama if it's running

### Cloud AI Providers (Coming Soon)

Configuration for Claude, ChatGPT, Gemini, and Grok will be added through the settings UI.

## Development Status

This project is in active development. The following components are currently implemented:

- ✅ Rust workspace with modular crates
- ✅ Text buffer with undo/redo and multi-cursor support
- ✅ File system operations with watching
- ✅ Workspace management
- ✅ AI provider abstraction
- ✅ Ollama integration
- ✅ Tauri commands for frontend-backend communication
- ✅ Monaco Editor integration
- ✅ UI components (Sidebar, Status Bar, Tabs)
- ✅ Terminal integration
- ✅ Git status visualization
- ✅ Theme system (Dark/Light)
- ✅ Command Palette
- 🚧 LSP client
- 🚧 Extension system

## Contributing

This is currently a personal project, but contributions and suggestions are welcome!

## License

MIT

## Acknowledgments

- Built with [Tauri](https://tauri.app/)
- Editor powered by [Monaco Editor](https://microsoft.github.io/monaco-editor/)
- Text processing with [Ropey](https://github.com/cessen/ropey)
- AI integration with [Ollama](https://ollama.ai) and cloud providers
