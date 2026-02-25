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
    let system_prompt = "You are a helpful assistant that provides alternative words for a given text, you will try to enhance or offer more appropriate suggestions for the words in the text, You will return a list fo word for the one you think will need changes and for each of this world you will give a list of alternatives, you will retrun the result in a json format, providing word,start_position,end_position and alternatives";
    let schema_text = r#"
        {
            "name": "AlternativeWordsResponse",
            "schema": {
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

    let schema: StructuredOutputFormat = serde_json::from_str(schema_text)?;
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .expect("OPENROUTER_API_KEY environment variable not set");
    let llm = LLMBuilder::new()
        .backend(llm::builder::LLMBackend::OpenRouter)
        .api_key(api_key)
        .model("google/gemini-2.5-flash")
        .max_tokens(1024)
        .temperature(0.7)
        .system(system_prompt)
        .schema(schema)
        .build()
        .expect("Failed to build LLM");

    let messages = vec![
        ChatMessage::user()
            .content("Here is the text I want you to analyze:".to_string() + text)
            .build(),
    ];

    match llm.chat(&messages).await {
        Ok(text) => match serde_json::from_str::<Vec<AlternativeWord>>(&text) {
            Ok(alternative_words) => Ok(alternative_words),
            Err(e) => Err(Box::new(e)),
        },
        Err(e) => eprintln!("Chat error: {e}"),
    }
}

#[derive(Debug)]
struct AlternativeWord {
    word: String,
    start_position: usize,
    end_position: usize,
    alternatives: Vec<String>,
}

#[derive(Default)]
struct MyEguiApp {
    initial_text: String,
}

impl MyEguiApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::light());
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        Self::default()
    }
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let button_accept = ui.button("Analyze text");

            if button_accept.clicked() {
                let initial_text = self.initial_text.clone();
                tokio::spawn(async move {
                    match ask_ai_for_alternative_words(&initial_text).await {
                        Ok(words) => {
                            // TODO: Handle the successful result (e.g., update self.words)
                            println!("Successfully got alternative words: {:?}", words);
                        }
                        Err(e) => {
                            // TODO: Handle the error
                            eprintln!("Error getting alternative words: {:?}", e);
                        }
                    }
                });
            };
            let _output = egui::TextEdit::multiline(&mut self.initial_text)
                .hint_text("Type something!")
                .desired_rows(54)
                .desired_width(f32::INFINITY)
                .show(ui);
        });
    }
}
