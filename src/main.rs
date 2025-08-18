use chrono::prelude::*;
use clap::{Parser, Subcommand, Args};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    io::{self, Write, Read},
    path::Path,
    process::Command,
    thread,
    time::{Duration, SystemTime},
};
use rand::seq::SliceRandom;
use crossterm::event;
use crossterm::event::{Event, KeyCode};
use chrono::{Utc, NaiveTime};
use std::process::Stdio;
use std::sync::Arc;

const DREAMS_FILE: &str = "dreams.json";
const CONFIG_FILE: &str = "config.json";
const STATS_FILE: &str = "stats.json";
const PROMPTS_FILE: &str = "prompts.txt";
const DAILY_LOG_FILE: &str = "daily_logs.json";
const TECHNIQUES_FILE: &str = "techniques.json";
const ALARMS_FILE: &str = "alarms.json";
const TECHNIQUE_HISTORY_FILE: &str = "technique_history.json";

use std::sync::atomic::{AtomicBool, Ordering};
static ALARM_ACTIVE: AtomicBool = AtomicBool::new(false);

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
    Alarm(AlarmCommands),
    Analyze,
    Report,
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

#[derive(Args)]
struct AlarmCommands {
    #[command(subcommand)]
    action: AlarmActions,
}

#[derive(Subcommand)]
enum AlarmActions {
    Set {
        #[arg(short, long)]
        bedtime: String,
        #[arg(short, long)]
        wake_time: String,
        #[arg(short, long, default_value = "30")]
        awake_minutes: u32,
    },
    List,
    Cancel {
        id: u32,
    },
}

#[derive(Args)]
struct TrainCommands {
    #[command(subcommand)]
    technique: Technique,
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
    technique_practice: Option<TechniquePractice>,
    wbtb_alarm_used: Option<u32>,
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
    technique_effectiveness: HashMap<String, TechniqueStats>,
}

#[derive(Serialize, Deserialize, Default, Debug)]
struct TechniqueStats {
    attempts: u32,
    successes: u32,
    last_practiced: String,
    success_rate: f32,
    optimal_conditions: HashMap<String, f32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct TechniqueData {
    name: String,
    description: String,
    steps: Vec<String>,
    last_practiced: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct TechniquePractice {
    technique: String,
    date: String,
    duration_minutes: u32,
    outcome: TechniqueOutcome,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "data")]
enum TechniqueOutcome {
    Unattempted,
    Failed,
    PartialLucid,
    FullLucid { control_level: u8 },
}

#[derive(Subcommand, Clone)]
enum Technique {
    Mild,
    Wbtb,
    Fild,
    Rc,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct WBTBAlarm {
    id: u32,
    bedtime: String,
    wake_time: String,
    awake_minutes: u32,
    enabled: bool,
    last_triggered: Option<String>,
    success: Option<bool>,
}

fn schedule_alarm(wake_time: &str, awake_minutes: u32) -> anyhow::Result<()> {
    let now = Utc::now();
    let wake_naive = NaiveTime::parse_from_str(wake_time, "%H:%M")?;
    
    let today = now.date_naive();
    let wake_today = today.and_time(wake_naive);
    let wake_utc = Utc.from_utc_datetime(&wake_today);
    
    let duration = if wake_utc > now {
        wake_utc - now
    } else {
        let tomorrow = today.succ_opt().unwrap();
        let wake_tomorrow = tomorrow.and_time(wake_naive);
        Utc.from_utc_datetime(&wake_tomorrow) - now
    };
    
    let secs = duration.num_seconds() as u64;
    
    println!("Alarm scheduled to trigger in {} seconds", secs);
    
    let wake_time = wake_time.to_string();
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(secs));
        trigger_alarm(&wake_time, awake_minutes);
    });
    
    Ok(())
}

fn load_alarms() -> anyhow::Result<Vec<WBTBAlarm>> {
    if !Path::new(ALARMS_FILE).exists() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string(ALARMS_FILE)?;
    Ok(serde_json::from_str(&data)?)
}

fn save_alarms(alarms: &[WBTBAlarm]) -> anyhow::Result<()> {
    let data = serde_json::to_string_pretty(alarms)?;
    fs::write(ALARMS_FILE, data)?;
    Ok(())
}

fn set_wbtb_alarm(bedtime: &str, wake_time: &str, awake_minutes: u32) -> anyhow::Result<()> {
    let mut alarms = load_alarms()?;
    let id = alarms.last().map_or(1, |a| a.id + 1);
    
    let new_alarm = WBTBAlarm {
        id,
        bedtime: bedtime.to_string(),
        wake_time: wake_time.to_string(),
        awake_minutes,
        enabled: true,
        last_triggered: None,
        success: None,
    };
    
    alarms.push(new_alarm);
    save_alarms(&alarms)?;
    
    println!("WBTB alarm set for bedtime: {}, wake at: {}, awake for {} minutes", 
        bedtime, wake_time, awake_minutes);
    
    schedule_alarm(wake_time, awake_minutes)?;
    
    Ok(())
}


fn list_alarms() -> anyhow::Result<()> {
    let alarms = load_alarms()?;
    if alarms.is_empty() {
        println!("No active alarms");
        return Ok(());
    }

    println!("{:<5} {:<10} {:<10} {:<8}", "ID", "Sleep time", "Wake time", "Awake time");
    for alarm in alarms {
        println!("{:<5} {:<10} {:<10} {:<8} min", 
            alarm.id, 
            alarm.bedtime, 
            alarm.wake_time, 
            alarm.awake_minutes);
    }
    
    Ok(())
}

fn cancel_alarm(id: u32) -> anyhow::Result<()> {
    let mut alarms = load_alarms()?;
    if let Some(index) = alarms.iter().position(|a| a.id == id) {
        alarms.remove(index);
        save_alarms(&alarms)?;
        println!("Alarm #{} canceled.", id);
    } else {
        println!("Alarm #{} not found.", id);
    }
    Ok(())
}

fn trigger_alarm(wake_time: &str, awake_minutes: u32) {
    ALARM_ACTIVE.store(true, Ordering::Relaxed);
    
    println!("\n\x1b[5;31m!!! WBTB ALARM !!!\x1b[0m");
    println!("Wake Back to Bed Technique Time!");
    println!("Stay awake for {} minutes", awake_minutes);
    
    play_alarm_sound();
    
    for _ in 0..10 {
        print!("\x1b[?5h");
        io::stdout().flush().unwrap();
        thread::sleep(Duration::from_millis(200));
        print!("\x1b[?5l");
        io::stdout().flush().unwrap();
        thread::sleep(Duration::from_millis(200));
    }
    
    println!("\nAlarm triggered at {}", wake_time);
    
    let awake_minutes = Arc::new(awake_minutes);
    let awake_minutes_clone = Arc::clone(&awake_minutes);
    
    thread::spawn(move || {
        println!("\n\x1b[1;34mAWAKE PERIOD STARTED\x1b[0m");
        println!("You have {} minutes to stay awake", awake_minutes_clone);
        
        for min in (1..=*awake_minutes_clone).rev() {
            println!("{} minutes remaining...", min);
            thread::sleep(Duration::from_secs(60));
        }
        
        println!("\n\x1b[1;32mTIME TO RETURN TO SLEEP!\x1b[0m");
        println!("Lie down, relax, and perform your lucid dream technique");
        println!("Good luck with your lucid dream!");
        
        play_return_to_sleep_sound();
        ALARM_ACTIVE.store(false, Ordering::Relaxed);
    });
}

fn play_return_to_sleep_sound() {
    if cfg!(target_os = "windows") {
        let _ = Command::new("powershell")
            .args(&["-c", "[console]::beep(500, 300)"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    } else {
        print!("\x07");
        std::io::stdout().flush().unwrap();
    }
}

fn play_alarm_sound() {
    if cfg!(target_os = "windows") {
        let _ = Command::new("powershell")
            .args(&["-c", "[console]::beep(1000, 1000)"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    } else {
        print!("\x07");
        std::io::stdout().flush().unwrap();
        
        if cfg!(target_os = "macos") {
            let _ = Command::new("afplay")
                .arg("/System/Library/Sounds/Ping.aiff")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        } else {
            let _ = Command::new("paplay")
                .arg("/usr/share/sounds/freedesktop/stereo/alarm-clock-elapsed.oga")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }
}

fn load_technique_history() -> anyhow::Result<Vec<TechniquePractice>> {
    if !Path::new(TECHNIQUE_HISTORY_FILE).exists() {
        return Ok(Vec::new());
    }
    
    let data = fs::read_to_string(TECHNIQUE_HISTORY_FILE)?;
    let history = serde_json::from_str(&data).unwrap_or_else(|_| Vec::new());
    Ok(history)
}

fn record_technique_practice(technique: &str, outcome: TechniqueOutcome, duration_minutes: u32) -> anyhow::Result<()> {
    let mut history = load_technique_history().unwrap_or_default();
    
    let practice = TechniquePractice {
        technique: technique.to_string(),
        date: Utc::now().format("%Y-%m-%d").to_string(),
        duration_minutes,
        outcome,
    };
    
    history.push(practice);
    let data = serde_json::to_string_pretty(&history)?;
    fs::write(TECHNIQUE_HISTORY_FILE, data)?;
    
    Ok(())
}

fn calculate_technique_effectiveness() -> anyhow::Result<HashMap<String, TechniqueStats>> {
    let history = load_technique_history()?;
    let mut stats: HashMap<String, TechniqueStats> = HashMap::new();
    
    for practice in history {
        let entry = stats.entry(practice.technique.clone()).or_insert_with(|| TechniqueStats {
            attempts: 0,
            successes: 0,
            last_practiced: practice.date.clone(),
            success_rate: 0.0,
            optimal_conditions: HashMap::new(),
        });
        
        entry.attempts += 1;
        
        match practice.outcome {
            TechniqueOutcome::PartialLucid | TechniqueOutcome::FullLucid { .. } => {
                entry.successes += 1;
            }
            _ => {}
        }
        
        if entry.attempts > 0 {
            entry.success_rate = (entry.successes as f32 / entry.attempts as f32) * 100.0;
        }
    }
    
    for stat in stats.values_mut() {
        for value in stat.optimal_conditions.values_mut() {
            *value = (*value / stat.successes as f32) * 100.0;
        }
    }
    
    Ok(stats)
}
fn generate_effectiveness_report() -> anyhow::Result<()> {
    let stats = calculate_technique_effectiveness()?;
    
    println!("\n\x1b[1;34mLUCID DREAM TECHNIQUE EFFECTIVENESS REPORT\x1b[0m");
    println!("===============================================\n");
    
    for (technique, data) in &stats {
        println!("\x1b[1;32m{} Technique\x1b[0m", technique);
        println!("  Success Rate: \x1b[1;33m{:.1}%\x1b[0m ({} successes / {} attempts)", 
            data.success_rate, data.successes, data.attempts);
        println!("  Last Practiced: {}", data.last_practiced);
        
        if !data.optimal_conditions.is_empty() {
            println!("\n  \x1b[1;36mOptimal Conditions:\x1b[0m");
            for (condition, rate) in &data.optimal_conditions {
                println!("    - {}: {:.1}% success rate", condition, rate);
            }
        }
        
        let recommendation = if data.success_rate > 70.0 {
            "Continue using as primary technique".to_string()
        } else if data.success_rate > 40.0 {
            "Combine with another technique".to_string()
        } else {
            "Try modifying approach or switch techniques".to_string()
        };
        
        println!("\n  \x1b[1;35mRecommendation:\x1b[0m {}", recommendation);
        println!();
    }
    
    if stats.len() > 1 {
        println!("\x1b[1;34mTECHNIQUE COMPARISON\x1b[0m");
        let mut sorted: Vec<_> = stats.iter().collect();
        sorted.sort_by(|a, b| b.1.success_rate.partial_cmp(&a.1.success_rate).unwrap());
        
        println!("  Most Effective: \x1b[1;32m{}\x1b[0m ({:.1}% success)", 
            sorted[0].0, sorted[0].1.success_rate);
        println!("  Least Effective: \x1b[1;31m{}\x1b[0m ({:.1}% success)", 
            sorted.last().unwrap().0, sorted.last().unwrap().1.success_rate);
    }
    
    let mut all_stats: Statistics = if Path::new(STATS_FILE).exists() {
        let data = fs::read_to_string(STATS_FILE)?;
        serde_json::from_str(&data)?
    } else {
        Statistics::default()
    };
    
    all_stats.technique_effectiveness = stats;
    let data = serde_json::to_string_pretty(&all_stats)?;
    fs::write(STATS_FILE, data)?;
    
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

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let mut should_wait = false;

    if let Commands::Alarm(AlarmCommands { action: AlarmActions::Set { bedtime, wake_time, awake_minutes } }) = &cli.command {
        set_wbtb_alarm(bedtime, wake_time, *awake_minutes)?;
        should_wait = true;
    } else {
        match cli.command {
            Commands::Dream(dream_cmd) => match dream_cmd.action {
                DreamActions::Add => add_dream()?,
                DreamActions::List => list_dreams()?,
                DreamActions::View { id } => view_dream(id)?,
                DreamActions::Search { keyword } => search_dreams(&keyword)?,
            },
            Commands::Train(train_cmd) => match train_cmd.technique {
                Technique::Mild => practice_technique("MILD")?,
                Technique::Wbtb => practice_technique("WBTB")?,
                Technique::Fild => practice_technique("FILD")?,
                Technique::Rc => practice_technique("RC")?,
            },
            Commands::Stats => show_statistics()?,
            Commands::RealityCheck => reality_check()?,
            Commands::Daily => daily_entry()?,
            Commands::Alarm(alarm_cmd) => match alarm_cmd.action {
                AlarmActions::List => list_alarms()?,
                AlarmActions::Cancel { id } => cancel_alarm(id)?,
                _ => unreachable!(),
            },
            Commands::Analyze => calculate_technique_effectiveness().map(|_| ())?,
            Commands::Report => generate_effectiveness_report()?,
        }
    }

    if should_wait {
        println!("Alarm is active. Press 'q' to quit or wait for alarm...");
        loop {
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key_event) = event::read()? {
                    if key_event.code == KeyCode::Char('q') {
                        println!("Exiting program. Alarm will not trigger.");
                        break;
                    }
                }
            }
            
            if ALARM_ACTIVE.load(Ordering::Relaxed) {
                println!("Alarm triggered. Waiting for awake period completion...");
                
                while ALARM_ACTIVE.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_secs(1));
                }
                
                println!("Awake period completed. Program will now exit.");
                break;
            }
        }
    }

    Ok(())
}


fn practice_technique(technique: &str) -> anyhow::Result<()> {
    let mut techniques = load_techniques()?;
    let tech = techniques.get_mut(technique)
        .ok_or_else(|| anyhow::anyhow!("Technique not found"))?;
    
    println!("\n--- Practicing {} ---", tech.name);
    println!("{}\n", tech.description);
    println!("Steps:");
    for (i, step) in tech.steps.iter().enumerate() {
        println!("{}. {}", i + 1, step);
    }
    
    let start_time = SystemTime::now();
    tech.last_practiced = Some(Utc::now().format("%Y-%m-%d").to_string());
    save_techniques(&techniques)?;
    
    println!("\nPractice started at {}", Utc::now().format("%H:%M"));
    println!("Press any key to complete practice...");
    wait_for_keypress()?;
    
    let duration = start_time.elapsed().unwrap().as_secs() / 60;
    println!("\nPractice duration: {} minutes", duration);
    
    println!("Select outcome:");
    println!("1. Failed (no lucidity)");
    println!("2. Partial lucidity (brief awareness)");
    println!("3. Full lucidity (complete control)");
    
    let mut choice = String::new();
    io::stdin().read_line(&mut choice)?;
    let outcome = match choice.trim() {
        "1" => TechniqueOutcome::Failed,
        "2" => TechniqueOutcome::PartialLucid,
        "3" => {
            print!("Control level (1-5): ");
            io::stdout().flush()?;
            let mut control = String::new();
            io::stdin().read_line(&mut control)?;
            let control_level = control.trim().parse().unwrap_or(3).clamp(1, 5);
            TechniqueOutcome::FullLucid { control_level }
        }
        _ => TechniqueOutcome::Unattempted,
    };
    
    record_technique_practice(technique, outcome, duration as u32)?;
    
    println!("\n✅ Practice recorded! Technique effectiveness updated.");
    Ok(())
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
    
    println!("\n\x1b[1;34mTECHNIQUE EFFECTIVENESS\x1b[0m");
    if let Ok(stats) = calculate_technique_effectiveness() {
        for (technique, data) in stats {
            println!("  {}: {:.1}% success ({} attempts)", 
                technique, data.success_rate, data.attempts);
        }
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
        technique_practice: None,
        wbtb_alarm_used: None,
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

    println!("\n--- WAKE BACK TO BED ---");
    let alarms = load_alarms()?;
    if !alarms.is_empty() {
        println!("Active alarms:");
        for alarm in &alarms {
            println!("[{}] Bed: {}, Wake: {}, Awake: {} min", 
                alarm.id, alarm.bedtime, alarm.wake_time, alarm.awake_minutes);
        }
        
        print!("Did you use a WBTB alarm? (enter ID or 0 for none): ");
        io::stdout().flush()?;
        let mut alarm_choice = String::new();
        io::stdin().read_line(&mut alarm_choice)?;
        if let Ok(id) = alarm_choice.trim().parse::<u32>() {
            if id > 0 && alarms.iter().any(|a| a.id == id) {
                new_log.wbtb_alarm_used = Some(id);
                
                print!("Was it successful? (y/n): ");
                io::stdout().flush()?;
                let mut success = String::new();
                io::stdin().read_line(&mut success)?;
                
                let mut alarms = load_alarms()?;
                if let Some(alarm) = alarms.iter_mut().find(|a| a.id == id) {
                    alarm.last_triggered = Some(today.clone());
                    alarm.success = Some(success.trim().eq_ignore_ascii_case("y"));
                }
                save_alarms(&alarms)?;
            }
        }
    }

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
    
    if let Some(alarm_id) = log.wbtb_alarm_used {
        println!("WBTB Alarm used: #{}", alarm_id);
    }
    
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
