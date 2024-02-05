use core::cmp::{max, Ordering};
use core::mem;
use device_tree::fdt::{FdtError, FlattenedDeviceTree, MemoryReservationEntry};
use thiserror_no_std::Error;

#[derive(Debug, Error)]
pub enum DeviceInfoError {
    #[error("There is no node in the device tree describing the requested information")]
    NoNodeInDeviceTree,
    #[error("The {0} node did not have the expected property {1}")]
    NoPropOnNode(&'static str, &'static str),
    #[error("The device tree could not be parsed")]
    DeviceTreeError(#[from] FdtError),
}

#[derive(Debug)]
pub struct DeviceInfo {
    pub usable_memory: (*mut u8, usize),
    pub fdt: FlattenedDeviceTree<'static>,
}

impl DeviceInfo {
    pub unsafe fn from_raw_ptr(ptr: *const u8) -> Result<Self, DeviceInfoError> {
        let fdt = FlattenedDeviceTree::from_ptr(ptr)?;

        Ok(Self {
            usable_memory: get_usable_memory(&fdt)?,
            fdt,
        })
    }
}
//
/// Search for reserved memory in the device tree and return a new reservation that concatenates all areas found
/// in the device tree.
///
/// **Note**: This only makes sense to do when the reserved memory lies completely at the beginning or end of the
/// whole devices memory.
/// Otherwise the area in the middle will be included through the concatenation and we are left with no usable
/// memory at all.
///
/// # Device Tree Details
/// The reserved memory regions are extracted from the device trees memory reservation block..
///
/// For details about the node, see [u-boot Reserved Memory Regions](https://github.com/qemu/u-boot/blob/master/doc/device-tree-bindings/reserved-memory/reserved-memory.txt).
fn get_reserved_memory(device_tree: &FlattenedDeviceTree<'_>) -> Option<MemoryReservationEntry> {
    // TODO Concatenating all reserved areas does not work if there are reservations at the top and bottom of physical memory. This should be improved

    device_tree
        .memory_reservations
        .clone()
        .reduce(|res_a, res_b| match res_a.address.cmp(&res_b.address) {
            Ordering::Less => MemoryReservationEntry::new(
                res_a.address,
                (res_b.address - res_a.address) + res_b.size,
            ),
            Ordering::Equal => {
                MemoryReservationEntry::new(res_a.address, max(res_a.size, res_b.size))
            }
            Ordering::Greater => MemoryReservationEntry::new(
                res_b.address,
                (res_a.address - res_b.address) + res_a.size,
            ),
        })
}

/// Search for memory description in the device tree and return the starting address and size of it.
///
/// **Note**: The memory returned here includes **all** device memory, including reserved regions.
/// It cannot be directly used and the reserved sections must be taken into account.
///
/// # Device Tree Details
/// The reserved memory regions are extracted from the device trees */memory* node.
///
/// For details about the node, see the [DeviceTree specs /memory node](https://devicetree-specification.readthedocs.io/en/v0.3/devicenodes.html#memory-node).
fn get_all_memory(
    device_tree: &FlattenedDeviceTree<'_>,
) -> Result<(*mut u8, usize), DeviceInfoError> {
    log::trace!("searching for memory node in device tree");

    let mem_node = device_tree
        .structure
        .children()
        .find(|node| node.name.starts_with("memory@"))
        .ok_or(DeviceInfoError::NoNodeInDeviceTree)?;
    log::trace!("found memory node {} in device tree", mem_node.name);

    let reg_prop = mem_node
        .props()
        .find(|prop| prop.name == "reg")
        .ok_or(DeviceInfoError::NoPropOnNode("memory", "reg"))?;

    let mem_start = u64::from_be_bytes(
        (&reg_prop.value[0..mem::size_of::<u64>()])
            .try_into()
            .unwrap(),
    );
    let mem_len = u64::from_be_bytes(
        (&reg_prop.value[mem::size_of::<u64>()..mem::size_of::<u64>() * 2])
            .try_into()
            .unwrap(),
    );

    log::trace!(
        "found reg property describing usable memory start = {:#x} len = {:#x} (end = {:#x})",
        mem_start,
        mem_len,
        mem_start + mem_len
    );

    Ok((mem_start as *mut u8, mem_len as usize))
}

/// Return the start address of general purpose memory and how much space is available
pub fn get_usable_memory(
    device_tree: &FlattenedDeviceTree<'_>,
) -> Result<(*mut u8, usize), DeviceInfoError> {
    let (all_mem_start, all_mem_len) = get_all_memory(device_tree)?;

    match get_reserved_memory(device_tree) {
        None => Ok((all_mem_start, all_mem_len)),
        Some(reservation) => {
            // TODO We currently assume that reserved memory starts at the bottom of physical memory. This is, of course, not always the case and should be properly handled
            assert_eq!(all_mem_start as u64, reservation.address);
            Ok((
                (reservation.address + reservation.size) as *mut u8,
                all_mem_len - reservation.size as usize,
            ))
        }
    }
}
