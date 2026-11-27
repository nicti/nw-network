//! A malformed wire count must not make a reader reserve memory for the
//! whole count before it has read any entry. Each case feeds a count at the
//! `WIRE_VEC_CAP` limit followed by no entry bytes and asserts the decode
//! fails with `BufferUnderrun` while no single allocation grows large.

use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};

use indexmap::{IndexMap, IndexSet};
use nw_network::hub::{ActorInstantiationParameters, SequenceNumber};
use nw_network::serialize::{
    CARRIER_ENDIAN, Codec, ContainerMarshaler, DefaultMarshaler, MapContainerMarshaler,
    MapSequenceCodec, Marshal, MarshalerError, ReadBuffer, ReplicatedContainer, SequenceCodec,
    Unmarshal, VlqU32Marshaler, WIRE_VEC_CAP, WriteBuffer, read_live_mask_batches,
};
use nw_network::states::social::ChatMutes;

struct Counting;

// One process-wide high-water mark: `check` resets and reads it, so this file
// must keep a single #[test] (parallel tests would pollute each other's reading).
static LARGEST: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        LARGEST.fetch_max(layout.size(), Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        LARGEST.fetch_max(new_size, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

/// Far below what `WIRE_VEC_CAP` entries of any of these types would need.
const SMALL: usize = 4096;

fn vlq(wb: &mut WriteBuffer, v: u32) {
    VlqU32Marshaler.marshal(wb, v);
}

fn bytes(f: impl FnOnce(&mut WriteBuffer)) -> Vec<u8> {
    let mut wb = WriteBuffer::new(CARRIER_ENDIAN);
    f(&mut wb);
    wb.into_vec()
}

fn check<T>(
    failures: &mut Vec<String>,
    name: &str,
    data: &[u8],
    decode: impl FnOnce(&mut ReadBuffer) -> Result<T, MarshalerError>,
) {
    LARGEST.store(0, Ordering::Relaxed);
    let mut rb = ReadBuffer::new(CARRIER_ENDIAN, data);
    let result = decode(&mut rb);
    let largest = LARGEST.load(Ordering::Relaxed);
    assert!(
        matches!(result, Err(MarshalerError::BufferUnderrun { .. })),
        "{name}: expected BufferUnderrun"
    );
    println!("{name}: largest single allocation {largest} bytes");
    if largest >= SMALL {
        failures.push(format!("{name}: reserved {largest} bytes up front"));
    }
}

#[test]
fn huge_count_with_tiny_buffer_reserves_nothing_large() {
    let cap = u32::try_from(WIRE_VEC_CAP).unwrap();
    let plain = bytes(|wb| vlq(wb, cap));
    let snapshot = bytes(|wb| {
        vlq(wb, 0);
        SequenceNumber::Seq(7).marshal(wb);
        vlq(wb, cap);
    });
    let delta = bytes(|wb| vlq(wb, cap));
    let mut failures = Vec::new();

    check(&mut failures, "sequence codec Vec<u64>", &plain, |rb| {
        <SequenceCodec<DefaultMarshaler<u64>> as Codec<Vec<u64>>>::unmarshal(rb)
    });
    check(&mut failures, "map sequence codec IndexMap", &plain, |rb| {
        <MapSequenceCodec<DefaultMarshaler<u32>, DefaultMarshaler<u64>> as Codec<
            IndexMap<u32, u64>,
        >>::unmarshal(rb)
    });
    check(&mut failures, "container Vec<u64>", &plain, |rb| {
        <ContainerMarshaler<u64> as Codec<Vec<u64>>>::unmarshal(rb)
    });
    check(&mut failures, "container IndexSet<u64>", &plain, |rb| {
        <ContainerMarshaler<u64> as Codec<IndexSet<u64>>>::unmarshal(rb)
    });
    check(&mut failures, "container HashSet<u64>", &plain, |rb| {
        <ContainerMarshaler<u64> as Codec<HashSet<u64>>>::unmarshal(rb)
    });
    check(&mut failures, "map container HashMap", &plain, |rb| {
        <MapContainerMarshaler<u32, u64> as Codec<HashMap<u32, u64>>>::unmarshal(rb)
    });
    check(&mut failures, "map container IndexMap", &plain, |rb| {
        <MapContainerMarshaler<u32, u64> as Codec<IndexMap<u32, u64>>>::unmarshal(rb)
    });
    check(&mut failures, "live mask batches", &[], |rb| {
        read_live_mask_batches(rb, WIRE_VEC_CAP, |_, _| Ok(0u64))
    });
    check(&mut failures, "replicated Vec snapshot", &snapshot, |rb| {
        ReplicatedContainer::<Vec<u64>>::unmarshal(rb)
    });
    check(
        &mut failures,
        "replicated IndexMap snapshot",
        &snapshot,
        ReplicatedContainer::<IndexMap<u32, u64>>::unmarshal,
    );
    check(
        &mut failures,
        "replicated HashMap snapshot",
        &snapshot,
        ReplicatedContainer::<HashMap<u32, u64>>::unmarshal,
    );
    check(&mut failures, "replicated delta", &delta, |rb| {
        ReplicatedContainer::<Vec<u64>>::unmarshal(rb)
    });
    check(
        &mut failures,
        "chat mutes snapshot strings",
        &snapshot,
        ChatMutes::unmarshal,
    );
    check(
        &mut failures,
        "chat mutes delta entries",
        &delta,
        ChatMutes::unmarshal,
    );
    check(
        &mut failures,
        "actor instantiation parameters",
        &bytes(|wb| 0xFFFFu16.marshal(wb)),
        ActorInstantiationParameters::unmarshal,
    );
    // Some entry bytes follow the count: the hint is then min(count, 16),
    // not 0, and the decode still fails once the entries run out.
    let some = bytes(|wb| {
        vlq(wb, cap);
        for _ in 0..16 {
            0u8.marshal(wb);
        }
    });
    check(
        &mut failures,
        "sequence codec Vec<u64> with 16 entry bytes",
        &some,
        <SequenceCodec<DefaultMarshaler<u64>> as Codec<Vec<u64>>>::unmarshal,
    );
    assert!(failures.is_empty(), "{failures:#?}");
}
