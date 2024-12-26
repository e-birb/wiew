use std::{any::{Any, TypeId}, collections::HashMap, ops::Deref, sync::{atomic::AtomicBool, Arc}};

//use type_map::TypeMap;

use crate::{RenderContext, ResId};

pub struct ResourceRegistry {
    id_maps: HashMap<ResId, ResourceHold>,
    singletons: HashMap<TypeId, ResourceHold>,
}

struct ResourceHold {
    used: AtomicBool,
    type_name: &'static str,
    resource: Arc<dyn Any + Send + Sync>,
}

impl ResourceRegistry {
    pub fn new() -> Self {
        Self {
            id_maps: HashMap::new(),
            singletons: HashMap::new(),
        }
    }

    pub fn clean(&mut self) {
        // clear all unused resources and set used to false
        self.id_maps = self.id_maps
            .drain()
            .filter_map(|(id, h)| {
                if !h.used.load(std::sync::atomic::Ordering::Relaxed) {
                    return None;
                }
                h.used.store(false, std::sync::atomic::Ordering::Relaxed);

                Some((id, h))
            })
            .collect();

        self.singletons = self.singletons
            .drain()
            .filter_map(|(id, h)| {
                if !h.used.load(std::sync::atomic::Ordering::Relaxed) {
                    return None;
                }
                h.used.store(false, std::sync::atomic::Ordering::Relaxed);

                Some((id, h))
            })
            .collect();
    }

    /// Insert and already created resource
    pub fn insert<T: 'static + Send + Sync>(&mut self, resource: Res<T>, value: T) -> Arc<T> {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();

        let value = Arc::new(value);

        let old = self.id_maps.insert(resource.id().clone(), ResourceHold {
            used: AtomicBool::new(true), // if just created, it's already used
            type_name,
            resource: value.clone(),
        });

        if let Some(old) = old {
            log::debug!("Resource with id {:?} already exists, replacing", resource.id());
            if old.resource.deref().type_id() != type_id {
                log::error!("Resource with id {:?} (now {type_name}) already exists, but has different type ({})", resource.id(), old.type_name);
            }
        }

        value
    }

    pub fn by_id<T: 'static + Send + Sync>(&self, id: &ResId) -> Option<Arc<T>> {
        self.id_maps.get(id).map(|h| {
            h.used.store(true, std::sync::atomic::Ordering::Relaxed);
            Arc::downcast(h.resource.clone()).expect(&format!("Resource type mismatch for id {id}"))
        })
    }

    pub fn get_singleton<S: SingletonResource>(&mut self) -> Option<Arc<S>> {
        let type_id = TypeId::of::<S>();
        let type_name = std::any::type_name::<S>();

        let hold = self.singletons.get(&type_id); // TODO use `HashMap::entry` API

        if let Some(hold) = hold {
            hold.used.store(true, std::sync::atomic::Ordering::Relaxed);
        };

        hold.map(|h| {
            Arc::downcast(h.resource.clone()).expect(&format!("Resource type mismatch for singleton {type_name}"))
        })
    }

    pub fn insert_singleton<S: SingletonResource>(&mut self, value: S) -> Arc<S> {
        let type_id = TypeId::of::<S>();
        let type_name = std::any::type_name::<S>();

        let value = Arc::new(value);

        let old = self.singletons.insert(type_id, ResourceHold {
            used: AtomicBool::new(true), // if just created, it's already used
            type_name,
            resource: value.clone(),
        });

        if let Some(old) = old {
            log::debug!("Singleton resource with type {type_name} already exists, replacing");

            debug_assert_eq!(old.resource.deref().type_id(), type_id);

            if old.resource.deref().type_id() != type_id {
                log::error!("Singleton resource with type {type_name} already exists, but has different type ({})", old.type_name);
            }
        }

        value
    }

    // TODO a method that combines get_singleton and insert_singleton!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
}

pub struct Res<T>(Arc<ResourceInner<T>>);

impl<T> Clone for Res<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

struct ResourceInner<T> { // TODO maybe remove this struct
    id: ResId,
    builder: Box<dyn ResourceBuilder<Resource = T> + Send + Sync>, // TODO maybe allow some way to load async resources
}

impl<T> Res<T> {
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