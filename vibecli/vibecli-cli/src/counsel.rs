//! Multi-LLM deliberation engine.
//!
//! Orchestrates structured debates between multiple AI providers, each assigned
//! a distinct role/persona. Supports multi-round deliberation with user
//! interjections, voting, and moderator-driven synthesis.

#![allow(dead_code)] // Module prepared for /counsel REPL command integration

use serde::{Deserialize, Serialize};
use vibe_ai::provider::{Message, MessageRole};

// ── Data Structures ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounselParticipant {
    pub provider_name: String,
    pub model_name: String,
    /// One of: "Expert", "Devil's Advocate", "Skeptic", "Creative",
    /// "Pragmatist", "Researcher", "Custom"
    pub role: String,
    pub persona: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounselResponse {
    pub participant_index: usize,
    pub content: String,
    pub duration_ms: u64,
    pub tokens: Option<usize>,
    pub votes: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounselRound {
    pub round_number: usize,
    pub responses: Vec<CounselResponse>,
    pub user_interjection: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CounselStatus {
    Idle,
    Deliberating,
    AwaitingUser,
    Synthesizing,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounselSession {
    pub id: String,
    pub topic: String,
    pub participants: Vec<CounselParticipant>,
    pub rounds: Vec<CounselRound>,
    pub moderator_index: usize,
    pub status: CounselStatus,
    pub synthesis: Option<String>,
}

// ── Role Personas ──

impl CounselParticipant {
    pub fn system_prompt(&self) -> String {
        if let Some(ref custom) = self.persona {
            return custom.clone();
        }
        match self.role.to_lowercase().as_str() {
            "expert" => format!(
                "You are an expert advisor using {}. Provide thorough, well-reasoned \
                 analysis. Be comprehensive and draw on deep knowledge.",
                self.model_name
            ),
            "devil's advocate" | "devils advocate" => format!(
                "You are a devil's advocate using {}. Challenge assumptions, find \
                 weaknesses, and present counterarguments. Be constructively critical \
                 \u{2014} don't just disagree, explain why alternatives might be better.",
                self.model_name
            ),
            "skeptic" => format!(
                "You are a skeptical analyst using {}. Question the evidence and \
                 reasoning. Ask 'how do we know this?', 'what could go wrong?', and \
                 demand solid justification for claims.",
                self.model_name
            ),
            "creative" => format!(
                "You are a creative thinker using {}. Propose unconventional approaches, \
                 think laterally, and suggest solutions others might overlook. Don't be \
                 constrained by conventional wisdom.",
                self.model_name
            ),
            "pragmatist" => format!(
                "You are a pragmatic advisor using {}. Focus on practical implementation, \
                 feasibility, timelines, and real-world constraints. What actually works \
                 vs what sounds good in theory?",
                self.model_name
            ),
            "researcher" => format!(
                "You are a research-focused analyst using {}. Cite relevant precedents, \
                 patterns, and established knowledge. Ground your analysis in evidence \
                 and established best practices.",
                self.model_name
            ),
            _ => format!(
                "You are a knowledgeable AI assistant using {}. Provide helpful, \
                 accurate analysis.",
                self.model_name
            ),
        }
    }
}

// ── Session Management ──

impl CounselSession {
    pub fn new(
        topic: String,
        participants: Vec<CounselParticipant>,
        moderator_index: usize,
    ) -> Self {
        let id = format!(
            "counsel-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );
        let clamped = if participants.is_empty() {
            0
        } else {
            moderator_index.min(participants.len() - 1)
        };
        Self {
            id,
            topic,
            participants,
            rounds: Vec::new(),
            moderator_index: clamped,
            status: CounselStatus::Idle,
            synthesis: None,
        }
    }

    /// Build the message history for a participant in a given round.
    ///
    /// Round 0 (first): system prompt + topic only.
    /// Round 1+: system prompt + topic + all previous round transcripts.
    pub fn build_messages(
        &self,
        participant_index: usize,
        round_number: usize,
    ) -> Vec<Message> {
        let participant = &self.participants[participant_index];
        let mut messages = vec![Message {
            role: MessageRole::System,
            content: participant.system_prompt(),
        }];

        // Build context from previous rounds
        if round_number > 0 && !self.rounds.is_empty() {
            let mut context = format!("Topic under discussion: {}\n\n", self.topic);
            for round in &self.rounds {
                context.push_str(&format!(
                    "=== Round {} ===\n",
                    round.round_number + 1
                ));
                for resp in &round.responses {
                    let p = &self.participants[resp.participant_index];
                    context.push_str(&format!(
                        "[{} ({}) - {}]:\n{}\n\n",
                        p.provider_name, p.model_name, p.role, resp.content
                    ));
                }
                if let Some(ref interjection) = round.user_interjection {
                    context.push_str(&format!(
                        "[User follow-up]: {}\n\n",
                        interjection
                    ));
                }
            }
            context.push_str(&format!(
                "\n=== Round {} ===\nNow it's your turn. Consider what others \
                 have said. Build on good ideas, challenge weak ones, and add \
                 your unique perspective as a {}.\n",
                round_number + 1,
                participant.role
            ));
            messages.push(Message {
                role: MessageRole::User,
                content: context,
            });
        } else {
            // Round 0: just the topic
            messages.push(Message {
                role: MessageRole::User,
                content: format!(
                    "Please analyze and discuss the following topic:\n\n{}\n\n\
                     Provide your perspective as a {}.",
                    self.topic, participant.role
                ),
            });
        }

        messages
    }

    /// Build synthesis prompt for the moderator.
    pub fn build_synthesis_prompt(&self) -> Vec<Message> {
        let mut transcript = format!(
            "You are the moderator synthesizing a multi-model discussion.\n\n\
             Topic: {}\n\n",
            self.topic
        );
        for round in &self.rounds {
            transcript.push_str(&format!(
                "=== Round {} ===\n",
                round.round_number + 1
            ));
            for resp in &round.responses {
                let p = &self.participants[resp.participant_index];
                transcript.push_str(&format!(
                    "[{} ({}) - {} | Votes: {:+}]:\n{}\n\n",
                    p.provider_name, p.model_name, p.role, resp.votes, resp.content
                ));
            }
            if let Some(ref interjection) = round.user_interjection {
                transcript.push_str(&format!("[User]: {}\n\n", interjection));
            }
        }

        vec![
            Message {
                role: MessageRole::System,
                content: "You are a wise moderator synthesizing a multi-expert AI \
                          discussion. Produce a balanced, comprehensive summary."
                    .to_string(),
            },
            Message {
                role: MessageRole::User,
                content: format!(
                    "{}\n\nPlease synthesize this discussion into:\n\
                     1. **Consensus**: Points all participants agree on\n\
                     2. **Disagreements**: Key areas of disagreement and the \
                        arguments for each side\n\
                     3. **Key Insights**: The most valuable insights from the \
                        discussion\n\
                     4. **Recommendation**: Your balanced recommendation \
                        considering all perspectives\n\
                     5. **Open Questions**: Unresolved questions that need \
                        further exploration",
                    transcript
                ),
            },
        ]
    }

    /// Record a completed round.
    pub fn add_round(&mut self, responses: Vec<CounselResponse>) {
        let round_number = self.rounds.len();
        self.rounds.push(CounselRound {
            round_number,
            responses,
            user_interjection: None,
        });
        self.status = CounselStatus::AwaitingUser;
    }

    /// Add user interjection to the latest round.
    pub fn inject_user_message(&mut self, message: String) {
        if let Some(round) = self.rounds.last_mut() {
            round.user_interjection = Some(message);
        }
    }

    /// Vote on a response.
    pub fn vote(
        &mut self,
        round_index: usize,
        participant_index: usize,
        delta: i32,
    ) {
        if let Some(round) = self.rounds.get_mut(round_index) {
            if let Some(resp) = round
                .responses
                .iter_mut()
                .find(|r| r.participant_index == participant_index)
            {
                resp.votes += delta;
            }
        }
    }

    /// Set synthesis result.
    pub fn set_synthesis(&mut self, synthesis: String) {
        self.synthesis = Some(synthesis);
        self.status = CounselStatus::Complete;
    }

    /// Get current round number (0-indexed, for the NEXT round to run).
    pub fn current_round(&self) -> usize {
        self.rounds.len()
    }
}

/// Available counsel roles.
pub fn available_roles() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Expert", "Thorough, well-reasoned analysis"),
        ("Devil's Advocate", "Challenges assumptions, finds flaws"),
        ("Skeptic", "Questions evidence and reasoning"),
        ("Creative", "Unconventional approaches, lateral thinking"),
        ("Pragmatist", "Practical implementation focus"),
        ("Researcher", "Evidence-based, cites precedents"),
        ("Custom", "User-defined persona"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_participants() -> Vec<CounselParticipant> {
        vec![
            CounselParticipant {
                provider_name: "claude".into(),
                model_name: "claude-3.5-sonnet".into(),
                role: "Expert".into(),
                persona: None,
            },
            CounselParticipant {
                provider_name: "openai".into(),
                model_name: "gpt-4".into(),
                role: "Skeptic".into(),
                persona: None,
            },
            CounselParticipant {
                provider_name: "gemini".into(),
                model_name: "gemini-pro".into(),
                role: "Creative".into(),
                persona: None,
            },
        ]
    }

    #[test]
    fn test_session_creation() {
        let session =
            CounselSession::new("Should we use Rust?".into(), make_participants(), 0);
        assert!(session.id.starts_with("counsel-"));
        assert_eq!(session.participants.len(), 3);
        assert_eq!(session.status, CounselStatus::Idle);
        assert!(session.rounds.is_empty());
    }

    #[test]
    fn test_round1_messages_topic_only() {
        let session =
            CounselSession::new("Is Rust good?".into(), make_participants(), 0);
        let msgs = session.build_messages(0, 0);
        assert_eq!(msgs.len(), 2); // system + user
        assert!(msgs[0].content.contains("expert"));
        assert!(msgs[1].content.contains("Is Rust good?"));
        // Should NOT contain "Round 1" transcript (no previous rounds)
        assert!(!msgs[1].content.contains("=== Round"));
    }

    #[test]
    fn test_round2_includes_previous() {
        let mut session =
            CounselSession::new("Test topic".into(), make_participants(), 0);
        session.add_round(vec![
            CounselResponse {
                participant_index: 0,
                content: "Claude says yes".into(),
                duration_ms: 100,
                tokens: Some(10),
                votes: 0,
            },
            CounselResponse {
                participant_index: 1,
                content: "GPT says maybe".into(),
                duration_ms: 200,
                tokens: Some(15),
                votes: 0,
            },
        ]);
        let msgs = session.build_messages(0, 1);
        assert_eq!(msgs.len(), 2);
        assert!(msgs[1].content.contains("Claude says yes"));
        assert!(msgs[1].content.contains("GPT says maybe"));
        assert!(msgs[1].content.contains("Round 1"));
    }

    #[test]
    fn test_user_interjection() {
        let mut session =
            CounselSession::new("Topic".into(), make_participants(), 0);
        session.add_round(vec![CounselResponse {
            participant_index: 0,
            content: "Response".into(),
            duration_ms: 100,
            tokens: None,
            votes: 0,
        }]);
        session.inject_user_message("What about performance?".into());
        assert_eq!(
            session.rounds[0].user_interjection,
            Some("What about performance?".into())
        );
        // Next round should include the interjection
        let msgs = session.build_messages(0, 1);
        assert!(msgs[1].content.contains("What about performance?"));
    }

    #[test]
    fn test_voting() {
        let mut session =
            CounselSession::new("Topic".into(), make_participants(), 0);
        session.add_round(vec![
            CounselResponse {
                participant_index: 0,
                content: "A".into(),
                duration_ms: 100,
                tokens: None,
                votes: 0,
            },
            CounselResponse {
                participant_index: 1,
                content: "B".into(),
                duration_ms: 100,
                tokens: None,
                votes: 0,
            },
        ]);
        session.vote(0, 0, 1);
        session.vote(0, 0, 1);
        session.vote(0, 1, -1);
        assert_eq!(session.rounds[0].responses[0].votes, 2);
        assert_eq!(session.rounds[0].responses[1].votes, -1);
    }

    #[test]
    fn test_synthesis_prompt() {
        let mut session =
            CounselSession::new("Test".into(), make_participants(), 0);
        session.add_round(vec![
            CounselResponse {
                participant_index: 0,
                content: "Expert view".into(),
                duration_ms: 100,
                tokens: None,
                votes: 2,
            },
            CounselResponse {
                participant_index: 1,
                content: "Skeptic view".into(),
                duration_ms: 200,
                tokens: None,
                votes: -1,
            },
        ]);
        let msgs = session.build_synthesis_prompt();
        assert_eq!(msgs.len(), 2);
        assert!(msgs[1].content.contains("Expert view"));
        assert!(msgs[1].content.contains("Skeptic view"));
        assert!(msgs[1].content.contains("Votes: +2"));
        assert!(msgs[1].content.contains("Votes: -1"));
        assert!(msgs[1].content.contains("Consensus"));
    }

    #[test]
    fn test_set_synthesis() {
        let mut session =
            CounselSession::new("Topic".into(), make_participants(), 0);
        session.set_synthesis("Final answer".into());
        assert_eq!(session.status, CounselStatus::Complete);
        assert_eq!(session.synthesis, Some("Final answer".into()));
    }

    #[test]
    fn test_persona_system_prompts() {
        let expert = CounselParticipant {
            provider_name: "test".into(),
            model_name: "m".into(),
            role: "Expert".into(),
            persona: None,
        };
        assert!(expert.system_prompt().contains("expert"));

        let devil = CounselParticipant {
            provider_name: "test".into(),
            model_name: "m".into(),
            role: "Devil's Advocate".into(),
            persona: None,
        };
        assert!(devil.system_prompt().contains("devil's advocate"));

        let custom = CounselParticipant {
            provider_name: "test".into(),
            model_name: "m".into(),
            role: "Custom".into(),
            persona: Some("You are a pirate".into()),
        };
        assert_eq!(custom.system_prompt(), "You are a pirate");
    }

    #[test]
    fn test_available_roles() {
        let roles = available_roles();
        assert_eq!(roles.len(), 7);
        assert!(roles.iter().any(|(name, _)| *name == "Expert"));
        assert!(roles.iter().any(|(name, _)| *name == "Devil's Advocate"));
    }

    #[test]
    fn test_current_round() {
        let mut session =
            CounselSession::new("T".into(), make_participants(), 0);
        assert_eq!(session.current_round(), 0);
        session.add_round(vec![]);
        assert_eq!(session.current_round(), 1);
        session.add_round(vec![]);
        assert_eq!(session.current_round(), 2);
    }

    #[test]
    fn test_status_transitions() {
        let mut session =
            CounselSession::new("T".into(), make_participants(), 0);
        assert_eq!(session.status, CounselStatus::Idle);
        session.status = CounselStatus::Deliberating;
        session.add_round(vec![]);
        assert_eq!(session.status, CounselStatus::AwaitingUser);
        session.set_synthesis("Done".into());
        assert_eq!(session.status, CounselStatus::Complete);
    }
}
