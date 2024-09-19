//               Copyright Adam McKellar 2024
// Distributed under the Boost Software License, Version 1.0.
//         (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

use std::fs;

use async_process::Command;
use once_cell::sync::Lazy;
use regex::Regex;
use tempfile::TempDir;

pub(super) async fn git_installed() -> bool {
    match Command::new("git").arg("--version").status().await {
        Ok(status) => status.success(),
        Err(_) => false,
    }
}

async fn get_git_tags(url: &String) -> Vec<String> {
    let output = Command::new("git")
                                .args(["ls-remote", "--tags", url.as_str()])
                                .output().await.expect("Failed executing git.");
    if !output.status.success() {
        return vec![];
    }

    static TAG_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^.*refs/tags/(?<tag>\w?[\d\.]*)$").unwrap()
    });

    let output_str = String::from_utf8(output.stdout).unwrap();

    let mut  tag_list = vec![];

    for line in output_str.lines() {
        if let Some(tag_capture) = TAG_REGEX.captures(line) {
            tag_list.push(tag_capture["tag"].to_owned());
        }
    }
    
    tag_list
}

async fn tag_of_repo(url: &String, tag_sub_str: &String) -> Option<String> {
    let tags = get_git_tags(url).await;
    for tag in tags {
        if tag.contains(tag_sub_str) {
            return Some(tag);
        }
    }
    None
}

pub(super) async fn get_license_text_from_git_repository(url: &String, tag_sub_str: &String) ->  Option<String> {
    let tmp_dir = TempDir::new().unwrap();
    let path = tmp_dir.path();

    let output = if let Some(tag) = tag_of_repo(url, tag_sub_str).await {
        Command::new("git")
            .current_dir(path)
            .args(["clone", "--branch", tag.as_str(), "--depth", "1", url.as_str()])
            .output().await.unwrap()
    } else {
        Command::new("git")
            .current_dir(path)
            .args(["clone", "--depth", "1", url.as_str()])
            .output().await.unwrap()
    };

    if !output.status.success() {
        return None;
    }

    let cloned_git_path = fs::read_dir(path).unwrap().next().unwrap().unwrap().path();
    debug_assert!(cloned_git_path.is_dir());

    static LICENSE_FILE_NAME_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i).*(license|copying|authors|notice|eula).*").unwrap()
    });

    let entries = fs::read_dir(cloned_git_path).unwrap();

    let mut potential_license_files = vec![];

    for entry in entries {
        if let Ok(entry) = entry {
            if let Ok(metadata) = entry.metadata() {
                if !metadata.is_file() {
                    continue;
                }
                if LICENSE_FILE_NAME_REGEX.is_match(&entry.file_name().to_string_lossy()) {
                    potential_license_files.push(entry.path());
                }
            }
        }
    }

    let mut license_text_vec = vec![];

    for license_file in potential_license_files {
        if let Ok(license_text) = fs::read_to_string(license_file) {
            license_text_vec.push(license_text);
        }
    }

    if license_text_vec.is_empty() {
        return None;
    }

    Some(license_text_vec.join("\n\n"))
}

