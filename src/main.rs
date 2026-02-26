use egui::{Color32, FontId, TextFormat};
use serde::{Deserialize, Serialize};
use std::{fs, sync::mpsc};

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
    provider_config: Option<ProviderConfig>,
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

    if provider_config.is_none() {
        eprintln!("No provider config provided, using defaults")
    }

    let schema: StructuredOutputFormat =
        serde_json::from_str(schema_text).map_err(|e| format!("Invalid JSON schema: {}", e))?;

    let provider_config = provider_config.unwrap_or_default();

    let llm = LLMBuilder::new()
        .backend(llm::builder::LLMBackend::OpenRouter)
        .api_key(provider_config.api_key)
        .model(provider_config.model)
        .temperature(provider_config.temperature)
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

#[derive(Debug, serde::Deserialize, Clone)]
struct AlternativeWord {
    word: String,
    alternatives: Vec<String>,
}

#[derive(PartialEq, Debug, Deserialize, Serialize, Clone)]
enum Provider {
    OpenRouter,
    Google,
    Groq,
    Antrophic,
    Custom(String),
}

impl Default for Provider {
    fn default() -> Self {
        Provider::OpenRouter
    }
}

#[derive(Default, Debug, Deserialize, Serialize, Clone)]
struct ProviderConfig {
    provider: Option<Provider>,
    api_key: String,
    model: String,
    temperature: f32,
}

impl ProviderConfig {
    fn load_from_file(&mut self) {
        let home_dir = dirs::home_dir().expect("Could not find home directory");
        let config_path = home_dir.join(".text_analyzer_config.json");

        if let Ok(config_str) = fs::read_to_string(&config_path) {
            if let Ok(config) = serde_json::from_str::<ProviderConfig>(&config_str) {
                *self = config;
            } else {
                eprintln!("Failed to parse config file, using defaults");
            }
        } else {
            eprintln!("No config file found, using defaults");
        }
    }

    fn save_to_file(&self) {
        let home_dir = dirs::home_dir().expect("Could not find home directory");
        let config_path = home_dir.join(".text_analyzer_config.json");

        if let Err(e) = fs::write(&config_path, serde_json::to_string_pretty(self).unwrap()) {
            eprintln!("Failed to save config: {}", e);
        } else {
            println!("Config saved to {:?}", config_path);
        }
    }
}

#[derive(Default, Deserialize)]
struct MyEguiApp {
    initial_text: String,
    alternatives: Vec<AlternativeWord>,
    error_message: Option<String>,
    options_menu_open: bool,
    options: ProviderConfig,
    // Channel to receive results from async task
    #[serde(skip)]
    result_receiver: Option<mpsc::Receiver<Result<Vec<AlternativeWord>, String>>>,
    #[serde(skip)]
    selected_idx: Option<usize>,
}

impl MyEguiApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::light());
        let mut this = Self::default();

        this.options.load_from_file();

        this
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
                    let options = self.options.clone();

                    let (tx, rx) = mpsc::channel();
                    self.result_receiver = Some(rx);

                    tokio::spawn(async move {
                        let result =
                            ask_ai_for_alternative_words(&initial_text, Some(options)).await;
                        let _ = tx.send(result.map_err(|e| e.to_string()));
                    });
                }

                let show_button = ui.button("Show Options");

                if show_button.clicked() {
                    self.options_menu_open = true;
                }

                if self.options_menu_open {
                    let mut is_open = true;
                    egui::Window::new("Options")
                        .open(&mut is_open)
                        .min_width(400.0)
                        .min_height(600.0)
                        .collapsible(false)
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .show(ctx, |ui| {
                            egui::ComboBox::from_label("Provider")
                                .selected_text(match &self.options.provider {
                                    Some(Provider::OpenRouter) => "OpenRouter",
                                    Some(Provider::Google) => "Google",
                                    Some(Provider::Groq) => "Groq",
                                    Some(Provider::Antrophic) => "Anthropic",
                                    Some(Provider::Custom(_)) => "Custom",
                                    None => "Select a provider",
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.options.provider,
                                        Some(Provider::OpenRouter),
                                        "OpenRouter",
                                    );
                                    ui.selectable_value(
                                        &mut self.options.provider,
                                        Some(Provider::Google),
                                        "Google",
                                    );
                                    ui.selectable_value(
                                        &mut self.options.provider,
                                        Some(Provider::Groq),
                                        "Groq",
                                    );
                                    ui.selectable_value(
                                        &mut self.options.provider,
                                        Some(Provider::Antrophic),
                                        "Anthropic",
                                    );
                                    ui.selectable_value(
                                        &mut self.options.provider,
                                        Some(Provider::Custom("Custom".to_string())),
                                        "Custom",
                                    );
                                });
                            ui.label("API Key:");
                            egui::TextEdit::singleline(&mut self.options.api_key)
                                .password(true)
                                .show(ui);
                            ui.label("Model:");
                            ui.text_edit_singleline(&mut self.options.model);
                            ui.label("Temperature:");
                            ui.add(
                                egui::Slider::new(&mut self.options.temperature, 0.0..=1.0)
                                    .text("Temperature"),
                            );

                            // how to detect on change of this

                            ui.button("Save").clicked().then(|| {
                                self.options.save_to_file();
                                self.options_menu_open = false;
                            });
                        });
                    if !is_open {
                        self.options_menu_open = false;
                    }
                }
            });

            // Show error if any
            if let Some(ref error) = self.error_message {
                ui.colored_label(egui::Color32::RED, error);
            }
            let mut layouter = |ui: &egui::Ui,
                                buf: &dyn egui::TextBuffer,
                                wrap_width: f32|
             -> std::sync::Arc<egui::Galley> {
                let content = &buf.as_str().to_string();

                if self.alternatives.is_empty() {
                    let layout_job = egui::text::LayoutJob::simple(
                        content.clone(),
                        FontId::proportional(16.0),
                        Color32::BLACK,
                        wrap_width,
                    );

                    let this = &ui;

                    let reader = |f: &mut egui::epaint::FontsView<'_>| f.layout_job(layout_job);
                    return this.ctx().fonts_mut(reader);
                }

                let mut cur_index = 0;
                let mut layout_job = egui::text::LayoutJob::default();
                layout_job.break_on_newline = true;
                layout_job.wrap = egui::text::TextWrapping {
                    max_width: wrap_width,
                    break_anywhere: false,
                    ..Default::default()
                };

                // delete duplicate alternatives for the same word, we will keep only the first one
                let mut seen_words = std::collections::HashSet::new();
                self.alternatives.retain(|alt| {
                    if seen_words.contains(&alt.word) {
                        false
                    } else {
                        seen_words.insert(alt.word.clone());
                        true
                    }
                });

                // if there are alternative, we will color the word with a different color
                for alt in &self.alternatives {
                    let word_start = content.find(&alt.word);
                    if word_start.is_none() {
                        continue;
                    }
                    let start_index = word_start.unwrap();
                    let end_index = start_index + alt.word.len();

                    layout_job.append(
                        &content[cur_index..start_index],
                        0.0,
                        TextFormat {
                            font_id: FontId::proportional(16.0),
                            color: Color32::BLACK,
                            ..Default::default()
                        },
                    );
                    layout_job.append(
                        &content[start_index..end_index],
                        0.0,
                        TextFormat {
                            font_id: FontId::proportional(16.0),
                            background: Color32::ORANGE,
                            ..Default::default()
                        },
                    );

                    cur_index = end_index;
                }

                let this = &ui;

                let reader = |f: &mut egui::epaint::FontsView<'_>| f.layout_job(layout_job);
                this.ctx().fonts_mut(reader)
            };
            let output = egui::TextEdit::multiline(&mut self.initial_text)
                .hint_text("Type something!")
                .desired_rows(30)
                .desired_width(f32::INFINITY)
                .layouter(&mut layouter)
                .show(ui);

            if output.response.clicked() {
                if let Some(cursor_range) = output.cursor_range {
                    let cursor_pos = cursor_range.primary.index; // this is the character offset (not the byte offset)
                    self.selected_idx = self
                        .alternatives
                        .iter()
                        .position(|alt| {
                            if let Some(pos) = self.initial_text.find(&alt.word) {
                                let word_start = pos;
                                let word_end = pos + alt.word.len();
                                cursor_pos >= word_start && cursor_pos <= word_end
                            } else {
                                false
                            }
                        })
                        .or(self.selected_idx);
                }
            }
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
