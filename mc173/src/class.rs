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
                $field_vis:vis $field_name:ident : $field_ty:ty ,
            )*
            $( ..{ $( $subclass_name:ident ),+ $(,)? } )?
        }
    ) => {
        paste::paste! {

            // NOTE: We MUST ensure, for the safety of the _super_ functions and deref 
            // below, that these structures are only ever allocated inside the root class.
            // To do that, we add a token that requires unsafe to instantiate.
            $(#[$meta])*
            #[derive(Clone)]
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
            enum [<$name Tag>] {
                $($( $subclass_name, )*)?
            }

            // A structure we use to store this class as a parent class inside subclasses.
            #[derive(Clone)]
            $vis struct [<$name Header>] {
                obj: $name,
                tag: [<$name Tag>],
            }

            impl $crate::class::Class for $name {
                type Header = [<$name Header>]; 
            }

            impl Default for $name {
                #[inline]
                fn default() -> Self {
                    Self {
                        $( __parent: <$superclass_name as Default>::default().[<__into_header_ $name:snake>](), )?
                        $( $field_name: <$field_ty as Default>::default(), )*
                    }
                }
            }

            impl ::std::fmt::Debug for $name {
                fn fmt(&self, fmt: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                    let mut fmt = fmt.debug_struct(stringify!($name));
                    $( fmt.field(stringify!([<$superclass_name:snake>]), &self.__parent.obj); )?
                    $( fmt.field(stringify!($field_name), &self.$field_name); )*
                    fmt.finish()
                }
            }

            $(
            $vis enum [<$name Ref>]<'a> {
                $( $subclass_name(&'a $subclass_name), )+
            }

            $vis enum [<$name Mut>]<'a> {
                $( $subclass_name(&'a mut $subclass_name), )+
            }

            impl $name {

                $(
                #[inline]
                pub fn [<__into_header_ $subclass_name:snake>](self) -> [<$name Header>] {
                    [<$name Header>] {
                        obj: self,
                        tag: [<$name Tag>]::$subclass_name,
                    }
                }
                )+

                #[inline]
                pub fn downcast_ref(&self) -> [<$name Ref>]<'_> {
                    // SAFETY: If this class is used as a parent class, we know that it 
                    // must be existing inside the header structure, which is itself 
                    // inside the subclass.
                    // SAFETY: The tag is a Copy enum, therefore we read it without caring
                    // about dropping or anything.
                    unsafe {
                        let header_ptr = (self as *const Self).byte_sub(::std::mem::offset_of!([<$name Header>], obj)).cast::<[<$name Header>]>();
                        let tag_ptr = header_ptr.byte_add(::std::mem::offset_of!([<$name Header>], tag)).cast::<[<$name Tag>]>();
                        match tag_ptr.read() {
                            $( [<$name Tag>]::$subclass_name => [<$name Ref>]::$subclass_name(&*header_ptr.byte_sub(::std::mem::offset_of!($subclass_name, __parent)).cast::<$subclass_name>()), )+
                        }
                    }
                }

                #[inline]
                pub fn downcast_mut(&mut self) -> [<$name Mut>]<'_> {
                    // SAFETY: If this class is used as a parent class, we know that it 
                    // must be existing inside the header structure, which is itself 
                    // inside the subclass.
                    // SAFETY: The tag is a Copy enum, therefore we read it without caring
                    // about dropping or anything.
                    unsafe {
                        let header_ptr = (self as *mut Self).byte_sub(::std::mem::offset_of!([<$name Header>], obj)).cast::<[<$name Header>]>();
                        let tag_ptr = header_ptr.byte_add(::std::mem::offset_of!([<$name Header>], tag)).cast::<[<$name Tag>]>();
                        match tag_ptr.read() {
                            $( [<$name Tag>]::$subclass_name => [<$name Mut>]::$subclass_name(&mut *header_ptr.byte_sub(::std::mem::offset_of!($subclass_name, __parent)).cast::<$subclass_name>()), )+
                        }
                    }
                }

            }
            )?

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
