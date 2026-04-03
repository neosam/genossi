use rest_types::MemberTO;

pub fn is_active(member: &MemberTO, reference_date: &time::Date) -> bool {
    member.is_active(reference_date)
}

pub fn exited_in_year(member: &MemberTO, reference_date: &time::Date) -> bool {
    member.exited_in_year(reference_date)
}

pub fn today() -> time::Date {
    let today = js_sys::Date::new_0();
    let year = today.get_full_year() as i32;
    let month: time::Month = (today.get_month() as u8 + 1)
        .try_into()
        .unwrap_or(time::Month::January);
    let day = today.get_date() as u8;
    time::Date::from_calendar_date(year, month, day)
        .unwrap_or_else(|_| time::Date::from_calendar_date(2025, time::Month::January, 1).unwrap())
}
