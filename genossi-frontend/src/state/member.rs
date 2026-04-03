use rest_types::MemberTO;

#[derive(Clone, Default)]
pub struct MemberState {
    pub items: Vec<MemberTO>,
    pub loading: bool,
    pub error: Option<String>,
    pub filter_query: String,
}
