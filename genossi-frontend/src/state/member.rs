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
pub struct SelectedMemberIds {
    pub selected_ids: Vec<Uuid>,
}

impl SelectedMemberIds {
    pub fn is_selected(&self, id: &Uuid) -> bool {
        self.selected_ids.contains(id)
    }

    pub fn toggle(&mut self, id: Uuid) {
        if let Some(pos) = self.selected_ids.iter().position(|i| i == &id) {
            self.selected_ids.remove(pos);
        } else {
            self.selected_ids.push(id);
        }
    }

    pub fn clear(&mut self) {
        self.selected_ids.clear();
    }

    pub fn count(&self) -> usize {
        self.selected_ids.len()
    }
}
