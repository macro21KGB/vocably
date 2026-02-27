# Vocably

> THIS IS STILL A PREVIEW, NOT THE FINAL APPLICATION, BEWARE OF BUGS AND CRASHES

Vocably is a smart, AI-powered desktop application written in Rust and powered by `egui`. It helps you refine your writing by analyzing your text and suggesting better, more contextually appropriate alternative words. The app uses Large Language Models (LLMs) to understand your sentences and highlight words that can be improved.

## Features

- **Local Desktop Interface**: Built with `eframe` (egui) for a fast, lightweight native experience.
- **Smart Synonym Suggestions**: Click on highlighted words to view tailored alternatives.
- **Multiple AI Providers**: Integrated support for leading LLM providers:
  - OpenRouter
  - Google
  - Groq
  - Anthropic
  - Custom Providers
- **One-Click Word Replacement**: Effortlessly swap your original words with the suggested improvements.
- **Local Configuration**: Securely saves API keys locally in `~/.text_analyzer_config.json`.

## Installation & Usage

1. **Build and Run**
   Ensure you have Rust and Cargo installed, then run:

   ```bash
   cargo run --release
   ```

2. **Setup Provider Options**
   - Click **Show Options** in the app interface.
   - Select your preferred provider and input your API Key.
   - Set your preferred Model and Temperature.
   - Click **Save**.

3. **Analyze Your Text**
   - Type or paste your text into the main window.
   - Click **Analyze text**.
   - Review the highlighted words, click them, and replace them seamlessly.
