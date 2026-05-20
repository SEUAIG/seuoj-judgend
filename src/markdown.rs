//! Markdown-based problem file parser and serializer.
//!
//! Parses `problem.md` (YAML frontmatter + Markdown body) into [`ProblemMetadata`],
//! and serializes back to the same format.

use crate::error::{AijError, Result};
use crate::schema::{ProblemExample, ProblemMetadata};
use axum::http::StatusCode;
use serde::Deserialize;

#[derive(Deserialize)]
struct Frontmatter {
    pid: String,
}

struct Section {
    heading: String,
    content: String,
}

/// Parse a `problem.md` file into [`ProblemMetadata`].
pub(crate) fn parse_problem_md(content: &str) -> Result<ProblemMetadata> {
    let content = content.replace("\r\n", "\n");

    let (frontmatter_str, body) = split_frontmatter(&content)?;
    let fm: Frontmatter = serde_yaml::from_str(frontmatter_str).map_err(|e| {
        AijError::Request(
            StatusCode::BAD_REQUEST,
            "INVALID_FRONTMATTER".to_string(),
            format!("Failed to parse YAML frontmatter: {}", e),
        )
    })?;

    let sections = split_h2_sections(body);

    let mut description = String::new();
    let mut input = String::new();
    let mut output = String::new();
    let mut hint = String::new();
    let mut examples: Vec<ProblemExample> = Vec::new();

    for section in &sections {
        let heading = section.heading.trim();
        match heading {
            "题目描述" | "Description" => description = section.content.clone(),
            "输入格式" | "Input" => input = section.content.clone(),
            "输出格式" | "Output" => output = section.content.clone(),
            "提示" | "Hint" => hint = section.content.clone(),
            "样例" | "Examples" => {
                examples = parse_examples(&section.content)?;
            }
            _ => {}
        }
    }

    Ok(ProblemMetadata {
        pid: fm.pid,
        description,
        input,
        output,
        hint,
        example: examples,
    })
}

/// Serialize [`ProblemMetadata`] into `problem.md` format.
pub(crate) fn serialize_problem_md(metadata: &ProblemMetadata) -> Result<String> {
    let mut out = String::new();

    // Frontmatter
    out.push_str("---\n");
    out.push_str(&format!("pid: {}\n", yaml_scalar(&metadata.pid)));
    out.push_str("---\n\n");

    // Sections
    out.push_str("## 题目描述\n\n");
    out.push_str(&metadata.description);
    out.push_str("\n\n");

    out.push_str("## 输入格式\n\n");
    out.push_str(&metadata.input);
    out.push_str("\n\n");

    out.push_str("## 输出格式\n\n");
    out.push_str(&metadata.output);
    out.push_str("\n\n");

    // Examples
    if !metadata.example.is_empty() {
        out.push_str("## 样例\n\n");
        for (i, ex) in metadata.example.iter().enumerate() {
            out.push_str(&format!("### 样例 {}\n\n", i + 1));
            out.push_str("#### 输入\n\n");
            out.push_str("```\n");
            out.push_str(&ex.r#in);
            if !ex.r#in.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("```\n\n");

            out.push_str("#### 输出\n\n");
            out.push_str("```\n");
            out.push_str(&ex.ans);
            if !ex.ans.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("```\n");

            if !ex.description.is_empty() {
                out.push('\n');
                out.push_str(&ex.description);
                out.push('\n');
            }
            out.push('\n');
        }
    }

    // Hint
    out.push_str("## 提示\n\n");
    if !metadata.hint.is_empty() {
        out.push_str(&metadata.hint);
        out.push('\n');
    }

    Ok(out)
}

fn yaml_scalar(s: &str) -> String {
    if s.contains(':') || s.contains('#') || s.contains('\'') || s.contains('"') || s.contains('\n')
    {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        s.to_string()
    }
}

fn split_frontmatter(content: &str) -> Result<(&str, &str)> {
    let content = content.trim_start();
    if !content.starts_with("---") {
        return Err(AijError::Request(
            StatusCode::BAD_REQUEST,
            "MISSING_FRONTMATTER".to_string(),
            "problem.md must start with YAML frontmatter (---)".to_string(),
        ));
    }

    let after_first = &content[3..];
    let after_first = after_first.strip_prefix('\n').unwrap_or(after_first);

    let end_pos = after_first.find("\n---").ok_or_else(|| {
        AijError::Request(
            StatusCode::BAD_REQUEST,
            "INVALID_FRONTMATTER".to_string(),
            "Could not find closing --- for frontmatter".to_string(),
        )
    })?;

    let frontmatter = &after_first[..end_pos];
    let body = &after_first[end_pos + 4..]; // skip "\n---"
    let body = body.strip_prefix('\n').unwrap_or(body);

    Ok((frontmatter, body))
}

fn split_h2_sections(body: &str) -> Vec<Section> {
    let mut sections = Vec::new();
    let mut current_heading: Option<String> = None;
    let mut current_lines: Vec<&str> = Vec::new();

    for line in body.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            if let Some(h) = current_heading.take() {
                sections.push(Section {
                    heading: h,
                    content: trim_section_content(&current_lines),
                });
            }
            current_heading = Some(heading.trim().to_string());
            current_lines.clear();
        } else {
            current_lines.push(line);
        }
    }

    if let Some(h) = current_heading {
        sections.push(Section {
            heading: h,
            content: trim_section_content(&current_lines),
        });
    }

    sections
}

fn trim_section_content(lines: &[&str]) -> String {
    let text = lines.join("\n");
    let trimmed = text.trim();
    trimmed.to_string()
}

fn parse_examples(content: &str) -> Result<Vec<ProblemExample>> {
    let mut examples = Vec::new();
    let blocks = split_by_h3(content);

    for block in &blocks {
        let example = parse_single_example(block)?;
        examples.push(example);
    }

    Ok(examples)
}

fn split_by_h3(content: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    let mut found_first = false;

    for line in content.lines() {
        if line.starts_with("### ") {
            if found_first && !current.is_empty() {
                blocks.push(current.join("\n"));
            }
            found_first = true;
            current.clear();
        } else if found_first {
            current.push(line);
        }
    }

    if found_first && !current.is_empty() {
        blocks.push(current.join("\n"));
    }

    blocks
}

fn parse_single_example(block: &str) -> Result<ProblemExample> {
    let mut input_text = String::new();
    let mut output_text = String::new();
    let mut description = String::new();

    enum SubSection {
        None,
        Input,
        Output,
    }

    let mut state = SubSection::None;
    let mut current_lines: Vec<&str> = Vec::new();

    for line in block.lines() {
        let trimmed = line.trim();
        if trimmed == "#### 输入" || trimmed == "#### Input" {
            state = SubSection::Input;
            current_lines.clear();
        } else if trimmed == "#### 输出" || trimmed == "#### Output" {
            if let SubSection::Input = state {
                input_text = extract_code_block(&current_lines);
            }
            state = SubSection::Output;
            current_lines.clear();
        } else {
            current_lines.push(line);
        }
    }

    // Process the last subsection
    match state {
        SubSection::Input => {
            input_text = extract_code_block(&current_lines);
        }
        SubSection::Output => {
            let (output, desc) = extract_code_block_and_remainder(&current_lines);
            output_text = output;
            description = desc;
        }
        _ => {}
    }

    Ok(ProblemExample {
        r#in: input_text,
        ans: output_text,
        description,
    })
}

fn extract_code_block(lines: &[&str]) -> String {
    let (code, _) = extract_code_block_and_remainder(lines);
    code
}

fn extract_code_block_and_remainder(lines: &[&str]) -> (String, String) {
    let mut in_block = false;
    let mut code_lines: Vec<&str> = Vec::new();
    let mut remainder_lines: Vec<&str> = Vec::new();
    let mut block_ended = false;

    for &line in lines {
        if block_ended {
            remainder_lines.push(line);
            continue;
        }

        let trimmed = line.trim();
        if !in_block && trimmed.starts_with("```") {
            in_block = true;
            continue;
        }

        if in_block {
            if trimmed == "```"
                || (trimmed.starts_with("```") && trimmed.trim_start_matches('`').is_empty())
            {
                block_ended = true;
                continue;
            }
            code_lines.push(line);
        }
    }

    let code = code_lines.join("\n");
    let remainder = remainder_lines.join("\n").trim().to_string();

    (code, remainder)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_trip() {
        let md = r#"---
pid: P1009
---

## 题目描述

给定一个 $n \times m$ 的网格迷宫。

## 输入格式

第一行两个正整数 $n, m$。

## 输出格式

一个整数，表示最少步数。

## 样例

### 样例 1

#### 输入

```
3 3
...
.#.
...
```

#### 输出

```
4
```

### 样例 2

#### 输入

```
1 1
.
```

#### 输出

```
0
```

起点即终点。

## 提示

BFS 即可。
"#;

        let parsed = parse_problem_md(md).unwrap();
        assert_eq!(parsed.pid, "P1009");
        assert_eq!(parsed.description, "给定一个 $n \\times m$ 的网格迷宫。");
        assert_eq!(parsed.input, "第一行两个正整数 $n, m$。");
        assert_eq!(parsed.output, "一个整数，表示最少步数。");
        assert_eq!(parsed.hint, "BFS 即可。");
        assert_eq!(parsed.example.len(), 2);
        assert_eq!(parsed.example[0].r#in, "3 3\n...\n.#.\n...");
        assert_eq!(parsed.example[0].ans, "4");
        assert_eq!(parsed.example[0].description, "");
        assert_eq!(parsed.example[1].r#in, "1 1\n.");
        assert_eq!(parsed.example[1].ans, "0");
        assert_eq!(parsed.example[1].description, "起点即终点。");

        // Round-trip
        let serialized = serialize_problem_md(&parsed).unwrap();
        let reparsed = parse_problem_md(&serialized).unwrap();
        assert_eq!(reparsed.pid, parsed.pid);
        assert_eq!(reparsed.description, parsed.description);
        assert_eq!(reparsed.input, parsed.input);
        assert_eq!(reparsed.output, parsed.output);
        assert_eq!(reparsed.hint, parsed.hint);
        assert_eq!(reparsed.example.len(), parsed.example.len());
        for (a, b) in reparsed.example.iter().zip(parsed.example.iter()) {
            assert_eq!(a.r#in, b.r#in);
            assert_eq!(a.ans, b.ans);
            assert_eq!(a.description, b.description);
        }
    }

    #[test]
    fn test_empty_hint() {
        let md = r#"---
pid: T001
---

## 题目描述

A problem.

## 输入格式

Input spec.

## 输出格式

Output spec.

## 样例

### 样例 1

#### 输入

```
1
```

#### 输出

```
2
```

## 提示
"#;

        let parsed = parse_problem_md(md).unwrap();
        assert_eq!(parsed.hint, "");
    }

    #[test]
    fn test_no_hint_section() {
        let md = r#"---
pid: T002
---

## 题目描述

A problem.

## 输入格式

Input spec.

## 输出格式

Output spec.

## 样例

### 样例 1

#### 输入

```
1
```

#### 输出

```
2
```
"#;

        let parsed = parse_problem_md(md).unwrap();
        assert_eq!(parsed.hint, "");
    }

    #[test]
    fn test_multiline_io() {
        let md = r#"---
pid: T003
---

## 题目描述

Desc.

## 输入格式

Input.

## 输出格式

Output.

## 样例

### 样例 1

#### 输入

```
3 3
1 2 3
4 5 6
7 8 9
```

#### 输出

```
yes
1 2 3
no
```
"#;

        let parsed = parse_problem_md(md).unwrap();
        assert_eq!(parsed.example[0].r#in, "3 3\n1 2 3\n4 5 6\n7 8 9");
        assert_eq!(parsed.example[0].ans, "yes\n1 2 3\nno");
    }

    #[test]
    fn test_english_headings() {
        let md = r#"---
pid: T004
---

## Description

A problem.

## Input

Input spec.

## Output

Output spec.

## Examples

### 样例 1

#### Input

```
1
```

#### Output

```
2
```

## Hint

Use math.
"#;

        let parsed = parse_problem_md(md).unwrap();
        assert_eq!(parsed.description, "A problem.");
        assert_eq!(parsed.input, "Input spec.");
        assert_eq!(parsed.output, "Output spec.");
        assert_eq!(parsed.hint, "Use math.");
        assert_eq!(parsed.example.len(), 1);
    }
}
