extern crate std;

use crate::capabilities::{CSpaceSource, CapPlacement, CapSpec};
use crate::manifest::LunatixManifest;
use alloc::vec::Vec;

const MANIFEST: &'static str = "
[metadata]
name=test_name
description=test_description

[environment]
cspace_radix=4
stack_size_bytes=4096

[capabilities]
1=cspace,source=self
2=irq,line=5
4=memory,min_size_bytes=4096
";

#[test]
fn test_metadata_name() {
    let m = LunatixManifest::from(MANIFEST);
    assert_eq!(m.metadata().unwrap().name().unwrap(), "test_name")
}

#[test]
fn test_metadata_description() {
    let m = LunatixManifest::from(MANIFEST);
    assert_eq!(
        m.metadata().unwrap().description().unwrap(),
        "test_description"
    )
}

#[test]
fn test_environment_cspace_radix() {
    let m = LunatixManifest::from(MANIFEST);
    assert_eq!(m.environment().unwrap().cspace_radix().unwrap(), 4,)
}

#[test]
fn test_environment_stack_size() {
    let m = LunatixManifest::from(MANIFEST);
    assert_eq!(m.environment().unwrap().stack_size_bytes().unwrap(), 4096,)
}

#[test]
fn test_capabilities() {
    let m = LunatixManifest::from(MANIFEST);
    let caps = m.capabilities().unwrap();

    let target = std::vec![
        CapPlacement {
            caddr: 1,
            spec: CapSpec::CSpace {
                source: CSpaceSource::OwnCSpace,
            }
        },
        CapPlacement {
            caddr: 2,
            spec: CapSpec::Irq { line: 5 }
        },
        CapPlacement {
            caddr: 4,
            spec: CapSpec::Memory {
                min_size_bytes: 4096
            }
        }
    ];

    assert_eq!(target, caps.collect::<Vec<_>>());
}
