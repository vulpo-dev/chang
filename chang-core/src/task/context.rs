use std::any::{Any, TypeId};
use std::collections::HashMap;

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
#[derive(Clone)]
pub struct Context {
    pub map: HashMap<TypeId, Box<dyn AnyClone + Send + Sync>>,
}

impl Context {
    pub fn new() -> Context {
        Context {
            map: HashMap::new(),
        }
    }

    pub fn put<T: 'static + Send + Sync + Clone>(&mut self, t: T) {
        self.map.insert(TypeId::of::<T>(), Box::new(t));
    }

    pub fn get<T: 'static + Send + Sync>(&self) -> Option<&T> {
        self.map
            .get(&TypeId::of::<T>())
            .and_then(|value| value.as_any().downcast_ref())
    }
}

impl From<&Context> for Context {
    fn from(ctx: &Context) -> Self {
        let mut map = HashMap::new();
        map.clone_from(&ctx.map);
        Context { map }
    }
}

#[derive(Clone)]
pub struct CurrentTask(pub serde_json::Value);
