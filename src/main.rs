#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::struct_excessive_bools)]

mod client;
mod protocol;
mod types;

use client::ExaClient;
use protocol::{format_results, Input, Output};
use std::io::{self, Read};
use types::{
    params::ContentsInput, ContentsOptions, FindSimilarOptions, GetContentsOptions, SearchOptions,
    TextOptions,
};

#[tokio::main]
async fn main() {
    // ── Read API key ─────────────────────────────────────────────────────────
    let api_key = match std::env::var("EXA_API_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => return emit_err("EXA_API_KEY environment variable not set"),
    };

    // ── Read stdin ───────────────────────────────────────────────────────────
    let mut buf = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut buf) {
        return emit_err(&format!("Failed to read stdin: {e}"));
    }

    // ── Parse input ──────────────────────────────────────────────────────────
    let input: Input = match serde_json::from_str(&buf) {
        Ok(i) => i,
        Err(e) => return emit_err(&format!("Invalid JSON input: {e}")),
    };

    // ── Build client ─────────────────────────────────────────────────────────
    let client = match ExaClient::new(api_key) {
        Ok(c) => c,
        Err(e) => return emit_err(&format!("Failed to create HTTP client: {e}")),
    };

    // ── Dispatch ─────────────────────────────────────────────────────────────
    let action = input.action.as_deref().unwrap_or("search");

    match action {
        "search" => handle_search(client, input).await,
        "find_similar" => handle_find_similar(client, input).await,
        "get_contents" => handle_get_contents(client, input).await,
        other => emit_err(&format!("Unknown action: {other}")),
    }
}

// ── Search handler ────────────────────────────────────────────────────────────

async fn handle_search(client: ExaClient, input: Input) {
    let Some(query) = input.query else {
        return emit_err("'query' is required for action 'search'");
    };

    // Resolve contents options
    let contents = Some(resolve_contents(input.contents.as_ref(), input.max_chars));

    let opts = SearchOptions {
        query,
        num_results: input.num_results,
        search_type: input.search_type,
        category: input.category,
        include_domains: input.include_domains,
        exclude_domains: input.exclude_domains,
        start_crawl_date: input.start_crawl_date,
        end_crawl_date: input.end_crawl_date,
        start_published_date: input.start_published_date,
        end_published_date: input.end_published_date,
        include_text: input.include_text,
        exclude_text: input.exclude_text,
        use_autoprompt: input.use_autoprompt,
        moderation: input.moderation,
        user_location: input.user_location,
        additional_queries: input.additional_queries,
        contents,
    };

    match client.search(opts).await {
        Ok(resp) => {
            let formatted = format_results(&resp.results);
            emit_ok(&Output::SearchOk {
                ok: true,
                action: "search".to_string(),
                results: resp.results,
                resolved_search_type: resp.resolved_search_type,
                auto_date: resp.auto_date,
                search_time_ms: resp.search_time_ms,
                cost_dollars: resp.cost_dollars,
                formatted,
            });
        }
        Err(e) => emit_err(&format!("Search failed: {e}")),
    }
}

// ── FindSimilar handler ───────────────────────────────────────────────────────

async fn handle_find_similar(client: ExaClient, input: Input) {
    let Some(url) = input.url else {
        return emit_err("'url' is required for action 'find_similar'");
    };

    let contents = Some(resolve_contents(input.contents.as_ref(), input.max_chars));

    let opts = FindSimilarOptions {
        url,
        num_results: input.num_results,
        include_domains: input.include_domains,
        exclude_domains: input.exclude_domains,
        start_crawl_date: input.start_crawl_date,
        end_crawl_date: input.end_crawl_date,
        start_published_date: input.start_published_date,
        end_published_date: input.end_published_date,
        include_text: input.include_text,
        exclude_text: input.exclude_text,
        exclude_source_domain: input.exclude_source_domain,
        category: input.category,
        contents,
    };

    match client.find_similar(opts).await {
        Ok(resp) => {
            let formatted = format_results(&resp.results);
            emit_ok(&Output::SearchOk {
                ok: true,
                action: "find_similar".to_string(),
                results: resp.results,
                resolved_search_type: None,
                auto_date: None,
                search_time_ms: None,
                cost_dollars: resp.cost_dollars,
                formatted,
            });
        }
        Err(e) => emit_err(&format!("FindSimilar failed: {e}")),
    }
}

// ── GetContents handler ───────────────────────────────────────────────────────

async fn handle_get_contents(client: ExaClient, input: Input) {
    let urls = match input.urls {
        Some(u) if !u.is_empty() => u,
        _ => return emit_err("'urls' (non-empty array) is required for action 'get_contents'"),
    };

    // For get_contents, contents options are passed as top-level fields
    let resolved = input
        .contents
        .map(ContentsInput::into_options)
        .unwrap_or_default();

    // Apply legacy max_chars to text if not already set
    let text = resolved.text.or_else(|| {
        input.max_chars.map(|mc| TextOptions {
            max_characters: Some(mc),
            ..Default::default()
        })
    });

    let opts = GetContentsOptions {
        urls,
        text,
        summary: resolved.summary,
        highlights: resolved.highlights,
        livecrawl: resolved.livecrawl,
        livecrawl_timeout: resolved.livecrawl_timeout,
        max_age_hours: resolved.max_age_hours,
        filter_empty_results: resolved.filter_empty_results,
        subpages: resolved.subpages,
        subpage_target: resolved.subpage_target,
        extras: resolved.extras,
    };

    match client.get_contents(opts).await {
        Ok(resp) => {
            emit_ok(&Output::ContentsOk {
                ok: true,
                action: "get_contents".to_string(),
                results: resp.results,
                cost_dollars: resp.cost_dollars,
            });
        }
        Err(e) => emit_err(&format!("GetContents failed: {e}")),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Resolve contents from protocol input + legacy `max_chars`.
/// If neither is provided, defaults to text with `max_characters=10000`
/// (preserves backwards-compat with old behaviour).
fn resolve_contents(
    contents_input: Option<&ContentsInput>,
    max_chars: Option<u32>,
) -> ContentsOptions {
    if let Some(ci) = contents_input {
        let mut opts = ci.clone().into_options();
        // Apply legacy max_chars if text is enabled with no max_characters set
        if let Some(mc) = max_chars {
            if let Some(ref mut text) = opts.text {
                if text.max_characters.is_none() {
                    text.max_characters = Some(mc);
                }
            }
        }
        opts
    } else {
        // Legacy default: always request text
        let max_characters = max_chars.or(Some(10_000));
        ContentsOptions {
            text: Some(TextOptions {
                max_characters,
                ..Default::default()
            }),
            ..Default::default()
        }
    }
}

fn emit_ok(output: &Output) {
    match serde_json::to_string(output) {
        Ok(s) => println!("{s}"),
        Err(e) => eprintln!("Fatal: failed to serialize output: {e}"),
    }
}

fn emit_err(msg: &str) {
    let output = Output::Err {
        ok: false,
        error: msg.to_string(),
    };
    match serde_json::to_string(&output) {
        Ok(s) => println!("{s}"),
        Err(e) => eprintln!("Fatal: failed to serialize error: {e}"),
    }
}
