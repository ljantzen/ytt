pub mod chatgpt;
mod error;
mod parser;

pub use error::{Result, TranscriptError};
use parser::TranscriptParser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const WATCH_URL: &str = "https://www.youtube.com/watch?v={video_id}";
const INNERTUBE_API_URL: &str = "https://www.youtube.com/youtubei/v1/player?key={api_key}";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptItem {
    pub text: String,
    pub start: f64,
    pub duration: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptResponse {
    pub video_id: String,
    pub language: String,
    pub language_code: String,
    pub is_generated: bool,
    pub is_translatable: bool,
    pub transcript: Vec<TranscriptItem>,
}

#[derive(Debug, Clone)]
pub struct TranscriptInfo {
    pub language_code: String,
    pub language: String,
    pub is_generated: bool,
    pub is_translatable: bool,
    pub base_url: String,
    pub translation_languages: Vec<TranslationLanguage>,
}

#[derive(Debug, Clone)]
pub struct TranslationLanguage {
    pub language: String,
    pub language_code: String,
}

pub struct TranscriptList {
    pub video_id: String,
    pub manually_created: HashMap<String, TranscriptInfo>,
    pub generated: HashMap<String, TranscriptInfo>,
    pub translation_languages: Vec<TranslationLanguage>,
}

impl TranscriptList {
    pub fn find_transcript(&self, language_codes: &[&str]) -> Result<&TranscriptInfo> {
        // Try manually created first, then generated
        for lang_code in language_codes {
            if let Some(transcript) = self.manually_created.get(*lang_code) {
                return Ok(transcript);
            }
            if let Some(transcript) = self.generated.get(*lang_code) {
                return Ok(transcript);
            }
        }
        Err(TranscriptError::NoTranscriptFound(
            self.video_id.clone(),
            language_codes.iter().map(|s| s.to_string()).collect(),
        ))
    }

    pub fn find_manually_created(&self, language_codes: &[&str]) -> Result<&TranscriptInfo> {
        for lang_code in language_codes {
            if let Some(transcript) = self.manually_created.get(*lang_code) {
                return Ok(transcript);
            }
        }
        Err(TranscriptError::NoTranscriptFound(
            self.video_id.clone(),
            language_codes.iter().map(|s| s.to_string()).collect(),
        ))
    }

    pub fn find_generated(&self, language_codes: &[&str]) -> Result<&TranscriptInfo> {
        for lang_code in language_codes {
            if let Some(transcript) = self.generated.get(*lang_code) {
                return Ok(transcript);
            }
        }
        Err(TranscriptError::NoTranscriptFound(
            self.video_id.clone(),
            language_codes.iter().map(|s| s.to_string()).collect(),
        ))
    }

    pub fn all_transcripts(&self) -> Vec<&TranscriptInfo> {
        let mut all: Vec<&TranscriptInfo> = self.manually_created.values().collect();
        all.extend(self.generated.values());
        all
    }
}

pub struct YouTubeTranscript {
    client: reqwest::Client,
    delay_ms: u64,
}

impl Default for YouTubeTranscript {
    fn default() -> Self {
        Self::new()
    }
}

impl YouTubeTranscript {
    pub fn new() -> Self {
        Self::with_delay(500) // Default 500ms delay
    }

    pub fn with_delay(delay_ms: u64) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::ACCEPT_LANGUAGE,
            reqwest::header::HeaderValue::from_static("en-US"),
        );

        Self {
            client: reqwest::Client::builder()
                .cookie_store(true)
                .default_headers(headers)
                .build()
                .expect("Failed to create HTTP client"),
            delay_ms,
        }
    }

    async fn delay(&self) {
        tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
    }

    /// Extract video ID from YouTube URL
    pub fn extract_video_id(url_or_id: &str) -> Result<String> {
        let input = url_or_id.trim();
        
        // Check if it's already a video ID (11 characters)
        if input.len() == 11
            && input
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Ok(input.to_string());
        }

        // Try parsing as URL (with or without protocol)
        let url_str = if input.starts_with("http://") || input.starts_with("https://") {
            input.to_string()
        } else if input.contains("youtube.com") || input.contains("youtu.be") {
            format!("https://{}", input)
        } else {
            input.to_string()
        };

        let url = match url::Url::parse(&url_str) {
            Ok(u) => u,
            Err(_) => {
                // If it's not a valid URL and not 11 chars, it's invalid
                return Err(TranscriptError::InvalidVideoId(format!(
                    "{} (YouTube video IDs must be 11 characters, or a valid YouTube URL)",
                    url_or_id
                )));
            }
        };

        if url
            .host_str()
            .map(|h| h.contains("youtube.com") || h.contains("youtu.be"))
            .unwrap_or(false)
        {
            // Standard watch URL: ?v=VIDEO_ID
            if let Some(video_id) = url
                .query_pairs()
                .find(|(k, _)| k == "v")
                .map(|(_, v)| v.to_string())
            {
                // Validate extracted ID is 11 characters
                if video_id.len() == 11
                    && video_id
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                {
                    return Ok(video_id);
                }
            }
            // Short URL: youtu.be/VIDEO_ID
            if url.host_str().map(|h| h == "youtu.be").unwrap_or(false) {
                if let Some(video_id) = url.path_segments().and_then(|mut s| s.next()) {
                    // Validate extracted ID is 11 characters
                    if video_id.len() == 11
                        && video_id
                            .chars()
                            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                    {
                        return Ok(video_id.to_string());
                    }
                }
            }
            // Embed URL: youtube.com/embed/VIDEO_ID
            if let Some(segments) = url.path_segments() {
                let segments: Vec<&str> = segments.collect();
                if segments.len() >= 2 && segments[0] == "embed" {
                    let video_id = segments[1];
                    if video_id.len() == 11
                        && video_id
                            .chars()
                            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                    {
                        return Ok(video_id.to_string());
                    }
                }
            }
        }

        Err(TranscriptError::InvalidVideoId(format!(
            "{} (YouTube video IDs must be 11 characters, or a valid YouTube URL)",
            url_or_id
        )))
    }

    /// List all available transcripts for a video
    pub async fn list_transcripts(&self, video_id: &str) -> Result<TranscriptList> {
        let html = self.fetch_video_html(video_id).await?;
        // Delay between HTML fetch and API call to avoid rate limiting
        self.delay().await;
        let api_key = self.extract_innertube_api_key(&html, video_id)?;
        let innertube_data = self.fetch_innertube_data(video_id, &api_key).await?;
        self.extract_captions_json(video_id, &innertube_data)
    }

    /// Fetch transcript for a specific language
    pub async fn fetch_transcript(
        &self,
        video_id: &str,
        languages: Option<Vec<&str>>,
    ) -> Result<TranscriptResponse> {
        let transcript_list = self.list_transcripts(video_id).await?;

        let languages = languages.unwrap_or_else(|| vec!["en"]);
        let transcript_info = transcript_list.find_transcript(&languages)?;

        self.fetch_transcript_data(video_id, transcript_info, None)
            .await
    }

    /// Translate a transcript to another language
    pub async fn translate_transcript(
        &self,
        video_id: &str,
        source_languages: &[&str],
        target_language: &str,
    ) -> Result<TranscriptResponse> {
        let transcript_list = self.list_transcripts(video_id).await?;
        let source_transcript = transcript_list.find_transcript(source_languages)?;

        if !source_transcript.is_translatable {
            return Err(TranscriptError::NotTranslatable(video_id.to_string()));
        }

        let translation_exists = source_transcript
            .translation_languages
            .iter()
            .any(|t| t.language_code == target_language);

        if !translation_exists {
            return Err(TranscriptError::TranslationLanguageNotAvailable(
                target_language.to_string(),
            ));
        }

        self.fetch_transcript_data(video_id, source_transcript, Some(target_language))
            .await
    }

    async fn fetch_video_html(&self, video_id: &str) -> Result<String> {
        // Add initial delay to avoid rate limiting
        self.delay().await;

        let url = WATCH_URL.replace("{video_id}", video_id);
        let mut response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| TranscriptError::HttpError(format!("Failed to fetch HTML: {}", e)))?;

        self.check_http_errors(&response, video_id)?;

        let html = response
            .text()
            .await
            .map_err(|e| TranscriptError::HttpError(format!("Failed to read HTML: {}", e)))?;

        // Handle consent cookie if needed
        if html.contains("action=\"https://consent.youtube.com/s\"") {
            self.create_consent_cookie(&html, video_id)?;
            // Add delay before retry
            self.delay().await;
            // Retry request
            response = self.client.get(&url).send().await.map_err(|e| {
                TranscriptError::HttpError(format!("Failed to fetch HTML after consent: {}", e))
            })?;

            self.check_http_errors(&response, video_id)?;

            let html = response
                .text()
                .await
                .map_err(|e| TranscriptError::HttpError(format!("Failed to read HTML: {}", e)))?;

            if html.contains("action=\"https://consent.youtube.com/s\"") {
                return Err(TranscriptError::FailedToCreateConsentCookie(
                    video_id.to_string(),
                ));
            }

            return Ok(html);
        }

        Ok(html)
    }

    fn extract_innertube_api_key(&self, html: &str, video_id: &str) -> Result<String> {
        use regex::Regex;

        // Check for bot detection first
        if html.contains("class=\"g-recaptcha\"") || html.contains("g-recaptcha") {
            return Err(TranscriptError::IpBlocked(video_id.to_string()));
        }

        let re = Regex::new(r#""INNERTUBE_API_KEY":\s*"([a-zA-Z0-9_-]+)""#)
            .map_err(|_| TranscriptError::YouTubeDataUnparsable(video_id.to_string()))?;

        if let Some(captures) = re.captures(html) {
            if let Some(api_key) = captures.get(1) {
                return Ok(api_key.as_str().to_string());
            }
        }

        Err(TranscriptError::YouTubeDataUnparsable(video_id.to_string()))
    }

    async fn fetch_innertube_data(
        &self,
        video_id: &str,
        api_key: &str,
    ) -> Result<serde_json::Value> {
        let url = INNERTUBE_API_URL.replace("{api_key}", api_key);

        let context = serde_json::json!({
            "context": {
                "client": {
                    "clientName": "ANDROID",
                    "clientVersion": "20.10.38"
                }
            },
            "videoId": video_id
        });

        // Add delay before API request to avoid rate limiting
        self.delay().await;

        let response = self
            .client
            .post(&url)
            .json(&context)
            .send()
            .await
            .map_err(|e| {
                TranscriptError::HttpError(format!("Failed to fetch InnerTube data: {}", e))
            })?;

        self.check_http_errors(&response, video_id)?;

        let data: serde_json::Value = response.json().await.map_err(|e| {
            TranscriptError::JsonParseError(format!("Failed to parse InnerTube response: {}", e))
        })?;

        Ok(data)
    }

    fn extract_captions_json(
        &self,
        video_id: &str,
        innertube_data: &serde_json::Value,
    ) -> Result<TranscriptList> {
        // Check playability status
        self.assert_playability(video_id, innertube_data)?;

        let captions_json = innertube_data
            .get("captions")
            .and_then(|c| c.get("playerCaptionsTracklistRenderer"));

        if captions_json.is_none() {
            return Err(TranscriptError::TranscriptsDisabled(video_id.to_string()));
        }

        let captions_json = captions_json.unwrap();

        // Extract translation languages
        let translation_languages: Vec<TranslationLanguage> = captions_json
            .get("translationLanguages")
            .and_then(|tl| tl.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|lang| {
                        let language_code = lang.get("languageCode")?.as_str()?.to_string();
                        let language = lang
                            .get("languageName")?
                            .get("runs")?
                            .as_array()?
                            .first()?
                            .get("text")?
                            .as_str()?
                            .to_string();
                        Some(TranslationLanguage {
                            language_code,
                            language,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let mut manually_created = HashMap::new();
        let mut generated = HashMap::new();

        if let Some(caption_tracks) = captions_json
            .get("captionTracks")
            .and_then(|ct| ct.as_array())
        {
            for caption in caption_tracks {
                let language_code = match caption.get("languageCode").and_then(|l| l.as_str()) {
                    Some(lc) => lc.to_string(),
                    None => continue,
                };

                let base_url = match caption.get("baseUrl").and_then(|u| u.as_str()) {
                    Some(url) => url.replace("&fmt=srv3", ""),
                    None => continue,
                };

                let language = caption
                    .get("name")
                    .and_then(|n| n.get("runs"))
                    .and_then(|r| r.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|r| r.get("text"))
                    .and_then(|t| t.as_str())
                    .unwrap_or(&language_code)
                    .to_string();

                let is_generated = caption
                    .get("kind")
                    .and_then(|k| k.as_str())
                    .map(|k| k == "asr")
                    .unwrap_or(false);

                let is_translatable = caption
                    .get("isTranslatable")
                    .and_then(|t| t.as_bool())
                    .unwrap_or(false);

                let transcript_translation_languages = if is_translatable {
                    translation_languages.clone()
                } else {
                    Vec::new()
                };

                let transcript_info = TranscriptInfo {
                    language_code: language_code.clone(),
                    language,
                    is_generated,
                    is_translatable,
                    base_url,
                    translation_languages: transcript_translation_languages,
                };

                if is_generated {
                    generated.insert(language_code, transcript_info);
                } else {
                    manually_created.insert(language_code, transcript_info);
                }
            }
        }

        if manually_created.is_empty() && generated.is_empty() {
            return Err(TranscriptError::TranscriptsDisabled(video_id.to_string()));
        }

        Ok(TranscriptList {
            video_id: video_id.to_string(),
            manually_created,
            generated,
            translation_languages,
        })
    }

    fn assert_playability(&self, video_id: &str, innertube_data: &serde_json::Value) -> Result<()> {
        let playability_status = match innertube_data.get("playabilityStatus") {
            Some(ps) => ps,
            None => return Ok(()),
        };

        let status = playability_status
            .get("status")
            .and_then(|s| s.as_str())
            .unwrap_or("");

        if status == "OK" {
            return Ok(());
        }

        let reason = playability_status
            .get("reason")
            .and_then(|r| r.as_str())
            .unwrap_or("");

        match status {
            "LOGIN_REQUIRED" => {
                if reason.contains("Sign in to confirm you're not a bot") {
                    return Err(TranscriptError::RequestBlocked(video_id.to_string()));
                }
                if reason.contains("inappropriate for some users") {
                    return Err(TranscriptError::AgeRestricted(video_id.to_string()));
                }
            }
            "ERROR" => {
                if reason.contains("unavailable") {
                    if video_id.starts_with("http://") || video_id.starts_with("https://") {
                        return Err(TranscriptError::InvalidVideoId(video_id.to_string()));
                    }
                    return Err(TranscriptError::VideoUnavailable(video_id.to_string()));
                }
            }
            _ => {}
        }

        Err(TranscriptError::VideoUnplayable(
            video_id.to_string(),
            reason.to_string(),
        ))
    }

    fn create_consent_cookie(&self, html: &str, video_id: &str) -> Result<()> {
        use regex::Regex;
        let re = Regex::new(r#"name="v" value="(.*?)""#)
            .map_err(|_| TranscriptError::FailedToCreateConsentCookie(video_id.to_string()))?;

        if let Some(captures) = re.captures(html) {
            if let Some(value) = captures.get(1) {
                let _cookie_value = format!("YES+{}", value.as_str());
                // Note: reqwest handles cookies automatically via cookie_store
                // We would need to manually set cookies if needed, but for now
                // the retry after detecting consent should work
                return Ok(());
            }
        }

        Err(TranscriptError::FailedToCreateConsentCookie(
            video_id.to_string(),
        ))
    }

    fn check_http_errors(&self, response: &reqwest::Response, video_id: &str) -> Result<()> {
        if response.status() == 429 {
            return Err(TranscriptError::IpBlocked(video_id.to_string()));
        }
        if !response.status().is_success() {
            return Err(TranscriptError::HttpError(format!(
                "HTTP {}: {}",
                response.status(),
                response
                    .status()
                    .canonical_reason()
                    .unwrap_or("Unknown error")
            )));
        }
        Ok(())
    }

    async fn fetch_transcript_data(
        &self,
        video_id: &str,
        transcript_info: &TranscriptInfo,
        translate_to: Option<&str>,
    ) -> Result<TranscriptResponse> {
        let mut url = transcript_info.base_url.clone();

        if let Some(target_lang) = translate_to {
            url = format!("{}&tlang={}", url, target_lang);
        }

        // Check for protected video token requirement
        if url.contains("&exp=xpe") {
            return Err(TranscriptError::PoTokenRequired(video_id.to_string()));
        }

        // Add delay before fetching transcript to avoid rate limiting
        self.delay().await;

        let response = self.client.get(&url).send().await.map_err(|e| {
            TranscriptError::HttpError(format!("Failed to fetch transcript: {}", e))
        })?;

        self.check_http_errors(&response, video_id)?;

        let xml_content = response
            .text()
            .await
            .map_err(|e| TranscriptError::HttpError(format!("Failed to read transcript: {}", e)))?;

        let parser = TranscriptParser::new(false);
        let transcript_items = parser
            .parse(&xml_content)
            .map_err(|e| TranscriptError::XmlParseError(format!("Failed to parse XML: {}", e)))?;

        let language = if let Some(target_lang) = translate_to {
            transcript_info
                .translation_languages
                .iter()
                .find(|t| t.language_code == target_lang)
                .map(|t| t.language.clone())
                .unwrap_or_else(|| transcript_info.language.clone())
        } else {
            transcript_info.language.clone()
        };

        Ok(TranscriptResponse {
            video_id: video_id.to_string(),
            language,
            language_code: translate_to
                .unwrap_or(&transcript_info.language_code)
                .to_string(),
            is_generated: transcript_info.is_generated || translate_to.is_some(),
            is_translatable: transcript_info.is_translatable,
            transcript: transcript_items,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_video_id_direct() {
        assert_eq!(
            YouTubeTranscript::extract_video_id("dQw4w9WgXcQ").unwrap(),
            "dQw4w9WgXcQ"
        );
    }

    #[test]
    fn test_extract_video_id_watch_url() {
        assert_eq!(
            YouTubeTranscript::extract_video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ")
                .unwrap(),
            "dQw4w9WgXcQ"
        );
    }

    #[test]
    fn test_extract_video_id_short_url() {
        assert_eq!(
            YouTubeTranscript::extract_video_id("https://youtu.be/dQw4w9WgXcQ").unwrap(),
            "dQw4w9WgXcQ"
        );
    }

    #[test]
    fn test_extract_video_id_short_url_with_query() {
        assert_eq!(
            YouTubeTranscript::extract_video_id("https://youtu.be/_NuH3D4SN-c?si=VSFea_rMwtaiR8Q7").unwrap(),
            "_NuH3D4SN-c"
        );
    }

    #[test]
    fn test_extract_video_id_invalid() {
        assert!(YouTubeTranscript::extract_video_id("not-a-valid-id").is_err());
        assert!(YouTubeTranscript::extract_video_id("https://example.com").is_err());
    }

    #[test]
    fn test_transcript_list_find_transcript() {
        let mut manually_created = HashMap::new();
        let mut generated = HashMap::new();

        manually_created.insert(
            "en".to_string(),
            TranscriptInfo {
                language_code: "en".to_string(),
                language: "English".to_string(),
                is_generated: false,
                is_translatable: true,
                base_url: "https://example.com/en".to_string(),
                translation_languages: vec![],
            },
        );

        generated.insert(
            "es".to_string(),
            TranscriptInfo {
                language_code: "es".to_string(),
                language: "Spanish".to_string(),
                is_generated: true,
                is_translatable: false,
                base_url: "https://example.com/es".to_string(),
                translation_languages: vec![],
            },
        );

        let list = TranscriptList {
            video_id: "test".to_string(),
            manually_created,
            generated,
            translation_languages: vec![],
        };

        // Should find manually created first
        assert_eq!(list.find_transcript(&["en"]).unwrap().language_code, "en");
        // Should find generated if manually created not available
        assert_eq!(list.find_transcript(&["es"]).unwrap().language_code, "es");
        // Should prefer manually created over generated
        assert_eq!(list.find_transcript(&["en", "es"]).unwrap().language_code, "en");
        // Should error if not found
        assert!(list.find_transcript(&["fr"]).is_err());
    }

    #[test]
    fn test_transcript_list_find_manually_created() {
        let mut manually_created = HashMap::new();
        manually_created.insert(
            "en".to_string(),
            TranscriptInfo {
                language_code: "en".to_string(),
                language: "English".to_string(),
                is_generated: false,
                is_translatable: true,
                base_url: "https://example.com/en".to_string(),
                translation_languages: vec![],
            },
        );

        let list = TranscriptList {
            video_id: "test".to_string(),
            manually_created,
            generated: HashMap::new(),
            translation_languages: vec![],
        };

        assert_eq!(
            list.find_manually_created(&["en"]).unwrap().language_code,
            "en"
        );
        assert!(list.find_manually_created(&["es"]).is_err());
    }

    #[test]
    fn test_transcript_list_find_generated() {
        let mut generated = HashMap::new();
        generated.insert(
            "es".to_string(),
            TranscriptInfo {
                language_code: "es".to_string(),
                language: "Spanish".to_string(),
                is_generated: true,
                is_translatable: false,
                base_url: "https://example.com/es".to_string(),
                translation_languages: vec![],
            },
        );

        let list = TranscriptList {
            video_id: "test".to_string(),
            manually_created: HashMap::new(),
            generated,
            translation_languages: vec![],
        };

        assert_eq!(list.find_generated(&["es"]).unwrap().language_code, "es");
        assert!(list.find_generated(&["en"]).is_err());
    }

    #[test]
    fn test_youtube_transcript_default() {
        let api = YouTubeTranscript::default();
        assert_eq!(api.delay_ms, 500);
    }

    #[test]
    fn test_youtube_transcript_with_delay() {
        let api = YouTubeTranscript::with_delay(1000);
        assert_eq!(api.delay_ms, 1000);
    }
}
