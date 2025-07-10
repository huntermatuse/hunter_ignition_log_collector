// this is the sqlite statement to get all logs sorted by unix time
// SELECT
//   timestmp,
//   level_string,
//   logger_name,
//   thread_name,
//   formatted_message
// FROM logging_event
// ORDER BY timestmp DESC;

// fn get_logs_and_do_something
// input log_file_path
// 0. 'log_file_path' and validate log file exists
// 0.1 break if not found
// 1. will call this query and store in a vector
// 2. will then reformat for 'HunterSuperLoggerFields' struct
// 3. will get the first timestamp returned from query and write it to 'last_timestmp_revced'
// 4. can drop the IgnitionLogsWeCareAbout from scope
// 5. print the first and last logs in the vector
// 6. sleep for a 10 seconds
// 7. get logs WHERE timestmp > 'last_timestmp_revced' and stor in vector, jump to step 2
// 7.2 if query returns empty it would jump to 6 and try again after 10 seconds

// fn read config file -> ignition_log_file_path
// ignition_install_directory = from config
// ignition_log_file_path = ignition_install_directory + "\Ignition\logs\system_logs.idb"

// fn query log file -> IgnitionLogsWeCareAbout
// using sqlite query from earlier
// should query with optional last_timestmp_revced if last_timestmp_revced is not None
// returns IgnitionLogsWeCareAbout
use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

fn main() {
    println!("Hello, world!");

    match run_log_monitor() {
        Ok(_) => println!("Log monitor completed successfully"),
        Err(e) => eprintln!("Error running log monitor: {}", e),
    }
}

#[derive(Debug, Clone)]
struct IgnitionLogsWeCareAbout {
    timestamp: i64,
    log_level: String,
    logger_name: String, // this would probably be the category
    thread_name: String, // this would probably be the source
    formatted_message: String,
}

#[derive(Debug, Clone)]
struct HunterSuperLoggerFields {
    timestamp: i64,
    log_level: String,
    source: String,
    category: String,
    message: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    ignition_install_directory: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            ignition_install_directory: r"C:\Program Files\Inductive Automation"
                .to_string(),
        }
    }
}

fn run_log_monitor() -> Result<(), Box<dyn std::error::Error>> {
    let config = read_config()?;
    let log_file_path = get_log_file_path(&config.ignition_install_directory);

    get_logs_and_do_something(&log_file_path)?;

    Ok(())
}

fn read_config() -> Result<Config, Box<dyn std::error::Error>> {
    let config_path = "config.json";

    if Path::new(config_path).exists() {
        let config_str = fs::read_to_string(config_path)?;
        let config: Config = serde_json::from_str(&config_str)?;
        Ok(config)
    } else {
        // Create default config file
        let default_config = Config::default();
        let config_str = serde_json::to_string_pretty(&default_config)?;
        fs::write(config_path, config_str)?;
        println!("Created default config file at {}", config_path);
        Ok(default_config)
    }
}

fn get_log_file_path(ignition_install_directory: &str) -> String {
    let mut path = PathBuf::from(ignition_install_directory);
    path.push("Ignition");
    path.push("logs");
    path.push("system_logs.idb");
    path.to_string_lossy().to_string()
}

fn get_logs_and_do_something(log_file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // 0. Validate log file exists
    if !Path::new(log_file_path).exists() {
        return Err(format!("Log file not found: {}", log_file_path).into());
    }

    println!("Found log file: {}", log_file_path);

    let mut last_timestamp_received: Option<i64> = None;

    loop {
        // 1. Query logs and store in vector
        let ignition_logs = query_log_file(log_file_path, last_timestamp_received)?;

        if ignition_logs.is_empty() {
            println!("No new logs found, waiting 10 seconds...");
            // 6. Sleep for 10 seconds
            thread::sleep(Duration::from_secs(10));
            continue;
        }

        // 2. Reformat for HunterSuperLoggerFields struct
        let hunter_logs: Vec<HunterSuperLoggerFields> = ignition_logs
            .iter()
            .map(|log| HunterSuperLoggerFields {
                timestamp: log.timestamp,
                log_level: log.log_level.clone(),
                source: log.thread_name.clone(),
                category: log.logger_name.clone(),
                message: log.formatted_message.clone(),
            })
            .collect();

        // 3. Get the first timestamp returned from query
        if let Some(first_log) = hunter_logs.first() {
            last_timestamp_received = Some(first_log.timestamp);
            println!("Updated last timestamp received: {}", first_log.timestamp);
        }

        // 4. Drop IgnitionLogsWeCareAbout from scope (happens automatically)

        // 5. Print the first and last logs in the vector
        if !hunter_logs.is_empty() {
            println!("\n=== FIRST LOG ===");
            print_log(&hunter_logs[0]);

            if hunter_logs.len() > 1 {
                println!("\n=== LAST LOG ===");
                print_log(&hunter_logs[hunter_logs.len() - 1]);
            }

            println!("\nProcessed {} logs", hunter_logs.len());
        }

        // 6. Sleep for 10 seconds
        println!("Waiting 10 seconds before next check...\n");
        thread::sleep(Duration::from_secs(10));
    }
}

fn query_log_file(
    log_file_path: &str,
    last_timestamp_received: Option<i64>,
) -> Result<Vec<IgnitionLogsWeCareAbout>, Box<dyn std::error::Error>> {
    let conn = Connection::open(log_file_path)?;

    let query = match last_timestamp_received {
        Some(timestamp) => {
            // Query for logs newer than the last received timestamp
            format!(
                "SELECT timestmp, level_string, logger_name, thread_name, formatted_message \
                 FROM logging_event \
                 WHERE timestmp > {} \
                 ORDER BY timestmp DESC",
                timestamp
            )
        }
        None => {
            // Initial query - get all logs
            "SELECT timestmp, level_string, logger_name, thread_name, formatted_message \
             FROM logging_event \
             ORDER BY timestmp DESC"
                .to_string()
        }
    };

    println!("Executing query: {}", query);

    let mut stmt = conn.prepare(&query)?;
    let log_iter = stmt.query_map([], |row| {
        Ok(IgnitionLogsWeCareAbout {
            timestamp: row.get(0)?,
            log_level: row.get(1)?,
            logger_name: row.get(2)?,
            thread_name: row.get(3)?,
            formatted_message: row.get(4)?,
        })
    })?;

    let mut logs = Vec::new();
    for log in log_iter {
        logs.push(log?);
    }

    Ok(logs)
}

fn print_log(log: &HunterSuperLoggerFields) {
    println!("Timestamp: {}", log.timestamp);
    println!("Level: {}", log.log_level);
    println!("Source: {}", log.source);
    println!("Category: {}", log.category);
    println!("Message: {}", log.message);
}
