# Getting Started with LMStudio Clone

This guide will help you get up and running with LMStudio Clone, an open-source alternative to LMStudio for running large language models locally.

## Prerequisites

Before you begin, ensure you have the following installed:

- **Node.js** 18 or higher
- **Python** 3.8 or higher (for llama.cpp bindings)
- **Git**
- **CMake** and a C++ compiler (for building llama.cpp)
- (Optional) **CUDA Toolkit** for NVIDIA GPU support

### System Requirements

**Minimum:**
- 8GB RAM
- 10GB free disk space
- CPU with AVX2 support

**Recommended:**
- 16GB+ RAM
- 50GB+ free disk space (for models)
- NVIDIA GPU with 6GB+ VRAM or Apple Silicon Mac

## Installation

### 1. Clone the Repository

```bash
git clone <repository-url>
cd lmstudio-clone
```

### 2. Install Dependencies

```bash
npm install
```

This will install all Node.js dependencies. The installation may take several minutes.

### 3. Build llama.cpp Bindings

```bash
npm run build:llama
```

This compiles the llama.cpp library with your system's native optimizations.

## Quick Start

### Option 1: Desktop Application

Start the desktop application with:

```bash
npm run dev
```

This will:
1. Start the Vite development server for the UI
2. Launch the Electron main process
3. Start the API server on port 1234

The application window will open automatically.

### Option 2: Headless Server Mode

Run as a headless server without the GUI:

```bash
npm run server
```

The API server will be available at `http://localhost:1234`

### Option 3: CLI Tool

Use the command-line interface:

```bash
npm run cli -- <command>
```

## Downloading Your First Model

### Using the Desktop UI

1. Launch the application
2. Navigate to the **Models** page
3. Click **Download Model**
4. Enter a Hugging Face model repository, for example:
   - `lmstudio-community/Meta-Llama-3-8B-Instruct-GGUF`
   - `TheBloke/Mistral-7B-Instruct-v0.2-GGUF`
5. Click **Download**

### Using the CLI

Download a model from Hugging Face:

```bash
npm run cli -- get lmstudio-community/Meta-Llama-3-8B-Instruct-GGUF
```

With specific quantization:

```bash
npm run cli -- get lmstudio-community/Meta-Llama-3-8B-Instruct-GGUF -q q4_k_m
```

### Manual Installation

You can also manually download GGUF models and place them in:

```
~/.lmstudio-clone/models/<model-id>/model.gguf
```

## Loading and Using Models

### Desktop UI

1. Go to the **Models** page
2. Find your downloaded model
3. Click **Load**
4. Navigate to the **Chat** page
5. Start chatting!

### CLI

Load a model:

```bash
npm run cli -- load <model-id>
```

Check loaded models:

```bash
npm run cli -- ps
```

List all models:

```bash
npm run cli -- ls
```

## Using the OpenAI-Compatible API

Once a model is loaded, you can use the OpenAI-compatible API:

### Python Example

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:1234/v1",
    api_key="not-needed"
)

response = client.chat.completions.create(
    model="your-model-id",
    messages=[
        {"role": "user", "content": "Hello! How are you?"}
    ]
)

print(response.choices[0].message.content)
```

### cURL Example

```bash
curl http://localhost:1234/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "your-model-id",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

### JavaScript/Node.js Example

```javascript
const response = await fetch('http://localhost:1234/v1/chat/completions', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    model: 'your-model-id',
    messages: [{ role: 'user', content: 'Hello!' }]
  })
});

const data = await response.json();
console.log(data.choices[0].message.content);
```

## Configuration

### Settings UI

Access settings in the desktop application:
1. Navigate to the **Settings** page
2. Configure:
   - Theme (Dark, Light, Sepia, System)
   - API port
   - Network access
   - Auto-load model
   - Developer mode

### Configuration File

Settings are stored in:
- **macOS**: `~/Library/Application Support/lmstudio-clone/config.json`
- **Windows**: `%APPDATA%\lmstudio-clone\config.json`
- **Linux**: `~/.config/lmstudio-clone/config.json`

## GPU Acceleration

### NVIDIA GPUs

The application automatically detects CUDA-capable GPUs. Configure GPU layers when loading a model:

```bash
npm run cli -- load <model-id> --gpu-layers 35
```

Higher numbers offload more layers to the GPU, improving performance but requiring more VRAM.

### Apple Silicon (M1/M2/M3)

Apple Metal acceleration is automatically enabled on macOS with Apple Silicon.

## CLI Commands Reference

```bash
# Show status
npm run cli -- status

# List all models
npm run cli -- ls

# List loaded models
npm run cli -- ps

# Download a model
npm run cli -- get <huggingface-repo> [-q quantization]

# Load a model
npm run cli -- load <model-id> [-g gpu-layers]

# Unload a model
npm run cli -- unload <model-id>

# Start API server
npm run cli -- server [-p port] [-h host]
```

## Troubleshooting

### Model Loading Fails

**Problem**: Model fails to load or crashes

**Solutions**:
- Reduce GPU layers: Try loading with fewer GPU layers
- Check RAM: Ensure you have enough RAM for the model
- Check logs: Enable developer mode for detailed logs

### Slow Inference

**Problem**: Generation is very slow

**Solutions**:
- Enable GPU acceleration: Increase `--gpu-layers`
- Use quantized models: Q4 or Q5 models are faster
- Close other applications: Free up system resources

### Port Already in Use

**Problem**: "Port 1234 already in use"

**Solutions**:
- Change port in Settings or use `-p` flag
- Stop other LMStudio instances
- Check for processes using the port: `lsof -i :1234` (Mac/Linux)

### Download Fails

**Problem**: Model download fails or times out

**Solutions**:
- Check internet connection
- Try downloading manually from Hugging Face
- Use a different model mirror

## Recommended Models

### For 8GB RAM Systems
- **TinyLlama 1.1B** (Q4): Great for testing
- **Phi-2 2.7B** (Q4): Good quality, small size
- **Gemma 2B** (Q4): Fast and capable

### For 16GB RAM Systems
- **Mistral 7B** (Q4): Excellent balance
- **Llama 3 8B** (Q4): High quality
- **CodeLlama 7B** (Q4): Best for coding

### For 32GB+ RAM or GPU Systems
- **Llama 3 70B** (Q4): Highest quality
- **Mixtral 8x7B** (Q4): Fast and powerful
- **CodeLlama 34B** (Q4): Professional coding

## Next Steps

- Explore [FEATURES.md](FEATURES.md) for complete feature list
- Read [ARCHITECTURE.md](ARCHITECTURE.md) for technical details
- Check [README.md](README.md) for development information
- Report issues on GitHub

## Getting Help

- **Documentation**: Check the docs/ folder
- **Issues**: Report bugs on GitHub Issues
- **Discussions**: Join GitHub Discussions
- **Community**: Check the project Discord/forum

## Building from Source

### Development Build

```bash
npm run dev
```

### Production Build

```bash
npm run build
npm start
```

### Create Installers

```bash
# All platforms
npm run dist

# Specific platform
npm run dist:win   # Windows
npm run dist:mac   # macOS
npm run dist:linux # Linux
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [llama.cpp](https://github.com/ggerganov/llama.cpp) - LLM inference engine
- [LMStudio](https://lmstudio.ai/) - Original inspiration
- [Hugging Face](https://huggingface.co/) - Model repository
- All open-source contributors
