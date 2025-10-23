use crate::api;
use crate::service::config::CONFIG;
use crate::state::DuplicateDetection;
use dioxus::prelude::*;
use rest_types::{CheckDuplicateRequestTO, DuplicateDetectionConfigTO};

pub static DUPLICATE_DETECTION: GlobalSignal<DuplicateDetection> = GlobalSignal::new(DuplicateDetection::default);

#[derive(Debug)]
#[allow(dead_code)]
pub enum DuplicateDetectionService {
    FindAllDuplicates,
    FindDuplicatesByEan(String),
    CheckDuplicates(CheckDuplicateRequestTO),
    UpdateConfig(DuplicateDetectionConfigTO),
    ClearError,
}

pub fn duplicate_detection_service() {
    spawn(async move {
        let config = CONFIG.read().clone();
        if !config.backend.is_empty() {
            // Initialize with default config
            DUPLICATE_DETECTION.write().config = DuplicateDetectionConfigTO {
                similarity_threshold: 0.55,
                exact_match_weight: 0.3,
                word_order_weight: 0.4,
                levenshtein_weight: 0.2,
                jaro_winkler_weight: 0.1,
                category_aware: true,
            };
        }
    });
}

pub async fn find_all_duplicates() {
    let config = CONFIG.read().clone();
    if config.backend.is_empty() {
        return;
    }

    DUPLICATE_DETECTION.write().loading = true;
    DUPLICATE_DETECTION.write().error = None;

    match api::find_all_duplicates(&config).await {
        Ok(duplicates) => {
            DUPLICATE_DETECTION.write().all_duplicates = duplicates;
            DUPLICATE_DETECTION.write().error = None;
        }
        Err(e) => {
            DUPLICATE_DETECTION.write().error = Some(format!("Failed to find duplicates: {}", e));
        }
    }
    DUPLICATE_DETECTION.write().loading = false;
}

pub async fn find_duplicates_by_ean(ean: String) {
    let config = CONFIG.read().clone();
    if config.backend.is_empty() {
        return;
    }

    DUPLICATE_DETECTION.write().loading = true;
    DUPLICATE_DETECTION.write().error = None;

    match api::find_duplicates_by_ean(&config, &ean).await {
        Ok(result) => {
            DUPLICATE_DETECTION.write().current_duplicates = Some(result);
            DUPLICATE_DETECTION.write().error = None;
        }
        Err(e) => {
            DUPLICATE_DETECTION.write().error = Some(format!("Failed to find duplicates for {}: {}", ean, e));
        }
    }
    DUPLICATE_DETECTION.write().loading = false;
}

pub async fn check_duplicates(request: CheckDuplicateRequestTO) {
    let config = CONFIG.read().clone();
    if config.backend.is_empty() {
        return;
    }

    DUPLICATE_DETECTION.write().loading = true;
    DUPLICATE_DETECTION.write().error = None;

    match api::check_duplicates(&config, request).await {
        Ok(matches) => {
            DUPLICATE_DETECTION.write().potential_matches = matches;
            DUPLICATE_DETECTION.write().error = None;
        }
        Err(e) => {
            DUPLICATE_DETECTION.write().error = Some(format!("Failed to check duplicates: {}", e));
        }
    }
    DUPLICATE_DETECTION.write().loading = false;
}

pub fn update_config(config: DuplicateDetectionConfigTO) {
    DUPLICATE_DETECTION.write().config = config;
}

pub fn clear_error() {
    DUPLICATE_DETECTION.write().error = None;
}