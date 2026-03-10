#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QuestionCategory {
    Scope,
    Architecture,
    Dependencies,
    Testing,
    Performance,
    Security,
    Deployment,
    Style,
    Compatibility,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    Questioning,
    Answered,
    PlanReady,
    Executing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    pub id: String,
    pub text: String,
    pub category: QuestionCategory,
    pub options: Option<Vec<String>>,
    pub default_answer: String,
    pub priority: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Answer {
    pub question_id: String,
    pub response_text: String,
    pub selected_option: Option<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub description: String,
    pub file_changes: Vec<String>,
    pub dependencies: Vec<String>,
    pub estimated_effort: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MegaPlan {
    pub title: String,
    pub steps: Vec<PlanStep>,
    pub estimated_files: usize,
    pub estimated_lines: usize,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarificationSession {
    pub task_description: String,
    pub questions: Vec<Question>,
    pub answers: HashMap<String, Answer>,
    pub status: SessionStatus,
    pub generated_plan: Option<MegaPlan>,
}

pub struct ClarifyingEngine {
    session: ClarificationSession,
}

impl ClarifyingEngine {
    pub fn new(task_description: &str) -> Self {
        let questions = Self::generate_questions(task_description);
        let session = ClarificationSession {
            task_description: task_description.to_string(),
            questions,
            answers: HashMap::new(),
            status: SessionStatus::Questioning,
            generated_plan: None,
        };
        ClarifyingEngine { session }
    }

    pub fn generate_questions(task: &str) -> Vec<Question> {
        let task_lower = task.to_lowercase();
        let mut questions = Vec::new();
        let mut id_counter: usize = 0;

        let mut next_id = |prefix: &str| -> String {
            id_counter += 1;
            format!("{}-{}", prefix, id_counter)
        };

        // Always ask about error handling
        questions.push(Question {
            id: next_id("err"),
            text: "What error handling strategy should be used?".to_string(),
            category: QuestionCategory::Architecture,
            options: Some(vec![
                "Result types with custom errors".to_string(),
                "anyhow/eyre for application errors".to_string(),
                "thiserror for library errors".to_string(),
                "Panic on unrecoverable errors only".to_string(),
            ]),
            default_answer: "Result types with custom errors".to_string(),
            priority: 1,
        });

        // Always ask about testing approach
        questions.push(Question {
            id: next_id("test"),
            text: "What testing approach is preferred?".to_string(),
            category: QuestionCategory::Testing,
            options: Some(vec![
                "Unit tests only".to_string(),
                "Unit + integration tests".to_string(),
                "Full TDD with coverage targets".to_string(),
                "Minimal smoke tests".to_string(),
            ]),
            default_answer: "Unit + integration tests".to_string(),
            priority: 1,
        });

        // API-related questions
        if task_lower.contains("api") {
            questions.push(Question {
                id: next_id("api"),
                text: "Should the API be REST or GraphQL?".to_string(),
                category: QuestionCategory::Architecture,
                options: Some(vec![
                    "REST".to_string(),
                    "GraphQL".to_string(),
                    "gRPC".to_string(),
                    "Hybrid (REST + GraphQL)".to_string(),
                ]),
                default_answer: "REST".to_string(),
                priority: 1,
            });
            questions.push(Question {
                id: next_id("auth"),
                text: "What authentication strategy should be used?".to_string(),
                category: QuestionCategory::Security,
                options: Some(vec![
                    "JWT tokens".to_string(),
                    "OAuth 2.0".to_string(),
                    "API keys".to_string(),
                    "Session-based".to_string(),
                ]),
                default_answer: "JWT tokens".to_string(),
                priority: 2,
            });
        }

        // Database-related questions
        if task_lower.contains("database") || task_lower.contains("db") {
            questions.push(Question {
                id: next_id("db"),
                text: "Should we use SQL or NoSQL?".to_string(),
                category: QuestionCategory::Architecture,
                options: Some(vec![
                    "SQL (PostgreSQL)".to_string(),
                    "SQL (SQLite)".to_string(),
                    "NoSQL (MongoDB)".to_string(),
                    "NoSQL (Redis)".to_string(),
                ]),
                default_answer: "SQL (PostgreSQL)".to_string(),
                priority: 1,
            });
            questions.push(Question {
                id: next_id("migrate"),
                text: "What migration strategy should be used?".to_string(),
                category: QuestionCategory::Deployment,
                options: Some(vec![
                    "Incremental migrations".to_string(),
                    "Schema versioning".to_string(),
                    "Blue-green deployment".to_string(),
                ]),
                default_answer: "Incremental migrations".to_string(),
                priority: 2,
            });
        }

        // Test-related questions
        if task_lower.contains("test") {
            questions.push(Question {
                id: next_id("testtype"),
                text: "Should we focus on unit tests or integration tests?".to_string(),
                category: QuestionCategory::Testing,
                options: Some(vec![
                    "Unit tests".to_string(),
                    "Integration tests".to_string(),
                    "Both equally".to_string(),
                    "End-to-end tests".to_string(),
                ]),
                default_answer: "Both equally".to_string(),
                priority: 1,
            });
            questions.push(Question {
                id: next_id("coverage"),
                text: "What code coverage target should we aim for?".to_string(),
                category: QuestionCategory::Testing,
                options: Some(vec![
                    "60%".to_string(),
                    "80%".to_string(),
                    "90%".to_string(),
                    "100%".to_string(),
                ]),
                default_answer: "80%".to_string(),
                priority: 3,
            });
        }

        // Refactor-related questions
        if task_lower.contains("refactor") {
            questions.push(Question {
                id: next_id("scope"),
                text: "What is the scope of the refactoring?".to_string(),
                category: QuestionCategory::Scope,
                options: Some(vec![
                    "Single module".to_string(),
                    "Multiple modules".to_string(),
                    "Entire crate".to_string(),
                    "Cross-crate".to_string(),
                ]),
                default_answer: "Single module".to_string(),
                priority: 1,
            });
            questions.push(Question {
                id: next_id("compat"),
                text: "Must backward compatibility be maintained?".to_string(),
                category: QuestionCategory::Compatibility,
                options: Some(vec![
                    "Yes, fully backward compatible".to_string(),
                    "Minor breaking changes allowed".to_string(),
                    "Major version bump acceptable".to_string(),
                ]),
                default_answer: "Yes, fully backward compatible".to_string(),
                priority: 1,
            });
        }

        // Performance-related questions
        if task_lower.contains("performance") || task_lower.contains("optimize") {
            questions.push(Question {
                id: next_id("perf"),
                text: "What is the primary performance concern?".to_string(),
                category: QuestionCategory::Performance,
                options: Some(vec![
                    "Latency".to_string(),
                    "Throughput".to_string(),
                    "Memory usage".to_string(),
                    "Startup time".to_string(),
                ]),
                default_answer: "Latency".to_string(),
                priority: 1,
            });
        }

        // Security-related questions
        if task_lower.contains("security") || task_lower.contains("auth") {
            questions.push(Question {
                id: next_id("sec"),
                text: "What security standards must be met?".to_string(),
                category: QuestionCategory::Security,
                options: Some(vec![
                    "OWASP Top 10 compliance".to_string(),
                    "SOC 2 compliance".to_string(),
                    "Basic input validation".to_string(),
                    "Full penetration testing".to_string(),
                ]),
                default_answer: "OWASP Top 10 compliance".to_string(),
                priority: 1,
            });
        }

        // Deploy-related questions
        if task_lower.contains("deploy") || task_lower.contains("production") {
            questions.push(Question {
                id: next_id("deploy"),
                text: "What is the target deployment environment?".to_string(),
                category: QuestionCategory::Deployment,
                options: Some(vec![
                    "Docker/Kubernetes".to_string(),
                    "Serverless (AWS Lambda)".to_string(),
                    "Bare metal".to_string(),
                    "Edge (Cloudflare Workers)".to_string(),
                ]),
                default_answer: "Docker/Kubernetes".to_string(),
                priority: 2,
            });
        }

        // Sort by priority (lower number = higher priority)
        questions.sort_by_key(|q| q.priority);
        questions
    }

    pub fn submit_answer(&mut self, question_id: &str, answer: Answer) -> Result<(), String> {
        let exists = self.session.questions.iter().any(|q| q.id == question_id);
        if !exists {
            return Err(format!("Question '{}' not found", question_id));
        }
        self.session.answers.insert(question_id.to_string(), answer);
        self.update_status();
        Ok(())
    }

    pub fn all_answered(&self) -> bool {
        self.session
            .questions
            .iter()
            .all(|q| self.session.answers.contains_key(&q.id))
    }

    pub fn get_unanswered(&self) -> Vec<&Question> {
        self.session
            .questions
            .iter()
            .filter(|q| !self.session.answers.contains_key(&q.id))
            .collect()
    }

    pub fn skip_question(&mut self, question_id: &str) -> Result<(), String> {
        let question = self
            .session
            .questions
            .iter()
            .find(|q| q.id == question_id)
            .ok_or_else(|| format!("Question '{}' not found", question_id))?
            .clone();

        let answer = Answer {
            question_id: question_id.to_string(),
            response_text: question.default_answer.clone(),
            selected_option: Some(question.default_answer),
            confidence: 0.5,
        };
        self.session.answers.insert(question_id.to_string(), answer);
        self.update_status();
        Ok(())
    }

    pub fn generate_plan(&mut self) -> Result<MegaPlan, String> {
        let critical_unanswered: Vec<_> = self
            .session
            .questions
            .iter()
            .filter(|q| q.priority == 1 && !self.session.answers.contains_key(&q.id))
            .collect();

        if !critical_unanswered.is_empty() {
            let names: Vec<_> = critical_unanswered.iter().map(|q| q.id.clone()).collect();
            return Err(format!(
                "Critical questions unanswered: {}",
                names.join(", ")
            ));
        }

        let task = &self.session.task_description;
        let task_lower = task.to_lowercase();

        let risk_level = self.assess_risk(&task_lower);
        let steps = self.build_plan_steps(&task_lower);
        let estimated_files = steps.iter().map(|s| s.file_changes.len().max(1)).sum();
        let estimated_lines = steps.len() * 150;

        let plan = MegaPlan {
            title: format!("Plan: {}", truncate_str(task, 60)),
            steps,
            estimated_files,
            estimated_lines,
            risk_level,
        };

        self.session.generated_plan = Some(plan.clone());
        self.session.status = SessionStatus::PlanReady;
        Ok(plan)
    }

    pub fn get_plan_summary(&self) -> String {
        match &self.session.generated_plan {
            None => "No plan generated yet.".to_string(),
            Some(plan) => {
                let mut summary = String::with_capacity(512);
                summary.push_str(&format!("# {}\n\n", plan.title));
                summary.push_str(&format!("Risk: {:?}\n", plan.risk_level));
                summary.push_str(&format!(
                    "Estimated: {} files, {} lines\n\n",
                    plan.estimated_files, plan.estimated_lines
                ));
                summary.push_str("## Steps\n\n");
                for (i, step) in plan.steps.iter().enumerate() {
                    summary.push_str(&format!(
                        "{}. {} (effort: {})\n",
                        i + 1,
                        step.description,
                        step.estimated_effort
                    ));
                    if !step.file_changes.is_empty() {
                        summary.push_str(&format!(
                            "   Files: {}\n",
                            step.file_changes.join(", ")
                        ));
                    }
                    if !step.dependencies.is_empty() {
                        summary.push_str(&format!(
                            "   Depends on: {}\n",
                            step.dependencies.join(", ")
                        ));
                    }
                }
                summary
            }
        }
    }

    pub fn session(&self) -> &ClarificationSession {
        &self.session
    }

    fn update_status(&mut self) {
        if self.all_answered() {
            self.session.status = SessionStatus::Answered;
        } else {
            self.session.status = SessionStatus::Questioning;
        }
    }

    fn assess_risk(&self, task_lower: &str) -> RiskLevel {
        let mut risk_score: u32 = 0;

        if task_lower.contains("database") || task_lower.contains("migration") {
            risk_score += 2;
        }
        if task_lower.contains("security") || task_lower.contains("auth") {
            risk_score += 2;
        }
        if task_lower.contains("refactor") {
            risk_score += 1;
        }
        if task_lower.contains("deploy") || task_lower.contains("production") {
            risk_score += 2;
        }
        if task_lower.contains("api") {
            risk_score += 1;
        }
        if task_lower.contains("delete") || task_lower.contains("remove") {
            risk_score += 1;
        }

        // Answers with low confidence increase risk
        let low_confidence_count = self
            .session
            .answers
            .values()
            .filter(|a| a.confidence < 0.5)
            .count();
        risk_score += low_confidence_count as u32;

        match risk_score {
            0..=1 => RiskLevel::Low,
            2..=3 => RiskLevel::Medium,
            4..=5 => RiskLevel::High,
            _ => RiskLevel::Critical,
        }
    }

    fn build_plan_steps(&self, task_lower: &str) -> Vec<PlanStep> {
        let mut steps = Vec::new();

        // Analysis step is always first
        steps.push(PlanStep {
            description: "Analyze existing codebase and identify affected areas".to_string(),
            file_changes: vec![],
            dependencies: vec![],
            estimated_effort: "15 min".to_string(),
        });

        if task_lower.contains("api") {
            let api_style = self.answer_text_for_category(QuestionCategory::Architecture);
            steps.push(PlanStep {
                description: format!(
                    "Design {} endpoint structure",
                    api_style.as_deref().unwrap_or("API")
                ),
                file_changes: vec!["src/routes.rs".to_string(), "src/handlers.rs".to_string()],
                dependencies: vec!["step-1".to_string()],
                estimated_effort: "30 min".to_string(),
            });
        }

        if task_lower.contains("database") || task_lower.contains("db") {
            steps.push(PlanStep {
                description: "Set up database schema and migrations".to_string(),
                file_changes: vec![
                    "src/models.rs".to_string(),
                    "migrations/".to_string(),
                ],
                dependencies: vec!["step-1".to_string()],
                estimated_effort: "45 min".to_string(),
            });
        }

        if task_lower.contains("refactor") {
            steps.push(PlanStep {
                description: "Refactor target modules with backward compatibility".to_string(),
                file_changes: vec!["src/lib.rs".to_string()],
                dependencies: vec!["step-1".to_string()],
                estimated_effort: "60 min".to_string(),
            });
        }

        // Implementation step
        steps.push(PlanStep {
            description: "Implement core logic and business rules".to_string(),
            file_changes: vec!["src/lib.rs".to_string()],
            dependencies: if steps.len() > 1 {
                vec![format!("step-{}", steps.len())]
            } else {
                vec!["step-1".to_string()]
            },
            estimated_effort: "45 min".to_string(),
        });

        // Testing step
        steps.push(PlanStep {
            description: "Write tests according to chosen strategy".to_string(),
            file_changes: vec!["tests/".to_string()],
            dependencies: vec![format!("step-{}", steps.len())],
            estimated_effort: "30 min".to_string(),
        });

        // Review step
        steps.push(PlanStep {
            description: "Review, lint, and finalize changes".to_string(),
            file_changes: vec![],
            dependencies: vec![format!("step-{}", steps.len())],
            estimated_effort: "15 min".to_string(),
        });

        steps
    }

    fn answer_text_for_category(&self, category: QuestionCategory) -> Option<String> {
        for q in &self.session.questions {
            if q.category == category {
                if let Some(answer) = self.session.answers.get(&q.id) {
                    return Some(answer.response_text.clone());
                }
            }
        }
        None
    }
}

fn truncate_str(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        &s[..max_len]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_engine_creates_session() {
        let engine = ClarifyingEngine::new("Build a REST API");
        assert_eq!(engine.session().task_description, "Build a REST API");
        assert_eq!(engine.session().status, SessionStatus::Questioning);
        assert!(!engine.session().questions.is_empty());
    }

    #[test]
    fn test_always_generates_error_handling_question() {
        let questions = ClarifyingEngine::generate_questions("simple task");
        assert!(questions.iter().any(|q| q.text.contains("error handling")));
    }

    #[test]
    fn test_always_generates_testing_question() {
        let questions = ClarifyingEngine::generate_questions("simple task");
        assert!(questions.iter().any(|q| q.text.contains("testing approach")));
    }

    #[test]
    fn test_api_task_generates_api_questions() {
        let questions = ClarifyingEngine::generate_questions("Build an API service");
        assert!(questions.iter().any(|q| q.text.contains("REST or GraphQL")));
        assert!(questions.iter().any(|q| q.text.contains("authentication")));
    }

    #[test]
    fn test_database_task_generates_db_questions() {
        let questions = ClarifyingEngine::generate_questions("Set up database layer");
        assert!(questions.iter().any(|q| q.text.contains("SQL or NoSQL")));
        assert!(questions.iter().any(|q| q.text.contains("migration")));
    }

    #[test]
    fn test_test_task_generates_test_questions() {
        let questions = ClarifyingEngine::generate_questions("Add test coverage");
        assert!(questions.iter().any(|q| q.text.contains("unit tests or integration")));
        assert!(questions.iter().any(|q| q.text.contains("coverage target")));
    }

    #[test]
    fn test_refactor_task_generates_scope_questions() {
        let questions = ClarifyingEngine::generate_questions("Refactor the module");
        assert!(questions.iter().any(|q| q.text.contains("scope")));
        assert!(questions.iter().any(|q| q.text.contains("backward compatibility")));
    }

    #[test]
    fn test_submit_answer_success() {
        let mut engine = ClarifyingEngine::new("simple task");
        let qid = engine.session().questions[0].id.clone();
        let answer = Answer {
            question_id: qid.clone(),
            response_text: "Custom errors".to_string(),
            selected_option: Some("Result types with custom errors".to_string()),
            confidence: 0.9,
        };
        assert!(engine.submit_answer(&qid, answer).is_ok());
        assert!(engine.session().answers.contains_key(&qid));
    }

    #[test]
    fn test_submit_answer_invalid_id() {
        let mut engine = ClarifyingEngine::new("simple task");
        let answer = Answer {
            question_id: "nonexistent".to_string(),
            response_text: "test".to_string(),
            selected_option: None,
            confidence: 1.0,
        };
        assert!(engine.submit_answer("nonexistent", answer).is_err());
    }

    #[test]
    fn test_all_answered_false_initially() {
        let engine = ClarifyingEngine::new("simple task");
        assert!(!engine.all_answered());
    }

    #[test]
    fn test_all_answered_true_after_answering_all() {
        let mut engine = ClarifyingEngine::new("simple task");
        let ids: Vec<String> = engine.session().questions.iter().map(|q| q.id.clone()).collect();
        for id in &ids {
            let answer = Answer {
                question_id: id.clone(),
                response_text: "yes".to_string(),
                selected_option: None,
                confidence: 0.8,
            };
            engine.submit_answer(id, answer).unwrap();
        }
        assert!(engine.all_answered());
    }

    #[test]
    fn test_get_unanswered_returns_all_initially() {
        let engine = ClarifyingEngine::new("simple task");
        let total = engine.session().questions.len();
        assert_eq!(engine.get_unanswered().len(), total);
    }

    #[test]
    fn test_get_unanswered_decreases_after_answer() {
        let mut engine = ClarifyingEngine::new("simple task");
        let total = engine.session().questions.len();
        let qid = engine.session().questions[0].id.clone();
        let answer = Answer {
            question_id: qid.clone(),
            response_text: "yes".to_string(),
            selected_option: None,
            confidence: 0.8,
        };
        engine.submit_answer(&qid, answer).unwrap();
        assert_eq!(engine.get_unanswered().len(), total - 1);
    }

    #[test]
    fn test_skip_question_uses_default() {
        let mut engine = ClarifyingEngine::new("simple task");
        let qid = engine.session().questions[0].id.clone();
        let default = engine.session().questions[0].default_answer.clone();
        engine.skip_question(&qid).unwrap();
        let answer = engine.session().answers.get(&qid).unwrap();
        assert_eq!(answer.response_text, default);
        assert_eq!(answer.confidence, 0.5);
    }

    #[test]
    fn test_skip_invalid_question_fails() {
        let mut engine = ClarifyingEngine::new("simple task");
        assert!(engine.skip_question("nonexistent").is_err());
    }

    #[test]
    fn test_status_transitions_to_answered() {
        let mut engine = ClarifyingEngine::new("simple task");
        let ids: Vec<String> = engine.session().questions.iter().map(|q| q.id.clone()).collect();
        for id in &ids {
            engine.skip_question(id).unwrap();
        }
        assert_eq!(engine.session().status, SessionStatus::Answered);
    }

    #[test]
    fn test_generate_plan_fails_without_critical_answers() {
        let mut engine = ClarifyingEngine::new("Build an API");
        assert!(engine.generate_plan().is_err());
    }

    #[test]
    fn test_generate_plan_succeeds_when_all_answered() {
        let mut engine = ClarifyingEngine::new("Build an API");
        let ids: Vec<String> = engine.session().questions.iter().map(|q| q.id.clone()).collect();
        for id in &ids {
            engine.skip_question(id).unwrap();
        }
        let plan = engine.generate_plan();
        assert!(plan.is_ok());
        let plan = plan.unwrap();
        assert!(!plan.title.is_empty());
        assert!(!plan.steps.is_empty());
    }

    #[test]
    fn test_plan_status_becomes_ready() {
        let mut engine = ClarifyingEngine::new("simple task");
        let ids: Vec<String> = engine.session().questions.iter().map(|q| q.id.clone()).collect();
        for id in &ids {
            engine.skip_question(id).unwrap();
        }
        engine.generate_plan().unwrap();
        assert_eq!(engine.session().status, SessionStatus::PlanReady);
    }

    #[test]
    fn test_risk_level_low_for_simple_task() {
        let mut engine = ClarifyingEngine::new("add a utility function");
        let ids: Vec<String> = engine.session().questions.iter().map(|q| q.id.clone()).collect();
        for id in &ids {
            engine.skip_question(id).unwrap();
        }
        let plan = engine.generate_plan().unwrap();
        assert_eq!(plan.risk_level, RiskLevel::Low);
    }

    #[test]
    fn test_risk_level_higher_for_database_security() {
        let mut engine = ClarifyingEngine::new("database migration with security auth");
        let ids: Vec<String> = engine.session().questions.iter().map(|q| q.id.clone()).collect();
        for id in &ids {
            engine.skip_question(id).unwrap();
        }
        let plan = engine.generate_plan().unwrap();
        assert!(matches!(plan.risk_level, RiskLevel::High | RiskLevel::Critical));
    }

    #[test]
    fn test_plan_summary_empty_without_plan() {
        let engine = ClarifyingEngine::new("simple task");
        assert_eq!(engine.get_plan_summary(), "No plan generated yet.");
    }

    #[test]
    fn test_plan_summary_contains_title() {
        let mut engine = ClarifyingEngine::new("simple task");
        let ids: Vec<String> = engine.session().questions.iter().map(|q| q.id.clone()).collect();
        for id in &ids {
            engine.skip_question(id).unwrap();
        }
        engine.generate_plan().unwrap();
        let summary = engine.get_plan_summary();
        assert!(summary.contains("Plan:"));
        assert!(summary.contains("Steps"));
    }

    #[test]
    fn test_questions_sorted_by_priority() {
        let questions = ClarifyingEngine::generate_questions("Build an API with database and deploy to production");
        for window in questions.windows(2) {
            assert!(window[0].priority <= window[1].priority);
        }
    }

    #[test]
    fn test_category_filtering() {
        let questions = ClarifyingEngine::generate_questions("Build an API with security");
        let security_questions: Vec<_> = questions
            .iter()
            .filter(|q| q.category == QuestionCategory::Security)
            .collect();
        assert!(!security_questions.is_empty());
    }

    #[test]
    fn test_performance_task_questions() {
        let questions = ClarifyingEngine::generate_questions("optimize performance of the query engine");
        assert!(questions.iter().any(|q| q.category == QuestionCategory::Performance));
    }

    #[test]
    fn test_deploy_task_questions() {
        let questions = ClarifyingEngine::generate_questions("deploy to production");
        assert!(questions.iter().any(|q| q.category == QuestionCategory::Deployment));
    }

    #[test]
    fn test_plan_has_analysis_step() {
        let mut engine = ClarifyingEngine::new("simple task");
        let ids: Vec<String> = engine.session().questions.iter().map(|q| q.id.clone()).collect();
        for id in &ids {
            engine.skip_question(id).unwrap();
        }
        let plan = engine.generate_plan().unwrap();
        assert!(plan.steps[0].description.contains("Analyze"));
    }

    #[test]
    fn test_plan_estimated_files_positive() {
        let mut engine = ClarifyingEngine::new("Build an API with database");
        let ids: Vec<String> = engine.session().questions.iter().map(|q| q.id.clone()).collect();
        for id in &ids {
            engine.skip_question(id).unwrap();
        }
        let plan = engine.generate_plan().unwrap();
        assert!(plan.estimated_files > 0);
        assert!(plan.estimated_lines > 0);
    }

    #[test]
    fn test_session_serialization() {
        let engine = ClarifyingEngine::new("simple task");
        let json = serde_json::to_string(engine.session()).unwrap();
        let deserialized: ClarificationSession = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.task_description, "simple task");
        assert_eq!(deserialized.questions.len(), engine.session().questions.len());
    }
}
