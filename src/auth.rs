//! Роли: владельцы — только из config.admin_ids (в БД не хранятся, ботом не
//! назначаются); групповые админы — из таблицы group_admins. Резолв зовётся на
//! каждом message/callback — точечные чтения SQLite на WAL это микросекунды.

use crate::store::Store;

#[derive(Debug, Clone, PartialEq)]
pub enum Role {
    /// Полный доступ: клиенты, группы, глобальные операции.
    Owner,
    /// CRUD клиентов только внутри перечисленных групп (id, сортировка по имени).
    GroupAdmin(Vec<i64>),
    /// Не владелец и не групповой админ.
    Denied,
}

impl Role {
    pub fn is_owner(&self) -> bool {
        matches!(self, Role::Owner)
    }

    /// Видимость клиента: владелец видит всех; групповой админ — только
    /// клиентов своих групп (клиенты без группы ему не видны).
    pub fn can_see_client(&self, group: Option<i64>) -> bool {
        match self {
            Role::Owner => true,
            Role::GroupAdmin(groups) => group.is_some_and(|g| groups.contains(&g)),
            Role::Denied => false,
        }
    }
}

pub fn resolve_role(user_id: i64, admin_ids: &[i64], store: &Store) -> Role {
    if admin_ids.contains(&user_id) {
        return Role::Owner;
    }
    let groups = store.admin_group_ids(user_id);
    if groups.is_empty() {
        Role::Denied
    } else {
        Role::GroupAdmin(groups)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;

    #[test]
    fn owner_from_config_wins() {
        let store = Store::open_in_memory();
        assert!(matches!(
            resolve_role(111, &[111, 222], &store),
            Role::Owner
        ));
    }

    #[test]
    fn group_admin_from_db() {
        let store = Store::open_in_memory();
        let g = store.create_group("g", 0).unwrap();
        store.add_group_admin(g, 42, 1, 10);
        match resolve_role(42, &[111], &store) {
            Role::GroupAdmin(groups) => assert_eq!(groups, vec![g]),
            other => panic!("ожидался GroupAdmin, получен {other:?}"),
        }
    }

    #[test]
    fn stranger_denied() {
        let store = Store::open_in_memory();
        assert!(matches!(resolve_role(999, &[111], &store), Role::Denied));
    }

    #[test]
    fn owner_precedes_group_admin() {
        // Владелец, даже будучи записан админом группы, остаётся Owner.
        let store = Store::open_in_memory();
        let g = store.create_group("g", 0).unwrap();
        store.add_group_admin(g, 111, 1, 10);
        assert!(matches!(resolve_role(111, &[111], &store), Role::Owner));
    }

    #[test]
    fn scope_checks() {
        assert!(Role::Owner.can_see_client(None));
        assert!(Role::Owner.can_see_client(Some(5)));
        let ga = Role::GroupAdmin(vec![1, 2]);
        assert!(ga.can_see_client(Some(1)));
        assert!(!ga.can_see_client(Some(3)));
        assert!(!ga.can_see_client(None)); // клиенты без группы админу не видны
        assert!(!Role::Denied.can_see_client(None));
    }
}
