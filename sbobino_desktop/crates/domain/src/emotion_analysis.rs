use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct EmotionOverview {
    #[serde(default)]
    pub primary_emotions: Vec<String>,
    #[serde(default)]
    pub emotional_arc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speaker_dynamics: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct EmotionTimelineEntry {
    pub segment_index: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_seconds: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_seconds: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speaker_label: Option<String>,
    #[serde(default)]
    pub dominant_emotions: Vec<String>,
    #[serde(default)]
    pub valence_score: f32,
    #[serde(default)]
    pub intensity_score: f32,
    #[serde(default)]
    pub evidence_text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shift_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct EmotionSemanticNode {
    pub id: String,
    pub label: String,
    pub kind: String,
    #[serde(default)]
    pub weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct EmotionSemanticEdge {
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub weight: f32,
    pub relation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct EmotionSemanticCluster {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub node_ids: Vec<String>,
    #[serde(default)]
    pub segment_indices: Vec<usize>,
    #[serde(default)]
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct EmotionSemanticMap {
    #[serde(default)]
    pub nodes: Vec<EmotionSemanticNode>,
    #[serde(default)]
    pub edges: Vec<EmotionSemanticEdge>,
    #[serde(default)]
    pub clusters: Vec<EmotionSemanticCluster>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct EmotionBridge {
    pub from_segment_index: usize,
    pub to_segment_index: usize,
    pub bridge_theme: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub shared_keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct EmotionAnalysisResult {
    #[serde(default)]
    pub overview: EmotionOverview,
    #[serde(default)]
    pub timeline: Vec<EmotionTimelineEntry>,
    #[serde(default)]
    pub semantic_map: EmotionSemanticMap,
    #[serde(default)]
    pub bridges: Vec<EmotionBridge>,
    #[serde(default)]
    pub reflection_prompts: Vec<String>,
    #[serde(default)]
    pub narrative_markdown: String,
}
