//! Trading post contracts: the reflected classes the contract messages carry.
//!
//! `ContractsComponentClientFacet_OnSearchContracts` (2538),
//! `OnLookupContracts` (223) and `OnLookupContractsByParticipant` (2278) hold
//! a counted sequence of optional class values; `OnCreateContract` (634),
//! `OnResolveContract` (1117), `OnExpireContract` (917),
//! `OnFullyCompleteContract` (2964) and `OnCancelContract` (2965) hold one.
//! The schema has the messages but none of the contract classes, so every
//! class value stopped the read with an unknown type index. The classes are
//! registered here, read the way the client's unmarshal functions read them.
//! Every field's wire shape is from the decompiled reader; the names are the
//! client's reflection names for the same instance offsets.
//!
//! `Contract` (312, reader NewWorld+0x2d68380, continued in
//! NewWorld+0x2d67990 and NewWorld+0x2d67b50) is the base the concrete
//! contracts read first:
//!
//! - `SellContract` (2991, NewWorld+0x2df3420): the base, `m_outpostId`,
//!   `m_gold`, `m_count` and `m_saleItemDescriptor`.
//! - `BuyContract` (2957, NewWorld+0x2d4e500): the base, `m_outpostId`,
//!   `m_gold` and `m_count`.
//! - `MakeGoodContract` (5483, NewWorld+0x2da6290): the base, `m_gold`, a
//!   u32 with no reflection name, `m_itemDescriptor`, `m_contractTitle` and
//!   `m_contractDescription`.
//!
//! Integers are the carrier's fixed-width big-endian `GridMate::Marshaler`
//! reads; counts are `VlqU32Marshaler` reads. The client rejects a count
//! above `0x2000000` (the participant map and its event lists, the perk
//! attribute set) and a perk list above 5, which `Vec` and `ArrayVec`
//! enforce on read.

use arrayvec::ArrayVec;

use crate::states::inventory::SimpleItemDescriptor;
use crate::{Marshaler, az_rtti, type_registry};

/// `Javelin::ContractParticipant` (reader NewWorld+0x2de96b0).
#[derive(Debug, Clone, Default, PartialEq, Eq, Marshaler)]
pub struct ContractParticipant {
    /// `m_id`, 16 raw bytes.
    pub id: [u8; 16],
    /// `m_type`.
    pub participant_type: i32,
}

/// `Javelin::ContractParticipantEvent` (reader NewWorld+0x2de9770).
#[derive(Debug, Clone, Default, PartialEq, Eq, Marshaler)]
pub struct ContractParticipantEvent {
    /// `m_id`.
    pub id: String,
    /// `m_status`.
    pub status: i32,
    /// `m_role`.
    pub role: i32,
    /// `m_reason`.
    pub reason: i32,
    /// `m_completionCount`.
    pub completion_count: u32,
    /// `m_itemDescriptor` (reader NewWorld+0x5ce8b20).
    pub item_descriptor: SimpleItemDescriptor,
}

/// `Javelin::ContractItemSimpleData` (reader NewWorld+0x2de94f0, continued
/// in NewWorld+0x2db7290). The wire order is not the reflection order.
#[derive(Debug, Clone, Default, PartialEq, Eq, Marshaler)]
pub struct ContractItemSimpleData {
    /// `m_itemCategory`.
    pub item_category: String,
    /// `m_itemFamily`.
    pub item_family: String,
    /// `m_itemGroup`.
    pub item_group: String,
    /// `m_itemName`.
    pub item_name: String,
    /// `m_gearScore`.
    pub gear_score: i32,
    /// `m_tier`.
    pub tier: i32,
    /// `m_perks`, at most 5 (reader NewWorld+0x2db7920).
    pub perks: ArrayVec<u32, 5>,
    /// `m_perkAttributes`, a set.
    pub perk_attributes: Vec<i32>,
    /// `m_rarityLevel`.
    pub rarity_level: i32,
    /// `m_requiredLevel`.
    pub required_level: i32,
    /// `m_perkCount`.
    pub perk_count: i32,
    /// `m_gemPerkCount`.
    pub gem_perk_count: i32,
    /// `m_itemInstanceId`, 16 raw bytes.
    pub item_instance_id: [u8; 16],
}

/// `Javelin::Contract`, the base every concrete contract reads first.
#[derive(Debug, Clone, Default, PartialEq, Eq, Marshaler)]
pub struct Contract {
    /// `m_id`, 16 raw bytes.
    pub id: [u8; 16],
    /// `m_version`.
    pub version: u32,
    /// `m_status`.
    pub status: i32,
    /// `m_creationTime`, wall-clock nanoseconds since the epoch.
    pub creation_time: u64,
    /// `m_completionTime`, wall-clock nanoseconds since the epoch.
    pub completion_time: u64,
    /// `m_lastUpdateTime`, wall-clock nanoseconds since the epoch.
    pub last_update_time: u64,
    /// `m_postingDuration`, nanoseconds (reader NewWorld+0x17b1630).
    pub posting_duration: u64,
    /// `m_visibilityCriteria`.
    pub visibility_criteria: ContractParticipant,
    /// `m_isLocalPlayerCreator`.
    pub is_local_player_creator: bool,
    /// `m_participantEventsMap`: per participant its events (reader
    /// NewWorld+0x2d67e80, event list NewWorld+0x2d68160).
    pub participant_events: Vec<(ContractParticipant, Vec<ContractParticipantEvent>)>,
    /// `m_itemData`.
    pub item_data: ContractItemSimpleData,
    /// `m_rootPlayerEntityId`.
    pub root_player_entity_id: u64,
    /// `m_containerComponentEntityId`.
    pub container_component_entity_id: u64,
    /// `m_currencyComponentEntityId`.
    pub currency_component_entity_id: u64,
    /// `m_interactorComponentEntityId`.
    pub interactor_component_entity_id: u64,
    /// `m_globalStorageComponentEntityId`.
    pub global_storage_component_entity_id: u64,
    /// `m_contractsComponentEntityId`.
    pub contracts_component_entity_id: u64,
    /// `m_invalidParticipant`.
    pub invalid_participant: ContractParticipant,
    /// `m_invalidParticipantEvent`.
    pub invalid_participant_event: ContractParticipantEvent,
    /// `m_invalidReason`.
    pub invalid_reason: i32,
}

/// `Javelin::SellContract` (2991).
#[derive(Debug, Clone, Default, PartialEq, Eq, Marshaler)]
#[az_rtti("FD50CD67-E6C9-439F-A5E1-304C2BA252C8")]
#[type_registry(2991, class)]
pub struct SellContract {
    pub contract: Contract,
    /// `m_outpostId`, 16 raw bytes.
    pub outpost_id: [u8; 16],
    /// `m_gold`.
    pub gold: u64,
    /// `m_count`.
    pub count: u32,
    /// `m_saleItemDescriptor` (reader NewWorld+0x5ce8b20).
    pub sale_item_descriptor: SimpleItemDescriptor,
}

/// `Javelin::BuyContract` (2957).
#[derive(Debug, Clone, Default, PartialEq, Eq, Marshaler)]
#[az_rtti("618FD41E-0259-42FA-AA80-90958A4E2275")]
#[type_registry(2957, class)]
pub struct BuyContract {
    pub contract: Contract,
    /// `m_outpostId`, 16 raw bytes.
    pub outpost_id: [u8; 16],
    /// `m_gold`.
    pub gold: u64,
    /// `m_count`.
    pub count: u32,
}

/// `Javelin::MakeGoodContract` (5483).
#[derive(Debug, Clone, Default, PartialEq, Eq, Marshaler)]
#[az_rtti("ED8EB27E-69E8-4BE3-AD8D-8735A3E0FF59")]
#[type_registry(5483, class)]
pub struct MakeGoodContract {
    pub contract: Contract,
    /// `m_gold`.
    pub gold: u64,
    /// The u32 at instance offset 0x3c8, which has no reflection name.
    pub field_3c8: u32,
    /// `m_itemDescriptor` (reader NewWorld+0x5ce8b20).
    pub item_descriptor: SimpleItemDescriptor,
    /// `m_contractTitle`.
    pub contract_title: String,
    /// `m_contractDescription`.
    pub contract_description: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated_messages::{
        ContractsComponentClientFacetOnCreateContract as OnCreate,
        ContractsComponentClientFacetOnLookupContracts as OnLookup,
        ContractsComponentClientFacetOnLookupContractsByParticipant as OnLookupByParticipant,
        ContractsComponentClientFacetOnSearchContracts as OnSearch,
    };
    use crate::serialize::{CARRIER_ENDIAN, Marshal, ReadBuffer, Unmarshal, WriteBuffer};

    fn string(out: &mut Vec<u8>, value: &str) {
        out.push(u8::try_from(value.len()).unwrap());
        out.extend_from_slice(value.as_bytes());
    }

    fn u32_be(out: &mut Vec<u8>, value: u32) {
        out.extend_from_slice(&value.to_be_bytes());
    }

    fn u64_be(out: &mut Vec<u8>, value: u64) {
        out.extend_from_slice(&value.to_be_bytes());
    }

    /// An item descriptor in the reader's order, with made-up values: item
    /// id, gear score, quantity, a paperdoll flag, two slot numbers, two
    /// flags, two perk lists, a flag, a third list.
    fn item_descriptor(out: &mut Vec<u8>) {
        u32_be(out, 0x1111_2222);
        out.extend_from_slice(&600u16.to_be_bytes());
        u32_be(out, 3);
        out.push(0);
        u32_be(out, 0xffff_ffff);
        u32_be(out, 0);
        out.push(1);
        out.push(0);
        out.push(2);
        u32_be(out, 0xaaaa_0001);
        u32_be(out, 0xaaaa_0002);
        out.push(0);
        out.push(0);
        out.push(1);
        u32_be(out, 7);
    }

    fn participant(out: &mut Vec<u8>, id: u8, kind: u32) {
        out.extend_from_slice(&[id; 16]);
        u32_be(out, kind);
    }

    fn participant_event(out: &mut Vec<u8>, id: &str) {
        string(out, id);
        u32_be(out, 1);
        u32_be(out, 2);
        u32_be(out, 0);
        u32_be(out, 4);
        item_descriptor(out);
    }

    /// A contract base in the reader's order, with made-up values.
    fn contract(out: &mut Vec<u8>) {
        out.extend_from_slice(&[0x10; 16]);
        u32_be(out, 5);
        u32_be(out, 1);
        u64_be(out, 1_000);
        u64_be(out, 0);
        u64_be(out, 2_000);
        u64_be(out, 86_400_000_000_000);
        participant(out, 0x20, 1);
        out.push(1);
        // One participant with two events.
        out.push(1);
        participant(out, 0x21, 2);
        out.push(2);
        participant_event(out, "event-a");
        participant_event(out, "");
        // Item data: four strings, gear score, tier, perks, attributes,
        // rarity, required level, perk counts, instance id.
        string(out, "Category");
        string(out, "Family");
        string(out, "Group");
        string(out, "Name");
        u32_be(out, 590);
        u32_be(out, 5);
        out.push(2);
        u32_be(out, 0xbbbb_0001);
        u32_be(out, 0xbbbb_0002);
        out.push(1);
        u32_be(out, 9);
        u32_be(out, 3);
        u32_be(out, 60);
        u32_be(out, 2);
        u32_be(out, 0);
        out.extend_from_slice(&[0x30; 16]);
        // Six entity ids.
        for id in 1..=6u64 {
            u64_be(out, id);
        }
        participant(out, 0, 0);
        participant_event(out, "");
        u32_be(out, 0);
    }

    fn sell_contract() -> Vec<u8> {
        let mut out = Vec::new();
        contract(&mut out);
        out.extend_from_slice(&[0x40; 16]);
        u64_be(&mut out, 12_345);
        u32_be(&mut out, 10);
        item_descriptor(&mut out);
        out
    }

    fn reads_whole_and_roundtrips<T: Unmarshal + Marshal>(body: &[u8]) -> T {
        let mut rb = ReadBuffer::new(CARRIER_ENDIAN, body);
        let value = T::unmarshal(&mut rb).expect("decodes");
        assert!(
            rb.remaining().is_empty(),
            "{} bytes unread",
            rb.remaining().len()
        );
        let mut wb = WriteBuffer::new(CARRIER_ENDIAN);
        value.marshal(&mut wb);
        assert_eq!(wb.into_vec(), body);
        value
    }

    #[test]
    fn sell_contract_reads_every_field() {
        let v: SellContract = reads_whole_and_roundtrips(&sell_contract());
        let c = &v.contract;
        assert_eq!(c.id, [0x10; 16]);
        assert_eq!((c.version, c.status), (5, 1));
        assert_eq!(
            (c.creation_time, c.completion_time, c.last_update_time),
            (1_000, 0, 2_000)
        );
        assert_eq!(c.posting_duration, 86_400_000_000_000);
        assert_eq!(c.visibility_criteria.id, [0x20; 16]);
        assert_eq!(c.visibility_criteria.participant_type, 1);
        assert!(c.is_local_player_creator);
        let [(who, events)] = c.participant_events.as_slice() else {
            panic!("one participant")
        };
        assert_eq!(who.participant_type, 2);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].id, "event-a");
        assert_eq!(
            (events[0].status, events[0].role, events[0].completion_count),
            (1, 2, 4)
        );
        assert_eq!(events[0].item_descriptor.field_10, 0x1111_2222);
        assert_eq!(events[0].item_descriptor.field_38.as_slice(), &[7]);
        let item = &c.item_data;
        assert_eq!(item.item_category, "Category");
        assert_eq!(item.item_name, "Name");
        assert_eq!((item.gear_score, item.tier), (590, 5));
        assert_eq!(item.perks.as_slice(), &[0xbbbb_0001, 0xbbbb_0002]);
        assert_eq!(item.perk_attributes, vec![9]);
        assert_eq!(
            (item.rarity_level, item.required_level, item.perk_count),
            (3, 60, 2)
        );
        assert_eq!(item.item_instance_id, [0x30; 16]);
        assert_eq!(c.root_player_entity_id, 1);
        assert_eq!(c.contracts_component_entity_id, 6);
        assert_eq!(v.outpost_id, [0x40; 16]);
        assert_eq!((v.gold, v.count), (12_345, 10));
        assert_eq!(v.sale_item_descriptor.field_0e, 600);
        assert_eq!(v.sale_item_descriptor.field_14, 3);
    }

    #[test]
    fn buy_and_make_good_contracts_read_their_tails() {
        let mut buy = Vec::new();
        contract(&mut buy);
        buy.extend_from_slice(&[0x41; 16]);
        u64_be(&mut buy, 99);
        u32_be(&mut buy, 4);
        let v: BuyContract = reads_whole_and_roundtrips(&buy);
        assert_eq!((v.outpost_id, v.gold, v.count), ([0x41; 16], 99, 4));

        let mut make_good = Vec::new();
        contract(&mut make_good);
        u64_be(&mut make_good, 50);
        u32_be(&mut make_good, 8);
        item_descriptor(&mut make_good);
        string(&mut make_good, "Title");
        string(&mut make_good, "Description");
        let v: MakeGoodContract = reads_whole_and_roundtrips(&make_good);
        assert_eq!((v.gold, v.field_3c8), (50, 8));
        assert_eq!(v.contract_title, "Title");
        assert_eq!(v.contract_description, "Description");
    }

    /// A search answer as the server writes it: request id pair, a 16-byte
    /// id, the results (entries, a string, a u32 and a u64) and a flag.
    fn search_answer(entries: &[Option<(u32, &[u8])>]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
        out.extend_from_slice(&[0x50; 16]);
        out.push(u8::try_from(entries.len()).unwrap());
        for entry in entries {
            match entry {
                None => out.push(0),
                Some((type_index, body)) => {
                    out.push(1);
                    // A VLQ type index of two bytes: 0x80 | low six bits,
                    // then the rest.
                    out.push(0x80 | u8::try_from(type_index & 0x3f).unwrap());
                    out.push(u8::try_from(type_index >> 6).unwrap());
                    out.extend_from_slice(body);
                }
            }
        }
        string(&mut out, "cursor");
        u32_be(&mut out, 2);
        u64_be(&mut out, 77);
        out.push(1);
        out
    }

    #[test]
    fn search_contracts_answer_reads_its_contracts() {
        let sell = sell_contract();
        let mut buy = Vec::new();
        contract(&mut buy);
        buy.extend_from_slice(&[0x41; 16]);
        u64_be(&mut buy, 99);
        u32_be(&mut buy, 4);
        let body = search_answer(&[Some((2991, &sell)), None, Some((2957, &buy))]);
        let v: OnSearch = reads_whole_and_roundtrips(&body);
        let (entries, cursor, count, total) = &v.field_2;
        assert_eq!((cursor.as_str(), *count, *total), ("cursor", 2, 77));
        assert!(v.field_3);
        assert_eq!(entries.len(), 3);
        let first = entries[0].as_ref().expect("a contract");
        assert_eq!(first.type_index(), 2991);
        assert_eq!(first.class_name(), Some("SellContract"));
        assert_eq!(first.body(), sell.as_slice());
        let read: SellContract = reads_whole_and_roundtrips(first.body());
        assert_eq!(read.gold, 12_345);
        assert!(entries[1].is_none());
        let third = entries[2].as_ref().expect("a contract");
        assert_eq!(third.type_index(), 2957);
        assert_eq!(third.body(), buy.as_slice());
    }

    #[test]
    fn lookup_and_create_answers_read_their_contracts() {
        let sell = sell_contract();
        let class_value = |out: &mut Vec<u8>| {
            out.push(0x80 | u8::try_from(2991u32 & 0x3f).unwrap());
            out.push(u8::try_from(2991u32 >> 6).unwrap());
            out.extend_from_slice(&sell);
        };

        // Request id pair, a 16-byte id, then one struct: the counted
        // optional contracts, a string and a u32.
        let mut lookup = Vec::new();
        lookup.extend_from_slice(&[7; 16]);
        lookup.extend_from_slice(&[8; 16]);
        lookup.push(2);
        lookup.push(1);
        class_value(&mut lookup);
        lookup.push(0);
        string(&mut lookup, "x");
        u32_be(&mut lookup, 4);
        let v: OnLookupByParticipant = reads_whole_and_roundtrips(&lookup);
        let entries = &v.field_2.0;
        assert_eq!(entries.len(), 2);
        let first = entries[0].as_ref().expect("a contract");
        assert_eq!(first.body(), sell.as_slice());
        assert!(entries[1].is_none());
        assert_eq!((v.field_2.1.as_str(), v.field_2.2), ("x", 4));

        // The same layout for the lookup by contract id (223), with no
        // contracts.
        let mut empty = Vec::new();
        empty.extend_from_slice(&[7; 16]);
        empty.extend_from_slice(&[8; 16]);
        empty.push(0);
        string(&mut empty, "");
        u32_be(&mut empty, 0);
        let v: OnLookup = reads_whole_and_roundtrips(&empty);
        assert!(v.field_2.0.is_empty());

        // Request id pair, a 16-byte id, a flag and one contract.
        let mut create = Vec::new();
        create.extend_from_slice(&[7; 16]);
        create.extend_from_slice(&[9; 16]);
        create.push(1);
        class_value(&mut create);
        let v: OnCreate = reads_whole_and_roundtrips(&create);
        assert!(v.field_1.field_1.0);
        assert_eq!(v.field_1.field_1.1.type_index(), 2991);
        assert_eq!(v.field_1.field_1.1.body(), sell.as_slice());
    }
}
