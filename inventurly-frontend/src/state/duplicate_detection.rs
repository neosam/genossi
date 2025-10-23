use rest_types::{DuplicateDetectionResultTO, DuplicateMatchTO, DuplicateDetectionConfigTO};

#[derive(Clone)]
pub struct DuplicateDetection {
    pub all_duplicates: Vec<DuplicateDetectionResultTO>,
    pub current_duplicates: Option<DuplicateDetectionResultTO>,
    pub potential_matches: Vec<DuplicateMatchTO>,
    pub loading: bool,
    pub error: Option<String>,
    pub config: DuplicateDetectionConfigTO,
}

impl Default for DuplicateDetection {
    fn default() -> Self {
        Self {
            all_duplicates: Vec::new(),
            current_duplicates: None,
            potential_matches: Vec::new(),
            loading: false,
            error: None,
            config: DuplicateDetectionConfigTO {
                similarity_threshold: 0.55,
                exact_match_weight: 0.3,
                word_order_weight: 0.4,
                levenshtein_weight: 0.2,
                jaro_winkler_weight: 0.1,
                category_aware: true,
            },
        }
    }
}