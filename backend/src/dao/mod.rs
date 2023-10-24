//! Module with database interfaces

mod sqlitex;

use futures::Future;
pub use sqlitex::Sqlite;

use crate::util::LateInit;

pub type StorageBackend = Sqlite;

pub static STORAGE: LateInit<StorageBackend> = LateInit::new();

/// Helper trait to allow calling `futures::executor::block_on` postfix
pub trait FutureBlock {
    type Output;
    fn blocking_run(self) -> Self::Output; 
}

impl<T> FutureBlock for T where T: Future {
    type Output = <Self as Future>::Output;

    fn blocking_run(self) -> Self::Output {
        futures::executor::block_on(self)
    }
}

/// Wrapper for decoding blob into fixed size array
#[derive(sqlx::Type, Debug)]
#[sqlx(transparent)]
pub struct SliceShim<'a>(&'a [u8]);

impl<'a, const N: usize> TryFrom<SliceShim<'a>> for [i8; N] {
    type Error = std::array::TryFromSliceError;

    fn try_from(value: SliceShim<'a>) -> Result<Self, Self::Error> {
        Self::try_from(bytemuck::cast_slice(value.0))
    }
}

impl<'a, const N: usize> TryFrom<SliceShim<'a>> for [u8; N] {
    type Error = std::array::TryFromSliceError;

    fn try_from(value: SliceShim<'a>) -> Result<Self, Self::Error> {
        Self::try_from(bytemuck::cast_slice(value.0))
    }
} 