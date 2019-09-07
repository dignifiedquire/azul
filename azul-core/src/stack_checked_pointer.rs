use std::{
    any::TypeId,
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
};

/// A `StackCheckedPointer<T>` is a type-erased, raw pointer to a
/// value **inside** of `T`.
///
/// Since we know that the pointer is "checked" to be contained (on the stack)
/// within `&T as usize` and `&T as usize + mem::size_of::<T>()`,
/// `StackCheckedPointer<T>` has the same lifetime as `T`
/// (but the type is erased, so it can be stored independent from `T`s lifetime).
///
/// Note for enums: Should the pointer point to an enum instead of a struct and
/// the enum (which in Rust is a union) changes its variant, the behaviour of
/// invoking this pointer is undefined (likely to segfault).
pub struct StackCheckedPointer<T> {
    /// Type-erased pointer to a value on the stack in the `app_data.data`
    /// model. When invoking default methods, we have to store a pointer to
    /// the data we should update, but storing it in a `Box<T>` to
    /// erase the type doesn't help anything - we trust the user of this
    /// pointer to know the exact type of this pointer.
    internal: *const (),
    /// Marker so that one stack checked pointer can't be shared across
    /// two data models that are both `T: Layout`.
    marker: PhantomData<T>,
    /// ID of the erased type behind the pointer
    pointer_type: TypeId,
}

impl<T: Sized + 'static> StackCheckedPointer<T> {
    /// Creates a `StackCheckedPointer` that points to the entire struct
    pub fn new_entire_struct(stack: &T) -> Self {
        StackCheckedPointer::new(stack, stack).unwrap()
    }
}

impl<T> StackCheckedPointer<T> {
    /// Validates that the pointer to U is contained in T.
    ///
    /// This means that the lifetime of U is the same lifetime as T -
    /// the returned `StackCheckedPointer` is valid for as long as `stack`
    /// is valid.
    pub fn new<U: Sized + 'static>(stack: &T, pointer: &U) -> Option<Self> {
        if is_subtype_of(stack, pointer) {
            Some(Self {
                internal: pointer as *const _ as *const (),
                marker: PhantomData,
                pointer_type: TypeId::of::<U>(),
            })
        } else {
            None
        }
    }

    /// **UNSAFE**: Invoke the pointer with a function pointer that can
    /// modify the pointer. It isn't checked that the type that the
    /// `StackCheckedPointer` was created with is the same as this `U`,
    /// but they **must be the same type**. This can't be checked since
    /// the type has been (deliberately) erased.
    ///
    /// **NOTE**: To avoid undefined behaviour, you **must** check that
    /// the `StackCheckedPointer` isn't mutably aliased at the time of
    /// calling the callback.
    #[inline]
    pub(crate) unsafe fn cast<'a, U: Sized + 'static>(&'a self) -> &'a mut U {
        #[cfg(debug_assertions)]
        {
            let type_id_new = TypeId::of::<U>();
            if type_id_new != self.pointer_type {
                panic!(
                    "Tried to cast a StackCheckedPointer to an invalid type - expected {:?}, got {:?}",
                    self.pointer_type, type_id_new
                );
            }
        }
        &mut *(self.internal as *mut U)
    }
}

// #[derive(Debug, Clone, PartialEq, Hash, Eq)] for StackCheckedPointer<T>

impl<T> fmt::Debug for StackCheckedPointer<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "StackCheckedPointer {{ internal: 0x{:x}, marker: {:?} }}",
            self.internal as usize, self.marker
        )
    }
}

impl<T> Clone for StackCheckedPointer<T> {
    fn clone(&self) -> Self {
        StackCheckedPointer {
            internal: self.internal,
            marker: self.marker.clone(),
            pointer_type: self.pointer_type,
        }
    }
}

impl<T> Hash for StackCheckedPointer<T> {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        state.write_usize(self.internal as usize);
    }
}

impl<T> PartialEq for StackCheckedPointer<T> {
    fn eq(&self, rhs: &Self) -> bool {
        self.internal as usize == rhs.internal as usize
    }
}

impl<T> Eq for StackCheckedPointer<T> {}
impl<T> Copy for StackCheckedPointer<T> {}

/// Returns true if U is a type inside of T
///
/// i.e:
///
/// ```ignore
/// # struct Data { i: usize, p: Vec<usize> }
/// let data = Data { i: 5, p: vec![5] };
///
/// // true because i is inside of data
/// assert_eq!(is_subtype_of(&data, &data.i), true);
/// // true because p is inside of data
/// assert_eq!(is_subtype_of(&data, &data.p), true);
/// // false because p is heap-allocated, therefore not inside of data
/// assert_eq!(is_subtype_of(&data, &data.p[0]), false);
/// ```
fn is_subtype_of<T, U>(data: &T, subtype: &U) -> bool {
    // determine in which direction the stack grows
    use std::mem::size_of;

    struct Invalid {
        a: u64,
        b: u64,
    }

    let invalid = Invalid { a: 0, b: 0 };

    let stack_grows_down = &invalid.b as *const _ as usize > &invalid.a as *const _ as usize;

    // calculate if U is a subtype of T
    let st = subtype as *const _ as usize;
    let t = data as *const _ as usize;

    if stack_grows_down {
        st >= t && st + size_of::<U>() <= t + size_of::<T>()
    } else {
        st <= t && st - size_of::<U>() >= t - size_of::<T>()
    }
}

#[test]
fn test_reflection_subtyping() {
    struct Data {
        i: usize,
        p: Vec<usize>,
    }
    let data = Data { i: 5, p: vec![5] };

    assert_eq!(is_subtype_of(&data, &data.i), true);
    assert_eq!(is_subtype_of(&data, &data.p), true);
    assert_eq!(is_subtype_of(&data, &data.p[0]), false);
}
