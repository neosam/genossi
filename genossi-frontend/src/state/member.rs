use rest_types::MemberTO;
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct MemberState {
    pub items: Vec<MemberTO>,
    pub loading: bool,
    pub error: Option<String>,
    pub filter_query: String,
}

#[derive(Clone, Default)]
pub struct MemberSelectionState {
    pub selected_ids: Vec<Uuid>,
}

impl MemberSelectionState {
    pub fn toggle(&mut self, id: Uuid) {
        if let Some(pos) = self.selected_ids.iter().position(|i| *i == id) {
            self.selected_ids.remove(pos);
        } else {
            self.selected_ids.push(id);
        }
    }

    pub fn select_all(&mut self, ids: Vec<Uuid>) {
        self.selected_ids = ids;
    }

    pub fn clear(&mut self) {
        self.selected_ids.clear();
    }

    pub fn is_selected(&self, id: &Uuid) -> bool {
        self.selected_ids.contains(id)
    }

    pub fn count(&self) -> usize {
        self.selected_ids.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uuid(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    #[test]
    fn toggle_adds_and_removes() {
        let mut state = MemberSelectionState::default();
        let id = uuid(1);

        state.toggle(id);
        assert!(state.is_selected(&id));
        assert_eq!(state.count(), 1);

        state.toggle(id);
        assert!(!state.is_selected(&id));
        assert_eq!(state.count(), 0);
    }

    #[test]
    fn select_all_replaces_selection() {
        let mut state = MemberSelectionState::default();
        state.toggle(uuid(1));

        state.select_all(vec![uuid(2), uuid(3), uuid(4)]);
        assert_eq!(state.count(), 3);
        assert!(!state.is_selected(&uuid(1)));
        assert!(state.is_selected(&uuid(2)));
        assert!(state.is_selected(&uuid(4)));
    }

    #[test]
    fn clear_empties_selection() {
        let mut state = MemberSelectionState::default();
        state.select_all(vec![uuid(1), uuid(2), uuid(3)]);
        assert_eq!(state.count(), 3);

        state.clear();
        assert_eq!(state.count(), 0);
        assert!(!state.is_selected(&uuid(1)));
    }

    #[test]
    fn toggle_does_not_duplicate() {
        let mut state = MemberSelectionState::default();
        let id = uuid(1);

        state.toggle(id);
        state.toggle(id);
        state.toggle(id);
        assert!(state.is_selected(&id));
        assert_eq!(state.count(), 1);
    }
}
