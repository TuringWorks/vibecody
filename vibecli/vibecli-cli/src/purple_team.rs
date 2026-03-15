use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum MitreTactic {
    InitialAccess,
    Execution,
    Persistence,
    PrivilegeEscalation,
    DefenseEvasion,
    CredentialAccess,
    Discovery,
    LateralMovement,
    Collection,
    Exfiltration,
    CommandAndControl,
    Impact,
    Reconnaissance,
    ResourceDevelopment,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExerciseStatus {
    Planned,
    InProgress,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AttackOutcome {
    Detected,
    PartiallyDetected,
    Missed,
    Blocked,
    NotTested,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CoverageLevel {
    Full,
    Partial,
    None,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DetectionSource {
    Edr,
    Siem,
    Ndr,
    Waf,
    Ips,
    Manual,
    CustomRule,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MitreAttackTechnique {
    pub id: String,
    pub name: String,
    pub tactic: MitreTactic,
    pub description: String,
    pub platforms: Vec<String>,
    pub data_sources: Vec<String>,
    pub detection_notes: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttackSimulation {
    pub id: String,
    pub technique_id: String,
    pub description: String,
    pub attack_steps: Vec<AttackStep>,
    pub expected_artifacts: Vec<String>,
    pub actual_outcome: AttackOutcome,
    pub detection_time_secs: Option<u64>,
    pub detection_source: Option<DetectionSource>,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttackStep {
    pub order: u32,
    pub action: String,
    pub tool: String,
    pub expected_evidence: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DetectionValidation {
    pub technique_id: String,
    pub detection_rule_id: String,
    pub expected_outcome: AttackOutcome,
    pub actual_outcome: AttackOutcome,
    pub gap_analysis: String,
    pub remediation: String,
    pub validated_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CoverageGap {
    pub technique_id: String,
    pub technique_name: String,
    pub tactic: MitreTactic,
    pub current_coverage: CoverageLevel,
    pub recommended_detection: String,
    pub priority: u8,
    pub effort_estimate_hours: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PurpleTeamExercise {
    pub id: String,
    pub name: String,
    pub status: ExerciseStatus,
    pub attack_simulations: Vec<AttackSimulation>,
    pub detection_validations: Vec<DetectionValidation>,
    pub coverage_gaps: Vec<CoverageGap>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub summary: String,
    pub lead: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttackMatrixCell {
    pub technique_id: String,
    pub technique_name: String,
    pub tactic: MitreTactic,
    pub coverage: CoverageLevel,
    pub last_tested: Option<String>,
    pub detection_sources: Vec<DetectionSource>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeatmapEntry {
    pub tactic: MitreTactic,
    pub total_techniques: usize,
    pub detected: usize,
    pub partial: usize,
    pub missed: usize,
    pub not_tested: usize,
    pub coverage_pct: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PurpleTeamManager {
    pub exercises: Vec<PurpleTeamExercise>,
    pub technique_db: Vec<MitreAttackTechnique>,
    pub attack_matrix: Vec<AttackMatrixCell>,
}

impl MitreTactic {
    fn label(&self) -> &str {
        match self {
            MitreTactic::InitialAccess => "Initial Access",
            MitreTactic::Execution => "Execution",
            MitreTactic::Persistence => "Persistence",
            MitreTactic::PrivilegeEscalation => "Privilege Escalation",
            MitreTactic::DefenseEvasion => "Defense Evasion",
            MitreTactic::CredentialAccess => "Credential Access",
            MitreTactic::Discovery => "Discovery",
            MitreTactic::LateralMovement => "Lateral Movement",
            MitreTactic::Collection => "Collection",
            MitreTactic::Exfiltration => "Exfiltration",
            MitreTactic::CommandAndControl => "Command and Control",
            MitreTactic::Impact => "Impact",
            MitreTactic::Reconnaissance => "Reconnaissance",
            MitreTactic::ResourceDevelopment => "Resource Development",
        }
    }
}

impl PurpleTeamManager {
    pub fn new() -> Self {
        let technique_db = vec![
            MitreAttackTechnique {
                id: "T1566".into(),
                name: "Phishing".into(),
                tactic: MitreTactic::InitialAccess,
                description: "Adversaries send phishing messages to gain access to victim systems".into(),
                platforms: vec!["Windows".into(), "macOS".into(), "Linux".into()],
                data_sources: vec!["Email".into(), "Network Traffic".into()],
                detection_notes: "Monitor for suspicious email attachments and links".into(),
            },
            MitreAttackTechnique {
                id: "T1059".into(),
                name: "Command and Scripting Interpreter".into(),
                tactic: MitreTactic::Execution,
                description: "Adversaries abuse command and script interpreters to execute commands".into(),
                platforms: vec!["Windows".into(), "macOS".into(), "Linux".into()],
                data_sources: vec!["Process".into(), "Command".into()],
                detection_notes: "Monitor process execution and command-line arguments".into(),
            },
            MitreAttackTechnique {
                id: "T1053".into(),
                name: "Scheduled Task/Job".into(),
                tactic: MitreTactic::Persistence,
                description: "Adversaries abuse task scheduling to execute malicious code at system startup or on a scheduled basis".into(),
                platforms: vec!["Windows".into(), "Linux".into(), "macOS".into()],
                data_sources: vec!["Scheduled Job".into(), "Process".into()],
                detection_notes: "Monitor scheduled task creation and modification".into(),
            },
            MitreAttackTechnique {
                id: "T1078".into(),
                name: "Valid Accounts".into(),
                tactic: MitreTactic::PrivilegeEscalation,
                description: "Adversaries obtain and abuse credentials of existing accounts".into(),
                platforms: vec!["Windows".into(), "Linux".into(), "macOS".into(), "Cloud".into()],
                data_sources: vec!["Logon Session".into(), "User Account".into()],
                detection_notes: "Monitor for unusual account activity and impossible travel".into(),
            },
            MitreAttackTechnique {
                id: "T1055".into(),
                name: "Process Injection".into(),
                tactic: MitreTactic::DefenseEvasion,
                description: "Adversaries inject code into processes to evade defenses and elevate privileges".into(),
                platforms: vec!["Windows".into(), "Linux".into(), "macOS".into()],
                data_sources: vec!["Process".into(), "Module".into()],
                detection_notes: "Monitor for process hollowing and DLL injection patterns".into(),
            },
            MitreAttackTechnique {
                id: "T1003".into(),
                name: "OS Credential Dumping".into(),
                tactic: MitreTactic::CredentialAccess,
                description: "Adversaries attempt to dump credentials from the OS".into(),
                platforms: vec!["Windows".into(), "Linux".into()],
                data_sources: vec!["Process".into(), "Command".into(), "File".into()],
                detection_notes: "Monitor for access to LSASS, SAM, and /etc/shadow".into(),
            },
            MitreAttackTechnique {
                id: "T1082".into(),
                name: "System Information Discovery".into(),
                tactic: MitreTactic::Discovery,
                description: "Adversaries attempt to get detailed information about the OS and hardware".into(),
                platforms: vec!["Windows".into(), "Linux".into(), "macOS".into()],
                data_sources: vec!["Process".into(), "Command".into()],
                detection_notes: "Monitor for systeminfo, uname, and similar commands".into(),
            },
            MitreAttackTechnique {
                id: "T1021".into(),
                name: "Remote Services".into(),
                tactic: MitreTactic::LateralMovement,
                description: "Adversaries use remote services such as RDP, SSH, or SMB to move laterally".into(),
                platforms: vec!["Windows".into(), "Linux".into(), "macOS".into()],
                data_sources: vec!["Logon Session".into(), "Network Traffic".into()],
                detection_notes: "Monitor for unusual remote service connections".into(),
            },
            MitreAttackTechnique {
                id: "T1005".into(),
                name: "Data from Local System".into(),
                tactic: MitreTactic::Collection,
                description: "Adversaries search local system sources to find files of interest and sensitive data".into(),
                platforms: vec!["Windows".into(), "Linux".into(), "macOS".into()],
                data_sources: vec!["File".into(), "Command".into()],
                detection_notes: "Monitor for bulk file access and staging behavior".into(),
            },
            MitreAttackTechnique {
                id: "T1041".into(),
                name: "Exfiltration Over C2 Channel".into(),
                tactic: MitreTactic::Exfiltration,
                description: "Adversaries steal data by exfiltrating it over an existing C2 channel".into(),
                platforms: vec!["Windows".into(), "Linux".into(), "macOS".into()],
                data_sources: vec!["Network Traffic".into(), "Command".into()],
                detection_notes: "Monitor for large outbound data transfers over C2".into(),
            },
            MitreAttackTechnique {
                id: "T1071".into(),
                name: "Application Layer Protocol".into(),
                tactic: MitreTactic::CommandAndControl,
                description: "Adversaries communicate using OSI application layer protocols to avoid detection".into(),
                platforms: vec!["Windows".into(), "Linux".into(), "macOS".into()],
                data_sources: vec!["Network Traffic".into()],
                detection_notes: "Monitor for unusual HTTP/HTTPS/DNS traffic patterns".into(),
            },
            MitreAttackTechnique {
                id: "T1486".into(),
                name: "Data Encrypted for Impact".into(),
                tactic: MitreTactic::Impact,
                description: "Adversaries encrypt data on target systems to interrupt availability (ransomware)".into(),
                platforms: vec!["Windows".into(), "Linux".into(), "macOS".into()],
                data_sources: vec!["File".into(), "Process".into()],
                detection_notes: "Monitor for mass file encryption and ransom note creation".into(),
            },
            MitreAttackTechnique {
                id: "T1595".into(),
                name: "Active Scanning".into(),
                tactic: MitreTactic::Reconnaissance,
                description: "Adversaries execute active reconnaissance scans to gather information".into(),
                platforms: vec!["Network".into()],
                data_sources: vec!["Network Traffic".into()],
                detection_notes: "Monitor for port scans and vulnerability scanning activity".into(),
            },
            MitreAttackTechnique {
                id: "T1583".into(),
                name: "Acquire Infrastructure".into(),
                tactic: MitreTactic::ResourceDevelopment,
                description: "Adversaries buy, lease, or rent infrastructure for targeting victims".into(),
                platforms: vec!["PRE".into()],
                data_sources: vec!["Domain Registration".into(), "DNS".into()],
                detection_notes: "Monitor for newly registered domains resembling the organization".into(),
            },
            MitreAttackTechnique {
                id: "T1190".into(),
                name: "Exploit Public-Facing Application".into(),
                tactic: MitreTactic::InitialAccess,
                description: "Adversaries exploit vulnerabilities in internet-facing applications".into(),
                platforms: vec!["Windows".into(), "Linux".into(), "macOS".into(), "Cloud".into()],
                data_sources: vec!["Application Log".into(), "Network Traffic".into()],
                detection_notes: "Monitor application logs for exploitation attempts".into(),
            },
            MitreAttackTechnique {
                id: "T1547".into(),
                name: "Boot or Logon Autostart Execution".into(),
                tactic: MitreTactic::Persistence,
                description: "Adversaries configure system settings to automatically execute a program during boot or logon".into(),
                platforms: vec!["Windows".into(), "Linux".into(), "macOS".into()],
                data_sources: vec!["Registry".into(), "File".into(), "Process".into()],
                detection_notes: "Monitor registry run keys, startup folders, and init scripts".into(),
            },
            MitreAttackTechnique {
                id: "T1027".into(),
                name: "Obfuscated Files or Information".into(),
                tactic: MitreTactic::DefenseEvasion,
                description: "Adversaries attempt to make payloads difficult to discover or analyze by obfuscation".into(),
                platforms: vec!["Windows".into(), "Linux".into(), "macOS".into()],
                data_sources: vec!["File".into(), "Process".into()],
                detection_notes: "Monitor for encoded/encrypted payloads and script deobfuscation".into(),
            },
            MitreAttackTechnique {
                id: "T1046".into(),
                name: "Network Service Discovery".into(),
                tactic: MitreTactic::Discovery,
                description: "Adversaries attempt to get a listing of services running on remote hosts".into(),
                platforms: vec!["Windows".into(), "Linux".into(), "macOS".into()],
                data_sources: vec!["Network Traffic".into(), "Process".into()],
                detection_notes: "Monitor for internal port scanning and service enumeration".into(),
            },
            MitreAttackTechnique {
                id: "T1110".into(),
                name: "Brute Force".into(),
                tactic: MitreTactic::CredentialAccess,
                description: "Adversaries use brute force techniques to attempt access to accounts".into(),
                platforms: vec!["Windows".into(), "Linux".into(), "macOS".into(), "Cloud".into()],
                data_sources: vec!["Logon Session".into(), "User Account".into()],
                detection_notes: "Monitor for multiple failed authentication attempts".into(),
            },
            MitreAttackTechnique {
                id: "T1048".into(),
                name: "Exfiltration Over Alternative Protocol".into(),
                tactic: MitreTactic::Exfiltration,
                description: "Adversaries steal data by exfiltrating it over a different protocol than C2".into(),
                platforms: vec!["Windows".into(), "Linux".into(), "macOS".into()],
                data_sources: vec!["Network Traffic".into(), "File".into()],
                detection_notes: "Monitor for DNS tunneling, ICMP exfil, and unusual protocols".into(),
            },
        ];

        let attack_matrix = technique_db
            .iter()
            .map(|t| AttackMatrixCell {
                technique_id: t.id.clone(),
                technique_name: t.name.clone(),
                tactic: t.tactic.clone(),
                coverage: CoverageLevel::Unknown,
                last_tested: None,
                detection_sources: Vec::new(),
            })
            .collect();

        PurpleTeamManager {
            exercises: Vec::new(),
            technique_db,
            attack_matrix,
        }
    }

    pub fn create_exercise(&mut self, name: &str, lead: &str) -> String {
        let id = format!("PT-{:04}", self.exercises.len() + 1);
        self.exercises.push(PurpleTeamExercise {
            id: id.clone(),
            name: name.to_string(),
            status: ExerciseStatus::Planned,
            attack_simulations: Vec::new(),
            detection_validations: Vec::new(),
            coverage_gaps: Vec::new(),
            started_at: None,
            completed_at: None,
            summary: String::new(),
            lead: lead.to_string(),
        });
        id
    }

    pub fn start_exercise(&mut self, id: &str) -> bool {
        if let Some(ex) = self.exercises.iter_mut().find(|e| e.id == id) {
            if ex.status == ExerciseStatus::Planned {
                ex.status = ExerciseStatus::InProgress;
                ex.started_at = Some("2026-03-14T00:00:00Z".to_string());
                return true;
            }
        }
        false
    }

    pub fn complete_exercise(&mut self, id: &str, summary: &str) -> bool {
        if let Some(ex) = self.exercises.iter_mut().find(|e| e.id == id) {
            if ex.status == ExerciseStatus::InProgress {
                ex.status = ExerciseStatus::Completed;
                ex.completed_at = Some("2026-03-14T23:59:59Z".to_string());
                ex.summary = summary.to_string();
                return true;
            }
        }
        false
    }

    pub fn add_simulation(
        &mut self,
        exercise_id: &str,
        technique_id: &str,
        description: &str,
    ) -> Option<String> {
        let ex = self.exercises.iter_mut().find(|e| e.id == exercise_id)?;
        let sim_id = format!("SIM-{:04}", ex.attack_simulations.len() + 1);
        ex.attack_simulations.push(AttackSimulation {
            id: sim_id.clone(),
            technique_id: technique_id.to_string(),
            description: description.to_string(),
            attack_steps: Vec::new(),
            expected_artifacts: Vec::new(),
            actual_outcome: AttackOutcome::NotTested,
            detection_time_secs: None,
            detection_source: None,
            notes: String::new(),
        });
        Some(sim_id)
    }

    pub fn add_attack_step(
        &mut self,
        exercise_id: &str,
        sim_id: &str,
        action: &str,
        tool: &str,
        expected_evidence: &str,
    ) -> bool {
        if let Some(ex) = self.exercises.iter_mut().find(|e| e.id == exercise_id) {
            if let Some(sim) = ex.attack_simulations.iter_mut().find(|s| s.id == sim_id) {
                let order = sim.attack_steps.len() as u32 + 1;
                sim.attack_steps.push(AttackStep {
                    order,
                    action: action.to_string(),
                    tool: tool.to_string(),
                    expected_evidence: expected_evidence.to_string(),
                });
                return true;
            }
        }
        false
    }

    pub fn record_outcome(
        &mut self,
        exercise_id: &str,
        sim_id: &str,
        outcome: AttackOutcome,
        detection_time: Option<u64>,
        detection_source: Option<DetectionSource>,
    ) -> bool {
        if let Some(ex) = self.exercises.iter_mut().find(|e| e.id == exercise_id) {
            if let Some(sim) = ex.attack_simulations.iter_mut().find(|s| s.id == sim_id) {
                sim.actual_outcome = outcome;
                sim.detection_time_secs = detection_time;
                sim.detection_source = detection_source;
                return true;
            }
        }
        false
    }

    pub fn validate_detection(
        &mut self,
        exercise_id: &str,
        technique_id: &str,
        rule_id: &str,
        expected: AttackOutcome,
        actual: AttackOutcome,
        gap_analysis: &str,
        remediation: &str,
    ) -> bool {
        if let Some(ex) = self.exercises.iter_mut().find(|e| e.id == exercise_id) {
            ex.detection_validations.push(DetectionValidation {
                technique_id: technique_id.to_string(),
                detection_rule_id: rule_id.to_string(),
                expected_outcome: expected,
                actual_outcome: actual,
                gap_analysis: gap_analysis.to_string(),
                remediation: remediation.to_string(),
                validated_at: "2026-03-14T12:00:00Z".to_string(),
            });
            return true;
        }
        false
    }

    pub fn identify_coverage_gaps(&mut self, exercise_id: &str) -> Vec<CoverageGap> {
        let ex = match self.exercises.iter().find(|e| e.id == exercise_id) {
            Some(e) => e,
            None => return Vec::new(),
        };

        let mut gaps = Vec::new();
        for sim in &ex.attack_simulations {
            match sim.actual_outcome {
                AttackOutcome::Missed | AttackOutcome::NotTested => {
                    let tech = self.technique_db.iter().find(|t| t.id == sim.technique_id);
                    let (name, tactic, rec) = match tech {
                        Some(t) => (
                            t.name.clone(),
                            t.tactic.clone(),
                            t.detection_notes.clone(),
                        ),
                        None => (
                            sim.technique_id.clone(),
                            MitreTactic::Execution,
                            "Review technique documentation".to_string(),
                        ),
                    };
                    let coverage = match sim.actual_outcome {
                        AttackOutcome::Missed => CoverageLevel::None,
                        _ => CoverageLevel::Unknown,
                    };
                    let priority = match sim.actual_outcome {
                        AttackOutcome::Missed => 1,
                        _ => 3,
                    };
                    gaps.push(CoverageGap {
                        technique_id: sim.technique_id.clone(),
                        technique_name: name,
                        tactic,
                        current_coverage: coverage,
                        recommended_detection: rec,
                        priority,
                        effort_estimate_hours: 8.0,
                    });
                }
                AttackOutcome::PartiallyDetected => {
                    let tech = self.technique_db.iter().find(|t| t.id == sim.technique_id);
                    let (name, tactic, rec) = match tech {
                        Some(t) => (
                            t.name.clone(),
                            t.tactic.clone(),
                            t.detection_notes.clone(),
                        ),
                        None => (
                            sim.technique_id.clone(),
                            MitreTactic::Execution,
                            "Improve detection coverage".to_string(),
                        ),
                    };
                    gaps.push(CoverageGap {
                        technique_id: sim.technique_id.clone(),
                        technique_name: name,
                        tactic,
                        current_coverage: CoverageLevel::Partial,
                        recommended_detection: rec,
                        priority: 2,
                        effort_estimate_hours: 4.0,
                    });
                }
                _ => {}
            }
        }

        // Store gaps in the exercise
        if let Some(ex) = self.exercises.iter_mut().find(|e| e.id == exercise_id) {
            ex.coverage_gaps = gaps.clone();
        }

        gaps
    }

    pub fn get_attack_matrix(&self) -> &Vec<AttackMatrixCell> {
        &self.attack_matrix
    }

    pub fn update_matrix_from_exercise(&mut self, exercise_id: &str) -> usize {
        let ex = match self.exercises.iter().find(|e| e.id == exercise_id) {
            Some(e) => e.clone(),
            None => return 0,
        };

        let mut updated = 0;
        for sim in &ex.attack_simulations {
            if let Some(cell) = self
                .attack_matrix
                .iter_mut()
                .find(|c| c.technique_id == sim.technique_id)
            {
                cell.coverage = match &sim.actual_outcome {
                    AttackOutcome::Detected | AttackOutcome::Blocked => CoverageLevel::Full,
                    AttackOutcome::PartiallyDetected => CoverageLevel::Partial,
                    AttackOutcome::Missed => CoverageLevel::None,
                    AttackOutcome::NotTested => CoverageLevel::Unknown,
                };
                cell.last_tested = Some("2026-03-14".to_string());
                if let Some(src) = &sim.detection_source {
                    if !cell.detection_sources.contains(src) {
                        cell.detection_sources.push(src.clone());
                    }
                }
                updated += 1;
            }
        }
        updated
    }

    pub fn generate_heatmap(&self, exercise_id: &str) -> Vec<HeatmapEntry> {
        let ex = match self.exercises.iter().find(|e| e.id == exercise_id) {
            Some(e) => e,
            None => return Vec::new(),
        };

        let mut tactic_map: HashMap<String, (MitreTactic, usize, usize, usize, usize, usize)> =
            HashMap::new();

        for sim in &ex.attack_simulations {
            let tech = self.technique_db.iter().find(|t| t.id == sim.technique_id);
            let tactic = match tech {
                Some(t) => t.tactic.clone(),
                None => continue,
            };
            let label = tactic.label().to_string();
            let entry = tactic_map
                .entry(label)
                .or_insert_with(|| (tactic, 0, 0, 0, 0, 0));
            entry.1 += 1; // total
            match &sim.actual_outcome {
                AttackOutcome::Detected | AttackOutcome::Blocked => entry.2 += 1,
                AttackOutcome::PartiallyDetected => entry.3 += 1,
                AttackOutcome::Missed => entry.4 += 1,
                AttackOutcome::NotTested => entry.5 += 1,
            }
        }

        let mut entries: Vec<HeatmapEntry> = tactic_map
            .into_values()
            .map(|(tactic, total, detected, partial, missed, not_tested)| {
                let tested = total - not_tested;
                let coverage_pct = if tested > 0 {
                    ((detected as f64 + partial as f64 * 0.5) / tested as f64) * 100.0
                } else {
                    0.0
                };
                HeatmapEntry {
                    tactic,
                    total_techniques: total,
                    detected,
                    partial,
                    missed,
                    not_tested,
                    coverage_pct,
                }
            })
            .collect();

        entries.sort_by(|a, b| {
            a.coverage_pct
                .partial_cmp(&b.coverage_pct)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        entries
    }

    pub fn calculate_coverage_score(&self, exercise_id: &str) -> f64 {
        let ex = match self.exercises.iter().find(|e| e.id == exercise_id) {
            Some(e) => e,
            None => return 0.0,
        };

        let tested: Vec<&AttackSimulation> = ex
            .attack_simulations
            .iter()
            .filter(|s| s.actual_outcome != AttackOutcome::NotTested)
            .collect();

        if tested.is_empty() {
            return 0.0;
        }

        let score: f64 = tested
            .iter()
            .map(|s| match &s.actual_outcome {
                AttackOutcome::Detected | AttackOutcome::Blocked => 1.0,
                AttackOutcome::PartiallyDetected => 0.5,
                _ => 0.0,
            })
            .sum();

        (score / tested.len() as f64) * 100.0
    }

    pub fn recommend_detections(&self, exercise_id: &str) -> Vec<(String, String)> {
        let ex = match self.exercises.iter().find(|e| e.id == exercise_id) {
            Some(e) => e,
            None => return Vec::new(),
        };

        let mut recommendations = Vec::new();
        for sim in &ex.attack_simulations {
            match sim.actual_outcome {
                AttackOutcome::Missed | AttackOutcome::PartiallyDetected | AttackOutcome::NotTested => {
                    if let Some(tech) = self.technique_db.iter().find(|t| t.id == sim.technique_id)
                    {
                        recommendations
                            .push((tech.name.clone(), tech.detection_notes.clone()));
                    }
                }
                _ => {}
            }
        }
        recommendations
    }

    pub fn export_exercise_report(&self, exercise_id: &str) -> Option<String> {
        let ex = self.exercises.iter().find(|e| e.id == exercise_id)?;

        let mut report = String::new();
        report.push_str(&format!("# Purple Team Exercise Report: {}\n\n", ex.name));
        report.push_str(&format!("**ID:** {}  \n", ex.id));
        report.push_str(&format!("**Lead:** {}  \n", ex.lead));
        report.push_str(&format!("**Status:** {:?}  \n", ex.status));
        if let Some(ref started) = ex.started_at {
            report.push_str(&format!("**Started:** {}  \n", started));
        }
        if let Some(ref completed) = ex.completed_at {
            report.push_str(&format!("**Completed:** {}  \n", completed));
        }
        report.push_str(&format!("\n## Summary\n\n{}\n\n", ex.summary));

        report.push_str("## Attack Simulations\n\n");
        report.push_str("| Technique | Description | Outcome | Detection Time |\n");
        report.push_str("|-----------|-------------|---------|----------------|\n");
        for sim in &ex.attack_simulations {
            let time = sim
                .detection_time_secs
                .map(|t| format!("{}s", t))
                .unwrap_or_else(|| "N/A".to_string());
            report.push_str(&format!(
                "| {} | {} | {:?} | {} |\n",
                sim.technique_id, sim.description, sim.actual_outcome, time
            ));
        }

        report.push_str("\n## Detection Validations\n\n");
        for dv in &ex.detection_validations {
            report.push_str(&format!(
                "- **{}** (Rule: {}): Expected {:?}, Actual {:?}\n  - Gap: {}\n  - Remediation: {}\n",
                dv.technique_id,
                dv.detection_rule_id,
                dv.expected_outcome,
                dv.actual_outcome,
                dv.gap_analysis,
                dv.remediation
            ));
        }

        if !ex.coverage_gaps.is_empty() {
            report.push_str("\n## Coverage Gaps\n\n");
            for gap in &ex.coverage_gaps {
                report.push_str(&format!(
                    "- **{} ({})**: {:?} coverage, Priority {}, ~{:.0}h effort\n  - Recommendation: {}\n",
                    gap.technique_name,
                    gap.technique_id,
                    gap.current_coverage,
                    gap.priority,
                    gap.effort_estimate_hours,
                    gap.recommended_detection
                ));
            }
        }

        let score = self.calculate_coverage_score(exercise_id);
        report.push_str(&format!(
            "\n## Overall Coverage Score: {:.1}%\n",
            score
        ));

        Some(report)
    }

    pub fn compare_exercises(&self, id1: &str, id2: &str) -> Option<String> {
        let ex1 = self.exercises.iter().find(|e| e.id == id1)?;
        let ex2 = self.exercises.iter().find(|e| e.id == id2)?;

        let score1 = self.calculate_coverage_score(id1);
        let score2 = self.calculate_coverage_score(id2);

        let mut report = String::new();
        report.push_str("# Exercise Comparison\n\n");
        report.push_str(&format!(
            "| Metric | {} | {} |\n",
            ex1.name, ex2.name
        ));
        report.push_str("|--------|------|------|\n");
        report.push_str(&format!(
            "| Simulations | {} | {} |\n",
            ex1.attack_simulations.len(),
            ex2.attack_simulations.len()
        ));
        report.push_str(&format!(
            "| Coverage Score | {:.1}% | {:.1}% |\n",
            score1, score2
        ));
        report.push_str(&format!(
            "| Coverage Gaps | {} | {} |\n",
            ex1.coverage_gaps.len(),
            ex2.coverage_gaps.len()
        ));

        let diff = score2 - score1;
        if diff > 0.0 {
            report.push_str(&format!(
                "\nCoverage improved by {:.1} percentage points.\n",
                diff
            ));
        } else if diff < 0.0 {
            report.push_str(&format!(
                "\nCoverage decreased by {:.1} percentage points.\n",
                diff.abs()
            ));
        } else {
            report.push_str("\nCoverage unchanged between exercises.\n");
        }

        Some(report)
    }

    pub fn get_techniques_by_tactic(&self, tactic: &MitreTactic) -> Vec<&MitreAttackTechnique> {
        self.technique_db
            .iter()
            .filter(|t| &t.tactic == tactic)
            .collect()
    }

    pub fn get_exercise(&self, id: &str) -> Option<&PurpleTeamExercise> {
        self.exercises.iter().find(|e| e.id == id)
    }

    pub fn list_exercises(&self) -> Vec<(String, String, ExerciseStatus)> {
        self.exercises
            .iter()
            .map(|e| (e.id.clone(), e.name.clone(), e.status.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_manager() -> PurpleTeamManager {
        PurpleTeamManager::new()
    }

    fn setup_with_exercise() -> (PurpleTeamManager, String) {
        let mut mgr = setup_manager();
        let id = mgr.create_exercise("Q1 Security Assessment", "Alice");
        (mgr, id)
    }

    #[test]
    fn test_new_manager_has_techniques() {
        let mgr = setup_manager();
        assert_eq!(mgr.technique_db.len(), 20);
        assert_eq!(mgr.attack_matrix.len(), 20);
        assert!(mgr.exercises.is_empty());
    }

    #[test]
    fn test_create_exercise() {
        let (mgr, id) = setup_with_exercise();
        assert_eq!(id, "PT-0001");
        assert_eq!(mgr.exercises.len(), 1);
        assert_eq!(mgr.exercises[0].status, ExerciseStatus::Planned);
        assert_eq!(mgr.exercises[0].lead, "Alice");
    }

    #[test]
    fn test_create_multiple_exercises() {
        let mut mgr = setup_manager();
        let id1 = mgr.create_exercise("Ex1", "Alice");
        let id2 = mgr.create_exercise("Ex2", "Bob");
        assert_eq!(id1, "PT-0001");
        assert_eq!(id2, "PT-0002");
        assert_eq!(mgr.exercises.len(), 2);
    }

    #[test]
    fn test_start_exercise() {
        let (mut mgr, id) = setup_with_exercise();
        assert!(mgr.start_exercise(&id));
        assert_eq!(mgr.get_exercise(&id).unwrap().status, ExerciseStatus::InProgress);
        assert!(mgr.get_exercise(&id).unwrap().started_at.is_some());
    }

    #[test]
    fn test_start_exercise_only_when_planned() {
        let (mut mgr, id) = setup_with_exercise();
        mgr.start_exercise(&id);
        // Cannot start again
        assert!(!mgr.start_exercise(&id));
    }

    #[test]
    fn test_start_exercise_unknown_id() {
        let mut mgr = setup_manager();
        assert!(!mgr.start_exercise("PT-9999"));
    }

    #[test]
    fn test_complete_exercise() {
        let (mut mgr, id) = setup_with_exercise();
        mgr.start_exercise(&id);
        assert!(mgr.complete_exercise(&id, "All tests passed"));
        let ex = mgr.get_exercise(&id).unwrap();
        assert_eq!(ex.status, ExerciseStatus::Completed);
        assert_eq!(ex.summary, "All tests passed");
        assert!(ex.completed_at.is_some());
    }

    #[test]
    fn test_complete_exercise_must_be_in_progress() {
        let (mut mgr, id) = setup_with_exercise();
        assert!(!mgr.complete_exercise(&id, "summary"));
    }

    #[test]
    fn test_add_simulation() {
        let (mut mgr, id) = setup_with_exercise();
        let sim_id = mgr.add_simulation(&id, "T1566", "Phishing test").unwrap();
        assert_eq!(sim_id, "SIM-0001");
        let ex = mgr.get_exercise(&id).unwrap();
        assert_eq!(ex.attack_simulations.len(), 1);
        assert_eq!(ex.attack_simulations[0].actual_outcome, AttackOutcome::NotTested);
    }

    #[test]
    fn test_add_simulation_unknown_exercise() {
        let mut mgr = setup_manager();
        assert!(mgr.add_simulation("PT-9999", "T1566", "test").is_none());
    }

    #[test]
    fn test_add_attack_step() {
        let (mut mgr, id) = setup_with_exercise();
        let sim_id = mgr.add_simulation(&id, "T1566", "Phishing test").unwrap();
        assert!(mgr.add_attack_step(&id, &sim_id, "Send email", "GoPhish", "Email log entry"));
        let ex = mgr.get_exercise(&id).unwrap();
        assert_eq!(ex.attack_simulations[0].attack_steps.len(), 1);
        assert_eq!(ex.attack_simulations[0].attack_steps[0].order, 1);
    }

    #[test]
    fn test_add_multiple_attack_steps() {
        let (mut mgr, id) = setup_with_exercise();
        let sim_id = mgr.add_simulation(&id, "T1059", "Command exec").unwrap();
        mgr.add_attack_step(&id, &sim_id, "Step 1", "bash", "log entry");
        mgr.add_attack_step(&id, &sim_id, "Step 2", "powershell", "event log");
        let ex = mgr.get_exercise(&id).unwrap();
        assert_eq!(ex.attack_simulations[0].attack_steps.len(), 2);
        assert_eq!(ex.attack_simulations[0].attack_steps[1].order, 2);
    }

    #[test]
    fn test_add_attack_step_unknown_sim() {
        let (mut mgr, id) = setup_with_exercise();
        assert!(!mgr.add_attack_step(&id, "SIM-9999", "action", "tool", "evidence"));
    }

    #[test]
    fn test_record_outcome_detected() {
        let (mut mgr, id) = setup_with_exercise();
        let sim_id = mgr.add_simulation(&id, "T1566", "Phishing test").unwrap();
        assert!(mgr.record_outcome(
            &id,
            &sim_id,
            AttackOutcome::Detected,
            Some(30),
            Some(DetectionSource::Siem)
        ));
        let ex = mgr.get_exercise(&id).unwrap();
        assert_eq!(ex.attack_simulations[0].actual_outcome, AttackOutcome::Detected);
        assert_eq!(ex.attack_simulations[0].detection_time_secs, Some(30));
    }

    #[test]
    fn test_record_outcome_missed() {
        let (mut mgr, id) = setup_with_exercise();
        let sim_id = mgr.add_simulation(&id, "T1055", "Injection test").unwrap();
        assert!(mgr.record_outcome(&id, &sim_id, AttackOutcome::Missed, None, None));
        let ex = mgr.get_exercise(&id).unwrap();
        assert_eq!(ex.attack_simulations[0].actual_outcome, AttackOutcome::Missed);
        assert!(ex.attack_simulations[0].detection_time_secs.is_none());
    }

    #[test]
    fn test_record_outcome_unknown_sim() {
        let (mut mgr, id) = setup_with_exercise();
        assert!(!mgr.record_outcome(&id, "SIM-9999", AttackOutcome::Detected, None, None));
    }

    #[test]
    fn test_validate_detection() {
        let (mut mgr, id) = setup_with_exercise();
        assert!(mgr.validate_detection(
            &id,
            "T1566",
            "RULE-001",
            AttackOutcome::Detected,
            AttackOutcome::PartiallyDetected,
            "Missing attachment analysis",
            "Add YARA rules for macro detection"
        ));
        let ex = mgr.get_exercise(&id).unwrap();
        assert_eq!(ex.detection_validations.len(), 1);
        assert_eq!(ex.detection_validations[0].detection_rule_id, "RULE-001");
    }

    #[test]
    fn test_validate_detection_unknown_exercise() {
        let mut mgr = setup_manager();
        assert!(!mgr.validate_detection(
            "PT-9999",
            "T1566",
            "RULE-001",
            AttackOutcome::Detected,
            AttackOutcome::Detected,
            "",
            ""
        ));
    }

    #[test]
    fn test_identify_coverage_gaps_missed() {
        let (mut mgr, id) = setup_with_exercise();
        let sim_id = mgr.add_simulation(&id, "T1055", "Process injection").unwrap();
        mgr.record_outcome(&id, &sim_id, AttackOutcome::Missed, None, None);
        let gaps = mgr.identify_coverage_gaps(&id);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].technique_id, "T1055");
        assert_eq!(gaps[0].current_coverage, CoverageLevel::None);
        assert_eq!(gaps[0].priority, 1);
    }

    #[test]
    fn test_identify_coverage_gaps_partial() {
        let (mut mgr, id) = setup_with_exercise();
        let sim_id = mgr.add_simulation(&id, "T1003", "Credential dump").unwrap();
        mgr.record_outcome(&id, &sim_id, AttackOutcome::PartiallyDetected, Some(120), Some(DetectionSource::Edr));
        let gaps = mgr.identify_coverage_gaps(&id);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].current_coverage, CoverageLevel::Partial);
        assert_eq!(gaps[0].priority, 2);
    }

    #[test]
    fn test_identify_coverage_gaps_none_when_all_detected() {
        let (mut mgr, id) = setup_with_exercise();
        let sim_id = mgr.add_simulation(&id, "T1566", "Phishing").unwrap();
        mgr.record_outcome(&id, &sim_id, AttackOutcome::Detected, Some(5), Some(DetectionSource::Siem));
        let gaps = mgr.identify_coverage_gaps(&id);
        assert!(gaps.is_empty());
    }

    #[test]
    fn test_identify_coverage_gaps_empty_exercise() {
        let (mut mgr, id) = setup_with_exercise();
        let gaps = mgr.identify_coverage_gaps(&id);
        assert!(gaps.is_empty());
    }

    #[test]
    fn test_identify_coverage_gaps_unknown_exercise() {
        let mut mgr = setup_manager();
        let gaps = mgr.identify_coverage_gaps("PT-9999");
        assert!(gaps.is_empty());
    }

    #[test]
    fn test_get_attack_matrix() {
        let mgr = setup_manager();
        let matrix = mgr.get_attack_matrix();
        assert_eq!(matrix.len(), 20);
        assert!(matrix.iter().all(|c| c.coverage == CoverageLevel::Unknown));
    }

    #[test]
    fn test_update_matrix_from_exercise() {
        let (mut mgr, id) = setup_with_exercise();
        let sim_id = mgr.add_simulation(&id, "T1566", "Phishing").unwrap();
        mgr.record_outcome(&id, &sim_id, AttackOutcome::Detected, Some(10), Some(DetectionSource::Siem));
        let updated = mgr.update_matrix_from_exercise(&id);
        assert_eq!(updated, 1);
        let cell = mgr.attack_matrix.iter().find(|c| c.technique_id == "T1566").unwrap();
        assert_eq!(cell.coverage, CoverageLevel::Full);
        assert!(cell.last_tested.is_some());
        assert!(cell.detection_sources.contains(&DetectionSource::Siem));
    }

    #[test]
    fn test_update_matrix_unknown_exercise() {
        let mut mgr = setup_manager();
        assert_eq!(mgr.update_matrix_from_exercise("PT-9999"), 0);
    }

    #[test]
    fn test_generate_heatmap() {
        let (mut mgr, id) = setup_with_exercise();
        let s1 = mgr.add_simulation(&id, "T1566", "Phishing").unwrap();
        let s2 = mgr.add_simulation(&id, "T1190", "Exploit app").unwrap();
        mgr.record_outcome(&id, &s1, AttackOutcome::Detected, Some(5), None);
        mgr.record_outcome(&id, &s2, AttackOutcome::Missed, None, None);
        let heatmap = mgr.generate_heatmap(&id);
        assert_eq!(heatmap.len(), 1); // both Initial Access
        let entry = &heatmap[0];
        assert_eq!(entry.total_techniques, 2);
        assert_eq!(entry.detected, 1);
        assert_eq!(entry.missed, 1);
        assert_eq!(entry.coverage_pct, 50.0);
    }

    #[test]
    fn test_generate_heatmap_empty() {
        let (mgr, id) = setup_with_exercise();
        let heatmap = mgr.generate_heatmap(&id);
        assert!(heatmap.is_empty());
    }

    #[test]
    fn test_generate_heatmap_unknown_exercise() {
        let mgr = setup_manager();
        let heatmap = mgr.generate_heatmap("PT-9999");
        assert!(heatmap.is_empty());
    }

    #[test]
    fn test_calculate_coverage_score_all_detected() {
        let (mut mgr, id) = setup_with_exercise();
        let s1 = mgr.add_simulation(&id, "T1566", "Phishing").unwrap();
        let s2 = mgr.add_simulation(&id, "T1059", "Cmd exec").unwrap();
        mgr.record_outcome(&id, &s1, AttackOutcome::Detected, Some(5), None);
        mgr.record_outcome(&id, &s2, AttackOutcome::Blocked, Some(1), None);
        assert_eq!(mgr.calculate_coverage_score(&id), 100.0);
    }

    #[test]
    fn test_calculate_coverage_score_all_missed() {
        let (mut mgr, id) = setup_with_exercise();
        let s1 = mgr.add_simulation(&id, "T1566", "Phishing").unwrap();
        let s2 = mgr.add_simulation(&id, "T1059", "Cmd exec").unwrap();
        mgr.record_outcome(&id, &s1, AttackOutcome::Missed, None, None);
        mgr.record_outcome(&id, &s2, AttackOutcome::Missed, None, None);
        assert_eq!(mgr.calculate_coverage_score(&id), 0.0);
    }

    #[test]
    fn test_calculate_coverage_score_mixed() {
        let (mut mgr, id) = setup_with_exercise();
        let s1 = mgr.add_simulation(&id, "T1566", "Phishing").unwrap();
        let s2 = mgr.add_simulation(&id, "T1059", "Cmd exec").unwrap();
        mgr.record_outcome(&id, &s1, AttackOutcome::Detected, Some(5), None);
        mgr.record_outcome(&id, &s2, AttackOutcome::Missed, None, None);
        assert_eq!(mgr.calculate_coverage_score(&id), 50.0);
    }

    #[test]
    fn test_calculate_coverage_score_empty() {
        let (mgr, id) = setup_with_exercise();
        assert_eq!(mgr.calculate_coverage_score(&id), 0.0);
    }

    #[test]
    fn test_calculate_coverage_score_partial() {
        let (mut mgr, id) = setup_with_exercise();
        let s1 = mgr.add_simulation(&id, "T1566", "Phishing").unwrap();
        mgr.record_outcome(&id, &s1, AttackOutcome::PartiallyDetected, Some(60), None);
        assert_eq!(mgr.calculate_coverage_score(&id), 50.0);
    }

    #[test]
    fn test_recommend_detections() {
        let (mut mgr, id) = setup_with_exercise();
        let s1 = mgr.add_simulation(&id, "T1055", "Injection").unwrap();
        let s2 = mgr.add_simulation(&id, "T1566", "Phishing").unwrap();
        mgr.record_outcome(&id, &s1, AttackOutcome::Missed, None, None);
        mgr.record_outcome(&id, &s2, AttackOutcome::Detected, Some(5), None);
        let recs = mgr.recommend_detections(&id);
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].0, "Process Injection");
    }

    #[test]
    fn test_recommend_detections_includes_partial() {
        let (mut mgr, id) = setup_with_exercise();
        let s1 = mgr.add_simulation(&id, "T1003", "Cred dump").unwrap();
        mgr.record_outcome(&id, &s1, AttackOutcome::PartiallyDetected, Some(120), None);
        let recs = mgr.recommend_detections(&id);
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].0, "OS Credential Dumping");
    }

    #[test]
    fn test_export_exercise_report() {
        let (mut mgr, id) = setup_with_exercise();
        mgr.start_exercise(&id);
        let sim_id = mgr.add_simulation(&id, "T1566", "Phishing test").unwrap();
        mgr.record_outcome(&id, &sim_id, AttackOutcome::Detected, Some(15), Some(DetectionSource::Siem));
        mgr.complete_exercise(&id, "Successful exercise");
        let report = mgr.export_exercise_report(&id).unwrap();
        assert!(report.contains("# Purple Team Exercise Report"));
        assert!(report.contains("Q1 Security Assessment"));
        assert!(report.contains("T1566"));
        assert!(report.contains("Detected"));
        assert!(report.contains("Coverage Score"));
    }

    #[test]
    fn test_export_exercise_report_unknown() {
        let mgr = setup_manager();
        assert!(mgr.export_exercise_report("PT-9999").is_none());
    }

    #[test]
    fn test_compare_exercises() {
        let mut mgr = setup_manager();
        let id1 = mgr.create_exercise("Ex1", "Alice");
        let id2 = mgr.create_exercise("Ex2", "Bob");

        let s1 = mgr.add_simulation(&id1, "T1566", "Phishing").unwrap();
        mgr.record_outcome(&id1, &s1, AttackOutcome::Missed, None, None);

        let s2 = mgr.add_simulation(&id2, "T1566", "Phishing v2").unwrap();
        mgr.record_outcome(&id2, &s2, AttackOutcome::Detected, Some(10), None);

        let report = mgr.compare_exercises(&id1, &id2).unwrap();
        assert!(report.contains("Exercise Comparison"));
        assert!(report.contains("Ex1"));
        assert!(report.contains("Ex2"));
        assert!(report.contains("improved"));
    }

    #[test]
    fn test_compare_exercises_unknown() {
        let mgr = setup_manager();
        assert!(mgr.compare_exercises("PT-0001", "PT-0002").is_none());
    }

    #[test]
    fn test_compare_exercises_same_score() {
        let mut mgr = setup_manager();
        let id1 = mgr.create_exercise("Ex1", "Alice");
        let id2 = mgr.create_exercise("Ex2", "Bob");
        let report = mgr.compare_exercises(&id1, &id2).unwrap();
        assert!(report.contains("unchanged"));
    }

    #[test]
    fn test_get_techniques_by_tactic() {
        let mgr = setup_manager();
        let ia = mgr.get_techniques_by_tactic(&MitreTactic::InitialAccess);
        assert_eq!(ia.len(), 2); // T1566, T1190
        assert!(ia.iter().any(|t| t.id == "T1566"));
        assert!(ia.iter().any(|t| t.id == "T1190"));
    }

    #[test]
    fn test_get_techniques_by_tactic_discovery() {
        let mgr = setup_manager();
        let disc = mgr.get_techniques_by_tactic(&MitreTactic::Discovery);
        assert_eq!(disc.len(), 2); // T1082, T1046
    }

    #[test]
    fn test_get_techniques_by_tactic_exfiltration() {
        let mgr = setup_manager();
        let exfil = mgr.get_techniques_by_tactic(&MitreTactic::Exfiltration);
        assert_eq!(exfil.len(), 2); // T1041, T1048
    }

    #[test]
    fn test_get_exercise() {
        let (mgr, id) = setup_with_exercise();
        let ex = mgr.get_exercise(&id).unwrap();
        assert_eq!(ex.name, "Q1 Security Assessment");
    }

    #[test]
    fn test_get_exercise_unknown() {
        let mgr = setup_manager();
        assert!(mgr.get_exercise("PT-9999").is_none());
    }

    #[test]
    fn test_list_exercises() {
        let mut mgr = setup_manager();
        mgr.create_exercise("Ex1", "Alice");
        mgr.create_exercise("Ex2", "Bob");
        let list = mgr.list_exercises();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].0, "PT-0001");
        assert_eq!(list[0].1, "Ex1");
        assert_eq!(list[0].2, ExerciseStatus::Planned);
    }

    #[test]
    fn test_list_exercises_empty() {
        let mgr = setup_manager();
        assert!(mgr.list_exercises().is_empty());
    }

    #[test]
    fn test_full_exercise_lifecycle() {
        let mut mgr = setup_manager();
        let id = mgr.create_exercise("Full Lifecycle", "Charlie");
        mgr.start_exercise(&id);

        let s1 = mgr.add_simulation(&id, "T1566", "Phishing").unwrap();
        let s2 = mgr.add_simulation(&id, "T1059", "Command exec").unwrap();
        let s3 = mgr.add_simulation(&id, "T1055", "Process injection").unwrap();

        mgr.add_attack_step(&id, &s1, "Craft email", "GoPhish", "SMTP log");
        mgr.add_attack_step(&id, &s1, "User clicks link", "Browser", "Proxy log");

        mgr.record_outcome(&id, &s1, AttackOutcome::Detected, Some(15), Some(DetectionSource::Siem));
        mgr.record_outcome(&id, &s2, AttackOutcome::PartiallyDetected, Some(300), Some(DetectionSource::Edr));
        mgr.record_outcome(&id, &s3, AttackOutcome::Missed, None, None);

        mgr.validate_detection(
            &id, "T1059", "RULE-CMD-001",
            AttackOutcome::Detected, AttackOutcome::PartiallyDetected,
            "Only PowerShell detected, not bash", "Add bash monitoring rules",
        );

        let gaps = mgr.identify_coverage_gaps(&id);
        assert_eq!(gaps.len(), 2); // T1059 partial + T1055 missed

        let updated = mgr.update_matrix_from_exercise(&id);
        assert_eq!(updated, 3);

        let heatmap = mgr.generate_heatmap(&id);
        assert!(!heatmap.is_empty());

        let score = mgr.calculate_coverage_score(&id);
        assert!(score > 0.0 && score < 100.0);

        let recs = mgr.recommend_detections(&id);
        assert_eq!(recs.len(), 2);

        mgr.complete_exercise(&id, "Completed full lifecycle test");
        let report = mgr.export_exercise_report(&id).unwrap();
        assert!(report.contains("Full Lifecycle"));
        assert!(report.contains("Coverage Score"));
    }

    #[test]
    fn test_matrix_cell_blocked_is_full_coverage() {
        let (mut mgr, id) = setup_with_exercise();
        let sim_id = mgr.add_simulation(&id, "T1486", "Ransomware sim").unwrap();
        mgr.record_outcome(&id, &sim_id, AttackOutcome::Blocked, Some(0), Some(DetectionSource::Ips));
        mgr.update_matrix_from_exercise(&id);
        let cell = mgr.attack_matrix.iter().find(|c| c.technique_id == "T1486").unwrap();
        assert_eq!(cell.coverage, CoverageLevel::Full);
    }

    #[test]
    fn test_heatmap_multiple_tactics() {
        let (mut mgr, id) = setup_with_exercise();
        let s1 = mgr.add_simulation(&id, "T1566", "Phishing").unwrap();
        let s2 = mgr.add_simulation(&id, "T1059", "Cmd exec").unwrap();
        let s3 = mgr.add_simulation(&id, "T1003", "Cred dump").unwrap();
        mgr.record_outcome(&id, &s1, AttackOutcome::Detected, Some(5), None);
        mgr.record_outcome(&id, &s2, AttackOutcome::Missed, None, None);
        mgr.record_outcome(&id, &s3, AttackOutcome::Detected, Some(10), None);
        let heatmap = mgr.generate_heatmap(&id);
        assert_eq!(heatmap.len(), 3); // InitialAccess, Execution, CredentialAccess
    }

    #[test]
    fn test_coverage_score_ignores_not_tested() {
        let (mut mgr, id) = setup_with_exercise();
        let s1 = mgr.add_simulation(&id, "T1566", "Phishing").unwrap();
        mgr.add_simulation(&id, "T1059", "Cmd exec - not tested").unwrap();
        mgr.record_outcome(&id, &s1, AttackOutcome::Detected, Some(5), None);
        // s2 is NotTested, should be excluded from score calculation
        assert_eq!(mgr.calculate_coverage_score(&id), 100.0);
    }
}
