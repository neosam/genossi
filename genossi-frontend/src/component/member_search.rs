use dioxus::prelude::*;
use rest_types::MemberTO;
use uuid::Uuid;

use crate::service::member::MEMBERS;

const MAX_RESULTS: usize = 10;

pub fn filter_members<'a>(
    members: &'a [MemberTO],
    query: &str,
    exclude_id: Option<Uuid>,
) -> Vec<&'a MemberTO> {
    if query.is_empty() {
        return Vec::new();
    }
    let query_lower = query.to_lowercase();
    let mut results: Vec<&MemberTO> = members
        .iter()
        .filter(|m| {
            if let Some(exc) = exclude_id {
                if m.id == Some(exc) {
                    return false;
                }
            }
            let number_str = m.member_number.to_string();
            m.first_name.to_lowercase().contains(&query_lower)
                || m.last_name.to_lowercase().contains(&query_lower)
                || number_str.contains(&query_lower)
        })
        .collect();
    results.sort_by_key(|m| m.member_number);
    results.truncate(MAX_RESULTS);
    results
}

fn format_member(m: &MemberTO) -> String {
    format!("#{} {} {}", m.member_number, m.first_name, m.last_name)
}

#[component]
pub fn MemberSearch(
    on_select: EventHandler<Option<Uuid>>,
    selected_id: Option<Uuid>,
    exclude_id: Option<Uuid>,
) -> Element {
    let mut query = use_signal(|| String::new());
    let mut show_dropdown = use_signal(|| false);

    let members_state = MEMBERS.read();
    let members = &members_state.items;

    // Find selected member for display
    let selected_member: Option<&MemberTO> = selected_id
        .and_then(|sid| members.iter().find(|m| m.id == Some(sid)));

    // Filter results
    let filtered = filter_members(members, &query.read(), exclude_id);

    rsx! {
        div {
            class: "relative",
            onfocusout: move |_| {
                // Small delay to allow click events on dropdown items to fire
                spawn(async move {
                    gloo_timers::future::TimeoutFuture::new(150).await;
                    show_dropdown.set(false);
                });
            },

            if let Some(member) = selected_member {
                // Show selected member with clear button
                div {
                    class: "flex items-center gap-2 w-full px-3 py-2 border border-gray-300 rounded-md bg-gray-50",
                    span { class: "flex-1", "{format_member(member)}" }
                    button {
                        class: "text-gray-500 hover:text-gray-700 font-bold",
                        r#type: "button",
                        onclick: move |e| {
                            e.stop_propagation();
                            query.set(String::new());
                            on_select.call(None);
                        },
                        "✕"
                    }
                }
            } else {
                // Show search input
                input {
                    class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
                    r#type: "text",
                    placeholder: "Name oder Nummer suchen...",
                    value: "{query}",
                    oninput: move |e| {
                        query.set(e.value().clone());
                        show_dropdown.set(!e.value().is_empty());
                    },
                    onfocus: move |_| {
                        if !query.read().is_empty() {
                            show_dropdown.set(true);
                        }
                    },
                }

                // Dropdown results
                if *show_dropdown.read() && !filtered.is_empty() {
                    div {
                        class: "absolute z-20 w-full mt-1 bg-white border border-gray-300 rounded-md shadow-lg max-h-60 overflow-y-auto",
                        for member in filtered.iter() {
                            {
                                let member_id = member.id;
                                let display = format_member(member);
                                rsx! {
                                    button {
                                        class: "w-full text-left px-3 py-2 hover:bg-blue-50 cursor-pointer border-b border-gray-100 last:border-b-0",
                                        r#type: "button",
                                        onmousedown: move |e| {
                                            e.stop_propagation();
                                            show_dropdown.set(false);
                                            query.set(String::new());
                                            on_select.call(member_id);
                                        },
                                        "{display}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_member(id: Uuid, number: i64, first: &str, last: &str) -> MemberTO {
        MemberTO {
            id: Some(id),
            member_number: number,
            first_name: first.to_string(),
            last_name: last.to_string(),
            email: None,
            company: None,
            comment: None,
            street: None,
            house_number: None,
            postal_code: None,
            city: None,
            join_date: time::Date::from_calendar_date(2025, time::Month::January, 1).unwrap(),
            shares_at_joining: 1,
            current_shares: 1,
            current_balance: 0,
            action_count: 0,
            migrated: false,
            exit_date: None,
            bank_account: None,
            created: None,
            deleted: None,
            version: None,
        }
    }

    fn test_members() -> Vec<MemberTO> {
        vec![
            make_member(Uuid::from_u128(1), 42, "Hans", "Müller"),
            make_member(Uuid::from_u128(2), 78, "Maria", "Müllenhoff"),
            make_member(Uuid::from_u128(3), 103, "Karl", "Schmidt"),
            make_member(Uuid::from_u128(4), 5, "Anna", "Weber"),
            make_member(Uuid::from_u128(5), 421, "Fritz", "Bauer"),
        ]
    }

    #[test]
    fn test_filter_by_last_name() {
        let members = test_members();
        let results = filter_members(&members, "müll", None);
        assert_eq!(results.len(), 2);
        // Sorted by member_number: 42 before 78
        assert_eq!(results[0].member_number, 42);
        assert_eq!(results[1].member_number, 78);
    }

    #[test]
    fn test_filter_by_first_name() {
        let members = test_members();
        let results = filter_members(&members, "anna", None);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].first_name, "Anna");
    }

    #[test]
    fn test_filter_by_number() {
        let members = test_members();
        let results = filter_members(&members, "42", None);
        assert_eq!(results.len(), 2); // 42 and 421
        assert_eq!(results[0].member_number, 42);
        assert_eq!(results[1].member_number, 421);
    }

    #[test]
    fn test_filter_exclude_id() {
        let members = test_members();
        let exclude = Uuid::from_u128(1); // Hans Müller
        let results = filter_members(&members, "müll", Some(exclude));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].member_number, 78); // Only Maria
    }

    #[test]
    fn test_filter_empty_query() {
        let members = test_members();
        let results = filter_members(&members, "", None);
        assert!(results.is_empty());
    }

    #[test]
    fn test_filter_no_match() {
        let members = test_members();
        let results = filter_members(&members, "zzzzz", None);
        assert!(results.is_empty());
    }

    #[test]
    fn test_filter_max_results() {
        // Create more than MAX_RESULTS members matching the query
        let members: Vec<MemberTO> = (0..15)
            .map(|i| make_member(Uuid::from_u128(i as u128), i + 1, "Test", "User"))
            .collect();
        let results = filter_members(&members, "test", None);
        assert_eq!(results.len(), MAX_RESULTS);
    }

    #[test]
    fn test_filter_sorted_by_number() {
        let members = test_members();
        let results = filter_members(&members, "a", None); // matches Hans, Maria, Karl, Anna
        for i in 1..results.len() {
            assert!(results[i - 1].member_number <= results[i].member_number);
        }
    }
}
