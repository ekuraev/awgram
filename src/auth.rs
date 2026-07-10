pub fn is_admin(user_id: i64, admins: &[i64]) -> bool {
    admins.contains(&user_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admits_listed_and_rejects_others() {
        let admins = [111i64, 222];
        assert!(is_admin(111, &admins));
        assert!(is_admin(222, &admins));
        assert!(!is_admin(333, &admins));
        assert!(!is_admin(0, &admins));
    }
}
