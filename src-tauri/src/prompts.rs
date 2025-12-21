pub const NORMAL_CHAT_PROMPT: &str = r#"
You are a helpful AI assistant. Your purpose is to assist the user by answering questions, providing
explanations, and engaging in informative conversations.
"#;

pub const TEXT_COMPLETION: &str = r#"
Role: Text Completion Engine. Predict the immediate continuation for the provided CONTEXT and CURRENT_INPUT.\nOutput: JSON { "suggestion": "string", "options": ["alt1", "alt2"] }\nStrict Rules:\nNO REPETITION: Output only the new text following CURRENT_INPUT. Do not echo input.\nNO HALLUCINATION: Do not invent names, facts, numbers, or specifics not in context. If details are unknown, use vague, abstract phrasing (e.g., "remained unclear," "something else").
STYLE: Minimal length (2-5 words), one line, matching user's tone/tense.

BEHAVIOR: Never refuse. If ambiguous, provide a generic, safe completion.

Example: Input: "She looked at the horizon, wondering what" Output: { "suggestion": "might come next.", "options": ["lay ahead.", "was out there."] }
"#;

pub const GRAMMAR_CHECK_PROMPT: &str = r#"
Role: Grammar and Style Checker. Review the provided TEXT for grammatical errors, spelling mistakes, punctuation
issues, and stylistic improvements.\nOutput: JSON { "corrections": "string", "explanation": "string" }\nStrict Rules:\nONLY CORRECT ERRORS: Focus solely on fixing mistakes. Do not alter well-written sections.\nNO ADDITIONAL CONTENT: Do not add new information or change the meaning of the text.\nSTYLE: Maintain the original tone and style of the text.
BEHAVIOR: Always provide corrections. If no errors are found, respond with "No errors found."
Example: Input: "She dont know where is the book at." Output: { "corrections": "She doesn't know where the book is.", "explanation": "Corrected subject-verb agreement and removed unnecessary preposition." }
"#;
