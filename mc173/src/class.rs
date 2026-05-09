//! Emulation of single-inheritance, like Java, in Rust.
//! 
//! Trigger warning this is so unsafe but so cool...


pub trait Class {

    /// The type of the root class.
    type RootClass: Class;

    /// This function allows borrowing this class from the root class, this function does
    /// not check that this is the correct variant and if the variant is not the correct
    /// one then it's undefined behavior.
    unsafe fn from_root_unchecked_mut(root: &mut Self::RootClass) -> &mut Self;

}

pub struct Token(());
impl Token {
    
    /// It is unsafe to construct this token, because it allows manual instantiation of
    /// subclasses, which should only ever be stored inside another another subclass
    /// or inside the root class. Only the root class should be the actual instantiation.
    #[inline]
    pub const unsafe fn new() -> Self {
        Self(())
    }

}

macro_rules! class {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident $( : $superclass_name:ident )? {
            $(
                $(#[$field_meta:meta])*
                $field_vis:vis $field_name:ident : $field_ty:ty $( = $field_default:expr )? ,
            )*
            $( ..{ $( $subclass_name:ident ),* $(,)? } )?
        }
    ) => {
        paste::paste! {

            // NOTE: This is our real type the users will be implementing methods onto,
            // it has the exact same layout as its inner type, we use this distinction
            // to return the inner type upon deref, to avoid having access to the _super_
            // methods just from the deref (this would not be a problem, but could be
            // annoying in case of duplicate definitions with overloads).
            $(#[$meta])*
            #[repr(transparent)]
            $vis struct $name([<$name Inner>]);

            // NOTE: We MUST ensure, for the safety of the _super_ functions and deref 
            // below, that these structures are only ever allocated inside the root class.
            // To do that, we add a token that requires unsafe to instantiate.
            $vis struct [<$name Inner>] {
                $( 
                $(#[$field_meta])*
                $field_vis $field_name: $field_ty,
                )*
                __tag: [<$name Tag>],
                __union: [<$name Union>],
                __token: $crate::class::Token,
            }

            // Impl the class trait, it depends on wether this is a subclass or a parent
            // class!
            $crate::class::class!(@impl_class_trait: $name, $($superclass_name)?);

            /// Internal tag for the class.
            #[derive(Clone, Copy)]
            enum [<$name Tag>] {
                None,
                $($( $subclass_name, )*)?
            }

            // NOTE: We use repr(C) because we want to be sure that all variants are at
            // offset 0 in the union, which is only guaranteed by this repr...
            #[repr(C)]
            union [<$name Union>] {
                none: (),
                $($( [<$subclass_name:snake>]: ::std::mem::ManuallyDrop<$subclass_name>, )*)?
            }

            $vis enum [<$name Ref>]<'a> {
                None(::std::marker::PhantomData<&'a ()>),
                $($( $subclass_name(&'a $subclass_name), )*)?
            }

            $vis enum [<$name Mut>]<'a> {
                None(::std::marker::PhantomData<&'a mut ()>),
                $($( $subclass_name(&'a mut $subclass_name), )*)?
            }

            // This top-level type dereferences to the inner type, if this class has a
            // superclass, this inner type will dereference to the superclass' inner.
            impl ::std::ops::Deref for $name {
                type Target = [<$name Inner>];
                #[inline]
                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }

            impl ::std::ops::DerefMut for $name {
                #[inline]
                fn deref_mut(&mut self) -> &mut Self::Target {
                    &mut self.0
                }
            }

            // Only implement the downcast methods on the real type, not the inner type
            // used for dereference.
            impl $name {

                /// Create a new instance of this class without subclass.
                #[inline]
                pub fn new_default() -> <Self as $crate::class::Class>::RootClass {
                    // SAFETY: The tag corresponds to the initialized union variant.
                    unsafe {
                        Self::__new([<$name Tag>]::None, [<$name Union>] {
                            none: (),
                        })
                    }
                }
                
                #[inline]
                pub fn new_with(func: impl FnOnce(&mut Self)) -> <Self as $crate::class::Class>::RootClass {
                    let mut ret = Self::new_default();
                    // SAFETY: We just initialized the variant, so it should, if the logic
                    // elsewhere is properly implemented.
                    func(unsafe { <Self as $crate::class::Class>::from_root_unchecked_mut(&mut ret) });
                    ret
                }

                /// Internal function to create a new instance of this function with the
                /// given tag and union value, we use this to initialize all fields
                /// here with their defaults.
                /// 
                /// SAFETY: The caller must ensure that the initialized variant of the
                /// union match the given tag!
                unsafe fn __new(tag: [<$name Tag>], un: [<$name Union>]) -> <Self as $crate::class::Class>::RootClass {
                    // SAFETY: We ensure that this class is only existing inside the
                    // root class or inside another subclass.
                    let token = unsafe { $crate::class::Token::new() };
                    $crate::class::class!(@superclass_new: $($superclass_name)?, [<__new_ $name:snake>], Self([<$name Inner>] {
                        $( $field_name: $crate::class::class!(@field_default: $( $field_default )?), )*
                        __tag: tag,
                        __union: un,
                        __token: token,
                    }))
                }

                $($(
                /// Internal function, exposed for convenience.
                /// This is unsafe because we have to ensure that the constructed subclass
                /// will only land here and not live by itself, if so this will cause UB
                /// in the dereferencing of this function.
                #[doc(hidden)]
                pub unsafe fn [<__new_ $subclass_name:snake>](subclass: $subclass_name) -> <Self as $crate::class::Class>::RootClass {
                    // SAFETY: The tag corresponds to the initialized union variant.
                    unsafe {
                        Self::__new([<$name Tag>]::$subclass_name, [<$name Union>] {
                            [<$subclass_name:snake>]: ::std::mem::ManuallyDrop::new(subclass),
                        })
                    }
                }
                )*)?

                /// Clone this class as a standalone class, this function is unsafe 
                /// because no subclass should ever be owned independently from the
                /// root class.
                #[doc(hidden)]
                pub unsafe fn __clone(&self) -> Self {
                    // SAFETY: We ensure that this class is only existing inside the
                    // root class or inside another subclass.
                    let token = unsafe { $crate::class::Token::new() };
                    Self([<$name Inner>] {
                        $( $field_name: Clone::clone(&self.$field_name), )*
                        __tag: self.0.__tag,
                        __union: match self.0.__tag {
                            [<$name Tag>]::None => [<$name Union>] { none: () },
                            $($( [<$name Tag>]::$subclass_name => unsafe { [<$name Union>] { [<$subclass_name:snake>]: ::std::mem::ManuallyDrop::new(self.0.__union.[<$subclass_name:snake>].__clone()) } }, )*)?
                        },
                        __token: token,
                    })
                }

                #[inline]
                pub fn downcast_ref(&self) -> [<$name Ref>]<'_> {
                    // SAFETY: The tag should always contain the tag of the currently 
                    // valid and initialized variant in the subclass union.
                    match self.0.__tag {
                        [<$name Tag>]::None => [<$name Ref>]::None(::std::marker::PhantomData),
                        $($( [<$name Tag>]::$subclass_name => unsafe { [<$name Ref>]::$subclass_name(&self.0.__union.[<$subclass_name:snake>]) }, )*)?
                    }
                }

                #[inline]
                pub fn downcast_mut(&mut self) -> [<$name Mut>]<'_> {
                    // SAFETY: Don't want to repeat: read above!
                    match self.0.__tag {
                        [<$name Tag>]::None => [<$name Mut>]::None(::std::marker::PhantomData),
                        $($( [<$name Tag>]::$subclass_name => unsafe { [<$name Mut>]::$subclass_name(&mut self.0.__union.[<$subclass_name:snake>]) }, )*)?
                    }
                }

            }

            // Both real type and inner type have the same debug printing.
            impl ::std::fmt::Debug for [<$name Inner>] {
                fn fmt(&self, fmt: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    let mut fmt = fmt.debug_struct(stringify!($name));
                    $( fmt.field(stringify!($field_name), &self.$field_name); )*
                    match self.__tag {
                        [<$name Tag>]::None => {}
                        $($( 
                        [<$name Tag>]::$subclass_name => {
                            fmt.field(stringify!([<$subclass_name:snake>]), unsafe { &*self.__union.[<$subclass_name:snake>] });
                        }
                        )*)?
                    };
                    fmt.finish()
                }
            }

            impl ::std::fmt::Debug for $name {
                #[inline]
                fn fmt(&self, fmt: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    ::std::fmt::Debug::fmt(&self.0, fmt)
                }
            }

            // Specific drop implementation for the subclass union...
            impl Drop for [<$name Inner>] {
                fn drop(&mut self) {
                    // SAFETY: The tag should always contain the tag of the currently 
                    // valid and initialized variant in the subclass union.
                    match self.__tag {
                        [<$name Tag>]::None => ( /* don't need to drop '()' */ ),
                        $($( [<$name Tag>]::$subclass_name => unsafe { ::std::mem::ManuallyDrop::drop(&mut self.__union.[<$subclass_name:snake>]) }, )*)?
                    }
                }
            }

            // If there are superclass!
            $(
            impl ::std::ops::Deref for [<$name Inner>] {
                // NOTE: We directly create a reference to the superclass's inner type 
                // directly because we don't want to give access to its implemented methods.
                // Note that because the real superclass type is transparent over the inner
                // type, we can safely switch interpretation between one and the other.
                type Target = <$superclass_name as ::std::ops::Deref>::Target;
                fn deref(&self) -> &Self::Target {
                    // SAFETY: We know that if this object exists, it can only exist 
                    // inside the union of its parent class, therefore we just have to 
                    // find the offset of the union within that parent class.
                    const OFFSET: usize = ::std::mem::offset_of!($superclass_name, 0.__union);
                    unsafe { &*((self as *const [<$name Inner>]).byte_sub(OFFSET) as *const Self::Target) }
                }
            }

            impl ::std::ops::DerefMut for [<$name Inner>] {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    // SAFETY: Don't want to repeat: read above!
                    const OFFSET: usize = ::std::mem::offset_of!($superclass_name, 0.__union);
                    unsafe { &mut *((self as *mut [<$name Inner>]).byte_sub(OFFSET) as *mut Self::Target) }
                }
            }

            // These super function allows getting a real and explicit reference to the super
            // class (mut) reference, to access its implemented functions.
            impl $name {

                /// Get a shared reference to the superclass.
                pub fn super_ref(&self) -> &$superclass_name {
                    // SAFETY: Read above, this is exactly the same, expect that we create
                    // a reference the the superclass type, and not the superclass's inner
                    // type, but because they have the same layout, we can interpret one
                    // or the other depending on our needs.
                    const OFFSET: usize = ::std::mem::offset_of!($superclass_name, 0.__union);
                    unsafe { &*((self as *const $name).byte_sub(OFFSET) as *const $superclass_name) }
                }

                /// Get an exclusive reference to the superclass.
                pub fn super_mut(&mut self) -> &mut $superclass_name {
                    // SAFETY: Read above, this is exactly the same, expect that we create
                    // a reference the the superclass type, and not the superclass's inner
                    // type, but because they have the same layout, we can interpret one
                    // or the other depending on our needs.
                    const OFFSET: usize = ::std::mem::offset_of!($superclass_name, 0.__union);
                    unsafe { &mut *((self as *mut $name).byte_sub(OFFSET) as *mut $superclass_name) }
                }

            }
            )?
            
        }
    };
    ( @impl_class_trait: $name:ident, /* no superclass */ ) => {
        paste::paste! {

            impl $crate::class::Class for $name {

                type RootClass = $name;

                #[inline]
                unsafe fn from_root_unchecked_mut(root: &mut Self::RootClass) -> &mut Self {
                    root
                }

            }

            // We only implement the real clone on the root class!
            impl Clone for $name {
                fn clone(&self) -> Self {
                    // SAFETY: We are the root class, so we can finally clone!
                    unsafe { self.__clone() }
                }
            }

        }
    };
    ( @impl_class_trait: $name:ident, $superclass_name:ident ) => {
        paste::paste! {

            impl $crate::class::Class for $name {

                type RootClass = <$superclass_name as $crate::class::Class>::RootClass;

                #[inline]
                unsafe fn from_root_unchecked_mut(root: &mut Self::RootClass) -> &mut Self {
                    // SAFETY: Here we are concerting from the root class to superclass
                    // of this class, then we just assume that the union's variant is
                    // initialized!
                    unsafe {
                        let superclass_instance: &mut $superclass_name = $crate::class::Class::from_root_unchecked_mut(root);
                        &mut superclass_instance.0.__union.[<$name:snake>]
                    }
                }

            }

        }
    };
    ( @superclass_new: /* no superclass */, $superclass_new:ident, $init:expr ) => { $init };
    ( @superclass_new: $superclass_name:ident, $superclass_new:ident, $init:expr ) => { 
        unsafe { $superclass_name::$superclass_new($init) }
    };
    ( @field_default: /* no default */ ) => { Default::default() };
    ( @field_default: $field_default:expr ) => { $field_default };
}

pub(crate) use class as class;
