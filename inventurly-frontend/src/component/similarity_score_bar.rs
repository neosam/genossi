use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct SimilarityScoreBarProps {
    pub score: f64,
    pub label: String,
}

#[component]
pub fn SimilarityScoreBar(props: SimilarityScoreBarProps) -> Element {
    let score_percentage = (props.score * 100.0) as i32;
    let bar_width = format!("{}%", score_percentage);
    
    let color_class = match props.score {
        s if s >= 0.9 => "bg-green-500",
        s if s >= 0.7 => "bg-yellow-500", 
        s if s >= 0.5 => "bg-orange-500",
        _ => "bg-red-500",
    };

    rsx! {
        div { class: "flex items-center space-x-2 mb-2",
            div { class: "w-24 text-sm font-medium text-gray-700",
                {props.label}
            }
            div { class: "flex-1 bg-gray-200 rounded-full h-4",
                div { 
                    class: format!("h-4 rounded-full {}", color_class),
                    style: format!("width: {}", bar_width),
                }
            }
            div { class: "w-16 text-sm text-gray-600 text-right",
                {format!("{:.1}%", score_percentage)}
            }
        }
    }
}