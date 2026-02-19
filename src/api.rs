use std::thread;
use std::time::Duration;

use anyhow::Result;
use chrono::{DateTime, Utc};
use log::{error, warn};

use crate::models::ReaderResponse;

pub fn build_url(cursor: Option<&str>, updated_after: Option<&DateTime<Utc>>) -> String {
    let base = "https://readwise.io/api/v3/list/";
    let mut params: Vec<String> = Vec::new();

    if let Some(c) = cursor {
        params.push(format!("pageCursor={c}"));
    }
    if let Some(ts) = updated_after {
        params.push(format!("updatedAfter={}", ts.format("%Y-%m-%dT%H:%M:%SZ")));
    }

    if params.is_empty() {
        base.to_string()
    } else {
        format!("{}?{}", base, params.join("&"))
    }
}

pub fn get_reading(url: &str, access_token: &str) -> Result<ReaderResponse> {
    loop {
        match ureq::get(url)
            .set("Authorization", &format!("Token {access_token}"))
            .set("Content-Type", "application/json")
            .call()
        {
            Ok(response) => {
                let body = response.into_string()?;
                let jd = &mut serde_json::Deserializer::from_str(&body);
                let page: ReaderResponse = serde_path_to_error::deserialize(jd).map_err(|err| {
                    error!(
                        "Failed to deserialize API response at '{}': {err}. Raw body: {body}",
                        err.path()
                    );
                    err
                })?;
                return Ok(page);
            }
            Err(ureq::Error::Status(code, response)) if code == 429 || code >= 500 => {
                let retry_after: u64 = response
                    .header("Retry-After")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_else(|| {
                        warn!(
                            "Missing or unparsable Retry-After header for HTTP {code}. Defaulting to 60s."
                        );
                        60
                    });
                warn!("Received HTTP {code}, retrying after {retry_after}s");
                thread::sleep(Duration::from_secs(retry_after));
            }
            Err(ureq::Error::Status(code, _)) => {
                anyhow::bail!("Non-retryable HTTP error {code} from Readwise API");
            }
            Err(ureq::Error::Transport(e)) => {
                error!("Network transport error: {e}. Retrying in 30s.");
                thread::sleep(Duration::from_secs(30));
            }
        }
    }
}
