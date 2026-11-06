//! Init selects immutable, build-time Linux launch profiles. Guest arguments
//! cannot choose credentials or change the executable of another process.
use alloc::vec::Vec;

pub const MAX_PROFILES: usize = 8;
pub const SECURITY_CONTEXTS: [&str; 8] = [
    "u:r:bndroid_profile_0:s0",
    "u:r:bndroid_profile_1:s0",
    "u:r:bndroid_profile_2:s0",
    "u:r:bndroid_profile_3:s0",
    "u:r:bndroid_profile_4:s0",
    "u:r:bndroid_profile_5:s0",
    "u:r:bndroid_profile_6:s0",
    "u:r:bndroid_profile_7:s0",
];
pub use bndr_abi::PROCESS_SPAWN_LINUX_PROFILE_SHIFT as PROFILE_SHIFT;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Credentials {
    pub uid: u32,
    pub gid: u32,
}
impl Credentials {
    pub const APPLICATION: Self = Self {
        uid: 10000,
        gid: 10000,
    };
}

#[derive(Clone, Copy)]
pub struct Profile {
    pub arguments: &'static [&'static [u8]],
    pub environment: &'static [&'static [u8]],
    pub credentials: Credentials,
    pub manager_domains: u8,
    pub service_grants: &'static [bndr_sm::service_authority::Grant<'static>],
    pub property_grants: &'static [&'static str],
}

pub fn profile_index(flags: u64, linux: bool, count: usize) -> Option<usize> {
    let index = (flags >> PROFILE_SHIFT) as usize;
    (flags as u32 == 0 && index < count && (linux || index == 0)).then_some(index)
}

pub struct Parsed<'a> {
    pub arguments: Vec<&'a str>,
    pub environment: Vec<&'a str>,
    pub credentials: Credentials,
    pub manager_domains: u8,
    pub service_grants: Vec<bndr_sm::service_authority::Grant<'a>>,
    pub property_grants: Vec<&'a str>,
}
impl Default for Parsed<'_> {
    fn default() -> Self {
        Self {
            arguments: Vec::new(),
            environment: Vec::new(),
            credentials: Credentials::APPLICATION,
            manager_domains: 7,
            service_grants: Vec::new(),
            property_grants: Vec::new(),
        }
    }
}

/// Parse the manifest used by the kernel build, including the legacy single
/// profile format. No guest-writable file is consulted at process creation.
pub fn parse(input: &str) -> Result<Vec<Parsed<'_>>, &'static str> {
    let mut profiles = Vec::new();
    let mut current = Parsed::default();
    let (mut uid_seen, mut gid_seen, mut domains_seen, mut section_seen) =
        (false, false, false, false);
    for line in input
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        if line.contains('\0') || line.contains('\r') {
            return Err("NUL or carriage return in Linux launch configuration");
        }
        let (key, value) = line.split_once('=').ok_or("expected key=value")?;
        match key {
            "profile" => {
                let index = value
                    .parse::<usize>()
                    .map_err(|_| "invalid profile index")?;
                if section_seen
                    || !current.arguments.is_empty()
                    || !current.environment.is_empty()
                    || uid_seen
                    || gid_seen
                    || domains_seen
                    || !current.service_grants.is_empty()
                    || !current.property_grants.is_empty()
                {
                    validate(&current)?;
                    profiles.push(current);
                    current = Parsed::default();
                }
                if index != profiles.len() || index >= MAX_PROFILES {
                    return Err("profiles must be consecutive from zero and bounded");
                }
                section_seen = true;
                uid_seen = false;
                gid_seen = false;
                domains_seen = false;
            }
            "arg" => current.arguments.push(value),
            "service_grant" => {
                let (rights, target) = value
                    .split_once(' ')
                    .ok_or("service grant needs mask and context")?;
                let rights = rights
                    .parse::<u32>()
                    .map_err(|_| "invalid service rights")?;
                if rights == 0
                    || rights & !bndr_sm::service_authority::ALL != 0
                    || target.len() > 160
                    || !target.starts_with("u:")
                    || !target
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"_:,.".contains(&b))
                    || current.service_grants.len() >= 32
                    || current.service_grants.iter().any(|g| g.target == target)
                {
                    return Err("invalid or duplicate service grant");
                }
                current
                    .service_grants
                    .push(bndr_sm::service_authority::Grant { target, rights });
            }
            "property_grant" => {
                if !bndr_sm::service_authority::property_name(value)
                    || value.starts_with("ro.")
                    || current.property_grants.len() >= 16
                    || current.property_grants.contains(&value)
                {
                    return Err("invalid or duplicate property grant");
                }
                current.property_grants.push(value);
            }
            "env" => {
                let (name, _) = value
                    .split_once('=')
                    .ok_or("environment needs NAME=value")?;
                if name.is_empty()
                    || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                    || current
                        .environment
                        .iter()
                        .any(|old| old.split_once('=').unwrap().0 == name)
                {
                    return Err("invalid or duplicate environment name");
                }
                current.environment.push(value);
            }
            "binder_managers" => {
                let mask = value
                    .parse::<u8>()
                    .map_err(|_| "invalid Binder domain mask")?;
                if domains_seen || mask & !7 != 0 {
                    return Err("duplicate or invalid Binder domain mask");
                }
                domains_seen = true;
                current.manager_domains = mask;
            }
            "uid" | "gid" => {
                let number = value.parse::<u32>().map_err(|_| "invalid credential")?;
                let seen = if key == "uid" {
                    &mut uid_seen
                } else {
                    &mut gid_seen
                };
                if *seen || number == u32::MAX {
                    return Err("duplicate or invalid credential");
                }
                *seen = true;
                if key == "uid" {
                    current.credentials.uid = number;
                } else {
                    current.credentials.gid = number;
                }
            }
            _ => return Err("unknown launch configuration field"),
        }
        if current.arguments.len() > 32 || current.environment.len() > 32 {
            return Err("too many Linux launch entries");
        }
    }
    validate(&current)?;
    profiles.push(current);
    Ok(profiles)
}

fn validate(profile: &Parsed<'_>) -> Result<(), &'static str> {
    if profile.arguments.first().copied() != Some("/apex/com.android.runtime/bin/linker64") {
        return Err("first argument must select the embedded interpreter");
    }
    let executable = profile.arguments.get(1).ok_or("missing executable")?;
    if !executable.starts_with('/')
        || executable
            .split('/')
            .skip(1)
            .any(|p| p.is_empty() || p == "." || p == "..")
    {
        return Err("second argument must be a canonical absolute executable path");
    }
    let bytes = (1 + profile.arguments.len() + 1 + profile.environment.len() + 1 + 18 * 2) * 8
        + profile
            .arguments
            .iter()
            .chain(&profile.environment)
            .map(|s| s.len() + 1)
            .sum::<usize>()
        + 31;
    if bytes > 4096 {
        return Err("Linux launch configuration exceeds one stack page");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;
    const BASE: &str =
        "arg=/apex/com.android.runtime/bin/linker64\narg=/system/bin/service\nenv=LANG=C\n";
    #[test]
    fn profiles_keep_arguments_credentials_and_authority_separate() {
        let text = format!(
            "profile=0\nuid=1000\ngid=1000\nbinder_managers=1\n{BASE}profile=1\nbinder_managers=0\n{BASE}arg=client\n"
        );
        let p = parse(&text).unwrap();
        assert_eq!(p.len(), 2);
        assert_eq!(
            p[0].credentials,
            Credentials {
                uid: 1000,
                gid: 1000
            }
        );
        assert_eq!(p[1].credentials, Credentials::APPLICATION);
        assert_eq!((p[0].manager_domains, p[1].manager_domains), (1, 0));
        assert_eq!(p[0].arguments.len(), 2);
        assert_eq!(p[1].arguments[2], "client");
        assert_eq!(
            parse(BASE).unwrap()[0].credentials,
            Credentials::APPLICATION
        );
        assert_eq!(profile_index(1 << PROFILE_SHIFT, true, 2), Some(1));
        for (flags, linux, count) in [
            (1, true, 2),
            (1 << PROFILE_SHIFT, false, 2),
            (2 << PROFILE_SHIFT, true, 2),
        ] {
            assert_eq!(profile_index(flags, linux, count), None);
        }
    }
    #[test]
    fn invalid_profiles_fail_before_publication() {
        for tail in [
            "uid=1\nuid=2\n",
            "gid=4294967295\n",
            "binder_managers=8\n",
            "env=LANG=D\n",
            "env==empty\n",
            "profile=2\n",
            "arg=bad\0value\n",
        ] {
            assert!(parse(&format!("{BASE}{tail}")).is_err());
        }
        assert!(
            parse("arg=/apex/com.android.runtime/bin/linker64\narg=/system/../secret\n").is_err()
        );
        assert!(parse(&format!("{BASE}arg={}\n", "x".repeat(4096))).is_err());
        let mut text = alloc::string::String::new();
        for index in 0..=MAX_PROFILES {
            text += &format!("profile={index}\n{BASE}");
        }
        assert!(parse(&text).is_err());
    }
    #[test]
    fn grants_are_bounded_explicit_and_never_implied_by_uid() {
        let text = format!(
            "{BASE}uid=1000\nservice_grant=2 u:object_r:service_manager_service:s0\nproperty_grant=servicemanager.ready\n"
        );
        let p = parse(&text).unwrap();
        assert_eq!(p[0].service_grants[0].rights, 2);
        assert_eq!(p[0].property_grants, ["servicemanager.ready"]);
        assert!(
            parse(&format!("{BASE}uid=0\n")).unwrap()[0]
                .service_grants
                .is_empty()
        );
        for field in [
            "service_grant=8 u:r:invalid:s0\n",
            "service_grant=1 invalid\n",
            "property_grant=ro.secret\n",
            "property_grant=a..b\n",
            "property_grant=servicemanager.ready\n",
        ] {
            assert!(parse(&format!("{text}{field}")).is_err());
        }
    }
}
