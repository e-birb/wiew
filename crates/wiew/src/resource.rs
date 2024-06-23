use std::{collections::HashMap, sync::Arc};

//use type_map::TypeMap;

use type_map::concurrent::TypeMap;

use crate::{RenderContext, ResId};

pub struct ResourceRegistry {
    singletons: TypeMap,
    id_maps: TypeMap,
}

struct ResourceMap<T> {
    id_map: HashMap<ResId, ResourceHold<T>>,
}

struct ResourceHold<T> {
    /// Tells whether the resource is still alive or not
    life: std::sync::Weak<ResourceInner<T>>,
    resource: Arc<T>,
}

impl ResourceRegistry {
    pub fn new() -> Self {
        Self {
            id_maps: TypeMap::new(),
            singletons: TypeMap::new(),
        }
    }

    /// Insert and already created resource
    pub fn insert<T: 'static + Send + Sync>(&mut self, resource: Resource<T>, value: T) -> Arc<T> {
        let value = Arc::new(value);

        let map = self.id_maps.entry::<ResourceMap<T>>().or_insert_with(|| ResourceMap {
            id_map: HashMap::new(),
        });

        map.id_map.insert(resource.id().clone(), ResourceHold {
            life: Arc::downgrade(&resource.0),
            resource: value.clone(),
        });

        value
    }

    pub fn by_id<T: 'static>(&self, id: &ResId) -> Option<&Arc<T>> {
        self.id_maps.get::<ResourceMap<T>>().and_then(|map| {
            map.id_map.get(id).map(|r| &r.resource)
        })
    }

    pub fn get_singleton<S: SingletonResource>(&mut self) -> Option<&Arc<S>> {
        self.singletons.get::<Arc<S>>()
    }

    pub fn insert_singleton<S: SingletonResource>(&mut self, value: S) -> Arc<S> {
        let value = Arc::new(value);
        self.singletons.insert(value.clone());
        value
    }
}

pub struct Resource<T>(Arc<ResourceInner<T>>);

impl<T> Clone for Resource<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

struct ResourceInner<T> { // TODO maybe remove this struct
    id: ResId,
    builder: Box<dyn ResourceBuilder<Resource = T> + Send + Sync>, // TODO maybe allow some way to load async resources
}

impl<T> Resource<T> {
    pub fn new(builder: impl ResourceBuilder<Resource = T> + 'static + Send + Sync) -> Self {
        Self(Arc::new(ResourceInner {
            id: ResId::new(),
            builder: Box::new(builder),
        }))
    }

    pub fn id(&self) -> &ResId {
        &self.0.id
    }

    pub fn builder(&self) -> &dyn ResourceBuilder<Resource = T> {
        &*self.0.builder
    }
}

pub trait ResourceBuilder {
    type Resource;

    // Note: use interior mutability if necessary
    fn build(&self, ctx: &mut RenderContext) -> Self::Resource;
}

impl<F, R> ResourceBuilder for F
where
    F: Fn(&mut RenderContext) -> R,
{
    type Resource = R;

    fn build(&self, ctx: &mut RenderContext) -> Self::Resource {
        self(ctx)
    }
}

pub trait SingletonResource: 'static + Send + Sync {
    fn init(ctx: &mut RenderContext) -> Self;
}