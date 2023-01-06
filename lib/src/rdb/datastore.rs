use std::collections::{HashMap, HashSet};
use std::i32;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::u64;
use std::usize;

use super::managers::*;
use crate::errors::{Error, Result};
use crate::util::next_uuid;
use crate::{
    BulkInsertItem, Datastore, DynIter, Edge, EdgeDirection, Identifier, Json, Query, QueryOutputValue, Transaction,
    Vertex,
};

use rocksdb::{DBCompactionStyle, Options, WriteBatch, DB};
use uuid::Uuid;

const CF_NAMES: [&str; 8] = [
    "vertices:v2",
    "edge_ranges:v2",
    "reversed_edge_ranges:v2",
    "vertex_properties:v2",
    "edge_properties:v2",
    "vertex_property_values:v2",
    "edge_property_values:v2",
    "metadata:v2",
];

fn get_options(max_open_files: Option<i32>) -> Options {
    // Current tuning based off of the total ordered example, flash
    // storage example on
    // https://github.com/facebook/rocksdb/wiki/RocksDB-Tuning-Guide
    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.set_compaction_style(DBCompactionStyle::Level);
    opts.set_write_buffer_size(67_108_864); // 64mb
    opts.set_max_write_buffer_number(3);
    opts.set_target_file_size_base(67_108_864); // 64mb
    opts.set_level_zero_file_num_compaction_trigger(8);
    opts.set_level_zero_slowdown_writes_trigger(17);
    opts.set_level_zero_stop_writes_trigger(24);
    opts.set_num_levels(4);
    opts.set_max_bytes_for_level_base(536_870_912); // 512mb
    opts.set_max_bytes_for_level_multiplier(8.0);

    if let Some(max_open_files) = max_open_files {
        opts.set_max_open_files(max_open_files);
    }

    opts
}

pub struct RocksdbTransaction<'a> {
    db: &'a DB,
    indexed_properties: Arc<RwLock<HashSet<Identifier>>>,
    vertex_manager: VertexManager<'a>,
    edge_manager: EdgeManager<'a>,
    edge_range_manager: EdgeRangeManager<'a>,
    reversed_edge_range_manager: EdgeRangeManager<'a>,
    vertex_property_manager: VertexPropertyManager<'a>,
    edge_property_manager: EdgePropertyManager<'a>,
    vertex_property_value_manager: VertexPropertyValueManager<'a>,
    edge_property_value_manager: EdgePropertyValueManager<'a>,
    metadata_manager: MetadataManager<'a>,
}

impl<'a> RocksdbTransaction<'a> {
    fn guard_indexed_property(&self, property: &Identifier) -> Result<()> {
        if !self.indexed_properties.read().unwrap().contains(property) {
            Err(Error::NotIndexed)
        } else {
            Ok(())
        }
    }

    // TODO: return iterators w/ these
    fn vertices_from_property_value_iterator(
        &self,
        iter: impl Iterator<Item = Result<VertexPropertyValueKey>> + 'a,
    ) -> Result<Vec<Vertex>> {
        let mut vertices = Vec::new();
        for item in iter {
            let (_, _, id) = item?;
            if let Some(t) = self.vertex_manager.get(id)? {
                vertices.push(Vertex::with_id(id, t));
            }
        }
        Ok(vertices)
    }
}

impl<'a> Transaction<'a> for RocksdbTransaction<'a> {
    fn vertex_count(&self) -> u64 {
        let iter = self.vertex_manager.iterate_for_range(Uuid::default());
        iter.count() as u64
    }

    fn all_vertices(&'a self) -> Result<DynIter<'a, Vertex>> {
        let iter = self.vertex_manager.iterate_for_range(Uuid::default());
        Ok(Box::new(iter))
    }

    fn range_vertices(&'a self, offset: Uuid) -> Result<DynIter<'a, Vertex>> {
        let iter = self.vertex_manager.iterate_for_range(offset);
        Ok(Box::new(iter))
    }

    fn specific_vertices(&'a self, ids: Vec<Uuid>) -> Result<DynIter<'a, Vertex>> {
        let iter = ids.into_iter().filter_map(move |id| match self.vertex_manager.get(id) {
            Ok(Some(t)) => Some(Ok(Vertex::with_id(id, t))),
            Ok(None) => None,
            Err(err) => Some(Err(err)),
        });

        Ok(Box::new(iter))
    }

    fn vertex_ids_with_property(&'a self, name: &Identifier) -> Result<Option<DynIter<'a, Uuid>>> {
        if self.indexed_properties.read().unwrap().contains(name) {
            let iter = self.vertex_property_value_manager.iterate_for_name(name);
            let vertices = self.vertices_from_property_value_iterator(iter)?;
            let iter = vertices.into_iter().map(|v| Ok(v.id));
            Ok(Some(Box::new(iter)))
        } else {
            Ok(None)
        }
    }

    fn vertex_ids_with_property_value(
        &'a self,
        name: &Identifier,
        value: &serde_json::Value,
    ) -> Result<Option<DynIter<'a, Uuid>>> {
        if self.indexed_properties.read().unwrap().contains(name) {
            let iter = self
                .vertex_property_value_manager
                .iterate_for_value(name, &Json::new(value.clone()));
            let vertices = self.vertices_from_property_value_iterator(iter)?;
            let iter = vertices.into_iter().map(|v| Ok(v.id));
            Ok(Some(Box::new(iter)))
        } else {
            Ok(None)
        }
    }

    fn edge_count(&self) -> u64 {
        let iter = self.edge_range_manager.iterate_for_all();
        iter.count() as u64
    }

    fn all_edges(&'a self) -> Result<DynIter<'a, Edge>> {
        let iter = self.edge_range_manager.iterate_for_all();
        Ok(Box::new(iter))
    }

    fn range_edges(&'a self, offset: Edge) -> Result<DynIter<'a, Edge>> {
        let iter = self
            .edge_range_manager
            .iterate_for_range(offset.outbound_id, &offset.t, offset.inbound_id)?;
        Ok(Box::new(iter))
    }

    fn range_reversed_edges(&'a self, offset: Edge) -> Result<DynIter<'a, Edge>> {
        let iter =
            self.reversed_edge_range_manager
                .iterate_for_range(offset.inbound_id, &offset.t, offset.outbound_id)?;
        Ok(Box::new(iter))
    }

    fn specific_edges(&'a self, edges: Vec<Edge>) -> Result<DynIter<'a, Edge>> {
        let iter = edges.into_iter().filter_map(move |e| {
            match self.edge_range_manager.contains(e.outbound_id, &e.t, e.inbound_id) {
                Ok(true) => Some(Ok(e)),
                Ok(false) => None,
                Err(err) => Some(Err(err)),
            }
        });

        Ok(Box::new(iter))
    }

    fn edges_with_property(&'a self, name: &Identifier) -> Result<Option<DynIter<'a, Edge>>> {
        if self.indexed_properties.read().unwrap().contains(name) {
            let iter = self
                .edge_property_value_manager
                .iterate_for_name(name)
                .map(|r| match r {
                    Ok((_, _, e)) => Ok(e),
                    Err(err) => Err(err),
                });
            Ok(Some(Box::new(iter)))
        } else {
            Ok(None)
        }
    }

    fn edges_with_property_value(
        &'a self,
        name: &Identifier,
        value: &serde_json::Value,
    ) -> Result<Option<DynIter<'a, Edge>>> {
        if self.indexed_properties.read().unwrap().contains(name) {
            let iter = self
                .edge_property_value_manager
                .iterate_for_value(name, &Json::new(value.clone()))
                .map(|r| match r {
                    Ok((_, _, e)) => Ok(e),
                    Err(err) => Err(err),
                });
            Ok(Some(Box::new(iter)))
        } else {
            Ok(None)
        }
    }

    fn vertex_property(&self, vertex: &Vertex, name: &Identifier) -> Result<Option<serde_json::Value>> {
        match self.vertex_property_manager.get(vertex.id, name)? {
            None => Ok(None),
            Some(value) => Ok(Some(value.0)),
        }
    }

    fn all_vertex_properties_for_vertex(
        &'a self,
        vertex: &Vertex,
    ) -> Result<DynIter<'a, (Identifier, serde_json::Value)>> {
        let iter = self.vertex_property_manager.iterate_for_owner(vertex.id)?;
        let props: Result<Vec<_>> = iter.collect();
        let iter = props?.into_iter().map(|((_, name), value)| Ok((name, value.0)));
        Ok(Box::new(iter))
    }

    fn edge_property(&self, edge: &Edge, name: &Identifier) -> Result<Option<serde_json::Value>> {
        match self
            .edge_property_manager
            .get(edge.outbound_id, &edge.t, edge.inbound_id, name)?
        {
            None => Ok(None),
            Some(value) => Ok(Some(value.0)),
        }
    }

    fn all_edge_properties_for_edge(&'a self, edge: &Edge) -> Result<DynIter<'a, (Identifier, serde_json::Value)>> {
        let iter = self
            .edge_property_manager
            .iterate_for_owner(edge.outbound_id, &edge.t, edge.inbound_id)?;
        let props: Result<Vec<_>> = iter.collect();
        let iter = props?.into_iter().map(|((_, _, _, name), value)| Ok((name, value.0)));
        Ok(Box::new(iter))
    }

    fn delete_vertices(&mut self, vertices: Vec<Vertex>) -> Result<()> {
        let indexed_properties = self.indexed_properties.read().unwrap();
        let mut batch = WriteBatch::default();

        for vertex in vertices.into_iter() {
            self.vertex_manager.delete(&mut batch, &indexed_properties, vertex.id)?;
        }

        self.db.write(batch)?;
        Ok(())
    }

    fn delete_edges(&mut self, edges: Vec<Edge>) -> Result<()> {
        let indexed_properties = self.indexed_properties.read().unwrap();
        let mut batch = WriteBatch::default();

        for edge in edges.into_iter() {
            if self.vertex_manager.get(edge.outbound_id)?.is_some() {
                self.edge_manager.delete(
                    &mut batch,
                    &indexed_properties,
                    edge.outbound_id,
                    &edge.t,
                    edge.inbound_id,
                )?;
            };
        }

        self.db.write(batch)?;
        Ok(())
    }

    fn delete_vertex_properties(&mut self, props: Vec<(Uuid, Identifier)>) -> Result<()> {
        let indexed_properties = self.indexed_properties.read().unwrap();
        let mut batch = WriteBatch::default();

        for (id, name) in props.into_iter() {
            self.vertex_property_manager
                .delete(&mut batch, &indexed_properties, id, &name)?;
        }

        self.db.write(batch)?;
        Ok(())
    }

    fn delete_edge_properties(&mut self, props: Vec<(Edge, Identifier)>) -> Result<()> {
        let indexed_properties = self.indexed_properties.read().unwrap();
        let mut batch = WriteBatch::default();

        for (edge, name) in props.into_iter() {
            self.edge_property_manager.delete(
                &mut batch,
                &indexed_properties,
                edge.outbound_id,
                &edge.t,
                edge.inbound_id,
                &name,
            )?;
        }

        self.db.write(batch)?;
        Ok(())
    }

    fn sync(&self) -> Result<()> {
        self.vertex_manager.compact();
        self.edge_range_manager.compact();
        self.edge_range_manager.compact();
        self.vertex_property_manager.compact();
        self.edge_property_manager.compact();
        self.vertex_property_value_manager.compact();
        self.edge_property_value_manager.compact();
        self.metadata_manager.compact();
        self.db.flush()?;
        Ok(())
    }

    fn create_vertex(&mut self, vertex: &Vertex) -> Result<bool> {
        if self.vertex_manager.exists(vertex.id)? {
            Ok(false)
        } else {
            let mut batch = WriteBatch::default();
            self.vertex_manager.create(&mut batch, vertex)?;
            self.db.write(batch)?;
            Ok(true)
        }
    }

    fn create_edge(&mut self, edge: &Edge) -> Result<bool> {
        let indexed_properties = self.indexed_properties.read().unwrap();

        if !self.vertex_manager.exists(edge.outbound_id)? || !self.vertex_manager.exists(edge.inbound_id)? {
            Ok(false)
        } else {
            let mut batch = WriteBatch::default();
            self.edge_manager
                .set(&mut batch, edge.outbound_id, &edge.t, edge.inbound_id)?;
            self.db.write(batch)?;
            Ok(true)
        }
    }

    // We override the default `bulk_insert` implementation because further
    // optimization can be done by using `WriteBatch`s.
    fn bulk_insert(&mut self, items: Vec<BulkInsertItem>) -> Result<()> {
        let indexed_properties = self.indexed_properties.read().unwrap();
        let mut batch = WriteBatch::default();

        for item in items {
            match item {
                BulkInsertItem::Vertex(ref vertex) => {
                    self.vertex_manager.create(&mut batch, vertex)?;
                }
                BulkInsertItem::Edge(ref key) => {
                    self.edge_manager
                        .set(&mut batch, key.outbound_id, &key.t, key.inbound_id)?;
                }
                BulkInsertItem::VertexProperty(id, ref name, ref value) => {
                    self.vertex_property_manager.set(
                        &mut batch,
                        &indexed_properties,
                        id,
                        name,
                        &Json::new(value.clone()),
                    )?;
                }
                BulkInsertItem::EdgeProperty(ref key, ref name, ref value) => {
                    self.edge_property_manager.set(
                        &mut batch,
                        &indexed_properties,
                        key.outbound_id,
                        &key.t,
                        key.inbound_id,
                        name,
                        &Json::new(value.clone()),
                    )?;
                }
            }
        }

        self.db.write(batch)?;
        Ok(())
    }

    fn index_property(&mut self, name: Identifier) -> Result<()> {
        let mut indexed_properties = self.indexed_properties.write().unwrap();
        if !indexed_properties.insert(name.clone()) {
            return Ok(());
        }

        let mut batch = WriteBatch::default();
        self.metadata_manager
            .set_indexed_properties(&mut batch, &indexed_properties)?;

        for item in self.vertex_manager.iterate_for_range(Uuid::default()) {
            let vertex = item?;
            if let Some(property_value) = self.vertex_property_manager.get(vertex.id, &name)? {
                self.vertex_property_value_manager
                    .set(&mut batch, vertex.id, &name, &property_value);
            }
        }

        for item in self.edge_range_manager.iterate_for_all() {
            let edge = item?;
            if let Some(property_value) =
                self.edge_property_manager
                    .get(edge.outbound_id, &edge.t, edge.inbound_id, &name)?
            {
                self.edge_property_value_manager.set(
                    &mut batch,
                    edge.outbound_id,
                    &edge.t,
                    edge.inbound_id,
                    &name,
                    &property_value,
                );
            }
        }

        self.db.write(batch)?;
        Ok(())
    }

    fn set_vertex_properties(&mut self, vertices: Vec<Uuid>, name: Identifier, value: serde_json::Value) -> Result<()> {
        let indexed_properties = self.indexed_properties.read().unwrap();
        let mut batch = WriteBatch::default();

        let wrapped_value = Json::new(value);
        for id in vertices.into_iter() {
            self.vertex_property_manager
                .set(&mut batch, &indexed_properties, id, &name, &wrapped_value)?;
        }

        self.db.write(batch)?;
        Ok(())
    }

    fn set_edge_properties(&mut self, edges: Vec<Edge>, name: Identifier, value: serde_json::Value) -> Result<()> {
        let indexed_properties = self.indexed_properties.read().unwrap();
        let mut batch = WriteBatch::default();

        let wrapped_value = Json::new(value);
        for edge in edges.into_iter() {
            self.edge_property_manager.set(
                &mut batch,
                &indexed_properties,
                edge.outbound_id,
                &edge.t,
                edge.inbound_id,
                &name,
                &wrapped_value,
            )?;
        }

        self.db.write(batch)?;
        Ok(())
    }
}

/// A datastore that is backed by rocksdb.
#[derive(Debug)]
pub struct RocksdbDatastore {
    db: Arc<DB>,
    indexed_properties: Arc<RwLock<HashSet<Identifier>>>,
}

impl RocksdbDatastore {
    /// Creates a new rocksdb datastore.
    ///
    /// # Arguments
    /// * `path`: The file path to the rocksdb database.
    /// * `max_open_files`: The maximum number of open files to have. If
    ///   `None`, the default will be used.
    pub fn new<P: AsRef<Path>>(path: P, max_open_files: Option<i32>) -> Result<RocksdbDatastore> {
        let opts = get_options(max_open_files);
        let path = path.as_ref();

        let db = match DB::open_cf(&opts, path, CF_NAMES) {
            Ok(db) => db,
            Err(_) => {
                let mut db = DB::open(&opts, path)?;

                for cf_name in &CF_NAMES {
                    db.create_cf(cf_name, &opts)?;
                }

                db
            }
        };

        let metadata_manager = MetadataManager::new(&db);
        let indexed_properties = metadata_manager.get_indexed_properties()?;

        Ok(RocksdbDatastore {
            db: Arc::new(db),
            indexed_properties: Arc::new(RwLock::new(indexed_properties)),
        })
    }

    /// Runs a repair operation on the rocksdb database.
    ///
    /// # Arguments
    /// * `path`: The file path to the rocksdb database.
    /// * `max_open_files`: The maximum number of open files to have. If
    ///   `None`, the default will be used.
    pub fn repair<P: AsRef<Path>>(path: P, max_open_files: Option<i32>) -> Result<()> {
        let opts = get_options(max_open_files);
        DB::repair(&opts, path)?;
        Ok(())
    }
}

impl Datastore for RocksdbDatastore {
    type Transaction<'a> = RocksdbTransaction<'a> where Self: 'a;
    fn transaction<'a>(&'a self) -> Self::Transaction<'a> {
        RocksdbTransaction {
            db: &self.db,
            indexed_properties: self.indexed_properties.clone(),
            vertex_manager: VertexManager::new(&self.db),
            edge_manager: EdgeManager::new(&self.db),
            edge_range_manager: EdgeRangeManager::new(&self.db),
            reversed_edge_range_manager: EdgeRangeManager::new(&self.db),
            vertex_property_manager: VertexPropertyManager::new(&self.db),
            edge_property_manager: EdgePropertyManager::new(&self.db),
            vertex_property_value_manager: VertexPropertyValueManager::new(&self.db),
            edge_property_value_manager: EdgePropertyValueManager::new(&self.db),
            metadata_manager: MetadataManager::new(&self.db),
        }
    }
}
