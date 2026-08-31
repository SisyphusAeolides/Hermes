//! Kernel-portable platform HAL for Hermes GSP.
//!
//! Hosts (Boulder, Linux, test doubles) implement `HermesPlatform`. The GSP
//! state machine never touches hardware except through this trait.

use hermes_abi::gpu::GpuCompatibilityManifest;
use hermes_abi::hermes::{
    HermesBootInstruction, HermesNormalizedCommand, HermesNormalizedEvent, HermesPciIdentity,
    HermesProbeEvidence, HermesTransportProfile,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HermesFault {
    NotNvidia,
    NotDisplayController,
    UnsupportedArchitecture,
    NoPersonality,
    AmbiguousPersonality,
    PersonalityCapacity,
    PersonalityRejected,
    CompatibilityRejected,
    ProfileRejected,
    CodecRejected,
    FirmwareMissing,
    FirmwareUnexpected,
    FirmwareSize,
    FirmwareAlignment,
    FirmwareRejected,
    DeviceIsolation,
    BarUnavailable,
    MmioOutOfRange,
    MmioRead,
    MmioWrite,
    UnstableMmio,
    DmaAllocation,
    DmaAccess,
    DmaAddressOverflow,
    QueueGeometry,
    QueueFull,
    QueueCorrupt,
    PendingCapacity,
    DuplicateRequest,
    UnknownResponse,
    ResponseMismatch,
    ResponseExpired,
    ProtocolMismatch,
    RequiredFeatureMissing,
    BootFuelExhausted,
    BootInstructionRejected,
    DeadlineExpired,
    DeviceFault,
    RecoveryRequired,
    CorrelationSpaceExhausted,
    ArithmeticOverflow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DmaPurpose {
    Firmware,
    CommandRing,
    EventRing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MmioWindow<Handle: Copy + Eq> {
    pub handle: Handle,
    pub bar: u8,
    pub length: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DmaRegion<Handle: Copy + Eq> {
    pub handle: Handle,
    pub device_address: u64,
    pub length: usize,
    pub alignment: usize,
    pub purpose: DmaPurpose,
}

/// Host services required by the GSP path. No default implementations invent
/// success: missing IOMMU, MMIO, or DMA is a hard fault at the call site.
pub trait HermesPlatform: Sync {
    type Domain: Copy + Eq;
    type Mmio: Copy + Eq;
    type Dma: Copy + Eq;

    fn isolate_device(&self, identity: HermesPciIdentity) -> Result<Self::Domain, HermesFault>;
    fn release_domain(&self, domain: Self::Domain);

    fn map_bar(
        &self,
        domain: Self::Domain,
        bar: u8,
        minimum_length: u64,
    ) -> Result<MmioWindow<Self::Mmio>, HermesFault>;
    fn unmap_bar(&self, window: MmioWindow<Self::Mmio>);

    fn read32(&self, window: MmioWindow<Self::Mmio>, offset: u32) -> Result<u32, HermesFault>;
    fn write32(
        &self,
        window: MmioWindow<Self::Mmio>,
        offset: u32,
        value: u32,
    ) -> Result<(), HermesFault>;
    fn io_fence(&self) -> Result<(), HermesFault>;

    fn allocate_dma(
        &self,
        domain: Self::Domain,
        length: usize,
        alignment: usize,
        purpose: DmaPurpose,
    ) -> Result<DmaRegion<Self::Dma>, HermesFault>;
    fn release_dma(&self, region: DmaRegion<Self::Dma>);
    fn dma_write(
        &self,
        region: DmaRegion<Self::Dma>,
        offset: usize,
        bytes: &[u8],
    ) -> Result<(), HermesFault>;
    fn dma_read(
        &self,
        region: DmaRegion<Self::Dma>,
        offset: usize,
        bytes: &mut [u8],
    ) -> Result<(), HermesFault>;
    fn dma_publish(
        &self,
        region: DmaRegion<Self::Dma>,
        offset: usize,
        length: usize,
    ) -> Result<(), HermesFault>;
    fn dma_acquire(
        &self,
        region: DmaRegion<Self::Dma>,
        offset: usize,
        length: usize,
    ) -> Result<(), HermesFault>;

    fn now_tick(&self) -> u64;
    fn relax(&self);

    /// Unpredictable, phase-decorrelated backoff combining chaotic attractors.
    /// Radically increases throughput for concurrent lockless rings by breaking phase-locks.
    fn chaos_relax(&self, scheduler: &mut crate::chaos::ChaosScheduler, dt: f32) {
        let iterations = scheduler.next_interval(dt);
        for _ in 0..iterations {
            self.relax();
        }
    }
}

/// Codec personality for GSP wire encode/decode (open-RM protocol family).
pub trait HermesCodec: Sync {
    fn personality_id(&self) -> u64;
    fn compatibility_manifest(&self) -> GpuCompatibilityManifest;
    fn match_device(
        &self,
        identity: &HermesPciIdentity,
        evidence: &HermesProbeEvidence,
    ) -> Result<u32, HermesFault>;
    fn describe_transport(
        &self,
        identity: &HermesPciIdentity,
        evidence: &HermesProbeEvidence,
    ) -> Result<HermesTransportProfile, HermesFault>;
    fn boot_instruction(
        &self,
        identity: &HermesPciIdentity,
        evidence: &HermesProbeEvidence,
        stage: u32,
        index: u32,
    ) -> Result<Option<HermesBootInstruction>, HermesFault>;
    fn encode_command(
        &self,
        profile: &HermesTransportProfile,
        command: &HermesNormalizedCommand,
        output: &mut [u8],
    ) -> Result<usize, HermesFault>;
    fn decode_event(
        &self,
        profile: &HermesTransportProfile,
        input: &[u8],
    ) -> Result<HermesNormalizedEvent, HermesFault>;
    fn reset(&self, profile: &HermesTransportProfile, new_epoch: u32) -> Result<(), HermesFault>;
}
