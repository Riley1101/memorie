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

// Global behavior: Ensures the AI only returns the edited text.

pub const INSERTION_GUARDRAILS: &str = r#"
Strict Rules for output text:
1. Output ONLY the modified text.
2. Do NOT include conversational filler, introductory remarks, or quotes.
3. Preserve existing Markdown (bold, italics, links).
4. If the text already meets the criteria, return it exactly as provided.
5. Always response just with the edited text, no explanations.
"#;

pub const EDIT_ACTION_BASE_PROMPT: &str = r#"
Role: AI assistant for writing improvements.\nTask: Perform the specified editing action on the provided text according to the given constraints.\nOutput: The fully edited text only, without any additional commentary or explanation.\n
Strict Rules for output text:
1. Output ONLY the modified text.
2. Do NOT include conversational filler, introductory remarks, or quotes.
3. Preserve existing Markdown (bold, italics, links).
4. If the text already meets the criteria, return it exactly as provided.
5. Always response just with the edited text, no explanations.
"#;

pub const PROMPT_EXPANSION_PROMPT: &str = r#""#;

pub const CORRECT_GRAMMAR_PROMPT: &str = r#"
Task: Fix grammar, spelling, and punctuation.
Constraints: Maintain original tone. Do not change style unless grammatically necessary.
Sentence:
"#;

pub const IMPROVE_CLARITY_PROMPT: &str = r#"
Task: Enhance readability and flow.
Constraints: Use active voice and concise phrasing. Preserve original meaning.
Input:
"#;

pub const MAKE_FORMAL_PROMPT: &str = r#"
Task: Rewrite with professional/academic tone.
Constraints: Remove slang and contractions. Suitable for business context.
Input:
"#;

pub const SIMPLIFY_PROMPT: &str = r#"
Task: Simplify text to a 6th-grade reading level.
Constraints: Short sentences, simple words, no jargon.
Input:
"#;

pub const RAG_CHAT_PROMPT: &str = r#"
You are an AI assistant that provides answers based on the provided CONTEXT. Use the context
to inform your responses, and if the answer is not found within the context, respond with "I don't know."
Take a look at the following context:\n\nCONTEXT:\n{context}\n\n
Now, answer the following question based on the above context:\n\nQUESTION:\n{query}
"#;
