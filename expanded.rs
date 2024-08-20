#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
use std::sync::Arc;
use game::tetris::GameState;
use parking_lot::RwLock;
use search::Graph;
mod mem {
    use std::{borrow::Borrow, mem::ManuallyDrop};
    use bumpalo_herd::Herd;
    use parking_lot::Mutex;
    /// A object pool to reuse Herds through generations
    pub struct HerdPool(Mutex<Vec<Herd>>);
    pub struct RentedHerd<'hp> {
        herd: ManuallyDrop<Herd>,
        owner: &'hp HerdPool,
    }
    impl HerdPool {
        pub fn new() -> Self {
            Self(Mutex::new(Vec::new()))
        }
        /// Rent a herd from the pool. If the pool is empty, a new herd will be created.
        pub fn rent(&self) -> RentedHerd {
            let mut arena = self.0.lock();
            let herd = arena.pop().unwrap_or_default();
            RentedHerd {
                herd: ManuallyDrop::new(herd),
                owner: &self,
            }
        }
    }
    impl Borrow<Herd> for RentedHerd<'_> {
        fn borrow(&self) -> &Herd {
            &self.herd
        }
    }
    impl Drop for RentedHerd<'_> {
        fn drop(&mut self) {
            self.herd.reset();
            let member = unsafe { ManuallyDrop::take(&mut self.herd) };
            self.owner.0.lock().push(member);
        }
    }
}
mod search {
    use bumpalo_herd::Herd;
    use dashmap::DashMap;
    use game::tetris::{BitBoard, GameState, Move, SevenBag};
    use once_cell::sync::Lazy;
    use ouroboros::self_referencing;
    pub struct Graph {
        root_gen: Box<Generation>,
        root_state: GameState<BitBoard>,
    }
    ///Encapsulates implementation details for a self-referencing struct. This module is only visible when using --document-private-items.
    mod ouroboros_impl_generation {
        use super::*;
        ///The self-referencing struct.
        #[repr(transparent)]
        pub struct Generation {
            actual_data: ::core::mem::MaybeUninit<GenerationInternal>,
        }
        struct GenerationInternal {
            #[doc(hidden)]
            next: Lazy<Box<Generation>>,
            #[doc(hidden)]
            lookup: DashMap<State, &'static Node<'static>>,
            #[doc(hidden)]
            herd: ::ouroboros::macro_help::AliasableBox<Herd>,
        }
        impl ::core::ops::Drop for Generation {
            fn drop(&mut self) {
                unsafe { self.actual_data.assume_init_drop() };
            }
        }
        fn check_if_okay_according_to_checkers(
            herd: Herd,
            lookup_builder: impl for<'this> ::core::ops::FnOnce(
                &'this Herd,
            ) -> DashMap<State, &'this Node<'this>>,
            next: Lazy<Box<Generation>>,
        ) {
            let herd = herd;
            let lookup = lookup_builder(&herd);
            let lookup = lookup;
            let next = next;
            BorrowedFields::<'_, '_> {
                herd: &herd,
                lookup: &lookup,
                next: &next,
            };
        }
        /**A more verbose but stable way to construct self-referencing structs. It is comparable to using `StructName { field1: value1, field2: value2 }` rather than `StructName::new(value1, value2)`. This has the dual benefit of making your code both easier to refactor and more readable. Call [`build()`](Self::build) to construct the actual struct. The fields of this struct should be used as follows:

| Field | Suggested Use |
| --- | --- |
| `herd` | Directly pass in the value this field should contain |
| `lookup_builder` | Use a function or closure: `(herd: &_) -> lookup: _` |
| `next` | Directly pass in the value this field should contain |
*/
        pub(super) struct GenerationBuilder<
            LookupBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this Herd,
                ) -> DashMap<State, &'this Node<'this>>,
        > {
            pub(super) herd: Herd,
            pub(super) lookup_builder: LookupBuilder_,
            pub(super) next: Lazy<Box<Generation>>,
        }
        impl<
            LookupBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this Herd,
                ) -> DashMap<State, &'this Node<'this>>,
        > GenerationBuilder<LookupBuilder_> {
            ///Calls [`Generation::new()`](Generation::new) using the provided values. This is preferable over calling `new()` directly for the reasons listed above.
            pub(super) fn build(self) -> Generation {
                Generation::new(self.herd, self.lookup_builder, self.next)
            }
        }
        /**A more verbose but stable way to construct self-referencing structs. It is comparable to using `StructName { field1: value1, field2: value2 }` rather than `StructName::new(value1, value2)`. This has the dual benefit of making your code both easier to refactor and more readable. Call [`build()`](Self::build) to construct the actual struct. The fields of this struct should be used as follows:

| Field | Suggested Use |
| --- | --- |
| `herd` | Directly pass in the value this field should contain |
| `lookup_builder` | Use a function or closure: `(herd: &_) -> lookup: _` |
| `next` | Directly pass in the value this field should contain |
*/
        pub(super) struct GenerationAsyncBuilder<
            LookupBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this Herd,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = DashMap<State, &'this Node<'this>>,
                            > + 'this,
                        >,
                    >,
        > {
            pub(super) herd: Herd,
            pub(super) lookup_builder: LookupBuilder_,
            pub(super) next: Lazy<Box<Generation>>,
        }
        impl<
            LookupBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this Herd,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = DashMap<State, &'this Node<'this>>,
                            > + 'this,
                        >,
                    >,
        > GenerationAsyncBuilder<LookupBuilder_> {
            ///Calls [`Generation::new()`](Generation::new) using the provided values. This is preferable over calling `new()` directly for the reasons listed above.
            pub(super) async fn build(self) -> Generation {
                Generation::new_async(self.herd, self.lookup_builder, self.next).await
            }
        }
        /**A more verbose but stable way to construct self-referencing structs. It is comparable to using `StructName { field1: value1, field2: value2 }` rather than `StructName::new(value1, value2)`. This has the dual benefit of making your code both easier to refactor and more readable. Call [`build()`](Self::build) to construct the actual struct. The fields of this struct should be used as follows:

| Field | Suggested Use |
| --- | --- |
| `herd` | Directly pass in the value this field should contain |
| `lookup_builder` | Use a function or closure: `(herd: &_) -> lookup: _` |
| `next` | Directly pass in the value this field should contain |
*/
        pub(super) struct GenerationAsyncSendBuilder<
            LookupBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this Herd,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = DashMap<State, &'this Node<'this>>,
                            > + ::core::marker::Send + 'this,
                        >,
                    >,
        > {
            pub(super) herd: Herd,
            pub(super) lookup_builder: LookupBuilder_,
            pub(super) next: Lazy<Box<Generation>>,
        }
        impl<
            LookupBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this Herd,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = DashMap<State, &'this Node<'this>>,
                            > + ::core::marker::Send + 'this,
                        >,
                    >,
        > GenerationAsyncSendBuilder<LookupBuilder_> {
            ///Calls [`Generation::new()`](Generation::new) using the provided values. This is preferable over calling `new()` directly for the reasons listed above.
            pub(super) async fn build(self) -> Generation {
                Generation::new_async_send(self.herd, self.lookup_builder, self.next)
                    .await
            }
        }
        /**A more verbose but stable way to construct self-referencing structs. It is comparable to using `StructName { field1: value1, field2: value2 }` rather than `StructName::new(value1, value2)`. This has the dual benefit of making your code both easier to refactor and more readable. Call [`try_build()`](Self::try_build) or [`try_build_or_recover()`](Self::try_build_or_recover) to construct the actual struct. The fields of this struct should be used as follows:

| Field | Suggested Use |
| --- | --- |
| `herd` | Directly pass in the value this field should contain |
| `lookup_builder` | Use a function or closure: `(herd: &_) -> Result<lookup: _, Error_>` |
| `next` | Directly pass in the value this field should contain |
*/
        pub(super) struct GenerationTryBuilder<
            LookupBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this Herd,
                ) -> ::core::result::Result<DashMap<State, &'this Node<'this>>, Error_>,
            Error_,
        > {
            pub(super) herd: Herd,
            pub(super) lookup_builder: LookupBuilder_,
            pub(super) next: Lazy<Box<Generation>>,
        }
        impl<
            LookupBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this Herd,
                ) -> ::core::result::Result<DashMap<State, &'this Node<'this>>, Error_>,
            Error_,
        > GenerationTryBuilder<LookupBuilder_, Error_> {
            ///Calls [`Generation::try_new()`](Generation::try_new) using the provided values. This is preferable over calling `try_new()` directly for the reasons listed above.
            pub(super) fn try_build(self) -> ::core::result::Result<Generation, Error_> {
                Generation::try_new(self.herd, self.lookup_builder, self.next)
            }
            ///Calls [`Generation::try_new_or_recover()`](Generation::try_new_or_recover) using the provided values. This is preferable over calling `try_new_or_recover()` directly for the reasons listed above.
            pub(super) fn try_build_or_recover(
                self,
            ) -> ::core::result::Result<Generation, (Error_, Heads)> {
                Generation::try_new_or_recover(self.herd, self.lookup_builder, self.next)
            }
        }
        /**A more verbose but stable way to construct self-referencing structs. It is comparable to using `StructName { field1: value1, field2: value2 }` rather than `StructName::new(value1, value2)`. This has the dual benefit of making your code both easier to refactor and more readable. Call [`try_build()`](Self::try_build) or [`try_build_or_recover()`](Self::try_build_or_recover) to construct the actual struct. The fields of this struct should be used as follows:

| Field | Suggested Use |
| --- | --- |
| `herd` | Directly pass in the value this field should contain |
| `lookup_builder` | Use a function or closure: `(herd: &_) -> Result<lookup: _, Error_>` |
| `next` | Directly pass in the value this field should contain |
*/
        pub(super) struct GenerationAsyncTryBuilder<
            LookupBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this Herd,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = ::core::result::Result<
                                    DashMap<State, &'this Node<'this>>,
                                    Error_,
                                >,
                            > + 'this,
                        >,
                    >,
            Error_,
        > {
            pub(super) herd: Herd,
            pub(super) lookup_builder: LookupBuilder_,
            pub(super) next: Lazy<Box<Generation>>,
        }
        impl<
            LookupBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this Herd,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = ::core::result::Result<
                                    DashMap<State, &'this Node<'this>>,
                                    Error_,
                                >,
                            > + 'this,
                        >,
                    >,
            Error_,
        > GenerationAsyncTryBuilder<LookupBuilder_, Error_> {
            ///Calls [`Generation::try_new()`](Generation::try_new) using the provided values. This is preferable over calling `try_new()` directly for the reasons listed above.
            pub(super) async fn try_build(
                self,
            ) -> ::core::result::Result<Generation, Error_> {
                Generation::try_new_async(self.herd, self.lookup_builder, self.next)
                    .await
            }
            ///Calls [`Generation::try_new_or_recover()`](Generation::try_new_or_recover) using the provided values. This is preferable over calling `try_new_or_recover()` directly for the reasons listed above.
            pub(super) async fn try_build_or_recover(
                self,
            ) -> ::core::result::Result<Generation, (Error_, Heads)> {
                Generation::try_new_or_recover_async(
                        self.herd,
                        self.lookup_builder,
                        self.next,
                    )
                    .await
            }
        }
        /**A more verbose but stable way to construct self-referencing structs. It is comparable to using `StructName { field1: value1, field2: value2 }` rather than `StructName::new(value1, value2)`. This has the dual benefit of making your code both easier to refactor and more readable. Call [`try_build()`](Self::try_build) or [`try_build_or_recover()`](Self::try_build_or_recover) to construct the actual struct. The fields of this struct should be used as follows:

| Field | Suggested Use |
| --- | --- |
| `herd` | Directly pass in the value this field should contain |
| `lookup_builder` | Use a function or closure: `(herd: &_) -> Result<lookup: _, Error_>` |
| `next` | Directly pass in the value this field should contain |
*/
        pub(super) struct GenerationAsyncSendTryBuilder<
            LookupBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this Herd,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = ::core::result::Result<
                                    DashMap<State, &'this Node<'this>>,
                                    Error_,
                                >,
                            > + ::core::marker::Send + 'this,
                        >,
                    >,
            Error_,
        > {
            pub(super) herd: Herd,
            pub(super) lookup_builder: LookupBuilder_,
            pub(super) next: Lazy<Box<Generation>>,
        }
        impl<
            LookupBuilder_: for<'this> ::core::ops::FnOnce(
                    &'this Herd,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = ::core::result::Result<
                                    DashMap<State, &'this Node<'this>>,
                                    Error_,
                                >,
                            > + ::core::marker::Send + 'this,
                        >,
                    >,
            Error_,
        > GenerationAsyncSendTryBuilder<LookupBuilder_, Error_> {
            ///Calls [`Generation::try_new()`](Generation::try_new) using the provided values. This is preferable over calling `try_new()` directly for the reasons listed above.
            pub(super) async fn try_build(
                self,
            ) -> ::core::result::Result<Generation, Error_> {
                Generation::try_new_async_send(self.herd, self.lookup_builder, self.next)
                    .await
            }
            ///Calls [`Generation::try_new_or_recover()`](Generation::try_new_or_recover) using the provided values. This is preferable over calling `try_new_or_recover()` directly for the reasons listed above.
            pub(super) async fn try_build_or_recover(
                self,
            ) -> ::core::result::Result<Generation, (Error_, Heads)> {
                Generation::try_new_or_recover_async_send(
                        self.herd,
                        self.lookup_builder,
                        self.next,
                    )
                    .await
            }
        }
        ///A struct for holding immutable references to all [tail and immutably borrowed fields](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions) in an instance of [`Generation`](Generation).
        pub(super) struct BorrowedFields<'outer_borrow, 'this>
        where
            'static: 'this,
            'this: 'outer_borrow,
        {
            pub(super) next: &'outer_borrow Lazy<Box<Generation>>,
            pub(super) lookup: &'outer_borrow DashMap<State, &'this Node<'this>>,
            pub(super) herd: &'this Herd,
        }
        ///A struct for holding mutable references to all [tail fields](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions) in an instance of [`Generation`](Generation).
        pub(super) struct BorrowedMutFields<'outer_borrow, 'this1, 'this0>
        where
            'static: 'this0,
            'static: 'this1,
            'this1: 'this0,
        {
            pub(super) next: &'outer_borrow mut Lazy<Box<Generation>>,
            pub(super) lookup: &'outer_borrow mut DashMap<State, &'this0 Node<'this0>>,
            pub(super) herd: &'this1 Herd,
        }
        ///A struct which contains only the [head fields](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions) of [`Generation`](Generation).
        pub(super) struct Heads {
            pub(super) next: Lazy<Box<Generation>>,
            pub(super) herd: Herd,
        }
        impl Generation {
            /**Constructs a new instance of this self-referential struct. (See also [`GenerationBuilder::build()`](GenerationBuilder::build)). Each argument is a field of the new struct. Fields that refer to other fields inside the struct are initialized using functions instead of directly passing their value. The arguments are as follows:

| Argument | Suggested Use |
| --- | --- |
| `herd` | Directly pass in the value this field should contain |
| `lookup_builder` | Use a function or closure: `(herd: &_) -> lookup: _` |
| `next` | Directly pass in the value this field should contain |
*/
            pub(super) fn new(
                herd: Herd,
                lookup_builder: impl for<'this> ::core::ops::FnOnce(
                    &'this Herd,
                ) -> DashMap<State, &'this Node<'this>>,
                next: Lazy<Box<Generation>>,
            ) -> Generation {
                let herd = ::ouroboros::macro_help::aliasable_boxed(herd);
                let herd_illegal_static_reference = unsafe {
                    ::ouroboros::macro_help::change_lifetime(&*herd)
                };
                let lookup = lookup_builder(herd_illegal_static_reference);
                unsafe {
                    Self {
                        actual_data: ::core::mem::MaybeUninit::new(GenerationInternal {
                            herd,
                            lookup,
                            next,
                        }),
                    }
                }
            }
            /**Constructs a new instance of this self-referential struct. (See also [`GenerationAsyncBuilder::build()`](GenerationAsyncBuilder::build)). Each argument is a field of the new struct. Fields that refer to other fields inside the struct are initialized using functions instead of directly passing their value. The arguments are as follows:

| Argument | Suggested Use |
| --- | --- |
| `herd` | Directly pass in the value this field should contain |
| `lookup_builder` | Use a function or closure: `(herd: &_) -> lookup: _` |
| `next` | Directly pass in the value this field should contain |
*/
            pub(super) async fn new_async(
                herd: Herd,
                lookup_builder: impl for<'this> ::core::ops::FnOnce(
                    &'this Herd,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = DashMap<State, &'this Node<'this>>,
                            > + 'this,
                        >,
                    >,
                next: Lazy<Box<Generation>>,
            ) -> Generation {
                let herd = ::ouroboros::macro_help::aliasable_boxed(herd);
                let herd_illegal_static_reference = unsafe {
                    ::ouroboros::macro_help::change_lifetime(&*herd)
                };
                let lookup = lookup_builder(herd_illegal_static_reference).await;
                unsafe {
                    Self {
                        actual_data: ::core::mem::MaybeUninit::new(GenerationInternal {
                            herd,
                            lookup,
                            next,
                        }),
                    }
                }
            }
            /**Constructs a new instance of this self-referential struct. (See also [`GenerationAsyncSendBuilder::build()`](GenerationAsyncSendBuilder::build)). Each argument is a field of the new struct. Fields that refer to other fields inside the struct are initialized using functions instead of directly passing their value. The arguments are as follows:

| Argument | Suggested Use |
| --- | --- |
| `herd` | Directly pass in the value this field should contain |
| `lookup_builder` | Use a function or closure: `(herd: &_) -> lookup: _` |
| `next` | Directly pass in the value this field should contain |
*/
            pub(super) async fn new_async_send(
                herd: Herd,
                lookup_builder: impl for<'this> ::core::ops::FnOnce(
                    &'this Herd,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = DashMap<State, &'this Node<'this>>,
                            > + ::core::marker::Send + 'this,
                        >,
                    >,
                next: Lazy<Box<Generation>>,
            ) -> Generation {
                let herd = ::ouroboros::macro_help::aliasable_boxed(herd);
                let herd_illegal_static_reference = unsafe {
                    ::ouroboros::macro_help::change_lifetime(&*herd)
                };
                let lookup = lookup_builder(herd_illegal_static_reference).await;
                unsafe {
                    Self {
                        actual_data: ::core::mem::MaybeUninit::new(GenerationInternal {
                            herd,
                            lookup,
                            next,
                        }),
                    }
                }
            }
            /**(See also [`GenerationTryBuilder::try_build()`](GenerationTryBuilder::try_build).) Like [`new`](Self::new), but builders for [self-referencing fields](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions) can return results. If any of them fail, `Err` is returned. If all of them succeed, `Ok` is returned. The arguments are as follows:

| Argument | Suggested Use |
| --- | --- |
| `herd` | Directly pass in the value this field should contain |
| `lookup_builder` | Use a function or closure: `(herd: &_) -> Result<lookup: _, Error_>` |
| `next` | Directly pass in the value this field should contain |
*/
            pub(super) fn try_new<Error_>(
                herd: Herd,
                lookup_builder: impl for<'this> ::core::ops::FnOnce(
                    &'this Herd,
                ) -> ::core::result::Result<DashMap<State, &'this Node<'this>>, Error_>,
                next: Lazy<Box<Generation>>,
            ) -> ::core::result::Result<Generation, Error_> {
                Generation::try_new_or_recover(herd, lookup_builder, next)
                    .map_err(|(error, _heads)| error)
            }
            /**(See also [`GenerationTryBuilder::try_build_or_recover()`](GenerationTryBuilder::try_build_or_recover).) Like [`try_new`](Self::try_new), but all [head fields](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions) are returned in the case of an error. The arguments are as follows:

| Argument | Suggested Use |
| --- | --- |
| `herd` | Directly pass in the value this field should contain |
| `lookup_builder` | Use a function or closure: `(herd: &_) -> Result<lookup: _, Error_>` |
| `next` | Directly pass in the value this field should contain |
*/
            pub(super) fn try_new_or_recover<Error_>(
                herd: Herd,
                lookup_builder: impl for<'this> ::core::ops::FnOnce(
                    &'this Herd,
                ) -> ::core::result::Result<DashMap<State, &'this Node<'this>>, Error_>,
                next: Lazy<Box<Generation>>,
            ) -> ::core::result::Result<Generation, (Error_, Heads)> {
                let herd = ::ouroboros::macro_help::aliasable_boxed(herd);
                let herd_illegal_static_reference = unsafe {
                    ::ouroboros::macro_help::change_lifetime(&*herd)
                };
                let lookup = match lookup_builder(herd_illegal_static_reference) {
                    ::core::result::Result::Ok(value) => value,
                    ::core::result::Result::Err(err) => {
                        return ::core::result::Result::Err((
                            err,
                            Heads {
                                herd: ::ouroboros::macro_help::unbox(herd),
                                next,
                            },
                        ));
                    }
                };
                ::core::result::Result::Ok(unsafe {
                    Self {
                        actual_data: ::core::mem::MaybeUninit::new(GenerationInternal {
                            herd,
                            lookup,
                            next,
                        }),
                    }
                })
            }
            /**(See also [`GenerationAsyncTryBuilder::try_build()`](GenerationAsyncTryBuilder::try_build).) Like [`new`](Self::new), but builders for [self-referencing fields](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions) can return results. If any of them fail, `Err` is returned. If all of them succeed, `Ok` is returned. The arguments are as follows:

| Argument | Suggested Use |
| --- | --- |
| `herd` | Directly pass in the value this field should contain |
| `lookup_builder` | Use a function or closure: `(herd: &_) -> Result<lookup: _, Error_>` |
| `next` | Directly pass in the value this field should contain |
*/
            pub(super) async fn try_new_async<Error_>(
                herd: Herd,
                lookup_builder: impl for<'this> ::core::ops::FnOnce(
                    &'this Herd,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = ::core::result::Result<
                                    DashMap<State, &'this Node<'this>>,
                                    Error_,
                                >,
                            > + 'this,
                        >,
                    >,
                next: Lazy<Box<Generation>>,
            ) -> ::core::result::Result<Generation, Error_> {
                Generation::try_new_or_recover_async(herd, lookup_builder, next)
                    .await
                    .map_err(|(error, _heads)| error)
            }
            /**(See also [`GenerationAsyncTryBuilder::try_build_or_recover()`](GenerationAsyncTryBuilder::try_build_or_recover).) Like [`try_new`](Self::try_new), but all [head fields](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions) are returned in the case of an error. The arguments are as follows:

| Argument | Suggested Use |
| --- | --- |
| `herd` | Directly pass in the value this field should contain |
| `lookup_builder` | Use a function or closure: `(herd: &_) -> Result<lookup: _, Error_>` |
| `next` | Directly pass in the value this field should contain |
*/
            pub(super) async fn try_new_or_recover_async<Error_>(
                herd: Herd,
                lookup_builder: impl for<'this> ::core::ops::FnOnce(
                    &'this Herd,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = ::core::result::Result<
                                    DashMap<State, &'this Node<'this>>,
                                    Error_,
                                >,
                            > + 'this,
                        >,
                    >,
                next: Lazy<Box<Generation>>,
            ) -> ::core::result::Result<Generation, (Error_, Heads)> {
                let herd = ::ouroboros::macro_help::aliasable_boxed(herd);
                let herd_illegal_static_reference = unsafe {
                    ::ouroboros::macro_help::change_lifetime(&*herd)
                };
                let lookup = match lookup_builder(herd_illegal_static_reference).await {
                    ::core::result::Result::Ok(value) => value,
                    ::core::result::Result::Err(err) => {
                        return ::core::result::Result::Err((
                            err,
                            Heads {
                                herd: ::ouroboros::macro_help::unbox(herd),
                                next,
                            },
                        ));
                    }
                };
                ::core::result::Result::Ok(unsafe {
                    Self {
                        actual_data: ::core::mem::MaybeUninit::new(GenerationInternal {
                            herd,
                            lookup,
                            next,
                        }),
                    }
                })
            }
            /**(See also [`GenerationAsyncSendTryBuilder::try_build()`](GenerationAsyncSendTryBuilder::try_build).) Like [`new`](Self::new), but builders for [self-referencing fields](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions) can return results. If any of them fail, `Err` is returned. If all of them succeed, `Ok` is returned. The arguments are as follows:

| Argument | Suggested Use |
| --- | --- |
| `herd` | Directly pass in the value this field should contain |
| `lookup_builder` | Use a function or closure: `(herd: &_) -> Result<lookup: _, Error_>` |
| `next` | Directly pass in the value this field should contain |
*/
            pub(super) async fn try_new_async_send<Error_>(
                herd: Herd,
                lookup_builder: impl for<'this> ::core::ops::FnOnce(
                    &'this Herd,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = ::core::result::Result<
                                    DashMap<State, &'this Node<'this>>,
                                    Error_,
                                >,
                            > + ::core::marker::Send + 'this,
                        >,
                    >,
                next: Lazy<Box<Generation>>,
            ) -> ::core::result::Result<Generation, Error_> {
                Generation::try_new_or_recover_async_send(herd, lookup_builder, next)
                    .await
                    .map_err(|(error, _heads)| error)
            }
            /**(See also [`GenerationAsyncSendTryBuilder::try_build_or_recover()`](GenerationAsyncSendTryBuilder::try_build_or_recover).) Like [`try_new`](Self::try_new), but all [head fields](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions) are returned in the case of an error. The arguments are as follows:

| Argument | Suggested Use |
| --- | --- |
| `herd` | Directly pass in the value this field should contain |
| `lookup_builder` | Use a function or closure: `(herd: &_) -> Result<lookup: _, Error_>` |
| `next` | Directly pass in the value this field should contain |
*/
            pub(super) async fn try_new_or_recover_async_send<Error_>(
                herd: Herd,
                lookup_builder: impl for<'this> ::core::ops::FnOnce(
                    &'this Herd,
                ) -> ::core::pin::Pin<
                        ::ouroboros::macro_help::alloc::boxed::Box<
                            dyn ::core::future::Future<
                                Output = ::core::result::Result<
                                    DashMap<State, &'this Node<'this>>,
                                    Error_,
                                >,
                            > + ::core::marker::Send + 'this,
                        >,
                    >,
                next: Lazy<Box<Generation>>,
            ) -> ::core::result::Result<Generation, (Error_, Heads)> {
                let herd = ::ouroboros::macro_help::aliasable_boxed(herd);
                let herd_illegal_static_reference = unsafe {
                    ::ouroboros::macro_help::change_lifetime(&*herd)
                };
                let lookup = match lookup_builder(herd_illegal_static_reference).await {
                    ::core::result::Result::Ok(value) => value,
                    ::core::result::Result::Err(err) => {
                        return ::core::result::Result::Err((
                            err,
                            Heads {
                                herd: ::ouroboros::macro_help::unbox(herd),
                                next,
                            },
                        ));
                    }
                };
                ::core::result::Result::Ok(unsafe {
                    Self {
                        actual_data: ::core::mem::MaybeUninit::new(GenerationInternal {
                            herd,
                            lookup,
                            next,
                        }),
                    }
                })
            }
            ///Provides limited immutable access to `herd`. This method was generated because the contents of `herd` are immutably borrowed by other fields.
            #[inline(always)]
            pub(super) fn with_herd<'outer_borrow, ReturnType>(
                &'outer_borrow self,
                user: impl for<'this> ::core::ops::FnOnce(
                    &'outer_borrow Herd,
                ) -> ReturnType,
            ) -> ReturnType {
                let field = &unsafe { self.actual_data.assume_init_ref() }.herd;
                user(field)
            }
            ///Provides limited immutable access to `herd`. This method was generated because the contents of `herd` are immutably borrowed by other fields.
            #[inline(always)]
            pub(super) fn borrow_herd<'this>(&'this self) -> &'this Herd {
                &unsafe { self.actual_data.assume_init_ref() }.herd
            }
            ///Provides an immutable reference to `lookup`. This method was generated because `lookup` is a [tail field](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions).
            #[inline(always)]
            pub(super) fn with_lookup<'outer_borrow, ReturnType>(
                &'outer_borrow self,
                user: impl for<'this> ::core::ops::FnOnce(
                    &'outer_borrow DashMap<State, &'this Node<'this>>,
                ) -> ReturnType,
            ) -> ReturnType {
                let field = &unsafe { self.actual_data.assume_init_ref() }.lookup;
                user(field)
            }
            ///Provides a mutable reference to `lookup`. This method was generated because `lookup` is a [tail field](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions). No `borrow_lookup_mut` function was generated because Rust's borrow checker is currently unable to guarantee that such a method would be used safely.
            #[inline(always)]
            pub(super) fn with_lookup_mut<'outer_borrow, ReturnType>(
                &'outer_borrow mut self,
                user: impl for<'this> ::core::ops::FnOnce(
                    &'outer_borrow mut DashMap<State, &'this Node<'this>>,
                ) -> ReturnType,
            ) -> ReturnType {
                let field = &mut unsafe { self.actual_data.assume_init_mut() }.lookup;
                user(field)
            }
            ///Provides an immutable reference to `next`. This method was generated because `next` is a [tail field](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions).
            #[inline(always)]
            pub(super) fn with_next<'outer_borrow, ReturnType>(
                &'outer_borrow self,
                user: impl for<'this> ::core::ops::FnOnce(
                    &'outer_borrow Lazy<Box<Generation>>,
                ) -> ReturnType,
            ) -> ReturnType {
                let field = &unsafe { self.actual_data.assume_init_ref() }.next;
                user(field)
            }
            ///Provides an immutable reference to `next`. This method was generated because `next` is a [tail field](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions).
            #[inline(always)]
            pub(super) fn borrow_next<'this>(
                &'this self,
            ) -> &'this Lazy<Box<Generation>> {
                &unsafe { self.actual_data.assume_init_ref() }.next
            }
            ///Provides a mutable reference to `next`. This method was generated because `next` is a [tail field](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions). No `borrow_next_mut` function was generated because Rust's borrow checker is currently unable to guarantee that such a method would be used safely.
            #[inline(always)]
            pub(super) fn with_next_mut<'outer_borrow, ReturnType>(
                &'outer_borrow mut self,
                user: impl for<'this> ::core::ops::FnOnce(
                    &'outer_borrow mut Lazy<Box<Generation>>,
                ) -> ReturnType,
            ) -> ReturnType {
                let field = &mut unsafe { self.actual_data.assume_init_mut() }.next;
                user(field)
            }
            ///This method provides immutable references to all [tail and immutably borrowed fields](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions).
            #[inline(always)]
            pub(super) fn with<'outer_borrow, ReturnType>(
                &'outer_borrow self,
                user: impl for<'this> ::core::ops::FnOnce(
                    BorrowedFields<'outer_borrow, 'this>,
                ) -> ReturnType,
            ) -> ReturnType {
                let this = unsafe { self.actual_data.assume_init_ref() };
                user(BorrowedFields {
                    next: &this.next,
                    lookup: &this.lookup,
                    herd: unsafe {
                        ::ouroboros::macro_help::change_lifetime(&*this.herd)
                    },
                })
            }
            ///This method provides mutable references to all [tail fields](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions).
            #[inline(always)]
            pub(super) fn with_mut<'outer_borrow, ReturnType>(
                &'outer_borrow mut self,
                user: impl for<'this0, 'this1> ::core::ops::FnOnce(
                    BorrowedMutFields<'outer_borrow, 'this1, 'this0>,
                ) -> ReturnType,
            ) -> ReturnType {
                let this = unsafe { self.actual_data.assume_init_mut() };
                user(BorrowedMutFields {
                    next: &mut this.next,
                    lookup: &mut this.lookup,
                    herd: unsafe {
                        ::ouroboros::macro_help::change_lifetime(&*this.herd)
                    },
                })
            }
            ///This function drops all internally referencing fields and returns only the [head fields](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html#definitions) of this struct.
            #[allow(clippy::drop_ref)]
            #[allow(clippy::drop_copy)]
            #[allow(clippy::drop_non_drop)]
            pub(super) fn into_heads(self) -> Heads {
                let this_ptr = &self as *const _;
                let this: GenerationInternal = unsafe {
                    ::core::mem::transmute_copy(&*this_ptr)
                };
                ::core::mem::forget(self);
                let next = this.next;
                ::core::mem::drop(this.lookup);
                let herd = this.herd;
                Heads {
                    next,
                    herd: ::ouroboros::macro_help::unbox(herd),
                }
            }
        }
        fn type_asserts() {}
    }
    pub use ouroboros_impl_generation::Generation;
    use ouroboros_impl_generation::GenerationBuilder;
    use ouroboros_impl_generation::GenerationAsyncBuilder;
    use ouroboros_impl_generation::GenerationAsyncSendBuilder;
    use ouroboros_impl_generation::GenerationTryBuilder;
    use ouroboros_impl_generation::GenerationAsyncTryBuilder;
    use ouroboros_impl_generation::GenerationAsyncSendTryBuilder;
    pub struct Node<'bump> {
        children: Option<ChildData<'bump>>,
        value: f64,
        acc: f64,
    }
    #[automatically_derived]
    impl<'bump> ::core::fmt::Debug for Node<'bump> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "Node",
                "children",
                &self.children,
                "value",
                &self.value,
                "acc",
                &&self.acc,
            )
        }
    }
    #[automatically_derived]
    impl<'bump> ::core::clone::Clone for Node<'bump> {
        #[inline]
        fn clone(&self) -> Node<'bump> {
            let _: ::core::clone::AssertParamIsClone<Option<ChildData<'bump>>>;
            let _: ::core::clone::AssertParamIsClone<f64>;
            *self
        }
    }
    #[automatically_derived]
    impl<'bump> ::core::marker::Copy for Node<'bump> {}
    pub struct ChildData<'bump>(&'bump [Action]);
    #[automatically_derived]
    impl<'bump> ::core::fmt::Debug for ChildData<'bump> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_tuple_field1_finish(f, "ChildData", &&self.0)
        }
    }
    #[automatically_derived]
    impl<'bump> ::core::clone::Clone for ChildData<'bump> {
        #[inline]
        fn clone(&self) -> ChildData<'bump> {
            let _: ::core::clone::AssertParamIsClone<&'bump [Action]>;
            *self
        }
    }
    #[automatically_derived]
    impl<'bump> ::core::marker::Copy for ChildData<'bump> {}
    pub struct Action {
        mv: Move,
        reward: f64,
        visits: u32,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Action {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "Action",
                "mv",
                &self.mv,
                "reward",
                &self.reward,
                "visits",
                &&self.visits,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Action {
        #[inline]
        fn clone(&self) -> Action {
            let _: ::core::clone::AssertParamIsClone<Move>;
            let _: ::core::clone::AssertParamIsClone<f64>;
            let _: ::core::clone::AssertParamIsClone<u32>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for Action {}
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for Action {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Action {
        #[inline]
        fn eq(&self, other: &Action) -> bool {
            self.mv == other.mv && self.reward == other.reward
                && self.visits == other.visits
        }
    }
    impl Graph {
        pub fn new(state: GameState<BitBoard>) -> Self {
            let this = Self {
                root_gen: Box::new(Generation::build()),
                root_state: state,
            };
            this.root_gen.write_node(state);
            this
        }
        pub fn work(&self) {
            let mut gen = &self.root_gen;
            let mut state = self.root_state.clone();
            loop {
                match gen.select(&state) {
                    SelectResult::Ok(action) => {
                        state.advance(action.mv);
                        gen = gen.borrow_next();
                    }
                    SelectResult::Expand => {
                        gen.expand(&state);
                        break;
                    }
                    SelectResult::Failed => {
                        return;
                    }
                }
            }
        }
    }
    impl Generation {
        pub fn build() -> Self {
            GenerationBuilder {
                herd: Herd::new(),
                lookup_builder: |_| DashMap::new(),
                next: Lazy::new(|| Box::new(Generation::build())),
            }
                .build()
        }
        pub fn get_node(&self, state: &GameState<BitBoard>) -> &Node {
            self.with_lookup(|d| *d.get(&state.clone().into()).unwrap())
        }
        pub fn write_node<'a>(
            &'a self,
            state: &GameState<BitBoard>,
            node: &'a Node<'a>,
        ) {
            self.with(|this| {
                this.lookup.insert(state.clone().into(), node);
            });
        }
        pub fn select(&self, state: &GameState<BitBoard>) -> SelectResult {
            let node = self.get_node(state);
            if node.is_leaf() {
                SelectResult::Expand
            } else {
                match node.select_child() {
                    Some(action) => SelectResult::Ok(action),
                    None => SelectResult::Failed,
                }
            }
        }
        pub fn expand(&self, state: &GameState<BitBoard>) {
            self.with(|this| {
                let node = this.lookup.get(&state.clone().into()).unwrap();
                let mut children = Vec::new();
                for mv in state.legal_moves(true).unwrap() {
                    children
                        .push(Action {
                            mv,
                            reward: 0.0,
                            visits: 0,
                        });
                }
                ChildData(this.herd.get().alloc_slice_copy(&children))
            });
        }
    }
    impl Node<'_> {
        pub fn select_child(&self) -> Option<Action> {
            if true {
                if !self.children.is_some() {
                    ::core::panicking::panic("assertion failed: self.children.is_some()")
                }
            }
            let children = self.children.as_ref().unwrap();
            let total_visits = self.acc;
            let mut best = None;
            let mut best_score = f64::NEG_INFINITY;
            for action in children.0 {
                let score = action.reward + (1.0 / (action.visits as f64).sqrt());
                if score > best_score {
                    best = Some(action);
                    best_score = score;
                }
            }
            Some((*best?).clone())
        }
        pub fn expand_self(
            &mut self,
            state: &GameState<BitBoard>,
            next_gen: &Generation,
        ) {
            if true {
                if !self.children.is_none() {
                    ::core::panicking::panic("assertion failed: self.children.is_none()")
                }
            }
            let moves = state.legal_moves(true).unwrap();
            let children = moves
                .iter()
                .map(|&mv| {
                    let mut state = state.clone();
                    state.advance(mv);
                    Action {
                        mv,
                        reward: 0.0,
                        visits: 0,
                    }
                });
        }
        fn is_leaf(&self) -> bool {
            self.children.is_none()
        }
    }
    struct State {
        board: BitBoard,
        bag: SevenBag,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for State {
        #[inline]
        fn clone(&self) -> State {
            State {
                board: ::core::clone::Clone::clone(&self.board),
                bag: ::core::clone::Clone::clone(&self.bag),
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for State {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for State {
        #[inline]
        fn eq(&self, other: &State) -> bool {
            self.board == other.board && self.bag == other.bag
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for State {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {
            let _: ::core::cmp::AssertParamIsEq<BitBoard>;
            let _: ::core::cmp::AssertParamIsEq<SevenBag>;
        }
    }
    #[automatically_derived]
    impl ::core::hash::Hash for State {
        #[inline]
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
            ::core::hash::Hash::hash(&self.board, state);
            ::core::hash::Hash::hash(&self.bag, state)
        }
    }
    impl From<GameState<BitBoard>> for State {
        fn from(state: GameState<BitBoard>) -> Self {
            Self {
                board: state.board,
                bag: state.bag,
            }
        }
    }
    pub enum SelectResult {
        /// Node has children, return the best one
        Ok(Action),
        /// Node is a leaf, expand it
        Expand,
        /// Selection function failed
        Failed,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for SelectResult {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                SelectResult::Ok(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(f, "Ok", &__self_0)
                }
                SelectResult::Expand => ::core::fmt::Formatter::write_str(f, "Expand"),
                SelectResult::Failed => ::core::fmt::Formatter::write_str(f, "Failed"),
            }
        }
    }
}
pub struct HikariFireflyBot {
    graph: Arc<RwLock<Option<Graph>>>,
}
impl HikariFireflyBot {
    pub fn new() -> Self {
        Self {
            graph: Arc::new(RwLock::new(None)),
        }
    }
    pub fn start(&self) {
        let state = GameState::default();
        self.graph.write().replace(Graph::new(state));
        for _ in 0..4 {
            let worker = Worker::new(self);
            rayon::spawn(move || {
                worker.work_loop();
            });
        }
    }
    pub fn stop(&self) {
        let mut graph = self.graph.write();
        *graph = None;
    }
}
struct Worker {
    graph: Arc<RwLock<Option<Graph>>>,
}
impl Worker {
    fn new(bot: &HikariFireflyBot) -> Self {
        Self { graph: bot.graph.clone() }
    }
    fn work_loop(&self) {
        loop {
            let graph = self.graph.read();
            if let Some(graph) = &*graph {
                graph.work();
            } else {
                {
                    ::std::io::_print(
                        format_args!(
                            "Worker {0} stopping\n",
                            rayon::current_thread_index().unwrap(),
                        ),
                    );
                };
                return;
            }
        }
    }
}
