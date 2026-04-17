use crate::router::Route;
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub struct StatusBarItem {
    pub label_with_count: Rc<str>,
    pub label_none: Rc<str>,
    pub count: Option<usize>,
    pub route: Route,
}

#[component]
pub fn StatusBar(items: Vec<StatusBarItem>) -> Element {
    rsx! {
        div { class: "flex items-center gap-2 text-sm text-gray-600 py-2",
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    span { class: "text-gray-400", "\u{2022}" }
                }
                Link {
                    to: item.route.clone(),
                    class: "hover:text-blue-600 hover:underline",
                    {format_item(item)}
                }
            }
        }
    }
}

fn format_item(item: &StatusBarItem) -> String {
    match item.count {
        Some(0) => item.label_none.to_string(),
        Some(n) => item.label_with_count.replace("{}", &n.to_string()),
        None => "\u{2014}".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(count: Option<usize>) -> StatusBarItem {
        StatusBarItem {
            label_with_count: "{} open applications".into(),
            label_none: "No open applications".into(),
            count,
            route: Route::ApplicationsPage {},
        }
    }

    #[test]
    fn format_item_with_count() {
        let item = make_item(Some(3));
        assert_eq!(format_item(&item), "3 open applications");
    }

    #[test]
    fn format_item_zero_shows_none_label() {
        let item = make_item(Some(0));
        assert_eq!(format_item(&item), "No open applications");
    }

    #[test]
    fn format_item_none_shows_dash() {
        let item = make_item(None);
        assert_eq!(format_item(&item), "\u{2014}");
    }

    #[test]
    fn format_item_route_matches() {
        let item = make_item(Some(5));
        assert_eq!(item.route, Route::ApplicationsPage {});
    }
}
