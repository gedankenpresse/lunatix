#![no_std]

pub mod pt;

pub unsafe trait PhysMapper {
    unsafe fn phys_to_mapped_mut<T>(&self, phys: *mut T) -> *mut T;
    unsafe fn phys_to_mapped<T>(&self, phys: *const T) -> *const T;
    unsafe fn mapped_to_phys_mut<T>(&self, mapped: *mut T) -> *mut T;
    unsafe fn mapped_to_phys<T>(&self, mapped: *const T) -> *const T;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
