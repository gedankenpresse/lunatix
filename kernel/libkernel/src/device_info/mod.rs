use core::cmp::{max, Ordering};
use fdt_rs::base::DevTree;
use fdt_rs::error::DevTreeError;
use fdt_rs::index::DevTreeIndex;
use fdt_rs::prelude::PropReader;
use thiserror_no_std::Error;

// TODO Maybe use the DevTreeIndex instead of the raw device tree to be more performant
// On the other hand. Interacting with the device description probably doesn't happen very often so this is probably
// fine for now

#[derive(Debug, Error)]
pub enum DeviceInfoError {
    #[error("There is no node in the device tree describing the requested information")]
    NoNodeInDeviceTree,
    #[error("The {0} node did not have the expected property {1}")]
    NoPropOnNode(&'static str, &'static str),
    #[error("The device tree could not be parsed")]
    DevTreeError(#[from] DevTreeError),
}

/// Information about the device on which the kernel is currently executing
#[derive(Debug)]
pub struct DeviceInfo<'index, 'fdt> {
    device_tree: DevTreeIndex<'index, 'fdt>,
}

impl<'index, 'fdt> DeviceInfo<'index, 'fdt> {
    pub unsafe fn from_device_tree(
        device_tree: *const u8,
        index_buf: &'index mut [u8],
    ) -> Result<Self, DevTreeError> {
        let dev_tree = DevTree::from_raw_pointer(device_tree)?;
        let dev_tree_idx = DevTreeIndex::new(dev_tree, index_buf)?;

        Ok(Self {
            device_tree: dev_tree_idx,
        })
    }

    /// Search for reserved memory in the device tree and return a new reservation that concatenates all areas found
    /// in the device tree.
    ///
    /// **Note**: This only makes sense to do when the reserved memory lies completely at the beginning or end of the
    /// whole devices memory.
    /// Otherwise the area in the middle will be included through the concatenation and we are left with no usable
    /// memory at all.
    ///
    /// # Device Tree Details
    /// The reserved memory regions are extracted from the device trees */reserved-memory* node.
    ///
    /// For details about the node, see [u-boot Reserved Memory Regions](https://github.com/qemu/u-boot/blob/master/doc/device-tree-bindings/reserved-memory/reserved-memory.txt).
    fn get_reserved_memory(&self) -> Option<(*mut u8, usize)> {
        // TODO Concatenating all reserved areas does not work if there are reservations at the top and bottom of physical memory. This should be improved

        // look for the "reserved-memory" node
        self.device_tree
            .nodes()
            .find(|node| node.name().unwrap() == "reserved-memory")
            .and_then(|node| {
                log::trace!("found reserved-memory node");

                // the node describes reserved areas via child nodes so let's find them now and extract the reserved area from the "reg" property
                node.children()
                    .map(|child_node| {
                        let reg_prop = child_node
                            .props()
                            .find(|prop| prop.name().unwrap() == "reg")
                            .unwrap();
                        let mem_start = reg_prop.u64(0).unwrap();
                        let mem_len = reg_prop.u64(1).unwrap();

                        log::trace!(
                            "found reserved memory area {}: start = {:#x} len = {:#x} (end = {:#x})",
                            child_node.name().unwrap(),
                            mem_start,
                            mem_len,
                            mem_start + mem_len
                        );

                        (mem_start as *mut u8, mem_len as usize)
                    })

                    // concatenate all reserved areas together
                    .reduce(
                        |(mem_start_a, mem_len_a), (mem_start_b, mem_len_b)| match mem_start_a
                            .cmp(&mem_start_b)
                        {
                            Ordering::Less => (
                                mem_start_a,
                                (mem_start_b as usize - mem_start_a as usize) + mem_len_b,
                            ),
                            Ordering::Equal => (mem_start_a, max(mem_len_a, mem_len_b)),
                            Ordering::Greater => (
                                mem_start_b,
                                (mem_start_a as usize - mem_start_b as usize) + mem_len_a,
                            ),
                        },
                    )
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
    fn get_all_memory(&self) -> Result<(*mut u8, usize), DeviceInfoError> {
        log::trace!("searching for memory node in device tree");

        let node = self
            .device_tree
            .nodes()
            .find(|node| node.name().unwrap().starts_with("memory@"))
            .ok_or(DeviceInfoError::NoNodeInDeviceTree)?;
        log::trace!("found memory node {} in device tree", node.name().unwrap());

        let reg_prop = node
            .props()
            .find(|prop| prop.name().unwrap() == "reg")
            .ok_or(DeviceInfoError::NoPropOnNode("memory", "reg"))?;
        let mem_start = reg_prop.u64(0)?;
        let mem_len = reg_prop.u64(1)?;
        log::trace!(
            "found reg property describing usable memory start = {:#x} len = {:#x} (end = {:#x})",
            mem_start,
            mem_len,
            mem_start + mem_len
        );

        Ok((mem_start as *mut u8, mem_len as usize))
    }

    /// Return the start address of general purpose memory and how much space is available
    pub fn get_usable_memory(&self) -> Result<(*mut u8, usize), DeviceInfoError> {
        let (all_mem_start, all_mem_len) = self.get_all_memory()?;

        match self.get_reserved_memory() {
            None => Ok((all_mem_start, all_mem_len)),
            Some((reserved_start, reserved_len)) => {
                // TODO We currently assume that reserved memory starts at the bottom of physical memory. This is, of course, not always the case and should be properly handled
                assert_eq!(all_mem_start, reserved_start);
                Ok((
                    unsafe { reserved_start.add(reserved_len) },
                    all_mem_len - reserved_len,
                ))
            }
        }
    }
}
