//! DRM object properties and blobs (software model).

use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PropType {
    Range,
    Enum,
    Blob,
    Bitmask,
    Object,
}

#[derive(Clone, Debug)]
pub struct Property {
    pub id: u32,
    pub name: String,
    pub prop_type: PropType,
    /// For range props: [min, max]. For enum: value list.
    pub values: Vec<u64>,
}

#[derive(Clone, Debug)]
pub struct PropBlob {
    pub id: u32,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct ObjectPropValue {
    pub prop_id: u32,
    pub value: u64,
}

#[derive(Clone, Debug, Default)]
pub struct PropertyStore {
    next_prop: u32,
    next_blob: u32,
    props: Vec<Property>,
    blobs: Vec<PropBlob>,
    /// (object_id, prop_id) -> value
    values: Vec<(u32, u32, u64)>,
}

impl PropertyStore {
    pub const fn new() -> Self {
        Self {
            next_prop: 1,
            next_blob: 1,
            props: Vec::new(),
            blobs: Vec::new(),
            values: Vec::new(),
        }
    }

    pub fn create_prop(
        &mut self,
        name: &str,
        prop_type: PropType,
        values: Vec<u64>,
    ) -> u32 {
        let id = self.next_prop;
        self.next_prop = self.next_prop.wrapping_add(1).max(1);
        self.props.push(Property {
            id,
            name: String::from(name),
            prop_type,
            values,
        });
        id
    }

    pub fn create_blob(&mut self, data: Vec<u8>) -> u32 {
        let id = self.next_blob;
        self.next_blob = self.next_blob.wrapping_add(1).max(1);
        self.blobs.push(PropBlob { id, data });
        id
    }

    pub fn blob(&self, id: u32) -> Option<&PropBlob> {
        self.blobs.iter().find(|b| b.id == id)
    }

    pub fn set(&mut self, object_id: u32, prop_id: u32, value: u64) {
        if let Some(slot) = self
            .values
            .iter_mut()
            .find(|(o, p, _)| *o == object_id && *p == prop_id)
        {
            slot.2 = value;
        } else {
            self.values.push((object_id, prop_id, value));
        }
    }

    pub fn get(&self, object_id: u32, prop_id: u32) -> Option<u64> {
        self.values
            .iter()
            .find(|(o, p, _)| *o == object_id && *p == prop_id)
            .map(|(_, _, v)| *v)
    }

    pub fn prop_count(&self) -> usize {
        self.props.len()
    }

    pub fn blob_count(&self) -> usize {
        self.blobs.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blob_and_object_value() {
        let mut s = PropertyStore::new();
        let edid_prop = s.create_prop("EDID", PropType::Blob, Vec::new());
        let blob = s.create_blob(alloc::vec![1, 2, 3, 4]);
        s.set(1, edid_prop, blob as u64);
        assert_eq!(s.get(1, edid_prop), Some(blob as u64));
        assert_eq!(s.blob(blob).unwrap().data, &[1, 2, 3, 4]);
    }
}
