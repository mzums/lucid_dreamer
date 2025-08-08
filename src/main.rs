use chrono::prelude::*;
use clap::{Parser, Subcommand, Args};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    io::{self, Write, Read},
    path::Path,
};
use rand::seq::SliceRandom;
use crossterm::event;
use crossterm::event::{Event, KeyCode};

const DREAMS_FILE: &str = "dreams.json";
const CONFIG_FILE: &str = "config.json";
const STATS_FILE: &str = "stats.json";
const PROMPTS_FILE: &str = "prompts.txt";
const DAILY_LOG_FILE: &str = "daily_logs.json";
const TECHNIQUES_FILE: &str = "techniques.json";

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
    Train(TrainCommands),
    Stats,
    Daily,
    RealityCheck,
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
struct SleepLog {
    date: String,
    bedtime: String,
    wake_time: String,
    quality: u8,
    notes: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct DailyLog {
    date: String,
    dream: Option<Dream>,
    sleep: Option<SleepLog>,
    wake_feeling: Option<String>,
    reality_checks: u32,
    notes: String,
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

#[derive(Serialize, Deserialize, Debug, Clone)]
struct TechniqueData {
    name: String,
    description: String,
    steps: Vec<String>,
    last_practiced: Option<String>,
}

#[derive(Subcommand, Clone)]
enum Technique {
    Mild,
    Wbtb,
    Fild,
    Rc,
}

#[derive(Args)]
struct TrainCommands {
    #[command(subcommand)]
    technique: Technique,
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
        Commands::Train(train_cmd) => match train_cmd.technique {
            Technique::Mild => practice_technique(Technique::Mild),
            Technique::Wbtb => practice_technique(Technique::Wbtb),
            Technique::Fild => practice_technique(Technique::Fild),
            Technique::Rc => practice_technique(Technique::Rc),
        },
        Commands::Stats => show_statistics(),
        Commands::RealityCheck => reality_check(),
        Commands::Daily => daily_entry(),
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
    let dreams = load_dreams()?;
    let daily_logs = load_daily_logs()?;
    
    let sleep_logs: Vec<_> = daily_logs.iter()
        .filter_map(|log| log.sleep.as_ref())
        .collect();
    
    println!("\n--- DREAM & SLEEP STATISTICS ---");
    
    println!("\nDREAM STATS:");
    println!("Total dreams recorded: {}", dreams.len());
    
    let lucid_dreams = dreams.iter()
        .filter(|d| d.lucid == Some(true))
        .count();
    println!("Lucid dreams: {} ({:.1}%)", 
        lucid_dreams,
        if !dreams.is_empty() {
            (lucid_dreams as f32 / dreams.len() as f32) * 100.0
        } else {
            0.0
        }
    );
    
    if !dreams.is_empty() {
        let total_words: usize = dreams.iter()
            .map(|d| d.content.split_whitespace().count())
            .sum();
        println!("Average dream length: {} words", total_words / dreams.len());
    }
    
    let mut word_counts = HashMap::new();
    for dream in &dreams {
        for word in dream.content.split_whitespace() {
            let word = word.to_lowercase().trim_matches(|c: char| !c.is_alphanumeric()).to_string();
            if !word.is_empty() {
                *word_counts.entry(word).or_insert(0) += 1;
            }
        }
    }
    
    let mut sorted_words: Vec<_> = word_counts.iter().collect();
    sorted_words.sort_by(|a, b| b.1.cmp(a.1));
    
    if !sorted_words.is_empty() {
        println!("\nMost frequent dream words:");
        for (i, (word, count)) in sorted_words.iter().take(10).enumerate() {
            println!("{}. {} ({} occurrences)", i + 1, word, count);
        }
    }
    
    println!("\nSLEEP STATS:");
    if sleep_logs.is_empty() {
        println!("No sleep data recorded yet.");
    } else {
        let mut total_duration = 0.0;
        let mut total_quality = 0.0;
        let mut sleep_durations = Vec::new();
        
        for log in &sleep_logs {
            if let Ok(bedtime) = NaiveTime::parse_from_str(&log.bedtime, "%H:%M") {
                if let Ok(wake_time) = NaiveTime::parse_from_str(&log.wake_time, "%H:%M") {
                    let mut duration = (wake_time - bedtime).num_minutes() as f32 / 60.0;
                    if duration < 0.0 {
                        duration += 24.0;
                    }
                    total_duration += duration;
                    sleep_durations.push(duration);
                }
            }
            total_quality += log.quality as f32;
        }
        
        let avg_duration = total_duration / sleep_logs.len() as f32;
        let avg_quality = total_quality / sleep_logs.len() as f32;
        
        let min_duration = sleep_durations.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        let max_duration = sleep_durations.iter().fold(0.0_f32, |a, &b| a.max(b));
        
        println!("Nights tracked: {}", sleep_logs.len());
        println!("Average sleep duration: {:.1} hours", avg_duration);
        println!("Min sleep: {:.1}h, Max sleep: {:.1}h", min_duration, max_duration);
        println!("Average sleep quality: {:.1}/5", avg_quality);
        
        let lucid_nights = daily_logs.iter()
            .filter(|log| 
                log.dream.as_ref().map_or(false, |d| d.lucid == Some(true)) &&
                log.sleep.is_some()
            )
            .count();
        
        if lucid_nights > 0 {
            println!("\nLucid dreams occurred on {:.1}% of tracked nights", 
                (lucid_nights as f32 / sleep_logs.len() as f32) * 100.0);
        }
        
        let lucid_quality: f32 = daily_logs.iter()
            .filter_map(|log| 
                if log.dream.as_ref().map_or(false, |d| d.lucid == Some(true)) {
                    log.sleep.as_ref().map(|s| s.quality as f32)
                } else {
                    None
                }
            )
            .sum();
        
        if lucid_nights > 0 {
            println!("Average sleep quality on lucid nights: {:.1}/5", 
                lucid_quality / lucid_nights as f32);
        }
        
        println!("\nSleep duration consistency:");
        for duration in sleep_durations.iter().take(30) {
            println!("{:.1}h: {}", duration, "▇".repeat((*duration * 2.0) as usize));
        }
    }
    
    println!("\nREALITY CHECKS:");
    let total_rc: u32 = daily_logs.iter().map(|log| log.reality_checks).sum();
    println!("Total reality checks recorded: {}", total_rc);
    
    if !daily_logs.is_empty() {
        let avg_rc = total_rc as f32 / daily_logs.len() as f32;
        println!("Average per day: {:.1}", avg_rc);
        
        let max_rc = daily_logs.iter().map(|log| log.reality_checks).max().unwrap_or(0);
        let min_rc = daily_logs.iter().map(|log| log.reality_checks).min().unwrap_or(0);
        println!("Most active day: {} checks, Least active: {}", max_rc, min_rc);
    }
    
    println!("\nDREAM CALENDAR:");
    let mut dream_calendar = HashMap::new();
    for dream in &dreams {
        *dream_calendar.entry(dream.date.clone()).or_insert(0) += 1;
    }
    
    let mut sorted_dates: Vec<_> = dream_calendar.iter().collect();
    sorted_dates.sort_by_key(|(date, _)| (*date).clone());

    for (date, count) in sorted_dates.iter().take(30) {
        println!("{}: {} {}", date, "★".repeat(**count as usize), count);
    }
    
    Ok(())
}

fn load_daily_logs() -> anyhow::Result<Vec<DailyLog>> {
    if !Path::new(DAILY_LOG_FILE).exists() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string(DAILY_LOG_FILE)?;
    Ok(serde_json::from_str(&data)?)
}

fn daily_entry() -> anyhow::Result<()> {
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let mut logs = load_daily_logs()?;
    
    if let Some(log) = logs.iter().find(|l| l.date == today) {
        println!("Daily entry already exists for today:");
        print_daily_summary(log);
        print!("Do you want to update it? (y/n): ");
        io::stdout().flush()?;
        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        
        if !answer.trim().eq_ignore_ascii_case("y") {
            return Ok(());
        }
    }

    let mut new_log = DailyLog {
        date: today.clone(),
        dream: None,
        sleep: None,
        wake_feeling: None,
        reality_checks: 0,
        notes: String::new(),
    };

    println!("\n--- SLEEP LOG ---");
    print!("Bedtime last night (HH:MM): ");
    io::stdout().flush()?;
    let mut bedtime = String::new();
    io::stdin().read_line(&mut bedtime)?;
    
    print!("Wake time today (HH:MM): ");
    io::stdout().flush()?;
    let mut wake_time = String::new();
    io::stdin().read_line(&mut wake_time)?;
    
    print!("Sleep quality (1-5): ");
    io::stdout().flush()?;
    let mut quality_input = String::new();
    io::stdin().read_line(&mut quality_input)?;
    let quality = quality_input.trim().parse::<u8>()?.clamp(1, 5);
    
    new_log.sleep = Some(SleepLog {
        date: today.clone(),
        bedtime: bedtime.trim().to_string(),
        wake_time: wake_time.trim().to_string(),
        quality,
        notes: String::new(),
    });

    println!("\n--- DREAM RECALL ---");
    print!("Do you remember a dream? (y/n): ");
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    
    if answer.trim().eq_ignore_ascii_case("y") {
        let mut dreams = load_dreams()?;
        let id = dreams.last().map_or(1, |d| d.id + 1);
        
        print!("Dream title: ");
        io::stdout().flush()?;
        let mut title = String::new();
        io::stdin().read_line(&mut title)?;
        
        println!("Dream content (Ctrl+D when finished):");
        let mut content = String::new();
        io::stdin().read_to_string(&mut content)?;
        
        print!("Was it a lucid dream? (y/n): ");
        io::stdout().flush()?;
        let mut lucid = String::new();
        io::stdin().read_line(&mut lucid)?;
        let is_lucid = lucid.trim().eq_ignore_ascii_case("y");
        
        print!("Did you notice any dream sign? (optional): ");
        io::stdout().flush()?;
        let mut sign = String::new();
        io::stdin().read_line(&mut sign)?;
        
        let mut tags = vec![];
        if is_lucid {
            tags.push("#lucid".to_string());
        }
        
        let dream = Dream {
            id,
            date: today.clone(),
            title: title.trim().to_string(),
            content: content.trim().to_string(),
            tags,
            lucid: Some(is_lucid),
            dream_sign: if sign.trim().is_empty() {
                None
            } else {
                Some(sign.trim().to_string())
            },
        };
        
        dreams.push(dream.clone());
        save_dreams(&dreams)?;
        new_log.dream = Some(dream);
    }

    println!("\n--- DAILY METRICS ---");
    print!("How do you feel after waking up?: ");
    io::stdout().flush()?;
    let mut feeling = String::new();
    io::stdin().read_line(&mut feeling)?;
    new_log.wake_feeling = Some(feeling.trim().to_string());
    
    print!("Number of reality checks performed: ");
    io::stdout().flush()?;
    let mut rc_input = String::new();
    io::stdin().read_line(&mut rc_input)?;
    new_log.reality_checks = rc_input.trim().parse().unwrap_or(0);
    
    println!("Additional notes (optional):");
    let mut notes = String::new();
    io::stdin().read_line(&mut notes)?;
    new_log.notes = notes.trim().to_string();

    if let Some(index) = logs.iter().position(|l| l.date == today) {
        logs[index] = new_log;
    } else {
        logs.push(new_log);
    }
    
    save_daily_logs(&logs)?;
    println!("\nDaily entry completed!");
    
    update_statistics()?;
    generate_weekly_report()?;
    
    Ok(())
}

fn save_daily_logs(logs: &[DailyLog]) -> anyhow::Result<()> {
    let data = serde_json::to_string_pretty(logs)?;
    fs::write(DAILY_LOG_FILE, data)?;
    Ok(())
}

fn generate_weekly_report() -> anyhow::Result<()> {
    let dreams = load_dreams()?;
    let now = Utc::now();
    let one_week_ago = now - chrono::Duration::days(7);
    
    let weekly_dreams: Vec<_> = dreams.iter()
        .filter(|d| {
            if let Ok(dream_date) = NaiveDate::parse_from_str(&d.date, "%Y-%m-%d") {
                let dream_datetime = dream_date.and_hms_opt(0, 0, 0).unwrap();
                dream_datetime >= one_week_ago.naive_utc()
            } else {
                false
            }
        })
        .collect();
    
    let lucid_count = weekly_dreams.iter()
        .filter(|d| d.tags.contains(&"#lucid".to_string()))
        .count();
    
    println!("\n--- Weekly Report ---");
    println!("Dreams this week: {}", weekly_dreams.len());
    println!("Lucid dreams: {}", lucid_count);
    println!("Dream frequency: {:.1} per day", weekly_dreams.len() as f32 / 7.0);
    
    if !weekly_dreams.is_empty() {
        let total_words: usize = weekly_dreams.iter()
            .map(|d| d.content.split_whitespace().count())
            .sum();
        println!("Average dream length: {} words", total_words / weekly_dreams.len());
    }
    
    Ok(())
}

fn load_config() -> anyhow::Result<Config> {
    if Path::new(PROMPTS_FILE).exists() {
        let prompts = fs::read_to_string(PROMPTS_FILE)?
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        
        return Ok(Config {
            reality_check_prompts: prompts,
        });
    }
    
    if Path::new(CONFIG_FILE).exists() {
        let data = fs::read_to_string(CONFIG_FILE)?;
        return Ok(serde_json::from_str(&data)?);
    }
    
    Ok(Config::default())
}

fn reality_check() -> anyhow::Result<()> {
    let config = load_config()?;
    if config.reality_check_prompts.is_empty() {
        return Err(anyhow::anyhow!("No reality check prompts found"));
    }
    
    let prompt = config.reality_check_prompts
        .choose(&mut rand::thread_rng())
        .unwrap();
    
    println!("\nREALITY CHECK: {}\n", prompt);
    Ok(())
}

fn print_daily_summary(log: &DailyLog) {
    println!("\n--- DAILY SUMMARY FOR {} ---", log.date);
    
    if let Some(sleep) = &log.sleep {
        println!("Sleep: {} to {} (Quality: {}/5)", 
            sleep.bedtime, sleep.wake_time, sleep.quality);
    }
    
    if let Some(dream) = &log.dream {
        println!("Dream: {} - {}", dream.title, 
            if dream.lucid == Some(true) { "(Lucid)" } else { "" });
    } else {
        println!("No dream recalled");
    }
    
    if let Some(feeling) = &log.wake_feeling {
        println!("Wake feeling: {}", feeling);
    }
    
    println!("Reality checks: {}", log.reality_checks);
    
    if !log.notes.is_empty() {
        println!("Notes: {}", log.notes);
    }
}

fn load_techniques() -> anyhow::Result<HashMap<String, TechniqueData>> {
    if Path::new(TECHNIQUES_FILE).exists() {
        let data = fs::read_to_string(TECHNIQUES_FILE)?;
        return Ok(serde_json::from_str(&data)?);
    }
    
    let mut techniques = HashMap::new();
    
    techniques.insert("MILD".to_string(), TechniqueData {
        name: "Mnemonic Induction of Lucid Dreams (MILD)".to_string(),
        description: "A technique that uses prospective memory to increase lucid dream frequency".to_string(),
        steps: vec![
            "Set intention to remember you're dreaming".to_string(),
            "Visualize yourself becoming lucid in a recent dream".to_string(),
            "Repeat a mantra like 'Next time I'm dreaming, I'll remember I'm dreaming'".to_string(),
            "Fall asleep while maintaining this intention".to_string(),
        ],
        last_practiced: None,
    });
    
    techniques.insert("WBTB".to_string(), TechniqueData {
        name: "Wake Back To Bed (WBTB)".to_string(),
        description: "Wake up after 4-6 hours of sleep, stay awake briefly, then return to sleep".to_string(),
        steps: vec![
            "Set alarm for 4-6 hours after bedtime".to_string(),
            "When alarm goes off, stay awake for 20-60 minutes".to_string(),
            "Engage in lucid dream preparation activities".to_string(),
            "Return to sleep while maintaining awareness".to_string(),
        ],
        last_practiced: None,
    });
    
    techniques.insert("FILD".to_string(), TechniqueData {
        name: "Finger Induced Lucid Dream (FILD)".to_string(),
        description: "A subtle finger movement technique to enter directly into a lucid dream".to_string(),
        steps: vec![
            "Wake up after 4-6 hours of sleep".to_string(),
            "Lie completely still".to_string(),
            "Gently move index and middle fingers as if playing piano".to_string(),
            "After 10-20 seconds, perform a reality check".to_string(),
        ],
        last_practiced: None,
    });
    
    techniques.insert("RC".to_string(), TechniqueData {
        name: "Reality Checks".to_string(),
        description: "Habitual checks throughout the day to test if you're dreaming".to_string(),
        steps: vec![
            "Perform 10+ reality checks daily".to_string(),
            "Question your reality: 'Am I dreaming?'".to_string(),
            "Examine your environment for dream signs".to_string(),
            "Try to push finger through palm or read text twice".to_string(),
        ],
        last_practiced: None,
    });
    
    Ok(techniques)
}

fn save_techniques(techniques: &HashMap<String, TechniqueData>) -> anyhow::Result<()> {
    let data = serde_json::to_string_pretty(techniques)?;
    fs::write(TECHNIQUES_FILE, data)?;
    Ok(())
}

fn practice_technique(technique: Technique) -> anyhow::Result<()> {
    let technique_name = match technique {
        Technique::Mild => "MILD",
        Technique::Wbtb => "WBTB",
        Technique::Fild => "FILD",
        Technique::Rc => "RC",
    };
    
    let mut techniques = load_techniques()?;
    let tech = techniques.get_mut(technique_name)
        .ok_or_else(|| anyhow::anyhow!("Technique not found"))?;
    
    println!("\n--- Practicing {} ---", tech.name);
    println!("{}\n", tech.description);
    println!("Steps:");
    for (i, step) in tech.steps.iter().enumerate() {
        println!("{}. {}", i + 1, step);
    }
    
    tech.last_practiced = Some(Utc::now().format("%Y-%m-%d").to_string());
    save_techniques(&techniques)?;
    
    println!("\nPractice started at {}", Utc::now().format("%H:%M"));
    println!("Press any key to complete practice...");
    wait_for_keypress()?;
    
    println!("\n✅ Practice completed! Remember to record results in your dream journal.");
    Ok(())
}

fn wait_for_keypress() -> anyhow::Result<()> {
    loop {
        if let Event::Key(event) = event::read()? {
            if event.code != KeyCode::Null {
                break;
            }
        }
    }
    Ok(())
}
