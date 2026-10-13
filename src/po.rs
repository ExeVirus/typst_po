use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

#[derive(Clone, Debug)]
pub struct PotEntry {
    pub msgid: String,
    pub references: Vec<String>,
    pub first_line: usize,
}

pub struct PotCatalog {
    // Preserve insertion order and sort by line number
    pub entries: Vec<PotEntry>,
}

impl PotCatalog {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn add(&mut self, msgid: String, reference: String, line: usize) {
        let trimmed = msgid.trim();
        if trimmed.is_empty() {
            return;
        }

        if let Some(existing) = self.entries.iter_mut().find(|e| e.msgid == trimmed) {
            if !existing.references.contains(&reference) {
                existing.references.push(reference);
            }
        } else {
            self.entries.push(PotEntry {
                msgid: trimmed.to_string(),
                references: vec![reference],
                first_line: line,
            });
        }
    }

    pub fn write_to_file<P: AsRef<Path>>(&mut self, path: P) -> std::io::Result<()> {
        // Sort entries strictly by their appearance line in the source file
        self.entries.sort_by_key(|e| e.first_line);

        let mut file = File::create(path)?;
        writeln!(file, "msgid \"\"")?;
        writeln!(file, "msgstr \"\"")?;
        writeln!(file, "\"Content-Type: text/plain; charset=UTF-8\\n\"")?;
        writeln!(file, "\"Content-Transfer-Encoding: 8bit\\n\"\n")?;

        for entry in &self.entries {
            for r in &entry.references {
                writeln!(file, "#: {}", r)?;
            }
            write_po_string(&mut file, "msgid", &entry.msgid)?;
            write_po_string(&mut file, "msgstr", "")?;
            writeln!(file)?;
        }
        Ok(())
    }
}

pub fn parse_po_file<P: AsRef<Path>>(path: P) -> std::io::Result<HashMap<String, String>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut map = HashMap::new();
    let mut current_msgid: Option<String> = None;
    let mut current_msgstr: Option<String> = None;
    let mut in_msgid = false;
    let mut in_msgstr = false;

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();

        if trimmed.starts_with('#') || trimmed.is_empty() {
            if let (Some(id), Some(str_val)) = (current_msgid.take(), current_msgstr.take()) {
                if !id.is_empty() && !str_val.is_empty() {
                    map.insert(id, str_val);
                }
            }
            in_msgid = false;
            in_msgstr = false;
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("msgid ") {
            if let (Some(id), Some(str_val)) = (current_msgid.take(), current_msgstr.take()) {
                if !id.is_empty() && !str_val.is_empty() {
                    map.insert(id, str_val);
                }
            }
            current_msgid = Some(extract_string_literal(rest));
            in_msgid = true;
            in_msgstr = false;
        } else if let Some(rest) = trimmed.strip_prefix("msgstr ") {
            current_msgstr = Some(extract_string_literal(rest));
            in_msgid = false;
            in_msgstr = true;
        } else if trimmed.starts_with('"') {
            let content = extract_string_literal(trimmed);
            if in_msgid {
                if let Some(ref mut id) = current_msgid {
                    id.push_str(&content);
                }
            } else if in_msgstr {
                if let Some(ref mut str_val) = current_msgstr {
                    str_val.push_str(&content);
                }
            }
        }
    }

    if let (Some(id), Some(str_val)) = (current_msgid, current_msgstr) {
        if !id.is_empty() && !str_val.is_empty() {
            map.insert(id, str_val);
        }
    }

    Ok(map)
}

fn write_po_string<W: Write>(w: &mut W, prefix: &str, s: &str) -> std::io::Result<()> {
    // Strip Windows CRLF
    let clean = s.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = clean.split('\n').collect();

    if lines.len() == 1 {
        writeln!(w, "{} \"{}\"", prefix, escape_line(lines[0]))?;
    } else {
        writeln!(w, "{} \"\"", prefix)?;
        for (i, line) in lines.iter().enumerate() {
            let is_last = i + 1 == lines.len();
            if is_last {
                if !line.is_empty() {
                    writeln!(w, "\"{}\"", escape_line(line))?;
                }
            } else {
                writeln!(w, "\"{}\\n\"", escape_line(line))?;
            }
        }
    }
    Ok(())
}

fn escape_line(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn extract_string_literal(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        unescape_po(&trimmed[1..trimmed.len() - 1])
    } else {
        String::new()
    }
}

fn unescape_po(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}