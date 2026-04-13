//! alt_explore — Tournament-style candidate scoring and ranking for alternative
//! agent explorations. Scores each candidate on test pass rate, diff size, and
//! compile success, then ranks and optionally disqualifies non-compilers.

/// A single agent-generated patch candidate.
#[derive(Debug, Clone, Default)]
pub struct ExploreCandidate {
    pub id: String,
    pub patch: String,
    pub pass_rate: f32,
    pub diff_lines: usize,
    pub compile_success: bool,
    pub score: f32,
}

impl ExploreCandidate {
    pub fn new(
        id: impl Into<String>,
        patch: impl Into<String>,
        pass_rate: f32,
        diff_lines: usize,
        compile_success: bool,
    ) -> Self {
        Self {
            id: id.into(),
            patch: patch.into(),
            pass_rate,
            diff_lines,
            compile_success,
            score: 0.0,
        }
    }
}

/// Weights used to compute the composite candidate score.
#[derive(Debug, Clone)]
pub struct TournamentConfig {
    /// Weight for test pass rate (0..=1).
    pub pass_rate_weight: f32,
    /// Weight penalizing large diffs (normalised by `diff_normalizer`).
    pub diff_penalty_weight: f32,
    /// Reference diff size for normalization.
    pub diff_normalizer: f32,
    /// Compile-failure penalty (subtracted from score when `false`).
    pub compile_penalty: f32,
    /// When `true`, non-compiling candidates are removed in `disqualify_non_compiling`.
    pub min_compile_required: bool,
}

impl Default for TournamentConfig {
    fn default() -> Self {
        Self {
            pass_rate_weight: 0.70,
            diff_penalty_weight: 0.10,
            diff_normalizer: 200.0,
            compile_penalty: 0.20,
            min_compile_required: false,
        }
    }
}

/// Scores, ranks, and disqualifies candidates.
#[derive(Debug, Clone)]
pub struct Tournament {
    pub config: TournamentConfig,
}

impl Tournament {
    pub fn new(config: TournamentConfig) -> Self {
        Self { config }
    }

    /// Compute and write the composite score into `candidate.score`.
    /// Formula: score = pass_rate - diff_penalty - compile_penalty, clamped to [0, 1].
    /// A perfect candidate (pass_rate=1.0, diff=0, compiles) scores exactly 1.0.
    pub fn score(&self, candidate: &mut ExploreCandidate) {
        let c = &self.config;
        let diff_penalty = c.diff_penalty_weight
            * (candidate.diff_lines as f32 / c.diff_normalizer).min(1.0);
        let compile_penalty = if candidate.compile_success { 0.0 } else { c.compile_penalty };
        candidate.score = (candidate.pass_rate - diff_penalty - compile_penalty).clamp(0.0, 1.0);
    }

    /// Score all candidates and return them sorted highest-score first.
    pub fn rank(&self, mut candidates: Vec<ExploreCandidate>) -> Vec<ExploreCandidate> {
        for c in &mut candidates {
            self.score(c);
        }
        candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        candidates
    }

    /// Remove candidates that failed to compile when `min_compile_required` is set.
    pub fn disqualify_non_compiling(
        &self,
        candidates: Vec<ExploreCandidate>,
    ) -> Vec<ExploreCandidate> {
        if self.config.min_compile_required {
            candidates.into_iter().filter(|c| c.compile_success).collect()
        } else {
            candidates
        }
    }
}

/// Outcome of a completed tournament.
#[derive(Debug, Clone)]
pub struct TournamentResult {
    pub winner: Option<ExploreCandidate>,
    pub total_candidates: usize,
}

impl TournamentResult {
    pub fn from_ranked(ranked: Vec<ExploreCandidate>) -> Self {
        let total = ranked.len();
        Self {
            winner: ranked.into_iter().next(),
            total_candidates: total,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(id: &str, pass_rate: f32, diff: usize, compiles: bool) -> ExploreCandidate {
        ExploreCandidate::new(id, "patch", pass_rate, diff, compiles)
    }

    #[test]
    fn test_score_perfect_candidate() {
        let t = Tournament::new(TournamentConfig::default());
        let mut c = cand("c1", 1.0, 0, true);
        t.score(&mut c);
        assert!((c.score - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_score_compile_failure_penalized() {
        let t = Tournament::new(TournamentConfig::default());
        let mut c_ok = cand("c1", 1.0, 0, true);
        let mut c_fail = cand("c2", 1.0, 0, false);
        t.score(&mut c_ok);
        t.score(&mut c_fail);
        assert!(c_ok.score > c_fail.score);
    }

    #[test]
    fn test_score_large_diff_penalized() {
        let t = Tournament::new(TournamentConfig::default());
        let mut c_small = cand("c1", 1.0, 10, true);
        let mut c_large = cand("c2", 1.0, 500, true);
        t.score(&mut c_small);
        t.score(&mut c_large);
        assert!(c_small.score > c_large.score);
    }

    #[test]
    fn test_rank_orders_by_score() {
        let t = Tournament::new(TournamentConfig::default());
        let candidates = vec![cand("a", 0.5, 0, true), cand("b", 1.0, 0, true), cand("c", 0.2, 0, true)];
        let ranked = t.rank(candidates);
        assert_eq!(ranked[0].id, "b");
    }

    #[test]
    fn test_disqualify_non_compiling_enabled() {
        let t = Tournament::new(TournamentConfig { min_compile_required: true, ..Default::default() });
        let candidates = vec![cand("a", 1.0, 0, true), cand("b", 1.0, 0, false)];
        let filtered = t.disqualify_non_compiling(candidates);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "a");
    }

    #[test]
    fn test_disqualify_non_compiling_disabled() {
        let t = Tournament::new(TournamentConfig::default());
        let candidates = vec![cand("a", 1.0, 0, true), cand("b", 1.0, 0, false)];
        let filtered = t.disqualify_non_compiling(candidates);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_tournament_result_winner() {
        let t = Tournament::new(TournamentConfig::default());
        let candidates = vec![cand("a", 0.8, 0, true), cand("b", 1.0, 0, true)];
        let ranked = t.rank(candidates);
        let result = TournamentResult::from_ranked(ranked);
        assert_eq!(result.winner.unwrap().id, "b");
        assert_eq!(result.total_candidates, 2);
    }

    #[test]
    fn test_tournament_result_empty() {
        let result = TournamentResult::from_ranked(vec![]);
        assert!(result.winner.is_none());
        assert_eq!(result.total_candidates, 0);
    }
}
