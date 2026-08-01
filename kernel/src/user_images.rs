use bndr_abi::UserImageId;

static INIT_ELF: &[u8] = include_bytes!(env!("BNDR_INIT_ELF"));
static SERVICE_MANAGER_ELF: &[u8] = include_bytes!(env!("BNDR_SERVICE_MANAGER_ELF"));
static PROVIDER_ELF: &[u8] = include_bytes!(env!("BNDR_ECHO_PROVIDER_ELF"));
static CLIENT_ELF: &[u8] = include_bytes!(env!("BNDR_ECHO_CLIENT_ELF"));
static SURFACE_SERVER_ELF: &[u8] = include_bytes!(env!("BNDR_SURFACE_SERVER_ELF"));
static LAUNCHER_ELF: &[u8] = include_bytes!(env!("BNDR_LAUNCHER_ELF"));
static APP_ELF: &[u8] = include_bytes!(env!("BNDR_APP_ELF"));
#[cfg(feature = "androidbox-process0")]
static ANDROID_APP_ELF: &[u8] = include_bytes!(env!("BNDR_ANDROID_APP_ELF"));
#[cfg(feature = "input-server-runtime")]
static INPUT_SERVER_ELF: &[u8] = include_bytes!(env!("BNDR_INPUT_SERVER_ELF"));
#[cfg(feature = "storage-server-runtime")]
static STORAGE_SERVER_ELF: &[u8] = include_bytes!(env!("BNDR_STORAGE_SERVER_ELF"));

pub const IMAGE_COUNT: usize = 7
    + cfg!(feature = "androidbox-process0") as usize
    + cfg!(feature = "input-server-runtime") as usize
    + cfg!(feature = "storage-server-runtime") as usize;

#[derive(Clone, Copy)]
pub struct EmbeddedUserImage {
    pub id: UserImageId,
    pub bytes: &'static [u8],
    pub digest: u64,
}

#[derive(Clone, Copy)]
pub struct UserImageCatalogSnapshot {
    pub count: usize,
    pub lengths: [usize; IMAGE_COUNT],
    pub digests: [u64; IMAGE_COUNT],
    pub nonempty: bool,
    pub distinct: bool,
}

pub fn get(id: UserImageId) -> EmbeddedUserImage {
    let bytes: &'static [u8] = match id {
        UserImageId::Init => INIT_ELF,
        UserImageId::ServiceManager => SERVICE_MANAGER_ELF,
        UserImageId::Provider => PROVIDER_ELF,
        UserImageId::Client => CLIENT_ELF,
        UserImageId::SurfaceServer => SURFACE_SERVER_ELF,
        UserImageId::Launcher => LAUNCHER_ELF,
        UserImageId::App => APP_ELF,
        #[cfg(feature = "androidbox-process0")]
        UserImageId::AndroidApp => ANDROID_APP_ELF,
        #[cfg(feature = "input-server-runtime")]
        UserImageId::InputServer => INPUT_SERVER_ELF,
        #[cfg(not(feature = "input-server-runtime"))]
        UserImageId::InputServer => panic!("InputServer image is not embedded in this profile"),
        #[cfg(feature = "storage-server-runtime")]
        UserImageId::StorageServer => STORAGE_SERVER_ELF,
        #[cfg(not(feature = "storage-server-runtime"))]
        UserImageId::StorageServer => panic!("StorageServer image is not embedded in this profile"),
    };
    EmbeddedUserImage {
        id,
        bytes,
        digest: fnv1a64(bytes),
    }
}

pub fn snapshot() -> UserImageCatalogSnapshot {
    let images = [
        get(UserImageId::Init),
        get(UserImageId::ServiceManager),
        get(UserImageId::Provider),
        get(UserImageId::Client),
        get(UserImageId::SurfaceServer),
        get(UserImageId::Launcher),
        get(UserImageId::App),
        #[cfg(feature = "input-server-runtime")]
        get(UserImageId::InputServer),
        #[cfg(feature = "storage-server-runtime")]
        get(UserImageId::StorageServer),
        #[cfg(feature = "androidbox-process0")]
        get(UserImageId::AndroidApp),
    ];
    let lengths = images.map(|image| image.bytes.len());
    let digests = images.map(|image| image.digest);
    let nonempty =
        lengths.iter().all(|length| *length != 0) && digests.iter().all(|digest| *digest != 0);
    let distinct = digests
        .iter()
        .enumerate()
        .all(|(index, digest)| digests[..index].iter().all(|other| other != digest));
    UserImageCatalogSnapshot {
        count: IMAGE_COUNT,
        lengths,
        digests,
        nonempty,
        distinct,
    }
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
