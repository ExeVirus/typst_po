use std::collections::HashMap;
use std::ops::Range;
use typst_syntax::{parse, SyntaxKind, SyntaxNode};

#[derive(Debug, Clone)]
pub struct ExtractChunk {
    pub range: Range<usize>,
    pub text: String,
    pub line: usize,
}

pub fn canonicalize(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

fn normalize_lines(s: &str) -> String {
    canonicalize(s)
        .lines()
        .map(|l| l.trim_end())
        .collect::<Vec<&str>>()
        .join("\n")
        .trim()
        .to_string()
}

pub fn extract_chunks(source: &str) -> Vec<ExtractChunk> {
    let root = parse(source);
    let mut chunks = Vec::new();
    traverse_node(&root, 0, source, &mut chunks);
    chunks.sort_by_key(|c| c.range.start);
    chunks
}

pub fn translate_source(source: &str, translations: &HashMap<String, String>) -> String {
    // Build canonicalized lookup maps (both exact \n and trailing-whitespace normalized)
    let mut canon_translations: HashMap<String, String> = HashMap::new();
    for (k, v) in translations {
        canon_translations.insert(canonicalize(k.trim()), canonicalize(v.trim()));
        canon_translations.insert(normalize_lines(k), canonicalize(v.trim()));
    }

    let chunks = extract_chunks(source);
    let mut replacements: Vec<(Range<usize>, String)> = Vec::new();

    for chunk in chunks {
        let clean_chunk = canonicalize(chunk.text.trim());
        let norm_chunk = normalize_lines(&chunk.text);

        let trans_opt = canon_translations
            .get(&clean_chunk)
            .or_else(|| canon_translations.get(&norm_chunk));

        if let Some(trans) = trans_opt {
            if !trans.is_empty() {
                let trans_formatted = if chunk.text.contains("\r\n") {
                    trans.replace('\n', "\r\n")
                } else {
                    trans.clone()
                };
                replacements.push((chunk.range, trans_formatted));
            }
        }
    }

    // Sort in reverse order to preserve earlier byte offsets
    replacements.sort_by(|a, b| b.0.start.cmp(&a.0.start));

    let mut result = source.to_string();
    for (range, trans) in replacements {
        result.replace_range(range, &trans);
    }
    result
}

fn traverse_node(node: &SyntaxNode, offset: usize, source: &str, chunks: &mut Vec<ExtractChunk>) {
    match node.kind() {
        SyntaxKind::Markup => {
            walk_markup(node, offset, source, chunks);
        }
        SyntaxKind::ContentBlock => {
            walk_content_block(node, offset, source, chunks);
        }
        _ => {
            let mut cur_offset = offset;
            for child in node.children() {
                let child_start = cur_offset;
                let child_end = child_start + child.len();
                cur_offset = child_end;
                traverse_node(child, child_start, source, chunks);
            }
        }
    }
}

fn walk_markup(node: &SyntaxNode, offset: usize, source: &str, chunks: &mut Vec<ExtractChunk>) {
    let mut cur_offset = offset;
    let mut para_start: Option<usize> = None;
    let mut para_end: usize = 0;

    for child in node.children() {
        let child_start = cur_offset;
        let child_end = child_start + child.len();
        cur_offset = child_end;

        let raw = &source[child_start..child_end];
        let trimmed_raw = raw.trim();

        // 1. Comments
        if child.kind() == SyntaxKind::LineComment || child.kind() == SyntaxKind::BlockComment {
            flush_para(source, para_start.take(), para_end, chunks);
            continue;
        }

        // 2. Headings, List items, Enum items
        if child.kind() == SyntaxKind::Heading
            || child.kind() == SyntaxKind::ListItem
            || child.kind() == SyntaxKind::EnumItem
        {
            flush_para(source, para_start.take(), para_end, chunks);
            extract_item_or_heading(child, child_start, source, chunks);
            continue;
        }

        // 3. Paragraph breaks (double newlines)
        if child.kind() == SyntaxKind::Parbreak {
            flush_para(source, para_start.take(), para_end, chunks);
            continue;
        }

        // 4. Content blocks [...]
        if child.kind() == SyntaxKind::ContentBlock {
            flush_para(source, para_start.take(), para_end, chunks);
            walk_content_block(child, child_start, source, chunks);
            continue;
        }

        // 5. Standalone code declarations (#let, #set, #show, #import, #v, #colbreak)
        if is_standalone_code(trimmed_raw) {
            flush_para(source, para_start.take(), para_end, chunks);
            walk_code_for_content(child, child_start, source, chunks);
            continue;
        }

        // 6. Block-level container functions like #columns(2)[...] or #align(center)[...]
        if has_content_block_arg(child) {
            flush_para(source, para_start.take(), para_end, chunks);
            walk_code_for_content(child, child_start, source, chunks);
            continue;
        }

        // 7. Paragraph elements (Text, inline #svg, formatting, #section-title)
        if child.kind() == SyntaxKind::Space {
            if para_start.is_some() {
                para_end = child_end;
            }
        } else {
            if para_start.is_none() {
                para_start = Some(child_start);
            }
            para_end = child_end;
        }
    }

    flush_para(source, para_start.take(), para_end, chunks);
}

fn walk_content_block(node: &SyntaxNode, offset: usize, source: &str, chunks: &mut Vec<ExtractChunk>) {
    let mut cur_offset = offset;
    for child in node.children() {
        let child_start = cur_offset;
        let child_end = child_start + child.len();
        cur_offset = child_end;

        if child.kind() == SyntaxKind::Markup {
            walk_markup(child, child_start, source, chunks);
        } else {
            traverse_node(child, child_start, source, chunks);
        }
    }
}

fn walk_code_for_content(node: &SyntaxNode, offset: usize, source: &str, chunks: &mut Vec<ExtractChunk>) {
    let mut cur_offset = offset;
    for child in node.children() {
        let child_start = cur_offset;
        let child_end = child_start + child.len();
        cur_offset = child_end;

        if child.kind() == SyntaxKind::ContentBlock {
            walk_content_block(child, child_start, source, chunks);
        } else {
            walk_code_for_content(child, child_start, source, chunks);
        }
    }
}

fn extract_item_or_heading(
    node: &SyntaxNode,
    offset: usize,
    source: &str,
    chunks: &mut Vec<ExtractChunk>,
) {
    let mut cur_offset = offset;
    let mut content_start: Option<usize> = None;
    let mut content_end: usize = 0;

    for child in node.children() {
        let child_start = cur_offset;
        let child_end = child_start + child.len();
        cur_offset = child_end;

        let raw = &source[child_start..child_end];
        let trimmed = raw.trim();

        if content_start.is_none()
            && (trimmed.starts_with('=')
                || trimmed.starts_with('-')
                || trimmed.starts_with('+')
                || trimmed.is_empty())
        {
            continue;
        }

        if content_start.is_none() {
            content_start = Some(child_start);
        }
        content_end = child_end;
    }

    flush_para(source, content_start, content_end, chunks);
}

fn has_content_block_arg(node: &SyntaxNode) -> bool {
    for child in node.children() {
        if child.kind() == SyntaxKind::ContentBlock {
            return true;
        }
        if child.kind() == SyntaxKind::Args && has_content_block_arg(child) {
            return true;
        }
    }
    false
}

fn is_standalone_code(raw: &str) -> bool {
    raw.starts_with("#let")
        || raw.starts_with("#set")
        || raw.starts_with("#show")
        || raw.starts_with("#import")
        || raw.starts_with("#v(")
        || raw.starts_with("#colbreak()")
}

fn flush_para(
    source: &str,
    start: Option<usize>,
    end: usize,
    chunks: &mut Vec<ExtractChunk>,
) {
    if let Some(s) = start {
        if s < end && end <= source.len() {
            let slice = &source[s..end];
            let trimmed = slice.trim();
            if !trimmed.is_empty() && trimmed != "#" {
                let leading_ws = slice.len() - slice.trim_start().len();
                let trailing_ws = slice.len() - slice.trim_end().len();
                let actual_start = s + leading_ws;
                let actual_end = end - trailing_ws;

                let line = source[..actual_start].chars().filter(|&c| c == '\n').count() + 1;
                chunks.push(ExtractChunk {
                    range: actual_start..actual_end,
                    text: trimmed.to_string(),
                    line,
                });
            }
        }
    }
}