mod config;
mod macors;

use {
    crate::config::Config,
    anyhow::Error,
    clap::{Parser, Subcommand},
    macors::*,
    std::{collections::HashMap, env, fs, process, thread, time::Duration},
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    /// Starts recording a macro
    Rec {
        /// Name of the macro to record
        name: String,

        /// Add a description to the macro
        #[arg(short, long, default_value = "add a description")]
        desc: String,

        /// Allow overwriting existing macro
        #[arg(short, long)]
        overwrite: bool,
    },
    /// Runs a recorded macro
    Run {
        /// Name of the macro to run
        name: String,

        /// Number of times to repeat the macro
        #[arg(short = 'n', long = "repeat", default_value_t = 1)]
        repeat: usize,

        /// Optional action selector to run only a specific event (e.g. mouse_press.Left:19th)
        #[arg(short = 'a', long = "action", value_name = "ACTION")]
        action: Option<String>,
    },
    /// List all recorded macros
    Ls,
    /// Show events in a macro with indices
    Show {
        /// Name of the macro to inspect
        name: String,
        /// Show statistics (grouped counts per event kind)
        #[arg(short = 's', long = "stat")]
        stat: bool,
        /// Include wait events in the listing
        #[arg(long = "all")]
        all: bool,
    },
    /// Remove the specified macro
    Rm {
        /// Name of the macro to remove
        name: String,
    },
    /// Edit a recorded macro using $EDITOR
    Edit {
        /// Name of the macro to edit (without .toml)
        name: String,
        /// Optional action selector (positional), e.g. mouse_press.Left:19th
        #[arg(value_name = "ACTION", conflicts_with = "action_flag")]
        action: Option<String>,
        /// Optional action selector flag, same format as positional
        #[arg(
            short = 'a',
            long = "action",
            value_name = "ACTION",
            conflicts_with = "action"
        )]
        action_flag: Option<String>,
    },
}

fn main() -> Result<(), Error> {
    let cfg = Config::load()?;
    let cli = Cli::parse();

    // Handle subcommands
    match &cli.command {
        Commands::Rec {
            name,
            desc,
            overwrite,
        } => {
            if !*overwrite {
                // if overwrite is not set, check if file exists and prevent overwriting
                let macros_dir = config::macros_path();
                let file_path = macros_dir.join(format!("{}.toml", name));
                if file_path.exists() {
                    eprintln!("macro \"{name}\" already exists, use --overwrite to overwrite");
                    return Ok(());
                }
            }

            let secs = cfg.countdown_seconds;
            println!(
                "Beginning recording, default mapping for ending the recording is Esc+Esc+Esc"
            );
            println!("Recording starts in...");
            for i in (1..=secs).rev() {
                println!("{}...", i);
                thread::sleep(Duration::from_millis(950));
            }
            println!("Start!");
            let middle_e_hz = 329;
            let a_bit_more_than_a_second_and_a_half_ms = 100;
            actually_beep::beep_with_hz_and_millis(
                middle_e_hz,
                a_bit_more_than_a_second_and_a_half_ms,
            )
            .unwrap();
            record(&cfg, name.to_string(), desc.to_string());
        }
        Commands::Run { name, repeat, action } => {
            let macros_dir = config::macros_path();
            let file_path = macros_dir.join(format!("{}.toml", name));
            if !file_path.exists() {
                eprintln!("macro \"{name}\" not found");
                return Ok(());
            }

            let action_event = if let Some(raw_action) = action.clone() {
                let contents = match fs::read_to_string(&file_path) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Failed to read macro file: {e}");
                        return Ok(());
                    }
                };
                let events: Macro = match toml::from_str(&contents) {
                    Ok(evs) => evs,
                    Err(e) => {
                        eprintln!("Failed to deserialize macro file: {e:?}");
                        return Ok(());
                    }
                };
                let selector = match parse_action(&raw_action) {
                    Ok(sel) => sel,
                    Err(e) => {
                        eprintln!("Invalid action: {e}");
                        return Ok(());
                    }
                };
                if selector.ordinal == 0 {
                    eprintln!("Ordinal must be 1 or greater");
                    return Ok(());
                }
                let op_idx = find_event_index(&events.events, &selector);
                let Some(event_idx) = op_idx else {
                    eprintln!("No matching event found for action {raw_action}");
                    return Ok(());
                };
                let ev = events.events[event_idx].clone();
                println!("Running action {raw_action} from macro {name} (match #{})", event_idx + 1);
                Some(ev)
            } else {
                println!("Running macro: {} for {} time(s)", name, repeat);
                None
            };

            // Countdown and start beep consistent with full playback
            let secs = cfg.countdown_seconds;
            println!("Playback starts in...");
            for i in (1..=secs).rev() {
                println!("{}...", i);
                thread::sleep(Duration::from_millis(950));
            }
            println!("Begin!");
            let middle_e_hz = 329;
            let a_bit_more_than_a_second_and_a_half_ms = 100;
            actually_beep::beep_with_hz_and_millis(
                middle_e_hz,
                a_bit_more_than_a_second_and_a_half_ms,
            )
            .unwrap();
            for _ in 0..*repeat {
                if let Some(ev) = &action_event {
                    ev.simulate();
                } else {
                    start_playback(&cfg, name);
                }
            }
        }
        Commands::Ls => {
            let macros_dir = config::macros_path();
            // write all files in the directory to stdout but not the toml extension
            for entry in fs::read_dir(macros_dir).expect("Failed to read macros directory") {
                let entry = entry.expect("Failed to read macros directory entry");
                let path = entry.path();
                if path.is_file() && path.extension().is_some() {
                    // get the description from the toml file
                    let contents = fs::read_to_string(&path).expect("Failed to read file");
                    let evs: Macro = match toml::from_str(&contents) {
                        Ok(evs) => evs,
                        Err(e) => {
                            println!("Failed to deserialize macro file: {:?}", e);
                            return Ok(());
                        }
                    };
                    let description = evs.description;
                    let name = path
                        .file_stem()
                        .expect("Failed to get file stem")
                        .to_str()
                        .expect("Failed to convert file stem to str");

                    print!("{name:<27} - ");

                    // print description with line breaks
                    let mut lines = description.lines();
                    if let Some(first_line) = lines.next() {
                        println!("{}", first_line); // Print the first line after the first field
                    }
                    for line in lines {
                        println!("{:<30}{line}", "");
                    }
                }
            }
        }
        Commands::Rm { name } => {
            let macros_dir = config::macros_path();
            let file_path = macros_dir.join(format!("{}.toml", name));
            if !file_path.exists() {
                eprintln!("macro \"{name}\" not found");
                return Ok(());
            }
            fs::remove_file(&file_path).expect("Failed to remove existing macro file");
        }
        Commands::Show { name, stat, all } => {
            let macros_dir = config::macros_path();
            let file_path = macros_dir.join(format!("{}.toml", name));
            let contents = match fs::read_to_string(&file_path) {
                Ok(c) => c,
                Err(_) => {
                    eprintln!("macro \"{name}\" not found");
                    return Ok(());
                }
            };

            let evs: Macro = match toml::from_str(&contents) {
                Ok(evs) => evs,
                Err(e) => {
                    eprintln!("Failed to deserialize macro file: {e:?}");
                    return Ok(());
                }
            };

            let mut shown = 0usize;
            let mut stats: HashMap<String, usize> = HashMap::new();
            let mut idx = 0usize;
            while idx < evs.events.len() {
                let ev = &evs.events[idx];

                if !all && matches!(ev, Event::Wait(_)) {
                    idx += 1;
                    continue;
                }

                if let Event::MousePress(_) = ev {
                    if let Some(collapse) = try_collapse_click(&evs.events, idx) {
                        shown += 1;
                        if !stat {
                            let mut msg = format!("click on ({}, {})", collapse.x, collapse.y);
                            if *all && collapse.wait_ms_total > 0 {
                                msg.push_str(&format!(" (+wait {} ms)", collapse.wait_ms_total));
                            }
                            println!("{:>4}: {}", shown, msg);
                        }
                        if *stat && *all && collapse.waits_consumed > 0 {
                            *stats.entry("wait".to_string()).or_insert(0) += collapse.waits_consumed;
                        }
                        let label = format!("click.{:?}", collapse.button);
                        *stats.entry(label).or_insert(0) += 1;
                        idx = collapse.release_idx + 1;
                        continue;
                    }
                }

                shown += 1;
                if !stat {
                    println!("{:>4}: {}", shown, describe_event(ev));
                }
                let label = stat_label(ev);
                *stats.entry(label).or_insert(0) += 1;
                idx += 1;
            }

            if *stat {
                let mut entries: Vec<(&String, &usize)> = stats.iter().collect();
                entries.sort_by(|a, b| a.0.cmp(b.0));
                for (label, count) in entries {
                    println!("{label}: {count}");
                }
            }
        }
        Commands::Edit {
            name,
            action,
            action_flag,
        } => {
            let editor = match env::var("EDITOR") {
                Ok(val) if !val.is_empty() => val,
                _ => {
                    eprintln!("$EDITOR is not set; set EDITOR to your preferred editor");
                    return Ok(());
                }
            };

            let macros_dir = config::macros_path();
            fs::create_dir_all(&macros_dir).expect("Failed to ensure macros directory exists");
            let file_path = macros_dir.join(format!("{}.toml", name));
            if !file_path.exists() {
                eprintln!("macro \"{name}\" not found");
                return Ok(());
            }

            let selected_action = action_flag.clone().or(action.clone());

            let target_line = if let Some(raw_action) = selected_action {
                let contents = match fs::read_to_string(&file_path) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Failed to read macro file: {e}");
                        return Ok(());
                    }
                };

                let events: Macro = match toml::from_str(&contents) {
                    Ok(evs) => evs,
                    Err(e) => {
                        eprintln!("Failed to deserialize macro file: {e:?}");
                        return Ok(());
                    }
                };

                let selector = match parse_action(&raw_action) {
                    Ok(sel) => sel,
                    Err(e) => {
                        eprintln!("Invalid action: {e}");
                        return Ok(());
                    }
                };

                if selector.ordinal == 0 {
                    eprintln!("Ordinal must be 1 or greater");
                    return Ok(());
                }

                let op_idx = find_event_index(&events.events, &selector);
                let Some(event_idx) = op_idx else {
                    eprintln!("No matching event found for action {raw_action}");
                    return Ok(());
                };

                let op_line = event_start_line(&contents, event_idx);
                let Some(line_num) = op_line else {
                    eprintln!("Could not locate event position in file for action {raw_action}");
                    return Ok(());
                };
                Some(line_num)
            } else {
                None
            };

            let editor_target = match target_line {
                Some(line) => format!("{}:{}", file_path.display(), line),
                None => file_path.display().to_string(),
            };

            let status = process::Command::new(editor).arg(editor_target).status();
            match status {
                Ok(s) if s.success() => {}
                Ok(s) => eprintln!("editor exited with status: {s}"),
                Err(e) => eprintln!("failed to launch editor: {e}"),
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct ActionSelector {
    selector: EventSelector,
    ordinal: usize,
}

#[derive(Debug, Clone)]
enum EventSelector {
    MousePress(Option<String>),
    MouseRelease(Option<String>),
    MouseMove,
    Wait,
    KeyPress(Option<String>),
    KeyRelease(Option<String>),
}

fn describe_event(ev: &Event) -> String {
    match ev {
        Event::Wait(ms) => format!("wait {} ms", ms),
        Event::MousePress(m) => format!(
            "mouse_press {:?} at ({}, {})",
            m.button, m.x as i64, m.y as i64
        ),
        Event::MouseRelease(m) => format!(
            "mouse_release {:?} at ({}, {})",
            m.button, m.x as i64, m.y as i64
        ),
        Event::MouseMove(m) => format!("mouse_move to ({}, {})", m.x as i64, m.y as i64),
        Event::KeyPress(k) => format!("key_press {:?}", k),
        Event::KeyRelease(k) => format!("key_release {:?}", k),
    }
}

#[derive(Debug, Clone, Copy)]
struct ClickCollapse<'a> {
    release_idx: usize,
    waits_consumed: usize,
    wait_ms_total: u64,
    button: &'a rdevin::Button,
    x: i64,
    y: i64,
}

fn is_same_click(a: &MouseEventButton, b: &MouseEventButton) -> bool {
    a.button == b.button && a.x == b.x && a.y == b.y
}

fn try_collapse_click<'a>(events: &'a [Event], start_idx: usize) -> Option<ClickCollapse<'a>> {
    let Event::MousePress(press) = &events[start_idx] else {
        return None;
    };

    let mut idx = start_idx + 1;
    let mut waits_consumed = 0usize;
    let mut wait_ms_total = 0u64;

    while idx < events.len() {
        match &events[idx] {
            Event::Wait(ms) => {
                waits_consumed += 1;
                wait_ms_total += *ms;
                idx += 1;
            }
            Event::MouseRelease(release) if is_same_click(press, release) => {
                return Some(ClickCollapse {
                    release_idx: idx,
                    waits_consumed,
                    wait_ms_total,
                    button: &press.button,
                    x: press.x as i64,
                    y: press.y as i64,
                });
            }
            _ => return None,
        }
    }

    None
}

fn stat_label(ev: &Event) -> String {
    match ev {
        Event::Wait(_) => "wait".to_string(),
        Event::MouseMove(_) => "mouse_move".to_string(),
        Event::MousePress(m) => format!("mouse_press.{:?}", m.button),
        Event::MouseRelease(m) => format!("mouse_release.{:?}", m.button),
        Event::KeyPress(k) => format!("key_press.{:?}", k),
        Event::KeyRelease(k) => format!("key_release.{:?}", k),
    }
}

fn parse_action(raw: &str) -> Result<ActionSelector, String> {
    let mut parts = raw.split(':');
    let head = parts
        .next()
        .ok_or_else(|| "missing selector before ':'".to_string())?;
    let ord_raw = parts
        .next()
        .ok_or_else(|| "missing ordinal after ':'".to_string())?;
    if parts.next().is_some() {
        return Err("too many ':' segments".into());
    }

    let digits: String = ord_raw.chars().filter(|c| c.is_ascii_digit()).collect();
    let ordinal = digits
        .parse::<usize>()
        .map_err(|_| "ordinal is not a number".to_string())?;

    let mut name_parts = head.split('.');
    let kind = name_parts
        .next()
        .ok_or_else(|| "missing event kind".to_string())?;
    let detail = name_parts.next().map(|s| s.to_string());
    if name_parts.next().is_some() {
        return Err("too many '.' segments".into());
    }

    let selector = match kind {
        "mouse_press" => EventSelector::MousePress(detail),
        "mouse_release" => EventSelector::MouseRelease(detail),
        "mouse_move" => EventSelector::MouseMove,
        "wait" => EventSelector::Wait,
        "key_press" => EventSelector::KeyPress(detail),
        "key_release" => EventSelector::KeyRelease(detail),
        other => return Err(format!("unsupported event kind: {other}")),
    };

    Ok(ActionSelector { selector, ordinal })
}

fn find_event_index(events: &[Event], selector: &ActionSelector) -> Option<usize> {
    let mut seen = 0usize;
    for (idx, ev) in events.iter().enumerate() {
        if matches_selector(ev, &selector.selector) {
            seen += 1;
            if seen == selector.ordinal {
                return Some(idx);
            }
        }
    }
    None
}

fn matches_selector(ev: &Event, selector: &EventSelector) -> bool {
    match (selector, ev) {
        (EventSelector::Wait, Event::Wait(_)) => true,
        (EventSelector::MouseMove, Event::MouseMove(_)) => true,
        (EventSelector::MousePress(button), Event::MousePress(m)) => match button {
            Some(b) => button_eq(b, &m.button),
            None => true,
        },
        (EventSelector::MouseRelease(button), Event::MouseRelease(m)) => match button {
            Some(b) => button_eq(b, &m.button),
            None => true,
        },
        (EventSelector::KeyPress(key_name), Event::KeyPress(k)) => match key_name {
            Some(kname) => key_eq(kname, k),
            None => true,
        },
        (EventSelector::KeyRelease(key_name), Event::KeyRelease(k)) => match key_name {
            Some(kname) => key_eq(kname, k),
            None => true,
        },
        _ => false,
    }
}

fn button_eq(name: &str, button: &rdevin::Button) -> bool {
    name.eq_ignore_ascii_case(&format!("{:?}", button))
}

fn key_eq(name: &str, key: &rdevin::Key) -> bool {
    name.eq_ignore_ascii_case(&format!("{:?}", key))
}

fn event_start_line(contents: &str, zero_based_index: usize) -> Option<usize> {
    let mut idx = 0usize;
    for (line_no, line) in contents.lines().enumerate() {
        if line.trim() == "[[events]]" {
            if idx == zero_based_index {
                return Some(line_no + 1); // 1-based for editors
            }
            idx += 1;
        }
    }
    None
}
