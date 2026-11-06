//! Supervisor-issued service authority. Android class/permission names are a
//! wire projection of these grants, not a second permission database.
pub const ADD: u32 = 1;
pub const FIND: u32 = 2;
pub const LIST: u32 = 4;
pub const ALL: u32 = ADD | FIND | LIST;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Grant<'a> {
    pub target: &'a str,
    pub rights: u32,
}
pub fn allowed(grants: &[Grant<'_>], target: &str) -> u32 {
    grants
        .iter()
        .filter(|g| g.target == target)
        .fold(0, |rights, g| rights | (g.rights & ALL))
}
pub fn property_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 96
        && !name.starts_with('.')
        && !name.ends_with('.')
        && !name.contains("..")
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
pub fn may_set_property(grants: &[&str], name: &str) -> bool {
    property_name(name) && !name.starts_with("ro.") && grants.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_grants_do_not_expand_to_other_services_or_boot_properties() {
        let grants = [Grant {
            target: "manager",
            rights: FIND,
        }];
        assert_eq!(allowed(&grants, "manager"), FIND);
        assert_eq!(allowed(&grants, "manager.child"), 0);
        assert_eq!(allowed(&[], "manager"), 0);
        assert!(may_set_property(
            &["servicemanager.ready"],
            "servicemanager.ready"
        ));
        assert!(!may_set_property(&["servicemanager.ready"], "sys.powerctl"));
        assert!(!may_set_property(&["ro.foo"], "ro.foo"));
        for bad in ["", ".x", "x.", "x..y", "x/y", "x\0y"] {
            assert!(!property_name(bad));
        }
    }
}
