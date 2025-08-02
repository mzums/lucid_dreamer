use chrono::prelude::*;
use clap::{Parser, Subcommand, Args};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    io::{self, Write, Read},
    path::Path,
};

const DREAMS_FILE: &str = "dreams.json";
const STATS_FILE: &str = "stats.json";

#[derive(Parser)]
#[command(name = "Lucid Dreamer")]
#[command(version = "1.0")]
#[command(about = "Terminal tool for lucid dream monitoring and training")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Dream(DreamCommands),
    Stats,
}

#[derive(Args)]
struct DreamCommands {
    #[command(subcommand)]
    action: DreamActions,
}

#[derive(Subcommand)]
enum DreamActions {
    Add,
    List,
    View { id: u32 },
    Search { keyword: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Dream {
    id: u32,
    date: String,
    title: String,
    content: String,
    tags: Vec<String>,
    lucid: Option<bool>,
    dream_sign: Option<String>,
}

#[derive(Serialize, Deserialize, Default)]
struct Config {
    reality_check_prompts: Vec<String>,
}

#[derive(Serialize, Deserialize, Default)]
struct Statistics {
    total_dreams: u32,
    lucid_dreams: u32,
    common_words: HashMap<String, u32>,
    dream_calendar: HashMap<String, u32>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Dream(dream_cmd) => match dream_cmd.action {
            DreamActions::Add => add_dream(),
            DreamActions::List => list_dreams(),
            DreamActions::View { id } => view_dream(id),
            DreamActions::Search { keyword } => search_dreams(&keyword),
        },
        Commands::Stats => show_statistics(),
    }
}

fn add_dream() -> anyhow::Result<()> {
    let mut dreams = load_dreams()?;
    let id = dreams.last().map_or(1, |d| d.id + 1);
    
    print!("Dream title: ");
    io::stdout().flush()?;
    let mut title = String::new();
    io::stdin().read_line(&mut title)?;
    
    println!("Dream content (Ctrl+D when finished):");
    let mut content = String::new();
    io::stdin().read_to_string(&mut content)?;
    
    print!("Tags (comma separated): ");
    io::stdout().flush()?;
    let mut tags_input = String::new();
    io::stdin().read_line(&mut tags_input)?;
    let tags: Vec<String> = tags_input.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    
    let new_dream = Dream {
        id,
        date: Utc::now().format("%Y-%m-%d").to_string(),
        title: title.trim().to_string(),
        content: content.trim().to_string(),
        tags,
        lucid: None,
        dream_sign: None,
    };
    
    dreams.push(new_dream);
    save_dreams(&dreams)?;
    println!("Dream #{} added successfully!", id);

    update_statistics()?;
    
    Ok(())
}

fn list_dreams() -> anyhow::Result<()> {
    let dreams = load_dreams()?;
    if dreams.is_empty() {
        println!("No dreams recorded yet.");
        return Ok(());
    }
    
    println!("{:<5} {:<12} {:<30} {:<20}", "ID", "Date", "Title", "Tags");
    for dream in dreams {
        let tags = dream.tags.join(", ");
        println!("{:<5} {:<12} {:<30} {:<20}", dream.id, dream.date, dream.title, tags);
    }
    
    Ok(())
}

fn view_dream(id: u32) -> anyhow::Result<()> {
    let dreams = load_dreams()?;
    if let Some(dream) = dreams.iter().find(|d| d.id == id) {
        println!("\n--- Dream #{} ---", dream.id);
        println!("Date: {}", dream.date);
        println!("Title: {}", dream.title);
        println!("Tags: {}", dream.tags.join(", "));
        println!("\nContent:\n{}\n", dream.content);
        
        if let Some(sign) = &dream.dream_sign {
            println!("Dream sign: {}", sign);
        }
        if let Some(lucid) = dream.lucid {
            println!("Lucid: {}", lucid);
        }
    } else {
        println!("Dream #{} not found.", id);
    }
    
    Ok(())
}

fn search_dreams(keyword: &str) -> anyhow::Result<()> {
    let dreams = load_dreams()?;
    let keyword = keyword.to_lowercase();
    let mut found = false;
    
    for dream in dreams {
        if dream.title.to_lowercase().contains(&keyword) || 
           dream.content.to_lowercase().contains(&keyword) ||
           dream.tags.iter().any(|t| t.to_lowercase().contains(&keyword)) {
            println!("\n--- Dream #{} ---", dream.id);
            println!("Date: {}", dream.date);
            println!("Title: {}", dream.title);
            println!("Tags: {}", dream.tags.join(", "));
            found = true;
        }
    }
    
    if !found {
        println!("No dreams found matching '{}'", keyword);
    }
    
    Ok(())
}

fn load_dreams() -> anyhow::Result<Vec<Dream>> {
    if !Path::new(DREAMS_FILE).exists() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string(DREAMS_FILE)?;
    Ok(serde_json::from_str(&data)?)
}

fn save_dreams(dreams: &[Dream]) -> anyhow::Result<()> {
    let data = serde_json::to_string_pretty(dreams)?;
    fs::write(DREAMS_FILE, data)?;
    Ok(())
}

fn update_statistics() -> anyhow::Result<()> {
    let dreams = load_dreams()?;
    let mut stats = if Path::new(STATS_FILE).exists() {
        let data = fs::read_to_string(STATS_FILE)?;
        serde_json::from_str(&data)?
    } else {
        Statistics::default()
    };
    
    stats.total_dreams = dreams.len() as u32;
    stats.lucid_dreams = dreams.iter()
        .filter(|d| d.tags.contains(&"#lucid".to_string()))
        .count() as u32;
    
    for dream in &dreams {
        for word in dream.content.split_whitespace() {
            let word = word.to_lowercase();
            *stats.common_words.entry(word).or_insert(0) += 1;
        }
    }
    
    for dream in &dreams {
        let date = dream.date.clone();
        *stats.dream_calendar.entry(date).or_insert(0) += 1;
    }
    
    let data = serde_json::to_string_pretty(&stats)?;
    fs::write(STATS_FILE, data)?;
    Ok(())
}

fn show_statistics() -> anyhow::Result<()> {
    let stats: Statistics = if Path::new(STATS_FILE).exists() {
        let data = fs::read_to_string(STATS_FILE)?;
        serde_json::from_str(&data)?
    } else {
        Statistics::default()
    };
    
    println!("\n--- Dream Statistics ---");
    println!("Total dreams: {}", stats.total_dreams);
    println!("Lucid dreams: {} ({:.1}%)", 
        stats.lucid_dreams,
        if stats.total_dreams > 0 {
            (stats.lucid_dreams as f32 / stats.total_dreams as f32) * 100.0
        } else {
            0.0
        }
    );
    
    let mut words: Vec<_> = stats.common_words.iter().collect();
    words.sort_by(|a, b| b.1.cmp(a.1));
    println!("\nMost frequent dream words:");
    for (i, (word, count)) in words.iter().take(10).enumerate() {
        println!("{}. {} ({} occurrences)", i + 1, word, count);
    }
    
    println!("\nDream calendar:");
    let mut dates: Vec<_> = stats.dream_calendar.iter().collect();
    dates.sort_by_key(|d| d.0);
    for (date, count) in dates.iter().take(30) {
        println!("{}: {} {}", date, "★".repeat(**count as usize), count);
    }
    
    Ok(())
}