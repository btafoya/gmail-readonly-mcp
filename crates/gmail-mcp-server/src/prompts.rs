//! Read-only email-analysis prompts.

use rmcp::model::{GetPromptResult, Prompt, PromptArgument, PromptMessage, Role};

/// The prompts this server exposes.
pub fn list_prompts() -> Vec<Prompt> {
    vec![
        Prompt::new(
            "analyze_email",
            Some("Analyze a single email message"),
            Some(vec![
                PromptArgument::new("account")
                    .with_description("Account alias")
                    .with_required(true),
                PromptArgument::new("message_id")
                    .with_description("Message application ID")
                    .with_required(true),
                PromptArgument::new("instructions")
                    .with_description("Additional analysis instructions"),
            ]),
        ),
        Prompt::new(
            "analyze_thread",
            Some("Analyze an email conversation/thread"),
            Some(vec![
                PromptArgument::new("account")
                    .with_description("Account alias")
                    .with_required(true),
                PromptArgument::new("thread_id")
                    .with_description("Thread application ID")
                    .with_required(true),
                PromptArgument::new("instructions")
                    .with_description("Additional analysis instructions"),
            ]),
        ),
    ]
}

/// Build a prompt result for a named prompt with the given arguments.
pub fn get_prompt(
    name: &str,
    args: &serde_json::Map<String, serde_json::Value>,
) -> Option<GetPromptResult> {
    let text = match name {
        "analyze_email" => {
            let account = args.get("account")?.as_str()?;
            let message_id = args.get("message_id")?.as_str()?;
            let instructions = args
                .get("instructions")
                .and_then(|v| v.as_str())
                .unwrap_or("Summarize the email, extract action items, decisions, requests, and important dates.");
            format!(
                "Analyze the email with message id `{message_id}` from account `{account}`.\n\
                 Use the get_message tool to retrieve it.\n\
                 Instructions: {instructions}\n\
                 Stay scoped to read-only email analysis."
            )
        }
        "analyze_thread" => {
            let account = args.get("account")?.as_str()?;
            let thread_id = args.get("thread_id")?.as_str()?;
            let instructions = args
                .get("instructions")
                .and_then(|v| v.as_str())
                .unwrap_or("Summarize the conversation, identify participants, decisions, unresolved questions, and next steps.");
            format!(
                "Analyze the conversation with thread id `{thread_id}` from account `{account}`.\n\
                 Use the get_thread tool to retrieve it.\n\
                 Instructions: {instructions}\n\
                 Stay scoped to read-only email analysis."
            )
        }
        _ => return None,
    };
    Some(GetPromptResult::new(vec![PromptMessage::new_text(
        Role::User,
        text,
    )]))
}
