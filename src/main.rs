use clap::Parser;
use regex::Regex;
use serde_json::Value;
use std::process::{Command, Stdio};

#[derive(Parser)]
#[command(version, about = "Attempts to recreate a Dockerfile from the provided Docker image", long_about = None)]
struct Args {
    #[arg(short, long)]
    verbose: bool,

    /// Docker image name
    image: String,
}

fn main() {
    let args = Args::parse();
    let output = Command::new("docker")
        .arg("history")
        .arg("--no-trunc")
        .arg("--format")
        .arg("json")
        .arg(args.image)
        .stdout(Stdio::piped())
        .output()
        .expect("Running docker history failed");

    if !output.status.success() {
        eprintln!(
            "Error returned by docker history: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }

    let docker_str = String::from_utf8_lossy(&output.stdout);

    let lines: Vec<&str> = docker_str
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let formatted_json = format!("[{}]", lines.join(","));

    let json_array: Vec<Value> = serde_json::from_str(&formatted_json)
        .expect("Failed to parse JSON. Ensure the Docker command output is correct.");

    let mut dockerfile = Vec::new();

    for item in json_array.iter().rev() {
        if let Some(created_by) = item.get("CreatedBy") {
            let command = created_by.as_str().unwrap_or("").trim();
            let docker_instruction = parse_created_by(command);
            if let Some(instruction) = docker_instruction {
                dockerfile.push(instruction);
            }
        }
    }

    for line in dockerfile {
        println!("{}\n", line.trim_start());
    }
}

fn parse_created_by(command: &str) -> Option<String> {
    if command.is_empty() {
        return None;
    }

    let re = Regex::new(r"^(ADD|ARG|COPY|RUN|ENV|CMD|LABEL|EXPOSE|VOLUME|ENTRYPOINT)\b").unwrap();

    if command.starts_with("/bin/sh -c #(nop) ") {
        return Some(command.trim_start_matches("/bin/sh -c #(nop) ").to_string());
    } else if command.starts_with("/bin/sh -c ") {
        return Some(format!("RUN {}", command.trim_start_matches("/bin/sh -c ")));
    } else if re.is_match(command) {
        return Some(command.to_string());
    }

    Some(format!("# Unrecognized command: {}", command))
}
