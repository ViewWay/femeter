//! Comprehensive tests for Set/Action/Block Transfer APDUs

use dlms_apdu::{
    ActionRequest, ActionRequestListItem, ActionRequestNormal, ActionRequestWithList,
    ActionResponse, ActionResponseListItem, ActionResponseNormal, ActionResponseWithList,
    BlockTransferCommand, GeneralBlockTransfer, SetRequest, SetRequestItem, SetRequestNormal,
    SetRequestWithList, SetResponse, SetResponseBlock, SetResponseError, SetResponseNormal,
};
use dlms_apdu::{AttributeDescriptor, InvokeId, MethodDescriptor};
use dlms_core::{DataAccessError, DlmsType, ObisCode};

fn test_obis() -> ObisCode {
    ObisCode::new(1, 0, 1, 8, 0, 255)
}

fn test_obis2() -> ObisCode {
    ObisCode::new(1, 0, 2, 8, 0, 255)
}

// ============================================================
// SetRequest Tests
// ============================================================

#[test]
fn test_set_request_normal_various_types() {
    let desc = AttributeDescriptor::new(3, test_obis(), 2);

    // u8
    let req = SetRequestNormal::new(InvokeId::new(1), desc.clone(), DlmsType::from_u8(255));
    let bytes = req.encode().unwrap();
    let dec = SetRequestNormal::decode(&bytes).unwrap();
    assert_eq!(dec.item.value, DlmsType::from_u8(255));

    // u16
    let req = SetRequestNormal::new(InvokeId::new(2), desc.clone(), DlmsType::from_u16(65535));
    let bytes = req.encode().unwrap();
    let dec = SetRequestNormal::decode(&bytes).unwrap();
    assert_eq!(dec.item.value, DlmsType::from_u16(65535));

    // u32
    let req = SetRequestNormal::new(
        InvokeId::new(3),
        desc.clone(),
        DlmsType::from_u32(0xFFFFFFFF),
    );
    let bytes = req.encode().unwrap();
    let dec = SetRequestNormal::decode(&bytes).unwrap();
    assert_eq!(dec.item.value, DlmsType::from_u32(0xFFFFFFFF));

    // OctetString
    let req = SetRequestNormal::new(
        InvokeId::new(4),
        desc.clone(),
        DlmsType::OctetString(vec![0xDE, 0xAD, 0xBE, 0xEF]),
    );
    let bytes = req.encode().unwrap();
    let dec = SetRequestNormal::decode(&bytes).unwrap();
    assert_eq!(
        dec.item.value,
        DlmsType::OctetString(vec![0xDE, 0xAD, 0xBE, 0xEF])
    );

    // Null
    let req = SetRequestNormal::new(InvokeId::new(5), desc.clone(), DlmsType::Null);
    let bytes = req.encode().unwrap();
    let dec = SetRequestNormal::decode(&bytes).unwrap();
    assert_eq!(dec.item.value, DlmsType::Null);

    // Boolean
    let req = SetRequestNormal::new(InvokeId::new(6), desc.clone(), DlmsType::Boolean(true));
    let bytes = req.encode().unwrap();
    let dec = SetRequestNormal::decode(&bytes).unwrap();
    assert_eq!(dec.item.value, DlmsType::Boolean(true));
}

#[test]
fn test_set_request_normal_empty_octet_string() {
    let desc = AttributeDescriptor::new(3, test_obis(), 2);
    let req = SetRequestNormal::new(InvokeId::new(1), desc, DlmsType::OctetString(vec![]));
    let bytes = req.encode().unwrap();
    let dec = SetRequestNormal::decode(&bytes).unwrap();
    assert_eq!(dec.item.value, DlmsType::OctetString(vec![]));
}

#[test]
fn test_set_request_with_list_single_item() {
    let items = vec![SetRequestItem {
        descriptor: AttributeDescriptor::new(3, test_obis(), 2),
        value: DlmsType::from_u8(42),
    }];
    let req = SetRequestWithList::new(InvokeId::new(1), items);
    let bytes = req.encode().unwrap();
    let dec = SetRequestWithList::decode(&bytes).unwrap();
    assert_eq!(dec.items.len(), 1);
    assert_eq!(dec.items[0].value, DlmsType::from_u8(42));
}

#[test]
fn test_set_request_with_list_multiple_items() {
    let items = vec![
        SetRequestItem {
            descriptor: AttributeDescriptor::new(3, test_obis(), 2),
            value: DlmsType::from_u8(10),
        },
        SetRequestItem {
            descriptor: AttributeDescriptor::new(3, test_obis2(), 2),
            value: DlmsType::from_u16(20000),
        },
        SetRequestItem {
            descriptor: AttributeDescriptor::new(8, test_obis(), 3),
            value: DlmsType::OctetString(vec![1, 2, 3]),
        },
    ];
    let req = SetRequestWithList::new(InvokeId::new(5), items.clone());
    let bytes = req.encode().unwrap();
    let dec = SetRequestWithList::decode(&bytes).unwrap();
    assert_eq!(dec.items.len(), 3);
    assert_eq!(dec.items[0].value, DlmsType::from_u8(10));
    assert_eq!(dec.items[1].value, DlmsType::from_u16(20000));
    assert_eq!(dec.items[2].value, DlmsType::OctetString(vec![1, 2, 3]));
}

#[test]
fn test_set_request_enum_dispatch_normal() {
    let desc = AttributeDescriptor::new(3, test_obis(), 2);
    let req = SetRequestNormal::new(InvokeId::new(1), desc, DlmsType::from_u32(100));
    let bytes = req.encode().unwrap();
    match SetRequest::decode(&bytes).unwrap() {
        SetRequest::Normal(r) => assert_eq!(r.item.value, DlmsType::from_u32(100)),
        _ => panic!("Expected Normal"),
    }
}

#[test]
fn test_set_request_enum_dispatch_with_list() {
    let items = vec![SetRequestItem {
        descriptor: AttributeDescriptor::new(3, test_obis(), 2),
        value: DlmsType::from_u8(1),
    }];
    let req = SetRequestWithList::new(InvokeId::new(2), items);
    let bytes = req.encode().unwrap();
    match SetRequest::decode(&bytes).unwrap() {
        SetRequest::WithList(r) => assert_eq!(r.items.len(), 1),
        _ => panic!("Expected WithList"),
    }
}

#[test]
fn test_set_request_decode_empty() {
    assert!(SetRequest::decode(&[]).is_err());
    assert!(SetRequest::decode(&[0xC1]).is_err());
}

#[test]
fn test_set_request_invoke_id_max() {
    let desc = AttributeDescriptor::new(3, test_obis(), 2);
    let req = SetRequestNormal::new(InvokeId::new(255), desc, DlmsType::from_u8(0));
    let bytes = req.encode().unwrap();
    let dec = SetRequestNormal::decode(&bytes).unwrap();
    assert_eq!(dec.invoke_id, InvokeId::new(255));
}

// ============================================================
// SetResponse Tests
// ============================================================

#[test]
fn test_set_response_success_with_data() {
    let resp = SetResponseNormal::success(InvokeId::new(1), DlmsType::from_u32(12345));
    let bytes = resp.encode().unwrap();
    let dec = SetResponseNormal::decode(&bytes).unwrap();
    match dec.result {
        dlms_apdu::AccessResult::Success(v) => assert_eq!(v, DlmsType::from_u32(12345)),
        _ => panic!("Expected success"),
    }
}

#[test]
fn test_set_response_error_various() {
    for err in [
        DataAccessError::ReadWriteDenied,
        DataAccessError::TypeUnmatched,
        DataAccessError::HardwareFault,
        DataAccessError::TemporaryFailure,
        DataAccessError::UnavailableObject,
        DataAccessError::ObjectUndefined,
        DataAccessError::UnsupportedClass,
    ] {
        let resp = SetResponseNormal::error(InvokeId::new(1), err);
        let bytes = resp.encode().unwrap();
        let dec = SetResponseNormal::decode(&bytes).unwrap();
        match dec.result {
            dlms_apdu::AccessResult::Error(e) => assert_eq!(e, err),
            _ => panic!("Expected error"),
        }
    }
}

#[test]
fn test_set_response_block_roundtrip() {
    let data = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
    let resp = SetResponseBlock::new(InvokeId::new(10), 0, false, data.clone());
    let bytes = resp.encode();
    let dec = SetResponseBlock::decode(&bytes).unwrap();
    assert_eq!(dec.invoke_id, InvokeId::new(10));
    assert_eq!(dec.block_number, 0);
    assert!(!dec.last_block);
    assert_eq!(dec.data, data);
}

#[test]
fn test_set_response_block_last_block() {
    let resp = SetResponseBlock::new(InvokeId::new(1), 99, true, vec![0xFF]);
    let bytes = resp.encode();
    let dec = SetResponseBlock::decode(&bytes).unwrap();
    assert!(dec.last_block);
    assert_eq!(dec.block_number, 99);
}

#[test]
fn test_set_response_block_empty_data() {
    let resp = SetResponseBlock::new(InvokeId::new(1), 0, true, vec![]);
    let bytes = resp.encode();
    let dec = SetResponseBlock::decode(&bytes).unwrap();
    assert!(dec.data.is_empty());
}

#[test]
fn test_set_response_error_variant() {
    let resp = SetResponseError::new(InvokeId::new(7), DataAccessError::TypeUnmatched);
    let bytes = resp.encode();
    let dec = SetResponseError::decode(&bytes).unwrap();
    assert_eq!(dec.error, DataAccessError::TypeUnmatched);
}

#[test]
fn test_set_response_enum_dispatch_data() {
    let resp = SetResponseNormal::success(InvokeId::new(1), DlmsType::Null);
    let bytes = resp.encode().unwrap();
    match SetResponse::decode(&bytes).unwrap() {
        SetResponse::Data(_) => {}
        _ => panic!("Expected Data"),
    }
}

#[test]
fn test_set_response_enum_dispatch_block() {
    let resp = SetResponseBlock::new(InvokeId::new(1), 0, true, vec![1, 2, 3]);
    let bytes = resp.encode();
    match SetResponse::decode(&bytes).unwrap() {
        SetResponse::Block(b) => assert_eq!(b.data, vec![1, 2, 3]),
        _ => panic!("Expected Block"),
    }
}

#[test]
fn test_set_response_enum_dispatch_error() {
    let resp = SetResponseError::new(InvokeId::new(1), DataAccessError::ReadWriteDenied);
    let bytes = resp.encode();
    match SetResponse::decode(&bytes).unwrap() {
        SetResponse::DataAccessError(e) => assert_eq!(e.error, DataAccessError::ReadWriteDenied),
        _ => panic!("Expected DataAccessError"),
    }
}

#[test]
fn test_set_response_decode_empty() {
    assert!(SetResponse::decode(&[]).is_err());
}

// ============================================================
// ActionRequest Tests
// ============================================================

#[test]
fn test_action_request_without_parameters() {
    let method = MethodDescriptor::new(70, test_obis(), 1);
    let req = ActionRequestNormal::new(InvokeId::new(1), method);
    let bytes = req.encode().unwrap();
    let dec = ActionRequestNormal::decode(&bytes).unwrap();
    // None parameters encode as empty octet-string, decode as Some(OctetString([]))
    assert_eq!(dec.parameters, Some(DlmsType::OctetString(vec![])));
}

#[test]
fn test_action_request_with_various_parameters() {
    let method = MethodDescriptor::new(70, test_obis(), 1);

    // u32
    let req = ActionRequestNormal::with_parameters(
        InvokeId::new(1),
        method.clone(),
        DlmsType::from_u32(42),
    );
    let bytes = req.encode().unwrap();
    let dec = ActionRequestNormal::decode(&bytes).unwrap();
    assert_eq!(dec.parameters, Some(DlmsType::from_u32(42)));

    // OctetString
    let req = ActionRequestNormal::with_parameters(
        InvokeId::new(2),
        method.clone(),
        DlmsType::OctetString(vec![0x01, 0x02]),
    );
    let bytes = req.encode().unwrap();
    let dec = ActionRequestNormal::decode(&bytes).unwrap();
    assert_eq!(
        dec.parameters,
        Some(DlmsType::OctetString(vec![0x01, 0x02]))
    );

    // Structure
    let req = ActionRequestNormal::with_parameters(
        InvokeId::new(3),
        method,
        DlmsType::Structure(vec![DlmsType::from_u8(1), DlmsType::from_u8(2)]),
    );
    let bytes = req.encode().unwrap();
    let dec = ActionRequestNormal::decode(&bytes).unwrap();
    assert_eq!(
        dec.parameters,
        Some(DlmsType::Structure(vec![
            DlmsType::from_u8(1),
            DlmsType::from_u8(2)
        ]))
    );
}

#[test]
fn test_action_request_with_list_roundtrip() {
    let items = vec![
        ActionRequestListItem {
            method: MethodDescriptor::new(70, test_obis(), 1),
            parameters: Some(DlmsType::from_u8(10)),
        },
        ActionRequestListItem {
            method: MethodDescriptor::new(32, test_obis2(), 2),
            parameters: None,
        },
    ];
    let req = ActionRequestWithList::new(InvokeId::new(5), items);
    let bytes = req.encode().unwrap();
    let dec = ActionRequestWithList::decode(&bytes).unwrap();
    assert_eq!(dec.items.len(), 2);
    assert_eq!(dec.items[0].parameters, Some(DlmsType::from_u8(10)));
    // None parameters encode as empty octet-string
    assert_eq!(dec.items[1].parameters, Some(DlmsType::OctetString(vec![])));
}

#[test]
fn test_action_request_enum_dispatch() {
    let method = MethodDescriptor::new(70, test_obis(), 1);

    let req = ActionRequestNormal::new(InvokeId::new(1), method.clone());
    let bytes = req.encode().unwrap();
    match ActionRequest::decode(&bytes).unwrap() {
        ActionRequest::Normal(_) => {}
        _ => panic!("Expected Normal"),
    }

    let items = vec![ActionRequestListItem {
        method,
        parameters: None,
    }];
    let req = ActionRequestWithList::new(InvokeId::new(1), items);
    let bytes = req.encode().unwrap();
    match ActionRequest::decode(&bytes).unwrap() {
        ActionRequest::WithList(r) => assert_eq!(r.items.len(), 1),
        _ => panic!("Expected WithList"),
    }
}

#[test]
fn test_action_request_decode_empty() {
    assert!(ActionRequest::decode(&[]).is_err());
}

// ============================================================
// ActionResponse Tests
// ============================================================

#[test]
fn test_action_response_success_various_data() {
    // Null
    let resp = ActionResponseNormal::success(InvokeId::new(1), DlmsType::Null);
    let bytes = resp.encode().unwrap();
    let dec = ActionResponseNormal::decode(&bytes).unwrap();
    match dec.result {
        Ok(v) => assert_eq!(v, DlmsType::Null),
        _ => panic!("Expected success"),
    }

    // u32
    let resp = ActionResponseNormal::success(InvokeId::new(2), DlmsType::from_u32(99999));
    let bytes = resp.encode().unwrap();
    let dec = ActionResponseNormal::decode(&bytes).unwrap();
    match dec.result {
        Ok(v) => assert_eq!(v, DlmsType::from_u32(99999)),
        _ => panic!("Expected success"),
    }

    // Boolean
    for b in [true, false] {
        let resp = ActionResponseNormal::success(InvokeId::new(3), DlmsType::Boolean(b));
        let bytes = resp.encode().unwrap();
        let dec = ActionResponseNormal::decode(&bytes).unwrap();
        match dec.result {
            Ok(v) => assert_eq!(v, DlmsType::Boolean(b)),
            _ => panic!("Expected success"),
        }
    }
}

#[test]
fn test_action_response_error_various() {
    for err in [
        DataAccessError::ReadWriteDenied,
        DataAccessError::TypeUnmatched,
        DataAccessError::HardwareFault,
        DataAccessError::TemporaryFailure,
        DataAccessError::UnavailableObject,
        DataAccessError::ObjectUndefined,
        DataAccessError::UnsupportedClass,
    ] {
        let resp = ActionResponseNormal::error(InvokeId::new(1), err);
        let bytes = resp.encode().unwrap();
        let dec = ActionResponseNormal::decode(&bytes).unwrap();
        match dec.result {
            Err(e) => assert_eq!(e, err),
            _ => panic!("Expected error"),
        }
    }
}

#[test]
fn test_action_response_with_list_roundtrip() {
    let items = vec![
        ActionResponseListItem {
            result: Ok(DlmsType::from_u8(1)),
        },
        ActionResponseListItem {
            result: Err(DataAccessError::ReadWriteDenied),
        },
    ];
    let resp = ActionResponseWithList::new(InvokeId::new(3), items);
    let bytes = resp.encode().unwrap();
    let dec = ActionResponseWithList::decode(&bytes).unwrap();
    assert_eq!(dec.items.len(), 2);
    assert_eq!(dec.items[0].result, Ok(DlmsType::from_u8(1)));
    assert_eq!(dec.items[1].result, Err(DataAccessError::ReadWriteDenied));
}

#[test]
fn test_action_response_enum_dispatch() {
    let resp = ActionResponseNormal::success(InvokeId::new(1), DlmsType::Null);
    let bytes = resp.encode().unwrap();
    match ActionResponse::decode(&bytes).unwrap() {
        ActionResponse::Normal(_) => {}
        _ => panic!("Expected Normal"),
    }

    let items = vec![ActionResponseListItem {
        result: Ok(DlmsType::Null),
    }];
    let resp = ActionResponseWithList::new(InvokeId::new(1), items);
    let bytes = resp.encode().unwrap();
    match ActionResponse::decode(&bytes).unwrap() {
        ActionResponse::WithList(r) => assert_eq!(r.items.len(), 1),
        _ => panic!("Expected WithList"),
    }
}

#[test]
fn test_action_response_decode_empty() {
    assert!(ActionResponse::decode(&[]).is_err());
}

// ============================================================
// Block Transfer Tests
// ============================================================

#[test]
fn test_block_transfer_last_block_roundtrip() {
    let gbt = GeneralBlockTransfer::new(
        InvokeId::new(1),
        42,
        true,
        BlockTransferCommand::LastBlockAcknowledged,
    );
    let bytes = gbt.encode();
    let dec = GeneralBlockTransfer::decode(&bytes).unwrap();
    assert_eq!(dec.block_number, 42);
    assert!(dec.last_block);
    assert_eq!(dec.command, BlockTransferCommand::LastBlockAcknowledged);
}

#[test]
fn test_block_transfer_get_request_block() {
    let data = vec![0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE];
    let gbt = GeneralBlockTransfer::new(
        InvokeId::new(5),
        0,
        false,
        BlockTransferCommand::GetRequestBlock { data: data.clone() },
    );
    let bytes = gbt.encode();
    let dec = GeneralBlockTransfer::decode(&bytes).unwrap();
    assert!(!dec.last_block);
    match dec.command {
        BlockTransferCommand::GetRequestBlock { data: d } => assert_eq!(d, data),
        _ => panic!("Expected GetRequestBlock"),
    }
}

#[test]
fn test_block_transfer_get_response_block() {
    let data = (1..=10u8).collect::<Vec<_>>();
    let gbt = GeneralBlockTransfer::new(
        InvokeId::new(10),
        5,
        false,
        BlockTransferCommand::GetResponseBlock { data: data.clone() },
    );
    let bytes = gbt.encode();
    let dec = GeneralBlockTransfer::decode(&bytes).unwrap();
    match dec.command {
        BlockTransferCommand::GetResponseBlock { data: d } => assert_eq!(d, data),
        _ => panic!("Expected GetResponseBlock"),
    }
}

#[test]
fn test_block_transfer_set_request_block() {
    let data = vec![0xFF, 0x00, 0xFF, 0x00];
    let gbt = GeneralBlockTransfer::new(
        InvokeId::new(1),
        0,
        false,
        BlockTransferCommand::SetRequestBlock { data: data.clone() },
    );
    let bytes = gbt.encode();
    let dec = GeneralBlockTransfer::decode(&bytes).unwrap();
    match dec.command {
        BlockTransferCommand::SetRequestBlock { data: d } => assert_eq!(d, data),
        _ => panic!("Expected SetRequestBlock"),
    }
}

#[test]
fn test_block_transfer_set_response_block() {
    let data = vec![0x00];
    let gbt = GeneralBlockTransfer::new(
        InvokeId::new(1),
        1,
        true,
        BlockTransferCommand::SetResponseBlock { data: data.clone() },
    );
    let bytes = gbt.encode();
    let dec = GeneralBlockTransfer::decode(&bytes).unwrap();
    assert!(dec.last_block);
    match dec.command {
        BlockTransferCommand::SetResponseBlock { data: d } => assert_eq!(d, data),
        _ => panic!("Expected SetResponseBlock"),
    }
}

#[test]
fn test_block_transfer_action_request_block() {
    let data = vec![0xAA, 0xBB];
    let gbt = GeneralBlockTransfer::new(
        InvokeId::new(3),
        2,
        false,
        BlockTransferCommand::ActionRequestBlock { data: data.clone() },
    );
    let bytes = gbt.encode();
    let dec = GeneralBlockTransfer::decode(&bytes).unwrap();
    match dec.command {
        BlockTransferCommand::ActionRequestBlock { data: d } => assert_eq!(d, data),
        _ => panic!("Expected ActionRequestBlock"),
    }
}

#[test]
fn test_block_transfer_action_response_block() {
    let data = vec![0x01, 0x02, 0x03];
    let gbt = GeneralBlockTransfer::new(
        InvokeId::new(4),
        3,
        true,
        BlockTransferCommand::ActionResponseBlock { data: data.clone() },
    );
    let bytes = gbt.encode();
    let dec = GeneralBlockTransfer::decode(&bytes).unwrap();
    assert!(dec.last_block);
    match dec.command {
        BlockTransferCommand::ActionResponseBlock { data: d } => assert_eq!(d, data),
        _ => panic!("Expected ActionResponseBlock"),
    }
}

#[test]
fn test_block_transfer_empty_data_block() {
    let gbt = GeneralBlockTransfer::new(
        InvokeId::new(1),
        0,
        true,
        BlockTransferCommand::GetRequestBlock { data: vec![] },
    );
    let bytes = gbt.encode();
    let dec = GeneralBlockTransfer::decode(&bytes).unwrap();
    match dec.command {
        BlockTransferCommand::GetRequestBlock { data: d } => assert!(d.is_empty()),
        _ => panic!("Expected GetRequestBlock"),
    }
}

#[test]
fn test_block_transfer_large_data_block() {
    let data: Vec<u8> = (0..=255).collect();
    let gbt = GeneralBlockTransfer::new(
        InvokeId::new(1),
        0,
        false,
        BlockTransferCommand::GetResponseBlock { data: data.clone() },
    );
    let bytes = gbt.encode();
    let dec = GeneralBlockTransfer::decode(&bytes).unwrap();
    match dec.command {
        BlockTransferCommand::GetResponseBlock { data: d } => assert_eq!(d, data),
        _ => panic!("Expected GetResponseBlock"),
    }
}

#[test]
fn test_block_transfer_block_number_boundaries() {
    for bn in [0u32, 1, 255, 65535, 0x7FFFFFFF, 0xFFFFFFFF] {
        let gbt = GeneralBlockTransfer::new(
            InvokeId::new(1),
            bn,
            bn % 2 == 0,
            BlockTransferCommand::LastBlockAcknowledged,
        );
        let bytes = gbt.encode();
        let dec = GeneralBlockTransfer::decode(&bytes).unwrap();
        assert_eq!(dec.block_number, bn);
        assert_eq!(dec.last_block, bn % 2 == 0);
    }
}

#[test]
fn test_block_transfer_command_to_byte_all() {
    assert_eq!(BlockTransferCommand::LastBlockAcknowledged.to_byte(), 0x01);
    assert_eq!(
        BlockTransferCommand::GetRequestBlock { data: vec![] }.to_byte(),
        0x02
    );
    assert_eq!(
        BlockTransferCommand::GetResponseBlock { data: vec![] }.to_byte(),
        0x03
    );
    assert_eq!(
        BlockTransferCommand::SetRequestBlock { data: vec![] }.to_byte(),
        0x04
    );
    assert_eq!(
        BlockTransferCommand::SetResponseBlock { data: vec![] }.to_byte(),
        0x05
    );
    assert_eq!(
        BlockTransferCommand::ActionRequestBlock { data: vec![] }.to_byte(),
        0x06
    );
    assert_eq!(
        BlockTransferCommand::ActionResponseBlock { data: vec![] }.to_byte(),
        0x07
    );
}

#[test]
fn test_block_transfer_decode_empty() {
    assert!(GeneralBlockTransfer::decode(&[]).is_err());
    assert!(GeneralBlockTransfer::decode(&[0xC8]).is_err());
}

#[test]
fn test_block_transfer_decode_invalid_command() {
    let bytes = vec![0xC8, 0x00, 0x01, 0x80 | 0xFF, 0x00, 0x00, 0x00, 0x00];
    assert!(GeneralBlockTransfer::decode(&bytes).is_err());
}

#[test]
fn test_block_transfer_decode_truncated_data() {
    let bytes = vec![
        0xC8, 0x00, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0xAA, 0xBB,
    ];
    assert!(GeneralBlockTransfer::decode(&bytes).is_err());
}

// ============================================================
// Cross-cutting: invoke_id consistency
// ============================================================

#[test]
fn test_set_invoke_id_consistency() {
    let id = InvokeId::new(123);
    let desc = AttributeDescriptor::new(3, test_obis(), 2);
    let set_req = SetRequestNormal::new(id, desc, DlmsType::from_u8(1));
    assert_eq!(set_req.invoke_id, id);
    assert_eq!(SetRequest::Normal(set_req).invoke_id(), id);

    let set_resp = SetResponseNormal::success(id, DlmsType::Null);
    assert_eq!(set_resp.invoke_id, id);
    assert_eq!(SetResponse::Data(set_resp).invoke_id(), id);
}
