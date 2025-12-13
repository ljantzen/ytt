# ytt - YouTube Transcript API (Rust)

A Rust implementation of the YouTube Transcript API, similar to the Python [youtube-transcript-api](https://pypi.org/project/youtube-transcript-api/) package.

## Features

- Fetch transcripts/captions from YouTube videos using InnerTube API
- Support for multiple languages with priority fallback
- Handle both manually created and auto-generated transcripts (prioritizes manual)
- Multiple output formats: JSON, text, TXT, SRT, Markdown
- Extract video ID from various YouTube URL formats
- Translation support for translatable transcripts
- Proper XML parsing with quick-xml
- Consent cookie handling for GDPR compliance
- Playability status checking
- List available transcripts for a video
- ChatGPT cleanup integration for improved transcripts
- Configurable request delays to avoid rate limiting
- File output support

## Installation

### Pre-built Binaries

Download pre-built executables from the [Releases](https://github.com/ljantzen/ytt/releases) page:

- **Linux**: `ytt-linux-x86_64.tar.gz` (GNU) or `ytt-linux-x86_64-musl.tar.gz` (musl)
- **macOS**: `ytt-macos-x86_64.tar.gz` (Intel) or `ytt-macos-arm64.tar.gz` (Apple Silicon)
- **Windows**: `ytt-windows-x86_64.exe.zip`

Extract and add to your PATH, or use directly.

### From Source

```bash
git clone https://github.com/ljantzen/ytt.git
cd ytt
cargo build --release
```

The binary will be available at `target/release/ytt` (or `target/release/ytt.exe` on Windows).

### Requirements

- Rust 1.70+ (edition 2021) - only needed for building from source
- Internet connection for fetching transcripts

## Quick Start

```bash
# Fetch transcript (default: plain text)
ytt mcbwS5Owclo

# Save to file
ytt mcbwS5Owclo -o transcript.txt

# List available transcripts
ytt mcbwS5Owclo --list

# Get JSON output
ytt mcbwS5Owclo -f json -o transcript.json

# Get Markdown output
ytt mcbwS5Owclo -f markdown -o transcript.md
```

## Usage

### Basic Commands

```bash
# Fetch transcript using video URL
ytt "https://www.youtube.com/watch?v=dQw4w9WgXcQ"

# Fetch transcript using video ID
ytt dQw4w9WgXcQ

# Specify language
ytt "https://www.youtube.com/watch?v=dQw4w9WgXcQ" --languages en

# List available transcripts
ytt dQw4w9WgXcQ --list

# Translate transcript
ytt dQw4w9WgXcQ --languages es --translate en

# Output as JSON
ytt dQw4w9WgXcQ --format json

# Output as SRT (subtitle format)
ytt dQw4w9WgXcQ --format srt -o subtitles.srt

# Output as Markdown
ytt dQw4w9WgXcQ --format markdown -o transcript.md

# Output with timestamps
ytt dQw4w9WgXcQ --timestamps
```

### Command Line Options

- `<VIDEO>`: YouTube video URL or video ID (can be placed anywhere)
- `-l, --languages <LANGUAGES>`: Language codes (e.g., en, es, fr). Can specify multiple. Prioritizes manually created transcripts.
- `-t, --translate <LANGUAGE>`: Translate transcript to this language code (requires source language)
- `-f, --format <FORMAT>`: Output format: `json`, `text`, `txt`, `srt`, `markdown`, or `md` (default: `text`)
- `-o, --output <OUTPUT>`: Output file path (if not specified, outputs to stdout)
- `--timestamps`: Show timestamps with transcript text (default: no timestamps)
- `--list`: List all available transcripts instead of fetching
- `--delay <DELAY>`: Delay between requests in milliseconds (default: 500ms)
- `--cleanup`: Clean up transcript using ChatGPT (requires OPENAI_API_KEY env var or --openai-key)
- `--openai-key <OPENAI_KEY>`: OpenAI API key (alternative to OPENAI_API_KEY env var)
- `-h, --help`: Print help

### Examples

```bash
# Get English transcript
ytt "https://www.youtube.com/watch?v=dQw4w9WgXcQ" -l en

# Get Spanish transcript
ytt dQw4w9WgXcQ -l es

# Save as SRT file
ytt dQw4w9WgXcQ --format srt -o transcript.srt

# Save as Markdown file
ytt dQw4w9WgXcQ --format markdown -o transcript.md

# Save as text file
ytt dQw4w9WgXcQ --format txt -o transcript.txt

# Get JSON output
ytt dQw4w9WgXcQ --format json | jq .

# Clean up transcript with ChatGPT
ytt dQw4w9WgXcQ --cleanup -f markdown -o cleaned.md

# With custom delay to avoid rate limiting
ytt dQw4w9WgXcQ --delay 2000
```

## Output Formats

### Text Format (default)
```
Hello world
This is a transcript
Without timestamps
```

### Text Format (with --timestamps flag)
```
[0.00] Hello world
[2.50] This is a transcript
[5.00] With timestamps
```

### JSON Format
```json
[
  {
    "text": "Hello world",
    "start": 0.0,
    "duration": 2.5
  },
  {
    "text": "This is a transcript",
    "start": 2.5,
    "duration": 2.5
  }
]
```

### SRT Format (Subtitle Format)
```
1
00:00:00,000 --> 00:00:02,500
Hello world

2
00:00:02,500 --> 00:00:05,000
This is a transcript
```

### Markdown Format
```markdown
# Transcript

Hello world

This is a transcript

With timestamps (if --timestamps flag is used):
**[0.00]** Hello world
**[2.50]** This is a transcript
```

## ChatGPT Cleanup

The `--cleanup` flag uses ChatGPT to improve transcripts:

- Fixes grammar errors
- Improves sentence structure
- Removes filler words and repetitions
- Makes text more readable
- Removes promotional content (products, websites, courses, etc.)
- Formats as Markdown when using `-f markdown`

### Setup

```bash
# Set API key as environment variable
export OPENAI_API_KEY="your-api-key-here"

# Or pass it directly
ytt video_id --cleanup --openai-key "your-key"
```

### Usage

```bash
# Basic cleanup
ytt mcbwS5Owclo --cleanup

# Cleanup with Markdown formatting
ytt mcbwS5Owclo --cleanup -f markdown -o cleaned.md

# Cleanup and save to file
ytt mcbwS5Owclo --cleanup -o cleaned.txt
```

See [docs/CHATGPT_CLEANUP.md](docs/CHATGPT_CLEANUP.md) for more details.

## Supported URL Formats

- `https://www.youtube.com/watch?v=VIDEO_ID`
- `https://youtu.be/VIDEO_ID`
- `VIDEO_ID` (direct 11-character video ID)

The video argument can be placed anywhere in the command:
```bash
ytt --languages en -f markdown mcbwS5Owclo
ytt mcbwS5Owclo --languages en
ytt --languages en mcbwS5Owclo -f markdown
```

## Rate Limiting

YouTube may rate limit requests if made too quickly. Use the `--delay` flag to add delays between requests:

```bash
# Default delay (500ms)
ytt video_id

# Custom delay (2000ms = 2 seconds)
ytt video_id --delay 2000

# For batch processing
ytt video_id --delay 5000
```

See [docs/RATE_LIMITING.md](docs/RATE_LIMITING.md) for more details.

## Library Usage

You can also use `ytt` as a library in your Rust projects:

```rust
use ytt::YouTubeTranscript;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api = YouTubeTranscript::new();
    
    // Extract video ID from URL
    let video_id = YouTubeTranscript::extract_video_id(
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
    )?;
    
    // Fetch transcript
    let transcript = api.fetch_transcript(&video_id, Some(vec!["en"])).await?;
    
    // Access transcript data
    for item in transcript.transcript {
        println!("[{}s] {}", item.start, item.text);
    }
    
    Ok(())
}
```

Add to your `Cargo.toml`:
```toml
[dependencies]
ytt = { path = "../ytt" }
# or from git
ytt = { git = "https://github.com/ljantzen/ytt.git" }
```

## Error Handling

The library provides specific error types for different failure scenarios:

- `VideoUnavailable` - Video doesn't exist or is deleted
- `TranscriptsDisabled` - Video has no transcripts available
- `NoTranscriptFound` - No transcript found for requested languages
- `AgeRestricted` - Video is age-restricted
- `IpBlocked` - IP address is blocked by YouTube
- `RequestBlocked` - Bot detection triggered
- `InvalidVideoId` - Invalid video ID format
- And more...

## Testing

Run tests with:
```bash
cargo test
```

The project includes comprehensive test coverage:
- 34 tests covering all major functionality
- Unit tests for video ID extraction
- XML parsing tests
- Error handling tests
- Output format tests
- ChatGPT integration tests

## Limitations

- Requires the video to have transcripts/captions available
- Some videos may not have transcripts in all languages
- Auto-generated transcripts may have lower accuracy than manual ones
- Protected videos requiring tokens may not work
- ChatGPT cleanup requires an OpenAI API key and incurs API costs

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT

## Links

- [GitHub Repository](https://github.com/ljantzen/ytt)
