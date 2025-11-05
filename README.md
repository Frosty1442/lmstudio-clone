# LMStudio Clone - Open Source Local LLM Platform

An open-source clone of LMStudio that allows you to run large language models locally on your computer with a beautiful desktop interface and OpenAI-compatible API.

## Features

- Run LLMs locally with GPU/CPU acceleration
- OpenAI-compatible API server
- Beautiful Electron-based desktop UI
- Model browser and downloader from Hugging Face
- Chat interface with RAG (document) support
- Vision model support
- Embedding generation
- CLI tool for automation
- Headless server mode
- Cross-platform (Windows, macOS, Linux)

## Quick Start

### Installation

```bash
npm install
```

### Running the Application

```bash
# Desktop mode
npm start

# Headless server mode
npm run server

# CLI tool
npm run cli
```

## Project Structure

```
lmstudio-clone/
├── src/
│   ├── main/               # Electron main process
│   ├── renderer/           # React frontend
│   ├── server/             # API server
│   ├── core/               # Core inference engine
│   ├── cli/                # CLI tool
│   └── shared/             # Shared utilities
├── models/                 # Downloaded models storage
├── chats/                  # Chat history storage
└── config/                 # Configuration files
```

## Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed architecture documentation.

## Features

See [FEATURES.md](FEATURES.md) for complete feature list.

## Development

### Prerequisites

- Node.js 18+
- Python 3.8+ (for llama.cpp bindings)
- CUDA Toolkit (optional, for NVIDIA GPU support)
- CMake and C++ compiler

### Building from Source

```bash
# Install dependencies
npm install

# Build llama.cpp bindings
npm run build:llama

# Build application
npm run build

# Run in development
npm run dev
```

## License

MIT License - see LICENSE file for details

## Credits

Inspired by [LMStudio](https://lmstudio.ai/) - this is an independent open-source implementation.

Built with:
- [llama.cpp](https://github.com/ggerganov/llama.cpp) - LLM inference engine
- [Electron](https://www.electronjs.org/) - Desktop framework
- [React](https://reactjs.org/) - UI framework
- [Express](https://expressjs.com/) - API server

## Contributing

Contributions are welcome! Please read CONTRIBUTING.md for guidelines.

## Roadmap

See [FEATURES.md](FEATURES.md) for implementation roadmap and phases.
