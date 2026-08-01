//! Fail-closed platform shutdown contract for M66 and M70.
//!
//! This module validates the complete kernel-side proof before the binary may
//! enter a platform backend. It deliberately contains no hardware operation.
//! Historical profiles use the AArch64 QEMU semihosting backend; M70 binds a
//! strictly discovered and version-probed QEMU PSCI conduit into the same
//! opaque validated token.

use bndr_abi::SHUTDOWN_SERVICE_ALL_MASK;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::fdt::{PsciCompatibleVersion, PsciMethod};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Backend {
    Unsupported = 0,
    QemuSemihosting = 1,
    QemuPsci = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PsciDescriptor {
    pub method: PsciMethod,
    pub compatible: PsciCompatibleVersion,
    pub version: u32,
}

impl PsciDescriptor {
    pub const fn major(self) -> u16 {
        (self.version >> 16) as u16
    }

    pub const fn minor(self) -> u16 {
        self.version as u16
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Contract {
    pub backend: Backend,
    pub psci: Option<PsciDescriptor>,
    pub generation: u64,
    pub shutdown_sealed: bool,
    pub registered_mask: u64,
    pub quiesced_mask: u64,
    pub live_processes: u64,
    pub live_dynamic_contexts: usize,
    pub live_dynamic_stacks: usize,
    pub storage_admission_closed: bool,
    pub storage_requests_terminal: bool,
    pub block_irq_armed: bool,
    pub local_irq_masked: bool,
    pub durable_boot_open: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContractError {
    UnsupportedBackend,
    UnexpectedPsciDescriptor,
    MissingPsciDescriptor,
    UnsupportedPsciVersion,
    InvalidGeneration,
    ShutdownNotSealed,
    ServiceGraphIncomplete,
    ProcessesLive,
    DynamicResourcesLive,
    StorageAdmissionOpen,
    StorageRequestsLive,
    BlockIrqArmed,
    LocalIrqEnabled,
    DurableSessionOpen,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedShutdown {
    backend: Backend,
    psci: Option<PsciDescriptor>,
    generation: u64,
}

impl ValidatedShutdown {
    pub const fn backend(self) -> Backend {
        self.backend
    }

    pub const fn generation(self) -> u64 {
        self.generation
    }

    pub const fn psci(self) -> Option<PsciDescriptor> {
        self.psci
    }
}

pub const fn psci_version_supported(version: u32) -> bool {
    let major = version >> 16;
    let minor = version & 0xffff;
    major > 0 || minor >= 2
}

const PSCI_DESCRIPTOR_EMPTY: u64 = 0;
static INSTALLED_PSCI: AtomicU64 = AtomicU64::new(PSCI_DESCRIPTOR_EMPTY);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PsciInstallError {
    UnsupportedVersion,
    AlreadyInstalled,
}

pub fn install_psci(descriptor: PsciDescriptor) -> Result<(), PsciInstallError> {
    if !psci_version_supported(descriptor.version) {
        return Err(PsciInstallError::UnsupportedVersion);
    }
    let encoded = encode_psci(descriptor);
    INSTALLED_PSCI
        .compare_exchange(
            PSCI_DESCRIPTOR_EMPTY,
            encoded,
            Ordering::AcqRel,
            Ordering::Acquire,
        )
        .map(|_| ())
        .map_err(|_| PsciInstallError::AlreadyInstalled)
}

pub fn installed_psci() -> Option<PsciDescriptor> {
    decode_psci(INSTALLED_PSCI.load(Ordering::Acquire))
}

const fn encode_psci(descriptor: PsciDescriptor) -> u64 {
    ((descriptor.version as u64) << 32)
        | ((descriptor.compatible as u64) << 8)
        | descriptor.method as u64
}

const fn decode_psci(encoded: u64) -> Option<PsciDescriptor> {
    if encoded == PSCI_DESCRIPTOR_EMPTY {
        return None;
    }
    let method = match encoded as u8 {
        1 => PsciMethod::Hvc,
        2 => PsciMethod::Smc,
        _ => return None,
    };
    let compatible = match (encoded >> 8) as u8 {
        1 => PsciCompatibleVersion::V0_2,
        2 => PsciCompatibleVersion::V1_0,
        _ => return None,
    };
    Some(PsciDescriptor {
        method,
        compatible,
        version: (encoded >> 32) as u32,
    })
}

pub const fn validate(contract: Contract) -> Result<ValidatedShutdown, ContractError> {
    let psci = match (contract.backend, contract.psci) {
        (Backend::Unsupported, _) => return Err(ContractError::UnsupportedBackend),
        (Backend::QemuSemihosting, Some(_)) => {
            return Err(ContractError::UnexpectedPsciDescriptor);
        }
        (Backend::QemuSemihosting, None) => None,
        (Backend::QemuPsci, None) => return Err(ContractError::MissingPsciDescriptor),
        (Backend::QemuPsci, Some(descriptor)) => {
            if !psci_version_supported(descriptor.version) {
                return Err(ContractError::UnsupportedPsciVersion);
            }
            Some(descriptor)
        }
    };
    if contract.generation == 0 {
        return Err(ContractError::InvalidGeneration);
    }
    if !contract.shutdown_sealed {
        return Err(ContractError::ShutdownNotSealed);
    }
    if contract.registered_mask != SHUTDOWN_SERVICE_ALL_MASK
        || contract.quiesced_mask != SHUTDOWN_SERVICE_ALL_MASK
    {
        return Err(ContractError::ServiceGraphIncomplete);
    }
    if contract.live_processes != 1 {
        return Err(ContractError::ProcessesLive);
    }
    if contract.live_dynamic_contexts != 0 || contract.live_dynamic_stacks != 0 {
        return Err(ContractError::DynamicResourcesLive);
    }
    if !contract.storage_admission_closed {
        return Err(ContractError::StorageAdmissionOpen);
    }
    if !contract.storage_requests_terminal {
        return Err(ContractError::StorageRequestsLive);
    }
    if contract.block_irq_armed {
        return Err(ContractError::BlockIrqArmed);
    }
    if !contract.local_irq_masked {
        return Err(ContractError::LocalIrqEnabled);
    }
    if contract.durable_boot_open {
        return Err(ContractError::DurableSessionOpen);
    }
    Ok(ValidatedShutdown {
        backend: contract.backend,
        psci,
        generation: contract.generation,
    })
}

#[cfg(test)]
mod tests {
    use super::{Backend, Contract, ContractError, PsciDescriptor, validate};
    #[cfg(feature = "unified-product-psci-shutdown-runtime")]
    use super::{decode_psci, encode_psci, psci_version_supported};
    use crate::fdt::{PsciCompatibleVersion, PsciMethod};
    use bndr_abi::SHUTDOWN_SERVICE_ALL_MASK;

    const VALID: Contract = Contract {
        backend: Backend::QemuSemihosting,
        psci: None,
        generation: 6,
        shutdown_sealed: true,
        registered_mask: SHUTDOWN_SERVICE_ALL_MASK,
        quiesced_mask: SHUTDOWN_SERVICE_ALL_MASK,
        live_processes: 1,
        live_dynamic_contexts: 0,
        live_dynamic_stacks: 0,
        storage_admission_closed: true,
        storage_requests_terminal: true,
        block_irq_armed: false,
        local_irq_masked: true,
        durable_boot_open: false,
    };

    #[test]
    fn complete_contract_issues_an_opaque_qemu_token() {
        let validated = validate(VALID).unwrap();
        assert_eq!(validated.backend(), Backend::QemuSemihosting);
        assert_eq!(validated.generation(), 6);
        assert_eq!(validated.psci(), None);
    }

    #[test]
    #[cfg(feature = "unified-product-psci-shutdown-runtime")]
    fn complete_contract_binds_the_probed_psci_descriptor() {
        let descriptor = PsciDescriptor {
            method: PsciMethod::Hvc,
            compatible: PsciCompatibleVersion::V1_0,
            version: 0x0001_0001,
        };
        let validated = validate(Contract {
            backend: Backend::QemuPsci,
            psci: Some(descriptor),
            ..VALID
        })
        .unwrap();
        assert_eq!(validated.backend(), Backend::QemuPsci);
        assert_eq!(validated.psci(), Some(descriptor));
        assert_eq!(descriptor.major(), 1);
        assert_eq!(descriptor.minor(), 1);
    }

    #[test]
    #[cfg(feature = "unified-product-psci-shutdown-runtime")]
    fn psci_version_and_descriptor_encoding_are_bounded() {
        assert!(!psci_version_supported(0x0000_0001));
        assert!(psci_version_supported(0x0000_0002));
        assert!(psci_version_supported(0x0001_0000));

        for descriptor in [
            PsciDescriptor {
                method: PsciMethod::Hvc,
                compatible: PsciCompatibleVersion::V1_0,
                version: 0x0001_0001,
            },
            PsciDescriptor {
                method: PsciMethod::Smc,
                compatible: PsciCompatibleVersion::V0_2,
                version: 0x0000_0002,
            },
        ] {
            assert_eq!(decode_psci(encode_psci(descriptor)), Some(descriptor));
        }
        assert_eq!(decode_psci(0), None);
    }

    #[test]
    fn every_platform_precondition_fails_closed() {
        let cases = [
            (
                Contract {
                    backend: Backend::Unsupported,
                    ..VALID
                },
                ContractError::UnsupportedBackend,
            ),
            (
                Contract {
                    psci: Some(PsciDescriptor {
                        method: PsciMethod::Hvc,
                        compatible: PsciCompatibleVersion::V1_0,
                        version: 0x0001_0001,
                    }),
                    ..VALID
                },
                ContractError::UnexpectedPsciDescriptor,
            ),
            (
                Contract {
                    backend: Backend::QemuPsci,
                    ..VALID
                },
                ContractError::MissingPsciDescriptor,
            ),
            (
                Contract {
                    backend: Backend::QemuPsci,
                    psci: Some(PsciDescriptor {
                        method: PsciMethod::Hvc,
                        compatible: PsciCompatibleVersion::V1_0,
                        version: 0x0000_0001,
                    }),
                    ..VALID
                },
                ContractError::UnsupportedPsciVersion,
            ),
            (
                Contract {
                    generation: 0,
                    ..VALID
                },
                ContractError::InvalidGeneration,
            ),
            (
                Contract {
                    shutdown_sealed: false,
                    ..VALID
                },
                ContractError::ShutdownNotSealed,
            ),
            (
                Contract {
                    registered_mask: 0,
                    ..VALID
                },
                ContractError::ServiceGraphIncomplete,
            ),
            (
                Contract {
                    quiesced_mask: 0,
                    ..VALID
                },
                ContractError::ServiceGraphIncomplete,
            ),
            (
                Contract {
                    live_processes: 2,
                    ..VALID
                },
                ContractError::ProcessesLive,
            ),
            (
                Contract {
                    live_dynamic_contexts: 1,
                    ..VALID
                },
                ContractError::DynamicResourcesLive,
            ),
            (
                Contract {
                    live_dynamic_stacks: 1,
                    ..VALID
                },
                ContractError::DynamicResourcesLive,
            ),
            (
                Contract {
                    storage_admission_closed: false,
                    ..VALID
                },
                ContractError::StorageAdmissionOpen,
            ),
            (
                Contract {
                    storage_requests_terminal: false,
                    ..VALID
                },
                ContractError::StorageRequestsLive,
            ),
            (
                Contract {
                    block_irq_armed: true,
                    ..VALID
                },
                ContractError::BlockIrqArmed,
            ),
            (
                Contract {
                    local_irq_masked: false,
                    ..VALID
                },
                ContractError::LocalIrqEnabled,
            ),
            (
                Contract {
                    durable_boot_open: true,
                    ..VALID
                },
                ContractError::DurableSessionOpen,
            ),
        ];
        for (contract, expected) in cases {
            assert_eq!(validate(contract), Err(expected));
        }
    }
}
