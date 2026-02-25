use serde::{Deserialize, Serialize};
use std::sync::mpsc;

use eframe::egui;
use llm::{
    builder::LLMBuilder,
    chat::{ChatMessage, StructuredOutputFormat},
};

#[tokio::main]
async fn main() {
    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native(
        "My egui App",
        native_options,
        Box::new(|cc| Ok(Box::new(MyEguiApp::new(cc)))),
    );
}

async fn ask_ai_for_alternative_words(
    text: &str,
) -> Result<Vec<AlternativeWord>, Box<dyn std::error::Error>> {
    let system_prompt = "You are a helpful assistant that provides alternative words for a given text, you will try to enhance or offer more appropriate suggestions for the words in the text, You will return a list fo word for the one you think will need changes and for each of this world you will give a list of alternatives, you will retrun the result in a json format only provide the ARRAY of AlternativeWord with the following keys: word,start_position,end_position and alternatives, just output the array of objects withoyt any additional text or explaination, and no keys for grouping like 'suggested_changes' or 'alternative_words', just the array of objects";
    let schema_text = r#"
        {
            "type": "array",
            "name": "alternative_words",
            "description": "A list of words that have suggested alternatives, along with their positions in the original text and the alternative suggestions.",
            "items": {
                "type": "object",
                "properties": {
                    "word": {
                        "type": "string",
                        "description": "The word that needs alternatives"
                    },
                    "start_position": {
                        "type": "integer",
                        "description": "The start position of the word in the original text"
                    },
                    "end_position": {
                        "type": "integer",
                        "description": "The end position of the word in the original text"
                    },
                    "alternatives": {
                        "type": "array",
                        "items": {
                            "type": "string",
                            "description": "An alternative word"
                        },
                        "description": "A list of alternative words for the given word"
                    }
                },
                "required": ["word", "start_position", "end_position", "alternatives"]
            }
        }
    "#;

    let schema: StructuredOutputFormat =
        serde_json::from_str(schema_text).map_err(|e| format!("Invalid JSON schema: {}", e))?;

    let api_key = std::env::var("OPENROUTER_API_KEY")
        .map_err(|_| "OPENROUTER_API_KEY environment variable not set")?;

    let llm = LLMBuilder::new()
        .backend(llm::builder::LLMBackend::OpenRouter)
        .api_key(api_key)
        .model("google/gemini-2.5-flash")
        .temperature(0.7)
        .system(system_prompt)
        .schema(schema)
        .build()
        .map_err(|e| format!("Failed to build LLM: {}", e))?;

    let messages = vec![
        ChatMessage::user()
            .content("Here is the text I want you to analyze:".to_string() + text)
            .build(),
    ];

    let response = llm
        .chat(&messages)
        .await
        .map_err(|e| format!("LLM chat error: {}", e))?;

    let text = response
        .text()
        .ok_or_else(|| "Failed to get text from response".to_string())?;

    // remove ```json and ``` if they exist
    let text = text.trim();
    let text = text.strip_prefix("```json").unwrap_or(text);
    let text = text.strip_suffix("```").unwrap_or(text);

    println!("Raw response text: {}", text);

    let alternative_words: Vec<AlternativeWord> =
        serde_json::from_str(&text).map_err(|e| format!("Failed to parse JSON response: {}", e))?;

    Ok(alternative_words)
}

#[derive(Debug, serde::Deserialize)]
struct AlternativeWord {
    word: String,
    start_position: usize,
    end_position: usize,
    alternatives: Vec<String>,
}

#[derive(Default, Deserialize)]
struct MyEguiApp {
    initial_text: String,
    alternatives: Vec<AlternativeWord>,
    error_message: Option<String>,
    // Channel to receive results from async task
    #[serde(skip)]
    result_receiver: Option<mpsc::Receiver<Result<Vec<AlternativeWord>, String>>>,
    #[serde(skip)]
    selected_idx: Option<usize>,
}

impl MyEguiApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::light());
        Self::default()
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for results from async task
        if let Some(ref receiver) = self.result_receiver {
            if let Ok(result) = receiver.try_recv() {
                match result {
                    Ok(words) => {
                        self.alternatives = words;
                        self.error_message = None;
                    }
                    Err(e) => {
                        self.error_message = Some(e);
                    }
                }
                self.result_receiver = None;
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Text Analyzer");
            ui.horizontal(|ui| {
                let button_accept = ui.button("Analyze text");

                if button_accept.clicked() && self.result_receiver.is_none() {
                    let initial_text = self.initial_text.clone();
                    let (tx, rx) = mpsc::channel();
                    self.result_receiver = Some(rx);

                    tokio::spawn(async move {
                        let result = ask_ai_for_alternative_words(&initial_text).await;
                        let _ = tx.send(result.map_err(|e| e.to_string()));
                    });
                }
            });

            // Show error if any
            if let Some(ref error) = self.error_message {
                ui.colored_label(egui::Color32::RED, error);
            }

            // Show alternatives
            if !self.alternatives.is_empty() {
                ui.separator();
                ui.heading("Suggestions:");
                ui.horizontal_wrapped(|ui| {
                    for (i, alt) in self.alternatives.iter().enumerate() {
                        if ui.button(&alt.word).clicked() {
                            self.selected_idx = Some(i);
                        }
                    }
                });
            }

            let _output = egui::TextEdit::multiline(&mut self.initial_text)
                .hint_text("Type something!")
                .desired_rows(10)
                .desired_width(f32::INFINITY)
                .show(ui);
        });

        let mut replace_action = None;
        if let Some(idx) = self.selected_idx {
            if let Some(alt) = self.alternatives.get(idx) {
                let mut is_open = true;
                egui::Window::new(format!("Alternatives for '{}'", alt.word))
                    .open(&mut is_open)
                    .collapsible(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        for word in &alt.alternatives {
                            if ui.button(word).clicked() {
                                replace_action = Some((idx, word.clone()));
                            }
                        }
                    });
                if !is_open {
                    self.selected_idx = None;
                }
            } else {
                self.selected_idx = None;
            }
        }

        if let Some((idx, new_word)) = replace_action {
            let alt = &self.alternatives[idx];
            self.initial_text = self.initial_text.replacen(&alt.word, &new_word, 1);
            self.alternatives.remove(idx);
            self.selected_idx = None;
        }
    }
}
