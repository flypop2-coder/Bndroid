#[cfg(feature = "app-data-runtime")]
use bndr_abi::AppDataPrincipal;
use bndr_abi::ObjectSignals;

use crate::channel::ChannelEndpoint;
use crate::event::Event;
use crate::graphics_buffer::GraphicsBuffer;
#[cfg(feature = "input-server-runtime")]
use crate::input_broker::InputCapability;
#[cfg(feature = "storage-server-runtime")]
use crate::storage_broker::StorageVolumeCapability;
use crate::surface::SurfaceCapability;
use crate::vmo::Vmo;

/// A non-transferable namespace root for one stable application principal.
///
/// Path authority never lives in a global pathname: every AppData syscall
/// first resolves one of these process-local capabilities and then applies a
/// canonical root-relative path inside that principal's namespace.
#[cfg(feature = "app-data-runtime")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppDataDirectoryCapability {
    principal: AppDataPrincipal,
}

#[cfg(feature = "app-data-runtime")]
impl AppDataDirectoryCapability {
    pub const fn new(principal: AppDataPrincipal) -> Self {
        Self { principal }
    }

    pub const fn principal(self) -> AppDataPrincipal {
        self.principal
    }
}

/// An owning reference to any kernel object that may live in a handle table or
/// travel through a Channel transfer message.
#[derive(Clone, Debug)]
pub enum KernelObject {
    Channel(ChannelEndpoint),
    Event(Event),
    GraphicsBuffer(GraphicsBuffer),
    #[cfg(feature = "input-server-runtime")]
    Input(InputCapability),
    Vmo(Vmo),
    SystemDirectory,
    #[cfg(feature = "app-data-runtime")]
    AppDataDirectory(AppDataDirectoryCapability),
    #[cfg(feature = "storage-server-runtime")]
    StorageVolume(StorageVolumeCapability),
    Surface(SurfaceCapability),
}

impl KernelObject {
    /// Returns the only signal bits that are meaningful for this object kind.
    /// Callers must reject wait requests outside this mask rather than publish
    /// a token that can never be satisfied by the underlying object.
    pub fn signal_mask(&self) -> ObjectSignals {
        match self {
            Self::Channel(_) => ObjectSignals::CHANNEL_ALL,
            Self::Event(_) => ObjectSignals::EVENT_ALL,
            Self::GraphicsBuffer(buffer) => buffer.signal_mask(),
            #[cfg(feature = "input-server-runtime")]
            Self::Input(input) => input.signal_mask(),
            Self::Vmo(_) | Self::SystemDirectory => ObjectSignals::NONE,
            #[cfg(feature = "app-data-runtime")]
            Self::AppDataDirectory(_) => ObjectSignals::NONE,
            #[cfg(feature = "storage-server-runtime")]
            Self::StorageVolume(volume) => volume.signal_mask(),
            Self::Surface(_) => ObjectSignals::SURFACE_ALL,
        }
    }

    pub fn signals(&self) -> ObjectSignals {
        match self {
            Self::Channel(endpoint) => endpoint.signals(),
            Self::Event(event) => event.signals(),
            Self::GraphicsBuffer(buffer) => buffer.signals(),
            #[cfg(feature = "input-server-runtime")]
            Self::Input(input) => input.signals(),
            Self::Vmo(_) | Self::SystemDirectory => ObjectSignals::NONE,
            #[cfg(feature = "app-data-runtime")]
            Self::AppDataDirectory(_) => ObjectSignals::NONE,
            #[cfg(feature = "storage-server-runtime")]
            Self::StorageVolume(volume) => volume.signals(),
            Self::Surface(surface) => surface.signals(),
        }
    }

    pub const fn as_channel(&self) -> Option<&ChannelEndpoint> {
        match self {
            Self::Channel(endpoint) => Some(endpoint),
            _ => None,
        }
    }

    pub const fn as_event(&self) -> Option<&Event> {
        match self {
            Self::Event(event) => Some(event),
            _ => None,
        }
    }

    pub const fn as_graphics_buffer(&self) -> Option<&GraphicsBuffer> {
        match self {
            Self::GraphicsBuffer(buffer) => Some(buffer),
            _ => None,
        }
    }

    pub const fn as_vmo(&self) -> Option<&Vmo> {
        match self {
            Self::Vmo(vmo) => Some(vmo),
            _ => None,
        }
    }

    pub const fn is_system_directory(&self) -> bool {
        matches!(self, Self::SystemDirectory)
    }

    #[cfg(feature = "app-data-runtime")]
    pub const fn as_app_data_directory(&self) -> Option<&AppDataDirectoryCapability> {
        match self {
            Self::AppDataDirectory(directory) => Some(directory),
            _ => None,
        }
    }

    #[cfg(feature = "storage-server-runtime")]
    pub const fn as_storage_volume(&self) -> Option<&StorageVolumeCapability> {
        match self {
            Self::StorageVolume(volume) => Some(volume),
            _ => None,
        }
    }

    pub const fn as_surface(&self) -> Option<&SurfaceCapability> {
        match self {
            Self::Surface(surface) => Some(surface),
            _ => None,
        }
    }

    #[cfg(feature = "input-server-runtime")]
    pub const fn as_input(&self) -> Option<&InputCapability> {
        match self {
            Self::Input(input) => Some(input),
            _ => None,
        }
    }

    pub fn into_channel(self) -> Result<ChannelEndpoint, Self> {
        match self {
            Self::Channel(endpoint) => Ok(endpoint),
            object => Err(object),
        }
    }

    pub fn into_event(self) -> Result<Event, Self> {
        match self {
            Self::Event(event) => Ok(event),
            object => Err(object),
        }
    }

    pub fn into_vmo(self) -> Result<Vmo, Self> {
        match self {
            Self::Vmo(vmo) => Ok(vmo),
            object => Err(object),
        }
    }

    pub fn into_graphics_buffer(self) -> Result<GraphicsBuffer, Self> {
        match self {
            Self::GraphicsBuffer(buffer) => Ok(buffer),
            object => Err(object),
        }
    }

    pub fn same_object(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Channel(left), Self::Channel(right)) => left.same_endpoint(right),
            (Self::Event(left), Self::Event(right)) => left.same_event(right),
            (Self::GraphicsBuffer(left), Self::GraphicsBuffer(right)) => left.same_buffer(right),
            #[cfg(feature = "input-server-runtime")]
            (Self::Input(left), Self::Input(right)) => left.same_input(right),
            (Self::Vmo(left), Self::Vmo(right)) => left.same_vmo(right),
            (Self::SystemDirectory, Self::SystemDirectory) => true,
            #[cfg(feature = "app-data-runtime")]
            (Self::AppDataDirectory(left), Self::AppDataDirectory(right)) => left == right,
            #[cfg(feature = "storage-server-runtime")]
            (Self::StorageVolume(left), Self::StorageVolume(right)) => left.same_volume(*right),
            (Self::Surface(left), Self::Surface(right)) => left.same_surface(right),
            _ => false,
        }
    }
}

impl From<ChannelEndpoint> for KernelObject {
    fn from(endpoint: ChannelEndpoint) -> Self {
        Self::Channel(endpoint)
    }
}

impl From<Event> for KernelObject {
    fn from(event: Event) -> Self {
        Self::Event(event)
    }
}

impl From<Vmo> for KernelObject {
    fn from(vmo: Vmo) -> Self {
        Self::Vmo(vmo)
    }
}

#[cfg(feature = "storage-server-runtime")]
impl From<StorageVolumeCapability> for KernelObject {
    fn from(volume: StorageVolumeCapability) -> Self {
        Self::StorageVolume(volume)
    }
}

#[cfg(feature = "app-data-runtime")]
impl From<AppDataDirectoryCapability> for KernelObject {
    fn from(directory: AppDataDirectoryCapability) -> Self {
        Self::AppDataDirectory(directory)
    }
}

impl From<GraphicsBuffer> for KernelObject {
    fn from(buffer: GraphicsBuffer) -> Self {
        Self::GraphicsBuffer(buffer)
    }
}

#[cfg(feature = "input-server-runtime")]
impl From<InputCapability> for KernelObject {
    fn from(input: InputCapability) -> Self {
        Self::Input(input)
    }
}

impl From<SurfaceCapability> for KernelObject {
    fn from(surface: SurfaceCapability) -> Self {
        Self::Surface(surface)
    }
}

#[cfg(test)]
mod tests {
    use super::KernelObject;
    use crate::channel::ChannelEndpoint;
    use crate::event::Event;
    use crate::graphics_buffer::{GraphicsBuffer, test_serial_guard};
    use crate::surface::SurfaceCapability;
    use crate::vmo::Vmo;
    use bndr_abi::ObjectSignals;

    #[test]
    fn dispatches_signals_for_each_object_kind() {
        let _serial = test_serial_guard();
        let event = Event::new();
        let event_object = KernelObject::from(event.clone());
        assert_eq!(event_object.signal_mask(), ObjectSignals::EVENT_ALL);
        assert_eq!(event_object.signals(), ObjectSignals::NONE);
        event.signal();
        assert_eq!(event_object.signals(), ObjectSignals::SIGNALED);

        let (left, _right) = ChannelEndpoint::pair();
        let channel_object = KernelObject::from(left);
        assert_eq!(channel_object.signal_mask(), ObjectSignals::CHANNEL_ALL);
        assert_eq!(channel_object.signals(), ObjectSignals::WRITABLE);

        let vmo_object = KernelObject::from(Vmo::try_from_slice(b"immutable").unwrap());
        assert_eq!(vmo_object.signal_mask(), ObjectSignals::NONE);
        assert_eq!(vmo_object.signals(), ObjectSignals::NONE);

        let graphics_object = KernelObject::from(GraphicsBuffer::try_new(101).unwrap());
        assert_eq!(graphics_object.signal_mask(), ObjectSignals::NONE);
        assert_eq!(graphics_object.signals(), ObjectSignals::NONE);

        let mapped_graphics = KernelObject::from(GraphicsBuffer::try_new_mappable(102).unwrap());
        assert_eq!(
            mapped_graphics.signal_mask(),
            ObjectSignals::from_bits(
                ObjectSignals::READABLE.bits() | ObjectSignals::WRITABLE.bits()
            )
            .unwrap()
        );
        assert_eq!(mapped_graphics.signals(), ObjectSignals::WRITABLE);
        let mapped = mapped_graphics.as_graphics_buffer().unwrap();
        assert_eq!(mapped.queue(0), Ok(1));
        assert_eq!(mapped_graphics.signals(), ObjectSignals::READABLE);
        assert_eq!(mapped.acquire(1, 0x0000_0001_0000_0006), Ok(1));
        assert_eq!(mapped_graphics.signals(), ObjectSignals::NONE);
        assert_eq!(mapped.release(1, 0x0000_0001_0000_0006), Ok(1));
        assert_eq!(mapped_graphics.signals(), ObjectSignals::WRITABLE);

        let directory = KernelObject::SystemDirectory;
        assert_eq!(directory.signal_mask(), ObjectSignals::NONE);
        assert_eq!(directory.signals(), ObjectSignals::NONE);

        let surface = SurfaceCapability::try_new(1).unwrap();
        let surface_object = KernelObject::from(surface.clone());
        assert_eq!(surface_object.signal_mask(), ObjectSignals::SURFACE_ALL);
        assert_eq!(surface_object.signals(), ObjectSignals::NONE);
        surface.signal_readable();
        assert_eq!(surface_object.signals(), ObjectSignals::READABLE);
        surface.signal_key_ready();
        assert_eq!(
            surface_object.signals(),
            ObjectSignals::from_bits(
                ObjectSignals::READABLE.bits() | ObjectSignals::KEY_READY.bits()
            )
            .unwrap()
        );
        assert!(
            surface_object
                .signal_mask()
                .contains(ObjectSignals::KEY_READY)
        );
    }

    #[test]
    fn object_identity_includes_kind_and_endpoint_side() {
        let _serial = test_serial_guard();
        let event = Event::new();
        let event_clone = KernelObject::from(event.clone());
        let event_original = KernelObject::from(event);
        assert!(event_original.same_object(&event_clone));

        let (left, right) = ChannelEndpoint::pair();
        let left_clone = KernelObject::from(left.clone());
        let left = KernelObject::from(left);
        let right = KernelObject::from(right);
        assert!(left.same_object(&left_clone));
        assert!(!left.same_object(&right));
        assert!(!left.same_object(&event_original));

        let vmo = Vmo::try_from_slice(b"shared bytes").unwrap();
        let vmo_clone = KernelObject::from(vmo.clone());
        let vmo_original = KernelObject::from(vmo);
        let distinct_vmo = KernelObject::from(Vmo::try_from_slice(b"shared bytes").unwrap());
        assert!(vmo_original.same_object(&vmo_clone));
        assert!(!vmo_original.same_object(&distinct_vmo));

        let directory = KernelObject::SystemDirectory;
        let other_directory = KernelObject::SystemDirectory;
        assert!(directory.same_object(&other_directory));
        for non_directory in [&left, &event_original, &vmo_original] {
            assert!(!directory.same_object(non_directory));
            assert!(!non_directory.same_object(&directory));
        }
        assert!(!vmo_original.same_object(&event_original));
        assert!(!event_original.same_object(&vmo_original));
        assert!(!vmo_original.same_object(&left));
        assert!(!left.same_object(&vmo_original));

        let buffer = GraphicsBuffer::try_new(102).unwrap();
        let buffer_clone = KernelObject::from(buffer.clone());
        let buffer_original = KernelObject::from(buffer);
        let distinct_buffer = KernelObject::from(GraphicsBuffer::try_new(103).unwrap());
        assert!(buffer_original.same_object(&buffer_clone));
        assert!(!buffer_original.same_object(&distinct_buffer));
        assert!(!buffer_original.same_object(&vmo_original));

        let surface = SurfaceCapability::try_new(1).unwrap();
        let surface_clone = KernelObject::from(surface.clone());
        let surface_original = KernelObject::from(surface);
        let distinct_surface = KernelObject::from(SurfaceCapability::try_new(2).unwrap());
        assert!(surface_original.same_object(&surface_clone));
        assert!(!surface_original.same_object(&distinct_surface));
        assert!(!surface_original.same_object(&directory));
    }

    #[test]
    fn vmo_and_system_directory_accessors_preserve_object_kind() {
        let _serial = test_serial_guard();
        let vmo = Vmo::try_from_slice(b"file contents").unwrap();
        let object = KernelObject::from(vmo.clone());
        assert!(object.as_channel().is_none());
        assert!(object.as_event().is_none());
        assert!(object.as_graphics_buffer().is_none());
        assert!(object.as_vmo().unwrap().same_vmo(&vmo));
        assert!(!object.is_system_directory());
        assert!(object.into_vmo().unwrap().same_vmo(&vmo));

        let directory = KernelObject::SystemDirectory;
        assert!(directory.as_channel().is_none());
        assert!(directory.as_event().is_none());
        assert!(directory.as_graphics_buffer().is_none());
        assert!(directory.as_vmo().is_none());
        assert!(directory.is_system_directory());
        assert!(directory.as_surface().is_none());
        let directory = directory.into_vmo().unwrap_err();
        assert!(directory.is_system_directory());

        let surface = SurfaceCapability::try_new(1).unwrap();
        let object = KernelObject::from(surface.clone());
        assert!(object.as_surface().unwrap().same_surface(&surface));
        assert!(object.as_channel().is_none());
        assert!(object.as_event().is_none());
        assert!(object.as_graphics_buffer().is_none());
        assert!(object.as_vmo().is_none());
        assert!(!object.is_system_directory());

        let buffer = GraphicsBuffer::try_new(104).unwrap();
        let object = KernelObject::from(buffer.clone());
        assert!(object.as_graphics_buffer().unwrap().same_buffer(&buffer));
        assert!(object.as_channel().is_none());
        assert!(object.as_event().is_none());
        assert!(object.as_vmo().is_none());
        assert!(object.as_surface().is_none());
        assert!(!object.is_system_directory());
        assert!(object.into_graphics_buffer().unwrap().same_buffer(&buffer));
    }
}
