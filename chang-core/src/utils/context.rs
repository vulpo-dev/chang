use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

pub trait AnyClone: Any {
    fn clone_box(&self) -> Box<dyn AnyClone + Send + Sync>;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

impl<T: Clone + Send + Sync + 'static> AnyClone for T {
    fn clone_box(&self) -> Box<dyn AnyClone + Send + Sync> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

impl Clone for Box<dyn AnyClone + Send + Sync> {
    fn clone(&self) -> Self {
        (**self).clone_box()
    }
}

type AnyMap = HashMap<TypeId, Box<dyn AnyClone + Send + Sync>, BuildHasherDefault<IdHasher>>;

#[derive(Default)]
pub struct IdHasher(u64);

impl Hasher for IdHasher {
    fn write(&mut self, _: &[u8]) {
        unreachable!("TypeId calls write_u64");
    }

    #[inline]
    fn write_u64(&mut self, id: u64) {
        self.0 = id;
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}

#[derive(Clone)]
pub struct Context {
    pub map: AnyMap,
}

impl Context {
    pub fn new() -> Context {
        Context {
            map: HashMap::default(),
        }
    }

    pub fn put<T: 'static + Send + Sync + Clone>(&mut self, t: T) {
        self.map.insert(TypeId::of::<T>(), Box::new(t));
    }

    pub fn get<T: 'static + Send + Sync + Clone>(&self) -> Option<&T> {
        self.map
            .get(&TypeId::of::<T>())
            .and_then(|boxed| (&**boxed).as_any().downcast_ref())
    }

    pub fn keys<'a>(&'a self) -> Vec<&'a TypeId> {
        self.map.keys().collect::<Vec<_>>()
    }

    pub fn values<'a>(&'a self) -> Vec<&'a Box<dyn AnyClone + Send + Sync>> {
        self.map.values().collect::<Vec<_>>()
    }
}

impl From<&Context> for Context {
    fn from(ctx: &Context) -> Self {
        let mut map = HashMap::default();
        map.clone_from(&ctx.map);
        Context { map }
    }
}

#[derive(Clone, Debug)]
pub struct CurrentTask(pub serde_json::Value);
