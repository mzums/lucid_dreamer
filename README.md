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

Download [Unix binary](https://hc-cdn.hel1.your-objectstorage.com/s/v3/57db2c4a99c09bca74aa82f7ba198830411c3fda_lucid_dreamer) or  
Download [Windows .exe](https://hc-cdn.hel1.your-objectstorage.com/s/v3/c426bbcf2736fce81bccbf73747d51dcfa18ad5b_lucid_dreamer.exe) or  

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
cargo run -- daily

# Add a dream directly
cargo run -- dream add

# List all dreams
cargo run -- dream list

# View dream details
cargo run -- dream view 5

# Search dreams
cargo run -- dream search flying

# Show comprehensive statistics
cargo run -- stats

# Practice a lucid dreaming technique
cargo run -- train mild

# Get a reality check prompt
cargo run -- reality-check
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
