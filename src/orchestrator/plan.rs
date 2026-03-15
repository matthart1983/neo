use crate::agents::AgentId;

/// A step in an execution plan produced by the Planner agent.
#[derive(Debug, Clone)]
pub struct PlanStep {
    pub id: usize,
    pub title: String,
    pub description: String,
    pub agent: AgentId,
    /// IDs of steps that must complete before this one can start.
    pub depends_on: Vec<usize>,
    /// Files this step is expected to touch (informational).
    pub files: Vec<String>,
}

/// An execution plan: an ordered set of steps with dependency edges.
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub steps: Vec<PlanStep>,
}

impl ExecutionPlan {
    /// Return groups of step indices that can execute in parallel.
    /// Each group contains steps whose dependencies are all in prior groups.
    pub fn parallel_groups(&self) -> Vec<Vec<usize>> {
        let total = self.steps.len();
        if total == 0 {
            return vec![];
        }

        let mut completed: Vec<bool> = vec![false; total];
        let mut groups: Vec<Vec<usize>> = Vec::new();

        loop {
            let mut group = Vec::new();
            for (i, step) in self.steps.iter().enumerate() {
                if completed[i] {
                    continue;
                }
                let deps_met = step.depends_on.iter().all(|&dep| {
                    self.steps
                        .iter()
                        .position(|s| s.id == dep)
                        .map(|idx| completed[idx])
                        .unwrap_or(true) // missing dep treated as met
                });
                if deps_met {
                    group.push(i);
                }
            }

            if group.is_empty() {
                // Remaining steps have unresolvable deps — force them through
                for (i, done) in completed.iter().enumerate() {
                    if !done {
                        group.push(i);
                    }
                }
                if group.is_empty() {
                    break;
                }
            }

            for &idx in &group {
                completed[idx] = true;
            }
            groups.push(group);
        }

        groups
    }
}

/// Parse the Planner agent's text output into an ExecutionPlan.
///
/// Expects numbered steps like:
///   1. **Title** — description [agent: coder] [depends: 1, 2] [files: src/foo.rs, src/bar.rs]
///
/// Parsing is best-effort; unparseable lines are treated as Coder tasks.
pub fn parse_plan(text: &str) -> ExecutionPlan {
    let mut steps = Vec::new();
    let mut current_id: usize = 0;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Match numbered lines: "1. ...", "1) ...", "Step 1: ..."
        let (num, rest) = if let Some(rest) = try_strip_numbered(trimmed) {
            current_id += 1;
            (current_id, rest)
        } else if trimmed.starts_with('-') || trimmed.starts_with('*') {
            // Bullet sub-item — skip or treat as continuation
            continue;
        } else {
            continue;
        };

        let title = extract_title(&rest);
        let description = rest.to_string();
        let agent = extract_agent(&rest);
        let depends_on = extract_depends(&rest);
        let files = extract_files(&rest);

        steps.push(PlanStep {
            id: num,
            title,
            description,
            agent,
            depends_on,
            files,
        });
    }

    ExecutionPlan { steps }
}

fn try_strip_numbered(line: &str) -> Option<String> {
    // "1. text" or "1) text" or "Step 1: text"
    let line = line.trim_start();

    if line.to_lowercase().starts_with("step ") {
        let rest = &line[5..];
        if let Some(pos) = rest.find(':') {
            let after = rest[pos + 1..].trim();
            if !after.is_empty() {
                return Some(after.to_string());
            }
        }
    }

    // Digit(s) followed by . or )
    let mut chars = line.chars();
    let first = chars.next()?;
    if !first.is_ascii_digit() {
        return None;
    }

    let mut idx = 1;
    for c in chars {
        if c.is_ascii_digit() {
            idx += 1;
            continue;
        }
        if c == '.' || c == ')' {
            let rest = &line[idx + 1..].trim();
            if !rest.is_empty() {
                return Some(rest.to_string());
            }
        }
        break;
    }
    None
}

fn extract_title(text: &str) -> String {
    // Try to get bold text: **Title**
    if let Some(start) = text.find("**") {
        if let Some(end) = text[start + 2..].find("**") {
            return text[start + 2..start + 2 + end].to_string();
        }
    }
    // Otherwise use first sentence / first 80 chars
    let end = text
        .find('.')
        .or_else(|| text.find('—'))
        .or_else(|| text.find('-'))
        .unwrap_or(text.len().min(80));
    text[..end].trim().to_string()
}

fn extract_agent(text: &str) -> AgentId {
    let lower = text.to_lowercase();
    // [agent: coder]
    if let Some(pos) = lower.find("[agent:") {
        let rest = &lower[pos + 7..];
        if let Some(end) = rest.find(']') {
            let name = rest[..end].trim();
            return match name {
                "planner" => AgentId::Planner,
                "reviewer" => AgentId::Reviewer,
                "debugger" => AgentId::Debugger,
                "tester" => AgentId::Tester,
                "documenter" => AgentId::Documenter,
                "oracle" => AgentId::Oracle,
                _ => AgentId::Coder,
            };
        }
    }
    // Heuristic from keywords
    if lower.contains("review") || lower.contains("check") {
        AgentId::Reviewer
    } else if lower.contains("test") {
        AgentId::Tester
    } else if lower.contains("document") || lower.contains("readme") {
        AgentId::Documenter
    } else if lower.contains("debug") || lower.contains("diagnos") {
        AgentId::Debugger
    } else {
        AgentId::Coder
    }
}

fn extract_depends(text: &str) -> Vec<usize> {
    let lower = text.to_lowercase();
    // [depends: 1, 2] or [deps: 1, 2]
    let start = lower
        .find("[depends:")
        .or_else(|| lower.find("[deps:"));
    if let Some(pos) = start {
        let after_colon = pos + lower[pos..].find(':').unwrap() + 1;
        let rest = &text[after_colon..];
        if let Some(end) = rest.find(']') {
            return rest[..end]
                .split(',')
                .filter_map(|s| s.trim().parse::<usize>().ok())
                .collect();
        }
    }
    vec![]
}

fn extract_files(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    // [files: src/foo.rs, src/bar.rs]
    if let Some(pos) = lower.find("[files:") {
        let rest = &text[pos + 7..];
        if let Some(end) = rest.find(']') {
            return rest[..end]
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
    }
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_plan() {
        let text = r#"
1. **Add struct** — Create the new data model [agent: coder] [files: src/model.rs]
2. **Implement logic** — Wire it up [agent: coder] [depends: 1] [files: src/lib.rs]
3. **Review changes** — Check correctness [agent: reviewer] [depends: 1, 2]
"#;
        let plan = parse_plan(text);
        assert_eq!(plan.steps.len(), 3);
        assert_eq!(plan.steps[0].title, "Add struct");
        assert_eq!(plan.steps[1].depends_on, vec![1]);
        assert_eq!(plan.steps[2].depends_on, vec![1, 2]);
        assert!(matches!(plan.steps[2].agent, AgentId::Reviewer));
    }

    #[test]
    fn test_parallel_groups() {
        let plan = ExecutionPlan {
            steps: vec![
                PlanStep {
                    id: 1,
                    title: "A".into(),
                    description: "".into(),
                    agent: AgentId::Coder,
                    depends_on: vec![],
                    files: vec![],
                },
                PlanStep {
                    id: 2,
                    title: "B".into(),
                    description: "".into(),
                    agent: AgentId::Coder,
                    depends_on: vec![],
                    files: vec![],
                },
                PlanStep {
                    id: 3,
                    title: "C".into(),
                    description: "".into(),
                    agent: AgentId::Reviewer,
                    depends_on: vec![1, 2],
                    files: vec![],
                },
            ],
        };

        let groups = plan.parallel_groups();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].len(), 2); // A and B in parallel
        assert_eq!(groups[1].len(), 1); // C after both
    }
}
