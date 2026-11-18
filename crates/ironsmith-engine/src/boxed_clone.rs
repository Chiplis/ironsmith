/// Define a trait's object-safe clone operation alongside the trait itself.
macro_rules! define_boxed_clone {
    ($clone_trait:ident, $behavior:ident) => {
        #[doc = concat!("Object-safe cloning for [`", stringify!($behavior), "`].")]
        pub trait $clone_trait {
            /// Clone the concrete value into a boxed trait object.
            fn clone_boxed(&self) -> Box<dyn $behavior>;
        }

        impl<T> $clone_trait for T
        where
            T: $behavior + Clone + 'static,
        {
            fn clone_boxed(&self) -> Box<dyn $behavior> {
                Box::new(self.clone())
            }
        }
    };
}

pub(crate) use define_boxed_clone;
