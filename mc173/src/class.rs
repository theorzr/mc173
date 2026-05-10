//! Emulation of single-inheritance, like Java, in Rust.
//! 
//! Trigger warning this is so unsafe but so cool...


pub trait Class {

    type Header;


    
    // /// The type of the root class.
    // type RootClass: Class;

    // /// This function allows borrowing this class from the root class, this function does
    // /// not check that this is the correct variant and if the variant is not the correct
    // /// one then it's undefined behavior.
    // unsafe fn from_root_unchecked_mut(root: &mut Self::RootClass) -> &mut Self;

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
            $( ..{ $( $subclass_name:ident ),+ $(,)? } )?
        }
    ) => {
        paste::paste! {

            // NOTE: We MUST ensure, for the safety of the _super_ functions and deref 
            // below, that these structures are only ever allocated inside the root class.
            // To do that, we add a token that requires unsafe to instantiate.
            $(#[$meta])*
            $vis struct $name {
                $(
                __parent: <$superclass_name as $crate::class::Class>::Header,
                )?
                $( 
                $(#[$field_meta])*
                $field_vis $field_name: $field_ty,
                )*
            }

            /// Internal tag for the class.
            #[derive(Clone, Copy)]
            pub enum [<$name Tag>] {
                $($( $subclass_name, )*)?
            }

            // A structure we use to store this class as a parent class inside subclasses.
            #[repr(C)]
            $vis struct [<$name Header>] {
                obj: $name,
                tag: [<$name Tag>],
            }

            impl $crate::class::Class for $name {
                type Header = [<$name Header>]; 
            }

            $(
            $vis enum [<$name Ref>]<'a> {
                $( $subclass_name(&'a $subclass_name), )+
            }

            $vis enum [<$name Mut>]<'a> {
                $( $subclass_name(&'a mut $subclass_name), )+
            }

            impl $name {

                pub fn downcast_ref(&self) -> [<$name Ref>]<'_> {
                    
                }

            }
            )?

            // // Only implement the downcast methods on the real type, not the inner type
            // // used for dereference.
            // impl $name {

            //     /// Create a new instance of this class without subclass.
            //     #[inline]
            //     pub fn new_default() -> <Self as $crate::class::Class>::RootClass {
            //         // SAFETY: The tag corresponds to the initialized union variant.
            //         unsafe {
            //             Self::__new([<$name Tag>]::None, [<$name Union>] {
            //                 none: (),
            //             })
            //         }
            //     }
                
            //     #[inline]
            //     pub fn new_with(func: impl FnOnce(&mut Self)) -> <Self as $crate::class::Class>::RootClass {
            //         let mut ret = Self::new_default();
            //         // SAFETY: We just initialized the variant, so it should, if the logic
            //         // elsewhere is properly implemented.
            //         func(unsafe { <Self as $crate::class::Class>::from_root_unchecked_mut(&mut ret) });
            //         ret
            //     }

            //     /// Internal function to create a new instance of this function with the
            //     /// given tag and union value, we use this to initialize all fields
            //     /// here with their defaults.
            //     /// 
            //     /// SAFETY: The caller must ensure that the initialized variant of the
            //     /// union match the given tag!
            //     unsafe fn __new(tag: [<$name Tag>], un: [<$name Union>]) -> <Self as $crate::class::Class>::RootClass {
            //         // SAFETY: We ensure that this class is only existing inside the
            //         // root class or inside another subclass.
            //         let token = unsafe { $crate::class::Token::new() };
            //         $crate::class::class!(@superclass_new: $($superclass_name)?, [<__new_ $name:snake>], Self {
            //             $( $field_name: $crate::class::class!(@field_default: $( $field_default )?), )*
            //             __tag: tag,
            //             __union: un,
            //             __token: token,
            //         })
            //     }

            //     $($(
            //     /// Internal function, exposed for convenience.
            //     /// This is unsafe because we have to ensure that the constructed subclass
            //     /// will only land here and not live by itself, if so this will cause UB
            //     /// in the dereferencing of this function.
            //     #[doc(hidden)]
            //     pub unsafe fn [<__new_ $subclass_name:snake>](subclass: $subclass_name) -> <Self as $crate::class::Class>::RootClass {
            //         // SAFETY: The tag corresponds to the initialized union variant.
            //         unsafe {
            //             Self::__new([<$name Tag>]::$subclass_name, [<$name Union>] {
            //                 [<$subclass_name:snake>]: ::std::mem::ManuallyDrop::new(subclass),
            //             })
            //         }
            //     }
            //     )*)?

            //     /// Clone this class as a standalone class, this function is unsafe 
            //     /// because no subclass should ever be owned independently from the
            //     /// root class.
            //     #[doc(hidden)]
            //     pub unsafe fn __clone(&self) -> Self {
            //         // SAFETY: We ensure that this class is only existing inside the
            //         // root class or inside another subclass.
            //         let token = unsafe { $crate::class::Token::new() };
            //         Self {
            //             $( $field_name: Clone::clone(&self.$field_name), )*
            //             __tag: self.__tag,
            //             __union: match self.__tag {
            //                 [<$name Tag>]::None => [<$name Union>] { none: () },
            //                 $($( [<$name Tag>]::$subclass_name => unsafe { [<$name Union>] { [<$subclass_name:snake>]: ::std::mem::ManuallyDrop::new(self.__union.[<$subclass_name:snake>].__clone()) } }, )*)?
            //             },
            //             __token: token,
            //         }
            //     }

            //     #[inline]
            //     pub fn downcast_ref(&self) -> [<$name Ref>]<'_> {
            //         // SAFETY: The tag should always contain the tag of the currently 
            //         // valid and initialized variant in the subclass union.
            //         match self.__tag {
            //             [<$name Tag>]::None => [<$name Ref>]::None(::std::marker::PhantomData),
            //             $($( [<$name Tag>]::$subclass_name => unsafe { [<$name Ref>]::$subclass_name(&self.__union.[<$subclass_name:snake>]) }, )*)?
            //         }
            //     }

            //     #[inline]
            //     pub fn downcast_mut(&mut self) -> [<$name Mut>]<'_> {
            //         // SAFETY: Don't want to repeat: read above!
            //         match self.__tag {
            //             [<$name Tag>]::None => [<$name Mut>]::None(::std::marker::PhantomData),
            //             $($( [<$name Tag>]::$subclass_name => unsafe { [<$name Mut>]::$subclass_name(&mut self.__union.[<$subclass_name:snake>]) }, )*)?
            //         }
            //     }

            // }

            // // Both real type and inner type have the same debug printing.
            // impl ::std::fmt::Debug for $name {
            //     fn fmt(&self, fmt: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
            //         let mut fmt = fmt.debug_struct(stringify!($name));
            //         $( fmt.field(stringify!($field_name), &self.$field_name); )*
            //         match self.__tag {
            //             [<$name Tag>]::None => {}
            //             $($( 
            //             [<$name Tag>]::$subclass_name => {
            //                 fmt.field(stringify!([<$subclass_name:snake>]), unsafe { &*self.__union.[<$subclass_name:snake>] });
            //             }
            //             )*)?
            //         };
            //         fmt.finish()
            //     }
            // }

            // // Specific drop implementation for the subclass union...
            // impl Drop for $name {
            //     fn drop(&mut self) {
            //         // SAFETY: The tag should always contain the tag of the currently 
            //         // valid and initialized variant in the subclass union.
            //         match self.__tag {
            //             [<$name Tag>]::None => ( /* don't need to drop '()' */ ),
            //             $($( [<$name Tag>]::$subclass_name => unsafe { ::std::mem::ManuallyDrop::drop(&mut self.__union.[<$subclass_name:snake>]) }, )*)?
            //         }
            //     }
            // }

            // If there are superclass!
            $(
            impl ::std::ops::Deref for $name {
                type Target = $superclass_name;
                fn deref(&self) -> &Self::Target {
                    &self.__parent.obj
                }
            }

            impl ::std::ops::DerefMut for $name {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    &mut self.__parent.obj
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
                        &mut superclass_instance.__union.[<$name:snake>]
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


// pub struct Root {

// }

// pub enum RootTag {
//     Sub,
// }

// pub struct Sub {
//     __parent: SubParent,
// }

// #[repr(C)]
// struct SubParent {
//     __obj: Root,
//     __tag: RootTag,
// }

// impl Root {

//     pub fn downcast_ref(&self) {
//         match self.__tag {
//             RootTag::Sub => todo!(),
//         }
//     }

// }


#[cfg(test)]
mod tests {

    use super::*;

    class! {
        struct Root {
            id: u32,
            ..{ Foo, Intermediate }
        }
    }

    class! {
        struct Foo: Root {
            foo: bool,
        }
    }

    class! {
        struct Intermediate: Root {
            inter: u8,
            ..{ Bar, Baz }
        }
    }

    class! {
        struct Bar: Intermediate {
            bar: f64,
        }
    }

    class! {
        struct Baz: Intermediate {
            baz: f32,
        }
    }

    #[test]
    fn classes() {

        let obj1 = Root::new_default();
        assert!(matches!(obj1.downcast_ref(), RootRef::None(_)));
        assert_eq!(obj1.id, 0);
        
        let obj2 = Baz::new_with(|baz| {
            baz.baz = 3.14;
            baz.inter = 8;
            baz.id = 99;
        });
        assert_eq!(obj2.id, 99);
        let RootRef::Intermediate(obj2) = obj2.downcast_ref() else { panic!() };
        assert_eq!(obj2.id, 99);
        assert_eq!(obj2.inter, 8);
        let IntermediateRef::Baz(obj2) = obj2.downcast_ref() else { panic!() };
        assert_eq!(obj2.id, 99);
        assert_eq!(obj2.inter, 8);
        assert_eq!(obj2.baz, 3.14);

    }

}
