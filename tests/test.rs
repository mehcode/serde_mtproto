#[macro_use]
extern crate lazy_static;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_mtproto as serde_mtproto_other_name;    // Tests `serde_mtproto_derive`
#[macro_use]
extern crate serde_mtproto_derive;


use serde_mtproto_other_name::{to_bytes_identifiable, to_writer_identifiable, from_bytes_identifiable, from_reader_identifiable};


#[derive(Debug, PartialEq, Serialize, Deserialize, MtProtoIdentifiable)]
#[id = "0xdeadbeef"]
struct Foo {
    has_receiver: bool,
    size: usize,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, MtProtoIdentifiable)]
enum Cafebabe {
    #[id = "0x0badf00d"]
    Bar {
        byte_id: i8,
        position: (u64, u64),
    },
    #[id = "0xbaaaaaad"]
    Baz {
        id: u64,
        name: String,
    },
}


lazy_static! {
    static ref FOO: Foo = Foo {
        has_receiver: true,
        size: 57,
    };

    static ref FOO_SERIALIZED: Vec<u8> = vec![
        0xef, 0xbe, 0xad, 0xde,     // id of Foo in little-endian
        181, 117, 114, 153,         // id of true in little-endian
        57, 0, 0, 0, 0, 0, 0, 0,    // 57 as little-endian 64-bit int
    ];

    static ref CAFEBABE_BAR: Cafebabe = Cafebabe::Bar {
        byte_id: -20,
        position: (350, 142857),
    };

    static ref CAFEBABE_BAR_SERIALIZED: Vec<u8> = vec![
        0x0d, 0xf0, 0xad, 0x0b,     // id of Cafebabe::Bar in little-endian
        236, 255, 255, 255,         // -20 as 32-bit int (MTProto doesn't support less than 32-bit)
        94, 1, 0, 0, 0, 0, 0, 0,    // 350 as little-endian 64-bit int
        9, 46, 2, 0, 0, 0, 0, 0,    // 142857 as little-endian 64-bit int
    ];

    static ref CAFEBABE_BAZ: Cafebabe = Cafebabe::Baz {
        id: u64::max_value(),
        name: "beef".to_owned(),
    };

    static ref CAFEBABE_BAZ_SERIALIZED: Vec<u8> = vec![
        0xad, 0xaa, 0xaa, 0xba,                    // id of Cafebabe::Baz in little-endian
        255, 255, 255, 255, 255, 255, 255, 255,    // u64::max_value() == 2 ** 64 - 1
        4, 98, 101, 101, 102, 0, 0, 0,             // string "beef" of length 4 and 3 bytes of padding
    ];
}


#[test]
fn test_struct_to_bytes_identifiable() {
    let vec = to_bytes_identifiable(&*FOO).unwrap();

    assert_eq!(vec, *FOO_SERIALIZED);
}

#[test]
fn test_struct_to_writer_identifiable() {
    let mut vec = Vec::new();
    to_writer_identifiable(&mut vec, &*FOO).unwrap();

    assert_eq!(vec, *FOO_SERIALIZED);
}

#[test]
fn test_struct_from_bytes_identifiable() {
    let foo_deserialized: Foo = from_bytes_identifiable(&*FOO_SERIALIZED, None).unwrap();

    assert_eq!(foo_deserialized, *FOO);
}

#[test]
fn test_struct_from_reader_identifiable() {
    let foo_deserialized: Foo = from_reader_identifiable(FOO_SERIALIZED.as_slice(), None).unwrap();

    assert_eq!(foo_deserialized, *FOO);
}


#[test]
fn test_enum_variant_to_bytes_identifiable() {
    let vec = to_bytes_identifiable(&*CAFEBABE_BAR).unwrap();

    assert_eq!(vec, *CAFEBABE_BAR_SERIALIZED);
}

#[test]
fn test_enum_variant_to_writer_identifiable() {
    let mut vec = Vec::new();
    to_writer_identifiable(&mut vec, &*CAFEBABE_BAR).unwrap();

    assert_eq!(vec, *CAFEBABE_BAR_SERIALIZED);
}

#[test]
fn test_enum_variant_from_bytes_identifiable() {
    let cafebabe_bar_deserialized: Cafebabe = from_bytes_identifiable(&*CAFEBABE_BAR_SERIALIZED, Some("Bar")).unwrap();

    assert_eq!(cafebabe_bar_deserialized, *CAFEBABE_BAR);
}

#[test]
fn test_enum_variant_from_reader_identifiable() {
    let cafebabe_bar_deserialized: Cafebabe = from_reader_identifiable(CAFEBABE_BAR_SERIALIZED.as_slice(), Some("Bar")).unwrap();

    assert_eq!(cafebabe_bar_deserialized, *CAFEBABE_BAR);
}


#[test]
fn test_enum_variant_to_bytes_identifiable2() {
    let vec = to_bytes_identifiable(&*CAFEBABE_BAZ).unwrap();

    assert_eq!(vec, *CAFEBABE_BAZ_SERIALIZED);
}

#[test]
fn test_enum_variant_to_writer_identifiable2() {
    let mut vec = Vec::new();
    to_writer_identifiable(&mut vec, &*CAFEBABE_BAZ).unwrap();

    assert_eq!(vec, *CAFEBABE_BAZ_SERIALIZED);
}

#[test]
fn test_enum_variant_from_bytes_identifiable2() {
    let cafebabe_baz_deserialized: Cafebabe = from_bytes_identifiable(&*CAFEBABE_BAZ_SERIALIZED, Some("Baz")).unwrap();

    assert_eq!(cafebabe_baz_deserialized, *CAFEBABE_BAZ);
}

#[test]
fn test_enum_variant_from_reader_identifiable2() {
    let cafebabe_baz_deserialized: Cafebabe = from_reader_identifiable(CAFEBABE_BAZ_SERIALIZED.as_slice(), Some("Baz")).unwrap();

    assert_eq!(cafebabe_baz_deserialized, *CAFEBABE_BAZ);
}


/// MTProto-serialized data must be aligned by 4 bytes.
#[test]
fn test_serialization_alignment() {
    assert!(FOO_SERIALIZED.len() % 4 == 0);
    assert!(CAFEBABE_BAR_SERIALIZED.len() % 4 == 0);
    assert!(CAFEBABE_BAZ_SERIALIZED.len() % 4 == 0);
}