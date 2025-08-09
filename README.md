# Lucid Dreamer

Lucid Dreamer is a comprehensive terminal-based tool designed to help you track, analyze, and enhance your lucid dreaming practice. It combines dream journaling with sleep tracking and reality check monitoring and provides powerful insights into your dreams while helping you develop the skills needed for lucid dreaming.

## Key Features

### Unified Dream & Sleep Tracking
- **Daily entry system** combining dream recall and sleep metrics
- Record bedtime, wake time, and sleep quality (1-5 scale)
- Track wake feelings and daily notes
- Automatic dream ID generation and timestamping
- Weekly reports

### Intelligent Dream Journal
- Add, list, view, and search dreams with tags
- Mark dreams as lucid with optional dream signs
- Persistent JSON storage for all dream records
- Powerful search by keyword, title, content, or tags

### Comprehensive Statistics
- **Dream analysis**: 
  - Lucid dream percentage and frequency
  - Most common dream words
  - Dream calendar visualization
- **Sleep insights**:
  - Average sleep duration and quality
  - Correlation between sleep patterns and lucid dreams
- **Reality check tracking**:
  - Daily and total reality checks performed
  - Most/least active days

### Lucid Dream Training
- Guided practice for proven techniques:
  - **MILD** (Mnemonic Induction of Lucid Dreams)
  - **WBTB** (Wake Back To Bed)
  - **FILD** (Finger Induced Lucid Dreams)
  - **RC** (Reality Checks)
- Technique descriptions and step-by-step instructions

## Installation

Install the binary from [release section](https://github.com/mzums/lucid_dreamer/releases)

1. Ensure you have Rust installed
2. Clone the repository:
   ```bash
   git clone https://github.com/mzums/lucid_dreamer.git
   cd lucid-dreamer
   ```
3. Build the project:
   ```bash
   cargo build --release
   ```
4. Run the executable:
   ```bash
   ./target/release/lucid_dreamer
   ```

## Usage

### Basic Commands

```bash
# Start your daily entry (sleep + dreams)
lucid-dreamer daily

# Add a dream directly
lucid-dreamer dream add

# List all dreams
lucid-dreamer dream list

# View dream details
lucid-dreamer dream view 5

# Search dreams
lucid-dreamer dream search flying

# Show comprehensive statistics
lucid-dreamer stats

# Practice a lucid dreaming technique
lucid-dreamer train mild

# Get a reality check prompt
lucid-dreamer reality-check
```

### Data Storage

All data is stored in JSON files in the application directory:

- `dreams.json` - Dream journal entries
- `daily_logs.json` - Combined sleep and dream records
- `techniques.json` - Lucid dreaming techniques explained
- `stats.json` - Dream statistics

## Why Use Lucid Dreamer?

Unlike generic journaling apps, Lucid Dreamer is specifically designed for dream explorers. By combining sleep science with dream analysis, it helps you:

1. Discover patterns in your dream content
2. Identify optimal sleep conditions for lucidity
3. Track your reality check habit development
4. Recognize your personal dream signs
5. Stay motivated with progress insights
