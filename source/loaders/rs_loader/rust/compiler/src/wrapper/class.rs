// use anyhow::Result;
// use impl_trait_for_tuples::*;
use std::any::*;
use std::cell::Ref;
use std::cell::RefCell;
use std::cell::RefMut;
use std::collections::HashMap;
use std::fmt;
use std::os::raw::c_void;
use std::sync::Arc;

type Result<T, E = i32> = core::result::Result<T, E>;

type Attributes = HashMap<&'static str, AttributeGetter>;
type AttributeSetters = HashMap<&'static str, AttributeSetter>;
type ClassMethods = HashMap<&'static str, ClassMethod>;
type InstanceMethods = HashMap<&'static str, InstanceMethod>;
pub type MetacallValue = *mut c_void;

#[derive(Clone)]
pub struct Class {
    /// The class name. Defaults to the `std::any::type_name`
    pub name: String,
    pub type_id: TypeId,
    constructor: Option<Constructor>,
    attributes: Attributes,
    attr_setters: AttributeSetters,
    instance_methods: InstanceMethods,
    pub class_methods: ClassMethods,
}

impl Class {
    pub fn builder<T: 'static>() -> ClassBuilder<T> {
        ClassBuilder::new()
    }

    pub fn init(&self, fields: Vec<MetacallValue>) -> Instance {
        self.constructor.as_ref().unwrap().invoke(fields).unwrap()
    }

    pub fn call(&self, attr: &str, args: Vec<MetacallValue>) -> Result<MetacallValue> {
        let attr = self.class_methods.get(attr).unwrap();

        attr.clone().invoke(args)
    }

    fn get_method(&self, name: &str) -> Option<InstanceMethod> {
        self.instance_methods.get(name).cloned()
    }
}

#[derive(Clone)]
pub struct ClassBuilder<T> {
    class: Class,
    /// A type marker. Used to ensure methods have the correct type.
    ty: std::marker::PhantomData<T>,
}
impl<T> ClassBuilder<T>
where
    T: 'static,
{
    /// Create a new class builder.
    fn new() -> Self {
        let fq_name = std::any::type_name::<T>().to_string();
        let short_name = fq_name.split("::").last().expect("type has invalid name");
        Self {
            class: Class {
                name: short_name.to_string(),
                constructor: None,
                attributes: Attributes::new(),
                attr_setters: AttributeSetters::new(),
                instance_methods: InstanceMethods::new(),
                class_methods: ClassMethods::new(),
                type_id: TypeId::of::<T>(),
            },
            ty: std::marker::PhantomData,
        }
    }
    /// Set the name of the polar class.
    pub fn name(mut self, name: &str) -> Self {
        self.class.name = name.to_string();
        self
    }

    /// Finish building a build the class
    pub fn build(self) -> Class {
        self.class
    }

    pub fn add_attribute_getter<F, R>(mut self, name: &'static str, f: F) -> Self
    where
        F: Fn(&T) -> R + Send + Sync + 'static,
        R: ToMetaResult,
        T: 'static,
    {
        self.class.attributes.insert(name, AttributeGetter::new(f));
        self
    }

    pub fn add_attribute_setter<F, Arg>(mut self, name: &'static str, f: F) -> Self
    where
        Arg: FromMeta,
        F: Fn(Arg, &mut T) + 'static,
        T: 'static,
    {
        self.class
            .attr_setters
            .insert(name, AttributeSetter::new(f));
        self
    }

    pub fn with_constructor<F, Args>(f: F) -> Self
    where
        F: Function<Args, Result = T>,
        T: Send + Sync,
        Args: FromMetaList,
    {
        let mut class: ClassBuilder<T> = ClassBuilder::new();
        class = class.set_constructor(f);
        class
    }

    pub fn set_constructor<F, Args>(mut self, f: F) -> Self
    where
        F: Function<Args, Result = T>,
        T: Send + Sync,
        Args: FromMetaList,
    {
        self.class.constructor = Some(Constructor::new(f));
        self
    }

    pub fn add_method<F, Args, R>(mut self, name: &'static str, f: F) -> Self
    where
        Args: FromMetaList,
        F: Method<T, Args, Result = R>,
        R: ToMetaResult + 'static,
    {
        self.class
            .instance_methods
            .insert(name, InstanceMethod::new(f));
        self
    }

    pub fn add_class_method<F, Args, R>(mut self, name: &'static str, f: F) -> Self
    where
        F: Function<Args, Result = R>,
        Args: FromMetaList + std::fmt::Debug,
        R: ToMetaResult + std::fmt::Debug + 'static,
    {
        self.class.class_methods.insert(name, ClassMethod::new(f));
        self
    }
}
#[derive(Clone)]
pub struct Instance {
    inner: Arc<RefCell<dyn std::any::Any + Send + Sync>>,

    /// The type name of the Instance, to be used for debugging purposes only.
    /// To get the registered name, use `Instance::name`.
    debug_type_name: &'static str,
}

impl fmt::Debug for Instance {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Instance<{}>", self.debug_type_name)
    }
}

impl Instance {
    /// Create a new instance
    pub fn new<T: Send + Sync + 'static>(instance: T) -> Self {
        Self {
            inner: Arc::new(RefCell::new(instance)),
            debug_type_name: std::any::type_name::<T>(),
        }
    }

    /// Check whether this is an instance of `class`
    pub fn instance_of(&self, class: &Class) -> bool {
        self.type_id() == class.type_id
    }

    pub fn type_id(&self) -> std::any::TypeId {
        self.inner.as_ref().type_id()
    }

    /// Looks up the `Class` for this instance on the provided `host`
    pub fn class<'a>(&self, host: &'a Host) -> Result<&'a Class> {
        host.get_class_by_type_id(self.inner.as_ref().type_id())
    }

    /// Get the canonical name of this instance.
    ///
    /// The canonical name is the registered name on host *if* if it registered.
    /// Otherwise, the debug name is returned.
    pub fn name<'a>(&self, host: &'a Host) -> &'a str {
        self.class(host).unwrap().name.as_ref()
    }

    /// Lookup an attribute on the instance via the registered `Class`
    pub fn get_attr(&self, name: &str, class: &Class) -> Result<MetacallValue> {
        let attr = class.attributes.get(name).unwrap().clone();
        attr.invoke(self)
    }
    pub fn set_attr(&mut self, name: &str, value: MetacallValue, class: &Class) {
        let attr = class.attr_setters.get(name).unwrap().clone();
        attr.invoke(value, self)
    }
    /// Attempt to downcast the inner type of the instance to a reference to the type `T`
    /// This should be the _only_ place using downcast to avoid mistakes.
    ///
    /// # Arguments
    ///
    /// * `host`: Pass host if possible to improve error handling.
    // impl Foo {
    //     pub fn get_items(&self) -> impl Deref<Target = Vec<i32>> + '_ {
    //         Ref::map(self.interior.borrow(), |mi| &mi.vec)
    //     }
    // }
    pub fn borrow(&self) -> Ref<dyn std::any::Any + Send + Sync> {
        self.inner.as_ref().borrow()
        // let r = self.inner.as_ref().borrow();
        // Ref::map(self.inner.as_ref().borrow(), |re| &re)
    }

    pub fn borrow_mut(&self) -> RefMut<dyn std::any::Any + Send + Sync> {
        self.inner.as_ref().borrow_mut()
        // let r = self.inner.as_ref().borrow();
        // Ref::map(self.inner.as_ref().borrow(), |re| &re)
    }

    // pub fn downcast_mut<T: 'static>(
    //     &self,
    //     host: Option<&Host>,
    // ) -> Ref<dyn std::any::Any + Send + Sync> {
    //     // let receiver = self.inner.as_ref().borrow_mut().downcast_mut().unwrap();
    //     // Ok(receiver)
    //     // let r = self.inner.as_ref().borrow();
    //     // Ref::map(r, |re| &Ok(re.downcast_mut().unwrap()))
    //     self.inner.as_ref().borrow()
    // }

    pub fn call(
        &self,
        name: &str,
        args: Vec<MetacallValue>,
        class: &Class,
    ) -> Result<MetacallValue> {
        let method = class.get_method(name).unwrap();
        method.invoke(self, args)
    }
}

pub trait Function<Args = ()>: Send + Sync + 'static {
    type Result;

    fn invoke(&self, args: Args) -> Self::Result;
}

/// Similar to a `Function` but also takes an explicit `receiver`
/// parameter than is the first argument of the call (i.e. the `self` param);
pub trait Method<Receiver, Args = ()>: Send + Sync + 'static {
    type Result;

    fn invoke(&self, receiver: &Receiver, args: Args) -> Self::Result;
}

macro_rules! tuple_impls {
    ( $( $name:ident )* ) => {
        impl<Fun, Res, $($name),*> Function<($($name,)*)> for Fun
        where
            Fun: Fn($($name),*) -> Res + Send + Sync + 'static
        {
            type Result = Res;

            fn invoke(&self, args: ($($name,)*)) -> Self::Result {
                #[allow(non_snake_case)]
                let ($($name,)*) = args;
                (self)($($name,)*)
            }
        }

        impl<Fun, Res, Receiver, $($name),*> Method<Receiver, ($($name,)*)> for Fun
        where
            Fun: Fn(&Receiver, $($name),*) -> Res + Send + Sync + 'static,
        {
            type Result = Res;

            fn invoke(&self, receiver: &Receiver, args: ($($name,)*)) -> Self::Result {
                #[allow(non_snake_case)]
                let ($($name,)*) = args;
                (self)(receiver, $($name,)*)
            }
        }
    };
}

tuple_impls! {}
tuple_impls! { A }
tuple_impls! { A B }
tuple_impls! { A B C }
tuple_impls! { A B C D }
tuple_impls! { A B C D E }
tuple_impls! { A B C D E F }
tuple_impls! { A B C D E F G }
tuple_impls! { A B C D E F G H }
tuple_impls! { A B C D E F G H I }
tuple_impls! { A B C D E F G H I J }
tuple_impls! { A B C D E F G H I J K }
tuple_impls! { A B C D E F G H I J K L }
tuple_impls! { A B C D E F G H I J K L M }
tuple_impls! { A B C D E F G H I J K L M N }
tuple_impls! { A B C D E F G H I J K L M N O }
tuple_impls! { A B C D E F G H I J K L M N O P }

fn join<A, B>(left: Result<A>, right: Result<B>) -> Result<(A, B)> {
    left.and_then(|l| right.map(|r| (l, r)))
}

type TypeErasedFunction<R> = Arc<dyn Fn(Vec<MetacallValue>) -> Result<R> + Send + Sync>;
type TypeErasedMethod<R> = Arc<dyn Fn(&Instance, Vec<MetacallValue>) -> Result<R> + Send + Sync>;

#[derive(Clone)]
pub struct Constructor(TypeErasedFunction<Instance>);

impl Constructor {
    pub fn new<Args, F>(f: F) -> Self
    where
        Args: FromMetaList,
        F: Function<Args>,
        F::Result: Send + Sync + 'static,
    {
        Constructor(Arc::new(move |args: Vec<MetacallValue>| {
            Args::from_meta_list(&args).map(|args| Instance::new(f.invoke(args)))
        }))
    }

    pub fn invoke(&self, args: Vec<MetacallValue>) -> Result<Instance> {
        self.0(args)
    }
}

#[derive(Clone)]
pub struct InstanceMethod(TypeErasedMethod<MetacallValue>);

impl InstanceMethod {
    pub fn new<T, F, Args>(f: F) -> Self
    where
        Args: FromMetaList,
        F: Method<T, Args>,
        F::Result: ToMetaResult,
        T: 'static,
    {
        Self(Arc::new(
            move |receiver: &Instance, args: Vec<MetacallValue>| {
                let borrowed_receiver = receiver.borrow();
                let receiver = Ok(borrowed_receiver.downcast_ref::<T>().unwrap());

                let args = Args::from_meta_list(&args);

                join(receiver, args)
                    .and_then(|(receiver, args)| f.invoke(receiver, args).to_meta_result())
            },
        ))
    }

    pub fn invoke(&self, receiver: &Instance, args: Vec<MetacallValue>) -> Result<MetacallValue> {
        self.0(receiver, args)
    }
}

#[derive(Clone)]
pub struct ClassMethod(TypeErasedFunction<MetacallValue>);

impl ClassMethod {
    pub fn new<F, Args>(f: F) -> Self
    where
        Args: FromMetaList + std::fmt::Debug,
        F: Function<Args>,
        F::Result: ToMetaResult + std::fmt::Debug,
    {
        Self(Arc::new(move |args: Vec<MetacallValue>| {
            Args::from_meta_list(&args).and_then(|args| {
                let res = f.invoke(args);
                res.to_meta_result()
            })
        }))
    }

    pub fn invoke(&self, args: Vec<MetacallValue>) -> Result<MetacallValue> {
        self.0(args)
    }
}

#[derive(Clone)]
pub struct NormalFunction(TypeErasedFunction<MetacallValue>);

impl NormalFunction {
    pub fn new<F, Args>(f: F) -> Self
    where
        Args: FromMetaList + std::fmt::Debug,
        F: Function<Args>,
        F::Result: ToMetaResult + std::fmt::Debug,
    {
        Self(Arc::new(move |args: Vec<MetacallValue>| {
            Args::from_meta_list(&args).and_then(|args| {
                let res = f.invoke(args);
                res.to_meta_result()
            })
        }))
    }

    pub fn invoke(&self, args: Vec<MetacallValue>) -> Result<MetacallValue> {
        self.0(args)
    }
}

pub trait ToMetaResult {
    fn to_meta_result(self) -> Result<MetacallValue>;
}

impl ToMetaResult for u32 {
    fn to_meta_result(self) -> Result<MetacallValue> {
        Ok(self as MetacallValue)
    }
}

impl ToMetaResult for i32 {
    fn to_meta_result(self) -> Result<MetacallValue> {
        Ok(self as MetacallValue)
    }
}

pub trait FromMetaList {
    fn from_meta_list(values: &[MetacallValue]) -> Result<Self>
    where
        Self: Sized;
}
pub trait FromMeta: Clone {
    fn from_meta(val: MetacallValue) -> Result<Self>;
}

impl FromMeta for MetacallValue {
    fn from_meta(val: MetacallValue) -> Result<Self> {
        Ok(val)
    }
}

impl FromMeta for u32 {
    fn from_meta(val: MetacallValue) -> Result<Self> {
        Ok(val as u32)
    }
}
impl FromMeta for i32 {
    fn from_meta(val: MetacallValue) -> Result<Self> {
        Ok(val as i32)
    }
}

// impl FromMetaList for (MetacallValue,) {
//     fn from_meta_list(values: &[MetacallValue]) -> Result<Self, anyhow::Error> {
//         Ok((values[0],))
//     }
// }

#[allow(unused)]
impl FromMetaList for () {
    fn from_meta_list(values: &[MetacallValue]) -> Result<Self> {
        let mut iter = values.iter();
        let result = Ok(());
        result
    }
}
#[allow(unused)]
impl<TupleElement0: FromMeta> FromMetaList for (TupleElement0,) {
    fn from_meta_list(values: &[MetacallValue]) -> Result<Self> {
        let mut iter = values.iter();
        let result = Ok((TupleElement0::from_meta(iter.next().unwrap().clone())?,));
        result
    }
}
#[allow(unused)]
impl<TupleElement0: FromMeta, TupleElement1: FromMeta> FromMetaList
    for (TupleElement0, TupleElement1)
{
    fn from_meta_list(values: &[MetacallValue]) -> Result<Self> {
        let mut iter = values.iter();
        let result = Ok((
            TupleElement0::from_meta(iter.next().unwrap().clone())?,
            TupleElement1::from_meta(iter.next().unwrap().clone())?,
        ));
        result
    }
}
#[allow(unused)]
impl<TupleElement0: FromMeta, TupleElement1: FromMeta, TupleElement2: FromMeta> FromMetaList
    for (TupleElement0, TupleElement1, TupleElement2)
{
    fn from_meta_list(values: &[MetacallValue]) -> Result<Self> {
        let mut iter = values.iter();
        let result = Ok((
            TupleElement0::from_meta(iter.next().unwrap().clone())?,
            TupleElement1::from_meta(iter.next().unwrap().clone())?,
            TupleElement2::from_meta(iter.next().unwrap().clone())?,
        ));
        result
    }
}
#[allow(unused)]
impl<
        TupleElement0: FromMeta,
        TupleElement1: FromMeta,
        TupleElement2: FromMeta,
        TupleElement3: FromMeta,
    > FromMetaList for (TupleElement0, TupleElement1, TupleElement2, TupleElement3)
{
    fn from_meta_list(values: &[MetacallValue]) -> Result<Self> {
        let mut iter = values.iter();
        let result = Ok((
            TupleElement0::from_meta(iter.next().unwrap().clone())?,
            TupleElement1::from_meta(iter.next().unwrap().clone())?,
            TupleElement2::from_meta(iter.next().unwrap().clone())?,
            TupleElement3::from_meta(iter.next().unwrap().clone())?,
        ));
        result
    }
}
#[allow(unused)]
impl<
        TupleElement0: FromMeta,
        TupleElement1: FromMeta,
        TupleElement2: FromMeta,
        TupleElement3: FromMeta,
        TupleElement4: FromMeta,
    > FromMetaList
    for (
        TupleElement0,
        TupleElement1,
        TupleElement2,
        TupleElement3,
        TupleElement4,
    )
{
    fn from_meta_list(values: &[MetacallValue]) -> Result<Self> {
        let mut iter = values.iter();
        let result = Ok((
            TupleElement0::from_meta(iter.next().unwrap().clone())?,
            TupleElement1::from_meta(iter.next().unwrap().clone())?,
            TupleElement2::from_meta(iter.next().unwrap().clone())?,
            TupleElement3::from_meta(iter.next().unwrap().clone())?,
            TupleElement4::from_meta(iter.next().unwrap().clone())?,
        ));
        result
    }
}
#[allow(unused)]
impl<
        TupleElement0: FromMeta,
        TupleElement1: FromMeta,
        TupleElement2: FromMeta,
        TupleElement3: FromMeta,
        TupleElement4: FromMeta,
        TupleElement5: FromMeta,
    > FromMetaList
    for (
        TupleElement0,
        TupleElement1,
        TupleElement2,
        TupleElement3,
        TupleElement4,
        TupleElement5,
    )
{
    fn from_meta_list(values: &[MetacallValue]) -> Result<Self> {
        let mut iter = values.iter();
        let result = Ok((
            TupleElement0::from_meta(iter.next().unwrap().clone())?,
            TupleElement1::from_meta(iter.next().unwrap().clone())?,
            TupleElement2::from_meta(iter.next().unwrap().clone())?,
            TupleElement3::from_meta(iter.next().unwrap().clone())?,
            TupleElement4::from_meta(iter.next().unwrap().clone())?,
            TupleElement5::from_meta(iter.next().unwrap().clone())?,
        ));
        result
    }
}
#[allow(unused)]
impl<
        TupleElement0: FromMeta,
        TupleElement1: FromMeta,
        TupleElement2: FromMeta,
        TupleElement3: FromMeta,
        TupleElement4: FromMeta,
        TupleElement5: FromMeta,
        TupleElement6: FromMeta,
    > FromMetaList
    for (
        TupleElement0,
        TupleElement1,
        TupleElement2,
        TupleElement3,
        TupleElement4,
        TupleElement5,
        TupleElement6,
    )
{
    fn from_meta_list(values: &[MetacallValue]) -> Result<Self> {
        let mut iter = values.iter();
        let result = Ok((
            TupleElement0::from_meta(iter.next().unwrap().clone())?,
            TupleElement1::from_meta(iter.next().unwrap().clone())?,
            TupleElement2::from_meta(iter.next().unwrap().clone())?,
            TupleElement3::from_meta(iter.next().unwrap().clone())?,
            TupleElement4::from_meta(iter.next().unwrap().clone())?,
            TupleElement5::from_meta(iter.next().unwrap().clone())?,
            TupleElement6::from_meta(iter.next().unwrap().clone())?,
        ));
        result
    }
}
#[allow(unused)]
impl<
        TupleElement0: FromMeta,
        TupleElement1: FromMeta,
        TupleElement2: FromMeta,
        TupleElement3: FromMeta,
        TupleElement4: FromMeta,
        TupleElement5: FromMeta,
        TupleElement6: FromMeta,
        TupleElement7: FromMeta,
    > FromMetaList
    for (
        TupleElement0,
        TupleElement1,
        TupleElement2,
        TupleElement3,
        TupleElement4,
        TupleElement5,
        TupleElement6,
        TupleElement7,
    )
{
    fn from_meta_list(values: &[MetacallValue]) -> Result<Self> {
        let mut iter = values.iter();
        let result = Ok((
            TupleElement0::from_meta(iter.next().unwrap().clone())?,
            TupleElement1::from_meta(iter.next().unwrap().clone())?,
            TupleElement2::from_meta(iter.next().unwrap().clone())?,
            TupleElement3::from_meta(iter.next().unwrap().clone())?,
            TupleElement4::from_meta(iter.next().unwrap().clone())?,
            TupleElement5::from_meta(iter.next().unwrap().clone())?,
            TupleElement6::from_meta(iter.next().unwrap().clone())?,
            TupleElement7::from_meta(iter.next().unwrap().clone())?,
        ));
        result
    }
}
#[allow(unused)]
impl<
        TupleElement0: FromMeta,
        TupleElement1: FromMeta,
        TupleElement2: FromMeta,
        TupleElement3: FromMeta,
        TupleElement4: FromMeta,
        TupleElement5: FromMeta,
        TupleElement6: FromMeta,
        TupleElement7: FromMeta,
        TupleElement8: FromMeta,
    > FromMetaList
    for (
        TupleElement0,
        TupleElement1,
        TupleElement2,
        TupleElement3,
        TupleElement4,
        TupleElement5,
        TupleElement6,
        TupleElement7,
        TupleElement8,
    )
{
    fn from_meta_list(values: &[MetacallValue]) -> Result<Self> {
        let mut iter = values.iter();
        let result = Ok((
            TupleElement0::from_meta(iter.next().unwrap().clone())?,
            TupleElement1::from_meta(iter.next().unwrap().clone())?,
            TupleElement2::from_meta(iter.next().unwrap().clone())?,
            TupleElement3::from_meta(iter.next().unwrap().clone())?,
            TupleElement4::from_meta(iter.next().unwrap().clone())?,
            TupleElement5::from_meta(iter.next().unwrap().clone())?,
            TupleElement6::from_meta(iter.next().unwrap().clone())?,
            TupleElement7::from_meta(iter.next().unwrap().clone())?,
            TupleElement8::from_meta(iter.next().unwrap().clone())?,
        ));
        result
    }
}
#[allow(unused)]
impl<
        TupleElement0: FromMeta,
        TupleElement1: FromMeta,
        TupleElement2: FromMeta,
        TupleElement3: FromMeta,
        TupleElement4: FromMeta,
        TupleElement5: FromMeta,
        TupleElement6: FromMeta,
        TupleElement7: FromMeta,
        TupleElement8: FromMeta,
        TupleElement9: FromMeta,
    > FromMetaList
    for (
        TupleElement0,
        TupleElement1,
        TupleElement2,
        TupleElement3,
        TupleElement4,
        TupleElement5,
        TupleElement6,
        TupleElement7,
        TupleElement8,
        TupleElement9,
    )
{
    fn from_meta_list(values: &[MetacallValue]) -> Result<Self> {
        let mut iter = values.iter();
        let result = Ok((
            TupleElement0::from_meta(iter.next().unwrap().clone())?,
            TupleElement1::from_meta(iter.next().unwrap().clone())?,
            TupleElement2::from_meta(iter.next().unwrap().clone())?,
            TupleElement3::from_meta(iter.next().unwrap().clone())?,
            TupleElement4::from_meta(iter.next().unwrap().clone())?,
            TupleElement5::from_meta(iter.next().unwrap().clone())?,
            TupleElement6::from_meta(iter.next().unwrap().clone())?,
            TupleElement7::from_meta(iter.next().unwrap().clone())?,
            TupleElement8::from_meta(iter.next().unwrap().clone())?,
            TupleElement9::from_meta(iter.next().unwrap().clone())?,
        ));
        result
    }
}
#[allow(unused)]
impl<
        TupleElement0: FromMeta,
        TupleElement1: FromMeta,
        TupleElement2: FromMeta,
        TupleElement3: FromMeta,
        TupleElement4: FromMeta,
        TupleElement5: FromMeta,
        TupleElement6: FromMeta,
        TupleElement7: FromMeta,
        TupleElement8: FromMeta,
        TupleElement9: FromMeta,
        TupleElement10: FromMeta,
    > FromMetaList
    for (
        TupleElement0,
        TupleElement1,
        TupleElement2,
        TupleElement3,
        TupleElement4,
        TupleElement5,
        TupleElement6,
        TupleElement7,
        TupleElement8,
        TupleElement9,
        TupleElement10,
    )
{
    fn from_meta_list(values: &[MetacallValue]) -> Result<Self> {
        let mut iter = values.iter();
        let result = Ok((
            TupleElement0::from_meta(iter.next().unwrap().clone())?,
            TupleElement1::from_meta(iter.next().unwrap().clone())?,
            TupleElement2::from_meta(iter.next().unwrap().clone())?,
            TupleElement3::from_meta(iter.next().unwrap().clone())?,
            TupleElement4::from_meta(iter.next().unwrap().clone())?,
            TupleElement5::from_meta(iter.next().unwrap().clone())?,
            TupleElement6::from_meta(iter.next().unwrap().clone())?,
            TupleElement7::from_meta(iter.next().unwrap().clone())?,
            TupleElement8::from_meta(iter.next().unwrap().clone())?,
            TupleElement9::from_meta(iter.next().unwrap().clone())?,
            TupleElement10::from_meta(iter.next().unwrap().clone())?,
        ));
        result
    }
}
#[allow(unused)]
impl<
        TupleElement0: FromMeta,
        TupleElement1: FromMeta,
        TupleElement2: FromMeta,
        TupleElement3: FromMeta,
        TupleElement4: FromMeta,
        TupleElement5: FromMeta,
        TupleElement6: FromMeta,
        TupleElement7: FromMeta,
        TupleElement8: FromMeta,
        TupleElement9: FromMeta,
        TupleElement10: FromMeta,
        TupleElement11: FromMeta,
    > FromMetaList
    for (
        TupleElement0,
        TupleElement1,
        TupleElement2,
        TupleElement3,
        TupleElement4,
        TupleElement5,
        TupleElement6,
        TupleElement7,
        TupleElement8,
        TupleElement9,
        TupleElement10,
        TupleElement11,
    )
{
    fn from_meta_list(values: &[MetacallValue]) -> Result<Self> {
        let mut iter = values.iter();
        let result = Ok((
            TupleElement0::from_meta(iter.next().unwrap().clone())?,
            TupleElement1::from_meta(iter.next().unwrap().clone())?,
            TupleElement2::from_meta(iter.next().unwrap().clone())?,
            TupleElement3::from_meta(iter.next().unwrap().clone())?,
            TupleElement4::from_meta(iter.next().unwrap().clone())?,
            TupleElement5::from_meta(iter.next().unwrap().clone())?,
            TupleElement6::from_meta(iter.next().unwrap().clone())?,
            TupleElement7::from_meta(iter.next().unwrap().clone())?,
            TupleElement8::from_meta(iter.next().unwrap().clone())?,
            TupleElement9::from_meta(iter.next().unwrap().clone())?,
            TupleElement10::from_meta(iter.next().unwrap().clone())?,
            TupleElement11::from_meta(iter.next().unwrap().clone())?,
        ));
        result
    }
}
#[allow(unused)]
impl<
        TupleElement0: FromMeta,
        TupleElement1: FromMeta,
        TupleElement2: FromMeta,
        TupleElement3: FromMeta,
        TupleElement4: FromMeta,
        TupleElement5: FromMeta,
        TupleElement6: FromMeta,
        TupleElement7: FromMeta,
        TupleElement8: FromMeta,
        TupleElement9: FromMeta,
        TupleElement10: FromMeta,
        TupleElement11: FromMeta,
        TupleElement12: FromMeta,
    > FromMetaList
    for (
        TupleElement0,
        TupleElement1,
        TupleElement2,
        TupleElement3,
        TupleElement4,
        TupleElement5,
        TupleElement6,
        TupleElement7,
        TupleElement8,
        TupleElement9,
        TupleElement10,
        TupleElement11,
        TupleElement12,
    )
{
    fn from_meta_list(values: &[MetacallValue]) -> Result<Self> {
        let mut iter = values.iter();
        let result = Ok((
            TupleElement0::from_meta(iter.next().unwrap().clone())?,
            TupleElement1::from_meta(iter.next().unwrap().clone())?,
            TupleElement2::from_meta(iter.next().unwrap().clone())?,
            TupleElement3::from_meta(iter.next().unwrap().clone())?,
            TupleElement4::from_meta(iter.next().unwrap().clone())?,
            TupleElement5::from_meta(iter.next().unwrap().clone())?,
            TupleElement6::from_meta(iter.next().unwrap().clone())?,
            TupleElement7::from_meta(iter.next().unwrap().clone())?,
            TupleElement8::from_meta(iter.next().unwrap().clone())?,
            TupleElement9::from_meta(iter.next().unwrap().clone())?,
            TupleElement10::from_meta(iter.next().unwrap().clone())?,
            TupleElement11::from_meta(iter.next().unwrap().clone())?,
            TupleElement12::from_meta(iter.next().unwrap().clone())?,
        ));
        result
    }
}
#[allow(unused)]
impl<
        TupleElement0: FromMeta,
        TupleElement1: FromMeta,
        TupleElement2: FromMeta,
        TupleElement3: FromMeta,
        TupleElement4: FromMeta,
        TupleElement5: FromMeta,
        TupleElement6: FromMeta,
        TupleElement7: FromMeta,
        TupleElement8: FromMeta,
        TupleElement9: FromMeta,
        TupleElement10: FromMeta,
        TupleElement11: FromMeta,
        TupleElement12: FromMeta,
        TupleElement13: FromMeta,
    > FromMetaList
    for (
        TupleElement0,
        TupleElement1,
        TupleElement2,
        TupleElement3,
        TupleElement4,
        TupleElement5,
        TupleElement6,
        TupleElement7,
        TupleElement8,
        TupleElement9,
        TupleElement10,
        TupleElement11,
        TupleElement12,
        TupleElement13,
    )
{
    fn from_meta_list(values: &[MetacallValue]) -> Result<Self> {
        let mut iter = values.iter();
        let result = Ok((
            TupleElement0::from_meta(iter.next().unwrap().clone())?,
            TupleElement1::from_meta(iter.next().unwrap().clone())?,
            TupleElement2::from_meta(iter.next().unwrap().clone())?,
            TupleElement3::from_meta(iter.next().unwrap().clone())?,
            TupleElement4::from_meta(iter.next().unwrap().clone())?,
            TupleElement5::from_meta(iter.next().unwrap().clone())?,
            TupleElement6::from_meta(iter.next().unwrap().clone())?,
            TupleElement7::from_meta(iter.next().unwrap().clone())?,
            TupleElement8::from_meta(iter.next().unwrap().clone())?,
            TupleElement9::from_meta(iter.next().unwrap().clone())?,
            TupleElement10::from_meta(iter.next().unwrap().clone())?,
            TupleElement11::from_meta(iter.next().unwrap().clone())?,
            TupleElement12::from_meta(iter.next().unwrap().clone())?,
            TupleElement13::from_meta(iter.next().unwrap().clone())?,
        ));
        result
    }
}
#[allow(unused)]
impl<
        TupleElement0: FromMeta,
        TupleElement1: FromMeta,
        TupleElement2: FromMeta,
        TupleElement3: FromMeta,
        TupleElement4: FromMeta,
        TupleElement5: FromMeta,
        TupleElement6: FromMeta,
        TupleElement7: FromMeta,
        TupleElement8: FromMeta,
        TupleElement9: FromMeta,
        TupleElement10: FromMeta,
        TupleElement11: FromMeta,
        TupleElement12: FromMeta,
        TupleElement13: FromMeta,
        TupleElement14: FromMeta,
    > FromMetaList
    for (
        TupleElement0,
        TupleElement1,
        TupleElement2,
        TupleElement3,
        TupleElement4,
        TupleElement5,
        TupleElement6,
        TupleElement7,
        TupleElement8,
        TupleElement9,
        TupleElement10,
        TupleElement11,
        TupleElement12,
        TupleElement13,
        TupleElement14,
    )
{
    fn from_meta_list(values: &[MetacallValue]) -> Result<Self> {
        let mut iter = values.iter();
        let result = Ok((
            TupleElement0::from_meta(iter.next().unwrap().clone())?,
            TupleElement1::from_meta(iter.next().unwrap().clone())?,
            TupleElement2::from_meta(iter.next().unwrap().clone())?,
            TupleElement3::from_meta(iter.next().unwrap().clone())?,
            TupleElement4::from_meta(iter.next().unwrap().clone())?,
            TupleElement5::from_meta(iter.next().unwrap().clone())?,
            TupleElement6::from_meta(iter.next().unwrap().clone())?,
            TupleElement7::from_meta(iter.next().unwrap().clone())?,
            TupleElement8::from_meta(iter.next().unwrap().clone())?,
            TupleElement9::from_meta(iter.next().unwrap().clone())?,
            TupleElement10::from_meta(iter.next().unwrap().clone())?,
            TupleElement11::from_meta(iter.next().unwrap().clone())?,
            TupleElement12::from_meta(iter.next().unwrap().clone())?,
            TupleElement13::from_meta(iter.next().unwrap().clone())?,
            TupleElement14::from_meta(iter.next().unwrap().clone())?,
        ));
        result
    }
}
#[allow(unused)]
impl<
        TupleElement0: FromMeta,
        TupleElement1: FromMeta,
        TupleElement2: FromMeta,
        TupleElement3: FromMeta,
        TupleElement4: FromMeta,
        TupleElement5: FromMeta,
        TupleElement6: FromMeta,
        TupleElement7: FromMeta,
        TupleElement8: FromMeta,
        TupleElement9: FromMeta,
        TupleElement10: FromMeta,
        TupleElement11: FromMeta,
        TupleElement12: FromMeta,
        TupleElement13: FromMeta,
        TupleElement14: FromMeta,
        TupleElement15: FromMeta,
    > FromMetaList
    for (
        TupleElement0,
        TupleElement1,
        TupleElement2,
        TupleElement3,
        TupleElement4,
        TupleElement5,
        TupleElement6,
        TupleElement7,
        TupleElement8,
        TupleElement9,
        TupleElement10,
        TupleElement11,
        TupleElement12,
        TupleElement13,
        TupleElement14,
        TupleElement15,
    )
{
    fn from_meta_list(values: &[MetacallValue]) -> Result<Self> {
        let mut iter = values.iter();
        let result = Ok((
            TupleElement0::from_meta(iter.next().unwrap().clone())?,
            TupleElement1::from_meta(iter.next().unwrap().clone())?,
            TupleElement2::from_meta(iter.next().unwrap().clone())?,
            TupleElement3::from_meta(iter.next().unwrap().clone())?,
            TupleElement4::from_meta(iter.next().unwrap().clone())?,
            TupleElement5::from_meta(iter.next().unwrap().clone())?,
            TupleElement6::from_meta(iter.next().unwrap().clone())?,
            TupleElement7::from_meta(iter.next().unwrap().clone())?,
            TupleElement8::from_meta(iter.next().unwrap().clone())?,
            TupleElement9::from_meta(iter.next().unwrap().clone())?,
            TupleElement10::from_meta(iter.next().unwrap().clone())?,
            TupleElement11::from_meta(iter.next().unwrap().clone())?,
            TupleElement12::from_meta(iter.next().unwrap().clone())?,
            TupleElement13::from_meta(iter.next().unwrap().clone())?,
            TupleElement14::from_meta(iter.next().unwrap().clone())?,
            TupleElement15::from_meta(iter.next().unwrap().clone())?,
        ));
        result
    }
}

// #[impl_for_tuples(16)]
// #[tuple_types_custom_trait_bound(FromMeta)]
// impl FromMetaList for Tuple {
//     fn from_meta_list(values: &[MetacallValue]) -> Result<Self> {
//         let mut iter = values.iter();
//         let result = Ok((for_tuples!(
//             #( Tuple::from_meta(iter.next().unwrap().clone())? ),*
//         )));
//         result
//     }
// }

#[derive(Clone)]
pub struct AttributeGetter(Arc<dyn Fn(&Instance) -> Result<MetacallValue> + Send + Sync>);
impl AttributeGetter {
    pub fn new<T, F, R>(f: F) -> Self
    where
        T: 'static,
        F: Fn(&T) -> R + Send + Sync + 'static,
        R: ToMetaResult,
    {
        Self(Arc::new(move |receiver| {
            let borrowed_receiver = receiver.borrow();
            let receiver = Ok(borrowed_receiver.downcast_ref::<T>().unwrap());
            receiver.map(&f).and_then(|v| v.to_meta_result())
        }))
    }

    pub fn invoke(&self, receiver: &Instance) -> Result<MetacallValue> {
        self.0(receiver)
    }
}

#[derive(Clone)]
pub struct AttributeSetter(Arc<dyn Fn(MetacallValue, &mut Instance)>);
impl AttributeSetter {
    pub fn new<T, F, Arg>(f: F) -> Self
    where
        T: 'static,
        Arg: FromMeta,
        F: Fn(Arg, &mut T) + 'static,
    {
        Self(Arc::new(move |value, receiver| {
            let mut borrowed_receiver = receiver.borrow_mut();
            let receiver = borrowed_receiver.downcast_mut::<T>().unwrap();
            f(FromMeta::from_meta(value).unwrap(), receiver)
        }))
    }

    pub fn invoke(&self, value: MetacallValue, receiver: &mut Instance) {
        self.0(value, receiver)
    }
}

/*
#[derive(Clone)]
pub struct ClassMethod(TypeErasedFunction<MetacallValue>);

impl ClassMethod {
    pub fn new<F, Args>(f: F) -> Self
    where
        Args: FromMetaList,
        F: Function<Args>,
        F::Result: ToMetaResult,
    {
        Self(Arc::new(move |args: Vec<MetacallValue>| {
            Args::from_meta_list(&args).and_then(|args| f.invoke(args).to_meta_result())
        }))
    }

    pub fn invoke(&self, args: Vec<MetacallValue>) -> Result<MetacallValue> {
        self.0(args)
    }
} */

fn metaclass() -> Class {
    Class::builder::<Class>()
        .name("metacall::host::Class")
        .build()
}

pub struct Host {
    /// Map from names to `Class`s
    pub classes: HashMap<String, Class>,

    /// Map of cached instances
    pub instances: HashMap<u64, Instance>,

    /// Map from type IDs, to class names
    /// This helps us go from a generic type `T` to the
    /// class name it is registered as
    pub class_names: HashMap<std::any::TypeId, String>,
}

impl Host {
    pub fn new() -> Self {
        let mut host = Self {
            class_names: HashMap::new(),
            classes: HashMap::new(),
            instances: HashMap::new(),
        };
        let type_class = metaclass();
        let name = type_class.name.clone();
        host.cache_class(type_class, name)
            .expect("could not register the metaclass");
        host
    }

    pub fn get_class(&self, name: &str) -> Result<&Class> {
        Ok(self.classes.get(name).unwrap())
    }

    pub fn get_class_by_type_id(&self, id: std::any::TypeId) -> Result<&Class> {
        let name = self.class_names.get(&id).unwrap();
        self.get_class(name)
    }

    pub fn make_instance(&mut self, name: &str, fields: Vec<MetacallValue>, id: u64) -> Result<()> {
        let class = self.get_class(name)?.clone();
        debug_assert!(self.instances.get(&id).is_none());
        let fields = fields;
        let instance = class.init(fields);
        self.cache_instance(instance, Some(id));
        Ok(())
    }

    pub fn get_instance(&self, id: u64) -> Result<&Instance> {
        Ok(self.instances.get(&id).unwrap())
    }

    pub fn cache_instance(&mut self, instance: Instance, id: Option<u64>) -> u64 {
        // Lookup the class for this instance
        let type_id = instance.type_id();
        let class = self.get_class_by_type_id(type_id);
        if class.is_err() {
            // if its not found, try and use the default class implementation
            // let default_class = DEFAULT_CLASSES.read().unwrap().get(&type_id).cloned();
            // if let Some(class) = default_class {
            //     let name = class.name.clone();
            //     let _ = self.cache_class(class, name);
            // }
            panic!("cannot find class");
        }

        let id = id.unwrap();
        self.instances.insert(id, instance);
        id
    }

    pub fn cache_class(&mut self, class: Class, name: String) -> Result<String> {
        // Insert into default classes here so that we don't repeat this the first
        // time we see an instance.
        // DEFAULT_CLASSES
        //     .write()
        //     .unwrap()
        //     .entry(class.type_id)
        //     .or_insert_with(|| class.clone());

        self.class_names.insert(class.type_id, name.clone());
        self.classes.insert(name.clone(), class);
        Ok(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn is_working() -> Result<()> {
        let mut host = Host::new();

        // callee side
        struct Foo {
            x: u32,
            y: i32,
        }

        impl Foo {
            fn new(x: u32, y: i32) -> Self {
                Self { x, y }
            }
            fn x_plus_y(&self, y: u32) -> u32 {
                self.x + y
            }
            fn get_price(&self) -> i32 {
                self.y
            }
            fn set_x(&mut self, x: u32) {
                self.x = x;
            }
            fn get_number() -> u32 {
                123
            }
        }

        fn test_func(x: i32) -> i32 {
            x + 10
        }

        let nf = NormalFunction::new(test_func);
        let nres = nf.invoke(vec![32 as MetacallValue]).unwrap();
        println!("nres: {}", nres as i32);
        // register the class
        let foo_class = Class::builder::<Foo>()
            .set_constructor(Foo::new)
            .add_attribute_getter("x", |f| f.x)
            .add_attribute_setter("y", |val, f| f.y = val)
            .add_attribute_getter("y", |f| f.y)
            .add_method("x_plus_y", Foo::x_plus_y)
            .add_method("get_price", Foo::get_price)
            .add_class_method("get_number", Foo::get_number)
            .build();
        // this should call register class in caller side.
        // cache_class(*mut Class, String);
        // ----

        //caller side
        // host.cache_class(class, "Foo".to_string())?;
        // host.make_instance(
        //     "Foo",
        //     vec![32 as MetacallValue, -12 as i32 as MetacallValue],
        //     1,
        // )?;
        // ----

        // let foo_class = host.get_class("Foo")?;
        let mut foo_instance =
            foo_class.init(vec![32 as MetacallValue, -12 as i32 as MetacallValue]);
        let x = foo_instance.get_attr("x", &foo_class)?;
        let y = foo_instance.get_attr("y", &foo_class)?;
        println!("{} : {}", x as u32, y as i32);
        foo_instance.set_attr("y", 100 as MetacallValue, &foo_class);
        // let res = foo_instance.call("x_plus_y", vec![10 as MetacallValue], &host)?;
        let res = foo_instance.call("get_price", vec![], &foo_class)?;
        println!("{} : ", res as i32);
        // let y = foo_instance.get_attr("y", &foo_class)?;
        // // println!("{} : {}", x as u32, y as i32);
        // let num = foo_class.call("get_number", vec![])?;
        // assert_eq!(123, num as u32);
        // assert_eq!(32, x as u32);
        // assert_eq!(100, y as i32);
        // // assert_eq!(42, res as u32);
        // println!("get_number: {}", num as u32);
        // println!("{} : {} : {}", x as u32, y as i32, res as u32);

        Ok(())
    }
}
