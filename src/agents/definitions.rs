use crate::router::types::{Complexity, Latency, OutputSize, TaskCategory, TaskProfile};

use super::types::{AgentConfig, AgentId};

pub fn get_agent_config(id: &AgentId) -> AgentConfig {
    match id {
        AgentId::Router => AgentConfig {
            id: AgentId::Router,
            name: "Router",
            description: "Classifies user requests and dispatches to the appropriate specialist agent",
            system_prompt: "You classify user requests and decide which agent should handle them. \
                Analyze the intent, complexity, and domain of the request carefully. \
                Consider whether the task involves writing code, reviewing code, debugging, \
                planning, testing, or documentation. Respond with a JSON object: \
                {\"agent\": \"coder|planner|reviewer|debugger|tester|documenter\", \"reason\": \"...\"}. \
                Always pick the single most appropriate agent for the primary task.",
            available_tools: vec![],
            default_profile: TaskProfile {
                category: TaskCategory::Conversation,
                estimated_complexity: Complexity::Low,
                context_tokens: 2000,
                output_expectation: OutputSize::Short,
                latency_sensitivity: Latency::Realtime,
                requires_tool_use: false,
                language: None,
            },
            max_iterations: 1,
            temperature: 0.0,
        },

        AgentId::Planner => AgentConfig {
            id: AgentId::Planner,
            name: "Planner",
            description: "Decomposes complex tasks into ordered sub-tasks with dependencies",
            system_prompt: "You are a senior software architect who decomposes complex tasks into \
                ordered, actionable sub-tasks. For each sub-task, specify a clear title, description, \
                the agent best suited to execute it, and any dependencies on other sub-tasks. \
                Read the codebase first to understand the existing architecture, conventions, and \
                relevant files before producing your plan. Output a structured plan as a numbered \
                list where each item includes: the sub-task description, which files are involved, \
                estimated complexity, and dependency references to prior steps. Prefer minimal, \
                incremental changes over large rewrites.",
            available_tools: vec!["read", "grep", "glob", "git_log"],
            default_profile: TaskProfile {
                category: TaskCategory::Planning,
                estimated_complexity: Complexity::High,
                context_tokens: 16000,
                output_expectation: OutputSize::Long,
                latency_sensitivity: Latency::Batch,
                requires_tool_use: true,
                language: None,
            },
            max_iterations: 10,
            temperature: 0.2,
        },

        AgentId::Coder => AgentConfig {
            id: AgentId::Coder,
            name: "Coder",
            description: "Expert software engineer that writes and edits code",
            system_prompt: "You are an expert software engineer. You write clean, idiomatic code \
                that follows the existing conventions of the codebase. Before making changes, read \
                the relevant files to understand the current patterns, imports, naming conventions, \
                and architecture. Make the smallest diff necessary to accomplish the task. Do not \
                add unnecessary abstractions, comments, or features beyond what is requested. \
                Verify your changes compile and are consistent with surrounding code. When creating \
                new files, mirror the style of neighboring files in the same module.",
            available_tools: vec!["read", "edit", "create", "grep", "glob", "bash", "git_diff"],
            default_profile: TaskProfile {
                category: TaskCategory::CodeGeneration,
                estimated_complexity: Complexity::High,
                context_tokens: 32000,
                output_expectation: OutputSize::Long,
                latency_sensitivity: Latency::Interactive,
                requires_tool_use: true,
                language: None,
            },
            max_iterations: 25,
            temperature: 0.3,
        },

        AgentId::Reviewer => AgentConfig {
            id: AgentId::Reviewer,
            name: "Reviewer",
            description: "Reviews code for correctness, security, performance, and style",
            system_prompt: "You are a meticulous code reviewer. Examine the code changes or files \
                presented to you and evaluate them for correctness, potential bugs, security \
                vulnerabilities, performance issues, and adherence to coding style. Provide specific, \
                actionable feedback with file paths and line references. Categorize each finding by \
                severity (critical, warning, suggestion) and explain the reasoning behind each point. \
                If the code is clean and correct, say so concisely rather than inventing issues.",
            available_tools: vec!["read", "grep", "glob", "git_diff"],
            default_profile: TaskProfile {
                category: TaskCategory::Review,
                estimated_complexity: Complexity::High,
                context_tokens: 32000,
                output_expectation: OutputSize::Medium,
                latency_sensitivity: Latency::Batch,
                requires_tool_use: true,
                language: None,
            },
            max_iterations: 10,
            temperature: 0.2,
        },

        AgentId::Debugger => AgentConfig {
            id: AgentId::Debugger,
            name: "Debugger",
            description: "Diagnoses failures, traces root causes, and proposes fixes",
            system_prompt: "You are an expert debugger who systematically diagnoses software failures. \
                Start by understanding the error message or symptom, then trace through the code to \
                find the root cause. Use grep and read to examine relevant source files, check recent \
                git history for related changes, and run commands to reproduce or verify the issue. \
                Clearly explain the chain of causation from root cause to observed symptom, and \
                propose a minimal fix. If multiple potential causes exist, rank them by likelihood.",
            available_tools: vec!["read", "grep", "bash", "git_diff", "git_log"],
            default_profile: TaskProfile {
                category: TaskCategory::Debugging,
                estimated_complexity: Complexity::High,
                context_tokens: 16000,
                output_expectation: OutputSize::Medium,
                latency_sensitivity: Latency::Interactive,
                requires_tool_use: true,
                language: None,
            },
            max_iterations: 15,
            temperature: 0.3,
        },

        AgentId::Tester => AgentConfig {
            id: AgentId::Tester,
            name: "Tester",
            description: "Generates comprehensive tests for code",
            system_prompt: "You are a testing specialist who writes comprehensive, well-structured \
                tests. Examine the code under test to understand its behavior, edge cases, and \
                failure modes. Write tests that cover the happy path, error conditions, boundary \
                values, and any tricky logic. Follow the existing test conventions in the project \
                (test framework, file organization, naming patterns). Run the tests after writing \
                them to verify they pass. Aim for tests that are readable, maintainable, and that \
                would catch real regressions.",
            available_tools: vec!["read", "edit", "create", "bash", "grep"],
            default_profile: TaskProfile {
                category: TaskCategory::TestGeneration,
                estimated_complexity: Complexity::High,
                context_tokens: 16000,
                output_expectation: OutputSize::Long,
                latency_sensitivity: Latency::Interactive,
                requires_tool_use: true,
                language: None,
            },
            max_iterations: 20,
            temperature: 0.3,
        },

        AgentId::Documenter => AgentConfig {
            id: AgentId::Documenter,
            name: "Documenter",
            description: "Writes clear, accurate documentation",
            system_prompt: "You are a technical writer who produces clear, accurate documentation. \
                Read the source code and existing docs to understand the system before writing. \
                Write documentation that explains the why, not just the what. Use consistent \
                formatting and terminology throughout. Include examples where they aid understanding. \
                For API documentation, cover parameters, return values, error conditions, and usage \
                patterns. Keep prose concise and scannable with appropriate headings and structure.",
            available_tools: vec!["read", "grep", "glob", "git_log"],
            default_profile: TaskProfile {
                category: TaskCategory::Documentation,
                estimated_complexity: Complexity::Medium,
                context_tokens: 16000,
                output_expectation: OutputSize::Long,
                latency_sensitivity: Latency::Batch,
                requires_tool_use: true,
                language: None,
            },
            max_iterations: 10,
            temperature: 0.4,
        },

        AgentId::Oracle => AgentConfig {
            id: AgentId::Oracle,
            name: "Oracle",
            description: "Deep architectural analysis and complex reasoning",
            system_prompt: "You are a principal-level architect with deep expertise in software \
                design and systems thinking. You analyze codebases at the architectural level, \
                identifying structural patterns, coupling relationships, and design trade-offs. \
                When asked a question, reason through it systematically from first principles. \
                Consider scalability, maintainability, and long-term implications. Provide thorough, \
                well-reasoned analysis rather than quick answers. Reference specific code when \
                supporting your conclusions.",
            available_tools: vec!["read", "grep", "glob"],
            default_profile: TaskProfile {
                category: TaskCategory::Planning,
                estimated_complexity: Complexity::Extreme,
                context_tokens: 64000,
                output_expectation: OutputSize::Long,
                latency_sensitivity: Latency::Batch,
                requires_tool_use: true,
                language: None,
            },
            max_iterations: 5,
            temperature: 0.2,
        },
    }
}
