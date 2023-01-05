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

const CF_NAMES: [&str; 9] = [
    "vertices:v1",
    "edges:v1",
    "edge_ranges:v1",
    "reversed_edge_ranges:v1",
    "vertex_properties:v1",
    "edge_properties:v1",
    "vertex_property_values:v1",
    "edge_property_values:v1",
    "metadata:v1",
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
    db: Arc<DB>,
}

impl<'a> RocksdbTransaction<'a> {
    fn guard_indexed_property(&self, property: &Identifier) -> Result<()> {
        if !self.indexed_properties.read().unwrap().contains(property) {
            Err(Error::NotIndexed)
        } else {
            Ok(())
        }
    }

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

    fn edges_from_property_value_iterator<'b>(
        &self,
        iter: impl Iterator<Item = Result<EdgePropertyValueKey>> + 'b,
    ) -> Result<Vec<EdgeRangeItem>> {
        let mut edges = Vec::new();
        for item in iter {
            let (_, _, (out_id, t, in_id)) = item?;
            if self.edge_manager.get(out_id, &t, in_id)? {
                edges.push((out_id, t, in_id));
            }
        }

        Ok(edges)
    }
}

impl<'a> Transaction<'a> for RocksdbTransaction<'a> {
    fn vertex_count(&self) -> u64 {
        let iterator = self.vertex_manager.iterate_for_range(Uuid::default());
        iterator.count() as u64
    }

    fn all_vertices(&'a self) -> Result<DynIter<'a, Vertex>> {
        let iter = self.vertex_manager.iterate_for_range(Uuid::default());
        Ok(Box::new(iter))
    }

    fn range_vertices(&'a self, offset: Uuid) -> Result<DynIter<'a, Vertex>> {
        todo!();
    }

    fn specific_vertices(&'a self, ids: &Vec<Uuid>) -> Result<DynIter<'a, Vertex>> {
        todo!();
    }

    fn vertex_ids_with_property(&'a self, name: &Identifier) -> Result<Option<DynIter<'a, Uuid>>> {
        todo!();
    }

    fn vertex_ids_with_property_value(
        &'a self,
        name: &Identifier,
        value: &serde_json::Value,
    ) -> Result<Option<DynIter<'a, Uuid>>> {
        todo!();
    }

    fn edge_count(&self) -> u64 {
        todo!();
    }

    fn all_edges(&'a self) -> Result<DynIter<'a, Edge>> {
        todo!();
    }

    fn range_edges(&'a self, offset: Edge) -> Result<DynIter<'a, Edge>> {
        todo!();
    }

    fn range_reversed_edges(&'a self, offset: Edge) -> Result<DynIter<'a, Edge>> {
        todo!();
    }

    fn specific_edges(&'a self, edges: &Vec<Edge>) -> Result<DynIter<'a, Edge>> {
        todo!();
    }

    fn edges_with_property(&'a self, name: &Identifier) -> Result<Option<DynIter<'a, Edge>>> {
        todo!();
    }

    fn edges_with_property_value(
        &'a self,
        name: &Identifier,
        value: &serde_json::Value,
    ) -> Result<Option<DynIter<'a, Edge>>> {
        todo!();
    }

    fn vertex_property(&self, vertex: &Vertex, name: &Identifier) -> Result<Option<serde_json::Value>> {
        todo!();
    }

    fn all_vertex_properties_for_vertex(
        &'a self,
        vertex: &Vertex,
    ) -> Result<DynIter<'a, (Identifier, serde_json::Value)>> {
        todo!();
    }

    fn edge_property(&self, edge: &Edge, name: &Identifier) -> Result<Option<serde_json::Value>> {
        todo!();
    }

    fn all_edge_properties_for_edge(&'a self, edge: &Edge) -> Result<DynIter<'a, (Identifier, serde_json::Value)>> {
        todo!();
    }

    fn delete_vertices(&mut self, vertices: Vec<Vertex>) -> Result<()> {
        todo!();
    }

    fn delete_edges(&mut self, edges: Vec<Edge>) -> Result<()> {
        todo!();
    }

    fn delete_vertex_properties(&mut self, props: Vec<(Uuid, Identifier)>) -> Result<()> {
        todo!();
    }

    fn delete_edge_properties(&mut self, props: Vec<(Edge, Identifier)>) -> Result<()> {
        todo!();
    }

    fn sync(&self) -> Result<()> {
        self.vertex_manager.compact();
        self.edge_manager.compact();
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
        todo!();
    }

    fn bulk_insert(&mut self, items: Vec<BulkInsertItem>) -> Result<()> {
        todo!();
    }

    fn index_property(&mut self, name: Identifier) -> Result<()> {
        todo!();
    }

    fn set_vertex_properties(&mut self, vertices: Vec<Uuid>, name: Identifier, value: serde_json::Value) -> Result<()> {
        todo!();
    }

    fn set_edge_properties(&mut self, edges: Vec<Edge>, name: Identifier, value: serde_json::Value) -> Result<()> {
        todo!();
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
        let db = self.db.clone();
        RocksdbTransaction {
            indexed_properties: self.indexed_properties.clone(),
            vertex_manager: VertexManager::new(&db),
            edge_manager: EdgeManager::new(&db),
            edge_range_manager: EdgeRangeManager::new(&db),
            reversed_edge_range_manager: EdgeRangeManager::new(&db),
            vertex_property_manager: VertexPropertyManager::new(&db),
            edge_property_manager: EdgePropertyManager::new(&db),
            vertex_property_value_manager: VertexPropertyValueManager::new(&db),
            edge_property_value_manager: EdgePropertyValueManager::new(&db),
            metadata_manager: MetadataManager::new(&db),
            db,
        }
    }
}

// fn get_vertices(&self, q: VertexQuery) -> Result<Vec<Vertex>> {
//     let db = self.db.clone();
//     let indexed_properties = self.indexed_properties.read().unwrap();
//     let db_ref = DBRef::new(&db, &indexed_properties);
//     let iter = execute_vertex_query(db_ref, q)?.into_iter();

//     let iter = iter.map(move |(id, t)| {
//         let vertex = Vertex::with_id(id, t);
//         Ok(vertex)
//     });

//     iter.collect()
// }

// fn delete_vertices(&self, q: VertexQuery) -> Result<()> {
//     let db = self.db.clone();
//     let indexed_properties = self.indexed_properties.read().unwrap();
//     let db_ref = DBRef::new(&db, &indexed_properties);
//     let iter = execute_vertex_query(db_ref, q)?.into_iter();
//     let vertex_manager = VertexManager::new(db_ref);
//     let mut batch = WriteBatch::default();

//     for (id, _) in iter {
//         vertex_manager.delete(&mut batch, id)?;
//     }

//     db.write(batch)?;
//     Ok(())
// }

// fn create_edge(&self, key: &EdgeKey) -> Result<bool> {
//     let db = self.db.clone();
//     let indexed_properties = self.indexed_properties.read().unwrap();
//     let db_ref = DBRef::new(&db, &indexed_properties);
//     let vertex_manager = VertexManager::new(db_ref);

//     if !vertex_manager.exists(key.outbound_id)? || !vertex_manager.exists(key.inbound_id)? {
//         Ok(false)
//     } else {
//         let edge_manager = EdgeManager::new(db_ref);
//         let mut batch = WriteBatch::default();
//         edge_manager.set(&mut batch, key.outbound_id, &key.t, key.inbound_id)?;
//         db.write(batch)?;
//         Ok(true)
//     }
// }

// fn get(&self, q: Query) -> Result<Vec<QueryOutputValue>> {
//     // TODO: use `Vec::with_capacity`.
//     let mut output = Vec::new();

//     let db = self.db.clone();
//     let indexed_properties = self.indexed_properties.read().unwrap();
//     let db_ref = DBRef::new(&db, &indexed_properties);

//     execute_query(db_ref, &q, &mut output)?;
//     Ok(output)
// }

// fn get_edges(&self, q: EdgeQuery) -> Result<Vec<Edge>> {
//     let db = self.db.clone();
//     let indexed_properties = self.indexed_properties.read().unwrap();
//     let db_ref = DBRef::new(&db, &indexed_properties);
//     let iter = execute_edge_query(db_ref, q)?.into_iter();

//     let iter = iter.map(move |(out_id, t, update_datetime, in_id)| {
//         let key = EdgeKey::new(out_id, t, in_id);
//         let edge = Edge::new(key, update_datetime);
//         Ok(edge)
//     });

//     iter.collect()
// }

// fn delete_edges(&self, q: EdgeQuery) -> Result<()> {
//     let db = self.db.clone();
//     let indexed_properties = self.indexed_properties.read().unwrap();
//     let db_ref = DBRef::new(&db, &indexed_properties);
//     let edge_manager = EdgeManager::new(db_ref);
//     let vertex_manager = VertexManager::new(db_ref);
//     let iter = execute_edge_query(db_ref, q)?;
//     let mut batch = WriteBatch::default();

//     for (out_id, t, update_datetime, in_id) in iter {
//         if vertex_manager.get(out_id)?.is_some() {
//             edge_manager.delete(&mut batch, out_id, &t, in_id, update_datetime)?;
//         };
//     }

//     db.write(batch)?;
//     Ok(())
// }

// fn get_edge_count(&self, id: Uuid, t: Option<&Identifier>, direction: EdgeDirection) -> Result<u64> {
//     let db = self.db.clone();
//     let indexed_properties = self.indexed_properties.read().unwrap();
//     let db_ref = DBRef::new(&db, &indexed_properties);

//     let edge_range_manager = match direction {
//         EdgeDirection::Outbound => EdgeRangeManager::new(db_ref),
//         EdgeDirection::Inbound => EdgeRangeManager::new_reversed(db_ref),
//     };

//     let count = edge_range_manager.iterate_for_range(id, t, None)?.count();

//     Ok(count as u64)
// }

// fn get_vertex_properties(&self, q: VertexPropertyQuery) -> Result<Vec<VertexProperty>> {
//     let db = self.db.clone();
//     let indexed_properties = self.indexed_properties.read().unwrap();
//     let db_ref = DBRef::new(&db, &indexed_properties);
//     let manager = VertexPropertyManager::new(db_ref);
//     let mut properties = Vec::new();

//     for (id, _) in execute_vertex_query(db_ref, q.inner)?.into_iter() {
//         let value = manager.get(id, &q.name)?;

//         if let Some(value) = value {
//             properties.push(VertexProperty::new(id, value.0));
//         }
//     }

//     Ok(properties)
// }

// fn get_all_vertex_properties(&self, q: VertexQuery) -> Result<Vec<VertexProperties>> {
//     let db = self.db.clone();
//     let indexed_properties = self.indexed_properties.read().unwrap();
//     let db_ref = DBRef::new(&db, &indexed_properties);
//     let iter = execute_vertex_query(db_ref, q)?.into_iter();
//     let manager = VertexPropertyManager::new(db_ref);

//     let iter = iter.map(move |(id, t)| {
//         let vertex = Vertex::with_id(id, t);

//         let it = manager.iterate_for_owner(id)?;
//         let props: Result<Vec<_>> = it.collect();
//         let props_iter = props?.into_iter();
//         let props = props_iter
//             .map(|((_, name), value)| NamedProperty::new(name, value.0))
//             .collect();

//         Ok(VertexProperties::new(vertex, props))
//     });

//     iter.collect()
// }

// fn set_vertex_properties(&self, q: VertexPropertyQuery, value: serde_json::Value) -> Result<()> {
//     let db = self.db.clone();
//     let indexed_properties = self.indexed_properties.read().unwrap();
//     let db_ref = DBRef::new(&db, &indexed_properties);
//     let manager = VertexPropertyManager::new(db_ref);
//     let mut batch = WriteBatch::default();

//     let wrapped_value = Json::new(value);
//     for (id, _) in execute_vertex_query(db_ref, q.inner)?.into_iter() {
//         manager.set(&mut batch, id, &q.name, &wrapped_value)?;
//     }

//     db.write(batch)?;
//     Ok(())
// }

// fn delete_vertex_properties(&self, q: VertexPropertyQuery) -> Result<()> {
//     let db = self.db.clone();
//     let indexed_properties = self.indexed_properties.read().unwrap();
//     let db_ref = DBRef::new(&db, &indexed_properties);
//     let manager = VertexPropertyManager::new(db_ref);
//     let mut batch = WriteBatch::default();

//     for (id, _) in execute_vertex_query(db_ref, q.inner)?.into_iter() {
//         manager.delete(&mut batch, id, &q.name)?;
//     }

//     db.write(batch)?;
//     Ok(())
// }

// fn get_edge_properties(&self, q: EdgePropertyQuery) -> Result<Vec<EdgeProperty>> {
//     let db = self.db.clone();
//     let indexed_properties = self.indexed_properties.read().unwrap();
//     let db_ref = DBRef::new(&db, &indexed_properties);
//     let manager = EdgePropertyManager::new(db_ref);
//     let mut properties = Vec::new();

//     for (out_id, t, _, in_id) in execute_edge_query(db_ref, q.inner)?.into_iter() {
//         let value = manager.get(out_id, &t, in_id, &q.name)?;

//         if let Some(value) = value {
//             let key = EdgeKey::new(out_id, t, in_id);
//             properties.push(EdgeProperty::new(key, value.0));
//         }
//     }

//     Ok(properties)
// }

// fn get_all_edge_properties(&self, q: EdgeQuery) -> Result<Vec<EdgeProperties>> {
//     let db = self.db.clone();
//     let indexed_properties = self.indexed_properties.read().unwrap();
//     let db_ref = DBRef::new(&db, &indexed_properties);
//     let iter = execute_edge_query(db_ref, q)?.into_iter();
//     let manager = EdgePropertyManager::new(db_ref);

//     let iter = iter.map(move |(out_id, t, time, in_id)| {
//         let edge = Edge::new(EdgeKey::new(out_id, t.clone(), in_id), time);
//         let it = manager.iterate_for_owner(out_id, &t, in_id)?;
//         let props: Result<Vec<_>> = it.collect();
//         let props_iter = props?.into_iter();
//         let props = props_iter
//             .map(|((_, _, _, name), value)| NamedProperty::new(name, value.0))
//             .collect();

//         Ok(EdgeProperties::new(edge, props))
//     });

//     iter.collect()
// }

// fn set_edge_properties(&self, q: EdgePropertyQuery, value: serde_json::Value) -> Result<()> {
//     let db = self.db.clone();
//     let indexed_properties = self.indexed_properties.read().unwrap();
//     let db_ref = DBRef::new(&db, &indexed_properties);
//     let manager = EdgePropertyManager::new(db_ref);
//     let mut batch = WriteBatch::default();

//     let wrapped_value = Json::new(value);
//     for (out_id, t, _, in_id) in execute_edge_query(db_ref, q.inner)?.into_iter() {
//         manager.set(&mut batch, out_id, &t, in_id, &q.name, &wrapped_value)?;
//     }

//     db.write(batch)?;
//     Ok(())
// }

// fn delete_edge_properties(&self, q: EdgePropertyQuery) -> Result<()> {
//     let db = self.db.clone();
//     let indexed_properties = self.indexed_properties.read().unwrap();
//     let db_ref = DBRef::new(&db, &indexed_properties);
//     let manager = EdgePropertyManager::new(db_ref);
//     let mut batch = WriteBatch::default();

//     for (out_id, t, _, in_id) in execute_edge_query(db_ref, q.inner)?.into_iter() {
//         manager.delete(&mut batch, out_id, &t, in_id, &q.name)?;
//     }

//     db.write(batch)?;
//     Ok(())
// }

// We override the default `bulk_insert` implementation because further
// optimization can be done by using `WriteBatch`s.
// fn bulk_insert(&self, items: Vec<BulkInsertItem>) -> Result<()> {
//     let db = self.db.clone();
//     let indexed_properties = self.indexed_properties.read().unwrap();
//     let db_ref = DBRef::new(&db, &indexed_properties);
//     let vertex_manager = VertexManager::new(db_ref);
//     let edge_manager = EdgeManager::new(db_ref);
//     let vertex_property_manager = VertexPropertyManager::new(db_ref);
//     let edge_property_manager = EdgePropertyManager::new(db_ref);
//     let mut batch = WriteBatch::default();

//     for item in items {
//         match item {
//             BulkInsertItem::Vertex(ref vertex) => {
//                 vertex_manager.create(&mut batch, vertex)?;
//             }
//             BulkInsertItem::Edge(ref key) => {
//                 edge_manager.set(&mut batch, key.outbound_id, &key.t, key.inbound_id)?;
//             }
//             BulkInsertItem::VertexProperty(id, ref name, ref value) => {
//                 vertex_property_manager.set(&mut batch, id, name, &Json::new(value.clone()))?;
//             }
//             BulkInsertItem::EdgeProperty(ref key, ref name, ref value) => {
//                 edge_property_manager.set(
//                     &mut batch,
//                     key.outbound_id,
//                     &key.t,
//                     key.inbound_id,
//                     name,
//                     &Json::new(value.clone()),
//                 )?;
//             }
//         }
//     }

//     self.db.write(batch)?;
//     Ok(())
// }

// fn index_property(&self, name: Identifier) -> Result<()> {
//     let mut indexed_properties = self.indexed_properties.write().unwrap();
//     if !indexed_properties.insert(name.clone()) {
//         return Ok(());
//     }

//     let db = self.db.clone();
//     let db_ref = DBRef::new(&db, &indexed_properties);
//     let mut batch = WriteBatch::default();
//     let vertex_manager = VertexManager::new(db_ref);
//     let edge_range_manager = EdgeRangeManager::new(db_ref);
//     let vertex_property_manager = VertexPropertyManager::new(db_ref);
//     let edge_property_manager = EdgePropertyManager::new(db_ref);
//     let vertex_property_value_manager = VertexPropertyValueManager::new(db_ref);
//     let edge_property_value_manager = EdgePropertyValueManager::new(db_ref);
//     let metadata_manager = MetadataManager::new(&db);
//     metadata_manager.set_indexed_properties(&mut batch, &indexed_properties)?;

//     for item in vertex_manager.iterate_for_range(Uuid::default()) {
//         let (vertex_id, _) = item?;
//         if let Some(property_value) = vertex_property_manager.get(vertex_id, &name)? {
//             vertex_property_value_manager.set(&mut batch, vertex_id, &name, &property_value);
//         }
//     }

//     for item in edge_range_manager.iterate_for_all() {
//         let (out_id, t, _, in_id) = item?;
//         if let Some(property_value) = edge_property_manager.get(out_id, &t, in_id, &name)? {
//             edge_property_value_manager.set(&mut batch, out_id, &t, in_id, &name, &property_value);
//         }
//     }

//     db.write(batch)?;
//     Ok(())
// }
// }

// fn query(db_ref: DBRef<'_>, q: &Query, output: &mut Vec<QueryOutputValue>) -> Result<()> {
//     let value = match q {
//         Query::AllVertex(_) => {
//             let vertex_manager = VertexManager::new(db_ref);
//             let mut iter: Box<dyn Iterator<Item = Result<VertexItem>>> =
//                 Box::new(vertex_manager.iterate_for_range(None));
//             QueryOutputValue::Vertices(vertices_from_iter(iter))
//         }
//         Query::RangeVertex(ref q) => {
//             let vertex_manager = VertexManager::new(db_ref);

//             let next_uuid = match q.start_id {
//                 Some(start_id) => {
//                     match next_uuid(start_id) {
//                         Ok(next_uuid) => next_uuid,
//                         // If we get an error back, it's because
//                         // `start_id` is the maximum possible value. We
//                         // know that no vertices exist whose ID is greater
//                         // than the maximum possible value, so just return
//                         // an empty list.
//                         Err(_) => return Ok(vec![]),
//                     }
//                 }
//                 None => Uuid::default(),
//             };

//             let mut iter: Box<dyn Iterator<Item = Result<VertexItem>>> =
//                 Box::new(vertex_manager.iterate_for_range(next_uuid));

//             if let Some(ref t) = q.t {
//                 iter = Box::new(iter.filter(move |item| match item {
//                     Ok((_, v)) => v == t,
//                     Err(_) => true,
//                 }));
//             }

//             let vertices: Result<Vec<VertexItem>> = iter.take(q.limit as usize);
//             QueryOutputValue::Vertices(vertices_from_iter(iter))
//         }
//         Query::SpecificVertex(ref q) => {
//             let vertex_manager = VertexManager::new(db_ref);

//             let iter = q.ids.into_iter().map(move |id| match vertex_manager.get(id)? {
//                 Some(value) => Ok(Some((id, value))),
//                 None => Ok(None),
//             });

//             let iter = iter.filter_map(|item| match item {
//                 Err(err) => Some(Err(err)),
//                 Ok(Some(value)) => Some(Ok(value)),
//                 _ => None,
//             });

//             QueryOutputValue::Vertices(vertices_from_iter(iter))
//         }
//         Query::Pipe(ref q) => {
//             self.query(&*q.inner, output)?;
//             let piped_values = output.pop().unwrap();

//             let values = match piped_values {
//                 QueryOutputValue::Edges(ref piped_edges) => {
//                     let vertex_manager = VertexManager::new(db_ref);
//                     let direction = q.direction;

//                     let iter = piped_edges.iter().map(move |(out_id, _, _, in_id)| {
//                         let id = match direction {
//                             EdgeDirection::Outbound => out_id,
//                             EdgeDirection::Inbound => in_id,
//                         };

//                         match vertex_manager.get(id)? {
//                             Some(value) => Ok(Some((id, value))),
//                             None => Ok(None),
//                         }
//                     });

//                     let iter = iter.filter_map(|item| match item {
//                         Err(err) => Some(Err(err)),
//                         Ok(Some(value)) => Some(Ok(value)),
//                         _ => None,
//                     });

//                     let mut iter: Box<dyn Iterator<Item = Result<VertexItem>>> = Box::new(iter);

//                     if let Some(ref t) = q.t {
//                         iter = Box::new(iter.filter(move |item| match item {
//                             Ok((_, v)) => v == t,
//                             Err(_) => true,
//                         }));
//                     }

//                     let iter = iter.take(q.limit as usize);
//                     QueryOutputValue::Vertices(vertices_from_iter(iter))
//                 }
//                 QueryOutputValue::Vertices(ref piped_vertices) => {
//                     let edge_range_manager = match q.direction {
//                         EdgeDirection::Outbound => EdgeRangeManager::new(db_ref),
//                         EdgeDirection::Inbound => EdgeRangeManager::new_reversed(db_ref),
//                     };

//                     // Ideally we'd use iterators all the way down, but things
//                     // start breaking apart due to conditional expressions not
//                     // returning the same type signature, issues with `Result`s
//                     // and some of the iterators, etc. So at this point, we'll
//                     // just resort to building a vector.
//                     let mut edges: Vec<EdgeRangeItem> = Vec::new();

//                     for (id, _) in piped_vertices {
//                         let edge_iterator = edge_range_manager.iterate_for_range(id, q.t.as_ref())?;

//                         for item in edge_iterator {
//                             let (edge_range_first_id, edge_range_t, edge_range_second_id) = item?;

//                             edges.push(match q.direction {
//                                 EdgeDirection::Outbound => (edge_range_first_id, edge_range_t, edge_range_second_id),
//                                 EdgeDirection::Inbound => (edge_range_second_id, edge_range_t, edge_range_first_id),
//                             });

//                             if edges.len() == q.limit as usize {
//                                 break;
//                             }
//                         }
//                     }

//                     QueryOutputValue::Edges(edges_from_iter(iter))
//                 }
//                 _ => {
//                     return Err(Error::Unsupported);
//                 }
//             };

//             if let Query::Include(_) = *q.inner {
//                 // keep the value exported
//                 output.push(piped_values);
//             }

//             values
//         }
//         Query::VertexWithPropertyPresence(ref q) => {
//             guard_indexed_property(db_ref, &q.name)?;
//             let vertex_property_value_manager = VertexPropertyValueManager::new(db_ref);
//             let iter = vertex_property_value_manager.iterate_for_name(&q.name);
//             let vertices = vertices_from_property_value_iterator(db_ref, iter);
//             QueryOutputValue::Vertices(vertices)
//         }
//         Query::VertexWithPropertyValue(ref q) => {
//             guard_indexed_property(db_ref, &q.name)?;
//             let vertex_property_value_manager = VertexPropertyValueManager::new(db_ref);
//             let iter = vertex_property_value_manager.iterate_for_value(&q.name, &Json::new(q.value));
//             let vertices = vertices_from_property_value_iterator(db_ref, iter);
//             QueryOutputValue::Vertices(vertices)
//         }
//         Query::EdgeWithPropertyPresence(ref q) => {
//             guard_indexed_property(db_ref, &q.name)?;
//             let edge_property_value_manager = EdgePropertyValueManager::new(db_ref);
//             let iter = edge_property_value_manager.iterate_for_name(&q.name);
//             let edges = edges_from_property_value_iterator(db_ref, iter);
//             QueryOutputValue::Edges(edges)
//         }
//         Query::EdgeWithPropertyValue(ref q) => {
//             guard_indexed_property(db_ref, &q.name)?;
//             let edge_property_value_manager = EdgePropertyValueManager::new(db_ref);
//             let iter = edge_property_value_manager.iterate_for_value(&q.name, &Json::new(q.value));
//             let edges = edges_from_property_value_iterator(db_ref, iter);
//             QueryOutputValue::Edges(edges)
//         }
//         Query::SpecificEdge(ref q) => {
//             let edge_manager = EdgeManager::new(db_ref);

//             let iter = q.keys.into_iter().map(move |key| -> Result<Option<EdgeRangeItem>> {
//                 match edge_manager.get(key.outbound_id, &key.t, key.inbound_id)? {
//                     Some(update_datetime) => {
//                         Ok(Some((key.outbound_id, key.t.clone(), update_datetime, key.inbound_id)))
//                     }
//                     None => Ok(None),
//                 }
//             });

//             let iter = iter.filter_map(|item| match item {
//                 Err(err) => Some(Err(err)),
//                 Ok(Some(value)) => Some(Ok(value)),
//                 _ => None,
//             });

//             let edges = edges_from_iter(iter)?;
//             QueryOutputValue::Edges(edges)
//         }
//         Query::Include(ref q) => {
//             self.query(&*q.inner, output)?;
//             output.pop().unwrap()
//         }
//     };

//     output.push(value);
//     Ok(())
// }

// fn vertices_from_piped_property_query(
//     &self,
//     vertices: Vec<Vertex>,
//     property_name: &Identifier,
//     intersection: bool,
// ) -> Result<Vec<Vertex>> {
//     let mut piped_vertices_mapping: HashMap<Uuid, Identifier> = vertices.into_iter().collect();
//     let piped_vertices: HashSet<Uuid> = piped_vertices_mapping.keys().cloned().collect();

//     let property_vertices = {
//         self.guard_indexed_property(property_name)?;
//         let vertex_property_value_manager = VertexPropertyValueManager::new(self.db);
//         let iter = vertex_property_value_manager.iterate_for_name(property_name);
//         let mut property_vertices = HashSet::new();
//         for item in iter {
//             let (_, _, id) = item?;
//             property_vertices.insert(id);
//         }
//         property_vertices
//     };

//     let merged_vertices: Box<dyn Iterator<Item = &Uuid>> = if intersection {
//         Box::new(piped_vertices.intersection(&property_vertices))
//     } else {
//         Box::new(piped_vertices.difference(&property_vertices))
//     };

//     Ok(merged_vertices
//         .map(move |id| Vertex::with_id(*id, piped_vertices_mapping.remove(id).unwrap()))
//         .collect())
// }

// fn edges_from_piped_property_query(
//     &self,
//     inner_query: EdgeQuery,
//     property_query: EdgeQuery,
//     intersection: bool,
// ) -> Result<Vec<EdgeRangeItem>> {
//     let mut piped_edges_mapping: HashMap<EdgeKey, DateTime<Utc>> = execute_edge_query(self.db, inner_query)?
//         .into_iter()
//         .map(move |item| {
//             let (out_id, t, dt, in_id) = item;
//             (EdgeKey::new(out_id, t, in_id), dt)
//         })
//         .collect();
//     let piped_edges: HashSet<EdgeKey> = piped_edges_mapping.keys().cloned().collect();

//     let property_edges: HashSet<EdgeKey> = {
//         execute_edge_query(self.db, property_query)?
//             .into_iter()
//             .map(move |item| {
//                 let (out_id, t, _, in_id) = item;
//                 EdgeKey::new(out_id, t, in_id)
//             })
//             .collect()
//     };

//     let merged_edges: Box<dyn Iterator<Item = &EdgeKey>> = if intersection {
//         Box::new(piped_edges.intersection(&property_edges))
//     } else {
//         Box::new(piped_edges.difference(&property_edges))
//     };

//     Ok(merged_edges
//         .map(move |key| {
//             let dt = piped_edges_mapping.remove(key).unwrap();
//             (key.outbound_id, key.t.clone(), dt, key.inbound_id)
//         })
//         .collect())
// }
