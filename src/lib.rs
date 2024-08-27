extern crate walkdir;
extern crate glob;
extern crate anyhow;

use std::collections::{HashSet, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use anyhow::Context;
use walkdir::{WalkDir, DirEntry};
use glob::MatchOptions;

fn gitignore_to_glob(pattern: &str) -> String {
    let mut glob_pattern = String::new();

    // Handle negation
    if pattern.starts_with('!') {
        glob_pattern.push('!');
        return gitignore_to_glob(&pattern[1..]);
    }

    // Handle patterns that should match only directories
    if pattern.ends_with('/') {
        glob_pattern.push_str(&pattern[..pattern.len() - 1]);
        glob_pattern.push_str("*");
        return glob_pattern;
    }

    // Handle *.ext patterns
    if pattern.starts_with('*') && !pattern.contains('/') {
        return format!("**/{}", pattern);
    }

    // Convert '**' to glob-compatible syntax
    let parts: Vec<&str> = pattern.split("**").collect();
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            glob_pattern.push_str("**");
        }
        glob_pattern.push_str(part);
    }

    // If pattern doesn't start with '/', make it match in any directory
    if !pattern.starts_with('/') && !glob_pattern.starts_with('*') {
        glob_pattern = format!("**/{}", glob_pattern);
    }

    glob_pattern
}

fn parse_gitignore(gitignore_path: &Path) -> anyhow::Result<Vec<String>> {
    let file = File::open(gitignore_path).context("Failed to open .gitignore file")?;
    let reader = BufReader::new(file);
    let mut patterns = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            let glob_pattern = gitignore_to_glob(trimmed);
            log::debug!("Converted .gitignore pattern '{}' to glob pattern '{}'", trimmed, glob_pattern);
            patterns.push(glob_pattern);
        }
    }

    Ok(patterns)
}

fn generate_ignore_set(directory: &Path, ignore_patterns: &[String], output_file: &Path, gitignore_path: Option<&Path>) -> anyhow::Result<HashSet<PathBuf>> {
    let mut ignore_set = HashSet::new();
    let options = MatchOptions {
        case_sensitive: true,
        require_literal_separator: false,
        require_literal_leading_dot: false,
    };

    let mut all_patterns = ignore_patterns.to_vec();
    if let Some(gitignore) = gitignore_path {
        let gitignore_patterns = parse_gitignore(gitignore)?;
        all_patterns.extend(gitignore_patterns);
    }

    // Add the output file to the ignore set
    let absolute_output = if output_file.is_absolute() {
        output_file.to_path_buf()
    } else {
        directory.join(output_file)
    };
    ignore_set.insert(absolute_output.clone());
    log::debug!("Added output file to ignore set: {}", absolute_output.display());

    for pattern in &all_patterns {
        let glob_pattern = if pattern.starts_with('/') {
            directory.join(&pattern[1..])
        } else {
            directory.join(pattern)
        };
        log::debug!("Checking glob pattern: {}", glob_pattern.display());
        match glob::glob_with(&glob_pattern.to_string_lossy(), options) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(path) => {
                            log::debug!("Adding to ignore set: {}", path.display());
                            ignore_set.insert(path);
                        },
                        Err(e) => log::warn!("Error matching path: {}", e),
                    }
                }
            },
            Err(e) => log::warn!("Error in glob pattern {}: {}", pattern, e),
        }
    }

    Ok(ignore_set)
}

pub fn should_ignore(entry: &DirEntry, ignore_set: &HashSet<PathBuf>) -> bool {
    ignore_set.contains(entry.path()) ||
    entry.path().ancestors().any(|p| ignore_set.contains(p))
}

pub fn write_directory_contents(directory: &Path, output: &Path, ignore_patterns: &[String], gitignore_path: Option<&Path>) -> anyhow::Result<()> {
    let absolute_directory = directory.canonicalize().context("Failed to get absolute path")?;
    let ignore_set = generate_ignore_set(&absolute_directory, ignore_patterns, output, gitignore_path)?;

    let filetree = WalkDir::new(&absolute_directory).into_iter().filter_entry(|e| !should_ignore(e, &ignore_set));
    let filetree_clone = WalkDir::new(&absolute_directory).into_iter().filter_entry(|e| !should_ignore(e, &ignore_set));

    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(output)
        .context("Failed to create or truncate output file")?;
    let mut writer = BufWriter::new(file);

    // Write directory tree
    writeln!(writer, "<tree>")?;
    for entry in filetree {
        let entry = entry.context("Failed to read directory entry")?;
        let depth = entry.depth();
        let indent = "    ".repeat(depth);
        let name = entry.file_name().to_string_lossy();
        if entry.file_type().is_dir() {
            writeln!(writer, "{}{}/", indent, name)?;
        } else {
            writeln!(writer, "{}{}", indent, name)?;
        }
    }
    writeln!(writer, "</tree>\n")?;

    struct DirectoryState {
        path: PathBuf,
        is_open: bool,
    }

    // Write file contents with nested directory structure
    let mut dir_stack: VecDeque<DirectoryState> = VecDeque::new();

    // Write file contents
    for entry in filetree_clone {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        let relative_path = path.strip_prefix(&absolute_directory).context("Failed to get relative path")?;

        // Close directories that are no longer needed
        while let Some(last_dir) = dir_stack.back() {
            if !relative_path.starts_with(&last_dir.path) {
                let closing_dir = dir_stack.pop_back().unwrap();
                if closing_dir.is_open {
                    let indent = "    ".repeat(closing_dir.path.components().count() - 1);
                    writeln!(writer, "{}</{}>\n", indent, closing_dir.path.file_name().unwrap().to_string_lossy())?;
                }
            } else {
                break;
            }
        }

        // Open new directories as needed
        let mut current_path = PathBuf::new();
        for component in relative_path.parent().unwrap_or(Path::new("")).components() {
            current_path.push(component);
            if !dir_stack.iter().any(|dir| dir.path == current_path) {
                let indent = "    ".repeat(current_path.components().count() - 1);
                writeln!(writer, "{}<{}>", indent, component.as_os_str().to_string_lossy())?;
                dir_stack.push_back(DirectoryState { path: current_path.clone(), is_open: true });
            }
        }

        // Write file content
        if entry.file_type().is_file() {
            let indent = "    ".repeat(relative_path.parent().map(|p| p.components().count()).unwrap_or(0));
            let file_name = relative_path.file_name().unwrap().to_string_lossy();
            writeln!(writer, "{}<{}>", indent, file_name)?;

            match std::fs::read_to_string(path) {
                Ok(content) => {
                    for line in content.lines() {
                        writeln!(writer, "{}{}", indent, line)?;
                    }
                },
                Err(_) => writeln!(writer, "{}Binary or inaccessible file: {}", indent, path.display())?,
            }

            writeln!(writer, "{}</{}>", indent, file_name)?;
        }
    }
    // Close any remaining open directories
    while let Some(closing_dir) = dir_stack.pop_back() {
        if closing_dir.is_open {
            let indent = "    ".repeat(closing_dir.path.components().count() - 1);
            writeln!(writer, "{}</{}>\n", indent, closing_dir.path.file_name().unwrap().to_string_lossy())?;
        }
    }
    writer.flush()?;

    Ok(())
}
